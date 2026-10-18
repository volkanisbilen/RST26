//! "Veritabanı Tabloları": generic browser/editor for any base table of the `public` schema.
//!
//! Safety model: table and column identifiers never come from user input directly -- they are
//! looked up in `information_schema` (unknown names are rejected) and only then quoted into the
//! SQL text. All values are bound as text parameters and cast server-side by the column's
//! `data_type` (`$1::text::int4`), one statement at a time. Updates/deletes always target the
//! primary key; tables without a PK are read-only. Password/token-like columns are masked and
//! never read, filtered, sorted or written; admin/session tables are hidden entirely.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sqlx::{postgres::PgRow, Row};

use crate::legacy::{audit, auth, queue_command, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/dbb/tables", get(list_tables))
        .route("/jstkoadminpanel/api/dbb/table/{table}/meta", get(table_meta))
        .route("/jstkoadminpanel/api/dbb/table/{table}/rows", get(table_rows))
        .route("/jstkoadminpanel/api/dbb/table/{table}/insert", post(insert_row))
        .route("/jstkoadminpanel/api/dbb/table/{table}/update", post(update_row))
        .route("/jstkoadminpanel/api/dbb/table/{table}/delete", post(delete_row))
        .route("/jstkoadminpanel/api/dbb/table/{table}/reload", post(reload_table))
}

// ───────────────────────── errors ─────────────────────────

enum DbbErr {
    Api(ApiError),
    Msg(StatusCode, String),
}
impl From<ApiError> for DbbErr {
    fn from(e: ApiError) -> Self {
        DbbErr::Api(e)
    }
}
impl IntoResponse for DbbErr {
    fn into_response(self) -> Response {
        match self {
            DbbErr::Api(e) => e.into_response(),
            DbbErr::Msg(c, m) => (c, Json(json!({ "error": m }))).into_response(),
        }
    }
}
type R = Result<Json<Value>, DbbErr>;

fn bad(msg: impl Into<String>) -> DbbErr {
    DbbErr::Msg(StatusCode::BAD_REQUEST, msg.into())
}
/// Surface the Postgres message (admin-only page) so the owner sees e.g. FK / cast errors.
fn db_err(what: &'static str) -> impl Fn(sqlx::Error) -> DbbErr {
    move |e| {
        tracing::warn!(error = %e, "dbb {what}");
        let msg = match &e {
            sqlx::Error::Database(d) => d.message().to_string(),
            _ => "Veritabanı hatası".to_string(),
        };
        DbbErr::Msg(StatusCode::BAD_REQUEST, msg)
    }
}

// ───────────────────────── policy ─────────────────────────

const MAX_CELL: i32 = 4000;
const MAX_VALUE_LEN: usize = 100_000;
const MAX_COLS_WRITE: usize = 300;
const EXACT_COUNT_BELOW: i64 = 200_000;

pub fn table_hidden(name: &str) -> bool {
    name.starts_with("admin_")
        || name.starts_with("_sqlx")
        || name == "web_email_verify"
        || name == "web_password_reset"
}

pub fn col_sensitive(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["passw", "pwd", "secret", "token", "hash", "otp", "salt"].iter().any(|k| n.contains(k))
}

/// `reload` admin_command target for tables the game keeps in memory (see admin_queue.rs).
pub fn reload_target(table: &str) -> Option<&'static str> {
    Some(match table {
        "monster_item" => "monster_drop_tables",
        "make_item_group" => "make_item_groups",
        "quest_helper" | "quest_monster" | "quest_menu" | "quest_talk" | "quest_skills_closed_check"
        | "quest_skills_closed_data" | "quest_skills_open_set_up" => "quest_tables",
        "event_schedule_main_list" | "event_schedule_day_list" | "event_opt_vroom" | "event_opt_ft"
        | "event_room_play_timer" | "event_trigger" | "event_timer_show_list" => "event_schedule",
        "event_rewards" => "event_rewards",
        "event_chaos_rewards" => "chaos_rewards",
        "daily_quests" => "daily_quests",
        "cindwar_reward" | "cindwar_reward_item" => "cinderella",
        "collection_race_reward" | "collection_race_settings" => "collection_race",
        "event_reward_rule" => "reward_rules",
        "npc_spawn" => "npc_spawn",
        "send_messages" => "send_messages",
        "item_exchange" => "item_exchange",
        "pus_category" | "pus_items" => "pus",
        "zone_kill_reward" | "zone_online_reward" => "zone_rewards",
        _ => return None,
    })
}

