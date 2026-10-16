//! Slave Merchant: a player who carries the Slave Scroll (item 910941725) and opens a merchant stall does not stand
//! at the stall himself. A copy of the character (a runtime "bot" with the player's looks, named `<name>_S`) takes the
//! stall over with the listed items; the player plays on. Everything the slave sells is paid to the owner at once
//! (gold / Knight Cash into the bag; when the owner is offline the money waits in `slave_merchant_credit` and is paid
//! at the next login). The owner closes the stall with the chat command `!slavekapat` (the unsold items come back to
//! the bag), `!slave` shows the state. The stall survives a server restart (table `slave_merchant`).
//!
//! Window (slavemerchant.uif, DLL slave_panel.cpp), hook sub-op 0xDD:
//!   S2C `[0][u8 show]`  the top-bar icon;
//!   S2C `[1][u8 active][u8 licence][u32 minutes left][u32 gold][u32 kc][u32 tl] 12 x [u32 item][u32 icon][u16 count] (the slave's stall)
//!        28 x [u32 item][u32 icon][u16 count] (the bag)`
//!   C2S `[1]` refresh, `[2]` take the KC, `[3]` take the gold, `[4]` close the stall, `[5]` use the scroll (one scroll = a week of use).

use std::sync::LazyLock;

use dashmap::DashMap;
use ko_db::DbPool;
use ko_protocol::{Opcode, Packet};
use serde_json::{json, Value};
use sqlx::Row;
use tracing::{info, warn};

use crate::session::ClientSession;
use crate::world::{
    BotAiState, BotInstance, BotPresence, MerchData, WorldState, MAX_MERCH_ITEMS,
};
use crate::zone::calc_region;

/// The item a player must carry for his stall to be taken over by a slave.
pub const SLAVE_ITEM: u32 = 810_107_000;
/// One scroll gives a week of slave merchant use.
const SUB_SECS: u64 = 7 * 24 * 3600;
/// lower-case owner -> unix seconds the slave merchant licence ends
static SUBS: LazyLock<DashMap<String, u64>> = LazyLock::new(DashMap::new);
static LAST_TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub const EXT_SUB_SLAVEMERC: u8 = 0xDD;
const MERCHANT_CLOSE: u8 = 2;
const SLOT_MAX: usize = 14;

/// bot id -> owner character name
static BY_BOT: LazyLock<DashMap<u32, String>> = LazyLock::new(DashMap::new);
/// lower-case owner name -> bot id
static BY_OWNER: LazyLock<DashMap<String, u32>> = LazyLock::new(DashMap::new);
/// bot id -> unix seconds the slave started
static STARTED: LazyLock<DashMap<u32, u64>> = LazyLock::new(DashMap::new);

fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn is_slave(bot_id: u32) -> bool {
    BY_BOT.contains_key(&bot_id)
}

fn notice(world: &WorldState, sid: crate::zone::SessionId, msg: &str) {
    world.send_to_session_owned(sid, crate::systems::timed_notice::build_notice_packet(8, msg));
}

fn items_json(items: &[MerchData; MAX_MERCH_ITEMS]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|m| {
                json!({"id": m.item_id, "dur": m.durability, "cnt": m.sell_count, "orig": m.original_count,
                       "serial": m.serial_num, "price": m.price, "slot": m.original_slot, "sold": m.sold_out, "kc": m.is_kc, "tl": m.is_tl})
            })
            .collect(),
    )
}

fn items_from_json(v: &Value) -> [MerchData; MAX_MERCH_ITEMS] {
    let mut out: [MerchData; MAX_MERCH_ITEMS] = Default::default();
    if let Some(a) = v.as_array() {
        for (i, e) in a.iter().take(MAX_MERCH_ITEMS).enumerate() {
            out[i] = MerchData {
                item_id: e["id"].as_u64().unwrap_or(0) as u32,
                durability: e["dur"].as_i64().unwrap_or(0) as i16,
                sell_count: e["cnt"].as_u64().unwrap_or(0) as u16,
                original_count: e["orig"].as_u64().unwrap_or(0) as u16,
                serial_num: e["serial"].as_u64().unwrap_or(0),
                price: e["price"].as_u64().unwrap_or(0) as u32,
                original_slot: e["slot"].as_u64().unwrap_or(0) as u8,
                sold_out: e["sold"].as_bool().unwrap_or(false),
                is_kc: e["kc"].as_bool().unwrap_or(false),
                is_tl: e["tl"].as_bool().unwrap_or(false),
            };
        }
    }
    out
}

