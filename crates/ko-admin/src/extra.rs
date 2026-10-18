//! Endpoints added on top of the ported panel: dashboard, accounts (KC/TL, bans), the four item
//! containers (character bags, normal bank, VIP bank, clan bank), item details, events,
//! audit log and the icon route. Everything goes straight to PostgreSQL.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
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
        .route("/jstkoadminpanel/api/dashboard", get(dashboard))
        .route("/jstkoadminpanel/api/online", get(online))
        .route("/jstkoadminpanel/api/accounts", get(accounts))
        .route("/jstkoadminpanel/api/accounts/{id}", get(account))
        .route("/jstkoadminpanel/api/accounts/{id}/balance", post(balance))
        .route("/jstkoadminpanel/api/accounts/{id}/ban", post(ban))
        .route("/jstkoadminpanel/api/accounts/{id}/authority", post(set_authority))
        .route("/jstkoadminpanel/api/accounts/{id}/activate", post(activate))
        .route("/jstkoadminpanel/api/game-options", get(game_options).post(set_game_options))
        .route("/jstkoadminpanel/api/clear-online/{name}", post(clear_online))
        .route("/jstkoadminpanel/api/item-search", get(item_search))
        .route("/jstkoadminpanel/api/item/{id}", get(item_detail))
        .route("/jstkoadminpanel/api/icon-range", post(icon_range))
        .route("/jstkoadminpanel/api/icon-debug/{id}", get(icon_debug))
        .route("/jstkoadminpanel/api/clans", get(clans))
        .route("/jstkoadminpanel/api/clans/{id}", get(clan_detail))
        .route("/jstkoadminpanel/api/characters/{name}/clan", get(character_clan))
        .route("/jstkoadminpanel/api/container", get(container))
        .route("/jstkoadminpanel/api/container/set", post(container_set))
        .route("/jstkoadminpanel/api/container/move", post(container_move))
        .route("/jstkoadminpanel/api/events", get(events))
        .route("/jstkoadminpanel/api/events/{id}", post(event_edit))
        .route("/jstkoadminpanel/api/audit", get(audit_log))
        .route("/jstkoadminpanel/api/commands", get(commands).post(send_command))
        .route("/jstkoadminpanel/icons/{file}", get(icon))
        .route("/jstkoadminpanel/static/{file}", get(static_file))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

// ── dashboard ────────────────────────────────────────────────────────────

async fn dashboard(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT jsonb_build_object(
            'accounts',(SELECT count(*) FROM tb_user),
            'characters',(SELECT count(*) FROM userdata),
            'online',(SELECT count(*) FROM currentuser),
            'clans',(SELECT count(*) FROM knights),
            'items',(SELECT count(*) FROM item),
            'monsters',(SELECT count(*) FROM npc_template WHERE is_monster),
            'karus',(SELECT count(*) FROM userdata WHERE nation=1),
            'elmorad',(SELECT count(*) FROM userdata WHERE nation=2),
            'new_today',(SELECT count(*) FROM tb_user WHERE dt_create_time > now()-interval '1 day'),
            'banned',(SELECT count(*) FROM tb_user WHERE str_authority<0),
            'heartbeat_age',(SELECT EXTRACT(EPOCH FROM now()-at)::int FROM admin_heartbeat WHERE id=1),
            'pending_commands',(SELECT count(*) FROM admin_command WHERE status='pending' AND created_at>now()-interval '10 minutes'),
            'db_time',now(),
            'recent_audit',(SELECT COALESCE(jsonb_agg(a),'[]'::jsonb) FROM (SELECT occurred_at,actor,action,target FROM admin_audit_log ORDER BY audit_id DESC LIMIT 12) a)
        )"#,
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("dashboard"))?;
    Ok(Json(v))
}

async fn online(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(x),'[]'::jsonb) FROM (
            SELECT c.str_account_id account, c.str_char_id name, c.str_client_ip ip, u.nation, u.class, u.level, u.zone
            FROM currentuser c LEFT JOIN userdata u ON lower(u.str_user_id)=lower(c.str_char_id)
            ORDER BY c.str_char_id LIMIT 500) x"#,
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("online"))?;
    Ok(Json(v))
}

// ── accounts ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Q {
    q: Option<String>,
    kind: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn accounts(headers: HeaderMap, State(app): State<App>, Query(q): Query<Q>) -> R {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    let like = format!("%{term}%");
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(x),'[]'::jsonb) FROM (
            SELECT t.str_account_id account, t.cash_point kc, t.bonus_cash_point tl, t.str_authority authority,
                   t.account_check account_check,
                   t.b_premium_type premium, t.dt_create_time created, t.email,
                   EXISTS(SELECT 1 FROM currentuser c WHERE c.str_account_id=t.str_account_id) online,
                   to_jsonb(ARRAY_REMOVE(ARRAY[ac.str_char_id1,ac.str_char_id2,ac.str_char_id3,ac.str_char_id4],NULL)) chars
            FROM tb_user t LEFT JOIN account_char ac ON ac.str_account_id=t.str_account_id
            WHERE ($1='' OR t.str_account_id ILIKE $2 OR ac.str_char_id1 ILIKE $2 OR ac.str_char_id2 ILIKE $2
               OR ac.str_char_id3 ILIKE $2 OR ac.str_char_id4 ILIKE $2 OR t.email ILIKE $2)
              AND ($5='' OR ($5='banned' AND t.str_authority<0)
                   OR ($5='online' AND EXISTS(SELECT 1 FROM currentuser c WHERE c.str_account_id=t.str_account_id))
                   OR ($5='gm' AND t.str_authority=0)
                   OR ($5='unverified' AND t.account_check=0))
            ORDER BY t.id DESC LIMIT $3 OFFSET $4) x"#,
    )
    .bind(&term)
    .bind(&like)
    .bind(q.limit.unwrap_or(50).clamp(1, 200))
    .bind(q.offset.unwrap_or(0).max(0))
    .bind(q.kind.clone().unwrap_or_default())
    .fetch_one(&app.pool)
    .await
    .map_err(internal("accounts search"))?;
    Ok(Json(v))
}

