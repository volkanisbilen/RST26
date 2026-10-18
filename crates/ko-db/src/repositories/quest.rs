//! Quest repository — database access for quest helper and quest monster tables.
//! - `GameServerDlg.cpp` — `m_QuestHelperArray`, `m_QuestMonsterArray`
//! - `QuestDatabase.cpp` — `LoadQuestData()`, `UpdateQuestData()`

use crate::models::{QuestHelperRow, QuestMonsterRow, UserQuestRow};
use crate::DbPool;

/// Repository for quest-related DB operations.
pub struct QuestRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> QuestRepository<'a> {
    /// Create a new quest repository.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Load all quest helper definitions.
    ///
    pub async fn load_quest_helpers(&self) -> Result<Vec<QuestHelperRow>, sqlx::Error> {
        sqlx::query_as::<_, QuestHelperRow>("SELECT * FROM quest_helper ORDER BY n_index")
            .fetch_all(self.pool)
            .await
    }

    /// Load all quest monster definitions.
    ///
    pub async fn load_quest_monsters(&self) -> Result<Vec<QuestMonsterRow>, sqlx::Error> {
        sqlx::query_as::<_, QuestMonsterRow>("SELECT * FROM quest_monster ORDER BY s_quest_num")
            .fetch_all(self.pool)
            .await
    }

    /// Load all quest progress for a character.
    ///
    pub async fn load_user_quests(
        &self,
        char_name: &str,
    ) -> Result<Vec<UserQuestRow>, sqlx::Error> {
        sqlx::query_as::<_, UserQuestRow>(
            "SELECT * FROM user_quest WHERE str_user_id = $1 ORDER BY quest_id",
        )
        .bind(char_name)
        .fetch_all(self.pool)
        .await
    }

    /// Save or update a single quest entry for a character.
    ///
    /// Uses upsert (INSERT ON CONFLICT UPDATE) to handle both new and existing quests.
    pub async fn save_user_quest(
        &self,
        char_name: &str,
        quest_id: i16,
        quest_state: i16,
        kill_counts: [i16; 4],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_quest (str_user_id, quest_id, quest_state, kill_count1, kill_count2, kill_count3, kill_count4)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (str_user_id, quest_id) DO UPDATE SET
                quest_state = EXCLUDED.quest_state,
                kill_count1 = EXCLUDED.kill_count1,
                kill_count2 = EXCLUDED.kill_count2,
                kill_count3 = EXCLUDED.kill_count3,
                kill_count4 = EXCLUDED.kill_count4",
        )
        .bind(char_name)
        .bind(quest_id)
        .bind(quest_state)
        .bind(kill_counts[0])
        .bind(kill_counts[1])
        .bind(kill_counts[2])
        .bind(kill_counts[3])
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Persist kill-driven progress monotonically.
    ///
    /// Monster deaths can enqueue several asynchronous writes in a very short
    /// interval. `GREATEST` prevents an older write that finishes later from
    /// rolling a quest counter or its completed state backwards.
    pub async fn save_user_quest_progress(
        &self,
        char_name: &str,
        quest_id: i16,
        quest_state: i16,
        kill_counts: [i16; 4],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_quest (str_user_id, quest_id, quest_state, kill_count1, kill_count2, kill_count3, kill_count4)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (str_user_id, quest_id) DO UPDATE SET
                quest_state = GREATEST(user_quest.quest_state, EXCLUDED.quest_state),
                kill_count1 = GREATEST(user_quest.kill_count1, EXCLUDED.kill_count1),
                kill_count2 = GREATEST(user_quest.kill_count2, EXCLUDED.kill_count2),
                kill_count3 = GREATEST(user_quest.kill_count3, EXCLUDED.kill_count3),
                kill_count4 = GREATEST(user_quest.kill_count4, EXCLUDED.kill_count4)",
        )
        .bind(char_name)
        .bind(quest_id)
        .bind(quest_state)
        .bind(kill_counts[0])
        .bind(kill_counts[1])
        .bind(kill_counts[2])
        .bind(kill_counts[3])
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Batch save or update all quest entries for a character in a single query.
    ///
    /// Each entry: (quest_id, quest_state, [kill_count1..4]).
    pub async fn save_user_quests_batch(
        &self,
        char_name: &str,
        entries: &[(i16, i16, [i16; 4])],
    ) -> Result<(), sqlx::Error> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO user_quest (str_user_id, quest_id, quest_state, kill_count1, kill_count2, kill_count3, kill_count4) ",
        );
        builder.push_values(entries, |mut b, &(qid, state, kc)| {
            b.push_bind(char_name)
                .push_bind(qid)
                .push_bind(state)
                .push_bind(kc[0])
                .push_bind(kc[1])
                .push_bind(kc[2])
                .push_bind(kc[3]);
        });
        builder.push(
            " ON CONFLICT (str_user_id, quest_id) DO UPDATE SET \
             quest_state = EXCLUDED.quest_state, \
             kill_count1 = EXCLUDED.kill_count1, kill_count2 = EXCLUDED.kill_count2, \
             kill_count3 = EXCLUDED.kill_count3, kill_count4 = EXCLUDED.kill_count4",
        );
        builder.build().execute(self.pool).await?;
        Ok(())
    }

    /// Delete a quest entry for a character (quest abandoned/removed).
    ///
    pub async fn delete_user_quest(
        &self,
        char_name: &str,
        quest_id: i16,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM user_quest WHERE str_user_id = $1 AND quest_id = $2")
            .bind(char_name)
            .bind(quest_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }
}
