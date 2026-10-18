#include "stdafx.h"
#include "KnightsManager.h"
#include "KingSystem.h"
#include "MagicInstance.h"
#include "DBAgent.h"
#include <algorithm>
#include "../shared/DateTime.h"

using namespace std;

#pragma region Constructor
/**
* @brief The constructor.
*/
CUser::CUser(uint16 socketID, SocketMgr* mgr) : KOSocket(socketID, mgr, -1, 1024 * 64, 1024 * 64/*32768, 9172*/), Unit(UnitType::UnitPlayer)
{
	//Initialize();
}
#pragma endregion

#pragma region CUser::OnConnect()
/**
* @brief	Executes the connect action.
*/
void CUser::OnConnect()
{
	KOSocket::OnConnect();
	Initialize();
}
#pragma endregion

#pragma region CUser::Initialize()
/**
* @brief	Initializes this object.
*/
void CUser::Initialize()
{
	m_VsDurumu = true;
	newChar = false;
	Unit::Initialize();
	m_reloadChestBlock = false;
	m_drop_scroll_amount = 0;
	off_merchantcheck = 0;
	to_gm_pmName = 0;
	m_zoneonrewardtime = 0;
	level_merchant_time = 0;
	m_serialitemlistLock.lock();
	m_EventDisandTime = 0;
	nHardwareID = 0;
	TimeTravelTemp = 0;
	m_bSlaveMerchant = false;
	m_bSlaveUserID = -1;
	SlaveTotalTime = 0;
	m_bUserPriestBotID = -1;
	hasPriestBot = false;
	HealYuzde = 0;
	Buffbool = 0;
	AccBool = 0;
	HealBool = 0;
	m_randomItem = 0;
	autoupscrolltype = 0;
	m_AutoUpgrade = false;
	m_AutoUpgradeCount = 0;
	m_AutoUpgradeisFast = 0;
	m_serialitemlist.clear();
	m_serialitemlistLock.unlock();
	m_addedmap = false;
	pCindWar.Initialize();
	m_gmsendpmtime = 0;
	m_jackpotype = 0;
	m_gmsendpmid = -1;
	m_flameilevel = 0;
	m_flametime = 0;
	m_event_afkcheck = 0;
	m_pusgifttime = 0;
	lastTickTime = 31;
	lastTickTime2 = 31;
	m_testkillcount = 0;
	m_clanntsreq = false;
	m_autoloot = false;
	m_fairycheck = false;
	m_strUserID.clear();
	m_strMemo.clear();
	m_strAccountID.clear();
	mytagname.clear();
	m_bLogout = 0;
	m_CraftingTime = 0;
	m_tagnamergb = 0;
	m_SmashingTime = 0;
	m_oldTargetID = -1;
	m_ismsevent = false;
	GMUpgradeThis = false; // JstKO GM Full Upgrade %100 Özel Eklendi 05.01.2025
	m_BeefExchangeTime = UNIXTIME2;

	m_lifeSkills.WarExp = 0;
	m_lifeSkills.HuntingExp = 0;
	m_lifeSkills.SmitheryExp = 0;
	m_lifeSkills.KarmaExp = 0;
	m_lifeSkillsLoaded = false;
	isCheckDecated = false;
	m_pusrefundtime = 0;
	m_lastrattacktime = 0;

	m_bCharacterDataLoaded = false;
	m_bAchieveLoginTime = 0;

	m_lastPetSatisfaction = m_AutoMiningFishingSystem = UNIXTIME;
	m_SlavePriestSystem = UNIXTIME;
	m_PettingOn = nullptr;

	memset(&m_strSkillData, 0x0, sizeof(m_strSkillData));
	m_strSkillCount = 0;
	m_bHasAlterOptained = false;
	m_SkillCastList.clear();
	m_BottomUserListTime = 0;
	m_iLastExchangeTime = 0;
	m_surallinfotime = 0;
	m_tLastDelayedCheck = 0;
	m_onlinecashtime = 0;
	XSafe_LASTCHECK = UNIXTIME;
	m_lasttargetnumbertime = UNIXTIME2;
	CrStage = 0;
	AutoGemCount = 0;
	AutoGemSell = 0;
	AutoGemIn = 0;
	AutoGemStart = 0;
	killmoney = killuser = 0;
	m_bKnightsReq = 0;
	m_petrespawntime = 0;
	m_iTotalTrainingExp = 0;
	m_lastTrainingTime = 0;
	m_iTotalTrainingTime = 0;
	m_MaxSp = 100;
	m_FlashExpBonus = 0;
	m_FlashDcBonus = 0;
	m_FlashWarBonus = 0;
	m_tGenieTimeNormal = m_flashchecktime = UNIXTIME;
	m_TownTime = 0;
	m_1098GenieTime = 0;
	m_sFirstUsingGenie = 0;
	priesterrorcount = 0;
	memset(m_GenieOptions, 0, sizeof(m_GenieOptions));

	/*m_sendknightflaglist = false;*/
	m_offmerchanttime = 0;

	m_tLastChatUseTime = UNIXTIME2;
	m_tLastType4SkillCheck = 0;
	m_tLastType69SkillCheck = 0;

	m_TowerOwnerID = -1;
	m_ClanExpBonus = 0;
	m_ClanNPBonus = 0;

	behav_type = UserStatus::USER_STATUS_DOT;
	behav_status = UserStatusBehaviour::USER_STATUS_CURE;

	// Achiement Related Initialization
	m_sCoverID = 0;
	m_sSkillID = 0;
	m_sCoverTitle = 0;
	m_sSkillTitle = 0;
	GateActimmi = false; // Gate Dışardan Paket Fix
	m_tGuardAliveExpiryTime = 0;
	isAryaOnlineWorld = false;
	m_bAuthority = 1;
	m_sBind = -1;
	m_ChatRoomIndex = -1;
	m_state = GameState::GAME_STATE_CONNECTED;

	PusArrayControl = false;
	TempItemsControl = false;

	m_bSelectedCharacter = false;
	m_bStoreOpen = false;
	m_bPartyLeader = false;
	m_bPartyCommandLeader = false;
	isBottomSign = false;
	m_bIsChicken = false;
	m_bIsHidingHelmet = false;
	m_bIsHidingCospre = false;
	m_bMining = false;
	m_bFishing = false;
	m_bPremiumMerchant = false;
	m_bInParty = false;
	m_bInEnterParty = false;
	sClanPremStatus = false;

	m_tLastMiningAttempt = UNIXTIME2; //16.09.2020 <- Seri mining&fishing yapma fix.
	m_tUpgradeDelay = UNIXTIME; //30.08.2020 Seri Upgrade Fix
	m_bMerchantState = MERCHANT_STATE_NONE;
	m_bSellingMerchantPreparing = false;
	m_bBuyingMerchantPreparing = false;
	m_bInvisibilityType = (uint8)InvisibilityType::INVIS_NONE;

	m_sDirection = 0;

	m_sItemMaxHp = m_sItemMaxMp = 0;
	m_sItemWeight = 0;
	m_sItemAc = 0;
	m_SpeedHackTrial = 0;
	m_bNPGainAmount = m_bNoahGainAmount = 100;
	m_sExpGainbuff33Amount = m_sExpGainbuff11Amount = 0;
	m_bItemExpGainAmount = m_bItemNoahGainAmount = 0;
	m_bItemNPBonus = m_bSkillNPBonus = 0;
	m_KillCount = 0;
	m_byAPBonusAmount = 0;
	memset(&m_byAPClassBonusAmount, 0, sizeof(m_byAPClassBonusAmount));
	memset(&m_byAcClassBonusAmount, 0, sizeof(m_byAcClassBonusAmount));
	memset(&m_bStats, 0, sizeof(m_bStats));
	memset(&m_sStatItemBonuses, 0, sizeof(m_sStatItemBonuses));
	memset(&m_sStatAchieveBonuses, 0, sizeof(m_sStatAchieveBonuses));
	memset(&m_bStatBuffs, 0, sizeof(m_bStatBuffs));
	memset(&m_bRebStats, 0, sizeof(m_bRebStats));
	memset(&m_bstrSkill, 0, sizeof(m_bstrSkill));
	memset(&m_arMerchantItems, 0, sizeof(m_arMerchantItems));
	memset(m_dailyrankrefresh, false, sizeof(m_dailyrankrefresh));
	AchieveChalengeCount = false;

	m_bAddWeaponDamage = 0;
	m_bPctArmourAc = 100;
	m_sAddArmourAc = 0;

	m_FreeRepurchaseID.clear();
	m_DeleteItemList.clear();

	mChestBlockItemLock.lock();
	mChestBlockItem.clear();
	mChestBlockItemLock.unlock();

	m_ZoneOnlineRewardLock.lock();
	m_ZoneOnlineReward.clear();
	m_ZoneOnlineRewardLock.unlock();

	m_sItemHitrate = 100;
	m_sItemEvasionrate = 100;
	m_bPlayerAttackAmount = 100;
	m_sSpeed = 0;

	m_bAuthority = (uint8)AuthorityTypes::AUTHORITY_PLAYER;
	m_bLevel = 1;
	m_bRebirthLevel = 0;
	m_iExp = 0;
	m_iBank = m_iGold = m_nKnightCash = 0;
	m_iLoyalty = m_iLoyaltyMonthly = 0;
	m_iMannerPoint = 0;
	m_sHp = m_sMp = m_sSp = 0;

	m_MaxHp = 0;
	m_MaxMp = 1;
	m_MaxSp = 0;
	m_iMaxExp = 0;
	m_sMaxWeight = 0;
	m_sMaxWeightBonus = 0;
	sKilledCount = 0;
	m_sUserUpgradeCount = 0;

	m_bResHpType = USER_STANDING;
	m_bWarp = false;
	m_bCheckWarpZoneChange = false;

	m_sMerchantsSocketID = -1;
	m_sChallengeUser = -1;
	m_sPartyIndex = -1;
	m_bRequestingChallenge = 0;
	m_bChallengeRequested = 0;

	m_sExchangeUser = -1;
	m_sTradeStatue = 1;
	m_isRequestSender = false;
	m_ExchangeItemList.clear();

	m_bBlockPrivateChat = false;
	m_sPrivateChatUser = 0;
	m_bNeedParty = 0x01;

	m_tHPLastTimeNormal = 0;		// For Automatic HP recovery. 
	m_tHPStartTimeNormal = 0;

	m_tSpLastTimeNormal = 0;		// For Automatic SP recovery. 
	m_tSpStartTimeNormal = 0;
	m_bSpIntervalNormal = 1;

	m_JobChangeTime = 0;
	SkillFailCount = 0;

	m_bHPAmountNormal = 0;
	m_bHPDurationNormal = 0;
	m_bHPIntervalNormal = 5;

	m_fSpeedHackClientTime = 0;
	m_fSpeedHackServerTime = 0;
	m_bSpeedHackCheck = 0;

	m_tBlinkExpiryTime = 0;

	m_tLastArrowUse = 0;

	m_mutetimestatus = m_attacktimestatus = 0;
	ChickenStatus = 0;
	m_iEventJoinOrder = 0;
	m_sJoinedEvent = -1;
	m_isFinalJoinedEvent = false;
	m_flashtime = 0;
	m_flashcount = m_flashtype = 0;

	m_bAbnormalType = ABNORMAL_NORMAL;	// User starts out in normal size.
	m_nOldAbnormalType = m_bAbnormalType;

	m_sWhoKilledMe = -1;
	m_iLostExp = 0;

	m_questMap.DeleteAllData();
	m_PremiumMap.DeleteAllData();
	m_AchieveInfo.Initialize();
	m_AchieveMap.DeleteAllData();
	m_AchieveTimedMap.DeleteAllData();
	m_dailyquestmap.DeleteAllData();
	m_PusRefundMap.DeleteAllData();

	ReturnSymbolisOK = 0;
	ReturnisLogOutTime = 0;
	ReturnSymbolTime = 0;

	m_sUserPartyType = 0;
	m_bPremiumInUse = 0;
	m_bClanPremiumInUse = 0;

	m_iSealedExp = 0;
	bExpSealStatus = 0;
	m_iDrakiTime = 0;
	m_bDrakiStage = 0;
	m_bDrakiEnteranceLimit = 0;

	m_bMaxWeightAmount = 100;

	m_tLastTrapAreaTime = 0;
	memset(m_iSelMsgEvent, -1, sizeof(m_iSelMsgEvent));
	m_sMilitaryTime = 0;
	m_switchPremiumCount = 0;
	m_sEventNid = m_sEventSid = -1;
	m_nQuestHelperID = 0;
	m_bZoneChangeFlag = false;
	m_bRegeneType = 0;
	m_tLastRegeneTime = 0;
	m_bZoneChangeSameZone = false;
	m_bZoneChangeControl = false;

	m_pKnightsUser = nullptr;

	m_sRivalID = -1;
	m_tRivalExpiryTime = 0;
	m_tPlayerEffect = 0;

	m_byAngerGauge = 0;

	m_bWeaponsDisabled = false;

	m_teamColour = TeamColour::TeamColourNone;
	m_bGenieStatus = false;
	m_iswanted = false;
	m_iswantedtime = 0;
	m_bAccountStatus = 0;
	
	// JstKO Yeni Cz Pvp Kill Ödül Notice Eklendi. 12.04.2025
	m_PvpKillCount = 0;
	m_PvpKill50Count = 0;
	m_PvpKill500Count = 0;
	// JstKO Yeni Cz Pvp Kill Ödül Notice Eklendi. The End. 12.04.2025

	for (int i = 0; i < 5; i++)
	{
		m_bPremiumType[i] = 0;
		m_sPremiumTime[i] = 0;
	}

	m_GameMastersMute = 0;
	m_GameMastersUnMute = 0;
	m_GameMastersUnBan = 0;
	m_GameMastersBanPermit = 0;
	m_GameMastersBanUnder = 0;
	m_GameMastersBanDays = 0;
	m_GameMastersBanCheating = 0;
	m_GameMastersBanScamming = 0;
	m_GameMastersAllowAttack = 0;
	m_GameMastersDisabledAttack = 0;
	m_GameMastersNpAdd = 0;
	m_GameMastersExpAdd = 0;
	m_GameMastersMoneyAdd = 0;
	m_GameMastersLoyaltyChange = 0;
	m_GameMastersExpChange = 0;
	m_GameMastersMoneyChange = 0;
	m_GameMastersGiveItem = 0;
	m_GameMastersGiveItemSelf = 0;
	m_GameMastersSummonUser = 0;
	m_GameMastersTpOnUser = 0;
	m_GameMastersZoneChange = 0;
	m_GameMastersLocationChange = 0;
	m_GameMastersMonsterSummon = 0;
	m_GameMastersNpcSummon = 0;
	m_GameMastersMonKilled = 0;
	m_GameMastersTeleportAllUser = 0;
	m_GameMastersClanSummon = 0;
	m_GameMastersResetPlayerRanking = 0;
	m_GameMastersChangeEventRoom = 0;
	m_GameMastersWarOpen = 0;
	m_GameMastersWarClose = 0;
	m_GameMastersCaptainElection = 0;
	m_GameMastersSendPrsion = 0;
	m_GameMastersKcChange = 0; // Kc Verme Yetkisi 27.09.2020
	m_GameMastersReloadTable = 0;
	m_GameMastersDropTest = 0;

	m_lockCoolDownList.lock();
	m_CoolDownList.clear();
	m_lockCoolDownList.unlock();

	pUserLootSetting.initiliaze();
	pUserMagicUsed.Initialize();
	pUserDailyRank.Initialize();
	pAssistKill.Initialize();
	pMove.Initialize();
	pUserTbInfo.Initialize();
	pPerks.Initialize();
	for (int i = 0; i < 10000; i++) m_FreeRepurchaseID.push_back(short(i));

	m_LastX = 0;
	m_LastZ = 0;
	m_bIsLoggingOut = false;
	m_bWaitingOrder = false;
	m_tOrderRemain = UNIXTIME + 5;

	m_equippedItemBonusLock.lock();
	m_equippedItemBonuses.clear();
	m_equippedItemBonusLock.unlock();

	m_buffLock.lock();
	m_buffMap.clear();
	m_buffLock.unlock();

	m_buff9lock.lock();
	m_type9BuffMap.clear();
	m_buff9lock.unlock();

	m_savedMagicLock.lock();
	m_savedMagicMap.clear();
	m_savedMagicLock.unlock();

	memset(m_sLevelArray, 0, sizeof(m_sLevelArray));
	CollectionRace.Initialize();
	//m_iteminfo.Initialize();
}
#pragma endregion

