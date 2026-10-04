//! Authenticated administrative control plane.
//! The bootstrap credential is supplied as a PBKDF2 hash in process environment;
//! additional administrators are stored as PBKDF2 hashes in PostgreSQL.

use std::{
    net::{IpAddr, SocketAddr},
    path::{Path as FsPath, PathBuf},
    sync::{Arc, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use dashmap::DashMap;
use hmac::{Hmac, Mac};
use ko_db::DbPool;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use sqlx::Row;
use subtle::ConstantTimeEq;
use tracing::{info, warn};

use crate::{
    systems::{
        event_room::{TempleEventType, VroomOpt},
        event_system::{self, EventOpenParams},
    },
    world::WorldState,
};

const HTML: &str = include_str!("../assets/admin/index.html");
const SESSION_TTL: Duration = Duration::from_secs(30 * 60);
const COOKIE: &str = "jstko_admin";
const ADMIN_PATH: &str = "/jstkoadminpanel";
const CSRF_HEADER: &str = "x-admin-csrf";
const MAX_LOGIN_ATTEMPTS: u32 = 8;
const LOGIN_WINDOW: Duration = Duration::from_secs(10 * 60);

#[derive(Clone)]
struct AdminSession {
    csrf: String,
    actor: String,
    expires: Instant,
}
struct LoginBucket {
    started: Instant,
    attempts: u32,
}
static SESSIONS: OnceLock<DashMap<String, AdminSession>> = OnceLock::new();
static LOGIN_BUCKETS: OnceLock<DashMap<IpAddr, LoginBucket>> = OnceLock::new();
fn sessions() -> &'static DashMap<String, AdminSession> {
    SESSIONS.get_or_init(DashMap::new)
}
fn buckets() -> &'static DashMap<IpAddr, LoginBucket> {
    LOGIN_BUCKETS.get_or_init(DashMap::new)
}

#[derive(Clone)]
struct App {
    world: Arc<WorldState>,
    pool: DbPool,
}

pub fn start(world: Arc<WorldState>, pool: DbPool, bind: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let app_state = App { world, pool };
        let router = Router::new()
            .route("/jstkoadminpanel/", get(index))
            .route("/jstkoadminpanel/api/login", post(login))
            .route("/jstkoadminpanel/api/logout", post(logout))
            .route("/jstkoadminpanel/api/me", get(me))
            .route(
                "/jstkoadminpanel/api/admin-users",
                get(admin_users).post(create_admin_user),
            )
            .route(
                "/jstkoadminpanel/api/admin-users/{username}",
                delete(delete_admin_user),
            )
            .route("/jstkoadminpanel/api/overview", get(overview))
            .route("/jstkoadminpanel/api/characters", get(characters))
            .route("/jstkoadminpanel/api/characters/{name}", get(character))
            .route(
                "/jstkoadminpanel/api/characters/{name}",
                post(edit_character),
            )
            .route(
                "/jstkoadminpanel/api/characters/{name}/force-logout",
                post(force_logout_character),
            )
            .route("/jstkoadminpanel/api/items", get(items))
            .route(
                "/jstkoadminpanel/api/quest-studio/npcs",
                get(quest_studio_npcs),
            )
            .route(
                "/jstkoadminpanel/api/quest-studio/npc/{sid}",
                get(quest_studio_npc),
            )
            .route(
                "/jstkoadminpanel/api/quest-studio/helper/{index}",
                post(quest_studio_save_helper),
            )
            .route(
                "/jstkoadminpanel/api/quest-studio/helper",
                post(quest_studio_create_helper),
            )
            .route(
                "/jstkoadminpanel/api/quest-studio/lua/{filename}",
                get(quest_studio_lua).post(quest_studio_save_lua),
            )
            .route(
                "/jstkoadminpanel/api/quest-studio/tbl/{table}",
                get(quest_studio_tbl_search),
            )
            .route(
                "/jstkoadminpanel/api/quest-studio/tbl/{table}/{row_id}",
                post(quest_studio_tbl_save),
            )
            .route(
                "/jstkoadminpanel/api/quest-studio/reload",
                post(quest_studio_reload),
            )
            .route("/jstkoadminpanel/api/letters/item", post(send_item_letter))
            .route(
                "/jstkoadminpanel/api/characters/{name}/inventory",
                get(inventory),
            )
            .route(
                "/jstkoadminpanel/api/characters/{name}/inventory",
                post(set_inventory),
            )
            .route(
                "/jstkoadminpanel/api/characters/{name}/warehouse",
                get(warehouse),
            )
            .route(
                "/jstkoadminpanel/api/characters/{name}/warehouse",
                post(set_warehouse),
            )
            .route(
                "/jstkoadminpanel/api/characters/{name}/warehouse/move",
                post(move_warehouse_slots),
            )
            .route(
                "/jstkoadminpanel/api/characters/{name}/inventory/move",
                post(move_inventory_slots),
            )
            .route("/jstkoadminpanel/api/drops", get(drop_tables))
            .route(
                "/jstkoadminpanel/api/drops/bulk-adjust",
                post(adjust_all_monster_drops),
            )
            .route("/jstkoadminpanel/api/drop-groups", get(drop_groups))
            .route("/jstkoadminpanel/api/drops/{sid}", post(edit_drop))
            .route("/jstkoadminpanel/api/group-items", get(group_items))
            .route(
                "/jstkoadminpanel/api/group-items/{group_num}",
                post(save_group_items).delete(delete_group_items),
            )
            .route(
                "/jstkoadminpanel/api/reload-editable-tables",
                post(reload_editable_tables),
            )
            .route("/jstkoadminpanel/api/start-positions", get(start_positions))
            .route(
                "/jstkoadminpanel/api/start-positions/{zone}",
                post(edit_start_position),
            )
            .route(
                "/jstkoadminpanel/api/starting-character",
                get(starting_characters),
            )
            .route(
                "/jstkoadminpanel/api/starting-character/{class}/{job}",
                post(edit_starting_character),
            )
            .route(
                "/jstkoadminpanel/api/starting-character/{class}/{job}/items",
                get(starting_items),
            )
            .route(
                "/jstkoadminpanel/api/starting-character/{class}/{job}/items",
                post(edit_starting_item),
            )
            .route("/jstkoadminpanel/api/control", post(control))
            .with_state(app_state);
        match tokio::net::TcpListener::bind(&bind).await {
            Ok(listener) => {
                info!(address = %bind, "Admin control panel listening");
                if let Err(e) = axum::serve(
                    listener,
                    router.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                {
                    warn!(%e, "Admin panel service stopped");
                }
            }
            Err(e) => warn!(address = %bind, %e, "Admin panel failed to bind"),
        }
    })
}

async fn index() -> Response {
    ([(header::CACHE_CONTROL, "no-store"), (header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'")], Html(HTML)).into_response()
}

#[derive(Deserialize)]
struct LoginInput {
    username: String,
    password: String,
}
#[derive(Serialize)]
struct LoginOutput {
    csrf: String,
    actor: String,
}
async fn login(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(app): State<App>,
    Json(input): Json<LoginInput>,
) -> Result<Response, ApiError> {
    check_origin(&headers)?;
    let now = Instant::now();
    let mut bucket = buckets().entry(peer.ip()).or_insert(LoginBucket {
        started: now,
        attempts: 0,
    });
    if now.duration_since(bucket.started) > LOGIN_WINDOW {
        bucket.started = now;
        bucket.attempts = 0;
    }
    if bucket.attempts >= MAX_LOGIN_ATTEMPTS {
        return Err(ApiError::TooManyRequests);
    }
    bucket.attempts += 1;
    let bootstrap_user = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "volkan".to_string());
    let bootstrap_hash = std::env::var("ADMIN_PASSWORD_HASH").unwrap_or_default();
    let stored_admin = sqlx::query("SELECT username,password_hash FROM admin_users WHERE lower(username)=lower($1) AND active=true")
        .bind(input.username.trim())
        .fetch_optional(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    let (expected_user, password_hash) = if let Some(row) = stored_admin {
        (
            row.try_get::<String, _>("username")
                .map_err(|_| ApiError::Internal)?,
            row.try_get::<String, _>("password_hash")
                .map_err(|_| ApiError::Internal)?,
        )
    } else if !bootstrap_hash.is_empty()
        && input.username.trim().eq_ignore_ascii_case(&bootstrap_user)
    {
        (bootstrap_user, bootstrap_hash)
    } else {
        (String::new(), String::new())
    };
    let user_ok =
        !expected_user.is_empty() && input.username.trim().eq_ignore_ascii_case(&expected_user);
    let pass_ok = verify_password(&input.password, &password_hash);
    if !(user_ok & pass_ok) {
        return Err(ApiError::Unauthorized);
    }
    drop(bucket);
    buckets().remove(&peer.ip());
    let token = random_token(32);
    let csrf = random_token(24);
    sessions().retain(|_, value| value.expires > now);
    buckets().retain(|_, value| now.duration_since(value.started) <= LOGIN_WINDOW);
    sessions().insert(
        token.clone(),
        AdminSession {
            csrf: csrf.clone(),
            actor: expected_user.clone(),
            expires: now + SESSION_TTL,
        },
    );
    let cookie = format!(
        "{COOKIE}={token}; Path={ADMIN_PATH}; HttpOnly; SameSite=Strict; Max-Age=1800{}",
        if is_secure_request(&headers) {
            "; Secure"
        } else {
            ""
        }
    );
    let mut response = Json(LoginOutput {
        csrf,
        actor: expected_user,
    })
    .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|_| ApiError::Internal)?,
    );
    Ok(response)
}

async fn logout(headers: HeaderMap) -> Result<Response, ApiError> {
    let (_, session) = auth(&headers, true)?;
    let token = cookie_token(&headers).ok_or(ApiError::Unauthorized)?;
    if !csrf_valid(&headers, &session) {
        return Err(ApiError::Forbidden);
    }
    sessions().remove(&token);
    let mut out = StatusCode::NO_CONTENT.into_response();
    out.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "jstko_admin=; Path=/jstkoadminpanel; HttpOnly; SameSite=Strict; Max-Age=0",
        ),
    );
    Ok(out)
}
async fn me(headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, false)?;
    Ok(Json(json!({"csrf":s.csrf,"actor":s.actor})))
}

#[derive(Deserialize)]
struct CreateAdminInput {
    username: String,
    password: String,
}

async fn admin_users(headers: HeaderMap, State(app): State<App>) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let mut rows = sqlx::query(
        "SELECT username,created_at FROM admin_users WHERE active=true ORDER BY lower(username)",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(|_| ApiError::Internal)?
    .into_iter()
    .map(|row| {
        json!({
            "username": row.try_get::<String, _>("username").unwrap_or_default(),
            "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            "bootstrap": false
        })
    })
    .collect::<Vec<_>>();
    let bootstrap = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "volkan".to_string());
    if !std::env::var("ADMIN_PASSWORD_HASH")
        .unwrap_or_default()
        .is_empty()
        && !rows.iter().any(|row| {
            row["username"]
                .as_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(&bootstrap))
        })
    {
        rows.insert(
            0,
            json!({"username": bootstrap, "created_at": null, "bootstrap": true}),
        );
    }
    Ok(Json(Value::Array(rows)))
}

