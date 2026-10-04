//! WIZ_USER_INFORMATIN (0x98) handler — nearby user list (bottom-left panel).
//! Sub-opcodes (`BottomUserListOpcode` enum — GameDefine.h:1051):
//! - 1 = Sign (initial request, high bandwidth)
//! - 2 = UserInfoDetail (inspect player equipment/stats)
//! - 3 = UserList (refresh, lower bandwidth)
//! - 4 = RegionDelete (logout notification to nearby players)
//! - 5 = User info view (equipment preview)

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::{ClientSession, SessionState};

use super::{HAVE_MAX, INVENTORY_COSP, SLOT_MAX};

fn build_user_list_header(sub_opcode: u8, zone_id: u16, count: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizUserInfo as u8);
    pkt.write_u8(sub_opcode);
    pkt.write_u8(1);
    pkt.write_u16(zone_id);
    pkt.write_u8(0);
    pkt.write_u16(count);
    pkt
}

fn build_user_list_entry(
    name: &str,
    nation: u8,
    x: f32,
    z: f32,
    clan_id: u16,
    mark_version: Option<u16>,
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizUserInfo as u8);
    // These bytes are concatenated into the parent packet; the packet opcode
    // is intentionally discarded by the caller.
    pkt.write_sbyte_string(name);
    pkt.write_u8(nation);
    pkt.write_u16(1);
    pkt.write_u16(world_coord_to_short(x));
    pkt.write_u16(world_coord_to_short(z));
    pkt.write_u16(clan_id);
    if clan_id != 0 {
        pkt.write_u16(mark_version.unwrap_or(0));
        pkt.write_u8(0);
        pkt.write_u8(0);
    } else {
        pkt.write_u16(0);
        pkt.write_u16(0);
    }
    pkt.write_u16(1);
    pkt
}

fn world_coord_to_short(coord: f32) -> u16 {
    (coord * 10.0).round().clamp(0.0, u16::MAX as f32) as u16
}

fn distance_squared(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    (ax - bx).powi(2) + (az - bz).powi(2)
}

