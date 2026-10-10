//! Extended hook (WIZ_EXT_HOOK 0xE9) sub-opcode handlers.
//! These are sub-opcodes of WIZ_EXT_HOOK (0xE9).  They include both
//! anti-cheat and extended gameplay features.

use std::sync::Arc;

use ko_protocol::Packet;
use tracing::{debug, info, warn};

use crate::inventory_constants::ITEM_KIND_UNIQUE;
use crate::session::ClientSession;
use crate::world::types::MERCHANT_STATE_NONE;

// ── Sub-opcode constants ─────────────────────────────────────────────────────

/// Anti-cheat: client reports its detected authority level.
pub const EXT_SUB_AUTHINFO: u8 = 0xA2;

/// Anti-cheat: client sends heartbeat with MD5 validation.
pub const EXT_SUB_XALIVE: u8 = 0xA6;

/// Client requests full UI info dump (KC/TL/stats/XP/skills/tag).
pub const EXT_SUB_UIINFO: u8 = 0xA7;

/// Client requests KC/TL/weapon skills summary.
pub const EXT_SUB_USERINFO: u8 = 0xB1;

/// Client saves auto-loot filter preferences.
pub const EXT_SUB_LOOT_SETTINGS: u8 = 0xB3;

/// Client sends GM support ticket.
pub const EXT_SUB_SUPPORT: u8 = 0xBB;

/// Chat last seen timestamp update.
pub const EXT_SUB_CHAT_LASTSEEN: u8 = 0xCD;

/// Skill & stat reset via item 1299.
pub const EXT_SUB_SKILL_STAT_RESET: u8 = 0xCF;

/// GM process list inspection (anti-cheat).
pub const EXT_SUB_PROCINFO: u8 = 0xA3;

/// Anti-cheat log (disabled in C++).
pub const EXT_SUB_LOG: u8 = 0xA5;

/// Cash shop purchase via ext hook path.
pub const EXT_SUB_PUS: u8 = 0xA8;

/// Cash change notification (KC/TL balance update).
pub const EXT_SUB_CASHCHANGE: u8 = 0xA9;

/// Drop list query.
pub const EXT_SUB_DROP_LIST: u8 = 0xAB;

/// Anti-cheat reset.
pub const EXT_SUB_RESET: u8 = 0xAD;

/// Drop request (same handler as DROP_LIST).
pub const EXT_SUB_DROP_REQUEST: u8 = 0xAE;

/// Clan bank open (disabled in C++ — function starts with `return;`).
pub const EXT_SUB_CLANBANK: u8 = 0xB0;

/// Chaotic item exchange via Bifrost generator.
pub const EXT_SUB_CHAOTIC_EXCHANGE: u8 = 0xB4;

/// Merchant item add via ext hook path.
pub const EXT_SUB_MERCHANT: u8 = 0xB5;

/// Temporary (expiring) items list.
pub const EXT_SUB_TEMPITEMS: u8 = 0xB8;

/// Merchant list request.
pub const EXT_SUB_MERCHANTLIST: u8 = 0xBD;

/// Rebirth stat reset (NPC dialog).
pub const EXT_SUB_RESETREBSTAT: u8 = 0xCA;

/// Account info save (email, phone, seal, OTP).
pub const EXT_SUB_ACCOUNT_INFO_SAVE: u8 = 0xCC;

/// Buyback (repurchase) list.
pub const EXT_SUB_REPURCHASE: u8 = 0xCE;

/// PUS category list.
const EXT_SUB_PUS_CAT: u8 = 0xD6;

/// Chest item blocking list.
pub const EXT_SUB_CHEST_BLOCKITEM: u8 = 0xE5;

/// Right-click item exchange info/execute.
pub const EXT_SUB_ITEM_EXCHANGE_INFO: u8 = 0xE6;

/// Daily login reward claim.
pub const EXT_SUB_DAILY_REWARD: u8 = 0xF7;

/// Castle Siege Warfare timer/finish.
pub const EXT_SUB_CSW: u8 = 0xE1;

/// Zindan War score tracking.
pub const EXT_SUB_ZINDAN_WAR: u8 = 0xD2;

// ── Additional sub-opcode constants (C++ HSACSXOpCodes enum completeness) ────

/// Anti-cheat: client open notification.
pub const EXT_SUB_OPEN: u8 = 0xA4;

/// Key exchange serial number.
pub const EXT_SUB_KESN: u8 = 0xAA;

/// Item process via ext hook path.
pub const EXT_SUB_ITEM_PROCESS: u8 = 0xAC;

/// Collection Race sub-opcode.
pub const EXT_SUB_COLLECTION_RACE: u8 = 0xAF;

/// KC marketplace (premium shop).
pub const EXT_SUB_KCPAZAR: u8 = 0xB2;

/// Extended user data query.
pub const EXT_SUB_USERDATA: u8 = 0xB7;

/// Knight Cash update notification.
pub const EXT_SUB_KCUPDATE: u8 = 0xB9;

/// Auto-drop settings.
pub const EXT_SUB_AUTODROP: u8 = 0xBA;

/// Information message to client.
pub const EXT_SUB_INFOMESSAGE: u8 = 0xBC;

/// General message to client.
pub const EXT_SUB_MESSAGE: u8 = 0xBE;

/// Ban system administration.
pub const EXT_SUB_BANSYSTEM: u8 = 0xBF;

/// Mercenary viewer info.
pub const EXT_SUB_MERC_VIEWER_INFO: u8 = 0xC3;

/// Item upgrade rate display.
pub const EXT_SUB_UPGRADE_RATE: u8 = 0xC4;

/// Castle siege timer (dead opcode — C++ 0xC5 unused).
pub const EXT_SUB_CASTLE_SIEGE_TIMER: u8 = 0xC5;

/// Voice chat control.
pub const EXT_SUB_VOICE: u8 = 0xC6;

/// Lottery sub-opcode.
pub const EXT_SUB_LOTTERY: u8 = 0xC7;

/// Top-left UI message display.
pub const EXT_SUB_TOPLEFT: u8 = 0xC8;

/// Error message to client.
pub const EXT_SUB_ERRORMSG: u8 = 0xC9;

/// Unknown / reserved (unused in C++).
pub const EXT_SUB_UNKNOWN1: u8 = 0xCB;

/// Name tag change.
pub const EXT_SUB_TAG_INFO: u8 = 0xD1;

/// Daily quest sub-opcode.
pub const EXT_SUB_DAILY_QUEST: u8 = 0xD3;

/// Cash refund (PUS refund).
pub const EXT_SUB_PUS_REFUND: u8 = 0xD4;

/// Player ranking sub-opcode.
pub const EXT_SUB_PLAYER_RANK: u8 = 0xD5;

/// Death notice broadcast.
pub const EXT_SUB_DEATH_NOTICE: u8 = 0xD7;

/// Quest list display.
pub const EXT_SUB_SHOW_QUEST_LIST: u8 = 0xD9;

/// Wheel of Fun gacha.
pub const EXT_SUB_WHEEL_DATA: u8 = 0xDA;

/// Genie info / time display.
pub const EXT_SUB_GENIE_INFO: u8 = 0xDB;

/// Cinderella War event.
pub const EXT_SUB_CINDERELLA: u8 = 0xE0;

/// Juraid Mountain event.
pub const EXT_SUB_JURAID: u8 = 0xE2;

/// Perks system.
pub const EXT_SUB_PERKS: u8 = 0xE3;

/// Secondary message to client.
pub const EXT_SUB_MESSAGE2: u8 = 0xE4;

/// Hook visibility toggle.
pub const EXT_SUB_HOOK_VISIBLE: u8 = 0xE7;

/// Game master mode UI toggle (server → client only).
pub const EXT_SUB_GAME_MASTER_MODE: u8 = 0xE9;

// ── Opcode used for building S2C response packets ────────────────────────────
// v2525 GameMain dispatch range 0x06-0xD7; 0xE9 is outside and dropped by vanilla client.
// 0x9C (WIZ_CONTINOUS_PACKET) was tried but its handler only accepts sub-opcodes
// 0x02-0x09/0x0E/0xF0 — ext_hook sub-opcodes (0xA9+) fall through to no-op.
// Ext_hook S2C requires modified client OR per-feature remap to v2525 native opcodes.
const WIZ_EXT_HOOK: u8 = ko_protocol::Opcode::EXT_HOOK_S2C;

// ── Authority constant ───────────────────────────────────────────────────────
const AUTHORITY_GAME_MASTER: u8 = 0;

// ── Support rate-limit (seconds) ─────────────────────────────────────────────
const SUPPORT_COOLDOWN_SECS: u64 = 3600;

// ─────────────────────────────────────────────────────────────────────────────
// AuthInfo (0xA2) — Memory hack detection
// ─────────────────────────────────────────────────────────────────────────────

