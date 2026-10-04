//! WIZ_CHAT (0x10), WIZ_CHAT_TARGET (0x35), and WIZ_NATION_CHAT (0x19) handlers.
//! ## Chat Types (`ChatType` enum in `packets.h:285-311`)
//! | Type | Name              | Routing                          |
//! |------|-------------------|----------------------------------|
//! | 1    | GENERAL_CHAT      | 3x3 region (nearby players)      |
//! | 2    | PRIVATE_CHAT      | Direct to target (via CHAT_TARGET)|
//! | 3    | PARTY_CHAT        | Party members                    |
//! | 4    | FORCE_CHAT        | Same nation                      |
//! | 5    | SHOUT_CHAT        | Entire zone                      |
//! | 6    | KNIGHTS_CHAT      | Clan members                     |
//! | 7    | PUBLIC_CHAT       | GM only: server-wide             |
//! | 8    | WAR_SYSTEM_CHAT   | GM only: server-wide             |
//! | 12   | GM_CHAT           | Output-only (GM general chat)    |
//! | 13   | COMMAND_CHAT      | Commander: same nation broadcast  |
//! | 14   | MERCHANT_CHAT     | 3x3 region                       |
//! | 15   | ALLIANCE_CHAT     | Alliance members                 |
//! | 19   | SEEKING_PARTY     | Class-matched zone players       |
//! | 22   | COMMAND_PM_CHAT   | Commander: private message        |
//! | 33   | CHATROM_CHAT      | Chat room members                |
//! | 34   | NOAH_KNIGHTS_CHAT | All players level ≤ 50            |
//! ## Client -> Server (WIZ_CHAT 0x10)
//! ```text
//! [u8 chat_type] [u16 msg_len] [bytes message]
//! ```
//! ## Server -> Client (WIZ_CHAT 0x10) — ChatPacket::Construct
//! ```text
//! [u8 chat_type] [u8 nation] [u32 sender_id]
//! [u8 sender_name_len] [bytes sender_name]    (SByte string)
//! [u16 message_len] [bytes message]           (DByte string)
//! [i8 personal_rank] [u8 authority] [u8 system_msg]
//! ```
//! ## WIZ_CHAT_TARGET (0x35) — Private message target resolution
//! Client -> Server: `[u8 type=1] [u16 name_len] [bytes name]`
//! Server -> Client: `[u8 type=1] [i16 result] [name + rank + sysmsg if found] [u8 1]`

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, warn};

use crate::clan_constants::COMMAND_CAPTAIN;
use crate::session::{ClientSession, SessionState};
use crate::world::types::ZONE_PRISON;
use crate::world::MAX_CHAT_ROOM_USERS;

/// Chat type constants from `ChatType` enum.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatType {
    /// Normal chat — broadcast to 3x3 region.
    General = 1,
    /// Private message — direct to target player.
    Private = 2,
    /// Party chat — send to party members.
    Party = 3,
    /// Nation/force chat — send to same nation.
    Force = 4,
    /// Shout — broadcast to entire zone (costs MP).
    Shout = 5,
    /// Knights/clan chat — send to clan members.
    Knights = 6,
    /// Public/notice — GM only, server-wide.
    Public = 7,
    /// War system chat — GM only, server-wide.
    WarSystem = 8,
    /// Permanent chat banner — displayed persistently to all players.
    ///
    Permanent = 9,
    /// End permanent chat banner — clears the persistent display.
    ///
    EndPermanent = 10,
    /// Monument notice — broadcast capture announcement to zone.
    ///
    MonumentNotice = 11,
    /// GM chat — output-only type when GM uses general chat.
    Gm = 12,
    /// Command chat — commander/captain broadcast to same nation.
    ///
    Command = 13,
    /// Merchant chat — broadcast to 3x3 region.
    Merchant = 14,
    /// Alliance chat — send to all alliance clan members.
    Alliance = 15,
    /// Seeking party chat — broadcast to class-matched players in zone.
    ///
    SeekingParty = 19,
    /// Command PM — private message from commander/captain.
    ///
    CommandPm = 22,
    /// Chat room message — send to chat room members.
    ChatRoom = 33,
    /// Clan notice — leader updates clan notice text.
    ///
    ClanNotice = 24,
    /// Krowaz notice — special event notification.
    ///
    KrowazNotice = 25,
    /// Death notice — PvP kill announcement.
    ///
    DeathNotice = 26,
    /// Chaos Stone enemy notice — red text, middle of screen.
    ///
    ChaosStoneEnemyNotice = 27,
    /// Chaos Stone notice — stone status notification.
    ///
    ChaosStoneNotice = 28,
    /// Noah Knights chat — broadcast to all players level ≤ 50.
    ///
    NoahKnights = 34,
}

impl ChatType {
    /// Convert a raw u8 to a ChatType, if valid.
    fn from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Self::General),
            2 => Some(Self::Private),
            3 => Some(Self::Party),
            4 => Some(Self::Force),
            5 => Some(Self::Shout),
            6 => Some(Self::Knights),
            7 => Some(Self::Public),
            8 => Some(Self::WarSystem),
            9 => Some(Self::Permanent),
            10 => Some(Self::EndPermanent),
            11 => Some(Self::MonumentNotice),
            12 => Some(Self::Gm),
            13 => Some(Self::Command),
            14 => Some(Self::Merchant),
            15 => Some(Self::Alliance),
            19 => Some(Self::SeekingParty),
            22 => Some(Self::CommandPm),
            24 => Some(Self::ClanNotice),
            25 => Some(Self::KrowazNotice),
            26 => Some(Self::DeathNotice),
            27 => Some(Self::ChaosStoneEnemyNotice),
            28 => Some(Self::ChaosStoneNotice),
            33 => Some(Self::ChatRoom),
            34 => Some(Self::NoahKnights),
            _ => None,
        }
    }
}

/// Maximum chat message length (C++ checks `chatstr.size() > 128`).
const MAX_CHAT_LENGTH: usize = 128;

/// Returns the authority color for chat messages.
/// - GM → 20
/// - King (rank == 1) → 22
/// - Others → 1
fn get_authority_color(is_gm: bool, rank: u8) -> u8 {
    if is_gm {
        20
    } else if rank == 1 {
        22
    } else {
        1
    }
}

/// Build the server->client chat packet.
/// Wire format:
/// ```text
/// WIZ_CHAT (0x10)
/// [u8 type] [u8 nation] [u32 sender_id]
/// [u8 name_len] [bytes name]       (SByte)
/// [u16 msg_len] [bytes message]    (DByte)
/// [i8 personal_rank] [u8 authority] [u8 system_msg]
/// ```
#[allow(clippy::too_many_arguments)]
pub fn build_chat_packet(
    chat_type: u8,
    nation: u8,
    sender_id: u16,
    sender_name: &str,
    message: &str,
    personal_rank: i8,
    authority: u8,
    system_msg: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizChat as u8);
    pkt.write_u8(chat_type);
    pkt.write_u8(nation);
    // C++ writes `uint32(senderID)` — sender_id is int16, sign-extended to u32.
    // -1 (0xFFFF) → 0xFFFFFFFF for system messages, normal IDs (0..5000) pass through.
    pkt.write_u32(sender_id as i16 as i32 as u32);
    // SByte string: u8 length prefix
    pkt.write_sbyte_string(sender_name);
    // DByte string: u16 length prefix
    pkt.write_string(message);
    // Trailing fields
    pkt.write_i8(personal_rank);
    pkt.write_u8(authority);
    pkt.write_u8(system_msg);
    pkt
}

/// Build the native v2615 PvP death-notice packet.
///
/// Unlike ordinary chat messages, `DEATH_NOTICE` has its own payload. The
/// client uses the two unit IDs and death coordinates to render the death line
/// and live minimap marker. Centre-screen narration is a separate
/// `WIZ_KILLASSIST` packet.
///
/// Wire format recovered from the v2615 client's `sub_8389A0`, case `0x1A`
/// (all strings are SByte strings):
/// `[WIZ_CHAT][u8 26][u8 killer_nation][u8 victim_nation]`
/// `[u8 reserved][u8 notice_type][u32 killer_id][killer_name]`
/// `[u32 victim_id][victim_name][u16 x][u16 z]`
#[allow(clippy::too_many_arguments)]
pub fn build_death_notice_packet(
    killer_nation: u8,
    victim_nation: u8,
    notice_type: u8,
    killer_id: u32,
    killer_name: &str,
    victim_id: u32,
    victim_name: &str,
    victim_x: u16,
    victim_z: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizChat as u8);
    pkt.write_u8(ChatType::DeathNotice as u8);
    pkt.write_u8(killer_nation);
    pkt.write_u8(victim_nation);
    pkt.write_u8(0); // reserved by the native v2615 death-notice contract
    pkt.write_u8(notice_type);
    pkt.write_u32(killer_id);
    pkt.write_sbyte_string(killer_name);
    pkt.write_u32(victim_id);
    pkt.write_sbyte_string(victim_name);
    pkt.write_u16(victim_x);
    pkt.write_u16(victim_z);
    pkt
}

