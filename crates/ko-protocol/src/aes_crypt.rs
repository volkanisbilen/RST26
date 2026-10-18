//! AES-128-CBC encryption for the Knight Online game protocol.
//!
//! Wireshark-verified: the original server uses AES-128-CBC instead of JvCryption
//! for game server communication. The key is sent in the 0x2B version response.
//!
//! ## Wire Protocol
//! - Flag byte `0x01` prefixes every AES-encrypted payload
//! - IV is fixed (extracted from PE binary)
//! - PKCS7 padding → ciphertext is always a multiple of 16 bytes
//! - No sequence counter, no CRC, no magic bytes
//!
//! ## IDA Verification
//! - `sub_833A30(0,0)` disables JvCryption
//! - `sub_833C10` sets the AES key from 0x2B response

use aes::Aes128;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;

/// AES flag byte — prefixed to every encrypted payload.
pub const AES_FLAG: u8 = 0x01;

/// Fixed IV extracted from the client PE binary.
const AES_IV: [u8; 16] = [
    0x32, 0x4E, 0xAA, 0x58, 0xBC, 0xB3, 0xAE, 0xE3, 0x6B, 0xC7, 0x4C, 0x56, 0x36, 0x47, 0x34, 0xF2,
];

/// AES-128-CBC encryption state for a client session.
pub struct AesCryption {
    key: [u8; 16],
    enabled: bool,
}

impl AesCryption {
    /// Create a new disabled AES encryption state.
    pub fn new() -> Self {
        Self {
            key: [0u8; 16],
            enabled: false,
        }
    }

    /// Set the AES key (called after version check response).
    pub fn set_key(&mut self, key: [u8; 16]) {
        self.key = key;
    }

    /// Enable AES encryption (called after key is sent to client).
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable AES encryption (e.g. after server select, session drops to plaintext).
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if AES encryption is active.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the current key.
    pub fn key(&self) -> &[u8; 16] {
        &self.key
    }

    /// Generate a random 16-byte key using uppercase alphanumeric characters.
    ///
    /// v2603 sniffer verified: original server keys are `[A-Z0-9]` only.
    /// Examples: `WNY2EXFDW0XQEPB4`, `2E17ZDKD5XYSVUAY`, `IVBEWMWKPA6MREWM`
    pub fn generate_key() -> [u8; 16] {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();
        let mut key = [0u8; 16];
        for byte in &mut key {
            *byte = CHARSET[rng.gen_range(0..CHARSET.len())];
        }
        key
    }

    /// Encrypt plaintext with AES-128-CBC + PKCS7 padding.
    ///
    /// Returns the ciphertext (without the 0x01 flag byte — caller adds it).
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        // PKCS7: pad to next 16-byte boundary (always adds at least 1 byte).
        let pad_len = 16 - (plaintext.len() % 16);
        let mut buf = vec![0u8; plaintext.len() + pad_len];
        buf[..plaintext.len()].copy_from_slice(plaintext);

        let ct = Aes128CbcEnc::new((&self.key).into(), (&AES_IV).into())
            .encrypt_padded_mut::<Pkcs7>(&mut buf, plaintext.len());

        match ct {
            Ok(ct) => ct.to_vec(),
            Err(_) => {
                // Should never happen with correct buffer sizing.
                tracing::error!("AES encrypt padding error (len={})", plaintext.len());
                Vec::new()
            }
        }
    }

    /// Decrypt AES-128-CBC ciphertext, removing PKCS7 padding.
    ///
    /// `ciphertext` should NOT include the 0x01 flag byte (caller strips it).
    /// Returns `None` if decryption or padding removal fails.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Option<Vec<u8>> {
        let mut buf = ciphertext.to_vec();
        Aes128CbcDec::new((&self.key).into(), (&AES_IV).into())
            .decrypt_padded_mut::<Pkcs7>(&mut buf)
            .ok()
            .map(|pt| pt.to_vec())
    }
}

impl Default for AesCryption {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_iv_constant() {
        assert_eq!(AES_IV.len(), 16);
        assert_eq!(AES_IV[0], 0x32);
        assert_eq!(AES_IV[15], 0xF2);
    }

    #[test]
    fn test_aes_flag_byte() {
        assert_eq!(AES_FLAG, 0x01);
    }

    #[test]
    fn test_new_disabled() {
        let aes = AesCryption::new();
        assert!(!aes.is_enabled());
        assert_eq!(aes.key(), &[0u8; 16]);
    }

    #[test]
    fn test_set_key_and_enable() {
        let mut aes = AesCryption::new();
        let key = [0x41u8; 16]; // "AAAA..."
        aes.set_key(key);
        assert_eq!(aes.key(), &key);
        assert!(!aes.is_enabled());

        aes.enable();
        assert!(aes.is_enabled());
    }

    #[test]
    fn test_generate_key_ascii() {
        let key = AesCryption::generate_key();
        assert_eq!(key.len(), 16);
        for &b in &key {
            assert!(
                (0x21..=0x7E).contains(&b),
                "key byte 0x{:02X} not in printable ASCII range",
                b
            );
        }
    }

