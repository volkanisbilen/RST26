//! Shared world state â€” session registry, position tracking, and broadcasting.
//! All sessions share a single `WorldState` via `Arc`. Handlers access it
//! through `session.world()` to broadcast packets or query nearby players.

pub mod box_breaker;
pub mod combat;
pub mod inventory;
mod loading;
pub mod moraranker;
pub mod npc;
pub mod session;
pub mod social;
pub mod tables;
pub mod trade;
pub mod zone;

fn initial_item_serial() -> u64 {
    // Seed the process-local counter from Unix microseconds so item serials do
    // not restart at 1 and collide with persisted inventory after a restart.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
        .min(i64::MAX as u128) as u64
}

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use tokio::sync::mpsc;

use ko_db::models::{
    AchieveComRow, AchieveMainRow, AchieveMonsterRow, AchieveNormalRow, AchieveTitleRow,
    AchieveWarRow, BotHandlerFarmRow, BotHandlerMerchantRow, BotKnightsRankRow, BotMerchantDataRow,
    BotPersonalRankRow, ChaosStoneSpawnRow, ChaosStoneSummonListRow, ChaosStoneSummonStageRow,
    CindwarItemRow, CindwarRewardItemRow, CindwarRewardRow, CindwarSettingRow, CindwarStatRow,
    CoefficientRow, CreateNewCharSetRow, CreateNewCharValueRow, DailyQuestRow, DamageSettingsRow,
    DfMonsterRow, DfStageRow, DrakiMonsterListRow, DrakiTowerStageRow, EventChaosRewardRow,
    EventRewardRow, FtStageRow, FtSummonRow, GameEventRow, HomeRow, Item, ItemExchangeExpRow,
    ItemExchangeRow, ItemGiveExchangeRow, ItemGroupRow, ItemOpRow, ItemRandomRow,
    ItemRightClickExchangeRow, ItemRightExchangeRow, ItemSellTableRow, ItemSmashRow,
    ItemSpecialSewingRow, ItemUpProbabilityRow, ItemUpgradeRow, ItemUpgradeSettingsRow,
    KnightsCapeCastellanBonusRow, KnightsCapeRow, KnightsCswOptRow, MagicRow, MagicType1Row,
    MagicType2Row, MagicType3Row, MagicType4Row, MagicType5Row, MagicType6Row, MagicType7Row,
    MagicType8Row, MagicType9Row, MakeDefensiveRow, MakeItemGradeCodeRow, MakeItemGroupRandomRow,
    MakeItemGroupRow, MakeItemLareCodeRow, MakeItemRow, MakeWeaponRow, MiningExchangeRow,
    MiningFishingItemRow, MonsterBossRandomSpawnRow, MonsterBossRandomStageRow,
    MonsterChallengeRow, MonsterChallengeSummonRow, MonsterItemRow, MonsterJuraidRespawnRow,
    MonsterRespawnLoopRow, MonsterStoneRespawnRow, MonsterSummonRow, MonsterUnderTheCastleRow,
    NewUpgradeRow, NpcItemRow, PerkRow, PetImageChangeRow, PetStatsInfoRow, PremiumItemExpRow,
    PremiumItemRow, PusCategoryRow, PusItemRow, QuestHelperRow, QuestMenuRow, QuestMonsterRow,
    QuestSkillsClosedCheckRow, QuestSkillsOpenSetUpRow, QuestTalkRow, RentalItemRow,
    ServerSettingsRow, SetItemRow, SpecialStoneRow, StartPositionRandomRow, StartPositionRow,
    UserBotRow, ZoneInfoRow, ZoneKillReward, ZoneOnlineReward,
};
use ko_db::DbPool;
use ko_protocol::{Opcode, Packet};

use crate::handler::draki_tower::DrakiTowerRoomInfo;
use crate::handler::forgotten_temple::ForgettenTempleState;
use crate::handler::sheriff::SheriffReportMap;
use crate::lua_engine::LuaEngine;
use crate::systems::bdw::BdwManager;
use crate::systems::event_room::EventRoomManager;
use crate::systems::juraid::JuraidBridgeState;
use crate::systems::monster_stone::MonsterStoneManager;
use crate::systems::time_weather::GameTimeWeather;

use crate::npc::{NpcId, NpcInstance, NpcTemplate, NPC_BAND};
use crate::zone::{
    calc_region, GameEvent, GameEventType, SessionId, ZoneAbilities, ZoneAbilityType, ZoneInfo,
    ZoneState,
};

pub mod types;
pub use types::*;

/// Shared world state â€” the central coordinator for all sessions.
pub struct WorldState {
    /// Active session registry: SessionId -> SessionHandle.
    sessions: DashMap<SessionId, SessionHandle>,
    /// Per-zone session index for O(1) zone-level lookups.
    zone_session_index: DashMap<u16, parking_lot::RwLock<HashSet<SessionId>>>,
    /// Character name → SessionId index for O(1) name lookups.
    /// Keys are lowercase. Updated on register_ingame/unregister_session/name_change.
    name_to_session: DashMap<String, SessionId>,
    /// Number of in-game sessions (with character loaded). Atomic for O(1) reads.
    online_count: AtomicU32,
    /// Per-zone state with region grids.
    zones: DashMap<u16, Arc<ZoneState>>,
    /// Atomic counter for assigning unique session IDs.
    next_session_id: AtomicU16,
    /// NPC/Monster templates keyed by (s_sid, is_monster).
    npc_templates: DashMap<(u16, bool), Arc<NpcTemplate>>,
    /// Runtime NPC instances keyed by NpcId.
    npc_instances: DashMap<NpcId, Arc<NpcInstance>>,
    /// Atomic counter for assigning unique NPC runtime IDs (starts at NPC_BAND).
    next_npc_id: AtomicU32,
    /// Class coefficient table keyed by s_class (101-115 Karus, 201-215 El Morad).
    ///
    coefficients: DashMap<u16, CoefficientRow>,

    // â”€â”€ Magic / Skill Tables â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Master magic table keyed by `magic_num`.
    ///
    magic_table: DashMap<i32, MagicRow>,
    /// Type 1 (melee) sub-table keyed by `i_num`.
    magic_type1: DashMap<i32, MagicType1Row>,
    /// Type 2 (ranged) sub-table keyed by `i_num`.
    magic_type2: DashMap<i32, MagicType2Row>,
    /// Type 3 (DOT/direct) sub-table keyed by `i_num`.
    magic_type3: DashMap<i32, MagicType3Row>,
    /// Type 4 (buff/debuff) sub-table keyed by `i_num`.
    magic_type4: DashMap<i32, MagicType4Row>,
    /// Type 5 (resurrection) sub-table keyed by `i_num`.
    magic_type5: DashMap<i32, MagicType5Row>,
    /// Type 6 (transform) sub-table keyed by `i_num`.
    magic_type6: DashMap<i32, MagicType6Row>,
    /// Type 7 (summon/CC) sub-table keyed by `n_index`.
    magic_type7: DashMap<i32, MagicType7Row>,
    /// Type 8 (teleport) sub-table keyed by `i_num`.
    magic_type8: DashMap<i32, MagicType8Row>,
    /// Type 9 (advanced CC) sub-table keyed by `i_num`.
    magic_type9: DashMap<i32, MagicType9Row>,

    /// Runtime NPC HP tracking (separate from immutable NpcInstance).
    ///
    /// static template data. Keyed by NpcId (runtime ID >= NPC_BAND).
    npc_hp: DashMap<NpcId, i32>,

    /// Per-NPC damage accumulator: tracks total damage dealt by each player.
    ///
    /// most total damage gets loot rights (not the last-hitter).
    /// Outer key = NpcId, inner key = SessionId, value = cumulative damage.
    npc_damage: DashMap<NpcId, DashMap<SessionId, i32>>,

    /// Runtime NPC AI state (mutable per-NPC: position, state, target).
    ///
    /// Only populated for NPCs that participate in AI (monsters with
    /// `search_range > 0` and `act_type >= 1`).
    npc_ai: DashMap<NpcId, NpcAiState>,

    // â”€â”€ Item Table â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Static item definitions keyed by item num.
    ///
    items: DashMap<u32, Item>,

    // â”€â”€ Party System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Runtime party groups keyed by party ID.
    ///
    parties: DashMap<u16, Party>,
    /// Atomic counter for assigning unique party IDs.
    ///
    next_party_id: AtomicU16,
    /// Pending party invitations: invitee_sid -> (party_id, inviter_sid).
    ///
    /// Tracks players who have been sent a PARTY_PERMIT invitation
    /// and are waiting to accept/decline.
    party_invitations: DashMap<SessionId, (u16, SessionId)>,

    // â”€â”€ Knights (Clan) System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Runtime knights (clan) data keyed by clan ID.
    ///
    knights: DashMap<u16, KnightsInfo>,
    /// Runtime alliance data keyed by main alliance clan ID.
    ///
    alliances: DashMap<u16, KnightsAlliance>,

    // â”€â”€ Ground Bundle System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Ground item bundles keyed by bundle ID.
    ///
    ground_bundles: DashMap<u32, GroundBundle>,
    /// Atomic counter for assigning unique bundle IDs.
    next_bundle_id: AtomicU32,
    /// Atomic counter for generating unique item serial numbers.
    ///
    next_item_serial: std::sync::atomic::AtomicU64,

    // â”€â”€ Quest System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Quest helper definitions keyed by nIndex.
    ///
    quest_helpers: DashMap<u32, QuestHelperRow>,
    collection_drop_items: DashMap<u32, ()>,
    /// Quest monster kill requirements keyed by sQuestNum (sEventDataIndex).
    ///
    quest_monsters: DashMap<u16, QuestMonsterRow>,
    /// NPC â†’ quest helper list for fast NPC-based lookup.
    ///
    quest_npc_list: DashMap<u16, Vec<u32>>,
    /// Quest menu options keyed by iNum.
    ///
    quest_menus: DashMap<i32, QuestMenuRow>,
    /// Quest talk text keyed by iNum.
    ///
    quest_talks: DashMap<i32, QuestTalkRow>,
    /// Quest skill closed check entries keyed by nIndex.
    quest_skills_closed_check: DashMap<i32, QuestSkillsClosedCheckRow>,
    /// Quest skill open setup entries keyed by nIndex.
    quest_skills_open_set_up: DashMap<i32, QuestSkillsOpenSetUpRow>,