// ───────────────────────── metadata ─────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Int,
    Num,
    Bool,
    Text,
    Time,
    Json,
    Bytea,
    Other,
}

/// (kind, SQL cast type) for an `information_schema` data_type; None cast = not editable.
pub fn kind_of(dt: &str) -> (Kind, Option<&'static str>) {
    match dt {
        "smallint" => (Kind::Int, Some("int2")),
        "integer" => (Kind::Int, Some("int4")),
        "bigint" => (Kind::Int, Some("int8")),
        "numeric" => (Kind::Num, Some("numeric")),
        "real" => (Kind::Num, Some("float4")),
        "double precision" => (Kind::Num, Some("float8")),
        "boolean" => (Kind::Bool, Some("bool")),
        "text" | "character varying" | "character" => (Kind::Text, Some("text")),
        "timestamp without time zone" => (Kind::Time, Some("timestamp")),
        "timestamp with time zone" => (Kind::Time, Some("timestamptz")),
        "date" => (Kind::Time, Some("date")),
        "time without time zone" => (Kind::Time, Some("time")),
        "jsonb" => (Kind::Json, Some("jsonb")),
        "json" => (Kind::Json, Some("json")),
        "bytea" => (Kind::Bytea, None),
        _ => (Kind::Other, None),
    }
}

#[derive(Clone, Debug)]
pub struct Col {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub max_len: Option<i32>,
    pub kind: Kind,
    pub cast: Option<&'static str>,
    pub sensitive: bool,
    pub generated: bool,
}
impl Col {
    pub fn editable(&self) -> bool {
        self.cast.is_some() && !self.sensitive && !self.generated
    }
    pub fn searchable(&self) -> bool {
        !self.sensitive && self.kind != Kind::Bytea
    }
}

#[derive(Clone, Debug)]
pub struct Meta {
    pub table: String,
    pub cols: Vec<Col>,
    pub pk: Vec<String>,
    pub readonly: Option<String>,
}
impl Meta {
    pub fn col(&self, name: &str) -> Option<&Col> {
        self.cols.iter().find(|c| c.name == name)
    }
}

pub fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Decide read-only state from the (already looked-up) columns and pk.
pub fn readonly_reason(cols: &[Col], pk: &[String]) -> Option<String> {
    if pk.is_empty() {
        return Some("Bu tabloda birincil anahtar (PK) yok; satırlar güvenle hedeflenemediği için salt okunur.".into());
    }
    for k in pk {
        match cols.iter().find(|c| &c.name == k) {
            Some(c) if c.cast.is_some() && !c.sensitive => {}
            _ => return Some(format!("PK sütunu '{k}' desteklenmeyen tipte; tablo salt okunur.")),
        }
    }
    None
}

async fn load_meta(app: &App, table: &str) -> Result<Meta, DbbErr> {
    if table_hidden(table) || table.len() > 128 {
        return Err(DbbErr::Api(ApiError::NotFound));
    }
    let rows = sqlx::query(
        "SELECT c.column_name::text, c.data_type::text, c.is_nullable::text, c.column_default::text, \
         c.character_maximum_length::int4, c.is_generated::text, COALESCE(c.identity_generation::text,'') \
         FROM information_schema.columns c JOIN information_schema.tables t \
           ON t.table_schema=c.table_schema AND t.table_name=c.table_name AND t.table_type='BASE TABLE' \
         WHERE c.table_schema='public' AND c.table_name::text=$1 \
         ORDER BY c.ordinal_position",
    )
    .bind(table)
    .fetch_all(&app.pool)
    .await
    .map_err(db_err("meta"))?;
    if rows.is_empty() {
        return Err(DbbErr::Api(ApiError::NotFound));
    }
    let mut cols = Vec::with_capacity(rows.len());
    for r in &rows {
        let name: String = r.try_get(0).map_err(db_err("meta"))?;
        let data_type: String = r.try_get(1).map_err(db_err("meta"))?;
        let nullable: String = r.try_get(2).map_err(db_err("meta"))?;
        let default: Option<String> = r.try_get(3).map_err(db_err("meta"))?;
        let max_len: Option<i32> = r.try_get(4).map_err(db_err("meta"))?;
        let is_gen: String = r.try_get(5).map_err(db_err("meta"))?;
        let ident: String = r.try_get(6).map_err(db_err("meta"))?;
        let (kind, cast) = kind_of(&data_type);
        cols.push(Col {
            sensitive: col_sensitive(&name),
            generated: is_gen == "ALWAYS" || ident == "ALWAYS",
            name,
            data_type,
            nullable: nullable == "YES",
            default,
            max_len,
            kind,
            cast,
        });
    }
    let pk: Vec<String> = sqlx::query_scalar(
        "SELECT kcu.column_name::text FROM information_schema.table_constraints tc \
         JOIN information_schema.key_column_usage kcu ON kcu.constraint_name=tc.constraint_name \
          AND kcu.constraint_schema=tc.constraint_schema AND kcu.table_name=tc.table_name \
         WHERE tc.table_schema='public' AND tc.table_name::text=$1 AND tc.constraint_type='PRIMARY KEY' \
         ORDER BY kcu.ordinal_position",
    )
    .bind(table)
    .fetch_all(&app.pool)
    .await
    .map_err(db_err("meta pk"))?;
    let readonly = readonly_reason(&cols, &pk);
    Ok(Meta { table: table.to_string(), cols, pk, readonly })
}