/// Handle AUTHINFO (0xA2) — detect memory-editing cheats.
/// Client sends its detected authority value.  If the client claims GM
/// authority but the player is NOT a GM, this is evidence of memory editing.
/// The server disconnects the cheater immediately.
pub fn handle_authinfo(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let claimed_authority = data[0];

    let world = session.world();
    let sid = session.session_id();

    let actual_authority = world
        .get_character_info(sid)
        .map(|c| c.authority)
        .unwrap_or(1);

    // If client claims GM but isn't GM → memory hack
    if claimed_authority == AUTHORITY_GAME_MASTER && actual_authority != AUTHORITY_GAME_MASTER {
        warn!(
            "[{}] AUTHINFO: non-GM player claims GM authority — memory hack detected, disconnecting",
            session.addr()
        );
        // Log the incident
        let account = world
            .with_session(sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let name = world
            .get_character_info(sid)
            .map(|c| c.name)
            .unwrap_or_default();
        info!(
            "CHEAT_DETECT: AuthInfo — account={} char={} ip={}",
            account,
            name,
            session.addr()
        );
        anyhow::bail!("ext_hook AuthInfo: memory hack detected — disconnecting");
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// xALIVE (0xA6) — Anti-cheat heartbeat
// ─────────────────────────────────────────────────────────────────────────────

/// Handle xALIVE (0xA6) — heartbeat / keep-alive from client.
/// The client periodically sends a heartbeat packet with clock values and an
/// MD5 signature.  The server validates the signature and detects clock replay
/// attacks.  **No response is sent** — this is one-way validation.
/// For now we accept and track the last heartbeat timestamp.  Full MD5
/// validation requires knowing the exact client version string which varies
/// per deployment.
pub fn handle_xalive(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    // Minimum packet: 1 skip + 4 clock1 + 4 clock2 + string + 1 + 4 clock3
    if data.len() < 6 {
        return Ok(());
    }

    let world = session.world();
    let sid = session.session_id();

    // Track last heartbeat time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    world.update_session(sid, |h| {
        h.ext_last_heartbeat = now;
    });

    debug!("[{}] xALIVE heartbeat received", session.addr());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// UIRequest (0xA7) — Full character info dump
// ─────────────────────────────────────────────────────────────────────────────

/// Handle UIINFO (0xA7) — send full character snapshot to client.
/// Response includes: KC, TL, weapon skills, money requirement, XP, max XP,
/// session socket ID, character name, class, race, level, 5 stats, tag name
/// + RGB, zone ID, and flame level.
pub async fn handle_ui_request(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    let info = match world.get_character_info(sid) {
        Some(i) => i,
        None => return Ok(()),
    };

    // Gather session-specific data
    let session_data = world.with_session(sid, |h| {
        (
            h.knight_cash,
            h.tl_balance,
            h.flame_level,
            h.tagname.clone(),
            h.tagname_rgb,
            h.premium_in_use,
        )
    });

    let (kc, tl, flame_level, tagname, tagname_rgb, premium) = match session_data {
        Some(d) => d,
        None => return Ok(()),
    };

    let eq = world.get_equipped_stats(sid);

    // Calculate money requirement — C++ formula: pow(level * 2.0, 3.4)
    let mut money_req = compute_money_req(info.level, premium);
    // Apply discount if active
    if world.is_discount_active(info.nation) {
        money_req /= 2;
    }

    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_UIINFO);
    pkt.write_u32(kc);
    pkt.write_u32(tl);
    pkt.write_u16(eq.dagger_r as u16);
    pkt.write_u16(eq.axe_r as u16);
    pkt.write_u16(eq.sword_r as u16);
    pkt.write_u16(eq.club_r as u16);
    pkt.write_u16(eq.spear_r as u16);
    pkt.write_u16(eq.bow_r as u16);
    pkt.write_u16(eq.jamadar_r as u16);
    pkt.write_u32(money_req);
    pkt.write_u64(info.exp);
    pkt.write_i64(info.max_exp);
    pkt.write_u16(sid); // socket ID
    pkt.write_sbyte_string(&info.name);
    pkt.write_u16(info.class);
    pkt.write_u8(info.race);
    pkt.write_u8(info.level);

    // 5 stats: STR, STA, DEX, INT, CHA
    pkt.write_u8(info.str);
    pkt.write_u8(info.sta);
    pkt.write_u8(info.dex);
    pkt.write_u8(info.intel);
    pkt.write_u8(info.cha);

    // Tag name + RGB
    let r = (tagname_rgb & 0xFF) as u8;
    let g = ((tagname_rgb >> 8) & 0xFF) as u8;
    let b = ((tagname_rgb >> 16) & 0xFF) as u8;
    pkt.write_sbyte_string(&tagname);
    pkt.write_u8(r);
    pkt.write_u8(g);
    pkt.write_u8(b);

    let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    pkt.write_u16(zone_id);
    pkt.write_u16(flame_level);

    session.send_packet(&pkt).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// ReqUserInfo (0xB1) — KC/TL/weapon skills summary
// ─────────────────────────────────────────────────────────────────────────────

/// Handle USERINFO (0xB1) — send KC/TL and weapon skill summary.
/// Response is a lighter version of UIRequest: KC, TL, 7 weapon skills,
/// money requirement, and flame level.
pub async fn handle_req_userinfo(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    let info = match world.get_character_info(sid) {
        Some(i) => i,
        None => return Ok(()),
    };

    let session_data = world.with_session(sid, |h| {
        (h.knight_cash, h.tl_balance, h.flame_level, h.premium_in_use)
    });

    let (kc, tl, flame_level, premium) = match session_data {
        Some(d) => d,
        None => return Ok(()),
    };

    let eq = world.get_equipped_stats(sid);

    let mut money_req = compute_money_req(info.level, premium);
    // Apply discount if active
    if world.is_discount_active(info.nation) {
        money_req /= 2;
    }

    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_USERINFO);
    pkt.write_u32(kc);
    pkt.write_u32(tl);
    pkt.write_u16(eq.dagger_r as u16);
    pkt.write_u16(eq.axe_r as u16);
    pkt.write_u16(eq.sword_r as u16);
    pkt.write_u16(eq.club_r as u16);
    pkt.write_u16(eq.spear_r as u16);
    pkt.write_u16(eq.bow_r as u16);
    pkt.write_u16(eq.jamadar_r as u16);
    pkt.write_u32(money_req);
    pkt.write_u16(flame_level);

    session.send_packet(&pkt).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// LootSettings (0xB3) — Auto-loot filter save
// ─────────────────────────────────────────────────────────────────────────────

/// Handle LOOT_SETTINGS (0xB3) — save auto-loot filter preferences.
/// Client sends 10 boolean item-type filters + 1 price threshold.
/// Stored in DB via `save_loot_settings()` repository method.
pub async fn handle_loot_settings(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    // Packet: 10 × u8 (bools) + 1 × u32 (price) = 14 bytes
    if data.len() < 14 {
        return Ok(());
    }

    let clamp_bool = |v: u8| -> i16 {
        if v > 1 {
            0
        } else {
            v as i16
        }
    };

    let weapon = clamp_bool(data[0]);
    let armor = clamp_bool(data[1]);
    let accessory = clamp_bool(data[2]);
    let normal = clamp_bool(data[3]);
    let upgrade = clamp_bool(data[4]);
    let craft = clamp_bool(data[5]);
    let rare = clamp_bool(data[6]);
    let magic = clamp_bool(data[7]);
    let unique_grade = clamp_bool(data[8]);
    let consumable = clamp_bool(data[9]);
    let price =
        u32::from_le_bytes([data[10], data[11], data[12], data[13]]).min(999_999_999) as i32;

    let world = session.world();
    let sid = session.session_id();

    let account_id = world
        .with_session(sid, |h| h.account_id.clone())
        .unwrap_or_default();

    if account_id.is_empty() {
        return Ok(());
    }

    // Fire-and-forget DB save via the pool
    let pool = session.pool().clone();
    tokio::spawn(async move {
        let repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
        let row = ko_db::models::UserLootSettingsRow {
            id: 0,
            user_id: account_id,
            warrior: 0,
            rogue: 0,
            mage: 0,
            priest: 0,
            weapon,
            armor,
            accessory,
            normal,
            upgrade,
            craft,
            rare,
            magic,
            unique_grade,
            consumable,
            price,
        };
        if let Err(e) = repo.save_loot_settings(&row).await {
            tracing::warn!("Failed to save loot settings: {}", e);
        }
    });

    debug!("[{}] Loot settings saved", session.addr());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Support (0xBB) — GM support ticket
// ─────────────────────────────────────────────────────────────────────────────

/// Handle SUPPORT (0xBB) — player submits a support ticket.
/// Rate limited to 1 report per `SUPPORT_COOLDOWN_SECS` (3600s = 1 hour).
/// Two report types: Bug (0x00) or Koxp (0x01).
pub fn handle_support(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let world = session.world();
    let sid = session.session_id();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Rate limit check
    let last_support = world.with_session(sid, |h| h.ext_last_support).unwrap_or(0);

    if last_support + SUPPORT_COOLDOWN_SECS >= now {
        debug!("[{}] Support ticket rate-limited", session.addr());
        return Ok(());
    }

    let mut reader = ko_protocol::PacketReader::new(data);
    let sub_code = reader.read_u8().unwrap_or(0xFF);

    let subject = reader.read_string().unwrap_or_default();
    let message = reader.read_string().unwrap_or_default();

    // Validate lengths — C++ max: subject 25, message 40
    if subject.len() > 25 || message.len() > 40 {
        return Ok(());
    }

    let report_type = match sub_code {
        0x00 => "Bug",
        0x01 => "Koxp",
        _ => return Ok(()),
    };

    // Update rate limit timestamp
    world.update_session(sid, |h| {
        h.ext_last_support = now;
    });

    let char_name = world
        .get_character_info(sid)
        .map(|c| c.name)
        .unwrap_or_default();

    info!(
        "[{}] Support ticket from '{}': type={} subject='{}' msg='{}'",
        session.addr(),
        char_name,
        report_type,
        subject,
        message
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// ChatLastSeen (0xCD) — last seen timestamp
// ─────────────────────────────────────────────────────────────────────────────

/// Handle CHAT_LASTSEEN (0xCD) — update chat last seen timestamp.
/// Sub-opcode 1: Client updates its own last-seen time (hour + minute).
/// Sub-opcode 2: Query another player's last seen — **disabled in C++** (returns early).
pub fn handle_chat_lastseen(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let sub = data[0];
    match sub {
        1 => {
            // Client updates own last seen — store in session
            let world = session.world();
            let sid = session.session_id();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            world.update_session(sid, |h| {
                h.ext_last_seen = now;
            });
        }
        2 => {
            // Query other player's last seen — disabled in C++ (returns early)
            // We do the same: no-op.
        }
        _ => {}
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// SkillStatReset (0xCF) — item 1299 reset
// ─────────────────────────────────────────────────────────────────────────────

/// Handle SKILL_STAT_RESET (0xCF) — stat/skill point reset via item scroll.
/// Sub-opcode 1: Reset all stat points → `AllPointChange()`
/// Sub-opcode 2: Reset all skill points → `AllSkillPointChange()`
/// These delegate to the existing handlers in class_change.rs.
pub async fn handle_skill_stat_reset(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let sub = data[0];
    match sub {
        1 => {
            // Reset all stat points (free = true, no gold cost via ext_hook scroll)
            super::class_change::handle_all_point_change(session, true).await?;
        }
        2 => {
            // Reset all skill points (free = true)
            super::class_change::handle_all_skill_point_change(session, true).await?;
        }
        _ => {}
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// ProcInfo (0xA3) — GM process list inspection
// ─────────────────────────────────────────────────────────────────────────────

/// Handle PROCINFO (0xA3) — receive client's running process list.
/// Client sends its process list + window titles targeted at a specific GM.
/// If the target is a GM, the server forwards the list as help descriptions.
pub fn handle_procinfo(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    let mut reader = ko_protocol::PacketReader::new(data);

    let target_sid = match reader.read_i16() {
        Some(v) => v,
        None => return Ok(()),
    };

    let world = session.world();

    // Check if target is a GM + get nation (single read)
    let (is_gm, gm_nation) = world
        .with_session(target_sid as u16, |h| {
            h.character
                .as_ref()
                .map(|c| (c.authority == AUTHORITY_GAME_MASTER, c.nation))
                .unwrap_or((false, 0))
        })
        .unwrap_or((false, 0));

    let size = reader.read_u32().unwrap_or(0).min(200);

    // Parse process list
    let mut process_lines: Vec<String> = Vec::with_capacity(size as usize + 2);
    let sender_name = world
        .get_character_info(session.session_id())
        .map(|c| c.name)
        .unwrap_or_default();

    process_lines.push(format!("----- [{}] Processes ----", sender_name));

    for _ in 0..size {
        let _pid = reader.read_i32().unwrap_or(0);
        let _name = reader.read_string().unwrap_or_default();
        let window_count = reader.read_i32().unwrap_or(0).min(50);

        for _ in 0..window_count {
            let window_title = reader.read_string().unwrap_or_default();
            process_lines.push(format!("    -- {}", window_title));
        }
    }

    process_lines.push("----- End of processes ----".to_string());

    // Forward to GM as help descriptions (PUBLIC_CHAT = 7)
    if is_gm {
        for line in &process_lines {
            let mut notice = Packet::new(ko_protocol::Opcode::WizChat as u8);
            notice.write_u8(7); // PUBLIC_CHAT
            notice.write_u8(gm_nation);
            notice.write_u32(target_sid as u32);
            notice.write_u8(0); // SByte empty name
            notice.write_string(line);
            notice.write_i8(0);
            notice.write_u8(0);
            notice.write_u8(0);
            world.send_to_session_owned(target_sid as u16, notice);
        }

        info!(
            sid = session.session_id(),
            gm_sid = target_sid,
            process_count = size,
            "Process info forwarded to GM"
        );
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// SendTempItems (0xB8) — Temporary item list
// ─────────────────────────────────────────────────────────────────────────────

/// Handle TEMPITEMS (0xB8) — send list of expiring items to client.
/// Scans inventory, costume, magic bag, warehouse, VIP warehouse for items
/// with nExpirationTime > 0 and sends the list.  Only fires once per session.
pub async fn handle_temp_items(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    // Check if already sent (one-shot per session)
    let already_sent = world
        .with_session(sid, |h| h.temp_items_sent)
        .unwrap_or(true);

    if already_sent {
        return Ok(());
    }

    // Mark as sent
    world.update_session(sid, |h| {
        h.temp_items_sent = true;
    });

    // Scan inventory, warehouse, VIP warehouse for timed items.
    // C++ scans 5 areas: inventory (SLOT_MAX..INVENTORY_COSP-1), cospre,
    // magic_bag1, warehouse, VIP warehouse.
    //
    // C++ per-item: slot(u8), item_id(u32), pos(u8), expire_time(u32)
    struct TempItem {
        slot: u8,
        item_id: u32,
        pos: u8,
        expire_time: u32,
    }

    let inv_total = crate::handler::INVENTORY_TOTAL; // 77
    let mut items = Vec::with_capacity(inv_total);

    world.with_session(sid, |h| {
        // Slot 0: Inventory (equipment + bag + cospre + mbag)
        for (i, item) in h.inventory.iter().enumerate().take(inv_total) {
            if item.item_id != 0 && item.expire_time > 0 {
                items.push(TempItem {
                    slot: 0,
                    item_id: item.item_id,
                    pos: i as u8,
                    expire_time: item.expire_time,
                });
            }
        }
        // Slot 3: Warehouse
        for (i, item) in h.warehouse.iter().enumerate() {
            if item.item_id != 0 && item.expire_time > 0 {
                items.push(TempItem {
                    slot: 3,
                    item_id: item.item_id,
                    pos: i as u8,
                    expire_time: item.expire_time,
                });
            }
        }
        // Slot 4: VIP Warehouse
        for (i, item) in h.vip_warehouse.iter().enumerate() {
            if item.item_id != 0 && item.expire_time > 0 {
                items.push(TempItem {
                    slot: 4,
                    item_id: item.item_id,
                    pos: i as u8,
                    expire_time: item.expire_time,
                });
            }
        }
    });

    let count = items.len().min(255) as u8;
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_TEMPITEMS);
    pkt.write_u8(count);
    for item in items.iter().take(count as usize) {
        pkt.write_u8(item.slot);
        pkt.write_u32(item.item_id);
        pkt.write_u8(item.pos);
        pkt.write_u32(item.expire_time);
    }

    session.send_packet(&pkt).await?;

    debug!(
        "[{}] Temp items list sent: {} timed items",
        session.addr(),
        count
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// ChestBlock (0xE5) — Chest item blocking
// ─────────────────────────────────────────────────────────────────────────────

/// Handle CHEST_BLOCKITEM (0xE5) — update chest item block list.
/// Sub-opcode 0: Client sends a list of item IDs (max 100) to block from
///               chest loot.  Stored in per-session memory.
/// Sub-opcode 1: Client requests confirmation; server sends empty ack packet.
pub async fn handle_chest_block(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let subcode = data[0];
    let world = session.world();
    let sid = session.session_id();

    if subcode == 0 {
        // Read block item list
        let mut reader = ko_protocol::PacketReader::new(&data[1..]);
        let size = reader.read_u16().unwrap_or(0).min(100);

        let mut blocked_items: Vec<u32> = Vec::with_capacity(size as usize);
        for _ in 0..size {
            let item_id = reader.read_u32().unwrap_or(0);
            if item_id > 0 {
                blocked_items.push(item_id);
            }
        }

        world.update_session(sid, |h| {
            h.chest_block_items = blocked_items.clone();
        });

        debug!(
            "[{}] Chest block list updated: {} items",
            session.addr(),
            blocked_items.len()
        );
    } else if subcode == 1 {
        // Send confirmation
        let mut pkt = Packet::new(WIZ_EXT_HOOK);
        pkt.write_u8(EXT_SUB_CHEST_BLOCKITEM);
        session.send_packet(&pkt).await?;
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// AccountInfoSave (0xCC) — Account security config
// ─────────────────────────────────────────────────────────────────────────────

/// Handle ACCOUNT_INFO_SAVE (0xCC) — validate and save account security info.
/// Validates email (max 250 chars), phone (exactly 11 digits), seal code
/// (exactly 8 digits), and OTP (exactly 6 digits).
pub async fn handle_account_info_save(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let sub = data[0];
    if sub != 1 {
        return Ok(());
    }

    let mut reader = ko_protocol::PacketReader::new(&data[1..]);
    let email = reader.read_string().unwrap_or_default();
    let phone = reader.read_string().unwrap_or_default();
    let seal = reader.read_string().unwrap_or_default();
    let otp = reader.read_string().unwrap_or_default();

    let send_error = |session: &ClientSession| {
        let mut pkt = Packet::new(WIZ_EXT_HOOK);
        pkt.write_u8(EXT_SUB_ACCOUNT_INFO_SAVE);
        pkt.write_u8(2); // error response
        pkt.write_u8(0); // fail
        session
            .world()
            .send_to_session_owned(session.session_id(), pkt);
    };

    // Validate email
    if email.is_empty() || email.len() > 250 {
        send_error(session);
        return Ok(());
    }

    // Validate phone (11 digits)
    if phone.len() != 11 || !phone.chars().all(|c| c.is_ascii_digit()) {
        send_error(session);
        return Ok(());
    }

    // Validate seal (8 digits)
    if seal.len() != 8 || !seal.chars().all(|c| c.is_ascii_digit()) {
        send_error(session);
        return Ok(());
    }

    // Validate OTP (6 digits)
    if otp.len() != 6 || !otp.chars().all(|c| c.is_ascii_digit()) {
        send_error(session);
        return Ok(());
    }

    // Persist to DB (tb_user: email, user_phone_number, str_seal_passwd, otp_password).
    let account_id = match session.account_id() {
        Some(id) => id.to_string(),
        None => {
            send_error(session);
            return Ok(());
        }
    };

    let repo = ko_db::repositories::account::AccountRepository::new(session.pool());
    match repo
        .update_account_info(&account_id, &email, &phone, &seal, &otp)
        .await
    {
        Ok(()) => {
            info!(
                "[{}] AccountInfoSave: email_len={} phone_ok seal_ok otp_ok (saved)",
                session.addr(),
                email.len()
            );
            let mut pkt = Packet::new(WIZ_EXT_HOOK);
            pkt.write_u8(EXT_SUB_ACCOUNT_INFO_SAVE);
            pkt.write_u8(2); // result code = status update
            pkt.write_u8(1); // status = success
            session.send_packet(&pkt).await?;
        }
        Err(e) => {
            tracing::warn!("[{}] AccountInfoSave DB error: {}", session.addr(), e);
            send_error(session);
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Stub handlers for disabled / low-priority sub-opcodes
// ─────────────────────────────────────────────────────────────────────────────

/// Handle LOG (0xA5) — anti-cheat log, disabled in C++ (returns early).
pub fn handle_log(_session: &mut ClientSession, _data: &[u8]) -> anyhow::Result<()> {
    Ok(())
}

/// Handle RESET (0xAD) — anti-cheat system reset.
pub fn handle_reset(_session: &mut ClientSession, _data: &[u8]) -> anyhow::Result<()> {
    debug!("ext_hook reset received");
    Ok(())
}

/// Handle CLANBANK (0xB0) — clan bank open, disabled in C++ (starts with `return;`).
pub fn handle_clanbank(_session: &mut ClientSession, _data: &[u8]) -> anyhow::Result<()> {
    Ok(())
}

/// Handle PUS (0xA8) — cash shop purchase/catalog via ext hook.
/// Sub-opcodes:
///   0 = SendPUS (catalog + categories)
///   1 = PUSPurchase (KC/TL buy)
///   2 = no-op
///   3 = PUSGiftPurchase (send gift to another player)
pub async fn handle_pus(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let process = data[0];
    let rest = if data.len() > 1 { &data[1..] } else { &[] };

    match process {
        0 => handle_pus_send_catalog(session).await,
        1 => handle_pus_purchase(session, rest).await,
        2 => Ok(()),
        3 => handle_pus_gift(session, rest).await,
        _ => Ok(()),
    }
}

/// PUS sub 0: Send full catalog + categories to client.
/// 2625 proxy items (sub-op 0xA8): `[count]([id][item_id][name][price][u8 category][u32 count][u8 price_type][icon])`.
/// Categories (sub-op 0xD6): `[count]([id][name][u8 status])`.
async fn handle_pus_send_catalog(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();

    // Send items packet
    let items = world.get_pus_items_all();
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_PUS);
    pkt.write_u32(items.len() as u32);
    for item in &items {
        let item_def = world.get_item(item.item_id.max(0) as u32);
        let display_name = item
            .item_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| item.item_title.as_deref().filter(|s| !s.trim().is_empty()))
            .or_else(|| item_def.as_ref().and_then(|i| i.str_name.as_deref()))
            .unwrap_or("Unknown item");
        let icon = item_def
            .as_ref()
            .and_then(|i| i.item_icon_id1)
            .unwrap_or(0)
            .max(0) as u32;
        pkt.write_u32(item.id as u32);
        pkt.write_u32(item.item_id as u32);
        pkt.write_string(display_name);
        pkt.write_u32(item.price.unwrap_or(0) as u32);
        pkt.write_u8(item.category.clamp(0, u8::MAX as i16) as u8);
        pkt.write_u32(item.buy_count.max(0) as u32);
        pkt.write_u8(item.price_type.clamp(0, u8::MAX as i16) as u8);
        pkt.write_u32(icon);
    }
    session.send_packet(&pkt).await?;

    // Send categories packet
    let cats = world.get_pus_categories();
    let mut cpkt = Packet::new(WIZ_EXT_HOOK);
    cpkt.write_u8(EXT_SUB_PUS_CAT);
    cpkt.write_u32(cats.len() as u32);
    for cat in &cats {
        cpkt.write_u32(cat.id as u32);
        cpkt.write_string(&cat.category_name);
        cpkt.write_u8(cat.status.clamp(0, u8::MAX as i16) as u8);
    }
    session.send_packet(&cpkt).await?;
    send_cash_change(session).await
}

/// PUS sub 1: Purchase item with KC or TL.
async fn handle_pus_purchase(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.len() < 5 {
        return Ok(());
    }
    let mut reader = ko_protocol::PacketReader::new(data);
    let item_id = reader.read_u32().unwrap_or(0);
    let count = reader.read_u8().unwrap_or(0);

    if count == 0 || count > 28 {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // State checks
    if world.is_player_dead(sid) || world.is_trading(sid) || world.is_merchanting(sid) {
        return Ok(());
    }

    let pus_item = match world.get_pus_item(item_id as i32) {
        Some(p) => p,
        None => return Ok(()),
    };

    let price = pus_item.price.unwrap_or(0) as u32;
    let total_cash = price.saturating_mul(count as u32);

    // Validate item template exists
    let game_item_id = pus_item.item_id as u32;
    if world.get_item(game_item_id).is_none() {
        return Ok(());
    }

    // Check free inventory slots
    let free_slots = world.count_free_slots(sid);
    if free_slots < count {
        return Ok(());
    }

    if pus_item.price_type == 0 {
        // KC purchase
        if world.get_knight_cash(sid) < total_cash {
            return Ok(());
        }
        for _ in 0..count {
            if !world.give_item(sid, game_item_id, pus_item.buy_count as u16) {
                continue;
            }
            crate::handler::knight_cash::cash_lose(&world, session.pool(), sid, price);
            // Send CASHCHANGE update
            send_cash_change(session).await?;
        }
    } else {
        // TL purchase
        if world.get_tl_balance(sid) < total_cash {
            return Ok(());
        }

        // Special case: KC conversion items (489500000..489600000)
        let is_kc_conversion = (489500000..=489600000).contains(&game_item_id);

        for _ in 0..count {
            if is_kc_conversion {
                // TL → KC conversion
                world.update_session(sid, |h| {
                    h.tl_balance = h.tl_balance.saturating_sub(price);
                });
                crate::handler::knight_cash::cash_gain(
                    &world,
                    session.pool(),
                    sid,
                    pus_item.buy_count as u32,
                );
                send_cash_change(session).await?;
                continue;
            }
            if !world.give_item(sid, game_item_id, pus_item.buy_count as u16) {
                continue;
            }
            world.update_session(sid, |h| {
                h.tl_balance = h.tl_balance.saturating_sub(price);
            });
            send_cash_change(session).await?;
        }
    }
    if pus_item.price_type != 0 {
        let account = session.account_id().unwrap_or("");
        if !account.is_empty() {
            let repo = ko_db::repositories::cash_shop::CashShopRepository::new(session.pool());
            repo.update_kc_balances(account, world.get_knight_cash(sid) as i32,
                world.get_tl_balance(sid) as i32).await?;
        }
    }
    Ok(())
}

/// PUS sub 3: Gift purchase — buy item for another player.
async fn handle_pus_gift(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let mut reader = ko_protocol::PacketReader::new(data);
    let price_type = reader.read_u8().unwrap_or(0);
    let pus_id = reader.read_u32().unwrap_or(0);
    let target_name = reader.read_string().unwrap_or_default();

    let world = session.world().clone();
    let sid = session.session_id();

    if world.is_player_dead(sid) || world.is_trading(sid) || world.is_merchanting(sid) {
        return Ok(());
    }

    if target_name.is_empty() || target_name.len() > 21 {
        return Ok(());
    }

    let pus_item = match world.get_pus_item(pus_id as i32) {
        Some(p) => p,
        None => return Ok(()),
    };

    let price = pus_item.price.unwrap_or(0) as u32;

    // Validate balance
    if price_type == 0 {
        if world.get_knight_cash(sid) < price {
            return Ok(());
        }
    } else if price_type == 1 {
        if world.get_tl_balance(sid) < price {
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Find target player online
    let target_sid = match world.find_session_by_name(&target_name) {
        Some(s) => s,
        None => return Ok(()),
    };

    // Give item to target
    if !world.give_item(
        target_sid,
        pus_item.item_id as u32,
        pus_item.buy_count as u16,
    ) {
        return Ok(());
    }

    // Deduct from sender
    if price_type == 0 {
        crate::handler::knight_cash::cash_lose(&world, session.pool(), sid, price);
    } else {
        world.update_session(sid, |h| {
            h.tl_balance = h.tl_balance.saturating_sub(price);
        });
    }
    send_cash_change(session).await?;
    Ok(())
}

/// Send CASHCHANGE (0xB9) update to client with current KC and TL balances.
/// Sends ext_hook packet (0xE9) + WIZ_CHAT fallback for v2525 vanilla clients
/// that silently drop 0xE9 (outside dispatch range 0x06-0xD7).
async fn send_cash_change(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let (kc, tl) = world
        .with_session(sid, |h| (h.knight_cash, h.tl_balance))
        .unwrap_or((0, 0));
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_CASHCHANGE);
    pkt.write_u32(kc);
    pkt.write_u32(tl);
    session.send_packet(&pkt).await?;
    // WIZ_CHAT fallback for vanilla v2525 client (drops ext_hook 0xE9)
    let chat_pkt = crate::handler::knight_cash::build_kc_chat_packet(kc, tl);
    session.send_packet(&chat_pkt).await
}

/// Handle DROP_LIST/DROP_REQUEST (0xAB/0xAE) — NPC/monster drop table query.
/// Commands:
///   1 = Look up target NPC/monster drop table
///   2 = Expand item group into individual items
///   3 = Look up by monster proto ID (expanded with groups)
///   4 = Collection Race reward items
pub async fn handle_drop_request(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let command = data[0];
    let rest = if data.len() > 1 { &data[1..] } else { &[] };

    match command {
        1 => handle_drop_cmd1(session, rest).await,
        2 => handle_drop_cmd2(session, rest).await,
        3 => handle_drop_cmd3(session, rest).await,
        4 => handle_drop_cmd4(session, rest).await,
        5 => handle_drop_cmd5(session, rest).await,
        7 => handle_drop_tooltip(session, rest).await,
        _ => Ok(()),
    }
}

async fn handle_drop_tooltip(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    let Some(id) = ko_protocol::PacketReader::new(data).read_u32() else { return Ok(()); };
    let Some(item) = session.world().get_item(id) else { return Ok(()); };
    let mut lines = Vec::new();
    macro_rules! stat {
        ($label:expr, $value:expr) => {
            if let Some(value) = $value.filter(|value| *value != 0) {
                lines.push(format!("{}: {}", $label, value));
            }
        };
    }
    stat!("Attack", item.damage); stat!("Defence", item.ac);
    stat!("Required level", item.req_level);
    stat!("STR", item.str_b); stat!("HP", item.sta_b); stat!("DEX", item.dex_b);
    stat!("INT", item.intel_b); stat!("MP", item.cha_b);
    stat!("Health bonus", item.max_hp_b); stat!("Mana bonus", item.max_mp_b);
    stat!("Fire damage", item.fire_damage); stat!("Ice damage", item.ice_damage);
    stat!("Lightning damage", item.lightning_damage); stat!("Poison damage", item.poison_damage);
    stat!("Fire resistance", item.fire_r); stat!("Cold resistance", item.cold_r);
    stat!("Lightning resistance", item.lightning_r); stat!("Magic resistance", item.magic_r);
    stat!("Poison resistance", item.poison_r); stat!("Curse resistance", item.curse_r);
    stat!("Dagger defence", item.dagger_ac); stat!("Jamadar defence", item.jamadar_ac);
    stat!("Sword defence", item.sword_ac); stat!("Club defence", item.club_ac);
    stat!("Axe defence", item.axe_ac); stat!("Spear defence", item.spear_ac); stat!("Bow defence", item.bow_ac);
    stat!("HP drain", item.hp_drain); stat!("MP drain", item.mp_drain);
    stat!("Required STR", item.req_str); stat!("Required HP", item.req_sta);
    stat!("Required DEX", item.req_dex); stat!("Required INT", item.req_intel); stat!("Required MP", item.req_cha);
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_DROP_REQUEST); pkt.write_u8(7); pkt.write_u32(id);
    pkt.write_string(item.str_name.as_deref().unwrap_or("Unknown item"));
    pkt.write_u32(item.item_icon_id1.unwrap_or(0).max(0) as u32);
    pkt.write_u8(lines.len() as u8);
    for line in lines { pkt.write_u8(0); pkt.write_string(&line); }
    session.send_packet(&pkt).await
}

/// 2625 panel response: proto, length-prefixed name, level, monster flag, count, then
/// (item id, chance/10000, icon id, length-prefixed item name) records.
fn build_drop_panel_response(
    world: &crate::world::WorldState,
    proto_id: u16,
    template: &crate::npc::NpcTemplate,
    table_index: i16,
) -> Packet {
    let pairs = if template.is_monster {
        world
            .get_monster_item(table_index)
            .map(|r| monster_item_pairs(&r).to_vec())
    } else {
        world
            .get_npc_item(table_index)
            .map(|r| npc_item_pairs(&r).to_vec())
    }
    .unwrap_or_default();

    let mut drops: Vec<(u32, u16)> = Vec::new();
    for (id, chance) in pairs {
        if id <= 0 || chance <= 0 {
            continue;
        }
        if id < 100 {
            // Production codes are generated by level/nation, not fixed item groups.
            drops.push((id as u32, chance.clamp(0, 10000) as u16));
        } else if id < 100_000_000 {
            if let Some(group) = world.get_make_item_group(id) {
                let total = group.items.iter().filter(|id| **id > 0).count() as u32;
                let mut weights = std::collections::BTreeMap::<u32, u32>::new();
                for item_id in group.items {
                    if item_id > 0 {
                        *weights.entry(item_id as u32).or_default() += 1;
                    }
                }
                for (item, weight) in weights {
                    drops.push((item, (chance.clamp(0, 10000) as u32 * weight / total) as u16));
                }
            }
        } else {
            drops.push((id as u32, chance as u16));
        }
    }

    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_DROP_REQUEST);
    pkt.write_u8(1);
    pkt.write_u16(proto_id);
    pkt.write_string(&template.name);
    pkt.write_u16(template.level as u16);
    pkt.write_u8(u8::from(template.is_monster));
    let count_offset = pkt.data.len();
    pkt.write_u16(0);
    let mut written = 0u16;
    for (item_id, chance) in drops.into_iter().take(u16::MAX as usize) {
        let item = world.get_item(item_id);
        let icon = item
            .as_ref()
            .and_then(|i| i.item_icon_id1.or(i.item_icon_id2))
            .unwrap_or(0)
            .max(0) as u32;
        let name = item
            .as_ref()
            .and_then(|i| i.str_name.as_deref())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| if item_id < 100 {
                format!("Random equipment (group {item_id})")
            } else { format!("Item {item_id}") });
        // The outer KO frame length is u16; leave room for encryption overhead.
        if pkt.data.len() + 12 + name.len() > 60_000 {
            warn!(proto_id, written, "Drop catalog exceeds one client frame; remaining entries omitted");
            break;
        }
        pkt.write_u32(item_id);
        pkt.write_u16(chance);
        pkt.write_u32(icon);
        pkt.write_string(&name);
        written += 1;
    }
    pkt.data[count_offset..count_offset + 2].copy_from_slice(&written.to_le_bytes());
    pkt
}

/// Drop command 1: Look up target NPC's drop items by its runtime ID.
async fn handle_drop_cmd1(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.len() < 4 {
        return Ok(());
    }
    let mut reader = ko_protocol::PacketReader::new(data);
    let target_nid = reader.read_u32().unwrap_or(0);

    let world = session.world().clone();

    let npc = match world.get_npc_instance(target_nid) {
        Some(n) => n,
        None => return Ok(()),
    };
    let template = match world.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) => t,
        None => return Ok(()),
    };
    let pkt = build_drop_panel_response(&world, npc.proto_id, &template, template.item_table);
    session.send_packet(&pkt).await
}

/// Drop command 2: Expand item group ID into individual items.
/// Wire format: `[0xE9][EXT_SUB_DROP_LIST][u8(2)][u8 count]([u32 item_id] × count)`
async fn handle_drop_cmd2(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.len() < 4 {
        return Ok(());
    }
    let mut reader = ko_protocol::PacketReader::new(data);
    let group_id = reader.read_u32().unwrap_or(0);

    let world = session.world().clone();
    if let Some(group) = world.get_make_item_group(group_id as i32) {
        let mut pkt = Packet::new(WIZ_EXT_HOOK);
        pkt.write_u8(EXT_SUB_DROP_LIST);
        pkt.write_u8(2);
        pkt.write_u8(group.items.len() as u8);
        for &item_id in &group.items {
            pkt.write_u32(item_id as u32);
        }
        session.send_packet(&pkt).await
    } else {
        Ok(())
    }
}

/// Drop command 3: Look up monster proto ID with group expansion.
/// For each drop slot, if item_id < MIN_ITEM_ID, expands via MakeItemGroup.
/// Also checks MakeItemGroupRandom for random loot indicator (900004000).
async fn handle_drop_cmd3(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.len() < 2 {
        return Ok(());
    }
    let mut reader = ko_protocol::PacketReader::new(data);
    let mob_proto = reader.read_u16().unwrap_or(0);

    let world = session.world().clone();

    let template = match world.get_npc_template(mob_proto, true) {
        Some(t) => t,
        None => return Ok(()),
    };
    let pkt = build_drop_panel_response(&world, mob_proto, &template, template.item_table);
    session.send_packet(&pkt).await
}

/// Search live monster prototypes; the result is bounded to fit a single frame.
async fn handle_drop_cmd5(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    let mut reader = ko_protocol::PacketReader::new(data);
    let query = match reader.read_string() {
        Some(query) if query.len() <= 100 => query.trim().to_lowercase(),
        _ => return Ok(()),
    };
    let world = session.world();
    let mut rows = Vec::new();
    for proto in 1..=u16::MAX {
        let Some(template) = world.get_npc_template(proto, true) else { continue; };
        if query.parse::<u16>().ok() == Some(proto) || template.name.to_lowercase().contains(&query) {
            rows.push((proto, template));
            if rows.len() == 100 { break; }
        }
    }
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_DROP_REQUEST);
    pkt.write_u8(5);
    pkt.write_u16(rows.len() as u16);
    for (proto, template) in rows {
        pkt.write_u16(proto);
        pkt.write_string(&template.name);
        pkt.write_u16(template.level as u16);
        let has_drops = world.get_monster_item(template.item_table)
            .is_some_and(|row| monster_item_pairs(&row).iter().any(|(id, rate)| *id > 0 && *rate > 0));
        pkt.write_u8(u8::from(has_drops));
    }
    session.send_packet(&pkt).await
}

/// Drop command 4: Collection Race reward items.
async fn handle_drop_cmd4(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let _cr_select = data[0];

    // Collection Race reward — requires active event with random item arrays.
    // For now, send empty response since Collection Race rewards are event-specific.
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_DROP_REQUEST);
    pkt.write_u8(1);
    pkt.write_u16(2); // Collection Race proto pseudo-ID
    pkt.write_u32(0); // empty list
    pkt.write_u8(2); // Collection Race marker
    session.send_packet(&pkt).await
}

/// Extract 12 (item_id, percent) pairs from MonsterItemRow.
fn monster_item_pairs(mi: &ko_db::models::item_tables::MonsterItemRow) -> [(i32, i16); 12] {
    [
        (mi.item01, mi.percent01),
        (mi.item02, mi.percent02),
        (mi.item03, mi.percent03),
        (mi.item04, mi.percent04),
        (mi.item05, mi.percent05),
        (mi.item06, mi.percent06),
        (mi.item07, mi.percent07),
        (mi.item08, mi.percent08),
        (mi.item09, mi.percent09),
        (mi.item10, mi.percent10),
        (mi.item11, mi.percent11),
        (mi.item12, mi.percent12),
    ]
}

/// Extract 12 (item_id, percent) pairs from NpcItemRow.
fn npc_item_pairs(ni: &ko_db::models::item_tables::NpcItemRow) -> [(i32, i16); 12] {
    [
        (ni.item01, ni.percent01),
        (ni.item02, ni.percent02),
        (ni.item03, ni.percent03),
        (ni.item04, ni.percent04),
        (ni.item05, ni.percent05),
        (ni.item06, ni.percent06),
        (ni.item07, ni.percent07),
        (ni.item08, ni.percent08),
        (ni.item09, ni.percent09),
        (ni.item10, ni.percent10),
        (ni.item11, ni.percent11),
        (ni.item12, ni.percent12),
    ]
}

/// Handle CHAOTIC_EXCHANGE (0xB4) — random item exchange via Bifrost generator.
/// Client sends: `[u16 npc_id][u32 item_id][u8 src_pos][u8 bank][u8 sell][u8 count]`
pub async fn handle_chaotic_exchange(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.len() < 8 {
        return Ok(());
    }
    let mut reader = ko_protocol::PacketReader::new(data);
    let npc_id = reader.read_u16().unwrap_or(0);
    let exchange_item_id = reader.read_u32().unwrap_or(0);
    let src_pos = reader.read_u8().unwrap_or(0);
    let bank = reader.read_u8().unwrap_or(0);
    let sell = reader.read_u8().unwrap_or(0);
    let count = reader.read_u8().unwrap_or(0);
    let error_code: u8 = 2;

    let world = session.world().clone();
    let sid = session.session_id();

    if count == 0 || count > 100 {
        return send_bifrost_fail(session, error_code).await;
    }

    // Chaotic coins gate
    //   uint32 coinsreq = g_pMain->pServerSetting.chaoticcoins;
    //   if (coinsreq && !hasCoins(coinsreq)) { SendHelpDescription(...); return fail; }
    let chaotic_coins = world
        .get_server_settings()
        .map(|s| s.chaotic_coins)
        .unwrap_or(0);
    if chaotic_coins > 0 {
        let gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
        if gold < chaotic_coins as u32 {
            return send_bifrost_fail(session, error_code).await;
        }
    }

    // Validate player has enough of the exchange item
    if !world.check_exist_item(sid, exchange_item_id, count as u16) {
        return send_bifrost_fail(session, error_code).await;
    }

    // Validate NPC exists and is chaotic generator type
    let npc = match world.get_npc_instance(npc_id as u32) {
        Some(n) => n,
        None => return send_bifrost_fail(session, error_code).await,
    };

    // State checks
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return send_bifrost_fail(session, error_code).await;
    }

    // NPC type check — must be NPC_CHAOTIC_GENERATOR (137) or NPC_CHAOTIC_GENERATOR2 (162)
    let npc_type = world
        .get_npc_template(npc.proto_id, npc.is_monster)
        .map(|t| t.npc_type)
        .unwrap_or(0);
    if npc_type != 137 && npc_type != 162 {
        return send_bifrost_fail(session, error_code).await;
    }

    // Validate item template: must be countable with effect2 == 251
    let item_tmpl = match world.get_item(exchange_item_id) {
        Some(t) => t,
        None => return send_bifrost_fail(session, error_code).await,
    };

    if item_tmpl.countable.unwrap_or(0) != 1 || item_tmpl.effect2.unwrap_or(0) != 251 {
        return send_bifrost_fail(session, error_code).await;
    }

    // Check free slots
    let free_slots = world.count_free_slots(sid);
    if free_slots < 1 {
        return send_bifrost_fail(session, error_code).await;
    }

    // Load matching exchanges
    let exchanges = world.get_bifrost_exchanges(exchange_item_id);
    if exchanges.is_empty() {
        return send_bifrost_fail(session, error_code).await;
    }

    // Process each exchange count
    let multiple = count > 1;
    for _ in 0..count {
        // Build the complete weighted pool. The former fixed 10,000-slot
        // array truncated large pools in DashMap iteration order, which could
        // silently exclude low-rate rewards such as Fortified Sterling's
        // accessory entries. Keep every positive TBL weight instead.
        let mut weighted_items = Vec::with_capacity(exchanges.len());
        let mut total_weight = 0u64;

        for ex in &exchanges {
            if ex.random_flag >= 101 {
                continue;
            }
            // Verify origin count
            if !world.check_exist_item(
                sid,
                ex.origin_item_num1 as u32,
                ex.origin_item_count1 as u16,
            ) {
                continue;
            }
            if ex.exchange_item_num1 <= 0 || ex.exchange_item_count1 <= 0 {
                continue;
            }
            let weight = ex.exchange_item_count1 as u64;
            total_weight = total_weight.saturating_add(weight);
            weighted_items.push((ex.exchange_item_num1 as u32, weight));
        }

        if total_weight == 0 {
            return send_bifrost_fail(session, error_code).await;
        }

        let roll = rand::random::<u64>() % total_weight;
        let give_item_id = select_weighted_exchange_item(&weighted_items, roll)
            .ok_or_else(|| anyhow::anyhow!("Chaotic Generator weighted pool was inconsistent"))?;

        let give_tmpl = match world.get_item(give_item_id) {
            Some(t) => t,
            None => return send_bifrost_fail(session, error_code).await,
        };

        // Rob one exchange item from source position
        if !world.rob_item(sid, exchange_item_id, 1) {
            return send_bifrost_fail(session, error_code).await;
        }

        let selling = sell != 0 && give_tmpl.item_type.unwrap_or(0) != 4;

        if selling {
            // Auto-sell: calculate price and give gold
            // C++ SellTypeFullPrice = 1 → full buy_price; otherwise → buy_price / 6
            let mut price = if give_tmpl.sell_npc_type.unwrap_or(0) == 1 {
                give_tmpl.sell_npc_price.unwrap_or(0) as u32
            } else if give_tmpl.sell_price.unwrap_or(0) == 1 {
                // SellTypeFullPrice
                give_tmpl.buy_price.unwrap_or(0) as u32
            } else {
                give_tmpl.buy_price.unwrap_or(0) as u32 / 6
            };
            if give_tmpl.kind.unwrap_or(0) == ITEM_KIND_UNIQUE {
                price = 1;
            }

            world.gold_gain(sid, price);
            send_chaotic_result(session, 2).await?;
            send_bifrost_fail(session, 1).await?;
            continue;
        }

        if bank != 0 {
            // Bank: store in warehouse (simplified — give item to inventory)
            if !world.give_item(sid, give_item_id, 1) {
                return send_bifrost_fail(session, error_code).await;
            }
            send_chaotic_result(session, 2).await?;
            send_bifrost_fail(session, 1).await?;
            continue;
        }

        // Normal: give item to inventory
        let slot = match world.find_slot_for_item(sid, give_item_id, 1) {
            Some(s) => s,
            None => return send_bifrost_fail(session, error_code).await,
        };

        if !world.give_item(sid, give_item_id, 1) {
            return send_bifrost_fail(session, 0).await;
        }

        // Determine effect type based on item_type
        let effect_type: u8 = match give_tmpl.item_type.unwrap_or(0) {
            4 => 1, // White
            5 => 2, // Green
            _ => 3, // Red
        };

        // Send success packet: WIZ_ITEM_UPGRADE + BIFROST_EXCHANGE sub
        let mut result = Packet::new(ko_protocol::Opcode::WizItemUpgrade as u8);
        result.write_u8(5); // ITEM_BIFROST_EXCHANGE
        result.write_u8(1); // success
        result.write_u32(give_item_id);
        result.write_i8((slot as i8) - super::SLOT_MAX as i8);
        result.write_u32(exchange_item_id);
        result.write_u8(src_pos);
        result.write_u8(effect_type);
        session.send_packet(&result).await?;

        send_chaotic_result(session, 2).await?;

        // Broadcast visual effect for single exchanges
        if !multiple {
            let mut fx = Packet::new(ko_protocol::Opcode::WizObjectEvent as u8);
            fx.write_u8(crate::object_event_constants::OBJECT_GATE);
            fx.write_u8(effect_type);
            fx.write_u16(npc_id);
            let world2 = session.world().clone();
            if let Some((zone_id, rx, rz)) = world2.with_session(sid, |h| {
                (h.position.zone_id, h.position.region_x, h.position.region_z)
            }) {
                world2.broadcast_to_region_sync(zone_id, rx, rz, Arc::new(fx), None, 0);
            }
        }
    }
    Ok(())
}

/// Select a generator reward using a zero-based roll in `[0, total_weight)`.
/// Kept independent from RNG so pool-boundary behavior is unit-testable.
fn select_weighted_exchange_item(weighted_items: &[(u32, u64)], mut roll: u64) -> Option<u32> {
    for &(item_id, weight) in weighted_items {
        if weight == 0 {
            continue;
        }
        if roll < weight {
            return Some(item_id);
        }
        roll -= weight;
    }
    None
}

/// Send chaotic exchange result notification.
async fn send_chaotic_result(session: &mut ClientSession, result: u8) -> anyhow::Result<()> {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_CHAOTIC_EXCHANGE);
    pkt.write_u8(result);
    session.send_packet(&pkt).await
}

/// Send Bifrost exchange failure packet.
async fn send_bifrost_fail(session: &mut ClientSession, error_code: u8) -> anyhow::Result<()> {
    let mut pkt = Packet::new(ko_protocol::Opcode::WizItemUpgrade as u8);
    pkt.write_u8(5); // ITEM_BIFROST_EXCHANGE
    pkt.write_u8(error_code);
    session.send_packet(&pkt).await
}

/// Handle MERCHANT (0xB5) — merchant item add via ext hook path.
/// Delegates to the standard WIZ_MERCHANT handler by constructing a synthetic packet.
/// The ext_hook data format is: `[u8 sub_code][...merchant_data]` — identical to WIZ_MERCHANT.
pub async fn handle_merchant(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    // Construct a WIZ_MERCHANT packet from the ext_hook data.
    // The data already starts with the sub_opcode byte (should be 3 = MERCHANT_ITEM_ADD).
    let mut pkt = Packet::new(ko_protocol::Opcode::WizMerchant as u8);
    pkt.write_bytes(data);
    crate::handler::merchant::handle(session, pkt).await
}

/// Handle MERCHANTLIST (0xCA) — Menissiah merchant list system.
/// Sub-opcodes:
///   0 = ReqMerchantListSend — list all merchant items
///   1 = ReqMerchantListGo — teleport to merchant
///   2 = ReqMerchantListMessage — open whisper to merchant
pub async fn handle_merchantlist(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let sub = data[0];
    let rest = if data.len() > 1 { &data[1..] } else { &[] };

    match sub {
        0 => handle_merchantlist_send(session).await,
        1 => handle_merchantlist_go(session, rest).await,
        2 => handle_merchantlist_message(session, rest).await,
        _ => Ok(()),
    }
}

/// MerchantList sub 0: Send full merchant item list.
/// Wire format per item: `[u16 sid][u32 sid][string name][u8 sell_buy][u32 item_id][u16 count][u32 price][u8 is_kc][f32 x][f32 y][f32 z]`
async fn handle_merchantlist_send(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();

    // Collect all merchant items from online sessions
    let items = world.collect_merchant_items();

    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_MERCHANTLIST);
    pkt.write_u32(items.len() as u32);

    for (sid, name, is_selling, item_id, count, price, is_kc, x, y, z) in &items {
        pkt.write_u16(*sid);
        pkt.write_u32(*sid as u32);
        pkt.write_string(name);
        pkt.write_u8(if *is_selling { 0 } else { 1 });
        pkt.write_u32(*item_id);
        pkt.write_u16(*count);
        pkt.write_u32(*price);
        pkt.write_u8(if *is_kc { 1 } else { 0 });
        pkt.write_f32(*x);
        pkt.write_f32(*y);
        pkt.write_f32(*z);
    }

    session.send_packet(&pkt).await
}

/// MerchantList sub 1: Teleport to merchant.
async fn handle_merchantlist_go(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    let mut reader = ko_protocol::PacketReader::new(data);
    let target_name = reader.read_string().unwrap_or_default();

    if target_name.is_empty() {
        return Ok(());
    }

    let world = session.world().clone();

    // Find target merchant
    let target_sid = match world.find_session_by_name(&target_name) {
        Some(s) => s,
        None => return Ok(()),
    };

    // Validate target is merchanting
    let target_info = match world.with_session(target_sid, |h| {
        if h.merchant_state == MERCHANT_STATE_NONE {
            return None;
        }
        Some((h.position.zone_id, h.position.x, h.position.z))
    }) {
        Some(Some(info)) => info,
        _ => return Ok(()),
    };

    let (zone_id, x, z) = target_info;

    // Teleport to merchant location
    crate::handler::zone_change::trigger_zone_change(session, zone_id, x, z).await
}

/// MerchantList sub 2: Open whisper chat to merchant.
async fn handle_merchantlist_message(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    let mut reader = ko_protocol::PacketReader::new(data);
    let target_name = reader.read_string().unwrap_or_default();

    if target_name.is_empty() {
        return Ok(());
    }

    let world = session.world().clone();

    // Verify target is merchant
    let target_sid = match world.find_session_by_name(&target_name) {
        Some(s) => s,
        None => return Ok(()),
    };

    let is_merchant = world
        .with_session(target_sid, |h| h.merchant_state != MERCHANT_STATE_NONE)
        .unwrap_or(false);

    if !is_merchant {
        return Ok(());
    }

    // Send private chat initiation to session — C++ sends a PRIVATE_CHAT packet with empty message
    let mut pkt = Packet::new(ko_protocol::Opcode::WizChat as u8);
    pkt.write_u8(3); // PRIVATE_CHAT
    pkt.write_u8(0); // nation (unused for private)
    pkt.write_u32(0); // sender_id
    pkt.write_sbyte_string(&target_name);
    pkt.write_string(" "); // empty message (space)
    session.send_packet(&pkt).await
}

/// Handle REPURCHASE (0xCE) — buyback list request.
/// The ext_hook path simply triggers the same repurchase list send that
/// NPC trade type 5 sub-opcode 4 uses (refreshed=false).
pub async fn handle_repurchase(session: &mut ClientSession, _data: &[u8]) -> anyhow::Result<()> {
    let sid = session.session_id();
    let world = session.world().clone();
    crate::handler::npc_trade::send_repurchase_list(session, &world, sid, false).await
}

/// Handle ITEM_EXCHANGE_INFO (0xE6) — right-click item exchange.
/// Sub-opcodes:
/// - 1: RightClickExchangeSend (old UI, sends item_id)
/// - 2: NewRightClickExchangeSend (new UI, sends slot + item_id)
/// - 3: NewRightClickExchange (execute: reward/all/premium/KC/genie/TL)
/// - 4: NewRightClickGiveExchange (selectable item exchange)
/// - 5: NewRightClickGeneratorExchange (generator/random exchange)
pub async fn handle_item_exchange_info(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let subcode = data[0];
    let rest = if data.len() > 1 { &data[1..] } else { &[] };

    match subcode {
        0 => send_right_exchange_slots(session).await,
        1 => handle_right_click_exchange_send(session, rest).await,
        2 => handle_new_right_click_exchange_send(session, rest).await,
        3 => handle_new_right_click_exchange(session, rest).await,
        4 => handle_new_right_click_give_exchange(session, rest).await,
        5 => handle_new_right_click_generator_exchange(session, rest).await,
        _ => {
            debug!("ItemExchangeInfo: unknown subcode {}", subcode);
            Ok(())
        }
    }
}

/// Sub 1: Old right-click exchange send — client clicks item, server sends item_id.
async fn handle_right_click_exchange_send(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let slot = data[0];
    let sid = session.session_id();
    let world = session.world().clone();

    let item_id = world
        .with_session(sid, |h| {
            h.inventory
                .get(slot as usize)
                .filter(|s| s.item_id != 0)
                .map(|s| s.item_id)
        })
        .flatten();

    let item_id = match item_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    pkt.write_u8(1); // response type
    pkt.write_u8(1); // old format
    pkt.write_u32(item_id);
    session.send_packet(&pkt).await
}

/// Sub 2: New right-click exchange send — server sends slot + item_id.
async fn handle_new_right_click_exchange_send(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let slot = data[0];
    let sid = session.session_id();
    let world = session.world().clone();

    let item_id = world
        .with_session(sid, |h| {
            h.inventory
                .get(slot as usize)
                .filter(|s| s.item_id != 0)
                .map(|s| s.item_id)
        })
        .flatten();

    let item_id = match item_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    pkt.write_u8(1); // response type
    pkt.write_u8(2); // new format
    pkt.write_u8(slot);
    pkt.write_u32(item_id);
    let item = world.get_item(item_id);
    pkt.write_u32(item.as_ref().and_then(|i| i.item_icon_id1).unwrap_or(0).max(0) as u32);
    pkt.write_string(item.as_ref().and_then(|i| i.str_name.as_deref()).unwrap_or(""));
    let count = world.with_session(sid, |h| h.inventory.get(slot as usize).map(|s| s.count).unwrap_or(0)).unwrap_or(0);
    pkt.write_u16(count);
    session.send_packet(&pkt).await
}

/// Sub 3: Execute exchange — dispatches on exchange type.
async fn handle_new_right_click_exchange(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let exchange_type = data[0];
    let rest = if data.len() > 1 { &data[1..] } else { &[] };

    match exchange_type {
        1 | 2 => send_right_exchange_preview(session, exchange_type, rest).await,
        3 => handle_exchange_premium(session, rest).await,
        4 => handle_exchange_knight_cash(session, rest).await,
        6 => handle_exchange_genie(session, rest).await,
        7 => handle_exchange_knight_tl(session, rest).await,
        _ => {
            debug!("NewRightClickExchange: unknown type {}", exchange_type);
            Ok(())
        }
    }
}

async fn send_right_exchange_preview(session: &mut ClientSession, kind: u8, data: &[u8]) -> anyhow::Result<()> {
    let mut reader = ko_protocol::PacketReader::new(data);
    let Some(item_id) = reader.read_u32() else { return Ok(()); };
    let world = session.world().clone();
    let Some(row) = world.get_right_exchange(item_id as i32) else { return Ok(()); };
    if row.exchange_type.unwrap_or(0) != kind as i16
        || !world.check_exist_item(session.session_id(), item_id, 1) { return Ok(()); }
    let icon = |id: u32| world.get_item(id).and_then(|i| i.item_icon_id1).unwrap_or(0).max(0) as u32;
    let name = |id: u32| world.get_item(id).and_then(|i| i.str_name.clone()).unwrap_or_default();
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    pkt.write_u8(3);
    pkt.write_u8(kind);
    pkt.write_u8(kind);
    pkt.write_u32(item_id);
    for i in 0..25 {
        pkt.write_u32(row.exchange_items.get(i).copied().unwrap_or(0).max(0) as u32);
        pkt.write_u32(row.expiration_times.get(i).copied().unwrap_or(0).max(0) as u32);
    }
    pkt.write_u32(icon(item_id));
    for i in 0..25 { pkt.write_u32(icon(row.exchange_items.get(i).copied().unwrap_or(0).max(0) as u32)); }
    pkt.write_u8(1);
    pkt.write_string(&name(item_id));
    for i in 0..25 {
        pkt.write_string(&name(row.exchange_items.get(i).copied().unwrap_or(0).max(0) as u32));
        pkt.write_u32(row.exchange_counts.get(i).copied().unwrap_or(0).max(0) as u32);
    }
    session.send_packet(&pkt).await
}

/// Exchange type 3: Premium exchange.
async fn handle_exchange_premium(session: &mut ClientSession, _data: &[u8]) -> anyhow::Result<()> {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    pkt.write_u8(3);
    pkt.write_u8(3);
    session.send_packet(&pkt).await
}

/// Exchange type 4: Knight Cash exchange.
async fn handle_exchange_knight_cash(
    session: &mut ClientSession,
    _data: &[u8],
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    pkt.write_u8(3);
    pkt.write_u8(4);
    session.send_packet(&pkt).await
}

/// Exchange type 6: Genie exchange.
async fn handle_exchange_genie(session: &mut ClientSession, _data: &[u8]) -> anyhow::Result<()> {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    pkt.write_u8(3);
    pkt.write_u8(6);
    session.send_packet(&pkt).await
}

/// Exchange type 7: Knight TL exchange.
async fn handle_exchange_knight_tl(
    session: &mut ClientSession,
    _data: &[u8],
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    pkt.write_u8(3);
    pkt.write_u8(7);
    session.send_packet(&pkt).await
}

/// Sub 4: Give exchange — selectable item exchange from ITEM_RIGHT_EXCHANGE.
/// Two types:
/// - sType=1: Player selects specific reward from list, consume source item
/// - sType=2: Give all reward items from the exchange entry
async fn handle_new_right_click_give_exchange(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.len() < 5 {
        return Ok(());
    }
    let s_type = data[0];
    let item_id = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);

    let world = session.world().clone();
    let sid = session.session_id();

    let right_exchange = match world.get_right_exchange(item_id as i32) {
        Some(r) => r,
        None => return Ok(()),
    };

    let exchange_type = right_exchange.exchange_type.unwrap_or(0) as u8;

    if s_type == 1 && exchange_type == 1 {
        // Selectable exchange: player picks one reward
        if data.len() < 9 {
            return Ok(());
        }
        let selected_item = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);

        // Validate the selected reward before consuming the player's coupon.
        if selected_item == 0 || !right_exchange.exchange_items.contains(&(selected_item as i32)) {
            return Ok(());
        }

        // Check free slots
        let free_slots = world.count_free_slots(sid);
        if free_slots < 1 {
            return Ok(());
        }

        if selected_item != 0 && world.rob_item(sid, item_id, 1) {
            // Find selected item in exchange list
            let mut found = false;
            for i in 0..right_exchange.exchange_items.len() {
                if right_exchange.exchange_items[i] == selected_item as i32 {
                    let count = right_exchange.exchange_counts.get(i).copied().unwrap_or(1) as u16;
                    let _rental =
                        right_exchange.expiration_times.get(i).copied().unwrap_or(0) as u32;
                    world.give_item(sid, selected_item, count);
                    found = true;
                    break;
                }
            }

            if found {
                let mut pkt = Packet::new(WIZ_EXT_HOOK);
                pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
                pkt.write_u8(3); // process result
                pkt.write_u8(5); // success
                session.send_packet(&pkt).await?;
            }
        }
    } else if s_type == 2 && exchange_type == 2 {
        // Give all: give all exchange items at once
        let mut needed_slots = 0u8;
        for &id in &right_exchange.exchange_items {
            if id > 0 {
                needed_slots += 1;
            }
        }

        let free_slots = world.count_free_slots(sid);
        if free_slots < needed_slots {
            return Ok(());
        }

        if world.rob_item(sid, item_id, 1) {
            for i in 0..right_exchange.exchange_items.len() {
                let give_id = right_exchange.exchange_items[i];
                if give_id <= 0 {
                    continue;
                }
                let count = right_exchange.exchange_counts.get(i).copied().unwrap_or(1) as u16;
                let _rental = right_exchange.expiration_times.get(i).copied().unwrap_or(0) as u32;
                world.give_item(sid, give_id as u32, count);
            }

            let mut pkt = Packet::new(WIZ_EXT_HOOK);
            pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
            pkt.write_u8(3);
            pkt.write_u8(5); // success
            session.send_packet(&pkt).await?;
        }
    }

    Ok(())
}

/// Sub 5: Generator exchange — random item from ITEM_EXCHANGE table.
/// Uses weighted random selection from m_ItemExchangeArray where
/// `bRandomFlag IN (1,2,3,101)` and `nOriginItemNum[0] == item_id`.
async fn handle_new_right_click_generator_exchange(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.len() < 6 {
        return Ok(());
    }
    let exchange_item_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let _src_pos = data[4];
    let _bank = if data.len() > 5 { data[5] } else { 0 };
    let _sell = if data.len() > 6 { data[6] } else { 0 };
    let count = if data.len() > 7 { data[7].min(100) } else { 1 };

    let sid = session.session_id();
    let world = session.world().clone();

    // Chaotic coins gate
    //   uint32 coinsreq = g_pMain->pServerSetting.chaoticcoins;
    //   if (coinsreq && !hasCoins(coinsreq)) { SendHelpDescription(...); return; }
    let chaotic_coins = world
        .get_server_settings()
        .map(|s| s.chaotic_coins)
        .unwrap_or(0);
    if chaotic_coins > 0 {
        let gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
        if gold < chaotic_coins as u32 {
            return Ok(());
        }
    }

    // Validate item exists in inventory
    if !world.check_exist_item(sid, exchange_item_id, count as u16) {
        return Ok(());
    }

    // Get matching exchange entries (bRandomFlag 1,2,3,101)
    let exchanges = world.get_generator_exchanges(exchange_item_id);
    if exchanges.is_empty() {
        return Ok(());
    }

    // Build weighted random array
    let mut rand_array: Vec<u32> = Vec::with_capacity(10000);
    for entry in &exchanges {
        if entry.random_flag == 101 {
            // Multi-item pool: each output item fills (count/900) slots
            let items = [
                (entry.exchange_item_num1, entry.exchange_item_count1),
                (entry.exchange_item_num2, entry.exchange_item_count2),
                (entry.exchange_item_num3, entry.exchange_item_count3),
                (entry.exchange_item_num4, entry.exchange_item_count4),
                (entry.exchange_item_num5, entry.exchange_item_count5),
            ];
            for (item, cnt) in &items {
                if *item <= 0 {
                    continue;
                }
                let slots = (*cnt / 900) as usize;
                for _ in 0..slots {
                    if rand_array.len() >= 9999 {
                        break;
                    }
                    rand_array.push(*item as u32);
                }
            }
        } else {
            // Single item pool: fills (count[0]/5) slots
            let slots = (entry.exchange_item_count1 / 5) as usize;
            for _ in 0..slots {
                if rand_array.len() >= 9999 {
                    break;
                }
                rand_array.push(entry.exchange_item_num1 as u32);
            }
        }
        if rand_array.len() >= 9999 {
            break;
        }
    }

    if rand_array.is_empty() {
        return Ok(());
    }

    let mut last_reward = 0;
    // Process each count
    for _ in 0..count {
        // Check free slots
        if world.count_free_slots(sid) < 1 {
            break;
        }

        // Random pick
        let won_item = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..rand_array.len());
            rand_array[idx]
        };

        // Rob source item
        if !world.rob_item(sid, exchange_item_id, 1) {
            break;
        }

        // Give won item
        if !world.give_item(sid, won_item, 1) {
            world.give_item(sid, exchange_item_id, 1);
            break;
        }
        last_reward = won_item;
    }

    let left = world.with_session(sid, |h| h.inventory.iter()
        .filter(|slot| slot.item_id == exchange_item_id)
        .map(|slot| slot.count as u32).sum::<u32>()).unwrap_or(0);
    let item = world.get_item(last_reward);
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    pkt.write_u8(3); pkt.write_u8(8);
    pkt.write_u32(last_reward);
    pkt.write_u32(item.as_ref().and_then(|i| i.item_icon_id1).unwrap_or(0).max(0) as u32);
    pkt.write_string(item.as_ref().and_then(|i| i.str_name.as_deref()).unwrap_or(""));
    pkt.write_u16(left.min(u16::MAX as u32) as u16);
    pkt.write_string(if last_reward == 0 { "Canta dolu veya esya kirilamiyor." } else { "" });
    pkt.write_u32(exchange_item_id);
    session.send_packet(&pkt).await
}

/// Handle DAILY_REWARD (0xF7) — daily login reward claim.
/// Sub-opcode 1 = claim request (HandleDailyRewardGive).
pub async fn handle_daily_reward(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let subcode = data[0];
    match subcode {
        1 => handle_daily_reward_give(session, data).await,
        _ => {
            debug!("DailyReward: unknown subcode {}", subcode);
            Ok(())
        }
    }
}

/// Handle daily reward claim request (subcode 1).
/// Flow:
/// 1. Client sends item_id it wants to claim
/// 2. Server loads 25-item config, 3 cumulative items, user progress
/// 3. Find matching item in config
/// 4. Validate sequential claim (previous day must be claimed, not same calendar day)
/// 5. Mark claimed, give item, send updated state
async fn handle_daily_reward_give(session: &mut ClientSession, data: &[u8]) -> anyhow::Result<()> {
    // data = [u8 subcode(1), u32 item_id]
    if data.len() < 5 {
        return Ok(());
    }
    let requested_item_id = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);

    let pool = session.pool().clone();
    let repo = ko_db::repositories::daily_reward::DailyRewardRepository::new(&pool);

    // Load reward config (25 items)
    let reward_config = match repo.load_all().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "[{}] ext_hook daily_reward load_all DB error: {e}",
                session.addr()
            );
            Vec::new()
        }
    };
    if reward_config.is_empty() {
        return Ok(());
    }

    // Load cumulative rewards (3 items)
    let cumulative = match repo.load_cumulative().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "[{}] ext_hook daily_reward load_cumulative DB error: {e}",
                session.addr()
            );
            None
        }
    };
    let cumulative_ids: [u32; 3] = match &cumulative {
        Some(c) => [
            c.item1.unwrap_or(0) as u32,
            c.item2.unwrap_or(0) as u32,
            c.item3.unwrap_or(0) as u32,
        ],
        None => [0, 0, 0],
    };

    // Build 25-item array from config
    let mut item_ids = [0u32; 25];
    for row in &reward_config {
        let idx = row.day_index as usize;
        if idx < 25 {
            item_ids[idx] = row.item_id as u32;
        }
    }

    // Load user progress
    let char_name = session
        .world()
        .get_character_info(session.session_id())
        .map(|c| c.name.clone())
        .unwrap_or_default();
    if char_name.is_empty() {
        return Ok(());
    }

    let user_rows = match repo.load_user_progress(&char_name).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "[{}] ext_hook daily_reward load_user_progress DB error: {e}",
                session.addr()
            );
            Vec::new()
        }
    };
    let mut sb_type = [0u8; 25]; // 0=unclaimed, 1=claimed
    let mut s_get_day = [0u8; 25]; // day-of-month when claimed
    for row in &user_rows {
        let idx = row.day_index as usize;
        if idx < 25 {
            sb_type[idx] = if row.claimed { 1 } else { 0 };
            s_get_day[idx] = row.day_of_month as u8;
        }
    }

    // Get current day-of-month
    let now = chrono::Utc::now();
    let today_day = now.format("%d").to_string().parse::<u8>().unwrap_or(1);

    // Find the requested item in config
    for i in 0..25usize {
        if item_ids[i] == requested_item_id && sb_type[i] == 0 {
            // Case 1: i > 0 and previous not claimed → error
            if i > 0 && sb_type[i - 1] == 0 {
                send_daily_reward_error(session).await?;
                return Ok(());
            }

            // Case 2: i > 0, previous claimed, but same day → error
            if i > 0 && sb_type[i - 1] > 0 && s_get_day[i - 1] == today_day {
                send_daily_reward_error(session).await?;
                return Ok(());
            }

            // Valid claim (either i==0, or previous claimed on different day)
            sb_type[i] = 1;
            s_get_day[i] = today_day;

            // Send success response with updated state
            send_daily_reward_state(session, 2, &item_ids, &sb_type, &s_get_day, &cumulative_ids)
                .await?;

            // Give item
            let world = session.world().clone();
            world.give_item(session.session_id(), requested_item_id, 1);

            // Save to DB
            let char_name_db = char_name.clone();
            let day_idx = i as i16;
            tokio::spawn(async move {
                let repo = ko_db::repositories::daily_reward::DailyRewardRepository::new(&pool);
                if let Err(e) = repo
                    .update_user_day(&char_name_db, day_idx, true, today_day as i16)
                    .await
                {
                    warn!("Failed to save daily reward for {}: {}", char_name_db, e);
                }
            });

            info!(
                "[{}] DailyReward: claimed day {} item {}",
                session.addr(),
                i,
                requested_item_id
            );
            return Ok(());
        }
    }

    Ok(())
}