async fn account(headers: HeaderMap, State(app): State<App>, Path(id): Path<String>) -> R {
    auth(&headers, false)?;
    // passwords, seal password and OTP are never selected
    let v: Option<Value> = sqlx::query_scalar(
        r#"SELECT jsonb_build_object(
            'account', t.str_account_id, 'kc', t.cash_point, 'tl', t.bonus_cash_point,
            'authority', t.str_authority, 'account_check', t.account_check,
            'premium', t.b_premium_type, 'premium_until', t.dt_premium_time,
            'created', t.dt_create_time, 'email', t.email, 'name', t.user_name, 'surname', t.user_surname,
            'phone', t.user_phone_number, 'security_question', t.security_question,
            'client_ip', t.str_client_ip, 'punishment_date', t.punishment_date, 'punishment_days', t.punishment_period,
            'online', EXISTS(SELECT 1 FROM currentuser c WHERE c.str_account_id=t.str_account_id),
            'warehouse_coins', COALESCE((SELECT coins FROM user_warehouse_coins w WHERE w.str_account_id=t.str_account_id),0),
            'characters', COALESCE((SELECT jsonb_agg(jsonb_build_object('name',u.str_user_id,'nation',u.nation,'class',u.class,
                'level',u.level,'rebirth',u.rebirth_level,'zone',u.zone,'loyalty',u.loyalty,'gold',u.gold,'authority',u.authority,
                'clan',u.knights,'last_login',u.n_last_login,
                'online',EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id))))
               FROM account_char ac JOIN userdata u ON lower(u.str_user_id) IN (lower(ac.str_char_id1),lower(ac.str_char_id2),lower(ac.str_char_id3),lower(ac.str_char_id4))
               WHERE ac.str_account_id=t.str_account_id),'[]'::jsonb)
        ) FROM tb_user t WHERE lower(t.str_account_id)=lower($1)"#,
    )
    .bind(&id)
    .fetch_optional(&app.pool)
    .await
    .map_err(internal("account detail"))?;
    v.map(Json).ok_or(ApiError::NotFound)
}

#[derive(Deserialize)]
struct BalanceIn {
    /// "kc" = tb_user.cash_point, "tl" = tb_user.bonus_cash_point
    field: String,
    /// "add" (negative removes) or "set"
    mode: String,
    amount: i64,
    reason: String,
}

async fn account_online(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, account: &str) -> Result<bool, ApiError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_account_id)=lower($1))")
        .bind(account)
        .fetch_one(&mut **tx)
        .await
        .map_err(internal("online check"))
}

async fn balance(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<String>,
    Json(p): Json<BalanceIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let col = match p.field.as_str() {
        "kc" => "cash_point",
        "tl" => "bonus_cash_point",
        _ => return Err(ApiError::BadRequest),
    };
    if p.reason.trim().chars().count() < 3 || !matches!(p.mode.as_str(), "add" | "set") || p.amount.abs() > 2_000_000_000 {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    if account_online(&mut tx, &id).await? {
        // the running game server keeps its own copy of the balance
        return Err(ApiError::Conflict);
    }
    let before: i32 = sqlx::query_scalar(&format!("SELECT COALESCE({col},0) FROM tb_user WHERE lower(str_account_id)=lower($1) FOR UPDATE"))
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal("balance read"))?
        .ok_or(ApiError::NotFound)?;
    let after = match p.mode.as_str() {
        "set" => p.amount,
        _ => before as i64 + p.amount,
    };
    if !(0..=2_000_000_000).contains(&after) {
        return Err(ApiError::BadRequest);
    }
    sqlx::query(&format!("UPDATE tb_user SET {col}=$2 WHERE lower(str_account_id)=lower($1)"))
        .bind(&id)
        .bind(after as i32)
        .execute(&mut *tx)
        .await
        .map_err(internal("balance write"))?;
    audit(
        &mut tx,
        &s.actor,
        "account.balance",
        &id,
        json!({"field":p.field,"mode":p.mode,"before":before,"after":after,"reason":p.reason.trim()}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"before":before,"after":after})))
}

#[derive(Deserialize)]
struct BanIn {
    ban: bool,
    days: Option<i32>,
    reason: String,
}

async fn ban(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<String>,
    Json(p): Json<BanIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.reason.trim().chars().count() < 3 {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let before: i16 = sqlx::query_scalar("SELECT str_authority FROM tb_user WHERE lower(str_account_id)=lower($1) FOR UPDATE")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal("ban read"))?
        .ok_or(ApiError::NotFound)?;
    let (new_auth, days) = if p.ban { (-1i16, p.days.unwrap_or(0).clamp(0, 36500)) } else { (1i16, 0) };
    sqlx::query("UPDATE tb_user SET str_authority=$2, punishment_date=CASE WHEN $2<0 THEN NOW() ELSE NULL END, punishment_period=$3 WHERE lower(str_account_id)=lower($1)")
        .bind(&id)
        .bind(new_auth)
        .bind(days)
        .execute(&mut *tx)
        .await
        .map_err(internal("ban write"))?;
    audit(
        &mut tx,
        &s.actor,
        if p.ban { "account.ban" } else { "account.unban" },
        &id,
        json!({"before_authority":before,"days":days,"reason":p.reason.trim()}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    if p.ban {
        queue_command(&app.pool, "kick_account", json!({"account": id})).await?;
    }
    Ok(Json(json!({"ok":true,"authority":new_auth})))
}

#[derive(Deserialize)]
struct AuthorityIn {
    /// str_authority: 0 or 2 = GM (unlocks /admin/ on the player web panel), 1 = normal player,
    /// negative = banned (use /ban instead so the ban/punishment fields stay consistent).
    authority: i16,
    reason: String,
}

async fn set_authority(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<String>,
    Json(p): Json<AuthorityIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.reason.trim().chars().count() < 3 || !(0..=2).contains(&p.authority) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let before: i16 = sqlx::query_scalar("SELECT str_authority FROM tb_user WHERE lower(str_account_id)=lower($1) FOR UPDATE")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal("authority read"))?
        .ok_or(ApiError::NotFound)?;
    sqlx::query("UPDATE tb_user SET str_authority=$2 WHERE lower(str_account_id)=lower($1)")
        .bind(&id)
        .bind(p.authority)
        .execute(&mut *tx)
        .await
        .map_err(internal("authority write"))?;
    audit(
        &mut tx,
        &s.actor,
        "account.set_authority",
        &id,
        json!({"before":before,"after":p.authority,"reason":p.reason.trim()}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"authority":p.authority})))
}

