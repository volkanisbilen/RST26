//! "Bot Zekâsı" page: the knobs of the shared bot decision layer
//! (`crates/ko-game/src/systems/bot_brain.rs`, design in `docs/BOT_AI.md`).
//!
//! One `bot_brain_config` row (id = 1). A save writes the row and queues a `bot_brain_reload`
//! admin_command so a running game server picks the change up without a restart (same pattern as
//! the "Etkinlik Botları" page).

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::legacy::{audit, auth, queue_command, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new().route("/jstkoadminpanel/api/bot-brain", get(load).post(save))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

/// Column name, default, min, max -- mirrors `BrainConfig` in ko-game (ko-admin has no dependency
/// on the game crate, same reasoning as `event_bots.rs`).
const FIELDS: &[(&str, i16, i16, i16)] = &[
    ("patrol_percent", 40, 0, 100),
    ("reaction_min_ms", 300, 0, 5000),
    ("reaction_max_ms", 900, 0, 5000),
    ("target_mistake_percent", 10, 0, 100),
    ("kite_miss_percent", 25, 0, 100),
    ("hp_potions", 30, 0, 999),
    ("mp_potions", 30, 0, 999),
    ("ts_scrolls", 2, 0, 99),
    ("buff_scrolls", 4, 0, 99),
    ("potion_cooldown_ms", 2400, 500, 30000),
    ("np_percent", 33, 0, 100),
    ("antifarm_secs", 300, 0, 7200),
    ("spawn_grace_secs", 20, 0, 600),
    ("party_max_bots", 6, 2, 8),
    ("party_max_per_class", 2, 1, 8),
    ("retreat_hp_percent", 35, 0, 90),
    ("outnumbered_percent", 200, 100, 1000),
    ("spacing_min", 3, 1, 20),
    ("spacing_max", 8, 1, 30),
];

async fn load(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let row = sqlx::query("SELECT * FROM bot_brain_config WHERE id = 1")
        .fetch_optional(&app.pool)
        .await
        .map_err(internal("bot-brain load"))?;
    let mut values = serde_json::Map::new();
    values.insert(
        "enabled".into(),
        json!(row.as_ref().and_then(|r| r.try_get::<bool, _>("enabled").ok()).unwrap_or(true)),
    );
    for (name, default, _, _) in FIELDS {
        let v = row
            .as_ref()
            .and_then(|r| r.try_get::<i16, _>(*name).ok())
            .unwrap_or(*default);
        values.insert((*name).into(), json!(v));
    }
    let limits: Vec<Value> = FIELDS
        .iter()
        .map(|(name, default, min, max)| json!({"name": name, "default": default, "min": min, "max": max}))
        .collect();
    Ok(Json(json!({ "config": Value::Object(values), "limits": limits })))
}

#[derive(Deserialize)]
struct SaveIn {
    enabled: bool,
    #[serde(flatten)]
    values: std::collections::HashMap<String, i64>,
}

async fn save(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<SaveIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut vals: Vec<i16> = Vec::with_capacity(FIELDS.len());
    for (name, default, min, max) in FIELDS {
        let v = p.values.get(*name).copied().unwrap_or(*default as i64);
        if v < *min as i64 || v > *max as i64 {
            return Err(ApiError::BadRequest);
        }
        vals.push(v as i16);
    }
    let get = |name: &str| -> i16 {
        FIELDS
            .iter()
            .position(|(n, ..)| *n == name)
            .map(|i| vals[i])
            .unwrap_or(0)
    };
    if get("reaction_min_ms") > get("reaction_max_ms") || get("spacing_min") > get("spacing_max") {
        return Err(ApiError::BadRequest);
    }

    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let sets: Vec<String> = FIELDS
        .iter()
        .enumerate()
        .map(|(i, (name, ..))| format!("{name}=${}", i + 2))
        .collect();
    let cols: Vec<&str> = FIELDS.iter().map(|(n, ..)| *n).collect();
    let placeholders: Vec<String> = (0..FIELDS.len()).map(|i| format!("${}", i + 2)).collect();
    let sql = format!(
        "INSERT INTO bot_brain_config(id,enabled,{},updated_at) VALUES(1,$1,{},NOW()) \
         ON CONFLICT(id) DO UPDATE SET enabled=$1,{},updated_at=NOW()",
        cols.join(","),
        placeholders.join(","),
        sets.join(",")
    );
    let mut q = sqlx::query(&sql).bind(p.enabled);
    for v in &vals {
        q = q.bind(*v);
    }
    q.execute(&mut *tx).await.map_err(internal("bot-brain save"))?;

    let mut audit_json = serde_json::Map::new();
    audit_json.insert("enabled".into(), json!(p.enabled));
    for (i, (name, ..)) in FIELDS.iter().enumerate() {
        audit_json.insert((*name).into(), json!(vals[i]));
    }
    audit(&mut tx, &s.actor, "bot_brain.save", "1", Value::Object(audit_json), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_command(&app.pool, "bot_brain_reload", json!({})).await?;
    Ok(Json(json!({"ok": true, "reload_id": id})))
}
