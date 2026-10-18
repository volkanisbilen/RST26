//! Trade, exchange, and merchant shop management.

use super::*;

impl WorldState {
    // ── Trade (Exchange) Methods ──────────────────────────────────────

    /// Check if a player is currently in an active trade.
    ///
    pub fn is_trading(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.exchange_user.is_some())
            .unwrap_or(false)
    }
    /// Get the trade state for a session.
    pub fn get_trade_state(&self, sid: SessionId) -> u8 {
        self.sessions
            .get(&sid)
            .map(|h| h.trade_state)
            .unwrap_or(TRADE_STATE_NONE)
    }
    /// Get the exchange partner for a session.
    pub fn get_exchange_user(&self, sid: SessionId) -> Option<SessionId> {
        self.sessions.get(&sid).and_then(|h| h.exchange_user)
    }
    /// Set up a trade request between two players.
    pub fn init_trade_request(&self, sender: SessionId, target: SessionId) {
        if let Some(mut h) = self.sessions.get_mut(&sender) {
            h.exchange_user = Some(target);
            h.trade_state = TRADE_STATE_SENDER;
            h.is_request_sender = true;
        }
        if let Some(mut h) = self.sessions.get_mut(&target) {
            h.exchange_user = Some(sender);
            h.trade_state = TRADE_STATE_TARGET;
        }
    }
    /// Advance both players to the TRADING state after agree.
    pub fn trade_agree(&self, sid: SessionId) {
        let partner = self.get_exchange_user(sid);
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.trade_state = TRADE_STATE_TRADING;
        }
        if let Some(pid) = partner {
            if let Some(mut h) = self.sessions.get_mut(&pid) {
                h.trade_state = TRADE_STATE_TRADING;
            }
        }
    }
    /// Decline trade (reset both).
    pub fn trade_decline(&self, sid: SessionId) {
        let partner = self.get_exchange_user(sid);
        self.reset_trade(sid);
        if let Some(pid) = partner {
            self.reset_trade(pid);
        }
    }
    /// Reset all trade state for a single session.
    ///
    pub fn reset_trade(&self, sid: SessionId) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.trade_state = TRADE_STATE_NONE;
            h.exchange_user = None;
            h.exchange_items.clear();
            h.is_request_sender = false;
        }
    }
    /// Add an item to a player's exchange list.
    pub fn add_exchange_item(&self, sid: SessionId, item: ExchangeItem) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.exchange_items.push(item);
        }
    }
    /// Get exchange items for a session (cloned).
    pub fn get_exchange_items(&self, sid: SessionId) -> Vec<ExchangeItem> {
        self.sessions
            .get(&sid)
            .map(|h| h.exchange_items.clone())
            .unwrap_or_default()
    }
    /// Update the count of an existing exchange item. Returns true if found and updated.
    pub fn update_exchange_item_count(&self, sid: SessionId, item_id: u32, add_count: u32) -> bool {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            for ex in h.exchange_items.iter_mut() {
                if ex.item_id == item_id {
                    ex.count = ex.count.saturating_add(add_count);
                    return true;
                }
            }
        }
        false
    }
    /// Set trade state for a session.
    pub fn set_trade_state(&self, sid: SessionId, state: u8) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.trade_state = state;
        }
    }
    /// Give back all exchange items to a player (cancel/fail).
    ///
    pub fn exchange_give_items_back(&self, sid: SessionId) {
        let items = {
            let handle = match self.sessions.get(&sid) {
                Some(h) => h,
                None => return,
            };
            handle.exchange_items.clone()
        };

        for ex_item in &items {
            if ex_item.item_id == ITEM_GOLD {
                self.gold_gain(sid, ex_item.count);
            } else {
                self.update_inventory(sid, |inv| {
                    let pos = ex_item.src_pos as usize;
                    if pos >= inv.len() {
                        return false;
                    }
                    if inv[pos].item_id == ex_item.item_id {
                        // Partial stack — slot still has the item, add count back
                        inv[pos].count = (inv[pos].count + ex_item.count as u16).min(ITEMCOUNT_MAX);
                    } else if inv[pos].item_id == 0 {
                        // Full stack was traded — slot was cleared to default.
                        // C++ never clears item_id on trade deduct (TradeHandler.cpp:313),
                        // but our exchange_add clears slot at count=0. Restore fully.
                        inv[pos].item_id = ex_item.item_id;
                        inv[pos].count = ex_item.count as u16;
                        inv[pos].durability = ex_item.durability;
                        inv[pos].serial_num = ex_item.serial_num;
                    }
                    // If slot has a different item (shouldn't happen during trade), skip
                    true
                });
            }
        }
    }
    // ── Merchant Methods ────────────────────────────────────────────

    /// Check if a player is in any merchant state.
    ///
    pub fn is_merchanting(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.merchant_state != MERCHANT_STATE_NONE)
            .unwrap_or_else(|| self.get_bot(sid as u32).is_some_and(|b| b.is_merchanting()))
    }
    /// Check if a player is currently mining.
    ///
    pub fn is_mining(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.is_mining)
            .unwrap_or(false)
    }
    /// Check if a player is currently fishing.
    ///
    pub fn is_fishing(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.is_fishing)
            .unwrap_or(false)
    }
    /// Check if a player is preparing a selling merchant.
    pub fn is_selling_merchant_preparing(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.selling_merchant_preparing)
            .unwrap_or(false)
    }
    /// Check if a player is preparing a buying merchant.
    pub fn is_buying_merchant_preparing(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.buying_merchant_preparing)
            .unwrap_or(false)
    }
    /// Check if a player is an active selling merchant.
    pub fn is_selling_merchant(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.merchant_state == MERCHANT_STATE_SELLING)
            .unwrap_or_else(|| {
                self.get_bot(sid as u32)
                    .is_some_and(|b| b.merchant_state == 0)
            })
    }
    /// Set selling merchant preparing state.
    pub fn set_selling_merchant_preparing(&self, sid: SessionId, val: bool) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.selling_merchant_preparing = val;
        }
    }
    /// Set a merchant item slot.
    pub fn set_merchant_item(&self, sid: SessionId, slot: usize, data: MerchData) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            if slot < MAX_MERCH_ITEMS {
                h.merchant_items[slot] = data;
            }
        }
    }
    /// Get a merchant item slot (cloned).
    pub fn get_merchant_item(&self, sid: SessionId, slot: usize) -> Option<MerchData> {
        self.sessions
            .get(&sid)
            .and_then(|h| {
                if slot < MAX_MERCH_ITEMS {
                    Some(h.merchant_items[slot].clone())
                } else {
                    None
                }
            })
            .or_else(|| {
                self.get_bot(sid as u32)
                    .and_then(|b| (slot < MAX_MERCH_ITEMS).then(|| b.merchant_items[slot].clone()))
            })
    }
    /// Get all merchant items for a session (cloned).
    pub fn get_merchant_items(&self, sid: SessionId) -> [MerchData; MAX_MERCH_ITEMS] {
        self.sessions
            .get(&sid)
            .map(|h| h.merchant_items.clone())
            .or_else(|| self.get_bot(sid as u32).map(|b| b.merchant_items.clone()))
            .unwrap_or_default()
    }
    /// Activate selling merchant state (after insert).
    pub fn activate_selling_merchant(&self, sid: SessionId) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.merchant_state = MERCHANT_STATE_SELLING;
            h.selling_merchant_preparing = false;
        }
    }
    /// Close merchant state and clear items.
    pub fn close_merchant(&self, sid: SessionId) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            // Unmark inventory items
            for slot in h.inventory.iter_mut() {
                slot.flag &= !0x10; // Clear merchant flag bit
            }
            h.merchant_state = MERCHANT_STATE_NONE;
            h.selling_merchant_preparing = false;
            h.buying_merchant_preparing = false;
            h.merchant_items = Default::default();
            h.merchant_looker = None;
        } else {
            self.update_bot(sid as u32, |bot| {
                bot.merchant_state = MERCHANT_STATE_NONE;
                bot.merchant_items = Default::default();
                bot.merchant_looker = None;
            });
        }
    }
    /// Set the merchant looker (who is browsing my shop).
    pub fn set_merchant_looker(&self, merchant_sid: SessionId, looker: Option<SessionId>) {
        if let Some(mut h) = self.sessions.get_mut(&merchant_sid) {
            h.merchant_looker = looker;
        } else {
            self.update_bot(merchant_sid as u32, |bot| bot.merchant_looker = looker);
        }
    }
    /// Get the merchant looker for a session.
    pub fn get_merchant_looker(&self, sid: SessionId) -> Option<SessionId> {
        self.sessions
            .get(&sid)
            .and_then(|h| h.merchant_looker)
            .or_else(|| self.get_bot(sid as u32).and_then(|b| b.merchant_looker))
    }
    /// Set which merchant shop this player is browsing.
    pub fn set_browsing_merchant(&self, sid: SessionId, merchant: Option<SessionId>) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.browsing_merchant = merchant;
        }
    }
    /// Get which merchant shop this player is browsing.
    pub fn get_browsing_merchant(&self, sid: SessionId) -> Option<SessionId> {
        self.sessions.get(&sid).and_then(|h| h.browsing_merchant)
    }
    /// Restore a merchant slot after a failed buy (undo `try_merchant_buy`).
    /// Re-adds the count back and clears `sold_out` if it was set.
    pub fn restore_merchant_buy(
        &self,
        merchant_sid: SessionId,
        item_slot: usize,
        expected_item_id: u32,
        count: u16,
    ) {
        if let Some(mut h) = self.sessions.get_mut(&merchant_sid) {
            if item_slot < MAX_MERCH_ITEMS {
                let slot = &mut h.merchant_items[item_slot];
                if slot.item_id == expected_item_id || (slot.sold_out && slot.item_id == 0) {
                    if slot.item_id == 0 {
                        slot.item_id = expected_item_id;
                    }
                    slot.sell_count = slot.sell_count.saturating_add(count);
                    slot.sold_out = false;
                }
            }
        } else {
            self.update_bot(merchant_sid as u32, |bot| {
                if item_slot < MAX_MERCH_ITEMS {
                    let slot = &mut bot.merchant_items[item_slot];
                    if slot.item_id == expected_item_id {
                        slot.sell_count = slot.sell_count.saturating_add(count);
                        slot.sold_out = false;
                    }
                }
            });
        }
    }

    /// Atomically attempt to buy items from a merchant slot.
    /// Checks that the slot has the expected item_id with enough stock,
    /// then decrements the count (or marks sold out) under the DashMap write lock.
    /// Returns `Some(merch_data_before_buy)` on success, `None` if already sold/mismatched.
    /// This prevents two concurrent buyers from both succeeding on the last item.
    pub fn try_merchant_buy(
        &self,
        merchant_sid: SessionId,
        item_slot: usize,
        expected_item_id: u32,
        buy_count: u16,
    ) -> Option<MerchData> {
        if let Some(mut h) = self.sessions.get_mut(&merchant_sid) {
            if item_slot >= MAX_MERCH_ITEMS {
                return None;
            }
            let slot = &mut h.merchant_items[item_slot];
            if slot.item_id == 0
                || slot.item_id != expected_item_id
                || slot.sold_out
                || slot.sell_count < buy_count
                || slot.price == 0
            {
                return None;
            }
            // Snapshot before modification
            let snapshot = slot.clone();
            // Decrement count
            slot.sell_count = slot.sell_count.saturating_sub(buy_count);
            if slot.sell_count == 0 {
                slot.sold_out = true;
            }
            Some(snapshot)
        } else if let Some(mut bot) = self.bots.get_mut(&(merchant_sid as u32)) {
            if item_slot >= MAX_MERCH_ITEMS {
                return None;
            }
            let slot = &mut bot.merchant_items[item_slot];
            if slot.item_id == 0
                || slot.item_id != expected_item_id
                || slot.sold_out
                || slot.sell_count < buy_count
                || slot.price == 0
            {
                return None;
            }
            let snapshot = slot.clone();
            slot.sell_count = slot.sell_count.saturating_sub(buy_count);
            slot.sold_out = slot.sell_count == 0;
            Some(snapshot)
        } else {
            None
        }
    }

    /// Check if a player is an active buying merchant.
    pub fn is_buying_merchant(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.merchant_state == MERCHANT_STATE_BUYING)
            .unwrap_or_else(|| {
                self.get_bot(sid as u32)
                    .is_some_and(|b| b.merchant_state == 1)
            })
    }

    /// Set buying merchant preparing state.
    pub fn set_buying_merchant_preparing(&self, sid: SessionId, val: bool) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.buying_merchant_preparing = val;
        }
    }

    /// Activate buying merchant state (after insert).
    pub fn activate_buying_merchant(&self, sid: SessionId) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.merchant_state = MERCHANT_STATE_BUYING;
            h.buying_merchant_preparing = false;
        }
    }

    /// Close buying merchant state and clear items.
    pub fn close_buying_merchant(&self, sid: SessionId) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            // Notify looker if any
            let _looker = h.merchant_looker.take();
            h.merchant_state = MERCHANT_STATE_NONE;
            h.buying_merchant_preparing = false;
            h.merchant_items = Default::default();
        }
    }

    /// Remove this player from the merchant they are currently browsing.
    pub fn remove_from_merchant_lookers(&self, sid: SessionId) {
        let merchant_sid = self.get_browsing_merchant(sid);
        if let Some(msid) = merchant_sid {
            // Clear the merchant's looker if it's us
            if self.get_merchant_looker(msid) == Some(sid) {
                self.set_merchant_looker(msid, None);
            }
        }
        self.set_browsing_merchant(sid, None);
    }

    // ── Offline Merchant Methods ──────────────────────────────────────

    /// Check if a session is in offline merchant mode.
    ///
    pub fn is_offline_status(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| h.is_offline)
            .unwrap_or(false)
    }

    /// Activate offline status for a merchant session.
    ///
    ///
    /// Checks the CFAIRY slot for a valid offline merchant item, then sets the
    /// offline flag, type, and timer.  Returns `true` if activation succeeded.
    pub fn activate_offline_status(
        &self,
        sid: SessionId,
        offline_type: OfflineCharacterType,
    ) -> bool {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            // Check CFAIRY slot for required item
            let required_item = match offline_type {
                OfflineCharacterType::Merchant => {
                    let cfairy_item = h.inventory.get(CFAIRY_SLOT).map(|s| s.item_id).unwrap_or(0);
                    if cfairy_item == MERCHANT_AUTO_FISHING {
                        MERCHANT_AUTO_FISHING
                    } else if cfairy_item == MERCHANT_AUTO_MANING {
                        MERCHANT_AUTO_MANING
                    } else {
                        OFFLINE_MERCHANT_ITEM
                    }
                }
                _ => return false, // Only merchant type supported for now
            };

            let cfairy_item = h.inventory.get(CFAIRY_SLOT).map(|s| s.item_id).unwrap_or(0);

            if cfairy_item != required_item {
                return false;
            }

            h.is_offline = true;
            h.offline_type = offline_type;
            h.offline_remaining_minutes = OFFLINE_DEFAULT_MINUTES;
            h.offline_next_check =
                Some(Instant::now() + std::time::Duration::from_secs(OFFLINE_CHECK_INTERVAL_SECS));
            true
        } else {
            false
        }
    }

    /// Deactivate offline status for a session.
    ///
    pub fn deactivate_offline_status(&self, sid: SessionId) {
        if let Some(mut h) = self.sessions.get_mut(&sid) {
            h.is_offline = false;
            h.offline_remaining_minutes = 0;
            h.offline_next_check = None;
        }
    }

    /// Get the remaining offline minutes for a session.
    ///
    pub fn get_offline_remaining_minutes(&self, sid: SessionId) -> u16 {
        self.sessions
            .get(&sid)
            .map(|h| h.offline_remaining_minutes)
            .unwrap_or(0)
    }

    /// Tick offline merchants — decrement remaining minutes for all offline sessions
    /// whose check interval has elapsed.
    ///
    ///
    /// Returns a list of session IDs whose offline time has expired (should be
    /// disconnected).
    pub fn tick_offline_merchants(&self) -> Vec<SessionId> {
        let now = Instant::now();
        let mut expired = Vec::new();

        for mut entry in self.sessions.iter_mut() {
            let h = entry.value_mut();
            if !h.is_offline {
                continue;
            }

            if let Some(next_check) = h.offline_next_check {
                if now >= next_check {
                    if h.offline_remaining_minutes == 0 {
                        expired.push(*entry.key());
                        continue;
                    }
                    h.offline_remaining_minutes = h.offline_remaining_minutes.saturating_sub(1);
                    if h.offline_remaining_minutes == 0 {
                        expired.push(*entry.key());
                    } else {
                        h.offline_next_check =
                            Some(now + std::time::Duration::from_secs(OFFLINE_CHECK_INTERVAL_SECS));
                    }
                }
            }
        }

        expired
    }

    /// Check if all merchant items for an offline session have been sold out.
    ///
    /// Used after a buy transaction to auto-close the merchant when everything is sold.
    pub fn are_all_merchant_items_sold(&self, sid: SessionId) -> bool {
        self.sessions
            .get(&sid)
            .map(|h| {
                // At least one item must have been listed
                let has_any = h.merchant_items.iter().any(|m| m.item_id > 0 || m.sold_out);
                if !has_any {
                    return false;
                }
                // All listed items must be sold out
                h.merchant_items
                    .iter()
                    .filter(|m| m.item_id > 0 || m.sold_out)
                    .all(|m| m.sold_out)
            })
            .unwrap_or(false)
    }

    /// Collect all active merchant items across all online sessions.
    ///
    /// Returns tuples of (session_id, name, is_selling, item_id, sell_count, price, is_kc, x, y, z).
    /// Used by the Menissiah merchant list system (ext_hook 0xCA).
    #[allow(clippy::type_complexity)]
    pub fn collect_merchant_items(
        &self,
    ) -> Vec<(SessionId, String, bool, u32, u16, u32, bool, f32, f32, f32)> {
        let mut items = Vec::new();
        for entry in self.sessions.iter() {
            let h = entry.value();
            if h.merchant_state == MERCHANT_STATE_NONE {
                continue;
            }
            let is_selling = h.merchant_state == MERCHANT_STATE_SELLING;
            for merch in &h.merchant_items {
                if merch.item_id == 0 || merch.price == 0 || merch.sold_out {
                    continue;
                }
                items.push((
                    *entry.key(),
                    h.character
                        .as_ref()
                        .map(|c| c.name.clone())
                        .unwrap_or_default(),
                    is_selling,
                    merch.item_id,
                    merch.sell_count,
                    merch.price,
                    merch.is_kc,
                    h.position.x,
                    h.position.y,
                    h.position.z,
                ));
            }
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn make_world_with_sessions(ids: &[SessionId]) -> WorldState {
        let world = WorldState::new();
        for &id in ids {
            let (tx, _rx) = mpsc::unbounded_channel::<Arc<ko_protocol::Packet>>();
            world.register_session(id, tx);
        }
        world
    }

    // ── Trade State Constants ────────────────────────────────────────

    #[test]
    fn test_trade_state_constants() {
        assert_eq!(TRADE_STATE_NONE, 1);
        assert_eq!(TRADE_STATE_SENDER, 2);
        assert_eq!(TRADE_STATE_TARGET, 3);
        assert_eq!(TRADE_STATE_TRADING, 4);
        assert_eq!(TRADE_STATE_DECIDING, 5);
    }

    #[test]
    fn test_merchant_state_constants() {
        assert_eq!(MERCHANT_STATE_NONE, -1);
        assert_eq!(MERCHANT_STATE_SELLING, 0);
        assert_eq!(MERCHANT_STATE_BUYING, 1);
        assert_eq!(MAX_MERCH_ITEMS, 12);
    }

    // ── Trade Flow Tests ────────────────────────────────────────────

    #[test]
    fn test_initial_trade_state_is_none() {
        let world = make_world_with_sessions(&[100, 200]);
        assert_eq!(world.get_trade_state(100), TRADE_STATE_NONE);
        assert!(!world.is_trading(100));
        assert_eq!(world.get_exchange_user(100), None);
    }

    #[test]
    fn test_init_trade_request() {
        let world = make_world_with_sessions(&[100, 200]);
        world.init_trade_request(100, 200);

        assert_eq!(world.get_trade_state(100), TRADE_STATE_SENDER);
        assert_eq!(world.get_trade_state(200), TRADE_STATE_TARGET);
        assert_eq!(world.get_exchange_user(100), Some(200));
        assert_eq!(world.get_exchange_user(200), Some(100));
        assert!(world.is_trading(100));
        assert!(world.is_trading(200));
    }

    #[test]
    fn test_trade_agree() {
        let world = make_world_with_sessions(&[100, 200]);
        world.init_trade_request(100, 200);
        world.trade_agree(100);

        assert_eq!(world.get_trade_state(100), TRADE_STATE_TRADING);
        assert_eq!(world.get_trade_state(200), TRADE_STATE_TRADING);
    }

    #[test]
    fn test_trade_decline() {
        let world = make_world_with_sessions(&[100, 200]);
        world.init_trade_request(100, 200);
        world.trade_decline(100);

        assert_eq!(world.get_trade_state(100), TRADE_STATE_NONE);
        assert_eq!(world.get_trade_state(200), TRADE_STATE_NONE);
        assert!(!world.is_trading(100));
        assert!(!world.is_trading(200));
    }

    #[test]
    fn test_reset_trade() {
        let world = make_world_with_sessions(&[100]);
        world.set_trade_state(100, TRADE_STATE_TRADING);
        world.reset_trade(100);
        assert_eq!(world.get_trade_state(100), TRADE_STATE_NONE);
        assert_eq!(world.get_exchange_items(100).len(), 0);
    }

    // ── Exchange Items ──────────────────────────────────────────────

    #[test]
    fn test_add_and_get_exchange_items() {
        let world = make_world_with_sessions(&[100]);
        let item = ExchangeItem {
            item_id: 379006001,
            count: 5,
            durability: 100,
            serial_num: 0,
            src_pos: 14,
            dst_pos: 0,
        };
        world.add_exchange_item(100, item);

        let items = world.get_exchange_items(100);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, 379006001);
        assert_eq!(items[0].count, 5);
    }

    #[test]
    fn test_update_exchange_item_count() {
        let world = make_world_with_sessions(&[100]);
        let item = ExchangeItem {
            item_id: 379006001,
            count: 5,
            durability: 100,
            serial_num: 0,
            src_pos: 14,
            dst_pos: 0,
        };
        world.add_exchange_item(100, item);
        assert!(world.update_exchange_item_count(100, 379006001, 10));

        let items = world.get_exchange_items(100);
        assert_eq!(items[0].count, 15);
    }

    #[test]
    fn test_update_exchange_item_count_not_found() {
        let world = make_world_with_sessions(&[100]);
        assert!(!world.update_exchange_item_count(100, 999, 10));
    }

    // ── Merchant Methods ────────────────────────────────────────────

    #[test]
    fn test_initial_merchant_state() {
        let world = make_world_with_sessions(&[100]);
        assert!(!world.is_merchanting(100));
        assert!(!world.is_selling_merchant(100));
        assert!(!world.is_buying_merchant(100));
    }

    #[test]
    fn test_activate_selling_merchant() {
        let world = make_world_with_sessions(&[100]);
        world.set_selling_merchant_preparing(100, true);
        assert!(world.is_selling_merchant_preparing(100));

        world.activate_selling_merchant(100);
        assert!(world.is_merchanting(100));
        assert!(world.is_selling_merchant(100));
        assert!(!world.is_selling_merchant_preparing(100));
    }

    #[test]
    fn test_activate_buying_merchant() {
        let world = make_world_with_sessions(&[100]);
        world.set_buying_merchant_preparing(100, true);
        assert!(world.is_buying_merchant_preparing(100));

        world.activate_buying_merchant(100);
        assert!(world.is_merchanting(100));
        assert!(world.is_buying_merchant(100));
        assert!(!world.is_buying_merchant_preparing(100));
    }

    #[test]
    fn test_close_merchant() {
        let world = make_world_with_sessions(&[100]);
        world.activate_selling_merchant(100);
        world.close_merchant(100);
        assert!(!world.is_merchanting(100));
        assert!(!world.is_selling_merchant(100));
    }

    #[test]
    fn test_merchant_item_set_get() {
        let world = make_world_with_sessions(&[100]);
        let data = MerchData {
            item_id: 379006001,
            sell_count: 10,
            price: 50000,
            ..Default::default()
        };
        world.set_merchant_item(100, 0, data.clone());
        let got = world.get_merchant_item(100, 0).unwrap();
        assert_eq!(got.item_id, 379006001);
        assert_eq!(got.sell_count, 10);
        assert_eq!(got.price, 50000);
    }

    #[test]
    fn test_merchant_item_out_of_bounds() {
        let world = make_world_with_sessions(&[100]);
        assert!(world.get_merchant_item(100, MAX_MERCH_ITEMS).is_none());
    }

    #[test]
    fn test_try_merchant_buy_success() {
        let world = make_world_with_sessions(&[100]);
        let data = MerchData {
            item_id: 379006001,
            sell_count: 10,
            price: 50000,
            ..Default::default()
        };
        world.set_merchant_item(100, 0, data);
        let snapshot = world.try_merchant_buy(100, 0, 379006001, 3);
        assert!(snapshot.is_some());
        let snap = snapshot.unwrap();
        assert_eq!(snap.sell_count, 10); // snapshot before buy

        let after = world.get_merchant_item(100, 0).unwrap();
        assert_eq!(after.sell_count, 7); // 10 - 3
        assert!(!after.sold_out);
    }

    #[test]
    fn test_try_merchant_buy_sells_out() {
        let world = make_world_with_sessions(&[100]);
        let data = MerchData {
            item_id: 379006001,
            sell_count: 5,
            price: 50000,
            ..Default::default()
        };
        world.set_merchant_item(100, 0, data);
        world.try_merchant_buy(100, 0, 379006001, 5);

        let after = world.get_merchant_item(100, 0).unwrap();
        assert_eq!(after.sell_count, 0);
        assert!(after.sold_out);
    }

    #[test]
    fn test_try_merchant_buy_wrong_item() {
        let world = make_world_with_sessions(&[100]);
        let data = MerchData {
            item_id: 379006001,
            sell_count: 10,
            price: 50000,
            ..Default::default()
        };
        world.set_merchant_item(100, 0, data);
        assert!(world.try_merchant_buy(100, 0, 999, 1).is_none());
    }

    #[test]
    fn test_merchant_looker() {
        let world = make_world_with_sessions(&[100, 200]);
        world.set_merchant_looker(100, Some(200));
        assert_eq!(world.get_merchant_looker(100), Some(200));

        world.set_browsing_merchant(200, Some(100));
        assert_eq!(world.get_browsing_merchant(200), Some(100));

        world.remove_from_merchant_lookers(200);
        assert_eq!(world.get_merchant_looker(100), None);
        assert_eq!(world.get_browsing_merchant(200), None);
    }

    // ── Mining / Fishing ────────────────────────────────────────────

    #[test]
    fn test_mining_fishing_default_false() {
        let world = make_world_with_sessions(&[100]);
        assert!(!world.is_mining(100));
        assert!(!world.is_fishing(100));
    }

    // ── Non-existent session ────────────────────────────────────────

    #[test]
    fn test_trade_state_nonexistent_session() {
        let world = WorldState::new();
        assert_eq!(world.get_trade_state(999), TRADE_STATE_NONE);
        assert!(!world.is_trading(999));
        assert!(!world.is_merchanting(999));
    }
}
