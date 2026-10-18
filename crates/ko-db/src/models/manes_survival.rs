//! Manes Survival PostgreSQL configuration rows.

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ManesSurvivalSpawnRow {
    pub id: i32,
    pub npc_id: i16,
    /// 1=low, 2=mid, 3=high, 4=Dark Dragon.
    pub grade: i16,
    /// 0=normal, 1=low boss, 2=castle boss, 3=Dark Dragon.
    pub boss_tier: i16,
    pub spawn_x: i32,
    pub spawn_z: i32,
    pub spawn_range: i16,
    pub spawn_count: i16,
    pub respawn_seconds: i16,
}

/// Decrypted MANES_MAGIC.tbl row used by the authoritative event runtime.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ManesSurvivalMagicRow {
    /// MANES_MAGIC selection/UI identifier (for example 301 or 6201).
    pub selection_id: i64,
    /// 1..=4 active tier, 5 passive, 100 potion.
    pub kind: i16,
    /// Real MAGIC table identifier used by WIZ_MAGIC_PROCESS.
    pub magic_id: i64,
    pub item_id: i32,
    pub item_count: i16,
    pub price: i32,
    pub hp_bonus: i16,
    pub attack_bonus_pct: i16,
    pub reduction_pct: i16,
}
