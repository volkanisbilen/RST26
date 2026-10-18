//! Knight Online packet framing — wire format encode/decode.
//! ## Wire Format
//! Both directions use the same framing on the wire:
//! ```text
//! [0xAA 0x55] [len: u16le] [encrypted payload] [0x55 0xAA]
//! ```
//! The C++ code checks `if (header != 0x55AA)` — on little-endian x86
//! the uint16 value `0x55AA` maps to wire bytes `[0xAA, 0x55]`.
//! Server also sends `"\xaa\x55"` literally. Same framing both ways.
//! **Encrypted payload (after decrypt):**
//! ```text
//! [sequence: u32le] [opcode: u8] [data ...]  [crc32: u32le]
//! ```
//! **Unencrypted payload:**
//! ```text
//! [opcode: u8] [data ...]
//! ```

/// Inbound (client→server) header marker.
/// C++ checks `header != 0x55AA` as uint16 — on LE that's wire bytes `[0xAA, 0x55]`.
pub const HEADER_INCOMING: [u8; 2] = [0xAA, 0x55];
/// Inbound (client→server) footer marker.
/// C++ checks `footer != 0xAA55` as uint16 — on LE that's wire bytes `[0x55, 0xAA]`.
pub const FOOTER_INCOMING: [u8; 2] = [0x55, 0xAA];

/// Outbound (server→client) header marker.
pub const HEADER_OUTGOING: [u8; 2] = [0xAA, 0x55];
/// Outbound (server→client) footer marker.
pub const FOOTER_OUTGOING: [u8; 2] = [0x55, 0xAA];

/// Magic header prepended to encrypted outbound payloads before encryption.
pub const OUTGOING_CRYPTO_MAGIC: u16 = 0x1EFC;

/// Maximum packet size to accept.
pub const MAX_PACKET_SIZE: usize = 65536;

/// Decode a byte slice from Windows-1254 (Turkish) encoding to a UTF-8 String.
/// ASCII bytes pass through unchanged. Non-ASCII bytes are mapped to the
/// correct Unicode code points per the Windows-1254 / ISO 8859-9 spec.
fn decode_win1254(bytes: &[u8]) -> String {
    // Fast path: if valid UTF-8, return as-is (covers pure ASCII and
    // strings already stored as UTF-8 in the DB).
    if let Ok(valid) = std::str::from_utf8(bytes) {
        return valid.to_owned();
    }
    // Slow path: decode each byte as Windows-1254
    bytes.iter().map(|&b| win1254_to_unicode(b)).collect()
}

/// Map a Windows-1254 byte to its Unicode character.
fn win1254_to_unicode(b: u8) -> char {
    match b {
        // ASCII range
        0x00..=0x7F => b as char,
        // Windows-1254 specific (0x80-0x9F)
        0x80 => '€',
        0x82 => '‚',
        0x83 => 'ƒ',
        0x84 => '„',
        0x85 => '…',
        0x86 => '†',
        0x87 => '‡',
        0x88 => 'ˆ',
        0x89 => '‰',
        0x8A => 'Š',
        0x8B => '‹',
        0x8C => 'Œ',
        0x91 => '\u{2018}', // '
        0x92 => '\u{2019}', // '
        0x93 => '\u{201C}', // "
        0x94 => '\u{201D}', // "
        0x95 => '•',
        0x96 => '–',
        0x97 => '—',
        0x98 => '˜',
        0x99 => '™',
        0x9A => 'š',
        0x9B => '›',
        0x9C => 'œ',
        0x9F => 'Ÿ',
        // ISO 8859-9 Turkish replacements (differs from Latin-1)
        0xD0 => 'Ğ', // Latin-1 has Ð
        0xDD => 'İ', // Latin-1 has Ý
        0xDE => 'Ş', // Latin-1 has Þ
        0xF0 => 'ğ', // Latin-1 has ð
        0xFD => 'ı', // Latin-1 has ý
        0xFE => 'ş', // Latin-1 has þ
        // Undefined positions → replacement char
        0x81 | 0x8D | 0x8E | 0x8F | 0x90 | 0x9D | 0x9E => '\u{FFFD}',
        // Rest of Latin-1 supplement (same as Unicode)
        _ => b as char,
    }
}