// ───────────────────────── SQL builders (pure) ─────────────────────────

/// SELECT list: every column as text (truncated), bytea as hex prefix, sensitive columns masked.
pub fn select_list(meta: &Meta) -> String {
    meta.cols
        .iter()
        .map(|c| {
            let q = quote_ident(&c.name);
            if c.sensitive {
                format!("NULL::text AS {q}")
            } else if c.kind == Kind::Bytea {
                format!("encode(substring({q} from 1 for 64),'hex') AS {q}")
            } else {
                format!("left({q}::text, {MAX_CELL}) AS {q}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn like_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for ch in s.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            o.push('\\');
        }
        o.push(ch);
    }
    o
}

/// (page, size, offset) with clamps: size 1..=500 (default 50), page >= 1, offset capped.
pub fn clamp_page(page: Option<i64>, size: Option<i64>) -> (i64, i64, i64) {
    let size = size.unwrap_or(50).clamp(1, 500);
    let page = page.unwrap_or(1).clamp(1, 1_000_000);
    let offset = ((page - 1) * size).min(50_000_000);
    (page, size, offset)
}

#[derive(Default, Debug)]
pub struct ListSpec {
    pub sort: Option<String>,
    pub desc: bool,
    pub q: Option<String>,
    pub filters: Vec<(String, String)>,
}

pub struct Built {
    pub where_sql: String,
    pub order_sql: String,
    pub binds: Vec<String>,
}

pub fn build_list(meta: &Meta, spec: &ListSpec) -> Result<Built, String> {
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<String> = Vec::new();
    if let Some(q) = spec.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let cols: Vec<String> = meta
            .cols
            .iter()
            .filter(|c| c.searchable())
            .map(|c| format!("{}::text ILIKE $1", quote_ident(&c.name)))
            .collect();
        if cols.is_empty() {
            return Err("Aranabilir sütun yok".into());
        }
        binds.push(format!("%{}%", like_escape(q)));
        conds.push(format!("({})", cols.join(" OR ")));
    }
    for (name, val) in &spec.filters {
        let val = val.trim();
        if val.is_empty() {
            continue;
        }
        let col = meta.col(name).ok_or_else(|| format!("Bilinmeyen sütun: {name}"))?;
        if !col.searchable() {
            return Err(format!("'{name}' sütunu filtrelenemez"));
        }
        let n = binds.len() + 1;
        if let Some(exact) = val.strip_prefix('=') {
            conds.push(format!("{}::text = ${n}", quote_ident(&col.name)));
            binds.push(exact.to_string());
        } else {
            conds.push(format!("{}::text ILIKE ${n}", quote_ident(&col.name)));
            binds.push(format!("%{}%", like_escape(val)));
        }
    }
    let where_sql = if conds.is_empty() { String::new() } else { format!(" WHERE {}", conds.join(" AND ")) };

    let mut order: Vec<String> = Vec::new();
    if let Some(s) = spec.sort.as_deref().filter(|s| !s.is_empty()) {
        let col = meta.col(s).ok_or_else(|| format!("Bilinmeyen sütun: {s}"))?;
        if !col.searchable() {
            return Err(format!("'{s}' sütunu sıralanamaz"));
        }
        order.push(format!("{} {} NULLS LAST", quote_ident(&col.name), if spec.desc { "DESC" } else { "ASC" }));
    }
    for k in &meta.pk {
        if spec.sort.as_deref() != Some(k.as_str()) {
            order.push(quote_ident(k));
        }
    }
    if meta.pk.is_empty() {
        order.push("ctid".into());
    }
    Ok(Built { where_sql, order_sql: format!(" ORDER BY {}", order.join(", ")), binds })
}

pub fn build_rows_sql(meta: &Meta, b: &Built, limit: i64, offset: i64) -> String {
    format!(
        "SELECT {} FROM {}{}{} LIMIT {} OFFSET {}",
        select_list(meta),
        quote_ident(&meta.table),
        b.where_sql,
        b.order_sql,
        limit,
        offset
    )
}

fn cast_param(i: usize, col: &Col) -> Result<String, String> {
    let c = col.cast.ok_or_else(|| format!("'{}' sütunu düzenlenemez (tip: {})", col.name, col.data_type))?;
    Ok(format!("${i}::text::{c}"))
}

/// Bound values: JSON null -> NULL; empty string on a non-text column -> NULL.
pub fn value_to_bind(col: &Col, v: &Value) -> Result<Option<String>, String> {
    let s = match v {
        Value::Null => return Ok(None),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    };
    if s.len() > MAX_VALUE_LEN {
        return Err(format!("'{}' değeri çok uzun", col.name));
    }
    if s.contains('\0') {
        return Err("NUL karakteri kabul edilmez".into());
    }
    if s.trim().is_empty() && col.kind != Kind::Text {
        return Ok(None);
    }
    Ok(Some(s))
}

fn pk_where(meta: &Meta, pk: &Map<String, Value>, binds: &mut Vec<Option<String>>) -> Result<String, String> {
    if pk.len() != meta.pk.len() {
        return Err("PK değerleri eksik".into());
    }
    let mut parts = Vec::new();
    for k in &meta.pk {
        let col = meta.col(k).ok_or("PK sütunu bulunamadı")?;
        let v = pk.get(k).ok_or_else(|| format!("PK değeri eksik: {k}"))?;
        let b = value_to_bind(col, v)?.ok_or_else(|| format!("PK değeri boş olamaz: {k}"))?;
        binds.push(Some(b));
        parts.push(format!("{} = {}", quote_ident(k), cast_param(binds.len(), col)?));
    }
    Ok(parts.join(" AND "))
}

pub fn build_select_pk(meta: &Meta, pk: &Map<String, Value>) -> Result<(String, Vec<Option<String>>), String> {
    let mut binds = Vec::new();
    let w = pk_where(meta, pk, &mut binds)?;
    Ok((format!("SELECT {} FROM {} WHERE {} FOR UPDATE", select_list(meta), quote_ident(&meta.table), w), binds))
}

pub fn build_update(meta: &Meta, pk: &Map<String, Value>, changes: &Map<String, Value>) -> Result<(String, Vec<Option<String>>), String> {
    if changes.is_empty() {
        return Err("Değişiklik yok".into());
    }
    if changes.len() > MAX_COLS_WRITE {
        return Err("Çok fazla sütun".into());
    }
    let mut binds = Vec::new();
    let mut sets = Vec::new();
    for (name, v) in changes {
        let col = meta.col(name).ok_or_else(|| format!("Bilinmeyen sütun: {name}"))?;
        if !col.editable() {
            return Err(format!("'{name}' sütunu düzenlenemez"));
        }
        binds.push(value_to_bind(col, v)?);
        sets.push(format!("{} = {}", quote_ident(name), cast_param(binds.len(), col)?));
    }
    let w = pk_where(meta, pk, &mut binds)?;
    Ok((format!("UPDATE {} SET {} WHERE {}", quote_ident(&meta.table), sets.join(", "), w), binds))
}

pub fn build_delete(meta: &Meta, pk: &Map<String, Value>) -> Result<(String, Vec<Option<String>>), String> {
    let mut binds = Vec::new();
    let w = pk_where(meta, pk, &mut binds)?;
    Ok((format!("DELETE FROM {} WHERE {}", quote_ident(&meta.table), w), binds))
}

pub fn build_insert(meta: &Meta, values: &Map<String, Value>) -> Result<(String, Vec<Option<String>>), String> {
    if values.len() > MAX_COLS_WRITE {
        return Err("Çok fazla sütun".into());
    }
    let mut binds = Vec::new();
    let mut names = Vec::new();
    let mut params = Vec::new();
    for (name, v) in values {
        let col = meta.col(name).ok_or_else(|| format!("Bilinmeyen sütun: {name}"))?;
        if !col.editable() {
            return Err(format!("'{name}' sütunu yazılamaz"));
        }
        binds.push(value_to_bind(col, v)?);
        names.push(quote_ident(name));
        params.push(cast_param(binds.len(), col)?);
    }
    let sql = if names.is_empty() {
        format!("INSERT INTO {} DEFAULT VALUES RETURNING {}", quote_ident(&meta.table), select_list(meta))
    } else {
        format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
            quote_ident(&meta.table),
            names.join(", "),
            params.join(", "),
            select_list(meta)
        )
    };
    Ok((sql, binds))
}

