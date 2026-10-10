//! Region helpers — INOUT packet building and region change notifications.
//! -  (UserInOut)
//! -  (RegionUserInOutForMe)
//! -  (GetUserInfo)
//! ## WIZ_USER_INOUT (0x07)
//! | Offset | Type   | Description |
//! |--------|--------|-------------|
//! | 0      | u8     | InOutType (1=in, 2=out, 3=respawn, 4=warp, 5=summon) |
//! | 1      | u8     | Reserved (0) |
//! | 2      | u32le  | Session (socket) ID |
//! | 6+     | ...    | GetUserInfo (if type != OUT) |
//! ## WIZ_REGIONCHANGE (0x15) — 3-phase protocol
//! - Phase 0 (start): `[0x15] [u8 0]`
//! - Phase 1 (data):  `[0x15] [u8 1] [u16 count] [u32 ids...]` (compressed)
//! - Phase 2 (end):   `[0x15] [u8 2]`

use ko_protocol::{Opcode, Packet};
use std::sync::Arc;

use crate::npc::{build_npc_inout, NPC_IN};
use crate::session::ClientSession;
use crate::world::{
    BroadcastState, CharacterInfo, KnightsInfo, Position, WorldState, ZONE_BATTLE3,
    ZONE_SPBATTLE_MAX, ZONE_SPBATTLE_MIN,
};
use crate::zone::SessionId;

/// Empty equipment visual data (all zeros) — used when visual data is not available.
pub const EMPTY_EQUIP_VISUAL: [(u32, i16, u8); 17] = [(0, 0, 0); 17];

/// Snapshot of all broadcast-relevant data for a player, fetched in a single DashMap read.
/// Replaces ~22 separate DashMap reads per nearby player in the broadcast loop
/// (get_character_info + get_position + 3 flag reads + 17 inventory reads).
struct BroadcastSnapshot {
    character: Option<CharacterInfo>,
    position: Position,
    invisibility_type: u8,
    abnormal_type: u32,
    broadcast_state: BroadcastState,
    equip_visual: [(u32, i16, u8); 17],
}

/// Fetch all broadcast-relevant data in a single session lock.
fn snapshot_broadcast_info(world: &WorldState, sid: SessionId) -> Option<BroadcastSnapshot> {
    let is_nation_battle_no_b3 = {
        let bs = world.get_battle_state();
        bs.is_nation_battle() && bs.battle_zone_id() != ZONE_BATTLE3
    };
    world.with_session(sid, |h| {
        // Equipped visual — single pass over inventory
        let mut equip = [(0u32, 0i16, 0u8); 17];
        for (i, &inv_slot) in VISUAL_SLOT_ORDER.iter().enumerate() {
            if let Some(item) = h.inventory.get(inv_slot) {
                if item.item_id != 0 {
                    equip[i] = (item.item_id, item.durability, item.flag);
                }
            }
        }
        // Dragon armor override during nation battle
        if is_nation_battle_no_b3 {
            if let Some(ch) = h.character.as_ref() {
                if let Some(items) = dragon_armor_for_class(ch.class) {
                    for (i, &dragon_id) in items.iter().enumerate() {
                        equip[i] = (dragon_id, equip[i].1, equip[i].2);
                    }
                }
            }
        }
        BroadcastSnapshot {
            character: h.character.clone(),
            position: h.position,
            invisibility_type: h.invisibility_type,
            abnormal_type: h.abnormal_type,
            broadcast_state: BroadcastState {
                need_party: h.need_party,
                party_leader: h.party_leader,
                is_devil: if h.is_devil { 1 } else { 0 },
                team_colour: h.team_colour,
                direction: h.direction as u16,
                is_hiding_helmet: if h.is_hiding_helmet { 1 } else { 0 },
                is_hiding_cospre: if h.is_hiding_cospre { 1 } else { 0 },
                knights_rank: if h.knights_rank == 0 {
                    -1
                } else {
                    h.knights_rank as i8
                },
                personal_rank: if h.personal_rank == 0 {
                    -1
                } else {
                    h.personal_rank as i8
                },
                is_in_genie: if h.genie_active { 1 } else { 0 },
                return_symbol_ok: h.return_symbol_ok,
            },
            equip_visual: equip,
        }
    })
}

/// Re-send a player's full appearance to nearby clients after a runtime visual
/// flag changes (for example Genie activation). This is event-driven only; it
/// is not part of the periodic NPC/player refresh path.
pub(crate) fn broadcast_user_appearance_update(world: &WorldState, sid: SessionId) {
    let Some(snap) = snapshot_broadcast_info(world, sid) else {
        return;
    };
    let Some(character) = snap.character.as_ref() else {
        return;
    };
    let clan = (character.knights_id > 0)
        .then(|| world.get_knights(character.knights_id))
        .flatten();
    let cape = clan
        .as_ref()
        .and_then(|clan| resolve_alliance_cape(clan, world));
    let is_king = world.is_king(character.nation, &character.name);
    let packet = build_user_inout_with_clan(
        INOUT_WARP,
        sid,
        Some(character),
        &snap.position,
        clan.as_ref(),
        cape,
        is_king,
        snap.invisibility_type,
        snap.abnormal_type,
        &snap.broadcast_state,
        &snap.equip_visual,
    );
    world.broadcast_to_3x3(
        snap.position.zone_id,
        snap.position.region_x,
        snap.position.region_z,
        Arc::new(packet),
        Some(sid),
        world.with_session(sid, |h| h.event_room).unwrap_or(0),
    );
}

/// Visual broadcast slot order — inventory slot indices for the 17 equipment visual slots.
/// Order: BREAST(4), LEG(10), HEAD(1), GLOVE(12), FOOT(13), SHOULDER(5),
///   RIGHTHAND(6), LEFTHAND(8), CWING(42), CHELMET(43), CLEFT(44), CRIGHT(45),
///   CTOP(46), CTATTOO(49), CFAIRY(48), CEMBLEM(47), CTALISMAN(50)
const VISUAL_SLOT_ORDER: [usize; 17] = [
    4, 10, 1, 12, 13, 5, 6, 8, // equipped slots (0-13)
    42, 43, 44, 45, 46, 49, 48, 47, 50, // cosplay slots (42-50)
];