/// Map a Unicode char to a Windows-1254 byte value.
/// Returns Some(byte) if the char can be encoded, None otherwise.
fn unicode_to_win1254(c: char) -> Option<u8> {
    let cp = c as u32;
    // ASCII range — identical in all encodings
    if cp <= 0x7F {
        return Some(cp as u8);
    }
    // Latin-1 supplement (U+00A0-U+00FF) — same as Windows-1254
    // except positions replaced by Turkish chars (handled below via ISO 8859-9)
    // ISO 8859-9 replaces 6 Icelandic chars with Turkish:
    //   0xD0: Ð → Ğ, 0xDD: Ý → İ, 0xDE: Þ → Ş
    //   0xF0: ð → ğ, 0xFD: ý → ı, 0xFE: þ → ş
    match cp {
        // Turkish-specific characters (ISO 8859-9 / Windows-1254)
        0x011E => Some(0xD0), // Ğ
        0x011F => Some(0xF0), // ğ
        0x0130 => Some(0xDD), // İ
        0x0131 => Some(0xFD), // ı
        0x015E => Some(0xDE), // Ş
        0x015F => Some(0xFE), // ş
        // Windows-1254 specific (0x80-0x9F range)
        0x20AC => Some(0x80), // €
        0x201A => Some(0x82), // ‚
        0x0192 => Some(0x83), // ƒ
        0x201E => Some(0x84), // „
        0x2026 => Some(0x85), // …
        0x2020 => Some(0x86), // †
        0x2021 => Some(0x87), // ‡
        0x02C6 => Some(0x88), // ˆ
        0x2030 => Some(0x89), // ‰
        0x0160 => Some(0x8A), // Š
        0x2039 => Some(0x8B), // ‹
        0x0152 => Some(0x8C), // Œ
        0x2018 => Some(0x91), // '
        0x2019 => Some(0x92), // '
        0x201C => Some(0x93), // "
        0x201D => Some(0x94), // "
        0x2022 => Some(0x95), // •
        0x2013 => Some(0x96), // –
        0x2014 => Some(0x97), // —
        0x02DC => Some(0x98), // ˜
        0x2122 => Some(0x99), // ™
        0x0161 => Some(0x9A), // š
        0x203A => Some(0x9B), // ›
        0x0153 => Some(0x9C), // œ
        0x0178 => Some(0x9F), // Ÿ
        // Latin-1 range (U+00A0-U+00FF) — direct mapping
        0x00A0..=0x00CF => Some(cp as u8), // before Ğ replacement
        0x00D0 => None,                    // Ð not in Win-1254 (replaced by Ğ)
        0x00D1..=0x00DC => Some(cp as u8),
        0x00DD => None, // Ý not in Win-1254 (replaced by İ)
        0x00DE => None, // Þ not in Win-1254 (replaced by Ş)
        0x00DF..=0x00EF => Some(cp as u8),
        0x00F0 => None, // ð not in Win-1254 (replaced by ğ)
        0x00F1..=0x00FC => Some(cp as u8),
        0x00FD => None,       // ý not in Win-1254 (replaced by ı)
        0x00FE => None,       // þ not in Win-1254 (replaced by ş)
        0x00FF => Some(0xFF), // ÿ
        _ => None,
    }
}

/// Encode a string to bytes for packet writing using Windows-1254 encoding.
/// Handles both client-originated strings (Latin-1 round-trip from read_string)
/// and server/DB-originated strings (UTF-8 with proper Turkish Unicode chars).
fn encode_string_bytes(s: &str) -> Vec<u8> {
    if s.is_ascii() {
        return s.as_bytes().to_vec();
    }
    // Try Windows-1254 encoding for each character
    let mut result = Vec::with_capacity(s.len());
    let mut all_ok = true;
    for c in s.chars() {
        match unicode_to_win1254(c) {
            Some(b) => result.push(b),
            None => {
                all_ok = false;
                break;
            }
        }
    }
    if all_ok {
        result
    } else {
        // Fallback: UTF-8 for strings with unmappable chars
        s.as_bytes().to_vec()
    }
}

