//! Event registry shared by the web panel and the automatic schedule.
//!
//! Every event is started and stopped through the same GM command handlers the in-game
//! `+command`s use (see `handler::operator::run_event_command`), so a panel click, a schedule
//! entry and a GM command always behave identically.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use ko_db::DbPool;
use parking_lot::Mutex;
use sqlx::Row;
use tracing::{info, warn};

use crate::handler::operator::{run_event_command, VirtualGm};
use crate::world::WorldState;

/// One event the panel can start, stop and schedule.
pub struct EventDef {
    pub key: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    /// GM command that opens the event (registration window or the event itself).
    pub start: &'static str,
    /// GM command that stops it ("" = it ends by itself).
    pub stop: &'static str,
    /// GM command that skips the registration and starts at once ("" = none).
    pub start_now: &'static str,
    /// Label of the optional argument ("" = no argument).
    pub arg_label: &'static str,
    pub arg_required: bool,
    /// Registration windows share one client window: only one of them may be open.
    pub shares_window: bool,
}

macro_rules! ev {
    ($key:expr, $name:expr, $group:expr, $start:expr, $stop:expr, $now:expr, $label:expr, $req:expr, $win:expr) => {
        EventDef {
            key: $key,
            name: $name,
            group: $group,
            start: $start,
            stop: $stop,
            start_now: $now,
            arg_label: $label,
            arg_required: $req,
            shares_window: $win,
        }
    };
}

pub const EVENTS: &[EventDef] = &[
    ev!("manes", "Manes Survival", "Kayıtlı etkinlik", "manesopen", "manesclose", "manesstart", "", false, true),
    ev!("ultima", "Ultima'nın Zindanı", "Kayıtlı etkinlik", "ultimaopen", "ultimaend", "ultimastart", "Kayıt süresi (dk)", false, true),
    ev!("rift", "Full Moon Rift", "Kayıtlı etkinlik", "riftreg", "riftend", "riftstart", "Kayıt süresi (dk)", false, true),
    ev!("golem", "Ateş & Buz (Golem Savaşı)", "Açık dünya", "golemopen", "golemend", "", "", false, false),
    ev!("bdw", "Border Defence War", "Tapınak etkinliği", "borderopen", "borderclose", "borderstart", "", false, false),
    ev!("chaos", "Chaos Expansion", "Tapınak etkinliği", "chaosopen", "chaosclose", "chaosstart", "", false, false),
    ev!("juraid", "Juraid Mountain", "Tapınak etkinliği", "juraidopen", "juraidclose", "juraidstart", "", false, false),
    ev!("utc", "Under The Castle", "Zindan", "utcopen", "utcclose", "", "Süre (dk, boş = 180)", false, false),
    ev!("ft", "Forgotten Temple", "Zindan", "ftopen", "ftclose", "", "", false, false),
    ev!("bifrost", "Bifrost", "Açık dünya", "beefopen", "beefclose", "", "", false, false),
    ev!("war1", "Ulus Savaşı — Napies Gorge", "Savaş", "open1", "close", "", "", false, false),
    ev!("war2", "Ulus Savaşı — Alseids Prairie", "Savaş", "open2", "close", "", "", false, false),
    ev!("war3", "Ulus Savaşı — Nieds Triangle", "Savaş", "open3", "close", "", "", false, false),
    ev!("war4", "Ulus Savaşı — Nereids Island", "Savaş", "open4", "close", "", "", false, false),
    ev!("war5", "Ulus Savaşı — Zipang", "Savaş", "open5", "close", "", "", false, false),
    ev!("war6", "Ulus Savaşı — Oreads", "Savaş", "open6", "close", "", "", false, false),
    ev!("snow", "Kar Savaşı", "Savaş", "snow", "close", "", "", false, false),
    ev!("lottery", "Çekiliş (Lottery)", "Diğer", "lottery", "lotteryclose", "", "Çekiliş ID", true, false),
    ev!("cr", "Collection Race", "Diğer", "cropen", "crclose", "", "Etkinlik ID", true, false),
    ev!("attendance", "Günlük Giriş (Attendance)", "İstemci etkinliği", "attendanceopen", "attendanceclose", "", "", false, false),
    ev!("roulette", "Rulet", "İstemci etkinliği", "rouletteopen", "rouletteclose", "", "", false, false),
    ev!("puzzle", "Yapboz (Jigsaw)", "İstemci etkinliği", "puzzleopen", "puzzleclose", "", "", false, false),
    ev!("coin", "Coin Etkinliği", "İstemci etkinliği", "coinopen", "coinclose", "", "", false, false),
    ev!("marble", "Bilye (Marble)", "İstemci etkinliği", "marbleopen", "marbleclose", "", "", false, false),
];

