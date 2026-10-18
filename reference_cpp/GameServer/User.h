#pragma once

#include "LuaEngine.h"
#include "../shared/KOSocket.h"
#include "MagicInstance.h"
#include "Unit.h"
#include "ChatHandler.h"

struct _KNIGHTS_USER;
struct _EXCHANGE_ITEM;
struct _USER_SEAL_ITEM;

typedef std::map<uint64, _USER_SEAL_ITEM*>	UserItemSealMap;
typedef	std::list<_EXCHANGE_ITEM*>			ItemList;
typedef CSTLMap <_USER_QUEST_INFO, uint16>	QuestMap;
typedef CSTLMap <_DAILY_USERQUEST, uint8>	DailyQuestMap;
typedef	std::map<uint32, _cooldown>			SkillCooldownList;
typedef	std::map<uint32, _castlist>			SkillCastList;
typedef	std::map<uint32, time_t>			PetSkillCooldownList;
typedef	std::map<uint8, _type_cooldown>			MagicTypeCooldownList;
typedef	std::map<uint32, ULONGLONG>			UserSavedMagicMap;

typedef std::map<uint8, int16> ItemBonusMap;
typedef std::map<uint8, ItemBonusMap> EquippedItemBonuses;

typedef std::vector<_ZONE_ONLINE_REWARD> ZoneOnlineReward;

// Time (in seconds) between each save request (10 min).
#define PLAYER_SAVE_INTERVAL			(10 * 60)
// Time (in milliseconds) between each skill request (-1 sec).
#define PLAYER_SKILL_REQUEST_INTERVAL	800 // milliseconds (ms)
// Time (in milliseconds) between each r hit request (-1 sec).
#define PLAYER_R_HIT_REQUEST_INTERVAL	900 // milliseconds (ms)
// Time (in milliseconds) between each skill request (-1 sec).
#define PLAYER_POTION_REQUEST_INTERVAL	2400 // milliseconds (ms)
// Time (in minute) for daily operations
#define DAILY_OPERATIONS_MINUTE			1440
// Time (in seconds) for nation monuments
#define NATION_MONUMENT_REWARD_SECOND	60

#define PLAYER_OFF_MERCHANT_INTERVAL (1*MINUTE)

// Time (in seconds) for stamina change
#define PLAYER_STAMINA_INTERVAL 2
// Time (in seconds) for training exp change
#define PLAYER_TRAINING_INTERVAL 15
// time genie updated
#define PLAYER_GENIE_INTERVAL	(1 * MINUTE)
#define PLAYER_FLASH_INTERVAL	(1 * MINUTE)
//time monster stone updated
#define PLAYER_MONSTER_STONE_INTERVAL	(30 * MINUTE)
#define PLAYER_MONSTER_STONE_KILL_INTERVAL	20

// Time for Check PPCard Serial Time
#define PPCARD_TIME (40)

enum class GameState
{
	NONE,
	GAME_STATE_CONNECTED,
	GAME_STATE_INGAME
};

enum MerchantState
{
	MERCHANT_STATE_NONE		= -1,
	MERCHANT_STATE_SELLING	= 0,
	MERCHANT_STATE_BUYING	= 1
};

enum BotState
{
	BOT_AFK = 0,
	BOT_MINING = 1,
	BOT_FISHING = 2,
	BOT_FARMER = 3,
	BOT_FARMERS = 4,
	BOT_MERCHANT = 5,
	BOT_DEAD = 6,
	BOT_MOVE = 7,
	BOT_MERCHANT_MOVE = 8,
	BOT_NPC_MOVE = 9
};

enum class WarpListResponse
{
	WarpListGenericError		= 0,
	WarpListSuccess				= 1,  // "You've arrived at."
	WarpListMinLevel			= 2,  // "You need to be at least level <level>."
	WarpListNotDuringCSW		= 3,  // "You cannot enter during the Castle Siege War."
	WarpListNotDuringWar		= 4,  // "You cannot enter during the Lunar War."
	WarpListNeedNP				= 5,  // "You cannot enter when you have 0 national points."
	WarpListWrongLevelDLW		= 6,  // "Only characters with level 30~50 can enter." (dialog) 
	WarpListDoNotQualify		= 7,  // "You can not enter because you do not qualify." (dialog) 
	WarpListRecentlyTraded		= 8,  // "You can't teleport for 2 minutes after trading." (dialog) 
	WarpListArenaFull			= 9,  // "Arena Server is full to capacity. Please try again later." (dialog) 
	WarpListFinished7KeysQuest	= 10, // "You can't enter because you completed Guardian of 7 Keys quest." (dialog) 
};

enum TeamColour
{
	TeamColourNone = 0,
	TeamColourBlue,
	TeamColourRed,
	TeamColourOutside,
	TeamColourMap
};

#define ARROW_EXPIRATION_TIME (2500) // 2.5 seconds

struct Arrow
{
	uint32 nSkillID;
	ULONGLONG tFlyingTime;

	Arrow(uint32 nSkillID, ULONGLONG tFlyingTime) 
	{
		this->nSkillID = nSkillID;
		this->tFlyingTime = tFlyingTime;
	}
};

typedef std::vector<Arrow> ArrowList;
typedef CSTLMap <_USER_ACHIEVE_INFO, uint16>	AchieveMap;
typedef CSTLMap <_PREMIUM_DATA>	PremiumMap;
typedef CSTLMap <_USER_ACHIEVE_TIMED_INFO>	AchieveTimedMap;
typedef CSTLMap <_DELETED_ITEM>	DeletedItemMap;
typedef CSTLMap <_PUS_REFUND, uint64> PusRefundMap;

#include "GameDefine.h"

class CGameServerDlg;
class CUser : public Unit, public KOSocket
{
public:
	virtual uint16 GetID() { return GetSocketID(); }
	bool newChar;
	void  BotUsingSkill(uint16 sTargetID, uint32 nSkillID);
	time_t testskillusetime;
	time_t testskillusetime2;
	time_t level_merchant_time;
	ULONGLONG m_BottomUserListTime, m_BeefExchangeTime;
	ULONGLONG m_iLastExchangeTime, m_surallinfotime, m_tLastDelayedCheck;
	time_t	XSafe_LASTCHECK, m_onlinecashtime;
	time_t off_merchantcheck;
	bool	m_bCharacterDataLoaded;
	bool m_bIsLoggingOut;
	bool m_addedmap;
	std::string & GetAccountName() { return m_strAccountID; }
	virtual std::string & GetName() { return m_strUserID; }

	void ZoneOnlineRewardCheck();
	void ZoneOnlineSendReward(_ZONE_ONLINE_REWARD pReward);
	void ZoneOnlineRewardStart();
	void ZoneOnlineRewardChange();
	void PlayerEffect(uint16 sEffect); // giri� efect eklendi DeaFSezo
	void CyberACS_SlavePriest(Packet& pkt);
	void OpenAkara(Packet& pkt);
	std::string	m_strAccountID, m_strUserID, m_strMemo;
	int64 nHardwareID;
	ULONGLONG m_SmashingTime;
	UserStatus behav_type;
	UserStatusBehaviour behav_status;
	EquippedItemBonuses m_equippedItemBonuses;
	std::recursive_mutex m_equippedItemBonusLock;

	/*bool m_sendknightflaglist;*/

	// Slave priest
	bool hasPriestBot;
	uint8 HealYuzde, Buffbool, AccBool, HealBool;

	short GetRepurchaseFreeID();
	void RepurchaseGiveIDBack(short RemoveID);
	std::vector<short> m_FreeRepurchaseID;
	std::recursive_mutex m_DeleteItemListLock;
	std::map<uint16, uint16> m_DeleteItemList;
	std::recursive_mutex m_FreeRepurchaseIDLock;
	_TBDATA pUserTbInfo;
	ULONGLONG m_pusgifttime;
	time_t m_gmsendpmtime;
	uint16 m_gmsendpmid;
	uint32 lastTickTime;
	uint32 lastTickTime2;
	uint8	m_testkillcount;
	uint16  m_KillCount;
	int		m_iDrakiTime;
	uint8	m_bDrakiStage;
	uint8	m_bDrakiEnteranceLimit;
	uint8	m_bRace;
	uint16	m_sClass;
	uint32	m_nHair;
	uint8	m_bRank;
	uint8	m_bTitle;
	int64	m_iExp;	
	uint32	m_iLoyalty,m_iLoyaltyMonthly;
	uint32	m_iMannerPoint;
	uint8	m_bFace;
	uint8	m_bCity;
	int16	m_bKnights;	
	int16	m_bKnightsReq;	
	uint8	m_bFame;
	int16	m_sHp, m_sMp, m_sSp;
	uint8	m_bStats[(uint8)StatType::STAT_COUNT];
	uint8	m_bRebStats[(uint8)StatType::STAT_COUNT];
	uint8	m_bAuthority;
	uint8	m_bAuthorityColor; // Renkli pm i�in geli�tirildi krala �zel renk felan 27.09.2020
	int16	m_sPoints; // this is just to shut the compiler up
	uint32	m_iGold, m_iBank;
	int16	m_sBind;
	int8    m_sUserUpgradeCount;
	uint8    m_bstrSkill[10];	

	//slave merchant
	uint8 isSlave;
	uint32 SlaveTotalKC;
	uint32 SlaveTotalCoins;
	uint32 SlaveTotalTime;
	uint32 SlaveItemID[12];
	uint16 SlaveItemCount[12];
	uint32 MerchantItemCount2[14], MerchantItem[14];

	std::string mytagname;
	uint32 m_tagnamergb;
	ULONGLONG m_lasttargetnumbertime;

	_KILLASSISTINFO pAssistKill;
	uint8 CrStage;
	_ITEM_DATA m_sItemArray[INVENTORY_TOTAL];
	_ITEM_DATA m_sWarehouseArray[WAREHOUSE_MAX];
	_ITEM_DATA m_sVIPWarehouseArray[VIPWAREHOUSE_MAX];
	time_t m_zoneonrewardtime;
	bool	   m_sLevelArray[83];

	std::string m_bVIPStoragePassword;
	uint8 m_bWIPStorePassowrdRequest;
	uint32 m_bVIPStorageVaultExpiration;

	uint32 m_bAchieveLoginTime;

	uint8	m_bLogout;
	uint32	m_dwTime;
	time_t	m_lastSaveTime;
	time_t  m_petrespawntime;
	time_t	m_ClanPremiumTime; 
	time_t	m_lastBonusTime;
	time_t	m_TimeOnline;
	uint32	m_EventDisandTime;
	time_t	m_tGenieTimeNormal, m_flashchecktime;
	time_t  m_tlastoffmerchanttime;
	ULONGLONG  m_TownTime;

	bool m_dailyrankrefresh[(int)DailyRankType::COUNT];

	uint16  m_offmerchanttime;

	uint8	m_bAccountStatus;
	uint8	m_bPremiumType[5];
	uint16	m_sPremiumTime[5];

	bool	m_bWaitingOrder;
	time_t	m_tOrderRemain;

	bool	m_autoloot;
	bool	m_fairycheck;

	time_t	m_event_afkcheck;
	uint8 TimeTravelTemp;
	uint32  m_nKnightCash;
	uint32  m_nTLBalance;
	uint8 priesterrorcount;
	time_t	m_1098GenieTime;
	uint8   m_sFirstUsingGenie;
	char	m_GenieOptions[100];
	bool	m_bGenieStatus;

	bool	m_iswanted;
	uint32  m_iswantedtime;

	int16   m_TowerOwnerID;

	int16	m_oldTargetID;
	// Mute-Attack Related
	int32 m_mutetimestatus, m_attacktimestatus;
	
	uint32 m_flashtime;
	uint8  m_flashcount, m_flashtype;

	//Civciv 30 level 1 kere kontrol
	uint8 ChickenStatus;

	// Achieve System
	_USER_ACHIEVE_SUMMARY m_AchieveInfo;
	uint16 m_sCoverID, m_sCoverTitle, m_sSkillID, m_sSkillTitle;
	AchieveMap m_AchieveMap;
	AchieveTimedMap m_AchieveTimedMap;

	time_t	m_lastStaminaTime;
	time_t	m_lastTrainingTime;
	uint32  m_iTotalTrainingExp;
	uint32  m_iTotalTrainingTime;

	int AutoGemCount;
	uint8 AutoGemSell;
	uint8 AutoGemIn;
	bool AutoGemStart;

	uint8 killmoney , killuser;

	time_t	m_lastPetSatisfaction, m_AutoMiningFishingSystem;
	time_t  m_SlavePriestSystem;
	PET_DATA* m_PettingOn;

	bool	m_bSelectedCharacter;
	bool	m_bStoreOpen;

	int8	m_bMerchantState;
	bool	m_bSellingMerchantPreparing;
	bool	m_bBuyingMerchantPreparing;
	int16	plookersocketid;
	int16	m_sMerchantsSocketID;
	void GetMerchantSlipList(_MERCH_DATA list[MAX_MERCH_ITEMS], CBot* pownermerch);
	_MERCH_DATA	m_arMerchantItems[MAX_MERCH_ITEMS];
	std::list<uint16>	m_arMerchantLookers;
	bool	m_bPremiumMerchant;
	UserItemSealMap m_sealedItemMap;
	uint8	m_bRequestingChallenge, // opcode of challenge request being sent by challenger
		m_bChallengeRequested;  // opcode of challenge request received by challengee
	int16	m_sChallengeUser;

	char	m_strSkillData[320];
	uint16  m_strSkillCount;

	// Event System
	uint32 m_iEventJoinOrder;
	int16 m_sJoinedEvent;
	bool   m_isFinalJoinedEvent;
	time_t m_pusrefundtime;
	ULONGLONG m_CraftingTime;
	// Rival system
	int16	m_sRivalID;			// rival's session ID
	time_t	m_tRivalExpiryTime;	// time when the rivalry ends

	// Anger gauge system 
	uint8	m_byAngerGauge; // values range from 0-5

	// Training
	uint32 m_iTrainingTime[2];

	// Magic System Cooldown checks
	SkillCooldownList m_CoolDownList;
	std::recursive_mutex	m_lockCoolDownList;

	PetSkillCooldownList m_PetSkillCooldownList;
	SkillCastList		m_SkillCastList;
	std::recursive_mutex	m_SkillCastListLock;
	ULONGLONG				t_timeLastPotionUse;

	_USER_LOOT_SETTING pUserLootSetting;

	// Magic System Same time magic type checksfv
	std::recursive_mutex m_MagicTypeCooldownListLock;
	MagicTypeCooldownList  m_MagicTypeCooldownList;

	ArrowList m_flyingArrows;
	ArrowList m_flyingArrowsSuccess;
	std::recursive_mutex m_arrowLock;
	DeletedItemMap m_sDeletedItemMap;
	PusRefundMap m_PusRefundMap;

	std::recursive_mutex m_serialitemlistLock;
	std::vector<uint64> m_serialitemlist;

	bool	isBottomSign;
	bool	m_bIsChicken; // Is the character taking the beginner/chicken quest?
	bool	m_bIsHidingHelmet;
	bool	m_bIsHidingCospre;
	bool	m_bIsHidingWings;
	bool	KnightRoyaleStatiMaxHp;
	bool	KnightRoyaleStatiMaxMp;
	// 1vs1 Event //
	bool m_VsDurumu;
	std::string VsYapilacakKullanici;
	uint32 VsMiktari;
	uint16 VsKillMiktari;
	uint8 YourKill;
	// 1vs1 Event //
	uint32  ReturnSymbolisOK;
	int64   ReturnisLogOutTime;
	int64   ReturnSymbolTime;
	uint8	m_bPremiumInUse;
	uint8	m_bClanPremiumInUse;
	PremiumMap	m_PremiumMap;
	uint32   m_iSealedExp;
	uint8	 bExpSealStatus;
	uint8	 m_sUserPartyType;

	bool	m_bMining;
	bool	m_bFishing;
	ULONGLONG	m_tLastMiningAttempt; //Seri mining&fishing yapma fix.
	time_t	m_tUpgradeDelay; //Seri Upgrade Fix
	bool	m_bSlaveMerchant;
	int16	m_bSlaveUserID;
	

	// usere ait tbl verileri
	std::string itemorg, skillmagic, zones, skillmagictk, srcversion; // dllversion uyumsuz ise dc eder

	int8	m_bPersonalRank;
	int8	m_bKnightsRank;

	float	m_oldx, m_oldy, m_oldz;
	uint16	m_oldwillx, m_oldwilly, m_oldwillz;
	uint16  curX1, curY1, curZ1;
	int16	m_sDirection;

	int64	m_iMaxExp;
	uint16  sKilledCount;
	uint32	m_sMaxWeight;
	uint16	m_sMaxWeightBonus;

	int16   m_sSpeed;

	uint8	m_bPlayerAttackAmount;
	uint8	m_bAddWeaponDamage;
	uint16	m_sAddArmourAc; 
	uint8	m_bPctArmourAc;

	int16	m_sItemMaxHp;
	int16	m_sItemMaxMp;
	uint32	m_sItemWeight;
	short	m_sItemAc;
	short	m_sItemHitrate;
	short	m_sItemEvasionrate;

	uint8	m_byAPBonusAmount;
	uint8	m_byAPClassBonusAmount[4]; // one for each of the 4 class types
	uint8	m_byAcClassBonusAmount[4]; // one for each of the 4 class types

