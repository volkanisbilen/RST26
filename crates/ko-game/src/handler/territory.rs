//! WIZ_TERRITORY (0xD5) handler — territory zone check.
//!
//! C2S opcode sent by the client to query territory ownership.
//! Server responds with owner clan/nation info for the queried zone.
//!
//! ## Client RE
//!
//! Panel at `[esi+0x6AC]` — UI-panel-dependent (Group B).
//! C2S format: `[u8 territory_type][u8 action]` — 2 bytes.
//! 1 send site. Client rate-limits (stores send timestamp, sets flag).
//!
//! Territory type is determined by current zone:
//! - Zones 204/211/212/104/111/112 → type=2
//! - Zone 850 (Delos/Castle Siege) → type=1 (forced)
//! - Other zones → pass-through from arg
//!
//! ## S2C Response Format (v2525-specific, no C++ reference)
//!
//! ```text
//! [u8 territory_type][u8 result]
//! [u16 owner_clan_id][u8 owner_nation][u8 tax_rate]
//! ```
//!
//! - `result`: 0 = no data (no territory in this zone), 1 = has data
//! - For type=1 (Castle): owner = siege master_knights clan, tax = delos_tariff
//! - For type=2 (Battle): owner = 0, nation = victorious nation (0/1/2), tax = 0

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};

// ── Territory Type Constants ────────────────────────────────────────

/// Castle Siege zone (Delos, zone 850).
pub const TERRITORY_CASTLE: u8 = 1;
/// Battle zones (204/211/212/104/111/112).
pub const TERRITORY_BATTLE: u8 = 2;

// ── S2C Builders ──────────────────────────────────────────────────────

/// Build a territory response with owner data.
///
/// Wire: `[u8 territory_type][u8 result=1][u16 owner_clan_id][u8 owner_nation][u8 tax_rate]`
pub fn build_territory_response(
    territory_type: u8,
    owner_clan_id: u16,
    owner_nation: u8,
    tax_rate: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizTerritory as u8);
    pkt.write_u8(territory_type);
    pkt.write_u8(1); // result = has data
    pkt.write_u16(owner_clan_id);
    pkt.write_u8(owner_nation);
    pkt.write_u8(tax_rate);
    pkt
}

/// Build a territory response indicating no territory data for this zone.
///
/// Wire: `[u8 territory_type][u8 result=0]`
pub fn build_territory_empty(territory_type: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::WizTerritory as u8);
    pkt.write_u8(territory_type);
    pkt.write_u8(0); // result = no data
    pkt
}

// ── C2S Handler ─────────────────────────────────────────────────────

