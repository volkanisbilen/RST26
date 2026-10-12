//! World content endpoints: zone maps (terrain preview from the real SMD files), NPC / monster
//! spawn editor, drop-table helpers (atomic replace, usage lookups).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::OnceLock;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
use ko_protocol::smd::SmdFile;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::legacy::{audit, auth, queue_command, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/zones", get(zones))
        .route("/jstkoadminpanel/api/zone/{id}/info", get(zone_info))
        .route("/jstkoadminpanel/zone/{id}/terrain.png", get(terrain_png))
        .route("/jstkoadminpanel/zone/{id}/image", get(zone_image))
        .route("/jstkoadminpanel/api/spawns", get(spawns).post(spawn_create))
        .route("/jstkoadminpanel/api/spawns/{id}", post(spawn_update).delete(spawn_delete))
        .route("/jstkoadminpanel/api/drops/{sid}/replace", post(drop_replace))
        .route("/jstkoadminpanel/api/drop-usage/{table}", get(drop_usage))
        .route("/jstkoadminpanel/api/group-usage/{group}", get(group_usage))
        .route("/jstkoadminpanel/api/npc-search", get(npc_search))
        .route("/jstkoadminpanel/api/group-next", get(group_next))
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

fn map_dir() -> PathBuf {
    PathBuf::from(std::env::var("MAP_DIR").unwrap_or_else(|_| "./Map".into()))
}
fn map_img_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("MAP_IMG_DIR")
            .unwrap_or_else(|_| "/var/www/jstkoadminpanel2/public/assets/img/maps".into()),
    )
}

struct Terrain {
    width: f32,
    height: f32,
    unit: f32,
    grid: i32,
    png: Vec<u8>,
    warps: Vec<Value>,
    regene: Vec<Value>,
}

fn cache() -> &'static DashMap<String, std::sync::Arc<Terrain>> {
    static C: OnceLock<DashMap<String, std::sync::Arc<Terrain>>> = OnceLock::new();
    C.get_or_init(DashMap::new)
}

async fn zone_row(app: &App, id: i16) -> Result<(String, String), ApiError> {
    let row: Option<(String, String)> = sqlx::query_as("SELECT smd_name, zone_name FROM zone_info WHERE zone_no=$1")
        .bind(id)
        .fetch_optional(&app.pool)
        .await
        .map_err(internal("zone row"))?;
    row.ok_or(ApiError::NotFound)
}

