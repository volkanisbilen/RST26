//! "Pazar Botları" page: the system merchant stalls (quest items in the castles / Eslant, fragments, gems and
//! +5 gear in Moradon, see `crates/ko-game/src/systems/system_merchant.rs`). A *group* is a place (zone, anchor,
//! nation, name prefix); its *pool* is the list of items it sells -- every 12 pool items open one more stall.
//! Saving queues a `reload` / `system_merchants` admin_command, the game server rebuilds the stalls in seconds.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::legacy::{audit, auth, queue_command, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/merchant-bots", get(list).post(create_group))
        .route("/jstkoadminpanel/api/merchant-bots/reload", post(reload))
        .route("/jstkoadminpanel/api/merchant-bots/sales", get(sales))
        .route("/jstkoadminpanel/api/merchant-bots/{id}", post(save_group).delete(delete_group))
        .route("/jstkoadminpanel/api/merchant-bots/{id}/pool", post(save_pool))
        .route("/jstkoadminpanel/api/merchant-bots/{id}/fill-quest", post(fill_quest))
}

type R = Result<Json<Value>, ApiError>;

/// Zones a group may stand in (zone id, label).
const ZONES: [(i32, &str); 5] = [(1, "Luferson Castle"), (2, "El Morad Castle"), (11, "Karus Eslant"), (12, "El Morad Eslant"), (21, "Moradon")];
const MAX_POOL: usize = 400;
const MAX_PRICE: i64 = 2_000_000_000;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

async fn queue_reload(app: &App) -> Result<i64, ApiError> {
    queue_command(&app.pool, "reload", json!({"what": "system_merchants"})).await
}

async fn list(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let groups = sqlx::query(
        "SELECT id, key, title, kind, zone, nation, anchor_x, anchor_z, columns, spacing, name_prefix, advert, max_stalls, enabled, status, live_stalls, \
         COALESCE(EXTRACT(EPOCH FROM (NOW() - status_at))::float8, -1) AS status_age FROM system_merchant_group ORDER BY id",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("merchant-bots groups"))?;
    let pool_rows = sqlx::query(
        "SELECT p.id, p.group_id, p.item_id, p.price, p.stock, p.is_kc, p.enabled, COALESCE(i.str_name, '') AS name, COALESCE(i.item_icon_id1, 0) AS icon, \
         COALESCE(i.race, 0) AS race, COALESCE(i.countable, 0) AS countable, i.num IS NOT NULL AS known \
         FROM system_merchant_pool p LEFT JOIN item i ON i.num = p.item_id ORDER BY p.group_id, p.sort, p.id",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("merchant-bots pool"))?;
    let stats = sqlx::query(
        "SELECT group_key, COUNT(*)::bigint AS n, COALESCE(SUM(CASE WHEN is_kc THEN 0 ELSE price END), 0)::bigint AS gold, \
         COALESCE(SUM(CASE WHEN is_kc THEN price ELSE 0 END), 0)::bigint AS kc, \
         COUNT(*) FILTER (WHERE at > NOW() - INTERVAL '24 hours')::bigint AS n24 \
         FROM system_merchant_sale GROUP BY group_key",
    )
    .fetch_all(&app.pool)
    .await
    .unwrap_or_default();
    let hb: Option<f64> = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM (NOW() - at))::float8 FROM admin_heartbeat WHERE id=1")
        .fetch_optional(&app.pool)
        .await
        .ok()
        .flatten();

    let out: Vec<Value> = groups
        .iter()
        .map(|g| {
            let id: i32 = g.try_get("id").unwrap_or(0);
            let key: String = g.try_get("key").unwrap_or_default();
            let items: Vec<Value> = pool_rows
                .iter()
                .filter(|p| p.try_get::<i32, _>("group_id").unwrap_or(-1) == id)
                .map(|p| {
                    json!({
                        "item_id": p.try_get::<i32, _>("item_id").unwrap_or(0),
                        "name": p.try_get::<String, _>("name").unwrap_or_default(),
                        "icon": p.try_get::<i32, _>("icon").unwrap_or(0),
                        "price": p.try_get::<i64, _>("price").unwrap_or(1),
                        "stock": p.try_get::<i32, _>("stock").unwrap_or(1),
                        "is_kc": p.try_get::<bool, _>("is_kc").unwrap_or(false),
                        "enabled": p.try_get::<bool, _>("enabled").unwrap_or(true),
                        "tradable": p.try_get::<i32, _>("race").unwrap_or(0) != 20 && p.try_get::<i32, _>("countable").unwrap_or(0) != 2,
                        "known": p.try_get::<bool, _>("known").unwrap_or(false),
                    })
                })
                .collect();
            let st = stats.iter().find(|s| s.try_get::<String, _>("group_key").unwrap_or_default() == key);
            json!({
                "id": id, "key": key,
                "title": g.try_get::<String, _>("title").unwrap_or_default(),
                "kind": g.try_get::<String, _>("kind").unwrap_or_default(),
                "zone": g.try_get::<i32, _>("zone").unwrap_or(0),
                "nation": g.try_get::<i16, _>("nation").unwrap_or(0),
                "anchor_x": g.try_get::<f32, _>("anchor_x").unwrap_or(0.0),
                "anchor_z": g.try_get::<f32, _>("anchor_z").unwrap_or(0.0),
                "columns": g.try_get::<i16, _>("columns").unwrap_or(6),
                "spacing": g.try_get::<f32, _>("spacing").unwrap_or(3.0),
                "name_prefix": g.try_get::<String, _>("name_prefix").unwrap_or_default(),
                "advert": g.try_get::<String, _>("advert").unwrap_or_default(),
                "max_stalls": g.try_get::<i16, _>("max_stalls").unwrap_or(12),
                "enabled": g.try_get::<bool, _>("enabled").unwrap_or(false),
                "status": g.try_get::<String, _>("status").unwrap_or_default(),
                "live_stalls": g.try_get::<i32, _>("live_stalls").unwrap_or(0),
                "status_age": g.try_get::<f64, _>("status_age").unwrap_or(-1.0),
                "sales_total": st.map(|s| s.try_get::<i64, _>("n").unwrap_or(0)).unwrap_or(0),
                "sales_24h": st.map(|s| s.try_get::<i64, _>("n24").unwrap_or(0)).unwrap_or(0),
                "gold_total": st.map(|s| s.try_get::<i64, _>("gold").unwrap_or(0)).unwrap_or(0),
                "kc_total": st.map(|s| s.try_get::<i64, _>("kc").unwrap_or(0)).unwrap_or(0),
                "items": items,
            })
        })
        .collect();
    Ok(Json(json!({
        "groups": out,
        "zones": ZONES.iter().map(|(z, n)| json!({"zone": z, "name": n})).collect::<Vec<_>>(),
        "server_online": hb.is_some_and(|age| age < 20.0),
    })))
}

