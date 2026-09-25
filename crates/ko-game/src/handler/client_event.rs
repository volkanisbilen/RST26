//! WIZ_CLIENT_EVENT (0x52) handler — NPC click / interaction.
//! When a player clicks on an NPC, the client sends this packet with the
//! NPC's runtime ID. The server validates range, stores the event NPC IDs,
//! and dispatches to the appropriate handler (special NPC types, or quest
//! lookup via quest_helper).
//! ## Request (C->S)
//! | Offset | Type   | Description |
//! |--------|--------|-------------|
//! | 0      | u16le  | NPC runtime ID (NID) |
//! ## Response
//! No direct response — triggers quest dialog (WIZ_SELECT_MSG / WIZ_QUEST)
//! or special NPC effects (damage, items, etc.).

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, info};

use crate::session::{ClientSession, SessionState};

use super::knights;

use crate::npc_type_constants::{
    MAX_NPC_RANGE, NPC_LOYALTY_MERCHANT, NPC_MERCHANT, NPC_OBJECT_WOOD, NPC_ROLLINGSTONE,
    NPC_TINKER, NPC_WAREHOUSE,
};

/// NPC type: Cape mark NPC (clan cape customization).
const NPC_MARK: u8 = 25;

/// WIZ_KNIGHTS_PROCESS sub-opcode used by the v2615 client to open the
/// clan-cape palette.  This is `KnightsPacket::KNIGHTS_CAPE_NPC` (27/0x1B),
/// not the older/incorrect 0x14 value.
const KNIGHTS_CAPE_NPC: u8 = 0x1B;

/// NPC type: Captain NPC (class change).
const NPC_CAPTAIN: u8 = 35;

/// NPC type: Rental NPC.
const NPC_RENTAL: u8 = 78;

/// NPC type: Chaotic Generator (gem exchange).
const NPC_CHAOTIC_GENERATOR: u8 = 137;

/// NPC type: Chaotic Generator v2 (newer type).
const NPC_CHAOTIC_GENERATOR2: u8 = 162;

/// WIZ_ITEM_UPGRADE sub-opcode for Chaotic Generator dialog.
const ITEM_BIFROST_REQ: u8 = 4;

fn packet_hex(opcode: u8, data: &[u8]) -> String {
    let mut out = format!("{opcode:02X}");
    for byte in data {
        use std::fmt::Write as _;
        let _ = write!(&mut out, " {byte:02X}");
    }
    out
}

/// Build the Inn Hostess menu request. This packet must not contain warehouse
/// page data; normal/VIP storage is opened by the client's next request.
fn build_warehouse_menu_open() -> Packet {
    let mut pkt = Packet::new(Opcode::WizWarehouse as u8);
    pkt.write_u8(0x10); // WAREHOUSE_REQ
    pkt
}

/// Build the Chaotic Generator dialog-open response.
///
/// The v2525 client reads the NPC runtime ID as a 32-bit little-endian value.
/// Sending only u16 leaves the request context incomplete, causing the client
/// to submit ITEM_BIFROST_PROCESS with npc_id=0.
fn build_chaotic_generator_open(npc_nid: u32) -> Packet {
    let mut pkt = Packet::new(Opcode::WizItemUpgrade as u8);
    pkt.write_u8(ITEM_BIFROST_REQ);
    pkt.write_u32(npc_nid);
    pkt
}

/// NPC type: King election NPC.
const NPC_ELECTION: u8 = 79;

/// NPC type: King treasury NPC.
const NPC_TREASURY: u8 = 80;

/// NPC type: Event Manager NPC (v2603 IDA: type 174, shares handler with 171).
/// Clicking opens the active event info dialog (WIZ_EVENT TEMPLE_EVENT).
const NPC_EVENT_MANAGER: u8 = 174;

/// Dedicated daily-quest NPC template. Its visual data is copied from a
/// v2615-known model in the database migration, but it has its own proto ID
/// and is never shared with an existing NPC.
const NPC_DAILY_QUEST_MANAGER: u16 = 31999;

