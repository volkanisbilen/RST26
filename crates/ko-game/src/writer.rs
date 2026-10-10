//! Writer task — receives packets from a channel, encrypts, and writes to TCP.
//!
//! Spawned when a session upgrades to in-game mode (GameStart phase 2).
//! Receives `Arc<Packet>` via `mpsc::UnboundedReceiver`, encrypts with JvCryption,
//! frames with KO wire format, and writes to the TCP socket.
//!
//! Uses write batching: after the first `recv().await`, all additional pending
//! packets are drained via `try_recv()` and their frames are appended to a
//! single buffer, which is flushed with one `write_all()` syscall.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc;

use ko_protocol::aes_crypt::AES_FLAG;
use ko_protocol::packet::{FOOTER_OUTGOING, HEADER_OUTGOING, OUTGOING_CRYPTO_MAGIC};
use ko_protocol::{AesCryption, JvCryption, Packet};

/// Writer task loop — runs until the channel is closed.
///
/// Receives packets from other tasks (broadcast, self-send) and writes them
/// to the TCP socket with encryption and framing.
///
/// Uses write batching to reduce TCP syscalls: after the first blocking
/// `recv()` returns a packet, all additional pending packets are drained
/// with non-blocking `try_recv()`. All frames are built into a single
/// buffer and flushed with one `write_all()`.
pub async fn writer_loop(
    mut write_half: OwnedWriteHalf,
    mut rx: mpsc::UnboundedReceiver<Arc<Packet>>,
    crypto: Arc<JvCryption>,
    sequence: Arc<AtomicU32>,
    aes: Arc<AesCryption>,
) {
    tracing::info!(
        "Writer task STARTED — AES enabled={}, JvCryption enabled={}",
        aes.is_enabled(),
        crypto.is_enabled(),
    );

    // Reusable buffer — avoids per-packet heap allocation.
    // Initial 4 KiB; grows as needed and stays at peak size.
    let mut buf = Vec::with_capacity(4096);
    let mut total_packets: u64 = 0;
    let mut total_writes: u64 = 0;

    while let Some(first_packet) = rx.recv().await {
        buf.clear();
        let seq = sequence.load(Ordering::Acquire);
        let mut batch_count: u32 = 1;
        let mut hook_routes = Vec::new();
        if first_packet.opcode == 0xE9 {
            hook_routes.push((first_packet.data.first().copied(), first_packet.data.len()));
        }
        total_packets += 1;

        tracing::info!(
            "Writer recv: opcode=0x{:02X} len={} plaintext={} seq={} total_pkts={}",
            first_packet.opcode,
            first_packet.data.len(),
            first_packet.plaintext,
            seq,
            total_packets,
        );

        // Build first packet frame
        if let Err(e) = build_frame(&crypto, seq, &first_packet, &mut buf, &aes) {
            tracing::warn!(
                "Writer FATAL build error on opcode=0x{:02X} len={}: {}",
                first_packet.opcode,
                first_packet.data.len(),
                e,
            );
            break;
        }

        // Drain all additional pending packets (non-blocking)
        while let Ok(packet) = rx.try_recv() {
            if packet.opcode == 0xE9 {
                hook_routes.push((packet.data.first().copied(), packet.data.len()));
            }
            let seq = sequence.load(Ordering::Acquire);
            batch_count += 1;
            total_packets += 1;
            if let Err(e) = build_frame(&crypto, seq, &packet, &mut buf, &aes) {
                tracing::warn!(
                    "Writer build error on batched opcode=0x{:02X} len={}: {}",
                    packet.opcode,
                    packet.data.len(),
                    e,
                );
                // Still try to send what we have so far
                break;
            }
        }

        // Single TCP write for all batched packet frames
        total_writes += 1;

        // ── Wire-level hex dump (Sprint 15 debug) ──────────────────────
        // Dump each frame in the batch buffer so we can compare byte-for-byte
        // with original server sniffer data when reproducing the map entry crash.
        {
            let mut off = 0usize;
            let mut frame_idx = 0u32;
            while off + 6 <= buf.len() {
                // header(2) + len(2) = 4 bytes minimum before payload
                let payload_len = u16::from_le_bytes([buf[off + 2], buf[off + 3]]) as usize;
                let frame_end = off + 4 + payload_len + 2; // header + len + payload + footer
                if frame_end > buf.len() {
                    break;
                }
                let wire: String = buf[off..frame_end]
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                let flag = buf[off + 4]; // first payload byte (0x01=AES, else opcode)
                let opcode = if flag == 0x01 && payload_len > 1 {
                    "AES"
                } else {
                    &format!("0x{:02X}", flag)
                };
                tracing::info!(
                    "WIRE S2C #{} [{}] payload={} wire_len={}: {}",
                    frame_idx,
                    opcode,
                    payload_len,
                    frame_end - off,
                    wire,
                );
                off = frame_end;
                frame_idx += 1;
            }
        }

        tracing::info!(
            "Writer flush: batch={} buf_bytes={} write_num={}",
            batch_count,
            buf.len(),
            total_writes,
        );
        if let Err(e) = write_half.write_all(&buf).await {
            tracing::warn!(
                "Writer FATAL TCP write error: {} (buf_bytes={}, batch={})",
                e,
                buf.len(),
                batch_count,
            );
            break;
        }
        for (sub, bytes) in hook_routes {
            tracing::info!(target: "hook_protocol", sub, bytes,
                "HOOK S2C TCP write completed");
        }
    }
    tracing::warn!(
        "Writer task EXITING — total_packets={}, total_writes={}",
        total_packets,
        total_writes,
    );
}