	int16	m_sStatItemBonuses[(int16)StatType::STAT_COUNT];
	int16	m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_COUNT];
	bool	AchieveChalengeCount;
	int8	m_bStatBuffs[(int8)StatType::STAT_COUNT];

	uint16	m_sExpGainbuff11Amount, m_sExpGainbuff33Amount;
	uint8	m_bItemExpGainAmount;
	uint8	m_bNPGainAmount, m_bItemNPBonus, m_bSkillNPBonus;
	uint8	m_bNoahGainAmount, m_bItemNoahGainAmount;
	uint8	m_bMaxWeightAmount;
		
	uint8   m_FlashExpBonus;
	uint8   m_FlashDcBonus;
	uint8   m_FlashWarBonus;
	uint8	m_ClanExpBonus;	// Clan Premium
	uint8	m_ClanNPBonus;	// Clan Premium

	bool m_reloadChestBlock;
	std::recursive_mutex mChestBlockItemLock;
	std::vector<uint32> mChestBlockItem;

	uint16  m_drop_scroll_amount;

	short	m_MaxHp, m_MaxMp, m_MaxSp;

	uint8	m_bResHpType;
	bool	m_bWarp, m_bCheckWarpZoneChange;
	uint8	m_bNeedParty;

	int16	m_sPartyIndex;
	bool	m_bInParty;
	bool    m_bInEnterParty;
	bool	m_bPartyLeader;
	bool	m_bPartyCommandLeader;

	uint8 m_jackpotype;

	//clanbank
	bool	sClanPremStatus;

	// JstKO GM Full Upgrade %100 �zel Eklendi 05.01.2025
	bool GMUpgradeThis;

	bool	m_bCanSeeStealth;
	uint8	m_bInvisibilityType;

	short	m_sExchangeUser;
	int8     m_sTradeStatue;
	bool	m_isRequestSender;
	ItemList	m_ExchangeItemList;

	bool	m_bBlockPrivateChat;
	uint16	m_sPrivateChatUser;

	time_t	m_tHPLastTimeNormal;					// For Automatic HP recovery. 
	time_t	m_tHPStartTimeNormal;
	time_t	m_tSpLastTimeNormal;					// For Automatic SP recovery. 
	time_t	m_tSpStartTimeNormal;

	uint32 m_randomItem;

	//PPCard Time
	time_t  PPCardTime;

	short	m_bHPAmountNormal;
	uint8	m_bHPDurationNormal;
	uint8	m_bHPIntervalNormal;
	uint8	m_bSpIntervalNormal;

	uint32 m_JobChangeTime;
	uint8 SkillFailCount;

	uint32	m_fSpeedHackClientTime, m_fSpeedHackServerTime;
	uint8	m_bSpeedHackCheck;

	time_t	m_tBlinkExpiryTime;			// When you should stop blinking.

	ULONGLONG   m_tLastArrowUse;

	uint32	m_bAbnormalType;			// Is the player normal, a giant, or a dwarf?
	uint32	m_nOldAbnormalType;

	int16	m_sWhoKilledMe;				// ID of the unit that killed you.
	int64	m_iLostExp;					// Experience points that were lost when you died.

	time_t	m_tLastTrapAreaTime;		// The last moment you were in the trap area.
	bool	m_bZoneChangeFlag;
	bool	m_bZoneChangeControl;
	uint8	m_switchPremiumCount;
	uint8	m_bRegeneType;				// Did you die and go home or did you type '/town'?

	time_t	m_tLastRegeneTime;			// The last moment you got resurrected.

	bool	m_bZoneChangeSameZone;		// Did the server change when you warped?

	bool	m_bHasAlterOptained;
	char	s_FlashNotice[128];

	int		m_iSelMsgEvent[MAX_MESSAGE_EVENT];
	uint8	m_bSelectMsgFlag;
	short	m_sEventNid, m_sEventSid;
	uint32	m_nQuestHelperID;

	time_t m_sMilitaryTime;
	bool	m_clanntsreq;

	bool	m_bWeaponsDisabled;
	bool	m_ismsevent;
	TeamColour	m_teamColour;

	// JstKO Yeni Cz Pvp Kill �d�l Notice Eklendi. 12.04.2025
	uint16    m_PvpKillCount;
	uint16    m_PvpKill50Count;
	uint16    m_PvpKill500Count;
	// JstKO Yeni Cz Pvp Kill �d�l Notice Eklendi. The End. 12.04.2025

	uint16 to_gm_pmName;

	uint16	m_flameilevel;
	time_t  m_flametime;
	uint32 m_BorderDefenceWarUserPoint;
	uint32 m_ChaosExpansionKillCount;
	uint32 m_ChaosExpansionDeadCount;
	uint32 m_PlayerKillingLoyaltyDaily;
	uint16 m_PlayerKillingLoyaltyPremiumBonus;

	ULONGLONG m_lastrattacktime;
	uint8 autoupscrolltype;
	uint8 m_AutoUpgradeisFast;
	uint16	 m_AutoUpgradeCount;

	uint16    m_ChatRoomIndex;

	uint8  m_LastSeen[2]; // [0] hour, [1] minute

	uint8 m_GameMastersMute
		, m_GameMastersUnMute
		, m_GameMastersUnBan
		, m_GameMastersBanPermit
		, m_GameMastersBanUnder
		, m_GameMastersBanDays
		, m_GameMastersBanCheating
		, m_GameMastersBanScamming
		, m_GameMastersAllowAttack
		, m_GameMastersDisabledAttack
		, m_GameMastersNpAdd
		, m_GameMastersExpAdd
		, m_GameMastersMoneyAdd
		, m_GameMastersDropAdd
		, m_GameMastersLoyaltyChange
		, m_GameMastersExpChange
		, m_GameMastersMoneyChange
		, m_GameMastersGiveItem
		, m_GameMastersGiveItemSelf
		, m_GameMastersSummonUser
		, m_GameMastersTpOnUser
		, m_GameMastersZoneChange
		, m_GameMastersLocationChange
		, m_GameMastersMonsterSummon
		, m_GameMastersNpcSummon
		, m_GameMastersMonKilled
		, m_GameMastersTeleportAllUser
		, m_GameMastersClanSummon
		, m_GameMastersResetPlayerRanking
		, m_GameMastersChangeEventRoom
		, m_GameMastersWarOpen
		, m_GameMastersWarClose
		, m_GameMastersCaptainElection
		, m_GameMastersCastleSiegeWarClose
		, m_GameMastersSendPrsion
		, m_GameMastersKcChange 
		, m_GameMastersReloadTable
		, m_GameMastersDropTest;

	float    m_LastX;
	float    m_LastZ;
	int			m_SpeedHackTrial;
	ULONGLONG	m_tLastChatUseTime;
	ULONGLONG	m_tLastType4SkillCheck;
	ULONGLONG m_tLastType69SkillCheck;
	_USER_LAST_MAGIC_USED pUserMagicUsed;
	_MOVE pMove;
	uint8 m_bRebirthLevel;

	_USER_DAILY_RANK pUserDailyRank;

	struct _LIFE_SKILLS 
	{
		uint64 WarExp;
		uint64 HuntingExp;
		uint64 SmitheryExp;
		uint64 KarmaExp;
	};

	enum class LIFE_SKILL 
	{
		WAR = 1,
		HUNTING,
		SMITHERY,
		KARMA
	};

	_LIFE_SKILLS m_lifeSkills;

	bool m_lifeSkillsLoaded;

