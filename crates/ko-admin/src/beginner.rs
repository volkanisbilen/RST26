//! Beginner (new character) editor: which set is active, starting stats per class/type and the
//! starting items per class/type. Items live in `create_new_char_set_level`; when a class/type has no
//! rows there the server falls back to the legacy `create_new_char_set`.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::legacy::{audit, auth, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/beginner", get(overview))
        .route("/jstkoadminpanel/api/beginner/active", post(set_active))
        .route("/jstkoadminpanel/api/beginner/set", get(get_set))
        .route("/jstkoadminpanel/api/beginner/slot", post(set_slot))
        .route("/jstkoadminpanel/api/beginner/move", post(move_slot))
        .route("/jstkoadminpanel/api/beginner/copy", post(copy_set))
        .route("/jstkoadminpanel/api/beginner/values", post(set_values))
}

type R = Result<Json<Value>, ApiError>;
const CLASSES: [i16; 5] = [1, 2, 3, 4, 13];

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}
fn valid(class: i16, ty: i16) -> bool {
    CLASSES.contains(&class) && (0..=4).contains(&ty)
}

async fn overview(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT jsonb_build_object(
            'active', COALESCE((SELECT beginner_type FROM beginner_settings WHERE server_no=1),1),
            'description', COALESCE((SELECT description FROM beginner_settings WHERE server_no=1),''),
            'values', COALESCE((SELECT jsonb_agg(v ORDER BY class_type,job_type) FROM (
                SELECT class_type, job_type, level, exp, strength, health, dexterity, intelligence, magic_power,
                       free_points, skill_point_free, skill_point_cat1, skill_point_cat2, skill_point_cat3,
                       skill_point_master, gold FROM create_new_char_value) v),'[]'::jsonb),
            'counts', COALESCE((SELECT jsonb_agg(c) FROM (
                SELECT class_type, beginner_type, count(*) FILTER (WHERE item_id>0) items
                FROM create_new_char_set_level GROUP BY class_type, beginner_type) c),'[]'::jsonb),
            'legacy', COALESCE((SELECT jsonb_agg(c) FROM (
                SELECT class_type, count(*) FILTER (WHERE item_id>0) items FROM create_new_char_set GROUP BY class_type) c),'[]'::jsonb)
        )"#,
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("beginner overview"))?;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct ActiveIn {
    ty: i16,
    reason: String,
}

async fn set_active(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<ActiveIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !(0..=4).contains(&p.ty) || p.reason.trim().chars().count() < 3 {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    sqlx::query("INSERT INTO beginner_settings(server_no,beginner_type) VALUES(1,$1) ON CONFLICT(server_no) DO UPDATE SET beginner_type=EXCLUDED.beginner_type")
        .bind(p.ty)
        .execute(&mut *tx)
        .await
        .map_err(internal("beginner active"))?;
    audit(&mut tx, &s.actor, "beginner.active", &p.ty.to_string(), json!({"reason":p.reason.trim()}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"note":"Yeni karakterlerin eşyaları hemen bu tipten alınır; başlangıç statları sunucu yeniden başlayınca yüklenir."})))
}

#[derive(Deserialize)]
struct SetQ {
    class: i16,
    ty: i16,
}

async fn get_set(headers: HeaderMap, State(app): State<App>, Query(q): Query<SetQ>) -> R {
    auth(&headers, false)?;
    if !valid(q.class, q.ty) {
        return Err(ApiError::BadRequest);
    }
    let own: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(jsonb_build_object('slot',s.slot_id,'id',s.item_id,'count',s.item_count,'durability',s.item_duration,
                  'flag',s.item_flag,'expire',s.item_expire_time,'name',COALESCE(i.str_name,''),'icon',COALESCE(i.item_icon_id1,0)) ORDER BY s.slot_id),'[]'::jsonb)
           FROM create_new_char_set_level s LEFT JOIN item i ON i.num=s.item_id
           WHERE s.class_type=$1 AND s.beginner_type=$2 AND s.item_id>0"#,
    )
    .bind(q.class)
    .bind(q.ty)
    .fetch_one(&app.pool)
    .await
    .map_err(internal("beginner set"))?;
    if own.as_array().is_some_and(|a| !a.is_empty()) {
        return Ok(Json(json!({"legacy":false,"items":own})));
    }
    let legacy: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(jsonb_build_object('slot',s.slot_id,'id',s.item_id,'count',s.item_count,'durability',s.item_duration,
                  'flag',s.item_flag,'expire',s.item_expire_time,'name',COALESCE(i.str_name,''),'icon',COALESCE(i.item_icon_id1,0)) ORDER BY s.slot_id),'[]'::jsonb)
           FROM create_new_char_set s LEFT JOIN item i ON i.num=s.item_id
           WHERE s.class_type=$1 AND s.item_id>0"#,
    )
    .bind(q.class)
    .fetch_one(&app.pool)
    .await
    .map_err(internal("beginner legacy set"))?;
    Ok(Json(json!({"legacy":true,"items":legacy})))
}

