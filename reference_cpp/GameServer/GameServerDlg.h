#pragma once

#include "resource.h"
#include "LuaEngine.h"

#include "Define.h"
#include "ChatHandler.h"

class C3DMap;
class CUser;
class CBot;
#include "../shared/HardwareInformation.h"
#include "LoadServerData.h"
#include "CBot.h"
#include "User.h"
#include "NpcThread.h"
#include "../shared/ClientSocketMgr.h"
#include "MagicProcess.h"
#include "curl/curl/curl.h"
#include "json.hpp"
#include "DBAgent.h"

typedef std::map<std::string, CUser*>  NameMap;
typedef std::map<std::string, CBot*>  BotNameMap;
typedef  std::map<uint16, uint16>          ForgettenTempleMonsterList;
typedef	std::map<uint16, uint16>		   UnderTheCastleMonsterList;
typedef std::vector<uint16> AntiAfkList;
typedef std::vector<std::string> GmList;
typedef std::vector<timershow> timershowlist;
typedef std::map<std::string, _USER_INFOSTORAGE> UserInfoStorMap;

using json = nlohmann::json;

class CGameServerDlg
{
public:
	bool JstKODetailedLogSystem;
	void ServerLog(const char* category, const char* fmt, ...);
	struct StreamUser
	{
		std::string displayName;
		std::string userName;
		uint32 streamMinute;
	};

	struct find_user
	{
		std::string displayName;
		find_user(std::string displayName) : displayName(displayName) {}
		bool operator () (const StreamUser& m) const
		{
			return m.displayName == displayName;
		}
	};

	struct find_user_nick
	{
		std::string nick;
		find_user_nick(std::string nick) : nick(nick) {}
		bool operator () (const StreamUser& m) const
		{
			return m.userName == nick;
		}
	};

	const bool ignoreStreamHistory = true;
	const uint32 rewardPerMinute = 1; // trovo ka� dakikada bir �d�l verecek
	const uint32 rewardKC = 5; // trovo ka� kc �d�l verecek
	const std::string serverName = "Tigerko"; // trovo server ismi (yay�n ba�l���nda)

	std::vector<StreamUser> streamUsers;

	const std::string api = "https://api-web.trovo.live/graphql?chunk=1&reqms=1669714810390&qid=bc91b24fff75e716&cli=4";
	const std::string query = "[{\"operationName\":\"live_LiveReaderService_GetCategoryDetail\",\"variables\":{\"params\":{\"pageSize\":50,\"currPage\":1,\"categoryShortName\":\"Knight\",\"sortType\":null,\"tagList\":[],\"sessionID\":\"63b4aa3aa0df260001515cd2\"}},\"extensions\":{\"persistedQuery\":{\"version\":1,\"sha256Hash\":\"b1458a913bbc4861848815ec63d1565bbfb4af0e9e9d2269d4ad78fdb41b5434\"}}},{\"operationName\":\"getRecommandTagInfo\",\"variables\":{\"params\":{\"sessionID\":\"\",\"recommNum\":5,\"categoryName\":\"Knight\",\"scene\":\"ERecommSceneBrowsePageRecomended\"}},\"extensions\":{\"persistedQuery\":{\"version\":1,\"sha256Hash\":\"b1458a913bbc4861848815ec63d1565bbfb4af0e9e9d2269d4ad78fdb41b5434\"}}}]";
	static size_t WriteCallback(void* contents, size_t size, size_t nmemb, void* userp);
	std::vector <uint32> RightClickExchangeItemList;
	bool startsWithCaseInsensitive(std::string mainStr, std::string toMatch);
	bool getTwoPoint(const std::string& s, const std::string& start_delim, const std::string& stop_delim, std::string& out);
	//uint32 StreamPassedTime(std::string display_name);
	void UserCallback(StreamUser u, uint32 multipier = 1);
	void TrovoLive();
	std::string server_itemorg, server_skillmagic, server_zones, server_skillmagictk;
	bool m_randomtable_reload;

	void SpecialEventProcess(CUser* pUser, uint8 zoneid, bool closed = false);
	_SPEVENT pSpecialEvent;

	bool LoadCindirellaItemsTable();
	bool LoadCindirellaStatSetTable();
	bool LoadCindirellaSettingTable();
	bool LoadCindirellaRewardsTable();
	bool LoadLevelRewardTable();
	bool LoadJackPotSettingTable();
	bool LoadPerksTable();
	bool LoadRankRewardTable();
	bool LoadLevelMerchantRewardTable();
	void SendGameNotice(
		ChatType chattype,
		std::string n = "",
		std::string sender = "",
		bool send_all = true,
		CUser* pUser = nullptr,
		bool anontip = true,
		uint8 zoneid = 0);
	bool LoadCSWTables();

	void UnderTheCastleTimerProc(); 
	void TheCastellanTimerProc(); 

	void ForgettenTempleTimerProc();
	void ForgettenTempleReset();
	void FtFinish();
	void ForgettenTempleSendItem();
	void ForgettenTempleStart(uint8 Type, uint8 MinLevel, uint8 MaxLevel);
	_FORGETTEN_TEMPLE_STAGES ForgettenTempleGetStage();
	std::vector< _FORGETTEN_TEMPLE_SUMMON> ForgettenTempleLoadSpawn();

	_FORGETTEN_TEMPLE pForgettenTemple; 
	_UNDER_THE_CASTLE pUnderTheCastle; 
#if(GAME_SOURCE_VERSION == 2369) 
	_CASTELLAN_WAR pCastellanWar;
#endif
	
	bool m_bisDamage;
	bool LoadMonsterItemTable();
	bool LoadCharacterSetValidTable();
	bool LoadNpcItemTable();
	bool LoadZoneKillReward();
	bool LoadMakeItemGroupTable();
	bool LoadMakeItemGroupRandomTable();
	bool LoadMakeWeaponItemTableData();
	bool LoadMakeDefensiveItemTableData();
	bool LoadMakeGradeItemTableData();
	bool LoadMakeLareItemTableData();
	bool LoadNpcTableData(bool bNpcData = true);
	bool LoadNpcPosTable();
	bool LoadDrakiTowerTables();
	bool LoadServerSettingsData();
	bool LoadGiftEventArray();
	bool LoadSendMessageTable();
	bool LoadRightTopTitleTable();
	bool LoadTopLeftTable();
	bool _CreateNpcThread();
	bool _LoaderSpawnCallBack(OdbcCommand* dbCommand);
	bool CreateBottomZoneMap();
	bool LicenseSystem();
	bool LoadCollectionRaceEventTable();
	bool LoadRimaLotteryEventTable();
	bool LoadDailyQuestListTable();

	std::atomic_int32_t m_iItemUniqueID;
	PusCategoryArray						m_PusCategoryArray;
	AdiniFerihaKoydum* m_afkqueue;

	ULONGLONG DelayedTime;

	// NPC Related Variables
	Atomic<uint16>	m_TotalNPC;				// the atomic variable that holds total NPC number
	Atomic<uint16>	m_CurrentNPC;			// the atomic variable that holds total NPC number alive
	uint16			m_sTotalMap;			// Total number of Zone
	bool			g_bNpcExit;

	CGameServerDlg();
	bool Startup();
	void Initialize();
	bool StartUserSocketSystem();
	bool StartTheardSocketSystem();
	bool LoadTables();
	void CreateDirectories();

	void GetTimeFromIni();
	void GetEventAwardsIni();
	void JstKOServerSettings(); // JstKO Files Defter Ayarlar Eklendi 01.01.2025

	bool ChaosStoneLoad();
	bool RandomBossSystemLoad();
	_MONSTER_BOSS_RANDOM_SPAWN GetRandomIndex(uint16 Stage, uint16 MonsterID, uint16 MonsterZone);
	void ChaosStoneRespawnTimer();
	void ChaosStoneSummon(uint16 ChaosGetID, uint8 RankID, uint16 ZoneID);
	uint8 ChaosStoneSummonSelectStage(uint16 ChaosGetID, uint8 RankID, uint16 ZoneID);
	uint8 ChaosStoneSummonSelectFamilyStage(uint8 ChaosIndex, uint8 FamilyID, uint16 ZoneID);

