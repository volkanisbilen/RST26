//! S→C heartbeat probe system — sends plaintext 0x02 probes to all in-game clients.
//!
//! ## PCAP verified (session 5, original server 185.81.239.132:15001)
//!
//! The original game server sends S→C 0x02 packets as **PLAINTEXT** (no AES flag),
//! even when AES encryption is active. The client detects plaintext by checking
//! `payload[0] != 0x01`.
//!
//! ```text
//! Wire: AA 55 11 00 02 [16 random bytes] 55 AA
//! Crypto: NONE
//! Frequency: ~14 seconds (2013 packets / 28800 seconds)
//! ```
//!
//! | payload_len | count | description |
//! |-------------|-------|-------------|
//! | 17          | 1959  | 1 AES block (16B random + opcode) |
//! | 65          | 28    | 4 blocks (64B + opcode) |
//! | 225         | 12    | larger data |
//! | 33          | 5     | 2 blocks |
//!
//! The most common form is 17 bytes (opcode + 16 random bytes).

use std::sync::Arc;
use std::time::Duration;

use ko_protocol::Packet;
use rand::Rng;

use crate::world::WorldState;

/// Heartbeat probe interval in seconds (PCAP: ~14s average).
const HEARTBEAT_PROBE_INTERVAL_SECS: u64 = 14;

/// Heartbeat probe data size in bytes (PCAP: most common = 16).
const HEARTBEAT_PROBE_DATA_LEN: usize = 16;

/// Opcode for S→C heartbeat probe (PCAP verified: 0x02).
const HEARTBEAT_PROBE_OPCODE: u8 = 0x02;

/// Start the S→C heartbeat probe background task.
///
/// Sends a plaintext 0x02 packet with 16 random bytes to all in-game sessions
/// every ~14 seconds. The packet bypasses AES encryption.
pub fn start_heartbeat_probe_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_secs(HEARTBEAT_PROBE_INTERVAL_SECS));
        loop {
            interval.tick().await;
            send_heartbeat_probes(&world);
        }
    })
}

/// Send an AES-encrypted heartbeat probe to all in-game sessions.
///
/// v2600 client does NOT check the AES flag byte (0x01). After 0x2B key
/// exchange, it decrypts EVERY incoming packet with AES-CBC. Sending
/// plaintext causes PKCS7 padding error (-25) and crashes the client.
fn send_heartbeat_probes(world: &Arc<WorldState>) {
    // Generate 16 random bytes for this tick
    let mut probe_data = [0u8; HEARTBEAT_PROBE_DATA_LEN];
    rand::thread_rng().fill(&mut probe_data);

    // MUST use AES-encrypted packet — v2600 client decrypts everything after 0x2B
    let mut packet = Packet::new(HEARTBEAT_PROBE_OPCODE);
    packet.data.extend_from_slice(&probe_data);

    let packet = Arc::new(packet);

    // Send to all in-game sessions (broadcast_to_all skips sessions without character)
    world.broadcast_to_all(packet, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_probe_packet_format() {
        let mut pkt = Packet::new(HEARTBEAT_PROBE_OPCODE);
        pkt.data
            .extend_from_slice(&[0xAB; HEARTBEAT_PROBE_DATA_LEN]);

        assert_eq!(pkt.opcode, 0x02);
        assert!(!pkt.plaintext); // MUST be encrypted — v2600 decrypts everything
        assert_eq!(pkt.data.len(), 16);
    }

    #[test]
    fn test_heartbeat_probe_interval() {
        assert_eq!(HEARTBEAT_PROBE_INTERVAL_SECS, 14);
    }

    #[test]
    fn test_heartbeat_probe_opcode() {
        assert_eq!(HEARTBEAT_PROBE_OPCODE, 0x02);
    }
}
