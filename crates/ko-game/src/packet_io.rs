//! Shared packet I/O helpers used by both Game Server and Login Server sessions.
//!
//! Eliminates duplication of packet framing, encryption, and sequence logic
//! between `ClientSession` (Game Server) and `LoginSession` (Login Server).

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::trace;

use ko_protocol::aes_crypt::AES_FLAG;
use ko_protocol::packet::{
    FOOTER_INCOMING, FOOTER_OUTGOING, HEADER_INCOMING, HEADER_OUTGOING, MAX_PACKET_SIZE,
    OUTGOING_CRYPTO_MAGIC,
};
use ko_protocol::{AesCryption, JvCryption, Packet};

/// Read a complete packet from a TcpStream (convenience wrapper).
///
/// Delegates to the generic [`read_packet_from`].
pub async fn read_packet(
    stream: &mut TcpStream,
    crypto: &JvCryption,
    sequence: &mut u32,
) -> anyhow::Result<Packet> {
    read_packet_from(stream, crypto, sequence).await
}

/// Read a complete packet from any async reader.
///
/// Wire format (inbound): `[0x55 0xAA] [len: u16le] [payload] [0xAA 0x55]`
///
/// If encryption is enabled, the payload is decrypted and CRC32-validated.
pub async fn read_packet_from<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    crypto: &JvCryption,
    sequence: &mut u32,
) -> anyhow::Result<Packet> {
    // Read header (2 bytes): 0x55 0xAA
    let mut header = [0u8; 2];
    reader.read_exact(&mut header).await?;
    if header != HEADER_INCOMING {
        anyhow::bail!("invalid header: {:02X}{:02X}", header[0], header[1]);
    }

    // Read length (u16 LE)
    let mut len_buf = [0u8; 2];
    reader.read_exact(&mut len_buf).await?;
    let payload_len = u16::from_le_bytes(len_buf) as usize;

    if payload_len == 0 || payload_len > MAX_PACKET_SIZE {
        anyhow::bail!("invalid payload length: {}", payload_len);
    }

    // Read payload
    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload).await?;

    // Read footer (2 bytes): 0xAA 0x55
    let mut footer = [0u8; 2];
    reader.read_exact(&mut footer).await?;
    if footer != FOOTER_INCOMING {
        anyhow::bail!("invalid footer: {:02X}{:02X}", footer[0], footer[1]);
    }

    // Decrypt if encryption is enabled
    if crypto.is_enabled() {
        let decrypted_len = crypto
            .decrypt_with_crc32(&mut payload)
            .ok_or_else(|| anyhow::anyhow!("CRC32 check failed"))?;

        // First 4 bytes = sequence (u32le), then opcode, then data
        if decrypted_len < 5 {
            anyhow::bail!("encrypted payload too short: {} bytes", decrypted_len);
        }

        let seq = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        *sequence += 1;
        if seq != *sequence {
            anyhow::bail!("sequence mismatch: expected {}, got {}", *sequence, seq);
        }

        let opcode = payload[4];
        let data = payload[5..decrypted_len].to_vec();
        trace!(
            "read_packet: opcode=0x{:02X}, data_len={}",
            opcode,
            data.len()
        );
        Ok(Packet::with_data(opcode, data))
    } else {
        // Unencrypted: first byte = opcode, rest = data
        let opcode = payload[0];
        let data = payload[1..].to_vec();
        trace!(
            "read_packet: opcode=0x{:02X}, data_len={} (unencrypted)",
            opcode,
            data.len()
        );
        Ok(Packet::with_data(opcode, data))
    }
}

