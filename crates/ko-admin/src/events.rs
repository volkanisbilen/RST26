//! Event control: live state of every event, manual start/stop through the game server's command
//! queue, and the automatic schedule (`event_auto_schedule`, evaluated by the game server).

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
        .route("/jstkoadminpanel/api/events/live", get(live))
        .route("/jstkoadminpanel/api/events/command/{id}", get(command_result))
        .route("/jstkoadminpanel/api/events/run/{key}", post(run))
        .route("/jstkoadminpanel/api/events/auto", post(auto_create))
        .route("/jstkoadminpanel/api/events/auto/{id}", post(auto_update))
        .route("/jstkoadminpanel/api/events/auto/{id}/delete", post(auto_delete))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

async fn live(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT jsonb_build_object(
            'events', COALESCE((SELECT jsonb_agg(jsonb_build_object(
                'key',event_key,'name',name,'group',grp,'state',state,'detail',detail,
                'updated_at',updated_at,'arg_label',arg_label,'arg_required',arg_required,
                'has_now',has_now,'can_stop',can_stop) ORDER BY sort_no, event_key)
                FROM admin_event_state),'[]'::jsonb),
            'schedules', COALESCE((SELECT jsonb_agg(jsonb_build_object(
                'id',id,'key',event_key,'enabled',enabled,'days',days,'hour',hour,'minute',minute,
                'arg',arg,'duration_min',duration_min,'note',note,'last_run',last_run,'last_result',last_result)
                ORDER BY hour, minute, id) FROM event_auto_schedule),'[]'::jsonb),
            'server_online', COALESCE((SELECT at > NOW() - INTERVAL '15 seconds' FROM admin_heartbeat WHERE id=1), false),
            'server_time', to_char(NOW() AT TIME ZONE current_setting('TimeZone'), 'YYYY-MM-DD HH24:MI:SS'),
            'timezone', current_setting('TimeZone')
        )"#,
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("events live"))?;
    Ok(Json(v))
}

async fn command_result(headers: HeaderMap, State(app): State<App>, Path(id): Path<i64>) -> R {
    auth(&headers, false)?;
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT status, result FROM admin_command WHERE id=$1 AND command='event'")
            .bind(id)
            .fetch_optional(&app.pool)
            .await
            .map_err(internal("event command"))?;
    let (status, result) = row.ok_or(ApiError::NotFound)?;
    Ok(Json(json!({"status":status,"result":result})))
}

#[derive(Deserialize)]
struct RunIn {
    action: String,
    #[serde(default)]
    arg: String,
}

async fn run(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(key): Path<String>,
    Json(p): Json<RunIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !matches!(p.action.as_str(), "start" | "startnow" | "stop") || p.arg.len() > 40 {
        return Err(ApiError::BadRequest);
    }
    let known: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_event_state WHERE event_key=$1)")
        .bind(&key)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("event key"))?;
    if !known {
        return Err(ApiError::NotFound);
    }
    let args = json!({"key":key,"action":p.action,"arg":p.arg});
    let id = queue_command(&app.pool, "event", args.clone()).await?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "event.run", &key, args, peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"id":id})))
}

#[derive(Deserialize)]
struct AutoIn {
    key: String,
    enabled: bool,
    days: i16,
    hour: i16,
    minute: i16,
    #[serde(default)]
    arg: String,
    #[serde(default)]
    duration_min: i32,
    #[serde(default)]
    note: String,
}

fn valid(p: &AutoIn) -> bool {
    (1..=127).contains(&p.days)
        && (0..24).contains(&p.hour)
        && (0..60).contains(&p.minute)
        && p.arg.len() <= 40
        && (0..=600).contains(&p.duration_min)
        && p.note.chars().count() <= 80
}

async fn auto_create(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<AutoIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid(&p) {
        return Err(ApiError::BadRequest);
    }
    let known: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_event_state WHERE event_key=$1)")
        .bind(&p.key)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("event key"))?;
    if !known {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let id: i32 = sqlx::query_scalar(
        "INSERT INTO event_auto_schedule(event_key,enabled,days,hour,minute,arg,duration_min,note) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
    )
    .bind(&p.key)
    .bind(p.enabled)
    .bind(p.days)
    .bind(p.hour)
    .bind(p.minute)
    .bind(p.arg.trim())
    .bind(p.duration_min)
    .bind(p.note.trim())
    .fetch_one(&mut *tx)
    .await
    .map_err(internal("auto create"))?;
    audit(&mut tx, &s.actor, "event.auto.create", &p.key, json!({"id":id,"days":p.days,"time":format!("{:02}:{:02}",p.hour,p.minute),"enabled":p.enabled}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"id":id})))
}

async fn auto_update(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i32>,
    Json(p): Json<AutoIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid(&p) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let n = sqlx::query(
        "UPDATE event_auto_schedule SET enabled=$2,days=$3,hour=$4,minute=$5,arg=$6,duration_min=$7,note=$8 WHERE id=$1",
    )
    .bind(id)
    .bind(p.enabled)
    .bind(p.days)
    .bind(p.hour)
    .bind(p.minute)
    .bind(p.arg.trim())
    .bind(p.duration_min)
    .bind(p.note.trim())
    .execute(&mut *tx)
    .await
    .map_err(internal("auto update"))?
    .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    audit(&mut tx, &s.actor, "event.auto.update", &p.key, json!({"id":id,"days":p.days,"time":format!("{:02}:{:02}",p.hour,p.minute),"enabled":p.enabled}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

async fn auto_delete(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i32>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let key: Option<String> = sqlx::query_scalar("DELETE FROM event_auto_schedule WHERE id=$1 RETURNING event_key")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal("auto delete"))?;
    let key = key.ok_or(ApiError::NotFound)?;
    audit(&mut tx, &s.actor, "event.auto.delete", &key, json!({"id":id}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}