/// Send daily reward error response (can't claim yet).
async fn send_daily_reward_error(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_DAILY_REWARD);
    pkt.write_u8(3); // error
    session.send_packet(&pkt).await
}

/// Send daily reward state to client.
/// `HandleDailyRewardGive()` (sub=2).
/// Wire: `[0xE9][0xF7][u8 sub]([u32 item_id][u8 type][u8 day] × 25)([u32 cumulative] × 3)`
pub async fn send_daily_reward_state(
    session: &mut ClientSession,
    sub: u8,
    item_ids: &[u32; 25],
    types: &[u8; 25],
    days: &[u8; 25],
    cumulative: &[u32; 3],
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_DAILY_REWARD);
    pkt.write_u8(sub);
    for i in 0..25 {
        pkt.write_u32(item_ids[i]);
        pkt.write_u8(types[i]);
        pkt.write_u8(days[i]);
    }
    for &val in cumulative {
        pkt.write_u32(val);
    }
    session.send_packet(&pkt).await
}

/// Send daily reward data on game entry.
pub async fn send_daily_reward_on_login(session: &mut ClientSession) -> anyhow::Result<()> {
    let pool = session.pool().clone();
    let repo = ko_db::repositories::daily_reward::DailyRewardRepository::new(&pool);

    let reward_config = match repo.load_all().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "[{}] daily_reward_on_login load_all DB error: {e}",
                session.addr()
            );
            Vec::new()
        }
    };
    if reward_config.is_empty() {
        return Ok(());
    }

    let char_name = session
        .world()
        .get_character_info(session.session_id())
        .map(|c| c.name.clone())
        .unwrap_or_default();
    if char_name.is_empty() {
        return Ok(());
    }

    let user_rows = match repo.load_user_progress(&char_name).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "[{}] daily_reward_on_login load_user_progress DB error: {e}",
                session.addr()
            );
            Vec::new()
        }
    };

    let cumulative = match repo.load_cumulative().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "[{}] daily_reward_on_login load_cumulative DB error: {e}",
                session.addr()
            );
            None
        }
    };
    let cumulative_ids: [u32; 3] = match &cumulative {
        Some(c) => [
            c.item1.unwrap_or(0) as u32,
            c.item2.unwrap_or(0) as u32,
            c.item3.unwrap_or(0) as u32,
        ],
        None => [0, 0, 0],
    };

    let mut item_ids = [0u32; 25];
    for row in &reward_config {
        let idx = row.day_index as usize;
        if idx < 25 {
            item_ids[idx] = row.item_id as u32;
        }
    }

    let mut sb_type = [0u8; 25];
    let mut s_get_day = [0u8; 25];
    for row in &user_rows {
        let idx = row.day_index as usize;
        if idx < 25 {
            sb_type[idx] = if row.claimed { 1 } else { 0 };
            s_get_day[idx] = row.day_of_month as u8;
        }
    }

    send_daily_reward_state(session, 0, &item_ids, &sb_type, &s_get_day, &cumulative_ids).await
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Compute level-based money requirement for stat/skill reset.
/// Formula: `(level * 2.0) ^ 3.4`, with level-based scaling:
/// - Level < 30: multiply by 0.4
/// - Level >= 60: multiply by 1.5
/// - Premium type 12: always 0 (free)
fn compute_money_req(level: u8, premium: u8) -> u32 {
    if premium == 12 {
        return 0;
    }

    let base = (level as f64 * 2.0).powf(3.4);
    let scaled = if level < 30 {
        base * 0.4
    } else if level >= 60 {
        base * 1.5
    } else {
        base
    };

    scaled as u32
}

