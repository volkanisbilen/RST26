//! "PK Otomatik Bot" page: which PK zones get bots by themselves when the game server starts.
//!
//! Rows live in `pk_bot_autospawn` (zone 71 Ronark Land, 72 Ardream, 73 Ronark Land Base). The game
//! server reads them ~25 s after it started and tops each enabled zone up to `count_per_nation`
//! bots of each nation; "Şimdi uygula" queues the same run (`pk_autospawn` admin command).

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
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
        .route("/jstkoadminpanel/api/pk-autospawn", get(list))
        .route("/jstkoadminpanel/api/pk-autospawn/apply", post(apply))
        .route("/jstkoadminpanel/api/pk-autospawn/{zone}", post(save))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

const ZONES: [(i16, &str, i16); 3] = [(71, "Ronark Land", 83), (72, "Ardream", 59), (73, "Ronark Land Base", 83)];

async fn list(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let rows = sqlx::query("SELECT zone_id, enabled, count_per_nation, level FROM pk_bot_autospawn")
        .fetch_all(&app.pool)
        .await
        .map_err(internal("pk autospawn list"))?;
    let zones: Vec<Value> = ZONES
        .iter()
        .map(|(id, label, max_level)| {
            let row = rows.iter().find(|r| r.get::<i16, _>("zone_id") == *id);
            json!({
                "zone": id,
                "label": label,
                "max_level": max_level,
                "enabled": row.map(|r| r.get::<bool, _>("enabled")).unwrap_or(false),
                "count": row.map(|r| r.get::<i16, _>("count_per_nation")).unwrap_or(100),
                "level": row.and_then(|r| r.try_get::<Option<i16>, _>("level").ok().flatten()),
            })
        })
        .collect();
    Ok(Json(json!({"zones": zones})))
}

#[derive(Deserialize)]
struct SaveIn {
    enabled: bool,
    count: i16,
    level: Option<i16>,
}

async fn save(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(zone): Path<i16>,
    Json(p): Json<SaveIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let Some((_, _, max_level)) = ZONES.iter().find(|z| z.0 == zone) else {
        return Err(ApiError::BadRequest);
    };
    if !(0..=1000).contains(&p.count) || p.level.is_some_and(|l| !(1..=*max_level).contains(&l)) {
        return Err(ApiError::BadRequest);
    }
    sqlx::query(
        "INSERT INTO pk_bot_autospawn (zone_id, enabled, count_per_nation, level) VALUES ($1,$2,$3,$4)
         ON CONFLICT (zone_id) DO UPDATE SET enabled = $2, count_per_nation = $3, level = $4, updated_at = NOW()",
    )
    .bind(zone)
    .bind(p.enabled)
    .bind(p.count)
    .bind(p.level)
    .execute(&app.pool)
    .await
    .map_err(internal("pk autospawn save"))?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "pk_autospawn.save", &zone.to_string(), json!({"enabled": p.enabled, "count": p.count, "level": p.level}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok": true})))
}

async fn apply(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, true)?;
    let id = queue_command(&app.pool, "pk_autospawn", json!({})).await?;
    Ok(Json(json!({"ok": true, "command": id})))
}
