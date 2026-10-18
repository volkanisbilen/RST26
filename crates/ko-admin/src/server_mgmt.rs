//! "Sunucu Yönetimi": live table reloads through the game server's command queue, a guarded
//! server restart (systemctl, with an in-game countdown) and the character-name live search.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
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
        .route("/jstkoadminpanel/api/server/status", get(status))
        .route("/jstkoadminpanel/api/server/reload", post(reload))
        .route("/jstkoadminpanel/api/server/command/{id}", get(command_result))
        .route("/jstkoadminpanel/api/server/restart", post(restart))
        .route("/jstkoadminpanel/api/server/restart/cancel", post(restart_cancel))
        .route("/jstkoadminpanel/api/char-search", get(char_search))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

/// Reload targets the panel offers: (key, game command, `what` argument). Every one of them is a
/// reload the game server already implements (see ko-game `admin_queue.rs`).
const TARGETS: &[(&str, &str, &str)] = &[
    ("drops", "reload", "monster_drop_tables"),
    ("groups", "reload", "make_item_groups"),
    ("quest", "reload", "quest_tables"),
    ("event_schedule", "reload", "event_schedule"),
    ("event_rewards", "reload", "event_rewards"),
    ("chaos_rewards", "reload", "chaos_rewards"),
    ("daily_quests", "reload", "daily_quests"),
    ("cinderella", "reload", "cinderella"),
    ("zone_rewards", "reload", "zone_rewards"),
    ("collection_race", "reload", "collection_race"),
    ("reward_rules", "reload", "reward_rules"),
    ("pus", "reload", "pus"),
    ("send_messages", "reload", "send_messages"),
    ("item_exchange", "reload", "item_exchange"),
    ("rankings", "reload", "rankings"),
    ("all", "reload", "all"),
    ("field_bots", "field_bot_reload", ""),
    ("bot_loadout", "bot_loadout_reload", ""),
    ("info_panel", "info_panel_reload", ""),
];

const UNIT: &str = "ko-server";
const COMMANDS: &[&str] = &["reload", "field_bot_reload", "bot_loadout_reload", "info_panel_reload", "announce"];

/// Generation of the pending restart (0 = none). A new request or a cancel changes it, which
/// makes an older countdown task stop.
static RESTART_GEN: AtomicU64 = AtomicU64::new(0);
/// Unix seconds of the planned restart, 0 = none.
static RESTART_AT: AtomicU64 = AtomicU64::new(0);
static NEXT_GEN: AtomicU64 = AtomicU64::new(1);

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `Ok(())` when `systemctl` exists and ko-server is a known unit, else a Turkish reason.
async fn restart_possible() -> Result<(), String> {
    let out = tokio::process::Command::new("systemctl")
        .args(["show", UNIT, "--property=LoadState", "--value"])
        .output()
        .await
        .map_err(|e| format!("systemctl çalıştırılamadı ({e}); panel bu makinede sunucuyu yeniden başlatamaz"))?;
    let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if state != "loaded" {
        return Err(format!("{UNIT} servisi bulunamadı (durum: {})", if state.is_empty() { "?" } else { &state }));
    }
    Ok(())
}

async fn status(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let online: bool = sqlx::query_scalar("SELECT COALESCE((SELECT at > NOW() - INTERVAL '15 seconds' FROM admin_heartbeat WHERE id=1), false)")
        .fetch_one(&app.pool)
        .await
        .map_err(internal("server status"))?;
    let players: i64 = sqlx::query_scalar("SELECT count(*) FROM currentuser")
        .fetch_one(&app.pool)
        .await
        .unwrap_or(0);
    let at = RESTART_AT.load(Ordering::SeqCst);
    let pending = RESTART_GEN.load(Ordering::SeqCst) != 0 && at > 0;
    let restart_ok = restart_possible().await;
    Ok(Json(json!({
        "server_online": online,
        "players": players,
        "restart_possible": restart_ok.is_ok(),
        "restart_reason": restart_ok.err().unwrap_or_default(),
        "restart_in": if pending { json!(at.saturating_sub(unix_now())) } else { Value::Null },
    })))
}

#[derive(Deserialize)]
struct ReloadIn {
    what: String,
}

