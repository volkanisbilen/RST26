//! Chaotic Generator box management: which items are breakable in the in-game Chaotic
//! Generator (item.effect2 = 251, see bifrost_piece_exchange in ko-game's item_upgrade.rs)
//! and what each one's reward pool (item_exchange, random_flag IN (1,2,3)) contains.
//!
//! item_exchange also holds ~5000 unrelated crafting/exchange recipes, so this does NOT
//! reuse the generic Reward Center (rewards.rs) — it only ever touches rows that already
//! belong to a box (origin_item_num1 of an effect2=251 item) or that this module itself
//! creates for one.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::legacy::{audit, auth, queue_command, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/chaotic/boxes", get(boxes).post(enable_box))
        .route("/jstkoadminpanel/api/chaotic/boxes/{id}/disable", post(disable_box))
        .route(
            "/jstkoadminpanel/api/chaotic/boxes/{id}/rewards",
            get(rewards).post(add_reward),
        )
        .route(
            "/jstkoadminpanel/api/chaotic/boxes/{id}/rewards/{n_index}",
            axum::routing::delete(remove_reward).post(update_reward),
        )
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

async fn queue_reload(pool: &ko_db::DbPool) {
    let _ = queue_command(pool, "reload", json!({"what": "item_exchange"})).await;
}

/// Every item currently breakable in the Chaotic Generator (effect2 = 251), with its
/// reward-row count so the UI can flag an enabled box that has nothing in it yet.
async fn boxes(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let rows = sqlx::query(
        "SELECT i.num, i.str_name, i.item_icon_id1,
                (SELECT COUNT(*) FROM item_exchange ie WHERE ie.origin_item_num1 = i.num AND ie.random_flag IN (1,2,3)) reward_count
         FROM item i WHERE i.effect2 = 251 ORDER BY i.str_name",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("chaotic boxes"))?;
    let list: Vec<Value> = rows
        .iter()
        .map(|r| {
            use sqlx::Row;
            json!({
                "item_id": r.get::<i32, _>("num"),
                "name": r.try_get::<String, _>("str_name").unwrap_or_default(),
                "icon": r.try_get::<i32, _>("item_icon_id1").unwrap_or(0),
                "reward_count": r.get::<i64, _>("reward_count"),
            })
        })
        .collect();
    Ok(Json(json!({"boxes": list})))
}

#[derive(Deserialize)]
struct EnableIn {
    item_id: i32,
}

async fn enable_box(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<EnableIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let row = sqlx::query("SELECT str_name, countable, effect2 FROM item WHERE num = $1")
        .bind(p.item_id)
        .fetch_optional(&app.pool)
        .await
        .map_err(internal("chaotic item lookup"))?
        .ok_or(ApiError::NotFound)?;
    use sqlx::Row;
    let countable: i16 = row.try_get("countable").unwrap_or(0);
    let effect2: i32 = row.try_get("effect2").unwrap_or(0);
    if countable == 0 {
        return Err(ApiError::BadRequest);
    }
    if effect2 != 251 {
        sqlx::query("UPDATE item SET effect2 = 251 WHERE num = $1")
            .bind(p.item_id)
            .execute(&app.pool)
            .await
            .map_err(internal("chaotic enable"))?;
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "chaotic.box.enable", &p.item_id.to_string(), json!({"item_id": p.item_id}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true})))
}

