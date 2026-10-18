//! Zone management, region tracking, and ground item bundles.

use super::*;

impl WorldState {
    /// Ensure a zone exists with the given map size.
    pub fn ensure_zone(&self, zone_id: u16, map_size: u16) {
        self.zones
            .entry(zone_id)
            .or_insert_with(|| Arc::new(ZoneState::new(zone_id, map_size)));
    }
    /// Get a zone by ID.
    pub fn get_zone(&self, zone_id: u16) -> Option<Arc<ZoneState>> {
        self.zones.get(&zone_id).map(|z| z.clone())
    }
    /// Set zone info on an existing zone (creates zone if needed).
    ///
    /// Used by tests and loading code to attach zone configuration.
    pub fn set_zone_info(&self, zone_id: u16, info: crate::zone::ZoneInfo) {
        use std::collections::HashMap;
        let zone = crate::zone::ZoneState::new_with_data(
            zone_id,
            info,
            None,
            HashMap::new(),
            HashMap::new(),
        );
        self.zones.insert(zone_id, Arc::new(zone));
    }
    /// Set zone info with map data on an existing zone (creates zone if needed).
    ///
    /// Used by tests to attach zone configuration with actual map data
    /// for path validation testing.
    pub fn set_zone_with_map(
        &self,
        zone_id: u16,
        info: crate::zone::ZoneInfo,
        map_data: crate::zone::MapData,
    ) {
        use std::collections::HashMap;
        let zone = crate::zone::ZoneState::new_with_data(
            zone_id,
            info,
            Some(map_data),
            HashMap::new(),
            HashMap::new(),
        );
        self.zones.insert(zone_id, Arc::new(zone));
    }

    /// Get all zone IDs currently present in the world.
    pub fn get_all_zone_ids(&self) -> Vec<u16> {
        self.zones.iter().map(|entry| *entry.key()).collect()
    }
    /// Check if a zone has any players in it (for AI optimization).
    /// Uses zone_session_index for O(1) lookup instead of scanning all sessions.
    pub fn zone_has_players(&self, zone_id: u16) -> bool {
        if let Some(entry) = self.zone_session_index.get(&zone_id) {
            !entry.read().is_empty()
        } else {
            false
        }
    }

    /// Count the number of players currently in a specific zone.
    /// Uses zone_session_index for O(1) lookup.
    pub fn zone_player_count(&self, zone_id: u16) -> u16 {
        if let Some(entry) = self.zone_session_index.get(&zone_id) {
            entry.read().len() as u16
        } else {
            0
        }
    }
    // ── Ground Bundle Methods ─────────────────────────────────────────

    /// Allocate a unique bundle ID.
    pub fn allocate_bundle_id(&self) -> u32 {
        self.next_bundle_id.fetch_add(1, Ordering::Relaxed)
    }
    /// Add a ground bundle to the world.
    pub fn add_ground_bundle(&self, bundle: GroundBundle) {
        self.ground_bundles.insert(bundle.bundle_id, bundle);
    }
    /// Get a ground bundle by ID.
    pub fn get_ground_bundle(&self, bundle_id: u32) -> Option<GroundBundle> {
        self.ground_bundles.get(&bundle_id).map(|b| b.clone())
    }
    /// Atomically take an item from a bundle, returning it if present.
    /// Only ONE caller will succeed — prevents item duplication race conditions.
    /// Returns `Some((item_id, count))` on success, `None` if already taken.
    pub fn try_take_bundle_item(&self, bundle_id: u32, slot_id: u16) -> Option<(u32, u16)> {
        let mut looter = 0u16;
        let (result, should_remove) =
            if let Some(mut bundle) = self.ground_bundles.get_mut(&bundle_id) {
                looter = bundle.looter;
                if (slot_id as usize) < NPC_HAVE_ITEM_LIST {
                    let slot = &mut bundle.items[slot_id as usize];
                    if slot.item_id == 0 || slot.count == 0 {
                        (None, false)
                    } else {
                        let item_id = slot.item_id;
                        let count = slot.count;
                        *slot = LootItem::default();
                        bundle.items_count = bundle.items_count.saturating_sub(1);
                        (Some((item_id, count)), bundle.items_count == 0)
                    }
                } else {
                    (None, false)
                }
            } else {
                (None, false)
            };

        if should_remove {
            self.ground_bundles.remove(&bundle_id);
        }
        if result.is_some() && looter != 0 {
            crate::handler::drop_box::notify_taken(self, bundle_id, slot_id, looter);
        }
        result
    }