// ───────────────────────── helpers ─────────────────────────

fn row_values(row: &PgRow, n: usize) -> Vec<Value> {
    (0..n)
        .map(|i| match row.try_get::<Option<String>, _>(i) {
            Ok(Some(s)) => Value::String(s),
            _ => Value::Null,
        })
        .collect()
}

fn obj_of(meta: &Meta, vals: &[Value]) -> Map<String, Value> {
    meta.cols.iter().zip(vals.iter()).map(|(c, v)| (c.name.clone(), v.clone())).collect()
}

fn pk_label(meta: &Meta, pk: &Map<String, Value>) -> String {
    let parts: Vec<String> = meta
        .pk
        .iter()
        .map(|k| match pk.get(k) {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        })
        .collect();
    format!("{}:{}", meta.table, parts.join(","))
}

fn bind_all<'q>(
    mut q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    binds: &'q [Option<String>],
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    for b in binds {
        q = q.bind(b.as_deref());
    }
    q
}

fn require_writable(meta: &Meta) -> Result<(), DbbErr> {
    match &meta.readonly {
        Some(r) => Err(DbbErr::Msg(StatusCode::FORBIDDEN, r.clone())),
        None => Ok(()),
    }
}

fn col_json(c: &Col) -> Value {
    json!({
        "name": c.name, "type": c.data_type, "kind": format!("{:?}", c.kind).to_lowercase(),
        "nullable": c.nullable, "default": c.default, "max_len": c.max_len,
        "editable": c.editable(), "sensitive": c.sensitive, "generated": c.generated,
    })
}

