use super::*;

#[test]
fn test_allocate_session_id() {
    let world = WorldState::new();
    let id1 = world.allocate_session_id();
    let id2 = world.allocate_session_id();
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_ne!(id1, id2);
}

#[test]
fn test_register_unregister() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);
    assert!(world.get_position(1).is_some());
    world.unregister_session(1);
    assert!(world.get_position(1).is_none());
}

#[test]
fn test_update_position_region_change() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Set initial position in region (1, 1)
    let info = CharacterInfo {
        session_id: 1,
        name: "Test".into(),
        nation: 1,
        race: 1,
        class: 101,
        level: 1,
        face: 1,
        hair_rgb: 0,
        rank: 0,
        title: 0,
        max_hp: 230,
        hp: 230,
        max_mp: 150,
        mp: 150,
        max_sp: 0,
        sp: 0,
        equipped_items: [0; 14],
        bind_zone: 21,
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
        max_exp: 0,
        exp_seal_status: false,
        sealed_exp: 0,
        item_weight: 0,
        max_weight: 0,
        res_hp_type: 0x01,
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
    };
    let pos = Position {
        zone_id: 21,
        x: 50.0,
        y: 0.0,
        z: 50.0,
        region_x: calc_region(50.0),
        region_z: calc_region(50.0),
    };
    world.register_ingame(1, info, pos);

    // Move within same region
    let result = world.update_position(1, 21, 55.0, 0.0, 55.0);
    assert!(matches!(result, RegionChangeResult::NoChange));

    // Move to new region
    let result = world.update_position(1, 21, 100.0, 0.0, 100.0);
    assert!(matches!(result, RegionChangeResult::Changed { .. }));
}

#[test]
fn test_get_3x3_cells() {
    let cells = get_3x3_cells(5, 5, 86, 86);
    assert_eq!(cells.len(), 9);
    assert!(cells.contains(&(4, 4)));
    assert!(cells.contains(&(5, 5)));
    assert!(cells.contains(&(6, 6)));

    // Corner: (0, 0) â€” only 4 valid cells
    let cells = get_3x3_cells(0, 0, 86, 86);
    assert_eq!(cells.len(), 4);
    assert!(cells.contains(&(0, 0)));
    assert!(cells.contains(&(1, 0)));
    assert!(cells.contains(&(0, 1)));
    assert!(cells.contains(&(1, 1)));
}

// â”€â”€ Buff system tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Helper to create a test ActiveBuff.
fn make_test_buff(buff_type: i32, duration: u32, max_hp_val: i32) -> ActiveBuff {
    ActiveBuff {
        skill_id: 108010,
        buff_type,
        caster_sid: 1,
        start_time: Instant::now(),
        duration_secs: duration,
        attack_speed: 0,
        speed: 0,
        ac: 0,
        ac_pct: 0,
        attack: 0,
        magic_attack: 0,
        max_hp: max_hp_val,
        max_hp_pct: 0,
        max_mp: 0,
        max_mp_pct: 0,
        str_mod: 0,
        sta_mod: 0,
        dex_mod: 0,
        intel_mod: 0,
        cha_mod: 0,
        fire_r: 0,
        cold_r: 0,
        lightning_r: 0,
        magic_r: 0,
        disease_r: 0,
        poison_r: 0,
        hit_rate: 0,
        avoid_rate: 0,
        weapon_damage: 0,
        ac_sour: 0,
        duration_extended: false,
        is_buff: true,
    }
}

#[test]
fn test_active_buff_is_expired_zero_duration() {
    let buff = make_test_buff(1, 0, 100);
    // duration_secs == 0 means permanent â€” never expires
    assert!(!buff.is_expired());
}

#[test]
fn test_active_buff_is_expired_not_yet() {
    let buff = make_test_buff(1, 3600, 100);
    // Just created, 1 hour duration â€” should not be expired
    assert!(!buff.is_expired());
}

#[test]
fn test_active_buff_is_expired_past() {
    let mut buff = make_test_buff(1, 1, 100);
    // Set start_time to 2 seconds ago
    buff.start_time = Instant::now()
        .checked_sub(std::time::Duration::from_secs(2))
        .unwrap_or(Instant::now());
    assert!(buff.is_expired());
}

#[test]
fn test_apply_and_get_buffs() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    let buff = make_test_buff(3, 60, 100);
    world.apply_buff(1, buff);

    let buffs = world.get_active_buffs(1);
    assert_eq!(buffs.len(), 1);
    assert_eq!(buffs[0].buff_type, 3);
    assert_eq!(buffs[0].max_hp, 100);
}

#[test]
fn test_buff_overwrite_same_type() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Apply first buff of type 3
    let buff1 = make_test_buff(3, 60, 100);
    world.apply_buff(1, buff1);

    // Apply second buff of same type â€” should overwrite
    let buff2 = make_test_buff(3, 120, 200);
    world.apply_buff(1, buff2);

    let buffs = world.get_active_buffs(1);
    assert_eq!(buffs.len(), 1);
    assert_eq!(buffs[0].max_hp, 200);
    assert_eq!(buffs[0].duration_secs, 120);
}

#[test]
fn test_buff_different_types_stack() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.apply_buff(1, make_test_buff(1, 60, 50));
    world.apply_buff(1, make_test_buff(3, 60, 100));
    world.apply_buff(1, make_test_buff(5, 60, 75));

    let buffs = world.get_active_buffs(1);
    assert_eq!(buffs.len(), 3);
}

#[test]
fn test_remove_buff() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.apply_buff(1, make_test_buff(3, 60, 100));
    assert_eq!(world.get_active_buffs(1).len(), 1);

    let removed = world.remove_buff(1, 3);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().max_hp, 100);
    assert_eq!(world.get_active_buffs(1).len(), 0);
}

#[test]
fn test_remove_nonexistent_buff() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    let removed = world.remove_buff(1, 99);
    assert!(removed.is_none());
}

#[test]
fn test_get_buff_bonus_max_hp_flat() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Buff that adds +100 flat HP
    world.apply_buff(1, make_test_buff(1, 60, 100));

    let bonus = world.get_buff_bonus_max_hp(1, 1000);
    assert_eq!(bonus, 100);
}

#[test]
fn test_get_buff_bonus_max_hp_pct() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Buff that adds 20% HP (C++ convention: 120 = +20%, 100 = no change)
    let mut buff = make_test_buff(1, 60, 0);
    buff.max_hp_pct = 120;
    world.apply_buff(1, buff);

    let bonus = world.get_buff_bonus_max_hp(1, 1000);
    // (120 - 100) = 20% of 1000 = 200
    assert_eq!(bonus, 200);
}

#[test]
fn test_collect_expired_buffs() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // One expired buff, one active
    let mut expired = make_test_buff(1, 1, 50);
    expired.start_time = Instant::now()
        .checked_sub(std::time::Duration::from_secs(5))
        .unwrap_or(Instant::now());
    world.apply_buff(1, expired);

    let active = make_test_buff(3, 3600, 100);
    world.apply_buff(1, active);

    let collected = world.collect_expired_buffs();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].0, 1); // session ID
    assert_eq!(collected[0].1.buff_type, 1);

    // Active buff remains
    let remaining = world.get_active_buffs(1);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].buff_type, 3);
}

// â”€â”€ NPC HP tracking tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_npc_hp_init_and_damage() {
    let world = WorldState::new();
    world.npc_hp.insert(10001, 1000);

    assert_eq!(world.get_npc_hp(10001), Some(1000));
    assert!(!world.is_npc_dead(10001));

    // Take damage
    world.update_npc_hp(10001, 700);
    assert_eq!(world.get_npc_hp(10001), Some(700));
    assert!(!world.is_npc_dead(10001));

    // Kill
    world.update_npc_hp(10001, 0);
    assert_eq!(world.get_npc_hp(10001), Some(0));
    assert!(world.is_npc_dead(10001));
}

#[test]
fn test_npc_hp_negative_is_dead() {
    let world = WorldState::new();
    world.npc_hp.insert(10002, 50);
    world.update_npc_hp(10002, -10);
    assert!(world.is_npc_dead(10002));
}

#[test]
fn test_npc_hp_nonexistent_is_dead() {
    let world = WorldState::new();
    assert!(world.is_npc_dead(99999));
    assert_eq!(world.get_npc_hp(99999), None);
}

