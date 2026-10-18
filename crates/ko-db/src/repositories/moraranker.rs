//! Database access for the v2615 native MORANKER statues.

use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MorankerCharacterRow {
    pub str_user_id: String,
    pub nation: i16,
    pub race: i16,
    pub class: i16,
    pub face: i16,
    pub hair_rgb: i32,
    pub loyalty: i32,
    pub loyalty_monthly: i32,
    pub str_memo: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MorankerEquipmentRow {
    pub str_user_id: String,
    pub slot_index: i16,
    pub item_id: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MorankerStatueSlotRow {
    pub slot_index: i16,
    pub npc_type: i32,
    pub nation: i16,
    pub x: f32,
    pub z: f32,
    pub direction: i16,
}

pub struct MorankerRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> MorankerRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Top three characters of each nation by current NP.
    pub async fn load_top_six(&self) -> Result<Vec<MorankerCharacterRow>, sqlx::Error> {
        sqlx::query_as::<_, MorankerCharacterRow>(
            "WITH ranked AS (
                SELECT str_user_id, nation, race, class, face, hair_rgb,
                       loyalty, loyalty_monthly, str_memo,
                       ROW_NUMBER() OVER (
                           PARTITION BY nation
                           ORDER BY loyalty DESC, loyalty_monthly DESC,
                                    LOWER(str_user_id), str_user_id
                       ) AS nation_rank
                 FROM userdata
                WHERE nation IN (1, 2) AND authority = 1
            )
            SELECT str_user_id, nation, race, class, face, hair_rgb,
                   loyalty, loyalty_monthly, str_memo
              FROM ranked
             WHERE nation_rank <= 3
             ORDER BY nation, nation_rank",
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn load_equipment(
        &self,
        names: &[&str],
    ) -> Result<Vec<MorankerEquipmentRow>, sqlx::Error> {
        sqlx::query_as::<_, MorankerEquipmentRow>(
            "SELECT str_user_id, slot_index, item_id
               FROM user_items
              WHERE str_user_id = ANY($1)
                AND slot_index = ANY($2)
                AND item_id > 0",
        )
        .bind(names)
        .bind(&[1_i16, 4, 6, 8, 10, 12, 13][..])
        .fetch_all(self.pool)
        .await
    }

    pub async fn load_statue_slots(&self) -> Result<Vec<MorankerStatueSlotRow>, sqlx::Error> {
        sqlx::query_as::<_, MorankerStatueSlotRow>(
            "SELECT slot_index, ASCII(npc_type) AS npc_type, nation, x, z, direction
               FROM moraranker_statue_slot
              ORDER BY slot_index",
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn load_memo(&self, name: &str) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT str_memo FROM userdata WHERE LOWER(str_user_id) = LOWER($1)",
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await
        .map(Option::flatten)
    }

    pub async fn update_memo(&self, name: &str, memo: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE userdata SET str_memo = $2, dt_update_time = NOW()
              WHERE LOWER(str_user_id) = LOWER($1)",
        )
        .bind(name)
        .bind(memo)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