	bool CindirellaCommand(bool close, int8 settingid, CUser* pUser = nullptr);
	void CindirellaTimer();
	void CindirellaTown();
	void CindirellaStart();
	uint32 CindirellaGetTime();
	void JstKOAutoOnlineCount(); // JstKO Yeni Auto Online Count Eklendi 01.01.2025
	void JstKOAddBotMiningStart(); // JstKO Bot Mining Otomatik Ba�ladi Eklendi. 23.02.2025
	bool LoadDailyRankTable();
	bool LoadBotTable();
	bool SpawnMorankerRankBots();
	void ClearMorankerRankBots();
	uint16 GetMorankerDisplayID(uint8 slotID);
	bool IsMorankerDisplayID(uint16 id);
	void BroadcastMorankerBotToZone(CBot* pBot);
	bool LoadBotMerchantTable();
	bool LoadDungeonDefenceMonsterTable();
	bool LoadDungeonDefenceStageTable();
	bool LoadKnightReturnLetterGiftItemTable();
	bool LoadAchieveTitleTable();
	bool LoadAchieveWarTable();
	bool LoadAchieveMainTable();
	bool LoadAchieveMonsterTable();
	bool LoadAchieveNormalTable();
	bool LoadAchieveComTable();
	bool LoadRandomBossTable();
	bool LoadRandomBoosStageTable();
	bool LoadItemTable();
	bool LoadWheelDataSetTable();
	bool LoadItemSellTable();
	bool LoadZoneOnlineRewardTable();
	bool XCodeLoadEventTables();
	bool XCodeLoadEventVroomTables();
	bool LoadItemPremiumGiftTable();
	bool LoadSetItemTable();
	bool LoadItemExchangeTable();
	bool LoadItemExchangeExpTable();
	bool LoadItemSpecialExchangeTable();
	bool LoadItemExchangeCrashTable();
	bool LoadItemUpgradeTable();
	bool LoadItemOpTable();
	bool LoadServerResourceTable();
	bool LoadQuestHelperTable();
	bool LoadQuestMonsterTable();
	bool LoadMagicTable();
	bool LoadMagicType1();
	bool LoadMagicType2();
	bool LoadMagicType3();
	bool LoadMagicType4();
	bool LoadMagicType5();
	bool LoadMagicType6();
	bool LoadMagicType7();
	bool LoadMagicType8();
	bool LoadMagicType9();
	bool LoadRentalList();
	bool LoadCoefficientTable();
	bool LoadCoefficientDamageTable();
	bool LoadLevelUpTable();
	bool LoadAllKnights();
	bool LoadKnightsAllianceTable(bool bIsslient = false);
	bool LoadKnightsSiegeWarsTable(bool bIsslient = false);
	bool LoadUserRankings();
	bool LoadCharacterSealItemTable();
	bool LoadCharacterSealItemTableAll();
	void CleanupUserRankings();
	bool LoadKnightsCapeTable();
	bool LoadKnightsRankTable(bool bWarTime = false, bool bIsslient = false);
	bool LoadStartPositionTable();
	bool LoadStartPositionRandomTable();
	bool LoadBattleTable();
	bool LoadCapeCastellanBonus();
	bool LoadKingSystem();
	bool LoadSheriffTable();
	bool LoadMonsterResourceTable();
	bool LoadMonsterSummonListTable();
	bool LoadChaosStoneMonsterListTable();
	bool LoadChaosStoneCoordinateTable();
	bool LoadChaosStoneStage();
	bool LoadForgettenTempleStagesListTable();
	bool LoadForgettenTempleSummonListTable();
	bool LoadMonsterRespawnLoopListTable();

	bool LoadMonsterUnderTheCastleTable();
	bool LoadMonsterStoneListInformationTable();
	bool LoadJuraidMountionListInformationTable();
	bool LoadMiningFishingItemTable();
	bool LoadPremiumItemTable();
	bool LoadPremiumItemExpTable();
	bool LoadUserDailyOpTable();
	bool LoadEventTriggerTable();
	bool LoadMonsterChallengeTable();
	bool LoadMonsterChallengeSummonListTable();
	bool LoadMiningExchangeListTable();
	bool LoadUserItemTable();
	bool LoadPusCategoryTable();
	bool LoadSaveBotDataTable();
	bool LoadObjectPosTable();
	bool LoadBanishWinnerTable();
	bool LoadGiveItemExchangeTable();
	bool LoadBeefEventPlayTimerTable();
	bool LoadPetStatsInfoTable();
	bool LoadPetImageChangeTable();
	bool LoadPusItemsTable();
	bool LoadUserLootSettingsTable();
	bool LoadZindanWarStageListTable();
	bool LoadZindanWarSummonListTable();
	bool LoadFtEventPlayTimerTable();
	bool LoadAutomaticCommand();
	bool LoadTimedNoticeTable();
	bool LoadAntiAfkListTable();
	bool LoadEventTimerShowTable();
	bool LoadItemRightClickExchangeTable();
	bool LoadDamageSettingTable();
	bool LoadRankBugTable();
	bool LoadEventRewardTable();

	bool MapFileLoad();
	bool LoadNoticeData();
	bool LoadNoticeUpData();
	bool GameStartClearRemainUser();
	bool CheckTrashItemListTime();
	bool ReLoadItemTable();
	bool king_selectreq;
	void king_loyaltyselection(); //Write Gkhn
	uint32 KingRewardKC;

	bool BurningFeatureTable();

	static uint32 THREADCALL Timer_CheckGameEvents(void* lpParam);
	static uint32 THREADCALL Timer_UpdateGameTime(void* lpParam);
	static uint32 THREADCALL Timer_UpdateSessions(void* lpParam);
	static uint32 THREADCALL Timer_BotSessions(void* lpParam);
	static uint32 THREADCALL Timer_UpdateConcurrent(void* lpParam);
	static uint32 THREADCALL Timer_TimedNotice(void* lpParam);
	static uint32 THREADCALL Timer_t_1(void* lpParam);
	static uint32 THREADCALL Timer_t_2(void* lpParam);
	static uint32 THREADCALL Timer_t_3(void* lpParam);
	static uint32 THREADCALL Timer_BotMoving(void* lpParam);
	//static uint32 THREADCALL Timer_BotAutoSpawn(void* lpParam);

	void BotHandlerMainTimer();
	uint16 SpawnEventAfkBotHandler(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel);
	uint16 SpawnEventBotMining(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel);
	uint16 SpawnEventBotMerchant(int Minute, uint8 byZone, float fX, float fY, float fZ, int16 nDir, uint8 minlevel);
	uint16 SpawnEventBotFishing(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel);
	uint16 SpawnEventBotFarm(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel, uint8 sPartyLider, uint8 sGenie, uint8 sClass, uint8 maxlevel = 83);
	uint16 SpawnEventBotMoveProcess(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel);
	uint16 SpawnEventBotPk(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel, uint8 sNation, uint8 sClass, CUser* pUser = nullptr, bool startup = false);
	uint16 SpawnUserBot(int Minute = 1, uint8 byZone = 0, float fX = 0.0f, float fY = 0.0f, float fZ = 0.0f, uint8 Restipi = 5, uint8 minlevel = 1, int16 direction = 0, uint32 SaveID = 0, uint8 Class = 0, _bot_merchant _merchant = {});

	uint16 ClanBankGetID[50], ClanBankGetZone[50];
	uint32 m_checktime;
	SIZE_T GetProcessMemoryUsage();
	std::string	m_gLicenseNum, m_gLicenseCode;

	/* Collection Race */
	_COLLECTION_RACE_EVENT pCollectionRaceEvent;
	time_t autocrchecktime;

	CollectionRaceEventListArray m_CollectionRaceListArray;
	void CollectionRaceStart(_COLLECTION_RACE_EVENT_LIST* pList, uint16 index, CUser* pUser);
	void CollectionRaceTimer();
	void CollectionRaceSendDead(Unit* pKiller, uint16 ProtoID);
	void CollectionRaceCounter();
	void CollectionRaceEnd();
	void SetConsoleColor(WORD color);
	void CollectionRaceDataReset();
	void CollectionRaceSend(Packet* pkt);
	/* Collection Race */

	/* Lottery System*/
	_RIMA_LOTTERY_PROCESS pLotteryProc;
	_DAMAGE_SETTING pDamageSetting;
	EVENT_CHAOSREWARD pChaosReward[18];

	void LotterySystemStart(uint32 ID);
	void LotterySendGift();
	void LotteryEventLetterProcess(std::string Name, uint16 index);

	void LotterySystemReset();
	void LotteryEventTimer();

	void LogosYolla(std::string LogosName, std::string LogosMessage, uint8 R, uint8 G, uint8 B);
	void ReqUpdateConcurrent();
	void GameEventMainTimer();
	void SendFlyingSantaOrAngel();
	void BattleZoneCurrentUsers();
	void GetCaptainUserPtr();
	void Send_CommandChat(Packet* pkt, int nation, CUser* pExceptUser = nullptr);
	void SendItemZoneUsers(uint8 ZoneID, uint32 nItemID, uint16 sCount = 1, uint32 Time = 0);
	void SendItemUnderTheCastleRoomUsers(uint8 ZoneID, uint16 Room, float GetX, float GetZ);
	void KickOutZoneUsers(uint8 ZoneID, uint8 TargetZoneID = 0, uint8 bNation = 0);
	void SendItemEventRoom(uint16 nEventRoom, uint32 nItemID, uint16 sCount = 1);
	uint64 GenerateItemSerial();
	uint32 GenerateItemUniqueID();
	int KickOutAllUsers();
	int GetKnightsGrade(uint32 nPoints);
	//uint16 GetKnightsAllMembers(uint16 sClanID, Packet & result, uint16 & pktSize, bool bClanLeader);
	void GetUserRank(CUser* pUser);
	void Announcement(uint16 type, int nation = 0, int chat_type = 8, CUser* pExceptUser = nullptr, CNpc* pExpectNpc = nullptr);
	void ResetBattleZone(uint8 bType);
	void BanishLosers();
	void BattleZoneVictoryCheck();
	void BattleZoneOpen(int nType, uint8 bZone = 0);
	void BattleEventGiveItem(int nType);
	void BattleZoneRemnantSpawn();
	void BattleZoneClose();
	void BattleZoneResult(uint8 nation);
	void BattleWinnerResult(BattleWinnerTypes winnertype);
	void BattleZoneSelectCommanders();
	void BattleZoneResetCommanders();
	void EventMainTimer();
	void VirtualEventTimer();
	void VirtualEventOpen(uint8 id, EVENT_OPENTIMELIST a, int timei, int nhour, int nminute);
	void level_merchant_timer();
	void SingleLunarEventTimer();
	void SingleOtherEventTimer();
	void SingleOtherEventLocalTimer();
	bool isgetonday(bool mday[7]);
	int32 ZindanWarStageSelect();
	bool  ZindanWarLoadSpawn();