/// Send a packet to the client.
///
/// Wire format (outbound): `[0xAA 0x55] [len: u16le] [payload] [0x55 0xAA]`
///
/// If encryption is enabled, the payload is built as:
/// `[0x1EFC: u16le] [sequence: u16le] [0x00] [opcode] [data]`
/// then encrypted in-place.
pub async fn send_packet(
    stream: &mut TcpStream,
    crypto: &JvCryption,
    sequence: u32,
    packet: &Packet,
) -> anyhow::Result<()> {
    let payload = if crypto.is_enabled() {
        let plain_len = 2 + 2 + 1 + 1 + packet.data.len();
        let mut buf = Vec::with_capacity(plain_len);
        buf.extend_from_slice(&OUTGOING_CRYPTO_MAGIC.to_le_bytes());
        buf.extend_from_slice(&(sequence as u16).to_le_bytes());
        buf.push(0x00);
        buf.push(packet.opcode);
        buf.extend_from_slice(&packet.data);

        // Encrypt in-place
        crypto.encrypt(&mut buf);
        buf
    } else {
        let mut buf = Vec::with_capacity(1 + packet.data.len());
        buf.push(packet.opcode);
        buf.extend_from_slice(&packet.data);
        buf
    };

    // Guard against payload exceeding u16 frame length (protocol limit).
    if payload.len() > u16::MAX as usize {
        anyhow::bail!(
            "Packet payload too large ({} bytes, max {})",
            payload.len(),
            u16::MAX
        );
    }

    // Frame: [header] [len: u16le] [payload] [footer]
    let frame_len = 2 + 2 + payload.len() + 2;
    let mut frame = Vec::with_capacity(frame_len);
    frame.extend_from_slice(&HEADER_OUTGOING);
    frame.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    frame.extend_from_slice(&payload);
    frame.extend_from_slice(&FOOTER_OUTGOING);

    stream.write_all(&frame).await?;
    Ok(())
}

/// Read a complete AES-encrypted packet from any async reader.
///
/// Wire format: `[0xAA 0x55] [len: u16le] [0x01 | AES_CBC(opcode | data)] [0x55 0xAA]`
///
/// The flag byte (0x01) indicates AES mode. The remaining bytes are
/// AES-128-CBC ciphertext with PKCS7 padding. After decryption, the
/// first byte is the opcode and the rest is data.
pub async fn read_packet_aes<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    aes: &AesCryption,
) -> anyhow::Result<Packet> {
    read_packet_aes_inner(reader, aes, true).await
}

/// Read AES packet without stripping seq counter (login server).
///
/// Login server C2S plaintext: `[opcode:u8][data...]` (no seq counter).
pub async fn read_packet_aes_no_seq<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    aes: &AesCryption,
) -> anyhow::Result<Packet> {
    read_packet_aes_inner(reader, aes, false).await
}

/// Read a JvCryption + AES double-encrypted packet (login server C2S after 0xF2).
///
/// Client encryption chain (GameGuard `joe_encrypt_on=1`):
///   1. Build `[opcode | data]`
///   2. JvCryption: prepend `[seq: u32le]`, append CRC32, XOR encrypt
///   3. AES-128-CBC encrypt
///   4. Wire: `[AA 55] [len] [0x01 flag] [AES ciphertext] [55 AA]`
///
/// Server decrypt reverses this: AES decrypt → JvCryption decrypt + CRC verify → skip seq.
pub async fn read_packet_jv_aes<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    aes: &AesCryption,
    jv: &JvCryption,
) -> anyhow::Result<Packet> {
    // Read frame header
    let mut header = [0u8; 2];
    reader.read_exact(&mut header).await?;
    if header != HEADER_INCOMING {
        anyhow::bail!("invalid header: {:02X}{:02X}", header[0], header[1]);
    }

    let mut len_buf = [0u8; 2];
    reader.read_exact(&mut len_buf).await?;
    let payload_len = u16::from_le_bytes(len_buf) as usize;

    if !(2..=MAX_PACKET_SIZE).contains(&payload_len) {
        anyhow::bail!("invalid JV+AES payload length: {}", payload_len);
    }

    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload).await?;

    let mut footer = [0u8; 2];
    reader.read_exact(&mut footer).await?;
    if footer != FOOTER_INCOMING {
        anyhow::bail!("invalid footer: {:02X}{:02X}", footer[0], footer[1]);
    }

    // Check AES flag
    if payload[0] != AES_FLAG {
        let opcode = payload[0];
        let data = payload[1..].to_vec();
        return Ok(Packet::with_data(opcode, data));
    }

    // Step 1: AES decrypt
    let ciphertext = &payload[1..];
    let aes_plaintext = match aes.decrypt(ciphertext) {
        Some(pt) => pt,
        None => anyhow::bail!("AES decryption failed (ct_len={})", ciphertext.len()),
    };

    // Step 2: JvCryption decrypt + CRC32 verify
    let mut jv_buf = aes_plaintext;
    let jv_payload_len = jv
        .decrypt_with_crc32(&mut jv_buf)
        .ok_or_else(|| anyhow::anyhow!("JvCryption CRC32 check failed"))?;

    // Step 3: Skip JvCryption seq (u32le = 4 bytes) → opcode + data
    if jv_payload_len < 5 {
        anyhow::bail!(
            "JV+AES payload too short for seq+opcode: {} bytes",
            jv_payload_len
        );
    }
    let opcode = jv_buf[4];
    let data = jv_buf[5..jv_payload_len].to_vec();

    tracing::debug!(
        "JV+AES C2S: seq={} opcode=0x{:02X} data_len={}",
        u32::from_le_bytes([jv_buf[0], jv_buf[1], jv_buf[2], jv_buf[3]]),
        opcode,
        data.len()
    );

    Ok(Packet::with_data(opcode, data))
}