/// First edit of a class/type copies the legacy set in, so the server never silently falls back again.
async fn materialize(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, class: i16, ty: i16) -> Result<(), ApiError> {
    let has: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM create_new_char_set_level WHERE class_type=$1 AND beginner_type=$2 AND item_id>0)")
        .bind(class)
        .bind(ty)
        .fetch_one(&mut **tx)
        .await
        .map_err(internal("materialize check"))?;
    if !has {
        sqlx::query("DELETE FROM create_new_char_set_level WHERE class_type=$1 AND beginner_type=$2").bind(class).bind(ty).execute(&mut **tx).await.map_err(internal("materialize clear"))?;
        sqlx::query("INSERT INTO create_new_char_set_level(class_type,beginner_type,slot_id,item_id,item_duration,item_count,item_flag,item_expire_time) SELECT class_type,$2,slot_id,item_id,item_duration,item_count,item_flag,item_expire_time FROM create_new_char_set WHERE class_type=$1 AND item_id>0")
            .bind(class)
            .bind(ty)
            .execute(&mut **tx)
            .await
            .map_err(internal("materialize copy"))?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct SlotIn {
    class: i16,
    ty: i16,
    slot: i32,
    item_id: i32,
    #[serde(default = "one")]
    count: i16,
    #[serde(default)]
    durability: i16,
}
fn one() -> i16 {
    1
}

async fn write_slot(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, class: i16, ty: i16, slot: i32, item: i32, count: i16, dur: i16) -> Result<(), ApiError> {
    sqlx::query("INSERT INTO create_new_char_set_level(class_type,beginner_type,slot_id,item_id,item_duration,item_count,item_flag,item_expire_time) VALUES($1,$2,$3,$4,$5,$6,0,0) ON CONFLICT(class_type,beginner_type,slot_id) DO UPDATE SET item_id=EXCLUDED.item_id,item_duration=EXCLUDED.item_duration,item_count=EXCLUDED.item_count,item_flag=0,item_expire_time=0")
        .bind(class)
        .bind(ty)
        .bind(slot)
        .bind(item)
        .bind(dur)
        .bind(if item == 0 { 0 } else { count.max(1) })
        .execute(&mut **tx)
        .await
        .map_err(internal("beginner slot"))?;
    Ok(())
}

async fn set_slot(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<SlotIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid(p.class, p.ty) || !(0..=96).contains(&p.slot) || p.item_id < 0 || p.count < 0 || p.durability < 0 {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let mut dur = p.durability;
    if p.item_id > 0 {
        let row: Option<(Option<i16>,)> = sqlx::query_as("SELECT duration FROM item WHERE num=$1").bind(p.item_id).fetch_optional(&mut *tx).await.map_err(internal("item"))?;
        let Some((d,)) = row else { return Err(ApiError::BadRequest) };
        if dur == 0 {
            dur = d.unwrap_or(0);
        }
    }
    materialize(&mut tx, p.class, p.ty).await?;
    write_slot(&mut tx, p.class, p.ty, p.slot, p.item_id, p.count, dur).await?;
    audit(&mut tx, &s.actor, "beginner.slot", &format!("{}:{}:{}", p.class, p.ty, p.slot), json!({"item_id":p.item_id,"count":p.count}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct MoveIn {
    class: i16,
    ty: i16,
    from: i32,
    to: i32,
}

async fn move_slot(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<MoveIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid(p.class, p.ty) || !(0..=96).contains(&p.from) || !(0..=96).contains(&p.to) {
        return Err(ApiError::BadRequest);
    }
    if p.from == p.to {
        return Ok(Json(json!({"ok":true})));
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    materialize(&mut tx, p.class, p.ty).await?;
    let read = |slot: i32| {
        sqlx::query_as::<_, (i32, i16, i16)>("SELECT item_id,item_count,item_duration FROM create_new_char_set_level WHERE class_type=$1 AND beginner_type=$2 AND slot_id=$3")
            .bind(p.class)
            .bind(p.ty)
            .bind(slot)
    };
    let a = read(p.from).fetch_optional(&mut *tx).await.map_err(internal("move read"))?.unwrap_or((0, 0, 0));
    let b = read(p.to).fetch_optional(&mut *tx).await.map_err(internal("move read"))?.unwrap_or((0, 0, 0));
    write_slot(&mut tx, p.class, p.ty, p.from, b.0, b.1, b.2).await?;
    write_slot(&mut tx, p.class, p.ty, p.to, a.0, a.1, a.2).await?;
    audit(&mut tx, &s.actor, "beginner.move", &format!("{}:{}:{}->{}", p.class, p.ty, p.from, p.to), json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct CopyIn {
    from_class: i16,
    from_ty: i16,
    to_class: i16,
    to_ty: i16,
    reason: String,
}

async fn copy_set(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<CopyIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid(p.from_class, p.from_ty) || !valid(p.to_class, p.to_ty) || p.reason.trim().chars().count() < 3 {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    materialize(&mut tx, p.from_class, p.from_ty).await?;
    sqlx::query("DELETE FROM create_new_char_set_level WHERE class_type=$1 AND beginner_type=$2").bind(p.to_class).bind(p.to_ty).execute(&mut *tx).await.map_err(internal("copy clear"))?;
    let n = sqlx::query("INSERT INTO create_new_char_set_level(class_type,beginner_type,slot_id,item_id,item_duration,item_count,item_flag,item_expire_time) SELECT $3,$4,slot_id,item_id,item_duration,item_count,item_flag,item_expire_time FROM create_new_char_set_level WHERE class_type=$1 AND beginner_type=$2 AND item_id>0")
        .bind(p.from_class)
        .bind(p.from_ty)
        .bind(p.to_class)
        .bind(p.to_ty)
        .execute(&mut *tx)
        .await
        .map_err(internal("copy"))?
        .rows_affected();
    audit(&mut tx, &s.actor, "beginner.copy", &format!("{}:{}->{}:{}", p.from_class, p.from_ty, p.to_class, p.to_ty), json!({"rows":n,"reason":p.reason.trim()}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"rows":n})))
}

#[derive(Deserialize)]
struct ValuesIn {
    class: i16,
    ty: i16,
    level: i16,
    exp: i64,
    strength: i16,
    health: i16,
    dexterity: i16,
    intelligence: i16,
    magic_power: i16,
    free_points: i16,
    skill_point_free: i16,
    skill_point_cat1: i16,
    skill_point_cat2: i16,
    skill_point_cat3: i16,
    skill_point_master: i16,
    gold: i32,
}

async fn set_values(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<ValuesIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let nums = [p.strength, p.health, p.dexterity, p.intelligence, p.magic_power, p.free_points, p.skill_point_free, p.skill_point_cat1, p.skill_point_cat2, p.skill_point_cat3, p.skill_point_master];
    if !valid(p.class, p.ty) || !(1..=83).contains(&p.level) || p.exp < 0 || p.gold < 0 || nums.iter().any(|v| *v < 0) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let n = sqlx::query("UPDATE create_new_char_value SET level=$3,exp=$4,strength=$5,health=$6,dexterity=$7,intelligence=$8,magic_power=$9,free_points=$10,skill_point_free=$11,skill_point_cat1=$12,skill_point_cat2=$13,skill_point_cat3=$14,skill_point_master=$15,gold=$16 WHERE class_type=$1 AND job_type=$2")
        .bind(p.class).bind(p.ty).bind(p.level).bind(p.exp).bind(p.strength).bind(p.health).bind(p.dexterity).bind(p.intelligence).bind(p.magic_power)
        .bind(p.free_points).bind(p.skill_point_free).bind(p.skill_point_cat1).bind(p.skill_point_cat2).bind(p.skill_point_cat3).bind(p.skill_point_master).bind(p.gold)
        .execute(&mut *tx)
        .await
        .map_err(internal("beginner values"))?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    audit(&mut tx, &s.actor, "beginner.values", &format!("{}:{}", p.class, p.ty), json!({"level":p.level,"gold":p.gold}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"note":"Başlangıç statları sunucu yeniden başlatılınca uygulanır."})))
}
