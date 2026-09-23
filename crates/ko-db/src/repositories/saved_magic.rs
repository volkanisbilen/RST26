//! Saved magic repository — user_saved_magic table access.
//! in `DBAgent.cpp:974-1047`.

use sqlx::{PgPool, QueryBuilder};

use crate::models::SavedMagicRow;

/// Maximum saved buff slots per character.
pub const MAX_SAVED_MAGIC_SLOTS: usize = 10;

/// Minimum duration (seconds) to persist a buff. Buffs under this are discarded.
pub const MIN_SAVED_DURATION: i32 = 5;

/// Maximum duration (seconds) to persist a buff (8 hours).
pub const MAX_SAVED_DURATION: i32 = 28800;

/// Repository for saved magic (buff persistence) database operations.
pub struct SavedMagicRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SavedMagicRepository<'a> {
    /// Create a new saved magic repository.
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Load all saved magic entries for a character.
    ///
    /// Returns only entries with valid skill_id > 0 and duration within range.
    ///
    pub async fn load_saved_magic(
        &self,
        character_id: &str,
    ) -> Result<Vec<SavedMagicRow>, sqlx::Error> {
        sqlx::query_as::<_, SavedMagicRow>(
            "SELECT character_id, slot, skill_id, remaining_duration \
             FROM user_saved_magic \
             WHERE character_id = $1 \
               AND skill_id > 0 \
               AND remaining_duration > $2 \
               AND remaining_duration < $3 \
             ORDER BY slot",
        )
        .bind(character_id)
        .bind(MIN_SAVED_DURATION)
        .bind(MAX_SAVED_DURATION)
        .fetch_all(self.pool)
        .await
    }

    /// Save (upsert) all saved magic entries for a character.
    ///
    /// Replaces all existing entries. Up to 10 entries are saved.
    /// Entries with skill_id == 0 or remaining_duration <= 0 are stored as empty slots.
    ///
    pub async fn save_saved_magic(
        &self,
        character_id: &str,
        entries: &[(u32, i32)], // (skill_id, remaining_duration_secs)
    ) -> Result<(), sqlx::Error> {
        // Replace the character's rows atomically so a failed insert cannot
        // leave the player with no saved scroll buffs after the DELETE.
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM user_saved_magic WHERE character_id = $1")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;

        // Filter valid entries (max 10, non-zero skill, positive duration)
        let valid: Vec<(i16, i32, i32)> = entries
            .iter()
            .enumerate()
            .take(MAX_SAVED_MAGIC_SLOTS)
            .filter(|(_, &(skill_id, duration))| skill_id > 0 && duration > 0)
            .map(|(slot, &(skill_id, duration))| (slot as i16, skill_id as i32, duration))
            .collect();

        if valid.is_empty() {
            tx.commit().await?;
            return Ok(());
        }

        // Batch insert all valid entries in a single query
        let mut builder: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
            "INSERT INTO user_saved_magic (character_id, slot, skill_id, remaining_duration) ",
        );
        builder.push_values(&valid, |mut b, &(slot, skill_id, duration)| {
            b.push_bind(character_id)
                .push_bind(slot)
                .push_bind(skill_id)
                .push_bind(duration);
        });
        builder.build().execute(&mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }

    /// Delete all saved magic entries for a character (e.g., on character delete).
    pub async fn delete_saved_magic(&self, character_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM user_saved_magic WHERE character_id = $1")
            .bind(character_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }
}