    // â”€â”€ Ranking System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// PK zone ranking arrays per nation (index 0=Karus, 1=El Morad).
    ///
    pk_zone_rankings: [DashMap<SessionId, PkZoneRanking>; 2],
    /// Special event (Zindan War) zone ranking arrays per nation.
    ///
    zindan_rankings: [DashMap<SessionId, PkZoneRanking>; 2],
    /// Border Defence War ranking arrays per nation.
    ///
    bdw_rankings: [DashMap<SessionId, BdwRanking>; 2],
    /// Chaos Expansion ranking (all players, keyed by session ID).
    ///
    chaos_rankings: DashMap<SessionId, ChaosRanking>,
    /// Whether a ranking update/reset is currently in progress.
    ///
    ranking_update_in_progress: std::sync::atomic::AtomicBool,

    // â”€â”€ Level-Up Experience Table â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Required XP per level, keyed by (level, rebirth_level).
    ///
    level_up_table: DashMap<(u8, u8), i64>,

    // â”€â”€ King System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Per-nation king system data (index 0=Karus, 1=Elmorad).
    ///
    king_systems: DashMap<u8, KingSystem>,

    // â”€â”€ Item Upgrade Tables â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// New upgrade recipes keyed by origin_number â†’ Vec of possible upgrades.
    ///
    upgrade_recipes: DashMap<i32, Vec<NewUpgradeRow>>,
    /// Item upgrade settings (success rates, required materials, costs).
    ///
    upgrade_settings: DashMap<u32, ItemUpgradeSettingsRow>,
    /// Item upgrade probability configuration.
    ///
    itemup_probability: parking_lot::RwLock<Option<ItemUpProbabilityRow>>,

    // â”€â”€ Item Reference Tables (Extended) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Item special effects keyed by item_id â†’ Vec of effects.
    ///
    item_ops: DashMap<i32, Vec<ItemOpRow>>,
    /// Set item bonuses keyed by set_index.
    ///
    set_items: DashMap<i32, SetItemRow>,
    /// Monster drop tables keyed by s_index.
    ///
    monster_items: DashMap<i16, MonsterItemRow>,
    /// NPC drop tables keyed by s_index.
    ///
    npc_items: DashMap<i16, NpcItemRow>,
    /// Item exchange/crafting recipes keyed by n_index.
    ///
    item_exchanges: DashMap<i32, ItemExchangeRow>,
    /// Item upgrade recipes (NPC-based) keyed by n_index.
    ///
    item_upgrades: DashMap<i32, ItemUpgradeRow>,
    /// Weapon crafting templates keyed by by_level.
    ///
    make_weapons: DashMap<i16, MakeWeaponRow>,
    /// Defensive crafting templates keyed by by_level.
    ///
    make_defensives: DashMap<i16, MakeDefensiveRow>,
    /// Crafting grade codes keyed by item_index.
    ///
    make_grade_codes: DashMap<i16, MakeItemGradeCodeRow>,
    /// Crafting rarity codes keyed by level_grade.
    ///
    make_lare_codes: DashMap<i16, MakeItemLareCodeRow>,
    /// Crafting item groups keyed by group_num.
    ///
    make_item_groups: DashMap<i32, MakeItemGroupRow>,
    /// Random crafting item group mappings keyed by n_index.
    ///
    make_item_group_randoms: DashMap<i32, MakeItemGroupRandomRow>,
    /// Crafting item code lookup keyed by s_index (1..10000).
    ///
    make_items: DashMap<i16, MakeItemRow>,
    /// Rental items keyed by rental_index.
    ///
    rental_items: DashMap<i32, RentalItemRow>,
    /// Atomic counter for generating unique rental indices.
    rental_index_counter: std::sync::atomic::AtomicI32,

    /// NPC sell table: selling_group â†’ list of sell table rows.
    ///
    /// Used at buy time to validate that the requested item is actually
    /// sold by the NPC. Grouped by selling_group for O(1) lookup.
    ///
    item_sell_table: DashMap<i32, Vec<ItemSellTableRow>>,

    /// Item special sewing (crafting) recipes: npc_id â†’ list of recipes.
    ///
    /// The Shozin Exchange / Special Part Sewing system uses this table.
    /// Grouped by NPC ID for efficient lookup during crafting.
    ///
    special_sewing: DashMap<i32, Vec<ItemSpecialSewingRow>>,
    /// Server sewing recipe (`n_index`) -> client `mixtable` row index.
    /// Negative keys (`-npc_id`) hold the per-NPC fallback row.
    special_sewing_client_idx: DashMap<i32, u16>,

    /// Item smash (old man exchange) entries: n_index â†’ smash row.
    ///
    /// Used by the item disassemble system. Entries are grouped by index range
    /// (1M=shields, 2M=weapons, 3M=armor, 4M=earrings, 5M=necklaces/rings).
    ///
    item_smash: DashMap<i32, ItemSmashRow>,

    // â”€â”€ Premium System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Premium type definitions keyed by premium_type (1-13).
    ///
    pub(crate) premium_item_types: DashMap<u8, PremiumItemRow>,
    /// Premium XP bonus entries (level-range based).
    ///
    pub(crate) premium_item_exp: parking_lot::RwLock<Vec<PremiumItemExpRow>>,
    /// Premium gift items keyed by premium_type -> list of gifts.
    ///
    /// by `bPremiumType`. Loaded from MSSQL `PREMIUM_GIFT_ITEM` table.
    premium_gift_items: DashMap<u8, Vec<PremiumGiftItem>>,

    // â”€â”€ Pet System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Pet stats info keyed by pet level (1-60).
    ///
    pet_stats_info: DashMap<u8, PetStatsInfoRow>,
    /// Pet image change/transform recipes keyed by sIndex.
    ///
    pet_image_changes: DashMap<i32, PetImageChangeRow>,

    // â”€â”€ Achievement System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Master achievement definitions keyed by s_index.
    ///
    achieve_main: DashMap<i32, AchieveMainRow>,
    /// War-type achievement sub-table keyed by s_index.
    ///
    achieve_war: DashMap<i32, AchieveWarRow>,
    /// Normal-type achievement sub-table keyed by s_index.
    ///
    achieve_normal: DashMap<i32, AchieveNormalRow>,
    /// Monster-kill achievement sub-table keyed by s_index.
    ///
    achieve_monster: DashMap<i32, AchieveMonsterRow>,
    /// Composite (requirement-based) achievement sub-table keyed by s_index.
    ///
    achieve_com: DashMap<i32, AchieveComRow>,
    /// Achievement title bonuses keyed by title index.
    ///
    achieve_title: DashMap<i32, AchieveTitleRow>,

    // â”€â”€ Perk System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Perk definitions keyed by p_index (0-12).
    ///
    perk_definitions: DashMap<i32, PerkRow>,

    /// JackPot settings: index 0 = EXP, index 1 = Noah.
    ///
    jackpot_settings: parking_lot::RwLock<[JackPotSetting; 2]>,

    //â”€â”€ Mining & Fishing System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Mining/fishing item drop table keyed by n_index.
    ///
    mining_fishing_items: DashMap<i32, MiningFishingItemRow>,
    /// Mining exchange (ore craft) table keyed by n_index.
    ///
    mining_exchanges: DashMap<i16, MiningExchangeRow>,

    // â”€â”€ Bot System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Farm bot population data keyed by bot id (+ MAX_USER offset).
    ///
    bot_farm_data: DashMap<i32, BotHandlerFarmRow>,
    /// Merchant bot item templates keyed by s_index.
    ///
    bot_merchant_templates: DashMap<i16, BotHandlerMerchantRow>,
    /// Pre-configured merchant bot stall data keyed by n_index.
    bot_merchant_data: DashMap<i32, BotMerchantDataRow>,
    /// User bot definitions keyed by id.
    user_bots: DashMap<i32, UserBotRow>,
    /// Bot knights ranking data keyed by sh_index.
    bot_knights_rank: parking_lot::RwLock<Vec<BotKnightsRankRow>>,
    /// Bot personal ranking data keyed by n_rank.
    bot_personal_rank: parking_lot::RwLock<Vec<BotPersonalRankRow>>,

    // ── User Rankings (periodic reload every 15 minutes) ──────────────
    /// User personal rank: uppercase char_name → rank_pos (1-based).
    ///
    /// Populated by `ReloadKnightAndUserRanks()` every 15 minutes.
    pub(crate) user_personal_rank: parking_lot::RwLock<std::collections::HashMap<String, u8>>,
    /// User knights (clan) rank: uppercase char_name → rank_pos (1-based).
    ///
    pub(crate) user_knights_rank: parking_lot::RwLock<std::collections::HashMap<String, u8>>,

    /// Runtime spawned bot instances keyed by BotId.
    ///
    /// Bots are inserted here when spawned (UserInOut INOUT_IN) and removed on
    /// despawn (UserInOut INOUT_OUT / RemoveMapBotList).
    pub(crate) bots: DashMap<BotId, BotInstance>,
    /// Atomic counter for assigning unique BotId values.
    ///
    /// We use a simple monotonic counter starting at BOT_ID_BASE.
    next_bot_id: AtomicU32,
    /// Field-bot (normal-map hunter) configuration and runtime state.
    pub(crate) field_bots: crate::systems::field_bots::FieldBotRegistry,

    // â”€â”€ Tournament (CvC / Party VS) System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Active tournament arenas keyed by zone_id (77, 78, 96-99).
    ///
    tournament_registry: crate::handler::tournament::TournamentRegistry,

    // â”€â”€ Siege Warfare â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Castle siege warfare state (one per castle, typically just Delos).
    ///
    siege_war: tokio::sync::RwLock<SiegeWarfare>,

    /// Castle siege warfare runtime event state (lifecycle, timers, clan kill list).
    ///
    csw_event: tokio::sync::RwLock<CswEventState>,

    // â”€â”€ Knights Cape System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Cape definitions keyed by cape index.
    ///
    knights_capes: DashMap<i16, KnightsCapeRow>,
    /// Castellan cape bonus definitions keyed by bonus type.
    ///
    castellan_bonuses: DashMap<i16, KnightsCapeCastellanBonusRow>,
    /// Castle siege war configuration (single row).
    ///
    csw_opt: parking_lot::RwLock<Option<KnightsCswOptRow>>,

    // â”€â”€ Game Time / Weather â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Shared game-time and weather state, updated by background tasks.
    ///
    game_time_weather: Arc<GameTimeWeather>,

    // â”€â”€ Party BBS (Seeking Party) System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// In-memory list of users/parties seeking party members.
    ///
    seeking_party: parking_lot::RwLock<Vec<SeekingPartyUser>>,

    // â”€â”€ Chat Room System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// In-memory chat rooms keyed by room index.
    ///
    chat_rooms: DashMap<u16, ChatRoom>,
    /// Atomic counter for assigning unique chat room IDs.
    next_chat_room_id: AtomicU16,