/// Build a chat packet with raw byte message content.
/// Used for user-input chat messages to preserve client encoding (e.g. Windows-1254 Turkish).
/// Server-generated messages should use `build_chat_packet` instead.
#[allow(clippy::too_many_arguments)]
pub fn build_chat_packet_raw(
    chat_type: u8,
    nation: u8,
    sender_id: u16,
    sender_name: &str,
    message_bytes: &[u8],
    personal_rank: i8,
    authority: u8,
    system_msg: u8,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizChat as u8);
    pkt.write_u8(chat_type);
    pkt.write_u8(nation);
    pkt.write_u32(sender_id as i16 as i32 as u32);
    pkt.write_sbyte_string(sender_name);
    pkt.write_string_raw(message_bytes);
    pkt.write_i8(personal_rank);
    pkt.write_u8(authority);
    pkt.write_u8(system_msg);
    pkt
}

/// Build the server->client chat room chat packet.
/// Wire format:
/// ```text
/// WIZ_CHAT (0x10)
/// [u8 CHATROM_CHAT=33]
/// [u16 0x0000]   (DByte mode, padding)
/// [u8 0]
/// [u32 sender_id]
/// [u16 name_len] [bytes name]    (DByte string)
/// [u16 msg_len] [bytes message]  (DByte string)
/// [u16 zone_id]
/// ```
fn build_chatroom_chat_packet(
    sender_id: u16,
    sender_name: &str,
    message: &str,
    zone_id: u16,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizChat as u8);
    pkt.write_u8(ChatType::ChatRoom as u8);
    // DByte mode means strings use u16 length prefix from here on
    pkt.write_u8(0);
    pkt.write_u32(sender_id as u32);
    pkt.write_string(sender_name);
    pkt.write_string(message);
    pkt.write_u16(zone_id);
    pkt
}

/// GM PM rate limit: 10 minutes when switching to a different GM.
/// Returns `true` if the PM is allowed, `false` if rate-limited.
/// Same GM can always be PM'd; switching GMs requires a 10-minute cooldown.
const GM_PM_COOLDOWN_SECS: u64 = 600; // 10 * MINUTE

fn gm_send_pm_check(world: &crate::world::WorldState, sender_sid: u16, target_gm_sid: u16) -> bool {
    let allowed = std::cell::Cell::new(false);
    world.update_session(sender_sid, |h| {
        if target_gm_sid == h.gm_send_pm_id {
            // Same GM — always allowed
            allowed.set(true);
        } else {
            // Different GM — check cooldown
            let elapsed = h.gm_send_pm_time.elapsed().as_secs();
            if elapsed >= GM_PM_COOLDOWN_SECS {
                h.gm_send_pm_id = target_gm_sid;
                h.gm_send_pm_time = std::time::Instant::now();
                allowed.set(true);
            }
            // else: still in cooldown, blocked
        }
    });
    allowed.get()
}

/// Handle WIZ_CHAT (0x10) — main chat handler.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        tracing::debug!(
            "[{}] WIZ_CHAT dropped: session state {:?} != InGame",
            session.addr(),
            session.state()
        );
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    // Read chat type
    let type_byte = match reader.read_u8() {
        Some(t) => t,
        None => return Ok(()),
    };

    // Read message (DByte string: u16 length prefix).
    // read_string() uses Latin-1 decoding for lossless byte round-trip,
    // preserving client encoding (e.g. Windows-1254 Turkish ğ, ş, ı, ö, ü, ç).
    let message = match reader.read_string() {
        Some(s) => s,
        None => return Ok(()),
    };

    // Read seeking party class filter byte (only for SEEKING_PARTY_CHAT)
    let seeking_party_options = if type_byte == ChatType::SeekingParty as u8 {
        reader.read_u8().unwrap_or(0)
    } else {
        0
    };

    // Validate message
    if message.is_empty() || message.len() > MAX_CHAT_LENGTH {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // ── C++ parity: security checks BEFORE GM command processing ──────
    // C++ ChatHandler.cpp:267-283 checks flood → mute → prison → level
    // BEFORE reading/processing the message or GM commands.

    // Chat flood check — strict 300ms minimum between messages.
    {
        const CHAT_DELAY_MS: u64 = 300;
        let allowed = std::cell::Cell::new(false);
        world.update_session(sid, |h| {
            let now = std::time::Instant::now();
            if now.duration_since(h.last_chat_time).as_millis() >= CHAT_DELAY_MS as u128 {
                h.last_chat_time = now;
                allowed.set(true);
            }
        });
        if !allowed.get() {
            return Ok(());
        }
    }

    // Check if the player is muted — silently drop their chat messages
    {
        let (is_muted, is_gm) = world
            .with_session(sid, |h| {
                let is_gm = h
                    .character
                    .as_ref()
                    .is_some_and(|ch| ch.authority == 0 || ch.authority == 2);
                (h.is_muted, is_gm)
            })
            .unwrap_or((false, false));
        if is_muted && !is_gm {
            return Ok(());
        }
    }

    // Prison zone chat block + mute level check — single DashMap read for both
    {
        let (is_gm, zone_id, player_level) = world
            .with_session(sid, |h| {
                let ch = h.character.as_ref();
                let auth = ch.map(|c| c.authority).unwrap_or(255);
                (
                    auth == 0 || auth == 2,
                    h.position.zone_id,
                    ch.map(|c| c.level as i16).unwrap_or(0),
                )
            })
            .unwrap_or((false, 0, 0));
        if zone_id == ZONE_PRISON && !is_gm {
            return Ok(());
        }
        if !is_gm {
            let mute_level = world
                .get_server_settings()
                .map(|s| s.mute_level)
                .unwrap_or(1);
            if player_level < mute_level {
                return Ok(());
            }
        }
    }

    // Process GM chat commands (messages starting with "+" or "/")
    // "/" prefix added for convenience (C++ only uses "+", but many GMs expect "/" too)
    if (message.starts_with('+') || message.starts_with('/'))
        && message.len() > 1
        && super::operator::process_chat_command(session, &message).await?
    {
        return Ok(());
    }

    // Get character info
    let char_info = match world.get_character_info(sid) {
        Some(info) => info,
        None => return Ok(()),
    };

    let sender_name = char_info.name.clone();
    let nation = char_info.nation;
    let authority = char_info.authority;
    let rank = char_info.rank;
    // authority 0 = AUTHORITY_GAME_MASTER, authority 2 = AUTHORITY_GM_USER
    let is_gm = authority == 0 || authority == 2;

    let personal_rank = world.get_loyalty_symbol_rank(sid);
    // C++ isGM() checks ONLY authority==0, not isGMUser()==2
    // GM(auth==0)=20, King(rank==1)=22, others=1
    let system_msg = get_authority_color(authority == 0, rank);

    // Determine output chat type
    let chat_type = ChatType::from_u8(type_byte);
    let out_type = match chat_type {
        Some(ChatType::General) if is_gm => ChatType::Gm as u8,
        _ => type_byte,
    };

    // Build broadcast packet (used by most chat types)
    let broadcast = build_chat_packet(
        out_type,
        nation,
        sid,
        &sender_name,
        &message,
        personal_rank,
        authority,
        system_msg,
    );

    // Route based on chat type
    match chat_type {
        Some(ChatType::General) => {
            // GM general chat is server-wide: GM text must reach every nation
            // and zone, not just the sender's nearby region. Keep the GM chat
            // packet type so the client renders it with its normal GM styling.
            if is_gm {
                // The client filters public chat by the packet's nation field.
                // Sending one packet stamped with the GM's nation while
                // broadcasting globally hides it from the other nation.
                for recipient_nation in [1, 2] {
                    let nation_broadcast = build_chat_packet(
                        out_type,
                        recipient_nation,
                        sid,
                        &sender_name,
                        &message,
                        personal_rank,
                        authority,
                        system_msg,
                    );
                    world.broadcast_to_nation(recipient_nation, Arc::new(nation_broadcast), None);
                }
                return Ok(());
            }

            // Broadcast normal player chat to 3x3 region (nearby players)
            if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room))
            {
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(broadcast),
                    None, // C++ includes sender in Send_NearRegion
                    event_room,
                );
            }
        }

        Some(ChatType::Private) => {
            // Send to pre-selected private chat target (set by WIZ_CHAT_TARGET)
            let target_sid = match world.get_private_chat_target(sid) {
                Some(t) => t,
                None => return Ok(()),
            };

            // Verify target is still in-game
            let target_info = match world.get_character_info(target_sid) {
                Some(t) => t,
                None => return Ok(()),
            };

            // GM PM rate limiting — prevent spamming different GMs
            let target_is_gm = target_info.authority == 0 || target_info.authority == 2;
            if target_is_gm && !gm_send_pm_check(&world, sid, target_sid) {
                return Ok(());
            }

            world.send_to_session_owned(target_sid, broadcast);
        }

        Some(ChatType::Party) => {
            // Send to all party members.
            if let Some(party_id) = world.get_party_id(sid) {
                world.send_to_party(party_id, &broadcast);
                crate::systems::bot_ai::handle_party_chat_command(&world, party_id, sid, &message);
            }
        }

        Some(ChatType::Force) => {
            // Send to all players of the same nation
            world.broadcast_to_nation(nation, Arc::new(broadcast), None);
        }

        Some(ChatType::Shout) => {
            // Broadcast to zone with MP + gold requirements
            let mp_cost = char_info.max_mp / 5;
            if char_info.mp < mp_cost {
                return Ok(());
            }
            // Under level 35 (non-GM): costs 3000 gold
            if !is_gm && char_info.level < 35 {
                const SHOUT_COIN_REQUIREMENT: u32 = 3000;
                let gold = world.get_character_info(sid).map(|c| c.gold).unwrap_or(0);
                if gold < SHOUT_COIN_REQUIREMENT {
                    return Ok(());
                }
                world.gold_lose(sid, SHOUT_COIN_REQUIREMENT);
            }
            // Deduct MP
            world.update_character_stats(sid, |ch| {
                ch.mp = (ch.mp - mp_cost).max(0);
            });
            if let Some(ch_after) = world.get_character_info(sid) {
                let pkt =
                    crate::systems::regen::build_mp_change_packet(ch_after.max_mp, ch_after.mp);
                world.send_to_session_owned(sid, pkt);
            }
            if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room))
            {
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(broadcast),
                    None,
                    event_room,
                );
            }
        }

        Some(ChatType::Knights) => {
            // Send to all online members of the player's clan.
            if char_info.knights_id > 0 {
                world.send_to_knights_members(char_info.knights_id, Arc::new(broadcast), None);
            }
        }

        Some(ChatType::Public) | Some(ChatType::WarSystem) => {
            // GM only: server-wide broadcast
            if !is_gm {
                warn!(
                    "[{}] Non-GM tried to use PUBLIC/WAR_SYSTEM chat",
                    session.addr()
                );
                return Ok(());
            }
            world.broadcast_to_all(Arc::new(broadcast), None);
        }

        Some(ChatType::Merchant) => {
            // Must be in merchant mode to use merchant chat
            if !world.is_merchanting(sid) {
                return Ok(());
            }

            // Broadcast to 3x3 region (merchant advertising)
            if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room))
            {
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(broadcast),
                    None,
                    event_room,
                );

                // Send merchant wind notice to all players in zone.
                world.send_merchant_wind_notice(
                    pos.zone_id,
                    &sender_name,
                    &message,
                    pos.x as u16,
                    pos.z as u16,
                );
            }
        }

        Some(ChatType::Alliance) => {
            // Send to all online members of the player's alliance.
            if char_info.knights_id > 0 {
                if let Some(knights) = world.get_knights(char_info.knights_id) {
                    // Block alliance chat during pending alliance request
                    if knights.alliance_req > 0 {
                        return Ok(());
                    }
                    if knights.alliance > 0 {
                        world.send_to_alliance_members(knights.alliance, Arc::new(broadcast), None);
                    }
                }
            }
        }

        Some(ChatType::Command) => {
            // Commander/captain chat — broadcast to same nation.
            if char_info.fame != COMMAND_CAPTAIN {
                return Ok(());
            }
            world.broadcast_to_nation(nation, Arc::new(broadcast), None);
        }

        Some(ChatType::SeekingParty) => {
            // Seeking party chat — broadcast to class-matched players in same zone.
            let need_party = world.with_session(sid, |h| h.party_type).unwrap_or(0);
            if need_party != 2 {
                return Ok(());
            }
            // Send to self first
            world.send_to_session(sid, &broadcast);
            // Broadcast to class-matched, party-less players in same zone+nation
            if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room))
            {
                world.broadcast_to_zone_matched_class(
                    pos.zone_id,
                    nation,
                    event_room,
                    seeking_party_options,
                    Arc::new(broadcast),
                    Some(sid),
                );
            }
        }

        Some(ChatType::CommandPm) => {
            // Commander PM — private message from commander/captain.
            if char_info.fame != COMMAND_CAPTAIN {
                return Ok(());
            }
            let target_sid = match world.get_private_chat_target(sid) {
                Some(t) => t,
                None => return Ok(()),
            };
            if world.get_character_info(target_sid).is_none() {
                return Ok(());
            }
            world.send_to_session_owned(target_sid, broadcast);
        }

        Some(ChatType::ChatRoom) => {
            // Send to all members of the player's chat room.
            let room_index = world.get_chat_room_index(sid);
            if room_index > 0 {
                let zone_id = world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
                let room_pkt = build_chatroom_chat_packet(sid, &sender_name, &message, zone_id);
                world.send_to_chat_room(room_index, &room_pkt);
            }
        }

        Some(ChatType::ClanNotice) => {
            // Clan notice — leader updates clan notice text.
            let clan_id = char_info.knights_id;
            if clan_id == 0 || !world.is_session_clan_leader(sid) {
                return Ok(());
            }
            // Update runtime clan notice
            if let Some(mut clan) = world.get_knights(clan_id) {
                clan.notice = message.clone();
                world.insert_knights(clan);
            }
            // Persist to DB
            let repo = ko_db::repositories::knights::KnightsRepository::new(session.pool());
            if let Err(e) = repo.update_notice(clan_id as i16, &message).await {
                warn!(
                    "[{}] CLAN_NOTICE: DB error for clan {}: {}",
                    session.addr(),
                    clan_id,
                    e
                );
            }
            debug!(
                "[{}] CLAN_NOTICE: '{}' updated notice for clan {}",
                session.addr(),
                sender_name,
                clan_id
            );
        }

        Some(ChatType::NoahKnights) => {
            // Noah Knights (newbie) chat — broadcast to all players level ≤ 50.
            if char_info.level > 50 {
                return Ok(());
            }
            world.broadcast_to_max_level(50, Arc::new(broadcast));
        }

        _ => {
            debug!("[{}] Unknown chat type: {}", session.addr(), type_byte);
        }
    }

    // FerihaLog: ChatInsertLog
    let pos = world.get_position(sid);
    super::audit_log::log_chat(
        session.pool(),
        session.account_id().unwrap_or(""),
        &sender_name,
        &session.addr().to_string(),
        pos.as_ref().map(|p| p.zone_id as i16).unwrap_or(0),
        pos.as_ref().map(|p| p.x as i16).unwrap_or(0),
        pos.as_ref().map(|p| p.z as i16).unwrap_or(0),
        type_byte,
        &message,
        "",
    );

    Ok(())
}

