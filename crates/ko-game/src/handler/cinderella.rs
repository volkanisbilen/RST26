//! WIZ_CINDERELLA (0xE0) handler -- Cinderella War (Fun Class) Event.
//! ## Sub-opcodes (cindopcode enum)
//! | Value | Name          | Description                               |
//! |-------|---------------|-------------------------------------------|
//! | 0     | selectclass   | Select/change class during Cinderella war  |
//! | 1     | nationchange  | Change nation during Cinderella war        |
//! | 2     | joinevent     | Join the Cinderella event (server-sent)    |
//! | 3     | starting      | Event starting notification (server-sent)  |
//! | 4     | updatekda     | KDA update (server-sent)                  |
//! | 5     | finish        | Event finished (server-sent)              |
//! | 6     | success       | Operation succeeded (server-sent)         |
//! | 7     | timewait      | Cooldown remaining (server-sent)          |
//! | 8     | notchange     | Cannot change (server-sent)               |
//! | 9     | alreadyclass  | Already that class (server-sent)          |
//! | 10    | alreadynation | Already that nation (server-sent)         |
//! Cinderella War is a special PvP event where players can change their
//! class and nation temporarily. The event uses special zones and has
//! its own matchmaking/room system. Players are given preset equipment,
//! stats, and skills based on the selected class and tier.

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};
use crate::world::types::UserItemSlot;
#[cfg(test)]
use crate::world::types::ZONE_MORADON;
use crate::world::WorldState;
use crate::zone::SessionId;

/// Cinderella sub-opcode constants.
pub mod sub_opcode {
    /// Select/change class during event.
    pub const SELECT_CLASS: u8 = 0;
    /// Change nation during event.
    pub const NATION_CHANGE: u8 = 1;
    /// Join event (server-sent to client).
    pub const JOIN_EVENT: u8 = 2;
    /// Event starting notification (server-sent).
    pub const STARTING: u8 = 3;
    /// KDA update (server-sent).
    pub const UPDATE_KDA: u8 = 4;
    /// Event finished (server-sent).
    pub const FINISH: u8 = 5;
    /// Operation succeeded (server-sent).
    pub const SUCCESS: u8 = 6;
    /// Cooldown remaining (server-sent).
    pub const TIME_WAIT: u8 = 7;
    /// Cannot change class (server-sent error).
    pub const NOT_CHANGE: u8 = 8;
    /// Already that class (server-sent error).
    pub const ALREADY_CLASS: u8 = 9;
    /// Already that nation (server-sent error).
    pub const ALREADY_NATION: u8 = 10;
}

/// Maximum number of tier presets (0-4).
pub const MAX_SETTING_TIERS: usize = 5;

/// Maximum number of classes in the event (warrior=0, rogue=1, mage=2, priest=3, kurian=4).
pub const MAX_CLASSES: usize = 5;

/// Class cooldown for class change (seconds).
pub const CLASS_CHANGE_COOLDOWN_SECS: u64 = 80;

/// Cooldown for nation change (seconds).
pub const NATION_CHANGE_COOLDOWN_SECS: u64 = 90;

/// Error cooldown applied on failed attempts (seconds).
pub const ERROR_COOLDOWN_SECS: u64 = 5;

// ── Per-Player State ─────────────────────────────────────────────────────

/// Per-player Cinderella War event state.
/// Stores the player's original data so it can be restored when the event
/// ends or the player leaves.
#[derive(Debug, Clone)]
pub struct CindirellaPlayerState {
    /// Whether this player is an active event participant.
    pub event_user: bool,
    /// Whether this is the player's first class selection.
    pub first_selected: bool,
    // ── Original data (saved on first join, restored on logout) ──
    /// Original class code.
    pub original_class: u16,
    /// Original race.
    pub original_race: u8,
    /// Original nation.
    pub original_nation: u8,
    /// Original level.
    pub original_level: u8,
    /// Original experience.
    pub original_exp: u64,
    /// Original gold.
    pub original_gold: u32,
    /// Original stats \[STR, STA, DEX, INT, CHA\].
    pub original_stats: [u8; 5],
    /// Original free stat points.
    pub original_stat_points: u16,
    /// Original inventory snapshot.
    pub original_inventory: Vec<UserItemSlot>,
    /// Original skill point array (free points + the four skill pages).
    pub original_skill_points: [u8; 10],
    // ── Event tracking ──
    /// Current event nation (can differ from original due to nation change).
    pub event_nation: u8,
    /// Currently selected class index (0-4).
    pub selected_class: u8,
    /// Kill count in the event.
    pub kill_count: u16,
    /// Death count in the event.
    pub dead_count: u16,
    /// EXP gained during the event.
    pub gained_exp: u64,
    /// Gold gained during the event.
    pub gained_noah: u32,
    // ── Cooldowns (unix timestamps) ──
    /// Class change cooldown expiry.
    pub class_change_cooldown: u64,
    /// Nation change cooldown expiry.
    pub nation_change_cooldown: u64,
}

impl Default for CindirellaPlayerState {
    fn default() -> Self {
        Self {
            event_user: false,
            first_selected: true,
            original_class: 0,
            original_race: 0,
            original_nation: 0,
            original_level: 0,
            original_exp: 0,
            original_gold: 0,
            original_stats: [0u8; 5],
            original_stat_points: 0u16,
            original_inventory: Vec::new(),
            original_skill_points: [0u8; 10],
            event_nation: 0,
            selected_class: 0,
            kill_count: 0,
            dead_count: 0,
            gained_exp: 0,
            gained_noah: 0,
            class_change_cooldown: 0,
            nation_change_cooldown: 0,
        }
    }
}

// ── Global Event State ───────────────────────────────────────────────────

/// Global Cinderella War event lifecycle state.
#[derive(Debug, Clone, Default)]
pub struct CindirellaEventState {
    /// Whether the event is in the prepare (registration) phase.
    pub prepare: bool,
    /// Whether the event is in the active (war) phase.
    pub start: bool,
    /// Unix timestamp when the prepare phase ends.
    pub prepare_time: u64,
    /// Unix timestamp when the war phase ends.
    pub finish_time: u64,
    /// Which setting tier (0-4) is active.
    pub setting_id: u8,
    /// Elmorad total kill count.
    pub elmorad_kills: u16,
    /// Karus total kill count.
    pub karus_kills: u16,
}

impl CindirellaEventState {
    /// Whether the event is active (prepare or war phase).
    ///
    pub fn is_on(&self) -> bool {
        self.prepare || self.start
    }
}