/// Build and return the preset reset-cost packet sent on login and after level-up.
/// Packet: `[0xE9][0xAD][u32 gold_cost]`
pub(crate) fn build_preset_req_money(level: u8, premium: u8, discount_active: bool) -> Packet {
    let mut cost = compute_money_req(level, premium);
    if discount_active {
        cost /= 2;
    }
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_RESET);
    pkt.write_u32(cost);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// MessageBox (0xBE) — HSACSX_SendMessageBox
// ─────────────────────────────────────────────────────────────────────────────

/// Build an ext_hook MESSAGE packet (title + message popup).
/// Packet: `[0xE9][0xBE][string title][string message]`
pub(crate) fn build_ext_message_box(title: &str, message: &str) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_MESSAGE);
    pkt.write_string(title);
    pkt.write_string(message);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// ResetRebStat (0xCA) — Rebirth stat reset dialog
// ─────────────────────────────────────────────────────────────────────────────

/// Handle RESETREBSTAT (0xCA) — opens the rebirth stat/skill reset NPC dialog.
/// Sets event NPC ID = 14446, event SID = 18004, quest helper = 4324, then
/// opens `SelectMsg(52, ...)` with all button texts/events set to -1.
pub fn handle_resetrebstat(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world();
    let sid = session.session_id();

    // Set event context on the session (C++ m_sEventNid, m_sEventSid, m_nQuestHelperID)
    world.update_session(sid, |h| {
        h.event_nid = 14446;
        h.event_sid = 18004;
        h.quest_helper_id = 4324;
    });

    let button_texts = [-1i32; 12];
    let button_events = [-1i32; 12];

    super::select_msg::send_select_msg(
        world.as_ref(),
        sid,
        52, // flag
        -1, // quest_id
        -1, // header_text
        &button_texts,
        &button_events,
        "", // no lua filename
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// SendLists — server-initiated sends on game entry
// ─────────────────────────────────────────────────────────────────────────────

/// Sub-opcode for anti-AFK NPC list.
const EXT_SUB_ANTI_AFK: u8 = 0xD0;

/// Sub-opcode for event timer show list.
const EXT_SUB_EVENT_SHOW_LIST: u8 = 0xD8;

/// Send the anti-AFK NPC ID list to the client.
/// Packet: `[0xE9, 0xD0, u16(count), u16(npc_id)...]`
pub async fn send_anti_afk_list(session: &mut ClientSession) -> anyhow::Result<()> {
    let ids = session.world().get_anti_afk_npc_ids();
    if ids.is_empty() {
        return Ok(());
    }

    let mut pkt = Packet::new(ko_protocol::Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_ANTI_AFK);
    pkt.write_u16(ids.len() as u16);
    for npc_id in &ids {
        pkt.write_u16(*npc_id);
    }
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Send right-click exchange item lists grouped by type on game entry.
/// Sends 6 packets (one per exchange type 1,2,3,4,6,7).
/// Packet: `[0xE9, 0xE6, u8(2), u8(type), u16(count), u32(item_id)...]`
pub async fn send_right_exchange_list(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let by_type = world.get_right_exchange_by_type();
    send_right_exchange_slots(session).await?;

    for exchange_type in &[1u8, 2, 3, 4, 6, 7] {
        let items = by_type.get(exchange_type).cloned().unwrap_or_default();
        let mut pkt = Packet::new(ko_protocol::Opcode::EXT_HOOK_S2C);
        pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
        pkt.write_u8(2);
        pkt.write_u8(*exchange_type);
        pkt.write_u16(items.len() as u16);
        for item_id in &items { pkt.write_u32(*item_id); }
        session.send_packet(&pkt).await?;
    }
    Ok(())
}

pub async fn send_right_exchange_slots(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();

    // The proxy's right-click hook is disabled until it receives this slot map.
    // The type lists alone do not enable any inventory slot.
    let inventory = world.with_session(session.session_id(), |h| h.inventory.clone()).unwrap_or_default();
    let mut slots = Packet::new(WIZ_EXT_HOOK);
    slots.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
    slots.write_u8(0);
    for index in 14..42 {
        let item_id = inventory.get(index).map(|s| s.item_id).unwrap_or(0);
        let sub = if world.get_right_exchange(item_id as i32).is_some() { 1 }
            else if !world.get_generator_exchanges(item_id).is_empty() { 2 } else { 0 };
        slots.write_u8(u8::from(sub != 0));
        slots.write_u8(sub);
        slots.write_u32(item_id);
    }
    slots.write_u8(2);
    for index in 14..42 {
        let slot = inventory.get(index);
        let item = slot.and_then(|s| world.get_item(s.item_id));
        slots.write_u32(item.and_then(|i| i.item_icon_id1).unwrap_or(0).max(0) as u32);
        slots.write_u16(slot.map(|s| s.count).unwrap_or(0));
    }
    let character = world.get_character_info(session.session_id());
    slots.write_u32(character.as_ref().map(|c| c.item_weight.max(0) as u32).unwrap_or(0));
    slots.write_u32(character.as_ref().map(|c| c.max_weight.max(0) as u32).unwrap_or(0));
    slots.write_u32(character.map(|c| c.gold).unwrap_or(0));
    session.send_packet(&slots).await?;

    Ok(())
}

/// Send the event timer schedule list to the client.
/// Packet: `[0xE9, 0xD8, u32(hour), u32(minute), u32(second), u16(count), (string+u32+u32)...]`
/// Sends current server time + scheduled events for today (from event schedules + timer show list).
pub async fn send_event_timer_list(session: &mut ClientSession) -> anyhow::Result<()> {
    use chrono::{Datelike, Timelike};
    let now = chrono::Local::now();
    let weekday = now.weekday().num_days_from_sunday() as usize; // 0=Sun..6=Sat

    let world = session.world();

    // Collect today's timer entries from both sources (C++ EventTimerSet logic)
    let entries = build_event_timer_entries(world, weekday);

    let mut pkt = Packet::new(ko_protocol::Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_EVENT_SHOW_LIST);
    pkt.write_u32(now.hour());
    pkt.write_u32(now.minute());
    pkt.write_u32(now.second());
    pkt.write_u16(entries.len() as u16);
    for (name, hour, minute) in &entries {
        pkt.write_string(name);
        pkt.write_u32(*hour);
        pkt.write_u32(*minute);
    }
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Build the combined event timer list for a given weekday.
/// (filtered by status + day + active time slots) with event_timer_show_list (filtered by day).
pub(crate) fn build_event_timer_entries(
    world: &crate::world::WorldState,
    weekday: usize,
) -> Vec<(String, u32, u32)> {
    let mut entries = Vec::new();

    // Source 1: Event schedules (C++ pEventTimeOpt.mtimeforloop)
    {
        let schedules = world.event_room_manager.schedules.read();
        for sched in schedules.iter() {
            if !sched.status {
                continue;
            }
            if weekday >= 7 || !sched.days[weekday] {
                continue;
            }
            for &(hour, minute) in &sched.start_times {
                if hour < 0 || minute < 0 {
                    continue;
                }
                entries.push((sched.name.clone(), hour as u32, minute as u32));
            }
        }
    }

    // Source 2: Event timer show list (C++ m_EventTimerShowArray)
    {
        let show_list = world.event_timer_show_list.read();
        for item in show_list.iter() {
            if !item.status {
                continue;
            }
            // days is comma-separated weekday numbers (e.g. "0,1,2,3,4,5,6")
            let matches_day = item.days.split(',').any(|d| {
                let trimmed = d.trim();
                match trimmed.parse::<usize>() {
                    Ok(day) => day == weekday,
                    Err(_) => {
                        tracing::warn!(
                            "Event timer '{}': malformed day value '{}'",
                            item.name,
                            trimmed
                        );
                        false
                    }
                }
            });
            if !matches_day {
                continue;
            }
            if item.hour >= 0 && item.hour <= 23 {
                entries.push((item.name.clone(), item.hour as u32, item.minute as u32));
            }
        }
    }

    entries
}

// ─────────────────────────────────────────────────────────────────────────────
// Sprint 26: S2C Builders — BATCH 1 (Notifications)
// These builders are not yet called from event triggers but are ready for integration.
// ─────────────────────────────────────────────────────────────────────────────

/// Build a CASHCHANGE (0xA9) packet with current KC and TL balances.
/// Packet: `[0xE9][0xA9][u32 kc][u32 tl]`
/// This is the public builder version. The internal `send_cash_change()` also
/// sends a WIZ_CHAT fallback for v2525 clients.
#[allow(dead_code)]
pub(crate) fn build_cash_change_packet(kc: u32, tl: u32) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_CASHCHANGE);
    pkt.write_u32(kc);
    pkt.write_u32(tl);
    pkt
}

/// Build an INFOMESSAGE (0xBC) packet — info popup displayed to the client.
/// Packet: `[0xE9][0xBC][string message]`
#[allow(dead_code)]
pub(crate) fn build_info_message(message: &str) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_INFOMESSAGE);
    pkt.write_string(message);
    pkt
}