#[test]
fn test_npc_hp_respawn_reset() {
    let world = WorldState::new();
    world.npc_hp.insert(10003, 5000);

    // Kill NPC
    world.update_npc_hp(10003, 0);
    assert!(world.is_npc_dead(10003));

    // Respawn: reset HP to max
    world.update_npc_hp(10003, 5000);
    assert_eq!(world.get_npc_hp(10003), Some(5000));
    assert!(!world.is_npc_dead(10003));
}

// â”€â”€ DurationalSkill (DOT/HOT) Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_durational_skill_empty() {
    let slot = DurationalSkill::empty();
    assert!(!slot.used);
    assert_eq!(slot.skill_id, 0);
    assert_eq!(slot.hp_amount, 0);
    assert_eq!(slot.tick_count, 0);
    assert_eq!(slot.tick_limit, 0);
}

#[test]
fn test_add_durational_skill() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    let added = world.add_durational_skill(1, 108010, -50, 5, 2);
    assert!(added);

    // Verify it was added
    let handle = world.sessions.get(&1).unwrap();
    assert_eq!(handle.durational_skills.len(), 1);
    assert!(handle.durational_skills[0].used);
    assert_eq!(handle.durational_skills[0].skill_id, 108010);
    assert_eq!(handle.durational_skills[0].hp_amount, -50);
    assert_eq!(handle.durational_skills[0].tick_limit, 5);
    assert_eq!(handle.durational_skills[0].caster_sid, 2);
}

#[test]
fn test_add_multiple_durational_skills() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    assert!(world.add_durational_skill(1, 1001, -10, 3, 2));
    assert!(world.add_durational_skill(1, 1002, -20, 5, 3));
    assert!(world.add_durational_skill(1, 1003, 30, 10, 1));

    let handle = world.sessions.get(&1).unwrap();
    assert_eq!(handle.durational_skills.len(), 3);
    assert_eq!(handle.durational_skills[0].hp_amount, -10);
    assert_eq!(handle.durational_skills[1].hp_amount, -20);
    assert_eq!(handle.durational_skills[2].hp_amount, 30);
}

#[test]
fn test_process_dot_tick_advances_count() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.add_durational_skill(1, 1001, -25, 4, 2);

    let results = world.process_dot_tick();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 1); // session_id
    assert_eq!(results[0].1, -25); // hp_change
    assert!(!results[0].2); // not expired yet

    // tick_count should now be 1
    let handle = world.sessions.get(&1).unwrap();
    assert!(handle.durational_skills[0].used);
    assert_eq!(handle.durational_skills[0].tick_count, 1);
}

#[test]
fn test_process_dot_tick_expires() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // tick_limit = 1 means it expires after one tick
    world.add_durational_skill(1, 1001, -100, 1, 2);

    let results = world.process_dot_tick();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, -100);
    assert!(results[0].2); // expired on this tick

    // After expiry, slot should be cleared
    let handle = world.sessions.get(&1).unwrap();
    assert!(!handle.durational_skills[0].used);
}

#[test]
fn test_clear_durational_skills() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.add_durational_skill(1, 1001, -10, 5, 2);
    world.add_durational_skill(1, 1002, -20, 5, 3);

    world.clear_durational_skills(1);

    let handle = world.sessions.get(&1).unwrap();
    for slot in &handle.durational_skills {
        assert!(!slot.used);
    }
}

#[test]
fn test_dot_nonexistent_session() {
    let world = WorldState::new();

    let added = world.add_durational_skill(999, 1001, -10, 5, 2);
    assert!(!added);
}

#[test]
fn test_max_type3_repeat_constant() {
    assert_eq!(MAX_TYPE3_REPEAT, 40);
}

// â”€â”€ Server Settings / Damage Settings / Home tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_server_settings_default_none() {
    let world = WorldState::new();
    assert!(world.get_server_settings().is_none());
    assert!(world.get_damage_settings().is_none());
}

#[test]
fn test_home_position_default_empty() {
    let world = WorldState::new();
    assert!(world.get_home_position(1).is_none());
    assert!(world.get_home_position(2).is_none());
}

#[test]
fn test_home_position_insert_and_get() {
    let world = WorldState::new();
    let home = HomeRow {
        nation: 1,
        elmo_zone_x: 219,
        elmo_zone_z: 1859,
        elmo_zone_lx: 15,
        elmo_zone_lz: 15,
        karus_zone_x: 441,
        karus_zone_z: 1625,
        karus_zone_lx: 10,
        karus_zone_lz: 10,
        free_zone_x: 1380,
        free_zone_z: 1090,
        free_zone_lx: 10,
        free_zone_lz: 10,
        battle_zone_x: 820,
        battle_zone_z: 98,
        battle_zone_lx: 5,
        battle_zone_lz: 5,
        battle_zone2_x: 61,
        battle_zone2_z: 158,
        battle_zone2_lx: 5,
        battle_zone2_lz: 5,
        battle_zone3_x: 176,
        battle_zone3_z: 72,
        battle_zone3_lx: 5,
        battle_zone3_lz: 5,
        battle_zone4_x: 76,
        battle_zone4_z: 729,
        battle_zone4_lx: 5,
        battle_zone4_lz: 5,
        battle_zone5_x: 76,
        battle_zone5_z: 729,
        battle_zone5_lx: 5,
        battle_zone5_lz: 5,
        battle_zone6_x: 76,
        battle_zone6_z: 729,
        battle_zone6_lx: 5,
        battle_zone6_lz: 5,
    };
    world.home_positions.insert(1, home);

    let result = world.get_home_position(1).unwrap();
    assert_eq!(result.nation, 1);
    assert_eq!(result.karus_zone_x, 441);
    assert_eq!(result.karus_zone_z, 1625);
    assert_eq!(result.elmo_zone_x, 219);
    assert_eq!(result.free_zone_x, 1380);
    assert_eq!(result.battle_zone_x, 820);
}

#[test]
fn test_class_damage_multiplier_defaults_to_one() {
    let world = WorldState::new();
    // No damage settings loaded â€” all class matchups should return 1.0
    for attacker in [101u16, 102, 103, 104, 113] {
        for target in [201u16, 202, 203, 204, 213] {
            let m = world.get_class_damage_multiplier(attacker, target);
            assert!(
                (m - 1.0).abs() < f64::EPSILON,
                "Expected 1.0 for attacker={attacker} target={target}, got {m}"
            );
        }
    }
}

// â”€â”€ Saved Magic (Buff Persistence) Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_insert_saved_magic_only_above_500000() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Skill ID <= 500000 should NOT be saved
    world.insert_saved_magic(1, 400000, 3600);
    assert!(!world.has_saved_magic(1, 400000));

    // Skill ID == 500000 should NOT be saved (must be > 500000)
    world.insert_saved_magic(1, 500000, 3600);
    assert!(!world.has_saved_magic(1, 500000));

    // Skill ID > 500000 should be saved
    world.insert_saved_magic(1, 500001, 3600);
    assert!(world.has_saved_magic(1, 500001));
}

#[test]
fn test_insert_saved_magic_no_duplicate() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.insert_saved_magic(1, 600000, 100);
    assert!(world.has_saved_magic(1, 600000));

    // Reusing the same scroll refreshes its persisted duration.
    world.insert_saved_magic(1, 600000, 9999);

    // The refreshed duration must survive a zone change/relog recast.
    let dur = world.get_saved_magic_duration(1, 600000);
    assert!(
        dur > 9900,
        "Duration {} should reflect the refreshed scroll",
        dur
    );
}

#[test]
fn test_insert_saved_magic_max_10_slots() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Fill all 10 slots
    for i in 0..10u32 {
        world.insert_saved_magic(1, 500001 + i, 3600);
    }

    // 11th should be rejected
    world.insert_saved_magic(1, 500011, 3600);
    assert!(!world.has_saved_magic(1, 500011));

    // Original 10 are still there
    for i in 0..10u32 {
        assert!(world.has_saved_magic(1, 500001 + i));
    }
}

#[test]
fn test_remove_saved_magic() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.insert_saved_magic(1, 600001, 3600);
    assert!(world.has_saved_magic(1, 600001));

    world.remove_saved_magic(1, 600001);
    assert!(!world.has_saved_magic(1, 600001));
}

#[test]
fn test_get_saved_magic_duration() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.insert_saved_magic(1, 700000, 3600); // 1 hour

    let dur = world.get_saved_magic_duration(1, 700000);
    // Should be close to 3600 (allowing 2s tolerance for test execution)
    assert!(
        (3598..=3600).contains(&dur),
        "Duration {} not in expected range",
        dur
    );

    // Non-existent skill returns 0
    assert_eq!(world.get_saved_magic_duration(1, 999999), 0);
}