// ───────────────────────── handlers ─────────────────────────

async fn list_tables(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let rows = sqlx::query(
        "SELECT c.relname::text, c.reltuples::bigint, \
         EXISTS(SELECT 1 FROM pg_index i WHERE i.indrelid=c.oid AND i.indisprimary) \
         FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') ORDER BY c.relname",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(db_err("tables"))?;
    let mut out = Vec::new();
    for r in rows {
        let name: String = r.try_get(0).map_err(db_err("tables"))?;
        if table_hidden(&name) {
            continue;
        }
        let est: i64 = r.try_get(1).map_err(db_err("tables"))?;
        let has_pk: bool = r.try_get(2).map_err(db_err("tables"))?;
        out.push(json!({"name": name, "est": est, "pk": has_pk, "reload": reload_target(&name)}));
    }
    Ok(Json(json!({ "tables": out })))
}

async fn table_meta(headers: HeaderMap, State(app): State<App>, Path(table): Path<String>) -> R {
    auth(&headers, false)?;
    let meta = load_meta(&app, &table).await?;
    Ok(Json(json!({
        "table": meta.table,
        "columns": meta.cols.iter().map(col_json).collect::<Vec<_>>(),
        "pk": meta.pk,
        "readonly": meta.readonly,
        "reload": reload_target(&meta.table),
    })))
}

#[derive(Deserialize, Default)]
struct RowsQ {
    page: Option<i64>,
    size: Option<i64>,
    sort: Option<String>,
    dir: Option<String>,
    q: Option<String>,
    /// JSON object {column: filter}
    f: Option<String>,
}

async fn table_rows(headers: HeaderMap, State(app): State<App>, Path(table): Path<String>, Query(p): Query<RowsQ>) -> R {
    auth(&headers, false)?;
    let meta = load_meta(&app, &table).await?;
    if p.q.as_deref().map_or(0, str::len) > 200 || p.f.as_deref().map_or(0, str::len) > 8000 {
        return Err(bad("Arama çok uzun"));
    }
    let mut spec = ListSpec {
        sort: p.sort.filter(|s| !s.is_empty()),
        desc: p.dir.as_deref() == Some("desc"),
        q: p.q,
        filters: Vec::new(),
    };
    if let Some(f) = p.f.as_deref().filter(|f| !f.is_empty()) {
        let m: Map<String, Value> = serde_json::from_str(f).map_err(|_| bad("Geçersiz filtre"))?;
        if m.len() > 100 {
            return Err(bad("Çok fazla filtre"));
        }
        for (k, v) in m {
            if let Value::String(s) = v {
                spec.filters.push((k, s));
            }
        }
    }
    let built = build_list(&meta, &spec).map_err(bad)?;
    let (page, size, offset) = clamp_page(p.page, p.size);
    let filtered = !built.where_sql.is_empty();

    let mut tx = app.pool.begin().await.map_err(db_err("tx"))?;
    sqlx::query("SET LOCAL statement_timeout = '10000'").execute(&mut *tx).await.map_err(db_err("timeout"))?;
    let binds: Vec<Option<String>> = built.binds.iter().cloned().map(Some).collect();
    let sql = build_rows_sql(&meta, &built, size, offset);
    let rows = bind_all(sqlx::query(&sql), &binds).fetch_all(&mut *tx).await.map_err(db_err("rows"))?;
    let n = meta.cols.len();
    let data: Vec<Vec<Value>> = rows.iter().map(|r| row_values(r, n)).collect();

    let est: i64 = sqlx::query_scalar(
        "SELECT c.reltuples::bigint FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relname::text=$1",
    )
    .bind(&meta.table)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err("est"))?
    .unwrap_or(-1);
    let (total, exact) = if !filtered && est >= EXACT_COUNT_BELOW {
        (est, false)
    } else {
        let csql = format!("SELECT COUNT(*)::int8 FROM {}{}", quote_ident(&meta.table), built.where_sql);
        let r = bind_all(sqlx::query(&csql), &binds).fetch_one(&mut *tx).await.map_err(db_err("count"))?;
        (r.try_get::<i64, _>(0).map_err(db_err("count"))?, true)
    };
    tx.rollback().await.ok();
    let pages = ((total + size - 1) / size).max(1);
    Ok(Json(json!({"rows": data, "total": total, "exact": exact, "page": page, "size": size, "pages": pages})))
}