async fn terrain(app: &App, id: i16) -> Result<std::sync::Arc<Terrain>, ApiError> {
    let (smd, _) = zone_row(app, id).await?;
    if let Some(t) = cache().get(&smd) {
        return Ok(t.clone());
    }
    let path = map_dir().join(&smd);
    let key = smd.clone();
    let t = tokio::task::spawn_blocking(move || -> Result<Terrain, String> {
        let f = SmdFile::load(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let n = f.map_size.max(2) as usize;
        let out = n.min(384);
        let (mut min_h, mut max_h) = (f32::MAX, f32::MIN);
        for h in &f.heights {
            min_h = min_h.min(*h);
            max_h = max_h.max(*h);
        }
        let span = (max_h - min_h).max(1.0);
        let mut rgb = vec![0u8; out * out * 3];
        for py in 0..out {
            for px in 0..out {
                // image x = world x, image y = world z
                let gx = px * (n - 1) / (out - 1).max(1);
                let gz = py * (n - 1) / (out - 1).max(1);
                let h = f.heights.get(gx * n + gz).copied().unwrap_or(min_h);
                let t = ((h - min_h) / span).clamp(0.0, 1.0);
                // light from the north-west so slopes read as relief
                let hx = f.heights.get((gx + 1).min(n - 1) * n + gz).copied().unwrap_or(h);
                let hz = f.heights.get(gx * n + (gz + 1).min(n - 1)).copied().unwrap_or(h);
                let shade = ((h - hx) + (h - hz)) / f.unit_dist.max(1.0) * 0.18;
                let v = (t * 0.55 + 0.25 + shade).clamp(0.05, 1.0);
                let i = (py * out + px) * 3;
                rgb[i] = (18.0 + 60.0 * v) as u8;
                rgb[i + 1] = (34.0 + 100.0 * v) as u8;
                rgb[i + 2] = (70.0 + 150.0 * v) as u8;
            }
        }
        let warps = f
            .warps
            .iter()
            .map(|w| json!({"id": w.warp_id, "name": w.name, "dest_zone": w.dest_zone, "dest_x": w.dest_x, "dest_z": w.dest_z, "nation": w.nation}))
            .collect();
        let regene = f.regene_events.iter().map(|r| json!({"x": r.pos_x, "z": r.pos_z})).collect();
        Ok(Terrain {
            width: f.map_width.max(f.map_size as f32 * f.unit_dist),
            height: f.map_height.max(f.map_size as f32 * f.unit_dist),
            unit: f.unit_dist,
            grid: f.map_size,
            png: encode_png(out as u32, out as u32, &rgb),
            warps,
            regene,
        })
    })
    .await
    .map_err(internal("terrain task"))?
    .map_err(|e| {
        tracing::warn!(error = %e, "SMD load failed");
        ApiError::NotFound
    })?;
    let t = std::sync::Arc::new(t);
    cache().insert(key, t.clone());
    Ok(t)
}

fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.to_lowercase().chars() {
        let c = match c {
            'ı' | 'İ' => 'i',
            'ğ' => 'g',
            'ü' => 'u',
            'ş' => 's',
            'ö' => 'o',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('_') && !s.is_empty() {
            s.push('_');
        }
    }
    s.trim_matches('_').to_string()
}

fn image_for(id: i16, name: &str) -> Option<PathBuf> {
    let dir = map_img_dir();
    for stem in [format!("zone_{id}"), slug(name)] {
        for ext in ["jpg", "png"] {
            let p = dir.join(format!("{stem}.{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

async fn zones(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(x ORDER BY zone),'[]'::jsonb) FROM (
            SELECT z.zone_no zone, z.zone_name name, z.smd_name smd,
              (SELECT count(*) FROM npc_spawn s WHERE s.zone_id=z.zone_no) spawns
            FROM zone_info z) x"#,
    )
    .fetch_one(&app.pool)
    .await
    .map_err(internal("zones"))?;
    Ok(Json(v))
}

async fn zone_info(headers: HeaderMap, State(app): State<App>, Path(id): Path<i16>) -> R {
    auth(&headers, false)?;
    let (_, name) = zone_row(&app, id).await?;
    let has_image = image_for(id, &name).is_some();
    match terrain(&app, id).await {
        Ok(t) => Ok(Json(json!({
            "zone": id, "name": name, "width": t.width, "height": t.height, "unit": t.unit, "grid": t.grid,
            "image": has_image, "terrain": true, "warps": t.warps, "regene": t.regene
        }))),
        Err(_) => Ok(Json(json!({"zone": id, "name": name, "width": 2048.0, "height": 2048.0, "unit": 4.0, "grid": 0,
            "image": has_image, "terrain": false, "warps": [], "regene": []}))),
    }
}

async fn terrain_png(State(app): State<App>, Path(id): Path<i16>) -> Response {
    match terrain(&app, id).await {
        Ok(t) => ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=3600")], t.png.clone()).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn zone_image(State(app): State<App>, Path(id): Path<i16>) -> Response {
    let Ok((_, name)) = zone_row(&app, id).await else { return ApiError::NotFound.into_response() };
    match image_for(id, &name) {
        Some(p) => match tokio::fs::read(&p).await {
            Ok(bytes) => {
                let ct = if p.extension().is_some_and(|e| e == "png") { "image/png" } else { "image/jpeg" };
                ([(header::CONTENT_TYPE, ct), (header::CACHE_CONTROL, "public, max-age=3600")], bytes).into_response()
            }
            Err(_) => ApiError::NotFound.into_response(),
        },
        None => ApiError::NotFound.into_response(),
    }
}

// ── PNG (stored deflate: no extra crates) ────────────────────────────────

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for b in data {
        crc = table[((crc ^ *b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for d in data {
        a = (a + *d as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn encode_png(w: u32, h: u32, rgb: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w as usize * 3 + 1) * h as usize);
    for y in 0..h as usize {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * w as usize * 3..(y + 1) * w as usize * 3]);
    }
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    while let Some(c) = chunks.next() {
        z.push(if chunks.peek().is_none() { 1 } else { 0 });
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}

// ── spawns ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ZoneQ {
    zone: Option<i16>,
    q: Option<String>,
    limit: Option<i64>,
}

async fn spawns(headers: HeaderMap, State(app): State<App>, Query(q): Query<ZoneQ>) -> R {
    auth(&headers, false)?;
    let zone = q.zone.ok_or(ApiError::BadRequest)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(x ORDER BY id),'[]'::jsonb) FROM (
            SELECT s.id, s.npc_id npc, s.is_monster monster, COALESCE(n.str_name,'?') name, COALESCE(n.s_level,0) level,
                   s.left_x x, s.top_z z, s.num_npc num, s.spawn_range range, s.regen_time regen, s.direction dir,
                   s.act_type act, s.regen_type regen_type, s.room
            FROM npc_spawn s LEFT JOIN npc_template n ON n.s_sid=s.npc_id AND n.is_monster=s.is_monster
            WHERE s.zone_id=$1 ORDER BY s.id LIMIT 8000) x"#,
    )
    .bind(zone)
    .fetch_one(&app.pool)
    .await
    .map_err(internal("spawns"))?;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct SpawnIn {
    zone: Option<i16>,
    npc: Option<i16>,
    monster: Option<bool>,
    x: Option<i32>,
    z: Option<i32>,
    num: Option<i16>,
    range: Option<i16>,
    regen: Option<i16>,
    dir: Option<i32>,
    act: Option<i16>,
}

async fn npc_exists(app: &App, npc: i16, monster: bool) -> Result<bool, ApiError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM npc_template WHERE s_sid=$1 AND is_monster=$2)")
        .bind(npc)
        .bind(monster)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("npc exists"))
}

async fn spawn_create(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Json(p): Json<SpawnIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let (Some(zone), Some(npc), Some(x), Some(z)) = (p.zone, p.npc, p.x, p.z) else { return Err(ApiError::BadRequest) };
    let monster = p.monster.unwrap_or(true);
    if !(0..=20000).contains(&x) || !(0..=20000).contains(&z) || !npc_exists(&app, npc, monster).await? {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let id: i32 = sqlx::query_scalar(
        "INSERT INTO npc_spawn(zone_id,npc_id,is_monster,act_type,left_x,top_z,num_npc,spawn_range,regen_time,direction) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id",
    )
    .bind(zone).bind(npc).bind(monster).bind(p.act.unwrap_or(if monster { 1 } else { 0 }))
    .bind(x).bind(z).bind(p.num.unwrap_or(1).clamp(1, 200)).bind(p.range.unwrap_or(5).clamp(0, 200))
    .bind(p.regen.unwrap_or(30).clamp(0, 32000)).bind(p.dir.unwrap_or(0))
    .fetch_one(&mut *tx)
    .await
    .map_err(internal("spawn insert"))?;
    audit(&mut tx, &s.actor, "spawn.create", &format!("zone{zone}:{id}"), json!({"npc":npc,"x":x,"z":z}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true,"id":id,"note":"Spawn kaydedildi; oyun sunucusunun yeniden başlatılması gerekir."})))
}

async fn spawn_update(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i32>,
    Json(p): Json<SpawnIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if let Some(npc) = p.npc {
        let monster = match p.monster {
            Some(m) => m,
            None => sqlx::query_scalar("SELECT is_monster FROM npc_spawn WHERE id=$1").bind(id).fetch_optional(&app.pool).await.map_err(internal("spawn read"))?.ok_or(ApiError::NotFound)?,
        };
        if !npc_exists(&app, npc, monster).await? {
            return Err(ApiError::BadRequest);
        }
    }
    if p.x.is_some_and(|v| !(0..=20000).contains(&v)) || p.z.is_some_and(|v| !(0..=20000).contains(&v)) {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let n = sqlx::query("UPDATE npc_spawn SET npc_id=COALESCE($2,npc_id), is_monster=COALESCE($3,is_monster), left_x=COALESCE($4,left_x), top_z=COALESCE($5,top_z), num_npc=COALESCE($6,num_npc), spawn_range=COALESCE($7,spawn_range), regen_time=COALESCE($8,regen_time), direction=COALESCE($9,direction), act_type=COALESCE($10,act_type) WHERE id=$1")
        .bind(id).bind(p.npc).bind(p.monster).bind(p.x).bind(p.z)
        .bind(p.num.map(|v| v.clamp(1, 200))).bind(p.range.map(|v| v.clamp(0, 200))).bind(p.regen.map(|v| v.clamp(0, 32000))).bind(p.dir).bind(p.act)
        .execute(&mut *tx).await.map_err(internal("spawn update"))?.rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    audit(&mut tx, &s.actor, "spawn.update", &id.to_string(), json!({"x":p.x,"z":p.z,"npc":p.npc,"num":p.num,"range":p.range}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

async fn spawn_delete(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(id): Path<i32>,
    Query(q): Query<ZoneQ>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let row: Option<(i16, i16)> = sqlx::query_as("DELETE FROM npc_spawn WHERE id=$1 RETURNING zone_id, npc_id")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal("spawn delete"))?;
    let (zone, npc) = row.ok_or(ApiError::NotFound)?;
    audit(&mut tx, &s.actor, "spawn.delete", &id.to_string(), json!({"zone":zone,"npc":npc,"reason":q.q}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(json!({"ok":true})))
}

async fn npc_search(headers: HeaderMap, State(app): State<App>, Query(q): Query<ZoneQ>) -> R {
    auth(&headers, false)?;
    let term = q.q.unwrap_or_default().trim().to_owned();
    if term.is_empty() {
        return Ok(Json(json!([])));
    }
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(x),'[]'::jsonb) FROM (
            SELECT s_sid id, str_name name, is_monster monster, s_level level, s_pid pid, i_hp_point hp
            FROM npc_template WHERE CAST(s_sid AS TEXT)=$1 OR str_name ILIKE $2
            ORDER BY is_monster DESC, CASE WHEN CAST(s_sid AS TEXT)=$1 THEN 0 ELSE 1 END, str_name LIMIT $3) x"#,
    )
    .bind(&term)
    .bind(format!("%{term}%"))
    .bind(q.limit.unwrap_or(40).clamp(1, 100))
    .fetch_one(&app.pool)
    .await
    .map_err(internal("npc search"))?;
    Ok(Json(v))
}

// ── drop helpers ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ReplaceIn {
    /// exactly 12 (item_or_group_id, raw 0..10000)
    slots: Vec<(i32, i16)>,
    #[serde(default)]
    reason: String,
}

async fn drop_replace(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(sid): Path<i16>,
    Json(p): Json<ReplaceIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if p.slots.len() != 12 || p.slots.iter().any(|(i, pc)| *i < 0 || !(0..=10000).contains(pc)) {
        return Err(ApiError::BadRequest);
    }
    let table: i16 = sqlx::query_scalar("SELECT s_item FROM npc_template WHERE s_sid=$1 AND is_monster=true")
        .bind(sid)
        .fetch_optional(&app.pool)
        .await
        .map_err(internal("drop table"))?
        .ok_or(ApiError::NotFound)?;
    for (id, _) in p.slots.iter().filter(|(i, _)| *i > 0) {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM item WHERE num=$1) OR ($1 BETWEEN 100 AND 99999999 AND EXISTS(SELECT 1 FROM make_item_group WHERE group_num=$1))",
        )
        .bind(*id)
        .fetch_one(&app.pool)
        .await
        .map_err(internal("drop item check"))?;
        if !ok {
            return Err(ApiError::BadRequest);
        }
    }
    let mut cols = vec!["s_index".to_string()];
    let mut vals = vec!["$1".to_string()];
    let mut sets = Vec::new();
    for k in 1..=12 {
        let (a, b) = (format!("item{k:02}"), format!("percent{k:02}"));
        vals.push(format!("${}", 2 * k));
        vals.push(format!("${}", 2 * k + 1));
        sets.push(format!("{a}=EXCLUDED.{a}"));
        sets.push(format!("{b}=EXCLUDED.{b}"));
        cols.push(a);
        cols.push(b);
    }
    let sql = format!(
        "INSERT INTO monster_item({}) VALUES({}) ON CONFLICT(s_index) DO UPDATE SET {}",
        cols.join(","),
        vals.join(","),
        sets.join(",")
    );
    let mut q = sqlx::query(&sql).bind(table);
    for (id, pc) in &p.slots {
        q = q.bind(*id).bind(if *id == 0 { 0 } else { *pc });
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    q.execute(&mut *tx).await.map_err(internal("drop replace"))?;
    audit(&mut tx, &s.actor, "drop.replace", &format!("npc:{sid} table:{table}"), json!({"slots":p.slots,"reason":p.reason}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    queue_command(&app.pool, "reload", json!({"what":"monster_drop_tables"})).await?;
    Ok(Json(json!({"ok":true,"table":table})))
}

async fn drop_usage(headers: HeaderMap, State(app): State<App>, Path(table): Path<i16>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        "SELECT COALESCE(jsonb_agg(jsonb_build_object('sid',s_sid,'name',str_name,'level',s_level) ORDER BY s_sid),'[]'::jsonb) FROM npc_template WHERE is_monster AND s_item=$1",
    )
    .bind(table)
    .fetch_one(&app.pool)
    .await
    .map_err(internal("drop usage"))?;
    Ok(Json(v))
}

async fn group_usage(headers: HeaderMap, State(app): State<App>, Path(group): Path<i32>) -> R {
    auth(&headers, false)?;
    let v: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(jsonb_build_object('sid',n.s_sid,'name',n.str_name,'slot',d.slot,'percent',d.pc) ORDER BY n.s_sid),'[]'::jsonb)
           FROM npc_template n JOIN monster_item m ON m.s_index=n.s_item
           CROSS JOIN LATERAL (VALUES (1,m.item01,m.percent01),(2,m.item02,m.percent02),(3,m.item03,m.percent03),(4,m.item04,m.percent04),
             (5,m.item05,m.percent05),(6,m.item06,m.percent06),(7,m.item07,m.percent07),(8,m.item08,m.percent08),
             (9,m.item09,m.percent09),(10,m.item10,m.percent10),(11,m.item11,m.percent11),(12,m.item12,m.percent12)) d(slot,item_id,pc)
           WHERE n.is_monster AND d.item_id=$1"#,
    )
    .bind(group)
    .fetch_one(&app.pool)
    .await
    .map_err(internal("group usage"))?;
    Ok(Json(v))
}

async fn group_next(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let n: i32 = sqlx::query_scalar("SELECT COALESCE(MAX(group_num),99)+1 FROM make_item_group WHERE group_num BETWEEN 100 AND 99999998")
        .fetch_one(&app.pool)
        .await
        .map_err(internal("group next"))?;
    Ok(Json(json!({ "next": n })))
}
