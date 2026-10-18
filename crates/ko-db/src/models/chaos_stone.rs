//! Chaos Stone event models.
//!                `_CHAOS_STONE_STAGE`, `EVENT_CHAOS_REWARDS` structs

/// A row from `chaos_stone_spawn` — defines a chaos stone spawn point.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChaosStoneSpawnRow {
    /// Primary key index (1-12).
    pub s_index: i16,
    /// Zone ID where the stone spawns (71, 72, 73).
    pub zone_id: i16,
    /// Whether this spawn point is active (1=open, 0=closed).
    pub is_open: bool,
    /// Rank/stage of the chaos stone (1-4 per zone).
    pub rank: i16,
    /// NPC template ID for the chaos stone (8945, 8946, 8947).
    pub chaos_id: i16,
    /// Number of stones to spawn.
    pub count: i16,
    /// X coordinate for spawn.
    pub spawn_x: i16,
    /// Z coordinate for spawn.
    pub spawn_z: i16,
    /// Spawn time multiplier.
    pub spawn_time: i16,
    /// Facing direction.
    pub direction: i16,
    /// Spawn radius range.
    pub radius_range: i16,
}

/// A row from `chaos_stone_summon_list` — monsters summoned on stone death.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChaosStoneSummonListRow {
    /// Primary key index.
    pub n_index: i32,
    /// Zone ID this summon belongs to.
    pub zone_id: i16,
    /// NPC template ID to spawn.
    pub sid: i16,
    /// Monster family grouping for stage progression.
    pub monster_spawn_family: i16,
}

/// A row from `chaos_stone_summon_stage` — stage/family definitions per zone.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChaosStoneSummonStageRow {
    /// Primary key index.
    pub n_index: i16,
    /// Zone ID.
    pub zone_id: i16,
    /// Monster family index for this stage.
    pub index_family: i16,
}

/// A row from `event_chaos_rewards` — rewards distributed by rank.
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct EventChaosRewardRow {
    /// Rank tier (1=best, 18=worst).
    pub rank_id: i16,
    /// First reward item ID.
    pub item_id1: i32,
    /// First reward item count.
    pub item_count1: i32,
    /// First reward item expiration (0=permanent).
    pub item_expiration1: i32,
    /// Second reward item ID.
    pub item_id2: i32,
    /// Second reward item count.
    pub item_count2: i32,
    /// Second reward item expiration.
    pub item_expiration2: i32,
    /// Third reward item ID.
    pub item_id3: i32,
    /// Third reward item count.
    pub item_count3: i32,
    /// Third reward item expiration.
    pub item_expiration3: i32,
    /// Fourth reward item ID.
    pub item_id4: i32,
    /// Fourth reward item count.
    pub item_count4: i32,
    /// Fourth reward item expiration.
    pub item_expiration4: i32,
    /// Fifth reward item ID.
    pub item_id5: i32,
    /// Fifth reward item count.
    pub item_count5: i32,
    /// Fifth reward item expiration.
    pub item_expiration5: i32,
    /// Experience reward.
    pub experience: i32,
    /// Loyalty (NP) reward.
    pub loyalty: i32,
    /// Cash (premium currency) reward.
    pub cash: i32,
    /// Noah (gold) reward.
    pub noah: i32,
}
