//! Cash shop (PUS — Premium User Store) models.
//!
//! Maps to `pus_category`, `pus_items`, and `pus_refund` PostgreSQL tables.
//!
//! - `ShoppingMallHandler.cpp` — STORE_OPEN / STORE_CLOSE flow
//! - PUS is an external web-based shop; items purchased are delivered via
//!   the `WEB_ITEMMALL` / `STORE_CLOSE` flow to the player's inventory.

/// A cash shop category row from the database.
///
/// Source: MSSQL `PUS_CATEGORY` (5 rows).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PusCategoryRow {
    /// Category primary ID.
    pub id: i16,
    /// Display name (e.g., "Scrolls", "Premium-Other").
    pub category_name: String,
    /// Short description.
    pub description: String,
    /// Logical category identifier (matches `pus_items.category`).
    pub category_id: i16,
    /// Whether this category is active (1=active, 0=hidden).
    pub status: i16,
}

/// A cash shop item listing from the database.
///
/// Source: MSSQL `PUS_ITEMS` (137 rows).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PusItemRow {
    /// Unique listing ID.
    pub id: i32,
    /// Game item ID that will be given to the player on purchase.
    pub item_id: i32,
    /// Display name.
    pub item_name: Option<String>,
    /// Title shown in the item popup.
    pub item_title: Option<String>,
    /// Price in the appropriate currency (see `price_type`).
    pub price: Option<i32>,
    /// Delivery method (1 = direct to inventory).
    pub send_type: Option<i32>,
    /// Quantity of items given per purchase.
    pub buy_count: i32,
    /// Description text.
    pub item_desc: String,
    /// Category this item belongs to (FK to `pus_category.category_id`).
    pub category: i16,
    /// Currency type: 0 = Knight Cash (KC), 1 = TL (real money balance).
    pub price_type: i16,
    /// Days until the delivered item expires. 0 = permanent (default).
    pub duration_days: i32,
}

/// A cash shop purchase/refund record.
///
/// Source: MSSQL `PUS_REFUND` (162 rows).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PusRefundRow {
    /// Unique transaction serial number.
    pub mserial: i64,
    /// Account that made the purchase.
    pub account_id: String,
    /// Game item ID purchased.
    pub item_id: i32,
    /// Quantity purchased.
    pub item_count: i16,
    /// Price paid at time of purchase.
    pub item_price: i32,
    /// Unix timestamp of purchase.
    pub buying_time: i32,
    /// Item duration (days, 0=permanent).
    pub item_duration: i16,
    /// Purchase type (0=normal).
    pub buy_type: i16,
}