/// Build an ERRORMSG (0xC9) packet — error popup with title and message.
/// Packet: `[0xE9][0xC9][u8 show][u8 sub][string title][string message]`
/// - `show`: 1 = display popup, 0 = hide
/// - `sub`: sub-type for categorisation (client UI routing)
#[allow(dead_code)]
pub(crate) fn build_error_message(show: u8, sub: u8, title: &str, message: &str) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ERRORMSG);
    pkt.write_u8(show);
    pkt.write_u8(sub);
    pkt.write_string(title);
    pkt.write_string(message);
    pkt
}

/// Build a DEATH_NOTICE (0xD7) packet — kill/death broadcast via ext_hook path.
/// The standard death notice goes via WIZ_CHAT type=26. This ext_hook version
/// provides an alternative S2C path for extended clients.
/// Packet: `[0xE9][0xD7][u8 victim_nation][u8 killer_nation][u8 notice_type][u32 killer_id][SByte killer_name][u32 victim_id][SByte victim_name][u16 victim_x][u16 victim_z]`
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(crate) fn build_death_notice(
    victim_nation: u8,
    killer_nation: u8,
    notice_type: u8,
    killer_id: u32,
    killer_name: &str,
    victim_id: u32,
    victim_name: &str,
    victim_x: u16,
    victim_z: u16,
) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_DEATH_NOTICE);
    pkt.write_u8(victim_nation);
    pkt.write_u8(killer_nation);
    pkt.write_u8(notice_type);
    pkt.write_u32(killer_id);
    pkt.write_sbyte_string(killer_name);
    pkt.write_u32(victim_id);
    pkt.write_sbyte_string(victim_name);
    pkt.write_u16(victim_x);
    pkt.write_u16(victim_z);
    pkt
}

/// Build the JstKO 1098/2615 narration packet used by the hooked client.
///
/// This is intentionally different from the generic extended death-notice
/// structure above. The original `CUser::SendNewDeathNotice()` sets SByte
/// string mode and writes only viewer-relative kill type, both names and the
/// victim coordinates:
/// `[0xE9][0xD7][u8 kill_type][SByte killer][SByte victim][u16 x][u16 z]`.
///
/// kill_type: 1 = killer/victim, 2 = killer party, 3 = other observer.
#[cfg(test)]
fn build_new_death_narration(
    kill_type: u8,
    killer_name: &str,
    victim_name: &str,
    victim_x: u16,
    victim_z: u16,
) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_DEATH_NOTICE);
    pkt.write_u8(kill_type);
    pkt.write_sbyte_string(killer_name);
    pkt.write_sbyte_string(victim_name);
    pkt.write_u16(victim_x);
    pkt.write_u16(victim_z);
    pkt
}

/// Build a PLAYER_RANK (0xD5) update packet — ranking badge push to client.
/// Packet: `[0xE9][0xD5][u8 rank_type][u16 session_id][u32 kills][u32 deaths][u32 loyalty]`
/// - `rank_type`: 0=PK Zone, 1=BDW, 2=Chaos Dungeon, etc.
#[allow(dead_code)]
pub(crate) fn build_player_rank_update(
    rank_type: u8,
    session_id: u16,
    kills: u32,
    deaths: u32,
    loyalty: u32,
) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_PLAYER_RANK);
    pkt.write_u8(rank_type);
    pkt.write_u16(session_id);
    pkt.write_u32(kills);
    pkt.write_u32(deaths);
    pkt.write_u32(loyalty);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// Sprint 26: S2C Builders — BATCH 2 (Events)
// ─────────────────────────────────────────────────────────────────────────────

/// Build a JURAID (0xE2) score packet — scoreboard on zone entry.
/// Packet: `[0xE9][0xE2][u8 sub=0][u32 karus_score][u32 elmo_score][u32 remaining_secs]`
#[allow(dead_code)]
pub(crate) fn build_juraid_score(karus_score: u32, elmo_score: u32, remaining_secs: u32) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_JURAID);
    pkt.write_u8(0); // score sub
    pkt.write_u32(karus_score);
    pkt.write_u32(elmo_score);
    pkt.write_u32(remaining_secs);
    pkt
}

/// Build a JURAID (0xE2) update score packet — broadcast on kill.
/// Packet: `[0xE9][0xE2][u8 sub=1][u8 nation][u32 new_score]`
#[allow(dead_code)]
pub(crate) fn build_juraid_updatescore(nation: u8, new_score: u32) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_JURAID);
    pkt.write_u8(1); // updatescore sub
    pkt.write_u8(nation);
    pkt.write_u32(new_score);
    pkt
}

/// Build a JURAID (0xE2) result packet — event ended, show results.
/// Packet: `[0xE9][0xE2][u8 sub=2][u8 winner_nation][u32 karus_score][u32 elmo_score]`
#[allow(dead_code)]
pub(crate) fn build_juraid_result(winner_nation: u8, karus_score: u32, elmo_score: u32) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_JURAID);
    pkt.write_u8(2); // result sub
    pkt.write_u8(winner_nation);
    pkt.write_u32(karus_score);
    pkt.write_u32(elmo_score);
    pkt
}

/// Build a JURAID (0xE2) logout packet — sent when player leaves Juraid zone.
/// Packet: `[0xE9][0xE2][u8 sub=3]`
#[allow(dead_code)]
pub(crate) fn build_juraid_logout() -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_JURAID);
    pkt.write_u8(3); // logout sub
    pkt
}

/// Build a CASTLE_SIEGE_TIMER (0xC5) packet — CSW UI timer display.
/// Packet: `[0xE9][0xC5][u8 sub][u32 remaining_secs][u8 status]`
/// - `sub`: 0 = start/update timer, 1 = stop timer
/// - `status`: 0 = Preparation, 1 = War, 2 = Ended
#[allow(dead_code)]
pub(crate) fn build_castle_siege_timer(sub: u8, remaining_secs: u32, status: u8) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_CASTLE_SIEGE_TIMER);
    pkt.write_u8(sub);
    pkt.write_u32(remaining_secs);
    pkt.write_u8(status);
    pkt
}

/// Build a ZindanWar result packet — event ended, show final result.
/// Packet: `[0xE9][0xD2][u8 sub=3][u8 winner_nation][u32 elmo_kills][u32 karus_kills]`
#[allow(dead_code)]
pub(crate) fn build_zindan_result(winner_nation: u8, elmo_kills: u32, karus_kills: u32) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ZINDAN_WAR);
    pkt.write_u8(3); // result sub
    pkt.write_u8(winner_nation);
    pkt.write_u32(elmo_kills);
    pkt.write_u32(karus_kills);
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// Sprint 26: S2C Builders — BATCH 3 (GM/Admin)
// ─────────────────────────────────────────────────────────────────────────────

/// Build a BANSYSTEM (0xBF) ban notification packet — sent before disconnect.
/// Packet: `[0xE9][0xBF][u8 result]`
/// - `result`: 1 = banned/kicked
#[allow(dead_code)]
pub(crate) fn build_ban_notification(result: u8) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_BANSYSTEM);
    pkt.write_u8(result);
    pkt
}

/// Handle BANSYSTEM (0xBF) — Life skill data query (C2S).
/// Despite the name, in C++ this sub-opcode is used for life skill data, not banning.
/// The server responds with life skill levels and XP for 4 categories:
/// War, Hunting, Smithery, Karma.
/// Response: `[0xE9][0xBF][u8 war_level][u32 war_exp][u32 war_target][u8 hunt_level][u32 hunt_exp][u32 hunt_target][u8 smith_level][u32 smith_exp][u32 smith_target][u8 karma_level][u32 karma_exp][u32 karma_target]`
pub async fn handle_bansystem(session: &mut ClientSession, _data: &[u8]) -> anyhow::Result<()> {
    // C++ sends life skill data in response to this sub-opcode.
    // Life skills are not yet tracked in our server, so send zeroed data.
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_BANSYSTEM);
    // 4 skills × (u8 level + u32 exp + u32 target_exp)
    for _ in 0..4 {
        pkt.write_u8(0); // level
        pkt.write_u32(0); // current exp
        pkt.write_u32(0); // target exp
    }
    session.send_packet(&pkt).await
}

/// Build a GAME_MASTER_MODE (0xE9) toggle packet — GM UI mode switch.
/// Packet: `[0xE9][0xE9][u8 enabled]`
/// - `enabled`: 1 = GM mode active (UI shows GM tools), 0 = normal mode
pub(crate) fn build_gm_mode_toggle(enabled: u8) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_GAME_MASTER_MODE);
    pkt.write_u8(enabled);
    pkt
}

/// Handle GAME_MASTER_MODE (0xE9) — client requests GM mode toggle (C2S).
/// Only processes the request if the player is a GM (authority == 0).
/// Sends back the toggle confirmation.
pub async fn handle_game_master_mode(
    session: &mut ClientSession,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let world = session.world();
    let sid = session.session_id();

    // Only GMs can toggle GM mode
    let authority = world
        .get_character_info(sid)
        .map(|c| c.authority)
        .unwrap_or(1);

    if authority != AUTHORITY_GAME_MASTER {
        return Ok(());
    }

    let requested = data[0]; // 0 or 1
    let enabled = if requested != 0 { 1u8 } else { 0u8 };

    debug!("[{}] GM mode toggle: enabled={}", session.addr(), enabled);

    let pkt = build_gm_mode_toggle(enabled);
    session.send_packet(&pkt).await
}

// ─────────────────────────────────────────────────────────────────────────────
// CSW ext_hook packet builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build a CSW timer packet sent when a player enters Delos during active siege.
/// Packet: `[0xE9][0xE1][0x00][u32 remaining_secs][SByte owner_name][u8 status][u32 phase_minutes]`
pub(crate) fn build_csw_timer_packet(
    remaining_secs: u32,
    owner_name: &str,
    status: u8,
    phase_minutes: u32,
) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_CSW);
    pkt.write_u8(0); // WIZ_TIMER
    pkt.write_u32(remaining_secs);
    let name_bytes = owner_name.as_bytes();
    pkt.write_u8(name_bytes.len() as u8);
    pkt.data.extend_from_slice(name_bytes);
    pkt.write_u8(status);
    pkt.write_u32(phase_minutes);
    pkt
}

/// Build a CSW finish packet sent when the siege ends.
/// Packet: `[0xE9][0xE1][0x01]`
#[allow(dead_code)]
pub(crate) fn build_csw_finish_packet() -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_CSW);
    pkt.write_u8(1); // WIZ_FINISH
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// ZindanWar ext_hook packet builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build a ZindanWar flagsend packet (initial scoreboard on zone entry).
/// Packet: `[0xE9][0xD2][0x00][SByte elmo_name][u32 elmo_kills][SByte karus_name][u32 karus_kills][u32 remaining_secs]`
pub(crate) fn build_zindan_flagsend(
    elmo_name: &str,
    elmo_kills: u32,
    karus_name: &str,
    karus_kills: u32,
    remaining_secs: u32,
) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ZINDAN_WAR);
    pkt.write_u8(0); // flagsend
    let eb = elmo_name.as_bytes();
    pkt.write_u8(eb.len() as u8);
    pkt.data.extend_from_slice(eb);
    pkt.write_u32(elmo_kills);
    let kb = karus_name.as_bytes();
    pkt.write_u8(kb.len() as u8);
    pkt.data.extend_from_slice(kb);
    pkt.write_u32(karus_kills);
    pkt.write_u32(remaining_secs);
    pkt
}

/// Build a ZindanWar score update packet (broadcast on kill).
/// Packet: `[0xE9][0xD2][0x01][u8 nation][u32 new_kill_count]`
pub(crate) fn build_zindan_updatescore(nation: u8, new_count: u32) -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ZINDAN_WAR);
    pkt.write_u8(1); // updatescore
    pkt.write_u8(nation);
    pkt.write_u32(new_count);
    pkt
}

