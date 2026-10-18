//! WIZ_CAPTCHA (0xC0) handler — accepts client security code answer.
//!
//! ## Sniffer Evidence (2026-03-23, session 27 seq 3)
//!
//! After WIZ_LOGIN success, server sends 0xC0 [01 02 01] challenge.
//! Client shows security code dialog, user enters code.
//! Client sends: 0xC0 [01 02] [string captcha_code]
//! Server auto-accepts and continues with WIZ_LOADING_LOGIN (0x9F).
//!
//! ## C2S Request
//!
//! | Offset | Type   | Description       |
//! |--------|--------|-------------------|
//! | 0      | u8     | Sub (always 1)    |
//! | 1      | u8     | Type (always 2)   |
//! | 2      | string | Captcha answer    |

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::ClientSession;

/// Handle C2S WIZ_CAPTCHA (0xC0) — auto-accept security code.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let _sub = reader.read_u8().unwrap_or(0);
    let _captcha_type = reader.read_u8().unwrap_or(0);
    let captcha_code = reader.read_string().unwrap_or_default();

    tracing::info!(
        "[{}] 0xC0 CAPTCHA answer: '{}' (auto-accept)",
        session.addr(),
        captcha_code,
    );

    // Sniffer verified: after CAPTCHA accept, send WIZ_LOADING_LOGIN
    // S2C 0x9F [01 00000000] (6 bytes)
    let mut loading = Packet::new(Opcode::WizLoadingLogin as u8); // 0x9F
    loading.write_u8(1);
    loading.write_u32(0);
    session.send_packet(&loading).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_captcha_opcode() {
        assert_eq!(Opcode::WizCaptcha as u8, 0xC0);
    }

    #[test]
    fn test_captcha_challenge_format() {
        let mut pkt = Packet::new(Opcode::WizCaptcha as u8);
        pkt.write_u8(1);
        pkt.write_u8(2);
        pkt.write_u8(1);
        assert_eq!(pkt.opcode, 0xC0);
        assert_eq!(pkt.data, vec![1, 2, 1]);
    }

    #[test]
    fn test_captcha_c2s_answer_format() {
        let mut pkt = Packet::new(Opcode::WizCaptcha as u8);
        pkt.write_u8(1); // sub
        pkt.write_u8(2); // type
        pkt.write_string("ABCD"); // captcha answer

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_string(), Some("ABCD".to_string()));
    }

    #[test]
    fn test_loading_login_response_format() {
        let mut pkt = Packet::new(Opcode::WizLoadingLogin as u8);
        pkt.write_u8(1);
        pkt.write_u32(0);
        assert_eq!(pkt.opcode, 0x9F);
        assert_eq!(pkt.data.len(), 5); // u8 + u32
    }
}