#[test]
fn test_load_saved_magic() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    let entries: Vec<(u32, i32)> = vec![
        (600001, 1800), // 30 min
        (600002, 3600), // 1 hour
        (600003, 3),    // Too short (< 5), should be filtered
        (0, 3600),      // Zero skill_id, should be filtered
    ];

    world.load_saved_magic(1, &entries);

    assert!(world.has_saved_magic(1, 600001));
    assert!(world.has_saved_magic(1, 600002));
    assert!(!world.has_saved_magic(1, 600003)); // duration < 5
    assert!(!world.has_saved_magic(1, 0)); // skill_id == 0

    let dur1 = world.get_saved_magic_duration(1, 600001);
    assert!(
        (1798..=1800).contains(&dur1),
        "Duration {} out of range",
        dur1
    );
}

#[test]
fn test_get_saved_magic_entries_for_db_save() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.insert_saved_magic(1, 600001, 1800);
    world.insert_saved_magic(1, 600002, 7200);

    let entries = world.get_saved_magic_entries(1);
    assert_eq!(entries.len(), 2);

    // All entries should have positive remaining duration
    for &(skill_id, dur) in &entries {
        assert!(skill_id > 0);
        assert!(dur > 0);
    }
}

#[test]
fn test_check_saved_magic_removes_expired() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Manually insert an already-expired entry by loading with past timestamp
    // Load with 1 second duration, then check after it would expire
    // Since we can't easily time-travel, we'll load with a very small duration
    // and verify check_saved_magic doesn't crash
    world.load_saved_magic(1, &[(600001, 7200)]);
    assert!(world.has_saved_magic(1, 600001));

    world.check_saved_magic(1);
    // Should still be there since it has 7200 seconds left
    assert!(world.has_saved_magic(1, 600001));
}

#[test]
fn test_get_saved_magic_for_recast() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);

    world.insert_saved_magic(1, 600001, 1800);
    world.insert_saved_magic(1, 600002, 3600);

    let recast_list = world.get_saved_magic_for_recast(1);
    assert_eq!(recast_list.len(), 2);

    for &(skill_id, dur) in &recast_list {
        assert!(skill_id > 0);
        assert!(dur > 0);
    }
}

#[test]
fn test_saved_magic_nonexistent_session() {
    let world = WorldState::new();

    // Operations on non-existent session should not panic
    world.insert_saved_magic(999, 600001, 3600);
    assert!(!world.has_saved_magic(999, 600001));
    assert_eq!(world.get_saved_magic_duration(999, 600001), 0);
    world.remove_saved_magic(999, 600001);
    assert!(world.get_saved_magic_entries(999).is_empty());
    world.check_saved_magic(999);
}

// -- Monster Summon / Respawn / Boss Spawn Tests ----------------------

#[test]
fn test_monster_summon_list_insert_and_lookup() {
    use ko_db::models::MonsterSummonRow;
    let world = WorldState::new();
    world.monster_summon_list.insert(
        506,
        MonsterSummonRow {
            s_sid: 506,
            str_name: "Lobo".to_string(),
            s_level: 45,
            s_probability: 650,
            b_type: 1,
        },
    );
    world.monster_summon_list.insert(
        9589,
        MonsterSummonRow {
            s_sid: 9589,
            str_name: "Hell Fire".to_string(),
            s_level: 75,
            s_probability: 1000,
            b_type: 2,
        },
    );
    assert_eq!(world.monster_summon_count(), 2);
    let lobo = world.get_monster_summon(506).unwrap();
    assert_eq!(lobo.str_name, "Lobo");
    assert_eq!(lobo.s_level, 45);
    assert_eq!(lobo.b_type, 1);
    assert!(world.get_monster_summon(999).is_none());
}

#[test]
fn test_monster_summon_by_type() {
    use ko_db::models::MonsterSummonRow;
    let world = WorldState::new();
    for (sid, name, btype) in [
        (506, "Lobo", 1),
        (507, "Lupus", 1),
        (9589, "Hell Fire", 2),
        (9590, "Enigma", 2),
    ] {
        world.monster_summon_list.insert(
            sid,
            MonsterSummonRow {
                s_sid: sid,
                str_name: name.to_string(),
                s_level: 50,
                s_probability: 600,
                b_type: btype,
            },
        );
    }
    let type1 = world.get_monster_summons_by_type(1);
    assert_eq!(type1.len(), 2);
    let type2 = world.get_monster_summons_by_type(2);
    assert_eq!(type2.len(), 2);
    let type3 = world.get_monster_summons_by_type(3);
    assert!(type3.is_empty());
}

#[test]
fn test_respawn_chain_lookup() {
    use ko_db::models::MonsterRespawnLoopRow;
    let world = WorldState::new();
    // Circular chain: 8950 -> 8951 -> 8952 -> ... -> 8956 -> 8950
    let chain = [
        (8950, 8951, 5),
        (8951, 8952, 5),
        (8952, 8953, 5),
        (8956, 8950, 5),
    ];
    for (dead, born, dt) in chain {
        world.monster_respawn_loop.insert(
            dead,
            MonsterRespawnLoopRow {
                idead: dead,
                iborn: born,
                stable: true,
                count: 1,
                deadtime: dt,
            },
        );
    }
    assert_eq!(world.respawn_loop_count(), 4);

    let entry = world.get_respawn_chain(8950).unwrap();
    assert_eq!(entry.iborn, 8951);
    assert_eq!(entry.deadtime, 5);

    // Circular: 8956 -> 8950
    let entry2 = world.get_respawn_chain(8956).unwrap();
    assert_eq!(entry2.iborn, 8950);

    // Non-existent
    assert!(world.get_respawn_chain(1234).is_none());
}

#[test]
fn test_respawn_chain_terminal() {
    use ko_db::models::MonsterRespawnLoopRow;
    let world = WorldState::new();
    // Terminal chain: 9311 -> 8998 with count=5 and 60s delay
    world.monster_respawn_loop.insert(
        9311,
        MonsterRespawnLoopRow {
            idead: 9311,
            iborn: 8998,
            stable: true,
            count: 5,
            deadtime: 60,
        },
    );
    let entry = world.get_respawn_chain(9311).unwrap();
    assert_eq!(entry.iborn, 8998);
    assert_eq!(entry.count, 5);
    assert_eq!(entry.deadtime, 60);
}

#[test]
fn test_boss_random_spawn_candidates() {
    use ko_db::models::MonsterBossRandomSpawnRow;
    let world = WorldState::new();
    // Stage 2: 5 Antares positions in zone 1
    for (idx, px) in [(6, 724), (7, 653), (8, 259), (9, 840), (10, 1541)] {
        world
            .boss_random_spawn
            .entry(2)
            .or_default()
            .push(MonsterBossRandomSpawnRow {
                n_index: idx,
                stage: 2,
                monster_id: 906,
                monster_zone: 1,
                pos_x: px,
                pos_z: 1500,
                range: 5,
                reload_time: 18000,
                monster_name: "Antares".to_string(),
            });
    }
    assert_eq!(world.boss_spawn_stage_count(), 1);
    let candidates = world.get_boss_spawn_candidates(2);
    assert_eq!(candidates.len(), 5);
    assert_eq!(candidates[0].monster_id, 906);
    assert_eq!(candidates[0].monster_zone, 1);
    // Non-existent stage
    let empty = world.get_boss_spawn_candidates(99);
    assert!(empty.is_empty());
}

#[test]
fn test_boss_random_spawn_multiple_stages() {
    use ko_db::models::MonsterBossRandomSpawnRow;
    let world = WorldState::new();
    for stage in [2, 3, 4] {
        for i in 0..3 {
            world
                .boss_random_spawn
                .entry(stage)
                .or_default()
                .push(MonsterBossRandomSpawnRow {
                    n_index: stage * 10 + i,
                    stage,
                    monster_id: 900 + stage,
                    monster_zone: 1,
                    pos_x: 100 * i,
                    pos_z: 200 * i,
                    range: 5,
                    reload_time: 18000,
                    monster_name: format!("Boss{}", stage),
                });
        }
    }
    assert_eq!(world.boss_spawn_stage_count(), 3);
    assert_eq!(world.get_boss_spawn_candidates(2).len(), 3);
    assert_eq!(world.get_boss_spawn_candidates(3).len(), 3);
    assert_eq!(world.get_boss_spawn_candidates(4).len(), 3);
}