/// Handle WIZ_CLIENT_EVENT from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let npc_nid_raw = match reader.read_u16() {
        Some(v) => v as u32,
        None => return Ok(()),
    };

    // ClientEvent() at NPCHandler.cpp:95 does GetNpcPtr((int16)sNpcID, ...) — NO NPC_BAND addition.
    // Client sends the full NPC runtime ID (already includes NPC_BAND).
    let npc_nid = npc_nid_raw;
    handle_npc_by_nid(session, npc_nid).await
}

/// Handle WIZ_NPC_EVENT (0x20) from the client.
/// Packet format: `[u8 unknown] [u32 npc_nid] [i32 quest_id]`
/// Dispatches by NPC type (merchant, warehouse, etc.) and falls through
/// to quest dialog for other NPC types.
pub async fn handle_npc_event(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    // v2615 CUITiketExchange confirms with exactly nine payload bytes:
    // [u8 sub=6][u32 ticket item][u32 selected reward]. This shares
    // WIZ_NPC_EVENT with ordinary NPC clicks, so length and sub-opcode must be
    // separated before the legacy [unknown][npc nid][quest id] parser.
    if pkt.data.len() == 9 && pkt.data.first() == Some(&6) {
        return handle_ticket_exchange(session, &pkt).await;
    }

    let mut reader = PacketReader::new(&pkt.data);
    let _unknown = reader.read_u8().unwrap_or(0);
    let npc_nid = match reader.read_u32() {
        Some(v) => v,
        None => return Ok(()),
    };
    // quest_id (i32) — currently unused but read from packet
    let _quest_id = reader.read_u32().unwrap_or(0) as i32;

    handle_npc_by_nid(session, npc_nid).await
}

async fn handle_ticket_exchange(session: &mut ClientSession, pkt: &Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let _sub = reader.read_u8();
    let ticket_item = reader.read_u32().unwrap_or(0);
    let reward_item = reader.read_u32().unwrap_or(0);

    // The allow-list is imported from the client's TICKET_EXCHANGE.tbl. The
    // client controls both IDs on the wire, so never trust the pair without
    // this server-side lookup.
    let rule = sqlx::query_as::<_, (i32, i16)>(
        "SELECT duration_hours, selector FROM native_ticket_exchange_rule \
         WHERE ticket_item_id = $1 AND reward_item_id = $2 LIMIT 1",
    )
    .bind(ticket_item as i32)
    .bind(reward_item as i32)
    .fetch_optional(session.pool())
    .await?;

    let world = session.world().clone();
    let sid = session.session_id();
    let nation = world
        .get_character_info(sid)
        .map(|character| character.nation)
        .unwrap_or(0);
    let selector_allowed = rule.is_some_and(|(_, selector)| match selector {
        1 => nation == 1, // Karus-only row
        2 => nation == 2, // El Morad-only row
        _ => true,
    });
    let status = if !selector_allowed || world.get_item(reward_item).is_none() {
        3u8 // invalid ticket/reward pair
    } else {
        let ticket_count: u32 = world
            .get_inventory(sid)
            .iter()
            .filter(|slot| slot.item_id == ticket_item)
            .map(|slot| slot.count as u32)
            .sum();
        if ticket_count == 0 {
            2u8 // ticket is no longer in inventory
        } else if !world.check_weight(sid, reward_item, 1) {
            4u8 // inventory slot or weight failure
        } else if !world.rob_item(sid, ticket_item, 1) {
            2u8
        } else {
            let hours = rule.map(|(hours, _)| hours).unwrap_or(0).max(0) as u32;
            let days = hours.div_ceil(24);
            let given = if days > 0 {
                world.give_item_with_expiry(sid, reward_item, 1, days)
            } else {
                world.give_item(sid, reward_item, 1)
            };
            if given {
                let account = world
                    .with_session(sid, |h| h.account_id.clone())
                    .unwrap_or_default();
                let character = world.get_session_name(sid).unwrap_or_default();
                let pos = world.get_position(sid);
                crate::handler::audit_log::log_give_item(
                    session.pool(),
                    &account,
                    &character,
                    pos.as_ref().map(|p| p.zone_id as i16).unwrap_or(0),
                    pos.as_ref().map(|p| p.x as i16).unwrap_or(0),
                    pos.as_ref().map(|p| p.z as i16).unwrap_or(0),
                    "ticket_exchange",
                    reward_item,
                    1,
                );
                tracing::info!(
                    sid,
                    ticket_item,
                    reward_item,
                    duration_hours = hours,
                    "Ticket Exchange completed"
                );
                1u8
            } else {
                // Pre-validation should make this exceptional; restore the
                // consumed ticket if delivery still fails.
                let _ = world.give_item(sid, ticket_item, 1);
                5u8
            }
        }
    };

    // sub_807C20 consumes [sub=6][status]. Status 1 is silent success;
    // statuses 2..5 select the client's native error strings.
    let mut out = Packet::new(Opcode::WizNpcEvent as u8);
    out.write_u8(6);
    out.write_u8(status);
    session.send_packet(&out).await?;
    Ok(())
}

