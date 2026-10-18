//! Letter (mail) model — maps to `letter` table.

/// A letter row from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LetterRow {
    /// Unique letter ID.
    pub letter_id: i32,
    /// Sender character name.
    pub sender_name: String,
    /// Recipient character name.
    pub recipient_name: String,
    /// Subject line (max 31 chars).
    pub subject: String,
    /// Message body (max 128 chars).
    pub message: String,
    /// Letter type: 1 = text only, 2 = with item.
    pub b_type: i16,
    /// Attached item ID (0 = no item).
    pub item_id: i32,
    /// Attached item count.
    pub item_count: i16,
    /// Attached item durability.
    pub item_durability: i16,
    /// Attached item serial number.
    pub item_serial: i64,
    /// Attached item expiry time.
    pub item_expiry: i32,
    /// Attached coins (disabled in most servers).
    pub coins: i32,
    /// Read status: 0 = unread, 1 = read.
    pub b_status: i16,
    /// Deletion flag: 0 = active, 1 = deleted.
    pub b_deleted: i16,
    /// Whether the attached item has been taken.
    pub item_taken: i16,
    /// Send date in yy*10000 + mm*100 + dd format.
    pub send_date: i32,
    /// Days remaining before auto-delete.
    pub days_remaining: i16,
}