#[derive(Deserialize)]
struct GroupIn {
    title: String,
    zone: i32,
    nation: i16,
    anchor_x: f32,
    anchor_z: f32,
    columns: i16,
    spacing: f32,
    name_prefix: String,
    advert: String,
    max_stalls: i16,
    enabled: bool,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    kind: Option<String>,
}

fn valid_group(p: &GroupIn) -> bool {
    ZONES.iter().any(|(z, _)| *z == p.zone)
        && (0..=2).contains(&p.nation)
        && p.anchor_x.is_finite()
        && p.anchor_z.is_finite()
        && (0.0..=4096.0).contains(&p.anchor_x)
        && (0.0..=4096.0).contains(&p.anchor_z)
        && (1..=20).contains(&p.columns)
        && (1.5..=20.0).contains(&p.spacing)
        && (0..=40).contains(&p.max_stalls)
        && !p.title.trim().is_empty()
        && p.title.chars().count() <= 80
        && !p.name_prefix.trim().is_empty()
        && p.name_prefix.chars().count() <= 14
        && p.name_prefix.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && p.advert.chars().count() <= 60
}

async fn create_group(headers: HeaderMap, ConnectInfo(peer): ConnectInfo<SocketAddr>, State(app): State<App>, Json(p): Json<GroupIn>) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid_group(&p) {
        return Err(ApiError::BadRequest);
    }
    let key = p
        .key
        .clone()
        .filter(|k| !k.is_empty() && k.len() <= 32 && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or_else(|| format!("grp_{}", chrono_free_stamp()));
    let kind = if p.kind.as_deref() == Some("quest") { "quest" } else { "custom" };
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let id: i32 = sqlx::query_scalar(
        "INSERT INTO system_merchant_group (key, title, kind, zone, nation, anchor_x, anchor_z, columns, spacing, name_prefix, advert, max_stalls, enabled) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) RETURNING id",
    )
    .bind(&key)
    .bind(p.title.trim())
    .bind(kind)
    .bind(p.zone)
    .bind(p.nation)
    .bind(p.anchor_x)
    .bind(p.anchor_z)
    .bind(p.columns)
    .bind(p.spacing)
    .bind(p.name_prefix.trim())
    .bind(p.advert.trim())
    .bind(p.max_stalls)
    .bind(p.enabled)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "merchant group create");
        ApiError::BadRequest
    })?;
    audit(&mut tx, &s.actor, "merchant_bots.group.create", &key, json!({"title": p.title, "zone": p.zone}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app).await?;
    Ok(Json(json!({"ok": true, "id": id})))
}