/// Manually marks a web-panel registration as e-mail-verified (tb_user.account_check), for when
/// SMTP isn't configured / the activation mail never arrived -- bypasses the e-mail link entirely.
async fn activate(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<String>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let updated: Option<String> = sqlx::query_scalar(
        "UPDATE tb_user SET account_check=1 WHERE lower(str_account_id)=lower($1) AND account_check=0 RETURNING str_account_id",
    )
    .bind(&id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal("activate"))?;
    let Some(account) = updated else {
        return Err(ApiError::NotFound);
    };
    audit(&mut tx, &s.actor, "account.activate", &account, json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

async fn game_options(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM (SELECT maintenance_mode, char_select_login, open_otp, auto_register, \
         free_limit, total_user_limit, server_ip FROM game_options WHERE id=1) t",
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("game_options"))?;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct GameOptionsIn {
    /// Whether the game client's own "type a new ID/password at the login screen" flow creates
    /// an account on the spot. Turn off to force registration through the web panel only.
    auto_register: bool,
}

async fn set_game_options(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<GameOptionsIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    sqlx::query("UPDATE game_options SET auto_register=$1 WHERE id=1")
        .bind(p.auto_register)
        .execute(&mut *tx)
        .await
        .map_err(internal("game_options write"))?;
    audit(&mut tx, &s.actor, "game_options.set", "auto_register", json!({"auto_register": p.auto_register}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"auto_register":p.auto_register})))
}

/// A character that is shown online although the game server is down keeps its rows locked.
async fn clear_online(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let age: Option<i32> = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM now()-at)::int FROM admin_heartbeat WHERE id=1")
        .fetch_optional(&app.pool)
        .await
        .map_err(internal("heartbeat"))?;
    if age.is_some_and(|a| a < 30) {
        // the server is alive: use the kick command instead of removing the row under it
        queue_command(&app.pool, "kick", json!({"name": name})).await?;
        return Ok(Json(json!({"ok":true,"queued":true})));
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let n = sqlx::query("DELETE FROM currentuser WHERE lower(str_char_id)=lower($1)")
        .bind(&name)
        .execute(&mut *tx)
        .await
        .map_err(internal("clear online"))?
        .rows_affected();
    audit(&mut tx, &s.actor, "character.clear_online", &name, json!({"rows":n}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"removed":n})))
}

// ── items ────────────────────────────────────────────────────────────────

async fn item_search(headers: HeaderMap, State(app): State<App>, Query(q): Query<Q>) -> R {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    let kind: Option<i32> = q.kind.and_then(|k| k.parse().ok());
    if term.is_empty() && kind.is_none() {
        return Ok(Json(json!([])));
    }
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(x),'[]'::jsonb) FROM (
            SELECT num id, str_name name, item_icon_id1 icon, kind, slot, countable, weight, duration, req_level
            FROM item
            WHERE ($1='' OR CAST(num AS TEXT)=$1 OR str_name ILIKE $2)
              AND ($3::int IS NULL OR kind=$3)
            ORDER BY CASE WHEN CAST(num AS TEXT)=$1 THEN 0 ELSE 1 END, str_name, num
            LIMIT $4) x"#,
    )
    .bind(&term)
    .bind(format!("%{term}%"))
    .bind(kind)
    .bind(q.limit.unwrap_or(60).clamp(1, 200))
    .fetch_one(&app.pool)
    .await
    .map_err(internal("item search"))?;
    Ok(Json(v))
}

async fn item_detail(headers: HeaderMap, State(app): State<App>, Path(id): Path<i32>) -> R {
    auth(&headers, false)?;
    let v: Option<Value> = sqlx::query_scalar("SELECT to_jsonb(i) FROM item i WHERE num=$1")
        .bind(id)
        .fetch_optional(&app.pool)
        .await
        .map_err(internal("item detail"))?;
    v.map(Json).ok_or(ApiError::NotFound)
}

const PLACEHOLDER: &[u8] = include_bytes!("../assets/noimage.png");

/// Icon file the client uses for an `item_icon_id1`: itemicon_<lead>_<dddd>_<dd>_<d>.jpg, i.e. the last
/// seven digits split 4/2/1 behind the leading digits. Colour variants fall back to the base icon.
fn icon_candidates(id: &str) -> Vec<String> {
    let fname = |s: &str| -> Option<String> {
        if s.len() < 8 {
            return None;
        }
        let n = s.len();
        Some(format!("itemicon_{}_{}_{}_{}.jpg", &s[..n - 7], &s[n - 7..n - 3], &s[n - 3..n - 1], &s[n - 1..]))
    };
    let mut out: Vec<String> = fname(id).into_iter().collect();
    if let Ok(v) = id.parse::<u64>() {
        out.extend(fname(&(v / 10 * 10).to_string()));
    }
    out
}

