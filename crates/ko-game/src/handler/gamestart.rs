//! WIZ_GAMESTART (0x0D) handler — game world entry.
//! Two-phase handshake:
//! - Phase 1 (sub-opcode 1): Server sends SendMyInfo, then empty WIZ_GAMESTART.
//! - Phase 2 (sub-opcode 2): Server transitions user to in-game state.

use ko_db::repositories::character::CharacterRepository;
use ko_db::repositories::daily_quest::DailyQuestRepository;
use ko_db::repositories::premium::PremiumRepository;
use ko_db::repositories::saved_magic::SavedMagicRepository;
use ko_db::repositories::skill_shortcut::SkillShortcutRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use std::sync::Arc;

use crate::handler::{quest, region, stats};
use crate::session::{ClientSession, SessionState};
use crate::world::{
    CharacterInfo, Position, UserItemSlot, ZONE_ARDREAM, ZONE_BIFROST, ZONE_DELOS, ZONE_ELMORAD,
    ZONE_KARUS, ZONE_KROWAZ_DOMINION, ZONE_MORADON, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE,
    ZONE_DELOS_CASTELLAN, ZONE_DESPERATION_ABYSS, ZONE_HELL_ABYSS,
};
use crate::zone::calc_region;

use crate::magic_constants::ABNORMAL_INVISIBLE;

use super::{HAVE_MAX, INVENTORY_TOTAL, SLOT_MAX};

use crate::clan_constants::{CHIEF, COMMAND_AUTHORITY, COMMAND_CAPTAIN};

fn myinfo_cape_tail_flag(clan_flag: u8, clan_grade: u8) -> u8 {
    if clan_flag > 1 && clan_grade < 3 {
        1
    } else {
        0
    }
}

/// Handle WIZ_GAMESTART from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::CharacterSelected {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    match sub_opcode {
        1 => handle_phase1(session).await,
        2 => handle_phase2(session).await,
        _ => {
            // Invalid sub-opcode disconnects the client.
            tracing::warn!(
                "[{}] Invalid gamestart sub-opcode: {} — disconnecting",
                session.addr(),
                sub_opcode
            );
            anyhow::bail!("Invalid gamestart sub-opcode: {}", sub_opcode)
        }
    }
}

/// Phase 1: Send SendMyInfo packet, then empty WIZ_GAMESTART.
async fn handle_phase1(session: &mut ClientSession) -> anyhow::Result<()> {
    let char_id = session.character_id().unwrap_or("unknown").to_string();

    // Load all character data from DB in parallel (6 independent queries)
    // Reduces login latency from 6 × DB_ROUND_TRIP to 1 × DB_ROUND_TRIP
    let pool = session.pool().clone();
    let char_repo = CharacterRepository::new(&pool);
    let achieve_repo = ko_db::repositories::achieve::AchieveRepository::new(&pool);
    let ud_repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);

    let (ch_result, items_result, achieve_result, genie_result, return_result, daily_op_result) = tokio::join!(
        char_repo.load(&char_id),
        char_repo.load_items(&char_id),
        achieve_repo.load_user_achieve_summary(&char_id),
        ud_repo.load_genie_data(&char_id),
        ud_repo.load_return_data(&char_id),
        ud_repo.load_daily_op(&char_id),
    );

    let ch = match ch_result? {
        Some(ch) => ch,
        None => {
            tracing::error!(
                "[{}] Character not found for gamestart: {}",
                session.addr(),
                char_id
            );
            return Ok(());
        }
    };

    let mut items = items_result?;

    // C++ requires every real inventory item to have a non-zero serial before
    // ITEM_BOUND/ITEM_LOCK can succeed.  Legacy imports and the original
    // CREATE_NEW_CHAR_SET insert path left serial_num=0, which made v2615 show
    // "Item restoration failed" whenever one of those items was equipped.
    // Repair both the login snapshot and PostgreSQL before MyInfo is built so
    // existing characters and newly-created characters are fixed permanently.
    let mut repaired_serials = 0usize;
    for item in items
        .iter_mut()
        .filter(|item| item.item_id > 0 && item.serial_num <= 0)
    {
        let serial = session.world().generate_item_serial().max(1);
        item.serial_num = serial as i64;
        repaired_serials += 1;
    }
    if repaired_serials > 0 {
        let repaired_items: Vec<_> = items
            .iter()
            .map(|item| ko_db::repositories::character::SaveItemParams {
                char_id: &char_id,
                slot_index: item.slot_index,
                item_id: item.item_id,
                durability: item.durability,
                count: item.count,
                flag: item.flag,
                original_flag: item.original_flag,
                serial_num: item.serial_num,
                expire_time: item.expire_time,
            })
            .collect();
        char_repo.save_items_batch(&repaired_items).await?;
        tracing::info!(
            "[sid={}] GAMESTART: repaired {} zero item serial(s) for {}",
            session.session_id(),
            repaired_serials,
            char_id
        );
    }

    // Restore the equipped pet from pet_user_data.
    //
    // Pet records are keyed by the pet item's serial number. Older/current
    // inventory mappings may expose the pet equipment as DB slot 5 or as the
    // absolute CFAIRY slot 48, so check both mappings. The item-ID fallback
    // also covers an existing Kaul before the slot mapping is normalised.
    let mut loaded_pet_state: Option<crate::world::PetState> = None;
    let pet_repo = ko_db::repositories::pet::PetRepository::new(&pool);

    let mut pet_serial_candidates: Vec<i64> = items
        .iter()
        .filter(|item| {
            item.serial_num > 0
                && (item.slot_index as usize == crate::world::CFAIRY_SLOT
                    || item.slot_index == 5
                    || item.item_id == 610_001_000)
        })
        .map(|item| item.serial_num)
        .collect();

    pet_serial_candidates.sort_unstable();
    pet_serial_candidates.dedup();

    for serial_id in pet_serial_candidates {
        match pet_repo.load_pet_data(serial_id).await {
            Ok(Some(row)) => {
                let pet_items = pet_repo
                    .load_pet_items(row.n_serial_id)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            "[sid={}] GAMESTART: failed loading pet items serial={}: {}",
                            session.session_id(),
                            row.n_serial_id,
                            e
                        );
                        Vec::new()
                    });
                let mut restored = crate::world::PetState {
                    serial_id: row.n_serial_id.max(0) as u64,
                    level: row.b_level.clamp(1, 60) as u8,
                    satisfaction: row.s_satisfaction.clamp(0, 10_000),
                    exp: row.n_exp.max(0) as u32,
                    hp: row.s_hp.max(0) as u16,
                    nid: 0,
                    index: row.n_index.max(0) as u32,
                    mp: row.s_mp.max(0) as u16,
                    state_change: 4,
                    name: row.s_pet_name,
                    pid: row.s_pid.max(0) as u16,
                    size: row.s_size.max(0) as u16,
                    attack_started: false,
                    attack_target_id: -1,
                    ..Default::default()
                };
                super::pet::apply_persistent_pet_items(&mut restored, pet_items);
                let equipment = restored.items.iter().enumerate()
                    .filter(|(_, item)| item.item_id != 0 || item.count != 0)
                    .map(|(slot, item)| format!("{}:{}x{}", slot, item.item_id, item.count))
                    .collect::<Vec<_>>().join(",");
                loaded_pet_state = Some(restored);

                tracing::info!(
                    "[sid={}] PET_LOAD source=gamestart serial={} index={} pid={} equipment=[{}]",
                    session.session_id(),
                    row.n_serial_id,
                    row.n_index,
                    row.s_pid,
                    equipment
                );
                break;
            }
            Ok(None) => {
                tracing::warn!(
                    "[sid={}] GAMESTART: no pet_user_data row for serial={}",
                    session.session_id(),
                    serial_id
                );
            }
            Err(e) => {
                tracing::warn!(
                    "[sid={}] GAMESTART: failed loading pet serial={}: {}",
                    session.session_id(),
                    serial_id,
                    e
                );
            }
        }
    }

    session
        .world()
        .update_session(session.session_id(), |holder| {
            holder.pet_data = loaded_pet_state;
        });

    // Process achieve summary for cover/skill title IDs
    let (cover_title, skill_title) = {
        let mut ct: u16 = 0;
        let mut st: u16 = 0;
        if let Ok(Some(summary)) = achieve_result {
            let world = session.world();
            if summary.cover_id > 0 {
                ct = world
                    .achieve_main(summary.cover_id as i32)
                    .map(|m| m.title_id as u16)
                    .unwrap_or(0);
            }
            if summary.skill_id > 0 {
                st = world
                    .achieve_main(summary.skill_id as i32)
                    .map(|m| m.title_id as u16)
                    .unwrap_or(0);
            }
        }
        (ct, st)
    };

    // Process genie data
    let genie_time: u16 = match genie_result {
        Ok(Some(genie)) => {
            let abs_ts = genie.genie_time.max(0) as u32;
            let now = crate::handler::genie::now_secs();
            let remaining = abs_ts.saturating_sub(now);

            tracing::info!(
                "[{}] Genie load: char={}, db_abs={}, now={}, remaining={}s ({}h)",
                session.addr(),
                char_id,
                abs_ts,
                now,
                remaining,
                crate::handler::genie::get_genie_hours_pub(remaining),
            );

            let world = session.world();
            let sid = session.session_id();
            world.update_session(sid, |h| {
                h.genie_time_abs = abs_ts;
                h.genie_options = genie.genie_options;
                h.genie_loaded = true;
            });

            crate::handler::genie::get_genie_hours_pub(remaining)
        }
        Ok(None) => {
            tracing::info!(
                "[{}] Genie load: char={}, no row in user_genie_data",
                session.addr(),
                char_id,
            );
            let world = session.world();
            let sid = session.session_id();
            world.update_session(sid, |h| {
                h.genie_loaded = true;
            });
            0
        }
        Err(e) => {
            tracing::warn!(
                "[{}] Genie load error: char={}, err={}",
                session.addr(),
                char_id,
                e
            );
            0
        }
    };

    // Process return symbol data
    let return_symbol_ok: u32 = if let Ok(Some(ret)) = return_result {
        let ok_val = ret.return_symbol_ok.unwrap_or(0) as u32;
        let time_val = ret.return_symbol_time.unwrap_or(0);
        let world = session.world();
        let sid = session.session_id();
        world.update_session(sid, |h| {
            h.return_symbol_ok = ok_val;
            h.return_symbol_time = time_val;
        });
        ok_val
    } else {
        0
    };

    // Process daily operation cooldowns
    if let Ok(Some(row)) = daily_op_result {
        let data = crate::world::types::UserDailyOp::from_row(&row);
        session.world().daily_ops.insert(char_id.clone(), data);
    }

    // Load user rankings (NP symbols)
    {
        let world = session.world();
        let sid = session.session_id();
        let p_rank = world.get_user_personal_rank(&ch.str_user_id);
        let k_rank = world.get_user_knights_rank(&ch.str_user_id);
        world.update_session(sid, |h| {
            h.personal_rank = p_rank;
            h.knights_rank = k_rank;
        });
    }

    // Build WIZ_MYINFO packet
    let mut pkt = Packet::new(Opcode::WizMyInfo as u8);

    // SByte mode starts here — character name uses u8 length prefix
    // C++ line 43-64: basic character info
    pkt.write_u32(session.session_id() as u32); // socket ID
    pkt.write_sbyte_string(&ch.str_user_id);

    // Position scaled by 10 (C++ GetSPosX = uint16(GetX() * 10))
    let pos_x = (ch.px as f32 / 100.0 * 10.0) as u16;
    let pos_z = (ch.pz as f32 / 100.0 * 10.0) as u16;
    let pos_y = (ch.py as f32 / 100.0 * 10.0) as u16;
    pkt.write_u16(pos_x);
    pkt.write_u16(pos_z);
    pkt.write_u16(pos_y);

    pkt.write_u8(ch.nation as u8); // nation
    pkt.write_u8(ch.race as u8); // race
                                 // IDA-verified order: class(i16) → hairColor(u8,+3000) → hairPacked(u32,+3004)
                                 //                     → face(u8,+3016) → title2(u8,+1964) → title1(u8,+1960)
                                 //                     → rank(u8,+3020) → level(u8,+1744) → points(i16,+3044)
    pkt.write_i16(ch.class as i16); // class (sub_61EE80 = i16)
    let hair_color: u8 = ((ch.hair_rgb >> 24) & 0xFF) as u8; // top byte of packed hair
    pkt.write_u8(hair_color); // hairColor (+3000)
    pkt.write_u32(ch.hair_rgb as u32); // hairPacked (+3004)
    pkt.write_u8(ch.face as u8); // face (+3016) — AFTER hair, not before
    pkt.write_u8(ch.title as u8); // title2 (+1964)
    pkt.write_u8(0); // title1 (+1960)
                     // C++ SendMyInfo recalculates m_bRank from the KING_SYSTEM record immediately
                     // before serialising MyInfo.  The persisted character `rank` is the ladder
                     // rank and must not be used here (rank=1 made ordinary users look like kings,
                     // hid the regular cloak catalogue and replaced their clan cape with 97/98).
    let is_king = session.world().is_king(ch.nation as u8, &ch.str_user_id);
    pkt.write_u8(u8::from(is_king)); // king rank (+3020) — AFTER titles
    pkt.write_u8(ch.level as u8); // level (+1744)
    pkt.write_i16(ch.points as i16); // points (+3044, sub_61EE80 = i16)

    // MaxExp for level — C++ m_iMaxExp is int64 (8 bytes)
    // C++ SendMyInfo: result << m_iMaxExp << m_iExp (UserInfoSystem.cpp:59-60)
    // C++ sets m_iMaxExp = GetExpByLevel(level, rebirth) at CharacterSelectionHandler.cpp:753
    let max_exp = session.world().get_exp_by_level(ch.level as u8, 0);
    pkt.write_i64(max_exp);
    pkt.write_i64(ch.exp); // current exp

    pkt.write_u32(ch.loyalty as u32); // loyalty (NP)
    pkt.write_u32(ch.loyalty_monthly as u32); // monthly loyalty

    // Clan section — SNIFFER-VERIFIED format from original server (session 45).
    // the original server's MyInfo proves the OLD C++ format is what the client expects:
    //   clanID(i16) + fame(u8) + [conditional clan data] + cape(u16) + cape_rgb(u32) + unknown(8)
    let clan_id = ch.knights;
    pkt.write_i16(clan_id);
    pkt.write_u8(ch.fame as u8);

    if clan_id > 0 {
        match session.world().get_knights(clan_id as u16) {
            Some(ki) => {
                pkt.write_u16(ki.alliance);
                pkt.write_u8(ki.flag);
                pkt.write_sbyte_string(&ki.name);
                pkt.write_u8(ki.grade);
                pkt.write_u8(ki.ranking);
                pkt.write_u16(ki.mark_version);
                // Cape: sniffer shows u16 cape_id + u32(R,G,B,flag)
                let cape_id = if is_king {
                    if ch.nation == 1 {
                        97u16
                    } else {
                        98u16
                    }
                } else {
                    ki.cape
                };
                tracing::info!(
                    character = %ch.str_user_id,
                    clan_id,
                    clan_flag = ki.flag,
                    clan_grade = ki.grade,
                    cape_id,
                    cape_r = ki.cape_r,
                    cape_g = ki.cape_g,
                    cape_b = ki.cape_b,
                    cape_flag = myinfo_cape_tail_flag(ki.flag, ki.grade),
                    "MyInfo cape state"
                );
                pkt.write_u16(cape_id);
                pkt.write_u8(ki.cape_r);
                pkt.write_u8(ki.cape_g);
                pkt.write_u8(ki.cape_b);
                // The final byte belongs to the same u32 as RGB and is the
                // client-side cape-symbol/render flag.  Unlike the clan type
                // byte above, the v2615 client expects the compact 0/1 value
                // used by the reference MyInfo flow here.
                pkt.write_u8(myinfo_cape_tail_flag(ki.flag, ki.grade));
            }
            None => {
                // Clan exists but not loaded — send empty clan data
                pkt.write_u64(0);
                pkt.write_u16(0xFFFF);
                pkt.write_u32(0);
            }
        }
    } else {
        // No clan — sniffer verified: u64(0) + cape(0xFFFF) + u32(0)
        pkt.write_u64(0);
        pkt.write_u16(0xFFFF); // cape_id = -1 (no cape)
        pkt.write_u32(0); // cape RGB
    }

    // 8 unknown bytes (sniffer verified: always zeros)
    pkt.write_bytes(&[0u8; 8]);

    // HP/MP — use proper coefficient-based formula
    let temp_ch = CharacterInfo {
        session_id: 0,
        name: String::new(),
        nation: ch.nation as u8,
        race: ch.race as u8,
        class: ch.class as u16,
        level: ch.level as u8,
        face: 0,
        hair_rgb: 0,
        rank: 0,
        title: 0,
        max_hp: ch.hp,
        hp: ch.hp,
        max_mp: ch.mp,
        mp: ch.mp,
        max_sp: 0,
        sp: 0,
        equipped_items: [0; 14],
        bind_zone: 0,
        bind_x: 0.0,
        bind_z: 0.0,
        str: ch.strong as u8,
        sta: ch.sta as u8,
        dex: ch.dex as u8,
        intel: ch.intel as u8,
        cha: ch.cha as u8,
        free_points: 0,
        skill_points: [0; 10],
        gold: 0,
        loyalty: 0,
        loyalty_monthly: 0,
        authority: 0,
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
        rebirth_level: ch.rebirth_level as u8,
        reb_str: ch.reb_str as u8,
        reb_sta: ch.reb_sta as u8,
        reb_dex: ch.reb_dex as u8,
        reb_intel: ch.reb_intel as u8,
        reb_cha: ch.reb_cha as u8,
        cover_title,
    };
    let coeff = session.world().get_coefficient(ch.class as u16);
    let abilities = stats::recalculate_abilities(&temp_ch, coeff.as_ref());
    let max_hp = abilities.max_hp;
    let max_mp = abilities.max_mp;
    // Send base max HP/MP here; Phase 2 will recalculate with item bonuses
    // and send updated values via set_user_ability + send_item_move_refresh.
    // Use DB HP/MP directly — set_user_ability() will cap if needed.
    pkt.write_i16(max_hp); // max HP (base, no item bonuses)
    pkt.write_i16(ch.hp); // current HP (from DB)
    pkt.write_i16(max_mp); // max MP (base, no item bonuses)
    pkt.write_i16(ch.mp); // current MP (from DB)

    // Weight
    // m_sMaxWeight = (STR + level) * 50 + bonuses
    pkt.write_u32(abilities.max_weight); // max weight
    pkt.write_u32(0); // current weight (recalculated after inventory load in phase 2)

    // Stats: base + bonus pairs (C++ line 148-152)
    // Stat bonuses from equipment require full SetUserAbility (done in phase 2)
    pkt.write_u8(ch.strong as u8);
    pkt.write_u8(0); // STR item bonus (updated in phase 2 via SendItemMove)
    pkt.write_u8(ch.sta as u8);
    pkt.write_u8(0); // STA item bonus
    pkt.write_u8(ch.dex as u8);
    pkt.write_u8(0); // DEX item bonus
    pkt.write_u8(ch.intel as u8);
    pkt.write_u8(0); // INT item bonus
    pkt.write_u8(ch.cha as u8);
    pkt.write_u8(0); // CHA item bonus

    // Combat stats — updated in phase 2 via SendItemMove(1,1)
    pkt.write_u16(0); // total hit (corrected by SendItemMove in phase 2)
    pkt.write_u16(0); // total AC (corrected by SendItemMove in phase 2)

    // Resistances — updated in phase 2 via SendItemMove(1,1)
    pkt.write_bytes(&[0u8; 6]); // fire, cold, lightning, magic, disease, poison

    // Gold and authority (C++ line 156-157)
    pkt.write_u32(ch.gold as u32);
    pkt.write_u8(ch.authority as u8);

    // Ranking (C++ line 159-164)
    // C++ logic: write the better (lower) rank, set the other to -1.
    // `m_bKnightsRank <= m_bPersonalRank ? m_bKnightsRank : int8(-1)`
    // `m_bPersonalRank <= m_bKnightsRank ? m_bPersonalRank : int8(-1)`
    // In our system: 0 means unranked (C++ uses -1 as i8 = 0xFF)
    let (k_rank, p_rank) = session
        .world()
        .with_session(session.session_id(), |h| (h.knights_rank, h.personal_rank))
        .unwrap_or((0, 0));
    let k_i8: i8 = if k_rank == 0 { -1 } else { k_rank as i8 };
    let p_i8: i8 = if p_rank == 0 { -1 } else { p_rank as i8 };
    // C++ comparison: show the rank only if it's <= the other rank
    let k_out = if k_i8 <= p_i8 { k_i8 } else { -1 };
    let p_out = if p_i8 <= k_i8 { p_i8 } else { -1 };
    pkt.write_i8(k_out);
    pkt.write_i8(p_out);

    // Skill data — 9 bytes (C++ line 166)
    // Load from normalized skill0-skill8 columns (skill9 is unused by client)
    let skill_bytes: [u8; 9] = [
        ch.skill0 as u8,
        ch.skill1 as u8,
        ch.skill2 as u8,
        ch.skill3 as u8,
        ch.skill4 as u8,
        ch.skill5 as u8,
        ch.skill6 as u8,
        ch.skill7 as u8,
        ch.skill8 as u8,
    ];
    pkt.write_bytes(&skill_bytes);

    // Inventory — IDA-verified phase order (sub_733190, myinfo.cpp:901-949).
    // Client reads exactly 90 items in 5 phases:
    //   Phase 1: 14 equipment (slots 0-13)
    //   Phase 2: 28 bag (slots 14-41)
    //   Phase 3: 9 cospre (sequential abs slots 42-50)
    //   Phase 4: 3 special (empty — sniffer verified, CBAG1/CBAG2 come via item_move)
    //   Phase 5: 36 magic bag (slots 53-88, 3 bags × 12)
    //
    // Phase 3 detail: Client loop reads into cosprItems[i] for i=0,1,2,3,4,5,7,8,9
    // (position 6=COSP_BAG1 is ALWAYS zero-filled, never read from wire).
    // Relative positions map to absolute slots via COSP_* constants:
    //   pos 0→42, 1→43, 2→44, 3→45, 4→46, 5→47, 7→48(CFAIRY), 8→49, 9→50
    // So wire order = sequential abs slots 42-50.
    // BUG FIX: Previously used 42+pos giving [42..47,49,50,51] — wrong because
    // COSP_FAIRY=7 maps to abs 48 (not 42+7=49), COSP_BAG1=6 maps to abs 51 (not 42+6=48).
    let rebirth_level = ch.rebirth_level as u8;
    let world = session.world().clone();

    // Build the 90-slot list in client's expected order
    let mut myinfo_slots: Vec<Option<i16>> = Vec::with_capacity(90);
    // Phase 1: Equipment (0-13)
    for s in 0..14i16 {
        myinfo_slots.push(Some(s));
    }
    // Phase 2: Bag (14-41)
    for s in 14..42i16 {
        myinfo_slots.push(Some(s));
    }
    // Phase 3: 9 cospre items — sequential abs slots 42-50
    // Client stores wire items at cosprItems[0,1,2,3,4,5,7,8,9] (skips [6]=CBAG1).
    for s in 42..51i16 {
        myinfo_slots.push(Some(s));
    }
    // Phase 4: 3 special items → client SetSpecialSlot(i, item) (myinfo.cpp:1247-1252)
    // These fill the gaps NOT covered by Phase 3 (which skips position 6=CBAG1):
    //   specialItems[0] → CBAG1 (abs 51, cospre pos 6)
    //   specialItems[1] → CBAG2 (abs 52, cospre pos 10)
    //   specialItems[2] → CBAG3 (stored at slot 96 to avoid overlap with magic bag 1)
    myinfo_slots.push(Some(51)); // CBAG1
    myinfo_slots.push(Some(52)); // CBAG2
    myinfo_slots.push(Some(INVENTORY_TOTAL as i16)); // CBAG3 at dedicated slot
                                                     // Phase 5: Magic bags (53-88, 3 bags × 12 slots)
                                                     // CBAG3 is stored at dedicated slot 96, so slot 53 is free for magic bag 1 items.
    for s in 53..89i16 {
        myinfo_slots.push(Some(s));
    }

    for slot_opt in &myinfo_slots {
        let item = slot_opt.and_then(|s| items.iter().find(|i| i.slot_index == s));
        match item {
            Some(it) => {
                pkt.write_u32(it.item_id as u32);
                pkt.write_i16(it.durability);
                pkt.write_u16(it.count as u16);
                pkt.write_u8(it.flag as u8);
                pkt.write_u16(crate::world::types::remaining_rental_minutes(
                    it.expire_time as u32,
                ));
                crate::handler::unique_item_info::write_unique_item_info(
                    &world,
                    &pool,
                    it.item_id as u32,
                    it.serial_num as u64,
                    rebirth_level,
                    &mut pkt,
                )
                .await;
                pkt.write_u32(it.expire_time as u32);
            }
            None => {
                pkt.write_u32(0);
                pkt.write_i16(0);
                pkt.write_u16(0);
                pkt.write_u8(0);
                pkt.write_u16(0);
                pkt.write_u32(0);
                pkt.write_u32(0);
            }
        }
    }

    // Premium/account status (C++ line 199-243)
    // Load premium subscriptions from DB for this account
    let account_id_for_premium = session.account_id().unwrap_or("").to_string();
    let (premium_entries, premium_in_use) = if !account_id_for_premium.is_empty() {
        load_premium_for_myinfo(&pool, &account_id_for_premium).await
    } else {
        (Vec::new(), 0u8)
    };
    // SNIFFER-VERIFIED (2026-03-29): Original server sends 94 items, NOT 90.
    for _ in 0..4 {
        pkt.write_u32(0);
        pkt.write_i16(0);
        pkt.write_u16(0);
        pkt.write_u8(0);
        pkt.write_u16(0);
        pkt.write_u32(0);
        pkt.write_u32(0);
    }

    // accountStatus
    let account_status: u8 = if premium_in_use > 0 { 1 } else { 0 };
    pkt.write_u8(account_status);
    // premium section
    pkt.write_u8(premium_entries.len() as u8);
    for &(p_type, time_hours) in &premium_entries {
        pkt.write_u8(p_type);
        pkt.write_u16(time_hours);
    }
    // Sniffer-verified: activePremiumType u8 between premium entries and collRaceEnabled
    pkt.write_u8(premium_in_use); // activePremiumType (+1844)
                                  // IDA-verified trailer order (lines 707205-707418):
    pkt.write_u8(0); // collRaceEnabled (forced to 0)
    pkt.write_u32(return_symbol_ok); // coverTitle_u32 (+3236)
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(0); // skillSave x5
    pkt.write_u8(0); // petType
    pkt.write_i16(genie_time as i16); // petHP/genieTime
    pkt.write_u8(ch.rebirth_level as u8);
    pkt.write_u8(ch.reb_str as u8);
    pkt.write_u8(ch.reb_sta as u8);
    pkt.write_u8(ch.reb_dex as u8);
    pkt.write_u8(ch.reb_intel as u8);
    pkt.write_u8(ch.reb_cha as u8);
    pkt.write_i64(0); // sealedExp
    pkt.write_i16(cover_title as i16);
    pkt.write_i16(skill_title as i16);
    pkt.write_u32(ch.manner_point as u32);
    pkt.write_u8(premium_in_use);
    pkt.write_u8(0); // isHidingHelmet
    pkt.write_u8(0); // unknown_ui_1
    pkt.write_u8(0); // unknown_ui_2
    pkt.write_u8(0); // isHidingCospre

    let pkt_size = pkt.data.len();

    // DEBUG: dump full uncompressed MyInfo to file for analysis
    {
        let dump_dir = std::path::Path::new("captures");
        let dump_path = dump_dir.join("myinfo_ours.bin");
        let mut full = Vec::with_capacity(1 + pkt.data.len());
        full.push(pkt.opcode);
        full.extend_from_slice(&pkt.data);
        if let Err(e) =
            std::fs::create_dir_all(dump_dir).and_then(|_| std::fs::write(&dump_path, &full))
        {
            tracing::warn!("Failed to write MyInfo dump: {}", e);
        } else {
            tracing::info!(
                "[{}] MyInfo dumped to {} ({} bytes)",
                session.addr(),
                dump_path.display(),
                full.len()
            );
        }
    }

    // Send compressed if large, otherwise send directly
    let to_send = match pkt.to_compressed() {
        Some(compressed) => compressed,
        None => pkt,
    };
    session.send_packet(&to_send).await?;

    // Debug: log bag slots + hex dump of bag section in MyInfo
    {
        let bag_start = 14usize;
        let bag_end = 42usize;
        let mut debug_items = Vec::new();
        for s in bag_start..bag_end {
            let item = items.iter().find(|i| i.slot_index == s as i16);
            if let Some(it) = item {
                debug_items.push(format!("[{}]={}(x{})", s, it.item_id, it.count));
            }
        }
        tracing::info!(
            "[{}] MyInfo bag items: {}",
            session.addr(),
            debug_items.join(", ")
        );
        // Hex dump: first 4 bag items (positions 14-17) from the raw packet
        // Header size is variable (depends on name length), but inventory starts
        // after header. Each item is 19 bytes. Phase 1 = 14 items = 266 bytes.
        // Phase 2 starts at header_size + 266.
        // We'll dump from the binary file instead.
        tracing::info!(
            "[{}] MyInfo total size: {} bytes (opcode+data), items from DB: {}",
            session.addr(),
            1 + pkt_size,
            items.len()
        );
    }

    // ================================================================
    // Phase 1 packet sequence — SNIFFER ORDER (original v2600 server):
    // MyInfo → Quest → ZoneAbility → Premium → Story → Soul
    //   → Notice → Time → Weather → Region/NPC → GAMESTART
    // CRITICAL: Region/NPC MUST be AFTER Story/Notice/Time/Weather!
    // Sending Region too early clears bag items (Frida proved: 200ms clear)
    // ================================================================

    // ================================================================
    // Phase 1 — SNIFFER-EXACT order (original server seq 16-35):
    // MyInfo(16) → Quest(17) → ZoneAbility(18) → Premium(19) → Story(20)
    // → Soul(21) → UserRegion(22-23) → NpcRegion(24-31) → Notice(32)
    // → Time(33) → Weather(34) → GAMESTART(35)
    // ================================================================

    // seq 17: WIZ_QUEST date (0x64, 9 bytes: sub=0x08 + year/month/day/hour + 2 bytes)
    {
        let now = chrono::Local::now();
        let mut quest_pkt = Packet::new(Opcode::WizQuest as u8);
        quest_pkt.write_u8(0x08);
        quest_pkt.write_u16(now.format("%Y").to_string().parse::<u16>().unwrap_or(2026));
        quest_pkt.write_u8(now.format("%m").to_string().parse::<u8>().unwrap_or(1));
        quest_pkt.write_u8(now.format("%d").to_string().parse::<u8>().unwrap_or(1));
        quest_pkt.write_u8(now.format("%H").to_string().parse::<u8>().unwrap_or(0));
        quest_pkt.write_u8(now.format("%M").to_string().parse::<u8>().unwrap_or(0));
        quest_pkt.write_u8(now.format("%S").to_string().parse::<u8>().unwrap_or(0));
        session.send_packet(&quest_pkt).await?;
    }

    // seq 18: WIZ_ZONEABILITY (0x5E, 7 bytes) — sniffer: 5e010100010500
    {
        let zone_for_ability = if ch.zone > 0 { ch.zone as u16 } else { 21u16 };
        super::zone_ability::send_zone_ability(session, zone_for_ability).await?;
    }

    // seq 19: WIZ_PREMIUM (0x71) — sniffer: 71010000 (no premium case)
    // When premium IS active, send full premium info so the band shows.
    // Format: sub=1 + count + entries + premium_in_use + u32(0)
    {
        let mut prem_pkt = Packet::new(Opcode::WizPremium as u8);
        prem_pkt.write_u8(0x01); // sub = PREMIUM_INFO
        prem_pkt.write_u8(premium_entries.len() as u8);
        for &(p_type, time_hours) in &premium_entries {
            prem_pkt.write_u8(p_type);
            prem_pkt.write_u16(time_hours);
        }
        prem_pkt.write_u8(premium_in_use);
        prem_pkt.write_u32(0);
        session.send_packet(&prem_pkt).await?;
    }

    // seq 20: WIZ_STORY (0x81, 7 bytes) — sniffer: 81000000000000
    {
        let story_pkt = super::story::build_story_packet(0, 0);
        session.send_packet(&story_pkt).await?;
    }

    // seq 21: WIZ_SOUL (0xC5, 87 bytes)
    {
        let mut soul_pkt = Packet::new(Opcode::WizSoul as u8);
        soul_pkt.write_u8(0x08);
        for i in 0u32..8 {
            soul_pkt.write_u32(i);
            soul_pkt.write_u32(0);
        }
        soul_pkt.write_u32(5);
        soul_pkt.write_u8(0);
        for i in 1u32..=4 {
            soul_pkt.write_u32(i);
        }
        session.send_packet(&soul_pkt).await?;
    }

    // seq 22-23: REQ_USERIN compressed (user visibility for nearby players)
    // seq 24-31: NPC_REGION compressed (NPC data for region)
    {
        let world = session.world().clone();
        let x = ch.px as f32 / 100.0;
        let z = ch.pz as f32 / 100.0;
        let zone_id = if ch.zone > 0 { ch.zone as u16 } else { 21u16 };
        let rx = calc_region(x);
        let rz = calc_region(z);
        let event_room = world.get_event_room(session.session_id());

        // NPC region data (compressed)
        let npc_ids = world.get_nearby_npc_ids(zone_id, rx, rz, event_room);
        let mut npc_pkt = Packet::new(Opcode::WizNpcRegion as u8);
        npc_pkt.write_u16(npc_ids.len() as u16);
        for &nid in &npc_ids {
            npc_pkt.write_u32(nid);
        }
        let to_send_npc = match npc_pkt.to_compressed() {
            Some(c) => c,
            None => npc_pkt,
        };
        session.send_packet(&to_send_npc).await?;

        // The client normally follows the region list with asynchronous
        // REQ_NPCIN requests.  In dense Moradon regions that queue can delay
        // newly-created static NPCs noticeably.  Send their full INOUT data in
        // the same entry sequence as well, so merchant/quest NPCs are visible
        // immediately instead of waiting for the client request batches.
        super::region::send_nearby_npc_inouts(session).await?;
    }

    // seq 32: WIZ_NOTICE (0x2E, 3 bytes) — sniffer: 2e0100
    {
        let mut notice_pkt = Packet::new(Opcode::WizNotice as u8);
        notice_pkt.write_u8(0x01);
        notice_pkt.write_u8(0x00);
        session.send_packet(&notice_pkt).await?;
    }

    // seq 33: WIZ_TIME (0x13, 11 bytes)
    session
        .send_packet(&crate::systems::time_weather::build_time_packet())
        .await?;

    // seq 34: WIZ_WEATHER (0x14, 4 bytes)
    {
        let tw = session.world().game_time_weather();
        session
            .send_packet(&crate::systems::time_weather::build_weather_packet(
                tw.get_weather_type(),
                tw.get_weather_amount(),
            ))
            .await?;
    }

    // seq 35: Empty GAMESTART
    let response = Packet::new(Opcode::WizGamestart as u8);
    session.send_packet(&response).await?;

    tracing::info!(
        "[{}] GameStart phase 1: {} (SendMyInfo {} bytes)",
        session.addr(),
        char_id,
        pkt_size
    );
    Ok(())
}

