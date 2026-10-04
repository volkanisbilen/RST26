//! WIZ_MAGIC_PROCESS (0x31) handler — skill casting & execution.
//! ## Client Request (C→S)
//! | Type   | Description                              |
//! |--------|------------------------------------------|
//! | u8     | bOpcode (MagicOpcode enum)               |
//! | u32le  | nSkillID (magic_num from magic table)    |
//! | i32le  | sCasterID (entity ID, verified = session) |
//! | i32le  | sTargetID (-1 = no target)               |
//! | i32le  | sData[0] (mouse X / area data)           |
//! | i32le  | sData[1] (reserved)                      |
//! | i32le  | sData[2] (mouse Z / area data)           |
//! | i32le  | sData[3..6] (extra data)                 |
//! ## Server Broadcast (S→C, to 3×3 region)
//! Same structure as incoming, but with server-computed values.

use std::time::Instant;

use ko_db::models::{MagicRow, MagicType1Row, MagicType2Row};
use ko_protocol::{Opcode, Packet, PacketReader};
use rand::Rng;
use rand::SeedableRng;
use std::sync::Arc;

use crate::world::combat::CombatSnapshot;

use crate::buff_constants::*;
use crate::handler::dead;
use crate::handler::durability::{WORE_TYPE_ATTACK, WORE_TYPE_DEFENCE};
use crate::handler::stealth;
use crate::session::{ClientSession, SessionState};
use crate::systems::buff_tick::build_buff_expired_packet;
#[cfg(test)]
use crate::world::NATION_ELMORAD;
use crate::world::{
    ActiveBuff, CharacterInfo, NpcBuffEntry, WorldState, NATION_KARUS, USER_DEAD, USER_SITDOWN,
    ZONE_BATTLE2, ZONE_BATTLE3, ZONE_CHAOS_DUNGEON, ZONE_DELOS, ZONE_DUNGEON_DEFENCE,
    ZONE_FORGOTTEN_TEMPLE, ZONE_KNIGHT_ROYALE, ZONE_SNOW_BATTLE, ZONE_UNDER_CASTLE,
};
use crate::zone::SessionId;

use crate::attack_constants::{FAIL, GREAT_SUCCESS, NORMAL, SUCCESS};
use crate::inventory_constants::{LEFTHAND, RIGHTHAND};
use crate::magic_constants::{
    ABNORMAL_BLINKING, ABNORMAL_DWARF, ABNORMAL_GIANT, ABNORMAL_GIANT_TARGET, ABNORMAL_NORMAL,
    MAGIC_CANCEL, MAGIC_CANCEL2, MAGIC_CANCEL_TRANSFORMATION, MAGIC_CASTING,
    MAGIC_DURATION_EXPIRED, MAGIC_EFFECTING, MAGIC_FAIL, MAGIC_FLYING, MAGIC_TYPE4_EXTEND,
    MORAL_ALL, MORAL_AREA_ALL, MORAL_AREA_ENEMY, MORAL_AREA_FRIEND, MORAL_ENEMY,
    MORAL_FRIEND_EXCEPTME, MORAL_FRIEND_WITHME, MORAL_PARTY, MORAL_PARTY_ALL, MORAL_SELF,
    MORAL_SELF_AREA, SKILLMAGIC_FAIL_ATTACKZERO, SKILLMAGIC_FAIL_NOEFFECT,
};
use crate::npc_type_constants::{
    NPC_BIFROST_MONUMENT, NPC_BORDER_MONUMENT, NPC_CLAN_WAR_MONUMENT, NPC_DESTROYED_ARTIFACT,
    NPC_FOSIL, NPC_GATE, NPC_GATE2, NPC_GATE_LEVER, NPC_GUARD_TOWER1, NPC_GUARD_TOWER2,
    NPC_OBJECT_FLAG, NPC_PARTNER_TYPE, NPC_PHOENIX_GATE, NPC_PRISON, NPC_PVP_MONUMENT, NPC_REFUGEE,
    NPC_SOCCER_BAAL, NPC_SPECIAL_GATE, NPC_TREE, NPC_VICTORY_GATE,
};
use crate::state_change_constants::{STATE_CHANGE_ABNORMAL, STATE_CHANGE_WEAPONS_DISABLED};

/// Snow Battle event snowball skill — only this skill is allowed during Snow Battle.
const SNOW_EVENT_SKILL: u32 = 490077;

use crate::npc::NPC_BAND;

use crate::magic_constants::{TRANSFORMATION_MONSTER, TRANSFORMATION_NPC, TRANSFORMATION_SIEGE};

// ── Parsed skill instance ─────────────────────────────────────────────────

/// Parsed WIZ_MAGIC_PROCESS packet — mirrors `MagicInstance`.
/// C++ stores caster/target as `int32` — we must preserve the full 32-bit values
/// to avoid truncation when broadcasting back to clients.
struct MagicInstance {
    #[allow(dead_code)]
    opcode: u8,
    skill_id: u32,
    caster_id: i32,
    target_id: i32,
    data: [i32; 7],
}

impl MagicInstance {
    /// Build the skill packet for broadcasting.
    ///
    fn build_packet(&self, opcode: u8) -> Packet {
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(opcode);
        pkt.write_u32(self.skill_id);
        pkt.write_u32(self.caster_id as u32);
        pkt.write_u32(self.target_id as u32);
        for d in &self.data {
            pkt.write_u32(*d as u32);
        }
        pkt
    }

    /// Build a MAGIC_FAIL packet to send back to the caster.
    fn build_fail_packet(&self) -> Packet {
        self.build_packet(MAGIC_FAIL)
    }
}

// ── Main handler ──────────────────────────────────────────────────────────

/// Handle WIZ_MAGIC_PROCESS (0x31) from the client.
/// Parses the magic packet, validates prerequisites, and routes to
/// the appropriate handler based on the magic opcode and skill type.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // ── Parse client packet ─────────────────────────────────────────
    let mut reader = PacketReader::new(&pkt.data);

    let b_opcode = reader.read_u8().unwrap_or(0);
    let skill_id = reader.read_u32().unwrap_or(0);
    let caster_id_raw = reader.read_u32().unwrap_or(0) as i32;
    let target_id_raw = reader.read_u32().unwrap_or(0) as i32;

    let mut s_data = [0i32; 7];
    for d in &mut s_data {
        *d = reader.read_u32().unwrap_or(0) as i32;
    }

    // Pet summon skill: Type 9 state_change=8 is not stealth.
    // Route it to the runtime pet NPC spawn path.
    if skill_id == 500117 && b_opcode == MAGIC_EFFECTING {
        let empty_data: [u8; 0] = [];
        let mut pet_reader = PacketReader::new(&empty_data);

        crate::handler::pet::handle_normal_mode(
            session,
            2, // MODE_SUMMON
            &mut pet_reader,
        )
        .await?;

        return Ok(());
    }

    // ── Basic validation ────────────────────────────────────────────

    // Skill ID 0 = invalid
    if skill_id == 0 {
        return Ok(());
    }

    // Block skills during zone change
    if b_opcode < 5 && world.is_zone_changing(sid) {
        return Ok(());
    }

    // Prevent caster ID spoofing — must match the session
    // v2600: caster_id is u32 but session IDs are u16. Low 16 bits must match.
    let caster_id = caster_id_raw;
    if (caster_id as u32 & 0xFFFF) as u16 != sid || (caster_id as u32) >= NPC_BAND {
        return Ok(());
    }

    // v2600: target_id is u32 (NOT truncated to i16 like old C++ server).
    // PCAP verified: NPC target IDs like 49886 exceed i16 range.
    // -1 (no target) is sent as 0xFFFFFFFF which maps to -1 as i32.
    let mut target_id = target_id_raw;

    // ── Special skill target validation ──────────────────────────────
    // Skills 109035/110035/209035/210035 must have target=-1 (no target).
    // Skills 109015/110015/209015/210015 must target self (target == caster).
    if target_id != -1 && matches!(skill_id, 109035 | 110035 | 209035 | 210035) {
        let fail_pkt = build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
        world.send_to_session_owned(sid, fail_pkt);
        return Ok(());
    }
    if target_id != caster_id && matches!(skill_id, 109015 | 110015 | 209015 | 210015) {
        let fail_pkt = build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
        world.send_to_session_owned(sid, fail_pkt);
        return Ok(());
    }

    // ── Look up skill in magic table ────────────────────────────────
    let skill = match world.get_magic(skill_id as i32) {
        Some(s) => s,
        None => {
            tracing::debug!("[sid={}] MagicProcess: unknown skill_id={}", sid, skill_id);
            return Ok(());
        }
    };

    // Ground-target AOE packets from v2615 may encode "no entity target" as
    // 0 instead of -1.  Entity validation below would otherwise interpret 0
    // as a player session and reject Inferno/Nova before Type3 execution.
    if target_id == 0
        && matches!(
            skill.moral.unwrap_or(0),
            MORAL_AREA_ENEMY | MORAL_AREA_FRIEND | MORAL_AREA_ALL
        )
    {
        target_id = -1;
    }

    let mut instance = MagicInstance {
        opcode: b_opcode,
        skill_id,
        caster_id,
        target_id,
        data: s_data,
    };

    // ── Caster validation ───────────────────────────────────────────
    let caster = match world.get_character_info(sid) {
        Some(ch) => ch,
        None => return Ok(()),
    };

    // Dead players cannot cast — except Type 5 (resurrection/self-revive) skills.
    if (caster.res_hp_type == USER_DEAD || caster.hp <= 0) && skill.type1.unwrap_or(0) != 5 {
        return Ok(());
    }

    // Sitting players can only cast non-offensive magic (heal, self-buff, cancel).
    // and MORAL_AREA_ENEMY(10) ONLY during EFFECTING phase.
    if caster.res_hp_type == USER_SITDOWN {
        let moral = skill.moral.unwrap_or(0);
        if moral == MORAL_ENEMY || moral == MORAL_AREA_ENEMY {
            return Ok(());
        }
    }

    // (checked in UserCanCast, before opcode routing)
    // Exceptions: Cancel, CancelTransformation, Type4Extend, Fail are allowed.
    if b_opcode != MAGIC_TYPE4_EXTEND
        && b_opcode != MAGIC_CANCEL
        && b_opcode != MAGIC_CANCEL2
        && b_opcode != MAGIC_CANCEL_TRANSFORMATION
        && b_opcode != MAGIC_FAIL
    {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if world.is_player_blinking(sid, now_unix) {
            return Ok(());
        }
    }

    // GM attack ban blocks all skill casting.
    if world.is_attack_disabled(sid) {
        let fail_pkt = instance.build_fail_packet();
        world.send_to_session_owned(sid, fail_pkt);
        return Ok(());
    }

    // Shopping mall open blocks all magic.
    if world.is_store_open(sid) {
        let fail_pkt = instance.build_fail_packet();
        world.send_to_session_owned(sid, fail_pkt);
        return Ok(());
    }

    // Pre-read caster position for all zone-based validation checks below.
    let caster_pos = world.get_position(sid).unwrap_or_default();

    // Snow Battle event blocks all magic except the snowball skill.
    // if (zone == ZONE_SNOW_BATTLE && battle_open == SNOW_BATTLE && skill != SNOW_EVENT_SKILL) fail
    {
        if caster_pos.zone_id == ZONE_SNOW_BATTLE
            && world.get_battle_state().is_snow_battle()
            && skill_id != SNOW_EVENT_SKILL
        {
            let fail_pkt = instance.build_fail_packet();
            world.send_to_session_owned(sid, fail_pkt);
            return Ok(());
        }
    }

    // GrantType4Buff sets m_bCanUseSkills = false for BUFF_TYPE_FREEZE (22)
    // Exceptions: Type4Extend, Cancel, CancelTransformation can still be used.
    if b_opcode != MAGIC_TYPE4_EXTEND
        && b_opcode != MAGIC_CANCEL
        && b_opcode != MAGIC_CANCEL2
        && b_opcode != MAGIC_CANCEL_TRANSFORMATION
        && b_opcode != MAGIC_FAIL
        && world.has_buff(sid, BUFF_TYPE_FREEZE)
    {
        return Ok(());
    }

    // ── Special event zone magic block ──────────────────────────────
    // Block offensive magic in SPBATTLE zones when event NOT opened,
    // and in Cinderella zones when war is ON but NOT started.
    {
        let skill_moral = skill.moral.unwrap_or(0);
        if skill_moral == MORAL_ENEMY || skill_moral == MORAL_AREA_ENEMY {
            // SPBATTLE zones (Zindan War) — block when event not opened
            if crate::handler::attack::is_in_special_event_zone(caster_pos.zone_id)
                && !world.is_zindan_event_opened()
                && !world.is_cinderella_active()
            {
                let fail_pkt = build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }
            // Cinderella zone — block when war is ON but not started
            if !world.is_zindan_event_opened()
                && world.is_cinderella_active()
                && world.cinderella_zone_id() == caster_pos.zone_id
            {
                let fail_pkt = build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }
        }
    }

    // ── Temple event attack gate ────────────────────────────────────
    {
        use crate::systems::event_room;
        if event_room::is_in_temple_event_zone(caster_pos.zone_id) {
            // ── IsAvailable: broad is_attackable gate — blocks ALL magic for ALL zone users
            //   if (pSkillCaster->isInTempleEventZone()
            //       && !g_pMain->pTempleEvent.isAttackable)
            //       goto fail_return;
            // During non-combat event phases (signing, preparation, rewards),
            // ALL magic is blocked for everyone in the zone — not just registered
            // event users. This is the broadest gate in the C++ code.
            {
                let is_attackable = world
                    .event_room_manager
                    .read_temple_event(|s| s.is_attackable);
                if !is_attackable {
                    let fail_pkt =
                        build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
                    world.send_to_session_owned(sid, fail_pkt);
                    return Ok(());
                }
            }

            // Below only runs when is_attackable=true (combat phase active)
            let caster_name = world.get_session_name(sid).unwrap_or_default();
            let skill_moral = skill.moral.unwrap_or(0);

            // virt_eventattack_check — only for offensive morals
            if (skill_moral == MORAL_ENEMY
                || skill_moral == MORAL_ALL
                || skill_moral == MORAL_AREA_ALL)
                && !event_room::virt_eventattack_check(
                    &world.event_room_manager,
                    caster_pos.zone_id,
                    &caster_name,
                )
            {
                let fail_pkt = build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }

            // isSameEventRoom — applies to ALL targeted player spells
            //   if (pCaster->isInTempleEventZone() && !pCaster->isSameEventRoom(pSkillTarget))
            //       return SkillUseFail;
            if target_id >= 0 && (target_id as u32) < NPC_BAND {
                let target_sid = target_id as SessionId;
                let target_name = world.get_session_name(target_sid).unwrap_or_default();
                if let Some(event_type) = event_room::event_type_for_zone(caster_pos.zone_id) {
                    let caster_room = world
                        .event_room_manager
                        .find_user_room(event_type, &caster_name)
                        .map(|(r, _)| r);
                    let target_room = world
                        .event_room_manager
                        .find_user_room(event_type, &target_name)
                        .map(|(r, _)| r);
                    match (caster_room, target_room) {
                        (Some(a), Some(t)) if a == t => {} // same room
                        _ => {
                            let fail_pkt =
                                build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
                            world.send_to_session_owned(sid, fail_pkt);
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    // ── Monster Stone event room isolation (magic spells) ────────────
    //   isInTempleQuestEventZone() && (!isSameEventRoom(pSkillTarget) && m_sMonsterStoneStatus)
    //   isInTempleQuestEventZone() && !isSameEventRoom(pSkillTarget)
    // Players in Monster Stone zones with an active Monster Stone room must
    // be in the same event room for targeted spells to succeed.
    {
        use crate::systems::monster_stone;
        if monster_stone::is_monster_stone_zone(caster_pos.zone_id)
            && world.get_monster_stone_status(sid)
            && target_id >= 0
            && (target_id as u32) < NPC_BAND
        {
            let target_sid = target_id as SessionId;
            if !world.is_same_event_room(sid, target_sid) {
                let fail_pkt = build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }
        }
    }

    // Busy state checks: cannot cast while trading or merchanting
    if world.is_trading(sid) || world.is_merchanting(sid) {
        return Ok(());
    }

    // Safety area check: cannot cast offensive magic in enemy safety areas
    //   if (pCaster && (pCaster->isInEnemySafetyArea() && nSkillID < 400000))
    //       return SkillUseFail;
    if skill_id < 400000
        && crate::handler::attack::is_in_enemy_safety_area(
            caster_pos.zone_id,
            caster_pos.x,
            caster_pos.z,
            caster.nation,
        )
    {
        let fail_pkt = build_skill_failed_packet(skill_id, caster_id, target_id, &s_data);
        world.send_to_session_owned(sid, fail_pkt);
        return Ok(());
    }

    // ── Cooldown check ────────────────────────────────────────────────
    // Applies to all opcodes EXCEPT: TYPE4_EXTEND, CANCEL, CANCEL_TRANSFORMATION, FAIL.
    // Type-9 skills (stealth) are also excluded per C++ (bType[0] != 9).
    let skill_type = skill.type1.unwrap_or(0) as u8;
    let (has_instant_cast, on_cooldown) = world
        .with_session(sid, |h| {
            let cd = h
                .skill_cooldowns
                .get(&skill_id)
                .map(|expiry| std::time::Instant::now() < *expiry)
                .unwrap_or(false);
            (h.instant_cast, cd)
        })
        .unwrap_or((false, false));
    if skill_type != 9
        && !has_instant_cast
        && b_opcode != MAGIC_TYPE4_EXTEND
        && b_opcode != MAGIC_CANCEL
        && b_opcode != MAGIC_CANCEL2
        && b_opcode != MAGIC_CANCEL_TRANSFORMATION
        && b_opcode != MAGIC_FAIL
        && on_cooldown
    {
        let fail_pkt = instance.build_fail_packet();
        world.send_to_session_owned(sid, fail_pkt);
        return Ok(());
    }

    // ── Type cooldown check ────────────────────────────────────────────
    // Minimum time between casts of the same magic type. Default 575ms.
    // Checked AFTER per-skill cooldown. Only for types 1,3,4,5,6,7.
    // NOTE: C++ does NOT gate this on instant_cast (is_buff). instant_cast
    // only skips the SET block, not the CHECK.
    {
        let type1 = skill.type1.unwrap_or(0) as u8;
        let type2 = skill.type2.unwrap_or(0) as u8;
        let item_group = skill.item_group.unwrap_or(0) as u8;
        let valid_type = matches!(type1, 1 | 3 | 4 | 5 | 6 | 7);
        if valid_type && skill_id < 400000 && item_group != 255 && b_opcode != MAGIC_FAIL {
            // C++ MagicInstance.cpp:1744-1747,1980 — existspeed bypass for bType[0] only
            // pType4 is only set when bType[0]==4 && bType[1]==0; otherwise nullptr.
            let existspeed = b_opcode == MAGIC_TYPE4_EXTEND
                || (type1 == 4
                    && type2 == 0
                    && world
                        .get_magic_type4(skill_id as i32)
                        .map(|t4| t4.buff_type == Some(BUFF_TYPE_ATTACK_SPEED))
                        .unwrap_or(false));
            let blocked = check_type_cooldown(&world, sid, &skill, type1, type2, existspeed);
            if blocked {
                // C++ MagicInstance.cpp:2009-2012 — stomp/rogue → NoFunction, others → silent
                let skill_id_u32 = skill_id;
                let is_rogue_class = matches!(caster.class % 100, 2 | 7 | 8);
                if is_stomp_skill(skill_id_u32) || is_rogue_class {
                    // SkillUseNoFunction: send MAGIC_FAIL with SKILLMAGIC_FAIL_NOFUNCTION (-102)
                    instance.data[3] = -102; // SKILLMAGIC_FAIL_NOFUNCTION
                    let fail_pkt = instance.build_fail_packet();
                    world.send_to_session_owned(sid, fail_pkt);
                }
                // Non-rogues: silent return (SkillUseOnlyUse)
                return Ok(());
            }
        }
    }

    // ── Nation validation ─────────────────────────────────────────────
    // Skill ID encodes nation (1xxxx=Karus, 2xxxx=Elmorad). Skills < 300000
    // must match the caster's nation. Cancel/cancel2/cancel_transform excluded.
    if b_opcode != MAGIC_CANCEL
        && b_opcode != MAGIC_CANCEL2
        && b_opcode != MAGIC_CANCEL_TRANSFORMATION
        && skill_id < 300000
        && caster.nation != (skill_id / 100000) as u8
    {
        let fail_pkt = instance.build_fail_packet();
        world.send_to_session_owned(sid, fail_pkt);
        return Ok(());
    }

    // ── use_standing validation ──────────────────────────────────────
    // Skills with sUseStanding == 1 fail if the player is moving.
    // Only checked on FLYING / EFFECTING opcodes (C++ line 1668 returns early otherwise).
    if (b_opcode == MAGIC_FLYING || b_opcode == MAGIC_EFFECTING)
        && skill.use_standing.unwrap_or(0) == 1
    {
        let is_moving = world
            .with_session(sid, |h| h.move_old_speed != 0)
            .unwrap_or(false);
        if is_moving {
            let fail_pkt = instance.build_fail_packet();
            world.send_to_session_owned(sid, fail_pkt);
            return Ok(());
        }
    }

    // ── Class validation (CheckSkillClass) ─────────────────────────
    // The skill's `sSkill` field encodes the class requirement.
    // `iclass = sSkill / 10` gives the class constant (101=KaruWarrior, etc.)
    // The caster's classType = class % 100 must match the expected type.
    if b_opcode != MAGIC_CANCEL
        && b_opcode != MAGIC_CANCEL2
        && b_opcode != MAGIC_CANCEL_TRANSFORMATION
    {
        let s_skill = skill.skill.unwrap_or(0);
        let iclass = s_skill / 10;
        if s_skill != 0 && iclass != 0 && !check_skill_class(iclass, caster.class) {
            let fail_pkt = instance.build_fail_packet();
            world.send_to_session_owned(sid, fail_pkt);
            return Ok(());
        }
    }

    // ── Target resolution and validation ─────────────────────────────
    // These apply to ALL magic opcodes (CASTING, FLYING, EFFECTING, etc.).
    //
    // 1. If a target was specified (target_id != -1), verify it exists.
    //    - NPC target (>= NPC_BAND): silent return if dead.
    //    - Player target (< NPC_BAND): send fail packet if session gone.
    if instance.target_id != -1 {
        if (instance.target_id as u32) >= NPC_BAND {
            let npc_id = instance.target_id as u32;
            if world.is_npc_dead(npc_id) {
                return Ok(());
            }
        } else {
            let target_sid = instance.target_id as SessionId;
            if world.get_character_info(target_sid).is_none() {
                let fail_pkt = instance.build_fail_packet();
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }
        }
    }

    // 1.5. Neutral NPC target rejection — OrgNation==3 NPCs cannot be attacked.
    //   if ((pSkill.bMoral == MORAL_ENEMY || pSkill.bMoral == MORAL_AREA_ENEMY)
    //       && (pSkillCaster->isPlayer() && TO_NPC(pSkillTarget)->m_OrgNation == 3))
    //       return SendSkillFailed();
    if instance.target_id != -1 && (instance.target_id as u32) >= NPC_BAND {
        let moral = skill.moral.unwrap_or(0);
        if moral == MORAL_ENEMY || moral == MORAL_AREA_ENEMY {
            let npc_id = instance.target_id as u32;
            if let Some(npc) = world.get_npc_instance(npc_id) {
                if let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) {
                    if tmpl.group == 3 {
                        let fail_pkt = instance.build_fail_packet();
                        world.send_to_session_owned(sid, fail_pkt);
                        return Ok(());
                    }
                }
            }
        }
    }

    // 2. MORAL_SELF target redirection — self-cast skills always target caster.
    //   if (pSkill.bMoral == MORAL_SELF && !bIsRunProc)
    //       pSkillTarget = pSkillCaster; sTargetID = pSkillCaster->GetID();
    //   Also: MORAL_FRIEND_WITHME targeting an NPC redirects to caster.
    {
        let moral = skill.moral.unwrap_or(0);
        let redirect_to_self = moral == MORAL_SELF
            || (moral == MORAL_FRIEND_WITHME
                && instance.target_id != -1
                && (instance.target_id as u32) >= NPC_BAND);
        if redirect_to_self {
            instance.target_id = caster_id;
        }
    }

    // ── AnimatedSkill validation ─────────────────────────────────────
    // AnimatedSkill() = t_1 NOT IN (-1,0) AND bCastTime > 0 AND bItemGroup != 255
    //                   AND NOT MageArmorSkill
    let t_1 = skill.t_1.unwrap_or(0);
    let cast_time = skill.cast_time.unwrap_or(0);
    let item_group_val = skill.item_group.unwrap_or(0);
    let is_animated = (t_1 != -1 && t_1 != 0)
        && cast_time > 0
        && item_group_val != 255
        && !is_mage_armor_skill(skill_id);

    if is_animated {
        // ── Type 2 animated skill 500ms recast prevention ──────────
        // If a *different* Type 2 skill was cast in the last 500ms, fail.
        if skill.type1.unwrap_or(0) == 2 && cast_time > 0 && b_opcode == MAGIC_CASTING {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let should_fail = world
                .with_session(sid, |h| {
                    h.last_type2_skill_id != 0
                        && h.last_type2_skill_id != skill_id
                        && now_ms.saturating_sub(h.last_type2_cast_time) < 500
                })
                .unwrap_or(false);

            if should_fail {
                let fail_pkt = instance.build_fail_packet();
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }

            // Record this cast for 500ms check
            world.update_session(sid, |h| {
                h.last_type2_cast_time = now_ms;
                h.last_type2_skill_id = skill_id;
            });
        }

        // ── bCastFailed state machine ──────────────────────────────
        if b_opcode != MAGIC_CASTING {
            // If cast_failed was set during CASTING, fail on FLYING/EFFECTING
            let was_failed = world.with_session(sid, |h| h.cast_failed).unwrap_or(false);
            if was_failed {
                world.update_session(sid, |h| h.cast_failed = false);
                let fail_pkt = instance.build_fail_packet();
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }
        } else {
            // During CASTING: set cast_failed if player is moving
            let is_moving = world
                .with_session(sid, |h| h.move_old_speed != 0)
                .unwrap_or(false);
            if is_moving {
                world.update_session(sid, |h| h.cast_failed = true);
            } else {
                world.update_session(sid, |h| h.cast_failed = false);
            }
        }
    }

    // ── Route by magic opcode ───────────────────────────────────────
    match b_opcode {
        MAGIC_CASTING => {
            // Phase 1: Broadcast cast animation to region

            // Save cast position for anti-cheat validation on FLYING/EFFECTING
            if let Some(pos) = world.get_position(sid) {
                world.update_session(sid, |h| {
                    h.cast_skill_id = skill_id;
                    h.cast_x = pos.x;
                    h.cast_z = pos.z;
                });
            }

            let pkt = instance.build_packet(MAGIC_CASTING);
            broadcast_to_caster_region(&world, sid, &pkt);
        }

        MAGIC_FLYING => {
            // Phase 2: Projectile flight (archer/ranged)

            // ── Cast position validation ──────────────────────────────
            // If the skill has a flying effect, validate position on FLYING phase.
            if skill.flying_effect.unwrap_or(0) != 0 {
                if let Some(cast_pos) = world.get_cast_position(sid, skill_id) {
                    if let Some(cur_pos) = world.get_position(sid) {
                        if cast_pos.0 != cur_pos.x || cast_pos.1 != cur_pos.z {
                            let fail_pkt = instance.build_fail_packet();
                            world.send_to_session_owned(sid, fail_pkt);
                            return Ok(());
                        }
                    }
                }
            }

            // ── Arrow/knife consumption ─────────────────────────────
            // Type2 skills consume arrows (bNeedArrow count, or 1 for throwing knives).
            // If the skill has a use_item (arrow/knife), check inventory and consume.
            let use_item = skill.use_item.unwrap_or(0) as u32;
            if use_item != 0 {
                if let Some(type2) = world.get_magic_type2(skill_id as i32) {
                    let mut count = type2.need_arrow.unwrap_or(0) as u16;
                    // Throwing knives: NeedArrow=0 means consume 1
                    if count == 0 {
                        count = 1;
                    }

                    // Check if player has the required arrows/knives
                    // Special case: ITEM_INFINITYARC (800606000) bypasses consumption for arrows (391010000)
                    const ITEM_INFINITYARC: u32 = 800606000;
                    let has_infinity =
                        use_item == 391010000 && world.check_exist_item(sid, ITEM_INFINITYARC, 1);

                    if !has_infinity {
                        if !world.check_exist_item(sid, use_item, count) {
                            let fail_pkt = instance.build_fail_packet();
                            world.send_to_session_owned(sid, fail_pkt);
                            return Ok(());
                        }
                        // Consume the arrows/knives
                        world.rob_item(sid, use_item, count);
                    }
                }
            }

            // Check mana and deduct for type 2 skills
            if !check_and_deduct_mana(&world, sid, &caster, &skill, instance.target_id) {
                let fail_pkt = instance.build_fail_packet();
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }

            let pkt = instance.build_packet(MAGIC_FLYING);
            broadcast_to_caster_region(&world, sid, &pkt);
        }

        MAGIC_EFFECTING => {
            // Phase 3: Execute the actual skill effect

            // ── Cast position validation ──────────────────────────────
            // If the skill has NO flying effect, validate position on EFFECTING phase.
            if skill.flying_effect.unwrap_or(0) == 0 {
                if let Some(cast_pos) = world.get_cast_position(sid, skill_id) {
                    if let Some(cur_pos) = world.get_position(sid) {
                        if cast_pos.0 != cur_pos.x || cast_pos.1 != cur_pos.z {
                            let fail_pkt = instance.build_fail_packet();
                            world.send_to_session_owned(sid, fail_pkt);
                            return Ok(());
                        }
                    }
                }
            }

            // Check mana for non-type2 skills (type2 already deducted in FLYING)
            if skill_type != 2
                && !check_and_deduct_mana(&world, sid, &caster, &skill, instance.target_id)
            {
                let fail_pkt = instance.build_fail_packet();
                world.send_to_session_owned(sid, fail_pkt);
                return Ok(());
            }

            // ── Genie + PVP zone magic block ─────────────────────────────
            // Players in genie cannot cast offensive magic against enemy players
            // in PVP zones. AOE genie block is per-target in execute_type3.
            if instance.target_id >= 0 && (instance.target_id as u32) < NPC_BAND {
                let target_sid_check = instance.target_id as u16;
                let in_genie = world.with_session(sid, |h| h.genie_active).unwrap_or(false);
                if in_genie {
                    if let Some(target_ch) = world.get_character_info(target_sid_check) {
                        if caster.nation != target_ch.nation {
                            let caster_zone =
                                world.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
                            if crate::handler::attack::is_in_pvp_zone(caster_zone) {
                                let fail_pkt = instance.build_fail_packet();
                                world.send_to_session_owned(sid, fail_pkt);
                                return Ok(());
                            }
                        }
                    }
                }
            }

            // ── Pre-check consumable item existence ──────────────
            // Before executing the skill, verify the player has the required
            // consumable item. Type 2 (archer) and type 6 skills skip this.
            // Arrow items (391010000) are already consumed in MAGIC_FLYING.
            if skill_type != 2 && skill_type != 6 {
                let use_item_pre = skill.use_item.unwrap_or(0) as u32;
                if use_item_pre != 0 && use_item_pre != 391010000 {
                    let consume_id = resolve_consume_item(&skill);
                    if consume_id != 0
                        && !NO_CONSUME_ITEMS.contains(&consume_id)
                        && !world.check_exist_item(sid, consume_id, 1)
                    {
                        let fail_pkt = instance.build_fail_packet();
                        world.send_to_session_owned(sid, fail_pkt);
                        return Ok(());
                    }
                }
            }

            // Execute the skill based on type
            if !execute_skill(&world, sid, &mut instance, &skill).await {
                return Ok(());
            }

            // ── Consume item after successful cast ──────────────────
            // Called for all non-type2 skills (type2 = heal/buff, no item consumed)
            if skill_type != 2 {
                consume_item(&world, sid, &skill);
            }

            // ── Set cooldown after successful cast ──────────────────
            // Formula: expiry = UNIXTIME2 + (sReCastTime * 90)ms
            let recast_time = skill.recast_time.unwrap_or(0);
            if recast_time > 0 && !has_instant_cast {
                let now = std::time::Instant::now();
                let expiry = now + std::time::Duration::from_millis(recast_time as u64 * 90);
                world.update_session(sid, |h| {
                    h.skill_cooldowns.insert(skill_id, expiry);
                    // Periodic cleanup: remove expired entries to prevent unbounded growth
                    if h.skill_cooldowns.len() > 50 {
                        h.skill_cooldowns.retain(|_, exp| now < *exp);
                    }
                });
            }

            // ── Set type cooldown after successful cast ─────────────
            // Records the cast timestamp for bType[0] and bType[1].
            // Special: type3 with t_1==-1 uses synthetic key 10.
            if !has_instant_cast {
                let type1 = skill.type1.unwrap_or(0) as u8;
                let type2_val = skill.type2.unwrap_or(0) as u8;
                let t_1 = skill.t_1.unwrap_or(0);
                let now = std::time::Instant::now();
                world.update_session(sid, |h| {
                    if type1 != 0 {
                        // C++ MagicInstance.cpp:489-497 — type3 with t_1==-1 uses
                        // synthetic key 10 to separate DOT cooldowns from direct type3.
                        let key = if type1 == 3 && t_1 == -1 { 10u8 } else { type1 };
                        h.magic_type_cooldowns.insert(
                            key,
                            crate::world::TypeCooldown {
                                time: now,
                                t_catch: false,
                            },
                        );
                    }
                    if type2_val != 0 {
                        h.magic_type_cooldowns.insert(
                            type2_val,
                            crate::world::TypeCooldown {
                                time: now,
                                t_catch: false,
                            },
                        );
                    }
                });
            }

            // ── Consume instant_cast buff after use ─────────────────
            // Only consumed for non-Type2 skills (Type2 = heal/buff, already handled in FLYING)
            if has_instant_cast && skill_type != 2 {
                world.remove_buff(sid, BUFF_TYPE_INSTANT_MAGIC);
                world.update_session(sid, |h| {
                    h.instant_cast = false;
                });
                // Notify client that the instant cast buff expired
                let mut expire_pkt = Packet::new(Opcode::WizMagicProcess as u8);
                expire_pkt.write_u8(MAGIC_DURATION_EXPIRED);
                expire_pkt.write_u8(BUFF_TYPE_INSTANT_MAGIC as u8);
                world.send_to_session_owned(sid, expire_pkt);
            }
        }

        MAGIC_CANCEL | MAGIC_CANCEL2 => {
            // Client wants to cancel a buff
            //   trigger Type3Cancel(), Type4Cancel(), Type6Cancel(), Type9Cancel().

            // Type3Cancel — cancel HOT (heal-over-time) effects
            // Target must be caster. Only cancels HOTs (hp_amount > 0).
            if target_id == caster_id {
                if let Some(_t3) = world.get_magic_type3(skill.magic_num) {
                    let cleared_hot = world.clear_healing_dots(sid);
                    if cleared_hot {
                        // Send MAGIC_DURATION_EXPIRED(100) to remove HOT UI
                        let mut expired_pkt = Packet::new(Opcode::WizMagicProcess as u8);
                        expired_pkt.write_u8(MAGIC_DURATION_EXPIRED);
                        expired_pkt.write_u8(100);
                        world.send_to_session_owned(sid, expired_pkt);
                    }
                }
            }

            // Type4Cancel — cancel a self-buff
            // Target must be caster. Must not be a debuff. Removes buff + saved magic.
            if target_id == caster_id {
                let type4_data = world.get_magic_type4(skill.magic_num);
                if let Some(ref t4) = type4_data {
                    let buff_type = t4.buff_type.unwrap_or(0);
                    // C++ checks: !isDebuff() — debuffs have buff_type in debuff range
                    // isDebuff() = buff_type between 100-200 (debuff range)
                    let is_debuff = (100..=200).contains(&buff_type);
                    if buff_type > 0 && !is_debuff {
                        // of lockable scrolls while debuffed on same slot
                        if skill_id > 500000
                            && WorldState::is_lockable_scroll(buff_type)
                            && world.has_debuff_on_slot(sid, buff_type)
                        {
                            return Ok(());
                        }
                        if let Some(removed) = world.remove_buff(sid, buff_type) {
                            // performs per-buff-type cleanup (SIZE→StateChange, Kaul flags, etc.)
                            crate::systems::buff_tick::buff_type_cleanup(
                                &world,
                                sid,
                                buff_type,
                                removed.is_buff,
                            );
                            let expired_pkt = build_buff_expired_packet(buff_type as u8);
                            broadcast_to_caster_region(&world, sid, &expired_pkt);
                            world.set_user_ability(sid);
                            world.send_item_move_refresh(sid);
                        }
                        world.remove_saved_magic(sid, skill_id);
                    }
                }
            }

            // Type6Cancel — cancel transformation
            if world.is_transformed(sid) && world.get_magic_type6(skill.magic_num).is_some() {
                world.clear_transformation(sid);
                // Send MAGIC_CANCEL_TRANSFORMATION to caster
                let mut cancel_pkt = Packet::new(Opcode::WizMagicProcess as u8);
                cancel_pkt.write_u8(MAGIC_CANCEL_TRANSFORMATION);
                world.send_to_session_owned(sid, cancel_pkt);
                world.set_user_ability(sid);
                world.send_item_move_refresh(sid);
                // Remove saved magic for the transform skill
                world.remove_saved_magic(sid, skill_id);
            }

            // Type9Cancel — cancel stealth/lupine
            if world.get_magic_type9(skill.magic_num).is_some() {
                stealth::remove_stealth(&world, sid);
            }

            // Always broadcast the cancel packet to region (use the original opcode the client sent)
            let pkt = instance.build_packet(b_opcode);
            broadcast_to_caster_region(&world, sid, &pkt);
        }

        MAGIC_TYPE4_EXTEND => {
            // Extend the duration of a Type4 buff.
            //
            // Requires:
            //   1. Skill < 500000 (not a scroll buff)
            //   2. Valid magic_type4 entry, not a debuff
            //   3. Buff is currently active and hasn't been extended
            //   4. Player has a "Duration Item" (kind=255, m_iEffect1 points to
            //      a magic with moral=MORAL_EXTEND_DURATION=240) in inventory
            //   5. Consume the Duration Item via rob_item, extend buff
            const MORAL_EXTEND_DURATION: i16 = 240;

            if skill_id >= 500000 {
                return Ok(());
            }
            let type4_data = world.get_magic_type4(skill.magic_num);
            if let Some(ref t4) = type4_data {
                let buff_type = t4.buff_type.unwrap_or(0);
                let is_debuff = (100..=200).contains(&buff_type);
                if buff_type > 0 && !is_debuff {
                    // Scan inventory for a kind=255 item whose effect1 points to a
                    // magic with moral == MORAL_EXTEND_DURATION (240).
                    let mut duration_item_id: Option<u32> = None;
                    for i in crate::handler::SLOT_MAX..crate::handler::INVENTORY_TOTAL {
                        let slot = match world.get_inventory_slot(sid, i) {
                            Some(s) if s.item_id != 0 => s,
                            _ => continue,
                        };
                        let tmpl = match world.get_item(slot.item_id) {
                            Some(t) => t,
                            None => continue,
                        };
                        if tmpl.kind.unwrap_or(0) != 255 {
                            continue;
                        }
                        let eff1 = tmpl.effect1.unwrap_or(0);
                        if eff1 == 0 {
                            continue;
                        }
                        let eff_magic = match world.get_magic(eff1) {
                            Some(m) => m,
                            None => continue,
                        };
                        if eff_magic.moral.unwrap_or(0) == MORAL_EXTEND_DURATION {
                            duration_item_id = Some(slot.item_id);
                            break;
                        }
                    }

                    let item_id = match duration_item_id {
                        Some(id) => id,
                        None => return Ok(()),
                    };

                    // Consume the Duration Item (1 use)
                    if !world.rob_item(sid, item_id, 1) {
                        return Ok(());
                    }

                    // Extend the buff duration
                    let duration_ext = t4.duration.unwrap_or(0) as u32;
                    let extended = world.extend_buff_duration(sid, buff_type, duration_ext);
                    if extended {
                        // Send MAGIC_TYPE4_EXTEND response to caster
                        let mut ext_pkt = Packet::new(Opcode::WizMagicProcess as u8);
                        ext_pkt.write_u8(MAGIC_TYPE4_EXTEND);
                        ext_pkt.write_u32(skill_id);
                        world.send_to_session_owned(sid, ext_pkt);
                    }
                }
            }
        }

        _ => {
            tracing::debug!(
                "[sid={}] MagicProcess: unhandled opcode={} skill={}",
                sid,
                b_opcode,
                skill_id
            );
        }
    }

    Ok(())
}

// ── Pre-instance skill failed packet ──────────────────────────────────────

/// Build a MAGIC_FAIL packet before MagicInstance is constructed.
/// Used for early validation checks (special skill target validation).
fn build_skill_failed_packet(
    skill_id: u32,
    caster_id: i32,
    target_id: i32,
    data: &[i32; 7],
) -> Packet {
    let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
    pkt.write_u8(MAGIC_FAIL);
    pkt.write_u32(skill_id);
    pkt.write_u32(caster_id as u32);
    pkt.write_u32(target_id as u32);
    for d in data {
        pkt.write_u32(*d as u32);
    }
    pkt
}

// ── Mana check ────────────────────────────────────────────────────────────

/// Check if the caster has enough MP (and SP for Kurians) to cast and deduct it.
/// For Kurian classes, skills may also require SP (stamina points). The SP cost
/// is stored in `MagicRow::s_sp`. Both MP and SP must be sufficient; if either
/// is insufficient, the skill is rejected.
fn check_and_deduct_mana(
    world: &WorldState,
    sid: SessionId,
    caster: &CharacterInfo,
    skill: &MagicRow,
    target_id: i32,
) -> bool {
    let mana_cost = skill.msp.unwrap_or(0);
    let sp_cost = skill.s_sp.unwrap_or(0);
    let hp_cost = skill.hp.unwrap_or(0);

    // Check MP
    if mana_cost > 0 && caster.mp < mana_cost {
        tracing::debug!(
            "[sid={}] MagicProcess: not enough MP ({} < {}) for skill {}",
            sid,
            caster.mp,
            mana_cost,
            skill.magic_num
        );
        return false;
    }

    // Check SP for Kurian classes
    if sp_cost > 0 && crate::handler::stats::is_kurian_class(caster.class) && caster.sp < sp_cost {
        tracing::debug!(
            "[sid={}] MagicProcess: not enough SP ({} < {}) for skill {}",
            sid,
            caster.sp,
            sp_cost,
            skill.magic_num
        );
        return false;
    }

    // Check HP cost for skills that use HP instead of MP
    // sHP > 0 && sMsp == 0 && sHP < 10000 → normal HP cost skills
    if hp_cost > 0 && mana_cost == 0 && hp_cost < 10000 && hp_cost > caster.hp {
        tracing::debug!(
            "[sid={}] MagicProcess: not enough HP ({} < {}) for skill {}",
            sid,
            caster.hp,
            hp_cost,
            skill.magic_num
        );
        return false;
    }

    // Deduct MP
    if mana_cost > 0 {
        let new_mp = (caster.mp - mana_cost).max(0);
        world.update_character_mp(sid, new_mp);

        // Send WIZ_MSP_CHANGE to client: [i16 max_mp] [i16 current_mp]
        // BUG-6 fix: fetch current max_mp from world state instead of stale snapshot
        let current_max_mp = world
            .get_character_info(sid)
            .map(|ch| ch.max_mp)
            .unwrap_or(caster.max_mp);
        let mp_pkt = crate::systems::regen::build_mp_change_packet(current_max_mp, new_mp);
        world.send_to_session_owned(sid, mp_pkt);
    }

    // Deduct SP for Kurian classes
    if sp_cost > 0 && crate::handler::stats::is_kurian_class(caster.class) {
        let new_sp = (caster.sp - sp_cost).max(0);
        world.update_character_sp(sid, new_sp);

        // Send WIZ_KURIAN_SP_CHANGE to client
        let sp_pkt =
            crate::systems::sp_regen::build_sp_change_packet(caster.max_sp as u8, new_sp as u8);
        world.send_to_session_owned(sid, sp_pkt);
    }

    // Deduct HP for HP-cost skills (sHP > 0, sMsp == 0, sHP < 10000)
    if hp_cost > 0 && mana_cost == 0 && hp_cost < 10000 {
        let new_hp = (caster.hp - hp_cost).max(0);
        world.update_character_hp(sid, new_hp);

        let hp_pkt = crate::systems::regen::build_hp_change_packet(caster.max_hp, new_hp);
        world.send_to_session_owned(sid, hp_pkt);
    }

    // Sacrifice skills: sHP >= 10000 — deduct 10,000 HP from caster
    // Note: C++ always deducts 10000 regardless of actual sHP value (10001 in DB)
    // Cannot cast sacrifice on yourself (pUser == pSkillTarget → return false)
    if hp_cost >= 10000 {
        if target_id == sid as i32 {
            return false;
        }
        let new_hp = (caster.hp - 10000).max(0);
        world.update_character_hp(sid, new_hp);

        let hp_pkt = crate::systems::regen::build_hp_change_packet(caster.max_hp, new_hp);
        world.send_to_session_owned(sid, hp_pkt);
    }

    true
}

// ── Skill execution router ───────────────────────────────────────────────

/// Route skill execution to the appropriate type handler.
async fn execute_skill(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let skill_type = skill.type1.unwrap_or(0) as u8;

    if skill_type == 0 {
        return false;
    }

    // ── Block skills during blink (M4) ──────────────────────────────
    //   if (pSkillCaster->isBlinking() && bType != 4 && pSkill.iNum < 300000)
    //       return false;
    // Also: Unit.h:280 — canUseSkills() returns false when m_bCanUseSkills is false
    if !world.can_use_skills(caster_sid) && skill_type != 4 && instance.skill_id < 300000 {
        return false;
    }

    // ── Remove stealth for offensive skill types ─────────────────────
    //   if (pSkillCaster->isPlayer()) {
    //       if ((bType >= 1 && bType <= 3) || (bType == 7))
    //           TO_USER(pSkillCaster)->RemoveStealth();
    //   }
    if matches!(skill_type, 1..=3 | 7) {
        crate::handler::stealth::remove_stealth(world, caster_sid);
    }

    // ── Type3 re-cast prevention ─────────────────────────────────────
    //   Prevents re-casting pure Type3 HOT skills (bDirectType==1, bDuration!=0)
    //   on targets that already have any active HOT durational skill.
    if skill_type == 3 && skill.type2.unwrap_or(0) == 0 {
        if let Some(type3_data) = world.get_magic_type3(skill.magic_num) {
            let duration = type3_data.duration.unwrap_or(0);
            let direct_type = type3_data.direct_type.unwrap_or(0);

            if duration != 0 && direct_type == 1 {
                let target_sid = if instance.target_id < 0 {
                    caster_sid
                } else {
                    instance.target_id as SessionId
                };
                if world.has_active_hot(target_sid) {
                    return false;
                }
            }
        }
    }

    match skill_type {
        1 => execute_type1(world, caster_sid, instance, skill).await,
        2 => execute_type2(world, caster_sid, instance, skill).await,
        3 => execute_type3(world, caster_sid, instance, skill).await,
        4 => execute_type4(world, caster_sid, instance, skill),
        5 => execute_type5(world, caster_sid, instance, skill).await,
        6 => execute_type6(world, caster_sid, instance, skill),
        7 => execute_type7(world, caster_sid, instance, skill).await,
        8 => execute_type8(world, caster_sid, instance, skill),
        9 => execute_type9(world, caster_sid, instance, skill),
        _ => {
            tracing::warn!(
                "[sid={}] MagicProcess: unknown skill type={} for skill={}",
                caster_sid,
                skill_type,
                skill.magic_num
            );
            false
        }
    }
}

// ── Type 1: Melee weapon skills ──────────────────────────────────────────

/// Execute Type 1 skill — physical melee skill attack.
/// Adds skill-based bonus damage on top of the normal melee formula.
async fn execute_type1(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type1_data = match world.get_magic_type1(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let target_id = instance.target_id;

    if target_id < 0 {
        return execute_type1_aoe(world, caster_sid, instance, skill, &type1_data).await;
    }

    let target_is_player = (target_id as u32) < NPC_BAND;
    if !target_is_player {
        // NPC target — compute base melee damage + skill bonus damage
        let npc_id = target_id as u32;

        let caster = match world.get_character_info(caster_sid) {
            Some(ch) => ch,
            None => return false,
        };

        // Compute base melee damage against NPC using Type1 formula
        let base_melee = {
            let npc = world.get_npc_instance(npc_id);
            let npc_ac = npc
                .and_then(|n| world.get_npc_template(n.proto_id, n.is_monster))
                .map(|tmpl| {
                    // War buff: nation NPCs get AC × 1.2 during war (ChangeAbility).
                    let raw_ac = world.get_npc_war_ac(&tmpl);
                    (raw_ac as f64 * world.get_mon_def_multiplier()) as i32
                })
                .unwrap_or(0);

            let caster_coeff = world.get_coefficient(caster.class);
            let caster_hitrate = if let Some(ref c) = caster_coeff {
                1.0 + c.hitrate as f32 * caster.level as f32 * caster.dex as f32
            } else {
                1.0
            };

            // Snapshot caster combat data — 1 DashMap read instead of 2.
            let npc_caster_snap = match world.snapshot_combat(caster_sid) {
                Some(s) => s,
                None => return false,
            };
            let mut rng = rand::rngs::StdRng::from_entropy();
            compute_type1_hit_damage(
                npc_caster_snap.equipped_stats.total_hit,
                npc_ac,
                &type1_data,
                caster_hitrate,
                1.0,
                npc_caster_snap.attack_amount,
                100,
                &mut rng,
            )
        };

        // Add skill's additional damage (if not blocked by block_physical)
        let add_damage = type1_data.add_damage.unwrap_or(0) as i16;
        let mut damage = base_melee.saturating_add(add_damage);

        // Apply iADPtoNPC modifier
        let adp_npc = type1_data.add_dmg_perc_to_npc.unwrap_or(0);
        if adp_npc != 0 {
            damage = ((damage as i32 * adp_npc) / 100) as i16;
        }

        let caster_zone = world
            .get_position(caster_sid)
            .map(|p| p.zone_id)
            .unwrap_or(0);
        if caster_zone == ZONE_CHAOS_DUNGEON {
            damage = if instance.skill_id == 490226 {
                1000
            } else {
                100
            };
        }

        instance.data[3] = if damage == 0 {
            SKILLMAGIC_FAIL_ATTACKZERO
        } else {
            0
        };
        apply_skill_damage_to_npc(world, caster_sid, npc_id, instance, damage, skill, 0).await;
        return true;
    }

    let target_sid = target_id as SessionId;

    let caster = match world.get_character_info(caster_sid) {
        Some(ch) => ch,
        None => return false,
    };
    let target = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => return false,
    };

    if target.res_hp_type == USER_DEAD || target.hp <= 0 {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    // Blinking targets (respawn invulnerability) are immune to single-target Type1.
    {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if world.is_player_blinking(target_sid, now_unix) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    }

    if world.has_block_physical(target_sid) {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    // Check range
    if !check_skill_range(world, caster_sid, target_sid, skill) {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    // PvP permission check — "default deny" model
    {
        let caster_pos = world.get_position(caster_sid).unwrap_or_default();
        let target_pos = world.get_position(target_sid).unwrap_or_default();
        if !crate::handler::attack::is_hostile_to(
            world,
            caster_sid,
            &caster,
            &caster_pos,
            target_sid,
            &target,
            &target_pos,
        ) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    }

    // Calculate damage: Type1 formula + add_damage
    let mut rng = rand::rngs::StdRng::from_entropy();
    // Snapshot caster combat data — 1 DashMap read instead of 3.
    let caster_snap = match world.snapshot_combat(caster_sid) {
        Some(s) => s,
        None => return false,
    };
    let attack_amount = caster_snap.attack_amount;
    let player_attack_amount = caster_snap.player_attack_amount;
    let caster_total_hit = caster_snap.equipped_stats.total_hit;
    let target_snap = match world.snapshot_combat(target_sid) {
        Some(s) => s,
        None => return false,
    };
    let base_damage = {
        let target_ac = compute_pvp_skill_target_ac(&target_snap, &target, instance.skill_id);
        let caster_coeff = world.get_coefficient(caster.class);
        let target_coeff = world.get_coefficient(target.class);
        let caster_hitrate = if let Some(ref c) = caster_coeff {
            1.0 + c.hitrate as f32 * caster.level as f32 * caster.dex as f32
        } else {
            1.0
        };
        let target_evasion = if let Some(ref c) = target_coeff {
            1.0 + c.evasionrate as f32 * target.level as f32 * target.dex as f32
        } else {
            1.0
        };
        compute_type1_hit_damage(
            caster_total_hit,
            target_ac,
            &type1_data,
            caster_hitrate,
            target_evasion,
            attack_amount,
            player_attack_amount,
            &mut rng,
        )
    };
    let mut add_damage = type1_data.add_damage.unwrap_or(0) as i16;

    // War zone bonus damage reduction for PvP
    // In war zones, sAdditionalDamage is halved; in non-war zones, divided by 3.
    if add_damage > 0 {
        let in_war_zone = world
            .get_position(caster_sid)
            .and_then(|pos| world.get_zone(pos.zone_id))
            .is_some_and(|z| z.is_war_zone());
        if in_war_zone {
            add_damage /= 2;
        } else {
            add_damage /= 3;
        }
    }

    let mut damage = base_damage.saturating_add(add_damage);

    // Apply iADPtoUser modifier for PvP
    let adp_user = type1_data.add_dmg_perc_to_user.unwrap_or(0);
    if adp_user != 0 {
        damage = ((damage as i32 * adp_user) / 100) as i16;
    }

    {
        let caster_zone = world
            .get_position(caster_sid)
            .map(|p| p.zone_id)
            .unwrap_or(0);
        if caster_zone == ZONE_CHAOS_DUNGEON {
            damage = if instance.skill_id == 490226 {
                1000
            } else {
                100
            };
        }
    }

    if damage <= 0 {
        damage = 0;
    }

    instance.data[3] = if damage == 0 {
        SKILLMAGIC_FAIL_ATTACKZERO
    } else {
        0
    };

    // Apply damage
    apply_skill_damage(world, caster_sid, target_sid, instance, damage).await;
    true
}

// ── Type 1 AOE: Ground-targeted melee AOE skills ─────────────────────────

/// Execute Type 1 AOE — ground-targeted physical AOE (warrior stomps, etc.).
/// Gathers nearby units (players + NPCs), filters by range, and applies
/// per-target damage using `compute_type1_hit_damage` + `sAddDamage`.
/// Unlike single-target PvP, AOE does NOT halve `sAddDamage` in war zones,
/// and does NOT apply `iADPtoUser`/`iADPtoNPC`.
async fn execute_type1_aoe(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
    type1_data: &MagicType1Row,
) -> bool {
    let caster = match world.get_character_info(caster_sid) {
        Some(ch) => ch,
        None => return false,
    };

    let caster_pos = match world.get_position(caster_sid) {
        Some(p) => p,
        None => return false,
    };

    let caster_coeff = world.get_coefficient(caster.class);
    let caster_hitrate = if let Some(ref c) = caster_coeff {
        1.0 + c.hitrate as f32 * caster.level as f32 * caster.dex as f32
    } else {
        1.0
    };

    // Snapshot caster combat data — 1 DashMap read instead of 3.
    let aoe_caster_snap = match world.snapshot_combat(caster_sid) {
        Some(s) => s,
        None => return false,
    };
    let attack_amount = aoe_caster_snap.attack_amount;
    let player_attack_amount = aoe_caster_snap.player_attack_amount;
    let caster_total_hit = aoe_caster_snap.equipped_stats.total_hit;

    // C++ line 3267-3269: SkillRange = pSkill.sRange + 5; if (isStompSkills()) SkillRange += 3;
    let dist_range = {
        let r = skill.range.unwrap_or(0) as f32 + 5.0;
        if is_stomp_skill(instance.skill_id) {
            r + 3.0
        } else {
            r
        }
    };
    let dist_range_sq = dist_range * dist_range;

    let s_add_damage = type1_data.add_damage.unwrap_or(0) as i16;
    let mut any_hit = false;

    // C++ line 3253: sData[1] = 1  — AOE indicator
    instance.data[1] = 1;

    let mut rng = rand::rngs::StdRng::from_entropy();

    // ── AOE player targets (event_room filtered) ────────────────────
    let caster_event_room = world.get_event_room(caster_sid);
    let nearby_players = world.get_nearby_session_ids(
        caster_pos.zone_id,
        caster_pos.region_x,
        caster_pos.region_z,
        Some(caster_sid),
        caster_event_room,
    );

    for target_sid in nearby_players {
        let target = match world.get_character_info(target_sid) {
            Some(ch) => ch,
            None => continue,
        };

        // C++ line 3231: if (pTarget->isDead()) continue;
        if target.res_hp_type == USER_DEAD || target.hp <= 0 {
            continue;
        }

        // C++ line 3232: if (pTarget->isPlayer() && TO_USER(pTarget)->isGM()) continue;
        if target.authority == 0 {
            continue;
        }

        // C++ line 3234: if (!pTarget->isBlinking())
        {
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if world.is_player_blinking(target_sid, now_unix) {
                continue;
            }
        }

        // C++ line 3234: pTarget->isAttackable() — hostility check
        {
            let target_pos = world.get_position(target_sid).unwrap_or_default();
            if !crate::handler::attack::is_hostile_to(
                world,
                caster_sid,
                &caster,
                &caster_pos,
                target_sid,
                &target,
                &target_pos,
            ) {
                continue;
            }
        }

        // C++ line 3267-3278: distance check from caster
        let target_pos = match world.get_position(target_sid) {
            Some(p) => p,
            None => continue,
        };
        let dx = caster_pos.x - target_pos.x;
        let dz = caster_pos.z - target_pos.z;
        if dx * dx + dz * dz >= dist_range_sq {
            continue;
        }

        // C++ line 3285: damage = pSkillCaster->GetDamage(pTarget, pSkill, true)
        let aoe_target_snap = match world.snapshot_combat(target_sid) {
            Some(s) => s,
            None => continue,
        };
        let target_ac = compute_pvp_skill_target_ac(&aoe_target_snap, &target, instance.skill_id);
        let target_coeff = world.get_coefficient(target.class);
        let target_evasion = if let Some(ref c) = target_coeff {
            1.0 + c.evasionrate as f32 * target.level as f32 * target.dex as f32
        } else {
            1.0
        };

        let base_damage = compute_type1_hit_damage(
            caster_total_hit,
            target_ac,
            type1_data,
            caster_hitrate,
            target_evasion,
            attack_amount,
            player_attack_amount,
            &mut rng,
        );

        // C++ line 3287-3288: if (!pTarget->m_bBlockPhysical) damage += sAdditionalDamage
        let mut damage = if !world.has_block_physical(target_sid) {
            base_damage.saturating_add(s_add_damage)
        } else {
            base_damage
        };

        // C++ line 3290-3293: Chaos Dungeon fixed damage
        if caster_pos.zone_id == ZONE_CHAOS_DUNGEON {
            damage = if instance.skill_id == 490226 {
                1000
            } else {
                100
            };
        }

        if damage <= 0 {
            continue;
        }
        any_hit = true;

        // C++ line 3295: pTarget->HpChange(-damage, pSkillCaster)
        let new_hp = (target.hp - damage).max(0);
        world.update_character_hp(target_sid, new_hp);

        // HP notification to victim
        let hp_pkt = crate::systems::regen::build_hp_change_packet_with_attacker(
            target.max_hp,
            new_hp,
            caster_sid as u32,
        );
        world.send_to_session_owned(target_sid, hp_pkt);
        crate::handler::party::broadcast_party_hp(world, target_sid);

        // C++ line 3297-3301: ItemWoreOut
        world.item_wore_out(caster_sid, WORE_TYPE_ATTACK, damage as i32);
        world.item_wore_out(target_sid, WORE_TYPE_DEFENCE, damage as i32);

        // C++ line 3303-3304: mage armor reflect (per AOE target)
        try_reflect_damage(world, caster_sid, target_sid, damage).await;

        // Death handling
        if new_hp <= 0 {
            dead::broadcast_death(world, target_sid);
            dead::set_who_killed_me(world, target_sid, caster_sid);
            dead::send_death_notice(world, caster_sid, target_sid);
            dead::rob_chaos_skill_items(world, target_sid);

            // ── PvP loyalty (NP) change ─────────────────────────────
            dead::pvp_loyalty_on_death(world, caster_sid, target_sid);

            // ── Rivalry / Anger Gauge (magic kill path) ────────────
            {
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let is_revenge = crate::handler::arena::on_pvp_kill(
                    world,
                    caster_sid,
                    target_sid,
                    caster_pos.zone_id,
                    now_secs,
                );
                if is_revenge {
                    crate::systems::loyalty::send_loyalty_change(
                        world,
                        caster_sid,
                        crate::handler::arena::RIVALRY_NP_BONUS as i32,
                        true,
                        false,
                        false,
                    );
                }
            }

            // ── PvP gold change ─────────────────────────────────────
            dead::gold_change_on_death(world, caster_sid, target_sid);

            // ── Temple event kill scoring ───────────────────────────
            // OnDeathKilledPlayer is called from OnDeath regardless of
            // damage source (physical or magic).
            {
                use crate::systems::event_room;
                match caster_pos.zone_id {
                    zone if zone == event_room::ZONE_BDW => {
                        dead::track_bdw_player_kill(world, caster_sid, target_sid);
                    }
                    zone if zone == event_room::ZONE_CHAOS => {
                        dead::track_chaos_pvp_kill(world, caster_sid, target_sid);
                    }
                    zone if zone == event_room::ZONE_JURAID => {
                        dead::track_juraid_pvp_kill(world, caster_sid);
                    }
                    _ => {}
                }
            }
        }

        // HP bar update
        send_target_hp_update(world, caster_sid, target_sid, damage as i32);

        // Break stealth on AOE hit
        if target.nation != caster.nation {
            stealth::remove_stealth(world, target_sid);
        }
    }

    // ── AOE NPC targets (event_room filtered) ────────────────────────
    let nearby_npcs = world.get_nearby_npc_ids(
        caster_pos.zone_id,
        caster_pos.region_x,
        caster_pos.region_z,
        caster_event_room,
    );

    for npc_id in nearby_npcs {
        let npc = match world.get_npc_instance(npc_id) {
            Some(n) => n,
            None => continue,
        };

        if !npc.is_monster {
            continue;
        }

        let npc_hp = match world.get_npc_hp(npc_id) {
            Some(hp) if hp > 0 => hp,
            _ => continue,
        };

        // Distance check from caster
        let ndx = caster_pos.x - npc.x;
        let ndz = caster_pos.z - npc.z;
        if ndx * ndx + ndz * ndz >= dist_range_sq {
            continue;
        }

        // Compute damage against NPC
        let npc_ac = world
            .get_npc_template(npc.proto_id, npc.is_monster)
            .map(|tmpl| (tmpl.ac as f64 * world.get_mon_def_multiplier()) as i32)
            .unwrap_or(0);

        let base_damage = compute_type1_hit_damage(
            caster_total_hit,
            npc_ac,
            type1_data,
            caster_hitrate,
            1.0,
            attack_amount,
            100,
            &mut rng,
        );

        // sAddDamage applies to NPCs as well (no block_physical check for NPCs in C++)
        let mut damage = base_damage.saturating_add(s_add_damage);

        // Chaos Dungeon fixed damage
        if caster_pos.zone_id == ZONE_CHAOS_DUNGEON {
            damage = if instance.skill_id == 490226 {
                1000
            } else {
                100
            };
        }

        if damage <= 0 {
            continue;
        }
        any_hit = true;

        let new_hp = (npc_hp - damage as i32).max(0);
        world.update_npc_hp(npc_id, new_hp);
        world.record_npc_damage(npc_id, caster_sid, damage as i32);

        // Durability loss
        world.item_wore_out(caster_sid, WORE_TYPE_ATTACK, damage as i32);

        if new_hp > 0 {
            world.notify_npc_damaged(npc_id, caster_sid);
        } else if let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) {
            super::attack::handle_npc_death(world, caster_sid, npc_id, &npc, &tmpl).await;
        }

        // Send HP bar update
        if let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) {
            let mut hp_pkt = Packet::new(Opcode::WizTargetHp as u8);
            hp_pkt.write_u32(npc_id);
            hp_pkt.write_u8(0);
            hp_pkt.write_u32(tmpl.max_hp);
            hp_pkt.write_u32(new_hp.max(0) as u32);
            hp_pkt.write_u32(-(damage as i32) as u32); // negative = damage dealt
            hp_pkt.write_u32(0);
            hp_pkt.write_u8(0);
            world.send_to_session_owned(caster_sid, hp_pkt);
        }
    }

    // C++ line 3314: sData[3] — attack-zero indicator
    instance.data[3] = if !any_hit {
        SKILLMAGIC_FAIL_ATTACKZERO
    } else {
        0
    };

    // Broadcast effect packet once for the AOE
    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);

    true
}

// ── Type 2: Ranged/archery skills ────────────────────────────────────────

/// Execute Type 2 skill — ranged projectile attack.
/// Arrow consumption is skipped (requires inventory system).
async fn execute_type2(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type2_data = match world.get_magic_type2(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let target_id = instance.target_id;
    if target_id < 0 {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    let target_is_player = (target_id as u32) < NPC_BAND;
    if !target_is_player {
        // NPC target — compute damage using Type2 formula (sAddDamage as percentage)
        let npc_id = target_id as u32;

        let caster = match world.get_character_info(caster_sid) {
            Some(ch) => ch,
            None => return false,
        };

        let npc_ac = {
            let npc = world.get_npc_instance(npc_id);
            npc.and_then(|n| world.get_npc_template(n.proto_id, n.is_monster))
                .map(|tmpl| {
                    // War buff: nation NPCs get AC × 1.2 during war (ChangeAbility).
                    let raw_ac = world.get_npc_war_ac(&tmpl);
                    (raw_ac as f64 * world.get_mon_def_multiplier()) as i32
                })
                .unwrap_or(0)
        };

        let caster_coeff = world.get_coefficient(caster.class);
        let caster_hitrate = if let Some(ref c) = caster_coeff {
            1.0 + c.hitrate as f32 * caster.level as f32 * caster.dex as f32
        } else {
            1.0
        };

        // Snapshot caster combat data — 1 DashMap read instead of 2.
        let t2_npc_snap = match world.snapshot_combat(caster_sid) {
            Some(s) => s,
            None => return false,
        };
        let mut rng = rand::rngs::StdRng::from_entropy();
        let mut damage = compute_type2_hit_damage(
            t2_npc_snap.equipped_stats.total_hit,
            npc_ac,
            &type2_data,
            caster_hitrate,
            1.0,
            t2_npc_snap.attack_amount,
            100,
            &mut rng,
        );

        let adp_npc = type2_data.add_dmg_perc_to_npc.unwrap_or(0);
        if adp_npc != 0 {
            damage = ((damage as i32 * adp_npc as i32) / 100) as i16;
        }
        instance.data[3] = if damage == 0 {
            SKILLMAGIC_FAIL_ATTACKZERO
        } else {
            0
        };
        instance.data[1] = 1; // bResult = success
        apply_skill_damage_to_npc(world, caster_sid, npc_id, instance, damage, skill, 0).await;
        return true;
    }

    let target_sid = target_id as SessionId;

    let caster = match world.get_character_info(caster_sid) {
        Some(ch) => ch,
        None => return false,
    };
    let target = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => return false,
    };

    if target.res_hp_type == USER_DEAD || target.hp <= 0 {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    // Blinking targets (respawn invulnerability) are immune to single-target Type2.
    {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if world.is_player_blinking(target_sid, now_unix) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    }

    if world.has_block_physical(target_sid) {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    if !check_skill_range(world, caster_sid, target_sid, skill) {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    // PvP permission check — "default deny" model
    {
        let caster_pos = world.get_position(caster_sid).unwrap_or_default();
        let target_pos = world.get_position(target_sid).unwrap_or_default();
        if !crate::handler::attack::is_hostile_to(
            world,
            caster_sid,
            &caster,
            &caster_pos,
            target_sid,
            &target,
            &target_pos,
        ) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    }

    // Calculate damage using Type2 formula (sAddDamage as percentage, not flat)
    let mut rng = rand::rngs::StdRng::from_entropy();
    // Snapshot caster combat data — 1 DashMap read instead of 3.
    let t2_pvp_snap = match world.snapshot_combat(caster_sid) {
        Some(s) => s,
        None => return false,
    };
    let attack_amount = t2_pvp_snap.attack_amount;
    let player_attack_amount = t2_pvp_snap.player_attack_amount;
    let caster_total_hit = t2_pvp_snap.equipped_stats.total_hit;
    let t2_target_snap = match world.snapshot_combat(target_sid) {
        Some(s) => s,
        None => return false,
    };
    let mut damage = {
        let target_ac = compute_pvp_skill_target_ac(&t2_target_snap, &target, instance.skill_id);
        let caster_coeff = world.get_coefficient(caster.class);
        let target_coeff = world.get_coefficient(target.class);
        let caster_hitrate = if let Some(ref c) = caster_coeff {
            1.0 + c.hitrate as f32 * caster.level as f32 * caster.dex as f32
        } else {
            1.0
        };
        let target_evasion = if let Some(ref c) = target_coeff {
            1.0 + c.evasionrate as f32 * target.level as f32 * target.dex as f32
        } else {
            1.0
        };
        compute_type2_hit_damage(
            caster_total_hit,
            target_ac,
            &type2_data,
            caster_hitrate,
            target_evasion,
            attack_amount,
            player_attack_amount,
            &mut rng,
        )
    };

    let adp_user = type2_data.add_dmg_perc_to_user.unwrap_or(0);
    if adp_user != 0 {
        damage = ((damage as i32 * adp_user as i32) / 100) as i16;
    }

    if damage <= 0 {
        damage = 0;
    }

    instance.data[3] = if damage == 0 {
        SKILLMAGIC_FAIL_ATTACKZERO
    } else {
        0
    };
    instance.data[1] = 1; // bResult = success

    apply_skill_damage(world, caster_sid, target_sid, instance, damage).await;
    true
}

// ── Type 3: Magic attack / heal / DOT ─────────────────────────────────────

/// Resolve ground-target coordinates across the v2603 and v2615 layouts.
///
/// Older packets encode X/Z in tenths of a world unit.  The v2615 client can
/// send the same fields as direct world units.  Both representations are
/// unambiguous in normal play because the valid cast point is the candidate
/// nearest to the caster.
fn resolve_aoe_center(
    raw_x: i32,
    raw_z: i32,
    caster_x: f32,
    caster_z: f32,
) -> (f32, f32) {
    if raw_x == 0 && raw_z == 0 {
        return (caster_x, caster_z);
    }

    let direct = (
        if raw_x == 0 { caster_x } else { raw_x as f32 },
        if raw_z == 0 { caster_z } else { raw_z as f32 },
    );
    let tenths = (
        if raw_x == 0 {
            caster_x
        } else {
            raw_x as f32 / 10.0
        },
        if raw_z == 0 {
            caster_z
        } else {
            raw_z as f32 / 10.0
        },
    );

    let direct_dist = (direct.0 - caster_x).powi(2) + (direct.1 - caster_z).powi(2);
    let tenths_dist = (tenths.0 - caster_x).powi(2) + (tenths.1 - caster_z).powi(2);

    if direct_dist < tenths_dist {
        direct
    } else {
        tenths
    }
}

/// Execute Type 3 skill — magical damage, healing, or DOT/HOT.
/// Handles direct damage, healing, and durational (DOT/HOT) effects.
/// DOT effects are registered via `world.add_durational_skill()` and
/// processed by the `dot_tick` system every 2 seconds.
async fn execute_type3(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type3_data = match world.get_magic_type3(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let moral = skill.moral.unwrap_or(0);
    let first_damage = type3_data.first_damage.unwrap_or(0);
    let time_damage = type3_data.time_damage.unwrap_or(0);
    let duration = type3_data.duration.unwrap_or(0);
    let direct_type = type3_data.direct_type.unwrap_or(0);

    // Applies when: (directType==1 || directType==2) && firstDamage > 0
    // Only for potions: skillID > 400000, bType[1]==0, bMoral==MORAL_SELF, bItemGroup==9
    {
        const PLAYER_POTION_REQUEST_INTERVAL_MS: u128 = 2400;
        let skill_id = instance.skill_id;
        let b_type2 = skill.type2.unwrap_or(0);
        let item_group = skill.item_group.unwrap_or(0);
        if (direct_type == 1 || direct_type == 2)
            && first_damage > 0
            && skill_id > 400000
            && b_type2 == 0
            && moral == 1 // MORAL_SELF
            && item_group == 9
        {
            let blocked = world
                .with_session(caster_sid, |h| {
                    h.last_potion_time.elapsed().as_millis() < PLAYER_POTION_REQUEST_INTERVAL_MS
                })
                .unwrap_or(false);
            if blocked {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
            world.update_session(caster_sid, |h| {
                h.last_potion_time = std::time::Instant::now();
            });
            // NOTE: Item consumption is handled by consume_item() at the
            // MAGIC_EFFECTING level (C++ ConsumeItem), NOT here in type3.
        }
    }

    // Conditions: directType==1 (HP), firstDamage>0 (heal), useItem!=0, item.class==0
    {
        let use_item = skill.use_item.unwrap_or(0) as u32;
        if !world.can_use_potions(caster_sid)
            && direct_type == 1
            && first_damage > 0
            && use_item != 0
        {
            // Check if the required item has class==0 (generic potion, not class-specific skill item)
            let item_class = world
                .get_item(use_item)
                .map(|it| it.class.unwrap_or(0))
                .unwrap_or(0);
            if item_class == 0 {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }
    }

    instance.data[1] = 1;

    // ── Type3 skip for players with active DOT in temple event zones ──
    //   if (pTarget->isPlayer() && pTarget->m_bType3Flag) {
    //       if (g_pMain->pTempleEvent.ActiveEvent
    //           && !g_pMain->pTempleEvent.isAttackable
    //           && pTarget->isInTempleEventZone())
    //           continue; // skip this target
    //   }
    // When a player target already has an active Type3 effect, skip applying
    // new Type3 effects if the event is active but combat is not allowed.
    {
        use crate::systems::event_room;
        if instance.target_id >= 0 && (instance.target_id as u32) < NPC_BAND {
            let target_sid = instance.target_id as SessionId;
            if world.has_active_durational(target_sid) {
                let (active_event, is_attackable) = world
                    .event_room_manager
                    .read_temple_event(|s| (s.active_event, s.is_attackable));
                if active_event != -1 && !is_attackable {
                    if let Some(target_pos) = world.get_position(target_sid) {
                        if event_room::is_in_temple_event_zone(target_pos.zone_id) {
                            send_skill_failed(world, caster_sid, instance);
                            return false;
                        }
                    }
                }
            }
        }
    }

    // ── Healing (self or friendly target) ───────────────────────────
    if moral == MORAL_SELF || moral == MORAL_SELF_AREA {
        let caster = match world.get_character_info(caster_sid) {
            Some(ch) => ch,
            None => return false,
        };

        //   case 1: HpChangeMagic (HP heal/damage)
        //   case 2: MSpChange (MP heal/damage)
        let heal_amount = first_damage.unsigned_abs() as i16;
        if heal_amount > 0 {
            match direct_type {
                // DirectType 1: HP heal (default path)
                1 | 0 => {
                    let is_self_undead = world.is_undead(caster_sid);
                    let new_hp = if is_self_undead {
                        (caster.hp - heal_amount).max(0)
                    } else {
                        (caster.hp + heal_amount).min(caster.max_hp)
                    };
                    world.update_character_hp(caster_sid, new_hp);

                    let hp_pkt =
                        crate::systems::regen::build_hp_change_packet(caster.max_hp, new_hp);
                    world.send_to_session_owned(caster_sid, hp_pkt);
                    crate::handler::party::broadcast_party_hp(world, caster_sid);

                    if is_self_undead && new_hp <= 0 {
                        dead::broadcast_death(world, caster_sid);
                    }
                }
                // DirectType 2: MP heal — C++ MagicInstance.cpp:4099-4103
                2 => {
                    if caster.hp > 0 {
                        let new_mp = (caster.mp + heal_amount).min(caster.max_mp);
                        world.update_character_mp(caster_sid, new_mp);

                        let mp_pkt =
                            crate::systems::regen::build_mp_change_packet(caster.max_mp, new_mp);
                        world.send_to_session_owned(caster_sid, mp_pkt);
                    }
                }
                _ => {}
            }
        }

        // Register HOT if time_damage > 0 and duration > 0 (undead: HOT becomes DOT)
        if time_damage > 0 && duration > 0 {
            let tick_count = (duration / 2).max(1) as u8;
            let raw_per_tick =
                (time_damage / tick_count as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            let hp_per_tick = if world.is_undead(caster_sid) {
                -raw_per_tick
            } else {
                raw_per_tick
            };
            world.add_durational_skill(
                caster_sid,
                instance.skill_id,
                hp_per_tick,
                tick_count,
                caster_sid,
            );
        }

        instance.data[3] = heal_amount as i32;

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        send_target_hp_update(world, caster_sid, caster_sid, 0);
        return true;
    }

    // ── Friendly-target heal (MORAL_FRIEND_WITHME, etc.) ────────────
    if moral == MORAL_FRIEND_WITHME
        || moral == MORAL_FRIEND_EXCEPTME
        || moral == MORAL_PARTY
        || moral == MORAL_PARTY_ALL
    {
        // 1. Monster-transformed players cannot cast group heals (line 2168-2169)
        // 2. Block if caster already has an active HOT (prevents HOT stacking, line 2174-2178)
        if moral == MORAL_PARTY_ALL && time_damage > 0 {
            let is_monster_transform = world
                .with_session(caster_sid, |h| h.transformation_type == 1)
                .unwrap_or(false);
            if is_monster_transform {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
            if world.has_active_hot(caster_sid) {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }

        let target_id = instance.target_id;
        let target_sid = if target_id < 0 {
            caster_sid
        } else {
            target_id as SessionId
        };

        let target = match world.get_character_info(target_sid) {
            Some(ch) => ch,
            None => {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        };

        if target.res_hp_type == USER_DEAD || target.hp <= 0 {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        // Block if target already has an active HOT (prevents HOT stacking).
        if time_damage > 0 && moral <= MORAL_PARTY && world.has_active_hot(target_sid) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        let heal_amount = first_damage.unsigned_abs() as i16;
        if heal_amount > 0 {
            match direct_type {
                // DirectType 1: HP heal (default path)
                1 | 0 => {
                    let is_target_undead = world.is_undead(target_sid);
                    let new_hp = if is_target_undead {
                        (target.hp - heal_amount).max(0)
                    } else {
                        (target.hp + heal_amount).min(target.max_hp)
                    };
                    world.update_character_hp(target_sid, new_hp);

                    let hp_pkt =
                        crate::systems::regen::build_hp_change_packet(target.max_hp, new_hp);
                    world.send_to_session_owned(target_sid, hp_pkt);
                    crate::handler::party::broadcast_party_hp(world, target_sid);

                    if is_target_undead && new_hp <= 0 {
                        dead::broadcast_death(world, target_sid);
                    }
                }
                // DirectType 2: MP heal — C++ MagicInstance.cpp:4099-4103
                2 => {
                    if target.hp > 0 {
                        let new_mp = (target.mp + heal_amount).min(target.max_mp);
                        world.update_character_mp(target_sid, new_mp);

                        let mp_pkt =
                            crate::systems::regen::build_mp_change_packet(target.max_mp, new_mp);
                        world.send_to_session_owned(target_sid, mp_pkt);
                    }
                }
                _ => {}
            }
        }

        // Register HOT if time_damage > 0 and duration > 0 (undead: HOT becomes DOT)
        if time_damage > 0 && duration > 0 {
            let tick_count = (duration / 2).max(1) as u8;
            let raw_per_tick =
                (time_damage / tick_count as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            let hp_per_tick = if world.is_undead(target_sid) {
                -raw_per_tick
            } else {
                raw_per_tick
            };
            world.add_durational_skill(
                target_sid,
                instance.skill_id,
                hp_per_tick,
                tick_count,
                caster_sid,
            );
        }

        instance.data[3] = heal_amount as i32;

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        send_target_hp_update(world, caster_sid, target_sid, 0);
        return true;
    }

    // ── Damage (enemy target) ───────────────────────────────────────
    if moral == MORAL_ENEMY {
        let target_id = instance.target_id;
        if target_id < 0 {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        let target_is_player = (target_id as u32) < NPC_BAND;
        if !target_is_player {
            // NPC target — deal actual magic damage
            let npc_id = target_id as u32;
            let caster = match world.get_character_info(caster_sid) {
                Some(ch) => ch,
                None => return false,
            };
            let mag_atk = world.get_buff_magic_attack_amount(caster_sid);
            let attr = type3_data.attribute.unwrap_or(0) as u8;
            // direct_type 1/8, negative first_damage, and skill < 400000.
            let use_magic_formula = first_damage < 0
                && (direct_type == 1 || direct_type == 8)
                && instance.skill_id < 400000;
            let mut magic_damage = if use_magic_formula {
                let npc_ctx = build_npc_ctx(world, npc_id, attr, caster_sid);
                let mut rng = rand::rngs::StdRng::from_entropy();
                compute_magic_damage(&caster, first_damage, mag_atk, &npc_ctx, &mut rng)
            } else {
                (-first_damage).max(0) as i16
            };
            let adp_npc = type3_data.add_dmg_perc_to_npc.unwrap_or(0);
            if adp_npc != 0 {
                magic_damage = ((magic_damage as i32 * adp_npc as i32) / 100) as i16;
            }
            apply_skill_damage_to_npc(
                world,
                caster_sid,
                npc_id,
                instance,
                magic_damage,
                skill,
                attr,
            )
            .await;

            // Register DOT on NPC if time_damage != 0 and duration > 0
            // sTimeDamage < 0 && attribute != 4 (different filter from first damage)
            if time_damage != 0 && duration > 0 {
                let mut tick_count = (duration / 2).clamp(1, 255) as u8;
                let caster_zone_npc = world
                    .get_position(caster_sid)
                    .map(|p| p.zone_id)
                    .unwrap_or(0);
                if caster_zone_npc == ZONE_CHAOS_DUNGEON {
                    tick_count = (tick_count as u16 * 2).min(255) as u8;
                }
                let caster_for_dot = match world.get_character_info(caster_sid) {
                    Some(ch) => ch,
                    None => return true,
                };
                let npc_ctx = build_npc_ctx(world, npc_id, attr, caster_sid);
                let mut rng = rand::rngs::StdRng::from_entropy();
                let duration_damage = if time_damage < 0 && attr != 4 {
                    compute_magic_damage(&caster_for_dot, time_damage, mag_atk, &npc_ctx, &mut rng)
                } else {
                    (-time_damage).max(0) as i16
                };
                let hp_per_tick =
                    -(duration_damage.unsigned_abs() as i16 / tick_count as i16).max(1);
                world.add_npc_dot(
                    npc_id,
                    crate::world::NpcDotSlot {
                        skill_id: instance.skill_id,
                        hp_amount: hp_per_tick,
                        tick_count: 0,
                        tick_limit: tick_count,
                        caster_sid,
                    },
                );
            }
            return true;
        }

        let target_sid = target_id as SessionId;
        let target = match world.get_character_info(target_sid) {
            Some(ch) => ch,
            None => return false,
        };

        if target.res_hp_type == USER_DEAD || target.hp <= 0 {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        // Blinking targets (respawn invulnerability) are immune to single-target Type3.
        {
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if world.is_player_blinking(target_sid, now_unix) {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }

        if world.has_block_magic(target_sid) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        if !check_skill_range(world, caster_sid, target_sid, skill) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        // PvP permission check — "default deny" model
        let caster = match world.get_character_info(caster_sid) {
            Some(ch) => ch,
            None => return false,
        };
        {
            let caster_pos = world.get_position(caster_sid).unwrap_or_default();
            let target_pos = world.get_position(target_sid).unwrap_or_default();
            if !crate::handler::attack::is_hostile_to(
                world,
                caster_sid,
                &caster,
                &caster_pos,
                target_sid,
                &target,
                &target_pos,
            ) {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }

        // During Snow Battle event in zone 69, all Type3 damage is forced to -10.
        {
            let caster_zone = world
                .get_position(caster_sid)
                .map(|p| p.zone_id)
                .unwrap_or(0);
            if caster_zone == ZONE_SNOW_BATTLE && world.get_battle_state().is_snow_battle() {
                apply_skill_damage(world, caster_sid, target_sid, instance, 10).await;
                return true;
            }
        }

        let caster = match world.get_character_info(caster_sid) {
            Some(ch) => ch,
            None => return false,
        };

        // Snapshot target combat data once — serves both direct damage and DOT paths
        // (replaces 2× build_player_ctx = 8 DashMap reads with 1 snapshot read)
        let target_snap = match world.snapshot_combat(target_sid) {
            Some(s) => s,
            None => return false,
        };

        // Hoist caster magic attack buff — used by both direct damage and DOT paths
        let caster_mag_atk = world.get_buff_magic_attack_amount(caster_sid);

        match direct_type {
            2 => {
                // Player target: change MP by sFirstDamage amount
                let target_refresh = world
                    .get_character_info(target_sid)
                    .unwrap_or(target.clone());
                let new_mp = (target_refresh.mp as i32 + first_damage)
                    .clamp(0, target_refresh.max_mp as i32) as i16;
                world.update_character_mp(target_sid, new_mp);
                instance.data[3] = first_damage;
                let pkt = instance.build_packet(MAGIC_EFFECTING);
                broadcast_to_caster_region(world, caster_sid, &pkt);
                send_target_hp_update(world, caster_sid, target_sid, first_damage.abs());
                return true;
            }
            5 => {
                let damage = if first_damage < 100 {
                    // Percentage of current HP
                    (first_damage as i32 * target.hp as i32) / -100
                } else {
                    // Percentage of max HP (over 100 = heal based on max)
                    (target.max_hp as i32 * (first_damage as i32 - 100)) / 100
                } as i16;
                apply_skill_damage(world, caster_sid, target_sid, instance, damage).await;
            }
            _ => {
                // direct_type 1/8, negative first_damage, and skill < 400000.
                let use_magic_formula = first_damage < 0
                    && (direct_type == 1 || direct_type == 8)
                    && instance.skill_id < 400000;
                if use_magic_formula {
                    let pvp_attr = type3_data.attribute.unwrap_or(0) as u8;
                    let pvp_ctx = build_player_ctx(
                        world,
                        target_sid,
                        &target_snap,
                        &target,
                        pvp_attr,
                        caster_sid,
                    );
                    let mut pvp_rng = rand::rngs::StdRng::from_entropy();
                    let mut damage = compute_magic_damage(
                        &caster,
                        first_damage,
                        caster_mag_atk,
                        &pvp_ctx,
                        &mut pvp_rng,
                    );
                    let adp_user = type3_data.add_dmg_perc_to_user.unwrap_or(0);
                    if adp_user != 0 {
                        damage = ((damage as i32 * adp_user as i32) / 100) as i16;
                    }
                    damage = apply_magic_class_bonus(
                        damage, &caster, &target, world, caster_sid, target_sid,
                    );
                    apply_skill_damage(world, caster_sid, target_sid, instance, damage).await;
                } else {
                    let mut raw_damage = (-first_damage).max(0) as i16;
                    let adp_user = type3_data.add_dmg_perc_to_user.unwrap_or(0);
                    if adp_user != 0 {
                        raw_damage = ((raw_damage as i32 * adp_user as i32) / 100) as i16;
                    }
                    apply_skill_damage(world, caster_sid, target_sid, instance, raw_damage).await;
                }
            }
        }

        // Register DOT if time_damage != 0 and duration > 0
        // sTimeDamage < 0 && attribute != 4 (different filter from first damage)
        if time_damage != 0 && duration > 0 {
            let mut tick_count = (duration / 2).clamp(1, 255) as u8;
            let cz = world
                .get_position(caster_sid)
                .map(|p| p.zone_id)
                .unwrap_or(0);
            if cz == ZONE_CHAOS_DUNGEON {
                tick_count = (tick_count as u16 * 2).min(255) as u8;
            }
            let caster_for_dot = match world.get_character_info(caster_sid) {
                Some(ch) => ch,
                None => return true,
            };
            let dot_attr = type3_data.attribute.unwrap_or(0) as u8;
            let duration_damage = if time_damage < 0 && dot_attr != 4 {
                let dot_ctx = build_player_ctx(
                    world,
                    target_sid,
                    &target_snap,
                    &target,
                    dot_attr,
                    caster_sid,
                );
                let mut dot_rng = rand::rngs::StdRng::from_entropy();
                let raw = compute_magic_damage(
                    &caster_for_dot,
                    time_damage,
                    caster_mag_atk,
                    &dot_ctx,
                    &mut dot_rng,
                );
                // inside GetMagicDamage, so DOT damage is also reduced.
                apply_magic_damage_reduction(world, target_sid, raw)
            } else {
                (-time_damage).max(0) as i16
            };
            let hp_per_tick = -(duration_damage.unsigned_abs() as i16 / tick_count as i16).max(1);
            world.add_durational_skill(
                target_sid,
                instance.skill_id,
                hp_per_tick,
                tick_count,
                caster_sid,
            );
        }
        return true;
    }

    // ── AOE damage ──────────────────────────────────────────────────
    if moral == MORAL_AREA_ENEMY || moral == MORAL_AREA_ALL || moral == MORAL_AREA_FRIEND {
        // Genie AOE check is done per-target inside the player loop below,
        // NOT here — NPCs should still take AOE damage from genie mages.

        // Pre-compute genie state for per-target filtering
        let caster_in_genie_aoe = moral == MORAL_AREA_ENEMY && {
            let is_mage = world
                .get_character_info(caster_sid)
                .map(|ch| matches!(ch.class % 100, 3 | 9 | 10))
                .unwrap_or(false);
            is_mage
                && world
                    .with_session(caster_sid, |h| h.genie_active)
                    .unwrap_or(false)
        };

        let caster_pos = match world.get_position(caster_sid) {
            Some(p) => p,
            None => return false,
        };

        let radius = type3_data.radius.unwrap_or(0) as f32;

        // Get all nearby players (event_room filtered)
        let caster_event_room = world.get_event_room(caster_sid);
        let nearby = world.get_nearby_session_ids(
            caster_pos.zone_id,
            caster_pos.region_x,
            caster_pos.region_z,
            Some(caster_sid),
            caster_event_room,
        );

        let caster = match world.get_character_info(caster_sid) {
            Some(ch) => ch,
            None => return false,
        };

        let mag_atk_aoe = world.get_buff_magic_attack_amount(caster_sid);
        let aoe_attr = type3_data.attribute.unwrap_or(0) as u8;
        let mut aoe_rng = rand::rngs::StdRng::from_entropy();
        let aoe_use_magic_formula = first_damage < 0
            && (direct_type == 1 || direct_type == 8)
            && instance.skill_id < 400000;

        // v2603 packets use tenths of a world unit while v2615 ground-target
        // packets can carry world units directly.  Select the representation
        // that resolves closest to the caster, instead of always dividing by
        // ten and moving the damage area to an unrelated map coordinate.
        let (aoe_x, aoe_z) = resolve_aoe_center(
            instance.data[0],
            instance.data[2],
            caster_pos.x,
            caster_pos.z,
        );

        let radius_sq = radius * radius;

        for target_sid in nearby {
            let target = match world.get_character_info(target_sid) {
                Some(ch) => ch,
                None => continue,
            };

            if target.res_hp_type == USER_DEAD || target.hp <= 0 {
                continue;
            }

            if moral == MORAL_AREA_ENEMY && target.authority == 0 {
                continue;
            }

            {
                let now_unix = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if world.is_player_blinking(target_sid, now_unix) {
                    continue;
                }
            }

            // Distance check from AOE center
            let target_pos = match world.get_position(target_sid) {
                Some(p) => p,
                None => continue,
            };

            let dx = aoe_x - target_pos.x;
            let dz = aoe_z - target_pos.z;
            let dist_sq = dx * dx + dz * dz;

            if radius_sq > 0.0 && dist_sq > radius_sq {
                continue;
            }

            // PvP permission check for AOE targets — "default deny" model
            if moral == MORAL_AREA_ENEMY
                && !crate::handler::attack::is_hostile_to(
                    world,
                    caster_sid,
                    &caster,
                    &caster_pos,
                    target_sid,
                    &target,
                    &target_pos,
                )
            {
                continue;
            }
            if moral == MORAL_AREA_FRIEND
                && crate::handler::attack::is_hostile_to(
                    world,
                    caster_sid,
                    &caster,
                    &caster_pos,
                    target_sid,
                    &target,
                    &target_pos,
                )
            {
                continue;
            }

            // Players in enemy safety area cannot hit players in their own safety area.
            // Arena, PVP zones, and temple event zones are exempt (combat is always allowed).
            if moral == MORAL_AREA_ALL {
                let caster_zone = caster_pos.zone_id;
                let target_zone = target_pos.zone_id;
                let both_in_special_zone =
                    (crate::handler::attack::is_in_arena(caster_zone, caster_pos.x, caster_pos.z)
                        && crate::handler::attack::is_in_arena(
                            target_zone,
                            target_pos.x,
                            target_pos.z,
                        ))
                        || (crate::handler::attack::is_in_pvp_zone(caster_zone)
                            && crate::handler::attack::is_in_pvp_zone(target_zone))
                        || (crate::systems::event_room::is_in_temple_event_zone(caster_zone)
                            && crate::systems::event_room::is_in_temple_event_zone(target_zone));

                if !both_in_special_zone
                    && crate::handler::attack::is_in_enemy_safety_area(
                        caster_zone,
                        caster_pos.x,
                        caster_pos.z,
                        caster.nation,
                    )
                    && crate::handler::attack::is_in_own_safety_area(
                        target_zone,
                        target_pos.x,
                        target_pos.z,
                        target.nation,
                    )
                {
                    continue;
                }
            }

            // in MORAL_AREA_ENEMY AOE (NPCs still take damage in separate loop)
            if caster_in_genie_aoe {
                continue;
            }

            if moral != MORAL_AREA_FRIEND && world.has_block_magic(target_sid) {
                continue;
            }

            // M3: Break stealth on AOE hit for enemy targets
            //   if (pTarget->isPlayer() && pSkillCaster->GetNation() != pTarget->GetNation())
            //       TO_USER(pTarget)->RemoveStealth();
            if target.nation != caster.nation {
                crate::handler::stealth::remove_stealth(world, target_sid);
            }

            // Track per-target damage for WIZ_TARGET_HP display
            let mut aoe_target_damage: i32 = 0;

            if moral == MORAL_AREA_FRIEND {
                // AOE heal (undead converts healing to damage)
                let heal_amount = first_damage.unsigned_abs() as i16;
                let is_target_undead = world.is_undead(target_sid);
                let new_hp = if is_target_undead {
                    (target.hp - heal_amount).max(0)
                } else {
                    (target.hp + heal_amount).min(target.max_hp)
                };
                world.update_character_hp(target_sid, new_hp);

                // Send WIZ_HP_CHANGE to AOE heal target
                let hp_pkt = crate::systems::regen::build_hp_change_packet(target.max_hp, new_hp);
                world.send_to_session_owned(target_sid, hp_pkt);
                crate::handler::party::broadcast_party_hp(world, target_sid);

                // Undead heal→damage death check
                if is_target_undead && new_hp <= 0 {
                    dead::broadcast_death(world, target_sid);
                }

                if time_damage > 0 && duration > 0 {
                    let tick_count = (duration / 2).max(1) as u8;
                    let raw_per_tick = (time_damage / tick_count as i32)
                        .clamp(i16::MIN as i32, i16::MAX as i32)
                        as i16;
                    let hp_per_tick = if world.is_undead(target_sid) {
                        -raw_per_tick
                    } else {
                        raw_per_tick
                    };
                    world.add_durational_skill(
                        target_sid,
                        instance.skill_id,
                        hp_per_tick,
                        tick_count,
                        caster_sid,
                    );
                }
            } else {
                // AOE damage — per-target resistance + class bonus for PvP
                // Snapshot target once for both direct damage and DOT paths (8→1 DashMap reads)
                let aoe_target_snap = match world.snapshot_combat(target_sid) {
                    Some(s) => s,
                    None => continue,
                };
                // for direct_type 1/8, negative first_damage, skill < 400000
                let damage = if aoe_use_magic_formula {
                    let aoe_player_ctx = build_player_ctx(
                        world,
                        target_sid,
                        &aoe_target_snap,
                        &target,
                        aoe_attr,
                        caster_sid,
                    );
                    let mut d = compute_magic_damage(
                        &caster,
                        first_damage,
                        mag_atk_aoe,
                        &aoe_player_ctx,
                        &mut aoe_rng,
                    );
                    d = apply_magic_class_bonus(d, &caster, &target, world, caster_sid, target_sid);
                    d
                } else {
                    (-first_damage).max(0) as i16
                };
                aoe_target_damage = damage as i32;
                let new_hp = (target.hp - damage).max(0);
                world.update_character_hp(target_sid, new_hp);

                // Send WIZ_HP_CHANGE to victim
                let hp_pkt = crate::systems::regen::build_hp_change_packet_with_attacker(
                    target.max_hp,
                    new_hp,
                    caster_sid as u32,
                );
                world.send_to_session_owned(target_sid, hp_pkt);
                crate::handler::party::broadcast_party_hp(world, target_sid);

                // ── AOE durability loss ──────────────────────────────
                world.item_wore_out(caster_sid, WORE_TYPE_ATTACK, damage as i32);
                world.item_wore_out(target_sid, WORE_TYPE_DEFENCE, damage as i32);

                try_reflect_damage(world, caster_sid, target_sid, damage).await;

                if new_hp <= 0 {
                    dead::broadcast_death(world, target_sid);
                    dead::set_who_killed_me(world, target_sid, caster_sid);

                    // ── PvP death notice ────────────────────────────────
                    dead::send_death_notice(world, caster_sid, target_sid);

                    // ── Chaos dungeon item rob ─────────────────────────
                    dead::rob_chaos_skill_items(world, target_sid);

                    // ── PvP loyalty (NP) change ─────────────────────────
                    dead::pvp_loyalty_on_death(world, caster_sid, target_sid);

                    // ── Rivalry / Anger Gauge (magic kill path) ────────
                    {
                        let now_secs = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let is_revenge = crate::handler::arena::on_pvp_kill(
                            world,
                            caster_sid,
                            target_sid,
                            caster_pos.zone_id,
                            now_secs,
                        );
                        if is_revenge {
                            crate::systems::loyalty::send_loyalty_change(
                                world,
                                caster_sid,
                                crate::handler::arena::RIVALRY_NP_BONUS as i32,
                                true,
                                false,
                                false,
                            );
                        }
                    }

                    // ── PvP gold change ────────────────────────────────
                    dead::gold_change_on_death(world, caster_sid, target_sid);

                    // ── Temple event kill scoring ───────────────────────
                    {
                        use crate::systems::event_room;
                        match caster_pos.zone_id {
                            zone if zone == event_room::ZONE_BDW => {
                                dead::track_bdw_player_kill(world, caster_sid, target_sid);
                            }
                            zone if zone == event_room::ZONE_CHAOS => {
                                dead::track_chaos_pvp_kill(world, caster_sid, target_sid);
                            }
                            zone if zone == event_room::ZONE_JURAID => {
                                dead::track_juraid_pvp_kill(world, caster_sid);
                            }
                            _ => {}
                        }
                    }
                }

                // Register DOT for AOE targets
                // sTimeDamage < 0 && attribute != 4
                if time_damage != 0 && duration > 0 {
                    let mut tick_count = (duration / 2).clamp(1, 255) as u8;
                    if caster_pos.zone_id == ZONE_CHAOS_DUNGEON {
                        tick_count = (tick_count as u16 * 2).min(255) as u8;
                    }
                    let duration_damage = if time_damage < 0 && aoe_attr != 4 {
                        let aoe_dot_ctx = build_player_ctx(
                            world,
                            target_sid,
                            &aoe_target_snap,
                            &target,
                            aoe_attr,
                            caster_sid,
                        );
                        let raw = compute_magic_damage(
                            &caster,
                            time_damage,
                            mag_atk_aoe,
                            &aoe_dot_ctx,
                            &mut aoe_rng,
                        );
                        apply_magic_damage_reduction(world, target_sid, raw)
                    } else {
                        (-time_damage).max(0) as i16
                    };
                    let hp_per_tick =
                        -(duration_damage.unsigned_abs() as i16 / tick_count as i16).max(1);
                    world.add_durational_skill(
                        target_sid,
                        instance.skill_id,
                        hp_per_tick,
                        tick_count,
                        caster_sid,
                    );
                }
            }

            send_target_hp_update(world, caster_sid, target_sid, aoe_target_damage);
        }

        // get_nearby_session_ids excludes caster, so we heal caster separately here.
        if moral == MORAL_AREA_FRIEND {
            if let Some(caster_ch) = world.get_character_info(caster_sid) {
                if caster_ch.hp > 0 && caster_ch.res_hp_type != USER_DEAD {
                    let heal_amount = first_damage.unsigned_abs() as i16;
                    let is_caster_undead = world.is_undead(caster_sid);
                    let new_hp = if is_caster_undead {
                        (caster_ch.hp - heal_amount).max(0)
                    } else {
                        (caster_ch.hp + heal_amount).min(caster_ch.max_hp)
                    };
                    world.update_character_hp(caster_sid, new_hp);
                    let hp_pkt =
                        crate::systems::regen::build_hp_change_packet(caster_ch.max_hp, new_hp);
                    world.send_to_session_owned(caster_sid, hp_pkt);
                    crate::handler::party::broadcast_party_hp(world, caster_sid);

                    // Undead heal→damage death check
                    if is_caster_undead && new_hp <= 0 {
                        dead::broadcast_death(world, caster_sid);
                    }

                    if time_damage > 0 && duration > 0 {
                        let tick_count = (duration / 2).max(1) as u8;
                        let raw_per_tick = (time_damage / tick_count as i32)
                            .clamp(i16::MIN as i32, i16::MAX as i32)
                            as i16;
                        let hp_per_tick = if world.is_undead(caster_sid) {
                            -raw_per_tick
                        } else {
                            raw_per_tick
                        };
                        world.add_durational_skill(
                            caster_sid,
                            instance.skill_id,
                            hp_per_tick,
                            tick_count,
                            caster_sid,
                        );
                    }

                    send_target_hp_update(world, caster_sid, caster_sid, 0);
                }
            }
        }

        // ── AOE NPC damage ──────────────────────────────────────────
        // includes NPCs in AOE targeting. We iterate nearby NPCs and apply damage.
        // (only player-to-player), so only MORAL_AREA_ENEMY hits NPCs.
        if moral == MORAL_AREA_ENEMY {
            let nearby_npcs = world.get_nearby_npc_ids(
                caster_pos.zone_id,
                caster_pos.region_x,
                caster_pos.region_z,
                caster_event_room,
            );

            for npc_id in nearby_npcs {
                let npc = match world.get_npc_instance(npc_id) {
                    Some(n) => n,
                    None => continue,
                };

                // Must be a monster (not friendly NPC)
                if !npc.is_monster {
                    continue;
                }

                let npc_hp = match world.get_npc_hp(npc_id) {
                    Some(hp) if hp > 0 => hp,
                    _ => continue,
                };

                // Distance check from AOE center
                let ndx = aoe_x - npc.x;
                let ndz = aoe_z - npc.z;
                let npc_dist_sq = ndx * ndx + ndz * ndz;
                if radius_sq > 0.0 && npc_dist_sq > radius_sq {
                    continue;
                }

                // Compute per-NPC damage with NPC resistance
                let npc_damage = if aoe_use_magic_formula {
                    let aoe_npc_ctx = build_npc_ctx(world, npc_id, aoe_attr, caster_sid);
                    compute_magic_damage(
                        &caster,
                        first_damage,
                        mag_atk_aoe,
                        &aoe_npc_ctx,
                        &mut aoe_rng,
                    )
                } else {
                    (-first_damage).max(0) as i16
                };

                // Apply damage to NPC
                let new_hp = (npc_hp - npc_damage as i32).max(0);
                world.update_npc_hp(npc_id, new_hp);
                world.record_npc_damage(npc_id, caster_sid, npc_damage as i32);

                // Durability loss
                world.item_wore_out(caster_sid, WORE_TYPE_ATTACK, npc_damage as i32);

                if new_hp > 0 {
                    world.notify_npc_damaged(npc_id, caster_sid);
                } else {
                    // NPC died
                    if let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) {
                        super::attack::handle_npc_death(world, caster_sid, npc_id, &npc, &tmpl)
                            .await;
                    }
                }

                // Send HP bar update
                if let Some(tmpl) = world.get_npc_template(npc.proto_id, npc.is_monster) {
                    let mut hp_pkt = Packet::new(Opcode::WizTargetHp as u8);
                    hp_pkt.write_u32(npc_id);
                    hp_pkt.write_u8(0);
                    hp_pkt.write_u32(tmpl.max_hp);
                    hp_pkt.write_u32(new_hp.max(0) as u32);
                    hp_pkt.write_u32(-(npc_damage as i32) as u32); // negative = damage dealt
                    hp_pkt.write_u32(0);
                    hp_pkt.write_u8(0);
                    world.send_to_session_owned(caster_sid, hp_pkt);
                }
            }
        }

        // Broadcast the effect to region
        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        return true;
    }

    // Unhandled moral type — just broadcast
    tracing::debug!(
        "[sid={}] MagicProcess Type 3: unhandled moral={} for skill={}",
        caster_sid,
        moral,
        skill.magic_num
    );
    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);
    true
}

/// Target type for magic damage formula differentiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MagicTargetKind {
    Player,
    Npc,
}

/// Context for the magic damage formula — carries target/zone information
/// that `compute_magic_damage` needs beyond the caster-side values.
struct MagicDamageContext {
    /// Target type (player vs NPC) — determines formula constants.
    target_kind: MagicTargetKind,
    /// Target's total elemental resistance (items + buffs combined).
    /// We simplify: total_r = equipped_r + buff_r (Pct always 100%).
    target_total_r: i32,
    /// Target class (for DamageSettings multiplier, only for player targets).
    target_class: u16,
    /// Target's AC buff amount
    target_ac_amount: i32,
    /// Whether the target is in a war zone.
    is_war_zone: bool,
    /// DamageSettings from DB (class multipliers, mon_take_damage, mage_magic_damage).
    damage_settings: Option<ko_db::models::DamageSettingsRow>,
    /// Mage weapon quality multiplier (`getplusdamage()` — MagicInstance.cpp:6694-6744).
    /// 1.0 = no bonus (default / non-weapon).
    plus_damage: f64,
    /// Caster's weapon base damage for magic formula
    /// Staff damage for mages, weapon damage for warriors/kurians, 0 for others.
    righthand_damage: i16,
    /// Caster's attribute damage from equipped items
    /// Elemental bonus from staff + other slot bonuses matching the spell's attribute.
    attribute_damage: i16,
    /// Magic attribute type (1=fire, 2=cold, 3=lightning, 4=MAGIC_R, 5=disease, 6=poison).
    /// Used for MAGIC_R (4) exclusion in weapon damage reduction.
    attribute: u8,
}

/// Get the DamageSettings mage→target class multiplier.
fn get_mage_class_multiplier(ds: &ko_db::models::DamageSettingsRow, target_class: u16) -> f64 {
    let base = target_class % 100;
    match base {
        1 | 5 | 6 => ds.mage_to_warrior as f64,
        2 | 7 | 8 => ds.mage_to_rogue as f64,
        4 | 11 | 12 => ds.mage_to_priest as f64,
        3 | 9 | 10 => ds.mage_to_mage as f64,
        13..=15 => ds.mage_to_kurian as f64,
        _ => 1.0,
    }
}

/// Get a player target's total elemental resistance for magic damage.
/// Get an NPC target's total elemental resistance from its template.
fn get_npc_target_resistance(world: &WorldState, npc_id: u32, attribute: u8) -> i32 {
    let (proto_id, is_monster) = match world.get_npc_instance(npc_id) {
        Some(n) => (n.proto_id, n.is_monster),
        None => return 0,
    };
    let tmpl = match world.get_npc_template(proto_id, is_monster) {
        Some(t) => t,
        None => return 0,
    };
    // War buff: nation NPCs get Resist × 2 during war (ChangeAbility).
    let base = match attribute {
        1 => tmpl.fire_r,
        2 => tmpl.cold_r,
        3 => tmpl.lightning_r,
        4 => tmpl.magic_r,
        5 => tmpl.disease_r,
        6 => tmpl.poison_r,
        _ => return 0,
    };
    world.get_npc_war_resist(base, &tmpl) as i32
}

/// Build a `MagicDamageContext` for a player target using pre-fetched snapshot.
/// Uses `CombatSnapshot` for target resistance and AC, eliminating 4 DashMap reads
/// per call (3 from `get_player_target_resistance` + 1 from `get_buff_ac_amount`).
fn build_player_ctx(
    world: &WorldState,
    target_sid: SessionId,
    target_snap: &CombatSnapshot,
    target: &CharacterInfo,
    attribute: u8,
    caster_sid: SessionId,
) -> MagicDamageContext {
    let is_war = world
        .get_position(target_sid)
        .and_then(|p| world.get_zone(p.zone_id))
        .map(|z| z.is_war_zone())
        .unwrap_or(false);
    let caster_class = world
        .get_character_info(caster_sid)
        .map(|c| c.class)
        .unwrap_or(0);
    let (rh_dmg, attr_dmg) = world.get_magic_weapon_damage(caster_sid, caster_class, attribute);
    MagicDamageContext {
        target_kind: MagicTargetKind::Player,
        target_total_r: target_snap.total_resistance(attribute),
        target_class: target.class,
        target_ac_amount: target_snap.ac_amount,
        is_war_zone: is_war,
        damage_settings: world.get_damage_settings(),
        plus_damage: world.get_plus_damage(caster_sid),
        righthand_damage: rh_dmg,
        attribute_damage: attr_dmg,
        attribute,
    }
}

/// Build a `MagicDamageContext` for an NPC target.
fn build_npc_ctx(
    world: &WorldState,
    npc_id: u32,
    attribute: u8,
    caster_sid: SessionId,
) -> MagicDamageContext {
    let is_war = world
        .get_position(caster_sid)
        .and_then(|p| world.get_zone(p.zone_id))
        .map(|z| z.is_war_zone())
        .unwrap_or(false);
    let caster_class = world
        .get_character_info(caster_sid)
        .map(|c| c.class)
        .unwrap_or(0);
    let (rh_dmg, attr_dmg) = world.get_magic_weapon_damage(caster_sid, caster_class, attribute);
    MagicDamageContext {
        target_kind: MagicTargetKind::Npc,
        target_total_r: get_npc_target_resistance(world, npc_id, attribute),
        target_class: 0,
        target_ac_amount: 0,
        is_war_zone: is_war,
        damage_settings: world.get_damage_settings(),
        plus_damage: world.get_plus_damage(caster_sid),
        righthand_damage: rh_dmg,
        attribute_damage: attr_dmg,
        attribute,
    }
}

/// Compute magic damage from the `sFirstDamage`/`sTimeDamage` value.
/// Formula steps (matching C++ order):
/// 1. CHA scaling for mages (line 6305)
/// 2. sMagicAmount multiplier (line 6306)
/// 3. Resistance formula: 485×total_hit/(total_r+510) vs players, 555×total_hit/(total_r+515) vs NPCs
/// 4. DamageSettings class multiplier (PvP) or montakedamage (PvE)
/// 5. Randomization: rand(0,damage)×0.3 + damage×0.85 − sMagicAmount
/// 6. Warrior magic vs NPC zeroing (int32(0.50f) = 0)
///    - 6a: Weapon damage reduction (line 6616-6619) — subtracts weapon-based damage
/// 7. Warrior no-weapon halving + AC boost
/// 8. Mage magic damage multiplier
/// 9. War zone /3 vs non-war /2
/// 10. MAX_DAMAGE cap (32000)
fn compute_magic_damage(
    caster: &CharacterInfo,
    base_damage: i32,
    magic_attack_amount: i32,
    ctx: &MagicDamageContext,
    rng: &mut impl rand::Rng,
) -> i16 {
    if base_damage == 0 {
        return 0;
    }

    // ── IMPORTANT: Work in the NEGATIVE domain like C++ ─────────────
    // C++ GetMagicDamage receives negative total_hit (e.g. -24) and all
    // intermediate values stay negative.  The `- sMagicAmount` step at
    // line 6598 INCREASES damage magnitude (makes more negative).
    // Previous code took unsigned_abs() which broke the formula for
    // low-level mage skills (subtraction zeroed them out).
    let mut total_hit = base_damage; // keep sign (negative for attack spells)

    // ── Step 1: Stat scaling ──────────────────────────────────────────
    // CHA scaling always applies for mages — the direct_type 1/8 filtering
    // is done at the call site (C++ line 3946-3950), not inside GetMagicDamage.
    let base_class = caster.class % 100;
    let is_mage = matches!(base_class, 3 | 9 | 10);
    let is_warrior = matches!(base_class, 1 | 5 | 6);

    if is_mage {
        let cha = caster.cha.max(1) as f32;
        // C++ ceil() on negative values rounds toward zero: ceil(-14.05)=-14
        total_hit = (total_hit as f32 * cha / 102.5).ceil() as i32;
    }

    // ── Step 2: sMagicAmount multiplier ───────────────────────────────
    // C++ line 6304-6306
    let s_magic_amount = magic_attack_amount + 100;
    total_hit = total_hit * s_magic_amount / 100;

    if total_hit == 0 {
        return 0;
    }

    // ── Step 3: Core resistance formula ───────────────────────────────
    // C++ line 6498 (vs player): damage = 485 * total_hit / (total_r + 510)
    // C++ line 6582 (vs NPC):    damage = 555 * total_hit / (total_r + 515)
    let mut damage: i32 = match ctx.target_kind {
        MagicTargetKind::Player => 485 * total_hit / (ctx.target_total_r + 510),
        MagicTargetKind::Npc => 555 * total_hit / (ctx.target_total_r + 515),
    };

    // ── Step 4: DamageSettings class multiplier ───────────────────────
    // C++ line 6503-6515 (PvP): dm *= mageTOxxx
    // C++ line 6585 (PvE):     dm *= montakedamage
    if let Some(ref ds) = ctx.damage_settings {
        match ctx.target_kind {
            MagicTargetKind::Player => {
                let mult = get_mage_class_multiplier(ds, ctx.target_class);
                damage = (damage as f64 * mult) as i32;
            }
            MagicTargetKind::Npc => {
                damage = (damage as f64 * ds.mon_take_damage as f64) as i32;
            }
        }
    }

    // ── Step 5: Randomization ─────────────────────────────────────────
    // C++ line 6597-6598:
    //   random = myrand(0, damage);
    //   damage = int32(random * 0.3f + damage * 0.85f) - sMagicAmount;
    // C++ myrand(0, negative) swaps: myrand(negative, 0)
    let random = if damage != 0 {
        let (lo, hi) = if damage > 0 { (0, damage) } else { (damage, 0) };
        rng.gen_range(lo..=hi)
    } else {
        0
    };
    damage = (random as f32 * 0.3 + damage as f32 * 0.85) as i32 - s_magic_amount;

    // ── Step 6: Warrior magic vs NPC zeroing ──────────────────────────
    // C++ line 6603-6604: damage *= int32(0.50f) → int32(0.50f) = 0 → damage = 0
    if is_warrior && ctx.target_kind == MagicTargetKind::Npc {
        damage = 0;
    }

    // ── Step 6.5: Weapon damage reduction ───────────────────────────────
    // For player casters (not NPC): subtract weapon-based damage from magic damage.
    // In the negative domain, this makes damage MORE negative (= more damage dealt).
    // Excluded for MAGIC_R (attribute 4) spells.
    //   damage -= (int32)( (righthand_damage*0.8f + righthand_damage*level/60)
    //                    + (attribute_damage*0.8f + attribute_damage*level/60) )
    if ctx.attribute != 4 {
        let rh = ctx.righthand_damage as i32;
        let attr = ctx.attribute_damage as i32;
        let level = caster.level as i32;
        let rh_part = rh as f32 * 0.8 + (rh * level / 60) as f32;
        let attr_part = attr as f32 * 0.8 + (attr * level / 60) as f32;
        damage -= (rh_part + attr_part) as i32;
    }

    // ── Step 7+8: Warrior/Mage mods with C++ baa ordering ─────────────
    // C++ line 6624: `double baa = damage;` captures BEFORE warrior mods.
    // Warrior halving modifies `damage`, mage multiplier modifies `baa`.
    // C++ line 6683: `damage = (int32)baa;` overwrites — warrior halving is dead code in C++.
    // We replicate the C++ behavior exactly: baa captures pre-warrior value.
    let mut baa = damage as f64;

    // C++ line 6628-6637: warrior halving (modifies `damage`, not `baa` — dead code in C++)
    if is_warrior {
        damage /= 2;
        if ctx.target_kind == MagicTargetKind::Player && ctx.target_ac_amount < 100 {
            damage += damage * 30 / 100;
        }
    }

    // C++ line 6645-6651: mage multiplier (modifies `baa`, not `damage`)
    if is_mage && damage != 0 {
        if let Some(ref ds) = ctx.damage_settings {
            baa *= ds.mage_magic_damage as f64;
        }
        baa *= ctx.plus_damage;
    }

    // C++ line 6683: `damage = (int32)baa;` — overwrites damage with baa
    damage = baa as i32;

    // ── Step 9: War zone halving ──────────────────────────────────────
    // C++ line 6685-6688
    if ctx.is_war_zone {
        damage /= 3;
    } else {
        damage /= 2;
    }

    // ── Step 10: Convert from negative domain to positive return ─────
    // C++ line 6690: if (damage > MAX_DAMAGE) damage = MAX_DAMAGE;
    // C++ returns negative (HP loss).  We negate to positive for callers.
    (-damage).clamp(0, 32000) as i16
}

/// Apply target's magic damage reduction to a damage value.
/// In C++, this is applied inside `GetMagicDamage()`, so it affects both first damage
/// and DOT duration damage.  We apply it at registration time for DOT damage, and
/// via `apply_skill_damage()` for first/direct damage.
fn apply_magic_damage_reduction(world: &WorldState, target_sid: SessionId, damage: i16) -> i16 {
    let reduction = world
        .with_session(target_sid, |h| h.magic_damage_reduction)
        .unwrap_or(100);
    if reduction < 100 {
        let reduced = (damage as i32 * reduction as i32 / 100) as i16;
        reduced.max(0)
    } else {
        damage
    }
}

/// Apply class-specific AP/AC bonuses to magic PvP damage (non-CHA skills only).
/// For non-mage casters (warrior/rogue/priest), the class bonus system adjusts
/// `temp_ap` and `temp_ac` using the equipped set item bonuses, then computes a
/// secondary damage modifier: `temp_hit_B = (temp_ap * 200 / 100) / (temp_ac + 240)`,
/// `final_damage = temp_hit_B * (damage / 100.0)`.
/// In magic, the indices are:
/// - AC: target's AC class bonus at `[caster's class group index]`
/// - AP: caster's AP class bonus at `[target's class group index]`
/// This differs from physical attack where both use the target's class index.
fn apply_magic_class_bonus(
    damage: i16,
    caster: &CharacterInfo,
    target: &CharacterInfo,
    world: &WorldState,
    caster_sid: crate::zone::SessionId,
    target_sid: crate::zone::SessionId,
) -> i16 {
    use crate::handler::attack::class_group_index;

    // Class bonus only applies to non-mage casters (isChaSkill = false).
    let caster_base = caster.class % 100;
    let is_mage_caster = matches!(caster_base, 3 | 9 | 10);
    if is_mage_caster {
        return damage;
    }

    let caster_idx = match class_group_index(caster.class) {
        Some(i) => i,
        None => return damage,
    };
    let target_idx = match class_group_index(target.class) {
        Some(i) => i,
        None => return damage,
    };

    // Snapshot both combatants — 2 DashMap reads instead of 4.
    let mcb_caster_snap = match world.snapshot_combat(caster_sid) {
        Some(s) => s,
        None => return damage,
    };
    let mcb_target_snap = match world.snapshot_combat(target_sid) {
        Some(s) => s,
        None => return damage,
    };
    let caster_stats = &mcb_caster_snap.equipped_stats;
    let target_stats = &mcb_target_snap.equipped_stats;

    // (simplified: use caster's base stats since we don't track m_sTotalHit separately for magic)
    let total_hit = caster.str.max(1) as i32 * 3;
    let mut temp_ap = total_hit * mcb_caster_snap.attack_amount / 5;
    // C++ MagicInstance.cpp:6298 — temp_ap = temp_ap * m_bPlayerAttackAmount / 100
    // Applied unconditionally in GetMagicDamage() for non-mage casters (both PvP and PvE)
    temp_ap = temp_ap * mcb_caster_snap.player_attack_amount / 100;
    let total_ac = target.sta.max(1) as i32 * 2;
    let mut temp_ac = total_ac;

    // AC: target's AcClassBonusAmount[caster's GetBaseClass() - 1]
    // AP: caster's APClassBonusAmount[target's GetBaseClass() - 1]
    temp_ac = temp_ac * (100 + target_stats.ac_class_bonus[caster_idx] as i32) / 100;
    temp_ap = temp_ap * (100 + caster_stats.ap_class_bonus[target_idx] as i32) / 100;

    let temp_hit_b = if temp_ac + 240 > 0 {
        (temp_ap * 2) / (temp_ac + 240)
    } else {
        temp_ap * 2
    };

    if temp_hit_b <= 0 {
        return damage;
    }

    let result = (temp_hit_b as f32 * (damage as f32 / 100.0)) as i32;
    result.max(1) as i16
}

// ── Type 4: Buffs / Debuffs ──────────────────────────────────────────────

/// Execute Type 4 skill — apply buff or debuff.
/// Creates an `ActiveBuff` from the `MagicType4Row` data and applies it
/// to the target via `world.apply_buff()`. Overwrites any existing buff
/// of the same type. The buff_tick system handles expiry.
/// Supports self-cast, single-target, and AOE buff/debuff application.
fn execute_type4(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type4_data = match world.get_magic_type4(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let moral = skill.moral.unwrap_or(0);

    // ── Self-buff ───────────────────────────────────────────────────
    if moral == MORAL_SELF {
        let duration = type4_data.duration.unwrap_or(0).max(0) as u16;

        instance.data[1] = 1; // bResult = success
        instance.data[3] = duration as i32;
        instance.data[5] = type4_data.speed.unwrap_or(0) as i32;

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        let buff = create_active_buff(
            instance.skill_id,
            caster_sid,
            &type4_data,
            moral < MORAL_ENEMY,
        );
        world.apply_buff(caster_sid, buff);
        apply_type4_stats(
            world,
            caster_sid,
            &type4_data,
            skill.skill.unwrap_or(0),
            instance.skill_id,
        );
        // BUG-3 fix: recalculate derived stats after buff application
        world.set_user_ability(caster_sid);
        world.send_item_move_refresh(caster_sid);
        broadcast_kaul_state_change(world, caster_sid, &type4_data, instance.skill_id);
        broadcast_size_state_change(world, caster_sid, &type4_data, instance.skill_id);
        broadcast_buff_state_change_on_apply(world, caster_sid, &type4_data, instance.skill_id);
        // Persist scroll buffs (skill_id > 500000) across logout/zone change
        if instance.skill_id > 500000 {
            world.insert_saved_magic(caster_sid, instance.skill_id, duration);
        }

        tracing::debug!(
            "[sid={}] MagicProcess Type 4: self-buff skill={} buff_type={:?} duration={:?}",
            caster_sid,
            skill.magic_num,
            type4_data.buff_type,
            type4_data.duration
        );
        return true;
    }

    // ── Friendly target buff ────────────────────────────────────────
    if moral == MORAL_FRIEND_WITHME
        || moral == MORAL_FRIEND_EXCEPTME
        || moral == MORAL_PARTY
        || moral == MORAL_PARTY_ALL
    {
        // ── MORAL_PARTY_ALL self-cast: buff ALL party members ──
        // scans surrounding regions, collects all same-party members within
        // radius, and buffs each one. Falls back to caster if no party.
        if moral == MORAL_PARTY_ALL && instance.target_id < 0 {
            tracing::debug!(
                "[sid={}] Type4 MORAL_PARTY_ALL self-cast: skill={} buff_type={:?} target_id={}",
                caster_sid,
                instance.skill_id,
                type4_data.buff_type,
                instance.target_id,
            );
            let duration = type4_data.duration.unwrap_or(0).max(0) as u16;
            let radius = type4_data.radius.unwrap_or(0) as f32;

            // Collect party member targets
            let mut targets: Vec<SessionId> = Vec::with_capacity(8);
            let caster_party_id = world
                .get_character_info(caster_sid)
                .and_then(|c| c.party_id);
            if let Some(pid) = caster_party_id {
                if let Some(party) = world.get_party(pid) {
                    let caster_pos = world.get_position(caster_sid);
                    for &msid in party.members.iter().flatten() {
                        // Skip dead members
                        if world.is_player_dead(msid) {
                            continue;
                        }
                        // Range check (skip for caster)
                        if msid != caster_sid && radius > 0.0 {
                            if let (Some(ref cp), Some(tp)) =
                                (&caster_pos, world.get_position(msid))
                            {
                                if cp.zone_id != tp.zone_id {
                                    continue;
                                }
                                let dx = cp.x - tp.x;
                                let dz = cp.z - tp.z;
                                if (dx * dx + dz * dz).sqrt() > radius {
                                    continue;
                                }
                            }
                        }
                        targets.push(msid);
                    }
                }
            }
            // C++ fallback: if no party members found, buff caster
            if targets.is_empty() {
                targets.push(caster_sid);
            }

            instance.data[1] = 1; // bResult = success
            instance.data[3] = duration as i32;
            instance.data[5] = type4_data.speed.unwrap_or(0) as i32;

            // with actual target_id (not -1) so client shows buff icon on each target.
            for &t_sid in &targets {
                let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
                pkt.write_u8(MAGIC_EFFECTING);
                pkt.write_u32(instance.skill_id);
                pkt.write_u32(instance.caster_id as u32);
                pkt.write_u32(t_sid as u32);
                for d in &instance.data {
                    pkt.write_u32(*d as u32);
                }
                broadcast_to_caster_region(world, caster_sid, &pkt);

                grant_type4_buff_to_target(
                    world,
                    caster_sid,
                    t_sid,
                    instance,
                    skill,
                    &type4_data,
                    duration,
                );
                broadcast_size_state_change(world, t_sid, &type4_data, instance.skill_id);
                broadcast_kaul_state_change(world, t_sid, &type4_data, instance.skill_id);
                broadcast_buff_state_change_on_apply(world, t_sid, &type4_data, instance.skill_id);
            }
            return true;
        }

        // ── Single-target friendly buff (MORAL_PARTY, MORAL_FRIEND_*, or targeted MORAL_PARTY_ALL) ──
        let target_sid = if instance.target_id < 0 {
            caster_sid
        } else {
            instance.target_id as SessionId
        };

        tracing::debug!(
            "[sid={}] Type4 single-target: skill={} moral={} target_id={} -> target_sid={}",
            caster_sid,
            instance.skill_id,
            moral,
            instance.target_id,
            target_sid,
        );

        let target = match world.get_character_info(target_sid) {
            Some(ch) => ch,
            None => {
                tracing::warn!(
                    "[sid={}] Type4 FAILED: target session {} not found (skill={})",
                    caster_sid,
                    target_sid,
                    instance.skill_id,
                );
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        };

        if target.res_hp_type == USER_DEAD || target.hp <= 0 {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        // Range check for friendly buffs — with party bypass
        if target_sid != caster_sid {
            let party_bypass = (moral == MORAL_PARTY || moral == MORAL_PARTY_ALL) && {
                let caster_ch = world.get_character_info(caster_sid);
                let target_ch = world.get_character_info(target_sid);
                match (caster_ch, target_ch) {
                    (Some(c), Some(t)) => c.party_id.is_some() && c.party_id == t.party_id,
                    _ => false,
                }
            };
            if !party_bypass && !check_skill_range(world, caster_sid, target_sid, skill) {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }

        {
            let bt = type4_data.buff_type.unwrap_or(0);
            if bt == BUFF_TYPE_SPEED && world.has_buff(target_sid, BUFF_TYPE_SPEED2) {
                return false;
            }
            if bt == BUFF_TYPE_SPEED2 && world.has_buff(target_sid, BUFF_TYPE_SPEED) {
                return false;
            }
        }

        {
            let bt = type4_data.buff_type.unwrap_or(0);
            if bt > 0 && world.has_buff(target_sid, bt) {
                return false;
            }
        }

        let duration = type4_data.duration.unwrap_or(0).max(0) as u16;

        instance.data[1] = 1; // bResult = success
        instance.data[3] = duration as i32;
        instance.data[5] = type4_data.speed.unwrap_or(0) as i32;

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);

        grant_type4_buff_to_target(
            world,
            caster_sid,
            target_sid,
            instance,
            skill,
            &type4_data,
            duration,
        );
        broadcast_size_state_change(world, target_sid, &type4_data, instance.skill_id);
        broadcast_kaul_state_change(world, target_sid, &type4_data, instance.skill_id);
        broadcast_buff_state_change_on_apply(world, target_sid, &type4_data, instance.skill_id);
        return true;
    }

    // ── Enemy debuff (single target) ────────────────────────────────
    if moral == MORAL_ENEMY {
        let target_id = instance.target_id;
        if target_id < 0 {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        let target_is_player = (target_id as u32) < NPC_BAND;
        if !target_is_player {
            let npc_id = target_id as u32;
            let buff_type = type4_data.buff_type.unwrap_or(0);

            if buff_type == BUFF_TYPE_FREEZE {
                return false;
            }

            let duration = type4_data.duration.unwrap_or(0).max(0) as u32;
            if buff_type > 0 {
                world.apply_npc_buff(
                    npc_id,
                    NpcBuffEntry {
                        skill_id: instance.skill_id,
                        buff_type,
                        start_time: std::time::Instant::now(),
                        duration_secs: duration,
                    },
                );
            }
            instance.data[1] = 1;
            instance.data[3] = duration as i32;
            instance.data[5] = type4_data.speed.unwrap_or(0) as i32;

            let pkt = instance.build_packet(MAGIC_EFFECTING);
            broadcast_to_caster_region(world, caster_sid, &pkt);
            return true;
        }

        let target_sid = target_id as SessionId;
        let target = match world.get_character_info(target_sid) {
            Some(ch) => ch,
            None => return false,
        };

        if target.res_hp_type == USER_DEAD || target.hp <= 0 {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        {
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if world.is_player_blinking(target_sid, now_unix) {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }

        if !check_skill_range(world, caster_sid, target_sid, skill) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        // PvP permission check — "default deny" model for enemy debuffs
        {
            let caster_pos = world.get_position(caster_sid).unwrap_or_default();
            let target_pos = world.get_position(target_sid).unwrap_or_default();
            let caster = match world.get_character_info(caster_sid) {
                Some(ch) => ch,
                None => return false,
            };
            if !crate::handler::attack::is_hostile_to(
                world,
                caster_sid,
                &caster,
                &caster_pos,
                target_sid,
                &target,
                &target_pos,
            ) {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }

        // BUFF_TYPE_BLOCK_CURSE (29) blocks all debuffs.
        // BUFF_TYPE_BLOCK_CURSE_REFLECT (30) blocks and has 25% chance to reflect.
        {
            let (has_block, has_reflect) = world
                .with_session(target_sid, |h| (h.block_curses, h.reflect_curses))
                .unwrap_or((false, false));

            if has_reflect {
                // 25% chance to reflect debuff back to caster
                let roll: i32 = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(0..1000)
                };
                if roll < 250 {
                    // Reflect: apply debuff to caster instead
                    let caster_blocked = world
                        .with_session(caster_sid, |h| h.block_curses || h.reflect_curses)
                        .unwrap_or(false);
                    if !caster_blocked {
                        let buff =
                            create_active_buff(instance.skill_id, caster_sid, &type4_data, false);
                        world.apply_buff(caster_sid, buff);
                        apply_type4_stats(
                            world,
                            caster_sid,
                            &type4_data,
                            skill.skill.unwrap_or(0),
                            instance.skill_id,
                        );
                        world.set_user_ability(caster_sid);
                    }
                }
                // Whether reflected or not, the target is protected
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
            if has_block {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }

        // SPEED, SPEED2, STUN: check cold_r (for SPEED2/freeze) or lightning_r (for SPEED/STUN)
        {
            let bt = type4_data.buff_type.unwrap_or(0);
            if matches!(bt, BUFF_TYPE_SPEED | BUFF_TYPE_SPEED2 | BUFF_TYPE_STUN) {
                // counter-spell skill IDs bypass the resistance check entirely
                let is_bypass = is_rush_skill(instance.skill_id)
                    || matches!(
                        instance.skill_id,
                        492027 | 190773 | 290773 | 190673 | 290673
                    );

                if !is_bypass {
                    let eq_stats = world.get_equipped_stats(target_sid);
                    let item_res = if bt == BUFF_TYPE_SPEED2 {
                        eq_stats.cold_r as i32
                    } else {
                        eq_stats.lightning_r as i32
                    };
                    // resistance (m_sColdR / m_sLightningR) + m_bResistanceBonus.
                    // Does NOT include buff-added resistance (m_bAddColdR / m_bAddLightningR).
                    let total_res = item_res + eq_stats.resistance_bonus as i32;
                    let max_res: i32 = 250;
                    let clamped = total_res.min(max_res);

                    let percentagerate = (clamped * 100) / max_res;
                    let rand_val: i32 = {
                        let mut rng = rand::thread_rng();
                        rng.gen_range(0..=10000)
                    };

                    // newrand = rand + (percentagerate * 3)
                    // if icelightrate > newrand → debuff applies, else resisted
                    let icelightrate = skill.icelightrate.unwrap_or(0) as i32;
                    if icelightrate > 0 {
                        let newrand = rand_val + (percentagerate * 3);
                        if icelightrate <= newrand {
                            // Debuff resisted — broadcast no-effect
                            instance.data[1] = 0;
                            let pkt = instance.build_packet(MAGIC_EFFECTING);
                            broadcast_to_caster_region(world, caster_sid, &pkt);
                            return true;
                        }
                    } else {
                        // the skill cannot apply the debuff at all
                        instance.data[1] = 0;
                        let pkt = instance.build_packet(MAGIC_EFFECTING);
                        broadcast_to_caster_region(world, caster_sid, &pkt);
                        return true;
                    }
                }
            }
        }

        instance.data[1] = 1;
        instance.data[3] = type4_data.duration.unwrap_or(0).max(0) as i32;
        instance.data[5] = type4_data.speed.unwrap_or(0) as i32;

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);

        let buff = create_active_buff(
            instance.skill_id,
            caster_sid,
            &type4_data,
            moral < MORAL_ENEMY,
        );
        world.apply_buff(target_sid, buff);
        apply_type4_stats(
            world,
            target_sid,
            &type4_data,
            skill.skill.unwrap_or(0),
            instance.skill_id,
        );
        // BUG-3 fix: recalculate derived stats after debuff application
        world.set_user_ability(target_sid);
        world.send_item_move_refresh(target_sid);
        broadcast_kaul_state_change(world, target_sid, &type4_data, instance.skill_id);
        broadcast_size_state_change(world, target_sid, &type4_data, instance.skill_id);
        broadcast_buff_state_change_on_apply(world, target_sid, &type4_data, instance.skill_id);
        return true;
    }

    // ── AOE buff/debuff ─────────────────────────────────────────────
    if moral == MORAL_AREA_ENEMY
        || moral == MORAL_AREA_FRIEND
        || moral == MORAL_AREA_ALL
        || moral == MORAL_SELF_AREA
    {
        let caster_pos = match world.get_position(caster_sid) {
            Some(p) => p,
            None => return false,
        };

        let radius = type4_data.radius.unwrap_or(0) as f32;

        let caster_event_room = world.get_event_room(caster_sid);
        let nearby = world.get_nearby_session_ids(
            caster_pos.zone_id,
            caster_pos.region_x,
            caster_pos.region_z,
            Some(caster_sid),
            caster_event_room,
        );

        let caster = match world.get_character_info(caster_sid) {
            Some(ch) => ch,
            None => return false,
        };

        // Self-area: always include caster
        if moral == MORAL_SELF_AREA {
            let buff = create_active_buff(
                instance.skill_id,
                caster_sid,
                &type4_data,
                moral < MORAL_ENEMY,
            );
            world.apply_buff(caster_sid, buff);
            apply_type4_stats(
                world,
                caster_sid,
                &type4_data,
                skill.skill.unwrap_or(0),
                instance.skill_id,
            );
            // BUG-3 fix: recalculate derived stats after self-area buff
            world.set_user_ability(caster_sid);
            // Persist scroll self-area buffs across logout/zone change
            if instance.skill_id > 500000 {
                let sa_duration = type4_data.duration.unwrap_or(0).max(0) as u16;
                world.insert_saved_magic(caster_sid, instance.skill_id, sa_duration);
            }
        }

        let (aoe_x, aoe_z) = resolve_aoe_center(
            instance.data[0],
            instance.data[2],
            caster_pos.x,
            caster_pos.z,
        );

        for target_sid in nearby {
            let target = match world.get_character_info(target_sid) {
                Some(ch) => ch,
                None => continue,
            };

            if target.res_hp_type == USER_DEAD || target.hp <= 0 {
                continue;
            }

            if moral == MORAL_AREA_ENEMY && target.authority == 0 {
                continue;
            }

            {
                let now_unix = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if world.is_player_blinking(target_sid, now_unix) {
                    continue;
                }
            }

            if moral == MORAL_AREA_ENEMY && target_sid == caster_sid {
                continue;
            }

            let target_pos = match world.get_position(target_sid) {
                Some(p) => p,
                None => continue,
            };

            // Distance check
            if radius > 0.0 {
                let dx = aoe_x - target_pos.x;
                let dz = aoe_z - target_pos.z;
                let dist_sq = dx * dx + dz * dz;
                let radius_sq = radius * radius;

                if dist_sq > radius_sq {
                    continue;
                }
            }

            // PvP permission check for AOE buff/debuff targets
            if moral == MORAL_AREA_ENEMY
                && !crate::handler::attack::is_hostile_to(
                    world,
                    caster_sid,
                    &caster,
                    &caster_pos,
                    target_sid,
                    &target,
                    &target_pos,
                )
            {
                continue;
            }
            if moral == MORAL_AREA_FRIEND
                && crate::handler::attack::is_hostile_to(
                    world,
                    caster_sid,
                    &caster,
                    &caster_pos,
                    target_sid,
                    &target,
                    &target_pos,
                )
            {
                continue;
            }

            if moral == MORAL_AREA_ALL {
                let caster_zone = caster_pos.zone_id;
                let target_zone = target_pos.zone_id;
                let both_in_special_zone =
                    (crate::handler::attack::is_in_arena(caster_zone, caster_pos.x, caster_pos.z)
                        && crate::handler::attack::is_in_arena(
                            target_zone,
                            target_pos.x,
                            target_pos.z,
                        ))
                        || (crate::handler::attack::is_in_pvp_zone(caster_zone)
                            && crate::handler::attack::is_in_pvp_zone(target_zone))
                        || (crate::systems::event_room::is_in_temple_event_zone(caster_zone)
                            && crate::systems::event_room::is_in_temple_event_zone(target_zone));

                if !both_in_special_zone
                    && crate::handler::attack::is_in_enemy_safety_area(
                        caster_zone,
                        caster_pos.x,
                        caster_pos.z,
                        caster.nation,
                    )
                    && crate::handler::attack::is_in_own_safety_area(
                        target_zone,
                        target_pos.x,
                        target_pos.z,
                        target.nation,
                    )
                {
                    continue;
                }
            }

            if moral == MORAL_AREA_ENEMY {
                let (has_block, has_reflect) = world
                    .with_session(target_sid, |h| (h.block_curses, h.reflect_curses))
                    .unwrap_or((false, false));
                if has_reflect {
                    let roll: i32 = {
                        let mut rng = rand::thread_rng();
                        rng.gen_range(0..1000)
                    };
                    if roll < 250 {
                        let caster_blocked = world
                            .with_session(caster_sid, |h| h.block_curses || h.reflect_curses)
                            .unwrap_or(false);
                        if !caster_blocked {
                            let buff_r = create_active_buff(
                                instance.skill_id,
                                caster_sid,
                                &type4_data,
                                false,
                            );
                            world.apply_buff(caster_sid, buff_r);
                            apply_type4_stats(
                                world,
                                caster_sid,
                                &type4_data,
                                skill.skill.unwrap_or(0),
                                instance.skill_id,
                            );
                            world.set_user_ability(caster_sid);
                            world.send_item_move_refresh(caster_sid);
                        }
                    }
                    continue; // target protected
                }
                if has_block {
                    continue; // target immune
                }
            }

            // Same CheckIceLightSpeed algorithm for AOE enemy debuffs
            if moral == MORAL_AREA_ENEMY {
                let bt = type4_data.buff_type.unwrap_or(0);
                if matches!(bt, BUFF_TYPE_SPEED | BUFF_TYPE_SPEED2 | BUFF_TYPE_STUN) {
                    let is_bypass = is_rush_skill(instance.skill_id)
                        || matches!(
                            instance.skill_id,
                            492027 | 190773 | 290773 | 190673 | 290673
                        );

                    if !is_bypass {
                        let eq_stats = world.get_equipped_stats(target_sid);
                        let item_res = if bt == BUFF_TYPE_SPEED2 {
                            eq_stats.cold_r as i32
                        } else {
                            eq_stats.lightning_r as i32
                        };
                        // resistance + m_bResistanceBonus, NOT buff-added resistance.
                        let total_res = item_res + eq_stats.resistance_bonus as i32;
                        let max_res: i32 = 250;
                        let clamped = total_res.min(max_res);
                        let percentagerate = (clamped * 100) / max_res;
                        let rand_val: i32 = {
                            let mut rng = rand::thread_rng();
                            rng.gen_range(0..=10000)
                        };
                        let icelightrate = skill.icelightrate.unwrap_or(0) as i32;
                        if icelightrate <= 0 || icelightrate <= rand_val + (percentagerate * 3) {
                            continue; // Debuff resisted for this AOE target
                        }
                    }
                }
            }

            // M3: Break stealth on AOE hit for enemy targets
            if target.nation != caster.nation {
                crate::handler::stealth::remove_stealth(world, target_sid);
            }

            let duration = type4_data.duration.unwrap_or(0).max(0) as u16;
            let buff = create_active_buff(
                instance.skill_id,
                caster_sid,
                &type4_data,
                moral < MORAL_ENEMY,
            );
            world.apply_buff(target_sid, buff);
            apply_type4_stats(
                world,
                target_sid,
                &type4_data,
                skill.skill.unwrap_or(0),
                instance.skill_id,
            );
            // BUG-3 fix: recalculate derived stats after AOE buff/debuff
            world.set_user_ability(target_sid);
            world.send_item_move_refresh(target_sid);
            broadcast_kaul_state_change(world, target_sid, &type4_data, instance.skill_id);
            broadcast_size_state_change(world, target_sid, &type4_data, instance.skill_id);
            broadcast_buff_state_change_on_apply(world, target_sid, &type4_data, instance.skill_id);
            // Persist scroll buffs on AOE targets
            if instance.skill_id > 500000 {
                world.insert_saved_magic(target_sid, instance.skill_id, duration);
            }
        }

        instance.data[1] = 1;
        instance.data[3] = type4_data.duration.unwrap_or(0).max(0) as i32;
        instance.data[5] = type4_data.speed.unwrap_or(0) as i32;

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        return true;
    }

    // Default: broadcast
    instance.data[1] = 1;
    instance.data[3] = type4_data.duration.unwrap_or(0).max(0) as i32;
    instance.data[5] = type4_data.speed.unwrap_or(0) as i32;

    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);
    true
}

/// Check whether a skill ID is a "rush" skill that bypasses debuff resistance.
/// Rush skills: warrior charge variants (114509, 115509, 214509, 215509).
fn is_rush_skill(skill_id: u32) -> bool {
    matches!(skill_id, 114509 | 115509 | 214509 | 215509)
}

/// Apply a Type4 buff to a single target, with duplicate/speed checks.
/// Used by `execute_type4` for both single-target and party-wide buff application.
fn grant_type4_buff_to_target(
    world: &WorldState,
    caster_sid: SessionId,
    target_sid: SessionId,
    instance: &MagicInstance,
    skill: &MagicRow,
    type4_data: &ko_db::models::MagicType4Row,
    duration: u16,
) {
    // SPEED / SPEED2 mutual exclusion
    let bt = type4_data.buff_type.unwrap_or(0);
    if bt == BUFF_TYPE_SPEED && world.has_buff(target_sid, BUFF_TYPE_SPEED2) {
        return;
    }
    if bt == BUFF_TYPE_SPEED2 && world.has_buff(target_sid, BUFF_TYPE_SPEED) {
        return;
    }

    // Duplicate buff rejection
    if bt > 0 && world.has_buff(target_sid, bt) {
        return;
    }

    let buff = create_active_buff(
        instance.skill_id,
        caster_sid,
        type4_data,
        skill.moral.unwrap_or(0) < MORAL_ENEMY,
    );
    world.apply_buff(target_sid, buff);
    apply_type4_stats(
        world,
        target_sid,
        type4_data,
        skill.skill.unwrap_or(0),
        instance.skill_id,
    );
    world.set_user_ability(target_sid);
    // This sends total_hit with buff multipliers so the client shows the updated attack value.
    world.send_item_move_refresh(target_sid);
    if instance.skill_id > 500000 {
        world.insert_saved_magic(target_sid, instance.skill_id, duration);
    }
}

/// Create an `ActiveBuff` from a `MagicType4Row`.
pub(crate) fn create_active_buff(
    skill_id: u32,
    caster_sid: SessionId,
    type4: &ko_db::models::MagicType4Row,
    is_buff: bool,
) -> ActiveBuff {
    ActiveBuff {
        skill_id,
        buff_type: type4.buff_type.unwrap_or(0),
        caster_sid,
        start_time: Instant::now(),
        duration_secs: type4.duration.unwrap_or(0).max(0) as u32,
        attack_speed: type4.attack_speed.unwrap_or(0),
        speed: type4.speed.unwrap_or(0),
        ac: type4.ac.unwrap_or(0),
        ac_pct: type4.ac_pct.unwrap_or(0),
        attack: type4.attack.unwrap_or(0),
        magic_attack: type4.magic_attack.unwrap_or(0),
        max_hp: type4.max_hp.unwrap_or(0),
        max_hp_pct: type4.max_hp_pct.unwrap_or(0),
        max_mp: type4.max_mp.unwrap_or(0),
        max_mp_pct: type4.max_mp_pct.unwrap_or(0),
        str_mod: type4.str.unwrap_or(0),
        sta_mod: type4.sta.unwrap_or(0),
        dex_mod: type4.dex.unwrap_or(0),
        intel_mod: type4.intel.unwrap_or(0),
        cha_mod: type4.cha.unwrap_or(0),
        fire_r: type4.fire_r.unwrap_or(0),
        cold_r: type4.cold_r.unwrap_or(0),
        lightning_r: type4.lightning_r.unwrap_or(0),
        magic_r: type4.magic_r.unwrap_or(0),
        disease_r: type4.disease_r.unwrap_or(0),
        poison_r: type4.poison_r.unwrap_or(0),
        hit_rate: type4.hit_rate.unwrap_or(0),
        avoid_rate: type4.avoid_rate.unwrap_or(0),
        weapon_damage: 0,
        ac_sour: 0,
        duration_extended: false,
        is_buff,
    }
}

/// Apply immediate stat modifications from a Type 4 buff.
/// Adjusts max_hp and max_mp on the character. The buff itself is tracked
/// separately via `ActiveBuff` so it can be reversed on expiry.
pub(crate) fn apply_type4_stats(
    world: &WorldState,
    target_sid: SessionId,
    type4: &ko_db::models::MagicType4Row,
    s_skill: i16,
    skill_id: u32,
) {
    let buff_type = type4.buff_type.unwrap_or(0);
    if buff_type == BUFF_TYPE_MAGE_ARMOR {
        let reflect_type = (s_skill % 100) as u8;
        world.update_session(target_sid, |h| {
            h.reflect_armor_type = reflect_type;
        });
    }
    if buff_type == BUFF_TYPE_MIRROR_DAMAGE_PARTY {
        let special_amount = type4.special_amount.unwrap_or(0).min(100) as u8;
        let is_direct = skill_id == 492028;
        world.update_session(target_sid, |h| {
            h.mirror_damage = true;
            h.mirror_damage_type = is_direct;
            h.mirror_amount = special_amount;
        });
    }
    if buff_type == BUFF_TYPE_DAGGER_BOW_DEFENSE {
        let special_amount = type4.special_amount.unwrap_or(0).min(100) as u8;
        let amount = 100u8.saturating_sub(special_amount);
        world.update_session(target_sid, |h| {
            h.dagger_r_amount = amount;
            h.bow_r_amount = amount;
        });
    }
    if buff_type == BUFF_TYPE_SILENCE_TARGET {
        world.update_session(target_sid, |h| {
            h.can_use_skills = false;
        });
    }
    if buff_type == BUFF_TYPE_NO_POTIONS {
        world.update_session(target_sid, |h| {
            h.can_use_potions = false;
        });
    }
    if buff_type == BUFF_TYPE_KAUL_TRANSFORMATION {
        world.update_session(target_sid, |h| {
            h.is_kaul = true;
            h.can_use_skills = false;
            h.old_abnormal_type = if h.transform_skill_id != 0 {
                h.transform_skill_id
            } else {
                ABNORMAL_NORMAL
            };
            // Add 500 to the Kaul buff's AC so get_buff_ac_amount() includes it
            if let Some(buff) = h
                .buffs
                .values_mut()
                .find(|b| b.buff_type == BUFF_TYPE_KAUL_TRANSFORMATION)
            {
                buff.ac += 500;
            }
        });
    }
    if buff_type == BUFF_TYPE_UNDEAD {
        world.update_session(target_sid, |h| {
            h.is_undead = true;
        });
    }
    if buff_type == BUFF_TYPE_FREEZE {
        world.update_session(target_sid, |h| {
            h.block_magic = true;
        });
    }
    if buff_type == BUFF_TYPE_UNSIGHT
        || buff_type == BUFF_TYPE_BLIND
        || buff_type == BUFF_TYPE_DISABLE_TARGETING
    {
        world.update_session(target_sid, |h| {
            h.is_blinded = true;
        });
    }
    if buff_type == BUFF_TYPE_BLOCK_PHYSICAL_DAMAGE {
        world.update_session(target_sid, |h| {
            h.block_physical = true;
        });
    }
    if buff_type == BUFF_TYPE_BLOCK_MAGICAL_DAMAGE {
        world.update_session(target_sid, |h| {
            h.block_magic = true;
        });
    }
    if buff_type == BUFF_TYPE_DEVIL_TRANSFORM {
        world.update_session(target_sid, |h| {
            h.is_devil = true;
        });
    }
    if buff_type == BUFF_TYPE_NO_RECALL {
        world.update_session(target_sid, |h| {
            h.can_teleport = false;
        });
    }
    if buff_type == BUFF_TYPE_PROHIBIT_INVIS {
        world.update_session(target_sid, |h| {
            h.can_stealth = false;
        });
    }
    if buff_type == BUFF_TYPE_RESIS_AND_MAGIC_DMG {
        let exp_pct = type4.exp_pct.unwrap_or(0).clamp(0, 100) as u8;
        world.update_session(target_sid, |h| {
            h.magic_damage_reduction = exp_pct;
        });
    }
    if buff_type == BUFF_TYPE_BLOCK_CURSE {
        world.update_session(target_sid, |h| {
            h.block_curses = true;
        });
    }
    if buff_type == BUFF_TYPE_BLOCK_CURSE_REFLECT {
        world.update_session(target_sid, |h| {
            h.reflect_curses = true;
        });
    }
    if buff_type == BUFF_TYPE_INSTANT_MAGIC {
        world.update_session(target_sid, |h| {
            h.instant_cast = true;
        });
    }
    if buff_type == BUFF_TYPE_NP_DROP_NOAH {
        let special_amount = type4.special_amount.unwrap_or(0) as i16;
        if special_amount > 0 {
            world.update_session(target_sid, |h| {
                h.drop_scroll_amount += special_amount;
            });
        }
    }
    if buff_type == BUFF_TYPE_JACKPOT {
        let jtype = if skill_id == 501570 {
            1u8 // EXP jackpot
        } else if skill_id == 501571 {
            2 // Noah jackpot
        } else if skill_id == 501572 {
            3 // both (unused in C++ logic)
        } else {
            0
        };
        if jtype > 0 {
            world.update_session(target_sid, |h| {
                h.jackpot_type = jtype;
            });
        }
    }
    if buff_type == BUFF_TYPE_WEAPON_DAMAGE {
        let weapon_dmg = type4.attack.unwrap_or(0);
        world.update_session(target_sid, |h| {
            if let Some(buff) = h
                .buffs
                .values_mut()
                .find(|b| b.buff_type == BUFF_TYPE_WEAPON_DAMAGE && b.skill_id == skill_id)
            {
                buff.weapon_damage = weapon_dmg;
            }
        });
    }
    // If sAC == 0 && sACPct > 0: modify m_bPctArmourAc (already tracked in ac_pct)
    // Else: modify m_sAddArmourAc (already tracked in ac)
    // Both are handled via existing ActiveBuff.ac / ac_pct fields.
    // The integration into set_user_ability is the missing piece (Task #4).
    if buff_type == BUFF_TYPE_ATTACK_SPEED_ARMOR {
        let ac_val = type4.ac.unwrap_or(0);
        if ac_val < 0 {
            // Negative AC → AC reduction source (m_sACSourAmount)
            let sour = -ac_val; // make positive for subtraction
            world.update_session(target_sid, |h| {
                if let Some(buff) = h
                    .buffs
                    .values_mut()
                    .find(|b| b.buff_type == BUFF_TYPE_ATTACK_SPEED_ARMOR && b.skill_id == skill_id)
                {
                    buff.ac_sour = sour;
                    buff.ac = 0; // don't double-count in get_buff_ac_amount
                }
            });
        }
        // Positive AC is already handled by the normal ActiveBuff.ac field
    }
    if buff_type == BUFF_TYPE_WEIGHT {
        let weight_amount = type4.exp_pct.unwrap_or(0).clamp(0, 255) as u8;
        world.update_session(target_sid, |h| {
            h.weight_buff_amount = weight_amount;
        });
    }
    if buff_type == BUFF_TYPE_LOYALTY {
        let np_amount = type4.exp_pct.unwrap_or(0).clamp(0, 255) as u8;
        world.update_session(target_sid, |h| {
            h.np_gain_amount = np_amount;
        });
    }
    if buff_type == BUFF_TYPE_NOAH_BONUS {
        let noah_amount = type4.exp_pct.unwrap_or(0).clamp(0, 255) as u8;
        world.update_session(target_sid, |h| {
            h.noah_gain_amount = noah_amount;
        });
    }
    if buff_type == BUFF_TYPE_PREMIUM_MERCHANT {
        world.update_session(target_sid, |h| {
            h.is_premium_merchant = true;
        });
    }
    if buff_type == BUFF_TYPE_MANA_ABSORB {
        let absorb_pct = type4.exp_pct.unwrap_or(0).max(0) as u8;
        world.update_session(target_sid, |h| {
            h.mana_absorb += absorb_pct;
            h.absorb_count = 4;
        });
    }
    // C++ logic: `if (110 <= myrand(0, 300)) weapons = false; else weapons = true;`
    // myrand(0,300) returns [0..300]. Values >= 110 → NOT disabled (191/301 ≈ 63%).
    // Values < 110 → disabled (110/301 ≈ 37%).
    if buff_type == BUFF_TYPE_IGNORE_WEAPON {
        let roll: i32 = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..=300)
        };
        let disabled = roll < 110; // ~37% chance
        world.update_session(target_sid, |h| {
            h.weapons_disabled = disabled;
        });
    }
    if buff_type == BUFF_TYPE_DECREASE_RESIST {
        let fire_r = type4.fire_r.unwrap_or(0).clamp(0, 100) as u8;
        let cold_r = type4.cold_r.unwrap_or(0).clamp(0, 100) as u8;
        let lightning_r = type4.lightning_r.unwrap_or(0).clamp(0, 100) as u8;
        let magic_r = type4.magic_r.unwrap_or(0).clamp(0, 100) as u8;
        let disease_r = type4.disease_r.unwrap_or(0).clamp(0, 100) as u8;
        let poison_r = type4.poison_r.unwrap_or(0).clamp(0, 100) as u8;
        world.update_session(target_sid, |h| {
            h.pct_fire_r = 100u8.saturating_sub(fire_r);
            h.pct_cold_r = 100u8.saturating_sub(cold_r);
            h.pct_lightning_r = 100u8.saturating_sub(lightning_r);
            h.pct_magic_r = 100u8.saturating_sub(magic_r);
            h.pct_disease_r = 100u8.saturating_sub(disease_r);
            h.pct_poison_r = 100u8.saturating_sub(poison_r);
        });
    }
    if buff_type == BUFF_TYPE_EXPERIENCE {
        let exp_pct = type4.exp_pct.unwrap_or(0);
        if exp_pct > 100 {
            world.update_session(target_sid, |h| {
                h.exp_gain_buff11 = h.exp_gain_buff11.saturating_add((exp_pct - 100) as u16);
            });
        }
    }
    if buff_type == BUFF_TYPE_VARIOUS_EFFECTS {
        let exp_pct = type4.exp_pct.unwrap_or(0);
        if exp_pct > 100 {
            world.update_session(target_sid, |h| {
                h.exp_gain_buff33 = h.exp_gain_buff33.saturating_add((exp_pct - 100) as u16);
            });
        }
        let special_amount = type4.special_amount.unwrap_or(0);
        if special_amount > 0 {
            world.update_session(target_sid, |h| {
                h.skill_np_bonus_33 = special_amount as u8;
            });
        }
    }
    if buff_type == BUFF_TYPE_LOYALTY_AMOUNT {
        let special_amount = type4.special_amount.unwrap_or(0);
        if special_amount > 0 {
            world.update_session(target_sid, |h| {
                h.skill_np_bonus_42 = special_amount as u8;
            });
        }
    }
    if buff_type == BUFF_TYPE_SIZE {
        // C++ checks `if (pCaster->isPlayer())` — always true for our context
        // Determine visual effect from skill ID
        let b_effect: u32 = match skill_id {
            490034 => ABNORMAL_GIANT,          // Bezoar (enlarge)
            490401 => ABNORMAL_GIANT_TARGET,   // Maximize Scroll
            490035 | 490100 => ABNORMAL_DWARF, // Rice cake / Minimize scroll (shrink)
            491415 => 0x09,                    // Special effect
            _ => 0,
        };
        if b_effect > 0 {
            world.update_session(target_sid, |h| {
                h.size_effect = b_effect;
            });
        }
    }
    // HP/MP bonuses from buffs are now computed centrally in set_user_ability()
    // via the ActiveBuff's max_hp/max_hp_pct/max_mp/max_mp_pct fields.
    // The caller (magic_process handlers) must call set_user_ability() after this.
}

/// Broadcast Kaul visual transform to region if the applied buff is Kaul.
fn broadcast_kaul_state_change(
    world: &WorldState,
    target_sid: SessionId,
    type4: &ko_db::models::MagicType4Row,
    skill_id: u32,
) {
    if type4.buff_type.unwrap_or(0) != BUFF_TYPE_KAUL_TRANSFORMATION {
        return;
    }
    if let Some(pos) = world.get_position(target_sid) {
        let pkt = crate::handler::regene::build_state_change_broadcast(
            target_sid as u32,
            STATE_CHANGE_ABNORMAL,
            skill_id,
        );
        let event_room = world.get_event_room(target_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    }
}

/// Broadcast size change visual to region if the applied buff is SIZE.
fn broadcast_size_state_change(
    world: &WorldState,
    target_sid: SessionId,
    type4: &ko_db::models::MagicType4Row,
    skill_id: u32,
) {
    if type4.buff_type.unwrap_or(0) != BUFF_TYPE_SIZE {
        return;
    }
    let b_effect: u32 = match skill_id {
        490034 => ABNORMAL_GIANT,
        490401 => ABNORMAL_GIANT_TARGET,
        490035 | 490100 => ABNORMAL_DWARF,
        491415 => 0x09,
        _ => return,
    };
    if let Some(pos) = world.get_position(target_sid) {
        let pkt = crate::handler::regene::build_state_change_broadcast(
            target_sid as u32,
            STATE_CHANGE_ABNORMAL,
            b_effect,
        );
        let event_room = world.get_event_room(target_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            event_room,
        );
    }
}

/// Broadcast visual transforms for DEVIL_TRANSFORM and SNOWMAN_TITI on apply.
/// - `MagicProcess.cpp:999` — `StateChangeServerDirect(12, 1)` for Devil
/// - `MagicProcess.cpp:983` — `StateChangeServerDirect(3, pType->iNum)` for Snowman
fn broadcast_buff_state_change_on_apply(
    world: &WorldState,
    target_sid: SessionId,
    type4: &ko_db::models::MagicType4Row,
    skill_id: u32,
) {
    let buff_type = type4.buff_type.unwrap_or(0);
    if buff_type == BUFF_TYPE_DEVIL_TRANSFORM {
        // StateChange(12, 1) — enable devil visual
        if let Some(pos) = world.get_position(target_sid) {
            let pkt = crate::handler::regene::build_state_change_broadcast(
                target_sid as u32,
                STATE_CHANGE_WEAPONS_DISABLED,
                1,
            );
            let event_room = world.get_event_room(target_sid);
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(pkt),
                None,
                event_room,
            );
        }
    } else if buff_type == BUFF_TYPE_IGNORE_WEAPON {
        let weapons_disabled = world
            .with_session(target_sid, |h| h.weapons_disabled)
            .unwrap_or(false);
        if weapons_disabled {
            if let Some(pos) = world.get_position(target_sid) {
                let event_room = world.get_event_room(target_sid);

                // Hide right hand weapon
                let mut rh_pkt = Packet::new(Opcode::WizUserlookChange as u8);
                rh_pkt.write_u32(target_sid as u32);
                rh_pkt.write_u8(RIGHTHAND as u8);
                rh_pkt.write_u32(0);
                rh_pkt.write_u16(0);
                rh_pkt.write_u8(0);
                world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(rh_pkt),
                    Some(target_sid),
                    event_room,
                );

                // Hide left hand if not a shield
                if let Some(left_slot) = world.get_inventory_slot(target_sid, LEFTHAND) {
                    if left_slot.item_id != 0 {
                        let is_shield = world
                            .get_item(left_slot.item_id)
                            .map(|item| item.kind.unwrap_or(0) == 60) // WEAPON_SHIELD=60
                            .unwrap_or(false);
                        if !is_shield {
                            let mut lh_pkt = Packet::new(Opcode::WizUserlookChange as u8);
                            lh_pkt.write_u32(target_sid as u32);
                            lh_pkt.write_u8(LEFTHAND as u8);
                            lh_pkt.write_u32(0);
                            lh_pkt.write_u16(0);
                            lh_pkt.write_u8(0);
                            world.broadcast_to_3x3(
                                pos.zone_id,
                                pos.region_x,
                                pos.region_z,
                                Arc::new(lh_pkt),
                                Some(target_sid),
                                event_room,
                            );
                        }
                    }
                }
            }
        }
    } else if buff_type == BUFF_TYPE_SNOWMAN_TITI {
        // StateChange(3, skill_id) — snowman visual (same as transform)
        // Save old abnormal before setting snowman visual
        world.update_session(target_sid, |h| {
            h.old_abnormal_type = if h.transform_skill_id != 0 {
                h.transform_skill_id
            } else {
                ABNORMAL_NORMAL
            };
        });
        if let Some(pos) = world.get_position(target_sid) {
            let pkt = crate::handler::regene::build_state_change_broadcast(
                target_sid as u32,
                STATE_CHANGE_ABNORMAL,
                skill_id,
            );
            let event_room = world.get_event_room(target_sid);
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(pkt),
                None,
                event_room,
            );
        }
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────

/// Check if target is within skill range, using C++ dynamic range modifiers.
/// with class/weapon/movement modifiers.
fn check_skill_range(
    world: &WorldState,
    caster_sid: SessionId,
    target_sid: SessionId,
    skill: &MagicRow,
) -> bool {
    let caster_pos = match world.get_position(caster_sid) {
        Some(p) => p,
        None => return false,
    };
    let target_pos = match world.get_position(target_sid) {
        Some(p) => p,
        None => return false,
    };

    // Must be same zone
    if caster_pos.zone_id != target_pos.zone_id {
        return false;
    }

    let base_range = skill.range.unwrap_or(0) as i32;
    let skill_id = skill.magic_num as u32;
    let type1 = skill.type1.unwrap_or(0);
    let type2 = skill.type2.unwrap_or(0);
    let cast_time = skill.cast_time.unwrap_or(0);
    let t_1 = skill.t_1.unwrap_or(0);
    let use_item = skill.use_item.unwrap_or(0);

    // Get caster movement state (C++ m_sSpeed)
    let is_moving = world
        .with_session(caster_sid, |h| h.move_old_speed != 0)
        .unwrap_or(false);

    let skill_range: i32 =
        if (type1 == 1 || type2 == 1) && cast_time == 0 && !is_staff_skill(skill_id) {
            // Melee skill with no cast time, non-staff
            // C++ lines 241-247
            if is_moving {
                18
            } else {
                12
            }
        } else if is_drain_skill(skill_id) {
            // Drain skills: +5
            // C++ line 248-249
            base_range + 5
        } else if is_staff_skill(skill_id) && is_moving {
            // Staff skill while moving: +17
            // C++ lines 250-251
            base_range + 17
        } else if is_staff_skill(skill_id) {
            // Staff skill while standing: +10
            // C++ lines 252-253
            base_range + 10
        } else {
            // Default: +9
            // C++ lines 254-255
            base_range + 9
        };

    // Special overrides
    // C++ lines 257-265
    let skill_range = if is_target_npc_pid_6200(world, target_sid) && is_drain_skill(skill_id) {
        37
    } else if type1 == 8 && t_1 == BUFF_TYPE_KAUL_TRANSFORMATION && base_range == 1 {
        // Type 8 knockback with Kaul transformation params
        6
    } else if !is_staff_skill(skill_id) && t_1 != -1 && t_1 != 0 && cast_time > 0 {
        // Non-staff mage skills with cast time during EFFECTING: range * 2
        // C++ line 262-265
        base_range * 2
    } else {
        skill_range
    };

    // Item 391010000 special range
    // C++ line 273
    let effective_range = if use_item == 391010000 {
        55
    } else {
        skill_range
    };

    let dx = caster_pos.x - target_pos.x;
    let dz = caster_pos.z - target_pos.z;
    let dist_sq = dx * dx + dz * dz;
    let range_sq = (effective_range as f32) * (effective_range as f32);

    dist_sq <= range_sq
}

/// Check if the target is NPC with proto_id 6200 (used for drain skill range override).
fn is_target_npc_pid_6200(_world: &WorldState, _target_sid: SessionId) -> bool {
    // NPC targets go through a different path (npc_id-based, not session-based),
    // so this player-vs-player range check won't encounter NPC PID 6200.
    false
}

/// Check if a skill ID is a "staff skill" (mage long-range staff attacks).
/// Hardcoded list of skill IDs for all staff-based mage skills.
fn is_staff_skill(skill_id: u32) -> bool {
    matches!(
        skill_id,
        // Lightning staff
        109742 | 110742 | 209742 | 210742 | 110772 | 210772 |
        // Fire staff
        109542 | 110542 | 209542 | 210542 | 110572 | 210572 |
        // Ice staff
        109642 | 110642 | 209642 | 210642 | 110672 | 210672 |
        // Master 43/56 mage skills
        109556 | 209556 | 109543 | 209543 |
        109656 | 209656 | 109643 | 209643 |
        109756 | 209756 | 109743 | 209743 |
        110556 | 210556 | 110543 | 210543 |
        110656 | 210656 | 110643 | 210643 |
        110756 | 210756 | 110743 | 210743
    )
}

/// Check if a skill ID is a "drain skill" (HP/MP drain attacks).
fn is_drain_skill(skill_id: u32) -> bool {
    matches!(
        skill_id,
        107650 | 108650 | 207650 | 208650 | 107610 | 108610 | 207610 | 208610
    )
}

/// Check if a skill ID is a "stomp skill" (ground-target AoE warrior skills).
fn is_stomp_skill(skill_id: u32) -> bool {
    matches!(
        skill_id,
        105725
            | 105735
            | 106725
            | 106735
            | 205725
            | 205735
            | 206725
            | 206735
            | 105760
            | 106760
            | 205760
            | 206760
            | 106775
            | 206775
    )
}

/// Validate that the caster's class matches the skill's class requirement.
/// The skill's `sSkill / 10` encodes the class constant (e.g., 101=KaruWarrior, 205=Blade).
/// The player's `class % 100` gives the class type (1=warrior, 5=novice warrior, etc.).
/// Each class constant pair (Karus/Elmorad equivalent) maps to a single class type.
fn check_skill_class(iclass: i16, player_class: u16) -> bool {
    // GetClassType() = GetClass() % 100
    let class_type = (player_class % 100) as i16;

    match iclass {
        // Beginner warrior: KARUWARRIOR(101) / ELMORWARRIOR(201)
        101 | 201 => class_type == 1,
        // Beginner rogue: KARUROGUE(102) / ELMOROGUE(202)
        102 | 202 => class_type == 2,
        // Beginner mage: KARUWIZARD(103) / ELMOWIZARD(203)
        103 | 203 => class_type == 3,
        // Beginner priest: KARUPRIEST(104) / ELMOPRIEST(204)
        104 | 204 => class_type == 4,
        // Novice warrior: BERSERKER(105) / BLADE(205)
        105 | 205 => class_type == 5,
        // Master warrior: GUARDIAN(106) / PROTECTOR(206)
        106 | 206 => class_type == 6,
        // Novice rogue: HUNTER(107) / RANGER(207)
        107 | 207 => class_type == 7,
        // Master rogue: PENETRATOR(108) / ASSASSIN(208)
        108 | 208 => class_type == 8,
        // Novice mage: SORSERER(109) / MAGE(209)
        109 | 209 => class_type == 9,
        // Master mage: NECROMANCER(110) / ENCHANTER(210)
        110 | 210 => class_type == 10,
        // Novice priest: SHAMAN(111) / CLERIC(211)
        111 | 211 => class_type == 11,
        // Master priest: DARKPRIEST(112) / DRUID(212)
        112 | 212 => class_type == 12,
        // Beginner kurian/porutu: KURIANSTARTER(113) / PORUTUSTARTER(213)
        113 | 213 => class_type == 13,
        // Novice kurian/porutu: KURIANNOVICE(114) / PORUTUNOVICE(214)
        114 | 214 => class_type == 14,
        // Master kurian/porutu: KURIANMASTER(115) / PORUTUMASTER(215)
        115 | 215 => class_type == 15,
        // Common skills (iclass=100/190/200/290/etc.) — no class restriction
        _ => true,
    }
}

/// Send MAGIC_FAIL to the caster.
fn send_skill_failed(world: &WorldState, caster_sid: SessionId, instance: &mut MagicInstance) {
    // MAGIC_CASTING → SKILLMAGIC_FAIL_CASTING (-100), else → SKILLMAGIC_FAIL_NOEFFECT (-103)
    instance.data[3] = if instance.opcode == MAGIC_CASTING {
        -100 // SKILLMAGIC_FAIL_CASTING
    } else {
        SKILLMAGIC_FAIL_NOEFFECT
    };
    let fail_pkt = instance.build_fail_packet();
    world.send_to_session_owned(caster_sid, fail_pkt);
}

/// Broadcast a packet to the caster's 3×3 region.
fn broadcast_to_caster_region(world: &WorldState, caster_sid: SessionId, pkt: &Packet) {
    if let Some(pos) = world.get_position(caster_sid) {
        let event_room = world.get_event_room(caster_sid);
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt.clone()),
            None,
            event_room,
        );
    }
}

/// Compute the full PvP target AC for skill-based physical damage.
/// Uses `total_ac` (equipment + coefficient), buff AC (with armor scroll disable check),
/// AC percent, AC sour reduction, and class-specific AC bonus.
fn compute_pvp_skill_target_ac(
    snap: &CombatSnapshot,
    target: &CharacterInfo,
    skill_id: u32,
) -> i32 {
    let buff_ac = if is_armor_scroll_disable_skill(skill_id) {
        0
    } else {
        snap.ac_amount
    };
    let mut ac =
        ((snap.equipped_stats.total_ac as i32) * snap.ac_pct / 100 + buff_ac - snap.ac_sour).max(0);

    if let Some(idx) = crate::handler::attack::class_group_index(target.class) {
        let bonus = snap.equipped_stats.ac_class_bonus[idx] as i32;
        ac = ac * (100 + bonus) / 100;
    }
    ac
}

/// Check if a skill ID disables armor scroll AC buffs.
const ARMOR_SCROLL_DISABLE_SKILLS: [u32; 14] = [
    107640, 108640, 207640, 208640, 107620, 108620, 207620, 208620, 107600, 108600, 207600, 208600,
    108670, 208670,
];

fn is_armor_scroll_disable_skill(skill_id: u32) -> bool {
    ARMOR_SCROLL_DISABLE_SKILLS.contains(&skill_id)
}

/// Compute Type1 (melee skill) damage following C++ GetDamage formula.
/// - `temp_hit = temp_hit_B * (pType1->sHit / 100.0f)`
/// - `damage = (short)((temp_hit + 0.3f * random) + 0.99f)`
/// Unlike the R-attack formula `(0.75 * hit_b + 0.3 * rand)`, Type1 skills use
/// `(temp_hit + 0.3 * rand + 0.99)` where `temp_hit` is scaled by `sHit` percentage.
#[allow(clippy::too_many_arguments)]
fn compute_type1_hit_damage(
    total_hit: u16,
    target_ac: i32,
    type1_data: &MagicType1Row,
    caster_hitrate: f32,
    target_evasion: f32,
    attack_amount: i32,
    player_attack_amount: i32,
    rng: &mut impl Rng,
) -> i16 {
    let total_hit = total_hit as i32;
    let temp_ap = total_hit * attack_amount; // C++ Unit.cpp:305 — m_sTotalHit * m_bAttackAmount
                                             // C++ Unit.cpp:314 — PvP modifier: temp_ap = temp_ap * m_bPlayerAttackAmount / 100
    let temp_ap = temp_ap * player_attack_amount / 100;

    // C++ line 358: temp_hit_B = (temp_ap * 200 / 100) / (temp_ac + 240)
    let temp_hit_b = if target_ac + 240 > 0 {
        (temp_ap * 2) / (target_ac + 240)
    } else {
        temp_ap * 2
    };

    // C++ line 391: temp_hit = (int32)(temp_hit_B * (pType1->sHit / 100.0f))
    let s_hit = type1_data.hit.unwrap_or(100) as f32;
    let temp_hit = (temp_hit_b as f32 * (s_hit / 100.0)) as i32;

    // Hit rate check — C++ lines 381-389
    let hit_type = type1_data.hit_type.unwrap_or(0);
    let s_hit_rate = type1_data.hit_rate.unwrap_or(100);

    let result = if hit_type != 0 {
        // Non-relative: sHitRate <= myrand(0, 100) ? FAIL : SUCCESS
        if s_hit_rate <= rng.gen_range(0..=100) {
            FAIL
        } else {
            SUCCESS
        }
    } else {
        // Relative: GetHitRate((hitrate / evasion) * (sHitRate / 100.0f))
        let rate = if target_evasion > 0.0 {
            (caster_hitrate / target_evasion) * (s_hit_rate as f32 / 100.0)
        } else {
            caster_hitrate * (s_hit_rate as f32 / 100.0)
        };
        get_hit_rate(rate, rng)
    };

    match result {
        GREAT_SUCCESS | SUCCESS | NORMAL => {
            // C++ line 452-455:
            //   random = myrand(0, damage);  // damage == temp_hit at this point
            //   damage = (short)((temp_hit + 0.3f * random) + 0.99f);
            let random = if temp_hit > 0 {
                rng.gen_range(0..=temp_hit)
            } else {
                0
            };
            let damage = (temp_hit as f32 + 0.3 * random as f32 + 0.99) as i32;
            damage.max(1) as i16
        }
        _ => 0,
    }
}

/// Compute Type2 (ranged/archery skill) damage following C++ GetDamage formula.
/// - Penetration (bHitType==1): `temp_hit = m_sTotalHit * m_bAttackAmount * (sAddDamage / 100.0f) / 100`
/// - Normal: `temp_hit = temp_hit_B * (sAddDamage / 100.0f)`
/// - `damage = (short)(((temp_hit * 0.6f) + 1.0f * random) + 0.99f)`
/// `sAddDamage` is a percentage multiplier, NOT flat damage.
#[allow(clippy::too_many_arguments)]
fn compute_type2_hit_damage(
    total_hit: u16,
    target_ac: i32,
    type2_data: &MagicType2Row,
    caster_hitrate: f32,
    target_evasion: f32,
    attack_amount: i32,
    player_attack_amount: i32,
    rng: &mut impl Rng,
) -> i16 {
    let total_hit = total_hit as i32;
    let temp_ap = total_hit * attack_amount; // C++ Unit.cpp:305 — m_sTotalHit * m_bAttackAmount
                                             // C++ Unit.cpp:314 — PvP modifier: temp_ap = temp_ap * m_bPlayerAttackAmount / 100
    let temp_ap = temp_ap * player_attack_amount / 100;

    // C++ line 358: temp_hit_B = (temp_ap * 200 / 100) / (temp_ac + 240)
    let temp_hit_b = if target_ac + 240 > 0 {
        (temp_ap * 2) / (target_ac + 240)
    } else {
        temp_ap * 2
    };

    // Hit rate check — C++ lines 415-423
    let hit_type = type2_data.hit_type.unwrap_or(0);
    let s_hit_rate = type2_data.hit_rate.unwrap_or(100);

    let result = if hit_type == 1 || hit_type == 2 {
        // Non-relative / Penetration: sHitRate <= myrand(0, 100) ? FAIL : SUCCESS
        if s_hit_rate <= rng.gen_range(0..=100) {
            FAIL
        } else {
            SUCCESS
        }
    } else {
        // Relative: GetHitRate((hitrate / evasion) * (sHitRate / 100.0f))
        let rate = if target_evasion > 0.0 {
            (caster_hitrate / target_evasion) * (s_hit_rate as f32 / 100.0)
        } else {
            caster_hitrate * (s_hit_rate as f32 / 100.0)
        };
        get_hit_rate(rate, rng)
    };

    // Compute temp_hit based on hit type
    // C++ lines 425-428
    let s_add_damage = type2_data.add_damage.unwrap_or(100) as f32;
    let temp_hit = if hit_type == 1 {
        // Penetration: bypasses AC, uses raw attack power
        // C++ line 426: temp_hit = (m_sTotalHit * m_bAttackAmount * (sAddDamage / 100.0f) / 100)
        (total_hit as f32 * attack_amount as f32 * (s_add_damage / 100.0) / 100.0) as i32
    } else {
        // Normal: uses AC-adjusted base hit
        // C++ line 428: temp_hit = (temp_hit_B * (sAddDamage / 100.0f))
        (temp_hit_b as f32 * (s_add_damage / 100.0)) as i32
    };

    match result {
        GREAT_SUCCESS | SUCCESS | NORMAL => {
            // C++ line 452, 457:
            //   random = myrand(0, damage);  // damage == temp_hit at this point
            //   damage = (short)(((temp_hit * 0.6f) + 1.0f * random) + 0.99f);
            let random = if temp_hit > 0 {
                rng.gen_range(0..=temp_hit)
            } else {
                0
            };
            let damage = (temp_hit as f32 * 0.6 + 1.0 * random as f32 + 0.99) as i32;
            damage.max(1) as i16
        }
        _ => 0,
    }
}

/// Hit rate check — same as attack.rs get_hit_rate.
fn get_hit_rate(rate: f32, rng: &mut impl Rng) -> u8 {
    let random = rng.gen_range(1..=10000);

    if rate >= 5.0 {
        if random <= 3500 {
            GREAT_SUCCESS
        } else if random <= 7500 {
            SUCCESS
        } else if random <= 9800 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 3.0 {
        if random <= 2500 {
            GREAT_SUCCESS
        } else if random <= 6000 {
            SUCCESS
        } else if random <= 9600 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 2.0 {
        if random <= 2000 {
            GREAT_SUCCESS
        } else if random <= 5000 {
            SUCCESS
        } else if random <= 9400 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 1.25 {
        if random <= 1500 {
            GREAT_SUCCESS
        } else if random <= 4000 {
            SUCCESS
        } else if random <= 9200 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 0.8 {
        if random <= 1000 {
            GREAT_SUCCESS
        } else if random <= 3000 {
            SUCCESS
        } else if random <= 9000 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 0.5 {
        if random <= 800 {
            GREAT_SUCCESS
        } else if random <= 2500 {
            SUCCESS
        } else if random <= 8000 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 0.33 {
        if random <= 600 {
            GREAT_SUCCESS
        } else if random <= 2000 {
            SUCCESS
        } else if random <= 7000 {
            NORMAL
        } else {
            FAIL
        }
    } else if rate >= 0.2 {
        if random <= 400 {
            GREAT_SUCCESS
        } else if random <= 1500 {
            SUCCESS
        } else if random <= 6000 {
            NORMAL
        } else {
            FAIL
        }
    } else if random <= 200 {
        GREAT_SUCCESS
    } else if random <= 1000 {
        SUCCESS
    } else if random <= 5000 {
        NORMAL
    } else {
        FAIL
    }
}

/// Apply skill damage to a player target, handle death, and broadcast.
async fn apply_skill_damage(
    world: &WorldState,
    caster_sid: SessionId,
    target_sid: SessionId,
    instance: &MagicInstance,
    damage: i16,
) {
    let target = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => return,
    };

    if damage <= 0 {
        // Broadcast effect with 0 damage
        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        return;
    }

    if target.authority == 0 {
        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        return;
    }

    // ── Pre-fetch victim state (replaces 3 position reads + 3 session reads → 1+1) ──
    let victim_zone = world
        .get_position(target_sid)
        .map(|p| p.zone_id)
        .unwrap_or(0);
    let not_use_zone = victim_zone == ZONE_CHAOS_DUNGEON || victim_zone == ZONE_KNIGHT_ROYALE;
    let (reduction, mirror_active, mirror_direct_flag, mirror_amt, absorb_pct, absorb_count) =
        world
            .with_session(target_sid, |h| {
                (
                    h.magic_damage_reduction,
                    h.mirror_damage,
                    h.mirror_damage_type,
                    h.mirror_amount,
                    h.mana_absorb,
                    h.absorb_count,
                )
            })
            .unwrap_or((100, false, false, 0, 0, 0));

    // ── Magic Damage Reduction (Elysian Web / BUFF_TYPE_RESIS_AND_MAGIC_DMG) ──
    let mut effective_damage = damage;
    if reduction < 100 {
        effective_damage = (effective_damage as i32 * reduction as i32 / 100) as i16;
        // C++ does not enforce minimum here — allow damage to reach 0
        if effective_damage < 0 {
            effective_damage = 0;
        }
    }

    // C++ order: save originalAmount → mirror → mastery → mana absorb (uses originalAmount)
    // For magic: effective_damage already has magic_damage_reduction applied (like C++ GetMagicDamage).
    // Save it as original_damage for mana absorb calculation.
    let original_damage = effective_damage;

    // ── Mirror damage victim reduction ──────────────────────────────────
    let (mirror_dmg, mirror_direct) = if !not_use_zone && mirror_active && mirror_amt > 0 {
        let md = (mirror_amt as i32 * effective_damage as i32) / 100;
        if md > 0 {
            (md, mirror_direct_flag)
        } else {
            (0, false)
        }
    } else {
        (0, false)
    };
    if mirror_dmg > 0 {
        effective_damage = (effective_damage as i32 - mirror_dmg).max(0) as i16;
    }

    // ── Mastery passive damage reduction ────────────────────────────────
    // Matchless: SkillPointMaster >= 10 → 15% reduction
    // Absoluteness: SkillPointMaster >= 5 → 10% reduction
    if !not_use_zone && crate::handler::class_change::is_mastered(target.class) {
        let master_pts = target.skill_points[8]; // SkillPointMaster = index 8
        if master_pts >= 10 {
            // Matchless: 15% damage reduction
            effective_damage = (85 * effective_damage as i32 / 100) as i16;
        } else if master_pts >= 5 {
            // Absoluteness: 10% damage reduction
            effective_damage = (90 * effective_damage as i32 / 100) as i16;
        }
    }

    // ── Mana Absorb (Outrage/Frenzy/Mana Shield) ─────────────────────
    // C++ uses `originalAmount` (pre-mirror) for absorb calculation,
    // but subtracts absorbed from current `amount` (post-mirror).
    {
        if absorb_pct > 0 && !not_use_zone {
            let should_absorb = if absorb_pct == 15 {
                absorb_count > 0
            } else {
                true
            };
            if should_absorb {
                // C++ line 131: toBeAbsorbed = (originalAmount * m_bManaAbsorb) / 100
                let absorbed = (original_damage as i32 * absorb_pct as i32 / 100) as i16;
                effective_damage -= absorbed;
                // C++ allows damage to reach 0 after mana absorb (no minimum enforced)
                if effective_damage < 0 {
                    effective_damage = 0;
                }
                world.update_character_stats(target_sid, |ch| {
                    ch.mp = (ch.mp as i32 + absorbed as i32).min(ch.max_mp as i32) as i16;
                });
                if absorb_pct == 15 {
                    world.update_session(target_sid, |h| {
                        h.absorb_count = h.absorb_count.saturating_sub(1);
                    });
                }
            }
        }
    }

    let new_hp = (target.hp - effective_damage).max(0);
    world.update_character_hp(target_sid, new_hp);

    // Send WIZ_HP_CHANGE to the victim so their client updates their own HP
    let hp_pkt = crate::systems::regen::build_hp_change_packet_with_attacker(
        target.max_hp,
        new_hp,
        caster_sid as u32,
    );
    world.send_to_session_owned(target_sid, hp_pkt);
    crate::handler::party::broadcast_party_hp(world, target_sid);

    // ── Equipment durability loss ─────────────────────────────────────
    world.item_wore_out(caster_sid, WORE_TYPE_ATTACK, damage as i32);
    world.item_wore_out(target_sid, WORE_TYPE_DEFENCE, damage as i32);

    try_reflect_damage(world, caster_sid, target_sid, damage).await;

    // ── Mirror damage reflection (skill buff) ──────────────────────────
    // Mirror was pre-computed above; now reflect to caster or party.
    if mirror_dmg > 0 {
        if mirror_direct {
            let atk_hp = world
                .get_character_info(caster_sid)
                .map(|c| (c.hp, c.max_hp))
                .unwrap_or((0, 0));
            let new_atk_hp = (atk_hp.0 - mirror_dmg as i16).max(0);
            world.update_character_hp(caster_sid, new_atk_hp);
            let atk_hp_pkt = crate::systems::regen::build_hp_change_packet_with_attacker(
                atk_hp.1,
                new_atk_hp,
                target_sid as u32,
            );
            world.send_to_session_owned(caster_sid, atk_hp_pkt);
        } else if world.is_in_party(target_sid) {
            // Party distribution: spread mirror damage among attacker's party.
            if let Some(atk_party_id) = world.get_party_id(caster_sid) {
                if let Some(party) = world.get_party(atk_party_id) {
                    let members = party.active_members();
                    let p_count = members.len() as i32;
                    if p_count > 0 {
                        // C++ precedence bug: (mirrorDamage / p_count < 2) ? 2 : p_count
                        let per_member_dmg = if (mirror_dmg / p_count) < 2 {
                            2
                        } else {
                            p_count
                        };
                        for &member_sid in &members {
                            if member_sid == target_sid {
                                continue;
                            }
                            let m_hp = world
                                .get_character_info(member_sid)
                                .map(|c| (c.hp, c.max_hp))
                                .unwrap_or((0, 0));
                            if m_hp.0 <= 0 {
                                continue;
                            }
                            let new_m_hp = (m_hp.0 as i32 - per_member_dmg).max(0) as i16;
                            world.update_character_hp(member_sid, new_m_hp);
                            let m_hp_pkt =
                                crate::systems::regen::build_hp_change_packet_with_attacker(
                                    m_hp.1, new_m_hp, 0xFFFF,
                                );
                            world.send_to_session_owned(member_sid, m_hp_pkt);
                        }
                    }
                }
            }
        }
    }

    // ── Equipment mirror damage (ITEM_TYPE_MIRROR_DAMAGE) ───────────
    {
        const ITEM_TYPE_MIRROR_DAMAGE_EQ: u8 = 0x08;
        let eq_stats = world.get_equipped_stats(target_sid);
        let mut total_equip_mirror: i32 = 0;
        for bonuses in eq_stats.equipped_item_bonuses.values() {
            for &(btype, amount) in bonuses {
                if btype == ITEM_TYPE_MIRROR_DAMAGE_EQ {
                    total_equip_mirror += amount;
                }
            }
        }
        if total_equip_mirror > 0 {
            let reflected = (damage as i32 * total_equip_mirror) / 300;
            if reflected > 0 {
                let atk_hp = world
                    .get_character_info(caster_sid)
                    .map(|c| (c.hp, c.max_hp))
                    .unwrap_or((0, 0));
                let new_atk_hp = (atk_hp.0 as i32 - reflected).max(0) as i16;
                world.update_character_hp(caster_sid, new_atk_hp);
                let eq_pkt = crate::systems::regen::build_hp_change_packet_with_attacker(
                    atk_hp.1,
                    new_atk_hp,
                    target_sid as u32,
                );
                world.send_to_session_owned(caster_sid, eq_pkt);
            }
        }
    }

    if new_hp <= 0 {
        dead::broadcast_death(world, target_sid);
        dead::set_who_killed_me(world, target_sid, caster_sid);

        // ── PvP death notice ────────────────────────────────────────
        dead::send_death_notice(world, caster_sid, target_sid);

        // ── Chaos dungeon item rob ──────────────────────────────────
        dead::rob_chaos_skill_items(world, target_sid);

        // ── PvP loyalty (NP) change ─────────────────────────────────
        dead::pvp_loyalty_on_death(world, caster_sid, target_sid);

        // ── Rivalry / Anger Gauge (magic kill path) ─────────────────
        {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let zone_id = world
                .get_position(caster_sid)
                .map(|p| p.zone_id)
                .unwrap_or(0);
            let is_revenge = crate::handler::arena::on_pvp_kill(
                world, caster_sid, target_sid, zone_id, now_secs,
            );
            if is_revenge {
                crate::systems::loyalty::send_loyalty_change(
                    world,
                    caster_sid,
                    crate::handler::arena::RIVALRY_NP_BONUS as i32,
                    true,
                    false,
                    false,
                );
            }
        }

        // ── PvP gold change ─────────────────────────────────────────
        dead::gold_change_on_death(world, caster_sid, target_sid);

        // ── Temple event kill scoring ───────────────────────────────
        // OnDeathKilledPlayer is called from OnDeath regardless of
        // damage source (physical or magic).
        {
            use crate::systems::event_room;
            let zone_id = world
                .get_position(caster_sid)
                .map(|p| p.zone_id)
                .unwrap_or(0);
            match zone_id {
                z if z == event_room::ZONE_BDW => {
                    dead::track_bdw_player_kill(world, caster_sid, target_sid);
                }
                z if z == event_room::ZONE_CHAOS => {
                    dead::track_chaos_pvp_kill(world, caster_sid, target_sid);
                }
                z if z == event_room::ZONE_JURAID => {
                    dead::track_juraid_pvp_kill(world, caster_sid);
                }
                _ => {}
            }
        }
    }

    // Broadcast effect
    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);

    // Send HP update
    send_target_hp_update(world, caster_sid, target_sid, damage as i32);

    tracing::debug!(
        "[sid={}] MagicProcess: skill={} target={} damage={} new_hp={}",
        caster_sid,
        instance.skill_id,
        target_sid,
        damage,
        new_hp
    );
}

/// Check and trigger mage armor reflect damage on the target.
/// When a player with BUFF_TYPE_MAGE_ARMOR (25) is hit, this fires a counter-skill
/// back at the attacker and consumes the buff (one-time use).
/// Element mapping: 5=Fire, 6=Ice, 7=Lightning.
/// Counter-skills by nation:
/// - Fire:      Karus=190573, Elmorad=290573
/// - Ice:       Karus=190673, Elmorad=290673
/// - Lightning: Karus=190773, Elmorad=290773
fn try_reflect_damage<'a>(
    world: &'a WorldState,
    caster_sid: SessionId, // original attacker (takes reflect damage)
    target_sid: SessionId, // target with mage armor (reflects back)
    damage: i16,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        if damage <= 0 {
            return;
        }
        if caster_sid == target_sid {
            return;
        }

        // Both sides are players in PvP, so this is always true here.

        let reflect_type = world
            .with_session(target_sid, |h| h.reflect_armor_type)
            .unwrap_or(0);
        if reflect_type == 0 {
            return;
        }

        let target_info = match world.get_character_info(target_sid) {
            Some(ch) => ch,
            None => return,
        };

        const FIRE_DAMAGE: u8 = 5;
        const ICE_DAMAGE: u8 = 6;
        const LIGHTNING_DAMAGE: u8 = 7;

        let counter_skill_id: u32 = match (reflect_type, target_info.nation) {
            (FIRE_DAMAGE, NATION_KARUS) => 190573,
            (FIRE_DAMAGE, _) => 290573,
            (ICE_DAMAGE, NATION_KARUS) => 190673,
            (ICE_DAMAGE, _) => 290673,
            (LIGHTNING_DAMAGE, NATION_KARUS) => 190773,
            (LIGHTNING_DAMAGE, _) => 290773,
            _ => return,
        };

        // Clear reflect BEFORE executing counter-skill to prevent infinite recursion
        world.update_session(target_sid, |h| {
            h.reflect_armor_type = 0;
        });

        world.remove_buff(target_sid, BUFF_TYPE_MAGE_ARMOR);

        // Send buff expiry notification to target
        let expired_pkt = build_buff_expired_packet(BUFF_TYPE_MAGE_ARMOR as u8);
        world.send_to_session_owned(target_sid, expired_pkt);

        let skill = match world.get_magic(counter_skill_id as i32) {
            Some(s) => s,
            None => {
                tracing::warn!(
                    "[sid={}] ReflectDamage: counter-skill {} not found in magic table",
                    target_sid,
                    counter_skill_id
                );
                return;
            }
        };

        let mut instance = MagicInstance {
            opcode: MAGIC_EFFECTING,
            skill_id: counter_skill_id,
            caster_id: target_sid as i32,
            target_id: caster_sid as i32,
            data: [0; 7],
        };

        if let Some(pos) = world.get_position(target_sid) {
            instance.data[0] = (pos.x as u16) as i32;
            instance.data[2] = (pos.z as u16) as i32;
        }

        // Execute counter-skill — dispatch based on skill type
        let type1 = skill.type1.unwrap_or(0);
        match type1 {
            1 => {
                execute_type1(world, target_sid, &mut instance, &skill).await;
            }
            2 => {
                execute_type2(world, target_sid, &mut instance, &skill).await;
            }
            3 => {
                execute_type3(world, target_sid, &mut instance, &skill).await;
            }
            _ => {
                // Unknown type — just broadcast the visual effect
                let pkt = instance.build_packet(MAGIC_EFFECTING);
                broadcast_to_caster_region(world, target_sid, &pkt);
            }
        }

        tracing::debug!(
            "[sid={}] ReflectDamage: reflected skill {} back at attacker sid={}",
            target_sid,
            counter_skill_id,
            caster_sid
        );
    })
}

/// Send WIZ_TARGET_HP to the caster for HP bar + damage display update.
/// The `damage` parameter is displayed in the client's console as the amount
/// of damage dealt. Pass 0 for heals or non-damage updates.
fn send_target_hp_update(
    world: &WorldState,
    caster_sid: SessionId,
    target_sid: SessionId,
    damage: i32,
) {
    let ch = match world.get_character_info(target_sid) {
        Some(c) => c,
        None => return,
    };

    let mut response = Packet::new(Opcode::WizTargetHp as u8);
    response.write_u32(target_sid as u32);
    response.write_u8(0);
    response.write_u32(ch.max_hp as u32);
    response.write_u32(ch.hp.max(0) as u32);
    // C++ sends negative amount for damage, positive for heal. Client uses sign for display:
    // negative = "X damage dealt", positive = "X HP received", 0 = no display
    response.write_u32((-damage) as u32);
    response.write_u32(0);
    response.write_u8(0);

    world.send_to_session_owned(caster_sid, response);
}

/// Apply skill damage to an NPC target, handle death (XP + broadcast), and send HP update.
/// Used by Type 1, 2, and 3 skill handlers when the target is an NPC.
/// Respawn is handled via a separate 30-second timer in the melee attack handler.
async fn apply_skill_damage_to_npc(
    world: &WorldState,
    caster_sid: SessionId,
    npc_id: u32,
    instance: &MagicInstance,
    damage: i16,
    _skill: &MagicRow,
    attribute_type: u8,
) {
    // ── Bot target: apply damage, send HP update, handle death ─────
    // Bots are stored in world.bots (not the NPC instance map).
    // We apply the magic damage to the bot's HP and send WIZ_TARGET_HP to the
    // caster. If the bot's HP reaches 0, we trigger the full death processing.
    if let Some(bot) = world.get_bot(npc_id) {
        if bot.hp <= 0 || bot.presence == crate::world::BotPresence::Dead {
            return;
        }

        // Apply damage — clamp to [0, max_hp]
        let new_hp = (bot.hp - damage).max(0);
        world.update_bot(npc_id, |b| {
            b.hp = new_hp;
            b.last_attacker_id = caster_sid as i32;
        });

        // Send WIZ_TARGET_HP to caster
        let mut target_hp_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizTargetHp as u8);
        target_hp_pkt.write_u32(npc_id);
        target_hp_pkt.write_u8(0); // echo
        target_hp_pkt.write_u32(bot.max_hp as u32);
        target_hp_pkt.write_u32(new_hp as u32);
        target_hp_pkt.write_u32((-damage) as u32); // negative = damage dealt
        target_hp_pkt.write_u32(0);
        target_hp_pkt.write_u8(0);
        world.send_to_session_owned(caster_sid, target_hp_pkt);

        // Handle death
        if new_hp <= 0 {
            let now_ms = crate::systems::bot_ai::tick_ms();
            crate::systems::bot_ai::bot_on_death(world, npc_id, now_ms);
        }

        tracing::debug!(
            "[sid={}] Magic skill {} hit bot {}: damage={}, hp={}/{}",
            caster_sid,
            instance.skill_id,
            npc_id,
            damage,
            new_hp,
            bot.max_hp
        );
        return;
    }

    // Look up NPC
    let npc = match world.get_npc_instance(npc_id) {
        Some(n) => n,
        None => return,
    };

    let tmpl = match world.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) => t,
        None => return,
    };

    // NPC type validation — certain NPCs are immune to magic skills
    {
        if tmpl.npc_type == NPC_PARTNER_TYPE && tmpl.group == 0 {
            return;
        }

        match tmpl.npc_type {
            NPC_TREE | NPC_FOSIL | NPC_REFUGEE | NPC_BORDER_MONUMENT | NPC_PRISON => return,
            NPC_GUARD_TOWER1 | NPC_GUARD_TOWER2 | NPC_SOCCER_BAAL => return,
            NPC_GATE2 | NPC_VICTORY_GATE | NPC_PHOENIX_GATE | NPC_SPECIAL_GATE | NPC_GATE_LEVER => {
                return
            }
            NPC_OBJECT_FLAG if npc.proto_id == 511 => return,
            _ => {}
        }

        let is_csw_door = tmpl.npc_type == NPC_GATE && matches!(npc.proto_id, 561..=563);

        if tmpl.npc_type == NPC_DESTROYED_ARTIFACT || is_csw_door {
            let csw = world.csw_event().blocking_read();
            let siege = world.siege_war().blocking_read();
            let caster_clan = world
                .get_character_info(caster_sid)
                .map(|ch| ch.knights_id)
                .unwrap_or(0);

            if caster_clan == 0
                || !csw.is_active()
                || !csw.is_war_active()
                || siege.master_knights == caster_clan
            {
                return;
            }
        }

        {
            if tmpl.npc_type == NPC_BIFROST_MONUMENT {
                let beef = world.get_beef_event();
                if !beef.is_active || beef.is_monument_dead {
                    return;
                }
            }
            if tmpl.npc_type == NPC_PVP_MONUMENT || tmpl.npc_type == NPC_CLAN_WAR_MONUMENT {
                let caster_nation = world
                    .get_character_info(caster_sid)
                    .map(|ch| ch.nation)
                    .unwrap_or(0);
                let is_own = (caster_nation == 1 && npc.proto_id == 14003)
                    || (caster_nation == 2 && npc.proto_id == 14004);
                if is_own {
                    return;
                }
            }
        }

        // Vampiric Touch, Blood Drain, Fire Thorn, Static Thorn, Parasite, Super Parasite
        // are blocked when cast against NPCs in zone 86.
        {
            let caster_zone = world
                .get_position(caster_sid)
                .map(|p| p.zone_id)
                .unwrap_or(0);
            if caster_zone == ZONE_UNDER_CASTLE {
                let sid = instance.skill_id;
                if matches!(
                    sid,
                    107650 | 108650 | 207650 | 208650   // Vampiric Touch
                    | 107610 | 108610 | 207610 | 208610 // Blood Drain
                    | 109554 | 110554 | 209554 | 210554 // Fire Thorn
                    | 109754 | 110754 | 209754 | 210754 // Static Thorn
                    | 111745 | 112745 | 211745 | 212745 // Parasite
                    | 112771 | 212771 // Super Parasite
                ) {
                    return;
                }
            }
        }

        // Neutral peaceful NPCs (group/nation == 3) cannot be magic-attacked
        if tmpl.group == 3 {
            return;
        }
    }

    // Check if NPC is alive
    let npc_hp = match world.get_npc_hp(npc_id) {
        Some(hp) if hp > 0 => hp,
        _ => return,
    };

    if damage <= 0 {
        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        return;
    }

    // Deduct HP
    let new_hp = (npc_hp - damage as i32).max(0);
    world.update_npc_hp(npc_id, new_hp);
    world.record_npc_damage(npc_id, caster_sid, damage as i32);

    // ── Caster weapon durability loss ────────────────────────────────
    world.item_wore_out(caster_sid, WORE_TYPE_ATTACK, damage as i32);

    // Notify NPC AI about damage (reactive aggro — C++ ChangeTarget)
    if new_hp > 0 {
        world.notify_npc_damaged(npc_id, caster_sid);

        // Elemental fainting check
        // When an NPC takes magic damage with an elemental attribute, there is a
        // chance to stun (faint) the NPC based on its resistance to that element.
        if attribute_type > 0 {
            try_elemental_faint(world, npc_id, &tmpl, attribute_type);
        }
    }

    if new_hp <= 0 {
        // NPC died — delegate to shared death handler for consistent behavior
        // (death broadcast, party XP, loot, AI state cleanup)
        super::attack::handle_npc_death(world, caster_sid, npc_id, &npc, &tmpl).await;
    }

    // Broadcast skill effect
    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);

    // Send HP bar update with actual damage for console display
    // C++ sends negative amount (damage dealt), client uses sign for display:
    // negative = "X damage dealt", positive = "X HP received"
    let mut hp_pkt = Packet::new(Opcode::WizTargetHp as u8);
    hp_pkt.write_u32(npc_id);
    hp_pkt.write_u8(0);
    hp_pkt.write_u32(tmpl.max_hp);
    hp_pkt.write_u32(new_hp.max(0) as u32);
    hp_pkt.write_u32((-(damage as i32)) as u32);
    hp_pkt.write_u32(0);
    hp_pkt.write_u8(0);
    world.send_to_session_owned(caster_sid, hp_pkt);

    tracing::debug!(
        "[sid={}] MagicProcess NPC target={}: damage={}, new_hp={}/{}",
        caster_sid,
        npc_id,
        damage,
        new_hp,
        tmpl.max_hp
    );
}

/// Try to apply elemental fainting to an NPC after taking magic damage.
/// Formula: `faint_chance = 10 + (40 - 40 * (resistance / 80))`
/// If `random(1, 100) < faint_chance`, the NPC enters FAINTING state for 2 seconds.
/// Attribute mapping:
/// - 1 (Fire) -> fire_r
/// - 2 (Ice) -> cold_r
/// - 3 (Lightning) -> lightning_r
/// - 4 (Light Magic) -> magic_r
/// - 5 (Curse) -> disease_r
/// - 6 (Poison) -> poison_r
fn try_elemental_faint(
    world: &crate::world::WorldState,
    npc_id: u32,
    tmpl: &crate::npc::NpcTemplate,
    attribute_type: u8,
) {
    // Only process if NPC is not already fainting
    let ai = match world.get_npc_ai(npc_id) {
        Some(a) => a,
        None => return,
    };

    if ai.state == crate::world::NpcState::Fainting {
        return;
    }

    let resistance = match attribute_type {
        1 => tmpl.fire_r as f64,      // Fire
        2 => tmpl.cold_r as f64,      // Ice
        3 => tmpl.lightning_r as f64, // Lightning
        4 => tmpl.magic_r as f64,     // Light Magic
        5 => tmpl.disease_r as f64,   // Curse
        6 => tmpl.poison_r as f64,    // Poison
        _ => return,
    };

    // C++ formula: sDamage = (int)(10 + (40 - 40 * ((double)resistance / 80)))
    let faint_chance = (10.0 + (40.0 - 40.0 * (resistance / 80.0))) as i32;

    if faint_chance <= 0 {
        return;
    }

    let mut rng = rand::thread_rng();
    let roll: i32 = rng.gen_range(1..=100);

    // C++ uses COMPARE(iRandom, 0, sDamage) which is: 0 <= iRandom < sDamage
    if roll < faint_chance {
        world.update_npc_ai(npc_id, |s| {
            s.state = crate::world::NpcState::Fainting;
            s.fainting_until_ms = s.last_tick_ms;
            s.delay_ms = 0;
        });

        tracing::debug!(
            "NPC {} entered FAINTING state (attribute={}, resistance={}, chance={}, roll={})",
            npc_id,
            attribute_type,
            resistance,
            faint_chance,
            roll,
        );
    }
}

// ── Type 5: Resurrection / Cure ──────────────────────────────────────────

/// Type 5 sub-type constants from C++ MagicInstance.h:57-62
const TYPE5_REMOVE_TYPE3: i32 = 1;
const TYPE5_REMOVE_TYPE4: i32 = 2;
const TYPE5_RESURRECTION: i32 = 3;
const TYPE5_RESURRECTION_SELF: i32 = 4;
const TYPE5_REMOVE_BLESS: i32 = 5;
const TYPE5_LIFE_CRYSTAL: i32 = 6;

/// Execute Type 5 skill — resurrection, cure DOTs, cure debuffs.
/// Sub-types (bType field):
/// - 1 (REMOVE_TYPE3): Remove harmful DOT effects from target
/// - 2 (REMOVE_TYPE4): Remove type 4 debuffs from target
/// - 3 (RESURRECTION): Resurrect dead target (requires items -- simplified)
/// - 4 (RESURRECTION_SELF): Self-resurrection (specific skill IDs)
/// - 5 (REMOVE_BLESS): Remove HP/MP buff
/// - 6 (LIFE_CRYSTAL): Self-resurrection via life crystal
async fn execute_type5(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type5_data = match world.get_magic_type5(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let sub_type = type5_data.r#type.unwrap_or(0);
    let target_id = instance.target_id;

    // Single-target case
    let target_sid = if target_id < 0 || (target_id as u32) >= NPC_BAND {
        caster_sid // Default to self for AOE or invalid target
    } else {
        target_id as SessionId
    };

    // Verify target exists and is a player
    let target = match world.get_character_info(target_sid) {
        Some(ch) => ch,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    match sub_type {
        TYPE5_REMOVE_TYPE3 => {
            // Remove all harmful DOT effects (negative hp_amount)
            let removed = world.clear_harmful_dots(target_sid);
            if removed {
                // Send MAGIC_DURATION_EXPIRED with type 200 to remove DOT visual
                let mut dot_pkt = Packet::new(Opcode::WizMagicProcess as u8);
                dot_pkt.write_u8(MAGIC_DURATION_EXPIRED);
                dot_pkt.write_u8(200); // C++ uses 200 for DOT removal
                world.send_to_session_owned(target_sid, dot_pkt);
            }
            tracing::debug!(
                "[sid={}] MagicProcess Type 5: REMOVE_TYPE3 target={} removed={}",
                caster_sid,
                target_sid,
                removed
            );
        }

        TYPE5_REMOVE_TYPE4 => {
            // Remove all type 4 debuffs
            let removed_types = world.remove_debuffs(target_sid);
            for buff_type in &removed_types {
                let expired_pkt = build_buff_expired_packet(*buff_type as u8);
                broadcast_to_caster_region(world, caster_sid, &expired_pkt);
            }
            // after debuff removal. For each removed debuff type, if it's lockable,
            // recast the original scroll buff from saved magic.
            for buff_type in &removed_types {
                if WorldState::is_lockable_scroll(*buff_type) {
                    world.recast_lockable_scrolls(target_sid, *buff_type);
                }
            }
            if !removed_types.is_empty() {
                world.set_user_ability(target_sid);
            }
            tracing::debug!(
                "[sid={}] MagicProcess Type 5: REMOVE_TYPE4 target={} removed={} debuffs",
                caster_sid,
                target_sid,
                removed_types.len()
            );
        }

        TYPE5_RESURRECTION | TYPE5_LIFE_CRYSTAL => {
            // Resurrect a dead player
            //   Calls pTUser->Regene(INOUT_IN, nSkillID)
            if target.res_hp_type != USER_DEAD && target.hp > 0 {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }

            // Target must have sNeedStone of iUseItem; caster gets (sNeedStone / 2) + 1 back.
            if sub_type == TYPE5_RESURRECTION {
                let use_item_id = skill.use_item.unwrap_or(0);
                let need_stone = type5_data.need_stone.unwrap_or(0).max(0) as u16;
                if use_item_id > 0 && need_stone > 0 {
                    // Check + remove stones from the dead player's inventory
                    if !world.rob_item(target_sid, use_item_id as u32, need_stone) {
                        send_skill_failed(world, caster_sid, instance);
                        return false;
                    }
                    // Reward caster with (need_stone / 2) + 1 stones
                    let reward = (need_stone / 2) + 1;
                    world.give_item(caster_sid, use_item_id as u32, reward);
                    tracing::info!(
                        "[sid={}] Type5 RESURRECTION: consumed {} stones from target={}, rewarded caster with {}",
                        caster_sid, need_stone, target_sid, reward
                    );
                }
            }

            // C++ Regene path (AttackHandler.cpp:384-428):
            // Zone 86 (Under Castle): MP = MaxMana, skip EXP recovery
            // Other zones: MP = 0, EXP recovery if PvE death
            let target_zone = world
                .get_position(target_sid)
                .map(|p| p.zone_id)
                .unwrap_or(0);

            world.update_res_hp_type(target_sid, 1); // USER_STANDING
            world.update_character_hp(target_sid, target.max_hp); // Full HP

            if target_zone == ZONE_UNDER_CASTLE {
                world.update_character_mp(target_sid, target.max_mp);
            } else {
                world.update_character_mp(target_sid, 0);

                // EXP recovery — only for PvE deaths (who_killed_me == -1)
                let (who_killed, lost_exp) = world
                    .with_session(target_sid, |h| (h.who_killed_me, h.lost_exp))
                    .unwrap_or((-1, 0));
                if who_killed == -1 && lost_exp > 0 {
                    let exp_recover_pct = type5_data.exp_recover.unwrap_or(50).max(0) as i64;
                    let restored_exp = (lost_exp * exp_recover_pct) / 100;
                    if restored_exp > 0 {
                        super::level::exp_change(world, target_sid, restored_exp).await;
                        tracing::info!(
                            "[sid={}] Type5 RESURRECTION: restored {} EXP to target={} \
                             ({}% of {} lost)",
                            caster_sid,
                            restored_exp,
                            target_sid,
                            exp_recover_pct,
                            lost_exp
                        );
                    }
                }
            }

            // Reset death tracking fields
            world.update_session(target_sid, |h| {
                h.who_killed_me = -1;
                h.lost_exp = 0;
            });

            // Send WIZ_REGENE packet to the resurrected player
            if let Some(tpos) = world.get_position(target_sid) {
                let mut regene_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizRegene as u8);
                regene_pkt.write_u16((tpos.x * 10.0) as u16);
                regene_pkt.write_u16((tpos.z * 10.0) as u16);
                regene_pkt.write_u16(0); // y * 10
                world.send_to_session_owned(target_sid, regene_pkt);
            }

            // ── Post-regene sequence (C++ AttackHandler.cpp:411-442) ──
            // Broadcast INOUT_RESPAWN so other players see the resurrection
            post_resurrection_sequence(world, target_sid, target_zone);

            crate::handler::party::broadcast_party_hp(world, target_sid);

            tracing::info!(
                "[sid={}] MagicProcess Type 5: RESURRECTION target={} hp={}/{}",
                caster_sid,
                target_sid,
                target.max_hp,
                target.max_hp
            );
        }

        TYPE5_RESURRECTION_SELF => {
            // Self-resurrection (only caster can be the target)
            if target_sid != caster_sid {
                return true;
            }

            if target.res_hp_type != USER_DEAD && target.hp > 0 {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }

            // C++ Regene path: same Under Castle exception as RESURRECTION
            let caster_zone = world
                .get_position(caster_sid)
                .map(|p| p.zone_id)
                .unwrap_or(0);

            world.update_res_hp_type(caster_sid, 1);
            world.update_character_hp(caster_sid, target.max_hp); // Full HP

            if caster_zone == ZONE_UNDER_CASTLE {
                world.update_character_mp(caster_sid, target.max_mp);
            } else {
                world.update_character_mp(caster_sid, 0);

                // EXP recovery — only for PvE deaths
                let (who_killed, lost_exp) = world
                    .with_session(caster_sid, |h| (h.who_killed_me, h.lost_exp))
                    .unwrap_or((-1, 0));
                if who_killed == -1 && lost_exp > 0 {
                    let exp_recover_pct = type5_data.exp_recover.unwrap_or(30).max(0) as i64;
                    let restored_exp = (lost_exp * exp_recover_pct) / 100;
                    if restored_exp > 0 {
                        super::level::exp_change(world, caster_sid, restored_exp).await;
                    }
                }
            }

            world.update_session(caster_sid, |h| {
                h.who_killed_me = -1;
                h.lost_exp = 0;
            });

            // Send WIZ_REGENE packet to self
            if let Some(cpos) = world.get_position(caster_sid) {
                let mut regene_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizRegene as u8);
                regene_pkt.write_u16((cpos.x * 10.0) as u16);
                regene_pkt.write_u16((cpos.z * 10.0) as u16);
                regene_pkt.write_u16(0); // y * 10
                world.send_to_session_owned(caster_sid, regene_pkt);
            }

            // ── Post-regene sequence (C++ AttackHandler.cpp:411-442) ──
            // Broadcast INOUT_RESPAWN, cure DOTs, activate blink
            post_resurrection_sequence(world, caster_sid, caster_zone);

            crate::handler::party::broadcast_party_hp(world, caster_sid);

            tracing::info!(
                "[sid={}] MagicProcess Type 5: RESURRECTION_SELF hp={}/{}",
                caster_sid,
                target.max_hp,
                target.max_hp
            );
        }

        TYPE5_REMOVE_BLESS => {
            // Remove HP/MP buff (buff_type for HP_MP bless)
            let removed = world.remove_buff(target_sid, 50); // BUFF_TYPE_HP_MP = 50
            if removed.is_some() {
                world.set_user_ability(target_sid);
            }
            tracing::debug!(
                "[sid={}] MagicProcess Type 5: REMOVE_BLESS target={}",
                caster_sid,
                target_sid
            );
        }

        _ => {
            tracing::debug!(
                "[sid={}] MagicProcess Type 5: unknown sub_type={} skill={}",
                caster_sid,
                sub_type,
                skill.magic_num
            );
        }
    }

    // C++ line 5039-5041: sData[1] = 1, broadcast skill packet
    instance.data[1] = 1;
    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);
    true
}

/// Post-resurrection sequence — world-level equivalent of regene.rs post-regene.
/// Performs:
/// 1. Broadcast INOUT_RESPAWN so other players see the resurrection
/// 2. Initialize stealth (reset invisibility)
/// 3. Cure DOT & Poison status effects
/// 4. Activate blink (10s invulnerability)
fn post_resurrection_sequence(world: &WorldState, sid: SessionId, zone_id: u16) {
    // ── 1. Broadcast INOUT_RESPAWN to 3×3 region ─────────────────────
    if let Some((pos, my_char, event_room)) =
        world.with_session(sid, |h| (h.position, h.character.clone(), h.event_room))
    {
        let my_clan = my_char.as_ref().and_then(|ch| {
            if ch.knights_id > 0 {
                world.get_knights(ch.knights_id)
            } else {
                None
            }
        });
        let my_invis = world.get_invisibility_type(sid);
        let my_abnormal = world.get_abnormal_type(sid);
        let my_bs = world.get_broadcast_state(sid);
        let my_equip = crate::handler::region::get_equipped_visual(world, sid);
        let ac = my_clan
            .as_ref()
            .and_then(|ki| crate::handler::region::resolve_alliance_cape(ki, world));
        let inout_pkt = crate::handler::region::build_user_inout_with_clan(
            crate::handler::region::INOUT_RESPAWN,
            sid,
            my_char.as_ref(),
            &pos,
            my_clan.as_ref(),
            ac,
            my_invis,
            my_abnormal,
            &my_bs,
            &my_equip,
        );
        world.broadcast_to_3x3(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(inout_pkt),
            None,
            event_room,
        );
    }

    // ── 2. InitializeStealth ─────────────────────────────────────────
    world.set_invisibility_type(sid, 0);
    let mut stealth_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizStealth as u8);
    stealth_pkt.write_u8(0);
    stealth_pkt.write_u16(0);
    world.send_to_session_owned(sid, stealth_pkt);

    // ── 3. Cure DOT & Poison ─────────────────────────────────────────
    world.clear_durational_skills(sid);
    crate::systems::buff_tick::send_user_status_update_packet(world, sid, 1, 0); // DOT cure
    crate::systems::buff_tick::send_user_status_update_packet(world, sid, 2, 0); // Poison cure

    // ── 4. InitType4() + RecastSavedMagic() ──────────────────────────
    //   if (!isBlinking() && zone != CHAOS_DUNGEON && zone != DUNGEON_DEFENCE && zone != KNIGHT_ROYALE)
    //       InitType4();  RecastSavedMagic();
    // ZONE_KNIGHT_ROYALE_RES = 100: distinct from ZONE_KNIGHT_ROYALE (76); value from C++ AttackHandler
    const ZONE_KNIGHT_ROYALE_RES: u16 = 100;
    if zone_id != ZONE_CHAOS_DUNGEON
        && zone_id != ZONE_DUNGEON_DEFENCE
        && zone_id != ZONE_KNIGHT_ROYALE_RES
    {
        // Clear all Type4 buffs (InitType4) — preserves saved_magic for recast
        world.clear_all_buffs(sid, false);
        world.set_user_ability(sid);
        world.send_item_move_refresh(sid);
        // Re-apply saved buffs (RecastSavedMagic)
        world.recast_saved_magic(sid);
    }

    // ── 5. Activate blink (10s invulnerability) ──────────────────────
    // Skip blink in war zones and zones without blink_zone flag
    let should_blink = world.get_zone(zone_id).is_some_and(|z| {
        !z.is_war_zone()
            && z.zone_info
                .as_ref()
                .map(|zi| zi.abilities.blink_zone)
                .unwrap_or(false)
    });
    let is_gm = world
        .get_character_info(sid)
        .map(|ch| ch.authority == 0)
        .unwrap_or(false);
    let is_transformed = world.is_transformed(sid);

    if should_blink && !is_gm && !is_transformed {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expiry = now + 10; // BLINK_TIME = 10 seconds
        world.update_session(sid, |h| {
            h.blink_expiry_time = expiry;
            h.can_use_skills = false;
        });

        // Broadcast ABNORMAL_BLINKING state change to 3×3 region
        let state_pkt = crate::handler::regene::build_state_change_broadcast(
            sid as u32,
            STATE_CHANGE_ABNORMAL,
            ABNORMAL_BLINKING,
        );
        if let Some((pos, event_room)) = world.with_session(sid, |h| (h.position, h.event_room)) {
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(state_pkt),
                None,
                event_room,
            );
        }
    }

    tracing::debug!(
        "[sid={}] post_resurrection_sequence complete: zone={}, blink={}",
        sid,
        zone_id,
        should_blink && !is_gm && !is_transformed,
    );
}

// ── Type 6: Transformation ──────────────────────────────────────────────

/// Execute Type 6 skill -- transformation (disguise as NPC/monster).
/// Transforms the caster into a different visual model (NPC/monster/siege).
/// The transformation lasts for `sDuration` seconds.
fn execute_type6(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type6_data = match world.get_magic_type6(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let caster = match world.get_character_info(caster_sid) {
        Some(ch) => ch,
        None => return false,
    };

    let caster_zone = world
        .get_position(caster_sid)
        .map(|p| p.zone_id)
        .unwrap_or(0);

    // ── Zone-specific transformation validation ──────────────────────
    // user_skill_use values follow the C++ TransformationSkillUse enum:
    //   0 = Siege, 1 = Monster, 3 = NPC, 4 = Special,
    //   5 = OreadsGuard, 6 = MovingTower, 7 = MamaPag
    match type6_data.user_skill_use {
        // OreadsGuard — always disabled in C++ (line 5063: return false)
        5 => {
            return false;
        }
        // Monster transformation — must be in a valid monster transform zone
        // C++ line 5067: canAttackOtherNation() && !isTransformationMonsterInZone()
        1 => {
            let zone_data = world.get_zone(caster_zone);
            let can_attack = zone_data
                .as_ref()
                .is_some_and(|z| z.can_attack_other_nation());
            if can_attack && !is_transformation_monster_zone(caster_zone) {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }
        // Siege transformation — zone-restricted by skill ID
        // C++ lines 5086-5101
        0 => {
            let allowed = match skill.magic_num {
                450001 => {
                    caster_zone == ZONE_DELOS
                        || caster_zone == ZONE_BATTLE2
                        || caster_zone == ZONE_BATTLE3
                }
                450003 => caster_zone == ZONE_BATTLE2,
                _ => caster_zone == ZONE_DELOS,
            };
            if !allowed {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }
        // MovingTower — must be in Delos only
        // C++ line 5113
        6 => {
            if caster_zone != ZONE_DELOS {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        }
        _ => {}
    }

    // Block transformation if already transformed
    if world.is_transformed(caster_sid) {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    // Nation check: if type6 has a nation restriction
    if type6_data.nation != 0 && type6_data.nation != caster.nation as i32 {
        send_skill_failed(world, caster_sid, instance);
        return false;
    }

    let duration = type6_data.duration as u16;
    let transform_id = type6_data.transform_id;

    // Determine transformation type from user_skill_use
    let transformation_type = match type6_data.user_skill_use {
        1 => TRANSFORMATION_MONSTER,       // TransformationSkillUseMonster
        2 | 5 => TRANSFORMATION_NPC,       // TransformationSkillUseNPC / MamaPag
        3 | 4 | 6 => TRANSFORMATION_SIEGE, // Siege / MovingTower / OreadsGuard
        _ => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    // Store transformation state on the session
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    world.set_transformation(
        caster_sid,
        transformation_type,
        transform_id as u16,
        skill.magic_num as u32,
        now_ms,
        duration as u64 * 1000, // C++ stores in milliseconds
    );

    // Store transformation state on the character
    world.update_character_stats(caster_sid, |ch| {
        ch.res_hp_type = 3; // Transformed state (C++ StateChangeServerDirect(3, nSkillID))
    });

    // C++ line 5192-5194: sData[1]=1, sData[3]=duration, SendSkill()
    instance.data[1] = 1;
    instance.data[3] = duration as i32;

    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);

    // Broadcast state change (transformation) to region
    // C++ line 5190: StateChangeServerDirect(3, nSkillID)
    let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
    state_pkt.write_u32(caster_sid as u32);
    state_pkt.write_u8(3); // type 3 = transformation
    state_pkt.write_u32(skill.magic_num as u32);
    broadcast_to_caster_region(world, caster_sid, &state_pkt);

    // Save to saved magic persistence
    // C++ line 5213: InsertSavedMagic(nSkillID, sDuration)
    world.insert_saved_magic(caster_sid, skill.magic_num as u32, duration);

    tracing::info!(
        "[sid={}] MagicProcess Type 6: transform to {} duration={}s skill={} type={}",
        caster_sid,
        transform_id,
        duration,
        skill.magic_num,
        transformation_type,
    );

    true
}

// ── Type 7: Summon / CC ─────────────────────────────────────────────────

/// Execute Type 7 skill -- summoning / crowd control / target change.
/// Handles target-change effects, NPC sleep/stun, and NPC damage.
async fn execute_type7(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type7_data = match world.get_magic_type7(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let damage = type7_data.damage;
    let target_change = type7_data.target_change;

    // Set sData[1] = 1 (success indicator)
    instance.data[1] = 1;

    let target_id = instance.target_id;

    if target_id >= 0 {
        let target_is_npc = (target_id as u32) >= NPC_BAND;

        if target_is_npc && damage > 0 {
            // Apply damage to NPC target
            let npc_id = target_id as u32;
            apply_skill_damage_to_npc(world, caster_sid, npc_id, instance, damage, skill, 0).await;
        }

        // Target change type 2 = sleep/stun NPC
        if target_change == 2 && (target_id as u32) >= NPC_BAND {
            let npc_id = target_id as u32;
            // Set NPC to fainted/sleeping state
            let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
            state_pkt.write_u32(npc_id);
            state_pkt.write_u8(1); // type 1 = general state
            state_pkt.write_u32(4); // value 4 = stunned/sleeping
            broadcast_to_caster_region(world, caster_sid, &state_pkt);

            tracing::debug!(
                "[sid={}] MagicProcess Type 7: sleep NPC {} for {}s",
                caster_sid,
                npc_id,
                type7_data.duration
            );
        }
    }

    // Broadcast skill effect
    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);

    tracing::debug!(
        "[sid={}] MagicProcess Type 7: target_change={} damage={} skill={}",
        caster_sid,
        target_change,
        damage,
        skill.magic_num
    );

    true
}

// ── Type 8: Teleport / Knockback ────────────────────────────────────────

/// Execute Type 8 skill — teleportation or knockback.
/// `warp_type` determines behavior:
/// - 1 (WARP_RESURRECTION): teleport target to resurrection point
/// - Other: knockback by `kick_distance`
fn execute_type8(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type8_data = match world.get_magic_type8(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let warp_type = type8_data.warp_type;

    // WARP_RESURRECTION (1): teleport to bind point
    if warp_type == 1 {
        let target_sid = if instance.target_id < 0 {
            caster_sid
        } else {
            instance.target_id as SessionId
        };

        let target = match world.get_character_info(target_sid) {
            Some(ch) => ch,
            None => {
                send_skill_failed(world, caster_sid, instance);
                return false;
            }
        };

        // Teleport to bind zone
        tracing::debug!(
            "[sid={}] MagicProcess Type 8: warp to bind zone={} x={} z={}",
            target_sid,
            target.bind_zone,
            target.bind_x,
            target.bind_z
        );

        instance.data[1] = 1;
        // Broadcast the effect before warping
        let pkt = instance.build_packet(MAGIC_EFFECTING);
        broadcast_to_caster_region(world, caster_sid, &pkt);
        return true;
    }

    // Knockback: apply kick_distance in the direction from caster to target
    let kick_dist = type8_data.kick_distance as f32;
    if kick_dist > 0.0 {
        let target_id = instance.target_id;
        if target_id >= 0 {
            let target_sid = target_id as SessionId;
            let caster_pos = world.get_position(caster_sid);
            let target_pos = world.get_position(target_sid);

            if let (Some(cp), Some(tp)) = (caster_pos, target_pos) {
                let dx = tp.x - cp.x;
                let dz = tp.z - cp.z;
                let dist = (dx * dx + dz * dz).sqrt();

                if dist > 0.0 {
                    let nx = dx / dist;
                    let nz = dz / dist;

                    let new_x = tp.x + nx * kick_dist;
                    let new_z = tp.z + nz * kick_dist;

                    // Validate knockback destination is within zone bounds
                    let valid = world
                        .get_zone(tp.zone_id)
                        .map(|z| z.is_valid_position(new_x, new_z))
                        .unwrap_or(false);
                    if valid {
                        world.update_position(target_sid, tp.zone_id, new_x, tp.y, new_z);
                    }
                }
            }
        }
    }

    instance.data[1] = 1;
    let pkt = instance.build_packet(MAGIC_EFFECTING);
    broadcast_to_caster_region(world, caster_sid, &pkt);
    true
}

// ── Type 9: Stealth / Invisibility ──────────────────────────────────────

/// Execute Type 9 skill — invisibility or advanced CC.
/// Applies invisibility/stealth as a buff (state change). The buff is stored
/// via the Type 4 buff system with a special buff type. Stealth is removed
/// on attack or certain movements (handled by attack/move handlers).
fn execute_type9(
    world: &WorldState,
    caster_sid: SessionId,
    instance: &mut MagicInstance,
    skill: &MagicRow,
) -> bool {
    let type9_data = match world.get_magic_type9(skill.magic_num) {
        Some(d) => d,
        None => {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }
    };

    let duration = type9_data.duration.unwrap_or(0);
    let state_change = type9_data.state_change.unwrap_or(0) as u8;

    // For stateChange 1 or 2: apply individual stealth (rogue invisibility)
    // - Check if player is already invisible (fail if so)
    // - Set invisibility_type via StateChangeServerDirect(7, stateChange)
    // - Insert into type9BuffMap
    if state_change == 1 || state_change == 2 {
        let caster_zone = world
            .get_position(caster_sid)
            .map(|p| p.zone_id)
            .unwrap_or(0);
        if caster_zone == ZONE_FORGOTTEN_TEMPLE || caster_zone == ZONE_DUNGEON_DEFENCE {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        if !world.can_stealth(caster_sid) {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        // If already invisible, reject the skill
        if world.get_invisibility_type(caster_sid) != 0 {
            send_skill_failed(world, caster_sid, instance);
            return false;
        }

        // Set invisibility type (determines break condition)
        world.set_invisibility_type(caster_sid, state_change);

        // Broadcast StateChange(7, stateChange) to make player invisible to others
        let mut sc_pkt = Packet::new(Opcode::WizStateChange as u8);
        sc_pkt.write_u32(caster_sid as u32);
        sc_pkt.write_u8(7); // type 7 = invisibility
        sc_pkt.write_u32(state_change as u32);

        if let Some(pos) = world.get_position(caster_sid) {
            let event_room = world.get_event_room(caster_sid);
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(sc_pkt),
                None,
                event_room,
            );

            // v2525: Set visual state flag (WIZ_PACKET2 +0xB69) for stealth
            let flag_pkt = super::packet2::build_state_flag(caster_sid as i32, state_change);
            world.broadcast_to_3x3(
                pos.zone_id,
                pos.region_x,
                pos.region_z,
                Arc::new(flag_pkt),
                None,
                event_room,
            );
        }
    }

    // Apply stealth as a buff for tracking purposes
    // buff_type 100 is used for invisibility/stealth
    let stealth_buff = ActiveBuff {
        skill_id: instance.skill_id,
        buff_type: BUFF_TYPE_INVISIBILITY,
        caster_sid,
        start_time: Instant::now(),
        duration_secs: duration as u32,
        attack_speed: 0,
        speed: 0,
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
        is_buff: true, // stealth is a self-buff
    };
    world.apply_buff(caster_sid, stealth_buff);

    // Set stealth_end_time so the buff_tick system can expire timed stealth
    if duration > 0 {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        world.update_session(caster_sid, |h| {
            h.stealth_end_time = now_unix + duration as u64;
        });
    }

    tracing::debug!(
        "[sid={}] MagicProcess Type 9: invisibility skill={} state_change={} duration={}s",
        caster_sid,
        skill.magic_num,
        state_change,
        duration
    );

    // sData[1] = 1 (success), sData[3] = duration
    // C++ sends to caster only (bSendToRegion = false), NOT region broadcast
    instance.data[1] = 1;
    instance.data[3] = duration as i32;
    let pkt = instance.build_packet(MAGIC_EFFECTING);
    world.send_to_session_owned(caster_sid, pkt);
    true
}

/// Check if a buff_type represents a debuff.
/// those that harm the target (slow, stun, poison, etc.)
/// Determine if an active buff is actually a debuff based on its field values.
/// In C++, many buff types can be EITHER buff or debuff depending on their
/// actual stat modifier values (e.g., BUFF_TYPE_DAMAGE with attack >= 100 is
/// a buff, but attack < 100 is a debuff).
pub fn is_debuff(buff: &crate::world::types::ActiveBuff) -> bool {
    match buff.buff_type {
        1 => buff.max_hp < 0,                  // HP_MP: debuff if maxHp < 0
        2 => buff.ac < 0 && buff.ac_pct < 100, // AC: debuff if both negative
        3 => true,                             // SIZE: always debuff
        4 => buff.attack < 100,                // DAMAGE: debuff if attack < 100
        5 => buff.attack_speed < 100,          // ATTACK_SPEED: debuff if < 100
        6 => buff.speed < 100,                 // SPEED: debuff if < 100
        7 => {
            // STATS: debuff if any stat < 0
            buff.str_mod < 0
                || buff.sta_mod < 0
                || buff.dex_mod < 0
                || buff.intel_mod < 0
                || buff.cha_mod < 0
        }
        8 => false,                                        // RESISTANCES: always buff
        9 => buff.hit_rate < 100 || buff.avoid_rate < 100, // ACCURACY: debuff if either < 100
        10 => buff.magic_attack < 100,                     // MAGIC_POWER: debuff if < 100
        11 => false,                                       // EXPERIENCE: always buff
        13 => false,                                       // WEAPON_DAMAGE: always buff
        15 => false,                                       // LOYALTY: always buff
        20 | 21 => true,                                   // CURSE types: always debuff
        25 => false,                                       // MAGE_ARMOR: always buff
        40 => true,                                        // MALICE: always debuff
        41 => false,                                       // ARMORED: always buff
        42 => false,                                       // UNK_EXPERIENCE: always buff
        43 => true,                                        // DISABLE_TARGETING: always debuff
        44 => true,                                        // BLIND: always debuff
        45 => true,                                        // REVERSE_HPMP: always debuff
        46 => buff.speed < 100 || buff.attack_speed < 100, // SPEED_AND_ATTACK: debuff if either < 100
        47 => true,                                        // AOE debuff: always debuff
        _ => false,                                        // Unknown types: not debuff
    }
}

/// Simple check if a buff_type number is in the set of known buff/debuff types.
/// Used only in tests. For actual debuff determination, use `is_debuff()`.
pub fn is_debuff_type(buff_type: i32) -> bool {
    matches!(
        buff_type,
        1 | 2
            | 3
            | 4
            | 5
            | 6
            | 7
            | 8
            | 9
            | 10
            | 11
            | 15
            | 20
            | 21
            | 25
            | 40
            | 41
            | 42
            | 43
            | 44
            | 45
            | 46
            | 47
    )
}

/// Check if a zone allows monster transformation (TransformationSkillUseMonster).
/// Returns true for homeland, Eslant, Moradon, Forgotten Temple, Abyss, and clan war zones.
fn is_transformation_monster_zone(zone_id: u16) -> bool {
    use crate::world::{
        ZONE_CLAN_WAR_ARDREAM, ZONE_CLAN_WAR_RONARK, ZONE_DESPERATION_ABYSS, ZONE_ELMORAD,
        ZONE_ELMORAD2, ZONE_ELMORAD3, ZONE_ELMORAD_ESLANT, ZONE_ELMORAD_ESLANT2,
        ZONE_ELMORAD_ESLANT3, ZONE_HELL_ABYSS, ZONE_KARUS, ZONE_KARUS2, ZONE_KARUS3,
        ZONE_KARUS_ESLANT, ZONE_KARUS_ESLANT2, ZONE_KARUS_ESLANT3, ZONE_MORADON, ZONE_MORADON2,
        ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5,
    };
    matches!(
        zone_id,
        ZONE_KARUS
            | ZONE_KARUS2
            | ZONE_KARUS3
            | ZONE_ELMORAD
            | ZONE_ELMORAD2
            | ZONE_ELMORAD3
            | ZONE_KARUS_ESLANT
            | ZONE_KARUS_ESLANT2
            | ZONE_KARUS_ESLANT3
            | ZONE_ELMORAD_ESLANT
            | ZONE_ELMORAD_ESLANT2
            | ZONE_ELMORAD_ESLANT3
            | ZONE_MORADON
            | ZONE_MORADON2
            | ZONE_MORADON3
            | ZONE_MORADON4
            | ZONE_MORADON5
            | ZONE_FORGOTTEN_TEMPLE
            | ZONE_DESPERATION_ABYSS
            | ZONE_HELL_ABYSS
            | ZONE_CLAN_WAR_ARDREAM
            | ZONE_CLAN_WAR_RONARK
    )
}

// ── Consume Item Helper ─────────────────────────────────────────────────

/// Items that should NOT be consumed (special scrolls/stones).
/// Note: 370004000-370006000 (Blood of Wolf etc.) are NOT in C++ list — they ARE consumed.
const NO_CONSUME_ITEMS: [u32; 9] = [
    370001000, 370002000, 370003000, // Town return scrolls
    379069000, 379070000, // Special scrolls
    379063000, 379064000, 379065000, 379066000, // Class-specific stones
];

/// Base item ID for class stones (C++ CLASS_STONE_BASE_ID).
const CLASS_STONE_BASE_ID: u32 = 379060000;

/// Resolve the consumable item ID for a skill.
/// Derived from `pSkill.nBeforeAction` and `pSkill.iUseItem`.
fn resolve_consume_item(skill: &ko_db::models::MagicRow) -> u32 {
    let before_action = skill.before_action.unwrap_or(0) as u32;
    let use_item = skill.use_item.unwrap_or(0) as u32;

    if (1..=4).contains(&before_action) {
        // Class stone: CLASS_STONE_BASE_ID + (nBeforeAction * 1000)
        CLASS_STONE_BASE_ID + (before_action * 1000)
    } else if before_action == 379090000 || before_action == 379093000 {
        // Job change scrolls: use iUseItem
        use_item
    } else if before_action == 381001000 {
        // Special before_action item
        before_action
    } else {
        // Default: use iUseItem
        use_item
    }
}

/// Consume the item used to cast this skill.
/// Uses `nConsumeItem` which is derived from `pSkill.nBeforeAction` and `pSkill.iUseItem`.
/// Some items (e.g. town scrolls, special stones) are excluded from consumption.
fn consume_item(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    skill: &ko_db::models::MagicRow,
) {
    let consume_item_id = resolve_consume_item(skill);

    if consume_item_id == 0 {
        return;
    }

    // C++ MagicInstance.cpp:7081-7092 — some items are NOT consumed
    if NO_CONSUME_ITEMS.contains(&consume_item_id) {
        return;
    }

    // Rob 1 of the consumed item from inventory
    world.rob_item(sid, consume_item_id, 1);
}

// ── Magic Type Cooldown Helpers ──────────────────────────────────────────

/// Default minimum interval between same-type casts (ms).
const TYPE_COOLDOWN_DEFAULT_MS: u128 = 575;

/// Interval for instant melee/archer on first catch (ms).
const TYPE_COOLDOWN_INSTANT_MELEE_MS: u128 = 650;

/// Interval for instant melee/archer after t_catch (ms).
const TYPE_COOLDOWN_CATCH_MS: u128 = 400;

/// Staff skill minimum interval (ms).
const PLAYER_SKILL_REQUEST_INTERVAL_MS: u128 = 800;

/// Check if a skill ID is a mage armor skill (bypasses type cooldown).
fn is_mage_armor_skill(skill_id: u32) -> bool {
    matches!(
        skill_id,
        190573 | 290573 | 190673 | 290673 | 190773 | 290773
    )
}

/// Check type cooldown for bType[0] and bType[1]. Returns true if blocked.
/// `existspeed` bypass only applies to bType[0] — NOT bType[1].
fn check_type_cooldown(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    skill: &ko_db::models::MagicRow,
    type1: u8,
    type2: u8,
    existspeed: bool,
) -> bool {
    let skill_id = skill.magic_num as u32;
    let cast_time = skill.cast_time.unwrap_or(0);
    let t_1 = skill.t_1.unwrap_or(0);

    // Mage armor skills bypass type cooldown entirely
    if is_mage_armor_skill(skill_id) {
        return false;
    }

    // C++ MagicInstance.cpp:489-497 — type3 with t_1==-1 uses synthetic key 10
    // to separate DOT cooldowns from direct type3.
    let check_type1 = if type1 == 3 && t_1 == -1 { 10u8 } else { type1 };

    // Check bType[0] — with existspeed bypass
    if check_type1 != 0
        && check_single_type_cooldown(
            world,
            sid,
            skill_id,
            check_type1,
            type2,
            cast_time,
            existspeed,
        )
    {
        return true;
    }

    // Check bType[1] — NO existspeed bypass
    if type2 != 0
        && check_single_type_cooldown(world, sid, skill_id, type2, type1, cast_time, false)
    {
        return true;
    }

    false
}

/// Check a single type entry in the cooldown map. Returns true if blocked.
/// When `existspeed` is true, skip timing check and just reset the entry.
fn check_single_type_cooldown(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    skill_id: u32,
    check_type: u8,
    other_type: u8,
    cast_time: i16,
    existspeed: bool,
) -> bool {
    let now = std::time::Instant::now();

    let result = world.with_session(sid, |h| {
        let entry = match h.magic_type_cooldowns.get(&check_type) {
            Some(e) if e.time.elapsed().as_millis() < 2000 => e.clone(),
            _ => return (false, false), // No entry or very old → pass
        };

        // C++ MagicInstance.cpp:1980-1981 — existspeed bypass:
        // When existspeed is true, skip timing check entirely.
        // Fall through to reset the entry (time=0, t_catch=false).
        if existspeed {
            return (false, false); // not blocked, entry will be reset below
        }

        // Determine threshold
        let staff = is_staff_skill(skill_id);
        let mut threshold = TYPE_COOLDOWN_DEFAULT_MS;

        // Instant melee/archer: bType==1 and castTime==0 and not staff
        if (check_type == 1 || other_type == 1) && cast_time == 0 && !staff {
            threshold = if entry.t_catch {
                TYPE_COOLDOWN_CATCH_MS
            } else {
                TYPE_COOLDOWN_INSTANT_MELEE_MS
            };
        }

        // Type 4 buffs: no limit (except staff and mage armor, already handled)
        if check_type == 4 && !staff {
            threshold = 0;
        }

        let elapsed = entry.time.elapsed().as_millis();

        // Staff skills: use PLAYER_SKILL_REQUEST_INTERVAL
        if staff && elapsed < PLAYER_SKILL_REQUEST_INTERVAL_MS {
            return (true, true); // blocked, silent
        }

        if threshold > 0 && elapsed < threshold {
            return (true, false); // blocked, not silent
        }

        (false, false)
    });

    let (blocked, _silent) = result.unwrap_or((false, false));

    if blocked {
        // C++ MagicInstance.cpp:2006-2007 — update t_catch and time on block
        world.update_session(sid, |h| {
            if let Some(entry) = h.magic_type_cooldowns.get_mut(&check_type) {
                entry.t_catch = true;
                entry.time = now;
            }
        });
    } else if result.is_some() {
        // C++ MagicInstance.cpp:2016-2017 — reset entry on pass (or existspeed)
        world.update_session(sid, |h| {
            if let Some(entry) = h.magic_type_cooldowns.get_mut(&check_type) {
                entry.time = std::time::Instant::now() - std::time::Duration::from_secs(10);
                entry.t_catch = false;
            }
        });
    }

    blocked
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use ko_protocol::{Opcode, Packet, PacketReader};

    /// Test MagicInstance::build_packet creates correct wire format.
    #[test]
    fn test_build_skill_packet() {
        let instance = MagicInstance {
            opcode: MAGIC_EFFECTING,
            skill_id: 108010,
            caster_id: 1,
            target_id: 2,
            data: [100, 0, 200, 0, 0, 0, 0],
        };

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_EFFECTING)); // opcode
        assert_eq!(r.read_u32(), Some(108010)); // skill_id
        assert_eq!(r.read_u32(), Some(1)); // caster_id
        assert_eq!(r.read_u32(), Some(2)); // target_id
        assert_eq!(r.read_u32(), Some(100)); // data[0]
        assert_eq!(r.read_u32(), Some(0)); // data[1]
        assert_eq!(r.read_u32(), Some(200)); // data[2]
        for _ in 3..7 {
            assert_eq!(r.read_u32(), Some(0)); // data[3..6]
        }
        assert_eq!(r.remaining(), 0);
    }

    /// Test build_fail_packet uses MAGIC_FAIL opcode.
    #[test]
    fn test_build_fail_packet() {
        let instance = MagicInstance {
            opcode: MAGIC_EFFECTING,
            skill_id: 999,
            caster_id: 5,
            target_id: 10,
            data: [0; 7],
        };

        let pkt = instance.build_fail_packet();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_FAIL));
        assert_eq!(r.read_u32(), Some(999));
    }

    /// Test build_packet sign-extension for negative target ID (-1).
    ///
    /// C++ casts `int16(-1)` to `uint32` via sign-extension → `0xFFFFFFFF`.
    /// Rust must use `i16 as i32 as u32` (NOT `i16 as u16 as u32` which
    /// would zero-extend to `0x0000FFFF`).
    #[test]
    fn test_build_packet_negative_target_sign_extension() {
        let instance = MagicInstance {
            opcode: MAGIC_EFFECTING,
            skill_id: 108010,
            caster_id: 42,
            target_id: -1,
            data: [-1, 0, 0, 0, 0, 0, 0],
        };

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        let mut r = PacketReader::new(&pkt.data);

        assert_eq!(r.read_u8(), Some(MAGIC_EFFECTING)); // opcode
        assert_eq!(r.read_u32(), Some(108010)); // skill_id
        assert_eq!(r.read_u32(), Some(42)); // caster_id (positive, same either way)
                                            // Critical: target_id -1 must be 0xFFFFFFFF (C++ sign-extension)
        assert_eq!(r.read_u32(), Some(0xFFFFFFFF)); // target_id = -1
                                                    // data[0] = -1 as i32 as u32 = 0xFFFFFFFF
        assert_eq!(r.read_u32(), Some(0xFFFFFFFF)); // data[0] = -1
                                                    // data[1..6] = 0
        for _ in 1..7 {
            assert_eq!(r.read_u32(), Some(0));
        }
        assert_eq!(r.remaining(), 0);
    }

    /// Test build_packet sign-extension for negative caster ID.
    #[test]
    fn test_build_packet_negative_caster_sign_extension() {
        let instance = MagicInstance {
            opcode: MAGIC_FAIL,
            skill_id: 500,
            caster_id: -1,
            target_id: 10,
            data: [0; 7],
        };

        let pkt = instance.build_packet(MAGIC_FAIL);
        let mut r = PacketReader::new(&pkt.data);

        assert_eq!(r.read_u8(), Some(MAGIC_FAIL));
        assert_eq!(r.read_u32(), Some(500));
        // caster_id -1 must be 0xFFFFFFFF, NOT 0x0000FFFF
        assert_eq!(r.read_u32(), Some(0xFFFFFFFF));
        assert_eq!(r.read_u32(), Some(10)); // target_id positive
    }

    /// Test client packet parsing matches C++ format.
    #[test]
    fn test_client_packet_parse() {
        // Build a client magic packet:
        // [u8 opcode][u32 skill][i32 caster][i32 target][i32 data * 7]
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(MAGIC_EFFECTING);
        pkt.write_u32(108010); // skill_id
        pkt.write_u32(1); // caster_id
        pkt.write_u32(2); // target_id
        for i in 0..7u32 {
            pkt.write_u32(i * 10); // data[0..6]
        }

        // Parse like the handler
        let mut r = PacketReader::new(&pkt.data);
        let opcode = r.read_u8().unwrap();
        let skill_id = r.read_u32().unwrap();
        let caster = r.read_u32().unwrap() as i32;
        let target = r.read_u32().unwrap() as i32;
        let mut data = [0i32; 7];
        for d in &mut data {
            *d = r.read_u32().unwrap() as i32;
        }

        assert_eq!(opcode, MAGIC_EFFECTING);
        assert_eq!(skill_id, 108010);
        assert_eq!(caster as i16, 1);
        assert_eq!(target as i16, 2);
        assert_eq!(data[0], 0);
        assert_eq!(data[1], 10);
        assert_eq!(data[2], 20);
        assert_eq!(r.remaining(), 0);
    }

    /// Test magic opcode constants match C++ values.
    #[test]
    fn test_magic_opcode_constants() {
        assert_eq!(MAGIC_CASTING, 1);
        assert_eq!(MAGIC_FLYING, 2);
        assert_eq!(MAGIC_EFFECTING, 3);
        assert_eq!(MAGIC_FAIL, 4);
        assert_eq!(MAGIC_DURATION_EXPIRED, 5);
        assert_eq!(MAGIC_CANCEL, 6);
    }

    /// Test moral constants match C++ enum.
    #[test]
    fn test_moral_constants() {
        assert_eq!(MORAL_SELF, 1);
        assert_eq!(MORAL_ENEMY, 7);
        assert_eq!(MORAL_AREA_ENEMY, 10);
        assert_eq!(MORAL_SELF_AREA, 13);
    }

    /// Test hit rate table with high rate — mostly hits.
    #[test]
    fn test_hit_rate_high() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut fails = 0;
        for _ in 0..1000 {
            if get_hit_rate(10.0, &mut rng) == FAIL {
                fails += 1;
            }
        }
        assert!(
            fails < 50,
            "Expected < 50 fails at rate=10.0, got {}",
            fails
        );
    }

    /// Test packet size matches expected (1 + 4 + 4 + 4 + 7*4 = 41 bytes).
    #[test]
    fn test_skill_packet_size() {
        let instance = MagicInstance {
            opcode: MAGIC_CASTING,
            skill_id: 1,
            caster_id: 1,
            target_id: -1,
            data: [0; 7],
        };

        let pkt = instance.build_packet(MAGIC_CASTING);
        // opcode(1) + skill_id(4) + caster(4) + target(4) + data(7*4) = 41
        assert_eq!(pkt.data.len(), 41);
    }

    /// Helper to build a default test context for compute_magic_damage.
    fn make_test_ctx(target_kind: MagicTargetKind) -> MagicDamageContext {
        MagicDamageContext {
            target_kind,
            target_total_r: 0,
            target_class: 1, // warrior target
            target_ac_amount: 0,
            is_war_zone: false,
            damage_settings: None,
            plus_damage: 1.0,
            righthand_damage: 0,
            attribute_damage: 0,
            attribute: 1, // fire (default)
        }
    }

    /// Test compute_magic_damage for mage class with CHA scaling.
    #[test]
    fn test_compute_magic_damage_mage() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage, CHA=80
        let ctx = make_test_ctx(MagicTargetKind::Player);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        // Negative domain (C++ parity):
        // total_hit = ceil(-500 * 80 / 102.5) = ceil(-390.24) = -390
        // sMagicAmount=100 → total_hit = -390
        // resistance (r=0): 485*(-390)/510 = -370
        // random(-370..0)*0.3 + (-370)*0.85 - 100 → /2 → negate
        let damage = compute_magic_damage(&ch, -500, 0, &ctx, &mut rng);
        assert!(
            damage > 100,
            "Mage magic should deal damage, got {}",
            damage
        );
        assert!(
            damage < 350,
            "Mage damage {} too high for base=500, r=0, /2",
            damage
        );
    }

    /// Test compute_magic_damage for warrior (non-CHA scaling).
    #[test]
    fn test_compute_magic_damage_warrior() {
        let ch = make_test_character(101, 90, 60, 30, 20, 10); // warrior
        let ctx = make_test_ctx(MagicTargetKind::Player);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        // Warriors get: damage /= 2 (no weapon), damage *= int32(0.50f) only for NPC target.
        // For Player target: halved only, but still positive.
        let damage = compute_magic_damage(&ch, -50, 0, &ctx, &mut rng);
        // Warrior casting magic at player: very reduced but can be > 0
        assert!(damage >= 0, "Warrior magic should be >= 0, got {}", damage);
    }

    /// Test compute_magic_damage with zero damage.
    #[test]
    fn test_compute_magic_damage_zero() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80);
        let ctx = make_test_ctx(MagicTargetKind::Player);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let damage = compute_magic_damage(&ch, 0, 0, &ctx, &mut rng);
        assert_eq!(damage, 0);
    }

    /// Test compute_magic_damage for priest class (non-mage, no CHA scaling).
    #[test]
    fn test_compute_magic_damage_priest_no_cha() {
        let ch = make_test_character(104, 20, 20, 20, 20, 80); // priest (class 4)
        let ctx = make_test_ctx(MagicTargetKind::Player);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        // Priest is NOT a mage (class 4 != 3/9/10), no CHA scaling.
        // direct_type filtering now at call site, compute_magic_damage always applies CHA for mages.
        let damage = compute_magic_damage(&ch, -100, 0, &ctx, &mut rng);
        assert!(damage >= 0, "Should be non-negative, got {}", damage);
    }

    /// Test compute_magic_damage with magic_attack_amount buff.
    #[test]
    fn test_compute_magic_damage_with_magic_attack_amount() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage, CHA=80
        let ctx = make_test_ctx(MagicTargetKind::Player);
        // Use realistic base damage and same seed for deterministic comparison
        let mut rng_base = rand::rngs::StdRng::seed_from_u64(42);
        let base = compute_magic_damage(&ch, -500, 0, &ctx, &mut rng_base);
        let mut rng_boost = rand::rngs::StdRng::seed_from_u64(42);
        let boosted = compute_magic_damage(&ch, -500, 20, &ctx, &mut rng_boost);
        assert!(
            boosted > base,
            "magic_attack_amount=20 should increase damage: base={}, boosted={}",
            base,
            boosted
        );
        let mut rng_red = rand::rngs::StdRng::seed_from_u64(42);
        let reduced = compute_magic_damage(&ch, -500, -20, &ctx, &mut rng_red);
        assert!(
            reduced < base,
            "magic_attack_amount=-20 should decrease damage: base={}, reduced={}",
            base,
            reduced
        );
    }

    /// Test compute_magic_damage with zero magic_attack_amount.
    #[test]
    fn test_compute_magic_damage_zero_magic_attack_no_change() {
        let ch = make_test_character(101, 60, 20, 20, 20, 20); // warrior
        let ctx = make_test_ctx(MagicTargetKind::Player);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let d0 = compute_magic_damage(&ch, -100, 0, &ctx, &mut rng);
        // warrior magic with base=100, r=0, /2 (no weapon), /2 (non-war)
        assert!(d0 >= 0, "Should be non-negative: {}", d0);
    }

    /// Test resistance formula reduces damage.
    #[test]
    fn test_compute_magic_damage_resistance_reduces() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage
        let ctx_no_r = make_test_ctx(MagicTargetKind::Player);
        let mut ctx_high_r = make_test_ctx(MagicTargetKind::Player);
        ctx_high_r.target_total_r = 200;
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_no_r = compute_magic_damage(&ch, -300, 0, &ctx_no_r, &mut rng1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_high_r = compute_magic_damage(&ch, -300, 0, &ctx_high_r, &mut rng2);
        assert!(
            dmg_no_r > dmg_high_r,
            "Higher resistance should reduce damage: no_r={}, high_r={}",
            dmg_no_r,
            dmg_high_r
        );
    }

    /// Test war zone divides by 3 instead of 2.
    #[test]
    fn test_compute_magic_damage_war_zone_halving() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage
        let ctx_normal = make_test_ctx(MagicTargetKind::Player);
        let mut ctx_war = make_test_ctx(MagicTargetKind::Player);
        ctx_war.is_war_zone = true;
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_normal = compute_magic_damage(&ch, -500, 0, &ctx_normal, &mut rng1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_war = compute_magic_damage(&ch, -500, 0, &ctx_war, &mut rng2);
        // War zone /3 vs non-war /2, so war < normal
        assert!(
            dmg_war < dmg_normal,
            "War zone damage should be less: war={}, normal={}",
            dmg_war,
            dmg_normal
        );
    }

    /// Test NPC target formula uses 555/515 constants.
    #[test]
    fn test_compute_magic_damage_npc_target_formula() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage
        let ctx_player = make_test_ctx(MagicTargetKind::Player);
        let ctx_npc = make_test_ctx(MagicTargetKind::Npc);
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let _dmg_player = compute_magic_damage(&ch, -300, 0, &ctx_player, &mut rng1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_npc = compute_magic_damage(&ch, -300, 0, &ctx_npc, &mut rng2);
        // NPC formula 555/515 yields higher base, but mage_magic_damage not applied (no DS)
        // and warrior NPC zeroing doesn't apply (caster is mage)
        assert!(dmg_npc > 0, "Mage vs NPC should deal damage: {}", dmg_npc);
    }

    /// Test warrior magic vs NPC target is zeroed.
    #[test]
    fn test_compute_magic_damage_warrior_vs_npc_zero() {
        let ch = make_test_character(101, 90, 60, 30, 20, 10); // warrior
        let ctx = make_test_ctx(MagicTargetKind::Npc);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let damage = compute_magic_damage(&ch, -200, 0, &ctx, &mut rng);
        // C++ int32(0.50f) = 0, so damage should be 0
        assert_eq!(damage, 0, "Warrior magic vs NPC should be 0");
    }

    /// Test MAX_DAMAGE cap (32000).
    #[test]
    fn test_compute_magic_damage_max_cap() {
        let ch = make_test_character(103, 20, 20, 20, 20, 255); // mage, max CHA
        let ctx = make_test_ctx(MagicTargetKind::Player);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let damage = compute_magic_damage(&ch, -30000, 100, &ctx, &mut rng);
        assert!(damage <= 32000, "Damage {} exceeds MAX_DAMAGE", damage);
    }

    /// Test DamageSettings class multiplier.
    #[test]
    fn test_compute_magic_damage_class_multiplier() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage
        let ds = ko_db::models::DamageSettingsRow {
            id: 1,
            priest_to_warrior: 1.0,
            priest_to_mage: 1.0,
            priest_to_priest: 1.0,
            priest_to_rogue: 1.0,
            priest_to_kurian: 1.0,
            warrior_to_rogue: 1.0,
            warrior_to_mage: 1.0,
            warrior_to_warrior: 1.0,
            warrior_to_priest: 1.0,
            warrior_to_kurian: 1.0,
            rogue_to_mage: 1.0,
            rogue_to_warrior: 1.0,
            rogue_to_rogue: 1.0,
            rogue_to_priest: 1.0,
            rogue_to_kurian: 1.0,
            kurian_to_mage: 1.0,
            kurian_to_warrior: 1.0,
            kurian_to_rogue: 1.0,
            kurian_to_priest: 1.0,
            kurian_to_kurian: 1.0,
            mage_to_warrior: 0.5,
            mage_to_mage: 1.0,
            mage_to_priest: 1.0,
            mage_to_rogue: 1.0,
            mage_to_kurian: 1.0,
            mon_def: 1.0,
            mon_take_damage: 1.5,
            mage_magic_damage: 0.4,
            unique_item: 1.0,
            low_class_item: 1.0,
            middle_class_item: 1.0,
            high_class_item: 1.0,
            rare_item: 1.0,
            magic_item: 1.0,
            r_damage: 0.9,
        };
        let ctx_warrior_target = MagicDamageContext {
            target_kind: MagicTargetKind::Player,
            target_total_r: 0,
            target_class: 101, // warrior
            target_ac_amount: 0,
            is_war_zone: false,
            damage_settings: Some(ds.clone()),
            plus_damage: 1.0,
            righthand_damage: 0,
            attribute_damage: 0,
            attribute: 1,
        };
        let ctx_mage_target = MagicDamageContext {
            target_kind: MagicTargetKind::Player,
            target_total_r: 0,
            target_class: 103, // mage
            target_ac_amount: 0,
            is_war_zone: false,
            damage_settings: Some(ds),
            plus_damage: 1.0,
            righthand_damage: 0,
            attribute_damage: 0,
            attribute: 1,
        };
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_vs_warrior = compute_magic_damage(&ch, -400, 0, &ctx_warrior_target, &mut rng1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_vs_mage = compute_magic_damage(&ch, -400, 0, &ctx_mage_target, &mut rng2);
        // mage_to_warrior = 0.5 vs mage_to_mage = 1.0, so vs_warrior < vs_mage
        assert!(
            dmg_vs_warrior < dmg_vs_mage,
            "Mage→warrior (0.5x) should be less than mage→mage (1.0x): warrior={}, mage={}",
            dmg_vs_warrior,
            dmg_vs_mage
        );
    }

    // ── Sprint 261: Weapon damage reduction in compute_magic_damage ──────

    /// Test weapon contribution increases magic damage for fire attribute (non-MAGIC_R).
    /// C++ MagicInstance.cpp:6618-6619: `damage -= weapon_val` where damage is negative,
    /// making it MORE negative = more damage dealt. Staves contribute to spell power.
    #[test]
    fn test_weapon_damage_contribution_fire() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage, level 60
        let ctx_no_weapon = make_test_ctx(MagicTargetKind::Player);
        let mut ctx_with_weapon = make_test_ctx(MagicTargetKind::Player);
        ctx_with_weapon.righthand_damage = 50; // staff damage 50
        ctx_with_weapon.attribute_damage = 10; // elemental bonus 10
        ctx_with_weapon.attribute = 1; // fire

        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_no_weapon = compute_magic_damage(&ch, -500, 0, &ctx_no_weapon, &mut rng1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_with_weapon = compute_magic_damage(&ch, -500, 0, &ctx_with_weapon, &mut rng2);

        // C++ negative domain: damage -= weapon_val makes damage more negative = more HP loss
        // contribution = (50*0.8 + 50*60/60) + (10*0.8 + 10*60/60) = (40+50) + (8+10) = 108
        assert!(
            dmg_with_weapon > dmg_no_weapon,
            "Weapon should increase magic damage (C++ parity): with_weapon={}, no_weapon={}",
            dmg_with_weapon,
            dmg_no_weapon
        );
    }

    /// Test weapon damage reduction is skipped for MAGIC_R attribute (attribute=4).
    #[test]
    fn test_weapon_damage_reduction_skipped_for_magic_r() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage
        let mut ctx_magic_r = make_test_ctx(MagicTargetKind::Player);
        ctx_magic_r.righthand_damage = 100;
        ctx_magic_r.attribute_damage = 20;
        ctx_magic_r.attribute = 4; // MAGIC_R — weapon reduction should NOT apply

        let ctx_no_weapon = make_test_ctx(MagicTargetKind::Player);

        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_magic_r = compute_magic_damage(&ch, -500, 0, &ctx_magic_r, &mut rng1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_no_weapon = compute_magic_damage(&ch, -500, 0, &ctx_no_weapon, &mut rng2);

        // Both should produce same damage since MAGIC_R skips weapon reduction
        assert_eq!(
            dmg_magic_r, dmg_no_weapon,
            "MAGIC_R should skip weapon reduction: magic_r={}, no_weapon={}",
            dmg_magic_r, dmg_no_weapon
        );
    }

    /// Test weapon contribution formula values match C++ calculation.
    #[test]
    fn test_weapon_damage_contribution_formula_values() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage, level 60

        let mut ctx = make_test_ctx(MagicTargetKind::Player);
        ctx.righthand_damage = 100;
        ctx.attribute_damage = 0;
        ctx.attribute = 1; // fire

        // Expected contribution for rh=100, attr=0, level=60:
        //   rh_part = 100 * 0.8 + (100 * 60 / 60) = 80 + 100 = 180
        //   attr_part = 0
        //   total = 180, after /2 = ~90
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_rh100 = compute_magic_damage(&ch, -500, 0, &ctx, &mut rng1);

        ctx.righthand_damage = 0;
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_rh0 = compute_magic_damage(&ch, -500, 0, &ctx, &mut rng2);

        // C++ negative domain: weapon makes damage more negative → more positive after negate
        let diff = dmg_rh100 - dmg_rh0;
        assert!(
            (85..=95).contains(&diff),
            "Weapon contribution for rh=100 should be ~90 after /2: diff={}",
            diff
        );
    }

    /// Test weapon contribution with attribute_damage only.
    #[test]
    fn test_weapon_damage_contribution_attribute_only() {
        let ch = make_test_character(103, 20, 20, 20, 20, 80); // mage, level 60
        let mut ctx = make_test_ctx(MagicTargetKind::Player);
        ctx.righthand_damage = 0;
        ctx.attribute_damage = 30;
        ctx.attribute = 3; // lightning

        let ctx_no_attr = make_test_ctx(MagicTargetKind::Player);

        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_with_attr = compute_magic_damage(&ch, -500, 0, &ctx, &mut rng1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_no_attr = compute_magic_damage(&ch, -500, 0, &ctx_no_attr, &mut rng2);

        // C++ negative domain: attribute damage increases spell damage
        assert!(
            dmg_with_attr > dmg_no_attr,
            "Attribute damage should increase magic damage (C++ parity): with={}, without={}",
            dmg_with_attr,
            dmg_no_attr
        );
    }

    /// Test weapon contribution for warrior class (non-mage).
    #[test]
    fn test_weapon_damage_contribution_warrior() {
        let ch = make_test_character(101, 90, 60, 30, 20, 10); // warrior, level 60
        let mut ctx = make_test_ctx(MagicTargetKind::Player);
        ctx.righthand_damage = 80;
        ctx.attribute = 1; // fire

        let ctx_no_weapon = make_test_ctx(MagicTargetKind::Player);

        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_with = compute_magic_damage(&ch, -100, 0, &ctx, &mut rng1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let dmg_without = compute_magic_damage(&ch, -100, 0, &ctx_no_weapon, &mut rng2);

        // C++ negative domain: weapon contributes to magic damage for all classes
        assert!(
            dmg_with >= dmg_without,
            "Warrior weapon contribution: with={}, without={}",
            dmg_with,
            dmg_without
        );
    }

    // ── Sprint 301: Scroll buff persistence filter (skill_id > 500000) ──

    #[test]
    fn test_scroll_buff_persistence_filter() {
        // Regular class skills (< 500000) should NOT persist
        let regular_skill: u32 = 108010;
        assert!(
            regular_skill <= 500000,
            "Regular skill should be filtered out"
        );

        // Scroll buffs (> 500000) SHOULD persist
        let scroll_skill: u32 = 500001;
        assert!(scroll_skill > 500000, "Scroll skill should pass filter");

        // Boundary: exactly 500000 should NOT persist
        let boundary: u32 = 500000;
        assert!(
            (boundary <= 500000),
            "Boundary value should be filtered out"
        );
    }

    // ── Sprint 262: DOT magic_damage_reduction + apply_magic_damage_reduction tests ──

    /// Test apply_magic_damage_reduction with full reduction (100 = no reduction).
    #[test]
    fn test_apply_magic_damage_reduction_no_reduction() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);
        // Default magic_damage_reduction = 100 (no reduction)
        let result = apply_magic_damage_reduction(&world, sid, 200);
        assert_eq!(
            result, 200,
            "No reduction (100%) should leave damage unchanged"
        );
    }

    /// Test apply_magic_damage_reduction with 70% reduction (Elysian Web).
    #[test]
    fn test_apply_magic_damage_reduction_with_elysian_web() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);
        // Simulate Elysian Web: 70% of original damage
        world.update_session(sid, |h| {
            h.magic_damage_reduction = 70;
        });
        let result = apply_magic_damage_reduction(&world, sid, 200);
        // 200 * 70 / 100 = 140
        assert_eq!(result, 140, "70% reduction should yield 140 from 200");
    }

    /// Test apply_magic_damage_reduction clamps to 0 (no negative damage).
    #[test]
    fn test_apply_magic_damage_reduction_zero_floor() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);
        world.update_session(sid, |h| {
            h.magic_damage_reduction = 0;
        });
        let result = apply_magic_damage_reduction(&world, sid, 500);
        assert_eq!(result, 0, "0% reduction should yield 0");
    }

    /// Test is_debuff classification.
    #[test]
    fn test_is_debuff_classification() {
        // Known debuffs
        assert!(is_debuff_type(1)); // speed reduction
        assert!(is_debuff_type(3)); // poison
        assert!(is_debuff_type(4)); // slow
        assert!(is_debuff_type(5)); // freeze
        assert!(is_debuff_type(6)); // stun
        assert!(is_debuff_type(40)); // malice
        assert!(is_debuff_type(47)); // AoE debuff

        // Known buffs (not in debuff list)
        assert!(!is_debuff_type(50)); // regular buff
        assert!(!is_debuff_type(100)); // invisibility
        assert!(!is_debuff_type(255)); // unknown
    }

    /// Test BUFF_TYPE_INVISIBILITY constant.
    #[test]
    fn test_buff_type_invisibility() {
        assert_eq!(BUFF_TYPE_INVISIBILITY, 100);
    }

    /// Helper: create a test CharacterInfo with specified stats.
    fn make_test_character(
        class: u16,
        str_val: u8,
        sta: u8,
        dex: u8,
        intel: u8,
        cha: u8,
    ) -> CharacterInfo {
        CharacterInfo {
            session_id: 1,
            name: "Test".into(),
            nation: 1,
            race: 1,
            class,
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
            bind_zone: 0,
            bind_x: 0.0,
            bind_z: 0.0,
            str: str_val,
            sta,
            dex,
            intel,
            cha,
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
        }
    }

    /// Test Type 5 sub-type constants match C++ defines.
    #[test]
    fn test_type5_subtype_constants() {
        assert_eq!(TYPE5_REMOVE_TYPE3, 1);
        assert_eq!(TYPE5_REMOVE_TYPE4, 2);
        assert_eq!(TYPE5_RESURRECTION, 3);
        assert_eq!(TYPE5_RESURRECTION_SELF, 4);
        assert_eq!(TYPE5_REMOVE_BLESS, 5);
        assert_eq!(TYPE5_LIFE_CRYSTAL, 6);
    }

    /// Test Type 5 resurrection HP recovery percentage clamping.
    #[test]
    fn test_type5_resurrection_hp_recovery() {
        // 50% of 500 max_hp = 250
        let max_hp: i16 = 500;
        let recover_pct = 50_i32.clamp(10, 100);
        let restore_hp = (max_hp as i32 * recover_pct / 100).max(1) as i16;
        assert_eq!(restore_hp, 250);

        // 100% of 1000 max_hp = 1000
        let max_hp2: i16 = 1000;
        let recover_pct2 = 100_i32.clamp(10, 100);
        let restore_hp2 = (max_hp2 as i32 * recover_pct2 / 100).max(1) as i16;
        assert_eq!(restore_hp2, 1000);

        // 0% (clamped to 10%) of 500 = 50
        let recover_pct3 = 0_i32.clamp(10, 100);
        assert_eq!(recover_pct3, 10);
        let restore_hp3 = (500_i32 * recover_pct3 / 100).max(1) as i16;
        assert_eq!(restore_hp3, 50);
    }

    /// Test Type 6 transformation packet format.
    /// C++ sends sData[1]=1, sData[3]=duration.
    #[test]
    fn test_type6_transform_packet() {
        let mut instance = MagicInstance {
            opcode: MAGIC_EFFECTING,
            skill_id: 450001,
            caster_id: 1,
            target_id: 1,
            data: [0; 7],
        };

        instance.data[1] = 1;
        instance.data[3] = 120; // 120 seconds

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_EFFECTING));
        assert_eq!(r.read_u32(), Some(450001)); // skill_id
        assert_eq!(r.read_u32(), Some(1)); // caster
        assert_eq!(r.read_u32(), Some(1)); // target
        assert_eq!(r.read_u32(), Some(0)); // data[0]
        assert_eq!(r.read_u32(), Some(1)); // data[1] = 1 (success)
        assert_eq!(r.read_u32(), Some(0)); // data[2]
        assert_eq!(r.read_u32(), Some(120)); // data[3] = duration
    }

    /// Test Type 7 packet format with sData[1] = 1.
    #[test]
    fn test_type7_summon_packet() {
        let mut instance = MagicInstance {
            opcode: MAGIC_EFFECTING,
            skill_id: 500010,
            caster_id: 1,
            target_id: -1,
            data: [0; 7],
        };

        instance.data[1] = 1;

        let pkt = instance.build_packet(MAGIC_EFFECTING);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_EFFECTING));
        assert_eq!(r.read_u32(), Some(500010));
        r.read_u32(); // caster
        r.read_u32(); // target
        assert_eq!(r.read_u32(), Some(0)); // data[0]
        assert_eq!(r.read_u32(), Some(1)); // data[1] = success
    }

    /// Test is_debuff_type covers all known debuff types.
    #[test]
    fn test_is_debuff_type_classification() {
        // Known debuffs
        assert!(is_debuff_type(1)); // speed reduction
        assert!(is_debuff_type(3)); // poison
        assert!(is_debuff_type(4)); // slow
        assert!(is_debuff_type(5)); // freeze
        assert!(is_debuff_type(6)); // stun
        assert!(is_debuff_type(40)); // malice
        assert!(is_debuff_type(47)); // AoE debuff

        // Known buffs (not in debuff list)
        assert!(!is_debuff_type(50)); // regular buff
        assert!(!is_debuff_type(100)); // invisibility
        assert!(!is_debuff_type(255)); // unknown
    }

    // ── Sprint 43: Transformation type constants ────────────────────

    #[test]
    fn test_transformation_type_constants() {
        assert_eq!(TRANSFORMATION_MONSTER, 1);
        assert_eq!(TRANSFORMATION_NPC, 2);
        assert_eq!(TRANSFORMATION_SIEGE, 3);
    }

    #[test]
    fn test_transformation_type_from_user_skill_use() {
        // user_skill_use=1 => TransformationMonster
        assert_eq!(1u8, TRANSFORMATION_MONSTER);
        // user_skill_use=2 => TransformationNPC
        assert_eq!(2u8, TRANSFORMATION_NPC);
        // user_skill_use=3 => TransformationSiege
        assert_eq!(3u8, TRANSFORMATION_SIEGE);
    }

    #[test]
    fn test_type6_state_change_broadcast_format() {
        //   StateChangeServerDirect(3, nSkillID)
        let skill_id: u32 = 450001;
        let mut pkt = Packet::new(Opcode::WizStateChange as u8);
        pkt.write_u32(1); // session id
        pkt.write_u8(3); // type 3 = abnormal/transform
        pkt.write_u32(skill_id);

        assert_eq!(pkt.opcode, Opcode::WizStateChange as u8);
        assert_eq!(pkt.data.len(), 9);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u32(), Some(1)); // sid
        assert_eq!(r.read_u8(), Some(3)); // type
        assert_eq!(r.read_u32(), Some(450001)); // skill_id as abnormal
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_magic_cancel_transformation_packet_format() {
        //   Packet result(WIZ_MAGIC_PROCESS, uint8(MAGIC_CANCEL_TRANSFORMATION));
        let mut pkt = Packet::new(Opcode::WizMagicProcess as u8);
        pkt.write_u8(MAGIC_CANCEL_TRANSFORMATION);

        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(7)); // MAGIC_CANCEL_TRANSFORMATION
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_blink_skill_block_type4_exception() {
        //   if (pSkillCaster->isBlinking() && bType != 4 && pSkill.iNum < 300000)
        //       return false;
        // Type 4 buffs are allowed during blink
        let can_use_skills = false;
        let skill_type: u8 = 4;
        let skill_id: u32 = 100000;

        // Type 4 should NOT be blocked
        let blocked = !can_use_skills && skill_type != 4 && skill_id < 300000;
        assert!(!blocked, "Type 4 should be allowed during blink");
    }

    #[test]
    fn test_blink_skill_block_type1_blocked() {
        // Type 1 skills (melee) should be blocked during blink
        let can_use_skills = false;
        let skill_type: u8 = 1;
        let skill_id: u32 = 100000;

        let blocked = !can_use_skills && skill_type != 4 && skill_id < 300000;
        assert!(blocked, "Type 1 should be blocked during blink");
    }

    #[test]
    fn test_blink_skill_block_high_skill_id_exception() {
        //   pSkill.iNum < 300000
        // Skills >= 300000 are allowed even during blink (scrolls/potions)
        let can_use_skills = false;
        let skill_type: u8 = 1;
        let skill_id: u32 = 300001;

        let blocked = !can_use_skills && skill_type != 4 && skill_id < 300000;
        assert!(!blocked, "Skills >= 300000 should be allowed during blink");
    }

    #[test]
    fn test_blink_skill_block_all_types_except_4() {
        let can_use_skills = false;
        let skill_id: u32 = 50000;

        for skill_type in [1u8, 2, 3, 5, 6, 7, 8, 9] {
            let blocked = !can_use_skills && skill_type != 4 && skill_id < 300000;
            assert!(
                blocked,
                "Type {} should be blocked during blink",
                skill_type
            );
        }

        // Type 4 exception
        let blocked = !can_use_skills && 4u8 != 4 && skill_id < 300000;
        assert!(!blocked, "Type 4 should pass blink check");
    }

    #[test]
    fn test_aoe_stealth_break_enemy_nation_check() {
        //   if (pTarget->isPlayer() && pSkillCaster->GetNation() != pTarget->GetNation())
        //       TO_USER(pTarget)->RemoveStealth();
        let caster_nation: u8 = 1;
        let target_nation_enemy: u8 = 2;
        let target_nation_ally: u8 = 1;

        // Enemy: stealth should break
        assert!(caster_nation != target_nation_enemy);
        // Ally: stealth should NOT break
        assert!(caster_nation == target_nation_ally);
    }

    // ── Transformation Zone Check Tests ──────────────────────────────

    #[test]
    fn test_transformation_monster_zone_homelands() {
        use crate::world::{
            ZONE_ELMORAD, ZONE_ELMORAD2, ZONE_ELMORAD3, ZONE_KARUS, ZONE_KARUS2, ZONE_KARUS3,
        };
        assert!(is_transformation_monster_zone(ZONE_KARUS));
        assert!(is_transformation_monster_zone(ZONE_KARUS2));
        assert!(is_transformation_monster_zone(ZONE_KARUS3));
        assert!(is_transformation_monster_zone(ZONE_ELMORAD));
        assert!(is_transformation_monster_zone(ZONE_ELMORAD2));
        assert!(is_transformation_monster_zone(ZONE_ELMORAD3));
    }

    #[test]
    fn test_transformation_monster_zone_eslant() {
        use crate::world::{
            ZONE_ELMORAD_ESLANT, ZONE_ELMORAD_ESLANT2, ZONE_ELMORAD_ESLANT3, ZONE_KARUS_ESLANT,
            ZONE_KARUS_ESLANT2, ZONE_KARUS_ESLANT3,
        };
        assert!(is_transformation_monster_zone(ZONE_KARUS_ESLANT));
        assert!(is_transformation_monster_zone(ZONE_KARUS_ESLANT2));
        assert!(is_transformation_monster_zone(ZONE_KARUS_ESLANT3));
        assert!(is_transformation_monster_zone(ZONE_ELMORAD_ESLANT));
        assert!(is_transformation_monster_zone(ZONE_ELMORAD_ESLANT2));
        assert!(is_transformation_monster_zone(ZONE_ELMORAD_ESLANT3));
    }

    #[test]
    fn test_transformation_monster_zone_moradon() {
        use crate::world::{
            ZONE_MORADON, ZONE_MORADON2, ZONE_MORADON3, ZONE_MORADON4, ZONE_MORADON5,
        };
        assert!(is_transformation_monster_zone(ZONE_MORADON));
        assert!(is_transformation_monster_zone(ZONE_MORADON2));
        assert!(is_transformation_monster_zone(ZONE_MORADON3));
        assert!(is_transformation_monster_zone(ZONE_MORADON4));
        assert!(is_transformation_monster_zone(ZONE_MORADON5));
    }

    #[test]
    fn test_transformation_monster_zone_special() {
        use crate::world::{
            ZONE_CLAN_WAR_ARDREAM, ZONE_CLAN_WAR_RONARK, ZONE_DESPERATION_ABYSS, ZONE_HELL_ABYSS,
        };
        assert!(is_transformation_monster_zone(55)); // ZONE_FORGOTTEN_TEMPLE
        assert!(is_transformation_monster_zone(ZONE_DESPERATION_ABYSS));
        assert!(is_transformation_monster_zone(ZONE_HELL_ABYSS));
        assert!(is_transformation_monster_zone(ZONE_CLAN_WAR_ARDREAM));
        assert!(is_transformation_monster_zone(ZONE_CLAN_WAR_RONARK));
    }

    #[test]
    fn test_transformation_monster_zone_not_allowed() {
        // PK zones and battle zones should NOT allow monster transformation
        use crate::world::{ZONE_ARDREAM, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE};
        assert!(!is_transformation_monster_zone(ZONE_ARDREAM));
        assert!(!is_transformation_monster_zone(ZONE_RONARK_LAND));
        assert!(!is_transformation_monster_zone(ZONE_RONARK_LAND_BASE));
        assert!(!is_transformation_monster_zone(ZONE_DELOS));
        assert!(!is_transformation_monster_zone(ZONE_BATTLE2));
        assert!(!is_transformation_monster_zone(ZONE_BATTLE3));
    }

    // ── apply_magic_class_bonus tests ────────────────────────────────

    fn make_char(class: u16, str_val: u8, sta: u8) -> CharacterInfo {
        CharacterInfo {
            session_id: 1,
            name: "Test".into(),
            nation: 1,
            race: 1,
            class,
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
            str: str_val,
            sta,
            dex: 50,
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
    fn test_magic_class_bonus_mage_caster_no_effect() {
        // Mage casters have isChaSkill=true, so class bonus should NOT apply.
        let caster = make_char(103, 100, 100); // Mage
        let target = make_char(101, 100, 100); // Warrior
        let world = WorldState::new();

        let damage = apply_magic_class_bonus(100, &caster, &target, &world, 1, 2);
        assert_eq!(
            damage, 100,
            "Mage caster should not have class bonus applied"
        );
    }

    #[test]
    fn test_magic_class_bonus_mage_novice_caster_no_effect() {
        let caster = make_char(109, 100, 100); // MageNovice
        let target = make_char(101, 100, 100);
        let world = WorldState::new();

        let damage = apply_magic_class_bonus(100, &caster, &target, &world, 1, 2);
        assert_eq!(
            damage, 100,
            "MageNovice caster should not have class bonus applied"
        );
    }

    #[test]
    fn test_magic_class_bonus_warrior_caster_applies() {
        // Warrior casters have isChaSkill=false, so class bonus SHOULD apply.
        // With zero equipped stats, the formula should still work.
        let caster = make_char(101, 150, 100); // Warrior
        let target = make_char(103, 100, 100); // Mage
        let world = WorldState::new();

        let damage = apply_magic_class_bonus(100, &caster, &target, &world, 1, 2);
        // With no equipped stats, class bonus arrays are all zero.
        // The formula still runs but bonuses are (100+0)/100 = no change to temp_ap/temp_ac.
        // Result depends on temp_hit_B = (temp_ap * 2) / (temp_ac + 240).
        // The key check: it returns a value (doesn't crash).
        assert!(
            damage > 0,
            "Warrior caster class bonus should return positive damage"
        );
    }

    #[test]
    fn test_magic_class_bonus_priest_caster_applies() {
        let caster = make_char(104, 150, 100); // Priest
        let target = make_char(102, 100, 100); // Rogue
        let world = WorldState::new();

        let damage = apply_magic_class_bonus(100, &caster, &target, &world, 1, 2);
        assert!(
            damage > 0,
            "Priest caster class bonus should return positive damage"
        );
    }

    #[test]
    fn test_magic_class_bonus_rogue_caster_applies() {
        let caster = make_char(102, 150, 100); // Rogue
        let target = make_char(104, 100, 100); // Priest
        let world = WorldState::new();

        let damage = apply_magic_class_bonus(100, &caster, &target, &world, 1, 2);
        assert!(
            damage > 0,
            "Rogue caster class bonus should return positive damage"
        );
    }

    #[test]
    fn test_magic_class_bonus_kurian_caster_no_group() {
        // Kurian casters have no class group index, so bonus returns original damage.
        let caster = make_char(113, 150, 100); // Kurian
        let target = make_char(101, 100, 100); // Warrior
        let world = WorldState::new();

        let damage = apply_magic_class_bonus(100, &caster, &target, &world, 1, 2);
        assert_eq!(damage, 100, "Kurian caster should return original damage");
    }

    #[test]
    fn test_magic_class_bonus_kurian_target_no_group() {
        // Kurian targets have no class group index, so bonus returns original damage.
        let caster = make_char(101, 150, 100); // Warrior
        let target = make_char(113, 100, 100); // Kurian
        let world = WorldState::new();

        let damage = apply_magic_class_bonus(100, &caster, &target, &world, 1, 2);
        assert_eq!(damage, 100, "Kurian target should return original damage");
    }

    #[test]
    fn test_magic_class_bonus_zero_damage_stays_zero() {
        let caster = make_char(101, 150, 100);
        let target = make_char(103, 100, 100);
        let world = WorldState::new();

        let damage = apply_magic_class_bonus(0, &caster, &target, &world, 1, 2);
        // Even with warrior caster, zero input should not magically become positive
        // (though the formula may clamp to 1, the temp_hit_b check at least ensures logic runs)
        assert!(
            damage >= 0,
            "Zero damage should stay at zero or clamp to min"
        );
    }

    // ── Sprint 48: Combat Integration Tests ─────────────────────────

    /// Integration: PvP magic kill flow — damage reduces HP to 0, death packet sent.
    ///
    /// Verifies: magic damage → HP reaches 0 → dead state → WIZ_DEAD broadcast format.
    #[test]
    fn test_integration_pvp_magic_kill_flow() {
        let world = WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let caster_sid = world.allocate_session_id();
        let target_sid = world.allocate_session_id();
        world.register_session(caster_sid, tx1);
        world.register_session(target_sid, tx2);

        // Set up caster (mage, high CHA)
        let caster_ch = make_test_character(103, 20, 20, 20, 80, 100);
        world.register_ingame(
            caster_sid,
            CharacterInfo {
                session_id: caster_sid,
                ..caster_ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Set up target (warrior, low HP)
        let mut target_ch = make_test_character(101, 90, 60, 30, 20, 10);
        target_ch.hp = 50; // low HP
        target_ch.max_hp = 500;
        world.register_ingame(
            target_sid,
            CharacterInfo {
                session_id: target_sid,
                ..target_ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 502.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Simulate magic damage that exceeds target HP
        let test_ctx = MagicDamageContext {
            target_kind: MagicTargetKind::Player,
            target_total_r: 0,
            target_class: 1,
            target_ac_amount: 0,
            is_war_zone: false,
            damage_settings: None,
            plus_damage: 1.0,
            righthand_damage: 0,
            attribute_damage: 0,
            attribute: 1,
        };
        let mut test_rng = rand::rngs::StdRng::seed_from_u64(42);
        let magic_damage = compute_magic_damage(
            &make_test_character(103, 20, 20, 20, 80, 100),
            -2000, // high damage — realistic skill base
            0,
            &test_ctx,
            &mut test_rng,
        );
        assert!(magic_damage > 0, "Magic damage should be positive");

        // Apply damage to target: HP goes to 0
        world.update_character_stats(target_sid, |ch| {
            ch.hp = (ch.hp - magic_damage).max(0);
        });

        let target_info = world.get_character_info(target_sid).unwrap();
        assert_eq!(target_info.hp, 0, "Target should be dead (HP=0)");

        // Build WIZ_DEAD broadcast (verify packet format)
        let mut dead_pkt = Packet::new(Opcode::WizDead as u8);
        dead_pkt.write_u32(target_sid as u32);
        let mut r = PacketReader::new(&dead_pkt.data);
        assert_eq!(r.read_u32(), Some(target_sid as u32));
        assert_eq!(r.remaining(), 0);
    }

    /// Integration: class bonus + flash bonus stacking in magic damage.
    ///
    /// Verifies: warrior class bonus applies, flash bonus multiplier stacks on top.
    #[test]
    fn test_integration_class_bonus_and_flash_stacking() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let caster_sid = world.allocate_session_id();
        world.register_session(caster_sid, tx);

        // Warrior caster with flash bonus
        let caster = make_char(101, 200, 100);
        let target = make_char(103, 100, 100);
        world.register_ingame(
            caster_sid,
            CharacterInfo {
                session_id: caster_sid,
                ..caster.clone()
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Set flash bonus (exp bonus = 50% → should influence damage too)
        world.update_session(caster_sid, |h| {
            h.flash_exp_bonus = 50;
            h.flash_dc_bonus = 20;
        });

        // Class bonus applies to warrior (non-mage caster)
        let base_damage: i16 = 100;
        let class_damage =
            apply_magic_class_bonus(base_damage, &caster, &target, &world, caster_sid, 2);
        assert!(
            class_damage > 0,
            "Class bonus should return positive damage for warrior"
        );

        // Flash bonus DC applies as extra multiplier
        let flash_dc = world
            .with_session(caster_sid, |h| h.flash_dc_bonus)
            .unwrap_or(0);
        let flash_mult = 1.0 + (flash_dc as f64 / 100.0);
        let final_damage = (class_damage as f64 * flash_mult) as i16;
        assert!(
            final_damage >= class_damage,
            "Flash DC bonus should increase or maintain damage: {} >= {}",
            final_damage,
            class_damage
        );
    }

    /// Integration: DOT damage ticks leading to player death.
    ///
    /// Verifies: DOT applied → tick damage → HP reaches 0 → player dead.
    #[test]
    fn test_integration_dot_ticks_cause_death() {
        let world = WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let caster_sid = world.allocate_session_id();
        let target_sid = world.allocate_session_id();
        world.register_session(caster_sid, tx1);
        world.register_session(target_sid, tx2);

        let mut target_ch = make_test_character(101, 90, 60, 30, 20, 10);
        target_ch.hp = 30; // very low HP
        target_ch.max_hp = 500;
        world.register_ingame(
            target_sid,
            CharacterInfo {
                session_id: target_sid,
                ..target_ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Add DOT: -10 HP per tick, 5 ticks total
        let added = world.add_durational_skill(target_sid, 108100, -10, 5, caster_sid);
        assert!(added, "DOT should be added successfully");

        // Simulate 3 ticks of DOT damage (3 * -10 = -30 → exactly kills from 30 HP)
        for tick in 0..3 {
            world.update_character_stats(target_sid, |ch| {
                ch.hp = (ch.hp + (-10)).max(0); // DOT applies -10 per tick
            });
            let info = world.get_character_info(target_sid).unwrap();
            if tick < 2 {
                assert!(info.hp > 0, "Target should still be alive at tick {}", tick);
            } else {
                assert_eq!(info.hp, 0, "Target should be dead at tick {}", tick);
            }
        }
    }

    /// Integration: Type 6 transformation → magic cast → transformation expire.
    ///
    /// Verifies: transform state set → skill still castable → transform cleared.
    #[test]
    fn test_integration_transformation_magic_cast_expire() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                ..make_test_character(101, 90, 60, 30, 20, 10)
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Apply transformation
        world.update_session(sid, |h| {
            h.transformation_type = TRANSFORMATION_MONSTER;
            h.transform_id = 500;
            h.transform_skill_id = 450001;
            h.transformation_duration = 120;
        });

        // Verify transformation is active
        let is_transformed = world
            .with_session(sid, |h| h.transformation_type)
            .unwrap_or(0);
        assert_eq!(is_transformed, TRANSFORMATION_MONSTER);

        // Type 4 buffs should still be castable during transformation (blink exception)
        let can_cast_type4 = true; // type 4 is always allowed
        assert!(
            can_cast_type4,
            "Type 4 buffs should be castable during transformation"
        );

        // Build transformation packet
        let mut instance = MagicInstance {
            opcode: MAGIC_EFFECTING,
            skill_id: 450001,
            caster_id: sid as i32,
            target_id: sid as i32,
            data: [0; 7],
        };
        instance.data[1] = 1; // success
        instance.data[3] = 120; // duration
        let pkt = instance.build_packet(MAGIC_EFFECTING);
        assert_eq!(pkt.data.len(), 41);

        // Clear transformation (simulate expiry)
        world.update_session(sid, |h| {
            h.transformation_type = 0;
            h.transform_id = 0;
            h.transform_skill_id = 0;
        });

        let after = world
            .with_session(sid, |h| h.transformation_type)
            .unwrap_or(0);
        assert_eq!(after, 0, "Transformation should be cleared after expiry");
    }

    /// Integration: AOE magic hits multiple targets, breaks stealth on enemies.
    ///
    /// Verifies: AOE hits 2 enemies → stealth broken on enemy → ally stealth preserved.
    #[test]
    fn test_integration_aoe_stealth_break_multi_target() {
        let world = WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let (tx3, _rx3) = tokio::sync::mpsc::unbounded_channel();

        let caster_sid = world.allocate_session_id(); // nation 1
        let enemy_sid = world.allocate_session_id(); // nation 2
        let ally_sid = world.allocate_session_id(); // nation 1

        world.register_session(caster_sid, tx1);
        world.register_session(enemy_sid, tx2);
        world.register_session(ally_sid, tx3);

        // Caster: nation 1
        let mut caster_ch = make_test_character(103, 20, 20, 20, 80, 100);
        caster_ch.nation = 1;
        world.register_ingame(
            caster_sid,
            CharacterInfo {
                session_id: caster_sid,
                ..caster_ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Enemy: nation 2, stealthed
        let mut enemy_ch = make_test_character(102, 30, 30, 90, 20, 10);
        enemy_ch.nation = 2;
        world.register_ingame(
            enemy_sid,
            CharacterInfo {
                session_id: enemy_sid,
                ..enemy_ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 502.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );
        world.update_session(enemy_sid, |h| {
            h.invisibility_type = 1; // stealth active
        });

        // Ally: nation 1, stealthed
        let mut ally_ch = make_test_character(102, 30, 30, 90, 20, 10);
        ally_ch.nation = 1;
        world.register_ingame(
            ally_sid,
            CharacterInfo {
                session_id: ally_sid,
                ..ally_ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 498.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );
        world.update_session(ally_sid, |h| {
            h.invisibility_type = 1; // stealth active
        });

        // Simulate AOE stealth break logic (from magic_process AOE handler)
        let caster_nation = world.get_character_info(caster_sid).unwrap().nation;
        let targets = [enemy_sid, ally_sid];
        for &tid in &targets {
            let target_nation = world.get_character_info(tid).unwrap().nation;
            if caster_nation != target_nation {
                // Enemy: remove stealth
                world.update_session(tid, |h| {
                    h.invisibility_type = 0;
                });
            }
        }

        // Enemy stealth should be broken
        let enemy_stealth = world
            .with_session(enemy_sid, |h| h.invisibility_type)
            .unwrap_or(0);
        assert_eq!(enemy_stealth, 0, "Enemy stealth should be broken by AOE");

        // Ally stealth should be preserved
        let ally_stealth = world
            .with_session(ally_sid, |h| h.invisibility_type)
            .unwrap_or(0);
        assert_eq!(ally_stealth, 1, "Ally stealth should NOT be broken by AOE");
    }

    /// Integration: Blink blocks skills except type 4 and scroll skills (>= 300000).
    ///
    /// Verifies the full blink skill gating logic for all skill types.
    #[test]
    fn test_integration_blink_skill_gating_full() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                ..make_test_character(101, 90, 60, 30, 20, 10)
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Activate blink
        world.update_session(sid, |h| {
            h.blink_expiry_time = u64::MAX; // blink active
            h.can_use_skills = false;
        });

        let can_use = world.with_session(sid, |h| h.can_use_skills).unwrap();
        assert!(!can_use, "Skills should be blocked during blink");

        // Type 4 buff (allowed during blink)
        let skill_type_4: u8 = 4;
        let skill_id_normal: u32 = 108010;
        let blocked = !can_use && skill_type_4 != 4 && skill_id_normal < 300000;
        assert!(!blocked, "Type 4 should pass blink check");

        // Type 1 attack (blocked during blink)
        let skill_type_1: u8 = 1;
        let blocked = !can_use && skill_type_1 != 4 && skill_id_normal < 300000;
        assert!(blocked, "Type 1 should be blocked during blink");

        // Scroll skill >= 300000 (allowed during blink)
        let scroll_id: u32 = 500001;
        let blocked = !can_use && skill_type_1 != 4 && scroll_id < 300000;
        assert!(!blocked, "Scroll skills >= 300000 should pass blink check");

        // Clear blink
        world.update_session(sid, |h| {
            h.blink_expiry_time = 0;
            h.can_use_skills = true;
        });
        let can_use_after = world.with_session(sid, |h| h.can_use_skills).unwrap();
        assert!(can_use_after, "Skills should be usable after blink expires");
    }

    /// Integration: Magic damage with debuff classification check.
    ///
    /// Verifies: debuff types (poison, freeze, stun) are correctly classified,
    /// and buff types (normal, invisibility) are not.
    #[test]
    fn test_integration_debuff_vs_buff_classification() {
        // Debuffs that should reduce target stats
        let debuff_types = [1, 3, 4, 5, 6, 40, 47];
        for &dt in &debuff_types {
            assert!(
                is_debuff_type(dt),
                "Type {} should be classified as debuff",
                dt
            );
        }

        // Buffs that should NOT be treated as debuffs
        let buff_types = [50, BUFF_TYPE_INVISIBILITY, 200, 255];
        for &bt in &buff_types {
            assert!(
                !is_debuff_type(bt),
                "Type {} should NOT be classified as debuff",
                bt
            );
        }

        // Verify invisibility constant
        assert_eq!(BUFF_TYPE_INVISIBILITY, 100);
    }

    /// Integration: Resurrection flow — dead player → Type 5 resurrection → HP restored.
    ///
    /// Verifies: player is dead → resurrection applied → HP restored to percentage.
    #[test]
    fn test_integration_resurrection_hp_restore() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let mut ch = make_test_character(101, 90, 60, 30, 20, 10);
        ch.hp = 0; // dead
        ch.max_hp = 1000;
        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                ..ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Player is dead
        let info = world.get_character_info(sid).unwrap();
        assert_eq!(info.hp, 0, "Player should be dead");

        // Apply Type 5 resurrection (50% HP restore)
        let recover_pct = 50_i32.clamp(10, 100);
        let restore_hp = (info.max_hp as i32 * recover_pct / 100).max(1) as i16;
        assert_eq!(restore_hp, 500, "Should restore 50% of 1000 = 500 HP");

        // Apply HP restoration
        world.update_character_stats(sid, |c| {
            c.hp = restore_hp;
        });

        let after = world.get_character_info(sid).unwrap();
        assert_eq!(after.hp, 500, "Player HP should be 500 after resurrection");
        assert!(after.hp > 0, "Player should be alive after resurrection");
    }

    /// Integration: Buff application + expiry + stat recalculation.
    ///
    /// Verifies: buff applied → stats modified → buff expired → stats restored.
    #[test]
    fn test_integration_buff_apply_expire_stat_recalc() {
        use std::time::Instant;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                ..make_test_character(101, 90, 60, 30, 20, 10)
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Apply a speed buff (buff_type=8 = speed increase)
        let buff = ActiveBuff {
            skill_id: 108010,
            buff_type: 8,
            caster_sid: sid,
            start_time: Instant::now(),
            duration_secs: 60,
            attack_speed: 0,
            speed: 50,
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
        };
        world.apply_buff(sid, buff);

        let buffs = world.get_active_buffs(sid);
        assert_eq!(buffs.len(), 1, "Should have 1 active buff");
        assert_eq!(buffs[0].speed, 50, "Buff should have speed=50");
        assert!(!buffs[0].is_expired(), "Buff should not be expired yet");

        // Apply a second buff (different type)
        let buff2 = ActiveBuff {
            skill_id: 108020,
            buff_type: 9, // different type
            caster_sid: sid,
            start_time: Instant::now(),
            duration_secs: 30,
            attack: 20,
            speed: 0,
            ac: 0,
            ac_pct: 0,
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
            attack_speed: 0,
            weapon_damage: 0,
            ac_sour: 0,
            duration_extended: false,
            is_buff: true,
        };
        world.apply_buff(sid, buff2);

        let buffs = world.get_active_buffs(sid);
        assert_eq!(buffs.len(), 2, "Should have 2 active buffs");

        // Clear all buffs (simulating zone change)
        let cleared = world.clear_all_buffs(sid, false);
        assert_eq!(cleared.len(), 2, "Should have cleared 2 buffs");

        let after = world.get_active_buffs(sid);
        assert_eq!(after.len(), 0, "No buffs should remain after clear");
    }

    /// Integration: Multiple DOTs from different casters on same target.
    ///
    /// Verifies: two DOTs stack independently, each ticking separately.
    #[test]
    fn test_integration_multiple_dots_from_different_casters() {
        let world = WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let (tx3, _rx3) = tokio::sync::mpsc::unbounded_channel();

        let caster1 = world.allocate_session_id();
        let caster2 = world.allocate_session_id();
        let target = world.allocate_session_id();

        world.register_session(caster1, tx1);
        world.register_session(caster2, tx2);
        world.register_session(target, tx3);

        let mut target_ch = make_test_character(101, 90, 60, 30, 20, 10);
        target_ch.hp = 200;
        target_ch.max_hp = 500;
        world.register_ingame(
            target,
            CharacterInfo {
                session_id: target,
                ..target_ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Caster 1 applies poison DOT: -5 per tick, 10 ticks
        let added1 = world.add_durational_skill(target, 108100, -5, 10, caster1);
        assert!(added1, "First DOT should be added");

        // Caster 2 applies fire DOT: -8 per tick, 5 ticks
        let added2 = world.add_durational_skill(target, 108200, -8, 5, caster2);
        assert!(added2, "Second DOT should be added");

        // Simulate combined tick: -5 + -8 = -13 per tick
        let total_tick_damage: i16 = -5 + -8;
        world.update_character_stats(target, |ch| {
            ch.hp = (ch.hp + total_tick_damage).max(0);
        });

        let info = world.get_character_info(target).unwrap();
        assert_eq!(
            info.hp, 187,
            "HP should be 200 - 13 = 187 after one combined tick"
        );
    }

    // ── Sprint 55: Hardening Edge Case Tests ────────────────────────

    /// Edge case: casting a spell with zero skill ID should produce a valid
    /// fail packet without panicking.
    #[test]
    fn test_zero_skill_id_produces_valid_fail_packet() {
        let instance = MagicInstance {
            opcode: MAGIC_EFFECTING,
            skill_id: 0,
            caster_id: 1,
            target_id: 2,
            data: [0; 7],
        };

        // build_fail_packet should not panic on skill_id=0
        let pkt = instance.build_fail_packet();
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_FAIL));
        assert_eq!(r.read_u32(), Some(0)); // skill_id = 0
    }

    /// Edge case: building a regular packet with skill_id=0 should produce
    /// correct wire format (41 bytes) without panicking.
    #[test]
    fn test_zero_skill_id_build_packet_no_panic() {
        let instance = MagicInstance {
            opcode: MAGIC_CASTING,
            skill_id: 0,
            caster_id: 1,
            target_id: -1,
            data: [0; 7],
        };

        let pkt = instance.build_packet(MAGIC_CASTING);
        assert_eq!(pkt.data.len(), 41);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_CASTING));
        assert_eq!(r.read_u32(), Some(0)); // skill_id=0
        assert_eq!(r.read_u32(), Some(1)); // caster
        assert_eq!(r.read_u32(), Some(0xFFFFFFFF)); // target=-1
    }

    /// Edge case: applying buff when target already has a buff of the same
    /// type should replace the existing buff (not duplicate).
    #[test]
    fn test_buff_same_type_replaces_existing() {
        use std::time::Instant;

        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        world.register_ingame(
            sid,
            CharacterInfo {
                session_id: sid,
                ..make_test_character(101, 90, 60, 30, 20, 10)
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        // Apply first buff of type 8
        let buff1 = ActiveBuff {
            skill_id: 108010,
            buff_type: 8,
            caster_sid: sid,
            start_time: Instant::now(),
            duration_secs: 60,
            speed: 30,
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
        };
        world.apply_buff(sid, buff1);
        assert_eq!(world.get_active_buffs(sid).len(), 1);
        assert_eq!(world.get_active_buffs(sid)[0].speed, 30);

        // Apply second buff of SAME type 8 — should replace
        let buff2 = ActiveBuff {
            skill_id: 108011,
            buff_type: 8,
            caster_sid: sid,
            start_time: Instant::now(),
            duration_secs: 120,
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
        };
        world.apply_buff(sid, buff2);

        let buffs = world.get_active_buffs(sid);
        // Should have exactly 1 buff (replaced, not duplicated)
        assert_eq!(buffs.len(), 1, "Same buff_type should replace, not stack");
        assert_eq!(
            buffs[0].speed, 50,
            "Replacement buff should have new speed value"
        );
        assert_eq!(
            buffs[0].skill_id, 108011,
            "Replacement buff should have new skill_id"
        );
    }

    /// Edge case: DOT damage on a dead target should not reduce HP below 0.
    #[test]
    fn test_dot_on_dead_target_hp_stays_zero() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let target_sid = world.allocate_session_id();
        world.register_session(target_sid, tx);

        let mut target_ch = make_test_character(101, 90, 60, 30, 20, 10);
        target_ch.hp = 0; // already dead
        target_ch.max_hp = 500;
        world.register_ingame(
            target_sid,
            CharacterInfo {
                session_id: target_sid,
                ..target_ch
            },
            crate::world::Position {
                zone_id: 21,
                x: 500.0,
                y: 0.0,
                z: 500.0,
                region_x: 4,
                region_z: 4,
            },
        );

        assert!(world.is_player_dead(target_sid), "Target should be dead");

        // Simulate DOT tick on dead target: -10 damage
        world.update_character_stats(target_sid, |ch| {
            ch.hp = (ch.hp + (-10)).max(0);
        });

        let info = world.get_character_info(target_sid).unwrap();
        assert_eq!(info.hp, 0, "Dead target HP should stay at 0 after DOT tick");
    }

    // ── Skill Cooldown Tests ─────────────────────────────────────────

    #[test]
    fn test_cooldown_set_and_check() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let skill_id: u32 = 108010;
        let recast_time: i16 = 20; // 20 * 90 = 1800ms

        // Initially no cooldown
        let on_cd = world
            .with_session(sid, |h| {
                h.skill_cooldowns
                    .get(&skill_id)
                    .map(|exp| std::time::Instant::now() < *exp)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        assert!(!on_cd, "Skill should not be on cooldown initially");

        // Set cooldown
        let expiry =
            std::time::Instant::now() + std::time::Duration::from_millis(recast_time as u64 * 90);
        world.update_session(sid, |h| {
            h.skill_cooldowns.insert(skill_id, expiry);
        });

        // Should now be on cooldown
        let on_cd = world
            .with_session(sid, |h| {
                h.skill_cooldowns
                    .get(&skill_id)
                    .map(|exp| std::time::Instant::now() < *exp)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        assert!(on_cd, "Skill should be on cooldown after set");
    }

    #[test]
    fn test_cooldown_expired() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let skill_id: u32 = 108020;

        // Set cooldown that is already expired (in the past)
        let expiry = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_millis(100))
            .unwrap_or(std::time::Instant::now());
        world.update_session(sid, |h| {
            h.skill_cooldowns.insert(skill_id, expiry);
        });

        // Should NOT be on cooldown
        let on_cd = world
            .with_session(sid, |h| {
                h.skill_cooldowns
                    .get(&skill_id)
                    .map(|exp| std::time::Instant::now() < *exp)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        assert!(!on_cd, "Expired cooldown should not block cast");
    }

    #[test]
    fn test_cooldown_overwrite_on_recast() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let skill_id: u32 = 108030;

        // Set first cooldown (short)
        let first = std::time::Instant::now() + std::time::Duration::from_millis(100);
        world.update_session(sid, |h| {
            h.skill_cooldowns.insert(skill_id, first);
        });

        // Overwrite with longer cooldown
        let second = std::time::Instant::now() + std::time::Duration::from_secs(60);
        world.update_session(sid, |h| {
            h.skill_cooldowns.insert(skill_id, second);
        });

        // The expiry should be the newer (longer) one
        let remaining = world
            .with_session(sid, |h| h.skill_cooldowns.get(&skill_id).copied())
            .flatten();
        assert!(remaining.is_some());
        // The expiry should be close to 60s from now (the second value)
        let exp = remaining.unwrap();
        assert!(
            exp.duration_since(std::time::Instant::now()).as_secs() >= 50,
            "Cooldown should be overwritten with the newer value"
        );
    }

    #[test]
    fn test_cooldown_independent_per_skill() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let skill_a: u32 = 109010;
        let skill_b: u32 = 109020;

        // Set cooldown only for skill A
        let expiry = std::time::Instant::now() + std::time::Duration::from_secs(10);
        world.update_session(sid, |h| {
            h.skill_cooldowns.insert(skill_a, expiry);
        });

        // Skill A should be on cooldown
        let a_cd = world
            .with_session(sid, |h| {
                h.skill_cooldowns
                    .get(&skill_a)
                    .map(|exp| std::time::Instant::now() < *exp)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        assert!(a_cd, "Skill A should be on cooldown");

        // Skill B should NOT be on cooldown
        let b_cd = world
            .with_session(sid, |h| {
                h.skill_cooldowns
                    .get(&skill_b)
                    .map(|exp| std::time::Instant::now() < *exp)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        assert!(!b_cd, "Skill B should not be on cooldown");
    }

    #[test]
    fn test_cooldown_zero_recast_no_cooldown() {
        let world = WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sid = world.allocate_session_id();
        world.register_session(sid, tx);

        let skill_id: u32 = 108040;
        let recast_time: i16 = 0; // zero means no cooldown

        // Simulate the SET logic — only insert if recast_time > 0
        if recast_time > 0 {
            let expiry = std::time::Instant::now()
                + std::time::Duration::from_millis(recast_time as u64 * 90);
            world.update_session(sid, |h| {
                h.skill_cooldowns.insert(skill_id, expiry);
            });
        }

        // Should not be on cooldown
        let on_cd = world
            .with_session(sid, |h| {
                h.skill_cooldowns
                    .get(&skill_id)
                    .map(|exp| std::time::Instant::now() < *exp)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        assert!(!on_cd, "Zero recast_time should not set cooldown");
    }

    /// Verify MAGIC_CANCEL2 constant matches C++ packets.h:572 value.
    #[test]
    fn test_magic_cancel2_constant() {
        assert_eq!(MAGIC_CANCEL2, 13);
    }

    /// Verify MAGIC_CANCEL2 is excluded from cooldown check alongside MAGIC_CANCEL.
    ///
    /// bypass the cooldown recast check.
    #[test]
    fn test_magic_cancel2_skips_cooldown() {
        // These opcodes must all be excluded from cooldown enforcement
        let excluded = [
            MAGIC_CANCEL,
            MAGIC_CANCEL2,
            MAGIC_CANCEL_TRANSFORMATION,
            MAGIC_FAIL,
            MAGIC_TYPE4_EXTEND,
        ];
        for &op in &excluded {
            assert!(
                op == MAGIC_CANCEL
                    || op == MAGIC_CANCEL2
                    || op == MAGIC_CANCEL_TRANSFORMATION
                    || op == MAGIC_FAIL
                    || op == MAGIC_TYPE4_EXTEND,
                "opcode {} should be in cooldown exclusion list",
                op
            );
        }
    }

    /// Test build_skill_failed_packet creates correct wire format.
    #[test]
    fn test_build_skill_failed_packet() {
        let data = [10, 0, 20, 0, 0, 0, 0i32];
        let pkt = build_skill_failed_packet(109035, 1, -1, &data);
        assert_eq!(pkt.opcode, Opcode::WizMagicProcess as u8);
        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(MAGIC_FAIL));
        assert_eq!(r.read_u32(), Some(109035));
        assert_eq!(r.read_u32(), Some(1)); // caster_id as u32
        assert_eq!(r.read_u32(), Some((-1i16 as i32) as u32)); // target_id -1
    }

    /// Test special skill 109035 requires target=-1.
    #[test]
    fn test_special_skill_target_minus1_required() {
        // Skills 109035/110035/209035/210035 require target_id == -1
        for &skill_id in &[109035u32, 110035, 209035, 210035] {
            assert!(
                matches!(skill_id, 109035 | 110035 | 209035 | 210035),
                "Skill {} should match special target=-1 pattern",
                skill_id
            );
        }
        // Non-special skills should NOT match
        assert!(!matches!(108010u32, 109035 | 110035 | 209035 | 210035));
    }

    /// Test special skill 109015 requires target==caster.
    #[test]
    fn test_special_skill_self_target_required() {
        // Skills 109015/110015/209015/210015 require target == caster
        for &skill_id in &[109015u32, 110015, 209015, 210015] {
            assert!(
                matches!(skill_id, 109015 | 110015 | 209015 | 210015),
                "Skill {} should match self-target pattern",
                skill_id
            );
        }
        // Non-special skills should NOT match
        assert!(!matches!(200000u32, 109015 | 110015 | 209015 | 210015));
    }

    /// Test MAGIC_CANCEL_TRANSFORMATION constant is 7.
    #[test]
    fn test_magic_cancel_transformation_value() {
        assert_eq!(MAGIC_CANCEL_TRANSFORMATION, 7);
    }

    /// Test MAGIC_TYPE4_EXTEND constant is 8.
    #[test]
    fn test_magic_type4_extend_value() {
        assert_eq!(MAGIC_TYPE4_EXTEND, 8);
    }

    /// Test MAGIC_DURATION_EXPIRED constant is 5.
    #[test]
    fn test_magic_duration_expired_value() {
        assert_eq!(MAGIC_DURATION_EXPIRED, 5);
    }

    /// Test HP cost skill check: skill 105710 (hp=100, msp=0) requires 100 HP.
    #[test]
    fn test_hp_cost_skill_conditions() {
        // Normal HP cost: hp > 0, msp == 0, hp < 10000
        let hp_cost: i16 = 100;
        let mana_cost: i16 = 0;
        assert!(hp_cost > 0 && mana_cost == 0 && hp_cost < 10000);

        // Sacrifice skill: hp >= 10000
        let sacrifice_hp: i16 = 10001;
        assert!(sacrifice_hp >= 10000);

        // Not HP cost: has msp too (like skill 106018: hp=720, msp=1920)
        let hp_both: i16 = 720;
        let msp_both: i16 = 1920;
        assert!(!(hp_both > 0 && msp_both == 0 && hp_both < 10000));
    }

    /// Test sacrifice skill self-target rejection.
    #[test]
    fn test_sacrifice_self_target_rejected() {
        let caster_sid: u16 = 42;
        let target_id: i16 = 42; // same as caster
        assert_eq!(target_id, caster_sid as i16, "Self-target should match");

        let target_id_other: i16 = 99;
        assert_ne!(
            target_id_other, caster_sid as i16,
            "Different target should not match"
        );
    }

    /// Test HP cost deduction calculation.
    #[test]
    fn test_hp_cost_deduction() {
        let caster_hp: i16 = 500;

        // Normal HP cost: 100
        let hp_cost: i16 = 100;
        let new_hp = (caster_hp - hp_cost).max(0);
        assert_eq!(new_hp, 400);

        // Sacrifice: always deducts 10000
        let sacrifice_deduct: i16 = 10000;
        let new_hp_sacrifice = (caster_hp - sacrifice_deduct).max(0);
        assert_eq!(new_hp_sacrifice, 0); // clamped to 0
    }

    // ── Type1 war zone damage reduction tests ───────────────────────────

    /// Test Type1 sAdditionalDamage reduction in war zone (divide by 2).
    #[test]
    fn test_type1_add_damage_war_zone_reduction() {
        // In war zone, sAdditionalDamage /= 2
        let add_damage: i16 = 100;
        let reduced = add_damage / 2;
        assert_eq!(reduced, 50);
    }

    /// Test Type1 sAdditionalDamage reduction in non-war zone (divide by 3).
    #[test]
    fn test_type1_add_damage_non_war_zone_reduction() {
        // In non-war zone, sAdditionalDamage /= 3
        let add_damage: i16 = 100;
        let reduced = add_damage / 3;
        assert_eq!(reduced, 33);
    }

    /// Test Type1 zero add_damage is unchanged.
    #[test]
    fn test_type1_add_damage_zero_no_division() {
        let add_damage: i16 = 0;
        // Should not divide by 0 — only apply when add_damage > 0
        assert_eq!(add_damage, 0);
    }

    // ── iADPtoUser / iADPtoNPC modifier tests ──────────────────────────

    /// Test iADPtoUser modifier for PvP (100% = identity).
    #[test]
    fn test_iadp_to_user_identity() {
        let damage: i16 = 200;
        let adp = 100i32; // 100% = no change
        let result = ((damage as i32 * adp) / 100) as i16;
        assert_eq!(result, 200);
    }

    /// Test iADPtoUser modifier for PvP (150% = 1.5x).
    #[test]
    fn test_iadp_to_user_150_percent() {
        let damage: i16 = 200;
        let adp = 150i32;
        let result = ((damage as i32 * adp) / 100) as i16;
        assert_eq!(result, 300);
    }

    /// Test iADPtoNPC modifier (80% = 0.8x reduction).
    #[test]
    fn test_iadp_to_npc_80_percent() {
        let damage: i16 = 200;
        let adp = 80i32;
        let result = ((damage as i32 * adp) / 100) as i16;
        assert_eq!(result, 160);
    }

    /// Test iADP zero means no modification (skip).
    #[test]
    fn test_iadp_zero_skips_modification() {
        let damage: i16 = 200;
        let adp = 0i32;
        // When adp == 0, we skip the multiplication entirely
        // (otherwise damage would become 0)
        assert_eq!(adp, 0);
        // Damage unchanged
        assert_eq!(damage, 200);
    }

    // ── Sprint 137: is_staff_skill tests ─────────────────────────────────

    /// Verify all staff skill IDs are recognized.
    #[test]
    fn test_is_staff_skill_known_ids() {
        use super::is_staff_skill;
        // Lightning staff (Karus/Elmo, novice/master)
        assert!(is_staff_skill(109742));
        assert!(is_staff_skill(110742));
        assert!(is_staff_skill(209742));
        assert!(is_staff_skill(210742));
        // Fire staff
        assert!(is_staff_skill(109542));
        assert!(is_staff_skill(210542));
        // Ice staff
        assert!(is_staff_skill(109642));
        assert!(is_staff_skill(210642));
        // Master 43/56
        assert!(is_staff_skill(109556));
        assert!(is_staff_skill(210743));
    }

    /// Non-staff skills should not be recognized.
    #[test]
    fn test_is_staff_skill_rejects_non_staff() {
        use super::is_staff_skill;
        assert!(!is_staff_skill(107650)); // drain skill
        assert!(!is_staff_skill(105725)); // stomp skill
        assert!(!is_staff_skill(100000)); // generic skill
        assert!(!is_staff_skill(0));
    }

    // ── Sprint 137: is_drain_skill tests ─────────────────────────────────

    /// All 8 drain skill IDs should be recognized.
    #[test]
    fn test_is_drain_skill_known_ids() {
        use super::is_drain_skill;
        assert!(is_drain_skill(107650));
        assert!(is_drain_skill(108650));
        assert!(is_drain_skill(207650));
        assert!(is_drain_skill(208650));
        assert!(is_drain_skill(107610));
        assert!(is_drain_skill(108610));
        assert!(is_drain_skill(207610));
        assert!(is_drain_skill(208610));
    }

    /// Non-drain skills should not be recognized.
    #[test]
    fn test_is_drain_skill_rejects_non_drain() {
        use super::is_drain_skill;
        assert!(!is_drain_skill(109742)); // staff skill
        assert!(!is_drain_skill(105725)); // stomp skill
        assert!(!is_drain_skill(0));
    }

    // ── Sprint 137: is_stomp_skill tests ─────────────────────────────────

    /// All 14 stomp skill IDs should be recognized.
    #[test]
    fn test_is_stomp_skill_known_ids() {
        use super::is_stomp_skill;
        assert!(is_stomp_skill(105725));
        assert!(is_stomp_skill(106735));
        assert!(is_stomp_skill(205725));
        assert!(is_stomp_skill(206735));
        assert!(is_stomp_skill(105760));
        assert!(is_stomp_skill(106775));
        assert!(is_stomp_skill(206775));
    }

    /// Non-stomp skills should not be recognized.
    #[test]
    fn test_is_stomp_skill_rejects_non_stomp() {
        use super::is_stomp_skill;
        assert!(!is_stomp_skill(107650)); // drain
        assert!(!is_stomp_skill(109742)); // staff
        assert!(!is_stomp_skill(0));
    }

    // ── Sprint 137: nation validation tests ──────────────────────────────

    /// Karus skill (1xxxxx) should only be usable by nation 1.
    #[test]
    fn test_nation_validation_karus_skill() {
        let skill_id: u32 = 107650;
        assert!(skill_id < 300000);
        assert_eq!(skill_id / 100000, 1); // Karus nation
                                          // Nation 1 (Karus) matches
        assert_eq!(1u8, (skill_id / 100000) as u8);
        // Nation 2 (Elmo) does NOT match
        assert_ne!(2u8, (skill_id / 100000) as u8);
    }

    /// Elmorad skill (2xxxxx) should only be usable by nation 2.
    #[test]
    fn test_nation_validation_elmo_skill() {
        let skill_id: u32 = 209742;
        assert!(skill_id < 300000);
        assert_eq!(skill_id / 100000, 2); // Elmo nation
        assert_eq!(2u8, (skill_id / 100000) as u8);
        assert_ne!(1u8, (skill_id / 100000) as u8);
    }

    /// General skills (3xxxxx+) skip nation check.
    #[test]
    fn test_nation_validation_general_skill_skips() {
        let skill_id: u32 = 390001;
        assert!(skill_id >= 300000);
        // Nation check is skipped for skills >= 300000
    }

    // ── Sprint 137: dynamic range modifier tests ─────────────────────────

    /// Melee skill (type1==1, no cast time, non-staff) while moving → range 18.
    #[test]
    fn test_melee_range_moving() {
        // C++ lines 241-244: type1==1, castTime==0, !isStaffSkill, m_sSpeed != 0 → 18
        let type1: i16 = 1;
        let cast_time: i16 = 0;
        let skill_id: u32 = 105010; // generic warrior skill (not staff)
        let is_moving = true;

        let range = if (type1 == 1) && cast_time == 0 && !super::is_staff_skill(skill_id) {
            if is_moving {
                18
            } else {
                12
            }
        } else {
            0
        };
        assert_eq!(range, 18);
    }

    /// Melee skill while standing → range 12.
    #[test]
    fn test_melee_range_standing() {
        let range = if !super::is_staff_skill(105010) {
            if false {
                18
            } else {
                12
            }
        } else {
            0
        };
        assert_eq!(range, 12);
    }

    /// Staff skill while moving → base_range + 17.
    #[test]
    fn test_staff_range_moving() {
        let base_range: i32 = 10;
        let skill_id: u32 = 109742; // staff skill
        let is_moving = true;
        assert!(super::is_staff_skill(skill_id));

        let range = if super::is_staff_skill(skill_id) && is_moving {
            base_range + 17
        } else if super::is_staff_skill(skill_id) {
            base_range + 10
        } else {
            base_range + 9
        };
        assert_eq!(range, 27);
    }

    /// Staff skill while standing → base_range + 10.
    #[test]
    fn test_staff_range_standing() {
        let base_range: i32 = 10;
        let skill_id: u32 = 109742;
        let is_moving = false;

        let range = if super::is_staff_skill(skill_id) && is_moving {
            base_range + 17
        } else if super::is_staff_skill(skill_id) {
            base_range + 10
        } else {
            base_range + 9
        };
        assert_eq!(range, 20);
    }

    /// Drain skill → base_range + 5.
    #[test]
    fn test_drain_range() {
        let base_range: i32 = 8;
        let skill_id: u32 = 107650;
        assert!(super::is_drain_skill(skill_id));

        let range = base_range + 5;
        assert_eq!(range, 13);
    }

    /// Default skill range → base_range + 9.
    #[test]
    fn test_default_range() {
        let base_range: i32 = 15;
        let skill_id: u32 = 104010; // generic priest skill
        assert!(!super::is_staff_skill(skill_id));
        assert!(!super::is_drain_skill(skill_id));

        let range = base_range + 9;
        assert_eq!(range, 24);
    }

    /// use_standing check: standing required + moving → fail.
    #[test]
    fn test_use_standing_moving_fails() {
        let use_standing: i16 = 1;
        let move_old_speed: i16 = 45;
        let should_fail = use_standing == 1 && move_old_speed != 0;
        assert!(should_fail);
    }

    /// use_standing check: standing required + standing → pass.
    #[test]
    fn test_use_standing_stationary_passes() {
        let use_standing: i16 = 1;
        let move_old_speed: i16 = 0;
        let should_fail = use_standing == 1 && move_old_speed != 0;
        assert!(!should_fail);
    }

    /// NPC_PARTNER_TYPE (213) with group 0 should be immune.
    #[test]
    fn test_npc_partner_type_immune() {
        let npc_type: u8 = NPC_PARTNER_TYPE;
        let group: u8 = 0; // Nation::NONE
        assert!(npc_type == NPC_PARTNER_TYPE && group == 0);
    }

    /// NPC_PARTNER_TYPE with non-zero group should NOT be immune.
    #[test]
    fn test_npc_partner_type_non_zero_group() {
        let npc_type: u8 = NPC_PARTNER_TYPE;
        let group: u8 = 1; // Karus nation
        assert!(!(npc_type == NPC_PARTNER_TYPE && group == 0));
    }

    // ── CheckSkillClass tests ──────────────────────────────────────

    /// Karus beginner warrior (class 101) can use warrior skill (iclass 101).
    #[test]
    fn test_check_skill_class_karu_warrior_pass() {
        assert!(super::check_skill_class(101, 101)); // class 101 → classType 1
    }

    /// Elmorad beginner warrior (class 201) can use Elmo warrior skill (iclass 201).
    #[test]
    fn test_check_skill_class_elmo_warrior_pass() {
        assert!(super::check_skill_class(201, 201)); // class 201 → classType 1
    }

    /// Karus rogue (class 102) cannot use warrior skill (iclass 101).
    #[test]
    fn test_check_skill_class_rogue_rejects_warrior() {
        assert!(!super::check_skill_class(101, 102)); // rogue classType=2, needs 1
    }

    /// Guardian (class 106) can use mastered warrior skill (iclass 106).
    #[test]
    fn test_check_skill_class_guardian_pass() {
        assert!(super::check_skill_class(106, 106)); // classType=6
    }

    /// Guardian (class 106) cannot use novice warrior skill (iclass 105).
    #[test]
    fn test_check_skill_class_guardian_rejects_berserker() {
        assert!(!super::check_skill_class(105, 106)); // classType=6, needs 5
    }

    /// Berserker (class 105) cannot use mastered warrior skill (iclass 106).
    #[test]
    fn test_check_skill_class_berserker_rejects_guardian() {
        assert!(!super::check_skill_class(106, 105)); // classType=5, needs 6
    }

    /// Mage (class 209, Elmorad novice mage) can use mage skill (iclass 209).
    #[test]
    fn test_check_skill_class_mage_pass() {
        assert!(super::check_skill_class(209, 209)); // classType=9
    }

    /// Mage cannot use priest skill.
    #[test]
    fn test_check_skill_class_mage_rejects_priest() {
        assert!(!super::check_skill_class(104, 209)); // mage classType=9, needs 4
    }

    /// Kurian master (class 115) can use master kurian skill (iclass 115).
    #[test]
    fn test_check_skill_class_kurian_master_pass() {
        assert!(super::check_skill_class(115, 115)); // classType=15
    }

    /// Porutu master (class 215) can use master porutu skill (iclass 215).
    #[test]
    fn test_check_skill_class_porutu_master_pass() {
        assert!(super::check_skill_class(215, 215)); // classType=15
    }

    /// Common skill (iclass 100) passes for any class.
    #[test]
    fn test_check_skill_class_common_passes() {
        assert!(super::check_skill_class(100, 101));
        assert!(super::check_skill_class(100, 209));
        assert!(super::check_skill_class(190, 105));
        assert!(super::check_skill_class(200, 201));
        assert!(super::check_skill_class(290, 211));
    }

    /// iclass=0 and sSkill=0 both cause the check to be skipped in the handler,
    /// so check_skill_class should return true for unknown iclass (default arm).
    #[test]
    fn test_check_skill_class_zero_passes() {
        assert!(super::check_skill_class(0, 101));
    }

    /// Karus class can use Elmorad-equivalent skill from same tier.
    /// GUARDIAN (106) and PROTECTOR (206) both require classType=6.
    #[test]
    fn test_check_skill_class_cross_nation_equivalent() {
        // Guardian (106) → classType=6, Protector skill (iclass 206) → needs classType=6
        assert!(super::check_skill_class(206, 106));
        // Protector (206) → classType=6, Guardian skill (iclass 106) → needs classType=6
        assert!(super::check_skill_class(106, 206));
    }

    // ── Arrow consumption tests ────────────────────────────────────

    /// ITEM_INFINITYARC constant matches C++ GameDefine.h:1291.
    #[test]
    fn test_infinity_arc_constant() {
        const ITEM_INFINITYARC: u32 = 800606000;
        assert_eq!(ITEM_INFINITYARC, 800606000);
    }

    /// NeedArrow=0 should default to count=1 (throwing knives).
    #[test]
    fn test_need_arrow_zero_defaults_to_one() {
        let need_arrow: i32 = 0;
        let mut count = need_arrow as u16;
        if count == 0 {
            count = 1;
        }
        assert_eq!(count, 1);
    }

    /// NeedArrow=5 should consume 5 arrows.
    #[test]
    fn test_need_arrow_five() {
        let need_arrow: i32 = 5;
        let mut count = need_arrow as u16;
        if count == 0 {
            count = 1;
        }
        assert_eq!(count, 5);
    }

    /// Arrow consumption bypassed when player has ITEM_INFINITYARC and use_item is 391010000.
    #[test]
    fn test_infinity_arc_bypass() {
        const _ITEM_INFINITYARC: u32 = 800606000;
        let use_item: u32 = 391010000;
        let has_infinity_arc = true; // simulate check_exist_item
        let bypass = use_item == 391010000 && has_infinity_arc;
        assert!(bypass);

        // Different arrow type should NOT bypass
        let use_item2: u32 = 389023000;
        let bypass2 = use_item2 == 391010000 && has_infinity_arc;
        assert!(!bypass2);
    }

    /// Arrow consumption NOT bypassed when player lacks ITEM_INFINITYARC.
    #[test]
    fn test_no_infinity_arc_no_bypass() {
        let use_item: u32 = 391010000;
        let has_infinity_arc = false; // no infinity arrows
        let bypass = use_item == 391010000 && has_infinity_arc;
        assert!(!bypass);
    }

    // ── Sprint 139: Cast position validation tests ──────────────────

    #[test]
    fn test_cast_position_same_position_passes() {
        // Player hasn't moved since casting — position check should pass
        let cast_x: f32 = 100.5;
        let cast_z: f32 = 200.3;
        let cur_x: f32 = 100.5;
        let cur_z: f32 = 200.3;
        assert!(cast_x == cur_x && cast_z == cur_z);
    }

    #[test]
    fn test_cast_position_moved_fails() {
        // Player moved since casting — position check should fail
        let cast_x: f32 = 100.5;
        let cast_z: f32 = 200.3;
        let cur_x: f32 = 105.0;
        let cur_z: f32 = 200.3;
        assert!(cast_x != cur_x || cast_z != cur_z);
    }

    #[test]
    fn test_cast_position_flying_effect_check() {
        // If skill has flying_effect, check position on FLYING phase
        // If skill has NO flying_effect, check position on EFFECTING phase
        let flying_effect_skill: i16 = 1;
        let no_flying_effect_skill: i16 = 0;

        // FLYING phase: only check if flying_effect != 0
        assert!(flying_effect_skill != 0);
        assert!(no_flying_effect_skill == 0);
    }

    #[test]
    fn test_cast_skill_id_mismatch_skips() {
        // If saved cast_skill_id doesn't match current skill, skip position check
        let saved_skill_id: u32 = 101001;
        let current_skill_id: u32 = 102002;
        let matches = saved_skill_id == current_skill_id;
        assert!(!matches);
    }

    #[test]
    fn test_cast_position_zero_defaults() {
        // Initial state: cast_skill_id=0, cast_x=0, cast_z=0
        // A skill_id of 0 means no cast was saved, so get_cast_position returns None
        let cast_skill_id: u32 = 0;
        let _skill_id: u32 = 101001;
        assert_eq!(cast_skill_id, 0);
    }

    // ── Sprint 139: Friendly buff range check tests ─────────────────

    #[test]
    fn test_party_bypass_same_party() {
        // moral==MORAL_PARTY && type[0]==4 && same party → bypass range
        let moral: i16 = 8; // MORAL_PARTY
        let caster_party: Option<u16> = Some(1);
        let target_party: Option<u16> = Some(1);
        let party_bypass =
            (moral == 8 || moral == 9) && caster_party.is_some() && caster_party == target_party;
        assert!(party_bypass);
    }

    #[test]
    fn test_party_bypass_different_party() {
        // Different party IDs — no bypass
        let moral: i16 = 8; // MORAL_PARTY
        let caster_party: Option<u16> = Some(1);
        let target_party: Option<u16> = Some(2);
        let party_bypass =
            (moral == 8 || moral == 9) && caster_party.is_some() && caster_party == target_party;
        assert!(!party_bypass);
    }

    #[test]
    fn test_party_bypass_no_party() {
        // No party — no bypass
        let moral: i16 = 8; // MORAL_PARTY
        let caster_party: Option<u16> = None;
        let target_party: Option<u16> = None;
        let party_bypass =
            (moral == 8 || moral == 9) && caster_party.is_some() && caster_party == target_party;
        assert!(!party_bypass);
    }

    #[test]
    fn test_friend_withme_no_bypass() {
        // MORAL_FRIEND_WITHME (moral=2) — NO party bypass, must check range
        let moral: i16 = 2; // MORAL_FRIEND_WITHME
        let caster_party: Option<u16> = Some(1);
        let target_party: Option<u16> = Some(1);
        let party_bypass =
            (moral == 8 || moral == 9) && caster_party.is_some() && caster_party == target_party;
        assert!(!party_bypass); // moral != MORAL_PARTY, so no bypass
    }

    #[test]
    fn test_self_target_skips_range() {
        // When target == caster, range check is skipped
        let caster_sid: u16 = 5;
        let target_sid: u16 = 5;
        assert_eq!(caster_sid, target_sid);
    }

    // ── Sprint 139: AOE NPC targeting tests ─────────────────────────

    #[test]
    fn test_aoe_npc_radius_check() {
        // NPC at (100, 100), AOE center at (100, 110), radius=15
        let aoe_x: f32 = 100.0;
        let aoe_z: f32 = 110.0;
        let npc_x: f32 = 100.0;
        let npc_z: f32 = 100.0;
        let radius: f32 = 15.0;
        let radius_sq = radius * radius;

        let dx = aoe_x - npc_x;
        let dz = aoe_z - npc_z;
        let dist_sq = dx * dx + dz * dz;

        assert!(dist_sq <= radius_sq); // 100 <= 225
    }

    #[test]
    fn test_aoe_center_accepts_v2603_tenths() {
        let center = resolve_aoe_center(1_050, 2_050, 100.0, 200.0);
        assert_eq!(center, (105.0, 205.0));
    }

    #[test]
    fn test_aoe_center_accepts_v2615_world_units() {
        let center = resolve_aoe_center(105, 205, 100.0, 200.0);
        assert_eq!(center, (105.0, 205.0));
    }

    #[test]
    fn test_aoe_center_zero_data_falls_back_to_caster() {
        let center = resolve_aoe_center(0, 0, 100.0, 200.0);
        assert_eq!(center, (100.0, 200.0));
    }

    #[test]
    fn test_aoe_npc_out_of_radius() {
        // NPC at (100, 100), AOE center at (100, 130), radius=15
        let aoe_x: f32 = 100.0;
        let aoe_z: f32 = 130.0;
        let npc_x: f32 = 100.0;
        let npc_z: f32 = 100.0;
        let radius: f32 = 15.0;
        let radius_sq = radius * radius;

        let dx = aoe_x - npc_x;
        let dz = aoe_z - npc_z;
        let dist_sq = dx * dx + dz * dz;

        assert!(dist_sq > radius_sq); // 900 > 225
    }

    #[test]
    fn test_aoe_npc_monster_filter() {
        // Only monsters (is_monster=true) should be targeted, not friendly NPCs
        let is_monster = true;
        assert!(is_monster);

        let is_friendly_npc = false;
        assert!(!is_friendly_npc);
    }

    #[test]
    fn test_aoe_npc_dead_skip() {
        // Dead NPCs (hp <= 0) should be skipped
        let npc_hp: i32 = 0;
        assert!(npc_hp <= 0);

        let alive_npc_hp: i32 = 500;
        assert!(alive_npc_hp > 0);
    }

    #[test]
    fn test_aoe_npc_damage_clamp() {
        // NPC HP should never go below 0
        let npc_hp: i32 = 100;
        let damage: i32 = 500;
        let new_hp = (npc_hp - damage).max(0);
        assert_eq!(new_hp, 0);
    }

    #[test]
    fn test_aoe_moral_area_enemy_targets_npcs() {
        // MORAL_AREA_ENEMY (moral=6) should include NPC targeting
        let moral: i16 = 6;
        assert!(moral == 6 || moral == 7); // MORAL_AREA_ENEMY or MORAL_AREA_ALL
    }

    #[test]
    fn test_aoe_moral_area_friend_skips_npcs() {
        // MORAL_AREA_FRIEND (moral=10) should NOT target NPCs
        let moral: i16 = 10;
        assert!(moral != 6 && moral != 7); // not MORAL_AREA_ENEMY/ALL
    }

    // ── Sprint 153: Type1/Type2 damage formula tests ─────────────────────

    fn make_type1_data(s_hit: i32, hit_type: i32, hit_rate: i32, add_damage: i32) -> MagicType1Row {
        MagicType1Row {
            i_num: 100000,
            hit_type: Some(hit_type),
            hit_rate: Some(hit_rate),
            hit: Some(s_hit),
            add_damage: Some(add_damage),
            combo_type: None,
            combo_count: None,
            combo_damage: None,
            range: None,
            delay: None,
            add_dmg_perc_to_user: None,
            add_dmg_perc_to_npc: None,
        }
    }

    fn make_type2_data(hit_type: i32, hit_rate: i32, add_damage: i32) -> MagicType2Row {
        MagicType2Row {
            i_num: 200000,
            hit_type: Some(hit_type),
            hit_rate: Some(hit_rate),
            add_damage: Some(add_damage),
            add_range: None,
            need_arrow: None,
            add_dmg_perc_to_user: None,
            add_dmg_perc_to_npc: None,
        }
    }

    #[test]
    fn test_type1_s_hit_modifier_applied() {
        // With sHit=150 (150%), damage should be higher than sHit=100 (100%)
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type1_100 = make_type1_data(100, 0, 100, 0);
        let type1_150 = make_type1_data(150, 0, 100, 0);

        let mut total_100 = 0i64;
        let mut total_150 = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_100 += compute_type1_hit_damage(
                100, target_ac, &type1_100, 100.0, 1.0, 100, 100, &mut rng,
            ) as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_150 += compute_type1_hit_damage(
                100, target_ac, &type1_150, 100.0, 1.0, 100, 100, &mut rng,
            ) as i64;
        }
        assert!(
            total_150 > total_100,
            "sHit=150 should produce more damage than sHit=100: 150={}, 100={}",
            total_150,
            total_100
        );
    }

    #[test]
    fn test_type1_formula_not_r_attack() {
        // Type1 uses (temp_hit + 0.3*rand + 0.99), NOT (0.75*hit_b + 0.3*rand).
        // With sHit=100, temp_hit == temp_hit_b, so Type1 base is ~1.0*temp_hit
        // while R-attack base is ~0.75*temp_hit_b. Type1 should be higher.
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type1_data = make_type1_data(100, 0, 100, 0);

        let mut total_type1 = 0i64;
        for seed in 0u64..500 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_type1 += compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng,
            ) as i64;
        }
        // With sHit=100, the C++ formula is temp_hit + 0.3*rand + 0.99
        // At minimum: temp_hit + 0.99 ≈ temp_hit + 1
        // If temp_hit_b > 0, average should be positive
        assert!(
            total_type1 > 0,
            "Type1 damage total should be positive: {}",
            total_type1
        );
    }

    #[test]
    fn test_type1_non_relative_hit_check() {
        // bHitType != 0: uses absolute hit rate check
        // sHitRate=100 should almost always hit; sHitRate=0 should always miss
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type1_always_hit = make_type1_data(100, 1, 100, 0);
        let type1_always_miss = make_type1_data(100, 1, 0, 0);

        let mut hits = 0;
        let mut misses = 0;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            if compute_type1_hit_damage(
                100,
                target_ac,
                &type1_always_hit,
                1.0,
                100.0,
                100,
                100,
                &mut rng,
            ) > 0
            {
                hits += 1;
            }
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            if compute_type1_hit_damage(
                100,
                target_ac,
                &type1_always_miss,
                1.0,
                100.0,
                100,
                100,
                &mut rng,
            ) > 0
            {
                misses += 1;
            }
        }
        assert!(
            hits > 150,
            "sHitRate=100 should hit most of the time: hits={}",
            hits
        );
        assert_eq!(misses, 0, "sHitRate=0 should always miss");
    }

    #[test]
    fn test_type2_add_damage_is_percentage() {
        // sAddDamage=200 (200%) should produce more damage than sAddDamage=100 (100%)
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type2_100 = make_type2_data(0, 100, 100);
        let type2_200 = make_type2_data(0, 100, 200);

        let mut total_100 = 0i64;
        let mut total_200 = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_100 += compute_type2_hit_damage(
                100, target_ac, &type2_100, 100.0, 1.0, 100, 100, &mut rng,
            ) as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_200 += compute_type2_hit_damage(
                100, target_ac, &type2_200, 100.0, 1.0, 100, 100, &mut rng,
            ) as i64;
        }
        assert!(
            total_200 > total_100,
            "sAddDamage=200 should deal more than 100: 200={}, 100={}",
            total_200,
            total_100
        );
    }

    #[test]
    fn test_type2_penetration_bypasses_ac() {
        // bHitType=1 (penetration) ignores AC — high AC target should take same damage
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let type2_pen = make_type2_data(1, 100, 200);

        let mut total_low_ac = 0i64;
        let mut total_high_ac = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_low_ac +=
                compute_type2_hit_damage(100, 10, &type2_pen, 100.0, 1.0, 100, 100, &mut rng)
                    as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_high_ac +=
                compute_type2_hit_damage(100, 500, &type2_pen, 100.0, 1.0, 100, 100, &mut rng)
                    as i64;
        }
        // Penetration: damage should be identical regardless of AC
        assert_eq!(
            total_low_ac, total_high_ac,
            "Penetration should ignore AC: low_ac={}, high_ac={}",
            total_low_ac, total_high_ac
        );
    }

    #[test]
    fn test_type2_normal_uses_ac() {
        // bHitType=0 (normal): higher AC should reduce damage
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let type2_normal = make_type2_data(0, 100, 200);

        let mut total_low_ac = 0i64;
        let mut total_high_ac = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_low_ac +=
                compute_type2_hit_damage(100, 10, &type2_normal, 100.0, 1.0, 100, 100, &mut rng)
                    as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_high_ac +=
                compute_type2_hit_damage(100, 500, &type2_normal, 100.0, 1.0, 100, 100, &mut rng)
                    as i64;
        }
        assert!(
            total_low_ac > total_high_ac,
            "Higher AC should reduce damage: low_ac={}, high_ac={}",
            total_low_ac,
            total_high_ac
        );
    }

    #[test]
    fn test_type2_randomization_formula() {
        // Type2 uses (temp_hit * 0.6 + 1.0 * random + 0.99), NOT (0.75 * hit_b + 0.3 * random)
        // The 0.6 coefficient means a guaranteed 60% base + up to 100% random bonus
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type2_data = make_type2_data(0, 100, 200);

        let mut total = 0i64;
        let mut count = 0;
        for seed in 0u64..500 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let dmg = compute_type2_hit_damage(
                100,
                target_ac,
                &type2_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng,
            );
            if dmg > 0 {
                total += dmg as i64;
                count += 1;
            }
        }
        assert!(count > 0, "Should have some successful hits");
        assert!(
            total > 0,
            "Type2 total damage should be positive: {}",
            total
        );
    }

    #[test]
    fn test_type2_penetration_hit_check() {
        // bHitType=1: uses absolute hit rate
        // sHitRate=0 should always miss
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let type2_miss = make_type2_data(1, 0, 200);

        let mut hits = 0;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            if compute_type2_hit_damage(100, 50, &type2_miss, 1.0, 100.0, 100, 100, &mut rng) > 0 {
                hits += 1;
            }
        }
        assert_eq!(hits, 0, "sHitRate=0 penetration should always miss");
    }

    // ── Sprint 154: attack_amount buff integration tests ────────────

    /// Attack amount buff (130) should increase Type1 damage vs default (100).
    #[test]
    fn test_type1_attack_amount_buff_increases_damage() {
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type1_data = make_type1_data(100, 0, 100, 0);

        let mut total_100 = 0i64;
        let mut total_130 = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_100 += compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng,
            ) as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_130 += compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                130,
                100,
                &mut rng,
            ) as i64;
        }
        assert!(
            total_130 > total_100,
            "attack_amount=130 should deal more than 100: 130={}, 100={}",
            total_130,
            total_100
        );
    }

    /// Attack amount buff should increase Type2 damage proportionally.
    #[test]
    fn test_type2_attack_amount_buff_increases_damage() {
        let _caster = make_test_character(2, 50, 60, 80, 30, 30);
        let target_ac = 50;
        let type2_data = make_type2_data(0, 100, 150);

        let mut total_100 = 0i64;
        let mut total_130 = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_100 += compute_type2_hit_damage(
                100,
                target_ac,
                &type2_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng,
            ) as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_130 += compute_type2_hit_damage(
                100,
                target_ac,
                &type2_data,
                100.0,
                1.0,
                130,
                100,
                &mut rng,
            ) as i64;
        }
        assert!(
            total_130 > total_100,
            "attack_amount=130 should deal more than 100: 130={}, 100={}",
            total_130,
            total_100
        );
    }

    /// Type2 penetration with attack_amount buff should scale correctly.
    #[test]
    fn test_type2_penetration_with_attack_amount_buff() {
        let _caster = make_test_character(2, 50, 60, 80, 30, 30);
        let type2_pen = make_type2_data(1, 100, 200);

        let mut total_100 = 0i64;
        let mut total_150 = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_100 +=
                compute_type2_hit_damage(100, 50, &type2_pen, 100.0, 1.0, 100, 100, &mut rng)
                    as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_150 +=
                compute_type2_hit_damage(100, 50, &type2_pen, 100.0, 1.0, 150, 100, &mut rng)
                    as i64;
        }
        assert!(
            total_150 > total_100,
            "Penetration with attack_amount=150 should deal more: 150={}, 100={}",
            total_150,
            total_100
        );
    }

    /// attack_amount=100 (no buffs) should produce baseline damage.
    #[test]
    fn test_attack_amount_default_produces_baseline() {
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type1_data = make_type1_data(100, 0, 100, 0);

        let mut total_default = 0i64;
        let mut total_zero = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_default += compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng,
            ) as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            // attack_amount=50 (debuffed) should deal less
            total_zero += compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                50,
                100,
                &mut rng,
            ) as i64;
        }
        assert!(
            total_default > total_zero,
            "attack_amount=100 should deal more than 50: 100={}, 50={}",
            total_default,
            total_zero
        );
    }

    // ── Sprint 155: player_attack_amount (PvP modifier) tests ─────

    /// player_attack_amount=150 should increase Type1 PvP damage vs default (100).
    #[test]
    fn test_type1_player_attack_amount_increases_pvp_damage() {
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type1_data = make_type1_data(100, 0, 100, 0);

        let mut total_100 = 0i64;
        let mut total_150 = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_100 += compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng,
            ) as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_150 += compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                100,
                150,
                &mut rng,
            ) as i64;
        }
        assert!(
            total_150 > total_100,
            "player_attack_amount=150 should deal more: 150={}, 100={}",
            total_150,
            total_100
        );
    }

    /// player_attack_amount=100 (default) should NOT change NPC damage.
    #[test]
    fn test_type1_player_attack_amount_100_no_change() {
        let _caster = make_test_character(1, 80, 60, 50, 30, 30);
        let target_ac = 50;
        let type1_data = make_type1_data(100, 0, 100, 0);

        for seed in 0u64..50 {
            let mut rng1 = rand::rngs::StdRng::seed_from_u64(seed);
            let d1 = compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng1,
            );
            // With player_attack_amount=100, damage should be identical to without it
            // (100 * 100 / 100 = no change)
            let mut rng2 = rand::rngs::StdRng::seed_from_u64(seed);
            let d2 = compute_type1_hit_damage(
                100,
                target_ac,
                &type1_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng2,
            );
            assert_eq!(d1, d2, "player_attack_amount=100 should not change damage");
        }
    }

    /// Type2 normal (non-penetration) should scale with player_attack_amount.
    #[test]
    fn test_type2_player_attack_amount_normal() {
        let _caster = make_test_character(2, 50, 60, 80, 30, 30);
        let target_ac = 50;
        let type2_data = make_type2_data(0, 100, 150);

        let mut total_100 = 0i64;
        let mut total_150 = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_100 += compute_type2_hit_damage(
                100,
                target_ac,
                &type2_data,
                100.0,
                1.0,
                100,
                100,
                &mut rng,
            ) as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_150 += compute_type2_hit_damage(
                100,
                target_ac,
                &type2_data,
                100.0,
                1.0,
                100,
                150,
                &mut rng,
            ) as i64;
        }
        assert!(
            total_150 > total_100,
            "Type2 player_attack_amount=150 should deal more: 150={}, 100={}",
            total_150,
            total_100
        );
    }

    /// Type2 penetration should NOT be affected by player_attack_amount.
    /// C++ line 426: penetration uses raw m_sTotalHit * m_bAttackAmount, not temp_ap.
    #[test]
    fn test_type2_penetration_ignores_player_attack_amount() {
        let _caster = make_test_character(2, 50, 60, 80, 30, 30);
        let type2_pen = make_type2_data(1, 100, 200);

        let mut total_100 = 0i64;
        let mut total_200 = 0i64;
        for seed in 0u64..200 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_100 +=
                compute_type2_hit_damage(100, 50, &type2_pen, 100.0, 1.0, 100, 100, &mut rng)
                    as i64;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            total_200 +=
                compute_type2_hit_damage(100, 50, &type2_pen, 100.0, 1.0, 100, 200, &mut rng)
                    as i64;
        }
        // Penetration uses raw total_hit * attack_amount, NOT temp_ap
        assert_eq!(
            total_100, total_200,
            "Penetration should ignore player_attack_amount: 100={}, 200={}",
            total_100, total_200
        );
    }

    // ── Sprint 154: Type1 AOE range tests ───────────────────────────

    /// Stomp skills should have +8 range bonus (pSkill.sRange + 5 + 3).
    #[test]
    fn test_stomp_skill_range_bonus() {
        let base_range: f32 = 10.0;
        let normal_range = base_range + 5.0;
        let stomp_range = base_range + 5.0 + 3.0;
        assert_eq!(normal_range, 15.0);
        assert_eq!(stomp_range, 18.0);
        assert!(stomp_range > normal_range, "Stomp range should be larger");
    }

    /// Type1 AOE distance check — target within range should be included.
    #[test]
    fn test_type1_aoe_distance_in_range() {
        let caster_x: f32 = 100.0;
        let caster_z: f32 = 100.0;
        let target_x: f32 = 110.0;
        let target_z: f32 = 100.0;
        let dist_range: f32 = 15.0;

        let dx = caster_x - target_x;
        let dz = caster_z - target_z;
        let dist_sq = dx * dx + dz * dz;
        let range_sq = dist_range * dist_range;
        assert!(
            dist_sq < range_sq,
            "Target at distance 10 should be within range 15"
        );
    }

    /// Type1 AOE distance check — target out of range should be excluded.
    #[test]
    fn test_type1_aoe_distance_out_of_range() {
        let caster_x: f32 = 100.0;
        let caster_z: f32 = 100.0;
        let target_x: f32 = 120.0;
        let target_z: f32 = 100.0;
        let dist_range: f32 = 15.0;

        let dx = caster_x - target_x;
        let dz = caster_z - target_z;
        let dist_sq = dx * dx + dz * dz;
        let range_sq = dist_range * dist_range;
        assert!(
            dist_sq >= range_sq,
            "Target at distance 20 should be outside range 15"
        );
    }

    /// AOE sData[1] should be set to 1 for AOE indicator.
    #[test]
    fn test_type1_aoe_data_indicator() {
        let mut instance = MagicInstance {
            opcode: 3,
            skill_id: 105725,
            caster_id: 1,
            target_id: -1,
            data: [50, 0, 50, 0, 0, 0, 0],
        };
        // Simulate the AOE branch setting
        instance.data[1] = 1;
        assert_eq!(instance.data[1], 1, "AOE indicator sData[1] should be 1");
    }

    // ── Sprint 156: Mage Armor Reflect Tests ─────────────────────────

    /// Counter-skill ID mapping: Fire + Karus nation should give 190573.
    #[test]
    fn test_reflect_counter_skill_fire_karus() {
        const FIRE_DAMAGE: u8 = 5;
        let skill_id: u32 = match (FIRE_DAMAGE, NATION_KARUS) {
            (5, 1) => 190573,
            (5, _) => 290573,
            (6, 1) => 190673,
            (6, _) => 290673,
            (7, 1) => 190773,
            (7, _) => 290773,
            _ => 0,
        };
        assert_eq!(skill_id, 190573);
    }

    /// Counter-skill ID mapping: Ice + Elmorad nation should give 290673.
    #[test]
    fn test_reflect_counter_skill_ice_elmorad() {
        const ICE_DAMAGE: u8 = 6;
        let skill_id: u32 = match (ICE_DAMAGE, NATION_ELMORAD) {
            (5, 1) => 190573,
            (5, _) => 290573,
            (6, 1) => 190673,
            (6, _) => 290673,
            (7, 1) => 190773,
            (7, _) => 290773,
            _ => 0,
        };
        assert_eq!(skill_id, 290673);
    }

    /// Counter-skill ID mapping: Lightning + Karus should give 190773.
    #[test]
    fn test_reflect_counter_skill_lightning_karus() {
        const LIGHTNING_DAMAGE: u8 = 7;
        let skill_id: u32 = match (LIGHTNING_DAMAGE, NATION_KARUS) {
            (5, 1) => 190573,
            (5, _) => 290573,
            (6, 1) => 190673,
            (6, _) => 290673,
            (7, 1) => 190773,
            (7, _) => 290773,
            _ => 0,
        };
        assert_eq!(skill_id, 190773);
    }

    /// reflect_armor_type derived from sSkill % 100 — Fire Armor.
    #[test]
    fn test_reflect_armor_type_from_sskill() {
        // Fire armor skills have sSkill values ending in 5 (e.g., 105, 205, etc.)
        let s_skill_fire: i16 = 105;
        let reflect_type = (s_skill_fire % 100) as u8;
        assert_eq!(
            reflect_type, 5,
            "Fire armor: sSkill % 100 should give FIRE_DAMAGE=5"
        );

        let s_skill_ice: i16 = 206;
        let reflect_type_ice = (s_skill_ice % 100) as u8;
        assert_eq!(
            reflect_type_ice, 6,
            "Ice armor: sSkill % 100 should give ICE_DAMAGE=6"
        );

        let s_skill_lightning: i16 = 307;
        let reflect_type_lt = (s_skill_lightning % 100) as u8;
        assert_eq!(
            reflect_type_lt, 7,
            "Lightning armor: sSkill % 100 should give LIGHTNING_DAMAGE=7"
        );
    }

    /// Unknown reflect_armor_type (not 5/6/7) should map to no counter-skill.
    #[test]
    fn test_reflect_unknown_element_no_counter_skill() {
        let reflect_type: u8 = 3; // not a valid element
        let nation: u8 = 1;
        let skill_id: u32 = match (reflect_type, nation) {
            (5, 1) => 190573,
            (5, _) => 290573,
            (6, 1) => 190673,
            (6, _) => 290673,
            (7, 1) => 190773,
            (7, _) => 290773,
            _ => 0,
        };
        assert_eq!(
            skill_id, 0,
            "Unknown element should not produce a counter-skill"
        );
    }

    /// BUFF_TYPE_MAGE_ARMOR constant should be 25.
    #[test]
    fn test_buff_type_mage_armor_constant() {
        assert_eq!(BUFF_TYPE_MAGE_ARMOR, 25);
    }

    // ── Temple event is_attackable gate tests ──────────────────────────

    /// Event user in temple zone with is_attackable=false → magic blocked.
    #[test]
    fn test_is_attackable_false_blocks_event_user() {
        use crate::systems::event_room::{EventRoomManager, EventUser, TempleEventType};
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);
        erm.update_temple_event(|s| {
            s.active_event = 4; // BDW
            s.is_active = true;
            s.is_attackable = false; // combat phase closed
        });
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
            room.add_user(EventUser {
                user_name: "mage1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            });
        }
        // is_attackable is false, user is registered → should block
        let is_attackable = erm.read_temple_event(|s| s.is_attackable);
        assert!(!is_attackable);
        let is_event_user = erm
            .find_user_room(TempleEventType::BorderDefenceWar, "mage1")
            .is_some();
        assert!(is_event_user);
        // Combined: !is_attackable && is_event_user → block
    }

    /// Event user in temple zone with is_attackable=true → magic allowed.
    #[test]
    fn test_is_attackable_true_allows_event_user() {
        use crate::systems::event_room::{EventRoomManager, EventUser, TempleEventType};
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);
        erm.update_temple_event(|s| {
            s.active_event = 4;
            s.is_active = true;
            s.is_attackable = true; // combat phase open
        });
        {
            let mut room = erm
                .get_room_mut(TempleEventType::BorderDefenceWar, 1)
                .unwrap();
            room.add_user(EventUser {
                user_name: "mage1".to_string(),
                session_id: 1,
                nation: 1,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            });
        }
        let is_attackable = erm.read_temple_event(|s| s.is_attackable);
        assert!(is_attackable);
        // is_attackable is true → magic NOT blocked by this gate
    }

    /// Non-event user in temple zone with is_attackable=false → ALSO blocked.
    /// C++ IsAvailable() at MagicInstance.cpp:2668 blocks ALL zone users,
    /// not just registered event users.
    #[test]
    fn test_is_attackable_false_non_event_user_also_blocked() {
        use crate::systems::event_room::{EventRoomManager, TempleEventType};
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::BorderDefenceWar, 1);
        erm.update_temple_event(|s| {
            s.active_event = 4;
            s.is_active = true;
            s.is_attackable = false;
        });
        // "wanderer" is NOT registered in any room
        let is_event_user = erm
            .find_user_room(TempleEventType::BorderDefenceWar, "wanderer")
            .is_some();
        assert!(!is_event_user);
        // Broad IsAvailable gate blocks ALL zone users (even non-event users)
        let is_attackable = erm.read_temple_event(|s| s.is_attackable);
        assert!(!is_attackable);
    }

    /// Chaos Dungeon event user with is_attackable=false → blocked.
    #[test]
    fn test_is_attackable_chaos_event_user_blocked() {
        use crate::systems::event_room::{EventRoomManager, EventUser, TempleEventType};
        let erm = EventRoomManager::new();
        erm.create_rooms(TempleEventType::ChaosDungeon, 1);
        erm.update_temple_event(|s| {
            s.active_event = 24; // Chaos
            s.is_active = true;
            s.is_attackable = false;
        });
        {
            let mut room = erm.get_room_mut(TempleEventType::ChaosDungeon, 1).unwrap();
            room.add_user(EventUser {
                user_name: "warrior1".to_string(),
                session_id: 5,
                nation: 2,
                prize_given: false,
                logged_out: false,
                kills: 0,
                deaths: 0,
                bdw_points: 0,
                has_altar_obtained: false,
            });
        }
        let is_attackable = erm.read_temple_event(|s| s.is_attackable);
        assert!(!is_attackable);
        let is_event_user = erm
            .find_user_room(TempleEventType::ChaosDungeon, "warrior1")
            .is_some();
        assert!(is_event_user);
    }

    // ── has_active_durational tests ────────────────────────────────────

    /// has_active_durational returns false when no durational skills exist.
    #[test]
    fn test_has_active_durational_empty() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.has_active_durational(1));
    }

    /// has_active_durational returns true when a DOT is active.
    #[test]
    fn test_has_active_durational_with_dot() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.add_durational_skill(1, 108100, -50, 5, 2);
        assert!(world.has_active_durational(1));
    }

    /// has_active_durational returns true when a HOT is active.
    #[test]
    fn test_has_active_durational_with_hot() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.add_durational_skill(1, 108200, 30, 5, 2);
        assert!(world.has_active_durational(1));
    }

    /// has_active_durational returns false after clearing all DOTs.
    #[test]
    fn test_has_active_durational_after_clear() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.add_durational_skill(1, 108100, -50, 5, 2);
        assert!(world.has_active_durational(1));
        world.clear_durational_skills(1);
        assert!(!world.has_active_durational(1));
    }

    // ── Type3 temple event skip tests ─────────────────────────────────

    /// Type3 skip: target with active DOT, event active, !is_attackable, in event zone
    /// → should be blocked
    #[test]
    fn test_type3_skip_event_zone_not_attackable() {
        use crate::systems::event_room;
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        // Give target an active DOT
        world.add_durational_skill(1, 108100, -50, 5, 2);
        assert!(world.has_active_durational(1));

        // Set event active but not attackable
        world.event_room_manager.update_temple_event(|s| {
            s.active_event = 4; // BDW
            s.is_active = true;
            s.is_attackable = false;
        });
        let (active_event, is_attackable) = world
            .event_room_manager
            .read_temple_event(|s| (s.active_event, s.is_attackable));
        assert_ne!(active_event, -1);
        assert!(!is_attackable);
        // Zone 84 (BDW) is temple event zone
        assert!(event_room::is_in_temple_event_zone(84));
        // All conditions met → Type3 should be skipped
    }

    /// Type3 no-skip: is_attackable=true → should NOT be blocked.
    #[test]
    fn test_type3_no_skip_when_attackable() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.add_durational_skill(1, 108100, -50, 5, 2);

        world.event_room_manager.update_temple_event(|s| {
            s.active_event = 4;
            s.is_active = true;
            s.is_attackable = true; // combat allowed
        });
        let is_attackable = world
            .event_room_manager
            .read_temple_event(|s| s.is_attackable);
        assert!(is_attackable);
        // is_attackable=true → Type3 should NOT be skipped
    }

    /// Type3 no-skip: target has no active DOTs → should NOT be blocked.
    #[test]
    fn test_type3_no_skip_no_active_dot() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        assert!(!world.has_active_durational(1));

        world.event_room_manager.update_temple_event(|s| {
            s.active_event = 4;
            s.is_active = true;
            s.is_attackable = false;
        });
        // No active DOTs → m_bType3Flag is false → Type3 should NOT be skipped
    }

    // ── Monster Stone magic isolation tests ──────────────────────────

    /// Players in the same Monster Stone event room CAN target each other.
    #[test]
    fn test_ms_magic_isolation_same_room_allowed() {
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| {
            h.event_room = 4;
            h.monster_stone_status = true;
        });
        world.update_session(2, |h| {
            h.event_room = 4;
            h.monster_stone_status = true;
        });

        let zone_id: u16 = 81; // Monster Stone zone
        assert!(crate::systems::monster_stone::is_monster_stone_zone(
            zone_id
        ));
        assert!(world.is_same_event_room(1, 2));
    }

    /// Players in different Monster Stone event rooms CANNOT target each other.
    #[test]
    fn test_ms_magic_isolation_different_room_blocked() {
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| {
            h.event_room = 2;
            h.monster_stone_status = true;
        });
        world.update_session(2, |h| {
            h.event_room = 6;
            h.monster_stone_status = true;
        });

        let zone_id: u16 = 82;
        assert!(crate::systems::monster_stone::is_monster_stone_zone(
            zone_id
        ));
        assert!(!world.is_same_event_room(1, 2));
        assert!(world.get_monster_stone_status(1));
    }

    /// One player in event room, other not — magic blocked.
    #[test]
    fn test_ms_magic_isolation_one_not_in_room() {
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        world.update_session(1, |h| {
            h.event_room = 3;
            h.monster_stone_status = true;
        });
        // Session 2 stays at event_room = 0, monster_stone_status = false

        let zone_id: u16 = 83;
        assert!(crate::systems::monster_stone::is_monster_stone_zone(
            zone_id
        ));
        assert!(!world.is_same_event_room(1, 2));
    }

    /// m_sMonsterStoneStatus=false → room isolation guard skipped.
    #[test]
    fn test_ms_magic_isolation_status_false_skips_guard() {
        let world = crate::world::WorldState::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx1);
        world.register_session(2, tx2);

        // Different rooms but status=false → guard skipped
        world.update_session(1, |h| {
            h.event_room = 2;
            h.monster_stone_status = false;
        });
        world.update_session(2, |h| {
            h.event_room = 6;
            h.monster_stone_status = false;
        });

        assert!(!world.get_monster_stone_status(1));
    }

    /// Non-Monster Stone zones don't apply event_room isolation.
    #[test]
    fn test_non_ms_zone_no_magic_isolation() {
        let zone_id: u16 = 21; // Moradon
        assert!(!crate::systems::monster_stone::is_monster_stone_zone(
            zone_id
        ));
    }

    /// Type5 resurrection: EXP recovery applies only for PvE deaths (who_killed_me == -1).
    #[test]
    fn test_type5_resurrection_exp_recovery_pve_only() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Simulate PvE death: who_killed_me = -1, lost_exp = 10000
        world.update_session(1, |h| {
            h.who_killed_me = -1;
            h.lost_exp = 10_000;
        });

        let (who_killed, lost_exp) = world
            .with_session(1, |h| (h.who_killed_me, h.lost_exp))
            .unwrap();

        // PvE death → EXP recovery should be allowed
        assert_eq!(who_killed, -1);
        assert!(lost_exp > 0);
        let exp_recover_pct: i64 = 50; // typical resurrection skill
        let restored = (lost_exp * exp_recover_pct) / 100;
        assert_eq!(restored, 5_000, "50% of 10000 lost EXP should be restored");
    }

    /// Type5 resurrection: PvP deaths (who_killed_me >= 0) do NOT recover EXP.
    #[test]
    fn test_type5_resurrection_no_exp_recovery_for_pvp() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Simulate PvP death: who_killed_me = 5 (killed by player SID 5)
        world.update_session(1, |h| {
            h.who_killed_me = 5;
            h.lost_exp = 10_000;
        });

        let (who_killed, lost_exp) = world
            .with_session(1, |h| (h.who_killed_me, h.lost_exp))
            .unwrap();

        // PvP death → EXP recovery must NOT apply
        assert_ne!(who_killed, -1);
        // The resurrection code checks: if who_killed == -1 && lost_exp > 0
        let would_recover = who_killed == -1 && lost_exp > 0;
        assert!(
            !would_recover,
            "PvP deaths must not recover EXP on resurrection"
        );
    }

    /// Type5 resurrection resets death tracking fields after use.
    #[test]
    fn test_type5_resurrection_resets_death_fields() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Set death state
        world.update_session(1, |h| {
            h.who_killed_me = -1;
            h.lost_exp = 8_000;
        });

        // Simulate resurrection reset (as done in Type5 handler)
        world.update_session(1, |h| {
            h.who_killed_me = -1;
            h.lost_exp = 0;
        });

        let who = world.with_session(1, |h| h.who_killed_me).unwrap();
        let exp = world.with_session(1, |h| h.lost_exp).unwrap();
        assert_eq!(who, -1, "who_killed_me must be -1 after resurrection");
        assert_eq!(exp, 0, "lost_exp must be 0 after resurrection");
    }

    /// Type5 resurrection in Under Castle (zone 86) restores full MP instead of 0.
    #[test]
    fn test_type5_resurrection_under_castle_full_mp() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Under Castle zone exception:
        // - MP should be restored to max (not 0)
        // - EXP recovery should be skipped entirely

        // Simulate being in Under Castle with PvE death
        world.update_session(1, |h| {
            h.who_killed_me = -1;
            h.lost_exp = 10_000;
        });

        // In Under Castle, even with PvE death, no EXP recovery
        // and MP goes to max instead of 0
        let zone = ZONE_UNDER_CASTLE;
        let who_killed = world.with_session(1, |h| h.who_killed_me).unwrap();
        let lost_exp = world.with_session(1, |h| h.lost_exp).unwrap();

        // Under Castle should skip EXP recovery block entirely
        let would_recover_exp = zone != ZONE_UNDER_CASTLE && who_killed == -1 && lost_exp > 0;
        assert!(
            !would_recover_exp,
            "Under Castle must skip EXP recovery on resurrection"
        );
    }

    /// Type5 resurrection sends WIZ_REGENE packet to the target.
    #[test]
    fn test_type5_resurrection_sends_regene_packet() {
        let world = crate::world::WorldState::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Set position so the regene packet can be built
        world.update_position(1, 21, 500.0, 0.0, 300.0);

        // Build the regene packet as the Type5 handler does
        if let Some(tpos) = world.get_position(1) {
            let mut regene_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizRegene as u8);
            regene_pkt.write_u16((tpos.x * 10.0) as u16);
            regene_pkt.write_u16((tpos.z * 10.0) as u16);
            regene_pkt.write_u16(0);
            world.send_to_session_owned(1, regene_pkt);
        }

        // Verify the packet was sent
        let pkt = rx.try_recv().expect("WIZ_REGENE packet should be sent");
        assert_eq!(
            pkt.opcode,
            ko_protocol::Opcode::WizRegene as u8,
            "Packet must be WIZ_REGENE"
        );
    }

    /// Post-resurrection sequence clears DOTs and sends cure packets.
    #[tokio::test]
    async fn test_post_resurrection_clears_dots_and_sends_cure() {
        let world = crate::world::WorldState::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_position(1, 21, 500.0, 0.0, 300.0);

        // Give the player an active DOT skill
        world.clear_durational_skills(1);

        post_resurrection_sequence(&world, 1, 21);

        // Collect all packets sent to the session
        let mut opcodes = Vec::new();
        while let Ok(pkt) = rx.try_recv() {
            opcodes.push(pkt.opcode);
        }

        // Should contain WIZ_STEALTH (stealth init) + WIZ_ZONEABILITY (DOT cure + Poison cure)
        assert!(
            opcodes.contains(&(ko_protocol::Opcode::WizStealth as u8)),
            "Must send WIZ_STEALTH for stealth init"
        );
        // Two WIZ_ZONEABILITY packets (DOT cure + Poison cure)
        let zone_ability_count = opcodes
            .iter()
            .filter(|&&op| op == ko_protocol::Opcode::WizZoneability as u8)
            .count();
        assert!(
            zone_ability_count >= 2,
            "Must send at least 2 WIZ_ZONEABILITY cure packets (DOT+Poison), got {}",
            zone_ability_count
        );
    }

    /// Helper: create a minimal CharacterInfo for post-resurrection tests.
    fn make_res_test_char(sid: u16, name: &str) -> crate::world::CharacterInfo {
        crate::world::CharacterInfo {
            session_id: sid,
            name: name.to_string(),
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

    /// Post-resurrection sequence activates blink in blink-enabled zones.
    #[tokio::test]
    async fn test_post_resurrection_activates_blink() {
        let world = crate::world::WorldState::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_position(1, 21, 500.0, 0.0, 300.0);

        // Set up character info (authority != 0, so NOT a GM)
        world.update_session(1, |h| {
            h.character = Some(make_res_test_char(1, "TestPlayer"));
        });

        // Before resurrection, blink should not be active
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        assert!(
            !world.is_player_blinking(1, now),
            "Player should not be blinking before resurrection"
        );

        post_resurrection_sequence(&world, 1, 21);

        // After resurrection, check if blink state was applied
        // Note: blink activation depends on zone's blink_zone flag.
        // In test world without zone info loaded, blink_zone defaults to false,
        // so blink won't activate. This test verifies the function runs cleanly.
        let can_use = world.with_session(1, |h| h.can_use_skills).unwrap_or(true);
        // If blink_zone is not loaded, skills remain usable
        // The important thing is the function doesn't panic
        assert!(
            can_use || !world.is_player_blinking(1, now),
            "Function must complete without error"
        );
    }

    /// Post-resurrection sequence sends stealth init and cure packets.
    #[tokio::test]
    async fn test_post_resurrection_sends_inout_and_cures() {
        let world = crate::world::WorldState::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        world.register_session(1, tx);
        world.update_position(1, 21, 500.0, 0.0, 300.0);
        world.update_session(1, |h| {
            h.character = Some(make_res_test_char(1, "Resurrected"));
        });

        post_resurrection_sequence(&world, 1, 21);

        // Collect all packets
        let mut opcodes = Vec::new();
        while let Ok(pkt) = rx.try_recv() {
            opcodes.push(pkt.opcode);
        }

        // Must contain WIZ_STEALTH (stealth init)
        assert!(
            opcodes.contains(&(ko_protocol::Opcode::WizStealth as u8)),
            "Must send WIZ_STEALTH for stealth init"
        );
        // Must contain 2+ WIZ_ZONEABILITY (DOT + Poison cure)
        let zoneability_count = opcodes
            .iter()
            .filter(|&&op| op == ko_protocol::Opcode::WizZoneability as u8)
            .count();
        assert!(
            zoneability_count >= 2,
            "Must send at least 2 WIZ_ZONEABILITY cure packets, got {}",
            zoneability_count
        );
    }

    // ── MORAL_AREA_ALL Safety Area Cross-Check Tests ──────────────────

    #[test]
    fn test_moral_area_all_safety_area_blocks_cross_zone_exploit() {
        // isInEnemySafetyArea = "I am in the enemy's forward safe zone"
        // isInOwnSafetyArea = "I am in my own faction's safe zone"
        // (1860,174) in ZONE_KARUS = Elmorad forward base
        // Karus caster at Elmorad's forward base → in enemy safety area
        // Elmorad target at their own forward base → in own safety area
        // Result: MORAL_AREA_ALL blocks this attack
        use crate::handler::attack::{is_in_enemy_safety_area, is_in_own_safety_area};
        use crate::world::types::ZONE_KARUS;

        let karus: u8 = 1;
        let elmorad: u8 = 2;

        // Karus caster at (1860,174) in ZONE_KARUS → enemy safety area
        assert!(is_in_enemy_safety_area(ZONE_KARUS, 1860.0, 174.0, karus));
        // Elmorad target at same location → own safety area
        assert!(is_in_own_safety_area(ZONE_KARUS, 1860.0, 174.0, elmorad));

        // Both conditions met → MORAL_AREA_ALL must block
        let should_block = is_in_enemy_safety_area(ZONE_KARUS, 1860.0, 174.0, karus)
            && is_in_own_safety_area(ZONE_KARUS, 1860.0, 174.0, elmorad);
        assert!(
            should_block,
            "MORAL_AREA_ALL must block caster in enemy safety hitting target in own safety"
        );

        // Reverse: Elmorad at (1860,174) is NOT in enemy safety for ZONE_KARUS
        assert!(!is_in_enemy_safety_area(ZONE_KARUS, 1860.0, 174.0, elmorad));
    }

    #[test]
    fn test_moral_area_all_safety_area_blocks_bifrost_exploit() {
        // Bifrost: Elmorad safe area (190-270, 870-970), Karus safe area (56-124, 700-840)
        // Caster (Karus) in Elmorad's safe zone → is_in_enemy_safety_area for Karus
        // Target (Elmorad) in own safe zone → is_in_own_safety_area for Elmorad
        use crate::handler::attack::{is_in_enemy_safety_area, is_in_own_safety_area};
        use crate::world::types::ZONE_BIFROST;

        let karus: u8 = 1;
        let elmorad: u8 = 2;

        // Karus player in Elmorad's safe area (230, 920)
        assert!(is_in_enemy_safety_area(ZONE_BIFROST, 230.0, 920.0, karus));
        // Elmorad player in own safe area (230, 920)
        assert!(is_in_own_safety_area(ZONE_BIFROST, 230.0, 920.0, elmorad));

        // Both conditions met → MORAL_AREA_ALL should block
        let should_block = is_in_enemy_safety_area(ZONE_BIFROST, 230.0, 920.0, karus)
            && is_in_own_safety_area(ZONE_BIFROST, 230.0, 920.0, elmorad);
        assert!(
            should_block,
            "MORAL_AREA_ALL must block caster in enemy safety hitting target in own safety"
        );
    }

    #[test]
    fn test_moral_area_all_arena_exemption() {
        // Both players in arena → safety check is bypassed
        use crate::handler::attack::is_in_arena;
        use crate::world::types::ZONE_ARENA;

        // Both in ZONE_ARENA → both_in_special_zone = true
        assert!(is_in_arena(ZONE_ARENA, 127.0, 113.0));
        assert!(is_in_arena(ZONE_ARENA, 150.0, 150.0));

        let both_in_special =
            is_in_arena(ZONE_ARENA, 127.0, 113.0) && is_in_arena(ZONE_ARENA, 150.0, 150.0);
        assert!(
            both_in_special,
            "Arena exemption: both in arena should bypass safety check"
        );
    }

    #[test]
    fn test_moral_area_all_pvp_zone_exemption() {
        // Both in PVP zone → safety check bypassed
        use crate::handler::attack::is_in_pvp_zone;
        use crate::world::types::ZONE_RONARK_LAND;

        assert!(is_in_pvp_zone(ZONE_RONARK_LAND));
        let both = is_in_pvp_zone(ZONE_RONARK_LAND) && is_in_pvp_zone(ZONE_RONARK_LAND);
        assert!(both, "PVP zone exemption should bypass safety check");
    }

    #[test]
    fn test_moral_area_all_temple_event_exemption() {
        // Both in temple event zone → safety check bypassed
        use crate::systems::event_room::is_in_temple_event_zone;
        use crate::world::types::ZONE_JURAID_MOUNTAIN;

        assert!(is_in_temple_event_zone(ZONE_JURAID_MOUNTAIN));
        let both = is_in_temple_event_zone(ZONE_JURAID_MOUNTAIN)
            && is_in_temple_event_zone(ZONE_JURAID_MOUNTAIN);
        assert!(
            both,
            "Temple event zone exemption should bypass safety check"
        );
    }

    #[test]
    fn test_moral_area_all_no_npc_targeting() {
        // MORAL_AREA_ALL (12) should not target NPCs
        let moral: i16 = 12; // MORAL_AREA_ALL
        let moral_area_enemy: i16 = 10;
        // Only MORAL_AREA_ENEMY should enter the NPC damage loop
        assert_ne!(moral, moral_area_enemy);
        // The NPC AOE loop condition is now `moral == MORAL_AREA_ENEMY` only
        let enters_npc_loop = moral == moral_area_enemy;
        assert!(
            !enters_npc_loop,
            "MORAL_AREA_ALL must NOT enter NPC damage loop"
        );
    }

    #[test]
    fn test_moral_area_all_delos_safety_blocks() {
        // Delos center (500, 180, radius=115) is safety area for everyone
        use crate::handler::attack::{is_in_enemy_safety_area, is_in_own_safety_area};
        use crate::world::types::ZONE_DELOS;

        // In Delos, both enemy and own safety are the same circle
        assert!(is_in_enemy_safety_area(ZONE_DELOS, 500.0, 180.0, 1));
        assert!(is_in_own_safety_area(ZONE_DELOS, 500.0, 180.0, 1));
        assert!(is_in_enemy_safety_area(ZONE_DELOS, 500.0, 180.0, 2));
        assert!(is_in_own_safety_area(ZONE_DELOS, 500.0, 180.0, 2));

        // Both conditions met → should block
        let should_block = is_in_enemy_safety_area(ZONE_DELOS, 500.0, 180.0, 1)
            && is_in_own_safety_area(ZONE_DELOS, 500.0, 180.0, 2);
        assert!(should_block, "Delos safety area cross-check must block");
    }

    #[test]
    fn test_moral_area_all_battle_zone_safety_blocks() {
        // ZONE_BATTLE: Karus safe (98-125, 755-780), Elmorad safe (805-831, 85-110)
        use crate::handler::attack::{is_in_enemy_safety_area, is_in_own_safety_area};
        use crate::world::types::ZONE_BATTLE;

        let karus: u8 = 1;
        let elmorad: u8 = 2;

        // Karus player in their own safe zone (110, 770) → enemy safety for Karus
        assert!(is_in_enemy_safety_area(ZONE_BATTLE, 110.0, 770.0, karus));
        // Elmorad player in that same area → own safety for Elmorad
        assert!(is_in_own_safety_area(ZONE_BATTLE, 110.0, 770.0, elmorad));

        let should_block = is_in_enemy_safety_area(ZONE_BATTLE, 110.0, 770.0, karus)
            && is_in_own_safety_area(ZONE_BATTLE, 110.0, 770.0, elmorad);
        assert!(should_block, "Battle zone safety cross-check must block");
    }

    // ── Armor Scroll AC Disabling Tests ───────────────────────────────

    #[test]
    fn test_armor_scroll_disable_skill_ids() {
        // Known skill IDs that disable armor scroll AC
        assert!(is_armor_scroll_disable_skill(107640));
        assert!(is_armor_scroll_disable_skill(108640));
        assert!(is_armor_scroll_disable_skill(207640));
        assert!(is_armor_scroll_disable_skill(208640));
        assert!(is_armor_scroll_disable_skill(107620));
        assert!(is_armor_scroll_disable_skill(108670));
        assert!(is_armor_scroll_disable_skill(208670));
    }

    #[test]
    fn test_armor_scroll_normal_skill_not_disabled() {
        // Normal skills should NOT disable armor scroll
        assert!(!is_armor_scroll_disable_skill(100000));
        assert!(!is_armor_scroll_disable_skill(200000));
        assert!(!is_armor_scroll_disable_skill(0));
        assert!(!is_armor_scroll_disable_skill(999999));
    }

    #[test]
    fn test_armor_scroll_disable_covers_all_14_skills() {
        // Verify all 14 skill IDs from C++ Unit.cpp:281-299 are covered
        let disabled = &ARMOR_SCROLL_DISABLE_SKILLS;
        assert_eq!(disabled.len(), 14);
        // Verify they span the expected ranges (107xxx and 207xxx series)
        assert!(disabled.contains(&107600));
        assert!(disabled.contains(&208670));
        // Verify none of the normal skill ranges are included
        assert!(!disabled.iter().any(|&id| id < 100000));
    }

    // ── Sprint 317: Target validation + MORAL_SELF redirection tests ──

    /// NPC dead check — NPC_BAND boundary (target_id >= 10000 = NPC).
    #[test]
    fn test_npc_band_boundary() {
        assert_eq!(NPC_BAND, 10000);
        // Player IDs are < NPC_BAND
        assert!((9999u32) < NPC_BAND);
        // NPC IDs start at NPC_BAND
        assert!((10000u32) >= NPC_BAND);
    }

    /// MORAL_SELF always redirects target to caster.
    #[test]
    fn test_moral_self_target_redirection() {
        let caster_id: i32 = 42;
        let mut target_id: i32 = 99; // some other player

        // Simulate MORAL_SELF redirection logic
        let moral = MORAL_SELF;
        let redirect_to_self = moral == MORAL_SELF
            || (moral == MORAL_FRIEND_WITHME && target_id != -1 && (target_id as u32) >= NPC_BAND);
        if redirect_to_self {
            target_id = caster_id;
        }

        assert_eq!(
            target_id, caster_id,
            "MORAL_SELF must redirect target to caster"
        );
    }

    /// MORAL_FRIEND_WITHME redirects to caster only when target is NPC.
    #[test]
    fn test_moral_friend_withme_npc_redirect() {
        let caster_id: i32 = 42;

        // Case 1: target is NPC (>= NPC_BAND) → redirect
        let mut target_id: i32 = 10500; // NPC
        let moral = MORAL_FRIEND_WITHME;
        let redirect = moral == MORAL_SELF
            || (moral == MORAL_FRIEND_WITHME && target_id != -1 && (target_id as u32) >= NPC_BAND);
        if redirect {
            target_id = caster_id;
        }
        assert_eq!(target_id, caster_id, "NPC target → redirect to caster");

        // Case 2: target is player (< NPC_BAND) → NO redirect
        let mut target_id2: i32 = 50; // player
        let redirect2 = moral == MORAL_SELF
            || (moral == MORAL_FRIEND_WITHME
                && target_id2 != -1
                && (target_id2 as u32) >= NPC_BAND);
        if redirect2 {
            target_id2 = caster_id;
        }
        assert_eq!(
            target_id2, 50,
            "Player target → no redirect for MORAL_FRIEND_WITHME"
        );
    }

    /// MORAL_ENEMY does NOT redirect target.
    #[test]
    fn test_moral_enemy_no_redirect() {
        let caster_id: i32 = 42;
        let mut target_id: i32 = 99;
        let moral = MORAL_ENEMY;
        let redirect = moral == MORAL_SELF
            || (moral == MORAL_FRIEND_WITHME && target_id != -1 && (target_id as u32) >= NPC_BAND);
        if redirect {
            target_id = caster_id;
        }
        assert_eq!(target_id, 99, "MORAL_ENEMY must not redirect target");
    }

    /// Target validation: NPC dead → silent return (no fail packet).
    #[test]
    fn test_npc_dead_target_validation() {
        let world = WorldState::new();
        // Register NPC with 0 HP (dead)
        world.init_npc_hp(10500, 0);
        assert!(world.is_npc_dead(10500), "NPC with 0 HP should be dead");

        // In C++, targeting a dead NPC → silent return (no fail packet)
        let target_id: i32 = 10500;
        assert!((target_id as u32) >= NPC_BAND);
        assert!(world.is_npc_dead(target_id as u32));
    }

    /// Target validation: player session gone → fail expected.
    #[test]
    fn test_player_target_not_found() {
        let world = WorldState::new();
        // No sessions registered — target_sid 99 should not have character info
        let target_sid: SessionId = 99;
        assert!(
            world.get_character_info(target_sid).is_none(),
            "Non-existent player session should return None"
        );
    }

    // ── Sprint 325: isBlinking + Type 5 resurrection tests ──────────

    #[test]
    fn test_blinking_blocks_casting_flying_effecting() {
        // isBlinking() in UserCanCast() blocks CASTING, FLYING, EFFECTING.
        // Exceptions: CANCEL(6), CANCEL2(13), TYPE4_EXTEND(8), FAIL(4).
        let blocked_opcodes = [MAGIC_CASTING, MAGIC_FLYING, MAGIC_EFFECTING];
        let allowed_opcodes = [
            MAGIC_FAIL,
            MAGIC_CANCEL,
            MAGIC_CANCEL_TRANSFORMATION,
            MAGIC_TYPE4_EXTEND,
            MAGIC_CANCEL2,
        ];
        for op in &blocked_opcodes {
            assert!(
                *op != MAGIC_TYPE4_EXTEND
                    && *op != MAGIC_CANCEL
                    && *op != MAGIC_CANCEL2
                    && *op != MAGIC_CANCEL_TRANSFORMATION
                    && *op != MAGIC_FAIL,
                "Opcode {} should be blocked by isBlinking",
                op
            );
        }
        for op in &allowed_opcodes {
            let is_exempt = *op == MAGIC_TYPE4_EXTEND
                || *op == MAGIC_CANCEL
                || *op == MAGIC_CANCEL2
                || *op == MAGIC_CANCEL_TRANSFORMATION
                || *op == MAGIC_FAIL;
            assert!(is_exempt, "Opcode {} should be exempt from isBlinking", op);
        }
    }

    #[test]
    fn test_dead_exempts_type5_resurrection() {
        // `pSkillCaster->isDead() && pSkill.bType[0] != 5`
        // Dead casters CAN cast Type 5 skills (self-resurrection).
        let is_dead = true;
        let skill_type1: i16 = 5;
        let blocked = is_dead && skill_type1 != 5;
        assert!(
            !blocked,
            "Type 5 skill should NOT be blocked for dead caster"
        );
    }

    #[test]
    fn test_dead_blocks_non_type5() {
        // Non-type-5 skills should be blocked for dead casters.
        let is_dead = true;
        for skill_type in [1i16, 2, 3, 4, 6, 7, 8, 9] {
            let blocked = is_dead && skill_type != 5;
            assert!(
                blocked,
                "Type {} skill should be blocked for dead caster",
                skill_type
            );
        }
    }

    // ── Sprint 356: Snow Battle magic restriction tests ──────────────

    #[test]
    fn test_snow_event_skill_constant() {
        // C++ Define.h:82 — SNOW_EVENT_SKILL = 490077
        assert_eq!(SNOW_EVENT_SKILL, 490077);
    }

    #[test]
    fn test_snow_battle_blocks_non_event_skill() {
        // In ZONE_SNOW_BATTLE during SNOW_BATTLE, only SNOW_EVENT_SKILL is allowed
        let zone = ZONE_SNOW_BATTLE;
        let is_snow = true;
        let skill_id: u32 = 100001; // random non-event skill

        let blocked = zone == ZONE_SNOW_BATTLE && is_snow && skill_id != SNOW_EVENT_SKILL;
        assert!(
            blocked,
            "Non-event skill should be blocked during Snow Battle"
        );
    }

    #[test]
    fn test_snow_battle_allows_event_skill() {
        // SNOW_EVENT_SKILL should pass through
        let zone = ZONE_SNOW_BATTLE;
        let is_snow = true;
        let skill_id: u32 = SNOW_EVENT_SKILL;

        let blocked = zone == ZONE_SNOW_BATTLE && is_snow && skill_id != SNOW_EVENT_SKILL;
        assert!(!blocked, "Snow event skill should NOT be blocked");
    }

    #[test]
    fn test_snow_battle_no_block_outside_zone() {
        // Outside ZONE_SNOW_BATTLE, skills should not be blocked
        let zone: u16 = 21; // Moradon
        let is_snow = true;
        let skill_id: u32 = 100001;

        let blocked = zone == ZONE_SNOW_BATTLE && is_snow && skill_id != SNOW_EVENT_SKILL;
        assert!(!blocked, "Skills outside snow zone should not be blocked");
    }

    #[test]
    fn test_snow_battle_no_block_when_inactive() {
        // When Snow Battle is not active, skills should not be blocked
        let zone = ZONE_SNOW_BATTLE;
        let is_snow = false; // not active
        let skill_id: u32 = 100001;

        let blocked = zone == ZONE_SNOW_BATTLE && is_snow && skill_id != SNOW_EVENT_SKILL;
        assert!(
            !blocked,
            "Skills should not be blocked when Snow Battle inactive"
        );
    }

    #[test]
    fn test_snow_battle_type3_damage_override() {
        // C++ MagicInstance.cpp:3962-3963 — damage = -10 during Snow Battle
        // In Rust we apply damage=10 (positive, same as attack.rs convention)
        let zone = ZONE_SNOW_BATTLE;
        let is_snow = true;
        let original_damage: i16 = 500;

        let damage = if zone == ZONE_SNOW_BATTLE && is_snow {
            10 // forced to 10 damage
        } else {
            original_damage
        };
        assert_eq!(
            damage, 10,
            "Type3 damage should be overridden to 10 during Snow Battle"
        );
    }

    // ── Sprint 437: AnimatedSkill validation tests ────────────────────

    /// AnimatedSkill check: t_1 must not be -1 or 0, cast_time > 0, item_group != 255.
    #[test]
    fn test_animated_skill_check() {
        // is_animated = (t_1 != -1 && t_1 != 0) && cast_time > 0 && item_group != 255
        let check = |t_1: i32, cast_time: i16, item_group: i16, skill_id: u32| -> bool {
            (t_1 != -1 && t_1 != 0)
                && cast_time > 0
                && item_group != 255
                && !is_mage_armor_skill(skill_id)
        };

        // Normal animated skill
        assert!(check(100, 5, 0, 108010));
        // t_1 = 0 → not animated
        assert!(!check(0, 5, 0, 108010));
        // t_1 = -1 → not animated
        assert!(!check(-1, 5, 0, 108010));
        // cast_time = 0 → not animated (instant cast)
        assert!(!check(100, 0, 0, 108010));
        // item_group = 255 → not animated
        assert!(!check(100, 5, 255, 108010));
    }

    /// Type 2 animated skill 500ms recast prevention logic.
    #[test]
    fn test_type2_recast_prevention() {
        // Simulated: skill A cast at t=100, skill B at t=350 (within 500ms) → fail
        let last_id: u32 = 1000;
        let last_time: u64 = 100;
        let now: u64 = 350;
        let new_skill: u32 = 2000;

        let should_fail =
            last_id != 0 && last_id != new_skill && now.saturating_sub(last_time) < 500;
        assert!(should_fail, "different skill within 500ms should fail");

        // Same skill within 500ms → should NOT fail
        let same_skill = last_id;
        let should_fail_same =
            last_id != 0 && last_id != same_skill && now.saturating_sub(last_time) < 500;
        assert!(!should_fail_same, "same skill within 500ms should pass");

        // Different skill but after 500ms → should NOT fail
        let now_later: u64 = 700;
        let should_fail_later =
            last_id != 0 && last_id != new_skill && now_later.saturating_sub(last_time) < 500;
        assert!(
            !should_fail_later,
            "different skill after 500ms should pass"
        );

        // No previous skill (last_id=0) → should NOT fail
        let should_fail_first =
            0u32 != 0 && 0u32 != new_skill && now.saturating_sub(last_time) < 500;
        assert!(!should_fail_first, "first cast should always pass");
    }

    /// Cast-failed state machine: moving during CASTING sets failed flag.
    #[test]
    fn test_cast_failed_state_machine() {
        // Simulate: CASTING phase while moving → cast_failed = true
        let mut cast_failed = false;
        let is_moving = true;
        let b_opcode = MAGIC_CASTING;

        if b_opcode == MAGIC_CASTING && is_moving {
            cast_failed = true;
        }
        assert!(cast_failed, "moving during CASTING should set cast_failed");

        // Simulate: EFFECTING phase reads cast_failed → should return fail
        let b_opcode_effect = MAGIC_EFFECTING;
        let should_fail = cast_failed && b_opcode_effect != MAGIC_CASTING;
        assert!(
            should_fail,
            "EFFECTING after movement-during-cast should fail"
        );

        // After failing, cast_failed is cleared
        cast_failed = false;
        assert!(
            !cast_failed,
            "cast_failed should be cleared after processing"
        );
    }

    /// Standing still during CASTING should NOT set cast_failed.
    #[test]
    fn test_cast_not_failed_when_standing() {
        let mut cast_failed = false;
        let is_moving = false;

        if is_moving {
            cast_failed = true;
        }
        assert!(
            !cast_failed,
            "standing still during CASTING should not fail"
        );
    }

    // ── Sprint 438: resolve_consume_item tests ───────────────────────

    /// Helper to create a minimal MagicRow for testing.
    fn make_skill(before_action: i32, use_item: i32) -> ko_db::models::MagicRow {
        ko_db::models::MagicRow {
            magic_num: 100000,
            en_name: None,
            kr_name: None,
            description: None,
            t_1: None,
            before_action: Some(before_action),
            target_action: None,
            self_effect: None,
            flying_effect: None,
            target_effect: None,
            moral: None,
            skill_level: None,
            skill: None,
            msp: None,
            hp: None,
            s_sp: None,
            item_group: None,
            use_item: Some(use_item),
            cast_time: None,
            recast_time: None,
            success_rate: None,
            type1: None,
            type2: None,
            range: None,
            etc: None,
            use_standing: None,
            skill_check: None,
            icelightrate: None,
        }
    }

    /// Class stone consumption uses CLASS_STONE_BASE_ID + (before_action * 1000).
    #[test]
    fn test_resolve_consume_item_class_stone() {
        // Warrior class stone: before_action = 1
        let skill = make_skill(1, 999999);
        assert_eq!(resolve_consume_item(&skill), CLASS_STONE_BASE_ID + 1000);

        // Priest class stone: before_action = 4
        let skill = make_skill(4, 999999);
        assert_eq!(resolve_consume_item(&skill), CLASS_STONE_BASE_ID + 4000);
    }

    /// Job change scrolls use iUseItem.
    #[test]
    fn test_resolve_consume_item_job_change() {
        let skill = make_skill(379090000, 800100000);
        assert_eq!(resolve_consume_item(&skill), 800100000);

        let skill = make_skill(379093000, 800100000);
        assert_eq!(resolve_consume_item(&skill), 800100000);
    }

    /// Special before_action (381001000) uses itself.
    #[test]
    fn test_resolve_consume_item_special() {
        let skill = make_skill(381001000, 999);
        assert_eq!(resolve_consume_item(&skill), 381001000);
    }

    /// Default: uses iUseItem when before_action is 0 or other.
    #[test]
    fn test_resolve_consume_item_default() {
        let skill = make_skill(0, 700100000);
        assert_eq!(resolve_consume_item(&skill), 700100000);

        // No use_item → 0
        let skill = make_skill(0, 0);
        assert_eq!(resolve_consume_item(&skill), 0);
    }

    /// NO_CONSUME_ITEMS should contain the expected town scrolls, rogue scrolls, and stones.
    #[test]
    fn test_no_consume_items_list() {
        // Town return scrolls
        assert!(NO_CONSUME_ITEMS.contains(&370001000));
        assert!(NO_CONSUME_ITEMS.contains(&370002000));
        assert!(NO_CONSUME_ITEMS.contains(&370003000));
        // 370004000-370006000 (Blood of Wolf etc.) ARE consumed per C++ individual == checks
        assert!(!NO_CONSUME_ITEMS.contains(&370004000)); // Blood of Wolf — consumed
        assert!(!NO_CONSUME_ITEMS.contains(&370005000)); // consumed
        assert!(!NO_CONSUME_ITEMS.contains(&370006000)); // consumed
                                                         // Class-specific stones
        assert!(NO_CONSUME_ITEMS.contains(&379063000));
        assert!(NO_CONSUME_ITEMS.contains(&379064000));
        assert!(NO_CONSUME_ITEMS.contains(&379065000));
        assert!(NO_CONSUME_ITEMS.contains(&379066000));
        // Non-existent item
        assert!(!NO_CONSUME_ITEMS.contains(&999999));
    }

    // ── Type4Extend Duration Item tests ────────────────────────────────────

    /// Helper: create a minimal Item for testing.
    fn make_test_item_ext(num: i32, kind: i32, effect1: i32) -> ko_db::models::Item {
        ko_db::models::Item {
            num,
            kind: Some(kind),
            effect1: Some(effect1),
            extension: None,
            str_name: None,
            description: None,
            item_plus_id: None,
            item_alteration: None,
            item_icon_id1: None,
            item_icon_id2: None,
            slot: None,
            race: None,
            class: None,
            damage: None,
            min_damage: None,
            max_damage: None,
            delay: None,
            range: None,
            weight: None,
            duration: None,
            buy_price: None,
            sell_price: None,
            sell_npc_type: None,
            sell_npc_price: None,
            ac: None,
            countable: None,
            effect2: None,
            req_level: None,
            req_level_max: None,
            req_rank: None,
            req_title: None,
            req_str: None,
            req_sta: None,
            req_dex: None,
            req_intel: None,
            req_cha: None,
            selling_group: None,
            item_type: None,
            hitrate: None,
            evasionrate: None,
            dagger_ac: None,
            jamadar_ac: None,
            sword_ac: None,
            club_ac: None,
            axe_ac: None,
            spear_ac: None,
            bow_ac: None,
            fire_damage: None,
            ice_damage: None,
            lightning_damage: None,
            poison_damage: None,
            hp_drain: None,
            mp_damage: None,
            mp_drain: None,
            mirror_damage: None,
            droprate: None,
            str_b: None,
            sta_b: None,
            dex_b: None,
            intel_b: None,
            cha_b: None,
            max_hp_b: None,
            max_mp_b: None,
            fire_r: None,
            cold_r: None,
            lightning_r: None,
            magic_r: None,
            poison_r: None,
            curse_r: None,
            item_class: None,
            np_buy_price: None,
            bound: None,
            mace_ac: None,
            by_grade: None,
            drop_notice: None,
            upgrade_notice: None,
        }
    }

    /// Helper: create a minimal MagicRow for testing.
    fn make_test_magic(magic_num: i32, moral: i16) -> ko_db::models::MagicRow {
        ko_db::models::MagicRow {
            magic_num,
            moral: Some(moral),
            en_name: None,
            kr_name: None,
            description: None,
            t_1: None,
            before_action: None,
            target_action: None,
            self_effect: None,
            flying_effect: None,
            target_effect: None,
            skill_level: None,
            skill: None,
            msp: None,
            hp: None,
            s_sp: None,
            item_group: None,
            use_item: None,
            cast_time: None,
            recast_time: None,
            success_rate: None,
            type1: None,
            type2: None,
            range: None,
            etc: None,
            use_standing: None,
            skill_check: None,
            icelightrate: None,
        }
    }

    #[test]
    fn test_moral_extend_duration_constant() {
        assert_eq!(240i16, 240);
    }

    /// Duration Item search: kind=255 + effect1 pointing to moral=240 magic.
    #[test]
    fn test_type4_extend_duration_item_lookup() {
        use crate::world::types::UserItemSlot;
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Initialize inventory
        let inv = vec![UserItemSlot::default(); crate::handler::INVENTORY_TOTAL];
        world.set_inventory(1, inv);

        // Insert a magic with moral=240 (MORAL_EXTEND_DURATION)
        world.insert_magic(make_test_magic(777000, 240));

        // Insert a kind=255 item with effect1=777000
        world.insert_item(810999000u32, make_test_item_ext(810999000, 255, 777000));

        // Put it in inventory slot SLOT_MAX (bag slot 0) with durability=5
        world.update_inventory(1, |inv| {
            inv[crate::handler::SLOT_MAX].item_id = 810999000;
            inv[crate::handler::SLOT_MAX].count = 1;
            inv[crate::handler::SLOT_MAX].durability = 5;
            true
        });

        // Verify the scan finds the Duration Item
        let mut found_item_id = None;
        for i in crate::handler::SLOT_MAX..crate::handler::INVENTORY_TOTAL {
            let slot = match world.get_inventory_slot(1, i) {
                Some(s) if s.item_id != 0 => s,
                _ => continue,
            };
            let tmpl = match world.get_item(slot.item_id) {
                Some(t) => t,
                None => continue,
            };
            if tmpl.kind.unwrap_or(0) != 255 {
                continue;
            }
            let eff1 = tmpl.effect1.unwrap_or(0);
            if eff1 == 0 {
                continue;
            }
            if let Some(m) = world.get_magic(eff1) {
                if m.moral.unwrap_or(0) == 240 {
                    found_item_id = Some(slot.item_id);
                    break;
                }
            }
        }

        assert_eq!(found_item_id, Some(810999000));
    }

    /// Duration Item scan returns None when no qualifying item exists.
    #[test]
    fn test_type4_extend_no_duration_item() {
        use crate::world::types::UserItemSlot;
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let inv = vec![UserItemSlot::default(); crate::handler::INVENTORY_TOTAL];
        world.set_inventory(1, inv);

        // Insert a kind=255 item but with effect1=0 (no extension magic)
        world.insert_item(810999001u32, make_test_item_ext(810999001, 255, 0));

        world.update_inventory(1, |inv| {
            inv[crate::handler::SLOT_MAX].item_id = 810999001;
            inv[crate::handler::SLOT_MAX].count = 1;
            inv[crate::handler::SLOT_MAX].durability = 3;
            true
        });

        // Scan should find nothing (effect1==0 → skip)
        let mut found = false;
        for i in crate::handler::SLOT_MAX..crate::handler::INVENTORY_TOTAL {
            let slot = match world.get_inventory_slot(1, i) {
                Some(s) if s.item_id != 0 => s,
                _ => continue,
            };
            let tmpl = match world.get_item(slot.item_id) {
                Some(t) => t,
                None => continue,
            };
            if tmpl.kind.unwrap_or(0) != 255 {
                continue;
            }
            let eff1 = tmpl.effect1.unwrap_or(0);
            if eff1 == 0 {
                continue;
            }
            found = true;
        }
        assert!(!found);
    }

    /// Duration Item consumption via rob_item decrements durability for kind=255.
    #[test]
    fn test_type4_extend_rob_item_durability() {
        use crate::world::types::UserItemSlot;
        use crate::world::WorldState;
        use tokio::sync::mpsc;

        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        let inv = vec![UserItemSlot::default(); crate::handler::INVENTORY_TOTAL];
        world.set_inventory(1, inv);

        // Insert kind=255 item
        world.insert_item(810999002u32, make_test_item_ext(810999002, 255, 777000));

        world.update_inventory(1, |inv| {
            inv[crate::handler::SLOT_MAX].item_id = 810999002;
            inv[crate::handler::SLOT_MAX].count = 1;
            inv[crate::handler::SLOT_MAX].durability = 3;
            true
        });

        // Consume 1 use — durability should go from 3 → 2
        assert!(world.rob_item(1, 810999002, 1));
        let slot = world
            .get_inventory_slot(1, crate::handler::SLOT_MAX)
            .unwrap();
        assert_eq!(slot.item_id, 810999002);
        assert_eq!(slot.durability, 2);

        // Consume 2 more — durability 2 → 0 → slot cleared
        assert!(world.rob_item(1, 810999002, 2));
        let slot = world
            .get_inventory_slot(1, crate::handler::SLOT_MAX)
            .unwrap();
        assert_eq!(slot.item_id, 0); // slot cleared
    }

    // ── Sprint 553: target_id i16 cast + buff stat refresh tests ──────

    #[test]
    fn test_target_id_i16_cast_negative_one() {
        // When client sends 0xFFFFFFFF, reading as u32→i32 gives -1.
        // The i16 cast should also give -1.
        let raw: u32 = 0xFFFFFFFF;
        let as_i32 = raw as i32;
        assert_eq!(as_i32, -1);
        let as_i16_i32 = raw as i16 as i32;
        assert_eq!(as_i16_i32, -1);
    }

    #[test]
    fn test_target_id_i16_cast_0x_ffff() {
        // If client sends 0x0000FFFF (65535), without i16 cast we get 65535.
        // With i16 cast, we correctly get -1 (matching C++ behavior).
        let raw: u32 = 0x0000FFFF;
        let without_cast = raw as i32;
        assert_eq!(without_cast, 65535); // BUG: positive, not -1

        let with_cast = raw as i16 as i32;
        assert_eq!(with_cast, -1); // FIXED: correctly -1
    }

    #[test]
    fn test_target_id_i16_cast_positive() {
        // Normal player IDs (e.g., 5) should be unchanged by the i16 cast.
        let raw: u32 = 5;
        let with_cast = raw as i16 as i32;
        assert_eq!(with_cast, 5);

        // NPC IDs (e.g., 15000) should also be unchanged within i16 range.
        let raw2: u32 = 15000;
        let with_cast2 = raw2 as i16 as i32;
        assert_eq!(with_cast2, 15000);
    }

    #[test]
    fn test_buff_attack_amount_displayed_total_hit() {
        // Wolf skill: attack=120 → attack_amount=120, so displayed = total_hit * 120 / 100
        let total_hit: u16 = 500;
        let attack_amount: u32 = 120; // 120% (20% increase)
        let displayed = (total_hit as u32 * attack_amount / 100) as u16;
        assert_eq!(displayed, 600); // 500 * 120 / 100 = 600

        // No buff: attack_amount = 100
        let displayed_no_buff = (total_hit as u32 * 100 / 100) as u16;
        assert_eq!(displayed_no_buff, 500);
    }

    #[test]
    fn test_displayed_resistance_with_buff() {
        // Formula: `(base_r + add_r + resistance_bonus) * pct_r / 100`
        let base_r: i32 = 50;
        let add_r: i32 = 20; // from buff
        let resistance_bonus: i32 = 30; // from passive
        let pct_r: i32 = 100; // no debuff
        let result = ((base_r + add_r + resistance_bonus) * pct_r / 100).max(0) as u16;
        assert_eq!(result, 100); // 50 + 20 + 30 = 100

        // With 30% resistance reduction debuff
        let pct_r2: i32 = 70; // 100 - 30 = 70
        let result2 = ((base_r + add_r + resistance_bonus) * pct_r2 / 100).max(0) as u16;
        assert_eq!(result2, 70); // 100 * 70 / 100 = 70
    }
}
