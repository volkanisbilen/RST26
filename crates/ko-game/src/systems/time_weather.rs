//! Game time and weather broadcast system.
//! - `User.cpp:1816-1831` — `CUser::SendTime()` / `CUser::SendWeather()`
//! - `GameServerDlg.cpp:1384-1428` — weather update cycle
//! - `GameServerDlg.h:1089-1091` — `m_sYear/m_sMonth/m_sDate/m_sHour/m_sMin/m_sSec`
//! ## WIZ_TIME (0x13) — Server -> Client
//! `[u16 year] [u16 month] [u16 day] [u16 hour] [u16 minute]`
//! ## WIZ_WEATHER (0x14) — Server -> Client
//! `[u8 weather_type] [u16 amount]`
//! Weather types (from `packets.h`):
//! - `WEATHER_FINE` (1) = clear sky
//! - `WEATHER_RAIN` (2) = rain
//! - `WEATHER_SNOW` (3) = snow

use std::sync::atomic::{AtomicU16, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ko_protocol::{Opcode, Packet};

use crate::world::WorldState;

/// Weather type: clear sky.
pub const WEATHER_FINE: u8 = 0x01;
/// Weather type: rain.
pub const WEATHER_RAIN: u8 = 0x02;
/// Weather type: snow.
pub const WEATHER_SNOW: u8 = 0x03;

/// Time broadcast interval in seconds.
const TIME_BROADCAST_INTERVAL_SECS: u64 = 30;

/// Weather change interval in seconds (every hour, matching C++).
/// Previously was 300s (5 min) which caused client-side resource reload crashes.
const WEATHER_CHANGE_INTERVAL_SECS: u64 = 3600;

/// Shared game-time and weather state.
/// Thread-safe via atomics — updated by the background task,
/// read by `send_time()` / `send_weather()` on game entry.
pub struct GameTimeWeather {
    /// Current weather type.
    pub weather_type: AtomicU8,
    /// Current weather intensity (0-100).
    pub weather_amount: AtomicU16,

    // ── GM event rate modifiers (server-wide) ─────────────────────
    /// Bonus EXP percentage set by GM `+exp_add` command.
    ///
    pub exp_event_amount: AtomicU8,
    /// Bonus coin (noah) percentage set by GM `+money_add` command.
    ///
    pub coin_event_amount: AtomicU8,
    /// Bonus NP (loyalty) percentage set by GM `+np_add` command.
    ///
    pub np_event_amount: AtomicU8,
    /// Bonus drop rate percentage set by GM `+drop_add` command.
    ///
    pub drop_event_amount: AtomicU8,
}

impl Default for GameTimeWeather {
    fn default() -> Self {
        Self::new()
    }
}

impl GameTimeWeather {
    /// Create with default clear weather and zero event rates.
    pub fn new() -> Self {
        Self {
            weather_type: AtomicU8::new(WEATHER_FINE),
            weather_amount: AtomicU16::new(0),
            exp_event_amount: AtomicU8::new(0),
            coin_event_amount: AtomicU8::new(0),
            np_event_amount: AtomicU8::new(0),
            drop_event_amount: AtomicU8::new(0),
        }
    }

    /// Get current weather type.
    pub fn get_weather_type(&self) -> u8 {
        self.weather_type.load(Ordering::Relaxed)
    }

    /// Get current weather amount.
    pub fn get_weather_amount(&self) -> u16 {
        self.weather_amount.load(Ordering::Relaxed)
    }
}

/// Start the game time broadcast task.
/// Broadcasts WIZ_TIME to all in-game sessions every 30 seconds.
pub fn start_time_broadcast_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(TIME_BROADCAST_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let pkt = build_time_packet();
            world.broadcast_to_all(Arc::new(pkt), None);
        }
    })
}

/// Start the weather cycle task.
/// Changes weather and broadcasts to all in-game sessions every 5 minutes.
pub fn start_weather_task(
    world: Arc<WorldState>,
    time_weather: Arc<GameTimeWeather>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(WEATHER_CHANGE_INTERVAL_SECS));
        // Simple counter for pseudo-random weather
        let mut tick_count: u32 = 0;
        loop {
            interval.tick().await;
            tick_count = tick_count.wrapping_add(1);

            // Currently always clear — weather cycling is kept simple.
            // The C++ code had: `goto sendweather` which always sent WEATHER_FINE.
            // We match that behavior for now (always clear).
            let weather = WEATHER_FINE;
            let amount: u16 = 0;

            time_weather.weather_type.store(weather, Ordering::Relaxed);
            time_weather.weather_amount.store(amount, Ordering::Relaxed);

            let pkt = build_weather_packet(weather, amount);
            world.broadcast_to_all(Arc::new(pkt), None);

            tracing::trace!(weather, amount, tick = tick_count, "weather broadcast");
        }
    })
}

/// Hours the client adds to the WIZ_TIME clock (see [`build_time_packet`]).
pub const CLIENT_CLOCK_SHIFT_HOURS: i64 = 9;

/// Turkey (Istanbul) is UTC+3 all year.
pub const TURKEY_UTC_OFFSET_HOURS: i64 = 3;

