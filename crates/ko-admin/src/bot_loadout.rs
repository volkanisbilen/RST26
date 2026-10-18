//! "Bot Donanımı" page: manual per-(class, level bracket) corrections for GM-spawned PK bots,
//! so the project owner can pin exact stats/items/skills instead of trusting the game server's
//! procedural generator (`crates/ko-game/src/systems/bot_ai.rs`: `generate_bot_equipment`,
//! `gm_bot_stats`, `select_bot_attack_skill` and the priest/rogue support pools). Every field is
//! optional -- clearing it goes back to the automatic pick. Writes queue a `bot_loadout_reload`
//! admin_command so a running game server picks the change up without a restart (the same
//! pattern the existing PK-bot spawn/clear page uses for the "bot" command).

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

use crate::legacy::{audit, auth, queue_command, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/bot-loadout/catalog/{gm_class}", get(catalog))
        .route(
            "/jstkoadminpanel/api/bot-loadout/{gm_class}/{level_bracket}",
            get(load).post(save_stats),
        )
        .route(
            "/jstkoadminpanel/api/bot-loadout/{gm_class}/{level_bracket}/item",
            post(save_item),
        )
}

type R = Result<Json<Value>, ApiError>;

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

/// `GM_CLASSES` in bot_ai.rs: 1=warrior, 2=rogue, 3=mage, 4=priest, 13=kurian/porutu.
const GM_CLASSES: [i16; 5] = [1, 2, 3, 4, 13];
const LEVEL_BRACKETS: [i16; 3] = [59, 69, 83];
/// The 8 equip_visual gear slots a GM bot wears, as real inventory slot constants
/// (inventory_constants.rs). The 9 cosplay/branding slots are hardcoded server-side, out of
/// scope here.
const GEAR_SLOTS: [i16; 8] = [4, 5, 6, 8, 10, 12, 13, 1]; // BREAST,SHOULDER,RIGHTHAND,LEFTHAND,LEG,GLOVE,FOOT,HEAD
fn slot_label(slot: i16) -> &'static str {
    match slot {
        1 => "Kask (Head)",
        4 => "Göğüs (Breast)",
        5 => "Omuz (Shoulder)",
        6 => "Sağ El (Right Hand)",
        8 => "Sol El (Left Hand / Kalkan)",
        10 => "Bacak (Leg)",
        12 => "Eldiven (Glove)",
        13 => "Bot (Foot)",
        _ => "?",
    }
}

/// Karus-base `magic_num` ids per GM class -- mirrors bot_ai.rs's attack pools
/// (WARRIOR/ROGUE/MAGE/PRIEST_ATTACK_POOL) plus the priest heal/buff/curse pools and the rogue
/// party buffs (ROGUE_SKILL_SWIFT/ROGUE_SKILL_WOLF). Kept in sync by hand: ko-admin has no
/// dependency on the ko-game crate (same reasoning as PK_BOT_ZONES in legacy.rs).
fn skill_pool_for_class(gm_class: i16) -> &'static [(i32, &'static str)] {
    const WARRIOR: &[(i32, &str)] = &[
        (101001, "Attack"), (105505, "Attack"), (105525, "Attack"), (105545, "Attack"),
        (105555, "Attack"), (105557, "Attack"), (106560, "Attack"), (106570, "Attack"),
        (106575, "Attack"), (106580, "Attack"), (106782, "Attack"),
    ];
    const ROGUE: &[(i32, &str)] = &[
        (101001, "Attack (Dagger)"), (108005, "Attack (Dagger)"), (108620, "Attack (Dagger)"),
        (108635, "Attack (Dagger)"), (108640, "Attack (Dagger)"), (108655, "Attack (Dagger)"),
        (108670, "Attack (Dagger)"), (108685, "Attack (Dagger)"),
        (107003, "Attack (Arrow)"), (107500, "Attack (Arrow)"), (107525, "Attack (Arrow)"),
        (107540, "Attack (Arrow)"), (107552, "Attack (Arrow)"), (108552, "Attack (Arrow)"),
        (108570, "Attack (Arrow)"), (108585, "Attack (Arrow)"),
        (108010, "Parti Buff (Swift)"), (108030, "Parti Buff (Strength of Wolf)"),
    ];
    const MAGE: &[(i32, &str)] = &[
        (109001, "Attack"), (109002, "Attack"), (109005, "Attack"), (109007, "Attack"),
        (109503, "Attack"), (109509, "Attack"), (109515, "Attack"), (109518, "Attack"),
        (109527, "Attack"), (109533, "Attack"), (109535, "Attack"), (109539, "Attack"),
        (109542, "Attack"), (109543, "Attack"), (109545, "Attack"), (109551, "Attack"),
        (109556, "Attack"), (109557, "Attack"), (110542, "Attack"), (110543, "Attack"),
        (110545, "Attack"), (110551, "Attack"), (110556, "Attack"), (110557, "Attack"),
        (110560, "Attack"), (110570, "Attack"), (110571, "Attack"), (110572, "Attack"),
        (110575, "Attack"),
    ];
    const PRIEST: &[(i32, &str)] = &[
        (101001, "Attack"), (111511, "Attack"), (111520, "Attack"), (111542, "Attack"),
        (111551, "Attack"), (112520, "Attack"), (112802, "Attack"), (112815, "Attack"),
        (112500, "Heal"), (112509, "Heal"), (112518, "Heal"), (112527, "Heal"), (112536, "Heal"), (112545, "Heal"),
        (112603, "AC Buff"), (112612, "AC Buff"), (112621, "AC Buff"), (112630, "AC Buff"),
        (112639, "AC Buff"), (112651, "AC Buff"), (112660, "AC Buff"), (112674, "AC Buff"),
        (112606, "HP Buff"), (112615, "HP Buff"), (112624, "HP Buff"), (112633, "HP Buff"),
        (112642, "HP Buff"), (112655, "HP Buff"), (112657, "HP Buff"), (112670, "HP Buff"), (112675, "HP Buff"),
        (112703, "Curse"), (112724, "Curse"), (112745, "Curse"), (112760, "Curse"),
    ];
    match gm_class {
        1 | 13 => WARRIOR,
        2 => ROGUE,
        3 => MAGE,
        4 => PRIEST,
        _ => &[],
    }
}

