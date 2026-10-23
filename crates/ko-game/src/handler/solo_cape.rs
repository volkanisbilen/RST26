//! Solo Cape: a character-bound cape (not tied to a clan) with its own bonuses.
//!
//! Flow (client side in `cape_gate.cpp`, `rce_store.cpp`, `solocape_icon.cpp`):
//! 1. Right click on the Solo Cape Scroll (item 810521000) → hook `0xED` op 3 → the server opens the
//!    native cape shop (`KNIGHTS_CAPE_NPC`), the DLL lists only the Solo designs.
//! 2. Buy in that shop → `WIZ_CAPE`; for a Solo design the server takes the scroll, stores the cape for
//!    the character and answers like a cape purchase.
//! 3. Every attach sends the top-bar icon (hook `0xF3`): `[u8 show][i16 cape][u8 r][u8 g][u8 b][str name]
//!    [u8 n] n x ([u8 colour][str line])`. A double click on it sends `[0xE9][0xF3][1]` and removes the cape.
//!
//! Bonuses: +300 max HP, +5 % attack, +5 % defence (see `World::set_user_ability`).

use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use ko_protocol::{Opcode, Packet, PacketReader};
use sqlx::Row;
use tracing::warn;

use crate::session::ClientSession;
use crate::world::WorldState;
use crate::zone::SessionId;

pub const EXT_SUB_SOLOCAPEICON: u8 = 0xF3;
pub const EXT_SUB_HGGENIE: u8 = 0xED;
pub const SCROLL_ITEM: u32 = 810521000;
/// "Superior Cape Exchange Voucher": also pays a Solo Cape for a player outside a clan (or a non-leader) at the cape NPC.
pub const VOUCHER_ITEM: u32 = 914006000;
const KNIGHTS_CAPE_NPC: u8 = 0x1B;
const WIZ_CAPE: u8 = 0x70;

/// Design numbers and names (the same list as the client's `cape_gate.cpp`); a cape id is
/// `design + 100 * pattern`, pattern 0..16.
const DESIGNS: [(u16, &str); 19] = [
    (55, "Solo Crimson"),
    (56, "Solo Flame"),
    (57, "Solo Viking"),
    (58, "Solo Demon"),
    (59, "Solo Bronze"),
    (81, "Solo Trabzonspor"),
    (82, "Solo Fenerbahce"),
    (83, "Solo Besiktas"),
    (84, "Solo Galatasaray"),
    (85, "Solo Blue Serpent"),
    (86, "Solo Gold Serpent"),
    (87, "Solo Flag"),
    (88, "Solo Dragon Scale"),
    (89, "Solo White Knight"),
    (90, "Solo Shadow"),
    (91, "Solo Royal Blue"),
    (92, "Solo Midnight"),
    (93, "Solo Crimson Velvet"),
    (94, "Solo Ragged"),
];
const PATTERNS: i16 = 17;

pub const BONUS_HP: i16 = 300;
pub const BONUS_ATTACK_PCT: u8 = 5;
pub const BONUS_DEFENCE_PCT: i32 = 5;

/// character name → (cape id, r, g, b)
static CAPES: LazyLock<DashMap<String, (u16, u8, u8, u8)>> = LazyLock::new(DashMap::new);

pub fn get(name: &str) -> Option<(u16, u8, u8, u8)> {
    CAPES.get(name).map(|c| *c)
}

pub fn has(name: &str) -> bool {
    CAPES.contains_key(name)
}

pub fn is_solo_cape_id(cape_id: i16) -> bool {
    if cape_id < 0 || cape_id / 100 >= PATTERNS {
        return false;
    }
    let design = (cape_id % 100) as u16;
    DESIGNS.iter().any(|(d, _)| *d == design)
}

fn design_name(cape_id: u16) -> &'static str {
    let design = cape_id % 100;
    DESIGNS
        .iter()
        .find(|(d, _)| *d == design)
        .map(|(_, n)| *n)
        .unwrap_or("Solo Cape")
}

