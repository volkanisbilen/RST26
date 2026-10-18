//! Manes Survival configuration repository.

use crate::models::manes_survival::{ManesSurvivalMagicRow, ManesSurvivalSpawnRow};
use crate::DbPool;

pub struct ManesSurvivalRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> ManesSurvivalRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn load_spawns(&self) -> Result<Vec<ManesSurvivalSpawnRow>, sqlx::Error> {
        sqlx::query_as::<_, ManesSurvivalSpawnRow>(
            "SELECT id, npc_id, grade, boss_tier, spawn_x, spawn_z, \
             spawn_range, spawn_count, respawn_seconds \
             FROM manes_survival_spawn ORDER BY grade, boss_tier, id",
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn load_magic(&self) -> Result<Vec<ManesSurvivalMagicRow>, sqlx::Error> {
        sqlx::query_as::<_, ManesSurvivalMagicRow>(
            "SELECT col_2::BIGINT AS selection_id, col_1 AS kind, col_3 AS magic_id, \
             col_5 AS item_id, col_6 AS item_count, col_7 AS price, \
             col_8 AS hp_bonus, col_9 AS attack_bonus_pct, col_10 AS reduction_pct \
             FROM manes_survival_client_magic ORDER BY col_0",
        )
        .fetch_all(self.pool)
        .await
    }
}
