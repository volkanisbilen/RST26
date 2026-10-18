//! Siege warfare repository -- loads and updates siege data from PostgreSQL.
//! - `CKnightsSiegeWarfare` in `KnightsSiegeWarFare.h` -- data loading
//! - `CDBAgent::UpdateSiegeTax()` in `DBAgent.cpp:2655` -- tariff updates
//! - `CDBAgent::UpdateSiege()` in `DBAgent.cpp:2645` -- siege state updates

use crate::models::KnightsSiegeWarfareRow;
use crate::DbPool;

/// Repository for `knights_siege_warfare` table access.
pub struct SiegeRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> SiegeRepository<'a> {
    /// Create a new repository backed by the given connection pool.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Load siege warfare data (one row per castle).
    ///
    pub async fn load_all(&self) -> Result<Vec<KnightsSiegeWarfareRow>, sqlx::Error> {
        sqlx::query_as::<_, KnightsSiegeWarfareRow>(
            "SELECT s_castle_index, s_master_knights, by_siege_type, \
             by_war_day, by_war_time, by_war_minute, \
             s_challenge_list_1, s_challenge_list_2, s_challenge_list_3, \
             s_challenge_list_4, s_challenge_list_5, s_challenge_list_6, \
             s_challenge_list_7, s_challenge_list_8, s_challenge_list_9, \
             s_challenge_list_10, \
             by_war_request_day, by_war_request_time, by_war_request_minute, \
             by_guerrilla_war_day, by_guerrilla_war_time, by_guerrilla_war_minute, \
             str_challenge_list, \
             s_moradon_tariff, s_delos_tariff, \
             n_dungeon_charge, n_moradon_tax, n_delos_tax, \
             s_request_list_1, s_request_list_2, s_request_list_3, \
             s_request_list_4, s_request_list_5, s_request_list_6, \
             s_request_list_7, s_request_list_8, s_request_list_9, \
             s_request_list_10 \
             FROM knights_siege_warfare ORDER BY s_castle_index",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Update tariff for a specific zone (Moradon or Delos).
    ///
    pub async fn update_tariff(&self, zone: u8, tariff: i16) -> Result<(), sqlx::Error> {
        if zone == 30 {
            // ZONE_DELOS
            sqlx::query("UPDATE knights_siege_warfare SET s_delos_tariff = $1")
                .bind(tariff)
                .execute(self.pool)
                .await?;
        } else {
            // ZONE_MORADON (21) or any other zone -> moradon tariff
            sqlx::query("UPDATE knights_siege_warfare SET s_moradon_tariff = $1")
                .bind(tariff)
                .execute(self.pool)
                .await?;
        }
        Ok(())
    }

    /// Update tax/charge amounts (after funds collection or revenue reset).
    ///
    pub async fn update_taxes(
        &self,
        moradon_tax: i32,
        delos_tax: i32,
        dungeon_charge: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights_siege_warfare SET \
             n_moradon_tax = $1, n_delos_tax = $2, n_dungeon_charge = $3",
        )
        .bind(moradon_tax)
        .bind(delos_tax)
        .bind(dungeon_charge)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update siege warfare state (master clan, war schedule).
    ///
    pub async fn update_siege(
        &self,
        castle_index: i16,
        master_knights: i16,
        siege_type: i16,
        war_day: i16,
        war_time: i16,
        war_minute: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights_siege_warfare SET \
             s_master_knights = $1, by_siege_type = $2, \
             by_war_day = $3, by_war_time = $4, by_war_minute = $5 \
             WHERE s_castle_index = $6",
        )
        .bind(master_knights)
        .bind(siege_type)
        .bind(war_day)
        .bind(war_time)
        .bind(war_minute)
        .bind(castle_index)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Persist the winning clan without overwriting the scheduled war date/time.
    pub async fn update_master_knights(
        &self,
        castle_index: i16,
        master_knights: i16,
    ) -> Result<(), sqlx::Error> {
        let result = sqlx::query(
            "UPDATE knights_siege_warfare SET s_master_knights = $1 WHERE s_castle_index = $2",
        )
        .bind(master_knights)
        .bind(castle_index)
        .execute(self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }
    /// Update challenge clan list (up to 10 slots).
    ///
    pub async fn update_challenge_list(
        &self,
        castle_index: i16,
        list: &[i16; 10],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights_siege_warfare SET \
             s_challenge_list_1 = $1, s_challenge_list_2 = $2, \
             s_challenge_list_3 = $3, s_challenge_list_4 = $4, \
             s_challenge_list_5 = $5, s_challenge_list_6 = $6, \
             s_challenge_list_7 = $7, s_challenge_list_8 = $8, \
             s_challenge_list_9 = $9, s_challenge_list_10 = $10 \
             WHERE s_castle_index = $11",
        )
        .bind(list[0])
        .bind(list[1])
        .bind(list[2])
        .bind(list[3])
        .bind(list[4])
        .bind(list[5])
        .bind(list[6])
        .bind(list[7])
        .bind(list[8])
        .bind(list[9])
        .bind(castle_index)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update request clan list (up to 10 slots).
    ///
    pub async fn update_request_list(
        &self,
        castle_index: i16,
        list: &[i16; 10],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights_siege_warfare SET \
             s_request_list_1 = $1, s_request_list_2 = $2, \
             s_request_list_3 = $3, s_request_list_4 = $4, \
             s_request_list_5 = $5, s_request_list_6 = $6, \
             s_request_list_7 = $7, s_request_list_8 = $8, \
             s_request_list_9 = $9, s_request_list_10 = $10 \
             WHERE s_castle_index = $11",
        )
        .bind(list[0])
        .bind(list[1])
        .bind(list[2])
        .bind(list[3])
        .bind(list[4])
        .bind(list[5])
        .bind(list[6])
        .bind(list[7])
        .bind(list[8])
        .bind(list[9])
        .bind(castle_index)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update war request schedule fields.
    ///
    pub async fn update_war_request_schedule(
        &self,
        castle_index: i16,
        day: i16,
        time: i16,
        minute: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights_siege_warfare SET \
             by_war_request_day = $1, by_war_request_time = $2, \
             by_war_request_minute = $3 \
             WHERE s_castle_index = $4",
        )
        .bind(day)
        .bind(time)
        .bind(minute)
        .bind(castle_index)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
