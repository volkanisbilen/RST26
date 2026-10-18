//! Knights cape repository -- loads cape definitions, castellan bonuses,
//! CSW options, and user knight data from PostgreSQL.
//! - `CKnightsCapeSet` in `KnightsCapeSet.h` — cape table loading
//! - `CCapeCastellanBonusSet` in `CapeCastellanBonusSet.h` — bonus loading
//! - `HandleCapeChange()` in `KnightCape.cpp` — cape purchase/save

use crate::models::knights_cape::{
    KnightsCapeCastellanBonusRow, KnightsCapeRow, KnightsCswOptRow, UserKnightDataRow,
};
use crate::DbPool;

/// Repository for knights cape-related table access.
pub struct KnightsCapeRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> KnightsCapeRepository<'a> {
    /// Create a new repository backed by the given connection pool.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Load all cape definitions from the `knights_cape` table.
    ///
    pub async fn load_all_capes(&self) -> Result<Vec<KnightsCapeRow>, sqlx::Error> {
        sqlx::query_as::<_, KnightsCapeRow>(
            "SELECT s_cape_index, n_buy_price, by_grade, n_buy_loyalty, \
             by_ranking, b_type, b_ticket, bonus_type \
             FROM knights_cape ORDER BY s_cape_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all castellan cape bonus definitions.
    ///
    pub async fn load_all_castellan_bonuses(
        &self,
    ) -> Result<Vec<KnightsCapeCastellanBonusRow>, sqlx::Error> {
        sqlx::query_as::<_, KnightsCapeCastellanBonusRow>(
            "SELECT bonus_type, type_name, ac_bonus, hp_bonus, mp_bonus, \
             str_bonus, sta_bonus, dex_bonus, int_bonus, cha_bonus, \
             flame_resist, glacier_resist, lightning_resist, magic_resist, \
             disease_resist, poison_resist, \
             xp_bonus_pct, coin_bonus_pct, ap_bonus_pct, ac_bonus_pct, \
             max_weight_bonus, np_bonus \
             FROM knights_cape_castellan_bonus ORDER BY bonus_type",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load the CSW configuration options (single row).
    ///
    pub async fn load_csw_opt(&self) -> Result<Option<KnightsCswOptRow>, sqlx::Error> {
        sqlx::query_as::<_, KnightsCswOptRow>(
            "SELECT id, preparing, war_time, money, tl, cash, loyalty, \
             item_id_1, item_count_1, item_time_1, \
             item_id_2, item_count_2, item_time_2, \
             item_id_3, item_count_3, item_time_3 \
             FROM knights_csw_opt LIMIT 1",
        )
        .fetch_optional(self.pool)
        .await
    }

    /// Load all user knight data rows for a specific clan.
    ///
    pub async fn load_by_clan(&self, clan_id: i16) -> Result<Vec<UserKnightDataRow>, sqlx::Error> {
        sqlx::query_as::<_, UserKnightDataRow>(
            "SELECT s_clan_id, str_user_id, n_donated_np, str_memo, \
             fame, s_class, level, last_login, loyalty, loyalty_monthly \
             FROM user_knightdata WHERE s_clan_id = $1 \
             ORDER BY str_user_id",
        )
        .bind(clan_id)
        .fetch_all(self.pool)
        .await
    }

    /// Load all user knight data rows (for full server startup).
    pub async fn load_all_user_knightdata(&self) -> Result<Vec<UserKnightDataRow>, sqlx::Error> {
        sqlx::query_as::<_, UserKnightDataRow>(
            "SELECT s_clan_id, str_user_id, n_donated_np, str_memo, \
             fame, s_class, level, last_login, loyalty, loyalty_monthly \
             FROM user_knightdata ORDER BY s_clan_id, str_user_id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Update a user's knight data (donation, loyalty, etc.) after changes.
    ///
    pub async fn update_user_knightdata(
        &self,
        clan_id: i16,
        user_id: &str,
        donated_np: i32,
        loyalty: i32,
        loyalty_monthly: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE user_knightdata SET n_donated_np = $1, loyalty = $2, \
             loyalty_monthly = $3 WHERE s_clan_id = $4 AND str_user_id = $5",
        )
        .bind(donated_np)
        .bind(loyalty)
        .bind(loyalty_monthly)
        .bind(clan_id)
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Insert or update user knight data when a user joins or changes clans.
    pub async fn upsert_user_knightdata(&self, row: &UserKnightDataRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_knightdata \
             (s_clan_id, str_user_id, n_donated_np, str_memo, fame, \
              s_class, level, last_login, loyalty, loyalty_monthly) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (s_clan_id, str_user_id) DO UPDATE SET \
             n_donated_np = EXCLUDED.n_donated_np, \
             str_memo = EXCLUDED.str_memo, \
             fame = EXCLUDED.fame, \
             s_class = EXCLUDED.s_class, \
             level = EXCLUDED.level, \
             last_login = EXCLUDED.last_login, \
             loyalty = EXCLUDED.loyalty, \
             loyalty_monthly = EXCLUDED.loyalty_monthly",
        )
        .bind(row.s_clan_id)
        .bind(&row.str_user_id)
        .bind(row.n_donated_np)
        .bind(&row.str_memo)
        .bind(row.fame)
        .bind(row.s_class)
        .bind(row.level)
        .bind(row.last_login)
        .bind(row.loyalty)
        .bind(row.loyalty_monthly)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Sync user_knightdata on logout/save — update level, class, loyalty, last_login.
    ///
    #[allow(clippy::too_many_arguments)]
    pub async fn sync_user_knightdata_on_save(
        &self,
        clan_id: i16,
        user_id: &str,
        class: i16,
        level: i16,
        loyalty: i32,
        loyalty_monthly: i32,
        last_login: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE user_knightdata SET s_class = $1, level = $2, loyalty = $3, \
             loyalty_monthly = $4, last_login = $5 \
             WHERE s_clan_id = $6 AND str_user_id = $7",
        )
        .bind(class)
        .bind(level)
        .bind(loyalty)
        .bind(loyalty_monthly)
        .bind(last_login)
        .bind(clan_id)
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Save a clan's cape to the knights table after purchase.
    ///
    pub async fn save_cape(
        &self,
        clan_id: i16,
        cape_index: i16,
        cape_r: i16,
        cape_g: i16,
        cape_b: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights SET s_cape = $1, b_cape_r = $2, b_cape_g = $3, \
             b_cape_b = $4 WHERE id_num = $5",
        )
        .bind(cape_index)
        .bind(cape_r)
        .bind(cape_g)
        .bind(cape_b)
        .bind(clan_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Save a castellan cape to the knights table.
    ///
    pub async fn save_castellan_cape(
        &self,
        clan_id: i16,
        cape_index: i16,
        cape_r: i16,
        cape_g: i16,
        cape_b: i16,
        expiry_time: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights SET s_cast_cape = $1, b_cast_cape_r = $2, \
             b_cast_cape_g = $3, b_cast_cape_b = $4, b_cast_time = $5 \
             WHERE id_num = $6",
        )
        .bind(cape_index)
        .bind(cape_r)
        .bind(cape_g)
        .bind(cape_b)
        .bind(expiry_time)
        .bind(clan_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Deduct clan points (gold from clan bank) after a cape purchase.
    ///
    pub async fn deduct_clan_points(&self, clan_id: i16, amount: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET clan_point_fund = clan_point_fund - $1 WHERE id_num = $2")
            .bind(amount)
            .bind(clan_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }
}
