//! Daily quest repository — definitions and user progress.

use sqlx::{PgPool, QueryBuilder};

use crate::models::daily_quest::{DailyQuestRow, UserDailyQuestRow};

/// Repository for daily quest definitions and user progress.
pub struct DailyQuestRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> DailyQuestRepository<'a> {
    /// Create a new daily quest repository.
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // ── Quest Definitions ────────────────────────────────────────────────

    /// Load all daily quest definitions from the database.
    ///
    pub async fn load_all_definitions(&self) -> Result<Vec<DailyQuestRow>, sqlx::Error> {
        sqlx::query_as::<_, DailyQuestRow>(
            "SELECT id, quest_name, quest_id, time_type, kill_type, \
             mob_id_1, mob_id_2, mob_id_3, mob_id_4, kill_count, \
             reward_1, reward_2, reward_3, reward_4, \
             count_1, count_2, count_3, count_4, \
             zone_id, min_level, max_level, replay_time, random_id \
             FROM daily_quests ORDER BY id",
        )
        .fetch_all(self.pool)
        .await
    }

    // ── User Quest Progress ──────────────────────────────────────────────

    /// Load all daily quest progress for a character.
    ///
    /// each entry is 8 bytes: (quest_id:u8, status:u8, kcount:u16, replaytime:u32).
    pub async fn load_user_quests(
        &self,
        character_id: &str,
    ) -> Result<Vec<UserDailyQuestRow>, sqlx::Error> {
        sqlx::query_as::<_, UserDailyQuestRow>(
            "SELECT character_id, quest_id, kill_count, status, replay_time \
             FROM user_daily_quest WHERE character_id = $1 AND is_selected = TRUE ORDER BY quest_id",
        )
        .bind(character_id)
        .fetch_all(self.pool)
        .await
    }

    /// Save (upsert) a single daily quest progress entry for a character.
    pub async fn save_user_quest(&self, row: &UserDailyQuestRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_daily_quest (character_id, quest_id, kill_count, status, replay_time, is_selected) \
             VALUES ($1, $2, $3, $4, $5, TRUE) \
             ON CONFLICT (character_id, quest_id) DO UPDATE SET \
               kill_count = EXCLUDED.kill_count, \
               status = EXCLUDED.status, \
               replay_time = EXCLUDED.replay_time, \
               is_selected = TRUE",
        )
        .bind(&row.character_id)
        .bind(row.quest_id)
        .bind(row.kill_count)
        .bind(row.status)
        .bind(row.replay_time)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Save all daily quest progress entries for a character (batch upsert).
    ///
    /// Replaces all existing entries with the provided list.
    /// Uses a single multi-row INSERT instead of N individual queries.
    pub async fn save_all_user_quests(
        &self,
        character_id: &str,
        entries: &[UserDailyQuestRow],
    ) -> Result<(), sqlx::Error> {
        // Replace only NPC-selected entries. Legacy auto-populated rows are
        // deliberately left untouched and remain `is_selected = false`.
        sqlx::query("DELETE FROM user_daily_quest WHERE character_id = $1 AND is_selected = TRUE")
            .bind(character_id)
            .execute(self.pool)
            .await?;

        if entries.is_empty() {
            return Ok(());
        }

        // Batch insert all entries in a single query
        let mut builder: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
            "INSERT INTO user_daily_quest (character_id, quest_id, kill_count, status, replay_time, is_selected) ",
        );
        builder.push_values(entries, |mut b, entry| {
            b.push_bind(character_id)
                .push_bind(entry.quest_id)
                .push_bind(entry.kill_count)
                .push_bind(entry.status)
                .push_bind(entry.replay_time)
                .push_bind(true);
        });
        builder.build().execute(self.pool).await?;

        Ok(())
    }

    /// Delete one daily quest progress entry (player abandoned the quest).
    pub async fn delete_user_quest(
        &self,
        character_id: &str,
        quest_id: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM user_daily_quest WHERE character_id = $1 AND quest_id = $2")
            .bind(character_id)
            .bind(quest_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Delete all daily quest progress for a character (e.g., on character delete).
    pub async fn delete_user_quests(&self, character_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM user_daily_quest WHERE character_id = $1")
            .bind(character_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }
}