	bool WordGuardSystem(std::string Word, uint8 WordStr);

	void ChaosExpansionManuelOpening();
	void BorderDefenceWarManuelOpening();
	void JuraidMountainManuelOpening();
	void BeefEventManuelOpening();

	void BeefEventManuelClosed();
	void ChaosExpansionManuelClosed();
	void BorderDefenceWarManuelClosed();
	void JuraidMountainManuelClosed();

	void Send_PartyMember(int party, Packet* result);
	void Send_KnightsMember(int index, Packet* pkt);
	void Send_KnightsAlliance(uint16 sAllianceID, Packet* pkt);
	void SetGameTime();
	void ResetPlayerRankings();
	void UpdateWeather();
	void UpdateGameTime();
	void ResetLoyaltyMonthly();
	CNpc* FindNpcInZone(uint16 sPid, uint8 byZone);

	CNpc* GetPetPtr(int16 m_sPetID, uint8 byZone);

	void DrakiTowerLimitReset();
	void UnderTheCastleTimer();
	time_t	m_lastBlessTime;
	time_t	m_lastBorderTime;

	uint16 TempPlcCodeGameMasterSocket; // /plc komutunu cal�st�r�r ve kulland�g� programlar� g�sterir sol altta notice olarak 27.09.2020

	void UserAuthorityUpdate(BanTypes s, CUser* pmaster = nullptr, std::string targetid = "", std::string desc = "", uint32 period = 0);

	void ResetPlayerKillingRanking();
	void TrashBorderDefenceWarRanking();
	void TrashChaosExpansionRanking();

	void DungeonDefenceTimer();
	void SummonDungeonDefenceMonsters(uint16 EventRoom);
	void SendDungeonDefenceDetail(uint16 EventRoom);
	uint8 DungeonDefenceSelectStage(uint16 EventRoom);
	void DungeonDefenceUserisOut(uint16 EventRoom);

	void DrakiTowerRoomCloseTimer();
	void DrakiTowerRoomCloseUserisOut(uint16 EventRoom);
	uint16 MaxSpawnCount;
	uint16 MaxBotFinish;
	uint16 BotStepCount;
	DWORD BotRespawnTick;
	uint32 BotTimeNext;
	uint8 BotAutoType;
	int botautoX;
	int botautoZ;
	std::string	m_byBattleSiegeWarWinKnightsNotice;
	uint8_t	m_byBattleSiegeWarNoticeTime;
	time_t m_CSWTimer;
	uint8_t	m_byBattleSiegeWarType;
	int32_t	m_byBattleSiegeWarStartTime;
	int32_t	m_byBattleSiegeWarOccupyTime;
	int16_t	m_byBattleSiegeWarRemainingTime;
	bool	m_byBattleSiegeWarOpen;
	bool	m_byBattleSiegeWarAttack;
	bool	m_byBattleSiegeWarMonument;

	bool tar_levelmercreward, tar_levelreward;

	// Clan Buff System
	bool ClanBuffSystemStatus;
	uint8 ClanBuff5UserOnlineExp;
	uint8 ClanBuff10UserOnlineExp;
	uint8 ClanBuff15UserOnlineExp;
	uint8 ClanBuff20UserOnlineExp;
	uint8 ClanBuff25UserOnlineExp;
	uint8 ClanBuff30UserOnlineExp;
	uint8 ClanBuff5UserOnlineNP;
	uint8 ClanBuff10UserOnlineNP;
	uint8 ClanBuff15UserOnlineNP;
	uint8 ClanBuff20UserOnlineNP;
	uint8 ClanBuff25UserOnlineNP;
	uint8 ClanBuff30UserOnlineNP;
	// 29.10.2020 Boss At�nca Notice Gecmesi Engellendi start
	bool IsSummon;
	std::string NameGm;
	// 29.10.2020 Boss At�nca Notice Gecmesi Engellendi end
	bool ChaosStoneRespawnOkey;

	bool MerchantViewInfoNotice; //

	bool m_ZoneOnlineRewardReload;

	void Send_Noah_Knights(Packet* pkt);
	std::string GetBattleAndNationMonumentName(int16 TrapNumber = -1, uint8 ZoneID = 0);
	void CheckNationMonumentRewards();

	void NewWantedEventMainTimer();
	void NewWantedEventUserListSend(uint8 room);
	void NewWantedEventSelecting(uint8 room);
	void NewWantedEventResetData(uint8 room);
	void NewWantedEventFinishing(uint8 room);
	int8 wantedgetroom(uint8 zoneid);
	int8 wantedgetzone(uint8 room);

	void AddMapBotList(CBot* pSession);
	void RemoveMapBotList(int16 GetSocketID, std::string strUserName);

	void TemplEventJuraidSendJoinScreenUpdate(CUser* pUser = nullptr);
	void TemplEventBDWSendJoinScreenUpdate(CUser* pUser = nullptr);
	void TempleEventJoinScreenUpdateSend(Packet& pkt);
	void TemplEventChaosSendJoinScreenUpdate(CUser* pUser = nullptr);
	void TempleEventChaosShowCounterScreen();
	void TempleEventReset(EventOpCode s, bool opened = false);
	void TempleEventBridgesClose(uint16 room);
	void TempleEventKickOutUser(CUser* pUser);
	void TempleEventSendActiveEventTime(CUser* pUser);
	void TempleEventGetActiveEventTime(CUser* pUser);
	void TempleEventFinish(int16 nRoom = -1);
	void TempleEventTeleportUsers();
	void TempleEventStart();
	void TempleEventManageRoom();
	void TempleEventSendAccept();
	void TempleEventRoomClose();
	void TempleEventBridgeCheck(uint8 DoorNumber);
	void TempleEventSendWinnerScreen();
	void TempleEventCreateParties();
	bool AddEventUser(CUser* pUser);
	void RemoveEventUser(CUser* pUser);

	void ResetBeefEvent();
	void BeefEventUpdateTime();
	void BeefEventSendNotice(uint8 NoticeType);

	void ForgettenTempleManuelOpening(uint8 Type);
	void ForgettenTempleManuelClosed();

	uint8 GetCntNewRace(uint16 newclass, Nation newnation);

	void EventTimerSet();

	void GameInfoNoticeTimer();
	void GameInfo1Packet();
	void GameInfo2Packet();

	// JstKO WelcomeStatus Eklendi 01.01.2025
	std::string WelcomeNoticeYeni;
	uint8 WelcomeNoticeR, WelcomeNoticeG, WelcomeNoticeB;
	bool JstKOWelcomeStatus;
	// JstKO WelcomeStatus Eklendi The End 01.01.2025

	// JstKO Files Defter Eklendi 01.01.2025
	bool UserLoginEffect;
	bool UserDeathEffect;
	bool UserTownEffect;
	bool UserRiseEffect;
	bool UserZoneChangeEffect;
	bool AutoKingUpdateServerList;
	bool GirisUserisOnlineNotice;
	bool Giri�GateZoneNotice;
	bool AutoOnlineCountNotice;
	bool GmInfoKomutNotice;
	bool UserGameInfoKomutNotice;
	uint8 ClanCreateMinLevel;
	uint32 ClanCreateMinNationalPoints;
	uint32 ClanCreateCoins;
	uint32	KillGoldGift, KillCashGift, Kill�temGift;
	uint16 KillCountGift, Kill50CountGift, Kill500CountGift;
	uint32	Kill50GoldGift, Kill50CashGift, Kill50�temGift;
	uint32	Kill500GoldGift, Kill500CashGift, Kill500�temGift;
	// JstKO Files Defter Settings Eklendi The End 01.01.2025

	void UpdateFlagAndCape();

	//Soccer Event System
	void TempleSoccerEventTimer();
	void TempleSoccerEventGool(uint8 nType = 2);
	void TempleSoccerEventEnd();

	void ReloadKnightAndUserRanks(bool command);
	void SetPlayerRankingRewards(uint16 ZoneID);