async fn reload(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<ReloadIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let (_, command, what) = TARGETS.iter().find(|t| t.0 == p.what).ok_or(ApiError::BadRequest)?;
    let args = if what.is_empty() { json!({}) } else { json!({"what": what}) };
    let id = queue_command(&app.pool, command, args.clone()).await?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "server.reload", &p.what, args, peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"id":id})))
}

async fn command_result(headers: HeaderMap, State(app): State<App>, Path(id): Path<i64>) -> R {
    auth(&headers, false)?;
    let row: Option<(String, String)> = sqlx::query_as("SELECT status, result FROM admin_command WHERE id=$1 AND command = ANY($2)")
        .bind(id)
        .bind(COMMANDS)
        .fetch_optional(&app.pool)
        .await
        .map_err(internal("server command"))?;
    let (status, result) = row.ok_or(ApiError::NotFound)?;
    Ok(Json(json!({"status":status,"result":result})))
}

#[derive(Deserialize)]
struct RestartIn {
    /// Seconds of in-game countdown before the restart (0..=600).
    #[serde(default)]
    delay_sec: u64,
}

async fn restart(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<RestartIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.delay_sec > 600 {
        return Err(ApiError::BadRequest);
    }
    if let Err(e) = restart_possible().await {
        return Ok(Json(json!({"ok":false,"error":e})));
    }
    let gen = NEXT_GEN.fetch_add(1, Ordering::SeqCst);
    RESTART_GEN.store(gen, Ordering::SeqCst);
    RESTART_AT.store(unix_now() + p.delay_sec, Ordering::SeqCst);

    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "server.restart", UNIT, json!({"delay_sec":p.delay_sec}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;

    let pool = app.pool.clone();
    let delay = p.delay_sec;
    tokio::spawn(async move {
        // announce at the start and at fixed marks; poll once a second so a cancel is quick
        let marks = [300u64, 120, 60, 30, 10, 5];
        let mut left = delay;
        if left > 0 {
            let _ = queue_command(&pool, "announce", json!({"message": format!("Sunucu {left} saniye icinde yeniden baslatilacak!")})).await;
        }
        while left > 0 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if RESTART_GEN.load(Ordering::SeqCst) != gen {
                return;
            }
            left -= 1;
            if marks.contains(&left) {
                let _ = queue_command(&pool, "announce", json!({"message": format!("Sunucu {left} saniye icinde yeniden baslatilacak!")})).await;
            }
        }
        if RESTART_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        RESTART_GEN.store(0, Ordering::SeqCst);
        RESTART_AT.store(0, Ordering::SeqCst);
        match tokio::process::Command::new("systemctl").args(["restart", UNIT]).output().await {
            Ok(o) if o.status.success() => tracing::warn!("ko-server restarted from the panel"),
            Ok(o) => tracing::warn!(stderr = %String::from_utf8_lossy(&o.stderr), "systemctl restart failed"),
            Err(e) => tracing::warn!(error = %e, "systemctl restart failed"),
        }
    });
    Ok(Json(json!({"ok":true})))
}

async fn restart_cancel(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let was = RESTART_GEN.swap(0, Ordering::SeqCst);
    RESTART_AT.store(0, Ordering::SeqCst);
    if was == 0 {
        return Ok(Json(json!({"ok":false,"error":"Bekleyen yeniden başlatma yok"})));
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "server.restart.cancel", UNIT, json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    let _ = queue_command(&app.pool, "announce", json!({"message":"Yeniden baslatma iptal edildi."})).await;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct CharQ {
    q: Option<String>,
}

/// Prefix search over character names (letters panel recipient box).
async fn char_search(headers: HeaderMap, State(app): State<App>, Query(q): Query<CharQ>) -> R {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_string();
    if term.is_empty() || term.chars().count() > 21 {
        return Ok(Json(json!([])));
    }
    // escape LIKE wildcards in the user's input
    let like = format!("{}%", term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let rows: Vec<(String, i16, bool)> = sqlx::query_as(
        "SELECT u.str_user_id, u.level, EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)) \
         FROM userdata u WHERE u.str_user_id ILIKE $1 ORDER BY u.str_user_id LIMIT 15",
    )
    .bind(like)
    .fetch_all(&app.pool)
    .await
    .map_err(internal("char search"))?;
    Ok(Json(Value::Array(
        rows.into_iter().map(|(n, l, o)| json!({"name":n,"level":l,"online":o})).collect(),
    )))
}