pub fn def(key: &str) -> Option<&'static EventDef> {
    EVENTS.iter().find(|e| e.key == key)
}

async fn run_command(
    world: &Arc<WorldState>,
    pool: &DbPool,
    command: &str,
    arg: &str,
) -> Result<String, String> {
    let args: Vec<&str> = arg.split_whitespace().collect();
    let mut gm = VirtualGm::new(world.clone(), pool.clone());
    match run_event_command(&mut gm, command, &args).await {
        Ok(true) => Ok(gm.lines.join(" | ")),
        Ok(false) => Err(format!("unknown event command {command}")),
        Err(e) => Err(e.to_string()),
    }
}

/// Open (or, with `now`, start at once) an event.
pub async fn start(
    world: &Arc<WorldState>,
    pool: &DbPool,
    key: &str,
    arg: &str,
    now: bool,
) -> Result<String, String> {
    let d = def(key).ok_or_else(|| format!("unknown event {key}"))?;
    if d.arg_required && arg.trim().is_empty() {
        return Err(format!("{}: {} gerekli", d.name, d.arg_label));
    }
    let command = if now && !d.start_now.is_empty() {
        d.start_now
    } else {
        d.start
    };
    // A registration window is the client's only one: do not open it on top of another.
    if d.shares_window && command == d.start {
        for other in EVENTS.iter().filter(|o| o.shares_window && o.key != d.key) {
            let (state, _) = status(world, pool, other.key).await;
            if state != "idle" {
                return Err(format!("{} sürerken {} açılamaz", other.name, d.name));
            }
        }
    }
    run_command(world, pool, command, arg).await
}

pub async fn stop(world: &Arc<WorldState>, pool: &DbPool, key: &str) -> Result<String, String> {
    let d = def(key).ok_or_else(|| format!("unknown event {key}"))?;
    if d.stop.is_empty() {
        return Err(format!("{} kendiliğinden biter", d.name));
    }
    run_command(world, pool, d.stop, "").await
}

/// `("idle" | "registration" | "active", detail)`.
pub async fn status(world: &Arc<WorldState>, pool: &DbPool, key: &str) -> (String, String) {
    let out = |s: &str, d: String| (s.to_string(), d);
    match key {
        "manes" => {
            let m = &world.manes_survival_manager;
            if m.is_active() {
                out("active", format!("{} sn kaldı, {} oyuncu", m.remaining_seconds(), m.participant_count()))
            } else if m.is_registration_open() {
                out("registration", format!("{} kayıtlı, {} sn", m.participant_count(), m.registration_remaining_seconds()))
            } else {
                out("idle", String::new())
            }
        }
        "ultima" => match crate::handler::ultima::phase_name() {
            "registration" => out("registration", String::new()),
            "running" => out("active", String::new()),
            _ => out("idle", String::new()),
        },
        "rift" => {
            let rooms = world
                .dd_rooms()
                .iter()
                .filter(|r| r.is_started.load(Ordering::Relaxed))
                .count();
            if rooms > 0 {
                out("active", format!("{rooms} oda"))
            } else if crate::handler::rift_entry::registration_open() {
                out("registration", String::new())
            } else {
                out("idle", String::new())
            }
        }
        "golem" => {
            if crate::handler::golem_war::is_active() {
                out("active", String::new())
            } else {
                out("idle", String::new())
            }
        }
        "bdw" | "chaos" | "juraid" => {
            let id = match key {
                "bdw" => crate::systems::event_room::TempleEventType::BorderDefenceWar as i16,
                "chaos" => crate::systems::event_room::TempleEventType::ChaosDungeon as i16,
                _ => crate::systems::event_room::TempleEventType::JuraidMountain as i16,
            };
            let (active, running) = world
                .event_room_manager
                .read_temple_event(|s| (s.active_event, s.is_active));
            if active != id {
                out("idle", String::new())
            } else if running {
                out("active", String::new())
            } else {
                out("registration", String::new())
            }
        }
        "utc" => {
            if world
                .under_the_castle_state()
                .is_active
                .load(Ordering::Relaxed)
            {
                out("active", String::new())
            } else {
                out("idle", String::new())
            }
        }
        "ft" => {
            if world
                .forgotten_temple_state()
                .is_active
                .load(Ordering::Relaxed)
            {
                out("active", String::new())
            } else {
                out("idle", String::new())
            }
        }
        "bifrost" => {
            if world.get_beef_event().is_active {
                out("active", String::new())
            } else {
                out("idle", String::new())
            }
        }
        k if k.starts_with("war") => {
            let n: u8 = k[3..].parse().unwrap_or(0);
            let st = world.get_battle_state();
            if st.is_nation_battle() && st.battle_zone == n {
                out("active", String::new())
            } else {
                out("idle", String::new())
            }
        }
        "snow" => {
            let st = world.get_battle_state();
            if st.battle_open == crate::systems::war::SNOW_BATTLE {
                out("active", String::new())
            } else {
                out("idle", String::new())
            }
        }
        "lottery" => {
            if world.lottery_process().read().lottery_start {
                out("active", String::new())
            } else {
                out("idle", String::new())
            }
        }
        "cr" => {
            let cr = world.collection_race_event();
            let cr = cr.read();
            if cr.is_active {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as u32)
                    .unwrap_or(0);
                let left = cr.event_end_time.saturating_sub(now);
                out("active", format!("{} · {} sn kaldı", cr.event_name, left))
            } else {
                out("idle", String::new())
            }
        }
        "attendance" | "roulette" | "puzzle" | "coin" | "marble" => {
            let db_key = if key == "puzzle" { "jigsaw" } else { key };
            let repo = ko_db::repositories::native_events::NativeEventsRepository::new(pool);
            match repo.is_active(db_key).await {
                Ok(true) => out("active", String::new()),
                _ => out("idle", String::new()),
            }
        }
        _ => out("idle", String::new()),
    }
}

