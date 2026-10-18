//! Knights (clan) repository — KNIGHTS table access.

use sqlx::PgPool;

use crate::models::{Knights, KnightsAllianceRow};

/// Repository for clan-related database operations.
pub struct KnightsRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> KnightsRepository<'a> {
    /// Create a new knights repository.
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Find a clan by its numeric ID.
    pub async fn find_by_id(&self, id_num: i16) -> Result<Option<Knights>, sqlx::Error> {
        sqlx::query_as::<_, Knights>("SELECT * FROM knights WHERE id_num = $1")
            .bind(id_num)
            .fetch_optional(self.pool)
            .await
    }

    /// Find a clan by name.
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Knights>, sqlx::Error> {
        sqlx::query_as::<_, Knights>("SELECT * FROM knights WHERE id_name = $1")
            .bind(name)
            .fetch_optional(self.pool)
            .await
    }

    /// Load all clans for a given nation.
    pub async fn find_by_nation(&self, nation: i16) -> Result<Vec<Knights>, sqlx::Error> {
        sqlx::query_as::<_, Knights>(
            "SELECT * FROM knights WHERE nation = $1 ORDER BY ranking, points DESC",
        )
        .bind(nation)
        .fetch_all(self.pool)
        .await
    }

    /// Load all clans (for server startup).
    pub async fn load_all(&self) -> Result<Vec<Knights>, sqlx::Error> {
        sqlx::query_as::<_, Knights>("SELECT * FROM knights ORDER BY id_num")
            .fetch_all(self.pool)
            .await
    }

    /// Create a new clan in the database.
    ///
    pub async fn create_knights(
        &self,
        id_num: i16,
        nation: i16,
        name: &str,
        chief: &str,
        flag: i16,
        points: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO knights (id_num, flag, nation, id_name, chief, members, points, s_cape)
             VALUES ($1, $2, $3, $4, $5, 1, $6, CASE WHEN $2 >= 2 THEN 0 ELSE -1 END)",
        )
        .bind(id_num)
        .bind(flag)
        .bind(nation)
        .bind(name)
        .bind(chief)
        .bind(points)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update a user's clan ID and fame in userdata.
    ///
    pub async fn update_user_knights(
        &self,
        char_name: &str,
        knights_id: i16,
        fame: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE userdata SET knights = $1, fame = $2 WHERE str_user_id = $3")
            .bind(knights_id)
            .bind(fame)
            .bind(char_name)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Remove a character from a specific clan (clan-conditional).
    ///
    /// Only clears knights + fame if the target currently belongs to `knights_id`.
    /// Prevents cross-clan removal when target is offline and can't be validated
    /// in-memory. C++ DB stored procedure applies similar clan_id filtering.
    pub async fn remove_from_knights(
        &self,
        char_name: &str,
        knights_id: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE userdata SET knights = 0, fame = 0 WHERE str_user_id = $1 AND knights = $2",
        )
        .bind(char_name)
        .bind(knights_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the member count for a clan.
    pub async fn update_member_count(&self, id_num: i16, members: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET members = $1 WHERE id_num = $2")
            .bind(members)
            .bind(id_num)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Update the chief for a clan (leadership transfer).
    ///
    pub async fn update_chief(&self, id_num: i16, chief: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET chief = $1 WHERE id_num = $2")
            .bind(chief)
            .bind(id_num)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Update a vice chief slot.
    pub async fn update_vice_chiefs(
        &self,
        id_num: i16,
        vc1: Option<&str>,
        vc2: Option<&str>,
        vc3: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights SET vice_chief_1 = $1, vice_chief_2 = $2, vice_chief_3 = $3
             WHERE id_num = $4",
        )
        .bind(vc1)
        .bind(vc2)
        .bind(vc3)
        .bind(id_num)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the clan notice.
    ///
    pub async fn update_notice(&self, id_num: i16, notice: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET str_clan_notice = $1 WHERE id_num = $2")
            .bind(notice)
            .bind(id_num)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Rename a clan in the database.
    ///
    ///
    /// Returns: 3 on success, 2 if new name already exists, 0 on error.
    pub async fn rename_clan(&self, clan_id: i16, new_name: &str) -> Result<u8, sqlx::Error> {
        // Check if the new name already exists
        let existing = self.find_by_name(new_name).await?;
        if existing.is_some() {
            return Ok(2); // Name already taken
        }
        let rows = sqlx::query("UPDATE knights SET id_name = $1 WHERE id_num = $2")
            .bind(new_name)
            .bind(clan_id)
            .execute(self.pool)
            .await?
            .rows_affected();
        if rows == 0 {
            return Ok(0); // Clan not found
        }
        Ok(3) // Success
    }

    /// Delete a clan from the database (disband).
    ///
    pub async fn destroy_knights(&self, id_num: i16) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM knights WHERE id_num = $1")
            .bind(id_num)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Clear the clan from all members in userdata.
    pub async fn clear_all_members(&self, knights_id: i16) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE userdata SET knights = 0, fame = 0 WHERE knights = $1")
            .bind(knights_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Update clan point fund.
    pub async fn update_clan_point_fund(&self, id_num: i16, fund: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET clan_point_fund = $1 WHERE id_num = $2")
            .bind(fund)
            .bind(id_num)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Save clan premium state to DB.
    ///
    pub async fn save_clan_premium(
        &self,
        id_num: i16,
        premium_time: i32,
        premium_in_use: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights SET s_premium_time = $1, s_premium_in_use = $2 WHERE id_num = $3",
        )
        .bind(premium_time)
        .bind(premium_in_use)
        .bind(id_num)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the clan mark (symbol/emblem) data in the DB.
    ///
    pub async fn update_mark(
        &self,
        id_num: i16,
        mark_version: i16,
        mark_data: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights SET mark = $1, s_mark_version = $2, s_mark_len = $3 WHERE id_num = $4",
        )
        .bind(mark_data)
        .bind(mark_version)
        .bind(mark_data.len() as i16)
        .bind(id_num)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Get the count of members in a clan (from userdata).
    pub async fn count_members(&self, knights_id: i16) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM userdata WHERE knights = $1")
            .bind(knights_id)
            .fetch_one(self.pool)
            .await?;
        Ok(row.0)
    }

    /// Load all clan members from userdata for a given clan ID.
    ///
    /// Returns (str_user_id, fame, level, class, loyalty, loyalty_monthly, n_last_login).
    pub async fn load_clan_members(
        &self,
        knights_id: i16,
    ) -> Result<Vec<KnightsMemberRow>, sqlx::Error> {
        sqlx::query_as::<_, KnightsMemberRow>(
            "SELECT str_user_id, fame, level, class, loyalty, loyalty_monthly, n_last_login, str_memo
             FROM userdata WHERE knights = $1 ORDER BY fame DESC, level DESC",
        )
        .bind(knights_id)
        .fetch_all(self.pool)
        .await
    }

    /// Find the next available clan ID for a given nation.
    ///
    pub async fn next_clan_id(&self, nation: u8) -> Result<Option<i16>, sqlx::Error> {
        let (min_id, max_id): (i16, i16) = if nation == 2 {
            // El Morad
            //
            // The reference server reserves the nation auto-clan ID (15001).
            // Real player-created clans must start after it; otherwise v2615
            // treats the clan as a system/auto clan and the cape UI/catalogue
            // does not behave as a normal promoted clan.
            (15002, 32000)
        } else {
            // Karus
            //
            // ID 1 is the Karus auto-clan in the reference server. Do not hand
            // it out to player-created clans.
            (2, 14999)
        };

        let row: Option<(i16,)> = sqlx::query_as(
            "SELECT (COALESCE(MAX(id_num), $1 - 1) + 1)::SMALLINT AS next_id
             FROM knights WHERE id_num >= $1 AND id_num <= $2",
        )
        .bind(min_id)
        .bind(max_id)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some((next_id,)) if next_id <= max_id => Ok(Some(next_id)),
            _ => Ok(Some(min_id)),
        }
    }

    // ── Alliance DB Methods ──────────────────────────────────────────

    /// Load all alliances (for server startup).
    ///
    pub async fn load_all_alliances(&self) -> Result<Vec<KnightsAllianceRow>, sqlx::Error> {
        sqlx::query_as::<_, KnightsAllianceRow>(
            "SELECT * FROM knights_alliance ORDER BY s_main_alliance_knights",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Create a new alliance in the database.
    ///
    pub async fn alliance_create(&self, main_clan: i16, sub_clan: i16) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO knights_alliance (s_main_alliance_knights, s_sub_alliance_knights)
             VALUES ($1, $2)
             ON CONFLICT (s_main_alliance_knights) DO NOTHING",
        )
        .bind(main_clan)
        .bind(sub_clan)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Insert a clan into an existing alliance.
    ///
    /// `slot`: 1=sub, 2=merc1, 3=merc2
    pub async fn alliance_insert(
        &self,
        main_clan: i16,
        target_clan: i16,
        slot: u8,
    ) -> Result<(), sqlx::Error> {
        let col = match slot {
            1 => "s_sub_alliance_knights",
            2 => "s_mercenary_clan_1",
            3 => "s_mercenary_clan_2",
            _ => return Ok(()),
        };
        let query = format!(
            "UPDATE knights_alliance SET {} = $1 WHERE s_main_alliance_knights = $2",
            col
        );
        sqlx::query(&query)
            .bind(target_clan)
            .bind(main_clan)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Remove a clan from an alliance (set its slot to 0).
    ///
    pub async fn alliance_remove(
        &self,
        main_clan: i16,
        target_clan: i16,
    ) -> Result<(), sqlx::Error> {
        // Clear whichever slot the target clan occupies
        sqlx::query(
            "UPDATE knights_alliance SET
                s_sub_alliance_knights = CASE WHEN s_sub_alliance_knights = $2 THEN 0 ELSE s_sub_alliance_knights END,
                s_mercenary_clan_1 = CASE WHEN s_mercenary_clan_1 = $2 THEN 0 ELSE s_mercenary_clan_1 END,
                s_mercenary_clan_2 = CASE WHEN s_mercenary_clan_2 = $2 THEN 0 ELSE s_mercenary_clan_2 END
             WHERE s_main_alliance_knights = $1",
        )
        .bind(main_clan)
        .bind(target_clan)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Destroy an alliance entirely.
    ///
    pub async fn alliance_destroy(&self, main_clan: i16) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM knights_alliance WHERE s_main_alliance_knights = $1")
            .bind(main_clan)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Update a clan's alliance ID.
    pub async fn update_alliance_id(
        &self,
        clan_id: i16,
        alliance_id: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET s_alliance_knights = $1 WHERE id_num = $2")
            .bind(alliance_id)
            .bind(clan_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Update the clan point method.
    ///
    pub async fn update_clan_point_method(
        &self,
        id_num: i16,
        method: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET clan_point_method = $1 WHERE id_num = $2")
            .bind(method)
            .bind(id_num)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Update a user's memo in userdata.
    ///
    pub async fn update_user_memo(&self, char_name: &str, memo: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE userdata SET str_memo = $1 WHERE str_user_id = $2")
            .bind(memo)
            .bind(char_name)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Update the alliance notice text.
    ///
    pub async fn update_alliance_notice(
        &self,
        main_clan: i16,
        notice: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE knights_alliance SET str_alliance_notice = $1 WHERE s_main_alliance_knights = $2",
        )
        .bind(notice)
        .bind(main_clan)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Transfer clan leadership in the database.
    ///
    pub async fn handover_leadership(
        &self,
        clan_id: i16,
        new_chief: &str,
        old_chief: &str,
    ) -> Result<(), sqlx::Error> {
        // Clear new chief from vice chief slots + set as chief
        sqlx::query(
            "UPDATE knights SET chief = $1,
                vice_chief_1 = CASE WHEN vice_chief_1 = $1 THEN NULL ELSE vice_chief_1 END,
                vice_chief_2 = CASE WHEN vice_chief_2 = $1 THEN NULL ELSE vice_chief_2 END,
                vice_chief_3 = CASE WHEN vice_chief_3 = $1 THEN NULL ELSE vice_chief_3 END
             WHERE id_num = $2",
        )
        .bind(new_chief)
        .bind(clan_id)
        .execute(self.pool)
        .await?;

        // Update fame: new chief = CHIEF(1), old chief = TRAINEE(5)
        sqlx::query("UPDATE userdata SET fame = 1 WHERE str_user_id = $1 AND knights = $2")
            .bind(new_chief)
            .bind(clan_id)
            .execute(self.pool)
            .await?;
        sqlx::query("UPDATE userdata SET fame = 5 WHERE str_user_id = $1 AND knights = $2")
            .bind(old_chief)
            .bind(clan_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Update a clan's type flag and normal cape after promotion/demotion.
    ///
    pub async fn update_flag_cape(
        &self,
        id_num: i16,
        flag: i16,
        cape: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET flag = $1, s_cape = $2 WHERE id_num = $3")
            .bind(flag)
            .bind(cape)
            .bind(id_num)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Update a clan's nation for clan-wide nation transfer (ClanNts).
    ///
    pub async fn save_clan_nation(&self, clan_id: i16, new_nation: i16) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE knights SET nation = $1 WHERE id_num = $2")
            .bind(new_nation)
            .bind(clan_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Get distinct account IDs for a set of character names.
    ///
    /// Used by ClanNts to find all accounts of clan members.
    pub async fn get_member_account_ids(&self, clan_id: i16) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT ac.str_account_id
             FROM userdata u
             INNER JOIN account_char ac
               ON u.str_user_id IN (ac.str_char_id1, ac.str_char_id2, ac.str_char_id3, ac.str_char_id4)
             WHERE u.knights = $1",
        )
        .bind(clan_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}

/// A lightweight row for clan member listing.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct KnightsMemberRow {
    pub str_user_id: String,
    pub fame: i16,
    pub level: i16,
    pub class: i16,
    pub loyalty: i32,
    pub loyalty_monthly: i32,
    pub n_last_login: i32,
    pub str_memo: Option<String>,
}
