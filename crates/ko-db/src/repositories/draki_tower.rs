//! Draki Tower instance dungeon repository.
//!                stage/monster loading and user data persistence.

use crate::models::draki_tower::{
    DrakiMonsterListRow, DrakiTowerRiftRankRow, DrakiTowerStageRow, UserDrakiTowerDataRow,
};
use crate::DbPool;

/// Repository for Draki Tower data access.
pub struct DrakiTowerRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DrakiTowerRepository<'a> {
    /// Create a new Draki Tower repository.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Load all stage definitions from the database.
    ///
    pub async fn load_all_stages(&self) -> Result<Vec<DrakiTowerStageRow>, sqlx::Error> {
        sqlx::query_as::<_, DrakiTowerStageRow>(
            "SELECT id, draki_stage, draki_sub_stage, draki_tower_npc_state \
             FROM draki_tower_stages ORDER BY id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all monster spawn definitions from the database.
    ///
    pub async fn load_all_monsters(&self) -> Result<Vec<DrakiMonsterListRow>, sqlx::Error> {
        sqlx::query_as::<_, DrakiMonsterListRow>(
            "SELECT id, stage_id, monster_id, pos_x, pos_z, s_direction, is_monster \
             FROM draki_monster_list ORDER BY stage_id, id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load user Draki Tower progress data.
    ///
    pub async fn load_user_data(
        &self,
        user_id: &str,
    ) -> Result<Option<UserDrakiTowerDataRow>, sqlx::Error> {
        sqlx::query_as::<_, UserDrakiTowerDataRow>(
            "SELECT str_user_id, class, class_name, i_draki_time, \
             b_draki_stage, b_draki_enterance_limit \
             FROM user_draki_tower_data WHERE str_user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Lazily apply the 18:00 Europe/Istanbul daily reset for one user.
    /// This makes the reset reliable even when the server was offline at 18:00.
    pub async fn reset_user_entrance_limit_if_due(
        &self,
        user_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE user_draki_tower_data \
             SET b_draki_enterance_limit = 3, \
                 draki_limit_reset_bucket = \
                     (CURRENT_TIMESTAMP AT TIME ZONE 'Europe/Istanbul' - INTERVAL '18 hours')::date \
             WHERE str_user_id = $1 \
               AND (draki_limit_reset_bucket IS NULL OR draki_limit_reset_bucket < \
                    (CURRENT_TIMESTAMP AT TIME ZONE 'Europe/Istanbul' - INTERVAL '18 hours')::date)",
        )
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Upsert user Draki Tower progress data (best-result only).
    ///
    /// Only updates stage/time if the new result is better:
    /// - new_stage > existing stage, OR
    /// - same stage but faster time.
    ///   Entrance limit is always updated.
    ///
    /// `IF @bDrakiStage > bDrakiStage OR (@bDrakiStage = bDrakiStage AND @iDrakiTime <= iDrakiTime)`
    pub async fn save_user_data(
        &self,
        user_id: &str,
        class: i32,
        class_name: &str,
        draki_time: i32,
        draki_stage: i16,
        enterance_limit: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_draki_tower_data \
             (str_user_id, class, class_name, i_draki_time, b_draki_stage, b_draki_enterance_limit, \
              draki_limit_reset_bucket) \
             VALUES ($1, $2, $3, $4, $5, $6, \
                     (CURRENT_TIMESTAMP AT TIME ZONE 'Europe/Istanbul' - INTERVAL '18 hours')::date) \
             ON CONFLICT (str_user_id) DO UPDATE SET \
             class = EXCLUDED.class, \
             class_name = EXCLUDED.class_name, \
             i_draki_time = CASE \
                 WHEN EXCLUDED.b_draki_stage > user_draki_tower_data.b_draki_stage THEN EXCLUDED.i_draki_time \
                 WHEN EXCLUDED.b_draki_stage = user_draki_tower_data.b_draki_stage \
                      AND EXCLUDED.i_draki_time <= user_draki_tower_data.i_draki_time THEN EXCLUDED.i_draki_time \
                 ELSE user_draki_tower_data.i_draki_time END, \
             b_draki_stage = CASE \
                 WHEN EXCLUDED.b_draki_stage > user_draki_tower_data.b_draki_stage THEN EXCLUDED.b_draki_stage \
                 WHEN EXCLUDED.b_draki_stage = user_draki_tower_data.b_draki_stage \
                      AND EXCLUDED.i_draki_time <= user_draki_tower_data.i_draki_time THEN EXCLUDED.b_draki_stage \
                 ELSE user_draki_tower_data.b_draki_stage END, \
             b_draki_enterance_limit = EXCLUDED.b_draki_enterance_limit",
        )
        .bind(user_id)
        .bind(class)
        .bind(class_name)
        .bind(draki_time)
        .bind(draki_stage)
        .bind(enterance_limit)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update only the entrance limit for a user.
    ///
    pub async fn update_entrance_limit(
        &self,
        user_id: &str,
        limit: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_draki_tower_data \
             (str_user_id, b_draki_enterance_limit, draki_limit_reset_bucket) \
             VALUES ($2, $1, \
                     (CURRENT_TIMESTAMP AT TIME ZONE 'Europe/Istanbul' - INTERVAL '18 hours')::date) \
             ON CONFLICT (str_user_id) DO UPDATE SET \
             b_draki_enterance_limit = EXCLUDED.b_draki_enterance_limit, \
             draki_limit_reset_bucket = EXCLUDED.draki_limit_reset_bucket",
        )
        .bind(limit)
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Reset all users' entrance limits to 3 (daily reset at 18:00).
    ///
    pub async fn reset_all_entrance_limits(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE user_draki_tower_data SET \
             b_draki_enterance_limit = 3, \
             draki_limit_reset_bucket = \
                 (CURRENT_TIMESTAMP AT TIME ZONE 'Europe/Istanbul' - INTERVAL '18 hours')::date",
        )
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Load all rift rankings ordered by class then rank.
    ///
    pub async fn load_rift_ranks(&self) -> Result<Vec<DrakiTowerRiftRankRow>, sqlx::Error> {
        sqlx::query_as::<_, DrakiTowerRiftRankRow>(
            "SELECT s_index, class, class_name, rank_id, str_user_id, b_stage, finish_time \
             FROM draki_tower_rift_rank ORDER BY class, rank_id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Upsert a rift ranking entry for a user in a specific class.
    ///
    pub async fn upsert_rift_rank(
        &self,
        class: i32,
        class_name: &str,
        rank_id: i32,
        user_id: &str,
        stage: i16,
        finish_time: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO draki_tower_rift_rank \
             (class, class_name, rank_id, str_user_id, b_stage, finish_time) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (class, rank_id) DO UPDATE SET \
             class_name = EXCLUDED.class_name, \
             str_user_id = EXCLUDED.str_user_id, \
             b_stage = EXCLUDED.b_stage, \
             finish_time = EXCLUDED.finish_time",
        )
        .bind(class)
        .bind(class_name)
        .bind(rank_id)
        .bind(user_id)
        .bind(stage)
        .bind(finish_time)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