/// A parsed packet with opcode and payload.
#[derive(Debug, Clone)]
pub struct Packet {
    /// The opcode byte.
    pub opcode: u8,
    /// The raw payload (excluding opcode).
    pub data: Vec<u8>,
    /// If true, send as plaintext (no AES/JvCryption) even when encryption is active.
    /// PCAP verified: Original game server sends S→C 0x02 heartbeat probes as plaintext.
    pub plaintext: bool,
}

impl Packet {
    /// Create a new packet with the given opcode and empty payload.
    pub fn new(opcode: u8) -> Self {
        Self {
            opcode,
            data: Vec::new(),
            plaintext: false,
        }
    }

    /// Create a packet with opcode and existing data.
    pub fn with_data(opcode: u8, data: Vec<u8>) -> Self {
        Self {
            opcode,
            data,
            plaintext: false,
        }
    }

    /// Create a plaintext packet (bypasses AES encryption).
    pub fn new_plaintext(opcode: u8) -> Self {
        Self {
            opcode,
            data: Vec::new(),
            plaintext: true,
        }
    }

    /// Write a u8 to the payload.
    pub fn write_u8(&mut self, val: u8) {
        self.data.push(val);
    }

    /// Write an i8 to the payload.
    pub fn write_i8(&mut self, val: i8) {
        self.data.push(val as u8);
    }

