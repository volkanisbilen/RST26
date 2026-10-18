//! Character creation data models — starting equipment and stats per class.
//! - `CDBAgent::LoadNewCharSet()` — inserts starting items for a new character
//! - `CDBAgent::LoadNewCharValue()` — updates starting stats/level/gold for a new character
//! These tables are loaded at server startup and used during character creation
//! to populate the new character's inventory and stat block.

/// A single starting item entry for a class.
/// Maps to `create_new_char_set` table. Each class has 75 slots (0-74),
/// where slot_id 0-13 = equipment, 14+ = inventory bag.
/// Rows with item_id=0 are empty slots.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CreateNewCharSetRow {
    /// Row identifier.
    pub id: i64,
    /// Class type (1=Warrior, 2=Rogue, 3=Mage, 4=Priest, 13=Kurian).
    pub class_type: i16,
    /// Inventory slot index (0-74).
    pub slot_id: i32,
    /// Item number (0 = empty slot).
    pub item_id: i32,
    /// Item durability.
    pub item_duration: i16,
    /// Item stack count.
    pub item_count: i16,
    /// Item flags.
    pub item_flag: i16,
    /// Item expiry time (0=none, 3=premium-timed).
    pub item_expire_time: i32,
}

/// Starting stat/level/gold configuration for a class + job type combination.
/// Maps to `create_new_char_value` table. Each class has entries for job types
/// 0-4 (base, 1st class change, 2nd class change, master, grand master).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CreateNewCharValueRow {
    /// Row identifier.
    pub n_index: i32,
    /// Class type (1=Warrior, 2=Rogue, 3=Mage, 4=Priest, 13=Kurian).
    pub class_type: i16,
    /// Job type (0=base, 1=1st, 2=2nd, 3=master, 4=grand master).
    pub job_type: i16,
    /// Starting level.
    pub level: i16,
    /// Starting experience.
    pub exp: i64,
    /// Starting STR bonus.
    pub strength: i16,
    /// Starting STA/HP bonus.
    pub health: i16,
    /// Starting DEX bonus.
    pub dexterity: i16,
    /// Starting INT bonus.
    pub intelligence: i16,
    /// Starting CHA/magic power bonus.
    pub magic_power: i16,
    /// Free stat points to allocate.
    pub free_points: i16,
    /// Free skill points.
    pub skill_point_free: i16,
    /// Skill points in category 1.
    pub skill_point_cat1: i16,
    /// Skill points in category 2.
    pub skill_point_cat2: i16,
    /// Skill points in category 3.
    pub skill_point_cat3: i16,
    /// Skill points in master category.
    pub skill_point_master: i16,
    /// Starting gold.
    pub gold: i32,
}