/// Writes the state of every event for the panel (one statement).
pub async fn publish_states(world: &Arc<WorldState>, pool: &DbPool) {
    let n = EVENTS.len();
    let mut keys = Vec::with_capacity(n);
    let mut states = Vec::with_capacity(n);
    let mut details = Vec::with_capacity(n);
    let mut names = Vec::with_capacity(n);
    let mut groups = Vec::with_capacity(n);
    let mut labels = Vec::with_capacity(n);
    let mut required = Vec::with_capacity(n);
    let mut has_now = Vec::with_capacity(n);
    let mut can_stop = Vec::with_capacity(n);
    let mut order = Vec::with_capacity(n);
    for (i, e) in EVENTS.iter().enumerate() {
        let (s, d) = status(world, pool, e.key).await;
        keys.push(e.key.to_string());
        states.push(s);
        details.push(d);
        names.push(e.name.to_string());
        groups.push(e.group.to_string());
        labels.push(e.arg_label.to_string());
        required.push(e.arg_required);
        has_now.push(!e.start_now.is_empty());
        can_stop.push(!e.stop.is_empty());
        order.push(i as i32);
    }
    let _ = sqlx::query(
        "INSERT INTO admin_event_state(event_key,state,detail,name,grp,arg_label,arg_required,has_now,can_stop,sort_no,updated_at) \
         SELECT k,s,d,n,g,l,r,w,c,o,NOW() FROM unnest($1::text[],$2::text[],$3::text[],$4::text[],$5::text[],$6::text[],$7::bool[],$8::bool[],$9::bool[],$10::int[]) \
              AS t(k,s,d,n,g,l,r,w,c,o) \
         ON CONFLICT(event_key) DO UPDATE SET state=EXCLUDED.state, detail=EXCLUDED.detail, name=EXCLUDED.name, \
              grp=EXCLUDED.grp, arg_label=EXCLUDED.arg_label, arg_required=EXCLUDED.arg_required, \
              has_now=EXCLUDED.has_now, can_stop=EXCLUDED.can_stop, sort_no=EXCLUDED.sort_no, updated_at=NOW()",
    )
    .bind(&keys)
    .bind(&states)
    .bind(&details)
    .bind(&names)
    .bind(&groups)
    .bind(&labels)
    .bind(&required)
    .bind(&has_now)
    .bind(&can_stop)
    .bind(&order)
    .execute(pool)
    .await;
}

// ── Automatic schedule ──────────────────────────────────────────────────

/// A freshly started event reports "registration" or "active"; "idle" means the command refused.
fn counts_as_idle_after_start(state: &str) -> bool {
    state == "idle"
}

