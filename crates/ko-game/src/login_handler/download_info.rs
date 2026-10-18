//! LS_DOWNLOADINFO_REQ (0x02) handler — patch download information.
//! The launcher sends its current client version. The server compares it
//! against the client_version database table and returns a list of patch
//! files to download. If the client is up-to-date, an empty list is sent.
//! ## Request (Client → Server)
//! | Offset | Type  | Description                    |
//! |--------|-------|--------------------------------|
//! | 0      | u16le | Client's current version       |
//! ## Response (Server → Client)
//! | Offset | Type   | Description                   |
//! |--------|--------|-------------------------------|
//! | 0      | string | Download URL (u16le len + bytes) |
//! | N      | string | Download path (u16le len + bytes) |
//! | M      | u16le  | Patch file count              |
//! | M+2    | string | Patch filename (repeated)     |

use ko_db::repositories::client_version::ClientVersionRepository;
use ko_protocol::{LoginOpcode, Packet, PacketReader};

use crate::login_session::LoginSession;

/// Handle LS_DOWNLOADINFO_REQ from the launcher.
/// Queries the client_version table for patches newer than the client's
/// version and returns them as a list of filenames to download.
pub async fn handle(session: &mut LoginSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let client_version = reader.read_u16().unwrap_or(0);

    let server_version = session.config().version;
    let download_url = session.config().ftp_url.clone();
    let download_path = session.config().ftp_path.clone();

    tracing::info!(
        "[{}] Patch check: client={}, server={}",
        session.addr(),
        client_version,
        server_version
    );

    let mut response = Packet::new(LoginOpcode::LsDownloadInfoReq as u8);
    response.write_string(&download_url);
    response.write_string(&download_path);

    // Always report "up to date" — our v2599 client connects to our server
    // even when game_version differs. Patch system not yet implemented.
    {
        response.write_u16(0);
        tracing::info!(
            "[{}] Client up to date (forced), client={}, server={}",
            session.addr(),
            client_version,
            server_version
        );
    }
    if false {
        // Query DB for patches after client version
        let repo = ClientVersionRepository::new(session.pool());
        match repo.get_patches_after(client_version as i16).await {
            Ok(patches) => {
                let count = patches.len().min(u16::MAX as usize) as u16;
                response.write_u16(count);
                for patch in &patches {
                    response.write_string(&patch.filename);
                    tracing::info!(
                        "[{}] Patch: v{} → {}",
                        session.addr(),
                        patch.version,
                        patch.filename
                    );
                }
                tracing::info!(
                    "[{}] Sent {} patches (url={}{})",
                    session.addr(),
                    count,
                    download_url,
                    download_path
                );
            }
            Err(e) => {
                tracing::error!("[{}] DB error querying patches: {}", session.addr(), e);
                response.write_u16(0);
            }
        }
    }

    session.send_packet(&response).await?;
    Ok(())
}
