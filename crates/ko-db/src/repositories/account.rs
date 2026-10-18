//! Account repository — TB_USER and ACCOUNT_CHAR access.

use sqlx::PgPool;

use crate::models::{AccountChar, TbUser};

/// Repository for account-related database operations.
pub struct AccountRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> AccountRepository<'a> {
    /// Create a new account repository.
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Find a user account by account ID.
    pub async fn find_by_account_id(
        &self,
        account_id: &str,
    ) -> Result<Option<TbUser>, sqlx::Error> {
        sqlx::query_as::<_, TbUser>("SELECT * FROM tb_user WHERE str_account_id = $1")
            .bind(account_id)
            .fetch_optional(self.pool)
            .await
    }

    /// Authenticate a user by account ID and password.
    pub async fn authenticate(
        &self,
        account_id: &str,
        password: &str,
    ) -> Result<Option<TbUser>, sqlx::Error> {
        sqlx::query_as::<_, TbUser>(
            "SELECT * FROM tb_user WHERE str_account_id = $1 AND str_passwd = $2",
        )
        .bind(account_id)
        .bind(password)
        .fetch_optional(self.pool)
        .await
    }

    /// Create an account for the automatic first-login registration flow.
    /// Passwords are stored exactly as received because the 2615 login client
    /// sends the same credential representation that authentication compares.
    /// A conflicting account is never overwritten.
    pub async fn create_auto_registered(
        &self,
        account_id: &str,
        password: &str,
        client_ip: &str,
    ) -> Result<Option<TbUser>, sqlx::Error> {
        sqlx::query_as::<_, TbUser>(
            "INSERT INTO tb_user (str_account_id, str_passwd, str_client_ip, str_authority, account_check) \
             VALUES ($1, $2, $3, 1, 1) \
             ON CONFLICT (str_account_id) DO NOTHING \
             RETURNING *",
        )
        .bind(account_id)
        .bind(password)
        .bind(client_ip)
        .fetch_optional(self.pool)
        .await
    }

    /// Get the character list for an account.
    pub async fn get_account_chars(
        &self,
        account_id: &str,
    ) -> Result<Option<AccountChar>, sqlx::Error> {
        sqlx::query_as::<_, AccountChar>("SELECT * FROM account_char WHERE str_account_id = $1")
            .bind(account_id)
            .fetch_optional(self.pool)
            .await
    }

    /// Check if an account is currently logged in.
    pub async fn is_online(&self, account_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM currentuser WHERE str_account_id = $1",
        )
        .bind(account_id)
        .fetch_one(self.pool)
        .await?;
        Ok(result > 0)
    }

    /// Register a user as online (insert into currentuser).
    pub async fn set_online(
        &self,
        account_id: &str,
        char_id: &str,
        server_no: i16,
        server_ip: &str,
        client_ip: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO currentuser (str_account_id, str_char_id, n_server_no, str_server_ip, str_client_ip)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (str_account_id) DO UPDATE SET
                str_char_id = $2, n_server_no = $3, str_server_ip = $4, str_client_ip = $5",
        )
        .bind(account_id)
        .bind(char_id)
        .bind(server_no)
        .bind(server_ip)
        .bind(client_ip)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Remove a user from the online list.
    pub async fn set_offline(&self, account_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM currentuser WHERE str_account_id = $1")
            .bind(account_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Clear all online entries — called at server startup to clean stale sessions
    /// from a previous crash.
    ///
    pub async fn clear_all_online(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM currentuser")
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Create or update nation selection for an account.
    /// Inserts a new account_char row if none exists, otherwise updates the nation.
    pub async fn set_nation(&self, account_id: &str, nation: i16) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO account_char (str_account_id, b_nation)
             VALUES ($1, $2)
             ON CONFLICT (str_account_id) DO UPDATE SET b_nation = $2",
        )
        .bind(account_id)
        .bind(nation)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Assign a character ID to a specific slot (1-4) in account_char.
    pub async fn set_char_slot(
        &self,
        account_id: &str,
        slot: u8,
        char_id: &str,
    ) -> Result<(), sqlx::Error> {
        let col = match slot {
            0 => "str_char_id1",
            1 => "str_char_id2",
            2 => "str_char_id3",
            3 => "str_char_id4",
            _ => return Ok(()),
        };
        let query = format!(
            "UPDATE account_char SET {} = $2, b_char_num = b_char_num + 1 WHERE str_account_id = $1",
            col
        );
        sqlx::query(&query)
            .bind(account_id)
            .bind(char_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Clear a character slot (for deletion) and decrement char count.
    pub async fn clear_char_slot(&self, account_id: &str, slot: u8) -> Result<(), sqlx::Error> {
        let col = match slot {
            0 => "str_char_id1",
            1 => "str_char_id2",
            2 => "str_char_id3",
            3 => "str_char_id4",
            _ => return Ok(()),
        };
        let query = format!(
            "UPDATE account_char SET {} = NULL, b_char_num = GREATEST(b_char_num - 1, 0) WHERE str_account_id = $1",
            col
        );
        sqlx::query(&query)
            .bind(account_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Reorder character slots by writing all 4 slot values at once.
    ///
    /// Each element of `names` is the character name (or `None`) for that slot.
    /// `b_char_num` is NOT modified — only the slot assignments change.
    pub async fn reorder_char_slots(
        &self,
        account_id: &str,
        names: [Option<&str>; 4],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE account_char SET str_char_id1 = $2, str_char_id2 = $3, \
             str_char_id3 = $4, str_char_id4 = $5 WHERE str_account_id = $1",
        )
        .bind(account_id)
        .bind(names[0])
        .bind(names[1])
        .bind(names[2])
        .bind(names[3])
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update account security info (email, phone, seal password, OTP).
    ///
    pub async fn update_account_info(
        &self,
        account_id: &str,
        email: &str,
        phone: &str,
        seal_passwd: &str,
        otp_password: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE tb_user SET email = $1, user_phone_number = $2, \
             str_seal_passwd = $3, otp_password = $4 WHERE str_account_id = $5",
        )
        .bind(email)
        .bind(phone)
        .bind(seal_passwd)
        .bind(otp_password)
        .bind(account_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Find the account ID that owns a given character name.
    ///
    /// Searches all 4 character slots in `account_char`.
    pub async fn find_account_by_char_name(
        &self,
        char_name: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT str_account_id FROM account_char \
             WHERE str_char_id1 = $1 OR str_char_id2 = $1 \
                OR str_char_id3 = $1 OR str_char_id4 = $1 \
             LIMIT 1",
        )
        .bind(char_name)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    /// Update the authority level for an account.
    ///
    /// Used by +block (set to -1) and +unblock (set to 1).
    pub async fn update_authority(
        &self,
        account_id: &str,
        authority: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE tb_user SET str_authority = $1 WHERE str_account_id = $2")
            .bind(authority)
            .bind(account_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Find the first empty character slot (0-3) for an account.
    /// Returns None if all 4 slots are occupied.
    pub async fn find_empty_slot(&self, account_id: &str) -> Result<Option<u8>, sqlx::Error> {
        let ac = self.get_account_chars(account_id).await?;
        match ac {
            None => Ok(Some(0)),
            Some(ac) => {
                if ac.str_char_id1.is_none() {
                    Ok(Some(0))
                } else if ac.str_char_id2.is_none() {
                    Ok(Some(1))
                } else if ac.str_char_id3.is_none() {
                    Ok(Some(2))
                } else if ac.str_char_id4.is_none() {
                    Ok(Some(3))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
