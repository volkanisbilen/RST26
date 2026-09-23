//! Server-hosted WebView2 Power Up Store.
//!
//! The browser never supplies a trusted price, item ID or account ID. A short-
//! lived token is bound to the live game session and every purchase is checked
//! against the catalog already loaded from PostgreSQL.

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use dashmap::DashMap;
use ko_db::DbPool;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::handler::knight_cash;
use crate::world::WorldState;
use crate::zone::SessionId;

const SESSION_TTL: Duration = Duration::from_secs(10 * 60);
const STORE_HTML: &str = include_str!("../assets/pus/index.html");

#[derive(Clone)]
struct WebSession {
    sid: SessionId,
    account_id: String,
    character_name: String,
    remote_ip: IpAddr,
    expires_at: Instant,
}

static SESSIONS: OnceLock<DashMap<String, WebSession>> = OnceLock::new();
static PURCHASE_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn sessions() -> &'static DashMap<String, WebSession> {
    SESSIONS.get_or_init(DashMap::new)
}

pub fn create_session(
    sid: SessionId,
    account_id: String,
    character_name: String,
    remote_ip: IpAddr,
) -> String {
    sessions().retain(|_, value| value.expires_at > Instant::now() && value.sid != sid);
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    sessions().insert(
        token.clone(),
        WebSession {
            sid,
            account_id,
            character_name,
            remote_ip,
            expires_at: Instant::now() + SESSION_TTL,
        },
    );
    token
}

#[derive(Clone)]
struct AppState {
    world: Arc<WorldState>,
    pool: DbPool,
}

pub fn start(
    world: Arc<WorldState>,
    pool: DbPool,
    bind_addr: String,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let state = AppState { world, pool };
        let app = Router::new()
            .route("/pus/", get(index).post(native_index))
            .route("/pus/api/session", get(session_info))
            .route("/pus/api/purchase", post(purchase))
            .with_state(state);
        match tokio::net::TcpListener::bind(&bind_addr).await {
            Ok(listener) => {
                info!(address = %bind_addr, "PUS WebView2 HTTP listener ready");
                if let Err(error) = axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                {
                    warn!(%error, "PUS web service stopped");
                }
            }
            Err(error) => warn!(address = %bind_addr, %error, "PUS web service could not bind"),
        }
    })
}

async fn index() -> Response {
    store_response(STORE_HTML.to_string())
}

/// Entry point used by the unmodified 2625 PUS flow. The client posts its
/// native mall authentication payload to the URL embedded in KnightOnline.exe.
/// We bind that request to the pending game session by source IP and by the
/// account/character identifier already present in the native payload, then
/// inject only the opaque short-lived token into the page.
async fn native_index(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    body: String,
) -> Result<Response, ApiError> {
    let now = Instant::now();
    let matched = sessions().iter().find_map(|entry| {
        let session = entry.value();
        let identity_matches =
            body.contains(&session.account_id) || body.contains(&session.character_name);
        (session.expires_at > now
            && session.remote_ip == peer.ip()
            && identity_matches
            && state.world.is_store_open(session.sid))
        .then(|| entry.key().clone())
    });
    let token = matched.ok_or(ApiError::Unauthorized)?;
    let bootstrap = format!("<script>window.__PUS_TOKEN__='{token}';</script>");
    Ok(store_response(STORE_HTML.replacen(
        "</head>",
        &format!("{bootstrap}</head>"),
        1,
    )))
}