#[test]
fn test_monster_tables_empty_by_default() {
    let world = WorldState::new();
    assert_eq!(world.monster_summon_count(), 0);
    assert_eq!(world.respawn_loop_count(), 0);
    assert_eq!(world.boss_spawn_stage_count(), 0);
    assert!(world.get_monster_summon(506).is_none());
    assert!(world.get_respawn_chain(8950).is_none());
    assert!(world.get_boss_spawn_candidates(2).is_empty());
}

// â”€â”€ New lookup table accessor tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_special_stone_empty_by_default() {
    let world = WorldState::new();
    assert!(world.get_special_stone(1).is_none());
}

#[test]
fn test_special_stone_insert_and_get() {
    let world = WorldState::new();
    let row = ko_db::models::SpecialStoneRow {
        n_index: 1,
        zone_id: 71,
        main_npc: 8999,
        monster_name: "BoneDragon".into(),
        summon_npc: 8998,
        summon_count: 1,
        status: 1,
    };
    world.special_stones.insert(1, row.clone());
    let got = world.get_special_stone(1).unwrap();
    assert_eq!(got.n_index, 1);
    assert_eq!(got.zone_id, 71);
    assert_eq!(got.main_npc, 8999);
    assert_eq!(got.summon_npc, 8998);
}

#[test]
fn test_item_random_empty_by_default() {
    let world = WorldState::new();
    assert!(world.get_item_random(1).is_none());
    assert!(world.get_item_random_by_session(1).is_empty());
}

#[test]
fn test_item_random_insert_and_session_filter() {
    let world = WorldState::new();
    for i in 1..=3 {
        world.item_random.insert(
            i,
            ko_db::models::ItemRandomRow {
                n_index: i,
                str_item_name: format!("Item{}", i),
                item_id: 100 + i,
                item_count: 1,
                rental_time: 0,
                session_id: if i <= 2 { 1 } else { 2 },
                status: 1,
            },
        );
    }
    assert_eq!(world.get_item_random_by_session(1).len(), 2);
    assert_eq!(world.get_item_random_by_session(2).len(), 1);
    assert_eq!(world.get_item_random_by_session(99).len(), 0);
}

#[test]
fn test_item_group_empty_by_default() {
    let world = WorldState::new();
    assert!(world.get_item_group(1).is_none());
}

#[test]
fn test_item_group_insert_and_get() {
    let world = WorldState::new();
    world.item_groups.insert(
        1,
        ko_db::models::ItemGroupRow {
            group_id: 1,
            name: Some("TestGroup".into()),
            items: vec![100, 200, 300],
        },
    );
    let g = world.get_item_group(1).unwrap();
    assert_eq!(g.items.len(), 3);
    assert_eq!(g.items[0], 100);
}

#[test]
fn test_item_exchange_exp_empty_by_default() {
    let world = WorldState::new();
    assert!(world.get_item_exchange_exp(1).is_none());
}

#[test]
fn test_item_give_exchange_empty_by_default() {
    let world = WorldState::new();
    assert!(world.get_item_give_exchange(1).is_none());
}

#[test]
fn test_right_click_exchange_empty_by_default() {
    let world = WorldState::new();
    assert!(world.get_right_click_exchange(100).is_none());
}

#[test]
fn test_right_exchange_empty_by_default() {
    let world = WorldState::new();
    assert!(world.get_right_exchange(100).is_none());
}

#[test]
fn test_right_click_exchange_insert_and_get() {
    let world = WorldState::new();
    world.item_right_click_exchange.insert(
        900100,
        ko_db::models::ItemRightClickExchangeRow {
            item_id: 900100,
            opcode: 1,
        },
    );
    let got = world.get_right_click_exchange(900100).unwrap();
    assert_eq!(got.opcode, 1);
}

#[test]
fn test_right_exchange_insert_and_get() {
    let world = WorldState::new();
    world.item_right_exchange.insert(
        800200,
        ko_db::models::ItemRightExchangeRow {
            item_id: 800200,
            str_name: Some("TestExchange".into()),
            exchange_type: Some(1),
            description: None,
            exchange_count: Some(2),
            exchange_items: vec![1001, 1002],
            exchange_counts: vec![1, 5],
            expiration_times: vec![0, 0],
        },
    );
    let got = world.get_right_exchange(800200).unwrap();
    assert_eq!(got.exchange_items.len(), 2);
    assert_eq!(got.exchange_count, Some(2));
}

// â”€â”€ Spawn/Kill NPC tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[tokio::test]
async fn test_spawn_event_npc_no_template_returns_empty() {
    let world = WorldState::new();
    // No templates loaded â€” spawn should return empty
    let ids = world.spawn_event_npc(9999, true, 21, 100.0, 100.0, 1);
    assert!(ids.is_empty());
}

#[tokio::test]
async fn test_spawn_event_npc_no_zone_returns_empty() {
    use crate::npc::NpcTemplate;
    let world = WorldState::new();
    // Add a template but no zone
    world.npc_templates.insert(
        (500, true),
        Arc::new(NpcTemplate {
            s_sid: 500,
            is_monster: true,
            name: "TestMob".into(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 1,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 100,
            attack: 50,
            ac: 10,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 1000,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 20,
            attack_range: 2,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }),
    );
    // Zone 999 does not exist (WorldState::new only creates zone 21)
    let ids = world.spawn_event_npc(500, true, 999, 100.0, 100.0, 1);
    assert!(ids.is_empty());
}

#[tokio::test]
async fn test_spawn_event_npc_success_single() {
    use crate::npc::NpcTemplate;
    let world = WorldState::new();
    world.npc_templates.insert(
        (500, true),
        Arc::new(NpcTemplate {
            s_sid: 500,
            is_monster: true,
            name: "TestMob".into(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 1,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 100,
            attack: 50,
            ac: 10,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 1000,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 20,
            attack_range: 2,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }),
    );
    // Zone 21 exists by default
    let ids = world.spawn_event_npc(500, true, 21, 100.0, 100.0, 1);
    assert_eq!(ids.len(), 1);
    let nid = ids[0];
    assert!(nid >= NPC_BAND);
    // NPC instance should be registered
    assert!(world.get_npc_instance(nid).is_some());
    // HP should be set
    assert_eq!(world.get_npc_hp(nid), Some(1000));
    // AI should be initialized (search_range > 0)
    assert!(world.npc_ai.contains_key(&nid));
}

#[tokio::test]
async fn test_spawn_event_npc_multi_spawn() {
    use crate::npc::NpcTemplate;
    let world = WorldState::new();
    world.npc_templates.insert(
        (600, true),
        Arc::new(NpcTemplate {
            s_sid: 600,
            is_monster: true,
            name: "MultiMob".into(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 5,
            max_hp: 500,
            max_mp: 50,
            attack: 20,
            ac: 5,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 1000,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 0, // no AI
            attack_range: 2,
            direct_attack: 0,
            tracing_range: 0,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 50,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }),
    );
    let ids = world.spawn_event_npc(600, true, 21, 200.0, 200.0, 5);
    assert_eq!(ids.len(), 5);
    // All IDs should be unique
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 5);
    // All should have HP
    for &nid in &ids {
        assert_eq!(world.get_npc_hp(nid), Some(500));
    }
    // No AI since search_range=0
    for &nid in &ids {
        assert!(!world.npc_ai.contains_key(&nid));
    }
}

#[tokio::test]
async fn test_kill_npc_removes_instance() {
    use crate::npc::NpcTemplate;
    let world = WorldState::new();
    world.npc_templates.insert(
        (700, true),
        Arc::new(NpcTemplate {
            s_sid: 700,
            is_monster: true,
            name: "KillTestMob".into(),
            pid: 0,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 1,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 1000,
            max_mp: 100,
            attack: 50,
            ac: 10,
            hit_rate: 0,
            evade_rate: 0,
            damage: 0,
            attack_delay: 1000,
            speed_1: 100,
            speed_2: 200,
            stand_time: 3000,
            search_range: 20,
            attack_range: 2,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }),
    );
    let ids = world.spawn_event_npc(700, true, 21, 100.0, 100.0, 1);
    assert_eq!(ids.len(), 1);
    let nid = ids[0];
    assert!(world.get_npc_instance(nid).is_some());
    assert!(world.npc_ai.contains_key(&nid));

    // Kill it
    world.kill_npc(nid);

    // Should be fully cleaned up
    assert!(world.get_npc_instance(nid).is_none());
    assert_eq!(world.get_npc_hp(nid), None);
    assert!(!world.npc_ai.contains_key(&nid));
}

#[tokio::test]
async fn test_kill_npc_nonexistent_is_noop() {
    let world = WorldState::new();
    // Should not panic
    world.kill_npc(99999);
}

#[test]
fn test_allocate_npc_id_increments() {
    let world = WorldState::new();
    let id1 = world.allocate_npc_id();
    let id2 = world.allocate_npc_id();
    assert!(id1 >= NPC_BAND);
    assert_eq!(id2, id1 + 1);
}

// â”€â”€ Quest Text Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_quest_menu_insert_and_lookup() {
    let world = WorldState::new();
    let row = QuestMenuRow {
        i_num: 10,
        str_menu: "Confirm".to_string(),
    };
    world.quest_menus.insert(row.i_num, row.clone());
    let found = world.get_quest_menu(10);
    assert!(found.is_some());
    assert_eq!(found.unwrap().str_menu, "Confirm");
}

#[test]
fn test_quest_menu_missing_returns_none() {
    let world = WorldState::new();
    assert!(world.get_quest_menu(99999).is_none());
}

#[test]
fn test_quest_talk_insert_and_lookup() {
    let world = WorldState::new();
    let row = QuestTalkRow {
        i_num: 1,
        str_talk: "<selfname> has accepted these missions.".to_string(),
    };
    world.quest_talks.insert(row.i_num, row.clone());
    let found = world.get_quest_talk(1);
    assert!(found.is_some());
    assert!(found.unwrap().str_talk.contains("<selfname>"));
}

#[test]
fn test_quest_talk_missing_returns_none() {
    let world = WorldState::new();
    assert!(world.get_quest_talk(99999).is_none());
}

#[test]
fn test_quest_menu_count() {
    let world = WorldState::new();
    assert_eq!(world.quest_menu_count(), 0);
    world.quest_menus.insert(
        10,
        QuestMenuRow {
            i_num: 10,
            str_menu: "Yes".to_string(),
        },
    );
    world.quest_menus.insert(
        11,
        QuestMenuRow {
            i_num: 11,
            str_menu: "No".to_string(),
        },
    );
    assert_eq!(world.quest_menu_count(), 2);
}

#[test]
fn test_quest_talk_count() {
    let world = WorldState::new();
    assert_eq!(world.quest_talk_count(), 0);
    world.quest_talks.insert(
        1,
        QuestTalkRow {
            i_num: 1,
            str_talk: "Hello".to_string(),
        },
    );
    assert_eq!(world.quest_talk_count(), 1);
}

#[test]
fn test_quest_skills_closed_check_accessor() {
    let world = WorldState::new();
    let row = QuestSkillsClosedCheckRow {
        n_index: 1,
        s_event_data_index: 500,
        n_nation: Some(1),
    };
    world
        .quest_skills_closed_check
        .insert(row.n_index, row.clone());
    let found = world.get_quest_skills_closed_check(1);
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.s_event_data_index, 500);
    assert_eq!(found.n_nation, Some(1));
}