/// Extract (item_id, durability, flag) for all 17 visual slots from inventory.
/// Returns data in VISUAL_SLOT_ORDER broadcast order. Includes both equipped (0-13)
/// and cosplay (42-50) inventory slots.
/// During nation battle (except ZONE_BATTLE3), the first 5 visual slots (BREAST,
/// LEG, HEAD, GLOVE, FOOT) are replaced with class-specific dragon armor item IDs.
/// Durability and flag are preserved from the original equipped items.
pub fn get_equipped_visual(world: &WorldState, sid: SessionId) -> [(u32, i16, u8); 17] {
    let mut result = [(0u32, 0i16, 0u8); 17];
    for (i, &inv_slot) in VISUAL_SLOT_ORDER.iter().enumerate() {
        if let Some(item) = world.get_inventory_slot(sid, inv_slot) {
            if item.item_id != 0 {
                result[i] = (item.item_id, item.durability, item.flag);
            }
        }
    }

    // Dragon armor transformation during nation battle
    // Condition: battle_open == NATION_BATTLE && battle_zone != ZONE_BATTLE3
    let bs = world.get_battle_state();
    if bs.is_nation_battle() && bs.battle_zone_id() != ZONE_BATTLE3 {
        if let Some(ch) = world.get_character_info(sid) {
            let dragon_items = dragon_armor_for_class(ch.class);
            if let Some(items) = dragon_items {
                // Replace first 5 visual slots (BREAST, LEG, HEAD, GLOVE, FOOT)
                // Only item_id changes — durability and flag are preserved
                for (i, &dragon_id) in items.iter().enumerate() {
                    result[i] = (dragon_id, result[i].1, result[i].2);
                }
            }
        }
    }

    result
}

/// Get dragon armor item IDs for a given class.
/// - Warrior (1,5,6) & Kurian (13,14,15): 507001010–507005010
/// - Rogue (2,7,8): 547001010–547005010
/// - Mage (3,9,10): 567001010–567005010
/// - Priest (4,11,12): 587001018–587005018
fn dragon_armor_for_class(class: u16) -> Option<[u32; 5]> {
    match class % 100 {
        1 | 5 | 6 | 13 | 14 | 15 => {
            // Warrior & Kurian share the same dragon armor
            Some([507001010, 507002010, 507003010, 507004010, 507005010])
        }
        2 | 7 | 8 => {
            // Rogue
            Some([547001010, 547002010, 547003010, 547004010, 547005010])
        }
        3 | 9 | 10 => {
            // Mage
            Some([567001010, 567002010, 567003010, 567004010, 567005010])
        }
        4 | 11 | 12 => {
            // Priest
            Some([587001018, 587002018, 587003018, 587004018, 587005018])
        }
        _ => None,
    }
}

/// INOUT type constants matching the `InOutType` enum.
pub const INOUT_IN: u8 = 1;
pub const INOUT_OUT: u8 = 2;
pub const INOUT_RESPAWN: u8 = 3;
pub const INOUT_WARP: u8 = 4;

/// Zone IDs for PK zone classification.
const ZONE_ARDREAM_PK: u16 = 72;
const ZONE_RONARK_LAND_PK: u16 = 71;
const ZONE_RONARK_LAND_BASE_PK: u16 = 73;

/// Check if a zone is a PK zone.
/// PK zones: Ardream (72), Ronark Land (71), Ronark Land Base (73),
/// and Special Event zones (SPBATTLE 105-115).
fn is_pk_zone(zone_id: u16) -> bool {
    zone_id == ZONE_ARDREAM_PK
        || zone_id == ZONE_RONARK_LAND_PK
        || zone_id == ZONE_RONARK_LAND_BASE_PK
        || (ZONE_SPBATTLE_MIN..=ZONE_SPBATTLE_MAX).contains(&zone_id)
}

/// Build a WIZ_USER_INOUT packet for a player.
/// For INOUT_OUT: only sends type + ID (no user info).
/// For other types: appends full GetUserInfo data.
/// Uses ABNORMAL_NORMAL (1) as the default abnormal type.
pub fn build_user_inout(
    inout_type: u8,
    session_id: SessionId,
    character: Option<&CharacterInfo>,
    position: &Position,
) -> Packet {
    build_user_inout_with_clan(
        inout_type,
        session_id,
        character,
        position,
        None,
        None,
        false,
        0,
        1,
        &BroadcastState::default(),
        &EMPTY_EQUIP_VISUAL,
    )
}

/// Build a WIZ_USER_INOUT packet with invisibility type and abnormal type.
/// Same as `build_user_inout` but allows specifying the player's current
/// invisibility type and abnormal type so they're correctly included in
/// the GetUserInfo block.
pub fn build_user_inout_with_invis(
    inout_type: u8,
    session_id: SessionId,
    character: Option<&CharacterInfo>,
    position: &Position,
    invisibility_type: u8,
    abnormal_type: u32,
    equip_visual: &[(u32, i16, u8); 17],
) -> Packet {
    build_user_inout_with_clan(
        inout_type,
        session_id,
        character,
        position,
        None,
        None,
        false,
        invisibility_type,
        abnormal_type,
        &BroadcastState::default(),
        equip_visual,
    )
}