fn chrono_free_stamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

async fn save_group(headers: HeaderMap, ConnectInfo(peer): ConnectInfo<SocketAddr>, State(app): State<App>, Path(id): Path<i32>, Json(p): Json<GroupIn>) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid_group(&p) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let n = sqlx::query(
        "UPDATE system_merchant_group SET title=$2, zone=$3, nation=$4, anchor_x=$5, anchor_z=$6, columns=$7, spacing=$8, name_prefix=$9, advert=$10, max_stalls=$11, enabled=$12 WHERE id=$1",
    )
    .bind(id)
    .bind(p.title.trim())
    .bind(p.zone)
    .bind(p.nation)
    .bind(p.anchor_x)
    .bind(p.anchor_z)
    .bind(p.columns)
    .bind(p.spacing)
    .bind(p.name_prefix.trim())
    .bind(p.advert.trim())
    .bind(p.max_stalls)
    .bind(p.enabled)
    .execute(&mut *tx)
    .await
    .map_err(internal("merchant group save"))?
    .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    audit(&mut tx, &s.actor, "merchant_bots.group.save", &id.to_string(), json!({"title": p.title, "zone": p.zone, "enabled": p.enabled, "x": p.anchor_x, "z": p.anchor_z}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app).await?;
    Ok(Json(json!({"ok": true})))
}

async fn delete_group(headers: HeaderMap, ConnectInfo(peer): ConnectInfo<SocketAddr>, State(app): State<App>, Path(id): Path<i32>) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let key: Option<String> = sqlx::query_scalar("DELETE FROM system_merchant_group WHERE id=$1 RETURNING key")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal("merchant group delete"))?;
    let key = key.ok_or(ApiError::NotFound)?;
    audit(&mut tx, &s.actor, "merchant_bots.group.delete", &key, json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app).await?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct PoolItem {
    item_id: i32,
    price: i64,
    stock: i32,
    #[serde(default)]
    is_kc: bool,
    #[serde(default = "yes")]
    enabled: bool,
}
fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct PoolIn {
    items: Vec<PoolItem>,
}