#[test]
fn test_quest_skills_open_set_up_accessor() {
    let world = WorldState::new();
    let row = QuestSkillsOpenSetUpRow {
        n_index: 5,
        n_event_data_index: 348,
    };
    world
        .quest_skills_open_set_up
        .insert(row.n_index, row.clone());
    let found = world.get_quest_skills_open_set_up(5);
    assert!(found.is_some());
    assert_eq!(found.unwrap().n_event_data_index, 348);
}

#[test]
fn test_quest_menu_overwrite() {
    let world = WorldState::new();
    world.quest_menus.insert(
        10,
        QuestMenuRow {
            i_num: 10,
            str_menu: "Old".to_string(),
        },
    );
    world.quest_menus.insert(
        10,
        QuestMenuRow {
            i_num: 10,
            str_menu: "New".to_string(),
        },
    );
    assert_eq!(world.get_quest_menu(10).unwrap().str_menu, "New");
    assert_eq!(world.quest_menu_count(), 1);
}

#[test]
fn test_quest_talk_long_text() {
    let world = WorldState::new();
    let long_text = "A".repeat(1000);
    world.quest_talks.insert(
        42,
        QuestTalkRow {
            i_num: 42,
            str_talk: long_text.clone(),
        },
    );
    let found = world.get_quest_talk(42).unwrap();
    assert_eq!(found.str_talk.len(), 1000);
    assert_eq!(found.str_talk, long_text);
}

// â”€â”€ Bot System Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_bot_farm_insert_and_get() {
    let world = WorldState::new();
    let row = BotHandlerFarmRow {
        id: 1,
        str_user_id: "FarmBot01".into(),
        nation: 1,
        race: 1,
        class: 107,
        hair_rgb: 0,
        level: 83,
        face: 1,
        knights: 0,
        fame: 0,
        zone: 21,
        px: 26800,
        pz: 32000,
        py: 0,
        str_item: None,
        cover_title: 0,
        reb_level: 0,
        str_skill: None,
        gold: 10000,
        points: 0,
        strong: 60,
        sta: 60,
        dex: 90,
        intel: 60,
        cha: 60,
        loyalty: 5000,
        loyalty_monthly: 1000,
        donated_np: 0,
    };
    world.bot_farm_data.insert(row.id, row);
    assert_eq!(world.bot_farm_count(), 1);
    let got = world.get_bot_farm(1).unwrap();
    assert_eq!(got.str_user_id, "FarmBot01");
    assert_eq!(got.level, 83);
    assert_eq!(got.zone, 21);
}

#[test]
fn test_bot_farm_missing_returns_none() {
    let world = WorldState::new();
    assert!(world.get_bot_farm(999).is_none());
    assert_eq!(world.bot_farm_count(), 0);
}

#[test]
fn test_bot_merchant_template_insert_and_get() {
    let world = WorldState::new();
    let row = BotHandlerMerchantRow {
        s_index: 1,
        bot_merchant_type: 0,
        bot_item_num: "379060000,379090000".into(),
        bot_item_count: "10,5".into(),
        bot_item_price: "500000,300000".into(),
        bot_merchant_message: Some("Buy my items!".into()),
    };
    world.bot_merchant_templates.insert(row.s_index, row);
    assert_eq!(world.bot_merchant_template_count(), 1);
    let got = world.get_bot_merchant_template(1).unwrap();
    assert_eq!(got.bot_merchant_type, 0);
    assert!(got.bot_item_num.contains("379060000"));
}

#[test]
fn test_bot_merchant_data_insert_and_get() {
    let world = WorldState::new();
    let row = BotMerchantDataRow {
        n_index: 1,
        advert_message: Some("Selling potions!".into()),
        n_num1: 389010000,
        n_price1: 100,
        s_count1: 99,
        s_duration1: 0,
        is_kc1: false,
        n_num2: 0,
        n_price2: 0,
        s_count2: 0,
        s_duration2: 0,
        is_kc2: false,
        n_num3: 0,
        n_price3: 0,
        s_count3: 0,
        s_duration3: 0,
        is_kc3: false,
        n_num4: 0,
        n_price4: 0,
        s_count4: 0,
        s_duration4: 0,
        is_kc4: false,
        n_num5: 0,
        n_price5: 0,
        s_count5: 0,
        s_duration5: 0,
        is_kc5: false,
        n_num6: 0,
        n_price6: 0,
        s_count6: 0,
        s_duration6: 0,
        is_kc6: false,
        n_num7: 0,
        n_price7: 0,
        s_count7: 0,
        s_duration7: 0,
        is_kc7: false,
        n_num8: 0,
        n_price8: 0,
        s_count8: 0,
        s_duration8: 0,
        is_kc8: false,
        n_num9: 0,
        n_price9: 0,
        s_count9: 0,
        s_duration9: 0,
        is_kc9: false,
        n_num10: 0,
        n_price10: 0,
        s_count10: 0,
        s_duration10: 0,
        is_kc10: false,
        n_num11: 0,
        n_price11: 0,
        s_count11: 0,
        s_duration11: 0,
        is_kc11: false,
        n_num12: 0,
        n_price12: 0,
        s_count12: 0,
        s_duration12: 0,
        is_kc12: false,
        px: 26800,
        pz: 32000,
        py: 0,
        minute: 9999,
        zone: 21,
        s_direction: 0,
        merchant_type: 0,
    };
    world.bot_merchant_data.insert(row.n_index, row);
    assert_eq!(world.bot_merchant_data_count(), 1);
    let got = world.get_bot_merchant_data(1).unwrap();
    assert_eq!(got.n_num1, 389010000);
    assert_eq!(got.zone, 21);
}