static LAST_MINUTE: AtomicI64 = AtomicI64::new(0);

fn fired() -> &'static Mutex<HashMap<(i64, i64), ()>> {
    static F: OnceLock<Mutex<HashMap<(i64, i64), ()>>> = OnceLock::new();
    F.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Called every second; once per minute the schedule table is checked (server local time,
/// the same clock the native event schedule uses).
pub fn tick(world: &Arc<WorldState>) {
    use chrono::{Datelike, Local, TimeZone, Timelike};
    let now = Local::now();
    let minute_key = now.timestamp() / 60;
    if LAST_MINUTE.swap(minute_key, Ordering::Relaxed) == minute_key {
        return;
    }
    let Some(pool) = world.db_pool().cloned() else {
        return;
    };
    let weekday_bit = 1i16 << now.weekday().num_days_from_sunday();
    let (hour, minute) = (now.hour() as i16, now.minute() as i16);
    let _ = Local.timestamp_opt(0, 0);
    let world = world.clone();
    tokio::spawn(async move {
        let rows = match sqlx::query(
            "SELECT id, event_key, arg, duration_min FROM event_auto_schedule \
             WHERE enabled AND hour=$1 AND minute=$2 AND (days & $3) <> 0",
        )
        .bind(hour)
        .bind(minute)
        .bind(weekday_bit)
        .fetch_all(&pool)
        .await
        {
            Ok(r) => r,
            Err(_) => return, // table not created yet
        };
        for row in rows {
            let id: i32 = row.try_get("id").unwrap_or(0);
            let key: String = row.try_get("event_key").unwrap_or_default();
            let arg: String = row.try_get("arg").unwrap_or_default();
            let duration: i32 = row.try_get("duration_min").unwrap_or(0);
            {
                let mut f = fired().lock();
                if f.insert((i64::from(id), minute_key), ()).is_some() {
                    continue;
                }
                if f.len() > 200 {
                    f.retain(|(_, m), _| *m >= minute_key - 5);
                }
            }
            let result = start(&world, &pool, &key, &arg, false).await;
            let (mut ok, mut text) = match &result {
                Ok(t) => (true, t.clone()),
                Err(e) => (false, e.clone()),
            };
            // The GM commands answer "ok" for refusals too ("already active", "table empty", ...):
            // an event whose state is still idle did not start, whatever the command said.
            if ok && counts_as_idle_after_start(status(&world, &pool, &key).await.0.as_str()) {
                ok = false;
                text = format!("event did not start ({})", text.trim());
            }
            if ok {
                info!(event = %key, "scheduled event started: {text}");
            } else {
                warn!(event = %key, "scheduled event not started: {text}");
            }
            let _ = sqlx::query(
                "UPDATE event_auto_schedule SET last_run=NOW(), last_result=$2 WHERE id=$1",
            )
            .bind(id)
            .bind(format!("{}: {}", if ok { "başladı" } else { "başlamadı" }, text))
            .execute(&pool)
            .await;
            if ok && duration > 0 {
                let (w, p, k) = (world.clone(), pool.clone(), key.clone());
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(duration as u64 * 60)).await;
                    match stop(&w, &p, &k).await {
                        Ok(t) => info!(event = %k, "scheduled event stopped: {t}"),
                        Err(e) => warn!(event = %k, "scheduled stop failed: {e}"),
                    }
                });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_that_leaves_the_event_idle_is_not_a_start() {
        assert!(counts_as_idle_after_start("idle"));
        assert!(!counts_as_idle_after_start("registration"));
        assert!(!counts_as_idle_after_start("active"));
    }

    #[test]
    fn registry_keys_are_unique_and_short() {
        let mut keys: Vec<&str> = EVENTS.iter().map(|e| e.key).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len());
        assert!(EVENTS.iter().all(|e| e.key.len() <= 20 && e.name.len() <= 60));
    }

    #[test]
    fn every_registry_command_is_handled_by_the_dispatcher() {
        let src = include_str!("handler/operator.rs");
        let body_start = src.find("pub(crate) async fn run_event_command").unwrap();
        let body = &src[body_start..];
        for e in EVENTS {
            for cmd in [e.start, e.stop, e.start_now] {
                if cmd.is_empty() {
                    continue;
                }
                assert!(
                    body.contains(&format!("\"{cmd}\" =>")),
                    "event {} uses command {cmd} that run_event_command does not handle",
                    e.key
                );
            }
        }
    }
}