fn pk_of(meta: &Meta, vals: &[Value]) -> Map<String, Value> {
    let o = obj_of(meta, vals);
    meta.pk.iter().filter_map(|k| o.get(k).map(|v| (k.clone(), v.clone()))).collect()
}

#[derive(Deserialize)]
struct InsertIn {
    values: Map<String, Value>,
}

async fn insert_row(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(table): Path<String>,
    Json(p): Json<InsertIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let meta = load_meta(&app, &table).await?;
    require_writable(&meta)?;
    let (sql, binds) = build_insert(&meta, &p.values).map_err(bad)?;
    let mut tx = app.pool.begin().await.map_err(db_err("tx"))?;
    let row = bind_all(sqlx::query(&sql), &binds).fetch_one(&mut *tx).await.map_err(db_err("insert"))?;
    let vals = row_values(&row, meta.cols.len());
    let pk = pk_of(&meta, &vals);
    audit(&mut tx, &s.actor, "db.row.insert", &pk_label(&meta, &pk), json!({"table": meta.table, "pk": pk, "values": p.values}), peer.ip()).await?;
    tx.commit().await.map_err(db_err("commit"))?;
    Ok(Json(json!({"ok": true, "row": vals})))
}

#[derive(Deserialize)]
struct UpdateIn {
    pk: Map<String, Value>,
    changes: Map<String, Value>,
}

async fn update_row(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(table): Path<String>,
    Json(p): Json<UpdateIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let meta = load_meta(&app, &table).await?;
    require_writable(&meta)?;
    let (usql, ubinds) = build_update(&meta, &p.pk, &p.changes).map_err(bad)?;
    let (ssql, sbinds) = build_select_pk(&meta, &p.pk).map_err(bad)?;
    let mut tx = app.pool.begin().await.map_err(db_err("tx"))?;
    let old = bind_all(sqlx::query(&ssql), &sbinds)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err("select"))?
        .ok_or(DbbErr::Api(ApiError::NotFound))?;
    let old_obj = obj_of(&meta, &row_values(&old, meta.cols.len()));
    let res = bind_all(sqlx::query(&usql), &ubinds).execute(&mut *tx).await.map_err(db_err("update"))?;
    if res.rows_affected() != 1 {
        return Err(bad("Tek satır güncellenemedi; işlem geri alındı"));
    }
    // PK itself may have changed: re-select by the new PK values when present in `changes`.
    let mut new_pk = p.pk.clone();
    for k in &meta.pk {
        if let Some(v) = p.changes.get(k) {
            new_pk.insert(k.clone(), v.clone());
        }
    }
    let (ssql2, sbinds2) = build_select_pk(&meta, &new_pk).map_err(bad)?;
    let new_row = bind_all(sqlx::query(&ssql2), &sbinds2).fetch_one(&mut *tx).await.map_err(db_err("select"))?;
    let new_vals = row_values(&new_row, meta.cols.len());
    let new_obj = obj_of(&meta, &new_vals);
    let diff: Map<String, Value> = p
        .changes
        .keys()
        .map(|k| (k.clone(), json!({"old": old_obj.get(k), "new": new_obj.get(k)})))
        .collect();
    audit(&mut tx, &s.actor, "db.row.update", &pk_label(&meta, &p.pk), json!({"table": meta.table, "pk": p.pk, "changes": diff}), peer.ip()).await?;
    tx.commit().await.map_err(db_err("commit"))?;
    Ok(Json(json!({"ok": true, "row": new_vals})))
}