/// Build a framed+encrypted packet and append it to `buf`.
///
/// Wire format: `[0xAA 0x55] [len: u16le] [payload] [0x55 0xAA]`
///
/// Unlike a send function, this only appends the frame bytes to `buf`
/// without performing any I/O. Multiple frames can be accumulated in
/// the same buffer and flushed with a single `write_all()`.
/// Build a framed+encrypted packet and append it to `buf`.
///
/// AES path: `[header] [len] [0x01 | AES_CBC(opcode|data)] [footer]`
/// JvCryption path: `[header] [len] [magic|seq|0|opcode|data → encrypted] [footer]`
/// Plaintext path: `[header] [len] [opcode|data] [footer]`
fn build_frame(
    crypto: &JvCryption,
    sequence: u32,
    packet: &Packet,
    buf: &mut Vec<u8>,
    aes: &AesCryption,
) -> anyhow::Result<()> {
    // PCAP verified: Some packets (e.g. S→C 0x02 heartbeat probe) are sent as
    // plaintext even when AES is active. The client detects by payload[0] != 0x01.
    if packet.plaintext {
        return build_frame_plaintext(packet, buf);
    }

    if aes.is_enabled() {
        return build_frame_aes(aes, packet, buf);
    }

    let frame_start = buf.len();

    // Header + length placeholder (filled after payload size is known)
    buf.extend_from_slice(&HEADER_OUTGOING);
    buf.extend_from_slice(&[0u8; 2]);
    let payload_start = frame_start + 4; // header(2) + len(2)

    if crypto.is_enabled() {
        buf.extend_from_slice(&OUTGOING_CRYPTO_MAGIC.to_le_bytes());
        buf.extend_from_slice(&(sequence as u16).to_le_bytes());
        buf.push(0x00);
        buf.push(packet.opcode);
        buf.extend_from_slice(&packet.data);
        crypto.encrypt(&mut buf[payload_start..]);
    } else {
        buf.push(packet.opcode);
        buf.extend_from_slice(&packet.data);
    }

    let payload_len = buf.len() - payload_start;
    if payload_len > u16::MAX as usize {
        anyhow::bail!(
            "Packet payload too large ({} bytes, max {})",
            payload_len,
            u16::MAX
        );
    }

    // Fill in the length field
    buf[frame_start + 2..frame_start + 4].copy_from_slice(&(payload_len as u16).to_le_bytes());
    buf.extend_from_slice(&FOOTER_OUTGOING);

    Ok(())
}

