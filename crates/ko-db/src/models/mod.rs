//! Database model structs — each struct maps to a PostgreSQL table row.
//! All models derive `sqlx::FromRow` for automatic row mapping.

mod account;
mod achieve;
pub mod anti_afk_list;
pub mod banish_of_winner;
pub mod beginner_settings;
pub mod bot_system;
pub mod burning_features;
pub mod cash_shop;
pub mod chaos_stone;
pub mod char_creation;
mod character;
pub mod character_seal;
pub mod check_account;
pub mod cinderella;
pub mod clan_warehouse;
pub mod client_version;
mod coefficient;
pub mod collection_race;
pub mod costume;
pub mod daily_quest;
pub mod daily_rank;
pub mod daily_reward;
pub mod draki_tower;
pub mod dungeon_defence;
pub mod enchant;
pub mod event_beef_play_timer;
pub mod event_schedule;
pub mod forgotten_temple;
mod friend;
mod game_event;
pub mod game_master_settings;
pub mod game_options;
pub mod guild_bank;
pub mod hermetic_seal;
mod item;
pub mod item_tables;
pub mod item_upgrade_ext;
pub mod jackpot;
mod king;
pub mod king_nomination_result;
mod knights;
pub mod knights_auto;
pub mod knights_cape;
pub mod knights_castellan;
pub mod letter;
pub mod letter_gift;
pub mod level_merchant_rewards;
mod level_up;
pub mod lottery_event;
pub mod magic;
pub mod manes_survival;
mod mining;
pub mod monster_event;
pub mod monster_resource;
pub mod native_events;
mod npc;
mod object_event;
pub mod perk;
pub mod pet;
pub mod pet_talk;
pub mod ppcard;
pub mod premium;
mod quest;
pub mod quest_text;
pub mod rankbug;
pub mod ranking;
mod saved_magic;
pub mod scheduled_tasks;
mod server_info;
mod server_settings;
mod siege;
mod skill_shortcut;
pub mod soul;
mod start_position;
pub mod timed_notice;
pub mod under_castle;
pub mod user_data;
mod user_rental_item;
pub mod wheel_of_fun;
pub mod world_boss;
mod zindan_war_stages;
pub mod zindan_war_summon_list;
mod zone_info;
pub mod zone_rewards;