async fn create_admin_user(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(app): State<App>,
    Json(input): Json<CreateAdminInput>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    let username = input.username.trim();
    let bootstrap = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "volkan".to_string());
    if username.len() < 3
        || username.len() > 32
        || !username
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        || username.eq_ignore_ascii_case(&bootstrap)
        || input.password.chars().count() < 12
        || input.password.chars().count() > 256
    {
        return Err(ApiError::BadRequest);
    }
    let hash = hash_password(&input.password);
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let inserted = sqlx::query("INSERT INTO admin_users(username,password_hash,created_by) VALUES($1,$2,$3) ON CONFLICT (lower(username)) DO NOTHING RETURNING username")
        .bind(username)
        .bind(hash)
        .bind(&session.actor)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;
    let Some(row) = inserted else {
        return Err(ApiError::Conflict);
    };
    let added = row
        .try_get::<String, _>("username")
        .map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &session.actor,
        "admin_user.create",
        &added,
        json!({}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"username": added})))
}

async fn delete_admin_user(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(username): Path<String>,
    headers: HeaderMap,
    State(app): State<App>,
) -> Result<StatusCode, ApiError> {
    let (_, session) = auth(&headers, true)?;
    let bootstrap = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "volkan".to_string());
    if username.eq_ignore_ascii_case(&bootstrap) || username.eq_ignore_ascii_case(&session.actor) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let deleted =
        sqlx::query("DELETE FROM admin_users WHERE lower(username)=lower($1) RETURNING username")
            .bind(&username)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal)?
            .ok_or(ApiError::NotFound)?;
    let removed = deleted
        .try_get::<String, _>("username")
        .map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &session.actor,
        "admin_user.delete",
        &removed,
        json!({}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            part.trim()
                .strip_prefix(&format!("{COOKIE}="))
                .map(str::to_owned)
        })
}
fn auth(headers: &HeaderMap, write: bool) -> Result<(String, AdminSession), ApiError> {
    check_origin(headers)?;
    let token = cookie_token(headers).ok_or(ApiError::Unauthorized)?;
    let session = sessions()
        .get(&token)
        .map(|v| v.clone())
        .filter(|s| s.expires > Instant::now())
        .ok_or(ApiError::Unauthorized)?;
    if write && !csrf_valid(headers, &session) {
        return Err(ApiError::Forbidden);
    }
    Ok((token, session))
}
fn csrf_valid(headers: &HeaderMap, session: &AdminSession) -> bool {
    headers
        .get(CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.as_bytes().ct_eq(session.csrf.as_bytes()).unwrap_u8() == 1)
        .unwrap_or(false)
}
fn check_origin(headers: &HeaderMap) -> Result<(), ApiError> {
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|x| x.to_str().ok()) {
        let host = headers
            .get(header::HOST)
            .and_then(|x| x.to_str().ok())
            .unwrap_or("");
        if origin != format!("http://{host}") && origin != format!("https://{host}") {
            return Err(ApiError::Forbidden);
        }
    }
    Ok(())
}
fn is_secure_request(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        == Some("https")
}
fn random_token(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Encoded format: pbkdf2_sha256$iterations$salt_hex$derived_key_hex.
fn verify_password(password: &str, encoded: &str) -> bool {
    let parts: Vec<_> = encoded.split('$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2_sha256" {
        return false;
    }
    let Ok(iterations) = parts[1].parse::<u32>() else {
        return false;
    };
    if !(100_000..=2_000_000).contains(&iterations) {
        return false;
    }
    let (Some(salt), Some(expected)) = (decode_hex(parts[2]), decode_hex(parts[3])) else {
        return false;
    };
    let actual = pbkdf2(password.as_bytes(), &salt, iterations);
    actual.len() == expected.len() && actual.ct_eq(&expected).unwrap_u8() == 1
}
fn hash_password(password: &str) -> String {
    const ROUNDS: u32 = 210_000;
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let derived = pbkdf2(password.as_bytes(), &salt, ROUNDS);
    format!(
        "pbkdf2_sha256${ROUNDS}${}${}",
        encode_hex(&salt),
        encode_hex(&derived)
    )
}
fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|c| {
            let h = (c[0] as char).to_digit(16)?;
            let l = (c[1] as char).to_digit(16)?;
            Some(((h << 4) | l) as u8)
        })
        .collect()
}
fn pbkdf2(password: &[u8], salt: &[u8], rounds: u32) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(password).expect("HMAC accepts any key");
    mac.update(salt);
    mac.update(&1u32.to_be_bytes());
    let mut u = mac.finalize().into_bytes().to_vec();
    let mut out = u.clone();
    for _ in 1..rounds {
        let mut m = Hmac::<Sha256>::new_from_slice(password).expect("HMAC accepts any key");
        m.update(&u);
        u = m.finalize().into_bytes().to_vec();
        for (a, b) in out.iter_mut().zip(&u) {
            *a ^= b;
        }
    }
    out
}

async fn overview(headers: HeaderMap, State(app): State<App>) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let counts = sqlx::query("SELECT (SELECT count(*) FROM userdata) AS characters, (SELECT count(*) FROM currentuser) AS online, (SELECT count(*) FROM item) AS items, (SELECT count(*) FROM npc_template WHERE is_monster) AS monsters")
        .fetch_one(&app.pool).await.map_err(|_| ApiError::Internal)?;
    let war = app.world.get_battle_state();
    let temple = app.world.event_room_manager.temple_event.read();
    let active_event = temple.active_event;
    let event_signing = temple.allow_join;
    drop(temple);
    Ok(Json(
        json!({"characters": counts.try_get::<i64,_>("characters").unwrap_or(0), "online": counts.try_get::<i64,_>("online").unwrap_or(0), "items": counts.try_get::<i64,_>("items").unwrap_or(0), "monsters": counts.try_get::<i64,_>("monsters").unwrap_or(0), "war_open": war.is_war_open(), "war_zone": war.battle_zone_id(), "event": active_event, "event_signing": event_signing}),
    ))
}

#[derive(Deserialize)]
struct Search {
    q: Option<String>,
    limit: Option<i64>,
}
async fn characters(
    headers: HeaderMap,
    State(app): State<App>,
    Query(q): Query<Search>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    if term.is_empty() {
        return Ok(Json(json!([])));
    }
    let rows = sqlx::query("SELECT u.str_user_id, u.nation, u.level, u.rebirth_level, u.loyalty, u.gold, u.zone, EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)) AS online FROM userdata u WHERE u.str_user_id ILIKE $1 OR u.str_user_id = $2 ORDER BY u.str_user_id LIMIT $3")
        .bind(format!("%{}%", term)).bind(&term).bind(q.limit.unwrap_or(50).clamp(1,100)).fetch_all(&app.pool).await.map_err(|_| ApiError::Internal)?;
    Ok(Json(Value::Array(rows.into_iter().map(|r| json!({"name":r.try_get::<String,_>("str_user_id").unwrap_or_default(),"nation":r.try_get::<i16,_>("nation").unwrap_or(0),"level":r.try_get::<i16,_>("level").unwrap_or(0),"rebirth":r.try_get::<i16,_>("rebirth_level").unwrap_or(0),"loyalty":r.try_get::<i32,_>("loyalty").unwrap_or(0),"gold":r.try_get::<i32,_>("gold").unwrap_or(0),"zone":r.try_get::<i16,_>("zone").unwrap_or(0),"online":r.try_get::<bool,_>("online").unwrap_or(false)})).collect())))
}
async fn character(
    headers: HeaderMap,
    State(app): State<App>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let row = sqlx::query("SELECT u.*, EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)) online FROM userdata u WHERE lower(str_user_id)=lower($1)").bind(name).fetch_optional(&app.pool).await.map_err(|_|ApiError::Internal)?.ok_or(ApiError::NotFound)?;
    Ok(Json(
        json!({"name":row.try_get::<String,_>("str_user_id").unwrap_or_default(),"nation":row.try_get::<i16,_>("nation").unwrap_or_default(),"class":row.try_get::<i16,_>("class").unwrap_or_default(),"authority":row.try_get::<i16,_>("authority").unwrap_or(1),"race":row.try_get::<i16,_>("race").unwrap_or_default(),"level":row.try_get::<i16,_>("level").unwrap_or_default(),"rebirth":row.try_get::<i16,_>("rebirth_level").unwrap_or_default(),"exp":row.try_get::<i64,_>("exp").unwrap_or_default(),"loyalty":row.try_get::<i32,_>("loyalty").unwrap_or_default(),"hp":row.try_get::<i16,_>("hp").unwrap_or_default(),"mp":row.try_get::<i16,_>("mp").unwrap_or_default(),"gold":row.try_get::<i32,_>("gold").unwrap_or_default(),"zone":row.try_get::<i16,_>("zone").unwrap_or_default(),"px":row.try_get::<i32,_>("px").unwrap_or_default(),"pz":row.try_get::<i32,_>("pz").unwrap_or_default(),"online":row.try_get::<bool,_>("online").unwrap_or(false)}),
    ))
}
#[derive(Deserialize)]
struct CharacterEdit {
    nation: Option<i16>,
    class: Option<i16>,
    authority: Option<i16>,
    race: Option<i16>,
    level: Option<i16>,
    rebirth: Option<i16>,
    exp: Option<i64>,
    loyalty: Option<i32>,
    gold: Option<i32>,
    hp: Option<i16>,
    mp: Option<i16>,
    zone: Option<i16>,
    px: Option<i32>,
    pz: Option<i32>,
}
async fn edit_character(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
    Json(p): Json<CharacterEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, true)?;
    if p.nation.is_some_and(|v| !matches!(v, 1 | 2))
        || p.authority.is_some_and(|v| !matches!(v, 0 | 1 | 2))
        || p.class
            .is_some_and(|v| !matches!(v % 100, 1..=16) || !(1..=6).contains(&(v / 100)))
    {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    if sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_char_id)=lower($1))",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?
    {
        return Err(ApiError::Conflict);
    }
    let r=sqlx::query("UPDATE userdata SET nation=COALESCE($2,nation), class=COALESCE($3,class), authority=COALESCE($4,authority), race=COALESCE($5,race), level=COALESCE($6,level), rebirth_level=COALESCE($7,rebirth_level), exp=COALESCE($8,exp), loyalty=COALESCE($9,loyalty), gold=COALESCE($10,gold), hp=COALESCE($11,hp), mp=COALESCE($12,mp), zone=COALESCE($13,zone), px=COALESCE($14,px), pz=COALESCE($15,pz), dt_update_time=NOW() WHERE lower(str_user_id)=lower($1) RETURNING str_user_id")
        .bind(&name).bind(p.nation).bind(p.class).bind(p.authority).bind(p.race.map(|v|v.clamp(0,255))).bind(p.level.map(|v|v.clamp(1,83))).bind(p.rebirth.map(|v|v.clamp(0,255))).bind(p.exp.map(|v|v.max(0))).bind(p.loyalty.map(|v|v.max(0))).bind(p.gold.map(|v|v.max(0))).bind(p.hp.map(|v|v.clamp(0,i16::MAX))).bind(p.mp.map(|v|v.clamp(0,i16::MAX))).bind(p.zone).bind(p.px).bind(p.pz).fetch_optional(&mut *tx).await.map_err(|_|ApiError::Internal)?.ok_or(ApiError::NotFound)?;
    audit(&mut tx,&s.actor,"character.edit",&name,json!({"fields":[p.nation.is_some(),p.class.is_some(),p.authority.is_some(),p.race.is_some(),p.level.is_some(),p.rebirth.is_some(),p.exp.is_some(),p.loyalty.is_some(),p.gold.is_some(),p.hp.is_some(),p.mp.is_some(),p.zone.is_some(),p.px.is_some(),p.pz.is_some()]}),peer.ip()).await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(
        json!({"ok":true,"name":r.try_get::<String,_>("str_user_id").unwrap_or_default()}),
    ))
}