/// Handle WIZ_TERRITORY (0xD5) from the client.
///
/// Queries the appropriate world state based on territory_type and
/// responds with owner clan/nation info.
///
/// Wire: `[u8 territory_type][u8 action]`
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let territory_type = reader.read_u8().unwrap_or(0);
    let action = reader.read_u8().unwrap_or(0);

    debug!(
        "[{}] WIZ_TERRITORY type={} action={}",
        session.addr(),
        territory_type,
        action
    );

    let world = session.world().clone();

    match territory_type {
        TERRITORY_CASTLE => {
            // Castle siege (Delos) — query siege warfare state.
            let siege = world.siege_war().read().await;
            let owner_clan = siege.master_knights;
            let tariff = siege.delos_tariff.min(100) as u8;
            drop(siege);

            let nation = if owner_clan > 0 {
                world.get_knights(owner_clan).map(|k| k.nation).unwrap_or(0)
            } else {
                0
            };

            session
                .send_packet(&build_territory_response(
                    TERRITORY_CASTLE,
                    owner_clan,
                    nation,
                    tariff,
                ))
                .await
        }
        TERRITORY_BATTLE => {
            // Battle zone — query nation war state.
            let bs = world.get_battle_state();
            let victor_nation = bs.victory; // 0=none, 1=Karus, 2=ElMorad

            session
                .send_packet(&build_territory_response(
                    TERRITORY_BATTLE,
                    0,
                    victor_nation,
                    0,
                ))
                .await
        }
        _ => {
            // Unknown territory type — respond with empty.
            session
                .send_packet(&build_territory_empty(territory_type))
                .await
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    // ── C2S Format ──────────────────────────────────────────────────

    #[test]
    fn test_c2s_format() {
        let mut pkt = Packet::new(Opcode::WizTerritory as u8);
        pkt.write_u8(TERRITORY_CASTLE);
        pkt.write_u8(1); // action

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(TERRITORY_CASTLE));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_c2s_data_length() {
        let mut pkt = Packet::new(Opcode::WizTerritory as u8);
        pkt.write_u8(0);
        pkt.write_u8(0);
        assert_eq!(pkt.data.len(), 2); // u8 + u8
    }

    #[test]
    fn test_opcode_value() {
        assert_eq!(Opcode::WizTerritory as u8, 0xD5);
    }

    #[test]
    fn test_territory_constants() {
        assert_eq!(TERRITORY_CASTLE, 1);
        assert_eq!(TERRITORY_BATTLE, 2);
    }

    // ── S2C Builder: territory_response ──────────────────────────────

    #[test]
    fn test_build_territory_response_opcode() {
        let pkt = build_territory_response(TERRITORY_CASTLE, 100, 2, 10);
        assert_eq!(pkt.opcode, Opcode::WizTerritory as u8);
    }

    #[test]
    fn test_build_territory_response_castle_format() {
        let pkt = build_territory_response(TERRITORY_CASTLE, 500, 2, 15);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(TERRITORY_CASTLE)); // type
        assert_eq!(r.read_u8(), Some(1)); // result = has data
        assert_eq!(r.read_u16(), Some(500)); // owner_clan_id
        assert_eq!(r.read_u8(), Some(2)); // nation (El Morad)
        assert_eq!(r.read_u8(), Some(15)); // tax_rate
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_territory_response_battle_format() {
        let pkt = build_territory_response(TERRITORY_BATTLE, 0, 1, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(TERRITORY_BATTLE));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(0)); // no clan for battle
        assert_eq!(r.read_u8(), Some(1)); // Karus victory
        assert_eq!(r.read_u8(), Some(0)); // no tax
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_territory_response_data_length() {
        // type(1) + result(1) + clan_id(2) + nation(1) + tax(1) = 6
        let pkt = build_territory_response(1, 0, 0, 0);
        assert_eq!(pkt.data.len(), 6);
    }

    #[test]
    fn test_build_territory_response_no_owner() {
        let pkt = build_territory_response(TERRITORY_CASTLE, 0, 0, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(TERRITORY_CASTLE));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(0)); // no owner
        assert_eq!(r.read_u8(), Some(0)); // no nation
        assert_eq!(r.read_u8(), Some(0)); // no tax
    }

    // ── S2C Builder: territory_empty ─────────────────────────────────

    #[test]
    fn test_build_territory_empty_opcode() {
        let pkt = build_territory_empty(3);
        assert_eq!(pkt.opcode, Opcode::WizTerritory as u8);
    }

    #[test]
    fn test_build_territory_empty_format() {
        let pkt = build_territory_empty(5);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(5)); // echoes type
        assert_eq!(r.read_u8(), Some(0)); // result = no data
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_territory_empty_data_length() {
        // type(1) + result(1) = 2
        let pkt = build_territory_empty(0);
        assert_eq!(pkt.data.len(), 2);
    }

    // ── Handler logic ─────────────────────────────────────────────────

    #[test]
    fn test_castle_territory_zones() {
        // Client RE: Zone 850 → type=1 (Delos / Castle Siege)
        // The client forces type=1 for zone 850
        assert_eq!(TERRITORY_CASTLE, 1);
    }

    #[test]
    fn test_battle_territory_zones() {
        // Client RE: Zones 204/211/212/104/111/112 → type=2
        assert_eq!(TERRITORY_BATTLE, 2);
    }

    #[test]
    fn test_tariff_capped_at_100() {
        // delos_tariff is u16 but tax_rate is u8 — cap at 100
        let tariff: u16 = 200;
        assert_eq!(tariff.min(100) as u8, 100);
        let normal: u16 = 15;
        assert_eq!(normal.min(100) as u8, 15);
    }
}
