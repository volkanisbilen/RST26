//! Executes commands queued by the standalone `ko-admin` panel (`admin_command` table) and
//! publishes a heartbeat so the panel can tell whether the game server is up.

use std::sync::Arc;
use std::time::Duration;

use ko_db::DbPool;
use sqlx::Row;
use tracing::{info, warn};

use crate::world::WorldState;

pub fn start(world: Arc<WorldState>, pool: DbPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(3));
        loop {
            tick.tick().await;
            if sqlx::query("INSERT INTO admin_heartbeat(id,at) VALUES(1,NOW()) ON CONFLICT(id) DO UPDATE SET at=NOW()")
                .execute(&pool)
                .await
                .is_err()
            {
                // table not created yet (ko-admin creates it on first start)
                continue;
            }
            crate::event_control::publish_states(&world, &pool).await;
            // commands older than ten minutes are stale (the server was down when they were queued)
            let _ = sqlx::query("UPDATE admin_command SET status='expired' WHERE status='pending' AND created_at < NOW() - INTERVAL '10 minutes'")
                .execute(&pool)
                .await;
            let rows = match sqlx::query("UPDATE admin_command SET status='running' WHERE id IN (SELECT id FROM admin_command WHERE status='pending' ORDER BY id LIMIT 10) RETURNING id, command, args")
                .fetch_all(&pool)
                .await
            {
                Ok(r) => r,
                Err(_) => continue,
            };
            for row in rows {
                let id: i64 = row.try_get("id").unwrap_or(0);
                let command: String = row.try_get("command").unwrap_or_default();
                let args: serde_json::Value = row.try_get("args").unwrap_or_default();
                let (status, result) = match run(&world, &pool, &command, &args).await {
                    Ok(msg) => ("done", msg),
                    Err(msg) => ("failed", msg),
                };
                info!(id, command, status, result, "admin command");
                let _ = sqlx::query("UPDATE admin_command SET status=$2, result=$3 WHERE id=$1")
                    .bind(id)
                    .bind(status)
                    .bind(result)
                    .execute(&pool)
                    .await;
            }
        }
    })
}