#pragma region CUser::OnDisconnect()
void CUser::OnDisconnect()
{
	if (isSlaveMerchant())
	{
		CBot* pSlaveUser = g_pMain->m_MapBotList.GetData(m_bSlaveUserID);
		if (pSlaveUser != nullptr) {
			GiveSlaveMerchantItems();
			pSlaveUser->UserInOut(INOUT_OUT);
		}

		m_bSlaveMerchant = false;
		m_bSlaveUserID = -1;
	}

	if (g_pMain->pServerSetting.GmisOnlineNotice) // JstKO Giriş Gm Offline Notice Eklendi 30.12.2024
		JstKOGmisOfflineNotice();

	LogOut();
	KOSocket::OnDisconnect();
}
#pragma endregion
std::string PacketSend(uint8 Opcode)
{
	std::string PacketHeader;
	switch (Opcode)
	{
	case WIZ_NPC_MOVE:
		return PacketHeader = "WIZ_NPC_MOVE";
		break;
	case WIZ_NPC_INOUT:
		return PacketHeader = "WIZ_NPC_INOUT";
		break;
	case WIZ_MOVE:
		return PacketHeader = "WIZ_MOVE";
		break;
	case WIZ_CHAT:
		return PacketHeader = "WIZ_CHAT";
		break;
	case WIZ_REQ_USERIN:
		return PacketHeader = "WIZ_REQ_USERIN";
		break;
	case WIZ_MYINFO:
		return PacketHeader = "WIZ_MYINFO";
		break;
	case WIZ_COMPRESS_PACKET:
		return PacketHeader = "WIZ_COMPRESS_PACKET";
		break;
	case 0xA0:
		return PacketHeader = "XIGNCODE";
		break;
	case WIZ_LOGIN:
		return PacketHeader = "WIZ_LOGIN";
		break;
	case WIZ_NEW_CHAR:
		return PacketHeader = "WIZ_NEW_CHAR";
		break;
	case WIZ_DEL_CHAR:
		return PacketHeader = "WIZ_DEL_CHAR";
		break;
	case WIZ_SEL_CHAR:
		return PacketHeader = "WIZ_SEL_CHAR";
		break;
	case WIZ_SEL_NATION:
		return PacketHeader = "WIZ_SEL_NATION";
		break;
	case WIZ_USER_INOUT:
		return PacketHeader = "WIZ_USER_INOUT";
		break;
	case WIZ_ATTACK:
		return PacketHeader = "WIZ_ATTACK";
		break;
	case WIZ_ROTATE:
		return PacketHeader = "WIZ_ROTATE";
		break;
	case WIZ_ALLCHAR_INFO_REQ:
		return PacketHeader = "WIZ_ALLCHAR_INFO_REQ";
		break;
	case WIZ_GAMESTART:
		return PacketHeader = "WIZ_GAMESTART";
		break;
	case WIZ_LOGOUT:
		return PacketHeader = "WIZ_LOGOUT";
		break;
	case WIZ_DEAD:
		return PacketHeader = "WIZ_DEAD";
		break;
	case WIZ_REGENE:
		return PacketHeader = "WIZ_REGENE";
		break;
	case WIZ_TIME:
		return PacketHeader = "WIZ_TIME";
		break;
	case WIZ_WEATHER:
		return PacketHeader = "WIZ_WEATHER";
		break;
	case WIZ_REGIONCHANGE:
		return PacketHeader = "WIZ_REGIONCHANGE";
		break;
	case WIZ_HP_CHANGE:
		return PacketHeader = "WIZ_HP_CHANGE";
		break;
	case WIZ_MSP_CHANGE:
		return PacketHeader = "WIZ_MSP_CHANGE";
		break;
	case WIZ_EXP_CHANGE:
		return PacketHeader = "WIZ_EXP_CHANGE";
		break;
	case WIZ_LEVEL_CHANGE:
		return PacketHeader = "WIZ_LEVEL_CHANGE";
		break;
	case WIZ_NPC_REGION:
		return PacketHeader = "WIZ_NPC_REGION";
		break;
	case WIZ_REQ_NPCIN:
		return PacketHeader = "WIZ_REQ_NPCIN";
		break;
	case WIZ_WARP:
		return PacketHeader = "WIZ_WARP";
		break;
	case WIZ_ITEM_MOVE:
		return PacketHeader = "WIZ_ITEM_MOVE";
		break;
	case WIZ_NPC_EVENT:
		return PacketHeader = "WIZ_NPC_EVENT";
		break;
	case WIZ_ITEM_TRADE:
		return PacketHeader = "WIZ_ITEM_TRADE";
		break;
	case WIZ_TARGET_HP:
		return PacketHeader = "WIZ_TARGET_HP";
		break;
	case WIZ_ITEM_DROP:
		return PacketHeader = "WIZ_ITEM_DROP";
		break;
	case WIZ_BUNDLE_OPEN_REQ:
		return PacketHeader = "WIZ_BUNDLE_OPEN_REQ";
		break;
	case WIZ_TRADE_NPC:
		return PacketHeader = "WIZ_TRADE_NPC";
		break;
	case WIZ_ITEM_GET:
		return PacketHeader = "WIZ_ITEM_GET";
		break;
	case WIZ_ZONE_CHANGE:
		return PacketHeader = "WIZ_ZONE_CHANGE";
		break;
	case WIZ_POINT_CHANGE:
		return PacketHeader = "WIZ_POINT_CHANGE";
		break;
	case WIZ_STATE_CHANGE:
		return PacketHeader = "WIZ_STATE_CHANGE";
		break;
	case WIZ_LOYALTY_CHANGE:
		return PacketHeader = "WIZ_LOYALTY_CHANGE";
		break;
	case WIZ_VERSION_CHECK:
		return PacketHeader = "WIZ_VERSION_CHECK";
		break;
	case WIZ_CRYPTION:
		return PacketHeader = "WIZ_CRYPTION";
		break;
	case WIZ_USERLOOK_CHANGE:
		return PacketHeader = "WIZ_USERLOOK_CHANGE";
		break;
	case WIZ_NOTICE:
		return PacketHeader = "WIZ_NOTICE";
		break;
	case WIZ_PARTY:
		return PacketHeader = "WIZ_PARTY";
		break;
	case WIZ_EXCHANGE:
		return PacketHeader = "WIZ_EXCHANGE";
		break;
	case WIZ_MAGIC_PROCESS:
		return PacketHeader = "WIZ_MAGIC_PROCESS";
		break;
	case WIZ_SKILLPT_CHANGE:
		return PacketHeader = "WIZ_SKILLPT_CHANGE";
		break;
	case WIZ_OBJECT_EVENT:
		return PacketHeader = "WIZ_OBJECT_EVENT";
		break;
	case WIZ_CLASS_CHANGE:
		return PacketHeader = "WIZ_CLASS_CHANGE";
		break;
	case WIZ_CHAT_TARGET:
		return PacketHeader = "WIZ_CHAT_TARGET";
		break;
	case WIZ_CONCURRENTUSER:
		return PacketHeader = "WIZ_CONCURRENTUSER";
		break;
	case WIZ_DATASAVE:
		return PacketHeader = "WIZ_DATASAVE";
		break;
	case WIZ_DURATION:
		return PacketHeader = "WIZ_DURATION";
		break;
	case WIZ_TIMENOTIFY:
		return PacketHeader = "WIZ_TIMENOTIFY";
		break;
	case WIZ_REPAIR_NPC:
		return PacketHeader = "WIZ_REPAIR_NPC";
		break;
	case WIZ_ITEM_REPAIR:
		return PacketHeader = "WIZ_ITEM_REPAIR";
		break;
	case WIZ_ITEM_COUNT_CHANGE:
		return PacketHeader = "WIZ_ITEM_COUNT_CHANGE";
		break;
	case WIZ_KNIGHTS_LIST:
		return PacketHeader = "WIZ_KNIGHTS_LIST";
		break;
	case WIZ_ITEM_REMOVE:
		return PacketHeader = "WIZ_ITEM_REMOVE";
		break;
	case WIZ_OPERATOR:
		return PacketHeader = "WIZ_OPERATOR";
		break;
	case WIZ_SPEEDHACK_CHECK:
		return PacketHeader = "WIZ_SPEEDHACK_CHECK";
		break;
	case WIZ_SERVER_CHECK:
		return PacketHeader = "WIZ_SERVER_CHECK";
		break;
	case WIZ_CONTINOUS_PACKET:
		return PacketHeader = "WIZ_CONTINOUS_PACKET";
		break;
	case WIZ_WAREHOUSE:
		return PacketHeader = "WIZ_WAREHOUSE";
		break;
	case WIZ_SERVER_CHANGE:
		return PacketHeader = "WIZ_SERVER_CHANGE";
		break;
	case WIZ_REPORT_BUG:
		return PacketHeader = "WIZ_REPORT_BUG";
		break;
	case WIZ_HOME:
		return PacketHeader = "WIZ_HOME";
		break;
	case WIZ_FRIEND_PROCESS:
		return PacketHeader = "WIZ_FRIEND_PROCESS";
		break;
	case WIZ_GOLD_CHANGE:
		return PacketHeader = "WIZ_GOLD_CHANGE";
		break;
	case WIZ_WARP_LIST:
		return PacketHeader = "WIZ_WARP_LIST";
		break;
	case WIZ_VIRTUAL_SERVER:
		return PacketHeader = "WIZ_VIRTUAL_SERVER";
		break;
	case WIZ_ZONE_CONCURRENT:
		return PacketHeader = "WIZ_ZONE_CONCURRENT";
		break;
	case WIZ_CORPSE:
		return PacketHeader = "WIZ_CORPSE";
		break;
	case WIZ_PARTY_BBS:
		return PacketHeader = "WIZ_PARTY_BBS";
		break;
	case WIZ_MARKET_BBS:
		return PacketHeader = "WIZ_MARKET_BBS";
		break;
	case WIZ_KICKOUT:
		return PacketHeader = "WIZ_KICKOUT";
		break;
	case WIZ_CLIENT_EVENT:
		return PacketHeader = "WIZ_CLIENT_EVENT";
		break;
	case WIZ_MAP_EVENT:
		return PacketHeader = "WIZ_MAP_EVENT";
		break;
	case WIZ_WEIGHT_CHANGE:
		return PacketHeader = "WIZ_WEIGHT_CHANGE";
		break;
	case WIZ_SELECT_MSG:
		return PacketHeader = "WIZ_SELECT_MSG";
		break;
	case WIZ_NPC_SAY:
		return PacketHeader = "WIZ_NPC_SAY";
		break;
	case WIZ_BATTLE_EVENT:
		return PacketHeader = "WIZ_BATTLE_EVENT";
		break;
	case WIZ_AUTHORITY_CHANGE:
		return PacketHeader = "WIZ_AUTHORITY_CHANGE";
		break;
	case WIZ_EDIT_BOX:
		return PacketHeader = "WIZ_EDIT_BOX";
		break;
	case WIZ_SANTA:
		return PacketHeader = "WIZ_SANTA";
		break;
	case WIZ_ITEM_UPGRADE:
		return PacketHeader = "WIZ_ITEM_UPGRADE";
		break;
	case WIZ_PACKET2:
		return PacketHeader = "WIZ_PACKET2";
		break;
	case WIZ_ZONEABILITY:
		return PacketHeader = "WIZ_ZONEABILITY";
		break;
	case WIZ_EVENT:
		return PacketHeader = "WIZ_EVENT";
		break;
	case WIZ_STEALTH:
		return PacketHeader = "WIZ_STEALTH";
		break;
	case WIZ_ROOM_PACKETPROCESS:
		return PacketHeader = "WIZ_ROOM_PACKETPROCESS";
		break;
	case WIZ_ROOM:
		return PacketHeader = "WIZ_ROOM";
		break;
	case WIZ_CLAN_BATTLE:
		return PacketHeader = "WIZ_CLAN_BATTLE";
		break;
	case WIZ_QUEST:
		return PacketHeader = "WIZ_QUEST";
		break;
	case WIZ_PP_CARD_LOGIN:
		return PacketHeader = "WIZ_PP_CARD_LOGIN";
		break;
	case WIZ_KISS:
		return PacketHeader = "WIZ_KISS";
		break;
	case WIZ_RECOMMEND_USER:
		return PacketHeader = "WIZ_RECOMMEND_USER";
		break;
	case WIZ_MERCHANT_INOUT:
		return PacketHeader = "WIZ_MERCHANT_INOUT";
		break;
	case WIZ_SHOPPING_MALL:
		return PacketHeader = "WIZ_SHOPPING_MALL";
		break;
	case WIZ_SERVER_INDEX:
		return PacketHeader = "WIZ_SERVER_INDEX";
		break;
	case WIZ_EFFECT:
		return PacketHeader = "WIZ_EFFECT";
		break;
	case WIZ_SIEGE:
		return PacketHeader = "WIZ_SIEGE";
		break;
	case WIZ_NAME_CHANGE:
		return PacketHeader = "WIZ_NAME_CHANGE";
		break;
	case WIZ_WEBPAGE:
		return PacketHeader = "WIZ_WEBPAGE";
		break;
	case WIZ_CAPE:
		return PacketHeader = "WIZ_CAPE";
		break;
	case WIZ_PREMIUM:
		return PacketHeader = "WIZ_PREMIUM";
		break;
	case WIZ_HACKTOOL:
		return PacketHeader = "WIZ_HACKTOOL";
		break;
	case WIZ_RENTAL:
		return PacketHeader = "WIZ_RENTAL";
		break;
	case WIZ_ITEM_EXPIRATION:
		return PacketHeader = "WIZ_ITEM_EXPIRATION";
		break;
	case WIZ_CHALLENGE:
		return PacketHeader = "WIZ_CHALLENGE";
		break;
	case WIZ_PET:
		return PacketHeader = "WIZ_PET";
		break;
	case WIZ_CHINA:
		return PacketHeader = "WIZ_CHINA";
		break;
	case WIZ_KING:
		return PacketHeader = "WIZ_KING";
		break;
	case WIZ_SKILLDATA:
		return PacketHeader = "WIZ_SKILLDATA";
		break;
	case WIZ_PROGRAMCHECK:
		return PacketHeader = "WIZ_PROGRAMCHECK";
		break;
	case WIZ_BIFROST:
		return PacketHeader = "WIZ_BIFROST";
		break;
	case WIZ_REPORT:
		return PacketHeader = "WIZ_REPORT";
		break;
	case WIZ_LOGOSSHOUT:
		return PacketHeader = "WIZ_REPORT";
		break;
	case WIZ_PACKET6:
		return PacketHeader = "WIZ_PACKET6";
		break;
	case WIZ_PACKET7:
		return PacketHeader = "WIZ_PACKET7";
		break;
	case WIZ_RANK:
		return PacketHeader = "WIZ_RANK";
		break;
	case WIZ_STORY:
		return PacketHeader = "WIZ_STORY";
		break;
	case WIZ_NATION_TRANSFER:
		return PacketHeader = "WIZ_NATION_TRANSFER";
		break;
	case WIZ_TERRAIN_EFFECTS:
		return PacketHeader = "WIZ_TERRAIN_EFFECTS";
		break;
	case WIZ_MOVING_TOWER:
		return PacketHeader = "WIZ_MOVING_TOWER";
		break;
	case WIZ_CAPTURE:
		return PacketHeader = "WIZ_CAPTURE";
		break;
	case WIZ_MINING:
		return PacketHeader = "WIZ_MINING";
		break;
	case WIZ_HELMET:
		return PacketHeader = "WIZ_HELMET";
		break;
	case WIZ_PVP:
		return PacketHeader = "WIZ_PVP";
		break;
	case WIZ_CHANGE_HAIR:
		return PacketHeader = "WIZ_CHANGE_HAIR";
		break;
	case WIZ_KAUL_A:
		return PacketHeader = "WIZ_KAUL_A";
		break;
	case WIZ_VIPWAREHOUSE:
		return PacketHeader = "WIZ_VIPWAREHOUSE";
		break;
	case WIZ_KAUL_C:
		return PacketHeader = "WIZ_KAUL_C";
		break;
	case WIZ_GENDER_CHANGE:
		return PacketHeader = "WIZ_GENDER_CHANGE";
		break;
	case WIZ_PACKET16:
		return PacketHeader = "WIZ_PACKET16";
		break;
	case WIZ_PACKET17:
		return PacketHeader = "WIZ_PACKET17";
		break;
	case WIZ_LOYALTY_SHOP:
		return PacketHeader = "WIZ_LOYALTY_SHOP";
		break;
	case WIZ_CLANPOINTS_BATTLE:
		return PacketHeader = "WIZ_CLANPOINTS_BATTLE";
		break;
	case WIZ_UTC_MOVIE:
		return PacketHeader = "WIZ_UTC_MOVIE";
		break;
	case WIZ_GENIE:
		return PacketHeader = "WIZ_GENIE";
		break;
	case WIZ_USER_INFORMATIN:
		return PacketHeader = "WIZ_USER_INFORMATIN";
		break;
	case WIZ_USER_ACHIEVE:
		return PacketHeader = "WIZ_USER_ACHIEVE";
		break;
	case WIZ_EXP_SEAL:
		return PacketHeader = "WIZ_EXP_SEAL";
		break;
	case WIZ_KURIAN_SP_CHANGE:
		return PacketHeader = "WIZ_KURIAN_SP_CHANGE";
		break;
	case WIZ_LOADING_LOGIN:
		return PacketHeader = "WIZ_LOADING_LOGIN";
		break;
	case WIZ_NATION_CHAT:
		return PacketHeader = "WIZ_NATION_CHAT";
		break;
	case WIZ_TEST_PACKET:
		return PacketHeader = "WIZ_TEST_PACKET";
		break;
	case WIZ_VANGUARD:
		return PacketHeader = "WIZ_VANGUARD";
		break;
	case WIZ_KNIGHT_ROYALE:
		return PacketHeader = "WIZ_KNIGHT_ROYALE";
		break;
	case WIZ_MERCHANT:
		return PacketHeader = "WIZ_MERCHANT";
		break;
	case WIZ_KNIGHTS_PROCESS:
		return PacketHeader = "WIZ_KNIGHTS_PROCESS";
		break;
	default:
		return PacketHeader = "UNKNOWN";
		break;
	}

	return "";
}
#pragma region CUser::HandlePacket(Packet & pkt)
bool CUser::HandlePacket(Packet& pkt)
{
	//TRACE("[CLIENT - GAMESERVER] - [SID=%d] Packet: [%02X] (len=%d)\n", GetSocketID(), pkt.GetOpcode(), pkt.size());
	uint8 command = pkt.GetOpcode();

	/*if (command == WIZ_MERCHANT) {
		std::string Header = PacketSend(command);
		printf("Send:%s 0x%02X ", Header.c_str(), command);
		for (int i = 0; i < pkt.size(); i++)
			printf("%02X", (int)pkt.contents()[i]);
		printf(" - Name:%s\n", GetName().c_str());
	}*/

	/*std::string Header = PacketSend(command);
	printf("Send:%s 0x%02X ", Header.c_str(), command);
	for (int i = 0; i < pkt.size(); i++)
		printf("%02X", (int)pkt.contents()[i]);
	printf(" - Name:%s\n", GetName().c_str());*/


	//TRACE("[SID=%d] Packet: %X (len=%d)\n", GetSocketID(), command, pkt.size());
	// If we're not authed yet, forced us to before we can do anything else.
	// NOTE: We're checking the account ID store here because it's only set on successful auth,
	// at which time the other account ID will be cleared out (yes, it's messy -- need to clean it up).
	if (m_strAccountID.empty())
	{
		if (command == XSafe)
		{
			XSafe_HandlePacket(pkt);
			return true;
		}

		if (command == WIZ_VERSION_CHECK)
		{
			VersionCheck(pkt);
			return true;
		}
		if (command == WIZ_LOGIN)
		{
			LoginProcess(pkt);
			return true;
		}

		if (command == WIZ_KICKOUT)
		{
			KickOutProcess(pkt);
			return true;
		}

		return false;
	}
	else if (!m_bSelectedCharacter)
	{
		switch (command)
		{/* XSafe Packet Handle */
		case WIZ_CAPTCHA: //xx
			CaptchaRequest(pkt);
			break;
		case XSafe:
			XSafe_HandlePacket(pkt);
			break;
			/* </XSafe Packet Handle> */
		case WIZ_SEL_NATION:
			SelNationToAgent(pkt);
			break;
		case WIZ_LOADING_LOGIN:
			LoadingLogin(pkt);
			break;
		case WIZ_ALLCHAR_INFO_REQ:
			AllCharInfo(pkt);
			break;
		case WIZ_CHANGE_HAIR:
			ChangeHair(pkt);
			break;
		case WIZ_NEW_CHAR:
			NewCharToAgent(pkt);
			break;
		case WIZ_DEL_CHAR:
			//DelCharToAgent(pkt);
			break;
		case WIZ_SEL_CHAR:
			SelCharToAgent(pkt);
			break;
		case WIZ_SPEEDHACK_CHECK:
			SpeedHackTime(pkt);
			break;
		case WIZ_NAME_CHANGE:
			HandleNameChange(pkt);
			break;
		case WIZ_HACKTOOL:
			break;
		case WIZ_PRESET:		//2370 Stat Skill Preset Paketi
			StatPresetHandle(pkt);
			break;
		case 0x47:
			break;
		default:
			printf("[SID=%d] Unhandled packet (%X) prior to selecting character AccountName: %s\n", GetSocketID(), command, GetAccountName().c_str());
			return false;
		}
		return true;
	}
	else if (!isInGame())
	{
		if (command == WIZ_SEL_CHAR)
			return true;

		switch (command)
		{
			/* XSafe Packet Handle */
		case XSafe:
			XSafe_HandlePacket(pkt);
			break;
			/* </XSafe Packet Handle> */
		case WIZ_GAMESTART:
			GameStart(pkt);
			break;
		case WIZ_ALLCHAR_INFO_REQ:
			AllCharInfo(pkt);
			break;
		case WIZ_SERVER_INDEX:
			SendServerIndex();
			break;
		case WIZ_RENTAL:
			RentalSystem(pkt);
			break;
		case WIZ_SKILLDATA:
			SkillDataProcess(pkt);
			break;
		case WIZ_REGENE:
			Regene(pkt.read<uint8>()); // respawn type
			break;
		case WIZ_REQ_USERIN:
			RequestUserIn(pkt);
			break;
		case WIZ_REQ_NPCIN:
			RequestNpcIn(pkt);
			break;
		case WIZ_ZONE_CHANGE:
			RecvZoneChange(pkt);
			break;
		case WIZ_STATE_CHANGE:
			StateChange(pkt);
			break;
		case WIZ_QUEST:
			QuestV2PacketProcess(pkt);
			break;
		case WIZ_MAP_EVENT:
			RecvMapEvent(pkt);
			break;
		case WIZ_MAGIC_PROCESS:
			CMagicProcess::MagicPacket(pkt, this);
			break;
		case WIZ_OBJECT_EVENT:
			ObjectEvent(pkt);
			break;
		case WIZ_WEATHER:
		case WIZ_TIME:
			UpdateGameWeather(pkt);
			break;
		case WIZ_CONCURRENTUSER:
			CountConcurrentUser();
			break;
		case WIZ_DATASAVE:
			UserDataSaveToAgent();
			break;
		case WIZ_KNIGHTS_PROCESS:
			CKnightsManager::PacketProcess(this, pkt);
			break;
		case WIZ_SPEEDHACK_CHECK:
			SpeedHackTime(pkt);
			break;
		case WIZ_FRIEND_PROCESS:
			FriendProcess(pkt);
			break;
		case WIZ_WARP_LIST:
			SelectWarpList(pkt);
			break;
		case WIZ_VIRTUAL_SERVER:
			ServerChangeOk(pkt);
			break;
		case WIZ_CLIENT_EVENT:
			ClientEvent(pkt.read<uint16>());
			break;
		case WIZ_SELECT_MSG:
			RecvSelectMsg(pkt);
			break;
		case WIZ_EVENT:
			if (GetZoneID() == ZONE_PRISON)
				return false;

			{
				Packet copy(pkt);
				uint8 tmpOpcode = copy.read<uint8>();
				TRACE("[DEBUG] WIZ_EVENT received opcode=%u from SID=%d\n", tmpOpcode, GetSocketID());
				if (tmpOpcode == MONSTER_STONE) {
					uint32 tmpItem = 0;
					// try to read item id if present
					tmpItem = copy.read<uint32>();
					TRACE("[DEBUG] WIZ_EVENT MONSTER_STONE itemid=%u from SID=%d\n", tmpItem, GetSocketID());
				}
			}

			TempleProcess(pkt);
			break;
		case WIZ_SHOPPING_MALL: // letter system's used in here too
			ShoppingMall(pkt);
			break;
		case WIZ_HELMET:
			HandleHelmet(pkt);
			break;
		case WIZ_USER_INFORMATIN:
			BottomLeftRegionUserList(pkt);
			break;
		case WIZ_GENIE:
			HandleGenie(pkt);
			break;
		case WIZ_LOGOSSHOUT:
			LogosShout(pkt);
			break;
		case WIZ_CHANGE_HAIR:
			ChangeHair(pkt);
			break;
		case WIZ_USER_ACHIEVE:
			HandleUserAchieve(pkt);
			break;
		case WIZ_EXP_SEAL:
			ExpSealHandler(pkt);
			break;
		case WIZ_PREMIUM:
			HandlePremium(pkt);
			break;
		case WIZ_HACKTOOL:
			break;
		case 0xBA:
			break;
		case WIZ_PROGRAMCHECK: // /plc komutunu calıstırır ve kullandıgı programları gösterir sol altta notice olarak 27.09.2020
			PlayerProgramCheck(pkt);
			break;
		case 0xB3:
			break;
		case WIZ_PRESET:	//2370 Skill stat preset paketi
			StatPresetHandle(pkt);
			break;
		case 0x47:
			break;
		default:
			printf("!isInGame [SID=%d] Unknown packet %X AccountName: %s\n", GetSocketID(), command, GetAccountName().c_str());
			return false;
			break;
		}
	}
	else if (isInGame())
	{
		if (command == WIZ_SEL_CHAR)
			return true;

		bool isCindIn = pCindWar.isEventUser() && g_pMain->isCindirellaZone(GetZoneID());

		switch (command)
		{
		case WIZ_AKARA:
			OpenAkara(pkt);
			break;
		case XSafe:
			XSafe_HandlePacket(pkt);
			break;
		case WIZ_PROGRAMCHECK: // /plc komutunu calıstırır ve kullandıgı programları gösterir sol altta notice olarak 27.09.2020 
			PlayerProgramCheck(pkt);//tam burada 0x7A vardı değiştirildi
			break;
		case WIZ_ALLCHAR_INFO_REQ:
			AllCharInfo(pkt);
			break;
		case WIZ_GAMESTART:
			GameStart(pkt);
			break;
		case WIZ_SERVER_INDEX:
			SendServerIndex();
			break;
		case WIZ_RENTAL:
			if (!isCindIn) RentalSystem(pkt);
			break;
		case WIZ_SKILLDATA:
			SkillDataProcess(pkt);
			break;
		case WIZ_MOVE:
			MoveProcess(pkt);
			break;
		case WIZ_ROTATE:
			Rotate(pkt);
			break;
		case WIZ_ATTACK:
			Attack(pkt);
			break;
		case WIZ_CHAT:
			Chat(pkt);
			break;
		case WIZ_CHAT_TARGET:
			ChatTargetSelect(pkt);
			break;
		case WIZ_REGENE:
			Regene(pkt.read<uint8>()); // respawn type
			break;
		case WIZ_REQ_USERIN:
			RequestUserIn(pkt);
			break;
		case WIZ_REQ_NPCIN:
			RequestNpcIn(pkt);
			break;
		case WIZ_WARP:
			RecvWarp(pkt);
			break;
		case WIZ_ITEM_MOVE:
			ItemMove(pkt);
			break;
		case WIZ_NATION_CHAT:
			ChatRoomHandle(pkt);
			break;
		case WIZ_NPC_EVENT:
			NpcEvent(pkt);
			break;
		case WIZ_ITEM_TRADE:
			/*if (!isCindIn)*/ ItemTrade(pkt); //11.04.2024
			break;
		case WIZ_TARGET_HP:
			HandleTargetHP(pkt);
			break;
		case WIZ_BUNDLE_OPEN_REQ:
			/*if (!isCindIn)*/ BundleOpenReq(pkt); //11.04.2024
			break;
		case WIZ_ITEM_GET:
			/*if (!isCindIn)*/ ItemGet(pkt); //11.04.2024
			break;
		case WIZ_ZONE_CHANGE:
			RecvZoneChange(pkt);
			break;
		case WIZ_POINT_CHANGE:
			PointChange(pkt);
			break;
		case WIZ_STATE_CHANGE:
			StateChange(pkt);
			break;
		case WIZ_PARTY:
			PartySystemProcess(pkt);
			break;
		case WIZ_EXCHANGE:
			if (!isCindIn) ExchangeProcess(pkt);
			break;
		case WIZ_QUEST:
			QuestV2PacketProcess(pkt);
			break;
		case WIZ_MAP_EVENT:
			RecvMapEvent(pkt);
			break;
		case WIZ_MERCHANT:
			if (!isCindIn) MerchantProcess(pkt);
			break;
		case WIZ_MAGIC_PROCESS:
			CMagicProcess::MagicPacket(pkt, this);
			break;
		case WIZ_SKILLPT_CHANGE:
			SkillPointChange(pkt);
			break;
		case WIZ_OBJECT_EVENT:
			ObjectEvent(pkt);
			break;
		case WIZ_WEATHER:
		case WIZ_TIME:
			UpdateGameWeather(pkt);
			break;
		case WIZ_CLASS_CHANGE:
			ClassChange(pkt);
			break;
		case WIZ_CONCURRENTUSER:
			CountConcurrentUser();
			break;
		case WIZ_DATASAVE:
			if (!isCindIn) UserDataSaveToAgent();
			break;
		case WIZ_ITEM_REPAIR:
			ItemRepair(pkt);
			break;
		case WIZ_KNIGHTS_PROCESS:
			CKnightsManager::PacketProcess(this, pkt);
			break;
		case WIZ_ITEM_REMOVE:
			ItemRemove(pkt);
			break;
		case WIZ_OPERATOR:
			OperatorCommand(pkt);
			break;
		case WIZ_SPEEDHACK_CHECK:
			SpeedHackTime(pkt);
			break;
		case WIZ_WAREHOUSE:
			if (!isCindIn) WarehouseProcess(pkt);
			break;
		case WIZ_CLANWAREHOUSE:
			if (!isCindIn) ClanWareHouseProcess(pkt);
			break;
		case WIZ_HOME:
			Home();
			break;
		case WIZ_FRIEND_PROCESS:
			FriendProcess(pkt);
			break;
		case WIZ_WARP_LIST:
			SelectWarpList(pkt);
			break;
		case WIZ_VIRTUAL_SERVER:
			ServerChangeOk(pkt);
			break;
		case WIZ_PARTY_BBS:
			PartyBBS(pkt);
			break;
		case WIZ_CLIENT_EVENT:
			ClientEvent(pkt.read<uint16>());
			break;
		case WIZ_SELECT_MSG:
			RecvSelectMsg(pkt);
			break;
		case WIZ_ITEM_UPGRADE:
			ExchangeSystemProcess(pkt);
			if (!isCindIn) break;
		case WIZ_EVENT:
			TempleProcess(pkt);
			break;
		case WIZ_SHOPPING_MALL: // letter system's used in here too
			if (!isCindIn) ShoppingMall(pkt);
			break;
		case WIZ_NAME_CHANGE:
			HandleNameChange(pkt);
			break;
		case WIZ_KING:
			CKingSystem::PacketProcess(this, pkt);
			break;
		case WIZ_HELMET:
			HandleHelmet(pkt);
			break;
		case WIZ_CAPE:
			HandleCapeChange(pkt);
			break;
		case WIZ_CHALLENGE:
			HandleChallenge(pkt);
			break;
		case WIZ_RANK:
			HandlePlayerRankings(pkt);
			break;
		case WIZ_MINING:
			if (!isCindIn) HandleMiningSystem(pkt);
			break;
		case WIZ_USER_INFORMATIN:
			BottomLeftRegionUserList(pkt);
			break;
		case WIZ_GENIE:
			HandleGenie(pkt);
			break;
		case WIZ_LOGOSSHOUT:
			LogosShout(pkt);
			break;
		case WIZ_SIEGE:
			if (!isCindIn) SiegeWarFareProcess(pkt);
			break;
		case WIZ_NATION_TRANSFER:
			if (!isCindIn) HandleNationChange(pkt);
			break;
		case WIZ_GENDER_CHANGE:
			if (!isCindIn) GenderChangeV2(pkt);
			break;
		case WIZ_CHANGE_HAIR:
			if (!isCindIn) ChangeHair(pkt);
			break;
		case WIZ_VIPWAREHOUSE:
			if (!isCindIn) VIPhouseProcess(pkt);
			break;
		case WIZ_REPORT:
			SheriffVote(pkt);
			break;
		case WIZ_MOVING_TOWER:
			if (!isCindIn) HandleTowerPackets(pkt);
			break;
#if 0
		case WIZ_CAPTURE:
			HandleBDWCapture(pkt);
			break;
#endif
		case WIZ_USER_ACHIEVE:
			if (!isCindIn) HandleUserAchieve(pkt);
			break;
		case WIZ_EXP_SEAL:
			ExpSealHandler(pkt);
			break;
		case WIZ_PREMIUM:
			HandlePremium(pkt);
			break;
			/*case WIZ_KNIGHT_ROYALE:
				break;*/
		case WIZ_PET:
			if (!isCindIn) MainPetProcess(pkt);
			break;
		case WIZ_VANGUARD:
			WantedProcess(pkt);
			break;
		case WIZ_EDIT_BOX:
			if (!isCindIn) PPCard(pkt);
			break;
		case WIZ_HACKTOOL:
			break;
		case 0xB3:
		case 0xBA:
			break;
		case WIZ_DAILYRANK:	//2370 Dailyrank paketi
			HandleDailyRank(pkt);
			break;
		case WIZ_PRESET:	//2370 stat skill preset 
			StatPresetHandle(pkt);
			break;
		case WIZ_REPORT_BUG:
			break;
		default:
			printf("isInGame [SID=%s] Unknown packet %X AccountName:%s\n", GetName().c_str(), command, GetAccountName().c_str());
			return false;
		}
	}
	return true;
}
#pragma endregion

void CUser::UpdateCheckSkillTime()
{
	if (!isBlinking())
	{
		if (m_tHPLastTimeNormal != 0 && (UNIXTIME - m_tHPLastTimeNormal) > m_bHPIntervalNormal)
			HPTimeChange();	// For Sitdown/Standup HP restoration.
	}

	if (isInGame() && isPortuKurian())
	{
		if (m_tSpLastTimeNormal != 0 && (UNIXTIME - m_tSpLastTimeNormal) > m_bSpIntervalNormal)
			SpTimeChange();
	}

	// Handles DOT/HOT skills (not COLD skills though.)
	if (m_bType3Flag)
		HPTimeChangeType3();

	if (UNIXTIME2 > m_tLastType4SkillCheck)
	{
		m_tLastType4SkillCheck = UNIXTIME2 + 100;
		Type4Duration();
		CheckSavedMagic();
	}

	if (UNIXTIME2 > m_tLastType69SkillCheck)
	{
		m_tLastType69SkillCheck = UNIXTIME2 + 700;
		if (isTransformed())
			Type6Duration();

		Type9Duration();
	}

	if (isBlinking())		// Should you stop blinking?
		BlinkTimeCheck();

	if (!isBlinking() && isTransformed() && m_bCanUseSkills == false)
		m_bCanUseSkills = true;

	if (hasRival() && hasRivalryExpired())
		RemoveRival();
}

void CUser::UpdateCheckTime()
{

#if(GAME_SOURCE_VERSION != 1098)
	if (m_bResHpType == USER_SITDOWN)
		TrainingProcess();
#endif

	if (GetGenieTime() > 0 && m_tGenieTimeNormal + PLAYER_GENIE_INTERVAL < UNIXTIME)
	{
		m_tGenieTimeNormal = UNIXTIME;
		CheckGenieTime();
	}

	if ((UNIXTIME - m_lastSaveTime) >= PLAYER_SAVE_INTERVAL)
	{
		m_lastSaveTime = UNIXTIME; // this is set by UpdateUser(), however it may result in multiple requests unless it's set first.
		UserDataSaveToAgent();
	}

	if (m_flashtime > 0 && m_flashchecktime + PLAYER_FLASH_INTERVAL < UNIXTIME)
	{
		m_flashchecktime = UNIXTIME;
		FlashUpdateTime(--m_flashtime);
	}
}

void CUser::UpdateCheckPremiumTime()
{
	bool anyExpiration = false;
	std::set<uint8> deleted;
	foreach_stlmap(itr, m_PremiumMap)
	{
		_PREMIUM_DATA* pPremium = itr->second;
		if (pPremium == nullptr || pPremium->iPremiumTime == 0 || pPremium->iPremiumTime > (uint32)UNIXTIME)
			continue;

		anyExpiration = true;
		pPremium->iPremiumTime = 0;
		deleted.insert(pPremium->bPremiumType);
		if (pPremium->bPremiumType == m_bPremiumInUse)
			m_bPremiumInUse = NO_PREMIUM;
	}

	if (anyExpiration)
	{
		foreach(itr, deleted) m_PremiumMap.DeleteData(*itr);
		SendPremiumInfo();
	}
}

void CUser::UpdateCheckItemTime()
{
	for (int i = 0; i < WAREHOUSE_MAX; i++)
	{
		_ITEM_DATA* pItem = &m_sWarehouseArray[i];

		if (pItem == nullptr)
			continue;

		if (pItem->nExpirationTime < (uint32)UNIXTIME && pItem->nExpirationTime != 0)
			memset(pItem, 0, sizeof(_ITEM_DATA));
	}

	for (int i = 0; i < VIPWAREHOUSE_MAX; i++)
	{
		_ITEM_DATA* pItem = &m_sVIPWarehouseArray[i];

		if (pItem == nullptr)
			continue;

		if (pItem->nExpirationTime < (uint32)UNIXTIME && pItem->nExpirationTime != 0)
			memset(pItem, 0, sizeof(_ITEM_DATA));
	}

	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = GetItem(i);
		if (pItem == nullptr)
			continue;

		if (pItem->nExpirationTime != 0 && pItem->nExpirationTime < (uint32)UNIXTIME)
		{
			if (i < SLOT_MAX)
				SendStackChange(0, 0, 0, i, true, 0, ITEM_SECTION_SLOT);
			else if (INVENTORY_INVENT <= i && i < INVENTORY_COSP)
				SendStackChange(0, 0, 0, i - INVENTORY_INVENT, true, 0, ITEM_SECTION_INVEN);
			else if (INVENTORY_COSP <= i && i < INVENTORY_MBAG)
			{
				if (i == CWING)
					SendStackChange(0, 0, 0, COSP_WINGS, true, 0, ITEM_SECTION_COSPRE);
				else if (i == CHELMET)
					SendStackChange(0, 0, 0, COSP_HELMET, true, 0, ITEM_SECTION_COSPRE);
				else if (i == CLEFT)
					SendStackChange(0, 0, 0, COSP_GLOVE, true, 0, ITEM_SECTION_COSPRE);
				else if (i == CRIGHT)
					SendStackChange(0, 0, 0, COSP_GLOVE2, true, 0, ITEM_SECTION_COSPRE);
				else if (i == CTOP)
					SendStackChange(0, 0, 0, COSP_BREAST, true, 0, ITEM_SECTION_COSPRE);
				else if (i == CFAIRY)
					SendStackChange(0, 0, 0, COSP_FAIRY, true, 0, ITEM_SECTION_COSPRE);
				else if (i == BAG1)
					SendStackChange(0, 0, 0, COSP_BAG1, true, 0, ITEM_SECTION_COSPRE);
				else if (i == BAG2)
					SendStackChange(0, 0, 0, COSP_BAG2, true, 0, ITEM_SECTION_COSPRE);
				else if (i == CTATTOO)
					SendStackChange(0, 0, 0, COSP_TATTO, true, 0, ITEM_SECTION_COSPRE);
			}
			else
				SendStackChange(0, 0, 0, i - INVENTORY_MBAG, true, 0, ITEM_SECTION_MBAG);

			if (i == SHOULDER
				&& (pItem->nNum == ROBIN_LOOT_ITEM || pItem->nNum == 850680000 || pItem->nNum == 510000000 || pItem->nNum == 520000000))
				m_autoloot = false;

			if (i == CFAIRY && pItem->nNum == ITEM_OREADS)
				m_fairycheck = false;

			memset(pItem, 0, sizeof(_ITEM_DATA));
		}
	}
}

void CUser::CheckDelayedTime()
{
	if (!isInGame())
		return;

	UpdateCheckPremiumTime();
	UpdateCheckItemTime();
}

void CUser::BurningTime()
{
	if (m_flametime == 0 || m_flameilevel >= 3 || UNIXTIME < m_flametime)
		return;

	m_flameilevel++;

	/*Packet test(WIZ_EXP_CHANGE, uint8(3));
	test << m_flameilevel;
	Send(&test);*/

	m_flametime = UNIXTIME + (1 * HOUR);
}