/// Build a WIZ_USER_INOUT packet with optional clan info, invisibility type, and abnormal type.
#[allow(clippy::too_many_arguments)]
pub fn build_user_inout_with_clan(
    inout_type: u8,
    session_id: SessionId,
    character: Option<&CharacterInfo>,
    position: &Position,
    clan: Option<&KnightsInfo>,
    alliance_cape: Option<(u16, u8, u8, u8)>,
    is_king: bool,
    invisibility_type: u8,
    abnormal_type: u32,
    bs: &BroadcastState,
    equip_visual: &[(u32, i16, u8); 17],
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizUserInout as u8);
    pkt.write_u8(inout_type);
    pkt.write_u8(0); // reserved
    pkt.write_u32(session_id as u32);

    if inout_type != INOUT_OUT {
        if let Some(ch) = character {
            write_user_info(
                &mut pkt,
                ch,
                position,
                clan,
                alliance_cape,
                is_king,
                invisibility_type,
                abnormal_type,
                bs,
                equip_visual,
            );
        }
    }

    pkt
}

/// Resolve alliance cape data for a clan member.
/// If the player's clan is in an alliance, returns `(cape_id, R, G, B)` from the
/// main alliance clan (with sub/mercenary color overrides per C++).
pub(crate) fn resolve_alliance_cape(
    ki: &KnightsInfo,
    world: &WorldState,
) -> Option<(u16, u8, u8, u8)> {
    if ki.alliance == 0 {
        return None;
    }
    let alliance = world.get_alliance(ki.alliance)?;
    let main_clan = world.get_knights(ki.alliance)?;
    let clan_id = ki.id;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    // Determine base cape from main alliance clan (castellan or regular)
    let (base_cape, base_r, base_g, base_b) =
        if main_clan.castellan_cape && main_clan.cast_cape_time >= now {
            (
                main_clan.cast_cape_id as u16,
                main_clan.cast_cape_r,
                main_clan.cast_cape_g,
                main_clan.cast_cape_b,
            )
        } else {
            (
                main_clan.cape,
                main_clan.cape_r,
                main_clan.cape_g,
                main_clan.cape_b,
            )
        };

    if alliance.sub_clan == clan_id {
        // Sub alliance → main clan's cape + own RGB
        Some((base_cape, ki.cape_r, ki.cape_g, ki.cape_b))
    } else if alliance.mercenary_1 == clan_id || alliance.mercenary_2 == clan_id {
        // Mercenary → main clan's cape + zero RGB
        Some((base_cape, 0, 0, 0))
    } else {
        // Main alliance (or default fallback) → main clan's cape + main clan's RGB
        Some((base_cape, base_r, base_g, base_b))
    }
}

