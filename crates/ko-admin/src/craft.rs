//! Craft page (Shozin 19073 / jeweler 31402): the special-part-sewing recipes of the combine window.
//!
//! Recipes live in `item_special_sewing` (up to 10 material stacks -> one result, `success_rate` out of
//! 10000, `is_shadow_success`: with a Shadow Piece (700009000) the craft always succeeds, otherwise a
//! Shadow Piece adds 40 % of the rate). The game server reads them at start and on the
//! `item_special_sewing` reload (queued after every change here).

use std::collections::BTreeMap;
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
        .route("/jstkoadminpanel/api/craft/recipes", get(list).post(create))
        .route("/jstkoadminpanel/api/craft/recipes/{n}", post(update).delete(remove))
        .route("/jstkoadminpanel/api/craft/bulk", post(bulk))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

const NPCS: [(i16, &str); 2] = [(19073, "Shozin (Hepa çırağı) — silah / zırh / Draki"), (31402, "Kuyumcu — takı birleştirme")];

async fn queue_reload(pool: &ko_db::DbPool) {
    let _ = queue_command(pool, "reload", json!({"what": "item_special_sewing"})).await;
}

#[derive(Deserialize)]
struct ListQ {
    npc: Option<i16>,
    q: Option<String>,
    desc: Option<String>,
    /// "yes" = only recipes the client formula book has, "no" = only those it lacks
    client: Option<String>,
}