    // â”€â”€ Server Settings â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Server-wide configuration (single row, loaded once at startup).
    ///
    server_settings: parking_lot::RwLock<Option<ServerSettingsRow>>,

    /// Persistent login messages (send_type=1) cached at startup.
    ///
    /// on login in `CharacterSelectionHandler.cpp:1106-1127`.
    send_messages: parking_lot::RwLock<Vec<ko_db::models::SendMessage>>,

    /// Scheduled automatic commands loaded from DB.
    ///
    /// second in `ServerStartStopHandler.cpp:64-84`.
    pub(crate) automatic_commands: parking_lot::RwLock<Vec<ko_db::models::AutomaticCommand>>,

    /// Damage balance multipliers (single row, loaded once at startup).
    ///
    damage_settings: parking_lot::RwLock<Option<DamageSettingsRow>>,

    /// Burning feature rate multipliers per flame level (3 tiers, index 0-2).
    ///
    pub burning_features: parking_lot::RwLock<[BurningFeatureRates; 3]>,

    /// Per-nation home/respawn coordinates keyed by nation (1=Karus, 2=Elmorad).
    ///
    /// Source: MSSQL `HOME` table
    home_positions: DashMap<u8, HomeRow>,

    /// Per-zone start positions for nation-specific respawn.
    ///
    start_positions: DashMap<u16, StartPositionRow>,

    /// Random spawn points for special zones, keyed by zone_id.
    ///
    start_positions_random: DashMap<u16, Vec<StartPositionRandomRow>>,

    // -- Monster Summon / Respawn / Boss Random Spawn ----------------------
    /// Monster summon list keyed by s_sid (monster template ID).
    ///
    monster_summon_list: DashMap<i16, MonsterSummonRow>,
    /// Monster respawn loop keyed by dead-monster s_sid.
    ///
    monster_respawn_loop: DashMap<i16, MonsterRespawnLoopRow>,
    /// Scheduled respawn queue — NPC deaths that trigger delayed respawns.
    ///
    /// Entries are checked each NPC AI tick; when their deadline passes, the NPC is spawned.
    scheduled_respawns: parking_lot::Mutex<Vec<ScheduledRespawn>>,
    /// Boss random spawn pool keyed by stage -> Vec of candidate positions.
    ///
    boss_random_spawn: DashMap<i32, Vec<MonsterBossRandomSpawnRow>>,

    // â”€â”€ NPC DOT System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Active DOT effects on NPCs, keyed by NpcId.
    ///
    /// Each NPC can have up to `MAX_TYPE3_REPEAT` active DOT slots,
    /// processed alongside player DOTs by the `dot_tick` system.
    ///
    npc_dots: DashMap<NpcId, Vec<NpcDotSlot>>,

    // â”€â”€ NPC Buff System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Active Type4 buffs/debuffs on NPCs, keyed by NpcId.
    ///
    /// Inner HashMap is keyed by buff_type (matching buffMap).
    /// The `buff_tick` system checks expiry every tick.
    ///
    npc_buffs: DashMap<NpcId, HashMap<i32, NpcBuffEntry>>,

    // â”€â”€ Bifrost Event State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Remaining seconds for the active bifrost event (0 = inactive).
    ///
    bifrost_remaining_secs: std::sync::atomic::AtomicU32,

    // â”€â”€ Item Exchange / Special Stone Tables â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Special stone definitions keyed by n_index.
    ///
    special_stones: DashMap<i32, SpecialStoneRow>,
    /// Monster resource kill notices keyed by proto_id (sid).
    ///
    monster_resources: DashMap<i16, ko_db::models::MonsterResource>,
    /// Random item table keyed by n_index.
    ///
    item_random: DashMap<i32, ItemRandomRow>,
    /// Item group table keyed by group_id.
    ///
    item_groups: DashMap<i16, ItemGroupRow>,
    /// Item exchange experience table keyed by n_index.
    ///
    item_exchange_exp: DashMap<i32, ItemExchangeExpRow>,
    /// Item give exchange table keyed by exchange_index.
    ///
    item_give_exchange: DashMap<i32, ItemGiveExchangeRow>,
    /// Right-click exchange mapping keyed by item_id.
    ///
    item_right_click_exchange: DashMap<i32, ItemRightClickExchangeRow>,
    /// Right exchange definitions keyed by item_id.
    ///
    item_right_exchange: DashMap<i32, ItemRightExchangeRow>,

    // â”€â”€ Event Room System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Event room manager â€” handles BDW, Chaos, Juraid, FT room lifecycle,
    /// event scheduling, sign-up tracking, and timer state machine.
    pub(crate) event_room_manager: EventRoomManager,

    /// Manes Survival spawn configuration and runtime NPC lifecycle.
    pub(crate) manes_survival_manager: crate::systems::manes_survival::ManesSurvivalManager,

    /// BDW per-room state (altar, monument counts, respawn timer).
    ///
    pub(crate) bdw_manager: parking_lot::RwLock<BdwManager>,

    /// Event rewards keyed by local_id (e.g. 9=BDW, 11=Juraid).
    ///
    event_rewards: DashMap<i16, Vec<EventRewardRow>>,

    /// Event timer show list entries for client UI.
    ///
    pub(crate) event_timer_show_list:
        parking_lot::RwLock<Vec<ko_db::models::event_schedule::EventTimerShowRow>>,

    /// Server info notice message 1 (broadcast every 30 min).
    ///
    pub(crate) game_info_notice1: parking_lot::RwLock<String>,

    /// Server info notice message 2 (broadcast every 60 min).
    ///
    pub(crate) game_info_notice2: parking_lot::RwLock<String>,

    // â”€â”€ Forgotten Temple Event â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// FT stage definitions loaded from DB at startup.
    ///
    ft_stages: parking_lot::RwLock<Vec<FtStageRow>>,
    /// FT monster summon definitions loaded from DB at startup.
    ///
    ft_summons: parking_lot::RwLock<Vec<FtSummonRow>>,
    /// Forgotten Temple runtime event state (timer, stage, monster count).
    ///
    forgotten_temple_state: ForgettenTempleState,

    // â”€â”€ Dungeon Defence (Full Moon Rift) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// DD stage definitions loaded from DB at startup.
    ///
    dd_stages: parking_lot::RwLock<Vec<DfStageRow>>,
    /// DD monster spawn definitions loaded from DB at startup.
    ///
    dd_monsters: parking_lot::RwLock<Vec<DfMonsterRow>>,
    /// DD runtime room pool (60 rooms, pre-initialized at startup).
    ///
    dd_rooms: Vec<crate::handler::dungeon_defence::DdRoomInfo>,

    //â”€â”€ Under The Castle Event â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Under The Castle monster spawn definitions loaded from DB at startup.
    ///
    utc_spawns: parking_lot::RwLock<Vec<MonsterUnderTheCastleRow>>,
    /// Under The Castle runtime event state.
    ///
    under_the_castle_state: crate::handler::under_castle::UnderTheCastleState,

    // â”€â”€ Daily Quest Definitions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Daily quest definitions keyed by quest id (1-53).
    ///
    daily_quests: DashMap<i16, DailyQuestRow>,

    // ── Daily Rank Cache ─────────────────────────────────────────────────
    /// Cached daily rank data loaded from `daily_rank` table at startup.
    ///
    daily_rank_cache: parking_lot::RwLock<Vec<ko_db::models::daily_rank::DailyRankRow>>,

    // â”€â”€ Draki Tower Event â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Draki Tower stage definitions loaded from DB at startup (41 rows).
    ///
    draki_tower_stages: parking_lot::RwLock<Vec<DrakiTowerStageRow>>,
    /// Draki Tower monster spawn definitions loaded from DB at startup (166 rows).
    ///
    draki_monster_list: parking_lot::RwLock<Vec<DrakiMonsterListRow>>,
    /// Draki Tower runtime room pool (60 rooms, keyed by room_id 1-60).
    ///
    draki_tower_rooms: parking_lot::RwLock<std::collections::HashMap<u16, DrakiTowerRoomInfo>>,

    // â”€â”€ Chaos Stone Event â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Chaos stone spawn point definitions loaded from DB at startup (12 rows).
    ///
    chaos_stone_spawns: parking_lot::RwLock<Vec<ChaosStoneSpawnRow>>,
    /// Chaos stone monster summon list loaded from DB at startup (18 rows).
    ///
    chaos_stone_summon_list: parking_lot::RwLock<Vec<ChaosStoneSummonListRow>>,
    /// Chaos stone stage/family definitions loaded from DB at startup (9 rows).
    ///
    chaos_stone_stages: parking_lot::RwLock<Vec<ChaosStoneSummonStageRow>>,
    /// Chaos stone event rewards by rank (18 rows).
    ///
    chaos_stone_rewards: parking_lot::RwLock<Vec<EventChaosRewardRow>>,
    /// Runtime chaos stone info map: chaos_index -> ChaosStoneInfo.
    ///
    /// Loaded from `chaos_stone_spawns` at startup via `load_chaos_stones()`.
    pub(crate) chaos_stone_infos:
        parking_lot::RwLock<HashMap<u8, crate::handler::chaos_stone::ChaosStoneInfo>>,

    // â”€â”€ Character Creation Data â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Starting equipment per class, keyed by class_type â†’ Vec of slot entries.
    ///
    new_char_set: DashMap<i16, Vec<CreateNewCharSetRow>>,
    /// Starting stats/level/gold per (class_type, job_type).
    ///
    new_char_value: DashMap<(i16, i16), CreateNewCharValueRow>,

    // â”€â”€ Monster Event Spawns â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Monster stone respawn definitions loaded from DB at startup.
    ///
    monster_stone_respawn: parking_lot::RwLock<Vec<MonsterStoneRespawnRow>>,
    /// Monster Stone room pool manager (750 rooms).
    ///
    pub(crate) monster_stone_manager: parking_lot::RwLock<MonsterStoneManager>,
    /// Boss random stage definitions loaded from DB at startup (33 rows).
    ///
    monster_boss_random_stages: parking_lot::RwLock<Vec<MonsterBossRandomStageRow>>,
    /// Juraid Mountain per-room bridge state snapshot.
    ///
    /// Synced from event_system when bridges open; used by `CheckDevaAttack()` in
    /// `handle_npc_attack()`. Key = room_id (1-based).
    ///
    juraid_bridge_states: DashMap<u8, JuraidBridgeState>,
    /// Pending Juraid Monument respawn timestamps.
    ///
    /// Key = (room_id, nation), nation 1=Karus Monument, 2=El Morad Monument.
    juraid_monument_respawns: DashMap<(u8, u8), u64>,