async fn force_logout_character(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let (_, admin) = auth(&headers, true)?;
    let canonical = sqlx::query_scalar::<_, String>(
        "SELECT str_user_id FROM userdata WHERE lower(str_user_id)=lower($1)",
    )
    .bind(&name)
    .fetch_optional(&app.pool)
    .await
    .map_err(|_| ApiError::Internal)?
    .ok_or(ApiError::NotFound)?;

    let session_id = app.world.find_session_by_name(&canonical);
    if let Some(id) = session_id {
        app.world.kick_session_for_duplicate(id).await;
    }

    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    sqlx::query("DELETE FROM currentuser WHERE lower(str_char_id)=lower($1)")
        .bind(&canonical)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &admin.actor,
        "character.force_logout",
        &canonical,
        json!({"live_session": session_id.is_some()}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok": true, "name": canonical, "online": false})))
}

async fn items(
    headers: HeaderMap,
    State(app): State<App>,
    Query(q): Query<Search>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    if term.is_empty() {
        return Ok(Json(json!([])));
    }
    let rows=sqlx::query("SELECT num,str_name,description,item_icon_id1,countable,weight,duration FROM item WHERE CAST(num AS TEXT)=$1 OR str_name ILIKE $2 OR description ILIKE $2 ORDER BY CASE WHEN CAST(num AS TEXT)=$1 THEN 0 ELSE 1 END,str_name LIMIT $3")
        .bind(&term).bind(format!("%{}%",term)).bind(q.limit.unwrap_or(60).clamp(1,100)).fetch_all(&app.pool).await.map_err(|_|ApiError::Internal)?;
    Ok(Json(Value::Array(rows.into_iter().map(|r|json!({"id":r.try_get::<i32,_>("num").unwrap_or(0),"name":r.try_get::<Option<String>,_>("str_name").ok().flatten().unwrap_or_default(),"description":r.try_get::<Option<String>,_>("description").ok().flatten().unwrap_or_default(),"icon":r.try_get::<Option<i32>,_>("item_icon_id1").ok().flatten(),"countable":r.try_get::<Option<i32>,_>("countable").ok().flatten().unwrap_or(0)})).collect())))
}

// Quest Studio ---------------------------------------------------------------

async fn quest_studio_npcs(
    headers: HeaderMap,
    State(app): State<App>,
    Query(q): Query<Search>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    if term.is_empty() {
        return Ok(Json(json!([])));
    }
    let rows = sqlx::query(
        r#"SELECT n.s_sid AS npc_id, COALESCE(n.str_name,'') AS npc_name,
         COUNT(q.n_index)::int AS quest_count, MIN(q.str_lua_filename) AS lua_filename
         FROM npc_template n LEFT JOIN quest_helper q ON q.s_npc_id=n.s_sid
         WHERE n.is_monster=false AND (CAST(n.s_sid AS TEXT)=$1 OR n.str_name ILIKE $2
            OR q.str_lua_filename ILIKE $2)
         GROUP BY n.s_sid,n.str_name ORDER BY n.str_name LIMIT $3"#,
    )
    .bind(&term)
    .bind(format!("%{term}%"))
    .bind(q.limit.unwrap_or(50).clamp(1, 100))
    .fetch_all(&app.pool)
    .await
    .map_err(|e| {
        warn!(error=%e, "quest studio NPC search failed");
        ApiError::Internal
    })?;
    Ok(Json(Value::Array(
        rows.into_iter()
            .map(|r| json!({
                "npc_id": r.try_get::<i16,_>("npc_id").unwrap_or(0),
                "name": r.try_get::<String,_>("npc_name").unwrap_or_default(),
                "quest_count": r.try_get::<i32,_>("quest_count").unwrap_or(0),
                "lua": r.try_get::<Option<String>,_>("lua_filename").ok().flatten().unwrap_or_default()
            }))
            .collect(),
    )))
}

async fn quest_studio_npc(
    headers: HeaderMap,
    State(app): State<App>,
    Path(sid): Path<i16>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    if sid <= 0 {
        return Err(ApiError::BadRequest);
    }
    let rows = sqlx::query(
        r#"SELECT q.*, COALESCE(n.str_name,'') AS npc_name,
         COALESCE(t_main.str_talk,'') AS main_text,
         COALESCE(t_event.str_talk,'') AS event_text,
         COALESCE(m.str_menu,'') AS menu_text
         FROM quest_helper q LEFT JOIN npc_template n
           ON n.s_sid=q.s_npc_id AND n.is_monster=false
         LEFT JOIN quest_talk t_main ON t_main.i_num=q.s_npc_main
         LEFT JOIN quest_talk t_event ON t_event.i_num=q.n_event_talk_index
         LEFT JOIN quest_menu m ON m.i_num=q.s_quest_menu
         WHERE q.s_npc_id=$1 ORDER BY q.n_index LIMIT 300"#,
    )
    .bind(sid)
    .fetch_all(&app.pool)
    .await
    .map_err(|e| {
        warn!(npc_id=sid,error=%e,"quest studio quest lookup failed");
        ApiError::Internal
    })?;
    if rows.is_empty() {
        let npc_name = sqlx::query_scalar::<_, String>(
            "SELECT COALESCE(str_name,'') FROM npc_template WHERE s_sid=$1 AND is_monster=false",
        )
        .bind(sid)
        .fetch_optional(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?
        .ok_or(ApiError::NotFound)?;
        return Ok(Json(json!({"npc_id":sid,"npc_name":npc_name,"quests":[]})));
    }
    Ok(Json(
        json!({"npc_id":sid,"npc_name":rows[0].try_get::<String,_>("npc_name").unwrap_or_default(),"quests":rows.into_iter().map(|r|json!({
        "index":r.try_get::<i32,_>("n_index").unwrap_or(0),
        "level":r.try_get::<i16,_>("b_level").unwrap_or(0),
        "exp":r.try_get::<i32,_>("n_exp").unwrap_or(0),
        "class":r.try_get::<i16,_>("b_class").unwrap_or(0),
        "nation":r.try_get::<i16,_>("b_nation").unwrap_or(0),
        "quest_type":r.try_get::<i16,_>("b_quest_type").unwrap_or(0),
        "zone":r.try_get::<i16,_>("b_zone").unwrap_or(0),
        "npc_id":r.try_get::<i16,_>("s_npc_id").unwrap_or(0),
        "event_data":r.try_get::<i16,_>("s_event_data_index").unwrap_or(0),
        "event_status":r.try_get::<i16,_>("b_event_status").unwrap_or(0),
        "trigger":r.try_get::<i32,_>("n_event_trigger_index").unwrap_or(0),
        "complete":r.try_get::<i32,_>("n_event_complete_index").unwrap_or(0),
        "exchange":r.try_get::<i32,_>("n_exchange_index").unwrap_or(0),
        "event_talk":r.try_get::<i32,_>("n_event_talk_index").unwrap_or(0),
        "lua":r.try_get::<String,_>("str_lua_filename").unwrap_or_default(),
        "menu_id":r.try_get::<i32,_>("s_quest_menu").unwrap_or(0),
        "main_talk_id":r.try_get::<i32,_>("s_npc_main").unwrap_or(0),
        "solo":r.try_get::<i16,_>("s_quest_solo").unwrap_or(0),
        "main_text":r.try_get::<String,_>("main_text").unwrap_or_default(),
        "event_text":r.try_get::<String,_>("event_text").unwrap_or_default(),
        "menu_text":r.try_get::<String,_>("menu_text").unwrap_or_default()
    })).collect::<Vec<_>>()}),
    ))
}

#[derive(Deserialize)]
struct QuestHelperEdit {
    level: Option<i16>,
    exp: Option<i32>,
    class: Option<i16>,
    nation: Option<i16>,
    quest_type: Option<i16>,
    zone: Option<i16>,
    event_data: Option<i16>,
    event_status: Option<i16>,
    trigger: Option<i32>,
    complete: Option<i32>,
    exchange: Option<i32>,
    event_talk: Option<i32>,
    lua: Option<String>,
    menu_id: Option<i32>,
    main_talk_id: Option<i32>,
    solo: Option<i16>,
}

#[derive(Deserialize)]
struct QuestHelperCreate {
    #[serde(default)]
    index: i32,
    npc_id: i16,
    #[serde(default)]
    level: i16,
    #[serde(default)]
    exp: i32,
    #[serde(default = "default_quest_class")]
    class: i16,
    #[serde(default = "default_quest_nation")]
    nation: i16,
    #[serde(default)]
    quest_type: i16,
    #[serde(default)]
    zone: i16,
    #[serde(default)]
    event_data: i16,
    #[serde(default)]
    event_status: i16,
    #[serde(default)]
    trigger: i32,
    #[serde(default)]
    complete: i32,
    #[serde(default)]
    exchange: i32,
    #[serde(default)]
    event_talk: i32,
    lua: String,
    #[serde(default)]
    menu_id: i32,
    #[serde(default)]
    main_talk_id: i32,
    #[serde(default)]
    solo: i16,
}
fn default_quest_class() -> i16 {
    5
}
fn default_quest_nation() -> i16 {
    3
}

async fn quest_studio_create_helper(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<QuestHelperCreate>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    if p.index < 0 || p.npc_id <= 0 || !valid_lua_filename(&p.lua) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let index = if p.index == 0 {
        sqlx::query_scalar::<_, i32>("SELECT COALESCE(MAX(n_index),0)+1 FROM quest_helper")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal)?
    } else {
        p.index
    };
    sqlx::query("INSERT INTO quest_helper (n_index,b_message_type,b_level,n_exp,b_class,b_nation,b_quest_type,b_zone,s_npc_id,s_event_data_index,b_event_status,n_event_trigger_index,n_event_complete_index,n_exchange_index,n_event_talk_index,str_lua_filename,s_quest_menu,s_npc_main,s_quest_solo) VALUES ($1,2,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)")
        .bind(index).bind(p.level).bind(p.exp).bind(p.class).bind(p.nation).bind(p.quest_type).bind(p.zone)
        .bind(p.npc_id).bind(p.event_data).bind(p.event_status).bind(p.trigger).bind(p.complete).bind(p.exchange)
        .bind(p.event_talk).bind(&p.lua).bind(p.menu_id).bind(p.main_talk_id).bind(p.solo)
        .execute(&mut *tx).await.map_err(|e|{warn!(index,npc=p.npc_id,error=%e,"quest helper create failed");ApiError::Conflict})?;
    audit(
        &mut tx,
        &session.actor,
        "quest_helper.create",
        &index.to_string(),
        json!({"npc_id":p.npc_id,"lua":p.lua,"zone":p.zone,"level":p.level}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    app.world
        .reload_quest_tables(&app.pool)
        .await
        .map_err(|e| {
            warn!(error=%e,"quest cache reload after create failed");
            ApiError::Internal
        })?;
    Ok(Json(json!({"ok":true,"index":index,"reloaded":true})))
}

async fn quest_studio_save_helper(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(index): Path<i32>,
    Json(p): Json<QuestHelperEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    if index <= 0 || p.lua.as_ref().is_some_and(|s| !valid_lua_filename(s)) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let result = sqlx::query(
        r#"UPDATE quest_helper SET b_level=COALESCE($2,b_level), n_exp=COALESCE($3,n_exp),
         b_class=COALESCE($4,b_class), b_nation=COALESCE($5,b_nation),
         b_quest_type=COALESCE($6,b_quest_type), b_zone=COALESCE($7,b_zone),
         s_event_data_index=COALESCE($8,s_event_data_index), b_event_status=COALESCE($9,b_event_status),
         n_event_trigger_index=COALESCE($10,n_event_trigger_index),
         n_event_complete_index=COALESCE($11,n_event_complete_index),
         n_exchange_index=COALESCE($12,n_exchange_index), n_event_talk_index=COALESCE($13,n_event_talk_index),
         str_lua_filename=COALESCE($14,str_lua_filename), s_quest_menu=COALESCE($15,s_quest_menu),
         s_npc_main=COALESCE($16,s_npc_main), s_quest_solo=COALESCE($17,s_quest_solo)
         WHERE n_index=$1"#,
    )
    .bind(index)
    .bind(p.level)
    .bind(p.exp)
    .bind(p.class)
    .bind(p.nation)
    .bind(p.quest_type)
    .bind(p.zone)
    .bind(p.event_data)
    .bind(p.event_status)
    .bind(p.trigger)
    .bind(p.complete)
    .bind(p.exchange)
    .bind(p.event_talk)
    .bind(p.lua.as_deref())
    .bind(p.menu_id)
    .bind(p.main_talk_id)
    .bind(p.solo)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        warn!(index,error=%e,"quest helper save failed");
        ApiError::Internal
    })?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    audit(&mut tx, &session.actor, "quest_helper.edit", &index.to_string(), json!({
        "level":p.level,"exp":p.exp,"class":p.class,"nation":p.nation,"quest_type":p.quest_type,
        "zone":p.zone,"event_data":p.event_data,"event_status":p.event_status,"trigger":p.trigger,
        "complete":p.complete,"exchange":p.exchange,"event_talk":p.event_talk,"lua":p.lua,
        "menu_id":p.menu_id,"main_talk_id":p.main_talk_id,"solo":p.solo
    }), peer.ip()).await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    app.world
        .reload_quest_tables(&app.pool)
        .await
        .map_err(|e| {
            warn!(index,error=%e,"live quest cache reload failed after save");
            ApiError::Internal
        })?;
    Ok(Json(json!({"ok":true,"index":index,"reloaded":true})))
}

#[derive(Serialize)]
struct LuaOutput {
    filename: String,
    content: String,
}

async fn quest_studio_lua(
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Json<LuaOutput>, ApiError> {
    auth(&headers, false)?;
    if !valid_lua_filename(&filename) {
        return Err(ApiError::BadRequest);
    }
    let path = quest_root().join(&filename);
    let content = tokio::fs::read_to_string(path).await.map_err(|e| {
        warn!(filename,error=%e,"quest Lua read failed");
        ApiError::NotFound
    })?;
    if content.len() > 1024 * 1024 {
        return Err(ApiError::BadRequest);
    }
    Ok(Json(LuaOutput { filename, content }))
}

#[derive(Deserialize)]
struct LuaEdit {
    content: String,
}

async fn quest_studio_save_lua(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(filename): Path<String>,
    Json(p): Json<LuaEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    if !valid_lua_filename(&filename) || p.content.len() > 1024 * 1024 {
        return Err(ApiError::BadRequest);
    }
    // Compile the proposed source before touching the live quest file.
    mlua::Lua::new()
        .load(&p.content)
        .set_name(filename.clone())
        .into_function()
        .map_err(|e| {
            warn!(filename,error=%e,"quest Lua edit rejected by syntax check");
            ApiError::BadRequest
        })?;
    let path = quest_root().join(&filename);
    let bytes = p.content.as_bytes().to_vec();
    tokio::task::spawn_blocking(move || atomic_backup_write(&path, &bytes))
        .await
        .map_err(|_| ApiError::Internal)??;
    let cached = app.world.lua_engine().invalidate_cache();
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &session.actor,
        "quest_lua.save",
        &filename,
        json!({"bytes":p.content.len(),"invalidated_scripts":cached}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(
        json!({"ok":true,"filename":filename,"syntax":"ok","live_cache_reloaded":true,"backup":true}),
    ))
}

fn valid_lua_filename(filename: &str) -> bool {
    filename.len() <= 80
        && filename.ends_with(".lua")
        && filename
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        && !filename.contains("..")
}

fn quest_root() -> PathBuf {
    std::env::var_os("JSTKO_QUEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./Quests"))
}

fn quest_tbl_path(name: &str) -> Result<PathBuf, ApiError> {
    const ALLOWED: &[&str] = &[
        "Quest_Helper.tbl",
        "Quest_Helper_us.tbl",
        "Quest_Talk.tbl",
        "Quest_Talk_us.tbl",
        "Quest_Talk_TK.tbl",
        "Quest_Menu.tbl",
        "Quest_Menu_us.tbl",
        "Quest_Menu_TK.tbl",
    ];
    if !ALLOWED.contains(&name) {
        return Err(ApiError::BadRequest);
    }
    let base = std::env::var_os("JSTKO_TBL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./docs/data"));
    Ok(base.join(name))
}