fn all_sold(items: &[MerchData; MAX_MERCH_ITEMS]) -> bool {
    items.iter().all(|m| m.item_id == 0 || m.sold_out || m.sell_count == 0)
}

fn visual_json(v: &[(u32, i16, u8); 17]) -> Value {
    Value::Array(v.iter().map(|(i, d, f)| json!([i, d, f])).collect())
}

fn visual_from_json(v: &Value) -> [(u32, i16, u8); 17] {
    let mut out = [(0u32, 0i16, 0u8); 17];
    if let Some(a) = v.as_array() {
        for (i, e) in a.iter().take(17).enumerate() {
            out[i] = (e[0].as_u64().unwrap_or(0) as u32, e[1].as_i64().unwrap_or(0) as i16, e[2].as_u64().unwrap_or(0) as u8);
        }
    }
    out
}

struct Look {
    name: String,
    nation: u8,
    race: u8,
    class: u16,
    level: u8,
    face: u8,
    hair: u32,
    knights_id: u16,
    fame: u8,
    zone: u16,
    x: f32,
    y: f32,
    z: f32,
    advert: String,
    visual: [(u32, i16, u8); 17],
}

/// Spawn the slave bot (shared by a new stall and the restart restore) and show it to the players around.
fn spawn_bot(world: &WorldState, look: &Look, items: [MerchData; MAX_MERCH_ITEMS]) -> u32 {
    let id = world.alloc_bot_id();
    let rx = calc_region(look.x);
    let rz = calc_region(look.z);
    let now_unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let now_ms = now_unix * 1000;
    let mut bot = BotInstance {
        id,
        db_id: 0,
        name: look.name.chars().take(18).collect::<String>() + "_S",
        nation: look.nation,
        race: look.race,
        class: look.class,
        hair_rgb: look.hair,
        level: look.level,
        face: look.face,
        knights_id: look.knights_id,
        fame: look.fame,
        zone_id: look.zone,
        x: look.x,
        y: look.y,
        z: look.z,
        direction: 0,
        region_x: rx,
        region_z: rz,
        hp: 1000,
        max_hp: 1000,
        mp: 500,
        max_mp: 500,
        sp: 0,
        max_sp: 0,
        str_stat: 60,
        sta_stat: 60,
        dex_stat: 60,
        int_stat: 60,
        cha_stat: 60,
        gold: 0,
        loyalty: 0,
        loyalty_monthly: 0,
        loyalty_daily: 0,
        in_game: true,
        presence: BotPresence::Standing,
        ai_state: BotAiState::Merchant,
        target_id: -1,
        target_changed: false,
        spawned_at: now_unix,
        duration_minutes: 0, // never expires: the owner closes it
        last_tick_ms: now_ms,
        last_move_ms: 0,
        last_mining_ms: 0,
        last_merchant_chat_ms: 0,
        last_hp_change_ms: now_ms,
        last_regen_ms: now_ms,
        last_attacker_id: -1,
        skill_cooldown: [0; 2],
        last_type4_ms: now_ms,
        regene_at_ms: 0,
        original_ai_state: BotAiState::Merchant,
        move_route: 0,
        move_state: 0,
        merchant_state: 0, // selling
        premium_merchant: false,
        merchant_chat: look.advert.clone(),
        merchant_items: items,
        merchant_looker: None,
        reb_level: 0,
        cover_title: 0,
        rival_id: -1,
        rival_expiry_time: 0,
        anger_gauge: 0,
        hiding_helmet: false,
        hiding_cospre: false,
        need_party: 1,
        equip_visual: look.visual,
        personal_rank: 0,
        knights_rank: 0,
    };
    bot.max_hp = look.level.max(1) as i16 * 100;
    bot.hp = bot.max_hp;
    let inout = crate::systems::bot_ai::build_bot_inout_packet(&bot, world, 1);
    world.insert_bot(bot);
    crate::systems::bot_ai::broadcast_to_bot_region(world, look.zone, rx, rz, &inout);
    // the stall sign over the character
    let mut m = Packet::new(Opcode::WizMerchantInout as u8);
    m.write_u8(1);
    m.write_u16(1);
    m.write_u32(id);
    m.write_u8(0);
    m.write_u8(0);
    crate::systems::bot_ai::broadcast_to_bot_region(world, look.zone, rx, rz, &m);
    id
}