/// Cinderella class index from class code.
/// Knight Online class codes:
///   Karus:   101-106(warrior), 107-108(rogue), 109-110(mage), 111-112(priest), 113-115(kurian)
///   Elmorad: 201-206(warrior), 207-208(rogue), 209-210(mage), 211-212(priest), 213-215(kurian)
/// Returns 0=Warrior, 1=Rogue, 2=Mage, 3=Priest, 4=Kurian, or None.
pub fn get_class_index(class_code: u16) -> Option<u8> {
    let normalized = if class_code > 200 {
        class_code - 100
    } else {
        class_code
    };
    match normalized {
        101..=106 => Some(0), // Warrior
        107..=108 => Some(1), // Rogue
        109..=110 => Some(2), // Mage
        111..=112 => Some(3), // Priest
        113..=115 => Some(4), // Kurian
        _ => None,
    }
}

/// Get the new class code for a Cinderella event class selection.
/// Returns the class code based on nation, selected class index, and tier level.
pub fn get_new_class(nation: u8, class_index: u8, beginner_level: u8) -> Option<u16> {
    let base_class = if nation == 2 {
        // Elmorad
        match class_index {
            0 => 206u16, // Warrior (Blade/Berserker)
            1 => 208,    // Rogue (Ranger/Assassin)
            2 => 210,    // Mage (Fire/Ice)
            3 => 212,    // Priest (BP/Buffer)
            4 => 215,    // Kurian (PortuKurian)
            _ => return None,
        }
    } else {
        // Karus
        match class_index {
            0 => 106u16,
            1 => 108,
            2 => 110,
            3 => 112,
            4 => 115,
            _ => return None,
        }
    };

    // If below level 60, use one tier lower class (e.g., 206 -> 205)
    if beginner_level < 60 {
        Some(base_class - 1)
    } else {
        Some(base_class)
    }
}

/// Get the new race for a Cinderella event class selection.
pub fn get_new_race(nation: u8, class_index: u8) -> Option<u8> {
    if nation == 2 {
        // Elmorad
        match class_index {
            0 => Some(12),
            1 => Some(12),
            2 => Some(13),
            3 => Some(13),
            4 => Some(14),
            _ => None,
        }
    } else {
        // Karus
        match class_index {
            0 => Some(1),
            1 => Some(2),
            2 => Some(4),
            3 => Some(4),
            4 => Some(6),
            _ => None,
        }
    }
}

/// Check if a zone is a Cinderella War zone.
pub fn is_cinderella_zone(zone_id: u16, event_zone_id: u16) -> bool {
    zone_id == event_zone_id
}

// ── Packet Builders ──────────────────────────────────────────────────────
//
// All Cinderella responses wrap in: WIZ_EXT_HOOK(0xE9) + CINDIRELLA(0xE0) + cindopcode + payload

/// Build base response packet with WIZ_EXT_HOOK + CINDIRELLA sub-opcode.
fn build_cind_base() -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(0xE0); // ExtHookSubOpcodes::CINDIRELLA
    pkt
}

/// Build join event response.
pub fn build_join_event(
    is_prepare: bool,
    class_index: u8,
    remaining_time: u32,
    kill_count: u16,
    dead_count: u16,
    karus_kills: u16,
    elmorad_kills: u16,
) -> Packet {
    let mut pkt = build_cind_base();
    pkt.write_u8(sub_opcode::JOIN_EVENT);
    pkt.write_u8(is_prepare as u8);
    pkt.write_u8(class_index);
    pkt.write_u32(remaining_time);
    pkt.write_u16(kill_count);
    pkt.write_u16(dead_count);
    pkt.write_u16(karus_kills);
    pkt.write_u16(elmorad_kills);
    pkt
}

/// Build starting notification.
pub fn build_starting(remaining_time: u32) -> Packet {
    let mut pkt = build_cind_base();
    pkt.write_u8(sub_opcode::STARTING);
    pkt.write_u32(remaining_time);
    pkt
}

/// Build individual KDA update (type=0: personal).
pub fn build_kda_personal(kill_count: u16, dead_count: u16) -> Packet {
    let mut pkt = build_cind_base();
    pkt.write_u8(sub_opcode::UPDATE_KDA);
    pkt.write_u8(0); // personal
    pkt.write_u16(kill_count);
    pkt.write_u16(dead_count);
    pkt
}

/// Build global KDA update (type=1: broadcast).
pub fn build_kda_global(elmorad_kills: u16, karus_kills: u16) -> Packet {
    let mut pkt = build_cind_base();
    pkt.write_u8(sub_opcode::UPDATE_KDA);
    pkt.write_u8(1); // global
    pkt.write_u16(elmorad_kills);
    pkt.write_u16(karus_kills);
    pkt
}

/// Build finish notification.
pub fn build_finish() -> Packet {
    let mut pkt = build_cind_base();
    pkt.write_u8(sub_opcode::FINISH);
    pkt
}

/// Build select class success response.
pub fn build_select_success(selected_class: u8) -> Packet {
    let mut pkt = build_cind_base();
    pkt.write_u8(sub_opcode::SELECT_CLASS);
    pkt.write_u8(sub_opcode::SUCCESS);
    pkt.write_u8(selected_class);
    pkt
}

/// Build nation change success response.
pub fn build_nation_success() -> Packet {
    let mut pkt = build_cind_base();
    pkt.write_u8(sub_opcode::NATION_CHANGE);
    pkt.write_u8(sub_opcode::SUCCESS);
    pkt
}

/// Build error response for selectclass or nationchange.
pub fn build_cind_error(is_nation: bool, error_code: u8, remaining_secs: u32) -> Packet {
    let mut pkt = build_cind_base();
    pkt.write_u8(if is_nation {
        sub_opcode::NATION_CHANGE
    } else {
        sub_opcode::SELECT_CLASS
    });
    pkt.write_u8(error_code);
    if error_code == sub_opcode::TIME_WAIT {
        pkt.write_u32(remaining_secs);
    }
    pkt
}

// ── WIZ_PRESET Builders ─────────────────────────────────────────────────

/// Build WIZ_PRESET type 1 — stat preset.
/// **v2525 CONFLICT**: 0xB9 = WIZ_PET_STAT. Cannot send preset packets.
/// Stats are applied to world state directly; client refreshes on zone change.
/// Wire: `[WIZ_PRESET][u8 1][u8 1][u16*5 stats][u16 free_points]`
#[cfg(test)]
fn build_preset_stats(stats: [i16; 5], free_points: i16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizPreset as u8);
    pkt.write_u8(1); // type 1 = stats
    pkt.write_u8(1); // unknown constant
    for s in &stats {
        pkt.write_i16(*s);
    }
    pkt.write_i16(free_points);
    pkt
}

/// Build WIZ_PRESET type 2 — skill preset.
/// **v2525 CONFLICT**: 0xB9 = WIZ_PET_STAT. Cannot send preset packets.
/// Wire: `[WIZ_PRESET][u8 2][u8 1][u8*4 pages][u8 free_skill]`
#[cfg(test)]
fn build_preset_skills(skill_pages: [i16; 4], free_skill_pts: i16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizPreset as u8);
    pkt.write_u8(2); // type 2 = skills
    pkt.write_u8(1); // unknown constant
    for p in &skill_pages {
        pkt.write_u8(*p as u8);
    }
    pkt.write_u8(free_skill_pts as u8);
    pkt
}