fn valid_class(c: i16) -> bool {
    GM_CLASSES.contains(&c)
}
fn valid_bracket(b: i16) -> bool {
    LEVEL_BRACKETS.contains(&b)
}

async fn catalog(headers: HeaderMap, State(app): State<App>, Path(gm_class): Path<i16>) -> R {
    auth(&headers, false)?;
    if !valid_class(gm_class) {
        return Err(ApiError::BadRequest);
    }
    let pool = skill_pool_for_class(gm_class);
    let ids: Vec<i32> = pool.iter().map(|(id, _)| *id).collect();
    let rows = sqlx::query("SELECT magic_num, COALESCE(en_name,'') AS en_name, COALESCE(kr_name,'') AS kr_name FROM magic WHERE magic_num = ANY($1)")
        .bind(&ids)
        .fetch_all(&app.pool)
        .await
        .map_err(internal("bot-loadout catalog"))?;
    let names: std::collections::HashMap<i32, String> = rows
        .into_iter()
        .map(|r| {
            let id: i32 = r.try_get("magic_num").unwrap_or(0);
            let en: String = r.try_get("en_name").unwrap_or_default();
            let kr: String = r.try_get("kr_name").unwrap_or_default();
            (id, if !en.is_empty() { en } else { kr })
        })
        .collect();
    let skills: Vec<Value> = pool
        .iter()
        .map(|(id, group)| {
            let name = names.get(id).cloned().unwrap_or_default();
            json!({"id":id,"group":group,"name": if name.is_empty() { format!("#{id}") } else { name }})
        })
        .collect();
    Ok(Json(json!({"skills": skills})))
}

async fn load(headers: HeaderMap, State(app): State<App>, Path((gm_class, level_bracket)): Path<(i16, i16)>) -> R {
    auth(&headers, false)?;
    if !valid_class(gm_class) || !valid_bracket(level_bracket) {
        return Err(ApiError::BadRequest);
    }
    let stats_row = sqlx::query("SELECT str_stat,sta_stat,dex_stat,int_stat,cha_stat,skills FROM bot_loadout_override WHERE gm_class=$1 AND level_bracket=$2")
        .bind(gm_class).bind(level_bracket).fetch_optional(&app.pool).await.map_err(internal("bot-loadout load"))?;
    let (stats, skills) = match &stats_row {
        Some(r) => {
            let skills_json: Value = r.try_get("skills").unwrap_or_else(|_| json!([]));
            (
                json!({
                    "str": r.try_get::<Option<i16>,_>("str_stat").ok().flatten(),
                    "sta": r.try_get::<Option<i16>,_>("sta_stat").ok().flatten(),
                    "dex": r.try_get::<Option<i16>,_>("dex_stat").ok().flatten(),
                    "int": r.try_get::<Option<i16>,_>("int_stat").ok().flatten(),
                    "cha": r.try_get::<Option<i16>,_>("cha_stat").ok().flatten(),
                }),
                skills_json,
            )
        }
        None => (json!({"str":null,"sta":null,"dex":null,"int":null,"cha":null}), json!([])),
    };
    let item_rows = sqlx::query("SELECT i.slot,i.item_id,i.durability,COALESCE(it.str_name,'') AS name,COALESCE(it.item_icon_id1,0)::int AS icon FROM bot_loadout_override_item i LEFT JOIN item it ON it.num=i.item_id WHERE i.gm_class=$1 AND i.level_bracket=$2")
        .bind(gm_class).bind(level_bracket).fetch_all(&app.pool).await.map_err(internal("bot-loadout items"))?;
    let items: Vec<Value> = item_rows.into_iter().map(|r| json!({
        "slot": r.try_get::<i16,_>("slot").unwrap_or(0),
        "slot_label": slot_label(r.try_get::<i16,_>("slot").unwrap_or(0)),
        "item_id": r.try_get::<i64,_>("item_id").unwrap_or(0),
        "durability": r.try_get::<i16,_>("durability").unwrap_or(0),
        "name": r.try_get::<String,_>("name").unwrap_or_default(),
        "icon": r.try_get::<i32,_>("icon").unwrap_or(0),
    })).collect();
    Ok(Json(json!({"stats": stats, "skills": skills, "items": items, "slots": GEAR_SLOTS})))
}