#pragma region CUser::Update()
void CUser::Update()
{
	if (IsOffCharacter()
		&& GetOffMerchantTime()
		&& UNIXTIME > off_merchantcheck)
	{
		if (--m_offmerchanttime == 0)
			return SetOffCharacter(_choffstatus::DEACTIVE);
		off_merchantcheck = UNIXTIME + 60;
	}

	UpdateCheckTime();
	UpdateCheckSkillTime();

	// JstKO: Her oyuncu icin sureli item / premium kontrolu ayri ayri calissin.
	// Eski global DelayedTime sadece tek oyuncuyu kontrol ettigi icin online sureli itemler silinmeyebiliyordu.
	if (UNIXTIME2 > m_tLastDelayedCheck)
	{
		m_tLastDelayedCheck = UNIXTIME2 + (30 * SECOND);
		CheckDelayedTime();
	}
	BurningTime();
	if (isInPKZone() && isWantedUser())
		WantedEventUserisMove();

	/*if (m_petrespawntime > 0 && UNIXTIME > m_petrespawntime)
	{
		m_petrespawntime = 0;
		PetSpawnProcess(true);
	}*/

	if (g_pMain->pServerSetting.onlinecash && g_pMain->pServerSetting.onlinecashtime && UNIXTIME > m_onlinecashtime)
	{
		if (GetZoneID() == ZONE_MORADON)
			GiveBalance(1);
		else if (GetZoneID() == ZONE_RONARK_LAND)
			GiveBalance(2);

		m_onlinecashtime = UNIXTIME + (g_pMain->pServerSetting.onlinecashtime * MINUTE);
	}

	if (m_lastPetSatisfaction + (PLAYER_TRAINING_INTERVAL * 4) < UNIXTIME)
	{
		PetSatisFactionUpdate(-100);
		m_lastPetSatisfaction = UNIXTIME;
	}

	if (m_bVIPStorageVaultExpiration && (uint32)UNIXTIME >= m_bVIPStorageVaultExpiration)
		m_bVIPStorageVaultExpiration = 0;

	// Start the 10 Level Skill
	if (GetLevel() == 10 && CheckExistEvent(71, 0))
		SaveEvent(71, 1);

	if (ReturnSymbolisOK > 0 && ReturnSymbolTime < int64(UNIXTIME))
	{
		ReturnSymbolTime = 0;
		ReturnSymbolisOK = 0;
	}

	if (GetZoneID() == ZONE_DRAKI_TOWER)
		DrakiTowerKickTimer();

	/*if (!IsOffCharacter() && XSafe_LASTCHECK > 0 && (UNIXTIME - XSafe_LASTCHECK) > 60)
	{
		printf("Alive Time Out ---> Disconnect %s <----- \n", GetName().c_str());
		return goDisconnect("XSafe_LASTCHECK alive timeout", __FUNCTION__);
	}*/

	if (UNIXTIME > m_AutoMiningFishingSystem && m_offchstatus == _choffstatus::DEACTIVE) {
		{

			m_AutoMiningFishingSystem = UNIXTIME + 5;

			std::vector<uint32> pMiningList, pFishingList;

			if (GetItem(SHOULDER)->nNum <= 0)
				XSafe_ItemNotice(2, 0);

			if (GetItem(SHOULDER)->nNum == AUTOMATIC_MINING || GetItem(SHOULDER)->nNum == MINING_ROBIN_ITEM)
			{
				g_pMain->m_MiningFishingItemArray.m_lock.lock();
				foreach_stlmap_nolock(itr, g_pMain->m_MiningFishingItemArray)
				{
					auto* pMining = itr->second;
					if (pMining == nullptr || pMining->nTableType != 0)
						continue;

					if (pMining->nWarStatus != 0 || pMining->UseItemType != 4)
						continue;

					if (GetPremium() == 0)
						continue;
					pMiningList.push_back(pMining->nIndex);
				}
				g_pMain->m_MiningFishingItemArray.m_lock.unlock();
			}

			if (GetItem(SHOULDER)->nNum == AUTOMATIC_FISHING || GetItem(SHOULDER)->nNum == FISHING_ROBIN_ITEM)
			{
				g_pMain->m_MiningFishingItemArray.m_lock.lock();
				foreach_stlmap_nolock(itr, g_pMain->m_MiningFishingItemArray)
				{
					auto* pFishing = itr->second;
					if (pFishing == nullptr || pFishing->nTableType != 1)
						continue;

					if (pFishing->nWarStatus != 0 || pFishing->UseItemType == 5)
						continue;

					if (GetPremium() == 0)
						continue;

					pFishingList.push_back(pFishing->nIndex);
				}
				g_pMain->m_MiningFishingItemArray.m_lock.unlock();
			}

			if (pMiningList.size())
			{
				uint32 bRandArray[10000], offset = 0; uint8 sItemSlot = 0;
				memset(bRandArray, 0, sizeof(bRandArray));
				foreach(itr, pMiningList)
				{
					auto* pFishing = g_pMain->m_MiningFishingItemArray.GetData(*itr);
					if (pFishing == nullptr)
						continue;

					if (offset >= 9999)
						break;

					for (int i = 0; i < int(pFishing->SuccessRate); i++)
					{
						if (i + offset >= 9999)
							break;

						bRandArray[offset + i] = pFishing->nGiveItemID;
					}
					offset += int(pFishing->SuccessRate);
				}

				if (offset > 9999) offset = 9999;
				uint32 bRandSlot = myrand(0, offset);
				uint32 nItemID = bRandArray[bRandSlot];

				_ITEM_TABLE pItem = g_pMain->GetItemPtr(nItemID);
				if (!pItem.isnull())
				{
					uint8 bFreeSlots = 0;
					for (int i = 0; i < WAREHOUSE_MAX; i++)
					{
						if (GetWerehouseItem(i)->nNum == 0 && ++bFreeSlots >= ITEMS_IN_EXCHANGE_GROUP)
							break;
					}

					int SlotForItem = FindWerehouseSlotForItem(pItem.m_iNum, 1);
					if (bFreeSlots > 0 && SlotForItem >= 0)
					{
						if (nItemID == ITEM_EXP)
						{
							if (GetLevel() >= 1 && GetLevel() <= 34)
								ExpChange("auto mining", 50);
							else if (GetLevel() >= 35 && GetLevel() <= 59)
								ExpChange("auto mining", 100);
							else if (GetLevel() >= 60 && GetLevel() <= 69)
								ExpChange("auto mining", 200);
							else if (GetLevel() >= 70 && GetLevel() <= 83)
								ExpChange("auto mining", 300);

							XSafe_ItemNotice(0, nItemID);
						}
						else
						{
							GiveWerehouseItem(nItemID, 1, false, true);
							XSafe_ItemNotice(0, nItemID);
						}
					}
				}
			}

			if (pFishingList.size() > 0)
			{
				uint32 bRandArray[10000], offset = 0; uint8 sItemSlot = 0;
				memset(bRandArray, 0, sizeof(bRandArray));
				foreach(itr, pFishingList)
				{
					auto* pFishing = g_pMain->m_MiningFishingItemArray.GetData(*itr);
					if (pFishing == nullptr)
						continue;

					if (offset >= 9999)
						break;

					for (int i = 0; i < int(pFishing->SuccessRate); i++)
					{
						if (i + offset >= 9999)
							break;

						bRandArray[offset + i] = pFishing->nGiveItemID;
					}
					offset += int(pFishing->SuccessRate);
				}

				if (offset > 9999) offset = 9999;
				uint32 bRandSlot = myrand(0, offset);
				uint32 nItemID = bRandArray[bRandSlot];

				_ITEM_TABLE pItem = g_pMain->GetItemPtr(nItemID);
				if (!pItem.isnull())
				{
					uint8 bFreeSlots = 0;
					for (int i = 0; i < WAREHOUSE_MAX; i++)
					{
						if (GetWerehouseItem(i)->nNum == 0 && ++bFreeSlots >= ITEMS_IN_EXCHANGE_GROUP)
							break;
					}

					int SlotForItem = FindWerehouseSlotForItem(pItem.m_iNum, 1);
					if (bFreeSlots > 0 && SlotForItem >= 0)
					{
						if (nItemID == ITEM_EXP)
						{
							if (GetLevel() >= 1 && GetLevel() <= 34)
								ExpChange("auto mining", 50);
							else if (GetLevel() >= 35 && GetLevel() <= 59)
								ExpChange("auto mining", 100);
							else if (GetLevel() >= 60 && GetLevel() <= 69)
								ExpChange("auto mining", 200);
							else if (GetLevel() >= 70 && GetLevel() <= 83)
								ExpChange("auto mining", 300);

							XSafe_ItemNotice(1, nItemID);
						}
						else
						{
							GiveWerehouseItem(nItemID, 1, true, false);
							XSafe_ItemNotice(1, nItemID);
						}
					}
				}
			}
		}
		if (m_tPlayerEffect <= UNIXTIME)
			m_tPlayerEffect = 0;

		KA_ResetCheck();
		EventAfkCheck();

		if (UNIXTIME > m_zoneonrewardtime)
		{
			ZoneOnlineRewardCheck();
			m_zoneonrewardtime = UNIXTIME + 10;
		}

		if (g_pMain->pLevMercInfo.status
			&& isMerchanting()
			&& UNIXTIME > level_merchant_time
			&& m_iMaxExp > 0)
		{
			int64 exp_count = m_iMaxExp / 20;
			if (exp_count > 0)
				ExpChange("level merchant reward", exp_count, true);

			level_merchant_time = UNIXTIME + (g_pMain->pLevMercInfo.rewardtime * MINUTE);
		}
	}
	if (UNIXTIME > m_SlavePriestSystem) {
		if (isInGame())
			CyberACS_BotPriestSystem();

		m_SlavePriestSystem = UNIXTIME + 4;
	}
}
#pragma endregion

void CUser::ZoneOnlineRewardCheck()
{
	// Zone Online / Pre Reward sistemi tamamen kapatildi.
	// Odul, mesaj, efekt ve sure kontrolu calismasin.
	return;
}

void CUser::ZoneOnlineRewardStart()
{
	// Zone Online / Pre Reward sistemi tamamen kapatildi.
	// Oyuncu girisinde reward listesi baslatilmasin.
	m_ZoneOnlineRewardLock.lock();
	m_ZoneOnlineReward.clear();
	m_ZoneOnlineRewardLock.unlock();
	return;
}

void CUser::ZoneOnlineRewardChange()
{
	// Zone degisiminde reward timer yenilenmesin.
	return;
}

void CUser::ZoneOnlineSendReward(_ZONE_ONLINE_REWARD pReward)
{
	// Emniyet: herhangi bir yerden cagrilsa bile odul/mesaj/efekt gonderme.
	return;
}

bool CUser::BDWAttackAreaCheck()
{
	if (GetNation() == (uint8)Nation::KARUS
		&& ((GetX() >= 183 && GetX() <= 250 && GetZ() >= 177 && GetZ() <= 232) || (GetX() >= 87 && GetX() <= 228 && GetZ() >= 203 && GetZ() <= 230)))
		return false;
	else if (GetNation() == (uint8)Nation::ELMORAD
		&& ((GetX() >= 0 && GetX() <= 73 && GetZ() >= 0 && GetZ() <= 85) || (GetX() >= 61 && GetX() <= 158 && GetZ() >= 27 && GetZ() <= 56)))
		return false;

	return true;
}

void CUser::EventAfkCheck()
{

	return;

	if (!isEventUser() || !isInTempleEventZone() || !m_event_afkcheck)
		return;

#define C_TIME 300
#define B_TIME 300
#define J_TIME 300
	if (!g_pMain->pTempleEvent.isActive || GetEventRoom() < 1 || GetEventRoom() > MAX_TEMPLE_EVENT_ROOM)
		return;

	if (UNIXTIME - m_event_afkcheck < B_TIME)
	{
		uint32 kalanzaman = B_TIME - static_cast<uint32>((UNIXTIME - m_event_afkcheck));
		uint8 TimeTravel = kalanzaman / 60;
		if (kalanzaman <= B_TIME)
		{
			if (TimeTravel != TimeTravelTemp && TimeTravelTemp <= 3)
				g_pMain->SendHelpDescription(this, string_format("Hareketsiz kaldığınız için %d dakika sonra moradon'a gönderileceksiniz", TimeTravel));

			TimeTravelTemp = TimeTravel;
		}
	}

	bool summon = false;
	if (g_pMain->pTempleEvent.ActiveEvent == TEMPLE_EVENT_CHAOS && GetZoneID() == ZONE_CHAOS_DUNGEON) {

		if (UNIXTIME - m_event_afkcheck < C_TIME)
			return;

		auto* pRoom = g_pMain->m_TempleEventChaosRoomList.GetData(GetEventRoom());
		if (!pRoom || pRoom->m_bFinished)
			return;

		summon = true;
	}
	else if (g_pMain->pTempleEvent.ActiveEvent == TEMPLE_EVENT_BORDER_DEFENCE_WAR && GetZoneID() == ZONE_BORDER_DEFENSE_WAR) {
		if (UNIXTIME - m_event_afkcheck < B_TIME/* || !BDWAttackAreaCheck()*/)
			return;

		auto* pRoom = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
		if (!pRoom || pRoom->m_bFinished)
			return;

		summon = true;
	}

	if (!summon) return;
	ResetTempleEventData();
	ZoneChange(ZONE_MORADON, 0.0f, 0.0f, 0);
	m_EventDisandTime = static_cast<uint32>(UNIXTIME + (12 * 60 * 60));
	SendBoxMessage(1, "A-F-K", "Event girişleriniz 12 saat boyunca yasaklanmıştır.");
	m_event_afkcheck = 0;
}

void CUser::SendServerIndex()
{
	Packet result(WIZ_SERVER_INDEX);
	result << uint16(1) << uint16(g_pMain->m_nServerNo);
	Send(&result);
}

/**
* @brief	Initiates a database request to save the character's information.
*/
void CUser::UserDataSaveToAgent()
{
	if (!isInGame())
		return;

	Packet result(WIZ_DATASAVE);
	result << GetAccountName() << GetName();
	g_pMain->AddDatabaseRequest(result, this);
}

/**
* @brief	Logs a player out.
*/
void CUser::LogOut()
{
	/*if (!m_addedmap || m_strAccountID.empty())
		return;*/

	if (m_strUserID.empty())
	{


		return g_pMain->RemoveSessionNames(this);
	}

	if (hasPriestBot == true)
	{
		CBot* pPriest = nullptr;
		pPriest = g_pMain->m_MapBotList.GetData(m_bUserPriestBotID);
		if (pPriest)
			pPriest->UserInOut(INOUT_OUT);

		m_bUserPriestBotID = -1;
		hasPriestBot = false;
		RemoveSavedMagic(495146);
	}

	//g_pMain->RemoveSessionNames(this);

	m_deleted = true;
	m_bIsLoggingOut = true;

	Packet newpkt(WIZ_LOGOUT);
	g_pMain->AddDatabaseRequest(newpkt, this);
}


void CUser::PlayerEffect(uint16 sEffect) // Giriş Effeck Eklendi DeaFSezo
{
	Packet result(WIZ_MINING, uint8(MiningAttempt));
	uint16 resultCode = MiningResultSuccess;
	result << resultCode << GetID() << sEffect;
	SendToRegion(&result);
}

/**
* @brief	Sends the server time.
*/
void CUser::SendTime()
{
	Packet result(WIZ_TIME);
	result << uint16(g_pMain->m_sYear) << uint16(g_pMain->m_sMonth) << uint16(g_pMain->m_sDate)
		<< uint16(g_pMain->m_sHour) << uint16(g_pMain->m_sMin);
	Send(&result);
}

/**s
* @brief	Sends the weather status.
*/
void CUser::SendWeather()
{
	Packet result(WIZ_WEATHER);
	result << g_pMain->m_byWeather << g_pMain->m_sWeatherAmount;
	Send(&result);
}

/**
* @brief	Sets various zone flags to control how
* 			the client handles other players/NPCs.
* 			Also sends the zone's current tax rate.
*/
void CUser::SetZoneAbilityChange(uint16 sNewZone)
{
	C3DMap* pMap = g_pMain->GetZoneByID(sNewZone);
	CKingSystem* pKingSystem = g_pMain->m_KingSystemArray.GetData(GetNation());

	if (pMap == nullptr)
		return;

	switch (sNewZone)
	{
	case ZONE_KARUS:
	case ZONE_KARUS2:
	case ZONE_KARUS3:
	case ZONE_ELMORAD:
	case ZONE_ELMORAD2:
	case ZONE_ELMORAD3:
	case ZONE_KARUS_ESLANT:
	case ZONE_KARUS_ESLANT2:
	case ZONE_KARUS_ESLANT3:
	case ZONE_ELMORAD_ESLANT:
	case ZONE_ELMORAD_ESLANT2:
	case ZONE_ELMORAD_ESLANT3:
	case ZONE_BIFROST:
	case ZONE_BATTLE:
	case ZONE_BATTLE2:
	case ZONE_BATTLE3:
	case ZONE_BATTLE4:
	case ZONE_BATTLE5:
	case ZONE_BATTLE6:
	case ZONE_SNOW_BATTLE:
	case ZONE_RONARK_LAND:
	case ZONE_ARDREAM:
	case ZONE_RONARK_LAND_BASE:
	case ZONE_KROWAZ_DOMINION:
	case ZONE_STONE1:
	case ZONE_STONE2:
	case ZONE_STONE3:
	case ZONE_BORDER_DEFENSE_WAR:
	case ZONE_UNDER_CASTLE:
	case ZONE_JURAID_MOUNTAIN:
	case ZONE_PARTY_VS_1:
	case ZONE_PARTY_VS_2:
	case ZONE_PARTY_VS_3:
	case ZONE_PARTY_VS_4:
	case ZONE_CLAN_WAR_ARDREAM:
	case ZONE_CLAN_WAR_RONARK:
	case ZONE_KNIGHT_ROYALE:
	case ZONE_CHAOS_DUNGEON:
		if (pKingSystem != nullptr)
			pMap->SetTariff(10 + pKingSystem->m_nTerritoryTariff);
		else
			pMap->SetTariff(10);
		break;
	case ZONE_MORADON:
	case ZONE_MORADON2:
	case ZONE_MORADON3:
	case ZONE_MORADON4:
	case ZONE_MORADON5:
	case ZONE_ARENA:
		pMap->SetTariff((uint8)g_pMain->pSiegeWar.sMoradonTariff);
		break;
	case ZONE_DELOS:
	case ZONE_DESPERATION_ABYSS:
	case ZONE_HELL_ABYSS:
	case ZONE_DELOS_CASTELLAN:
		pMap->SetTariff((uint8)g_pMain->pSiegeWar.sDellosTariff);
		break;
	default:
		//printf("King and Deos Tariff unhandled zone %d \n", sNewZone);
		TRACE("King and Deos Tariff unhandled zone %d \n", sNewZone);
		break;
	}

	Packet result(WIZ_ZONEABILITY, uint8(1));
	result << pMap->canTradeWithOtherNation()
		<< pMap->GetZoneType()
		<< pMap->canTalkToOtherNation()
		<< uint16(pMap->GetTariff());

	Send(&result);

	if (sNewZone == ZONE_ARDREAM
		|| sNewZone == ZONE_RONARK_LAND
		|| sNewZone == ZONE_RONARK_LAND_BASE
		|| GetZoneID() == ZONE_ARDREAM
		|| GetZoneID() == ZONE_RONARK_LAND
		|| GetZoneID() == ZONE_BIFROST
		|| GetZoneID() == ZONE_RONARK_LAND_BASE)
		PlayerKillingAddPlayerRank();
	else
		PlayerKillingRemovePlayerRank();

	if (isInSpecialEventZone((uint8)sNewZone) || isInWarZone((uint8)sNewZone))
		ZindanWarKillingAddPlayerRank();
	else
		ZindanWarKillingRemovePlayerRank();

	if (g_pMain->m_byBattleOpen == NATION_BATTLE
		&& g_pMain->m_byBattleZone + ZONE_BATTLE_BASE != ZONE_BATTLE3)
	{
		result.Initialize(WIZ_MAP_EVENT);
		result << uint8(GOLDSHELL) << uint8(true) << GetSocketID();
		Send(&result);
	}

	if (isSiegeTransformation())
	{
		_MAGIC_TYPE6* pType = g_pMain->m_Magictype6Array.GetData(m_bAbnormalType);

		m_sMaxHPAmount = 0;
		m_sMaxMPAmount = 0;

		if (pType != nullptr)
		{
			if (pType->sTotalAc == 0 && pType->sTotalAc > 0)
				m_sACPercent -= (pType->sTotalAc - 100);
			else
				m_sACAmount -= pType->sTotalAc;
		}
		result.Initialize(WIZ_MAGIC_PROCESS);
		result << uint8(MagicOpcode::MAGIC_CANCEL_TRANSFORMATION);

		m_transformationType = TransformationType::TransformationNone;
		m_transformSkillUseid = TransformationSkillUse::TransformationSkillUseNone;
		m_sTransformHpchange = false;
		m_sTransformMpchange = false;
		Send(&result);

		SetUserAbility();
		RemoveSavedMagic(m_bAbnormalType);
		StateChangeServerDirect(3, ABNORMAL_NORMAL);
	}

	if (sNewZone == ZONE_BIFROST
		|| sNewZone == ZONE_BATTLE4
		|| sNewZone == ZONE_BATTLE5
		|| sNewZone == ZONE_RONARK_LAND)
		g_pMain->SendEventRemainingTime(false, this, (uint8)sNewZone);

	if (g_pMain->pLotteryProc.LotteryStart && g_pMain->pLotteryProc.EventTime != 0)
		LotteryGameStartSend();

	if (sNewZone == ZONE_BIFROST || GetZoneID() == ZONE_BIFROST || sNewZone == ZONE_RONARK_LAND || GetZoneID() == ZONE_RONARK_LAND) BeefEventGetTime();

	m_lockCoolDownList.lock();
	m_CoolDownList.clear();
	m_lockCoolDownList.unlock();

	m_PetSkillCooldownList.clear();
	m_SkillCastList.clear();
	m_MagicTypeCooldownList.clear();
}

/**
* @brief	Requests user info for the specified session IDs.
*
* @param	pkt	The packet.
*/
void CUser::RequestUserIn(Packet& pkt)
{
	Packet result(WIZ_REQ_USERIN);
	short user_count = pkt.read<uint16>(), online_count = 0;
	if (user_count > 1000)
		user_count = 1000;

	result << uint16(0); // placeholder for user count

	std::vector<CUser*> mList;
	for (int i = 0; i < user_count; i++)
	{
		uint16 nid = pkt.read<uint16>();

		if (nid < MAX_USER)
		{
			CUser* pUser = g_pMain->GetUserPtr(nid);
			if (pUser == nullptr || !pUser->isInGame())
				continue;

			if (GetEventRoom() >= 0 && pUser->GetEventRoom() != GetEventRoom())
				continue;

			if (pUser->m_bAbnormalType == ABNORMAL_INVISIBLE)
				continue;

			if (GetSocketID() == pUser->GetSocketID())
				continue;

			if (!CheckInvisibility(pUser))
				continue;

			result << uint8(0) << pUser->GetSocketID();
			pUser->GetUserInfo(result);
			mList.push_back(pUser);

			online_count++;
		}
		else
		{
			CBot* pUser = g_pMain->GetBotPtr(nid);
			if (pUser == nullptr || !pUser->isInGame())
				continue;

			result << uint8(0) << pUser->GetID();
			pUser->GetUserInfo(result);

			online_count++;
		}
	}

	result.put(0, online_count);
	if (result.size() > 32500)
		SendCompressed(&result);
	else
		Send(&result);

	if (online_count) 
		g_pMain->MerchantUserInOutForMe(this);

	if (!mList.empty())
		UserInOutTag(mList);
}

void CUser::RequestNpcIn(Packet& pkt)
{
	Packet result(WIZ_REQ_NPCIN);
	uint16 recvcount = 0, Counter = 0;
	pkt >> recvcount;

	if (!recvcount)
		return;

	if (recvcount > 1000)
		recvcount = 1000;

	result << uint16(0);

	for (int i = 0; i < recvcount; i++)
	{
		uint16 sockid = pkt.read<uint16>();
		if (sockid < NPC_BAND /*|| sockid > INVALID_BAND*/)
			continue;

		CNpc* pNpc = g_pMain->GetNpcPtr(sockid, GetZoneID());
		if (pNpc == nullptr || pNpc->isDead() || GetZoneID() != pNpc->GetZoneID() || pNpc->GetEventRoom() != GetEventRoom())
			continue;


		result << pNpc->GetID();
		pNpc->GetNpcInfo(result, GetClanID());
		Counter++;

		if (result.size() > 500)
		{
			result.put(0, Counter);
			SendCompressed(&result);
			Counter = 0;
			result.Initialize(WIZ_REQ_NPCIN);
			result << uint16(0); // placeholder for NPC count
		}
	}
	result.put(0, Counter);
	Send(&result);
}

void CUser::SetSlotItemValue()
{
	_ITEM_TABLE pTable = _ITEM_TABLE();
	int item_hit = 0, item_ac = 0;

	m_sItemMaxHp = m_sItemMaxMp = 0;
	m_sItemAc = 0;
	m_sItemWeight = m_sMaxWeightBonus = 0;
	m_sItemHitrate = m_sItemEvasionrate = 100;

	memset(m_sStatItemBonuses, 0, sizeof(uint16) * (uint16)StatType::STAT_COUNT);
	m_sFireR = m_sColdR = m_sLightningR = m_sMagicR = m_sDiseaseR = m_sPoisonR = 0;
	m_sDaggerR = m_sSwordR = m_sJamadarR = m_sAxeR = m_sClubR = m_sSpearR = m_sBowR = 0;

	m_byAPBonusAmount = 0;
	memset(&m_byAPClassBonusAmount, 0, sizeof(m_byAPClassBonusAmount));
	memset(&m_byAcClassBonusAmount, 0, sizeof(m_byAcClassBonusAmount));

	m_bItemExpGainAmount = m_bItemNPBonus = m_bItemNoahGainAmount = 0;

	m_equippedItemBonusLock.lock();
	m_equippedItemBonuses.clear();
	m_equippedItemBonusLock.unlock();

	map<uint16, uint32> setItems;

	// Apply stat bonuses from all equipped & cospre items.
	// Total up the weight of all items.
	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = nullptr;
		pTable = GetItemPrototype(i, pItem);

		if (pTable.isnull())
			continue;

		// Bags increase max weight, they do not weigh anything.
		if (i == INVENTORY_COSP + COSP_BAG1 + 1
			|| i == INVENTORY_COSP + COSP_BAG2 + 1)
		{
			m_sMaxWeightBonus += pTable.m_sDuration;
		}
		// All other items are attributed to the total weight of items in our inventory.
		else
		{
			if (!pTable.m_bCountable)
				m_sItemWeight += pTable.m_sWeight;
			else
				// Non-stackable items should have a count of 1. If not, something's broken.
				m_sItemWeight += pTable.m_sWeight * pItem->sCount;
		}

		// Do not apply stats to unequipped items
		if ((i >= SLOT_MAX && i < INVENTORY_COSP)
			// or disabled weapons.
			|| (isWeaponsDisabled()
				&& (i == RIGHTHAND || i == LEFTHAND)
				&& !pTable.isShield())
			// or items in magic bags.
			|| i >= INVENTORY_MBAG
			|| pItem->isDuplicate())
			continue;

		item_ac = pTable.m_sAc;
		if (pItem->sDuration == 0)
			item_ac /= 10;

		m_sItemMaxHp += pTable.m_MaxHpB;
		m_sItemMaxMp += pTable.m_MaxMpB;
		m_sItemAc += item_ac;
		m_sStatItemBonuses[(int16)StatType::STAT_STR] += pTable.m_sStrB;
		m_sStatItemBonuses[(int16)StatType::STAT_STA] += pTable.m_sStaB;
		m_sStatItemBonuses[(int16)StatType::STAT_DEX] += pTable.m_sDexB;
		m_sStatItemBonuses[(int16)StatType::STAT_INT] += pTable.m_sIntelB;
		m_sStatItemBonuses[(int16)StatType::STAT_CHA] += pTable.m_sChaB;
		m_sItemHitrate += pTable.m_sHitrate;
		m_sItemEvasionrate += pTable.m_sEvarate;

		m_sFireR += pTable.m_bFireR;
		m_sColdR += pTable.m_bColdR;
		m_sLightningR += pTable.m_bLightningR;
		m_sMagicR += pTable.m_bMagicR;
		m_sDiseaseR += pTable.m_bCurseR;
		m_sPoisonR += pTable.m_bPoisonR;

		m_sDaggerR += pTable.m_sDaggerAc;
		m_sJamadarR += pTable.m_JamadarAc;
		m_sSwordR += pTable.m_sSwordAc;
		m_sAxeR += pTable.m_sAxeAc;
		m_sClubR += pTable.m_sClubAc;
		m_sSpearR += pTable.m_sSpearAc;
		m_sBowR += pTable.m_sBowAc;

		ItemBonusMap bonusMap;
		if (pTable.m_bFireDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_FIRE, pTable.m_bFireDamage));

		if (pTable.m_bIceDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_COLD, pTable.m_bIceDamage));

		if (pTable.m_bLightningDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_LIGHTNING, pTable.m_bLightningDamage));

		if (pTable.m_bPoisonDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_POISON, pTable.m_bPoisonDamage));

		if (pTable.m_bHPDrain)
			bonusMap.insert(std::make_pair(ITEM_TYPE_HP_DRAIN, pTable.m_bHPDrain));

		if (pTable.m_bMPDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_MP_DAMAGE, pTable.m_bMPDamage));

		if (pTable.m_bMPDrain)
			bonusMap.insert(std::make_pair(ITEM_TYPE_MP_DRAIN, pTable.m_bMPDrain));

		if (pTable.m_bMirrorDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_MIRROR_DAMAGE, pTable.m_bMirrorDamage));

		// If we have bonuses to apply, store them.
		if (!bonusMap.empty())
		{
			m_equippedItemBonusLock.lock();
			m_equippedItemBonuses[i] = bonusMap;
			m_equippedItemBonusLock.unlock();
		}

		bool cospreextraitem = pTable.Getnum() == 610019000;

		// Apply cospre item stats
		if (pTable.GetKind() == ITEM_KIND_COSPRE || cospreextraitem)
		{
			// If this item exists in the set table, it has bonuses to be applied.
			_SET_ITEM* pSetItem = g_pMain->m_SetItemArray.GetData(pTable.m_iNum);

			if (pSetItem != nullptr)
				ApplySetItemBonuses(pSetItem);
		}

		// All set items start with race over 100
		if (pTable.m_bRace < 100)
			continue;

		// Each set is uniquely identified by item's race
		auto itr = setItems.find(pTable.m_bRace);

		// If the item doesn't exist in our map yet...
		if (itr == setItems.end())
		{
			// Generate the base set ID and insert it into our map
			setItems.insert(make_pair(pTable.m_bRace, pTable.m_bRace * 10000));
			itr = setItems.find(pTable.m_bRace);
		}

		// Update the final set ID depending on the equipped set item 
		switch (pTable.m_bSlot)
		{
		case ItemSlotHelmet:
			itr->second += 2;
			break;
		case ItemSlotPauldron:
			itr->second += 16;
			break;
		case ItemSlotPads:
			itr->second += 512;
			break;
		case ItemSlotGloves:
			itr->second += 2048;
			break;
		case ItemSlotBoots:
			itr->second += 4096;
			break;
		}
	}

	// Now we can add up all the set bonuses, if any.
	foreach(itr, setItems)
	{
		// Test if this set item exists (if we're not using at least 2 items from the set, this will fail)
		_SET_ITEM* pItem = g_pMain->m_SetItemArray.GetData(itr->second);

		if (pItem == nullptr)
			continue;

		ApplySetItemBonuses(pItem);
	}

	if (isInClan())
	{
		if (CKnights* pKnights = g_pMain->GetClanPtr(GetClanID()))
		{
			if (pKnights->isCastellanCape())
			{
				auto* pKnightCape = g_pMain->m_KnightsCapeArray.GetData(pKnights->m_castCapeID);
				if (pKnightCape && pKnightCape->BonusType > 0) ApplyCastellanCapeBonueses(pKnightCape);
			}
			else
			{
				auto* pKnightCape = g_pMain->m_KnightsCapeArray.GetData(pKnights->GetCapeID());
				if (pKnightCape && pKnightCape->BonusType > 0) ApplyCastellanCapeBonueses(pKnightCape);
			}
		}
	}
	/*26.04.2024 RANK BONUS SYSTEM*/
	if (m_bPersonalRank == 1)
		ApplyRankBonueses(uint8(1));
	else if (m_bPersonalRank > 1 && m_bPersonalRank <= 4)
		ApplyRankBonueses(uint8(2));
	else if (m_bPersonalRank > 4 && m_bPersonalRank <= 10)
		ApplyRankBonueses(uint8(3));
	else if (m_bPersonalRank > 10 && m_bPersonalRank <= 40)
		ApplyRankBonueses(uint8(4));
	else if (m_bPersonalRank > 40 && m_bPersonalRank <= 100)
		ApplyRankBonueses(uint8(5));
	else if (m_bPersonalRank > 100 && m_bPersonalRank <= 200)
		ApplyRankBonueses(uint8(6));
	/*26.04.2024 RANK BONUS SYSTEM*/

	if (m_sAddArmourAc > 0)
		m_sItemAc += m_sAddArmourAc;
	else
		m_sItemAc = m_sItemAc * m_bPctArmourAc / 100;

	// JstKO Yeni Süre İtemler Deleted Notice Eklendi. 06.02.2025 
	JstKOExprationItemDeleted();
}

