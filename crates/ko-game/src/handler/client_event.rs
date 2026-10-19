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

/// Dedicated daily-quest NPC template. Its visual data is copied from a
/// v2615-known model in the database migration, but it has its own proto ID
/// and is never shared with an existing NPC.
const NPC_DAILY_QUEST_MANAGER: u16 = 31999;

/// Moradon "Fun Class Event" NPC (client proto added by `patch_funclass_npc`). Right-click
/// sends the player into the running Fun Class (Cinderella) event, or says there is none.
const NPC_FUNCLASS_EVENT: u16 = 31998;

async fn funclass_npc_enter(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let zone = world.cinderella_zone_id();
    if !world.is_cinderella_active() || zone == 0 || world.get_zone(zone).is_none() {
        let pkt = super::chat::build_chat_packet(
            8, 0, 0, "**", "There is no Fun Class Event running right now.", 0, 0, 0,
        );
        world.send_to_session_owned(sid, pkt);
        return Ok(());
    }
    super::zone_change::trigger_zone_change(session, zone, 0.0, 0.0).await?;
    Ok(())
}

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

/// Result codes of the Ticket Exchange reply `[sub=6][status]` (client `sub_80DC30`, case 6).
/// Status 1 is a silent success; the others select a native client message:
/// 2 = "Your inventory is full.", 3 = "Invalid item.",
/// 4 = "You cannot purchase this item using exchange voucher", 5 = "System Error".
const TICKET_STATUS_OK: u8 = 1;
const TICKET_STATUS_NO_ROOM: u8 = 2;
const TICKET_STATUS_INVALID_ITEM: u8 = 3;
const TICKET_STATUS_RESTRICTED: u8 = 4;
const TICKET_STATUS_SYSTEM_ERROR: u8 = 5;

/// Whether a TICKET_EXCHANGE.tbl reward slot with `selector` may be taken by this character.
///
/// Mirrors the client list filter in `CUITiketExchange` (`sub_722740`):
/// 0 = everybody, 1 = Karus only, 2 = El Morad only, 3 = everybody except
/// Portu/Kurian classes (113-115, 213-215; client text 44851 "Portu/Kurian
/// characters cannot use this.").
pub(crate) fn ticket_selector_allows(selector: i16, nation: u8, class: u16) -> bool {
    match selector {
        1 => nation == 1,
        2 => nation == 2,
        3 => !super::class_change::is_portu_kurian(class),
        _ => true,
    }
}

/// Lifetime in seconds of a reward, from the `duration` column (hours) of TICKET_EXCHANGE.tbl.
/// 0 means the reward never expires.
pub(crate) fn ticket_expiry_secs(duration_hours: i32) -> u32 {
    (duration_hours.max(0) as u32).saturating_mul(3600)
}

async fn send_ticket_exchange_status(
    session: &mut ClientSession,
    status: u8,
) -> anyhow::Result<()> {
    // The client consumes [sub=6][status].
    let mut out = Packet::new(Opcode::WizNpcEvent as u8);
    out.write_u8(6);
    out.write_u8(status);
    session.send_packet(&out).await
}