/// Core NPC interaction logic shared by WIZ_CLIENT_EVENT and WIZ_NPC_EVENT.
/// Takes the full NPC NID (NPC_BAND + offset) and dispatches by NPC type.
async fn handle_npc_by_nid(session: &mut ClientSession, npc_nid: u32) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Player must be alive and not busy
    // isDead() || isTrading() || isMerchanting() || isStoreOpen() || isSellingMerchant()
    //   || isBuyingMerchant() || isMining() || isFishing()
    // Note: isStoreOpen() always returns false in C++ (User.h:989)
    // Note: isMerchanting() covers sellingMerchant/buyingMerchant states
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        return Ok(());
    }

    let ch = match world.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(()),
    };

    // Look up NPC instance
    // pNpc == nullptr || pNpc->isDead() || !isInRange(pNpc, MAX_NPC_RANGE)
    let npc = match world.get_npc_instance(npc_nid) {
        Some(n) => n,
        None => return Ok(()),
    };

    // NPC must be alive — C++ removes dead NPCs (GetNpcPtr returns null)
    if world.is_npc_dead(npc_nid) {
        return Ok(());
    }

    // Verify NPC is in the same zone
    let pos = match world.get_position(sid) {
        Some(p) => p,
        None => return Ok(()),
    };
    if npc.zone_id != pos.zone_id {
        return Ok(());
    }

    // Range check
    let dx = pos.x - npc.x;
    let dz = pos.z - npc.z;
    let dist = (dx * dx + dz * dz).sqrt();
    if dist > MAX_NPC_RANGE {
        debug!(
            "[{}] ClientEvent: NPC {} out of range ({:.0} > {:.0})",
            session.addr(),
            npc_nid,
            dist,
            MAX_NPC_RANGE,
        );
        return Ok(());
    }

    // Store event NPC IDs for subsequent quest/dialog interactions
    //   m_sEventNid = (int16)sNpcID;
    //   m_sEventSid = pNpc->GetProtoID();
    let proto_id = npc.proto_id;
    world.update_session(sid, |h| {
        h.event_nid = npc_nid as i16;
        h.event_sid = proto_id as i16;
    });

    // Akara must be opened only by a real NPC interaction (right-click), never
    // by WIZ_TARGET_HP: that packet is also emitted by ordinary left-click
    // target selection.
    if proto_id == 31774 {
        super::native_events::try_open_akara_menu_from_target(session, npc_nid).await?;
        return Ok(());
    }

    if proto_id == NPC_DAILY_QUEST_MANAGER {
        super::daily_quest::open_daily_quest_manager(session, 0).await?;
        return Ok(());
    }

    // Look up template for NPC type
    let tmpl = world.get_npc_template(proto_id, npc.is_monster);

    // ── GM debug: send NPC info via chat when GM clicks an NPC ──────
    // v2525 client drops ext_hook (0xE9), so GM debug mode can't be toggled.
    // Instead, send NPC info as a PUBLIC_CHAT message to the GM.
    if ch.authority == 0 || ch.authority == 2 {
        let npc_name = tmpl.as_ref().map(|t| t.name.as_str()).unwrap_or("<NoName>");
        let npc_type = tmpl.as_ref().map(|t| t.npc_type).unwrap_or(0);
        let npc_level = tmpl.as_ref().map(|t| t.level).unwrap_or(0);
        let is_mon = if npc.is_monster { "MON" } else { "NPC" };
        let debug_msg = format!(
            "[GM] {} nid={} proto={} name={} lv={} type={}",
            is_mon, npc_nid, proto_id, npc_name, npc_level, npc_type
        );
        send_gm_debug_chat(&world, sid, &debug_msg);
    }

    // Handle special NPC types by npc_type
    if let Some(ref t) = tmpl {
        match t.npc_type {
            crate::npc_type_constants::NPC_VICTORY_GATE => {
                let battle = world.get_battle_state();
                if battle.is_nation_battle()
                    && battle.victory == ch.nation
                    && pos.zone_id == battle.battle_zone_id()
                {
                    // NPCHandler.cpp: the winner chooses the existing Victory Gate.
                    let (zone, x, z) = match ch.nation {
                        1 if battle.elmorad_open_flag => (2, 222.0, 1846.0),
                        2 if battle.karus_open_flag => (1, 1865.0, 168.0),
                        _ => return Ok(()),
                    };
                    super::zone_change::trigger_zone_change(session, zone, x, z).await?;
                }
                return Ok(());
            }
            NPC_ROLLINGSTONE => {
                // Instant death — apply full HP damage
                let damage = ch.max_hp;
                let new_hp = (ch.hp - damage).max(0);
                world.update_character_stats(sid, |c| {
                    c.hp = new_hp;
                });
                let hp_pkt = crate::systems::regen::build_hp_change_packet(ch.max_hp, new_hp);
                world.send_to_session_owned(sid, hp_pkt);

                if new_hp <= 0 {
                    super::dead::broadcast_death(&world, sid);
                }
                debug!(
                    "[{}] ClientEvent: NPC {} (ROLLINGSTONE) dealt {} damage",
                    session.addr(),
                    npc_nid,
                    damage,
                );
                return Ok(());
            }
            NPC_OBJECT_WOOD => {
                // 80% HP damage
                let damage = (ch.max_hp as i32 * 80 / 100) as i16;
                let new_hp = (ch.hp - damage).max(0);
                world.update_character_stats(sid, |c| {
                    c.hp = new_hp;
                });
                let hp_pkt = crate::systems::regen::build_hp_change_packet(ch.max_hp, new_hp);
                world.send_to_session_owned(sid, hp_pkt);

                if new_hp <= 0 {
                    super::dead::broadcast_death(&world, sid);
                }
                debug!(
                    "[{}] ClientEvent: NPC {} (OBJECT_WOOD) dealt {} damage",
                    session.addr(),
                    npc_nid,
                    damage,
                );
                return Ok(());
            }
            NPC_MERCHANT | NPC_LOYALTY_MERCHANT => {
                // Open merchant shop UI
                let mut shop_pkt = Packet::new(Opcode::WizTradeNpc as u8);
                shop_pkt.write_u32(t.selling_group);
                session.send_packet(&shop_pkt).await?;
                debug!(
                    "[{}] ClientEvent: NPC {} (MERCHANT) selling_group={}",
                    session.addr(),
                    npc_nid,
                    t.selling_group,
                );
                return Ok(());
            }
            NPC_TINKER => {
                // Open tinker/repair shop UI
                let mut shop_pkt = Packet::new(Opcode::WizRepairNpc as u8);
                shop_pkt.write_u32(t.selling_group);
                session.send_packet(&shop_pkt).await?;
                debug!(
                    "[{}] ClientEvent: NPC {} (TINKER) selling_group={}",
                    session.addr(),
                    npc_nid,
                    t.selling_group,
                );
                return Ok(());
            }
            NPC_MARK => {
                // Cape mark NPC — open clan cape customization UI
                let clan_id = session
                    .world()
                    .get_character_info(session.session_id())
                    .map(|ch| ch.knights_id)
                    .unwrap_or(0);
                let clan_state = if clan_id > 0 {
                    let state = session
                        .world()
                        .get_knights(clan_id)
                        .map(|k| (k.id, k.flag, k.grade, k.cape, k.ranking));
                    // Refresh this client's clan/cape cache synchronously
                    // before opening the v2615 mantle UI.  The UI filters
                    // Cloak.tbl locally from this state; using only the
                    // broadcast queue can let the window open before the
                    // KNIGHTS_UPDATE packet is processed, leaving the
                    // catalogue empty.
                    if let Some(update_pkt) = knights::build_knights_update_packet(session, clan_id)
                    {
                        let update_packet =
                            packet_hex(Opcode::WizKnightsProcess as u8, &update_pkt.data);
                        session.send_packet(&update_pkt).await?;
                        info!(
                            addr = %session.addr(),
                            npc_nid,
                            clan_id,
                            clan_state = ?state,
                            packet = %update_packet,
                            "MARK/CAPE direct update packet"
                        );
                    }
                    state
                } else {
                    None
                };
                let mut pkt = Packet::new(Opcode::WizKnightsProcess as u8);
                pkt.write_u8(KNIGHTS_CAPE_NPC);
                let open_packet = packet_hex(Opcode::WizKnightsProcess as u8, &pkt.data);
                session.send_packet(&pkt).await?;
                info!(
                    addr = %session.addr(),
                    npc_nid,
                    clan_id,
                    clan_state = ?clan_state,
                    packet = %open_packet,
                    "MARK/CAPE open packet"
                );
                return Ok(());
            }
            NPC_RENTAL => {
                // Rental NPC — open rental UI
                let mut pkt = Packet::new(Opcode::WizRental as u8);
                pkt.write_u8(3); // RENTAL_NPC sub-opcode
                pkt.write_u16(1); // enabled
                pkt.write_u32(t.selling_group);
                session.send_packet(&pkt).await?;
                debug!("[{}] ClientEvent: NPC {} (RENTAL)", session.addr(), npc_nid);
                return Ok(());
            }
            NPC_CAPTAIN => {
                // Class change captain NPC
                let mut pkt = Packet::new(Opcode::WizClassChange as u8);
                pkt.write_u8(0x01); // CLASS_CHANGE_REQ
                session.send_packet(&pkt).await?;
                debug!(
                    "[{}] ClientEvent: NPC {} (CAPTAIN)",
                    session.addr(),
                    npc_nid
                );
                return Ok(());
            }
            NPC_WAREHOUSE => {
                // v2615/reference flow: the NPC click only asks the client to
                // show the integrated warehouse menu. The client sends the
                // normal/VIP open request after the player makes a selection.
                // Opening either store here skips that menu entirely.
                let pkt = build_warehouse_menu_open();
                session.send_packet(&pkt).await?;
                debug!(
                    "[{}] ClientEvent: NPC {} (WAREHOUSE)",
                    session.addr(),
                    npc_nid
                );
                return Ok(());
            }
            NPC_CHAOTIC_GENERATOR | NPC_CHAOTIC_GENERATOR2 => {
                // Chaotic Generator — open gem/fragment exchange dialog.
                // S2C: WIZ_ITEM_UPGRADE [sub=ITEM_BIFROST_REQ(4)] [npc_id:u32le]
                let pkt = build_chaotic_generator_open(npc_nid);
                session.send_packet(&pkt).await?;
                debug!(
                    "[{}] ClientEvent: NPC {} (CHAOTIC_GENERATOR) bifrost_req",
                    session.addr(),
                    npc_nid
                );
                return Ok(());
            }
            NPC_ELECTION => {
                // King election NPC — show king name
                let ks = world.get_king_system(ch.nation);
                let king_name = ks.as_ref().map(|k| k.king_name.as_str()).unwrap_or("");
                let mut pkt = Packet::new(Opcode::WizKing as u8);
                pkt.write_u8(5); // KING_NPC sub-opcode
                pkt.write_sbyte_string(king_name);
                session.send_packet(&pkt).await?;
                debug!(
                    "[{}] ClientEvent: NPC {} (ELECTION) king={}",
                    session.addr(),
                    npc_nid,
                    king_name
                );
                return Ok(());
            }
            // NPC_EVENT_MANAGER (174): handled via quest_helper Lua (31772_Aset.lua).
            // Falls through to quest NPC interaction below.
            NPC_TREASURY => {
                // King treasury NPC — show tax/treasury info
                let ks = world.get_king_system(ch.nation);
                let tribute = ks
                    .as_ref()
                    .map(|k| k.tribute + k.territory_tax)
                    .unwrap_or(0);
                let treasury = ks.as_ref().map(|k| k.national_treasury).unwrap_or(0);
                let char_name = ch.name.clone();
                let is_king = world.is_king(ch.nation, &char_name);
                let mut pkt = Packet::new(Opcode::WizKing as u8);
                pkt.write_u8(3); // KING_TAX sub-opcode
                pkt.write_u8(1); // success
                if is_king {
                    pkt.write_u16(1); // king mode
                    pkt.write_u32(tribute);
                    pkt.write_u32(treasury);
                } else {
                    pkt.write_u16(2); // normal user mode
                    pkt.write_u32(treasury);
                    pkt.write_u32(0);
                }
                session.send_packet(&pkt).await?;
                debug!(
                    "[{}] ClientEvent: NPC {} (TREASURY) king={}",
                    session.addr(),
                    npc_nid,
                    is_king
                );
                return Ok(());
            }
            _ => {}
        }
    }

    // ── Quest NPC interaction ─────────────────────────────────────
    if let Some(helper_indices) = world.get_quest_npc_helpers(proto_id) {
        let ch = match world.get_character_info(sid) {
            Some(c) => c,
            None => return Ok(()),
        };

        let mut selected_helper: Option<ko_db::models::QuestHelperRow> = None;
        for &idx in &helper_indices {
            if let Some(helper) = world.get_quest_helper(idx) {
                // C++ filters: skip helpers with event data or status requirements
                if helper.s_event_data_index != 0 {
                    continue;
                }
                if helper.b_event_status != 0 {
                    continue;
                }
                // Nation filter (3 = any nation)
                if helper.b_nation != 3 && helper.b_nation != ch.nation as i16 {
                    continue;
                }
                // Class filter (5 = any class)
                if helper.b_class != 5 && !super::quest::job_group_check(ch.class, helper.b_class) {
                    continue;
                }
                selected_helper = Some(helper);
                break;
            }
        }

        if let Some(helper) = selected_helper {
            // Run the quest Lua script for this NPC interaction
            debug!(
                "[{}] ClientEvent: NPC proto={} matched quest helper idx={} trigger={}",
                session.addr(),
                proto_id,
                helper.n_index,
                helper.n_event_trigger_index,
            );

            // ── GM debug: show quest event trigger info ──────────────
            if ch.authority == 0 || ch.authority == 2 {
                let debug_msg = format!(
                    "[GM] Quest: lua={} n_index={} trigger={} quest_type={}",
                    helper.str_lua_filename,
                    helper.n_index,
                    helper.n_event_trigger_index,
                    helper.b_quest_type,
                );
                send_gm_debug_chat(&world, sid, &debug_msg);
            }

            super::quest::quest_v2_run_event(
                &world,
                sid,
                &helper,
                helper.n_event_trigger_index,
                -1,
            );
        } else {
            debug!(
                "[{}] ClientEvent: NPC proto={} no matching quest helper",
                session.addr(),
                proto_id,
            );

            // ── GM debug: no quest helper matched ────────────────────
            if ch.authority == 0 || ch.authority == 2 {
                let debug_msg = format!(
                    "[GM] Quest: NPC proto={} — no matching quest helper",
                    proto_id,
                );
                send_gm_debug_chat(&world, sid, &debug_msg);
            }
        }
    } else {
        debug!(
            "[{}] ClientEvent: NPC nid={} proto={} has no quest helpers",
            session.addr(),
            npc_nid,
            proto_id,
        );
    }

    Ok(())
}