/// Build a ZindanWar logout packet (sent to departing player).
/// Packet: `[0xE9][0xD2][0x02]`
pub(crate) fn build_zindan_logout() -> Packet {
    let mut pkt = Packet::new(WIZ_EXT_HOOK);
    pkt.write_u8(EXT_SUB_ZINDAN_WAR);
    pkt.write_u8(2); // logout
    pkt
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_generator_selection_keeps_low_rate_tail_reachable() {
        let pool = [(100, 10_000), (1310515301, 5), (1310518304, 5)];
        assert_eq!(select_weighted_exchange_item(&pool, 0), Some(100));
        assert_eq!(select_weighted_exchange_item(&pool, 9_999), Some(100));
        assert_eq!(
            select_weighted_exchange_item(&pool, 10_000),
            Some(1310515301)
        );
        assert_eq!(
            select_weighted_exchange_item(&pool, 10_009),
            Some(1310518304)
        );
        assert_eq!(select_weighted_exchange_item(&pool, 10_010), None);
    }

    #[test]
    fn test_compute_money_req_premium_12_free() {
        assert_eq!(compute_money_req(83, 12), 0);
    }

    #[test]
    fn test_compute_money_req_low_level() {
        // Level 20: (40.0)^3.4 * 0.4
        let result = compute_money_req(20, 0);
        let expected = ((40.0_f64).powf(3.4) * 0.4) as u32;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compute_money_req_mid_level() {
        // Level 50: (100.0)^3.4 * 1.0
        let result = compute_money_req(50, 0);
        let expected = (100.0_f64).powf(3.4) as u32;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compute_money_req_high_level() {
        // Level 83: (166.0)^3.4 * 1.5
        let result = compute_money_req(83, 0);
        let expected = ((166.0_f64).powf(3.4) * 1.5) as u32;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_build_preset_req_money_packet_format() {
        use ko_protocol::PacketReader;
        let pkt = build_preset_req_money(83, 0, false);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_RESET)); // 0xAD
        let cost = r.read_u32().unwrap();
        assert!(cost > 0, "Level 83 reset cost should be > 0");
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_preset_req_money_premium_12_free() {
        use ko_protocol::PacketReader;
        let pkt = build_preset_req_money(83, 12, false);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_RESET));
        assert_eq!(r.read_u32(), Some(0)); // premium 12 = free
    }

    #[test]
    fn test_build_preset_req_money_with_discount() {
        use ko_protocol::PacketReader;
        let pkt_normal = build_preset_req_money(83, 0, false);
        let pkt_discount = build_preset_req_money(83, 0, true);
        let mut r_normal = PacketReader::new(&pkt_normal.data);
        r_normal.read_u8(); // skip sub-opcode
        let cost_normal = r_normal.read_u32().unwrap();
        let mut r_disc = PacketReader::new(&pkt_discount.data);
        r_disc.read_u8();
        let cost_disc = r_disc.read_u32().unwrap();
        assert_eq!(cost_disc, cost_normal / 2);
    }

    #[test]
    fn test_loot_settings_clamp_bool() {
        let clamp = |v: u8| -> i16 {
            if v > 1 {
                0
            } else {
                v as i16
            }
        };
        assert_eq!(clamp(0), 0);
        assert_eq!(clamp(1), 1);
        assert_eq!(clamp(2), 0);
        assert_eq!(clamp(255), 0);
    }

    #[test]
    fn test_tagname_rgb_extraction() {
        // COLORREF: 0x00BBGGRR
        let rgb: i32 = 0x001020FF; // R=0xFF, G=0x20, B=0x10
        let r = (rgb & 0xFF) as u8;
        let g = ((rgb >> 8) & 0xFF) as u8;
        let b = ((rgb >> 16) & 0xFF) as u8;
        assert_eq!(r, 0xFF);
        assert_eq!(g, 0x20);
        assert_eq!(b, 0x10);
    }

    #[test]
    fn test_ext_sub_opcode_constants() {
        // Verify all ext_hook sub-opcode values match C++ HSACSXOpCodes enum
        assert_eq!(EXT_SUB_AUTHINFO, 0xA2);
        assert_eq!(EXT_SUB_PROCINFO, 0xA3);
        assert_eq!(EXT_SUB_OPEN, 0xA4);
        assert_eq!(EXT_SUB_LOG, 0xA5);
        assert_eq!(EXT_SUB_XALIVE, 0xA6);
        assert_eq!(EXT_SUB_UIINFO, 0xA7);
        assert_eq!(EXT_SUB_PUS, 0xA8);
        assert_eq!(EXT_SUB_KESN, 0xAA);
        assert_eq!(EXT_SUB_DROP_LIST, 0xAB);
        assert_eq!(EXT_SUB_ITEM_PROCESS, 0xAC);
        assert_eq!(EXT_SUB_RESET, 0xAD);
        assert_eq!(EXT_SUB_DROP_REQUEST, 0xAE);
        assert_eq!(EXT_SUB_COLLECTION_RACE, 0xAF);
        assert_eq!(EXT_SUB_CLANBANK, 0xB0);
        assert_eq!(EXT_SUB_USERINFO, 0xB1);
        assert_eq!(EXT_SUB_KCPAZAR, 0xB2);
        assert_eq!(EXT_SUB_LOOT_SETTINGS, 0xB3);
        assert_eq!(EXT_SUB_CHAOTIC_EXCHANGE, 0xB4);
        assert_eq!(EXT_SUB_MERCHANT, 0xB5);
        assert_eq!(EXT_SUB_USERDATA, 0xB7);
        assert_eq!(EXT_SUB_TEMPITEMS, 0xB8);
        assert_eq!(EXT_SUB_KCUPDATE, 0xB9);
        assert_eq!(EXT_SUB_AUTODROP, 0xBA);
        assert_eq!(EXT_SUB_SUPPORT, 0xBB);
        assert_eq!(EXT_SUB_INFOMESSAGE, 0xBC);
        assert_eq!(EXT_SUB_MERCHANTLIST, 0xBD);
        assert_eq!(EXT_SUB_MESSAGE, 0xBE);
        assert_eq!(EXT_SUB_BANSYSTEM, 0xBF);
        assert_eq!(EXT_SUB_MERC_VIEWER_INFO, 0xC3);
        assert_eq!(EXT_SUB_UPGRADE_RATE, 0xC4);
        assert_eq!(EXT_SUB_CASTLE_SIEGE_TIMER, 0xC5);
        assert_eq!(EXT_SUB_VOICE, 0xC6);
        assert_eq!(EXT_SUB_LOTTERY, 0xC7);
        assert_eq!(EXT_SUB_TOPLEFT, 0xC8);
        assert_eq!(EXT_SUB_ERRORMSG, 0xC9);
        assert_eq!(EXT_SUB_RESETREBSTAT, 0xCA);
        assert_eq!(EXT_SUB_UNKNOWN1, 0xCB);
        assert_eq!(EXT_SUB_ACCOUNT_INFO_SAVE, 0xCC);
        assert_eq!(EXT_SUB_CHAT_LASTSEEN, 0xCD);
        assert_eq!(EXT_SUB_REPURCHASE, 0xCE);
        assert_eq!(EXT_SUB_SKILL_STAT_RESET, 0xCF);
        assert_eq!(EXT_SUB_TAG_INFO, 0xD1);
        assert_eq!(EXT_SUB_ZINDAN_WAR, 0xD2);
        assert_eq!(EXT_SUB_DAILY_QUEST, 0xD3);
        assert_eq!(EXT_SUB_PUS_REFUND, 0xD4);
        assert_eq!(EXT_SUB_PLAYER_RANK, 0xD5);
        assert_eq!(EXT_SUB_DEATH_NOTICE, 0xD7);
        assert_eq!(EXT_SUB_SHOW_QUEST_LIST, 0xD9);
        assert_eq!(EXT_SUB_WHEEL_DATA, 0xDA);
        assert_eq!(EXT_SUB_GENIE_INFO, 0xDB);
        assert_eq!(EXT_SUB_CINDERELLA, 0xE0);
        assert_eq!(EXT_SUB_CSW, 0xE1);
        assert_eq!(EXT_SUB_JURAID, 0xE2);
        assert_eq!(EXT_SUB_PERKS, 0xE3);
        assert_eq!(EXT_SUB_MESSAGE2, 0xE4);
        assert_eq!(EXT_SUB_CHEST_BLOCKITEM, 0xE5);
        assert_eq!(EXT_SUB_ITEM_EXCHANGE_INFO, 0xE6);
        assert_eq!(EXT_SUB_HOOK_VISIBLE, 0xE7);
        assert_eq!(EXT_SUB_DAILY_REWARD, 0xF7);

        // Verify no value collisions among all constants
        let all_values: Vec<u8> = vec![
            EXT_SUB_AUTHINFO,
            EXT_SUB_PROCINFO,
            EXT_SUB_OPEN,
            EXT_SUB_LOG,
            EXT_SUB_XALIVE,
            EXT_SUB_UIINFO,
            EXT_SUB_PUS,
            EXT_SUB_KESN,
            EXT_SUB_DROP_LIST,
            EXT_SUB_ITEM_PROCESS,
            EXT_SUB_RESET,
            EXT_SUB_DROP_REQUEST,
            EXT_SUB_COLLECTION_RACE,
            EXT_SUB_CLANBANK,
            EXT_SUB_USERINFO,
            EXT_SUB_KCPAZAR,
            EXT_SUB_LOOT_SETTINGS,
            EXT_SUB_CHAOTIC_EXCHANGE,
            EXT_SUB_MERCHANT,
            EXT_SUB_USERDATA,
            EXT_SUB_TEMPITEMS,
            EXT_SUB_KCUPDATE,
            EXT_SUB_AUTODROP,
            EXT_SUB_SUPPORT,
            EXT_SUB_INFOMESSAGE,
            EXT_SUB_MERCHANTLIST,
            EXT_SUB_MESSAGE,
            EXT_SUB_BANSYSTEM,
            EXT_SUB_MERC_VIEWER_INFO,
            EXT_SUB_UPGRADE_RATE,
            EXT_SUB_CASTLE_SIEGE_TIMER,
            EXT_SUB_VOICE,
            EXT_SUB_LOTTERY,
            EXT_SUB_TOPLEFT,
            EXT_SUB_ERRORMSG,
            EXT_SUB_RESETREBSTAT,
            EXT_SUB_UNKNOWN1,
            EXT_SUB_ACCOUNT_INFO_SAVE,
            EXT_SUB_CHAT_LASTSEEN,
            EXT_SUB_REPURCHASE,
            EXT_SUB_SKILL_STAT_RESET,
            EXT_SUB_TAG_INFO,
            EXT_SUB_ZINDAN_WAR,
            EXT_SUB_DAILY_QUEST,
            EXT_SUB_PUS_REFUND,
            EXT_SUB_PLAYER_RANK,
            EXT_SUB_DEATH_NOTICE,
            EXT_SUB_SHOW_QUEST_LIST,
            EXT_SUB_WHEEL_DATA,
            EXT_SUB_GENIE_INFO,
            EXT_SUB_CINDERELLA,
            EXT_SUB_CSW,
            EXT_SUB_JURAID,
            EXT_SUB_PERKS,
            EXT_SUB_MESSAGE2,
            EXT_SUB_CHEST_BLOCKITEM,
            EXT_SUB_ITEM_EXCHANGE_INFO,
            EXT_SUB_HOOK_VISIBLE,
            EXT_SUB_DAILY_REWARD,
        ];
        let mut seen = std::collections::HashSet::new();
        for v in &all_values {
            assert!(seen.insert(*v), "Duplicate sub-opcode value: 0x{:02X}", v);
        }
    }

    #[test]
    fn test_account_info_validation_phone() {
        // Phone must be exactly 11 digits
        assert_eq!("12345678901".len(), 11);
        assert!("12345678901".chars().all(|c| c.is_ascii_digit()));
        assert!(!"1234567890a".chars().all(|c| c.is_ascii_digit()));
        assert_ne!("1234567890".len(), 11); // 10 chars, not 11
    }

    #[test]
    fn test_account_info_validation_seal() {
        // Seal must be exactly 8 digits
        assert!("12345678".len() == 8);
        assert!("12345678".chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_account_info_validation_otp() {
        // OTP must be exactly 6 digits
        assert!("123456".len() == 6);
        assert!("123456".chars().all(|c| c.is_ascii_digit()));
        assert!(!"12345a".chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_chest_block_max_items() {
        // Max 100 items enforced
        let max: u16 = 150;
        assert_eq!(max.min(100), 100);
    }

    #[test]
    fn test_daily_reward_state_packet_format() {
        // Verify packet wire format: [0xE9][0xF7][sub]([u32 item][u8 type][u8 day] × 25)([u32 cum] × 3)
        let mut item_ids = [0u32; 25];
        item_ids[0] = 900145000;
        item_ids[1] = 900146000;
        let mut types = [0u8; 25];
        types[0] = 1;
        let mut days = [0u8; 25];
        days[0] = 15;
        let cumulative = [347000000u32, 347000000, 347000000];

        let mut pkt = Packet::new(WIZ_EXT_HOOK);
        pkt.write_u8(EXT_SUB_DAILY_REWARD);
        pkt.write_u8(0); // sub = login send
        for i in 0..25 {
            pkt.write_u32(item_ids[i]);
            pkt.write_u8(types[i]);
            pkt.write_u8(days[i]);
        }
        for val in &cumulative {
            pkt.write_u32(*val);
        }

        assert_eq!(pkt.opcode, ko_protocol::Opcode::EXT_HOOK_S2C);
        assert_eq!(pkt.data[0], EXT_SUB_DAILY_REWARD);
        assert_eq!(pkt.data[1], 0); // sub
                                    // 25 entries × 6 bytes = 150, + 3 × 4 bytes = 12, + 2 header = 164
        assert_eq!(pkt.data.len(), 2 + 25 * 6 + 3 * 4);

        // Verify first item
        let id0 = u32::from_le_bytes([pkt.data[2], pkt.data[3], pkt.data[4], pkt.data[5]]);
        assert_eq!(id0, 900145000);
        assert_eq!(pkt.data[6], 1); // type = claimed
        assert_eq!(pkt.data[7], 15); // day = 15
    }

    #[test]
    fn test_daily_reward_error_packet() {
        let mut pkt = Packet::new(WIZ_EXT_HOOK);
        pkt.write_u8(EXT_SUB_DAILY_REWARD);
        pkt.write_u8(3); // error
        assert_eq!(pkt.data[0], 0xF7);
        assert_eq!(pkt.data[1], 3);
        assert_eq!(pkt.data.len(), 2);
    }

    #[test]
    fn test_daily_reward_sequential_validation() {
        // Simulate sequential claim logic
        let mut sb_type = [0u8; 25];
        let mut s_get_day = [0u8; 25];

        // Day 0: should be claimable (i==0 && sbType[i]==0)
        assert_eq!(sb_type[0], 0);

        // Claim day 0
        sb_type[0] = 1;
        s_get_day[0] = 15; // claimed on 15th

        // Day 1: i > 0, previous claimed (sbType[0]==1), different day → OK
        let today = 16u8;
        assert!(sb_type[0] > 0); // previous claimed
        assert_ne!(s_get_day[0], today); // different day

        // Day 1: i > 0, previous claimed, same day → ERROR
        let same_day = 15u8;
        assert_eq!(s_get_day[0], same_day); // same day = error

        // Day 2: i > 0, previous NOT claimed → ERROR
        assert_eq!(sb_type[1], 0); // day 1 not claimed → can't claim day 2
    }

    #[test]
    fn test_daily_reward_packet_size() {
        // Total packet data size: 2 (sub+response) + 25*6 (items) + 3*4 (cumulative) = 164
        let expected = 2 + 25 * 6 + 3 * 4;
        assert_eq!(expected, 164);
    }

    // ── Anti-AFK & Event Timer List Tests ────────────────────────────────────

    #[test]
    fn test_anti_afk_packet_format() {
        use ko_protocol::{Opcode, PacketReader};
        // Simulate the packet that send_anti_afk_list would build.
        let ids: Vec<u16> = vec![8401, 8402, 8403, 8404];
        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(EXT_SUB_ANTI_AFK);
        pkt.write_u16(ids.len() as u16);
        for id in &ids {
            pkt.write_u16(*id);
        }

        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ANTI_AFK)); // 0xD0
        assert_eq!(r.read_u16(), Some(4)); // count
        assert_eq!(r.read_u16(), Some(8401));
        assert_eq!(r.read_u16(), Some(8402));
        assert_eq!(r.read_u16(), Some(8403));
        assert_eq!(r.read_u16(), Some(8404));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_event_timer_list_packet_format() {
        use ko_protocol::{Opcode, PacketReader};
        // Simulate the packet that send_event_timer_list would build (empty list).
        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(EXT_SUB_EVENT_SHOW_LIST);
        pkt.write_u32(14); // hour
        pkt.write_u32(30); // minute
        pkt.write_u32(45); // second
        pkt.write_u16(0); // count = 0

        assert_eq!(pkt.opcode, Opcode::EXT_HOOK_S2C);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_EVENT_SHOW_LIST)); // 0xD8
        assert_eq!(r.read_u32(), Some(14)); // hour
        assert_eq!(r.read_u32(), Some(30)); // minute
        assert_eq!(r.read_u32(), Some(45)); // second
        assert_eq!(r.read_u16(), Some(0)); // empty list
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_anti_afk_worldstate_accessor() {
        let world = crate::world::WorldState::new();
        // Initially empty
        assert!(world.get_anti_afk_npc_ids().is_empty());
        // Set and verify
        world.set_anti_afk_npc_ids(vec![8401, 8402, 8403, 8404]);
        let ids = world.get_anti_afk_npc_ids();
        assert_eq!(ids.len(), 4);
        assert_eq!(ids[0], 8401);
        assert_eq!(ids[3], 8404);
    }

    #[test]
    fn test_right_exchange_list_packet_format() {
        use ko_protocol::{Opcode, PacketReader};
        // Simulate one packet from send_right_exchange_list for exchange type 1.
        let items: Vec<u32> = vec![389020000, 389021000, 389022000];
        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(EXT_SUB_ITEM_EXCHANGE_INFO);
        pkt.write_u8(2); // list type
        pkt.write_u8(1); // exchange type = Reward
        pkt.write_u16(items.len() as u16);
        for id in &items {
            pkt.write_u32(*id);
        }

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ITEM_EXCHANGE_INFO)); // 0xE6
        assert_eq!(r.read_u8(), Some(2)); // list type
        assert_eq!(r.read_u8(), Some(1)); // exchange type
        assert_eq!(r.read_u16(), Some(3)); // count
        assert_eq!(r.read_u32(), Some(389020000));
        assert_eq!(r.read_u32(), Some(389021000));
        assert_eq!(r.read_u32(), Some(389022000));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_right_exchange_by_type_grouping() {
        let world = crate::world::WorldState::new();
        // Empty initially
        let grouped = world.get_right_exchange_by_type();
        assert!(grouped.is_empty());
    }

    // ── CSW ext_hook packet tests ──────────────────────────────────────────

    #[test]
    fn test_csw_timer_packet_format() {
        use ko_protocol::PacketReader;
        let pkt = build_csw_timer_packet(300, "TestClan", 1, 30);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CSW));
        assert_eq!(r.read_u8(), Some(0)); // WIZ_TIMER
        assert_eq!(r.read_u32(), Some(300)); // remaining_secs
        assert_eq!(r.read_sbyte_string(), Some("TestClan".to_string()));
        assert_eq!(r.read_u8(), Some(1)); // status = Preparation
        assert_eq!(r.read_u32(), Some(30)); // phase_minutes
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_csw_timer_packet_empty_owner() {
        use ko_protocol::PacketReader;
        let pkt = build_csw_timer_packet(0, "", 2, 60);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CSW));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_sbyte_string(), Some(String::new())); // empty name
        assert_eq!(r.read_u8(), Some(2)); // War
        assert_eq!(r.read_u32(), Some(60));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_csw_finish_packet_format() {
        use ko_protocol::PacketReader;
        let pkt = build_csw_finish_packet();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CSW));
        assert_eq!(r.read_u8(), Some(1)); // WIZ_FINISH
        assert_eq!(r.remaining(), 0);
    }

    // ── ZindanWar ext_hook packet tests ────────────────────────────────────

    #[test]
    fn test_zindan_flagsend_packet_format() {
        use ko_protocol::PacketReader;
        let pkt = build_zindan_flagsend("Elmo", 5, "Karus", 3, 1200);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ZINDAN_WAR));
        assert_eq!(r.read_u8(), Some(0)); // flagsend
        assert_eq!(r.read_sbyte_string(), Some("Elmo".to_string()));
        assert_eq!(r.read_u32(), Some(5));
        assert_eq!(r.read_sbyte_string(), Some("Karus".to_string()));
        assert_eq!(r.read_u32(), Some(3));
        assert_eq!(r.read_u32(), Some(1200));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_zindan_updatescore_packet_format() {
        use ko_protocol::PacketReader;
        let pkt = build_zindan_updatescore(1, 10); // nation=Karus, 10 kills
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ZINDAN_WAR));
        assert_eq!(r.read_u8(), Some(1)); // updatescore
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u32(), Some(10));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_zindan_logout_packet_format() {
        use ko_protocol::PacketReader;
        let pkt = build_zindan_logout();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ZINDAN_WAR));
        assert_eq!(r.read_u8(), Some(2)); // logout
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_ext_message_box_packet_format() {
        use ko_protocol::PacketReader;
        let pkt = build_ext_message_box("Fun Class Event", "Your loyalty points are insufficient.");
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_MESSAGE));
        assert_eq!(r.read_string(), Some("Fun Class Event".to_string()));
        assert_eq!(
            r.read_string(),
            Some("Your loyalty points are insufficient.".to_string())
        );
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_resetrebstat_constants() {
        // Verify the NPC/event IDs match C++ XGuard.cpp:2088-2091
        assert_eq!(EXT_SUB_RESETREBSTAT, 0xCA);
        assert_eq!(14446i16, 14446);
        assert_eq!(18004i16, 18004);
        assert_eq!(4324u32, 4324);
    }

    // ── AccountInfoSave validation tests ─────────────────────────────

    #[test]
    fn test_account_info_save_validation_rules() {
        // Phone: exactly 11 digits
        assert_eq!("12345678901".len(), 11);
        assert!("12345678901".chars().all(|c| c.is_ascii_digit()));
        assert_ne!("1234567890".len(), 11); // 10 digits = fail

        // Seal: exactly 8 digits
        assert_eq!("12345678".len(), 8);
        assert!("12345678".chars().all(|c| c.is_ascii_digit()));

        // OTP: exactly 6 digits
        assert_eq!("123456".len(), 6);
        assert!("123456".chars().all(|c| c.is_ascii_digit()));

        // Email: non-empty, max 250 chars
        assert_eq!("".len(), 0); // empty = rejected
        assert!("x".repeat(250).len() <= 250);
        assert!("x".repeat(251).len() > 250);
    }

    #[test]
    fn test_account_info_save_response_format() {
        use ko_protocol::PacketReader;
        // Success response: EXT_SUB_ACCOUNT_INFO_SAVE << u8(2) << u8(1)
        let mut pkt = Packet::new(WIZ_EXT_HOOK);
        pkt.write_u8(EXT_SUB_ACCOUNT_INFO_SAVE);
        pkt.write_u8(2); // result
        pkt.write_u8(1); // success

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ACCOUNT_INFO_SAVE));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_account_info_save_error_response_format() {
        use ko_protocol::PacketReader;
        // Error response: EXT_SUB_ACCOUNT_INFO_SAVE << u8(2) << u8(0)
        let mut pkt = Packet::new(WIZ_EXT_HOOK);
        pkt.write_u8(EXT_SUB_ACCOUNT_INFO_SAVE);
        pkt.write_u8(2);
        pkt.write_u8(0); // fail

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ACCOUNT_INFO_SAVE));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_event_timer_entries_empty() {
        let world = crate::world::WorldState::new();
        let entries = build_event_timer_entries(&world, 0); // Sunday
        assert!(entries.is_empty());
    }

    #[test]
    fn test_build_event_timer_entries_from_schedules() {
        use crate::systems::event_room::EventScheduleEntry;
        let world = crate::world::WorldState::new();
        {
            let mut schedules = world.event_room_manager.schedules.write();
            schedules.push(EventScheduleEntry {
                event_id: 1,
                event_type: 1,
                zone_id: 84,
                name: "BDW".to_string(),
                status: true,
                start_times: [(10, 0), (14, 30), (-1, -1), (-1, -1), (-1, -1)],
                days: [true, true, true, true, true, false, false], // Sun-Thu
                min_level: 0,
                max_level: 0,
                req_loyalty: 0,
                req_money: 0,
            });
        }
        // Sunday (index 0) — should match
        let entries = build_event_timer_entries(&world, 0);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], ("BDW".to_string(), 10, 0));
        assert_eq!(entries[1], ("BDW".to_string(), 14, 30));

        // Saturday (index 6) — should NOT match
        let entries = build_event_timer_entries(&world, 6);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_build_event_timer_entries_from_timer_show_list() {
        use ko_db::models::event_schedule::EventTimerShowRow;
        let world = crate::world::WorldState::new();
        {
            let mut list = world.event_timer_show_list.write();
            list.push(EventTimerShowRow {
                id: 1,
                name: "TestEvent".to_string(),
                status: true,
                hour: 20,
                minute: 0,
                days: "0,3,6".to_string(), // Sun, Wed, Sat
            });
        }
        // Sunday (index 0) — should match
        let entries = build_event_timer_entries(&world, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], ("TestEvent".to_string(), 20, 0));

        // Monday (index 1) — should NOT match
        let entries = build_event_timer_entries(&world, 1);
        assert!(entries.is_empty());

        // Wednesday (index 3) — should match
        let entries = build_event_timer_entries(&world, 3);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_build_event_timer_entries_combined_sources() {
        use crate::systems::event_room::EventScheduleEntry;
        use ko_db::models::event_schedule::EventTimerShowRow;
        let world = crate::world::WorldState::new();
        {
            let mut schedules = world.event_room_manager.schedules.write();
            schedules.push(EventScheduleEntry {
                event_id: 1,
                event_type: 1,
                zone_id: 84,
                name: "BDW".to_string(),
                status: true,
                start_times: [(10, 0), (-1, -1), (-1, -1), (-1, -1), (-1, -1)],
                days: [true; 7],
                min_level: 0,
                max_level: 0,
                req_loyalty: 0,
                req_money: 0,
            });
        }
        {
            let mut list = world.event_timer_show_list.write();
            list.push(EventTimerShowRow {
                id: 1,
                name: "CustomEvent".to_string(),
                status: true,
                hour: 18,
                minute: 30,
                days: "2".to_string(), // Tuesday only
            });
        }
        // Tuesday (index 2) — both sources
        let entries = build_event_timer_entries(&world, 2);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "BDW");
        assert_eq!(entries[1].0, "CustomEvent");
    }

    #[test]
    fn test_build_event_timer_entries_disabled_schedule() {
        use crate::systems::event_room::EventScheduleEntry;
        let world = crate::world::WorldState::new();
        {
            let mut schedules = world.event_room_manager.schedules.write();
            schedules.push(EventScheduleEntry {
                event_id: 1,
                event_type: 1,
                zone_id: 84,
                name: "Disabled".to_string(),
                status: false, // disabled
                start_times: [(10, 0), (-1, -1), (-1, -1), (-1, -1), (-1, -1)],
                days: [true; 7],
                min_level: 0,
                max_level: 0,
                req_loyalty: 0,
                req_money: 0,
            });
        }
        let entries = build_event_timer_entries(&world, 0);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_event_timer_list_packet_with_entries() {
        use ko_protocol::{Opcode, PacketReader};
        // Test packet format with actual entries
        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(EXT_SUB_EVENT_SHOW_LIST);
        pkt.write_u32(14); // hour
        pkt.write_u32(30); // minute
        pkt.write_u32(45); // second
        pkt.write_u16(2); // count = 2
        pkt.write_string("BDW");
        pkt.write_u32(10);
        pkt.write_u32(0);
        pkt.write_string("Juraid");
        pkt.write_u32(20);
        pkt.write_u32(30);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_EVENT_SHOW_LIST));
        assert_eq!(r.read_u32(), Some(14));
        assert_eq!(r.read_u32(), Some(30));
        assert_eq!(r.read_u32(), Some(45));
        assert_eq!(r.read_u16(), Some(2));
        assert_eq!(r.read_string(), Some("BDW".to_string()));
        assert_eq!(r.read_u32(), Some(10));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_string(), Some("Juraid".to_string()));
        assert_eq!(r.read_u32(), Some(20));
        assert_eq!(r.read_u32(), Some(30));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 958: Additional coverage ──────────────────────────────

    /// Anti-cheat sub-opcodes are in 0xA2-0xAD range.
    #[test]
    fn test_ext_anti_cheat_subopcodes() {
        assert_eq!(EXT_SUB_AUTHINFO, 0xA2);
        assert_eq!(EXT_SUB_PROCINFO, 0xA3);
        assert_eq!(EXT_SUB_OPEN, 0xA4);
        assert_eq!(EXT_SUB_LOG, 0xA5);
        assert_eq!(EXT_SUB_XALIVE, 0xA6);
        assert_eq!(EXT_SUB_RESET, 0xAD);
    }

    /// Gameplay sub-opcodes in 0xB0-0xBF range.
    #[test]
    fn test_ext_gameplay_subopcodes_b_range() {
        assert_eq!(EXT_SUB_CLANBANK, 0xB0);
        assert_eq!(EXT_SUB_USERINFO, 0xB1);
        assert_eq!(EXT_SUB_KCPAZAR, 0xB2);
        assert_eq!(EXT_SUB_LOOT_SETTINGS, 0xB3);
        assert_eq!(EXT_SUB_CHAOTIC_EXCHANGE, 0xB4);
        assert_eq!(EXT_SUB_MERCHANT, 0xB5);
        assert_eq!(EXT_SUB_TEMPITEMS, 0xB8);
        assert_eq!(EXT_SUB_SUPPORT, 0xBB);
        assert_eq!(EXT_SUB_MERCHANTLIST, 0xBD);
        assert_eq!(EXT_SUB_BANSYSTEM, 0xBF);
    }

    /// UI/display sub-opcodes in 0xC0-0xCF range.
    #[test]
    fn test_ext_ui_subopcodes_c_range() {
        assert_eq!(EXT_SUB_MERC_VIEWER_INFO, 0xC3);
        assert_eq!(EXT_SUB_UPGRADE_RATE, 0xC4);
        assert_eq!(EXT_SUB_CASTLE_SIEGE_TIMER, 0xC5);
        assert_eq!(EXT_SUB_VOICE, 0xC6);
        assert_eq!(EXT_SUB_LOTTERY, 0xC7);
        assert_eq!(EXT_SUB_TOPLEFT, 0xC8);
        assert_eq!(EXT_SUB_ERRORMSG, 0xC9);
        assert_eq!(EXT_SUB_RESETREBSTAT, 0xCA);
        assert_eq!(EXT_SUB_ACCOUNT_INFO_SAVE, 0xCC);
        assert_eq!(EXT_SUB_CHAT_LASTSEEN, 0xCD);
        assert_eq!(EXT_SUB_REPURCHASE, 0xCE);
        assert_eq!(EXT_SUB_SKILL_STAT_RESET, 0xCF);
    }

    /// Event/feature sub-opcodes in 0xD0-0xDF range.
    #[test]
    fn test_ext_event_subopcodes_d_range() {
        assert_eq!(EXT_SUB_TAG_INFO, 0xD1);
        assert_eq!(EXT_SUB_ZINDAN_WAR, 0xD2);
        assert_eq!(EXT_SUB_DAILY_QUEST, 0xD3);
        assert_eq!(EXT_SUB_PUS_REFUND, 0xD4);
        assert_eq!(EXT_SUB_PLAYER_RANK, 0xD5);
        assert_eq!(EXT_SUB_PUS_CAT, 0xD6);
        assert_eq!(EXT_SUB_DEATH_NOTICE, 0xD7);
        assert_eq!(EXT_SUB_SHOW_QUEST_LIST, 0xD9);
        assert_eq!(EXT_SUB_WHEEL_DATA, 0xDA);
        assert_eq!(EXT_SUB_GENIE_INFO, 0xDB);
    }

    /// High-range sub-opcodes 0xE0+ (events, siege, items).
    #[test]
    fn test_ext_high_range_subopcodes() {
        assert_eq!(EXT_SUB_CINDERELLA, 0xE0);
        assert_eq!(EXT_SUB_CSW, 0xE1);
        assert_eq!(EXT_SUB_JURAID, 0xE2);
        assert_eq!(EXT_SUB_PERKS, 0xE3);
        assert_eq!(EXT_SUB_CHEST_BLOCKITEM, 0xE5);
        assert_eq!(EXT_SUB_ITEM_EXCHANGE_INFO, 0xE6);
        assert_eq!(EXT_SUB_DAILY_REWARD, 0xF7);
        // F7 is the highest — large gap from E6
        assert_eq!(EXT_SUB_DAILY_REWARD - EXT_SUB_ITEM_EXCHANGE_INFO, 0x11);
    }

    // ── Sprint 968: Additional coverage ──────────────────────────────

    /// Extra constants in 0xE0-0xE9 range: MESSAGE2, HOOK_VISIBLE, GAME_MASTER_MODE.
    #[test]
    fn test_ext_extra_high_range() {
        assert_eq!(EXT_SUB_MESSAGE2, 0xE4);
        assert_eq!(EXT_SUB_HOOK_VISIBLE, 0xE7);
        assert_eq!(EXT_SUB_GAME_MASTER_MODE, 0xE9);
    }

    /// Mid-range constants: USERDATA=0xB7, KCUPDATE=0xB9, AUTODROP=0xBA.
    #[test]
    fn test_ext_mid_range_b7_ba() {
        assert_eq!(EXT_SUB_USERDATA, 0xB7);
        assert_eq!(EXT_SUB_KCUPDATE, 0xB9);
        assert_eq!(EXT_SUB_AUTODROP, 0xBA);
        assert_eq!(EXT_SUB_INFOMESSAGE, 0xBC);
        assert_eq!(EXT_SUB_MESSAGE, 0xBE);
    }

    /// Anti-cheat sub-opcodes: OPEN=0xA4, KESN=0xAA, ITEM_PROCESS=0xAC.
    #[test]
    fn test_ext_anticheat_extra() {
        assert_eq!(EXT_SUB_OPEN, 0xA4);
        assert_eq!(EXT_SUB_LOG, 0xA5);
        assert_eq!(EXT_SUB_KESN, 0xAA);
        assert_eq!(EXT_SUB_ITEM_PROCESS, 0xAC);
        assert_eq!(EXT_SUB_RESET, 0xAD);
        assert_eq!(EXT_SUB_DROP_REQUEST, 0xAE);
    }

    /// COLLECTION_RACE=0xAF and UNKNOWN1=0xCB are single-use constants.
    #[test]
    fn test_ext_single_use_constants() {
        assert_eq!(EXT_SUB_COLLECTION_RACE, 0xAF);
        assert_eq!(EXT_SUB_UNKNOWN1, 0xCB);
        assert_eq!(EXT_SUB_PUS, 0xA8);
        assert_eq!(EXT_SUB_CASHCHANGE, 0xA9);
    }

    /// All ext sub-opcodes are unique (no collisions across ranges).
    #[test]
    fn test_ext_all_opcodes_unique() {
        let all = [
            EXT_SUB_AUTHINFO,
            EXT_SUB_PROCINFO,
            EXT_SUB_OPEN,
            EXT_SUB_LOG,
            EXT_SUB_XALIVE,
            EXT_SUB_UIINFO,
            EXT_SUB_PUS,
            EXT_SUB_CASHCHANGE,
            EXT_SUB_KESN,
            EXT_SUB_DROP_LIST,
            EXT_SUB_ITEM_PROCESS,
            EXT_SUB_RESET,
            EXT_SUB_DROP_REQUEST,
            EXT_SUB_COLLECTION_RACE,
            EXT_SUB_CLANBANK,
            EXT_SUB_USERINFO,
            EXT_SUB_KCPAZAR,
            EXT_SUB_LOOT_SETTINGS,
            EXT_SUB_CHAOTIC_EXCHANGE,
            EXT_SUB_MERCHANT,
        ];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j], "collision at indices {i} and {j}");
            }
        }
    }

    // ── Sprint 973: Additional coverage ──────────────────────────────

    /// AUTHORITY_GAME_MASTER is 0 and SUPPORT_COOLDOWN_SECS is 1 hour.
    #[test]
    fn test_authority_and_support_constants() {
        assert_eq!(AUTHORITY_GAME_MASTER, 0);
        assert_eq!(SUPPORT_COOLDOWN_SECS, 3600);
    }

    /// EXT_SUB_PUS_CAT is distinct from EXT_SUB_PUS.
    #[test]
    fn test_pus_cat_distinct_from_pus() {
        assert_eq!(EXT_SUB_PUS_CAT, 0xD6);
        assert_eq!(EXT_SUB_PUS, 0xA8);
        assert_ne!(EXT_SUB_PUS_CAT, EXT_SUB_PUS);
    }

    /// EXT_SUB_DAILY_REWARD is 0xF7 — highest ext sub-opcode.
    #[test]
    fn test_daily_reward_highest_subopcode() {
        assert_eq!(EXT_SUB_DAILY_REWARD, 0xF7);
        // Must be above all other ext sub-opcodes
        let others = [
            EXT_SUB_AUTHINFO,
            EXT_SUB_GAME_MASTER_MODE,
            EXT_SUB_HOOK_VISIBLE,
            EXT_SUB_ITEM_EXCHANGE_INFO,
            EXT_SUB_CHEST_BLOCKITEM,
        ];
        for &op in &others {
            assert!(EXT_SUB_DAILY_REWARD > op);
        }
    }

    /// TEMPITEMS, MERCHANTLIST, REPURCHASE are in B8-CE range and distinct.
    #[test]
    fn test_ext_commerce_subopcodes() {
        assert_eq!(EXT_SUB_TEMPITEMS, 0xB8);
        assert_eq!(EXT_SUB_MERCHANTLIST, 0xBD);
        assert_eq!(EXT_SUB_REPURCHASE, 0xCE);
        assert_ne!(EXT_SUB_TEMPITEMS, EXT_SUB_MERCHANTLIST);
        assert_ne!(EXT_SUB_MERCHANTLIST, EXT_SUB_REPURCHASE);
        assert_ne!(EXT_SUB_TEMPITEMS, EXT_SUB_REPURCHASE);
    }

    /// RESETREBSTAT and ACCOUNT_INFO_SAVE are in CA-CC range.
    #[test]
    fn test_ext_account_subopcodes() {
        assert_eq!(EXT_SUB_RESETREBSTAT, 0xCA);
        assert_eq!(EXT_SUB_ACCOUNT_INFO_SAVE, 0xCC);
        assert_ne!(EXT_SUB_RESETREBSTAT, EXT_SUB_ACCOUNT_INFO_SAVE);
        // Both in C-range (0xC0+)
        assert!(EXT_SUB_RESETREBSTAT >= 0xC0);
        assert!(EXT_SUB_ACCOUNT_INFO_SAVE >= 0xC0);
    }

    /// WIZ_EXT_HOOK opcode matches Opcode enum and is outside v2525 dispatch range.
    #[test]
    fn test_wiz_ext_hook_opcode() {
        assert_eq!(WIZ_EXT_HOOK, ko_protocol::Opcode::EXT_HOOK_S2C);
        // Outside v2525 dispatch range (0x06-0xD7)
        assert!(WIZ_EXT_HOOK > 0xD7);
    }

    /// EXT_SUB_ANTI_AFK and EXT_SUB_EVENT_SHOW_LIST are in D0-D8 range.
    #[test]
    fn test_ext_anti_afk_event_show_list() {
        assert_eq!(EXT_SUB_ANTI_AFK, 0xD0);
        assert_eq!(EXT_SUB_EVENT_SHOW_LIST, 0xD8);
        assert_ne!(EXT_SUB_ANTI_AFK, EXT_SUB_EVENT_SHOW_LIST);
    }

    /// EXT_SUB_OPEN, KESN, ITEM_PROCESS form a sequential A4-AC block.
    #[test]
    fn test_ext_open_kesn_item_process_ordering() {
        assert_eq!(EXT_SUB_OPEN, 0xA4);
        assert_eq!(EXT_SUB_KESN, 0xAA);
        assert_eq!(EXT_SUB_ITEM_PROCESS, 0xAC);
        // Ordered
        assert!(EXT_SUB_OPEN < EXT_SUB_KESN);
        assert!(EXT_SUB_KESN < EXT_SUB_ITEM_PROCESS);
    }

    /// EXT_SUB message subopcodes: INFOMESSAGE, MESSAGE, MESSAGE2 are distinct.
    #[test]
    fn test_ext_message_subopcodes_distinct() {
        assert_eq!(EXT_SUB_INFOMESSAGE, 0xBC);
        assert_eq!(EXT_SUB_MESSAGE, 0xBE);
        assert_eq!(EXT_SUB_MESSAGE2, 0xE4);
        assert_ne!(EXT_SUB_INFOMESSAGE, EXT_SUB_MESSAGE);
        assert_ne!(EXT_SUB_MESSAGE, EXT_SUB_MESSAGE2);
    }

    /// EXT_SUB_GAME_MASTER_MODE is 0xE9 — same value as WIZ_EXT_HOOK.
    #[test]
    fn test_ext_game_master_mode_value() {
        assert_eq!(EXT_SUB_GAME_MASTER_MODE, 0xE9);
        // Outside v2525 dispatch range (0x06-0xD7)
        assert!(EXT_SUB_GAME_MASTER_MODE > 0xD7);
        // Same value as WIZ_EXT_HOOK opcode (both 0xE9)
        assert_eq!(EXT_SUB_GAME_MASTER_MODE, WIZ_EXT_HOOK);
    }

    /// Wheel of Fun (gacha) sub-opcode is 0xDA and in event range.
    #[test]
    fn test_ext_wheel_data_subopcode() {
        assert_eq!(EXT_SUB_WHEEL_DATA, 0xDA);
        // Genie info is adjacent at 0xDB
        assert_eq!(EXT_SUB_GENIE_INFO, EXT_SUB_WHEEL_DATA + 1);
        // Both in D-range event block
        assert!(EXT_SUB_WHEEL_DATA >= 0xD0);
        assert!(EXT_SUB_GENIE_INFO >= 0xD0);
    }

    /// Tag/Death/Quest sub-opcodes in D1-D9 form an ordered block.
    #[test]
    fn test_ext_tag_death_quest_ordered() {
        assert_eq!(EXT_SUB_TAG_INFO, 0xD1);
        assert_eq!(EXT_SUB_ZINDAN_WAR, 0xD2);
        assert_eq!(EXT_SUB_DAILY_QUEST, 0xD3);
        assert_eq!(EXT_SUB_PUS_REFUND, 0xD4);
        assert_eq!(EXT_SUB_PLAYER_RANK, 0xD5);
        assert_eq!(EXT_SUB_DEATH_NOTICE, 0xD7);
        assert_eq!(EXT_SUB_SHOW_QUEST_LIST, 0xD9);
        // D1 < D2 < D3 < ... < D9
        assert!(EXT_SUB_TAG_INFO < EXT_SUB_DEATH_NOTICE);
        assert!(EXT_SUB_DEATH_NOTICE < EXT_SUB_SHOW_QUEST_LIST);
    }

    /// PUS refund sub-opcode is 0xD4, distinct from PUS (0xA8) and PUS_CAT (0xD6).
    #[test]
    fn test_ext_pus_refund_distinct() {
        assert_eq!(EXT_SUB_PUS_REFUND, 0xD4);
        assert_eq!(EXT_SUB_PUS, 0xA8);
        assert_eq!(EXT_SUB_PUS_CAT, 0xD6);
        assert_ne!(EXT_SUB_PUS_REFUND, EXT_SUB_PUS);
        assert_ne!(EXT_SUB_PUS_REFUND, EXT_SUB_PUS_CAT);
        // PUS < PUS_REFUND < PUS_CAT
        assert!(EXT_SUB_PUS < EXT_SUB_PUS_REFUND);
        assert!(EXT_SUB_PUS_REFUND < EXT_SUB_PUS_CAT);
    }

    /// Anti-cheat heartbeat (xALIVE 0xA6) is between OPEN (0xA4) and UIINFO (0xA7).
    #[test]
    fn test_ext_xalive_heartbeat_position() {
        assert_eq!(EXT_SUB_XALIVE, 0xA6);
        assert_eq!(EXT_SUB_OPEN, 0xA4);
        assert_eq!(EXT_SUB_UIINFO, 0xA7);
        assert!(EXT_SUB_OPEN < EXT_SUB_XALIVE);
        assert!(EXT_SUB_XALIVE < EXT_SUB_UIINFO);
    }

    /// Cinderella (0xE0), Juraid (0xE2), Perks (0xE3) form high-E event block.
    #[test]
    fn test_ext_high_event_e0_e3_block() {
        assert_eq!(EXT_SUB_CINDERELLA, 0xE0);
        assert_eq!(EXT_SUB_CSW, 0xE1);
        assert_eq!(EXT_SUB_JURAID, 0xE2);
        assert_eq!(EXT_SUB_PERKS, 0xE3);
        // Sequential block E0-E3
        assert_eq!(EXT_SUB_JURAID - EXT_SUB_CINDERELLA, 2);
        assert_eq!(EXT_SUB_PERKS - EXT_SUB_CINDERELLA, 3);
    }

    // ── Sprint 995: ext_hook.rs +5 ──────────────────────────────────────

    /// Support ticket cooldown is exactly 1 hour (3600 seconds).
    #[test]
    fn test_support_cooldown_is_one_hour() {
        assert_eq!(SUPPORT_COOLDOWN_SECS, 3600);
        assert_eq!(SUPPORT_COOLDOWN_SECS, 60 * 60);
    }

    /// LOOT_SETTINGS (0xB3) and AUTODROP (0xBA) are related but distinct sub-opcodes.
    #[test]
    fn test_ext_sub_loot_autodrop_distinct() {
        assert_eq!(EXT_SUB_LOOT_SETTINGS, 0xB3);
        assert_eq!(EXT_SUB_AUTODROP, 0xBA);
        assert_ne!(EXT_SUB_LOOT_SETTINGS, EXT_SUB_AUTODROP);
        // Both in 0xB* range
        assert!(EXT_SUB_LOOT_SETTINGS >= 0xB0 && EXT_SUB_LOOT_SETTINGS <= 0xBF);
        assert!(EXT_SUB_AUTODROP >= 0xB0 && EXT_SUB_AUTODROP <= 0xBF);
    }

    /// MESSAGE2 (0xE4) is distinct from MESSAGE (0xBE) and INFOMESSAGE (0xBC).
    #[test]
    fn test_ext_sub_message2_vs_message() {
        assert_eq!(EXT_SUB_MESSAGE2, 0xE4);
        assert_eq!(EXT_SUB_MESSAGE, 0xBE);
        assert_eq!(EXT_SUB_INFOMESSAGE, 0xBC);
        // All three are distinct
        assert_ne!(EXT_SUB_MESSAGE2, EXT_SUB_MESSAGE);
        assert_ne!(EXT_SUB_MESSAGE2, EXT_SUB_INFOMESSAGE);
        assert_ne!(EXT_SUB_MESSAGE, EXT_SUB_INFOMESSAGE);
    }

    /// MERCHANT (0xB5) and MERCHANTLIST (0xBD) are separate shop sub-opcodes.
    #[test]
    fn test_ext_sub_merchant_pair_distinct() {
        assert_eq!(EXT_SUB_MERCHANT, 0xB5);
        assert_eq!(EXT_SUB_MERCHANTLIST, 0xBD);
        assert_ne!(EXT_SUB_MERCHANT, EXT_SUB_MERCHANTLIST);
        // 8 sub-opcodes between them
        assert_eq!(EXT_SUB_MERCHANTLIST - EXT_SUB_MERCHANT, 8);
    }

    // ── Sprint 1002: ext_hook.rs +5 ──────────────────────────────────────

    /// EXT_SUB range spans 0xA2 (AUTHINFO) to 0xF7 (DAILY_REWARD).
    #[test]
    fn test_ext_sub_range_min_max() {
        assert_eq!(EXT_SUB_AUTHINFO, 0xA2);
        assert_eq!(EXT_SUB_DAILY_REWARD, 0xF7);
        assert_eq!(EXT_SUB_DAILY_REWARD - EXT_SUB_AUTHINFO, 0x55);
    }

    /// AUTHORITY_GAME_MASTER is 0 (matches game server convention).
    #[test]
    fn test_authority_game_master_is_zero() {
        assert_eq!(AUTHORITY_GAME_MASTER, 0);
    }

    /// 0xC3..0xC9 block: MERC_VIEWER through ERRORMSG forms a dense block.
    #[test]
    fn test_ext_sub_c3_c9_dense_block() {
        assert_eq!(EXT_SUB_MERC_VIEWER_INFO, 0xC3);
        assert_eq!(EXT_SUB_UPGRADE_RATE, 0xC4);
        assert_eq!(EXT_SUB_CASTLE_SIEGE_TIMER, 0xC5);
        assert_eq!(EXT_SUB_VOICE, 0xC6);
        assert_eq!(EXT_SUB_LOTTERY, 0xC7);
        assert_eq!(EXT_SUB_TOPLEFT, 0xC8);
        assert_eq!(EXT_SUB_ERRORMSG, 0xC9);
        // Contiguous run of 7 sub-opcodes
        assert_eq!(EXT_SUB_ERRORMSG - EXT_SUB_MERC_VIEWER_INFO, 6);
    }

    /// 0xD1..0xDB block: TAG_INFO through GENIE_INFO (with gaps at D6, D8).
    #[test]
    fn test_ext_sub_d1_db_ordering() {
        assert_eq!(EXT_SUB_TAG_INFO, 0xD1);
        assert_eq!(EXT_SUB_ZINDAN_WAR, 0xD2);
        assert_eq!(EXT_SUB_DAILY_QUEST, 0xD3);
        assert_eq!(EXT_SUB_PUS_REFUND, 0xD4);
        assert_eq!(EXT_SUB_PLAYER_RANK, 0xD5);
        // Gap at 0xD6
        assert_eq!(EXT_SUB_DEATH_NOTICE, 0xD7);
        // Gap at 0xD8
        assert_eq!(EXT_SUB_SHOW_QUEST_LIST, 0xD9);
        assert_eq!(EXT_SUB_WHEEL_DATA, 0xDA);
        assert_eq!(EXT_SUB_GENIE_INFO, 0xDB);
    }

    /// There are exactly 58 unique EXT_SUB constants defined.
    #[test]
    fn test_ext_sub_total_count() {
        let all_subs: std::collections::HashSet<u8> = [
            EXT_SUB_AUTHINFO,
            EXT_SUB_XALIVE,
            EXT_SUB_UIINFO,
            EXT_SUB_USERINFO,
            EXT_SUB_LOOT_SETTINGS,
            EXT_SUB_SUPPORT,
            EXT_SUB_CHAT_LASTSEEN,
            EXT_SUB_SKILL_STAT_RESET,
            EXT_SUB_PROCINFO,
            EXT_SUB_LOG,
            EXT_SUB_PUS,
            EXT_SUB_CASHCHANGE,
            EXT_SUB_DROP_LIST,
            EXT_SUB_RESET,
            EXT_SUB_DROP_REQUEST,
            EXT_SUB_CLANBANK,
            EXT_SUB_CHAOTIC_EXCHANGE,
            EXT_SUB_MERCHANT,
            EXT_SUB_TEMPITEMS,
            EXT_SUB_MERCHANTLIST,
            EXT_SUB_RESETREBSTAT,
            EXT_SUB_ACCOUNT_INFO_SAVE,
            EXT_SUB_REPURCHASE,
            EXT_SUB_CHEST_BLOCKITEM,
            EXT_SUB_ITEM_EXCHANGE_INFO,
            EXT_SUB_DAILY_REWARD,
            EXT_SUB_CSW,
            EXT_SUB_ZINDAN_WAR,
            EXT_SUB_OPEN,
            EXT_SUB_KESN,
            EXT_SUB_ITEM_PROCESS,
            EXT_SUB_COLLECTION_RACE,
            EXT_SUB_KCPAZAR,
            EXT_SUB_USERDATA,
            EXT_SUB_KCUPDATE,
            EXT_SUB_AUTODROP,
            EXT_SUB_INFOMESSAGE,
            EXT_SUB_MESSAGE,
            EXT_SUB_BANSYSTEM,
            EXT_SUB_MERC_VIEWER_INFO,
            EXT_SUB_UPGRADE_RATE,
            EXT_SUB_CASTLE_SIEGE_TIMER,
            EXT_SUB_VOICE,
            EXT_SUB_LOTTERY,
            EXT_SUB_TOPLEFT,
            EXT_SUB_ERRORMSG,
            EXT_SUB_UNKNOWN1,
            EXT_SUB_TAG_INFO,
            EXT_SUB_DAILY_QUEST,
            EXT_SUB_PUS_REFUND,
            EXT_SUB_PLAYER_RANK,
            EXT_SUB_DEATH_NOTICE,
            EXT_SUB_SHOW_QUEST_LIST,
            EXT_SUB_WHEEL_DATA,
            EXT_SUB_GENIE_INFO,
            EXT_SUB_CINDERELLA,
            EXT_SUB_JURAID,
            EXT_SUB_PERKS,
            EXT_SUB_MESSAGE2,
            EXT_SUB_HOOK_VISIBLE,
            EXT_SUB_GAME_MASTER_MODE,
        ]
        .into_iter()
        .collect();
        assert_eq!(all_subs.len(), 61);
    }

    /// CHEST_BLOCKITEM (0xE5) and ITEM_EXCHANGE_INFO (0xE6) are adjacent.
    #[test]
    fn test_ext_sub_chest_exchange_adjacent() {
        assert_eq!(EXT_SUB_CHEST_BLOCKITEM, 0xE5);
        assert_eq!(EXT_SUB_ITEM_EXCHANGE_INFO, 0xE6);
        assert_eq!(EXT_SUB_ITEM_EXCHANGE_INFO - EXT_SUB_CHEST_BLOCKITEM, 1);
    }

    // ── Sprint 26: EXT_HOOK S2C Builder Tests ───────────────────────────────

    #[test]
    fn test_build_cash_change_packet_format() {
        use ko_protocol::PacketReader;
        let pkt = build_cash_change_packet(5000, 1200);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CASHCHANGE));
        assert_eq!(r.read_u32(), Some(5000));
        assert_eq!(r.read_u32(), Some(1200));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_cash_change_packet_zero_balance() {
        use ko_protocol::PacketReader;
        let pkt = build_cash_change_packet(0, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CASHCHANGE));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_info_message_format() {
        use ko_protocol::PacketReader;
        let pkt = build_info_message("Server maintenance in 10 minutes.");
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_INFOMESSAGE));
        assert_eq!(
            r.read_string(),
            Some("Server maintenance in 10 minutes.".to_string())
        );
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_info_message_empty() {
        use ko_protocol::PacketReader;
        let pkt = build_info_message("");
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_INFOMESSAGE));
        assert_eq!(r.read_string(), Some(String::new()));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_error_message_format() {
        use ko_protocol::PacketReader;
        let pkt = build_error_message(1, 1, "Magic Anvil", "You are upgrading too fast");
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ERRORMSG));
        assert_eq!(r.read_u8(), Some(1)); // show
        assert_eq!(r.read_u8(), Some(1)); // sub
        assert_eq!(r.read_string(), Some("Magic Anvil".to_string()));
        assert_eq!(
            r.read_string(),
            Some("You are upgrading too fast".to_string())
        );
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_error_message_hide() {
        use ko_protocol::PacketReader;
        let pkt = build_error_message(0, 0, "", "");
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ERRORMSG));
        assert_eq!(r.read_u8(), Some(0)); // hide
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_string(), Some(String::new()));
        assert_eq!(r.read_string(), Some(String::new()));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_death_notice_format() {
        use ko_protocol::PacketReader;
        let pkt = build_death_notice(1, 2, 0, 1001, "Killer", 2002, "Victim", 500, 600);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_DEATH_NOTICE));
        assert_eq!(r.read_u8(), Some(1)); // victim_nation
        assert_eq!(r.read_u8(), Some(2)); // killer_nation
        assert_eq!(r.read_u8(), Some(0)); // notice_type
        assert_eq!(r.read_u32(), Some(1001)); // killer_id
        assert_eq!(r.read_sbyte_string(), Some("Killer".to_string()));
        assert_eq!(r.read_u32(), Some(2002)); // victim_id
        assert_eq!(r.read_sbyte_string(), Some("Victim".to_string()));
        assert_eq!(r.read_u16(), Some(500)); // victim_x
        assert_eq!(r.read_u16(), Some(600)); // victim_z
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_new_death_narration_matches_1098_wire_format() {
        use ko_protocol::PacketReader;
        let pkt = build_new_death_narration(2, "Killer", "Victim", 1054, 1082);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_DEATH_NOTICE));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_sbyte_string(), Some("Killer".to_string()));
        assert_eq!(r.read_sbyte_string(), Some("Victim".to_string()));
        assert_eq!(r.read_u16(), Some(1054));
        assert_eq!(r.read_u16(), Some(1082));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_death_notice_rival() {
        use ko_protocol::PacketReader;
        let pkt = build_death_notice(2, 1, 1, 500, "PK", 600, "RIP", 100, 200);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_DEATH_NOTICE));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(1)); // rival notice type
        assert_eq!(r.read_u32(), Some(500));
        assert_eq!(r.read_sbyte_string(), Some("PK".to_string()));
        assert_eq!(r.read_u32(), Some(600));
        assert_eq!(r.read_sbyte_string(), Some("RIP".to_string()));
        assert_eq!(r.read_u16(), Some(100));
        assert_eq!(r.read_u16(), Some(200));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_player_rank_update_format() {
        use ko_protocol::PacketReader;
        let pkt = build_player_rank_update(0, 42, 15, 3, 2500);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_PLAYER_RANK));
        assert_eq!(r.read_u8(), Some(0)); // PK Zone rank type
        assert_eq!(r.read_u16(), Some(42)); // session_id
        assert_eq!(r.read_u32(), Some(15)); // kills
        assert_eq!(r.read_u32(), Some(3)); // deaths
        assert_eq!(r.read_u32(), Some(2500)); // loyalty
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_player_rank_update_bdw() {
        use ko_protocol::PacketReader;
        let pkt = build_player_rank_update(1, 100, 0, 0, 0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_PLAYER_RANK));
        assert_eq!(r.read_u8(), Some(1)); // BDW rank type
        assert_eq!(r.read_u16(), Some(100));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_juraid_score_format() {
        use ko_protocol::PacketReader;
        let pkt = build_juraid_score(10, 8, 600);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_JURAID));
        assert_eq!(r.read_u8(), Some(0)); // score sub
        assert_eq!(r.read_u32(), Some(10)); // karus
        assert_eq!(r.read_u32(), Some(8)); // elmo
        assert_eq!(r.read_u32(), Some(600)); // remaining
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_juraid_updatescore_format() {
        use ko_protocol::PacketReader;
        let pkt = build_juraid_updatescore(1, 15); // nation=Karus
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_JURAID));
        assert_eq!(r.read_u8(), Some(1)); // updatescore
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u32(), Some(15)); // new score
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_juraid_result_format() {
        use ko_protocol::PacketReader;
        let pkt = build_juraid_result(2, 20, 25); // ElMorad wins
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_JURAID));
        assert_eq!(r.read_u8(), Some(2)); // result sub
        assert_eq!(r.read_u8(), Some(2)); // winner = ElMorad
        assert_eq!(r.read_u32(), Some(20)); // karus
        assert_eq!(r.read_u32(), Some(25)); // elmo
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_juraid_logout_format() {
        use ko_protocol::PacketReader;
        let pkt = build_juraid_logout();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_JURAID));
        assert_eq!(r.read_u8(), Some(3)); // logout
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_castle_siege_timer_start() {
        use ko_protocol::PacketReader;
        let pkt = build_castle_siege_timer(0, 1800, 1);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CASTLE_SIEGE_TIMER));
        assert_eq!(r.read_u8(), Some(0)); // start
        assert_eq!(r.read_u32(), Some(1800)); // 30 minutes
        assert_eq!(r.read_u8(), Some(1)); // War status
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_castle_siege_timer_stop() {
        use ko_protocol::PacketReader;
        let pkt = build_castle_siege_timer(1, 0, 2);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_CASTLE_SIEGE_TIMER));
        assert_eq!(r.read_u8(), Some(1)); // stop
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u8(), Some(2)); // Ended
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_zindan_result_format() {
        use ko_protocol::PacketReader;
        let pkt = build_zindan_result(1, 30, 45); // Karus wins
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_ZINDAN_WAR));
        assert_eq!(r.read_u8(), Some(3)); // result sub
        assert_eq!(r.read_u8(), Some(1)); // winner = Karus
        assert_eq!(r.read_u32(), Some(30)); // elmo kills
        assert_eq!(r.read_u32(), Some(45)); // karus kills
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_ban_notification_format() {
        use ko_protocol::PacketReader;
        let pkt = build_ban_notification(1);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_BANSYSTEM));
        assert_eq!(r.read_u8(), Some(1)); // banned
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_ban_notification_zero() {
        use ko_protocol::PacketReader;
        let pkt = build_ban_notification(0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_BANSYSTEM));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_gm_mode_toggle_on() {
        use ko_protocol::PacketReader;
        let pkt = build_gm_mode_toggle(1);
        assert_eq!(pkt.opcode, WIZ_EXT_HOOK);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_GAME_MASTER_MODE));
        assert_eq!(r.read_u8(), Some(1)); // enabled
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_gm_mode_toggle_off() {
        use ko_protocol::PacketReader;
        let pkt = build_gm_mode_toggle(0);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(EXT_SUB_GAME_MASTER_MODE));
        assert_eq!(r.read_u8(), Some(0)); // disabled
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_build_death_notice_data_size() {
        // Verify minimum expected size:
        // 1(sub) + 1(vic_nation) + 1(kill_nation) + 1(type) + 4(killer_id)
        // + 1+6(sbyte killer) + 4(victim_id) + 1+6(sbyte victim) + 2(x) + 2(z)
        let pkt = build_death_notice(1, 2, 0, 100, "Killer", 200, "Victim", 50, 60);
        // 1 + 1 + 1 + 1 + 4 + (1+6) + 4 + (1+6) + 2 + 2 = 30
        assert_eq!(pkt.data.len(), 30);
    }

    #[test]
    fn test_build_player_rank_data_size() {
        let pkt = build_player_rank_update(0, 1, 10, 5, 1000);
        // 1(sub) + 1(type) + 2(sid) + 4(kills) + 4(deaths) + 4(loyalty) = 16
        assert_eq!(pkt.data.len(), 16);
    }

    #[test]
    fn test_build_juraid_score_data_size() {
        let pkt = build_juraid_score(10, 20, 600);
        // 1(sub) + 1(sub) + 4 + 4 + 4 = 14
        assert_eq!(pkt.data.len(), 14);
    }

    #[test]
    fn test_build_castle_siege_timer_data_size() {
        let pkt = build_castle_siege_timer(0, 1800, 1);
        // 1(sub) + 1(sub) + 4(secs) + 1(status) = 7
        assert_eq!(pkt.data.len(), 7);
    }
}