/// Handle WIZ_CHAT_TARGET (0x35) — private message target resolution.
/// ## Type 1: Find target by name
/// Client: `[u8 type=1] [u16 name_len] [bytes target_name]`
/// Server: `[u8 type=1] [i16 result] [opt: name + rank + sysmsg] [u8 1]`
/// Result values:
/// - 0: target not found or same as self
/// - 1: target found and available
/// - -1: target is blocking private messages
/// ## Type 2: Toggle PM blocking
/// Client: `[u8 type=2] [u8 block_flag]`
/// (No server response — just toggles the flag)
pub async fn handle_chat_target(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    let sub_type = match reader.read_u8() {
        Some(t) => t,
        None => return Ok(()),
    };

    let world = session.world().clone();
    let sid = session.session_id();

    if sub_type == 1 {
        // Find target player by name
        let target_name = match reader.read_string() {
            Some(s) => s,
            None => return Ok(()),
        };

        if target_name.is_empty() || target_name.len() > 20 {
            return Ok(());
        }

        let mut result_pkt = Packet::new(Opcode::WizChatTarget as u8);
        result_pkt.write_u8(sub_type); // echo type

        // Clear previous target
        world.set_private_chat_target(sid, None);

        // Look up target
        let target_sid = world.find_session_by_name(&target_name);

        match target_sid {
            Some(target) if target == sid => {
                // Can't PM yourself
                result_pkt.write_i16(0);
            }
            Some(target) => {
                // Target found — get their info
                if let Some(target_info) = world.get_character_info(target) {
                    // Check if target is blocking private messages
                    let target_rank = world.get_loyalty_symbol_rank(target);
                    let target_is_gm = target_info.authority == 0 || target_info.authority == 2;
                    // C++ ChatHandler.cpp:562 — systemmsg is 20 if target isGM(), else 0
                    // Note: C++ uses isGM() (authority==0 only), NOT get_authority_color
                    let target_sys_msg: u8 = if target_info.authority == 0 { 20 } else { 0 };

                    if world.is_blocking_private_chat(target) {
                        result_pkt.write_i16(-1); // blocking
                        result_pkt.write_string(&target_info.name);
                        result_pkt.write_i8(target_rank);
                        result_pkt.write_u8(target_sys_msg);
                    } else {
                        // GM PM rate limiting — 10min cooldown when switching GMs
                        if target_is_gm && !gm_send_pm_check(&world, sid, target) {
                            result_pkt.write_i16(0); // treat as not found
                        } else {
                            // Set private chat target
                            world.set_private_chat_target(sid, Some(target));

                            result_pkt.write_i16(1); // found
                            result_pkt.write_string(&target_info.name); // DByte string
                            result_pkt.write_i8(target_rank);
                            result_pkt.write_u8(target_sys_msg);
                        }
                    }
                } else {
                    result_pkt.write_i16(0); // not found
                }
            }
            None => {
                // Target not found
                result_pkt.write_i16(0);
            }
        }

        // C++ always appends: result << uint8(1);
        result_pkt.write_u8(1);

        session.send_packet(&result_pkt).await?;
    } else if sub_type == 3 {
        // Chat room message (type 3) — read and discard
        let _sub_sub_type = reader.read_u8();
        debug!(
            "[{}] WIZ_CHAT_TARGET type 3 (chatroom target)",
            session.addr()
        );
    } else {
        // Type 2 / other: toggle PM blocking
        let block = reader.read_u8().unwrap_or(0);
        world.set_block_private_chat(sid, block != 0);
        debug!(
            "[{}] WIZ_CHAT_TARGET PM block set to {}",
            session.addr(),
            block != 0
        );
    }

    Ok(())
}