/// Write GetUserInfo data to a packet.
/// Writes full player info including clan data when available.
/// The `invisibility_type` parameter carries the player's current stealth state
/// (`m_bInvisibilityType`). Per C++ UserInfoSystem.cpp:362-367, the client
/// only understands value 1 (`INVIS_DISPEL_ON_MOVE`), so any non-zero invisibility
/// type is converted to 1 before being written to the packet.
/// The `abnormal_type` parameter carries the player's current abnormal state
/// (`m_bAbnormalType`). This reflects GM invisibility, transformations,
/// and other visual state changes. Default is 1 (ABNORMAL_NORMAL).
#[allow(clippy::too_many_arguments)]
pub fn write_user_info(
    pkt: &mut Packet,
    ch: &CharacterInfo,
    pos: &Position,
    clan: Option<&KnightsInfo>,
    alliance_cape: Option<(u16, u8, u8, u8)>,
    is_king: bool,
    invisibility_type: u8,
    abnormal_type: u32,
    bs: &BroadcastState,
    equip_visual: &[(u32, i16, u8); 17],
) {
    // Name (SByte — u8 length prefix)
    pkt.write_sbyte_string(&ch.name);

    // Nation
    pkt.write_u8(ch.nation);

    // 3 padding bytes
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(0);

    // Clan ID + fame
    // C++ line 269: GetClanID() << uint8(isInPKZone() == true ? uint8(0) : GetFame())
    pkt.write_i16(ch.knights_id as i16);
    let fame = if is_pk_zone(pos.zone_id) { 0 } else { ch.fame };
    pkt.write_u8(fame);

    // Clan info block
    match clan {
        Some(ki) => {
            // Has clan: write real data
            // C++ line 281: alliance_id(u16) << clan_name(SByte) << grade(u8) << ranking(u8) << mark_version(u16)
            pkt.write_u16(ki.alliance);
            pkt.write_sbyte_string(&ki.name);
            pkt.write_u8(ki.grade);
            pkt.write_u8(ki.ranking);
            pkt.write_u16(ki.mark_version);

            // Cape data — C++ UserInfoSystem.cpp:286-352
            // King always gets nation cape (97 Karus, 98 Elmo) regardless of clan/alliance
            if is_king {
                let king_cape = if ch.nation == 1 { 97u16 } else { 98u16 };
                pkt.write_u16(king_cape);
                pkt.write_u32(0);
            } else if let Some((ac_cape, ac_r, ac_g, ac_b)) = alliance_cape {
                // Alliance cape — resolved by resolve_alliance_cape()
                pkt.write_u16(ac_cape);
                pkt.write_u8(ac_r);
                pkt.write_u8(ac_g);
                pkt.write_u8(ac_b);
                pkt.write_u8(0);
            } else if ki.castellan_cape
                && ki.cast_cape_time
                    >= (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as u32)
            {
                // Own castellan cape (non-alliance)
                pkt.write_u16(ki.cast_cape_id as u16);
                pkt.write_u8(ki.cast_cape_r);
                pkt.write_u8(ki.cast_cape_g);
                pkt.write_u8(ki.cast_cape_b);
                pkt.write_u8(0);
            } else {
                // Own regular cape (non-alliance)
                pkt.write_u16(ki.cape);
                pkt.write_u8(ki.cape_r);
                pkt.write_u8(ki.cape_g);
                pkt.write_u8(ki.cape_b);
                pkt.write_u8(0);
            }

            // v2615 UserInfoSystem.cpp writes m_byFlag at this exact field.
            // Recomputing it from clan grade sent 0 for promoted clans and
            // made the client hide both the equipped cape and cape catalog.
            pkt.write_u8(ki.flag);
        }
        None => {
            // No clan: write empty clan data
            // C++ line 275: pKnights == nullptr → uint32(0)+uint16(0)+uint8(0) = 7 zero bytes
            pkt.write_u16(0); // alliance_id
            pkt.write_u8(0); // empty clan name (SByte len=0)
            pkt.write_u8(0); // grade
            pkt.write_u8(0); // ranking
            pkt.write_u16(0); // mark_version
                              // King cape: C++ line 274-275 — isKing() writes nation-specific cape ID
            let cape_id = if is_king {
                if ch.nation == 1 {
                    97u16
                } else {
                    98u16
                }
            } else {
                0xFFFF // -1 = no cape
            };
            pkt.write_u16(cape_id);
            pkt.write_u32(0); // cape R,G,B,pad
            pkt.write_u8(0); // symbol flag
        }
    }

    // Level, race, class
    pkt.write_u8(ch.level);
    pkt.write_u8(ch.race);
    pkt.write_u16(ch.class);

    // Position — C++ GetSPosX() = uint16(GetX() * 10)
    pkt.write_u16((pos.x * 10.0) as u16);
    pkt.write_u16((pos.z * 10.0) as u16);
    pkt.write_u16((pos.y * 10.0) as u16);

    // Face + Hair
    pkt.write_u8(ch.face);
    pkt.write_u32(ch.hair_rgb);

    // Status flags
    pkt.write_u8(ch.res_hp_type); // m_bResHpType — C++ USER_STANDING=1, USER_DEAD=3
    pkt.write_u32(abnormal_type); // m_bAbnormalType from session
    pkt.write_u8(0); // v2600: unknown byte after abnormal_type (always 0 in sniff)
    pkt.write_u8(bs.need_party); // m_bNeedParty (1=looking for party)
    pkt.write_u8(ch.authority); // m_bAuthority (0=GM, 1=Player)
    pkt.write_u8(bs.party_leader); // m_bPartyLeader

    // The client only understands INVIS_DISPEL_ON_MOVE (1). Any non-zero
    // invisibility type (including INVIS_DISPEL_ON_ATTACK=2) is converted to 1.
    let client_invis = if invisibility_type != 0 { 1u8 } else { 0u8 };
    pkt.write_u8(client_invis); // bInvisibilityType

    pkt.write_u8(bs.team_colour); // m_teamColour
    pkt.write_u8(bs.is_devil); // m_bIsDevil
    pkt.write_u8(0); // padding
    pkt.write_u16(bs.direction); // m_sDirection
    pkt.write_u8(if ch.level < 30 { 1 } else { 0 }); // m_bIsChicken — C++ GetLevel() < 30
    pkt.write_u8(ch.rank);

    // Dual rank flags
    pkt.write_u8(0);
    pkt.write_u8(0);

    // Knights/personal rank — C++ UserInfoSystem.cpp:401
    // Write the better (lower value = higher rank) of the two; -1 if unranked.
    let kr = bs.knights_rank;
    let pr = bs.personal_rank;
    pkt.write_i8(if kr <= pr { kr } else { -1 });
    pkt.write_i8(if pr <= kr { pr } else { -1 });

    // Equipment — 17 visual slots: item_id(u32) + duration(u16) + flag(u8)
    // equip_visual is in VISUAL_SLOT_ORDER: 8 equipped + 9 cosplay
    for &(item_id, dur, flag) in equip_visual {
        pkt.write_u32(item_id); // nNum
        pkt.write_u16(dur as u16); // sDuration
        pkt.write_u8(flag); // bFlag
    }

    // Zone + trailing data
    pkt.write_u16(pos.zone_id);
    pkt.write_i32(-1); // unknown (-1)
    pkt.write_u8(0); // padding
    pkt.write_u32(0); // unknown
    pkt.write_u8(bs.is_hiding_helmet); // m_bIsHidingHelmet
    pkt.write_u8(bs.is_hiding_cospre); // m_bIsHidingCospre
    pkt.write_u8(bs.is_in_genie); // isInGenie()
    pkt.write_u8(ch.rebirth_level); // C++ GetRebirthLevel()
    pkt.write_u16(ch.cover_title); // cover title
    pkt.write_u32(bs.return_symbol_ok); // ReturnSymbolisOK — C++ User.h:366 uint32
    pkt.write_u8(0); // padding
    pkt.write_u8(0); // v2600: unknown (u8, not u32 — verified by sniff)
    pkt.write_u8(1); // v2600: final byte (always 1 in sniff)
}

/// Send region change data to a player — the 3-phase WIZ_REGIONCHANGE protocol.
/// Phase 0: start marker
/// Phase 1: compressed list of nearby user/NPC socket IDs
/// Phase 2: end marker
pub async fn send_region_user_in_out_for_me(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (pos, my_event_room) = match world.with_session(sid, |h| (h.position, h.event_room)) {
        Some(v) => v,
        None => return Ok(()),
    };

    // Phase 0: START
    let mut start = Packet::new(Opcode::WizRegionChange as u8);
    start.write_u8(0);
    session.send_packet(&start).await?;

    // Phase 1: DATA — list of nearby session IDs (event_room filtered)
    let nearby = world.get_nearby_session_ids(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Some(sid),
        my_event_room,
    );
    let nearby_bots: Vec<_> = world
        .get_bots_in_zone_live(pos.zone_id)
        .into_iter()
        .filter(|bot| {
            (bot.region_x as i32 - pos.region_x as i32).unsigned_abs() <= 1
                && (bot.region_z as i32 - pos.region_z as i32).unsigned_abs() <= 1
        })
        .collect();

    let mut data = Packet::new(Opcode::WizRegionChange as u8);
    data.write_u8(1); // phase 1
    data.write_u16((nearby.len() + nearby_bots.len()) as u16);
    for &other_id in &nearby {
        data.write_u32(other_id as u32);
    }
    for bot in &nearby_bots {
        data.write_u32(bot.id);
    }

    // Send compressed (C++ uses SendCompressed)
    let to_send = match data.to_compressed() {
        Some(compressed) => compressed,
        None => data,
    };
    session.send_packet(&to_send).await?;

    // Phase 2: END
    let mut end = Packet::new(Opcode::WizRegionChange as u8);
    end.write_u8(2);
    session.send_packet(&end).await?;

    Ok(())
}