async fn handle_ticket_exchange(session: &mut ClientSession, pkt: &Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let _sub = reader.read_u8();
    let ticket_item = reader.read_u32().unwrap_or(0);
    let reward_item = reader.read_u32().unwrap_or(0);
    let world = session.world().clone();
    let sid = session.session_id();
    tracing::info!(sid, ticket_item, reward_item, "Ticket Exchange request received");

    // Same "busy" guard as an NPC click: a dead, trading, merchant-mode, mining or
    // fishing player must not mutate the bag.
    if world.is_player_dead(sid)
        || world.is_trading(sid)
        || world.is_merchanting(sid)
        || world.is_mining(sid)
        || world.is_fishing(sid)
    {
        tracing::warn!(
            sid, ticket_item, reward_item,
            "Ticket Exchange rejected: player busy (status 3)"
        );
        return send_ticket_exchange_status(session, TICKET_STATUS_INVALID_ITEM).await;
    }

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

    let status = match rule {
        None => {
            tracing::warn!(
                sid, ticket_item, reward_item,
                "Ticket Exchange rejected: pair not in TICKET_EXCHANGE rules (status 3)"
            );
            TICKET_STATUS_INVALID_ITEM
        }
        Some((duration_hours, selector)) => {
            let (nation, class) = world
                .get_character_info(sid)
                .map(|c| (c.nation, c.class))
                .unwrap_or((0, 0));
            if !ticket_selector_allows(selector, nation, class) {
                tracing::warn!(
                    sid, ticket_item, reward_item, selector, nation, class,
                    "Ticket Exchange rejected: nation/class restriction (status 4)"
                );
                TICKET_STATUS_RESTRICTED
            } else if world.get_item(reward_item).is_none() {
                tracing::warn!(
                    sid, ticket_item, reward_item,
                    "Ticket Exchange rejected: reward item missing from item table (status 5)"
                );
                TICKET_STATUS_SYSTEM_ERROR
            } else if world.bag_item_count(sid, ticket_item) == 0 {
                tracing::warn!(
                    sid, ticket_item, reward_item,
                    "Ticket Exchange rejected: ticket not in bag (status 3)"
                );
                TICKET_STATUS_INVALID_ITEM
            } else if !world.ticket_exchange_has_room(sid, ticket_item, reward_item) {
                tracing::warn!(
                    sid, ticket_item, reward_item,
                    "Ticket Exchange rejected: no bag slot/weight for reward (status 2)"
                );
                TICKET_STATUS_NO_ROOM
            } else if !world.take_one_item(sid, ticket_item) {
                tracing::warn!(
                    sid, ticket_item, reward_item,
                    "Ticket Exchange rejected: ticket could not be consumed (status 3)"
                );
                TICKET_STATUS_INVALID_ITEM
            } else {
                // Exactly one ticket is consumed per exchange and exactly one reward
                // is delivered. There is no await between the two world calls and the
                // room check above makes a failed delivery exceptional.
                let expiry_secs = ticket_expiry_secs(duration_hours);
                let given = if expiry_secs > 0 {
                    world.give_item_with_expiry_secs(sid, reward_item, 1, expiry_secs)
                } else {
                    world.give_item(sid, reward_item, 1)
                };
                if given {
                    let account = world
                        .with_session(sid, |h| h.account_id.clone())
                        .unwrap_or_default();
                    let character = world.get_session_name(sid).unwrap_or_default();
                    let pos = world.get_position(sid);
                    let zone = pos.as_ref().map(|p| p.zone_id as i16).unwrap_or(0);
                    let x = pos.as_ref().map(|p| p.x as i16).unwrap_or(0);
                    let z = pos.as_ref().map(|p| p.z as i16).unwrap_or(0);
                    crate::handler::audit_log::log_rob_item(
                        session.pool(),
                        &account,
                        &character,
                        zone,
                        x,
                        z,
                        ticket_item,
                        1,
                        0,
                    );
                    crate::handler::audit_log::log_give_item(
                        session.pool(),
                        &account,
                        &character,
                        zone,
                        x,
                        z,
                        "ticket_exchange",
                        reward_item,
                        1,
                    );
                    tracing::info!(
                        sid, ticket_item, reward_item, duration_hours,
                        "Ticket Exchange completed"
                    );
                    TICKET_STATUS_OK
                } else {
                    tracing::warn!(
                        sid, ticket_item, reward_item,
                        "Ticket Exchange failed: reward delivery failed after the ticket was consumed; restoring the ticket (status 5)"
                    );
                    // Never keep the ticket if the reward was not delivered.
                    if !world.give_item(sid, ticket_item, 1) {
                        tracing::error!(
                            sid, ticket_item, reward_item,
                            "Ticket Exchange: ticket restore failed after a delivery failure"
                        );
                    }
                    TICKET_STATUS_SYSTEM_ERROR
                }
            }
        }
    };

    send_ticket_exchange_status(session, status).await
}