async fn list(headers: HeaderMap, State(app): State<App>, Query(p): Query<ListQ>) -> R {
    auth(&headers, false)?;
    let npc = p.npc.unwrap_or(19073);
    let needle = p.q.as_deref().unwrap_or("").trim().to_lowercase();
    let rows = sqlx::query(
        "SELECT s.*, g.str_name gname, g.item_icon_id1 gicon,
                m1.str_name n1, m1.item_icon_id1 i1, m2.str_name n2, m2.item_icon_id1 i2, m3.str_name n3, m3.item_icon_id1 i3,
                m4.str_name n4, m4.item_icon_id1 i4, m5.str_name n5, m5.item_icon_id1 i5, m6.str_name n6, m6.item_icon_id1 i6,
                m7.str_name n7, m7.item_icon_id1 i7, m8.str_name n8, m8.item_icon_id1 i8, m9.str_name n9, m9.item_icon_id1 i9,
                m10.str_name n10, m10.item_icon_id1 i10
         FROM item_special_sewing s
         LEFT JOIN item g ON g.num = s.give_item_id
         LEFT JOIN item m1 ON m1.num = s.req_item_id_1 LEFT JOIN item m2 ON m2.num = s.req_item_id_2
         LEFT JOIN item m3 ON m3.num = s.req_item_id_3 LEFT JOIN item m4 ON m4.num = s.req_item_id_4
         LEFT JOIN item m5 ON m5.num = s.req_item_id_5 LEFT JOIN item m6 ON m6.num = s.req_item_id_6
         LEFT JOIN item m7 ON m7.num = s.req_item_id_7 LEFT JOIN item m8 ON m8.num = s.req_item_id_8
         LEFT JOIN item m9 ON m9.num = s.req_item_id_9 LEFT JOIN item m10 ON m10.num = s.req_item_id_10
         WHERE s.npc_id = $1 ORDER BY s.n_index",
    )
    .bind(npc)
    .fetch_all(&app.pool)
    .await
    .map_err(internal("craft list"))?;

    let descs = sqlx::query(
        "SELECT COALESCE(description,'') d, count(*) n FROM item_special_sewing WHERE npc_id = $1 GROUP BY 1 ORDER BY 2 DESC",
    )
    .bind(npc)
    .fetch_all(&app.pool)
    .await
    .map_err(internal("craft descs"))?;

    // which recipes the client formula book lists (craft_client_mix, made by tools/craft_client_sync.py)
    let mut client: std::collections::HashMap<i32, (i32, String, String)> = std::collections::HashMap::new();
    if let Ok(cl) = sqlx::query("SELECT unnest(server_rows) n, mix_index, category_name, status FROM craft_client_mix WHERE npc_id = $1")
        .bind(npc as i32)
        .fetch_all(&app.pool)
        .await
    {
        for c in cl {
            client.insert(c.get::<i32, _>("n"), (c.get::<i32, _>("mix_index"), c.get::<String, _>("category_name"), c.get::<String, _>("status")));
        }
    }
    let only_client = p.client.as_deref() == Some("yes");
    let only_no_client = p.client.as_deref() == Some("no");

    // one group = same description + same material multiset; its rows are the possible results
    let mut groups: BTreeMap<String, Value> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    for r in &rows {
        let desc: String = r.try_get::<Option<String>, _>("description").ok().flatten().unwrap_or_default();
        if p.desc.as_deref().is_some_and(|d| !d.is_empty() && d != desc) {
            continue;
        }
        let mut mats: Vec<(i32, i32, String, i32)> = Vec::new();
        for i in 1..=10 {
            let id: i32 = r.get(format!("req_item_id_{i}").as_str());
            let cnt: i32 = r.get(format!("req_item_count_{i}").as_str());
            if id > 0 && cnt > 0 {
                mats.push((
                    id,
                    cnt,
                    r.try_get::<Option<String>, _>(format!("n{i}").as_str()).ok().flatten().unwrap_or_default(),
                    r.try_get::<Option<i32>, _>(format!("i{i}").as_str()).ok().flatten().unwrap_or(0),
                ));
            }
        }
        let n_index: i32 = r.get("n_index");
        let in_client = client.contains_key(&n_index);
        if (only_client && !in_client) || (only_no_client && in_client) {
            continue;
        }
        let give: i32 = r.get("give_item_id");
        let gname: String = r.try_get::<Option<String>, _>("gname").ok().flatten().unwrap_or_default();
        if !needle.is_empty() {
            let hay = format!(
                "{} {} {} {}",
                desc.to_lowercase(),
                gname.to_lowercase(),
                give,
                mats.iter().map(|m| format!("{} {}", m.2.to_lowercase(), m.0)).collect::<Vec<_>>().join(" ")
            );
            if !hay.contains(&needle) {
                continue;
            }
        }
        let mut key_mats = mats.clone();
        key_mats.sort();
        let key = format!("{desc}|{}", key_mats.iter().map(|m| format!("{}x{}", m.0, m.1)).collect::<Vec<_>>().join(","));
        let variant = json!({
            "n": r.get::<i32, _>("n_index"),
            "give": give,
            "name": gname,
            "icon": r.try_get::<Option<i32>, _>("gicon").ok().flatten().unwrap_or(0),
            "count": r.get::<i32, _>("give_item_count"),
            "rate": r.get::<i32, _>("success_rate"),
            "shadow": r.get::<bool, _>("is_shadow_success"),
            "notice": r.get::<bool, _>("is_notice"),
        });
        let cinfo = client.get(&n_index).cloned();
        match groups.get_mut(&key) {
            Some(g) => {
                g["variants"].as_array_mut().unwrap().push(variant);
                if g["client"].is_null() {
                    if let Some((mix, cat, st)) = &cinfo {
                        g["client"] = json!({"mix": mix, "category": cat, "status": st});
                    }
                }
            }
            None => {
                order.push(key.clone());
                groups.insert(
                    key,
                    json!({
                        "desc": desc,
                        "materials": mats.iter().map(|m| json!({"id": m.0, "count": m.1, "name": m.2, "icon": m.3})).collect::<Vec<_>>(),
                        "variants": [variant],
                        "client": cinfo.map(|(mix, cat, st)| json!({"mix": mix, "category": cat, "status": st})),
                    }),
                );
            }
        }
    }
    let total = order.len();
    let list: Vec<Value> = order.into_iter().take(300).filter_map(|k| groups.remove(&k)).collect();
    Ok(Json(json!({
        "npcs": NPCS.iter().map(|(id, label)| json!({"id": id, "label": label})).collect::<Vec<_>>(),
        "descs": descs.iter().map(|r| json!({"d": r.get::<String, _>("d"), "n": r.get::<i64, _>("n")})).collect::<Vec<_>>(),
        "groups": list,
        "total_groups": total,
        "shadow_item": 700009000,
    })))
}

#[derive(Deserialize)]
struct Edit {
    rate: Option<i32>,
    shadow: Option<bool>,
    give_count: Option<i32>,
    give_item: Option<i32>,
}

async fn apply_edit(app: &App, n: i32, p: &Edit) -> Result<u64, ApiError> {
    if p.rate.is_some_and(|r| !(0..=10000).contains(&r)) || p.give_count.is_some_and(|c| !(1..=999).contains(&c)) {
        return Err(ApiError::BadRequest);
    }
    if let Some(item) = p.give_item {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM item WHERE num = $1)")
            .bind(item)
            .fetch_one(&app.pool)
            .await
            .map_err(internal("craft item check"))?;
        if !exists {
            return Err(ApiError::BadRequest);
        }
    }
    let done = sqlx::query(
        "UPDATE item_special_sewing SET success_rate = COALESCE($2, success_rate), is_shadow_success = COALESCE($3, is_shadow_success),
                give_item_count = COALESCE($4, give_item_count), give_item_id = COALESCE($5, give_item_id)
         WHERE n_index = $1 AND npc_id IN (19073, 31402)",
    )
    .bind(n)
    .bind(p.rate)
    .bind(p.shadow)
    .bind(p.give_count)
    .bind(p.give_item)
    .execute(&app.pool)
    .await
    .map_err(internal("craft update"))?
    .rows_affected();
    Ok(done)
}