async fn disable_box(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i32>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let done = sqlx::query("UPDATE item SET effect2 = 0 WHERE num = $1 AND effect2 = 251")
        .bind(id)
        .execute(&app.pool)
        .await
        .map_err(internal("chaotic disable"))?
        .rows_affected();
    if done == 0 {
        return Err(ApiError::NotFound);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "chaotic.box.disable", &id.to_string(), json!({"item_id": id}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok": true})))
}

/// Reward rows for one box, heaviest weight first (so the common fillers sort to the top).
async fn rewards(headers: HeaderMap, State(app): State<App>, Path(id): Path<i32>) -> R {
    auth(&headers, false)?;
    let rows = sqlx::query(
        "SELECT ie.n_index, ie.exchange_item_num1 item_id, ie.exchange_item_count1 weight,
                i.str_name, i.item_icon_id1,
                ie.exchange_item_count1::float8 / NULLIF(SUM(ie.exchange_item_count1) OVER (), 0) pct
         FROM item_exchange ie LEFT JOIN item i ON i.num = ie.exchange_item_num1
         WHERE ie.origin_item_num1 = $1 AND ie.random_flag IN (1,2,3)
         ORDER BY ie.exchange_item_count1 DESC, i.str_name",
    )
    .bind(id)
    .fetch_all(&app.pool)
    .await
    .map_err(internal("chaotic rewards"))?;
    let list: Vec<Value> = rows
        .iter()
        .map(|r| {
            use sqlx::Row;
            json!({
                "n_index": r.get::<i32, _>("n_index"),
                "item_id": r.get::<i32, _>("item_id"),
                "name": r.try_get::<String, _>("str_name").unwrap_or_default(),
                "icon": r.try_get::<i32, _>("item_icon_id1").unwrap_or(0),
                "weight": r.get::<i32, _>("weight"),
                "pct": r.try_get::<f64, _>("pct").unwrap_or(0.0) * 100.0,
            })
        })
        .collect();
    Ok(Json(json!({"rewards": list})))
}

#[derive(Deserialize)]
struct RewardIn {
    item_id: i32,
    weight: i32,
}

async fn add_reward(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i32>,
    Json(p): Json<RewardIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.weight <= 0 || p.weight > 100_000 {
        return Err(ApiError::BadRequest);
    }
    let box_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM item WHERE num = $1 AND effect2 = 251)")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("chaotic box check"))?;
    if !box_exists {
        return Err(ApiError::NotFound);
    }
    let reward_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM item WHERE num = $1)")
        .bind(p.item_id)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("chaotic reward item check"))?;
    if !reward_exists {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    // Panel-made rows live above the generated ranges (2100000000.., 2110000000..2110099999) and
    // below i32::MAX, because item_exchange.n_index is a 32-bit integer.
    let next_idx: i32 = sqlx::query_scalar(
        "SELECT (GREATEST(COALESCE(MAX(n_index) FILTER (WHERE n_index >= 2110100000), 0), 2110099999) + 1)::int4 FROM item_exchange",
    )
        .fetch_one(&mut *tx)
        .await
        .map_err(internal("next n_index"))?;
    sqlx::query(
        "INSERT INTO item_exchange (n_index, random_flag, origin_item_num1, origin_item_count1, exchange_item_num1, exchange_item_count1)
         VALUES ($1, 1, $2, 1, $3, $4)",
    )
    .bind(next_idx)
    .bind(id)
    .bind(p.item_id)
    .bind(p.weight)
    .execute(&mut *tx)
    .await
    .map_err(internal("chaotic reward insert"))?;
    audit(
        &mut tx,
        &s.actor,
        "chaotic.reward.add",
        &id.to_string(),
        json!({"box": id, "item_id": p.item_id, "weight": p.weight}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true, "n_index": next_idx})))
}

#[derive(Deserialize)]
struct RewardEditIn {
    weight: Option<i32>,
    item_id: Option<i32>,
}

/// Change one reward row's weight and/or the item it pays.
async fn update_reward(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path((id, n_index)): Path<(i32, i32)>,
    Json(p): Json<RewardEditIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.weight.is_some_and(|w| w <= 0 || w > 100_000) || (p.weight.is_none() && p.item_id.is_none()) {
        return Err(ApiError::BadRequest);
    }
    if let Some(item) = p.item_id {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM item WHERE num = $1)")
            .bind(item)
            .fetch_one(&app.pool)
            .await
            .map_err(internal("chaotic reward item check"))?;
        if !exists {
            return Err(ApiError::BadRequest);
        }
    }
    let done = sqlx::query(
        "UPDATE item_exchange SET exchange_item_count1 = COALESCE($3, exchange_item_count1), exchange_item_num1 = COALESCE($4, exchange_item_num1)
         WHERE n_index = $1 AND origin_item_num1 = $2 AND random_flag IN (1,2,3)",
    )
    .bind(n_index)
    .bind(id)
    .bind(p.weight)
    .bind(p.item_id)
    .execute(&app.pool)
    .await
    .map_err(internal("chaotic reward update"))?
    .rows_affected();
    if done == 0 {
        return Err(ApiError::NotFound);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "chaotic.reward.edit", &id.to_string(), json!({"box": id, "n_index": n_index, "weight": p.weight, "item_id": p.item_id}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true})))
}

async fn remove_reward(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path((id, n_index)): Path<(i32, i32)>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let done = sqlx::query("DELETE FROM item_exchange WHERE n_index = $1 AND origin_item_num1 = $2 AND random_flag IN (1,2,3)")
        .bind(n_index)
        .bind(id)
        .execute(&app.pool)
        .await
        .map_err(internal("chaotic reward delete"))?
        .rows_affected();
    if done == 0 {
        return Err(ApiError::NotFound);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "chaotic.reward.remove", &id.to_string(), json!({"box": id, "n_index": n_index}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true})))
}