/// Load every character's Solo Cape at start-up (the table is small).
pub async fn load_all(pool: &ko_db::DbPool) {
    match sqlx::query("SELECT char_name, cape_id, r, g, b FROM solo_cape")
        .fetch_all(pool)
        .await
    {
        Ok(rows) => {
            for row in rows {
                let name: String = row.try_get("char_name").unwrap_or_default();
                let cape = row.try_get::<i32, _>("cape_id").unwrap_or(0) as u16;
                let r = row.try_get::<i16, _>("r").unwrap_or(0) as u8;
                let g = row.try_get::<i16, _>("g").unwrap_or(0) as u8;
                let b = row.try_get::<i16, _>("b").unwrap_or(0) as u8;
                if !name.is_empty() {
                    CAPES.insert(name, (cape, r, g, b));
                }
            }
            tracing::info!(capes = CAPES.len(), "solo capes loaded");
        }
        Err(e) => warn!("solo capes not loaded (run migration 20261014000003): {e}"),
    }
}

/// The icon packet for the top bar (`show = 0` hides it).
fn icon_packet(name: &str) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_SOLOCAPEICON);
    match get(name) {
        Some((cape, r, g, b)) => {
            pkt.write_u8(1);
            pkt.write_i16(cape as i16);
            pkt.write_u8(r);
            pkt.write_u8(g);
            pkt.write_u8(b);
            pkt.write_string(design_name(cape));
            let lines = [
                (1u8, format!("+{BONUS_HP} Max HP")),
                (1, format!("+{BONUS_ATTACK_PCT}% Attack")),
                (1, format!("+{BONUS_DEFENCE_PCT}% Defense")),
            ];
            pkt.write_u8(lines.len() as u8);
            for (colour, line) in &lines {
                pkt.write_u8(*colour);
                pkt.write_string(line);
            }
        }
        None => {
            pkt.write_u8(0);
            pkt.write_i16(-1);
            pkt.write_u8(0);
            pkt.write_u8(0);
            pkt.write_u8(0);
            pkt.write_string("");
            pkt.write_u8(0);
        }
    }
    pkt
}

/// Send the icon state to the owner (login, purchase, removal).
pub fn send_icon(world: &WorldState, sid: SessionId) {
    if let Some(ch) = world.get_character_info(sid) {
        if has(&ch.name) {
            world.send_to_session_owned(sid, icon_packet(&ch.name));
        }
    }
}

/// Show the new cape to everybody nearby.
fn refresh_appearance(world: &WorldState, sid: SessionId) {
    let Some((pos, ch, event_room)) = world.with_session(sid, |h| (h.position, h.character.clone(), h.event_room))
    else {
        return;
    };
    let Some(ch) = ch else {
        return;
    };
    let out = super::region::build_user_inout(super::region::INOUT_OUT, sid, Some(&ch), &pos);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(out),
        Some(sid),
        event_room,
    );
    let warp = super::region::build_user_inout(super::region::INOUT_WARP, sid, Some(&ch), &pos);
    world.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(warp),
        None,
        event_room,
    );
}

/// Hook 0xED (JstKO genie channel): op 3 = open the cape shop from the Solo Cape Scroll.
pub async fn handle_hggenie(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut r = PacketReader::new(&pkt.data);
    let op = r.read_u8().unwrap_or(0);
    if op == 5 {
        // genie window feedback: the buttons of the JstKO genie answer with a chat line
        use super::operator::GmCtx;
        let text = match r.read_u8().unwrap_or(0) {
            1 => "[Genie] Ayarlar kaydedildi.",
            2 => "[Genie] Yaratik listeye eklendi.",
            3 => "[Genie] Yaratik listeden silindi.",
            4 => "[Genie] Once bir yaratik hedefle (Z / tikla), sonra + tusuna bas.",
            5 => "[Genie] Silinecek yaratik yok.",
            _ => return Ok(()),
        };
        session.say(text);
        return Ok(());
    }
    if op != 3 {
        return Ok(());
    }
    let world = session.world().clone();
    let sid = session.session_id();
    if !super::class_change::has_item_in_inventory(&world, sid, SCROLL_ITEM) {
        return Ok(());
    }
    let mut open = Packet::new(Opcode::WizKnightsProcess as u8);
    open.write_u8(KNIGHTS_CAPE_NPC);
    session.send_packet(&open).await
}