/*26.04.2024 RANK BONUS SYSTEM*/
std::string GenerateBonusNotice(const _CAPE_CASTELLAN_BONUS* pBonus) {
	if (pBonus == nullptr)
		return "";

	std::string noticeMessage;

	if (pBonus->HPBonus > 0) {
		noticeMessage += string_format("HPBonus +%d", pBonus->HPBonus);
	}

	if (pBonus->MPBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("MaxMPBonus +%d", pBonus->MPBonus);
	}

	if (pBonus->StrengthBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("StrBonus +%d", pBonus->StrengthBonus);
	}

	if (pBonus->StaminaBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("StaminaBonus +%d", pBonus->StaminaBonus);
	}

	if (pBonus->DexterityBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("DexBonus +%d", pBonus->DexterityBonus);
	}

	if (pBonus->IntelBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("IntBonus +%d", pBonus->IntelBonus);
	}

	if (pBonus->CharismaBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("MPBonus +%d", pBonus->CharismaBonus);
	}

	if (pBonus->FlameResistance > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("FrResist +%d", pBonus->FlameResistance);
	}

	if (pBonus->GlacierResistance > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("GrResist +%d", pBonus->GlacierResistance);
	}

	if (pBonus->LightningResistance > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("LrResist +%d", pBonus->LightningResistance);
	}

	if (pBonus->PoisonResistance > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("PoisonResist +%d", pBonus->PoisonResistance);
	}

	if (pBonus->MagicResistance > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("MagicResist +%d", pBonus->MagicResistance);
	}

	if (pBonus->CurseResistance > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("CurseResist +%d", pBonus->CurseResistance);
	}

	if (pBonus->XPBonusPercent > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("XPBonus +%d", pBonus->XPBonusPercent);
	}

	if (pBonus->CoinBonusPercent > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("CoinBonus +%d", pBonus->CoinBonusPercent);
	}

	if (pBonus->APBonusPercent > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("APBonus +%d", pBonus->APBonusPercent);
	}

	if (pBonus->APBonusClassType > 0 && pBonus->APBonusClassPercent > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("APBonusClass +%d |for ClassType %d", pBonus->APBonusClassPercent, pBonus->APBonusClassType);
	}

	if (pBonus->ACBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("ACBonus +%d", pBonus->ACBonus);
	}

	if (pBonus->ACBonusClassType > 0 && pBonus->ACBonusClassPercent > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("ACBonusClass +%d |for ClassType %d", pBonus->ACBonusClassPercent, pBonus->ACBonusClassType);
	}

	if (pBonus->MaxWeightBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("MaxWeightBonus +%d", pBonus->MaxWeightBonus);
	}

	if (pBonus->NPBonus > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("NPBonus +%d", pBonus->NPBonus);
	}

	return noticeMessage;
}

void CUser::ApplyRankBonueses(uint8 sType)
{
	if (!sType)
		return;

	_CAPE_CASTELLAN_BONUS* pBonus = g_pMain->m_CapeCastellanBonus.GetData(sType);
	if (pBonus == nullptr)
		return;

	m_sItemAc += pBonus->ACBonus;
	m_sItemMaxHp += pBonus->HPBonus;
	m_sItemMaxMp += pBonus->MPBonus;

	m_sStatItemBonuses[(int16)StatType::STAT_STR] += pBonus->StrengthBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_STA] += pBonus->StaminaBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_DEX] += pBonus->DexterityBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_INT] += pBonus->IntelBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_CHA] += pBonus->CharismaBonus;

	m_sFireR += pBonus->FlameResistance;
	m_sColdR += pBonus->GlacierResistance;
	m_sLightningR += pBonus->LightningResistance;
	m_sMagicR += pBonus->MagicResistance;
	m_sDiseaseR += pBonus->CurseResistance;
	m_sPoisonR += pBonus->PoisonResistance;

	m_bItemExpGainAmount += pBonus->XPBonusPercent;
	m_bItemNoahGainAmount += pBonus->CoinBonusPercent;
	m_bItemNPBonus += pBonus->NPBonus;

	m_sMaxWeightBonus += pBonus->MaxWeightBonus;

	// NOTE: The following percentages use values such as 3 to indicate +3% (not the typical 103%).
	// Also note that at this time, there are no negative values used, so we can assume it's always a bonus.
	m_byAPBonusAmount += pBonus->APBonusPercent;
	if (pBonus->APBonusClassType >= 1 && pBonus->APBonusClassType <= 4)
		m_byAPClassBonusAmount[pBonus->APBonusClassType - 1] += pBonus->APBonusClassPercent;

	if (pBonus->ACBonusClassType >= 1 && pBonus->ACBonusClassType <= 4)
		m_byAcClassBonusAmount[pBonus->ACBonusClassType - 1] += pBonus->ACBonusClassPercent;

	Packet notice(WIZ_NOTICE);
	notice.DByte();
	notice << uint8(4) << uint8(1) << "Rank Bonus" << GenerateBonusNotice(pBonus);
	Send(&notice);
}
/*26.04.2024 RANK BONUS SYSTEM*/

void CUser::ApplyCastellanCapeBonueses(_KNIGHTS_CAPE* pCape)
{
	if (pCape == nullptr || pCape->BonusType <= 0)
		return;

	_CAPE_CASTELLAN_BONUS* pBonus = g_pMain->m_CapeCastellanBonus.GetData(pCape->BonusType);
	if (pBonus == nullptr)
		return;

	CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
	if (pKnights == nullptr)
		return;

	m_sItemAc += pBonus->ACBonus;
	m_sItemMaxHp += pBonus->HPBonus;
	m_sItemMaxMp += pBonus->MPBonus;

	m_sStatItemBonuses[(int16)StatType::STAT_STR] += pBonus->StrengthBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_STA] += pBonus->StaminaBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_DEX] += pBonus->DexterityBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_INT] += pBonus->IntelBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_CHA] += pBonus->CharismaBonus;

	m_sFireR += pBonus->FlameResistance;
	m_sColdR += pBonus->GlacierResistance;
	m_sLightningR += pBonus->LightningResistance;
	m_sMagicR += pBonus->MagicResistance;
	m_sDiseaseR += pBonus->CurseResistance;
	m_sPoisonR += pBonus->PoisonResistance;

	m_bItemExpGainAmount += pBonus->XPBonusPercent;
	m_bItemNoahGainAmount += pBonus->CoinBonusPercent;
	m_bItemNPBonus += pBonus->NPBonus;

	m_sMaxWeightBonus += pBonus->MaxWeightBonus;

	// NOTE: The following percentages use values such as 3 to indicate +3% (not the typical 103%).
	// Also note that at this time, there are no negative values used, so we can assume it's always a bonus.
	m_byAPBonusAmount += pBonus->APBonusPercent;
	if (pBonus->APBonusClassType >= 1 && pBonus->APBonusClassType <= 4)
		m_byAPClassBonusAmount[pBonus->APBonusClassType - 1] += pBonus->APBonusClassPercent;

	if (pBonus->ACBonusClassType >= 1 && pBonus->ACBonusClassType <= 4)
		m_byAcClassBonusAmount[pBonus->ACBonusClassType - 1] += pBonus->ACBonusClassPercent;
}

void CUser::ApplySetItemBonuses(_SET_ITEM* pItem)
{
	if (pItem == nullptr)
		return;

	m_sItemAc += pItem->ACBonus;
	m_sItemMaxHp += pItem->HPBonus;
	m_sItemMaxMp += pItem->MPBonus;

	m_sStatItemBonuses[(int16)StatType::STAT_STR] += pItem->StrengthBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_STA] += pItem->StaminaBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_DEX] += pItem->DexterityBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_INT] += pItem->IntelBonus;
	m_sStatItemBonuses[(int16)StatType::STAT_CHA] += pItem->CharismaBonus;

	m_sFireR += pItem->FlameResistance;
	m_sColdR += pItem->GlacierResistance;
	m_sLightningR += pItem->LightningResistance;
	m_sMagicR += pItem->MagicResistance;
	m_sDiseaseR += pItem->CurseResistance;
	m_sPoisonR += pItem->PoisonResistance;

	m_bItemExpGainAmount += pItem->XPBonusPercent;
	m_bItemNoahGainAmount += pItem->CoinBonusPercent;
	m_bItemNPBonus += pItem->NPBonus;

	m_sMaxWeightBonus += pItem->MaxWeightBonus;

	// NOTE: The following percentages use values such as 3 to indicate +3% (not the typical 103%).
	// Also note that at this time, there are no negative values used, so we can assume it's always a bonus.
	m_byAPBonusAmount += pItem->APBonusPercent;
	if (pItem->APBonusClassType >= 1 && pItem->APBonusClassType <= 4)
		m_byAPClassBonusAmount[pItem->APBonusClassType - 1] += pItem->APBonusClassPercent;

	if (pItem->ACBonusClassType >= 1 && pItem->ACBonusClassType <= 4)
		m_byAcClassBonusAmount[pItem->ACBonusClassType - 1] += pItem->ACBonusClassPercent;
}

void CUser::RecvUserExp(CNpc* pNpc, uint32 iDamage, uint32 iTotalDamage)
{
	if (!iDamage || !iTotalDamage)
		return;

	if (pNpc == nullptr || !isInRange(pNpc, RANGE_50M))
		return;

	if (pNpc->GetType() == NPC_SANTA)
		return;

	if (pNpc->isMonster() && pNpc->GetProtoID() == 4301 || pNpc->GetProtoID() == 4351)
		if (!VaccuniBoxExp(1))
			return;

	if (pNpc->isMonster() && pNpc->GetProtoID() == 616 || pNpc->GetProtoID() == 666)
		if (!VaccuniBoxExp(2))
			return;

	if (pNpc->isMonster() && pNpc->GetProtoID() == 605 || pNpc->GetProtoID() == 611 || pNpc->GetProtoID() == 655)
		if (!VaccuniBoxExp(3))
			return;

	uint16 protoid = pNpc->GetProtoID();

	int32 iNpcExp = pNpc->GetProto()->m_iExp, iNpcLoyalty = pNpc->GetProto()->m_iLoyalty;
	if (iNpcExp <= 0 && iNpcLoyalty <= 0)
		return;

	uint32 nFinalExp = 0, nFinalLoyalty = 0;
	double TempValue = 0;

	if (iDamage > iTotalDamage)
		iDamage = iTotalDamage;

	if (iNpcExp > 0)
	{
		TempValue = iNpcExp * ((double)iDamage / (double)iTotalDamage);
		if (TempValue < 0.0) TempValue = 0.0;

		nFinalExp = (int)TempValue;
		if (TempValue > nFinalExp)
			nFinalExp++;
	}

	if (iNpcLoyalty > 0)
	{
		TempValue = iNpcLoyalty * ((double)iDamage / (double)iTotalDamage);
		if (TempValue < 0.0) TempValue = 0.0;

		nFinalLoyalty = (int)TempValue;
		if (TempValue > nFinalLoyalty)
			nFinalLoyalty++;
	}

	_PARTY_GROUP* pParty = nullptr;
	if (!isInParty() || (pParty = g_pMain->GetPartyPtr(GetPartyID())) == nullptr)
	{
		if (isDead()) return;

		float fModifier = pNpc->GetRewardModifier(GetLevel());
		if (iNpcExp > 0)
		{

			TempValue = nFinalExp * fModifier;
			nFinalExp = (int)TempValue;
			if (TempValue > nFinalExp)
				nFinalExp++;

			if (nFinalExp > 0)
			{
				if (!JackPotExp(nFinalExp))
					ExpChange(string_format("monster (1)exp protoid=%d", protoid), nFinalExp);
			}
		}

		if (iNpcLoyalty > 0)
		{
			TempValue = nFinalLoyalty * fModifier;
			nFinalLoyalty = (int)TempValue;
			if (TempValue > nFinalLoyalty)
				nFinalLoyalty++;

			if (nFinalLoyalty > 0)
				SendLoyaltyChange("Npc Loyalty", nFinalLoyalty);
		}
		return;
	}

	// Handle party XP/NP gain
	std::vector<CUser*> partyUsers;
	uint32 nTotalLevel = 0;
	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUser == nullptr || pUser->isDead() || !pUser->isInRange(pNpc, RANGE_50M)) continue;
		partyUsers.push_back(pUser);
	}

	if (partyUsers.empty())
		return;

	const float fPartyModifierXP = 0.3f, fPartyModifierNP = 0.2f;

	int nPartyMembers = (int)partyUsers.size();
	foreach(itr, partyUsers)
	{
		CUser* pUser = (*itr);
		if (pUser == nullptr || pUser->isDead() || !pUser->isInRange(pNpc, RANGE_50M))
		{
			if (nPartyMembers > 0) nPartyMembers--;
			continue;
		}
		nTotalLevel += pUser->GetLevel();
	}
	if (nPartyMembers < 2) nPartyMembers = 1;

	// Calculate the amount to adjust the XP/NP based on level difference.
	float fModifier = 1;// pNpc->GetPartyRewardModifier(nTotalLevel, nPartyMembers);

	if (iNpcExp > 0)
	{
		TempValue = nFinalExp * fModifier;
		nFinalExp = (int)TempValue;
		if (TempValue > nFinalExp)
			nFinalExp++;
	}

	if (iNpcLoyalty > 0)
	{
		TempValue = nFinalLoyalty * fModifier;
		nFinalLoyalty = (int)TempValue;
		if (TempValue > nFinalLoyalty)
			nFinalLoyalty++;
	}

	// Hand out kill rewards to all users in the party and still in range.
	foreach(itr, partyUsers)
	{
		CUser* pUser = (*itr);
		if (pUser == nullptr || pUser->isDead() || !pUser->isInRange(pNpc, RANGE_50M))
			continue;

		if (nFinalExp > 0 && !pUser->JackPotExp(nFinalExp))
			pUser->ExpChange(string_format("monster (2)exp protoid=%d", protoid), nFinalExp);

		if (iNpcLoyalty > 0)
		{
			TempValue = (nFinalLoyalty * (1 + fPartyModifierNP * (nPartyMembers - 1))) * (double)pUser->GetLevel() / (double)nTotalLevel;
			int iLoyalty = (int)TempValue;
			if (TempValue > iLoyalty) iLoyalty++;
			pUser->SendLoyaltyChange("Npc Loyalty", iLoyalty);
		}
	}
}

/**
* @brief	Sends a HP update to the user's party.
*/
void CUser::SendPartyHPUpdate()
{
	Packet result(WIZ_PARTY);
	result << uint8(PARTY_HPCHANGE)
		<< GetSocketID()
		<< m_MaxHp << m_sHp
		<< m_MaxMp << m_sMp;
	g_pMain->Send_PartyMember(GetPartyID(), &result);
}

void CUser::SendPartyHpManager(uint8 type) //17.12.2020 Partyde HP Degeri Yazi Olarak Gosterme
{
	int hp = 0, maxhp = 0;
	hp = m_sHp;
	maxhp = m_MaxHp;
	Packet result(WIZ_PARTY_HP);
	result << uint8(1) << GetName() << maxhp << hp;

	if (type == 0)
		g_pMain->Send_PartyMember(GetPartyID(), &result);
	else if (type == 1)
		Send(&result);
}

/**
* @brief	Shows the specified skill's effect
* 			to the surrounding regions.
*
* @param	nSkillID	Skill identifier.
*/
void CUser::ShowEffect(uint32 nSkillID)
{
	Packet result(WIZ_EFFECT);
	if (isInGame())
	{
		result << GetID() << nSkillID;
		SendToRegion(&result);
	}
}

/**
* @brief	Shows an effect on the NPC currently
* 			being interacted with.
*
* @param	nEffectID	Identifier for the effect.
*/
void CUser::ShowNpcEffect(uint32 nEffectID, bool bSendToRegion)
{
	Packet result(WIZ_OBJECT_EVENT, uint8(OBJECT_NPC));
	result << uint8(3) << m_sEventNid << nEffectID;
	if (bSendToRegion)
		SendToRegion(&result);
	else
		Send(&result);
}

/**
* @brief	Sends the target's HP to the player.
*
* @param	echo  	Client-based flag that we must echo back to the client.
* 					Set to 0 if not responding to the client.
* @param	tid   	The target's ID.
* @param	damage	The amount of damage taken on this request, 0 if it does not apply.
*/
//void CUser::SendTargetHP(uint8 echo, int tid, int damage)
void CUser::SendTargetHP(uint8 echo, int tid, int damage, bool TargetStatus)// target Scroll Bar Eklemesi 03.02.2024 end
{
	if (tid == -1)
		return;

	std::vector<uint32_t> Skillicon;//target sc new end
	bool partycheck = false;
	int hp = 0, maxhp = 0;

	uint8 prototype = 0;
	/*
		1.monster
		2.npc
		3.bot
		4.real user
	*/

	Packet pkt(WIZ_TARGET_HP);
	pkt << uint16(tid) << echo;
	if (tid >= NPC_BAND)
	{
		CNpc* pNpc = g_pMain->GetNpcPtr(tid, GetZoneID());
		if (!pNpc || pNpc->isDead() || pNpc->GetZoneID() != GetZoneID())
			return;

		pkt << (int)pNpc->m_MaxHP << (int)pNpc->m_iHP;

		if (pNpc->isMonster())
			prototype = 1;
		else
			prototype = 2;


		//target sc new
		if (TargetStatus)
		{
			{
				Guard lock(pNpc->m_buffLock);
				foreach(itr, pNpc->m_buffMap)
				{
					Skillicon.push_back(itr->second.m_nSkillID);
				}
			}
			for (auto& durationalSkill : pNpc->m_durationalSkills)
			{
				MagicType3* pEffect = &durationalSkill;
				if (!pEffect->m_byUsed)
					continue;

				Skillicon.push_back(pEffect->m_skill);
			}
		}
		//target sc new end

	}
	else
	{

		if (tid < MAX_USER)
		{
			CUser* pUser = g_pMain->GetUserPtr(tid);
			if (pUser == nullptr || pUser->isDead() || pUser->GetZoneID() != GetZoneID())
				return;

			pkt << (int)pUser->m_MaxHp << (int)pUser->m_sHp;

			if (pUser->isInParty() && isInParty() && pUser->GetPartyID() == GetPartyID())
				partycheck = true;

			prototype = 4;

			//target sc new
			if (TargetStatus)
			{
				{
					Guard lock(pUser->m_buffLock);
					foreach(itr, pUser->m_buffMap)
					{
						Skillicon.push_back(itr->second.m_nSkillID);
					}
				}
				for (auto& durationalSkill : pUser->m_durationalSkills)
				{
					MagicType3* pEffect = &durationalSkill;
					if (!pEffect->m_byUsed)
						continue;

					Skillicon.push_back(pEffect->m_skill);
				}
			}
			//target sc new end
		}
		else
		{
			CBot* pBot = g_pMain->GetBotPtr(tid);
			if (pBot == nullptr || pBot->isDead())
				return;

			pkt << pBot->GetMaxHealth() << pBot->GetHealth();
			prototype = 3;

			//target sc new
			if (TargetStatus)
			{
				{
					Guard lock(pBot->m_buffLock);
					foreach(itr, pBot->m_buffMap)
					{
						Skillicon.push_back(itr->second.m_nSkillID);
					}
				}
				for (auto& durationalSkill : pBot->m_durationalSkills)
				{
					MagicType3* pEffect = &durationalSkill;
					if (!pEffect->m_byUsed)
						continue;

					Skillicon.push_back(pEffect->m_skill);
				}
			}
			//target sc new end

		}
	}

	pkt << uint16(damage) << GetID() << prototype;
	Send(&pkt);
	//target sc new
	if (TargetStatus && tid != GetID())
	{
		if (Skillicon != Skillsave2)
		{
			Packet SendPacket(XSafe, uint8_t(XSafeOpCodes::PKT_HOOK_TARGET_SCROLL));
			SendPacket << uint8_t(1);
			for (uint32_t skillID : Skillicon)
			{
				SendPacket << skillID;
			}
			Send(&SendPacket);
			Skillsave2 = std::move(Skillicon);
		}
	}
	//target sc new end

	if (isInParty()
		&& isPartyCommandLeader()
		&& tid != m_oldTargetID
		&& tid != GetSocketID()
		&& !partycheck)
	{
		Packet newpkt(WIZ_PARTY, uint8(PARTY_TARGET_NUMBER));
		newpkt << (uint16)tid << uint8(0);
		m_oldTargetID = tid;
		g_pMain->Send_PartyMember(GetPartyID(), &newpkt);
	}
}

#pragma region CUser::HandleTargetHP(Packet & pkt)
void CUser::HandleTargetHP(Packet& pkt)
{
	uint16 uid = pkt.read<uint16>();
	uint8 echo = pkt.read<uint8>();


	if (isGM() && uid != GetTargetID())
	{
		if (uid >= MAX_USER)
		{
			CBot* pBot = g_pMain->GetBotPtr(uid);
			if (pBot != nullptr)
			{
				string strMsg = string_format("ID = %d, Name = %s, Nation = %d", pBot->GetID(), pBot->GetName().c_str(), pBot->GetNation());
				Packet Notice(WIZ_CHAT, uint8(GENERAL_CHAT));
				Notice << uint8(0) << int16(-1) << uint8(0) << strMsg;
				Send(&Notice);
			}
		}
	}

	m_targetID = uid;
	SendTargetHP(echo, uid);
	//GM YARATIK BİLGİSİ ALMA
	//if (isGM() && echo == 1)
	//{
	//	CNpc* pNpc = g_pMain->GetNpcPtr(uid, GetZoneID());
	//	if (pNpc == nullptr)
	//		return;

	//	std::string strNpcName = pNpc->GetName();
	//	if (strNpcName.length() == 0)
	//		strNpcName = "<NoName>";
	//	g_pMain->SendHelpDescription(this, string_format("[Npc Name]  = %s | [Npc ID]  = %d | [Npc Proto ID] = %d",
	//		strNpcName.c_str(), pNpc->GetID(), pNpc->GetProtoID()));

	//}
	//GM YARATIK BİLGİSİ ALMA BİTİŞ
}
#pragma endregion

/**
* @brief	Packet handler for various player state changes.
*
* @param	pkt	The packet.
*/
void CUser::StateChange(Packet& pkt)
{
	if (isDead())
		return;

	uint8 bType = pkt.read<uint8>(), buff;
	uint32 nBuff = pkt.read<uint32>();

	if (bType == 3 && (!isGM() || (nBuff != 1 && nBuff != 5 && nBuff != 2 && nBuff != 3)))
		return;

	buff = *(uint8*)&nBuff; // don't ask
	m_iTotalTrainingExp = 0;
	m_iTotalTrainingTime = 0;
	m_lastTrainingTime = 0;

	switch (bType)
	{
	case 1:
		if (buff != USER_STANDING && buff != USER_SITDOWN && buff != USER_MONUMENT)
			return;
		break;
	case 2:
		break;
	case 3:
		// /unview | /view
		if ((buff == 1 || buff == 5 || buff == 2 || buff == 3) && !isGM())
			return;
		break;

	case 4: // emotions
		switch (buff)
		{
		case 1: // Greeting 1-3
		case 2:
		case 3:
		case 11: // Provoke 1-3
		case 12:
			// kurian güncellemesi 16.12.2023
		case 13:
			break;
		case 15:
			m_nOldAbnormalType = m_bAbnormalType;

			// If we're a GM, we need to show ourselves before transforming.
			// Otherwise the visibility state is completely desynced.
			if (isGM())
				StateChangeServerDirect(5, 1);

			m_bAbnormalType = nBuff;

			UserInOut(INOUT_OUT);
			UserInOut(INOUT_IN);

			bType = 3;

			break;
			// kurian güncellemesi 16.12.2023 end

		default:
			TRACE("[SID=%d] StateChange: %s tripped (bType=%d, buff=%d, nBuff=%d) somehow, HOW!?\n", GetSocketID(), GetName().c_str(), bType, buff, nBuff);
			printf("[SID=%d] StateChange: %s tripped (bType=%d, buff=%d, nBuff=%d) somehow, HOW!?\n", GetSocketID(), GetName().c_str(), bType, buff, nBuff);
			break;
		}
		break;

	case 5:
		if ((buff == 0 && m_bAbnormalType == 0) || (buff == 1 && m_bAbnormalType == 1) || (!isGM())) return;
		break;

	case 7: // invisibility flag, we don't want users overriding server behaviour.
		return;

	default:
		TRACE("[SID=%d] StateChange: %s tripped (bType=%d, buff=%d, nBuff=%d) somehow, HOW!?\n", GetSocketID(), GetName().c_str(), bType, buff, nBuff);
		printf("[SID=%d] StateChange: %s tripped (bType=%d, buff=%d, nBuff=%d) somehow, HOW!?\n", GetSocketID(), GetName().c_str(), bType, buff, nBuff);
		return;
	}

	StateChangeServerDirect(bType, nBuff);
}

/**
* @brief	Changes a player's state directly from the server
* 			without any checks.
*
* @param	bType	State type.
* @param	nBuff	The buff/flag (depending on the state type).
*/
void CUser::StateChangeServerDirect(uint8 bType, uint32 nBuff)
{
	uint8 buff = *(uint8*)&nBuff; // don't ask
	switch (bType)
	{
	case 1:
		m_bResHpType = buff;
		break;

	case 2:
		m_bNeedParty = buff;
		break;

	case 3:
		m_nOldAbnormalType = m_bAbnormalType;

		// If we're a GM, we need to show ourselves before transforming.
		// Otherwise the visibility state is completely desynced.
		if (isGM())
			StateChangeServerDirect(5, 1);

		m_bAbnormalType = nBuff;
		break;
	case 4:
		break;
	case 5:
		if (!isGM())
			return;

		m_bAbnormalType = nBuff;
		nBuff == 0 ? GmInOut(INOUT_OUT) : GmInOut(INOUT_IN);
		break;

	case 6:
		m_bPartyLeader = nBuff; // we don't set this here.
		break;

	case 7:
		UpdateVisibility((InvisibilityType)buff);
		break;

	case 8: // beginner quest
		break;

	case 11:
		if (buff == (uint8)TeamColour::TeamColourRed)
			m_teamColour = TeamColour::TeamColourRed;
		else if (buff == (uint8)TeamColour::TeamColourBlue)
			m_teamColour = TeamColour::TeamColourBlue;
		else
			m_teamColour = TeamColour::TeamColourNone;
		break;
	case 12:
		break;
	default:
		printf("StateChangeServerDirect unhandled buff %d \n", bType);
		break;
	}

	Packet result(WIZ_STATE_CHANGE);
	result << GetSocketID() << bType << nBuff;

	if (GetEventRoom() > 0)
		SendToRegion(&result, nullptr, GetEventRoom());
	else
		SendToRegion(&result);
}

#pragma region CUser::LoyaltyChange(int16 tid, uint16 bonusNP /*= 0*/)
/**
* @brief	Takes a target's loyalty points (NP)
* 			and rewards some/all to the killer (current user).
*
* @param	tid		The target's ID.
* @param	bonusNP Bonus NP to be awarded to the killer as-is.
*/
void CUser::LoyaltyChange(Unit* pdying, uint16 bonusNP /*= 0*/)
{
	short loyalty_source = 0, loyalty_target = 0;
	if (GetMap() == nullptr)
		return;

	if (pdying == nullptr
		|| (pdying->isPlayer() && !TO_USER(pdying)->isInGame()))
		return;

	if (GetZoneID() == ZONE_DELOS)
	{
		if (pdying->GetNation() == GetNation() && !GetMap()->canAttackSameNation())
			return;
		else if (pdying->GetNation() != GetNation() && !GetMap()->canAttackOtherNation())
			return;
	}
	else
	{
		// TODO: Rewrite this out, it shouldn't handle all cases so generally like this
		if ((!GetMap()->isNationPVPZone())
			|| GetZoneID() == ZONE_DESPERATION_ABYSS
			|| GetZoneID() == ZONE_HELL_ABYSS
			|| GetZoneID() == ZONE_DRAGON_CAVE
			|| GetZoneID() == ZONE_CAITHAROS_ARENA)
			return;
	}

	if ((pdying->GetZoneID() == ZONE_DELOS || GetZoneID() == ZONE_DELOS)
		&& (!g_pMain->isCswWarActive() || (pdying->isPlayer() && (!TO_USER(pdying)->isInClan() || TO_USER(pdying)->isInAutoClan())) || !isInClan() || isInAutoClan()))
		return;

	bool give_loyalty = (pdying->GetNation() != GetNation())
		|| (pdying->GetZoneID() == ZONE_DELOS && GetZoneID() == ZONE_DELOS
			&& g_pMain->isCswWarActive() && (pdying->isPlayer() && TO_USER(pdying)->isInClan()) && isInClan());

	if (!give_loyalty)
		return;

	if (give_loyalty)
	{
		uint32 recvLoyalty = 0;
		if (pdying->isPlayer())
			recvLoyalty = TO_USER(pdying)->GetLoyalty();
		else
			recvLoyalty = TO_BOT(pdying)->GetLoyalty();

		if (recvLoyalty == 0)
			loyalty_source = loyalty_target = 0;

		// Ardream
		else if (pdying->GetZoneID() == ZONE_ARDREAM)
		{
			loyalty_source = g_pMain->m_Loyalty_Ardream_Source;
			loyalty_target = g_pMain->m_Loyalty_Ardream_Target;
		}
		// Ronark Land Base
		else if (pdying->GetZoneID() == ZONE_RONARK_LAND_BASE)
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Base_Source;
			loyalty_target = g_pMain->m_Loyalty_Ronark_Land_Base_Target;
		}
		else if (pdying->GetZoneID() == ZONE_RONARK_LAND)
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Source;
			loyalty_target = g_pMain->m_Loyalty_Ronark_Land_Target;
		}
		else if (pdying->GetZoneID() == ZONE_KARUS || pdying->GetZoneID() == ZONE_ELMORAD
			|| (pdying->GetZoneID() >= ZONE_BATTLE && pdying->GetZoneID() <= ZONE_BATTLE6))
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Source;
			loyalty_target = g_pMain->m_Loyalty_Other_Zone_Target;
		}
		// Other zones
		else
		{
			loyalty_source = g_pMain->m_Loyalty_Other_Zone_Source;
			loyalty_target = g_pMain->m_Loyalty_Other_Zone_Target;
		}
	}

	// Include any bonus NP (e.g. rival NP bonus)
	loyalty_source += bonusNP;
	loyalty_target -= bonusNP;

	if (loyalty_source != 0)
		SendLoyaltyChange("Loyalty Change Function", loyalty_source, true, false, true/*pTUser->GetMonthlyLoyalty() > 0 ? true : false*/);

	if (loyalty_target != 0)
	{
		if (pdying->isPlayer())
			TO_USER(pdying)->SendLoyaltyChange("Loyalty Change Function", loyalty_target, true, false, true/* pTUser->GetMonthlyLoyalty() > 0 ? true : false*/);
		else
			TO_BOT(pdying)->SendLoyaltyChange(loyalty_target, true, false, true/* pTUser->GetMonthlyLoyalty() > 0 ? true : false*/);
	}

	// TODO: Move this to a better place (death handler, preferrably)
	// If a war's running, and we died/killed in a war zone... (this method should NOT be so tied up in specifics( 
	if (g_pMain->isWarOpen() && GetMap()->isWarZone())
	{
		if (pdying->GetNation() == (uint8)Nation::KARUS)
			g_pMain->m_sKarusDead++;
		else
			g_pMain->m_sElmoradDead++;
	}
}
#pragma endregion

