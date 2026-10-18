//! Character sheet (the game's "Character Info" window): stats, resistances, KC/TL, edit and stat reset.

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

use crate::legacy::{audit, auth, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/characters/{name}/sheet", get(character_sheet))
        .route("/jstkoadminpanel/api/characters/{name}/stats", post(character_stats))
        .route("/jstkoadminpanel/api/characters/{name}/stat-reset", post(character_stat_reset))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

async fn character_sheet(headers: HeaderMap, State(app): State<App>, Path(name): Path<String>) -> R {
    auth(&headers, false)?;
    let v: Option<Value> = sqlx::query_scalar(
        r#"SELECT jsonb_build_object(
            'name',u.str_user_id,'nation',u.nation,'class',u.class,'race',u.race,'level',u.level,'rebirth',u.rebirth_level,
            'exp',u.exp,'exp_max',(SELECT l.exp FROM level_up l WHERE l.level=u.level AND l.rebirth_level=u.rebirth_level LIMIT 1),
            'loyalty',u.loyalty,'loyalty_monthly',u.loyalty_monthly,'manner',u.manner_point,
            'str',u.strong,'sta',u.sta,'dex',u.dex,'intel',u.intel,'cha',u.cha,'points',u.points,
            'reb_str',u.reb_str,'reb_sta',u.reb_sta,'reb_dex',u.reb_dex,'reb_intel',u.reb_intel,'reb_cha',u.reb_cha,
            'authority',u.authority,'gold',u.gold,
            'online',EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)),
            'account',(SELECT a.str_account_id FROM account_char a WHERE lower(u.str_user_id) IN (lower(a.str_char_id1),lower(a.str_char_id2),lower(a.str_char_id3),lower(a.str_char_id4)) LIMIT 1),
            'equip',(SELECT jsonb_build_object(
                'attack',COALESCE(MAX(CASE WHEN ui.slot_index IN (6,8) THEN i.damage END),0),
                'defence',COALESCE(SUM(i.ac),0),
                'fire',COALESCE(SUM(i.fire_r),0),'cold',COALESCE(SUM(i.cold_r),0),'lightning',COALESCE(SUM(i.lightning_r),0),
                'magic',COALESCE(SUM(i.magic_r),0),'curse',COALESCE(SUM(i.curse_r),0),'poison',COALESCE(SUM(i.poison_r),0),
                'str_b',COALESCE(SUM(i.str_b),0),'sta_b',COALESCE(SUM(i.sta_b),0),'dex_b',COALESCE(SUM(i.dex_b),0),
                'intel_b',COALESCE(SUM(i.intel_b),0),'cha_b',COALESCE(SUM(i.cha_b),0))
              FROM user_items ui JOIN item i ON i.num=ui.item_id
              WHERE lower(ui.str_user_id)=lower(u.str_user_id) AND ui.slot_index<14 AND ui.item_id>0)
        ) FROM userdata u WHERE lower(u.str_user_id)=lower($1)"#,
    )
    .bind(&name)
    .fetch_optional(&app.pool)
    .await
    .map_err(internal("character sheet"))?;
    let mut v = v.ok_or(ApiError::NotFound)?;
    if let Some(acct) = v["account"].as_str().map(str::to_owned) {
        let row = sqlx::query("SELECT cash_point, bonus_cash_point FROM tb_user WHERE lower(str_account_id)=lower($1)")
            .bind(&acct)
            .fetch_optional(&app.pool)
            .await
            .map_err(internal("sheet balance"))?;
        if let Some(r) = row {
            v["kc"] = json!(r.try_get::<i32, _>("cash_point").unwrap_or(0));
            v["tl"] = json!(r.try_get::<Option<i32>, _>("bonus_cash_point").ok().flatten().unwrap_or(0));
        }
    }
    Ok(Json(v))
}

#[derive(Deserialize)]
struct StatsIn {
    level: i16,
    rebirth: i16,
    exp: i64,
    loyalty: i32,
    loyalty_monthly: i32,
    manner: i32,
    str: i16,
    sta: i16,
    dex: i16,
    intel: i16,
    cha: i16,
    points: i16,
    reason: String,
}