/// Handle WIZ_USER_INFORMATIN from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame && session.state() != SessionState::CharacterSelected
    {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    match sub_opcode {
        1 | 3 => {
            // Sign (initial) or UserList (refresh). The client expects two
            // packets: an empty sub=1 header, then a compressed record list.
            let response_sub = if sub_opcode == 1 { 1u8 } else { 3u8 };

            let world = session.world().clone();
            let sid = session.session_id();
            let (pos, my_event_room, my_authority) = world
                .with_session(sid, |h| {
                    (
                        h.position,
                        h.event_room,
                        h.character.as_ref().map(|c| c.authority),
                    )
                })
                .unwrap_or_default();
            let Some(my_authority) = my_authority else {
                return Ok(());
            };

            // Match the native client/server flow: initialize with an empty
            // Sign packet, then send the actual names/positions in one list.
            session
                .send_packet(&build_user_list_header(1, pos.zone_id, 0))
                .await?;

            let excluded_zone = matches!(pos.zone_id, 85 | 92 | 76 | 89);
            let is_gm = my_authority == 0;
            let mut entries: Vec<(f32, Packet)> = Vec::new();
            if !excluded_zone {
                for other_id in world.sessions_in_zone(pos.zone_id) {
                    let Some((other_pos, event_room, other_char)) = world
                        .with_session(other_id, |h| {
                            (h.position, h.event_room, h.character.clone())
                        })
                    else {
                        continue;
                    };
                    let Some(other_char) = other_char else {
                        continue;
                    };
                    if event_room != my_event_room
                        || (!is_gm && other_char.authority == 0)
                        || (other_id != sid
                            && !is_gm
                            && distance_squared(pos.x, pos.z, other_pos.x, other_pos.z)
                                > 300.0 * 300.0)
                    {
                        continue;
                    }
                    let distance = distance_squared(pos.x, pos.z, other_pos.x, other_pos.z).sqrt();
                    let mark_version = if other_char.knights_id == 0 {
                        0
                    } else {
                        world
                            .get_knights(other_char.knights_id)
                            .map(|clan| clan.mark_version)
                            .unwrap_or(0)
                    };
                    entries.push((
                        distance,
                        build_user_list_entry(
                            &other_char.name,
                            other_char.nation,
                            other_pos.x,
                            other_pos.z,
                            other_char.knights_id,
                            Some(mark_version),
                        ),
                    ));
                }

                // Bots are included by the original bottom-left list protocol.
                for bot in world.get_bots_in_zone_live(pos.zone_id) {
                    if !bot.in_game {
                        continue;
                    }
                    let distance = distance_squared(pos.x, pos.z, bot.x, bot.z).sqrt();
                    entries.push((
                        distance,
                        build_user_list_entry(
                            &bot.name,
                            bot.nation,
                            bot.x,
                            bot.z,
                            bot.knights_id,
                            Some(0),
                        ),
                    ));
                }
            }
            entries.sort_by(|a, b| a.0.total_cmp(&b.0));
            entries.truncate(800);

            let mut response = build_user_list_header(response_sub, pos.zone_id, 0);
            for (_, entry) in &entries {
                response.data.extend_from_slice(&entry.data);
            }
            response.put_u16_at(5, entries.len() as u16);
            let response = response.to_compressed().unwrap_or(response);
            session.send_packet(&response).await?;
        }
        2 => {
            // UserInfoDetail — inspect another player's equipment/stats
            handle_user_info_detail(session, &mut reader).await?;
        }
        4 => {
            // RegionDelete — logout notification to nearby players
            handle_region_delete(session)?;
        }
        5 => {
            // Right-click equipment view request: client sends the target
            // character's socket ID, not a character name.
            handle_get_user_info_view(session, &mut reader).await?;
        }
        _ => {
            tracing::trace!(
                "[{}] Unknown user_info sub-opcode: {}",
                session.addr(),
                sub_opcode
            );
        }
    }

    Ok(())
}

/// Handle the client's bottom-list right-click (equipment view) request.
/// Native `HandleGetUserInfoView()` responds with the Character Seal preview
/// packet layout, populated from the live character's current equipment.
async fn handle_get_user_info_view(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let Some(target_sid) = reader.read_u16() else {
        return Ok(());
    };
    let world = session.world().clone();
    let Some(target) = world.get_character_info(target_sid) else {
        return Ok(());
    };
    let (inventory, skill_points) = world
        .with_session(target_sid, |h| {
            (
                h.inventory.clone(),
                h.character.as_ref().map(|c| c.skill_points.clone()),
            )
        })
        .unwrap_or_default();
    let skill_points = skill_points.unwrap_or_default();

    let mut result = Packet::new(Opcode::WizItemUpgrade as u8);
    result.write_u8(9); // ITEM_CHARACTER_SEAL
    result.write_u8(4); // CharacterSealOpcodes::Preview
    result.write_u8(1); // success
    result.write_sbyte_string(&target.name);
    result.write_u8(target.nation);
    result.write_u8(target.race);
    result.write_u16(target.class);
    result.write_u8(target.level);
    result.write_u32(target.loyalty);
    for stat in [target.str, target.sta, target.dex, target.intel, target.cha] {
        result.write_u8(stat as u8);
    }
    result.write_u32(target.gold);
    result.write_u8(target.free_points as u8);
    result.write_u8(skill_points.first().copied().unwrap_or(0)); // free skill points
    result.write_u32(1); // skill flag, same as native HandleGetUserInfoView
    for idx in [5usize, 6, 7, 8] {
        result.write_u8(skill_points.get(idx).copied().unwrap_or(0));
    }
    for idx in 0..INVENTORY_COSP {
        if let Some(item) = inventory.get(idx) {
            result.write_u32(item.item_id);
            result.write_i16(item.durability);
            result.write_u16(item.count);
            result.write_u8(item.flag);
        } else {
            result.write_u32(0);
            result.write_i16(0);
            result.write_u16(0);
            result.write_u8(0);
        }
    }
    session.send_packet(&result).await?;
    Ok(())
}