// ── Handler ──────────────────────────────────────────────────────────────

/// Handle incoming WIZ_CINDERELLA (0xE0) packet.
/// The first byte is a sub-opcode. Only `selectclass` (0) and
/// `nationchange` (1) are client-initiated; the rest are server-sent.
/// The event system requires WorldState integration to be fully functional.
/// Currently validates packets and sub-opcodes, but event activation
/// depends on GM commands setting up the event state.
/// Zone the Fun Class runs in. C++ forces the mini maps (107 / 115); this client crashes while
/// loading those maps (zones 107, 111, 115), so the old PvP / mini settings fall back to the
/// Snow War map (110), which loads fine. Any other configured zone is kept.
pub fn forced_event_zone(zone: u16) -> u16 {
    match zone {
        71..=73 | 107 | 115 => 110,
        _ => zone,
    }
}

/// Apply the DB stat / skill preset of the chosen class (cindwar_stat).
fn apply_stat_preset(world: &WorldState, sid: SessionId, setting_id: u8, selected_class: u8) -> Option<ko_db::models::cinderella::CindwarStatRow> {
    let sp = world.get_cindwar_stat_preset(setting_id as i16, (selected_class + 1) as i16);
    world.update_character_stats(sid, |ch| {
        // C++ clears the skills, then sets the preset pages when a preset exists
        ch.skill_points = [0u8; 10];
        if let Some(ref sp) = sp {
            ch.str = sp.stat_str as u8;
            ch.sta = sp.stat_sta as u8;
            ch.dex = sp.stat_dex as u8;
            ch.intel = sp.stat_int as u8;
            ch.cha = sp.stat_cha as u8;
            ch.free_points = sp.stat_freepoint as u16;
            ch.skill_points[0] = sp.skill_freepoint as u8;
            ch.skill_points[5] = sp.skill_page1 as u8;
            ch.skill_points[6] = sp.skill_page2 as u8;
            ch.skill_points[7] = sp.skill_page3 as u8;
            ch.skill_points[8] = sp.skill_page4 as u8;
        }
    });
    sp
}

/// Recalculate abilities, fill HP/MP and push the new identity (class, level, items, stats)
/// to the player and to everyone who sees them (C++ SendMyInfo + UserInOut).
async fn refresh_client(session: &mut ClientSession, world: &WorldState, sid: SessionId) -> anyhow::Result<()> {
    world.update_character_stats(sid, |ch| {
        ch.max_exp = world.get_exp_by_level(ch.level, 0);
    });
    world.set_user_ability(sid);
    world.update_character_stats(sid, |ch| {
        ch.hp = ch.max_hp;
        ch.mp = ch.max_mp;
    });
    super::gamestart::send_myinfo_refresh(session).await?;
    super::region::broadcast_user_appearance_update(world, sid);
    Ok(())
}

/// C++ ZoneChange -> CindirellaSign: entering the event zone signs the player up with
/// their current class; the client then opens the class window from the joinevent packet.
pub async fn auto_join(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let (is_prepare, setting_id, e_kills, k_kills) = {
        let event = world.cindwar_event();
        if !event.is_on() {
            return Ok(());
        }
        (event.prepare, event.setting_id, event.elmorad_kills, event.karus_kills)
    };
    if world.get_cindwar_player(sid).is_some_and(|p| p.event_user) {
        return Ok(());
    }
    let class_code = world.get_character_info(sid).map(|c| c.class).unwrap_or(0);
    let idx = get_class_index(class_code).unwrap_or(0);
    cinderella_sign(session, &world, sid, idx, setting_id, is_prepare, e_kills, k_kills, now_unix()).await
}

/// Give the event players back their own identity (class, level, items, stats, skills).
fn restore_player(world: &WorldState, sid: SessionId, ps: CindirellaPlayerState) {
    world.set_inventory(sid, ps.original_inventory);
    world.update_character_stats(sid, |ch| {
        ch.class = ps.original_class;
        ch.race = ps.original_race;
        ch.nation = ps.original_nation;
        ch.level = ps.original_level;
        ch.exp = ps.original_exp.saturating_add(ps.gained_exp);
        ch.gold = ps.original_gold.saturating_add(ps.gained_noah);
        ch.str = ps.original_stats[0];
        ch.sta = ps.original_stats[1];
        ch.dex = ps.original_stats[2];
        ch.intel = ps.original_stats[3];
        ch.cha = ps.original_stats[4];
        ch.free_points = ps.original_stat_points;
        ch.skill_points = ps.original_skill_points;
        ch.max_exp = world.get_exp_by_level(ps.original_level, 0);
    });
    world.set_user_ability(sid);
    world.update_character_stats(sid, |ch| {
        ch.hp = ch.max_hp;
        ch.mp = ch.max_mp;
    });
    world.remove_cinderella_user(sid);
    world.mark_cind_refresh_pending(sid);
}

pub async fn handle(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // Dead players cannot interact with cinderella events
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&packet.data);
    let opcode = reader.read_u8().unwrap_or(0);

    match opcode {
        sub_opcode::SELECT_CLASS => {
            handle_select_class(session, &mut reader).await?;
        }
        sub_opcode::NATION_CHANGE => {
            handle_nation_change(session, &mut reader).await?;
        }
        _ => {
            debug!(
                "[{}] WIZ_CINDERELLA unhandled sub-opcode={}",
                session.addr(),
                opcode
            );
        }
    }

    Ok(())
}

/// Get current unix timestamp in seconds.
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Handle selectclass sub-opcode.
/// + `CUser::CindirellaSign()`
async fn handle_select_class(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let selected_class = reader.read_u8().unwrap_or(255);
    let world = session.world().clone();
    let sid = session.session_id();

    // Validate class range (0-4)
    if selected_class > 4 {
        return Ok(());
    }

    // Event must be active
    let (is_prepare, setting_id, e_kills, k_kills) = {
        let event = world.cindwar_event();
        if !event.is_on() {
            return Ok(());
        }
        (
            event.prepare,
            event.setting_id,
            event.elmorad_kills,
            event.karus_kills,
        )
    };

    let now = now_unix();

    // Check if player is already an event user
    let player_state = world.get_cindwar_player(sid);

    if let Some(ref ps) = player_state {
        if ps.event_user {
            // Already in event — class switch (CindirellaSelectClass)
            let cind_zone = world.cinderella_zone_id();
            let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
            if !is_cinderella_zone(zone_id, cind_zone) {
                return Ok(());
            }

            // Cooldown check
            if ps.class_change_cooldown > now {
                let remaining = (ps.class_change_cooldown - now) as u32;
                let pkt = build_cind_error(false, sub_opcode::TIME_WAIT, remaining);
                session.send_packet(&pkt).await?;
                return Ok(());
            }

            // Already same class?
            if ps.selected_class == selected_class {
                let pkt = build_cind_error(false, sub_opcode::ALREADY_CLASS, 0);
                session.send_packet(&pkt).await?;
                return Ok(());
            }

            // Apply class change
            world.update_cindwar_player(sid, |s| {
                s.selected_class = selected_class;
                s.class_change_cooldown = now + CLASS_CHANGE_COOLDOWN_SECS;
            });

            cinderella_cha_modify(
                session,
                &world,
                sid,
                selected_class,
                setting_id,
                false,
                false,
            )
            .await?;

            let pkt = build_select_success(selected_class);
            session.send_packet(&pkt).await?;

            debug!(
                "[{}] CindirellaWar: class switch to {}",
                session.addr(),
                selected_class
            );
            return Ok(());
        }
    }

    // First time join — CindirellaSign
    cinderella_sign(
        session,
        &world,
        sid,
        selected_class,
        setting_id,
        is_prepare,
        e_kills,
        k_kills,
        now,
    )
    .await
}

