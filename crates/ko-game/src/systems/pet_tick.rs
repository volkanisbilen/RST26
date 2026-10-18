//! Pet satisfaction decay tick system.
//! Every 60 seconds (`PLAYER_TRAINING_INTERVAL * 4` = 15 * 4), for each online
//! player with an active pet, decrease satisfaction by 100. When satisfaction
//! reaches 0, the pet dies and is de-summoned.
//! ## Constants (from C++)
//! - `PLAYER_TRAINING_INTERVAL` = 15 seconds (`User.h:46`)
//! - Decay interval = 15 * 4 = 60 seconds
//! - Decay amount = -100 per tick
//! - Max satisfaction = 10000, pet dies at 0

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ko_protocol::{Opcode, Packet};
use tracing::debug;

use crate::world::{WorldState, PET_DECAY_AMOUNT};

/// Background tick interval for pet decay checks.
/// We check every 15 seconds (matching `PLAYER_TRAINING_INTERVAL`) but only
/// decay if 60 seconds have elapsed for each individual session since their
/// last decay. This allows staggered decay without a single 60s global tick.
const PET_TICK_INTERVAL_SECS: u64 = 15;

/// Pet satisfaction update sub-opcode (MODE_SATISFACTION_UPDATE).
const MODE_SATISFACTION_UPDATE: u8 = 0x0F;

/// Pet mode function opcode (1).
const PET_MODE_FUNCTION: u8 = 1;

/// Pet death notification: sub-opcode 5 (NormalMode), sub-sub 2 (death).
const NORMAL_MODE: u8 = 5;

/// Start the pet satisfaction decay background task.
/// Returns a `JoinHandle` so the caller can abort on shutdown.
pub fn start_pet_tick_task(world: Arc<WorldState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(PET_TICK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            process_pet_decay_tick(&world);
        }
    })
}

