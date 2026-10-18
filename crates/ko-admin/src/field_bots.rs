//! "Saha Botlari" page: per-zone control of the field bots (server-controlled players that hunt
//! monsters in the normal maps and can be invited to a party, see
//! `crates/ko-game/src/systems/field_bots.rs`).
//!
//! Everything lives in the `field_bot_zone` table and is OFF by default. Saving a row queues a
//! `field_bot_reload` admin_command (the game server re-reads the table and brings the
//! population in line), "respawn" queues a `field_bot` clear (bots of enabled zones refill within
//! seconds). The technical ceilings below are duplicated from the game server on purpose -- ko-admin has no
//! dependency on ko-game (same reasoning as `PK_BOT_ZONES` in legacy.rs) -- and the game server
//! enforces them again, so a hand-edited row can never overload it.

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
        .route("/jstkoadminpanel/api/field-bots", get(list))
        .route("/jstkoadminpanel/api/field-bots/{zone}", post(save))
        .route("/jstkoadminpanel/api/field-bots/{zone}/respawn", post(respawn))
}

type R = Result<Json<Value>, ApiError>;

/// Technical safety ceilings (mirrors ko-game): not a gameplay limit.
const MAX_PER_NATION_ZONE: i16 = 1000;
const MAX_TOTAL: i64 = 5000;

/// (zone id, label, owner: 0 = both nations, 1 = Karus, 2 = El Morad, zone minimum level).
const ZONES: [(i16, &str, i16, i16); 5] = [
    (21, "Moradon", 0, 1),
    (1, "Luferson (Karus)", 1, 35),
    (2, "El Morad", 2, 35),
    (11, "Karus Eslant", 1, 60),
    (12, "El Morad Eslant", 2, 60),
];

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

fn zone_meta(zone: i16) -> Option<(i16, &'static str, i16, i16)> {
    ZONES.iter().copied().find(|z| z.0 == zone)
}

/// One row, clamped to what the game server will actually use.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row5 {
    enabled: bool,
    karus: i16,
    elmorad: i16,
    level_min: i16,
    level_max: i16,
    mix: [i16; 5],
}

fn sanitize(zone: i16, p: &SaveIn) -> Option<Row5> {
    let (_, _, owner, min_level) = zone_meta(zone)?;
    let level_min = p.level_min.clamp(min_level.max(1), 83);
    let level_max = p.level_max.clamp(level_min, 83);
    let karus = if owner == 0 || owner == 1 { p.karus_count.clamp(0, MAX_PER_NATION_ZONE) } else { 0 };
    let elmorad = if owner == 0 || owner == 2 { p.elmorad_count.clamp(0, MAX_PER_NATION_ZONE) } else { 0 };
    let mut mix = p.mix;
    for w in &mut mix {
        *w = (*w).clamp(0, 100);
    }
    if mix.iter().all(|w| *w == 0) {
        mix = [30, 20, 25, 25, 0];
    }
    Some(Row5 { enabled: p.enabled, karus, elmorad, level_min, level_max, mix })
}

async fn list(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let rows = sqlx::query(
        "SELECT zone_id, enabled, karus_count, elmorad_count, level_min, level_max, mix_warrior, mix_rogue, \
         mix_mage, mix_priest, mix_kurian, live_karus, live_elmorad, live_note, \
         COALESCE(EXTRACT(EPOCH FROM (NOW() - live_updated_at))::float8, -1) AS live_age \
         FROM field_bot_zone",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("field-bots list"))?;
    let hb: Option<f64> = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM (NOW() - at))::float8 FROM admin_heartbeat WHERE id=1")
        .fetch_optional(&app.pool)
        .await
        .ok()
        .flatten();
    let server_online = hb.is_some_and(|age| age < 20.0);

    let mut zones = Vec::new();
    for (zone, name, owner, min_level) in ZONES {
        let r = rows.iter().find(|r| r.try_get::<i16, _>("zone_id").unwrap_or(0) == zone);
        let g = |col: &str, default: i16| r.and_then(|r| r.try_get::<i16, _>(col).ok()).unwrap_or(default);
        zones.push(json!({
            "zone": zone,
            "name": name,
            "owner": owner,
            "min_level": min_level,
            "enabled": r.and_then(|r| r.try_get::<bool, _>("enabled").ok()).unwrap_or(false),
            "karus_count": g("karus_count", 0),
            "elmorad_count": g("elmorad_count", 0),
            "level_min": g("level_min", min_level.max(1)),
            "level_max": g("level_max", 83),
            "mix": [g("mix_warrior", 30), g("mix_rogue", 20), g("mix_mage", 25), g("mix_priest", 25), g("mix_kurian", 0)],
            "live_karus": g("live_karus", 0),
            "live_elmorad": g("live_elmorad", 0),
            "live_note": r.and_then(|r| r.try_get::<Option<String>, _>("live_note").ok().flatten()).unwrap_or_default(),
            "live_age": r.and_then(|r| r.try_get::<f64, _>("live_age").ok()).unwrap_or(-1.0),
        }));
    }
    Ok(Json(json!({
        "zones": zones,
        "caps": {"per_nation_zone": MAX_PER_NATION_ZONE, "total": MAX_TOTAL},
        "server_online": server_online,
    })))
}