/// First-time event join — save original data, apply preset.
#[allow(clippy::too_many_arguments)]
async fn cinderella_sign(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    selected_class: u8,
    setting_id: u8,
    is_prepare: bool,
    e_kills: u16,
    k_kills: u16,
    now: u64,
) -> anyhow::Result<()> {
    let char_info = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Get event setting for beginner_level
    let setting = match world.get_cindwar_setting(setting_id) {
        Some(s) => s,
        None => return Ok(()),
    };

    let beginner_level = setting.beginner_level as u8;
    let event_nation = char_info.nation;

    // Save original data
    let original_inventory = world.get_inventory(sid);
    let mut ps = CindirellaPlayerState {
        event_user: true,
        first_selected: false,
        original_class: char_info.class,
        original_race: char_info.race,
        original_nation: event_nation,
        original_level: char_info.level,
        original_exp: char_info.exp,
        original_gold: char_info.gold,
        original_stats: [
            char_info.str,
            char_info.sta,
            char_info.dex,
            char_info.intel,
            char_info.cha,
        ],
        original_stat_points: char_info.free_points,
        original_inventory,
        original_skill_points: char_info.skill_points,
        event_nation,
        selected_class,
        kill_count: 0,
        dead_count: 0,
        gained_exp: 0,
        gained_noah: 0,
        class_change_cooldown: now + CLASS_CHANGE_COOLDOWN_SECS,
        nation_change_cooldown: now + NATION_CHANGE_COOLDOWN_SECS,
    };

    // Apply preset class
    let new_class = match get_new_class(event_nation, selected_class, beginner_level) {
        Some(c) => c,
        None => return Ok(()),
    };
    let new_race = match get_new_race(event_nation, selected_class) {
        Some(r) => r,
        None => return Ok(()),
    };

    // Update character stats
    world.update_character_stats(sid, |ch| {
        ch.class = new_class;
        ch.race = new_race;
        ch.level = beginner_level;
        ch.exp = 0;
        ch.gold = 0;
    });

    // Apply stat / skill preset
    let stat_preset = apply_stat_preset(world, sid, setting_id, selected_class);

    // Apply preset items
    apply_preset_items(world, sid, setting_id, selected_class);

    // v2525: 0xB9 = WIZ_PET_STAT. Cannot send preset packets.
    // Stats are applied to world state above; client refreshes on zone change.

    // Send WIZ_CLASS_CHANGE
    let mut class_pkt = Packet::new(Opcode::WizClassChange as u8);
    class_pkt.write_u8(5); // ALL_SKILLPT_CHANGE
    class_pkt.write_u8(1); // success
    class_pkt.write_u32(0); // coins
    if let Some(ref sp) = stat_preset {
        class_pkt.write_u8(sp.skill_freepoint as u8);
    } else {
        class_pkt.write_u8(0);
    }
    session.send_packet(&class_pkt).await?;

    refresh_client(session, world, sid).await?;

    // Calculate remaining time
    let remaining = {
        let event = world.cindwar_event();
        if event.prepare {
            event.prepare_time.saturating_sub(now) as u32
        } else {
            event.finish_time.saturating_sub(now) as u32
        }
    };

    // Send join event response
    let join_pkt = build_join_event(
        is_prepare,
        selected_class,
        remaining,
        ps.kill_count,
        ps.dead_count,
        k_kills,
        e_kills,
    );
    session.send_packet(&join_pkt).await?;

    // Add to event user lists
    world.add_cinderella_user(sid);
    ps.event_user = true;
    world.set_cindwar_player(sid, ps);

    debug!(
        "[{}] CindirellaWar: joined event as class {} (nation={})",
        session.addr(),
        selected_class,
        event_nation
    );

    Ok(())
}

/// Apply preset items from cindwar_items to the player's inventory.
fn apply_preset_items(world: &WorldState, sid: SessionId, setting_id: u8, class_index: u8) {
    let items = world.get_cindwar_items_for_class(setting_id as i16, (class_index + 1) as i16);

    // Build inventory from preset items
    let mut inventory = world.get_inventory(sid);

    // Clear existing inventory (but keep the size)
    for slot in inventory.iter_mut() {
        *slot = UserItemSlot::default();
    }

    // Apply preset items by slot_id
    for item in &items {
        let slot = item.slot_id as usize;
        if slot < inventory.len() {
            inventory[slot] = UserItemSlot {
                item_id: item.item_id as u32,
                durability: item.item_duration,
                count: item.item_count as u16,
                flag: item.item_flag as u8,
                original_flag: 0,
                serial_num: 0,
                expire_time: 0,
            };
        }
    }

    world.set_inventory(sid, inventory);
}

/// Apply class/nation change — shared by class switch and nation change.
async fn cinderella_cha_modify(
    session: &mut ClientSession,
    world: &WorldState,
    sid: SessionId,
    selected_class: u8,
    setting_id: u8,
    _regene: bool,
    nation_change: bool,
) -> anyhow::Result<()> {
    let ps = match world.get_cindwar_player(sid) {
        Some(s) => s,
        None => return Ok(()),
    };

    let event_nation = ps.event_nation;
    let setting = match world.get_cindwar_setting(setting_id) {
        Some(s) => s,
        None => return Ok(()),
    };

    let beginner_level = setting.beginner_level as u8;
    let new_class = match get_new_class(event_nation, selected_class, beginner_level) {
        Some(c) => c,
        None => return Ok(()),
    };
    let new_race = match get_new_race(event_nation, selected_class) {
        Some(r) => r,
        None => return Ok(()),
    };

    // Update character class, race
    world.update_character_stats(sid, |ch| {
        ch.class = new_class;
        ch.race = new_race;
        if nation_change {
            ch.nation = event_nation;
        }
    });

    // Apply preset items
    apply_preset_items(world, sid, setting_id, selected_class);

    // Apply stat / skill preset
    let stat_preset = apply_stat_preset(world, sid, setting_id, selected_class);

    // Send WIZ_CLASS_CHANGE
    let mut class_pkt = Packet::new(Opcode::WizClassChange as u8);
    class_pkt.write_u8(5); // ALL_SKILLPT_CHANGE
    class_pkt.write_u8(1); // success
    class_pkt.write_u32(0);
    if let Some(ref sp) = stat_preset {
        class_pkt.write_u8(sp.skill_freepoint as u8);
    } else {
        class_pkt.write_u8(0);
    }
    session.send_packet(&class_pkt).await?;
    refresh_client(session, world, sid).await?;

    Ok(())
}