/// Create the tables and bring the saved slave stalls back after a restart.
pub async fn init(world: &WorldState, pool: &DbPool) {
    for sql in [
        "CREATE TABLE IF NOT EXISTS slave_merchant (
            owner VARCHAR(21) PRIMARY KEY, nation SMALLINT NOT NULL, race SMALLINT NOT NULL, class INTEGER NOT NULL,
            level SMALLINT NOT NULL, face SMALLINT NOT NULL, hair BIGINT NOT NULL, knights_id INTEGER NOT NULL DEFAULT 0,
            fame SMALLINT NOT NULL DEFAULT 0, zone INTEGER NOT NULL, x REAL NOT NULL, y REAL NOT NULL, z REAL NOT NULL,
            advert VARCHAR(100) NOT NULL DEFAULT '', visual JSONB NOT NULL, items JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())",
        "CREATE TABLE IF NOT EXISTS slave_merchant_credit (
            owner VARCHAR(21) PRIMARY KEY, gold BIGINT NOT NULL DEFAULT 0, kc BIGINT NOT NULL DEFAULT 0)",
        "ALTER TABLE slave_merchant_credit ADD COLUMN IF NOT EXISTS tl BIGINT NOT NULL DEFAULT 0",
        "CREATE TABLE IF NOT EXISTS slave_merchant_sub (
            owner VARCHAR(21) PRIMARY KEY, expires BIGINT NOT NULL DEFAULT 0)",
    ] {
        if let Err(e) = sqlx::query(sql).execute(pool).await {
            warn!("slave merchant: table setup failed: {e}");
            return;
        }
    }
    if let Ok(subs) = sqlx::query("SELECT owner, expires FROM slave_merchant_sub").fetch_all(pool).await {
        for r in subs {
            SUBS.insert(r.try_get::<String, _>("owner").unwrap_or_default().to_lowercase(), r.try_get::<i64, _>("expires").unwrap_or(0).max(0) as u64);
        }
    }
    let rows = match sqlx::query("SELECT owner, nation, race, class, level, face, hair, knights_id, fame, zone, x, y, z, advert, visual, items, EXTRACT(EPOCH FROM created_at)::BIGINT AS started FROM slave_merchant")
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("slave merchant: load failed: {e}");
            return;
        }
    };
    let mut n = 0;
    for r in rows {
        let owner: String = r.try_get("owner").unwrap_or_default();
        let items = items_from_json(&r.try_get::<Value, _>("items").unwrap_or(Value::Null));
        if all_sold(&items) {
            continue;
        }
        let look = Look {
            name: owner.clone(),
            nation: r.try_get::<i16, _>("nation").unwrap_or(1) as u8,
            race: r.try_get::<i16, _>("race").unwrap_or(1) as u8,
            class: r.try_get::<i32, _>("class").unwrap_or(0) as u16,
            level: r.try_get::<i16, _>("level").unwrap_or(1) as u8,
            face: r.try_get::<i16, _>("face").unwrap_or(1) as u8,
            hair: r.try_get::<i64, _>("hair").unwrap_or(0) as u32,
            knights_id: r.try_get::<i32, _>("knights_id").unwrap_or(0) as u16,
            fame: r.try_get::<i16, _>("fame").unwrap_or(0) as u8,
            zone: r.try_get::<i32, _>("zone").unwrap_or(21) as u16,
            x: r.try_get("x").unwrap_or(0.0),
            y: r.try_get("y").unwrap_or(0.0),
            z: r.try_get("z").unwrap_or(0.0),
            advert: r.try_get("advert").unwrap_or_default(),
            visual: visual_from_json(&r.try_get::<Value, _>("visual").unwrap_or(Value::Null)),
        };
        let id = spawn_bot(world, &look, items);
        BY_BOT.insert(id, owner.clone());
        BY_OWNER.insert(owner.to_lowercase(), id);
        STARTED.insert(id, r.try_get::<i64, _>("started").unwrap_or(0).max(0) as u64);
        n += 1;
    }
    if n > 0 {
        info!(count = n, "slave merchants restored");
    }
}

