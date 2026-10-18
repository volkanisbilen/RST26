//! PostgreSQL rows used by the native v2615 event panels.

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NativeRouletteReward {
    pub roulette_type: i16,
    pub slot: i16,
    pub item_id: i32,
    pub item_count: i16,
    pub weight: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NativeRoulettePending {
    pub pending_type: Option<i16>,
    pub pending_slot: Option<i16>,
    pub pending_item_id: Option<i32>,
    pub pending_item_count: Option<i16>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NativeRouletteHistory {
    pub roulette_type: i16,
    pub item_id: i32,
    pub item_count: i16,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NativeJigsawState {
    pub piece_counts: Vec<i16>,
    pub reward_claimed: Vec<bool>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NativeCoinState {
    pub points: i32,
    pub reward_claimed: Vec<bool>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NativeMarbleState {
    pub position: i16,
    pub laps: i32,
    pub rolls_today: i16,
    pub treasure_claimed: Vec<bool>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NativeMarbleTile {
    pub board_index: i16,
    pub item_id: i32,
    pub item_count: i16,
    pub tile_type: i16,
}

#[derive(Debug, Clone)]
pub struct NativeRewardGrant {
    pub item_id: i32,
    pub item_count: i16,
}