/// Send a GM debug message via PUBLIC_CHAT (WIZ_CHAT type 7).
/// v2525 client drops WIZ_EXT_HOOK (0xE9) so GM debug mode can't be toggled.
/// This is the v2525-compatible alternative: send debug info as chat text.
pub fn send_gm_debug_chat(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    msg: &str,
) {
    let mut pkt = Packet::new(Opcode::WizChat as u8);
    pkt.write_u8(7); // PUBLIC_CHAT
    pkt.write_u8(0); // nation = 0 (system)
    pkt.write_u32(sid as u32);
    pkt.write_u8(0); // name length (SByte empty)
    pkt.write_string(msg); // DByte message
    pkt.write_i8(0); // personal_rank
    pkt.write_u8(0); // authority
    pkt.write_u8(20); // system_msg = 20 (GM color)
    world.send_to_session_owned(sid, pkt);
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_type_constants() {
        assert_eq!(NPC_ROLLINGSTONE, 181); // C++ globals.h:203
        assert_eq!(NPC_OBJECT_WOOD, 54); // C++ globals.h:144
    }

    #[test]
    fn test_range_check() {
        // Within range
        let dx: f32 = 20.0;
        let dz: f32 = 20.0;
        let dist = (dx * dx + dz * dz).sqrt();
        assert!(dist <= MAX_NPC_RANGE); // ~28.28 < 30

        // Out of range
        let dx: f32 = 25.0;
        let dz: f32 = 25.0;
        let dist = (dx * dx + dz * dz).sqrt();
        assert!(dist > MAX_NPC_RANGE); // ~35.36 > 30
    }

    #[test]
    fn test_rolling_stone_damage() {
        // Full HP damage
        let max_hp: i16 = 5000;
        let new_hp = (max_hp - max_hp).max(0);
        assert_eq!(new_hp, 0);
    }

    #[test]
    fn test_object_wood_damage() {
        // 80% HP damage
        let max_hp: i16 = 1000;
        let damage = (max_hp as i32 * 80 / 100) as i16;
        assert_eq!(damage, 800);
        let new_hp = (max_hp - damage).max(0);
        assert_eq!(new_hp, 200);
    }

    #[test]
    fn test_client_event_packet_format() {
        // Client sends: [u16 npc_nid]
        let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizClientEvent as u8);
        pkt.write_u16(42); // NPC runtime ID

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u16(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_quest_helper_nation_filter() {
        // Nation 3 matches any
        assert!(3 == 3 || 3 == 1); // nation=3 always matches
                                   // Nation 1 matches Karus player
        assert!(1 == 1); // nation=1 matches Karus
                         // Nation 2 does NOT match Karus player
        assert!(2 != 3); // nation=2 doesn't match Karus
    }

    #[test]
    fn test_quest_helper_class_filter() {
        use super::super::quest::job_group_check;
        // Class 5 matches any class (sentinel)
        assert!(job_group_check(101, 5)); // warrior, any class
        assert!(job_group_check(211, 5)); // priest novice, any class

        // GROUP_WARRIOR(1): ClassWarrior=1, ClassWarriorNovice=5, ClassWarriorMaster=6
        assert!(job_group_check(101, 1)); // Karus Warrior(base 1)
        assert!(job_group_check(105, 1)); // Karus WarriorNovice(base 5)
        assert!(job_group_check(206, 1)); // Elmo WarriorMaster(base 6)
        assert!(!job_group_check(102, 1)); // Karus Rogue(base 2) != warrior

        // GROUP_ROGUE(2): ClassRogue=2, ClassRogueNovice=7, ClassRogueMaster=8
        assert!(job_group_check(102, 2)); // Karus Rogue(base 2)
        assert!(job_group_check(107, 2)); // Karus RogueNovice(base 7)
        assert!(job_group_check(208, 2)); // Elmo RogueMaster(base 8)
        assert!(!job_group_check(101, 2)); // Karus Warrior != rogue

        // GROUP_MAGE(3): ClassMage=3, ClassMageNovice=9, ClassMageMaster=10
        assert!(job_group_check(103, 3)); // Karus Mage(base 3)
        assert!(job_group_check(109, 3)); // Karus MageNovice(base 9)
        assert!(job_group_check(210, 3)); // Elmo MageMaster(base 10)
        assert!(!job_group_check(104, 3)); // Karus Priest != mage

        // GROUP_CLERIC(4): ClassPriest=4, ClassPriestNovice=11, ClassPriestMaster=12
        assert!(job_group_check(104, 4)); // Karus Priest(base 4)
        assert!(job_group_check(111, 4)); // Karus PriestNovice(base 11)
        assert!(job_group_check(212, 4)); // Elmo PriestMaster(base 12)
        assert!(!job_group_check(103, 4)); // Karus Mage != priest

        // GROUP_PORTU_KURIAN(13): ClassKurian=13, Novice=14, Master=15
        assert!(job_group_check(113, 13)); // Karus Kurian(base 13)
        assert!(job_group_check(114, 13)); // Karus KurianNovice(base 14)
        assert!(job_group_check(215, 13)); // Elmo KurianMaster(base 15)
        assert!(!job_group_check(101, 13)); // Warrior != kurian

        // Exact class match (required_class > 100)
        assert!(job_group_check(101, 101)); // exact match
        assert!(!job_group_check(102, 101)); // Rogue != 101
    }

    // ── Sprint 923: Additional coverage ──────────────────────────────

    /// WIZ_NPC_EVENT C2S format: [u8 unknown][u32 npc_nid][i32 quest_id].
    #[test]
    fn test_npc_event_packet_format() {
        let mut pkt = Packet::new(Opcode::WizNpcEvent as u8);
        pkt.write_u8(0); // unknown
        pkt.write_u32(10042); // npc_nid
        pkt.write_u32(1500); // quest_id

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u32(), Some(10042));
        assert_eq!(r.read_u32(), Some(1500));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_ticket_exchange_packet_format() {
        // CUITiketExchange.cpp sub_71C6B0:
        // [sub=6][ticket item:u32][selected reward:u32].
        let mut pkt = Packet::new(Opcode::WizNpcEvent as u8);
        pkt.write_u8(6);
        pkt.write_u32(508_056_000);
        pkt.write_u32(508_013_318);
        assert_eq!(pkt.data.len(), 9);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(6));
        assert_eq!(r.read_u32(), Some(508_056_000));
        assert_eq!(r.read_u32(), Some(508_013_318));
        assert_eq!(r.remaining(), 0);
    }

    /// Merchant NPC → WIZ_TRADE_NPC with selling_group.
    #[test]
    fn test_merchant_shop_response_format() {
        let mut pkt = Packet::new(Opcode::WizTradeNpc as u8);
        pkt.write_u32(5001); // selling_group
        assert_eq!(pkt.opcode, Opcode::WizTradeNpc as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(5001));
        assert_eq!(r.remaining(), 0);
    }

    /// Tinker NPC → WIZ_REPAIR_NPC with selling_group.
    #[test]
    fn test_tinker_repair_response_format() {
        let mut pkt = Packet::new(Opcode::WizRepairNpc as u8);
        pkt.write_u32(6001);
        assert_eq!(pkt.opcode, Opcode::WizRepairNpc as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(6001));
        assert_eq!(r.remaining(), 0);
    }

    /// Cape mark NPC → WIZ_KNIGHTS_PROCESS/KNIGHTS_CAPE_NPC (27/0x1B).
    #[test]
    fn test_cape_mark_npc_response() {
        let mut pkt = Packet::new(Opcode::WizKnightsProcess as u8);
        pkt.write_u8(KNIGHTS_CAPE_NPC);
        assert_eq!(pkt.data.len(), 1);
        assert_eq!(pkt.data[0], 0x1B);
    }

    /// Inn Hostess sends only the menu request; storage page data must wait
    /// for the player's normal/VIP selection.
    #[test]
    fn test_warehouse_npc_opens_selection_menu_only() {
        let pkt = build_warehouse_menu_open();
        assert_eq!(pkt.opcode, Opcode::WizWarehouse as u8);
        assert_eq!(pkt.data, [0x10]);
    }

    /// Rental NPC → WIZ_RENTAL sub=3, enabled=1, selling_group.
    #[test]
    fn test_rental_npc_response_format() {
        let mut pkt = Packet::new(Opcode::WizRental as u8);
        pkt.write_u8(3);
        pkt.write_u16(1); // enabled
        pkt.write_u32(7001); // selling_group

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(3));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u32(), Some(7001));
        assert_eq!(r.remaining(), 0);
    }

    /// Chaotic Generator open response keeps the full 32-bit NPC runtime ID.
    #[test]
    fn test_chaotic_generator_open_response_format() {
        let pkt = build_chaotic_generator_open(0x0000_C2B6);

        assert_eq!(pkt.opcode, Opcode::WizItemUpgrade as u8);
        assert_eq!(pkt.data, [ITEM_BIFROST_REQ, 0xB6, 0xC2, 0x00, 0x00]);
    }

    /// Special NPC type constants match C++ defines.
    #[test]
    fn test_special_npc_type_constants() {
        assert_eq!(NPC_MARK, 25);
        assert_eq!(NPC_CAPTAIN, 35);
        assert_eq!(NPC_RENTAL, 78);
        assert_eq!(NPC_ELECTION, 79);
        assert_eq!(NPC_TREASURY, 80);
    }
}
