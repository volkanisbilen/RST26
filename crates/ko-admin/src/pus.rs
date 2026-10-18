//! Power Up Store ESN codes (KC / TL e-pins): batch generation, usage tracking, KC/TL ledger.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use rand::Rng;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::legacy::{audit, auth, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/pus/summary", get(summary))
        .route("/jstkoadminpanel/api/pus/generate", post(generate))
        .route("/jstkoadminpanel/api/pus/codes", get(codes))
        .route("/jstkoadminpanel/api/pus/batch/{id}/export", get(export))
        .route("/jstkoadminpanel/api/pus/batch/{id}/delete", post(delete_batch))
        .route("/jstkoadminpanel/api/pus/code/{id}", post(update_code))
        .route("/jstkoadminpanel/api/pus/ledger", get(ledger))
        .route("/jstkoadminpanel/api/pus/attempts", get(attempts))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

/// no 0/O/1/I/L: the code is typed by hand from a chat or a shop page
const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

fn new_code() -> String {
    let mut rng = rand::thread_rng();
    let mut s = String::with_capacity(19);
    for g in 0..4 {
        if g > 0 {
            s.push('-');
        }
        for _ in 0..4 {
            s.push(ALPHABET[rng.gen_range(0..ALPHABET.len())] as char);
        }
    }
    s
}

async fn summary(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT jsonb_build_object(
            'total', (SELECT COUNT(*) FROM pus_code),
            'used', (SELECT COUNT(*) FROM pus_code WHERE used_at IS NOT NULL),
            'kc_issued', COALESCE((SELECT SUM(kc) FROM pus_code),0),
            'tl_issued', COALESCE((SELECT SUM(tl) FROM pus_code),0),
            'kc_used', COALESCE((SELECT SUM(kc) FROM pus_code WHERE used_at IS NOT NULL),0),
            'tl_used', COALESCE((SELECT SUM(tl) FROM pus_code WHERE used_at IS NOT NULL),0),
            'batches', COALESCE((SELECT jsonb_agg(jsonb_build_object(
                'id',b.id,'label',b.label,'kc',b.kc,'tl',b.tl,'quantity',b.quantity,'note',b.note,
                'created_by',b.created_by,'created_at',b.created_at,'expires_at',b.expires_at,
                'used',(SELECT COUNT(*) FROM pus_code c WHERE c.batch_id=b.id AND c.used_at IS NOT NULL))
                ORDER BY b.id DESC) FROM (SELECT * FROM pus_code_batch ORDER BY id DESC LIMIT 100) b),'[]'::jsonb)
        )"#,
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("pus summary"))?;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct GenIn {
    #[serde(default)]
    label: String,
    #[serde(default)]
    kc: i32,
    #[serde(default)]
    tl: i32,
    quantity: i32,
    #[serde(default)]
    note: String,
    /// validity in days, 0 = never expires
    #[serde(default)]
    expires_days: i32,
}

async fn generate(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<GenIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !(1..=2000).contains(&p.quantity)
        || !(0..=10_000_000).contains(&p.kc)
        || !(0..=10_000_000).contains(&p.tl)
        || (p.kc == 0 && p.tl == 0)
        || p.label.chars().count() > 80
        || p.note.chars().count() > 200
        || !(0..=3650).contains(&p.expires_days)
    {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let batch: i64 = sqlx::query_scalar(
        "INSERT INTO pus_code_batch(label,kc,tl,quantity,note,created_by,expires_at) \
         VALUES($1,$2,$3,$4,$5,$6, CASE WHEN $7 > 0 THEN NOW() + make_interval(days => $7) END) RETURNING id",
    )
    .bind(&p.label)
    .bind(p.kc)
    .bind(p.tl)
    .bind(p.quantity)
    .bind(&p.note)
    .bind(&s.actor)
    .bind(p.expires_days)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal("batch"))?;
    let mut made = 0;
    while made < p.quantity {
        let r = sqlx::query(
            "INSERT INTO pus_code(code,batch_id,kc,tl,note,expires_at) \
             SELECT $1,$2,$3,$4,$5, expires_at FROM pus_code_batch WHERE id=$2 ON CONFLICT (code) DO NOTHING",
        )
        .bind(new_code())
        .bind(batch)
        .bind(p.kc)
        .bind(p.tl)
        .bind(&p.note)
        .execute(&mut *tx)
        .await
        .map_err(internal("code"))?;
        made += r.rows_affected() as i32;
    }
    audit(
        &mut tx,
        &s.actor,
        "pus.generate",
        &batch.to_string(),
        json!({"kc":p.kc,"tl":p.tl,"quantity":p.quantity,"label":p.label}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"batch":batch})))
}

#[derive(Deserialize)]
struct CodesQ {
    batch: Option<i64>,
    /// all | unused | used | disabled
    state: Option<String>,
    q: Option<String>,
    #[serde(default)]
    offset: i64,
}