/// Handle nationchange sub-opcode.
async fn handle_nation_change(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let selected_nation = reader.read_u8().unwrap_or(0);
    let world = session.world().clone();
    let sid = session.session_id();

    // Validate nation (1 = Karus, 2 = Elmorad)
    if selected_nation != 1 && selected_nation != 2 {
        return Ok(());
    }

    // Must be event user in event zone
    let ps = match world.get_cindwar_player(sid) {
        Some(s) if s.event_user => s,
        _ => return Ok(()),
    };

    let cind_zone = world.cinderella_zone_id();
    let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    if !is_cinderella_zone(zone_id, cind_zone) {
        return Ok(());
    }

    // Not in party
    if world.get_party_id(sid).is_some() {
        return Ok(());
    }

    let now = now_unix();

    // Cooldown check
    if ps.nation_change_cooldown > now {
        let remaining = (ps.nation_change_cooldown - now) as u32;
        let pkt = build_cind_error(true, sub_opcode::TIME_WAIT, remaining);
        session.send_packet(&pkt).await?;
        return Ok(());
    }

    // Already same nation?
    if ps.event_nation == selected_nation {
        let pkt = build_cind_error(true, sub_opcode::ALREADY_NATION, 0);
        session.send_packet(&pkt).await?;
        return Ok(());
    }

    let selected_class = ps.selected_class;
    let setting_id = world.cindwar_event().setting_id;

    // Update nation
    world.update_cindwar_player(sid, |s| {
        s.event_nation = selected_nation;
        s.nation_change_cooldown = now + NATION_CHANGE_COOLDOWN_SECS;
    });

    // Apply nation change
    cinderella_cha_modify(
        session,
        &world,
        sid,
        selected_class,
        setting_id,
        false,
        true,
    )
    .await?;

    let pkt = build_nation_success();
    session.send_packet(&pkt).await?;

    debug!(
        "[{}] CindirellaWar: nation change to {}",
        session.addr(),
        selected_nation
    );

    Ok(())
}

// ── Restore System ───────────────────────────────────────────────────────

/// Restore player's original data and clean up event participation.
/// Called on disconnect, zone exit, or event end.
pub fn cinderella_logout(world: &WorldState, sid: SessionId, exit_game: bool) {
    let ps = match world.remove_cindwar_player(sid) {
        Some(s) if s.event_user => s,
        _ => return,
    };
    restore_player(world, sid, ps);
    if exit_game {
        world.take_cind_refresh_pending(sid);
    }
    world.send_to_session_owned(sid, build_finish());
}

// ── KDA System ───────────────────────────────────────────────────────────

/// Update KDA on PvP kill in Cinderella War zone.
pub fn cinderella_update_kda(world: &WorldState, killer_sid: SessionId, victim_sid: SessionId) {
    // Victim: increment deaths
    world.update_cindwar_player(victim_sid, |s| {
        s.dead_count += 1;
    });
    if let Some(vs) = world.get_cindwar_player(victim_sid) {
        let pkt = build_kda_personal(vs.kill_count, vs.dead_count);
        world.send_to_session_owned(victim_sid, pkt);
    }

    // Killer: increment kills
    let killer_nation = world
        .get_character_info(killer_sid)
        .map(|c| c.nation)
        .unwrap_or(0);
    world.update_cindwar_player(killer_sid, |s| {
        s.kill_count += 1;
    });
    if let Some(ks) = world.get_cindwar_player(killer_sid) {
        let pkt = build_kda_personal(ks.kill_count, ks.dead_count);
        world.send_to_session_owned(killer_sid, pkt);
    }

    // Global nation kill count
    let (e_kills, k_kills) = {
        let mut event = world.cindwar_event_mut();
        if killer_nation == 2 {
            event.elmorad_kills = event.elmorad_kills.saturating_add(1);
        } else {
            event.karus_kills = event.karus_kills.saturating_add(1);
        }
        (event.elmorad_kills, event.karus_kills)
    };

    // Broadcast global KDA to event zone
    let zone_id = world.cinderella_zone_id();
    let global_pkt = build_kda_global(e_kills, k_kills);
    world.broadcast_to_zone(zone_id, Arc::new(global_pkt), None);
}

// ── Timer ────────────────────────────────────────────────────────────────

/// Cinderella War per-second timer tick.
pub fn cinderella_timer_tick(world: &WorldState) {
    let now = now_unix();
    let event = world.cindwar_event();
    if !event.is_on() {
        return;
    }

    let prepare = event.prepare;
    let start = event.start;
    let prepare_time = event.prepare_time;
    let finish_time = event.finish_time;
    let setting_id = event.setting_id;
    drop(event);

    if prepare {
        let remaining = prepare_time.saturating_sub(now);
        let remaining_min = remaining / 60;

        // Countdown announcements
        if remaining > 0 {
            if matches!(remaining_min, 30 | 20 | 10 | 5 | 4 | 3 | 2 | 1)
                && remaining.is_multiple_of(60)
            {
                let msg = format!(
                    "{} minutes until the Fun Class Event begins...",
                    remaining_min
                );
                let zone_id = world.cinderella_zone_id();
                let pkt = super::chat::build_chat_packet(8, 0, 0, "**", &msg, 0, 0, 0);
                world.broadcast_to_zone(zone_id, Arc::new(pkt), None);
            }
            return;
        }

        // Transition: Prepare → War
        let setting = match world.get_cindwar_setting(setting_id) {
            Some(s) => s,
            None => return,
        };

        {
            let mut ev = world.cindwar_event_mut();
            ev.prepare = false;
            ev.start = true;
            ev.finish_time = now + (setting.playtime as u64) * 60;
        }

        // Send starting notification to all event users
        let play_time = (setting.playtime as u32) * 60;
        let starting = build_starting(play_time);
        // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
        let chat_fallback = {
            let msg = format!(
                "[Fun Class] War phase started! Time: {} minutes.",
                setting.playtime
            );
            crate::systems::timed_notice::build_notice_packet(8, &msg)
        };
        // C++ CindirellaTown: everyone goes back to their base when the war starts
        let zone = world.cinderella_zone_id();
        for sid in world.cindwar_all_users() {
            if world.get_position(sid).is_some_and(|p| p.zone_id == zone) {
                super::zone_change::server_teleport_to_zone_force(world, sid, zone, 0.0, 0.0);
            }
        }
        let arc_starting = Arc::new(starting);
        let arc_chat = Arc::new(chat_fallback);
        for sid in world.cindwar_all_users() {
            world.send_to_session_arc(sid, Arc::clone(&arc_starting));
            world.send_to_session_arc(sid, Arc::clone(&arc_chat));
        }

        debug!(
            "CindirellaWar: War phase started (playtime={}min)",
            setting.playtime
        );
    } else if start {
        let remaining = finish_time.saturating_sub(now);
        let remaining_min = remaining / 60;

        // Countdown announcements
        if remaining > 0 {
            if matches!(remaining_min, 30 | 20 | 10 | 5 | 4 | 3 | 2 | 1)
                && remaining.is_multiple_of(60)
            {
                let msg = format!(
                    "{} minutes until the end of the Fun Class event.",
                    remaining_min
                );
                let zone_id = world.cinderella_zone_id();
                let pkt = super::chat::build_chat_packet(8, 0, 0, "**", &msg, 0, 0, 0);
                world.broadcast_to_zone(zone_id, Arc::new(pkt), None);
            }
            return;
        }

        // Event end — distribute rewards and cleanup
        cinderella_finish(world);
    }
}