	void UpdateSiege(int16 m_sCastleIndex, int16 m_sMasterKnights, int16 m_bySiegeType, int16 m_byWarDay, int16 m_byWarTime, int16 m_byWarMinute);
	void UpdateSiegeTax(uint8 Zone, int16 ZoneTarrif);
	void HandleSiegeDatabaseRequest(CUser* pUser, Packet& pkt);

	void ReqHandleDatabasRequest(Packet& pkt);
	void ReqGmCommandLetterGiveItem(Packet& pkt);
	void ReqLotteryReward(Packet& pkt);
	void ReqBotLoadSaveData(Packet& pkt);
	void ReqCollectionRaceStart(Packet& pkt);
	void AddDatabaseRequest(Packet& pkt, CUser* pUser = nullptr);
	void AddLogRequest(Packet& pkt);

	// The Under Castle & Krowas Dominion
	void TheEventUserKickOut(uint8 ZoneID = 0);

	// Get info for NPCs in region
	void GetRegionNpcIn(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom, CUser* pSendUser);

	// Get list of NPC IDs in region
	void GetRegionNpcList(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom = 0);

	// Get list of NPCs for regions around user (WIZ_NPC_REGION)
	void RegionNpcInfoForMe(CUser* pSendUser);

	// Get raw list of all units in regions surrounding pOwner.
	void GetUnitListFromSurroundingRegions(Unit* pOwner, std::vector<uint16>* pList);

	// Get info for users in regions around user (WIZ_REQ_USERIN)
	void UserInOutForMe(CUser* pSendUser);
	void BotInOutForMe(CUser* pSendUser);

	// Get list of user IDs in region
	void GetRegionUserList(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom = 0, bool SurUpdate = false, CUser* pExcept = nullptr);

	// Get list of users for regions around user (WIZ_REGIONCHANGE)
	void RegionUserInOutForMe(CUser* pSendUser);

	// Get info for Bots in region
	void GetRegionBotIn(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom = 0);

	// Get list of Bot IDs in region
	void GetRegionBotList(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom = 0, bool SurUpdate = false);


	// Get info for merchants in regions around user
	void MerchantUserInOutForMe(CUser* pSendUser);

	INLINE bool isCswActive() { return pCswEvent.Started; }
	INLINE bool isCswWarActive() { return pCswEvent.Status == CswOpStatus::War; }

	// Get war status
	INLINE bool isWarOpen() { return m_byBattleOpen != NO_BATTLE; }

	INLINE bool isBorderDefenceWarActive() { return pTempleEvent.ActiveEvent == TEMPLE_EVENT_BORDER_DEFENCE_WAR && pTempleEvent.isActive == true; }
	INLINE bool isJuraidMountainActive() { return pTempleEvent.ActiveEvent == TEMPLE_EVENT_JURAD_MOUNTAIN && pTempleEvent.isActive == true; }
	INLINE bool isChaosExpansionActive() { return pTempleEvent.ActiveEvent == TEMPLE_EVENT_CHAOS && pTempleEvent.isActive == true; }

	INLINE bool isCindirellaZone(uint8 zoneid)
	{
		// Cindirella/Fun Class sadece mini event haritalarinda calismali.
		// Eski DB ayarlarinda CZ/Ardream/RLB yazsa bile normal PK/CZ akisini
		// event gibi algilamamasi icin burada yalnizca 107/115 kabul edilir.
		return zoneid == 107 || zoneid == 115;
	}

	INLINE bool isForgettenTempleActive() { return pForgettenTemple.isActive && !pForgettenTemple.isFinished; }


	// Get list of merchants in region
	void GetRegionMerchantUserIn(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom = 0);
	void GetRegionMerchantBotIn(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom = 0);

	void SendHelpDescription(CUser* pUser, std::string sHelpMessage);
	void SendInfoMessage(CUser* pUser, std::string sHelpMessage, uint8 type);
	void SendYeniNoticeZone(CUser* pUser, std::string sHelpMessage);

	INLINE bool isPermanentMessageSet() { return m_bPermanentChatMode; }
	void SetPermanentMessage(const char* format, ...);

	void HandleConsoleCommand(const char* msg);

	template <ChatType chatType>
	INLINE void SendGMvoid(const char* msg, bool bFormatNotice = false)
	{
		Packet result;
		std::string buffer;

		if (bFormatNotice)
			GetServerResource(999, &buffer, msg); // 999 olan k�s�m SERVER_RESOURCE tablosunda 999 olusturulup %s eklenecek
		else
			buffer = msg;

		ChatPacket::Construct(&result, (uint8)chatType, &buffer);
		Send_GM(&result);
	}
	// end
	template <ChatType chatType>
	INLINE void SendChat(const char* msg, uint8 byNation = Nation::ALL, bool bFormatNotice = false)
	{
		Packet result;
		std::string buffer;

		if (bFormatNotice)
			GetServerResource(IDP_ANNOUNCEMENT, &buffer, msg);
		else
			buffer = msg;

		ChatPacket::Construct(&result, (uint8)chatType, &buffer);
		Send_All(&result, nullptr, byNation);

		SendNoticeWindAll(buffer, 0xFFFFFF00);
	}

	template <ChatType chatType>
	INLINE void SendChatToZone(const char* msg, uint8 ZoneID, uint8 byNation = Nation::ALL, bool bFormatNotice = false)
	{
		Packet result;
		std::string buffer;

		if (bFormatNotice)
			GetServerResource(IDP_ANNOUNCEMENT, &buffer, msg);
		else
			buffer = msg;

		ChatPacket::Construct(&result, (uint8)chatType, &buffer);
		Send_Zone(&result, ZoneID, nullptr, byNation);

		SendNoticeWindAll(buffer, 0xFFFFFF00, ZoneID);
	}

	template <ChatType chatType>
	INLINE void SendFormattedChat(const char* msg, uint8 byNation = Nation::ALL, bool bFormatNotice = false, va_list args = nullptr)
	{
		char buffer[512];
		vsnprintf(buffer, sizeof(buffer), msg, args);
		SendChat<chatType>(buffer, byNation, bFormatNotice);
		va_end(args);
	}

	template <ChatType chatType>
	void SendFormattedChat(const char* msg, uint8 byNation = Nation::ALL, bool bFormatNotice = false, ...)
	{
		va_list ap;
		va_start(ap, byNation);
		SendFormattedChat<chatType>(msg, byNation, bFormatNotice, ap);
		va_end(ap);
	}

	/* The following are simply wrappers for more readable SendChat() calls */

	INLINE void SendNotice(const char* msg, uint8 byNation = (uint8)Nation::ALL)
	{
		SendChat<ChatType::PUBLIC_CHAT>(msg, byNation, true);
	}
	INLINE void SendGM(const char* msg)
	{
		SendGMvoid<ChatType::SHOUT_CHAT>(msg, true);
	}
	// end
	template <ChatType chatType>
	INLINE void SendNotice(const char* msg, uint8 ZoneID, uint8 byNation = (uint8)Nation::ALL, bool bFormatNotice = false)
	{
		SendChatToZone<chatType>(msg, ZoneID, byNation, bFormatNotice);
	}

	void SendFormattedNotice(const char* msg, uint8 byNation = (uint8)Nation::ALL, ...)
	{
		va_list ap;
		va_start(ap, byNation);
		SendFormattedChat<ChatType::PUBLIC_CHAT>(msg, byNation, true, ap);
		va_end(ap);
	}

	INLINE void SendAnnouncement(const char* msg, uint8 byNation = (uint8)Nation::ALL)
	{
		SendChat<ChatType::WAR_SYSTEM_CHAT>(msg, byNation, true);
	}

	void SendFormattedAnnouncement(const char* msg, uint8 byNation = (uint8)Nation::ALL, ...)
	{
		va_list ap;
		va_start(ap, byNation);
		SendFormattedChat<ChatType::WAR_SYSTEM_CHAT>(msg, byNation, true, ap);
		va_end(ap);
	}

	void SendFormattedResource(uint32 nResourceID, uint8 byNation = (uint8)Nation::ALL, bool bIsNotice = true, ...);

	void Send_Region(Packet* pkt, C3DMap* pMap, int x, int z, CUser* pExceptUser = nullptr, uint16 nEventRoom = 0);
	void Send_UnitRegion(Packet* pkt, C3DMap* pMap, int x, int z, CUser* pExceptUser = nullptr, uint16 nEventRoom = 0);
	void Send_OldRegions(Packet* pkt, int old_x, int old_z, C3DMap* pMap, int x, int z, CUser* pExceptUser = nullptr, uint16 nEventRoom = 0);
	void Send_NewRegions(Packet* pkt, int new_x, int new_z, C3DMap* pMap, int x, int z, CUser* pExceptUser = nullptr, uint16 nEventRoom = 0);