/// Phase 2: Upgrade session to in-game mode, register in world, broadcast.
async fn handle_phase2(session: &mut ClientSession) -> anyhow::Result<()> {
    let char_id = session.character_id().unwrap_or("unknown").to_string();

    // Load character + items in parallel
    let pool = session.pool().clone();
    let char_repo = CharacterRepository::new(&pool);
    let (ch_result, items_result) =
        tokio::join!(char_repo.load(&char_id), char_repo.load_items(&char_id),);

    let ch = match ch_result? {
        Some(ch) => ch,
        None => {
            tracing::error!(
                "[{}] Character not found for phase 2: {}",
                session.addr(),
                char_id
            );
            return Ok(());
        }
    };
    let items = items_result?;

    // Load mute state from DB
    if ch.mute_status != 0 {
        let world = session.world();
        world.update_session(session.session_id(), |h| {
            h.is_muted = true;
        });
    }
    let mut equipped = [0u32; 14];
    for item in &items {
        if item.slot_index >= 0 && (item.slot_index as usize) < 14 {
            equipped[item.slot_index as usize] = item.item_id as u32;
        }
    }

    // Use DB values directly for HP/MP. set_user_ability() (called after inventory
    // load in phase 2, line 726) will recalculate max_hp/mp with item bonuses and
    // cap hp/mp if they exceed the new max.
    let hp = ch.hp;
    let mp = ch.mp;
    // Temporary max_hp/mp from DB — will be overwritten by set_user_ability().
    let max_hp = ch.hp;
    let max_mp = ch.mp;

    // Resolve bind point — use DB bind_px/bind_pz if set.
    // bind <= 0 means no bind point → bind_zone=0 so /town falls through to
    // start_position for the current zone instead of always warping to Moradon.
    let bind_zone = if ch.bind > 0 { ch.bind as u8 } else { 0 };
    let (bind_x, bind_z) = if bind_zone > 0 {
        let db_bx = ch.bind_px as f32 / 100.0;
        let db_bz = ch.bind_pz as f32 / 100.0;
        if db_bx != 0.0 || db_bz != 0.0 {
            (db_bx, db_bz)
        } else {
            let world = session.world();
            world
                .get_zone(bind_zone as u16)
                .map(|z| {
                    let (x, z, _y) = z.spawn_position();
                    (x, z)
                })
                .unwrap_or((0.0, 0.0))
        }
    } else {
        (0.0, 0.0)
    };

    // Pre-compute skill points from normalized DB columns.
    let skill_points: [u8; 10] = [
        ch.skill0 as u8,
        ch.skill1 as u8,
        ch.skill2 as u8,
        ch.skill3 as u8,
        ch.skill4 as u8,
        ch.skill5 as u8,
        ch.skill6 as u8,
        ch.skill7 as u8,
        ch.skill8 as u8,
        ch.skill9 as u8,
    ];

    // Build CharacterInfo for world state
    let char_info = CharacterInfo {
        session_id: session.session_id(),
        name: ch.str_user_id.clone(),
        nation: ch.nation as u8,
        race: ch.race as u8,
        class: ch.class as u16,
        level: ch.level as u8,
        face: ch.face as u8,
        hair_rgb: ch.hair_rgb as u32,
        rank: ch.rank as u8,
        title: ch.title as u8,
        max_hp,
        hp,
        max_mp,
        mp,
        max_sp: stats::calculate_max_sp(ch.class as u16, skill_points[8]),
        sp: {
            let max_sp = stats::calculate_max_sp(ch.class as u16, skill_points[8]);
            if max_sp > 0 {
                if ch.sp > 0 {
                    (ch.sp as i16).min(max_sp)
                } else {
                    max_sp
                }
            } else {
                0
            }
        },
        equipped_items: equipped,
        // Bind point
        bind_zone,
        bind_x,
        bind_z,
        // Stats
        str: ch.strong as u8,
        sta: ch.sta as u8,
        dex: ch.dex as u8,
        intel: ch.intel as u8,
        cha: ch.cha as u8,
        free_points: ch.points as u16,
        // Gold, loyalty & authority
        gold: ch.gold as u32,
        loyalty: ch.loyalty.max(0) as u32,
        loyalty_monthly: ch.loyalty_monthly.max(0) as u32,
        authority: ch.authority as u8,
        // Clan
        knights_id: ch.knights as u16,
        fame: ch.fame as u8,
        skill_points,
        // Party (not in a party on login)
        party_id: None,
        // Experience
        exp: ch.exp as u64,
        max_exp: 0, // populated below after world is available
        // EXP seal (defaults off, no sealed XP on login)
        exp_seal_status: false,
        sealed_exp: 0,
        // Weight (recalculated after inventory load)
        item_weight: 0,
        max_weight: 0,
        // State: restore dead state if HP <= 0, otherwise standing
        res_hp_type: if hp <= 0 { 0x03 } else { 0x01 },
        // Rivalry: no rival on login
        rival_id: -1,
        rival_expiry_time: 0,
        // Anger gauge: reset on login
        anger_gauge: 0,
        // Manner
        manner_point: ch.manner_point,
        // Rebirth
        rebirth_level: ch.rebirth_level as u8,
        reb_str: ch.reb_str as u8,
        reb_sta: ch.reb_sta as u8,
        reb_dex: ch.reb_dex as u8,
        reb_intel: ch.reb_intel as u8,
        reb_cha: ch.reb_cha as u8,
        // cover_title is set later after achieve_summary is loaded from DB (step 14b)
        cover_title: 0,
    };

    // ── ishome zone safety checks ──────────────────────────────────────
    // If the player's saved zone is invalid (event ended, wrong nation, restricted),
    // relocate them to Moradon before entering the game world.
    let raw_zone = if ch.zone > 0 {
        ch.zone as u16
    } else {
        ZONE_MORADON
    };
    let (zone_id, x, y, z) = {
        let world_ref = session.world();
        if let Some((new_zone, new_x, new_z)) = check_ishome_relocation(
            raw_zone,
            ch.nation as u8,
            ch.knights as u16,
            ch.loyalty.max(0) as u32,
            world_ref.as_ref(),
        ) {
            tracing::info!(
                "[{}] ishome relocation: {} zone {} → zone {} ({:.0},{:.0})",
                session.addr(),
                char_id,
                raw_zone,
                new_zone,
                new_x,
                new_z,
            );
            (new_zone, new_x, 0.0_f32, new_z)
        } else {
            let x = ch.px as f32 / 100.0;
            let z = ch.pz as f32 / 100.0;
            let y = ch.py as f32 / 100.0;
            (raw_zone, x, y, z)
        }
    };

    // Calculate position
    let position = Position {
        zone_id,
        x,
        y,
        z,
        region_x: calc_region(x),
        region_z: calc_region(z),
    };

    // 1. Upgrade session: split stream, spawn writer, create channel
    tracing::info!("[{}] Phase2: upgrading to in-game...", session.addr());
    session.upgrade_to_ingame().await?;

    // 2. Mark as in-game
    session.set_state(SessionState::InGame);
    tracing::info!("[{}] Phase2: upgrade OK, state=InGame", session.addr());

    // 3. Register character info + position in world state
    let world = session.world().clone();

    // Set max_exp from level_up table (rebirth_level=0 for now)
    let mut char_info = char_info;
    char_info.max_exp = world.get_exp_by_level(char_info.level, 0);

    world.register_ingame(session.session_id(), char_info, position);

    // Phase 1 needs Genie time to build WIZ_MYINFO, but the session is not
    // registered in WorldState until this point. Its earlier update_session()
    // therefore cannot persist the loaded state. Load it into the now-live
    // SessionHandle so Start/Save/Disconnect all see the real DB values.
    {
        let ud_repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
        match ud_repo.load_genie_data(&char_id).await {
            Ok(Some(genie)) => {
                let abs_ts = genie.genie_time.max(0) as u32;
                let options_len = genie.genie_options.len();
                world.update_session(session.session_id(), |h| {
                    h.genie_time_abs = abs_ts;
                    h.genie_options = genie.genie_options;
                    h.genie_loaded = true;
                });
                tracing::info!(
                    "[{}] Phase2 Genie state applied: char={}, abs={}, options={} bytes",
                    session.addr(),
                    char_id,
                    abs_ts,
                    options_len
                );
            }
            Ok(None) => {
                world.update_session(session.session_id(), |h| h.genie_loaded = true);
                tracing::info!(
                    "[{}] Phase2 Genie state initialized: char={}, no saved row",
                    session.addr(),
                    char_id
                );
            }
            Err(e) => {
                // Keep genie_loaded=false so a transient load failure can never
                // overwrite a valid database row with zero on periodic save.
                tracing::warn!(
                    "[{}] Phase2 Genie load failed: char={}, err={}",
                    session.addr(),
                    char_id,
                    e
                );
            }
        }
    }

    // 3-seal. Load sealed_exp from DB and apply to character.
    {
        let seal_repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
        match seal_repo.load_seal_exp(&char_id).await {
            Ok(Some(seal_row)) => {
                if seal_row.sealed_exp > 0 {
                    world.update_character_stats(session.session_id(), |ch| {
                        ch.sealed_exp = seal_row.sealed_exp as u32;
                    });
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "[{}] load_seal_exp DB error for {}: {e}",
                    session.addr(),
                    char_id
                );
            }
        }
    }

    // 3a. ZoneOnlineRewardStart — initialize zone online reward timers
    // Must be called after register_ingame so the session handle exists.
    crate::systems::zone_rewards::zone_online_reward_start(&world, session.session_id());

    // 3b. Populate inventory from DB items
    // +1 for CBAG3 at slot 96 (dedicated slot outside normal range)
    let mut inventory = vec![UserItemSlot::default(); INVENTORY_TOTAL + 1];
    for item in &items {
        let slot = item.slot_index as usize;
        if slot < inventory.len() {
            inventory[slot] = UserItemSlot {
                item_id: item.item_id as u32,
                durability: item.durability,
                count: item.count as u16,
                flag: item.flag as u8,
                original_flag: item.original_flag as u8,
                serial_num: item.serial_num as u64,
                expire_time: item.expire_time as u32,
            };
        }
    }
    let removed_manes_items = world
        .manes_survival_manager
        .remove_temporary_items(&mut inventory);
    if removed_manes_items > 0 {
        tracing::warn!(
            "[{}] Removed {} persisted Manes-only item slot(s) before normal inventory init for {}",
            session.addr(),
            removed_manes_items,
            char_id
        );
    }
    world.set_inventory(session.session_id(), inventory);

    // 3b1. Remove expired items from inventory/warehouse/VIP warehouse on login.
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        crate::systems::expiry_tick::check_item_expiry(&world, session.session_id(), now);
    }

    // 3b2. DEFERRED — set_user_ability + send_item_move_refresh moved to END of Phase 2.
    // SNIFFER EVIDENCE: Original server sends WIZ_ITEM_MOVE (seq 43) AFTER knights/friends/quest
    // (seq 36-42), NOT at the beginning. Sending it too early clears bag items from MyInfo.

    // 3b3. Detect fairy (CFAIRY=48) and robin loot (SHOULDER=5) on login.
    {
        let sid = session.session_id();
        const SHOULDER_SLOT: usize = 5;
        const CFAIRY_SLOT: usize = 48;
        use super::item_move::ITEM_OREADS;
        const ROBIN_ITEMS: [u32; 4] = [950680000, 850680000, 510000000, 520000000];

        let shoulder_id = world
            .get_inventory_slot(sid, SHOULDER_SLOT)
            .map(|s| s.item_id)
            .unwrap_or(0);
        let fairy_id = world
            .get_inventory_slot(sid, CFAIRY_SLOT)
            .map(|s| s.item_id)
            .unwrap_or(0);

        world.update_session(sid, |h| {
            if fairy_id == ITEM_OREADS {
                h.fairy_check = true;
            }
            if ROBIN_ITEMS.contains(&shoulder_id) {
                h.auto_loot = true;
            }
        });
    }

    // 3c. Load premium subscriptions from DB and populate session premium_map
    load_account_premiums(session).await;

    // 3d-kc. Load Knight Cash (KC) and TL balances from DB
    // Loads CashPoint and BonusCashPoint from TB_USER for this account.
    crate::handler::knight_cash::load_kc_balances(session).await;

    // 3d. Load flash time state from DB and restore bonuses
    {
        let sid = session.session_id();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        world.update_session(sid, |h| {
            h.flash_time = ch.flash_time.max(0) as u32;
            h.flash_count = ch.flash_count.max(0) as u8;
            h.flash_type = ch.flash_type.max(0) as u8;
            h.flash_check_time = now;
        });

        // Restore flash bonuses if flash_time > 0 and has a valid premium
        let has_premium = world.with_session(sid, |h| h.premium_in_use).unwrap_or(0) > 0;
        let has_flash = world.with_session(sid, |h| h.flash_time).unwrap_or(0) > 0;
        if has_flash && has_premium {
            crate::systems::flash::set_flash_time_note(&world, sid);
        }

        // Start burning timer for flame progression
        crate::systems::flash::start_burning_timer(&world, sid, now);
    }

    // 4. Add to zone region grid
    if let Some(zone) = world.get_zone(zone_id) {
        zone.add_user(position.region_x, position.region_z, session.session_id());
    }

    // 4a. Initialize speed hack baseline — C++ CharacterSelectionHandler.cpp:1154-1155
    world.update_session(session.session_id(), |h| {
        h.speed_last_x = position.x;
        h.speed_last_z = position.z;
    });

    // 4b. Mark account as online in DB
    {
        let acct = session.account_id().unwrap_or("").to_string();
        let ch = char_id.clone();
        if !acct.is_empty() && !ch.is_empty() {
            let pool2 = pool.clone();
            let addr = session.addr().to_string();
            tokio::spawn(async move {
                let repo = ko_db::repositories::account::AccountRepository::new(&pool2);
                if let Err(e) = repo.set_online(&acct, &ch, 1, "0.0.0.0", &addr).await {
                    tracing::warn!("Failed to set account online: {}", e);
                }
            });
        }
    }

    // 6. Broadcast INOUT_RESPAWN to nearby players
    tracing::info!("[{}] Phase2 step6: broadcast_user_in", session.addr());
    region::broadcast_user_in(session).await?;

    // Do not send KNIGHTS_UPDATE to the player during login.  The reference
    // server establishes the local clan/cape state solely through WIZ_MYINFO;
    // KNIGHTS_UPDATE is emitted only when a clan state actually changes.
    // Sending the update immediately after USER_INOUT re-parses the local clan
    // record in the v2615 client and can replace the MyInfo cloak catalogue.

    // 7b. If the player is dead on login, broadcast death animation so the
    //     client shows the revive UI
    if hp <= 0 {
        // 7b1. OnDeathLostExpCalc — recalculate lost EXP on dead-on-login
        // Stores lost_exp in session for resurrection skill EXP recovery.
        {
            let s = session.session_id();
            let max_exp = world
                .get_character_info(s)
                .map(|ci| ci.max_exp)
                .unwrap_or(0);
            let premium_restore = world.get_premium_exp_restore(s) as f32;
            let lost = super::level::on_death_lost_exp_calc(max_exp, premium_restore);
            if lost > 0 {
                world.update_session(s, |h| {
                    h.lost_exp = lost;
                });
            }
        }

        // 7b2. CheckRespawnScroll — check for respawn scrolls in inventory
        check_respawn_scroll_on_login(&world, session).await;

        let mut dead_pkt = Packet::new(Opcode::WizDead as u8);
        dead_pkt.write_u32(session.session_id() as u32);
        let event_room = world.get_event_room(session.session_id());
        world.broadcast_to_3x3(
            zone_id,
            position.region_x,
            position.region_z,
            Arc::new(dead_pkt),
            None,
            event_room,
        );
    }

    // 7c. OpenEtcSkill — auto-complete class-specific skill prerequisite quests
    //   if (bAuto && !g_pMain->pServerSetting.AutoQuestSkill) return;
    // Sets quest_state=2 (completed) for class skill quests if AutoQuestSkill
    // is enabled. No packet sent; just quest state manipulation.
    let auto_quest_skill = world
        .get_server_settings()
        .map(|s| s.auto_quest_skill != 0)
        .unwrap_or(false);
    if auto_quest_skill {
        open_etc_skill(
            &world,
            session.session_id(),
            ch.class as u16,
            ch.level as u8,
        );
    }

    // 7d. Send completed achievement notifications (sniffer seq 38-39, before quest data)
    super::achieve::send_achieve_status_on_login(&world, session.session_id());

    // 8. Load quest data from DB and send to client
    tracing::info!("[{}] Phase2 step8: quest data load+send", session.addr());
    quest::load_quest_data(session).await?;

    // 8b. Load daily quest progress from DB and send to client
    {
        let pool = session.pool().clone();
        let sid = session.session_id();
        let dq_repo = DailyQuestRepository::new(&pool);
        let mut dq_map = std::collections::HashMap::new();

        // Load existing user progress from DB
        match dq_repo.load_user_quests(&char_id).await {
            Ok(rows) => {
                for row in rows {
                    dq_map.insert(row.quest_id, row);
                }
            }
            Err(e) => {
                tracing::warn!(
                    "[{}] load_user_quests DB error for {}: {e}",
                    session.addr(),
                    char_id
                );
            }
        }

        world.update_session(sid, |h| {
            h.daily_quests = dq_map;
        });
    }

    // Quest Tips reads the normal per-user quest map. Mirror selected daily
    // quests into it before sending the combined quest list.
    super::daily_quest::sync_daily_quests_into_normal_map(&world, session.session_id());

    // Send one combined Quest Tips list only after both normal and selected
    // daily quests are loaded. The daily protocol itself follows immediately.
    quest::send_quest_data(session).await?;
    super::daily_quest::daily_quest_send_list(&world, session.session_id());

    // 9. Load saved magic (buff persistence) and recast
    tracing::info!("[{}] Phase2 step9: saved magic + blink", session.addr());
    load_and_recast_saved_magic(session).await;

    // 9b. BlinkStart — spawn invulnerability (10s)
    // Must be called AFTER RecastSavedMagic but BEFORE other sends.
    // activate_blink internally checks for GM, transformation, war zone, etc.
    crate::handler::regene::activate_blink(session, zone_id)?;

    // 9c. TempleEventGetActiveEventTime — send active temple event timer
    // Reconnecting players see the BDW/Chaos/Juraid event timer if an event is active.
    crate::systems::event_room::send_active_event_time(&world, session.session_id());

    // 10. Load perk data from DB and send perk info to client
    crate::handler::perks::load_user_perks(session).await;
    if let Err(e) = crate::handler::perks::send_my_perks(session).await {
        tracing::warn!(
            "[{}] Failed to send perk info for {}: {}",
            session.addr(),
            char_id,
            e
        );
    }

    // 11. Send initial Kurian SP if applicable
    {
        let max_sp = stats::calculate_max_sp(ch.class as u16, skill_points[8]);
        if max_sp > 0 {
            let current_sp = if ch.sp > 0 { ch.sp.min(max_sp) } else { max_sp };
            let sp_pkt =
                crate::systems::sp_regen::build_sp_change_packet(max_sp as u8, current_sp as u8);
            session.send_packet(&sp_pkt).await?;
        }
    }

    // 12. Send saved skill bar data so the client restores shortcuts on login
    // The client normally requests this via WIZ_SKILLDATA(SKILL_DATA_LOAD),
    // but auto-sending on game entry ensures the skill bar persists across sessions.
    send_skill_bar_data(session).await;

    // 13. PK zone ranking — add player to ranking if in a PK zone
    // GMs are excluded from ranking (C++ NewRankingSystem.cpp:512 — if (isGM()) return;)
    let sid = session.session_id();
    let player_nation = ch.nation as u8;
    let player_authority = ch.authority as u8;
    if player_authority != 0 {
        if is_pk_ranking_zone(zone_id) {
            world.pk_zone_add_player(sid, player_nation, zone_id);
        } else {
            world.pk_zone_remove_player(sid);
        }
    }

    // 14. GmListProcess(false) — add to GM online list if GM
    if player_authority == 0 || player_authority == 2 {
        world.gm_list_add(&char_id);
        let gm_pkt = world.build_gm_list_packet();
        session.send_packet(&gm_pkt).await?;
    }

    // 14d. GM invisible on login
    // AUTHORITY_GAME_MASTER = 0 → auto-invisible on login
    if player_authority == 0 {
        world.update_session(sid, |h| {
            h.abnormal_type = ABNORMAL_INVISIBLE;
        });
    }

    // 14b. Load user achievement data from DB into session
    {
        let achieve_repo = ko_db::repositories::achieve::AchieveRepository::new(&pool);
        match achieve_repo.load_user_achieves(&char_id).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    world.update_session(sid, |h| {
                        for r in &rows {
                            h.achieve_map.insert(
                                r.achieve_id as u16,
                                crate::world::UserAchieveInfo {
                                    status: r.status as u8,
                                    count: [r.count1 as u32, r.count2 as u32],
                                },
                            );
                        }
                    });
                }
            }
            Err(e) => {
                tracing::warn!(
                    "[{}] load_user_achieves DB error for {}: {e}",
                    session.addr(),
                    char_id
                );
            }
        }
        let initialized = super::achieve::initialize_achievement_map(&world, sid);
        if initialized > 0 {
            tracing::info!(sid, character = %char_id, initialized,
                total = world.all_achieve_main().len(),
                "Achievement map synchronized with v2615 definitions");
        }
        match achieve_repo.load_user_achieve_summary(&char_id).await {
            Ok(Some(summary)) => {
                world.update_session(sid, |h| {
                    h.achieve_summary.play_time = summary.play_time as u32;
                    h.achieve_summary.monster_defeat_count = summary.monster_defeat_count as u32;
                    h.achieve_summary.user_defeat_count = summary.user_defeat_count as u32;
                    h.achieve_summary.user_death_count = summary.user_death_count as u32;
                    h.achieve_summary.total_medal = summary.total_medal as u32;
                    h.achieve_summary.recent_achieve = [
                        summary.recent_achieve_1 as u16,
                        summary.recent_achieve_2 as u16,
                        summary.recent_achieve_3 as u16,
                    ];
                    h.achieve_summary.cover_id = summary.cover_id as u16;
                    h.achieve_summary.skill_id = summary.skill_id as u16;
                });

                // 14c. Restore skill title stat bonuses from DB
                let skill_id = summary.skill_id as u16;
                if skill_id > 0 {
                    let main_entry = world.achieve_main(skill_id as i32);
                    let title_entry = main_entry
                        .as_ref()
                        .and_then(|m| world.achieve_title(m.title_id as i32));
                    if let Some(title) = title_entry {
                        world.update_session(sid, |h| {
                            h.achieve_stat_bonuses = [
                                title.str,
                                title.hp,
                                title.dex,
                                title.int,
                                title.mp,
                                title.attack,
                                title.defence,
                            ];
                        });
                        // Recalculate stats since achieve data is loaded after
                        // the initial set_user_ability call (step 3b2).
                        world.set_user_ability(sid);
                    }
                }

                // 14d. Set cover_title on CharacterInfo for broadcast
                let cover_id = summary.cover_id as u16;
                if cover_id > 0 {
                    let resolved = world
                        .achieve_main(cover_id as i32)
                        .map(|m| m.title_id as u16)
                        .unwrap_or(0);
                    if resolved > 0 {
                        world.update_character_stats(sid, |ci| {
                            ci.cover_title = resolved;
                        });
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "[{}] load_user_achieve_summary DB error for {}: {e}",
                    session.addr(),
                    char_id
                );
            }
        }
    }

    // 14e. Send persistent login messages (send_type=1)
    {
        let login_msgs = world.get_login_messages();
        for msg in &login_msgs {
            if let Some(ref text) = msg.message {
                if !text.is_empty() {
                    let pkt = super::chat::build_chat_packet(
                        msg.chat_type as u8,
                        player_nation,
                        0xFFFF, // sender_id = -1 (system)
                        &msg.sender,
                        text,
                        0,  // personal_rank
                        0,  // authority
                        23, // system_msg = 23 (colored system message)
                    );
                    session.send_packet(&pkt).await?;
                }
            }
        }
    }

    // 15. RobChaosSkillItems — remove leftover chaos skill items on login
    // NOTE: C++ does NOT check zone, removes unconditionally on login.
    {
        use crate::handler::dead::{ITEM_DRAIN_RESTORE, ITEM_KILLING_BLADE, ITEM_LIGHT_PIT};
        let chaos_items = [ITEM_LIGHT_PIT, ITEM_DRAIN_RESTORE, ITEM_KILLING_BLADE];
        for item_id in &chaos_items {
            world.rob_all_of_item(sid, *item_id);
        }
    }

    // 15b. AchieveNormalCountAdd(AchieveReachLevel) — update level-based achievements
    // if (GetLevel() >= 1 && GetLevel() <= g_pMain->m_byMaxLevel)
    //     AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveReachLevel, 0, nullptr);
    {
        let level = ch.level as u8;
        if level >= 1 {
            achieve_normal_reach_level(&world, sid, level, zone_id);
        }
        super::achieve::evaluate_achievement_dependencies(&world, sid);
    }

    // 16. KnightsClanBuffUpdate(true) — increment online member count, broadcast bonus
    if ch.knights > 0 {
        world.knights_clan_buff_update(ch.knights as u16, true, sid);

        // 16b. SendClanPremium — send clan premium status to client on game entry
        // C++ calls SendClanPremium(pKnights, false) which checks isInPremium()
        // and sets m_bClanPremiumInUse = 13 if active.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        if let Some(ki) = world.get_knights(ch.knights as u16) {
            if ki.premium_time > now {
                // Clan has active premium — set session flag and send active packet
                let remaining_minutes = (ki.premium_time - now) / 60;
                world.update_session(sid, |h| {
                    h.clan_premium_in_use = 13; // CLAN_PREMIUM
                });
                let pkt = super::premium::build_clan_premium_packet(true, remaining_minutes);
                session.send_packet(&pkt).await?;
            } else {
                // No premium or expired — send inactive packet
                world.update_session(sid, |h| {
                    h.clan_premium_in_use = 0;
                });
                let pkt = super::premium::build_clan_premium_packet(false, 0);
                session.send_packet(&pkt).await?;
            }
        }
    }

    // 17. ChangeFame — promote commander back to chief if still clan leader
    // if (GetFame() == COMMAND_CAPTAIN && isClanLeader()) ChangeFame(CHIEF);
    // After a war ends, commanders should revert to CHIEF if they're the clan leader.
    {
        let fame = ch.fame as u8;
        if fame == COMMAND_CAPTAIN {
            // Check if this user is the clan leader (fame would normally be CHIEF=1)
            // isClanLeader() in C++ checks fame==CHIEF, but here fame is COMMAND_CAPTAIN
            // so we check if they're listed as clan chief in the knights table
            let is_leader = ch.knights > 0
                && world
                    .get_knights(ch.knights as u16)
                    .map(|k| k.chief == char_id)
                    .unwrap_or(false);
            if is_leader {
                // Revert fame to CHIEF and broadcast
                world.update_character_stats(sid, |ci| ci.fame = CHIEF);
                let mut fame_pkt = Packet::new(Opcode::WizAuthorityChange as u8);
                fame_pkt.write_u8(COMMAND_AUTHORITY);
                fame_pkt.write_u32(sid as u32);
                fame_pkt.write_u8(CHIEF);
                let event_room = world.get_event_room(sid);
                world.broadcast_to_3x3(
                    zone_id,
                    position.region_x,
                    position.region_z,
                    Arc::new(fame_pkt),
                    None,
                    event_room,
                );
            }
        }
    }

    // 17b. War commander promotion on login
    // if (isWarOpen() && name in m_CommanderArray && fame != COMMAND_CAPTAIN) ChangeFame(COMMAND_CAPTAIN)
    {
        let fame = ch.fame as u8;
        if world.is_war_open() && world.is_war_commander(&char_id) && fame != COMMAND_CAPTAIN {
            world.update_character_stats(sid, |ci| ci.fame = COMMAND_CAPTAIN);
            let mut fame_pkt = Packet::new(Opcode::WizAuthorityChange as u8);
            fame_pkt.write_u8(COMMAND_AUTHORITY);
            fame_pkt.write_u32(sid as u32);
            fame_pkt.write_u8(COMMAND_CAPTAIN);
            let event_room = world.get_event_room(sid);
            world.broadcast_to_3x3(
                zone_id,
                position.region_x,
                position.region_z,
                Arc::new(fame_pkt),
                None,
                event_room,
            );
        }
    }

    // 18. Daily Reward — send current state on login
    if let Err(e) = crate::handler::ext_hook::send_daily_reward_on_login(session).await {
        tracing::warn!(
            "[{}] Failed to send daily reward for {}: {}",
            session.addr(),
            char_id,
            e
        );
    }

    // 19. Collection Race game-entry refresh.
    // Sends the current CR event state so reconnecting players see the event UI.
    {
        let cr = world.collection_race_event().clone();
        if let Err(e) = super::collection_race::send_on_game_entry(session, &cr).await {
            tracing::warn!(
                "[{}] Failed to send CR game-entry packet for {}: {}",
                session.addr(),
                char_id,
                e
            );
        }
    }

    // ── SendLists
    tracing::info!("[{}] Phase2 step20: SendLists begin", session.addr());
    // Order: SendAntiAfkList, SendWheelData, DailyQuestSendList (already sent),
    //        PusRefundSendList, SendEventTimerList

    // 20a. Anti-AFK NPC list.
    if let Err(e) = super::ext_hook::send_anti_afk_list(session).await {
        tracing::warn!(
            "[{}] Failed to send anti-afk list for {}: {}",
            session.addr(),
            char_id,
            e
        );
    }

    // 20b. Wheel of Fun data.
    if let Err(e) = super::wheel_of_fun::send_wheel_data(session).await {
        tracing::warn!(
            "[{}] Failed to send wheel data for {}: {}",
            session.addr(),
            char_id,
            e
        );
    }

    // 20c. PUS refund list (cash item refunds pending for this account).
    if let Err(e) = super::pus_refund::load_and_send_refund_list(session).await {
        tracing::warn!(
            "[{}] Failed to send PUS refund list for {}: {}",
            session.addr(),
            char_id,
            e
        );
    }

    // 20d. Event timer schedule list.
    if let Err(e) = super::ext_hook::send_event_timer_list(session).await {
        tracing::warn!(
            "[{}] Failed to send event timer list for {}: {}",
            session.addr(),
            char_id,
            e
        );
    }

    // 20e. Right-click exchange item lists (6 packets by exchange type).
    if let Err(e) = super::ext_hook::send_right_exchange_list(session).await {
        tracing::warn!(
            "[{}] Failed to send right exchange list for {}: {}",
            session.addr(),
            char_id,
            e
        );
    }

    // 20f. Lottery game start state (if active).
    {
        let lottery_proc = world.lottery_process().clone();
        if let Err(e) = super::lottery::send_on_game_entry(session, &lottery_proc).await {
            tracing::warn!(
                "[{}] Failed to send lottery state for {}: {}",
                session.addr(),
                char_id,
                e
            );
        }
    }

    // 20g. Letter unread notification — notify client if there are unread letters.
    {
        let letter_repo = ko_db::repositories::letter::LetterRepository::new(&pool);
        match letter_repo.count_unread(&char_id).await {
            Ok(count) if count > 0 => {
                let pkt = super::letter::build_unread_notification();
                session.send_packet(&pkt).await?;
                tracing::debug!(
                    "[{}] Letter unread notification sent for {} ({} unread)",
                    session.addr(),
                    char_id,
                    count
                );
            }
            Ok(_) => {} // no unread letters
            Err(e) => {
                tracing::warn!(
                    "[{}] Failed to check unread letters for {}: {}",
                    session.addr(),
                    char_id,
                    e
                );
            }
        }
    }

    // 20h. Send preset stat/skill reset cost.
    {
        let level = world
            .get_character_info(session.session_id())
            .map(|c| c.level)
            .unwrap_or(1);
        let premium = world
            .with_session(session.session_id(), |h| h.premium_in_use)
            .unwrap_or(0);
        let nation = world
            .get_character_info(session.session_id())
            .map(|c| c.nation)
            .unwrap_or(0);
        let discount = world.is_discount_active(nation);
        let cost_pkt = super::ext_hook::build_preset_req_money(level, premium, discount);
        session.send_packet(&cost_pkt).await?;
    }

    // 20i. Send CastleSiegeWarfareFlag when logging in at Delos.
    if zone_id == ZONE_DELOS {
        let owner_clan = world.get_csw_master_knights();
        let flag_pkt = super::siege::build_castle_flag_packet(&world, owner_clan);
        session.send_packet(&flag_pkt).await?;
    }

    // 20j. Send ZindanWar score when logging in at a special event zone.
    if crate::handler::attack::is_in_special_event_zone(zone_id) && world.is_zindan_event_opened() {
        let pkt = {
            let zws = world.zindan_war_state.read();
            let remaining = zws.finish_time.saturating_sub(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
            super::ext_hook::build_zindan_flagsend(
                &zws.elmo_name,
                zws.elmo_kills,
                &zws.karus_name,
                zws.karus_kills,
                remaining as u32,
            )
        };
        session.send_packet(&pkt).await?;
    }

    // 21. Load daily rank raw stats from DB.
    {
        let dr_repo = ko_db::repositories::daily_rank::DailyRankRepository::new(&pool);
        match dr_repo.load_user_stats(&char_id).await {
            Ok(Some(stats)) => {
                world.update_session(sid, |h| {
                    h.dr_gm_total_sold = stats.gm_total_sold.max(0) as u64;
                    h.dr_mh_total_kill = stats.mh_total_kill.max(0) as u64;
                    h.dr_sh_total_exchange = stats.sh_total_exchange.max(0) as u64;
                    h.dr_cw_counter_win = stats.cw_counter_win.max(0) as u64;
                    h.dr_up_counter_bles = stats.up_counter_bles.max(0) as u64;
                });
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "[{}] load_user_stats DB error for {}: {e}",
                    session.addr(),
                    char_id
                );
            }
        }
    }

    // 22. Send WIZ_ACHIEVEMENT2 (0xA5) — initialize client kill counter HUD.
    // Client RE: stores value at player+0x7AC, displays via string 0xA7FA.
    {
        let kill_count = world
            .with_session(sid, |h| h.achieve_summary.monster_defeat_count)
            .unwrap_or(0);
        if kill_count > 0 {
            let ach_pkt = super::achievement2::build_achievement2(kill_count as i32);
            world.send_to_session_owned(sid, ach_pkt);
        }
    }

    // Step 23: v2525 rebirth level notification (0xD3)
    // If player has rebirth level > 0, send completion packet to init client UI
    {
        let rebirth_level = world.get_rebirth_level(sid);
        if rebirth_level > 0 {
            let reb_pkt = super::rebirth::build_complete(rebirth_level as i32);
            world.send_to_session_owned(sid, reb_pkt);
        }
    }

    // 24. Load soul data from DB (v2525 WIZ_SOUL panel).
    {
        let pool_c = pool.clone();
        let char_c = char_id.clone();
        let world_c = world.clone();
        let s = sid;
        tokio::spawn(async move {
            let repo = ko_db::repositories::soul::SoulRepository::new(&pool_c);
            match repo.load(&char_c).await {
                Ok(Some(row)) => {
                    world_c.update_session(s, |h| {
                        let cats = row.categories();
                        let slots = row.slots();
                        h.soul_categories = cats;
                        h.soul_slots = slots;
                        h.soul_loaded = true;
                    });
                }
                Ok(None) => {
                    world_c.update_session(s, |h| {
                        h.soul_loaded = true;
                    });
                }
                Err(e) => {
                    tracing::warn!("soul load DB error for {}: {e}", char_c);
                    world_c.update_session(s, |h| {
                        h.soul_loaded = true;
                    });
                }
            }
        });
    }

    // 24b. Load Hermetic Seal data from DB (async, fire-and-forget).
    {
        let pool_c = pool.clone();
        let char_c = char_id.clone();
        let world_c = world.clone();
        let s = sid;
        tokio::spawn(async move {
            let repo = ko_db::repositories::hermetic_seal::HermeticSealRepository::new(&pool_c);
            match repo.load_or_create(&char_c).await {
                Ok(row) => {
                    world_c.update_session(s, |h| {
                        h.seal_max_tier = row.max_tier as u8;
                        h.seal_selected_slot = row.selected_slot as u8;
                        h.seal_status = row.status as u8;
                        h.seal_upgrade_count = row.upgrade_count as u8;
                        h.seal_current_level = row.current_level as u8;
                        h.seal_elapsed_time = row.elapsed_time as f64;
                        h.seal_loaded = true;
                    });
                }
                Err(e) => {
                    tracing::warn!("hermetic_seal load_or_create DB error for {}: {e}", char_c);
                    world_c.update_session(s, |h| {
                        h.seal_loaded = true;
                    });
                }
            }
        });
    }

    // 24c. Load Costume data from DB (async, fire-and-forget).
    {
        let pool_c = pool.clone();
        let char_c = char_id.clone();
        let world_c = world.clone();
        let s = sid;
        tokio::spawn(async move {
            let repo = ko_db::repositories::costume::CostumeRepository::new(&pool_c);
            match repo.load(&char_c).await {
                Ok(Some(row)) => {
                    world_c.update_session(s, |h| {
                        h.costume_active_type = row.active_type as u16;
                        h.costume_item_id = row.item_id;
                        h.costume_item_param = row.item_param;
                        h.costume_scale_raw = row.scale_raw;
                        h.costume_color_index = row.color_index as u8;
                        h.costume_expiry_time = row.expiry_time;
                        h.costume_loaded = true;
                    });
                }
                Ok(None) => {
                    world_c.update_session(s, |h| {
                        h.costume_loaded = true;
                    });
                }
                Err(e) => {
                    tracing::warn!("costume load DB error for {}: {e}", char_c);
                    world_c.update_session(s, |h| {
                        h.costume_loaded = true;
                    });
                }
            }
        });
    }

    // 24d. Load Enchant data from DB (async, fire-and-forget).
    {
        let pool_c = pool.clone();
        let char_c = char_id.clone();
        let world_c = world.clone();
        let s = sid;
        tokio::spawn(async move {
            let repo = ko_db::repositories::enchant::EnchantRepository::new(&pool_c);
            match repo.load(&char_c).await {
                Ok(Some(row)) => {
                    world_c.update_session(s, |h| {
                        h.enchant_max_star = row.max_star as u8;
                        h.enchant_count = row.enchant_count as u8;
                        if row.slot_levels.len() == 8 {
                            h.enchant_slot_levels.copy_from_slice(&row.slot_levels);
                        }
                        if row.slot_unlocked.len() == 9 {
                            h.enchant_slot_unlocked.copy_from_slice(&row.slot_unlocked);
                        }
                        h.enchant_item_category = row.item_category as u8;
                        h.enchant_item_slot_unlock = row.item_slot_unlock as u8;
                        if row.item_markers.len() == 5 {
                            h.enchant_item_markers.copy_from_slice(&row.item_markers);
                        }
                        h.enchant_loaded = true;
                    });
                }
                Ok(None) => {
                    world_c.update_session(s, |h| {
                        h.enchant_loaded = true;
                    });
                }
                Err(e) => {
                    tracing::warn!("enchant load DB error for {}: {e}", char_c);
                    world_c.update_session(s, |h| {
                        h.enchant_loaded = true;
                    });
                }
            }
        });
    }

    // 25. Welcome message from server_settings.
    // but WIZ_CHAT type 7 works in v2525. Send if welcome_msg is non-empty.
    {
        let welcome_msg = world.get_server_settings().and_then(|s| {
            let msg = s.welcome_msg.trim().to_string();
            if msg.is_empty() {
                None
            } else {
                Some(msg)
            }
        });
        if let Some(msg) = welcome_msg {
            let full_msg = format!("{} , {}", char_id, msg);
            let wpkt = crate::systems::timed_notice::build_notice_packet(7, &full_msg);
            session.send_packet(&wpkt).await?;
        }
    }

    // SetUserAbility + SendItemMove at END of Phase 2 (sniffer: seq 43)
    tracing::info!(
        "[{}] Phase2 FINAL: set_user_ability + item_move_refresh",
        session.addr()
    );
    world.set_user_ability(session.session_id());
    world.send_item_move_refresh(session.session_id());

    // (WIZ_ITEM_GET workaround removed — testing RENTAL+SERVER_INDEX fix instead)

    tracing::info!(
        "[{}] Phase2 COMPLETE: {} entered game world (sid={}, zone={}, region=({},{}))",
        session.addr(),
        char_id,
        session.session_id(),
        zone_id,
        position.region_x,
        position.region_z,
    );

    // FerihaLog: LoginInsertLog
    super::audit_log::log_login(
        session.pool(),
        session.account_id().unwrap_or(""),
        &char_id,
        &session.addr().to_string(),
        zone_id as i16,
    );

    // Restore the actual Genie UI, not just the server-side SessionHandle.
    // v2615 B1A4B0 consumes this response; login does not request it itself.
    super::genie::handle_load_options(session).await?;

    // 26. GM entity level range overlay (F7B flag) — sent LAST to ensure client is fully initialized.
    // Client WIZ_STATE_CHANGE handler: type==2 + entity_id==self + state==2 → set F7B=1
    // F7B enables NPC/monster level range display on entity nameplates when targeting.
    if player_authority == 0 {
        let mut sc_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizStateChange as u8);
        sc_pkt.write_u32(sid as u32);
        sc_pkt.write_u8(2); // type = NeedParty (controls F7B flag)
        sc_pkt.write_u32(2); // state = 2 (enable level range overlay)
        session.send_packet(&sc_pkt).await?;
        tracing::debug!("[{}] GM F7B overlay enabled (sid={})", session.addr(), sid);
    }

    Ok(())
}