#[derive(Deserialize)]
struct DeleteIn {
    pk: Map<String, Value>,
}

async fn delete_row(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(table): Path<String>,
    Json(p): Json<DeleteIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let meta = load_meta(&app, &table).await?;
    require_writable(&meta)?;
    let (dsql, dbinds) = build_delete(&meta, &p.pk).map_err(bad)?;
    let (ssql, sbinds) = build_select_pk(&meta, &p.pk).map_err(bad)?;
    let mut tx = app.pool.begin().await.map_err(db_err("tx"))?;
    let old = bind_all(sqlx::query(&ssql), &sbinds)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err("select"))?
        .ok_or(DbbErr::Api(ApiError::NotFound))?;
    let old_obj = obj_of(&meta, &row_values(&old, meta.cols.len()));
    let res = bind_all(sqlx::query(&dsql), &dbinds).execute(&mut *tx).await.map_err(db_err("delete"))?;
    if res.rows_affected() != 1 {
        return Err(bad("Tek satır silinemedi; işlem geri alındı"));
    }
    audit(&mut tx, &s.actor, "db.row.delete", &pk_label(&meta, &p.pk), json!({"table": meta.table, "pk": p.pk, "old": old_obj}), peer.ip()).await?;
    tx.commit().await.map_err(db_err("commit"))?;
    Ok(Json(json!({"ok": true})))
}

async fn reload_table(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(table): Path<String>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let meta = load_meta(&app, &table).await?;
    let Some(target) = reload_target(&meta.table) else {
        return Err(bad("Bu tablo için canlı yenileme yok; oyun sunucusunu yeniden başlatmak gerekir"));
    };
    let id = queue_command(&app.pool, "reload", json!({"what": target})).await?;
    let mut tx = app.pool.begin().await.map_err(db_err("tx"))?;
    audit(&mut tx, &s.actor, "db.table.reload", &meta.table, json!({"what": target, "command_id": id}), peer.ip()).await?;
    tx.commit().await.map_err(db_err("commit"))?;
    Ok(Json(json!({"ok": true, "id": id, "what": target})))
}