    /// Juraid Mountain monster respawn definitions loaded from DB at startup (136 rows).
    ///
    monster_juraid_respawn: parking_lot::RwLock<Vec<MonsterJuraidRespawnRow>>,
    /// Monster challenge config entries loaded from DB at startup (3 rows).
    ///
    monster_challenge: parking_lot::RwLock<Vec<MonsterChallengeRow>>,
    /// Monster challenge summon list loaded from DB at startup (168 rows).
    ///
    monster_challenge_summon: parking_lot::RwLock<Vec<MonsterChallengeSummonRow>>,

    // â”€â”€ Sheriff System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// In-memory sheriff reports (player reporting / voting system).
    ///
    sheriff_reports: Arc<SheriffReportMap>,

    // â”€â”€ Cinderella War (Fun Class) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Cinderella War tier settings loaded from DB at startup (5 rows).
    ///
    cindwar_settings: parking_lot::RwLock<Vec<CindwarSettingRow>>,
    /// Cinderella War equipment items per tier, keyed by (tier, class).
    ///
    cindwar_items: parking_lot::RwLock<Vec<CindwarItemRow>>,
    /// Cinderella War rank-based rewards (20 rows).
    ///
    cindwar_rewards: parking_lot::RwLock<Vec<CindwarRewardRow>>,
    /// Cinderella War reward items (normalized from flat arrays).
    ///
    cindwar_reward_items: parking_lot::RwLock<Vec<CindwarRewardItemRow>>,
    /// Cinderella War per-class stat/skill presets (16 rows).
    ///
    cindwar_stats: parking_lot::RwLock<Vec<CindwarStatRow>>,
    /// Whether a Cinderella War event is currently active.
    ///
    pub cindwar_active: std::sync::atomic::AtomicBool,
    /// Zone ID where the current Cinderella War is taking place (0 = none).
    ///
    pub cindwar_zone_id: AtomicU16,
    /// Set of session IDs that are participants in the active Cinderella War.
    ///
    cindwar_event_users: DashMap<SessionId, ()>,
    /// Sessions whose client still shows the Fun Class identity; refreshed once the zone change lands.
    cind_refresh_pending: DashMap<SessionId, ()>,
    /// Ultima's Dungeon: bosses that cannot lose HP while their seal stands.
    pub(crate) ultima_boss_guard: DashMap<u32, ()>,
    /// Ultima's Dungeon: seal structures (only the fixed basic-attack hit counts).
    pub(crate) ultima_seals: DashMap<u32, i32>,
    /// Ultima's Dungeon: bosses in their lethal window (warriors hit 1.5x).
    pub(crate) ultima_lethal: DashMap<u32, ()>,
    /// Per-player Cinderella War state (original data backup, cooldowns, KDA).
    ///
    cindwar_player_states: DashMap<SessionId, crate::handler::cinderella::CindirellaPlayerState>,
    /// Global Cinderella War event lifecycle state.
    ///
    cindwar_event_state: parking_lot::RwLock<crate::handler::cinderella::CindirellaEventState>,

    // â”€â”€ Cash Shop (PUS) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Whether the Zindan War (special event) is currently opened/active.
    ///
    /// When true, attacks and magic are allowed in SPBATTLE zones (105-115).
    pub zindan_event_opened: std::sync::atomic::AtomicBool,

    /// Zindan War score/timer state (names, kill counts, finish time).
    ///
    pub(crate) zindan_war_state: parking_lot::RwLock<ZindanWarState>,

    /// Active PUS categories keyed by category_id (5 rows).
    ///
    pub(crate) pus_categories: DashMap<i16, PusCategoryRow>,
    /// PUS item listings keyed by category_id -> Vec of items.
    ///
    pub(crate) pus_items_by_category: DashMap<i16, Vec<PusItemRow>>,
    /// PUS item lookup keyed by listing ID for direct purchase validation.
    pub(crate) pus_items_by_id: DashMap<i32, PusItemRow>,

    // â”€â”€ Soccer Event System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Per-zone soccer event rooms (Moradon 21-25), purely in-memory.
    ///
    soccer_state: crate::handler::soccer::SharedSoccerState,

    // â”€â”€ Battle (War) System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Nation battle state â€” all war-related counters, flags, and timers.
    battle_state: parking_lot::RwLock<crate::systems::war::BattleState>,

    /// War commander names — top-ranked clan leaders designated during war.
    ///
    /// `BattleZoneSelectCommanders()` / `LoadKnightsRankTable()`.
    /// On login during war, matching players get `COMMAND_CAPTAIN` fame.
    war_commanders: parking_lot::RwLock<std::collections::HashSet<String>>,

    /// Permanent chat banner state.
    ///
    /// When `Some(text)`, the banner is displayed to all players. `None` = off.
    permanent_chat: parking_lot::RwLock<Option<String>>,

    /// Banish-of-winner spawn definitions loaded from `banish_of_winner` table.
    ///
    /// Used by `BattleZoneRemnantSpawn()` to spawn event NPCs after a war victory.
    banish_of_winner: parking_lot::RwLock<Vec<ko_db::models::BanishOfWinner>>,

    /// Gold cost discount for stat/skill reset: 0=off, 1=winning nation, 2=all.
    ///
    /// Set by GM commands `+discount` (1), `+alldiscount` (2), `+offdiscount` (0).
    /// When active, stat/skill reset gold costs are halved.
    pub(crate) discount: std::sync::atomic::AtomicU8,

    /// Whether NPC war buffs are currently active (BATTLEZONE_OPEN applied).
    ///
    /// (type > 10, nation in {Karus, Elmorad}) get HP×1.2, AC×1.2, Damage×0.5,
    /// Resist×2 during war. Reverted on BATTLEZONE_CLOSE.
    pub(crate) npc_war_buffed: std::sync::atomic::AtomicBool,
    // â”€â”€ PVP Monument System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Per-zone PVP monument ownership by nation (0=neutral, 1=Karus, 2=Elmorad).
    ///
    pub(crate) pvp_monument_nation: DashMap<u16, u8>,

    // â”€â”€ Beef Roast (Bifrost) Event State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Bifrost/beef roast event runtime state.
    ///
    beef_event: parking_lot::RwLock<BeefEventState>,

    // ── Bowl Event ──────────────────────────────────────────────────────
    /// Whether the bowl event is currently active.
    ///
    bowl_event_active: std::sync::atomic::AtomicBool,
    /// Remaining seconds for the bowl event timer (0 = inactive).
    ///
    bowl_event_time: std::sync::atomic::AtomicU16,
    /// Zone ID where the bowl event is running.
    ///
    bowl_event_zone: std::sync::atomic::AtomicU8,

    // â”€â”€ Lua Quest Engine â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Lua quest scripting engine for NPC dialog and quest logic.
    ///
    lua_engine: Arc<LuaEngine>,

    // â”€â”€ Daily Operation Tracking â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Per-character daily operation timestamps, keyed by character name.
    ///
    /// Tracks when each daily-limited action was last performed so the
    /// server can enforce 24-hour (1440 minute) cooldowns.
    ///
    pub(crate) daily_ops: DashMap<String, UserDailyOp>,

    /// Temporary GM socket for `/plc` program check command.
    /// Stores the requesting GM's session ID so the client response can be routed back.
    ///
    pub(crate) plc_gm_socket: AtomicU32,

    // â”€â”€ Notice Board System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Notice board entries (title, message) pairs â€” type=2, max 5 entries.
    ///
    /// as (title=even, message=odd), used in `CUser::SendNotice()`.
    notice_board_entries: parking_lot::RwLock<Vec<(String, String)>>,
    /// Top-right notice messages â€” type=1, max 20 entries.
    ///
    /// `Notice_up.txt`, used in `CUser::TopSendNotice()`.
    top_notice_entries: parking_lot::RwLock<Vec<String>>,
    /// Cape bonus notice entries (title, message) pairs â€” type=2, max 5 entries.
    ///
    /// `CapeBonus.txt`, sent via `CUser::SendCapeBonusNotice()`.
    cape_bonus_entries: parking_lot::RwLock<Vec<(String, String)>>,
    /// Clan premium notice entries (title, message) pairs â€” type=2, max 5 entries.
    ///
    /// `ClanPremiumNotice.txt`, sent via `CUser::SendClanPremiumNotice()`.
    clan_premium_entries: parking_lot::RwLock<Vec<(String, String)>>,
    /// Right-top title messages (title, message) pairs — sent via WIZ_NOTICE sub=4.
    ///
    /// `RIGHT_TOP_TITLE` DB table, sent via `CUser::RightTopTitleMsg()`.
    right_top_titles: parking_lot::RwLock<Vec<(String, String)>>,
    /// Admin-managed right-top info list entries (`info_panel_entry` table, see `handler::info_panel`).
    info_panel_entries: parking_lot::RwLock<Arc<Vec<crate::handler::info_panel::InfoPanelEntry>>>,

    // â”€â”€ Zone Reward Tables â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Zone kill reward definitions (all rows, iterated per kill).
    ///
    zone_kill_rewards: parking_lot::RwLock<Vec<ZoneKillReward>>,
    /// Zone online reward definitions (all rows, used for periodic online rewards).
    ///
    zone_online_rewards: parking_lot::RwLock<Vec<ZoneOnlineReward>>,

    // â”€â”€ Wanted Event System â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Per-room wanted event state (3 rooms: Ronark Land, Ardream, Ronark Land Base).
    ///
    wanted_rooms: parking_lot::RwLock<[WantedEventRoom; MAX_WANTED_ROOMS]>,
    /// Whether the auto-wanted event system is enabled.
    ///
    pub(crate) wanted_auto_enabled: std::sync::atomic::AtomicBool,
    /// Last time the wanted system broadcast position to the map (unix timestamp).
    ///
    pub(crate) wanted_map_show_time: std::sync::atomic::AtomicU64,
    // ── Login Lock ──────────────────────────────────────────────────────
    /// Set of account IDs currently being processed in the login flow.
    ///
    /// Prevents race conditions when two simultaneous login attempts arrive
    /// for the same account. The login handler inserts the account (lowercase)
    /// before processing and removes it when done.
    login_in_progress: DashMap<String, ()>,

    /// Names of currently connected GM users.
    ///
    gm_list: parking_lot::RwLock<Vec<String>>,

    // â”€â”€ Database Pool â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Shared database connection pool for async persistence from subsystems.
    ///
    /// `None` in test contexts where no DB is available.
    db_pool: Option<DbPool>,

    /// Rate limiter for flood protection — throttles packets per session/IP.
    rate_limiter: crate::rate_limiter::RateLimiter,

    // ── Lottery Event System ─────────────────────────────────────────────
    /// Global lottery event runtime state.
    ///
    /// Shared via `Arc<RwLock<...>>` so the timer task and handlers can both access it.
    lottery_process: crate::handler::lottery::SharedLotteryProcess,

