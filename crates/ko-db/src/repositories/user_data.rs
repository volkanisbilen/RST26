//! User data persistence repository — genie, daily ops, loot settings, seal exp, return data.
//!                various `USER_*` table operations.

use sqlx::PgPool;

use crate::models::user_data::{
    DailyRewardUserRow, UserDailyOpRow, UserGenieDataRow, UserLootSettingsRow, UserReturnDataRow,
    UserSealExpRow,
};

/// Repository for user data persistence tables.
pub struct UserDataRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> UserDataRepository<'a> {
    /// Create a new user data repository.
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // ── Genie Data ───────────────────────────────────────────────────────

    /// Load genie data for a user account.
    ///
    pub async fn load_genie_data(
        &self,
        user_id: &str,
    ) -> Result<Option<UserGenieDataRow>, sqlx::Error> {
        sqlx::query_as::<_, UserGenieDataRow>(
            "SELECT user_id, genie_time, genie_options, first_using_genie \
             FROM user_genie_data WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Save runtime Genie state. Options are updated only by the explicit
    /// options save, so an older periodic/disconnect snapshot cannot replace
    /// a newer settings packet.
    ///
    pub async fn save_genie_data(
        &self,
        user_id: &str,
        genie_time: i32,
        genie_options: &[u8],
        first_using_genie: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_genie_data (user_id, genie_time, genie_options, first_using_genie) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id) DO UPDATE SET \
               genie_time = EXCLUDED.genie_time, \
               first_using_genie = EXCLUDED.first_using_genie",
        )
        .bind(user_id)
        .bind(genie_time)
        .bind(genie_options)
        .bind(first_using_genie)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_genie_options(
        &self,
        user_id: &str,
        options: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_genie_data (user_id, genie_time, genie_options, first_using_genie) \
             VALUES ($1, 0, $2, 0) ON CONFLICT (user_id) DO UPDATE \
             SET genie_options = EXCLUDED.genie_options",
        )
        .bind(user_id)
        .bind(options)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ── Daily Operations ─────────────────────────────────────────────────

    /// Load daily operations data for a user account.
    pub async fn load_daily_op(
        &self,
        user_id: &str,
    ) -> Result<Option<UserDailyOpRow>, sqlx::Error> {
        sqlx::query_as::<_, UserDailyOpRow>(
            "SELECT user_id, chaos_map_time, user_rank_reward_time, personal_rank_reward_time, \
             king_wing_time, warder_killer_time1, warder_killer_time2, keeper_killer_time, \
             user_loyalty_wing_reward_time, full_moon_rift_map_time, copy_information_time \
             FROM user_daily_op WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Save (upsert) daily operations data for a user account.
    pub async fn save_daily_op(&self, row: &UserDailyOpRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_daily_op (user_id, chaos_map_time, user_rank_reward_time, \
             personal_rank_reward_time, king_wing_time, warder_killer_time1, warder_killer_time2, \
             keeper_killer_time, user_loyalty_wing_reward_time, full_moon_rift_map_time, \
             copy_information_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             ON CONFLICT (user_id) DO UPDATE SET \
               chaos_map_time = EXCLUDED.chaos_map_time, \
               user_rank_reward_time = EXCLUDED.user_rank_reward_time, \
               personal_rank_reward_time = EXCLUDED.personal_rank_reward_time, \
               king_wing_time = EXCLUDED.king_wing_time, \
               warder_killer_time1 = EXCLUDED.warder_killer_time1, \
               warder_killer_time2 = EXCLUDED.warder_killer_time2, \
               keeper_killer_time = EXCLUDED.keeper_killer_time, \
               user_loyalty_wing_reward_time = EXCLUDED.user_loyalty_wing_reward_time, \
               full_moon_rift_map_time = EXCLUDED.full_moon_rift_map_time, \
               copy_information_time = EXCLUDED.copy_information_time",
        )
        .bind(&row.user_id)
        .bind(row.chaos_map_time)
        .bind(row.user_rank_reward_time)
        .bind(row.personal_rank_reward_time)
        .bind(row.king_wing_time)
        .bind(row.warder_killer_time1)
        .bind(row.warder_killer_time2)
        .bind(row.keeper_killer_time)
        .bind(row.user_loyalty_wing_reward_time)
        .bind(row.full_moon_rift_map_time)
        .bind(row.copy_information_time)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ── Loot Settings ────────────────────────────────────────────────────

    /// Load loot settings for a user.
    pub async fn load_loot_settings(
        &self,
        user_id: &str,
    ) -> Result<Option<UserLootSettingsRow>, sqlx::Error> {
        sqlx::query_as::<_, UserLootSettingsRow>(
            "SELECT id, user_id, warrior, rogue, mage, priest, weapon, armor, accessory, \
             normal, upgrade, craft, rare, magic, unique_grade, consumable, price \
             FROM user_loot_settings WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Save (upsert) loot settings for a user.
    pub async fn save_loot_settings(&self, row: &UserLootSettingsRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_loot_settings (user_id, warrior, rogue, mage, priest, weapon, \
             armor, accessory, normal, upgrade, craft, rare, magic, unique_grade, consumable, price) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) \
             ON CONFLICT (user_id) DO UPDATE SET \
               warrior = EXCLUDED.warrior, rogue = EXCLUDED.rogue, \
               mage = EXCLUDED.mage, priest = EXCLUDED.priest, \
               weapon = EXCLUDED.weapon, armor = EXCLUDED.armor, \
               accessory = EXCLUDED.accessory, normal = EXCLUDED.normal, \
               upgrade = EXCLUDED.upgrade, craft = EXCLUDED.craft, \
               rare = EXCLUDED.rare, magic = EXCLUDED.magic, \
               unique_grade = EXCLUDED.unique_grade, consumable = EXCLUDED.consumable, \
               price = EXCLUDED.price",
        )
        .bind(&row.user_id)
        .bind(row.warrior)
        .bind(row.rogue)
        .bind(row.mage)
        .bind(row.priest)
        .bind(row.weapon)
        .bind(row.armor)
        .bind(row.accessory)
        .bind(row.normal)
        .bind(row.upgrade)
        .bind(row.craft)
        .bind(row.rare)
        .bind(row.magic)
        .bind(row.unique_grade)
        .bind(row.consumable)
        .bind(row.price)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ── Seal Experience ──────────────────────────────────────────────────

    /// Load sealed experience for a user.
    pub async fn load_seal_exp(
        &self,
        user_id: &str,
    ) -> Result<Option<UserSealExpRow>, sqlx::Error> {
        sqlx::query_as::<_, UserSealExpRow>(
            "SELECT user_id, sealed_exp FROM user_seal_exp WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Save (upsert) sealed experience for a user.
    pub async fn save_seal_exp(&self, user_id: &str, sealed_exp: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_seal_exp (user_id, sealed_exp) VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET sealed_exp = EXCLUDED.sealed_exp",
        )
        .bind(user_id)
        .bind(sealed_exp)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ── Return Data ──────────────────────────────────────────────────────

    /// Load return data for a character.
    pub async fn load_return_data(
        &self,
        character_id: &str,
    ) -> Result<Option<UserReturnDataRow>, sqlx::Error> {
        sqlx::query_as::<_, UserReturnDataRow>(
            "SELECT character_id, return_symbol_ok, return_logout_time, return_symbol_time \
             FROM user_return_data WHERE character_id = $1",
        )
        .bind(character_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Save (upsert) return data for a character.
    pub async fn save_return_data(&self, row: &UserReturnDataRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_return_data (character_id, return_symbol_ok, return_logout_time, return_symbol_time) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (character_id) DO UPDATE SET \
               return_symbol_ok = EXCLUDED.return_symbol_ok, \
               return_logout_time = EXCLUDED.return_logout_time, \
               return_symbol_time = EXCLUDED.return_symbol_time",
        )
        .bind(&row.character_id)
        .bind(row.return_symbol_ok)
        .bind(row.return_logout_time)
        .bind(row.return_symbol_time)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ── Daily Rewards ────────────────────────────────────────────────────

    /// Load all daily reward entries for a user.
    pub async fn load_daily_rewards(
        &self,
        user_id: &str,
    ) -> Result<Vec<DailyRewardUserRow>, sqlx::Error> {
        sqlx::query_as::<_, DailyRewardUserRow>(
            "SELECT user_id, day_index, claimed, day_of_month FROM daily_reward_user \
             WHERE user_id = $1 ORDER BY day_index",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }
}