	void Send_NearRegion(Packet* pkt, C3DMap* pMap, int region_x, int region_z, float curx, float curz, CUser* pExceptUser = nullptr, uint16 nEventRoom = 0);
	void Send_FilterUnitRegion(Packet* pkt, C3DMap* pMap, int x, int z, float ref_x, float ref_z, CUser* pExceptUser = nullptr, uint16 nEventRoom = 0);

	void Send_Zone_Matched_Class(Packet* pkt, uint8 bZoneID, CUser* pExceptUser, uint8 nation, uint8 seekingPartyOptions, uint16 nEventRoom = 0);
	void Send_Zone(Packet* pkt, uint8 bZoneID, CUser* pExceptUser = nullptr, uint8 nation = 0, uint16 nEventRoom = 0, float fRange = 0.0f);
	void Send_Merchant(Packet* pkt, uint8 bZoneID, CUser* pExceptUser = nullptr, uint8 nation = 0, uint16 nEventRoom = 0, float fRange = 0.0f);

	void Send_All(Packet* pkt, CUser* pExceptUser = nullptr, uint8 nation = 0, uint8 ZoneID = 0, bool isSendEventUsers = false, uint16 nEventRoom = 0);
	void Send_GM(Packet* pkt);
	void SendNoticeWindAll(std::string notice, uint32 color, uint8 ZoneID = 0);
	// end

	void ReqHandleAuthority(Packet& pkt, CUser* pUser);

	void GetServerResource(int nResourceID, std::string* result, ...);
	_START_POSITION* GetStartPosition(int nZoneID);

	int64 GetExpByLevel(int nLevel, int RebithLevel);
	C3DMap* GetZoneByID(int zoneID);

	CUser* GetUserPtr(std::string findName, NameType type);
	CUser* GetUserPtr(uint16 sUserId);
	CNpc* GetNpcPtr(uint16 sNpcId, uint16 sZoneID);
	Unit* GetUnitPtr(uint16 id, uint16 sZoneID = 0);
	CBot* GetBotPtr(uint16 sBotId);
	CBot* GetBotPtr(std::string findName, NameType type);

	// Spawns an event NPC/monster
	void SpawnEventNpc(uint16 sSid, bool bIsMonster, uint8 byZone, float fX, float fY, float fZ, uint16 sCount = 1, uint16 sRadius = 0, uint16 sDuration = 0, uint8 nation = 0, int16 socketID = -1, uint16 nEventRoom = 0, uint8 byDirection = 0, uint8 bMoveType = 0, uint8 bGateOpen = 0, uint16 nSummonSpecialType = 0, uint32 nSummonSpecialID = 0);

	// Spawns an event Pet / GuardSummon
	void SpawnEventPet(uint16 sSid, bool bIsMonster, uint8 byZone, float fX, float fY, float fZ, uint16 sCount = 1, uint16 sRadius = 0, uint16 sDuration = 0, uint8 nation = 0, int16 socketID = -1, uint16 nEventRoom = 0, uint8 nType = 0, uint32 nSkillID = 0);

	// Removes all the NPC/monster in the specified event room
	void RemoveAllEventNpc(uint8 byZone);

	// Resets all the NPC/monster in the specified event room
	void ResetAllEventObject(uint8 byZone);
	
	// Spawns an event SlaveUserBots
	uint16 SpawnSlaveUserBot(int Minute, CUser* pUser = nullptr);

	// Spawns an event SlavePriest
	uint16 SpawnPriestBot(int Minute, CUser* pUser = nullptr);

	// Kill a Npc/Monster
	void KillNpc(uint16 sNid, uint8 byZone, Unit* pKiller = nullptr);

	// Change NPC/Monster properties for Respawn
	void NpcUpdate(uint16 sSid, bool bIsMonster, uint8 byGroup = 0, uint16 sPid = 0);

	// Adds the account name & session to a hashmap (on login)
	void AddAccountName(CUser* pSession);

	// Adds the character name & session to a hashmap (when in-game)
	void AddCharacterName(CUser* pSession);

	// Removes an existing character name/session from the hashmap, replaces the character's name 
	// and reinserts the session with the new name into the hashmap.
	void ReplaceCharacterName(CUser* pSession, std::string& strNewUserID);

	// Removes the account name & character names from the hashmaps (on logout)
	void RemoveSessionNames(CUser* pSession);

	// Send to Zone NPC Effect
	void ShowNpcEffect(uint16 sNpcID, uint32 nEffectID, uint8 ZoneID, uint16 nEventRoom = 0);

	void csw_prepareopen();
	void csw_waropen();
	void csw_usertools(bool Notice = false,
		CswNotice s = CswNotice::Preparation,
		bool Flag = false,
		bool KickOutPreapre = false,
		bool KickOutWar = false,
		bool town = false,
		bool reward = false);
	void csw_maintimer();
	void csw_remnotice(uint8 NoticeType, int32 Time);
	void csw_reset();
	void csw_close();

	//Bdw Monument
	void BDWMonumentAltarTimer();
	void BDWMonumentAltarRespawn(uint16 bRoom);

	void LotteryClose(CUser* pUser = nullptr);

	_PARTY_GROUP* GetPartyPtr(uint16 sPartyID);
	CKnights* GetClanPtr(uint16 sClanID);
	_KNIGHTS_ALLIANCE* GetAlliancePtr(uint16 sAllianceID);
	_ITEM_TABLE GetItemPtr(uint32 nItemID);
	_LEVEL_REWARDS GetLevelRewards(uint8 bLevel);
	_PARTY_GROUP* CreateParty(CUser* pLeader);
	_PARTY_GROUP* CreateParty(CBot* pLeader);
	void DeleteParty(uint16 sIndex);
	
	_MAGIC_TABLE GetMagicPtr(uint32 nSkillID, bool mutex = true);
	/*_MAGIC_TYPE1 GetMagic1Ptr(uint32 nSkillID, bool mutex = true);
	_MAGIC_TYPE2 GetMagic2Ptr(uint32 nSkillID, bool mutex = true);
	_MAGIC_TYPE3 GetMagic3Ptr(uint32 nSkillID, bool mutex = true);
	_MAGIC_TYPE4 GetMagic4Ptr(uint32 nSkillID, bool mutex = true);
	_MAGIC_TYPE5 GetMagic5Ptr(uint32 nSkillID, bool mutex = true);
	_MAGIC_TYPE6 GetMagic6Ptr(uint32 nSkillID, bool mutex = true);
	_MAGIC_TYPE7 GetMagic7Ptr(uint32 nSkillID, bool mutex = true);
	_MAGIC_TYPE8 GetMagic8Ptr(uint32 nSkillID, bool mutex = true);
	_MAGIC_TYPE9 GetMagic9Ptr(uint32 nSkillID, bool mutex = true);*/

	_WANTED_MAIN pWantedMain[3];
	_EVENT_STATUS pTempleEvent;
	_EVENT_BRIDGE pEventBridge;
	_BEEF_EVENT_STATUS pBeefEvent;
	UserLootSettingsArray UserLootSettings;
	_SERVER_SETTING pServerSetting;
	RANKBUG pRankBug;
	_KNIGHTS_SIEGE_WARFARE pSiegeWar;

	_CSW_STATUS pCswEvent;
	_JACKPOT_SETTING pJackPot[2];
	_LEV_MERCH_INFO pLevMercInfo;
	EVENTMAINTIME pEventTimeOpt;
	_CINDWARGAME pCindWar;
	_BOT_INFO pBotInfo;
	std::atomic<uint32> m_TempleEventLastUserOrder;
	_SOCCER_STATUS pSoccerEvent;
	BURNING_FEATURES pBurningFea[3];

	_MONSTER_STONE_INFO m_TempleEventMonsterStoneRoomList[MAX_MONSTER_STONE_ROOM];

	~CGameServerDlg();

	char	m_ppNotice[20][128];
	char	m_peNotice[20][128];
	

	// Monster Stone Process
	void TempleMonsterStoneTimer();
	void TempleMonsterStoneAutoResetRoom(_MONSTER_STONE_INFO& pRoom);
	void TempleMonsterStoneResetNpcs(int16 roomid, uint8 zoneid);
	uint16	m_MonsterStoneSquadEventRoom;

	// Clan Vs Time System
	void ClanTournamentTimer();
	void UpdateClanTournamentScoreBoard(CUser* pUser);

	// NPC Related Arrays
	NpcThreadArray						m_arNpcThread;
	NpcItemArray						m_NpcItemArray;
	MonsterItemArray					m_MonsterItemArray;
	MakeItemGroupArray					m_MakeItemGroupArray;
	MakeItemGroupRandomArray			m_MakeItemGroupRandomArray;
	MakeWeaponItemTableArray			m_MakeWeaponItemArray;
	MakeWeaponItemTableArray			m_MakeDefensiveItemArray;
	MakeGradeItemTableArray				m_MakeGradeItemArray;
	MakeLareItemTableArray				m_MakeLareItemArray;
	NpcTableArray						m_arMonTable;
	NpcTableArray						m_arNpcTable;
	LoadRightClickExchange				m_LoadRightClickExchange;
	std::recursive_mutex				m_ZoneOnlineRewardArrayLock;
	ZoneOnlineRewardArray				m_ZoneOnlineRewardArray;