/// Send merchant INOUT info for nearby player/bot merchants.
/// Builds a `WIZ_MERCHANT_INOUT` packet containing all actively merchanting
/// players and bots in the 3×3 region grid around the caller. Sent on region
/// change, zone change, and game entry.
/// Packet: `[0x69][u8(1)][u16(count)][for each: u32(id) + u8(state) + u8(premium)]`
pub async fn send_merchant_user_in_out_for_me(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (pos, my_event_room) = match world.with_session(sid, |h| (h.position, h.event_room)) {
        Some(v) => v,
        None => return Ok(()),
    };

    let mut pkt = Packet::new(Opcode::WizMerchantInout as u8);
    pkt.write_u8(1); // INOUT_IN type
    let count_offset = pkt.data.len();
    pkt.write_u16(0); // placeholder for count

    let mut count: u16 = 0;

    // Scan nearby sessions for merchants
    let nearby = world.get_nearby_session_ids(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Some(sid),
        my_event_room,
    );

    for &other_id in &nearby {
        let info = world.with_session(other_id, |h| {
            if h.merchant_state == crate::world::types::MERCHANT_STATE_NONE {
                return None;
            }
            // C++ skips invisible players (ABNORMAL_INVISIBLE = 0)
            if h.abnormal_type == 0 {
                return None;
            }
            let state = h.merchant_state as u8;
            let premium = if h.merchant_state == crate::world::types::MERCHANT_STATE_BUYING {
                false
            } else {
                h.is_premium_merchant
            };
            Some((other_id as u32, state, premium))
        });
        if let Some(Some((id, state, premium))) = info {
            pkt.write_u32(id);
            pkt.write_u8(state);
            pkt.write_u8(if premium { 1 } else { 0 });
            count += 1;
        }
    }

    // Also check bots in same zone for merchant bots.
    // C++ uses region-level bot arrays; our bots aren't region-tracked,
    // so we scan all bots in the zone (small overhead since merchant bots are rare).
    for entry in world.bots.iter() {
        let bot = entry.value();
        if bot.zone_id != pos.zone_id {
            continue;
        }
        if bot.merchant_state == -1 {
            continue;
        }
        // Check if bot is within 3×3 region grid
        let drx = (bot.region_x as i32 - pos.region_x as i32).unsigned_abs();
        let drz = (bot.region_z as i32 - pos.region_z as i32).unsigned_abs();
        if drx > 1 || drz > 1 {
            continue;
        }
        let state = bot.merchant_state as u8;
        let premium = if bot.merchant_state == 1 {
            false
        } else {
            bot.premium_merchant
        };
        pkt.write_u32(bot.id);
        pkt.write_u8(state);
        pkt.write_u8(if premium { 1 } else { 0 });
        count += 1;
    }

    // Patch count
    pkt.data[count_offset] = (count & 0xFF) as u8;
    pkt.data[count_offset + 1] = ((count >> 8) & 0xFF) as u8;

    // Send compressed (C++ uses SendCompressed)
    let to_send = match pkt.to_compressed() {
        Some(compressed) => compressed,
        None => pkt,
    };
    session.send_packet(&to_send).await?;
    Ok(())
}

/// Broadcast INOUT_IN to nearby players when a user enters a region.
/// Also sends INOUT_IN of each nearby player TO the entering user.
/// Uses INOUT_RESPAWN as the broadcast type (default for cross-zone and respawn).
pub async fn broadcast_user_in(session: &mut ClientSession) -> anyhow::Result<()> {
    broadcast_user_in_with_type(session, INOUT_RESPAWN).await
}

/// Broadcast user appearance to nearby players with a specific INOUT type.
/// `inout_type` determines what the client shows for the entering user:
/// - INOUT_RESPAWN (3): normal respawn/cross-zone appearance
/// - INOUT_WARP (4): same-zone warp appearance
pub async fn broadcast_user_in_with_type(
    session: &mut ClientSession,
    inout_type: u8,
) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (pos, my_event_room) = match world.with_session(sid, |h| (h.position, h.event_room)) {
        Some(v) => v,
        None => return Ok(()),
    };

    let my_char = world.get_character_info(sid);
    let my_clan = my_char.as_ref().and_then(|ch| {
        if ch.knights_id > 0 {
            world.get_knights(ch.knights_id)
        } else {
            None
        }
    });

    // Build my INOUT packet for others
    let my_invis = world.get_invisibility_type(sid);
    let my_abnormal = world.get_abnormal_type(sid);
    let my_bs = world.get_broadcast_state(sid);
    let my_equip = get_equipped_visual(&world, sid);
    let my_alliance_cape = my_clan
        .as_ref()
        .and_then(|ki| resolve_alliance_cape(ki, &world));
    let my_is_king = my_char
        .as_ref()
        .is_some_and(|ch| world.is_king(ch.nation, &ch.name));
    let my_inout = build_user_inout_with_clan(
        inout_type,
        sid,
        my_char.as_ref(),
        &pos,
        my_clan.as_ref(),
        my_alliance_cape,
        my_is_king,
        my_invis,
        my_abnormal,
        &my_bs,
        &my_equip,
    );

    // Broadcast to 3×3 grid (everyone else sees me entering)
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(my_inout),
        Some(sid),
        my_event_room,
    );

    // Send INOUT_IN of each nearby player TO ME (event_room filtered)
    let nearby = world.get_nearby_session_ids(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Some(sid),
        my_event_room,
    );

    for &other_id in &nearby {
        // Single DashMap read for all broadcast data (replaces ~22 reads per player)
        let snap = match snapshot_broadcast_info(&world, other_id) {
            Some(s) => s,
            None => continue,
        };
        let other_clan = snap.character.as_ref().and_then(|ch| {
            if ch.knights_id > 0 {
                world.get_knights(ch.knights_id)
            } else {
                None
            }
        });
        let other_alliance_cape = other_clan
            .as_ref()
            .and_then(|ki| resolve_alliance_cape(ki, &world));
        let other_is_king = snap
            .character
            .as_ref()
            .is_some_and(|ch| world.is_king(ch.nation, &ch.name));
        let other_inout = build_user_inout_with_clan(
            INOUT_IN,
            other_id,
            snap.character.as_ref(),
            &snap.position,
            other_clan.as_ref(),
            other_alliance_cape,
            other_is_king,
            snap.invisibility_type,
            snap.abnormal_type,
            &snap.broadcast_state,
            &snap.equip_visual,
        );
        session.send_packet(&other_inout).await?;
    }

    // Send tag list for nearby players who have custom name tags.
    let tag_entries = super::tag_change::collect_region_tags(
        &world,
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Some(sid),
    );
    if !tag_entries.is_empty() {
        let tag_pkt = super::tag_change::build_tag_list_packet(&tag_entries);
        session.send_packet(&tag_pkt).await?;
    }

    Ok(())
}

