//! System merchants: stall bots run by the server. Their items come from the admin-edited pools
//! (`system_merchant_pool`, grouped by `system_merchant_group`): quest items in the castles / Eslant, fragments, gems and
//! +5 gear in Moradon. A pool of N items fills ceil(N / 12) stalls (a stall has 12 slots), side by side from the group's
//! anchor point. The stall never runs dry (every sale is restocked at once), the gold it earns is thrown away, every
//! sale is logged to `system_merchant_sale` for the admin page. Reloaded with the admin command `reload` / `system_merchants`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock};

use dashmap::DashMap;
use ko_db::DbPool;
use ko_protocol::{Opcode, Packet};
use sqlx::Row;
use tracing::{info, warn};

use crate::systems::bot_ai::{self, SpawnBotParams};
use crate::world::{BotAiState, MerchData, WorldState, MAX_MERCH_ITEMS};
use crate::zone::calc_region;

/// bot id -> index into DEFS
static BOTS: LazyLock<DashMap<u32, usize>> = LazyLock::new(DashMap::new);

#[derive(Clone)]
struct DefItem {
    item_id: u32,
    price: u32,
    stock: u16,
    is_kc: bool,
}

#[derive(Clone)]
struct StallDef {
    group_key: String,
    name: String,
    zone: u16,
    x: f32,
    z: f32,
    nation: u8,
    advert: String,
    items: Vec<DefItem>,
    look: usize,
}

static DEFS: LazyLock<RwLock<Vec<StallDef>>> = LazyLock::new(|| RwLock::new(Vec::new()));
static LAST_TICK: AtomicU64 = AtomicU64::new(0);