fn save_items(pool: &DbPool, owner: &str, items: &[MerchData; MAX_MERCH_ITEMS]) {
    let pool = pool.clone();
    let owner = owner.to_string();
    let v = items_json(items);
    tokio::spawn(async move {
        if let Err(e) = sqlx::query("UPDATE slave_merchant SET items = $2 WHERE owner = $1").bind(&owner).bind(v).execute(&pool).await {
            warn!("slave merchant: item save failed: {e}");
        }
    });
}

fn delete_row(pool: &DbPool, owner: &str) {
    let pool = pool.clone();
    let owner = owner.to_string();
    tokio::spawn(async move {
        let _ = sqlx::query("DELETE FROM slave_merchant WHERE owner = $1").bind(owner).execute(&pool).await;
    });
}

fn add_credit(pool: &DbPool, owner: &str, gold: u32, kc: u32, tl: u32) {
    let pool = pool.clone();
    let owner = owner.to_string();
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO slave_merchant_credit(owner, gold, kc, tl) VALUES($1,$2,$3,$4) ON CONFLICT(owner) DO UPDATE SET gold = slave_merchant_credit.gold + $2, kc = slave_merchant_credit.kc + $3, tl = slave_merchant_credit.tl + $4",
        )
        .bind(owner)
        .bind(gold as i64)
        .bind(kc as i64)
        .bind(tl as i64)
        .execute(&pool)
        .await;
    });
}

/// Seconds of slave merchant licence the character has left.
fn sub_left(name: &str) -> u64 {
    SUBS.get(&name.to_lowercase()).map(|e| e.saturating_sub(now_unix())).unwrap_or(0)
}

/// MERCHANT_INSERT hook: a player with an active Slave Merchant licence gets a slave at his stall. Returns `true` when it did.
pub fn try_convert(session: &mut ClientSession, advert: &str) -> bool {
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else { return false };
    if sub_left(&ch.name) == 0 {
        return false;
    }
    if BY_OWNER.contains_key(&ch.name.to_lowercase()) {
        notice(&world, sid, "[Slave] Zaten acik bir slave pazarin var. Slave Merchant penceresinden kapatabilirsin.");
        return false;
    }
    let stall = world.get_merchant_items(sid);
    let mut items: [MerchData; MAX_MERCH_ITEMS] = Default::default();
    let mut removed: Vec<usize> = Vec::new();
    for (i, m) in stall.iter().enumerate() {
        if m.item_id == 0 || m.sold_out || m.sell_count == 0 || m.price == 0 {
            continue;
        }
        let slot = m.original_slot as usize;
        let ok = world
            .get_inventory_slot(sid, slot)
            .map(|s| s.item_id == m.item_id && s.count >= m.sell_count)
            .unwrap_or(false);
        if !ok {
            continue;
        }
        items[i] = m.clone();
        removed.push(i);
    }
    if removed.is_empty() {
        return false;
    }
    // the items leave the owner's bag: the slave holds them now
    for &i in &removed {
        let m = items[i].clone();
        let slot = m.original_slot as usize;
        world.update_inventory(sid, |inv| {
            if slot < inv.len() && inv[slot].item_id == m.item_id {
                inv[slot].count = inv[slot].count.saturating_sub(m.sell_count);
                if inv[slot].count == 0 {
                    inv[slot] = crate::world::UserItemSlot::default();
                }
            }
            true
        });
        let now = world.get_inventory_slot(sid, slot).unwrap_or_default();
        let mut p = Packet::new(Opcode::WizItemCountChange as u8);
        p.write_u16(1);
        p.write_u8(1);
        p.write_u8(slot.saturating_sub(SLOT_MAX) as u8);
        p.write_u32(now.item_id);
        p.write_u32(now.count as u32);
        p.write_u8(0);
        p.write_u16(now.durability as u16);
        p.write_u32(0);
        world.send_to_session_owned(sid, p);
    }
    // the owner's own stall is closed
    world.close_merchant(sid);
    let mut close = Packet::new(Opcode::WizMerchant as u8);
    close.write_u8(MERCHANT_CLOSE);
    close.write_u32(sid as u32);
    world.send_to_session_owned(sid, close);

    let pos = world.get_position(sid).unwrap_or_default();
    let fame = ch.rank; // not used by the client for bots outside PK zones
    let look = Look {
        name: ch.name.clone(),
        nation: ch.nation,
        race: ch.race,
        class: ch.class,
        level: ch.level,
        face: ch.face,
        hair: ch.hair_rgb,
        knights_id: world.with_session(sid, |h| h.character.as_ref().map(|c| c.knights_id)).flatten().unwrap_or(0),
        fame: world.with_session(sid, |h| h.character.as_ref().map(|c| c.fame)).flatten().unwrap_or(fame),
        zone: pos.zone_id,
        x: pos.x + 1.5,
        y: pos.y,
        z: pos.z,
        advert: advert.chars().take(60).collect(),
        visual: crate::handler::region::get_equipped_visual(&world, sid),
    };
    let bot_id = spawn_bot(&world, &look, items.clone());
    BY_BOT.insert(bot_id, ch.name.clone());
    BY_OWNER.insert(ch.name.to_lowercase(), bot_id);
    STARTED.insert(bot_id, now_unix());

    let pool = session.pool().clone();
    let (owner, vis, itv) = (ch.name.clone(), visual_json(&look.visual), items_json(&items));
    let (nation, race, class, level, face, hair, kid, fame, zone, x, y, z, adv) = (
        look.nation as i16, look.race as i16, look.class as i32, look.level as i16, look.face as i16, look.hair as i64,
        look.knights_id as i32, look.fame as i16, look.zone as i32, look.x, look.y, look.z, look.advert.clone(),
    );
    tokio::spawn(async move {
        let r = sqlx::query(
            "INSERT INTO slave_merchant(owner,nation,race,class,level,face,hair,knights_id,fame,zone,x,y,z,advert,visual,items) \
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) \
             ON CONFLICT(owner) DO UPDATE SET items = EXCLUDED.items, advert = EXCLUDED.advert, x = EXCLUDED.x, z = EXCLUDED.z, zone = EXCLUDED.zone",
        )
        .bind(&owner).bind(nation).bind(race).bind(class).bind(level).bind(face).bind(hair).bind(kid).bind(fame)
        .bind(zone).bind(x).bind(y).bind(z).bind(adv).bind(vis).bind(itv)
        .execute(&pool)
        .await;
        if let Err(e) = r {
            warn!("slave merchant: save failed for {owner}: {e}");
        }
    });
    notice(&world, sid, "[Slave] Pazarini slave kurdu. Kazanc Slave Merchant penceresinde birikir, oradan alirsin.");
    info!(owner = %ch.name, bot_id, items = removed.len(), "slave merchant opened");
    true
}