	IDMapBotList						m_MapBotList;
	ArtificialIntelligenceArray			m_ArtificialIntelligenceArray;
	ArtificialMerchantArray				m_ArtificialMerchantArray;
	ZoneArray							m_ZoneArray;
	ItemtableArray						m_ItemtableArray;
	ItemSellTableArray					m_ItemSellTableArray;
	ItemPremiumGiftArray				m_ItemPremiumGiftArray;
	SetItemArray						m_SetItemArray;
	ItemWheelArray						m_ItemWheelArray;
	CapeCastellanBonus					m_CapeCastellanBonus;
	MagictableArray						m_MagictableArray;
	Magictype1Array						m_Magictype1Array;
	Magictype2Array						m_Magictype2Array;
	Magictype3Array						m_Magictype3Array;
	Magictype4Array						m_Magictype4Array;
	Magictype5Array						m_Magictype5Array;
	Magictype6Array						m_Magictype6Array;
	Magictype7Array						m_Magictype7Array;
	Magictype8Array						m_Magictype8Array;
	Magictype9Array						m_Magictype9Array;
	CoefficientArray					m_CoefficientArray;
	CoefficientDamageArray				m_CoefficientDamageArray;
	LevelUpArray						m_LevelUpArray;
	PartyArray							m_PartyArray;
	KnightsArray						m_KnightsArray;
	KnightsRatingArray					m_KnightsRatingArray[2]; // one for both nations
	KnightsAllianceArray				m_KnightsAllianceArray;
	LoadUpgrade							m_loadUpgrade;
	UpgradeSettings						m_UpgradeSettings;
	KnightsCapeArray					m_KnightsCapeArray;
	UserNameRankMap						m_UserKarusPersonalRankMap;
	UserNameRankMap						m_UserElmoPersonalRankMap;
	UserNameRankMap						m_UserKarusKnightsRankMap;
	UserNameRankMap						m_UserElmoKnightsRankMap;
	ZoneKillReward						m_ZoneKillReward;
	UserRankMap							m_playerRankings[2]; // one for both nations
	std::recursive_mutex				m_userRankingsLock;

	TempleEventChaosRoomList			m_TempleEventChaosRoomList;
	TempleEventBDWRoomList				m_TempleEventBDWRoomList;
	TempleEventJuraidRoomList			m_TempleEventJuraidRoomList;

	StartPositionArray					m_StartPositionArray;
	ServerResourceArray					m_ServerResourceArray;
	BotSaveDataArray					m_BotSaveDataArray;
	QuestHelperArray					m_QuestHelperArray;
	QuestHelperNationArray				m_QuestHelperNationArray;
	QuestNpcList						m_QuestNpcList;
	MonsterResourceArray				m_MonsterResourceArray;
	QuestMonsterArray					m_QuestMonsterArray;
	RentalItemArray						m_RentalItemArray;
	SpecialStonearray					m_SpecialStonearray;
	ItemExchangeArray					m_ItemExchangeArray;
	ItemExchangeExpArray				m_ItemExchangeExpArray;
	ItemSpecialExchangeArray			m_ItemSpecialExchangeArray;
	ItemExchangeCrashArray				m_ItemExchangeCrashArray;
	ItemUpgradeArray					m_ItemUpgradeArray;
	ItemOpArray							m_ItemOpArray;
	KingSystemArray						m_KingSystemArray;
	SheriffArray						m_SheriffArray;
	CommanderArray						m_CommanderArray;
	EventTriggerArray					m_EventTriggerArray;

	ForgettenTempleMonsterArray			m_ForgettenTempleMonsterArray;
	ForgettenTempleStagesArray			m_ForgettenTempleStagesArray;
	AutomaticCommandArray				m_AutomaticCommandArray;

	MonsterChallengeArray				m_MonsterChallengeArray;
	MonsterChallengeSummonListArray		m_MonsterChallengeSummonListArray;
	MonsterSummonListArray				m_MonsterSummonList;
	MonsterUnderTheCastleArray			m_MonsterUnderTheCastleArray;
	MonsterUnderTheCastleList			m_MonsterUnderTheCastleList;
	MonsterStoneListInformationArray	m_MonsterStoneListInformationArray;
	JuraidMountionListInformationArray	m_JuraidMountionListInformationArray;
	MiningFishingItemArray				m_MiningFishingItemArray;
	PremiumItemArray					m_PremiumItemArray;
	PremiumItemExpArray					m_PremiumItemExpArray;
	UserChaosExpansionRankingArray		m_UserChaosExpansionRankingArray;
	UserBorderDefenceWarRankingArray	m_UserBorderDefenceWarRankingArray[2];
	UserPlayerKillingZoneRankingArray	m_UserPlayerKillingZoneRankingArray[2];
	ZindanWarZoneRankingArray			m_ZindanWarZoneRankingArray[2];
	UserDailyOpMap						m_UserDailyOpMap;
	TempleEventUserMap					m_TempleEventUserMap;
	//TempleEventJuraidRoomList			m_TempleEventJuraidRoomList;
	//TempleEventBDWRoomList			m_TempleEventBDWRoomList;
	//TempleEventChaosRoomList			m_TempleEventChaosRoomList;
	TempleSoccerEventUserArray			m_TempleSoccerEventUserArray;
	NationMonumentInformationArray		m_NationMonumentWinnerNationArray;
	NationMonumentInformationArray		m_NationMonumentDefeatedNationArray;
	StartPositionRandomArray			m_StartPositionRandomArray;
	UserItemArray						m_UserItemArray;
	ObjectEventArray					m_ObjectEventArray;
	HardwareInformation					g_HardwareInformation;
	std::recursive_mutex				m_SeekingPartyArrayLock;
	SeekingPartyArray					m_SeekingPartyArray;
	ChatRoomArray						m_ChatRoomArray;
	BottomUserListArray					m_BottomUserListArray;
	CharacterSealItemArray				m_CharacterSealItemArray;
	CharacterSealItemMapping			m_CharacterSealItemMapping;
	MiningExchangeArray					m_MiningExchangeArray;

	ChaosStoneStageArray				m_ChaosStoneStageArray;
	ChaosStoneSummonListArray			m_ChaosStoneSummonListArray;
	ChaosStoneRespawnCoordinateArray	m_ChaosStoneRespawnCoordinateArray;
	ChaosStoneInfoArray					m_ChaosStoneInfoArray;
	LuaGiveItemExchangeArray			m_LuaGiveItemExchangeArray;

	BeefEventPlayTimerArray				m_BeefEventPlayTimerArray;
	EventTimerShowArray					m_EventTimerShowArray;

	MonsterRespawnLoopArray				m_MonsterRespawnLoopArray;

	ZindanWarStageListArray				m_ZindanWarStageListArray;
	ZindanWarSummonListArray			m_ZindanWarSummonListArray;

	DailyQuestArray						m_DailyQuestArray;

	// Achiement System
	AchieveTitleArray					m_AchieveTitleArray;
	AchieveMainArray					m_AchieveMainArray;
	AchieveMonsterArray					m_AchieveMonsterArray;
	AchieveNormalArray					m_AchieveNormalArray;
	AchieveWarArray						m_AchieveWarArray;
	AchieveComArray						m_AchieveComArray;
	DrakiRoomList						m_DrakiRoomListArray;
	DrakiMonsterList					m_DrakiMonsterListArray;
	DrakiRankList						m_DrakiRankListArray;
	MonsterDrakiTowerList				m_MonsterDrakiTowerList;
	ClanVsDataList						m_ClanVsDataList;
	DungeonDefenceMonsterListArray		m_DungeonDefenceMonsterListArray;
	DungeonDefenceRoomListArray			m_DungeonDefenceRoomListArray;
	DungeonDefenceStageListArray		m_DungeonDefenceStageListArray;
	KnightReturnLetterGiftListArray		m_KnightReturnLetterGiftListArray;
	WarBanishOfWinnerArray				m_WarBanishOfWinnerArray;

	PusItemArray						m_PusItemArray;
	EventRewardArray					m_EventRewardArray;

	RimaLotteryArray					m_RimaLotteryArray;
	RimaLotteryUserInfo					m_RimaLotteryUserInfo;

	//Pet System
	PetDataSystemArray					m_PetDataSystemArray;
	PetInfoSystemArray					m_PetInfoSystemArray;
	PetTransformSystemArray				m_PetTransformSystemArray;

	MonsterBossRandomSpawn				m_MonsterBossRandom;
	MonsterBossRandomStage				m_MonsterBossStage;
	std::recursive_mutex				m_CharacterSetValidArraylock;
	CharacterSetValidArray				m_CharacterSetValidArray;
	TimedNoticeArray					m_TimedNoticeArray;
	CindirellaItemsArray				m_CindirellaItemsArray[5];
	CindirellaStatArray					m_CindirellaStatArray;
	LevelRewardArray					m_LevelRewardArray;
	LevelMerchantRewardArray			m_LevelMerchantRewardArray;