void CUser::SpeedHackUser()
{
	if (!isInGame() || isGM())
		return;

	int16 nMaxSpeed = 45;
	if (GetFame() == COMMAND_CAPTAIN || isRogue() || GetZoneID() == ZONE_CHAOS_DUNGEON || GetZoneID() == ZONE_DUNGEON_DEFENCE)
		nMaxSpeed = 90;
	else if (isWarrior() || isMage() || isPriest() || isPortuKurian())
		nMaxSpeed = 67;
	if (m_sSpeed > nMaxSpeed || m_sSpeed < -nMaxSpeed)
		goDisconnect("Couldn't pass speed control. speedhack check", __FUNCTION__);
}

bool CUser::CheckInvisibility(CUser* ptarget)
{
	return true;

	if (!ptarget || !ptarget->m_bInvisibilityType || isGM())
		return true;

	if (!GetMap() || !GetMap()->GetZoneType())
		return true;

	if (GetNation() == ptarget->GetNation())
		return true;

	Guard lock(m_buff9lock);
	return m_type9BuffMap.find(4) != m_type9BuffMap.end();
}

void CUser::UserLookChange(int pos, int itemid, int durability)
{
	if (pos >= SLOT_MAX) // let's leave it at this for the moment, the updated check needs considerable reworking
		return;

	auto pItem = g_pMain->GetItemPtr(itemid);
	if (!pItem.isnull() && (pItem.m_bKind == 91 || pItem.m_bKind == 92 || pItem.m_bKind == 93 || pItem.m_bKind == 94))
		return;

	Packet result(WIZ_USERLOOK_CHANGE);
	result << GetSocketID() << uint8(pos) << itemid << uint16(durability);
	SendToRegion(&result, this, GetEventRoom());
}

void CUser::SendNotice()
{
	Packet result(WIZ_NOTICE);
	uint8 count = 0;

	result << uint8(2); // new-style notices (top-right of screen)
	result << count; // placeholder the count

	// Use first line for header, 2nd line for data, 3rd line for header... etc.
	// It's most likely what they do officially (as usual, | is their line separator)
	for (int i = 0; i < 10; i += 2)
		AppendNoticeEntry(result, count, g_pMain->m_ppNotice[i + 1], g_pMain->m_ppNotice[i]);

	AppendExtraNoticeData(result, count);
	result.put(1, count); // replace the placeholdered line count

	Send(&result);
}
#pragma region CUser::SendCapeBonusNotice()
void CUser::SendCapeBonusNotice()
{
	CKnights* pknight = g_pMain->GetClanPtr(GetClanID());
	if (!pknight)
		return;

	auto* pCape = g_pMain->m_KnightsCapeArray.GetData(pknight->m_castCapeID);
	if (!pCape || !pCape->BonusType)
		return;

	if (!isInClan()) return;

	if (pknight->isCastellanCape() && pCape->BonusType)
	{
		_CAPE_CASTELLAN_BONUS* pBonus = g_pMain->m_CapeCastellanBonus.GetData(pCape->BonusType);
		if (pBonus)
		{
			Packet notice(WIZ_NOTICE);
			notice.DByte();
			notice << uint8(4) << uint8(1) << "Cape Bonus" << GenerateBonusNotice(pBonus);
			Send(&notice);
		}
	}
}

void CUser::TopSendNotice()
{
	Packet result(WIZ_NOTICE);
	uint8 count = 0;//uint8

	result << uint8(1); // Old-style notices (top-right of screen)
	result << count; // placeholder the 
	result.SByte();
	// Use first line for header, 2nd line for data, 3rd line for header... etc.
	// It's most likely what they do officially (as usual, | is their line separator)
	for (int i = 0; i < 20; i++)
		AppendNoticeEntryOld(result, count, g_pMain->m_peNotice[i]);

	AppendExtraNoticeData(result, count);
	result.put(1, count); // replace the placeholdered line count
	Send(&result);
}
void CUser::AppendNoticeEntryOld(Packet& pkt, uint8& elementCount, const char* message)
{
	if (message == nullptr || *message == '\0')
		return;

	pkt << message;
	elementCount++;
}
void CUser::AppendNoticeEntry(Packet& pkt, uint8& elementCount, const char* message, const char* title)
{
	if (message == nullptr || *message == '\0'
		|| title == nullptr || *title == '\0')
		return;

	pkt << title << message;
	elementCount++;
}

void CUser::AppendExtraNoticeData(Packet& pkt, uint8& elementCount)
{
	string message;

	if (g_pMain->m_byExpEventAmount > 0)
	{
		g_pMain->GetServerResource(IDS_EXP_REPAY_EVENT, &message, g_pMain->m_byExpEventAmount);
		AppendNoticeEntry(pkt, elementCount, message.c_str(), "Exp Event");
	}

	if (g_pMain->m_byCoinEventAmount > 0)
	{
		g_pMain->GetServerResource(IDS_MONEY_REPAY_EVENT, &message, g_pMain->m_byCoinEventAmount);
		AppendNoticeEntry(pkt, elementCount, message.c_str(), "Noah Event");
	}

	if (g_pMain->m_byNPEventAmount > 0)
	{
		g_pMain->GetServerResource(IDS_NP_REPAY_EVENT, &message, g_pMain->m_byNPEventAmount);
		AppendNoticeEntry(pkt, elementCount, message.c_str(), "Np Event");
	}

	if (g_pMain->m_byDropEventAmount > 0)
	{
		g_pMain->GetServerResource(IDS_DROP_REPAY_EVENT, &message, g_pMain->m_byDropEventAmount);
		AppendNoticeEntry(pkt, elementCount, message.c_str(), "Drop Event");
	}
}

void CUser::UpdateGameWeather(Packet& pkt)
{
	if (!isGM())	// is this user a GM?
		return;

	if (pkt.GetOpcode() == WIZ_WEATHER)
	{
		pkt >> g_pMain->m_byWeather >> g_pMain->m_sWeatherAmount;
	}
	else
	{
		uint16 y, m, d;
		pkt >> y >> m >> d >> g_pMain->m_sHour >> g_pMain->m_sMin;
	}
	Send(&pkt); // pass the packet straight on
}

void CUser::CountConcurrentUser()
{
	if (!isGM())
		return;

	uint16 count = 0;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		count++;
	}

	Guard lock(g_pMain->m_BotcharacterNameLock);
	count += (uint16)g_pMain->m_BotcharacterNameMap.size();

	Packet result(WIZ_CONCURRENTUSER);
	result << count;
	Send(&result);
}
void CUser::GiveKillReward()
{
	foreach(itr, g_pMain->m_ZoneKillReward)
	{
		_ZONE_KILL_REWARD* pReward = itr->second;
		uint32 sRater = myrand(1, 10000);

		if (sRater > itr->second->sRate && itr->second->sRate != 10000)
			continue;

		if (itr->second->ZoneID != GetZoneID())
			continue;

		if (itr->second->Status == 0)
			continue;

		if (m_KillCount % itr->second->KillCount != 0)
			continue;

		if (pReward->Party == 1 && !isInParty()) // Sadece Party
			continue;

		if (pReward->Party == 0 && isInParty()) // SOLO
			continue;

		bool self = true;
		if (isInParty() && pReward->isPartyItem)
		{
			_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
			if (pParty == nullptr)
				return;

			for (int i = 0; i < MAX_PARTY_USERS; i++)
			{
				CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (pUser == nullptr || !pUser->isInGame())
					continue;

				if (pUser->GetZoneID() != GetZoneID() || pUser->GetEventRoom() != GetEventRoom() || pUser->GetPartyID() != GetPartyID())
					continue;

				if (pReward->isBank)
					pUser->GiveWerehouseItem(pReward->ItemID, pReward->sCount, false, false, pReward->Time);
				else if (pReward->ItemID == ITEM_GOLD)
					pUser->GoldGain(pReward->sCount);
				else
					pUser->GiveItem("", pReward->ItemID, pReward->sCount, true, pReward->Time);
			}
			self = false;
		}
		else if (isInParty() && !pReward->isPartyItem && !isPriest())
		{
			uint32 rand = myrand(0, 10000);
			if (uint32(pReward->priestrate * 100) > rand)
			{
				_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
				if (pParty == nullptr)
					return;

				for (int i = 0; i < MAX_PARTY_USERS; i++)
				{
					CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
					if (pUser == nullptr || !pUser->isInGame() || !pUser->isPriest())
						continue;

					if (pUser->GetZoneID() != GetZoneID() || pUser->GetEventRoom() != GetEventRoom() || pUser->GetPartyID() != GetPartyID())
						continue;

					if (pReward->isBank)
						pUser->GiveWerehouseItem(pReward->ItemID, pReward->sCount, false, false, pReward->Time);
					else if (pReward->ItemID == ITEM_GOLD)
						pUser->GoldGain(pReward->sCount);
					else
						pUser->GiveItem("", pReward->ItemID, pReward->sCount, true, pReward->Time);

				}
			}
		}

		if (self)
		{
			if (pReward->isBank)
				GiveWerehouseItem(pReward->ItemID, pReward->sCount, false, false, pReward->Time);
			else if (pReward->ItemID == ITEM_GOLD)
				GoldGain(pReward->sCount);
			else
				GiveItem("", pReward->ItemID, pReward->sCount, true, pReward->Time);
		}

	}
}
void CUser::SendNewDeathNotice(Unit* pKiller)
{
	if (pKiller == nullptr || !pKiller->isPlayer() || !TO_USER(pKiller)->isInGame())
		return;

	float fRange = 0.0f;
	uint8 killtype = 3;
	std::string color = "#f08080";

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr)
			continue;

		if (pUser->GetZoneID() != GetZoneID() || pUser->GetEventRoom() != GetEventRoom())
			continue;

		if (TO_USER(pKiller)->GetID() == pUser->GetID())
			killtype = 1;
		else if (GetID() == pUser->GetID())
			killtype = 1;
		else if (TO_USER(pKiller)->isInParty() && pUser->GetPartyID() == TO_USER(pKiller)->GetPartyID())
			killtype = 2;
		else
			killtype = 3;


		//if (killtype == 1) //mor
		//	color = "#A020F0";
		//else if (killtype == 2)//mavi partide biri kesince
		//	color = "#0096FF";
		//else if (killtype == 3)//sarı başkalarının kestiğini gördüğümüz
		//	color = "#FFFF00";
		//else
		//	color = "#FFFF00";

		//Packet result(WIZ_CHAT, uint8(ChatType::DEATH_NOTICE));
		//result.SByte();
		//result << GetNation()
		//	<< uint8(DeathNoticeType::DeathNoticeCoordinates)
		//	<< pKiller->GetID() // session ID?
		//	<< std::string(string_format("<font color=@%s@>%s", color.c_str(),pKiller->GetName().c_str()))
		//	<< GetID() // session ID?
		//	<< GetName()
		//	<< uint16(GetX()) << uint16(GetZ());
		//pUser->Send(&result);

		Packet result(XSafe, uint8(XSafeOpCodes::DeathNotice));
		result.SByte();
		result << killtype << TO_USER(pKiller)->GetName().c_str() << GetName().c_str() << uint16(GetX()) << uint16(GetZ());
		pUser->Send(&result);
	}
}

void CUser::ItemWoreOut(int type, int damage)
{
	static uint8 weaponsTypes[] = { RIGHTHAND, LEFTHAND };
	static uint8 armourTypes[] = { HEAD, BREAST, LEG, GLOVE, FOOT };
	static uint8 repairTypes[] = { RIGHTHAND, LEFTHAND, HEAD, BREAST, LEG, GLOVE, FOOT };
	int8 totalSlots = -1;
	int worerate;

	if (type == ACID_ALL
		|| type == UTC_ATTACK
		|| type == UTC_DEFENCE)
		worerate = damage;
	else
		worerate = myrand(2, 5);

	if (worerate == 0) return;

	if (type != ATTACK
		&& type != DEFENCE
		&& type != REPAIR_ALL
		&& type != ACID_ALL
		&& type != UTC_ATTACK
		&& type != UTC_DEFENCE)
		return;

	if (type == ATTACK)
		totalSlots = sizeof(weaponsTypes) / sizeof(*weaponsTypes);
	else if (type == DEFENCE)
		totalSlots = sizeof(armourTypes) / sizeof(*armourTypes);
	else if (type == REPAIR_ALL)
		totalSlots = sizeof(repairTypes) / sizeof(*repairTypes);
	else if (type == ACID_ALL)
		totalSlots = sizeof(armourTypes) / sizeof(*armourTypes);
	else if (type == UTC_ATTACK)
		totalSlots = sizeof(weaponsTypes) / sizeof(*weaponsTypes);
	else if (type == UTC_DEFENCE)
		totalSlots = sizeof(armourTypes) / sizeof(*armourTypes);

	if (totalSlots == -1)
		return;

	for (uint8 i = 0; i < totalSlots; i++)
	{
		uint8 slot;
		if (type == ATTACK)
			slot = weaponsTypes[i];
		else if (type == DEFENCE)
			slot = armourTypes[i];
		else if (type == REPAIR_ALL)
			slot = repairTypes[i];
		else if (type == UTC_ATTACK)
			slot = weaponsTypes[i];
		else if (type == UTC_DEFENCE)
			slot = armourTypes[i];
		else
			slot = armourTypes[i];

		_ITEM_DATA* pItem = GetItem(slot);
		if (!pItem || pItem->nNum == 0)
			continue;

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(pItem->nNum);
		if (pTable.isnull())
			continue;

		// Is a non-broken item equipped?
		if (pItem == nullptr
			|| (pItem->sDuration <= 0 && type != REPAIR_ALL)
			// Does the item exist?
			// If it's in the left or righthand slot, is it a shield? (this doesn't apply to weapons)
			|| ((type == ATTACK
				|| type == UTC_ATTACK)
				&& ((slot == LEFTHAND
					|| slot == RIGHTHAND)
					&& pTable.m_bSlot == ItemSlot1HLeftHand)))
			continue;

		if (type == REPAIR_ALL)
		{
			pItem->sDuration = pTable.m_sDuration;
			SendDurability(slot, pItem->sDuration);
			UserLookChange(slot, pItem->nNum, pItem->sDuration);
			SetUserAbility(true);
			continue;
		}

		int beforepercent = (int)((pItem->sDuration / (double)pTable.m_sDuration) * 100);
		int curpercent;

		if (worerate > pItem->sDuration)
			pItem->sDuration = 0;
		else
			pItem->sDuration -= worerate;

		if (m_sItemArray[slot].sDuration <= 0)
			m_sItemArray[slot].sDuration = 0;

		if (m_sItemArray[slot].sDuration == 0)
		{
			SendDurability(slot, 0);
			SetUserAbility(false);
			SendItemMove(1, 1);
			continue;
		}

		SendDurability(slot, pItem->sDuration);

		curpercent = (int)((pItem->sDuration / (double)pTable.m_sDuration) * 100);

		if ((curpercent / 5) != (beforepercent / 5))
		{
			if (curpercent >= 65 && curpercent < 70
				|| curpercent >= 25 && curpercent < 30)
				UserLookChange(slot, pItem->nNum, pItem->sDuration);
		}
	}
}

void CUser::SendDurability(uint8 slot, uint16 durability)
{
	if (!isInGame())
		return;

	Packet result(WIZ_DURATION, slot);
	result << durability;
	Send(&result);
}

void CUser::SendItemMove(uint8 command, uint8 subcommand)
{
	if (!isInGame())
		return;

	Packet result;
	result.clear();
	result.Initialize(WIZ_ITEM_MOVE);
	result << command << subcommand;

	if (m_bAttackAmount == 0) m_bAttackAmount = 100;

	// If the subcommand is not error, send the stats.
	if (subcommand != 0)
	{
		uint32 moneyReq = 0;
		if (GetPremium() == 12)
			moneyReq = 0;
		else
		{
			moneyReq = (int)pow((GetLevel() * 2.0f), 3.4f);
			if (GetLevel() < 30)
				moneyReq = (int)(moneyReq * 0.4f);
			else if (GetLevel() >= 60)
				moneyReq = (int)(moneyReq * 1.5f);

			if ((g_pMain->m_sDiscount == 1
				&& g_pMain->m_byOldVictory == GetNation())
				|| g_pMain->m_sDiscount == 2)
				moneyReq /= 2;
		}
		uint16 TotalAc = (m_sTotalAc + m_sACAmount) - m_sACSourAmount;
		result << uint16(m_sTotalHit * m_bAttackAmount / 100)
			<< TotalAc
			//<< m_sMaxWeight << uint8(1) << uint16(m_MaxHp) << uint16(m_MaxMp)
			<< m_sMaxWeight << uint8(0) << uint8(0) << uint16(m_MaxHp) << uint16(m_MaxMp) //xx
			//<< m_sMaxWeight << uint8(0) << uint16(m_MaxHp) << uint16(m_MaxMp)// 13.11.2020 Target seçiliyken 25 metre mesafe dışına cıkınca skill to far hatası fix artık minnör atıyor
			<< GetStatBonusTotal(StatType::STAT_STR) << GetStatBonusTotal(StatType::STAT_STA)
			<< GetStatBonusTotal(StatType::STAT_DEX) << GetStatBonusTotal(StatType::STAT_INT) << GetStatBonusTotal(StatType::STAT_CHA)
			<< uint16(((m_sFireR + m_bAddFireR + m_bResistanceBonus) * m_bPctFireR / 100))
			<< uint16(((m_sColdR + m_bAddColdR + m_bResistanceBonus) * m_bPctColdR / 100))
			<< uint16(((m_sLightningR + m_bAddLightningR + m_bResistanceBonus) * m_bPctLightningR / 100))
			<< uint16(((m_sMagicR + m_bAddMagicR + m_bResistanceBonus) * m_bPctMagicR / 100))
			<< uint16(((m_sDiseaseR + m_bAddDiseaseR + m_bResistanceBonus) * m_bPctDiseaseR / 100))
			<< uint16(((m_sPoisonR + m_bAddPoisonR + m_bResistanceBonus) * m_bPctPoisonR / 100))

			<< uint32(m_nKnightCash) << uint16(m_sDaggerR) << uint16(m_sAxeR) << uint16(m_sSwordR)
			<< uint16(m_sClubR) << uint16(m_sSpearR) << uint16(m_sBowR) << uint16(m_sJamadarR) << uint32(moneyReq) << m_sHp << m_sMp;
	}
	Send(&result);



	if (isInParty())
		SendPartyHpManager(PartyType::Send_All); //17.12.2020 Partyde HP Degeri Yazi Olarak Gosterme
}

void CUser::SendAllKnightsID()
{
	Packet result(WIZ_KNIGHTS_LIST, uint8(1));
	uint16 count = 0;

	foreach_stlmap(itr, g_pMain->m_KnightsArray)
	{
		CKnights* pKnights = itr->second;
		if (pKnights == nullptr)
			continue;
		result << pKnights->m_sIndex << pKnights->m_strName;
		count++;
	}

	result.put(0, count);
	SendCompressed(&result);
}

int CUser::FindSlotForItem(uint32 nItemID, uint16 sCount /*= 1*/)
{
	int result = -1;
	_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
	if (pTable.isnull())
		return result;

	_ITEM_DATA* pItem = nullptr;
	// If the item's stackable, try to find it a home.
	// We could do this in the same logic, but I'd prefer one initial check
	// over the additional logic hit each loop iteration.
	if (pTable.m_bCountable > 0)
	{
		for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
		{
			pItem = GetItem(i);
			if (pItem == nullptr)
				continue;

			if (pItem->nNum == nItemID && pItem->sCount + sCount <= ITEMCOUNT_MAX)
				return i;

			// If it's the item we're after, and there will be room to store it...
			if (pItem->nNum == 0 && result < 0)
				result = i;
		}

		// If we didn't find a slot countaining our stackable item, it's possible we found
		// an empty slot. So return that (or -1 if it none was found; no point searching again).
		return result;
	}

	// If it's not stackable, don't need any additional logic.
	// Just find the first free slot.
	return GetEmptySlot();
}

int CUser::GetEmptySlot()
{
	for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
	{
		_ITEM_DATA* pItem = GetItem(i);
		if (!pItem) continue;
		if (pItem->nNum == 0)return i;
	}

	return -1;
}

int CUser::FindWerehouseSlotForItem(uint32 nItemID, uint16 sCount /*= 1*/)
{
	int result = -1;
	_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
	_ITEM_DATA* pItem = nullptr;

	if (pTable.isnull())
		return result;

	// If the item's stackable, try to find it a home.
	// We could do this in the same logic, but I'd prefer one initial check
	// over the additional logic hit each loop iteration.
	if (pTable.m_bCountable)
	{
		for (int i = 0; i < WAREHOUSE_MAX; i++)
		{
			pItem = GetWerehouseItem(i);

			if (pItem == nullptr)
				continue;

			// If it's the item we're after, and there will be room to store it...
			if (pItem->nNum == nItemID
				&& pItem->sCount + sCount <= ITEMCOUNT_MAX)
				return i;

			// Found a free slot, we'd prefer to stack it though
			// so store the first free slot, and ignore it.
			if (pItem->nNum == 0 && result < 0)
				result = i;
		}

		// If we didn't find a slot countaining our stackable item, it's possible we found
		// an empty slot. So return that (or -1 if it none was found; no point searching again).
		return result;
	}

	// If it's not stackable, don't need any additional logic.
	// Just find the first free slot.
	return GetWerehouseEmptySlot();
}

int CUser::GetWerehouseEmptySlot()
{
	_ITEM_DATA* pItem;

	for (int i = 0; i < WAREHOUSE_MAX; i++)
	{
		pItem = GetWerehouseItem(i);

		if (!pItem)
			continue;

		if (pItem->nNum == 0)
			return i;
	}

	return -1;
}

void CUser::Home(bool check /*=true*/)
{
#define TOWN_TİME 1200

	if (check)
	{
		if (isDead()
			|| isKaul()
			|| GetHealth() < (GetMaxHealth() / 2)
			|| isInEventZone()
			|| hasBuff(BUFF_TYPE_FREEZE)
			|| (UNIXTIME2 - m_TownTime) < TOWN_TİME)
			return;
	}
	// The point where you will be warped to.
	short x = 0, z = 0;
	_OBJECT_EVENT* pEvent = nullptr;
	pEvent = GetMap()->GetObjectEvent(m_sBind);
	// Resurrect at a bind/respawn point
	if (pEvent && pEvent->byLife == 1 && GetZoneID() != ZONE_DELOS && pEvent->sIndex != 101 && pEvent->sIndex != 201)
	{
		SetPosition(pEvent->fPosX + x, 0.0f, pEvent->fPosZ + z);
		x = short(pEvent->fPosX);
		z = short(pEvent->fPosZ);
		Warp(x * 10, z * 10);
		return;
	}

	if (isInSoccerEvent())
	{
		isEventSoccerEnd();

		if (g_pMain->m_TempleSoccerEventUserArray.GetData(GetSocketID()) != nullptr)
			g_pMain->m_TempleSoccerEventUserArray.DeleteData(GetSocketID());
	}

	// Forgotten Temple
	if (GetZoneID() == ZONE_FORGOTTEN_TEMPLE)
	{
		KickOutZoneUser(true);
		return;
	}
	// Prevent /town'ing in quest arenas
	else if ((GetZoneID() / 10) == 5
		|| !GetStartPosition(x, z))
		return;

	/*if (GetZoneID() == ZONE_DELOS)
	{
		if (g_pMain->CastleSiegeDeatchmatchOpened == true && GetZoneID() == ZONE_DELOS)
		{
			_DEATHMATCH_BARRACK_INFO* pData4 = g_pMain->m_KnightSiegeWarBarrackList.GetData(GetClanID());
			if (pData4 == nullptr)
				return;

			_DEATHMATCH_WAR_INFO* pKnightList = g_pMain->m_KnightSiegeWarClanList.GetData(GetClanID());
			if (pKnightList == nullptr)
				return;

			uint8 xrand = myrand(1, 5);
			uint8 zrand = myrand(1, 5);

			Warp((pData4->m_tBaseX + xrand) * 10, (pData4->m_tBaseZ + zrand) * 10);
			return;
		}
	}*/
	if (g_pMain->isBowlEventActive && GetZoneID() == g_pMain->tBowlEventZone)
		GetStartPositionRandom(x, z, GetZoneID());
	m_TownTime = UNIXTIME2;
	Warp(x * 10, z * 10);
	PetHome(x * 10, 0, z * 10);

	// JstKO User Town Effect Eklendi 01.01.2025
	if (g_pMain->UserTownEffect)
		ShowEffect(300596);
}

bool CUser::GetStartPosition(short& x, short& z, uint8 bZone /*= 0 */, bool isCind)
{
	// Get start position data for current zone (unless we specified a zone).
	int nZoneID = (bZone == 0 ? GetZoneID() : bZone);
	_START_POSITION* pData = g_pMain->GetStartPosition(nZoneID);
	if (pData == nullptr)
		return false;

	if (GetZoneID() == ZONE_DELOS)
	{
		short x_1 = 481, z_1 = 255, x_2 = 507, z_2 = 255, x_3 = 532, z_3 = 255;
		if (g_pMain->pSiegeWar.sMasterKnights == GetClanID())
		{
			x = 505 + myrand(0, pData->bRangeX);
			z = 840 + myrand(0, pData->bRangeZ);
		}
		else
		{
			x = 507 + myrand(-5, 20);
			z = 256 + myrand(-5, 20);

			/*uint32 rand = myrand(1, 3);
			if (rand == 1) {
				x = x_1 + myrand(0, 5);
				z = z_1 + myrand(0, 5);
			}
			else if (rand == 2) {
				x = x_2 + myrand(0, 5);
				z = z_2 + myrand(0, 5);
			}
			else if (rand == 3) {
				x = x_3 + myrand(0, 5);
				z = z_3 + myrand(0, 5);
			}
			else {
				x = x_2 + myrand(0, 5);
				z = z_2 + myrand(0, 5);
			}*/
		}
	}
	else
	{
		uint8 mNation = GetNation();
		if (isCind && (pCindWar.m_eNation == 1 || pCindWar.m_eNation == 2))
			mNation = pCindWar.m_eNation;

		if (mNation == (uint8)Nation::KARUS)
		{
			x = pData->sKarusX + myrand(0, pData->bRangeX);
			z = pData->sKarusZ + myrand(0, pData->bRangeZ);
		}
		else
		{
			x = pData->sElmoradX + myrand(0, pData->bRangeX);
			z = pData->sElmoradZ + myrand(0, pData->bRangeZ);
		}
	}

	return true;
}

bool CUser::GetStartPositionRandom(short& x, short& z, uint8 bZone)
{
	int nRandom = myrand(0, g_pMain->m_StartPositionRandomArray.GetSize() - 1);
	goto GetPosition;

GetPosition:
	{
		if (g_pMain->m_StartPositionRandomArray.GetData(nRandom)->ZoneID == (bZone == 0 ? GetZoneID() : bZone))
		{
			x = g_pMain->m_StartPositionRandomArray.GetData(nRandom)->PosX + myrand(0, g_pMain->m_StartPositionRandomArray.GetData(nRandom)->Radius);
			z = g_pMain->m_StartPositionRandomArray.GetData(nRandom)->PosZ + myrand(0, g_pMain->m_StartPositionRandomArray.GetData(nRandom)->Radius);
			return true;
		}

		nRandom = myrand(0, g_pMain->m_StartPositionRandomArray.GetSize() - 1);
		goto GetPosition;
	}

	return GetStartPosition(x, z);
}

void CUser::ResetWindows()
{
	if (isTrading())
		ExchangeCancel();

	if (m_bRequestingChallenge)
		HandleChallengeCancelled(m_bRequestingChallenge);

	if (m_bChallengeRequested)
		HandleChallengeRejected(m_bChallengeRequested);

	// If we're a vendor, close the stall
	if (isMerchanting() || isSellingMerchantingPreparing())  // Karakter pazar kurarken oyunu kapatırsa item kaybolması fixlendi.
		MerchantClose();

	if (isBuyingMerchant())
		BuyingMerchantClose();

	// If we're just browsing, free up our spot so others can browse the vendor.
	if (m_sMerchantsSocketID >= 0)
		CancelMerchant();

	if (isMining())
		HandleMiningStop();

	if (isFishing())
		HandleFishingStop((Packet)(WIZ_MINING, FishingStop));

	/*if (isGenieActive())
		GenieStop();*/

	if (m_bCheckWarpZoneChange)
		m_bCheckWarpZoneChange = false;
}