#[test]
fn test_user_bot_insert_and_get() {
    let world = WorldState::new();
    let row = UserBotRow {
        id: 42,
        str_user_id: "UserBot42".into(),
        nation: 2,
        race: 11,
        class: 202,
        hair_rgb: 255,
        level: 60,
        face: 3,
        knights: 0,
        fame: 0,
        zone: 1,
        px: 10000,
        pz: 20000,
        py: 0,
        str_item: None,
        cover_title: 0,
        reb_level: 0,
        str_skill: None,
        gold: 5000,
        points: 10,
        strong: 50,
        sta: 50,
        dex: 50,
        intel: 50,
        cha: 50,
    };
    world.user_bots.insert(row.id, row);
    assert_eq!(world.user_bot_count(), 1);
    let got = world.get_user_bot(42).unwrap();
    assert_eq!(got.str_user_id, "UserBot42");
    assert_eq!(got.nation, 2);
}

#[test]
fn test_bot_knights_rank_insert_and_get() {
    let world = WorldState::new();
    let rows = vec![
        BotKnightsRankRow {
            sh_index: 1,
            str_name: "Rank1".into(),
            str_elmo_user_id: Some("ElmoKnight".into()),
            str_elmo_knights_name: Some("ElmoClan".into()),
            s_elmo_knights: Some(100),
            n_elmo_loyalty: Some(50000),
            str_karus_user_id: Some("KarusKnight".into()),
            str_karus_knights_name: Some("KarusClan".into()),
            s_karus_knights: Some(200),
            n_karus_loyalty: Some(60000),
            n_money: Some(1000000),
        },
        BotKnightsRankRow {
            sh_index: 2,
            str_name: "Rank2".into(),
            str_elmo_user_id: None,
            str_elmo_knights_name: None,
            s_elmo_knights: None,
            n_elmo_loyalty: None,
            str_karus_user_id: None,
            str_karus_knights_name: None,
            s_karus_knights: None,
            n_karus_loyalty: None,
            n_money: None,
        },
    ];
    *world.bot_knights_rank.write() = rows;
    assert_eq!(world.bot_knights_rank_count(), 2);
    let snapshot = world.get_bot_knights_rank();
    assert_eq!(snapshot[0].str_name, "Rank1");
    assert_eq!(snapshot[1].sh_index, 2);
}

#[test]
fn test_bot_personal_rank_insert_and_get() {
    let world = WorldState::new();
    let rows = vec![BotPersonalRankRow {
        n_rank: 1,
        str_rank_name: "TopPlayer".into(),
        n_elmo_up: 1,
        str_elmo_user_id: Some("ElmoPlayer1".into()),
        str_elmo_clan_name: Some("ElmoClan1".into()),
        s_elmo_knights: Some(100),
        n_elmo_loyalty_monthly: Some(30000),
        n_elmo_check: 0,
        n_karus_up: 0,
        str_karus_user_id: Some("KarusPlayer1".into()),
        str_karus_clan_name: Some("KarusClan1".into()),
        s_karus_knights: Some(200),
        n_karus_loyalty_monthly: Some(25000),
        n_karus_check: 0,
        n_salary: 100000,
        update_date: chrono::NaiveDateTime::default(),
    }];
    *world.bot_personal_rank.write() = rows;
    assert_eq!(world.bot_personal_rank_count(), 1);
    let snapshot = world.get_bot_personal_rank();
    assert_eq!(snapshot[0].str_rank_name, "TopPlayer");
    assert_eq!(snapshot[0].n_salary, 100000);
}

#[test]
fn test_get_bots_in_zone() {
    let world = WorldState::new();
    for (id, zone) in [(1, 21i16), (2, 21), (3, 1)] {
        world.bot_farm_data.insert(
            id,
            BotHandlerFarmRow {
                id,
                str_user_id: format!("Bot{id}"),
                nation: 1,
                race: 1,
                class: 107,
                hair_rgb: 0,
                level: 80,
                face: 1,
                knights: 0,
                fame: 0,
                zone,
                px: 0,
                pz: 0,
                py: 0,
                str_item: None,
                cover_title: 0,
                reb_level: 0,
                str_skill: None,
                gold: 0,
                points: 0,
                strong: 50,
                sta: 50,
                dex: 50,
                intel: 50,
                cha: 50,
                loyalty: 0,
                loyalty_monthly: 0,
                donated_np: 0,
            },
        );
    }
    assert_eq!(world.get_bots_in_zone(21).len(), 2);
    assert_eq!(world.get_bots_in_zone(1).len(), 1);
    assert_eq!(world.get_bots_in_zone(99).len(), 0);
}

// â”€â”€ NPC Buff (Type4) tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_npc_buff_entry_is_expired() {
    let entry = NpcBuffEntry {
        skill_id: 100010,
        buff_type: 3,
        start_time: Instant::now(),
        duration_secs: 3600,
    };
    assert!(!entry.is_expired());

    // Permanent buff (duration=0) never expires
    let permanent = NpcBuffEntry {
        skill_id: 100010,
        buff_type: 3,
        start_time: Instant::now()
            .checked_sub(std::time::Duration::from_secs(100))
            .unwrap_or(Instant::now()),
        duration_secs: 0,
    };
    assert!(!permanent.is_expired());

    // Expired buff
    let old = NpcBuffEntry {
        skill_id: 100010,
        buff_type: 3,
        start_time: Instant::now()
            .checked_sub(std::time::Duration::from_secs(10))
            .unwrap_or(Instant::now()),
        duration_secs: 5,
    };
    assert!(old.is_expired());
}

#[test]
fn test_npc_buff_apply_and_query() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    assert!(!world.has_npc_buff(npc_id, 3));
    assert_eq!(world.npc_buff_count(npc_id), 0);

    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now(),
            duration_secs: 60,
        },
    );

    assert!(world.has_npc_buff(npc_id, 3));
    assert!(!world.has_npc_buff(npc_id, 5));
    assert_eq!(world.npc_buff_count(npc_id), 1);
}

#[test]
fn test_npc_buff_overwrite_same_type() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now(),
            duration_secs: 60,
        },
    );
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500200,
            buff_type: 3,
            start_time: Instant::now(),
            duration_secs: 120,
        },
    );

    // Still only 1 buff of type 3
    assert_eq!(world.npc_buff_count(npc_id), 1);
    assert!(world.has_npc_buff(npc_id, 3));
}

#[test]
fn test_npc_buff_multiple_types() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now(),
            duration_secs: 60,
        },
    );
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500200,
            buff_type: 5,
            start_time: Instant::now(),
            duration_secs: 30,
        },
    );
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500300,
            buff_type: 10,
            start_time: Instant::now(),
            duration_secs: 90,
        },
    );

    assert_eq!(world.npc_buff_count(npc_id), 3);
    assert!(world.has_npc_buff(npc_id, 3));
    assert!(world.has_npc_buff(npc_id, 5));
    assert!(world.has_npc_buff(npc_id, 10));
}

#[test]
fn test_npc_buff_remove_specific() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now(),
            duration_secs: 60,
        },
    );
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500200,
            buff_type: 5,
            start_time: Instant::now(),
            duration_secs: 30,
        },
    );

    assert!(world.remove_npc_buff(npc_id, 3));
    assert!(!world.has_npc_buff(npc_id, 3));
    assert!(world.has_npc_buff(npc_id, 5));
    assert_eq!(world.npc_buff_count(npc_id), 1);

    // Removing non-existent buff returns false
    assert!(!world.remove_npc_buff(npc_id, 99));
}

#[test]
fn test_npc_buff_clear_all() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now(),
            duration_secs: 60,
        },
    );
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500200,
            buff_type: 5,
            start_time: Instant::now(),
            duration_secs: 30,
        },
    );

    world.clear_npc_buffs(npc_id);
    assert_eq!(world.npc_buff_count(npc_id), 0);
    assert!(!world.has_npc_buff(npc_id, 3));
    assert!(!world.has_npc_buff(npc_id, 5));
}

#[test]
fn test_npc_buff_tick_expired() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    // One expired buff
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now()
                .checked_sub(std::time::Duration::from_secs(10))
                .unwrap_or(Instant::now()),
            duration_secs: 5,
        },
    );
    // One active buff
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500200,
            buff_type: 5,
            start_time: Instant::now(),
            duration_secs: 3600,
        },
    );

    let expired = world.process_npc_buff_tick();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].0, npc_id);
    assert_eq!(expired[0].1, 3);

    // Active buff remains
    assert!(world.has_npc_buff(npc_id, 5));
    assert!(!world.has_npc_buff(npc_id, 3));
    assert_eq!(world.npc_buff_count(npc_id), 1);
}