/// Move an in-game user within the same zone and immediately synchronize the
/// region grid and all visibility packets. WIZ_WARP alone only moves the local
/// client; without the matching INOUT/region updates other clients keep the
/// old entity until their next region refresh.
pub fn relocate_user_within_zone(
    world: &WorldState,
    sid: SessionId,
    dest_x: f32,
    dest_y: f32,
    dest_z: f32,
) -> bool {
    let Some(old_pos) = world.get_position(sid) else {
        return false;
    };
    let Some(zone) = world.get_zone(old_pos.zone_id) else {
        return false;
    };
    if !zone.is_valid_position(dest_x, dest_z) {
        return false;
    }

    let (event_room, abnormal_type) = world
        .with_session(sid, |h| (h.event_room, h.abnormal_type))
        .unwrap_or((0, 1));
    let is_gm = world.is_gm(sid);

    // Remove the stale in-region entity first, even when source and target are
    // in the same cell. This guarantees a fresh INOUT_WARP at the destination.
    let out = build_user_inout(INOUT_OUT, sid, None, &old_pos);
    world.broadcast_to_3x3(
        old_pos.zone_id,
        old_pos.region_x,
        old_pos.region_z,
        Arc::new(out),
        Some(sid),
        event_room,
    );
    zone.remove_user(old_pos.region_x, old_pos.region_z, sid);

    world.update_position(sid, old_pos.zone_id, dest_x, dest_y, dest_z);
    let Some(new_pos) = world.get_position(sid) else {
        zone.add_user(old_pos.region_x, old_pos.region_z, sid);
        return false;
    };
    zone.add_user(new_pos.region_x, new_pos.region_z, sid);

    // Keep the server-side region grid and client position change atomic from
    // the client perspective: WIZ_WARP, then refresh nearby entities.
    let mut warp = Packet::new(Opcode::WizWarp as u8);
    warp.write_u16((dest_x * 10.0).round().clamp(0.0, u16::MAX as f32) as u16);
    warp.write_u16((dest_z * 10.0).round().clamp(0.0, u16::MAX as f32) as u16);
    warp.write_i16(-1);
    world.send_to_session_owned(sid, warp);

    let own_inout = snapshot_broadcast_info(world, sid).and_then(|snap| {
        let ch = snap.character.as_ref()?;
        let clan = (ch.knights_id > 0)
            .then(|| world.get_knights(ch.knights_id))
            .flatten();
        let cape = clan
            .as_ref()
            .and_then(|knights| resolve_alliance_cape(knights, world));
        let is_king = world.is_king(ch.nation, &ch.name);
        Some(build_user_inout_with_clan(
            INOUT_WARP,
            sid,
            Some(ch),
            &snap.position,
            clan.as_ref(),
            cape,
            is_king,
            snap.invisibility_type,
            snap.abnormal_type,
            &snap.broadcast_state,
            &snap.equip_visual,
        ))
    });
    if let Some(inout) = own_inout {
        // Do not reveal an invisible GM, matching normal movement behavior.
        if !is_gm || abnormal_type != 0 {
            world.broadcast_to_3x3(
                new_pos.zone_id,
                new_pos.region_x,
                new_pos.region_z,
                Arc::new(inout),
                Some(sid),
                event_room,
            );
        }
    }

    // Rebuild the nearby-user ID set and appearance packets for the moved user.
    let nearby = world.get_nearby_session_ids(
        new_pos.zone_id,
        new_pos.region_x,
        new_pos.region_z,
        Some(sid),
        event_room,
    );
    let bots = world.get_bots_in_zone_live(new_pos.zone_id);
    let nearby_bots: Vec<_> = bots
        .into_iter()
        .filter(|bot| {
            (bot.region_x as i32 - new_pos.region_x as i32).unsigned_abs() <= 1
                && (bot.region_z as i32 - new_pos.region_z as i32).unsigned_abs() <= 1
        })
        .collect();
    let mut start = Packet::new(Opcode::WizRegionChange as u8);
    start.write_u8(0);
    world.send_to_session_owned(sid, start);
    let mut users = Packet::new(Opcode::WizRegionChange as u8);
    users.write_u8(1);
    users.write_u16((nearby.len() + nearby_bots.len()).min(u16::MAX as usize) as u16);
    for other_id in &nearby {
        users.write_u32(*other_id as u32);
    }
    for bot in &nearby_bots {
        users.write_u32(bot.id);
    }
    let users = users.to_compressed().unwrap_or(users);
    world.send_to_session_owned(sid, users);
    let mut end = Packet::new(Opcode::WizRegionChange as u8);
    end.write_u8(2);
    world.send_to_session_owned(sid, end);

    for other_id in nearby {
        let Some(snap) = snapshot_broadcast_info(world, other_id) else {
            continue;
        };
        let clan = snap.character.as_ref().and_then(|ch| {
            (ch.knights_id > 0)
                .then(|| world.get_knights(ch.knights_id))
                .flatten()
        });
        let cape = clan
            .as_ref()
            .and_then(|knights| resolve_alliance_cape(knights, world));
        let is_king = snap
            .character
            .as_ref()
            .is_some_and(|ch| world.is_king(ch.nation, &ch.name));
        let inout = build_user_inout_with_clan(
            INOUT_IN,
            other_id,
            snap.character.as_ref(),
            &snap.position,
            clan.as_ref(),
            cape,
            is_king,
            snap.invisibility_type,
            snap.abnormal_type,
            &snap.broadcast_state,
            &snap.equip_visual,
        );
        world.send_to_session_owned(sid, inout);
    }

    true
}