pub use account::{AccountChar, CurrentUser, TbUser};
pub use achieve::{
    AchieveComRow, AchieveMainRow, AchieveMonsterRow, AchieveNormalRow, AchieveTitleRow,
    AchieveWarRow, UserAchieveRow, UserAchieveSummaryRow,
};
pub use anti_afk_list::AntiAfkEntry;
pub use banish_of_winner::BanishOfWinner;
pub use beginner_settings::BeginnerSettings;
pub use bot_system::{
    BotHandlerFarmRow, BotHandlerMerchantRow, BotKnightsRankRow, BotMerchantDataRow,
    BotPersonalRankRow, UserBotRow,
};
pub use burning_features::BurningFeatures;
pub use cash_shop::{PusCategoryRow, PusItemRow, PusRefundRow};
pub use chaos_stone::{
    ChaosStoneSpawnRow, ChaosStoneSummonListRow, ChaosStoneSummonStageRow, EventChaosRewardRow,
};
pub use char_creation::{CreateNewCharSetRow, CreateNewCharValueRow};
pub use character::{
    TrashItemRow, UserData, UserDeletedItem, UserItem, VipWarehouseItemRow, VipWarehouseRow,
    WarehouseCoins, WarehouseItem,
};
pub use character_seal::{CharacterSealItemRow, CharacterSealMappingRow};
pub use check_account::CheckAccount;
pub use cinderella::{
    CindwarItemRow, CindwarRewardItemRow, CindwarRewardRow, CindwarSettingRow, CindwarStatRow,
};
pub use clan_warehouse::ClanWarehouseItemRow;
pub use coefficient::CoefficientRow;
pub use collection_race::{CollectionRaceReward, CollectionRaceSettings};
pub use daily_quest::{DailyQuestRow, DailyQuestStatus, DailyQuestTimeType, UserDailyQuestRow};
pub use daily_reward::{DailyReward, DailyRewardCumulative};
pub use draki_tower::{
    DrakiMonsterListRow, DrakiTowerRiftRankRow, DrakiTowerStageRow, UserDrakiTowerDataRow,
};
pub use dungeon_defence::{DfMonsterRow, DfStageRow};
pub use event_beef_play_timer::EventBeefPlayTimer;
pub use event_schedule::{
    EventOptFtRow, EventOptVroomRow, EventRewardRow, EventRoomPlayTimerRow, EventScheduleDayRow,
    EventScheduleMainRow, EventStartScheduleRow, EventStartTimeSlotRow, EventTimerShowRow,
    EventTriggerRow,
};
pub use forgotten_temple::{FtStageRow, FtSummonRow};
pub use friend::FriendRow;
pub use game_event::GameEventRow;
pub use game_master_settings::GameMasterSettings;
pub use game_options::GameOptions;
pub use item::Item;
pub use item_tables::{
    ItemExchangeExpRow, ItemExchangeRow, ItemGiveExchangeRow, ItemGroupRow, ItemOpRow,
    ItemRandomRow, ItemRightClickExchangeRow, ItemRightExchangeRow, ItemSellTableRow, ItemSmashRow,
    ItemSpecialSewingRow, ItemUpgradeRow, ItemUpgradeSettingsRow, MakeDefensiveRow,
    MakeItemGradeCodeRow, MakeItemGroupRandomRow, MakeItemGroupRow, MakeItemLareCodeRow,
    MakeItemRow, MakeWeaponRow, MiningExchangeRow, MonsterItemRow, NewUpgradeRow, NpcItemRow,
    RentalItemRow, SealedItemRow, SetItemRow, SpecialStoneRow, ITEMS_SPECIAL_EXCHANGE_GROUP,
};
pub use item_upgrade_ext::ItemUpProbabilityRow;
pub use jackpot::JackPotSettingRow;
pub use king::{
    KingCandidacyNoticeBoardRow, KingElectionListRow, KingNominationListRow, KingSystemRow,
};
pub use king_nomination_result::KingNominationResult;
pub use knights::{Knights, KnightsAllianceRow};
pub use knights_auto::KnightsAuto;
pub use knights_cape::{
    KnightsCapeCastellanBonusRow, KnightsCapeRow, KnightsCswOptRow, UserKnightDataRow,
};
pub use knights_castellan::KnightsCastellan;
pub use letter::LetterRow;
pub use letter_gift::LetterGift;
pub use level_merchant_rewards::LevelMerchantRewards;
pub use level_up::LevelUpRow;
pub use lottery_event::LotteryEventSettings;
pub use magic::{
    MagicRow, MagicType1Row, MagicType2Row, MagicType3Row, MagicType4Row, MagicType5Row,
    MagicType6Row, MagicType7Row, MagicType8Row, MagicType9Row,
};
pub use manes_survival::{ManesSurvivalMagicRow, ManesSurvivalSpawnRow};
pub use mining::MiningFishingItemRow;
pub use monster_event::{
    MonsterBossRandomStageRow, MonsterChallengeRow, MonsterChallengeSummonRow,
    MonsterJuraidRespawnRow, MonsterStoneRespawnRow,
};
pub use monster_resource::MonsterResource;
pub use npc::{
    MonsterBossRandomSpawnRow, MonsterRespawnLoopRow, MonsterSummonRow, NpcSpawnRow, NpcTemplateRow,
};
pub use object_event::ObjectEventRow;
pub use perk::{PerkRow, UserPerkRow, PERK_COUNT};
pub use pet::{PetImageChangeRow, PetStatsInfoRow, PetUserDataRow, PetUserItemRow};
pub use pet_talk::PetTalk;
pub use ppcard::PPCardRow;
pub use premium::{AccountPremiumRow, PremiumItemExpRow, PremiumItemRow};
pub use quest::{QuestHelperRow, QuestMonsterRow, UserQuestRow};
pub use quest_text::{
    QuestMenuRow, QuestSkillsClosedCheckRow, QuestSkillsClosedDataRow, QuestSkillsOpenSetUpRow,
    QuestTalkRow,
};
pub use ranking::UserRankRow;
pub use saved_magic::SavedMagicRow;
pub use scheduled_tasks::{AutomaticCommand, SendMessage};
pub use server_info::ServerInfo;
pub use server_settings::{DamageSettingsRow, HomeRow, ServerSettingsRow};
pub use siege::KnightsSiegeWarfareRow;
pub use skill_shortcut::SkillShortcutRow;
pub use start_position::{StartPositionRandomRow, StartPositionRow};
pub use timed_notice::TimedNoticeRow;
pub use under_castle::MonsterUnderTheCastleRow;
pub use user_data::{
    DailyRewardUserRow, UserDailyOpRow, UserGenieDataRow, UserLootSettingsRow, UserReturnDataRow,
    UserSealExpRow,
};
pub use user_rental_item::UserRentalItemRow;
pub use wheel_of_fun::{WheelOfFunItem, WheelOfFunSettings};
pub use zindan_war_stages::ZindanWarStageRow;
pub use zindan_war_summon_list::ZindanWarSummon;
pub use zone_info::ZoneInfoRow;
pub use zone_rewards::{ZoneKillReward, ZoneOnlineReward};