const CODE_FILTER: &str = r#"($1::bigint IS NULL OR c.batch_id=$1)
    AND ($2='' OR ($2='unused' AND c.used_at IS NULL AND NOT c.disabled) OR ($2='used' AND c.used_at IS NOT NULL) OR ($2='disabled' AND c.disabled))
    AND ($3='%%' OR c.code ILIKE $3 OR c.used_account ILIKE $3 OR c.used_char ILIKE $3 OR c.note ILIKE $3)"#;

async fn codes(headers: HeaderMap, State(app): State<App>, Query(q): Query<CodesQ>) -> R {
    auth(&headers, false)?;
    let state = q.state.unwrap_or_default();
    let like = q.q.unwrap_or_default().trim().to_string();
    if like.len() > 40 || q.offset < 0 {
        return Err(ApiError::BadRequest);
    }
    let pat = format!("%{}%", like.replace(['%', '_'], ""));
    let sql = format!(
        r#"SELECT jsonb_build_object(
            'total', (SELECT COUNT(*) FROM pus_code c WHERE {f}),
            'rows', COALESCE((SELECT jsonb_agg(r) FROM (
                SELECT c.id,c.code,c.batch_id,b.label,c.kc,c.tl,c.note,c.disabled,c.expires_at,c.used_at,c.used_account,c.used_char,c.used_ip
                FROM pus_code c JOIN pus_code_batch b ON b.id=c.batch_id
                WHERE {f}
                ORDER BY c.used_at DESC NULLS LAST, c.id DESC LIMIT 100 OFFSET $4) r),'[]'::jsonb))"#,
        f = CODE_FILTER
    );
    let v: Value = sqlx::query_scalar(&sql)
        .bind(q.batch)
        .bind(&state)
        .bind(&pat)
        .bind(q.offset)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("pus codes"))?;
    Ok(Json(v))
}

async fn export(headers: HeaderMap, State(app): State<App>, Path(id): Path<i64>) -> R {
    auth(&headers, false)?;
    let rows: Vec<(String, i32, i32)> =
        sqlx::query_as("SELECT code,kc,tl FROM pus_code WHERE batch_id=$1 ORDER BY id")
            .bind(id)
            .fetch_all(&app.pool)
            .await
            .map_err(internal("pus export"))?;
    Ok(Json(json!({
        "codes": rows.into_iter().map(|(c, k, t)| json!({"code":c,"kc":k,"tl":t})).collect::<Vec<_>>()
    })))
}

async fn delete_batch(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i64>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    // a batch with redeemed codes stays: it is the usage record
    let used: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pus_code WHERE batch_id=$1 AND used_at IS NOT NULL")
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(internal("used"))?;
    if used > 0 {
        return Err(ApiError::Conflict);
    }
    sqlx::query("DELETE FROM pus_code_batch WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal("delete"))?;
    audit(&mut tx, &s.actor, "pus.batch.delete", &id.to_string(), json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct CodeIn {
    disabled: Option<bool>,
    note: Option<String>,
}

async fn update_code(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i64>,
    Json(p): Json<CodeIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.note.as_ref().map(|n| n.chars().count() > 200).unwrap_or(false) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let n = sqlx::query("UPDATE pus_code SET disabled = COALESCE($2, disabled), note = COALESCE($3, note) WHERE id=$1")
        .bind(id)
        .bind(p.disabled)
        .bind(p.note.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(internal("update"))?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    audit(
        &mut tx,
        &s.actor,
        "pus.code.update",
        &id.to_string(),
        json!({"disabled":p.disabled,"note":p.note}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
struct AccQ {
    account: Option<String>,
    #[serde(default)]
    offset: i64,
}

async fn ledger(headers: HeaderMap, State(app): State<App>, Query(q): Query<AccQ>) -> R {
    auth(&headers, false)?;
    let acc = q.account.unwrap_or_default().trim().to_string();
    if acc.len() > 40 || q.offset < 0 {
        return Err(ApiError::BadRequest);
    }
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(r),'[]'::jsonb) FROM (
            SELECT id,at,account,char_name,kc_delta,tl_delta,kc_after,tl_after,reason,ref FROM kc_ledger
            WHERE $1='' OR account ILIKE $1 OR char_name ILIKE $1 ORDER BY id DESC LIMIT 100 OFFSET $2) r"#,
    )
    .bind(&acc)
    .bind(q.offset)
    .fetch_one(&app.pool)
    .await
    .map_err(internal("pus ledger"))?;
    Ok(Json(json!({"rows":v})))
}

async fn attempts(headers: HeaderMap, State(app): State<App>, Query(q): Query<AccQ>) -> R {
    auth(&headers, false)?;
    let acc = q.account.unwrap_or_default().trim().to_string();
    if acc.len() > 40 || q.offset < 0 {
        return Err(ApiError::BadRequest);
    }
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(r),'[]'::jsonb) FROM (
            SELECT id,at,account,char_name,ip,code,result FROM pus_code_attempt
            WHERE $1='' OR account ILIKE $1 OR char_name ILIKE $1 OR ip=$1 ORDER BY id DESC LIMIT 100 OFFSET $2) r"#,
    )
    .bind(&acc)
    .bind(q.offset)
    .fetch_one(&app.pool)
    .await
    .map_err(internal("pus attempts"))?;
    Ok(Json(json!({"rows":v})))
}
