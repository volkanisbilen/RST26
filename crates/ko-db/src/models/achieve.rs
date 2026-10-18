//! Achievement system models — maps to PostgreSQL achievement tables.
//! - `GameDefine.h:2357` — `_ACHIEVE_MAIN`
//! - `GameDefine.h:2407` — `_ACHIEVE_WAR`
//! - `GameDefine.h:2414` — `_ACHIEVE_NORMAL`
//! - `GameDefine.h:2398` — `_ACHIEVE_MONSTER`
//! - `GameDefine.h:2421` — `_ACHIEVE_COM`
//! - `GameDefine.h:2291` — `_ACHIEVE_TITLE`
//! - `shared/database/AchieveMain.h`, `AchieveWar.h`, etc.
//! These tables are bulk-loaded at startup and cached in WorldState.

/// Master achievement definition from the `achieve_main` table.
/// MSSQL source: `ACHIEVE_MAIN` (456 rows).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AchieveMainRow {
    /// Unique achievement index (primary key).
    pub s_index: i32,
    /// Sub-table type: 1=War, 2=Monster, 3=Com, 4=Normal.
    ///
    pub r#type: i16,
    /// Title ID for display.
    pub title_id: i16,
    /// Medal points awarded on completion.
    pub point: i16,
    /// Reward item number (0 = no item reward).
    pub item_num: i32,
    /// Reward item count.
    pub count: i32,
    /// Required zone ID (0 = any zone).
    pub zone_id: i16,
    /// Unknown field (preserved from C++).
    pub unknown2: i16,
    /// Achievement category: 0=Normal, 1=Quest, 2=War, 3=Adventure, 4=Challenge.
    pub achieve_type: i16,
    /// Time limit in seconds for timed challenges (0 = no limit).
    pub req_time: i16,
    /// Sub-category byte 1.
    pub byte1: i16,
    /// Sub-category byte 2 (41/42 = challenge type).
    pub byte2: i16,
}

/// War-type achievement from the `achieve_war` table.
/// MSSQL source: `ACHIEVE_WAR` (92 rows).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AchieveWarRow {
    /// Achievement index (primary key, FK to achieve_main).
    pub s_index: i32,
    /// War sub-type from `UserAchieveWarTypes`.
    pub r#type: i16,
    /// Required count to complete.
    pub s_count: i32,
}

/// Normal-type achievement from the `achieve_normal` table.
/// MSSQL source: `ACHIEVE_NORMAL` (46 rows).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AchieveNormalRow {
    /// Achievement index (primary key, FK to achieve_main).
    pub s_index: i32,
    /// Normal sub-type from `UserAchieveNormalTypes`.
    pub r#type: i16,
    /// Required count to complete.
    pub count: i32,
}

/// Monster-kill achievement from the `achieve_monster` table.
/// MSSQL source: `ACHIEVE_MON` (248 rows).
/// Structure: 2 groups (ACHIEVE_MOB_GROUPS) x 4 monsters (ACHIEVE_MOBS_PER_GROUP) each.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AchieveMonsterRow {
    /// Achievement index (primary key, FK to achieve_main).
    pub s_index: i32,
    /// Monster achievement sub-type.
    pub r#type: i16,
    /// Extra byte field.
    pub byte: i16,
    // Group 1
    pub monster1_1: i32,
    pub monster1_2: i32,
    pub monster1_3: i32,
    pub monster1_4: i32,
    pub mon_count1: i32,
    // Group 2
    pub monster2_1: i32,
    pub monster2_2: i32,
    pub monster2_3: i32,
    pub monster2_4: i32,
    pub mon_count2: i32,
}

/// Composite (requirement-based) achievement from the `achieve_com` table.
/// MSSQL source: `ACHIEVE_COM` (71 rows).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AchieveComRow {
    /// Achievement index (primary key, FK to achieve_main).
    pub s_index: i32,
    /// Composite type: 1=RequireQuest, 2=RequireAchieve.
    pub r#type: i16,
    /// Required quest/achieve ID #1.
    pub req1: i32,
    /// Required quest/achieve ID #2 (0 = none).
    pub req2: i32,
}

/// Title stat bonuses from the `achieve_title` table.
/// MSSQL source: `ACHIEVE_TITLE` (136 rows).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AchieveTitleRow {
    /// Title index (primary key).
    pub s_index: i32,
    pub str: i16,
    pub hp: i16,
    pub dex: i16,
    pub int: i16,
    pub mp: i16,
    pub attack: i16,
    pub defence: i16,
    pub s_loyalty_bonus: i16,
    pub s_exp_bonus: i16,
    pub s_short_sword_ac: i16,
    pub s_jamadar_ac: i16,
    pub s_sword_ac: i16,
    pub s_blow_ac: i16,
    pub s_axe_ac: i16,
    pub s_spear_ac: i16,
    pub s_arrow_ac: i16,
    pub s_fire_bonus: i16,
    pub s_ice_bonus: i16,
    pub s_light_bonus: i16,
    pub s_fire_resist: i16,
    pub s_ice_resist: i16,
    pub s_light_resist: i16,
    pub s_magic_resist: i16,
    pub s_curse_resist: i16,
    pub s_poison_resist: i16,
}

/// Per-player achievement progress from the `user_achieve` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserAchieveRow {
    /// Character name.
    pub str_user_id: String,
    /// Achievement index.
    pub achieve_id: i32,
    /// Status: 0=ChallengeIncomplete, 1=Incomplete, 4=Finished, 5=Completed.
    pub status: i16,
    /// Progress counter (group 1).
    pub count1: i32,
    /// Progress counter (group 2).
    pub count2: i32,
}

/// Per-player achievement summary from the `user_achieve_summary` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserAchieveSummaryRow {
    /// Character name.
    pub str_user_id: String,
    /// Total play time in seconds.
    pub play_time: i32,
    /// Total monsters defeated.
    pub monster_defeat_count: i32,
    /// Total enemy users defeated.
    pub user_defeat_count: i32,
    /// Total deaths to other users.
    pub user_death_count: i32,
    /// Total medal points.
    pub total_medal: i32,
    /// Most recent achievement IDs.
    pub recent_achieve_1: i16,
    pub recent_achieve_2: i16,
    pub recent_achieve_3: i16,
    /// Equipped cover title achievement ID.
    pub cover_id: i16,
    /// Equipped skill title achievement ID.
    pub skill_id: i16,
    /// ACHIEVE_TITLE row the player selected (stat bonus row).
    #[sqlx(default)]
    pub skill_title_row: i16,
}
