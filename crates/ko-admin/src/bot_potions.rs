//! "Bot Potları" page: which HP / MP potions the bots (PK, field, event) drink.
//!
//! The bots drink virtual potions: a potion *skill* of the magic table (item_group 9, direct type 1 = HP, 2 = MP). Without
//! settings a bot takes the biggest potion that heals at most 60 % of its bar. This page lists every such potion with the
//! items that carry it, lets the admin switch potions off, change the 60 % cap per kind, or pin one potion per kind.
//! Tables `bot_potion_setting` / `bot_potion_disabled`, reloaded by the `bot_potion_reload` admin command.

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
    Router::new().route("/jstkoadminpanel/api/bot-potions", get(list).post(save))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

/// `(magic number, 1 HP / 2 MP, amount, name, items json)` -- the same selection as `WorldState::potion_skills`.
async fn potions(app: &App) -> Result<Vec<(i32, i32, i32, String, Value)>, ApiError> {
    let rows = sqlx::query(
        "SELECT m.magic_num, t3.direct_type, t3.first_damage, COALESCE(m.en_name, '') AS name, \
         COALESCE((SELECT json_agg(json_build_object('id', i.num, 'name', COALESCE(i.str_name, ''), 'icon', COALESCE(i.item_icon_id1, 0)) ORDER BY i.num) \
                   FROM item i WHERE i.effect1 = m.magic_num), '[]'::json)::text AS items \
         FROM magic m JOIN magic_type3 t3 ON t3.i_num = m.magic_num \
         WHERE m.item_group = 9 AND m.moral = 1 AND m.type1 = 3 AND COALESCE(m.type2, 0) = 0 AND m.magic_num > 400000 \
           AND t3.direct_type IN (1, 2) AND t3.first_damage > 0 \
         ORDER BY t3.direct_type, t3.first_damage, m.magic_num",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("bot potions list"))?;
    Ok(rows
        .iter()
        .map(|r| {
            (
                r.try_get::<i32, _>("magic_num").unwrap_or(0),
                r.try_get::<i32, _>("direct_type").unwrap_or(0),
                r.try_get::<i32, _>("first_damage").unwrap_or(0),
                r.try_get::<String, _>("name").unwrap_or_default(),
                serde_json::from_str(&r.try_get::<String, _>("items").unwrap_or_default()).unwrap_or(json!([])),
            )
        })
        .collect())
}

async fn list(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let all = potions(&app).await?;
    let disabled: Vec<i32> = sqlx::query_scalar("SELECT skill_id FROM bot_potion_disabled")
        .fetch_all(&app.pool)
        .await
        .unwrap_or_default();
    let settings = sqlx::query("SELECT kind, cap_percent, fixed_skill FROM bot_potion_setting")
        .fetch_all(&app.pool)
        .await
        .unwrap_or_default();
    let get = |kind: &str| {
        let r = settings.iter().find(|r| r.try_get::<String, _>("kind").unwrap_or_default() == kind);
        json!({
            "cap_percent": r.and_then(|r| r.try_get::<i16, _>("cap_percent").ok()).unwrap_or(60),
            "fixed_skill": r.and_then(|r| r.try_get::<Option<i32>, _>("fixed_skill").ok().flatten()),
        })
    };
    let stock = sqlx::query("SELECT hp_potions, mp_potions, potion_cooldown_ms FROM bot_brain_config WHERE id = 1")
        .fetch_optional(&app.pool)
        .await
        .ok()
        .flatten();
    let hb: Option<f64> = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM (NOW() - at))::float8 FROM admin_heartbeat WHERE id=1")
        .fetch_optional(&app.pool)
        .await
        .ok()
        .flatten();
    Ok(Json(json!({
        "potions": all.iter().map(|(id, kind, amount, name, items)| json!({
            "skill_id": id, "kind": if *kind == 1 { "hp" } else { "mp" }, "amount": amount, "name": name,
            "items": items, "enabled": !disabled.contains(id),
        })).collect::<Vec<_>>(),
        "hp": get("hp"),
        "mp": get("mp"),
        "stock": {
            "hp": stock.as_ref().and_then(|r| r.try_get::<i16, _>("hp_potions").ok()).unwrap_or(30),
            "mp": stock.as_ref().and_then(|r| r.try_get::<i16, _>("mp_potions").ok()).unwrap_or(30),
            "cooldown_ms": stock.as_ref().and_then(|r| r.try_get::<i16, _>("potion_cooldown_ms").ok()).unwrap_or(2400),
        },
        "server_online": hb.is_some_and(|age| age < 20.0),
    })))
}

#[derive(Deserialize)]
struct KindIn {
    cap_percent: i16,
    fixed_skill: Option<i32>,
}

#[derive(Deserialize)]
struct SaveIn {
    hp: KindIn,
    mp: KindIn,
    disabled: Vec<i32>,
}

async fn save(headers: HeaderMap, ConnectInfo(peer): ConnectInfo<SocketAddr>, State(app): State<App>, Json(p): Json<SaveIn>) -> R {
    let (_, s) = auth(&headers, true)?;
    let all = potions(&app).await?;
    for (kind, k) in [(1, &p.hp), (2, &p.mp)] {
        if !(5..=100).contains(&k.cap_percent) {
            return Err(ApiError::BadRequest);
        }
        if let Some(f) = k.fixed_skill {
            if !all.iter().any(|(id, dt, ..)| *id == f && *dt == kind) {
                return Err(ApiError::BadRequest);
            }
        }
    }
    if p.disabled.iter().any(|d| !all.iter().any(|(id, ..)| id == d)) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    for (kind, k) in [("hp", &p.hp), ("mp", &p.mp)] {
        sqlx::query(
            "INSERT INTO bot_potion_setting (kind, cap_percent, fixed_skill) VALUES ($1, $2, $3) \
             ON CONFLICT (kind) DO UPDATE SET cap_percent = EXCLUDED.cap_percent, fixed_skill = EXCLUDED.fixed_skill",
        )
        .bind(kind)
        .bind(k.cap_percent)
        .bind(k.fixed_skill)
        .execute(&mut *tx)
        .await
        .map_err(internal("bot potions save"))?;
    }
    sqlx::query("DELETE FROM bot_potion_disabled").execute(&mut *tx).await.map_err(internal("bot potions clear"))?;
    for d in &p.disabled {
        sqlx::query("INSERT INTO bot_potion_disabled (skill_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(d)
            .execute(&mut *tx)
            .await
            .map_err(internal("bot potions disabled"))?;
    }
    audit(
        &mut tx,
        &s.actor,
        "bot_potions.save",
        "-",
        json!({"hp_cap": p.hp.cap_percent, "mp_cap": p.mp.cap_percent, "hp_fixed": p.hp.fixed_skill, "mp_fixed": p.mp.fixed_skill, "disabled": p.disabled.len()}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_command(&app.pool, "bot_potion_reload", json!({})).await?;
    Ok(Json(json!({"ok": true, "id": id})))
}