async fn update(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(n): Path<i32>,
    Json(p): Json<Edit>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if apply_edit(&app, n, &p).await? == 0 {
        return Err(ApiError::NotFound);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "craft.recipe.edit", &n.to_string(), json!({"rate": p.rate, "shadow": p.shadow, "give_count": p.give_count, "give_item": p.give_item}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct BulkIn {
    n: Vec<i32>,
    rate: Option<i32>,
    shadow: Option<bool>,
}

async fn bulk(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<BulkIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.n.is_empty() || p.n.len() > 3000 || (p.rate.is_none() && p.shadow.is_none()) {
        return Err(ApiError::BadRequest);
    }
    let edit = Edit { rate: p.rate, shadow: p.shadow, give_count: None, give_item: None };
    let mut changed = 0u64;
    for n in &p.n {
        changed += apply_edit(&app, *n, &edit).await?;
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "craft.bulk", &format!("{} rows", p.n.len()), json!({"rate": p.rate, "shadow": p.shadow, "rows": changed}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true, "changed": changed})))
}

#[derive(Deserialize)]
struct Mat {
    id: i32,
    count: i32,
}

#[derive(Deserialize)]
struct CreateIn {
    npc: i16,
    description: String,
    materials: Vec<Mat>,
    give_item: i32,
    give_count: i32,
    rate: i32,
    shadow: bool,
}

async fn create(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<CreateIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !NPCS.iter().any(|n| n.0 == p.npc)
        || p.materials.is_empty()
        || p.materials.len() > 10
        || p.materials.iter().any(|m| m.id <= 0 || !(1..=999).contains(&m.count))
        || !(1..=999).contains(&p.give_count)
        || !(0..=10000).contains(&p.rate)
        || p.description.chars().count() > 120
    {
        return Err(ApiError::BadRequest);
    }
    let mut ids: Vec<i32> = p.materials.iter().map(|m| m.id).collect();
    ids.push(p.give_item);
    ids.sort_unstable();
    ids.dedup();
    let found: i64 = sqlx::query_scalar("SELECT count(*) FROM item WHERE num = ANY($1)")
        .bind(&ids)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("craft item check"))?;
    if found as usize != ids.len() {
        return Err(ApiError::BadRequest);
    }
    let mut sql = String::from("INSERT INTO item_special_sewing (n_index, description");
    for i in 1..=p.materials.len() {
        sql.push_str(&format!(", req_item_id_{i}, req_item_count_{i}"));
    }
    sql.push_str(", give_item_id, give_item_count, success_rate, npc_id, is_notice, is_shadow_success) SELECT COALESCE(MAX(n_index),0)+1, $1");
    let mut k = 2;
    for _ in 0..p.materials.len() {
        sql.push_str(&format!(", ${k}, ${}", k + 1));
        k += 2;
    }
    sql.push_str(&format!(", ${k}, ${}, ${}, ${}, FALSE, ${} FROM item_special_sewing RETURNING n_index", k + 1, k + 2, k + 3, k + 4));
    let mut q = sqlx::query_scalar::<_, i32>(&sql).bind(&p.description);
    for m in &p.materials {
        q = q.bind(m.id).bind(m.count);
    }
    let n = q
        .bind(p.give_item)
        .bind(p.give_count)
        .bind(p.rate)
        .bind(p.npc)
        .bind(p.shadow)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("craft insert"))?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "craft.recipe.add", &n.to_string(), json!({"npc": p.npc, "give": p.give_item, "rate": p.rate}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true, "n": n})))
}

async fn remove(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(n): Path<i32>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let done = sqlx::query("DELETE FROM item_special_sewing WHERE n_index = $1 AND npc_id IN (19073, 31402)")
        .bind(n)
        .execute(&app.pool)
        .await
        .map_err(internal("craft delete"))?
        .rows_affected();
    if done == 0 {
        return Err(ApiError::NotFound);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    audit(&mut tx, &s.actor, "craft.recipe.remove", &n.to_string(), json!({}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_reload(&app.pool).await;
    Ok(Json(json!({"ok": true})))
}