/// Called after a buyer bought from a bot stall: pays the owner when the stall is a slave's.
/// `currency`: 0 coins, 1 Knight Cash, 2 TL.
pub fn on_sale(world: &WorldState, pool: &DbPool, bot_id: u32, price: u32, currency: u8) {
    let Some(owner) = BY_BOT.get(&bot_id).map(|e| e.clone()) else { return };
    if currency == 0 {
        world.update_bot(bot_id, |b| b.gold = b.gold.saturating_sub(price));
    }
    // the earnings pile up for the owner; the Slave Merchant window pays them out
    match currency {
        2 => add_credit(pool, &owner, 0, 0, price),
        1 => add_credit(pool, &owner, 0, price, 0),
        _ => add_credit(pool, &owner, price, 0, 0),
    }
    if let Some(osid) = world.find_session_by_name(&owner) {
        notice(world, osid, &format!("[Slave] Satis: +{price} {}", match currency { 2 => "TL", 1 => "KC", _ => "coin" }));
    }
    if let Some(bot) = world.get_bot(bot_id) {
        save_items(pool, &owner, &bot.merchant_items);
        if all_sold(&bot.merchant_items) {
            finish(world, pool, bot_id, &owner);
            if let Some(osid) = world.find_session_by_name(&owner) {
                notice(world, osid, "[Slave] Tum esyalar satildi, slave pazar kapandi. Kazancini Slave Merchant penceresinden al.");
            }
        }
    }
}

fn finish(world: &WorldState, pool: &DbPool, bot_id: u32, owner: &str) {
    crate::systems::bot_ai::despawn_bot(world, bot_id);
    BY_BOT.remove(&bot_id);
    BY_OWNER.remove(&owner.to_lowercase());
    STARTED.remove(&bot_id);
    delete_row(pool, owner);
}

fn item_cell(world: &WorldState, id: u32, count: u32) -> (u32, u32, u16) {
    if id == 0 || count == 0 {
        return (0, 0, 0);
    }
    let icon = world.get_item(id).and_then(|i| i.item_icon_id1).unwrap_or(0).max(0) as u32;
    (id, icon, count.min(u16::MAX as u32) as u16)
}