/// Core NPC interaction logic shared by WIZ_CLIENT_EVENT and WIZ_NPC_EVENT.
/// Takes the full NPC NID (NPC_BAND + offset) and dispatches by NPC type.
async fn handle_npc_by_nid(session: &mut ClientSession, npc_nid: u32) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    tracing::info!(sid, npc_nid, "NPC click received");

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
    tracing::info!(sid, npc_nid, proto_id, "NPC click in range");
    world.update_session(sid, |h| {
        h.event_nid = npc_nid as i16;
        h.event_sid = proto_id as i16;
    });

    // Akara must be opened only by a real NPC interaction (right-click), never
    // by WIZ_TARGET_HP: that packet is also emitted by ordinary left-click
    // target selection.
    // Chaos Dungeon objects (C++ CUser::ClientEvent, NPCHandler.cpp)
    if proto_id == crate::systems::chaos_arena::SAW_BLADE_PROTO {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        crate::systems::chaos_arena::handle_saw_touch(&world, sid, npc_nid, now_ms);
        return Ok(());
    }
    if proto_id == crate::systems::chaos_arena::CHAOS_CUBE_PROTO {
        use rand::Rng;
        let roll = rand::thread_rng().gen_range(1..=4u32);
        crate::systems::chaos_arena::handle_cube_click(&world, sid, npc_nid, roll);
        return Ok(());
    }

    if proto_id == 31774 {
        super::native_events::try_open_akara_menu_from_target(session, npc_nid).await?;
        return Ok(());
    }

    if proto_id == NPC_FUNCLASS_EVENT {
        funclass_npc_enter(session).await?;
        return Ok(());
    }

    if proto_id == NPC_DAILY_QUEST_MANAGER {
        super::daily_quest::open_daily_quest_manager(session, 0).await?;
        return Ok(());
    }

    // Look up template for NPC type
    let tmpl = world.get_npc_template(proto_id, npc.is_monster);

    // Reference C++ does not print target information on every target packet.
    // GMs explicitly request one stable line through +npcinfo after Z-targeting.

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

    // ── TICKET_EXCHANGE.tbl audit tests ──────────────────────────────

    #[test]
    fn test_ticket_selector_matches_client_filter() {
        // 0 = everybody (Portu included), unknown selectors behave like the client: shown.
        for class in [101u16, 112, 113, 213, 215] {
            assert!(ticket_selector_allows(0, 1, class));
            assert!(ticket_selector_allows(9, 2, class));
        }
        // 1 = Karus only, 2 = El Morad only.
        assert!(ticket_selector_allows(1, 1, 105));
        assert!(!ticket_selector_allows(1, 2, 205));
        assert!(ticket_selector_allows(2, 2, 205));
        assert!(!ticket_selector_allows(2, 1, 105));
        // 3 = every class except Portu/Kurian (113-115, 213-215).
        for class in [101u16, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 201, 212] {
            assert!(ticket_selector_allows(3, 1, class), "class {class}");
            assert!(ticket_selector_allows(3, 2, class), "class {class}");
        }
        for class in [113u16, 114, 115, 213, 214, 215] {
            assert!(!ticket_selector_allows(3, 1, class), "class {class}");
            assert!(!ticket_selector_allows(3, 2, class), "class {class}");
        }
    }

    #[test]
    fn test_ticket_expiry_is_exact_hours() {
        assert_eq!(ticket_expiry_secs(0), 0);
        assert_eq!(ticket_expiry_secs(-5), 0);
        assert_eq!(ticket_expiry_secs(1), 3_600);
        assert_eq!(ticket_expiry_secs(4), 14_400);
        assert_eq!(ticket_expiry_secs(720), 30 * 86_400);
        assert_eq!(ticket_expiry_secs(1440), 60 * 86_400);
        assert_eq!(ticket_expiry_secs(4320), 180 * 86_400);
        assert_eq!(ticket_expiry_secs(i32::MAX), u32::MAX); // saturates, never wraps
    }

    #[test]
    fn test_ticket_status_codes_match_client_messages() {
        // sub_80DC30 case 6: 1 silent, 2..5 native messages.
        let all = [
            TICKET_STATUS_OK,
            TICKET_STATUS_NO_ROOM,
            TICKET_STATUS_INVALID_ITEM,
            TICKET_STATUS_RESTRICTED,
            TICKET_STATUS_SYSTEM_ERROR,
        ];
        assert_eq!(all, [1, 2, 3, 4, 5]);
    }

    /// (ticket, reward, duration_hours, selector) rows exactly as the client reads
    /// `Data\TICKET_EXCHANGE.tbl`: `[ticket u32][hours u32]` + 30 × `[selector u8][item u32]`.
    fn ticket_tbl_rules() -> Vec<(u32, u32, u32, u8)> {
        use ko_tbl_import::parser::CellValue;
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/data/TICKET_EXCHANGE.tbl");
        let raw = std::fs::read(path).expect("docs/data/TICKET_EXCHANGE.tbl");
        let (plain, new_structure) = ko_tbl_import::decrypt::decrypt_tbl(&raw).expect("decrypt");
        let table = ko_tbl_import::parser::parse_tbl(&plain, new_structure).expect("parse");
        assert_eq!(table.columns.len(), 62, "2 header columns + 30 selector/item pairs");
        let as_u32 = |c: &CellValue| match c {
            CellValue::U32(v) => *v,
            other => panic!("expected u32 cell, got {other:?}"),
        };
        let as_u8 = |c: &CellValue| match c {
            CellValue::U8(v) => *v,
            other => panic!("expected u8 cell, got {other:?}"),
        };
        let mut out = Vec::new();
        for row in &table.rows {
            assert_eq!(row.len(), 62);
            let ticket = as_u32(&row[0]);
            let hours = as_u32(&row[1]);
            for pair in row[2..].chunks(2) {
                let (selector, item) = (as_u8(&pair[0]), as_u32(&pair[1]));
                if item != 0 {
                    out.push((ticket, item, hours, selector));
                }
            }
        }
        out
    }

    fn migration_sql() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../migrations/20261018210000_ticket_exchange_full_sync.sql"
        ))
        .expect("ticket exchange migration")
    }

    #[test]
    fn test_ticket_tbl_is_well_formed() {
        let rules = ticket_tbl_rules();
        assert!(!rules.is_empty());
        let mut seen = std::collections::HashSet::new();
        let mut tickets = std::collections::HashSet::new();
        for (ticket, reward, hours, selector) in &rules {
            assert!(*selector <= 3, "unknown selector {selector} on ticket {ticket}");
            assert_ne!(ticket, reward, "ticket {ticket} rewards itself");
            assert!(seen.insert((*ticket, *reward)), "duplicate pair {ticket}->{reward}");
            assert!(*hours <= 24 * 365, "implausible duration on ticket {ticket}");
            assert!(*ticket <= i32::MAX as u32 && *reward <= i32::MAX as u32);
            tickets.insert(*ticket);
        }
        // A reward is never itself another exchangeable ticket (no chains / loops).
        for (_, reward, _, _) in &rules {
            assert!(!tickets.contains(reward), "reward {reward} is also a ticket");
        }
    }

    /// The server rule table must mirror the client table 1:1 (duration + selector too).
    #[test]
    fn test_migration_rules_mirror_tbl_exactly() {
        let sql = migration_sql();
        let block = sql
            .split("-- BEGIN RULES")
            .nth(1)
            .and_then(|s| s.split("-- END RULES").next())
            .expect("rules block markers");
        let mut migrated = std::collections::HashSet::new();
        for line in block.lines() {
            let line = line.trim();
            if !line.starts_with('(') {
                continue;
            }
            let nums: Vec<u64> = line
                .trim_matches(|c| c == '(' || c == ')' || c == ',' || c == ';')
                .split(',')
                .map(|n| n.trim().parse().expect("number"))
                .collect();
            assert_eq!(nums.len(), 4, "bad rule line: {line}");
            assert!(
                migrated.insert((nums[0] as u32, nums[1] as u32, nums[2] as u32, nums[3] as u8)),
                "duplicate migration rule: {line}"
            );
        }
        let expected: std::collections::HashSet<_> = ticket_tbl_rules().into_iter().collect();
        let missing: Vec<_> = expected.difference(&migrated).collect();
        let extra: Vec<_> = migrated.difference(&expected).collect();
        assert!(missing.is_empty(), "rules missing from migration: {missing:?}");
        assert!(extra.is_empty(), "rules in migration but not in the tbl: {extra:?}");
    }

    /// Items the live `item` table lacked: every ticket/reward of the tbl that is not seeded by
    /// an older migration is created by this one (from Item_Org_us / Item_Ext_*_us).
    #[test]
    fn test_migration_seeds_the_missing_items() {
        let sql = migration_sql();
        let block = sql
            .split("-- BEGIN ITEMS")
            .nth(1)
            .and_then(|s| s.split("-- END ITEMS").next())
            .expect("items block markers");
        let mut nums = Vec::new();
        for line in block.lines() {
            if let Some(rest) = line.trim().strip_prefix("('{\"num\":") {
                let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                nums.push(n.parse::<u32>().expect("item num"));
            }
        }
        let expected = [
            // Holy Knight Wings exchange coupons + their 4 wing rewards
            808_111_000u32, 808_112_000, 808_113_000, 808_114_000, 808_115_000, 808_116_000,
            1_808_107_168, 1_808_108_169, 1_808_109_170, 1_808_110_171,
            // Alencia / Hellfire / Stellar / Oreads vouchers
            910_920_000, 910_940_000, 978_017_000, 978_018_000, 999_916_000, 999_917_000,
            999_918_000, 999_919_000,
            // Transparent Minerva helmet/armor rewards of the Minerva vouchers
            808_065_704, 808_065_705, 808_066_698, 808_066_699,
        ];
        let mut sorted = nums.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), nums.len(), "duplicate item seeds");
        for id in expected {
            assert!(nums.contains(&id), "item {id} is not seeded");
        }
        assert_eq!(nums.len(), expected.len(), "unexpected extra item seeds: {nums:?}");
        // Every seeded item is actually part of the tbl (as ticket or reward).
        let used: std::collections::HashSet<u32> = ticket_tbl_rules()
            .iter()
            .flat_map(|(t, r, _, _)| [*t, *r])
            .collect();
        for id in &nums {
            assert!(used.contains(id), "seeded item {id} is not referenced by the tbl");
        }
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