async fn run(world: &Arc<WorldState>, pool: &DbPool, command: &str, args: &serde_json::Value) -> Result<String, String> {
    let s = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    match command {
        "reload" => match s("what").as_str() {
            "monster_drop_tables" | "drops" => world
                .reload_monster_drop_tables(pool)
                .await
                .map(|n| format!("{n} drop rows"))
                .map_err(|e| e.to_string()),
            "make_item_groups" | "groups" => world
                .reload_make_item_groups(pool)
                .await
                .map(|n| format!("{n} group rows"))
                .map_err(|e| e.to_string()),
            "quest_tables" | "quest" => world
                .reload_quest_tables(pool)
                .await
                .map(|n| format!("{n} quest helper rows"))
                .map_err(|e| e.to_string()),
            "npc_spawn" | "npc_spawns" => world
                .reload_npc_spawns(pool)
                .await
                .map(|(a, r, c, s)| format!("npc_spawn: +{a} added, -{r} removed, ~{c} changed, {s} skipped"))
                .map_err(|e| e.to_string()),
            "rankings" => {
                world.reload_user_rankings(pool).await;
                Ok("user rankings reloaded".into())
            }
            "event_schedule" => {
                world.reload_event_schedule(pool).await;
                Ok("event schedule reloaded".into())
            }
            "all" => {
                let a = world.reload_monster_drop_tables(pool).await.map_err(|e| e.to_string())?;
                let b = world.reload_make_item_groups(pool).await.map_err(|e| e.to_string())?;
                let c = world.reload_quest_tables(pool).await.map_err(|e| e.to_string())?;
                world.reload_event_schedule(pool).await;
                Ok(format!("drops {a}, groups {b}, quest {c}, events reloaded"))
            }
            "event_rewards" | "chaos_rewards" | "daily_quests" | "cinderella" | "zone_rewards" | "collection_race" | "reward_rules" | "pus" | "send_messages" | "item_exchange" | "item_upgrade_settings" | "item_special_sewing" => {
                world.reload_reward_tables(pool, &s("what")).await
            }
            other => Err(format!("unknown reload target {other}")),
        },
        "kick" => {
            let name = s("name");
            match world.find_session_by_name(&name) {
                Some(id) => {
                    world.kick_session_for_duplicate(id).await;
                    Ok(format!("{name} kicked"))
                }
                None => Ok(format!("{name} is not online")),
            }
        }
        // Web "Karakter Kurtar" with the Moradon option: kick the character if it is online (that saves it where it
        // stands), then put it at the Moradon start point of its nation so a character stuck in a zone can log in again.
        "rescue_move" => {
            let name = s("name");
            if let Some(id) = world.find_session_by_name(&name) {
                world.kick_session_for_duplicate(id).await;
            }
            let nation: Option<i16> = sqlx::query_scalar("SELECT nation FROM userdata WHERE lower(str_user_id)=lower($1)")
                .bind(&name)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;
            let Some(nation) = nation else { return Ok(format!("{name} not found")) };
            let (zone, x, z) = crate::handler::gamestart::get_moradon_coords(nation as u8, world).unwrap_or((21, 267.0, 303.0));
            let n = sqlx::query("UPDATE userdata SET zone=$2, px=$3, py=0, pz=$4 WHERE lower(str_user_id)=lower($1)")
                .bind(&name)
                .bind(zone as i16)
                .bind((x * 100.0) as i32)
                .bind((z * 100.0) as i32)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?
                .rows_affected();
            Ok(format!("{name} moved to zone {zone} ({x:.0},{z:.0}), rows {n}"))
        }
        "kick_account" => {
            let account = s("account");
            let names: Vec<String> = sqlx::query_scalar(
                "SELECT str_char_id FROM currentuser WHERE lower(str_account_id)=lower($1)",
            )
            .bind(&account)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            let mut n = 0;
            for name in names {
                if let Some(id) = world.find_session_by_name(&name) {
                    world.kick_session_for_duplicate(id).await;
                    n += 1;
                }
            }
            Ok(format!("{n} session(s) kicked"))
        }
        "announce" => {
            let msg = s("message");
            if msg.trim().is_empty() || msg.len() > 200 {
                return Err("empty or too long".into());
            }
            world.broadcast_to_all(
                Arc::new(crate::systems::timed_notice::build_notice_packet(8, &msg)),
                None,
            );
            Ok("announced".into())
        }
        "event" => {
            let key = s("key");
            let arg = s("arg");
            match s("action").as_str() {
                "start" => crate::event_control::start(world, pool, &key, &arg, false).await,
                "startnow" => crate::event_control::start(world, pool, &key, &arg, true).await,
                "stop" => crate::event_control::stop(world, pool, &key).await,
                other => Err(format!("unknown event action {other}")),
            }
        }
        "control" => {
            let zone = args.get("zone").and_then(|v| v.as_u64()).map(|z| z as u8);
            crate::admin_web::execute_control(world, &s("kind"), &s("action"), zone)
                .map(|d| d.to_string())
                .map_err(|e| e.to_string())
        }
        "bot" => {
            let zone_id = args.get("zone").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            match s("action").as_str() {
                "spawn" => {
                    let count_per_nation = args
                        .get("count_per_nation")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize);
                    let level = args.get("level").and_then(|v| v.as_u64()).map(|v| v as u8);
                    crate::systems::bot_ai::spawn_pk_bot_wave(world, zone_id, count_per_nation, level)
                        .map(|r| {
                            format!(
                                "Spawned {} PK bots in zone {} ({} per nation, level {})",
                                r.spawned, r.zone_id, r.count_per_nation, r.level
                            )
                        })
                }
                "clear" => {
                    let removed = crate::systems::bot_ai::despawn_gm_pk_bots_in_zone(world, zone_id);
                    Ok(format!("Removed {removed} PK bot(s) from zone {zone_id}"))
                }
                other => Err(format!("unknown bot action {other}")),
            }
        }
        "field_bot_reload" => crate::systems::field_bots::reload_config(world, pool)
            .await
            .map(|n| format!("{n} field bot zone row(s) loaded"))
            .map_err(|e| e.to_string()),
        "field_bot" => {
            let zone = args.get("zone").and_then(|v| v.as_u64()).map(|z| z as u16);
            match s("action").as_str() {
                "clear" => {
                    let removed = crate::systems::field_bots::clear(world, zone);
                    Ok(format!("Removed {removed} field bot(s); enabled zones refill within seconds"))
                }
                other => Err(format!("unknown field_bot action {other}")),
            }
        }
        "pk_autospawn" => crate::systems::bot_ai::autospawn_pk_bots(world, pool).await,
        "bot_snapshot" => crate::systems::bot_ai::write_bot_snapshot(world, pool)
            .await
            .map(|n| format!("{n} bot(s) written to bot_snapshot")),
        "bot_loadout_reload" => crate::systems::bot_ai::reload_bot_loadout_overrides(pool)
            .await
            .map(|n| format!("{n} bot loadout override bracket(s) loaded"))
            .map_err(|e| e.to_string()),
        "bot_brain_reload" => crate::systems::bot_brain::reload_config(pool)
            .await
            .map(|n| format!("{n} bot brain config row(s) loaded"))
            .map_err(|e| e.to_string()),
        "info_panel_reload" => crate::handler::info_panel::reload_and_sync(world, pool).await,
        "event_bots_reload" => crate::systems::event_bots::reload_config(pool, world)
            .await
            .map(|n| format!("{n} event bot setting(s) loaded"))
            .map_err(|e| e.to_string()),
        "event_bots_clear" => {
            let removed = crate::systems::event_bots::cleanup_all(world);
            Ok(format!("Removed {removed} event bot(s)"))
        }
        other => {
            warn!(command = other, "unknown admin command");
            Err(format!("unknown command {other}"))
        }
    }
}
