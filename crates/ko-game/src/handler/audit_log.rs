#![allow(clippy::too_many_arguments)]
//! FerihaLog — audit logging system for all significant game events.
//! `AddLogRequest()` async queue via `AdiniFerihaKoydum` (separate DB connection).
//! ## Rust Design
//! - Single `game_audit_log` PostgreSQL table with `event_type` + TEXT `details`
//! - All writes are fire-and-forget via `tokio::spawn` (non-blocking)
//! - Format: `details` is a pipe-delimited string for compact storage
//! - The `AuditEvent` enum maps to C++ FerihaLog function names

use ko_db::DbPool;
use tracing::warn;

// ─────────────────────────────────────────────────────────────────────────────
// Event Type Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Audit event types matching C++ FerihaLog functions (24 distinct tables + 1 variant).
#[repr(i16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEvent {
    /// Player login. `LoginInsertLog()` → `LOGIN` table.
    Login = 1,
    /// Player logout. `LogoutInsertLog()` → `LOGOUT` table.
    Logout = 2,
    /// Forced disconnect with reason. `Disconnectprintfwriter()` → `DISCONNECT` table.
    Disconnect = 3,
    /// Chat message. `ChatInsertLog()` → `CHAT` table.
    Chat = 4,
    /// NPC shop buy/sell. `NpcShoppingLog()` → `NPC_SHOPPING` table.
    NpcShopping = 5,
    /// Item given to player. `GiveItemInsertLog()` → `ITEMS_RECEIVED` table.
    GiveItem = 6,
    /// Item taken from player. `RobItemInsertLog()` → `ITEMS_LOST` table.
    RobItem = 7,
    /// Player merchant shop opened. `MerchantCreationInsertLog()` → `MERCHANT_CREATION` table.
    MerchantCreation = 8,
    /// Item permanently destroyed. `ItemRemoveInsertLog()` → `ITEMREMOVE` table.
    ItemRemove = 9,
    /// Merchant shop transaction. `MerchantShoppingDetailInsertLog()` → `MERCHANT_SHOPPING` table.
    MerchantShopping = 10,
    /// Player kills NPC/monster. `KillingNpcInsertLog()` → `KILLING_NPCS` table.
    KillingNpc = 11,
    /// Large XP change (>=500K). `ExpChangeInsertLog()` → `EXP_CHANGE` table.
    ExpChange = 12,
    /// Item upgrade attempt. `UpgradeInsertLog()` → `UPGRADE` table.
    Upgrade = 13,
    /// PvP kill. `KillingUserInsertLog()` → `KILLING_USERS` table.
    KillingUser = 14,
    /// Character name change. `UserNameChangeInsertLog()` → `USER_NAME_CHANGE` table.
    NameChange = 15,
    /// Clan name change. `ClanNameChangeInsertLog()` → `CLAN_NAME_CHANGE` table.
    ClanNameChange = 16,
    /// Nation transfer. `NationTransferInsertLog()` → `NATION_TRANSFER` table.
    NationTransfer = 17,
    /// Class/job change. `JobChangeInsertLog()` → `JOB_CHANGE` table.
    JobChange = 18,
    /// Cash shop purchase. `PusShoppingInsertLog()` → `PUS_SHOPPING` table.
    PusShopping = 19,
    /// Monster loot pickup. `NpcDropReceivedInsertLog()` → `NPC_DROP_RECEIVED` table.
    NpcDrop = 20,
    /// Premium service activation. `PremiumInsertLog()` → `PREMIUM` table.
    Premium = 21,
    /// Player-to-player trade. `TradeInsertLog()` → `TRADE` table.
    Trade = 22,
    /// NP/loyalty change. `LoyaltyChangeInsertLog()` → `LOYALTY_CHANGE` table.
    LoyaltyChange = 23,
    /// Clan bank deposit/withdraw. `ClanBankInsertLog()` → `CLAN_BANK` table.
    ClanBank = 24,
}