CUser* CUser::GetItemRoutingUser(uint32 nItemID, uint16 sCount)
{
	if (!isInParty())
		return this;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr || pParty->bItemRouting >= MAX_PARTY_USERS)
		return nullptr;

	_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
	if (pTable.isnull())
		return nullptr;

	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(pParty->uid[pParty->bItemRouting]);

		if (pParty->bItemRouting > 6)
			pParty->bItemRouting = 0;
		else
			pParty->bItemRouting++;

		if (pUser == nullptr) continue;

		if (!pUser->isInRange(GetX(), GetZ(), RANGE_50M)
			|| pUser->isDead()
			|| !pUser->CheckWeight(pTable, nItemID, sCount))
			continue;

		return pUser;
	}

	return nullptr;
}

void CUser::SendAnvilRequest(uint16 sNpcID, uint8 bType /*= ITEM_UPGRADE_REQ*/)
{
	Packet result(WIZ_ITEM_UPGRADE, uint8(bType));
	result << sNpcID;
	Send(&result);
}

void CUser::UpdateVisibility(InvisibilityType bNewType)
{
	m_bInvisibilityType = (uint8)(bNewType);
}

/**
* @brief	Forces a reset of the GM's persistent visibility
* 			state.
*/
void CUser::ResetGMVisibility()
{
	if (!isGM()/* || isTransformed()*/)
		return;

	if (m_bAbnormalType != ABNORMAL_NORMAL)
	{
		Packet result(WIZ_STATE_CHANGE);
		result << GetID() << uint8(5) << uint8(0);
		Send(&result);
		m_bAbnormalType = ABNORMAL_INVISIBLE;
	}
}

void CUser::BlinkStart(int exBlinkTime)
{
	if (GetMap() == nullptr || isGM() || isTransformed()) return;

	if (GetMap()->isWarZone() || GetMap()->m_bBlinkZone != 1)
	{
		if (m_bAbnormalType == (uint32)AbnormalType::ABNORMAL_BLINKING)
		{
			m_bRegeneType = REGENE_NORMAL;
			m_bCanUseSkills = true;

			if (GetZoneID() == ZONE_CHAOS_DUNGEON || GetZoneID() == ZONE_DUNGEON_DEFENCE) StateChangeServerDirect(3, (uint32)AbnormalType::ABNORMAL_CHAOS_NORMAL);
			else StateChangeServerDirect(3, (uint32)AbnormalType::ABNORMAL_NORMAL);
			UpdateVisibility(InvisibilityType::INVIS_NONE);
		}
		return;
	}

	m_bAbnormalType = (uint32)AbnormalType::ABNORMAL_BLINKING;
	m_tBlinkExpiryTime = UNIXTIME + BLINK_TIME + exBlinkTime;
	m_bRegeneType = REGENE_ZONECHANGE;
	m_bCanUseSkills = false;
	UpdateVisibility(InvisibilityType::INVIS_DISPEL_ON_ATTACK); // AI shouldn't see us
	m_bInvisibilityType = (uint8)InvisibilityType::INVIS_NONE; // but players should. 
	StateChangeServerDirect(3, (uint32)AbnormalType::ABNORMAL_BLINKING);
}

void CUser::BlinkTimeCheck()
{
	if (UNIXTIME < m_tBlinkExpiryTime) return;
	else if (UNIXTIME > m_tBlinkExpiryTime && !m_bCanUseSkills) m_bCanUseSkills = !m_bCanUseSkills;
	m_bRegeneType = REGENE_NORMAL;
	m_bCanUseSkills = true;

	if (GetZoneID() == ZONE_CHAOS_DUNGEON || GetZoneID() == ZONE_DUNGEON_DEFENCE /*|| GetZoneID() == ZONE_KNIGHT_ROYALE*/)
		StateChangeServerDirect(3, (uint32)AbnormalType::ABNORMAL_CHAOS_NORMAL);
	else StateChangeServerDirect(3, (uint32)AbnormalType::ABNORMAL_NORMAL);

	UpdateVisibility(InvisibilityType::INVIS_NONE);
}

void CUser::TrapProcess()
{
	// If the time interval has passed
	if ((UNIXTIME - m_tLastTrapAreaTime) >= ZONE_TRAP_INTERVAL)
	{
		if (GetZoneID() == ZONE_BIFROST)
			SendUserStatusUpdate(UserStatus::USER_STATUS_BLIND, UserStatusBehaviour::USER_STATUS_INFLICT);

		HpChange(-ZONE_TRAP_DAMAGE, this);
		m_tLastTrapAreaTime = UNIXTIME;
	}
}

void CUser::KickOutZoneUser(bool home, uint8 nZoneID)
{
	if (home)
	{
		C3DMap* pMap = g_pMain->GetZoneByID(nZoneID);
		if (pMap == nullptr)
			return;

		int eventID = 0;
		int random = myrand(0, 9000);
		if (random >= 0 && random < 3000)			eventID = 0;
		else if (random >= 3000 && random < 6000)	eventID = 1;
		else if (random >= 6000 && random < 9001)	eventID = 2;

		_REGENE_EVENT* pRegene = pMap->GetRegeneEvent(eventID);
		if (pRegene == nullptr)
		{
			KickOutZoneUser();
			return;
		}

		ZoneChange(pMap->m_nZoneNumber,
			pRegene->fRegenePosX + (float)myrand(0, (int)pRegene->fRegeneAreaX),
			pRegene->fRegenePosZ + (float)myrand(0, (int)pRegene->fRegeneAreaZ));
		return;
	}

	// Teleport the player to their native zone.
	short x, z;

	if (GetZoneID() == ZONE_DELOS || GetZoneID() == ZONE_DESPERATION_ABYSS || GetZoneID() == ZONE_HELL_ABYSS)
	{
		if (GetStartPosition(x, z, nZoneID))
			ZoneChange(nZoneID, (float)x, (float)z);
	}
	else if (GetStartPosition(x, z, GetNation()))
		ZoneChange(GetNation(), x, z);
	else
	{
		TRACE("### KickOutZoneUser - StartPosition not found : Nation=%d", GetNation());
	}
}

void CUser::NativeZoneReturn()
{
	if (GetLevel() < MIN_LEVEL_ARDREAM)
	{
		_START_POSITION* pStartPosition = g_pMain->m_StartPositionArray.GetData(ZONE_MORADON);
		if (pStartPosition == nullptr)
			return;

		m_bZone = ZONE_MORADON;

		m_curx = (float)((m_bNation == (uint8)Nation::KARUS ? pStartPosition->sKarusX : pStartPosition->sElmoradX) + myrand(0, pStartPosition->bRangeX));
		m_curz = (float)((m_bNation == (uint8)Nation::KARUS ? pStartPosition->sKarusZ : pStartPosition->sElmoradZ) + myrand(0, pStartPosition->bRangeZ));
	}
	else
	{
		_START_POSITION* pStartPosition = g_pMain->m_StartPositionArray.GetData(m_bNation);
		if (pStartPosition == nullptr)
			return;

		m_bZone = m_bNation;

		m_curx = (float)((m_bNation == (uint8)Nation::KARUS ? pStartPosition->sKarusX : pStartPosition->sElmoradX) + myrand(0, pStartPosition->bRangeX));
		m_curz = (float)((m_bNation == (uint8)Nation::KARUS ? pStartPosition->sKarusZ : pStartPosition->sElmoradZ) + myrand(0, pStartPosition->bRangeZ));
	}
}

/**
* @brief	Sends a packet to all players within the
* 			user's current region and surrounding regions
* 			(i.e. visible area)
*
* @param	pkt		   	The packet.
* @param	pExceptUser	User to except. If specified, will ignore this user.
*/
void CUser::SendToRegion(Packet* pkt, CUser* pExceptUser /*= nullptr*/, uint16 nEventRoom /*-1*/)
{
	g_pMain->Send_Region(pkt, GetMap(), GetRegionX(), GetRegionZ(), pExceptUser, nEventRoom);
}

/**
* @brief	Sends a packet to all players within the
* 			user's current zone.
*
* @param	pkt		   	The packet.
* @param	pExceptUser	User to except. If specified, will ignore this user.
*/
void CUser::SendToZone(Packet* pkt, CUser* pExceptUser /*= nullptr*/, uint16 nEventRoom /*-1*/, float fRange)
{
	g_pMain->Send_Zone(pkt, GetZoneID(), pExceptUser, 0, nEventRoom, fRange);
}

void CUser::SendToMerchant(Packet* pkt, CUser* pExceptUser /*= nullptr*/, uint16 nEventRoom /*-1*/, float fRange)
{
	g_pMain->Send_Merchant(pkt, GetZoneID(), pExceptUser, 0, nEventRoom, fRange);
}

// We have no clan handler, we probably won't need to implement it (but we'll see).
void CUser::SendClanUserStatusUpdate(bool bToRegion /*= true*/)
{
	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_MODIFY_FAME));
	result << uint8(1) << GetSocketID()
		<< GetClanID() << GetFame();

	// TODO: Make this region code user-specific to perform faster.
	if (bToRegion)
		SendToRegion(&result);
	else
		Send(&result);
}

void CUser::HandleHelmet(Packet& pkt)
{
	if (isDead())
		return;

	Packet result(WIZ_HELMET);
	m_bIsHidingHelmet = pkt.read<bool>();
	m_bIsHidingCospre = pkt.read<bool>();
	result << m_bIsHidingHelmet << m_bIsHidingCospre << uint32(GetSocketID());

	if (GetEventRoom() > 0)
		SendToRegion(&result, nullptr, GetEventRoom());
	else
		SendToRegion(&result);

	if (m_bIsHidingCospre)
	{
		uint8 equippedItems[] = { BREAST, LEG, GLOVE, FOOT };
		foreach_array(i, equippedItems)
		{
			_ITEM_DATA* pItem = GetItem(equippedItems[i]);

			if (pItem == nullptr)
			{
				UserLookChange(equippedItems[i], 0, 0);
				continue;
			}
			UserLookChange(equippedItems[i], pItem->nNum, pItem->sDuration);
		}
	}
}

bool isDrainSkills(uint32 iskillid /*= 0*/)
{
	if (iskillid == 107650 ||
		iskillid == 108650 ||
		iskillid == 207650 ||
		iskillid == 208650 ||
		iskillid == 107610 ||
		iskillid == 108610 ||
		iskillid == 207610 ||
		iskillid == 208610)
		return true;
	return false;
}

bool Unit::isInAttackRange(Unit* pTarget, _MAGIC_TABLE pSkill /*= nullptr*/)
{
	if (pTarget == nullptr)
		return false;

	if (pTarget == this
		|| !isPlayer())
		return true;

	const float fBaseMeleeRange = 15.0f; // far too generous
	const float fBaseRangedRange = 65.0f;

	float fRange = fBaseMeleeRange, fWeaponRange = 0.0f;

	_ITEM_DATA* pItem = nullptr;

	_ITEM_TABLE pTable = TO_USER(this)->GetItemPrototype(RIGHTHAND, pItem);
	if (!pTable.isnull() && pItem->sDuration > 0)
	{
		fWeaponRange = float(pTable.m_sRange) / 10;
	}
	else
	{
		pTable = TO_USER(this)->GetItemPrototype(LEFTHAND, pItem);
		if (!pTable.isnull() && pItem->sDuration != 0)
			fWeaponRange = float(pTable.m_sRange) / 10;
	}

	if (!pSkill.isnull())
	{
		// For physical attack skills (type 1 - melee, type 2 - ranged), we'll need take into account 
		// the weapon's range.
		if (pSkill.bType[0] != 3)
			fRange = fWeaponRange;

		if (pSkill.iNum == 208575 || pSkill.iNum == 108575)
			fWeaponRange = 0.0f;

		// For physical melee & magic skills, try to use the skill's range if it's set.
		// Need to allow more for lag, and poorly thought out skill ranges.
		// If not, resort to using the weapon range -- or predefined 15m range in the case of type 3 skills.
		if (pSkill.bType[0] != 2)
		{
			uint16 sRange = pSkill.sRange;
			if (isDrainSkills(pSkill.iNum)) sRange += 5;

			if (isPlayer() && TO_USER(this)->m_sSpeed != 0 && pSkill.sUseStanding == 0) // If EventRoom is not same disable Skill.
				return isInRangeSlow(pTarget, fBaseMeleeRange * 1.5f + (sRange == 0 ? fRange : sRange) + (pSkill.bType[0] == 1 ? fWeaponRange : 0));
			else
				return isInRangeSlow(pTarget, fBaseMeleeRange + (sRange == 0 ? fRange : sRange) + (pSkill.bType[0] == 1 ? fWeaponRange : 0));
		}
		// Ranged skills (type 2) don't typically have the main skill range set to anything useful, so
		// we need to allow for the: bow's range, flying skill-specific range, and an extra 50m for the
		// also extremely poorly thought out ranges.
		else
		{
			//_MAGIC_TYPE2 * pType2 = g_pMain->m_Magictype2Array.GetData(pSkill.iNum);
			//return pType2 != nullptr && isInRangeSlow(pTarget, fRange + float(pType2->sAddRange + 5)/* / 10*/ + fBaseRangedRange);
			return isInRangeSlow(pTarget, fRange + float(100 + 5)/* / 10*/ + fBaseRangedRange);
		}
	}

	// Regular attack range.
	if (fWeaponRange != 0.0f)
		fRange = fBaseMeleeRange + fWeaponRange;

	return isInRangeSlow(pTarget, fRange);
}

/**
* @brief	Determine if we can use the specified item
* 			via the magic/skill system.
*
* @param	itemid	The ID of the item.
* @param	count 	Stack (can probably disregard, as it's always 1).
*
* @return	true if we can use item, false if not.
*/
bool CUser::CanUseItem(uint32 nItemID, uint16 sCount /*= 1*/)
{
	_ITEM_TABLE pItem = g_pMain->GetItemPtr(nItemID);
	if (pItem.isnull())
		return false;

	// Debug: Log Monster Stone checks to diagnose skillbar greyed-out issue
	if (nItemID >= 300144000 && nItemID <= 300148000) {
		bool exists = CheckExistItem(nItemID, sCount);
		TRACE("[DEBUG] CanUseItem MonsterStone check: itemid=%u exists=%d isTransformed=%d level=%d-%d\n", nItemID, exists, isTransformed(), pItem.m_bReqLevel, pItem.m_bReqLevelMax);
	}

	// Disable scroll usage while transformed.
	if (isTransformed())
	{
		// Various NPC transformations ("Transform Scrolls") are exempt from this rule -- it's just monsters.
		// Also, siege transformations can use their own buff scrolls.
		if (isNPCTransformation()
			&& isSiegeTransformation())
			return false;
	}

	// If the item's class specific, ensure it can be used by this user.
	if ((pItem.m_bClass != 0 && !JobGroupCheck(pItem.m_bClass))
		// Check the item's level requirement
		|| (GetLevel() < pItem.m_bReqLevel || GetLevel() > pItem.m_bReqLevelMax)
		// Ensure the item exists.
		|| !CheckExistItem(nItemID, sCount))
		return false;

	return true;
}

void CUser::SendUserStatusUpdate(UserStatus type, UserStatusBehaviour status)
{
	Packet result(WIZ_ZONEABILITY, uint8(2));
	result << uint8(type) << uint8(status);
	/*
	1				, 0 = Cure damage over time
	1				, 1 = Damage over time
	2				, 0 = Cure poison
	2				, 1 = poison (purple)
	3				, 0 = Cure disease
	3				, 1 = disease (green)
	4				, 1 = blind
	5				, 0 = Cure grey HP
	5				, 1 = HP is grey (not sure what this is)
	*/

	if (type == behav_type && status == behav_status)
		return;

	behav_type = type; behav_status = status;
	Send(&result);
	if (isInParty())
		SendPartyStatusUpdate((uint8)type, (uint8)status);
}

/**
* @brief	Gets an item's prototype from a slot in a player's inventory.
*
* @param	pos	The position of the item in the player's inventory.
* @returns	nullptr if an invalid position is specified, or if there's no item at that position.
* 			The item's prototype (_ITEM_TABLE *) otherwise.
*/
_ITEM_TABLE CUser::GetItemPrototype(uint8 pos, _ITEM_DATA*& pItem)
{
	if (pos >= INVENTORY_TOTAL) return _ITEM_TABLE();
	pItem = GetItem(pos);
	return pItem->nNum == 0 ? _ITEM_TABLE() : g_pMain->GetItemPtr(pItem->nNum);
}

/**
* @brief	Checks & removes any expired skills from
* 			the saved magic list.
*/
void CUser::CheckSavedMagic()
{
	Guard lock(m_savedMagicLock);
	if (m_savedMagicMap.empty()) return;
	std::set<uint32> deleteSet;
	foreach(itr, m_savedMagicMap)
	{
		if (itr->second <= UNIXTIME2) deleteSet.insert(itr->first);
	} foreach(itr, deleteSet) m_savedMagicMap.erase(*itr);
}

/**
* @brief	Inserts a skill to the saved magic list
* 			to persist across zone changes/logouts.
*
* @param	nSkillID 	Identifier for the skill.
* @param	sDuration	The duration.
*/
void CUser::InsertSavedMagic(uint32 nSkillID, uint16 sDuration)
{
	Guard lock(m_savedMagicLock);
	UserSavedMagicMap::iterator itr = m_savedMagicMap.find(nSkillID);

	// If the buff is already in the savedBuffMap there's no need to add it again!
	if (itr != m_savedMagicMap.end()) return;
	m_savedMagicMap.insert(std::make_pair(nSkillID, (uint64)UNIXTIME2 + ((uint64)sDuration * 1000)));
}

/**
* @brief	Removes the specified skill from the saved magic list.
*
* @param	nSkillID	Identifier for the skill.
*/
void CUser::RemoveSavedMagic(uint32 nSkillID)
{
	Guard lock(m_savedMagicLock);
	m_savedMagicMap.erase(nSkillID);
}

/**
* @brief	Checks if the given skill ID is already in our
* 			saved magic list.
*
* @param	nSkillID	Identifier for the skill.
*
* @return	true if the skill exists in the saved magic list, false if not.
*/
bool CUser::HasSavedMagic(uint32 nSkillID)
{
	Guard lock(m_savedMagicLock);
	return m_savedMagicMap.find(nSkillID) != m_savedMagicMap.end();
}

/**
* @brief	Gets the duration for a saved skill,
* 			if applicable.
*
* @param	nSkillID	Identifier for the skill.
*
* @return	The saved magic duration.
*/
int16 CUser::GetSavedMagicDuration(uint32 nSkillID)
{
	Guard lock(m_savedMagicLock);
	auto itr = m_savedMagicMap.find(nSkillID);
	if (itr == m_savedMagicMap.end())
		return 0;

	return int16((itr->second - UNIXTIME2) / 1000);
}

/**
* @brief	Recasts any saved skills on login/zone change.
*/
void CUser::RecastSavedMagic(uint8 buffType /* = 0*/)
{
	m_savedMagicLock.lock();
	UserSavedMagicMap castSet;
	foreach(itr, m_savedMagicMap)
	{
		if (itr->first != 0 || itr->second != 0)
			castSet.insert(make_pair(itr->first, itr->second));
	}
	m_savedMagicLock.unlock();

	if (castSet.empty())
		return;

	foreach(itr, castSet)
	{
		if (buffType > 0)
		{
			_MAGIC_TYPE4* pType = g_pMain->m_Magictype4Array.GetData(itr->first);

			if (pType == nullptr)
				continue;

			if (pType->bBuffType != buffType)
				continue;
		}

		if (isSiegeTransformation())
			continue;

		MagicInstance instance;
		instance.sCasterID = GetID();
		instance.sTargetID = GetID();
		instance.nSkillID = itr->first;
		instance.sSkillCasterZoneID = GetZoneID();
		instance.bIsRecastingSavedMagic = true;
		instance.Run();
	}
}

/**
* @brief	Recasts any lockable scrolls on debuff.
*/
void CUser::RecastLockableScrolls(uint8 buffType)
{
	InitType4(false, buffType);
	RecastSavedMagic(buffType);
}

//Stop
void CUser::InitializeStealth()
{
	Packet pkt(WIZ_STEALTH);
	pkt << uint8(0) << uint16(0);
	Send(&pkt);
}

uint32 CUser::GetEventTrigger()
{
	CNpc* pNpc = g_pMain->GetNpcPtr(GetTargetID(), GetZoneID());
	if (pNpc == nullptr)
		return 0;

	foreach_stlmap(itr, g_pMain->m_EventTriggerArray)
	{
		_EVENT_TRIGGER* pEventTrigger = itr->second;
		if (pEventTrigger == nullptr
			|| pNpc->GetType() != pEventTrigger->bNpcType)
			continue;

		if (pNpc->m_byTrapNumber == pEventTrigger->sNpcID)
			return pEventTrigger->nTriggerNum;
	}

	return 0;
}

void CUser::RemoveStealth()
{
	if (this->m_bInvisibilityType != (uint8)InvisibilityType::INVIS_NONE)
	{
		CMagicProcess::RemoveStealth(this, InvisibilityType::INVIS_DISPEL_ON_MOVE);
		CMagicProcess::RemoveStealth(this, InvisibilityType::INVIS_DISPEL_ON_ATTACK);
	}
}

void CUser::RobChaosSkillItems()
{
	if (GetItemCount(ITEM_LIGHT_PIT) > 0)
		RobItem(ITEM_LIGHT_PIT, GetItemCount(ITEM_LIGHT_PIT));
	if (GetItemCount(ITEM_DRAIN_RESTORE) > 0)
		RobItem(ITEM_DRAIN_RESTORE, GetItemCount(ITEM_DRAIN_RESTORE));
	if (GetItemCount(ITEM_KILLING_BLADE) > 0)
		RobItem(ITEM_KILLING_BLADE, GetItemCount(ITEM_KILLING_BLADE));
}

void CUser::LogosShout(Packet& pkt)
{
	uint8 sOpCode, R, G, B, C;
	string sMessage;
	DateTime time;

	pkt.SByte();
	pkt >> sOpCode >> R >> G >> B >> C >> sMessage;

	Packet result(WIZ_LOGOSSHOUT);

	if (!CheckExistItem(LOGOSSHOUT1, 1))
		sOpCode = 2;
	else if (sMessage.empty()
		|| sMessage.size() > 128)
		sOpCode = 2;
	else
		sOpCode = 1;

	string bMessage = GetName() + ": " + sMessage;
	result.SByte();

	if (sOpCode == 1)
		result << uint8(2) << sOpCode << R << G << B << C << bMessage;
	else
		result << uint8(1) << sOpCode;

	if (CheckExistItem(LOGOSSHOUT1, 1))
		RobItem(LOGOSSHOUT1, 1);

	if (sOpCode == 1)
		g_pMain->Send_All(&result);
	else
		Send(&result);
}

bool CUser::VaccuniBoxExp(uint16 MonsterType)
{
	if (!isInGame())
		return false;

	if (!MonsterType || MonsterType > 3) return true;

	switch (MonsterType)
	{
	case 1:
		if ((CheckExistEvent(793, 1) || CheckExistEvent(794, 1)))
		{
			_ITEM_DATA* pItem;
			_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
			if (pItem != nullptr && !pTable.isnull() && pItem->sDuration > 0 && pTable.isTimingFlowTyon())
				return false;
		}
		break;
	case 2:
		if (CheckExistEvent(798, 1) || CheckExistEvent(799, 1))
		{
			_ITEM_DATA* pItem;
			_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
			if (pItem != nullptr && !pTable.isnull() && pItem->sDuration > 0 && pTable.isTimingFlowHellHound())
				return false;
		}
		break;
	case 3:
		if ((CheckExistEvent(795, 1) || CheckExistEvent(796, 1)))
		{
			_ITEM_DATA* pItem;
			_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
			if (pItem != nullptr && !pTable.isnull() && pItem->sDuration > 0 && pTable.isTimingFlowMeganthereon())
				return false;
		}
		break;
	}
	return true;
}

bool CUser::VaccuniAttack()
{
	CNpc* pNpc = g_pMain->GetNpcPtr(GetTargetID(), GetZoneID());
	if (pNpc == nullptr || !isInGame())
		return false;

	switch (pNpc->GetProtoID())
	{
	case 4301:
		break;
	case 4351:
		if (CheckExistEvent(793, 1) || CheckExistEvent(794, 1))
		{
			_ITEM_DATA* pItem;
			_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
			if (pItem != nullptr && !pTable.isnull() && pItem->sDuration > 0 && pTable.isTimingFlowTyon())
				return true;
		}
		break;
	case 605:
		break;
	case 611:
		break;
	case 655:
		if (CheckExistEvent(795, 1) || CheckExistEvent(796, 1))
		{
			_ITEM_DATA* pItem;
			_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
			if (pItem != nullptr && !pTable.isnull() && pItem->sDuration > 0 && pTable.isTimingFlowMeganthereon())
				return true;
		}
		break;
	case 616:
		break;
	case 666:
		if (CheckExistEvent(798, 1) || CheckExistEvent(799, 1))
		{
			_ITEM_DATA* pItem;
			_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
			if (pItem != nullptr && !pTable.isnull() && pItem->sDuration > 0 && pTable.isTimingFlowHellHound())
				return true;
		}
		break;
	default:
		return false;
		break;
	}

	return false;
}

int CUser::GetExpPercent()
{
	foreach(itr, g_pMain->m_LevelUpArray)
	{
		_LEVEL_UP* pLevel = itr->second;
		if (pLevel == nullptr)
			continue;

		if (pLevel->Level == GetLevel() && pLevel->RebithLevel == GetRebirthLevel())
			return pLevel->Exp == m_iExp ? 100 : 0;
	}

	return 0;
}


enum PPCardOpcodes
{
	PPCARD_FAILED = 0,
	PPCARD_SUCCES = 1,
	PROMOTION_USED = 5,
	PROMOTION_FAILED = 3,
	PROMOTION_SUCCES = 4
};

enum PPCardErrorCodes
{
	PPCardSuccess = 1, // Iteam has inserted successfully.Please check the letter with pressing [L].
	PPCardFailed = 2, // The serial in not existed or wrong.Please insert other serial after 5 minutes
};

void CUser::SendPPCardFail(uint8 errorcode)
{
	Packet newpkt(WIZ_EDIT_BOX);
	newpkt << uint8(4) << errorcode;
	Send(&newpkt);
}

void CUser::PPCard(Packet& pkt)
{
	uint8 opcode = pkt.read<uint8>();

	if (opcode == 4)
	{
		pkt.SByte();
		int key1; std::string key2;
		pkt >> key1 >> key2;

		if (PPCardTime > UNIXTIME)
		{
			g_pMain->SendHelpDescription(this, string_format("[PPCard] : Please waiting"));
			return SendPPCardFail(PPCardFailed);
		}
		PPCardTime = UNIXTIME + PPCARD_TIME;

		std::string skey1 = std::to_string(key1);
		if (key1 >= 0 && key1 < 1000)
			skey1 = string_format("%04d", key1);

		// normalize user input
		key2.erase(std::remove_if(key2.begin(), key2.end(),
			[](unsigned char c) { return c == ' ' || c == '-' || c == '\t'; }), key2.end());

		if (skey1.empty() || skey1.length() != 4 || key2.empty() || key2.length() != 16)
		{
			g_pMain->SendHelpDescription(this, string_format("[PPCard] : Please check the characters you entered"));
			return SendPPCardFail(PPCardFailed);
		}

		std::string finalkey = skey1 + key2;
		if (finalkey.empty() || finalkey.length() != 20)
		{
			g_pMain->SendHelpDescription(this, string_format("[PPCard] : Please check the characters you entered"));
			return SendPPCardFail(PPCardFailed);
		}

		Packet newpkt(WIZ_EDIT_BOX, uint8(4));
		newpkt << finalkey;
		g_pMain->AddDatabaseRequest(newpkt, this);
	}
}

void CUser::ReqPPCard(Packet& pkt)
{
	uint8 opcode = pkt.read<uint8>();

	if (opcode == 4)
	{
		std::string cardkey;
		pkt >> cardkey;

		cardkey.erase(std::remove_if(cardkey.begin(), cardkey.end(),
			[](unsigned char c) { return c == ' ' || c == '-' || c == '\t'; }), cardkey.end());

		if (cardkey.empty() || cardkey.length() != 20)
		{
			g_pMain->SendHelpDescription(this, string_format("[PPCard] : Please check the characters you entered"));
			return SendPPCardFail(PPCardFailed);
		}

		uint8 result = g_DBAgent.LoadPPCard(cardkey, this);
		if (result == PPCARD_FAILED)
		{
			g_pMain->SendHelpDescription(this, string_format("[PPCard] : Key previously used or incorrect."));
			return SendPPCardFail(PPCardFailed);
		}

		g_pMain->SendHelpDescription(this, string_format("[PPCard] : Your transaction was successful."));
		SendPPCardFail(PPCardSuccess);
	}
}

// /plc komutunu calıstırır ve kullandıgı programları gösterir sol altta notice olarak 27.09.2020 start
#pragma region CUser::PlayerProgramCheck(Packet & pkt)
void CUser::PlayerProgramCheck(Packet& pkt)
{

	uint8 OPCode;

	std::string Name, ProgramInfo;

	pkt >> OPCode;

	if (OPCode == 1) // GameMaster Tarafı Gm /plc Nick Yapınca OpCode 1
	{
		if (!isGM())
			return;

		g_pMain->TempPlcCodeGameMasterSocket = -1; // tekrar gelirse sıfırlama

		pkt >> Name;

		CUser* PUser = g_pMain->GetUserPtr(Name, NameType::TYPE_CHARACTER);

		if (PUser == nullptr)
			return;

		Packet result(WIZ_PROGRAMCHECK, uint8(0x02));
		result << uint8(0x01);
		PUser->Send(&result);

		g_pMain->TempPlcCodeGameMasterSocket = GetSocketID(); // Sonraki İşlem İçin Gm Socket Id Hafıza Alıyoruz
	}
	else
	{
		// ko exenin gönderdiğin
		CUser* GMTemp = g_pMain->GetUserPtr(g_pMain->TempPlcCodeGameMasterSocket);

		if (GMTemp == nullptr)
			return;

		pkt >> ProgramInfo;
		g_pMain->SendHelpDescription(GMTemp, ProgramInfo.c_str());
		// socket id sini aldıgımız gm ye sendhelp gönderiyoruz
		// LOG SQL Yapılabilir.

	}
}
#pragma endregion
// /plc komutunu calıstırır ve kullandıgı programları gösterir sol altta notice olarak 27.09.2020 end

