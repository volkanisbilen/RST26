//! Realtime notifications for letters inserted by the local admin GUI.

use std::sync::Arc;
use std::time::Duration;

use ko_db::DbPool;
use tracing::{debug, warn};

use crate::world::WorldState;

const POLL_INTERVAL_MS: u64 = 500;

pub fn start_letter_admin_notify_task(
    world: Arc<WorldState>,
    pool: DbPool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(POLL_INTERVAL_MS));
        loop {
            interval.tick().await;
            if let Err(error) = poll(&world, &pool).await {
                warn!("Letter admin notification poll failed: {}", error);
            }
        }
    })
}

async fn poll(world: &WorldState, pool: &DbPool) -> Result<(), sqlx::Error> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT notification_id, recipient_name FROM letter_admin_notification \
         WHERE processed_at IS NULL ORDER BY notification_id LIMIT 100",
    )
    .fetch_all(pool)
    .await?;

    for (notification_id, recipient) in rows {
        let delivered_online = if let Some(sid) = world.find_session_by_name(&recipient) {
            world.send_to_session_owned(sid, crate::handler::letter::build_unread_notification());
            true
        } else {
            false
        };

        sqlx::query(
            "UPDATE letter_admin_notification SET processed_at = NOW() \
             WHERE notification_id = $1 AND processed_at IS NULL",
        )
        .bind(notification_id)
        .execute(pool)
        .await?;

        debug!(
            "Letter admin notification {} recipient={} online={}",
            notification_id, recipient, delivered_online
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_interval_is_realtime() {
        assert!(POLL_INTERVAL_MS <= 1_000);
    }
}