async fn is_online(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, name: &str) -> Result<bool, ApiError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_char_id)=lower($1))")
        .bind(name)
        .fetch_one(&mut **tx)
        .await
        .map_err(internal("online"))
}

async fn character_stats(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
    Json(p): Json<StatsIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let stats = [p.str, p.sta, p.dex, p.intel, p.cha, p.points];
    if !(1..=83).contains(&p.level)
        || !(0..=255).contains(&p.rebirth)
        || p.exp < 0
        || p.loyalty < 0
        || p.loyalty_monthly < 0
        || p.manner < 0
        || stats.iter().any(|v| !(0..=255).contains(v))
        || p.reason.trim().chars().count() < 3
    {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    if is_online(&mut tx, &name).await? {
        return Err(ApiError::Conflict);
    }
    let n = sqlx::query("UPDATE userdata SET level=$2,rebirth_level=$3,exp=$4,loyalty=$5,loyalty_monthly=$6,manner_point=$7,strong=$8,sta=$9,dex=$10,intel=$11,cha=$12,points=$13,dt_update_time=NOW() WHERE lower(str_user_id)=lower($1)")
        .bind(&name)
        .bind(p.level)
        .bind(p.rebirth)
        .bind(p.exp)
        .bind(p.loyalty)
        .bind(p.loyalty_monthly)
        .bind(p.manner)
        .bind(p.str)
        .bind(p.sta)
        .bind(p.dex)
        .bind(p.intel)
        .bind(p.cha)
        .bind(p.points)
        .execute(&mut *tx)
        .await
        .map_err(internal("character stats"))?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    audit(
        &mut tx,
        &s.actor,
        "character.stats",
        &name,
        json!({"level":p.level,"stats":[p.str,p.sta,p.dex,p.intel,p.cha],"points":p.points,"reason":p.reason.trim()}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct ReasonIn {
    reason: String,
}

/// Give every allocated stat point back; the class base stats come from the active beginner set.
async fn character_stat_reset(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
    Json(p): Json<ReasonIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.reason.trim().chars().count() < 3 {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    if is_online(&mut tx, &name).await? {
        return Err(ApiError::Conflict);
    }
    let row: Option<(i16, i16, i16, i16, i16, i16, i16)> =
        sqlx::query_as("SELECT class,strong,sta,dex,intel,cha,points FROM userdata WHERE lower(str_user_id)=lower($1) FOR UPDATE")
            .bind(&name)
            .fetch_optional(&mut *tx)
            .await
            .map_err(internal("reset read"))?;
    let (class, st, sta, dex, intel, cha, points) = row.ok_or(ApiError::NotFound)?;
    let base: Option<(i16, i16, i16, i16, i16)> = sqlx::query_as(
        "SELECT strength,health,dexterity,intelligence,magic_power FROM create_new_char_value WHERE class_type=$1 AND job_type=COALESCE((SELECT beginner_type FROM beginner_settings WHERE server_no=1),0)",
    )
    .bind(class % 100)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal("reset base"))?;
    let (bs, bh, bd, bi, bm) = base.ok_or(ApiError::BadRequest)?;
    let refund = (st - bs).max(0) + (sta - bh).max(0) + (dex - bd).max(0) + (intel - bi).max(0) + (cha - bm).max(0);
    sqlx::query("UPDATE userdata SET strong=LEAST(strong,$2),sta=LEAST(sta,$3),dex=LEAST(dex,$4),intel=LEAST(intel,$5),cha=LEAST(cha,$6),points=$7 WHERE lower(str_user_id)=lower($1)")
        .bind(&name)
        .bind(bs)
        .bind(bh)
        .bind(bd)
        .bind(bi)
        .bind(bm)
        .bind(points + refund)
        .execute(&mut *tx)
        .await
        .map_err(internal("reset write"))?;
    audit(&mut tx, &s.actor, "character.stat_reset", &name, json!({"refund":refund,"reason":p.reason.trim()}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"refund":refund})))
}