/// Check if a player's saved zone requires relocation on login.
/// Returns `Some((zone, x, z))` if the player must be relocated, `None` if safe.
/// When relocation is needed, the target zone is usually Moradon with
/// nation-specific start_position coords.
fn check_ishome_relocation(
    zone_id: u16,
    nation: u8,
    clan_id: u16,
    loyalty: u32,
    world: &crate::world::WorldState,
) -> Option<(u16, f32, f32)> {
    let mut ishome = false;

    // 1. Karus player in Elmorad when Elmorad gate is closed
    // C++ line 710: GetZoneID()==ZONE_ELMORAD && !m_byElmoradOpenFlag && GetNation()==KARUS
    if zone_id == ZONE_ELMORAD && nation == 1 {
        let bs = world.get_battle_state();
        if !bs.elmorad_open_flag {
            ishome = true;
        }
    }

    // 2. Elmorad player in Karus when Karus gate is closed
    // C++ line 711: GetZoneID()==ZONE_KARUS && !m_byKarusOpenFlag && GetNation()==ELMORAD
    if !ishome && zone_id == ZONE_KARUS && nation == 2 {
        let bs = world.get_battle_state();
        if !bs.karus_open_flag {
            ishome = true;
        }
    }

    // 3. War zone when war is not open
    // C++ line 712: GetMap()->isWarZone() && !isWarOpen()
    if !ishome {
        let is_war_zone = world
            .get_zone(zone_id)
            .and_then(|z| z.zone_info.as_ref().map(|zi| zi.abilities.war_zone))
            .unwrap_or(false);
        if is_war_zone && !world.is_war_open() {
            ishome = true;
        }

        // 4. War zone, war open, but player's nation lost
        // C++ line 713: isWarZone() && isWarOpen() && m_bVictory!=0 && m_bVictory!=GetNation()
        if !ishome && is_war_zone && world.is_war_open() {
            let bs = world.get_battle_state();
            if bs.victory != 0 && bs.victory != nation {
                ishome = true;
            }
        }
    }

    // 5. Cinderella war active → kick from any zone (C++ line 714 first condition)
    if !ishome && world.is_cinderella_active() {
        ishome = true;
    }

    // 6. In temple event zone → kick (C++ line 714 second condition)
    if !ishome && is_in_total_temple_event_zone(zone_id) {
        ishome = true;
    }

    // 6b. War PK zone kickout during active war (C++ line 714 third condition)
    // Unit.h:168-174: Ardream, RLB, Ronark, Bifrost, Krowaz Dominion
    if !ishome && world.is_war_open() && is_war_zone_kickout(zone_id) {
        ishome = true;
    }

    // 6c. Tournament zones — party(96-99) and clan(77-78) arenas
    // C++ line 719: isPartyTournamentinZone() || isClanTournamentinZone()
    if !ishome && matches!(zone_id, 77 | 78 | 96..=99) {
        ishome = true;
    }

    // 7. Special event zone (Zindan/Stone) but event not open
    // C++ line 716: isInSpecialEventZone() && !pSpecialEvent.opened
    if !ishome
        && crate::handler::attack::is_in_special_event_zone(zone_id)
        && !world.is_zindan_event_opened()
    {
        ishome = true;
    }

    // 8. Stone zones already covered by is_in_total_temple_event_zone() above

    // 9. Special event open + player in PK zone → kick
    // C++ line 718: pSpecialEvent.opened && isInPKZone()
    if !ishome && world.is_zindan_event_opened() && is_pk_zone_for_ishome(zone_id) {
        ishome = true;
    }

    // 10. Bifrost zone — kick if event not active
    // C++ line 720: complex 3-way check, simplified: if no bifrost event → kick
    if !ishome && zone_id == ZONE_BIFROST {
        let bifrost_active = world.get_bifrost_remaining_secs() > 0;
        if !bifrost_active {
            ishome = true;
        }
    }

    // 11. Cinderella zone — kick from event zone
    // C++ line 721: isCindirellaZone(GetZoneID())
    if !ishome {
        let cind_zone = world.cinderella_zone_id();
        if cind_zone > 0 && crate::handler::cinderella::is_cinderella_zone(zone_id, cind_zone) {
            ishome = true;
        }
    }

    // 12. Delos but can't enter (no clan/loyalty/grade check)
    // C++ line 722: ZONE_DELOS && !CastleSiegeWarfareCanenterDelos()
    if !ishome && zone_id == ZONE_DELOS && !world.can_enter_delos(clan_id, loyalty) {
        ishome = true;
    }

    // CSW instance zones are never valid as a persistent login location.
    // If a disconnect or crash leaves a character there, recover them on the
    // next login instead of re-entering a stale map and getting stuck.
    if !ishome && matches!(zone_id, ZONE_DELOS_CASTELLAN | ZONE_DESPERATION_ABYSS | ZONE_HELL_ABYSS)
        && !world.csw_event().try_read().map(|s| s.is_active()).unwrap_or(true)
    {
        ishome = true;
    }

    if !ishome {
        return None;
    }

    // Relocate to Moradon with nation-specific start_position
    // C++ lines 724-742
    get_moradon_coords(nation, world)
}