    // ── Collection Race Event System ──────────────────────────────────────
    /// Global Collection Race event runtime state.
    ///
    /// Shared via `Arc<RwLock<...>>` so the timer task and handlers can both access it.
    pub(crate) collection_race_event: crate::handler::collection_race::SharedCollectionRaceEvent,

    /// Collection Race event definition table (loaded from DB at startup).
    ///
    pub(crate) collection_race_settings: DashMap<i16, crate::handler::collection_race::CrEventDef>,
    /// Rewards of the events edited in the web panel (event_reward_rule), see `reward_rules`.
    pub(crate) reward_rules: parking_lot::RwLock<Vec<crate::reward_rules::RewardRule>>,

    // ── Wheel of Fun ─────────────────────────────────────────────────────────
    /// Wheel of Fun settings loaded from DB at startup.
    ///
    wheel_of_fun_settings:
        parking_lot::RwLock<Vec<ko_db::models::wheel_of_fun::WheelOfFunSettings>>,

    // ── Anti-AFK NPC List ──────────────────────────────────────────────────
    /// Anti-AFK NPC IDs sent to the client on game entry.
    ///
    anti_afk_npc_ids: parking_lot::RwLock<Vec<u16>>,

    // ── Flying Santa/Angel Event ────────────────────────────────────────────
    /// Flying Santa/Angel visual event state (0=none, 1=santa, 2=angel).
    ///
    pub(crate) santa_or_angel: std::sync::atomic::AtomicU8,

    // ── RANKBUG Configuration ───────────────────────────────────────────────
    /// Ranking system multiplier configuration.
    ///
    pub(crate) rank_bug: parking_lot::RwLock<ko_db::models::rankbug::RankBugConfig>,

    // ── Player Ranking Rewards ──────────────────────────────────────────────
    /// Loyalty (NP) given to top-10 PK zone ranked players every minute.
    ///
    pub(crate) player_ranking_loyalty_reward: std::sync::atomic::AtomicU32,

    /// Knight Cash given to top-10 PK zone ranked players every minute.
    ///
    pub(crate) player_ranking_kc_reward: std::sync::atomic::AtomicU32,