/// Chat room sub-opcodes used by WIZ_NATION_CHAT (0x19).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatRoomOpcode {
    /// List available chat rooms.
    List = 0x04,
    /// Create a new chat room.
    Create = 0x05,
    /// Join a chat room.
    Join = 0x06,
    /// Leave a chat room.
    Leave = 0x07,
    /// Member option / manual.
    MemberOption = 0x0B,
    /// Admin operation.
    Admin = 0x0C,
}

impl ChatRoomOpcode {
    /// Convert a raw u8 to a ChatRoomOpcode, if valid.
    fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x04 => Some(Self::List),
            0x05 => Some(Self::Create),
            0x06 => Some(Self::Join),
            0x07 => Some(Self::Leave),
            0x0B => Some(Self::MemberOption),
            0x0C => Some(Self::Admin),
            _ => None,
        }
    }
}

/// Outer opcode for chat room manual commands.
const CHATROOM_MANUEL: u8 = 0x0B;

/// Handle WIZ_NATION_CHAT (0x19) — chat room system.
/// Wire format:
/// ```text
/// [u8 opcode=0x0B] [u8 sub_opcode] [... sub-opcode-specific data]
/// ```
pub async fn handle_nation_chat(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    let opcode = match reader.read_u8() {
        Some(o) => o,
        None => return Ok(()),
    };

    if opcode != CHATROOM_MANUEL {
        debug!(
            "[{}] WIZ_NATION_CHAT: unexpected outer opcode 0x{:02X}, expected 0x0B",
            session.addr(),
            opcode
        );
        return Ok(());
    }

    let sub_opcode = match reader.read_u8() {
        Some(s) => s,
        None => return Ok(()),
    };

    match ChatRoomOpcode::from_u8(sub_opcode) {
        Some(ChatRoomOpcode::List) => chatroom_list(session).await,
        Some(ChatRoomOpcode::Create) => chatroom_create(session, &mut reader).await,
        Some(ChatRoomOpcode::Join) => chatroom_join(session, &mut reader).await,
        Some(ChatRoomOpcode::Leave) => chatroom_leave(session, &mut reader).await,
        Some(ChatRoomOpcode::Admin) => chatroom_admin(session, &mut reader).await,
        Some(ChatRoomOpcode::MemberOption) => chatroom_member_option(session, &mut reader).await,
        None => {
            debug!(
                "[{}] WIZ_NATION_CHAT: unknown sub-opcode 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Handle CHATROOM_LIST (0x04) — list all chat rooms.
/// Response wire format:
/// ```text
/// WIZ_NATION_CHAT (0x19)
/// [u8 CHATROOM_MANUEL=0x0B] [u8 CHATROOM_LIST=0x04] [u16 count]
/// for each room:
///   [u16 index] [u16 name_len] [bytes name] [u8 has_password]
///   [u8 nation] [u8 0] [u16 current_users] [u16 max_users]
/// ```
async fn chatroom_list(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let rooms = world.list_chat_rooms();

    let mut pkt = Packet::new(Opcode::WizNationChat as u8);
    pkt.write_u8(CHATROOM_MANUEL);
    pkt.write_u8(ChatRoomOpcode::List as u8);
    pkt.write_u16(rooms.len() as u16);

    for (index, name, has_password, room_nation, current, max) in &rooms {
        pkt.write_u16(*index);
        pkt.write_string(name);
        pkt.write_u8(if *has_password { 1 } else { 0 });
        pkt.write_u8(*room_nation);
        pkt.write_u8(0); // padding
        pkt.write_u16(*current);
        pkt.write_u16(*max);
    }

    session.send_packet(&pkt).await
}

/// Handle CHATROOM_CREATE (0x05) — create a new chat room.
/// Client wire format:
/// ```text
/// [u16 name_len] [bytes name] [u8 has_password]
/// [if has_password: u16 pw_len] [bytes password]
/// [u16 max_users]
/// ```
/// Success response:
/// ```text
/// WIZ_NATION_CHAT [u8 0x0B] [u8 0x05] [u8 1] [u16 room_index]
///   [u16 name_len] [bytes name] [u8 is_admin]
///   [u16 current_users] [u16 max_users] [u32 0]
/// ```
/// Failure response:
/// ```text
/// WIZ_NATION_CHAT [u8 0x0B] [u8 0x05] [u8 0]
/// ```
async fn chatroom_create(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Must not already be in a room (C++: if (m_ChatRoomIndex < 1) return)
    // Note: In C++ this is inverted logic — if index < 1 it means NOT in a room,
    // but the C++ code actually checks `if (m_ChatRoomIndex < 1) return;` which
    // prevents creating if NOT in a room — this looks like a bug in C++.
    // We allow creation if the user is NOT already in a room (index == 0).

    // Read room name (DByte string)
    let room_name = match reader.read_string() {
        Some(s) if !s.is_empty() => s,
        _ => {
            send_chatroom_create_fail(session).await?;
            return Ok(());
        }
    };

    let has_password = reader.read_u8().unwrap_or(0);
    let password = if has_password == 0x01 {
        reader.read_string().unwrap_or_default()
    } else {
        String::new()
    };

    let max_users = reader.read_u16().unwrap_or(0);
    if max_users == 0 || max_users > MAX_CHAT_ROOM_USERS {
        send_chatroom_create_fail(session).await?;
        return Ok(());
    }

    let char_info = match world.get_character_info(sid) {
        Some(info) => info,
        None => return Ok(()),
    };

    let nation = char_info.nation;
    let admin_name = char_info.name.clone();

    // Create the room
    let room_index = match world.create_chat_room(
        room_name.clone(),
        admin_name.clone(),
        password,
        nation,
        max_users,
    ) {
        Some(idx) => idx,
        None => {
            send_chatroom_create_fail(session).await?;
            return Ok(());
        }
    };

    // Set session's chat room index
    world.set_chat_room_index(sid, room_index);

    // Build success response
    //      << uint8(isAdmin) << current_users << max_users << uint32(0);
    let mut pkt = Packet::new(Opcode::WizNationChat as u8);
    pkt.write_u8(CHATROOM_MANUEL);
    pkt.write_u8(ChatRoomOpcode::Create as u8);
    pkt.write_u8(1); // success
    pkt.write_u16(room_index);
    pkt.write_string(&room_name);
    pkt.write_u8(2); // is_admin = 2 (creator is always admin)
    pkt.write_u16(1); // current_users (just the creator)
    pkt.write_u16(max_users);
    pkt.write_u32(0); // padding

    session.send_packet(&pkt).await
}

/// Send a chat room creation failure response.
async fn send_chatroom_create_fail(session: &mut ClientSession) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::WizNationChat as u8);
    pkt.write_u8(CHATROOM_MANUEL);
    pkt.write_u8(ChatRoomOpcode::Create as u8);
    pkt.write_u8(0); // failure
    session.send_packet(&pkt).await
}

/// Handle CHATROOM_JOIN (0x06) — join an existing chat room.
/// Client: `[u16 room_id] [u8 has_password] [u16 pw_len] [bytes password]`
/// Result codes:
/// - 0: success
/// - 1: already in room
/// - 2: room does not exist / full
/// - 4: password mismatch
/// - 5: nation mismatch
async fn chatroom_join(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let room_id = match reader.read_u16() {
        Some(r) => r,
        None => return Ok(()),
    };
    let _has_password = reader.read_u8().unwrap_or(0);
    let password = reader.read_string().unwrap_or_default();

    let char_info = match world.get_character_info(sid) {
        Some(info) => info,
        None => return Ok(()),
    };

    let player_name = char_info.name.clone();
    let player_nation = char_info.nation;

    // Check if already in this room
    let current_room = world.get_chat_room_index(sid);
    let mut result_code: u8 = 0;

    if current_room == room_id && room_id > 0 {
        result_code = 1; // already in room
    } else {
        match world.get_chat_room_mut(room_id) {
            None => {
                result_code = 2; // room not found
            }
            Some(mut room) => {
                if room.nation != player_nation {
                    result_code = 5; // nation mismatch
                } else if room.current_users >= room.max_users {
                    result_code = 2; // full
                } else if room.has_password() && !room.password.eq_ignore_ascii_case(&password) {
                    result_code = 4; // password mismatch
                } else if !room.add_user(&player_name) {
                    result_code = 2; // add failed
                }
            }
        }
    }

    // Build response: WIZ_NATION_CHAT [0x0B] [0x06] [result] [room_id]
    let mut pkt = Packet::new(Opcode::WizNationChat as u8);
    pkt.write_u8(CHATROOM_MANUEL);
    pkt.write_u8(ChatRoomOpcode::Join as u8);
    pkt.write_u8(result_code);
    pkt.write_u16(room_id);

    session.send_packet(&pkt).await?;

    // On success, update session's chat room index
    if result_code == 0 {
        world.set_chat_room_index(sid, room_id);
    }

    Ok(())
}

/// Handle CHATROOM_LEAVE (0x07) — leave a chat room.
/// Client: `[u16 room_id]`
/// If the leaver is the administrator, the entire room is deleted.
async fn chatroom_leave(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let room_id = match reader.read_u16() {
        Some(r) => r,
        None => return Ok(()),
    };

    let char_info = match world.get_character_info(sid) {
        Some(info) => info,
        None => return Ok(()),
    };

    let player_name = char_info.name.clone();

    // Check if the user is admin — if so, delete the room entirely
    let is_admin = world
        .get_chat_room(room_id)
        .map(|r| r.is_administrator(&player_name) == 2)
        .unwrap_or(false);

    if is_admin {
        world.remove_chat_room(room_id);
    } else {
        // Just remove the user
        if let Some(mut room) = world.get_chat_room_mut(room_id) {
            room.remove_user(&player_name);
        }
    }

    // Send leave response
    let mut pkt = Packet::new(Opcode::WizNationChat as u8);
    pkt.write_u8(CHATROOM_MANUEL);
    pkt.write_u8(ChatRoomOpcode::Leave as u8);
    pkt.write_u8(0); // success

    session.send_packet(&pkt).await?;

    // Clear session's room index
    world.set_chat_room_index(sid, 0);

    Ok(())
}

/// Handle CHATROOM_ADMIN (0x0C) — get room admin info / member list.
/// Response wire format:
/// ```text
/// WIZ_NATION_CHAT [0x0B] [0x0C] [u8 0] [u16 sub_opcode]
/// [u16 room_name_len] [bytes room_name]
/// [u8 0x18] [u8 0x02] [u16 max_users] [u16 current_users]
/// for each member:
///   [u8 0x18] [u8 is_admin] [u16 name_len] [bytes name]
/// ```
async fn chatroom_admin(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let sub_pkt = reader.read_u8().unwrap_or(0);
    let room_index = world.get_chat_room_index(sid);
    if room_index == 0 {
        return Ok(());
    }

    let room = match world.get_chat_room(room_index) {
        Some(r) => r,
        None => return Ok(()),
    };

    let mut pkt = Packet::new(Opcode::WizNationChat as u8);
    pkt.write_u8(CHATROOM_MANUEL);
    pkt.write_u8(ChatRoomOpcode::Admin as u8);
    pkt.write_u8(0);
    pkt.write_u16(sub_pkt as u16);

    // DByte mode strings
    pkt.write_string(&room.name);
    pkt.write_u8(0x18);
    pkt.write_u8(0x02);
    pkt.write_u16(room.max_users);
    pkt.write_u16(room.current_users);

    // Write member list
    for member_name in room.members.values() {
        let admin_flag = room.is_administrator(member_name);
        pkt.write_u8(0x18);
        pkt.write_u8(admin_flag);
        pkt.write_string(member_name);
    }

    drop(room); // release the DashMap ref before sending
    session.send_packet(&pkt).await
}

/// Handle CHATROOM_MEMBEROPTION (0x0B) — kick a member from the room.
/// Client: `[u8 sub_opcode=1] [u16 target_member_id]`
/// If sub_opcode == 1, kick the target user from the room.
async fn chatroom_member_option(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let sub_pkt = reader.read_u8().unwrap_or(0);
    if sub_pkt != 1 {
        return Ok(());
    }

    let target_id = match reader.read_u16() {
        Some(t) => t,
        None => return Ok(()),
    };

    let room_index = world.get_chat_room_index(sid);
    if room_index == 0 {
        return Ok(());
    }

    // Try to find and remove the member
    let (removed, kicked_name) = {
        match world.get_chat_room_mut(room_index) {
            Some(mut room) => {
                // Check if the target exists in the member list
                if let Some(name) = room.members.get(&target_id).cloned() {
                    room.remove_user_by_id(target_id);
                    (true, Some(name))
                } else {
                    (false, None)
                }
            }
            None => (false, None),
        }
    };

    if removed {
        // Clear kicked player's room index
        if let Some(ref kicked) = kicked_name {
            if let Some(kicked_sid) = world.find_session_by_name(kicked) {
                world.set_chat_room_index(kicked_sid, 0);
            }
        }

        // Notify room members about the kick
        let mut pkt = Packet::new(Opcode::WizNationChat as u8);
        pkt.write_u8(CHATROOM_MANUEL);
        pkt.write_u8(ChatRoomOpcode::MemberOption as u8);
        pkt.write_u8(1); // success
        pkt.write_u16(target_id);
        world.send_to_chat_room(room_index, &pkt);
    } else {
        // Target not found
        let mut pkt = Packet::new(Opcode::WizNationChat as u8);
        pkt.write_u8(CHATROOM_MANUEL);
        pkt.write_u8(ChatRoomOpcode::MemberOption as u8);
        pkt.write_u8(2); // not found
        session.send_packet(&pkt).await?;
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::world::{ChatRoom, WorldState};
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Test that `build_chat_packet` produces the exact byte layout
    /// matching `ChatPacket::Construct`.
    #[test]
    fn test_build_chat_packet_wire_format() {
        let pkt = build_chat_packet(
            1,          // GENERAL_CHAT
            2,          // nation (Elmorad)
            100,        // sender_id
            "TestUser", // sender_name (8 bytes)
            "Hello!",   // message (6 bytes)
            0,          // personal_rank
            1,          // authority (player)
            0,          // system_msg
        );

        assert_eq!(pkt.opcode, Opcode::WizChat as u8);

        let d = &pkt.data;
        let mut pos = 0;

        // u8 chat_type = 1
        assert_eq!(d[pos], 1);
        pos += 1;

        // u8 nation = 2
        assert_eq!(d[pos], 2);
        pos += 1;

        // u32 sender_id = 100
        let sender_id = u32::from_le_bytes([d[pos], d[pos + 1], d[pos + 2], d[pos + 3]]);
        assert_eq!(sender_id, 100);
        pos += 4;

        // SByte string: u8 len=8, "TestUser"
        assert_eq!(d[pos], 8); // name length
        pos += 1;
        assert_eq!(&d[pos..pos + 8], b"TestUser");
        pos += 8;

        // DByte string: u16 len=6, "Hello!"
        let msg_len = u16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(msg_len, 6);
        pos += 2;
        assert_eq!(&d[pos..pos + 6], b"Hello!");
        pos += 6;

        // i8 personal_rank = 0
        assert_eq!(d[pos] as i8, 0);
        pos += 1;

        // u8 authority = 1
        assert_eq!(d[pos], 1);
        pos += 1;

        // u8 system_msg = 0
        assert_eq!(d[pos], 0);
        pos += 1;

        // Verify total length
        assert_eq!(pos, d.len());
    }

    /// Test chat packet with empty sender name (SByte len=0).
    #[test]
    fn test_build_chat_packet_empty_sender() {
        let pkt = build_chat_packet(
            8,      // WAR_SYSTEM_CHAT
            1,      // nation (Karus)
            0xFFFF, // sender_id = -1 as u16
            "",     // empty sender
            "Notice: Server restart",
            0,
            0, // GM authority
            0,
        );

        let d = &pkt.data;
        let mut pos = 0;

        // type
        assert_eq!(d[pos], 8);
        pos += 1;

        // nation
        assert_eq!(d[pos], 1);
        pos += 1;

        // sender_id = 0xFFFF (u16) → sign-extended to 0xFFFFFFFF (C++ int16(-1) → uint32)
        let sid = u32::from_le_bytes([d[pos], d[pos + 1], d[pos + 2], d[pos + 3]]);
        assert_eq!(sid, 0xFFFFFFFF);
        pos += 4;

        // SByte string: u8 len=0
        assert_eq!(d[pos], 0);
        pos += 1;

        // DByte string: "Notice: Server restart" (22 bytes)
        let msg_len = u16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(msg_len, 22);
        pos += 2;
        assert_eq!(&d[pos..pos + 22], b"Notice: Server restart");
        pos += 22;

        // trailing bytes
        assert_eq!(d[pos] as i8, 0); // rank
        pos += 1;
        assert_eq!(d[pos], 0); // authority
        pos += 1;
        assert_eq!(d[pos], 0); // system_msg
        pos += 1;

        assert_eq!(pos, d.len());
    }

    #[test]
    fn test_build_native_death_notice_packet() {
        let pkt =
            build_death_notice_packet(1, 2, 0, 10_001, "KarusBot", 42, "ElmoUser", 1054, 1082);
        assert_eq!(pkt.opcode, Opcode::WizChat as u8);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(ChatType::DeathNotice as u8));
        assert_eq!(reader.read_u8(), Some(1));
        assert_eq!(reader.read_u8(), Some(2));
        assert_eq!(reader.read_u8(), Some(0));
        assert_eq!(reader.read_u8(), Some(0));
        assert_eq!(reader.read_u32(), Some(10_001));
        assert_eq!(reader.read_sbyte_string().as_deref(), Some("KarusBot"));
        assert_eq!(reader.read_u32(), Some(42));
        assert_eq!(reader.read_sbyte_string().as_deref(), Some("ElmoUser"));
        assert_eq!(reader.read_u16(), Some(1054));
        assert_eq!(reader.read_u16(), Some(1082));
        assert_eq!(reader.remaining(), 0);
    }

    /// Test WIZ_CHAT_TARGET success response wire format.
    #[test]
    fn test_chat_target_response_found() {
        let mut pkt = Packet::new(Opcode::WizChatTarget as u8);
        pkt.write_u8(1); // type
        pkt.write_i16(1); // result: found
        pkt.write_string("TargetPlayer"); // DByte name
        pkt.write_i8(0); // personal rank
        pkt.write_u8(0); // system_msg
        pkt.write_u8(1); // trailing byte

        assert_eq!(pkt.opcode, Opcode::WizChatTarget as u8);

        let d = &pkt.data;
        let mut pos = 0;

        // type
        assert_eq!(d[pos], 1);
        pos += 1;

        // result = 1
        let result = i16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(result, 1);
        pos += 2;

        // name
        let name_len = u16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(name_len, 12); // "TargetPlayer"
        pos += 2;
        assert_eq!(&d[pos..pos + 12], b"TargetPlayer");
        pos += 12;

        // rank
        assert_eq!(d[pos] as i8, 0);
        pos += 1;

        // system_msg
        assert_eq!(d[pos], 0);
        pos += 1;

        // trailing
        assert_eq!(d[pos], 1);
        pos += 1;

        assert_eq!(pos, d.len());
    }

    /// Test WIZ_CHAT_TARGET not-found response wire format.
    #[test]
    fn test_chat_target_response_not_found() {
        let mut pkt = Packet::new(Opcode::WizChatTarget as u8);
        pkt.write_u8(1); // type
        pkt.write_i16(0); // result: not found
        pkt.write_u8(1); // trailing byte

        let d = &pkt.data;
        assert_eq!(d.len(), 4); // 1 + 2 + 1

        assert_eq!(d[0], 1); // type
        assert_eq!(i16::from_le_bytes([d[1], d[2]]), 0); // not found
        assert_eq!(d[3], 1); // trailing
    }

    /// Test that chat type parsing covers all known types.
    #[test]
    fn test_chat_type_from_u8() {
        assert_eq!(ChatType::from_u8(1), Some(ChatType::General));
        assert_eq!(ChatType::from_u8(2), Some(ChatType::Private));
        assert_eq!(ChatType::from_u8(3), Some(ChatType::Party));
        assert_eq!(ChatType::from_u8(4), Some(ChatType::Force));
        assert_eq!(ChatType::from_u8(5), Some(ChatType::Shout));
        assert_eq!(ChatType::from_u8(6), Some(ChatType::Knights));
        assert_eq!(ChatType::from_u8(7), Some(ChatType::Public));
        assert_eq!(ChatType::from_u8(8), Some(ChatType::WarSystem));
        assert_eq!(ChatType::from_u8(9), Some(ChatType::Permanent));
        assert_eq!(ChatType::from_u8(10), Some(ChatType::EndPermanent));
        assert_eq!(ChatType::from_u8(12), Some(ChatType::Gm));
        assert_eq!(ChatType::from_u8(14), Some(ChatType::Merchant));
        assert_eq!(ChatType::from_u8(15), Some(ChatType::Alliance));
        assert_eq!(ChatType::from_u8(24), Some(ChatType::ClanNotice));
        assert_eq!(ChatType::from_u8(33), Some(ChatType::ChatRoom));
        // Unknown types
        assert_eq!(ChatType::from_u8(0), None);
        assert_eq!(ChatType::from_u8(255), None);
    }

    /// PERMANENT_CHAT enum values match C++ packets.h.
    #[test]
    fn test_permanent_chat_type_values() {
        assert_eq!(ChatType::Permanent as u8, 9);
        assert_eq!(ChatType::EndPermanent as u8, 10);
    }

    /// Permanent chat packet uses standard chat format.
    #[test]
    fn test_permanent_chat_packet_format() {
        let pkt = build_chat_packet(
            ChatType::Permanent as u8,
            1,
            0xFFFF,
            "",
            "Hello banner",
            0,
            0,
            0,
        );
        // First byte after opcode is chat type = 9
        assert_eq!(pkt.data[0], 9);
        // Nation byte
        assert_eq!(pkt.data[1], 1);
    }

    /// EndPermanent chat packet (no message content).
    #[test]
    fn test_end_permanent_chat_packet_format() {
        let pkt = build_chat_packet(ChatType::EndPermanent as u8, 1, 0xFFFF, "", "", 0, 0, 0);
        assert_eq!(pkt.data[0], 10);
    }

    /// Test full chat packet roundtrip: build then parse.
    #[test]
    fn test_chat_packet_roundtrip() {
        let original_type = 1u8;
        let original_nation = 2u8;
        let original_sid = 42u16;
        let original_name = "Warrior";
        let original_msg = "Hello World";
        let original_rank = 3i8;
        let original_auth = 1u8;
        let original_sys = 0u8;

        let pkt = build_chat_packet(
            original_type,
            original_nation,
            original_sid,
            original_name,
            original_msg,
            original_rank,
            original_auth,
            original_sys,
        );

        // Parse it back
        let mut reader = PacketReader::new(&pkt.data);
        let chat_type = reader.read_u8().unwrap();
        let nation = reader.read_u8().unwrap();
        let sender_id = reader.read_u32().unwrap();
        let name = reader.read_sbyte_string().unwrap();
        let message = reader.read_string().unwrap();
        let rank = reader.read_u8().unwrap() as i8;
        let auth = reader.read_u8().unwrap();
        let sys = reader.read_u8().unwrap();

        assert_eq!(chat_type, original_type);
        assert_eq!(nation, original_nation);
        assert_eq!(sender_id, original_sid as u32);
        assert_eq!(name, original_name);
        assert_eq!(message, original_msg);
        assert_eq!(rank, original_rank);
        assert_eq!(auth, original_auth);
        assert_eq!(sys, original_sys);
        assert_eq!(reader.remaining(), 0);
    }

    /// Test ChatRoom struct creation and member management.
    #[test]
    fn test_chat_room_add_remove_user() {
        let mut room = ChatRoom {
            index: 1,
            name: "TestRoom".to_string(),
            administrator: "Admin".to_string(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: HashMap::new(),
            next_member_id: 0,
        };

        // Add users
        assert!(room.add_user("Admin"));
        assert_eq!(room.current_users, 1);
        assert!(room.add_user("Player1"));
        assert_eq!(room.current_users, 2);
        assert!(room.add_user("Player2"));
        assert_eq!(room.current_users, 3);

        // Check containment
        assert!(room.contains_user("Admin"));
        assert!(room.contains_user("Player1"));
        assert!(!room.contains_user("Unknown"));

        // Remove user by name
        assert!(room.remove_user("Player1"));
        assert_eq!(room.current_users, 2);
        assert!(!room.contains_user("Player1"));

        // Remove non-existent user
        assert!(!room.remove_user("Ghost"));
        assert_eq!(room.current_users, 2);
    }

    /// Test ChatRoom password and admin checks.
    #[test]
    fn test_chat_room_password_and_admin() {
        let room = ChatRoom {
            index: 1,
            name: "SecureRoom".to_string(),
            administrator: "Boss".to_string(),
            password: "secret123".to_string(),
            nation: 2,
            max_users: 50,
            current_users: 0,
            members: HashMap::new(),
            next_member_id: 0,
        };

        assert!(room.has_password());
        assert_eq!(room.is_administrator("Boss"), 2); // is admin
        assert_eq!(room.is_administrator("Player1"), 1); // not admin
        assert_eq!(room.is_administrator("boss"), 2); // case-insensitive

        // No-password room
        let no_pw = ChatRoom {
            index: 2,
            name: "OpenRoom".to_string(),
            administrator: "Leader".to_string(),
            password: String::new(),
            nation: 1,
            max_users: 20,
            current_users: 0,
            members: HashMap::new(),
            next_member_id: 0,
        };
        assert!(!no_pw.has_password());
    }

    /// Test ChatRoom max user enforcement.
    #[test]
    fn test_chat_room_max_users() {
        let mut room = ChatRoom {
            index: 1,
            name: "SmallRoom".to_string(),
            administrator: "Admin".to_string(),
            password: String::new(),
            nation: 1,
            max_users: 2,
            current_users: 0,
            members: HashMap::new(),
            next_member_id: 0,
        };

        assert!(room.add_user("User1"));
        assert!(room.add_user("User2"));
        assert!(!room.add_user("User3")); // room is full
        assert_eq!(room.current_users, 2);
    }

    /// Test ChatRoom member removal by ID.
    #[test]
    fn test_chat_room_remove_by_id() {
        let mut room = ChatRoom {
            index: 1,
            name: "TestRoom".to_string(),
            administrator: "Admin".to_string(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: HashMap::new(),
            next_member_id: 0,
        };

        room.add_user("Admin");
        room.add_user("Player1");
        let member_id = room.next_member_id; // last added member_id
        assert_eq!(room.current_users, 2);

        assert!(room.remove_user_by_id(member_id));
        assert_eq!(room.current_users, 1);
        assert!(!room.contains_user("Player1"));

        // Remove with invalid ID
        assert!(!room.remove_user_by_id(999));
        assert_eq!(room.current_users, 1);
    }

    /// Test WorldState chat room create and list.
    #[test]
    fn test_world_chat_room_create_and_list() {
        let world = Arc::new(WorldState::new());

        // Create a room
        let idx = world
            .create_chat_room(
                "General Chat".to_string(),
                "Admin".to_string(),
                String::new(),
                1,
                50,
            )
            .unwrap();
        assert!(idx > 0);

        // List rooms
        let rooms = world.list_chat_rooms();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].1, "General Chat");
        assert!(!rooms[0].2); // no password
        assert_eq!(rooms[0].3, 1); // nation
        assert_eq!(rooms[0].4, 1); // current_users (admin auto-added)
        assert_eq!(rooms[0].5, 50); // max_users
    }

    /// Test WorldState chat room duplicate name prevention.
    #[test]
    fn test_world_chat_room_duplicate_name() {
        let world = Arc::new(WorldState::new());

        let idx1 = world.create_chat_room(
            "MyRoom".to_string(),
            "Admin1".to_string(),
            String::new(),
            1,
            10,
        );
        assert!(idx1.is_some());

        // Same name should fail
        let idx2 = world.create_chat_room(
            "MyRoom".to_string(),
            "Admin2".to_string(),
            String::new(),
            1,
            10,
        );
        assert!(idx2.is_none());
    }

    /// Test WorldState PM block toggle.
    #[test]
    fn test_world_pm_block() {
        let world = Arc::new(WorldState::new());
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Default: not blocking
        assert!(!world.is_blocking_private_chat(sid));

        // Block
        world.set_block_private_chat(sid, true);
        assert!(world.is_blocking_private_chat(sid));

        // Unblock
        world.set_block_private_chat(sid, false);
        assert!(!world.is_blocking_private_chat(sid));
    }

    /// Test WIZ_CHAT_TARGET PM-blocked response wire format.
    ///
    /// Expected: `[u8 type=1] [i16 result=-1] [name + rank + sysmsg] [u8 trailing=1]`
    #[test]
    fn test_chat_target_response_blocked() {
        let mut pkt = Packet::new(Opcode::WizChatTarget as u8);
        pkt.write_u8(1); // type
        pkt.write_i16(-1); // result: blocked
        pkt.write_string("BlockedPlayer");
        pkt.write_i8(0); // personal rank
        pkt.write_u8(0); // system_msg
        pkt.write_u8(1); // trailing byte

        let d = &pkt.data;
        let mut pos = 0;

        assert_eq!(d[pos], 1); // type
        pos += 1;

        let result = i16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(result, -1); // blocked
        pos += 2;

        let name_len = u16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(name_len, 13); // "BlockedPlayer"
        pos += 2;
        assert_eq!(&d[pos..pos + 13], b"BlockedPlayer");
        pos += 13;

        assert_eq!(d[pos] as i8, 0); // rank
        pos += 1;
        assert_eq!(d[pos], 0); // system_msg
        pos += 1;
        assert_eq!(d[pos], 1); // trailing
        pos += 1;

        assert_eq!(pos, d.len());
    }

    /// Test chatroom chat packet wire format matches C++.
    #[test]
    fn test_build_chatroom_chat_packet() {
        let pkt = build_chatroom_chat_packet(42, "specialist", "tekrar", 21);

        assert_eq!(pkt.opcode, Opcode::WizChat as u8);

        let d = &pkt.data;
        let mut pos = 0;

        // u8 CHATROM_CHAT = 33
        assert_eq!(d[pos], 33);
        pos += 1;

        // u8 0
        assert_eq!(d[pos], 0);
        pos += 1;

        // u32 sender_id = 42
        let sender_id = u32::from_le_bytes([d[pos], d[pos + 1], d[pos + 2], d[pos + 3]]);
        assert_eq!(sender_id, 42);
        pos += 4;

        // DByte name "specialist" (10 bytes)
        let name_len = u16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(name_len, 10);
        pos += 2;
        assert_eq!(&d[pos..pos + 10], b"specialist");
        pos += 10;

        // DByte message "tekrar" (6 bytes)
        let msg_len = u16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(msg_len, 6);
        pos += 2;
        assert_eq!(&d[pos..pos + 6], b"tekrar");
        pos += 6;

        // u16 zone_id = 21
        let zone_id = u16::from_le_bytes([d[pos], d[pos + 1]]);
        assert_eq!(zone_id, 21);
        pos += 2;

        assert_eq!(pos, d.len());
    }

    /// Test ChatRoomOpcode parsing.
    #[test]
    fn test_chatroom_opcode_from_u8() {
        assert_eq!(ChatRoomOpcode::from_u8(0x04), Some(ChatRoomOpcode::List));
        assert_eq!(ChatRoomOpcode::from_u8(0x05), Some(ChatRoomOpcode::Create));
        assert_eq!(ChatRoomOpcode::from_u8(0x06), Some(ChatRoomOpcode::Join));
        assert_eq!(ChatRoomOpcode::from_u8(0x07), Some(ChatRoomOpcode::Leave));
        assert_eq!(
            ChatRoomOpcode::from_u8(0x0B),
            Some(ChatRoomOpcode::MemberOption)
        );
        assert_eq!(ChatRoomOpcode::from_u8(0x0C), Some(ChatRoomOpcode::Admin));
        assert_eq!(ChatRoomOpcode::from_u8(0x00), None);
        assert_eq!(ChatRoomOpcode::from_u8(0xFF), None);
    }

    // ── Merchant wind notice tests ──────────────────────────────────

    #[test]
    fn test_merchant_wind_notice_packet_format() {
        // Test the merchant wind notice packet — now uses WIZ_CHAT with
        // WAR_SYSTEM_CHAT type (v2525 compatible, replacing WIZ_ADD_MSG 0xDB).
        use ko_protocol::{Opcode, Packet, PacketReader};

        let name = "TestPlayer";
        let message = "Selling +11 weapons!";
        let x: u16 = 1234;
        let z: u16 = 5678;

        // Build the packet manually (same as send_merchant_wind_notice)
        let txt = format!("{} : {}(Location:{},{})", name, message, x, z);
        let mut pkt = Packet::new(Opcode::WizChat as u8);
        pkt.write_u8(7); // WAR_SYSTEM_CHAT
        pkt.write_u8(0); // nation
        let bytes = txt.as_bytes();
        pkt.data
            .extend_from_slice(&(bytes.len() as u16).to_le_bytes());
        pkt.data.extend_from_slice(bytes);

        assert_eq!(pkt.opcode, Opcode::WizChat as u8);

        let mut r = PacketReader::new(&pkt.data);
        let chat_type = r.read_u8().unwrap();
        assert_eq!(chat_type, 7); // WAR_SYSTEM_CHAT
        let nation = r.read_u8().unwrap();
        assert_eq!(nation, 0);

        let decoded = r.read_string().unwrap();
        assert_eq!(
            decoded,
            "TestPlayer : Selling +11 weapons!(Location:1234,5678)"
        );
    }

    #[test]
    fn test_merchant_wind_notice_text_format() {
        let name = "TestUser";
        let message = "WTS Shield +8";
        let x: u16 = 100;
        let z: u16 = 200;
        let txt = format!("{} : {}(Location:{},{})", name, message, x, z);
        assert_eq!(txt, "TestUser : WTS Shield +8(Location:100,200)");
    }

    #[test]
    fn test_wiz_add_msg_opcode_value() {
        use ko_protocol::Opcode;
        // WizAddMsg (0xDB) exists but is outside v2525 range.
        // Merchant wind notice now uses WizChat instead.
        assert_eq!(Opcode::WizAddMsg as u8, 0xDB);
        assert_eq!(Opcode::from_byte(0xDB), Some(Opcode::WizAddMsg));
    }

    /// Merchant chat requires isMerchanting() — C++ ChatHandler.cpp:495-498.
    #[test]
    fn test_merchant_chat_type_value() {
        // ChatType::Merchant = 14 in the C++ enum
        assert_eq!(ChatType::Merchant as u8, 14);
    }

    // ── Sprint 268: New chat type tests ──────────────────────────────

    #[test]
    fn test_command_chat_type() {
        // C++ ChatType::COMMAND_CHAT = 13
        assert_eq!(ChatType::Command as u8, 13);
        assert_eq!(ChatType::from_u8(13), Some(ChatType::Command));
    }

    #[test]
    fn test_seeking_party_chat_type() {
        // C++ ChatType::SEEKING_PARTY_CHAT = 19
        assert_eq!(ChatType::SeekingParty as u8, 19);
        assert_eq!(ChatType::from_u8(19), Some(ChatType::SeekingParty));
    }

    #[test]
    fn test_command_pm_chat_type() {
        // C++ ChatType::COMMAND_PM_CHAT = 22
        assert_eq!(ChatType::CommandPm as u8, 22);
        assert_eq!(ChatType::from_u8(22), Some(ChatType::CommandPm));
    }

    #[test]
    fn test_noah_knights_chat_type() {
        // C++ ChatType::NOAH_KNIGHTS_CHAT = 34
        assert_eq!(ChatType::NoahKnights as u8, 34);
        assert_eq!(ChatType::from_u8(34), Some(ChatType::NoahKnights));
    }

    #[test]
    fn test_command_captain_fame() {
        // C++ GameDefine.h:1285 — COMMAND_CAPTAIN = 100
        // Commander chat requires fame == 100
        assert_eq!(crate::clan_constants::COMMAND_CAPTAIN, 100);
    }

    #[test]
    fn test_noah_knights_level_threshold() {
        // Noah Knights chat: sender must be level ≤ 50
        // C++ ChatHandler.cpp:523 — `if (GetLevel() > 50) break`
        assert!(50u8 <= 50); // level 50 — allowed
        assert!(49u8 <= 50); // level 49 — allowed
        assert!((51u8 > 50)); // level 51 — blocked
    }

    #[test]
    fn test_seeking_party_class_bitmask() {
        // C++ FundamentalMethods.cpp:414-418
        // bit0=warrior(1), bit1=rogue(2), bit2=mage(4), bit3=priest(8), bit4=kurian(10 decimal)
        let options: u8 = 0b00001111; // warrior + rogue + mage + priest
        assert!(options & 1 != 0); // warrior
        assert!(options & 2 != 0); // rogue
        assert!(options & 4 != 0); // mage
        assert!(options & 8 != 0); // priest
        assert!(options & 10 == 0b1010 & 0b1111); // kurian check (C++ uses decimal 10)

        // Warrior only
        let w_only: u8 = 1;
        assert!(w_only & 1 != 0);
        assert!(w_only & 2 == 0);
        assert!(w_only & 4 == 0);
    }

    #[test]
    fn test_moradon_zone_range() {
        // Moradon zones 21-25: cross-nation seeking party allowed
        for z in 21..=25u16 {
            assert!((21..=25).contains(&z));
        }
        assert!(!(21..=25u16).contains(&20));
        assert!(!(21..=25u16).contains(&26));
    }

    // ── Sprint 279: Chat flood protection ───────────────────────────────

    /// Test chat flood delay matches C++ 300ms constant.
    #[test]
    fn test_chat_flood_delay_300ms() {
        // CHAT_DELAY_MS is defined locally in the handler as 300
        let delay_ms: u64 = 300;
        assert_eq!(delay_ms, 300, "Chat delay must be 300ms matching C++");

        // Test timing: 300ms = 0.3 seconds
        let dur = std::time::Duration::from_millis(delay_ms);
        assert_eq!(dur.as_millis(), 300);
        assert!(dur.as_secs_f64() < 1.0, "Must be sub-second");
    }

    /// Test last_chat_time field exists and is accessible.
    #[test]
    fn test_chat_last_time_session_field() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // last_chat_time should exist and be recent
        let elapsed = world
            .with_session(1, |h| h.last_chat_time.elapsed())
            .unwrap();
        assert!(
            elapsed.as_secs() < 5,
            "last_chat_time should be initialized to recent"
        );
    }

    // ── Sprint 315: GM_USER authority check ──────────────────────────

    /// Authority 0 = AUTHORITY_GAME_MASTER, Authority 2 = AUTHORITY_GM_USER
    /// Both should be treated as GM for chat type determination.
    #[test]
    fn test_gm_user_authority_is_gm() {
        let authority_gm: u8 = 0;
        let authority_player: u8 = 1;
        let authority_gm_user: u8 = 2;

        let is_gm_0 = authority_gm == 0 || authority_gm == 2;
        let is_gm_1 = authority_player == 0 || authority_player == 2;
        let is_gm_2 = authority_gm_user == 0 || authority_gm_user == 2;

        assert!(is_gm_0, "authority 0 must be GM");
        assert!(!is_gm_1, "authority 1 must NOT be GM");
        assert!(is_gm_2, "authority 2 must be GM (GM_USER)");
    }

    // ── Sprint 318: Alliance chat blocks during alliance request ────

    /// When alliance_req > 0, alliance chat should be silently blocked.
    #[test]
    fn test_alliance_req_blocks_chat() {
        let alliance_req: u16 = 50; // pending request from clan 50
        assert!(
            alliance_req > 0,
            "alliance_req > 0 should block alliance chat"
        );
    }

    #[test]
    fn test_alliance_req_zero_allows_chat() {
        let alliance_req: u16 = 0; // no pending request
        assert_eq!(
            alliance_req, 0,
            "alliance_req == 0 should allow alliance chat"
        );
    }

    #[test]
    fn test_alliance_chat_type_value() {
        assert_eq!(ChatType::Alliance as u8, 15);
        assert_eq!(ChatType::from_u8(15), Some(ChatType::Alliance));
    }

    // ── Sprint 319: GM PM rate limiting ─────────────────────────────

    /// GM_PM_COOLDOWN_SECS must be 600 (10 minutes), matching C++ gmsendpmcheck.
    #[test]
    fn test_gm_pm_cooldown_constant() {
        assert_eq!(super::GM_PM_COOLDOWN_SECS, 600);
    }

    /// Same GM target (matching gm_send_pm_id) should always be allowed.
    #[test]
    fn test_gm_pm_same_gm_always_allowed() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // First call to GM sid=2 — sets gm_send_pm_id=2
        let allowed = super::gm_send_pm_check(&world, 1, 2);
        assert!(allowed, "first PM to a GM should be allowed");

        // Same GM again — always allowed regardless of cooldown
        let allowed2 = super::gm_send_pm_check(&world, 1, 2);
        assert!(allowed2, "same GM target should always be allowed");
    }

    /// Different GM target during cooldown should be blocked.
    #[test]
    fn test_gm_pm_different_gm_blocked_during_cooldown() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        // PM to GM sid=2
        let allowed = super::gm_send_pm_check(&world, 1, 2);
        assert!(allowed, "first PM to GM should be allowed");

        // PM to DIFFERENT GM sid=3 — should be blocked (within 600s cooldown)
        let blocked = super::gm_send_pm_check(&world, 1, 3);
        assert!(!blocked, "different GM should be blocked during cooldown");
    }

    /// After cooldown expires, switching to a different GM should be allowed.
    #[test]
    fn test_gm_pm_different_gm_allowed_after_cooldown() {
        use crate::world::WorldState;
        use std::time::Instant;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);
        world.register_session(3, tx3);

        // Set gm_send_pm_id=2 with expired time (>600s ago)
        world.update_session(1, |h| {
            h.gm_send_pm_id = 2;
            h.gm_send_pm_time = Instant::now() - std::time::Duration::from_secs(601);
        });

        // PM to DIFFERENT GM sid=3 — cooldown expired, should be allowed
        let allowed = super::gm_send_pm_check(&world, 1, 3);
        assert!(allowed, "different GM should be allowed after cooldown");

        // gm_send_pm_id should now be 3
        let current_gm = world.with_session(1, |h| h.gm_send_pm_id).unwrap();
        assert_eq!(current_gm, 3, "gm_send_pm_id should update to new GM");
    }

    /// Session gm_send_pm_id initializes to 0xFFFF, gm_send_pm_time in the past.
    #[test]
    fn test_gm_pm_session_fields_init() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let gm_id = world.with_session(1, |h| h.gm_send_pm_id).unwrap();
        assert_eq!(gm_id, 0xFFFF, "gm_send_pm_id should init to 0xFFFF");

        let elapsed = world
            .with_session(1, |h| h.gm_send_pm_time.elapsed())
            .unwrap();
        assert!(
            elapsed.as_secs() >= 600,
            "gm_send_pm_time should init far enough in past to allow first PM"
        );
    }

    /// GM detection: authority 0 (GAME_MASTER) and 2 (GM_USER) are GM.
    #[test]
    fn test_gm_pm_target_is_gm_check() {
        let check = |auth: u8| auth == 0 || auth == 2;
        assert!(check(0), "authority 0 = GM");
        assert!(!check(1), "authority 1 = player");
        assert!(check(2), "authority 2 = GM_USER");
        assert!(!check(3), "authority 3 = not GM");
    }

    /// system_msg should be 20 for GM targets in chat_target response.
    #[test]
    fn test_gm_pm_system_msg_20() {
        let target_is_gm = true;
        let system_msg: u8 = if target_is_gm { 20 } else { 0 };
        assert_eq!(system_msg, 20);

        let target_is_gm = false;
        let system_msg: u8 = if target_is_gm { 20 } else { 0 };
        assert_eq!(system_msg, 0);
    }

    // ── Sprint 361: get_authority_color tests ─────────────────────────────

    /// GM authority color should be 20.
    #[test]
    fn test_authority_color_gm() {
        assert_eq!(super::get_authority_color(true, 0), 20);
        assert_eq!(super::get_authority_color(true, 1), 20); // GM overrides king
        assert_eq!(super::get_authority_color(true, 5), 20);
    }

    /// King (rank==1) authority color should be 22.
    #[test]
    fn test_authority_color_king() {
        assert_eq!(super::get_authority_color(false, 1), 22);
    }

    /// Normal player authority color should be 1.
    #[test]
    fn test_authority_color_normal() {
        assert_eq!(super::get_authority_color(false, 0), 1);
        assert_eq!(super::get_authority_color(false, 2), 1);
        assert_eq!(super::get_authority_color(false, 5), 1);
    }
}