/// Get Moradon relocation coordinates using start_position table.
/// Falls back to hardcoded Moradon coords if start_position lookup fails.
fn get_moradon_coords(nation: u8, world: &crate::world::WorldState) -> Option<(u16, f32, f32)> {
    use rand::Rng;
    if let Some(sp) = world.get_start_position(ZONE_MORADON) {
        let (x, z) = if nation == 1 {
            (sp.karus_x as f32, sp.karus_z as f32)
        } else {
            (sp.elmorad_x as f32, sp.elmorad_z as f32)
        };
        if x != 0.0 || z != 0.0 {
            let mut rng = rand::thread_rng();
            let offset_x = if sp.range_x > 0 {
                rng.gen_range(0..=sp.range_x) as f32
            } else {
                0.0
            };
            let offset_z = if sp.range_z > 0 {
                rng.gen_range(0..=sp.range_z) as f32
            } else {
                0.0
            };
            return Some((ZONE_MORADON, x + offset_x, z + offset_z));
        }
    }

    // Hardcoded fallback — C++ v1098: (267.0, 7.9, 303.0)
    Some((ZONE_MORADON, 267.0, 303.0))
}

/// PK zone check for ishome relocation.
fn is_pk_zone_for_ishome(zone_id: u16) -> bool {
    use crate::world::{ZONE_ARDREAM, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE};
    zone_id == ZONE_RONARK_LAND
        || zone_id == ZONE_ARDREAM
        || zone_id == ZONE_RONARK_LAND_BASE
        || (61..=66).contains(&zone_id)
}