/// Send the NPC region list to a player — WIZ_NPC_REGION (0x1C) compressed.
/// Sends a compressed list of nearby NPC IDs so the client knows which NPCs
/// exist in the player's visibility range.
pub async fn send_region_npc_info_for_me(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (pos, my_event_room) = match world.with_session(sid, |h| (h.position, h.event_room)) {
        Some(v) => v,
        None => return Ok(()),
    };

    let npc_ids = world.get_nearby_npc_ids(pos.zone_id, pos.region_x, pos.region_z, my_event_room);

    let mut pkt = Packet::new(Opcode::WizNpcRegion as u8);
    pkt.write_u16(npc_ids.len() as u16);
    for &nid in &npc_ids {
        pkt.write_u32(nid);
    }

    // Send compressed (C++ uses SendCompressed for region packets)
    let to_send = match pkt.to_compressed() {
        Some(compressed) => compressed,
        None => pkt,
    };
    session.send_packet(&to_send).await?;

    Ok(())
}

/// Send WIZ_NPC_INOUT(IN) for all nearby NPCs to a player.
/// Called during game entry so the player sees all NPCs in their vicinity.
pub async fn send_nearby_npc_inouts(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    let (pos, my_event_room) = match world.with_session(sid, |h| (h.position, h.event_room)) {
        Some(v) => v,
        None => return Ok(()),
    };

    let npc_ids = world.get_nearby_npc_ids(pos.zone_id, pos.region_x, pos.region_z, my_event_room);

    for &nid in &npc_ids {
        let instance = match world.get_npc_instance(nid) {
            Some(n) => n,
            None => continue,
        };

        let template = match world.get_npc_template(instance.proto_id, instance.is_monster) {
            Some(t) => t,
            None => continue,
        };

        let template = world.npc_visual_template(&instance, &template);
        let pkt = build_npc_inout(NPC_IN, &instance, &template);
        session.send_packet(&pkt).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dragon_armor_warrior() {
        // Warrior classes: 1, 5, 6
        for class in [1, 5, 6, 101, 105, 106, 201, 205, 206] {
            let items = dragon_armor_for_class(class);
            assert_eq!(
                items,
                Some([507001010, 507002010, 507003010, 507004010, 507005010]),
                "warrior class {class}"
            );
        }
    }

    #[test]
    fn test_dragon_armor_kurian() {
        // Kurian classes: 13, 14, 15 — share warrior dragon armor
        for class in [13, 14, 15, 113, 114, 115] {
            let items = dragon_armor_for_class(class);
            assert_eq!(
                items,
                Some([507001010, 507002010, 507003010, 507004010, 507005010]),
                "kurian class {class}"
            );
        }
    }

    #[test]
    fn test_dragon_armor_rogue() {
        // Rogue classes: 2, 7, 8
        for class in [2, 7, 8, 102, 107, 108] {
            let items = dragon_armor_for_class(class);
            assert_eq!(
                items,
                Some([547001010, 547002010, 547003010, 547004010, 547005010]),
                "rogue class {class}"
            );
        }
    }

    #[test]
    fn test_dragon_armor_mage() {
        // Mage classes: 3, 9, 10
        for class in [3, 9, 10, 103, 109, 110] {
            let items = dragon_armor_for_class(class);
            assert_eq!(
                items,
                Some([567001010, 567002010, 567003010, 567004010, 567005010]),
                "mage class {class}"
            );
        }
    }

    #[test]
    fn test_dragon_armor_priest() {
        // Priest classes: 4, 11, 12
        for class in [4, 11, 12, 104, 111, 112] {
            let items = dragon_armor_for_class(class);
            assert_eq!(
                items,
                Some([587001018, 587002018, 587003018, 587004018, 587005018]),
                "priest class {class}"
            );
        }
    }

    #[test]
    fn test_dragon_armor_invalid_class() {
        assert_eq!(dragon_armor_for_class(0), None);
        assert_eq!(dragon_armor_for_class(16), None);
        assert_eq!(dragon_armor_for_class(99), None);
    }

    #[test]
    fn test_visual_slot_order_length() {
        assert_eq!(VISUAL_SLOT_ORDER.len(), 17);
        // First 8 are equipped slots
        assert_eq!(VISUAL_SLOT_ORDER[0], 4); // BREAST
        assert_eq!(VISUAL_SLOT_ORDER[1], 10); // LEG
        assert_eq!(VISUAL_SLOT_ORDER[2], 1); // HEAD
        assert_eq!(VISUAL_SLOT_ORDER[3], 12); // GLOVE
        assert_eq!(VISUAL_SLOT_ORDER[4], 13); // FOOT
        assert_eq!(VISUAL_SLOT_ORDER[5], 5); // SHOULDER
        assert_eq!(VISUAL_SLOT_ORDER[6], 6); // RIGHTHAND
        assert_eq!(VISUAL_SLOT_ORDER[7], 8); // LEFTHAND
                                             // Next 9 are cosplay slots
        assert_eq!(VISUAL_SLOT_ORDER[8], 42); // CWING
    }

    #[test]
    fn test_empty_equip_visual() {
        for &(id, dur, flag) in &EMPTY_EQUIP_VISUAL {
            assert_eq!(id, 0);
            assert_eq!(dur, 0);
            assert_eq!(flag, 0);
        }
    }

    #[test]
    fn test_merchant_inout_packet_format() {
        // Build a merchant INOUT packet with 2 entries
        let mut pkt = Packet::new(Opcode::WizMerchantInout as u8);
        pkt.write_u8(1); // INOUT_IN
        pkt.write_u16(2); // count
                          // Entry 1: selling, premium
        pkt.write_u32(100); // session_id
        pkt.write_u8(0); // MERCHANT_STATE_SELLING
        pkt.write_u8(1); // premium
                         // Entry 2: buying, not premium
        pkt.write_u32(200);
        pkt.write_u8(1); // MERCHANT_STATE_BUYING
        pkt.write_u8(0); // not premium

        assert_eq!(pkt.opcode, Opcode::WizMerchantInout as u8);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1)); // INOUT_IN
        assert_eq!(r.read_u16(), Some(2)); // count
                                           // Entry 1
        assert_eq!(r.read_u32(), Some(100));
        assert_eq!(r.read_u8(), Some(0)); // selling
        assert_eq!(r.read_u8(), Some(1)); // premium
                                          // Entry 2
        assert_eq!(r.read_u32(), Some(200));
        assert_eq!(r.read_u8(), Some(1)); // buying
        assert_eq!(r.read_u8(), Some(0)); // not premium
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_merchant_inout_buying_never_premium() {
        // C++ logic: buying merchants always have premium=false
        // "pUser->GetMerchantState() == 1 ? false : pUser->m_bPremiumMerchant"
        let merchant_state: i8 = 1; // BUYING
        let is_premium = true;
        let result = if merchant_state == 1 {
            false
        } else {
            is_premium
        };
        assert!(!result, "buying merchant should never be premium");
    }

    #[test]
    fn test_merchant_inout_empty_no_merchants() {
        // Test that empty packet (count=0) is valid
        let mut pkt = Packet::new(Opcode::WizMerchantInout as u8);
        pkt.write_u8(1);
        pkt.write_u16(0);
        let mut r = ko_protocol::PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 950: Additional coverage ──────────────────────────────

    /// INOUT type constants: IN=1, OUT=2, RESPAWN=3, WARP=4.
    #[test]
    fn test_inout_type_constants() {
        assert_eq!(INOUT_IN, 1);
        assert_eq!(INOUT_OUT, 2);
        assert_eq!(INOUT_RESPAWN, 3);
        assert_eq!(INOUT_WARP, 4);
    }

    /// EMPTY_EQUIP_VISUAL has exactly 17 entries.
    #[test]
    fn test_empty_equip_visual_length() {
        assert_eq!(EMPTY_EQUIP_VISUAL.len(), 17);
    }

    /// VISUAL_SLOT_ORDER has all unique slots.
    #[test]
    fn test_visual_slot_order_unique() {
        let mut seen = std::collections::HashSet::new();
        for &slot in &VISUAL_SLOT_ORDER {
            assert!(seen.insert(slot), "duplicate slot {slot}");
        }
    }

    /// Battle zone constants are distinct.
    #[test]
    fn test_battle_zone_constants() {
        assert!(ZONE_SPBATTLE_MIN < ZONE_SPBATTLE_MAX);
        assert_ne!(ZONE_BATTLE3, ZONE_SPBATTLE_MIN);
    }

    /// Dragon armor sets have exactly 5 items each.
    #[test]
    fn test_dragon_armor_set_size() {
        for class in [1, 2, 3, 4] {
            let items = dragon_armor_for_class(class);
            assert_eq!(items.unwrap().len(), 5, "class {class}");
        }
    }

    // ── Sprint 966: Additional coverage ──────────────────────────────

    /// VISUAL_SLOT_ORDER cosplay slots start at index 42.
    #[test]
    fn test_visual_slot_order_cosplay_range() {
        // First 8 are equipment slots (all < 42)
        for &slot in &VISUAL_SLOT_ORDER[..8] {
            assert!(slot < 42, "equipment slot {} should be < 42", slot);
        }
        // Last 9 are cosplay slots (all >= 42)
        for &slot in &VISUAL_SLOT_ORDER[8..] {
            assert!(slot >= 42, "cosplay slot {} should be >= 42", slot);
        }
    }

    /// Dragon armor item ID prefixes distinguish class types.
    #[test]
    fn test_dragon_armor_id_prefixes() {
        let warrior = dragon_armor_for_class(1).unwrap();
        let rogue = dragon_armor_for_class(2).unwrap();
        let mage = dragon_armor_for_class(3).unwrap();
        let priest = dragon_armor_for_class(4).unwrap();
        // Warrior: 507xxx, Rogue: 547xxx, Mage: 567xxx, Priest: 587xxx
        assert!(warrior[0] / 1_000_000 == 507);
        assert!(rogue[0] / 1_000_000 == 547);
        assert!(mage[0] / 1_000_000 == 567);
        assert!(priest[0] / 1_000_000 == 587);
    }

    /// ZONE_BATTLE3=63 is outside special battle range.
    #[test]
    fn test_zone_battle3_outside_spbattle() {
        assert_eq!(ZONE_BATTLE3, 63);
        assert_eq!(ZONE_SPBATTLE_MIN, 105);
        assert_eq!(ZONE_SPBATTLE_MAX, 115);
        assert!(ZONE_BATTLE3 < ZONE_SPBATTLE_MIN);
    }

    /// NPC_IN constant matches region INOUT protocol.
    #[test]
    fn test_npc_in_constant() {
        assert_eq!(NPC_IN, 1);
        assert_eq!(INOUT_IN, NPC_IN);
    }

    /// Dragon armor class mapping covers all advanced classes (100+, 200+).
    #[test]
    fn test_dragon_armor_advanced_classes() {
        // Base warrior=1, advanced=101, master=201 — all map to same armor
        let base = dragon_armor_for_class(1).unwrap();
        let adv = dragon_armor_for_class(101).unwrap();
        let master = dragon_armor_for_class(201).unwrap();
        assert_eq!(base, adv);
        assert_eq!(base, master);
    }
}
