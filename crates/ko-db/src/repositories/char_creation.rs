//! Character creation repository — loads starting equipment and stats from PostgreSQL.
//! - `CDBAgent::LoadNewCharSet()` — stored procedure `LOAD_NEW_CHAR_SET`
//! - `CDBAgent::LoadNewCharValue()` — stored procedure `LOAD_NEW_CHAR_VALUE`

use crate::models::char_creation::{CreateNewCharSetRow, CreateNewCharValueRow};
use crate::DbPool;

/// Repository for character creation data tables.
pub struct CharCreationRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> CharCreationRepository<'a> {
    /// Create a new repository backed by the given connection pool.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Load all starting equipment entries (375 rows: 5 classes x 75 slots).
    ///
    pub async fn load_all_char_set(&self) -> Result<Vec<CreateNewCharSetRow>, sqlx::Error> {
        sqlx::query_as::<_, CreateNewCharSetRow>(
            "SELECT id::bigint AS id, class_type, slot_id, item_id, item_duration, item_count, \
             item_flag, item_expire_time \
             FROM create_new_char_set ORDER BY class_type, slot_id",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Load all starting stat/level/gold entries (25 rows: 5 classes x 5 job types).
    ///
    pub async fn load_all_char_value(&self) -> Result<Vec<CreateNewCharValueRow>, sqlx::Error> {
        sqlx::query_as::<_, CreateNewCharValueRow>(
            "SELECT n_index, class_type, job_type, level, exp, strength, health, \
             dexterity, intelligence, magic_power, free_points, skill_point_free, \
             skill_point_cat1, skill_point_cat2, skill_point_cat3, skill_point_master, gold \
             FROM create_new_char_value ORDER BY class_type, job_type",
        )
        .fetch_all(self.pool)
        .await
    }

    /// C++ LOAD_NEW_CHAR_VALUE parity: resolve the job type from BEGINNER_SETTINGS.
    pub async fn load_beginner_type(&self, server_no: i16) -> Result<i16, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i16>(
            "SELECT beginner_type FROM (
                 SELECT beginner_type, 0 AS priority FROM beginner_settings WHERE server_no = $1
                 UNION ALL
                 SELECT beginner_type, 1 AS priority FROM beginner_settings WHERE server_no = 1
             ) s ORDER BY priority LIMIT 1",
        )
        .bind(server_no)
        .fetch_optional(self.pool)
        .await?;
        Ok(value.unwrap_or(1).clamp(0, 4))
    }

    /// Load level-specific starting equipment, falling back to the legacy class set.
    pub async fn load_starting_equipment(
        &self,
        class_type: i16,
        beginner_type: i16,
    ) -> Result<Vec<CreateNewCharSetRow>, sqlx::Error> {
        let rows = sqlx::query_as::<_, CreateNewCharSetRow>(
            "SELECT id::bigint AS id, class_type, slot_id, item_id, item_duration, item_count,
                    item_flag, item_expire_time
             FROM create_new_char_set_level
             WHERE class_type = $1 AND beginner_type = $2 AND item_id > 0
             ORDER BY slot_id",
        )
        .bind(class_type)
        .bind(beginner_type)
        .fetch_all(self.pool)
        .await?;
        if !rows.is_empty() {
            return Ok(rows);
        }
        sqlx::query_as::<_, CreateNewCharSetRow>(
            "SELECT id::bigint AS id, class_type, slot_id, item_id, item_duration, item_count,
                    item_flag, item_expire_time
             FROM create_new_char_set
             WHERE class_type = $1 AND item_id > 0
             ORDER BY slot_id",
        )
        .bind(class_type)
        .fetch_all(self.pool)
        .await
    }
}