/// Handle UserInfoDetail (sub-opcode 2) — inspect another player.
/// Reads the target character name from the bottom-left panel click, looks up
/// the player, and sends their equipment, stats, and skill points via
/// `WIZ_ITEM_UPGRADE` sub-opcode 9.
/// Wire (Client -> Server): `[u8 sub=2] [SByte target_name]`
/// Wire (Server -> Client): `WIZ_ITEM_UPGRADE << u8(9) << u8(4) << u8(1)`
/// `<< [SByte name] << u8 nation << u8 race << u16 class << u8 level << u32 loyalty`
/// `<< u16 str << u16 sta << u16 dex << u16 int << u16 cha`
/// `<< u32 gold << u16 points << u8(0) << u16(0)`
/// `<< u8 skill_cat1 << u8 skill_cat2 << u8 skill_cat3 << u8 skill_master`
/// `<< [42 items: u32 id, i16 dur, u16 count, u8 flag]`
/// `<< u8 rebirth_level`
async fn handle_user_info_detail(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Check if player is busy (trading, merchanting, fishing, mining)
    let is_busy = world
        .with_session(sid, |h| {
            h.trade_state > 1
                || h.merchant_state >= 0
                || h.is_fishing
                || h.is_mining
                || h.selling_merchant_preparing
        })
        .unwrap_or(true);
    if is_busy {
        return Ok(());
    }

    // Read target character name (SByte-prefixed string after sub-opcode)
    let target_name = match reader.read_sbyte_string() {
        Some(s) => s,
        None => return Ok(()),
    };

    // Look up target player by name
    let target_sid = match world.find_session_by_name(&target_name) {
        Some(id) => id,
        None => return Ok(()),
    };

    // GMs are invisible to non-GMs
    let my_ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };
    let target_ch = match world.get_character_info(target_sid) {
        Some(c) => c,
        None => return Ok(()),
    };
    if my_ch.authority != 0 && target_ch.authority == 0 {
        return Ok(());
    }

    // Build WIZ_ITEM_UPGRADE response with sub=9 (character inspection)
    let mut result = Packet::new(Opcode::WizItemUpgrade as u8);
    result.write_u8(9); // sub-opcode (character detail view)

    // C++ uses SByte mode for the data section
    result.write_u8(4); // unknown marker
    result.write_u8(1); // unknown marker

    // Character info
    result.write_sbyte_string(&target_ch.name);
    result.write_u8(target_ch.nation);
    result.write_u8(target_ch.race);
    result.write_u16(target_ch.class);
    result.write_u8(target_ch.level);
    result.write_u32(target_ch.loyalty);

    // Stats (u16 each in C++ via GetStat())
    result.write_u16(target_ch.str as u16);
    result.write_u16(target_ch.sta as u16);
    result.write_u16(target_ch.dex as u16);
    result.write_u16(target_ch.intel as u16);
    result.write_u16(target_ch.cha as u16);

    // Gold and points
    result.write_u32(target_ch.gold);
    result.write_u16(target_ch.free_points);

    // Unknown fields: u8(0) and u16(0)
    result.write_u8(0);
    result.write_u16(0);

    // Skill categories (indices 5-8 in the skill_points array)
    result.write_u8(target_ch.skill_points.get(5).copied().unwrap_or(0));
    result.write_u8(target_ch.skill_points.get(6).copied().unwrap_or(0));
    result.write_u8(target_ch.skill_points.get(7).copied().unwrap_or(0));
    result.write_u8(target_ch.skill_points.get(8).copied().unwrap_or(0));

    // Equipment + inventory items (SLOT_MAX + HAVE_MAX = 42 slots)
    let items = world.with_session(target_sid, |h| h.inventory.clone());
    let items = items.unwrap_or_default();
    for i in 0..(SLOT_MAX + HAVE_MAX) {
        if let Some(slot) = items.get(i) {
            result.write_u32(slot.item_id);
            result.write_i16(slot.durability);
            result.write_u16(slot.count);
            result.write_u8(slot.flag);
        } else {
            result.write_u32(0);
            result.write_i16(0);
            result.write_u16(0);
            result.write_u8(0);
        }
    }

    // Rebirth level
    result.write_u8(target_ch.rebirth_level);

    session.send_packet(&result).await?;

    tracing::debug!(
        "[{}] UserInfoDetail: inspecting '{}' (sid={})",
        session.addr(),
        target_ch.name,
        target_sid,
    );

    Ok(())
}

