//! WIZ_SERVERCHANGE (0x46) — S2C server transfer / redirect.
//! Wire format: `[SByte ip][u16 port][u8 init_flag][u16 zone_id][u8 nation]`
//! Used for cross-server transfer. Single-server deployments respond
//! with empty packet (rejection) or the current server's address.

use ko_protocol::{Opcode, Packet};

/// Build a server change redirect packet.
/// Format: `[SByte ip][u16 port][u8 init][u16 zone][u8 nation]`
pub fn build_server_change(ip: &str, port: u16, init_flag: u8, zone_id: u16, nation: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizServerChange as u8);
    pkt.write_sbyte_string(ip);
    pkt.write_u16(port);
    pkt.write_u8(init_flag);
    pkt.write_u16(zone_id);
    pkt.write_u8(nation);
    pkt
}

/// Build a server busy / rejection packet (empty payload = client stays).
pub fn build_server_busy() -> Packet {
    let mut pkt = Packet::new(Opcode::WizServerChange as u8);
    pkt.write_u8(0); // empty = rejection
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_server_change_packet_format() {
        let pkt = build_server_change("192.168.1.100", 15001, 1, 21, 2);
        let mut r = PacketReader::new(&pkt.data);
        let ip = r.read_sbyte_string().unwrap();
        assert_eq!(ip, "192.168.1.100");
        assert_eq!(r.read_u16(), Some(15001));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(21));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_server_busy_packet() {
        let pkt = build_server_busy();
        assert_eq!(pkt.opcode, Opcode::WizServerChange as u8);
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 0);
    }
}
