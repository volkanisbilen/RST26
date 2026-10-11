//! Cash shop repository — loads PUS categories, items, and manages
//! purchase/refund records from PostgreSQL.
//! - `ShoppingMallHandler.cpp` — PUS open/close flow
//! - `DBAgent.cpp` — `LoadWebItemMall()`, `CreatePusSession()`

use crate::models::cash_shop::{PusCategoryRow, PusItemRow, PusRefundRow};
use crate::DbPool;

/// Repository for cash shop (PUS) table access.
pub struct CashShopRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> CashShopRepository<'a> {
    /// Create a new repository backed by the given connection pool.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Load all active cash shop categories (bulk load at startup).
    ///
    /// Returns categories where status = 1 (active).
    pub async fn load_all_categories(&self) -> Result<Vec<PusCategoryRow>, sqlx::Error> {
        sqlx::query_as::<_, PusCategoryRow>(
            "SELECT id, category_name, description, category_id, status \
             FROM pus_category WHERE status = 1 ORDER BY id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all cash shop items (bulk load at startup).
    ///
    /// Returns all items sorted by category then ID.
    pub async fn load_all_items(&self) -> Result<Vec<PusItemRow>, sqlx::Error> {
        sqlx::query_as::<_, PusItemRow>(
            "SELECT id, item_id, item_name, item_title, price, send_type, \
             buy_count, item_desc, category, price_type \
             FROM pus_items ORDER BY category, id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load purchase history for a specific account.
    ///
    /// Returns refund records ordered by most recent first.
    pub async fn load_account_purchases(
        &self,
        account_id: &str,
    ) -> Result<Vec<PusRefundRow>, sqlx::Error> {
        sqlx::query_as::<_, PusRefundRow>(
            "SELECT mserial, account_id, item_id, item_count, item_price, \
             buying_time, item_duration, buy_type \
             FROM pus_refund WHERE account_id = $1 ORDER BY buying_time DESC",
        )
        .bind(account_id)
        .fetch_all(self.pool)
        .await
    }

    /// Record a new purchase in the refund/history table.
    pub async fn insert_purchase(&self, row: &PusRefundRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO pus_refund (mserial, account_id, item_id, item_count, \
             item_price, buying_time, item_duration, buy_type) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(row.mserial)
        .bind(&row.account_id)
        .bind(row.item_id)
        .bind(row.item_count)
        .bind(row.item_price)
        .bind(row.buying_time)
        .bind(row.item_duration)
        .bind(row.buy_type)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update account's cash point balance after a purchase.
    ///
    /// Deducts `amount` from `cash_point` (for KC purchases)
    /// or `bonus_cash_point` column isn't used here — TL purchases
    /// deduct from cash_point as well.
    pub async fn deduct_cash_point(
        &self,
        account_id: &str,
        amount: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE tb_user SET cash_point = cash_point - $1 \
             WHERE str_account_id = $2 AND cash_point >= $1",
        )
        .bind(amount)
        .bind(account_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Get current cash point balance for an account.
    pub async fn get_cash_point(&self, account_id: &str) -> Result<i32, sqlx::Error> {
        let row: (i32,) =
            sqlx::query_as("SELECT cash_point FROM tb_user WHERE str_account_id = $1")
                .bind(account_id)
                .fetch_one(self.pool)
                .await?;
        Ok(row.0)
    }

    /// Load both KC (cash_point) and TL (bonus_cash_point) balances for an account.
    ///
    /// Selects `CashPoint, BonusCashPoint` from `TB_USER`.
    pub async fn load_kc_balances(&self, account_id: &str) -> Result<(i32, i32), sqlx::Error> {
        let row: (i32, Option<i32>) = sqlx::query_as(
            "SELECT cash_point, bonus_cash_point \
             FROM tb_user WHERE str_account_id = $1",
        )
        .bind(account_id)
        .fetch_one(self.pool)
        .await?;
        Ok((row.0, row.1.unwrap_or(0)))
    }

    /// Adjust only TL. KC must not be rewritten by a GM TL grant.
    pub async fn adjust_tl_balance(&self, account_id: &str, amount: i32) -> Result<Option<i32>, sqlx::Error> {
        let row: Option<(i32,)> = sqlx::query_as(
            "UPDATE tb_user SET bonus_cash_point = COALESCE(bonus_cash_point, 0)::bigint + $2 \
             WHERE str_account_id = $1 \
             AND COALESCE(bonus_cash_point, 0)::bigint + $2 BETWEEN 0 AND 2147483647 \
             RETURNING bonus_cash_point",
        ).bind(account_id).bind(amount as i64).fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.0))
    }

    /// Deduct amount from TL balance (bonus_cash_point).
    ///
    /// Uses `bonus_cash_point` column for TL purchases.
    pub async fn deduct_tl_balance(
        &self,
        account_id: &str,
        amount: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE tb_user SET bonus_cash_point = COALESCE(bonus_cash_point, 0) - $1 \
             WHERE str_account_id = $2 AND COALESCE(bonus_cash_point, 0) >= $1",
        )
        .bind(amount)
        .bind(account_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update both KC and TL balances for an account.
    ///
    /// Calls `UPDATE_BALANCE` stored procedure equivalent.
    pub async fn update_kc_balances(
        &self,
        account_id: &str,
        knight_cash: i32,
        tl_balance: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE tb_user SET cash_point = $1, bonus_cash_point = $2 \
             WHERE str_account_id = $3",
        )
        .bind(knight_cash)
        .bind(tl_balance)
        .bind(account_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Delete a refund record after a successful item return.
    ///
    pub async fn delete_purchase(&self, account_id: &str, serial: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM pus_refund WHERE account_id = $1 AND mserial = $2")
            .bind(account_id)
            .bind(serial)
            .execute(self.pool)
            .await?;
        Ok(())
    }
}