/// Inner AES packet reader. `strip_seq` controls whether the first byte
/// is treated as a sequence counter (game server) or as the opcode (login server).
async fn read_packet_aes_inner<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    aes: &AesCryption,
    strip_seq: bool,
) -> anyhow::Result<Packet> {
    // Read header (2 bytes): 0xAA 0x55
    let mut header = [0u8; 2];
    reader.read_exact(&mut header).await?;
    if header != HEADER_INCOMING {
        anyhow::bail!("invalid header: {:02X}{:02X}", header[0], header[1]);
    }

    // Read length (u16 LE)
    let mut len_buf = [0u8; 2];
    reader.read_exact(&mut len_buf).await?;
    let payload_len = u16::from_le_bytes(len_buf) as usize;

    if !(2..=MAX_PACKET_SIZE).contains(&payload_len) {
        anyhow::bail!("invalid AES payload length: {}", payload_len);
    }

    // Read payload (flag byte + ciphertext)
    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload).await?;

    // Read footer (2 bytes): 0x55 0xAA
    let mut footer = [0u8; 2];
    reader.read_exact(&mut footer).await?;
    if footer != FOOTER_INCOMING {
        anyhow::bail!("invalid footer: {:02X}{:02X}", footer[0], footer[1]);
    }

    if payload[0] != AES_FLAG {
        // Not AES — treat as unencrypted fallback.
        let opcode = payload[0];
        let data = payload[1..].to_vec();
        trace!(
            "read_packet_aes: opcode=0x{:02X}, data_len={} (unencrypted fallback)",
            opcode,
            data.len()
        );
        return Ok(Packet::with_data(opcode, data));
    }

    // AES decrypt: skip flag byte, decrypt the rest.
    let ciphertext = &payload[1..];

    let plaintext = match aes.decrypt(ciphertext) {
        Some(pt) => pt,
        None => anyhow::bail!("AES decryption failed (len={})", ciphertext.len()),
    };

    if plaintext.is_empty() {
        anyhow::bail!("AES decrypted payload is empty");
    }

    if strip_seq {
        // Game server C2S: [xor_seq:u8] [opcode:u8] [data...]
        if plaintext.len() < 2 {
            anyhow::bail!(
                "AES decrypted payload too short for seq+opcode: {} bytes",
                plaintext.len()
            );
        }
        let seq = plaintext[0];
        let opcode = plaintext[1];
        let data = plaintext[2..].to_vec();
        tracing::debug!(
            "AES C2S decrypted: seq={} opcode=0x{:02X} data_len={}",
            seq,
            opcode,
            data.len()
        );
        Ok(Packet::with_data(opcode, data))
    } else {
        // Login server C2S: [opcode:u8] [data...] (no seq counter)
        let opcode = plaintext[0];
        let data = plaintext[1..].to_vec();
        tracing::debug!(
            "AES C2S decrypted (no seq): opcode=0x{:02X} data_len={}",
            opcode,
            data.len()
        );
        Ok(Packet::with_data(opcode, data))
    }
}