#[derive(Deserialize)]
struct StatsIn {
    str_stat: Option<i16>,
    sta_stat: Option<i16>,
    dex_stat: Option<i16>,
    int_stat: Option<i16>,
    cha_stat: Option<i16>,
    #[serde(default)]
    skills: Vec<i32>,
}

fn stat_ok(v: Option<i16>) -> bool {
    v.is_none_or(|v| (1..=255).contains(&v))
}

async fn save_stats(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path((gm_class, level_bracket)): Path<(i16, i16)>,
    Json(p): Json<StatsIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid_class(gm_class)
        || !valid_bracket(level_bracket)
        || !stat_ok(p.str_stat)
        || !stat_ok(p.sta_stat)
        || !stat_ok(p.dex_stat)
        || !stat_ok(p.int_stat)
        || !stat_ok(p.cha_stat)
        || p.skills.len() > 64
    {
        return Err(ApiError::BadRequest);
    }
    let allowed: Vec<i32> = skill_pool_for_class(gm_class).iter().map(|(id, _)| *id).collect();
    if p.skills.iter().any(|id| !allowed.contains(id)) {
        return Err(ApiError::BadRequest);
    }
    let skills_json = json!(p.skills);
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    sqlx::query(
        "INSERT INTO bot_loadout_override(gm_class,level_bracket,str_stat,sta_stat,dex_stat,int_stat,cha_stat,skills,updated_at) \
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,NOW()) \
         ON CONFLICT(gm_class,level_bracket) DO UPDATE SET \
         str_stat=EXCLUDED.str_stat,sta_stat=EXCLUDED.sta_stat,dex_stat=EXCLUDED.dex_stat,int_stat=EXCLUDED.int_stat,\
         cha_stat=EXCLUDED.cha_stat,skills=EXCLUDED.skills,updated_at=NOW()",
    )
    .bind(gm_class).bind(level_bracket).bind(p.str_stat).bind(p.sta_stat).bind(p.dex_stat).bind(p.int_stat).bind(p.cha_stat).bind(&skills_json)
    .execute(&mut *tx).await.map_err(internal("bot-loadout save stats"))?;
    audit(&mut tx, &s.actor, "bot_loadout.stats.save", &format!("{gm_class}:{level_bracket}"),
        json!({"stats":{"str":p.str_stat,"sta":p.sta_stat,"dex":p.dex_stat,"int":p.int_stat,"cha":p.cha_stat},"skills":p.skills}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_command(&app.pool, "bot_loadout_reload", json!({})).await?;
    Ok(Json(json!({"ok":true,"reload_id":id})))
}

#[derive(Deserialize)]
struct ItemIn {
    slot: i16,
    item_id: i64,
    #[serde(default = "default_durability")]
    durability: i16,
}
fn default_durability() -> i16 {
    100
}

async fn save_item(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path((gm_class, level_bracket)): Path<(i16, i16)>,
    Json(p): Json<ItemIn>,
) -> R {
    let (_, s) = auth(&headers, true)?;
    if !valid_class(gm_class)
        || !valid_bracket(level_bracket)
        || !GEAR_SLOTS.contains(&p.slot)
        || p.item_id < 0
        || !(0..=32767).contains(&p.durability)
    {
        return Err(ApiError::BadRequest);
    }
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    if p.item_id == 0 {
        sqlx::query("DELETE FROM bot_loadout_override_item WHERE gm_class=$1 AND level_bracket=$2 AND slot=$3")
            .bind(gm_class).bind(level_bracket).bind(p.slot)
            .execute(&mut *tx).await.map_err(internal("bot-loadout clear item"))?;
    } else {
        if !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM item WHERE num=$1)")
            .bind(p.item_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(internal("item lookup"))?
        {
            return Err(ApiError::BadRequest);
        }
        sqlx::query(
            "INSERT INTO bot_loadout_override_item(gm_class,level_bracket,slot,item_id,durability) VALUES($1,$2,$3,$4,$5) \
             ON CONFLICT(gm_class,level_bracket,slot) DO UPDATE SET item_id=EXCLUDED.item_id,durability=EXCLUDED.durability",
        )
        .bind(gm_class).bind(level_bracket).bind(p.slot).bind(p.item_id).bind(p.durability)
        .execute(&mut *tx).await.map_err(internal("bot-loadout save item"))?;
    }
    audit(&mut tx, &s.actor, "bot_loadout.item.save", &format!("{gm_class}:{level_bracket}:{}", p.slot),
        json!({"item_id":p.item_id,"durability":p.durability}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    let id = queue_command(&app.pool, "bot_loadout_reload", json!({})).await?;
    Ok(Json(json!({"ok":true,"reload_id":id})))
}