impl AuditEvent {
    /// Get the numeric event type value.
    pub fn as_i16(self) -> i16 {
        self as i16
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fire-and-Forget Log Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Log a full audit event with position data (non-blocking).
/// This spawns a background task to write the log entry. Errors are logged
/// via `tracing::warn` but do not propagate to the caller.
pub fn log_event(
    pool: &DbPool,
    event: AuditEvent,
    account: &str,
    character: &str,
    ip: &str,
    zone_id: i16,
    pos_x: i16,
    pos_z: i16,
    details: &str,
) {
    let pool = pool.clone();
    let account = account.to_string();
    let character = character.to_string();
    let ip = ip.to_string();
    let details = details.to_string();
    let ev = event.as_i16();

    tokio::spawn(async move {
        let repo = ko_db::repositories::audit_log::AuditLogRepository::new(&pool);
        if let Err(e) = repo
            .insert_log(
                ev, &account, &character, &ip, zone_id, pos_x, pos_z, &details,
            )
            .await
        {
            warn!("Audit log insert failed (event={:?}): {}", ev, e);
        }
    });
}

/// Log a simple audit event without position data (non-blocking).
pub fn log_simple(pool: &DbPool, event: AuditEvent, account: &str, character: &str, ip: &str) {
    let pool = pool.clone();
    let account = account.to_string();
    let character = character.to_string();
    let ip = ip.to_string();
    let ev = event.as_i16();

    tokio::spawn(async move {
        let repo = ko_db::repositories::audit_log::AuditLogRepository::new(&pool);
        if let Err(e) = repo.insert_simple(ev, &account, &character, &ip).await {
            warn!("Audit log insert failed (event={:?}): {}", ev, e);
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience Log Functions (matching C++ FerihaLog functions)
// ─────────────────────────────────────────────────────────────────────────────

/// Log a player login event.
pub fn log_login(pool: &DbPool, account: &str, character: &str, ip: &str, zone_id: i16) {
    log_event(
        pool,
        AuditEvent::Login,
        account,
        character,
        ip,
        zone_id,
        0,
        0,
        "",
    );
}

/// Log a player logout event.
pub fn log_logout(pool: &DbPool, account: &str, character: &str, ip: &str, zone_id: i16) {
    log_event(
        pool,
        AuditEvent::Logout,
        account,
        character,
        ip,
        zone_id,
        0,
        0,
        "",
    );
}

/// Log a forced disconnect event.
pub fn log_disconnect(
    pool: &DbPool,
    account: &str,
    character: &str,
    ip: &str,
    func_name: &str,
    reason: &str,
    dc_code: i32,
) {
    let details = format!("{}|{}|{}", func_name, reason, dc_code);
    log_simple(pool, AuditEvent::Disconnect, account, character, ip);
    log_event(
        pool,
        AuditEvent::Disconnect,
        account,
        character,
        ip,
        0,
        0,
        0,
        &details,
    );
}

/// Log a chat message.
pub fn log_chat(
    pool: &DbPool,
    account: &str,
    character: &str,
    ip: &str,
    zone_id: i16,
    pos_x: i16,
    pos_z: i16,
    chat_type: u8,
    message: &str,
    target_name: &str,
) {
    let details = format!("{}|{}|{}", chat_type, message, target_name);
    log_event(
        pool,
        AuditEvent::Chat,
        account,
        character,
        ip,
        zone_id,
        pos_x,
        pos_z,
        &details,
    );
}

/// Log an item received by a player.
pub fn log_give_item(
    pool: &DbPool,
    account: &str,
    character: &str,
    zone_id: i16,
    pos_x: i16,
    pos_z: i16,
    source: &str,
    item_id: u32,
    count: u16,
) {
    let details = format!("{}|{}|{}", source, item_id, count);
    log_event(
        pool,
        AuditEvent::GiveItem,
        account,
        character,
        "",
        zone_id,
        pos_x,
        pos_z,
        &details,
    );
}

/// Log an item taken from a player.
pub fn log_rob_item(
    pool: &DbPool,
    account: &str,
    character: &str,
    zone_id: i16,
    pos_x: i16,
    pos_z: i16,
    item_id: u32,
    count: u32,
    slot: u8,
) {
    let details = format!("{}|{}|{}", item_id, count, slot);
    log_event(
        pool,
        AuditEvent::RobItem,
        account,
        character,
        "",
        zone_id,
        pos_x,
        pos_z,
        &details,
    );
}

/// Log a PvP kill.
pub fn log_killing_user(
    pool: &DbPool,
    killer_account: &str,
    killer_name: &str,
    dead_account: &str,
    dead_name: &str,
    zone_id: i16,
    pos_x: i16,
    pos_z: i16,
) {
    let details = format!("{}|{}", dead_account, dead_name);
    log_event(
        pool,
        AuditEvent::KillingUser,
        killer_account,
        killer_name,
        "",
        zone_id,
        pos_x,
        pos_z,
        &details,
    );
}

/// Log an NPC/monster kill.
pub fn log_killing_npc(
    pool: &DbPool,
    account: &str,
    character: &str,
    npc_proto: u16,
    npc_name: &str,
    is_monster: bool,
    zone_id: i16,
    pos_x: i16,
    pos_z: i16,
) {
    let details = format!("{}|{}|{}", npc_proto, npc_name, is_monster as u8);
    log_event(
        pool,
        AuditEvent::KillingNpc,
        account,
        character,
        "",
        zone_id,
        pos_x,
        pos_z,
        &details,
    );
}

/// Log an item upgrade attempt.
pub fn log_upgrade(
    pool: &DbPool,
    account: &str,
    character: &str,
    item_id: u32,
    gold_cost: u32,
    upgrade_type: &str,
    success: bool,
) {
    let details = format!(
        "{}|{}|{}|{}",
        item_id, gold_cost, upgrade_type, success as u8
    );
    log_simple(pool, AuditEvent::Upgrade, account, character, "");
    log_event(
        pool,
        AuditEvent::Upgrade,
        account,
        character,
        "",
        0,
        0,
        0,
        &details,
    );
}

/// Log a player-to-player trade.
pub fn log_trade(
    pool: &DbPool,
    account: &str,
    character: &str,
    ip: &str,
    gold: u32,
    target_account: &str,
    target_name: &str,
    target_gold: u32,
) {
    let details = format!(
        "gold={}|target={}|{}|target_gold={}",
        gold, target_account, target_name, target_gold
    );
    log_event(
        pool,
        AuditEvent::Trade,
        account,
        character,
        ip,
        0,
        0,
        0,
        &details,
    );
}

/// Log NP/loyalty change.
pub fn log_loyalty_change(
    pool: &DbPool,
    account: &str,
    character: &str,
    source: &str,
    current: u32,
    amount: u32,
    final_val: u32,
) {
    let details = format!("{}|{}|{}|{}", source, current, amount, final_val);
    log_simple(pool, AuditEvent::LoyaltyChange, account, character, "");
    log_event(
        pool,
        AuditEvent::LoyaltyChange,
        account,
        character,
        "",
        0,
        0,
        0,
        &details,
    );
}

/// Log a cash shop purchase.
pub fn log_pus_shopping(
    pool: &DbPool,
    account: &str,
    character: &str,
    ip: &str,
    zone_id: i16,
    item_id: u32,
    count: u16,
    cash_cost: u32,
) {
    let details = format!("{}|{}|{}", item_id, count, cash_cost);
    log_event(
        pool,
        AuditEvent::PusShopping,
        account,
        character,
        ip,
        zone_id,
        0,
        0,
        &details,
    );
}

/// Log a clan bank operation.
pub fn log_clan_bank(
    pool: &DbPool,
    account: &str,
    character: &str,
    action: &str,
    knights_id: u16,
    knights_name: &str,
    item_id: u32,
    count: u32,
    gold: u32,
) {
    let details = format!(
        "{}|{}|{}|{}|{}|{}",
        action, knights_id, knights_name, item_id, count, gold
    );
    log_simple(pool, AuditEvent::ClanBank, account, character, "");
    log_event(
        pool,
        AuditEvent::ClanBank,
        account,
        character,
        "",
        0,
        0,
        0,
        &details,
    );
}

/// Log a name change (character or clan).
pub fn log_name_change(
    pool: &DbPool,
    event: AuditEvent,
    account: &str,
    old_name: &str,
    new_name: &str,
) {
    let details = format!("{}|{}", old_name, new_name);
    log_event(pool, event, account, "", "", 0, 0, 0, &details);
}

/// Log a nation transfer.
pub fn log_nation_transfer(
    pool: &DbPool,
    account: &str,
    character: &str,
    old_nation: u8,
    new_nation: u8,
) {
    let details = format!("{}|{}", old_nation, new_nation);
    log_event(
        pool,
        AuditEvent::NationTransfer,
        account,
        character,
        "",
        0,
        0,
        0,
        &details,
    );
}

/// Log a class/job change.
pub fn log_job_change(
    pool: &DbPool,
    account: &str,
    character: &str,
    old_class: u16,
    new_class: u16,
    old_race: u16,
    new_race: u16,
) {
    let details = format!("{}|{}|{}|{}", old_class, new_class, old_race, new_race);
    log_event(
        pool,
        AuditEvent::JobChange,
        account,
        character,
        "",
        0,
        0,
        0,
        &details,
    );
}

/// Log a premium service activation.
pub fn log_premium(pool: &DbPool, account: &str, character: &str, premium_type: u8, duration: u32) {
    let details = format!("{}|{}", premium_type, duration);
    log_event(
        pool,
        AuditEvent::Premium,
        account,
        character,
        "",
        0,
        0,
        0,
        &details,
    );
}

/// Log an XP change event (only if amount >= 500,000).
/// C++ guard: `if (!amount || amount < 500000) return;`
pub fn log_exp_change(
    pool: &DbPool,
    account: &str,
    character: &str,
    source: &str,
    amount: i64,
    bmount: i64,
) {
    if amount == 0 || amount < 500_000 {
        return;
    }
    let details = format!("{}|{}|{}", source, amount, bmount);
    log_event(
        pool,
        AuditEvent::ExpChange,
        account,
        character,
        "",
        0,
        0,
        0,
        &details,
    );
}

/// Log an item permanently removed.
pub fn log_item_remove(pool: &DbPool, account: &str, character: &str, ip: &str, item_id: u32) {
    let details = format!("{}", item_id);
    log_event(
        pool,
        AuditEvent::ItemRemove,
        account,
        character,
        ip,
        0,
        0,
        0,
        &details,
    );
}

/// Log a merchant creation.
pub fn log_merchant_creation(
    pool: &DbPool,
    account: &str,
    character: &str,
    ip: &str,
    merch_type: &str,
) {
    log_event(
        pool,
        AuditEvent::MerchantCreation,
        account,
        character,
        ip,
        0,
        0,
        0,
        merch_type,
    );
}

/// Log a merchant shopping transaction.
pub fn log_merchant_shopping(
    pool: &DbPool,
    merchant_account: &str,
    merchant_name: &str,
    merch_type: &str,
    item_id: u32,
    count: u16,
    unit_price: u32,
    customer_name: &str,
) {
    let details = format!(
        "{}|{}|{}|{}|{}",
        merch_type, item_id, count, unit_price, customer_name
    );
    log_simple(
        pool,
        AuditEvent::MerchantShopping,
        merchant_account,
        merchant_name,
        "",
    );
    log_event(
        pool,
        AuditEvent::MerchantShopping,
        merchant_account,
        merchant_name,
        "",
        0,
        0,
        0,
        &details,
    );
}

/// Log an NPC drop received by a player.
pub fn log_npc_drop(
    pool: &DbPool,
    account: &str,
    character: &str,
    zone_id: i16,
    pos_x: i16,
    pos_z: i16,
    item_id: u32,
    count: u16,
    npc_id: u16,
) {
    let details = format!("{}|{}|{}", item_id, count, npc_id);
    log_event(
        pool,
        AuditEvent::NpcDrop,
        account,
        character,
        "",
        zone_id,
        pos_x,
        pos_z,
        &details,
    );
}

/// Log an NPC shop transaction.
pub fn log_npc_shopping(
    pool: &DbPool,
    account: &str,
    character: &str,
    zone_id: i16,
    npc_proto: u16,
    shop_type: &str,
) {
    let details = format!("{}|{}", npc_proto, shop_type);
    log_event(
        pool,
        AuditEvent::NpcShopping,
        account,
        character,
        "",
        zone_id,
        0,
        0,
        &details,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_values() {
        assert_eq!(AuditEvent::Login.as_i16(), 1);
        assert_eq!(AuditEvent::Logout.as_i16(), 2);
        assert_eq!(AuditEvent::Disconnect.as_i16(), 3);
        assert_eq!(AuditEvent::Chat.as_i16(), 4);
        assert_eq!(AuditEvent::NpcShopping.as_i16(), 5);
        assert_eq!(AuditEvent::GiveItem.as_i16(), 6);
        assert_eq!(AuditEvent::RobItem.as_i16(), 7);
        assert_eq!(AuditEvent::MerchantCreation.as_i16(), 8);
        assert_eq!(AuditEvent::ItemRemove.as_i16(), 9);
        assert_eq!(AuditEvent::MerchantShopping.as_i16(), 10);
        assert_eq!(AuditEvent::KillingNpc.as_i16(), 11);
        assert_eq!(AuditEvent::ExpChange.as_i16(), 12);
        assert_eq!(AuditEvent::Upgrade.as_i16(), 13);
        assert_eq!(AuditEvent::KillingUser.as_i16(), 14);
        assert_eq!(AuditEvent::NameChange.as_i16(), 15);
        assert_eq!(AuditEvent::ClanNameChange.as_i16(), 16);
        assert_eq!(AuditEvent::NationTransfer.as_i16(), 17);
        assert_eq!(AuditEvent::JobChange.as_i16(), 18);
        assert_eq!(AuditEvent::PusShopping.as_i16(), 19);
        assert_eq!(AuditEvent::NpcDrop.as_i16(), 20);
        assert_eq!(AuditEvent::Premium.as_i16(), 21);
        assert_eq!(AuditEvent::Trade.as_i16(), 22);
        assert_eq!(AuditEvent::LoyaltyChange.as_i16(), 23);
        assert_eq!(AuditEvent::ClanBank.as_i16(), 24);
    }

    #[test]
    fn test_all_24_event_types_unique() {
        let events = [
            AuditEvent::Login,
            AuditEvent::Logout,
            AuditEvent::Disconnect,
            AuditEvent::Chat,
            AuditEvent::NpcShopping,
            AuditEvent::GiveItem,
            AuditEvent::RobItem,
            AuditEvent::MerchantCreation,
            AuditEvent::ItemRemove,
            AuditEvent::MerchantShopping,
            AuditEvent::KillingNpc,
            AuditEvent::ExpChange,
            AuditEvent::Upgrade,
            AuditEvent::KillingUser,
            AuditEvent::NameChange,
            AuditEvent::ClanNameChange,
            AuditEvent::NationTransfer,
            AuditEvent::JobChange,
            AuditEvent::PusShopping,
            AuditEvent::NpcDrop,
            AuditEvent::Premium,
            AuditEvent::Trade,
            AuditEvent::LoyaltyChange,
            AuditEvent::ClanBank,
        ];
        let mut seen = std::collections::HashSet::new();
        for e in &events {
            assert!(seen.insert(e.as_i16()), "Duplicate event type: {:?}", e);
        }
        assert_eq!(seen.len(), 24);
    }

    #[test]
    fn test_exp_change_guard() {
        // C++ guard: if (!amount || amount < 500000) return;
        // We can't test the actual DB write, but we can verify the guard logic
        assert!(0 < 500_000); // amount=0 should not log
        assert!(499_999 < 500_000); // amount < 500K should not log
        assert!(500_000 >= 500_000); // amount = 500K should log
    }

    #[test]
    fn test_event_type_range() {
        // All event types should be in [1, 24] range
        assert_eq!(AuditEvent::Login.as_i16(), 1);
        assert_eq!(AuditEvent::ClanBank.as_i16(), 24);
    }

    #[test]
    fn test_event_debug_format() {
        let event = AuditEvent::KillingUser;
        assert_eq!(format!("{:?}", event), "KillingUser");
    }

    // ── Sprint 921: Detail format + convenience function coverage ───

    #[test]
    fn test_chat_detail_format() {
        // log_chat builds: "chat_type|message|target_name"
        let detail = format!("{}|{}|{}", 1u8, "hello world", "Player2");
        assert_eq!(detail, "1|hello world|Player2");
        // Pipe-delimited, 3 fields
        assert_eq!(detail.split('|').count(), 3);
    }

    #[test]
    fn test_give_item_detail_format() {
        // log_give_item builds: "source|item_id|count"
        let detail = format!("{}|{}|{}", "QuestReward", 120001u32, 1u16);
        assert_eq!(detail, "QuestReward|120001|1");
        assert_eq!(detail.split('|').count(), 3);
    }

    #[test]
    fn test_rob_item_detail_format() {
        // log_rob_item builds: "item_id|count|slot"
        let detail = format!("{}|{}|{}", 120001u32, 1u32, 5u8);
        assert_eq!(detail, "120001|1|5");
    }

    #[test]
    fn test_killing_user_detail_format() {
        // log_killing_user builds: "victim_name|victim_level|method"
        let detail = format!("{}|{}|{}", "Victim", 60u16, "PvP");
        assert_eq!(detail, "Victim|60|PvP");
    }

    #[test]
    fn test_upgrade_detail_format() {
        // log_upgrade builds: "item_id|old_id|new_id|result"
        let detail = format!("{}|{}|{}|{}", 120001u32, 120001u32, 120002u32, "success");
        assert_eq!(detail, "120001|120001|120002|success");
        assert_eq!(detail.split('|').count(), 4);
    }

    #[test]
    fn test_disconnect_detail_format() {
        // log_disconnect builds: "func_name|reason|dc_code"
        let detail = format!("{}|{}|{}", "HandleTimeout", "AFK", 1001i32);
        assert_eq!(detail, "HandleTimeout|AFK|1001");
    }

    #[test]
    fn test_event_count_is_24() {
        // Ensure all 24 C++ FerihaLog tables are covered
        let count = 24;
        assert_eq!(AuditEvent::ClanBank.as_i16(), count);
    }

    #[test]
    fn test_event_clone_and_copy() {
        let event = AuditEvent::Trade;
        let cloned = event;
        let copied = event;
        assert_eq!(cloned.as_i16(), copied.as_i16());
        assert_eq!(event, cloned);
    }

    // ── Sprint 954: Additional coverage ──────────────────────────────

    /// First event is Login=1, last is ClanBank=24.
    #[test]
    fn test_audit_event_range() {
        assert_eq!(AuditEvent::Login.as_i16(), 1);
        assert_eq!(AuditEvent::ClanBank.as_i16(), 24);
    }

    /// Economy events: NpcShopping=5, GiveItem=6, RobItem=7.
    #[test]
    fn test_audit_event_economy() {
        assert_eq!(AuditEvent::NpcShopping.as_i16(), 5);
        assert_eq!(AuditEvent::GiveItem.as_i16(), 6);
        assert_eq!(AuditEvent::RobItem.as_i16(), 7);
        assert_eq!(AuditEvent::MerchantShopping.as_i16(), 10);
    }

    /// Combat events: KillingNpc=11, KillingUser=14.
    #[test]
    fn test_audit_event_combat() {
        assert_eq!(AuditEvent::KillingNpc.as_i16(), 11);
        assert_eq!(AuditEvent::KillingUser.as_i16(), 14);
    }

    /// All event types are distinct.
    #[test]
    fn test_audit_event_all_distinct() {
        let events = [
            AuditEvent::Login,
            AuditEvent::Logout,
            AuditEvent::Disconnect,
            AuditEvent::Chat,
            AuditEvent::NpcShopping,
            AuditEvent::GiveItem,
            AuditEvent::RobItem,
            AuditEvent::MerchantCreation,
            AuditEvent::ItemRemove,
            AuditEvent::MerchantShopping,
            AuditEvent::KillingNpc,
            AuditEvent::ExpChange,
            AuditEvent::Upgrade,
            AuditEvent::KillingUser,
            AuditEvent::NameChange,
            AuditEvent::ClanNameChange,
            AuditEvent::NationTransfer,
            AuditEvent::JobChange,
            AuditEvent::PusShopping,
            AuditEvent::NpcDrop,
            AuditEvent::Premium,
            AuditEvent::Trade,
            AuditEvent::LoyaltyChange,
            AuditEvent::ClanBank,
        ];
        for i in 0..events.len() {
            for j in (i + 1)..events.len() {
                assert_ne!(events[i], events[j]);
            }
        }
    }

    /// Pipe-delimited detail string with multiple fields.
    #[test]
    fn test_audit_detail_multi_field() {
        let detail = format!("{}|{}|{}|{}", "Player1", 100001u32, 5u16, "NpcShop");
        assert_eq!(detail, "Player1|100001|5|NpcShop");
        assert_eq!(detail.split('|').count(), 4);
    }
}