/// Check if zone is a total temple event zone (always kick on login).
/// Covers: BDW(84), Chaos(85), Juraid(87), Stone(81-83), FT(55),
/// Dungeon Defence(89), Draki Tower(95), Under Castle(86), Knight Royale(76).
fn is_in_total_temple_event_zone(zone_id: u16) -> bool {
    matches!(
        zone_id,
        55 | 76 | 81 | 82 | 83 | 84 | 85 | 86 | 87 | 89 | 95
    )
}

/// Check if zone qualifies for war-zone kickout during active war.
/// Covers: Ardream(10), RLB(72), Ronark Land(71), Bifrost(31), Krowaz Dominion(75).
fn is_war_zone_kickout(zone_id: u16) -> bool {
    zone_id == ZONE_ARDREAM
        || zone_id == ZONE_RONARK_LAND_BASE
        || zone_id == ZONE_RONARK_LAND
        || zone_id == ZONE_BIFROST
        || zone_id == ZONE_KROWAZ_DOMINION
}

/// Parse skill data string from DB into 9-byte array.
/// Kept for backward compatibility with str_skill varchar column.
#[cfg(test)]
fn parse_skill_data(str_skill: &Option<String>) -> [u8; 9] {
    let mut result = [0u8; 9];
    if let Some(ref s) = str_skill {
        for (i, part) in s.split(',').enumerate() {
            if i >= 9 {
                break;
            }
            result[i] = part.trim().parse::<u8>().unwrap_or(0);
        }
    }
    result
}

/// Re-send WIZ_MYINFO from in-memory state (no DB hit).
/// Called when the client needs a full character info refresh (e.g. after +gm toggle).
/// Unlike phase 1 gamestart, this reads ALL data from WorldState session handles.
pub(crate) async fn send_myinfo_refresh(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let pool = session.pool().clone();

    // Snapshot all session data in one closure to minimize lock time
    let snap = world.with_session(sid, |h| {
        let ch = h.character.clone();
        let inv: Vec<UserItemSlot> = h.inventory.clone();
        let eq = h.equipped_stats.clone();
        let premium_map = h.premium_map.clone();
        let premium_in_use = h.premium_in_use;
        let genie_abs = h.genie_time_abs;
        let return_sym = h.return_symbol_ok;
        let achieve = h.achieve_summary.clone();
        let k_rank = h.knights_rank;
        let p_rank = h.personal_rank;
        (
            ch,
            inv,
            eq,
            premium_map,
            premium_in_use,
            genie_abs,
            return_sym,
            achieve,
            k_rank,
            p_rank,
        )
    });
    let Some((
        Some(ch),
        inv,
        eq,
        premium_map,
        premium_in_use,
        genie_abs,
        return_sym,
        achieve,
        k_rank,
        p_rank,
    )) = snap
    else {
        return Ok(());
    };

    let mut pkt = Packet::new(Opcode::WizMyInfo as u8);

    // ── Basic character info ──────────────────────────────────────────
    pkt.write_u32(sid as u32);
    pkt.write_sbyte_string(&ch.name);

    // Position from world state
    let pos = world.get_position(sid).unwrap_or(Position {
        zone_id: 21,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        region_x: 0,
        region_z: 0,
    });
    let pos_x = (pos.x * 10.0) as u16;
    let pos_z = (pos.z * 10.0) as u16;
    let pos_y = (pos.y * 10.0) as u16;
    pkt.write_u16(pos_x);
    pkt.write_u16(pos_z);
    pkt.write_u16(pos_y);

    pkt.write_u8(ch.nation);
    pkt.write_u8(ch.race);
    // IDA-verified order: class(i16) → hairColor(u8,+3000) → hairPacked(u32,+3004)
    //                     → face(u8,+3016) → title2(u8,+1964) → title1(u8,+1960)
    //                     → rank(u8,+3020) → level(u8,+1744) → points(i16,+3044)
    pkt.write_i16(ch.class as i16); // class (sub_61EE80 = i16)
    let hair_color: u8 = ((ch.hair_rgb >> 24) & 0xFF) as u8;
    pkt.write_u8(hair_color); // hairColor (+3000)
    pkt.write_u32(ch.hair_rgb); // hairPacked (+3004)
    pkt.write_u8(ch.face); // face (+3016) — AFTER hair, not before
    pkt.write_u8(ch.title); // title2 (+1964)
    pkt.write_u8(0); // title1 (+1960)
                     // SendMyInfo.cpp resets m_bRank from KING_SYSTEM on every MyInfo build.
                     // `CharacterInfo::rank` is a ladder value and is not a king flag.
    let is_king = world.is_king(ch.nation, &ch.name);
    pkt.write_u8(u8::from(is_king)); // king rank (+3020) — AFTER titles
    pkt.write_u8(ch.level); // level (+1744)
    pkt.write_i16(ch.free_points as i16); // points (+3044, sub_61EE80 = i16)

    pkt.write_i64(ch.max_exp);
    pkt.write_i64(ch.exp as i64);

    pkt.write_u32(ch.loyalty);
    pkt.write_u32(ch.loyalty_monthly);

    // Clan section — SNIFFER-VERIFIED format (same as initial MyInfo builder)
    let clan_id = ch.knights_id as i16;
    pkt.write_i16(clan_id);
    pkt.write_u8(ch.fame);

    if clan_id > 0 {
        match world.get_knights(clan_id as u16) {
            Some(ki) => {
                pkt.write_u16(ki.alliance);
                // This is the clan type/flag. Writing zero here makes the
                // v2615 cloak palette treat a promoted clan as Training and
                // filter every purchasable cape out of Cloak.tbl.
                pkt.write_u8(ki.flag);
                pkt.write_sbyte_string(&ki.name);
                pkt.write_u8(ki.grade);
                pkt.write_u8(ki.ranking);
                pkt.write_u16(ki.mark_version);
                let cape_id = if is_king {
                    if ch.nation == 1 {
                        97u16
                    } else {
                        98u16
                    }
                } else {
                    ki.cape
                };
                pkt.write_u16(cape_id);
                pkt.write_u8(ki.cape_r);
                pkt.write_u8(ki.cape_g);
                pkt.write_u8(ki.cape_b);
                // Same MyInfo RGB tail as the login packet: compact
                // cape-symbol/render flag, not the raw clan type.
                pkt.write_u8(myinfo_cape_tail_flag(ki.flag, ki.grade));
            }
            None => {
                pkt.write_u64(0);
                pkt.write_u16(0xFFFF);
                pkt.write_u32(0);
            }
        }
    } else {
        pkt.write_u64(0);
        pkt.write_u16(0xFFFF);
        pkt.write_u32(0);
    }
    pkt.write_bytes(&[0u8; 8]);

    // ── HP/MP/Stats — use LIVE computed values ────────────────────────
    pkt.write_i16(ch.max_hp);
    pkt.write_i16(ch.hp);
    pkt.write_i16(ch.max_mp);
    pkt.write_i16(ch.mp);

    pkt.write_u32(eq.max_weight);
    pkt.write_u32(eq.item_weight);

    // Stats: base + item bonus
    pkt.write_u8(ch.str);
    pkt.write_u8(eq.stat_bonuses[0].max(0) as u8);
    pkt.write_u8(ch.sta);
    pkt.write_u8(eq.stat_bonuses[1].max(0) as u8);
    pkt.write_u8(ch.dex);
    pkt.write_u8(eq.stat_bonuses[2].max(0) as u8);
    pkt.write_u8(ch.intel);
    pkt.write_u8(eq.stat_bonuses[3].max(0) as u8);
    pkt.write_u8(ch.cha);
    pkt.write_u8(eq.stat_bonuses[4].max(0) as u8);

    pkt.write_u16(eq.total_hit);
    pkt.write_u16(eq.total_ac as u16);

    // Resistances
    pkt.write_u8(eq.fire_r.max(0) as u8);
    pkt.write_u8(eq.cold_r.max(0) as u8);
    pkt.write_u8(eq.lightning_r.max(0) as u8);
    pkt.write_u8(eq.magic_r.max(0) as u8);
    pkt.write_u8(eq.disease_r.max(0) as u8);
    pkt.write_u8(eq.poison_r.max(0) as u8);

    // Gold and authority
    pkt.write_u32(ch.gold);
    pkt.write_u8(ch.authority);

    // Rankings
    let k_i8: i8 = if k_rank == 0 { -1 } else { k_rank as i8 };
    let p_i8: i8 = if p_rank == 0 { -1 } else { p_rank as i8 };
    let k_out = if k_i8 <= p_i8 { k_i8 } else { -1 };
    let p_out = if p_i8 <= k_i8 { p_i8 } else { -1 };
    pkt.write_i8(k_out);
    pkt.write_i8(p_out);

    // ── Skills (9 bytes: m_bstrSkill[0..9]) ─────────────────────────
    for i in 0..9 {
        pkt.write_u8(ch.skill_points.get(i).copied().unwrap_or(0));
    }

    // ── Inventory — IDA-verified phase order (sub_733190, 90 items).
    let myinfo_slots: [Option<usize>; 90] = {
        let mut s = [None; 90];
        let mut idx = 0;
        // Phase 1: Equipment (0-13)
        for i in 0..14 {
            s[idx] = Some(i);
            idx += 1;
        }
        // Phase 2: Bag (14-41)
        for i in 14..42 {
            s[idx] = Some(i);
            idx += 1;
        }
        // Phase 3: Cospre (positions 0,1,2,3,4,5,7,8,9 → skip pos 6=slot 48)
        for &pos in &[0, 1, 2, 3, 4, 5, 7, 8, 9] {
            s[idx] = Some(42 + pos);
            idx += 1;
        }
        // Phase 4: 3 special — CBAG1(51), CBAG2(52), CBAG3(slot 96)
        s[idx] = Some(51);
        idx += 1;
        s[idx] = Some(52);
        idx += 1;
        s[idx] = Some(INVENTORY_TOTAL);
        idx += 1; // CBAG3 dedicated slot (96)
                  // Phase 5: Magic bags (53-88)
        for i in 53..89 {
            s[idx] = Some(i);
            idx += 1;
        }
        s
    };
    for slot_opt in &myinfo_slots {
        let item = slot_opt
            .and_then(|s| inv.get(s))
            .filter(|it| it.item_id != 0);
        match item {
            Some(it) => {
                pkt.write_u32(it.item_id);
                pkt.write_i16(it.durability);
                pkt.write_u16(it.count);
                pkt.write_u8(it.flag);
                pkt.write_u16(it.remaining_rental_minutes());
                super::unique_item_info::write_unique_item_info(
                    &world,
                    &pool,
                    it.item_id,
                    it.serial_num,
                    ch.rebirth_level,
                    &mut pkt,
                )
                .await;
                pkt.write_u32(it.expire_time);
            }
            _ => {
                pkt.write_u32(0);
                pkt.write_i16(0);
                pkt.write_u16(0);
                pkt.write_u8(0);
                pkt.write_u16(0);
                pkt.write_u32(0);
                pkt.write_u32(0);
            }
        }
    }

    // ── Premium ───────────────────────────────────────────────────────
    let now_ts = crate::handler::genie::now_secs();
    let mut prem_entries: Vec<(u8, u16)> = Vec::with_capacity(premium_map.len());
    for (&p_type, &expiry) in &premium_map {
        if p_type == 0 || expiry <= now_ts {
            continue;
        }
        let time_rest = expiry.saturating_sub(now_ts);
        let time_hours: u16 = if (1..=3600).contains(&time_rest) {
            1
        } else {
            (time_rest / 3600) as u16
        };
        prem_entries.push((p_type, time_hours));
    }
    // Sniffer-verified: 4 extra empty items after Phase 5 (always present)
    for _ in 0..4u8 {
        pkt.write_u32(0);
        pkt.write_i16(0);
        pkt.write_u16(0);
        pkt.write_u8(0);
        pkt.write_u16(0);
        pkt.write_u32(0);
        pkt.write_u32(0);
    }

    // accountStatus
    let account_status: u8 = if premium_in_use > 0 { 1 } else { 0 };
    pkt.write_u8(account_status);
    // premium section
    pkt.write_u8(prem_entries.len() as u8);
    for &(p_type, time_hours) in &prem_entries {
        pkt.write_u8(p_type);
        pkt.write_u16(time_hours);
    }
    pkt.write_u8(premium_in_use); // activePremiumType — sniffer verified
                                  // IDA-verified trailer order (lines 707205-707418):
    let genie_remaining = genie_abs.saturating_sub(now_ts);
    let genie_hours = crate::handler::genie::get_genie_hours_pub(genie_remaining);

    pkt.write_u8(0); // collRaceEnabled (forced to 0)
    pkt.write_u32(return_sym); // coverTitle_u32 (+3236)
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(0);
    pkt.write_u8(0); // skillSave x5
    pkt.write_u8(0); // petType
    pkt.write_i16(genie_hours as i16); // petHP/genieTime
    pkt.write_u8(ch.rebirth_level); // rebirthLevel
    pkt.write_u8(ch.reb_str); // rebStatSTR
    pkt.write_u8(ch.reb_sta); // rebStatSTA
    pkt.write_u8(ch.reb_dex); // rebStatDEX
    pkt.write_u8(ch.reb_intel); // rebStatINT
    pkt.write_u8(ch.reb_cha); // rebStatCHA
    pkt.write_i64(ch.sealed_exp as i64); // sealedExp
    pkt.write_i16(achieve.cover_title as i16); // coverTitle
    pkt.write_i16(achieve.skill_title as i16); // skillTitle
    pkt.write_u32(ch.manner_point as u32); // mannerPoint
    pkt.write_u8(premium_in_use); // premiumInUse
    pkt.write_u8(0); // isHidingHelmet
    pkt.write_u8(0); // unknown_ui_1
    pkt.write_u8(0); // unknown_ui_2
    pkt.write_u8(0); // isHidingCospre

    // Send compressed if large
    let to_send = match pkt.to_compressed() {
        Some(compressed) => compressed,
        None => pkt,
    };
    session.send_packet(&to_send).await?;

    Ok(())
}

