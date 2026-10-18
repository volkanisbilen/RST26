//! Achievement repository — database access for achievement reference and user progress tables.
//! - `LoadServerData.h` — `CAchieveMainSet`, `CAchieveWarSet`, etc.
//! - `DBAgent.cpp:3842` — `UpdateAchieveData()`, `LoadAchieveData()`

use crate::models::{
    AchieveComRow, AchieveMainRow, AchieveMonsterRow, AchieveNormalRow, AchieveTitleRow,
    AchieveWarRow, UserAchieveRow, UserAchieveSummaryRow,
};
use crate::DbPool;

/// Repository for achievement-related DB operations.
pub struct AchieveRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> AchieveRepository<'a> {
    /// Create a new achievement repository.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    // ── Reference Table Loading (startup) ───────────────────────────────

    /// Load all achievement main definitions.
    ///
    pub async fn load_achieve_main(&self) -> Result<Vec<AchieveMainRow>, sqlx::Error> {
        sqlx::query_as::<_, AchieveMainRow>("SELECT * FROM achieve_main ORDER BY s_index")
            .fetch_all(self.pool)
            .await
    }

    /// Load all war-type achievements.
    ///
    pub async fn load_achieve_war(&self) -> Result<Vec<AchieveWarRow>, sqlx::Error> {
        sqlx::query_as::<_, AchieveWarRow>("SELECT * FROM achieve_war ORDER BY s_index")
            .fetch_all(self.pool)
            .await
    }

    /// Load all normal-type achievements.
    ///
    pub async fn load_achieve_normal(&self) -> Result<Vec<AchieveNormalRow>, sqlx::Error> {
        sqlx::query_as::<_, AchieveNormalRow>("SELECT * FROM achieve_normal ORDER BY s_index")
            .fetch_all(self.pool)
            .await
    }

    /// Load all monster-kill achievements.
    ///
    pub async fn load_achieve_monster(&self) -> Result<Vec<AchieveMonsterRow>, sqlx::Error> {
        sqlx::query_as::<_, AchieveMonsterRow>("SELECT * FROM achieve_monster ORDER BY s_index")
            .fetch_all(self.pool)
            .await
    }

    /// Load all composite (requirement-based) achievements.
    ///
    pub async fn load_achieve_com(&self) -> Result<Vec<AchieveComRow>, sqlx::Error> {
        sqlx::query_as::<_, AchieveComRow>("SELECT * FROM achieve_com ORDER BY s_index")
            .fetch_all(self.pool)
            .await
    }

    /// Load all achievement title bonuses.
    ///
    pub async fn load_achieve_title(&self) -> Result<Vec<AchieveTitleRow>, sqlx::Error> {
        sqlx::query_as::<_, AchieveTitleRow>("SELECT * FROM achieve_title ORDER BY s_index")
            .fetch_all(self.pool)
            .await
    }

    // ── Per-Player Data ─────────────────────────────────────────────────

    /// Load all achievement progress for a character.
    ///
    pub async fn load_user_achieves(
        &self,
        char_name: &str,
    ) -> Result<Vec<UserAchieveRow>, sqlx::Error> {
        sqlx::query_as::<_, UserAchieveRow>(
            "SELECT * FROM user_achieve WHERE str_user_id = $1 ORDER BY achieve_id",
        )
        .bind(char_name)
        .fetch_all(self.pool)
        .await
    }

    /// Load achievement summary for a character.
    ///
    pub async fn load_user_achieve_summary(
        &self,
        char_name: &str,
    ) -> Result<Option<UserAchieveSummaryRow>, sqlx::Error> {
        sqlx::query_as::<_, UserAchieveSummaryRow>(
            "SELECT * FROM user_achieve_summary WHERE str_user_id = $1",
        )
        .bind(char_name)
        .fetch_optional(self.pool)
        .await
    }

    /// Save or update a single achievement entry for a character.
    ///
    /// Uses upsert (INSERT ON CONFLICT UPDATE).
    pub async fn save_user_achieve(
        &self,
        char_name: &str,
        achieve_id: i32,
        status: i16,
        count1: i32,
        count2: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_achieve (str_user_id, achieve_id, status, count1, count2)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (str_user_id, achieve_id) DO UPDATE SET
                status = EXCLUDED.status,
                count1 = EXCLUDED.count1,
                count2 = EXCLUDED.count2",
        )
        .bind(char_name)
        .bind(achieve_id)
        .bind(status)
        .bind(count1)
        .bind(count2)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Batch save or update all achievement entries for a character in a single query.
    ///
    /// Each entry: (achieve_id, status, count1, count2).
    pub async fn save_user_achieves_batch(
        &self,
        char_name: &str,
        entries: &[(i32, i16, i32, i32)],
    ) -> Result<(), sqlx::Error> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO user_achieve (str_user_id, achieve_id, status, count1, count2) ",
        );
        builder.push_values(entries, |mut b, &(achieve_id, status, count1, count2)| {
            b.push_bind(char_name)
                .push_bind(achieve_id)
                .push_bind(status)
                .push_bind(count1)
                .push_bind(count2);
        });
        builder.push(
            " ON CONFLICT (str_user_id, achieve_id) DO UPDATE SET \
             status = EXCLUDED.status, count1 = EXCLUDED.count1, count2 = EXCLUDED.count2",
        );
        builder.build().execute(self.pool).await?;
        Ok(())
    }

    /// Save or update achievement summary for a character.
    ///
    #[allow(clippy::too_many_arguments)]
    pub async fn save_user_achieve_summary(
        &self,
        char_name: &str,
        play_time: i32,
        monster_defeat_count: i32,
        user_defeat_count: i32,
        user_death_count: i32,
        total_medal: i32,
        recent: [i16; 3],
        cover_id: i16,
        skill_id: i16,
        skill_title_row: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_achieve_summary (str_user_id, play_time, monster_defeat_count,
             user_defeat_count, user_death_count, total_medal, recent_achieve_1, recent_achieve_2,
             recent_achieve_3, cover_id, skill_id, skill_title_row)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             ON CONFLICT (str_user_id) DO UPDATE SET
                play_time = EXCLUDED.play_time,
                monster_defeat_count = EXCLUDED.monster_defeat_count,
                user_defeat_count = EXCLUDED.user_defeat_count,
                user_death_count = EXCLUDED.user_death_count,
                total_medal = EXCLUDED.total_medal,
                recent_achieve_1 = EXCLUDED.recent_achieve_1,
                recent_achieve_2 = EXCLUDED.recent_achieve_2,
                recent_achieve_3 = EXCLUDED.recent_achieve_3,
                cover_id = EXCLUDED.cover_id,
                skill_id = EXCLUDED.skill_id,
                skill_title_row = EXCLUDED.skill_title_row",
        )
        .bind(char_name)
        .bind(play_time)
        .bind(monster_defeat_count)
        .bind(user_defeat_count)
        .bind(user_death_count)
        .bind(total_medal)
        .bind(recent[0])
        .bind(recent[1])
        .bind(recent[2])
        .bind(cover_id)
        .bind(skill_id)
        .bind(skill_title_row)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
