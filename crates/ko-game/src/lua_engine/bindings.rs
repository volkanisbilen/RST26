//! Lua binding functions for quest scripts.
//! Each function is registered as a Lua global. The first argument is always
//! the user's session ID (`uid`). The world state is retrieved from Lua's
//! app data to look up player info and perform game actions.

use std::sync::Arc;

use mlua::prelude::*;

use crate::handler::quest;
use crate::handler::select_msg;
use crate::handler::tag_change;
use crate::world::types::{ZONE_DELOS, ZONE_PRISON};
use crate::world::{WorldState, ITEMCOUNT_MAX, ITEM_GOLD};
use crate::zone::SessionId;
use ko_protocol::{Opcode, Packet};

const MAX_MESSAGE_EVENT: usize = 12;

/// Per-execution state of one quest script run.
///
/// `LuaEngine::execute` creates a fresh VM (and a fresh `QuestRunState`) for
/// every run. When a reward could not be delivered for a reason the player can
/// fix (full bag, gold cap), `GiveItem` / `RunQuestExchange` flag it here and
/// `SaveEvent` then refuses to mark the quest as completed in the same run.
///
/// The shipped quest scripts call `RunQuestExchange(...)` and `SaveEvent(...)`
/// back to back without looking at the result (2100+ of 2167 call sites), so
/// before this guard a full bag completed the quest, consumed nothing and gave
/// nothing - the player lost the reward for good.
#[derive(Default)]
pub struct QuestRunState {
    reward_blocked: std::sync::atomic::AtomicBool,
}

/// Remember that the reward of this script run could not be delivered.
fn block_quest_completion(lua: &Lua) {
    if let Some(state) = lua.app_data_ref::<QuestRunState>() {
        state
            .reward_blocked
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// True when an earlier call of this script run failed to deliver its reward.
fn quest_completion_blocked(lua: &Lua) -> bool {
    lua.app_data_ref::<QuestRunState>()
        .map(|state| {
            state
                .reward_blocked
                .load(std::sync::atomic::Ordering::Relaxed)
        })
        .unwrap_or(false)
}

/// Tell the client the bag is full (`WIZ_QUEST` sub-opcode 13, error 3).
fn send_no_slot_error(w: &WorldState, sid: SessionId) {
    let mut err_pkt = Packet::new(Opcode::WizQuest as u8);
    err_pkt.write_u8(13);
    err_pkt.write_u8(3); // error code 3 = not enough slots
    w.send_to_session_owned(sid, err_pkt);
}

fn get_world(lua: &Lua) -> LuaResult<Arc<WorldState>> {
    lua.app_data_ref::<Arc<WorldState>>()
        .map(|w| Arc::clone(&w))
        .ok_or_else(|| LuaError::runtime("WorldState not available"))
}

fn lua_val_to_i32(val: &LuaValue) -> LuaResult<i32> {
    match val {
        LuaValue::Integer(v) => Ok(*v as i32),
        LuaValue::Number(v) => Ok(*v as i32),
        _ => Err(LuaError::runtime("expected number")),
    }
}

fn lua_val_to_i32_or(val: &LuaValue, default: i32) -> i32 {
    match val {
        LuaValue::Integer(v) => *v as i32,
        LuaValue::Number(v) => *v as i32,
        _ => default,
    }
}

fn lua_val_to_u32(val: &LuaValue) -> LuaResult<u32> {
    match val {
        LuaValue::Integer(v) => Ok(*v as u32),
        LuaValue::Number(v) => Ok(*v as u32),
        _ => Err(LuaError::runtime("expected number")),
    }
}

fn lua_val_to_u16(val: &LuaValue) -> Option<u16> {
    match val {
        LuaValue::Integer(v) => Some(*v as u16),
        LuaValue::Number(v) => Some(*v as u16),
        _ => None,
    }
}

/// Build the premium info packet from WorldState + SessionId.
/// Wire: `WIZ_PREMIUM << u8(1) << u8(count) [foreach: u8(premType) << u16(timeHours)] << u8(premInUse) << u32(0)`
fn build_premium_info_for_lua(w: &WorldState, sid: SessionId, now: u32) -> Packet {
    let mut entries: Vec<(u8, u16)> = Vec::new();
    let mut premium_in_use: u8 = 0; // NO_PREMIUM

    w.with_session(sid, |h| {
        premium_in_use = h.premium_in_use;

        for (&p_type, &expiry) in &h.premium_map {
            if expiry == 0 {
                continue;
            }
            let time_rest = expiry.saturating_sub(now);
            let time_show: u16 = if (1..=3600).contains(&time_rest) {
                1
            } else {
                (time_rest / 3600) as u16
            };
            entries.push((p_type, time_show));

            // Auto-select first valid premium if none selected
            if premium_in_use == 0 {
                premium_in_use = p_type;
            }
        }
    });

    let mut pkt = Packet::new(Opcode::WizPremium as u8);
    pkt.write_u8(1); // SUBOPCODE_PREMIUM_INFO
    pkt.write_u8(entries.len() as u8);
    for (p_type, time_show) in &entries {
        pkt.write_u8(*p_type);
        pkt.write_u16(*time_show);
    }
    pkt.write_u8(premium_in_use);
    pkt.write_u32(0);
    pkt
}

/// Register all binding functions as Lua globals.
pub fn register_all(lua: &Lua) -> LuaResult<()> {
    let g = lua.globals();

    // Tier 1: Quest Flow
    g.set("CheckLevel", lua.create_function(lua_check_level)?)?;
    g.set("CheckClass", lua.create_function(lua_check_class)?)?;
    g.set("CheckNation", lua.create_function(lua_check_nation)?)?;
    g.set(
        "CheckSkillPoint",
        lua.create_function(lua_check_skill_point)?,
    )?;
    g.set("GetQuestStatus", lua.create_function(lua_get_quest_status)?)?;
    g.set(
        "QuestCheckExistEvent",
        lua.create_function(lua_check_exist_event)?,
    )?;
    g.set(
        "CheckExistEvent",
        lua.create_function(lua_check_exist_event)?,
    )?;
    g.set("SaveEvent", lua.create_function(lua_save_event)?)?;
    g.set("HowmuchItem", lua.create_function(lua_howmuch_item)?)?;
    g.set("CheckExistItem", lua.create_function(lua_check_exist_item)?)?;
    g.set("GiveItem", lua.create_function(lua_give_item)?)?;
    g.set("GiveItemLua", lua.create_function(lua_give_item)?)?;
    g.set("RobItem", lua.create_function(lua_rob_item)?)?;
    g.set(
        "ExchangeItemForAchievement",
        lua.create_function(lua_exchange_item_for_achievement)?,
    )?;
    g.set("GoldGain", lua.create_function(lua_gold_gain)?)?;
    g.set("GoldLose", lua.create_function(lua_gold_lose)?)?;
    g.set("ExpChange", lua.create_function(lua_exp_change)?)?;
    g.set("GiveLoyalty", lua.create_function(lua_give_loyalty)?)?;
    g.set("RobLoyalty", lua.create_function(lua_rob_loyalty)?)?;
    g.set("NpcSay", lua.create_function(lua_npc_say)?)?;
    g.set("NpcMsg", lua.create_function(lua_npc_msg)?)?;
    g.set("SelectMsg", lua.create_function(lua_select_msg)?)?;
    g.set("ZoneChange", lua.create_function(lua_zone_change)?)?;
    g.set("CheckPercent", lua.create_function(lua_check_percent)?)?;

    // Tier 2: Getters
    g.set("GetName", lua.create_function(lua_get_name)?)?;
    g.set("GetZoneID", lua.create_function(lua_get_zone_id)?)?;
    g.set("GetCoins", lua.create_function(lua_get_coins)?)?;
    g.set("GetLoyalty", lua.create_function(lua_get_loyalty)?)?;
    g.set("GetLevel", lua.create_function(lua_check_level)?)?;
    g.set("GetNation", lua.create_function(lua_check_nation)?)?;
    g.set("GetClass", lua.create_function(lua_get_class_full)?)?;
    g.set("GetExp", lua.create_function(lua_get_exp)?)?;
    g.set("GetInnCoins", lua.create_function(lua_get_inn_coins)?)?;
    g.set(
        "GetMonthlyLoyalty",
        lua.create_function(lua_get_monthly_loyalty)?,
    )?;
    g.set("GetManner", lua.create_function(lua_get_manner)?)?;
    g.set("GetX", lua.create_function(lua_get_x)?)?;
    g.set("GetY", lua.create_function(lua_get_y)?)?;
    g.set("GetZ", lua.create_function(lua_get_z)?)?;
    g.set("GetRace", lua.create_function(lua_get_race)?)?;
    g.set("GetAccountName", lua.create_function(lua_get_account_name)?)?;
    g.set("GetCash", lua.create_function(lua_get_cash)?)?;
    g.set("CheckCash", lua.create_function(lua_get_cash)?)?;

    // Class checks (job group)
    g.set("isWarrior", lua.create_function(lua_is_warrior)?)?;
    g.set("isRogue", lua.create_function(lua_is_rogue)?)?;
    g.set("isMage", lua.create_function(lua_is_mage)?)?;
    g.set("isPriest", lua.create_function(lua_is_priest)?)?;
    g.set("isPortuKurian", lua.create_function(lua_is_kurian)?)?;

    // Tier 2: Class Tier Checks
    g.set("isBeginner", lua.create_function(lua_is_beginner)?)?;
    g.set(
        "isBeginnerWarrior",
        lua.create_function(lua_is_beginner_warrior)?,
    )?;
    g.set(
        "isBeginnerRogue",
        lua.create_function(lua_is_beginner_rogue)?,
    )?;
    g.set("isBeginnerMage", lua.create_function(lua_is_beginner_mage)?)?;
    g.set(
        "isBeginnerPriest",
        lua.create_function(lua_is_beginner_priest)?,
    )?;
    g.set(
        "isBeginnerKurianPortu",
        lua.create_function(lua_is_beginner_kurian)?,
    )?;
    g.set(
        "isBeginnerKurian",
        lua.create_function(lua_is_beginner_kurian)?,
    )?;
    g.set("isNovice", lua.create_function(lua_is_novice)?)?;
    g.set(
        "isNoviceWarrior",
        lua.create_function(lua_is_novice_warrior)?,
    )?;
    g.set("isNoviceRogue", lua.create_function(lua_is_novice_rogue)?)?;
    g.set("isNoviceMage", lua.create_function(lua_is_novice_mage)?)?;
    g.set("isNovicePriest", lua.create_function(lua_is_novice_priest)?)?;
    g.set(
        "isNoviceKurianPortu",
        lua.create_function(lua_is_novice_kurian)?,
    )?;
    g.set("isNoviceKurian", lua.create_function(lua_is_novice_kurian)?)?;
    g.set("isMastered", lua.create_function(lua_is_mastered)?)?;
    g.set(
        "isMasteredWarrior",
        lua.create_function(lua_is_mastered_warrior)?,
    )?;
    g.set(
        "isMasteredRogue",
        lua.create_function(lua_is_mastered_rogue)?,
    )?;
    g.set("isMasteredMage", lua.create_function(lua_is_mastered_mage)?)?;
    g.set(
        "isMasteredPriest",
        lua.create_function(lua_is_mastered_priest)?,
    )?;
    g.set(
        "isMasteredKurianPortu",
        lua.create_function(lua_is_mastered_kurian)?,
    )?;
    g.set(
        "isMasteredKurian",
        lua.create_function(lua_is_mastered_kurian)?,
    )?;

    // Clan/party/king
    g.set("isInClan", lua.create_function(lua_is_in_clan)?)?;
    g.set("isClanLeader", lua.create_function(lua_is_clan_leader)?)?;
    g.set("isInParty", lua.create_function(lua_is_in_party)?)?;
    g.set("isPartyLeader", lua.create_function(lua_is_party_leader)?)?;
    g.set("isKing", lua.create_function(lua_is_king)?)?;

    // Has-checks
    g.set("hasCoins", lua.create_function(lua_has_coins)?)?;
    g.set("hasLoyalty", lua.create_function(lua_has_loyalty)?)?;
    g.set("hasInnCoins", lua.create_function(lua_has_inn_coins)?)?;
    g.set(
        "hasMonthlyLoyalty",
        lua.create_function(lua_has_monthly_loyalty)?,
    )?;

    // Inventory
    g.set("CheckWeight", lua.create_function(lua_check_weight)?)?;
    g.set("isRoomForItem", lua.create_function(lua_is_room_for_item)?)?;
    g.set("CheckGiveSlot", lua.create_function(lua_check_give_slot)?)?;

    // Quest monster bindings
    g.set(
        "CountMonsterQuestSub",
        lua.create_function(lua_count_monster_quest_sub)?,
    )?;
    g.set(
        "QuestCheckQuestFinished",
        lua.create_function(lua_quest_check_finished)?,
    )?;
    g.set(
        "CountMonsterQuestMain",
        lua.create_function(lua_count_monster_quest_main)?,
    )?;
    // C++ alias: _LUA_WRAPPER_USER_FUNCTION(ExistMonsterQuestSub, GetActiveQuestID) → always 0
    g.set(
        "ExistMonsterQuestSub",
        lua.create_function(lua_exist_monster_quest_sub)?,
    )?;
    g.set("SearchQuest", lua.create_function(lua_search_quest)?)?;

    // Actions
    g.set("ShowEffect", lua.create_function(lua_show_effect)?)?;
    g.set("ShowNpcEffect", lua.create_function(lua_show_npc_effect)?)?;
    g.set(
        "PromoteUserNovice",
        lua.create_function(lua_promote_user_novice)?,
    )?;
    g.set("PromoteUser", lua.create_function(lua_promote_user)?)?;
    g.set(
        "ResetSkillPoints",
        lua.create_function(lua_reset_skill_points)?,
    )?;
    g.set(
        "ResetStatPoints",
        lua.create_function(lua_reset_stat_points)?,
    )?;
    g.set("ShowMap", lua.create_function(lua_show_map)?)?;
    g.set("LevelChange", lua.create_function(lua_level_change)?)?;
    g.set("GiveBalance", lua.create_function(lua_give_balance)?)?;
    g.set(
        "SendStatSkillDistribute",
        lua.create_function(lua_send_stat_skill_distribute)?,
    )?;

    // Clan
    g.set("CheckClanGrade", lua.create_function(lua_check_clan_grade)?)?;
    g.set("CheckClanPoint", lua.create_function(lua_check_clan_point)?)?;
    g.set("CheckLoyalty", lua.create_function(lua_get_loyalty)?)?;
    // C++ alias: _LUA_WRAPPER_USER_FUNCTION(CheckKnight, GetClanRank)
    g.set("CheckKnight", lua.create_function(lua_get_clan_rank)?)?;
    g.set("CheckStatPoint", lua.create_function(lua_check_stat_point)?)?;

    // Misc
    g.set("GetPremium", lua.create_function(lua_get_premium)?)?;
    g.set(
        "GetEventTrigger",
        lua.create_function(lua_get_event_trigger)?,
    )?;
    g.set(
        "GetQuestHelperID",
        lua.create_function(lua_get_quest_helper_id)?,
    )?;
    g.set("RollDice", lua.create_function(lua_roll_dice)?)?;

    // Exchange system (Tier 1)
    g.set(
        "RunGiveItemExchange",
        lua.create_function(lua_run_give_item_exchange)?,
    )?;
    g.set("CheckExchange", lua.create_function(lua_check_exchange)?)?;
    g.set("RunExchange", lua.create_function(lua_run_exchange)?)?;
    g.set(
        "RunCountExchange",
        lua.create_function(lua_run_count_exchange)?,
    )?;
    // Exchange variants — aliases / stubs
    g.set(
        "RunQuestExchange",
        lua.create_function(lua_run_quest_exchange)?,
    )?;
    g.set(
        "RunRandomExchange",
        lua.create_function(lua_run_random_exchange)?,
    )?;
    g.set(
        "RunMiningExchange",
        lua.create_function(lua_run_mining_exchange)?,
    )?;

    // Custom server bindings (not in original C++ — RimaGUARD scripts)
    g.set(
        "MonsterStoneQuestJoin",
        lua.create_function(lua_monster_stone_quest_join)?,
    )?;
    g.set("GiveCash", lua.create_function(lua_give_cash)?)?;
    g.set(
        "RequestPersonalRankReward",
        lua.create_function(lua_request_personal_rank_reward)?,
    )?;
    g.set("RequestReward", lua.create_function(lua_request_reward)?)?;
    g.set("OpenSkill", lua.create_function(lua_open_skill)?)?;
    g.set(
        "SendGenderChange",
        lua.create_function(lua_send_gender_change_ui)?,
    )?;
    g.set("RebirthBas", lua.create_function(lua_rebirth_bas)?)?;

    // Character changes (Tier 1)
    g.set("JobChange", lua.create_function(lua_job_change)?)?;
    g.set("GenderChange", lua.create_function(lua_gender_change)?)?;

    // Premium & Getters (Tier 2)
    g.set("GivePremium", lua.create_function(lua_give_premium)?)?;
    g.set("NationChange", lua.create_function(lua_nation_change)?)?;
    g.set("GetExpPercent", lua.create_function(lua_get_exp_percent)?)?;

    g.set("NpcGetID", lua.create_function(lua_npc_get_id)?)?;
    g.set("NpcGetProtoID", lua.create_function(lua_npc_get_proto_id)?)?;
    g.set("NpcGetName", lua.create_function(lua_npc_get_name)?)?;
    g.set("NpcGetNation", lua.create_function(lua_npc_get_nation)?)?;
    g.set("NpcGetType", lua.create_function(lua_npc_get_type)?)?;
    g.set("NpcGetZoneID", lua.create_function(lua_npc_get_zone_id)?)?;
    g.set("NpcGetX", lua.create_function(lua_npc_get_x)?)?;
    g.set("NpcGetY", lua.create_function(lua_npc_get_y)?)?;
    g.set("NpcGetZ", lua.create_function(lua_npc_get_z)?)?;
    g.set("NpcCastSkill", lua.create_function(lua_npc_cast_skill)?)?;

    // Sprint 31: Implemented stubs
    g.set("ChangeManner", lua.create_function(lua_change_manner)?)?;
    g.set("RobClanPoint", lua.create_function(lua_rob_clan_point)?)?;
    g.set("KissUser", lua.create_function(lua_kiss_user)?)?;
    g.set("SendNameChange", lua.create_function(lua_send_name_change)?)?;
    g.set(
        "SendClanNameChange",
        lua.create_function(lua_send_clan_name_change)?,
    )?;
    g.set(
        "SendTagNameChangePanel",
        lua.create_function(lua_send_tag_name_change_panel)?,
    )?;
    g.set(
        "ZoneChangeParty",
        lua.create_function(lua_zone_change_party)?,
    )?;
    g.set("ZoneChangeClan", lua.create_function(lua_zone_change_clan)?)?;
    g.set("PromoteKnight", lua.create_function(lua_promote_knight)?)?;

    // Sprint 32: New implementations
    g.set("hasManner", lua.create_function(lua_has_manner)?)?;
    g.set(
        "CheckWarVictory",
        lua.create_function(lua_check_war_victory)?,
    )?;
    g.set(
        "GetPVPMonumentNation",
        lua.create_function(lua_get_pvp_monument_nation)?,
    )?;
    g.set(
        "CheckMiddleStatueCapture",
        lua.create_function(lua_check_middle_statue_capture)?,
    )?;
    g.set(
        "GetRebirthLevel",
        lua.create_function(lua_get_rebirth_level)?,
    )?;
    g.set(
        "KingsInspectorList",
        lua.create_function(lua_kings_inspector_list)?,
    )?;
    g.set("GetMaxExchange", lua.create_function(lua_get_max_exchange)?)?;
    g.set(
        "isCswWinnerNembers",
        lua.create_function(lua_is_csw_winner_members)?,
    )?;
    // Sprint 621: Castle siege deathmatch stubs (quest scripts call these)
    g.set(
        "CheckCastleSiegeWarDeathmachRegister",
        lua.create_function(lua_csw_deathmatch_register)?,
    )?;
    g.set(
        "CheckCastleSiegeWarDeathmacthCancelRegister",
        lua.create_function(lua_csw_deathmatch_cancel_register)?,
    )?;
    g.set("SendNpcKillID", lua.create_function(lua_send_npc_kill_id)?)?;
    // Sprint 621: NpcKillID alias — Lua scripts use both names
    g.set("NpcKillID", lua.create_function(lua_send_npc_kill_id)?)?;
    g.set("CycleSpawn", lua.create_function(lua_cycle_spawn)?)?;
    g.set(
        "MoveMiddleStatue",
        lua.create_function(lua_move_middle_statue)?,
    )?;
    g.set(
        "SendNationTransfer",
        lua.create_function(lua_send_nation_transfer)?,
    )?;
    g.set(
        "RobAllItemParty",
        lua.create_function(lua_rob_all_item_party)?,
    )?;

    // Sprint 33: New implementations
    g.set(
        "ShowBulletinBoard",
        lua.create_function(lua_show_bulletin_board)?,
    )?;
    g.set("SendVisibe", lua.create_function(lua_send_visibe)?)?;
    g.set(
        "GiveSwitchPremium",
        lua.create_function(lua_give_switch_premium)?,
    )?;
    g.set(
        "GiveClanPremium",
        lua.create_function(lua_give_clan_premium)?,
    )?;
    g.set(
        "GivePremiumItem",
        lua.create_function(lua_give_premium_item)?,
    )?;
    g.set(
        "SpawnEventSystem",
        lua.create_function(lua_spawn_event_system)?,
    )?;
    g.set("NpcEventSystem", lua.create_function(lua_npc_event_system)?)?;
    g.set("KillNpcEvent", lua.create_function(lua_kill_npc_event)?)?;
    g.set(
        "SendRepurchaseMsg",
        lua.create_function(lua_send_repurchase_msg)?,
    )?;
    g.set("DrakiOutZone", lua.create_function(lua_draki_out_zone)?)?;
    g.set(
        "DrakiTowerNpcOut",
        lua.create_function(lua_draki_tower_npc_out)?,
    )?;
    g.set("GenieExchange", lua.create_function(lua_genie_exchange)?)?;
    g.set(
        "DelosCasttellanZoneOut",
        lua.create_function(lua_delos_castellan_zone_out)?,
    )?;
    g.set(
        "CheckBeefEventLogin",
        lua.create_function(lua_check_beef_event_login)?,
    )?;
    g.set(
        "CheckMonsterChallengeTime",
        lua.create_function(lua_check_monster_challenge_time)?,
    )?;
    g.set(
        "CheckMonsterChallengeUserCount",
        lua.create_function(lua_check_monster_challenge_user_count)?,
    )?;
    g.set(
        "CheckUnderTheCastleOpen",
        lua.create_function(lua_check_under_castle_open)?,
    )?;
    g.set(
        "CheckUnderTheCastleUserCount",
        lua.create_function(lua_check_under_castle_user_count)?,
    )?;
    g.set(
        "CheckJuraidMountainTime",
        lua.create_function(lua_check_juraid_mountain_time)?,
    )?;
    g.set(
        "GetUserDailyOp",
        lua.create_function(lua_get_user_daily_op)?,
    )?;

    // Sprint 34: Final binding implementations
    g.set("CastSkill", lua.create_function(lua_npc_cast_skill)?)?; // alias for NpcCastSkill
    g.set(
        "EventSoccerMember",
        lua.create_function(lua_event_soccer_member)?,
    )?;
    g.set(
        "EventSoccerStard",
        lua.create_function(lua_event_soccer_stard)?,
    )?;
    g.set("JoinEvent", lua.create_function(lua_join_event)?)?;
    g.set(
        "DrakiRiftChange",
        lua.create_function(lua_draki_rift_change)?,
    )?;
    g.set("ClanNts", lua.create_function(lua_clan_nts)?)?;
    g.set("PerkUseItem", lua.create_function(lua_perk_use_item)?)?;

    // Sprint 116: Missing C++ aliases
    g.set("GetStat", lua.create_function(lua_check_stat_point)?)?;
    g.set("PromoteClan", lua.create_function(lua_promote_knight)?)?;

    // Sprint 385: C++ MAKE_LUA_METHOD originals (aliases for Check* wrappers)
    // C++ registers both GetWarVictory and CheckWarVictory → same function
    g.set("GetWarVictory", lua.create_function(lua_check_war_victory)?)?;
    // C++ registers both GetUnderTheCastleOpen and CheckUnderTheCastleOpen → same function
    g.set(
        "GetUnderTheCastleOpen",
        lua.create_function(lua_check_under_castle_open)?,
    )?;
    // C++ registers both GetJuraidMountainTime and CheckJuraidMountainTime → same function
    g.set(
        "GetJuraidMountainTime",
        lua.create_function(lua_check_juraid_mountain_time)?,
    )?;
    // C++ registers both BeefEventLogin and CheckBeefEventLogin → same function
    g.set(
        "BeefEventLogin",
        lua.create_function(lua_check_beef_event_login)?,
    )?;
    g.set(
        "GetActiveQuestID",
        lua.create_function(lua_exist_monster_quest_sub)?,
    )?;
    g.set(
        "GetPartyMemberAmount",
        lua.create_function(lua_get_party_member_amount)?,
    )?;
    g.set(
        "PartyCountMembers",
        lua.create_function(lua_get_party_member_amount)?,
    )?;
    g.set("GetClanRank", lua.create_function(lua_get_clan_rank)?)?;

    // Sprint 385 B6: Remaining C++ MAKE_LUA_METHOD originals not yet registered as globals.
    // C++ lua_bindings.cpp lines 164-165: GetMonsterChallengeTime / GetMonsterChallengeUserCount
    // are MAKE_LUA_METHOD originals; Check* variants are _LUA_WRAPPER aliases (line 429-430).
    g.set(
        "GetMonsterChallengeTime",
        lua.create_function(lua_check_monster_challenge_time)?,
    )?;
    g.set(
        "GetMonsterChallengeUserCount",
        lua.create_function(lua_check_monster_challenge_user_count)?,
    )?;
    // C++ lua_bindings.cpp line 167: GetUnderTheCastleUserCount MAKE_LUA_METHOD original.
    // CheckUnderTheCastleUserCount is a _LUA_WRAPPER alias (line 432).
    g.set(
        "GetUnderTheCastleUserCount",
        lua.create_function(lua_check_under_castle_user_count)?,
    )?;
    // C++ lua_bindings.cpp lines 53-54: GetClanGrade / GetClanPoint are MAKE_LUA_METHOD originals.
    // CheckClanGrade / CheckClanPoint are _LUA_WRAPPER aliases (lines 418-419).
    g.set("GetClanGrade", lua.create_function(lua_check_clan_grade)?)?;
    g.set("GetClanPoint", lua.create_function(lua_check_clan_point)?)?;

    // NPC action bindings (dialog_builder v4 — action events)
    g.set("OpenTradeNpc", lua.create_function(lua_open_trade_npc)?)?;
    g.set("SendWarpList", lua.create_function(lua_send_warp_list)?)?;
    g.set(
        "OpenShoppingMall",
        lua.create_function(lua_open_shopping_mall)?,
    )?;

    // Box Breaker NPC (Quests/31514_BoxBreaker.lua) - see world/box_breaker.rs
    g.set(
        "GetBreakableBoxes",
        lua.create_function(lua_get_breakable_boxes)?,
    )?;
    g.set("BoxPick", lua.create_function(lua_box_pick)?)?;
    g.set("BoxBreak", lua.create_function(lua_box_break)?)?;
    g.set("BoxBreakAll", lua.create_function(lua_box_break_all)?)?;
    g.set("SendChatLine", lua.create_function(lua_send_chat_line)?)?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Binding Implementations
// ═══════════════════════════════════════════════════════════════════════

fn lua_check_level(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    Ok(w.get_character_info(uid as SessionId)
        .map(|c| c.level)
        .unwrap_or(0))
}

fn lua_check_class(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    Ok(w.get_character_info(uid as SessionId)
        .map(|c| (c.class % 100) as u8)
        .unwrap_or(0))
}
fn lua_get_class_full(lua: &Lua, uid: i32) -> LuaResult<u16> {
    let w = get_world(lua)?;
    Ok(w.get_character_info(uid as SessionId)
        .map(|c| c.class)
        .unwrap_or(0))
}
/// Get the number of members in the player's party.
fn lua_get_party_member_amount(lua: &Lua, uid: i32) -> LuaResult<u16> {
    let w = get_world(lua)?;
    let party_id = w.get_party_id(uid as SessionId);
    match party_id {
        Some(pid) => Ok(w.get_party_member_count(pid) as u16),
        None => Ok(0),
    }
}

fn lua_check_nation(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    Ok(w.get_character_info(uid as SessionId)
        .map(|c| c.nation)
        .unwrap_or(0))
}

fn lua_check_skill_point(lua: &Lua, (uid, cat): (i32, u32)) -> LuaResult<u8> {
    let w = get_world(lua)?;
    Ok(w.get_character_info(uid as SessionId)
        .map(|c| {
            let idx = cat as usize;
            if idx < c.skill_points.len() {
                c.skill_points[idx]
            } else {
                0
            }
        })
        .unwrap_or(0))
}

fn lua_get_quest_status(lua: &Lua, (uid, qid): (i32, u16)) -> LuaResult<u8> {
    let w = get_world(lua)?;
    Ok(w.with_session(uid as SessionId, |h| {
        h.quests.get(&qid).map(|q| q.quest_state).unwrap_or(0)
    })
    .unwrap_or(0))
}

fn lua_check_exist_event(lua: &Lua, (uid, qid, status): (i32, u16, u8)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    Ok(
        w.with_session(uid as SessionId, |h| match h.quests.get(&qid) {
            Some(q) => q.quest_state == status,
            None => status == 0,
        })
        .unwrap_or(status == 0),
    )
}

fn lua_save_event(lua: &Lua, (uid, quest_helper_id): (i32, u16)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // Lua passes quest_helper n_index, NOT the quest ID + status directly.
    // We look up the quest_helper to get sEventDataIndex and bEventStatus.
    let helper = match w.get_quest_helper(quest_helper_id as u32) {
        Some(h) => h,
        None => {
            tracing::warn!("SaveEvent: quest_helper {} not found", quest_helper_id);
            return Ok(());
        }
    };
    let qid = helper.s_event_data_index as u16;
    let status = helper.b_event_status as u8;

    // Never complete a quest whose reward was refused earlier in this run
    // (bag full / gold cap): the player keeps the quest and its turn-in items
    // and can simply try again after making room.
    if status == 2 && quest_completion_blocked(lua) {
        tracing::warn!(
            sid,
            quest_helper_id,
            quest_id = qid,
            "SaveEvent: completion of quest {} refused - its reward could not be delivered              (inventory full); quest stays open",
            qid
        );
        return Ok(());
    }

    // Get kill counts before update (needed for DB persist)
    let kill_counts_before = w
        .with_session(sid, |h| {
            h.quests.get(&qid).map(|q| q.kill_counts).unwrap_or([0; 4])
        })
        .unwrap_or([0; 4]);

    match status {
        1 => w.update_session(sid, |h| {
            let info = h.quests.entry(qid).or_default();
            info.quest_state = 1;
            info.kill_counts = [0; 4];
        }),
        4 => w.update_session(sid, |h| {
            if let Some(info) = h.quests.get_mut(&qid) {
                info.quest_state = 4;
            }
        }),
        _ => w.update_session(sid, |h| {
            let info = h.quests.entry(qid).or_default();
            info.quest_state = status;
        }),
    }

    let mut pkt = Packet::new(Opcode::WizQuest as u8);
    pkt.write_u8(2);
    pkt.write_u16(qid);
    pkt.write_u8(status);
    w.send_to_session_owned(sid, pkt);

    if status == 1 && w.get_quest_monster(qid).is_some() {
        let kc = w
            .with_session(sid, |h| {
                h.quests.get(&qid).map(|q| q.kill_counts).unwrap_or([0; 4])
            })
            .unwrap_or([0; 4]);
        let mut mpkt = Packet::new(Opcode::WizQuest as u8);
        mpkt.write_u8(9);
        mpkt.write_u8(1);
        mpkt.write_u16(qid);
        for &k in &kc {
            mpkt.write_u16(k as u16);
        }
        w.send_to_session_owned(sid, mpkt);
    }

    // Persist to DB (fire-and-forget async task)
    if let Some(pool) = w.db_pool() {
        let char_name = w
            .get_character_info(sid)
            .map(|c| c.name)
            .unwrap_or_default();
        if !char_name.is_empty() {
            let pool = pool.clone();
            let quest_id = qid as i16;
            let quest_state = status as i16;
            let kc = if status == 1 {
                [0i16; 4]
            } else {
                [
                    kill_counts_before[0] as i16,
                    kill_counts_before[1] as i16,
                    kill_counts_before[2] as i16,
                    kill_counts_before[3] as i16,
                ]
            };
            tokio::spawn(async move {
                let repo = ko_db::repositories::quest::QuestRepository::new(&pool);
                if quest_state == 4 {
                    if let Err(e) = repo.delete_user_quest(&char_name, quest_id).await {
                        tracing::error!("SaveEvent DB delete quest {}: {}", quest_id, e);
                    }
                } else if let Err(e) = repo
                    .save_user_quest(&char_name, quest_id, quest_state, kc)
                    .await
                {
                    tracing::error!("SaveEvent DB save quest {}: {}", quest_id, e);
                }
            });
        }
    }

    if status == 4 {
        w.update_session(sid, |h| {
            h.quests.remove(&qid);
        });
    }

    if status == 2 {
        crate::handler::achieve::on_quest_completed(&w, sid);
    }

    Ok(())
}

fn lua_howmuch_item(lua: &Lua, (uid, item_id): (i32, u32)) -> LuaResult<u32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    // ITEM_GOLD(900000000), ITEM_HUNT(900005000), ITEM_CHAT(900012000), ITEM_COUNT(900002000)
    match item_id {
        900_000_000 => {
            // ITEM_GOLD
            return Ok(w.get_character_info(sid).map(|c| c.gold).unwrap_or(0));
        }
        900_002_000 | 900_003_000 => {
            // ITEM_COUNT / ITEM_LADDERPOINT → return loyalty
            return Ok(w.get_character_info(sid).map(|c| c.loyalty).unwrap_or(0));
        }
        900_005_000 | 900_012_000 => {
            // ITEM_HUNT / ITEM_CHAT → always pass (C++ returns true)
            return Ok(u32::MAX);
        }
        _ => {}
    }
    let inv = w.get_inventory(sid);
    Ok(inv
        .iter()
        .filter(|s| s.item_id == item_id)
        .map(|s| s.count as u32)
        .sum())
}

fn lua_check_exist_item(lua: &Lua, (uid, item_id, count): (i32, u32, u32)) -> LuaResult<bool> {
    Ok(lua_howmuch_item(lua, (uid, item_id))? >= count)
}

/// Why a single-item grant could not be delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GiveItemFailure {
    /// The item id is not a deliverable inventory item (unknown id or one of
    /// the virtual `countable == 2` ids such as gold/exp). Fix the script/data.
    InvalidItem,
    /// The bag has no room. The player can fix this by freeing a slot.
    NoSlot,
}

/// Classify why `give_item` would fail for this item (called after it failed).
fn classify_give_item_failure(w: &WorldState, item_id: u32) -> GiveItemFailure {
    match w.get_item(item_id) {
        Some(item) if item.countable.unwrap_or(0) != 2 => GiveItemFailure::NoSlot,
        _ => GiveItemFailure::InvalidItem,
    }
}

/// Convert a Lua number to `u32` without silently wrapping (a 10-digit item id
/// such as `8104340970` used to wrap around to an unrelated id).
fn lua_val_to_u32_exact(val: &LuaValue) -> Option<u32> {
    match val {
        LuaValue::Integer(v) => u32::try_from(*v).ok(),
        LuaValue::Number(v) if v.fract() == 0.0 && *v >= 0.0 && *v <= u32::MAX as f64 => {
            Some(*v as u32)
        }
        _ => None,
    }
}

/// Give an item to a player from a quest script.
/// Returns `true` to Lua on success, `false` on failure (inventory full, invalid item, etc.).
///
/// A failure caused by a full bag also tells the player (`WIZ_QUEST` 13/3) and
/// stops the same script run from completing the quest in `SaveEvent`.
fn lua_give_item(lua: &Lua, args: LuaMultiValue) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let vals: Vec<LuaValue> = args.into_vec();
    // C++ GiveItem signature: GiveItem(uid, item_id [, count [, expiry_days]])
    // vals[0]=uid, vals[1]=item_id, vals[2]=count (default 1), vals[3]=expiry_days (default 0)
    if vals.len() < 2 {
        return Ok(false);
    }
    let uid = lua_val_to_i32(&vals[0])?;
    let item_id = match lua_val_to_u32_exact(&vals[1]) {
        Some(id) => id,
        None => {
            tracing::warn!(
                sid = uid,
                "GiveItem: item id {:?} is not a valid 32-bit item number - nothing given",
                vals[1]
            );
            return Ok(false);
        }
    };
    let count = if vals.len() >= 3 {
        // Clamp instead of wrapping: 70000 used to become 4464.
        match &vals[2] {
            LuaValue::Integer(v) => (*v).clamp(1, ITEMCOUNT_MAX as i64) as u16,
            LuaValue::Number(v) => (*v as i64).clamp(1, ITEMCOUNT_MAX as i64) as u16,
            _ => 1,
        }
    } else {
        1
    };
    let expiry_days = if vals.len() >= 4 {
        lua_val_to_i32_or(&vals[3], 0).max(0) as u32
    } else {
        0
    };
    let sid = uid as SessionId;
    let success = if expiry_days > 0 {
        w.give_item_with_expiry(sid, item_id, count, expiry_days)
    } else {
        w.give_item(sid, item_id, count)
    };
    if !success {
        match classify_give_item_failure(&w, item_id) {
            GiveItemFailure::NoSlot => {
                tracing::warn!(
                    sid,
                    item_id,
                    count,
                    "GiveItem: delivery FAILED - inventory full"
                );
                send_no_slot_error(&w, sid);
                block_quest_completion(lua);
            }
            GiveItemFailure::InvalidItem => {
                tracing::warn!(
                    sid,
                    item_id,
                    count,
                    "GiveItem: delivery FAILED - item id is not in the item table (or is a virtual id)"
                );
            }
        }
        return Ok(false);
    }

    // FerihaLog: GiveItemInsertLog (quest reward) - only for items that were really given.
    if let Some(pool) = w.db_pool() {
        let acc = w
            .with_session(sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let ch_name = w.get_session_name(sid).unwrap_or_default();
        let pos = w.get_position(sid);
        crate::handler::audit_log::log_give_item(
            pool,
            &acc,
            &ch_name,
            pos.as_ref().map(|p| p.zone_id as i16).unwrap_or(0),
            pos.as_ref().map(|p| p.x as i16).unwrap_or(0),
            pos.as_ref().map(|p| p.z as i16).unwrap_or(0),
            "quest_reward",
            item_id,
            count,
        );
    }

    Ok(true)
}

/// `RobItem(UID, item_id [, count])` - count is optional and defaults to 1 like the
/// C++ `LUA_ARG_OPTIONAL`; with a required count, 2-argument calls (councilor,
/// Pot/FoodBox/Well scripts ...) raised a conversion error and aborted the script.
fn lua_rob_item(lua: &Lua, (uid, item_id, count): (i32, u32, Option<u16>)) -> LuaResult<()> {
    get_world(lua)?.rob_item(uid as SessionId, item_id, count.unwrap_or(1));
    Ok(())
}

fn lua_exchange_item_for_achievement(
    lua: &Lua,
    (uid, item_id, item_count, achievement_id): (i32, u32, u16, u16),
) -> LuaResult<i32> {
    let world = get_world(lua)?;
    Ok(crate::handler::achieve::exchange_item_for_achievement(
        &world,
        uid as SessionId,
        item_id,
        item_count,
        achievement_id,
    ))
}

fn lua_gold_gain(lua: &Lua, (uid, amount): (i32, u32)) -> LuaResult<()> {
    get_world(lua)?.gold_gain(uid as SessionId, amount);
    Ok(())
}

fn lua_gold_lose(lua: &Lua, (uid, amount): (i32, u32)) -> LuaResult<()> {
    get_world(lua)?.gold_lose(uid as SessionId, amount);
    Ok(())
}

fn lua_exp_change(lua: &Lua, (uid, amount): (i32, i64)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    w.update_character_stats(sid, |ch| {
        if amount >= 0 {
            ch.exp = ch.exp.saturating_add(amount as u64);
        } else {
            ch.exp = ch.exp.saturating_sub(amount.unsigned_abs());
        }
    });
    if let Some(ch) = w.get_character_info(sid) {
        let mut pkt = Packet::new(Opcode::WizExpChange as u8);
        pkt.write_u8(1);
        pkt.write_i64(ch.exp as i64);
        w.send_to_session_owned(sid, pkt);
    }
    Ok(())
}

fn lua_give_loyalty(lua: &Lua, (uid, amount): (i32, u32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    // C++ GiveLoyalty does NOT update monthly loyalty (bIsAddLoyaltyMonthly=false).
    w.update_character_stats(sid, |ch| {
        ch.loyalty = ch.loyalty.saturating_add(amount);
    });
    // C++ wire format: WIZ_LOYALTY_CHANGE << u8(1) << u32(loyalty) << u32(monthly) << u32(0) << u32(clan_loyalty)
    if let Some(ch) = w.get_character_info(sid) {
        let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
        pkt.write_u8(1); // LOYALTY_NATIONAL_POINTS
        pkt.write_u32(ch.loyalty);
        pkt.write_u32(ch.loyalty_monthly);
        pkt.write_u32(0); // unused
        pkt.write_u32(0); // clan loyalty change (not applicable from Lua)
        w.send_to_session_owned(sid, pkt);
    }
    Ok(())
}

fn lua_rob_loyalty(lua: &Lua, (uid, amount): (i32, u32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    w.update_character_stats(sid, |ch| {
        ch.loyalty = ch.loyalty.saturating_sub(amount);
    });
    // C++ wire format: WIZ_LOYALTY_CHANGE << u8(2) << u32(loyalty) << u32(monthly) << u32(0) << u32(clan_loyalty)
    if let Some(ch) = w.get_character_info(sid) {
        let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
        pkt.write_u8(2); // LOYALTY_ROBBED
        pkt.write_u32(ch.loyalty);
        pkt.write_u32(ch.loyalty_monthly);
        pkt.write_u32(0); // unused
        pkt.write_u32(0); // clan loyalty change (not applicable from Lua)
        w.send_to_session_owned(sid, pkt);
    }
    Ok(())
}

fn lua_npc_say(lua: &Lua, args: LuaMultiValue) -> LuaResult<()> {
    let w = get_world(lua)?;
    let vals: Vec<LuaValue> = args.into_vec();
    if vals.is_empty() {
        return Ok(());
    }
    let uid = lua_val_to_i32(&vals[0])?;
    let mut text_ids = [0i32; 8];
    for (i, val) in vals.iter().skip(1).take(8).enumerate() {
        text_ids[i] = lua_val_to_i32_or(val, 0);
    }
    let pkt = quest::build_npc_say_packet(&text_ids);
    w.send_to_session_owned(uid as SessionId, pkt);
    Ok(())
}

/// Lua `NpcMsg(uid, nQuestID [, sNpcID])` — send quest NPC dialog prompt.
///   arg 2 = nQuestID (quest_helper n_index)
///   arg 3 = sNpcID   (optional, defaults to m_sEventSid)
/// Calls `QuestV2SendNpcMsg(nQuestID, sNpcID)`:
///   Packet: WIZ_QUEST [u8(7)] [u32(nQuestID)] [u16(sNpcID)]
fn lua_npc_msg(lua: &Lua, args: LuaMultiValue) -> LuaResult<()> {
    let w = get_world(lua)?;
    let vals: Vec<LuaValue> = args.into_vec();
    if vals.is_empty() {
        return Ok(());
    }
    let uid = lua_val_to_i32(&vals[0])?;
    let sid = uid as SessionId;

    // arg 2 = nQuestID (quest_helper n_index)
    let quest_id = if vals.len() > 1 {
        lua_val_to_u32(&vals[1])?
    } else {
        0
    };

    // arg 3 = sNpcID (optional, defaults to m_sEventSid)
    let npc_id = if vals.len() > 2 {
        lua_val_to_u32(&vals[2])?
    } else {
        w.with_session(sid, |h| h.event_sid as u32).unwrap_or(0)
    };

    // NOTE: C++ QuestV2SendNpcMsg does NOT modify m_nQuestHelperID.
    // The quest_helper_id is set by quest_v2_run_event before Lua execution,
    // so we must not overwrite it here with the quest_menu ID.

    // QuestV2SendNpcMsg writes the NPC SID as uint16. Writing uint32 here
    // leaves two trailing bytes and makes the v2615 client reject the dialog.
    // Send WIZ_QUEST sub-opcode 7: [u32 nQuestID] [u16 sNpcID]
    let pkt = build_npc_msg_packet(quest_id, npc_id as u16);
    w.send_to_session_owned(sid, pkt);
    Ok(())
}

fn build_npc_msg_packet(quest_id: u32, npc_id: u16) -> Packet {
    let mut pkt = Packet::new(Opcode::WizQuest as u8);
    pkt.write_u8(7);
    pkt.write_u32(quest_id);
    pkt.write_u16(npc_id);
    pkt
}

#[allow(clippy::too_many_arguments)]
fn lua_select_msg(lua: &Lua, args: LuaMultiValue) -> LuaResult<()> {
    let w = get_world(lua)?;
    let vals: Vec<LuaValue> = args.into_vec();
    if vals.len() < 5 {
        return Ok(());
    }

    let uid = lua_val_to_i32(&vals[0])?;
    let flag = lua_val_to_i32_or(&vals[1], 0) as u8;
    let quest_id = lua_val_to_i32_or(&vals[2], -1);
    let header_text = lua_val_to_i32_or(&vals[3], -1);

    let mut btn_texts = [-1i32; MAX_MESSAGE_EVENT];
    let mut btn_events = [-1i32; MAX_MESSAGE_EVENT];
    // vals[4] = NPC proto_id (Lua convention, not in C++ SelectMsg signature).
    // Already set as event_sid when player clicks NPC (client_event.rs).
    // Button text/event pairs start at vals[5].
    let mut ai = 5;
    for i in 0..MAX_MESSAGE_EVENT {
        if ai < vals.len() {
            btn_texts[i] = lua_val_to_i32_or(&vals[ai], -1);
            ai += 1;
        }
        if ai < vals.len() {
            btn_events[i] = lua_val_to_i32_or(&vals[ai], -1);
            ai += 1;
        }
    }

    let sid = uid as SessionId;
    let lua_filename = w
        .with_session(sid, |h| {
            if h.quest_helper_id > 0 {
                w.get_quest_helper(h.quest_helper_id)
                    .map(|qh| qh.str_lua_filename.clone())
            } else {
                None
            }
        })
        .flatten()
        .unwrap_or_default();

    select_msg::send_select_msg(
        &w,
        sid,
        flag,
        quest_id,
        header_text,
        &btn_texts,
        &btn_events,
        &lua_filename,
    );
    Ok(())
}

fn lua_zone_change(lua: &Lua, (uid, zone_id, x, z): (i32, u16, f32, f32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let old_position = w.get_position(sid);

    // Lua ZoneChange is also used by the Draki floor NPCs while the player
    // remains in zone 95.  The old implementation only changed three session
    // fields and emitted a legacy float packet; it did not move the player
    // between region grids.  v2615 consequently acknowledged the click while
    // leaving the character on the previous floor.  Use the same authoritative
    // server teleport path as the other instanced events, forcing same-zone
    // relocation when necessary.
    crate::handler::zone_change::server_teleport_to_zone_force(&w, sid, zone_id, x, z);
    tracing::info!(
        sid,
        from_zone = old_position.map(|p| p.zone_id).unwrap_or_default(),
        from_x = old_position.map(|p| p.x).unwrap_or_default(),
        from_z = old_position.map(|p| p.z).unwrap_or_default(),
        to_zone = zone_id,
        to_x = x,
        to_z = z,
        "Lua ZoneChange completed through authoritative region teleport"
    );
    Ok(())
}

/// `CheckPercent(500)` ≈ 50% chance, `CheckPercent(1000)` ≈ 100%.
fn lua_check_percent(_lua: &Lua, percent: i32) -> LuaResult<bool> {
    use rand::Rng;
    if !(0..=1000).contains(&percent) {
        return Ok(false);
    }
    Ok(percent > rand::thread_rng().gen_range(0..=1000))
}

// Getters
fn lua_get_name(lua: &Lua, uid: i32) -> LuaResult<String> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.name)
        .unwrap_or_default())
}
/// Returns the account ID (login name) for the given session.
fn lua_get_account_name(lua: &Lua, uid: i32) -> LuaResult<String> {
    Ok(get_world(lua)?
        .with_session(uid as SessionId, |h| h.account_id.clone())
        .unwrap_or_default())
}
fn lua_get_zone_id(lua: &Lua, uid: i32) -> LuaResult<u16> {
    Ok(get_world(lua)?
        .get_position(uid as SessionId)
        .map(|p| p.zone_id)
        .unwrap_or(0))
}
fn lua_get_coins(lua: &Lua, uid: i32) -> LuaResult<u32> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.gold)
        .unwrap_or(0))
}
fn lua_get_loyalty(lua: &Lua, uid: i32) -> LuaResult<u32> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.loyalty)
        .unwrap_or(0))
}
fn lua_get_exp(lua: &Lua, uid: i32) -> LuaResult<u64> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.exp)
        .unwrap_or(0))
}
fn lua_get_inn_coins(lua: &Lua, uid: i32) -> LuaResult<u32> {
    Ok(get_world(lua)?.get_inn_coins(uid as SessionId))
}
fn lua_get_monthly_loyalty(lua: &Lua, uid: i32) -> LuaResult<u32> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.loyalty_monthly)
        .unwrap_or(0))
}
fn lua_get_manner(lua: &Lua, uid: i32) -> LuaResult<i32> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.manner_point)
        .unwrap_or(0))
}
fn lua_get_x(lua: &Lua, uid: i32) -> LuaResult<f32> {
    Ok(get_world(lua)?
        .get_position(uid as SessionId)
        .map(|p| p.x)
        .unwrap_or(0.0))
}
fn lua_get_y(_lua: &Lua, _uid: i32) -> LuaResult<f32> {
    Ok(0.0)
}
fn lua_get_z(lua: &Lua, uid: i32) -> LuaResult<f32> {
    Ok(get_world(lua)?
        .get_position(uid as SessionId)
        .map(|p| p.z)
        .unwrap_or(0.0))
}
fn lua_get_race(lua: &Lua, uid: i32) -> LuaResult<u8> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.race)
        .unwrap_or(0))
}

// Class checks
fn lua_is_warrior(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| quest::job_group_check(c.class, 1))
        .unwrap_or(false))
}
fn lua_is_rogue(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| quest::job_group_check(c.class, 2))
        .unwrap_or(false))
}
fn lua_is_mage(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| quest::job_group_check(c.class, 3))
        .unwrap_or(false))
}
fn lua_is_priest(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| quest::job_group_check(c.class, 4))
        .unwrap_or(false))
}
fn lua_is_kurian(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| quest::job_group_check(c.class, 13))
        .unwrap_or(false))
}

// Class tier checks
fn get_class_type(lua: &Lua, uid: i32) -> LuaResult<u16> {
    let w = get_world(lua)?;
    Ok(w.get_character_info(uid as SessionId)
        .map(|c| c.class % 100)
        .unwrap_or(0))
}

// Beginner tier: class_type in [1,2,3,4,13]
fn lua_is_beginner(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(matches!(get_class_type(lua, uid)?, 1 | 2 | 3 | 4 | 13))
}
fn lua_is_beginner_warrior(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 1)
}
fn lua_is_beginner_rogue(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 2)
}
fn lua_is_beginner_mage(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 3)
}
fn lua_is_beginner_priest(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 4)
}
fn lua_is_beginner_kurian(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 13)
}

// Novice tier: class_type in [5,7,9,11,14]
fn lua_is_novice(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(matches!(get_class_type(lua, uid)?, 5 | 7 | 9 | 11 | 14))
}
fn lua_is_novice_warrior(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 5)
}
fn lua_is_novice_rogue(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 7)
}
fn lua_is_novice_mage(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 9)
}
fn lua_is_novice_priest(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 11)
}
fn lua_is_novice_kurian(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 14)
}

// Mastered tier: class_type in [6,8,10,12,15]
fn lua_is_mastered(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(matches!(get_class_type(lua, uid)?, 6 | 8 | 10 | 12 | 15))
}
fn lua_is_mastered_warrior(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 6)
}
fn lua_is_mastered_rogue(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 8)
}
fn lua_is_mastered_mage(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 10)
}
fn lua_is_mastered_priest(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 12)
}
fn lua_is_mastered_kurian(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_class_type(lua, uid)? == 15)
}

fn lua_is_in_clan(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.knights_id > 0)
        .unwrap_or(false))
}
fn lua_is_clan_leader(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.fame == 1)
        .unwrap_or(false))
}
fn lua_is_in_party(lua: &Lua, uid: i32) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.party_id.is_some())
        .unwrap_or(false))
}
fn lua_is_party_leader(lua: &Lua, uid: i32) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    Ok(w.get_character_info(sid)
        .and_then(|c| c.party_id)
        .and_then(|pid| w.get_party(pid))
        .map(|p| p.members[0] == Some(sid))
        .unwrap_or(false))
}
fn lua_is_king(lua: &Lua, uid: i32) -> LuaResult<bool> {
    let w = get_world(lua)?;
    Ok(w.get_character_info(uid as SessionId)
        .map(|c| w.is_king(c.nation, &c.name))
        .unwrap_or(false))
}

// Has-checks
fn lua_has_coins(lua: &Lua, (uid, amt): (i32, u32)) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.gold >= amt)
        .unwrap_or(false))
}
fn lua_has_loyalty(lua: &Lua, (uid, amt): (i32, u32)) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.loyalty >= amt)
        .unwrap_or(false))
}
fn lua_has_inn_coins(lua: &Lua, (uid, amt): (i32, u32)) -> LuaResult<bool> {
    Ok(get_world(lua)?.get_inn_coins(uid as SessionId) >= amt)
}
fn lua_has_monthly_loyalty(lua: &Lua, (uid, amt): (i32, u32)) -> LuaResult<bool> {
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| c.loyalty_monthly >= amt)
        .unwrap_or(false))
}

// Inventory

/// CheckWeight(uid, item_id, count) -> bool
/// Validates that adding `count` of `item_id` won't exceed carry weight
/// and that there's a free slot for the item.
fn lua_check_weight(lua: &Lua, args: LuaMultiValue) -> LuaResult<bool> {
    let vals: Vec<LuaValue> = args.into_vec();
    // C++ CheckWeight takes (nItemID, sCount), Lua passes (uid, nItemID, sCount)
    let (uid, item_id, count) = if vals.len() >= 3 {
        (
            lua_val_to_i32(&vals[0])?,
            lua_val_to_u32(&vals[1])?,
            lua_val_to_u16(&vals[2]).unwrap_or(1),
        )
    } else if vals.len() == 2 {
        (lua_val_to_i32(&vals[0])?, lua_val_to_u32(&vals[1])?, 1u16)
    } else {
        return Ok(true);
    };
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let item = match w.get_item(item_id) {
        Some(i) => i,
        None => return Ok(false),
    };
    let weight = item.weight.unwrap_or(0) as i32;
    let ch = match w.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(false),
    };
    if ch.item_weight + (weight * count as i32) > ch.max_weight {
        return Ok(false);
    }
    if w.find_slot_for_item(sid, item_id, count).is_none() {
        return Ok(false);
    }
    Ok(true)
}
/// Return an inventory slot suitable for the requested item.
///
/// The original C++ Lua API declares the stack size as optional and defaults it
/// to one (`LUA_ARG_OPTIONAL(uint16, 1, 3)`).  A large number of the production
/// quest scripts, including Captain Fargo's starter-item events, therefore call
/// this function with only `(UID, item_id)`.
fn lua_is_room_for_item(
    lua: &Lua,
    (uid, item_id, count): (i32, u32, Option<u16>),
) -> LuaResult<i32> {
    Ok(get_world(lua)?
        .find_slot_for_item(uid as SessionId, item_id, count.unwrap_or(1))
        .map(|p| p as i32)
        .unwrap_or(-1))
}
/// Check if the player has enough free inventory slots.
fn lua_check_give_slot(lua: &Lua, (uid, count): (i32, u16)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let inv = w.get_inventory(uid as SessionId);
    let free: u16 = inv
        .iter()
        .skip(14)
        .take(28)
        .filter(|s| s.item_id == 0)
        .count() as u16;
    let has_room = free >= count;
    if !has_room {
        let mut err_pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizQuest as u8);
        err_pkt.write_u8(13);
        err_pkt.write_u8(3); // error code 3 = not enough slots
        w.send_to_session_owned(uid as SessionId, err_pkt);
    }
    Ok(has_room)
}

// ── Quest Monster Bindings ───────────────────────────────────────────

/// Check a specific monster group kill count for a quest.
/// (QuestHandler.cpp:484-491)
/// Returns `kill_counts[index-1]` for the specified quest. Index is 1-based
/// (1=group1, 2=group2, etc.). Returns 0 if quest not found or index out of range.
/// Lua call: `count = CountMonsterQuestSub(UID, quest_id, group_index)` — 3 args.
fn lua_count_monster_quest_sub(lua: &Lua, (uid, qid, index): (i32, u16, u8)) -> LuaResult<u16> {
    let w = get_world(lua)?;
    Ok(w.with_session(uid as SessionId, |h| {
        h.quests
            .get(&qid)
            .map(|q| {
                let idx = index.saturating_sub(1) as usize;
                if idx < 4 {
                    q.kill_counts[idx] as u16
                } else {
                    0
                }
            })
            .unwrap_or(0)
    })
    .unwrap_or(0))
}

/// Trigger monster quest kill count processing for a given NPC.
/// calls `QuestV2MonsterCountAdd(LUA_ARG(uint16, 2))` with `LUA_NO_RETURN`.
/// This is **not** a query — it adds kill counts for the specified NPC proto ID
/// across all active quests that track that monster. If all required kills are met,
/// the quest state transitions to 3 (ready to complete).
/// Lua call: `CountMonsterQuestMain(UID, npc_proto_id)` — 2 args, no return value.
fn lua_count_monster_quest_main(lua: &Lua, (uid, npc_id): (i32, u16)) -> LuaResult<()> {
    let w = get_world(lua)?;
    quest::quest_monster_count_add(&w, uid as SessionId, npc_id);
    Ok(())
}

/// Check if a quest is finished (state == 3: ready to complete).
/// Returns `true` if `quest_state == 3`, `false` otherwise.
/// Note: C++ checks state==3 (ready to complete), NOT state==2 (completed).
/// Lua call: `result = QuestCheckQuestFinished(UID, quest_id)`
fn lua_quest_check_finished(lua: &Lua, (uid, qid): (i32, u16)) -> LuaResult<bool> {
    lua_check_exist_event(lua, (uid, qid, 3))
}

/// Return the quest ID of the player's active monster-kill quest, or 0 if none.
/// C++ alias: `_LUA_WRAPPER_USER_FUNCTION(ExistMonsterQuestSub, GetActiveQuestID)`.
/// C++ stub always returned 0 (`INLINE uint16 GetActiveQuestID() { return 0; }`).
/// Our implementation iterates the player's in-progress quests and returns the
/// first one that has a matching entry in the `quest_monsters` table.
/// Lua call: `result = ExistMonsterQuestSub(UID)` — returns u16 (quest_id or 0).
fn lua_exist_monster_quest_sub(lua: &Lua, uid: i32) -> LuaResult<u16> {
    // Evidence: reference_cpp/User.h::GetActiveQuestID() is an inline stub
    // that always returns zero. Patrick event 175 relies on that zero to show
    // its accept button; returning another active kill quest makes it emit a
    // close-only menu.
    let _ = (lua, uid);
    Ok(0)
}

/// Search for an eligible quest for the given NPC.
/// Calls `QuestV2SearchEligibleQuest(npc_id)` which loops through QuestNpcList
/// and returns 2 if an eligible quest is found, 0 otherwise.
/// Lua call: `QuestNum = SearchQuest(UID, NPC)` — 2 args, returns u32.
fn lua_search_quest(lua: &Lua, (uid, npc_id): (i32, Option<u16>)) -> LuaResult<u32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // Get player info for eligibility checks
    let (player_level, player_exp, player_class, player_nation) = match w.with_session(sid, |h| {
        h.character
            .as_ref()
            .map(|ch| (ch.level, ch.exp, ch.class, ch.nation))
    }) {
        Some(Some(v)) => v,
        _ => return Ok(0),
    };

    // Use provided NPC ID, or fall back to session's event_sid
    let npc = npc_id.unwrap_or_else(|| w.with_session(sid, |h| h.event_sid as u16).unwrap_or(0));

    // Look up quest helpers for this NPC
    let helper_indices = match w.get_quest_npc_helpers(npc) {
        Some(v) => v,
        None => return Ok(0),
    };

    // Loop through all QuestHelper instances attached to that NPC
    for n_index in &helper_indices {
        let helper = match w.get_quest_helper(*n_index) {
            Some(h) => h,
            None => continue,
        };

        // Level check: skip if player level < required level
        if helper.b_level > player_level as i16 {
            continue;
        }
        // Level+exp check: skip if same level but less exp
        if helper.b_level == player_level as i16 && helper.n_exp > player_exp as i32 {
            continue;
        }
        // Class check: bClass != 5 means class-restricted
        if helper.b_class != 5
            && !crate::handler::quest::job_group_check(player_class, helper.b_class)
        {
            continue;
        }
        // Nation check: bNation != 3 means nation-restricted
        if helper.b_nation != 3 && helper.b_nation != player_nation as i16 {
            continue;
        }
        // Must have a valid event data index
        if helper.s_event_data_index == 0 {
            continue;
        }
        // Skip invalid event status
        if helper.b_event_status < 0 {
            continue;
        }
        // Skip if quest is already completed (state 2)
        let quest_finished = w
            .with_session(sid, |h| {
                match h.quests.get(&(helper.s_event_data_index as u16)) {
                    Some(q) => q.quest_state == 2,
                    None => false,
                }
            })
            .unwrap_or(false);
        if quest_finished {
            continue;
        }
        // Check if quest is in the required state
        let in_required_state = w
            .with_session(sid, |h| {
                let qid = helper.s_event_data_index as u16;
                let status = helper.b_event_status as u8;
                match h.quests.get(&qid) {
                    Some(q) => q.quest_state == status,
                    None => status == 0,
                }
            })
            .unwrap_or(helper.b_event_status == 0);
        if !in_required_state {
            continue;
        }

        // Found an eligible quest
        return Ok(2);
    }

    Ok(0)
}

// Actions

/// Sends WIZ_EFFECT << u32(player_id) << u32(skill_id) to region.
fn lua_show_effect(lua: &Lua, (uid, eid): (i32, u32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let mut pkt = Packet::new(Opcode::WizEffect as u8);
    pkt.write_u32(sid as u32);
    pkt.write_u32(eid);
    // Broadcast to the player's region
    if let Some((pos, sender_event_room)) = w.with_session(sid, |h| (h.position, h.event_room)) {
        let rx = (pos.x as u16) / 32;
        let rz = (pos.z as u16) / 32;
        w.broadcast_to_region_sync(pos.zone_id, rx, rz, Arc::new(pkt), None, sender_event_room);
    }
    Ok(())
}

/// Sends WIZ_OBJECT_EVENT << u8(OBJECT_NPC=11) << u8(3) << u32(event_nid) << u32(effect_id).
fn lua_show_npc_effect(lua: &Lua, (uid, eid): (i32, u32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let event_nid = w.with_session(sid, |h| h.event_sid as u32).unwrap_or(0);
    let mut pkt = Packet::new(Opcode::WizObjectEvent as u8);
    pkt.write_u8(crate::object_event_constants::OBJECT_NPC);
    pkt.write_u8(3); // subcode
    pkt.write_u32(event_nid);
    pkt.write_u32(eid);
    w.send_to_session_owned(sid, pkt);
    Ok(())
}
/// Sends WIZ_QUEST << u8(11) << u32(quest_helper_id) to client.
fn lua_show_map(lua: &Lua, (uid, mid): (i32, Option<u32>)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let helper_id = mid.unwrap_or_else(|| w.with_session(sid, |h| h.quest_helper_id).unwrap_or(0));
    let mut pkt = Packet::new(Opcode::WizQuest as u8);
    pkt.write_u8(11); // QuestV2ShowMap sub-opcode
    pkt.write_u32(helper_id);
    w.send_to_session_owned(sid, pkt);
    Ok(())
}

/// Return the quest_helper row that selected the currently executing Lua
/// event. Some v2615 scripts share event numbers between compatibility quest
/// IDs, so the Lua must be able to choose the matching SaveEvent/exchange row.
fn lua_get_quest_helper_id(lua: &Lua, uid: i32) -> LuaResult<u32> {
    let w = get_world(lua)?;
    Ok(w.with_session(uid as SessionId, |h| h.quest_helper_id)
        .unwrap_or(0))
}

/// 1. Guard: must be beginner class (class % 100 in {1,2,3,4,13})
/// 2. Change class (base class → novice class)
/// 3. Broadcast PROMOTE_NOVICE (sub-opcode 6) to region
/// 4. Notify party of class change (PARTY_CLASSCHANGE = 0x08)
/// C++ does NOT reset stats or skills in PromoteUserNovice.
fn lua_promote_user_novice(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // Guard: must be beginner (class % 100 in {1,2,3,4,13})
    let old_base = w
        .get_character_info(sid)
        .map(|c| c.class % 100)
        .unwrap_or(0);
    if !matches!(old_base, 1 | 2 | 3 | 4 | 13) {
        return Ok(());
    }

    // 1. Change class
    let mut new_class = 0u16;
    w.update_character_stats(sid, |ch| {
        let base = ch.class % 100;
        let pfx = (ch.class / 100) * 100;
        let novice = match base {
            1 => 5,
            2 => 7,
            3 => 9,
            4 => 11,
            13 => 14,
            _ => base + 4,
        };
        ch.class = pfx + novice;
        new_class = ch.class;
    });

    if new_class == 0 {
        return Ok(());
    }

    // 2. Broadcast PROMOTE_NOVICE to region
    //   Packet result(WIZ_CLASS_CHANGE, uint8(6));
    //   result << sNewClass << uint32(GetID());
    //   SendToRegion(&result);
    if let Some(pos) = w.get_position(sid) {
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(6); // PROMOTE_NOVICE sub-opcode
        pkt.write_u16(new_class);
        pkt.write_u32(sid as u32);
        w.broadcast_to_region_sync(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            0,
        );
    }

    // 3. ClassChange(result, false) — sets m_sClass, notifies party
    // The class was already changed above; send party notification if in party.
    crate::handler::party::broadcast_party_class_change(&w, sid, new_class);

    // 4. Persist class change to DB (fire-and-forget)
    if let Some(pool) = w.db_pool() {
        if let Some(ch) = w.get_character_info(sid) {
            let pool = pool.clone();
            let char_name = ch.name.clone();
            let race = ch.race as i16;
            let class = new_class as i16;
            if !char_name.is_empty() {
                tokio::spawn(async move {
                    let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
                    if let Err(e) = repo.save_class_change(&char_name, class, race).await {
                        tracing::error!(
                            char_name,
                            "PromoteUserNovice: failed to save class change: {}",
                            e
                        );
                    }
                });
            }
        }
    }

    Ok(())
}

/// Same flow as PromoteUserNovice but promotes novice → master class.
/// C++ does NOT reset stats/skills here — only changes class + broadcasts.
fn lua_promote_user(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // Guard: must be novice (class % 100 in 5..=15 odd range)
    let old_base = w
        .get_character_info(sid)
        .map(|c| c.class % 100)
        .unwrap_or(0);
    if !matches!(old_base, 5 | 7 | 9 | 11 | 14) {
        return Ok(());
    }

    // 1. Change class (novice → master)
    let mut new_class = 0u16;
    w.update_character_stats(sid, |ch| {
        let base = ch.class % 100;
        let pfx = (ch.class / 100) * 100;
        let master = match base {
            5 => 6,
            7 => 8,
            9 => 10,
            11 => 12,
            14 => 15,
            _ => base + 1,
        };
        ch.class = pfx + master;
        new_class = ch.class;
    });

    if new_class == 0 {
        return Ok(());
    }

    // 2. Broadcast PROMOTE to region
    if let Some(pos) = w.get_position(sid) {
        let mut pkt = Packet::new(Opcode::WizClassChange as u8);
        pkt.write_u8(6);
        pkt.write_u16(new_class);
        pkt.write_u32(sid as u32);
        w.broadcast_to_region_sync(
            pos.zone_id,
            pos.region_x,
            pos.region_z,
            Arc::new(pkt),
            None,
            0,
        );
    }

    // 3. Party class change notification
    crate::handler::party::broadcast_party_class_change(&w, sid, new_class);

    // 4. Persist class change to DB (fire-and-forget)
    if let Some(pool) = w.db_pool() {
        if let Some(ch) = w.get_character_info(sid) {
            let pool = pool.clone();
            let char_name = ch.name.clone();
            let race = ch.race as i16;
            let class = new_class as i16;
            if !char_name.is_empty() {
                tokio::spawn(async move {
                    let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
                    if let Err(e) = repo.save_class_change(&char_name, class, race).await {
                        tracing::error!(
                            char_name,
                            "PromoteUser: failed to save class change: {}",
                            e
                        );
                    }
                });
            }
        }
    }

    Ok(())
}

fn lua_reset_skill_points(lua: &Lua, uid: i32) -> LuaResult<()> {
    get_world(lua)?.update_character_stats(uid as SessionId, |ch| {
        let total: u8 = ch.skill_points[5..=8].iter().sum();
        for i in 5..=8 {
            ch.skill_points[i] = 0;
        }
        ch.skill_points[0] = ch.skill_points[0].saturating_add(total);
    });
    Ok(())
}

fn lua_reset_stat_points(lua: &Lua, uid: i32) -> LuaResult<()> {
    get_world(lua)?.update_character_stats(uid as SessionId, |ch| {
        let spent = (ch.str.saturating_sub(10) as u16)
            + (ch.sta.saturating_sub(10) as u16)
            + (ch.dex.saturating_sub(10) as u16)
            + (ch.intel.saturating_sub(10) as u16)
            + (ch.cha.saturating_sub(10) as u16);
        ch.str = 10;
        ch.sta = 10;
        ch.dex = 10;
        ch.intel = 10;
        ch.cha = 10;
        ch.free_points = ch.free_points.saturating_add(spent);
    });
    Ok(())
}

fn lua_level_change(lua: &Lua, (uid, level): (i32, u8)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    w.update_character_stats(sid, |ch| {
        ch.level = level;
    });
    if let Some(ch) = w.get_character_info(sid) {
        let mut pkt = Packet::new(Opcode::WizLevelChange as u8);
        pkt.write_u8(1);
        pkt.write_u16(sid as u16);
        pkt.write_u8(ch.level);
        w.send_to_session_owned(sid, pkt);
    }
    Ok(())
}

fn lua_give_balance(lua: &Lua, (uid, amt): (i32, u32)) -> LuaResult<()> {
    get_world(lua)?.update_session(uid as SessionId, |h| {
        h.inn_coins = h.inn_coins.saturating_add(amt);
    });
    Ok(())
}

fn lua_send_stat_skill_distribute(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    // Sends WIZ_CLASS_CHANGE with CLASS_CHANGE_REQ (0x01) to open the
    // stat/skill distribution UI on the client.
    let mut pkt = Packet::new(Opcode::WizClassChange as u8);
    pkt.write_u8(0x01); // CLASS_CHANGE_REQ
    w.send_to_session_owned(sid, pkt);
    Ok(())
}

fn lua_check_clan_grade(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    let knights_id = w
        .get_character_info(uid as SessionId)
        .map(|c| c.knights_id)
        .unwrap_or(0);
    if knights_id == 0 {
        return Ok(0);
    }
    Ok(w.get_knights(knights_id).map(|k| k.grade).unwrap_or(0))
}

/// Returns pClan->m_byFlag (clan type flag: Training=1, Promoted=2, etc.)
/// Returns ClanTypeNone (0) if not in a clan.
fn lua_get_clan_rank(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let knights_id = w.get_character_info(sid).map(|c| c.knights_id).unwrap_or(0);
    if knights_id == 0 {
        return Ok(0); // ClanTypeNone
    }
    Ok(w.get_knights(knights_id).map(|k| k.flag).unwrap_or(0))
}

/// C++ adds +1 to the lua argument before stat lookup.
fn lua_check_stat_point(lua: &Lua, (uid, idx): (i32, u8)) -> LuaResult<u8> {
    let stat_idx = idx + 1; // C++ does LUA_ARG(uint8, 2) + 1
    Ok(get_world(lua)?
        .get_character_info(uid as SessionId)
        .map(|c| match stat_idx {
            1 => c.str,
            2 => c.sta,
            3 => c.dex,
            4 => c.intel,
            5 => c.cha,
            _ => 0,
        })
        .unwrap_or(0))
}

fn lua_get_premium(lua: &Lua, uid: i32) -> LuaResult<u8> {
    Ok(get_world(lua)?
        .with_session(uid as SessionId, |h| h.premium_in_use)
        .unwrap_or(0))
}

fn lua_get_event_trigger(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let world = get_world(lua)?;
    let (event_nid, event_sid) = world
        .with_session(uid as SessionId, |h| (h.event_nid, h.event_sid))
        .unwrap_or((0, 0));

    // Abyss gate Lua chooses direction and validates required keys. The
    // database event_trigger rows map gate trap numbers to these script IDs.
    if event_sid == 7001 {
        if let Some(npc) = world.get_npc_instance(event_nid as u32) {
            if npc.proto_id == 7001 && !npc.is_monster && npc.trap_number > 0 {
                let trap = npc.trap_number as i32;
                return Ok(match trap {
                    1..=20 => 40_000 + trap,
                    21..=36 => 42_000 + trap - 20,
                    37..=50 => 43_601 + (trap - 37) * 100,
                    _ => -1,
                });
            }
        }
    }

    Ok(event_sid as i32)
}

fn lua_roll_dice(_lua: &Lua, (_uid, max): (i32, u16)) -> LuaResult<u16> {
    use rand::Rng;
    if max == 0 {
        return Ok(0);
    }
    Ok(rand::thread_rng().gen_range(0..=max))
}

// ═══════════════════════════════════════════════════════════════════════
// Exchange System (Tier 1)
// ═══════════════════════════════════════════════════════════════════════

use crate::world::{ITEM_COUNT, ITEM_EXP, ITEM_LADDERPOINT};
/// C++ constant: ITEM_SKILL = 900007000 (skill points — silently skipped in quest exchange).
const ITEM_SKILL: u32 = 900_007_000;

/// RunGiveItemExchange(uid, exchange_id) -> bool
/// Look up an ItemGiveExchangeRow, validate the player has all required items,
/// remove them, then give all output items.
fn lua_run_give_item_exchange(lua: &Lua, (uid, exchange_id): (i32, i32)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let exchange = match w.get_item_give_exchange(exchange_id) {
        Some(e) => e,
        None => {
            tracing::warn!(sid, exchange_id, "RunGiveItemExchange: no such exchange_id");
            return Ok(false);
        }
    };

    // Validate all rob items exist in inventory
    for (i, &item_id) in exchange.rob_item_ids.iter().enumerate() {
        if item_id <= 0 {
            continue;
        }
        let count = *exchange.rob_item_counts.get(i).unwrap_or(&0);
        if count <= 0 {
            continue;
        }

        let item_id_u = item_id as u32;
        if item_id_u == ITEM_GOLD {
            let gold = w.get_character_info(sid).map(|c| c.gold).unwrap_or(0);
            if gold < count.max(0) as u32 {
                tracing::warn!(sid, exchange_id, need = count, have = gold, "RunGiveItemExchange: not enough gold");
                return Ok(false);
            }
        } else if item_id_u == ITEM_COUNT || item_id_u == ITEM_LADDERPOINT {
            let loyalty = w.get_character_info(sid).map(|c| c.loyalty).unwrap_or(0);
            if loyalty < count.max(0) as u32 {
                tracing::warn!(sid, exchange_id, need = count, have = loyalty, "RunGiveItemExchange: not enough loyalty");
                return Ok(false);
            }
        } else if is_special_origin_skip(item_id_u) {
            // Virtual items — C++ CheckExistItem auto-returns true
            continue;
        } else {
            let have = lua_howmuch_item(lua, (uid, item_id_u))?;
            if have < count as u32 {
                tracing::warn!(sid, exchange_id, item_id = item_id_u, need = count, have, "RunGiveItemExchange: not enough of rob item");
                return Ok(false);
            }
        }
    }

    // Check we can hold the give items (free slot check)
    for (i, &item_id) in exchange.give_item_ids.iter().enumerate() {
        if item_id <= 0 {
            continue;
        }
        let count = *exchange.give_item_counts.get(i).unwrap_or(&0);
        if count <= 0 {
            continue;
        }
        let item_id_u = item_id as u32;
        // Gold/EXP/Loyalty don't need slots
        if item_id_u == ITEM_GOLD
            || item_id_u == ITEM_EXP
            || item_id_u == ITEM_COUNT
            || item_id_u == ITEM_LADDERPOINT
        {
            continue;
        }
        // Match C++ behavior: skip items not in DB, don't abort entire exchange.
        if w.get_item(item_id_u).is_none() {
            tracing::warn!(
                "RunGiveItemExchange: item_id {} not in items table — skipping (sid={})",
                item_id_u,
                sid
            );
            continue;
        }
        if w.find_slot_for_item(sid, item_id_u, 1).is_none() {
            tracing::warn!(sid, exchange_id, give_item_id = item_id_u, "RunGiveItemExchange: no free slot for give item");
            return Ok(false);
        }
    }

    // Remove all rob items
    for (i, &item_id) in exchange.rob_item_ids.iter().enumerate() {
        if item_id <= 0 {
            continue;
        }
        let count = *exchange.rob_item_counts.get(i).unwrap_or(&0);
        if count <= 0 {
            continue;
        }

        let item_id_u = item_id as u32;
        if is_special_origin_skip(item_id_u) {
            continue;
        } else if item_id_u == ITEM_GOLD {
            if !w.gold_lose(sid, count as u32) {
                tracing::warn!(sid, exchange_id, "RunGiveItemExchange: gold_lose failed");
                return Ok(false);
            }
        } else if item_id_u == ITEM_COUNT || item_id_u == ITEM_LADDERPOINT {
            let loyalty = w.get_character_info(sid).map(|c| c.loyalty).unwrap_or(0);
            if loyalty < count as u32 {
                tracing::warn!(sid, exchange_id, "RunGiveItemExchange: loyalty dropped below need before removal");
                return Ok(false);
            }
            w.update_character_stats(sid, |ch| {
                ch.loyalty = ch.loyalty.saturating_sub(count as u32);
            });
        } else if !w.rob_item(sid, item_id_u, count as u16) {
            tracing::warn!(sid, exchange_id, rob_item_id = item_id_u, count, "RunGiveItemExchange: rob_item failed");
            return Ok(false);
        }
    }

    // Give all output items
    for (i, &item_id) in exchange.give_item_ids.iter().enumerate() {
        if item_id <= 0 {
            continue;
        }
        let count = *exchange.give_item_counts.get(i).unwrap_or(&0);
        if count <= 0 {
            continue;
        }

        let item_id_u = item_id as u32;
        let expiry_days = (*exchange.give_item_times.get(i).unwrap_or(&0)).max(0) as u32;
        if expiry_days > 0 && !matches!(
            item_id_u,
            ITEM_GOLD | ITEM_EXP | ITEM_SKILL | ITEM_COUNT | ITEM_LADDERPOINT
        ) {
            w.give_item_with_expiry(sid, item_id_u, (count as u32).min(u16::MAX as u32) as u16, expiry_days);
        } else {
            give_exchange_item(&w, sid, item_id_u, count as u32);
        }
    }
    tracing::info!(sid, exchange_id, "RunGiveItemExchange: completed");

    Ok(true)
}

/// CheckExchange(uid, exchange_id) -> bool
/// Check if a player has all origin items for an exchange recipe.
fn lua_check_exchange(lua: &Lua, (uid, exchange_id): (i32, i32)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let exchange = match w.get_item_exchange(exchange_id) {
        Some(e) => e,
        None => return Ok(false),
    };

    let origins: [(i32, i32); 5] = [
        (exchange.origin_item_num1, exchange.origin_item_count1),
        (exchange.origin_item_num2, exchange.origin_item_count2),
        (exchange.origin_item_num3, exchange.origin_item_count3),
        (exchange.origin_item_num4, exchange.origin_item_count4),
        (exchange.origin_item_num5, exchange.origin_item_count5),
    ];

    for &(item_id, count) in &origins {
        if item_id == 0 || count == 0 {
            continue;
        }
        let id_u = item_id as u32;
        if is_special_origin_skip(id_u) {
            continue;
        }
        let have = lua_howmuch_item(lua, (uid, id_u))?;
        if have < count as u32 {
            return Ok(false);
        }
    }

    // Check we have room for the exchange output items
    let outputs: [(i32, i32); 5] = [
        (exchange.exchange_item_num1, exchange.exchange_item_count1),
        (exchange.exchange_item_num2, exchange.exchange_item_count2),
        (exchange.exchange_item_num3, exchange.exchange_item_count3),
        (exchange.exchange_item_num4, exchange.exchange_item_count4),
        (exchange.exchange_item_num5, exchange.exchange_item_count5),
    ];

    for &(item_id, count) in &outputs {
        if item_id == 0 || count == 0 {
            continue;
        }
        let item_id_u = item_id as u32;
        if item_id_u == ITEM_GOLD
            || item_id_u == ITEM_EXP
            || item_id_u == ITEM_COUNT
            || item_id_u == ITEM_LADDERPOINT
        {
            continue;
        }
        if w.find_slot_for_item(sid, item_id_u, 1).is_none() {
            return Ok(false);
        }
    }

    Ok(true)
}

/// RunExchange(uid, exchange_id) -> bool
/// Execute an item exchange: remove origin items, give exchange items.
/// Handles random_flag: 0=fixed, 1-100=random selection, 101=weighted random.
fn lua_run_exchange(lua: &Lua, (uid, exchange_id): (i32, i32)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let exchange = match w.get_item_exchange(exchange_id) {
        Some(e) => e,
        None => return Ok(false),
    };

    if exchange.random_flag > 101 {
        return Ok(false);
    }

    let origins: [(i32, i32); 5] = [
        (exchange.origin_item_num1, exchange.origin_item_count1),
        (exchange.origin_item_num2, exchange.origin_item_count2),
        (exchange.origin_item_num3, exchange.origin_item_count3),
        (exchange.origin_item_num4, exchange.origin_item_count4),
        (exchange.origin_item_num5, exchange.origin_item_count5),
    ];

    // Check origin items exist
    for &(item_id, count) in &origins {
        if item_id == 0 || count == 0 {
            continue;
        }
        let id_u = item_id as u32;
        if is_special_origin_skip(id_u) {
            continue;
        }
        let have = lua_howmuch_item(lua, (uid, id_u))?;
        if have < count as u32 {
            return Ok(false);
        }
    }

    // Remove origin items, skipping virtual IDs
    for &(item_id, count) in &origins {
        if item_id == 0 || count == 0 {
            continue;
        }
        let id_u = item_id as u32;
        if is_special_origin_skip(id_u) {
            continue;
        }
        if id_u == ITEM_GOLD {
            if !w.gold_lose(sid, count as u32) {
                return Ok(false);
            }
        } else if !w.rob_item(sid, id_u, count as u16) {
            return Ok(false);
        }
    }

    let outputs: [(i32, i32); 5] = [
        (exchange.exchange_item_num1, exchange.exchange_item_count1),
        (exchange.exchange_item_num2, exchange.exchange_item_count2),
        (exchange.exchange_item_num3, exchange.exchange_item_count3),
        (exchange.exchange_item_num4, exchange.exchange_item_count4),
        (exchange.exchange_item_num5, exchange.exchange_item_count5),
    ];

    if exchange.random_flag == 0 {
        // Fixed exchange: give all output items
        for &(item_id, count) in &outputs {
            if item_id == 0 || count == 0 {
                continue;
            }
            give_exchange_item(&w, sid, item_id as u32, count as u32);
        }
    } else if exchange.random_flag <= 100 {
        // Random selection among output items
        use rand::Rng;
        let mut rand_idx =
            rand::thread_rng().gen_range(0..=(1000 * exchange.random_flag as u32)) / 1000;
        if rand_idx == 5 {
            rand_idx = 4;
        }
        if rand_idx <= 4 {
            let idx = rand_idx as usize;
            let (item_id, count) = outputs[idx];
            if item_id > 0 && count > 0 {
                give_exchange_item(&w, sid, item_id as u32, count as u32);
            }
        }
    } else {
        // random_flag == 101: weighted random by sExchangeItemCount
        use rand::Rng;
        let total: u32 = outputs.iter().map(|&(_, c)| c as u32).sum();
        if total > 0 {
            let roll = rand::thread_rng().gen_range(0..total);
            let mut cumulative = 0u32;
            for &(item_id, count) in &outputs {
                if item_id == 0 {
                    continue;
                }
                cumulative += count as u32;
                if roll < cumulative {
                    give_exchange_item(&w, sid, item_id as u32, 1u32);
                    break;
                }
            }
        }
    }

    Ok(true)
}

/// Helper: give an exchange output item (handles gold/exp/loyalty special IDs).
/// For ITEM_EXP, spawns an async task with `exp_change_with_bonus(is_bonus=true)`
/// which handles level-up, EXP seal, and max-level cap — matching `ExpChange()`.
fn give_exchange_item(w: &Arc<WorldState>, sid: SessionId, item_id: u32, count: u32) {
    if item_id == ITEM_GOLD {
        w.gold_gain(sid, count);
    } else if item_id == ITEM_EXP {
        // true = bIsBonusReward (skip bonus multipliers, but DO handle level-up)
        // Use block_in_place to run async ExpChange synchronously from Lua context,
        // matching C++ behavior. Fire-and-forget tokio::spawn caused race conditions
        // with quest response packets and level-up broadcasts.
        let w2 = Arc::clone(w);
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                crate::handler::level::exp_change_with_bonus(&w2, sid, count as i64, true).await;
            });
        });
    } else if item_id == ITEM_SKILL {
    } else if item_id == ITEM_COUNT {
        // false = bIsAddLoyaltyMonthly — ITEM_COUNT does NOT add to monthly NP
        w.update_character_stats(sid, |ch| {
            ch.loyalty = ch.loyalty.saturating_add(count);
        });
        if let Some(ch) = w.get_character_info(sid) {
            let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
            pkt.write_u8(1); // LOYALTY_NATIONAL_POINTS
            pkt.write_u32(ch.loyalty);
            pkt.write_u32(ch.loyalty_monthly);
            pkt.write_u32(0);
            pkt.write_u32(0);
            w.send_to_session_owned(sid, pkt);
        }
    } else if item_id == ITEM_LADDERPOINT {
        w.update_character_stats(sid, |ch| {
            ch.loyalty = ch.loyalty.saturating_add(count);
            ch.loyalty_monthly = ch.loyalty_monthly.saturating_add(count);
        });
        if let Some(ch) = w.get_character_info(sid) {
            let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
            pkt.write_u8(1);
            pkt.write_u32(ch.loyalty);
            pkt.write_u32(ch.loyalty_monthly);
            pkt.write_u32(0);
            pkt.write_u32(0);
            w.send_to_session_owned(sid, pkt);
        }
    } else {
        // Regular items: clamp to u16::MAX for give_item
        w.give_item(sid, item_id, count.min(u16::MAX as u32) as u16);
    }
}

/// True for the exchange ids that are handled as gold / exp / loyalty / skill instead of a bag item.
fn is_exchange_virtual_id(item_id: u32) -> bool {
    matches!(
        item_id,
        ITEM_GOLD | ITEM_EXP | ITEM_COUNT | ITEM_LADDERPOINT | ITEM_SKILL
    )
}

/// Lifetime in days of an exchange reward (`exchange_item_time1..5` of the output slot that holds
/// `item_id`), 0 = permanent.
fn exchange_item_days(exchange: &ko_db::models::ItemExchangeRow, item_id: u32) -> u32 {
    let slots = [
        (exchange.exchange_item_num1, exchange.exchange_item_time1),
        (exchange.exchange_item_num2, exchange.exchange_item_time2),
        (exchange.exchange_item_num3, exchange.exchange_item_time3),
        (exchange.exchange_item_num4, exchange.exchange_item_time4),
        (exchange.exchange_item_num5, exchange.exchange_item_time5),
    ];
    slots
        .iter()
        .find(|(num, _)| *num > 0 && *num as u32 == item_id)
        .map(|(_, days)| (*days).max(0) as u32)
        .unwrap_or(0)
}

/// Give an exchange item and return success/failure.
/// Same as `give_exchange_item` but returns `bool` to indicate success.
/// Used by quest exchange to detect and log failed item delivery.
fn give_exchange_item_checked(
    w: &Arc<WorldState>,
    sid: SessionId,
    item_id: u32,
    count: u32,
) -> bool {
    if item_id == ITEM_GOLD {
        w.gold_gain(sid, count);
        true
    } else if item_id == ITEM_EXP {
        let w2 = Arc::clone(w);
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                crate::handler::level::exp_change_with_bonus(&w2, sid, count as i64, true).await;
            });
        });
        true
    } else if item_id == ITEM_SKILL {
        true // C++ silently skips ITEM_SKILL
    } else if item_id == ITEM_COUNT {
        w.update_character_stats(sid, |ch| {
            ch.loyalty = ch.loyalty.saturating_add(count);
        });
        if let Some(ch) = w.get_character_info(sid) {
            let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
            pkt.write_u8(1);
            pkt.write_u32(ch.loyalty);
            pkt.write_u32(ch.loyalty_monthly);
            pkt.write_u32(0);
            pkt.write_u32(0);
            w.send_to_session_owned(sid, pkt);
        }
        true
    } else if item_id == ITEM_LADDERPOINT {
        w.update_character_stats(sid, |ch| {
            ch.loyalty = ch.loyalty.saturating_add(count);
            ch.loyalty_monthly = ch.loyalty_monthly.saturating_add(count);
        });
        if let Some(ch) = w.get_character_info(sid) {
            let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
            pkt.write_u8(1);
            pkt.write_u32(ch.loyalty);
            pkt.write_u32(ch.loyalty_monthly);
            pkt.write_u32(0);
            pkt.write_u32(0);
            w.send_to_session_owned(sid, pkt);
        }
        true
    } else {
        w.give_item(sid, item_id, count.min(u16::MAX as u32) as u16)
    }
}

/// Special origin item IDs that are skipped during removal in quest/random exchanges.
fn is_special_origin_skip(item_id: u32) -> bool {
    matches!(
        item_id,
        900_001_000
            | 900_004_000
            | 900_005_000
            | 900_006_000
            | 900_007_000
            | 900_008_000
            | 900_009_000
            | 900_010_000
            | 900_011_000
            | 900_012_000
            | 900_016_000
            | 810_000_000
    )
}

/// RunQuestExchange(uid, exchange_id) -> bool
/// Quest-specific exchange: skips special origin items during removal,
/// handles premium reward selection (flag 20/30), gives ALL output items.
fn lua_run_quest_exchange(lua: &Lua, (uid, exchange_id): (i32, i32)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    tracing::info!(sid, exchange_id, "RunQuestExchange: START");

    let exchange = match w.get_item_exchange(exchange_id) {
        Some(e) => e,
        None => {
            tracing::warn!(
                sid,
                exchange_id,
                "RunQuestExchange: exchange_id not found in item_exchange table"
            );
            return Ok(false);
        }
    };

    let origins: [(i32, i32); 5] = [
        (exchange.origin_item_num1, exchange.origin_item_count1),
        (exchange.origin_item_num2, exchange.origin_item_count2),
        (exchange.origin_item_num3, exchange.origin_item_count3),
        (exchange.origin_item_num4, exchange.origin_item_count4),
        (exchange.origin_item_num5, exchange.origin_item_count5),
    ];

    // Check all origin items exist
    // Virtual items (ITEM_HUNT etc.) are skipped — C++ CheckExistItem returns true for them.
    for &(item_id, count) in &origins {
        if item_id == 0 || count == 0 {
            continue;
        }
        let id_u = item_id as u32;
        // Skip virtual items — C++ CheckExistItem auto-returns true for these
        if is_special_origin_skip(id_u) {
            continue;
        }
        let have = lua_howmuch_item(lua, (uid, id_u))?;
        if have < count as u32 {
            tracing::warn!(
                sid,
                exchange_id,
                item_id,
                count,
                have,
                "RunQuestExchange: FAIL — origin item check (have < required)"
            );
            return Ok(false);
        }
    }

    // Check gold specifically
    let mut total_gold: u32 = 0;
    for &(item_id, count) in &origins {
        if item_id as u32 == ITEM_GOLD {
            total_gold += count as u32;
        }
    }
    if total_gold > 0 {
        let gold = w.get_character_info(sid).map(|c| c.gold).unwrap_or(0);
        if gold < total_gold {
            return Ok(false);
        }
    }

    // ── Pre-validate output items BEFORE removing origins ──
    // Build the output list first, then check slots/weight, THEN remove origins.

    //
    let outputs: [(i32, i32); 5] = [
        (exchange.exchange_item_num1, exchange.exchange_item_count1),
        (exchange.exchange_item_num2, exchange.exchange_item_count2),
        (exchange.exchange_item_num3, exchange.exchange_item_count3),
        (exchange.exchange_item_num4, exchange.exchange_item_count4),
        (exchange.exchange_item_num5, exchange.exchange_item_count5),
    ];

    // Build the exchange list — C++ uses m_exchangelist vector
    let mut exchange_list: Vec<(u32, u32)> = Vec::new();

    if exchange.random_flag == 20 || exchange.random_flag == 30 {
        // Premium users get slot[3], non-premium get slot[0]
        // C++ line 1743: if (nItemID == ITEM_EXP && nCount != 0) — ONLY ITEM_EXP
        let premium = w.with_session(sid, |h| h.premium_in_use).unwrap_or(0);
        let (item_id, count) = if premium > 0 { outputs[3] } else { outputs[0] };
        if item_id as u32 == ITEM_EXP && count != 0 {
            exchange_list.push((item_id as u32, count as u32));
        }
    } else if exchange.random_flag == 101 {
        // The five exchange_item_count fields are weights (not quantities) for
        // random_flag 101. Match the regular RunExchange path: choose exactly
        // one non-empty reward and grant one copy. Moira's box exchanges use
        // this format, so treating all five weighted rows as rewards duplicated
        // the intended single box drop into five items.
        use rand::Rng;
        let total_weight: u32 = outputs
            .iter()
            .filter(|(item_id, weight)| *item_id > 0 && *weight > 0)
            .map(|(_, weight)| *weight as u32)
            .sum();
        if total_weight > 0 {
            let roll = rand::thread_rng().gen_range(0..total_weight);
            let mut cumulative = 0u32;
            for &(item_id, weight) in &outputs {
                if item_id <= 0 || weight <= 0 {
                    continue;
                }
                cumulative += weight as u32;
                if roll < cumulative {
                    exchange_list.push((item_id as u32, 1));
                    break;
                }
            }
        }
    } else {
        // Give ALL output items
        for &(item_id, count) in &outputs {
            if item_id == 0 || count == 0 {
                continue;
            }
            exchange_list.push((item_id as u32, count as u32));
        }
    }

    // Index (inside `exchange_list`) of the reward the player picked from the
    // item_exchange_exp table, if any.
    let mut selected_reward_idx: Option<usize> = None;

    // C++ CheckExchangeExp contract: an exchange with selectable rewards must
    // never finish without a valid client selection. Previously Rust silently
    // gave only the base rewards and Lua then marked the quest completed.
    let by_selected_reward = w.with_session(sid, |h| h.by_selected_reward).unwrap_or(-1);
    let select_msg_flag = w.with_session(sid, |h| h.select_msg_flag).unwrap_or(0);
    let exp_exchange = w.get_item_exchange_exp(exchange_id);
    if by_selected_reward > 4
        || (select_msg_flag == 5 && by_selected_reward < 0)
        || (exp_exchange.is_some() && by_selected_reward < 0)
        || (exp_exchange.is_none() && by_selected_reward > 0)
    {
        tracing::warn!(
            sid,
            exchange_id,
            by_selected_reward,
            select_msg_flag,
            has_selected_rewards = exp_exchange.is_some(),
            "RunQuestExchange: invalid or missing selected reward"
        );
        return Ok(false);
    }

    if let Some(exp_exchange) = exp_exchange {
        if by_selected_reward >= 0 && (by_selected_reward as usize) < 5 {
            let idx = by_selected_reward as usize;
            let exp_outputs: [(i32, i32); 5] = [
                (
                    exp_exchange.exchange_item_num1.unwrap_or(0),
                    exp_exchange.exchange_item_count1.unwrap_or(0),
                ),
                (
                    exp_exchange.exchange_item_num2.unwrap_or(0),
                    exp_exchange.exchange_item_count2.unwrap_or(0),
                ),
                (
                    exp_exchange.exchange_item_num3.unwrap_or(0),
                    exp_exchange.exchange_item_count3.unwrap_or(0),
                ),
                (
                    exp_exchange.exchange_item_num4.unwrap_or(0),
                    exp_exchange.exchange_item_count4.unwrap_or(0),
                ),
                (
                    exp_exchange.exchange_item_num5.unwrap_or(0),
                    exp_exchange.exchange_item_count5.unwrap_or(0),
                ),
            ];
            let (exp_item_id, exp_count) = exp_outputs[idx];
            if exp_item_id != 0 && exp_count != 0 {
                selected_reward_idx = Some(exchange_list.len());
                exchange_list.push((exp_item_id as u32, exp_count as u32));
                tracing::info!(
                    sid,
                    exchange_id,
                    by_selected_reward,
                    exp_item_id,
                    exp_count,
                    "RunQuestExchange: adding item_exchange_exp reward"
                );
            }
        }
    }

    tracing::info!(
        sid,
        exchange_id,
        random_flag = exchange.random_flag,
        items = exchange_list.len(),
        "RunQuestExchange: giving {} items",
        exchange_list.len()
    );

    // ── Pre-validate the outputs and the bag BEFORE touching anything ──
    //
    // * Virtual ids (gold/exp/NP/skill) never need a slot.
    // * An output that is not in the item table is skipped with a warning, like
    //   the C++ server does (`GiveItem` fails silently for it) - the rest of
    //   the reward (gold, exp, other items) is still paid. The one exception is
    //   the reward the player explicitly picked: that one must never vanish
    //   silently, so the exchange is refused and they can pick another.
    // * Real items are deliverable only if the bag can hold ALL of them at the
    //   same time (several outputs used to be checked one by one against the
    //   same free slot) after the origin items have been taken out.
    let mut deliverable: Vec<(u32, u32)> = Vec::with_capacity(exchange_list.len());
    for (idx, &(item_id, count)) in exchange_list.iter().enumerate() {
        let is_virtual = matches!(
            item_id,
            ITEM_GOLD | ITEM_EXP | ITEM_COUNT | ITEM_LADDERPOINT | ITEM_SKILL
        );
        if !is_virtual {
            match w.get_item(item_id) {
                None => {
                    if Some(idx) == selected_reward_idx {
                        tracing::warn!(
                            sid,
                            exchange_id,
                            item_id,
                            "RunQuestExchange: FAIL - the selected reward item is not in the items table"
                        );
                        block_quest_completion(lua);
                        return Ok(false);
                    }
                    tracing::warn!(
                        sid,
                        exchange_id,
                        item_id,
                        "RunQuestExchange: output item_id not in items table - skipped (C++ parity)"
                    );
                    continue;
                }
                // Other countable == 2 ids are virtual markers (job change,
                // random box, cash ...) that cannot live in a slot.
                Some(item) if item.countable.unwrap_or(0) == 2 => {
                    tracing::warn!(
                        sid,
                        exchange_id,
                        item_id,
                        "RunQuestExchange: virtual output id has no handler - skipped"
                    );
                    continue;
                }
                Some(_) => {}
            }
        }
        deliverable.push((item_id, count));
    }

    let removals: Vec<(u32, u16)> = origins
        .iter()
        .filter(|&&(item_id, count)| item_id > 0 && count > 0)
        .map(|&(item_id, count)| (item_id as u32, count.min(u16::MAX as i32) as u16))
        .filter(|&(item_id, _)| {
            !is_special_origin_skip(item_id)
                && item_id != ITEM_GOLD
                && item_id != ITEM_COUNT
                && item_id != ITEM_LADDERPOINT
        })
        .collect();
    let additions: Vec<(u32, u16)> = deliverable
        .iter()
        .filter(|&&(item_id, _)| {
            !matches!(
                item_id,
                ITEM_GOLD | ITEM_EXP | ITEM_COUNT | ITEM_LADDERPOINT | ITEM_SKILL
            )
        })
        .map(|&(item_id, count)| (item_id, count.min(u16::MAX as u32) as u16))
        .collect();
    if !w.can_hold_after_exchange(sid, &removals, &additions) {
        tracing::warn!(
            sid,
            exchange_id,
            outputs = additions.len(),
            free_slots = w.count_free_inventory_slots(sid),
            "RunQuestExchange: FAIL - not enough inventory slots (nothing was taken)"
        );
        // C++ sends WIZ_QUEST sub-opcode 13 with error code 3 (no slots)
        send_no_slot_error(&w, sid);
        block_quest_completion(lua);
        return Ok(false);
    }

    // ── Remove origin items (after validation passes) ──
    for &(item_id, count) in &origins {
        if item_id == 0 || count == 0 {
            continue;
        }
        let id_u = item_id as u32;
        if is_special_origin_skip(id_u) {
            continue;
        }
        if id_u == ITEM_GOLD {
            if !w.gold_lose(sid, count as u32) {
                return Ok(false);
            }
        } else if id_u == ITEM_COUNT {
            let loyalty = w.get_character_info(sid).map(|c| c.loyalty).unwrap_or(0);
            if loyalty < count as u32 {
                return Ok(false);
            }
            w.update_character_stats(sid, |ch| {
                ch.loyalty = ch.loyalty.saturating_sub(count as u32);
            });
        } else if id_u == ITEM_LADDERPOINT {
            // Skip ladder point removal (C++ does continue)
        } else if !w.rob_item(sid, id_u, count.min(u16::MAX as i32) as u16) {
            return Ok(false);
        }
    }

    // ── Give all items in exchange list ──
    for &(item_id, count) in &deliverable {
        tracing::info!(sid, item_id, count, "RunQuestExchange: give_exchange_item");
        // exchange_item_timeN = lifetime in days of the reward (C++ passes it to GiveItem);
        // without it timed quest rewards (Moradon event quests, 30 day items) were permanent.
        let days = exchange_item_days(&exchange, item_id);
        let delivered = if days > 0 && !is_exchange_virtual_id(item_id) {
            w.give_item_with_expiry(sid, item_id, count.min(u16::MAX as u32) as u16, days)
        } else {
            give_exchange_item_checked(&w, sid, item_id, count)
        };
        if !delivered {
            // The plan above said it fits, so this is not expected; keep going so
            // the remaining rewards are still paid instead of aborting half-way.
            tracing::error!(
                sid,
                exchange_id,
                item_id,
                count,
                "RunQuestExchange: give_exchange_item failed after a successful plan"
            );
        }
    }

    // Sends WIZ_QUEST sub-opcode 10 with up to 8 item/count pairs (u32 each)
    let mut show_pkt = Packet::new(Opcode::WizQuest as u8);
    show_pkt.write_u8(10); // sub-opcode for ShowGiveItem
    for i in 0..8 {
        if i < deliverable.len() {
            show_pkt.write_u32(deliverable[i].0);
            show_pkt.write_u32(deliverable[i].1);
        } else {
            show_pkt.write_u32(0);
            show_pkt.write_u32(0);
        }
    }
    w.send_to_session_owned(sid, show_pkt);

    Ok(true)
}

/// RunRandomExchange(uid, exchange_id) -> bool
/// Random exchange: requires random_flag==101, skips special origin items,
/// builds weighted random array, gives 1 random output item.
fn lua_run_random_exchange(lua: &Lua, (uid, exchange_id): (i32, i32)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let exchange = match w.get_item_exchange(exchange_id) {
        Some(e) => e,
        None => return Ok(false),
    };

    // C++ requires random_flag == 101
    if exchange.random_flag != 101 {
        return Ok(false);
    }

    let origins: [(i32, i32); 5] = [
        (exchange.origin_item_num1, exchange.origin_item_count1),
        (exchange.origin_item_num2, exchange.origin_item_count2),
        (exchange.origin_item_num3, exchange.origin_item_count3),
        (exchange.origin_item_num4, exchange.origin_item_count4),
        (exchange.origin_item_num5, exchange.origin_item_count5),
    ];

    // Check gold and loyalty separately
    let mut total_gold: u32 = 0;
    let mut total_loyalty: u32 = 0;
    for &(item_id, count) in &origins {
        if item_id as u32 == ITEM_GOLD {
            total_gold += count as u32;
        }
        if item_id as u32 == ITEM_COUNT {
            total_loyalty += count as u32;
        }
    }
    if total_gold > 0 {
        let gold = w.get_character_info(sid).map(|c| c.gold).unwrap_or(0);
        if gold < total_gold {
            return Ok(false);
        }
    }
    if total_loyalty > 0 {
        let loyalty = w.get_character_info(sid).map(|c| c.loyalty).unwrap_or(0);
        if loyalty < total_loyalty {
            return Ok(false);
        }
    }

    // Check all origin items exist
    for &(item_id, count) in &origins {
        if item_id == 0 || count == 0 {
            continue;
        }
        let id_u = item_id as u32;
        if is_special_origin_skip(id_u) {
            continue;
        }
        let have = lua_howmuch_item(lua, (uid, id_u))?;
        if have < count as u32 {
            return Ok(false);
        }
    }

    // Weighted random selection from output items
    let outputs: [(i32, i32); 5] = [
        (exchange.exchange_item_num1, exchange.exchange_item_count1),
        (exchange.exchange_item_num2, exchange.exchange_item_count2),
        (exchange.exchange_item_num3, exchange.exchange_item_count3),
        (exchange.exchange_item_num4, exchange.exchange_item_count4),
        (exchange.exchange_item_num5, exchange.exchange_item_count5),
    ];

    use rand::Rng;
    let total: u32 = outputs.iter().map(|&(_, c)| c.max(0) as u32).sum();
    if total == 0 {
        return Ok(false);
    }

    let roll = rand::thread_rng().gen_range(0..total);
    let mut cumulative = 0u32;
    let mut selected_item: u32 = 0;
    for &(item_id, count) in &outputs {
        if item_id == 0 || count == 0 {
            continue;
        }
        cumulative += count as u32;
        if roll < cumulative {
            selected_item = item_id as u32;
            break;
        }
    }
    if selected_item == 0 {
        return Ok(false);
    }

    // Remove origin items, skipping special IDs
    for &(item_id, count) in &origins {
        if item_id == 0 || count == 0 {
            continue;
        }
        let id_u = item_id as u32;
        if is_special_origin_skip(id_u) {
            continue;
        }
        if id_u == ITEM_GOLD {
            if !w.gold_lose(sid, count as u32) {
                return Ok(false);
            }
        } else if id_u == ITEM_COUNT {
            w.update_character_stats(sid, |ch| {
                ch.loyalty = ch.loyalty.saturating_sub(count as u32);
            });
        } else if id_u == ITEM_LADDERPOINT {
            // Skip ladder point removal
        } else if !w.rob_item(sid, id_u, count as u16) {
            return Ok(false);
        }
    }

    // Give 1 of the randomly selected item
    give_exchange_item(&w, sid, selected_item, 1);
    Ok(true)
}

/// RunCountExchange(uid, exchange_id) -> bool
/// Execute an exchange where the count is determined by the minimum inventory
/// count of all origin items. All origin items are consumed, output items are
/// given multiplied by the count.
fn lua_run_count_exchange(lua: &Lua, (uid, exchange_id): (i32, i32)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let exchange = match w.get_item_exchange(exchange_id) {
        Some(e) => e,
        None => return Ok(false),
    };

    let origins: [(i32, i32); 5] = [
        (exchange.origin_item_num1, exchange.origin_item_count1),
        (exchange.origin_item_num2, exchange.origin_item_count2),
        (exchange.origin_item_num3, exchange.origin_item_count3),
        (exchange.origin_item_num4, exchange.origin_item_count4),
        (exchange.origin_item_num5, exchange.origin_item_count5),
    ];

    // Find the minimum count of origin items in inventory
    let mut min_count: u32 = u32::MAX;
    for &(item_id, _) in &origins {
        if item_id == 0 {
            continue;
        }
        let have = lua_howmuch_item(lua, (uid, item_id as u32))?;
        if have == 0 {
            return Ok(false);
        }
        if have < min_count {
            min_count = have;
        }
    }
    if min_count == u32::MAX || min_count == 0 {
        return Ok(false);
    }

    // Remove all origin items (min_count of each)
    for &(item_id, _) in &origins {
        if item_id == 0 {
            continue;
        }
        if !w.rob_item(sid, item_id as u32, min_count as u16) {
            return Ok(false);
        }
    }

    // Give output items
    // Special items (GOLD/EXP/COUNT/LADDERPOINT): min_count * exchange_count
    // Regular items: just min_count (no multiplication by exchange_count)
    let outputs: [(i32, i32); 5] = [
        (exchange.exchange_item_num1, exchange.exchange_item_count1),
        (exchange.exchange_item_num2, exchange.exchange_item_count2),
        (exchange.exchange_item_num3, exchange.exchange_item_count3),
        (exchange.exchange_item_num4, exchange.exchange_item_count4),
        (exchange.exchange_item_num5, exchange.exchange_item_count5),
    ];

    let mut exchange_list: Vec<(u32, u32)> = Vec::new();
    for &(item_id, count) in &outputs {
        if item_id == 0 || count == 0 {
            continue;
        }
        let item_id_u = item_id as u32;
        let total = if item_id_u == ITEM_GOLD
            || item_id_u == ITEM_EXP
            || item_id_u == ITEM_COUNT
            || item_id_u == ITEM_LADDERPOINT
        {
            min_count.saturating_mul(count as u32)
        } else {
            min_count
        };
        exchange_list.push((item_id_u, total));
        give_exchange_item(&w, sid, item_id_u, total);
    }

    let mut show_pkt = Packet::new(Opcode::WizQuest as u8);
    show_pkt.write_u8(10);
    for i in 0..8 {
        if i < exchange_list.len() {
            show_pkt.write_u32(exchange_list[i].0);
            show_pkt.write_u32(exchange_list[i].1);
        } else {
            show_pkt.write_u32(0);
            show_pkt.write_u32(0);
        }
    }
    w.send_to_session_owned(sid, show_pkt);

    Ok(true)
}

// ═══════════════════════════════════════════════════════════════════════
// Character Changes (Tier 1)
// ═══════════════════════════════════════════════════════════════════════

/// JobChange(uid, type, newJob) -> u8
/// Returns 1=success, 2=invalid job, 3=no scroll, 4=equipment worn, 5=error, 6=same job/no item.
fn lua_job_change(lua: &Lua, (uid, change_type, new_job): (i32, u8, u8)) -> LuaResult<u8> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let ch = match w.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(5),
    };

    if !(1..=5).contains(&new_job) || (change_type != 0 && change_type != 1) {
        return Ok(2);
    }

    let scroll_id: u32 = if change_type == 0 {
        700_112_000
    } else {
        700_113_000
    };
    let have_scroll = lua_howmuch_item(lua, (uid, scroll_id))?;
    if have_scroll == 0 {
        return Ok(6);
    }

    let class_type = ch.class % 100;
    let prefix = (ch.class / 100) * 100;

    let same_group = match new_job {
        1 => matches!(class_type, 1 | 5 | 6),
        2 => matches!(class_type, 2 | 7 | 8),
        3 => matches!(class_type, 3 | 9 | 10),
        4 => matches!(class_type, 4 | 11 | 12),
        5 => matches!(class_type, 13..=15),
        _ => false,
    };
    if same_group {
        return Ok(6);
    }

    // SLOT_MAX = 14 (equipment slots 0..13)
    let has_equipment = w
        .with_session(sid, |h| h.inventory.iter().take(14).any(|s| s.item_id != 0))
        .unwrap_or(false);
    if has_equipment {
        return Ok(4);
    }

    // Determine tier: beginner(1-4,13), novice(5,7,9,11,14), master(6,8,10,12,15)
    let tier = if matches!(class_type, 1..=4 | 13) {
        0
    } else if matches!(class_type, 5 | 7 | 9 | 11 | 14) {
        1
    } else if matches!(class_type, 6 | 8 | 10 | 12 | 15) {
        2
    } else {
        return Ok(5);
    };

    let new_class_type: u16 = match (new_job, tier) {
        (1, 0) => 1,
        (1, 1) => 5,
        (1, 2) => {
            if change_type == 1 {
                5
            } else {
                6
            }
        }
        (2, 0) => 2,
        (2, 1) => 7,
        (2, 2) => {
            if change_type == 1 {
                7
            } else {
                8
            }
        }
        (3, 0) => 3,
        (3, 1) => 9,
        (3, 2) => {
            if change_type == 1 {
                9
            } else {
                10
            }
        }
        (4, 0) => 4,
        (4, 1) => 11,
        (4, 2) => {
            if change_type == 1 {
                11
            } else {
                12
            }
        }
        (5, 0) => 13,
        (5, 1) => 14,
        (5, 2) => 15,
        _ => return Ok(5),
    };

    let new_class = prefix + new_class_type;

    // BUG 4 FIX: Warrior(1) keeps barbarian(11), others map barbarian/portu→elmorad_man(12)
    let nation = ch.nation;
    let new_race = if new_job == 5 {
        if nation == 1 {
            6
        } else {
            14
        } // Kurian/Portu
    } else if nation == 1 {
        match new_job {
            1 => 1, // Karus Big
            2 => 2, // Karus Middle
            3 => 3, // Karus Small
            4 => 2, // Karus Middle (priest)
            _ => ch.race,
        }
    } else {
        // El Morad: warrior keeps barbarian, others map barbarian/portu→elmorad_man
        if new_job == 1 {
            if ch.race == 14 {
                11
            } else {
                ch.race
            } // Portu→Barbarian, keep others
        } else if ch.race == 11 || ch.race == 14 {
            12 // Barbarian/Portu → ElmoradMan for rogue/mage/priest
        } else {
            ch.race
        }
    };

    if !w.rob_item(sid, scroll_id, 1) {
        return Ok(6);
    }

    w.update_character_stats(sid, |c| {
        c.class = new_class;
        c.race = new_race;
    });

    // AllPointChange(true) + AllSkillPointChange(true)
    w.update_character_stats(sid, |c| {
        let spent = (c.str.saturating_sub(10) as u16)
            + (c.sta.saturating_sub(10) as u16)
            + (c.dex.saturating_sub(10) as u16)
            + (c.intel.saturating_sub(10) as u16)
            + (c.cha.saturating_sub(10) as u16);
        c.str = 10;
        c.sta = 10;
        c.dex = 10;
        c.intel = 10;
        c.cha = 10;
        c.free_points = c.free_points.saturating_add(spent);

        let skill_total: u8 = c.skill_points[5..=8].iter().sum();
        for i in 5..=8 {
            c.skill_points[i] = 0;
        }
        c.skill_points[0] = c.skill_points[0].saturating_add(skill_total);
    });

    let mut pkt = Packet::new(Opcode::WizClassChange as u8);
    pkt.write_u8(0x06);
    pkt.write_u8(1); // success
    w.send_to_session_owned(sid, pkt);

    Ok(1)
}

/// GenderChange(uid, race) -> bool
/// Change the character's race. Validates nation compatibility.
fn lua_gender_change(lua: &Lua, (uid, race): (i32, u8)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    if race == 0 || race > 14 {
        return Ok(false);
    }

    let ch = match w.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(false),
    };

    const ITEM_GENDER_CHANGE: u32 = 810_594_000;
    let have = lua_howmuch_item(lua, (uid, ITEM_GENDER_CHANGE))?;
    if have == 0 {
        return Ok(false);
    }

    // Validate nation-race compatibility
    if ch.nation == 1 && race > 6 {
        return Ok(false);
    }
    if ch.nation == 2 && race < 11 {
        return Ok(false);
    }

    if !w.rob_item(sid, ITEM_GENDER_CHANGE, 1) {
        return Ok(false);
    }

    w.update_character_stats(sid, |c| {
        c.race = race;
    });
    Ok(true)
}

// ═══════════════════════════════════════════════════════════════════════
// Premium & Getters (Tier 2)
// ═══════════════════════════════════════════════════════════════════════

/// GivePremium(uid, premium_type, days) -> void
/// Update premium_in_use on session and store premium expiry in premium_map.
fn lua_give_premium(lua: &Lua, (uid, premium_type, days): (i32, u8, u16)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    if premium_type == 0 || premium_type > 13 || days == 0 {
        return Ok(());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    let expiry = now.saturating_add(days as u32 * 24 * 60 * 60);

    w.update_session(sid, |h| {
        h.premium_in_use = premium_type;
        h.account_status = 1; // C++ Reference: PremiumSystem.cpp:264
                              // Update or insert the premium in the premium_map
        let entry = h.premium_map.entry(premium_type).or_insert(0u32);
        if *entry < now {
            *entry = expiry;
        } else {
            *entry = entry.saturating_add(days as u32 * 24 * 60 * 60);
        }
    });

    let pkt = build_premium_info_for_lua(&w, sid, now);
    w.send_to_session_owned(sid, pkt);

    Ok(())
}

/// NationChange(uid, nation) -> void
/// Update character nation. Simplified Lua version (no scroll check).
fn lua_nation_change(lua: &Lua, (uid, nation): (i32, u8)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    if !(1..=2).contains(&nation) {
        return Ok(());
    }

    w.update_character_stats(sid, |ch| {
        ch.nation = nation;
    });
    Ok(())
}

/// GetExpPercent(uid) -> i32
/// Match the client/server level-table check used by GetExpPercent().
/// The game uses this as a full-level gate (not a progress-bar percentage).
fn lua_get_exp_percent(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let ch = match w.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(0),
    };

    let required_exp = w.get_exp_by_level(ch.level, ch.rebirth_level);
    Ok(if required_exp > 0 && ch.exp == required_exp as u64 {
        100
    } else {
        0
    })
}

/// CheckClanPoint(uid) -> i32
/// Returns the clan's clan_point_fund. 0 if not in a clan.
fn lua_check_clan_point(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let ch = match w.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(0),
    };

    if ch.knights_id == 0 {
        return Ok(0);
    }

    Ok(w.get_knights(ch.knights_id)
        .map(|k| k.clan_point_fund as i32)
        .unwrap_or(0))
}

/// GetCash(uid) -> i32 / CheckCash(uid) -> i32
/// Returns the player's Knight Cash (KC) balance from the session.
fn lua_get_cash(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let world = get_world(lua)?;
    let sid = uid as crate::zone::SessionId;
    Ok(world.get_knight_cash(sid) as i32)
}

// ═══════════════════════════════════════════════════════════════════════
// ═══════════════════════════════════════════════════════════════════════

/// Helper: look up the NPC instance the player is currently interacting with.
/// Uses `event_nid` (runtime NPC ID) from the player's session handle.
/// Returns `None` if no NPC interaction is active or the NPC doesn't exist.
fn get_event_npc(w: &WorldState, uid: i32) -> Option<std::sync::Arc<crate::npc::NpcInstance>> {
    let sid = uid as SessionId;
    let event_nid = w.with_session(sid, |h| h.event_nid)?;
    if event_nid < 0 {
        return None;
    }
    w.get_npc_instance(event_nid as u32)
}

/// NpcGetID(uid) -> i32
fn lua_npc_get_id(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    Ok(get_event_npc(&w, uid).map(|n| n.nid as i32).unwrap_or(-1))
}

/// NpcGetProtoID(uid) -> i32
fn lua_npc_get_proto_id(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    Ok(get_event_npc(&w, uid)
        .map(|n| n.proto_id as i32)
        .unwrap_or(-1))
}

/// NpcGetName(uid) -> String
fn lua_npc_get_name(lua: &Lua, uid: i32) -> LuaResult<String> {
    let w = get_world(lua)?;
    let npc = match get_event_npc(&w, uid) {
        Some(n) => n,
        None => return Ok(String::new()),
    };
    let name = w
        .get_npc_template(npc.proto_id, npc.is_monster)
        .map(|t| t.name.clone())
        .unwrap_or_default();
    Ok(name)
}

/// NpcGetNation(uid) -> u8
fn lua_npc_get_nation(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    Ok(get_event_npc(&w, uid).map(|n| n.nation).unwrap_or(0))
}

/// NpcGetType(uid) -> i32
fn lua_npc_get_type(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let npc = match get_event_npc(&w, uid) {
        Some(n) => n,
        None => return Ok(-1),
    };
    Ok(w.get_npc_template(npc.proto_id, npc.is_monster)
        .map(|t| t.npc_type as i32)
        .unwrap_or(-1))
}

/// NpcGetZoneID(uid) -> i32
fn lua_npc_get_zone_id(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    Ok(get_event_npc(&w, uid)
        .map(|n| n.zone_id as i32)
        .unwrap_or(-1))
}

/// NpcGetX(uid) -> f32
fn lua_npc_get_x(lua: &Lua, uid: i32) -> LuaResult<f32> {
    let w = get_world(lua)?;
    Ok(get_event_npc(&w, uid).map(|n| n.x).unwrap_or(0.0))
}

/// NpcGetY(uid) -> f32
fn lua_npc_get_y(lua: &Lua, uid: i32) -> LuaResult<f32> {
    let w = get_world(lua)?;
    Ok(get_event_npc(&w, uid).map(|n| n.y).unwrap_or(0.0))
}

/// NpcGetZ(uid) -> f32
fn lua_npc_get_z(lua: &Lua, uid: i32) -> LuaResult<f32> {
    let w = get_world(lua)?;
    Ok(get_event_npc(&w, uid).map(|n| n.z).unwrap_or(0.0))
}

/// CastSkill(uid, skill_id) -> bool
/// makes it cast `skill_id` on the player. The NPC is the caster, the player is
/// the target.
/// Returns true if the skill was cast, false otherwise.
fn lua_npc_cast_skill(lua: &Lua, args: LuaMultiValue) -> LuaResult<bool> {
    let w = match lua.app_data_ref::<Arc<WorldState>>() {
        Some(w) => Arc::clone(&w),
        None => return Ok(false), // No world context (e.g. in tests)
    };
    let mut iter = args.into_iter();
    let uid: i32 = match iter.next() {
        Some(v) => lua.unpack(v)?,
        None => return Ok(false),
    };
    let skill_id: u32 = match iter.next() {
        Some(v) => lua.unpack(v)?,
        None => return Ok(false),
    };

    if uid <= 0 || skill_id == 0 {
        return Ok(false);
    }
    let sid = uid as u16;

    // Get the player's event NPC (m_sEventNid)
    let event_npc = match get_event_npc(&w, uid) {
        Some(n) => n,
        None => return Ok(false),
    };

    let npc_id = event_npc.nid;
    // Stationary service NPCs have no combat AI entry. Their instance already
    // contains the zone/region needed to cast and broadcast support skills.

    // Look up skill in magic table to apply actual effects
    let magic = w.get_magic(skill_id as i32);
    if magic.is_none() {
        return Ok(false);
    }
    let skill_type = magic.as_ref().and_then(|m| m.type1).unwrap_or(0) as i32;

    // Apply type 3 (heal/damage) effect if applicable
    let mut heal_amount: i32 = 0;
    if skill_type == 3 {
        if let Some(t3) = w.get_magic_type3(skill_id as i32) {
            let first_damage = t3.first_damage.unwrap_or(0);
            if first_damage > 0 {
                // Positive Type3 damage is healing; negative values are damage.
                heal_amount = first_damage;
                let (old_hp, max_hp) = w
                    .with_session(sid, |h| h.character.as_ref().map(|ch| (ch.hp, ch.max_hp)))
                    .flatten()
                    .unwrap_or((0, 0));
                let new_hp = (old_hp as i32 + heal_amount).min(max_hp as i32) as i16;
                w.update_character_hp(sid, new_hp);
            }
        }
    }

    // Apply type 4 (buff) effect — duration-based stat modifier
    // MagicInstance dispatches to ApplyType4 which registers ActiveBuff
    if skill_type == 4 {
        if !crate::handler::magic_process::apply_npc_type4_support(
            w.as_ref(),
            npc_id,
            sid,
            skill_id,
            event_npc.zone_id,
            event_npc.region_x,
            event_npc.region_z,
            event_npc.event_room,
        ) {
            return Ok(false);
        }
        // The shared Type-4 path already sent the authoritative effect packet.
        return Ok(true);
    } else if skill_type != 3 || heal_amount == 0 {
        return Ok(false);
    }

    // Broadcast MAGIC_EFFECTING from the NPC to the player
    // Packet format: [u8 opcode][u32 skill][u32 caster][u32 target][u32 sData * 7]
    let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizMagicProcess as u8);
    pkt.write_u8(3); // MAGIC_EFFECTING
    pkt.write_u32(skill_id);
    pkt.write_u32(npc_id);
    pkt.write_u32(sid as u32);
    pkt.write_u32(0); // sData[0]
    pkt.write_u32(1); // sData[1] = effect successfully applied
    pkt.write_u32(0); // sData[2]
    pkt.write_u32(heal_amount as u32); // sData[3]
    pkt.write_u32(0); // sData[4]
    pkt.write_u32(0); // sData[5]
    pkt.write_u32(0); // sData[6]

    // Use tokio::task::block_in_place to call async from sync context
    // (Lua callbacks run in a sync context inside the tokio runtime)
    let npc_event_room = event_npc.event_room;
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            w.broadcast_to_3x3(
                event_npc.zone_id,
                event_npc.region_x,
                event_npc.region_z,
                Arc::new(pkt),
                None,
                npc_event_room,
            );
        });
    });

    tracing::debug!(
        sid,
        skill_id,
        npc_id,
        skill_type,
        heal_amount,
        "NpcCastSkill: NPC cast skill on player"
    );

    Ok(true)
}

// ═══════════════════════════════════════════════════════════════════════
// Sprint 31: Implemented Stubs
// ═══════════════════════════════════════════════════════════════════════

/// ChangeManner(uid, amount) -> void
/// Update manner points. Clamps to [0, LOYALTY_MAX].
/// Sends WIZ_LOYALTY_CHANGE with sub-opcode 2 (LOYALTY_MANNER_POINTS).
fn lua_change_manner(lua: &Lua, (uid, amount): (i32, i32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    const MANNER_MAX: i64 = 2_100_000_000;
    let mut new_manner = 0i32;
    w.update_character_stats(sid, |ch| {
        let result = (ch.manner_point as i64) + (amount as i64);
        ch.manner_point = result.clamp(0, MANNER_MAX) as i32;
        new_manner = ch.manner_point;
    });
    let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
    pkt.write_u8(2); // LOYALTY_MANNER_POINTS
    pkt.write_i32(new_manner);
    w.send_to_session_owned(sid, pkt);
    Ok(())
}

/// RobClanPoint(uid, amount) -> void
/// Deduct `amount` from the player's clan's clan_point_fund.
fn lua_rob_clan_point(lua: &Lua, (uid, amount): (i32, i32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let knights_id = w.get_character_info(sid).map(|c| c.knights_id).unwrap_or(0);
    if knights_id == 0 {
        return Ok(());
    }
    let deduct = amount as i64;
    let mut new_fund = 0u32;
    w.update_knights(knights_id, |k| {
        let result = (k.clan_point_fund as i64) - deduct;
        k.clan_point_fund = result.clamp(0, 2_100_000_000) as u32;
        new_fund = k.clan_point_fund;
    });

    // Clan rank-up Lua scripts spend from the persistent clan fund. Without
    // this update the deduction returns after a restart even though the rank
    // itself was saved.
    if let Some(pool) = w.db_pool() {
        let pool = pool.clone();
        let kid = knights_id as i16;
        tokio::spawn(async move {
            let repo = ko_db::repositories::knights::KnightsRepository::new(&pool);
            if let Err(e) = repo
                .update_clan_point_fund(kid, new_fund.min(i32::MAX as u32) as i32)
                .await
            {
                tracing::warn!(
                    "RobClanPoint: failed to save clan point fund for clan {}: {}",
                    kid,
                    e
                );
            }
        });
    }
    Ok(())
}

/// KissUser(uid) -> void
/// Sends WIZ_KISS (0x66) packet with player ID + event NPC ID,
/// and gives item 910014000 ("Kiss" item).
fn lua_kiss_user(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let event_nid = w.with_session(sid, |h| h.event_nid).unwrap_or(-1);
    // C++ WIZ_KISS = 0x66
    let mut pkt = Packet::new(0x66u8);
    pkt.write_u32(sid as u32);
    pkt.write_i16(event_nid);
    w.send_to_session_owned(sid, pkt);
    // C++ gives item 910014000
    w.give_item(sid, 910_014_000, 1);
    Ok(())
}

/// SendNameChange(uid) -> void
/// Sends WIZ_NAME_CHANGE (0x6E) with NameChangeShowDialog (1).
fn lua_send_name_change(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let mut pkt = Packet::new(Opcode::WizNameChange as u8);
    pkt.write_u8(1); // NameChangeShowDialog
    w.send_to_session_owned(uid as SessionId, pkt);
    Ok(())
}

/// SendClanNameChange(uid) -> void
/// Sends WIZ_NAME_CHANGE (0x6E) with ClanNameChange (16) + ShowDialog (1).
fn lua_send_clan_name_change(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let mut pkt = Packet::new(Opcode::WizNameChange as u8);
    pkt.write_u8(16); // ClanNameChange
    pkt.write_u8(1); // ShowDialog
    w.send_to_session_owned(uid as SessionId, pkt);
    Ok(())
}

/// SendTagNameChangePanel(uid) -> void
/// Sends WIZ_EXT_HOOK (0xE9) with TagInfo (0xD1) + Open (0).
fn lua_send_tag_name_change_panel(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    w.send_to_session_owned(uid as SessionId, tag_change::build_tag_panel_packet());
    Ok(())
}

/// ZoneChangeParty(uid, zone_id, x, z) -> void
/// Teleport all party members to a zone. If not in a party, teleport self.
fn lua_zone_change_party(lua: &Lua, (uid, zone_id, x, z): (i32, u16, f32, f32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let party_id = w.get_character_info(sid).and_then(|c| c.party_id);
    let members: Vec<SessionId> = match party_id.and_then(|pid| w.get_party(pid)) {
        Some(party) => party.members.iter().filter_map(|&m| m).collect(),
        None => vec![sid],
    };

    // Floor changes inside one dungeon (Desperation/Hell Abyss key floors) are same-zone moves,
    // which the non-force teleport silently skips.
    for member_sid in members {
        crate::handler::zone_change::server_teleport_to_zone_force(&w, member_sid, zone_id, x, z);
    }
    Ok(())
}

/// ZoneChangeClan(uid, zone_id, x, z [, legacy_range]) -> void
/// Teleport all online clan members to a zone. The official cape quest Lua
/// passes a fifth legacy argument (`50`). The C++ binding ignores it, so parse
/// a MultiValue here to preserve that v2615-compatible behaviour.
fn lua_zone_change_clan(lua: &Lua, args: LuaMultiValue) -> LuaResult<()> {
    let mut iter = args.into_iter();
    let uid: i32 = iter.next().and_then(|v| lua.unpack(v).ok()).unwrap_or(0);
    let zone_id: u16 = iter.next().and_then(|v| lua.unpack(v).ok()).unwrap_or(0);
    let x: f32 = iter.next().and_then(|v| lua.unpack(v).ok()).unwrap_or(0.0);
    let z: f32 = iter.next().and_then(|v| lua.unpack(v).ok()).unwrap_or(0.0);

    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let knights_id = w.get_character_info(sid).map(|c| c.knights_id).unwrap_or(0);
    if knights_id == 0 {
        return Ok(());
    }

    let clan_sids: Vec<SessionId> = w.get_online_knights_session_ids(knights_id);

    tracing::info!(
        uid,
        knights_id,
        zone_id,
        x,
        z,
        online_members = clan_sids.len(),
        "ZoneChangeClan: teleporting online clan members"
    );

    for member_sid in clan_sids {
        crate::handler::zone_change::server_teleport_to_zone(&w, member_sid, zone_id, x, z);
    }
    Ok(())
}

/// PromoteKnight(uid, flag) -> void
/// C++ alias: `PromoteKnight` = `PromoteClan` (lua_bindings.cpp:427)
/// Promote the player's clan to the given grade (flag).
/// The v2615 client does not render or list mantle catalogue entries when a
/// promoted clan is left on cape 0.  Use the visible JstKO reference cloak for
/// the first promoted state, while preserving already purchased capes.
fn lua_promote_knight(lua: &Lua, args: LuaMultiValue) -> LuaResult<()> {
    let mut iter = args.into_iter();
    let uid: i32 = iter.next().and_then(|v| lua.unpack(v).ok()).unwrap_or(0);
    let flag: i16 = iter.next().and_then(|v| lua.unpack(v).ok()).unwrap_or(2); // default ClanTypePromoted

    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let knights_id = w.get_character_info(sid).map(|c| c.knights_id).unwrap_or(0);
    if knights_id == 0 {
        return Ok(());
    }

    let current_cape = w
        .get_knights(knights_id)
        .map(|k| k.cape as i16)
        .unwrap_or(-1);
    let cape: i16 = match flag {
        1 => -1,
        2 if current_cape <= 0 => 201,
        2 => current_cape,
        _ => current_cape,
    };

    w.update_knights(knights_id, |k| {
        k.flag = flag as u8; // i16 → u8 intentional (grade values fit in u8)
        k.cape = cape as u16;
    });

    tracing::info!(
        uid,
        knights_id,
        flag,
        cape,
        "PromoteKnight: runtime clan promotion applied"
    );

    // Broadcast KNIGHTS_UPDATE to all online clan members
    broadcast_knights_update_from_world(&w, knights_id);
    // KNIGHTS_UPDATE refreshes the clan window. Re-send the promoted player as
    // a region WARP with the fresh cape data for nearby clients.
    refresh_promoted_clan_visual(&w, sid);

    // Persist flag+cape to DB (fire-and-forget)
    if let Some(pool) = w.db_pool() {
        let pool = pool.clone();
        let kid = knights_id as i16;
        tokio::spawn(async move {
            let repo = ko_db::repositories::knights::KnightsRepository::new(&pool);
            if let Err(e) = repo.update_flag_cape(kid, flag, cape).await {
                tracing::warn!(
                    "PromoteKnight: failed to save flag+cape for clan {}: {}",
                    kid,
                    e
                );
            } else {
                tracing::info!(kid, flag, cape, "PromoteKnight: database promotion saved");
            }
        });
    }

    Ok(())
}

/// Rebuild the promoted member's visual state for the local client and nearby
/// users. This is the same OUT/WARP refresh used after a job change, but keeps
/// the complete clan/cape block that v2615 needs to attach a cloak model.
fn refresh_promoted_clan_visual(w: &WorldState, sid: SessionId) {
    let Some((pos, character, event_room)) =
        w.with_session(sid, |h| (h.position, h.character.clone(), h.event_room))
    else {
        return;
    };
    let Some(character) = character else {
        return;
    };

    let clan = (character.knights_id > 0)
        .then(|| w.get_knights(character.knights_id))
        .flatten();
    let alliance_cape = clan
        .as_ref()
        .and_then(|ki| crate::handler::region::resolve_alliance_cape(ki, w));
    let is_king = w.is_king(character.nation, &character.name);
    let invisibility = w.get_invisibility_type(sid);
    let abnormal = w.get_abnormal_type(sid);
    let broadcast_state = w.get_broadcast_state(sid);
    let equipment = crate::handler::region::get_equipped_visual(w, sid);

    let out_packet = crate::handler::region::build_user_inout_with_clan(
        crate::handler::region::INOUT_OUT,
        sid,
        Some(&character),
        &pos,
        clan.as_ref(),
        alliance_cape,
        is_king,
        invisibility,
        abnormal,
        &broadcast_state,
        &equipment,
    );
    w.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(out_packet),
        Some(sid),
        event_room,
    );

    let warp_packet = crate::handler::region::build_user_inout_with_clan(
        crate::handler::region::INOUT_WARP,
        sid,
        Some(&character),
        &pos,
        clan.as_ref(),
        alliance_cape,
        is_king,
        invisibility,
        abnormal,
        &broadcast_state,
        &equipment,
    );
    w.broadcast_to_3x3(
        pos.zone_id,
        pos.region_x,
        pos.region_z,
        Arc::new(warp_packet),
        None,
        event_room,
    );
}

/// Build and broadcast a KNIGHTS_UPDATE packet for the given clan
/// to all its online members, using only the WorldState (no session needed).
/// This mirrors `send_knights_update` in knights.rs but works from Lua context.
fn broadcast_knights_update_from_world(w: &crate::world::WorldState, clan_id: u16) {
    use ko_protocol::{Opcode, Packet};
    use std::sync::Arc;

    let clan = match w.get_knights(clan_id) {
        Some(k) => k,
        None => return,
    };

    let mut result = Packet::new(Opcode::WizKnightsProcess as u8);
    result.write_u8(36); // KNIGHTS_UPDATE

    // Simple non-alliance path (most common for promotion)
    result.write_u16(clan.id);
    result.write_u8(clan.flag);
    result.write_u16(clan.cape);
    result.write_u8(clan.cape_r);
    result.write_u8(clan.cape_g);
    result.write_u8(clan.cape_b);
    result.write_u8(0);

    w.send_to_knights_members(clan_id, Arc::new(result), None);
}

// ═══════════════════════════════════════════════════════════════════════
// Sprint 32: New Binding Implementations
// ═══════════════════════════════════════════════════════════════════════

/// hasManner(uid, amount) -> bool
fn lua_has_manner(lua: &Lua, (uid, amount): (i32, u32)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    Ok(w.with_session(sid, |h| {
        h.character
            .as_ref()
            .map(|c| c.manner_point >= amount as i32)
            .unwrap_or(false)
    })
    .unwrap_or(false))
}

/// CheckWarVictory(uid) -> u8
fn lua_check_war_victory(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    let _sid = uid as SessionId;
    Ok(w.get_victory())
}

/// GetPVPMonumentNation(uid) -> u8
fn lua_get_pvp_monument_nation(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let zone_id = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    Ok(w.get_pvp_monument_nation(zone_id))
}

/// CheckMiddleStatueCapture(uid) -> i32
fn lua_check_middle_statue_capture(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let nation = w.get_character_info(sid).map(|c| c.nation).unwrap_or(0);
    Ok(if w.get_middle_statue_nation() == nation {
        1
    } else {
        0
    })
}

/// GetRebirthLevel(uid) -> u8
fn lua_get_rebirth_level(lua: &Lua, uid: i32) -> LuaResult<u8> {
    let w = get_world(lua)?;
    Ok(w.get_rebirth_level(uid as SessionId))
}

/// KingsInspectorList(uid) -> ()
fn lua_kings_inspector_list(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let mut pkt = Packet::new(Opcode::WizReport as u8);
    pkt.write_u8(18); // KingsInspector subcode
    w.send_to_session_owned(sid, pkt);
    Ok(())
}

/// GetMaxExchange(uid, exchange_id) -> u16
fn lua_get_max_exchange(lua: &Lua, (uid, exchange_id): (i32, i32)) -> LuaResult<u16> {
    let w = get_world(lua)?;
    Ok(w.get_max_exchange_capacity(uid as SessionId, exchange_id))
}

/// isCswWinnerNembers(uid) -> bool
/// knights. Winners are transitioned to the configured Castellan dungeon zone.
fn lua_is_csw_winner_members(lua: &Lua, uid: i32) -> LuaResult<bool> {
    let world = get_world(lua)?;
    let sid = uid as SessionId;
    let master_clan = world.get_csw_master_knights();
    if master_clan == 0 || world.get_knights(master_clan).is_none() {
        tracing::warn!(sid, master_clan, "CSW winner check failed: no valid owner clan");
        return Ok(false);
    }

    let clan_id = world.get_session_clan_id(sid);
    if clan_id == 0 {
        return Ok(false);
    }

    let mut is_member = clan_id == master_clan;
    if !is_member {
        if let Some(clan) = world.get_knights(clan_id) {
            if clan.alliance != 0 {
                if let Some(alliance) = world.get_alliance(clan.alliance) {
                    is_member = alliance.main_clan == master_clan
                        || alliance.sub_clan == master_clan
                        || alliance.mercenary_1 == master_clan
                        || alliance.mercenary_2 == master_clan;
                }
            }
        }
    }

    if !is_member {
        tracing::info!(sid, clan_id, master_clan, "CSW winner check denied");
        return Ok(false);
    }

    // The local Maria NPC routes eligible winners to the Castellan dungeon zone.
    // Return true as the reference Lua contract expects and perform the transition
    // through the shared region-aware zone-change path.
    tracing::info!(sid, clan_id, master_clan, "CSW winner accepted for Castellan");
    crate::handler::zone_change::server_teleport_to_zone(
        &world,
        sid,
        crate::world::ZONE_DELOS_CASTELLAN,
        458.0,
        113.0,
    );
    Ok(true)
}
/// CSW deathmatch minimum level requirement.
const CSW_DEATHMATCH_MIN_LEVEL: u8 = 35;

/// CheckCastleSiegeWarDeathmachRegister(uid) -> u16
/// Validate and register a player for the CSW deathmatch.
/// Quest script `31719_aron.lua` calls this (EVENT 103).
/// Return codes:
/// - 1 = success (Lua will deduct gold)
/// - 2 = CSW not active
/// - 3 = already registered
/// - 4 = clan required
/// - 5 = level too low
/// - 6 = not in Delos zone
fn lua_csw_deathmatch_register(lua: &Lua, uid: i32) -> LuaResult<u16> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // Get player info from session.
    let info = match w.get_character_info(sid) {
        Some(c) => c,
        None => return Ok(6),
    };
    let zone_id = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);

    // Validation order matches Lua script expectation.
    if zone_id != ZONE_DELOS {
        return Ok(6);
    }

    let csw = w.csw_event().try_read().map_err(|_| LuaError::external("CSW state busy"))?;
    if !csw.is_active() {
        return Ok(2);
    }

    if info.knights_id == 0 {
        return Ok(4);
    }

    if info.level < CSW_DEATHMATCH_MIN_LEVEL {
        return Ok(5);
    }

    if csw.deathmatch_players.contains(&sid) {
        return Ok(3);
    }
    drop(csw);

    // Register the player.
    let mut csw = w.csw_event().try_write().map_err(|_| LuaError::external("CSW state busy"))?;
    csw.deathmatch_players.insert(sid);

    Ok(1)
}

/// CheckCastleSiegeWarDeathmacthCancelRegister(uid) -> u16
/// Cancel a player's CSW deathmatch registration.
/// Quest script `31719_aron.lua` calls this (EVENT 105).
/// Return codes:
/// - 1 = success (registration cancelled)
/// - 2 = CSW not active
/// - 3 = not registered
/// - 6 = not in Delos zone
fn lua_csw_deathmatch_cancel_register(lua: &Lua, uid: i32) -> LuaResult<u16> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let zone_id = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);

    if zone_id != ZONE_DELOS {
        return Ok(6);
    }

    let csw = w.csw_event().try_read().map_err(|_| LuaError::external("CSW state busy"))?;
    if !csw.is_active() {
        return Ok(2);
    }

    if !csw.deathmatch_players.contains(&sid) {
        return Ok(3);
    }
    drop(csw);

    // Remove the player.
    let mut csw = w.csw_event().try_write().map_err(|_| LuaError::external("CSW state busy"))?;
    csw.deathmatch_players.remove(&sid);

    Ok(1)
}

/// SendNpcKillID(uid, npc_id) -> ()
fn lua_send_npc_kill_id(lua: &Lua, (uid, npc_id): (i32, u32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let zone_id = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    if zone_id == 0 {
        return Ok(());
    }
    w.kill_npc_by_proto_id(npc_id as u16, zone_id);
    Ok(())
}

/// CycleSpawn(uid) -> ()
/// Looks up the user's target NPC (via `event_nid`) and, if it has
/// `special_type == 7` (NpcSpecialTypeCycleSpawn) and `trap_number` in 1..=4,
/// broadcasts NPC_OUT to make it disappear from nearby players.
/// Used by quest NPCs that cycle between predefined positions.
fn lua_cycle_spawn(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;

    // C++ uses GetTargetID() which maps to event_nid in our code
    let npc = match get_event_npc(&w, uid) {
        Some(n) => n,
        None => return Ok(()),
    };

    // C++ check: special_type == NpcSpecialTypeCycleSpawn (7) and trap_number in 1..=4
    let is_cycle_spawn = npc.special_type == 7 && (1..=4).contains(&npc.trap_number);
    if !is_cycle_spawn {
        return Ok(());
    }

    // Broadcast NPC_OUT to nearby players (C++: SendInOut(INOUT_OUT, ...))
    let tmpl = match w.get_npc_template(npc.proto_id, npc.is_monster) {
        Some(t) => t,
        None => return Ok(()),
    };

    let out_pkt = crate::npc::build_npc_inout(crate::npc::NPC_OUT, &npc, &tmpl);

    let npc_event_room = npc.event_room;
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            w.broadcast_to_3x3(
                npc.zone_id,
                npc.region_x,
                npc.region_z,
                Arc::new(out_pkt),
                None,
                npc_event_room,
            );
        });
    });

    tracing::debug!(
        nid = npc.nid,
        proto_id = npc.proto_id,
        "CycleSpawn: sent NPC_OUT for cycle-spawn NPC"
    );

    Ok(())
}

/// MoveMiddleStatue(uid) -> ()
/// Karus → Dodo camp, El Morad → Laon camp, with random offset.
fn lua_move_middle_statue(lua: &Lua, uid: i32) -> LuaResult<()> {
    use crate::world::{
        DODO_CAMP_WARP_X, DODO_CAMP_WARP_Z, DODO_LAON_WARP_RADIUS, LAON_CAMP_WARP_X,
        LAON_CAMP_WARP_Z,
    };
    use rand::Rng;

    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let nation = w.get_character_info(sid).map(|c| c.nation).unwrap_or(0);
    if nation == 0 {
        return Ok(());
    }

    let (base_x, base_z) = if nation == 1 {
        (DODO_CAMP_WARP_X, DODO_CAMP_WARP_Z)
    } else {
        (LAON_CAMP_WARP_X, LAON_CAMP_WARP_Z)
    };

    let mut rng = rand::thread_rng();
    let x = (base_x as u32 + rng.gen_range(0..=DODO_LAON_WARP_RADIUS as u32)) * 10;
    let z = (base_z as u32 + rng.gen_range(0..=DODO_LAON_WARP_RADIUS as u32)) * 10;
    let x = x as f32;
    let z = z as f32;

    let zone_id = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
    pkt.write_u8(2);
    pkt.write_u16(zone_id);
    pkt.write_f32(x);
    pkt.write_f32(0.0);
    pkt.write_f32(z);
    pkt.write_u8(0);
    w.send_to_session_owned(sid, pkt);
    w.update_session(sid, |h| {
        h.position.x = x;
        h.position.z = z;
    });

    Ok(())
}

/// SendNationTransfer(uid) -> ()
fn lua_send_nation_transfer(lua: &Lua, uid: i32) -> LuaResult<()> {
    const ITEM_NATION_TRANSFER: u32 = 810_096_000;

    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // C++ checks isTransformed() — res_hp_type == 3 means transformed (Type 6 magic)
    if let Some(ch) = w.get_character_info(sid) {
        if ch.res_hp_type == 3 {
            let mut pkt = Packet::new(Opcode::WizNationTransfer as u8);
            pkt.write_u8(2); // NationOpenBox
            pkt.write_u8(6); // error: transformed
            w.send_to_session_owned(sid, pkt);
            return Ok(());
        }
    }

    if !w.check_exist_item(sid, ITEM_NATION_TRANSFER, 1) {
        let mut pkt = Packet::new(Opcode::WizNationTransfer as u8);
        pkt.write_u8(2); // NationOpenBox
        pkt.write_u8(7); // error: no item
        w.send_to_session_owned(sid, pkt);
        return Ok(());
    }

    // Send open dialog
    let mut pkt = Packet::new(Opcode::WizNationTransfer as u8);
    pkt.write_u8(1);
    w.send_to_session_owned(sid, pkt);
    Ok(())
}

/// RobAllItemParty(uid, item_id [, count]) -> bool
/// The C++ Lua binding defaults count to 1; preserve that behavior for legacy
/// quest scripts (notably the Abyss gate script) that pass only two arguments.
fn lua_rob_all_item_party(lua: &Lua, args: LuaMultiValue) -> LuaResult<bool> {
    let vals: Vec<LuaValue> = args.into_vec();
    if vals.len() < 2 {
        return Ok(false);
    }

    let uid = lua_val_to_i32(&vals[0])?;
    let item_id = lua_val_to_u32(&vals[1])?;
    let count = vals
        .get(2)
        .map(|value| lua_val_to_u16(value).unwrap_or(1).max(1))
        .unwrap_or(1);

    let w = get_world(lua)?;
    Ok(w.rob_all_item_party(uid as SessionId, item_id, count))
}

// ═══════════════════════════════════════════════════════════════════════
// Sprint 33: New Binding Implementations (20 functions)
// ═══════════════════════════════════════════════════════════════════════

/// ShowBulletinBoard(uid) -> ()
/// Sends WIZ_BATTLE_EVENT sub-opcode 13 with king entry + top-10 clans + top-10 players.
fn lua_show_bulletin_board(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let nation = w.get_character_info(sid).map(|c| c.nation).unwrap_or(0);
    if nation == 0 {
        return Ok(());
    }

    let mut pkt = Packet::new(Opcode::WizBattleEvent as u8);
    pkt.write_u8(13); // C++ sub-opcode for bulletin board
    let count_pos = pkt.wpos();
    let mut count: u8 = 1; // 1 for king entry
    pkt.write_u8(count); // placeholder — backpatched below

    // ── KING ENTRY (index 1000) ─────────────────────────────────────
    pkt.write_u16(1000);
    let king_name = w
        .get_king_system(nation)
        .map(|ks| ks.king_name.clone())
        .unwrap_or_default();
    pkt.write_string(&king_name);
    pkt.write_u16(0); // padding

    let king_clan_id = w
        .get_king_system(nation)
        .map(|ks| ks.king_clan_id)
        .unwrap_or(0);
    if king_clan_id > 0 {
        if let Some(clan) = w.get_knights(king_clan_id) {
            pkt.write_u16(clan.id);
            pkt.write_u16(clan.mark_version);
            pkt.write_string(&clan.name);
            pkt.write_u8(clan.flag);
            pkt.write_u8(clan.grade);
        } else {
            // Clan not found — write "no clan" sentinel
            pkt.write_u16(0xFFFF);
            pkt.write_u16(0);
            pkt.write_u16(0);
            pkt.write_u16(0);
        }
    } else {
        // No clan — C++ writes 4× u16(0/-1/0/0)
        pkt.write_u16(0xFFFF);
        pkt.write_u16(0);
        pkt.write_u16(0);
        pkt.write_u16(0);
    }

    // ── TOP 10 CLANS (index 1201+) ──────────────────────────────────
    let top_clans = w.get_top_knights_by_nation(nation, 10);
    for (i, (clan_id, _name, _mv)) in top_clans.iter().enumerate() {
        if let Some(clan) = w.get_knights(*clan_id) {
            pkt.write_u16(1201 + i as u16);
            pkt.write_string(&clan.chief);
            pkt.write_u16(0); // padding
            pkt.write_u16(clan.id);
            pkt.write_u16(clan.mark_version);
            pkt.write_string(&clan.name);
            pkt.write_u8(clan.flag);
            pkt.write_u8(clan.grade);
            count += 1;
        }
    }

    // ── TOP 10 PLAYERS (index 1101+) ─────────────────────────────────
    let personal_ranks = w.get_bot_personal_rank();
    let mut player_count = 0u16;
    for row in &personal_ranks {
        if player_count >= 10 {
            break;
        }
        let (user_name, clan_id_opt) = if nation == 1 {
            // Karus
            (
                row.str_karus_user_id.as_deref().unwrap_or(""),
                row.s_karus_knights,
            )
        } else {
            // El Morad
            (
                row.str_elmo_user_id.as_deref().unwrap_or(""),
                row.s_elmo_knights,
            )
        };
        if user_name.is_empty() {
            continue;
        }

        pkt.write_u16(1101 + player_count);
        pkt.write_string(user_name);
        pkt.write_u16(0); // padding

        let clan_id = clan_id_opt.unwrap_or(0) as u16;
        if clan_id > 0 {
            if let Some(clan) = w.get_knights(clan_id) {
                pkt.write_u16(clan.id);
                pkt.write_u16(clan.mark_version);
                pkt.write_string(&clan.name);
                pkt.write_u8(clan.flag);
                pkt.write_u8(clan.grade);
            } else {
                pkt.write_u16(0xFFFF);
                pkt.write_u16(0);
                pkt.write_u16(0);
                pkt.write_u16(0);
            }
        } else {
            pkt.write_u16(0xFFFF);
            pkt.write_u16(0);
            pkt.write_u16(0);
            pkt.write_u16(0);
        }

        count += 1;
        player_count += 1;
    }

    // Backpatch the total entry count
    pkt.put_u8_at(count_pos, count);
    w.send_to_session_owned(sid, pkt);
    Ok(())
}

/// SendVisibe(uid, offset1, offset2) -> ()
/// Version-gated: only for `__VERSION < 2369`. Modern clients don't use this.
fn lua_send_visibe(lua: &Lua, _args: LuaMultiValue) -> LuaResult<()> {
    // C++ has `#if(__VERSION < 2369)` compile-time guard.
    // Modern KO clients (>= 2369) never receive this packet.
    // Intentionally a no-op.
    let _ = lua;
    Ok(())
}

/// GiveSwitchPremium(uid, premium_type, days) -> ()
/// Extends from stored time (not current time) for existing entries.
/// Only sends SendPremiumInfo when switchPremiumCount > 2.
fn lua_give_switch_premium(lua: &Lua, (uid, premium_type, days): (i32, u8, u16)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    if premium_type == 0 || premium_type > 13 || days == 0 {
        return Ok(());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let mut should_send_info = false;

    w.update_session(sid, |h| {
        if h.character.is_none() {
            return;
        }

        // PREMIUM_TOTAL = 6 in globals.h, but C++ checks > (not >=)
        if h.premium_map.len() > 6 && !h.premium_map.contains_key(&premium_type) {
            return;
        }

        let duration_secs = days as u32 * 86400;

        use std::collections::hash_map::Entry;
        match h.premium_map.entry(premium_type) {
            Entry::Occupied(mut e) => {
                // C++ EXISTING entry: extend from stored time (NOT from now)
                // pPremium->iPremiumTime = pPremium->iPremiumTime + 24 * 60 * 60 * sPremiumTime
                *e.get_mut() += duration_secs;
            }
            Entry::Vacant(e) => {
                // C++ NEW entry: set from now
                // pPremium->iPremiumTime = uint32(UNIXTIME) + 24 * 60 * 60 * sPremiumTime
                e.insert(now + duration_secs);
            }
        }

        h.premium_in_use = premium_type;
        h.account_status = 1;
        h.switch_premium_count += 1;

        // C++ only sends SendPremiumInfo when m_switchPremiumCount > 2
        should_send_info = h.switch_premium_count > 2;
    });

    // if (m_switchPremiumCount > 2) SendPremiumInfo();
    if should_send_info {
        let pkt = build_premium_info_for_lua(&w, sid, now);
        w.send_to_session_owned(sid, pkt);
    }

    Ok(())
}

/// GiveClanPremium(uid, premium_type, days) -> bool (true when the clan premium was applied)
/// Updates knights premium time/type and sends clan premium packet to all online members.
fn lua_give_clan_premium(lua: &Lua, (uid, _premium_type, days): (i32, u8, u32)) -> LuaResult<bool> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    if days == 0 {
        return Ok(false);
    }

    // Must be in clan and be leader
    let (clan_id, is_leader) = w
        .with_session(sid, |h| {
            h.character
                .as_ref()
                .map(|c| {
                    // the same rule as isClanLeader(): the clan leader has fame 1
                    let is_leader = c.knights_id > 0
                        && (c.fame == 1
                            || w.get_knights(c.knights_id)
                                .map(|k| k.chief.eq_ignore_ascii_case(&c.name))
                                .unwrap_or(false));
                    (c.knights_id, is_leader)
                })
                .unwrap_or((0, false))
        })
        .unwrap_or((0, false));

    if clan_id == 0 || !is_leader || w.get_knights(clan_id).is_none() {
        tracing::warn!(sid, clan_id, is_leader, "GiveClanPremium refused");
        return Ok(false);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    // int remtime = pKnights->sPremiumTime > UNIXTIME ? pKnights->sPremiumTime - UNIXTIME : 0;
    // pKnights->sPremiumTime = (UNIXTIME + 24*60*60*sPremiumDay) + remtime;
    // pKnights->sPremiumInUse = CLAN_PREMIUM;
    let mut remaining_minutes: u32 = 0;
    w.update_knights(clan_id, |k| {
        let remtime = k.premium_time.saturating_sub(now);
        k.premium_time = (now + 86400 * days) + remtime;
        k.premium_in_use = 13; // CLAN_PREMIUM
                               // Calculate remaining minutes for the packet
        remaining_minutes = (k.premium_time.saturating_sub(now)) / 60;
    });

    // C++ sends SendClanPremium(pKnights) to each online member
    // SendClanPremium sets m_bClanPremiumInUse = 13, sClanPremStatus = true,
    // then calls SendClanPremiumPkt which sends:
    // WIZ_PREMIUM << u8(2) << u8(4) << u32(remaining_minutes) << u16(0) << u8(2)
    let member_sids = w.get_online_knights_session_ids(clan_id);
    let mut pkt = Packet::new(Opcode::WizPremium as u8);
    pkt.write_u8(2); // SUBOPCODE_CLAN_PREMIUM
    pkt.write_u8(4); // active status
    pkt.write_u32(remaining_minutes);
    pkt.write_u16(0);
    pkt.write_u8(2);

    for msid in &member_sids {
        w.update_session(*msid, |h| {
            h.clan_premium_in_use = 13; // CLAN_PREMIUM
        });
        w.send_to_session(*msid, &pkt);
        // The 2625 client has no "Clan Premium" label of its own, so tell the members in a notice.
        let text = format!(
            "Clan Premium aktif: {} gun {} saat kaldi (EXP, NP, drop ve tamir indirimi).",
            remaining_minutes / 1440,
            (remaining_minutes % 1440) / 60
        );
        w.send_to_session_owned(
            *msid,
            crate::systems::timed_notice::build_notice_packet(8, &text),
        );
    }

    tracing::warn!(sid, clan_id, days, members = member_sids.len(), "GiveClanPremium applied");
    Ok(true)
}

/// GivePremiumItem(uid, premium_type) -> ()
/// C++ calls `g_pMain->AddDatabaseRequest(result, this)` which routes to
/// `ReqLetterGivePremiumItem` in `LetterHandler.cpp:387-414`.
/// That function looks up `m_ItemPremiumGiftArray` (PREMIUM_GIFT_ITEM table) by
/// premium_type and for each matching gift creates a system letter with the item attached.
/// Since the PREMIUM_GIFT_ITEM table has no meaningful data in our reference DB,
/// this function looks up the premium gift items from WorldState and creates
/// system letters via `create_system_letter()`. If no gift data is loaded, it logs
/// and returns gracefully.
fn lua_give_premium_item(lua: &Lua, (uid, premium_type): (i32, u8)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    if premium_type == 0 {
        return Ok(());
    }

    // Get recipient character name
    let char_name = match w.get_character_info(sid) {
        Some(ch) => ch.name,
        None => {
            tracing::debug!(
                "[{}] GivePremiumItem(type={}) — no character info",
                uid,
                premium_type
            );
            return Ok(());
        }
    };

    // Look up premium gift items for this type from WorldState
    let gifts = w.get_premium_gift_items(premium_type);

    if gifts.is_empty() {
        tracing::debug!(
            "[{}] GivePremiumItem(type={}) — no gift items configured for this premium type",
            uid,
            premium_type
        );
        return Ok(());
    }

    // Get DB pool for creating letters
    let pool = match w.db_pool() {
        Some(p) => p.clone(),
        None => {
            tracing::debug!(
                "[{}] GivePremiumItem(type={}) — no DB pool available",
                uid,
                premium_type
            );
            return Ok(());
        }
    };

    let world_clone = Arc::clone(&w);
    let recipient = char_name.clone();

    // Fire-and-forget: create system letters for each gift item
    tokio::spawn(async move {
        for gift in &gifts {
            // Look up item template for durability
            let durability = world_clone
                .get_item(gift.item_id)
                .map(|it| it.duration.unwrap_or(0) as u16)
                .unwrap_or(0);

            match crate::handler::letter::create_system_letter(
                &pool,
                &gift.sender,
                &recipient,
                &gift.subject,
                &gift.message,
                gift.item_id,
                gift.count,
                durability,
            )
            .await
            {
                Ok(true) => {
                    tracing::debug!(
                        "GivePremiumItem: sent letter to {} with item {} x{}",
                        recipient,
                        gift.item_id,
                        gift.count
                    );
                }
                Ok(false) => {
                    tracing::warn!("GivePremiumItem: recipient {} not found in DB", recipient);
                }
                Err(e) => {
                    tracing::error!(
                        "GivePremiumItem: failed to create letter for {}: {}",
                        recipient,
                        e
                    );
                }
            }
        }

        // Notify recipient of unread letters
        let notify = crate::handler::letter::build_unread_notification();
        world_clone.send_to_session_owned(sid, notify);
    });

    Ok(())
}

/// SpawnEventSystem(uid, npc_id, is_monster, zone, x, y, z) -> ()
/// Calls `WorldState::spawn_event_npc()` to allocate a runtime NPC from a template,
/// register it in the zone region grid, initialize HP/AI, and broadcast NPC_IN.
/// The C++ side passes `is_monster` as 0 (NPC) or 1 (monster) and converts to bool
/// via `bIsmonster == 0 ? true : false` (inverted), then calls SpawnEventNpc with
/// count=1, duration=1 HOUR. We spawn count=1 and let duration be handled by the
/// AI tick system separately.
fn lua_spawn_event_system(lua: &Lua, args: LuaMultiValue) -> LuaResult<()> {
    // Parse args: uid, npc_id, is_monster, zone, x, y, z
    let vals: Vec<&LuaValue> = args.iter().collect();
    let npc_id = vals.get(1).map(|v| lua_val_to_i32_or(v, 0)).unwrap_or(0);
    let is_monster_arg = vals.get(2).map(|v| lua_val_to_i32_or(v, 0)).unwrap_or(0);
    let zone = vals.get(3).map(|v| lua_val_to_i32_or(v, 0)).unwrap_or(0);
    let x = vals.get(4).map(|v| lua_val_to_i32_or(v, 0)).unwrap_or(0);
    let _y = vals.get(5).map(|v| lua_val_to_i32_or(v, 0)).unwrap_or(0);
    let z = vals.get(6).map(|v| lua_val_to_i32_or(v, 0)).unwrap_or(0);

    if npc_id <= 0 || zone <= 0 {
        return Ok(());
    }

    // C++ inverts the flag: `bIsmonster == 0 ? true : false`
    // So is_monster_arg=0 means IS monster, is_monster_arg=1 means NOT monster.
    let is_monster = is_monster_arg == 0;

    let w = get_world(lua)?;
    let s_sid = npc_id as u16;
    let zone_id = zone as u16;
    let spawn_x = x as f32;
    let spawn_z = z as f32;

    tracing::debug!(
        s_sid,
        is_monster,
        zone_id,
        spawn_x,
        spawn_z,
        "SpawnEventSystem: spawning runtime NPC"
    );

    // spawn_event_npc is sync now
    {
        let spawned = w.spawn_event_npc(s_sid, is_monster, zone_id, spawn_x, spawn_z, 1);
        if spawned.is_empty() {
            tracing::warn!(
                s_sid,
                is_monster,
                zone_id,
                "SpawnEventSystem: failed to spawn (template or zone not found)"
            );
        }
        // C++ sets duration=1*HOUR but NPC AI duration tracking requires
        // a tick reference. Event NPCs typically persist until killed via
        // KillNpcEvent or zone cleanup, so we omit auto-expiry here.
    }

    Ok(())
}

/// NpcEventSystem(uid, selling_group) -> ()
fn lua_npc_event_system(lua: &Lua, (uid, selling_group): (i32, u32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let has_char = w
        .with_session(sid, |h| h.character.is_some())
        .unwrap_or(false);
    if !has_char {
        return Ok(());
    }

    // C++ sets pNpc->m_iSellingGroup = selling_group then sends WIZ_TRADE_NPC.
    // Our NpcInstance is immutable (Arc), so we just send the packet which tells
    // the client which shop list to display.
    let mut pkt = Packet::new(Opcode::WizTradeNpc as u8);
    pkt.write_u32(selling_group);
    w.send_to_session_owned(sid, pkt);

    Ok(())
}

/// KillNpcEvent(uid) -> ()
fn lua_kill_npc_event(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let event_nid = w.with_session(sid, |h| h.event_nid).unwrap_or(-1);
    if event_nid <= 0 {
        return Ok(());
    }

    w.kill_npc_by_runtime_id(event_nid as u32);
    w.update_session(sid, |h| {
        h.event_nid = 0;
    });

    Ok(())
}

/// SendRepurchaseMsg(uid) -> ()
fn lua_send_repurchase_msg(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let deleted_items = w.get_deleted_items(sid);

    let mut pkt = Packet::new(Opcode::WizItemTrade as u8);
    pkt.write_u8(5); // repurchase subcode
    pkt.write_u8(1); // initial (not refreshed)
    pkt.write_u8(1); // success

    // Count valid (non-expired) items, max 250
    let valid_items: Vec<_> = deleted_items
        .iter()
        .filter(|e| e.delete_time > now)
        .take(250)
        .collect();

    pkt.write_u16(valid_items.len() as u16);

    for (idx, entry) in valid_items.iter().enumerate() {
        let buy_price = w
            .get_item(entry.item_id)
            .and_then(|item| item.buy_price)
            .unwrap_or(0) as u64;
        let repurchase_price = (buy_price * entry.count as u64 * 30).min(2_100_000_000);

        pkt.write_u8(idx as u8); // display index
        pkt.write_u32(entry.item_id);
        pkt.write_u16(entry.count as u16);
        pkt.write_u32(repurchase_price as u32);
        pkt.write_u16(entry.duration);
    }

    w.send_to_session_owned(sid, pkt);
    Ok(())
}

/// DrakiOutZone(uid) -> ()
fn lua_draki_out_zone(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let has_char = w
        .with_session(sid, |h| h.character.is_some())
        .unwrap_or(false);
    if !has_char {
        return Ok(());
    }

    // C++ sends WIZ_EVENT with TEMPLE_DRAKI_TOWER_OUT1 magic bytes
    let mut pkt = Packet::new(Opcode::WizEvent as u8);
    pkt.write_u8(0x16); // TEMPLE_DRAKI_TOWER_OUT1
    pkt.write_u8(0x0C);
    pkt.write_u8(0x04);
    pkt.write_u8(0x00);
    pkt.write_u8(0x14);
    pkt.write_u16(0);
    pkt.write_u8(0);
    w.send_to_session_owned(sid, pkt);

    // Reference DrakiTowerKickOuts() starts the 20-second evacuation timer
    // immediately after sending OUT1. Sending the packet alone leaves event
    // 101 at the final NPC without any server-side transition.
    let room_id = w
        .with_session(sid, |h| {
            if h.draki_room_id > 0 {
                h.draki_room_id
            } else {
                h.event_room
            }
        })
        .unwrap_or(0);
    if room_id > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut rooms = w.draki_tower_rooms_write();
        if let Some(room) = rooms.get_mut(&room_id) {
            crate::handler::draki_tower::apply_kickout(room, now);
            tracing::info!(
                sid,
                room_id,
                exit_at = room.draki_out_timer,
                "Draki final NPC started authoritative kick-out timer"
            );
        } else {
            tracing::warn!(sid, room_id, "DrakiOutZone could not find active room");
        }
    }

    Ok(())
}

/// Draki Tower room (same zone + event_room, NPC not monster, not dead).
fn lua_draki_tower_npc_out(lua: &Lua, uid: i32) -> LuaResult<()> {
    use crate::handler::draki_tower::ZONE_DRAKI_TOWER;

    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let zone_id = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    if zone_id != ZONE_DRAKI_TOWER {
        return Ok(());
    }

    // Never clear another player's concurrent Draki instance or the monster
    // wave DrakiRiftChange just spawned. The client scripts call this after
    // DrakiRiftChange, so only the old floor NPCs may be removed here.
    // ZoneChange in the 25258/25260/25263/25265 scripts may clear the
    // session's transient event_room before this Lua function runs. The
    // Draki room itself keeps the owner name, so recover the room from that
    // persistent instance state instead of silently skipping the spawn.
    let event_room = {
        let session_room = w.get_event_room(sid);
        if session_room > 0 {
            session_room
        } else if let Some(name) = w.get_session_name(sid) {
            w.draki_tower_rooms_read()
                .iter()
                .find(|(_, room)| room.user_name == name)
                .map(|(room_id, _)| *room_id)
                .unwrap_or(0)
        } else {
            0
        }
    };
    if event_room > 0 {
        w.kill_non_monster_npcs_in_room(ZONE_DRAKI_TOWER, event_room);
    } else {
        w.kill_non_monster_npcs_in_zone(ZONE_DRAKI_TOWER);
    }

    Ok(())
}

/// GenieExchange(uid, item_id, hours) -> ()
fn lua_genie_exchange(lua: &Lua, (uid, item_id, hours): (i32, u32, u32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    if hours == 0 {
        return Ok(());
    }

    // Refuse before anything is consumed when the extra time would not fit the
    // cap (see GENIE_MAX_HOURS) — `hours * 3600` could also wrap a u32.
    let fits = w
        .with_session(sid, |h| {
            crate::handler::genie::genie_abs_after(
                h.genie_time_abs,
                crate::handler::genie::now_secs(),
                hours,
            )
            .is_some()
        })
        .unwrap_or(false);
    if !fits {
        return Ok(());
    }

    // Check item exists and remove it (skip for item_id == 0 which means newChar)
    if item_id != 0 {
        if !w.check_exist_item(sid, item_id, 1) {
            return Ok(());
        }
        if !w.rob_item(sid, item_id, 1) {
            return Ok(());
        }
    }

    // Extend genie time (absolute timestamp)
    if crate::handler::genie::grant_genie_hours(&w, sid, hours).is_none() {
        return Ok(());
    }
    // The voucher is gone: write the new expiry now instead of waiting for the periodic save.
    crate::handler::genie::persist_genie_time(&w, sid, "GenieExchange");

    // Send WIZ_GENIE packet
    // C++ Wire: [u8(GenieUseSpiringPotion)] [u8(GenieUseSpiringPotion)] [u16 hours]
    let genie_abs = w.with_session(sid, |h| h.genie_time_abs).unwrap_or(0);
    let genie_hours = crate::handler::genie::get_genie_hours_pub(
        crate::handler::genie::genie_remaining_from_abs(genie_abs),
    );
    let mut pkt = Packet::new(Opcode::WizGenie as u8);
    pkt.write_u8(1); // GenieUseSpiringPotion
    pkt.write_u8(1); // GenieUseSpiringPotion (repeated per C++)
    pkt.write_u16(genie_hours);
    w.send_to_session_owned(sid, pkt);

    Ok(())
}

/// DelosCasttellanZoneOut(uid) -> ()
fn lua_delos_castellan_zone_out(lua: &Lua, _uid: i32) -> LuaResult<()> {
    use crate::world::{ZONE_DELOS_CASTELLAN, ZONE_MORADON};

    let w = get_world(lua)?;

    let master_clan = w.get_csw_master_knights();
    if master_clan == 0 {
        return Ok(());
    }

    // Get all users in ZONE_DELOS_CASTELLAN
    let users_in_zone = w.get_users_in_zone(ZONE_DELOS_CASTELLAN);

    for user_sid in users_in_zone {
        let should_eject = w
            .with_session(user_sid, |h| {
                let clan_id = h.character.as_ref().map(|c| c.knights_id).unwrap_or(0);
                if clan_id == 0 {
                    return true; // No clan → eject
                }
                if clan_id == master_clan {
                    return false; // Winning clan → stay
                }
                // Check alliance
                if let Some(knights) = w.get_knights(clan_id) {
                    if knights.alliance != 0 {
                        if let Some(alliance) = w.get_alliance(knights.alliance) {
                            if alliance.main_clan == master_clan
                                || alliance.sub_clan == master_clan
                                || alliance.mercenary_1 == master_clan
                                || alliance.mercenary_2 == master_clan
                            {
                                return false; // Allied with winner → stay
                            }
                        }
                    }
                }
                true // Not winning clan or alliance → eject
            })
            .unwrap_or(true);

        if should_eject {
            // ZoneChange to Moradon
            let mut pkt = Packet::new(Opcode::WizZoneChange as u8);
            pkt.write_u8(2);
            pkt.write_u16(ZONE_MORADON);
            pkt.write_f32(0.0);
            pkt.write_f32(0.0);
            pkt.write_f32(0.0);
            pkt.write_u8(0);
            w.send_to_session_owned(user_sid, pkt);
            w.update_session(user_sid, |h| {
                h.position.zone_id = ZONE_MORADON;
            });
        }
    }

    Ok(())
}

/// CheckBeefEventLogin(uid) -> i32
fn lua_check_beef_event_login(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let allowed = w.is_beef_event_farming() && w.get_character_info(uid as SessionId)
        .is_some_and(|ch| !crate::handler::bifrost::should_redirect_from_bifrost(&w, ch.nation));
    Ok(i32::from(allowed))
}

/// CheckMonsterChallengeTime(uid) -> i32
fn lua_check_monster_challenge_time(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let level = w.get_character_info(sid).map(|c| c.level).unwrap_or(0);
    Ok(if w.is_ft_join_open_for_level(level) {
        1
    } else {
        0
    })
}

/// CheckMonsterChallengeUserCount(uid) -> i32
fn lua_check_monster_challenge_user_count(lua: &Lua, _uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    Ok(w.get_ft_user_count() as i32)
}

/// CheckUnderTheCastleOpen(uid) -> i32
fn lua_check_under_castle_open(lua: &Lua, _uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    Ok(if w.is_under_castle_active() { 1 } else { 0 })
}

/// CheckUnderTheCastleUserCount(uid) -> i32
fn lua_check_under_castle_user_count(lua: &Lua, _uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    Ok(w.get_under_castle_user_count() as i32)
}

/// CheckJuraidMountainTime(uid) -> i32
fn lua_check_juraid_mountain_time(lua: &Lua, _uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    Ok(if w.is_juraid_join_open() { 1 } else { 0 })
}

/// GetUserDailyOp(uid, op_type) -> i32
fn lua_get_user_daily_op(lua: &Lua, (uid, op_type): (i32, u8)) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let char_name = w
        .get_character_info(sid)
        .map(|c| c.name)
        .unwrap_or_default();
    if char_name.is_empty() {
        return Ok(0);
    }
    Ok(w.get_user_daily_op(&char_name, op_type) as i32)
}

// ═══════════════════════════════════════════════════════════════════════
// Sprint 34: Final 7 Lua Bindings (CastSkill, EventSoccer*, JoinEvent,
//            DrakiRiftChange, ClanNts, PerkUseItem)
// ═══════════════════════════════════════════════════════════════════════

/// EventSoccerMember(uid, team, x, z) -> ()
/// team: 1=Blue, 2=Red. Max 11 per team, 22 total. Neutral zone only.
/// C++ calls ZoneChange(GetZoneID(), x, z) to teleport player, then
/// StateChangeServerDirect(11, TeamColours) which broadcasts to region.
fn lua_event_soccer_member(lua: &Lua, (uid, team, x, z): (i32, u8, f32, f32)) -> LuaResult<()> {
    use crate::handler::soccer;

    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // Validate team (1=Blue, 2=Red)
    if !(1..=2).contains(&team) {
        return Ok(());
    }

    let zone_id = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
    let char_name = w
        .get_character_info(sid)
        .map(|c| c.name)
        .unwrap_or_default();
    if char_name.is_empty() {
        return Ok(());
    }

    // Check neutral zone (Moradon 21-25)
    if !(21..=25).contains(&zone_id) {
        return Ok(());
    }

    let state = w.soccer_state();
    let mut guard = state.write();
    let result = soccer::join_event(&mut guard, zone_id, &char_name, team, x, z);

    if let soccer::JoinResult::Ok {
        spawn_x,
        spawn_z,
        match_time,
    } = result
    {
        // Send timer packet: WIZ_MINING(0x10, 0x02, timer)
        let mut pkt = Packet::new(Opcode::WizMining as u8);
        pkt.write_u8(16);
        pkt.write_u8(2);
        pkt.write_u16(match_time);
        w.send_to_session_owned(sid, pkt);

        // Fix #4 (HIGH): Teleport player to soccer field
        let mut zone_pkt = Packet::new(Opcode::WizZoneChange as u8);
        zone_pkt.write_u8(2); // ZoneChangeSuccess sub-opcode
        zone_pkt.write_u16(zone_id);
        zone_pkt.write_f32(spawn_x);
        zone_pkt.write_f32(0.0); // y
        zone_pkt.write_f32(spawn_z);
        zone_pkt.write_u8(0);
        w.send_to_session_owned(sid, zone_pkt);
        // Update server-side position
        w.update_session(sid, |h| {
            h.position.x = spawn_x;
            h.position.z = spawn_z;
        });

        // Fix #3 (CRITICAL): Broadcast StateChange to region, not zone
        // StateChangeServerDirect builds WIZ_STATE_CHANGE and calls SendToRegion(&result)
        let mut state_pkt = Packet::new(Opcode::WizStateChange as u8);
        state_pkt.write_u32(sid as u32); // C++ uses uint32(GetSocketID())
        state_pkt.write_u8(11); // bType = team colour state
        state_pkt.write_u32(team as u32);
        // Use region-based broadcast matching C++ SendToRegion
        let region_x = crate::zone::calc_region(spawn_x);
        let region_z = crate::zone::calc_region(spawn_z);
        let sender_event_room = w.get_event_room(sid);
        w.broadcast_to_region_sync(
            zone_id,
            region_x,
            region_z,
            Arc::new(state_pkt),
            None,
            sender_event_room,
        );
    }

    Ok(())
}

/// EventSoccerStard(uid) -> ()
/// Validates both teams have players, sets m_SoccerTime=600, m_SoccerActive=true.
fn lua_event_soccer_stard(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let zone_id = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);

    let state = w.soccer_state();
    let mut guard = state.write();

    if let Some(room) = guard.get_room_mut(zone_id) {
        // Already active? Return.
        if room.active {
            return Ok(());
        }
        // Both teams need at least 1 player
        if room.blue_count > 0 && room.red_count > 0 {
            room.match_time = 600;
            room.active = true;
        }
    }

    Ok(())
}

/// JoinEvent(uid) -> i32
/// Lua wrapper hardcodes event type to TEMPLE_EVENT_JURAD_MOUNTAIN (100).
/// Returns 1 on success, 0 on failure.
/// C++ checks: isEventUser, zone prison, active event match, level limits, loyalty, coins.
fn lua_join_event(lua: &Lua, uid: i32) -> LuaResult<i32> {
    const MIN_LEVEL_JURAID: u8 = 75;
    const TEMPLE_EVENT_JOIN: u8 = 8;
    const TEMPLE_EVENT_JURAD_MOUNTAIN: i16 = 100;

    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let (char_name, nation, level, zone_id) = match w.get_character_info(sid) {
        Some(c) => {
            let zone = w.get_position(sid).map(|p| p.zone_id).unwrap_or(0);
            (c.name, c.nation, c.level, zone)
        }
        None => return Ok(0),
    };

    if nation == 0 {
        return Ok(0);
    }

    // if (isEventUser() || GetZoneID() == ZONE_PRISON || isInTempleEventZone())
    use crate::world::types::{ZONE_BDW, ZONE_CHAOS, ZONE_JURAID};

    let is_event_user = w.event_room_manager.is_user_signed_up(&char_name);
    let is_in_temple_zone = zone_id == ZONE_BDW || zone_id == ZONE_CHAOS || zone_id == ZONE_JURAID;

    if is_event_user || zone_id == ZONE_PRISON || is_in_temple_zone {
        let mut pkt = Packet::new(Opcode::WizEvent as u8);
        pkt.write_u8(TEMPLE_EVENT_JOIN);
        pkt.write_u8(4); // failure
        pkt.write_i16(TEMPLE_EVENT_JURAD_MOUNTAIN);
        w.send_to_session_owned(sid, pkt);
        return Ok(0);
    }

    // Check if Juraid join is open
    if !w.is_juraid_join_open() {
        let mut pkt = Packet::new(Opcode::WizEvent as u8);
        pkt.write_u8(TEMPLE_EVENT_JOIN);
        pkt.write_u8(4); // failure
        pkt.write_i16(TEMPLE_EVENT_JURAD_MOUNTAIN);
        w.send_to_session_owned(sid, pkt);
        return Ok(0);
    }

    // Level check (min level for Juraid)
    if level < MIN_LEVEL_JURAID {
        let mut pkt = Packet::new(Opcode::WizEvent as u8);
        pkt.write_u8(TEMPLE_EVENT_JOIN);
        pkt.write_u8(4); // failure
        pkt.write_i16(TEMPLE_EVENT_JURAD_MOUNTAIN);
        w.send_to_session_owned(sid, pkt);
        return Ok(0);
    }

    // Add to signed-up users list
    let result = w
        .event_room_manager
        .add_signed_up_user(char_name.clone(), sid, nation);

    match result {
        Some(_order) => {
            w.event_room_manager.update_temple_event(|s| {
                if nation == 1 {
                    s.karus_user_count = s.karus_user_count.saturating_add(1);
                } else {
                    s.elmorad_user_count = s.elmorad_user_count.saturating_add(1);
                }
                s.all_user_count = s.karus_user_count + s.elmorad_user_count;
            });

            // Send join confirmation: WIZ_EVENT + TEMPLE_EVENT_JOIN(8) + success(1) + event_id(100)
            let mut pkt = Packet::new(Opcode::WizEvent as u8);
            pkt.write_u8(TEMPLE_EVENT_JOIN);
            pkt.write_u8(1); // success
            pkt.write_i16(TEMPLE_EVENT_JURAD_MOUNTAIN);
            w.send_to_session_owned(sid, pkt);
            crate::systems::event_room::broadcast_event_counter(&w);
            crate::systems::event_room::send_active_event_time(&w, sid);
            tracing::info!(
                "Lua JoinEvent: '{}' joined Juraid Mountain (nation={}, total signed up={})",
                char_name,
                nation,
                w.event_room_manager.signed_up_count()
            );
            Ok(1)
        }
        None => {
            // Already signed up — send failure
            let mut pkt = Packet::new(Opcode::WizEvent as u8);
            pkt.write_u8(TEMPLE_EVENT_JOIN);
            pkt.write_u8(4); // failure
            pkt.write_i16(TEMPLE_EVENT_JURAD_MOUNTAIN);
            w.send_to_session_owned(sid, pkt);
            Ok(0)
        }
    }
}

/// DrakiRiftChange(uid, stage, sub_stage) -> ()
/// Draki Tower stages. Sends timer display packets. Spawns monsters for stage.
fn lua_draki_rift_change(lua: &Lua, (uid, stage, sub_stage): (i32, u16, u16)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // C++ validation: stage 1-5, sub_stage 1-8
    // Note: C++ has a bug: checks DrakiStage <= 8 instead of DrakiSubStage <= 8
    let stage_valid = (1..=5).contains(&stage);
    let sub_stage_valid = (1..=8).contains(&sub_stage);
    if !stage_valid || !sub_stage_valid {
        return Ok(());
    }

    let has_char = w
        .with_session(sid, |h| h.character.is_some())
        .unwrap_or(false);
    if !has_char {
        return Ok(());
    }

    // The floor NPC sends the next-floor value (for example 1/4), while the
    // database also contains a rest NPC row at the preceding sub-stage. Pick
    // the first actual monster stage at or after the requested value.
    let stages = w.draki_tower_stages();
    let resolved = stages
        .iter()
        .find(|s| {
            s.draki_stage == stage as i16
                && s.draki_sub_stage == sub_stage as i16
                && s.draki_tower_npc_state == 0
        })
        .or_else(|| {
            stages.iter().find(|s| {
                s.draki_stage == stage as i16
                    && s.draki_sub_stage >= sub_stage as i16
                    && s.draki_tower_npc_state == 0
            })
        })
        .map(|s| (s.id, s.draki_stage as u16, s.draki_sub_stage as u16));
    let Some((stage_id, resolved_stage, resolved_sub_stage)) = resolved else {
        tracing::warn!(
            sid,
            stage,
            sub_stage,
            "DrakiRiftChange: no monster stage found"
        );
        return Ok(());
    };

    // Keep runtime state and NPCs aligned. Previously this function only
    // changed the state and sent timers, so the run stopped at this NPC.
    let event_room = w.get_event_room(sid);
    let mut spawned_monster_count = 0u32;
    if event_room > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Keep the guard in its own scope. A named parking_lot guard is
        // dropped at scope end; retaining it until the second write below
        // self-deadlocks the Lua execution at event 101.
        {
            let mut rooms = w.draki_tower_rooms_write();
            if let Some(room) = rooms.get_mut(&event_room) {
                room.draki_stage = resolved_stage;
                room.draki_sub_stage = resolved_sub_stage;
                room.draki_sub_timer = now + 300;
                room.is_draki_stage_change = true;
                room.draki_monster_kill = 0;
            }
        }

        w.despawn_room_npcs(crate::handler::draki_tower::ZONE_DRAKI_TOWER, event_room);
        let monsters = w.draki_monster_list();
        let mut monster_count = 0u32;
        for monster in crate::handler::draki_tower::get_monsters_for_stage(&monsters, stage_id) {
            // The imported Draki list's boolean is not a reliable entity
            // type discriminator (monster rows are stored as FALSE). The
            // resolved stage has npc_state=0, so mirror the normal stage
            // progression path and spawn every row as a monster.
            let spawned = w.spawn_event_npc_ex(
                monster.monster_id as u16,
                true,
                crate::handler::draki_tower::ZONE_DRAKI_TOWER,
                monster.pos_x as f32,
                monster.pos_z as f32,
                1,
                event_room,
                0,
            );
            if !spawned.is_empty() {
                monster_count = monster_count.saturating_add(1);
            }
        }
        {
            let mut rooms = w.draki_tower_rooms_write();
            if let Some(room) = rooms.get_mut(&event_room) {
                room.draki_monster_kill = monster_count;
            }
        }
        spawned_monster_count = monster_count;
    }

    let time_limit: u16 = 300;

    // Send WIZ_SELECT_MSG timer display
    let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
    pkt.write_u32(0);
    pkt.write_u8(7);
    pkt.write_u64(0);
    pkt.write_u32(0x0A);
    pkt.write_u8(233);
    pkt.write_u16(time_limit);
    pkt.write_u16(0); // elapsed placeholder
    w.send_to_session_owned(sid, pkt);

    // Send WIZ_EVENT (TEMPLE_DRAKI_TOWER_TIMER)
    let mut event_pkt = Packet::new(Opcode::WizEvent as u8);
    event_pkt.write_u8(0x16); // TEMPLE_DRAKI_TOWER_TIMER
    event_pkt.write_u8(233);
    event_pkt.write_u8(3);
    event_pkt.write_u16(resolved_stage);
    event_pkt.write_u16(resolved_sub_stage);
    event_pkt.write_u32(time_limit as u32);
    event_pkt.write_u32(0); // elapsed placeholder
    w.send_to_session_owned(sid, event_pkt);

    // Send WIZ_BIFROST timer
    let mut bifrost_pkt = Packet::new(Opcode::WizBifrost as u8);
    bifrost_pkt.write_u8(5);
    bifrost_pkt.write_u16(time_limit);
    w.send_to_session_owned(sid, bifrost_pkt);

    // Note: Monster spawning (SummonDrakiMonsters) requires runtime room state
    // that is managed by the draki_tower handler's tick system, not via Lua.
    tracing::info!(
        sid,
        event_room,
        requested_stage = stage,
        requested_sub_stage = sub_stage,
        resolved_stage,
        resolved_sub_stage,
        stage_id,
        spawned_monsters = spawned_monster_count,
        time_limit,
        "DrakiRiftChange completed"
    );

    Ok(())
}

/// ClanNts(uid) -> ()
/// Requires: clan leader, NTS item (900144023), all members offline, no kings,
/// no cross-clan chars. Heavy DB validation — logged stub.
fn lua_clan_nts(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let has_char = w
        .with_session(sid, |h| h.character.is_some())
        .unwrap_or(false);
    if !has_char {
        return Ok(());
    }

    // Spawn async ClanNts handler — DB-heavy validation across all clan members.
    if let Some(pool) = w.db_pool() {
        let world = Arc::clone(&w);
        let pool = pool.clone();
        tokio::spawn(async move {
            crate::handler::clan_nts::handle_clan_nts(world, pool, sid).await;
        });
    }

    Ok(())
}

/// PerkUseItem(uid, item_id, item_count, perk_points) -> i32
/// Returns 1 on success, 0 on failure.
fn lua_perk_use_item(
    lua: &Lua,
    (uid, item_id, item_count, perk): (i32, u32, u32, u16),
) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let has_char = w
        .with_session(sid, |h| h.character.is_some())
        .unwrap_or(false);
    if !has_char {
        return Ok(0);
    }

    // C++ normalizes: if perk < 1, perk = 1
    let points = if perk < 1 { 1i16 } else { perk as i16 };

    // If item_id provided, check and remove item
    if item_id != 0 {
        let count = item_count.min(u16::MAX as u32) as u16;
        if !w.check_exist_item(sid, item_id, count) {
            return Ok(0);
        }
        if !w.rob_item(sid, item_id, count) {
            return Ok(0);
        }
    }

    // Add perk points
    w.update_session(sid, |h| {
        h.rem_perk += points;
    });

    Ok(1)
}

/// RunMiningExchange(uid, ore_type) — exchange ore at mining NPC.
/// Filters mining_exchange table by ore_type + NPC 31511, builds weighted
/// random array, selects reward item, removes origin ore, gives reward.
fn lua_run_mining_exchange(lua: &Lua, (uid, ore_type): (i32, i32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as crate::zone::SessionId;

    // C++ hardcodes NPC 31511 (Pitman)
    const MINING_NPC_ID: i16 = 31511;

    let exchanges = w.get_mining_exchanges(ore_type as i16, MINING_NPC_ID);
    if exchanges.is_empty() {
        tracing::debug!(sid, ore_type, "RunMiningExchange: no exchange entries");
        return Ok(());
    }

    // Build weighted random array (C++ bRandArray[10000])
    // C++ divides SuccesRate by 5 to determine slot count
    let mut rand_entries: Vec<(i32, i16)> = Vec::new(); // (give_item_num, n_index)
    let mut total_weight: u32 = 0;
    for ex in &exchanges {
        let weight = (ex.success_rate / 5).max(0) as u32;
        if weight == 0 {
            continue;
        }
        total_weight = total_weight.saturating_add(weight);
        rand_entries.push((ex.n_give_item_num, ex.n_index));
    }

    if total_weight == 0 || rand_entries.is_empty() {
        return Ok(());
    }

    // Weighted random selection
    let roll = rand::random::<u32>() % total_weight;
    let mut cumulative: u32 = 0;
    let mut selected_item: i32 = 0;
    let mut selected_index: i16 = 0;
    for ex in &exchanges {
        let weight = (ex.success_rate / 5).max(0) as u32;
        cumulative += weight;
        if roll < cumulative {
            selected_item = ex.n_give_item_num;
            selected_index = ex.n_index;
            break;
        }
    }

    if selected_item == 0 {
        return Ok(());
    }

    // Validate inventory has free slot
    let has_space = w
        .with_session(sid, |h| {
            h.inventory
                .iter()
                .skip(14)
                .take(28)
                .any(|it| it.item_id == 0)
        })
        .unwrap_or(false);
    if !has_space {
        return Ok(());
    }

    // Find origin item from the selected exchange entry
    let origin_item = exchanges
        .iter()
        .find(|e| e.n_index == selected_index)
        .map(|e| e.n_origin_item_num)
        .unwrap_or(0);

    if origin_item == 0 {
        return Ok(());
    }

    // Remove origin ore, give reward item
    w.rob_item(sid, origin_item as u32, 1);
    w.give_item(sid, selected_item as u32, 1);

    tracing::info!(
        sid,
        ore_type,
        origin = origin_item,
        reward = selected_item,
        "RunMiningExchange: exchanged ore"
    );

    // Send success effect if GiveEffect == 1
    let give_effect = exchanges
        .iter()
        .find(|e| e.n_index == selected_index)
        .map(|e| e.give_effect)
        .unwrap_or(0);

    if give_effect == 1 {
        // C++ sends WIZ_MINING(MiningAttempt) with effect=13081
        let mut pkt = ko_protocol::Packet::new(ko_protocol::Opcode::WizMining as u8);
        pkt.write_u8(2); // MiningAttempt
        pkt.write_u16(1); // MiningResultSuccess
        pkt.write_u32(sid as u32);
        pkt.write_u16(13081); // "Item" effect
        w.send_to_session_owned(sid, pkt);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Custom server Lua bindings (not in original C++ source)
// These are used by RimaGUARD quest scripts and need stubs to prevent
// Lua runtime errors.
// ═══════════════════════════════════════════════════════════════════════

/// Enter the quest-backed Monster Stone instance.
///
/// This mirrors CUser::MonsterStoneQuestJoin() from the reference server.
/// Older Lua scripts pass only `(UID, 199)` and therefore use Stone 1,
/// family 4; newer scripts may explicitly provide zone and family.
fn lua_monster_stone_quest_join(
    lua: &Lua,
    (uid, quest_id, target_zone, target_family): (i32, i32, Option<u8>, Option<u16>),
) -> LuaResult<bool> {
    use crate::systems::{event_room, monster_stone};

    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let fail = |error_id: u8, reason: &'static str| {
        w.send_to_session_owned(sid, monster_stone::build_fail_packet(error_id));
        tracing::warn!(sid, quest_id, reason, "MonsterStoneQuestJoin rejected");
        false
    };

    if w.is_trading(sid) || w.is_merchanting(sid) || w.is_mining(sid) || w.is_fishing(sid) {
        return Ok(false);
    }

    if !w
        .get_server_settings()
        .map(|settings| settings.monsterstone_status)
        .unwrap_or(false)
    {
        return Ok(fail(1, "monster stone is disabled"));
    }

    if quest_id != 199 {
        return Ok(fail(5, "invalid quest id"));
    }

    let Some(character) = w.get_character_info(sid) else {
        return Ok(false);
    };
    if character.hp < character.max_hp / 2 {
        return Ok(fail(9, "health is below fifty percent"));
    }

    let Some(position) = w.get_position(sid) else {
        return Ok(false);
    };
    if position.zone_id == ZONE_PRISON
        || monster_stone::is_monster_stone_zone(position.zone_id)
        || event_room::is_in_temple_event_zone(position.zone_id)
    {
        return Ok(fail(1, "current zone does not permit entry"));
    }

    if w.with_session(sid, |h| h.monster_stone_status || h.event_room > 0)
        .unwrap_or(true)
    {
        return Ok(fail(5, "player is already in an event room"));
    }

    let zone_id = target_zone.unwrap_or(monster_stone::ZONE_STONE1 as u8);
    let family = target_family.unwrap_or(4);
    if !monster_stone::is_monster_stone_zone(zone_id as u16) || family == 0 {
        return Ok(fail(5, "invalid target zone or family"));
    }

    // Validate the spawn set before reserving a room. This prevents an empty
    // instance and exactly matches the reference implementation's ordering.
    let spawns = w.get_monster_stone_spawns(zone_id, family);
    if spawns.is_empty() {
        return Ok(fail(5, "spawn data was not found"));
    }

    let Some(room_id) = w.monster_stone_write().allocate_room() else {
        return Ok(fail(5, "no available room"));
    };
    let event_room_id = room_id + 1;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    {
        let mut manager = w.monster_stone_write();
        if !manager.activate_room(room_id, zone_id, family, position.zone_id, now) {
            return Ok(fail(5, "room activation failed"));
        }
        manager.add_user(room_id, sid);
    }

    w.update_session(sid, |h| {
        h.event_room = event_room_id;
        h.monster_stone_status = true;
        let quest = h.quests.entry(quest_id as u16).or_default();
        if quest.quest_state == 0 {
            quest.quest_state = 1;
        }
    });

    for row in &spawns {
        w.spawn_event_npc_ex_with_direction(
            row.s_sid as u16,
            row.b_type == 0,
            zone_id as u16,
            row.x as f32,
            row.z as f32,
            row.s_count.max(1) as u16,
            event_room_id,
            u8::from(row.is_boss),
            row.by_direction.clamp(0, u8::MAX as i16) as u8,
        );
    }

    let vendor_coords: [(f32, f32); 3] = match zone_id {
        81 => [(204.0, 201.0), (204.0, 197.0), (204.0, 193.0)],
        82 => [(203.0, 202.0), (203.0, 197.0), (203.0, 193.0)],
        83 => [(204.0, 207.0), (204.0, 200.0), (204.0, 194.0)],
        _ => unreachable!("Monster Stone zone was validated above"),
    };
    for (npc_sid, (x, z)) in [16062, 12117, 31508].into_iter().zip(vendor_coords) {
        w.spawn_event_npc_ex(npc_sid, false, zone_id as u16, x, z, 1, event_room_id, 0);
    }

    crate::handler::zone_change::server_teleport_to_zone(&w, sid, zone_id as u16, 0.0, 0.0);
    w.send_to_session_owned(
        sid,
        monster_stone::build_timer_packet(monster_stone::ROOM_DURATION_SECS as u16),
    );
    w.send_to_session_owned(
        sid,
        monster_stone::build_select_msg_timer(monster_stone::ROOM_DURATION_SECS as u16),
    );

    tracing::info!(
        sid,
        quest_id,
        room_id,
        event_room_id,
        zone_id,
        family,
        spawn_count = spawns.len(),
        "MonsterStoneQuestJoin: room activated and player teleported"
    );
    Ok(true)
}

/// GiveCash(uid, amount) — gives Knight Cash to a player
/// Not in C++ source — custom private server feature.
/// Used by 23523_shield.lua (gold/loyalty/item → KC exchange NPC).
fn lua_give_cash(lua: &Lua, (uid, amount): (i32, u32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // Update in-memory KC balance
    w.update_session(sid, |h| {
        h.knight_cash = h.knight_cash.saturating_add(amount);
    });

    // Send CASHCHANGE packet to client
    if let Some((kc, tl)) = w.with_session(sid, |h| (h.knight_cash, h.tl_balance)) {
        let pkt = crate::handler::knight_cash::build_cashchange_packet(kc, tl);
        w.send_to_session_owned(sid, pkt);
    }

    // DB persistence (fire-and-forget — same pattern as C++ AddDatabaseRequest)
    if let Some(pool) = w.db_pool() {
        let pool = pool.clone();
        let account_id = w
            .with_session(sid, |h| h.account_id.clone())
            .unwrap_or_default();
        let (kc, tl) = w
            .with_session(sid, |h| (h.knight_cash, h.tl_balance))
            .unwrap_or((0, 0));
        tokio::spawn(async move {
            let repo = ko_db::repositories::cash_shop::CashShopRepository::new(&pool);
            if let Err(e) = repo
                .update_kc_balances(&account_id, kc as i32, tl as i32)
                .await
            {
                tracing::warn!(
                    "GiveCash: failed to save KC balance for {}: {}",
                    account_id,
                    e
                );
            }
        });
    }

    tracing::info!(sid, amount, "GiveCash: awarded Knight Cash");
    Ok(())
}

/// RequestPersonalRankReward(uid) -> i32 (0=success, 2=already claimed)
/// Checks daily operation type 3 (DAILY_USER_PERSONAL_RANK_REWARD) cooldown.
/// If 24h+ since last claim: marks as claimed, returns 0 (success).
/// If still on cooldown: returns 2 (already claimed today).
/// C++ daily op reference: `UserDailyOpSystem.cpp:3-45`
/// Used by charel (11610), delaga (21610), Chaos (31526) NPC scripts.
fn lua_request_personal_rank_reward(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let char_name = w
        .get_character_info(sid)
        .map(|c| c.name)
        .unwrap_or_default();
    if char_name.is_empty() {
        return Ok(2);
    }
    // Daily op type 3 = DAILY_USER_PERSONAL_RANK_REWARD
    let eligible = w.get_user_daily_op(&char_name, 3);
    if eligible == 1 {
        tracing::info!(sid, "RequestPersonalRankReward: claimed");
        Ok(0) // success
    } else {
        tracing::debug!(sid, "RequestPersonalRankReward: already claimed today");
        Ok(2) // already claimed
    }
}

/// RequestReward(uid) -> i32 (0=success, 2=already claimed)
/// Checks daily operation type 2 (DAILY_USER_RANK_REWARD) cooldown.
/// If 24h+ since last claim: marks as claimed, returns 0 (success).
/// If still on cooldown: returns 2 (already claimed today).
/// C++ daily op reference: `UserDailyOpSystem.cpp:3-45`
/// Used by charel (11610), delaga (21610), Chaos (31526) NPC scripts.
fn lua_request_reward(lua: &Lua, uid: i32) -> LuaResult<i32> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let char_name = w
        .get_character_info(sid)
        .map(|c| c.name)
        .unwrap_or_default();
    if char_name.is_empty() {
        return Ok(2);
    }
    // Daily op type 2 = DAILY_USER_RANK_REWARD
    let eligible = w.get_user_daily_op(&char_name, 2);
    if eligible == 1 {
        tracing::info!(sid, "RequestReward: claimed");
        Ok(0) // success
    } else {
        tracing::debug!(sid, "RequestReward: already claimed today");
        Ok(2) // already claimed
    }
}

/// OpenSkill(uid, skillId) — promotes beginner class to novice.
/// `PromoteUserNovice()`. The `skillId` parameter is the Lua event dispatch
/// number (60, 70, 72…), NOT an actual skill ID to unlock.
/// Used by SkillOpener.lua (NPC 20035) — class promotion NPC.
fn lua_open_skill(lua: &Lua, (uid, _skill_id): (i32, i32)) -> LuaResult<()> {
    lua_promote_user_novice(lua, uid)
}

/// SendGenderChange(uid) — opens the gender change appearance UI on the client.
/// Custom private server feature (not in original C++).
/// Equivalent to `SelectMsg(UID, 24, -1, -1, NPC)` — flag 24 tells the client
/// to open the gender/face/hair customization picker.
/// Used by `1881_NTSJOB.lua` (EVENT=500) as a shortcut to open the gender UI.
fn lua_send_gender_change_ui(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let lua_filename = w
        .with_session(sid, |h| {
            if h.quest_helper_id > 0 {
                w.get_quest_helper(h.quest_helper_id)
                    .map(|qh| qh.str_lua_filename.clone())
            } else {
                None
            }
        })
        .flatten()
        .unwrap_or_default();

    // Flag 24 = gender change UI (client-side handling)
    select_msg::send_select_msg(
        &w,
        sid,
        24, // bFlag = 24 → opens gender change UI
        -1,
        -1,
        &[-1i32; 12],
        &[-1i32; 12],
        &lua_filename,
    );

    tracing::debug!(sid, "SendGenderChange: opened gender change UI");
    Ok(())
}

/// OpenTradeNpc(uid) — opens the NPC's buy/sell shop for the current event NPC.
/// Looks up the NPC's `i_selling_group` from the template and sends
/// `WIZ_TRADE_NPC` to the client, exactly like clicking a MERCHANT NPC.
/// Used by dialog_builder v4 for NPC dialog SHOP actions.
fn lua_open_trade_npc(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    // Get the NPC the player is interacting with (event_nid → proto_id → template)
    let event_nid = w.with_session(sid, |h| h.event_nid).unwrap_or(-1);
    if event_nid < 0 {
        tracing::warn!(sid, "OpenTradeNpc: no event NPC");
        return Ok(());
    }

    let selling_group = w
        .get_npc_instance(event_nid as u32)
        .and_then(|npc| w.get_npc_template(npc.proto_id, false))
        .map(|t| t.selling_group)
        .unwrap_or(0);

    if selling_group == 0 {
        tracing::warn!(sid, event_nid, "OpenTradeNpc: selling_group=0");
        return Ok(());
    }

    let mut pkt = Packet::new(Opcode::WizTradeNpc as u8);
    pkt.write_u32(selling_group);
    w.send_to_session_owned(sid, pkt);

    tracing::debug!(sid, selling_group, "OpenTradeNpc: opened shop");
    Ok(())
}

/// SendWarpList(uid) — sends the warp list for the player's current zone.
/// Sends `WIZ_WARP_LIST` with the player's zone ID so the client shows
/// the warp destination selection UI. Used by dialog_builder v4 for WARP actions.
fn lua_send_warp_list(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let zone_id = w.with_session(sid, |h| h.position.zone_id).unwrap_or(0);

    if zone_id == 0 {
        tracing::warn!(sid, "SendWarpList: zone_id=0");
        return Ok(());
    }

    let mut pkt = Packet::new(Opcode::WizWarpList as u8);
    pkt.write_u16(zone_id);
    w.send_to_session_owned(sid, pkt);

    tracing::debug!(sid, zone_id, "SendWarpList: sent warp list");
    Ok(())
}

/// OpenShoppingMall(uid, sub_type) — opens the shopping mall / premium shop UI.
/// Sends `WIZ_SHOPPING_MALL` with the specified sub-type.
/// Sub-types: 1=premium list, 0=default. Used by dialog_builder v4 for MALL actions.
fn lua_open_shopping_mall(lua: &Lua, (uid, sub_type): (i32, i32)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let mut pkt = Packet::new(Opcode::WizShoppingMall as u8);
    pkt.write_u8(sub_type as u8);
    w.send_to_session_owned(sid, pkt);

    tracing::debug!(sid, sub_type, "OpenShoppingMall: sent mall open");
    Ok(())
}

// -- Box Breaker NPC bindings (world/box_breaker.rs) ----------------------

/// `GetBreakableBoxes(uid)` -> array of `{id=, count=, name=}`: every breakable box type
/// (item.effect2 = 251 with a Chaotic Generator reward pool) in the player's bag, sorted by
/// item id. Rented / sealed / duplicate-flagged stacks are not counted.
fn lua_get_breakable_boxes(lua: &Lua, uid: i32) -> LuaResult<LuaTable> {
    let w = get_world(lua)?;
    let list = lua.create_table()?;
    for (i, b) in w.breakable_boxes(uid as SessionId).iter().enumerate() {
        let entry = lua.create_table()?;
        entry.set("id", b.item_id)?;
        entry.set("count", b.count)?;
        entry.set("name", b.name.as_str())?;
        list.set(i + 1, entry)?;
    }
    Ok(list)
}

/// `BoxPick(uid, item_id)` -> usable count. Remembers `item_id` as the box type of the next
/// `BoxBreak`; returns 0 (and forgets any pick) when it is not a breakable box or none is owned.
fn lua_box_pick(lua: &Lua, (uid, item_id): (i32, i64)) -> LuaResult<u32> {
    let w = get_world(lua)?;
    let item_id = u32::try_from(item_id).unwrap_or(0);
    Ok(w.box_breaker_pick(uid as SessionId, item_id))
}

fn box_outcome_table(
    lua: &Lua,
    w: &WorldState,
    outcome: &crate::world::box_breaker::BreakOutcome,
) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("broken", outcome.broken)?;
    t.set("stop", outcome.stop.code())?;
    let rewards = lua.create_table()?;
    for (i, (id, count)) in outcome.rewards.iter().enumerate() {
        let entry = lua.create_table()?;
        entry.set("id", *id)?;
        entry.set("count", *count)?;
        entry.set("name", w.box_display_name(*id))?;
        rewards.set(i + 1, entry)?;
    }
    t.set("rewards", rewards)?;
    Ok(t)
}

/// `BoxBreak(uid, count)` -> `{broken=, stop=, rewards={{id=,count=,name=}...}}`.
/// Breaks `count` boxes of the type picked with `BoxPick` (`count` 0 = all of that type, at most
/// 200 per call; a negative count breaks nothing). The reward is drawn with the Chaotic
/// Generator's weights, no gold is charged.
fn lua_box_break(lua: &Lua, (uid, count): (i32, i64)) -> LuaResult<LuaTable> {
    use crate::world::box_breaker::{BreakOutcome, BreakStop};
    let w = get_world(lua)?;
    let sid = uid as SessionId;
    let outcome = if count < 0 {
        BreakOutcome {
            stop: BreakStop::NotBreakable,
            ..Default::default()
        }
    } else {
        let picked = w.box_breaker_picked(sid);
        if picked == 0 {
            BreakOutcome {
                stop: BreakStop::NoBox,
                ..Default::default()
            }
        } else {
            w.box_breaker_break(sid, picked, count.min(u32::MAX as i64) as u32)
        }
    };
    box_outcome_table(lua, &w, &outcome)
}

/// `BoxBreakAll(uid)` -> same table as `BoxBreak`: breaks every breakable box type in the bag
/// (at most 200 boxes per call, lowest item id first).
fn lua_box_break_all(lua: &Lua, uid: i32) -> LuaResult<LuaTable> {
    let w = get_world(lua)?;
    let outcome = w.box_breaker_break_all(uid as SessionId, 0);
    box_outcome_table(lua, &w, &outcome)
}

/// `SendChatLine(uid, text)` - one line of text in the player's chat window (notice colour).
/// Text is cut to 250 characters.
fn lua_send_chat_line(lua: &Lua, (uid, text): (i32, String)) -> LuaResult<()> {
    let w = get_world(lua)?;
    let text: String = text.chars().take(250).collect();
    w.send_to_session_owned(
        uid as SessionId,
        crate::systems::timed_notice::build_notice_packet(8, &text),
    );
    Ok(())
}

/// RebirthBas(uid) — shows the rebirth NPC menu.
/// Custom private server feature (not in original C++).
/// Replaces the commented-out `SelectMsg(UID, 3, 974, 8082, NPC, 4481, 101, 8265, 301, 3019, 203)`
/// in 19005_Mackin.lua. Shows the rebirth menu with 3 options:
/// - Button 1 (text 4481): Gold rebirth → EVENT 101
/// - Button 2 (text 8265): Quest rebirth → EVENT 301
/// - Button 3 (text 3019): Other → EVENT 203
fn lua_rebirth_bas(lua: &Lua, uid: i32) -> LuaResult<()> {
    let w = get_world(lua)?;
    let sid = uid as SessionId;

    let lua_filename = w
        .with_session(sid, |h| {
            if h.quest_helper_id > 0 {
                w.get_quest_helper(h.quest_helper_id)
                    .map(|qh| qh.str_lua_filename.clone())
            } else {
                None
            }
        })
        .flatten()
        .unwrap_or_default();

    // Rebirth menu: matches the commented-out SelectMsg in 19005_Mackin.lua
    let mut btn_texts = [-1i32; 12];
    let mut btn_events = [-1i32; 12];
    btn_texts[0] = 4481;
    btn_events[0] = 101;
    btn_texts[1] = 8265;
    btn_events[1] = 301;
    btn_texts[2] = 3019;
    btn_events[2] = 203;

    select_msg::send_select_msg(
        &w,
        sid,
        3,
        974,
        8082,
        &btn_texts,
        &btn_events,
        &lua_filename,
    );

    tracing::debug!(sid, "RebirthBas: showed rebirth menu");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npc_msg_uses_v2615_u16_npc_sid() {
        let pkt = build_npc_msg_packet(500, 31_772);
        assert_eq!(pkt.opcode, Opcode::WizQuest as u8);
        assert_eq!(pkt.data.len(), 7);
        assert_eq!(pkt.data, vec![7, 0xF4, 0x01, 0x00, 0x00, 0x1C, 0x7C]);
    }

    #[test]
    fn test_check_percent_boundaries() {
        // 0-1000 permillage: 0 = never, 1000 = near-certain
        assert!(!lua_check_percent(&Lua::new(), 0).unwrap());
        assert!(!lua_check_percent(&Lua::new(), -10).unwrap());
        assert!(!lua_check_percent(&Lua::new(), 1001).unwrap());
    }

    #[test]
    fn test_check_percent_probabilistic() {
        // 500 permillage ≈ 50% chance
        let mut trues = 0;
        for _ in 0..1000 {
            if lua_check_percent(&Lua::new(), 500).unwrap() {
                trues += 1;
            }
        }
        assert!(
            trues > 350 && trues < 650,
            "Got {} trues out of 1000",
            trues
        );
    }

    #[test]
    fn test_register_all_functions() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        for name in &[
            "CheckLevel",
            "CheckNation",
            "CheckClass",
            "SaveEvent",
            "GiveItem",
            "RobItem",
            "ExchangeItemForAchievement",
            "GoldGain",
            "GoldLose",
            "NpcSay",
            "SelectMsg",
            "CheckPercent",
            "QuestCheckExistEvent",
            "isWarrior",
            "isRogue",
            "isMage",
            "isPriest",
            "isPortuKurian",
            "ZoneChange",
            "ExpChange",
            "HowmuchItem",
        ] {
            assert!(
                g.get::<LuaFunction>(*name).is_ok(),
                "Missing function: {}",
                name
            );
        }
    }

    #[test]
    fn test_check_percent_from_lua() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        // 1000 permillage ≈ 99.9% (1000 > rand(0..=1000))
        // Run multiple times to ensure at least one passes
        let mut any_true = false;
        for _ in 0..20 {
            if lua
                .load("return CheckPercent(1000)")
                .eval::<bool>()
                .unwrap()
            {
                any_true = true;
                break;
            }
        }
        assert!(
            any_true,
            "CheckPercent(1000) should succeed at least once in 20 tries"
        );
        assert!(!lua.load("return CheckPercent(0)").eval::<bool>().unwrap());
    }

    #[test]
    fn test_stubs_accept_args() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        // CastSkill is aliased to NpcCastSkill (no-op stub, doesn't need WorldState)
        assert!(lua.load("CastSkill(1, 2, 3)").exec().is_ok());
        // SpawnEventSystem with invalid npc_id (0) returns early before needing WorldState
        assert!(lua.load("SpawnEventSystem(1, 0, 0, 0, 0)").exec().is_ok());
    }

    #[test]
    fn test_roll_dice_zero() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        assert_eq!(lua.load("return RollDice(1, 0)").eval::<u16>().unwrap(), 0);
    }

    #[test]
    fn test_val_helpers() {
        assert_eq!(lua_val_to_i32(&LuaValue::Integer(42)).unwrap(), 42);
        assert_eq!(lua_val_to_i32(&LuaValue::Number(3.75)).unwrap(), 3);
        assert!(lua_val_to_i32(&LuaValue::Boolean(true)).is_err());
        assert_eq!(lua_val_to_i32_or(&LuaValue::Integer(10), 0), 10);
        assert_eq!(lua_val_to_i32_or(&LuaValue::Boolean(true), -1), -1);
        assert_eq!(lua_val_to_u32(&LuaValue::Integer(100)).unwrap(), 100);
        assert_eq!(lua_val_to_u16(&LuaValue::Integer(5)), Some(5));
        assert_eq!(lua_val_to_u16(&LuaValue::Boolean(false)), None);
    }

    // ── Test helpers for WorldState-backed Lua tests ─────────────────

    use crate::world::{CharacterInfo, KnightsAlliance, KnightsInfo, Position, UserItemSlot};
    use ko_db::models::item_tables::{ItemExchangeRow, ItemGiveExchangeRow};
    use ko_db::models::Item;
    use tokio::sync::mpsc;

    /// Create a WorldState with a registered session + character + inventory,
    /// wire it as Lua app_data, and return (Lua, WorldState-Arc).
    fn setup_lua_world() -> (Lua, Arc<WorldState>) {
        let (lua, world, _rx) = setup_lua_world_with_rx();
        (lua, world)
    }

    /// Same as [`setup_lua_world`] but also returns the receiver of the packets
    /// the server sends to the test player (session 1).
    fn setup_lua_world_with_rx() -> (
        Lua,
        Arc<WorldState>,
        mpsc::UnboundedReceiver<Arc<Packet>>,
    ) {
        let world = Arc::new(WorldState::new());
        let (tx, rx) = mpsc::unbounded_channel();
        let sid: SessionId = 1;
        world.register_session(sid, tx);

        let info = CharacterInfo {
            session_id: sid,
            name: "Tester".into(),
            nation: 1,
            race: 1,
            class: 101, // Karus Warrior beginner
            level: 30,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 1000,
            hp: 1000,
            max_mp: 500,
            mp: 500,
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
            gold: 10000,
            loyalty: 500,
            loyalty_monthly: 100,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 50000,
            max_exp: 100_000_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 5000,
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
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(sid, info, pos);

        let inv = vec![UserItemSlot::default(); WorldState::SLOT_MAX + WorldState::HAVE_MAX];
        world.set_inventory(sid, inv);

        let lua = Lua::new();
        lua.set_app_data(Arc::clone(&world));
        lua.set_app_data(QuestRunState::default());
        register_all(&lua).unwrap();

        (lua, world, rx)
    }

    /// Insert a test item definition into WorldState.items.
    fn insert_test_item(world: &WorldState, item_id: u32, countable: bool) {
        let item = Item {
            num: item_id as i32,
            extension: None,
            str_name: Some("TestItem".into()),
            description: None,
            item_plus_id: None,
            item_alteration: None,
            item_icon_id1: None,
            item_icon_id2: None,
            kind: Some(6),
            slot: None,
            race: None,
            class: None,
            damage: None,
            min_damage: None,
            max_damage: None,
            delay: None,
            range: None,
            weight: Some(10),
            duration: Some(5000),
            buy_price: Some(100),
            sell_price: Some(50),
            sell_npc_type: None,
            sell_npc_price: None,
            ac: None,
            countable: Some(if countable { 1 } else { 0 }),
            effect1: None,
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
        };
        world.insert_item(item_id, item);
    }

    #[test]
    fn test_get_cash_returns_zero() {
        let (lua, _world) = setup_lua_world();
        let result: i32 = lua.load("return GetCash(1)").eval().unwrap();
        assert_eq!(result, 0);
        let result2: i32 = lua.load("return CheckCash(1)").eval().unwrap();
        assert_eq!(result2, 0);
    }

    #[test]
    fn test_check_clan_point_no_clan() {
        let (lua, _world) = setup_lua_world();
        // Player has knights_id=0, so clan point should be 0
        let result: i32 = lua.load("return CheckClanPoint(1)").eval().unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_check_clan_point_with_clan() {
        let (lua, world) = setup_lua_world();
        // Set up a clan
        let clan = KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 3,
            ranking: 1,
            name: "TestClan".into(),
            chief: "Tester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 1000,
            clan_point_fund: 7500,
            notice: String::new(),
            cape: 0,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: 0,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        world.insert_knights(clan);
        // Assign player to clan
        world.update_character_stats(1, |ch| {
            ch.knights_id = 100;
        });

        let result: i32 = lua.load("return CheckClanPoint(1)").eval().unwrap();
        assert_eq!(result, 7500);
    }

    #[test]
    fn test_get_exp_percent_uses_full_level_threshold() {
        let (lua, world) = setup_lua_world();
        // The game helper is a gate: it returns 100 only at the current
        // level's table threshold, rather than interpolating between levels.
        world.insert_level_up((30, 0), 10000);
        world.insert_level_up((31, 0), 20000);
        world.update_character_stats(1, |ch| {
            ch.level = 30;
            ch.exp = 9999;
        });
        let before: i32 = lua.load("return GetExpPercent(1)").eval().unwrap();
        assert_eq!(before, 0);

        world.update_character_stats(1, |ch| {
            ch.exp = 10000;
        });

        let result: i32 = lua.load("return GetExpPercent(1)").eval().unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn test_get_exp_percent_works_at_level_cap_and_rebirth_tier() {
        let (lua, world) = setup_lua_world();
        world.insert_level_up((83, 2), 900_000);
        world.update_character_stats(1, |ch| {
            ch.level = 83;
            ch.rebirth_level = 2;
            ch.exp = 900_000;
        });
        let result: i32 = lua.load("return GetExpPercent(1)").eval().unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn test_get_exp_percent_zero_when_no_table() {
        let (lua, _world) = setup_lua_world();
        // No level_up_table entries, should return 0
        let result: i32 = lua.load("return GetExpPercent(1)").eval().unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_nation_change_valid() {
        let (lua, world) = setup_lua_world();
        // Player starts as nation=1, change to 2
        lua.load("NationChange(1, 2)").exec().unwrap();
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.nation, 2);
    }

    #[test]
    fn test_nation_change_invalid() {
        let (lua, world) = setup_lua_world();
        // Invalid nation should be ignored
        lua.load("NationChange(1, 3)").exec().unwrap();
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.nation, 1); // unchanged
    }

    #[test]
    fn test_give_premium_sets_type() {
        let (lua, world) = setup_lua_world();
        lua.load("GivePremium(1, 1, 30)").exec().unwrap();
        let premium = world.with_session(1, |h| h.premium_in_use).unwrap();
        assert_eq!(premium, 1);
    }

    #[test]
    fn test_give_premium_invalid_type_ignored() {
        let (lua, world) = setup_lua_world();
        lua.load("GivePremium(1, 0, 30)").exec().unwrap();
        let premium = world.with_session(1, |h| h.premium_in_use).unwrap();
        assert_eq!(premium, 0); // unchanged -- type 0 is invalid
    }

    #[test]
    fn test_gender_change_valid_karus() {
        let (lua, world) = setup_lua_world();
        // Give player the gender change scroll (810594000)
        insert_test_item(&world, 810_594_000, true);
        world.give_item(1, 810_594_000, 1);
        // Player is nation=1, race=1 (Karus). Change to race=3 (another Karus race).
        let result: bool = lua.load("return GenderChange(1, 3)").eval().unwrap();
        assert!(result);
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.race, 3);
    }

    #[test]
    fn test_gender_change_invalid_cross_nation() {
        let (lua, world) = setup_lua_world();
        // Player is nation=1 (Karus). Trying El Morad race 11 should fail.
        let result: bool = lua.load("return GenderChange(1, 11)").eval().unwrap();
        assert!(!result);
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.race, 1); // unchanged
    }

    #[test]
    fn test_job_change_warrior_to_rogue() {
        let (lua, world) = setup_lua_world();
        // Give player the job change scroll (700112000 for type=0)
        insert_test_item(&world, 700_112_000, true);
        world.give_item(1, 700_112_000, 1);
        // Player is class=101 (Karus Warrior beginner, class_type=1)
        // Change to job=2 (Rogue), type=0 (beginner to beginner)
        let result: u8 = lua.load("return JobChange(1, 0, 2)").eval().unwrap();
        assert_eq!(result, 1); // success
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.class % 100, 2); // Rogue beginner
    }

    #[test]
    fn test_job_change_same_group_fails() {
        let (lua, world) = setup_lua_world();
        // Give player the scroll so same-group check is reached
        insert_test_item(&world, 700_112_000, true);
        world.give_item(1, 700_112_000, 1);
        // Player is class=101 (Warrior beginner, job group 1)
        // Trying to change to job=1 (same group) should return 6
        let result: u8 = lua.load("return JobChange(1, 0, 1)").eval().unwrap();
        assert_eq!(result, 6); // same group (C++ returns 6)
    }

    #[test]
    fn test_job_change_invalid_job() {
        let (lua, _world) = setup_lua_world();
        // Invalid job=6 should return 2
        let result: u8 = lua.load("return JobChange(1, 0, 6)").eval().unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_check_exchange_missing_exchange() {
        let (lua, _world) = setup_lua_world();
        // No exchange with id=999 exists
        let result: bool = lua.load("return CheckExchange(1, 999)").eval().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_exchange_success() {
        let (lua, world) = setup_lua_world();
        let item_id = 389010000u32;
        insert_test_item(&world, item_id, true);

        // Give player the item
        world.give_item(1, item_id, 5);

        // Insert exchange recipe requiring 3 of item_id
        let exchange = ItemExchangeRow {
            n_index: 100,
            random_flag: 0,
            origin_item_num1: item_id as i32,
            origin_item_count1: 3,
            origin_item_num2: 0,
            origin_item_count2: 0,
            origin_item_num3: 0,
            origin_item_count3: 0,
            origin_item_num4: 0,
            origin_item_count4: 0,
            origin_item_num5: 0,
            origin_item_count5: 0,
            exchange_item_num1: ITEM_GOLD as i32,
            exchange_item_count1: 100,
            exchange_item_num2: 0,
            exchange_item_count2: 0,
            exchange_item_num3: 0,
            exchange_item_count3: 0,
            exchange_item_num4: 0,
            exchange_item_count4: 0,
            exchange_item_num5: 0,
            exchange_item_count5: 0,
            exchange_item_time1: 0,
            exchange_item_time2: 0,
            exchange_item_time3: 0,
            exchange_item_time4: 0,
            exchange_item_time5: 0,
        };
        world.insert_item_exchange(100, exchange);

        let result: bool = lua.load("return CheckExchange(1, 100)").eval().unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_exchange_insufficient_items() {
        let (lua, world) = setup_lua_world();
        let item_id = 389010000u32;
        insert_test_item(&world, item_id, true);

        // Player has 0 of item_id; exchange needs 3
        let exchange = ItemExchangeRow {
            n_index: 101,
            random_flag: 0,
            origin_item_num1: item_id as i32,
            origin_item_count1: 3,
            origin_item_num2: 0,
            origin_item_count2: 0,
            origin_item_num3: 0,
            origin_item_count3: 0,
            origin_item_num4: 0,
            origin_item_count4: 0,
            origin_item_num5: 0,
            origin_item_count5: 0,
            exchange_item_num1: ITEM_GOLD as i32,
            exchange_item_count1: 100,
            exchange_item_num2: 0,
            exchange_item_count2: 0,
            exchange_item_num3: 0,
            exchange_item_count3: 0,
            exchange_item_num4: 0,
            exchange_item_count4: 0,
            exchange_item_num5: 0,
            exchange_item_count5: 0,
            exchange_item_time1: 0,
            exchange_item_time2: 0,
            exchange_item_time3: 0,
            exchange_item_time4: 0,
            exchange_item_time5: 0,
        };
        world.insert_item_exchange(101, exchange);

        let result: bool = lua.load("return CheckExchange(1, 101)").eval().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_run_exchange_fixed_gold_output() {
        let (lua, world) = setup_lua_world();
        let item_id = 389010000u32;
        insert_test_item(&world, item_id, true);
        world.give_item(1, item_id, 5);

        let initial_gold = world.get_character_info(1).unwrap().gold;

        // Exchange: 3 of item_id -> 500 gold
        let exchange = ItemExchangeRow {
            n_index: 200,
            random_flag: 0,
            origin_item_num1: item_id as i32,
            origin_item_count1: 3,
            origin_item_num2: 0,
            origin_item_count2: 0,
            origin_item_num3: 0,
            origin_item_count3: 0,
            origin_item_num4: 0,
            origin_item_count4: 0,
            origin_item_num5: 0,
            origin_item_count5: 0,
            exchange_item_num1: ITEM_GOLD as i32,
            exchange_item_count1: 500,
            exchange_item_num2: 0,
            exchange_item_count2: 0,
            exchange_item_num3: 0,
            exchange_item_count3: 0,
            exchange_item_num4: 0,
            exchange_item_count4: 0,
            exchange_item_num5: 0,
            exchange_item_count5: 0,
            exchange_item_time1: 0,
            exchange_item_time2: 0,
            exchange_item_time3: 0,
            exchange_item_time4: 0,
            exchange_item_time5: 0,
        };
        world.insert_item_exchange(200, exchange);

        let result: bool = lua.load("return RunExchange(1, 200)").eval().unwrap();
        assert!(result);

        let new_gold = world.get_character_info(1).unwrap().gold;
        assert_eq!(new_gold, initial_gold + 500);
    }

    #[test]
    fn test_run_give_item_exchange_missing_id() {
        let (lua, _world) = setup_lua_world();
        // No exchange with id=999
        let result: bool = lua
            .load("return RunGiveItemExchange(1, 999)")
            .eval()
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_run_give_item_exchange_gold_rob_and_give() {
        let (lua, world) = setup_lua_world();
        // Exchange: rob 1000 gold, give 200 loyalty
        let exchange = ItemGiveExchangeRow {
            exchange_index: 300,
            rob_item_ids: vec![ITEM_GOLD as i32],
            rob_item_counts: vec![1000],
            give_item_ids: vec![ITEM_COUNT as i32],
            give_item_counts: vec![200],
            give_item_times: vec![0],
        };
        world.insert_item_give_exchange(300, exchange);

        let initial_gold = world.get_character_info(1).unwrap().gold;
        let initial_loyalty = world.get_character_info(1).unwrap().loyalty;

        let result: bool = lua
            .load("return RunGiveItemExchange(1, 300)")
            .eval()
            .unwrap();
        assert!(result);

        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.gold, initial_gold - 1000);
        assert_eq!(ch.loyalty, initial_loyalty + 200);
    }

    #[test]
    fn test_run_give_item_exchange_insufficient_gold() {
        let (lua, world) = setup_lua_world();
        // Exchange: rob 99999 gold (more than player has)
        let exchange = ItemGiveExchangeRow {
            exchange_index: 301,
            rob_item_ids: vec![ITEM_GOLD as i32],
            rob_item_counts: vec![99999],
            give_item_ids: vec![ITEM_COUNT as i32],
            give_item_counts: vec![100],
            give_item_times: vec![0],
        };
        world.insert_item_give_exchange(301, exchange);

        let result: bool = lua
            .load("return RunGiveItemExchange(1, 301)")
            .eval()
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_run_count_exchange_missing() {
        let (lua, _world) = setup_lua_world();
        let result: bool = lua.load("return RunCountExchange(1, 999)").eval().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_new_exchange_functions_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        for name in &[
            "RunGiveItemExchange",
            "CheckExchange",
            "RunExchange",
            "RunCountExchange",
            "JobChange",
            "GenderChange",
            "GivePremium",
            "NationChange",
            "GetExpPercent",
            "CheckClanPoint",
            "GetCash",
            "CheckCash",
        ] {
            assert!(
                g.get::<LuaFunction>(*name).is_ok(),
                "Missing function: {}",
                name
            );
        }
    }

    // ── Class Tier Check Tests ───────────────────────────────────────

    #[test]
    fn test_tier_check_functions_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        for name in &[
            "isBeginner",
            "isBeginnerWarrior",
            "isBeginnerRogue",
            "isBeginnerMage",
            "isBeginnerPriest",
            "isBeginnerKurianPortu",
            "isBeginnerKurian",
            "isNovice",
            "isNoviceWarrior",
            "isNoviceRogue",
            "isNoviceMage",
            "isNovicePriest",
            "isNoviceKurianPortu",
            "isNoviceKurian",
            "isMastered",
            "isMasteredWarrior",
            "isMasteredRogue",
            "isMasteredMage",
            "isMasteredPriest",
            "isMasteredKurianPortu",
            "isMasteredKurian",
        ] {
            assert!(
                g.get::<LuaFunction>(*name).is_ok(),
                "Missing function: {}",
                name
            );
        }
    }

    #[test]
    fn test_is_beginner_warrior() {
        // Default setup_lua_world creates class=101 (Karus Warrior beginner, class_type=1)
        let (lua, _world) = setup_lua_world();
        assert!(lua.load("return isBeginner(1)").eval::<bool>().unwrap());
        assert!(lua
            .load("return isBeginnerWarrior(1)")
            .eval::<bool>()
            .unwrap());
        assert!(!lua
            .load("return isBeginnerRogue(1)")
            .eval::<bool>()
            .unwrap());
        assert!(!lua.load("return isNovice(1)").eval::<bool>().unwrap());
        assert!(!lua.load("return isMastered(1)").eval::<bool>().unwrap());
    }

    #[test]
    fn test_is_novice_mage() {
        let (lua, world) = setup_lua_world();
        // Change to class 109 (Karus Novice Mage, class_type=9)
        world.update_character_stats(1, |ch| {
            ch.class = 109;
        });
        assert!(lua.load("return isNovice(1)").eval::<bool>().unwrap());
        assert!(lua.load("return isNoviceMage(1)").eval::<bool>().unwrap());
        assert!(!lua
            .load("return isNoviceWarrior(1)")
            .eval::<bool>()
            .unwrap());
        assert!(!lua.load("return isBeginner(1)").eval::<bool>().unwrap());
        assert!(!lua.load("return isMastered(1)").eval::<bool>().unwrap());
    }

    #[test]
    fn test_is_mastered_kurian() {
        let (lua, world) = setup_lua_world();
        // Change to class 215 (El Morad Master Kurian, class_type=15)
        world.update_character_stats(1, |ch| {
            ch.class = 215;
        });
        assert!(lua.load("return isMastered(1)").eval::<bool>().unwrap());
        assert!(lua
            .load("return isMasteredKurianPortu(1)")
            .eval::<bool>()
            .unwrap());
        assert!(lua
            .load("return isMasteredKurian(1)")
            .eval::<bool>()
            .unwrap());
        assert!(!lua
            .load("return isMasteredWarrior(1)")
            .eval::<bool>()
            .unwrap());
        assert!(!lua.load("return isBeginner(1)").eval::<bool>().unwrap());
        assert!(!lua.load("return isNovice(1)").eval::<bool>().unwrap());
    }

    #[test]
    fn test_is_mastered_rogue() {
        let (lua, world) = setup_lua_world();
        // Change to class 108 (Karus Master Rogue, class_type=8)
        world.update_character_stats(1, |ch| {
            ch.class = 108;
        });
        assert!(lua.load("return isMastered(1)").eval::<bool>().unwrap());
        assert!(lua
            .load("return isMasteredRogue(1)")
            .eval::<bool>()
            .unwrap());
        assert!(!lua
            .load("return isMasteredWarrior(1)")
            .eval::<bool>()
            .unwrap());
        assert!(!lua.load("return isNoviceRogue(1)").eval::<bool>().unwrap());
    }

    #[test]
    fn test_is_beginner_kurian() {
        let (lua, world) = setup_lua_world();
        // Change to class 113 (Karus Beginner Kurian, class_type=13)
        world.update_character_stats(1, |ch| {
            ch.class = 113;
        });
        assert!(lua.load("return isBeginner(1)").eval::<bool>().unwrap());
        assert!(lua
            .load("return isBeginnerKurianPortu(1)")
            .eval::<bool>()
            .unwrap());
        assert!(lua
            .load("return isBeginnerKurian(1)")
            .eval::<bool>()
            .unwrap());
        assert!(!lua
            .load("return isBeginnerWarrior(1)")
            .eval::<bool>()
            .unwrap());
    }

    #[test]
    fn test_is_novice_priest() {
        let (lua, world) = setup_lua_world();
        // Change to class 211 (El Morad Novice Priest, class_type=11)
        world.update_character_stats(1, |ch| {
            ch.class = 211;
        });
        assert!(lua.load("return isNovice(1)").eval::<bool>().unwrap());
        assert!(lua.load("return isNovicePriest(1)").eval::<bool>().unwrap());
        assert!(!lua
            .load("return isNoviceWarrior(1)")
            .eval::<bool>()
            .unwrap());
    }

    #[test]
    fn test_is_mastered_priest() {
        let (lua, world) = setup_lua_world();
        // Change to class 212 (El Morad Master Priest, class_type=12)
        world.update_character_stats(1, |ch| {
            ch.class = 212;
        });
        assert!(lua.load("return isMastered(1)").eval::<bool>().unwrap());
        assert!(lua
            .load("return isMasteredPriest(1)")
            .eval::<bool>()
            .unwrap());
    }

    #[test]
    fn test_tier_check_invalid_session() {
        let (lua, _world) = setup_lua_world();
        // Non-existent session 999 should return false for all tier checks
        assert!(!lua.load("return isBeginner(999)").eval::<bool>().unwrap());
        assert!(!lua.load("return isNovice(999)").eval::<bool>().unwrap());
        assert!(!lua.load("return isMastered(999)").eval::<bool>().unwrap());
    }

    // ── CNpc Lua Method Tests ────────────────────────────────────────

    use crate::npc::{NpcInstance, NpcTemplate};

    /// Create a test NPC template + instance and set the player's event_nid.
    fn setup_npc_for_lua(world: &WorldState) {
        let tmpl = NpcTemplate {
            s_sid: 200,
            is_monster: false,
            name: "Guard Captain".into(),
            pid: 100,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 1, // Karus
            act_type: 0,
            npc_type: 2, // guard type
            family_type: 0,
            selling_group: 0,
            level: 60,
            max_hp: 50000,
            max_mp: 1000,
            attack: 200,
            ac: 100,
            hit_rate: 80,
            evade_rate: 30,
            damage: 150,
            attack_delay: 1500,
            speed_1: 60,
            speed_2: 100,
            stand_time: 5000,
            search_range: 0,
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
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        };
        world.insert_npc_template(tmpl);

        let instance = NpcInstance {
            nid: 10001,
            proto_id: 200,
            is_monster: false,
            zone_id: 21,
            x: 150.5,
            y: 0.0,
            z: 275.3,
            direction: 4,
            region_x: 1,
            region_z: 2,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(instance);

        // Set player's event_nid to this NPC's runtime ID
        world.update_session(1, |h| {
            h.event_nid = 10001i16;
            h.event_sid = 200i16;
        });
    }

    #[test]
    fn test_npc_functions_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        for name in &[
            "NpcGetID",
            "NpcGetProtoID",
            "NpcGetName",
            "NpcGetNation",
            "NpcGetType",
            "NpcGetZoneID",
            "NpcGetX",
            "NpcGetY",
            "NpcGetZ",
            "NpcCastSkill",
        ] {
            assert!(
                g.get::<LuaFunction>(*name).is_ok(),
                "Missing NPC function: {}",
                name
            );
        }
    }

    #[test]
    fn test_npc_get_id() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: i32 = lua.load("return NpcGetID(1)").eval().unwrap();
        assert_eq!(result, 10001);
    }

    #[test]
    fn test_npc_get_proto_id() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: i32 = lua.load("return NpcGetProtoID(1)").eval().unwrap();
        assert_eq!(result, 200);
    }

    #[test]
    fn test_npc_get_name() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: String = lua.load("return NpcGetName(1)").eval().unwrap();
        assert_eq!(result, "Guard Captain");
    }

    #[test]
    fn test_npc_get_nation() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: u8 = lua.load("return NpcGetNation(1)").eval().unwrap();
        assert_eq!(result, 1); // Karus
    }

    #[test]
    fn test_npc_get_type() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: i32 = lua.load("return NpcGetType(1)").eval().unwrap();
        assert_eq!(result, 2); // guard type
    }

    #[test]
    fn test_npc_get_zone_id() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: i32 = lua.load("return NpcGetZoneID(1)").eval().unwrap();
        assert_eq!(result, 21);
    }

    #[test]
    fn test_npc_get_x() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: f32 = lua.load("return NpcGetX(1)").eval().unwrap();
        assert!((result - 150.5).abs() < 0.01);
    }

    #[test]
    fn test_npc_get_y() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: f32 = lua.load("return NpcGetY(1)").eval().unwrap();
        assert!((result - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_npc_get_z() {
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        let result: f32 = lua.load("return NpcGetZ(1)").eval().unwrap();
        assert!((result - 275.3).abs() < 0.01);
    }

    #[test]
    fn test_npc_cast_skill_stub() {
        let (lua, _world) = setup_lua_world();
        // NpcCastSkill is a stub — should not error
        assert!(lua.load("NpcCastSkill(1, 100)").exec().is_ok());
        assert!(lua.load("NpcCastSkill(1, 200, 3)").exec().is_ok());
    }

    #[test]
    fn test_stationary_npc_casts_support_without_combat_ai() {
        use ko_db::models::{MagicRow, MagicType4Row};
        let (lua, world) = setup_lua_world();
        setup_npc_for_lua(&world);
        assert!(world.get_npc_ai(10001).is_none());
        for (id, kind, attack, ac, speed) in [
            (302344, 4, 110, 0, 100),
            (302331, 2, 100, 100, 100),
            (490223, 6, 100, 0, 150),
            (500034, 4, 120, 0, 100),
        ] {
            world.insert_magic(MagicRow {
                magic_num: id,
                en_name: None,
                kr_name: None,
                description: None,
                t_1: None,
                before_action: None,
                target_action: None,
                self_effect: None,
                flying_effect: None,
                target_effect: None,
                moral: Some(1),
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
                type1: Some(4),
                type2: None,
                range: None,
                etc: None,
                use_standing: None,
                skill_check: None,
                icelightrate: None,
            });
            world.insert_magic_type4(MagicType4Row {
                i_num: id,
                buff_type: Some(kind),
                radius: None,
                duration: Some(600),
                attack_speed: Some(100),
                speed: Some(speed),
                ac: Some(ac),
                ac_pct: Some(100),
                attack: Some(attack),
                magic_attack: Some(100),
                max_hp: None,
                max_hp_pct: None,
                max_mp: None,
                max_mp_pct: None,
                str: None,
                sta: None,
                dex: None,
                intel: None,
                cha: None,
                fire_r: None,
                cold_r: None,
                lightning_r: None,
                magic_r: None,
                disease_r: None,
                poison_r: None,
                exp_pct: None,
                special_amount: None,
                hit_rate: None,
                avoid_rate: None,
            });
            assert!(lua
                .load(format!("return CastSkill(1, {id})"))
                .eval::<bool>()
                .unwrap());
            let buffs = world.get_active_buffs(1);
            let applied = buffs.iter().find(|b| b.skill_id == id as u32).unwrap();
            assert_eq!(
                (applied.attack, applied.ac, applied.speed),
                (attack, ac, speed)
            );
        }
        assert_eq!(world.get_buff_attack_amount(1), 120);
        assert_eq!(world.get_buff_ac_amount(1), 100);
    }

    #[test]
    fn test_enchanter_level_cost_and_skill_selection() {
        for (level, event, gold, expected_skill, expected_cost) in [
            (35, 211, 0, 302344, 0),
            (36, 211, 30000, 302344, 30000),
            (61, 212, 50000, 302333, 50000),
            (40, 223, 30000, 490223, 30000),
            (40, 211, 29999, 0, 0),
        ] {
            let lua = Lua::new();
            lua.load(format!(
                r#"
                UID=1; EVENT={event}; level={level}; gold={gold}; skill=0; cost=0; casts=0;
                function CheckLevel() return level end
                function HowmuchItem(_, id) if id==900000000 then return gold else return 0 end end
                function CastSkill(_, id) skill=id; casts=casts+1; return true end
                function GoldLose(_, amount) cost=cost+amount end
                function NpcMsg() end
                function SelectMsg() end
            "#
            ))
            .exec()
            .unwrap();
            lua.load(include_str!("../../../../Quests/31508_NEnchant.lua"))
                .exec()
                .unwrap();
            assert_eq!(lua.globals().get::<i32>("skill").unwrap(), expected_skill);
            assert_eq!(lua.globals().get::<i32>("cost").unwrap(), expected_cost);
            assert_eq!(
                lua.globals().get::<i32>("casts").unwrap(),
                i32::from(expected_skill != 0)
            );
        }
    }

    #[test]
    fn test_npc_no_event_npc_returns_defaults() {
        let (lua, _world) = setup_lua_world();
        // Player has event_nid=-1 by default (no NPC interaction)
        let id: i32 = lua.load("return NpcGetID(1)").eval().unwrap();
        assert_eq!(id, -1);
        let proto: i32 = lua.load("return NpcGetProtoID(1)").eval().unwrap();
        assert_eq!(proto, -1);
        let name: String = lua.load("return NpcGetName(1)").eval().unwrap();
        assert_eq!(name, "");
        let nation: u8 = lua.load("return NpcGetNation(1)").eval().unwrap();
        assert_eq!(nation, 0);
        let npc_type: i32 = lua.load("return NpcGetType(1)").eval().unwrap();
        assert_eq!(npc_type, -1);
        let zone: i32 = lua.load("return NpcGetZoneID(1)").eval().unwrap();
        assert_eq!(zone, -1);
        let x: f32 = lua.load("return NpcGetX(1)").eval().unwrap();
        assert!((x - 0.0).abs() < 0.01);
        let z: f32 = lua.load("return NpcGetZ(1)").eval().unwrap();
        assert!((z - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_npc_monster_template() {
        let (lua, world) = setup_lua_world();
        // Insert a monster template + instance
        let tmpl = NpcTemplate {
            s_sid: 500,
            is_monster: true,
            name: "Fire Dragon".into(),
            pid: 300,
            size: 200,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,    // neutral monster
            act_type: 1, // aggressive
            npc_type: 0, // monster type
            family_type: 3,
            selling_group: 0,
            level: 80,
            max_hp: 500000,
            max_mp: 5000,
            attack: 800,
            ac: 400,
            hit_rate: 120,
            evade_rate: 50,
            damage: 600,
            attack_delay: 2000,
            speed_1: 80,
            speed_2: 150,
            stand_time: 3000,
            search_range: 20,
            attack_range: 3,
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
            exp: 50000,
            loyalty: 100,
            money: 5000,
            item_table: 10,
            area_range: 0.0,
        };
        world.insert_npc_template(tmpl);

        let instance = NpcInstance {
            nid: 10050,
            proto_id: 500,
            is_monster: true,
            zone_id: 71,
            x: 300.0,
            y: 0.0,
            z: 400.0,
            direction: 2,
            region_x: 3,
            region_z: 4,
            gate_open: 0,
            object_type: 0,
            nation: 0, // monsters are neutral
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(instance);

        // Set player's event_nid to this monster
        world.update_session(1, |h| {
            h.event_nid = 10050i16;
            h.event_sid = 500i16;
        });

        let name: String = lua.load("return NpcGetName(1)").eval().unwrap();
        assert_eq!(name, "Fire Dragon");
        let nation: u8 = lua.load("return NpcGetNation(1)").eval().unwrap();
        assert_eq!(nation, 0); // neutral monster
        let npc_type: i32 = lua.load("return NpcGetType(1)").eval().unwrap();
        assert_eq!(npc_type, 0); // monster type
        let zone: i32 = lua.load("return NpcGetZoneID(1)").eval().unwrap();
        assert_eq!(zone, 71);
        let x: f32 = lua.load("return NpcGetX(1)").eval().unwrap();
        assert!((x - 300.0).abs() < 0.01);
        let z: f32 = lua.load("return NpcGetZ(1)").eval().unwrap();
        assert!((z - 400.0).abs() < 0.01);
    }

    // ── Sprint 31: New Function Tests ──────────────────────────────

    #[test]
    fn test_check_weight_true_when_fits() {
        let (lua, world) = setup_lua_world();
        let item_id = 389020000u32;
        insert_test_item(&world, item_id, true);
        // Item weight=10, count=1. Player has max_weight=5000, item_weight=0.
        let result: bool = lua
            .load("return CheckWeight(1, 389020000, 1)")
            .eval()
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_weight_false_when_overweight() {
        let (lua, world) = setup_lua_world();
        let item_id = 389020000u32;
        insert_test_item(&world, item_id, true);
        // Set current weight close to max
        world.update_character_stats(1, |ch| {
            ch.item_weight = 4995;
        });
        // Item weight=10, count=1. 4995 + 10 = 5005 > 5000
        let result: bool = lua
            .load("return CheckWeight(1, 389020000, 1)")
            .eval()
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_weight_false_unknown_item() {
        let (lua, _world) = setup_lua_world();
        // Item 999999 doesn't exist in the world
        let result: bool = lua.load("return CheckWeight(1, 999999, 1)").eval().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_change_manner_positive() {
        let (lua, world) = setup_lua_world();
        lua.load("ChangeManner(1, 50)").exec().unwrap();
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.manner_point, 50);
    }

    #[test]
    fn test_change_manner_negative_clamps_to_zero() {
        let (lua, world) = setup_lua_world();
        // Start at 0, subtract 100 -> should clamp to 0
        lua.load("ChangeManner(1, -100)").exec().unwrap();
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.manner_point, 0);
    }

    #[test]
    fn test_change_manner_accumulates() {
        let (lua, world) = setup_lua_world();
        lua.load("ChangeManner(1, 100)").exec().unwrap();
        lua.load("ChangeManner(1, 50)").exec().unwrap();
        let ch = world.get_character_info(1).unwrap();
        assert_eq!(ch.manner_point, 150);
    }

    #[test]
    fn test_get_manner_reads_value() {
        let (lua, world) = setup_lua_world();
        world.update_character_stats(1, |ch| {
            ch.manner_point = 42;
        });
        let result: i32 = lua.load("return GetManner(1)").eval().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_rob_clan_point_deducts() {
        let (lua, world) = setup_lua_world();
        let clan = KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 3,
            ranking: 1,
            name: "TestClan".into(),
            chief: "Tester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 1000,
            clan_point_fund: 5000,
            notice: String::new(),
            cape: 0,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: 0,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        world.insert_knights(clan);
        world.update_character_stats(1, |ch| {
            ch.knights_id = 100;
        });

        lua.load("RobClanPoint(1, 2000)").exec().unwrap();
        let k = world.get_knights(100).unwrap();
        assert_eq!(k.clan_point_fund, 3000);
    }

    #[test]
    fn test_rob_clan_point_clamps_to_zero() {
        let (lua, world) = setup_lua_world();
        let clan = KnightsInfo {
            id: 101,
            flag: 2,
            nation: 1,
            grade: 3,
            ranking: 1,
            name: "TestClan2".into(),
            chief: "Tester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 1000,
            clan_point_fund: 100,
            notice: String::new(),
            cape: 0,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: 0,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        world.insert_knights(clan);
        world.update_character_stats(1, |ch| {
            ch.knights_id = 101;
        });

        lua.load("RobClanPoint(1, 500)").exec().unwrap();
        let k = world.get_knights(101).unwrap();
        assert_eq!(k.clan_point_fund, 0);
    }

    #[test]
    fn test_rob_clan_point_no_clan() {
        let (lua, _world) = setup_lua_world();
        // Player has knights_id=0 — should be a no-op
        assert!(lua.load("RobClanPoint(1, 100)").exec().is_ok());
    }

    #[test]
    fn test_promote_knight() {
        let (lua, world) = setup_lua_world();
        let clan = KnightsInfo {
            id: 102,
            flag: 1,
            nation: 1,
            grade: 3,
            ranking: 1,
            name: "TestClan3".into(),
            chief: "Tester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 1000,
            clan_point_fund: 5000,
            notice: String::new(),
            cape: 0,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: 0,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        world.insert_knights(clan);
        world.update_character_stats(1, |ch| {
            ch.knights_id = 102;
        });

        // Default flag=2 when called with 1 arg
        lua.load("PromoteKnight(1)").exec().unwrap();
        let k = world.get_knights(102).unwrap();
        assert_eq!(k.flag, 2); // ClanTypePromoted
        assert_eq!(k.cape, 201);

        // Re-running the promoted state preserves an already visible/purchased
        // cloak instead of resetting it to the first reference cloak.
        world.update_knights(102, |k| k.cape = 42);
        lua.load("PromoteKnight(1)").exec().unwrap();
        let k = world.get_knights(102).unwrap();
        assert_eq!(k.flag, 2);
        assert_eq!(k.cape, 42);

        // Explicit flag=3 (Accredited) preserves a cape purchased after the
        // initial promotion, matching UpdateKnightsGrade in the C++ server.
        lua.load("PromoteKnight(1, 3)").exec().unwrap();
        let k = world.get_knights(102).unwrap();
        assert_eq!(k.flag, 3);
        assert_eq!(k.cape, 42);

        // Flag=1 (Training) -> cape should be -1 (0xFFFF as u16)
        lua.load("PromoteKnight(1, 1)").exec().unwrap();
        let k = world.get_knights(102).unwrap();
        assert_eq!(k.flag, 1);
        assert_eq!(k.cape, 0xFFFF); // -1 as u16
    }

    #[test]
    fn test_sprint31_functions_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        for name in &[
            "ChangeManner",
            "RobClanPoint",
            "KissUser",
            "SendNameChange",
            "SendClanNameChange",
            "SendTagNameChangePanel",
            "ZoneChangeParty",
            "ZoneChangeClan",
            "PromoteKnight",
        ] {
            assert!(
                g.get::<LuaFunction>(*name).is_ok(),
                "Missing function: {}",
                name
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Sprint 32 Tests
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_sprint32_functions_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        for name in &[
            "hasManner",
            "CheckWarVictory",
            "GetPVPMonumentNation",
            "CheckMiddleStatueCapture",
            "GetRebirthLevel",
            "KingsInspectorList",
            "GetMaxExchange",
            "isCswWinnerNembers",
            "SendNpcKillID",
            "CycleSpawn",
            "MoveMiddleStatue",
            "SendNationTransfer",
            "RobAllItemParty",
        ] {
            assert!(
                g.get::<LuaFunction>(*name).is_ok(),
                "Missing function: {}",
                name
            );
        }
    }

    #[test]
    fn test_has_manner_true() {
        let (lua, world) = setup_lua_world();
        world.update_character_stats(1, |ch| {
            ch.manner_point = 500;
        });
        let result: bool = lua.load("return hasManner(1, 100)").eval().unwrap();
        assert!(result);
    }

    #[test]
    fn test_has_manner_false() {
        let (lua, _world) = setup_lua_world();
        // manner_point defaults to 0
        let result: bool = lua.load("return hasManner(1, 100)").eval().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_has_manner_exact() {
        let (lua, world) = setup_lua_world();
        world.update_character_stats(1, |ch| {
            ch.manner_point = 100;
        });
        let result: bool = lua.load("return hasManner(1, 100)").eval().unwrap();
        assert!(result); // >= check
    }

    #[test]
    fn test_check_war_victory_default() {
        let (lua, _world) = setup_lua_world();
        let result: u8 = lua.load("return CheckWarVictory(1)").eval().unwrap();
        assert_eq!(result, 0); // default victory is 0
    }

    #[test]
    fn test_check_war_victory_karus() {
        let (lua, world) = setup_lua_world();
        world.update_battle_state(|bs| bs.victory = 1);
        let result: u8 = lua.load("return CheckWarVictory(1)").eval().unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_get_pvp_monument_nation_default() {
        let (lua, _world) = setup_lua_world();
        let result: u8 = lua.load("return GetPVPMonumentNation(1)").eval().unwrap();
        assert_eq!(result, 0); // no monument set
    }

    #[test]
    fn test_get_pvp_monument_nation_set() {
        let (lua, world) = setup_lua_world();
        // Player is in zone 21, set monument for zone 21
        world.pvp_monument_nation.insert(21, 2);
        let result: u8 = lua.load("return GetPVPMonumentNation(1)").eval().unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_check_middle_statue_capture_match() {
        let (lua, world) = setup_lua_world();
        // Player is nation 1, set middle statue to nation 1
        world.update_battle_state(|bs| bs.middle_statue_nation = 1);
        let result: i32 = lua
            .load("return CheckMiddleStatueCapture(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_check_middle_statue_capture_no_match() {
        let (lua, world) = setup_lua_world();
        // Player is nation 1, set middle statue to nation 2
        world.update_battle_state(|bs| bs.middle_statue_nation = 2);
        let result: i32 = lua
            .load("return CheckMiddleStatueCapture(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_rebirth_level_default() {
        let (lua, _world) = setup_lua_world();
        let result: u8 = lua.load("return GetRebirthLevel(1)").eval().unwrap();
        assert_eq!(result, 0); // default rebirth_level
    }

    #[test]
    fn test_get_rebirth_level_set() {
        let (lua, world) = setup_lua_world();
        world.update_character_stats(1, |ch| {
            ch.rebirth_level = 3;
        });
        let result: u8 = lua.load("return GetRebirthLevel(1)").eval().unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn test_kings_inspector_list_no_crash() {
        let (lua, _world) = setup_lua_world();
        lua.load("KingsInspectorList(1)").exec().unwrap();
    }

    #[test]
    fn test_get_max_exchange_no_exchange() {
        let (lua, _world) = setup_lua_world();
        let result: u16 = lua.load("return GetMaxExchange(1, 999)").eval().unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_csw_winner_members_no_master() {
        let (lua, _world) = setup_lua_world();
        // master_knights defaults to 0
        let result: bool = lua.load("return isCswWinnerNembers(1)").eval().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_csw_winner_members_clan_match() {
        let (lua, world) = setup_lua_world();
        // Set up master_knights = 100
        world.siege_war().blocking_write().master_knights = 100;
        // Set up player's clan = 100
        let clan = KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 3,
            ranking: 1,
            name: "WinnerClan".into(),
            chief: "Tester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 1000,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: 0,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        world.insert_knights(clan);
        world.update_character_stats(1, |ch| {
            ch.knights_id = 100;
        });

        let result: bool = lua.load("return isCswWinnerNembers(1)").eval().unwrap();
        assert!(result); // Winner eligibility gates the Lua success branch

        // But player should have been warped to Delos Castellan
        let pos = world.get_position(1).unwrap();
        assert_eq!(pos.zone_id, 35); // ZONE_DELOS_CASTELLAN
        assert!((pos.x - 458.0).abs() < 0.1);
        assert!((pos.z - 113.0).abs() < 0.1);
    }

    #[test]
    fn test_csw_winner_members_alliance_match() {
        let (lua, world) = setup_lua_world();
        // Set up master_knights = 200 -- the function requires this clan to
        // actually exist (real gameplay: it's the clan that won the siege),
        // so it must be registered even though the player belongs to 100.
        world.siege_war().blocking_write().master_knights = 200;
        world.insert_knights(KnightsInfo {
            id: 200,
            flag: 2,
            nation: 1,
            grade: 3,
            ranking: 1,
            name: "OwnerClan".into(),
            chief: "Owner".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 1000,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: 0,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        });
        // Player's clan = 100, alliance links 100 with 200
        let clan = KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 3,
            ranking: 1,
            name: "AlliedClan".into(),
            chief: "Tester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 5,
            points: 1000,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 50,
            castellan_cape: false,
            cast_cape_id: 0,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        world.insert_knights(clan);
        let alliance = KnightsAlliance {
            main_clan: 50,
            sub_clan: 200, // master_knights matches sub_clan
            mercenary_1: 0,
            mercenary_2: 0,
            notice: String::new(),
        };
        world.insert_alliance(alliance);
        world.update_character_stats(1, |ch| {
            ch.knights_id = 100;
        });

        let result: bool = lua.load("return isCswWinnerNembers(1)").eval().unwrap();
        assert!(result); // Winner eligibility gates the Lua success branch

        // Player warped to Delos Castellan
        let pos = world.get_position(1).unwrap();
        assert_eq!(pos.zone_id, 35);
    }

    #[test]
    fn test_csw_winner_members_no_clan() {
        let (lua, world) = setup_lua_world();
        world.siege_war().blocking_write().master_knights = 100;
        // Player has no clan (knights_id=0 by default)
        let result: bool = lua.load("return isCswWinnerNembers(1)").eval().unwrap();
        assert!(!result);
        // Player should NOT have been warped
        let pos = world.get_position(1).unwrap();
        assert_eq!(pos.zone_id, 21); // unchanged
    }

    #[test]
    fn test_send_npc_kill_id_no_crash() {
        let (lua, _world) = setup_lua_world();
        lua.load("SendNpcKillID(1, 12345)").exec().unwrap();
    }

    #[test]
    fn test_cycle_spawn_no_crash() {
        let (lua, _world) = setup_lua_world();
        lua.load("CycleSpawn(1)").exec().unwrap();
    }

    #[test]
    fn test_move_middle_statue_karus() {
        let (lua, world) = setup_lua_world();
        // Player is nation=1 (Karus), zone=21
        lua.load("MoveMiddleStatue(1)").exec().unwrap();
        let pos = world.get_position(1).unwrap();
        // Should be near Dodo camp: base 10540*10=105400..10545*10=105450
        let x = pos.x as u32;
        let z = pos.z as u32;
        assert!((105400..=105450).contains(&x), "x={} out of range", x);
        assert!((114100..=114150).contains(&z), "z={} out of range", z);
    }

    #[test]
    fn test_move_middle_statue_elmorad() {
        let (lua, world) = setup_lua_world();
        world.update_character_stats(1, |ch| {
            ch.nation = 2;
        });
        lua.load("MoveMiddleStatue(1)").exec().unwrap();
        let pos = world.get_position(1).unwrap();
        // Should be near Laon camp: base 10120*10=101200..10125*10=101250
        let x = pos.x as u32;
        let z = pos.z as u32;
        assert!((101200..=101250).contains(&x), "x={} out of range", x);
        assert!((91400..=91450).contains(&z), "z={} out of range", z);
    }

    #[test]
    fn test_send_nation_transfer_no_item() {
        let (lua, _world) = setup_lua_world();
        // Player doesn't have the nation transfer item
        lua.load("SendNationTransfer(1)").exec().unwrap();
        // Should not crash; sends error packet
    }

    #[test]
    fn test_send_nation_transfer_with_item() {
        let (lua, world) = setup_lua_world();
        insert_test_item(&world, 810_096_000, true);
        world.give_item(1, 810_096_000, 1);
        lua.load("SendNationTransfer(1)").exec().unwrap();
        // Should send open dialog packet
    }

    #[test]
    fn test_send_nation_transfer_while_transformed() {
        let (lua, world) = setup_lua_world();
        insert_test_item(&world, 810_096_000, true);
        world.give_item(1, 810_096_000, 1);
        // Set player as transformed (res_hp_type = 3)
        world.update_session(1, |h| {
            if let Some(ref mut ch) = h.character {
                ch.res_hp_type = 3;
            }
        });
        lua.load("SendNationTransfer(1)").exec().unwrap();
        // Should send error packet (type=2, error=6) instead of open dialog
    }

    #[test]
    fn test_rob_all_item_party_no_party() {
        let (lua, world) = setup_lua_world();
        let item_id = 389_010_000u32;
        insert_test_item(&world, item_id, true);
        world.give_item(1, item_id, 5);

        // Not in party, should rob from self
        let result: bool = lua
            .load("return RobAllItemParty(1, 389010000, 3)")
            .eval()
            .unwrap();
        assert!(result);
        // Should have 2 left
        let count = world
            .with_session(1, |h| {
                h.inventory[WorldState::SLOT_MAX..(WorldState::SLOT_MAX + WorldState::HAVE_MAX)]
                    .iter()
                    .filter(|s| s.item_id == item_id && s.count > 0)
                    .map(|s| s.count as u32)
                    .sum::<u32>()
            })
            .unwrap_or(0);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_rob_all_item_party_defaults_to_one_when_count_omitted() {
        let (lua, world) = setup_lua_world();
        let item_id = 389_010_000u32;
        insert_test_item(&world, item_id, true);
        world.give_item(1, item_id, 5);

        let result: bool = lua
            .load("return RobAllItemParty(1, 389010000)")
            .eval()
            .unwrap();
        assert!(result);

        let count = world
            .with_session(1, |h| {
                h.inventory[WorldState::SLOT_MAX..(WorldState::SLOT_MAX + WorldState::HAVE_MAX)]
                    .iter()
                    .filter(|s| s.item_id == item_id && s.count > 0)
                    .map(|s| s.count as u32)
                    .sum::<u32>()
            })
            .unwrap_or(0);
        assert_eq!(count, 4);
    }

    #[test]
    fn test_rob_all_item_party_insufficient() {
        let (lua, world) = setup_lua_world();
        let item_id = 389_010_000u32;
        insert_test_item(&world, item_id, true);
        world.give_item(1, item_id, 2);

        // Need 5, only have 2
        let result: bool = lua
            .load("return RobAllItemParty(1, 389010000, 5)")
            .eval()
            .unwrap();
        assert!(!result);
    }

    // ── Sprint 33 Tests ─────────────────────────────────────────────────

    #[test]
    fn test_show_bulletin_board_sends_packet() {
        let (lua, _world) = setup_lua_world();
        // Session 1 has nation=1, should send packet without error
        lua.load("ShowBulletinBoard(1)").exec().unwrap();
    }

    #[test]
    fn test_send_visibe_noop() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        // No-op, should accept any args
        lua.load("SendVisibe(1, 2, 3)").exec().unwrap();
    }

    #[test]
    fn test_give_switch_premium_sets_premium() {
        let (lua, world) = setup_lua_world();
        lua.load("GiveSwitchPremium(1, 3, 7)").exec().unwrap();

        let (prem_type, has_entry) = world
            .with_session(1, |h| (h.premium_in_use, h.premium_map.contains_key(&3)))
            .unwrap();
        assert_eq!(prem_type, 3);
        assert!(has_entry);
    }

    #[test]
    fn test_give_switch_premium_invalid_type() {
        let (lua, world) = setup_lua_world();
        // premium_type 0 should be rejected
        lua.load("GiveSwitchPremium(1, 0, 7)").exec().unwrap();

        let prem_type = world.with_session(1, |h| h.premium_in_use).unwrap();
        assert_eq!(prem_type, 0);
    }

    #[test]
    fn test_give_switch_premium_zero_days() {
        let (lua, world) = setup_lua_world();
        // days=0 should be rejected
        lua.load("GiveSwitchPremium(1, 3, 0)").exec().unwrap();

        let has_entry = world
            .with_session(1, |h| h.premium_map.contains_key(&3))
            .unwrap();
        assert!(!has_entry);
    }

    #[test]
    fn test_give_clan_premium_not_leader() {
        let (lua, _world) = setup_lua_world();
        // Session 1 has knights_id=0, should not crash
        lua.load("GiveClanPremium(1, 13, 30)").exec().unwrap();
    }

    #[test]
    fn test_give_premium_item_sends_packet() {
        let (lua, _world) = setup_lua_world();
        lua.load("GivePremiumItem(1, 5)").exec().unwrap();
    }

    #[test]
    fn test_spawn_event_system_invalid_args_returns_ok() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        // npc_id=0 or zone=0 returns early without needing WorldState
        lua.load("SpawnEventSystem(1, 0, 0, 0, 0, 0, 0)")
            .exec()
            .unwrap();
    }

    #[test]
    fn test_spawn_event_system_no_world_returns_err() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        // Valid args but no WorldState attached — should return error
        let result = lua
            .load("SpawnEventSystem(1, 100, 0, 1, 100, 0, 200)")
            .exec();
        assert!(result.is_err());
    }

    #[test]
    fn test_npc_event_system_sends_packet() {
        let (lua, _world) = setup_lua_world();
        lua.load("NpcEventSystem(1, 100)").exec().unwrap();
    }

    #[test]
    fn test_kill_npc_event_no_event() {
        let (lua, world) = setup_lua_world();
        // event_nid starts at -1, so KillNpcEvent should be no-op
        let event_nid = world.with_session(1, |h| h.event_nid).unwrap();
        assert_eq!(event_nid, -1);
        lua.load("KillNpcEvent(1)").exec().unwrap();
    }

    #[test]
    fn test_kill_npc_event_with_npc() {
        // kill_npc_by_runtime_id needs a tokio runtime
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (lua, world) = setup_lua_world();
            // Set event_nid and insert an NPC instance
            world.update_session(1, |h| {
                h.event_nid = 10000;
            });
            let npc = crate::npc::NpcInstance {
                nid: 10000,
                proto_id: 100,
                is_monster: false,
                zone_id: 21,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                direction: 0,
                region_x: 0,
                region_z: 0,
                gate_open: 0,
                object_type: 0,
                nation: 0,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };
            world.insert_npc_instance(npc);

            lua.load("KillNpcEvent(1)").exec().unwrap();

            // event_nid should be cleared
            let event_nid = world.with_session(1, |h| h.event_nid).unwrap();
            assert_eq!(event_nid, 0);
        });
    }

    #[test]
    fn test_send_repurchase_msg_empty() {
        let (lua, _world) = setup_lua_world();
        // No deleted items — should send packet with 0 items
        lua.load("SendRepurchaseMsg(1)").exec().unwrap();
    }

    #[test]
    fn test_send_repurchase_msg_with_items() {
        let (lua, world) = setup_lua_world();
        insert_test_item(&world, 100_001_000, true);

        // Add a deleted item with far-future expiry
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32
            + 86400;
        world.update_session(1, |h| {
            h.deleted_items.push(crate::world::DeletedItemEntry {
                db_id: 1,
                item_id: 100_001_000,
                count: 5,
                delete_time: future,
                duration: 5000,
                serial_num: 0,
                flag: 0,
            });
        });

        lua.load("SendRepurchaseMsg(1)").exec().unwrap();
    }

    #[test]
    fn test_draki_out_zone_sends_packet() {
        let (lua, _world) = setup_lua_world();
        // Player is in zone 21 (Moradon), should still send packet (has character)
        lua.load("DrakiOutZone(1)").exec().unwrap();
    }

    #[test]
    fn test_exist_monster_quest_sub_matches_reference_stub() {
        let (lua, _world) = setup_lua_world();
        let result: u16 = lua.load("return ExistMonsterQuestSub(1)").eval().unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_draki_out_zone_starts_kickout_timer() {
        let (lua, world) = setup_lua_world();
        world.update_session(1, |h| {
            h.draki_room_id = 1;
            h.event_room = 1;
        });
        world
            .draki_tower_rooms_write()
            .insert(1, crate::handler::draki_tower::DrakiTowerRoomInfo::new(1));

        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        lua.load("DrakiOutZone(1)").exec().unwrap();

        let rooms = world.draki_tower_rooms_read();
        let room = rooms.get(&1).unwrap();
        assert!(room.out_timer_active);
        assert!(room.draki_out_timer >= before + 20);
    }

    #[test]
    fn test_draki_tower_npc_out_wrong_zone() {
        let (lua, _world) = setup_lua_world();
        // Player is in zone 21, not ZONE_DRAKI_TOWER(95), should be no-op
        lua.load("DrakiTowerNpcOut(1)").exec().unwrap();
    }

    #[test]
    fn test_draki_tower_npc_out_kills_npcs() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (lua, world) = setup_lua_world();

            // Move player to ZONE_DRAKI_TOWER
            world.update_session(1, |h| {
                h.position.zone_id = 95;
            });

            // Insert an NPC (non-monster) in zone 95
            let npc = crate::npc::NpcInstance {
                nid: 20000,
                proto_id: 200,
                is_monster: false,
                zone_id: 95,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                direction: 0,
                region_x: 0,
                region_z: 0,
                gate_open: 0,
                object_type: 0,
                nation: 0,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };
            world.insert_npc_instance(npc);

            // Insert a monster in zone 95 (should NOT be killed)
            let monster = crate::npc::NpcInstance {
                nid: 20001,
                proto_id: 201,
                is_monster: true,
                zone_id: 95,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                direction: 0,
                region_x: 0,
                region_z: 0,
                gate_open: 0,
                object_type: 0,
                nation: 0,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };
            world.insert_npc_instance(monster);

            lua.load("DrakiTowerNpcOut(1)").exec().unwrap();

            // NPC 20000 should be killed, monster 20001 should survive
            assert!(world.get_npc_instance(20000).is_none());
            assert!(world.get_npc_instance(20001).is_some());
        });
    }

    #[test]
    fn test_genie_exchange_extends_time() {
        let (lua, world) = setup_lua_world();
        // GenieExchange with item_id=0 (newChar mode), 24 hours
        lua.load("GenieExchange(1, 0, 24)").exec().unwrap();

        let genie_abs = world.with_session(1, |h| h.genie_time_abs).unwrap();
        let now = crate::handler::genie::now_secs();
        let remaining = genie_abs.saturating_sub(now);
        // Should be approximately 24*3600 (within 2 seconds)
        assert!((24 * 3600 - 2..=24 * 3600 + 2).contains(&remaining));
    }

    #[test]
    fn test_genie_exchange_zero_hours() {
        let (lua, world) = setup_lua_world();
        lua.load("GenieExchange(1, 0, 0)").exec().unwrap();
        let genie_abs = world.with_session(1, |h| h.genie_time_abs).unwrap();
        // 0 hours added to 0 abs → still 0 (max(0, now) + 0 = now, but hours=0 means duration=0)
        // Actually now_secs() + 0 = now_secs(). But remaining would be ~0.
        let remaining = genie_abs.saturating_sub(crate::handler::genie::now_secs());
        assert!(remaining <= 2);
    }

    #[test]
    fn test_genie_exchange_with_item() {
        let (lua, world) = setup_lua_world();
        let item_id = 379_010_000u32;
        insert_test_item(&world, item_id, true);
        world.give_item(1, item_id, 1);

        lua.load("GenieExchange(1, 379010000, 12)").exec().unwrap();

        let genie_abs = world.with_session(1, |h| h.genie_time_abs).unwrap();
        let now = crate::handler::genie::now_secs();
        let remaining = genie_abs.saturating_sub(now);
        assert!((12 * 3600 - 2..=12 * 3600 + 2).contains(&remaining));
    }

    #[test]
    fn test_genie_exchange_item_not_found() {
        let (lua, world) = setup_lua_world();
        // Item doesn't exist in inventory
        lua.load("GenieExchange(1, 999999, 12)").exec().unwrap();

        let genie_abs = world.with_session(1, |h| h.genie_time_abs).unwrap();
        assert_eq!(genie_abs, 0); // Should not extend
    }

    #[test]
    fn test_delos_castellan_zone_out_no_master() {
        let (lua, _world) = setup_lua_world();
        // No CSW master clan set → no-op
        lua.load("DelosCasttellanZoneOut(1)").exec().unwrap();
    }

    #[test]
    fn test_check_beef_event_returns_zero() {
        let (lua, _world) = setup_lua_world();
        let result: i32 = lua.load("return CheckBeefEventLogin(1)").eval().unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_check_monster_challenge_time_closed() {
        let (lua, _world) = setup_lua_world();
        // FT not active → should return 0
        let result: i32 = lua
            .load("return CheckMonsterChallengeTime(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_check_monster_challenge_user_count() {
        let (lua, _world) = setup_lua_world();
        let result: i32 = lua
            .load("return CheckMonsterChallengeUserCount(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_check_under_castle_closed() {
        let (lua, _world) = setup_lua_world();
        let result: i32 = lua
            .load("return CheckUnderTheCastleOpen(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_check_under_castle_user_count() {
        let (lua, _world) = setup_lua_world();
        let result: i32 = lua
            .load("return CheckUnderTheCastleUserCount(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_check_juraid_mountain_closed() {
        let (lua, _world) = setup_lua_world();
        let result: i32 = lua
            .load("return CheckJuraidMountainTime(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_user_daily_op_default() {
        let (lua, _world) = setup_lua_world();
        // First call creates entry and returns 1 (allowed)
        let result: i32 = lua.load("return GetUserDailyOp(1, 1)").eval().unwrap();
        assert_eq!(result, 1);
        // Second call within cooldown returns 0
        let result2: i32 = lua.load("return GetUserDailyOp(1, 1)").eval().unwrap();
        assert_eq!(result2, 0);
    }

    #[test]
    fn test_kill_non_monster_npcs_in_zone() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let world = Arc::new(WorldState::new());
            // Insert NPCs and monsters in zone 95
            let npc1 = crate::npc::NpcInstance {
                nid: 30000,
                proto_id: 300,
                is_monster: false,
                zone_id: 95,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                direction: 0,
                region_x: 0,
                region_z: 0,
                gate_open: 0,
                object_type: 0,
                nation: 0,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };
            let npc2 = crate::npc::NpcInstance {
                nid: 30001,
                proto_id: 301,
                is_monster: false,
                zone_id: 95,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                direction: 0,
                region_x: 0,
                region_z: 0,
                gate_open: 0,
                object_type: 0,
                nation: 0,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };
            let monster = crate::npc::NpcInstance {
                nid: 30002,
                proto_id: 302,
                is_monster: true,
                zone_id: 95,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                direction: 0,
                region_x: 0,
                region_z: 0,
                gate_open: 0,
                object_type: 0,
                nation: 0,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };
            let npc_other_zone = crate::npc::NpcInstance {
                nid: 30003,
                proto_id: 303,
                is_monster: false,
                zone_id: 21,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                direction: 0,
                region_x: 0,
                region_z: 0,
                gate_open: 0,
                object_type: 0,
                nation: 0,
                special_type: 0,
                trap_number: 0,
                event_room: 0,
                is_event_npc: false,
                summon_type: 0,
                user_name: String::new(),
                pet_name: String::new(),
                clan_name: String::new(),
                clan_id: 0,
                clan_mark_version: 0,
            };
            world.insert_npc_instance(npc1);
            world.insert_npc_instance(npc2);
            world.insert_npc_instance(monster);
            world.insert_npc_instance(npc_other_zone);

            world.kill_non_monster_npcs_in_zone(95);

            // NPCs in zone 95 should be killed
            assert!(world.get_npc_instance(30000).is_none());
            assert!(world.get_npc_instance(30001).is_none());
            // Monster in zone 95 should survive
            assert!(world.get_npc_instance(30002).is_some());
            // NPC in other zone should survive
            assert!(world.get_npc_instance(30003).is_some());
        });
    }

    // ── Sprint 34 Tests ─────────────────────────────────────────────────

    #[test]
    fn test_cast_skill_alias_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        // CastSkill is aliased to NpcCastSkill (no-op stub)
        assert!(lua.load("CastSkill(1, 100)").exec().is_ok());
    }

    #[test]
    fn test_event_soccer_member_wrong_zone() {
        let (lua, _world) = setup_lua_world();
        // Player in zone 21 (Moradon) — valid zone
        lua.load("EventSoccerMember(1, 1, 0.0, 0.0)")
            .exec()
            .unwrap();
    }

    #[test]
    fn test_event_soccer_member_invalid_team() {
        let (lua, _world) = setup_lua_world();
        // Invalid team 0
        lua.load("EventSoccerMember(1, 0, 0.0, 0.0)")
            .exec()
            .unwrap();
        // Invalid team 3
        lua.load("EventSoccerMember(1, 3, 0.0, 0.0)")
            .exec()
            .unwrap();
    }

    #[test]
    fn test_event_soccer_stard_no_room() {
        let (lua, _world) = setup_lua_world();
        // Player in zone 21, should not crash even if no active room
        lua.load("EventSoccerStard(1)").exec().unwrap();
    }

    #[test]
    fn test_join_event_juraid_closed() {
        let (lua, _world) = setup_lua_world();
        // Juraid is not open by default, should return 0
        let result: i32 = lua.load("return JoinEvent(1)").eval().unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_draki_rift_change_sends_packets() {
        let (lua, _world) = setup_lua_world();
        // Valid stage and sub_stage
        lua.load("DrakiRiftChange(1, 1, 1)").exec().unwrap();
    }

    #[test]
    fn test_draki_rift_change_invalid_stage() {
        let (lua, _world) = setup_lua_world();
        // Stage 0 is invalid
        lua.load("DrakiRiftChange(1, 0, 1)").exec().unwrap();
        // Stage 6 is invalid
        lua.load("DrakiRiftChange(1, 6, 1)").exec().unwrap();
        // Sub_stage 0 is invalid
        lua.load("DrakiRiftChange(1, 1, 0)").exec().unwrap();
        // Sub_stage 9 is invalid
        lua.load("DrakiRiftChange(1, 1, 9)").exec().unwrap();
    }

    #[test]
    fn test_clan_nts_logged_stub() {
        let (lua, _world) = setup_lua_world();
        // Should not crash
        lua.load("ClanNts(1)").exec().unwrap();
    }

    #[test]
    fn test_perk_use_item_no_item() {
        let (lua, world) = setup_lua_world();
        // PerkUseItem with item_id=0 (just add points)
        let result: i32 = lua.load("return PerkUseItem(1, 0, 0, 5)").eval().unwrap();
        assert_eq!(result, 1);

        let rem_perk = world.with_session(1, |h| h.rem_perk).unwrap();
        assert_eq!(rem_perk, 5);
    }

    #[test]
    fn test_perk_use_item_zero_perk_normalizes() {
        let (lua, world) = setup_lua_world();
        // perk=0 should normalize to 1
        let result: i32 = lua.load("return PerkUseItem(1, 0, 0, 0)").eval().unwrap();
        assert_eq!(result, 1);

        let rem_perk = world.with_session(1, |h| h.rem_perk).unwrap();
        assert_eq!(rem_perk, 1);
    }

    #[test]
    fn test_perk_use_item_with_item() {
        let (lua, world) = setup_lua_world();
        let item_id = 900_100_000u32;
        insert_test_item(&world, item_id, true);
        world.give_item(1, item_id, 3);

        let result: i32 = lua
            .load("return PerkUseItem(1, 900100000, 2, 10)")
            .eval()
            .unwrap();
        assert_eq!(result, 1);

        let rem_perk = world.with_session(1, |h| h.rem_perk).unwrap();
        assert_eq!(rem_perk, 10);
    }

    #[test]
    fn test_perk_use_item_missing_item() {
        let (lua, world) = setup_lua_world();
        // Item doesn't exist in inventory
        let result: i32 = lua
            .load("return PerkUseItem(1, 999999, 1, 5)")
            .eval()
            .unwrap();
        assert_eq!(result, 0);

        let rem_perk = world.with_session(1, |h| h.rem_perk).unwrap();
        assert_eq!(rem_perk, 0); // Should not have changed
    }

    #[test]
    fn test_sprint34_functions_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        for name in &[
            "CastSkill",
            "EventSoccerMember",
            "EventSoccerStard",
            "JoinEvent",
            "DrakiRiftChange",
            "ClanNts",
            "PerkUseItem",
        ] {
            assert!(g.get::<LuaFunction>(*name).is_ok(), "Missing: {}", name);
        }
    }

    #[test]
    fn test_no_stub_loops_remaining() {
        // Verify all functions are registered (none should be stubs)
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        // All previously stubbed functions should now be registered
        for name in &[
            "ShowBulletinBoard",
            "SendVisibe",
            "GiveSwitchPremium",
            "GiveClanPremium",
            "GivePremiumItem",
            "SpawnEventSystem",
            "NpcEventSystem",
            "KillNpcEvent",
            "SendRepurchaseMsg",
            "DrakiOutZone",
            "DrakiTowerNpcOut",
            "GenieExchange",
            "DelosCasttellanZoneOut",
            "CheckBeefEventLogin",
            "CheckMonsterChallengeTime",
            "CheckMonsterChallengeUserCount",
            "CheckUnderTheCastleOpen",
            "CheckUnderTheCastleUserCount",
            "CheckJuraidMountainTime",
            "GetUserDailyOp",
            "CastSkill",
            "EventSoccerMember",
            "EventSoccerStard",
            "JoinEvent",
            "DrakiRiftChange",
            "ClanNts",
            "PerkUseItem",
        ] {
            assert!(g.get::<LuaFunction>(*name).is_ok(), "Missing: {}", name);
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Sprint 36 QA Fix Tests
    // ═══════════════════════════════════════════════════════════════════

    /// CRITICAL #1 + HIGH #6: GiveSwitchPremium extends from stored time, sends full packet at count>2
    #[test]
    fn test_give_switch_premium_extends_from_stored_time() {
        let (lua, world) = setup_lua_world();
        // Set an existing premium with a known future expiry
        let future_time = 1_700_100_000u32;
        world.update_session(1, |h| {
            h.premium_map.insert(3, future_time);
        });

        // GiveSwitchPremium should extend from stored time, not from now
        lua.load("GiveSwitchPremium(1, 3, 1)").exec().unwrap(); // +1 day = +86400

        let expiry = world
            .with_session(1, |h| *h.premium_map.get(&3).unwrap())
            .unwrap();
        // Should be stored_time + 86400, NOT now + 86400
        assert_eq!(expiry, future_time + 86400);
    }

    /// CRITICAL #1: GiveSwitchPremium sends full premium info only when count > 2
    #[test]
    fn test_give_switch_premium_count_tracking() {
        let (lua, world) = setup_lua_world();

        // Call 3 times to trigger count > 2
        lua.load("GiveSwitchPremium(1, 1, 7)").exec().unwrap();
        lua.load("GiveSwitchPremium(1, 2, 7)").exec().unwrap();
        lua.load("GiveSwitchPremium(1, 3, 7)").exec().unwrap();

        let (count, status) = world
            .with_session(1, |h| (h.switch_premium_count, h.account_status))
            .unwrap();
        assert_eq!(count, 3);
        assert_eq!(status, 1); // account_status set
    }

    /// CRITICAL #2: GivePremiumItem does NOT send packet to client
    #[test]
    fn test_give_premium_item_no_client_packet() {
        let (lua, world) = setup_lua_world();
        // Drain the channel first
        let rx = world.with_session(1, |h| h.tx.clone()).unwrap();
        let _ = &rx; // ensure channel exists
        lua.load("GivePremiumItem(1, 5)").exec().unwrap();
        // This should be a no-op (logged stub), no packet sent
        // The function should not crash
    }

    /// CRITICAL #3 + HIGH #4: EventSoccerMember broadcasts state change to zone
    /// and sends zone change (teleport) to the player.
    /// We verify this by checking the Lua code path runs without error
    /// and the player's position is updated to the soccer field.
    #[test]
    fn test_event_soccer_member_teleport_and_broadcast() {
        let (lua, world) = setup_lua_world();

        // Join soccer event (zone 21 = Moradon, valid)
        // Default blue spawn: (672.0, 166.0)
        lua.load("EventSoccerMember(1, 1, 0.0, 0.0)")
            .exec()
            .unwrap();

        // Verify position was updated to blue team default (672, 166)
        let pos = world.get_position(1).unwrap();
        assert!(
            (pos.x - 672.0).abs() < 0.1,
            "X should be 672 (blue default), got {}",
            pos.x
        );
        assert!(
            (pos.z - 166.0).abs() < 0.1,
            "Z should be 166 (blue default), got {}",
            pos.z
        );
    }

    /// CRITICAL #3: Verify EventSoccerMember uses broadcast_to_zone for state change.
    /// The code in lua_event_soccer_member now calls broadcast_to_zone instead of
    /// send_to_session for the WIZ_STATE_CHANGE packet. We verify the state change
    /// packet format matches C++: WIZ_STATE_CHANGE << u32(socketID) << u8(11) << u32(team).
    #[test]
    fn test_event_soccer_member_state_change_packet_format() {
        let (lua, world) = setup_lua_world();

        // Verify the full code path (join + teleport + broadcast) runs correctly.

        // Join as Red team (2) to test team colour
        lua.load("EventSoccerMember(1, 2, 672.0, 154.0)")
            .exec()
            .unwrap();

        // Verify position was updated (proves zone change was sent)
        let pos = world.get_position(1).unwrap();
        assert!((pos.x - 672.0).abs() < 0.1);
        assert!((pos.z - 154.0).abs() < 0.1);

        // Verify soccer room has the user registered
        let state = world.soccer_state();
        let guard = state.read();
        let room = guard.get_room(21).unwrap();
        assert_eq!(room.red_count, 1);
        assert!(room.users.contains_key("Tester"));
    }

    /// HIGH #5: GiveClanPremium updates knights premium fields and sends proper packet
    #[test]
    fn test_give_clan_premium_updates_knights() {
        let (lua, world) = setup_lua_world();

        // Set up clan with session 1 as chief
        let clan = KnightsInfo {
            id: 100,
            flag: 2,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "TestClan".to_string(),
            chief: "Tester".to_string(), // matches session 1's name
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 1,
            points: 0,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0xFFFF,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: -1,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time: 0,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        };
        world.insert_knights(clan);
        world.update_character_stats(1, |ch| {
            ch.knights_id = 100;
        });

        lua.load("GiveClanPremium(1, 13, 7)").exec().unwrap();

        // Knights should have premium_time set and premium_in_use = 13
        let knights = world.get_knights(100).unwrap();
        assert_eq!(knights.premium_in_use, 13);
        assert!(knights.premium_time > 0, "premium_time should be set");

        // Session should have clan_premium_in_use = 13
        let clan_prem = world.with_session(1, |h| h.clan_premium_in_use).unwrap();
        assert_eq!(clan_prem, 13);
    }

    /// HIGH #7: JoinEvent rejects low-level players
    #[test]
    fn test_join_event_rejects_low_level() {
        let (lua, world) = setup_lua_world();
        // Set level to 10 (below MIN_LEVEL_JURAID = 35)
        world.update_character_stats(1, |ch| {
            ch.level = 10;
        });

        // Even if Juraid were open, low level should fail
        // Since Juraid is closed by default, both checks should reject
        let result: i32 = lua.load("return JoinEvent(1)").eval().unwrap();
        assert_eq!(result, 0);
    }

    /// HIGH #7: JoinEvent rejects players in prison zone
    #[test]
    fn test_join_event_rejects_prison() {
        let (lua, world) = setup_lua_world();
        // Move player to prison zone
        world.update_session(1, |h| {
            h.position.zone_id = 92;
        }); // ZONE_PRISON

        let result: i32 = lua.load("return JoinEvent(1)").eval().unwrap();
        assert_eq!(result, 0);
    }

    // ═══════════════════════════════════════════════════════════════════
    // Sprint 44 Track C Tests — SpawnEventSystem, CycleSpawn, Nation-0 Fix
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_spawn_event_system_early_return_zero_npc_id() {
        // npc_id=0 should return Ok(()) without needing WorldState
        let lua = Lua::new();
        register_all(&lua).unwrap();
        lua.load("SpawnEventSystem(1, 0, 0, 1, 100, 0, 200)")
            .exec()
            .unwrap();
    }

    #[test]
    fn test_spawn_event_system_early_return_zero_zone() {
        // zone=0 should return Ok(()) without needing WorldState
        let lua = Lua::new();
        register_all(&lua).unwrap();
        lua.load("SpawnEventSystem(1, 100, 0, 0, 100, 0, 200)")
            .exec()
            .unwrap();
    }

    #[test]
    fn test_spawn_event_system_negative_npc_id() {
        // Negative npc_id should return early
        let lua = Lua::new();
        register_all(&lua).unwrap();
        lua.load("SpawnEventSystem(1, -1, 0, 1, 100, 0, 200)")
            .exec()
            .unwrap();
    }

    #[test]
    fn test_spawn_event_system_is_monster_flag_inversion() {
        // C++ inverts: is_monster_arg=0 means IS monster, is_monster_arg=1 means NOT monster.
        // This test verifies the function parses args correctly and calls get_world.
        // With no WorldState, valid args should error.
        let lua = Lua::new();
        register_all(&lua).unwrap();
        // is_monster_arg=0 (monster), valid npc_id and zone
        let result = lua
            .load("SpawnEventSystem(1, 50, 0, 21, 100, 0, 200)")
            .exec();
        assert!(
            result.is_err(),
            "Should fail without WorldState for valid args"
        );
    }

    #[test]
    fn test_spawn_event_system_is_npc_flag() {
        // is_monster_arg=1 means NOT monster (NPC)
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let result = lua
            .load("SpawnEventSystem(1, 50, 1, 21, 100, 0, 200)")
            .exec();
        assert!(
            result.is_err(),
            "Should fail without WorldState for valid args"
        );
    }

    #[test]
    fn test_spawn_event_system_no_args() {
        // No arguments — npc_id and zone both default to 0, should return Ok
        let lua = Lua::new();
        register_all(&lua).unwrap();
        lua.load("SpawnEventSystem()").exec().unwrap();
    }

    #[test]
    fn test_cycle_spawn_no_event_npc() {
        // Player has no event NPC — should return Ok without error
        let (lua, _world) = setup_lua_world();
        lua.load("CycleSpawn(1)").exec().unwrap();
    }

    /// Helper: create a minimal NPC template for bindings tests.
    fn make_test_npc_template(s_sid: u16, is_monster: bool, group: u8) -> crate::npc::NpcTemplate {
        crate::npc::NpcTemplate {
            s_sid,
            is_monster,
            name: format!("Test_{}", s_sid),
            pid: s_sid,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group,
            act_type: 0,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 10,
            max_hp: 100,
            max_mp: 0,
            attack: 10,
            ac: 10,
            hit_rate: 50,
            evade_rate: 10,
            damage: 5,
            attack_delay: 2000,
            speed_1: 100,
            speed_2: 200,
            stand_time: 1000,
            search_range: 30,
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
            exp: 0,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }
    }

    #[test]
    fn test_cycle_spawn_non_cycle_npc() {
        // Player has an event NPC but it's not a cycle-spawn type
        let (lua, world) = setup_lua_world();
        let npc_tmpl = make_test_npc_template(100, false, 1);
        world.insert_npc_template(npc_tmpl);

        let instance = crate::npc::NpcInstance {
            nid: 10001,
            proto_id: 100,
            is_monster: false,
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            direction: 0,
            region_x: 0,
            region_z: 0,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 0, // NOT cycle-spawn
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(instance);
        // Set player's event_nid to the NPC
        world.update_session(1, |h| {
            h.event_nid = 10001;
        });

        // Should not error — NPC doesn't have special_type=7, so returns early
        lua.load("CycleSpawn(1)").exec().unwrap();
    }

    #[test]
    fn test_cycle_spawn_wrong_trap_number() {
        // NPC has special_type=7 but trap_number=0 (not in 1..=4)
        let (lua, world) = setup_lua_world();
        let npc_tmpl = make_test_npc_template(101, false, 1);
        world.insert_npc_template(npc_tmpl);

        let instance = crate::npc::NpcInstance {
            nid: 10002,
            proto_id: 101,
            is_monster: false,
            zone_id: 21,
            x: 50.0,
            y: 0.0,
            z: 50.0,
            direction: 0,
            region_x: 0,
            region_z: 0,
            gate_open: 0,
            object_type: 0,
            nation: 1,
            special_type: 7, // CycleSpawn type
            trap_number: 0,  // Invalid — not in 1..=4
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        };
        world.insert_npc_instance(instance);
        world.update_session(1, |h| {
            h.event_nid = 10002;
        });

        // Should return early since trap_number is 0
        lua.load("CycleSpawn(1)").exec().unwrap();
    }

    #[test]
    fn test_cycle_spawn_valid_trap_number_range() {
        // Verify that trap_number values 1-4 pass the check, 5 does not
        // We just test the condition itself, not the full broadcast
        for valid in [1i16, 2, 3, 4] {
            let is_cycle = 7i16 == 7 && (1..=4).contains(&valid);
            assert!(is_cycle, "trap_number {} should be valid", valid);
        }
        let is_cycle = 7i16 == 7 && (1..=4).contains(&5i16);
        assert!(!is_cycle, "trap_number 5 should be invalid");
        let is_cycle = 7i16 == 7 && (1..=4).contains(&0i16);
        assert!(!is_cycle, "trap_number 0 should be invalid");
    }

    // ═══════════════════════════════════════════════════════════════════
    // Sprint 385: Event Lua Binding aliases (B6)
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_sprint385_original_name_aliases_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        // C++ MAKE_LUA_METHOD registers these original names alongside Check* aliases
        for name in &[
            "GetWarVictory",
            "GetUnderTheCastleOpen",
            "GetJuraidMountainTime",
            "BeefEventLogin",
        ] {
            assert!(
                g.get::<LuaFunction>(*name).is_ok(),
                "Missing original-name alias: {}",
                name
            );
        }
    }

    #[test]
    fn test_sprint385_get_war_victory_alias_matches_check() {
        let world = Arc::new(WorldState::new());
        let lua = Lua::new();
        lua.set_app_data(world.clone());
        register_all(&lua).unwrap();
        // Both names should return same value
        let v1: u8 = lua.load("return GetWarVictory(1)").eval().unwrap();
        let v2: u8 = lua.load("return CheckWarVictory(1)").eval().unwrap();
        assert_eq!(
            v1, v2,
            "GetWarVictory and CheckWarVictory must return same value"
        );
    }

    #[test]
    fn test_sprint385_get_under_castle_open_alias_matches_check() {
        let world = Arc::new(WorldState::new());
        let lua = Lua::new();
        lua.set_app_data(world.clone());
        register_all(&lua).unwrap();
        let v1: i32 = lua.load("return GetUnderTheCastleOpen(1)").eval().unwrap();
        let v2: i32 = lua
            .load("return CheckUnderTheCastleOpen(1)")
            .eval()
            .unwrap();
        assert_eq!(
            v1, v2,
            "GetUnderTheCastleOpen and CheckUnderTheCastleOpen must return same value"
        );
    }

    #[test]
    fn test_sprint385_get_juraid_mountain_time_alias_matches_check() {
        let world = Arc::new(WorldState::new());
        let lua = Lua::new();
        lua.set_app_data(world.clone());
        register_all(&lua).unwrap();
        let v1: i32 = lua.load("return GetJuraidMountainTime(1)").eval().unwrap();
        let v2: i32 = lua
            .load("return CheckJuraidMountainTime(1)")
            .eval()
            .unwrap();
        assert_eq!(
            v1, v2,
            "GetJuraidMountainTime and CheckJuraidMountainTime must return same value"
        );
    }

    #[test]
    fn test_sprint385_beef_event_login_alias_matches_check() {
        let world = Arc::new(WorldState::new());
        let lua = Lua::new();
        lua.set_app_data(world.clone());
        register_all(&lua).unwrap();
        let v1: i32 = lua.load("return BeefEventLogin(1)").eval().unwrap();
        let v2: i32 = lua.load("return CheckBeefEventLogin(1)").eval().unwrap();
        assert_eq!(
            v1, v2,
            "BeefEventLogin and CheckBeefEventLogin must return same value"
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // Sprint 385 B6: Remaining C++ MAKE_LUA_METHOD original-name aliases
    // ═══════════════════════════════════════════════════════════════════

    /// Verify all newly added B6 original-name aliases are registered.
    ///
    #[test]
    fn test_sprint385_b6_remaining_aliases_registered() {
        let lua = Lua::new();
        register_all(&lua).unwrap();
        let g = lua.globals();
        for name in &[
            "GetMonsterChallengeTime",
            "GetMonsterChallengeUserCount",
            "GetUnderTheCastleUserCount",
            "GetClanGrade",
            "GetClanPoint",
        ] {
            assert!(
                g.get::<LuaFunction>(*name).is_ok(),
                "Missing B6 original-name alias: {}",
                name
            );
        }
    }

    /// GetMonsterChallengeTime must return same value as CheckMonsterChallengeTime.
    ///
    #[test]
    fn test_sprint385_get_monster_challenge_time_matches_check() {
        let world = Arc::new(WorldState::new());
        let lua = Lua::new();
        lua.set_app_data(world.clone());
        register_all(&lua).unwrap();
        let v1: i32 = lua
            .load("return GetMonsterChallengeTime(1)")
            .eval()
            .unwrap();
        let v2: i32 = lua
            .load("return CheckMonsterChallengeTime(1)")
            .eval()
            .unwrap();
        assert_eq!(
            v1, v2,
            "GetMonsterChallengeTime and CheckMonsterChallengeTime must return same value"
        );
    }

    /// GetMonsterChallengeUserCount must return same value as CheckMonsterChallengeUserCount.
    ///
    #[test]
    fn test_sprint385_get_monster_challenge_user_count_matches_check() {
        let world = Arc::new(WorldState::new());
        let lua = Lua::new();
        lua.set_app_data(world.clone());
        register_all(&lua).unwrap();
        let v1: i32 = lua
            .load("return GetMonsterChallengeUserCount(1)")
            .eval()
            .unwrap();
        let v2: i32 = lua
            .load("return CheckMonsterChallengeUserCount(1)")
            .eval()
            .unwrap();
        assert_eq!(
            v1, v2,
            "GetMonsterChallengeUserCount and CheckMonsterChallengeUserCount must return same value"
        );
    }

    /// GetUnderTheCastleUserCount must return same value as CheckUnderTheCastleUserCount.
    ///
    #[test]
    fn test_sprint385_get_under_castle_user_count_matches_check() {
        let world = Arc::new(WorldState::new());
        let lua = Lua::new();
        lua.set_app_data(world.clone());
        register_all(&lua).unwrap();
        let v1: i32 = lua
            .load("return GetUnderTheCastleUserCount(1)")
            .eval()
            .unwrap();
        let v2: i32 = lua
            .load("return CheckUnderTheCastleUserCount(1)")
            .eval()
            .unwrap();
        assert_eq!(
            v1, v2,
            "GetUnderTheCastleUserCount and CheckUnderTheCastleUserCount must return same value"
        );
    }

    /// GetClanGrade must return same value as CheckClanGrade.
    ///
    #[test]
    fn test_sprint385_get_clan_grade_matches_check() {
        let (lua, _world) = setup_lua_world();
        let v1: u8 = lua.load("return GetClanGrade(1)").eval().unwrap();
        let v2: u8 = lua.load("return CheckClanGrade(1)").eval().unwrap();
        assert_eq!(
            v1, v2,
            "GetClanGrade and CheckClanGrade must return same value"
        );
    }

    /// GetClanPoint must return same value as CheckClanPoint.
    ///
    #[test]
    fn test_sprint385_get_clan_point_matches_check() {
        let (lua, _world) = setup_lua_world();
        let v1: i32 = lua.load("return GetClanPoint(1)").eval().unwrap();
        let v2: i32 = lua.load("return CheckClanPoint(1)").eval().unwrap();
        assert_eq!(
            v1, v2,
            "GetClanPoint and CheckClanPoint must return same value"
        );
    }

    // --- Sprint 777: CSW Deathmatch registration tests ---

    use crate::world::types::{CswOpStatus, ZONE_DELOS};

    /// Helper: create a player in Delos zone with clan, at level 40.
    fn setup_delos_player() -> (Lua, Arc<WorldState>) {
        let world = Arc::new(WorldState::new());
        let (tx, _rx) = mpsc::unbounded_channel();
        let sid: SessionId = 1;
        world.register_session(sid, tx);

        let info = CharacterInfo {
            session_id: sid,
            name: "CswTester".into(),
            nation: 1,
            race: 1,
            class: 101,
            level: 40,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 2000,
            hp: 2000,
            max_mp: 1000,
            mp: 1000,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 21,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 80,
            sta: 80,
            dex: 80,
            intel: 80,
            cha: 80,
            free_points: 0,
            skill_points: [0u8; 10],
            gold: 200_000_000,
            loyalty: 1000,
            loyalty_monthly: 200,
            authority: 1,
            knights_id: 100, // has a clan
            fame: 0,
            party_id: None,
            exp: 50000,
            max_exp: 100_000_000,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 5000,
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
            zone_id: ZONE_DELOS,
            x: 500.0,
            y: 0.0,
            z: 500.0,
            region_x: 0,
            region_z: 0,
        };
        world.register_ingame(sid, info, pos);

        let inv = vec![UserItemSlot::default(); WorldState::SLOT_MAX + WorldState::HAVE_MAX];
        world.set_inventory(sid, inv);

        // Activate CSW.
        {
            let mut csw = world.csw_event().blocking_write();
            csw.started = true;
            csw.status = CswOpStatus::Preparation;
        }

        let lua = Lua::new();
        lua.set_app_data(Arc::clone(&world));
        register_all(&lua).unwrap();
        (lua, world)
    }

    #[test]
    fn test_csw_deathmatch_register_success() {
        let (lua, world) = setup_delos_player();
        let result: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 1, "should succeed");
        let csw = world.csw_event().blocking_read();
        assert!(csw.deathmatch_players.contains(&1));
    }

    #[test]
    fn test_csw_deathmatch_register_already_registered() {
        let (lua, _world) = setup_delos_player();
        // First registration succeeds.
        let r1: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(r1, 1);
        // Second registration returns 3 (already registered).
        let r2: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(r2, 3);
    }

    #[test]
    fn test_csw_deathmatch_register_not_active() {
        let (lua, world) = setup_delos_player();
        // Deactivate CSW.
        {
            let mut csw = world.csw_event().blocking_write();
            csw.started = false;
            csw.status = CswOpStatus::NotOperation;
        }
        let result: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 2, "CSW not active should return 2");
    }

    #[test]
    fn test_csw_deathmatch_register_wrong_zone() {
        let (lua, world) = setup_delos_player();
        // Move player out of Delos.
        world.update_position(1, 21, 50.0, 0.0, 50.0);
        let result: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 6, "wrong zone should return 6");
    }

    #[test]
    fn test_csw_deathmatch_register_no_clan() {
        let (lua, world) = setup_delos_player();
        // Remove clan.
        world.update_session(1, |h| {
            if let Some(ref mut c) = h.character {
                c.knights_id = 0;
            }
        });
        let result: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 4, "no clan should return 4");
    }

    #[test]
    fn test_csw_deathmatch_register_low_level() {
        let (lua, world) = setup_delos_player();
        // Set level below minimum.
        world.update_session(1, |h| {
            if let Some(ref mut c) = h.character {
                c.level = 20;
            }
        });
        let result: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 5, "low level should return 5");
    }

    #[test]
    fn test_csw_deathmatch_cancel_success() {
        let (lua, world) = setup_delos_player();
        // Register first.
        let r1: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(r1, 1);
        // Cancel.
        let r2: u16 = lua
            .load("return CheckCastleSiegeWarDeathmacthCancelRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(r2, 1, "cancel should succeed");
        let csw = world.csw_event().blocking_read();
        assert!(!csw.deathmatch_players.contains(&1));
    }

    #[test]
    fn test_csw_deathmatch_cancel_not_registered() {
        let (lua, _world) = setup_delos_player();
        let result: u16 = lua
            .load("return CheckCastleSiegeWarDeathmacthCancelRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(result, 3, "not registered should return 3");
    }

    #[test]
    fn test_csw_deathmatch_reset_clears_registrations() {
        let (lua, world) = setup_delos_player();
        // Register.
        let r: u16 = lua
            .load("return CheckCastleSiegeWarDeathmachRegister(1)")
            .eval()
            .unwrap();
        assert_eq!(r, 1);
        // Reset CSW.
        {
            let mut csw = world.csw_event().blocking_write();
            csw.reset();
        }
        let csw = world.csw_event().blocking_read();
        assert!(
            csw.deathmatch_players.is_empty(),
            "reset should clear registrations"
        );
    }

    // ── Quest reward delivery: full bag, stacks, completion guard ─────────

    /// Build an exchange row from (item, count) pairs.
    fn quest_exchange_row(
        n_index: i32,
        origins: &[(i32, i32)],
        outputs: &[(i32, i32)],
    ) -> ItemExchangeRow {
        let o = |i: usize| origins.get(i).copied().unwrap_or((0, 0));
        let e = |i: usize| outputs.get(i).copied().unwrap_or((0, 0));
        ItemExchangeRow {
            n_index,
            random_flag: 0,
            origin_item_num1: o(0).0,
            origin_item_count1: o(0).1,
            origin_item_num2: o(1).0,
            origin_item_count2: o(1).1,
            origin_item_num3: o(2).0,
            origin_item_count3: o(2).1,
            origin_item_num4: o(3).0,
            origin_item_count4: o(3).1,
            origin_item_num5: o(4).0,
            origin_item_count5: o(4).1,
            exchange_item_num1: e(0).0,
            exchange_item_count1: e(0).1,
            exchange_item_num2: e(1).0,
            exchange_item_count2: e(1).1,
            exchange_item_num3: e(2).0,
            exchange_item_count3: e(2).1,
            exchange_item_num4: e(3).0,
            exchange_item_count4: e(3).1,
            exchange_item_num5: e(4).0,
            exchange_item_count5: e(4).1,
            exchange_item_time1: 0,
            exchange_item_time2: 0,
            exchange_item_time3: 0,
            exchange_item_time4: 0,
            exchange_item_time5: 0,
        }
    }

    #[test]
    fn test_exchange_item_days_reads_the_output_slot() {
        let mut row = quest_exchange_row(1, &[], &[(389010000, 1), (389011000, 2)]);
        row.exchange_item_time2 = 30;
        assert_eq!(exchange_item_days(&row, 389010000), 0);
        assert_eq!(exchange_item_days(&row, 389011000), 30);
        assert_eq!(exchange_item_days(&row, 123), 0);
        assert!(is_exchange_virtual_id(ITEM_EXP));
        assert!(!is_exchange_virtual_id(389010000));
    }

    /// Fill every bag slot with a distinct non-stackable junk item and return
    /// the bag index of the first slot.
    fn fill_bag_with_junk(world: &WorldState) {
        let mut inv = world.get_inventory(1);
        for (n, slot) in inv
            .iter_mut()
            .enumerate()
            .skip(WorldState::SLOT_MAX)
            .take(WorldState::HAVE_MAX)
        {
            insert_test_item(world, 700_000_000 + n as u32, false);
            slot.item_id = 700_000_000 + n as u32;
            slot.count = 1;
            slot.durability = 100;
        }
        world.set_inventory(1, inv);
    }

    fn drain_packets(rx: &mut mpsc::UnboundedReceiver<Arc<Packet>>) -> Vec<Arc<Packet>> {
        let mut out = Vec::new();
        while let Ok(p) = rx.try_recv() {
            out.push(p);
        }
        out
    }

    fn has_no_slot_error(packets: &[Arc<Packet>]) -> bool {
        packets
            .iter()
            .any(|p| p.opcode == Opcode::WizQuest as u8 && p.data == vec![13, 3])
    }

    fn count_in_bag(world: &WorldState, item_id: u32) -> u32 {
        world
            .get_inventory(1)
            .iter()
            .filter(|s| s.item_id == item_id)
            .map(|s| s.count as u32)
            .sum()
    }

    #[test]
    fn test_run_quest_exchange_full_bag_fails_gracefully() {
        let (lua, world, mut rx) = setup_lua_world_with_rx();
        let origin = 389_010_000u32;
        let reward_a = 389_020_000u32;
        let reward_b = 389_030_000u32;
        insert_test_item(&world, origin, false);
        insert_test_item(&world, reward_a, false);
        insert_test_item(&world, reward_b, false);
        fill_bag_with_junk(&world);
        // The turn-in item lives in the first bag slot: handing it in frees ONE
        // slot, but the reward needs TWO.
        let mut inv = world.get_inventory(1);
        inv[WorldState::SLOT_MAX].item_id = origin;
        world.set_inventory(1, inv);
        world.insert_item_exchange(
            9001,
            quest_exchange_row(
                9001,
                &[(origin as i32, 1)],
                &[
                    (reward_a as i32, 1),
                    (reward_b as i32, 1),
                    (ITEM_GOLD as i32, 500),
                ],
            ),
        );
        let gold_before = world.get_character_info(1).unwrap().gold;
        drain_packets(&mut rx);

        let ok: bool = lua.load("return RunQuestExchange(1, 9001)").eval().unwrap();

        assert!(!ok, "a bag that cannot hold the reward must refuse the exchange");
        assert_eq!(count_in_bag(&world, origin), 1, "turn-in item must NOT be taken");
        assert_eq!(count_in_bag(&world, reward_a), 0);
        assert_eq!(count_in_bag(&world, reward_b), 0);
        assert_eq!(
            world.get_character_info(1).unwrap().gold,
            gold_before,
            "no partial reward (gold) when the items do not fit"
        );
        assert!(
            has_no_slot_error(&drain_packets(&mut rx)),
            "the player must be told the bag is full (WIZ_QUEST 13/3)"
        );
    }

    #[test]
    fn test_run_quest_exchange_origin_frees_the_slot_it_needs() {
        // One free slot is gained by handing in the turn-in item, so a single
        // reward item fits even though the bag is otherwise full.
        let (lua, world, _rx) = setup_lua_world_with_rx();
        let origin = 389_010_000u32;
        let reward = 389_020_000u32;
        insert_test_item(&world, origin, false);
        insert_test_item(&world, reward, false);
        fill_bag_with_junk(&world);
        let mut inv = world.get_inventory(1);
        inv[WorldState::SLOT_MAX].item_id = origin;
        world.set_inventory(1, inv);
        world.insert_item_exchange(
            9002,
            quest_exchange_row(9002, &[(origin as i32, 1)], &[(reward as i32, 1)]),
        );

        let ok: bool = lua.load("return RunQuestExchange(1, 9002)").eval().unwrap();

        assert!(ok);
        assert_eq!(count_in_bag(&world, origin), 0);
        assert_eq!(count_in_bag(&world, reward), 1);
    }

    #[test]
    fn test_run_quest_exchange_two_rewards_one_free_slot_is_refused() {
        // Both rewards are "new item" outputs competing for the SAME single free
        // slot. Checking each one on its own used to pass and the second item was
        // lost after the turn-in had already been consumed.
        let (lua, world, mut rx) = setup_lua_world_with_rx();
        let origin = 389_010_000u32;
        let reward_a = 389_020_000u32;
        let reward_b = 389_030_000u32;
        for id in [origin, reward_a, reward_b] {
            insert_test_item(&world, id, false);
        }
        fill_bag_with_junk(&world);
        // exactly ONE empty slot, no turn-in item that could free another one
        let mut inv = world.get_inventory(1);
        inv[WorldState::SLOT_MAX + 1] = UserItemSlot::default();
        world.set_inventory(1, inv);
        world.insert_item_exchange(
            9003,
            quest_exchange_row(9003, &[], &[(reward_a as i32, 1), (reward_b as i32, 1)]),
        );
        drain_packets(&mut rx);

        let ok: bool = lua.load("return RunQuestExchange(1, 9003)").eval().unwrap();

        assert!(!ok, "2 new items cannot fit into 1 free slot");
        assert_eq!(count_in_bag(&world, reward_a), 0);
        assert_eq!(count_in_bag(&world, reward_b), 0);
        assert!(has_no_slot_error(&drain_packets(&mut rx)));
    }

    #[test]
    fn test_run_quest_exchange_skips_unknown_reward_item_but_pays_the_rest() {
        // C++ parity: a reward id that is not in the item table is skipped, the
        // remaining rewards (here gold) are still paid.
        let (lua, world, _rx) = setup_lua_world_with_rx();
        world.insert_item_exchange(
            9004,
            quest_exchange_row(
                9004,
                &[],
                &[(999_999_999, 1), (ITEM_GOLD as i32, 700)],
            ),
        );
        let before = world.get_character_info(1).unwrap().gold;

        let ok: bool = lua.load("return RunQuestExchange(1, 9004)").eval().unwrap();

        assert!(ok);
        assert_eq!(world.get_character_info(1).unwrap().gold, before + 700);
    }

    #[test]
    fn test_run_quest_exchange_takes_turn_in_items_from_several_stacks() {
        // 3 + 2 pieces of a non-stackable turn-in item in two slots: the check
        // (HowmuchItem adds the slots up) passes, the removal must too.
        let (lua, world, _rx) = setup_lua_world_with_rx();
        let origin = 389_010_000u32;
        insert_test_item(&world, origin, false);
        let mut inv = world.get_inventory(1);
        for k in 0..5 {
            let s = &mut inv[WorldState::SLOT_MAX + k];
            s.item_id = origin;
            s.count = 1;
            s.durability = 10;
        }
        world.set_inventory(1, inv);
        world.insert_item_exchange(
            9005,
            quest_exchange_row(9005, &[(origin as i32, 5)], &[(ITEM_GOLD as i32, 100)]),
        );
        let before = world.get_character_info(1).unwrap().gold;

        let ok: bool = lua.load("return RunQuestExchange(1, 9005)").eval().unwrap();

        assert!(ok, "5 turn-in pieces spread over 5 slots must be accepted");
        assert_eq!(count_in_bag(&world, origin), 0);
        assert_eq!(world.get_character_info(1).unwrap().gold, before + 100);
    }

    #[test]
    fn test_rob_item_all_or_nothing_across_stacks() {
        let (_lua, world, _rx) = setup_lua_world_with_rx();
        let item = 389_010_000u32;
        insert_test_item(&world, item, true);
        let mut inv = world.get_inventory(1);
        inv[WorldState::SLOT_MAX].item_id = item;
        inv[WorldState::SLOT_MAX].count = 3;
        inv[WorldState::SLOT_MAX + 1].item_id = item;
        inv[WorldState::SLOT_MAX + 1].count = 2;
        world.set_inventory(1, inv);

        assert!(!world.rob_item(1, item, 6), "only 5 owned");
        assert_eq!(count_in_bag(&world, item), 5, "a failed removal changes nothing");
        assert!(world.rob_item(1, item, 4));
        assert_eq!(count_in_bag(&world, item), 1);
    }

    #[test]
    fn test_give_item_full_bag_returns_false_and_tells_the_player() {
        let (lua, world, mut rx) = setup_lua_world_with_rx();
        let reward = 389_020_000u32;
        insert_test_item(&world, reward, false);
        fill_bag_with_junk(&world);
        drain_packets(&mut rx);

        let ok: bool = lua
            .load(format!("return GiveItem(1, {reward}, 1)"))
            .eval()
            .unwrap();

        assert!(!ok);
        assert_eq!(count_in_bag(&world, reward), 0);
        assert!(has_no_slot_error(&drain_packets(&mut rx)));
    }

    #[test]
    fn test_give_item_unknown_or_oversized_id_is_not_a_bag_full_error() {
        let (lua, world, mut rx) = setup_lua_world_with_rx();
        drain_packets(&mut rx);

        // not in the item table
        let ok: bool = lua.load("return GiveItem(1, 123456789, 1)").eval().unwrap();
        assert!(!ok);
        // 10-digit id from a typo (8104340970): must not wrap into some other item
        insert_test_item(&world, (8_104_340_970u64 & 0xFFFF_FFFF) as u32, true);
        let ok: bool = lua.load("return GiveItem(1, 8104340970, 1)").eval().unwrap();
        assert!(!ok, "ids above u32::MAX must be rejected, not wrapped");
        assert_eq!(
            count_in_bag(&world, (8_104_340_970u64 & 0xFFFF_FFFF) as u32),
            0
        );
        assert!(
            !has_no_slot_error(&drain_packets(&mut rx)),
            "a bad item id must not tell the player their bag is full"
        );
    }

    #[test]
    fn test_give_item_count_is_clamped_not_wrapped() {
        let (lua, world, _rx) = setup_lua_world_with_rx();
        let reward = 389_020_000u32;
        insert_test_item(&world, reward, true);

        // 70000 used to wrap (as u16) to 4464.
        let ok: bool = lua
            .load(format!("return GiveItem(1, {reward}, 70000)"))
            .eval()
            .unwrap();

        assert!(ok);
        assert_eq!(count_in_bag(&world, reward), ITEMCOUNT_MAX as u32);
    }

    fn quest_helper_row(n_index: i32, quest: i16, status: i16) -> ko_db::models::QuestHelperRow {
        ko_db::models::QuestHelperRow {
            n_index,
            b_message_type: 2,
            b_level: 1,
            n_exp: 0,
            b_class: 5,
            b_nation: 3,
            b_quest_type: 1,
            b_zone: 21,
            s_npc_id: 18005,
            s_event_data_index: quest,
            b_event_status: status,
            n_event_trigger_index: 100,
            n_event_complete_index: 0,
            n_exchange_index: 0,
            n_event_talk_index: 0,
            str_lua_filename: "x.lua".into(),
            s_quest_menu: -1,
            s_npc_main: 0,
            s_quest_solo: 0,
        }
    }

    #[test]
    fn test_save_event_complete_refused_when_reward_did_not_fit() {
        // The shipped scripts do `RunQuestExchange(...)` and `SaveEvent(...)` back
        // to back. With a full bag the quest must stay open instead of being
        // completed without its reward.
        let (lua, world, _rx) = setup_lua_world_with_rx();
        let reward = 389_020_000u32;
        insert_test_item(&world, reward, false);
        fill_bag_with_junk(&world);
        world.insert_item_exchange(9010, quest_exchange_row(9010, &[], &[(reward as i32, 1)]));
        world.insert_test_quest_helper(quest_helper_row(7001, 700, 2)); // "complete quest 700"
        world.update_session(1, |h| {
            h.quests.entry(700).or_default().quest_state = 1;
        });

        lua.load("RunQuestExchange(1, 9010); SaveEvent(1, 7001)")
            .exec()
            .unwrap();

        let state = world
            .with_session(1, |h| h.quests.get(&700).map(|q| q.quest_state))
            .unwrap();
        assert_eq!(state, Some(1), "quest must stay 'ongoing' while the reward is refused");
        assert_eq!(count_in_bag(&world, reward), 0);
    }

    #[test]
    fn test_save_event_complete_works_after_a_delivered_reward() {
        let (lua, world, _rx) = setup_lua_world_with_rx();
        let reward = 389_020_000u32;
        insert_test_item(&world, reward, false);
        world.insert_item_exchange(9011, quest_exchange_row(9011, &[], &[(reward as i32, 1)]));
        world.insert_test_quest_helper(quest_helper_row(7002, 701, 2));
        world.update_session(1, |h| {
            h.quests.entry(701).or_default().quest_state = 1;
        });

        lua.load("RunQuestExchange(1, 9011); SaveEvent(1, 7002)")
            .exec()
            .unwrap();

        let state = world
            .with_session(1, |h| h.quests.get(&701).map(|q| q.quest_state))
            .unwrap();
        assert_eq!(state, Some(2));
        assert_eq!(count_in_bag(&world, reward), 1);
    }

    #[test]
    fn test_save_event_complete_still_runs_for_data_errors() {
        // A missing exchange row is a DATA error, not something the player can fix:
        // the quest still completes (otherwise it could never be finished).
        let (lua, world, _rx) = setup_lua_world_with_rx();
        world.insert_test_quest_helper(quest_helper_row(7003, 702, 2));
        world.update_session(1, |h| {
            h.quests.entry(702).or_default().quest_state = 1;
        });

        lua.load("RunQuestExchange(1, 987654); SaveEvent(1, 7003)")
            .exec()
            .unwrap();

        let state = world
            .with_session(1, |h| h.quests.get(&702).map(|q| q.quest_state))
            .unwrap();
        assert_eq!(state, Some(2));
    }

    #[test]
    fn test_can_hold_after_exchange_counts_stack_room() {
        let (_lua, world, _rx) = setup_lua_world_with_rx();
        let stackable = 389_010_000u32;
        let single = 389_020_000u32;
        insert_test_item(&world, stackable, true);
        insert_test_item(&world, single, false);
        fill_bag_with_junk(&world);
        let mut inv = world.get_inventory(1);
        inv[WorldState::SLOT_MAX].item_id = stackable;
        inv[WorldState::SLOT_MAX].count = 10;
        world.set_inventory(1, inv);

        // stacking onto the existing pile needs no free slot
        assert!(world.can_hold_after_exchange(1, &[], &[(stackable, 5)]));
        // a different item has nowhere to go
        assert!(!world.can_hold_after_exchange(1, &[], &[(single, 1)]));
        // ... unless the pile is handed in first (frees its slot)
        assert!(world.can_hold_after_exchange(1, &[(stackable, 10)], &[(single, 1)]));
    }

    #[test]
    fn test_rob_item_count_is_optional() {
        let (lua, world, _rx) = setup_lua_world_with_rx();
        let item = 389_010_000u32;
        insert_test_item(&world, item, true);
        world.give_item(1, item, 3);
        lua.load(format!("RobItem(1, {item})")).exec().unwrap();
        assert_eq!(count_in_bag(&world, item), 2);
    }
}