fn store_response(html: String) -> Response {
    (
        [(header::CACHE_CONTROL, "no-store"), (header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'")],
        Html(html),
    ).into_response()
}

fn authenticated(token: &str, world: &WorldState) -> Result<WebSession, ApiError> {
    let entry = sessions().get(token).ok_or(ApiError::Unauthorized)?;
    let session = entry.clone();
    drop(entry);
    if session.expires_at <= Instant::now() || !world.is_store_open(session.sid) {
        sessions().remove(token);
        return Err(ApiError::Unauthorized);
    }
    let live_account = world
        .with_session(session.sid, |h| h.account_id.clone())
        .unwrap_or_default();
    if live_account != session.account_id {
        sessions().remove(token);
        return Err(ApiError::Unauthorized);
    }
    Ok(session)
}

#[derive(Deserialize)]
struct TokenQuery {
    token: String,
}

#[derive(Serialize)]
struct CategoryDto {
    id: i16,
    name: String,
    description: String,
}

#[derive(Serialize)]
struct ItemDto {
    id: i32,
    item_id: i32,
    name: String,
    title: String,
    description: String,
    price: i32,
    price_type: i16,
    quantity: i32,
    category: i16,
}

#[derive(Serialize)]
struct SessionDto {
    character: String,
    knight_cash: u32,
    bonus_point: u32,
    categories: Vec<CategoryDto>,
    items: Vec<ItemDto>,
}

async fn session_info(
    State(state): State<AppState>,
    Query(query): Query<TokenQuery>,
) -> Result<Json<SessionDto>, ApiError> {
    let auth = authenticated(&query.token, &state.world)?;
    let repo = ko_db::repositories::cash_shop::CashShopRepository::new(&state.pool);
    let category_rows = repo
        .load_all_categories()
        .await
        .map_err(|_| ApiError::Internal)?;
    let item_rows = repo
        .load_all_items()
        .await
        .map_err(|_| ApiError::Internal)?;
    let categories = category_rows
        .into_iter()
        .map(|c| CategoryDto {
            id: c.category_id,
            name: c.category_name,
            description: c.description,
        })
        .collect();
    let items = item_rows
        .into_iter()
        .filter_map(|i| {
            let price = i.price.unwrap_or(0);
            (price >= 0).then(|| ItemDto {
                id: i.id,
                item_id: i.item_id,
                name: i.item_name.unwrap_or_else(|| format!("Eşya {}", i.item_id)),
                title: i.item_title.unwrap_or_default(),
                description: i.item_desc,
                price,
                price_type: i.price_type,
                quantity: i.buy_count,
                category: i.category,
            })
        })
        .collect();
    Ok(Json(SessionDto {
        character: auth.character_name,
        knight_cash: state.world.get_knight_cash(auth.sid),
        bonus_point: state.world.get_tl_balance(auth.sid),
        categories,
        items,
    }))
}

#[derive(Deserialize)]
struct PurchaseRequest {
    token: String,
    listing_id: i32,
    count: u8,
}

#[derive(Serialize)]
struct PurchaseResponse {
    ok: bool,
    message: String,
    knight_cash: u32,
    bonus_point: u32,
}

async fn purchase(
    State(state): State<AppState>,
    Json(request): Json<PurchaseRequest>,
) -> Result<Json<PurchaseResponse>, ApiError> {
    // Balance verification, deduction and delivery must be one in-process
    // operation. This prevents rapid duplicate WebView2 requests from spending
    // the same balance twice.
    let _purchase_guard = PURCHASE_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await;
    let auth = authenticated(&request.token, &state.world)?;
    if request.count == 0 || request.count > 10 {
        return Err(ApiError::BadRequest("Adet 1 ile 10 arasında olmalıdır."));
    }
    if state.world.is_player_dead(auth.sid)
        || state.world.is_trading(auth.sid)
        || state.world.is_merchanting(auth.sid)
    {
        return Err(ApiError::BadRequest("Bu durumdayken mağaza kullanılamaz."));
    }
    let repo = ko_db::repositories::cash_shop::CashShopRepository::new(&state.pool);
    let categories = repo
        .load_all_categories()
        .await
        .map_err(|_| ApiError::Internal)?;
    let listing = repo
        .load_all_items()
        .await
        .map_err(|_| ApiError::Internal)?
        .into_iter()
        .find(|item| item.id == request.listing_id)
        .ok_or(ApiError::BadRequest("Ürün artık satışta değil."))?;
    if !categories
        .iter()
        .any(|category| category.category_id == listing.category)
    {
        return Err(ApiError::BadRequest("Ürün kategorisi aktif değil."));
    }
    let unit_price = listing.price.unwrap_or(-1);
    if unit_price < 0 || listing.buy_count <= 0 || listing.buy_count > u16::MAX as i32 {
        return Err(ApiError::BadRequest("Ürün bilgisi geçersiz."));
    }
    let total = unit_price
        .checked_mul(request.count as i32)
        .ok_or(ApiError::BadRequest("Tutar geçersiz."))?;
    if state.world.get_item(listing.item_id as u32).is_none() {
        return Err(ApiError::BadRequest("Eşya oyun verisinde bulunamadı."));
    }
    if state.world.count_free_slots(auth.sid) < request.count {
        return Err(ApiError::BadRequest("Envanterde yeterli boş yer yok."));
    }

    let deducted = if listing.price_type == 0 {
        deduct_balance(&state, &auth, total, true).await?
    } else if listing.price_type == 1 {
        deduct_balance(&state, &auth, total, false).await?
    } else {
        return Err(ApiError::BadRequest("Para birimi geçersiz."));
    };
    if !deducted {
        return Err(ApiError::BadRequest("Yetersiz bakiye."));
    }

    let mut delivered = 0u8;
    for _ in 0..request.count {
        if state
            .world
            .give_item(auth.sid, listing.item_id as u32, listing.buy_count as u16)
        {
            delivered += 1;
        }
    }
    if delivered != request.count {
        // A concurrent inventory change can still make delivery fail. Refund
        // the undelivered part immediately instead of charging for it.
        let refund = unit_price * (request.count - delivered) as i32;
        if refund > 0 {
            let _ = credit_balance(&state, &auth, refund, listing.price_type == 0).await;
        }
    }
    if delivered == 0 {
        return Err(ApiError::BadRequest(
            "Eşya teslim edilemedi; ücret iade edildi.",
        ));
    }
    Ok(Json(PurchaseResponse {
        ok: true,
        message: format!(
            "{} envantere gönderildi.",
            listing.item_name.unwrap_or_default()
        ),
        knight_cash: state.world.get_knight_cash(auth.sid),
        bonus_point: state.world.get_tl_balance(auth.sid),
    }))
}

async fn deduct_balance(
    state: &AppState,
    auth: &WebSession,
    amount: i32,
    kc: bool,
) -> Result<bool, ApiError> {
    let current = if kc {
        state.world.get_knight_cash(auth.sid)
    } else {
        state.world.get_tl_balance(auth.sid)
    };
    if current < amount as u32 {
        return Ok(false);
    }
    let new_kc = if kc {
        current - amount as u32
    } else {
        state.world.get_knight_cash(auth.sid)
    };
    let new_tl = if kc {
        state.world.get_tl_balance(auth.sid)
    } else {
        current - amount as u32
    };
    let repo = ko_db::repositories::cash_shop::CashShopRepository::new(&state.pool);
    repo.update_kc_balances(&auth.account_id, new_kc as i32, new_tl as i32)
        .await
        .map_err(|_| ApiError::Internal)?;
    state.world.set_kc_balance(auth.sid, new_kc, new_tl);
    state.world.send_to_session(
        auth.sid,
        &knight_cash::build_cashchange_packet(new_kc, new_tl),
    );
    Ok(true)
}

async fn credit_balance(
    state: &AppState,
    auth: &WebSession,
    amount: i32,
    kc: bool,
) -> Result<(), ApiError> {
    let new_kc =
        state
            .world
            .get_knight_cash(auth.sid)
            .saturating_add(if kc { amount as u32 } else { 0 });
    let new_tl =
        state
            .world
            .get_tl_balance(auth.sid)
            .saturating_add(if kc { 0 } else { amount as u32 });
    let repo = ko_db::repositories::cash_shop::CashShopRepository::new(&state.pool);
    repo.update_kc_balances(&auth.account_id, new_kc as i32, new_tl as i32)
        .await
        .map_err(|_| ApiError::Internal)?;
    state.world.set_kc_balance(auth.sid, new_kc, new_tl);
    state.world.send_to_session(
        auth.sid,
        &knight_cash::build_cashchange_packet(new_kc, new_tl),
    );
    Ok(())
}

enum ApiError {
    Unauthorized,
    BadRequest(&'static str),
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Mağaza oturumu geçersiz veya süresi dolmuş.",
            ),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "İşlem tamamlanamadı."),
        };
        (
            status,
            Json(serde_json::json!({"ok": false, "message": message})),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_sessions_use_unique_192_bit_tokens() {
        let first = create_session(
            61,
            "account-a".into(),
            "character-a".into(),
            "127.0.0.1".parse().unwrap(),
        );
        let second = create_session(
            62,
            "account-b".into(),
            "character-b".into(),
            "127.0.0.2".parse().unwrap(),
        );

        assert_eq!(first.len(), 48);
        assert_eq!(second.len(), 48);
        assert_ne!(first, second);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn embedded_store_contains_dynamic_api_hooks() {
        assert!(STORE_HTML.contains("/pus/api/session"));
        assert!(STORE_HTML.contains("/pus/api/purchase"));
        assert!(STORE_HTML.contains("Power Up Store"));
        assert!(STORE_HTML.contains("Satın Al"));
    }
}
