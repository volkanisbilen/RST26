//! "Etkinlik Botları" page: per-event configuration of the event bot system
//! (`crates/ko-game/src/systems/event_bots.rs`, design in `docs/EVENT_BOTS.md`).
//!
//! One `event_bot_config` row per event. Everything is OFF by default. A save writes the row and
//! queues an `event_bots_reload` admin_command so a running game server picks the change up
//! without a restart (the same pattern the "Bot Donanımı" page uses). A separate button queues
//! `event_bots_clear` to remove every event bot that is currently standing in a room.

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
        .route("/jstkoadminpanel/api/event-bots", get(list))
        .route("/jstkoadminpanel/api/event-bots/clear", post(clear))
        .route("/jstkoadminpanel/api/event-bots/{event_key}", post(save))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

/// Static description of an event the bot system knows. Mirrors `EventBotKind` in ko-game
/// (ko-admin has no dependency on the ko-game crate, same reasoning as `bot_loadout.rs`).
struct Def {
    key: &'static str,
    name: &'static str,
    /// `full` | `partial` | `unsupported`
    support: &'static str,
    /// Largest "team" the event has (per nation in BDW, whole room in Chaos, ...).
    capacity: i16,
    /// What "team" means for this event, shown next to the bot count.
    team_label: &'static str,
    note: &'static str,
}

const EVENTS: &[Def] = &[
    Def {
        key: "chaos",
        name: "Chaos Dungeon (Chaos Expansion)",
        support: "full",
        capacity: 18,
        team_label: "oda (herkes herkese)",
        note: "Botlar odaya girer, odadaki herkesle (oyuncu ve diğer botlar) savaşır; kill/death odaya işlenir. Bota ödül verilmez, sıralama yalnızca oyuncular arasındadır.",
    },
    Def {
        key: "bdw",
        name: "Border Defence War",
        support: "partial",
        capacity: 8,
        team_label: "ulus (oda başına)",
        note: "Botlar iki ulusun boş yerlerini doldurur ve karşı ulusla savaşır, skor oyuncular gibi işlenir. Altar bayrağı yalnızca oyuncular tarafından taşınır (bot altar'a dokunmaz).",
    },
    Def {
        key: "ultima",
        name: "Ultima Dungeon",
        support: "full",
        capacity: 30,
        team_label: "oda",
        note: "Botlar ilk odaya ek raid üyesi olarak girer, mühür ve boss'a vurur; minimum 4 oyuncu şartına sayılır. Ödüller yalnızca oyunculara gider.",
    },
    Def {
        key: "rift",
        name: "Full Moon Rift",
        support: "partial",
        capacity: 8,
        team_label: "grup",
        note: "Destek botları Outer Tower'ı tutar, canavarların canını %10'a kadar düşürür ama öldürücü vuruşu yapmaz; böylece tüm drop/ödül oyuncularda kalır. Zorluk yalnızca gerçek oyuncu sayısına göre belirlenir.",
    },
    Def {
        key: "juraid",
        name: "Juraid Mountain",
        support: "unsupported",
        capacity: 8,
        team_label: "ulus",
        note: "Henüz desteklenmiyor: köprü/canavar/Deva ilerlemesi güvenilir şekilde botlarla sürülemiyor (docs/EVENT_BOTS.md).",
    },
    Def {
        key: "ft",
        name: "Forgotten Temple",
        support: "unsupported",
        capacity: 0,
        team_label: "-",
        note: "Desteklenmiyor: kayıt aşaması yok, bölgeye serbest giriş; ödül bölgedeki oyunculara dağıtılıyor.",
    },
    Def {
        key: "manes",
        name: "Manes Survival",
        support: "unsupported",
        capacity: 0,
        team_label: "-",
        note: "Desteklenmiyor: bireysel hayatta kalma sıralaması; botlar sıralamayı ve Manes Orb ödüllerini bozardı.",
    },
];

