//! ArdReam / Ronark Land Base level-bracket enforcement tick.
//! A player's level can cross a zone's level bracket mid-session (leveling up,
//! or a GM/event granting levels) without ever re-entering through a warp
//! gate, so the warp-time check in `warp_list.rs` alone is not enough. This
//! runs periodically and kicks anyone now outside their current zone's
//! bracket back to Moradon.

use std::sync::Arc;
use std::time::Duration;

use crate::handler::zone_change;
use crate::world::types::{ZONE_ARDREAM, ZONE_MORADON, ZONE_RONARK_LAND_BASE};
use crate::world::WorldState;

/// Check interval. Leveling is not latency-sensitive, so a slow poll is fine.
const ZONE_LEVEL_KICK_INTERVAL_SECS: u64 = 15;

/// Start the background task. Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_zone_level_kick_tick_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(ZONE_LEVEL_KICK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            process_zone_level_kick_tick(&world);
        }
    })
}

fn process_zone_level_kick_tick(world: &WorldState) {
    for sid in world.get_in_game_session_ids() {
        if world.is_gm(sid) {
            continue;
        }
        let Some(pos) = world.get_position(sid) else {
            continue;
        };
        if !matches!(pos.zone_id, ZONE_ARDREAM | ZONE_RONARK_LAND_BASE) {
            continue;
        }
        let Some(level) = world.get_character_info(sid).map(|c| c.level) else {
            continue;
        };
        let Some(info) = world
            .get_zone(pos.zone_id)
            .and_then(|z| z.zone_info.clone())
        else {
            continue;
        };
        if level < info.min_level || level > info.max_level {
            tracing::info!(
                sid,
                zone = pos.zone_id,
                level,
                min = info.min_level,
                max = info.max_level,
                "Zone level bracket: kicking player out of zone to Moradon"
            );
            zone_change::server_teleport_to_zone_force(world, sid, ZONE_MORADON, 0.0, 0.0);
        }
    }
}
