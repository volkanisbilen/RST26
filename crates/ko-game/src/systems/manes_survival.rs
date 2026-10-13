//! Manes Survival runtime monster lifecycle.
//!
//! Client contract: Zones.tbl 570/580/590/600 map to server zones 57/58/59/60;
//! WIZ_SURVIVAL uses shared opcode byte 0xD0. Event monsters are runtime
//! NPCs and never belong in the global npc_spawn table.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::{DashMap, DashSet};
use ko_db::models::{ManesSurvivalMagicRow, ManesSurvivalSpawnRow};
use ko_protocol::{Opcode, Packet};
use parking_lot::RwLock;
use rand::seq::SliceRandom;

use crate::world::UserItemSlot;
use crate::world::WorldState;
use crate::zone::SessionId;

pub const ZONES_MANES_SURVIVAL: [u16; 4] = [57, 58, 59, 60];
/// Zones 57-60 are dedicated physical instances, so their NPCs share room 0
/// with players entering through the normal zone-change flow.
pub const MANES_EVENT_ROOM: u16 = 0;
/// Event-only consumables. These two potion IDs must never survive Manes.
pub const MANES_TEMP_HP_POTION_ITEM_ID: u32 = 978_023_000;
pub const MANES_TEMP_MP_POTION_ITEM_ID: u32 = 978_024_000;
pub const MANES_ORB_ITEM_ID: u32 = 978_026_000;
pub const DARK_DRAGON_SID: i16 = 10733;
pub const DARK_DRAGON_UI_NAME: &str = "[boss]red dragon";
pub const MIN_ACTIVE_PARTICIPANTS: usize = 1;
pub const MANES_EXIT_DELAY_SECONDS: u64 = 10;
pub const MANES_NORMAL_RESPAWN_MS: u64 = 10_000;
pub const MANES_BOSS_RESPAWN_MS: u64 = 120_000;
const MANES_FINALIZE_DELAY_MS: u64 = 750;
const LOW_MIDDLE_SPAWN_MULTIPLIER: usize = 3;

const KARUS_ENTRY_START: (f32, f32) = (345.0, 207.0);
const KARUS_ENTRY_END: (f32, f32) = (260.0, 581.0);
const ELMORAD_ENTRY_START: (f32, f32) = (722.0, 644.0);
const ELMORAD_ENTRY_END: (f32, f32) = (728.0, 219.0);

pub const MANES_MAX_LEVEL: u8 = 30;

