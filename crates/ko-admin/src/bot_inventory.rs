//! "Bot Envanteri" page: what every live bot wears and what that gives (max HP, item HP, AC).
//!
//! The bots live in the game server's memory, so the page reads `bot_snapshot`, a table the game
//! server rewrites when it gets the `bot_snapshot` admin command (the page's "Yenile" button queues
//! it). PK / field bots all have a fixed max HP per class (`bot_ai::bot_hp_target`); the
//! "formül HP" column is what level + stats + gear alone would have given.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::legacy::{auth, queue_command, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/bot-inventory", get(list))
        .route("/jstkoadminpanel/api/bot-inventory/refresh", post(refresh))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

#[derive(Deserialize)]
struct Q {
    kind: Option<String>,
    class_family: Option<String>,
    q: Option<String>,
}

/// Class families by `class % 100` (same grouping as the HP targets).
fn family_sql(f: &str) -> Option<&'static str> {
    Some(match f {
        "warrior" => "(class % 100) IN (1,5,6)",
        "rogue" => "(class % 100) IN (2,7,8)",
        "mage" => "(class % 100) IN (3,9,10)",
        "priest" => "(class % 100) IN (4,11,12)",
        "kurian" => "(class % 100) >= 13",
        _ => return None,
    })
}

async fn list(headers: HeaderMap, State(app): State<App>, Query(q): Query<Q>) -> R {
    auth(&headers, false)?;
    let mut sql = String::from(
        "SELECT bot_id, name, class, level, nation, zone_id, kind, max_hp, formula_hp, item_hp, item_ac, items,
                to_char(taken_at AT TIME ZONE 'UTC', 'HH24:MI:SS') AS taken
         FROM bot_snapshot WHERE TRUE",
    );
    if let Some(k) = q.kind.as_deref().filter(|k| matches!(*k, "pk" | "field" | "event")) {
        sql.push_str(&format!(" AND kind = '{k}'"));
    }
    if let Some(cond) = q.class_family.as_deref().and_then(family_sql) {
        sql.push_str(&format!(" AND {cond}"));
    }
    sql.push_str(" ORDER BY max_hp DESC, level DESC, name LIMIT 600");
    let rows = sqlx::query(&sql).fetch_all(&app.pool).await.map_err(internal("bot snapshot"))?;

    // names of every worn item
    let mut ids: Vec<i32> = Vec::new();
    for r in &rows {
        if let Ok(Value::Array(a)) = r.try_get::<Value, _>("items") {
            for it in a {
                if let Some(n) = it["item"].as_i64() {
                    ids.push(n as i32);
                }
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    let names = sqlx::query("SELECT num, str_name, item_icon_id1 FROM item WHERE num = ANY($1)")
        .bind(&ids)
        .fetch_all(&app.pool)
        .await
        .map_err(internal("bot snapshot names"))?;
    let mut name_map = serde_json::Map::new();
    for n in names {
        name_map.insert(
            n.get::<i32, _>("num").to_string(),
            json!({"name": n.try_get::<String, _>("str_name").unwrap_or_default(), "icon": n.try_get::<i32, _>("item_icon_id1").unwrap_or(0)}),
        );
    }

    let needle = q.q.as_deref().unwrap_or("").trim().to_lowercase();
    let bots: Vec<Value> = rows
        .iter()
        .filter(|r| needle.is_empty() || r.get::<String, _>("name").to_lowercase().contains(&needle))
        .map(|r| {
            json!({
                "id": r.get::<i64, _>("bot_id"),
                "name": r.get::<String, _>("name"),
                "class": r.get::<i32, _>("class"),
                "level": r.get::<i32, _>("level"),
                "nation": r.get::<i32, _>("nation"),
                "zone": r.get::<i32, _>("zone_id"),
                "kind": r.get::<String, _>("kind"),
                "max_hp": r.get::<i32, _>("max_hp"),
                "formula_hp": r.get::<i32, _>("formula_hp"),
                "item_hp": r.get::<i32, _>("item_hp"),
                "ac": r.get::<i32, _>("item_ac"),
                "items": r.try_get::<Value, _>("items").unwrap_or(Value::Null),
            })
        })
        .collect();

    // per class family summary over the WHOLE snapshot (not just the filtered/limited rows)
    let summary = sqlx::query(
        "SELECT CASE WHEN class % 100 IN (1,5,6) THEN 'warrior' WHEN class % 100 IN (2,7,8) THEN 'rogue'
                     WHEN class % 100 IN (3,9,10) THEN 'mage' WHEN class % 100 IN (4,11,12) THEN 'priest' ELSE 'kurian' END AS fam,
                count(*) n, min(max_hp) lo, max(max_hp) hi, round(avg(max_hp))::int avg_hp,
                min(formula_hp) flo, max(formula_hp) fhi, round(avg(item_ac))::int avg_ac
         FROM bot_snapshot GROUP BY 1 ORDER BY 1",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal("bot snapshot summary"))?;
    let taken = rows.first().map(|r| r.get::<String, _>("taken"));
    Ok(Json(json!({
        "bots": bots,
        "names": Value::Object(name_map),
        "taken": taken,
        "summary": summary.iter().map(|r| json!({
            "family": r.get::<String, _>("fam"), "n": r.get::<i64, _>("n"),
            "hp_min": r.get::<i32, _>("lo"), "hp_max": r.get::<i32, _>("hi"), "hp_avg": r.get::<i32, _>("avg_hp"),
            "formula_min": r.get::<i32, _>("flo"), "formula_max": r.get::<i32, _>("fhi"), "ac_avg": r.get::<i32, _>("avg_ac"),
        })).collect::<Vec<_>>(),
    })))
}

async fn refresh(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, true)?;
    let id = queue_command(&app.pool, "bot_snapshot", json!({})).await?;
    Ok(Json(json!({"ok": true, "command": id})))
}