/// Send every player standing in the event zone to Moradon (C++ KickOutZoneUsers).
pub fn kick_out_event_zone(world: &WorldState, zone_id: u16) {
    if zone_id == 0 {
        return;
    }
    for sid in world.all_ingame_session_ids() {
        if world.get_position(sid).is_some_and(|p| p.zone_id == zone_id) {
            world.mark_cind_refresh_pending(sid);
            super::zone_change::server_teleport_to_zone(world, sid, crate::world::types::ZONE_MORADON, 0.0, 0.0);
        }
    }
}

/// End the Cinderella War event: rank, pay the DB rewards, give everyone their own
/// character back and send the whole zone to Moradon (C++ CindirellaTimer finish block).
fn cinderella_finish(world: &WorldState) {
    let event_zone = world.cinderella_zone_id();
    let users = world.cindwar_all_users();

    // Sort by kill_count DESC, dead_count ASC
    let mut rankings: Vec<(SessionId, u16, u16, u8)> = users
        .iter()
        .filter_map(|&sid| {
            let ps = world.get_cindwar_player(sid)?;
            let nation = world.get_character_info(sid).map(|c| c.nation).unwrap_or(0);
            Some((sid, ps.kill_count, ps.dead_count, nation))
        })
        .collect();
    rankings.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));

    // Rank per nation (top 200 each)
    let mut ranks: std::collections::HashMap<SessionId, i16> = std::collections::HashMap::new();
    let (mut elmorad_rank, mut karus_rank) = (0i16, 0i16);
    for &(sid, _, _, nation) in &rankings {
        let rank = if nation == 2 {
            elmorad_rank += 1;
            elmorad_rank
        } else {
            karus_rank += 1;
            karus_rank
        };
        if rank <= 200 {
            ranks.insert(sid, rank);
        }
    }

    // exp / gold rewards ride on the restore (gained_*), the rest is paid afterwards
    for (&sid, &rank) in &ranks {
        if let Some(reward) = world.get_cindwar_reward(rank) {
            world.update_cindwar_player(sid, |s| {
                s.gained_exp = s.gained_exp.saturating_add(reward.exp_count.max(0) as u64);
                s.gained_noah = s.gained_noah.saturating_add(reward.money_count.max(0) as u32);
            });
        }
    }

    let arc_finish = Arc::new(build_finish());
    for &sid in &users {
        world.send_to_session_arc(sid, Arc::clone(&arc_finish));
    }

    for &sid in &users {
        if let Some(ps) = world.remove_cindwar_player(sid) {
            if ps.event_user {
                restore_player(world, sid, ps);
            }
        }
        // loyalty / cash / items go to the restored character
        if let Some(&rank) = ranks.get(&sid) {
            if let Some(reward) = world.get_cindwar_reward(rank) {
                if reward.loyalty_count > 0 {
                    crate::systems::loyalty::send_loyalty_change(world, sid, reward.loyalty_count, false, true, false);
                }
                if reward.cash_count > 0 {
                    world.update_session(sid, |h| h.knight_cash = h.knight_cash.saturating_add(reward.cash_count as u32));
                }
            }
            let items: Vec<_> = world
                .cindwar_reward_items()
                .iter()
                .filter(|i| i.rank_id == rank && i.item_id > 0)
                .cloned()
                .collect();
            for item in items {
                world.give_item(sid, item.item_id as u32, item.item_count.max(1) as u16);
            }
        }
    }

    // Clear event state, then empty the zone
    {
        let mut ev = world.cindwar_event_mut();
        *ev = CindirellaEventState::default();
    }
    world.set_cinderella_active(false, 0);
    kick_out_event_zone(world, event_zone);

    let pkt = super::chat::build_chat_packet(8, 0, 0, "**", "Fun Class has Finished. Have Fun.", 0, 0, 0);
    world.broadcast_to_all(Arc::new(pkt), None);

    debug!("CindirellaWar: Event finished, {} participants processed", users.len());
}

#[cfg(test)]
mod tests {
    use ko_protocol::Opcode;

    use super::*;

    #[test]
    fn test_cinderella_opcode_value() {
        assert_eq!(Opcode::WizCinderella as u8, 0xE0);
    }

    #[test]
    fn test_cinderella_sub_opcode_values() {
        assert_eq!(sub_opcode::SELECT_CLASS, 0);
        assert_eq!(sub_opcode::NATION_CHANGE, 1);
        assert_eq!(sub_opcode::JOIN_EVENT, 2);
        assert_eq!(sub_opcode::STARTING, 3);
        assert_eq!(sub_opcode::UPDATE_KDA, 4);
        assert_eq!(sub_opcode::FINISH, 5);
        assert_eq!(sub_opcode::SUCCESS, 6);
        assert_eq!(sub_opcode::TIME_WAIT, 7);
        assert_eq!(sub_opcode::NOT_CHANGE, 8);
        assert_eq!(sub_opcode::ALREADY_CLASS, 9);
        assert_eq!(sub_opcode::ALREADY_NATION, 10);
    }

    #[test]
    fn test_max_setting_tiers() {
        assert_eq!(MAX_SETTING_TIERS, 5);
    }

    #[test]
    fn test_max_classes() {
        assert_eq!(MAX_CLASSES, 5);
    }

    #[test]
    fn test_class_change_cooldown() {
        assert_eq!(CLASS_CHANGE_COOLDOWN_SECS, 80);
    }

    #[test]
    fn test_nation_change_cooldown() {
        assert_eq!(NATION_CHANGE_COOLDOWN_SECS, 90);
    }

    #[test]
    fn test_error_cooldown() {
        assert_eq!(ERROR_COOLDOWN_SECS, 5);
    }

    // ── get_new_class tests ──────────────────────────────────────────