/// Medium-paced 20-minute progression. The 29 entries are the EXP required
/// to advance from levels 1..=29. Total EXP to level 30 is 11,020.
const MANES_LEVEL_EXP: [u16; 29] = [
    100, 120, 140, 160, 180, 200, 220, 240, 260, 280, 300, 320, 340, 360, 380, 400, 420, 440, 460,
    480, 500, 520, 540, 560, 580, 600, 620, 640, 660,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManesProgress {
    pub level: u8,
    pub exp: u16,
    pub max_exp: u16,
    pub leveled_up: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManesOffer {
    pub skills: Vec<u16>,
    pub potions: Vec<u16>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ManesCombatBonuses {
    pub hp: i16,
    pub attack_pct: i16,
    pub reduction_pct: i16,
}

pub fn required_exp_for_level(level: u8) -> u16 {
    if level >= MANES_MAX_LEVEL {
        0
    } else {
        MANES_LEVEL_EXP[(level.saturating_sub(1)) as usize]
    }
}

/// Reference-client Manes vitals.
///
/// Confirmed from the supplied gameplay capture:
/// level 10 => 10060 HP / 2000 MP, level 13 => 13060 HP / 2600 MP.
pub fn vitals_for_level(level: u8, hp_bonus: i16) -> (i16, i16) {
    let level = level.clamp(1, MANES_MAX_LEVEL);
    let max_hp = (i32::from(level) * 1_000 + 60 + i32::from(hp_bonus.max(0)))
        .clamp(1, i16::MAX as i32) as i16;
    let max_mp = (i32::from(level) * 200).clamp(1, i16::MAX as i32) as i16;
    (max_hp, max_mp)
}

pub fn score_for_level(level: u8) -> u16 {
    u16::from(level.saturating_sub(1)) * 20
}

/// Level-driven attack scale shared by the authoritative damage path and the
/// stat packet shown by the client. Level 1 starts at 15% and each event level
/// adds 2.5 percentage points, reaching 87.5% at level 30.
pub fn attack_scale_per_mille(level: u8) -> u16 {
    150 + u16::from(level.clamp(1, MANES_MAX_LEVEL).saturating_sub(1)) * 25
}

pub fn orb_reward_for_rank(rank: usize) -> u16 {
    match rank {
        1 => 150,
        2 => 100,
        3 => 50,
        _ => 20,
    }
}

/// EXP rewards are intentionally independent from the persistent NPC EXP.
/// They follow the supplied Manes grade ranges and let an active player reach
/// level 30 after clearing most of one physical zone, without making the early
/// levels grindy.
pub fn monster_survival_exp(npc_sid: u16) -> u16 {
    match npc_sid {
        10701..=10705 => 80,
        10706..=10709 => 140,
        10710..=10715 => 160,
        10716..=10719 => 260,
        10720..=10728 => 280,
        10729..=10732 => 450,
        10733 => 1_000,
        _ => 0,
    }
}

pub struct ManesSurvivalManager {
    spawns: RwLock<Vec<ManesSurvivalSpawnRow>>,
    magic: RwLock<Vec<ManesSurvivalMagicRow>>,
    registration_open: AtomicBool,
    active: AtomicBool,
    participants: DashSet<SessionId>,
    progress: DashMap<SessionId, ManesProgress>,
    unlocked_magic: DashMap<SessionId, DashSet<u32>>,
    selected_rows: DashMap<SessionId, DashMap<u16, u16>>,
    offers: DashMap<SessionId, ManesOffer>,
    /// Real max HP/MP of every participant, put back exactly when they leave.
    saved_vitals: DashMap<SessionId, (i16, i16)>,
    /// Orb rewards that did not fit the bag at the end; handed over again on exit.
    pending_rewards: DashMap<SessionId, u16>,
    registration_deadline: RwLock<Option<Instant>>,
    started_at: RwLock<Option<Instant>>,
    final_boss_killed_at: RwLock<Option<Instant>>,
}

impl Default for ManesSurvivalManager {
    fn default() -> Self {
        Self {
            spawns: RwLock::new(Vec::new()),
            magic: RwLock::new(Vec::new()),
            registration_open: AtomicBool::new(false),
            active: AtomicBool::new(false),
            participants: DashSet::new(),
            progress: DashMap::new(),
            unlocked_magic: DashMap::new(),
            selected_rows: DashMap::new(),
            offers: DashMap::new(),
            saved_vitals: DashMap::new(),
            pending_rewards: DashMap::new(),
            registration_deadline: RwLock::new(None),
            started_at: RwLock::new(None),
            final_boss_killed_at: RwLock::new(None),
        }
    }
}

impl ManesSurvivalManager {
    pub fn set_spawns(&self, rows: Vec<ManesSurvivalSpawnRow>) -> anyhow::Result<()> {
        if rows.len() != 33 {
            anyhow::bail!(
                "Manes Survival requires exactly 33 configured monster types; loaded {}",
                rows.len()
            );
        }
        if rows
            .iter()
            .filter(|r| r.npc_id == DARK_DRAGON_SID && r.boss_tier == 3)
            .count()
            != 1
        {
            anyhow::bail!("Manes Survival requires exactly one Dark Dragon configuration");
        }
        if rows
            .iter()
            .any(|r| r.spawn_count <= 0 || r.spawn_x < 0 || r.spawn_z < 0 || r.spawn_range < 0)
        {
            anyhow::bail!("Manes Survival contains invalid spawn count or coordinates");
        }
        *self.spawns.write() = rows;
        Ok(())
    }

    pub fn set_magic(&self, rows: Vec<ManesSurvivalMagicRow>) -> anyhow::Result<()> {
        if rows.len() != 154 {
            anyhow::bail!(
                "Manes Survival requires all 154 MANES_MAGIC rows; loaded {}",
                rows.len()
            );
        }
        if rows.iter().any(|row| {
            row.selection_id <= 0
                || row.selection_id > u16::MAX as i64
                || row.magic_id <= 0
                || row.magic_id > u32::MAX as i64
        }) {
            anyhow::bail!("Manes Survival contains an invalid MANES_MAGIC identifier");
        }
        *self.magic.write() = rows;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub fn is_registration_open(&self) -> bool {
        self.registration_open.load(Ordering::Acquire)
    }

    pub fn open_registration(&self) -> bool {
        if self.is_active() {
            return false;
        }
        self.participants.clear();
        *self.registration_deadline.write() = Some(
            Instant::now()
                + Duration::from_secs(u64::from(
                    crate::handler::survival::REGISTRATION_DURATION_SECONDS,
                )),
        );
        !self.registration_open.swap(true, Ordering::AcqRel)
    }

    pub fn register(&self, session_id: SessionId) -> bool {
        self.is_registration_open() && self.participants.insert(session_id)
    }

    pub fn unregister(&self, session_id: SessionId) -> bool {
        self.is_registration_open() && self.participants.remove(&session_id).is_some()
    }

    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    pub fn participant_ids(&self) -> Vec<SessionId> {
        self.participants.iter().map(|entry| *entry.key()).collect()
    }

    pub fn active_participant_ids(&self, world: &WorldState) -> Vec<SessionId> {
        let mut ids: Vec<_> = self
            .participant_ids()
            .into_iter()
            .filter(|sid| world.get_character_info(*sid).is_some())
            .collect();
        ids.sort_unstable();
        ids
    }

    pub fn configured_count(&self) -> usize {
        self.spawns.read().len()
    }

    pub fn progress(&self, session_id: SessionId) -> Option<ManesProgress> {
        self.progress.get(&session_id).map(|entry| *entry.value())
    }

    pub fn remaining_seconds(&self) -> u32 {
        self.started_at
            .read()
            .as_ref()
            .map(|started| {
                u64::from(crate::handler::survival::EVENT_DURATION_SECONDS)
                    .saturating_sub(started.elapsed().as_secs())
                    .min(u64::from(u32::MAX)) as u32
            })
            .unwrap_or(0)
    }

    pub fn broadcast_dark_dragon_status(
        &self,
        world: &WorldState,
        zone_id: u16,
        boss_name: &str,
        max_hp: u32,
        current_hp: u32,
    ) {
        if !self.is_active() {
            return;
        }
        let packet = crate::handler::survival::build_event_boss_status(
            boss_name,
            max_hp,
            current_hp,
            self.remaining_seconds(),
        );
        for sid in self.participant_ids() {
            if world
                .get_position(sid)
                .map(|position| position.zone_id == zone_id)
                .unwrap_or(false)
            {
                world.send_to_session_owned(sid, packet.clone());
            }
        }
    }

    pub fn request_final_boss_finish(&self, npc_sid: u16) -> bool {
        if !self.is_active() || npc_sid != DARK_DRAGON_SID as u16 {
            return false;
        }
        let mut requested_at = self.final_boss_killed_at.write();
        if requested_at.is_some() {
            return false;
        }
        *requested_at = Some(Instant::now());
        true
    }

    /// Consume a Dark Dragon death request after its final combat packets have
    /// reached the client, then use the normal reward/countdown/exit path.
    pub fn finalize_requested_event(&self, world: Arc<WorldState>) {
        let should_finalize = self
            .final_boss_killed_at
            .read()
            .as_ref()
            .map(|killed_at| killed_at.elapsed() >= Duration::from_millis(MANES_FINALIZE_DELAY_MS))
            .unwrap_or(false);
        if !should_finalize {
            return;
        }
        self.final_boss_killed_at.write().take();
        if !self.is_active() {
            return;
        }

        let (rewarded, failed) = self.finish_event(&world);
        tracing::info!(
            rewarded,
            failed,
            "Manes Survival ended automatically after Dark Dragon death"
        );
    }

    /// One-based event rank, ordered by the score derived from survival level.
    pub fn rank_for_session(&self, session_id: SessionId) -> u16 {
        let Some(progress) = self.progress(session_id) else {
            return 0;
        };
        let score = score_for_level(progress.level);
        let higher_scores = self
            .progress
            .iter()
            .filter(|entry| score_for_level(entry.value().level) > score)
            .count();
        u16::try_from(higher_scores.saturating_add(1)).unwrap_or(u16::MAX)
    }

    fn row(&self, selection_id: u16) -> Option<ManesSurvivalMagicRow> {
        self.magic
            .read()
            .iter()
            .find(|row| row.selection_id == i64::from(selection_id))
            .cloned()
    }

    pub fn unlock_magic(&self, session_id: SessionId, row_id: u16) -> Option<u32> {
        let row = self.row(row_id)?;
        if !(1..=5).contains(&row.kind) {
            return None;
        }
        let magic_id = row.magic_id as u32;
        let branch = row_id / 100;
        let selected = self.selected_rows.entry(session_id).or_default();
        if let Some(previous) = selected.insert(branch, row_id) {
            if let Some(previous_row) = self.row(previous) {
                if let Some(skills) = self.unlocked_magic.get(&session_id) {
                    skills.remove(&(previous_row.magic_id as u32));
                }
            }
        }
        self.unlocked_magic
            .entry(session_id)
            .or_default()
            .insert(magic_id);
        Some(magic_id)
    }

    pub fn create_offer(&self, session_id: SessionId) -> ManesOffer {
        let selected = self.selected_rows.get(&session_id);
        let rows = self.magic.read();
        let mut skills: Vec<u16> = rows
            .iter()
            .filter_map(|row| {
                if !(1..=5).contains(&row.kind) {
                    return None;
                }
                let id = u16::try_from(row.selection_id).ok()?;
                let branch = id / 100;
                let tier = id % 100;
                let owned = selected
                    .as_ref()
                    .and_then(|branches| branches.get(&branch).map(|value| *value));
                match owned {
                    Some(current) if tier == current % 100 + 1 => Some(id),
                    None if tier == 1 => Some(id),
                    _ => None,
                }
            })
            .collect();
        let mut potions: Vec<u16> = rows
            .iter()
            .filter(|row| row.kind == 100)
            .filter_map(|row| u16::try_from(row.selection_id).ok())
            .collect();
        let mut rng = rand::thread_rng();
        skills.shuffle(&mut rng);
        potions.shuffle(&mut rng);
        skills.truncate(3);
        potions.truncate(2);
        let offer = ManesOffer { skills, potions };
        self.offers.insert(session_id, offer.clone());
        offer
    }

    pub fn offered_skill(&self, session_id: SessionId, selection_id: u16) -> bool {
        self.offers
            .get(&session_id)
            .map(|offer| offer.skills.contains(&selection_id))
            .unwrap_or(false)
    }

    pub fn offered_potion(&self, session_id: SessionId, selection_id: u16) -> bool {
        self.offers
            .get(&session_id)
            .map(|offer| offer.potions.contains(&selection_id))
            .unwrap_or(false)
    }

    pub fn potion_purchase(&self, selection_id: u16) -> Option<(u32, u16, u32)> {
        let row = self.row(selection_id)?;
        (row.kind == 100).then_some((row.item_id as u32, row.item_count as u16, row.price as u32))
    }

    /// Remove only the event HP/MP potions from an inventory copy before it is
    /// exposed as a normal character inventory or persisted to `user_items`.
    ///
    /// Manes Orb is a permanent exchange reward and must survive event exit,
    /// disconnect and the next login. Do not derive this filter from every
    /// MANES_MAGIC potion row: that previously made permanent rewards eligible
    /// for deletion.
    pub fn remove_temporary_items(&self, inventory: &mut [UserItemSlot]) -> usize {
        let mut removed = 0;
        for slot in inventory {
            if matches!(
                slot.item_id,
                MANES_TEMP_HP_POTION_ITEM_ID | MANES_TEMP_MP_POTION_ITEM_ID
            ) {
                *slot = UserItemSlot::default();
                removed += 1;
            }
        }
        removed
    }

    /// Remove temporary Manes consumables from the live session inventory and
    /// immediately refresh the client. Persistent save paths independently use
    /// `remove_temporary_items`, so disconnects are also safe.
    pub fn cleanup_temporary_items_for_session(
        &self,
        world: &WorldState,
        session_id: SessionId,
    ) -> usize {
        // Normal Manes consumables live in bag slots. Use the regular removal
        // path so every cleared slot gets its own WIZ_ITEM_COUNT_CHANGE packet;
        // send_item_move_refresh() only refreshes derived equipment stats and
        // leaves deleted bag items visible as client-side ghosts.
        let mut removed =
            usize::from(world.rob_all_of_item(session_id, MANES_TEMP_HP_POTION_ITEM_ID))
                + usize::from(world.rob_all_of_item(session_id, MANES_TEMP_MP_POTION_ITEM_ID));

        // Defensive sanitisation for a malformed/legacy slot outside the bag.
        // Persistence and login use the same exact two-ID filter.
        let mut removed_outside_bag = 0;
        world.update_inventory(session_id, |inventory| {
            removed_outside_bag = self.remove_temporary_items(inventory);
            removed_outside_bag > 0
        });
        if removed_outside_bag > 0 {
            world.send_item_move_refresh(session_id);
        }
        removed += removed_outside_bag;
        removed
    }

    pub fn combat_bonuses(&self, session_id: SessionId) -> ManesCombatBonuses {
        let Some(selected) = self.selected_rows.get(&session_id) else {
            return ManesCombatBonuses::default();
        };
        selected
            .iter()
            .filter_map(|entry| self.row(*entry.value()))
            .fold(ManesCombatBonuses::default(), |mut bonuses, row| {
                bonuses.hp = bonuses.hp.max(row.hp_bonus);
                bonuses.attack_pct = bonuses.attack_pct.max(row.attack_bonus_pct);
                bonuses.reduction_pct = bonuses.reduction_pct.max(row.reduction_pct);
                bonuses
            })
    }

    pub fn reduce_incoming_damage(&self, session_id: SessionId, damage: i16) -> i16 {
        let reduction = self.combat_bonuses(session_id).reduction_pct.clamp(0, 100) as i32;
        (damage as i32 * (100 - reduction) / 100).clamp(0, i16::MAX as i32) as i16
    }

    pub fn has_unlocked_magic(&self, session_id: SessionId, magic_id: u32) -> bool {
        self.unlocked_magic
            .get(&session_id)
            .map(|skills| skills.contains(&magic_id))
            .unwrap_or(false)
    }

    /// While a participant plays, every recalculation (items, buffs, respawn, death)
    /// must keep the event vitals instead of falling back to the real character's.
    pub fn vitals_override(&self, session_id: SessionId) -> Option<(i16, i16)> {
        if !self.is_active() {
            return None;
        }
        let progress = self.progress(session_id)?;
        let hp_bonus = self.combat_bonuses(session_id).hp.clamp(0, i16::MAX);
        Some(vitals_for_level(progress.level, hp_bonus))
    }

    /// True while the player is a live participant (progress exists and the event runs).
    pub fn is_participant_playing(&self, session_id: SessionId) -> bool {
        self.is_active() && self.progress.contains_key(&session_id)
    }

    /// Drop every per-player event record (ranking, skills, offers).
    fn forget(&self, session_id: SessionId) {
        self.participants.remove(&session_id);
        self.progress.remove(&session_id);
        self.unlocked_magic.remove(&session_id);
        self.selected_rows.remove(&session_id);
        self.offers.remove(&session_id);
    }

    /// Entering the event: remember the real max HP/MP and clear event leftovers.
    fn begin_for(&self, world: &WorldState, session_id: SessionId) {
        self.cleanup_temporary_items_for_session(world, session_id);
        if let Some(ch) = world.get_character_info(session_id) {
            self.saved_vitals
                .entry(session_id)
                .or_insert((ch.max_hp, ch.max_mp));
        }
    }

    /// Leaving the event by any road (finish, GM close, warp, disconnect): the real
    /// max HP/MP come back exactly, event-only potions are taken, the player is no
    /// longer a participant. With `notify` the client gets the new bars as well.
    /// Returns false when the player was not inside the event.
    pub fn leave_event(&self, world: &WorldState, session_id: SessionId, notify: bool) -> bool {
        let saved = self.saved_vitals.remove(&session_id);
        // Registered-only players and everybody who leaves lose their event records.
        self.forget(session_id);
        let Some((_, (max_hp, max_mp))) = saved else {
            return false;
        };
        self.cleanup_temporary_items_for_session(world, session_id);
        world.update_character_stats(session_id, |ch| {
            ch.max_hp = max_hp;
            ch.max_mp = max_mp;
            ch.hp = max_hp;
            ch.mp = max_mp;
        });
        if notify {
            world.send_to_session_owned(
                session_id,
                crate::handler::max_hp_change::build_max_hp_change(i32::from(max_hp)),
            );
            world.send_to_session_owned(
                session_id,
                crate::systems::regen::build_hp_change_packet(max_hp, max_hp),
            );
            world.send_to_session_owned(
                session_id,
                crate::systems::regen::build_mp_change_packet(max_mp, max_mp),
            );
            world.send_item_move_refresh(session_id);
        }
        true
    }

    /// A dead participant comes back on their nation's entry line inside the same zone.
    pub fn respawn_point(
        &self,
        world: &WorldState,
        session_id: SessionId,
        zone_id: u16,
    ) -> Option<(f32, f32)> {
        use rand::Rng;
        if !ZONES_MANES_SURVIVAL.contains(&zone_id) || !self.is_participant_playing(session_id) {
            return None;
        }
        let nation = world.get_character_info(session_id)?.nation;
        let (start, end) = if nation == crate::world::NATION_KARUS {
            (KARUS_ENTRY_START, KARUS_ENTRY_END)
        } else {
            (ELMORAD_ENTRY_START, ELMORAD_ENTRY_END)
        };
        let zone = world.get_zone(zone_id)?;
        let mut rng = rand::thread_rng();
        for _ in 0..8 {
            let f: f32 = rng.gen_range(0.05..0.95);
            let x = start.0 + (end.0 - start.0) * f;
            let z = start.1 + (end.1 - start.1) * f;
            if zone.is_valid_position(x, z) {
                return Some((x, z));
            }
        }
        None
    }

    /// Every second: the registration countdown starts the event, the 20 minutes end it.
    pub fn tick(&self, world: &Arc<WorldState>) {
        if self.is_registration_open() && !self.is_active() {
            let due = self
                .registration_deadline
                .read()
                .as_ref()
                .map(|d| Instant::now() >= *d)
                .unwrap_or(false);
            if due {
                self.registration_deadline.write().take();
                match self.start_registered_event(world) {
                    Ok((count, players)) => {
                        tracing::info!(
                            count,
                            players,
                            "Manes Survival started after the registration countdown"
                        );
                    }
                    Err(error) => {
                        tracing::warn!("Manes Survival could not start: {error:#}");
                        let notice = Arc::new(crate::systems::timed_notice::build_notice_packet(
                            8,
                            "Manes Survival iptal edildi: yeterli katilimci yok.",
                        ));
                        world.broadcast_to_all(notice, None);
                        self.stop(world);
                    }
                }
            }
        }
        if self.is_active() && self.started_at.read().is_some() && self.remaining_seconds() == 0 {
            tracing::info!("Manes Survival time is up");
            self.finish_event(world);
        }
    }

    /// The client empties its bag when the Survival mode starts. Once everybody has loaded
    /// the zone, the real bag is sent again (twice, in case the first one raced the load).
    fn schedule_bag_refresh(world: Arc<WorldState>) {
        tokio::spawn(async move {
            for wait_ms in [2_500u64, 4_000] {
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                let playing: Vec<SessionId> = world
                    .manes_survival_manager
                    .participant_ids()
                    .into_iter()
                    .filter(|sid| world.manes_survival_manager.is_participant_playing(*sid))
                    .collect();
                for sid in playing {
                    if let Some(pkt) = crate::handler::survival::build_bag_refresh(&world, sid).await {
                        world.send_to_session_owned(sid, pkt);
                    }
                }
            }
        });
    }

    /// Rewards, stop and the delayed evacuation of every participant.
    pub fn finish_event(&self, world: &Arc<WorldState>) -> (usize, usize) {
        let participants = self.participant_ids();
        let result = self.reward_rankings_and_stop(world);
        Self::schedule_participant_exit(world.clone(), participants);
        result
    }

    pub fn reset_progress(&self, session_id: SessionId) -> ManesProgress {
        let progress = ManesProgress {
            level: 1,
            exp: 0,
            max_exp: required_exp_for_level(1),
            leveled_up: false,
        };
        self.progress.insert(session_id, progress);
        self.unlocked_magic.remove(&session_id);
        self.selected_rows.remove(&session_id);
        self.offers.remove(&session_id);
        progress
    }

    pub fn award_monster_exp(&self, session_id: SessionId, npc_sid: u16) -> Option<ManesProgress> {
        if !self.is_active() {
            return None;
        }
        let reward = monster_survival_exp(npc_sid);
        if reward == 0 {
            return None;
        }

        let mut entry = self.progress.entry(session_id).or_insert(ManesProgress {
            level: 1,
            exp: 0,
            max_exp: required_exp_for_level(1),
            leveled_up: false,
        });
        let mut state = *entry;
        state.leveled_up = false;

        if state.level < MANES_MAX_LEVEL {
            state.exp = state.exp.saturating_add(reward);
            while state.level < MANES_MAX_LEVEL && state.exp >= state.max_exp {
                state.exp -= state.max_exp;
                state.level += 1;
                state.leveled_up = true;
                state.max_exp = required_exp_for_level(state.level);
            }
            if state.level >= MANES_MAX_LEVEL {
                state.level = MANES_MAX_LEVEL;
                state.exp = 0;
                state.max_exp = 0;
            }
        }

        *entry = state;
        Some(state)
    }

    /// Start the event and place every online registrant on the nation-specific
    /// outer entry line. The client-confirmed registration countdown sends this
    /// transition once it expires; the GM command uses this same path.
    pub fn start_registered_event(
        &self,
        world: &Arc<WorldState>,
    ) -> anyhow::Result<(usize, usize)> {
        let participants = self.active_participant_ids(world);
        if participants.len() < MIN_ACTIVE_PARTICIPANTS {
            anyhow::bail!(
                "Manes Survival requires at least {MIN_ACTIVE_PARTICIPANTS} active participant"
            );
        }

        let monster_count = self.start(world)?;
        if monster_count == 0 {
            return Ok((0, participants.len()));
        }

        let placed = self.place_participants(world, &participants);
        if placed > 0 {
            Self::schedule_bag_refresh(world.clone());
        }
        if placed == 0 {
            self.stop(world);
            anyhow::bail!("no registered participant could be placed in the event zones");
        }

        Ok((monster_count, placed))
    }

    /// Put every registrant on their nation's entry line. Players that went offline,
    /// died or cannot be placed are dropped from the event instead of aborting it.
    /// Returns how many players are inside.
    fn place_participants(&self, world: &WorldState, participants: &[SessionId]) -> usize {
        let mut karus_index = 0usize;
        let mut elmorad_index = 0usize;
        let mut placed = 0usize;
        let skip_notice = |sid: SessionId, text: &str| {
            world.send_to_session_owned(
                sid,
                crate::systems::timed_notice::build_notice_packet(8, text),
            );
        };

        for (zone_index, sid) in participants.iter().copied().enumerate() {
            let Some(character) = world.get_character_info(sid) else {
                self.forget(sid);
                continue;
            };
            if world.is_player_dead(sid) {
                skip_notice(sid, "Manes Survival: olu oldugun icin etkinlige alinmadin.");
                self.forget(sid);
                continue;
            }
            let nation = character.nation;
            let (start, end, nation_index) = match nation {
                crate::world::NATION_KARUS => {
                    let index = karus_index;
                    karus_index += 1;
                    (KARUS_ENTRY_START, KARUS_ENTRY_END, index)
                }
                crate::world::NATION_ELMORAD => {
                    let index = elmorad_index;
                    elmorad_index += 1;
                    (ELMORAD_ENTRY_START, ELMORAD_ENTRY_END, index)
                }
                _ => {
                    tracing::warn!(sid, nation, "Manes Survival participant has an invalid nation");
                    self.forget(sid);
                    continue;
                }
            };

            // Golden-ratio spacing is deterministic, avoids stacked users and
            // stays strictly between the user-supplied outer-ring endpoints.
            let fraction = (((nation_index + 1) as f32) * 0.618_034).fract();
            let x = start.0 + (end.0 - start.0) * fraction;
            let z = start.1 + (end.1 - start.1) * fraction;
            let zone_id = ZONES_MANES_SURVIVAL[zone_index % ZONES_MANES_SURVIVAL.len()];
            let valid = world
                .get_zone(zone_id)
                .map(|zone| zone.is_valid_position(x, z))
                .unwrap_or(false);
            if !valid {
                tracing::warn!(sid, zone_id, x, z, "Manes Survival entry position is invalid");
                self.forget(sid);
                continue;
            }

            // Real max HP/MP are remembered first; leftovers of an older run are taken.
            self.begin_for(world, sid);

            // The client keeps the registration countdown alive until this
            // exact event-state packet initialises CSurvival. Send it before
            // zone change so the first event NPC death cannot race ahead of
            // client initialisation.
            let progress = self.reset_progress(sid);
            let initial_max_exp = progress.max_exp;
            // Client reverse contract (sub_7113D0 -> sub_716B50): this
            // 1-based value selects the complete temporary Manes loadout.
            // Manes is a Chaos-style event: every participant must receive
            // the same skills, clothing and weapon regardless of their normal
            // class, nation, race or gender. Row 3 is the client-confirmed
            // canonical loadout used by the last known-good test.
            const UNIFORM_MANES_LOADOUT: u8 = 3;
            world.send_to_session_owned(
                sid,
                crate::handler::survival::build_event_start(
                    UNIFORM_MANES_LOADOUT,
                    crate::handler::survival::EVENT_DURATION_SECONDS,
                    0,
                    initial_max_exp,
                    1,
                ),
            );
            crate::handler::attack::sync_manes_vitals_and_level(world, sid, progress, true);

            crate::handler::zone_change::server_teleport_to_zone_force(world, sid, zone_id, x, z);
            placed += 1;
        }

        placed
    }

    /// Spawn the configured event population exactly once.
    pub fn start(&self, world: &WorldState) -> anyhow::Result<usize> {
        if self.active.swap(true, Ordering::AcqRel) {
            return Ok(0);
        }

        self.registration_open.store(false, Ordering::Release);
        *self.started_at.write() = Some(Instant::now());
        self.final_boss_killed_at.write().take();

        let rows = self.spawns.read().clone();
        if rows.is_empty() {
            self.active.store(false, Ordering::Release);
            anyhow::bail!("Manes Survival spawn configuration is empty");
        }

        let mut total = 0usize;
        for zone_id in ZONES_MANES_SURVIVAL {
            let zone = match world.get_zone(zone_id) {
                Some(zone) => zone,
                None => {
                    self.stop(world);
                    anyhow::bail!("Manes Survival zone {zone_id} is not loaded");
                }
            };

            for row in &rows {
                let count = row.spawn_count as usize
                    * if matches!(row.grade, 1 | 2) {
                        LOW_MIDDLE_SPAWN_MULTIPLIER
                    } else {
                        1
                    };
                let phase = (row.npc_id.rem_euclid(360) as f32).to_radians();
                let mut created = 0usize;

                for index in 0..count {
                    // Deterministic golden-angle scatter fills the configured local area
                    // without stacking every monster on a diagonal line.
                    let fraction = if count <= 1 {
                        0.0
                    } else {
                        ((index + 1) as f32 / count as f32).sqrt()
                    };
                    let angle = phase + index as f32 * 2.399_963_1;
                    let radius = row.spawn_range as f32 * fraction;
                    let x = row.spawn_x as f32 + angle.cos() * radius;
                    let z = row.spawn_z as f32 + angle.sin() * radius;

                    if !zone.is_valid_position(x, z) {
                        self.stop(world);
                        anyhow::bail!(
                            "Manes Survival zone {zone_id} NPC {} has out-of-map position ({x:.1}, {z:.1})",
                            row.npc_id
                        );
                    }

                    let spawned_ids = world.spawn_event_npc_ex(
                        row.npc_id as u16,
                        true,
                        zone_id,
                        x,
                        z,
                        1,
                        MANES_EVENT_ROOM,
                        row.boss_tier as u8,
                    );
                    let regen_time_ms = if row.npc_id == DARK_DRAGON_SID {
                        0
                    } else if row.boss_tier > 0 {
                        MANES_BOSS_RESPAWN_MS
                    } else {
                        MANES_NORMAL_RESPAWN_MS
                    };
                    for npc_id in &spawned_ids {
                        world.update_npc_ai(*npc_id, |ai| {
                            ai.regen_time_ms = regen_time_ms;
                        });
                    }
                    created += spawned_ids.len();
                }

                if created != count {
                    self.stop(world);
                    anyhow::bail!(
                        "Manes Survival zone {zone_id} failed to spawn NPC {}: expected {}, created {}",
                        row.npc_id,
                        count,
                        created
                    );
                }
                total += created;
            }
        }
        Ok(total)
    }

    /// Remove only runtime NPCs owned by the four Manes physical zones.
    pub fn stop(&self, world: &WorldState) {
        for zone_id in ZONES_MANES_SURVIVAL {
            world.despawn_room_npcs(zone_id, MANES_EVENT_ROOM);
        }

        // Clean every live inventory before event state is discarded. A player
        // may already have left the Manes zone or been removed from participants,
        // but temporary HP/MP potions must still disappear immediately. Offline
        // users remain covered by filtered save and login sanitisation.
        for session_id in world.collect_session_ids() {
            self.cleanup_temporary_items_for_session(world, session_id);
        }

        self.active.store(false, Ordering::Release);
        self.registration_open.store(false, Ordering::Release);
        self.started_at.write().take();
        self.registration_deadline.write().take();
        self.final_boss_killed_at.write().take();
        self.participants.clear();
        self.progress.clear();
        self.unlocked_magic.clear();
        self.selected_rows.clear();
        self.offers.clear();
    }

    /// Finalise a successfully running event. Rewards are snapshotted and
    /// granted before `stop()` clears participant/progress state.
    ///
    /// Returns `(rewarded, failed)` so the GM command and logs never report a
    /// successful payout when an inventory could not accept the item.
    pub fn reward_rankings_and_stop(&self, world: &WorldState) -> (usize, usize) {
        let mut ranking: Vec<_> = self
            .participant_ids()
            .into_iter()
            .filter_map(|sid| {
                let progress = self.progress(sid)?;
                let character = world.get_character_info(sid)?;
                Some((
                    sid,
                    progress.level,
                    score_for_level(progress.level),
                    character.name,
                ))
            })
            .collect();
        ranking.sort_by(|left, right| {
            right
                .2
                .cmp(&left.2)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.0.cmp(&right.0))
        });

        let mut rewarded = 0usize;
        let mut failed = 0usize;
        for (index, (sid, level, score, name)) in ranking.into_iter().enumerate() {
            let rank = index + 1;
            let count = orb_reward_for_rank(rank);
            // The event potions may fill the bag; they are worthless after the event.
            self.cleanup_temporary_items_for_session(world, sid);
            if world.give_item(sid, MANES_ORB_ITEM_ID, count) {
                rewarded += 1;
                tracing::info!(
                    sid,
                    character = %name,
                    rank,
                    survival_level = level,
                    survival_score = score,
                    item_id = MANES_ORB_ITEM_ID,
                    item_count = count,
                    "Manes Survival ranking reward granted"
                );
            } else {
                failed += 1;
                self.pending_rewards.insert(sid, count);
                tracing::error!(
                    sid,
                    character = %name,
                    rank,
                    item_id = MANES_ORB_ITEM_ID,
                    item_count = count,
                    "Manes Survival ranking reward could not be granted (retried on exit)"
                );
            }
        }

        self.stop(world);
        (rewarded, failed)
    }

    /// End-of-event evacuation. Rewards and temporary-item cleanup have already
    /// completed when this is scheduled. After a short result-viewing window,
    /// restore the persistent character stats/HUD and move every participant
    /// still inside a physical Manes zone to their nation homeland.
    pub fn schedule_participant_exit(world: Arc<WorldState>, participants: Vec<SessionId>) {
        let notice = crate::systems::timed_notice::build_notice_packet(
            8,
            &format!(
                "Manes Survival sona erdi. {} saniye sonra normal haritaya aktarilacaksiniz.",
                MANES_EXIT_DELAY_SECONDS
            ),
        );
        for sid in participants.iter().copied() {
            world.send_to_session_owned(sid, notice.clone());
        }

        tokio::spawn(async move {
            // Keep the original center Survival UIF visible as the result
            // countdown. D0 02 02 owns the boss-name/timer area; updating it
            // once per second gives the same visible finish sequence as the
            // other instanced events instead of a chat-only notice.
            for remaining in (1..=MANES_EXIT_DELAY_SECONDS).rev() {
                let finish_status = crate::handler::survival::build_event_boss_status(
                    "Etkinlik sona erdi",
                    1,
                    0,
                    remaining as u32,
                );
                for sid in participants.iter().copied() {
                    if world
                        .get_position(sid)
                        .map(|position| ZONES_MANES_SURVIVAL.contains(&position.zone_id))
                        .unwrap_or(false)
                    {
                        world.send_to_session_owned(sid, finish_status.clone());
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            for sid in participants {
                let Some(position) = world.get_position(sid) else {
                    world.manes_survival_manager.pending_rewards.remove(&sid);
                    continue;
                };

                // The real max HP/MP come back exactly, event potions go, the
                // client gets the real bars before its Survival mode is closed.
                world.manes_survival_manager.leave_event(&world, sid, true);

                // An Orb reward that did not fit the bag is tried again now.
                if let Some((_, count)) = world.manes_survival_manager.pending_rewards.remove(&sid)
                {
                    let text = if world.give_item(sid, MANES_ORB_ITEM_ID, count) {
                        format!("Manes Survival odulu: {count} Manes Orb envanterine eklendi.")
                    } else {
                        format!(
                            "Envanterin dolu: {count} Manes Orb verilemedi. Yer actiktan sonra bir GM ile gorus."
                        )
                    };
                    world.send_to_session_owned(
                        sid,
                        crate::systems::timed_notice::build_notice_packet(8, &text),
                    );
                }

                if !ZONES_MANES_SURVIVAL.contains(&position.zone_id) {
                    continue;
                }

                let Some(character) = world.get_character_info(sid) else {
                    continue;
                };
                // D0 02 04 is the verified Survival shutdown contract. It
                // disables the temporary loadout and restores all 28 normal
                // bag slots, including the permanent Manes Orb reward.
                if let Some(restore_packet) =
                    crate::handler::survival::build_event_finish_restore(&world, sid).await
                {
                    world.send_to_session_owned(sid, restore_packet);
                }
                let equipped = world.get_equipped_stats(sid);
                let mut level_packet = Packet::new(Opcode::WizLevelChange as u8);
                level_packet.write_u32(sid as u32);
                level_packet.write_u8(character.level);
                level_packet.write_i16(character.free_points as i16);
                level_packet.write_u8(character.skill_points[0]);
                level_packet.write_i64(character.max_exp);
                level_packet.write_i64(character.exp as i64);
                level_packet.write_i16(character.max_hp);
                level_packet.write_i16(character.hp);
                level_packet.write_i16(character.max_mp);
                level_packet.write_i16(character.mp);
                level_packet.write_u32(equipped.max_weight);
                level_packet.write_u32(equipped.item_weight);
                world.send_to_session_owned(sid, level_packet);
                world.send_item_move_refresh(sid);

                // A dead player is brought back first, otherwise they would arrive dead.
                if world.is_player_dead(sid) {
                    world.update_res_hp_type(sid, crate::world::USER_STANDING);
                    world.update_session(sid, |h| {
                        h.who_killed_me = -1;
                        h.lost_exp = 0;
                    });
                    world.update_character_hp(sid, character.max_hp);
                    world.send_to_session_owned(
                        sid,
                        crate::systems::regen::build_hp_change_packet(
                            character.max_hp,
                            character.max_hp,
                        ),
                    );
                }

                let home_zone = crate::systems::war::nation_home_zone(character.nation);
                crate::handler::zone_change::server_teleport_to_zone(
                    &world, sid, home_zone, 0.0, 0.0,
                );
                tracing::info!(
                    sid,
                    character = %character.name,
                    persistent_level = character.level,
                    inventory_slots = world
                        .with_session(sid, |handle| handle.inventory.len())
                        .unwrap_or(0),
                    home_zone,
                    "Manes Survival participant restored and evacuated"
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_vitals_match_supplied_gameplay_capture() {
        assert_eq!(vitals_for_level(1, 0), (1_060, 200));
        assert_eq!(vitals_for_level(10, 0), (10_060, 2_000));
        assert_eq!(vitals_for_level(13, 0), (13_060, 2_600));
        assert_eq!(vitals_for_level(13, 500), (13_560, 2_600));
    }

    #[test]
    fn reference_score_matches_supplied_gameplay_capture() {
        assert_eq!(score_for_level(1), 0);
        assert_eq!(score_for_level(10), 180);
        assert_eq!(score_for_level(13), 240);
    }

    #[test]
    fn attack_scale_increases_at_every_survival_level() {
        assert_eq!(attack_scale_per_mille(1), 150);
        assert_eq!(attack_scale_per_mille(2), 175);
        assert_eq!(attack_scale_per_mille(30), 875);
    }

    #[test]
    fn ranking_orb_rewards_match_event_contract() {
        assert_eq!(orb_reward_for_rank(1), 150);
        assert_eq!(orb_reward_for_rank(2), 100);
        assert_eq!(orb_reward_for_rank(3), 50);
        assert_eq!(orb_reward_for_rank(4), 20);
        assert_eq!(orb_reward_for_rank(200), 20);
    }

    #[test]
    fn vitals_override_only_while_a_participant_plays() {
        let manager = ManesSurvivalManager::default();
        assert_eq!(manager.vitals_override(1), None);
        manager.reset_progress(1);
        // event not running yet: the real vitals stay
        assert_eq!(manager.vitals_override(1), None);
        manager.active.store(true, Ordering::Release);
        assert_eq!(manager.vitals_override(1), Some((1_060, 200)));
        assert!(manager.is_participant_playing(1));
        assert!(!manager.is_participant_playing(2));
        manager.forget(1);
        assert_eq!(manager.vitals_override(1), None);
    }

    #[test]
    fn temporary_cleanup_removes_only_event_potions() {
        let manager = ManesSurvivalManager::default();
        let mut inventory = vec![
            UserItemSlot {
                item_id: MANES_TEMP_HP_POTION_ITEM_ID,
                ..UserItemSlot::default()
            },
            UserItemSlot {
                item_id: MANES_TEMP_MP_POTION_ITEM_ID,
                ..UserItemSlot::default()
            },
            UserItemSlot {
                item_id: MANES_ORB_ITEM_ID, // Manes Orb / permanent exchange reward
                ..UserItemSlot::default()
            },
        ];

        assert_eq!(manager.remove_temporary_items(&mut inventory), 2);
        assert_eq!(inventory[0].item_id, 0);
        assert_eq!(inventory[1].item_id, 0);
        assert_eq!(inventory[2].item_id, MANES_ORB_ITEM_ID);
    }
}
