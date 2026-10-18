//! Discord Rich Presence for the proxy DLL (`discord_presence.cpp`):
//! `E9 F6 [str app id][str image][str details][str state]` (u16 length strings).
//! Sent when a zone has loaded (also right after login) and on level up, so Discord shows
//! "<character> Lv<level> - <zone>" instead of the login-screen text.
//! `DISCORD_APP_ID`, `DISCORD_IMAGE` and `DISCORD_DETAILS` override the defaults.

use ko_protocol::Packet;

use crate::world::WorldState;
use crate::zone::SessionId;

const DEFAULT_APP_ID: &str = "1558987961775693945";
const DEFAULT_IMAGE: &str = "logo";
const DEFAULT_DETAILS: &str = "JstKO 2625";

fn setting(var: &str, default: &str) -> String {
    std::env::var(var).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| default.to_string())
}

/// Readable zone name for the Discord line.
pub fn zone_name(zone_id: u16) -> String {
    let name = match zone_id {
        1 => "Luferson Castle",
        2 => "El Morad Castle",
        11 => "Karus Eslant",
        12 => "El Morad Eslant",
        21..=25 => "Moradon",
        30 => "Delos",
        31 => "Bifrost",
        32 => "Desperation Abyss",
        33 => "Hell Abyss",
        48 => "Arena",
        55 => "Forgotten Temple",
        71 => "Ronark Land",
        72 => "Ardream",
        73 => "Ronark Land Base",
        84 => "Border Defence War",
        85 => "Chaos Dungeon",
        86 => "Under the Castle",
        87 => "Juraid Mountain",
        89 => "Full Moon Rift",
        95 => "Draki Tower",
        110 => "Fun Class Event",
        _ => return format!("Zone {zone_id}"),
    };
    name.to_string()
}

pub fn build_packet(name: &str, level: u8, zone_id: u16) -> Packet {
    let mut pkt = Packet::new(0xE9);
    pkt.write_u8(0xF6);
    pkt.write_string(&setting("DISCORD_APP_ID", DEFAULT_APP_ID));
    pkt.write_string(&setting("DISCORD_IMAGE", DEFAULT_IMAGE));
    pkt.write_string(&setting("DISCORD_DETAILS", DEFAULT_DETAILS));
    pkt.write_string(&format!("{name} Lv{level} - {}", zone_name(zone_id)));
    pkt
}

/// Tell this player's client what to show on Discord.
pub fn send(world: &WorldState, sid: SessionId) {
    let (Some(ch), Some(pos)) = (world.get_character_info(sid), world.get_position(sid)) else {
        return;
    };
    world.send_to_session_owned(sid, build_packet(&ch.name, ch.level, pos.zone_id));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_line_names_character_level_and_zone() {
        let pkt = build_packet("vlk", 83, 89);
        let text = String::from_utf8_lossy(&pkt.data).to_string();
        assert!(text.contains("vlk Lv83 - Full Moon Rift"));
        assert_eq!(pkt.opcode, 0xE9);
        assert_eq!(pkt.data[0], 0xF6);
    }

    #[test]
    fn unknown_zone_falls_back() {
        assert_eq!(zone_name(250), "Zone 250");
    }
}