fn def(key: &str) -> Option<&'static Def> {
    EVENTS.iter().find(|d| d.key == key)
}

async fn list(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let rows = sqlx::query(
        "SELECT event_key, enabled, bots_per_team, min_level, max_level, fill_only_when_short, min_humans \
         FROM event_bot_config",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("event-bots list"))?;
    let mut events = Vec::with_capacity(EVENTS.len());
    for d in EVENTS {
        let row = rows
            .iter()
            .find(|r| r.try_get::<String, _>("event_key").is_ok_and(|k| k == d.key));
        let get_i16 = |col: &str, default: i16| -> i16 {
            row.and_then(|r| r.try_get::<i16, _>(col).ok()).unwrap_or(default)
        };
        events.push(json!({
            "key": d.key,
            "name": d.name,
            "support": d.support,
            "capacity": d.capacity,
            "team_label": d.team_label,
            "note": d.note,
            "enabled": d.support != "unsupported" && row.and_then(|r| r.try_get::<bool, _>("enabled").ok()).unwrap_or(false),
            "bots_per_team": get_i16("bots_per_team", 4),
            "min_level": get_i16("min_level", 60),
            "max_level": get_i16("max_level", 83),
            "fill_only_when_short": row.and_then(|r| r.try_get::<bool, _>("fill_only_when_short").ok()).unwrap_or(true),
            "min_humans": get_i16("min_humans", 1),
        }));
    }
    Ok(Json(json!({ "events": events })))
}

#[derive(Deserialize)]
struct SaveIn {
    enabled: bool,
    bots_per_team: i16,
    min_level: i16,
    max_level: i16,
    fill_only_when_short: bool,
    min_humans: i16,
}

async fn save(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(event_key): Path<String>,
    Json(p): Json<SaveIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let Some(d) = def(&event_key) else {
        return Err(ApiError::BadRequest);
    };
    // unsupported events have nothing to configure (the game server keeps them off anyway)
    if d.support == "unsupported" {
        return Err(ApiError::BadRequest);
    }
    if !(0..=d.capacity).contains(&p.bots_per_team)
        || !(1..=83).contains(&p.min_level)
        || !(1..=83).contains(&p.max_level)
        || p.min_level > p.max_level
        || !(1..=30).contains(&p.min_humans)
    {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    sqlx::query(
        "INSERT INTO event_bot_config(event_key,enabled,bots_per_team,min_level,max_level,fill_only_when_short,min_humans,updated_at) \
         VALUES($1,$2,$3,$4,$5,$6,$7,NOW()) \
         ON CONFLICT(event_key) DO UPDATE SET enabled=EXCLUDED.enabled,bots_per_team=EXCLUDED.bots_per_team,\
         min_level=EXCLUDED.min_level,max_level=EXCLUDED.max_level,fill_only_when_short=EXCLUDED.fill_only_when_short,\
         min_humans=EXCLUDED.min_humans,updated_at=NOW()",
    )
    .bind(d.key)
    .bind(p.enabled)
    .bind(p.bots_per_team)
    .bind(p.min_level)
    .bind(p.max_level)
    .bind(p.fill_only_when_short)
    .bind(p.min_humans)
    .execute(&mut *tx)
    .await
    .map_err(internal("event-bots save"))?;
    audit(
        &mut tx,
        &s.actor,
        "event_bots.save",
        d.key,
        json!({
            "enabled": p.enabled,
            "bots_per_team": p.bots_per_team,
            "min_level": p.min_level,
            "max_level": p.max_level,
            "fill_only_when_short": p.fill_only_when_short,
            "min_humans": p.min_humans,
        }),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_command(&app.pool, "event_bots_reload", json!({})).await?;
    Ok(Json(json!({"ok": true, "reload_id": id})))
}

async fn clear(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "event_bots.clear", "all", json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_command(&app.pool, "event_bots_clear", json!({})).await?;
    Ok(Json(json!({"ok": true, "command_id": id})))
}