fn atomic_backup_write(path: &FsPath, bytes: &[u8]) -> Result<(), ApiError> {
    use std::io::Write;
    let parent = path.parent().ok_or(ApiError::BadRequest)?;
    std::fs::create_dir_all(parent).map_err(|_| ApiError::Internal)?;
    if path.exists() {
        let backup_dir = parent.join(".admin-backups");
        std::fs::create_dir_all(&backup_dir).map_err(|_| ApiError::Internal)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let name = path.file_name().ok_or(ApiError::BadRequest)?;
        let backup = backup_dir.join(format!("{}.{}.bak", name.to_string_lossy(), stamp));
        std::fs::copy(path, backup).map_err(|_| ApiError::Internal)?;
    }
    let tmp = parent.join(format!(".quest-edit-{}.tmp", rand::random::<u64>()));
    {
        let mut file = std::fs::File::create(&tmp).map_err(|_| ApiError::Internal)?;
        file.write_all(bytes).map_err(|_| ApiError::Internal)?;
        file.sync_all().map_err(|_| ApiError::Internal)?;
    }
    #[cfg(windows)]
    if path.exists() {
        std::fs::remove_file(path).map_err(|_| ApiError::Internal)?;
    }
    std::fs::rename(&tmp, path).map_err(|_| ApiError::Internal)?;
    Ok(())
}

async fn quest_studio_tbl_search(
    headers: HeaderMap,
    Path(table_name): Path<String>,
    Query(q): Query<Search>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let path = quest_tbl_path(&table_name)?;
    let raw = tokio::fs::read(&path).await.map_err(|e| {
        warn!(table=%table_name,path=?path,error=%e,"quest TBL read failed");
        ApiError::NotFound
    })?;
    let (plain, new_structure) = ko_tbl_import::decrypt::decrypt_tbl(&raw).map_err(|e| {
        warn!(table=%table_name,error=%e,"quest TBL decrypt failed");
        ApiError::Internal
    })?;
    let table = ko_tbl_import::parser::parse_tbl(&plain, new_structure).map_err(|e| {
        warn!(table=%table_name,error=%e,"quest TBL parse failed");
        ApiError::Internal
    })?;
    let term = q.q.unwrap_or_default().trim().to_lowercase();
    let rows: Vec<Value> = table.rows.iter().enumerate()
        .filter_map(|(idx,row)| {
            let cells: Vec<String> = row.iter().map(tbl_cell_string).collect();
            if term.is_empty() || cells.iter().any(|cell| cell.to_lowercase().contains(&term)) {
                Some(json!({"row":idx,"id":cells.first().cloned().unwrap_or_default(),"cells":cells}))
            } else { None }
        })
        .take(q.limit.unwrap_or(100).clamp(1,200) as usize)
        .collect();
    Ok(Json(
        json!({"table":table_name,"row_count":table.rows.len(),"columns":table.columns.len(),"rows":rows}),
    ))
}

#[derive(Deserialize)]
struct TblRowEdit {
    cells: Vec<String>,
}

async fn quest_studio_tbl_save(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path((table_name, row_id)): Path<(String, String)>,
    Json(p): Json<TblRowEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    let path = quest_tbl_path(&table_name)?;
    if row_id.len() > 32 || p.cells.len() > 128 {
        return Err(ApiError::BadRequest);
    }
    let raw = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound)?;
    let (plain, new_structure) =
        ko_tbl_import::decrypt::decrypt_tbl(&raw).map_err(|_| ApiError::Internal)?;
    let mut table =
        ko_tbl_import::parser::parse_tbl(&plain, new_structure).map_err(|_| ApiError::Internal)?;
    if p.cells.len() != table.columns.len() {
        return Err(ApiError::BadRequest);
    }
    let target = table
        .rows
        .iter()
        .position(|row| row.first().map(tbl_cell_string).as_deref() == Some(row_id.as_str()));
    let new_row = p
        .cells
        .iter()
        .enumerate()
        .map(|(index, value)| tbl_parse_cell(&table.columns[index], value))
        .collect::<Result<Vec<_>, _>>()?;
    let inserted = if let Some(row_index) = target {
        table.rows[row_index] = new_row;
        false
    } else {
        table.rows.push(new_row);
        true
    };
    let encoded = ko_tbl_import::decrypt::encrypt_tbl(&ko_tbl_import::parser::serialize_tbl(
        &table,
        new_structure,
    ));
    tokio::task::spawn_blocking(move || atomic_backup_write(&path, &encoded))
        .await
        .map_err(|_| ApiError::Internal)??;
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &session.actor,
        "quest_tbl.row_save",
        &format!("{table_name}:{row_id}"),
        json!({"inserted":inserted,"columns":p.cells.len()}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(
        json!({"ok":true,"table":table_name,"id":row_id,"inserted":inserted,"encrypted":true,"backup":true}),
    ))
}

