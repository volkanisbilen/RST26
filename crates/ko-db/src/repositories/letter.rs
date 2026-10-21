//! Letter (mail) repository — letter table access.

use sqlx::PgPool;

use crate::models::LetterRow;

/// Repository for letter/mail database operations.
pub struct LetterRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> LetterRepository<'a> {
    /// Create a new letter repository.
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Deletes (soft) every letter older than 30 days, taken or not. Returns how many.
    pub async fn purge_expired(&self) -> Result<u64, sqlx::Error> {
        let done = sqlx::query(
            "UPDATE letter SET b_deleted = 1 WHERE b_deleted = 0 AND created_at < NOW() - INTERVAL '30 days'",
        )
        .execute(self.pool)
        .await?;
        Ok(done.rows_affected())
    }

    /// Count unread letters for a character.
    ///
    pub async fn count_unread(&self, recipient: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM letter WHERE recipient_name = $1 AND b_status = 0 AND b_deleted = 0",
        )
        .bind(recipient)
        .fetch_one(self.pool)
        .await?;

        Ok(row.0)
    }

    /// Load letter list for a character.
    ///
    ///
    /// - `new_only=true`: unread letters (b_status=false)
    /// - `new_only=false`: read letters (b_status=true, history)
    pub async fn load_letters(
        &self,
        recipient: &str,
        new_only: bool,
    ) -> Result<Vec<LetterRow>, sqlx::Error> {
        // A letter whose attached item has not been taken stays in "new mail" even after it was
        // opened; it only moves to the read list once the item is out.
        let cond = if new_only {
            "(b_status = 0 OR (b_type = 2 AND item_id > 0 AND item_taken = 0))"
        } else {
            "(b_status = 1 AND NOT (b_type = 2 AND item_id > 0 AND item_taken = 0))"
        };
        let sql = format!(
            "SELECT letter.*, \
                    to_char(created_at, 'YYYYMMDDHH24MISS')::bigint AS created_stamp, \
                    GREATEST(0, 30 - (CURRENT_DATE - created_at::date))::int AS days_left \
             FROM letter WHERE recipient_name = $1 AND b_deleted = 0 AND {cond} \
             ORDER BY created_at DESC LIMIT 127"
        );
        sqlx::query_as::<_, LetterRow>(&sql)
        .bind(recipient)
        .fetch_all(self.pool)
        .await
    }

    /// Read a letter's message body and mark it as read.
    ///
    pub async fn read_letter(
        &self,
        recipient: &str,
        letter_id: i32,
    ) -> Result<Option<String>, sqlx::Error> {
        // First fetch the message
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT message FROM letter WHERE letter_id = $1 AND recipient_name = $2 AND b_deleted = 0",
        )
        .bind(letter_id)
        .bind(recipient)
        .fetch_optional(self.pool)
        .await?;

        if let Some((message,)) = row {
            // Mark as read
            sqlx::query(
                "UPDATE letter SET b_status = 1 WHERE letter_id = $1 AND recipient_name = $2",
            )
            .bind(letter_id)
            .bind(recipient)
            .execute(self.pool)
            .await?;

            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    /// Send a letter.
    ///
    /// Returns true on success.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_letter(
        &self,
        sender: &str,
        recipient: &str,
        subject: &str,
        message: &str,
        b_type: i16,
        item_id: i32,
        item_count: i16,
        item_durability: i16,
        item_serial: i64,
        item_expiry: i32,
        coins: i32,
        send_date: i32,
    ) -> Result<bool, sqlx::Error> {
        // Check recipient exists
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM userdata WHERE str_user_id = $1")
            .bind(recipient)
            .fetch_one(self.pool)
            .await?;

        if exists.0 == 0 {
            return Ok(false);
        }

        // The client shows the date as yymmdd. Some callers pass a unix timestamp: convert it.
        let send_date = if send_date > 991_231 {
            chrono::DateTime::from_timestamp(send_date as i64, 0)
                .map(|d| {
                    use chrono::Datelike;
                    (d.year() % 100) * 10_000 + d.month() as i32 * 100 + d.day() as i32
                })
                .unwrap_or(send_date)
        } else {
            send_date
        };

        sqlx::query(
            "INSERT INTO letter (sender_name, recipient_name, subject, message, b_type, \
             item_id, item_count, item_durability, item_serial, item_expiry, coins, \
             send_date, days_remaining) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 30)",
        )
        .bind(sender)
        .bind(recipient)
        .bind(subject)
        .bind(message)
        .bind(b_type)
        .bind(item_id)
        .bind(item_count)
        .bind(item_durability)
        .bind(item_serial)
        .bind(item_expiry)
        .bind(coins)
        .bind(send_date)
        .execute(self.pool)
        .await?;

        Ok(true)
    }

    /// Get item data from a letter (for LETTER_GET_ITEM).
    ///
    pub async fn get_item_from_letter(
        &self,
        recipient: &str,
        letter_id: i32,
    ) -> Result<Option<LetterRow>, sqlx::Error> {
        sqlx::query_as::<_, LetterRow>(
            "SELECT * FROM letter WHERE letter_id = $1 AND recipient_name = $2 AND b_deleted = 0 AND item_taken = 0",
        )
        .bind(letter_id)
        .bind(recipient)
        .fetch_optional(self.pool)
        .await
    }

    /// Mark item as taken from a letter.
    ///
    pub async fn mark_item_taken(
        &self,
        recipient: &str,
        letter_id: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE letter SET item_taken = 1 WHERE letter_id = $1 AND recipient_name = $2",
        )
        .bind(letter_id)
        .bind(recipient)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Delete a letter (soft delete).
    ///
    pub async fn delete_letter(&self, recipient: &str, letter_id: i32) -> Result<(), sqlx::Error> {
        // a letter with an item that was not taken yet cannot be deleted
        sqlx::query("UPDATE letter SET b_deleted = 1 WHERE letter_id = $1 AND recipient_name = $2 AND NOT (b_type = 2 AND item_id > 0 AND item_taken = 0)")
            .bind(letter_id)
            .bind(recipient)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
