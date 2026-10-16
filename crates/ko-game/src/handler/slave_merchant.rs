//! Slave Merchant: a player who carries the Slave Scroll (item 910941725) and opens a merchant stall does not stand
//! at the stall himself. A copy of the character (a runtime "bot" with the player's looks, named `<name>_S`) takes the
//! stall over with the listed items; the player plays on. Everything the slave sells is paid to the owner at once
//! (gold / Knight Cash into the bag; when the owner is offline the money waits in `slave_merchant_credit` and is paid
//! at the next login). The owner closes the stall with the chat command `!slavekapat` (the unsold items come back to
//! the bag), `!slave` shows the state. The stall survives a server restart (table `slave_merchant`).

use std::sync::{Arc, LazyLock};

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
pub const SLAVE_ITEM: u32 = 910_941_725;

const MERCHANT_CLOSE: u8 = 2;
const SLOT_MAX: usize = 14;

/// bot id -> owner character name
static BY_BOT: LazyLock<DashMap<u32, String>> = LazyLock::new(DashMap::new);
/// lower-case owner name -> bot id
static BY_OWNER: LazyLock<DashMap<String, u32>> = LazyLock::new(DashMap::new);

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
                       "serial": m.serial_num, "price": m.price, "slot": m.original_slot, "sold": m.sold_out, "kc": m.is_kc})
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
    ] {
        if let Err(e) = sqlx::query(sql).execute(pool).await {
            warn!("slave merchant: table setup failed: {e}");
            return;
        }
    }
    let rows = match sqlx::query("SELECT owner, nation, race, class, level, face, hair, knights_id, fame, zone, x, y, z, advert, visual, items FROM slave_merchant")
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

fn add_credit(pool: &DbPool, owner: &str, gold: u32, kc: u32) {
    let pool = pool.clone();
    let owner = owner.to_string();
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO slave_merchant_credit(owner, gold, kc) VALUES($1,$2,$3) ON CONFLICT(owner) DO UPDATE SET gold = slave_merchant_credit.gold + $2, kc = slave_merchant_credit.kc + $3",
        )
        .bind(owner)
        .bind(gold as i64)
        .bind(kc as i64)
        .execute(&pool)
        .await;
    });
}

/// MERCHANT_INSERT hook: when the player carries the scroll, a slave takes the stall over. Returns `true` when it did.
pub fn try_convert(session: &mut ClientSession, advert: &str) -> bool {
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else { return false };
    let has_scroll = world
        .with_session(sid, |h| h.inventory.iter().any(|s| s.item_id == SLAVE_ITEM && s.count > 0))
        .unwrap_or(false);
    if !has_scroll {
        return false;
    }
    if BY_OWNER.contains_key(&ch.name.to_lowercase()) {
        notice(&world, sid, "[Slave] Zaten acik bir slave pazarin var. Once !slavekapat yaz.");
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
    notice(&world, sid, "[Slave] Pazarini slave kurdu. Satislarin parasi sana gelir. Kapatmak icin !slavekapat yaz.");
    info!(owner = %ch.name, bot_id, items = removed.len(), "slave merchant opened");
    true
}

/// Called after a buyer bought from a bot stall: pays the owner when the stall is a slave's.
pub fn on_sale(world: &WorldState, pool: &DbPool, bot_id: u32, price: u32, is_kc: bool) {
    let Some(owner) = BY_BOT.get(&bot_id).map(|e| e.clone()) else { return };
    if !is_kc {
        world.update_bot(bot_id, |b| b.gold = b.gold.saturating_sub(price));
    }
    match world.find_session_by_name(&owner) {
        Some(osid) => {
            if is_kc {
                crate::handler::knight_cash::cash_gain(world, pool, osid, price);
                notice(world, osid, &format!("[Slave] Satis: +{price} KC"));
            } else {
                world.gold_gain(osid, price);
                notice(world, osid, &format!("[Slave] Satis: +{price} coin"));
            }
        }
        None => {
            if is_kc {
                add_credit(pool, &owner, 0, price);
            } else {
                add_credit(pool, &owner, price, 0);
            }
        }
    }
    if let Some(bot) = world.get_bot(bot_id) {
        save_items(pool, &owner, &bot.merchant_items);
        if all_sold(&bot.merchant_items) {
            finish(world, pool, bot_id, &owner);
            if let Some(osid) = world.find_session_by_name(&owner) {
                notice(world, osid, "[Slave] Tum esyalar satildi, slave pazar kapandi.");
            }
        }
    }
}

fn finish(world: &WorldState, pool: &DbPool, bot_id: u32, owner: &str) {
    crate::systems::bot_ai::despawn_bot(world, bot_id);
    BY_BOT.remove(&bot_id);
    BY_OWNER.remove(&owner.to_lowercase());
    delete_row(pool, owner);
}

/// The money earned while the owner was away.
pub async fn pay_pending(session: &mut ClientSession) {
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else { return };
    let row = sqlx::query("DELETE FROM slave_merchant_credit WHERE owner = $1 RETURNING gold, kc")
        .bind(&ch.name)
        .fetch_optional(session.pool())
        .await;
    if let Ok(Some(r)) = row {
        let gold = r.try_get::<i64, _>("gold").unwrap_or(0).clamp(0, u32::MAX as i64) as u32;
        let kc = r.try_get::<i64, _>("kc").unwrap_or(0).clamp(0, u32::MAX as i64) as u32;
        if gold > 0 {
            world.gold_gain(sid, gold);
        }
        if kc > 0 {
            crate::handler::knight_cash::cash_gain(&world, session.pool(), sid, kc);
        }
        if gold > 0 || kc > 0 {
            notice(&world, sid, &format!("[Slave] Sen yokken slave pazarin kazandi: {gold} coin, {kc} KC."));
        }
    }
}

/// `!slave` (state) and `!slavekapat` (close and take the unsold items back).
pub async fn chat_command(session: &mut ClientSession, text: &str) -> anyhow::Result<bool> {
    let cmd = text.trim().to_lowercase();
    if cmd != "!slave" && cmd != "!slavekapat" {
        return Ok(false);
    }
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else { return Ok(true) };
    let Some(bot_id) = BY_OWNER.get(&ch.name.to_lowercase()).map(|e| *e) else {
        notice(&world, sid, "[Slave] Acik bir slave pazarin yok. Slave Scroll tasirken pazar kurarsan slave kurar.");
        return Ok(true);
    };
    let Some(bot) = world.get_bot(bot_id) else {
        BY_BOT.remove(&bot_id);
        BY_OWNER.remove(&ch.name.to_lowercase());
        return Ok(true);
    };
    if cmd == "!slave" {
        let left = bot.merchant_items.iter().filter(|m| m.item_id != 0 && !m.sold_out && m.sell_count > 0).count();
        notice(&world, sid, &format!("[Slave] Pazar acik, satilmayan {left} kalem var. Kapatmak icin !slavekapat."));
        return Ok(true);
    }
    // close: the unsold items go back to the owner's bag
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
        save_items(session.pool(), &ch.name, &items);
        notice(&world, sid, &format!("[Slave] Cantanda yer yok, {kept} kalem slave'de kaldi. Yer acip tekrar !slavekapat yaz."));
        return Ok(true);
    }
    finish(&world, session.pool(), bot_id, &ch.name);
    notice(&world, sid, "[Slave] Slave pazar kapandi, satilmayan esyalar cantana geri geldi.");
    let _ = Arc::strong_count(&world);
    Ok(true)
}