fn tbl_cell_string(cell: &ko_tbl_import::parser::CellValue) -> String {
    use ko_tbl_import::parser::CellValue as C;
    match cell {
        C::I8(v) => v.to_string(),
        C::U8(v) => v.to_string(),
        C::I16(v) => v.to_string(),
        C::U16(v) => v.to_string(),
        C::I32(v) => v.to_string(),
        C::U32(v) => v.to_string(),
        C::Str(v) => v.clone(),
        C::F32(v) => v.to_string(),
        C::F64(v) => v.to_string(),
        C::I64(v) => v.to_string(),
        C::U64(v) => v.to_string(),
    }
}

fn tbl_parse_cell(
    kind: &ko_tbl_import::parser::ColumnType,
    value: &str,
) -> Result<ko_tbl_import::parser::CellValue, ApiError> {
    use ko_tbl_import::parser::{CellValue as C, ColumnType as T};
    macro_rules! parse {
        ($ty:ty, $variant:ident) => {
            value
                .parse::<$ty>()
                .map(C::$variant)
                .map_err(|_| ApiError::BadRequest)
        };
    }
    match kind {
        T::SignedByte => parse!(i8, I8),
        T::UnsignedByte => parse!(u8, U8),
        T::SignedShort => parse!(i16, I16),
        T::UnsignedShort => parse!(u16, U16),
        T::SignedInt => parse!(i32, I32),
        T::UnsignedInt => parse!(u32, U32),
        T::String => Ok(C::Str(value.to_owned())),
        T::Float => parse!(f32, F32),
        T::Double => parse!(f64, F64),
        T::SignedLong => parse!(i64, I64),
        T::UnsignedLong => parse!(u64, U64),
    }
}

async fn quest_studio_reload(
    headers: HeaderMap,
    State(app): State<App>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, true)?;
    let helpers = app
        .world
        .reload_quest_tables(&app.pool)
        .await
        .map_err(|e| {
            warn!(error=%e,"quest studio live table reload failed");
            ApiError::Internal
        })?;
    let scripts = app.world.lua_engine().invalidate_cache();
    Ok(Json(
        json!({"ok":true,"quest_helpers":helpers,"scripts_invalidated":scripts}),
    ))
}

#[derive(Deserialize)]
struct AdminItemLetterInput {
    recipient: String,
    item_id: i32,
    count: i16,
    durability: i16,
    sender: String,
    subject: String,
    message: String,
}

async fn send_item_letter(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(app): State<App>,
    Json(input): Json<AdminItemLetterInput>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    let recipient_query = input.recipient.trim();
    let sender = input.sender.trim();
    let subject = input.subject.trim();
    let message = input.message.trim();
    if recipient_query.is_empty()
        || recipient_query.chars().count() > 21
        || input.item_id <= 0
        || input.count < 1
        || input.durability < 0
        || sender.is_empty()
        || sender.chars().count() > 21
        || sender.chars().any(char::is_control)
        || subject.is_empty()
        || subject.chars().count() > 32
        || subject.chars().any(char::is_control)
        || message.is_empty()
        || message.chars().count() > 128
        || message.chars().any(char::is_control)
    {
        return Err(ApiError::BadRequest);
    }

    let recipient = sqlx::query_scalar::<_, String>(
        "SELECT str_user_id FROM userdata WHERE lower(str_user_id)=lower($1) LIMIT 1",
    )
    .bind(recipient_query)
    .fetch_optional(&app.pool)
    .await
    .map_err(|_| ApiError::Internal)?
    .ok_or(ApiError::NotFound)?;

    let item = sqlx::query("SELECT countable FROM item WHERE num=$1")
        .bind(input.item_id)
        .fetch_optional(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?
        .ok_or(ApiError::NotFound)?;
    let countable = item
        .try_get::<Option<i32>, _>("countable")
        .ok()
        .flatten()
        .unwrap_or(0)
        != 0;
    if !countable && input.count != 1 {
        return Err(ApiError::BadRequest);
    }

    let now = chrono::Utc::now();
    let send_date = (now.format("%y%m%d").to_string())
        .parse::<i32>()
        .map_err(|_| ApiError::Internal)?;
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let letter_id = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO letter (sender_name,recipient_name,subject,message,b_type,item_id,item_count,item_durability,item_serial,item_expiry,coins,send_date,days_remaining)
         VALUES ($1,$2,$3,$4,2,$5,$6,$7,0,0,0,$8,30) RETURNING letter_id"#,
    )
    .bind(sender)
    .bind(&recipient)
    .bind(subject)
    .bind(message)
    .bind(input.item_id)
    .bind(input.count)
    .bind(input.durability)
    .bind(send_date)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;
    sqlx::query("INSERT INTO letter_admin_notification(letter_id,recipient_name) VALUES($1,$2)")
        .bind(letter_id)
        .bind(&recipient)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &session.actor,
        "letter.item.send",
        &recipient,
        json!({"letter_id":letter_id,"item_id":input.item_id,"count":input.count,"durability":input.durability,"subject":subject}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(
        json!({"ok":true,"letter_id":letter_id,"recipient":recipient}),
    ))
}