/// Process one pet decay tick for all sessions with active pets.
/// ```cpp
/// if (m_bPetLastTime + (PLAYER_TRAINING_INTERVAL * 4) < UNIXTIME)
/// {
///     PetSatisFactionUpdate(-100);
///     m_bPetLastTime = UNIXTIME;
/// }
/// ```
fn process_pet_decay_tick(world: &WorldState) {
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Collect all sessions that need pet decay processing
    let decay_data = world.collect_pet_decay_data(now_unix);

    for pd in decay_data {
        // Apply the decay amount (-100)
        let new_sat = world.apply_pet_decay(pd.session_id, -(PET_DECAY_AMOUNT), now_unix);

        match new_sat {
            Some(satisfaction) => {
                // Pet is still alive — send satisfaction update packet
                //                       << satisfaction << u32(nid)
                let mut pkt = Packet::new(Opcode::WizPet as u8);
                pkt.write_u8(PET_MODE_FUNCTION);
                pkt.write_u8(MODE_SATISFACTION_UPDATE);
                pkt.write_u16(satisfaction as u16);
                pkt.write_u32(pd.pet_nid as u32);
                world.send_to_session_owned(pd.session_id, pkt);

                debug!(
                    "[pet_tick] sid={} pet satisfaction decayed to {}",
                    pd.session_id, satisfaction
                );
            }
            None => {
                if pd.pet_nid != 0 {
                    world.kill_npc(pd.pet_nid as u32);
                }
                // Pet died (satisfaction hit 0) — send death notification
                let mut death_pkt = Packet::new(Opcode::WizPet as u8);
                death_pkt.write_u8(PET_MODE_FUNCTION);
                death_pkt.write_u8(NORMAL_MODE);
                death_pkt.write_u8(2); // death sub-code
                death_pkt.write_u16(1);
                death_pkt.write_u32(pd.pet_index);
                world.send_to_session_owned(pd.session_id, death_pkt);

                debug!(
                    "[pet_tick] sid={} pet died (index={})",
                    pd.session_id, pd.pet_index
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::PET_DECAY_INTERVAL_SECS;
    use ko_protocol::PacketReader;

    #[test]
    fn test_pet_decay_interval_matches_cpp() {
        // C++ PLAYER_TRAINING_INTERVAL = 15, pet decay = 15 * 4 = 60
        assert_eq!(PET_DECAY_INTERVAL_SECS, 60);
    }

    #[test]
    fn test_pet_decay_amount_matches_cpp() {
        // C++ PetSatisFactionUpdate(-100)
        assert_eq!(PET_DECAY_AMOUNT, 100);
    }

    #[test]
    fn test_pet_tick_interval() {
        // We check every 15 seconds (PLAYER_TRAINING_INTERVAL)
        assert_eq!(PET_TICK_INTERVAL_SECS, 15);
    }

    #[test]
    fn test_pet_satisfaction_update_packet_format() {
        let mut pkt = Packet::new(Opcode::WizPet as u8);
        pkt.write_u8(PET_MODE_FUNCTION);
        pkt.write_u8(MODE_SATISFACTION_UPDATE);
        pkt.write_u16(9900); // satisfaction after -100 from 10000
        pkt.write_u32(42); // pet NPC id

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(MODE_SATISFACTION_UPDATE));
        assert_eq!(r.read_u16(), Some(9900));
        assert_eq!(r.read_u32(), Some(42));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_death_packet_format() {
        let mut pkt = Packet::new(Opcode::WizPet as u8);
        pkt.write_u8(PET_MODE_FUNCTION);
        pkt.write_u8(NORMAL_MODE);
        pkt.write_u8(2); // death
        pkt.write_u16(1);
        pkt.write_u32(12345);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(PET_MODE_FUNCTION));
        assert_eq!(r.read_u8(), Some(NORMAL_MODE));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u16(), Some(1));
        assert_eq!(r.read_u32(), Some(12345));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_pet_decay_with_world_state() {
        use crate::world::{PetState, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set up a pet with 500 satisfaction
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 500,
                nid: 10,
                index: 99,
                ..Default::default()
            });
            h.last_pet_decay_time = 0;
        });

        // Apply decay
        let result = world.apply_pet_decay(sid, -100, 100);
        assert_eq!(result, Some(400));

        // Verify the pet satisfaction was updated
        let sat = world
            .with_session(sid, |h| h.pet_data.as_ref().map(|p| p.satisfaction))
            .flatten();
        assert_eq!(sat, Some(400));

        // Verify last_pet_decay_time was updated
        let last_time = world.with_session(sid, |h| h.last_pet_decay_time).unwrap();
        assert_eq!(last_time, 100);
    }

    #[test]
    fn test_pet_decay_kills_pet_at_zero() {
        use crate::world::{PetState, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set up a pet with 50 satisfaction (will die with -100 decay)
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 50,
                nid: 5,
                index: 42,
                ..Default::default()
            });
            h.last_pet_decay_time = 0;
        });

        // Apply decay — should kill the pet
        let result = world.apply_pet_decay(sid, -100, 200);
        assert!(result.is_none(), "Pet should be dead");

        // The runtime companion disappears, but its persistent data and
        // equipped items must survive a dismissal/death.
        let pet = world
            .with_session(sid, |h| h.pet_data.clone())
            .unwrap()
            .unwrap();
        assert_eq!(pet.nid, 0);
        assert_eq!(pet.satisfaction, 0);
    }

    #[test]
    fn test_pet_decay_exactly_zero() {
        use crate::world::{PetState, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Pet with exactly 100 satisfaction — decays to 0
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 100,
                nid: 1,
                index: 1,
                ..Default::default()
            });
        });

        let result = world.apply_pet_decay(sid, -100, 300);
        assert!(result.is_none(), "Pet should die at exactly 0");
    }

    #[test]
    fn test_pet_decay_no_pet() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // No pet — should be a no-op
        let result = world.apply_pet_decay(sid, -100, 400);
        assert!(result.is_none());
    }

    #[test]
    fn test_collect_pet_decay_data_filters_no_pet() {
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // No pet — should not be collected
        let data = world.collect_pet_decay_data(100);
        assert!(data.is_empty());
    }

    /// Helper to create a minimal CharacterInfo for tests.
    fn make_test_character(sid: crate::zone::SessionId) -> crate::world::CharacterInfo {
        crate::world::CharacterInfo {
            session_id: sid,
            name: format!("TestPlayer{}", sid),
            nation: 1,
            race: 1,
            class: 101,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 5000,
            hp: 5000,
            max_mp: 3000,
            mp: 3000,
            max_sp: 0,
            sp: 0,
            equipped_items: [0u32; 14],
            bind_zone: 0,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 60,
            free_points: 0,
            skill_points: [0u8; 10],
            gold: 0,
            loyalty: 0,
            loyalty_monthly: 0,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 100_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 1000,
            res_hp_type: 1,
            rival_id: -1,
            rival_expiry_time: 0,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        }
    }

    #[test]
    fn test_collect_pet_decay_data_filters_by_interval() {
        use crate::world::{PetState, WorldState};
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let sid = world.allocate_session_id();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(sid, tx);

        // Set up a pet but with recent decay time
        world.update_session(sid, |h| {
            h.pet_data = Some(PetState {
                satisfaction: 5000,
                nid: 10,
                index: 99,
                ..Default::default()
            });
            h.last_pet_decay_time = 50; // decayed at time 50
            h.character = Some(make_test_character(sid));
        });

        // Time 80 — only 30s since last decay, need 60 — should NOT collect
        let data = world.collect_pet_decay_data(80);
        assert!(
            data.is_empty(),
            "Should not collect when interval not elapsed"
        );

        // Time 120 — 70s since last decay (50 + 60 < 120) — SHOULD collect
        let data = world.collect_pet_decay_data(120);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].session_id, sid);
        assert_eq!(data[0].satisfaction, 5000);
    }
}