/// Build a WIZ_TIME (0x13) packet with the current real time.
/// ```text
/// result << uint16(m_sYear) << uint16(m_sMonth) << uint16(m_sDate)
///        << uint16(m_sHour) << uint16(m_sMin);
/// ```
pub fn build_time_packet() -> Packet {
    // The 2625 client shows the packet time +9 hours (it treats it as UTC and adds Korean time): send the server's
    // local clock minus 9 h so the in-game clock shows the real local time (VPS 21:51 showed 06:51 before).
    // Istanbul time (UTC+3, no DST since 2016) regardless of the host's time zone, minus the client's +9 h.
    let now = chrono::Utc::now() + chrono::Duration::hours(TURKEY_UTC_OFFSET_HOURS - CLIENT_CLOCK_SHIFT_HOURS);
    let mut pkt = Packet::new(Opcode::WizTime as u8);
    // v2600: year is 2-digit (year - 2000), sniff verified
    let year_2d = now.format("%Y").to_string().parse::<u16>().unwrap_or(2025) - 2000;
    pkt.write_u16(year_2d);
    pkt.write_u16(now.format("%m").to_string().parse::<u16>().unwrap_or(1));
    pkt.write_u16(now.format("%d").to_string().parse::<u16>().unwrap_or(1));
    pkt.write_u16(now.format("%H").to_string().parse::<u16>().unwrap_or(0));
    pkt.write_u16(now.format("%M").to_string().parse::<u16>().unwrap_or(0));
    pkt
}

/// Build a WIZ_TIME packet with explicit values (for testing).
pub fn build_time_packet_with(year: u16, month: u16, day: u16, hour: u16, minute: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizTime as u8);
    pkt.write_u16(year);
    pkt.write_u16(month);
    pkt.write_u16(day);
    pkt.write_u16(hour);
    pkt.write_u16(minute);
    pkt
}

/// Build a WIZ_WEATHER (0x14) packet.
/// ```text
/// result << g_pMain->m_byWeather << g_pMain->m_sWeatherAmount;
/// ```
pub fn build_weather_packet(weather_type: u8, amount: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizWeather as u8);
    pkt.write_u8(weather_type);
    pkt.write_u16(amount);
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    #[test]
    fn test_time_packet_format() {
        let pkt = build_time_packet_with(2025, 6, 15, 14, 30);
        assert_eq!(pkt.opcode, Opcode::WizTime as u8);
        assert_eq!(pkt.data.len(), 10); // 5 * u16 = 10 bytes

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u16().unwrap(), 2025);
        assert_eq!(reader.read_u16().unwrap(), 6);
        assert_eq!(reader.read_u16().unwrap(), 15);
        assert_eq!(reader.read_u16().unwrap(), 14);
        assert_eq!(reader.read_u16().unwrap(), 30);
    }

    #[test]
    fn test_weather_packet_format() {
        let pkt = build_weather_packet(WEATHER_RAIN, 75);
        assert_eq!(pkt.opcode, Opcode::WizWeather as u8);
        assert_eq!(pkt.data.len(), 3); // u8 + u16 = 1 + 2

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), WEATHER_RAIN);
        assert_eq!(reader.read_u16().unwrap(), 75);
    }

    #[test]
    fn test_weather_packet_fine() {
        let pkt = build_weather_packet(WEATHER_FINE, 0);
        assert_eq!(pkt.opcode, 0x14);
        assert_eq!(pkt.data, vec![0x01, 0x00, 0x00]);
    }

    #[test]
    fn test_weather_packet_snow() {
        let pkt = build_weather_packet(WEATHER_SNOW, 100);
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), WEATHER_SNOW);
        assert_eq!(reader.read_u16().unwrap(), 100);
    }

    #[test]
    fn test_time_packet_roundtrip() {
        let pkt = build_time_packet_with(2026, 2, 8, 22, 45);
        let mut reader = PacketReader::new(&pkt.data);
        let year = reader.read_u16().unwrap();
        let month = reader.read_u16().unwrap();
        let day = reader.read_u16().unwrap();
        let hour = reader.read_u16().unwrap();
        let minute = reader.read_u16().unwrap();
        assert_eq!(year, 2026);
        assert_eq!(month, 2);
        assert_eq!(day, 8);
        assert_eq!(hour, 22);
        assert_eq!(minute, 45);
    }

    #[test]
    fn test_game_time_weather_defaults() {
        let tw = GameTimeWeather::new();
        assert_eq!(tw.get_weather_type(), WEATHER_FINE);
        assert_eq!(tw.get_weather_amount(), 0);
    }

    // ── Sprint 934: Additional coverage ──────────────────────────────

    /// Weather type constants match C++ packets.h.
    #[test]
    fn test_weather_type_constants() {
        assert_eq!(WEATHER_FINE, 1);
        assert_eq!(WEATHER_RAIN, 2);
        assert_eq!(WEATHER_SNOW, 3);
    }

    /// Time broadcast interval is 30 seconds.
    #[test]
    fn test_time_broadcast_interval() {
        assert_eq!(TIME_BROADCAST_INTERVAL_SECS, 30);
    }

    /// Weather change interval is 1 hour (3600 seconds).
    #[test]
    fn test_weather_change_interval() {
        assert_eq!(WEATHER_CHANGE_INTERVAL_SECS, 3600);
    }

    /// Time packet data length: 5 * u16 = 10 bytes.
    #[test]
    fn test_time_packet_data_length() {
        let pkt = build_time_packet_with(2025, 1, 1, 0, 0);
        assert_eq!(pkt.data.len(), 10);
    }
}