void CUser::ClientMerchantWindNotice(std::string Msg, std::string Name, uint16 GetX, uint16 GetZ, uint32 color)
{
	if (!isInGame())
		return;

	std::string txt_msg = string_format("%s : %s(Location:%d,%d)", Name.c_str(), Msg.c_str(), GetX, GetZ);

	Packet result(WIZ_ADD_MSG, uint8(1));
	result << txt_msg << color << color;
	SendToMerchant(&result);
}

#pragma region CaptchaRequest
void CUser::CaptchaRequest(Packet& pkt) //xx
{
	switch (pkt[1])
	{
	case 0x01:
		CaptchaProcess();
		break;

	case 0x02:
		CaptchaVerification(pkt);
		break;

	default:
		printf("[SID=%d] Unhandled (%X) captcha request\n", GetSocketID(), pkt[1]);
		break;
	}
}
#pragma endregion

#pragma region CaptchaVerification
void CUser::CaptchaVerification(Packet& pkt) //xx
{

	Packet result(0xC0);
	BYTE truePkt[5] = { 0x01, 0x02, 0x01, 0x00, 0x00 };
	result.append(truePkt, 5);
	Send(&result);

}
#pragma endregion

#pragma region CaptchaProcess
void CUser::CaptchaProcess() //xx
{
	/*BYTE captCHA[3824] = { 0xD3,0x0E,0x00,0x00,0x1A,0xC0,0x00,0x00,0x00,0x00,0x00,0x00,0x06,0xC0,0x01,0x01,0x01,0x12,0xC0,0x00,0x20,0x00,0x00,0x02,0x20,0x03,0xA0,0x00,0x05,0x01,0x40,0x00,0x18,0x00,0xF0,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xB3,0x00,0x0B,0xC0,0xC0,0xC0,0x80,0x80,0x80,0x70,0x70,0x70,0xB0,0xB0,0xB0,0xE0,0x45,0xC7,0x05,0xD0,0xD0,0xD0,0x90,0x90,0x90,0x20,0x59,0x20,0x05,0x20,0x0B,0xE0,0x45,0x5C,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x30,0x00,0x22,0xFC,0x05,0x10,0x10,0x10,0x00,0x00,0x00,0x22,0xA8,0xE0,0x30,0x44,0xE0,0x0C,0x00,0x05,0xA0,0xA0,0xA0,0x30,0x30,0x30,0x20,0x59,0x20,0x05,0x20,0x0B,0xE0,0x0C,0x23,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x69,0x00,0xE2,0x51,0xFF,0x25,0xF3,0x02,0x50,0x50,0x50,0xE2,0xFF,0xFF,0xE0,0xFF,0x00,0xE0,0x87,0x00,0xE2,0x51,0xFF,0x20,0x00,0x28,0xF9,0x26,0x5C,0x20,0x00,0x02,0x60,0x60,0x60,0x23,0x0E,0x20,0x11,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x42,0x00,0x2B,0xC0,0x28,0xC3,0x20,0x05,0xE0,0x24,0x53,0xE2,0x54,0xFF,0x28,0xF6,0x28,0xFC,0xE6,0x45,0x02,0x2C,0xB0,0x20,0x00,0xE0,0x24,0xE3,0xE0,0x33,0x00,0x21,0x4F,0xE1,0x2A,0x55,0xE0,0xFF,0x00,0xE0,0x63,0x00,0xE1,0x2D,0xA9,0x22,0x4B,0x02,0x40,0x40,0x40,0x22,0xA2,0x21,0xE5,0xE0,0x48,0xAD,0xE2,0x4B,0xFF,0x80,0xE6,0x20,0x00,0x23,0xB9,0x20,0xBC,0xE0,0x48,0xB3,0xE0,0x06,0x00,0x80,0x6E,0x20,0x05,0xE0,0x06,0x17,0xE0,0xFF,0x00,0xE0,0x2D,0x00,0x21,0x4F,0x21,0xC1,0xE0,0x1B,0x00,0x20,0x29,0xE0,0x2D,0x62,0xE0,0x21,0x00,0x22,0x48,0x22,0xFF,0x25,0xA8,0xE0,0x48,0x6B,0xE2,0x4B,0xFF,0x02,0xE0,0xE0,0xE0,0x29,0x4D,0x20,0xB0,0x29,0x56,0x21,0x40,0x20,0x0E,0x20,0xB6,0x20,0x08,0xE0,0x1B,0x00,0x20,0x29,0xE1,0x21,0x16,0xE0,0x81,0x00,0x20,0xB6,0x20,0x00,0xE0,0x48,0x8F,0x20,0x53,0xE1,0x06,0x34,0x20,0x11,0x80,0x65,0x20,0x08,0xE0,0x06,0x1A,0x20,0x00,0x20,0x14,0x80,0x1D,0xE0,0x1E,0x00,0x20,0x32,0xE0,0x03,0x00,0x20,0x3E,0x20,0x00,0x80,0x11,0x22,0x6C,0x21,0xC1,0xE0,0x1B,0x00,0x20,0x29,0x80,0x32,0x20,0x00,0x20,0x3E,0xE0,0x1E,0x7A,0xE0,0x24,0x00,0x22,0xFF,0x22,0x4B,0x20,0x8C,0x20,0x62,0xE0,0x24,0x38,0xE0,0x18,0x00,0xE2,0x30,0xFF,0x20,0xE9,0x20,0x8C,0xE0,0x0F,0x00,0x21,0x13,0x20,0xAD,0x20,0x00,0x20,0x08,0x23,0x0B,0x20,0x05,0x20,0x0B,0xE0,0x1B,0x00,0x20,0x29,0xE0,0x09,0x53,0x20,0x6B,0xE0,0x18,0xC8,0xE0,0x72,0x00,0x20,0x9E,0x20,0xB3,0x20,0x05,0xE0,0x3F,0x83,0x20,0x4A,0x20,0x50,0x20,0x00,0x21,0x0D,0xE1,0x06,0x34,0x20,0x14,0x80,0x65,0x20,0x08,0xE0,0x06,0x1A,0x20,0x00,0x20,0x2F,0x20,0x17,0xE0,0x03,0x00,0x20,0x4A,0x80,0x2F,0xE0,0x0C,0x00,0x20,0x32,0xE0,0x03,0x00,0x20,0x3E,0x20,0x00,0xE0,0x00,0x11,0x20,0x0B,0xE0,0x03,0x4A,0xE0,0x0C,0x00,0x20,0x23,0xE0,0x00,0x2F,0x20,0x00,0x20,0x0E,0xE0,0x00,0x26,0xE0,0x15,0x7A,0xE0,0x24,0x00,0xE2,0x84,0xFF,0x20,0xD7,0x20,0xE3,0x20,0xE9,0x20,0xEF,0xE0,0x2D,0x00,0x20,0x3B,0x20,0x41,0xE0,0x09,0x00,0x20,0x17,0xE0,0x0C,0x50,0x20,0x17,0x20,0x2C,0x20,0x77,0xE1,0x24,0x34,0xE0,0x60,0x00,0x20,0x98,0x20,0x00,0xE0,0x39,0x6E,0x20,0x44,0x20,0xE6,0xE0,0x03,0x00,0x20,0xF8,0xE1,0x06,0x10,0x20,0x11,0x80,0x20,0x20,0x08,0xE0,0x06,0x1A,0xE0,0x0C,0x00,0x20,0x26,0x80,0x2F,0xE0,0x00,0x00,0x20,0x65,0x80,0xAA,0x20,0x08,0x20,0x14,0x20,0x20,0x20,0x00,0x20,0x08,0x20,0x00,0x20,0x08,0xE0,0x03,0x44,0x20,0x0E,0x20,0x14,0x20,0x26,0x80,0x2F,0xE0,0x0C,0x00,0x20,0x1D,0x20,0x23,0x80,0x00,0x20,0x2F,0xE0,0x09,0x3E,0x20,0x00,0x20,0x23,0xE0,0x0C,0x3B,0xE0,0x21,0x00,0xE2,0x7E,0xFF,0x20,0xC8,0x20,0xCE,0xE0,0x00,0xE6,0x20,0x08,0x20,0x0E,0xE0,0x09,0x00,0x20,0x17,0xE1,0x03,0x07,0x26,0xB9,0x2C,0xBF,0xE0,0x03,0x14,0x20,0x0B,0x20,0x35,0x20,0x4D,0xE1,0x06,0x01,0x20,0x11,0x20,0x17,0xE0,0x06,0x00,0x20,0x2C,0x80,0x4D,0x20,0x08,0x20,0x1A,0x20,0x23,0xE0,0x06,0x35,0xE0,0xAE,0x00,0x20,0xC8,0x20,0xCE,0xE0,0x03,0x00,0x20,0xE0,0x80,0xE9,0x20,0x14,0x80,0xD4,0xE0,0x09,0x23,0xE0,0x18,0x00,0xE0,0x00,0x44,0xE0,0x0F,0x00,0x20,0x20,0x80,0x44,0x20,0x08,0x20,0x5F,0x20,0x05,0x20,0x0E,0xE0,0x06,0x6E,0x20,0x14,0xE0,0x00,0x3E,0x20,0x0B,0x20,0x20,0x20,0x29,0x80,0x23,0xE0,0x12,0x00,0x20,0x23,0x20,0x29,0x80,0x00,0x20,0x35,0xE0,0x06,0x41,0xE0,0x03,0x00,0x20,0x29,0xE0,0x12,0x47,0xE0,0x09,0x00,0xE2,0x00,0xFF,0x20,0x56,0xE0,0x03,0x47,0xE0,0x03,0x00,0x28,0xC9,0x2B,0x69,0xE0,0x09,0x3B,0xE0,0x15,0x00,0x34,0xF3,0xF8,0x1B,0x02,0x20,0xB0,0x20,0x6B,0x20,0x7D,0x20,0xD4,0x20,0x05,0x20,0x0B,0x20,0x00,0x20,0x14,0xE0,0x06,0x5C,0x20,0x11,0x20,0x17,0x20,0x20,0x20,0x26,0xE0,0x00,0x00,0x22,0xFF,0x20,0x14,0x20,0x9E,0x20,0x05,0x20,0x00,0xE0,0x00,0x1D,0x20,0x08,0x20,0x0E,0xE0,0x15,0x00,0x20,0x50,0x20,0x00,0x20,0x23,0x20,0x2C,0x80,0x50,0x20,0x08,0x20,0x0E,0x80,0x00,0x20,0x1A,0x80,0x80,0x20,0x08,0x20,0x11,0x20,0x05,0x80,0x0E,0xE0,0x96,0x00,0x20,0xA7,0x20,0xAD,0x20,0xC8,0x80,0xD1,0xE0,0x06,0x00,0x20,0x17,0x20,0x20,0xE0,0x30,0xC2,0x20,0x3B,0x20,0x41,0xE0,0x06,0x53,0xE0,0x0C,0x00,0x20,0x26,0x20,0x00,0x20,0x1A,0x20,0x8C,0xE0,0x06,0x6E,0x20,0x44,0x20,0x14,0x20,0x00,0x80,0x20,0x20,0x00,0x20,0x08,0x20,0x0E,0x20,0x00,0x20,0x1A,0xE0,0x06,0x2C,0xE0,0x0C,0x00,0x20,0x26,0x20,0x2C,0x20,0x00,0x20,0x38,0x20,0x3E,0xE0,0x06,0x00,0x20,0x14,0x20,0x1A,0x20,0x00,0x20,0x26,0xE0,0x0C,0x3E,0xE0,0x06,0x00,0xE2,0x00,0xFF,0xE0,0x12,0x00,0x2B,0x66,0x22,0xCC,0xE0,0x06,0x38,0xE0,0x1B,0x00,0xF8,0x09,0x02,0x20,0x95,0x20,0x9B,0xE0,0x03,0x00,0x20,0xB0,0x80,0x7A,0x20,0x08,0x20,0x1D,0xE0,0x09,0x56,0x20,0x14,0x20,0x2F,0x20,0x1D,0xE0,0x00,0x26,0x20,0x0E,0x20,0x92,0x80,0x11,0x25,0xFF,0x20,0x20,0x80,0x35,0x20,0x08,0x20,0x1A,0x80,0x17,0xE0,0x18,0x00,0x20,0x26,0x20,0x2C,0x20,0x32,0x20,0x00,0x20,0x0B,0xE0,0x06,0x2F,0x20,0x11,0x20,0x1D,0x20,0x00,0x20,0x08,0x20,0x1A,0x20,0x05,0x20,0x26,0x80,0x65,0xE0,0x90,0x00,0x20,0xA1,0x20,0xA7,0x20,0xAD,0xE0,0x06,0x00,0x20,0x14,0x20,0xC8,0x20,0x00,0x20,0x20,0xE0,0x21,0xBC,0x20,0x2C,0x20,0x32,0x80,0x00,0x20,0x0B,0x80,0x38,0x20,0x08,0x80,0x11,0xE0,0x03,0x00,0x20,0x5F,0x80,0x71,0x20,0x08,0x80,0x17,0x20,0x26,0x20,0x11,0x20,0x0E,0x20,0x08,0x80,0x38,0xE0,0x09,0x00,0x20,0x1A,0x20,0x2C,0x20,0x23,0x20,0x29,0x80,0x00,0x20,0x0B,0x20,0x14,0xE0,0x09,0x29,0xE0,0x0F,0x00,0x20,0x2C,0x20,0x41,0xE0,0x03,0x00,0x20,0x41,0x80,0x4A,0x20,0x00,0x20,0x0B,0x20,0x20,0xE0,0x0F,0x3B,0xE0,0x00,0x00,0xE2,0x45,0xFF,0x20,0x71,0xE0,0x00,0x8F,0x20,0x0B,0x23,0x35,0x23,0x02,0x20,0x11,0x20,0x05,0x20,0x0B,0x20,0x11,0x20,0x95,0x20,0x9B,0xE0,0x06,0x00,0x20,0x14,0x20,0x23,0x20,0x1D,0xE0,0x00,0x95,0x80,0x00,0x20,0x11,0x20,0x17,0x20,0x1D,0x80,0x2F,0x20,0x08,0x20,0x0E,0x20,0x14,0x20,0x1D,0x26,0x8F,0x23,0x02,0x20,0x17,0x86,0x02,0x20,0x0E,0x20,0x14,0x80,0x00,0x20,0x20,0x20,0x26,0xE0,0x15,0x00,0x20,0x4A,0x20,0x3B,0x20,0x00,0x20,0x08,0x20,0x29,0x20,0x05,0x20,0x0B,0xE0,0x15,0x00,0x20,0x23,0x20,0x59,0x80,0x62,0xE0,0x7B,0x00,0x20,0x8C,0x20,0xB9,0xE0,0x03,0x00,0x20,0xA1,0x80,0xC2,0x20,0x08,0xE0,0x03,0x17,0x20,0x29,0xE0,0x2A,0xB0,0x20,0x35,0xE0,0x00,0x44,0x20,0x0B,0xE0,0x12,0x41,0x20,0x1D,0x20,0x71,0x20,0x7A,0x20,0x05,0x20,0x0B,0x20,0x29,0x20,0x05,0x20,0x3B,0x20,0x05,0x20,0x0B,0xE0,0x15,0x00,0x20,0x23,0x20,0x29,0x20,0x00,0x20,0x3B,0xE0,0x03,0x41,0xE0,0x06,0x00,0x80,0x26,0x20,0x05,0xE0,0x06,0x17,0xE0,0x06,0x00,0x20,0x20,0x20,0x4A,0x20,0x00,0xE0,0x00,0x4D,0x20,0x0B,0x80,0x00,0x20,0x1A,0xE0,0x06,0x2C,0x80,0x00,0xE2,0x00,0xFF,0x20,0x32,0x80,0x29,0xE0,0x09,0x00,0x28,0x60,0xE2,0x03,0x60,0xE0,0x12,0x00,0x20,0x3B,0x20,0xC5,0x80,0x00,0xE0,0x09,0x50,0x20,0x11,0xE0,0x0C,0x1A,0xE0,0x03,0x9B,0x20,0x0B,0x20,0x44,0x80,0x00,0xE0,0x06,0x2F,0x20,0x1A,0xE0,0x03,0x7A,0x20,0x0E,0x80,0x20,0xE6,0x03,0x02,0xE0,0x1E,0x00,0x20,0x3B,0x20,0x62,0x80,0x3E,0xE0,0x21,0x00,0x20,0x2F,0x20,0x35,0x20,0x3B,0xE0,0x1E,0x65,0xE0,0x51,0x00,0x20,0x83,0x20,0x8C,0xE0,0x0C,0xB9,0x20,0x17,0x20,0x1D,0xE0,0x51,0x7A,0xE0,0x0C,0x00,0x20,0x71,0x20,0x77,0x20,0x8F,0x20,0x05,0x20,0x0B,0x21,0x25,0xE0,0x0C,0x26,0xE0,0x12,0x00,0x20,0x35,0x20,0x3B,0xE0,0x00,0x41,0xE0,0x00,0x29,0x20,0x17,0x20,0x4D,0x80,0x00,0x20,0x0B,0xE0,0x00,0x17,0xE0,0x15,0x00,0x20,0x29,0x80,0x4D,0xE0,0x00,0x00,0x20,0x0E,0x20,0x44,0x20,0x17,0xE0,0x06,0x38,0xE2,0x00,0xFF,0x20,0x1D,0xE0,0x06,0x1A,0xE0,0x2A,0x00,0x20,0x5F,0x20,0x68,0xE0,0x00,0x74,0xE0,0x0F,0x00,0x20,0x23,0x20,0x6E,0x20,0x7A,0xFA,0x06,0xC3,0xE0,0x09,0x3E,0x80,0x29,0x20,0x17,0xE0,0x0C,0x8C,0x20,0x17,0x20,0x20,0x26,0x02,0x23,0x02,0xE3,0x21,0xA4,0xE0,0x00,0x00,0x20,0x3E,0x20,0x86,0x20,0x41,0xE0,0x0C,0xA7,0x20,0x17,0x20,0x00,0xE0,0x09,0x1A,0x20,0x14,0x20,0x35,0x20,0x00,0x20,0x3E,0xE0,0x00,0x4A,0xE0,0x60,0x00,0x20,0x74,0x20,0x7A,0x20,0x83,0x20,0x98,0x20,0x05,0x20,0x0B,0x20,0x00,0x20,0x08,0x20,0x0E,0x20,0x00,0x20,0x08,0x20,0x20,0xE0,0x60,0x8C,0xE0,0x03,0x00,0x20,0x77,0x20,0x7D,0x80,0x83,0x20,0x92,0xE0,0x03,0x1A,0xE0,0x1E,0x00,0xE0,0x06,0x41,0x80,0x00,0x20,0x53,0x20,0x00,0x20,0x1A,0xE0,0x1E,0x44,0xE0,0x00,0x00,0x20,0x32,0x80,0x41,0x20,0x00,0x20,0x41,0x20,0x98,0x20,0x00,0x20,0x08,0x20,0x17,0xE0,0x00,0x23,0x20,0x00,0x22,0x84,0x22,0x3F,0x20,0x1A,0x20,0x26,0x20,0x0E,0xE0,0x39,0x00,0x20,0x5F,0x20,0x65,0x20,0x50,0xE0,0x18,0x00,0x20,0x77,0x20,0x7D,0xE0,0x00,0x71,0xE0,0x00,0x38,0x20,0x3E,0x20,0x8C,0x80,0x00,0x20,0x14,0xE0,0x00,0x20,0xE0,0x0C,0x00,0x22,0xF6,0x20,0x2F,0x20,0x6B,0x23,0x02,0xE0,0x18,0x2C,0xE0,0x0C,0x00,0x20,0x35,0x80,0x41,0x20,0x00,0x20,0x08,0x20,0x00,0x20,0x08,0x20,0x00,0x20,0x08,0x80,0x83,0xE0,0x03,0x00,0x20,0x14,0x20,0x1A,0x80,0x00,0x20,0x0B,0xE0,0x00,0x1A,0x20,0x44,0xE0,0x0C,0x5C,0xE0,0x3F,0x00,0x20,0x5F,0x80,0x6B,0x20,0x74,0xE0,0x00,0x7D,0x20,0x14,0x20,0x00,0x20,0x17,0x80,0x14,0x20,0x05,0x20,0x0E,0xE0,0x3F,0x71,0xE0,0x2A,0x00,0x20,0x7D,0x20,0x83,0x20,0x9E,0x20,0x92,0xE0,0x2A,0x3E,0x20,0x00,0xE0,0x00,0x41,0xE0,0x03,0x00,0x20,0x53,0x20,0x50,0x20,0x1A,0x20,0x20,0xE0,0x2D,0x00,0x20,0x3B,0x20,0x41,0x20,0x47,0x20,0x56,0x20,0x05,0x20,0x0E,0xE0,0x00,0x47,0x22,0x81,0x22,0x87,0x20,0x17,0x20,0x20,0xE0,0x00,0x14,0xE0,0x30,0x00,0x20,0x59,0x20,0x47,0x20,0x62,0x20,0x50,0x20,0x05,0x20,0x0B,0x32,0x02,0x80,0x5F,0x80,0x0E,0x80,0x00,0x20,0x0B,0x20,0x20,0x20,0x17,0x20,0x7D,0x20,0x6B,0xE0,0x06,0x35,0x20,0x0E,0x20,0x14,0xE0,0x1E,0x00,0x29,0x98,0xEC,0x0F,0x08,0xE0,0x21,0x00,0x20,0x95,0x20,0x8F,0x20,0x00,0x20,0x08,0x20,0x7D,0x20,0x05,0x80,0xA1,0x20,0x05,0x20,0x0E,0xE0,0x03,0x47,0x20,0x0E,0x20,0x1D,0x20,0x00,0x20,0x1A,0x20,0x2F,0xE0,0x00,0x00,0x20,0x0E,0x20,0x14,0x20,0x1D,0xE0,0x03,0x2C,0xE0,0x3C,0x00,0x20,0x53,0x20,0x59,0x20,0x5F,0xE0,0x09,0x6B,0x20,0x00,0xE0,0x00,0x17,0x20,0x08,0x20,0x0E,0xE0,0x3C,0x71,0xE0,0x33,0x00,0x20,0x83,0x20,0xAD,0x20,0x05,0xE0,0x30,0x44,0x20,0x3B,0x20,0x41,0xE0,0x03,0x00,0x20,0xDA,0x20,0xFB,0x20,0x05,0x20,0x14,0x20,0x1D,0xE0,0x33,0x59,0x20,0x44,0x80,0x4A,0xE0,0x00,0x47,0x22,0x81,0x22,0x87,0x20,0x14,0x20,0x5C,0xE0,0x30,0x59,0x80,0x00,0x20,0x53,0x80,0x5F,0x20,0x00,0x20,0x08,0x20,0x0E,0x20,0x17,0xE5,0x00,0x9F,0xE0,0x03,0x5F,0x20,0x1A,0x20,0x0E,0x20,0x7A,0x20,0x00,0x20,0x08,0x20,0x2C,0x20,0x32,0x20,0x00,0x20,0x08,0x20,0x1A,0x20,0x35,0xE0,0x24,0x00,0x20,0x32,0x20,0x38,0x20,0x3E,0xE6,0x30,0x02,0x20,0x83,0x20,0x3E,0x20,0x00,0x20,0x08,0x80,0x7A,0xE0,0x00,0x53,0x20,0x59,0x20,0x0B,0x80,0x14,0xE0,0x03,0x00,0x20,0x14,0x20,0x2C,0x80,0x00,0x20,0x23,0x20,0x3B,0x20,0x00,0x20,0x08,0x80,0x11,0xE0,0x00,0x00,0x20,0x26,0xE0,0x03,0x35,0xE0,0x27,0x00,0x20,0x3E,0x20,0x53,0x20,0x59,0x20,0x00,0x20,0x08,0x80,0x56,0x20,0x14,0x20,0x47,0x20,0x05,0x20,0x11,0xE0,0x00,0x17,0x20,0x0E,0x20,0x14,0xE0,0xCC,0x00,0x20,0xDA,0x80,0xE9,0x20,0x00,0x20,0x08,0x20,0x0E,0xE0,0x30,0xE6,0x20,0x3B,0x20,0x41,0xE0,0x03,0x47,0x22,0x81,0xE2,0x36,0xFF,0x20,0x53,0x81,0x97,0xE0,0x00,0x59,0x20,0x0E,0x20,0x14,0x80,0xA4,0xE2,0x00,0xFF,0x26,0x08,0x32,0x05,0x20,0x1A,0xE0,0x03,0x00,0x80,0x32,0x20,0x00,0x20,0x08,0x20,0x17,0xE0,0x00,0x35,0xE0,0x27,0x00,0x20,0x59,0x20,0xC5,0x20,0x00,0x20,0xCE,0xE0,0x27,0x3B,0x20,0x00,0x20,0x7A,0x20,0x83,0x20,0x00,0x20,0x08,0x20,0x0E,0x80,0x00,0x20,0x8C,0x20,0x95,0x20,0x14,0x20,0x05,0x20,0x0B,0x80,0x14,0xE0,0x0C,0x00,0x20,0x1D,0x20,0x38,0x20,0x26,0x20,0x2C,0xE0,0x06,0x00,0x20,0x14,0x20,0x1A,0x20,0x20,0xE0,0x0C,0x38,0xE0,0x03,0x00,0x20,0x23,0x20,0x29,0xE0,0x06,0x00,0x20,0x3E,0x20,0x50,0x20,0x05,0x20,0x17,0x20,0x20,0x80,0x2F,0x20,0x08,0x20,0x0E,0x20,0x14,0x80,0x1A,0x20,0x0E,0x80,0x17,0xE0,0xD2,0x00,0x20,0xE3,0xE0,0x03,0xF2,0x20,0x0B,0x20,0x00,0x20,0x14,0xE0,0x2A,0xF2,0x20,0x35,0xE1,0x00,0x3A,0x20,0x44,0x25,0x33,0x21,0x46,0x20,0x00,0x20,0x14,0x20,0x0E,0xE0,0x0C,0x00,0x22,0x54,0xEE,0x06,0xA8,0xE0,0x00,0x00,0xE0,0x00,0x4D,0x80,0x00,0x20,0x44,0x20,0x11,0xE0,0x00,0x1D,0x20,0x00,0x20,0x0E,0x20,0x14,0x20,0x1D,0x22,0x9C,0x20,0x00,0x20,0x08,0xE0,0x06,0x00,0x20,0x1D,0x20,0x74,0x20,0x26,0x20,0x2C,0xE0,0x30,0x00,0x35,0x5C,0x23,0x5F,0x20,0x59,0x20,0x05,0x20,0x0B,0xE0,0x27,0x47,0x20,0x80,0x20,0x3B,0x20,0x00,0x20,0x08,0xE0,0x03,0x3B,0x20,0x95,0x20,0x9E,0x20,0x17,0x20,0x05,0x20,0x1A,0xE0,0x06,0x00,0x20,0x1D,0xE0,0x06,0x2C,0xE0,0x06,0x20,0x20,0x35,0x20,0x3B,0x20,0x00,0x20,0x08,0x20,0x1A,0xE0,0x06,0x2F,0xE0,0x03,0x00,0x20,0x1A,0x20,0x23,0x20,0x29,0xE0,0x09,0x00,0x20,0x17,0x20,0x1D,0x80,0x2C,0x20,0x08,0x20,0x47,0x20,0x11,0x80,0x26,0x20,0x08,0x20,0x11,0x80,0x1A,0xE0,0xDB,0x00,0x20,0xEC,0x20,0xF2,0x80,0xFB,0x20,0x00,0x20,0x0B,0x21,0x0D,0x20,0x14,0xE0,0x27,0xFB,0x20,0x32,0x20,0x3B,0x20,0x41,0xE0,0x1E,0x00,0x02,0x20,0x20,0x20,0x25,0x60,0x80,0x65,0x20,0x3B,0x20,0x74,0xE0,0x00,0x00,0x80,0x47,0x20,0x05,0x80,0x11,0x20,0x1D,0xE0,0x06,0x26,0x2B,0x9C,0x20,0x3E,0xE0,0x00,0x68,0xE0,0x00,0x29,0xE0,0x00,0x00,0xE0,0x00,0x32,0xE0,0x36,0x00,0x20,0xA1,0x23,0x5F,0x20,0x00,0x20,0x08,0xE0,0x27,0x4A,0x20,0x8C,0x80,0xA1,0x20,0x08,0xE0,0x06,0x3B,0x20,0x9E,0x20,0xB3,0x80,0x1D,0xE0,0x06,0x00,0x20,0x17,0x20,0x1D,0xE0,0x06,0x2F,0xE0,0x03,0x00,0x20,0x1D,0x20,0x50,0xE0,0x0C,0x00,0x20,0x1A,0x80,0x29,0x20,0x08,0x20,0x20,0x20,0x4A,0x20,0x5C,0x20,0x05,0x20,0x0B,0xE0,0x12,0x00,0x20,0x20,0x20,0x26,0x20,0x00,0x20,0x08,0x20,0x26,0x20,0x3B,0x80,0x44,0xE0,0xE1,0x00,0x20,0xF2,0x20,0xF8,0x20,0x00,0x21,0x01,0x21,0x07,0x20,0x00,0x20,0x08,0x20,0x0E,0x20,0x00,0x20,0x1A,0xE1,0x21,0x07,0x20,0x2C,0x20,0x32,0x20,0x00,0x20,0x3E,0x20,0x44,0xE0,0x15,0x00,0x22,0x54,0x26,0x26,0x20,0x2C,0x20,0x00,0xE0,0x0C,0x2F,0x20,0x14,0x20,0x1A,0xE0,0x03,0x00,0x20,0x11,0xE0,0x0C,0x53,0x23,0x0E,0x2E,0xB1,0x80,0xA7,0x20,0x08,0x20,0x86,0x20,0x00,0x20,0x08,0x80,0x11,0xE0,0x33,0x00,0x25,0xF6,0x25,0xFC,0x20,0x74,0x20,0x05,0x20,0x0B,0xE0,0x24,0x4A,0x20,0xB9,0x20,0x38,0x20,0x00,0x20,0xB6,0x20,0x0B,0xE0,0x00,0x00,0x20,0x9B,0x20,0x47,0x20,0x05,0xE0,0x00,0x11,0xE0,0x00,0x00,0x20,0x29,0xE0,0x06,0x2F,0x20,0x26,0x20,0x2C,0xE0,0x12,0x00,0x20,0x3B,0x20,0x32,0x80,0x00,0x20,0x3E,0x20,0x0E,0x20,0x00,0x20,0x08,0x80,0x11,0x20,0x00,0x20,0x0B,0x20,0x11,0xE0,0x06,0x00,0x20,0x14,0x20,0x1A,0xE0,0x00,0x00,0x20,0x0E,0x20,0x20,0xE0,0x18,0x6B,0xE0,0xD8,0x00,0x21,0x01,0x21,0x07,0x21,0x0D,0xE1,0x06,0x19,0xE0,0x27,0x00,0xE0,0x03,0x41,0xE0,0x18,0x00,0x20,0x2C,0x80,0x5F,0x20,0x08,0xE0,0x21,0x2C,0x20,0x00,0x22,0x93,0x23,0x44,0x20,0x32,0x20,0x38,0x23,0x02,0x20,0x11,0x20,0x00,0x20,0x08,0x20,0x0E,0x20,0x1A,0xE9,0x0F,0x5C,0xE0,0x27,0x00,0x25,0xF9,0x20,0x65,0x20,0x00,0x20,0x08,0xE0,0x24,0x3B,0x20,0x8C,0x20,0x9B,0xE0,0x09,0x00,0x20,0x9E,0x20,0x1A,0x80,0x00,0x2C,0xC8,0xE3,0x03,0xA7,0x21,0x7F,0x20,0x1D,0xE0,0x03,0x32,0x20,0x0E,0x80,0x29,0xE0,0x0F,0x00,0x20,0x20,0xE0,0x03,0x2F,0xE0,0x21,0x00,0x20,0x38,0xE0,0x00,0x53,0x20,0x77,0xE0,0x24,0xDA,0xE0,0xD5,0x00,0x21,0x0D,0x81,0x19,0x21,0x22,0xE1,0x21,0x4F,0xE0,0x27,0x00,0x20,0x5C,0x80,0x65,0x20,0x6E,0x81,0x4F,0x20,0x08,0x80,0x11,0xE0,0x12,0x00,0x20,0x23,0x80,0x2C,0x20,0x00,0x25,0x90,0x2C,0x44,0x26,0x44,0x22,0x9C,0xE0,0x06,0x7D,0x20,0x14,0xE2,0x1E,0xFF,0x22,0x7E,0x20,0x00,0x20,0x4D,0xE0,0x0F,0x00,0x20,0x65,0x23,0x4D,0x20,0x5F,0x20,0x05,0x20,0x0B,0xE0,0x0F,0x26,0xE0,0x09,0x00,0x20,0xC5,0x20,0x35,0xE0,0x00,0x00,0xE0,0x00,0xEF,0x20,0x08,0xE0,0x00,0x14,0xE0,0x09,0xA7,0x20,0xE0,0x20,0x38,0x80,0x00,0xE0,0x03,0x29,0xE0,0x24,0x00,0x20,0x38,0x80,0x41,0xE0,0x12,0x00,0x20,0x68,0xE0,0x09,0xB3,0xE0,0xFF,0x00,0x80,0x00,0x21,0x22,0xE1,0x12,0x40,0xE0,0x36,0x00,0x20,0x5C,0x80,0x65,0xE0,0x33,0x00,0x22,0xBA,0x20,0x47,0x25,0x99,0x23,0x08,0x21,0xF4,0x82,0x24,0x2C,0x1A,0x20,0x0E,0x20,0x1A,0xE0,0x1B,0x59,0x20,0x26,0x20,0x00,0x20,0x29,0x20,0x47,0x80,0xD1,0x20,0x08,0x20,0x0E,0xE0,0x03,0x00,0x20,0x11,0x20,0x1A,0x20,0x65,0xE2,0x2D,0xFF,0x20,0x98,0x20,0x3E,0x20,0x44,0x80,0x53,0x20,0x0B,0x80,0xA7,0x20,0x00,0xE2,0x09,0xFF,0xE0,0x03,0x00,0x20,0x32,0x20,0x2C,0xE0,0x2A,0x00,0x20,0x38,0xE0,0x03,0x47,0xE0,0xFF,0x00,0xE0,0xD5,0x00,0x21,0xF4,0x22,0x2A,0x22,0xF9,0xE8,0x06,0x54,0xE0,0x1E,0x00,0x20,0x3B,0x22,0x90,0x20,0x00,0x82,0xAE,0xE0,0x06,0x00,0x20,0x5C,0xE0,0x1E,0x47,0xE0,0x00,0x00,0x20,0x41,0x20,0x4D,0x20,0x00,0x20,0x08,0xE0,0x00,0x14,0x20,0x00,0x20,0x4A,0x20,0x11,0x80,0x00,0xE0,0x2D,0xAA,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x36,0x00,0x82,0x90,0x22,0xF6,0x22,0xAB,0xE0,0x0F,0x00,0x20,0x1D,0x20,0x26,0xE0,0x24,0x68,0x22,0xE1,0x80,0x4D,0x20,0x08,0xE0,0x24,0x38,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x57,0x00,0x22,0xD8,0x22,0xA2,0xE0,0x06,0x00,0x22,0xF0,0x82,0xBD,0xE0,0x00,0x00,0x20,0x11,0x26,0x5C,0xEC,0x1E,0x56,0x20,0x00,0x20,0x50,0x80,0x3B,0x20,0x08,0x20,0x0E,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x93,0x00,0xE3,0x03,0x11,0x22,0xF6,0x82,0xC6,0x20,0x00,0x2F,0x68,0x23,0x02,0xE0,0x1E,0xB9,0x22,0xF6,0x20,0x32,0x20,0x00,0x20,0x08,0xE0,0x1E,0x32,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x6F,0x00,0x22,0xFF,0x20,0x00,0x80,0x7D,0x20,0x08,0x22,0xC0,0x80,0x00,0x22,0xFC,0xE5,0xFF,0xFF,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x09,0x00,0x23,0x38,0x86,0x3B,0x20,0x05,0x83,0x3E,0x80,0x00,0x20,0x17,0xE0,0x09,0x2C,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xBD,0x00,0x22,0xF0,0x28,0xFF,0xE0,0x06,0x00,0xE3,0x03,0x0E,0xE6,0x24,0x50,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x99,0x00,0x25,0xEA,0x23,0x02,0x20,0x00,0x22,0xF3,0xE3,0x06,0x05,0x80,0x00,0x38,0xAA,0x38,0x56,0xE0,0x99,0xC8,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x27,0x00,0xE3,0x00,0x08,0x80,0x00,0xE3,0x00,0x0E,0x2F,0x50,0x3E,0x56,0xEC,0x21,0x53,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xAE,0x00,0xE3,0x03,0x0E,0xE9,0xFF,0x02,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xE1,0x00,0xEF,0x03,0x65,0x80,0x00,0x20,0x11,0xE0,0xE1,0xFE,0xE0,0xFF,0x00,0xE0,0xF0,0x00,0x22,0xED,0x82,0xF6,0xE0,0x00,0x00,0x20,0x11,0xE1,0xF0,0x0D,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x4E,0x00,0x24,0x6A,0x84,0x76,0x20,0x08,0xE0,0x4E,0x62,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x81,0x00,0x22,0xF3,0x82,0xFC,0x20,0x00,0x20,0x0B,0x80,0x98,0x20,0x08,0xE0,0x03,0x0E,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xC3,0x00,0x22,0xEA,0x20,0x00,0x80,0xD1,0x20,0x08,0x22,0xF6,0xE0,0x00,0x00,0x20,0x0E,0x80,0x17,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xCF,0x00,0x22,0xF0,0x82,0xFC,0x20,0x08,0xE0,0xCF,0xE3,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0xFF,0x00,0xE0,0x9D,0x00,0x01,0xF0,0xF0,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x25 };

	Packet result(0x42);
	result.append(captCHA, 3824);
	Send(&result);*/

	Packet result(0xC0);
	result << uint8(1) << uint8(2) << uint8(1);
	Send(&result);
}
#pragma endregion