    #[test]
    fn test_get_new_class_karus_warrior_high_level() {
        // Karus warrior with beginner_level >= 60
        assert_eq!(get_new_class(1, 0, 65), Some(106));
    }

    #[test]
    fn test_get_new_class_karus_warrior_low_level() {
        // Karus warrior with beginner_level < 60
        assert_eq!(get_new_class(1, 0, 47), Some(105));
    }

    #[test]
    fn test_get_new_class_elmorad_rogue_high_level() {
        assert_eq!(get_new_class(2, 1, 83), Some(208));
    }

    #[test]
    fn test_get_new_class_elmorad_mage() {
        assert_eq!(get_new_class(2, 2, 90), Some(210));
    }

    #[test]
    fn test_get_new_class_elmorad_priest() {
        assert_eq!(get_new_class(2, 3, 65), Some(212));
    }

    #[test]
    fn test_get_new_class_elmorad_kurian() {
        assert_eq!(get_new_class(2, 4, 90), Some(215));
    }

    #[test]
    fn test_get_new_class_invalid_class() {
        assert_eq!(get_new_class(1, 5, 65), None);
        assert_eq!(get_new_class(2, 6, 65), None);
        assert_eq!(get_new_class(1, 255, 65), None);
    }

    #[test]
    fn test_get_new_class_karus_all_classes() {
        // All Karus classes at high level
        assert_eq!(get_new_class(1, 0, 65), Some(106));
        assert_eq!(get_new_class(1, 1, 65), Some(108));
        assert_eq!(get_new_class(1, 2, 65), Some(110));
        assert_eq!(get_new_class(1, 3, 65), Some(112));
        assert_eq!(get_new_class(1, 4, 65), Some(115));
    }

    #[test]
    fn test_get_new_class_elmorad_all_classes() {
        // All Elmorad classes at high level
        assert_eq!(get_new_class(2, 0, 65), Some(206));
        assert_eq!(get_new_class(2, 1, 65), Some(208));
        assert_eq!(get_new_class(2, 2, 65), Some(210));
        assert_eq!(get_new_class(2, 3, 65), Some(212));
        assert_eq!(get_new_class(2, 4, 65), Some(215));
    }

    // ── get_new_race tests ──────────────────────────────────────────

    #[test]
    fn test_get_new_race_karus() {
        assert_eq!(get_new_race(1, 0), Some(1)); // warrior
        assert_eq!(get_new_race(1, 1), Some(2)); // rogue
        assert_eq!(get_new_race(1, 2), Some(4)); // mage
        assert_eq!(get_new_race(1, 3), Some(4)); // priest
        assert_eq!(get_new_race(1, 4), Some(6)); // kurian
    }

    #[test]
    fn test_get_new_race_elmorad() {
        assert_eq!(get_new_race(2, 0), Some(12)); // warrior
        assert_eq!(get_new_race(2, 1), Some(12)); // rogue
        assert_eq!(get_new_race(2, 2), Some(13)); // mage
        assert_eq!(get_new_race(2, 3), Some(13)); // priest
        assert_eq!(get_new_race(2, 4), Some(14)); // kurian
    }

    #[test]
    fn test_get_new_race_invalid() {
        assert_eq!(get_new_race(1, 5), None);
        assert_eq!(get_new_race(2, 255), None);
    }

    // ── is_cinderella_zone tests ─────────────────────────────────────

    #[test]
    fn test_is_cinderella_zone_match() {
        assert!(is_cinderella_zone(110, 110));
    }

    #[test]
    fn test_is_cinderella_zone_no_match() {
        assert!(!is_cinderella_zone(21, 110));
        assert!(!is_cinderella_zone(0, 110));
    }

    // ── Level threshold boundary tests ───────────────────────────────

    #[test]
    fn test_get_new_class_level_boundary_59() {
        // Level 59 is below 60 threshold
        assert_eq!(get_new_class(1, 0, 59), Some(105));
        assert_eq!(get_new_class(2, 0, 59), Some(205));
    }

    #[test]
    fn test_get_new_class_level_boundary_60() {
        // Level 60 is at threshold (not below)
        assert_eq!(get_new_class(1, 0, 60), Some(106));
        assert_eq!(get_new_class(2, 0, 60), Some(206));
    }

    #[test]
    fn test_get_new_class_level_boundary_1() {
        // Minimum level
        assert_eq!(get_new_class(1, 0, 1), Some(105));
        assert_eq!(get_new_class(2, 4, 1), Some(214));
    }

    // ── get_class_index tests (fixed implementation) ─────────────────

    #[test]
    fn test_get_class_index_karus_warriors() {
        assert_eq!(get_class_index(101), Some(0));
        assert_eq!(get_class_index(103), Some(0));
        assert_eq!(get_class_index(106), Some(0));
    }

    #[test]
    fn test_get_class_index_elmorad_warriors() {
        assert_eq!(get_class_index(201), Some(0));
        assert_eq!(get_class_index(205), Some(0));
        assert_eq!(get_class_index(206), Some(0));
    }

    #[test]
    fn test_get_class_index_rogues() {
        assert_eq!(get_class_index(107), Some(1));
        assert_eq!(get_class_index(108), Some(1));
        assert_eq!(get_class_index(207), Some(1));
        assert_eq!(get_class_index(208), Some(1));
    }

    #[test]
    fn test_get_class_index_mages() {
        assert_eq!(get_class_index(109), Some(2));
        assert_eq!(get_class_index(110), Some(2));
        assert_eq!(get_class_index(209), Some(2));
        assert_eq!(get_class_index(210), Some(2));
    }

    #[test]
    fn test_get_class_index_priests() {
        assert_eq!(get_class_index(111), Some(3));
        assert_eq!(get_class_index(112), Some(3));
        assert_eq!(get_class_index(211), Some(3));
        assert_eq!(get_class_index(212), Some(3));
    }

    #[test]
    fn test_get_class_index_kurians() {
        assert_eq!(get_class_index(113), Some(4));
        assert_eq!(get_class_index(115), Some(4));
        assert_eq!(get_class_index(213), Some(4));
        assert_eq!(get_class_index(215), Some(4));
    }

    #[test]
    fn test_get_class_index_invalid() {
        assert_eq!(get_class_index(0), None);
        assert_eq!(get_class_index(100), None);
        assert_eq!(get_class_index(200), None);
        assert_eq!(get_class_index(300), None);
    }

    // ── State struct tests ──────────────────────────────────────────

    #[test]
    fn test_player_state_default() {
        let ps = CindirellaPlayerState::default();
        assert!(!ps.event_user);
        assert!(ps.first_selected);
        assert_eq!(ps.kill_count, 0);
        assert_eq!(ps.dead_count, 0);
        assert_eq!(ps.selected_class, 0);
    }

    #[test]
    fn test_event_state_default() {
        let es = CindirellaEventState::default();
        assert!(!es.prepare);
        assert!(!es.start);
        assert!(!es.is_on());
    }

