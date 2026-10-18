//! # ko-protocol
//!
//! Packet definitions, binary serialization, and encryption routines
//! for the Knight Online client-server protocol.
//!
//! All packet serialization uses manual little-endian encoding via
//! `to_le_bytes()` / `from_le_bytes()` to match the client's wire format.
//! Every struct must have a corresponding roundtrip byte-level test.

pub mod aes_crypt;
pub mod crypto;
pub mod gameguard;
pub mod info_panel;
pub mod opcode;
pub mod packet;
pub mod smd;

pub use aes_crypt::AesCryption;
pub use crypto::JvCryption;
pub use opcode::{LoginOpcode, Opcode};
pub use packet::{Packet, PacketReader};
