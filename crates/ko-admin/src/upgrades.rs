//! Item upgrade (anvil) rate management.
//!
//! The server decides an upgrade attempt from `item_upgrade_settings` (see ko-game's
//! `item_upgrade.rs`): the row is picked by the item's `item_type`, the recipe grade (or the
//! wildcard grade 99) and the two material items the player put on the anvil (scroll, Trina /
//! Karivdis ...). `success_rate` is out of 10000, `item_req_coins` is paid on success and failure.
//! The recipes themselves (`new_upgrade`: origin item -> next item, with the scroll) only say which
//! item becomes which, so this page shows both together.
//!
//! The table has no primary key, so a row is addressed by its `ctid` (re-read after every change).

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
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
        .route("/jstkoadminpanel/api/upgrades/rules", get(rules).post(rule_create))
        .route("/jstkoadminpanel/api/upgrades/rules/{ctid}", post(rule_update).delete(rule_delete))
        .route("/jstkoadminpanel/api/upgrades/bulk", post(bulk))
        .route("/jstkoadminpanel/api/upgrades/item/{num}", get(item_chain))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

async fn queue_reload(pool: &ko_db::DbPool) {
    let _ = queue_command(pool, "reload", json!({"what": "item_upgrade_settings"})).await;
}

/// ctid text such as "(12,3)": digits, comma and parentheses only.
fn valid_ctid(s: &str) -> bool {
    s.len() < 24 && s.starts_with('(') && s.ends_with(')') && s.chars().all(|c| c.is_ascii_digit() || matches!(c, '(' | ')' | ','))
}

const RULE_SELECT: &str = "SELECT s.ctid::text AS ctid, s.item_type, s.item_rate, s.item_grade, s.item_req_coins, s.success_rate,
        s.req_item_id1, s.req_item_id2, COALESCE(NULLIF(i1.str_name,''), s.req_item_name1, '') AS name1,
        COALESCE(NULLIF(i2.str_name,''), s.req_item_name2, '') AS name2, COALESCE(s.upgrade_note,'') AS note
     FROM item_upgrade_settings s
     LEFT JOIN item i1 ON i1.num = s.req_item_id1 AND s.req_item_id1 > 0
     LEFT JOIN item i2 ON i2.num = s.req_item_id2 AND s.req_item_id2 > 0";

fn rule_json(r: &sqlx::postgres::PgRow) -> Value {
    json!({
        "ctid": r.get::<String, _>("ctid"),
        "item_type": r.get::<i16, _>("item_type"),
        "item_rate": r.get::<i16, _>("item_rate"),
        "grade": r.get::<i16, _>("item_grade"),
        "coins": r.get::<i32, _>("item_req_coins"),
        "rate": r.get::<i16, _>("success_rate"),
        "req1": r.get::<i32, _>("req_item_id1"),
        "req2": r.get::<i32, _>("req_item_id2"),
        "name1": r.try_get::<String, _>("name1").unwrap_or_default(),
        "name2": r.try_get::<String, _>("name2").unwrap_or_default(),
        "note": r.try_get::<String, _>("note").unwrap_or_default(),
    })
}

#[derive(Deserialize)]
struct RuleQ {
    item_type: Option<i16>,
    item_rate: Option<i16>,
    grade_from: Option<i16>,
    grade_to: Option<i16>,
    material: Option<i32>,
}