/// Build an AES-encrypted frame and append it to `buf`.
///
/// Wire format: `[0xAA 0x55] [len: u16le] [0x01 | AES_CBC(CRC32 + opcode + data)] [0x55 0xAA]`
/// CRC32 is computed over the raw plaintext (opcode + data) and prepended before encryption.
fn build_frame_aes(aes: &AesCryption, packet: &Packet, buf: &mut Vec<u8>) -> anyhow::Result<()> {
    // Build plaintext: [opcode | data]
    // Wireshark-verified: NO CRC32 in either direction. Just raw opcode + data.
    let mut plaintext = Vec::with_capacity(1 + packet.data.len());
    plaintext.push(packet.opcode);
    plaintext.extend_from_slice(&packet.data);

    let ciphertext = aes.encrypt(&plaintext);
    let payload_len = 1 + ciphertext.len(); // flag(1) + ciphertext

    if payload_len > u16::MAX as usize {
        anyhow::bail!(
            "AES packet payload too large ({} bytes, max {})",
            payload_len,
            u16::MAX
        );
    }

    buf.extend_from_slice(&HEADER_OUTGOING);
    buf.extend_from_slice(&(payload_len as u16).to_le_bytes());
    buf.push(AES_FLAG);
    buf.extend_from_slice(&ciphertext);
    buf.extend_from_slice(&FOOTER_OUTGOING);

    Ok(())
}