public:
	/* <XSafe Handles Defines> */
	// Life Skills
	struct LF_VEC2
	{
		uint64 EXP;
		uint64 TargetEXP;
		LF_VEC2(uint64 _EXP, uint64 _TargetEXP)
		{
			EXP = _EXP;
			TargetEXP = _TargetEXP;
		}
	};

	void LifeSkillControl()
	{
		if (m_lifeSkills.WarExp > 500000000) m_lifeSkills.WarExp = 500000000;
		if (m_lifeSkills.HuntingExp > 500000000) m_lifeSkills.HuntingExp = 500000000;
		if (m_lifeSkills.SmitheryExp > 500000000) m_lifeSkills.SmitheryExp = 500000000;
		if (m_lifeSkills.KarmaExp > 500000000) m_lifeSkills.KarmaExp = 500000000;
	}

	INLINE uint8 GetLifeSkillLevelFromEXP(int64 EXP)
	{
		double multipier = 1.15;

		uint8 level = 1;
		double tmp = 100;
		int64_t TotalExp = 0;

		if (EXP < tmp)
			return level;

		for (;;)
		{
			tmp *= multipier;
			TotalExp = (int64_t)tmp;
			level++;
			if (TotalExp > EXP)
				break;
		}

		return level;
	}

	INLINE LF_VEC2 GetLifeSkillProgress(int64 EXP)
	{
		uint8 level = GetLifeSkillLevelFromEXP(EXP) + 1;

		double targetEXP = 100;

		for (int i = 0; i < level; i++)
			targetEXP *= 1.15;

		uint64_t TotalExp = (uint64_t)targetEXP;

		return LF_VEC2(EXP, TotalExp);
	}

	INLINE uint8 GetLifeSkillLevel(LIFE_SKILL skill)
	{
		LifeSkillControl();
		switch ((LIFE_SKILL)skill)
		{
		case LIFE_SKILL::WAR:
			return GetLifeSkillLevelFromEXP(m_lifeSkills.WarExp);
			break;
		case LIFE_SKILL::HUNTING:
			return GetLifeSkillLevelFromEXP(m_lifeSkills.HuntingExp);
			break;
		case LIFE_SKILL::SMITHERY:
			return GetLifeSkillLevelFromEXP(m_lifeSkills.SmitheryExp);
			break;
		case LIFE_SKILL::KARMA:
			return GetLifeSkillLevelFromEXP(m_lifeSkills.KarmaExp);
			break;
		default:
			return 1;
			break;
		}
	}
	INLINE LF_VEC2 GetLifeSkillCurrent(LIFE_SKILL skill)
	{
		LifeSkillControl();
		switch (skill)
		{
		case LIFE_SKILL::WAR:
			return GetLifeSkillProgress(m_lifeSkills.WarExp);
			break;
		case LIFE_SKILL::HUNTING:
			return GetLifeSkillProgress(m_lifeSkills.HuntingExp);
			break;
		case LIFE_SKILL::SMITHERY:
			return GetLifeSkillProgress(m_lifeSkills.SmitheryExp);
			break;
		case LIFE_SKILL::KARMA:
			return GetLifeSkillProgress(m_lifeSkills.KarmaExp);
			break;
		default:
			return LF_VEC2(0, 100);
			break;
		}
	}
	// globals
	uint32 m_mackey;
	void XSafe_Main();
	void WareHouseAddItemProcess(uint32 ItemID, uint32 Count);
	uint8_t CheckEmptyWareHouseSlot();

	void WantedEventUserisMove();
	void WantedProcess(Packet& pkt);
	void WantedRegister(Packet& pkt);
	void NewWantedEventLoqOut(Unit *pKiller = nullptr);
	void CyberACS_BotPriestSystem();
	float getplusdamage();
	int16	m_bUserPriestBotID;
	// handlers
	void XSafe_HandlePacket(Packet& pkt);
	void XSafe_General(Packet& pkt);
	void XSafe_StayAlive(Packet& pkt);
	void XSafe_Reset(Packet& pkt);
	void XSafe_AuthInfo(Packet& pkt);
	void XSafe_ProcInfo(Packet& pkt);
	void XSafe_Log(Packet& pkt);
	void XSafe_UIRequest(Packet& pkt);
	void XSafe_PusRequest(Packet& pkt);
	void XSafe_PUSPurchase(Packet& pkt);
	void PUSGiftPurchase(Packet&pkt);
	void XSafe_DropRequest(Packet& pkt);
	void XSafe_ReqUserInfo(Packet& pkt);
	void XSafe_SaveLootSettings(Packet& pkt);
	void XSafe_ChaoticExchange(Packet& pkt);
	void SendChaoticResult(uint8 result);
	void XSafe_Merchant(Packet& pkt);
	void XSafe_ReqUserData();
	void XSafe_SendTempItems();
	void XSafe_Support(Packet& pkt);
	void XSafe_ItemNotice(uint8 type, uint32 nItemID);
	void XSafe_ReqMerchantList(Packet& pkt);
	void XSafe_SendMessageBox(std::string title, std::string message);
	void XSafe_SendLifeSkills();
	void XSafe_HandleLifeSkill(Packet& pkt);
	void XSafe_LotteryJoinFunction(Packet &pkt);
	void XSafe_AccountInfoSave(Packet& pkt);
	void xSafe_SendAccountRegister();
	void XSafe_LastSeenProcess(Packet& pkt);
	void XSafe_SendRepurchaseMsg(Packet& pkt);
	void XSafe_Send1299SkillAndStatReset(Packet& pkt);
	void RebirthBas();
	void SlaveMerc(Packet& pkt);
	void TagChange(Packet &pkt);
	void HandleTagChange(Packet &ptk);
	void UserInOutTag(std::vector<CUser*> mlist);

	bool isBridgeStage1(_JURAID_ROOM_INFO* pRoomInfo);
	bool isBridgeStage2(_JURAID_ROOM_INFO* pRoomInfo);
	bool isDevaStage(_JURAID_ROOM_INFO* pRoomInfo);

	//Clan Nation Transfer
	void ClanNtsHandler();
	void ReqClanNts();
	void ReqNtsCommand(Packet &pkt);
	void ClanNtsSendMsg(cntscode p, std::string strUserID = "");

	void UserNameChangeInsertLog(std::string oldname, std::string newname);
	void ClanNameChangeInsertLog(std::string oldname, std::string newname);
	void NationTransferInsertLog(uint8 oldnation, uint8 newnation);
	void JobChangeInsertLog(uint16 oldClass, uint16 newClass, uint16 oldRace, uint16 newRace);
	void PusShoppingInsertLog(uint32 itemid, uint16 itemcount, uint32 itemcashcount);
	void ChatInsertLog(uint8 chattype, std::string descp, std::string message, CUser* ptarget);
	void NpcShoppingLog(std::vector<ITEMS> p, uint16 protoid, uint8 Type);
	void LoginInsertLog();
	void LogoutInsertLog();
	std::string Disconnectprintfstyle();
	void Disconnectprintfwriter(std::string fonkname, std::string descp, int dccode);
	void GiveItemInsertLog(std::string placeowned, uint32 itemid, uint16 itemcount, CUser* powned);
	void RobItemInsertLog(uint32 itemid, uint16 itemcount, uint8 pos, CUser* powned);
	void MerchantCreationInsertLog(uint8 merchtype);
	void ItemRemoveInsertLog(uint32 itemid);
	void MerchantShoppingDetailInsertLog(bool bot, uint8 merchtype, uint32 itemid, uint16 itemcount, uint32 itemprice, CUser* pCostumer);
	void KillingUserInsertLog(CUser* pdeaduser);
	void KillingNpcInsertLog(CNpc* pdeadnpc);
	void UpgradeInsertLog2(uint32 itemid, uint32 money, uint8 uptype, bool status, uint32 reqitemid, uint16 reqitemcount);
	void UpgradeInsertLog(uint32 itemid, uint32 money, uint8 uptype, bool status, uint32 reqitemid[9], uint16 reqitemcount[9]);
	void NpcDropReceivedInsertLog(uint32 itemid, uint16 itemcount, uint16 npcid);
	void PremiumInsertLog(uint8 premiumtype, uint32 premiumtime);
	void TradeInsertLog(CUser* ptarget, uint32 money, uint32 tmoney, uint32 itemid[9], uint16 itemcount[9], uint32 titemid[9], uint16 titemcount[9]);
	void LoyaltyChangeInsertLog(std::string placeowned, uint32 currentloyalty, uint32 amount, uint32 amountend, uint32 finalloyalty);
	void ClanBankInsertLog(CKnights *pKnights, uint32 itemid, uint32 count, uint32 money, bool add);
	void ExpChangeInsertLog(int64 amount, int64 bmount, std::string decp);

	void Send_myPerks();
	void HandlePerks(Packet& pkt);
	void PerkPlus(Packet& pkt);
	void PerkReset(Packet& pkt);
	bool PerkUseItem(uint32 itemid,uint32 itemcount, uint16 perk);
	void PerkTargetInfo(Packet& pkt);
	void SendPerkError(perksError error);
	void   SendBoxMessage(uint8 type = 0, std::string title = "", std::string message = "", uint8 time = 0, messagecolour a = messagecolour::red);
	void HandleItemReturn(Packet&pkt);
	void ItemReturnSendErrorOpcode(pusrefunopcode p);
	bool PusRefundPurchase(uint64 serial, uint32 itemid, uint16 itemcount, uint32 itemprice, _ITEM_TABLE ptable, uint8 buytype);

	void SendAcsMessage(std::string message);
	// senders
	void XSafe_SendAliveRequest();
	void XSafe_SendPUS();
	void XSafe_OpenWeb(std::string url);
	void XSafe_SendProcessInfoRequest(CUser* toWHO = nullptr);
	/* </XSafe Handles> */

	_PERKS_DATA pPerks;

	_CINDWARUSER pCindWar;
	int8   get_cindclassindex();
	uint8  CindirellaJoin(int8 iclass);
	uint8  CindirellaGetNewClass(uint8 iclass, uint8 systemlevel);
	uint8  CindirellaGetNewRace(uint8 iclass);
	bool   CindirellaSign();
	bool   CindirellaChaModify(bool regene = false, bool nation = false);
	void   CindirellaLogOut(bool exitgame = false);
	void   CindirellaSendFinish();
	void   CindireallaUpdateKDA(CUser* pKiller);
	void   CindirillaHandler(Packet& pkt);
	void   CindirellaSelectClass(Packet& pkt);
	void   CindirellaNationChange(Packet& pkt);
	bool   IsCindIn(uint8 zoneid = 0);
	void   SendCindSelectError(cindopcode a = cindopcode::timewait, bool nation = false);

	// Get info for NPCs in regions around user (WIZ_REQ_NPCIN)
	void NpcInOutForMe();

	bool PusArrayControl;
	bool TempItemsControl;

	INLINE bool isMeChatRoom(uint16 Room) { return m_ChatRoomIndex == Room; };
	INLINE bool  isGenieActive(){ return m_bGenieStatus;}

	INLINE uint16 GetGenieTime() { 
		int hour = int(m_1098GenieTime > UNIXTIME ? m_1098GenieTime - UNIXTIME : 0);
		if (hour <= 0)   return 0;
		if (hour < 3600) return 1;

		double test = hour / static_cast<double>(3600);
		return (uint16)round(test);
	}
	INLINE uint16 GetAchieveLoginTime() { return m_bAchieveLoginTime; }

	INLINE bool isMuted() { return m_mutetimestatus > (int32)UNIXTIME || m_mutetimestatus == -1; }
	INLINE bool isAttackDisabled() { return m_attacktimestatus > (int32)UNIXTIME || m_attacktimestatus == -1; }
	INLINE bool isGM() { return GetAuthority() == (uint8)AuthorityTypes::AUTHORITY_GAME_MASTER; }
	INLINE bool isGMUser() { return GetAuthority() == (uint8)AuthorityTypes::AUTHORITY_GM_USER; } //anindagm i�in

	virtual bool isDead() { return m_bResHpType == USER_DEAD || m_sHp <= 0; }
	virtual bool isBlinking() { return m_bAbnormalType == ABNORMAL_BLINKING; }
	INLINE bool isInGame() { return GetState() == GameState::GAME_STATE_INGAME; }

	INLINE bool isInAutoClan() { return GetClanID() == 1 || GetClanID() == 15001; }

	INLINE bool isInParty() { return m_bInParty && m_bInEnterParty; }
	INLINE bool isInSameParty(CUser* pTarget) { return m_sPartyIndex == pTarget->m_sPartyIndex && m_sPartyIndex != -1; }
	INLINE bool isInApprovedParty() { return m_bInParty; }
	INLINE bool isPartyLeader() { return isInParty() && m_bPartyLeader; }
	INLINE bool isPartyCommandLeader() { return m_bPartyCommandLeader; }

	INLINE bool isInClan() { return GetClanID() > 0; }
	INLINE bool isKing() { return m_bRank == 1; }
	INLINE bool isClanLeader() { return GetFame() == CHIEF; }
	INLINE bool isClanAssistant() { return GetFame() == VICECHIEF; }
	INLINE bool isSlaveMerchant() { return m_bSlaveMerchant; }
	INLINE int16 GetSlaveGetID() { return m_bSlaveUserID; }
	INLINE bool isInValidRoom(uint8 type, uint16 roomid = 0) {
		/* 0: temple room
		1: stone room
		2: draki room */
		uint16 v_roomid = roomid > 0 ? roomid : GetEventRoom();
		if (type == 0)
			return isInTempleEventZone() && v_roomid > 0 && v_roomid <= MAX_TEMPLE_EVENT_ROOM;
		else if (type == 1)
			return isInMonsterStoneZone() && v_roomid > 0 && v_roomid <= MAX_MONSTER_STONE_ROOM;
		return false;
	}

	INLINE bool isWarrior() { return JobGroupCheck((short)ClassType::ClassWarrior); }
	INLINE bool isRogue() { return JobGroupCheck((short)ClassType::ClassRogue); }
	INLINE bool isMage() { return JobGroupCheck((short)ClassType::ClassMage); }
	INLINE bool isPriest() { return JobGroupCheck((short)ClassType::ClassPriest); }
	INLINE bool isPortuKurian() { return JobGroupCheck((short)ClassType::ClassPortuKurian); }
	INLINE bool isBeginner() 
	{
		uint16 sClass = GetClassType();
		return (sClass <= (short)ClassType::ClassPriest
			|| sClass == (short)ClassType::ClassPortuKurian);
	}

	INLINE bool isBeginnerWarrior() { return GetClassType() == (uint8)ClassType::ClassWarrior; }
	INLINE bool isBeginnerRogue()   { return GetClassType() == (uint8)ClassType::ClassRogue; }
	INLINE bool isBeginnerMage()    { return GetClassType() == (uint8)ClassType::ClassMage; }
	INLINE bool isBeginnerPriest()  { return GetClassType() == (uint8)ClassType::ClassPriest; }
	INLINE bool isBeginnerKurianPortu() { return GetClassType() == (uint8)ClassType::ClassPortuKurian; }

	INLINE bool isNovice() 
	{
		uint16 sClass = GetClassType();
		return (sClass == (uint16)ClassType::ClassWarriorNovice
			|| sClass == (uint16)ClassType::ClassRogueNovice
			|| sClass == (uint16)ClassType::ClassMageNovice
			|| sClass == (uint16)ClassType::ClassPriestNovice
			|| sClass == (uint16)ClassType::ClassPortuKurianNovice);
	}

	INLINE bool isNoviceWarrior() { return GetClassType() == (uint8)ClassType::ClassWarriorNovice; }
	INLINE bool isNoviceRogue()   { return GetClassType() == (uint8)ClassType::ClassRogueNovice; }
	INLINE bool isNoviceMage()    { return GetClassType() == (uint8)ClassType::ClassMageNovice; }
	INLINE bool isNovicePriest()  { return GetClassType() == (uint8)ClassType::ClassPriestNovice; }
	INLINE bool isNoviceKurianPortu()  { return GetClassType() == (uint8)ClassType::ClassPortuKurianNovice; }

	INLINE bool isMastered() 
	{
		uint16 sClass = GetClassType();
		return (sClass == (uint16)ClassType::ClassWarriorMaster
			|| sClass == (uint16)ClassType::ClassRogueMaster
			|| sClass == (uint16)ClassType::ClassMageMaster
			|| sClass == (uint16)ClassType::ClassPriestMaster
			|| sClass == (uint16)ClassType::ClassPortuKurianMaster);
	}

	INLINE bool isMasteredWarrior() { return GetClassType() == (uint8)ClassType::ClassWarriorMaster; }
	INLINE bool isMasteredRogue()   { return GetClassType() == (uint8)ClassType::ClassRogueMaster; }
	INLINE bool isMasteredMage()    { return GetClassType() == (uint8)ClassType::ClassMageMaster; }
	INLINE bool isMasteredPriest()  { return GetClassType() == (uint8)ClassType::ClassPriestMaster; }
	INLINE bool isMasteredKurianPortu() { return GetClassType() == (uint8)ClassType::ClassPortuKurianMaster; }

	INLINE bool isTrading() { return m_sTradeStatue != 1; }
	INLINE bool isStoreOpen() { return false; }
	INLINE bool isMerchanting() { return GetMerchantState() != MERCHANT_STATE_NONE; }
	INLINE bool isSellingMerchantingPreparing() { return m_bSellingMerchantPreparing; }
	INLINE bool isBuyingMerchantingPreparing() { return m_bBuyingMerchantPreparing; }
	INLINE bool isSellingMerchant() { return GetMerchantState() == MERCHANT_STATE_SELLING; }
	INLINE bool isBuyingMerchant() { return GetMerchantState() == MERCHANT_STATE_BUYING; }
	INLINE bool isMining() { return m_bMining; }
	INLINE bool isFishing() { return m_bFishing; }

	INLINE bool isBlockingPrivateChat() { return m_bBlockPrivateChat; }
	INLINE bool isWeaponsDisabled() { return m_bWeaponsDisabled; }
	INLINE bool isTowerOwner() { return GetTowerID() > -1; }
	INLINE bool isWantedUser() { return m_iswanted; }
	INLINE bool isInPKZone() { 
		return GetZoneID() == ZONE_ARDREAM 
			|| GetZoneID() == ZONE_RONARK_LAND 
			|| GetZoneID() == ZONE_RONARK_LAND_BASE
			|| isInSpecialEventZone(); }

	INLINE bool isInPKZone(uint16 ZoneID) {
		return ZoneID == ZONE_ARDREAM
			|| ZoneID == ZONE_RONARK_LAND
			|| ZoneID == ZONE_RONARK_LAND_BASE
			|| (ZoneID >= ZONE_SPBATTLE1 && ZoneID <= ZONE_SPBATTLE12);
	}

	INLINE bool isInPKBotZone() {
		return GetZoneID() == ZONE_BIFROST
			|| GetZoneID() == ZONE_DELOS;
	}

	INLINE bool isInWarZone(uint8 ZoneID = 0) {
		if (ZoneID == 0) 
			ZoneID = GetZoneID();

		return ZoneID >= ZONE_BATTLE && ZoneID <= ZONE_BATTLE6;
	}

	INLINE bool isInEventZone() {  
		return GetZoneID() == ZONE_BORDER_DEFENSE_WAR 
			|| GetZoneID() == ZONE_JURAID_MOUNTAIN 
			|| GetZoneID() == ZONE_CHAOS_DUNGEON; }

	INLINE int8 GetMerchantState() { return m_bMerchantState; }
	INLINE bool GetMerchantPremiumState() { return m_bPremiumMerchant; }
	INLINE uint8 GetAuthority() { return m_bAuthority; }
	INLINE uint8 GetAuthorityColor() { return m_bAuthorityColor; } // Renkli pm i�in geli�tirildi krala �zel renk felan 27.09.2020
	INLINE uint8 GetFame() { return m_bFame; }
	INLINE uint16 GetClass() { return m_sClass; }
	INLINE uint8 GetPremium() { return m_bPremiumInUse; }
	INLINE uint8 GetClanPremium() { return m_bClanPremiumInUse; }

	INLINE bool isLockableScroll(uint8 buffType) { 
		return (buffType == BUFF_TYPE_HP_MP 
			|| buffType == BUFF_TYPE_AC 
			|| buffType == BUFF_TYPE_FISHING 
			|| buffType == BUFF_TYPE_DAMAGE 
			|| buffType == BUFF_TYPE_SPEED 
			|| buffType == BUFF_TYPE_STATS 
			|| buffType == BUFF_TYPE_BATTLE_CRY); }

	INLINE uint8 GetRace() { return m_bRace; }
	INLINE uint8 GetRebirthLevel() { return m_bRebirthLevel; }
	INLINE uint32 GetExp() { return (uint32)m_iExp; }
	INLINE int16 GetTowerID() { return m_TowerOwnerID; }
	/**
	* @brief	Gets the player's base class type, independent of nation.
	*
	* @return	The class type.
	*/
	INLINE ClassType GetBaseClassType()
	{
		static const ClassType classTypes[] =
		{
			ClassType::ClassWarrior, 
			ClassType::ClassRogue, 
			ClassType::ClassMage, 
			ClassType::ClassPriest,
			ClassType::ClassWarriorNovice,
			ClassType::ClassWarriorMaster,		// job changed / mastered
			ClassType::ClassRogueNovice, 
			ClassType::ClassRogueMaster,			// job changed / mastered
			ClassType::ClassMageNovice, 
			ClassType::ClassMageMaster,			// job changed / mastered
			ClassType::ClassPriestNovice,
			ClassType::ClassPriestMaster,		// job changed / mastered
			ClassType::ClassPortuKurian,
			ClassType::ClassPortuKurianNovice,	// job changed / mastered 
			ClassType::ClassPortuKurianMaster,						// job changed / mastered
		};

		uint8 classType = GetClassType();
		if (!classType || classType > 15) {
			goDisconnect("diffrent class type", __FUNCTION__);
			return ClassType(0);
		}

		return classTypes[classType - 1];
	}

	/**
	* @brief	Gets class type, independent of nation.
	*
	* @return	The class type.
	*/
	INLINE uint8 GetClassType()
	{
		return GetClass() % 100;
	}

	INLINE int16 GetPartyID() { return m_sPartyIndex; }

	INLINE int16 GetClanID() { return m_bKnights; }
	INLINE void SetClanID(int16 val) { m_bKnights = val; }

	INLINE uint32 GetCoins() { return m_iGold; }
	INLINE uint32 GetCash() { return m_nKnightCash; }
	INLINE uint32 GetInnCoins() { return m_iBank; }
	INLINE uint32 GetLoyalty() { return m_iLoyalty; }
	INLINE uint16 GetOffMerchantTime() { return m_offmerchanttime; }
	
	INLINE uint32 GetMonthlyLoyalty() { return m_iLoyaltyMonthly; }
	INLINE uint32 GetManner() { return m_iMannerPoint; }
	INLINE uint32 GetTagNameColour() { return m_tagnamergb; }

	int32 GetStamina() { return m_sSp; };
	int32 GetMaxStamina() { return m_MaxSp; }

	virtual int32 GetHealth() { return m_sHp; }
	virtual int32 GetMaxHealth() { return m_MaxHp; }
	virtual int32 GetMana() { return m_sMp; }
	virtual int32 GetMaxMana() { return m_MaxMp; }
	 
	// Shortcuts for lazy people
	uint8 GetQuestStatus(uint16 sQuestID);
	INLINE bool hasCoins(uint32 amount) { return (GetCoins() >= amount); }
	INLINE bool hasInnCoins(uint32 amount) { return (GetInnCoins() >= amount); }
	INLINE bool hasLoyalty(uint32 amount) { return (GetLoyalty() >= amount); }
	INLINE bool hasMonthlyLoyalty(uint32 amount) { return (GetMonthlyLoyalty() >= amount); }
	INLINE bool hasManner(uint32 amount) { return (GetManner() >= amount); }
	INLINE uint8 GetAngerGauge() { return m_byAngerGauge; }
	INLINE bool hasFullAngerGauge() { return GetAngerGauge() >= MAX_ANGER_GAUGE; }
	INLINE bool hasRival() { return GetRivalID() >= 0; }
	INLINE bool hasRivalryExpired() { return UNIXTIME >= m_tRivalExpiryTime; }
	INLINE int16 GetRivalID() { return m_sRivalID; }
	INLINE GameState GetState() { return m_state; }
	INLINE uint16 GetActiveQuestID() { return 0; }
	

	uint8 GetClanGrade();
	uint8 GetClanRank();
	uint32 GetClanPoint();
	void SendClanPointChange(int32 nChangeAmount = 0);
	uint8 GetRankReward(bool isMonthly);
	uint8 GetWarVictory();
	uint8 CheckMiddleStatueCapture();
	uint8 GetMonsterChallengeTime();
	uint8 GetMonsterChallengeUserCount();
	void MoveMiddleStatue();
	uint8 GetPVPMonumentNation();
	bool GetUnderTheCastleOpen();
	uint16 GetUnderTheCastleUserCount();
	bool GetJuraidMountainTime();

	INLINE int16 GetJoinedEvent() { return m_sJoinedEvent; }

	INLINE uint8 GetStat(StatType type)
	{
		if (type >= StatType::STAT_COUNT)
			return 0;

		return m_bStats[(uint8)type];
	}

	INLINE void SetStat(StatType type, uint8 val)
	{
		if (type >= StatType::STAT_COUNT)
			return;

		m_bStats[(uint8)type] = val;
	}

	INLINE int32 GetStatTotal() // NOTE: Shares name with another, but lack-of args should be self-explanatory
	{
		int32 total = 0; // NOTE: this loop should be unrolled by the compiler
		foreach_array (i, m_bStats)
			total += m_bStats[i];
		return total;
	}

	INLINE int16 GetStatAchieveBonus(UserAchieveStatTypes type)
	{
		if (type >= UserAchieveStatTypes::ACHIEVE_STAT_COUNT)
			return 0;

		return m_sStatAchieveBonuses[(int16)type];
	}

	INLINE int16 GetStatItemBonus(StatType type)
	{
		if (type < StatType::STAT_STR || type >= StatType::STAT_COUNT)
			return 0;

		return m_sStatItemBonuses[(int16)type];
	}

	INLINE int16 GetStatWithItemBonus(StatType type)
	{
		return GetStat(type) + GetStatItemBonus(type);
	}

	INLINE int32 GetStatItemBonusTotal()
	{
		int32 total = 0; // NOTE: this loop should be unrolled by the compiler
		foreach_array (i, m_sStatItemBonuses)
			total += m_sStatItemBonuses[i];
		return total;
	}

	INLINE int16 GetStatBonusTotal(StatType type)
	{return GetStatBuff(type) + GetRebStatBuff(type) + GetStatItemBonus(type) + GetStatAchieveBonus((UserAchieveStatTypes)type);}

	INLINE uint8 GetRebStatBuff(StatType type)
	{
		if (type >= StatType::STAT_COUNT)
		{
			return 0;
		}
		return m_bRebStats[(uint8)type];
	}

	INLINE void SetRebStatBuff(StatType type, uint8 val)
	{
		if (type >= StatType::STAT_COUNT)
		{
			return;
		}

		m_bRebStats[(uint8)type] = val;
	}

	INLINE int8 GetStatBuff(StatType type)
	{
		if (type < StatType::STAT_STR || type >= StatType::STAT_COUNT) return 0;
		return m_bStatBuffs[(int8)type];
	}

	INLINE void SetStatBuffAdd(StatType type, uint8 val)
	{
		if (type < StatType::STAT_STR || type >= StatType::STAT_COUNT) return;
		m_bStatBuffs[(int8)type] += val;
	}

	INLINE void SetStatBuffRemove(StatType type, uint8 val)
	{
		if (type < StatType::STAT_STR || type >= StatType::STAT_COUNT) return;
		m_bStatBuffs[(int8)type] -= val;
	}

	INLINE uint32 GetStatBuffTotal()
	{
		uint32 total = 0;
		foreach_array(i, m_bStatBuffs) total += m_bStatBuffs[i];
		return total;
	}

	INLINE int16 getStatTotal(StatType type)
	{
		return GetStat(type) + GetStatBonusTotal(type);
	}

	INLINE uint16 GetTotalSkillPoints()
	{
		return m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree]
			+ m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat1]
			+ m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat2]
			+ m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat3]
			+ m_bstrSkill[(uint8)SkillPointCategory::SkillPointMaster];
	}

	INLINE uint8 GetSkillPoints(SkillPointCategory category)
	{
		if (category < SkillPointCategory::SkillPointFree
			|| category > SkillPointCategory::SkillPointMaster)
			return 0;

		return m_bstrSkill[(uint8)category];
	}

	INLINE _ITEM_DATA * GetItem(uint8 pos) 
	{
		if (pos > INVENTORY_TOTAL)
			return nullptr;

		if (pos < INVENTORY_TOTAL)
			return &m_sItemArray[pos]; 

		return nullptr;
	}

	INLINE _ITEM_DATA * GetWerehouseItem(uint8 pos)
	{
		if (pos > WAREHOUSE_MAX)
			return nullptr;

		if (pos < WAREHOUSE_MAX)
			return &m_sWarehouseArray[pos];

		return nullptr;
	}

	INLINE _ITEM_DATA* GetWipStorageItem(int8 pos)
	{
		if (pos > VIPWAREHOUSE_MAX)
			return nullptr;

		if (pos < VIPWAREHOUSE_MAX)
			return &m_sVIPWarehouseArray[pos];

		return nullptr;
	}

	INLINE _ITEM_TABLE GetItemPrototype(uint8 pos) 
	{
		if (pos > INVENTORY_TOTAL)
			return _ITEM_TABLE();

		_ITEM_DATA * pItem;
		if (pos < INVENTORY_TOTAL)
			return GetItemPrototype(pos, pItem);

		return _ITEM_TABLE();
	}

	INLINE void ResetTempleEventData()
	{
		m_bEventRoom = 0;
		m_iEventJoinOrder = 0;
		m_sJoinedEvent = -1;
		m_isFinalJoinedEvent = false;
	}

	_ITEM_TABLE GetItemPrototype(uint8 pos, _ITEM_DATA *& pItem);

	INLINE KOMap * GetMap() { return m_pMap; }

	CUser(uint16 socketID, SocketMgr *mgr); 

	virtual void OnConnect();
	virtual void OnDisconnect();
	virtual bool HandlePacket(Packet & pkt);

	void Update();
	void BurningTime();

	void UpdateCheckTime();
	void UpdateCheckSkillTime();
	void UpdateCheckPremiumTime();
	void UpdateCheckItemTime();
	void CheckDelayedTime();

	virtual void AddToRegion(int16 new_region_x, int16 new_region_z);
	void ExpFlash();
	void DcFlash();
	void WarFlash();
	void HandleBDWCapture(Packet & pkt);
	void HandleTowerPackets(Packet & pkt);
	void TowerExitsFunciton(bool dead = false);
	void HandleGenie(Packet & pkt);
	void GenieNonAttackProgress(Packet & pkt);
	void GenieNotice(Packet&pkt);
	void UpdateGenieTime(uint16 m_sGenieTime);
	void GenieAttackProgress(Packet & pkt);
	void GenieStart();
	void GenieStop();
	void GenieUseGenieSpirint(Packet & pkt);
	void HandleGenieLoadOptions();
	void HandleGenieSaveOptions(Packet & pkt);

	void GetMerchantSlipList(_MERCH_DATA list[MAX_MERCH_ITEMS], CUser* pownermerch);
	void MerchantSlipRefList(CUser* pownermerch, bool merchcrea = false);
	void GiveSlaveMerchantItems();
	void MerchantSlaveClose();
	void MonsterDeathCount();
	void SetRival(Unit * pRival);
	void RemoveRival();
	void SendLoyaltyChange(std::string placeowned = "-", int32 nChangeAmount = 0, bool bIsKillReward = false, bool bIsBonusTime = false, bool bIsAddLoyaltyMonthly = true);
	void NativeZoneReturn();
	void KickOutZoneUser(bool home = false, uint8 nZoneID = 21);
	void TrapProcess();
	bool JobGroupCheck(short jobgroupid);
	uint8 GetBaseClass();
	void SendSay(int32 nTextID[8]);
	void SelectMsg(uint8 bFlag, int32 nQuestID, int32 menuHeaderText, int32 menuButtonText[MAX_MESSAGE_EVENT], int32 menuButtonEvents[MAX_MESSAGE_EVENT]);
	bool CheckClass(short class1, short class2 = -1, short class3 = -1, short class4 = -1, short class5 = -1, short class6 = -1);
	bool GiveItem(std::string placeowned,uint32 nItemID, uint16 sCount = 1, bool send_packet = true, uint32 Time = 0, _giveitempusinfo pPus = _giveitempusinfo(false,0,0), uint8 nSession = 0);
	bool GiveItemChecks(uint32 nItemID, uint16 sCount);
	bool GiveWerehouseItem(uint32 nItemID, uint16 sCount = 1, bool mining = true, bool fishing = true, uint32 Time = 0);
	bool RobItem(uint32 nItemID, uint16 sCount = 1, bool sendpacket = true);
	bool RobItem(uint8 bPos, _ITEM_TABLE pTable, uint16 sCount = 1, bool sendpacket = true, bool slaveitem = false);
	bool RobItemPet(uint32 nItemID, uint32 sCount = 1);
	bool RobItemPet(uint8 bPos, _ITEM_TABLE pTable, uint16 sCount = 1);
	bool RobAllItemParty(uint32 nItemID, uint16 sCount = 1);
	uint16 GeneratorCheckExistItemCount(int itemid, short count = 1);
	bool CheckExistItem(int itemid, short count = 1);
	bool CheckExistItemAnd(int32 nItemID1, int16 sCount1, int32 nItemID2, int16 sCount2, int32 nItemID3, int16 sCount3, int32 nItemID4, int16 sCount4, int32 nItemID5, int16 sCount5);
	bool CheckExistSpacialItemAnd(int32 nItemID1, int16 sCount1, int32 nItemID2, int16 sCount2, int32 nItemID3, int16 sCount3, int32 nItemID4, int16 sCount4, int32 nItemID5, int16 sCount5, int32 nItemID6, int16 sCount6, int32 nItemID7, int16 sCount7, int32 nItemID8, int16 sCount8, int32 nItemID9, int16 sCount9, int32 nItemID10, int16 sCount10);
	void SendNewDeathNotice(Unit* pKiller);
	bool GenieExchange(uint32 itemid, uint32 time, bool newChar =false);
	void CheckGenieTime();
	void GiveKillReward();
	bool GiveItemLua(
		uint32 nGiveItemID1 = 0,
		uint32 nGiveItemCount1 = 1,
		uint32 nGiveItemTime1 = 0,
		uint32 nGiveItemID2 = 0,
		uint32 nGiveItemCount2 = 1,
		uint32 nGiveItemTime2 = 0,
		uint32 nGiveItemID3 = 0,
		uint32 nGiveItemCount3 = 1,
		uint32 nGiveItemTime3 = 0,
		uint32 nGiveItemID4 = 0,
		uint32 nGiveItemCount4 = 1,
		uint32 nGiveItemTime4 = 0,
		uint32 nGiveItemID5 = 0,
		uint32 nGiveItemCount5 = 1,
		uint32 nGiveItemTime5 = 0,
		uint32 nGiveItemID6 = 0,
		uint32 nGiveItemCount6 = 1,
		uint32 nGiveItemTime6 = 0,
		uint32 nGiveItemID7 = 0,
		uint32 nGiveItemCount7 = 1,
		uint32 nGiveItemTime7 = 0,
		uint32 nGiveItemID8 = 0,
		uint32 nGiveItemCount8 = 1,
		uint32 nGiveItemTime8 = 0,
		uint32 nRobItemID1 = 0,
		uint32 nRobItemCount1 = 0,
		uint32 nRobItemID2 = 0,
		uint32 nRobItemCount2 = 0,
		uint32 nRobItemID3 = 0,
		uint32 nRobItemCount3 = 0,
		uint32 nRobItemID4 = 0,
		uint32 nRobItemCount4 = 0,
		uint32 nRobItemID5 = 0,
		uint32 nRobItemCount5 = 0);

	bool CheckExistItemLua(uint32 itemid, uint32 count = 1);
	bool CheckExistItemAndLua(uint32 nItemID1, uint32 sCount1, uint32 nItemID2, uint32 sCount2, uint32 nItemID3, uint32 sCount3, uint32 nItemID4, uint32 sCount4, uint32 nItemID5, uint32 sCount5);
	bool RobItemLua(uint8 bPos, _ITEM_TABLE pTable, uint16 sCount = 1);
	bool RobItemLua(uint32 nItemID1, uint32 sCount1, uint32 nItemID2, uint32 sCount2, uint32 nItemID3, uint32 sCount3, uint32 nItemID4, uint32 sCount4, uint32 nItemID5, uint32 sCount5);

	uint16 GetItemCount(uint32 nItemID);
	void CheckRespawnScroll();
	bool CheckGiveSlot(uint8 sSlot);
	bool LuaCheckGiveSlot(uint8 sSlot);
	void MonsterStoneTimerScreen(uint16 Time);
	bool CheckWeight(uint32 nItemID, uint16 sCount);
	bool CheckWeight(_ITEM_TABLE pTable, uint32 nItemID, uint16 sCount);
	bool CheckSkillPoint(uint8 skillnum, uint8 min, uint8 max);
	bool GoldLose(uint32 gold, bool bSendPacket = true);
	bool CashLose(uint32 cash);
	void CashGain(uint32 cash);
	bool LoyaltyLose(uint32 loyalty);
	void GoldGain(uint32 gold, bool bSendPacket = true, bool bApplyBonus = false);
	void LuaGoldGain(uint32 gold, bool bSendPacket = true);
	
	bool JackPotNoah(uint32 gold);
	bool JackPotExp(int64 iExp);
	
	bool isCheckDecated;
	void ShowBulletinBoard();
	void ShowKarusRankBoard();
	void ShowElmoradRankBoard();

	//�eriff
	void KingsInspectorList();
	void SheriffVote(Packet & pkt);

	void SendItemWeight();
	void UpdateVisibility(InvisibilityType bNewType);
	void ResetGMVisibility();
	void BlinkStart(int exBlinkTime = 0);
	void BlinkTimeCheck();
	void SetFlashTimeNote(bool remove = false);
	void FlashUpdateTime(uint32 time);
	void GoldChange(short tid, int gold);
	CUser * GetItemRoutingUser(uint32 nItemID, uint16 sCount);
	bool GetStartPosition(short & x, short & y, uint8 bZone = 0, bool isCind = false);
	bool GetStartPositionRandom(short & x, short & z, uint8 bZone = 0);
	int FindSlotForItem(uint32 nItemID, uint16 sCount = 1);
	int GetEmptySlot();
	int FindWerehouseSlotForItem(uint32 nItemID, uint16 sCount = 1);
	int GetWerehouseEmptySlot();
	void SendAllKnightsID();
	void SendStackChange(uint32 nItemID, uint32 nCount /* needs to be 4 bytes, not a bug */, uint16 sDurability, 
		uint8 bPos, bool bNewItem = false, uint32 Time = 0, uint8 bSlotSection = 1);
	void SendStackChangeSpecial(uint32 nItemID, uint32 nCount, uint16 sDurability,
		uint8 bPos, bool bNewItem = false, uint32 Time = 0, _ITEM_DATA* pItem = nullptr);
	void SpawnEventSystem(uint16 sSid, uint8 bIsmonster, uint8 byZone, float fX, float fY, float fZ);
	void Type4Duration();
	void Type6Duration();
	void Type9Duration(bool bForceExpiration = false);
	void HPTimeChange();
	void SpTimeChange();
	void HPTimeChangeType3();
	void TrainingProcess();

	short GetDamage(Unit *pTarget, _MAGIC_TABLE pSkill = _MAGIC_TABLE(), bool bPreviewOnly = false);
	void OnAttack(Unit * pTarget, AttackType attackType);
	bool TriggerProcZoneChecking(uint8 TargetZoneID, uint8 OwnerZoneID);
	void OnDefend(Unit * pAttacker, AttackType attackType);
	bool TriggerProcItem(uint8 bSlot, Unit * pTarget, ItemTriggerType triggerType);

	void SendDurability(uint8 slot, uint16 durability);
	void SendItemMove(uint8 command, uint8 subcommand);
	void SendNpcKillID(uint32 sNpcID); //npckill
	void ItemWoreOut( int type, int damage );
	void Dead();
	bool ItemEquipAvailable( _ITEM_TABLE pTable );
	bool ItemClassAvailable( _ITEM_TABLE pTable );
	
	void SpChange(int amount);
	void SiegeTransformHpChange(int amount);

	virtual void HpChange(int amount, Unit *pAttacker = nullptr, bool isDOT = false);
	virtual void MSpChange(int amount);
	
	void SendPartyHPUpdate();
	void SendPartyHpManager(uint8 type); //17.12.2020 Partyde HP Degeri Yazi Olarak Gosterme
	void ShowEffect(uint32 nSkillID);
	void ShowNpcEffect(uint32 nEffectID, bool bSendToRegion = false);
	void SendAnvilRequest(uint16 sNpcID, uint8 bType = ITEM_UPGRADE_REQ);
	void RecastSavedMagic(uint8 buffType = 0);
	void RecastLockableScrolls(uint8 buffType);
	void SetMaxSp();
	void HealMagic();
	void HealAreaCheck(C3DMap * pMap, int rx, int rz);
	// packet handlers start here
	void VersionCheck(Packet & pkt);
	void KickOutProcess(Packet & pkt);
	void LoginProcess(Packet & pkt);
	void SendLoginFailed(int8 errorid);
	void LoadingLogin(Packet & pkt);
	void AllCharInfoToAgent();
	void AllCharNameChangeInfo(Packet & pkt);
	void AllCharInfo(Packet & pkt);
	void AllCharLocationSendToAgent(Packet & pkt);
	void AllCharLocationRecvToAgent(Packet & pkt);

	void SelNationToAgent(Packet & pkt);
	void ChangeHair(Packet & pkt);
	void NewCharToAgent(Packet & pkt);
	bool NewCharValid(uint8 bRace, uint16 bClass);
	bool NewCharClassVaid(uint16 bClass);
	bool NewCharRaceVaid(uint16 bRace);
	void DelCharToAgent(Packet & pkt);
	void SelCharToAgent(Packet & pkt);
	void SelectCharacter(Packet & pkt); // from the database
	void SetLogInInfoToDB(uint8 bInit);
	void RecvLoginInfo(Packet & pkt); // from the database
	void VSEventProcess(int Code); //vs event

	void ZindanWarSendScore();
	void ZindanWarUpdateScore();
	void ZindanWarLogOut();

	void SpeedHackTime(Packet & pkt);
	void TempleProcess(Packet & pkt );
	bool TempleJoinEvent(int16 sEvent);
	void TempleDisbandEvent(int16 sEvent);

	//Xtreme Monster Stone Recive Exp vermesi
	bool TempleGetExp(Packet& pkt);

	void csw_notice(CswNotice p);
	void csw_sendwarflag(CKnights *pOwner = nullptr, bool finish = false);
	bool csw_canenterdelos();
	void csw_town();
	void csw_rank();
	void csw_rank_register();
	void csw_rank_killupdate();

	void MonsterStoneProcess(Packet & pkt);
	bool MonsterStoneQuestJoin(uint16 sQuestID, uint8 bTargetZoneID = 0, uint8 bTargetFamily = 0);
	void TempleMonsterStoneItemExitRoom();
	void SendMonsterStoneFail(uint8 errorid);
	void DrakiTowerList();
	void DrakiTowerTempleEnter(Packet & pkt);
	void SendDrakiTempleDetail(bool MonsterSummon);
	void DrakiRiftChange(uint16 DrakiStage, uint16 DrakiSubStage);
	uint8 SelectDrakiRoom();
	uint8 SelectNpcDrakiRoom();
	void SummonDrakiMonsters(uint8 RoomIndex);
	void DrakiTowerNpcOut();
	void ChangeDrakiMode();
	void DrakiTowerSavedUserInfo();
	void UpdateDrakiRank(uint32 Time, uint8 bRoom);
	void DrakiTowerKickOuts();
	void DrakiTowerKickTimer();
	void DrakiTowerTown();
	void TournamentSendTimer();
	void GameStart(Packet & pkt);
	void RentalSystem(Packet & pkt);
	void SkillDataProcess(Packet & pkt);
	void SkillDataSave(Packet & pkt);
	void SkillDataLoad();
	void MoveProcess(Packet & pkt);
	void Rotate(Packet & pkt);
	void Attack(Packet & pkt);

	void BeefEventGetTime();
	void BeefEventGetNotice(uint8 NoticeType);
	bool BeefEventLogin();

	static void InitChatCommands();
	static void CleanupChatCommands();

	void goDisconnect(std::string log = "", std::string funcname = "");

	void Chat(Packet & pkt);
	bool gmsendpmcheck(uint16 id);
	void ChatTargetSelect(Packet & pkt);
	void SendDeathNotice(Unit * pKiller, DeathNoticeType noticeType, bool isToZone = true);
	bool ProcessChatCommand(std::string & message);

	uint8 GetUserDailyOp(uint8 type = 0);
	void SetUserDailyOp(uint8 type = 0, bool isInsert = false);

	uint32 GetEventTrigger();

	void RemoveStealth();
	bool virt_eventattack_check();
	bool isHealDamageChecking(Unit* pTarget, _MAGIC_TABLE pSkill);
	bool isDKMToMonsterDamageSkills(uint32 nSkillID);
	bool isDKMToUserDamageSkills(uint32 nSkillID);

	void GiveBalance(int32 cashcount = 0, int32 tlcount = 0, short ownerid = -1);
	void RobChaosSkillItems();

	// Nation Transfer, Gender Change and Job Change (in game)
	void HandleNationChange(Packet & pkt);
	void ReqHandleNationChange(Packet & pkt);
	void SendNationTransfer();
	void ReqSendNationTransfer();
	uint8	NationChange();
	uint8	GetNewRace();
	bool GenderChange(uint8 nRace = 0);
	bool GenderChangeGM(uint8 nRace = 0);
	uint8 JobChange(uint8 type, uint8 NewJob);
	uint8 JobChangeGM(uint8 NewJob = 0);
	void HandleJobChangeScroll(Packet& pkt);
	void GenderChangeV2(Packet & pkt);

	bool isAryaOnlineWorld;
	uint32 m_tGuardAliveExpiryTime;

	void WheelOfFun(Packet& pkt);
	void HandleDailyRank(Packet& pkt);
	void HandleDailyRankShow(Packet& pkt);
	void ReqHandleDailyRank(Packet& pkt);
	void HandleChestBlock(Packet& pkt);
	void HandleRightClickExchange(Packet& pkt);
	bool CheckChestBlockItem(uint32 itemid);
	void BotMerchantAdd(uint16 count, uint32 time, uint16 type);
	bool CheckRightClickExchange(uint32 nExchangeID);
	COMMAND_HANDLER(HandleHelpCommand);
	COMMAND_HANDLER(HandleResetRLoyaltyCommand);
	COMMAND_HANDLER(HandleGiveItemCommand);
	COMMAND_HANDLER(HandleOnlineZoneGiveItemCommand);
	COMMAND_HANDLER(HandleOnlineGiveItemCommand);
	COMMAND_HANDLER(HandleZoneChangeCommand);
	COMMAND_HANDLER(HandleMonsterSummonCommand);
	COMMAND_HANDLER(HandleMonsterStoneTestCommand);
	COMMAND_HANDLER(HandleNPCSummonCommand);
	COMMAND_HANDLER(HandleMonKillCommand);
	COMMAND_HANDLER(HandleWar1OpenCommand);
	COMMAND_HANDLER(HandleWar2OpenCommand);
	COMMAND_HANDLER(HandleWar3OpenCommand);
	COMMAND_HANDLER(HandleWar4OpenCommand);
	COMMAND_HANDLER(HandleWar5OpenCommand);
	COMMAND_HANDLER(HandleWar6OpenCommand);
	COMMAND_HANDLER(HandleWarMOpenCommand);
	COMMAND_HANDLER(HandleCaptainCommand);
	COMMAND_HANDLER(HandleSnowWarOpenCommand);
	COMMAND_HANDLER(HandleSiegeWarOpenCommand);
	COMMAND_HANDLER(HandleWarCloseCommand);
	COMMAND_HANDLER(HandleLoyaltyChangeCommand);
	COMMAND_HANDLER(HandleExpChangeCommand);
	COMMAND_HANDLER(HandleGoldChangeCommand);
	COMMAND_HANDLER(HandleKcChangeCommand); //gm koduyla kc yukleme veya eksiltme
	COMMAND_HANDLER(HandleTLBalanceCommand); //gm koduyla kc yukleme veya eksiltme
	COMMAND_HANDLER(HandleExpAddCommand); /* for the server XP event */
	COMMAND_HANDLER(HandleNPAddCommand); /* for the server XP event */
	COMMAND_HANDLER(HandleMoneyAddCommand); /* for the server coin event */
	COMMAND_HANDLER(HandleDropAddCommand); /* for the server drop event */
	COMMAND_HANDLER(HandleTeleportAllCommand);
	COMMAND_HANDLER(HandlePrivateAllCommand);
	COMMAND_HANDLER(HandleKnightsSummonCommand);
	COMMAND_HANDLER(HandleWarResultCommand);
	COMMAND_HANDLER(HandleResetPlayerRankingCommand);

	COMMAND_HANDLER(HandleunbannedCommand);
	COMMAND_HANDLER(Handlebannedcommand);
	COMMAND_HANDLER(HandlePcBlock);

	COMMAND_HANDLER(HandleCountZoneCommand);
	COMMAND_HANDLER(HandleCountLevelCommand);

	COMMAND_HANDLER(HandleGiveItemSelfCommand);
	COMMAND_HANDLER(HandleNationChangeCommand);
	
	COMMAND_HANDLER(HandleSummonUserCommand);
	COMMAND_HANDLER(HandleTpOnUserCommand);
	COMMAND_HANDLER(HandleGoUserCommand);
	COMMAND_HANDLER(HandleLocationChange);

	COMMAND_HANDLER(HandleForgettenTempleEventClose);
	COMMAND_HANDLER(HandleForgettenTempleEvent);

	COMMAND_HANDLER(HandleMuteCommand);
	COMMAND_HANDLER(HandleUnMuteCommand);
	COMMAND_HANDLER(HandleAllowAttackCommand);
	COMMAND_HANDLER(HandleDisableCommand);
	COMMAND_HANDLER(HandleChangeRoom);
	COMMAND_HANDLER(HandleCastleSiegeWarClose);
	COMMAND_HANDLER(HandleSummonPrison);
	COMMAND_HANDLER(HandleServerGameTestCommand);
	COMMAND_HANDLER(HandleEffectTestCommand);
	COMMAND_HANDLER(HandleBotSpawnMining);
	COMMAND_HANDLER(HandleBotSpawnFishing);
	COMMAND_HANDLER(HandleBotSpawnFarm);
	COMMAND_HANDLER(HandleBotSpawnFarmMix);
	COMMAND_HANDLER(HandleBotSpawnPk);
	COMMAND_HANDLER(HandleBotSpawnPkMix);
	COMMAND_HANDLER(HandlePkBoxSingleCommand);
	COMMAND_HANDLER(HandleMerchantBotMix);
	COMMAND_HANDLER(HandleBotSpawnMerchant);
	COMMAND_HANDLER(HandleBotSpawnMerchantMove);
	COMMAND_HANDLER(HandleBotDisconnected);
	COMMAND_HANDLER(HandleBotAllDisconnected);
	COMMAND_HANDLER(HandleBotAfkSystem);
	COMMAND_HANDLER(HandleBugdanKurtarCommand);
	COMMAND_HANDLER(HandleOpenMaster);
	COMMAND_HANDLER(HandleOpenSkill);
	COMMAND_HANDLER(HandleOpenQuestSkill);
	COMMAND_HANDLER(HandleReloadNoticeCommand);
	COMMAND_HANDLER(HandleReloadTablesCommand);
	COMMAND_HANDLER(HandleReloadTables2Command);
	COMMAND_HANDLER(HandleReloadAllTabCommand);
	COMMAND_HANDLER(HandleReloadTables3Command);
	COMMAND_HANDLER(HandleReloadMagicsCommand);
	COMMAND_HANDLER(HandleReloadQuestCommand);
	COMMAND_HANDLER(HandleReloadDailyQuestCommand);
	COMMAND_HANDLER(HandleReloadRanksCommand);
	COMMAND_HANDLER(HandleReloadDropsCommand);
	COMMAND_HANDLER(HandleReloadDropsRandomCommand);
	COMMAND_HANDLER(HandleReloadKingsCommand);
	COMMAND_HANDLER(HandleReloadRightTopTitleCommand);
	COMMAND_HANDLER(HandleReloadPusItemCommand);
	COMMAND_HANDLER(HandleReloadItemsCommand);
	COMMAND_HANDLER(HandleReloadUpgradeCommand);
	COMMAND_HANDLER(HandleReloadRankBugCommand);
	COMMAND_HANDLER(HandleReloadLevelRewardCommand);
	COMMAND_HANDLER(HandleReloadMerchantLevelRewardCommand);
	COMMAND_HANDLER(HandleSaveMerchant);
	COMMAND_HANDLER(HandleLoadMerchant);
	COMMAND_HANDLER(HandleAIResetCommand);
	COMMAND_HANDLER(HandleReloadCindirellaCommand);
	COMMAND_HANDLER(HandleSpecialEventOpenCommand);
	COMMAND_HANDLER(HandleGiveGenieTime);
	COMMAND_HANDLER(HandleBowlEvent);
	COMMAND_HANDLER(HandleReloadZoneOnlineRewardCommand);

	COMMAND_HANDLER(HandleReloadDungeonDefenceTables);
	COMMAND_HANDLER(HandleReloadDrakiTowerTables);
	COMMAND_HANDLER(HandleEventScheduleResetTable);

	COMMAND_HANDLER(HandleTopLeftCommand);
	COMMAND_HANDLER(HandleReloadBonusNotice);
	COMMAND_HANDLER(HandleReloadItems);

	COMMAND_HANDLER(HandleChaosExpansionOpen);
	COMMAND_HANDLER(HandleBorderDefenceWarOpen);
	COMMAND_HANDLER(HandleJuraidMountainOpen);
	COMMAND_HANDLER(HandleBeefEventOpen);
	COMMAND_HANDLER(HandleCindirellaWarOpen);
	COMMAND_HANDLER(HandleCindirellaWarClose);
	COMMAND_HANDLER(HandleBeefEventClose);
	COMMAND_HANDLER(HandleChaosExpansionClosed);
	COMMAND_HANDLER(HandleBorderDefenceWarClosed);
	COMMAND_HANDLER(HandleJuraidMountainClosed);
	COMMAND_HANDLER(HandleReloadClanPremiumTable);

	COMMAND_HANDLER(HandleAnindaGM); 
	COMMAND_HANDLER(HandlePartyTP); 
	COMMAND_HANDLER(HandleChangeGM); 
	COMMAND_HANDLER(HandleGenieStartStop); 
	COMMAND_HANDLER(HandleNpcBilgi);
	COMMAND_HANDLER(HandleJobChangeGM);
	COMMAND_HANDLER(HandleGenderChangeGM); 
	COMMAND_HANDLER(HandleNpcDropTester);
	COMMAND_HANDLER(HandleReloadTable);
	COMMAND_HANDLER(HandleMiningDropTester);
	COMMAND_HANDLER(HandleFishingDropTester);
	COMMAND_HANDLER(HandleInventoryClear);
	COMMAND_HANDLER(HandleLotteryStart);
	COMMAND_HANDLER(HandleLotteryClose);
	COMMAND_HANDLER(HandleLevelChange);
	COMMAND_HANDLER(HandleCountCommand);
	COMMAND_HANDLER(HandleMerchantBotCommand);
	COMMAND_HANDLER(ClanChatFollowCommand); // JstKO GameMaster Klan Sohbetini Oku Eklendi 01.01.2025
	COMMAND_HANDLER(FullUpgradeStatusHandlers); // JstKO GM Full Upgrade %100 �zel Eklendi 05.01.2025

	void SetOffCha(_choffstatus s, offcharactertype type);
	void Regene(uint8 regene_type, uint32 magicid = 0);
	void RequestUserIn(Packet & pkt);
	void RequestNpcIn(Packet & pkt);
	void RecvWarp(Packet & pkt);
	void Warp(uint16 sPosX, uint16 sPosZ);
	void ItemMove(Packet & pkt);
	void NpcEvent(Packet & pkt);
	void VsEvent(Packet& pkt);

	void InventorySystem(Packet & pkt);
	void InventorySystemReflesh(Packet & pkt);
	void InventoryClear(CUser * ClearUser);

	void ItemTrade(Packet & pkt);
	void ItemTradeRepurchase(Packet & pkt);
	void BundleOpenReq(Packet & pkt);
	void JstKOAutoLooter(_LOOT_BUNDLE* pBundle);
	void ItemGet(Packet & pkt);
	bool m_AutoUpgrade;
	CUser * GetLootUser(_LOOT_BUNDLE * pBundle, _LOOT_ITEM pItem);
	uint8 isLootingWeaponCheck(uint8 Kind);

	void RecvZoneChange(Packet & pkt);
	void PointChange(Packet & pkt);

	void StateChange(Packet & pkt);
	virtual void StateChangeServerDirect(uint8 bType, uint32 nBuff);



	void PartySystemProcess(Packet & pkt);
	void PartyisDelete();
	void PartyLeaderPromote(uint16 GetLeaderID, _PARTY_GROUP* pmyparty = nullptr);
	void PartyNemberRemove(uint16 UserID, _PARTY_GROUP* pmyparty = nullptr);
	void AgreeToJoinTheParty();
	void DoNotAcceptJoiningTheParty();
	void PartyCreateRequest(Packet & pkt);
	void PartyInvitationRequest(Packet & pkt);
	void PartyInvitationCheck(uint16 UserSocketID, uint8 PartyType = 0);
	void PartyCreateCheck(std::string strUserID, int8 PartyType = 0);
	void PartyInsertOrCancel(Packet & pkt);
	void PartyAlert(Packet & pkt);
	void PartyCommand(Packet & pkt);
	void PartyTargetNumber(Packet & pkt);
	void PartyEventSystemCreatePartyCheck(uint16 InviteID);
	void PartyEventSystemCreatePartyTools();

	void EventPartyCreate();
	void EventPartyInvitationCheck(uint16 EnterPartyGetID);
	void EventPartyInvitation();

	void GmListProcess(bool logout);

	//ClanBank
	void ClanWareHouseProcess(Packet & pkt);
	void ClanWarehouseOpen(Packet & pkt);
	void ClanWarehouseItemInput(Packet & pkt);
	void ClanWarehouseItemOutput(Packet & pkt);
	void ClanWarehouseItemMove(Packet & pkt);
	void ClanWarehouseInventoryItemMove(Packet & pkt);

	// Trade system
	void ExchangeProcess(Packet & pkt);
	void ExchangeReq(Packet & pkt);
	void ExchangeAgree(Packet & pkt);
	void ExchangeAdd(Packet & pkt);
	void ExchangeDecide();
	void ExchangeCancel(bool bIsOnDeath = false);
	void SetSpecialItemBuffer(uint32 itemid, uint64 serial, ByteBuffer& result);
	void ExchangeGiveItemsBack();
	void ExchangeFinish();
	void NewTradeAgreeErrorReset();
	/* Lottery System */
	void LotteryJoinFunction();
	void LotteryGameStartSend();
	bool LotteryItemCheck(uint32 ItemID1, uint32 ItemCount1,
		uint32 ItemID2, uint32 ItemCount2,
		uint32 ItemID3, uint32 ItemCount3,
		uint32 ItemID4, uint32 ItemCount4,
		uint32 ItemID5, uint32 ItemCount5);

	bool CheckExchange();
	void ExecuteExchange(uint32 &get_money);
	bool CheckExecuteExchange();

	// Merchant system (both types)
	void MerchantProcess(Packet & pkt);

	// regular merchants
	void MerchantOpen();
	void MerchantClose();
	void MerchantItemAdd(Packet & pkt);
	void MerchantItemCancel(Packet & pkt);
	void MerchantItemList(Packet & pkt);
	void MerchantItemBuy(Packet & pkt);
	void MerchantInsert(Packet & pkt);
	void CancelMerchant();
	void MerchantList(Packet& pkt);
	void MerchantEye();

	void UserInfoStorage(bool login);

	// buying merchants
	void BuyingMerchantOpen(Packet & pkt);
	void BuyingMerchantClose();
	void BuyingMerchantInsert(Packet & pkt);
	void BuyingMerchantInsertRegion();
	void BuyingMerchantList(Packet & pkt);
	void BuyingMerchantBuy(Packet & pkt);

	//Merchant Official List
	void MerchantOfficialList(Packet & pkt);
	void MerchantListSend(Packet & pkt);
	void MerchantListMoveProcess(Packet & pkt);

	void RemoveFromMerchantLookers();

	void SkillPointChange(Packet & pkt);

	void ObjectEvent(Packet & pkt);
	bool BindObjectEvent(_OBJECT_EVENT *pEvent);
	bool GateLeverObjectEvent(_OBJECT_EVENT *pEvent, int nid);
	bool LogLeverBuringLog(_OBJECT_EVENT *pEvent, int nid);
	bool FlagObjectEvent(_OBJECT_EVENT *pEvent, int nid);
	bool WarpListObjectEvent(_OBJECT_EVENT *pEvent);
	bool KrowazGateEvent(_OBJECT_EVENT *pEvent, int nid);

	void UpdateGameWeather(Packet & pkt);
	void PPCard(Packet & pkt);
	void SendPPCardFail(uint8 errorcode);
	void ClassChange(Packet & pkt, bool bFromClient = true);
	void ClassChangeReq();
	void SendStatSkillDistribute();
	void AllPointChange(bool bIsFree = false);
	void AllSkillPointChange(bool bIsFree = false);
	void SendTagNameChangePanel();

	uint32 getskillpointreqmoney();
	void SendPresetReqMoney();

	void AllPointChange2(bool bIsFree = false);
	void StatPresetHandle(Packet& pkt);
	void AllSkillPointChange2(bool bIsFree = false);

	void PresetMyStatReset(Packet& pkt);
	void PresetMySkillReset(Packet& pkt);
	void CountConcurrentUser();
	void UserDataSaveToAgent();

	void ItemRepair(Packet & pkt);
	void ItemRemove(Packet & pkt);
	void SendRepurchaseMsg(bool isRefreshed = false);
	void BuyingItemRepurchase(Packet &pkt);
	void ResetRepurchaseData();
	void OperatorCommand(Packet & pkt);
	void WarehouseProcess(Packet & pkt);
	void VIPhouseProcess(Packet & pkt);
	void ReqVipStorageProcess(Packet & pkt);
	void ReqConcurrentProcess(Packet & pkt);
	void DailyOpProcess(Packet & pkt);
	void UseVipKeyError(uint8 subcode, uint8 error);
	void Home(bool check = true);
	void WarehouseSendError(uint8 opcode, WarehouseError e);
	void FriendProcess(Packet & pkt);
	void FriendRequest();
	void FriendModify(Packet & pkt, uint8 opcode);
	void RecvFriendModify(Packet & pkt, uint8 opcode);
	void FriendReport(Packet & pkt);
	uint8 GetFriendStatus(std::string & charName, int16 & sid);

	void SelectWarpList(Packet & pkt);
	bool GetWarpList( int warp_group );
	bool GetLevelChangeStat();
	bool GetLevelChangeSkill();
	bool GateActimmi; //Gate D��ardan Paket Fix
	void PlayerProgramCheck(Packet &pkt); // /plc komutunu cal�st�r�r ve kulland�g� programlar� g�sterir sol altta notice olarak
	void ServerChangeOk(Packet & pkt);
	void ClientMerchantWindNotice(std::string Msg, std::string Name, uint16 GetX, uint16 GetZ, uint32 color);

	void CaptchaRequest(Packet& pkt);//xx
	void CaptchaVerification(Packet& pkt);//xx
	void CaptchaProcess();//xx

	void KA_KillUpdate();
	void KA_ResetCheck(bool zonechange = false);
	
	void KA_AssistDebufUpdate(CUser *pkiller);
	void KA_AssistScreenSend(uint8 type = 0);

	void EventAfkCheck();
	bool BDWAttackAreaCheck();
	void PartyBBS(Packet & pkt);
	void PartyBBSRegister(Packet & pkt);
	void PartyBBSDelete(Packet & pkt);
	void PartyBBSNeeded(Packet & pkt);
	void PartyBBSWanted(Packet & pkt);
	uint8 GetPartyMemberAmount(_PARTY_GROUP *pParty = nullptr);
	void SendPartyBBSNeeded(Packet & pkt);
	void PartyBBSUserLoqOut();
	void PartyBBSPartyChange(Packet & pkt);
	void PartyBBSPartyDelete(Packet & pkt);

	void DungeonDefencePartyBBSRegister(Packet & pkt);
	void DungeonDefencePartyBBSDelete(Packet & pkt);
	void DungeonDefencePartyBBSNeeded(Packet & pkt);
	void DungeonDefencePartyBBSWanted(Packet & pkt);
	void DungeonDefenceSendPartyBBSNeeded(Packet & pkt);
	void DungeonDefencePartyBBSPartyChange(Packet & pkt);
	void DungeonDefencePartyBBSPartyDelete(Packet & pkt);

	void DungeonDefenceSign();
	void ChangeDungeonDefenceStage();
	void DungeonDefenceSendFinishTimer();
	void DungeonDefenceRobItemSkills();

	void ClientEvent(uint16 sNpcID);
	void KissUser();

	void RecvMapEvent(Packet & pkt);
	void RecvSelectMsg(Packet & pkt);
	bool AttemptSelectMsg(uint8 bMenuID, int8 bySelectedReward);

	// from the client
	void ExchangeSystemProcess(Packet & pkt);
	void ItemUpgrade2(Packet& pkt, uint8 nUpgradeType = ITEM_UPGRADE);
	void ItemUpgrade(Packet & pkt, uint8 nUpgradeType = ITEM_UPGRADE);
	void ItemUpgradeNotice(_ITEM_TABLE pItem, uint8 UpgradeResult, uint32 scroll_id);
	void ItemUpgradeAccessories(Packet & pkt);
	void BifrostPieceSendFail(uint8 errorid);
	void BifrostPieceProcess(Packet & pkt); // originally named BeefRoastPieceProcess() -- that's not happening.
	void ShozinExchange(Packet & pkt);
	void ShozinSendFail(CraftingErrorCode erroid = CraftingErrorCode::WrongMaterial);
	void ItemDisassemble(Packet & pkt);
	void SendItemDisassembleFail(SmashExchangeError a);
	bool ItemSmashSystemRobItemCheck(uint8 bPos, uint32 nItemID, uint16 sCount = 1);
	void ItemUpgradeRebirth(Packet & pkt);
	void ItemSealProcess(Packet & pkt);
	void SealItem(uint8 bSealType, uint8 bSrcPos);
	void HactchingTransformExchange(Packet &pkt);
	void HatchingImageTransformExchange(Packet &pkt);
	void OtherExchangeSystem();
	bool CheckVipPassword(std::string aa);
	// Character Seal Process Related
	void CharacterSealProcess(Packet & pkt);
	void CharacterSealShowList(Packet &pkt);
	void CharacterSealUseScroll(Packet &pkt);
	void CharacterSealUseRing(Packet &pkt);
	void CharacterSealPreview(Packet &pkt);
	void CharacterSealAchieveList(Packet &pkt);
	void ShowCyperRingItemInfo(Packet & pkt, uint64 nSerialNum);

	void ReqCharacterSealProcess(Packet & pkt);
	void ReqCharacterSealShowList(Packet &pkt);
	void ReqCharacterSealUseScroll(Packet &pkt);
	void ReqCharacterSealUseRing(Packet &pkt);

	void ReqHatchingTransforms(Packet& pkt);

	void ShoppingMall(Packet & pkt);
	void HandleStoreOpen(Packet & pkt);
	void HandleStoreClose();
	void ReqPusSessionCreate(Packet & pkt);
	void LetterSystem(Packet & pkt);

	void ReqLetterSystem(Packet & pkt);
	void ReqLetterUnread();
	void ReqLetterList(bool bNewLettersOnly = true);
	void ReqLetterRead(Packet & pkt);
	void ReqLetterSend(Packet & pkt);
	void ReqLetterGetItem(Packet & pkt);
	void ReqLetterDelete(Packet & pkt);
	void ReqLetterGivePremiumItem(Packet & pkt);
	void ReqLetterGiveBeginnerItem(Packet & pkt);
	void HandleNameChange(Packet & pkt);
	void HandlePlayerNameChange(Packet & pkt);
	void HandleSelectCharacterNameChange(Packet & pkt);
	void HandlePlayerClanNameChange(Packet & pkt);
	void SendNameChange();
	void SendClanNameChange();

	void HandleHelmet(Packet & pkt);
	void HandleCapeChange(Packet & pkt);
	void SendCapeFail(int16 errorcode);
	void HandleChallenge(Packet & pkt);
	void HandleChallengeRequestPVP(Packet & pkt);
	void HandleChallengeRequestCVC(Packet & pkt);
	void HandleChallengeAcceptPVP(Packet & pkt);
	void HandleChallengeAcceptCVC(Packet & pkt);
	void HandleChallengeCancelled(uint8 opcode);
	void HandleChallengeRejected(uint8 opcode);

	// Ranking System
	void BorderDefenceAddPlayerRank();
	void ChaosExpansionAddPlayerRank();
	void PlayerKillingAddPlayerRank();
	void ZindanWarKillingAddPlayerRank();
	void BorderDefenceRemovePlayerRank();
	void ChaosExpansionRemovePlayerRank();
	void PlayerKillingRemovePlayerRank();
	void ZindanWarKillingRemovePlayerRank();
	void HandlePlayerRankings(Packet & pkt);
	void HandleRankingPKZone(Packet & pkt);
	int8 GetLoyaltySymbolRank();
	uint8 GetSymbol();
	void ShowEffectRankPlayer(uint16_t Ranking, int nation);
	void ShowEffectPlayer(uint16_t sEffect);
	time_t    m_tPlayerEffect;
	void HandleRankingSpecialEvent(Packet & pkt);
	void HandleRankingBDW(Packet & pkt);
	void HandleRankingChaosDungeon(Packet & pkt);
	void UpdateChaosExpansionRank();
	void UpdateBorderDefenceWarRank();
	void UpdatePlayerKillingRank();
	void UpdateZindanWarKillingRank();
	uint16 GetPlayerRank(uint8 nRankType);

	void HandleMiningSystem(Packet & pkt);
	void HandleMiningStart(Packet & pkt);
	void HandleMiningAttempt(Packet & pkt);
	uint32 GetMiningExpCount();
	std::vector<_MINING_FISHING_ITEM> MiningItemList(_ITEM_TABLE pItem);
	void HandleMiningStop();
	void HandleFishingStart(Packet & pkt);
	void HandleFishingAttempt(Packet & pkt);
	void HandleFishingStop(Packet & pkt);
	void HandleBettingGame(Packet & pkt);
	bool MiningFishingAreaCheck(uint8 Type);

	// Achievement System
	void HandleUserAchieve(Packet & pkt);
	void HandleUserAchieveGiftRequest(Packet & pkt);
	void HandleUserAchieveUserDetail(Packet & pkt);
	void HandleUserAchieveSummary(Packet & pkt);
	void HandleUserAchieveStart(Packet & pkt);
	void HandleUserAchieveStop(Packet & pkt);
	void HandleUserAchieveCoverTitle(Packet & pkt);
	void HandleUserAchieveSkillTitle(Packet & pkt);
	void HandleUserAchieveCoverTitleReset(Packet & pkt);
	void HandleUserAchieveSkillTitleReset(Packet & pkt);

	void UpdateAchievePlayTime();

	void UpdateAchieve(uint16 sIndex, uint8 Status);
	bool CheckAchieveStatus(uint16 sIndex, uint16 Status);
	void AchieveMonsterCountAdd(uint16 sNpcID);
	void AchieveWarCountAdd(UserAchieveWarTypes type, uint16 sNpcID = 0, CUser *pTarget = nullptr);
	void AchieveNormalCountAdd(UserAchieveNormalTypes type, uint16 sNpcID = 0, CUser *pTarget = nullptr);
	void AchieveComCountAdd(UserAchieveComTypes type, uint16 sNpcID = 0, CUser *pTarget = nullptr);

	void KnightReturnGiveLetterItem();

	void AchieveWarCheck(_USER_ACHIEVE_INFO *pAchieveData, _ACHIEVE_WAR *pAchieveWar, _ACHIEVE_MAIN *pAchieveMain, uint16 sNpcID = 0, CUser *pTarget = nullptr);
	void AchieveNormalCheck(_USER_ACHIEVE_INFO *pAchieveData, _ACHIEVE_NORMAL *pAchieveNormal, _ACHIEVE_MAIN *pAchieveMain, uint16 sNpcID = 0, CUser *pTarget = nullptr);
	bool AchieveComCheck(_USER_ACHIEVE_INFO *pAchieveData, _ACHIEVE_COM *pAchieveCom, _ACHIEVE_MAIN *pAchieveMain, uint16 sNpcID = 0, CUser *pTarget = nullptr);
	void AchieveMonsterCheck(_USER_ACHIEVE_INFO *pAchieveData, _ACHIEVE_MONSTER *pAchieveMonster, _ACHIEVE_MAIN *pAchieveMain, uint16 sNpcID);

	void SendGenieStart(bool isToRegion = false);
	void SendGenieStop(bool isToRegion = false);
	void HandleGenieStart();
	void HandleGenieStop();

	void HandleSoccer(Packet & pkt);

	// JstKO Settings Eklendi 30.12.2024
	void JstKOGmisOnlineNotice();
	void JstKOGmisOfflineNotice();
	void JstKOOyunOnlineNotice();
	void JstKOLoginUserisOnlineNotice();
	void JstKOSendInfoNotice();
	void JstKOGmInfoKomutNotice();
	void JstKOUserGameInfoNotice();
	void JstKOGateZoneNotice();
	void JstKOExprationItemList();
	void JstKOExprationItemDeleted();
	void JstKOWelcomeLogos(std::string LogosName, std::string LogosMessage, uint8 R, uint8 G, uint8 B);
	// JstKO Settings Eklendi The End 30.12.2024

	void SendNotice();
	void Clean_expnoahdropnp_notice(uint8 id);
	void SendFlashNotice(bool isRemove = false);
	void TopSendNotice();
	void SendCapeBonusNotice();
	void AppendNoticeEntry(Packet & pkt, uint8 & elementCount, const char * message, const char * title);
	void AppendNoticeEntryOld(Packet & pkt, uint8 & elementCount, const char * message);
	void AppendExtraNoticeData(Packet & pkt, uint8 & elementCount);
	void UserLookChange( int pos, int itemid, int durability );
	void SpeedHackUser();
	bool CheckInvisibility(CUser* ptarget);
	void SendLoginNotice();
	void LoyaltyChange(Unit* pdying, uint16 bonusNP = 0);
	void LoyaltyDivide(std::string placeowned = "-", Unit* pdying = nullptr, uint16 bonusNP = 0);
	int16 GetLoyaltyDivideSource(uint8 totalmember = 0);
	int16 GetLoyaltyDivideTarget();
	void GrantChickenManner();
	void SendMannerChange(int32 iMannerPoints);
	bool CanLevelQualify(uint8 sLevel);
	bool CanChangeZone(C3DMap * pTargetMap, WarpListResponse & errorReason);
	bool ZoneChange(uint16 sNewZone, 
		float x, float z, 
		int16 eventroom = -1, 
		bool check = true,
		bool arrest_summon = false,
		bool v_eventlogout = false
	);

	void ZoneChangeParty(uint16 sNewZone, float x, float z);
	void ZoneChangeClan(uint16 sNewZone, float x, float z);


	void BottomLeftRegionUserList(Packet & pkt);
	void BottomUserLogOut();
	void HandleBottomUserInfoDetail(Packet & pkt);

	void SurroundingAllInfo(bool first = false, bool change = false);

	void HandleGetUserInfoView(Packet &pkt);

	bool isInSoccerEvent();
	bool isEventUser();
	bool CheckDevaAttack(bool isnpc = false, uint16 protoid = 0);
	bool isSoccerEventUser();
	void isEventSoccerMember(uint8 TeamColours, float x, float z);
	void isEventSoccerStard();
	void isEventSoccerEnd();

	// Premium Related Methods
	void HandlePremium(Packet &pkt);
	void SendPremiumInfo();
	void GivePremium(uint8 bPremiumType, uint16 sPremiumTime, bool minute = false);
	void GiveSwitchPremium(uint8 bPremiumType, uint16 sPremiumTime); // Switch Premium Yukler Yuklemez 3 �n� birden goruntuluyecek sekilde yeni komut eklendi
	void GiveClanPremium(uint8 bPremiumType, uint32 sPremiumDay);
	void GivePremiumItem(uint8 bPremiumType);

	void SendClanPremium(CKnights* pKnights, bool exits = false);
	void SendClanPremiumPkt(CKnights* pknights, bool exits);
	uint32 GetClanPremiumTime(CKnights* pKnights);

	void SendAntiAfkList();
	void SendWheelData();
	void SendRightClickExchange();
	void SendLists();
	void SendCrRandomList();
	void DailyQuestSendList();
	void PusRefundSendList();
	void SendEventTimerList();
	void UpdateDailyQuestState(_DAILY_USERQUEST* pquest, uint8 newstate);
	void UpdateDailyQuestCount(uint16 monsterid);
	void DailyQuestFinished(_DAILY_USERQUEST* puserq, _DAILY_QUEST* pquest);
	void ReqDailyQuestSendReward(Packet &pkt);

	// Pet System
	void MainPetProcess(Packet& pkt);
	void SelectingModeFunction(Packet& pkt);
	void HandlePetUseSkill(Packet& pkt);
	void ShowPetItemInfo(Packet& pkt, uint64 nSerialNum);
	void PetSpawnProcess(bool LevelUp = false);
	void LootingPet(float x, float z);
	void SendPetHP(int tid, int damage = 0);
	void SendPetHpChange(int tid, int damage = 0);
	void SendPetExpChange(int32 iExp, int tid = -1);
	void SetPetInfo(PET_DATA * pPet, int tid = -1);
	void PetFeeding(Packet& pkt, uint8 bType);
	void PetSatisFactionUpdate(int16 amount);
	void PetOnDeath();
	void PetHome(uint16 x, uint16 y, uint16 z);

	void HandleTargetHP(Packet & pkt);
	void SendTargetHP(uint8 echo, int tid, int damage = 0, bool TargetStatus = true);
	std::vector<uint32_t> Skillsave2;
	//void SendTargetHP( uint8 echo, int tid, int damage = 0 );
	bool IsValidSlotPos( _ITEM_TABLE pTable, int destpos );
	void SetUserAbility(bool bSendPacket = true);
	float SetCoefficient();
	void LevelChange(uint8 level, bool bLevelUp = true);
	void SetSlotItemValue();
	void ApplySetItemBonuses(_SET_ITEM * pItem);
	void ApplyCastellanCapeBonueses(_KNIGHTS_CAPE * pCape);
	void ApplyRankBonueses(uint8 sType); /*26.04.2024 RANK BONUS SYSTEM*/
	void ClanPremiumBonusNotice(bool exits = false);
	void SendTime();
	void SendWeather();
	void SetZoneAbilityChange(uint16 sNewZone);
	void SetMaxMp();
	void SetMaxHp(int iFlag = 0);
	void RecvUserExp(CNpc* pNpc, uint32 iDamage, uint32 iTotalDamage);
	void ExpChange(std::string descp, int64 iExp, bool bIsBonusReward = false);
	void LogOut();
	void SendMyInfo(bool after = false);
	void MyInfo();
	void XSafe_ReqAutoUpgradeList(Packet& pkt);

	void RightTopTitleMsg(); 
	void HookUStatePanelRequest();
	void RightTopTitleMsgDelete(); 
	void SendServerChange(std::string & ip, uint8 bInit);
	uint16 GetPremiumProperty(PremiumPropertyOpCodes type);
	float GetPremiumPropertyExp(PremiumPropertyOpCodes type);
	uint16 GetClanPremiumProperty(PremiumPropertyOpCodes type);
	float GetClanPremiumPropertyExp(PremiumPropertyOpCodes type);

	void LogosShout(Packet & pkt);
	void EventTrapProcess(float x, float z, CUser * pUser);
	virtual void GetInOut(Packet & result, uint8 bType);
	void UserInOut(uint8 bType);
	void GmInOut(uint8 bType);
	void GmGetInOut(Packet& result, uint8 bType);
	
	void SiegeWarFareProcess(Packet & pkt);
	void DelosCasttellanZoneOut();
	bool isCswWinnerNembers();

	void GetUserInfo(Packet & pkt);
	void SendUserStatusUpdate(UserStatus type, UserStatusBehaviour status);
	virtual void Initialize();

	//Bdw Jr Monument
	void BDWMonumentPointProcess();
	void BDWAltarScreenAndPlayerPointChange();
	void BDWUserHasObtainedLoqOut();
	void BDWSendPacket(Packet& pkt);
	void BDWUpdateRoomKillCount();
	void JRUpdateRoomKillCount();
	void JuraidMountainWarp();
	void ExpSealHandler(Packet & pkt);
	void ExpSealUpdateStatus(uint8 status);
	void ExpSealChangeExp(uint64 amount);
	void ExpSealSealedPotion(Packet & pkt);
	void BDWUserLogOut();
	void KnightReturnLetterItemDeleted();
	void OreadsZoneTerrainEvent();
	void UserWallCheatCheckRegion();
	void ChangeFame(uint8 bFame);
	void SendServerIndex();

	/* Collection Race */
	_COLLECTION_RACE_EVENT_USER CollectionRace;
	COMMAND_HANDLER(HandleCollectionRaceStart);
	COMMAND_HANDLER(HandleCollectionRaceClose);
	void CollectionGetActiveTime();
	void CollectionRaceFirstLoad();
	void CollectionRaceHide();
	void CollectionRaceFinish();
	void GiveRandomItem(uint32& nItemID, uint32& nCount,uint8 bySession);
	/* Collection Race */

	// gm tbl kaydetme komutu tan�m� AA11BB22CC99
	COMMAND_HANDLER(HandleTBL);
	COMMAND_HANDLER(HandleProcInfo);

	void SendToRegion(Packet *pkt, CUser *pExceptUser = nullptr, uint16 nEventRoom = 0);
	void SendToZone(Packet *pkt, CUser *pExceptUser = nullptr, uint16 nEventRoom = 0, float fRange = 0.0f);
	void SendToMerchant(Packet *pkt, CUser *pExceptUser = nullptr, uint16 nEventRoom = 0, float fRange = 0.0f);
	void CharacterOlduguYerdeYenile();
	bool dupplicate_itemcheck(uint64 serial);

	virtual void OnDeath(Unit *pKiller);
	void OnDeathitDoesNotMatter(Unit *pKiller);
	void OnDeathKilledPlayer(CUser* pKiller);
	void OnDeathKilledBot(CBot* pKiller);
	void OnDeathKilledNpc(CNpc* pKiller);
	int64 OnDeathLostExpCalc(int64 maxexp);
	void InitOnDeath(Unit *pKiller);
	void UpdateAngerGauge(uint8 byAngerGauge);
	void InitializeStealth();

	// Exchange system
	bool BifrostCheckExchange(int nExchangeID);
	bool CheckExchange(int nExchangeID);
	bool CheckExchangeExp(int nExchangeID);
	bool RunExchange(int nExchangeID, uint16 count = 0 /* No random flag */,bool isQuest = false, int32 SelectedAward = -1, uint16 giveCount = 1);
	bool RunQuestExchange(int nExchangeID, int32 QuestIndex = -1);
	bool RunRandomExchange(int nExchangeID);
	bool RunQuestCheck(_ITEM_EXCHANGE* pExchange = nullptr);
	uint16 GetMaxExchange(int nExchangeID);	

	int8 bySelectedReward; uint8 bMenuID;
	void LogosItemNotice(_ITEM_TABLE ptable);

	void SendChat(uint8 chattype, std::string msg, std::string Sender = ""); //tekli pm

	bool RunCountExchange(int nExchangeID);
	bool RunCountCheckExchange(int nExchangeID);

	bool RunGiveItemExchange(int nExchangeID, uint8 Type);
	bool RunGiveItemCheckExchange(int nExchangeID);
	bool RunGiveCheckExistItem(int itemid, short count = 1);
	bool RunGiveCheckExistItemAnd(
		int32 nItemID1, int16 sCount1, int32 nItemID2, int16 sCount2, int32 nItemID3, int16 sCount3, int32 nItemID4, int16 sCount4, int32 nItemID5, int16 sCount5,
		int32 nItemID6, int16 sCount6, int32 nItemID7, int16 sCount7, int32 nItemID8, int16 sCount8, int32 nItemID9, int16 sCount9, int32 nItemID10, int16 sCount10);

	bool LuaCheckExchange(int nLuaExchangeID);
	void MiningExchange(uint16 ID);
	void FishingDropTester(uint8 WeaponType, uint8 Hour);
	void MiningDropTester(uint8 WeaponType, uint8 Hour);

	// Clan system
	void SendClanUserStatusUpdate(bool bToRegion = true);
	
	// Party system
	void SendPartyStatusUpdate(uint8 bStatus, uint8 bResult = 0);

	//Vaccuni system
	bool VaccuniBoxExp(uint16 MonsterType = 0);
	bool VaccuniAttack();
	int GetExpPercent();
	bool CanUseItem(uint32 nItemID, uint16 sCount = 1);

	void CheckSavedMagic();
	virtual void InsertSavedMagic(uint32 nSkillID, uint16 sDuration);
	virtual void RemoveSavedMagic(uint32 nSkillID);
	virtual bool HasSavedMagic(uint32 nSkillID);
	virtual int16 GetSavedMagicDuration(uint32 nSkillID);

	void SaveEvent(uint16 sQuestID, uint8 bQuestState);
	void DeleteEvent(uint16 sQuestID);
	bool CheckExistEvent(uint16 sQuestID, uint8 bQuestState);

	void QuestV2MonsterCountAdd(uint16 sNpcID);
	uint8 QuestV2CheckQuestFinished(uint16 sQuestID);
	uint8 QuestV2CheckMonsterCount(uint16 sQuestID, uint8 sRate);

	void QuestV2SaveEvent(uint16 sQuestID);
	void QuestV2SendNpcMsg(uint32 nQuestID, uint16 sNpcID);
	void QuestV2ShowGiveItem(uint32 ItemID1 = 0, uint32 ItemCount1 = 0,
		uint32 ItemID2 = 0, uint32 ItemCount2 = 0,
		uint32 ItemID3 = 0, uint32 ItemCount3 = 0,
		uint32 ItemID4 = 0, uint32 ItemCount4 = 0,
		uint32 ItemID5 = 0, uint32 ItemCount5 = 0,
		uint32 ItemID6 = 0, uint32 ItemCount6 = 0,
		uint32 ItemID7 = 0, uint32 ItemCount7 = 0,
		uint32 ItemID8 = 0, uint32 ItemCount8 = 0);

	void LuaGiveItemShowGiveItem(uint32 nUnk1, uint32 sUnk1,
		uint32 nUnk2, uint32 sUnk2,
		uint32 nUnk3, uint32 sUnk3,
		uint32 nUnk4, uint32 sUnk4,
		uint32 nUnk5, uint32 sUnk5,
		uint32 nUnk6, uint32 sUnk6,
		uint32 nUnk7, uint32 sUnk7,
		uint32 nUnk8, uint32 sUnk8,
		uint32 nUnk9, uint32 sUnk9,
		uint32 nUnk10 = 0, uint32 sUnk10 = 0);

	uint16 QuestV2SearchEligibleQuest(uint16 sNpcID);
	void QuestV2ShowMap(uint32 nQuestHelperID);

	// Sends the quest completion statuses
	void QuestDataRequest(bool gamestarted = false);
	void OpenEtcSkill(bool bAuto = true);
	void _getEtcList(std::vector<uint16>& metclist, bool all);
	// Handles new quest packets
	void QuestV2PacketProcess(Packet & pkt);
	void QuestV2RemoveEvent(uint16 sQuestID);
	void QuestV2MonsterDataRequest(uint16 sQuestID);
	void QuestV2ExecuteHelper(_QUEST_HELPER * pQuestHelper);
	void QuestV2CheckFulfill(_QUEST_HELPER * pQuestHelper);
	bool QuestV2RunEvent(_QUEST_HELPER * pQuestHelper, uint32 nEventID, int8 bSelectedReward = -1);

	bool PromoteUserNovice();
	bool PromoteUser();
	void PromoteClan(ClanTypeFlag byFlag);

	// CheatRoom
	void ChatRoomHandle(Packet & pkt);
	void ChatRoomCreate(Packet & pkt);
	void ChatRoomList(Packet & pkt);
	void ChatRoomJoin(Packet & pkt);
	void ChatRoomLeave(Packet & pkt);
	void ChatRoomKickOut(Packet & pkt);
	void ChatRoomKickOut(uint16 userID);
	void ChatRoomChat(std::string * strMessage, std::string strSender);
	void ChatRoomAdmin(Packet & pkt);
	void ChatRoomMemberoption(Packet & pkt);
	void ChatRoomChangeAdmin(Packet & pkt);
	void SendChatRoom(Packet & pkt);

	void KnightsClanBuffUpdate(bool sign = false, CKnights* pmyknight = nullptr);
	bool isMoral2Checking(Unit* pTarget, _MAGIC_TABLE pSkill);

	// Attack/zone checks
	bool isHostileTo(Unit * pTarget);
	bool isPriestUseZone();
	bool isInClanArena();
	bool isInMeleeArena();
	bool isInPartyArena();

	bool SendPrisonZoneArea();

	void ResetWindows();

	void CloseProcess();
	virtual ~CUser() {}

	/* Database requests */
	void ReqAccountLogIn(Packet & pkt);
	void ReqSelectNation(Packet & pkt);
	void ReqKickOutProcess(Packet & pkt);
	void ReqAllCharInfo(Packet & pkt);
	void ReqChangeHair(Packet & pkt);
	void ReqCreateNewChar(Packet & pkt);
	void ReqDeleteChar(Packet & pkt);
	void ReqSelectCharacter(Packet & pkt);
	void ReqSaveCharacter();
	void ReqUserLogOut();
	void ReqNationTransfer(Packet & pkt);
	void ReqRegisterClanSymbol(Packet & pkt);
	void ReqSetLogInInfo(Packet & pkt);
	void ReqRemoveCurrentUser(Packet & pkt);
	void ReqUserKickOut(Packet & pkt);
	void BattleEventResult(Packet & pkt);
	void ReqShoppingMall(Packet & pkt);
	void ReqLoadWebItemMall();
	void ReqSkillDataProcess(Packet & pkt);
	void ReqSkillDataSave(Packet & pkt);
	void ReqSkillDataLoad(Packet & pkt);
	void ReqFriendProcess(Packet & pkt);
	void ReqRequestFriendList(Packet & pkt);
	void ReqAddFriend(Packet & pkt);
	void ReqRemoveFriend(Packet & pkt);
	void NameChangeSystem(Packet & pkt);
	void ReqChangeCape(Packet & pkt);
	void ReqSealItem(Packet & pkt);
	void ReqItemUpgrade(Packet &pkt);


	//npc kill
	void KillNpcEvent();
	void NpcEventSystem(uint32 m_iSellingGroup);

	void ChangePosition();

	void ReqHandleUserDatabaseRequest(Packet& pkt);
	void ReqHandleEvent(Packet& pkt, CUser* pUser);
	void ReqSendLetterItem(Packet& pkt);
	void Addletteritem(_LINFO pitem);
	void ReqHandleXSafe(Packet &pkt);
	void ReqPPCard(Packet &pkt);

	//private:
	static ChatCommandTable s_commandTable;
	GameState m_state;

	// users quest map. this should hold all the info needed. 
	// the quest kill count for each quest, 
	QuestMap m_questMap; DailyQuestMap m_dailyquestmap;
	std::recursive_mutex m_ZoneOnlineRewardLock;
	ZoneOnlineReward m_ZoneOnlineReward;
	UserSavedMagicMap m_savedMagicMap;
	std::recursive_mutex m_savedMagicLock;
	SRWLock	 m_savedMagicLockx;
	_KNIGHTS_USER * m_pKnightsUser;