int8 CUser::GetLoyaltySymbolRank()
{
	if (m_bPersonalRank > 100 && m_bPersonalRank <= 200 || m_bKnightsRank > 100 && m_bKnightsRank <= 200)
		return -1;

	return m_bKnightsRank <= m_bPersonalRank ? m_bKnightsRank : m_bPersonalRank <= m_bKnightsRank ? m_bPersonalRank : -1;
}
uint8 CUser::GetSymbol()
{
	if (m_bPersonalRank < 1 && m_bKnightsRank < 1)
		return 0;
	uint8 bRaking = 0;
	if (m_bPersonalRank < m_bKnightsRank)
	{
		if (m_bPersonalRank == 1)
			bRaking = 30;
		else if (m_bPersonalRank > 1 && m_bPersonalRank <= 4)
			bRaking = 31;
		else if (m_bPersonalRank > 4 && m_bPersonalRank <= 10)
			bRaking = 32;
		else if (m_bPersonalRank > 10 && m_bPersonalRank <= 40)
			bRaking = 33;
		else if (m_bPersonalRank > 40 && m_bPersonalRank <= 100)
			bRaking = 34;
		else if (m_bPersonalRank > 100 && m_bPersonalRank <= 200)
			bRaking = 35;
	}
	else if (m_bPersonalRank > m_bKnightsRank)
	{
		if (m_bKnightsRank == 1)
			bRaking = 20;
		else if (m_bKnightsRank > 1 && m_bKnightsRank <= 4)
			bRaking = 21;
		else if (m_bKnightsRank > 4 && m_bKnightsRank <= 10)
			bRaking = 22;
		else if (m_bKnightsRank > 10 && m_bKnightsRank <= 40)
			bRaking = 23;
		else if (m_bKnightsRank > 40 && m_bKnightsRank <= 100)
			bRaking = 24;
		else if (m_bKnightsRank > 100 && m_bKnightsRank <= 200)
			bRaking = 25;
	}
	else if (m_bPersonalRank == m_bKnightsRank)
	{
		if (m_bKnightsRank == 1)
			bRaking = 20;
		else if (m_bKnightsRank > 1 && m_bKnightsRank <= 4)
			bRaking = 21;
		else if (m_bKnightsRank > 4 && m_bKnightsRank <= 10)
			bRaking = 22;
		else if (m_bKnightsRank > 10 && m_bKnightsRank <= 40)
			bRaking = 23;
		else if (m_bKnightsRank > 40 && m_bKnightsRank <= 100)
			bRaking = 24;
		else if (m_bKnightsRank > 100 && m_bKnightsRank <= 200)
			bRaking = 25;
	}

	//g_pMain->SendHelpDescription(this, string_format("return %d", bRaking));
	return bRaking;
}
void CUser::GmListProcess(bool logout)
{

	Guard lock(g_pMain->m_GmListlock);
	if (isGM() || isGMUser())
	{
		if (logout)
		{
			if (std::find(g_pMain->m_GmList.begin(), g_pMain->m_GmList.end(), GetName()) == g_pMain->m_GmList.end())
				return;

			g_pMain->m_GmList.erase(std::remove(g_pMain->m_GmList.begin(), g_pMain->m_GmList.end(), GetName()), g_pMain->m_GmList.end());
		}
		else
		{
			if (std::find(g_pMain->m_GmList.begin(), g_pMain->m_GmList.end(), GetName()) != g_pMain->m_GmList.end())
				return;

			g_pMain->m_GmList.push_back(GetName());
		}
	}

	uint8 count = 0;
	Packet newpkt(WIZ_NOTICE, uint8(5));
	newpkt.DByte();
	newpkt << count;
	for (auto& itr : g_pMain->m_GmList)
		if (++count) newpkt << itr;
	newpkt.put(1, count);
	Send(&newpkt);
}

void CUser::SendChat(uint8 chattype, string msg, string Sender) {
	Packet chtpkt;
	ChatPacket::Construct(&chtpkt, chattype, msg.c_str(), Sender.c_str(), GetNation());
	Send(&chtpkt);
}

void CUser::SendLoginNotice()
{
	if (g_pMain->pServerSetting.WelcomeMsg.empty())
		return;

	std::string notice = GetName() + " , " + g_pMain->pServerSetting.WelcomeMsg;
	//std::string msg = string_format("%s , Welcome to First Myko, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to Maxi-KO, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to MemoryKO, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to KOFilozof, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to MykoNetwork, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to Ko4Ever, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to MykoZone, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to Xydonis Empire, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to HomekoGame, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to BorderKO, enjoy your stay.", GetName().c_str());
	//std::string msg = string_format("%s , Welcome to Myko World, enjoy your stay.", GetName().c_str());
	Packet result(WIZ_NOTICE_SEND);
	result << notice;
	Send(&result);
}


void CUser::goDisconnect(std::string log /*= ""*/, std::string funcname /*= ""*/)
{
	if (log.size() && funcname.size())
	{
		Packet newpkt(0);
		newpkt << std::string("DISCONNECT");
		newpkt << string_format("'%s','%s',GETDATE(),'%s','%s'", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(),
			regex_quotes(funcname).c_str(), regex_quotes(log).c_str());
		g_pMain->AddLogRequest(newpkt);
	}

	Disconnect();
}

bool CUser::dupplicate_itemcheck(uint64 serial)
{
	if (!serial)
		return true;

	Guard lock(m_serialitemlistLock);
	if (std::find(m_serialitemlist.begin(), m_serialitemlist.end(), serial) != m_serialitemlist.end())
		return true;

	m_serialitemlist.push_back(serial);
	return false;
}
//void CUser::CharacterOlduguYerdeYenile() //Write Gkhn
//{
//	SendMyInfo();
//	UserInOut(INOUT_OUT);
//	SetRegion(GetNewRegionX(), GetNewRegionZ());
//	UserInOut(INOUT_WARP);
//	g_pMain->UserInOutForMe(this);
//	NpcInOutForMe();
//	g_pMain->MerchantUserInOutForMe(this);
//	ResetWindows();
//	InitType4();
//	RecastSavedMagic();
//}

void CUser::CharacterOlduguYerdeYenile()
{
	SendMyInfo();
	UserInOut(INOUT_OUT);
	SetRegion(GetNewRegionX(), GetNewRegionZ());
	UserInOut(INOUT_WARP);
	g_pMain->RegionUserInOutForMe(this);
	g_pMain->RegionNpcInfoForMe(this);
	g_pMain->MerchantUserInOutForMe(this);
	ResetWindows();
	InitType4();
	RecastSavedMagic();
}
// xtreme ekleme 05.02.2024

void CUser::ClanPremiumBonusNotice(bool exits)
{
	std::string noticeMessage;

	if (GetClanPremium() > 0 && GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumExpRestorePercent) > 0) {
		noticeMessage += string_format("ExpLostPercent +%d", GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumExpRestorePercent));
	}

	if (GetClanPremium() > 0 && GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent) > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("NoahPercent +%d", GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent));
	}

	if (GetClanPremium() > 0 && GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumDropPercent) > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("DropPercent +%d", GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumDropPercent));
	}

	if (GetClanPremium() > 0 && GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty) > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("BonusLoyalty +%d", GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty));
	}

	if (GetClanPremium() > 0 && GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumRepairDiscountPercent) > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("RepairPercent +%d", GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumRepairDiscountPercent));
	}

	if (GetClanPremium() > 0 && GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumItemSellPercent) > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("SellPercent +%d", GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumItemSellPercent));
	}

	if (GetClanPremium() > 0 && GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumExpPercent) > 0) {
		if (!noticeMessage.empty()) {
			noticeMessage += "|";
		}
		noticeMessage += string_format("ExpPercent +%d", GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumExpPercent));
	}
	uint8 Type = 1;
	if (exits)
		Type = 2;

	Packet notice(WIZ_NOTICE);
	notice.DByte();
	notice << uint8(4) << uint8(Type) << "Clan Premium Bonus" << noticeMessage;
	Send(&notice);
}

void CUser::SlaveMerc(Packet& pkt) {

	uint8 sOpCode;
	pkt >> sOpCode;
	if (sOpCode == 1) //UPDATE
	{
		g_DBAgent.LoadSlave(GetName());

		//uint32 sellinItem[12], sellinItemCount[12];

		for (int i = 0; i < 12; i++)
			SlaveItemID[i];
		for (int i = 0; i < 12; i++)
			SlaveItemCount[i];



		Packet Update(XSafe, uint8(XSafeOpCodes::PL_USER_TOOLS));
		Update << uint8(1) << uint8(3) << SlaveTotalKC << SlaveTotalCoins;
		for (int i = 0; i < 12; i++)
			Update << SlaveItemID[i] << SlaveItemCount[i];
		for (int i = 14; i < 42; i++)
			Update << m_sItemArray[i].nNum << m_sItemArray[i].sCount;

		Send(&Update);
	}
	else if (sOpCode == 2)// give coins&kc back
	{
		uint32 kc = 0, coin = 0; // çekilen miktar
		uint32 total_kc, total_coin; // geriye kalan
		pkt >> total_kc >> total_coin;

		if (total_kc == 0 && total_coin == 0)
			SendBoxMessage(1, "Uyarı", "Lütfen çekmek istediğiniz tutardan 1 eksik yazınız", 0, messagecolour::red);

		if (total_kc > 0 && total_coin <= 0) {
			kc = SlaveTotalKC - total_kc;
			coin = 0;
		}
		else if (total_kc <= 0 && total_coin > 0) {
			kc = 0;
			coin = SlaveTotalCoins - total_coin;
		}

		if (kc > 0 && coin <= 0) {
			GiveBalance(kc);
			g_DBAgent.RemoveSlaveKC(GetName(), total_kc);
		}
		else if (kc <= 0 && coin > 0) {
			GoldGain(coin);
			g_DBAgent.RemoveSlaveCoins(GetName(), total_coin);
		}
	}
	else if (sOpCode == 3)// give coins&kc back
	{
		uint8 secondOp;
		pkt >> secondOp;

		if (secondOp == 0)
		{
			if (CheckExistItem(751730000, 1))
			{
				//	RobItem(751730000, 1); item üstnüde olsa yeterli?

				SlaveTotalTime++;
				Packet slaveUpTime(XSafe, uint8(XSafeOpCodes::PL_USER_TOOLS));
				slaveUpTime << uint8(1) << uint8(7) << uint8(1);
				Send(&slaveUpTime);
				//g_pMain->SendHelpDescription(this, string_format("Using Slave Scrol - Time %d", SlaveTotalTime));
			}
			else
			{
				g_pMain->SendHelpDescription(this, "Error : You dont have a Slave Scrol");
			}
		}
		else if (secondOp == 1)
		{
			SlaveTotalTime = 0;
		}
	}
	else if (sOpCode == 4) // send item to inventory
	{
		for (int i = 0; i < 14; i++)
		{
			if (!GiveItem("SlaveMerc", SlaveItemID[i], SlaveItemCount[i]))
				g_pMain->SendHelpDescription(this, "Error : Item could not be added");
		}
	}
	else if (sOpCode == 5)
	{
		uint8 isCashOrCoin, result;
		uint32 coins2, COINS;

		pkt >> COINS >> isCashOrCoin;

		coins2 = COINS;
		result = isCashOrCoin;

		Packet Update(XSafe, uint8(XSafeOpCodes::PL_USER_TOOLS));
		Update << uint8(1) << uint8(5) << coins2 << result;
		Send(&Update);
	}
	else if (sOpCode == 6)
	{
		uint8 CanIopenMerchant = 0;
		pkt >> CanIopenMerchant;

		if (CanIopenMerchant == 1)
		{
			isSlave = CanIopenMerchant; // 1 oluyor
		}
		else if (CanIopenMerchant == 0)
		{
			isSlave = CanIopenMerchant; // 0 oluyor
		}

		if (isSlave == 0) {

			CBot* pSlaveUser = g_pMain->m_MapBotList.GetData(m_bSlaveUserID);
			if (pSlaveUser != nullptr) {
				GiveSlaveMerchantItems();
				pSlaveUser->UserInOut(INOUT_OUT);
			}

			for (int i = 0; i < 12; i++)
				SlaveItemID[i] = 0;
			for (int i = 0; i < 12; i++)
				SlaveItemCount[i] = 0;

		}

	}
}


void CUser::CyberACS_SlavePriest(Packet& pkt)
{
	uint8 oppcode, yuzde, buff, ac, heal;


	pkt >> oppcode;

	if (oppcode == 1)
	{
		pkt >> yuzde >> buff >> ac >> heal;


		HealYuzde = yuzde;
		Buffbool = buff;
		AccBool = ac;
		HealBool = heal;

		_PARTY_GROUP* pParty = nullptr;

		if (HealYuzde == 0)
		{
			g_pMain->SendHelpDescription(this, string_format("Please Set Priest Options"));
			return;
		}


		if (m_sPartyIndex != NULL)
			pParty = g_pMain->GetPartyPtr(GetPartyID());

		if (!isPriestUseZone())
		{
			if (m_bUserPriestBotID == -1 && pParty == nullptr)
			{
				g_pMain->SpawnPriestBot(GetSavedMagicDuration(495146), this);
			}
			else if (m_bUserPriestBotID == -1 && pParty != nullptr)
			{
				int ToplamPriest = 0;

				for (int i = 0; i < MAX_PARTY_USERS; i++)
				{
					CUser* User = g_pMain->GetUserPtr(pParty->uid[i]);
					if (User == nullptr)
						continue;

					CBot* pPriest = nullptr;
					pPriest = g_pMain->m_MapBotList.GetData(User->m_bUserPriestBotID);


					if (pPriest != nullptr)
						ToplamPriest++;
				}


				if (ToplamPriest == 0)
				{
					g_pMain->SpawnPriestBot(GetSavedMagicDuration(495146), this);
				}
				else
				{
					Packet result2(PL_MESSAGE);
					result2 << uint8(0) << "Error" << "There is a priest in the Party.";
					Send(&result2);
					return;
				}

			}
		}
		else if (isPriestUseZone())
		{
			Packet result2(PL_MESSAGE);
			result2 << uint8(0) << "Error" << "You cannot use this here.";
			Send(&result2);
			return;
		}
	}
	else if (oppcode == 2)
	{
		pkt >> yuzde >> buff >> ac >> heal;

		HealYuzde = yuzde;
		Buffbool = buff;
		AccBool = ac;
		HealBool = heal;

		if (hasPriestBot == true)
		{
			CBot* pPriest = nullptr;
			pPriest = g_pMain->m_MapBotList.GetData(m_bUserPriestBotID);
			if (pPriest)
				pPriest->UserInOut(INOUT_OUT);

			m_bUserPriestBotID = -1;
			hasPriestBot = false;
		}
	}

}

uint32 saathesap()
{

	DateTime gun;

	return ((gun.GetHour() * 60 * 60) + (gun.GetMinute() * 60) + gun.GetSecond());
}

void CUser::OpenAkara(Packet& pkt)
{
	uint8 OpCode = 0;

	pkt >> OpCode;

	if (OpCode == 1)
	{
		std::vector<AKARA_LIST> AkaraList;

		DateTime gun;

		uint32 LastSeconds = 0;
		LastSeconds = uint32(86400 - saathesap());

		Packet result(WIZ_AKARA);
		result << uint8(1)
			<< uint8(1)
			<< uint8(0)
			<< uint32(0x14)
			<< uint32(1)
			<< uint8(1)
			<< uint32(LastSeconds);//bitmesine geri kalan süre

		uint8 liste;
		if (gun.GetDay() % 14 == 0)
			liste = 1;
		else
			liste = gun.GetDay() % 14;

		g_DBAgent.LoadAkaraList(AkaraList);
		if (AkaraList.empty())
			return;

		result << uint8(liste)
			<< uint8(8);

		uint8 counter = 0;
		result.SByte();
		bool isTrue = false;
		for (int i = 0; i < 3; i++)
		{
			if (AkaraList.at(liste - 1).ItemNum[i] > 0)
			{
				if (isTrue == true)
				{
					result << uint8(i);
				}
				else
				{
					result << uint8(0)
						<< uint8(i);
				}
				result << uint32(AkaraList.at(liste - 1).ItemNum[i]);//item num
				if (AkaraList.at(liste - 1).BetMax[i] > AkaraList.at(liste - 1).Bet[i])
				{
					result << uint32(AkaraList.at(liste - 1).BetMax[i]);//item parasi
					result << uint32(0);
					result << AkaraList.at(liste - 1).Owner[i];
					isTrue = true;
				}
				else
				{
					result << uint32(AkaraList.at(liste - 1).Bet[i]);//item parasi
					result << uint32(0);
					isTrue = false;
				}



				counter++;
			}
		}

		for (int i = counter; i < 8; i++)
		{
			result << uint8(0)
				<< uint8(i)
				<< uint32(0)//item num
				<< uint32(0)//item parasi
				<< uint32(0);//şu anki teklif

		}
		result << uint8(0);

		Send(&result);
	}
	else if (OpCode == 2)
	{
		uint8 Nothing, Sira;//
		uint16 Nothing2;
		uint32 ItemID;
		uint32 MyBet, MaxBet, Nothing3;
		pkt >> Sira >> ItemID >> Nothing >> Nothing2
			>> MyBet >> MaxBet >> Nothing3;

		std::vector<AKARA_LIST> AkaraList;

		DateTime gun;

		uint8 liste;
		if (gun.GetDay() % 14 == 0)
			liste = 1;
		else
			liste = gun.GetDay() % 14;

		g_DBAgent.LoadAkaraList(AkaraList);//
		if (AkaraList.empty())
			return;

		if (!hasCoins(MyBet))
			return;

		if ((AkaraList.at(liste - 1).ItemNum[Sira] == ItemID && MyBet > AkaraList.at(liste - 1).BetMax[Sira]) || (AkaraList.at(liste - 1).Owner[Sira] == "" && MyBet >= AkaraList.at(liste - 1).BetMax[Sira]))
		{
			if (MyBet > AkaraList.at(liste - 1).BetMax[Sira] && AkaraList.at(liste - 1).Owner[Sira] != "" && AkaraList.at(liste - 1).Owner[Sira] != GetName())
			{
				g_DBAgent.UpdateUserAkaraHistory(liste, Sira, AkaraList, 4);
			}
			g_DBAgent.UpdateAkaraList(GetID(), AkaraList.at(liste - 1).ItemNum[Sira], MyBet, Sira, liste);

			Packet result(WIZ_AKARA, uint8(2));
			result << uint8(1) << uint8(0) << uint8(0xFF) << uint8(0xFF);

			Send(&result);

			GoldLose(MyBet);
		}
	}
	else if (OpCode == 7)
	{
		Packet result(WIZ_AKARA, uint8(7));
		result << uint16(14);
		std::vector<ByteBuffer> AkaraList;
		g_DBAgent.LoadAkaraHistory(AkaraList);

		uint32* listeNum = new uint32[AkaraList.size()];
		uint32* sira = new uint32[AkaraList.size()];
		uint32* ItemNum = new uint32[AkaraList.size()];
		uint32* MaxBet = new uint32[AkaraList.size()];
		for (int i = 0; i < AkaraList.size(); i++)
		{
			AkaraList.at(i) >> listeNum[i] >> sira[i]
				>> ItemNum[i] >> MaxBet[i];
		}

		for (int i = 0; i < AkaraList.size(); i++)
		{
			for (int b = 0; b < AkaraList.size(); b++)
			{
				if (listeNum[b] == (i + 1))
				{
					if (i == 0 && b == 0)
					{
						result << uint32(ItemNum[b])
							<< uint32(MaxBet[b])
							<< uint32(0);
					}
					else
					{
						result << uint8(3)
							<< uint32(ItemNum[b])
							<< uint32(MaxBet[b])
							<< uint32(0);
					}
				}
			}
		}
		Send(&result);
	}
	else if (OpCode == 4)
	{
		std::vector<AKARA_LIST> AkaraList;

		DateTime gun;

		uint8 liste;
		if (gun.GetDay() % 14 == 0)
			liste = 1;
		else
			liste = gun.GetDay() % 14;

		g_DBAgent.LoadAkaraList(AkaraList);//
		if (AkaraList.empty())
			return;

		std::vector<ByteBuffer> List;
		g_DBAgent.LoadUserAkaraHistory(List, GetName());
		if (List.empty())
			return;


		uint32* listeNum = new uint32[List.size()];
		uint32* sira = new uint32[List.size()];
		uint32* ItemNum = new uint32[List.size()];
		uint32* MaxBet = new uint32[List.size()];
		if (List.size() > 0)
		{
			for (int i = 0; i < List.size(); i++)
			{
				List.at(i) >> listeNum[i] >> sira[i]
					>> ItemNum[i] >> MaxBet[i];
			}
		}

		if (AkaraList.size() > 0)
		{
			Packet result(WIZ_AKARA, uint8(4));
			result << uint16(1) << uint8(1) << uint32(1);
			uint8 counter = 0;
			for (int i = 0; i < AkaraList.size(); i++)
			{
				for (int b = 0; b < 3; b++) {
					if (AkaraList.at(i).Owner[b] == GetName())
					{
						result << uint8(3) << uint8(0) << AkaraList.at(i).ItemNum[b]
							<< uint16(1)
							<< AkaraList.at(i).BetMax[b]
							<< uint8(0)
							<< AkaraList.at(i).BetMax[b]
							<< uint32(0) << uint8(1);
						counter++;
					}
				}

			}
			//result << uint8(1);
			if (List.size() > 0)
			{
				for (int i = 0; i < List.size(); i++)
				{
					result << uint16(3)
						<< ItemNum[i]
						<< uint16(1)
						<< MaxBet[i]
						<< uint8(0)
						<< MaxBet[i]
						<< uint32(0);
				}
			}
			Send(&result);
		}
		else
		{
			Packet result(WIZ_AKARA, uint8(4));
			result << uint8(0) << uint8(0) << uint8(0);
			Send(&result);
		}
	}
	return;
}