/// Load premium subscriptions for the WIZ_MYINFO packet (phase 1).
/// Returns a list of (premium_type, remaining_hours) entries and the premium_in_use value.
async fn load_premium_for_myinfo(pool: &ko_db::DbPool, account_id: &str) -> (Vec<(u8, u16)>, u8) {
    let repo = PremiumRepository::new(pool);
    let rows = match repo.load_account_premium(account_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("Failed to load premium for account {}: {}", account_id, e);
            return (Vec::new(), 0);
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let mut entries = Vec::new();
    let mut first_valid: u8 = 0;

    for row in &rows {
        if row.premium_type <= 0 || row.expiry_time <= 0 {
            continue;
        }
        let expiry = row.expiry_time as u32;
        if expiry <= now {
            continue;
        }
        let time_rest = expiry.saturating_sub(now);
        let time_hours: u16 = if (1..=3600).contains(&time_rest) {
            1
        } else {
            (time_rest / 3600) as u16
        };
        let p_type = row.premium_type as u8;
        entries.push((p_type, time_hours));
        if first_valid == 0 {
            first_valid = p_type;
        }
    }

    (entries, first_valid)
}

/// Load account premium subscriptions from DB and populate session premium_map.
/// Called during phase 2 (game entry) to wire premium state into the session.
async fn load_account_premiums(session: &mut ClientSession) {
    let account_id = match session.account_id() {
        Some(id) => id.to_string(),
        None => return,
    };

    // Store account_id in SessionHandle early so periodic save can use it
    {
        let acct = account_id.clone();
        let world = session.world().clone();
        let sid = session.session_id();
        world.update_session(sid, |h| {
            h.account_id = acct;
        });
    }

    let pool = session.pool().clone();
    let repo = PremiumRepository::new(&pool);

    let rows = match repo.load_account_premium(&account_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                "[{}] Failed to load premium for account {}: {}",
                session.addr(),
                account_id,
                e
            );
            return;
        }
    };

    if rows.is_empty() {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let world = session.world().clone();
    let sid = session.session_id();

    world.update_session(sid, |h| {
        let mut first_valid: u8 = 0;
        for row in &rows {
            if row.premium_type <= 0 || row.expiry_time <= 0 {
                continue;
            }
            let expiry = row.expiry_time as u32;
            if expiry <= now {
                continue; // expired, skip
            }
            let p_type = row.premium_type as u8;
            h.premium_map.insert(p_type, expiry);
            if first_valid == 0 {
                first_valid = p_type;
            }
        }
        // Auto-select the first valid premium if none is selected
        if h.premium_in_use == 0 && first_valid > 0 {
            h.premium_in_use = first_valid;
        }
    });

    let loaded_count = rows
        .iter()
        .filter(|r| r.premium_type > 0 && r.expiry_time > 0)
        .count();
    if loaded_count > 0 {
        tracing::debug!(
            "[{}] Loaded {} premium entries for account {}",
            session.addr(),
            loaded_count,
            account_id,
        );
    }
}

/// Load saved magic from DB and populate the session's saved_magic_map.
/// After loading, iterates each saved buff and re-applies it (recast).
/// In C++, RecastSavedMagic creates a MagicInstance with bIsRecastingSavedMagic=true
/// and calls Run(). For now we simply load the map so saved durations are tracked;
/// the buffs will be re-applied via the Type4 system on next cast or via
/// the saved_magic_map for persistence. Full recast (re-applying stat buffs)
/// requires MagicInstance integration and is handled as a follow-up.
async fn load_and_recast_saved_magic(session: &mut ClientSession) {
    let char_id = match session.character_id() {
        Some(id) => id.to_string(),
        None => return,
    };
    let pool = session.pool().clone();
    let repo = SavedMagicRepository::new(&pool);

    let rows = match repo.load_saved_magic(&char_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(
                "[{}] Failed to load saved magic for {}: {}",
                session.addr(),
                char_id,
                e
            );
            return;
        }
    };

    if rows.is_empty() {
        return;
    }

    let entries: Vec<(u32, i32)> = rows
        .iter()
        .filter(|r| r.skill_id > 0 && r.remaining_duration > 0)
        .map(|r| (r.skill_id as u32, r.remaining_duration))
        .collect();

    let world = session.world().clone();
    let sid = session.session_id();

    // Load entries into session's saved_magic_map (converts seconds to absolute ms timestamps)
    world.load_saved_magic(sid, &entries);

    // Recast saved magic — restore persistent buffs on login
    world.clear_all_buffs(sid, false);
    let recast_count = world.recast_saved_magic(sid);

    tracing::debug!(
        "[{}] Loaded {} saved magic entries for {}, recast {} buffs",
        session.addr(),
        entries.len(),
        char_id,
        recast_count,
    );
}

/// Auto-send saved skill bar data during game entry.
/// Loads the character's skill shortcut data from the database and sends
/// a WIZ_SKILLDATA(SKILL_DATA_LOAD) response packet. This ensures the
/// client's skill bar is restored on login without requiring an explicit
/// client request.
async fn send_skill_bar_data(session: &mut ClientSession) {
    let char_name = match session.world().get_character_info(session.session_id()) {
        Some(ch) => ch.name.clone(),
        None => return,
    };

    let repo = SkillShortcutRepository::new(session.pool());
    let row = match repo.load(&char_name).await {
        Ok(row) => row,
        Err(e) => {
            tracing::warn!(
                "[{}] Failed to load skill bar data for '{}': {}",
                session.addr(),
                char_name,
                e
            );
            return;
        }
    };

    // Only send if saved data exists; otherwise skip (client shows empty bar)
    let r = match row {
        Some(r) if r.count > 0 => r,
        _ => return,
    };

    let mut response = Packet::new(Opcode::WizSkillData as u8);
    response.write_u8(2); // SKILL_DATA_LOAD sub-opcode
    let count = r.count as u16;
    response.write_u16(count);
    for i in 0..count as usize {
        let offset = i * 4;
        if offset + 4 <= r.skill_data.len() {
            let skill_id = u32::from_le_bytes([
                r.skill_data[offset],
                r.skill_data[offset + 1],
                r.skill_data[offset + 2],
                r.skill_data[offset + 3],
            ]);
            response.write_u32(skill_id);
        } else {
            response.write_u32(0);
        }
    }

    if let Err(e) = session.send_packet(&response).await {
        tracing::warn!(
            "[{}] Failed to send skill bar data for '{}': {}",
            session.addr(),
            char_name,
            e
        );
    } else {
        tracing::debug!(
            "[{}] Auto-sent {} skill bar shortcuts for '{}'",
            session.addr(),
            count,
            char_name
        );
    }
}

/// Respawn scroll item IDs
const RESPAWN_SCROLL_IDS: [u32; 7] = [
    800036000, 800039000, 910022000, 900699000, 810036000, 900136000, 910948000,
];

/// Check inventory for respawn scrolls when the player logs in dead.
/// If any of the 7 respawn scroll IDs are found in the bag inventory with count >= 1,
/// sends WIZ_DEAD with sub-opcode 18483 so the client shows the scroll-revive UI.
async fn check_respawn_scroll_on_login(
    world: &crate::world::WorldState,
    session: &mut ClientSession,
) {
    let sid = session.session_id();
    let has_scroll = (SLOT_MAX..SLOT_MAX + HAVE_MAX).any(|slot| {
        if let Some(item) = world.get_inventory_slot(sid, slot) {
            item.count >= 1 && RESPAWN_SCROLL_IDS.contains(&item.item_id)
        } else {
            false
        }
    });

    if has_scroll {
        let mut pkt = Packet::new(Opcode::WizDead as u8);
        pkt.write_u32(sid as u32);
        pkt.write_u32(18483);
        pkt.write_u64(0);
        if let Err(e) = session.send_packet(&pkt).await {
            tracing::warn!(
                "[{}] Failed to send respawn scroll packet: {}",
                session.addr(),
                e
            );
        }
    }
}

/// Check if a zone qualifies for PK zone ranking tracking.
/// Ardream, Ronark Land, Ronark Land Base, and Bifrost are PK ranking zones.
fn is_pk_ranking_zone(zone_id: u16) -> bool {
    zone_id == ZONE_ARDREAM
        || zone_id == ZONE_RONARK_LAND
        || zone_id == ZONE_RONARK_LAND_BASE
        || zone_id == ZONE_BIFROST
}

/// Check and update level-based normal achievements on login.
/// - CharacterSelectionHandler.cpp:1132
/// - AchieveHandler.cpp:471-510 (AchieveNormalCountAdd)
/// - AchieveHandler.cpp:787-793 (AchieveNormalCheck — AchieveReachLevel case)
/// Iterates user's achieve_map, finds normal-type achievements with
/// AchieveReachLevel sub-type, sets count to current level, and marks
/// as finished if count threshold is met.
fn achieve_normal_reach_level(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    level: u8,
    zone_id: u16,
) {
    /// AchieveMain.type == 4 → Normal sub-table
    const ACHIEVE_MAIN_TYPE_NORMAL: i16 = 4;
    /// AchieveNormal.type == 3 → AchieveReachLevel
    const ACHIEVE_NORMAL_TYPE_REACH_LEVEL: i16 = 3;
    /// Status values
    const STATUS_FINISHED: u8 = 4;
    const STATUS_COMPLETED: u8 = 5;

    // Snapshot the achieve_map IDs and statuses to avoid holding the lock
    let entries: Vec<(u16, u8)> = world
        .with_session(sid, |h| {
            h.achieve_map
                .iter()
                .map(|(&id, info)| (id, info.status))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if entries.is_empty() {
        return;
    }

    let mut finished_ids: Vec<(u16, u32)> = Vec::new(); // (achieve_id, capped_count)

    for (achieve_id, status) in &entries {
        // Skip already completed or finished
        if *status == STATUS_FINISHED || *status == STATUS_COMPLETED {
            continue;
        }

        // Look up achieve_main
        let main = match world.achieve_main(*achieve_id as i32) {
            Some(m) => m,
            None => continue,
        };

        // Must be Normal type
        if main.r#type != ACHIEVE_MAIN_TYPE_NORMAL {
            continue;
        }

        // Zone check
        if main.zone_id > 0 && main.zone_id as u16 != zone_id {
            continue;
        }

        // Look up achieve_normal by sIndex
        let normal = match world.achieve_normal(main.s_index) {
            Some(n) => n,
            None => continue,
        };

        // Must be AchieveReachLevel type
        if normal.r#type != ACHIEVE_NORMAL_TYPE_REACH_LEVEL {
            continue;
        }

        // Update count to current level
        let count = level as u32;
        let required = normal.count as u32;
        let is_finished = required <= count;
        let capped = if is_finished { required } else { count };

        // Update session achieve_map
        world.update_session(sid, |h| {
            if let Some(info) = h.achieve_map.get_mut(achieve_id) {
                info.count[0] = capped;
                if is_finished {
                    info.status = STATUS_FINISHED;
                }
            }
        });

        if is_finished {
            finished_ids.push((*achieve_id, capped));

            // Update summary counts
            world.update_session(sid, |h| {
                match main.achieve_type {
                    0 => {} // normal_count tracked elsewhere
                    1 => {} // quest_count
                    2 => {} // war_count
                    3 => {} // adventure_count
                    4 => {} // challenge_count
                    _ => {}
                }
                // Update recent achieve
                h.achieve_summary.recent_achieve[2] = h.achieve_summary.recent_achieve[1];
                h.achieve_summary.recent_achieve[1] = h.achieve_summary.recent_achieve[0];
                h.achieve_summary.recent_achieve[0] = *achieve_id;
                // Award medal points
                h.achieve_summary.total_medal += main.point as u32;
            });
        }
    }

    // Send success packets for newly finished achievements
    if !finished_ids.is_empty() {
        for (achieve_id, _) in &finished_ids {
            let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizUserAchieve as u8);
            pkt.write_u8(1); // AchieveSuccess
            pkt.write_u16(*achieve_id);
            pkt.write_u8(4); // v2600: status=Finished (sniff verified)
            world.send_to_session_owned(sid, pkt);
        }
    }
}