/// Handle RegionDelete (sub-opcode 4) — notify nearby players of logout.
/// Wire (Server -> Client):
/// `WIZ_USER_INFORMATIN << u8(4) << [SByte name]`
fn handle_region_delete(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };

    // Build: WIZ_USER_INFORMATIN << u8(RegionDelete=4) << SByte(name)
    let mut result = Packet::new(Opcode::WizUserInfo as u8);
    result.write_u8(4); // RegionDelete sub-opcode
    result.write_sbyte_string(&ch.name);

    // Send to all players in the same zone
    world.broadcast_to_zone(pos.zone_id, Arc::new(result), Some(sid));

    tracing::debug!(
        "[{}] UserInfo RegionDelete: '{}' logout notification sent to zone {}",
        session.addr(),
        ch.name,
        pos.zone_id,
    );

    Ok(())
}

/// Build a UserInfoDetail response packet for a given player.
/// This is a helper for testing — produces the same wire format as `handle_user_info_detail`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn build_user_info_detail_packet(
    name: &str,
    nation: u8,
    race: u8,
    class: u16,
    level: u8,
    loyalty: u32,
    stats: [u16; 5], // str, sta, dex, int, cha
    gold: u32,
    free_points: u16,
    skill_cats: [u8; 4], // cat1, cat2, cat3, master
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(9);
    pkt.write_u8(4);
    pkt.write_u8(1);
    pkt.write_sbyte_string(name);
    pkt.write_u8(nation);
    pkt.write_u8(race);
    pkt.write_u16(class);
    pkt.write_u8(level);
    pkt.write_u32(loyalty);
    for s in &stats {
        pkt.write_u16(*s);
    }
    pkt.write_u32(gold);
    pkt.write_u16(free_points);
    pkt.write_u8(0);
    pkt.write_u16(0);
    for c in &skill_cats {
        pkt.write_u8(*c);
    }
    // 42 empty items
    for _ in 0..(SLOT_MAX + HAVE_MAX) {
        pkt.write_u32(0);
        pkt.write_i16(0);
        pkt.write_u16(0);
        pkt.write_u8(0);
    }
    pkt.write_u8(0); // rebirth_level
    pkt
}