// ───────────────────────── tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, dt: &str) -> Col {
        let (kind, cast) = kind_of(dt);
        Col {
            name: name.into(),
            data_type: dt.into(),
            nullable: true,
            default: None,
            max_len: None,
            kind,
            cast,
            sensitive: col_sensitive(name),
            generated: false,
        }
    }
    fn meta() -> Meta {
        Meta {
            table: "quest_monster".into(),
            cols: vec![col("s_quest_num", "smallint"), col("s_num", "integer"), col("name", "text"), col("str_passwd", "text"), col("blob", "bytea")],
            pk: vec!["s_quest_num".into()],
            readonly: None,
        }
    }

    #[test]
    fn quoting() {
        assert_eq!(quote_ident("a\"b"), "\"a\"\"b\"");
        assert_eq!(like_escape("50%_\\x"), "50\\%\\_\\\\x");
    }

    #[test]
    fn page_clamps() {
        assert_eq!(clamp_page(None, None), (1, 50, 0));
        assert_eq!(clamp_page(Some(0), Some(0)), (1, 1, 0));
        assert_eq!(clamp_page(Some(3), Some(100000)), (3, 500, 1000));
        assert_eq!(clamp_page(Some(-5), Some(-5)).0, 1);
    }

    #[test]
    fn sensitive_and_hidden() {
        assert!(col_sensitive("str_passwd") && col_sensitive("password_hash") && col_sensitive("OTP_password"));
        assert!(!col_sensitive("name"));
        assert!(table_hidden("admin_users") && table_hidden("_sqlx_migrations") && !table_hidden("item"));
        assert!(!col("str_passwd", "text").editable());
        assert!(!col("blob", "bytea").editable());
    }

    #[test]
    fn list_whitelist() {
        let m = meta();
        let b = build_list(&m, &ListSpec { sort: Some("s_num".into()), desc: true, q: Some("a%b".into()), filters: vec![("s_num".into(), "=5".into())] }).unwrap();
        assert!(b.where_sql.contains("\"s_quest_num\"::text ILIKE $1"));
        assert!(!b.where_sql.contains("str_passwd") && !b.where_sql.contains("blob"));
        assert!(b.where_sql.contains("\"s_num\"::text = $2"));
        assert_eq!(b.binds, vec!["%a\\%b%".to_string(), "5".to_string()]);
        assert_eq!(b.order_sql, " ORDER BY \"s_num\" DESC NULLS LAST, \"s_quest_num\"");
        assert!(build_list(&m, &ListSpec { sort: Some("nope; drop".into()), ..Default::default() }).is_err());
        assert!(build_list(&m, &ListSpec { sort: Some("str_passwd".into()), ..Default::default() }).is_err());
        assert!(build_list(&m, &ListSpec { filters: vec![("blob".into(), "x".into())], ..Default::default() }).is_err());
        assert!(build_list(&m, &ListSpec { filters: vec![("x\" OR 1=1".into(), "x".into())], ..Default::default() }).is_err());
    }

    #[test]
    fn no_pk_orders_by_ctid_and_select_masks() {
        let mut m = meta();
        m.pk.clear();
        let b = build_list(&m, &ListSpec::default()).unwrap();
        assert_eq!(b.order_sql, " ORDER BY ctid");
        let sel = select_list(&m);
        assert!(sel.contains("NULL::text AS \"str_passwd\""));
        assert!(sel.contains("encode(substring(\"blob\" from 1 for 64),'hex')"));
        assert!(readonly_reason(&m.cols, &m.pk).is_some());
    }

    #[test]
    fn update_insert_delete() {
        let m = meta();
        let pk: Map<String, Value> = serde_json::from_value(json!({"s_quest_num": "12"})).unwrap();
        let ch: Map<String, Value> = serde_json::from_value(json!({"s_num": ""})).unwrap();
        let (sql, binds) = build_update(&m, &pk, &ch).unwrap();
        assert_eq!(sql, "UPDATE \"quest_monster\" SET \"s_num\" = $1::text::int4 WHERE \"s_quest_num\" = $2::text::int2");
        assert_eq!(binds, vec![None, Some("12".into())]);
        let txt: Map<String, Value> = serde_json::from_value(json!({"name": ""})).unwrap();
        assert_eq!(build_update(&m, &pk, &txt).unwrap().1[0], Some(String::new()));
        let bad_ch: Map<String, Value> = serde_json::from_value(json!({"str_passwd": "x"})).unwrap();
        assert!(build_update(&m, &pk, &bad_ch).is_err());
        let unk: Map<String, Value> = serde_json::from_value(json!({"zzz": "x"})).unwrap();
        assert!(build_update(&m, &pk, &unk).is_err());
        assert!(build_update(&m, &Map::new(), &ch).is_err());
        let (d, db) = build_delete(&m, &pk).unwrap();
        assert_eq!(d, "DELETE FROM \"quest_monster\" WHERE \"s_quest_num\" = $1::text::int2");
        assert_eq!(db.len(), 1);
        let (i, _) = build_insert(&m, &Map::new()).unwrap();
        assert!(i.contains("DEFAULT VALUES RETURNING"));
        let iv: Map<String, Value> = serde_json::from_value(json!({"s_num": 5})).unwrap();
        let (i, ib) = build_insert(&m, &iv).unwrap();
        assert!(i.starts_with("INSERT INTO \"quest_monster\" (\"s_num\") VALUES ($1::text::int4)"));
        assert_eq!(ib, vec![Some("5".into())]);
    }

    #[test]
    fn reload_map() {
        assert_eq!(reload_target("quest_monster"), Some("quest_tables"));
        assert_eq!(reload_target("npc_spawn"), Some("npc_spawn"));
        assert_eq!(reload_target("item"), None);
    }
}