	//Wanted System
	RandomItemArray						m_RandomItemArray;
	SendMessageArray					m_SendMessageArray;
	RightTopTitleArray					m_RightTopTitleArray;
	TopLeftArray						m_TopLeftArray;
	DailyRank							m_DailyRank;
	PerksArray							m_PerksArray;
	RankRewardArray						m_RankRewardArray;
	Atomic<int16>						m_sPartyIndex;
	Condition* s_hEvent;

	std::recursive_mutex m_CommanderArrayLock, m_BottomUserListArrayLock;
	std::recursive_mutex m_PetSystemlock, giftlock;

	std::vector< _RIMA_LOOTERY_USER_INFO> mLotteryUserList;
	uint32 mLotteryRewardItem[4];

	//Bowl Spawn Event
	bool isBowlEventActive;
	uint8 tBowlEventZone;
	void	CloseBowlEvent();
	uint16 tBowlEventTime;
	uint16 m_sYear, m_sMonth, m_sDate, m_sHour, m_sMin, m_sSec;
	uint8 m_byWeather;
	uint16 m_sWeatherAmount;
	int m_nCastleCapture;
	uint8 m_ReloadKnightAndUserRanksMinute;
	uint16 ClanPreFazlaNP;	// Clan Premium
	uint8 ClanPreFazlagold;	// Clan Premium
	bool ClanPrePazar;
	uint8 ClanPreFazlaExp;
	uint8   m_byBattleOpen, m_byOldBattleOpen, m_byBattleNoticeTime;
	uint8  m_byBattleZone, m_byBattleZoneType, m_bVictory, m_byOldVictory, m_bResultDelayVictory, m_bKarusFlag, m_bElmoradFlag, m_bMiddleStatueNation;
	int32	m_byBattleOpenedTime;
	int32	m_byBattleTime;
	int32	m_byBattleRemainingTime;
	int32	m_byNereidsIslandRemainingTime;
	int32	m_sBattleTimeDelay;
	int32  m_sBattleResultDelay;

	//clanbank
	bool ClanBankStatus;

	uint8	m_sKilledKarusNpc, m_sKilledElmoNpc;
	uint8	m_sKarusMonuments, m_sElmoMonuments;
	uint16  m_sKarusMonumentPoint, m_sElmoMonumentPoint;
	bool    m_byKarusOpenFlag, m_byElmoradOpenFlag, m_byBanishFlag, m_byBattleSave, m_bResultDelay, m_bySiegeBanishFlag;
	short   m_sDiscount;
	short	m_sKarusDead, m_sElmoradDead, m_sBanishDelay, m_sKarusCount, m_sElmoradCount;

	std::string m_strKarusCaptain, m_strElmoradCaptain;

	uint8	m_nPVPMonumentNation[MAX_ZONE_ID];
	uint8	m_sNereidsIslandMonuArray[7];
	bool    m_bCommanderSelected;
	uint32  m_GameServerPort;
	std::string  m_LoginServerIP;
	uint32  m_LoginServerPort;
	uint8	m_byMaxLevel;
	int32	m_nGameMasterRHitDamage;
	int32	m_nBonusTimeInterval, m_nBonusTimeGold, m_nBonusPVPWarItem;
	uint16	m_CountofTickets;
	uint8	m_nPlayerRankingResetTime;

	std::string  m_sPlayerRankingsRewardZones;
	uint32  m_nPlayerRankingKnightCashReward;
	uint32  m_nPlayerRankingLoyaltyReward, m_Grade1, m_Grade2, m_Grade3, m_Grade4;
	uint32  m_nChaosEventAwards0, m_nChaosEventAwards1, m_nChaosEventAwards2, m_nChaosEventAwards3, m_nChaosEventAwards4, m_nChaosEventAwards5;
	uint32	m_nBorderEventAwards, m_nBorderEventAwardsLoser;
	uint32  m_nBorderEventAwards0, m_nBorderEventAwards1, m_nBorderEventAwards2, m_nBorderEventAwards3, m_nBorderEventAwards4, m_nBorderEventAwards5, m_nBorderEventAwards6,
		m_nBorderEventAwards7, m_nBorderEventAwards8, m_nBorderEventAwards9, m_nBorderEventAwards10, m_nBorderEventAwards11, m_nBorderEventAwards12, m_nBorderEventAwards13, m_nBorderEventAwards14,
		m_nBorderEventAwards15, m_nBorderEventAwards16, m_nBorderEventAwards17, m_nBorderEventAwards18, m_nBorderEventAwards19, m_nBorderEventAwards20, m_nBorderEventAwards21, m_nBorderEventAwards22, m_nBorderEventAwards23,
		m_nBorderEventAwards24, m_nBorderEventAwards25, m_nBorderEventAwards26, m_nBorderEventAwards27, m_nBorderEventAwards28, m_nBorderEventAwards29, m_nBorderEventAwards30,
		m_nBorderEventAwardsLoser0, m_nBorderEventAwardsLoser1, m_nBorderEventAwardsLoser2, m_nBorderEventAwardsLoser3, m_nBorderEventAwardsLoser4, m_nBorderEventAwardsLoser5, m_nBorderEventAwardsLoser6, m_nBorderEventAwardsLoser7,
		m_nBorderEventAwardsLoser8, m_nBorderEventAwardsLoser9, m_nBorderEventAwardsLoser10, m_nBorderEventAwardsLoser11, m_nBorderEventAwardsLoser12, m_nBorderEventAwardsLoser13, m_nBorderEventAwardsLoser14, m_nBorderEventAwardsLoser15;

	uint32	m_nJuraidEventAwards, m_nJuraidEventAwards1;
	uint32	m_nCastleSiegeWarfareLoyaltyEventAwards, m_nCastleSiegeWarfareLoyaltyEventAwardsLoser;
	uint32	m_nCastleSiegeWarfareCashPointEventAwards, m_nCastleSiegeWarfareCashPointEventAwardsLoser;

	uint16  m_GameInfo1Time;
	uint16  m_GameInfo2Time;

	uint8	m_bMaxRegenePoint;

	bool	m_bPermanentChatMode;
	std::string	m_strPermanentChat;

	uint8	m_bSantaOrAngel;
	uint8	m_sRankResetHour;
	bool    m_RankRewardSendStatus;

	// National Points Settings
	int m_Loyalty_Ardream_Source;
	int m_Loyalty_Ardream_Target;
	int m_Loyalty_Ronark_Land_Base_Source;
	int m_Loyalty_Ronark_Land_Base_Target;
	int m_Loyalty_Ronark_Land_Source;
	int m_Loyalty_Ronark_Land_Target;
	int m_Loyalty_Other_Zone_Source;
	int m_Loyalty_Other_Zone_Target;

	//Military Camp
	uint8 m_sKarusMilitary;
	uint8 m_sKarusEslantMilitary;
	uint8 m_sHumanMilitary;
	uint8 m_sHumanEslantMilitary;
	uint8 m_sMoradonMilitary;

	uint8 m_sJackExpPots;
	uint8 m_sJackGoldPots;
	uint32 MonsterDeadCount;

	uint32 m_WantedSystemMapShowTime;
	uint32 m_WantedSystemTownControlTime;

	void SendEventRemainingTime(bool bSendAll = false, CUser* pUser = nullptr, uint8 ZoneID = 0);

	bool m_IsMagicTableInUpdateProcess;
	bool m_IsPlayerRankingUpdateProcess;

	//server proceess
	bool LoadedGameServer;
	int LoadedTables;
	int LastLoadedTables;


	uint32 ZindanWarStartTimeTickCount;
	bool   ZindanWarSummonCheck;
	uint32 ZindanLastSummonTime;
	int32 ZindanWarSummonStage;
	bool   ZindanWarisSummon;
	bool   ZindanWarLastSummon;
	std::vector<uint32> ZindanSpawnList;

	std::recursive_mutex m_addbotlistLock;
	std::map<uint32, _botadd> m_addbotlist;

	void	SpecialEventFinish();

	std::vector<int64> m_HardwareIDArray;

	std::vector<uint16>				m_nZindanWarUsers;
	// Under The Castle
	std::vector<uint16>				m_nUnderTheCastleUsers;
	UnderTheCastleMonsterList		m_UnderTheCastleMonsterList;
	bool							m_bUnderTheCastleIsActive;
	bool							m_bUnderTheCastleMonster;
	int32							m_nUnderTheCastleEventTime;
	

	bool npcthreadreload;
	int					m_nServerNo, m_nServerGroupNo;
	int					m_nServerGroup;	
	ServerArray			m_ServerArray;
	ServerArray			m_ServerGroupArray;

	UserInfoStorMap	m_InfoStorMap;
	uint16 m1anvilid;
	uint16 m2anvilid;
	uint16 m3anvilid;
	uint16 m4anvilid;
	uint16 m5anvilid;
	std::recursive_mutex ingameaccountslock, m_InfoStorMapLock;
	std::map<std::string, time_t> ingameaccounts;