/// Send an AES-encrypted packet to the client.
///
/// Wire format: `[0xAA 0x55] [len: u16le] [0x01 | AES_CBC(opcode + data)] [0x55 0xAA]`
pub async fn send_packet_aes<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    aes: &AesCryption,
    packet: &Packet,
) -> anyhow::Result<()> {
    let mut plaintext = Vec::with_capacity(1 + packet.data.len());
    plaintext.push(packet.opcode);
    plaintext.extend_from_slice(&packet.data);

    let ciphertext = aes.encrypt(&plaintext);

    let payload_len = 1 + ciphertext.len();
    if payload_len > u16::MAX as usize {
        anyhow::bail!(
            "AES packet payload too large ({} bytes, max {})",
            payload_len,
            u16::MAX
        );
    }

    let frame_len = 2 + 2 + payload_len + 2;
    let mut frame = Vec::with_capacity(frame_len);
    frame.extend_from_slice(&HEADER_OUTGOING);
    frame.extend_from_slice(&(payload_len as u16).to_le_bytes());
    frame.push(AES_FLAG);
    frame.extend_from_slice(&ciphertext);
    frame.extend_from_slice(&FOOTER_OUTGOING);

    // ── Wire-level hex dump (Sprint 15 debug) ──────────────────────
    {
        let wire: String = frame
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        tracing::info!(
            "WIRE S2C [AES] pre-auth opcode=0x{:02X} payload={} wire_len={}: {}",
            packet.opcode,
            payload_len,
            frame.len(),
            wire,
        );
    }

    writer.write_all(&frame).await?;
    Ok(())
}

/// Send a packet as PLAINTEXT (bypassing AES), used for RE-KEY (0xF2) mid-session.
///
/// PCAP verified: Original server sends 0xF2 RE-KEY as plaintext even when
/// AES is active. The client detects plaintext by checking if payload[0] != 0x01.
pub async fn send_packet_plaintext<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    packet: &Packet,
) -> anyhow::Result<()> {
    let mut payload = Vec::with_capacity(1 + packet.data.len());
    payload.push(packet.opcode);
    payload.extend_from_slice(&packet.data);

    let frame_len = 2 + 2 + payload.len() + 2;
    let mut frame = Vec::with_capacity(frame_len);
    frame.extend_from_slice(&HEADER_OUTGOING);
    frame.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    frame.extend_from_slice(&payload);
    frame.extend_from_slice(&FOOTER_OUTGOING);

    writer.write_all(&frame).await?;
    Ok(())
}