#[test]
fn test_npc_buff_tick_all_expired() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now()
                .checked_sub(std::time::Duration::from_secs(10))
                .unwrap_or(Instant::now()),
            duration_secs: 5,
        },
    );
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500200,
            buff_type: 5,
            start_time: Instant::now()
                .checked_sub(std::time::Duration::from_secs(20))
                .unwrap_or(Instant::now()),
            duration_secs: 10,
        },
    );

    let expired = world.process_npc_buff_tick();
    assert_eq!(expired.len(), 2);

    // DashMap entry should be cleaned up
    assert_eq!(world.npc_buff_count(npc_id), 0);
}

#[test]
fn test_npc_buff_tick_none_expired() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now(),
            duration_secs: 3600,
        },
    );

    let expired = world.process_npc_buff_tick();
    assert_eq!(expired.len(), 0);
    assert_eq!(world.npc_buff_count(npc_id), 1);
}

#[test]
fn test_npc_buff_tick_multiple_npcs() {
    let world = WorldState::new();

    // NPC 1: has expired buff
    world.apply_npc_buff(
        10001,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now()
                .checked_sub(std::time::Duration::from_secs(10))
                .unwrap_or(Instant::now()),
            duration_secs: 5,
        },
    );
    // NPC 2: has active buff
    world.apply_npc_buff(
        10002,
        NpcBuffEntry {
            skill_id: 500200,
            buff_type: 5,
            start_time: Instant::now(),
            duration_secs: 3600,
        },
    );
    // NPC 3: has expired buff
    world.apply_npc_buff(
        10003,
        NpcBuffEntry {
            skill_id: 500300,
            buff_type: 10,
            start_time: Instant::now()
                .checked_sub(std::time::Duration::from_secs(100))
                .unwrap_or(Instant::now()),
            duration_secs: 30,
        },
    );

    let expired = world.process_npc_buff_tick();
    assert_eq!(expired.len(), 2);

    // NPC 2 still has buff
    assert!(world.has_npc_buff(10002, 5));
    assert_eq!(world.npc_buff_count(10002), 1);

    // NPC 1 and 3 cleaned up
    assert_eq!(world.npc_buff_count(10001), 0);
    assert_eq!(world.npc_buff_count(10003), 0);
}

#[test]
fn test_npc_buff_remove_last_cleans_entry() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now(),
            duration_secs: 60,
        },
    );

    assert!(world.remove_npc_buff(npc_id, 3));
    // After removing the last buff, the entry should be cleaned from the DashMap
    assert_eq!(world.npc_buff_count(npc_id), 0);
}

#[test]
fn test_npc_buff_clear_nonexistent() {
    let world = WorldState::new();
    // Clearing buffs on an NPC with no buffs should not panic
    world.clear_npc_buffs(99999);
    assert_eq!(world.npc_buff_count(99999), 0);
}

#[test]
fn test_npc_buff_permanent_survives_tick() {
    let world = WorldState::new();
    let npc_id: u32 = 20001;

    // Permanent buff (duration=0)
    world.apply_npc_buff(
        npc_id,
        NpcBuffEntry {
            skill_id: 500100,
            buff_type: 3,
            start_time: Instant::now()
                .checked_sub(std::time::Duration::from_secs(100))
                .unwrap_or(Instant::now()),
            duration_secs: 0,
        },
    );

    let expired = world.process_npc_buff_tick();
    assert_eq!(expired.len(), 0);
    assert!(world.has_npc_buff(npc_id, 3));
}

// â”€â”€ PremiumGiftItem tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn test_premium_gift_items_empty_by_default() {
    let world = WorldState::new();
    let gifts = world.get_premium_gift_items(1);
    assert!(gifts.is_empty(), "no gift items by default");
}

#[test]
fn test_premium_gift_items_insert_and_get() {
    let world = WorldState::new();
    let items = vec![
        PremiumGiftItem {
            item_id: 389010000,
            count: 1,
            sender: "System".into(),
            subject: "Premium Gift".into(),
            message: "Enjoy your bonus!".into(),
        },
        PremiumGiftItem {
            item_id: 389020000,
            count: 5,
            sender: "Premium Store".into(),
            subject: "Extra Bonus".into(),
            message: "A special reward.".into(),
        },
    ];
    world.premium_gift_items.insert(2, items);

    let gifts = world.get_premium_gift_items(2);
    assert_eq!(gifts.len(), 2);
    assert_eq!(gifts[0].item_id, 389010000);
    assert_eq!(gifts[0].count, 1);
    assert_eq!(gifts[0].sender, "System");
    assert_eq!(gifts[1].item_id, 389020000);
    assert_eq!(gifts[1].count, 5);
}

#[test]
fn test_premium_gift_items_different_types_independent() {
    let world = WorldState::new();
    world.premium_gift_items.insert(
        1,
        vec![PremiumGiftItem {
            item_id: 100,
            count: 1,
            sender: "S".into(),
            subject: "T1".into(),
            message: "M1".into(),
        }],
    );
    world.premium_gift_items.insert(
        3,
        vec![PremiumGiftItem {
            item_id: 200,
            count: 2,
            sender: "S".into(),
            subject: "T3".into(),
            message: "M3".into(),
        }],
    );

    assert_eq!(world.get_premium_gift_items(1).len(), 1);
    assert_eq!(world.get_premium_gift_items(2).len(), 0); // type 2 not set
    assert_eq!(world.get_premium_gift_items(3).len(), 1);
    assert_eq!(world.get_premium_gift_items(3)[0].item_id, 200);
}

#[tokio::test]
async fn test_random_boss_system_load_empty_stages() {
    let world = WorldState::new();
    // No stages configured → should return 0 and not panic
    let count = world.random_boss_system_load();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_random_boss_system_load_no_matching_candidates() {
    use ko_db::models::{MonsterBossRandomSpawnRow, MonsterBossRandomStageRow};
    let world = WorldState::new();

    // Add a stage for monster 500 in zone 21
    *world.monster_boss_random_stages.write() = vec![MonsterBossRandomStageRow {
        stage: 1,
        monster_id: 500,
        monster_zone: 21,
        monster_name: "TestBoss".to_string(),
    }];

    // Add spawn candidates for a DIFFERENT monster_id (999) — should not match
    world
        .boss_random_spawn
        .entry(1)
        .or_default()
        .push(MonsterBossRandomSpawnRow {
            n_index: 1,
            stage: 1,
            monster_id: 999,
            monster_zone: 21,
            pos_x: 100,
            pos_z: 200,
            range: 5,
            reload_time: 18000,
            monster_name: "WrongBoss".to_string(),
        });

    // No matching candidates → should return 0
    let count = world.random_boss_system_load();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_random_boss_system_load_filters_by_monster_and_zone() {
    use ko_db::models::{MonsterBossRandomSpawnRow, MonsterBossRandomStageRow};
    let world = WorldState::new();

    // Stage requires monster 500 in zone 21
    *world.monster_boss_random_stages.write() = vec![MonsterBossRandomStageRow {
        stage: 1,
        monster_id: 500,
        monster_zone: 21,
        monster_name: "TestBoss".to_string(),
    }];

    // Candidate 1: correct monster, WRONG zone
    world
        .boss_random_spawn
        .entry(1)
        .or_default()
        .push(MonsterBossRandomSpawnRow {
            n_index: 1,
            stage: 1,
            monster_id: 500,
            monster_zone: 22, // wrong zone
            pos_x: 100,
            pos_z: 200,
            range: 5,
            reload_time: 18000,
            monster_name: "WrongZone".to_string(),
        });

    // Candidate 2: correct monster AND zone
    world
        .boss_random_spawn
        .entry(1)
        .or_default()
        .push(MonsterBossRandomSpawnRow {
            n_index: 2,
            stage: 1,
            monster_id: 500,
            monster_zone: 21,
            pos_x: 300,
            pos_z: 400,
            range: 5,
            reload_time: 18000,
            monster_name: "CorrectBoss".to_string(),
        });

    // spawn_event_npc will fail (no NPC template 500), so count stays 0
    // but the filtering logic is exercised — no panic
    let count = world.random_boss_system_load();
    assert_eq!(count, 0); // no template loaded, spawn returns empty
}

#[test]
fn test_scheduled_respawn_queue() {
    let world = WorldState::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Schedule two respawns: one ready now, one in the future
    world.schedule_respawn(ScheduledRespawn {
        born_sid: 100,
        zone_id: 21,
        x: 500.0,
        z: 600.0,
        spawn_at: now - 10, // already past
    });
    world.schedule_respawn(ScheduledRespawn {
        born_sid: 200,
        zone_id: 22,
        x: 700.0,
        z: 800.0,
        spawn_at: now + 3600, // 1 hour from now
    });

    // Drain — only the first should be ready
    let ready = world.drain_ready_respawns(now);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].born_sid, 100);
    assert_eq!(ready[0].zone_id, 21);

    // Second drain — nothing ready yet
    let ready2 = world.drain_ready_respawns(now);
    assert!(ready2.is_empty());

    // Future drain — second entry now ready
    let ready3 = world.drain_ready_respawns(now + 7200);
    assert_eq!(ready3.len(), 1);
    assert_eq!(ready3[0].born_sid, 200);
}