	std::recursive_mutex m_GmListlock;
	NameMap		m_accountNameMap, m_characterNameMap, m_GMNameMap;
	BotNameMap	m_BotcharacterNameMap;
	std::recursive_mutex m_AntiAfkListLock, m_RightClickItemListLock;
	AntiAfkList m_AntiAfkList;
	GmList m_GmList;
	std::recursive_mutex m_CrRandomListLock;
	std::recursive_mutex m_timershowlistLock;
	timershowlist m_timershowlist;

	bool timershowreset;
	//VS Event
	uint32 vsEventMaxNp;
	uint32 vsEventMinNp;
	uint32 m_VsEventLastRoom;
	bool m_bCzSantaEventActive;
	uint32 m_CzSantaEventEndTime;
	uint32 m_CzSantaNextSpawnTime;
	uint32 m_CzSantaLastOpenHour;

	std::recursive_mutex	m_accountNameLock,
		m_characterNameLock,
		m_questNpcLock, m_BotcharacterNameLock,
		m_MapBotListLock;

	// Controlled weather events set by Kings
	uint8 m_byKingWeatherEvent;
	uint8 m_byKingWeatherEvent_Day;
	uint8 m_byKingWeatherEvent_Hour;
	uint8 m_byKingWeatherEvent_Minute;

	// XP/coin/NP events
	uint8 m_byExpEventAmount, m_byCoinEventAmount, m_byNPEventAmount, m_byDropEventAmount;

	INLINE CLuaEngine* GetLuaEngine() { return &m_luaEngine; }

	KOSocketMgr<CUser> m_socketMgr;
	bool m_LoginServerConnected;

	void SendYesilNotice(CUser* pUser, std::string sHelpMessage); // JstKO Offline Merchant i�in Eklendi 29.12.2024
	void SendRgbNotice(std::string Message, uint8 R, uint8 G, uint8 B); // JstKO Logos Rgb Notice Eklendi 29.12.2024
	void HandleCzSantaEventTimer();

	uint16 m_sLoginingPlayer;

	Atomic<uint16>	m_DrakiRiftTowerRoomIndex;
	Atomic<uint16>	m_DungeonDefenceRoomIndex;

	time_t m_Shutdownfinishtime;
	bool   m_Shutdownstart;
	bool   m_Shutdownfinished;
	bool   m_ShutdownKickStart;
	void   ShutdownTimer();

private:
	CLuaEngine	m_luaEngine;

	std::string m_strGameDSN, m_strGameUID, m_strGamePWD;
	std::string m_strAccountDSN, m_strAccountUID, m_strAccountPWD;
	std::string m_strLogDSN, m_strLogUID, m_strLogPWD;
	bool m_bMarsEnabled;

	bool ProcessServerCommand(std::string& command);

public:
	void InitServerCommands();
	void CleanupServerCommands();

	static ServerCommandTable s_commandTable;

	COMMAND_HANDLER(HandleMorankerCommand);
	COMMAND_HANDLER(HandleReloadCOEFFICIENTCommand);
	COMMAND_HANDLER(HandleHelpCommand);
	COMMAND_HANDLER(HandleForgettenTempleEventClose);
	COMMAND_HANDLER(HandleForgettenTempleEvent);
	COMMAND_HANDLER(HandleResetRLoyaltyCommand);
	COMMAND_HANDLER(HandleNoticeCommand);
	COMMAND_HANDLER(HandleNoticeallCommand);
	COMMAND_HANDLER(HandleKillUserCommand);
	COMMAND_HANDLER(HandleWar1OpenCommand);
	COMMAND_HANDLER(HandleWar2OpenCommand);
	COMMAND_HANDLER(HandleWar3OpenCommand);
	COMMAND_HANDLER(HandleWar4OpenCommand);
	COMMAND_HANDLER(HandleWar5OpenCommand);
	COMMAND_HANDLER(HandleWar6OpenCommand);
	COMMAND_HANDLER(HandleSnowWarOpenCommand);
	COMMAND_HANDLER(HandleSiegeWarOpenCommand);
	COMMAND_HANDLER(HandleWarCloseCommand);
	COMMAND_HANDLER(HandleShutdownCommand);
	COMMAND_HANDLER(HandleDiscountCommand);
	COMMAND_HANDLER(HandleGlobalDiscountCommand);
	COMMAND_HANDLER(HandleDiscountOffCommand);
	COMMAND_HANDLER(HandleCaptainCommand);
	COMMAND_HANDLER(HandleSantaCommand);
	COMMAND_HANDLER(HandleSantaOffCommand);
	COMMAND_HANDLER(HandleAngelCommand);
	COMMAND_HANDLER(HandlePermanentChatCommand);
	COMMAND_HANDLER(HandlePermanentChatOffCommand);
	COMMAND_HANDLER(HandleReloadNoticeCommand);
	COMMAND_HANDLER(HandleReloadAllTabCommand);
	COMMAND_HANDLER(HandleReloadBonusNotice);
	COMMAND_HANDLER(HandleReloadTablesCommand);
	COMMAND_HANDLER(HandleSpecialEventOpenCommand);
	COMMAND_HANDLER(HandleReloadTables2Command);
	COMMAND_HANDLER(HandleReloadTables3Command);
	COMMAND_HANDLER(HandleReloadPusItemCommand);
	COMMAND_HANDLER(HandleReloadMagicsCommand);
	COMMAND_HANDLER(HandleReloadUpgradeCommand);
	COMMAND_HANDLER(HandleReloadQuestCommand);
	COMMAND_HANDLER(HandleReloadDailyQuestCommand);
	COMMAND_HANDLER(HandleReloadRankBugCommand);
	COMMAND_HANDLER(HandleReloadLevelRewardCommand);
	COMMAND_HANDLER(HandleReloadMerchantLevelRewardCommand);
	COMMAND_HANDLER(HandleReloadRanksCommand);
	COMMAND_HANDLER(HandleReloadDropsCommand);
	COMMAND_HANDLER(HandleReloadDropsRandomCommand);
	COMMAND_HANDLER(HandleReloadKingsCommand);
	COMMAND_HANDLER(HandleReloadRightTopTitleCommand);
	COMMAND_HANDLER(HandleReloadItemsCommand);
	COMMAND_HANDLER(HandleReloadItems);
	COMMAND_HANDLER(HandleReloadDungeonDefenceTables);
	COMMAND_HANDLER(HandleReloadDrakiTowerTables);
	COMMAND_HANDLER(HandleReloadZoneOnlineRewardCommand);
	COMMAND_HANDLER(HandleNPAddCommand);
	COMMAND_HANDLER(HandleExpAddCommand);
	COMMAND_HANDLER(HandleMoneyAddCommand);
	COMMAND_HANDLER(HandleDropAddCommand);
	COMMAND_HANDLER(HandleTeleportAllCommand);
	COMMAND_HANDLER(HandleCountCommand);
	COMMAND_HANDLER(HandleWarResultCommand);
	COMMAND_HANDLER(HandleEventUnderTheCastleCommand);
	COMMAND_HANDLER(HandleCastleSiegeWarClose);
	COMMAND_HANDLER(HandleTournamentStart);
	COMMAND_HANDLER(HandleTournamentClose);
	COMMAND_HANDLER(Handleunbannedcommand);
	COMMAND_HANDLER(Handlebannedcommand);

	COMMAND_HANDLER(HandleServerGameTestCommand);
	COMMAND_HANDLER(HandleChaosExpansionOpen);
	COMMAND_HANDLER(HandleBorderDefenceWar);
	COMMAND_HANDLER(HandleJuraidMountain);
	COMMAND_HANDLER(HandleBeefEvent);
	COMMAND_HANDLER(HandleBugdanKurtarCommand);

	COMMAND_HANDLER(HandleChaosExpansionClose);
	COMMAND_HANDLER(HandleBorderDefenceWarClose);
	COMMAND_HANDLER(HandleJuraidMountainClose);
	COMMAND_HANDLER(HandleBeefEventClose);
	COMMAND_HANDLER(HandleReloadClanPremiumTable);

	COMMAND_HANDLER(HandleEventScheduleResetTable);
	COMMAND_HANDLER(HandleLotteryStart);
	COMMAND_HANDLER(HandleLotteryClose);
	COMMAND_HANDLER(HandleTopLeftCommand);
	COMMAND_HANDLER(HandleCindirellaWarOpen);
	COMMAND_HANDLER(HandleAIResetCommand);
	COMMAND_HANDLER(HandleReloadCindirellaCommand);
	COMMAND_HANDLER(HandleCindirellaWarClose);
	COMMAND_HANDLER(HandleServerPkBotMixCommand);
	COMMAND_HANDLER(HandleServerPkBoxSingleCommand);
	COMMAND_HANDLER(HandleServerFarmBotMixCommand);
	COMMAND_HANDLER(HandleServerMerchantBotMixCommand);
	COMMAND_HANDLER(HandleServerOpenMasterCommand);
};

extern CGameServerDlg* g_pMain;