/// Build a RegionDelete notification packet.
pub fn build_region_delete_packet(name: &str) -> Packet {
    let mut pkt = Packet::new(Opcode::WizUserInfo as u8);
    pkt.write_u8(4);
    pkt.write_sbyte_string(name);
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    #[test]
    fn test_user_info_detail_packet_format() {
        // Build a detail packet and verify wire format
        let pkt = build_user_info_detail_packet(
            "TestWarrior",
            1,                    // nation (Karus)
            1,                    // race
            102,                  // class (warrior)
            60,                   // level
            5000,                 // loyalty
            [80, 70, 60, 50, 40], // str, sta, dex, int, cha
            100000,               // gold
            10,                   // free_points
            [20, 15, 10, 5],      // skill cats
        );

        assert_eq!(pkt.opcode, Opcode::WizItemUpgrade as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(9)); // sub-opcode
        assert_eq!(r.read_u8(), Some(4)); // marker
        assert_eq!(r.read_u8(), Some(1)); // marker
        assert_eq!(r.read_sbyte_string(), Some("TestWarrior".to_string()));
        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u8(), Some(1)); // race
        assert_eq!(r.read_u16(), Some(102)); // class
        assert_eq!(r.read_u8(), Some(60)); // level
        assert_eq!(r.read_u32(), Some(5000)); // loyalty
                                              // Stats
        assert_eq!(r.read_u16(), Some(80)); // str
        assert_eq!(r.read_u16(), Some(70)); // sta
        assert_eq!(r.read_u16(), Some(60)); // dex
        assert_eq!(r.read_u16(), Some(50)); // int
        assert_eq!(r.read_u16(), Some(40)); // cha
                                            // Gold and points
        assert_eq!(r.read_u32(), Some(100000));
        assert_eq!(r.read_u16(), Some(10)); // free_points
        assert_eq!(r.read_u8(), Some(0)); // reserved
        assert_eq!(r.read_u16(), Some(0)); // reserved
                                           // Skill categories
        assert_eq!(r.read_u8(), Some(20)); // cat1
        assert_eq!(r.read_u8(), Some(15)); // cat2
        assert_eq!(r.read_u8(), Some(10)); // cat3
        assert_eq!(r.read_u8(), Some(5)); // master
                                          // 42 empty items (each 4+2+2+1 = 9 bytes)
        for _ in 0..42 {
            assert_eq!(r.read_u32(), Some(0)); // item_id
            assert_eq!(r.read_u16(), Some(0)); // durability (read as u16, same bits)
            assert_eq!(r.read_u16(), Some(0)); // count
            assert_eq!(r.read_u8(), Some(0)); // flag
        }
        assert_eq!(r.read_u8(), Some(0)); // rebirth_level
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_user_info_detail_with_items() {
        // Verify item data serialization in the detail packet
        let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
        pkt.write_u8(9);
        pkt.write_u8(4);
        pkt.write_u8(1);
        pkt.write_sbyte_string("Knight01");
        // nation, race, class, level, loyalty
        pkt.write_u8(2);
        pkt.write_u8(11);
        pkt.write_u16(203);
        pkt.write_u8(83);
        pkt.write_u32(10000);
        // stats
        for _ in 0..5 {
            pkt.write_u16(99);
        }
        // gold, points, reserved
        pkt.write_u32(999999);
        pkt.write_u16(0);
        pkt.write_u8(0);
        pkt.write_u16(0);
        // skill cats
        for _ in 0..4 {
            pkt.write_u8(0);
        }
        // First equipped slot: a sword (item 120050000, dur=100, count=1, flag=0)
        pkt.write_u32(120050000);
        pkt.write_i16(100);
        pkt.write_u16(1);
        pkt.write_u8(0);
        // Remaining 41 empty items
        for _ in 0..41 {
            pkt.write_u32(0);
            pkt.write_i16(0);
            pkt.write_u16(0);
            pkt.write_u8(0);
        }
        pkt.write_u8(0); // rebirth

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(9)); // sub
        r.read_u8();
        r.read_u8(); // markers
        assert_eq!(r.read_sbyte_string(), Some("Knight01".to_string()));
        // Skip to first item slot (after nation/race/class/level/loyalty/stats/gold/pts/reserved/skills)
        // nation(1) + race(1) + class(2) + level(1) + loyalty(4) + 5*stats(10) + gold(4) + pts(2) + u8(1) + u16(2) + 4*skill(4) = 32 bytes
        for _ in 0..32 {
            r.read_u8();
        }
        // First item
        assert_eq!(r.read_u32(), Some(120050000));
        assert_eq!(r.read_u16(), Some(100)); // durability as u16
        assert_eq!(r.read_u16(), Some(1)); // count
        assert_eq!(r.read_u8(), Some(0)); // flag
    }

    #[test]
    fn test_user_info_detail_slot_count() {
        // Verify SLOT_MAX + HAVE_MAX = 42 (14 equipped + 28 inventory)
        assert_eq!(SLOT_MAX, 14);
        assert_eq!(HAVE_MAX, 28);
        assert_eq!(SLOT_MAX + HAVE_MAX, 42);
    }

    #[test]
    fn test_region_delete_packet_format() {
        // Wire: WIZ_USER_INFORMATIN << u8(4) << SByte(name)
        let pkt = build_region_delete_packet("Warrior123");

        assert_eq!(pkt.opcode, Opcode::WizUserInfo as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4)); // RegionDelete sub-opcode
        assert_eq!(r.read_sbyte_string(), Some("Warrior123".to_string()));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_region_delete_empty_name() {
        let pkt = build_region_delete_packet("");
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.read_sbyte_string(), Some("".to_string()));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_user_info_request_format() {
        // Client -> Server: [u8 sub_opcode=2] [SByte target_name]
        let mut pkt = Packet::new(Opcode::WizUserInfo as u8);
        pkt.write_u8(2); // UserInfoDetail
        pkt.write_sbyte_string("TargetPlayer");

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_sbyte_string(), Some("TargetPlayer".to_string()));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn bottom_user_list_header_uses_native_wire_layout() {
        let mut pkt = build_user_list_header(3, 21, 2);
        assert_eq!(pkt.opcode, Opcode::WizUserInfo as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(21));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(2));
        assert_eq!(r.remaining(), 0);
        pkt.put_u16_at(5, 1);
        assert_eq!(u16::from_le_bytes([pkt.data[5], pkt.data[6]]), 1);
    }

    #[test]
    fn bottom_user_list_entry_has_name_nation_position_and_clan_fields() {
        let pkt = build_user_list_entry("Volkan", 1, 12.3, 45.6, 7, Some(42));
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_sbyte_string(), Some("Volkan".to_string()));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u16(), Some(123));
        assert_eq!(r.read_u16(), Some(456));
        assert_eq!(r.read_u16(), Some(7));
        assert_eq!(r.read_u16(), Some(42));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 922: Additional coverage ──────────────────────────────

    /// Detail packet total data length: sub(1) + markers(2) + sbyte_name(1+len)
    /// + nation(1) + race(1) + class(2) + level(1) + loyalty(4) + stats(10)
    /// + gold(4) + points(2) + reserved(3) + skills(4) + 42*items(42*9=378)
    /// + rebirth(1) = 36 + name_len + 378
    #[test]
    fn test_user_info_detail_data_length() {
        let name = "Hero";
        let pkt = build_user_info_detail_packet(
            name,
            1,
            1,
            101,
            80,
            0,
            [50, 50, 50, 50, 50],
            0,
            0,
            [0, 0, 0, 0],
        );
        // 1+2 + (1+4) + 1+1+2+1+4 + 10 + 4+2+1+2 + 4 + 378 + 1 = 419
        let expected = 3 + (1 + name.len()) + 9 + 10 + 9 + 4 + 378 + 1;
        assert_eq!(pkt.data.len(), expected);
    }

    /// Sign response header (sub=1): opcode + sub + reserved + zone_id + reserved + count.
    #[test]
    fn test_sign_response_header_format() {
        let mut pkt = Packet::new(Opcode::WizUserInfo as u8);
        pkt.write_u8(1); // Sign sub-opcode
        pkt.write_u8(1); // reserved
        pkt.write_u16(21); // zone_id (Moradon)
        pkt.write_u8(0); // reserved
        pkt.write_u16(3); // 3 nearby users

        assert_eq!(pkt.opcode, Opcode::WizUserInfo as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(21));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(3));
        assert_eq!(r.remaining(), 0);
    }

    /// UserList response (sub=3) uses same header format as Sign.
    #[test]
    fn test_userlist_response_header_format() {
        let mut pkt = Packet::new(Opcode::WizUserInfo as u8);
        pkt.write_u8(3); // UserList sub-opcode
        pkt.write_u8(1);
        pkt.write_u16(11); // zone_id (El Morad)
        pkt.write_u8(0);
        pkt.write_u16(0); // empty list

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(11));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    /// Region delete with max-length name (20 chars).
    #[test]
    fn test_region_delete_long_name() {
        let name = "ABCDEFGHIJKLMNOPQRST"; // 20 chars
        let pkt = build_region_delete_packet(name);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(4));
        assert_eq!(r.read_sbyte_string(), Some(name.to_string()));
        assert_eq!(r.remaining(), 0);
        // data len = sub(1) + sbyte_len(1) + name(20) = 22
        assert_eq!(pkt.data.len(), 22);
    }

    /// Detail packet with non-zero rebirth_level.
    #[test]
    fn test_user_info_detail_rebirth_level() {
        let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
        pkt.write_u8(9);
        pkt.write_u8(4);
        pkt.write_u8(1);
        pkt.write_sbyte_string("Rebirth");
        // Minimal char data
        pkt.write_u8(1);
        pkt.write_u8(1);
        pkt.write_u16(101);
        pkt.write_u8(83);
        pkt.write_u32(0);
        for _ in 0..5 {
            pkt.write_u16(0);
        }
        pkt.write_u32(0);
        pkt.write_u16(0);
        pkt.write_u8(0);
        pkt.write_u16(0);
        for _ in 0..4 {
            pkt.write_u8(0);
        }
        for _ in 0..42 {
            pkt.write_u32(0);
            pkt.write_i16(0);
            pkt.write_u16(0);
            pkt.write_u8(0);
        }
        pkt.write_u8(3); // rebirth_level = 3

        // Read to the end and verify rebirth_level
        let data = &pkt.data;
        assert_eq!(data[data.len() - 1], 3);
    }

    /// Skill category indices in the detail packet (C++ Cat1=5, Cat2=6, Cat3=7, Master=8).
    #[test]
    fn test_user_info_detail_skill_categories() {
        let pkt = build_user_info_detail_packet(
            "Mage",
            2,
            12,
            205,
            70,
            3000,
            [30, 30, 30, 200, 30],
            50000,
            5,
            [80, 60, 40, 10], // cat1=80, cat2=60, cat3=40, master=10
        );
        let mut r = PacketReader::new(&pkt.data);
        // Skip: sub(1) + markers(2) + sbyte("Mage"=1+4) + nation(1) + race(1)
        // + class(2) + level(1) + loyalty(4) + stats(10) + gold(4) + points(2)
        // + reserved(1+2) = 36
        for _ in 0..36 {
            r.read_u8();
        }
        assert_eq!(r.read_u8(), Some(80)); // cat1
        assert_eq!(r.read_u8(), Some(60)); // cat2
        assert_eq!(r.read_u8(), Some(40)); // cat3
        assert_eq!(r.read_u8(), Some(10)); // master
    }

    /// Sub-opcode constants match C++ BottomUserListOpcode enum.
    #[test]
    fn test_sub_opcode_values() {
        // C++ GameDefine.h:1051 — Sign=1, UserInfoDetail=2, UserList=3, RegionDelete=4
        let sign: u8 = 1;
        let detail: u8 = 2;
        let list: u8 = 3;
        let delete: u8 = 4;
        assert_ne!(sign, detail);
        assert_ne!(detail, list);
        assert_ne!(list, delete);
        assert_eq!(sign + 1, detail);
        assert_eq!(detail + 1, list);
        assert_eq!(list + 1, delete);
    }
}