    /// Write raw bytes to the payload.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    /// Write a u16 (little-endian) to the payload.
    pub fn write_u16(&mut self, val: u16) {
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    /// Write an i16 (little-endian) to the payload.
    pub fn write_i16(&mut self, val: i16) {
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    /// Write a u32 (little-endian) to the payload.
    pub fn write_u32(&mut self, val: u32) {
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    /// Write an i32 (little-endian) to the payload.
    pub fn write_i32(&mut self, val: i32) {
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    /// Write a u64 (little-endian) to the payload.
    pub fn write_u64(&mut self, val: u64) {
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    /// Write an i64 (little-endian) to the payload.
    pub fn write_i64(&mut self, val: i64) {
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    /// Write an f32 (little-endian) to the payload.
    pub fn write_f32(&mut self, val: f32) {
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    /// Write a length-prefixed string to the payload.
    ///
    /// Uses Latin-1 encoding for strings containing non-ASCII chars (U+0080-U+00FF)
    /// to preserve client encoding round-trip with `read_string()`.
    /// Pure ASCII and strings with chars > U+00FF use standard UTF-8.
    pub fn write_string(&mut self, s: &str) {
        let bytes = encode_string_bytes(s);
        self.write_u16(bytes.len() as u16);
        self.data.extend_from_slice(&bytes);
    }

    /// Write raw bytes as a length-prefixed string (u16 length + bytes).
    /// No encoding conversion — bytes are written as-is.
    /// Used for user-input text (chat messages) to preserve client encoding.
    pub fn write_string_raw(&mut self, bytes: &[u8]) {
        self.write_u16(bytes.len() as u16);
        self.data.extend_from_slice(bytes);
    }

    /// Overwrite a u16 value at a specific offset in the payload.
    ///
    /// counts after iterating (e.g., ranking entry counts).
    ///
    /// # Panics
    /// Panics if `offset + 2 > data.len()`.
    pub fn put_u16_at(&mut self, offset: usize, val: u16) {
        let bytes = val.to_le_bytes();
        self.data[offset] = bytes[0];
        self.data[offset + 1] = bytes[1];
    }

    /// Overwrite a u8 value at a specific offset in the payload.
    ///
    pub fn put_u8_at(&mut self, offset: usize, val: u8) {
        self.data[offset] = val;
    }

    /// Get the current write position (end of data).
    ///
    pub fn wpos(&self) -> usize {
        self.data.len()
    }

    /// Write an SByte string (u8 length prefix + bytes).
    /// Used by `Packet::SByte()` mode for short strings.
    /// Uses Latin-1 encoding for round-trip preservation (see `write_string`).
    pub fn write_sbyte_string(&mut self, s: &str) {
        let bytes = encode_string_bytes(s);
        self.write_u8(bytes.len() as u8);
        self.data.extend_from_slice(&bytes);
    }

    /// Wrap this packet with LZF compression if payload > threshold.
    ///
    ///
    /// If the packet size < 500 bytes, returns None (send normally).
    /// Otherwise wraps in WIZ_COMPRESS_PACKET (0x42):
    /// `[u32 compressed_len] [u32 uncompressed_len] [u32 crc32] [compressed_data]`
    pub fn to_compressed(&self) -> Option<Packet> {
        // C++ threshold: pkt->size() < 500 → send normally
        if self.data.len() < 500 {
            return None;
        }

        // Build uncompressed buffer: [opcode] + [data]
        let in_length = 1 + self.data.len();
        let mut buffer = Vec::with_capacity(in_length);
        buffer.push(self.opcode);
        buffer.extend_from_slice(&self.data);

        // LZF compress
        let compressed = match lzf::compress(&buffer) {
            Ok(c) => c,
            Err(_) => return None,
        };

        // Build WIZ_COMPRESS_PACKET
        // PCAP verified: original server always sends CRC32=0.
        // v2600 client reads the field but does NOT validate it.
        let mut result = Packet::new(0x42); // WIZ_COMPRESS_PACKET
        result.write_u32(compressed.len() as u32);
        result.write_u32(in_length as u32);
        result.write_u32(0); // CRC32 = 0 (matches original server)
        result.write_bytes(&compressed);

        Some(result)
    }

    /// Serialize the full outbound packet frame (unencrypted).
    /// Format: `[0xAA55] [len: u16] [opcode] [data] [0x55AA]`
    pub fn to_outbound_frame(&self) -> Vec<u8> {
        let payload_len = 1 + self.data.len(); // opcode + data
        let mut frame = Vec::with_capacity(2 + 2 + payload_len + 2);
        frame.extend_from_slice(&HEADER_OUTGOING);
        frame.extend_from_slice(&(payload_len as u16).to_le_bytes());
        frame.push(self.opcode);
        frame.extend_from_slice(&self.data);
        frame.extend_from_slice(&FOOTER_OUTGOING);
        frame
    }
}

/// A simple cursor for reading from a packet payload.
pub struct PacketReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> PacketReader<'a> {
    /// Create a reader over the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Bytes remaining.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Read a u8.
    pub fn read_u8(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let val = self.data[self.pos];
            self.pos += 1;
            Some(val)
        } else {
            None
        }
    }

    /// Read an i8.
    pub fn read_i8(&mut self) -> Option<i8> {
        if self.pos < self.data.len() {
            let val = self.data[self.pos] as i8;
            self.pos += 1;
            Some(val)
        } else {
            None
        }
    }

    /// Read a u16 (little-endian).
    pub fn read_u16(&mut self) -> Option<u16> {
        if self.pos + 2 <= self.data.len() {
            let val = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            Some(val)
        } else {
            None
        }
    }

    /// Read an i16 (little-endian).
    pub fn read_i16(&mut self) -> Option<i16> {
        if self.pos + 2 <= self.data.len() {
            let val = i16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            Some(val)
        } else {
            None
        }
    }

    /// Read a u32 (little-endian).
    pub fn read_u32(&mut self) -> Option<u32> {
        if self.pos + 4 <= self.data.len() {
            let val = u32::from_le_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
            ]);
            self.pos += 4;
            Some(val)
        } else {
            None
        }
    }

    /// Read an i32 (little-endian).
    pub fn read_i32(&mut self) -> Option<i32> {
        if self.pos + 4 <= self.data.len() {
            let val = i32::from_le_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
            ]);
            self.pos += 4;
            Some(val)
        } else {
            None
        }
    }

    /// Read an f32 (little-endian).
    pub fn read_f32(&mut self) -> Option<f32> {
        if self.pos + 4 <= self.data.len() {
            let val = f32::from_le_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
            ]);
            self.pos += 4;
            Some(val)
        } else {
            None
        }
    }

    /// Read a u64 (little-endian).
    pub fn read_u64(&mut self) -> Option<u64> {
        if self.pos + 8 <= self.data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 8]);
            self.pos += 8;
            Some(u64::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// Read an i64 (little-endian).
    pub fn read_i64(&mut self) -> Option<i64> {
        if self.pos + 8 <= self.data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 8]);
            self.pos += 8;
            Some(i64::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// Maximum string length accepted from inbound packets (16 KiB).
    /// Prevents oversized string allocations from malicious length prefixes.
    const MAX_STRING_LEN: usize = 16384;

    /// Read a length-prefixed string (u16 length + bytes).
    ///
    /// Decodes bytes using Windows-1254 (Turkish) encoding for lossless
    /// round-trip with `write_string()`. Client sends Windows-1254 bytes;
    /// we decode to proper Unicode (ğ=U+011F, ş=U+015F, ı=U+0131 etc.)
    /// so the String can be stored in DB or logged correctly.
    /// `write_string()` encodes back to Windows-1254 for the wire.
    pub fn read_string(&mut self) -> Option<String> {
        let len = self.read_u16()? as usize;
        if len > Self::MAX_STRING_LEN || self.pos + len > self.data.len() {
            return None;
        }
        let bytes = &self.data[self.pos..self.pos + len];
        let s = decode_win1254(bytes);
        self.pos += len;
        Some(s)
    }

    /// Read an SByte string (u8 length prefix + bytes).
    /// Used by `Packet::SByte()` mode.
    /// Uses Windows-1254 decoding (see `read_string`).
    pub fn read_sbyte_string(&mut self) -> Option<String> {
        let len = self.read_u8()? as usize;
        if self.pos + len <= self.data.len() {
            let bytes = &self.data[self.pos..self.pos + len];
            let s = decode_win1254(bytes);
            self.pos += len;
            Some(s)
        } else {
            None
        }
    }

    /// Read a length-prefixed raw byte string (u16 length + bytes).
    /// Returns raw bytes without any encoding conversion.
    /// Used for user-input text (chat messages) where the client encoding
    /// (e.g. Windows-1254 for Turkish) must be preserved byte-for-byte.
    pub fn read_string_raw(&mut self) -> Option<Vec<u8>> {
        let len = self.read_u16()? as usize;
        if len > Self::MAX_STRING_LEN || self.pos + len > self.data.len() {
            return None;
        }
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Some(bytes)
    }

    /// Read remaining bytes as a slice.
    pub fn read_remaining(&mut self) -> &'a [u8] {
        let rest = &self.data[self.pos..];
        self.pos = self.data.len();
        rest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_outbound_frame() {
        let mut pkt = Packet::new(0x2B);
        pkt.write_u8(0x00);
        pkt.write_u16(2369);

        let frame = pkt.to_outbound_frame();
        // Header
        assert_eq!(frame[0], 0xAA);
        assert_eq!(frame[1], 0x55);
        // Length = 1 (opcode) + 3 (data) = 4
        assert_eq!(u16::from_le_bytes([frame[2], frame[3]]), 4);
        // Opcode
        assert_eq!(frame[4], 0x2B);
        // Data
        assert_eq!(frame[5], 0x00);
        assert_eq!(u16::from_le_bytes([frame[6], frame[7]]), 2369);
        // Footer
        assert_eq!(frame[8], 0x55);
        assert_eq!(frame[9], 0xAA);
    }

    #[test]
    fn test_packet_reader_string() {
        let mut pkt = Packet::new(0x01);
        pkt.write_string("testuser");

        let mut reader = PacketReader::new(&pkt.data);
        let s = reader.read_string().unwrap();
        assert_eq!(s, "testuser");
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_packet_sbyte_string_roundtrip() {
        let mut pkt = Packet::new(0x89);
        pkt.write_sbyte_string("Warrior01");

        let mut reader = PacketReader::new(&pkt.data);
        let s = reader.read_sbyte_string().unwrap();
        assert_eq!(s, "Warrior01");
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_packet_reader_mixed() {
        let mut pkt = Packet::new(0x01);
        pkt.write_u8(42);
        pkt.write_u16(1000);
        pkt.write_u32(0xDEADBEEF);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(42));
        assert_eq!(reader.read_u16(), Some(1000));
        assert_eq!(reader.read_u32(), Some(0xDEADBEEF));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_read_i8_positive_and_negative() {
        let mut pkt = Packet::new(0x01);
        pkt.write_i8(127);
        pkt.write_i8(-1);
        pkt.write_i8(-128);
        pkt.write_i8(0);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_i8(), Some(127));
        assert_eq!(reader.read_i8(), Some(-1));
        assert_eq!(reader.read_i8(), Some(-128));
        assert_eq!(reader.read_i8(), Some(0));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_read_i8_eof_returns_none() {
        let reader_data: [u8; 0] = [];
        let mut reader = PacketReader::new(&reader_data);
        assert_eq!(reader.read_i8(), None);
    }

    #[test]
    fn test_read_i16_positive_and_negative() {
        let mut pkt = Packet::new(0x01);
        pkt.write_i16(32767);
        pkt.write_i16(-1);
        pkt.write_i16(-32768);
        pkt.write_i16(0);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_i16(), Some(32767));
        assert_eq!(reader.read_i16(), Some(-1));
        assert_eq!(reader.read_i16(), Some(-32768));
        assert_eq!(reader.read_i16(), Some(0));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_read_i16_eof_returns_none() {
        let reader_data: [u8; 1] = [0xFF];
        let mut reader = PacketReader::new(&reader_data);
        assert_eq!(reader.read_i16(), None);
        assert_eq!(reader.remaining(), 1);
    }

    #[test]
    fn test_read_i32_positive_and_negative() {
        let mut pkt = Packet::new(0x01);
        pkt.write_i32(2_147_483_647); // i32::MAX
        pkt.write_i32(-1);
        pkt.write_i32(-2_147_483_648); // i32::MIN
        pkt.write_i32(0);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_i32(), Some(2_147_483_647));
        assert_eq!(reader.read_i32(), Some(-1));
        assert_eq!(reader.read_i32(), Some(-2_147_483_648));
        assert_eq!(reader.read_i32(), Some(0));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_read_i32_eof_returns_none() {
        let reader_data: [u8; 3] = [0xFF, 0xFF, 0xFF];
        let mut reader = PacketReader::new(&reader_data);
        assert_eq!(reader.read_i32(), None);
        assert_eq!(reader.remaining(), 3);
    }

    #[test]
    fn test_read_f32_roundtrip() {
        let mut pkt = Packet::new(0x01);
        pkt.write_f32(std::f32::consts::PI);
        pkt.write_f32(-0.5);
        pkt.write_f32(0.0);
        pkt.write_f32(f32::MAX);

        let mut reader = PacketReader::new(&pkt.data);
        let v1 = reader.read_f32().unwrap();
        assert!((v1 - std::f32::consts::PI).abs() < 1e-5);
        let v2 = reader.read_f32().unwrap();
        assert!((v2 - (-0.5)).abs() < 1e-7);
        assert_eq!(reader.read_f32(), Some(0.0));
        assert_eq!(reader.read_f32(), Some(f32::MAX));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_read_f32_eof_returns_none() {
        let reader_data: [u8; 3] = [0x00, 0x00, 0x00];
        let mut reader = PacketReader::new(&reader_data);
        assert_eq!(reader.read_f32(), None);
        assert_eq!(reader.remaining(), 3);
    }

    #[test]
    fn test_signed_unsigned_interop() {
        // Verify that writing signed and reading unsigned (and vice versa)
        // produces consistent results for the same bit pattern.
        let mut pkt = Packet::new(0x01);
        pkt.write_i16(-1); // 0xFFFF on wire
        pkt.write_u16(0xFFFF); // also 0xFFFF on wire

        let mut reader = PacketReader::new(&pkt.data);
        // read_u16 on -1 written as i16
        assert_eq!(reader.read_u16(), Some(0xFFFF));
        // read_i16 on 0xFFFF written as u16
        assert_eq!(reader.read_i16(), Some(-1));
        assert_eq!(reader.remaining(), 0);
    }
}