    /// Restore an item back into a bundle slot (undo a `try_take_bundle_item`).
    /// Used when inventory add fails after atomic take — puts the item back so
    /// another player can pick it up.
    pub fn restore_bundle_item(&self, bundle_id: u32, slot_id: u16, item_id: u32, count: u16) {
        if let Some(mut bundle) = self.ground_bundles.get_mut(&bundle_id) {
            if (slot_id as usize) < NPC_HAVE_ITEM_LIST {
                let slot = &mut bundle.items[slot_id as usize];
                slot.item_id = item_id;
                slot.count = count;
                slot.slot_id = slot_id;
                bundle.items_count = bundle.items_count.saturating_add(1);
            }
        } else {
            // Bundle was removed (expired or all items taken) — re-create it
            // This is a rare edge case but prevents item loss.
            let mut items: [LootItem; NPC_HAVE_ITEM_LIST] = Default::default();
            if (slot_id as usize) < NPC_HAVE_ITEM_LIST {
                items[slot_id as usize] = LootItem {
                    item_id,
                    count,
                    slot_id,
                };
            }
            self.ground_bundles.insert(
                bundle_id,
                GroundBundle {
                    bundle_id,
                    items_count: 1,
                    items,
                    ..Default::default()
                },
            );
        }
    }

    /// Remove a specific item slot from a bundle. If all items are gone, remove the bundle.
    pub fn remove_bundle_item(&self, bundle_id: u32, slot_id: u16) {
        let should_remove = if let Some(mut bundle) = self.ground_bundles.get_mut(&bundle_id) {
            if (slot_id as usize) < NPC_HAVE_ITEM_LIST {
                bundle.items[slot_id as usize] = LootItem::default();
                bundle.items_count = bundle.items_count.saturating_sub(1);
            }
            bundle.items_count == 0
        } else {
            false
        };

        if should_remove {
            self.ground_bundles.remove(&bundle_id);
        }
    }
    /// Remove a ground bundle entirely.
    pub fn remove_ground_bundle(&self, bundle_id: u32) {
        self.ground_bundles.remove(&bundle_id);
    }
    /// Collect expired bundles (older than 60 seconds).
    pub fn collect_expired_bundles(&self) -> Vec<GroundBundle> {
        let mut expired = Vec::new();
        let mut to_remove = Vec::new();

        for entry in self.ground_bundles.iter() {
            if entry.value().drop_time.elapsed().as_secs() >= 60 {
                expired.push(entry.value().clone());
                to_remove.push(*entry.key());
            }
        }

        for id in to_remove {
            self.ground_bundles.remove(&id);
        }

        expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bundle(id: u32, item_id: u32, count: u16) -> GroundBundle {
        let mut bundle = GroundBundle {
            bundle_id: id,
            items_count: 1,
            x: 100.0,
            z: 100.0,
            ..Default::default()
        };
        bundle.items[0] = LootItem {
            item_id,
            count,
            slot_id: 0,
        };
        bundle
    }

    /// Test try_take_bundle_item: first caller succeeds, second gets None.
    #[test]
    fn test_try_take_bundle_item_atomic() {
        let world = WorldState::new();
        world.add_ground_bundle(make_bundle(1, 100_001, 5));

        // First take succeeds
        let result = world.try_take_bundle_item(1, 0);
        assert_eq!(result, Some((100_001, 5)));

        // Second take fails — already taken
        let result2 = world.try_take_bundle_item(1, 0);
        assert_eq!(result2, None);
    }

    /// Test try_take_bundle_item removes bundle when last item taken.
    #[test]
    fn test_try_take_removes_empty_bundle() {
        let world = WorldState::new();
        world.add_ground_bundle(make_bundle(1, 100_001, 5));

        let _ = world.try_take_bundle_item(1, 0);
        // Bundle should be removed since items_count was 1
        assert!(world.get_ground_bundle(1).is_none());
    }

    /// Test restore_bundle_item puts item back after failed inventory add.
    #[test]
    fn test_restore_bundle_item_after_take() {
        let world = WorldState::new();
        let mut bundle = make_bundle(1, 100_001, 5);
        bundle.items[1] = LootItem {
            item_id: 100_002,
            count: 3,
            slot_id: 1,
        };
        bundle.items_count = 2;
        world.add_ground_bundle(bundle);

        // Take slot 0
        let taken = world.try_take_bundle_item(1, 0);
        assert_eq!(taken, Some((100_001, 5)));

        // Bundle still exists (slot 1 has items)
        assert!(world.get_ground_bundle(1).is_some());

        // Restore slot 0
        world.restore_bundle_item(1, 0, 100_001, 5);
        let restored = world.get_ground_bundle(1).unwrap();
        assert_eq!(restored.items[0].item_id, 100_001);
        assert_eq!(restored.items[0].count, 5);
        assert_eq!(restored.items_count, 2);
    }

    /// Test restore_bundle_item re-creates bundle if it was already removed.
    #[test]
    fn test_restore_bundle_item_recreates_removed_bundle() {
        let world = WorldState::new();
        world.add_ground_bundle(make_bundle(1, 100_001, 5));

        // Take last item — bundle gets removed
        let _ = world.try_take_bundle_item(1, 0);
        assert!(world.get_ground_bundle(1).is_none());

        // Restore — should re-create bundle
        world.restore_bundle_item(1, 0, 100_001, 5);
        let restored = world.get_ground_bundle(1).unwrap();
        assert_eq!(restored.items[0].item_id, 100_001);
        assert_eq!(restored.items[0].count, 5);
        assert_eq!(restored.items_count, 1);
    }

    /// Test try_take with invalid slot returns None.
    #[test]
    fn test_try_take_invalid_slot() {
        let world = WorldState::new();
        world.add_ground_bundle(make_bundle(1, 100_001, 5));

        // Slot out of range
        let result = world.try_take_bundle_item(1, NPC_HAVE_ITEM_LIST as u16);
        assert_eq!(result, None);

        // Non-existent bundle
        let result2 = world.try_take_bundle_item(999, 0);
        assert_eq!(result2, None);
    }

    // ── Sprint 934: Additional coverage ──────────────────────────────

    /// ensure_zone creates a zone if it doesn't exist.
    #[test]
    fn test_ensure_zone_creates() {
        let world = WorldState::new();
        assert!(world.get_zone(9999).is_none());
        world.ensure_zone(9999, 4096);
        assert!(world.get_zone(9999).is_some());
    }

    /// get_all_zone_ids includes newly ensured zones.
    #[test]
    fn test_get_all_zone_ids() {
        let world = WorldState::new();
        world.ensure_zone(9901, 4096);
        world.ensure_zone(9902, 4096);
        let ids = world.get_all_zone_ids();
        assert!(ids.contains(&9901));
        assert!(ids.contains(&9902));
    }

    /// zone_has_players returns false for empty zones.
    #[test]
    fn test_zone_has_players_empty() {
        let world = WorldState::new();
        world.ensure_zone(9999, 4096);
        assert!(!world.zone_has_players(9999));
    }

    /// allocate_bundle_id returns unique IDs.
    #[test]
    fn test_allocate_bundle_id_unique() {
        let world = WorldState::new();
        let id1 = world.allocate_bundle_id();
        let id2 = world.allocate_bundle_id();
        let id3 = world.allocate_bundle_id();
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
    }

    /// remove_ground_bundle removes the bundle entirely.
    #[test]
    fn test_remove_ground_bundle() {
        let world = WorldState::new();
        world.add_ground_bundle(make_bundle(42, 100_001, 1));
        assert!(world.get_ground_bundle(42).is_some());
        world.remove_ground_bundle(42);
        assert!(world.get_ground_bundle(42).is_none());
    }
}