async fn icon(State(app): State<App>, Path(file): Path<String>) -> Response {
    let id: String = file.chars().take_while(|c| c.is_ascii_digit()).collect();
    let cache = (header::CACHE_CONTROL, "public, max-age=86400");
    let jpeg = (header::CONTENT_TYPE, "image/jpeg");
    if !id.is_empty() && id != "0" {
        for name in icon_candidates(&id) {
            if let Ok(bytes) = tokio::fs::read(app.icon_dir.join(&name)).await {
                return ([jpeg.clone(), cache.clone()], bytes).into_response();
            }
        }
    }
    // missing picture: the folder's own bos.jpg (never cached, so a later fix shows up at once)
    let no_cache = (header::CACHE_CONTROL, "no-store");
    match tokio::fs::read(app.icon_dir.join("bos.jpg")).await {
        Ok(bytes) => ([jpeg, no_cache], bytes).into_response(),
        Err(_) => ([(header::CONTENT_TYPE, "image/png"), no_cache], PLACEHOLDER).into_response(),
    }
}

// ── clans ────────────────────────────────────────────────────────────────

async fn clans(headers: HeaderMap, State(app): State<App>, Query(q): Query<Q>) -> R {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(x),'[]'::jsonb) FROM (
            SELECT k.id_num id, k.id_name name, k.nation, k.members, k.chief, k.points, k.gold,
                   COALESCE(b.gold,0) bank_gold, COALESCE(b.max_tabs,1) max_tabs
            FROM knights k LEFT JOIN guild_bank b ON b.knights_id=k.id_num
            WHERE $1='' OR k.id_name ILIKE $2 OR CAST(k.id_num AS TEXT)=$1
            ORDER BY k.id_name LIMIT 50) x"#,
    )
    .bind(&term)
    .bind(format!("%{term}%"))
    .fetch_one(&app.pool)
    .await
    .map_err(internal("clans"))?;
    Ok(Json(v))
}

async fn character_clan(headers: HeaderMap, State(app): State<App>, Path(name): Path<String>) -> R {
    auth(&headers, false)?;
    let v: Option<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id',k.id_num,'name',k.id_name) FROM userdata u JOIN knights k ON k.id_num=u.knights WHERE lower(u.str_user_id)=lower($1) AND u.knights>0",
    )
    .bind(&name)
    .fetch_optional(&app.pool)
    .await
    .map_err(internal("character clan"))?;
    Ok(Json(v.unwrap_or_else(|| json!({}))))
}

async fn clan_detail(headers: HeaderMap, State(app): State<App>, Path(id): Path<i16>) -> R {
    auth(&headers, false)?;
    let v: Option<Value> = sqlx::query_scalar(
        r#"SELECT jsonb_build_object('id',k.id_num,'name',k.id_name,'nation',k.nation,'chief',k.chief,'points',k.points,'gold',k.gold,
            'members_count',k.members,'bank_gold',COALESCE(b.gold,0),'max_tabs',COALESCE(b.max_tabs,1),
            'members',COALESCE((SELECT jsonb_agg(m ORDER BY m.level DESC, m.name) FROM (
                SELECT u.str_user_id name,u.level,u.class,u.fame,u.loyalty,
                       EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)) online
                FROM userdata u WHERE u.knights=k.id_num LIMIT 300) m),'[]'::jsonb))
           FROM knights k LEFT JOIN guild_bank b ON b.knights_id=k.id_num WHERE k.id_num=$1"#,
    )
    .bind(id)
    .fetch_optional(&app.pool)
    .await
    .map_err(internal("clan detail"))?;
    v.map(Json).ok_or(ApiError::NotFound)
}

// ── item containers ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Inv,
    Wh,
    Vip,
    Clan,
}

impl Kind {
    fn parse(s: &str) -> Result<Self, ApiError> {
        match s {
            "inv" => Ok(Self::Inv),
            "wh" => Ok(Self::Wh),
            "vip" => Ok(Self::Vip),
            "clan" => Ok(Self::Clan),
            _ => Err(ApiError::BadRequest),
        }
    }
    fn table(self) -> &'static str {
        match self {
            Self::Inv => "user_items",
            Self::Wh => "user_warehouse",
            Self::Vip => "vip_warehouse_items",
            Self::Clan => "guild_bank_item",
        }
    }
    fn owner_col(self) -> &'static str {
        match self {
            Self::Inv => "str_user_id",
            Self::Wh | Self::Vip => "str_account_id",
            Self::Clan => "knights_id",
        }
    }
    fn slot_col(self) -> &'static str {
        if self == Self::Clan { "slot_id" } else { "slot_index" }
    }
    fn capacity(self) -> i32 {
        match self {
            Self::Inv => 97, // 0-95 plus the third cospre bag at 96
            Self::Wh => 192,
            Self::Vip => 48,
            Self::Clan => 24,
        }
    }
    /// (json key, db column) in canonical order; missing columns are skipped for the kind.
    fn cols(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Inv | Self::Wh => &[
                ("id", "item_id"),
                ("count", "count"),
                ("durability", "durability"),
                ("flag", "flag"),
                ("original_flag", "original_flag"),
                ("serial", "serial_num"),
                ("expire", "expire_time"),
            ],
            Self::Vip => &[
                ("id", "item_id"),
                ("count", "count"),
                ("durability", "durability"),
                ("flag", "flag"),
                ("serial", "serial_num"),
                ("expire", "expire_time"),
            ],
            Self::Clan => &[
                ("id", "item_id"),
                ("count", "item_count"),
                ("durability", "cur_durability"),
                ("max_durability", "max_durability"),
                ("flag", "flag"),
                ("expire", "expiry_time"),
            ],
        }
    }
}

/// Owner as the container tables key it: character / account text, or the clan id.
enum Owner {
    Text(String),
    Id(i32),
}