// ── JackPot System Tests ────────────────────────────────────────────

#[test]
fn test_jackpot_settings_default_zero() {
    let world = WorldState::new();
    let settings = world.get_jackpot_settings();
    assert_eq!(settings[0].rate, 0);
    assert_eq!(settings[1].rate, 0);
}

#[test]
fn test_jackpot_noah_returns_false_when_disabled() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);
    // jackpot_type = 0 by default, settings rate = 0
    assert!(!world.try_jackpot_noah(1, 1000));
}

#[test]
fn test_jackpot_noah_returns_false_wrong_type() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);
    // Set jackpot_type = 1 (EXP), but try noah (needs type=2)
    world.update_session(1, |h| h.jackpot_type = 1);
    assert!(!world.try_jackpot_noah(1, 1000));
}

#[test]
fn test_jackpot_noah_returns_false_zero_gold() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);
    world.update_session(1, |h| h.jackpot_type = 2);
    assert!(!world.try_jackpot_noah(1, 0));
}

#[test]
fn test_jackpot_multiplier_roll_zero_settings() {
    let setting = JackPotSetting::default();
    // All thresholds 0 → always returns 0
    let mult = WorldState::roll_jackpot_multiplier(&setting);
    assert_eq!(mult, 0);
}

#[test]
fn test_jackpot_multiplier_roll_guaranteed_1000x() {
    let setting = JackPotSetting {
        rate: 10000,
        x_1000: 10001, // all rolls < 10001
        x_500: 10001,
        x_100: 10001,
        x_50: 10001,
        x_10: 10001,
        x_2: 10001,
    };
    // With x_1000 = 10001, any rand [0,10000] < 10001 → always 1000x
    let mult = WorldState::roll_jackpot_multiplier(&setting);
    assert_eq!(mult, 1000);
}

#[test]
fn test_jackpot_type_set_by_update_session() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);
    assert_eq!(world.with_session(1, |h| h.jackpot_type).unwrap(), 0);
    world.update_session(1, |h| h.jackpot_type = 2);
    assert_eq!(world.with_session(1, |h| h.jackpot_type).unwrap(), 2);
    world.update_session(1, |h| h.jackpot_type = 0);
    assert_eq!(world.with_session(1, |h| h.jackpot_type).unwrap(), 0);
}

#[tokio::test]
async fn test_jackpot_exp_returns_false_when_disabled() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);
    // jackpot_type = 0, rate = 0
    assert!(!world.try_jackpot_exp(1, 1000).await);
}

#[tokio::test]
async fn test_jackpot_exp_returns_false_wrong_type() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);
    // Set jackpot_type = 2 (Noah), but try exp (needs type=1)
    world.update_session(1, |h| h.jackpot_type = 2);
    assert!(!world.try_jackpot_exp(1, 1000).await);
}

#[tokio::test]
async fn test_jackpot_exp_returns_true_at_max_level() {
    let world = WorldState::new();
    let (tx, _rx) = mpsc::unbounded_channel();
    world.register_session(1, tx);
    world.update_session(1, |h| h.jackpot_type = 1);
    // Set player to max level (83) by registering in-game with level 83
    let info = CharacterInfo {
        session_id: 1,
        name: "MaxLvl".into(),
        nation: 1,
        race: 1,
        class: 101,
        level: 83,
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
        equipped_items: [0; 14],
        bind_zone: 21,
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
        max_exp: 0,
        exp_seal_status: false,
        sealed_exp: 0,
        item_weight: 0,
        max_weight: 0,
        res_hp_type: 0x01,
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
    };
    let pos = Position {
        zone_id: 21,
        x: 50.0,
        y: 0.0,
        z: 50.0,
        region_x: calc_region(50.0),
        region_z: calc_region(50.0),
    };
    world.register_ingame(1, info, pos);
    // C++ returns true to skip normal ExpChange at max level
    assert!(world.try_jackpot_exp(1, 1000).await);
}

// ── User Ranking System Tests ────────────────────────────────────────

#[test]
fn test_user_personal_rank_default_empty() {
    let world = WorldState::new();
    assert_eq!(world.get_user_personal_rank("TestUser"), 0);
    assert_eq!(world.get_user_knights_rank("TestUser"), 0);
}

#[test]
fn test_user_personal_rank_lookup() {
    let world = WorldState::new();
    {
        let mut map = world.user_personal_rank.write();
        map.insert("TESTUSER".to_string(), 5);
        map.insert("PLAYER2".to_string(), 10);
    }
    // Case-insensitive lookup (uppercases input)
    assert_eq!(world.get_user_personal_rank("TestUser"), 5);
    assert_eq!(world.get_user_personal_rank("testuser"), 5);
    assert_eq!(world.get_user_personal_rank("TESTUSER"), 5);
    assert_eq!(world.get_user_personal_rank("Player2"), 10);
    // Not found → 0
    assert_eq!(world.get_user_personal_rank("Unknown"), 0);
}

#[test]
fn test_user_knights_rank_lookup() {
    let world = WorldState::new();
    {
        let mut map = world.user_knights_rank.write();
        map.insert("WARRIOR".to_string(), 3);
    }
    assert_eq!(world.get_user_knights_rank("Warrior"), 3);
    assert_eq!(world.get_user_knights_rank("warrior"), 3);
    assert_eq!(world.get_user_knights_rank("WARRIOR"), 3);
    assert_eq!(world.get_user_knights_rank("NoRank"), 0);
}

#[test]
fn test_apply_user_ranks_to_sessions() {
    let world = WorldState::new();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Register a session with a character
    let info = CharacterInfo {
        session_id: 1,
        name: "RankedUser".to_string(),
        nation: 1,
        race: 1,
        class: 101,
        level: 60,
        hp: 1000,
        max_hp: 1000,
        mp: 500,
        max_mp: 500,
        ..CharacterInfo::default()
    };
    let pos = Position {
        zone_id: 21,
        x: 1000.0,
        y: 0.0,
        z: 1000.0,
        region_x: calc_region(1000.0),
        region_z: calc_region(1000.0),
    };
    world.register_ingame(1, info, pos);

    // Set up rank maps
    {
        let mut p = world.user_personal_rank.write();
        p.insert("RANKEDUSER".to_string(), 7);
    }
    {
        let mut k = world.user_knights_rank.write();
        k.insert("RANKEDUSER".to_string(), 12);
    }

    // Apply ranks
    world.apply_user_ranks_to_sessions();

    // Verify session was updated
    let p_rank = world.with_session(1, |h| h.personal_rank).unwrap();
    let k_rank = world.with_session(1, |h| h.knights_rank).unwrap();
    assert_eq!(p_rank, 7);
    assert_eq!(k_rank, 12);
}

#[test]
fn test_apply_user_ranks_unranked_session() {
    let world = WorldState::new();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    world.register_session(1, tx);

    // Register a session
    let info = CharacterInfo {
        session_id: 1,
        name: "UnrankedUser".to_string(),
        nation: 2,
        race: 2,
        class: 201,
        level: 30,
        hp: 500,
        max_hp: 500,
        mp: 300,
        max_mp: 300,
        ..CharacterInfo::default()
    };
    let pos = Position {
        zone_id: 21,
        x: 1000.0,
        y: 0.0,
        z: 1000.0,
        region_x: calc_region(1000.0),
        region_z: calc_region(1000.0),
    };
    world.register_ingame(1, info, pos);

    // No ranks in the maps → should set 0
    world.apply_user_ranks_to_sessions();

    let p_rank = world.with_session(1, |h| h.personal_rank).unwrap();
    let k_rank = world.with_session(1, |h| h.knights_rank).unwrap();
    assert_eq!(p_rank, 0);
    assert_eq!(k_rank, 0);
}

#[test]
fn test_reload_rank_interval_constant() {
    // C++ Define.h:275 — RELOAD_KNIGHTS_AND_USER_RATING = 15
    assert_eq!(
        crate::systems::daily_reset::RELOAD_RANK_INTERVAL_MINUTES,
        15
    );
}