/// Send a JvCryption + AES double-encrypted packet (login server S2C after 0xF2).
///
/// Client recv path (IDA sub_8339D0 + sub_833C60):
///   1. AES-128-CBC decrypt
///   2. JvCryption XOR decrypt (sub_4EEF20 — NO CRC32 check)
///   3. Check magic 0x1EFC at offset 0-1
///   4. Strip 5 bytes: [magic:2][seq:u16:2][zero:1]
///   5. Result = [opcode][data]
///
/// Server encryption chain (reverse of client recv):
///   1. Build `[0x1EFC: u16le] [seq: u16le] [0x00] [opcode] [data]`
///   2. JvCryption XOR encrypt (NO CRC32 — same as send_packet)
///   3. AES-128-CBC encrypt
///   4. Wire: `[AA 55] [len] [0x01 flag] [AES ciphertext] [55 AA]`
pub async fn send_packet_jv_aes<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    aes: &AesCryption,
    jv: &JvCryption,
    jv_seq: &mut u32,
    packet: &Packet,
) -> anyhow::Result<()> {
    // Step 1: Build [magic:0x1EFC][seq:u16le][0x00][opcode][data]
    // Same format as send_packet (non-AES JvCryption path)
    *jv_seq += 1;
    let plain_len = 2 + 2 + 1 + 1 + packet.data.len();
    let mut buf = Vec::with_capacity(plain_len);
    buf.extend_from_slice(&OUTGOING_CRYPTO_MAGIC.to_le_bytes());
    buf.extend_from_slice(&(*jv_seq as u16).to_le_bytes());
    buf.push(0x00);
    buf.push(packet.opcode);
    buf.extend_from_slice(&packet.data);

    // Step 2: JvCryption XOR encrypt (NO CRC32 — sub_4EEF20, not sub_4EEFA0)
    jv.encrypt(&mut buf);

    // Step 3: AES encrypt
    let ciphertext = aes.encrypt(&buf);

    // Step 4: Frame with AES flag
    let payload_len = 1 + ciphertext.len();
    if payload_len > u16::MAX as usize {
        anyhow::bail!(
            "JV+AES packet payload too large ({} bytes, max {})",
            payload_len,
            u16::MAX
        );
    }

    let frame_len = 2 + 2 + payload_len + 2;
    let mut frame = Vec::with_capacity(frame_len);
    frame.extend_from_slice(&HEADER_OUTGOING);
    frame.extend_from_slice(&(payload_len as u16).to_le_bytes());
    frame.push(AES_FLAG);
    frame.extend_from_slice(&ciphertext);
    frame.extend_from_slice(&FOOTER_OUTGOING);

    writer.write_all(&frame).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Incoming header is [0xAA, 0x55] and footer is [0x55, 0xAA].
    #[test]
    fn test_incoming_header_footer() {
        assert_eq!(HEADER_INCOMING, [0xAA, 0x55]);
        assert_eq!(FOOTER_INCOMING, [0x55, 0xAA]);
    }

    /// Outgoing header/footer match incoming (same wire bytes).
    #[test]
    fn test_outgoing_matches_incoming() {
        assert_eq!(HEADER_OUTGOING, HEADER_INCOMING);
        assert_eq!(FOOTER_OUTGOING, FOOTER_INCOMING);
    }

    /// MAX_PACKET_SIZE is 64 KiB.
    #[test]
    fn test_max_packet_size() {
        assert_eq!(MAX_PACKET_SIZE, 65536);
        assert_eq!(MAX_PACKET_SIZE, u16::MAX as usize + 1);
    }

    /// OUTGOING_CRYPTO_MAGIC is 0x1EFC.
    #[test]
    fn test_crypto_magic() {
        assert_eq!(OUTGOING_CRYPTO_MAGIC, 0x1EFC);
        // LE bytes: 0xFC, 0x1E
        let bytes = OUTGOING_CRYPTO_MAGIC.to_le_bytes();
        assert_eq!(bytes, [0xFC, 0x1E]);
    }

    /// Read unencrypted packet from byte buffer.
    #[tokio::test]
    async fn test_read_unencrypted_packet() {
        // Build a wire frame: header + len + [opcode + data] + footer
        let mut wire = Vec::new();
        wire.extend_from_slice(&HEADER_INCOMING);
        wire.extend_from_slice(&3u16.to_le_bytes()); // payload = 3 bytes
        wire.push(0x42); // opcode
        wire.extend_from_slice(&[0x01, 0x02]); // data
        wire.extend_from_slice(&FOOTER_INCOMING);

        let mut cursor = std::io::Cursor::new(wire);
        let crypto = JvCryption::new();
        let mut seq = 0u32;
        let pkt = read_packet_from(&mut cursor, &crypto, &mut seq)
            .await
            .unwrap();
        assert_eq!(pkt.opcode, 0x42);
        assert_eq!(pkt.data, vec![0x01, 0x02]);
    }

    // ── Sprint 940: Additional coverage ──────────────────────────────

    /// Invalid header rejects packet.
    #[tokio::test]
    async fn test_read_invalid_header() {
        let mut wire = Vec::new();
        wire.extend_from_slice(&[0xFF, 0xFF]); // bad header
        wire.extend_from_slice(&1u16.to_le_bytes());
        wire.push(0x01);
        wire.extend_from_slice(&FOOTER_INCOMING);

        let mut cursor = std::io::Cursor::new(wire);
        let crypto = JvCryption::new();
        let mut seq = 0u32;
        assert!(read_packet_from(&mut cursor, &crypto, &mut seq)
            .await
            .is_err());
    }

    /// Invalid footer rejects packet.
    #[tokio::test]
    async fn test_read_invalid_footer() {
        let mut wire = Vec::new();
        wire.extend_from_slice(&HEADER_INCOMING);
        wire.extend_from_slice(&1u16.to_le_bytes());
        wire.push(0x01);
        wire.extend_from_slice(&[0xFF, 0xFF]); // bad footer

        let mut cursor = std::io::Cursor::new(wire);
        let crypto = JvCryption::new();
        let mut seq = 0u32;
        assert!(read_packet_from(&mut cursor, &crypto, &mut seq)
            .await
            .is_err());
    }

    /// Zero payload length rejects packet.
    #[tokio::test]
    async fn test_read_zero_length() {
        let mut wire = Vec::new();
        wire.extend_from_slice(&HEADER_INCOMING);
        wire.extend_from_slice(&0u16.to_le_bytes());
        wire.extend_from_slice(&FOOTER_INCOMING);

        let mut cursor = std::io::Cursor::new(wire);
        let crypto = JvCryption::new();
        let mut seq = 0u32;
        assert!(read_packet_from(&mut cursor, &crypto, &mut seq)
            .await
            .is_err());
    }

    /// Opcode-only packet (1 byte payload, no data).
    #[tokio::test]
    async fn test_read_opcode_only() {
        let mut wire = Vec::new();
        wire.extend_from_slice(&HEADER_INCOMING);
        wire.extend_from_slice(&1u16.to_le_bytes());
        wire.push(0xAB);
        wire.extend_from_slice(&FOOTER_INCOMING);

        let mut cursor = std::io::Cursor::new(wire);
        let crypto = JvCryption::new();
        let mut seq = 0u32;
        let pkt = read_packet_from(&mut cursor, &crypto, &mut seq)
            .await
            .unwrap();
        assert_eq!(pkt.opcode, 0xAB);
        assert!(pkt.data.is_empty());
    }

    /// Frame overhead is 6 bytes (header 2 + len 2 + footer 2).
    #[test]
    fn test_frame_overhead() {
        assert_eq!(HEADER_OUTGOING.len() + 2 + FOOTER_OUTGOING.len(), 6);
    }

    // ── AES packet I/O tests ──────────────────────────────────────────

    /// Helper: build an AES-encrypted C2S wire frame.
    ///
    /// C2S plaintext: `[seq:u8][opcode:u8][data...]`
    /// Wire: `[AA 55][len][0x01 flag][AES(seq+opcode+data)][55 AA]`
    fn build_c2s_aes_wire_frame(aes: &AesCryption, seq: u8, opcode: u8, data: &[u8]) -> Vec<u8> {
        let mut plaintext = vec![seq, opcode];
        plaintext.extend_from_slice(data);
        let ciphertext = aes.encrypt(&plaintext);
        let payload_len = 1 + ciphertext.len(); // flag + ciphertext

        let mut wire = Vec::new();
        wire.extend_from_slice(&HEADER_INCOMING);
        wire.extend_from_slice(&(payload_len as u16).to_le_bytes());
        wire.push(AES_FLAG); // 0x01
        wire.extend_from_slice(&ciphertext);
        wire.extend_from_slice(&FOOTER_INCOMING);
        wire
    }

    /// C2S AES roundtrip: build C2S frame (with seq) → read_packet_aes → correct opcode+data.
    #[tokio::test]
    async fn test_aes_c2s_roundtrip() {
        let mut aes = AesCryption::new();
        aes.set_key(AesCryption::generate_key());
        aes.enable();

        let wire = build_c2s_aes_wire_frame(&aes, 1, 0x0C, &[0x01, 0x01]);
        let mut cursor = std::io::Cursor::new(wire);
        let pkt = read_packet_aes(&mut cursor, &aes).await.unwrap();
        assert_eq!(pkt.opcode, 0x0C);
        assert_eq!(pkt.data, vec![0x01, 0x01]);
    }

    /// C2S AES: seq counter is stripped, not included in opcode or data.
    #[tokio::test]
    async fn test_aes_c2s_seq_stripped() {
        let mut aes = AesCryption::new();
        aes.set_key(AesCryption::generate_key());
        aes.enable();

        // seq=42, opcode=0xC0 (ServerIndex), data=[0x01]
        let wire = build_c2s_aes_wire_frame(&aes, 42, 0xC0, &[0x01]);
        let mut cursor = std::io::Cursor::new(wire);
        let pkt = read_packet_aes(&mut cursor, &aes).await.unwrap();
        assert_eq!(pkt.opcode, 0xC0, "opcode must be 0xC0, not seq byte 42");
        assert_eq!(pkt.data, vec![0x01]);
    }

    /// C2S AES: full login flow sequence (seq=1 → Login 0x01).
    #[tokio::test]
    async fn test_aes_c2s_login_seq() {
        let mut aes = AesCryption::new();
        aes.set_key(AesCryption::generate_key());
        aes.enable();

        let login_data = b"testuser\x00testpass\x00";
        let wire = build_c2s_aes_wire_frame(&aes, 1, 0x01, login_data);
        let mut cursor = std::io::Cursor::new(wire);
        let pkt = read_packet_aes(&mut cursor, &aes).await.unwrap();
        assert_eq!(pkt.opcode, 0x01, "Login opcode");
        assert_eq!(&pkt.data, login_data);
    }

    /// AES frame has flag byte 0x01 at payload start.
    #[tokio::test]
    async fn test_aes_frame_flag_byte() {
        let mut aes = AesCryption::new();
        aes.set_key([0x41; 16]);
        aes.enable();

        let mut buf = Vec::new();
        let pkt = Packet::new(0x10);
        send_packet_aes(&mut buf, &aes, &pkt).await.unwrap();

        // buf = [AA 55] [len u16le] [0x01 | ciphertext] [55 AA]
        assert_eq!(&buf[0..2], &HEADER_OUTGOING);
        assert_eq!(buf[4], AES_FLAG); // flag byte
        let footer_start = buf.len() - 2;
        assert_eq!(&buf[footer_start..], &FOOTER_OUTGOING);
    }

    /// AES ciphertext is block-aligned (16-byte multiple).
    #[tokio::test]
    async fn test_aes_ciphertext_block_aligned() {
        let mut aes = AesCryption::new();
        aes.set_key([0x42; 16]);
        aes.enable();

        let mut buf = Vec::new();
        let pkt = Packet::new(0x0C); // opcode only = 1 byte plaintext
        send_packet_aes(&mut buf, &aes, &pkt).await.unwrap();

        // payload = flag(1) + ciphertext(16) = 17
        let payload_len = u16::from_le_bytes([buf[2], buf[3]]) as usize;
        let ct_len = payload_len - 1; // subtract flag byte
        assert_eq!(ct_len % 16, 0, "ciphertext must be 16-byte aligned");
    }

    /// C2S AES: opcode-only packet (no data, just seq+opcode).
    #[tokio::test]
    async fn test_aes_c2s_opcode_only() {
        let mut aes = AesCryption::new();
        aes.set_key(AesCryption::generate_key());
        aes.enable();

        let wire = build_c2s_aes_wire_frame(&aes, 5, 0xFF, &[]);
        let mut cursor = std::io::Cursor::new(wire);
        let decoded = read_packet_aes(&mut cursor, &aes).await.unwrap();
        assert_eq!(decoded.opcode, 0xFF);
        assert!(decoded.data.is_empty());
    }

    /// AES fallback: non-encrypted packet (no 0x01 flag) still works.
    #[tokio::test]
    async fn test_aes_fallback_unencrypted() {
        let aes = AesCryption::new(); // disabled

        // Build unencrypted frame
        let mut wire = Vec::new();
        wire.extend_from_slice(&HEADER_INCOMING);
        wire.extend_from_slice(&3u16.to_le_bytes());
        wire.push(0x42); // opcode (not 0x01 flag)
        wire.extend_from_slice(&[0x01, 0x02]);
        wire.extend_from_slice(&FOOTER_INCOMING);

        let mut cursor = std::io::Cursor::new(wire);
        let pkt = read_packet_aes(&mut cursor, &aes).await.unwrap();
        assert_eq!(pkt.opcode, 0x42);
        assert_eq!(pkt.data, vec![0x01, 0x02]);
    }
}