    /// Zone IDs where player ranking rewards are distributed.
    ///
    pub(crate) player_ranking_reward_zones: parking_lot::RwLock<Vec<u16>>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldState {
    /// Create a new world state with a fallback zone 21 (no DB).
    ///
    /// For testing or when DB is unavailable.
    pub fn new() -> Self {
        let world = Self {
            sessions: DashMap::new(),
            zone_session_index: DashMap::new(),
            name_to_session: DashMap::new(),
            online_count: AtomicU32::new(0),
            zones: DashMap::new(),
            next_session_id: AtomicU16::new(1), // Start at 1 (0 = invalid)
            npc_templates: DashMap::new(),
            npc_instances: DashMap::new(),
            next_npc_id: AtomicU32::new(NPC_BAND),
            coefficients: DashMap::new(),
            magic_table: DashMap::new(),
            magic_type1: DashMap::new(),
            magic_type2: DashMap::new(),
            magic_type3: DashMap::new(),
            magic_type4: DashMap::new(),
            magic_type5: DashMap::new(),
            magic_type6: DashMap::new(),
            magic_type7: DashMap::new(),
            magic_type8: DashMap::new(),
            magic_type9: DashMap::new(),
            npc_hp: DashMap::new(),
            npc_damage: DashMap::new(),
            npc_ai: DashMap::new(),
            items: DashMap::new(),
            parties: DashMap::new(),
            next_party_id: AtomicU16::new(1),
            party_invitations: DashMap::new(),
            knights: DashMap::new(),
            alliances: DashMap::new(),
            ground_bundles: DashMap::new(),
            next_bundle_id: AtomicU32::new(1),
            next_item_serial: std::sync::atomic::AtomicU64::new(initial_item_serial()),
            quest_helpers: DashMap::new(),
            collection_drop_items: DashMap::new(),
            quest_monsters: DashMap::new(),
            quest_npc_list: DashMap::new(),
            quest_menus: DashMap::new(),
            quest_talks: DashMap::new(),
            quest_skills_closed_check: DashMap::new(),
            quest_skills_open_set_up: DashMap::new(),
            pk_zone_rankings: [DashMap::new(), DashMap::new()],
            zindan_rankings: [DashMap::new(), DashMap::new()],
            bdw_rankings: [DashMap::new(), DashMap::new()],
            chaos_rankings: DashMap::new(),
            ranking_update_in_progress: std::sync::atomic::AtomicBool::new(false),
            level_up_table: DashMap::new(),
            king_systems: DashMap::new(),
            upgrade_recipes: DashMap::new(),
            upgrade_settings: DashMap::new(),
            itemup_probability: parking_lot::RwLock::new(None),
            item_ops: DashMap::new(),
            set_items: DashMap::new(),
            monster_items: DashMap::new(),
            npc_items: DashMap::new(),
            item_exchanges: DashMap::new(),
            item_upgrades: DashMap::new(),
            make_weapons: DashMap::new(),
            make_defensives: DashMap::new(),
            make_grade_codes: DashMap::new(),
            make_lare_codes: DashMap::new(),
            make_item_groups: DashMap::new(),
            make_item_group_randoms: DashMap::new(),
            make_items: DashMap::new(),
            rental_items: DashMap::new(),
            rental_index_counter: std::sync::atomic::AtomicI32::new(0),
            item_sell_table: DashMap::new(),
            special_sewing: DashMap::new(),
            special_sewing_client_idx: DashMap::new(),
            item_smash: DashMap::new(),
            premium_item_types: DashMap::new(),
            premium_item_exp: parking_lot::RwLock::new(Vec::new()),
            premium_gift_items: DashMap::new(),
            pet_stats_info: DashMap::new(),
            pet_image_changes: DashMap::new(),
            achieve_main: DashMap::new(),
            achieve_war: DashMap::new(),
            achieve_normal: DashMap::new(),
            achieve_monster: DashMap::new(),
            achieve_com: DashMap::new(),
            achieve_title: DashMap::new(),
            perk_definitions: DashMap::new(),
            jackpot_settings: parking_lot::RwLock::new([JackPotSetting::default(); 2]),
            mining_fishing_items: DashMap::new(),
            mining_exchanges: DashMap::new(),
            bot_farm_data: DashMap::new(),
            bot_merchant_templates: DashMap::new(),
            bot_merchant_data: DashMap::new(),
            user_bots: DashMap::new(),
            bot_knights_rank: parking_lot::RwLock::new(Vec::new()),
            bot_personal_rank: parking_lot::RwLock::new(Vec::new()),
            user_personal_rank: parking_lot::RwLock::new(std::collections::HashMap::new()),
            user_knights_rank: parking_lot::RwLock::new(std::collections::HashMap::new()),
            bots: DashMap::new(),
            next_bot_id: AtomicU32::new(BOT_ID_BASE),
            field_bots: Default::default(),
            tournament_registry: crate::handler::tournament::new_tournament_registry(),
            siege_war: tokio::sync::RwLock::new(SiegeWarfare::default()),
            war_commanders: parking_lot::RwLock::new(std::collections::HashSet::new()),
            permanent_chat: parking_lot::RwLock::new(None),
            banish_of_winner: parking_lot::RwLock::new(Vec::new()),
            csw_event: tokio::sync::RwLock::new(CswEventState::default()),
            game_time_weather: Arc::new(GameTimeWeather::new()),
            seeking_party: parking_lot::RwLock::new(Vec::new()),
            chat_rooms: DashMap::new(),
            next_chat_room_id: AtomicU16::new(1),
            server_settings: parking_lot::RwLock::new(None),
            send_messages: parking_lot::RwLock::new(Vec::new()),
            automatic_commands: parking_lot::RwLock::new(Vec::new()),
            damage_settings: parking_lot::RwLock::new(None),
            burning_features: parking_lot::RwLock::new([BurningFeatureRates::default(); 3]),
            home_positions: DashMap::new(),
            start_positions: DashMap::new(),
            start_positions_random: DashMap::new(),
            monster_summon_list: DashMap::new(),
            monster_respawn_loop: DashMap::new(),
            scheduled_respawns: parking_lot::Mutex::new(Vec::new()),
            boss_random_spawn: DashMap::new(),
            npc_dots: DashMap::new(),
            npc_buffs: DashMap::new(),
            bifrost_remaining_secs: std::sync::atomic::AtomicU32::new(0),
            special_stones: DashMap::new(),
            monster_resources: DashMap::new(),
            item_random: DashMap::new(),
            item_groups: DashMap::new(),
            item_exchange_exp: DashMap::new(),
            item_give_exchange: DashMap::new(),
            item_right_click_exchange: DashMap::new(),
            item_right_exchange: DashMap::new(),
            event_room_manager: EventRoomManager::new(),
            manes_survival_manager: crate::systems::manes_survival::ManesSurvivalManager::default(),
            bdw_manager: parking_lot::RwLock::new(BdwManager::default()),
            event_rewards: DashMap::new(),
            event_timer_show_list: parking_lot::RwLock::new(Vec::new()),
            game_info_notice1: parking_lot::RwLock::new(String::new()),
            game_info_notice2: parking_lot::RwLock::new(String::new()),
            ft_stages: parking_lot::RwLock::new(Vec::new()),
            ft_summons: parking_lot::RwLock::new(Vec::new()),
            forgotten_temple_state: ForgettenTempleState::new(),
            dd_stages: parking_lot::RwLock::new(Vec::new()),
            dd_monsters: parking_lot::RwLock::new(Vec::new()),
            dd_rooms: (1..=crate::handler::dungeon_defence::DD_MAX_ROOMS)
                .map(crate::handler::dungeon_defence::DdRoomInfo::new)
                .collect(),
            utc_spawns: parking_lot::RwLock::new(Vec::new()),
            under_the_castle_state: crate::handler::under_castle::UnderTheCastleState::new(),
            daily_quests: DashMap::new(),
            daily_rank_cache: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_spawns: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_summon_list: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_stages: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_rewards: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_infos: parking_lot::RwLock::new(HashMap::new()),
            draki_tower_stages: parking_lot::RwLock::new(Vec::new()),
            draki_monster_list: parking_lot::RwLock::new(Vec::new()),
            draki_tower_rooms: parking_lot::RwLock::new({
                let mut m = std::collections::HashMap::new();
                for i in 1..=crate::handler::draki_tower::EVENT_MAX_ROOM {
                    m.insert(i, DrakiTowerRoomInfo::new(i));
                }
                m
            }),
            new_char_set: DashMap::new(),
            new_char_value: DashMap::new(),
            knights_capes: DashMap::new(),
            castellan_bonuses: DashMap::new(),
            csw_opt: parking_lot::RwLock::new(None),
            monster_stone_respawn: parking_lot::RwLock::new(Vec::new()),
            monster_stone_manager: parking_lot::RwLock::new(MonsterStoneManager::new()),
            monster_boss_random_stages: parking_lot::RwLock::new(Vec::new()),
            juraid_bridge_states: DashMap::new(),
            juraid_monument_respawns: DashMap::new(),
            monster_juraid_respawn: parking_lot::RwLock::new(Vec::new()),
            monster_challenge: parking_lot::RwLock::new(Vec::new()),
            monster_challenge_summon: parking_lot::RwLock::new(Vec::new()),
            sheriff_reports: crate::handler::sheriff::new_sheriff_map(),
            cindwar_settings: parking_lot::RwLock::new(Vec::new()),
            cindwar_items: parking_lot::RwLock::new(Vec::new()),
            cindwar_rewards: parking_lot::RwLock::new(Vec::new()),
            cindwar_reward_items: parking_lot::RwLock::new(Vec::new()),
            cindwar_stats: parking_lot::RwLock::new(Vec::new()),
            cindwar_active: std::sync::atomic::AtomicBool::new(false),
            cindwar_zone_id: AtomicU16::new(0),
            cindwar_event_users: DashMap::new(),
            cind_refresh_pending: DashMap::new(),
            ultima_boss_guard: DashMap::new(),
            ultima_seals: DashMap::new(),
            ultima_lethal: DashMap::new(),
            cindwar_player_states: DashMap::new(),
            cindwar_event_state: parking_lot::RwLock::new(
                crate::handler::cinderella::CindirellaEventState::default(),
            ),
            zindan_event_opened: std::sync::atomic::AtomicBool::new(false),
            zindan_war_state: parking_lot::RwLock::new(ZindanWarState::default()),
            pus_categories: DashMap::new(),
            pus_items_by_category: DashMap::new(),
            pus_items_by_id: DashMap::new(),
            soccer_state: crate::handler::soccer::new_soccer_state(),
            battle_state: parking_lot::RwLock::new(crate::systems::war::BattleState::new()),
            discount: std::sync::atomic::AtomicU8::new(0),
            npc_war_buffed: std::sync::atomic::AtomicBool::new(false),
            pvp_monument_nation: DashMap::new(),
            beef_event: parking_lot::RwLock::new(BeefEventState::default()),
            bowl_event_active: std::sync::atomic::AtomicBool::new(false),
            bowl_event_time: std::sync::atomic::AtomicU16::new(0),
            bowl_event_zone: std::sync::atomic::AtomicU8::new(0),
            daily_ops: DashMap::new(),
            plc_gm_socket: AtomicU32::new(u32::MAX),
            lua_engine: Arc::new(LuaEngine::new()),
            notice_board_entries: parking_lot::RwLock::new(Vec::new()),
            top_notice_entries: parking_lot::RwLock::new(Vec::new()),
            cape_bonus_entries: parking_lot::RwLock::new(Vec::new()),
            clan_premium_entries: parking_lot::RwLock::new(Vec::new()),
            right_top_titles: parking_lot::RwLock::new(Vec::new()),
            info_panel_entries: parking_lot::RwLock::new(Arc::new(Vec::new())),
            zone_kill_rewards: parking_lot::RwLock::new(Vec::new()),
            zone_online_rewards: parking_lot::RwLock::new(Vec::new()),
            wanted_rooms: parking_lot::RwLock::new(Default::default()),
            wanted_auto_enabled: std::sync::atomic::AtomicBool::new(false),
            wanted_map_show_time: std::sync::atomic::AtomicU64::new(0),
            login_in_progress: DashMap::new(),
            gm_list: parking_lot::RwLock::new(Vec::new()),
            db_pool: None,
            rate_limiter: crate::rate_limiter::RateLimiter::new(),
            lottery_process: crate::handler::lottery::new_lottery_process(),
            collection_race_event: crate::handler::collection_race::new_collection_race_event(),
            collection_race_settings: DashMap::new(),
            reward_rules: parking_lot::RwLock::new(Vec::new()),
            wheel_of_fun_settings: parking_lot::RwLock::new(Vec::new()),
            anti_afk_npc_ids: parking_lot::RwLock::new(Vec::new()),
            santa_or_angel: std::sync::atomic::AtomicU8::new(0),
            rank_bug: parking_lot::RwLock::new(ko_db::models::rankbug::RankBugConfig::default()),
            player_ranking_loyalty_reward: std::sync::atomic::AtomicU32::new(0),
            player_ranking_kc_reward: std::sync::atomic::AtomicU32::new(0),
            player_ranking_reward_zones: parking_lot::RwLock::new(vec![71, 72, 73]),
        };

        // Fallback: zone 21 = Moradon with default 1024 map size
        world.ensure_zone(21, 1024);

        world
    }

    /// Load world state from the database and SMD files.
    ///
    /// This is the primary constructor for production use.
    ///
    pub async fn load(pool: &DbPool, map_dir: &Path) -> anyhow::Result<Self> {
        let world = Self {
            sessions: DashMap::new(),
            zone_session_index: DashMap::new(),
            name_to_session: DashMap::new(),
            online_count: AtomicU32::new(0),
            zones: DashMap::new(),
            next_session_id: AtomicU16::new(1),
            npc_templates: DashMap::new(),
            npc_instances: DashMap::new(),
            next_npc_id: AtomicU32::new(NPC_BAND),
            coefficients: DashMap::new(),
            magic_table: DashMap::new(),
            magic_type1: DashMap::new(),
            magic_type2: DashMap::new(),
            magic_type3: DashMap::new(),
            magic_type4: DashMap::new(),
            magic_type5: DashMap::new(),
            magic_type6: DashMap::new(),
            magic_type7: DashMap::new(),
            magic_type8: DashMap::new(),
            magic_type9: DashMap::new(),
            npc_hp: DashMap::new(),
            npc_damage: DashMap::new(),
            npc_ai: DashMap::new(),
            items: DashMap::new(),
            parties: DashMap::new(),
            next_party_id: AtomicU16::new(1),
            party_invitations: DashMap::new(),
            knights: DashMap::new(),
            alliances: DashMap::new(),
            ground_bundles: DashMap::new(),
            next_bundle_id: AtomicU32::new(1),
            next_item_serial: std::sync::atomic::AtomicU64::new(initial_item_serial()),
            quest_helpers: DashMap::new(),
            collection_drop_items: DashMap::new(),
            quest_monsters: DashMap::new(),
            quest_npc_list: DashMap::new(),
            quest_menus: DashMap::new(),
            quest_talks: DashMap::new(),
            quest_skills_closed_check: DashMap::new(),
            quest_skills_open_set_up: DashMap::new(),
            pk_zone_rankings: [DashMap::new(), DashMap::new()],
            zindan_rankings: [DashMap::new(), DashMap::new()],
            bdw_rankings: [DashMap::new(), DashMap::new()],
            chaos_rankings: DashMap::new(),
            ranking_update_in_progress: std::sync::atomic::AtomicBool::new(false),
            level_up_table: DashMap::new(),
            king_systems: DashMap::new(),
            upgrade_recipes: DashMap::new(),
            upgrade_settings: DashMap::new(),
            itemup_probability: parking_lot::RwLock::new(None),
            item_ops: DashMap::new(),
            set_items: DashMap::new(),
            monster_items: DashMap::new(),
            npc_items: DashMap::new(),
            item_exchanges: DashMap::new(),
            item_upgrades: DashMap::new(),
            make_weapons: DashMap::new(),
            make_defensives: DashMap::new(),
            make_grade_codes: DashMap::new(),
            make_lare_codes: DashMap::new(),
            make_item_groups: DashMap::new(),
            make_item_group_randoms: DashMap::new(),
            make_items: DashMap::new(),
            rental_items: DashMap::new(),
            rental_index_counter: std::sync::atomic::AtomicI32::new(0),
            item_sell_table: DashMap::new(),
            special_sewing: DashMap::new(),
            special_sewing_client_idx: DashMap::new(),
            item_smash: DashMap::new(),
            premium_item_types: DashMap::new(),
            premium_item_exp: parking_lot::RwLock::new(Vec::new()),
            premium_gift_items: DashMap::new(),
            pet_stats_info: DashMap::new(),
            pet_image_changes: DashMap::new(),
            achieve_main: DashMap::new(),
            achieve_war: DashMap::new(),
            achieve_normal: DashMap::new(),
            achieve_monster: DashMap::new(),
            achieve_com: DashMap::new(),
            achieve_title: DashMap::new(),
            perk_definitions: DashMap::new(),
            jackpot_settings: parking_lot::RwLock::new([JackPotSetting::default(); 2]),
            mining_fishing_items: DashMap::new(),
            mining_exchanges: DashMap::new(),
            bot_farm_data: DashMap::new(),
            bot_merchant_templates: DashMap::new(),
            bot_merchant_data: DashMap::new(),
            user_bots: DashMap::new(),
            bot_knights_rank: parking_lot::RwLock::new(Vec::new()),
            bot_personal_rank: parking_lot::RwLock::new(Vec::new()),
            user_personal_rank: parking_lot::RwLock::new(std::collections::HashMap::new()),
            user_knights_rank: parking_lot::RwLock::new(std::collections::HashMap::new()),
            bots: DashMap::new(),
            next_bot_id: AtomicU32::new(BOT_ID_BASE),
            field_bots: Default::default(),
            tournament_registry: crate::handler::tournament::new_tournament_registry(),
            siege_war: tokio::sync::RwLock::new(SiegeWarfare::default()),
            war_commanders: parking_lot::RwLock::new(std::collections::HashSet::new()),
            permanent_chat: parking_lot::RwLock::new(None),
            banish_of_winner: parking_lot::RwLock::new(Vec::new()),
            csw_event: tokio::sync::RwLock::new(CswEventState::default()),
            game_time_weather: Arc::new(GameTimeWeather::new()),
            seeking_party: parking_lot::RwLock::new(Vec::new()),
            chat_rooms: DashMap::new(),
            next_chat_room_id: AtomicU16::new(1),
            server_settings: parking_lot::RwLock::new(None),
            send_messages: parking_lot::RwLock::new(Vec::new()),
            automatic_commands: parking_lot::RwLock::new(Vec::new()),
            damage_settings: parking_lot::RwLock::new(None),
            burning_features: parking_lot::RwLock::new([BurningFeatureRates::default(); 3]),
            home_positions: DashMap::new(),
            start_positions: DashMap::new(),
            start_positions_random: DashMap::new(),
            monster_summon_list: DashMap::new(),
            monster_respawn_loop: DashMap::new(),
            scheduled_respawns: parking_lot::Mutex::new(Vec::new()),
            boss_random_spawn: DashMap::new(),
            npc_dots: DashMap::new(),
            npc_buffs: DashMap::new(),
            bifrost_remaining_secs: std::sync::atomic::AtomicU32::new(0),
            special_stones: DashMap::new(),
            monster_resources: DashMap::new(),
            item_random: DashMap::new(),
            item_groups: DashMap::new(),
            item_exchange_exp: DashMap::new(),
            item_give_exchange: DashMap::new(),
            item_right_click_exchange: DashMap::new(),
            item_right_exchange: DashMap::new(),
            event_room_manager: EventRoomManager::new(),
            manes_survival_manager: crate::systems::manes_survival::ManesSurvivalManager::default(),
            bdw_manager: parking_lot::RwLock::new(BdwManager::default()),
            monster_stone_manager: parking_lot::RwLock::new(MonsterStoneManager::new()),
            event_rewards: DashMap::new(),
            event_timer_show_list: parking_lot::RwLock::new(Vec::new()),
            game_info_notice1: parking_lot::RwLock::new(String::new()),
            game_info_notice2: parking_lot::RwLock::new(String::new()),
            ft_stages: parking_lot::RwLock::new(Vec::new()),
            ft_summons: parking_lot::RwLock::new(Vec::new()),
            forgotten_temple_state: ForgettenTempleState::new(),
            dd_stages: parking_lot::RwLock::new(Vec::new()),
            dd_monsters: parking_lot::RwLock::new(Vec::new()),
            dd_rooms: (1..=crate::handler::dungeon_defence::DD_MAX_ROOMS)
                .map(crate::handler::dungeon_defence::DdRoomInfo::new)
                .collect(),
            chaos_stone_spawns: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_summon_list: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_stages: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_rewards: parking_lot::RwLock::new(Vec::new()),
            chaos_stone_infos: parking_lot::RwLock::new(HashMap::new()),
            utc_spawns: parking_lot::RwLock::new(Vec::new()),
            under_the_castle_state: crate::handler::under_castle::UnderTheCastleState::new(),
            daily_quests: DashMap::new(),
            daily_rank_cache: parking_lot::RwLock::new(Vec::new()),
            draki_tower_stages: parking_lot::RwLock::new(Vec::new()),
            draki_monster_list: parking_lot::RwLock::new(Vec::new()),
            draki_tower_rooms: parking_lot::RwLock::new({
                let mut m = std::collections::HashMap::new();
                for i in 1..=crate::handler::draki_tower::EVENT_MAX_ROOM {
                    m.insert(i, DrakiTowerRoomInfo::new(i));
                }
                m
            }),
            new_char_set: DashMap::new(),
            new_char_value: DashMap::new(),
            knights_capes: DashMap::new(),
            castellan_bonuses: DashMap::new(),
            csw_opt: parking_lot::RwLock::new(None),
            monster_stone_respawn: parking_lot::RwLock::new(Vec::new()),
            monster_boss_random_stages: parking_lot::RwLock::new(Vec::new()),
            juraid_bridge_states: DashMap::new(),
            juraid_monument_respawns: DashMap::new(),
            monster_juraid_respawn: parking_lot::RwLock::new(Vec::new()),
            monster_challenge: parking_lot::RwLock::new(Vec::new()),
            monster_challenge_summon: parking_lot::RwLock::new(Vec::new()),
            sheriff_reports: crate::handler::sheriff::new_sheriff_map(),
            cindwar_settings: parking_lot::RwLock::new(Vec::new()),
            cindwar_items: parking_lot::RwLock::new(Vec::new()),
            cindwar_rewards: parking_lot::RwLock::new(Vec::new()),
            cindwar_reward_items: parking_lot::RwLock::new(Vec::new()),
            cindwar_stats: parking_lot::RwLock::new(Vec::new()),
            cindwar_active: std::sync::atomic::AtomicBool::new(false),
            cindwar_zone_id: AtomicU16::new(0),
            cindwar_event_users: DashMap::new(),
            cind_refresh_pending: DashMap::new(),
            ultima_boss_guard: DashMap::new(),
            ultima_seals: DashMap::new(),
            ultima_lethal: DashMap::new(),
            cindwar_player_states: DashMap::new(),
            cindwar_event_state: parking_lot::RwLock::new(
                crate::handler::cinderella::CindirellaEventState::default(),
            ),
            zindan_event_opened: std::sync::atomic::AtomicBool::new(false),
            zindan_war_state: parking_lot::RwLock::new(ZindanWarState::default()),
            pus_categories: DashMap::new(),
            pus_items_by_category: DashMap::new(),
            pus_items_by_id: DashMap::new(),
            soccer_state: crate::handler::soccer::new_soccer_state(),
            battle_state: parking_lot::RwLock::new(crate::systems::war::BattleState::new()),
            discount: std::sync::atomic::AtomicU8::new(0),
            npc_war_buffed: std::sync::atomic::AtomicBool::new(false),
            pvp_monument_nation: DashMap::new(),
            beef_event: parking_lot::RwLock::new(BeefEventState::default()),
            bowl_event_active: std::sync::atomic::AtomicBool::new(false),
            bowl_event_time: std::sync::atomic::AtomicU16::new(0),
            bowl_event_zone: std::sync::atomic::AtomicU8::new(0),
            daily_ops: DashMap::new(),
            plc_gm_socket: AtomicU32::new(u32::MAX),
            lua_engine: Arc::new(LuaEngine::new()),
            notice_board_entries: parking_lot::RwLock::new(Vec::new()),
            top_notice_entries: parking_lot::RwLock::new(Vec::new()),
            cape_bonus_entries: parking_lot::RwLock::new(Vec::new()),
            clan_premium_entries: parking_lot::RwLock::new(Vec::new()),
            right_top_titles: parking_lot::RwLock::new(Vec::new()),
            info_panel_entries: parking_lot::RwLock::new(Arc::new(Vec::new())),
            zone_kill_rewards: parking_lot::RwLock::new(Vec::new()),
            zone_online_rewards: parking_lot::RwLock::new(Vec::new()),
            wanted_rooms: parking_lot::RwLock::new(Default::default()),
            wanted_auto_enabled: std::sync::atomic::AtomicBool::new(false),
            wanted_map_show_time: std::sync::atomic::AtomicU64::new(0),
            login_in_progress: DashMap::new(),
            gm_list: parking_lot::RwLock::new(Vec::new()),
            db_pool: Some(pool.clone()),
            rate_limiter: crate::rate_limiter::RateLimiter::new(),
            lottery_process: crate::handler::lottery::new_lottery_process(),
            collection_race_event: crate::handler::collection_race::new_collection_race_event(),
            collection_race_settings: DashMap::new(),
            reward_rules: parking_lot::RwLock::new(Vec::new()),
            wheel_of_fun_settings: parking_lot::RwLock::new(Vec::new()),
            anti_afk_npc_ids: parking_lot::RwLock::new(Vec::new()),
            santa_or_angel: std::sync::atomic::AtomicU8::new(0),
            rank_bug: parking_lot::RwLock::new(ko_db::models::rankbug::RankBugConfig::default()),
            player_ranking_loyalty_reward: std::sync::atomic::AtomicU32::new(0),
            player_ranking_kc_reward: std::sync::atomic::AtomicU32::new(0),
            player_ranking_reward_zones: parking_lot::RwLock::new(vec![71, 72, 73]),
        };

        // Load all startup DB tables via the loading module.
        world.load_all_tables(pool, map_dir).await?;

        Ok(world)
    }

    // â”€â”€ Notice Board Accessors â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// Get a clone of the current notice board entries (title, message pairs).
    ///
    pub fn get_notice_board(&self) -> Vec<(String, String)> {
        self.notice_board_entries.read().clone()
    }

    /// Get a clone of the current top-right notice entries.
    ///
    pub fn get_top_notices(&self) -> Vec<String> {
        self.top_notice_entries.read().clone()
    }

    /// Replace the notice board entries (max 5 title+message pairs).
    ///
    /// from `Notice.txt`, paired as (title=even, message=odd).
    pub fn set_notice_board(&self, entries: Vec<(String, String)>) {
        let truncated: Vec<_> = entries.into_iter().take(5).collect();
        *self.notice_board_entries.write() = truncated;
    }

    /// Replace the top-right notice entries (max 20 messages).
    ///
    /// 20 lines from `Notice_up.txt`.
    pub fn set_top_notices(&self, entries: Vec<String>) {
        let truncated: Vec<_> = entries.into_iter().take(20).collect();
        *self.top_notice_entries.write() = truncated;
    }

    /// Get a clone of cape bonus notice entries.
    ///
    pub fn get_cape_bonus_entries(&self) -> Vec<(String, String)> {
        self.cape_bonus_entries.read().clone()
    }

    /// Replace the cape bonus notice entries (max 5 title+message pairs).
    ///
    /// from `CapeBonus.txt`, paired as (title=even, message=odd).
    pub fn set_cape_bonus_entries(&self, entries: Vec<(String, String)>) {
        let truncated: Vec<_> = entries.into_iter().take(5).collect();
        *self.cape_bonus_entries.write() = truncated;
    }

    /// Get a clone of clan premium notice entries.
    ///
    pub fn get_clan_premium_entries(&self) -> Vec<(String, String)> {
        self.clan_premium_entries.read().clone()
    }

    /// Replace the clan premium notice entries (max 5 title+message pairs).
    ///
    /// from `ClanPremiumNotice.txt`, paired as (title=even, message=odd).
    pub fn set_clan_premium_entries(&self, entries: Vec<(String, String)>) {
        let truncated: Vec<_> = entries.into_iter().take(5).collect();
        *self.clan_premium_entries.write() = truncated;
    }
    /// Get right-top title messages.
    ///
    pub fn get_right_top_titles(&self) -> Vec<(String, String)> {
        self.right_top_titles.read().clone()
    }

    /// Set right-top title messages (loaded from DB at startup).
    pub fn set_right_top_titles(&self, titles: Vec<(String, String)>) {
        *self.right_top_titles.write() = titles;
    }

    /// Admin-managed right-top info list entries (already sanitized, ordered).
    pub fn info_panel_entries(&self) -> Arc<Vec<crate::handler::info_panel::InfoPanelEntry>> {
        self.info_panel_entries.read().clone()
    }

    /// Replace the admin-managed right-top info list entries.
    pub fn set_info_panel_entries(&self, entries: Vec<crate::handler::info_panel::InfoPanelEntry>) {
        *self.info_panel_entries.write() = Arc::new(entries);
    }

    /// Set the permanent chat banner text and broadcast to all players.
    ///
    pub fn set_permanent_chat(&self, text: String) {
        *self.permanent_chat.write() = Some(text);
    }

    /// Clear the permanent chat banner.
    ///
    pub fn clear_permanent_chat(&self) {
        *self.permanent_chat.write() = None;
    }

    /// Get the current permanent chat text, if any.
    pub fn get_permanent_chat(&self) -> Option<String> {
        self.permanent_chat.read().clone()
    }

    // ── Banish of Winner Accessors ─────────────────────────────────────

    /// Get all banish-of-winner spawn entries for the given winning nation.
    ///
    pub fn get_banish_of_winner(&self, winner_nation: u8) -> Vec<ko_db::models::BanishOfWinner> {
        self.banish_of_winner
            .read()
            .iter()
            .filter(|b| b.nation_id == Some(winner_nation as i16))
            .cloned()
            .collect()
    }

    /// Set the banish-of-winner data (called once at startup).
    pub fn set_banish_of_winner(&self, data: Vec<ko_db::models::BanishOfWinner>) {
        *self.banish_of_winner.write() = data;
    }

    // ── Lottery Event Accessor ───────────────────────────────────────────

    /// Return a reference to the shared lottery process handle.
    ///
    pub fn lottery_process(&self) -> &crate::handler::lottery::SharedLotteryProcess {
        &self.lottery_process
    }

    // ── Collection Race Accessors ─────────────────────────────────────────

    /// Return a reference to the shared Collection Race event handle.
    ///
    pub fn collection_race_event(
        &self,
    ) -> &crate::handler::collection_race::SharedCollectionRaceEvent {
        &self.collection_race_event
    }

    /// Get the Collection Race event definition for the given index.
    ///
    pub fn get_collection_race_def(
        &self,
        index: i16,
    ) -> Option<crate::handler::collection_race::CrEventDef> {
        self.collection_race_settings.get(&index).map(|r| r.clone())
    }

    // ── Chaos Stone Info Accessors ────────────────────────────────────────

    /// Read-lock the runtime chaos stone info map.
    ///
    pub fn chaos_stone_infos(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, HashMap<u8, crate::handler::chaos_stone::ChaosStoneInfo>>
    {
        self.chaos_stone_infos.read()
    }
}

/// Convert a `ZoneInfoRow` from the DB to a `ZoneInfo` struct.
fn zone_info_from_row(row: &ZoneInfoRow) -> ZoneInfo {
    ZoneInfo {
        smd_name: row.smd_name.clone(),
        zone_name: row.zone_name.clone(),
        zone_type: ZoneAbilityType::from_i16(row.zone_type),
        min_level: row.min_level as u8,
        max_level: row.max_level as u8,
        // DB stores coords Ã— 100
        init_x: row.init_x as f32 / 100.0,
        init_z: row.init_z as f32 / 100.0,
        init_y: row.init_y as f32 / 100.0,
        abilities: ZoneAbilities {
            trade_other_nation: row.trade_other_nation,
            talk_other_nation: row.talk_other_nation,
            attack_other_nation: row.attack_other_nation,
            attack_same_nation: row.attack_same_nation,
            friendly_npc: row.friendly_npc,
            war_zone: row.war_zone,
            clan_updates: row.clan_updates,
            teleport: row.teleport,
            gate: row.gate,
            escape: row.escape,
            calling_friend: row.calling_friend,
            teleport_friend: row.teleport_friend,
            blink: row.blink,
            pet_spawn: row.pet_spawn,
            exp_lost: row.exp_lost,
            give_loyalty: row.give_loyalty,
            guard_summon: row.guard_summon,
            military_zone: row.military_zone,
            mining_zone: row.mining_zone,
            blink_zone: row.blink_zone,
            auto_loot: row.auto_loot,
            gold_lose: row.gold_lose,
        },
        status: row.status,
    }
}

/// Convert `GameEventRow` DB rows to a HashMap of `GameEvent` keyed by event_num.
fn events_from_rows(rows: &[GameEventRow]) -> HashMap<i16, GameEvent> {
    let mut map = HashMap::new();
    for row in rows {
        if let Some(event_type) = GameEventType::from_i16(row.event_type) {
            map.insert(
                row.event_num,
                GameEvent {
                    event_type,
                    cond: [row.cond1, row.cond2, row.cond3, row.cond4, row.cond5],
                    exec: [row.exec1, row.exec2, row.exec3, 0, 0],
                },
            );
        }
    }
    map
}

/// Get all valid cells in a 3Ã—3 grid centered on (rx, rz).
fn get_3x3_cells(rx: u16, rz: u16, max_x: u16, max_z: u16) -> HashSet<(u16, u16)> {
    let mut cells = HashSet::new();
    for dx in -1i16..=1 {
        for dz in -1i16..=1 {
            let nx = rx as i16 + dx;
            let nz = rz as i16 + dz;
            if nx >= 0 && nz >= 0 && (nx as u16) < max_x && (nz as u16) < max_z {
                cells.insert((nx as u16, nz as u16));
            }
        }
    }
    cells
}

// â”€â”€ Sprint 48: Stealth duration + Rivalry expiry tick helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€

impl WorldState {
    /// Collect sessions whose stealth duration has expired.
    ///
    /// When `tEndTime != -1 && UNIXTIME >= tEndTime`, the stealth is expired.
    ///
    /// Snapshot every currently registered session ID.
    ///
    /// Used by event shutdown cleanup so inventory sanitisation does not depend
    /// on an event participant set that may already have been updated by a zone
    /// change or disconnect.
    pub fn collect_session_ids(&self) -> Vec<SessionId> {
        self.sessions.iter().map(|entry| *entry.key()).collect()
    }

    /// Returns a list of session IDs whose stealth_end_time > 0 and <= now.
    pub fn collect_expired_stealths(&self, now_unix: u64) -> Vec<SessionId> {
        let mut expired = Vec::new();
        for entry in self.sessions.iter() {
            let h = entry.value();
            if h.invisibility_type != 0 && h.stealth_end_time > 0 && now_unix >= h.stealth_end_time
            {
                expired.push(*entry.key());
            }
        }
        expired
    }

    /// Collect alive, non-wanted players in a given zone, separated by nation.
    ///
    /// Returns `(elmorad_sids, karus_sids)`.
    pub fn collect_zone_alive_by_nation(&self, zone_id: u16) -> (Vec<SessionId>, Vec<SessionId>) {
        let mut elmo = Vec::new();
        let mut karus = Vec::new();
        for entry in self.sessions.iter() {
            let h = entry.value();
            if let Some(ref ch) = h.character {
                if h.position.zone_id == zone_id
                    && ch.hp > 0
                    && ch.res_hp_type != USER_DEAD
                    && !h.is_wanted
                {
                    if ch.nation == 2 {
                        // NATION_ELMORAD
                        elmo.push(*entry.key());
                    } else if ch.nation == 1 {
                        // NATION_KARUS
                        karus.push(*entry.key());
                    }
                }
            }
        }
        (elmo, karus)
    }

    /// Collect sessions whose rivalry has expired.
    ///
    /// `hasRival()` = `GetRivalID() >= 0`
    /// `hasRivalryExpired()` = `UNIXTIME >= m_tRivalExpiryTime`
    ///
    /// Returns a list of session IDs with active rivalry that has expired.
    pub fn collect_expired_rivalries(&self, now_unix: u64) -> Vec<SessionId> {
        let mut expired = Vec::new();
        for entry in self.sessions.iter() {
            let h = entry.value();
            if let Some(ref ch) = h.character {
                if ch.rival_id >= 0 && ch.rival_expiry_time > 0 && now_unix >= ch.rival_expiry_time
                {
                    expired.push(*entry.key());
                }
            }
        }
        expired
    }
}

#[cfg(test)]
impl WorldState {
    /// Insert a grade code row for testing.
    pub fn insert_test_make_grade_code(&self, row: MakeItemGradeCodeRow) {
        self.make_grade_codes.insert(row.item_index, row);
    }
    /// Insert a lare code row for testing.
    pub fn insert_test_make_lare_code(&self, row: MakeItemLareCodeRow) {
        self.make_lare_codes.insert(row.level_grade, row);
    }
    /// Insert a weapon row for testing.
    pub fn insert_test_make_weapon(&self, row: MakeWeaponRow) {
        self.make_weapons.insert(row.by_level, row);
    }
    /// Insert a defensive row for testing.
    pub fn insert_test_make_defensive(&self, row: MakeDefensiveRow) {
        self.make_defensives.insert(row.by_level, row);
    }
    /// Insert a make item row for testing.
    pub fn insert_test_make_item(&self, row: MakeItemRow) {
        self.make_items.insert(row.s_index, row);
    }
    /// Insert a new char set row for testing.
    pub fn insert_test_new_char_set(&self, race: i16, row: CreateNewCharSetRow) {
        self.new_char_set.entry(race).or_default().push(row);
    }
    /// Insert a new char value row for testing.
    pub fn insert_test_new_char_value(&self, key: (i16, i16), row: CreateNewCharValueRow) {
        self.new_char_value.insert(key, row);
    }
    /// Insert event reward rows for testing.
    pub fn insert_event_rewards(&self, local_id: i16, rows: Vec<EventRewardRow>) {
        self.event_rewards.insert(local_id, rows);
    }
    /// Insert a coefficient row for testing.
    pub fn insert_test_coefficient(&self, class: u16, row: CoefficientRow) {
        self.coefficients.insert(class, row);
    }
    /// Insert a set item row for testing.
    pub fn insert_test_set_item(&self, set_index: i32, row: SetItemRow) {
        self.set_items.insert(set_index, row);
    }
    /// Insert a knights cape row for testing.
    pub fn insert_test_knights_cape(&self, cape_index: i16, row: KnightsCapeRow) {
        self.knights_capes.insert(cape_index, row);
    }
    /// Insert a castellan bonus row for testing.
    pub fn insert_test_castellan_bonus(&self, bonus_type: i16, row: KnightsCapeCastellanBonusRow) {
        self.castellan_bonuses.insert(bonus_type, row);
    }

    /// Insert a quest monster row for testing.
    pub fn insert_test_quest_monster(&self, row: QuestMonsterRow) {
        self.quest_monsters.insert(row.s_quest_num as u16, row);
    }

    /// Insert a quest helper row for testing.
    pub fn insert_test_quest_helper(&self, row: QuestHelperRow) {
        self.quest_helpers.insert(row.n_index as u32, row);
    }
}

#[cfg(test)]
mod mod_tests;