/// Resolve what the UI sent (character name, account id or clan id) to the container's real owner key.
async fn resolve_owner(pool: &sqlx::PgPool, kind: Kind, owner: &str) -> Result<(Owner, String), ApiError> {
    match kind {
        Kind::Inv => {
            let name: String = sqlx::query_scalar("SELECT str_user_id FROM userdata WHERE lower(str_user_id)=lower($1)")
                .bind(owner)
                .fetch_optional(pool)
                .await
                .map_err(internal("owner"))?
                .ok_or(ApiError::NotFound)?;
            Ok((Owner::Text(name.clone()), name))
        }
        Kind::Wh | Kind::Vip => {
            // owner may be a character name or an account id
            let acct: Option<String> = sqlx::query_scalar(
                "SELECT str_account_id FROM account_char WHERE lower(str_char_id1)=lower($1) OR lower(str_char_id2)=lower($1) OR lower(str_char_id3)=lower($1) OR lower(str_char_id4)=lower($1) OR lower(str_account_id)=lower($1) LIMIT 1",
            )
            .bind(owner)
            .fetch_optional(pool)
            .await
            .map_err(internal("owner"))?;
            let acct = acct.ok_or(ApiError::NotFound)?;
            Ok((Owner::Text(acct.clone()), acct))
        }
        Kind::Clan => {
            let id: i32 = owner.parse().map_err(|_| ApiError::BadRequest)?;
            let ok: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM knights WHERE id_num=$1)")
                .bind(id as i16)
                .fetch_one(pool)
                .await
                .map_err(internal("owner"))?;
            if !ok {
                return Err(ApiError::NotFound);
            }
            Ok((Owner::Id(id), id.to_string()))
        }
    }
}

/// Is somebody who uses this container logged in? Mutations are blocked then.
async fn owner_online(pool: &sqlx::PgPool, kind: Kind, owner: &Owner) -> Result<bool, ApiError> {
    let r = match (kind, owner) {
        (Kind::Inv, Owner::Text(n)) => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_char_id)=lower($1))").bind(n).fetch_one(pool).await,
        (Kind::Wh | Kind::Vip, Owner::Text(a)) => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_account_id)=lower($1))").bind(a).fetch_one(pool).await,
        (Kind::Clan, Owner::Id(id)) => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM currentuser c JOIN userdata u ON lower(u.str_user_id)=lower(c.str_char_id) WHERE u.knights=$1)").bind(*id as i16).fetch_one(pool).await,
        _ => return Err(ApiError::BadRequest),
    };
    r.map_err(internal("online check"))
}

#[derive(Deserialize)]
struct ContainerQ {
    #[serde(rename = "type")]
    kind: String,
    owner: String,
    tab: Option<i32>,
}

async fn container(headers: HeaderMap, State(app): State<App>, Query(q): Query<ContainerQ>) -> R {
    auth(&headers, false)?;
    let kind = Kind::parse(&q.kind)?;
    let (owner, label) = resolve_owner(&app.pool, kind, &q.owner).await?;
    let online = owner_online(&app.pool, kind, &owner).await?;
    let tab = q.tab.unwrap_or(0).clamp(0, 8);

    let select_cols: String = kind
        .cols()
        .iter()
        .map(|(k, c)| format!("'{k}', t.{c}"))
        .collect::<Vec<_>>()
        .join(",");
    let tab_filter = if kind == Kind::Clan { " AND t.tab_index=$2" } else { "" };
    let sql = format!(
        "SELECT COALESCE(jsonb_agg(jsonb_build_object('slot',t.{slot},{select_cols},'name',COALESCE(i.str_name,''),'icon',COALESCE(i.item_icon_id1,0),'kind',i.kind,'countable',COALESCE(i.countable,0)) ORDER BY t.{slot}),'[]'::jsonb) FROM {table} t LEFT JOIN item i ON i.num=t.item_id WHERE {ocol}{cmp}$1{tab_filter} AND t.item_id<>0",
        slot = kind.slot_col(),
        table = kind.table(),
        ocol = if kind == Kind::Inv { "lower(t.str_user_id)" } else if kind == Kind::Clan { "t.knights_id" } else { "t.str_account_id" },
        cmp = " = ",
    );
    let mut query = sqlx::query_scalar::<_, Value>(&sql);
    query = match &owner {
        Owner::Text(s) if kind == Kind::Inv => query.bind(s.to_lowercase()),
        Owner::Text(s) => query.bind(s.clone()),
        Owner::Id(i) => query.bind(*i),
    };
    if kind == Kind::Clan {
        query = query.bind(tab as i16);
    }
    let slots = query.fetch_one(&app.pool).await.map_err(internal("container read"))?;

    // extras: gold / coins / unlocked tabs
    let extra: Value = match (&kind, &owner) {
        (Kind::Wh, Owner::Text(a)) => json!({"coins": sqlx::query_scalar::<_, i32>("SELECT coins FROM user_warehouse_coins WHERE str_account_id=$1").bind(a).fetch_optional(&app.pool).await.map_err(internal("coins"))?.unwrap_or(0)}),
        (Kind::Vip, Owner::Text(a)) => {
            let r = sqlx::query("SELECT vault_expiry FROM vip_warehouse WHERE str_account_id=$1").bind(a).fetch_optional(&app.pool).await.map_err(internal("vip"))?;
            json!({"vault_expiry": r.and_then(|r| r.try_get::<i32,_>("vault_expiry").ok()).unwrap_or(0)})
        }
        (Kind::Clan, Owner::Id(id)) => {
            let r = sqlx::query("SELECT gold,max_tabs,permissions FROM guild_bank WHERE knights_id=$1").bind(*id).fetch_optional(&app.pool).await.map_err(internal("bank"))?;
            match r {
                Some(r) => json!({"gold": r.try_get::<i64,_>("gold").unwrap_or(0), "max_tabs": r.try_get::<i16,_>("max_tabs").unwrap_or(1), "permissions": r.try_get::<i16,_>("permissions").unwrap_or(0)}),
                None => json!({"gold":0,"max_tabs":1,"permissions":0}),
            }
        }
        _ => json!({}),
    };
    Ok(Json(json!({
        "type": q.kind, "owner": label, "tab": tab, "online": online,
        "capacity": kind.capacity(), "slots": slots, "extra": extra
    })))
}