#[derive(Deserialize)]
struct SaveIn {
    enabled: bool,
    karus_count: i16,
    elmorad_count: i16,
    level_min: i16,
    level_max: i16,
    /// warrior, rogue, mage, priest, kurian/porutu weights.
    mix: [i16; 5],
}

async fn save(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(zone): Path<i16>,
    Json(p): Json<SaveIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let row = sanitize(zone, &p).ok_or(ApiError::BadRequest)?;

    // The world-wide cap: all other enabled zones plus this one must stay within it.
    let others: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(karus_count + elmorad_count), 0)::bigint FROM field_bot_zone WHERE enabled AND zone_id <> $1",
    )
    .bind(zone)
    .fetch_one(&app.pool)
    .await
    .map_err(internal("field-bots total"))?;
    let mine = if row.enabled { (row.karus + row.elmorad) as i64 } else { 0 };
    if others + mine > MAX_TOTAL {
        return Err(ApiError::BadRequest);
    }

    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    sqlx::query(
        "INSERT INTO field_bot_zone(zone_id,enabled,karus_count,elmorad_count,level_min,level_max,\
         mix_warrior,mix_rogue,mix_mage,mix_priest,mix_kurian,updated_at) \
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW()) \
         ON CONFLICT(zone_id) DO UPDATE SET enabled=EXCLUDED.enabled,karus_count=EXCLUDED.karus_count,\
         elmorad_count=EXCLUDED.elmorad_count,level_min=EXCLUDED.level_min,level_max=EXCLUDED.level_max,\
         mix_warrior=EXCLUDED.mix_warrior,mix_rogue=EXCLUDED.mix_rogue,mix_mage=EXCLUDED.mix_mage,\
         mix_priest=EXCLUDED.mix_priest,mix_kurian=EXCLUDED.mix_kurian,updated_at=NOW()",
    )
    .bind(zone)
    .bind(row.enabled)
    .bind(row.karus)
    .bind(row.elmorad)
    .bind(row.level_min)
    .bind(row.level_max)
    .bind(row.mix[0])
    .bind(row.mix[1])
    .bind(row.mix[2])
    .bind(row.mix[3])
    .bind(row.mix[4])
    .execute(&mut *tx)
    .await
    .map_err(internal("field-bots save"))?;
    let detail = json!({
        "enabled": row.enabled, "karus": row.karus, "elmorad": row.elmorad,
        "level": [row.level_min, row.level_max], "mix": row.mix,
    });
    audit(&mut tx, &s.actor, "field_bots.save", &zone.to_string(), detail, peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_command(&app.pool, "field_bot_reload", json!({})).await?;
    Ok(Json(json!({
        "ok": true,
        "reload_id": id,
        "saved": {
            "enabled": row.enabled, "karus_count": row.karus, "elmorad_count": row.elmorad,
            "level_min": row.level_min, "level_max": row.level_max, "mix": row.mix,
        },
    })))
}

async fn respawn(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(zone): Path<i16>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    zone_meta(zone).ok_or(ApiError::BadRequest)?;
    let detail = json!({"action": "clear", "zone": zone});
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "field_bots.respawn", &zone.to_string(), detail.clone(), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_command(&app.pool, "field_bot", detail).await?;
    Ok(Json(json!({"ok": true, "id": id})))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(enabled: bool, k: i16, e: i16, lo: i16, hi: i16, mix: [i16; 5]) -> SaveIn {
        SaveIn { enabled, karus_count: k, elmorad_count: e, level_min: lo, level_max: hi, mix }
    }

    #[test]
    fn unsupported_zone_is_rejected() {
        assert!(sanitize(71, &input(true, 5, 5, 1, 83, [1, 1, 1, 1, 1])).is_none());
        assert!(sanitize(21, &input(true, 5, 5, 1, 83, [1, 1, 1, 1, 1])).is_some());
    }

    #[test]
    fn counts_levels_and_owner_are_clamped() {
        // Karus land: the El Morad count is dropped, the Karus count not capped at the old 40.
        let r = sanitize(1, &input(true, 500, 30, 1, 200, [10, 10, 10, 10, 10])).unwrap();
        assert_eq!((r.karus, r.elmorad), (500, 0));
        assert_eq!((r.level_min, r.level_max), (35, 83), "level starts at the zone minimum");
        // Neutral Moradon keeps both nations, each capped.
        let m = sanitize(21, &input(true, -4, 99, 70, 20, [10, 10, 10, 10, 10])).unwrap();
        assert_eq!((m.karus, m.elmorad), (0, 99));
        assert!(m.level_min <= m.level_max);
    }

    #[test]
    fn empty_mix_falls_back_to_defaults() {
        let r = sanitize(21, &input(true, 1, 1, 1, 83, [0, 0, 0, 0, 0])).unwrap();
        assert_eq!(r.mix, [30, 20, 25, 25, 0]);
        let r = sanitize(21, &input(true, 1, 1, 1, 83, [500, -3, 1, 1, 1])).unwrap();
        assert_eq!(r.mix[0], 100);
        assert_eq!(r.mix[1], 0);
    }
}