async fn save_pool(headers: HeaderMap, ConnectInfo(peer): ConnectInfo<SocketAddr>, State(app): State<App>, Path(id): Path<i32>, Json(p): Json<PoolIn>) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.items.len() > MAX_POOL {
        return Err(ApiError::BadRequest);
    }
    let mut seen = std::collections::HashSet::new();
    for it in &p.items {
        if it.item_id <= 0 || !(1..=MAX_PRICE).contains(&it.price) || !(1..=9999).contains(&it.stock) || !seen.insert(it.item_id) {
            return Err(ApiError::BadRequest);
        }
    }
    // only real, tradable items may go on a stall
    let ids: Vec<i32> = p.items.iter().map(|i| i.item_id).collect();
    let ok: Vec<i32> = sqlx::query_scalar("SELECT num FROM item WHERE num = ANY($1) AND COALESCE(race,0) <> 20 AND COALESCE(countable,0) <> 2")
        .bind(&ids)
        .fetch_all(&app.pool)
        .await
        .map_err(internal("merchant pool check"))?;
    if ok.len() != ids.len() {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let exists: Option<i32> = sqlx::query_scalar("SELECT id FROM system_merchant_group WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal("merchant pool lock"))?;
    exists.ok_or(ApiError::NotFound)?;
    sqlx::query("DELETE FROM system_merchant_pool WHERE group_id=$1").bind(id).execute(&mut *tx).await.map_err(internal("merchant pool clear"))?;
    for (i, it) in p.items.iter().enumerate() {
        sqlx::query("INSERT INTO system_merchant_pool (group_id, sort, item_id, price, stock, is_kc, enabled) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(id)
            .bind(i as i32)
            .bind(it.item_id)
            .bind(it.price)
            .bind(it.stock)
            .bind(it.is_kc)
            .bind(it.enabled)
            .execute(&mut *tx)
            .await
            .map_err(internal("merchant pool insert"))?;
    }
    audit(&mut tx, &s.actor, "merchant_bots.pool.save", &id.to_string(), json!({"items": p.items.len()}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app).await?;
    Ok(Json(json!({"ok": true, "items": p.items.len()})))
}

/// Add every tradable item the quest NPCs of the group's zone ask for (existing pool rows keep their price / stock).
async fn fill_quest(headers: HeaderMap, ConnectInfo(peer): ConnectInfo<SocketAddr>, State(app): State<App>, Path(id): Path<i32>) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let zone: Option<i32> = sqlx::query_scalar("SELECT zone FROM system_merchant_group WHERE id=$1").bind(id).fetch_optional(&mut *tx).await.map_err(internal("fill zone"))?;
    let zone = zone.ok_or(ApiError::NotFound)?;
    let added = sqlx::query(
        "INSERT INTO system_merchant_pool (group_id, sort, item_id, price, stock, is_kc) \
         SELECT $1, COALESCE((SELECT MAX(sort) FROM system_merchant_pool WHERE group_id=$1), 0) + ROW_NUMBER() OVER (ORDER BY it.item_id), it.item_id, \
                LEAST(GREATEST(COALESCE(i.buy_price, 1), 1), 2000000000)::bigint, CASE WHEN COALESCE(i.countable, 0) = 1 THEN 500 ELSE 1 END, FALSE \
         FROM (SELECT DISTINCT o.it AS item_id FROM quest_helper h \
               JOIN npc_spawn s ON s.npc_id = h.s_npc_id AND s.is_monster = FALSE AND s.zone_id = $2 \
               JOIN item_exchange e ON e.n_index = h.n_exchange_index \
               CROSS JOIN LATERAL unnest(ARRAY[e.origin_item_num1, e.origin_item_num2, e.origin_item_num3, e.origin_item_num4, e.origin_item_num5]) AS o(it) \
               WHERE h.n_exchange_index > 0 AND o.it > 0 AND o.it < 900000000) it \
         JOIN item i ON i.num = it.item_id AND COALESCE(i.race, 0) <> 20 AND COALESCE(i.countable, 0) <> 2 \
         ON CONFLICT (group_id, item_id) DO NOTHING",
    )
    .bind(id)
    .bind(zone)
    .execute(&mut *tx)
    .await
    .map_err(internal("fill quest"))?
    .rows_affected();
    audit(&mut tx, &s.actor, "merchant_bots.pool.fill_quest", &id.to_string(), json!({"added": added}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app).await?;
    Ok(Json(json!({"ok": true, "added": added})))
}

async fn reload(headers: HeaderMap, ConnectInfo(peer): ConnectInfo<SocketAddr>, State(app): State<App>) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "merchant_bots.reload", "-", json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_reload(&app).await?;
    Ok(Json(json!({"ok": true, "id": id})))
}

#[derive(Deserialize)]
struct SalesQ {
    limit: Option<i64>,
}

async fn sales(headers: HeaderMap, State(app): State<App>, Query(q): Query<SalesQ>) -> R {
    auth(&headers, false)?;
    let rows = sqlx::query(
        "SELECT EXTRACT(EPOCH FROM s.at)::bigint AS at, s.group_key, s.stall, s.buyer, s.item_id, s.count, s.price, s.is_kc, COALESCE(i.str_name, '') AS name, COALESCE(i.item_icon_id1, 0) AS icon \
         FROM system_merchant_sale s LEFT JOIN item i ON i.num = s.item_id ORDER BY s.id DESC LIMIT $1",
    )
    .bind(q.limit.unwrap_or(40).clamp(1, 200))
    .fetch_all(&app.pool)
    .await
    .map_err(internal("merchant sales"))?;
    Ok(Json(json!(rows
        .iter()
        .map(|r| json!({
            "at": r.try_get::<i64, _>("at").unwrap_or(0),
            "group": r.try_get::<String, _>("group_key").unwrap_or_default(),
            "stall": r.try_get::<String, _>("stall").unwrap_or_default(),
            "buyer": r.try_get::<String, _>("buyer").unwrap_or_default(),
            "item_id": r.try_get::<i32, _>("item_id").unwrap_or(0),
            "name": r.try_get::<String, _>("name").unwrap_or_default(),
            "icon": r.try_get::<i32, _>("icon").unwrap_or(0),
            "count": r.try_get::<i32, _>("count").unwrap_or(0),
            "price": r.try_get::<i64, _>("price").unwrap_or(0),
            "is_kc": r.try_get::<bool, _>("is_kc").unwrap_or(false),
        }))
        .collect::<Vec<_>>())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group() -> GroupIn {
        GroupIn {
            title: "t".into(),
            zone: 21,
            nation: 0,
            anchor_x: 800.0,
            anchor_z: 600.0,
            columns: 6,
            spacing: 3.0,
            name_prefix: "MorEsya".into(),
            advert: "x".into(),
            max_stalls: 4,
            enabled: true,
            key: None,
            kind: None,
        }
    }

    #[test]
    fn group_limits() {
        assert!(valid_group(&group()));
        let mut g = group();
        g.zone = 71;
        assert!(!valid_group(&g), "unknown zone");
        let mut g = group();
        g.name_prefix = "Bad Name!".into();
        assert!(!valid_group(&g), "stall names stay ascii");
        let mut g = group();
        g.spacing = 0.5;
        assert!(!valid_group(&g));
    }
}