public:
	DECLARE_LUA_CLASS(CUser);

	// Standard getters
	DECLARE_LUA_GETTER(GetName)
	DECLARE_LUA_GETTER(GetAccountName)
	DECLARE_LUA_GETTER(GetZoneID)
	DECLARE_LUA_GETTER(GetX)
	DECLARE_LUA_GETTER(GetY)
	DECLARE_LUA_GETTER(GetZ)
	DECLARE_LUA_GETTER(GetNation)
	DECLARE_LUA_GETTER(GetLevel)
	DECLARE_LUA_GETTER(GetRebirthLevel)
	DECLARE_LUA_GETTER(GetExp)
	DECLARE_LUA_GETTER(GetClass)
	DECLARE_LUA_GETTER(GetCoins)
	DECLARE_LUA_GETTER(GetInnCoins)
	DECLARE_LUA_GETTER(GetLoyalty)
	DECLARE_LUA_GETTER(GetMonthlyLoyalty)
	DECLARE_LUA_GETTER(GetManner)
	DECLARE_LUA_GETTER(GetCash)
	DECLARE_LUA_GETTER(GetActiveQuestID)
	DECLARE_LUA_GETTER(GetClanGrade)
	DECLARE_LUA_GETTER(GetClanPoint)
	DECLARE_LUA_GETTER(GetClanRank)
	DECLARE_LUA_GETTER(isWarrior)
	DECLARE_LUA_GETTER(isRogue)
	DECLARE_LUA_GETTER(isMage)
	DECLARE_LUA_GETTER(isPriest)
	DECLARE_LUA_GETTER(isPortuKurian)
	DECLARE_LUA_GETTER(isBeginner)
	DECLARE_LUA_GETTER(isBeginnerWarrior)
	DECLARE_LUA_GETTER(isBeginnerRogue)
	DECLARE_LUA_GETTER(isBeginnerMage)
	DECLARE_LUA_GETTER(isBeginnerPriest)
	DECLARE_LUA_GETTER(isBeginnerKurianPortu)
	DECLARE_LUA_GETTER(isNovice)
	DECLARE_LUA_GETTER(isNoviceWarrior)
	DECLARE_LUA_GETTER(isNoviceRogue)
	DECLARE_LUA_GETTER(isNoviceMage)
	DECLARE_LUA_GETTER(isNovicePriest)
	DECLARE_LUA_GETTER(isNoviceKurianPortu)
	DECLARE_LUA_GETTER(isMastered)
	DECLARE_LUA_GETTER(isMasteredWarrior)
	DECLARE_LUA_GETTER(isMasteredRogue)
	DECLARE_LUA_GETTER(isMasteredMage)
	DECLARE_LUA_GETTER(isMasteredPriest)
	DECLARE_LUA_GETTER(isMasteredKurianPortu)
	DECLARE_LUA_GETTER(isInClan)
	DECLARE_LUA_GETTER(isClanLeader)
	DECLARE_LUA_GETTER(isInParty)
	DECLARE_LUA_GETTER(isPartyLeader)
	DECLARE_LUA_GETTER(isKing)
	DECLARE_LUA_GETTER(GetPartyMemberAmount)
	DECLARE_LUA_GETTER(GetPremium)
	DECLARE_LUA_GETTER(GetWarVictory)
	DECLARE_LUA_GETTER(GetMonsterChallengeTime)
	DECLARE_LUA_GETTER(GetMonsterChallengeUserCount)
	DECLARE_LUA_GETTER(GetUnderTheCastleOpen)
	DECLARE_LUA_GETTER(GetUnderTheCastleUserCount)
	DECLARE_LUA_GETTER(GetJuraidMountainTime)
	DECLARE_LUA_GETTER(GetRace)
	DECLARE_LUA_GETTER(BeefEventLogin)

	// Shortcuts for lazy people
	DECLARE_LUA_FUNCTION(GetQuestStatus) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GetQuestStatus(LUA_ARG(uint16, 2)));
	}

	DECLARE_LUA_FUNCTION(hasCoins)  {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->hasCoins(LUA_ARG(uint32, 2))); 
	}

	DECLARE_LUA_FUNCTION(hasInnCoins) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->hasInnCoins(LUA_ARG(uint32, 2))); 
	}

	DECLARE_LUA_FUNCTION(hasLoyalty) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->hasLoyalty(LUA_ARG(uint32, 2))); 
	}

	DECLARE_LUA_FUNCTION(hasMonthlyLoyalty) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->hasMonthlyLoyalty(LUA_ARG(uint32, 2)));
	}

	DECLARE_LUA_FUNCTION(hasManner) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->hasManner(LUA_ARG(uint32, 2))); 
	}

	// The useful method wrappers
	DECLARE_LUA_FUNCTION(GiveItem) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GiveItem(
			std::string("lua"),
			LUA_ARG(uint32, 2), 
			LUA_ARG_OPTIONAL(uint16, 1, 3), 
			true,
			LUA_ARG_OPTIONAL(uint32, 0, 4)));
	}
	
	DECLARE_LUA_FUNCTION(GenieExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GenieExchange(LUA_ARG(uint32, 2), LUA_ARG(uint32, 3)));
	}

	DECLARE_LUA_FUNCTION(GiveItemLua) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GiveItemLua(
			LUA_ARG(uint32, 2),
			LUA_ARG(uint32, 3),
			LUA_ARG(uint32, 4),

			LUA_ARG(uint32, 5),
			LUA_ARG(uint32, 6),
			LUA_ARG(uint32, 7),

			LUA_ARG(uint32, 8),
			LUA_ARG(uint32, 9),
			LUA_ARG(uint32, 10),

			LUA_ARG(uint32, 11),
			LUA_ARG(uint32, 12),
			LUA_ARG(uint32, 13),

			LUA_ARG(uint32, 14),
			LUA_ARG(uint32, 15),
			LUA_ARG(uint32, 16),

			LUA_ARG(uint32, 17),
			LUA_ARG(uint32, 18),

			LUA_ARG(uint32, 19),
			LUA_ARG(uint32, 20),

			LUA_ARG(uint32, 21),
			LUA_ARG(uint32, 22),

			LUA_ARG(uint32, 23),
			LUA_ARG(uint32, 24),

			LUA_ARG(uint32, 25),
			LUA_ARG(uint32, 26)));
	}

	DECLARE_LUA_FUNCTION(RobItem) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->RobItem(
			LUA_ARG(uint32, 2), 
			LUA_ARG_OPTIONAL(uint16, 1, 3)));
	}

	DECLARE_LUA_FUNCTION(RobAllItemParty) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->RobItem(
			LUA_ARG(uint32, 2), 
			LUA_ARG_OPTIONAL(uint16, 1, 3)));
	}

	DECLARE_LUA_FUNCTION(CheckExistItem) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->CheckExistItem(
			LUA_ARG(uint32, 2), 
			LUA_ARG_OPTIONAL(uint16, 1, 3)));
	}

	DECLARE_LUA_FUNCTION(GoldGain) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->LuaGoldGain(LUA_ARG(int32, 2)));
	}

	DECLARE_LUA_FUNCTION(GoldLose) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->GoldLose(LUA_ARG(uint32, 2)));	
	}

	DECLARE_LUA_FUNCTION(RunGiveItemExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->RunGiveItemExchange(LUA_ARG(uint32, 2),LUA_ARG_OPTIONAL(uint8,1,3)));
	}

	DECLARE_LUA_FUNCTION(ExpChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ExpChange(std::string("lua"), LUA_ARG(int32, 2), LUA_ARG(bool, 3)));
	}

	DECLARE_LUA_FUNCTION(SaveEvent) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->QuestV2SaveEvent(
			LUA_ARG(uint16, 2)));  // quest ID
	}

	DECLARE_LUA_FUNCTION(SearchQuest) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(pUser->QuestV2SearchEligibleQuest(LUA_ARG_OPTIONAL(uint16, pUser->m_sEventSid, 2))); // NPC ID
	}

	DECLARE_LUA_FUNCTION(ShowMap) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(pUser->QuestV2ShowMap(LUA_ARG_OPTIONAL(uint32, pUser->m_nQuestHelperID, 2))); // quest helper ID
	}

	DECLARE_LUA_FUNCTION(CountMonsterQuestSub) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->QuestV2CheckMonsterCount(LUA_ARG(uint16, 2), LUA_ARG(uint8, 3)));
	}

	DECLARE_LUA_FUNCTION(QuestCheckQuestFinished) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->QuestV2CheckQuestFinished((LUA_ARG(uint16, 2))));
	}

	DECLARE_LUA_FUNCTION(QuestCheckExistEvent) { // arg1:eventDataIndex arg2:eventStatus 
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->CheckExistEvent((LUA_ARG(uint16, 2)), (LUA_ARG(uint8, 3))));
	}

	DECLARE_LUA_FUNCTION(CountMonsterQuestMain) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->QuestV2MonsterCountAdd((LUA_ARG(uint16, 2))));
	} 

	DECLARE_LUA_FUNCTION(NpcSay) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}

		uint32 arg = 2; // start from after the user instance.
		int32 nTextID[8]; 

		foreach_array(i, nTextID)
			nTextID[i] = LUA_ARG_OPTIONAL(int32, -1, arg++);

		LUA_NO_RETURN(pUser->SendSay(nTextID));
	}

	// This is probably going to be cleaned up, as the methodology behind these menus is kind of awful.
	// For now, we'll just copy existing behaviour: that is, pass along a set of text IDs & button IDs.
	DECLARE_LUA_FUNCTION(SelectMsg) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}

		uint32 arg = 2; // start from after the user instance.
		int32 menuButtonText[MAX_MESSAGE_EVENT], 
			menuButtonEvents[MAX_MESSAGE_EVENT];
		uint8 bFlag = LUA_ARG(uint8, arg++);
		int32 nQuestID = LUA_ARG_OPTIONAL(int32, -1, arg++);
		int32 menuHeaderText = LUA_ARG(int32, arg++);

		foreach_array(i, menuButtonText)
		{
			menuButtonText[i] = LUA_ARG_OPTIONAL(int32, -1, arg++);
			menuButtonEvents[i] = LUA_ARG_OPTIONAL(int32, -1, arg++);
		}

		LUA_NO_RETURN(pUser->SelectMsg(bFlag, nQuestID, menuHeaderText, menuButtonText, menuButtonEvents));
	}

	DECLARE_LUA_FUNCTION(NpcMsg) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}

		LUA_NO_RETURN(pUser->QuestV2SendNpcMsg(
			LUA_ARG(uint32, 2),
			LUA_ARG_OPTIONAL(uint16, pUser->m_sEventSid, 3)));
	}

	DECLARE_LUA_FUNCTION(CheckWeight) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->CheckWeight(
			LUA_ARG(uint32, 2),		// item ID
			LUA_ARG_OPTIONAL(uint16, 1, 3)));	// stack size
	}

	DECLARE_LUA_FUNCTION(CheckSkillPoint) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->CheckSkillPoint(
			LUA_ARG(uint8, 2),		// skill point category
			LUA_ARG(uint8, 3),		// min
			LUA_ARG(uint8, 4)));	// max
	}

	DECLARE_LUA_FUNCTION(isRoomForItem) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->FindSlotForItem(
			LUA_ARG(uint32, 2),					// item ID
			LUA_ARG_OPTIONAL(uint16, 1, 3)));	// stack size
	}

	DECLARE_LUA_FUNCTION(CheckGiveSlot) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->LuaCheckGiveSlot(LUA_ARG(uint8, 2)));	// Slot Control
	}

	DECLARE_LUA_FUNCTION(CheckExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->CheckExchange(LUA_ARG(uint32, 2)));	// exchange ID
	}

	DECLARE_LUA_FUNCTION(RunQuestExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->RunQuestExchange(
			LUA_ARG(uint32, 2), // EXCHANGE ID
			LUA_ARG_OPTIONAL(int32, -1, 3))); // QUESTNUM
	}
	
	DECLARE_LUA_FUNCTION(RunRandomExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->RunRandomExchange(
			LUA_ARG(uint32, 2))); // QUESTNUM
	}

	DECLARE_LUA_FUNCTION(RunMiningExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->MiningExchange(LUA_ARG(uint16, 2)));
	}
	DECLARE_LUA_FUNCTION(KissUser) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->KissUser());
	}

	DECLARE_LUA_FUNCTION(PromoteUserNovice) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->PromoteUserNovice());
	}

	DECLARE_LUA_FUNCTION(PromoteUser) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->PromoteUser());
	}

	DECLARE_LUA_FUNCTION(ShowEffect) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ShowEffect(LUA_ARG(uint32, 2))); // effect ID
	}

	DECLARE_LUA_FUNCTION(ShowNpcEffect) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ShowNpcEffect(LUA_ARG(uint32, 2))); // effect ID
	}

	DECLARE_LUA_FUNCTION(ZoneChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ZoneChange(
			LUA_ARG(uint16, 2),		// zone ID
			LUA_ARG(float, 3),		// x
			LUA_ARG(float, 4)));	// z
	}

	DECLARE_LUA_FUNCTION(ZoneChangeParty) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ZoneChangeParty(
			LUA_ARG(uint16, 2),		// zone ID
			LUA_ARG(float, 3),		// x
			LUA_ARG(float, 4)));	// z
	}

	DECLARE_LUA_FUNCTION(ZoneChangeClan) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ZoneChangeClan(
			LUA_ARG(uint16, 2),		// zone ID
			LUA_ARG(float, 3),		// x
			LUA_ARG(float, 4)));	// z
	}

	DECLARE_LUA_FUNCTION(SendNameChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendNameChange());
	}

	DECLARE_LUA_FUNCTION(SendClanNameChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendClanNameChange());
	}

	DECLARE_LUA_FUNCTION(SendStatSkillDistribute) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendStatSkillDistribute());
	}

	DECLARE_LUA_FUNCTION(SendTagNameChangePanel) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendTagNameChangePanel());
	}

	DECLARE_LUA_FUNCTION(ResetStatPoints) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->AllPointChange2(false));
	}

	DECLARE_LUA_FUNCTION(ResetSkillPoints) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->AllSkillPointChange2(false));
	}

	DECLARE_LUA_FUNCTION(GiveLoyalty) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendLoyaltyChange("-", LUA_ARG(int32, 2), false, false, false));
	}

	DECLARE_LUA_FUNCTION(RobLoyalty) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendLoyaltyChange("-", -(LUA_ARG(int32, 2)), false, false, false));
	}

	DECLARE_LUA_FUNCTION(GiveBalance) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->GiveBalance(LUA_ARG(uint32, 2), LUA_ARG(uint32, 3)));
	}

	DECLARE_LUA_FUNCTION(ShowBulletinBoard) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ShowBulletinBoard());	
	}

	DECLARE_LUA_FUNCTION(ChangeManner) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendMannerChange(LUA_ARG(int32, 2)));	
	}

	DECLARE_LUA_FUNCTION(PromoteClan) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->PromoteClan((ClanTypeFlag)LUA_ARG_OPTIONAL(uint8, (uint8)ClanTypeFlag::ClanTypePromoted, 2)));
	}

	DECLARE_LUA_FUNCTION(GetStat) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GetStat((StatType)(LUA_ARG(uint8, 2) + 1)));	
	}

	DECLARE_LUA_FUNCTION(RobClanPoint) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendClanPointChange(-(LUA_ARG(int32, 2))));	
	}

	DECLARE_LUA_FUNCTION(RequestPersonalRankReward) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GetRankReward(true));
	}

	DECLARE_LUA_FUNCTION(RequestReward) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GetRankReward(false));
	}


	//npckill
	DECLARE_LUA_FUNCTION(SendNpcKillID) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendNpcKillID((LUA_ARG(uint32, 2))));
	}

	DECLARE_LUA_FUNCTION(RunExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->RunExchange(
			LUA_ARG(int, 2),		
			LUA_ARG(uint16, 3)));
	}

	DECLARE_LUA_FUNCTION(GetMaxExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GetMaxExchange((LUA_ARG(int, 2))));
	}

	DECLARE_LUA_FUNCTION(RunCountExchange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->RunCountExchange((LUA_ARG(int, 2))));
	}

	DECLARE_LUA_FUNCTION(GetUserDailyOp) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GetUserDailyOp((LUA_ARG(uint8, 2))));
	}

	DECLARE_LUA_FUNCTION(GetEventTrigger) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GetEventTrigger());
	}

	DECLARE_LUA_FUNCTION(CheckMiddleStatueCapture) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->CheckMiddleStatueCapture());
	}

	DECLARE_LUA_FUNCTION(MoveMiddleStatue) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->MoveMiddleStatue());
	}

	DECLARE_LUA_FUNCTION(LevelChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->LevelChange(LUA_ARG(uint8, 2), false));
	}

	DECLARE_LUA_FUNCTION(GivePremium) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->GivePremium(LUA_ARG(uint8, 2), LUA_ARG_OPTIONAL(uint8, 1, 3)));
	}

	DECLARE_LUA_FUNCTION(GiveSwitchPremium) { // Switch Premium Yukler Yuklemez 3 �n� birden goruntuluyecek sekilde yeni komut eklendi
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->GiveSwitchPremium(LUA_ARG(uint8, 2), LUA_ARG_OPTIONAL(uint8, 1, 3)));
	}

	DECLARE_LUA_FUNCTION(GiveClanPremium) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->GiveClanPremium(LUA_ARG(uint8, 2), LUA_ARG_OPTIONAL(uint8, 1, 3)));
	}

	DECLARE_LUA_FUNCTION(GivePremiumItem) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->GivePremiumItem(LUA_ARG(uint8, 2)));
	}

	DECLARE_LUA_FUNCTION(GetPVPMonumentNation) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GetPVPMonumentNation());
	}

	DECLARE_LUA_FUNCTION(NationChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->NationChange());
	}

	DECLARE_LUA_FUNCTION(GenderChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->GenderChange((LUA_ARG(uint8, 2))));
	}

	DECLARE_LUA_FUNCTION(JobChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->JobChange(
			LUA_ARG(uint8, 2),
			LUA_ARG(uint8, 3)));
	}

	DECLARE_LUA_FUNCTION(EventSoccerMember) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->isEventSoccerMember(
			LUA_ARG(uint8, 2),		// Soccer Team
			LUA_ARG(float, 3),		// x
			LUA_ARG(float, 4)));	// z
	}

	DECLARE_LUA_FUNCTION(EventSoccerStard) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->isEventSoccerStard());
	}

	DECLARE_LUA_FUNCTION(SpawnEventSystem) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SpawnEventSystem(
			LUA_ARG(uint16, 2),		//	sSid
			LUA_ARG(uint8, 3),		//	(NPC 0 Monster 1)
			LUA_ARG(uint8, 4),		//	ZONEID
			LUA_ARG(float, 5),		//	x
			LUA_ARG(float, 6),		//	y
			LUA_ARG(float, 7)));	//	z
	}

	DECLARE_LUA_FUNCTION(SendNationTransfer) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendNationTransfer());
	}

	DECLARE_LUA_FUNCTION(RebirthBas) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->RebirthBas());
	}

	DECLARE_LUA_FUNCTION(SendRepurchaseMsg) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->SendRepurchaseMsg());
	}

	DECLARE_LUA_FUNCTION(JoinEvent) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->TempleJoinEvent(TEMPLE_EVENT_JURAD_MOUNTAIN)); // nIndex or quest ID
	}

	DECLARE_LUA_FUNCTION(KillNpcEvent) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->KillNpcEvent());
	}

	DECLARE_LUA_FUNCTION(NpcEventSystem) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->NpcEventSystem((LUA_ARG(uint32, 2)))); // NPoints Amount
	}

	DECLARE_LUA_FUNCTION(KingsInspectorList) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->KingsInspectorList());
	}

	DECLARE_LUA_FUNCTION(isCswWinnerNembers) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->isCswWinnerNembers());
	}

	DECLARE_LUA_FUNCTION(DelosCasttellanZoneOut) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->DelosCasttellanZoneOut());
	}

	DECLARE_LUA_FUNCTION(CycleSpawn) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ChangePosition());
	}

	DECLARE_LUA_FUNCTION(DrakiRiftChange) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->DrakiRiftChange(
			LUA_ARG(uint16, 2),
			LUA_ARG(uint16, 3)));
	}

	DECLARE_LUA_FUNCTION(DrakiOutZone) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->DrakiTowerKickOuts());
	}

	DECLARE_LUA_FUNCTION(DrakiTowerNpcOut) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->DrakiTowerNpcOut());
	}

	DECLARE_LUA_FUNCTION(ClanNts) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_NO_RETURN(LUA_GET_INSTANCE()->ClanNtsHandler());
	}
	
	DECLARE_LUA_FUNCTION(PerkUseItem) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(LUA_GET_INSTANCE()->PerkUseItem(
			LUA_ARG(uint32, 2),
			LUA_ARG(uint32, 3),
			LUA_ARG(uint16, 4)));
	}

	DECLARE_LUA_FUNCTION(MonsterStoneQuestJoin) {
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}

		LUA_RETURN(pUser->MonsterStoneQuestJoin(
			LUA_ARG(uint16, 2),
			LUA_ARG_OPTIONAL(uint8, 0, 3),
			LUA_ARG_OPTIONAL(uint8, 0, 4)));
	}

	DECLARE_LUA_FUNCTION(GetExpPercent)
	{
		CUser* pUser = LUA_GET_INSTANCE();
		if (!pUser || !pUser->isInGame()) {
			LUA_RETURN(0);
		}
		LUA_RETURN(pUser->GetExpPercent());
	}
};