/// Every rule (optionally filtered), plus the filter dropdown contents.
async fn rules(headers: HeaderMap, State(app): State<App>, Query(q): Query<RuleQ>) -> R {
    auth(&headers, false)?;
    let sql = format!(
        "{RULE_SELECT}
         WHERE ($1::int2 IS NULL OR s.item_type = $1) AND ($2::int2 IS NULL OR s.item_rate = $2)
           AND ($3::int2 IS NULL OR s.item_grade >= $3) AND ($4::int2 IS NULL OR s.item_grade <= $4)
           AND ($5::int4 IS NULL OR s.req_item_id1 = $5 OR s.req_item_id2 = $5)
         ORDER BY s.item_type, s.item_rate, s.item_grade, s.req_item_id1, s.req_item_id2 LIMIT 1500"
    );
    let rows = sqlx::query(&sql)
        .bind(q.item_type)
        .bind(q.item_rate)
        .bind(q.grade_from)
        .bind(q.grade_to)
        .bind(q.material)
        .fetch_all(&app.pool)
        .await
        .map_err(internal("upgrade rules"))?;
    let types = sqlx::query(
        "SELECT item_type, item_rate, count(*) n FROM item_upgrade_settings GROUP BY 1,2 ORDER BY 1,2",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("upgrade types"))?;
    let materials = sqlx::query(
        "SELECT m.id, COALESCE(NULLIF(i.str_name,''), '#'||m.id) name FROM (
            SELECT req_item_id1 id FROM item_upgrade_settings WHERE req_item_id1 > 0
            UNION SELECT req_item_id2 FROM item_upgrade_settings WHERE req_item_id2 > 0) m
         LEFT JOIN item i ON i.num = m.id ORDER BY 2",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("upgrade materials"))?;
    Ok(Json(json!({
        "rules": rows.iter().map(rule_json).collect::<Vec<_>>(),
        "limit_hit": rows.len() >= 1500,
        "groups": types.iter().map(|r| json!({"item_type": r.get::<i16,_>("item_type"), "item_rate": r.get::<i16,_>("item_rate"), "n": r.get::<i64,_>("n")})).collect::<Vec<_>>(),
        "materials": materials.iter().map(|r| json!({"id": r.get::<i32,_>("id"), "name": r.get::<String,_>("name")})).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct RuleEdit {
    rate: Option<i32>,
    coins: Option<i32>,
}

async fn rule_update(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(ctid): Path<String>,
    Json(p): Json<RuleEdit>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid_ctid(&ctid) || p.rate.is_some_and(|v| !(0..=10_000).contains(&v)) || p.coins.is_some_and(|v| v < 0) {
        return Err(ApiError::BadRequest);
    }
    let row = sqlx::query(
        "UPDATE item_upgrade_settings SET success_rate = COALESCE($2::int2, success_rate), item_req_coins = COALESCE($3, item_req_coins)
         WHERE ctid = $1::text::tid RETURNING item_type, item_grade, req_item_id1, req_item_id2",
    )
    .bind(&ctid)
    .bind(p.rate.map(|v| v as i16))
    .bind(p.coins)
    .fetch_optional(&app.pool)
    .await
    .map_err(internal("upgrade rule update"))?
    .ok_or(ApiError::NotFound)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(
        &mut tx,
        &s.actor,
        "upgrade.rule.edit",
        &format!("t{}g{}", row.get::<i16, _>("item_type"), row.get::<i16, _>("item_grade")),
        json!({"rate": p.rate, "coins": p.coins, "req1": row.get::<i32,_>("req_item_id1"), "req2": row.get::<i32,_>("req_item_id2")}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct RuleNew {
    item_type: i16,
    item_rate: Option<i16>,
    grade: i16,
    req1: Option<i32>,
    req2: Option<i32>,
    rate: i32,
    coins: i32,
}

async fn rule_create(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<RuleNew>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !(0..=10_000).contains(&p.rate) || p.coins < 0 || !(0..=99).contains(&p.grade) {
        return Err(ApiError::BadRequest);
    }
    sqlx::query(
        "INSERT INTO item_upgrade_settings (req_item_id1, req_item_id2, item_type, item_rate, item_grade, item_req_coins, success_rate, item_upgrade_status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 1)",
    )
    .bind(p.req1.unwrap_or(0))
    .bind(p.req2.unwrap_or(0))
    .bind(p.item_type)
    .bind(p.item_rate.unwrap_or(0))
    .bind(p.grade)
    .bind(p.coins)
    .bind(p.rate as i16)
    .execute(&app.pool)
    .await
    .map_err(internal("upgrade rule insert"))?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "upgrade.rule.add", &format!("t{}g{}", p.item_type, p.grade), json!({"rate": p.rate, "coins": p.coins, "req1": p.req1, "req2": p.req2}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true})))
}

async fn rule_delete(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(ctid): Path<String>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid_ctid(&ctid) {
        return Err(ApiError::BadRequest);
    }
    let done = sqlx::query("DELETE FROM item_upgrade_settings WHERE ctid = $1::text::tid")
        .bind(&ctid)
        .execute(&app.pool)
        .await
        .map_err(internal("upgrade rule delete"))?
        .rows_affected();
    if done == 0 {
        return Err(ApiError::NotFound);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "upgrade.rule.remove", &ctid, json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct BulkIn {
    item_type: Option<i16>,
    item_rate: Option<i16>,
    grade_from: Option<i16>,
    grade_to: Option<i16>,
    material: Option<i32>,
    /// "rate" or "coins"
    field: String,
    /// "set" (absolute), "mul" (multiply, value in percent: 150 = x1.5) or "add" (add points / coins)
    mode: String,
    value: f64,
    /// true = only report what would change
    preview: bool,
}

fn apply(mode: &str, field: &str, old: i32, v: f64) -> i32 {
    let n = match mode {
        "set" => v,
        "mul" => old as f64 * v / 100.0,
        _ => old as f64 + v,
    };
    let n = n.round();
    if field == "rate" { n.clamp(0.0, 10_000.0) as i32 } else { n.clamp(0.0, 2_000_000_000.0) as i32 }
}

async fn bulk(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<BulkIn>,
) -> R {
    let (_, s) = auth(&headers, !p.preview)?;
    if !matches!(p.field.as_str(), "rate" | "coins") || !matches!(p.mode.as_str(), "set" | "mul" | "add") || !p.value.is_finite() {
        return Err(ApiError::BadRequest);
    }
    let col = if p.field == "rate" { "success_rate::int4" } else { "item_req_coins" };
    let sql = format!(
        "SELECT ctid::text AS ctid, {col} AS cur FROM item_upgrade_settings s
         WHERE ($1::int2 IS NULL OR s.item_type = $1) AND ($2::int2 IS NULL OR s.item_rate = $2)
           AND ($3::int2 IS NULL OR s.item_grade >= $3) AND ($4::int2 IS NULL OR s.item_grade <= $4)
           AND ($5::int4 IS NULL OR s.req_item_id1 = $5 OR s.req_item_id2 = $5)"
    );
    let rows = sqlx::query(&sql)
        .bind(p.item_type)
        .bind(p.item_rate)
        .bind(p.grade_from)
        .bind(p.grade_to)
        .bind(p.material)
        .fetch_all(&app.pool)
        .await
        .map_err(internal("upgrade bulk select"))?;
    let changes: Vec<(String, i32, i32)> = rows
        .iter()
        .map(|r| {
            let cur: i32 = r.get("cur");
            (r.get::<String, _>("ctid"), cur, apply(&p.mode, &p.field, cur, p.value))
        })
        .filter(|(_, old, new)| old != new)
        .collect();
    let sample: Vec<Value> = changes.iter().take(8).map(|(_, o, n)| json!({"old": o, "new": n})).collect();
    if p.preview {
        return Ok(Json(json!({"matched": rows.len(), "changed": changes.len(), "sample": sample})));
    }
    if changes.is_empty() {
        return Ok(Json(json!({"matched": rows.len(), "changed": 0})));
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let upd = if p.field == "rate" {
        "UPDATE item_upgrade_settings SET success_rate = $2::int4::int2 WHERE ctid = $1::text::tid"
    } else {
        "UPDATE item_upgrade_settings SET item_req_coins = $2 WHERE ctid = $1::text::tid"
    };
    // updating a row moves its ctid, so every change addresses the ctid read above exactly once
    for (ctid, _, new) in &changes {
        let q = sqlx::query(upd).bind(ctid);
        let q = q.bind(*new);
        q.execute(&mut *tx).await.map_err(internal("upgrade bulk update"))?;
    }
    audit(
        &mut tx,
        &s.actor,
        "upgrade.bulk",
        &format!("{}:{}", p.field, p.mode),
        json!({"item_type": p.item_type, "item_rate": p.item_rate, "from": p.grade_from, "to": p.grade_to, "material": p.material, "value": p.value, "rows": changes.len()}),
        peer.ip(),
    )
    .await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"matched": rows.len(), "changed": changes.len(), "sample": sample})))
}

/// Upgrade path of one item: every step the recipes allow (origin -> next item, scroll) with the rules
/// (materials, coins, success %) that apply to it.
async fn item_chain(headers: HeaderMap, State(app): State<App>, Path(num): Path<i32>) -> R {
    auth(&headers, false)?;
    let item = sqlx::query("SELECT num, str_name, item_type, item_class, by_grade, item_icon_id1 FROM item WHERE num = $1")
        .bind(num)
        .fetch_optional(&app.pool)
        .await
        .map_err(internal("upgrade item"))?
        .ok_or(ApiError::NotFound)?;
    let item_type: i16 = item.try_get::<i32, _>("item_type").unwrap_or(0) as i16;

    // Walk the recipe graph (breadth first, bounded).
    let mut steps: Vec<Value> = Vec::new();
    let mut seen: HashSet<i32> = HashSet::from([num]);
    let mut queue: VecDeque<(i32, u32)> = VecDeque::from([(num, 0)]);
    let mut names: HashMap<i32, String> = HashMap::new();
    let mut max_depth = 0u32;
    while let Some((cur, depth)) = queue.pop_front() {
        if depth >= 40 || steps.len() >= 300 {
            continue;
        }
        let rows = sqlx::query(
            "SELECT new_number, req_item, grade FROM new_upgrade WHERE origin_number = $1 AND new_number > 0 ORDER BY new_number, req_item LIMIT 40",
        )
        .bind(cur)
        .fetch_all(&app.pool)
        .await
        .map_err(internal("upgrade chain"))?;
        for r in rows {
            let to: i32 = r.get("new_number");
            let scroll: i32 = r.get("req_item");
            let grade: i16 = r.get("grade");
            steps.push(json!({"depth": depth + 1, "from": cur, "to": to, "scroll": scroll, "grade": grade}));
            max_depth = max_depth.max(depth + 1);
            if seen.insert(to) {
                queue.push_back((to, depth + 1));
            }
        }
    }
    // names for everything mentioned
    let mut ids: HashSet<i32> = HashSet::from([num]);
    for st in &steps {
        for k in ["from", "to", "scroll"] {
            ids.insert(st[k].as_i64().unwrap_or(0) as i32);
        }
    }
    let idv: Vec<i32> = ids.into_iter().filter(|i| *i > 0).collect();
    for r in sqlx::query("SELECT num, str_name FROM item WHERE num = ANY($1)")
        .bind(&idv)
        .fetch_all(&app.pool)
        .await
        .map_err(internal("upgrade names"))?
    {
        names.insert(r.get("num"), r.try_get::<String, _>("str_name").unwrap_or_default());
    }
    // rules for this item type
    let sql = format!("{RULE_SELECT} WHERE s.item_type = $1 ORDER BY s.item_grade, s.req_item_id1");
    let rule_rows = sqlx::query(&sql)
        .bind(item_type)
        .fetch_all(&app.pool)
        .await
        .map_err(internal("upgrade chain rules"))?;
    let rules: Vec<Value> = rule_rows.iter().map(rule_json).collect();
    let steps: Vec<Value> = steps
        .into_iter()
        .map(|mut st| {
            let grade = st["grade"].as_i64().unwrap_or(0);
            let scroll = st["scroll"].as_i64().unwrap_or(0);
            let matching: Vec<&Value> = rules
                .iter()
                .filter(|r| {
                    let g = r["grade"].as_i64().unwrap_or(-1);
                    (g == grade || g == 99) && (scroll == 0 || r["req1"].as_i64() == Some(scroll) || r["req2"].as_i64() == Some(scroll))
                })
                .collect();
            st["from_name"] = json!(names.get(&(st["from"].as_i64().unwrap_or(0) as i32)).cloned().unwrap_or_default());
            st["to_name"] = json!(names.get(&(st["to"].as_i64().unwrap_or(0) as i32)).cloned().unwrap_or_default());
            st["scroll_name"] = json!(names.get(&(scroll as i32)).cloned().unwrap_or_default());
            st["rules"] = json!(matching);
            st
        })
        .collect();
    Ok(Json(json!({
        "item": {
            "num": num,
            "name": item.try_get::<String, _>("str_name").unwrap_or_default(),
            "item_type": item_type,
            "item_class": item.try_get::<i16, _>("item_class").unwrap_or(0),
            "icon": item.try_get::<i32, _>("item_icon_id1").unwrap_or(0),
        },
        "max_steps": max_depth,
        "steps": steps,
        "rule_count": rules.len(),
    })))
}