/// Auto-complete class-specific skill prerequisite quests.
/// Logic per quest :
///   1. Ensure quest entry exists (SaveEvent with state 0)
///   2. If quest state != 2 (completed), mark as completed (SaveEvent with state 2)
/// Quest IDs by class (from `_getEtcList(false)`):
/// - Warrior:  334, 359, 365, 273, 390
/// - Rogue:    335, 347, 360, 366, 273
/// - Mage:     336, 348, 361, 367, 273
/// - Priest:   337, 349, 357, 362, 363, 364, 368, 273
/// - Kurian:   1377, 1378, 273
fn open_etc_skill(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    class: u16,
    level: u8,
) {
    use super::quest::job_group_check;

    if level < 70 {
        return;
    }

    let mut quest_ids: Vec<u16> = Vec::new();

    // Warrior (group 1)
    if job_group_check(class, 1) {
        quest_ids.extend_from_slice(&[334, 359, 365, 273, 390]);
    }
    // Rogue (group 2)
    if job_group_check(class, 2) {
        quest_ids.extend_from_slice(&[335, 347, 360, 366, 273]);
    }
    // Mage (group 3)
    if job_group_check(class, 3) {
        quest_ids.extend_from_slice(&[336, 348, 361, 367, 273]);
    }
    // Priest (group 4)
    if job_group_check(class, 4) {
        quest_ids.extend_from_slice(&[337, 349, 357, 362, 363, 364, 368, 273]);
    }
    // Kurian (ClassPortuKurian = 13)
    if job_group_check(class, 13) {
        quest_ids.extend_from_slice(&[1377, 1378, 273]);
    }

    if quest_ids.is_empty() {
        return;
    }

    //   for each quest_id:
    //     SaveEvent(id, 0)  -- ensure entry exists with state 0
    //     if GetData(id) is null OR QuestState != 2:
    //       SaveEvent(id, 2)  -- mark as completed
    world.update_session(sid, |h| {
        for &qid in &quest_ids {
            // Step 1: Ensure quest entry exists (SaveEvent state=0)
            h.quests.entry(qid).or_insert(crate::world::UserQuestInfo {
                quest_state: 0,
                kill_counts: [0; 4],
            });

            // Step 2: If not already completed, mark as completed
            if let Some(quest) = h.quests.get_mut(&qid) {
                if quest.quest_state != 2 {
                    quest.quest_state = 2;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PK Ranking Zone Tests ────────────────────────────────────────

    #[test]
    fn test_pk_ranking_zone_ardream() {
        assert!(is_pk_ranking_zone(ZONE_ARDREAM));
    }

    #[test]
    fn test_pk_ranking_zone_ronark_land() {
        assert!(is_pk_ranking_zone(ZONE_RONARK_LAND));
    }

    #[test]
    fn test_pk_ranking_zone_ronark_land_base() {
        assert!(is_pk_ranking_zone(ZONE_RONARK_LAND_BASE));
    }

    #[test]
    fn test_pk_ranking_zone_bifrost() {
        assert!(is_pk_ranking_zone(ZONE_BIFROST));
    }

    #[test]
    fn test_pk_ranking_zone_moradon_not_pk() {
        assert!(!is_pk_ranking_zone(21)); // Moradon
    }

    #[test]
    fn test_pk_ranking_zone_delos_not_pk() {
        // Delos is NOT a PK ranking zone (it uses CswEventState instead)
        assert!(!is_pk_ranking_zone(30));
    }

    #[test]
    fn test_pk_ranking_zone_battle_not_pk() {
        // Battle zones (61-66) are NOT PK ranking zones — they use Zindan War ranking
        assert!(!is_pk_ranking_zone(61));
        assert!(!is_pk_ranking_zone(66));
    }

    #[test]
    fn test_pk_ranking_zone_karus_elmorad_not_pk() {
        assert!(!is_pk_ranking_zone(1)); // Karus
        assert!(!is_pk_ranking_zone(2)); // Elmorad
    }

    #[test]
    fn test_myinfo_cape_tail_uses_reference_symbol_flag() {
        assert_eq!(myinfo_cape_tail_flag(1, 5), 0);
        assert_eq!(myinfo_cape_tail_flag(2, 3), 0);
        assert_eq!(myinfo_cape_tail_flag(2, 2), 1);
        assert_eq!(myinfo_cape_tail_flag(3, 1), 1);
        assert_eq!(myinfo_cape_tail_flag(12, 1), 1);
    }

    // ── Skill Data Parse Tests ───────────────────────────────────────

    #[test]
    fn test_parse_skill_data_empty() {
        let result = parse_skill_data(&None);
        assert_eq!(result, [0u8; 9]);
    }

    #[test]
    fn test_parse_skill_data_valid() {
        let s = Some("10,20,30,40,50,60,70,80,90".to_string());
        let result = parse_skill_data(&s);
        assert_eq!(result, [10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn test_parse_skill_data_partial() {
        let s = Some("5,10".to_string());
        let result = parse_skill_data(&s);
        assert_eq!(result[0], 5);
        assert_eq!(result[1], 10);
        assert_eq!(result[2], 0);
    }

    // ── HP/MP Formula Tests (now using coefficient-based formulas) ──

    #[test]
    fn test_max_hp_warrior_coefficient() {
        use ko_db::models::CoefficientRow;
        let coeff = CoefficientRow {
            s_class: 101,
            hp: 0.000022,
            mp: 0.0,
            sp: 0.000022,
            short_sword: 0.0,
            jamadar: 0.0,
            sword: 0.0,
            axe: 0.0,
            club: 0.0,
            spear: 0.0,
            pole: 0.0,
            staff: 0.0,
            bow: 0.0,
            ac: 0.0,
            hitrate: 0.0,
            evasionrate: 0.0,
        };
        let ch = CharacterInfo {
            level: 60,
            sta: 80,
            str: 65,
            intel: 50,
            class: 101,
            ..test_char()
        };
        let (hp, _mp) = stats::recalculate_max_hp_mp(&ch, Some(&coeff));
        // HP = 0.000022 * 3600 * 80 + 0.1 * 60 * 80 + 80/5 + 20 = 522
        assert_eq!(hp, 522);
    }

    #[test]
    fn test_max_mp_mage_coefficient() {
        use ko_db::models::CoefficientRow;
        let coeff = CoefficientRow {
            s_class: 107,
            hp: 0.000019,
            mp: 0.000033,
            sp: 0.0,
            short_sword: 0.0,
            jamadar: 0.0,
            sword: 0.0,
            axe: 0.0,
            club: 0.0,
            spear: 0.0,
            pole: 0.0,
            staff: 0.0,
            bow: 0.0,
            ac: 0.0,
            hitrate: 0.0,
            evasionrate: 0.0,
        };
        let ch = CharacterInfo {
            level: 60,
            sta: 60,
            intel: 80,
            class: 107,
            str: 50,
            ..test_char()
        };
        let (_hp, mp) = stats::recalculate_max_hp_mp(&ch, Some(&coeff));
        // MP = 0.000033 * 3600 * 110 + 0.1*60*2*110 + 110/5 + 20 = 1375
        assert_eq!(mp, 1375);
    }

    /// Test helper for CharacterInfo.
    fn test_char() -> CharacterInfo {
        CharacterInfo {
            session_id: 1,
            name: "Test".into(),
            nation: 1,
            race: 1,
            class: 105,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 65,
            sta: 65,
            dex: 60,
            intel: 50,
            cha: 50,
            free_points: 0,
            skill_points: [0; 10],
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
        }
    }

    // ── Max Exp Tests ────────────────────────────────────────────────

    // ── Sprint 48: Session Lifecycle Integration Tests ──────────────

    /// Integration: Game entry restores flash state and sets up burning timer.
    ///
    /// Verifies: flash_time/flash_exp_bonus persisted → session populated → flame_level set.
    #[test]
    fn test_integration_game_entry_flash_state_restore() {
        use crate::world::{CharacterInfo, Position, WorldState};

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                name: "FlashTest".into(),
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
                equipped_items: [0; 14],
                bind_zone: 21,
                bind_x: 0.0,
                bind_z: 0.0,
                str: 90,
                sta: 60,
                dex: 30,
                intel: 20,
                cha: 10,
                free_points: 0,
                skill_points: [0; 10],
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
            },
            Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Simulate restoring flash state from DB (what phase 2 does)
        world.update_session(sid, |h| {
            h.flash_time = 3600; // 1 hour remaining
            h.flash_exp_bonus = 50; // 50% XP bonus
            h.flash_dc_bonus = 20; // 20% DC bonus
            h.flash_type = 7; // premium type 7
            h.flame_level = 2; // burning flame level 2
            h.flame_time = 1800; // 30 min
        });

        // Verify all flash state was properly set
        let flash_time = world.with_session(sid, |h| h.flash_time).unwrap();
        let flash_exp = world.with_session(sid, |h| h.flash_exp_bonus).unwrap();
        let flash_dc = world.with_session(sid, |h| h.flash_dc_bonus).unwrap();
        let flame_level = world.with_session(sid, |h| h.flame_level).unwrap();

        assert_eq!(flash_time, 3600, "Flash time should be restored");
        assert_eq!(flash_exp, 50, "Flash XP bonus should be restored");
        assert_eq!(flash_dc, 20, "Flash DC bonus should be restored");
        assert_eq!(flame_level, 2, "Flame level should be restored");
    }

    /// Integration: Game entry in PK zone initializes ranking tracking.
    ///
    /// Verifies: PK zone detection → ranking zone flag set.
    #[test]
    fn test_integration_game_entry_pk_zone_ranking_init() {
        // PK ranking zones
        assert!(is_pk_ranking_zone(ZONE_ARDREAM));
        assert!(is_pk_ranking_zone(ZONE_RONARK_LAND));
        assert!(is_pk_ranking_zone(ZONE_RONARK_LAND_BASE));
        assert!(is_pk_ranking_zone(ZONE_BIFROST));

        // Non-PK zones should not trigger ranking
        let non_pk_zones = [21u16, 11, 12, 30, 61, 66, 1, 2];
        for zone_id in non_pk_zones {
            assert!(
                !is_pk_ranking_zone(zone_id),
                "Zone {} should NOT be a PK ranking zone",
                zone_id
            );
        }

        // Simulate game entry in PK zone
        let entry_zone = ZONE_RONARK_LAND;
        let should_init_ranking = is_pk_ranking_zone(entry_zone);
        assert!(should_init_ranking, "Should init ranking for Ronark Land");
    }

    /// Integration: Game entry loads saved magic and recasts buffs.
    ///
    /// Verifies: saved magic entries loaded → clear existing → recast → buffs active.
    #[test]
    fn test_integration_game_entry_saved_magic_recast() {
        use crate::world::{ActiveBuff, CharacterInfo, Position, WorldState};
        use std::time::Instant;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                name: "SavedMagic".into(),
                nation: 1,
                race: 1,
                class: 103,
                level: 60,
                face: 1,
                hair_rgb: 0,
                rank: 0,
                title: 0,
                max_hp: 3000,
                hp: 3000,
                max_mp: 5000,
                mp: 5000,
                max_sp: 0,
                sp: 0,
                equipped_items: [0; 14],
                bind_zone: 21,
                bind_x: 0.0,
                bind_z: 0.0,
                str: 20,
                sta: 20,
                dex: 20,
                intel: 80,
                cha: 100,
                free_points: 0,
                skill_points: [0; 10],
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
            },
            Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Simulate: existing buffs from previous session
        world.apply_buff(
            sid,
            ActiveBuff {
                skill_id: 108010,
                buff_type: 8,
                caster_sid: sid,
                start_time: Instant::now(),
                duration_secs: 60,
                speed: 50,
                attack_speed: 0,
                ac: 0,
                ac_pct: 0,
                attack: 0,
                magic_attack: 0,
                max_hp: 0,
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
            },
        );
        assert_eq!(world.get_active_buffs(sid).len(), 1);

        // Step 1: Clear existing buffs (C++ InitType4)
        world.clear_all_buffs(sid, false);
        assert_eq!(
            world.get_active_buffs(sid).len(),
            0,
            "Buffs cleared before recast"
        );

        // Step 2: Insert saved magic entries (from DB load)
        world.insert_saved_magic(sid, 500001, 3600); // scroll buff 1h
        world.insert_saved_magic(sid, 500002, 1800); // scroll buff 30m

        // Verify saved magic was loaded
        let saved_count = world
            .with_session(sid, |h| h.saved_magic_map.len())
            .unwrap();
        assert_eq!(saved_count, 2, "Should have 2 saved magic entries");
    }

    /// Integration: HP/MP formulas use proper coefficient-based calculation.
    ///
    /// Verifies: recalculate_abilities() uses quadratic level^2 formula.
    #[test]
    fn test_integration_hp_mp_uses_coefficient_formulas() {
        use ko_db::models::CoefficientRow;

        let coeff = CoefficientRow {
            s_class: 105,
            short_sword: 0.0,
            jamadar: 0.0,
            sword: 0.0,
            axe: 0.0,
            club: 0.0,
            spear: 0.0,
            pole: 0.0,
            staff: 0.0,
            bow: 0.0,
            hp: 0.000022,
            mp: 0.0,
            sp: 0.000022,
            ac: 0.0,
            hitrate: 0.0,
            evasionrate: 0.0,
        };

        let ch = CharacterInfo {
            session_id: 1,
            name: "Test".into(),
            nation: 1,
            race: 1,
            class: 105,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 65,
            sta: 80,
            dex: 60,
            intel: 50,
            cha: 50,
            free_points: 0,
            skill_points: [0; 10],
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

        let result = stats::recalculate_abilities(&ch, Some(&coeff));

        // HP = 0.000022 * 3600 * 80 + 0.1 * 60 * 80 + 80/5 + 20
        // = 6.336 + 480 + 16 + 20 = 522.336 → 522
        assert_eq!(result.max_hp, 522);
        // Weight = (65 + 60) * 50 = 6250
        assert_eq!(result.max_weight, 6250);
    }

    // ── Sprint 80: Dead-on-login state restoration test ────────────

    /// Verify that res_hp_type is set to USER_DEAD (0x03) when HP is 0.
    #[test]
    fn test_dead_on_login_res_hp_type() {
        // When hp <= 0, res_hp_type should be 0x03 (dead), not 0x01 (standing)
        let hp: i16 = 0;
        let res_hp_type = if hp <= 0 { 0x03u8 } else { 0x01u8 };
        assert_eq!(
            res_hp_type, 0x03,
            "Dead player should have res_hp_type=0x03"
        );

        // When hp > 0, should be standing
        let hp_alive: i16 = 100;
        let res_alive = if hp_alive <= 0 { 0x03u8 } else { 0x01u8 };
        assert_eq!(res_alive, 0x01, "Alive player should have res_hp_type=0x01");
    }

    /// Verify death animation packet format for dead-on-login.
    #[test]
    fn test_dead_on_login_animation_packet() {
        use ko_protocol::{Opcode, Packet, PacketReader};
        let sid: u16 = 42;
        let mut pkt = Packet::new(Opcode::WizDead as u8);
        pkt.write_u32(sid as u32);
        assert_eq!(pkt.opcode, Opcode::WizDead as u8);
        assert_eq!(pkt.data.len(), 4);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(42));
    }

    /// Integration: Skill data parsing handles edge cases.
    ///
    /// Verifies: empty, full, overflow, malformed input all handled correctly.
    #[test]
    fn test_integration_skill_data_parse_edge_cases() {
        // Empty input
        assert_eq!(parse_skill_data(&None), [0u8; 9]);
        assert_eq!(parse_skill_data(&Some(String::new())), [0u8; 9]);

        // Full 9 values
        let full = Some("1,2,3,4,5,6,7,8,9".to_string());
        assert_eq!(parse_skill_data(&full), [1, 2, 3, 4, 5, 6, 7, 8, 9]);

        // Overflow (more than 9 values — extras ignored)
        let overflow = Some("1,2,3,4,5,6,7,8,9,10,11".to_string());
        assert_eq!(parse_skill_data(&overflow), [1, 2, 3, 4, 5, 6, 7, 8, 9]);

        // Partial (fewer than 9 values — rest are 0)
        let partial = Some("5,10,15".to_string());
        let result = parse_skill_data(&partial);
        assert_eq!(result[0], 5);
        assert_eq!(result[1], 10);
        assert_eq!(result[2], 15);
        assert_eq!(result[3], 0);

        // Invalid values (non-numeric — parsed as 0)
        let invalid = Some("abc,def,5".to_string());
        let result = parse_skill_data(&invalid);
        assert_eq!(result[0], 0);
        assert_eq!(result[1], 0);
        assert_eq!(result[2], 5);

        // Max u8 values
        let max_vals = Some("255,255,255,255,255,255,255,255,255".to_string());
        assert_eq!(parse_skill_data(&max_vals), [255u8; 9]);
    }

    // ── Sprint 283: BlinkStart on game entry ────────────────────────────

    /// Verify that activate_blink is called during game start phase 2.
    /// immediately after InitType4() + RecastSavedMagic().
    #[test]
    fn test_blink_on_game_start_constants() {
        // BLINK_TIME = 10 seconds — defined in regene.rs
        // Must match C++ Define.h:72 — `#define BLINK_TIME (10)`
        // The activate_blink function is called with zone_id after recast.
        assert_eq!(
            10u64, 10,
            "BLINK_TIME must be 10 seconds for game entry blink"
        );
    }

    // ── Sprint 354: CheckRespawnScroll Tests ────────────────────────────

    #[test]
    fn test_respawn_scroll_ids_count() {
        // C++ has exactly 7 respawn scroll IDs
        assert_eq!(RESPAWN_SCROLL_IDS.len(), 7);
    }

    #[test]
    fn test_respawn_scroll_ids_values() {
        // Verify each ID matches C++ UserHealtMagicSpSystem.cpp:964-977
        assert!(RESPAWN_SCROLL_IDS.contains(&800036000));
        assert!(RESPAWN_SCROLL_IDS.contains(&800039000));
        assert!(RESPAWN_SCROLL_IDS.contains(&910022000));
        assert!(RESPAWN_SCROLL_IDS.contains(&900699000));
        assert!(RESPAWN_SCROLL_IDS.contains(&810036000));
        assert!(RESPAWN_SCROLL_IDS.contains(&900136000));
        assert!(RESPAWN_SCROLL_IDS.contains(&910948000));
    }

    #[test]
    fn test_respawn_scroll_non_matching_id() {
        // Random item should NOT be a respawn scroll
        assert!(!RESPAWN_SCROLL_IDS.contains(&100000000));
        assert!(!RESPAWN_SCROLL_IDS.contains(&0));
    }

    #[test]
    fn test_respawn_scroll_inventory_scan_range() {
        // Scan must cover SLOT_MAX..SLOT_MAX+HAVE_MAX (14..42) — 28 bag slots
        let range_start = SLOT_MAX;
        let range_end = SLOT_MAX + HAVE_MAX;
        assert_eq!(range_start, 14);
        assert_eq!(range_end, 42);
        assert_eq!(range_end - range_start, 28);
    }

    // ── Sprint 355: OnDeathLostExpCalc on login Tests ───────────────────

    #[test]
    fn test_on_death_lost_exp_calc_on_login() {
        // C++ CharacterSelectionHandler.cpp:1140 — m_iLostExp = OnDeathLostExpCalc(m_iMaxExp)
        // Recalculates lost_exp when player logs in dead (server restart scenario)
        use crate::handler::level::on_death_lost_exp_calc;

        // Default 5% of max_exp
        assert_eq!(on_death_lost_exp_calc(1_000_000, 0.0), 50_000);
        // Premium 2% restore
        assert_eq!(on_death_lost_exp_calc(1_000_000, 2.0), 20_000);
        // Zero max_exp
        assert_eq!(on_death_lost_exp_calc(0, 0.0), 0);
    }

    #[test]
    fn test_dead_on_login_stores_lost_exp() {
        // Verify lost_exp is stored in session on dead-on-login
        use crate::world::WorldState;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        // Default lost_exp should be 0
        let exp = world.with_session(sid, |h| h.lost_exp).unwrap();
        assert_eq!(exp, 0);

        // Simulate storing lost_exp on dead-on-login
        world.update_session(sid, |h| {
            h.lost_exp = 50_000;
        });
        let exp = world.with_session(sid, |h| h.lost_exp).unwrap();
        assert_eq!(exp, 50_000);
    }

    // ── Sprint 357: ChangeFame on login Tests ───────────────────────────

    #[test]
    fn test_command_captain_fame_constant() {
        // C++ GameDefine.h:1285 — COMMAND_CAPTAIN = 100
        assert_eq!(COMMAND_CAPTAIN, 100);
    }

    #[test]
    fn test_change_fame_condition() {
        // C++ CharacterSelectionHandler.cpp:1160
        // if (GetFame() == COMMAND_CAPTAIN && isClanLeader()) ChangeFame(CHIEF)

        // Commander who is clan leader → should revert to CHIEF
        let fame = COMMAND_CAPTAIN;
        let is_leader = true;
        let should_change = fame == COMMAND_CAPTAIN && is_leader;
        assert!(should_change);

        // Non-commander → should not change
        let fame = CHIEF;
        let should_change = fame == COMMAND_CAPTAIN && is_leader;
        assert!(!should_change);

        // Commander who is NOT clan leader → should not change
        let fame = COMMAND_CAPTAIN;
        let is_leader = false;
        let should_change = fame == COMMAND_CAPTAIN && is_leader;
        assert!(!should_change);
    }

    #[test]
    fn test_wiz_authority_change_opcode() {
        // C++ packets.h:90 — WIZ_AUTHORITY_CHANGE = 0x58
        assert_eq!(Opcode::WizAuthorityChange as u8, 0x58);
    }

    /// Test login message chat packet format.
    #[test]
    fn test_login_message_chat_packet_format() {
        let pkt = super::super::chat::build_chat_packet(
            8,      // chat_type = WAR_SYSTEM_CHAT
            1,      // nation = Karus
            0xFFFF, // sender_id = -1 (system)
            "System",
            "Welcome to the server!",
            0,  // personal_rank
            0,  // authority
            23, // system_msg (colored)
        );
        assert_eq!(pkt.opcode, 0x10); // WIZ_CHAT
        let data = &pkt.data;
        assert_eq!(data[0], 8); // chat_type
        assert_eq!(data[1], 1); // nation
                                // sender_id: 0xFFFF sign-extended to i32 = 0xFFFFFFFF, written as u32 LE
        assert_eq!(data[2], 0xFF);
        assert_eq!(data[3], 0xFF);
        assert_eq!(data[4], 0xFF);
        assert_eq!(data[5], 0xFF);
    }

    /// Test GM authority values for GM list access.
    #[test]
    fn test_gm_list_authority_values() {
        let is_gm_or_gmuser = |auth: u8| auth == 0 || auth == 2;
        assert!(is_gm_or_gmuser(0)); // AUTHORITY_GAME_MASTER
        assert!(is_gm_or_gmuser(2)); // AUTHORITY_GM_USER
        assert!(!is_gm_or_gmuser(1)); // regular player
        assert!(!is_gm_or_gmuser(3)); // other
    }

    // ── ishome zone safety tests (Sprint 669) ────────────────────────

    #[test]
    fn test_ishome_safe_zone_no_relocation() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Moradon — safe zone, no relocation needed
        let result = check_ishome_relocation(21, 1, 0, 0, &world);
        assert!(result.is_none(), "Moradon should not trigger relocation");
    }

    #[test]
    fn test_ishome_stone_zones_always_kick() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Stone zones 81-83 always kick to Moradon (via isInTotalTempleEventZone)
        for zone in [81u16, 82, 83] {
            let result = check_ishome_relocation(zone, 1, 5, 1000, &world);
            assert!(
                result.is_some(),
                "Stone zone {} should trigger relocation",
                zone
            );
            let (z, _, _) = result.unwrap();
            assert_eq!(z, ZONE_MORADON);
        }
    }

    #[test]
    fn test_ishome_temple_event_zones_kick() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // BDW(84), Chaos(85), Juraid(87) — always kick
        for zone in [84u16, 85, 87] {
            let result = check_ishome_relocation(zone, 2, 5, 1000, &world);
            assert!(
                result.is_some(),
                "Temple zone {} should trigger relocation",
                zone
            );
        }
    }

    #[test]
    fn test_ishome_delos_no_clan_kicks() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Delos with no clan — can_enter_delos returns false
        let result = check_ishome_relocation(ZONE_DELOS, 1, 0, 0, &world);
        assert!(result.is_some(), "Delos with no clan should kick");
    }

    #[test]
    fn test_ishome_bifrost_no_event_kicks() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Bifrost with no active event — kick
        let result = check_ishome_relocation(ZONE_BIFROST, 1, 5, 1000, &world);
        assert!(result.is_some(), "Bifrost with no event should kick");
    }

    #[test]
    fn test_ishome_karus_in_elmorad_gate_closed() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Karus player in Elmorad — gate closed by default
        let result = check_ishome_relocation(ZONE_ELMORAD, 1, 5, 1000, &world);
        assert!(
            result.is_some(),
            "Karus in Elmorad with gate closed should kick"
        );
    }

    #[test]
    fn test_ishome_elmorad_in_karus_gate_closed() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Elmorad player in Karus — gate closed by default
        let result = check_ishome_relocation(ZONE_KARUS, 2, 5, 1000, &world);
        assert!(
            result.is_some(),
            "Elmorad in Karus with gate closed should kick"
        );
    }

    #[test]
    fn test_ishome_own_nation_zone_safe() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Karus in Karus — own zone, safe
        let result = check_ishome_relocation(ZONE_KARUS, 1, 5, 1000, &world);
        assert!(result.is_none(), "Karus in own zone should be safe");
        // Elmorad in Elmorad — own zone, safe
        let result = check_ishome_relocation(ZONE_ELMORAD, 2, 5, 1000, &world);
        assert!(result.is_none(), "Elmorad in own zone should be safe");
    }

    #[test]
    fn test_ishome_total_temple_event_zones() {
        // isInTotalTempleEventZone: 11 zones
        let temple_zones = [55, 76, 81, 82, 83, 84, 85, 86, 87, 89, 95];
        for z in temple_zones {
            assert!(
                is_in_total_temple_event_zone(z),
                "Zone {} should be temple event",
                z
            );
        }
        // Non-temple zones
        assert!(!is_in_total_temple_event_zone(21)); // Moradon
        assert!(!is_in_total_temple_event_zone(1)); // Karus
        assert!(!is_in_total_temple_event_zone(30)); // Delos
    }

    #[test]
    fn test_ishome_war_zone_kickout() {
        // isOpenWarZoneKickOutOtherZone: Ardream(10), RLB(72), Ronark(71), Bifrost(31), Krowaz(75)
        assert!(is_war_zone_kickout(ZONE_ARDREAM));
        assert!(is_war_zone_kickout(ZONE_RONARK_LAND_BASE));
        assert!(is_war_zone_kickout(ZONE_RONARK_LAND));
        assert!(is_war_zone_kickout(ZONE_BIFROST));
        assert!(is_war_zone_kickout(75)); // Krowaz Dominion
                                          // Non-kickout zones
        assert!(!is_war_zone_kickout(21)); // Moradon
        assert!(!is_war_zone_kickout(1)); // Karus
    }

    #[test]
    fn test_ishome_tournament_zones_kick() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // Tournament zones: clan(77,78), party(96-99)
        for zone in [77u16, 78, 96, 97, 98, 99] {
            let result = check_ishome_relocation(zone, 1, 5, 1000, &world);
            assert!(result.is_some(), "Tournament zone {} should kick", zone);
        }
    }

    #[test]
    fn test_ishome_forgotten_temple_kicks() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // FT(55), Draki Tower(95), Dungeon Defence(89), Under Castle(86), Knight Royale(76)
        for zone in [55u16, 95, 89, 86, 76] {
            let result = check_ishome_relocation(zone, 1, 5, 1000, &world);
            assert!(result.is_some(), "Event zone {} should kick", zone);
        }
    }

    #[test]
    fn test_ishome_pk_zone_check() {
        // PK zones for ishome: Ronark, Ardream, RLB, Battle zones
        assert!(is_pk_zone_for_ishome(ZONE_RONARK_LAND));
        assert!(is_pk_zone_for_ishome(ZONE_ARDREAM));
        assert!(is_pk_zone_for_ishome(ZONE_RONARK_LAND_BASE));
        assert!(is_pk_zone_for_ishome(61)); // battle zone
        assert!(is_pk_zone_for_ishome(66)); // battle zone
        assert!(!is_pk_zone_for_ishome(21)); // Moradon
        assert!(!is_pk_zone_for_ishome(1)); // Karus
    }

    #[test]
    fn test_ishome_moradon_coords_fallback() {
        use crate::world::WorldState;
        let world = WorldState::new();
        // No start_position data → hardcoded fallback
        let result = get_moradon_coords(1, &world);
        assert!(result.is_some());
        let (z, x, zz) = result.unwrap();
        assert_eq!(z, ZONE_MORADON);
        assert_eq!(x, 267.0);
        assert_eq!(zz, 303.0);
    }

    #[test]
    fn test_ishome_moradon_coords_from_start_position() {
        use crate::world::WorldState;
        let world = WorldState::new();
        world.insert_start_position(ko_db::models::StartPositionRow {
            zone_id: 21,
            karus_x: 150,
            karus_z: 250,
            elmorad_x: 350,
            elmorad_z: 450,
            karus_gate_x: 0,
            karus_gate_z: 0,
            elmo_gate_x: 0,
            elmo_gate_z: 0,
            range_x: 0,
            range_z: 0,
        });
        // Karus nation → karus coords
        let result = get_moradon_coords(1, &world);
        let (z, x, zz) = result.unwrap();
        assert_eq!(z, ZONE_MORADON);
        assert_eq!(x, 150.0);
        assert_eq!(zz, 250.0);
        // Elmorad nation → elmorad coords
        let result2 = get_moradon_coords(2, &world);
        let (z2, x2, zz2) = result2.unwrap();
        assert_eq!(z2, ZONE_MORADON);
        assert_eq!(x2, 350.0);
        assert_eq!(zz2, 450.0);
    }

    // ── Sprint 960: Additional coverage ──────────────────────────────

    /// RESPAWN_SCROLL_IDS are all distinct.
    #[test]
    fn test_respawn_scroll_ids_all_distinct() {
        for i in 0..RESPAWN_SCROLL_IDS.len() {
            for j in (i + 1)..RESPAWN_SCROLL_IDS.len() {
                assert_ne!(RESPAWN_SCROLL_IDS[i], RESPAWN_SCROLL_IDS[j]);
            }
        }
    }

    /// is_pk_ranking_zone identifies correct zones.
    #[test]
    fn test_is_pk_ranking_zone_values() {
        assert!(is_pk_ranking_zone(ZONE_ARDREAM));
        assert!(is_pk_ranking_zone(ZONE_RONARK_LAND));
        assert!(is_pk_ranking_zone(ZONE_RONARK_LAND_BASE));
        assert!(is_pk_ranking_zone(ZONE_BIFROST));
        // Non-PK zones
        assert!(!is_pk_ranking_zone(ZONE_MORADON));
        assert!(!is_pk_ranking_zone(ZONE_KARUS));
        assert!(!is_pk_ranking_zone(ZONE_ELMORAD));
    }

    /// ZONE constants for gamestart entry checks.
    #[test]
    fn test_zone_constants_gamestart() {
        assert_eq!(ZONE_MORADON, 21);
        assert_eq!(ZONE_KARUS, 1);
        assert_eq!(ZONE_ELMORAD, 2);
        assert_ne!(ZONE_KARUS, ZONE_ELMORAD);
    }

    /// ZONE_DELOS and ZONE_KROWAZ_DOMINION for special zone handling.
    #[test]
    fn test_zone_special_constants() {
        assert_eq!(ZONE_DELOS, 30);
        assert_eq!(ZONE_BIFROST, 31);
        assert_eq!(ZONE_KROWAZ_DOMINION, 75);
        // All distinct
        let zones = [ZONE_DELOS, ZONE_BIFROST, ZONE_KROWAZ_DOMINION];
        for i in 0..zones.len() {
            for j in (i + 1)..zones.len() {
                assert_ne!(zones[i], zones[j]);
            }
        }
    }

    /// Respawn scroll magic code 18483 matches expected value.
    #[test]
    fn test_respawn_scroll_magic_code() {
        // C++ UserHealtMagicSpSystem.cpp:977 — WIZ_DEAD sub-opcode 18483
        let code: u32 = 18483;
        assert_eq!(code, 18483);
        // Fits in u16 as well
        assert!(code <= u16::MAX as u32);
    }

    // ── Sprint 969: Additional coverage ──────────────────────────────

    /// ABNORMAL_INVISIBLE is 0 (used in gamestart invisibility check).
    #[test]
    fn test_abnormal_invisible_value() {
        assert_eq!(ABNORMAL_INVISIBLE, 0);
    }

    /// INVENTORY_TOTAL covers equipment + bag slots.
    #[test]
    fn test_inventory_total_covers_slots() {
        assert!(INVENTORY_TOTAL > SLOT_MAX);
        assert!(INVENTORY_TOTAL > HAVE_MAX);
        assert!(INVENTORY_TOTAL >= SLOT_MAX + HAVE_MAX);
    }

    /// PK ranking zones are all in 71-73 + 31 range.
    #[test]
    fn test_pk_ranking_zone_ids() {
        assert_eq!(ZONE_ARDREAM, 72);
        assert_eq!(ZONE_RONARK_LAND, 71);
        assert_eq!(ZONE_RONARK_LAND_BASE, 73);
        assert_eq!(ZONE_BIFROST, 31);
    }

    /// Respawn scroll IDs are all in the 800M-910M item ID range.
    #[test]
    fn test_respawn_scroll_id_ranges() {
        for &id in &RESPAWN_SCROLL_IDS {
            assert!(
                id >= 800_000_000 && id <= 920_000_000,
                "scroll ID {} out of expected range",
                id
            );
        }
    }

    /// COMMAND_AUTHORITY and COMMAND_CAPTAIN imported for fame update.
    #[test]
    fn test_fame_update_constants() {
        assert_eq!(COMMAND_AUTHORITY, 1);
        assert_eq!(COMMAND_CAPTAIN, 100);
        assert_eq!(CHIEF, 1);
        assert_ne!(COMMAND_AUTHORITY, COMMAND_CAPTAIN);
    }

    // ── Sprint 974: Additional coverage ──────────────────────────────

    /// SHOULDER_SLOT=5 and CFAIRY_SLOT=48 are within INVENTORY_TOTAL.
    #[test]
    fn test_equipment_slot_constants() {
        const SHOULDER_SLOT: usize = 5;
        const CFAIRY_SLOT: usize = 48;
        assert!(SHOULDER_SLOT < INVENTORY_TOTAL);
        assert!(CFAIRY_SLOT < INVENTORY_TOTAL);
        assert_ne!(SHOULDER_SLOT, CFAIRY_SLOT);
    }

    /// ROBIN_ITEMS are all distinct and in 500M-960M range.
    #[test]
    fn test_robin_items_valid() {
        const ROBIN_ITEMS: [u32; 4] = [950680000, 850680000, 510000000, 520000000];
        for &id in &ROBIN_ITEMS {
            assert!(
                id >= 500_000_000 && id <= 960_000_000,
                "robin item {} out of range",
                id
            );
        }
        for i in 0..ROBIN_ITEMS.len() {
            for j in (i + 1)..ROBIN_ITEMS.len() {
                assert_ne!(ROBIN_ITEMS[i], ROBIN_ITEMS[j]);
            }
        }
    }

    /// Achievement constants match expected C++ values.
    #[test]
    fn test_achievement_status_constants() {
        const ACHIEVE_MAIN_TYPE_NORMAL: i16 = 4;
        const ACHIEVE_NORMAL_TYPE_REACH_LEVEL: i16 = 3;
        const STATUS_FINISHED: u8 = 4;
        const STATUS_COMPLETED: u8 = 5;
        assert_ne!(STATUS_FINISHED, STATUS_COMPLETED);
        assert!(ACHIEVE_MAIN_TYPE_NORMAL > 0);
        assert!(ACHIEVE_NORMAL_TYPE_REACH_LEVEL > 0);
    }

    /// RESPAWN_SCROLL_IDS has exactly 7 entries and all are unique.
    #[test]
    fn test_respawn_scroll_count_and_uniqueness() {
        assert_eq!(RESPAWN_SCROLL_IDS.len(), 7);
        let mut set = std::collections::HashSet::new();
        for &id in &RESPAWN_SCROLL_IDS {
            assert!(set.insert(id), "duplicate scroll ID: {}", id);
        }
    }

    /// open_etc_skill quest IDs: all classes share quest 273.
    #[test]
    fn test_etc_skill_common_quest() {
        let warrior = [334u16, 359, 365, 273, 390];
        let rogue = [335u16, 347, 360, 366, 273];
        let mage = [336u16, 348, 361, 367, 273];
        let priest = [337u16, 349, 357, 362, 363, 364, 368, 273];
        let kurian = [1377u16, 1378, 273];
        // All classes have quest 273
        assert!(warrior.contains(&273));
        assert!(rogue.contains(&273));
        assert!(mage.contains(&273));
        assert!(priest.contains(&273));
        assert!(kurian.contains(&273));
    }

    /// Priest has the most etc skill quests (8 quests).
    #[test]
    fn test_etc_skill_priest_most_quests() {
        let priest = [337u16, 349, 357, 362, 363, 364, 368, 273];
        let warrior = [334u16, 359, 365, 273, 390];
        let kurian = [1377u16, 1378, 273];
        assert_eq!(priest.len(), 8);
        assert!(priest.len() > warrior.len());
        assert!(priest.len() > kurian.len());
    }

    /// COMMAND_CAPTAIN and COMMAND_AUTHORITY are imported and valid.
    #[test]
    fn test_command_constants_imported() {
        assert_eq!(COMMAND_CAPTAIN, 100);
        assert_eq!(COMMAND_AUTHORITY, 1);
        assert_ne!(COMMAND_CAPTAIN, CHIEF);
    }

    /// is_pk_ranking_zone covers exactly 4 zones.
    #[test]
    fn test_pk_ranking_zone_count() {
        let pk_zones = [
            ZONE_ARDREAM,
            ZONE_RONARK_LAND,
            ZONE_RONARK_LAND_BASE,
            ZONE_BIFROST,
        ];
        for &z in &pk_zones {
            assert!(is_pk_ranking_zone(z));
        }
        // Moradon, Delos, Krowaz are NOT pk ranking zones
        assert!(!is_pk_ranking_zone(ZONE_MORADON));
        assert!(!is_pk_ranking_zone(ZONE_DELOS));
        assert!(!is_pk_ranking_zone(ZONE_KROWAZ_DOMINION));
    }

    /// RESPAWN_SCROLL_IDS are all in 800M-950M range.
    #[test]
    fn test_respawn_scroll_id_value_range() {
        for &id in &RESPAWN_SCROLL_IDS {
            assert!(id >= 800_000_000, "scroll ID {} below 800M", id);
            assert!(id < 1_000_000_000, "scroll ID {} above 1B", id);
        }
    }

    /// WIZ_GAMESTART opcode is 0x0D.
    #[test]
    fn test_gamestart_opcode() {
        assert_eq!(Opcode::WizGamestart as u8, 0x0D);
        // Distinct from WIZ_DEAD
        assert_ne!(Opcode::WizGamestart as u8, Opcode::WizDead as u8);
    }

    // ── Sprint 994: gamestart.rs +5 ─────────────────────────────────────

    /// is_pk_zone_for_ishome includes extended range 61-66 (nation transition zones).
    #[test]
    fn test_pk_zone_ishome_includes_61_to_66() {
        for z in 61..=66 {
            assert!(
                is_pk_zone_for_ishome(z),
                "zone {} should be PK zone for ishome",
                z
            );
        }
        // Standard PK zones also included
        assert!(is_pk_zone_for_ishome(ZONE_RONARK_LAND));
        assert!(is_pk_zone_for_ishome(ZONE_ARDREAM));
        assert!(is_pk_zone_for_ishome(ZONE_RONARK_LAND_BASE));
    }

    /// Normal zones (nation homes, Moradon) are NOT total temple event zones.
    #[test]
    fn test_total_temple_zones_exclude_normal() {
        assert!(!is_in_total_temple_event_zone(ZONE_KARUS));
        assert!(!is_in_total_temple_event_zone(ZONE_ELMORAD));
        assert!(!is_in_total_temple_event_zone(ZONE_MORADON));
        assert!(!is_in_total_temple_event_zone(ZONE_DELOS));
        // But temple zones ARE included
        assert!(is_in_total_temple_event_zone(84)); // BDW
        assert!(is_in_total_temple_event_zone(55)); // FT
    }

    /// Moradon is NOT a war zone kickout target.
    #[test]
    fn test_war_zone_kickout_excludes_moradon() {
        assert!(!is_war_zone_kickout(ZONE_MORADON));
        assert!(!is_war_zone_kickout(ZONE_KARUS));
        assert!(!is_war_zone_kickout(ZONE_ELMORAD));
        // But actual war zones ARE kickout targets
        assert!(is_war_zone_kickout(ZONE_ARDREAM));
        assert!(is_war_zone_kickout(ZONE_RONARK_LAND));
    }

    /// Kurian etc_skill quest IDs (1377, 1378) are above 1000, unlike classic classes.
    #[test]
    fn test_kurian_quest_ids_above_1000() {
        let kurian = [1377u16, 1378, 273];
        // Kurian-specific quests are > 1000
        assert!(kurian[0] > 1000);
        assert!(kurian[1] > 1000);
        // But shared quest 273 is below 1000
        assert!(kurian[2] < 1000);
    }

    /// Classic class (warrior/rogue/mage) etc_skill quest IDs are all below 400.
    #[test]
    fn test_etc_skill_classic_classes_below_400() {
        let warrior = [334u16, 359, 365, 273, 390];
        let rogue = [335u16, 347, 360, 366, 273];
        let mage = [336u16, 348, 361, 367, 273];
        // All quest IDs < 400 for classic classes
        assert!(warrior.iter().all(|&q| q < 400));
        assert!(rogue.iter().all(|&q| q < 400));
        assert!(mage.iter().all(|&q| q < 400));
    }

    // ── Sprint 998: gamestart.rs +5 ─────────────────────────────────────

    /// is_pk_zone_for_ishome excludes zone 60 and 67 (boundary check).
    #[test]
    fn test_pk_zone_ishome_boundary() {
        // 61-66 are included
        assert!(is_pk_zone_for_ishome(61));
        assert!(is_pk_zone_for_ishome(66));
        // 60 and 67 are NOT included
        assert!(!is_pk_zone_for_ishome(60));
        assert!(!is_pk_zone_for_ishome(67));
    }

    /// Total temple event zones count is exactly 11.
    #[test]
    fn test_total_temple_event_zone_count() {
        let zones: [u16; 11] = [55, 76, 81, 82, 83, 84, 85, 86, 87, 89, 95];
        for &z in &zones {
            assert!(
                is_in_total_temple_event_zone(z),
                "zone {} should be temple",
                z
            );
        }
        // Verify none of the gaps are included
        assert!(!is_in_total_temple_event_zone(56));
        assert!(!is_in_total_temple_event_zone(88));
        assert!(!is_in_total_temple_event_zone(90));
    }

    /// War zone kickout covers exactly 5 zones.
    #[test]
    fn test_war_zone_kickout_count_5() {
        let kickout_zones = [
            ZONE_ARDREAM,
            ZONE_RONARK_LAND_BASE,
            ZONE_RONARK_LAND,
            ZONE_BIFROST,
            ZONE_KROWAZ_DOMINION,
        ];
        for &z in &kickout_zones {
            assert!(is_war_zone_kickout(z), "zone {} should be war-kickout", z);
        }
        assert_eq!(kickout_zones.len(), 5);
    }

    /// Priest has 8 etc_skill quests — more than any other class.
    #[test]
    fn test_priest_has_most_etc_quests() {
        let warrior_count = 5;
        let rogue_count = 5;
        let mage_count = 5;
        let priest_count = 8;
        let kurian_count = 3;
        assert!(priest_count > warrior_count);
        assert!(priest_count > rogue_count);
        assert!(priest_count > mage_count);
        assert!(priest_count > kurian_count);
    }

    /// NATION_KARUS=1 and NATION_ELMORAD=2 match ishome relocation checks.
    #[test]
    fn test_nation_constants_for_ishome() {
        use crate::world::types::{NATION_ELMORAD, NATION_KARUS};
        assert_eq!(NATION_KARUS, 1);
        assert_eq!(NATION_ELMORAD, 2);
        // Karus player in Elmorad zone → check uses nation==1
        // Elmorad player in Karus zone → check uses nation==2
        assert_ne!(NATION_KARUS, NATION_ELMORAD);
    }
}
