//! GameGuard protocol packet structures.
//!
//! Opcodes 0xFA-0xFE for GameGuard client-server communication.
//! These are separate from the native KO protocol and do not interfere
//! with existing game logic.

use crate::packet::PacketReader;

/// GameGuard protocol version
pub const GG_VERSION: u16 = 0x0100;

/// UDP heartbeat port
pub const GG_HB_PORT: u16 = 7777;

/// Default heartbeat interval (seconds)
pub const GG_HB_INTERVAL: u16 = 10;

/// Heartbeat timeout (seconds)
pub const GG_HB_TIMEOUT_SECS: u64 = 60;

// ── Key Exchange (0xFE) ─────────────────────────────────────────────

/// Client → Server: KEY_EXCHANGE request
pub struct GgKeyExchangeRequest {
    pub gg_version: u16,
    pub client_public_key: [u8; 32],
    pub flags: u8,
}

impl GgKeyExchangeRequest {
    pub fn parse(reader: &mut PacketReader) -> Option<Self> {
        let sub = reader.read_u8()?;
        if sub != 0x01 {
            return None;
        }
        let gg_version = reader.read_u16()?;
        let remaining = reader.read_remaining();
        if remaining.len() < 33 {
            return None;
        }
        let mut client_public_key = [0u8; 32];
        client_public_key.copy_from_slice(&remaining[..32]);
        let flags = remaining[32];
        Some(Self {
            gg_version,
            client_public_key,
            flags,
        })
    }
}

/// Server → Client: KEY_EXCHANGE response
pub struct GgKeyExchangeResponse {
    pub result: u8,
    pub server_public_key: [u8; 32],
    pub challenge_nonce: [u8; 16],
    pub hb_port: u16,
    pub hb_interval: u16,
}

impl GgKeyExchangeResponse {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(55);
        buf.push(0xFE); // opcode
        buf.push(0x02); // sub: SERVER_HELLO
        buf.push(self.result);
        buf.extend_from_slice(&self.server_public_key);
        buf.extend_from_slice(&self.challenge_nonce);
        buf.extend_from_slice(&self.hb_port.to_le_bytes());
        buf.extend_from_slice(&self.hb_interval.to_le_bytes());
        buf
    }
}

// ── Auth (0xFD) ─────────────────────────────────────────────────────

/// Client → Server: AUTH request
pub struct GgAuthRequest {
    pub hmac: [u8; 32],
    pub text_crc: u32,
}

impl GgAuthRequest {
    pub fn parse(reader: &mut PacketReader) -> Option<Self> {
        let sub = reader.read_u8()?;
        if sub != 0x01 {
            return None;
        }
        let remaining = reader.read_remaining();
        if remaining.len() < 36 {
            return None;
        }
        let mut hmac = [0u8; 32];
        hmac.copy_from_slice(&remaining[..32]);
        let text_crc =
            u32::from_le_bytes([remaining[32], remaining[33], remaining[34], remaining[35]]);
        Some(Self { hmac, text_crc })
    }
}

/// Server → Client: AUTH result
pub struct GgAuthResult {
    pub result: u8,
    pub session_id: u32,
}

impl GgAuthResult {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(9);
        buf.push(0xFD); // opcode
        buf.push(0x02); // sub: AUTH_RESULT
        buf.push(self.result);
        buf.extend_from_slice(&self.session_id.to_le_bytes());
        buf.extend_from_slice(&[0, 0]); // reserved
        buf
    }
}

// ── UDP Heartbeat (0xFC) ────────────────────────────────────────────

/// UDP heartbeat datagram (84 bytes minimum)
pub struct GgHeartbeat {
    pub session_id: u32,
    pub hb_count: u32,
    pub timestamp: u64,
    pub text_crc: u32,
    pub flags: u8,
    pub nonce: [u8; 24],
    pub encrypted: Vec<u8>,
}

impl GgHeartbeat {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 84 {
            return None;
        }
        let session_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let hb_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let timestamp = u64::from_le_bytes([
            data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
        ]);
        let text_crc = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        let flags = data[20];
        let mut nonce = [0u8; 24];
        nonce.copy_from_slice(&data[24..48]);
        let encrypted = data[48..].to_vec();

        Some(Self {
            session_id,
            hb_count,
            timestamp,
            text_crc,
            flags,
            nonce,
            encrypted,
        })
    }
}

// ── Challenge/Response (0xFB/0xFA) ──────────────────────────────────

/// Server → Client: Challenge
pub struct GgChallenge {
    pub challenge_type: u8,
    pub challenge_id: u16,
    pub challenge_data: [u8; 32],
    pub timeout_sec: u8,
}

impl GgChallenge {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(37);
        buf.push(0xFB);
        buf.push(self.challenge_type);
        buf.extend_from_slice(&self.challenge_id.to_le_bytes());
        buf.extend_from_slice(&self.challenge_data);
        buf.push(self.timeout_sec);
        buf
    }
}

/// Client → Server: Challenge response
pub struct GgChallengeResponse {
    pub challenge_type: u8,
    pub challenge_id: u16,
    pub response_data: [u8; 32],
    pub status: u8,
}

impl GgChallengeResponse {
    pub fn parse(reader: &mut PacketReader) -> Option<Self> {
        let challenge_type = reader.read_u8()?;
        let challenge_id = reader.read_u16()?;
        let remaining = reader.read_remaining();
        if remaining.len() < 33 {
            return None;
        }
        let mut response_data = [0u8; 32];
        response_data.copy_from_slice(&remaining[..32]);
        let status = remaining[32];
        Some(Self {
            challenge_type,
            challenge_id,
            response_data,
            status,
        })
    }
}