#[derive(Deserialize)]
struct SetIn {
    #[serde(rename = "type")]
    kind: String,
    owner: String,
    #[serde(default)]
    tab: i32,
    slot: i32,
    /// 0 empties the slot
    id: i32,
    #[serde(default = "one")]
    count: i32,
    #[serde(default)]
    durability: i32,
    #[serde(default)]
    flag: i32,
    #[serde(default)]
    expire: i32,
    #[serde(default)]
    reason: String,
}
fn one() -> i32 {
    1
}

fn new_serial() -> i64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64)
        .wrapping_abs()
        .max(1)
}

/// Write one slot (all kinds). `vals` is keyed by the kind's canonical JSON names.
async fn write_slot(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    kind: Kind,
    owner: &Owner,
    tab: i32,
    slot: i32,
    vals: &[(&str, i64)],
) -> Result<(), ApiError> {
    let cols = kind.cols();
    let mut names = vec![kind.owner_col().to_string()];
    if kind == Kind::Clan {
        names.push("tab_index".into());
    }
    names.push(kind.slot_col().to_string());
    let fixed = names.len();
    for (key, col) in cols {
        if vals.iter().any(|(k, _)| k == key) {
            names.push((*col).to_string());
        }
    }
    let placeholders: Vec<String> = (1..=names.len()).map(|i| format!("${i}")).collect();
    let conflict: Vec<String> = names[..fixed].to_vec();
    let updates: Vec<String> = names[fixed..].iter().map(|c| format!("{c}=EXCLUDED.{c}")).collect();
    let sql = format!(
        "INSERT INTO {t}({n}) VALUES({p}) ON CONFLICT({c}) DO UPDATE SET {u}",
        t = kind.table(),
        n = names.join(","),
        p = placeholders.join(","),
        c = conflict.join(","),
        u = updates.join(","),
    );
    let mut q = sqlx::query(&sql);
    q = match owner {
        Owner::Text(s) => q.bind(s.clone()),
        Owner::Id(i) => q.bind(*i),
    };
    if kind == Kind::Clan {
        q = q.bind(tab as i16);
    }
    q = if kind == Kind::Clan { q.bind(slot) } else { q.bind(slot as i16) };
    for (key, col) in cols {
        if let Some((_, v)) = vals.iter().find(|(k, _)| k == key) {
            q = match *col {
                "item_id" | "expire_time" | "expiry_time" => q.bind(*v as i32),
                "serial_num" => q.bind(*v),
                _ => q.bind(*v as i16),
            };
        }
    }
    q.execute(&mut **tx).await.map_err(internal("slot write"))?;
    Ok(())
}

fn slot_in_range(kind: Kind, slot: i32, tab: i32) -> bool {
    match kind {
        Kind::Clan => (0..24).contains(&slot) && (0..9).contains(&tab),
        _ => (0..kind.capacity()).contains(&slot),
    }
}