/// The top-bar icon (every player gets it at game entry).
pub fn send_icon(world: &WorldState, sid: crate::zone::SessionId) {
    let mut p = Packet::new(Opcode::EXT_HOOK_S2C);
    p.write_u8(EXT_SUB_SLAVEMERC);
    p.write_u8(0);
    p.write_u8(1);
    world.send_to_session_owned(sid, p);
}

/// State of the window: the slave's stall, the bag and the money waiting for the owner.
async fn send_state(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else { return Ok(()) };
    let bot = BY_OWNER.get(&ch.name.to_lowercase()).and_then(|id| world.get_bot(*id));
    let (gold, kc, tl) = match sqlx::query("SELECT gold, kc, tl FROM slave_merchant_credit WHERE owner = $1")
        .bind(&ch.name)
        .fetch_optional(session.pool())
        .await
    {
        Ok(Some(r)) => (
            r.try_get::<i64, _>("gold").unwrap_or(0).clamp(0, u32::MAX as i64) as u32,
            r.try_get::<i64, _>("kc").unwrap_or(0).clamp(0, u32::MAX as i64) as u32,
            r.try_get::<i64, _>("tl").unwrap_or(0).clamp(0, u32::MAX as i64) as u32,
        ),
        _ => (0, 0, 0),
    };
    let minutes = bot
        .as_ref()
        .and_then(|b| STARTED.get(&b.id).map(|t| (now_unix().saturating_sub(*t) / 60) as u32))
        .unwrap_or(0);
    let mut p = Packet::new(Opcode::EXT_HOOK_S2C);
    p.write_u8(EXT_SUB_SLAVEMERC);
    p.write_u8(1);
    p.write_u8(u8::from(bot.is_some()));
    p.write_u8(u8::from(sub_left(&ch.name) > 0));
    let _ = minutes;
    p.write_u32((sub_left(&ch.name) / 60) as u32);
    p.write_u32(gold);
    p.write_u32(kc);
    p.write_u32(tl);
    for i in 0..MAX_MERCH_ITEMS {
        let (id, icon, cnt) = match &bot {
            Some(b) if !b.merchant_items[i].sold_out && b.merchant_items[i].sell_count > 0 => {
                item_cell(&world, b.merchant_items[i].item_id, b.merchant_items[i].sell_count as u32)
            }
            _ => (0, 0, 0),
        };
        p.write_u32(id);
        p.write_u32(icon);
        p.write_u16(cnt);
    }
    for i in 0..28 {
        let slot = world.get_inventory_slot(sid, SLOT_MAX + i).unwrap_or_default();
        let (id, icon, cnt) = item_cell(&world, slot.item_id, slot.count as u32);
        p.write_u32(id);
        p.write_u32(icon);
        p.write_u16(cnt);
    }
    session.send_packet(&p).await
}

/// Takes the KC or the gold the slave earned (the amount is read and zeroed in one statement).
async fn withdraw(session: &mut ClientSession, name: &str, col: &str) {
    let sql = format!(
        "UPDATE slave_merchant_credit c SET {col} = 0 FROM (SELECT {col} AS old FROM slave_merchant_credit WHERE owner = $1 FOR UPDATE) o \
         WHERE c.owner = $1 RETURNING o.old"
    );
    let world = session.world().clone();
    let sid = session.session_id();
    if let Ok(Some(r)) = sqlx::query(&sql).bind(name).fetch_optional(session.pool()).await {
        let v = r.try_get::<i64, _>("old").unwrap_or(0).clamp(0, u32::MAX as i64) as u32;
        if v == 0 {
            // nothing of this kind waits
        } else if col == "tl" {
            crate::handler::knight_cash::tl_gain(&world, session.pool(), sid, v);
            notice(&world, sid, &format!("[Slave] {v} TL alindi."));
        } else if col == "kc" {
            crate::handler::knight_cash::cash_gain(&world, session.pool(), sid, v);
            notice(&world, sid, &format!("[Slave] {v} KC alindi."));
        } else {
            world.gold_gain(sid, v);
            notice(&world, sid, &format!("[Slave] {v} coin alindi."));
        }
    }
}