    #[test]
    fn test_event_state_is_on() {
        let mut es = CindirellaEventState::default();
        assert!(!es.is_on());

        es.prepare = true;
        assert!(es.is_on());

        es.prepare = false;
        es.start = true;
        assert!(es.is_on());

        es.prepare = true;
        assert!(es.is_on());
    }

    // ── Packet builder tests ────────────────────────────────────────

    #[test]
    fn test_build_join_event_packet() {
        let pkt = build_join_event(true, 2, 300, 5, 3, 10, 8);
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        assert_eq!(pkt.data[0], 0xE0); // CINDIRELLA sub
        assert_eq!(pkt.data[1], sub_opcode::JOIN_EVENT);
        assert_eq!(pkt.data[2], 1); // is_prepare = true
        assert_eq!(pkt.data[3], 2); // class_index
                                    // remaining_time (u32 LE) = 300
        assert_eq!(u32::from_le_bytes(pkt.data[4..8].try_into().unwrap()), 300);
        // kills/deaths/k_kills/e_kills (u16 LE each)
        assert_eq!(u16::from_le_bytes(pkt.data[8..10].try_into().unwrap()), 5);
        assert_eq!(u16::from_le_bytes(pkt.data[10..12].try_into().unwrap()), 3);
        assert_eq!(u16::from_le_bytes(pkt.data[12..14].try_into().unwrap()), 10);
        assert_eq!(u16::from_le_bytes(pkt.data[14..16].try_into().unwrap()), 8);
        assert_eq!(pkt.data.len(), 16);
    }

    #[test]
    fn test_build_starting_packet() {
        let pkt = build_starting(600);
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        assert_eq!(pkt.data[0], 0xE0);
        assert_eq!(pkt.data[1], sub_opcode::STARTING);
        assert_eq!(u32::from_le_bytes(pkt.data[2..6].try_into().unwrap()), 600);
        assert_eq!(pkt.data.len(), 6);
    }

    #[test]
    fn test_build_kda_personal_packet() {
        let pkt = build_kda_personal(7, 2);
        assert_eq!(pkt.data[0], 0xE0);
        assert_eq!(pkt.data[1], sub_opcode::UPDATE_KDA);
        assert_eq!(pkt.data[2], 0); // personal type
        assert_eq!(u16::from_le_bytes(pkt.data[3..5].try_into().unwrap()), 7);
        assert_eq!(u16::from_le_bytes(pkt.data[5..7].try_into().unwrap()), 2);
        assert_eq!(pkt.data.len(), 7);
    }

    #[test]
    fn test_build_kda_global_packet() {
        let pkt = build_kda_global(15, 12);
        assert_eq!(pkt.data[0], 0xE0);
        assert_eq!(pkt.data[1], sub_opcode::UPDATE_KDA);
        assert_eq!(pkt.data[2], 1); // global type
        assert_eq!(u16::from_le_bytes(pkt.data[3..5].try_into().unwrap()), 15);
        assert_eq!(u16::from_le_bytes(pkt.data[5..7].try_into().unwrap()), 12);
    }

    #[test]
    fn test_build_finish_packet() {
        let pkt = build_finish();
        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        assert_eq!(pkt.data[0], 0xE0);
        assert_eq!(pkt.data[1], sub_opcode::FINISH);
        assert_eq!(pkt.data.len(), 2);
    }

    #[test]
    fn test_build_select_success_packet() {
        let pkt = build_select_success(3);
        assert_eq!(pkt.data[0], 0xE0);
        assert_eq!(pkt.data[1], sub_opcode::SELECT_CLASS);
        assert_eq!(pkt.data[2], sub_opcode::SUCCESS);
        assert_eq!(pkt.data[3], 3); // class
        assert_eq!(pkt.data.len(), 4);
    }

    #[test]
    fn test_build_nation_success_packet() {
        let pkt = build_nation_success();
        assert_eq!(pkt.data[0], 0xE0);
        assert_eq!(pkt.data[1], sub_opcode::NATION_CHANGE);
        assert_eq!(pkt.data[2], sub_opcode::SUCCESS);
        assert_eq!(pkt.data.len(), 3);
    }

    #[test]
    fn test_build_error_time_wait_packet() {
        let pkt = build_cind_error(false, sub_opcode::TIME_WAIT, 45);
        assert_eq!(pkt.data[0], 0xE0);
        assert_eq!(pkt.data[1], sub_opcode::SELECT_CLASS);
        assert_eq!(pkt.data[2], sub_opcode::TIME_WAIT);
        assert_eq!(u32::from_le_bytes(pkt.data[3..7].try_into().unwrap()), 45);
        assert_eq!(pkt.data.len(), 7);
    }

    #[test]
    fn test_build_error_already_class_packet() {
        let pkt = build_cind_error(false, sub_opcode::ALREADY_CLASS, 0);
        assert_eq!(pkt.data[1], sub_opcode::SELECT_CLASS);
        assert_eq!(pkt.data[2], sub_opcode::ALREADY_CLASS);
        assert_eq!(pkt.data.len(), 3); // no remaining_secs for non-TIME_WAIT
    }

    #[test]
    fn test_build_error_nation_variant() {
        let pkt = build_cind_error(true, sub_opcode::ALREADY_NATION, 0);
        assert_eq!(pkt.data[1], sub_opcode::NATION_CHANGE); // nation variant
        assert_eq!(pkt.data[2], sub_opcode::ALREADY_NATION);
    }

    // ── WIZ_PRESET builder tests ────────────────────────────────────

    #[test]
    fn test_build_preset_stats_packet() {
        let pkt = build_preset_stats([100, 80, 60, 90, 50], 10);
        assert_eq!(pkt.opcode, Opcode::WizPreset as u8);
        assert_eq!(pkt.data[0], 1); // type 1
        assert_eq!(pkt.data[1], 1); // unknown
        assert_eq!(i16::from_le_bytes(pkt.data[2..4].try_into().unwrap()), 100);
        assert_eq!(i16::from_le_bytes(pkt.data[4..6].try_into().unwrap()), 80);
        assert_eq!(pkt.data.len(), 14); // 2 + 5*2 + 2
    }

    #[test]
    fn test_build_preset_skills_packet() {
        let pkt = build_preset_skills([10, 20, 30, 40], 5);
        assert_eq!(pkt.opcode, Opcode::WizPreset as u8);
        assert_eq!(pkt.data[0], 2); // type 2
        assert_eq!(pkt.data[1], 1); // unknown
        assert_eq!(pkt.data[2], 10);
        assert_eq!(pkt.data[3], 20);
        assert_eq!(pkt.data[4], 30);
        assert_eq!(pkt.data[5], 40);
        assert_eq!(pkt.data[6], 5); // free skill pts
        assert_eq!(pkt.data.len(), 7);
    }

    #[test]
    fn test_zone_moradon_constant() {
        assert_eq!(ZONE_MORADON, 21);
    }
}