async fn container_set(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<SetIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let kind = Kind::parse(&p.kind)?;
    if !slot_in_range(kind, p.slot, p.tab) || p.id < 0 || !(0..=32767).contains(&p.count) {
        return Err(ApiError::BadRequest);
    }
    if p.id == 0 && p.reason.trim().chars().count() < 3 {
        return Err(ApiError::BadRequest);
    }
    let (owner, label) = resolve_owner(&app.pool, kind, &p.owner).await?;
    if owner_online(&app.pool, kind, &owner).await? {
        return Err(ApiError::Conflict);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let mut vals: Vec<(&str, i64)> = Vec::new();
    if p.id == 0 {
        vals.extend([("id", 0), ("count", 0), ("durability", 0), ("flag", 0), ("original_flag", 0), ("serial", 0), ("expire", 0), ("max_durability", 0)]);
    } else {
        let row = sqlx::query("SELECT countable,duration FROM item WHERE num=$1")
            .bind(p.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(internal("item"))?
            .ok_or(ApiError::BadRequest)?;
        let countable: i16 = row.try_get::<Option<i16>, _>("countable").ok().flatten().unwrap_or(0);
        let max_dur: i32 = row.try_get::<Option<i16>, _>("duration").ok().flatten().unwrap_or(0) as i32;
        let count = if countable == 0 { 1 } else { p.count.max(1) };
        let dur = if p.durability > 0 { p.durability } else { max_dur };
        vals.extend([
            ("id", p.id as i64),
            ("count", count as i64),
            ("durability", dur as i64),
            ("max_durability", dur as i64),
            ("flag", p.flag as i64),
            ("original_flag", p.flag as i64),
            ("serial", new_serial()),
            ("expire", p.expire as i64),
        ]);
    }
    write_slot(&mut tx, kind, &owner, p.tab, p.slot, &vals).await?;
    audit(
        &mut tx,
        &s.actor,
        "container.set",
        &format!("{}:{}:{}/{}", p.kind, label, p.tab, p.slot),
        json!({"item_id":p.id,"count":p.count,"reason":p.reason}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct MoveIn {
    #[serde(rename = "type")]
    kind: String,
    owner: String,
    #[serde(default)]
    tab: i32,
    #[serde(default)]
    to_tab: Option<i32>,
    from: i32,
    to: i32,
}

/// Swap (or move) two slots atomically with every metadata field kept.
async fn container_move(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<MoveIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let kind = Kind::parse(&p.kind)?;
    let to_tab = p.to_tab.unwrap_or(p.tab);
    if !slot_in_range(kind, p.from, p.tab) || !slot_in_range(kind, p.to, to_tab) {
        return Err(ApiError::BadRequest);
    }
    if p.from == p.to && p.tab == to_tab {
        return Ok(Json(json!({"ok":true})));
    }
    let (owner, label) = resolve_owner(&app.pool, kind, &p.owner).await?;
    if owner_online(&app.pool, kind, &owner).await? {
        return Err(ApiError::Conflict);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let a = read_slot(&mut tx, kind, &owner, p.tab, p.from).await?;
    let b = read_slot(&mut tx, kind, &owner, to_tab, p.to).await?;
    write_slot(&mut tx, kind, &owner, p.tab, p.from, &as_refs(&b)).await?;
    write_slot(&mut tx, kind, &owner, to_tab, p.to, &as_refs(&a)).await?;
    audit(
        &mut tx,
        &s.actor,
        "container.move",
        &format!("{}:{}:{}/{}->{}/{}", p.kind, label, p.tab, p.from, to_tab, p.to),
        json!({}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

fn as_refs(v: &[(&'static str, i64)]) -> Vec<(&'static str, i64)> {
    v.to_vec()
}

async fn read_slot(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    kind: Kind,
    owner: &Owner,
    tab: i32,
    slot: i32,
) -> Result<Vec<(&'static str, i64)>, ApiError> {
    let cols = kind.cols();
    let select: Vec<String> = cols.iter().map(|(_, c)| format!("COALESCE({c},0)::bigint")).collect();
    let tab_f = if kind == Kind::Clan { " AND tab_index=$3" } else { "" };
    let ocol = if kind == Kind::Inv { "lower(str_user_id)" } else { kind.owner_col() };
    let sql = format!(
        "SELECT {sel} FROM {t} WHERE {o}=$1 AND {sc}=$2{tab_f} FOR UPDATE",
        sel = select.join(","),
        t = kind.table(),
        o = ocol,
        sc = kind.slot_col(),
    );
    let mut q = sqlx::query(&sql);
    q = match owner {
        Owner::Text(s) if kind == Kind::Inv => q.bind(s.to_lowercase()),
        Owner::Text(s) => q.bind(s.clone()),
        Owner::Id(i) => q.bind(*i),
    };
    q = if kind == Kind::Clan { q.bind(slot) } else { q.bind(slot as i16) };
    if kind == Kind::Clan {
        q = q.bind(tab as i16);
    }
    let row = q.fetch_optional(&mut **tx).await.map_err(internal("slot read"))?;
    Ok(cols
        .iter()
        .enumerate()
        .map(|(i, (k, _))| (*k, row.as_ref().and_then(|r| r.try_get::<i64, _>(i).ok()).unwrap_or(0)))
        .collect())
}

// ── events ───────────────────────────────────────────────────────────────

async fn events(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT jsonb_build_object(
            'schedule', COALESCE((SELECT jsonb_agg(jsonb_build_object(
                'id',m.eventid,'type',m.event_type,'zone',m.zoneid,'name',m.name,'status',m.status,
                'times',jsonb_build_array(jsonb_build_array(m.hour1,m.minute1),jsonb_build_array(m.hour2,m.minute2),
                    jsonb_build_array(m.hour3,m.minute3),jsonb_build_array(m.hour4,m.minute4),jsonb_build_array(m.hour5,m.minute5)),
                'min_level',m.min_level,'max_level',m.max_level,'req_loyalty',m.req_loyalty,'req_money',m.req_money,
                'days',jsonb_build_array(d.sunday,d.monday,d.tuesday,d.wednesday,d.thursday,d.friday,d.saturday)) ORDER BY m.eventid)
                FROM event_schedule_main_list m LEFT JOIN event_schedule_day_list d ON d.eventid=m.eventid),'[]'::jsonb),
            'rooms', COALESCE((SELECT jsonb_agg(v ORDER BY zoneid) FROM event_opt_vroom v),'[]'::jsonb)
        )"#,
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("events"))?;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct EventEdit {
    status: i16,
    times: Vec<[i16; 2]>,
    days: Vec<i16>,
    min_level: i16,
    max_level: i16,
    req_loyalty: i32,
    req_money: i32,
    reason: String,
}

async fn event_edit(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i16>,
    Json(p): Json<EventEdit>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.times.len() != 5
        || p.days.len() != 7
        || !matches!(p.status, 0 | 1)
        || p.reason.trim().chars().count() < 3
        || p.times.iter().any(|[h, m]| !((*h == -1 && *m == -1) || ((0..24).contains(h) && (0..60).contains(m))))
        || p.days.iter().any(|d| !matches!(d, 0 | 1))
        || p.min_level < 0
        || p.max_level < 0
        || p.req_loyalty < 0
        || p.req_money < 0
    {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let t = &p.times;
    let n = sqlx::query("UPDATE event_schedule_main_list SET status=$2,hour1=$3,minute1=$4,hour2=$5,minute2=$6,hour3=$7,minute3=$8,hour4=$9,minute4=$10,hour5=$11,minute5=$12,min_level=$13,max_level=$14,req_loyalty=$15,req_money=$16 WHERE eventid=$1")
        .bind(id).bind(p.status)
        .bind(t[0][0]).bind(t[0][1]).bind(t[1][0]).bind(t[1][1]).bind(t[2][0]).bind(t[2][1])
        .bind(t[3][0]).bind(t[3][1]).bind(t[4][0]).bind(t[4][1])
        .bind(p.min_level).bind(p.max_level).bind(p.req_loyalty).bind(p.req_money)
        .execute(&mut *tx).await.map_err(internal("event write"))?.rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    let d = &p.days;
    sqlx::query("INSERT INTO event_schedule_day_list(eventid,sunday,monday,tuesday,wednesday,thursday,friday,saturday) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(eventid) DO UPDATE SET sunday=EXCLUDED.sunday,monday=EXCLUDED.monday,tuesday=EXCLUDED.tuesday,wednesday=EXCLUDED.wednesday,thursday=EXCLUDED.thursday,friday=EXCLUDED.friday,saturday=EXCLUDED.saturday")
        .bind(id).bind(d[0]).bind(d[1]).bind(d[2]).bind(d[3]).bind(d[4]).bind(d[5]).bind(d[6])
        .execute(&mut *tx).await.map_err(internal("event days"))?;
    audit(&mut tx, &s.actor, "event.schedule", &id.to_string(), json!({"status":p.status,"times":p.times,"days":p.days,"reason":p.reason.trim()}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_command(&app.pool, "reload", json!({"what":"event_schedule"})).await?;
    Ok(Json(json!({"ok":true,"note":"Takvim kaydedildi. Oyun sunucusu çalışıyorsa yeniden yükleme sıraya alındı; kapalıysa açılışta okunur."})))
}

// ── audit and commands ───────────────────────────────────────────────────

async fn audit_log(headers: HeaderMap, State(app): State<App>, Query(q): Query<Q>) -> R {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default();
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(a),'[]'::jsonb) FROM (
            SELECT audit_id id, occurred_at, actor, action, target, details, host(remote_ip) ip
            FROM admin_audit_log
            WHERE $1='' OR actor ILIKE $2 OR action ILIKE $2 OR target ILIKE $2
            ORDER BY audit_id DESC LIMIT $3 OFFSET $4) a"#,
    )
    .bind(&term)
    .bind(format!("%{term}%"))
    .bind(q.limit.unwrap_or(100).clamp(1, 500))
    .bind(q.offset.unwrap_or(0).max(0))
    .fetch_one(&app.pool)
    .await
    .map_err(internal("audit"))?;
    Ok(Json(v))
}

async fn commands(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        "SELECT COALESCE(jsonb_agg(c),'[]'::jsonb) FROM (SELECT id,created_at,command,args,status,result FROM admin_command ORDER BY id DESC LIMIT 40) c",
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("commands"))?;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct CommandIn {
    command: String,
    #[serde(default)]
    args: Value,
}

async fn send_command(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<CommandIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !matches!(p.command.as_str(), "reload" | "announce" | "kick" | "control" | "event") {
        return Err(ApiError::BadRequest);
    }
    let id = queue_command(&app.pool, &p.command, p.args.clone()).await?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "command.queue", &p.command, p.args, peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"id":id,"status":StatusCode::ACCEPTED.as_u16()})))
}

const ASSETS: [(&str, &str, &str); 14] = [
    ("style.css", "text/css; charset=utf-8", include_str!("../assets/style.css")),
    ("core.js", "text/javascript; charset=utf-8", include_str!("../assets/core.js")),
    ("containers.js", "text/javascript; charset=utf-8", include_str!("../assets/containers.js")),
    ("drops.js", "text/javascript; charset=utf-8", include_str!("../assets/drops.js")),
    ("mapedit.js", "text/javascript; charset=utf-8", include_str!("../assets/mapedit.js")),
    ("pages.js", "text/javascript; charset=utf-8", include_str!("../assets/pages.js")),
    ("events.js", "text/javascript; charset=utf-8", include_str!("../assets/events.js")),
    ("pus.js", "text/javascript; charset=utf-8", include_str!("../assets/pus.js")),
    ("rewards.js", "text/javascript; charset=utf-8", include_str!("../assets/rewards.js")),
    ("chaotic.js", "text/javascript; charset=utf-8", include_str!("../assets/chaotic.js")),
    ("bot_loadout.js", "text/javascript; charset=utf-8", include_str!("../assets/bot_loadout.js")),
    ("field_bots.js", "text/javascript; charset=utf-8", include_str!("../assets/field_bots.js")),
    ("info_panel.js", "text/javascript; charset=utf-8", include_str!("../assets/info_panel.js")),
    ("event_bots.js", "text/javascript; charset=utf-8", include_str!("../assets/event_bots.js")),
];

async fn static_file(Path(file): Path<String>) -> Response {
    match ASSETS.iter().find(|(n, _, _)| *n == file) {
        Some((_, ct, body)) => ([(header::CONTENT_TYPE, *ct), (header::CACHE_CONTROL, "no-store")], *body).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
struct IconRange {
    from: i32,
    to: i32,
    icon_id: i32,
    reason: String,
}

/// Point a range of item numbers at one icon (for items whose DB icon id is wrong).
async fn icon_range(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<IconRange>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.from <= 0 || p.to < p.from || p.to - p.from > 100_000_000 || p.icon_id < 10_000_000 || p.reason.trim().chars().count() < 3 {
        return Err(ApiError::BadRequest);
    }
    let found = icon_candidates(&p.icon_id.to_string())
        .iter()
        .any(|n| app.icon_dir.join(n).is_file());
    if !found {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let n = sqlx::query("UPDATE item SET item_icon_id1=$3, item_icon_id2=$3 WHERE num BETWEEN $1 AND $2")
        .bind(p.from)
        .bind(p.to)
        .bind(p.icon_id)
        .execute(&mut *tx)
        .await
        .map_err(internal("icon range"))?
        .rows_affected();
    audit(&mut tx, &s.actor, "item.icon_range", &format!("{}-{}", p.from, p.to), json!({"icon_id":p.icon_id,"rows":n,"reason":p.reason.trim()}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"rows":n})))
}

/// Which file would be served for an icon id, and does it exist on this machine?
async fn icon_debug(headers: HeaderMap, State(app): State<App>, Path(id): Path<String>) -> R {
    auth(&headers, false)?;
    let files: Vec<Value> = icon_candidates(&id)
        .into_iter()
        .map(|n| json!({"file": n, "exists": app.icon_dir.join(&n).is_file()}))
        .collect();
    let count = std::fs::read_dir(&app.icon_dir).map(|d| d.count()).unwrap_or(0);
    Ok(Json(json!({"dir": app.icon_dir.display().to_string(), "dir_exists": app.icon_dir.is_dir(), "files_in_dir": count, "candidates": files})))
}