/// Build a plaintext frame (no encryption), append to `buf`.
///
/// PCAP verified: Original game server sends S→C 0x02 heartbeat probes
/// as plaintext (no AES flag). Wire: `[AA 55] [len] [opcode|data] [55 AA]`
fn build_frame_plaintext(packet: &Packet, buf: &mut Vec<u8>) -> anyhow::Result<()> {
    let payload_len = 1 + packet.data.len();
    if payload_len > u16::MAX as usize {
        anyhow::bail!("Plaintext packet too large ({} bytes)", payload_len);
    }
    buf.extend_from_slice(&HEADER_OUTGOING);
    buf.extend_from_slice(&(payload_len as u16).to_le_bytes());
    buf.push(packet.opcode);
    buf.extend_from_slice(&packet.data);
    buf.extend_from_slice(&FOOTER_OUTGOING);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercise the actual channel/TCP path: both the first E9 response and a
    /// queued E9 response used to disappear before build_frame was called.
    #[tokio::test]
    async fn extension_responses_reach_tcp_in_order() {
        use tokio::io::AsyncReadExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = tokio::net::TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        let (_, writer) = server.into_split();
        let (tx, rx) = mpsc::unbounded_channel();
        let crypto = Arc::new(JvCryption::new());
        let aes = Arc::new(AesCryption::new());
        let mut expected = Vec::new();
        for (opcode, payload) in [(0xE9, vec![0xAE, 1, 0]), (0x10, vec![1]), (0xE9, vec![0xA8, 0])] {
            let mut packet = Packet::new(opcode);
            packet.data.extend_from_slice(&payload);
            build_frame(&crypto, 0, &packet, &mut expected, &aes).unwrap();
            tx.send(Arc::new(packet)).unwrap();
        }
        drop(tx);
        let task = tokio::spawn(writer_loop(writer, rx, crypto,
            Arc::new(AtomicU32::new(0)), aes));
        let mut actual = Vec::new();
        tokio::time::timeout(std::time::Duration::from_secs(3), client.read_to_end(&mut actual))
            .await.unwrap().unwrap();
        task.await.unwrap();
        assert_eq!(actual, expected);
    }

    /// Unencrypted frame: header(2) + len(2) + opcode(1) + data + footer(2).
    #[test]
    fn test_build_frame_unencrypted_format() {
        let crypto = JvCryption::new();
        let aes = AesCryption::new();
        let mut pkt = Packet::new(0x42);
        pkt.data.extend_from_slice(&[0x01, 0x02, 0x03]);
        let mut buf = Vec::new();
        build_frame(&crypto, 0, &pkt, &mut buf, &aes).unwrap();
        // header AA 55
        assert_eq!(&buf[0..2], &HEADER_OUTGOING);
        // payload len = opcode(1) + data(3) = 4
        assert_eq!(u16::from_le_bytes([buf[2], buf[3]]), 4);
        // opcode
        assert_eq!(buf[4], 0x42);
        // data
        assert_eq!(&buf[5..8], &[0x01, 0x02, 0x03]);
        // footer 55 AA
        assert_eq!(&buf[8..10], &FOOTER_OUTGOING);
        assert_eq!(buf.len(), 10);
    }

    /// Header is always 0xAA 0x55 and footer is always 0x55 0xAA.
    #[test]
    fn test_header_footer_bytes() {
        assert_eq!(HEADER_OUTGOING, [0xAA, 0x55]);
        assert_eq!(FOOTER_OUTGOING, [0x55, 0xAA]);
    }

    /// Length field is little-endian u16.
    #[test]
    fn test_length_field_le() {
        let crypto = JvCryption::new();
        let aes = AesCryption::new();
        // 200-byte data + 1 opcode = 201 payload
        let mut pkt = Packet::new(0x10);
        pkt.data = vec![0xAB; 200];
        let mut buf = Vec::new();
        build_frame(&crypto, 0, &pkt, &mut buf, &aes).unwrap();
        let len = u16::from_le_bytes([buf[2], buf[3]]);
        assert_eq!(len, 201);
    }

    /// Multiple frames appended to the same buffer.
    #[test]
    fn test_multiple_frames_batched() {
        let crypto = JvCryption::new();
        let aes = AesCryption::new();
        let pkt1 = Packet::new(0x01);
        let pkt2 = Packet::new(0x02);
        let mut buf = Vec::new();
        build_frame(&crypto, 0, &pkt1, &mut buf, &aes).unwrap();
        let first_len = buf.len();
        build_frame(&crypto, 0, &pkt2, &mut buf, &aes).unwrap();
        // Second frame appended after first
        assert_eq!(buf.len(), first_len * 2);
        // First frame header
        assert_eq!(&buf[0..2], &HEADER_OUTGOING);
        // Second frame header
        assert_eq!(&buf[first_len..first_len + 2], &HEADER_OUTGOING);
    }

    /// Empty data packet: payload = opcode only (1 byte).
    #[test]
    fn test_empty_data_frame() {
        let crypto = JvCryption::new();
        let aes = AesCryption::new();
        let pkt = Packet::new(0xFF);
        let mut buf = Vec::new();
        build_frame(&crypto, 0, &pkt, &mut buf, &aes).unwrap();
        // header(2) + len(2) + opcode(1) + footer(2) = 7
        assert_eq!(buf.len(), 7);
        let len = u16::from_le_bytes([buf[2], buf[3]]);
        assert_eq!(len, 1); // just the opcode
        assert_eq!(buf[4], 0xFF);
    }

    // ── Sprint 939: Additional coverage ──────────────────────────────

    /// Footer always ends the frame.
    #[test]
    fn test_footer_at_end() {
        let crypto = JvCryption::new();
        let aes = AesCryption::new();
        let mut pkt = Packet::new(0x10);
        pkt.data = vec![0xAA; 50];
        let mut buf = Vec::new();
        build_frame(&crypto, 0, &pkt, &mut buf, &aes).unwrap();
        let footer_start = buf.len() - 2;
        assert_eq!(&buf[footer_start..], &FOOTER_OUTGOING);
    }

    /// Opcode is preserved in the frame.
    #[test]
    fn test_opcode_preserved() {
        let crypto = JvCryption::new();
        let aes = AesCryption::new();
        for opcode in [0x00, 0x42, 0x99, 0xFF] {
            let pkt = Packet::new(opcode);
            let mut buf = Vec::new();
            build_frame(&crypto, 0, &pkt, &mut buf, &aes).unwrap();
            assert_eq!(buf[4], opcode);
        }
    }

    /// Data bytes are preserved in the frame payload.
    #[test]
    fn test_data_preserved() {
        let crypto = JvCryption::new();
        let aes = AesCryption::new();
        let mut pkt = Packet::new(0x01);
        pkt.data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let mut buf = Vec::new();
        build_frame(&crypto, 0, &pkt, &mut buf, &aes).unwrap();
        assert_eq!(&buf[5..9], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    /// Buffer is reusable after clear.
    #[test]
    fn test_buffer_reuse() {
        let crypto = JvCryption::new();
        let aes = AesCryption::new();
        let mut buf = Vec::with_capacity(4096);
        let pkt = Packet::new(0x10);
        build_frame(&crypto, 0, &pkt, &mut buf, &aes).unwrap();
        let first_len = buf.len();
        buf.clear();
        build_frame(&crypto, 0, &pkt, &mut buf, &aes).unwrap();
        assert_eq!(buf.len(), first_len);
        assert!(buf.capacity() >= 4096);
    }

    /// OUTGOING_CRYPTO_MAGIC constant is 0x1EFC.
    #[test]
    fn test_crypto_magic_value() {
        assert_eq!(OUTGOING_CRYPTO_MAGIC, 0x1EFC);
    }
}