    #[test]
    fn test_generate_key_randomness() {
        let key1 = AesCryption::generate_key();
        let key2 = AesCryption::generate_key();
        // Extremely unlikely to be equal
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut aes = AesCryption::new();
        aes.set_key(AesCryption::generate_key());

        let plaintext = b"Hello, Knight Online!";
        let ciphertext = aes.encrypt(plaintext);
        let decrypted = aes.decrypt(&ciphertext).expect("decrypt failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_output_is_block_aligned() {
        let mut aes = AesCryption::new();
        aes.set_key([0x42; 16]);

        // 1 byte → 16 bytes (15 padding)
        assert_eq!(aes.encrypt(&[0x01]).len(), 16);
        // 15 bytes → 16 bytes (1 padding)
        assert_eq!(aes.encrypt(&[0x01; 15]).len(), 16);
        // 16 bytes → 32 bytes (16 padding — PKCS7 always pads)
        assert_eq!(aes.encrypt(&[0x01; 16]).len(), 32);
        // 17 bytes → 32 bytes (15 padding)
        assert_eq!(aes.encrypt(&[0x01; 17]).len(), 32);
    }

    #[test]
    fn test_decrypt_bad_ciphertext() {
        let mut aes = AesCryption::new();
        aes.set_key([0x42; 16]);
        // Random garbage — should fail
        assert!(aes.decrypt(&[0xFF; 16]).is_none());
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let mut aes1 = AesCryption::new();
        aes1.set_key([0x41; 16]);

        let mut aes2 = AesCryption::new();
        aes2.set_key([0x42; 16]);

        let ct = aes1.encrypt(b"secret data");
        // Decrypting with wrong key should fail (bad padding)
        assert!(aes2.decrypt(&ct).is_none());
    }

    #[test]
    fn test_roundtrip_empty_plaintext() {
        let mut aes = AesCryption::new();
        aes.set_key([0x55; 16]);

        let ct = aes.encrypt(b"");
        assert_eq!(ct.len(), 16); // PKCS7: 16 bytes of 0x10 padding
        let pt = aes.decrypt(&ct).expect("decrypt failed");
        assert!(pt.is_empty());
    }

    #[test]
    fn test_roundtrip_packet_like_data() {
        let mut aes = AesCryption::new();
        aes.set_key(AesCryption::generate_key());

        // Simulate opcode + data
        let mut plaintext = vec![0x0C]; // opcode
        plaintext.push(0x01); // sub
        plaintext.push(0x01); // result
        plaintext.extend_from_slice(&[0x00; 100]); // empty data

        let ct = aes.encrypt(&plaintext);
        let pt = aes.decrypt(&ct).expect("decrypt failed");
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_ciphertext_differs_from_plaintext() {
        let mut aes = AesCryption::new();
        aes.set_key([0x41; 16]);

        let plaintext = [0x42; 32];
        let ciphertext = aes.encrypt(&plaintext);
        assert_ne!(&ciphertext[..32], &plaintext[..]);
    }

    #[test]
    fn test_default_trait() {
        let aes = AesCryption::default();
        assert!(!aes.is_enabled());
    }

    // ── PCAP Test Vectors (ko_full_session.pcap — original server) ──────

    /// PCAP verified: decrypt login C2S with original server key.
    /// Key: "85JT36DEES3XG7VJ", ciphertext from wire capture.
    #[test]
    fn test_pcap_login_c2s_decrypt() {
        let mut aes = AesCryption::new();
        aes.set_key(*b"85JT36DEES3XG7VJ");

        let ciphertext: [u8; 48] = [
            0x33, 0x00, 0x96, 0xAF, 0xC2, 0xF5, 0xBF, 0xE1, 0xB5, 0xD9, 0x2B, 0x44, 0x06, 0x2B,
            0x50, 0x1C, 0x64, 0xCE, 0xEB, 0x1A, 0x8B, 0x58, 0x87, 0xDC, 0x1E, 0x13, 0x9C, 0xF3,
            0x64, 0x39, 0x79, 0x7E, 0xC3, 0x00, 0x9C, 0x0D, 0x94, 0x2E, 0xEC, 0xE0, 0x3A, 0x34,
            0xB4, 0x04, 0xA1, 0x76, 0x6D, 0x05,
        ];

        let expected_plaintext: Vec<u8> = vec![
            0xF3, 0x0A, 0x00, 0x64, 0x65, 0x72, 0x64, 0x65, 0x6D, 0x34, 0x34, 0x33, 0x34, 0x15,
            0x00, 0x55, 0x35, 0x46, 0x32, 0x30, 0x53, 0x30, 0x57, 0x30, 0x45, 0x41, 0x31, 0x59,
            0x30, 0x43, 0x4C, 0x34, 0x52, 0x35, 0x43, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let plaintext = aes.decrypt(&ciphertext).expect("PCAP decrypt failed");
        assert_eq!(plaintext, expected_plaintext);
        assert_eq!(plaintext[0], 0xF3, "opcode must be LS_LOGIN_REQ");
    }

    /// PCAP verified: decrypt game server login response.
    /// Key: "B54BYUXR37A3W9MO".
    #[test]
    fn test_pcap_game_login_response_decrypt() {
        let mut aes = AesCryption::new();
        aes.set_key(*b"B54BYUXR37A3W9MO");

        let ciphertext: [u8; 16] = [
            0x22, 0xBA, 0x32, 0xA7, 0x16, 0xB7, 0xA9, 0x88, 0x9E, 0xFF, 0x27, 0x2D, 0x89, 0x1D,
            0xEA, 0xDB,
        ];

        let expected: Vec<u8> = vec![0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00];

        let plaintext = aes.decrypt(&ciphertext).expect("PCAP game decrypt failed");
        assert_eq!(plaintext, expected);
        assert_eq!(plaintext[0], 0x01, "opcode must be WIZ_LOGIN");
        assert_eq!(plaintext[1], 0x02, "nation must be Elmorad");
    }

    /// Verify IV matches the client PE binary value (VA 0x00F8E33C).
    #[test]
    fn test_iv_matches_client_binary() {
        let expected_iv: [u8; 16] = [
            0x32, 0x4E, 0xAA, 0x58, 0xBC, 0xB3, 0xAE, 0xE3, 0x6B, 0xC7, 0x4C, 0x56, 0x36, 0x47,
            0x34, 0xF2,
        ];
        assert_eq!(AES_IV, expected_iv);
    }
}