pub fn is_system(bot_id: u32) -> bool {
    BOTS.contains_key(&bot_id)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn slots(def: &StallDef, world: &WorldState) -> [MerchData; MAX_MERCH_ITEMS] {
    let mut out: [MerchData; MAX_MERCH_ITEMS] = Default::default();
    for (i, it) in def.items.iter().take(MAX_MERCH_ITEMS).enumerate() {
        let duration = world.get_item(it.item_id).map(|d| d.duration.unwrap_or(0)).unwrap_or(0);
        out[i] = MerchData {
            item_id: it.item_id,
            durability: duration as i16,
            sell_count: it.stock,
            original_count: it.stock,
            serial_num: 0,
            price: it.price,
            original_slot: i as u8,
            sold_out: false,
            is_kc: it.is_kc,
            is_tl: false,
        };
    }
    out
}

/// A spot for a stall: the wanted one if the map allows it, otherwise the nearest valid one (spiral search).
fn find_spot(world: &WorldState, zone: u16, x: f32, z: f32, taken: &[(f32, f32)]) -> Option<(f32, f32)> {
    let zone = world.get_zone(zone)?;
    let free = |px: f32, pz: f32| zone.is_valid_position(px, pz) && taken.iter().all(|(tx, tz)| (tx - px).abs() + (tz - pz).abs() >= 2.0);
    if free(x, z) {
        return Some((x, z));
    }
    for r in 1..=24 {
        let rf = r as f32;
        let n = 8 * r;
        for k in 0..n {
            let a = (k as f32) * std::f32::consts::TAU / (n as f32);
            let (px, pz) = (x + rf * a.cos(), z + rf * a.sin());
            if free(px, pz) {
                return Some((px, pz));
            }
        }
    }
    None
}

/// The bot template whose looks (race, class, gear) the stall copies: same nation, level 60+, never a merchant template.
fn look_row(world: &WorldState, nation: u8, pick: usize) -> Option<ko_db::models::bot_system::BotHandlerFarmRow> {
    let merchants: std::collections::HashSet<i32> = world.get_all_bot_merchant_data().into_iter().map(|m| m.n_index).collect();
    let mut rows: Vec<_> = world
        .get_all_bot_templates()
        .into_iter()
        .filter(|r| r.nation as u8 == nation && r.level >= 60 && !merchants.contains(&r.id))
        .collect();
    if rows.is_empty() {
        rows = world.get_all_bot_templates().into_iter().filter(|r| r.nation as u8 == nation).collect();
    }
    if rows.is_empty() {
        return None;
    }
    let n = rows.len();
    Some(rows.swap_remove(pick % n))
}

/// Spawn one stall; the bot id on success, the reason on failure.
fn spawn_stall(world: &WorldState, idx: usize, def: &StallDef) -> Result<u32, String> {
    let mut row = look_row(world, def.nation, def.look).ok_or_else(|| "bot şablonu yok".to_string())?;
    row.id = 0;
    row.str_user_id = def.name.clone();
    row.knights = 0;
    row.gold = 0;
    let taken: Vec<(f32, f32)> = {
        let defs = DEFS.read().map_err(|_| "kilit".to_string())?;
        BOTS.iter().filter(|e| *e.value() != idx).filter_map(|e| defs.get(*e.value()).filter(|d| d.zone == def.zone).map(|d| (d.x, d.z))).collect()
    };
    let (x, z) = find_spot(world, def.zone, def.x, def.z, &taken).ok_or_else(|| "geçerli yer bulunamadı".to_string())?;
    let items = slots(def, world);
    let id = bot_ai::spawn_farm_bot(
        world,
        &row,
        SpawnBotParams { zone_id: def.zone, x, y: 0.0, z, duration_minutes: 0, ai_state: BotAiState::Merchant },
    );
    world.update_bot(id, |b| {
        b.merchant_state = 0;
        b.premium_merchant = false;
        b.merchant_chat = def.advert.clone();
        b.merchant_items = items;
        b.move_route = 0;
        b.move_state = 0;
        b.gold = 0;
    });
    let (rx, rz) = (calc_region(x), calc_region(z));
    let mut m = Packet::new(Opcode::WizMerchantInout as u8);
    m.write_u8(1);
    m.write_u16(1);
    m.write_u32(id);
    m.write_u8(0);
    m.write_u8(0);
    bot_ai::broadcast_to_bot_region(world, def.zone, rx, rz, &m);
    Ok(id)
}

#[derive(sqlx::FromRow)]
struct GroupRow {
    id: i32,
    key: String,
    zone: i32,
    nation: i16,
    anchor_x: f32,
    anchor_z: f32,
    columns: i16,
    spacing: f32,
    name_prefix: String,
    advert: String,
    max_stalls: i16,
    mode: String,
    bot_count: i32,
    area_w: f32,
    area_h: f32,
    per_bot_min: i16,
    per_bot_max: i16,
}

async fn write_status(pool: &DbPool, id: i32, status: &str, live: usize) {
    let _ = sqlx::query("UPDATE system_merchant_group SET status = $2, live_stalls = $3, status_at = NOW() WHERE id = $1")
        .bind(id)
        .bind(status)
        .bind(live as i32)
        .execute(pool)
        .await;
}

fn despawn_all(world: &WorldState) {
    let ids: Vec<u32> = BOTS.iter().map(|e| *e.key()).collect();
    BOTS.clear();
    for id in ids {
        bot_ai::despawn_bot(world, id);
    }
}

/// A player-like name for a stall from the shared bot nickname pool (stable per group + stall number); `used` is updated.
fn pick_name(used: &mut std::collections::HashSet<String>, group: &str, n: usize, fallback_prefix: &str) -> String {
    let pool = bot_ai::bot_name_pool();
    if pool.is_empty() {
        return format!("{}{:02}", fallback_prefix.chars().take(14).collect::<String>(), n + 1);
    }
    let mut h: u64 = 1469598103934665603;
    for b in group.bytes().chain((n as u64).to_le_bytes()) {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    let start = (h % pool.len() as u64) as usize;
    for off in 0..pool.len() {
        let name: String = pool[(start + off) % pool.len()].chars().take(21).collect();
        if used.insert(name.to_lowercase()) {
            return name;
        }
    }
    let name = format!("{}{}", pool[start].chars().take(16).collect::<String>(), n + 2);
    used.insert(name.to_lowercase());
    name
}

/// (Re)build every stall from the tables. Returns a one-line summary.
pub async fn reload(world: &WorldState, pool: &DbPool) -> Result<String, String> {
    despawn_all(world);
    let groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, key, zone, nation, anchor_x, anchor_z, columns, spacing, name_prefix, advert, max_stalls, \
         mode, bot_count, area_w, area_h, per_bot_min, per_bot_max \
         FROM system_merchant_group WHERE enabled ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    // groups that are switched off say so
    let _ = sqlx::query("UPDATE system_merchant_group SET status = 'Kapalı', live_stalls = 0, status_at = NOW() WHERE NOT enabled").execute(pool).await;

    // names already taken: live bots, saved characters
    let mut used: std::collections::HashSet<String> = world.bots.iter().map(|e| e.value().name.to_lowercase()).collect();
    if let Ok(names) = sqlx::query_scalar::<_, String>("SELECT lower(str_user_id) FROM userdata").fetch_all(pool).await {
        used.extend(names);
    }
    let mut defs: Vec<StallDef> = Vec::new();
    let mut spans: Vec<(i32, usize, usize, Vec<String>)> = Vec::new(); // group id, first def, count, notes
    for g in &groups {
        let pool_rows = sqlx::query("SELECT item_id, price, stock, is_kc FROM system_merchant_pool WHERE group_id = $1 AND enabled ORDER BY sort, id")
            .bind(g.id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
        let mut items: Vec<DefItem> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        for r in &pool_rows {
            let item_id = r.try_get::<i32, _>("item_id").unwrap_or(0).max(0) as u32;
            if item_id == 0 || world.get_item(item_id).is_none() {
                notes.push(format!("item {item_id} tabloda yok"));
                continue;
            }
            items.push(DefItem {
                item_id,
                price: r.try_get::<i64, _>("price").unwrap_or(1).clamp(1, 2_000_000_000) as u32,
                stock: r.try_get::<i32, _>("stock").unwrap_or(1).clamp(1, 9999) as u16,
                is_kc: r.try_get::<bool, _>("is_kc").unwrap_or(false),
            });
        }
        let first = defs.len();
        let cols = g.columns.max(1) as usize;
        let nation_of = |n: usize| -> u8 {
            match g.nation {
                1 => 1u8,
                2 => 2u8,
                _ => {
                    if n % 2 == 0 {
                        2
                    } else {
                        1
                    }
                }
            }
        };
        if g.mode == "mixed" {
            // many stalls, each with its own random selection of the pool, standing on valid ground of the box
            let want = g.bot_count.clamp(0, 400) as usize;
            let mut spots: Vec<(f32, f32)> = Vec::new();
            if let Some(zone) = world.get_zone(g.zone as u16) {
                let step = g.spacing.max(1.5);
                let mut zz = g.anchor_z;
                while zz <= g.anchor_z + g.area_h && spots.len() < want {
                    let mut xx = g.anchor_x;
                    while xx <= g.anchor_x + g.area_w && spots.len() < want {
                        if zone.is_valid_position(xx, zz) {
                            spots.push((xx, zz));
                        }
                        xx += step;
                    }
                    zz += step;
                }
            }
            if spots.len() < want {
                notes.push(format!("kutuda yalnızca {} geçerli yer var ({} istendi) - alanı büyüt", spots.len(), want));
            }
            if items.is_empty() {
                notes.push("havuz boş".to_string());
            } else {
                use rand::seq::SliceRandom;
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let lo = g.per_bot_min.clamp(1, 12) as usize;
                let hi = g.per_bot_max.clamp(g.per_bot_min.max(1), 12) as usize;
                let mut sets: Vec<Vec<DefItem>> = (0..spots.len())
                    .map(|_| {
                        let k = rng.gen_range(lo..=hi).min(items.len());
                        items.choose_multiple(&mut rng, k).cloned().collect()
                    })
                    .collect();
                // every pool item is sold by at least one stall
                for it in &items {
                    if !sets.iter().any(|st| st.iter().any(|x| x.item_id == it.item_id)) {
                        let free: Vec<usize> = (0..sets.len()).filter(|&i| sets[i].len() < MAX_MERCH_ITEMS).collect();
                        if let Some(&i) = free.choose(&mut rng) {
                            sets[i].push(it.clone());
                        }
                    }
                }
                for (n, ((x, z), set)) in spots.iter().zip(sets).enumerate() {
                    defs.push(StallDef {
                        group_key: g.key.clone(),
                        name: pick_name(&mut used, &g.key, n, &g.name_prefix),
                        zone: g.zone as u16,
                        x: *x,
                        z: *z,
                        nation: nation_of(n),
                        advert: g.advert.chars().take(60).collect(),
                        items: set,
                        look: (g.id as usize) * 7 + n * 3,
                    });
                }
            }
        } else {
            for (n, chunk) in items.chunks(MAX_MERCH_ITEMS).enumerate().take(g.max_stalls.max(0) as usize) {
                defs.push(StallDef {
                    group_key: g.key.clone(),
                    name: pick_name(&mut used, &g.key, n, &g.name_prefix),
                    zone: g.zone as u16,
                    x: g.anchor_x + (n % cols) as f32 * g.spacing,
                    z: g.anchor_z + (n / cols) as f32 * g.spacing,
                    nation: nation_of(n),
                    advert: g.advert.chars().take(60).collect(),
                    items: chunk.to_vec(),
                    look: (g.id as usize) * 7 + n * 3,
                });
            }
            if items.len() > g.max_stalls.max(0) as usize * MAX_MERCH_ITEMS {
                notes.push(format!("havuzda {} item var, yalnızca ilk {} tanesi satılıyor (en çok {} stall)", items.len(), g.max_stalls as usize * MAX_MERCH_ITEMS, g.max_stalls));
            }
        }
        spans.push((g.id, first, defs.len() - first, notes));
    }
    *DEFS.write().map_err(|_| "kilit".to_string())? = defs.clone();

    let mut total = 0usize;
    for (gid, first, count, mut notes) in spans {
        let mut live = 0usize;
        for i in first..first + count {
            match spawn_stall(world, i, &defs[i]) {
                Ok(bot) => {
                    BOTS.insert(bot, i);
                    live += 1;
                }
                Err(e) => notes.push(format!("{}: {e}", defs[i].name)),
            }
        }
        total += live;
        let status = if count == 0 && notes.is_empty() {
            "Havuz boş - stall yok".to_string()
        } else if notes.is_empty() {
            format!("{live} stall açık")
        } else {
            format!("{live}/{count} stall açık - {}", notes.join("; "))
        };
        if live < count || !notes.is_empty() {
            warn!(group = gid, status = %status, "system merchants");
        }
        write_status(pool, gid, &status, live).await;
    }
    info!(stalls = total, "system merchants spawned");
    Ok(format!("{total} stalls"))
}

pub async fn init(world: &WorldState, pool: &DbPool) {
    if let Err(e) = reload(world, pool).await {
        warn!("system merchants: not loaded ({e}); run the system_merchants migration");
    }
}

/// A purchase from a system stall: restock it, throw the gold away, log the sale.
pub fn on_sale(world: &WorldState, pool: &DbPool, bot_id: u32, buyer: &str, item_id: u32, count: u16, price: u32, slot: usize) {
    let Some(idx) = BOTS.get(&bot_id).map(|e| *e.value()) else { return };
    world.restore_merchant_buy(bot_id as _, slot, item_id, count);
    world.update_bot(bot_id, |b| b.gold = 0);
    let (group, stall, is_kc) = {
        let Ok(defs) = DEFS.read() else { return };
        let Some(d) = defs.get(idx) else { return };
        (d.group_key.clone(), d.name.clone(), d.items.get(slot).is_some_and(|i| i.is_kc))
    };
    let pool = pool.clone();
    let buyer = buyer.to_string();
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO system_merchant_sale (group_key, stall, buyer, item_id, count, price, is_kc) VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(group)
        .bind(stall)
        .bind(buyer)
        .bind(item_id as i32)
        .bind(count as i32)
        .bind(price as i64)
        .bind(is_kc)
        .execute(&pool)
        .await;
    });
}

/// Once a minute: full stock everywhere, and a stall that disappeared (a "bot sil" style command) comes back.
pub fn tick(world: &WorldState) {
    let min = now_unix() / 60;
    if LAST_TICK.swap(min, Ordering::Relaxed) == min {
        return;
    }
    let defs: Vec<StallDef> = match DEFS.read() {
        Ok(d) => d.clone(),
        Err(_) => return,
    };
    if defs.is_empty() {
        return;
    }
    let alive: std::collections::HashSet<usize> = BOTS
        .iter()
        .filter(|e| world.get_bot(*e.key()).is_some())
        .map(|e| *e.value())
        .collect();
    BOTS.retain(|id, _| world.get_bot(*id).is_some());
    for (i, def) in defs.iter().enumerate() {
        if alive.contains(&i) {
            continue;
        }
        match spawn_stall(world, i, def) {
            Ok(bot) => {
                BOTS.insert(bot, i);
            }
            Err(e) => warn!(stall = %def.name, "system merchant respawn failed: {e}"),
        }
    }
    let ids: Vec<(u32, usize)> = BOTS.iter().map(|e| (*e.key(), *e.value())).collect();
    for (id, i) in ids {
        let items = slots(&defs[i], world);
        world.update_bot(id, |b| {
            b.merchant_items = items;
            b.gold = 0;
        });
    }
}