/// Hook 0xF3: op 1 = remove the cape (double click on the top-bar icon).
pub async fn handle_icon(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut r = PacketReader::new(&pkt.data);
    if r.read_u8().unwrap_or(0) != 1 {
        return Ok(());
    }
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else {
        return Ok(());
    };
    if CAPES.remove(&ch.name).is_none() {
        return Ok(());
    }
    let _ = sqlx::query("DELETE FROM solo_cape WHERE char_name = $1")
        .bind(&ch.name)
        .execute(session.pool())
        .await;
    world.set_user_ability(sid);
    refresh_appearance(&world, sid);
    session.send_packet(&icon_packet(&ch.name)).await
}

/// `WIZ_CAPE` for a Solo design: take the scroll, store the cape, show it. Returns `true` when the
/// request was a Solo purchase (answered here), `false` to let the clan cape flow continue.
pub async fn try_buy(
    session: &mut ClientSession,
    cape_id: i16,
    r: u8,
    g: u8,
    b: u8,
    is_leader: bool,
) -> anyhow::Result<bool> {
    if !is_solo_cape_id(cape_id) {
        return Ok(false);
    }
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else {
        return Ok(true);
    };
    let has_scroll = super::class_change::has_item_in_inventory(&world, sid, SCROLL_ITEM);
    // the voucher only counts for players who cannot buy a clan cape anyway (non-leaders)
    let has_voucher = !is_leader && super::class_change::has_item_in_inventory(&world, sid, VOUCHER_ITEM);
    if is_leader && !has_scroll {
        return Ok(false); // a clan leader buying at the cape NPC: normal clan cape
    }
    let fail = |code: i16| {
        let mut p = Packet::new(WIZ_CAPE);
        p.write_i16(code);
        p
    };
    let paid = if has_scroll {
        world.rob_item(sid, SCROLL_ITEM, 1)
    } else if has_voucher {
        world.rob_item(sid, VOUCHER_ITEM, 1)
    } else {
        false
    };
    if !paid {
        session.send_packet(&fail(-10)).await?;
        return Ok(true);
    }
    CAPES.insert(ch.name.clone(), (cape_id as u16, r, g, b));
    let _ = sqlx::query(
        "INSERT INTO solo_cape(char_name, cape_id, r, g, b) VALUES($1,$2,$3,$4,$5) \
         ON CONFLICT (char_name) DO UPDATE SET cape_id=$2, r=$3, g=$4, b=$5",
    )
    .bind(&ch.name)
    .bind(cape_id as i32)
    .bind(r as i16)
    .bind(g as i16)
    .bind(b as i16)
    .execute(session.pool())
    .await;
    world.set_user_ability(sid);

    // the buyer's own client: the same answer as a clan cape purchase
    let mut ok = Packet::new(WIZ_CAPE);
    ok.write_u16(1);
    ok.write_u16(0);
    ok.write_u16(ch.knights_id);
    ok.write_u16(cape_id as u16);
    ok.write_u8(r);
    ok.write_u8(g);
    ok.write_u8(b);
    ok.write_u8(0);
    session.send_packet(&ok).await?;
    refresh_appearance(&world, sid);
    session.send_packet(&icon_packet(&ch.name)).await?;
    warn!("[{}] solo cape {} bought by {}", session.addr(), cape_id, ch.name);
    Ok(true)
}
