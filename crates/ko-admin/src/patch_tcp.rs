//! TCP half of the launcher patch system: speaks the exact binary protocol the real JstKO
//! Launcher (`NTT2625-d3d9proxy/Launcher/APISocket.cpp` + `LauncherEngine.cpp`) already uses —
//! this crate doesn't invent a protocol, it implements the one the launcher's C++ expects.
//! Traffic is unencrypted on purpose: `CAPISocket::InitCrypt` is never called with a non-zero
//! key anywhere in the launcher project, so `s_bCryptionFlag` stays false and frames go over
//! the wire exactly as built below.
//!
//! Frame (both directions): `[u16 0xAA55 big-endian][u16 size, raw little-endian][size bytes:
//! opcode + body][u16 0x55AA big-endian]`, `size = 1 + body.len()`.
//! String encoding inside a body: `[u16 len, raw little-endian][len bytes, no terminator]`.
//!
//! Opcodes (client -> server, server -> client):
//! - `1`: client sends no body; server replies `[i16 latest_version]`.
//! - `2`: client sends `[i16 client_version]`; server replies
//!   `[string http_host_port][string url_path][u16 file_count][file_count x string "<ver>.zip"]`
//!   — every version after `client_version` up to the latest, in order. The launcher downloads
//!   and extracts each zip in sequence via `http://{http_host_port}/{url_path}/{file}`
//!   (`patch.rs` serves that).
//! - `3`: client sends no body; server replies `[u16 notice_count][notice_count x string]`,
//!   read from `JSTKO_PATCH_DIR/notices.txt` (one notice per line), or 0 notices if that file
//!   doesn't exist.
//!
//! Versions come from the `<N>.zip` files present under `JSTKO_PATCH_DIR/zips/`, built by
//! `package_patch_zip.py` — this module never writes there, only reads.

use std::path::{Path, PathBuf};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

const HEADER: u16 = 0xAA55;
const TAIL: u16 = 0x55AA;
/// Matches `RECEIVE_BUF_SIZE` in the launcher's `APISocket.h` — also our sanity cap per frame.
const MAX_FRAME: usize = 262_144;

/// Starts the listener as a background task. Errors (bind failure) are logged, not fatal to
/// ko-admin as a whole, since the HTTP admin panel should keep working either way.
pub fn spawn(patch_dir: PathBuf, http_addr: String, bind: String) {
    tokio::spawn(async move {
        let listener = match TcpListener::bind(&bind).await {
            Ok(l) => l,
            Err(e) => {
                warn!(%bind, error = %e, "launcher patch TCP server failed to bind, patch pulls are unavailable");
                return;
            }
        };
        info!(address = %bind, "launcher patch TCP server listening");
        loop {
            let Ok((stream, peer)) = listener.accept().await else { continue };
            let patch_dir = patch_dir.clone();
            let http_addr = http_addr.clone();
            tokio::spawn(async move {
                if let Err(e) = handle(stream, &patch_dir, &http_addr).await {
                    warn!(%peer, error = %e, "launcher patch connection ended");
                }
            });
        }
    });
}

async fn handle(mut stream: TcpStream, patch_dir: &Path, http_addr: &str) -> anyhow::Result<()> {
    loop {
        let Some((opcode, body)) = read_frame(&mut stream).await? else {
            return Ok(());
        };
        match opcode {
            1 => {
                let latest = latest_version(patch_dir).await;
                let mut out = Vec::new();
                write_i16(&mut out, latest as i16);
                send_frame(&mut stream, 1, &out).await?;
            }
            2 => {
                let client_version = read_i16(&body).unwrap_or(0).max(0) as u32;
                let latest = latest_version(patch_dir).await;
                let files: Vec<String> = ((client_version + 1)..=latest)
                    .filter(|v| zip_exists(patch_dir, *v))
                    .map(|v| format!("{v}.zip"))
                    .collect();
                let mut out = Vec::new();
                write_str(&mut out, http_addr);
                write_str(&mut out, "patches");
                write_u16(&mut out, files.len() as u16);
                for f in &files {
                    write_str(&mut out, f);
                }
                send_frame(&mut stream, 2, &out).await?;
            }
            3 => {
                let notices = notices(patch_dir).await;
                let mut out = Vec::new();
                write_u16(&mut out, notices.len() as u16);
                for n in &notices {
                    write_str(&mut out, n);
                }
                send_frame(&mut stream, 3, &out).await?;
            }
            other => {
                warn!(opcode = other, "launcher patch: unknown opcode, ignoring");
            }
        }
    }
}

async fn latest_version(patch_dir: &Path) -> u32 {
    let zips = patch_dir.join("zips");
    let mut best = 0u32;
    if let Ok(mut rd) = tokio::fs::read_dir(&zips).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if let Some(stem) = name.strip_suffix(".zip") {
                if let Ok(v) = stem.parse::<u32>() {
                    best = best.max(v);
                }
            }
        }
    }
    best
}

fn zip_exists(patch_dir: &Path, version: u32) -> bool {
    patch_dir.join("zips").join(format!("{version}.zip")).is_file()
}

async fn notices(patch_dir: &Path) -> Vec<String> {
    match tokio::fs::read_to_string(patch_dir.join("notices.txt")).await {
        Ok(text) => text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).map(str::to_owned).collect(),
        Err(_) => Vec::new(),
    }
}

// ---- framing --------------------------------------------------------------

/// Reads one frame; `Ok(None)` means the peer closed the connection cleanly.
async fn read_frame(stream: &mut TcpStream) -> anyhow::Result<Option<(u8, Vec<u8>)>> {
    let mut hdr = [0u8; 4];
    if let Err(e) = stream.read_exact(&mut hdr).await {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(None);
        }
        return Err(e.into());
    }
    let header = u16::from_be_bytes([hdr[0], hdr[1]]);
    if header != HEADER {
        anyhow::bail!("bad frame header 0x{header:04X}");
    }
    let size = u16::from_le_bytes([hdr[2], hdr[3]]) as usize;
    if size == 0 || size > MAX_FRAME {
        anyhow::bail!("bad frame size {size}");
    }
    let mut payload = vec![0u8; size];
    stream.read_exact(&mut payload).await?;
    let mut tail = [0u8; 2];
    stream.read_exact(&mut tail).await?;
    if u16::from_be_bytes(tail) != TAIL {
        anyhow::bail!("bad frame tail");
    }
    let opcode = payload[0];
    Ok(Some((opcode, payload[1..].to_vec())))
}

async fn send_frame(stream: &mut TcpStream, opcode: u8, body: &[u8]) -> anyhow::Result<()> {
    let size = 1 + body.len();
    let mut out = Vec::with_capacity(size + 6);
    out.extend_from_slice(&HEADER.to_be_bytes());
    out.extend_from_slice(&(size as u16).to_le_bytes());
    out.push(opcode);
    out.extend_from_slice(body);
    out.extend_from_slice(&TAIL.to_be_bytes());
    stream.write_all(&out).await?;
    Ok(())
}

// ---- KO-style value encoding -----------------------------------------------

fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_i16(out: &mut Vec<u8>, v: i16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    write_u16(out, s.len() as u16);
    out.extend_from_slice(s.as_bytes());
}

fn read_i16(body: &[u8]) -> Option<i16> {
    Some(i16::from_le_bytes([*body.first()?, *body.get(1)?]))
}