/// Close the stall: the unsold items go back to the owner's bag.
async fn close_stall(session: &mut ClientSession, name: &str) {
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(bot_id) = BY_OWNER.get(&name.to_lowercase()).map(|e| *e) else {
        notice(&world, sid, "[Slave] Acik bir slave pazarin yok.");
        return;
    };
    let Some(bot) = world.get_bot(bot_id) else {
        BY_BOT.remove(&bot_id);
        BY_OWNER.remove(&name.to_lowercase());
        return;
    };
    let mut items = bot.merchant_items.clone();
    let mut kept = 0;
    for m in items.iter_mut() {
        if m.item_id == 0 || m.sold_out || m.sell_count == 0 {
            continue;
        }
        if world.give_item(sid, m.item_id, m.sell_count) {
            m.sold_out = true;
            m.sell_count = 0;
        } else {
            kept += 1;
        }
    }
    if kept > 0 {
        world.update_bot(bot_id, |b| b.merchant_items = items.clone());
        save_items(session.pool(), name, &items);
        notice(&world, sid, &format!("[Slave] Cantanda yer yok, {kept} kalem slave'de kaldi. Yer acip tekrar kapat."));
        return;
    }
    finish(&world, session.pool(), bot_id, name);
    notice(&world, sid, "[Slave] Slave pazar kapandi, satilmayan esyalar cantana geri geldi.");
}

/// Window requests (hook sub-op 0xDD, the data after the sub-op).
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let op = pkt.data.first().copied().unwrap_or(0);
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else { return Ok(()) };
    match op {
        2 => {
            withdraw(session, &ch.name, "kc").await;
            withdraw(session, &ch.name, "tl").await;
        }
        3 => withdraw(session, &ch.name, "gold").await,
        4 => close_stall(session, &ch.name).await,
        5 => {
            // "Use Slave Scroll": one scroll = a week of slave merchant use (it adds up when used again)
            if !world.take_one_item(sid, SLAVE_ITEM) {
                notice(&world, sid, "[Slave] Slave Merchant esyan yok. Power Up Store'dan alabilirsin.");
            } else {
                let base = SUBS.get(&ch.name.to_lowercase()).map(|e| *e).unwrap_or(0).max(now_unix());
                let exp = base + SUB_SECS;
                SUBS.insert(ch.name.to_lowercase(), exp);
                let pool = session.pool().clone();
                let owner = ch.name.clone();
                tokio::spawn(async move {
                    let _ = sqlx::query("INSERT INTO slave_merchant_sub(owner, expires) VALUES($1,$2) ON CONFLICT(owner) DO UPDATE SET expires = $2")
                        .bind(owner)
                        .bind(exp as i64)
                        .execute(&pool)
                        .await;
                });
                notice(&world, sid, "[Slave] Slave Merchant 1 hafta aktif. Pazar kurunca slave devralir.");
            }
        }
        _ => {}
    }
    send_state(session).await
}

/// Once a minute: a stall whose licence ended is closed and the unsold items go back to the owner (when he is online).
pub fn tick(world: &WorldState) {
    let now = now_unix();
    if now / 60 == LAST_TICK.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    LAST_TICK.store(now / 60, std::sync::atomic::Ordering::Relaxed);
    let Some(pool) = world.db_pool().cloned() else { return };
    let owners: Vec<(u32, String)> = BY_BOT.iter().map(|e| (*e.key(), e.value().clone())).collect();
    for (bot_id, owner) in owners {
        if sub_left(&owner) > 0 {
            continue;
        }
        let Some(osid) = world.find_session_by_name(&owner) else { continue };
        let Some(bot) = world.get_bot(bot_id) else { continue };
        let mut items = bot.merchant_items.clone();
        let mut kept = 0;
        for m in items.iter_mut() {
            if m.item_id == 0 || m.sold_out || m.sell_count == 0 {
                continue;
            }
            if world.give_item(osid, m.item_id, m.sell_count) {
                m.sold_out = true;
                m.sell_count = 0;
            } else {
                kept += 1;
            }
        }
        if kept > 0 {
            world.update_bot(bot_id, |b| b.merchant_items = items.clone());
            save_items(&pool, &owner, &items);
            notice(world, osid, "[Slave] Slave Merchant suresi bitti ama cantanda yer yok. Yer acinca pazar kapanir.");
        } else {
            finish(world, &pool, bot_id, &owner);
            notice(world, osid, "[Slave] Slave Merchant suren bitti, pazar kapandi, esyalar cantana geldi.");
        }
    }
}