async fn resolve_account(pool: &DbPool, name: &str) -> Result<String, ApiError> {
    sqlx::query_scalar::<_,String>("SELECT a.str_account_id FROM account_char a WHERE lower(a.str_char_id1)=lower($1) OR lower(a.str_char_id2)=lower($1) OR lower(a.str_char_id3)=lower($1) OR lower(a.str_char_id4)=lower($1) LIMIT 1").bind(name).fetch_optional(pool).await.map_err(|_|ApiError::Internal)?.ok_or(ApiError::NotFound)
}
async fn inventory(
    headers: HeaderMap,
    State(app): State<App>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    if character_online(&app.pool, &name).await? {
        return Err(ApiError::Conflict);
    };
    let rows=sqlx::query("SELECT u.slot_index,u.item_id,u.count,u.durability,u.flag,u.serial_num,u.expire_time,COALESCE(i.str_name,'') name,COALESCE(i.item_icon_id1,0)::int icon FROM user_items u LEFT JOIN item i ON i.num=u.item_id WHERE lower(u.str_user_id)=lower($1) ORDER BY u.slot_index").bind(name).fetch_all(&app.pool).await.map_err(|_|ApiError::Internal)?;
    Ok(Json(Value::Array(rows.into_iter().map(|r|json!({"slot":r.try_get::<i16,_>("slot_index").unwrap_or(0),"id":r.try_get::<i32,_>("item_id").unwrap_or(0),"count":r.try_get::<i16,_>("count").unwrap_or(0),"durability":r.try_get::<i16,_>("durability").unwrap_or(0),"flag":r.try_get::<i16,_>("flag").unwrap_or(0),"serial":r.try_get::<i64,_>("serial_num").unwrap_or(0),"expire":r.try_get::<i32,_>("expire_time").unwrap_or(0),"name":r.try_get::<String,_>("name").unwrap_or_default(),"icon":r.try_get::<i32,_>("icon").unwrap_or(0)})).collect())))
}
async fn character_online(pool: &DbPool, name: &str) -> Result<bool, ApiError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_char_id)=lower($1))",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::Internal)
}
#[derive(Deserialize)]
struct SlotEdit {
    slot: i16,
    item_id: i32,
    count: i16,
    #[serde(default)]
    durability: i16,
}
async fn set_inventory(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
    Json(p): Json<SlotEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, true)?;
    if !(0..96).contains(&p.slot) || p.count < 0 || p.item_id < 0 {
        return Err(ApiError::BadRequest);
    };
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    if sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_char_id)=lower($1))",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?
    {
        return Err(ApiError::Conflict);
    };
    let valid_drop_code = if p.item_id == 0 {
        true
    } else {
        sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(SELECT 1 FROM item WHERE num=$1) OR
               ($1 BETWEEN 100 AND 99999999 AND EXISTS(SELECT 1 FROM make_item_group WHERE group_num=$1))"#,
        )
        .bind(p.item_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?
    };
    if !valid_drop_code {
        return Err(ApiError::BadRequest);
    };
    let serial = if p.item_id == 0 {
        0
    } else {
        (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64)
            .wrapping_abs()
            .max(1)
    };
    sqlx::query("INSERT INTO user_items(str_user_id,slot_index,item_id,count,durability,serial_num) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(str_user_id,slot_index) DO UPDATE SET item_id=EXCLUDED.item_id,count=EXCLUDED.count,durability=EXCLUDED.durability,serial_num=EXCLUDED.serial_num,flag=0,expire_time=0,original_flag=0").bind(&name).bind(p.slot).bind(p.item_id).bind(if p.item_id==0{0}else{p.count.max(1)}).bind(p.durability).bind(serial).execute(&mut *tx).await.map_err(|e|{warn!(character=%name,slot=p.slot,item_id=p.item_id,error=%e,"admin inventory slot write failed");ApiError::Internal})?;
    audit(
        &mut tx,
        &s.actor,
        "inventory.slot.set",
        &format!("{name}:{}", p.slot),
        json!({"item_id":p.item_id,"count":p.count}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok":true})))
}

async fn warehouse(
    headers: HeaderMap,
    State(app): State<App>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    if character_online(&app.pool, &name).await? {
        return Err(ApiError::Conflict);
    };
    let account = resolve_account(&app.pool, &name).await?;
    let rows=sqlx::query("SELECT u.slot_index,u.item_id,u.count,u.durability,u.flag,u.serial_num,u.expire_time,COALESCE(i.str_name,'') name,COALESCE(i.item_icon_id1,0)::int icon FROM user_warehouse u LEFT JOIN item i ON i.num=u.item_id WHERE u.str_account_id=$1 ORDER BY u.slot_index").bind(account).fetch_all(&app.pool).await.map_err(|_|ApiError::Internal)?;
    Ok(Json(Value::Array(rows.into_iter().map(|r|json!({"slot":r.try_get::<i16,_>("slot_index").unwrap_or(0),"id":r.try_get::<i32,_>("item_id").unwrap_or(0),"count":r.try_get::<i16,_>("count").unwrap_or(0),"durability":r.try_get::<i16,_>("durability").unwrap_or(0),"flag":r.try_get::<i16,_>("flag").unwrap_or(0),"serial":r.try_get::<i64,_>("serial_num").unwrap_or(0),"expire":r.try_get::<i32,_>("expire_time").unwrap_or(0),"name":r.try_get::<String,_>("name").unwrap_or_default(),"icon":r.try_get::<i32,_>("icon").unwrap_or(0)})).collect())))
}
async fn set_warehouse(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
    Json(p): Json<SlotEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, true)?;
    if !(0..192).contains(&p.slot) || p.count < 0 || p.item_id < 0 {
        return Err(ApiError::BadRequest);
    };
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    if sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_char_id)=lower($1))",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?
    {
        return Err(ApiError::Conflict);
    };
    let account = resolve_account(&app.pool, &name).await?;
    let valid_item = p.item_id == 0
        || sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM item WHERE num=$1)")
            .bind(p.item_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal)?;
    if !valid_item {
        return Err(ApiError::BadRequest);
    };
    let serial = if p.item_id == 0 {
        0
    } else {
        (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64)
            .wrapping_abs()
            .max(1)
    };
    sqlx::query("INSERT INTO user_warehouse(str_account_id,slot_index,item_id,count,durability,serial_num) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(str_account_id,slot_index) DO UPDATE SET item_id=EXCLUDED.item_id,count=EXCLUDED.count,durability=EXCLUDED.durability,serial_num=EXCLUDED.serial_num,flag=0,expire_time=0,original_flag=0").bind(&account).bind(p.slot).bind(p.item_id).bind(if p.item_id==0{0}else{p.count.max(1)}).bind(p.durability).bind(serial).execute(&mut *tx).await.map_err(|e|{warn!(account=%account,slot=p.slot,item_id=p.item_id,error=%e,"admin warehouse slot write failed");ApiError::Internal})?;
    audit(
        &mut tx,
        &s.actor,
        "warehouse.slot.set",
        &format!("{account}:{}", p.slot),
        json!({"item_id":p.item_id,"count":p.count}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct WarehouseMove {
    from_slot: i16,
    to_slot: i16,
}

/// Atomically swaps/moves two bank slots so drag-and-drop never duplicates or
/// loses an item if the browser disconnects between requests.
async fn move_warehouse_slots(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
    Json(p): Json<WarehouseMove>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    if !(0..192).contains(&p.from_slot) || !(0..192).contains(&p.to_slot) {
        return Err(ApiError::BadRequest);
    }
    if p.from_slot == p.to_slot {
        return Ok(Json(json!({"ok":true})));
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    if sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_char_id)=lower($1))",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?
    {
        return Err(ApiError::Conflict);
    }
    let account = resolve_account(&app.pool, &name).await?;
    let rows = sqlx::query("SELECT slot_index,item_id,count,durability,flag,serial_num,expire_time FROM user_warehouse WHERE str_account_id=$1 AND slot_index IN ($2,$3) ORDER BY slot_index FOR UPDATE")
        .bind(&account).bind(p.from_slot).bind(p.to_slot).fetch_all(&mut *tx).await.map_err(|_|ApiError::Internal)?;
    let read_slot = |slot: i16| -> (i32, i16, i16, i16, i64, i32) {
        rows.iter()
            .find(|r| r.try_get::<i16, _>("slot_index").ok() == Some(slot))
            .map(|r| {
                (
                    r.try_get("item_id").unwrap_or(0),
                    r.try_get("count").unwrap_or(0),
                    r.try_get("durability").unwrap_or(0),
                    r.try_get("flag").unwrap_or(0),
                    r.try_get("serial_num").unwrap_or(0),
                    r.try_get("expire_time").unwrap_or(0),
                )
            })
            .unwrap_or((0, 0, 0, 0, 0, 0))
    };
    let from = read_slot(p.from_slot);
    let to = read_slot(p.to_slot);
    for (slot, value) in [(p.from_slot, to), (p.to_slot, from)] {
        sqlx::query("INSERT INTO user_warehouse(str_account_id,slot_index,item_id,count,durability,flag,serial_num,expire_time) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(str_account_id,slot_index) DO UPDATE SET item_id=EXCLUDED.item_id,count=EXCLUDED.count,durability=EXCLUDED.durability,flag=EXCLUDED.flag,serial_num=EXCLUDED.serial_num,expire_time=EXCLUDED.expire_time")
            .bind(&account).bind(slot).bind(value.0).bind(value.1).bind(value.2).bind(value.3).bind(value.4).bind(value.5)
            .execute(&mut *tx).await.map_err(|_|ApiError::Internal)?;
    }
    audit(
        &mut tx,
        &session.actor,
        "warehouse.slot.move",
        &format!("{account}:{}->{}", p.from_slot, p.to_slot),
        json!({"from_slot":p.from_slot,"to_slot":p.to_slot}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok":true})))
}

/// Atomically swap two bag slots while preserving item flags, serials and expiry.
async fn move_inventory_slots(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(name): Path<String>,
    Json(p): Json<WarehouseMove>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    if !(14..96).contains(&p.from_slot) || !(14..96).contains(&p.to_slot) {
        return Err(ApiError::BadRequest);
    }
    if p.from_slot == p.to_slot {
        return Ok(Json(json!({"ok":true})));
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    if sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM currentuser WHERE lower(str_char_id)=lower($1))",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?
    {
        return Err(ApiError::Conflict);
    }
    let rows = sqlx::query("SELECT slot_index,item_id,count,durability,flag,original_flag,serial_num,expire_time FROM user_items WHERE lower(str_user_id)=lower($1) AND slot_index IN ($2,$3) ORDER BY slot_index FOR UPDATE")
        .bind(&name).bind(p.from_slot).bind(p.to_slot).fetch_all(&mut *tx).await.map_err(|e|{warn!(character=%name,error=%e,"admin inventory slot move read failed");ApiError::Internal})?;
    let read_slot = |slot: i16| -> (i32, i16, i16, i16, i16, i64, i32) {
        rows.iter()
            .find(|r| r.try_get::<i16, _>("slot_index").ok() == Some(slot))
            .map(|r| {
                (
                    r.try_get("item_id").unwrap_or(0),
                    r.try_get("count").unwrap_or(0),
                    r.try_get("durability").unwrap_or(0),
                    r.try_get("flag").unwrap_or(0),
                    r.try_get("original_flag").unwrap_or(0),
                    r.try_get("serial_num").unwrap_or(0),
                    r.try_get("expire_time").unwrap_or(0),
                )
            })
            .unwrap_or((0, 0, 0, 0, 0, 0, 0))
    };
    let from = read_slot(p.from_slot);
    let to = read_slot(p.to_slot);
    for (slot, value) in [(p.from_slot, to), (p.to_slot, from)] {
        sqlx::query("INSERT INTO user_items(str_user_id,slot_index,item_id,count,durability,flag,original_flag,serial_num,expire_time) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(str_user_id,slot_index) DO UPDATE SET item_id=EXCLUDED.item_id,count=EXCLUDED.count,durability=EXCLUDED.durability,flag=EXCLUDED.flag,original_flag=EXCLUDED.original_flag,serial_num=EXCLUDED.serial_num,expire_time=EXCLUDED.expire_time")
            .bind(&name).bind(slot).bind(value.0).bind(value.1).bind(value.2).bind(value.3).bind(value.4).bind(value.5).bind(value.6)
            .execute(&mut *tx).await.map_err(|e|{warn!(character=%name,from=p.from_slot,to=p.to_slot,error=%e,"admin inventory slot move write failed");ApiError::Internal})?;
    }
    audit(
        &mut tx,
        &session.actor,
        "inventory.slot.move",
        &format!("{name}:{}->{}", p.from_slot, p.to_slot),
        json!({"from_slot":p.from_slot,"to_slot":p.to_slot}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok":true})))
}

async fn drop_tables(
    headers: HeaderMap,
    State(app): State<App>,
    Query(q): Query<Search>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default();
    let rows=sqlx::query("WITH selected AS (SELECT n.s_sid,n.str_name,n.s_item,m.* FROM npc_template n LEFT JOIN monster_item m ON m.s_index=n.s_item WHERE n.is_monster=true AND ($1='' OR n.str_name ILIKE $2 OR CAST(n.s_sid AS TEXT)=$1) ORDER BY n.str_name LIMIT $3) SELECT s.s_sid,s.str_name,s.s_item,d.slot,d.item_id,d.percent,COALESCE(i.str_name,'') item_name,COALESCE(i.item_icon_id1,0)::int icon FROM selected s CROSS JOIN LATERAL (VALUES (1,s.item01,s.percent01),(2,s.item02,s.percent02),(3,s.item03,s.percent03),(4,s.item04,s.percent04),(5,s.item05,s.percent05),(6,s.item06,s.percent06),(7,s.item07,s.percent07),(8,s.item08,s.percent08),(9,s.item09,s.percent09),(10,s.item10,s.percent10),(11,s.item11,s.percent11),(12,s.item12,s.percent12)) AS d(slot,item_id,percent) LEFT JOIN item i ON i.num=d.item_id ORDER BY s.str_name,d.slot").bind(&term).bind(format!("%{term}%")).bind(q.limit.unwrap_or(100).clamp(1,100)).fetch_all(&app.pool).await.map_err(|_|ApiError::Internal)?;
    let mut monsters: Vec<Value> = Vec::new();
    for r in rows {
        let sid = r.try_get::<i16, _>("s_sid").unwrap_or(0);
        let index = if let Some(index) = monsters.iter().position(|m| m["sid"] == sid) {
            index
        } else {
            monsters.push(json!({"sid":sid,"name":r.try_get::<String,_>("str_name").unwrap_or_default(),"s_item":r.try_get::<i16,_>("s_item").unwrap_or(0),"slots":[]}));
            monsters.len() - 1
        };
        if let Some(slots) = monsters[index]["slots"].as_array_mut() {
            slots.push(json!({"slot":r.try_get::<i32,_>("slot").unwrap_or(0),"item_id":r.try_get::<Option<i32>,_>("item_id").ok().flatten().unwrap_or(0),"percent":r.try_get::<Option<i16>,_>("percent").ok().flatten().unwrap_or(0),"name":r.try_get::<String,_>("item_name").unwrap_or_default(),"icon":r.try_get::<i32,_>("icon").unwrap_or(0)}));
        }
    }
    Ok(Json(Value::Array(monsters)))
}
#[derive(Deserialize)]
struct DropEdit {
    slot: u8,
    item_id: i32,
    percent: i16,
}
async fn edit_drop(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(sid): Path<i16>,
    Json(p): Json<DropEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, true)?;
    if !(1..=12).contains(&p.slot) || p.item_id < 0 || !(0..=10000).contains(&p.percent) {
        return Err(ApiError::BadRequest);
    };
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let s_item: i16 = sqlx::query_scalar(
        "SELECT s_item FROM npc_template WHERE s_sid=$1 AND is_monster=true LIMIT 1",
    )
    .bind(sid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?
    .ok_or(ApiError::NotFound)?;
    // Monster slots accept concrete item IDs and MAKE_ITEM_GROUP group IDs.
    // Validate the group against its own table instead of rejecting it because
    // no matching `item.num` row exists.
    let valid_drop_code = p.item_id == 0
        || sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(SELECT 1 FROM item WHERE num=$1) OR
               ($1 BETWEEN 100 AND 99999999 AND EXISTS(
                   SELECT 1 FROM make_item_group WHERE group_num=$1
               ))"#,
        )
        .bind(p.item_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;
    if !valid_drop_code {
        return Err(ApiError::BadRequest);
    };
    sqlx::query("INSERT INTO monster_item(s_index) VALUES($1) ON CONFLICT(s_index) DO NOTHING")
        .bind(s_item)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;
    let item_col = format!("item{:02}", p.slot);
    let pct_col = format!("percent{:02}", p.slot);
    let sql = format!("UPDATE monster_item SET {item_col}=$2,{pct_col}=$3 WHERE s_index=$1");
    sqlx::query(&sql)
        .bind(s_item)
        .bind(p.item_id)
        .bind(p.percent)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &s.actor,
        "drop.slot.set",
        &format!("npc:{sid}:{}", p.slot),
        json!({"item_id":p.item_id,"percent":p.percent}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    let live_rows = app
        .world
        .reload_monster_drop_tables(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok":true,"live_rows":live_rows})))
}

#[derive(Deserialize)]
struct BulkDropAdjustment {
    /// Signed relative percentage, e.g. 20 means +20%, -30 means -30%.
    percent: i16,
}

async fn adjust_all_monster_drops(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<BulkDropAdjustment>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    if !(-100..=500).contains(&p.percent) || p.percent == 0 {
        return Err(ApiError::BadRequest);
    }

    // Update only populated monster drop slots. Percentages are stored on a
    // 0..10000 basis, so clamp after applying the relative adjustment.
    let mut assignments = Vec::with_capacity(12);
    for slot in 1..=12 {
        assignments.push(format!(
            "percent{slot:02}=CASE WHEN m.item{slot:02}>0 THEN GREATEST(0,LEAST(10000,ROUND(m.percent{slot:02}::numeric*(100+$1)::numeric/100)::integer))::smallint ELSE m.percent{slot:02} END"
        ));
    }
    let statement = format!(
        "UPDATE monster_item m SET {} WHERE EXISTS (SELECT 1 FROM npc_template n WHERE n.is_monster=true AND n.s_item=m.s_index)",
        assignments.join(",")
    );
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let affected = sqlx::query(&statement)
        .bind(p.percent)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?
        .rows_affected();
    audit(
        &mut tx,
        &session.actor,
        "drop.bulk_adjust",
        "all-monsters",
        json!({"percent":p.percent,"monster_drop_rows":affected}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;

    let live_rows = app
        .world
        .reload_monster_drop_tables(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({
        "ok":true,
        "percent":p.percent,
        "updated_rows":affected,
        "live_rows":live_rows
    })))
}

async fn drop_groups(
    headers: HeaderMap,
    State(app): State<App>,
    Query(q): Query<Search>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    if term.is_empty() {
        return Ok(Json(json!([])));
    }
    let rows = sqlx::query(
        r#"SELECT g.group_num, COUNT(u.item_id)::int AS item_count,
           COALESCE(string_agg(DISTINCT i.str_name, ', ' ORDER BY i.str_name)
             FILTER (WHERE i.str_name IS NOT NULL),'') AS examples,
           COALESCE(MIN(i.item_icon_id1),0)::int AS icon
           FROM make_item_group g
           LEFT JOIN LATERAL unnest(g.items) AS u(item_id) ON true
           LEFT JOIN item i ON i.num=u.item_id
           WHERE g.group_num BETWEEN 100 AND 99999999
             AND (CAST(g.group_num AS TEXT)=$1 OR i.str_name ILIKE $2)
           GROUP BY g.group_num ORDER BY g.group_num LIMIT $3"#,
    )
    .bind(&term)
    .bind(format!("%{term}%"))
    .bind(q.limit.unwrap_or(50).clamp(1, 100))
    .fetch_all(&app.pool)
    .await
    .map_err(|e| {
        warn!(error=%e, "drop group search failed");
        ApiError::Internal
    })?;
    Ok(Json(Value::Array(
        rows.into_iter()
            .map(|r| {
                json!({
                    "group_num":r.try_get::<i32,_>("group_num").unwrap_or(0),
                    "item_count":r.try_get::<i32,_>("item_count").unwrap_or(0),
                    "examples":r.try_get::<String,_>("examples").unwrap_or_default(),
                    "icon":r.try_get::<i32,_>("icon").unwrap_or(0),
                })
            })
            .collect(),
    )))
}

async fn group_items(
    headers: HeaderMap,
    State(app): State<App>,
    Query(q): Query<Search>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    let rows = sqlx::query(
        // MAKE_ITEM_GROUP arrays are fixed-width in the source DB and use
        // trailing zeroes as empty slots. They are not valid item IDs and must
        // not be sent to the editor's save API.
        "WITH matched AS (SELECT DISTINCT g.group_num FROM make_item_group g LEFT JOIN LATERAL unnest(g.items) AS u(item_id) ON u.item_id > 0 LEFT JOIN item i ON i.num=u.item_id WHERE ($1='' OR g.group_num::text=$1 OR u.item_id::text=$1 OR i.str_name ILIKE $2) ORDER BY g.group_num LIMIT $3) SELECT g.group_num,u.ordinality,u.item_id,COALESCE(i.str_name,'') AS item_name,COALESCE(i.item_icon_id1,0)::int AS icon FROM matched m JOIN make_item_group g ON g.group_num=m.group_num LEFT JOIN LATERAL unnest(g.items) WITH ORDINALITY AS u(item_id,ordinality) ON u.item_id > 0 LEFT JOIN item i ON i.num=u.item_id ORDER BY g.group_num,u.ordinality",
    )
    .bind(&term)
    .bind(format!("%{term}%"))
    .bind(q.limit.unwrap_or(100).clamp(1, 200))
    .fetch_all(&app.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    let mut groups: Vec<Value> = Vec::new();
    for row in rows {
        let group_num = row.try_get::<i32, _>("group_num").unwrap_or_default();
        let index = if let Some(index) = groups.iter().position(|g| g["group_num"] == group_num) {
            index
        } else {
            groups.push(json!({"group_num":group_num,"items":[]}));
            groups.len() - 1
        };
        if let Some(slot) = row.try_get::<Option<i64>, _>("ordinality").ok().flatten() {
            let item_id = row
                .try_get::<Option<i32>, _>("item_id")
                .ok()
                .flatten()
                .unwrap_or(0);
            if let Some(items) = groups[index]["items"].as_array_mut() {
                items.push(json!({
                    "slot": slot,
                    "item_id": item_id,
                    "name": row.try_get::<String, _>("item_name").unwrap_or_default(),
                    "icon": row.try_get::<i32, _>("icon").unwrap_or_default()
                }));
            }
        }
    }
    Ok(Json(Value::Array(groups)))
}

#[derive(Deserialize)]
struct GroupItemsEdit {
    items: Vec<i32>,
}

async fn save_group_items(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(group_num): Path<i32>,
    Json(p): Json<GroupItemsEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    if !(1..100_000_000).contains(&group_num)
        || p.items.len() > 200
        || p.items.iter().any(|id| *id <= 0)
    {
        return Err(ApiError::BadRequest);
    }
    let unique_count = p
        .items
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>()
        .len();
    let existing_count: i64 = sqlx::query_scalar("SELECT count(*) FROM item WHERE num=ANY($1)")
        .bind(&p.items)
        .fetch_one(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if existing_count < unique_count as i64 {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    sqlx::query("INSERT INTO make_item_group(group_num,items) VALUES($1,$2) ON CONFLICT(group_num) DO UPDATE SET items=EXCLUDED.items")
        .bind(group_num)
        .bind(&p.items)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &session.actor,
        "group_item.save",
        &group_num.to_string(),
        json!({"item_count":p.items.len(),"items":p.items}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    let live_groups = app
        .world
        .reload_make_item_groups(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(Json(
        json!({"ok":true,"group_num":group_num,"live_groups":live_groups}),
    ))
}

async fn delete_group_items(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(group_num): Path<i32>,
) -> Result<StatusCode, ApiError> {
    let (_, session) = auth(&headers, true)?;
    if !(1..100_000_000).contains(&group_num) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let deleted = sqlx::query("DELETE FROM make_item_group WHERE group_num=$1 RETURNING group_num")
        .bind(group_num)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal)?
        .ok_or(ApiError::NotFound)?;
    let removed = deleted
        .try_get::<i32, _>("group_num")
        .map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &session.actor,
        "group_item.delete",
        &removed.to_string(),
        json!({}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    app.world
        .reload_make_item_groups(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reload_editable_tables(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
) -> Result<Json<Value>, ApiError> {
    let (_, session) = auth(&headers, true)?;
    let drops = app
        .world
        .reload_monster_drop_tables(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    let groups = app
        .world
        .reload_make_item_groups(&app.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &session.actor,
        "tables.reload_editable",
        "monster_item,make_item_group",
        json!({"drop_rows":drops,"group_rows":groups}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(
        json!({"ok":true,"drop_rows":drops,"group_rows":groups}),
    ))
}

async fn start_positions(
    headers: HeaderMap,
    State(app): State<App>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let rows=sqlx::query("SELECT zone_id,karus_x,karus_z,elmorad_x,elmorad_z,karus_gate_x,karus_gate_z,elmo_gate_x,elmo_gate_z FROM start_position ORDER BY zone_id").fetch_all(&app.pool).await.map_err(|_|ApiError::Internal)?;
    Ok(Json(Value::Array(rows.into_iter().map(|r|json!({"zone":r.try_get::<i16,_>("zone_id").unwrap_or(0),"karus_x":r.try_get::<i32,_>("karus_x").unwrap_or(0),"karus_z":r.try_get::<i32,_>("karus_z").unwrap_or(0),"elmorad_x":r.try_get::<i32,_>("elmorad_x").unwrap_or(0),"elmorad_z":r.try_get::<i32,_>("elmorad_z").unwrap_or(0),"karus_gate_x":r.try_get::<i32,_>("karus_gate_x").unwrap_or(0),"karus_gate_z":r.try_get::<i32,_>("karus_gate_z").unwrap_or(0),"elmo_gate_x":r.try_get::<i32,_>("elmo_gate_x").unwrap_or(0),"elmo_gate_z":r.try_get::<i32,_>("elmo_gate_z").unwrap_or(0)})).collect())))
}
#[derive(Deserialize)]
struct StartEdit {
    karus_x: i32,
    karus_z: i32,
    elmorad_x: i32,
    elmorad_z: i32,
    karus_gate_x: i32,
    karus_gate_z: i32,
    elmo_gate_x: i32,
    elmo_gate_z: i32,
}
async fn edit_start_position(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(zone): Path<i16>,
    Json(p): Json<StartEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, true)?;
    if !(0..=255).contains(&zone)
        || [
            p.karus_x,
            p.karus_z,
            p.elmorad_x,
            p.elmorad_z,
            p.karus_gate_x,
            p.karus_gate_z,
            p.elmo_gate_x,
            p.elmo_gate_z,
        ]
        .iter()
        .any(|v| *v < 0 || *v > i16::MAX as i32)
    {
        return Err(ApiError::BadRequest);
    };
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let result=sqlx::query("UPDATE start_position SET karus_x=$2,karus_z=$3,elmorad_x=$4,elmorad_z=$5,karus_gate_x=$6,karus_gate_z=$7,elmo_gate_x=$8,elmo_gate_z=$9 WHERE zone_id=$1").bind(zone).bind(p.karus_x as i16).bind(p.karus_z as i16).bind(p.elmorad_x as i16).bind(p.elmorad_z as i16).bind(p.karus_gate_x as i16).bind(p.karus_gate_z as i16).bind(p.elmo_gate_x as i16).bind(p.elmo_gate_z as i16).execute(&mut *tx).await.map_err(|_|ApiError::Internal)?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    };
    audit(&mut tx,&s.actor,"start_position.edit",&zone.to_string(),json!({"karus_x":p.karus_x,"karus_z":p.karus_z,"elmorad_x":p.elmorad_x,"elmorad_z":p.elmorad_z}),peer.ip()).await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(
        json!({"ok":true,"note":"DB güncellendi; çalışan oyun dünyasındaki önbellek sunucu yeniden başladığında yenilenir."}),
    ))
}

async fn starting_characters(
    headers: HeaderMap,
    State(app): State<App>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    let rows = sqlx::query("SELECT n_index,class_type,job_type,level,exp,free_points,skill_point_free,gold FROM create_new_char_value ORDER BY job_type,class_type")
        .fetch_all(&app.pool).await.map_err(|_| ApiError::Internal)?;
    Ok(Json(Value::Array(rows.into_iter().map(|r| json!({
        "id":r.try_get::<i32,_>("n_index").unwrap_or(0), "class":r.try_get::<i16,_>("class_type").unwrap_or(0),
        "job":r.try_get::<i16,_>("job_type").unwrap_or(0), "level":r.try_get::<i16,_>("level").unwrap_or(1),
        "exp":r.try_get::<i64,_>("exp").unwrap_or(0), "points":r.try_get::<i16,_>("free_points").unwrap_or(0),
        "skill_points":r.try_get::<i16,_>("skill_point_free").unwrap_or(0), "gold":r.try_get::<i32,_>("gold").unwrap_or(0)
    })).collect())))
}
#[derive(Deserialize)]
struct StartingCharacterEdit {
    level: i16,
    exp: i64,
    points: i16,
    skill_points: i16,
    gold: i32,
}
async fn edit_starting_character(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path((class, job)): Path<(i16, i16)>,
    Json(p): Json<StartingCharacterEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, true)?;
    if ![1, 2, 3, 4, 13].contains(&class)
        || !(0..=4).contains(&job)
        || !(1..=83).contains(&p.level)
        || p.exp < 0
        || p.points < 0
        || p.skill_points < 0
        || p.gold < 0
    {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    let result=sqlx::query("UPDATE create_new_char_value SET level=$3,exp=$4,free_points=$5,skill_point_free=$6,gold=$7 WHERE class_type=$1 AND job_type=$2").bind(class).bind(job).bind(p.level).bind(p.exp).bind(p.points).bind(p.skill_points).bind(p.gold).execute(&mut *tx).await.map_err(|_|ApiError::Internal)?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    audit(&mut tx,&s.actor,"starting_character.edit",&format!("{class}:{job}"),json!({"level":p.level,"exp":p.exp,"points":p.points,"skill_points":p.skill_points,"gold":p.gold}),peer.ip()).await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok":true})))
}
async fn starting_items(
    headers: HeaderMap,
    State(app): State<App>,
    Path((class, job)): Path<(i16, i16)>,
) -> Result<Json<Value>, ApiError> {
    auth(&headers, false)?;
    if ![1, 2, 3, 4, 13].contains(&class) || !(0..=4).contains(&job) {
        return Err(ApiError::BadRequest);
    }
    let mut rows=sqlx::query("SELECT s.slot_id,s.item_id,s.item_duration,s.item_count,s.item_flag,s.item_expire_time,COALESCE(i.str_name,'') AS name,COALESCE(i.item_icon_id1,0)::int icon FROM create_new_char_set_level s LEFT JOIN item i ON i.num=s.item_id WHERE s.class_type=$1 AND s.beginner_type=$2 ORDER BY s.slot_id").bind(class).bind(job).fetch_all(&app.pool).await.map_err(|_|ApiError::Internal)?;
    if rows.is_empty() {
        rows=sqlx::query("SELECT s.slot_id,s.item_id,s.item_duration,s.item_count,s.item_flag,s.item_expire_time,COALESCE(i.str_name,'') AS name,COALESCE(i.item_icon_id1,0)::int icon FROM create_new_char_set s LEFT JOIN item i ON i.num=s.item_id WHERE s.class_type=$1 AND s.item_id>0 ORDER BY s.slot_id").bind(class).fetch_all(&app.pool).await.map_err(|_|ApiError::Internal)?;
    }
    Ok(Json(Value::Array(rows.into_iter().map(|r|json!({"slot":r.try_get::<i32,_>("slot_id").unwrap_or(0),"id":r.try_get::<i32,_>("item_id").unwrap_or(0),"durability":r.try_get::<i16,_>("item_duration").unwrap_or(0),"count":r.try_get::<i16,_>("item_count").unwrap_or(0),"flag":r.try_get::<i16,_>("item_flag").unwrap_or(0),"expire":r.try_get::<i32,_>("item_expire_time").unwrap_or(0),"name":r.try_get::<String,_>("name").unwrap_or_default(),"icon":r.try_get::<i32,_>("icon").unwrap_or(0)})).collect())))
}
#[derive(Deserialize)]
struct StartingItemEdit {
    slot: i32,
    item_id: i32,
    count: i16,
    durability: i16,
}
async fn edit_starting_item(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path((class, job)): Path<(i16, i16)>,
    Json(p): Json<StartingItemEdit>,
) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, true)?;
    if ![1, 2, 3, 4, 13].contains(&class)
        || !(0..=4).contains(&job)
        || !(0..96).contains(&p.slot)
        || p.item_id < 0
        || p.count < 0
        || p.durability < 0
    {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    if p.item_id > 0
        && !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM item WHERE num=$1)")
            .bind(p.item_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal)?
    {
        return Err(ApiError::BadRequest);
    }
    sqlx::query("INSERT INTO create_new_char_set_level(class_type,beginner_type,slot_id,item_id,item_duration,item_count,item_flag,item_expire_time) SELECT class_type,$2,slot_id,item_id,item_duration,item_count,item_flag,item_expire_time FROM create_new_char_set WHERE class_type=$1 ON CONFLICT(class_type,beginner_type,slot_id) DO NOTHING").bind(class).bind(job).execute(&mut *tx).await.map_err(|_|ApiError::Internal)?;
    sqlx::query("INSERT INTO create_new_char_set_level(class_type,beginner_type,slot_id,item_id,item_duration,item_count,item_flag,item_expire_time) VALUES($1,$2,$3,$4,$5,$6,0,0) ON CONFLICT(class_type,beginner_type,slot_id) DO UPDATE SET item_id=EXCLUDED.item_id,item_duration=EXCLUDED.item_duration,item_count=EXCLUDED.item_count,item_flag=0,item_expire_time=0").bind(class).bind(job).bind(p.slot).bind(p.item_id).bind(p.durability).bind(if p.item_id==0{0}else{p.count.max(1)}).execute(&mut *tx).await.map_err(|_|ApiError::Internal)?;
    audit(
        &mut tx,
        &s.actor,
        "starting_item.set",
        &format!("{class}:{job}:{}", p.slot),
        json!({"item_id":p.item_id,"count":p.count}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct Control {
    kind: String,
    action: String,
    zone: Option<u8>,
}
async fn control(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<Control>,
) -> Result<Json<Value>, ApiError> {
    let (_, s) = auth(&headers, true)?;
    let mut detail = json!({"kind":p.kind,"action":p.action,"zone":p.zone});
    match (p.kind.as_str(), p.action.as_str()) {
        ("war", "start") => {
            let z = p
                .zone
                .filter(|z| (1..=6).contains(z))
                .ok_or(ApiError::BadRequest)?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i32;
            let opened = app.world.update_battle_state(|state| {
                if state.is_war_open() {
                    crate::systems::war::reset_battle_zone(state);
                }
                crate::systems::war::battle_zone_open(
                    state,
                    crate::systems::war::BATTLEZONE_OPEN,
                    z,
                    now,
                )
            });
            if !opened {
                return Err(ApiError::Conflict);
            };
            app.world.change_ability_all_npcs(true);
            crate::systems::war::broadcast_war_announcement(
                &app.world,
                "The war zone has opened!",
                None,
            );
            detail["zone_id"] = json!(app.world.get_battle_state().battle_zone_id());
        }
        ("war", "stop") => {
            let prev = app
                .world
                .update_battle_state(crate::systems::war::battle_zone_close);
            if prev == crate::systems::war::NO_BATTLE {
                return Err(ApiError::Conflict);
            };
            app.world.change_ability_all_npcs(false);
            crate::systems::war::broadcast_war_announcement(&app.world, "The war has ended!", None);
        }
        ("event", "start") => {
            let index = p.zone.filter(|z| *z <= 2).ok_or(ApiError::BadRequest)?;
            let event_type = match index {
                0 => TempleEventType::BorderDefenceWar,
                1 => TempleEventType::ChaosDungeon,
                _ => TempleEventType::JuraidMountain,
            };
            let opts = app.world.event_room_manager.vroom_opts.read()[index as usize]
                .clone()
                .unwrap_or(VroomOpt {
                    name: format!("Event {index}"),
                    sign: 5,
                    play: 30,
                    attack_open: 5,
                    attack_close: 30,
                    finish: 60,
                });
            let opened = event_system::open_virtual_event(
                &app.world.event_room_manager,
                &EventOpenParams {
                    vroom_index: index,
                    event_type,
                    vroom_opts: opts,
                    is_automatic: false,
                    min_level: 0,
                    max_level: 0,
                    req_loyalty: 0,
                    req_money: 0,
                },
            );
            if !opened {
                return Err(ApiError::Conflict);
            };
            detail["event_type"] = json!(event_type as i16);
        }
        ("event", "stop") => {
            if !event_system::manual_close(&app.world.event_room_manager) {
                return Err(ApiError::Conflict);
            };
        }
        _ => return Err(ApiError::BadRequest),
    };
    let mut tx = app.pool.begin().await.map_err(|_| ApiError::Internal)?;
    audit(
        &mut tx,
        &s.actor,
        "control.execute",
        &p.kind,
        detail,
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(Json(json!({"ok":true})))
}

async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &str,
    action: &str,
    target: &str,
    details: Value,
    ip: IpAddr,
) -> Result<(), ApiError> {
    sqlx::query("INSERT INTO admin_audit_log(actor,action,target,details,remote_ip) VALUES($1,$2,$3,$4,$5::inet)").bind(actor).bind(action).bind(target).bind(details).bind(ip.to_string()).execute(&mut **tx).await.map_err(|_|ApiError::Internal)?;
    Ok(())
}

#[derive(Debug)]
enum ApiError {
    Unauthorized,
    Forbidden,
    TooManyRequests,
    NotFound,
    Conflict,
    BadRequest,
    Internal,
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Kimlik doğrulanamadı"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "İstek doğrulanamadı"),
            Self::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "Çok fazla giriş denemesi"),
            Self::NotFound => (StatusCode::NOT_FOUND, "Kayıt bulunamadı"),
            Self::Conflict => (
                StatusCode::CONFLICT,
                "Çevrimiçi karakter düzenlenemez veya işlem çakışıyor",
            ),
            Self::BadRequest => (StatusCode::BAD_REQUEST, "Geçersiz değer"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "İşlem tamamlanamadı"),
        };
        (status, Json(json!({"error":msg}))).into_response()
    }
}
