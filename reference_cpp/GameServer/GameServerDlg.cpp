#include "stdafx.h"
#include "CBot.h"
#include "KingSystem.h"
#include "KnightsManager.h"
#include "../shared/ClientSocketMgr.h"
#include "../shared/Ini.h"
#include "../shared/DateTime.h"
#include "../shared/SocketPoll.h"

#include <time.h>
#include <iostream>
#include <fstream>
#include <cstdarg>
#include <mutex>
#include <psapi.h> 
#include "DBAgent.h"

using namespace std;

std::vector<Thread*> g_timerThreads;

static void StartupTrace(const char* msg)
{
	std::ofstream f("boot_trace.log", std::ios::app);
	if (!f.is_open())
		return;
	SYSTEMTIME st;
	GetLocalTime(&st);
	f << "[" << st.wHour << ":" << st.wMinute << ":" << st.wSecond << "." << st.wMilliseconds << "] " << msg << std::endl;
}

static std::mutex g_runtimeLogMutex;

static std::string JstKOFormatTimestamp(const SYSTEMTIME& st)
{
	char timeBuffer[64] = {};
	_snprintf_s(timeBuffer, sizeof(timeBuffer), _TRUNCATE,
		"%04u-%02u-%02u %02u:%02u:%02u.%03u",
		st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, st.wMilliseconds);
	return std::string(timeBuffer);
}

static std::string JstKOLogFileName(const SYSTEMTIME& st)
{
	char fileBuffer[MAX_PATH] = {};
	_snprintf_s(fileBuffer, sizeof(fileBuffer), _TRUNCATE,
		"Logs\\JstKO_%04u-%02u-%02u.log",
		st.wYear, st.wMonth, st.wDay);
	return std::string(fileBuffer);
}

void CGameServerDlg::ServerLog(const char* category, const char* fmt, ...)
{
	if (!JstKODetailedLogSystem)
		return;

	char message[8192] = {};
	va_list args;
	va_start(args, fmt);
	_vsnprintf_s(message, sizeof(message), _TRUNCATE, fmt, args);
	va_end(args);

	SYSTEMTIME st;
	GetLocalTime(&st);

	std::lock_guard<std::mutex> guard(g_runtimeLogMutex);
	CreateDirectoryA("Logs", nullptr);

	std::ofstream f(JstKOLogFileName(st).c_str(), std::ios::app);
	if (!f.is_open())
		return;

	f << "[" << JstKOFormatTimestamp(st) << "]"
		<< "[PID:" << GetCurrentProcessId() << "]"
		<< "[TID:" << GetCurrentThreadId() << "]"
		<< "[" << (category != nullptr && category[0] != '\0' ? category : "GENERAL") << "] "
		<< message << std::endl;
}

#pragma region CGameServerDlg::CGameServerDlg()
CGameServerDlg::CGameServerDlg()
{
	Initialize();
}
#pragma endregion

#pragma region CGameServerDlg::Initialize()

void CGameServerDlg::Initialize()
{
	DateTime now;
	JstKODetailedLogSystem = false;
	king_selectreq = false;
	KingRewardKC = 0;
	//Sanal Oda
	m_VsEventLastRoom = 2000;
	m_bisDamage = false;
	m1anvilid = 0;
	m2anvilid = 0;
	m3anvilid = 0;
	m4anvilid = 0;
	m5anvilid = 0;
	tar_levelmercreward = tar_levelreward = false;
	pLevMercInfo.Initialize();
	pCswEvent.Initialize();
	pForgettenTemple.Initialize();
	m_randomtable_reload = false;
	m_ZoneOnlineRewardReload = false;
	autocrchecktime = 0;
	pCindWar.Initialize(true);
	m_iItemUniqueID = 0;
	g_bNpcExit = false;
	npcthreadreload = false;
	m_sYear = now.GetYear();
	m_sMonth = now.GetMonth();
	m_sDate = now.GetDay();
	m_sHour = now.GetHour();
	m_sMin = now.GetMinute();
	m_sSec = now.GetSecond();
	m_ReloadKnightAndUserRanksMinute = 0;
	m_byWeather = WEATHER_FINE;
	m_sWeatherAmount = 100;
	m_byKingWeatherEvent = 0;
	m_byKingWeatherEvent_Day = 0;
	m_byKingWeatherEvent_Hour = 0;
	m_byKingWeatherEvent_Minute = 0;
	m_byExpEventAmount = 0;
	m_byCoinEventAmount = 0;
	m_byNPEventAmount = 0;
	m_byDropEventAmount = 0;
	m_sPartyIndex = 0;
	m_nCastleCapture = 0;
	m_nServerNo = 0;
	m_nServerGroupNo = 0;
	m_nServerGroup = 0;
	m_sDiscount = 0;


	m_GmList.clear();
	DelayedTime = 0;

	ZindanWarStartTimeTickCount = 0;
	ZindanWarSummonCheck = false;
	ZindanWarSummonStage = 1;
	ZindanWarisSummon = false;
	ZindanWarLastSummon = false;
	ZindanLastSummonTime = 0;
	ZindanSpawnList.clear();

	TempPlcCodeGameMasterSocket = -1; // DeaFSezo /plc komutunu calıstırır ve kullandıgı programları gösterir sol altta notice olarak 27.09.2020
	MonsterDeadCount = 0;
	m_WantedSystemMapShowTime = (uint32)UNIXTIME;
	m_WantedSystemTownControlTime = (uint32)UNIXTIME;
	m_bPermanentChatMode = false;
	m_bSantaOrAngel = FLYING_NONE;
	m_bCzSantaEventActive = false;
	m_CzSantaEventEndTime = 0;
	m_CzSantaNextSpawnTime = 0;
	m_CzSantaLastOpenHour = 99;
	m_sLoginingPlayer = 0;
	m_DrakiRiftTowerRoomIndex = 0;
	m_DungeonDefenceRoomIndex = 0;
	isBowlEventActive = false;
	tBowlEventTime = 0;
	tBowlEventZone = 0;
	m_byBattleSiegeWarNoticeTime = 0;
	m_byBattleSiegeWarType = 0;
	m_byBattleSiegeWarStartTime = 0;
	m_byBattleSiegeWarOccupyTime = 0;
	m_byBattleSiegeWarRemainingTime = 0;
	m_byBattleSiegeWarOpen = false;
	m_byBattleSiegeWarAttack = false;
	m_byBattleSiegeWarMonument = false;
	m_CSWTimer = 0;
	m_LoginServerConnected = false;
	MaxSpawnCount = 0;
	MaxBotFinish = 0;
	BotStepCount = 0;
	BotAutoType = 0;
	botautoX = 0;
	botautoZ = 0;
	BotRespawnTick = 0;
	BotTimeNext = 0;

	for (int i = 0; i < 18; i++)
		pChaosReward[i].Initialize();

	pEventBridge.Initialize();
	pRankBug.Initiliaze();
	pServerSetting.Initialize();
	m_afkqueue = NULL;
	timershowreset = false;

	m_timershowlistLock.lock();
	m_timershowlist.clear();
	m_timershowlistLock.unlock();

	pDamageSetting.Initialize();
	pSpecialEvent.Initialize();
	pSiegeWar.Initialize();

	for (int i = 0; i < 3; i++)
		pBurningFea[i].Initialize();

	memset(mLotteryRewardItem, 0, sizeof(mLotteryRewardItem));
	mLotteryUserList.clear();

	for (int i = 0; i < 2; i++)
		pJackPot[i].Initialize();

	memset(ClanBankGetID, 0, sizeof(ClanBankGetID));
	memset(ClanBankGetZone, 0, sizeof(ClanBankGetZone));
}
#pragma endregion

// DeaFSezo Fonksiyon metin rengini ayarlamak için kullanılır Ekledim 28.12.2024
void CGameServerDlg::SetConsoleColor(WORD color)
{
	HANDLE hConsole = GetStdHandle(STD_OUTPUT_HANDLE);
	SetConsoleTextAttribute(hConsole, color);
}

void PrintHeader() 
{
	HANDLE hConsole = GetStdHandle(STD_OUTPUT_HANDLE);

	// DeaFSezo Setting text color to red and printing the header Ekledim 28.12.2024
	g_pMain->SetConsoleColor(FOREGROUND_RED + FOREGROUND_BLUE); // DeaFSezo Renk Mor Mükemmel Ekledim 28.12.2024
	std::cout << "                               ************************************************" << std::endl;
	std::cout << "                               *                                              *" << std::endl;
	std::cout << "                               *     JstKO V1098 & V2385 SERVER FILES         *" << std::endl;
	std::cout << "                               *                                              *" << std::endl;
	std::cout << "                               *                                              *" << std::endl;
	std::cout << "                               *             DISCORD : JStVK                  *" << std::endl;
	std::cout << "                               *				              *" << std::endl;
	std::cout << "                               *                                              *" << std::endl;
	std::cout << "                               ************************************************" << std::endl;

	// DeaFSezo Resetting text color to default Ekledim 28.12.2024
	g_pMain->SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE);
}

bool CGameServerDlg::Startup()
{
	StartupTrace("Startup: enter");
	PrintHeader();  // DeaFSezo Call the function to print the header Ekledim 28.12.2024
	StartupTrace("Startup: PrintHeader ok");

	DateTime time;

	// DeaFSezo Server Start - Lisans Bilgileri Ekledim 28.12.2024
	SetConsoleColor(FOREGROUND_GREEN | FOREGROUND_INTENSITY);
	printf("Server started on %04d-%02d-%02d at %02d:%02d\n\n", time.GetYear(), time.GetMonth(), time.GetDay(), time.GetHour(), time.GetMinute());
	StartupTrace("Startup: initial banner ok");

#if LISANS_SYSTEM  == 1 // DeaFSezo Lisans System Açıp Kapatma Define.h Ayarlar Ekledim 28.12.2024

	// DeaFSezo License System Stard System.
	m_HardwareIDArray.push_back(9223372036854775807);
	m_HardwareIDArray.push_back(4650229548);

	if (!g_HardwareInformation.IsValidHardwareID(m_HardwareIDArray))
	{
		SetConsoleColor(FOREGROUND_RED | FOREGROUND_INTENSITY);
		printf("new hddid=%I64d\n", g_HardwareInformation.GetHardwareID());
		printf("Gecersiz Lisans, Lutfen Lisans Aliniz.Server Kapatiliyor. :(\n");
		SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE);
		return false;
	}
	// DeaFSezo License System End System.

	g_pMain->LicenseSystem();
#endif // DeaFSezo Lisans System The End 28.12.2024

	GetTimeFromIni();
	StartupTrace("Startup: GetTimeFromIni ok");
	GetEventAwardsIni();
	StartupTrace("Startup: GetEventAwardsIni ok");
	JstKOServerSettings(); // JstKO Files x64 JstKOSettings.ini Ayarlar Eklendi 01.01.2025 
	StartupTrace("Startup: JstKOServerSettings ok");

	if (!g_DBAgent.Startup(m_bMarsEnabled, m_strAccountDSN, m_strAccountUID, m_strAccountPWD, m_strGameDSN, m_strGameUID, m_strGamePWD, m_strLogDSN, m_strLogUID, m_strLogPWD))
	{
		StartupTrace("Startup: g_DBAgent.Startup FAILED");
		SetConsoleColor(FOREGROUND_RED | FOREGROUND_INTENSITY);
		printf("ERROR: Unable to connect to the database using the details configured.\n");
		SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE);
		return false;
	}
	StartupTrace("Startup: g_DBAgent.Startup ok");

	SetConsoleColor(FOREGROUND_GREEN | FOREGROUND_INTENSITY);
	printf("Database Baglantisi [ OK ].\n");
	Sleep(100);

	// Load all tables from the database
	if (!LoadTables())
		return false;

	// Clear any remaining users in the currently logged in list
	// that may be left as a result of an improper shutdown.
	g_DBAgent.ClearRemainUsers();
	g_DBAgent.LoadJstKOWelcomeLogosNotice(); // JstKO Welcome Logos Notice Eklendi 01.01.2025
	CreateDirectories();

	// system("cls");
	LoadNoticeData();
	LoadNoticeUpData();

	printf("\n");
	if (!m_luaEngine.Initialise())
		return false;

	// Initialise the command tables
	InitServerCommands();
	CUser::InitChatCommands();

	ResetBeefEvent();
	EventTimerSet();
	// Disabled: spawning auto bots at startup was causing visible fake logins at game launch.
	// JstKOAddBotMiningStart();
	g_pMain->UpdateFlagAndCape();

	if (!StartTheardSocketSystem())
		return false;

	if (!StartUserSocketSystem())
	{
		SetConsoleColor(FOREGROUND_RED | FOREGROUND_INTENSITY);
		printf("HATA : Failed to listen on server port (%d).\n", m_GameServerPort);
		SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE);
		return false;
	}

	m_afkqueue = new AdiniFerihaKoydum();

	// Resetting text color to default
	SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE);
	return true;
}
void CGameServerDlg::EventTimerSet()
{
	if (timershowreset)
		return;

	timershowreset = true;

	uint32 nWeekDay = g_localTime.tm_wday, nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;

	std::vector< EVENT_OPENTIMELIST> copymtimeforloop = pEventTimeOpt.mtimeforloop;

	m_timershowlistLock.lock();
	m_timershowlist.clear();
	foreach(ptime, copymtimeforloop)
	{
		if (!ptime->status || !isgetonday(ptime->iday))
			continue;

		for (int timei = 0; timei < EVENT_START_TIMES; timei++)
		{
			if (!ptime->timeactive[timei])
				continue;

			m_timershowlist.push_back(timershow(ptime->name, ptime->hour[timei], ptime->minute[timei]));
		}
	}
	m_timershowlistLock.unlock();

	g_pMain->m_EventTimerShowArray.m_lock.lock();
	auto copymap = g_pMain->m_EventTimerShowArray.m_UserTypeMap;
	g_pMain->m_EventTimerShowArray.m_lock.unlock();

	m_timershowlistLock.lock();
	foreach(itr, copymap)
	{
		auto* p = itr->second;
		if (!p || !p->status)
			continue;

		std::list<std::string> nInGameEvent = StrSplit(p->days, ",");
		uint8 daysize = (uint8)nInGameEvent.size();
		if (!daysize)
			continue;

		uint8 nInGameDays = 0;
		for (int i = 0; i < daysize; i++)
		{
			nInGameDays = atoi(nInGameEvent.front().c_str());
			if (nInGameDays == nWeekDay && p->hour >= 0 && p->hour <= 23)
				m_timershowlist.push_back(timershow(p->name, p->hour, p->minute));

			nInGameEvent.pop_front();
		}
	}
	m_timershowlistLock.unlock();

	timershowreset = false;
}

#pragma region CGameServerDlg::StartTheardSocketSystem()
bool CGameServerDlg::StartTheardSocketSystem()
{
	g_timerThreads.push_back(new Thread(Timer_CheckGameEvents));
	g_timerThreads.push_back(new Thread(Timer_UpdateGameTime));
	g_timerThreads.push_back(new Thread(Timer_UpdateSessions));
	g_timerThreads.push_back(new Thread(Timer_BotSessions));
	//g_timerThreads.push_back(new Thread(Timer_t_1));
	//g_timerThreads.push_back(new Thread(Timer_t_2));
	//g_timerThreads.push_back(new Thread(Timer_t_3));
	//g_timerThreads.push_back(new Thread(Timer_UpdateConcurrent));
	g_timerThreads.push_back(new Thread(Timer_TimedNotice));
	g_timerThreads.push_back(new Thread(Timer_BotMoving));
	//g_timerThreads.push_back(new Thread(Timer_BotAutoSpawn));
	return true;
}
#pragma endregion

#pragma region CGameServerDlg::StartUserSocketSystem()
bool CGameServerDlg::StartUserSocketSystem()
{
	if (!m_socketMgr.Listen(m_GameServerPort, MAX_USER))
		return false;

	m_socketMgr.GetIdleSessionMap().erase(0);
	m_socketMgr.RunServer(true);
	return true;
}
#pragma endregion

#pragma region CGameServerDlg::LoadTables()

bool CGameServerDlg::LoadTables()
{

	if (!GameStartClearRemainUser()
		|| !CheckTrashItemListTime()
		|| !LoadCollectionRaceEventTable()
		|| !LoadItemTable()
		|| !LoadItemSellTable()
		|| !LoadSetItemTable()
		|| !LoadItemExchangeTable()
		|| !LoadItemExchangeExpTable()
		|| !LoadItemSpecialExchangeTable()
		|| !LoadItemExchangeCrashTable()
		//|| !LoadItemUpgradeTable()
		|| !LoadItemOpTable()
		|| !LoadServerResourceTable()
		|| !LoadQuestHelperTable()
		|| !LoadQuestMonsterTable()
		|| !LoadMagicTable()
		|| !LoadMagicType1()
		|| !LoadMagicType2()
		|| !LoadMagicType3()
		|| !LoadMagicType4()
		|| !LoadMagicType5()
		|| !LoadMagicType6()
		|| !LoadMagicType7()
		|| !LoadMagicType8()
		|| !LoadMagicType9()
		|| !LoadObjectPosTable()
		|| !LoadPusItemsTable()
		|| !LoadRentalList()
		|| !LoadMakeLareItemTableData()
		|| !LoadMakeGradeItemTableData()
		|| !LoadMakeDefensiveItemTableData()
		|| !LoadMakeWeaponItemTableData()
		|| !LoadMakeItemGroupTable()
		|| !LoadMakeItemGroupRandomTable()
		|| !LoadNpcItemTable()
		|| !LoadMonsterItemTable()
		|| !LoadCharacterSetValidTable()
		|| !LoadGiftEventArray()
		|| !LoadSendMessageTable()
		|| !LoadRightTopTitleTable()
		|| !LoadTopLeftTable()
		|| !LoadNpcTableData(false)
		|| !LoadNpcTableData(true)
		//|| !MapFileLoad()
		|| !LoadCoefficientTable()
		|| !LoadCoefficientDamageTable()
		|| !LoadLevelUpTable()
		|| !LoadAllKnights()
		|| !LoadKnightsAllianceTable()
		|| !LoadKnightsSiegeWarsTable()
		|| !LoadUserRankings()
		|| !LoadCharacterSealItemTable()
		|| !LoadCharacterSealItemTableAll()
		|| !LoadKnightsCapeTable()
		|| !LoadCapeCastellanBonus()
		|| !LoadKnightsRankTable()
		|| !LoadStartPositionTable()
		|| !LoadStartPositionRandomTable()
		|| !LoadBattleTable()
		|| !MapFileLoad()
		|| !LoadKingSystem()
		|| !LoadSheriffTable()
		|| !LoadEventTriggerTable()
		|| !LoadMonsterResourceTable()
		|| !LoadMonsterChallengeTable()
		|| !LoadMonsterChallengeSummonListTable()
		|| !LoadMonsterSummonListTable()
		|| !LoadChaosStoneMonsterListTable()
		|| !LoadChaosStoneCoordinateTable()
		|| !LoadChaosStoneStage()
		|| !LoadMonsterUnderTheCastleTable()
		|| !LoadMonsterStoneListInformationTable()
		|| !LoadJuraidMountionListInformationTable()
		|| !LoadMiningFishingItemTable()
		|| !LoadPremiumItemTable()
		|| !LoadPremiumItemExpTable()
		|| !LoadItemPremiumGiftTable()
		|| !LoadMiningExchangeListTable()
		|| !LoadUserDailyOpTable()
		|| !LoadUserItemTable()
		|| !LoadAchieveTitleTable()
		|| !LoadAchieveMainTable()
		|| !LoadAchieveMonsterTable()
		|| !LoadAchieveNormalTable()
		|| !LoadAchieveComTable()
		|| !LoadAchieveWarTable()
		|| !LoadDrakiTowerTables()
		|| !LoadDungeonDefenceMonsterTable()
		|| !LoadDungeonDefenceStageTable()
		|| !LoadKnightReturnLetterGiftItemTable()
		|| !LoadBanishWinnerTable()
		|| !LoadGiveItemExchangeTable()
		|| !LoadBeefEventPlayTimerTable()
		|| !LoadFtEventPlayTimerTable()
		|| !LoadServerSettingsData()
		|| !LoadBotTable()
		|| !LoadPetStatsInfoTable()
		|| !LoadPetImageChangeTable()
		|| !LoadBotMerchantTable()
		|| !LoadMonsterRespawnLoopListTable()
		|| !LoadUserLootSettingsTable()
		|| !LoadRimaLotteryEventTable()
		|| !LoadRandomBoosStageTable()
		|| !LoadRandomBossTable()
		|| !LoadZindanWarStageListTable()
		|| !LoadZindanWarSummonListTable()
		|| !LoadAntiAfkListTable()
		|| !LoadDailyQuestListTable()
		|| !LoadDailyRankTable()
		|| !LoadTimedNoticeTable()
		|| !LoadRankBugTable()
		|| !LoadWheelDataSetTable()
		|| !LoadEventTimerShowTable()
		|| !LoadDamageSettingTable()
		|| !XCodeLoadEventTables()
		|| !XCodeLoadEventVroomTables()
		|| !LoadCindirellaItemsTable()
		|| !LoadCindirellaStatSetTable()
		|| !LoadCindirellaSettingTable()
		|| !LoadCindirellaRewardsTable()
		|| !_CreateNpcThread()
		|| !CreateBottomZoneMap()
		|| !ChaosStoneLoad()
		|| !RandomBossSystemLoad()
		|| !LoadPusCategoryTable()
		|| !BurningFeatureTable()
		|| !LoadZoneKillReward()
		|| !LoadZoneOnlineRewardTable()
		|| !LoadSaveBotDataTable()
		|| !LoadEventRewardTable()
		|| !LoadCSWTables()
		|| !LoadLevelRewardTable()
		|| !LoadLevelMerchantRewardTable()
		|| !LoadJackPotSettingTable()
		|| !LoadPerksTable()
		|| !LoadRankRewardTable()
		|| !LoadAutomaticCommand()
		|| !LoadForgettenTempleStagesListTable()
		|| !LoadForgettenTempleSummonListTable()
		|| !LoadItemRightClickExchangeTable())
		return false;

	g_DBAgent.SpecialStone();
	g_DBAgent.LoadUpgrade();
	g_DBAgent.LoadItemUpgradeSettings();
	return true;
}

#pragma endregion

bool CGameServerDlg::WordGuardSystem(std::string Word, uint8 WordStr)
{
	char* pword = &Word[0];
	bool bGuard[32] = { false };
	std::string WordGuard = "qwertyuopadfhsgcijklzxvbnmQWERTYUOPSGICADFHJKLZXVBNM1234567890:_";
	char* pWordGuard = &WordGuard[0];
	for (uint8 i = 0; i < WordStr; i++)
	{
		for (uint8 j = 0; j < strlen(pWordGuard); j++)
			if (pword[i] == pWordGuard[j])
				bGuard[i] = true;

		if (bGuard[i] == false)
			return false;
	}
	return true;
}

#pragma region CGameServerDlg::CreateDirectories()

void CGameServerDlg::CreateDirectories()
{
	// Logs Start
	CreateDirectory("Logs", NULL);
}

#pragma endregion

void CGameServerDlg::GetEventAwardsIni()
{
	CIni ini(CONF_EVENT_AWARDS);

	/*CHAOS ÖDÜLLERİ*/
	m_nChaosEventAwards0 = ini.GetInt("CHAOS_EXPANSION", "RANKING_1_TO_3", BLUE_TREASURE_CHEST);
	m_nChaosEventAwards1 = ini.GetInt("CHAOS_EXPANSION", "RANKING_1_TO_3_MORE", VOUCHER_OF_ORACLE);
	m_nChaosEventAwards2 = ini.GetInt("CHAOS_EXPANSION", "RANKING_4_TO_7", GREEN_TREASURE_CHEST);
	m_nChaosEventAwards3 = ini.GetInt("CHAOS_EXPANSION", "RANKING_8_TO_10", RED_TREASURE_CHEST);
	m_nChaosEventAwards4 = ini.GetInt("CHAOS_EXPANSION", "RANKING_MORE", VOUCHER_OF_CHAOS);
	////m_nChaosEventAwards5 = ini.GetInt("CHAOS_EXPANSION", "RANKING_MORES", LUNAR_ORDER_TOKEN);

	/*BORDER DEFANCE WAR ÖDÜLLERİ*/
	m_nBorderEventAwards = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LOYALTY", 500);
	////m_nBorderEventAwards0 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_20_TO_29_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwards1 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_20_TO_29_2", BORDER_SECURITY_SCROLL1);
	m_nBorderEventAwards2 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_20_TO_29_3", RED_TREASURE_CHEST);
	m_nBorderEventAwards3 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_20_TO_29_4", CERTIFICATE_OF_VICTORY);
	////m_nBorderEventAwards4 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_30_TO_39_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwards5 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_30_TO_39_2", BORDER_SECURITY_SCROLL1);
	m_nBorderEventAwards6 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_30_TO_39_3", RED_TREASURE_CHEST);
	m_nBorderEventAwards7 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_30_TO_39_4", CERTIFICATE_OF_VICTORY);
	////m_nBorderEventAwards8 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_40_TO_49_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwards9 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_40_TO_49_2", BORDER_SECURITY_SCROLL2);
	m_nBorderEventAwards10 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_40_TO_49_3", RED_TREASURE_CHEST);
	m_nBorderEventAwards11 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_40_TO_49_4", CERTIFICATE_OF_VICTORY);
	//m_nBorderEventAwards11 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_50_TO_57_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwards12 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_50_TO_57_2", BORDER_SECURITY_SCROLL2);
	m_nBorderEventAwards13 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_50_TO_57_3", RED_TREASURE_CHEST);
	m_nBorderEventAwards14 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_50_TO_57_4", CERTIFICATE_OF_VICTORY);
	//m_nBorderEventAwards15 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_58_TO_64_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwards16 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_58_TO_64_2", BORDER_SECURITY_SCROLL2);
	m_nBorderEventAwards17 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_58_TO_64_3", RED_TREASURE_CHEST);
	m_nBorderEventAwards18 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_58_TO_64_4", CERTIFICATE_OF_VICTORY);
	//m_nBorderEventAwards19 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_65_TO_72_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwards20 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_65_TO_72_2", BORDER_SECURITY_SCROLL2);
	m_nBorderEventAwards21 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_65_TO_72_3", RED_TREASURE_CHEST);
	m_nBorderEventAwards22 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_65_TO_72_4", CERTIFICATE_OF_VICTORY);
	//m_nBorderEventAwards23 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_73_TO_80_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwards24 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_73_TO_80_2", BORDER_SECURITY_SCROLL2);
	m_nBorderEventAwards25 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_73_TO_80_3", RED_TREASURE_CHEST);
	m_nBorderEventAwards26 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_73_TO_80_4", CERTIFICATE_OF_VICTORY);
	//m_nBorderEventAwards27 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_81_TO_MAX_LEVEL_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwards28 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_81_TO_MAX_LEVEL_2", BORDER_SECURITY_SCROLL2);
	m_nBorderEventAwards29 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_81_TO_MAX_LEVEL_3", RED_TREASURE_CHEST);
	m_nBorderEventAwards30 = ini.GetInt("BORDER_DEFENSE_WAR", "WINNING_LEVEL_81_TO_MAX_LEVEL_4", CERTIFICATE_OF_VICTORY);

	m_nBorderEventAwardsLoser = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LOYALTY", 100);
	//m_nBorderEventAwardsLoser0 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_20_TO_29_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwardsLoser1 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_20_TO_29_2", BORDER_SECURITY_SCROLL1);
	//m_nBorderEventAwardsLoser2 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_30_TO_39_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwardsLoser3 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_30_TO_39_2", BORDER_SECURITY_SCROLL1);
	//m_nBorderEventAwardsLoser4 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_40_TO_49_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwardsLoser5 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_40_TO_49_2", BORDER_SECURITY_SCROLL2);
	//m_nBorderEventAwardsLoser6 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_50_TO_57_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwardsLoser7 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_50_TO_57_2", BORDER_SECURITY_SCROLL2);
	//m_nBorderEventAwardsLoser8 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_58_TO_64_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwardsLoser9 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_58_TO_64_2", BORDER_SECURITY_SCROLL2);
	//m_nBorderEventAwardsLoser10 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_65_TO_72_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwardsLoser11 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_65_TO_72_2", BORDER_SECURITY_SCROLL2);
	//m_nBorderEventAwardsLoser12 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_73_TO_80_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwardsLoser13 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_73_TO_80_2", BORDER_SECURITY_SCROLL2);
	//m_nBorderEventAwardsLoser14 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_81_TO_MAX_LEVEL_1", LUNAR_ORDER_TOKEN);
	m_nBorderEventAwardsLoser15 = ini.GetInt("BORDER_DEFENSE_WAR", "LOSER_LEVEL_81_TO_MAX_LEVEL_2", BORDER_SECURITY_SCROLL2);

	/*Juraid Mountain Defense Ödülleri*/
	m_nJuraidEventAwards = ini.GetInt("JURAID_MOUNTION_DEFENSE", "WINNING", SILVERY_GEM);
	m_nJuraidEventAwards1 = ini.GetInt("JURAID_MOUNTION_DEFENSE", "LOSER", BLACK_GEM);

	/*Castle Siege Warfare Defense Ödülleri*/
	m_nCastleSiegeWarfareLoyaltyEventAwards = ini.GetInt("CASTLE_SIEGE_WARFARE", "WINNING_LOYALTY", 3000);
	m_nCastleSiegeWarfareLoyaltyEventAwardsLoser = ini.GetInt("CASTLE_SIEGE_WARFARE", "LOSER_LOYALTY", 500);
	m_nCastleSiegeWarfareCashPointEventAwards = ini.GetInt("CASTLE_SIEGE_WARFARE", "WINNING_CASH_POINT", 300);
	m_nCastleSiegeWarfareCashPointEventAwardsLoser = ini.GetInt("CASTLE_SIEGE_WARFARE", "LOSER_CASH_POINT", 100);

	/*VS Event*/
	vsEventMaxNp = ini.GetInt("VS_EVENT", "MAX_NP", 1000);
	vsEventMinNp = ini.GetInt("VS_EVENT", "MIN_NP", 50);
}
/**
* @brief	Loads the server's config from the INI file.
*/
void CGameServerDlg::GetTimeFromIni()
{
	CIni ini(CONF_GAME_SERVER);
	CIni Events(CONF_EVENT_SETTINGS);
	int year = 0, month = 0, date = 0, hour = 0, server_count = 0, sgroup_count = 0, i = 0;
	char ipkey[20];

	// This is so horrible.
	ini.GetString("ODBC", "GAME_DSN", "KO_GAME", m_strGameDSN, false);
	ini.GetString("ODBC", "GAME_UID", "username", m_strGameUID, false);
	ini.GetString("ODBC", "GAME_PWD", "password", m_strGamePWD, false);
	ini.GetString("ODBC", "ACCOUNT_DSN", "KO_MAIN", m_strAccountDSN, false);
	ini.GetString("ODBC", "ACCOUNT_UID", "username", m_strAccountUID, false);
	ini.GetString("ODBC", "ACCOUNT_PWD", "password", m_strAccountPWD, false);
	ini.GetString("ODBC", "LOG_DSN", "KO_LOG", m_strLogDSN, false);
	ini.GetString("ODBC", "LOG_UID", "username", m_strLogUID, false);
	ini.GetString("ODBC", "LOG_PWD", "password", m_strLogPWD, false);

	m_bMarsEnabled = ini.GetBool("ODBC", "GAME_MARS", true);
	bool bMarsEnabled = ini.GetBool("ODBC", "ACCOUNT_MARS", true);
	bMarsEnabled = ini.GetBool("ODBC", "LOG_MARS", true);

	// Both need to be enabled to use MARS.
	if (!m_bMarsEnabled || !bMarsEnabled)
		m_bMarsEnabled = false;

	m_byWeather = ini.GetInt("TIMER", "WEATHER", 1);
	m_nCastleCapture = ini.GetInt("CASTLE", "NATION", 1);
	m_nServerNo = ini.GetInt("ZONE_INFO", "MY_INFO", 1);
	m_nServerGroup = ini.GetInt("ZONE_INFO", "SERVER_NUM", 0);
	server_count = ini.GetInt("ZONE_INFO", "SERVER_COUNT", 1);
	if (server_count < 1)
	{
		printf("ERROR: Invalid SERVER_COUNT property in INI.\n");
		return;
	}

	for (i = 0; i < server_count; i++)
	{
		_ZONE_SERVERINFO* pInfo = new _ZONE_SERVERINFO;
		sprintf(ipkey, "SERVER_%02d", i);
		pInfo->sServerNo = ini.GetInt("ZONE_INFO", ipkey, 1);
		sprintf(ipkey, "SERVER_IP_%02d", i);
		ini.GetString("ZONE_INFO", ipkey, "127.0.0.1", pInfo->strServerIP);
		m_ServerArray.PutData(pInfo->sServerNo, pInfo);
	}

	if (m_nServerGroup != 0)
	{
		m_nServerGroupNo = ini.GetInt("SG_INFO", "GMY_INFO", 1);
		sgroup_count = ini.GetInt("SG_INFO", "GSERVER_COUNT", 1);
		if (server_count < 1)
		{
			printf("ERROR: Invalid GSERVER_COUNT property in INI.\n");
			return;
		}

		for (i = 0; i < sgroup_count; i++)
		{
			_ZONE_SERVERINFO* pInfo = new _ZONE_SERVERINFO;
			sprintf(ipkey, "GSERVER_%02d", i);
			pInfo->sServerNo = ini.GetInt("SG_INFO", ipkey, 1);
			sprintf(ipkey, "GSERVER_IP_%02d", i);
			ini.GetString("SG_INFO", ipkey, "127.0.0.1", pInfo->strServerIP);

			m_ServerGroupArray.PutData(pInfo->sServerNo, pInfo);
		}
	}

	ClanPreFazlaNP = ini.GetInt("CLAN_PREMIUM", "NPBONUS", 100); // direk üzerine ekler 50 + 100 gibi
	ClanPreFazlagold = ini.GetInt("CLAN_PREMIUM", "GOLDBONUS", 2);
	ClanPrePazar = ini.GetBool("CLAN_PREMIUM", "PREMIUM-MERCHANT", true);
	ClanPreFazlaExp = ini.GetInt("CLAN_PREMIUM", "EXPBONUS", 10); // yüzde olarak ekler

	// Clan Buff System 
	ClanBuffSystemStatus = Events.GetBool("CLAN_BUFF_SYSTEM", "STATUS", true);
	ClanBuff5UserOnlineExp = Events.GetInt("CLAN_BUFF_SYSTEM", "5_ONLINE_EXP", 1);
	ClanBuff10UserOnlineExp = Events.GetInt("CLAN_BUFF_SYSTEM", "10_ONLINE_EXP", 1);
	ClanBuff15UserOnlineExp = Events.GetInt("CLAN_BUFF_SYSTEM", "15_ONLINE_EXP", 1);
	ClanBuff20UserOnlineExp = Events.GetInt("CLAN_BUFF_SYSTEM", "20_ONLINE_EXP", 1);
	ClanBuff25UserOnlineExp = Events.GetInt("CLAN_BUFF_SYSTEM", "25_ONLINE_EXP", 1);
	ClanBuff30UserOnlineExp = Events.GetInt("CLAN_BUFF_SYSTEM", "30_ONLINE_EXP", 1);

	ClanBuff5UserOnlineNP = Events.GetInt("CLAN_BUFF_SYSTEM", "5_ONLINE_NP", 1);
	ClanBuff10UserOnlineNP = Events.GetInt("CLAN_BUFF_SYSTEM", "10_ONLINE_NP", 1);
	ClanBuff15UserOnlineNP = Events.GetInt("CLAN_BUFF_SYSTEM", "15_ONLINE_NP", 1);
	ClanBuff20UserOnlineNP = Events.GetInt("CLAN_BUFF_SYSTEM", "20_ONLINE_NP", 1);
	ClanBuff25UserOnlineNP = Events.GetInt("CLAN_BUFF_SYSTEM", "25_ONLINE_NP", 1);
	ClanBuff30UserOnlineNP = Events.GetInt("CLAN_BUFF_SYSTEM", "30_ONLINE_NP", 1);

	ClanBankStatus = ini.GetBool("CLAN_BANK", "STATUS", true);

	m_GameServerPort = ini.GetInt("SETTINGS", "PORT", 15001);

	ini.GetString("SETTINGS", "LOGIN_IP", "127.0.0.1", m_LoginServerIP, false);
	m_LoginServerPort = ini.GetInt("SETTINGS", "LOGIN_PORT", 15100);

	m_byMaxLevel = ini.GetInt("SETTINGS", "GAMEMAXLEVEL", 83);
	m_nGameMasterRHitDamage = ini.GetInt("SETTINGS", "GAME_MASTER_R_HIT_DAMAGE", 30000);
	m_nPlayerRankingResetTime = ini.GetInt("SETTINGS", "PLAYER_RANKINGS_RESET_TIME", 1);
	ini.GetString("SETTINGS", "PLAYER_RANKINGS_REWARD_ZONES", "71,72,73", m_sPlayerRankingsRewardZones, false);
	m_nPlayerRankingKnightCashReward = ini.GetInt("SETTINGS", "PLAYER_RANKINGS_KNIGHT_CASH_REWARD", 0);
	m_nPlayerRankingLoyaltyReward = ini.GetInt("SETTINGS", "PLAYER_RANKINGS_LOYALTY_REWARD", 0);

	m_Grade1 = ini.GetInt("CLAN_GRADE", "GRADE1", 720000);
	m_Grade2 = ini.GetInt("CLAN_GRADE", "GRADE2", 360000);
	m_Grade3 = ini.GetInt("CLAN_GRADE", "GRADE3", 144000);
	m_Grade4 = ini.GetInt("CLAN_GRADE", "GRADE4", 72000);

	// Merchant View List Merhanct User
	MerchantViewInfoNotice = ini.GetBool("MERCHANT_VIEW_INFO_NOTICE", "STATUS", true); //
	m_byExpEventAmount = ini.GetInt("BONUS", "EXP", 0);
	m_byCoinEventAmount = ini.GetInt("BONUS", "MONEY", 0);
	m_byNPEventAmount = ini.GetInt("BONUS", "NP", 0);
	m_byDropEventAmount = ini.GetInt("BONUS", "DROP", 0);

	m_Loyalty_Ardream_Source = ini.GetInt("NATIONAL_POINTS", "ARDREAM_SOURCE", 32);
	m_Loyalty_Ardream_Target = ini.GetInt("NATIONAL_POINTS", "ARDREAM_TARGET", -25);
	m_Loyalty_Ronark_Land_Base_Source = ini.GetInt("NATIONAL_POINTS", "RONARK_LAND_BASE_SOURCE", 64);
	m_Loyalty_Ronark_Land_Base_Target = ini.GetInt("NATIONAL_POINTS", "RONARK_LAND_BASE_TARGET", -50);
	m_Loyalty_Ronark_Land_Source = ini.GetInt("NATIONAL_POINTS", "RONARK_LAND_SOURCE", 64);
	m_Loyalty_Ronark_Land_Target = ini.GetInt("NATIONAL_POINTS", "RONARK_LAND_TARGET", -50);
	m_Loyalty_Other_Zone_Source = ini.GetInt("NATIONAL_POINTS", "OTHER_ZONE_SOURCE", 64);
	m_Loyalty_Other_Zone_Target = ini.GetInt("NATIONAL_POINTS", "OTHER_ZONE_TARGET", -50);

	m_sKarusMilitary = ini.GetInt("MILITARY_CAMP", "KARUS_ZONE", 0);
	m_sKarusEslantMilitary = ini.GetInt("MILITARY_CAMP", "KARUS_ESLANT_ZONE", 0);
	m_sHumanMilitary = ini.GetInt("MILITARY_CAMP", "HUMAN_ZONE", 0);
	m_sHumanEslantMilitary = ini.GetInt("MILITARY_CAMP", "HUMAN_ESLANT_ZONE", 0);
	m_sMoradonMilitary = ini.GetInt("MILITARY_CAMP", "MORADON_ZONE", 0);

	m_sJackExpPots = ini.GetInt("JACK_POTS", "JACK_EXP_POT", 0);
	m_sJackGoldPots = ini.GetInt("JACK_POTS", "JACK_NOAH_POT", 0);

	// iniden tbl verisi alma AA11BB22CC99
	ini.GetString("TBL_HASH", "ITEM_ORG", "", server_itemorg);
	ini.GetString("TBL_HASH", "MAGIC_MAIN", "", server_skillmagic);
	ini.GetString("TBL_HASH", "ZONES", "", server_zones);
	ini.GetString("TBL_HASH", "MAGIC_MAIN_TK", "", server_skillmagictk);
	KingRewardKC = ini.GetInt("KING_REWARD_CASH_POINT", "CASH_POINT", 100);

	if (m_sKarusMilitary > 3
		|| m_sKarusEslantMilitary > 3
		|| m_sHumanMilitary > 3
		|| m_sHumanEslantMilitary > 3
		|| m_sMoradonMilitary > 5)
	{
		if (m_sKarusMilitary > 3)
			m_sKarusMilitary = 3;

		if (m_sKarusEslantMilitary > 3)
			m_sKarusEslantMilitary = 3;

		if (m_sHumanMilitary > 3)
			m_sHumanMilitary = 3;

		if (m_sHumanEslantMilitary > 3)
			m_sHumanEslantMilitary = 3;

		if (m_sMoradonMilitary > 5)
			m_sMoradonMilitary = 5;
	}

	m_sRankResetHour = 0;
	m_RankRewardSendStatus = false;
	m_GameInfo1Time = 1800;
	m_GameInfo2Time = 3600;

	ChaosStoneRespawnOkey = true;

	m_Shutdownfinishtime = 0;
	m_Shutdownstart = false;
	m_Shutdownfinished = false;
	m_ShutdownKickStart = false;

	m_byBattleOpenedTime = 0;
	m_byBattleNoticeTime = 0;
	m_byBattleTime = (2 * 60) * 60;	// 2 Hours

	m_IsMagicTableInUpdateProcess = false;
	m_IsPlayerRankingUpdateProcess = false;

	m_nPVPMonumentNation[ZONE_RONARK_LAND] = 0;

	//Monster Stone EventRoom
	m_MonsterStoneSquadEventRoom = MAX_TEMPLE_QUEST_EVENT_ROOM;

	m_UserPlayerKillingZoneRankingArray[0].DeleteAllData();
	m_UserPlayerKillingZoneRankingArray[1].DeleteAllData();
	m_UserBorderDefenceWarRankingArray[0].DeleteAllData();
	m_UserBorderDefenceWarRankingArray[1].DeleteAllData();
	m_UserChaosExpansionRankingArray.DeleteAllData();
	m_ZindanWarZoneRankingArray[0].DeleteAllData();
	m_ZindanWarZoneRankingArray[1].DeleteAllData();

	m_CurrentNPC = 0;
	m_TotalNPC = 0;

	//Soccer Event System
	pSoccerEvent.m_SoccerActive = false;
	pSoccerEvent.m_SoccerTimer = false;
	pSoccerEvent.m_SoccerSocketID = -1;
	pSoccerEvent.m_SoccerTime = 0;
	pSoccerEvent.m_SoccerTimers = 0;
	pSoccerEvent.m_SoccerRedColour = 0;
	pSoccerEvent.m_SoccerBlueColour = 0;
	pSoccerEvent.m_SoccerBlueGool = 0;
	pSoccerEvent.m_SoccerRedGool = 0;

	for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++)
	{
		_BDW_ROOM_INFO* p_1 = new _BDW_ROOM_INFO();
		p_1->Initialize();
		m_TempleEventBDWRoomList.PutData(i, p_1);

		_CHAOS_ROOM_INFO* p_2 = new _CHAOS_ROOM_INFO();
		p_2->Initialize();
		m_TempleEventChaosRoomList.PutData(i, p_2);

		_JURAID_ROOM_INFO* p_3 = new _JURAID_ROOM_INFO();
		p_3->Initialize();
		m_TempleEventJuraidRoomList.PutData(i, p_3);
	}

	for (int i = 1; i <= 50; i++)
	{
		_CHAOS_STONE_INFO* pRoomInfo = new _CHAOS_STONE_INFO();
		pRoomInfo->Initialize();
		pRoomInfo->sChaosIndex = i;
		m_ChaosStoneInfoArray.PutData(i, pRoomInfo);
	}

	if (m_MonsterUnderTheCastleList.GetSize() == 0)
	{
		_UNDER_THE_CASTLE_INFO* pRoomInfo = new _UNDER_THE_CASTLE_INFO();
		pRoomInfo->Initialize();
		m_MonsterUnderTheCastleList.PutData(1, pRoomInfo);
	}

	if (m_MonsterDrakiTowerList.GetSize() == 0)
	{
		for (int i = 1; i <= EVENTMAXROOM; i++)
		{
			_DRAKI_TOWER_INFO* pRoomInfo = new _DRAKI_TOWER_INFO();
			pRoomInfo->Initialize();
			pRoomInfo->m_tDrakiRoomID = i;
			m_MonsterDrakiTowerList.PutData(pRoomInfo->m_tDrakiRoomID, pRoomInfo);
		}
	}

	for (int i = 0; i < MAX_MONSTER_STONE_ROOM; i++)
	{
		auto& p1 = m_TempleEventMonsterStoneRoomList[i];
		p1.Initialize();
		p1.icanbeused = true;
		p1.roomid = i;
	}

	if (m_DungeonDefenceRoomListArray.GetSize() == 0)
	{
		for (int i = 1; i <= EVENTMAXROOM; i++)
		{
			_DUNGEON_DEFENCE_ROOM_INFO* pRoomInfo = new _DUNGEON_DEFENCE_ROOM_INFO();
			pRoomInfo->Initialize();
			pRoomInfo->m_DefenceRoomID = i;
			m_DungeonDefenceRoomListArray.PutData(pRoomInfo->m_DefenceRoomID, pRoomInfo);
		}
	}

	for (int i = 0; i < 3; i++) pWantedMain[i].Initialize();

	pTempleEvent.Initialize();
	m_TempleEventLastUserOrder = 1;

	pBeefEvent.isActive = false;
	pBeefEvent.isAttackable = false;
	pBeefEvent.isMonumentDead = false;
	pBeefEvent.isFarmingPlay = false;
	pBeefEvent.WictoryNation = 0;
	pBeefEvent.BeefSendTime = 0;
	pBeefEvent.BeefMonumentStartTime = 0;
	pBeefEvent.BeefFarmingPlayTime = 0;
	pBeefEvent.LoserNationSignTime = 0;
	pBeefEvent.isLoserSign = false;

	pLotteryProc.Initialize();

	// 29.10.2020 Boss Atınca Notice Gecmesi Engellendi start
	IsSummon = false;
	NameGm = "";
	// 29.10.2020 Boss Atınca Notice Gecmesi Engellendi end

	// Under The Castle
	m_bUnderTheCastleIsActive = false;
	m_bUnderTheCastleMonster = false;
	m_nUnderTheCastleEventTime = 0;
	m_checktime = 60 * SECOND;
}

/**
* @brief	Gets & formats a cached server resource (_SERVER_RESOURCE entry).
*
* @param	nResourceID	Identifier for the resource.
* @param	result	   	The string to store the formatted result in.
*/
void CGameServerDlg::GetServerResource(int nResourceID, string* result, ...)
{
	_SERVER_RESOURCE* pResource = m_ServerResourceArray.GetData(nResourceID);
	if (pResource == nullptr)
	{
		*result = nResourceID;
		return;
	}

	va_list args;
	va_start(args, result);
	_string_format(pResource->strResource, result, args);
	va_end(args);
}

/**
* @brief	Gets the starting positions (for both nations)
* 			for the specified zone.
*
* @param	nZoneID	Identifier for the zone.
*/
_START_POSITION* CGameServerDlg::GetStartPosition(int nZoneID)
{
	return m_StartPositionArray.GetData(nZoneID);
}

/**
* @brief	Gets the experience points required for the
* 			specified level.
*
* @param	nLevel	The level.
*
* @return	The experience points required to level up from
* 			the specified level.
*/
int64 CGameServerDlg::GetExpByLevel(int nLevel, int RebithLevel)
{
	foreach(itr, g_pMain->m_LevelUpArray)
	{
		if (itr->second == nullptr)
			continue;

		if (nLevel < MAX_LEVEL && RebithLevel > 0)
		{
			if (itr->second->Level == nLevel && itr->second->RebithLevel == 0)
				return itr->second->Exp;
		}
		else
		{
			if (itr->second->Level == nLevel && itr->second->RebithLevel == RebithLevel)
				return itr->second->Exp;
		}
	}
	return 0;
}

/**
* @brief	Gets zone by its identifier.
*
* @param	zoneID	Identifier for the zone.
*
* @return	null if it fails, otherwise the zone.
*/
C3DMap* CGameServerDlg::GetZoneByID(int zoneID)
{
	return m_ZoneArray.GetData(zoneID);
}

CBot* CGameServerDlg::GetBotPtr(string findName, NameType type)
{
	// As findName is a copy of the string passed in, we can change it
	// without worry of affecting anything.
	STRTOUPPER(findName);

	BotNameMap::iterator itr;
	if (type == NameType::TYPE_CHARACTER)
	{
		m_BotcharacterNameLock.lock();
		itr = m_BotcharacterNameMap.find(findName);
		m_BotcharacterNameLock.unlock();
		return (itr != m_BotcharacterNameMap.end() ? itr->second : nullptr);
	}

	return nullptr;
}

void CGameServerDlg::AddMapBotList(CBot* pSession)
{
	if (pSession == nullptr)
		return;

	m_MapBotListLock.lock();
	m_MapBotList.PutData(pSession->GetID(), pSession);
	m_MapBotListLock.unlock();

	string upperName = pSession->GetName();
	STRTOUPPER(upperName);
	m_BotcharacterNameLock.lock();
	m_BotcharacterNameMap.insert(std::make_pair(upperName, pSession));
	m_BotcharacterNameLock.unlock();
}

void CGameServerDlg::RemoveMapBotList(int16 GetSocketID, std::string strUserName)
{
	if (GetSocketID > MAX_USER && GetSocketID < NPC_BAND)
	{
		m_MapBotListLock.lock();
		m_MapBotList.DeleteData(GetSocketID);
		m_MapBotListLock.unlock();
	}

	if (!strUserName.empty())
	{
		STRTOUPPER(strUserName);
		m_BotcharacterNameLock.lock();
		m_BotcharacterNameMap.erase(strUserName);
		m_BotcharacterNameLock.unlock();
	}
}


/**
* @brief	Looks up a user by name.
*
* @param	findName	The name to find.
* @param	type		The type of name (account, character).
*
* @return	null if it fails, else the user pointer.
*/
CUser* CGameServerDlg::GetUserPtr(string findName, NameType type)
{
	// As findName is a copy of the string passed in, we can change it
	// without worry of affecting anything.
	STRTOUPPER(findName);

	NameMap::iterator itr;
	if (type == NameType::TYPE_ACCOUNT)
	{
		m_accountNameLock.lock();
		itr = m_accountNameMap.find(findName);
		m_accountNameLock.unlock();
		return (itr != m_accountNameMap.end() ? itr->second : nullptr);
	}
	else if (type == NameType::TYPE_CHARACTER)
	{
		m_characterNameLock.lock();
		itr = m_characterNameMap.find(findName);
		m_characterNameLock.unlock();
		return (itr != m_characterNameMap.end() ? itr->second : nullptr);
	}

	return nullptr;
}

/**
* @brief	Adds the account name & session to a hashmap (on login)
*
* @param	pSession	The session.
*/
void CGameServerDlg::AddAccountName(CUser* pSession)
{
	std::string upperName = pSession->m_strAccountID;
	STRTOUPPER(upperName);

	Guard _l(m_accountNameLock);
	m_accountNameMap[upperName] = pSession;

	pSession->m_addedmap = true;
}

/**
* @brief	Adds the character name & session to a hashmap (when in-game)
*
* @param	pSession	The session.
*/
void CGameServerDlg::AddCharacterName(CUser* pSession)
{
	if (pSession == nullptr)
		return;


	string upperName = pSession->GetName();
	STRTOUPPER(upperName);
	m_characterNameLock.lock();
	m_characterNameMap[upperName] = pSession;
	m_characterNameLock.unlock();

	if (pSession->isGM())
		m_GMNameMap[upperName] = pSession;
}

/**
* @brief	Removes an existing character name/session from the hashmap,
* 			replaces the character's name and reinserts the session with
* 			the new name into the hashmap.
*
* @param	pSession		The session.
* @param	strNewUserID	Character's new name.
*/
void CGameServerDlg::ReplaceCharacterName(CUser* pSession, std::string& strNewUserID)
{
	if (pSession == nullptr)
		return;

	// Remove the old name from the map
	string upperName = pSession->GetName();
	STRTOUPPER(upperName);
	m_characterNameLock.lock();
	m_characterNameMap.erase(upperName);
	m_characterNameLock.unlock();

	if (pSession->isGM())
		m_GMNameMap.erase(upperName);

	// Update the character's name & re-add them to the map.
	pSession->m_strUserID = strNewUserID;
	AddCharacterName(pSession);
}

/**
* @brief	Removes the account name & character names from the hashmaps (on logout)
*
* @param	pSession	The session.
*/
void CGameServerDlg::RemoveSessionNames(CUser* pSession)
{
	string upperName = pSession->m_strAccountID;
	STRTOUPPER(upperName);

	{ // remove account name from map (limit scope)
		m_accountNameLock.lock();
		m_accountNameMap.erase(upperName);
		m_accountNameLock.unlock();
	}

	if (pSession->isInGame())
	{
		upperName = pSession->GetName();
		STRTOUPPER(upperName);

		m_characterNameLock.lock();

		if (pSession->isGM())
			m_GMNameMap.erase(upperName);

		m_characterNameMap.erase(upperName);
		m_characterNameLock.unlock();
	}
}

CUser* CGameServerDlg::GetUserPtr(uint16 sUserId) { return m_socketMgr[sUserId]; }
CBot* CGameServerDlg::GetBotPtr(uint16 sBotId)
{
	return m_MapBotList.GetData(sBotId);
}
CNpc* CGameServerDlg::GetNpcPtr(uint16 sNpcId, uint16 sZoneID)
{
	CNpcThread* zoneitrThread = m_arNpcThread.GetData(sZoneID);
	if (zoneitrThread == nullptr)
		return false;

	return zoneitrThread->m_arNpcArray.GetData(sNpcId);
}
_PARTY_GROUP* CGameServerDlg::GetPartyPtr(uint16 sPartyID) { return m_PartyArray.GetData(sPartyID); }
CKnights* CGameServerDlg::GetClanPtr(uint16 sClanID) { return m_KnightsArray.GetData(sClanID); }
_KNIGHTS_ALLIANCE* CGameServerDlg::GetAlliancePtr(uint16 sAllianceID) { return m_KnightsAllianceArray.GetData(sAllianceID); }

_ITEM_TABLE CGameServerDlg::GetItemPtr(uint32 nItemID)
{
	//if (preload.itemtable) return _ITEM_TABLE();
	auto* pItem = m_ItemtableArray.GetData(nItemID);
	return pItem == nullptr ? _ITEM_TABLE() : (*pItem);
}

_LEVEL_REWARDS CGameServerDlg::GetLevelRewards(uint8 bLevel)
{
	if (tar_levelreward) return _LEVEL_REWARDS();
	auto* pReward = m_LevelRewardArray.GetData(bLevel);
	return pReward == nullptr ? _LEVEL_REWARDS() : (*pReward);
}

_MAGIC_TABLE CGameServerDlg::GetMagicPtr(uint32 nSkillID, bool mutex)
{
	auto p = _MAGIC_TABLE();
	if (m_IsMagicTableInUpdateProcess)
		return p;

	auto* pSkill = m_MagictableArray.GetData(nSkillID, mutex);
	return pSkill == nullptr ? p : (*pSkill);
}

//_MAGIC_TYPE1 CGameServerDlg::GetMagic1Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE1();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype1Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}
//_MAGIC_TYPE2 CGameServerDlg::GetMagic2Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE2();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype2Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}
//_MAGIC_TYPE3 CGameServerDlg::GetMagic3Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE3();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype3Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}
//_MAGIC_TYPE4 CGameServerDlg::GetMagic4Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE4();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype4Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}
//_MAGIC_TYPE5 CGameServerDlg::GetMagic5Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE5();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype5Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}
//_MAGIC_TYPE6 CGameServerDlg::GetMagic6Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE6();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype6Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}
//_MAGIC_TYPE7 CGameServerDlg::GetMagic7Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE7();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype7Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}
//_MAGIC_TYPE8 CGameServerDlg::GetMagic8Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE8();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype8Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}
//_MAGIC_TYPE9 CGameServerDlg::GetMagic9Ptr(uint32 nSkillID, bool mutex) {
//	auto p = _MAGIC_TYPE9();
//	if (preload.skill) return p;
//	auto* pSkill = m_Magictype9Array.GetData(nSkillID, mutex);
//	return pSkill == nullptr ? p : (*pSkill);
//}

Unit* CGameServerDlg::GetUnitPtr(uint16 id, uint16 sZoneID /*= 0*/)
{
	if (id < NPC_BAND)
	{
		if (id < MAX_USER)
			return GetUserPtr(id);
		else
			return GetBotPtr(id);
	}

	if (sZoneID == 0)
		return nullptr;

	return GetNpcPtr(id, sZoneID);
}

#pragma region CGameServerDlg::KillNpc(uint16 sNid, uint8 byZone, Unit *pKiller)

/**
* @brief	Kills the NPC specified.
*
* @param	sNid	the NPC ID
*/
void CGameServerDlg::KillNpc(uint16 sNid, uint8 byZone, Unit* pKiller)
{
	CNpcThread* zoneitrThread = m_arNpcThread.GetData(byZone);
	if (zoneitrThread == nullptr) return;

	if (sNid < NPC_BAND)
	{
		zoneitrThread->m_arNpcArray.m_lock.lock();
		foreach_stlmap_nolock(itr, zoneitrThread->m_arNpcArray)
		{
			CNpc* pNpc = TO_NPC(itr->second);
			if (pNpc == nullptr || pNpc->isDead()) continue;

			if (pNpc->m_oSocketID != sNid)
				continue;

			pNpc->m_oSocketID = -1;
			pNpc->Dead(pKiller);
		}
		zoneitrThread->m_arNpcArray.m_lock.unlock();
	}
	else
	{
		CNpc* pNpc = (pKiller == nullptr ? g_pMain->GetNpcPtr(sNid, byZone) : g_pMain->GetNpcPtr(sNid, pKiller->GetZoneID()));
		if (pNpc)
			pNpc->Dead(pKiller);
	}
}

#pragma endregion

#pragma region CGameServerDlg::NpcUpdate(uint16 sSid, bool bIsMonster, uint8 byGroup, uint16 sPid)

void CGameServerDlg::NpcUpdate(uint16 sSid, bool bIsMonster, uint8 byGroup, uint16 sPid)
{
	CNpcTable* proto = nullptr;

	if (bIsMonster)
		proto = m_arMonTable.GetData(sSid);
	else
		proto = m_arNpcTable.GetData(sSid);

	if (proto == nullptr)
		return;

	if (byGroup > 0)
		proto->m_byGroupSpecial = byGroup;

	if (sPid > 0)
		proto->m_sPid = sPid;
}

#pragma endregion

#pragma region _PARTY_GROUP* CGameServerDlg::CreateParty(CUser* pLeader)

_PARTY_GROUP* CGameServerDlg::CreateParty(CUser* pLeader)
{
	_PARTY_GROUP* pParty = new _PARTY_GROUP;

	pLeader->m_bInParty = true;
	pLeader->m_sPartyIndex = m_sPartyIndex.increment();

	pParty->wIndex = pLeader->GetPartyID();
	pParty->uid[0] = pLeader->GetSocketID();
	if (!m_PartyArray.PutData(pParty->wIndex, pParty))
	{
		delete pParty;
		pLeader->m_bInParty = false;
		pLeader->m_sPartyIndex = -1;
		pParty = nullptr;
	}


	return pParty;
}

_PARTY_GROUP* CGameServerDlg::CreateParty(CBot* pLeader)
{
	_PARTY_GROUP* pParty = new _PARTY_GROUP;

	pLeader->m_bInParty = true;
	pLeader->m_bPartyLeader = true;
	pLeader->m_sPartyIndex = m_sPartyIndex.increment();

	pParty->wIndex = pLeader->GetPartyID();
	pParty->uid[0] = pLeader->GetID();
	if (!m_PartyArray.PutData(pParty->wIndex, pParty))
	{
		delete pParty;
		pLeader->m_bInParty = false;
		pLeader->m_bPartyLeader = false;
		pLeader->m_sPartyIndex = 0;
		pParty = nullptr;
	}

	return pParty;
}

#pragma endregion

#pragma region CGameServerDlg::Timer_CheckGameEvents(void * lpParam)

uint32 CGameServerDlg::Timer_CheckGameEvents(void* lpParam)
{
	while (true)
	{
		g_pMain->GameInfoNoticeTimer();
		g_pMain->ShutdownTimer();
		g_pMain->JstKOAutoOnlineCount(); // JstKO Yeni Auto Online Count Notice Eklendi 01.01.2025

		//Event Timer
		g_pMain->GameEventMainTimer();

		if (g_pMain->isWarOpen() && ((UNIXTIME - g_pMain->m_lastBlessTime) >= (5 * NATION_MONUMENT_REWARD_SECOND)))
			g_pMain->CheckNationMonumentRewards();

		sleep(1 * SECOND);
	}
	return 0;
}

#pragma endregion

void CGameServerDlg::GameEventMainTimer()
{
	CollectionRaceTimer();
	EventMainTimer();
	TempleSoccerEventTimer();
	TempleMonsterStoneTimer();
	DrakiTowerLimitReset();
	BDWMonumentAltarTimer();
	ClanTournamentTimer();
	DungeonDefenceTimer();
	DrakiTowerRoomCloseTimer();
	ChaosStoneRespawnTimer();
	NewWantedEventMainTimer();
	LotteryEventTimer();
	CindirellaTimer();
}

void CGameServerDlg::ResetLoyaltyMonthly()
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);

		if (pUser == nullptr)
			continue;

		pUser->m_iLoyaltyMonthly = 0;
		pUser->SendLoyaltyChange();
	}

	// Attempt to update the database in this thread directly, while the session map's locked.
	// This prevents new users from logging in before they've been reset (hence keeping last month's total).
	Packet pkt(WIZ_DB_SAVE, uint8(ProcDbServerType::ResetLoyalty));
	g_pMain->AddDatabaseRequest(pkt);
}

void CGameServerDlg::UpdateWeather()
{
	if (m_byKingWeatherEvent)
	{
		int16 sEventExpiry;
		if (g_localTime.tm_mday == m_byKingWeatherEvent_Day)
			sEventExpiry = g_localTime.tm_min + 60 * (g_localTime.tm_hour - m_byKingWeatherEvent_Hour) - m_byKingWeatherEvent_Minute;
		else
			sEventExpiry = g_localTime.tm_min + 60 * (g_localTime.tm_hour - m_byKingWeatherEvent_Hour + 24) - m_byKingWeatherEvent_Minute;

		// Weather events last for 5 minutes
		if (sEventExpiry > 5)
		{
			m_byKingWeatherEvent = 0;
			m_byKingWeatherEvent_Day = 0;
			m_byKingWeatherEvent_Hour = 0;
			m_byKingWeatherEvent_Minute = 0;
		}
	}
	else
	{
		m_byWeather = WEATHER_FINE;
		m_sWeatherAmount = 0;
		goto sendweather;

		int weather = 0, rnd = myrand(0, 100);
		if (rnd < 2)		weather = WEATHER_SNOW;
		else if (rnd < 7)	weather = WEATHER_RAIN;
		else				weather = WEATHER_FINE;

		m_sWeatherAmount = myrand(0, 100);
		if (weather == WEATHER_FINE)
		{
			if (m_sWeatherAmount > 70)
				m_sWeatherAmount /= 2;
			else
				m_sWeatherAmount = 0;
		}
		m_byWeather = weather;
	}

sendweather:

	// Real weather data for most users.
	Packet realWeather(WIZ_WEATHER, m_byWeather);
	realWeather << m_sWeatherAmount;

	// Fake, clear weather for users in certain zones (e.g. Desp & Hell Abysses, Arena)
	Packet fakeWeather(WIZ_WEATHER, uint8(WEATHER_FINE));
	fakeWeather << m_sWeatherAmount;

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (pUser->GetZoneID() == 32
			|| pUser->GetZoneID() == 33
			|| pUser->GetZoneID() == 48)
			pUser->Send(&fakeWeather);
		else
			pUser->Send(&realWeather);
	}
}

void CGameServerDlg::SetGameTime()
{
	CIni ini(CONF_GAME_SERVER);
	ini.SetInt("TIMER", "WEATHER", m_byWeather);
}

void CGameServerDlg::AddDatabaseRequest(Packet& pkt, CUser* pUser /*= nullptr*/)
{
	if (!m_afkqueue) return;
	Packet* newPacket = new Packet(pkt.GetOpcode(), pkt.size() + 2);
	*newPacket << int16(pUser == nullptr ? -1 : pUser->GetSocketID());
	if (pkt.size()) newPacket->append(pkt.contents(), pkt.size());
	m_afkqueue->AddRequest((int)dbreqtype::Database, newPacket);
}

void CGameServerDlg::AddLogRequest(Packet& pkt)
{
	if (!m_afkqueue) return;
	Packet* newPacket = new Packet();
	if (pkt.size()) newPacket->append(pkt.contents(), pkt.size());
	m_afkqueue->AddRequest((int)dbreqtype::Logger, newPacket);
}

void CGameServerDlg::HandleConsoleCommand(const char* msg)
{
	string message = msg;
	if (message.empty())
		return;

	if (ProcessServerCommand(message))
	{
		printf("Command accepted.\n");
		return;
	}

	printf("Invalid command. If you're trying to send a notice, please use /notice\n");
}

bool CGameServerDlg::LoadNoticeData()
{
	ifstream file("./Notice.txt");
	string line;
	int count = 0;

	// Clear out the notices first
	memset(&m_ppNotice, 0, sizeof(m_ppNotice));

	if (!file)
	{
		TRACE("Notice.txt could not be opened.\n");
		printf("Notice.txt could not be opened.\n");
		return false;
	}

	while (!file.eof())
	{
		if (count > 19)
		{
			TRACE("Too many lines in Notice.txt\n");
			printf("Too many lines in Notice.txt\n");
			break;
		}

		getline(file, line);
		if (line.length() > 128)
		{
			TRACE("Notice.txt contains line that exceeds the limit of 128 characters.\n");
			printf("Notice.txt contains line that exceeds the limit of 128 characters.\n");
			break;
		}

		strcpy(m_ppNotice[count++], line.c_str());
	}

	file.close();
	return true;
}

bool CGameServerDlg::LoadNoticeUpData()
{
	ifstream file("./Notice_up.txt");
	string line;
	int count = 0;

	// Clear out the notices first
	memset(&m_peNotice, 0, sizeof(m_peNotice));

	if (!file)
	{
		TRACE("Notice_up.txt could not be opened.\n");
		printf("Notice_up.txt could not be opened.\n");
		return false;
	}

	while (!file.eof())
	{
		if (count > 19)
		{
			TRACE("Too many lines in Notice_up.txt\n");
			printf("Too many lines in Notice_up.txt\n");
			break;
		}

		getline(file, line);
		if (line.length() > 128)
		{
			TRACE("Notice_up.txt contains line that exceeds the limit of 128 characters.\n");
			printf("Notice_up.txt contains line that exceeds the limit of 128 characters.\n");
			break;
		}

		strcpy(m_peNotice[count++], line.c_str());
	}

	file.close();
	return true;
}

#pragma region CGameServerDlg::FindNpcInZone(uint16 sPid, uint8 byZone)

/**
* @brief	Searches for the first NPC in the specified zone
* 			with the specified picture/model ID.
*
* @param	sPid	Picture/model ID of the NPC.
* @param	byZone	Zone to search in.
*
* @return	null if it fails, else the NPC instance we found.
*/
CNpc* CGameServerDlg::FindNpcInZone(uint16 sPid, uint8 byZone)
{
	CNpcThread* zoneitrThread = m_arNpcThread.GetData(byZone);
	if (zoneitrThread == nullptr)
		return nullptr;

	foreach_stlmap(itr, zoneitrThread->m_arNpcArray)
	{
		CNpc* pNpc = itr->second;
		if (pNpc == nullptr || pNpc->GetZoneID() != byZone
			// This isn't a typo, it's actually just a hack.
			// The picture/model ID of most spawns is the same as their prototype ID.
			// When there's more than one spawn prototype (i.e. different sSid), we keep using
			// the same picture/model ID. So we check this instead of the sSid...
			|| pNpc->GetPID() != sPid)
			continue;

		return pNpc;
	}

	return nullptr;
}

#pragma endregion

#pragma region CNpc*  CGameServerDlg::GetPetPtr(int16 m_sPetID, uint8 byZone)

/**
* @brief	Searches for the first PET in the specified zone
* 			with the specified picture/model ID.
*
* @param	sPid	Picture/model ID of the NPC.
* @param	byZone	Zone to search in.
*
* @return	null if it fails, else the NPC instance we found.
*/
CNpc* CGameServerDlg::GetPetPtr(int16 m_sPetID, uint8 byZone)
{
	CNpcThread* zoneitrThread = m_arNpcThread.GetData(byZone);
	if (zoneitrThread == nullptr)
		return nullptr;

	foreach_stlmap(itr, zoneitrThread->m_arNpcArray)
	{
		CNpc* pNpc = itr->second;
		if (pNpc == nullptr
			|| pNpc->GetZoneID() != byZone
			// This isn't a typo, it's actually just a hack.
			// The picture/model ID of most spawns is the same as their prototype ID.
			// When there's more than one spawn prototype (i.e. different sSid), we keep using
			// the same picture/model ID. So we check this instead of the sSid...
			|| pNpc->GetPetID() != m_sPetID)
			continue;

		return pNpc;
	}

	return nullptr;
}

#pragma endregion
void CGameServerDlg::TheEventUserKickOut(uint8 ZoneID)
{
	uint8 nZoneID = 0;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (ZoneID == ZONE_UNDER_CASTLE)
		{
			if (pUser->GetLevel() >= 70
				&& pUser->GetLevel() <= 83
				&& pUser->GetNation() == (uint8)Nation::KARUS)
				nZoneID = ZONE_KARUS;
			else if (pUser->GetLevel() >= 70
				&& pUser->GetLevel() <= 83
				&& pUser->GetNation() == (uint8)Nation::ELMORAD)
				nZoneID = ZONE_ELMORAD;

			pUser->ZoneChange(nZoneID, 0.0f, 0.0f);
		}
	}
}

void CGameServerDlg::Announcement(uint16 type, int nation, int chat_type, CUser* pExceptUser, CNpc* pExpectNpc)
{
	string chatstr;
	uint8 ZoneID = 0;
	std::string sEventName;

	switch (type)
	{
	case BATTLEZONE_OPEN:
		if (m_byBattleZone + ZONE_BATTLE_BASE == ZONE_BATTLE3)
			GetServerResource(IDS_MID_LEVEL_BATTLEZONE_OPEN, &chatstr);
		else
			GetServerResource(IDP_BATTLEZONE_OPEN, &chatstr);
		break;
	case DECLARE_WINNER:
		if (m_bVictory == (uint8)Nation::KARUS)
			GetServerResource(IDP_KARUS_VICTORY, &chatstr, m_sElmoradDead, m_sKarusDead);
		else if (m_bVictory == (uint8)Nation::ELMORAD)
			GetServerResource(IDP_ELMORAD_VICTORY, &chatstr, m_sKarusDead, m_sElmoradDead);
		else
			return;
		break;
	case DECLARE_LOSER:
		if (m_bVictory == (uint8)Nation::KARUS)
			GetServerResource(IDS_ELMORAD_LOSER, &chatstr, m_sKarusDead, m_sElmoradDead);
		else if (m_bVictory == (uint8)Nation::ELMORAD)
			GetServerResource(IDS_KARUS_LOSER, &chatstr, m_sElmoradDead, m_sKarusDead);
		else
			return;
		break;
	case DECLARE_BAN:
		if (m_bVictory == (uint8)Nation::KARUS || m_bVictory == (uint8)Nation::ELMORAD)
			GetServerResource(IDS_BANISH_LOSER, &chatstr);
		else
			GetServerResource(IDS_BANISH_USER, &chatstr);
		break;
	case SNOW_BAN:
		if (m_bVictory == (uint8)Nation::KARUS || m_bVictory == (uint8)Nation::ELMORAD)
			GetServerResource(IDS_BANISH_LOSER, &chatstr);
		else
			GetServerResource(IDS_SNOWBATTLE_BANISH_USER, &chatstr);
		break;
	case DECLARE_BATTLE_ZONE_STATUS:
		if (m_byBattleZone + ZONE_BATTLE_BASE == ZONE_BATTLE4)
		{
			GetServerResource(IDS_BATTLE_MONUMENT_STATUS, &chatstr, m_sKarusMonumentPoint, m_sElmoMonumentPoint, m_sKarusDead, m_sElmoradDead);
			SendNotice<ChatType::PUBLIC_CHAT>(chatstr.c_str(), ZONE_BATTLE4, (uint8)Nation::ALL, true);
		}
		else if (m_byBattleZone + ZONE_BATTLE_BASE == ZONE_BATTLE5)
		{
			GetServerResource(IDS_BATTLE_MONUMENT_STATUS, &chatstr, m_sKarusMonumentPoint, m_sElmoMonumentPoint, m_sKarusDead, m_sElmoradDead);
			SendNotice<ChatType::PUBLIC_CHAT>(chatstr.c_str(), ZONE_BATTLE5, (uint8)Nation::ALL, true);
		}
		else
			GetServerResource(IDS_BATTLEZONE_STATUS, &chatstr, m_sKarusDead, m_sElmoradDead);
		break;
	case DECLARE_BATTLE_MONUMENT_STATUS:
		if (pExceptUser)
		{
			GetServerResource(IDS_BATTLE_MONUMENT_WON_MESSAGE, &chatstr, GetBattleAndNationMonumentName(chat_type).c_str());
			g_pMain->SendNotice<ChatType::PUBLIC_CHAT>(chatstr.c_str(), pExceptUser->GetZoneID());
			GetServerResource(IDS_BATTLE_MONUMENT_LOST_MESSAGE, &chatstr, GetBattleAndNationMonumentName(chat_type).c_str());
			g_pMain->SendNotice<ChatType::PUBLIC_CHAT>(chatstr.c_str(), pExceptUser->GetZoneID());
			return;
		}
		break;
	case DECLARE_NATION_MONUMENT_STATUS:
		if (pExceptUser)
		{
			uint16 nTrapNumber = pExceptUser->GetZoneID() == ZONE_KARUS ? chat_type - LUFERSON_MONUMENT_SID : chat_type - ELMORAD_MONUMENT_SID;

			if ((pExceptUser->GetNation() == (uint8)Nation::KARUS && pExceptUser->GetZoneID() == (uint8)Nation::ELMORAD) || (pExceptUser->GetNation() == (uint8)Nation::ELMORAD && pExceptUser->GetZoneID() == (uint8)Nation::KARUS))
			{
				GetServerResource(IDS_INFILTRATION_CONQUER, &chatstr, GetBattleAndNationMonumentName(nTrapNumber, pExceptUser->GetZoneID()).c_str());
				g_pMain->SendAnnouncement(chatstr.c_str());
				return;
			}
			else if ((pExceptUser->GetNation() == (uint8)Nation::KARUS && pExceptUser->GetZoneID() == (uint8)Nation::KARUS) || (pExceptUser->GetNation() == (uint8)Nation::ELMORAD && pExceptUser->GetZoneID() == (uint8)Nation::ELMORAD))
			{
				GetServerResource(IDS_INFILTRATION_RECAPTURE, &chatstr, GetBattleAndNationMonumentName(nTrapNumber, pExceptUser->GetZoneID()).c_str());
				g_pMain->SendAnnouncement(chatstr.c_str());
				return;
			}
			return;
		}
		break;
	case DECLARE_NATION_REWARD_STATUS:
		if (pExpectNpc)
		{
			uint16 nTrapNumber = pExpectNpc->GetZoneID() == ZONE_KARUS ? chat_type - LUFERSON_MONUMENT_SID : chat_type - ELMORAD_MONUMENT_SID;

			GetServerResource(pExpectNpc->GetNation() == (uint8)Nation::KARUS ? IDS_INFILTRATION_REWARD_KARUS : IDS_INFILTRATION_REWARD_ELMORAD, &chatstr, GetBattleAndNationMonumentName(nTrapNumber, pExpectNpc->GetZoneID()).c_str());
			g_pMain->SendAnnouncement(chatstr.c_str(), (uint8)Nation::ALL);
			return;
		}
		break;
	case SNOW_BATTLEZONE_OPEN:
		GetServerResource(IDS_SNOWBATTLE_OPEN, &chatstr);
		break;
	case UNDER_ATTACK_NOTIFY:
		if (m_bVictory == (uint8)Nation::KARUS)
			GetServerResource(IDS_UNDER_ATTACK_ELMORAD, &chatstr, m_sKarusDead, m_sElmoradDead);
		else if (m_bVictory == (uint8)Nation::ELMORAD)
			GetServerResource(IDS_UNDER_ATTACK_KARUS, &chatstr, m_sElmoradDead, m_sKarusDead);
		else
			return;
		break;
	case BATTLEZONE_CLOSE:
		GetServerResource(IDS_BATTLE_CLOSE, &chatstr);
		break;
	case SNOW_BATTLEZONE_CLOSE:
		GetServerResource(IDS_SNOWBATTLE_CLOSE, &chatstr);
		break;
	case KARUS_CAPTAIN_NOTIFY:
		GetServerResource(IDS_KARUS_CAPTAIN, &chatstr, m_strKarusCaptain.c_str());
		break;
	case ELMORAD_CAPTAIN_NOTIFY:
		GetServerResource(IDS_ELMO_CAPTAIN, &chatstr, m_strElmoradCaptain.c_str());
		break;
	case KARUS_CAPTAIN_DEPRIVE_NOTIFY:
		if (pExceptUser)
		{
			if (!pExceptUser->isInClan())
				return;

			CKnights* pKnights = g_pMain->GetClanPtr(pExceptUser->GetClanID());
			GetServerResource(IDS_KARUS_CAPTAIN_DEPRIVE, &chatstr, pKnights == nullptr ? "***" : pKnights->GetName().c_str(), pExceptUser->GetName().c_str());
		}
		break;
	case ELMORAD_CAPTAIN_DEPRIVE_NOTIFY:
		if (pExceptUser)
		{
			if (!pExceptUser->isInClan())
				return;

			CKnights* pKnights = g_pMain->GetClanPtr(pExceptUser->GetClanID());
			GetServerResource(IDS_ELMO_CAPTAIN_DEPRIVE, &chatstr, pKnights == nullptr ? "***" : pKnights->GetName().c_str(), pExceptUser->GetName().c_str());
		}
		break;
	case IDS_MONSTER_CHALLENGE_ANNOUNCEMENT:
		GetServerResource(IDS_MONSTER_CHALLENGE_ANNOUNCEMENT, &chatstr, pForgettenTemple.NoticeStartHour, pForgettenTemple.MinLevel, pForgettenTemple.MaxLevel);
		break;
	case IDS_MONSTER_CHALLENGE_OPEN:
		GetServerResource(IDS_MONSTER_CHALLENGE_OPEN, &chatstr);
		break;
	case IDS_MONSTER_CHALLENGE_START:
		ZoneID = ZONE_FORGOTTEN_TEMPLE;
		GetServerResource(IDS_MONSTER_CHALLENGE_START, &chatstr);
		break;
	case IDS_MONSTER_CHALLENGE_VICTORY:
		ZoneID = ZONE_FORGOTTEN_TEMPLE;
		GetServerResource(IDS_MONSTER_CHALLENGE_VICTORY, &chatstr);
		break;
	case IDS_MONSTER_CHALLENGE_CLOSE:
		GetServerResource(IDS_MONSTER_CHALLENGE_CLOSE, &chatstr);
		break;
		/*case IDS_UNDER_THE_CASTLE_ANNOUNCEMENT:
			GetServerResource(IDS_UNDER_THE_CASTLE_ANNOUNCEMENT, &chatstr, m_nUnderTheCastleLevelMin, m_nUnderTheCastleLevelMax, m_nUnderTheCastleStartHour);
			break;*/
	case IDS_UNDER_THE_CASTLE_OPEN:
		GetServerResource(IDS_UNDER_THE_CASTLE_OPEN, &chatstr);
		break;
		/*case IDS_UNDER_THE_CASTLE_START:
			ZoneID = ZONE_UNDER_CASTLE;
			GetServerResource(IDS_UNDER_THE_CASTLE_START, &chatstr);
			break;*/
	case IDS_UNDER_THE_CASTLE_VICTORY:
		ZoneID = ZONE_UNDER_CASTLE;
		GetServerResource(IDS_UNDER_THE_CASTLE_VICTORY, &chatstr);
		break;
	case IDS_UNDER_THE_CASTLE_CLOSE:
		GetServerResource(IDS_UNDER_THE_CASTLE_CLOSE, &chatstr);
		break;
	case IDS_SIEGE_WAR_TIME_NOTICE:
		GetServerResource(IDS_SIEGE_WAR_TIME_NOTICE, &chatstr, m_byBattleSiegeWarNoticeTime);
		break;
	case IDS_SIEGE_WAR_START:
		GetServerResource(IDS_SIEGE_WAR_START, &chatstr);
		break;
	case IDS_SIEGE_WAR_END:
		GetServerResource(IDS_SIEGE_WAR_END, &chatstr);
		break;
	case IDS_NPC_GUIDON_DESTORY:
		GetServerResource(IDS_NPC_GUIDON_DESTORY, &chatstr, m_byBattleSiegeWarWinKnightsNotice.c_str());
		break;
	case IDS_SIEGE_WAR_VICTORY:
		GetServerResource(IDS_SIEGE_WAR_VICTORY, &chatstr, m_byBattleSiegeWarWinKnightsNotice.c_str());
		break;
	case IDS_PRE_BATTLE_ANNOUNCEMENT:
		GetServerResource(IDS_PRE_BATTLE_ANNOUNCEMENT, &chatstr, m_byBattleNoticeTime);
		break;
	case IDS_REMNANT_SUMMON_INFO:
		GetServerResource(IDS_REMNANT_SUMMON_INFO, &chatstr);
		break;
	}

	Packet result;
	string finalstr;
	GetServerResource(IDP_ANNOUNCEMENT, &finalstr, chatstr.c_str());
	ChatPacket::Construct(&result, (uint8)chat_type, &finalstr);
	Send_All(&result, nullptr, nation, ZoneID);
}

/**
* @brief	Loads the specified user's NP ranks
* 			from the rankings tables.
*
* @param	pUser	The user.
*/
void CGameServerDlg::GetUserRank(CUser* pUser)
{
	if (pUser == nullptr)
		return;
	Guard lock(m_userRankingsLock);
	// Acquire the lock for thread safety
	// Get character's name & convert it to upper case for case insensitivity
	string strUserID = pUser->GetName();
	STRTOUPPER(strUserID);

	// Grab the personal rank from the map, if applicable.
	if (pUser->GetNation() == (uint8)Nation::KARUS)
	{
		UserNameRankMap::iterator itr = m_UserKarusPersonalRankMap.find(strUserID);
		pUser->m_bPersonalRank = itr != m_UserKarusPersonalRankMap.end() ? uint8(itr->second->nRank) : -1;

		// Grab the knights rank from the map, if applicable.
		itr = m_UserKarusKnightsRankMap.find(strUserID);
		pUser->m_bKnightsRank = itr != m_UserKarusKnightsRankMap.end() ? uint8(itr->second->nRank) : -1;
	}
	else if (pUser->GetNation() == (uint8)Nation::ELMORAD)
	{
		UserNameRankMap::iterator itr = m_UserElmoPersonalRankMap.find(strUserID);
		pUser->m_bPersonalRank = itr != m_UserElmoPersonalRankMap.end() ? uint8(itr->second->nRank) : -1;

		// Grab the knights rank from the map, if applicable.
		itr = m_UserElmoKnightsRankMap.find(strUserID);
		pUser->m_bKnightsRank = itr != m_UserElmoKnightsRankMap.end() ? uint8(itr->second->nRank) : -1;
	}
}

/**
* @brief	Calculates the clan grade from the specified
* 			loyalty points (NP).
*
* @param	nPoints	Loyalty points (NP).
* 					The points will be converted to clan points
* 					by this method.
*
* @return	The clan grade.
*/
int CGameServerDlg::GetKnightsGrade(uint32 nPoints)
{
	uint32 nClanPoints = nPoints;

	if (nClanPoints >= m_Grade1)
		return 1;
	else if (nClanPoints >= m_Grade2)
		return 2;
	else if (nClanPoints >= m_Grade3)
		return 3;
	else if (nClanPoints >= m_Grade4)
		return 4;

	return 5;
}

/**
* @brief	Disconnects all players in the server.
*
* @return	The number of users who were in-game.
*/
int CGameServerDlg::KickOutAllUsers()
{
	int count = 0;

	Packet result;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (pUser->isInGame())
			count++;

		result.clear();
		result.Initialize(WIZ_DB_SAVE_USER);
		result << uint8(ProcDbType::KickOutAllUser);
		AddDatabaseRequest(result, pUser);
	}

	std::vector<uint32> HandleBotAllDisconnected;

	g_pMain->m_MapBotList.m_lock.lock();
	auto m_sMapBotListArray = g_pMain->m_MapBotList.m_UserTypeMap;
	g_pMain->m_MapBotList.m_lock.unlock();

	foreach(itr, m_sMapBotListArray)
	{
		CBot* pBot = g_pMain->GetBotPtr(itr->first);
		if (pBot == nullptr)
			continue;

		HandleBotAllDisconnected.push_back(pBot->GetID());
	}

	if (HandleBotAllDisconnected.size() > 0)
	{
		foreach(itr, HandleBotAllDisconnected)
		{
			CBot* pBot = g_pMain->GetBotPtr(*itr);
			if (pBot == nullptr)
				continue;

			if (pBot->isInGame())
				count++;

			pBot->UserInOut(INOUT_OUT);
			g_pMain->RemoveMapBotList(pBot->GetID(), pBot->GetName());
		}
	}

	return count;
}

/**
* @brief	Kick out all users from the specified zone
* 			to their home zone.
*
* @param	zone	The zone to kick users out from.
*/
void CGameServerDlg::KickOutZoneUsers(uint8 ZoneID, uint8 TargetZoneID, uint8 bNation)
{
	// TODO: Make this localised to zones.
	C3DMap* pKarusMap = GetZoneByID((uint8)Nation::KARUS);
	C3DMap* pElMoradMap = GetZoneByID((uint8)Nation::ELMORAD);

	if (pKarusMap == nullptr
		|| pElMoradMap == nullptr)
		return;

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr
			|| !pUser->isInGame()
			|| pUser->GetZoneID() != ZoneID)
			continue;

		if (TargetZoneID > 0 && bNation == pUser->GetNation())
		{
			pUser->ZoneChange(TargetZoneID, 0.0f, 0.0f);
			continue;
		}
		else if (TargetZoneID > 0 && bNation == (uint8)Nation::ALL)
		{
			pUser->ZoneChange(TargetZoneID, 0.0f, 0.0f);
			continue;
		}

#if 0
		C3DMap* pMap = (pUser->GetNation() == (uint8)Nation::KARUS ? pKarusMap : pElMoradMap);
		if (pMap != nullptr && TargetZoneID == 0)
			pUser->ZoneChange(pMap->m_nZoneNumber, pMap->m_fInitX, pMap->m_fInitZ);
#else
		if (TargetZoneID == 0)
			pUser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f);
#endif
	}
}

void CGameServerDlg::SendItemUnderTheCastleRoomUsers(uint8 ZoneID, uint16 Room, float GetX, float GetZ)
{
	if (Room != 1
		&& Room != 2
		&& Room != 3
		&& Room != 4
		&& Room != 5)
		return;

	if (Room == 1 || Room == 2)
	{
		if (Room == 1)
		{
			for (uint16 i = 0; i < MAX_USER; i++)
			{
				CUser* pUtcPlayer = g_pMain->GetUserPtr(i);
				if (pUtcPlayer == nullptr
					|| !pUtcPlayer->isInGame()
					|| pUtcPlayer->GetZoneID() != ZoneID)
					continue;

				if (!pUtcPlayer->isInRangeSlow(121.0f, 297.0f, 80)
					&& !pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
					continue;

				if (pUtcPlayer->isInRangeSlow(121.0f, 297.0f, 80)
					&& pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
					pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME, 2);
				else if (pUtcPlayer->isInRangeSlow(121.0f, 297.0f, 80)
					|| pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
					pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME);
			}
		}
		else if (Room == 2)
		{
			for (uint16 i = 0; i < MAX_USER; i++)
			{
				CUser* pUtcPlayer = g_pMain->GetUserPtr(i);
				if (pUtcPlayer == nullptr
					|| !pUtcPlayer->isInGame()
					|| pUtcPlayer->GetZoneID() != ZoneID)
					continue;

				if (!pUtcPlayer->isInRangeSlow(121.0f, 297.0f, 80)
					&& !pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
					continue;

				if (pUtcPlayer->isInRangeSlow(121.0f, 297.0f, 80)
					&& pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
					pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME, 2);
				else if (pUtcPlayer->isInRangeSlow(121.0f, 297.0f, 80)
					|| pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
					pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME);

				pUtcPlayer->GiveItem("Utc Event", DENTED_IRONMASS);
			}
		}
	}
	else if (Room == 3)
	{
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUtcPlayer = g_pMain->GetUserPtr(i);
			if (pUtcPlayer == nullptr
				|| !pUtcPlayer->isInGame()
				|| pUtcPlayer->GetZoneID() != ZoneID)
				continue;

			if (!pUtcPlayer->isInRangeSlow(520.0f, 494.0f, 115)
				&& !pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				continue;

			if (pUtcPlayer->isInRangeSlow(520.0f, 494.0f, 115)
				&& pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME, 2);
			else if (pUtcPlayer->isInRangeSlow(520.0f, 494.0f, 115)
				|| pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME);

			pUtcPlayer->GiveItem("Utc Event", PETRIFIED_WEAPON_SHRAPNEL);
		}
	}
	else if (Room == 4)
	{
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUtcPlayer = g_pMain->GetUserPtr(i);
			if (pUtcPlayer == nullptr
				|| !pUtcPlayer->isInGame()
				|| pUtcPlayer->GetZoneID() != ZoneID)
				continue;

			if (!pUtcPlayer->isInRangeSlow(642.0f, 351.0f, 100)
				&& !pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				continue;

			if (pUtcPlayer->isInRangeSlow(642.0f, 351.0f, 100)
				&& pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME, 2);
			else if (pUtcPlayer->isInRangeSlow(642.0f, 351.0f, 100)
				|| pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME);

			pUtcPlayer->GiveItem("Utc Event", IRON_POWDER_OF_CHAIN);
		}
	}
	else if (Room == 5)
	{
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUtcPlayer = g_pMain->GetUserPtr(i);

			if (pUtcPlayer == nullptr
				|| !pUtcPlayer->isInGame()
				|| pUtcPlayer->GetZoneID() != ZoneID)
				continue;

			if (!pUtcPlayer->isInRangeSlow(803.0f, 839.0f, 110)
				&& !pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				continue;

			if (pUtcPlayer->isInRangeSlow(803.0f, 839.0f, 110)
				&& pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME, 2);
			else if (pUtcPlayer->isInRangeSlow(803.0f, 839.0f, 110)
				|| pUtcPlayer->isInRangeSlow(GetX, GetZ, 15))
				pUtcPlayer->GiveItem("Utc Event", TROPHY_OF_FLAME);

			pUtcPlayer->GiveItem("Utc Event", PLWITOONS_TEAR);
			pUtcPlayer->GiveItem("Utc Event", HORN_OF_PLUWITOON);
		}
	}
}

void CGameServerDlg::SendItemZoneUsers(uint8 ZoneID, uint32 nItemID, uint16 sCount /*= 1*/, uint32 Time/*= 0*/)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr
			|| !pUser->isInGame()
			|| pUser->GetZoneID() != ZoneID)
			continue;

		pUser->GiveItem("Send Zone Item Command", nItemID, sCount, true, Time);
	}
}

void CGameServerDlg::SendItemEventRoom(uint16 nEventRoom, uint32 nItemID, uint16 sCount /*= 1*/)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr
			|| !pUser->isInGame()
			|| pUser->GetEventRoom() != nEventRoom)
			continue;

		pUser->GiveItem("Send Event Room Item Command", nItemID, sCount);
	}
}

void CGameServerDlg::GetCaptainUserPtr()
{
	foreach_stlmap(itr, m_KnightsArray)
	{
		CKnights* pKnights = itr->second;
		if (pKnights == nullptr)
			continue;

		if (pKnights->m_byRanking != 1)
			continue;
	}
}

/**
* @brief	Sends the flying santa/angel packet to all users in the server.
*/
void CGameServerDlg::SendFlyingSantaOrAngel()
{
	Packet result(WIZ_SANTA, uint8(m_bSantaOrAngel));
	Send_All(&result);
}

CGameServerDlg::~CGameServerDlg()
{
	printf("The players are about to be kicked out of the game.");
	g_pMain->m_socketMgr.GetLock().lock();
	SessionMap sessMap = g_pMain->m_socketMgr.GetActiveSessionMap();
	g_pMain->m_socketMgr.GetLock().unlock();
	foreach(itr, sessMap)
	{
		CUser* pUser = TO_USER(itr->second);
		if (pUser == nullptr || !pUser->isInGame()) continue;
		pUser->OnDisconnect();
	}
	Sleep(30 * SECOND);

	g_bNpcExit = true;

	printf("Waiting for timer threads to exit.");
	foreach(itr, g_timerThreads)
	{
		if ((*itr) == nullptr) continue;
		if ((*itr)->isStarted()) (*itr)->waitForExit();
		delete (*itr);
	}
	printf("done.\n");
	Sleep(1 * SECOND);

	printf("Waiting for NPC threads to exit.");
	m_arNpcThread.m_lock.lock();
	foreach_stlmap_nolock(itr, m_arNpcThread)
	{
		CNpcThread* pThread = itr->second;
		if (pThread) pThread->Shutdown();
	}
	m_arNpcThread.m_lock.unlock();
	printf("done.\n");
	Sleep(1 * SECOND);

	printf("Shutting down Lua engine.");
	m_luaEngine.Shutdown();
	printf("done.\n");
	Sleep(1 * SECOND);

	printf("Shutting down adiniferihakoydum system.");
	for (int i = 0; i < (int)dbreqtype::COUNT; i++)
		if (m_afkqueue)
			m_afkqueue->Shutdown(i);
	printf("done.\n");
	Sleep(1 * SECOND);

	printf("Shutting down socket system.");
	m_socketMgr.Shutdown();
	printf("done.\n");
	Sleep(1 * SECOND);

	printf("Shutting down packet poll system.");
	SocketPoll::Shutdown();
	printf("OK!\n");

	CUser::CleanupChatCommands();
	CGameServerDlg::CleanupServerCommands();
	CleanupUserRankings();
	Sleep(3 * SECOND);
}

void CGameServerDlg::ShowNpcEffect(uint16 sNpcID, uint32 nEffectID, uint8 ZoneID, uint16 nEventRoom /* = 0 */)
{
	Packet result(WIZ_OBJECT_EVENT, uint8(OBJECT_NPC));
	result << uint8(3) << sNpcID << nEffectID;
	g_pMain->Send_Zone(&result, ZoneID, nullptr, 0, nEventRoom, 0.0f);
}

#pragma endregion

void CGameServerDlg::ReloadKnightAndUserRanks(bool command)
{
	Packet pkt(WIZ_DB_SAVE, uint8(ProcDbServerType::ReloadSymbolAndKnights));
	AddDatabaseRequest(pkt);

	if (command)
	{
		Packet result(WIZ_DB_SAVE, uint8(ProcDbServerType::UpdateKnights));
		g_pMain->AddDatabaseRequest(result);
	}
}

void CGameServerDlg::SetPlayerRankingRewards(uint16 ZoneID)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr)
			continue;

		if (!pUser->isInGame()
			|| pUser->isGM()
			|| pUser->GetZoneID() != ZoneID)
			continue;

		if (pUser->GetPlayerRank(RANK_TYPE_PK_ZONE) > 0 && pUser->GetPlayerRank(RANK_TYPE_PK_ZONE) <= 10)
		{
			if (m_nPlayerRankingLoyaltyReward > 0)
				pUser->SendLoyaltyChange("Player Ranking Reward", m_nPlayerRankingLoyaltyReward, false, true, false);

			if (m_nPlayerRankingKnightCashReward > 0)
				pUser->m_nKnightCash += m_nPlayerRankingKnightCashReward;

			auto* rankreward = g_pMain->m_RankRewardArray.GetData(pUser->GetPlayerRank(RANK_TYPE_PK_ZONE));
			if (!rankreward)
				continue;

			_ITEM_TABLE itemdata = g_pMain->GetItemPtr(rankreward->nItemID);
			if (itemdata.isnull())
				continue;

			std::string RewardSenderID = "Rank-Reward";
			std::string RewardSubject = "Rank-Reward";
			if (!pUser->GiveItem(RewardSubject, rankreward->nItemID, rankreward->sCount))
			{
				_ITEM_DATA pItem{};
				pItem.nNum = rankreward->nItemID;
				pItem.nSerialNum = g_pMain->GenerateItemSerial();
				pItem.sCount = rankreward->sCount;
				pItem.sDuration = itemdata.m_sDuration;
				g_DBAgent.SendLetter(RewardSenderID, pUser->GetName(), RewardSubject, RewardSubject, 2, &pItem, 0);
			}
		}
	}
}

void CGameServerDlg::HandleSiegeDatabaseRequest(CUser* pUser, Packet& pkt)
{
	uint8 bOpcode;
	pkt >> bOpcode;

	switch (bOpcode)
	{
	case 0:
	{
		int16 m_sCastleIndex, m_sMasterKnights, m_bySiegeType, m_byWarDay, m_byWarTime, m_byWarMinute;
		pkt >> m_sCastleIndex >> m_sMasterKnights >> m_bySiegeType >> m_byWarDay >> m_byWarTime >> m_byWarMinute;
		g_DBAgent.UpdateSiege(m_sCastleIndex, m_sMasterKnights, m_bySiegeType, m_byWarDay, m_byWarTime, m_byWarMinute);
	}
	break;
	case 1:
	{
		uint8 Zone;
		int16 ZoneTarrif;
		pkt >> Zone >> ZoneTarrif;
		g_DBAgent.UpdateSiegeTax(Zone, ZoneTarrif);
	}
	break;
	}
}

void CGameServerDlg::UpdateSiege(int16 m_sCastleIndex, int16 m_sMasterKnights, int16 m_bySiegeType, int16 m_byWarDay, int16 m_byWarTime, int16 m_byWarMinute)
{
	Packet result(WIZ_SIEGE, uint8(0));
	result << m_sCastleIndex << m_sMasterKnights << m_bySiegeType << m_byWarDay << m_byWarTime << m_byWarMinute;
	g_pMain->AddDatabaseRequest(result);
}

void CGameServerDlg::UpdateSiegeTax(uint8 Zone, int16 ZoneTarrif)
{
	Packet result(WIZ_SIEGE, uint8(1));
	result << Zone << ZoneTarrif;
	g_pMain->AddDatabaseRequest(result);
}

void CGameServerDlg::ShutdownTimer()
{
	if (!m_Shutdownstart || !m_Shutdownfinishtime) return;

	if (!m_ShutdownKickStart)
	{
		printf("Server shutdown in %d seconds\n", uint32(m_Shutdownfinishtime - UNIXTIME));
		if (UNIXTIME >= m_Shutdownfinishtime) { m_ShutdownKickStart = true; printf("Server shutdown, %d users kicked out.\n", KickOutAllUsers()); }
		return;
	}
	if (!m_ShutdownKickStart || m_Shutdownfinished) return;

	if (m_afkqueue)
	{
		for (int i = 0; i < (uint8)dbreqtype::COUNT; i++)
		{
			if (m_afkqueue->_queue[i].size())
			{
				std::string lname = "-";
				if (i == 0) lname = "Database";
				else if (i == 1) lname = "Logger";
				printf("%s remaining number of queries=%zd\n", lname.c_str(), m_afkqueue->_queue[i].size());
				return;
			}
		}
	}
	m_Shutdownfinished = true;

	int counter = 0;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser && pUser->isInGame())
		{
			pUser->UserDataSaveToAgent();
			pUser->Disconnect();
			counter++;
		}
	}

	if (counter) Sleep(10 * SECOND);

	printf("Please wait, the server is shutting down. Remaining player count=%d\n", counter);
	Packet result(WIZ_DB_SAVE, uint8(ProcDbServerType::UpdateKnights));
	g_pMain->AddDatabaseRequest(result);

	Packet result2(WIZ_DB_SAVE, uint8(ProcDbServerType::ReloadSymbolAndKnights));
	g_pMain->AddDatabaseRequest(result2);

	Sleep(5 * SECOND);
	for (int i = 0; i < (int)dbreqtype::COUNT; i++) if (m_afkqueue)m_afkqueue->Shutdown(i);
	printf("Shutdown Process is Finished! You can turn off the game server.\n");
}

void CGameServerDlg::SendYesilNotice(CUser* pUser, std::string sHelpMessage) // JstKO Offline Merchant için Eklendi 29.12.2024
{
	if (pUser == nullptr || sHelpMessage == "")
		return;

	Packet result;
	std::string buffer = string_format("%s", sHelpMessage.c_str());
	ChatPacket::Construct(&result, (uint8)ChatType::KNIGHTS_CHAT, &buffer);
	pUser->Send(&result);
}

void CGameServerDlg::SendRgbNotice(std::string Message, uint8 R, uint8 G, uint8 B) // JstKO Logos Rgb Notice Eklendi 29.12.2024
{
	// JstKO Logos Notice Eklendi. 29.12.2024
	Packet Logos(WIZ_LOGOSSHOUT);
	std::string DeaFSezo = string_format(" : %s", Message.c_str());
	Logos.SByte();
	Logos << uint8(2) << uint8(1) << R << G << B << uint8(0) << DeaFSezo;
	g_pMain->Send_All(&Logos);
	// JstKO Logos Notice The End. 29.12.2024
}

// JstKO WelcomeLogos Notice Eklendi. 01.01.2025
#pragma region CUser::WelcomeLogos(std::string LogosName, std::string LogosMessage, uint8 R, uint8 G, uint8 B)
void CUser::JstKOWelcomeLogos(std::string LogosName, std::string LogosMessage, uint8 R, uint8 G, uint8 B)
{
	// JstKO WelcomeLogos Notice Eklendi. 01.01.2025
	Packet result(WIZ_LOGOSSHOUT);
	std::string JstKO = string_format(" : %s", LogosMessage.c_str());
	result.SByte();
	result << uint8(2) << uint8(1) << R << G << B << uint8(0) << JstKO;
	Send(&result);
	// JstKO WelcomeLogos Notice The End. 01.01.2025
}
#pragma endregion

#pragma region CGameServerDlg::SendNewNoticeZone(CUser* pUser, std::string sHelpMessage)
void CGameServerDlg::SendYeniNoticeZone(CUser* pUser, std::string sHelpMessage)
{
	if (pUser == nullptr || sHelpMessage == "")
		return;

	Guard lock(g_pMain->m_characterNameLock);
	NameMap sessMap = g_pMain->m_characterNameMap;
	foreach(itr, sessMap)
	{
		CUser* aUser = TO_USER(itr->second);
		if (aUser == nullptr)
			continue;

		if (aUser->isInGame())
		{
			if (pUser->GetZoneID() != aUser->GetZoneID())
				continue;

			Packet result(WIZ_CHAT, uint8(MONUMENT_NOTICE));
			result << uint8(FORCE_CHAT) << aUser->GetNation() << sHelpMessage.c_str();
			aUser->Send(&result);
		}
	}
}
#pragma endregion

enum class ZindanOp { flagsend, updatescore, logout };
time_t getzindanremtime() { return g_pMain->pSpecialEvent.finishtime - UNIXTIME >= 0 ? g_pMain->pSpecialEvent.finishtime - UNIXTIME : (uint32)0; }
void CUser::ZindanWarSendScore()
{
	if (!g_pMain->pSpecialEvent.opened || GetZoneID() != ZONE_SPBATTLE1)
		return;

	Packet X(XSafe, uint8(XSafeOpCodes::ZindanWar));
	X << (uint8)ZindanOp::flagsend << g_pMain->pSpecialEvent.ename << g_pMain->pSpecialEvent.ekillcount << g_pMain->pSpecialEvent.kname << g_pMain->pSpecialEvent.kkillcount << getzindanremtime();
	Send(&X);
}

void CUser::ZindanWarUpdateScore()
{
	if (!g_pMain->pSpecialEvent.opened || GetZoneID() != ZONE_SPBATTLE1)
		return;

	Packet X(XSafe, uint8(XSafeOpCodes::ZindanWar));
	X << (uint8)ZindanOp::updatescore << GetNation();
	if (GetNation() == (uint8)Nation::ELMORAD) X << ++g_pMain->pSpecialEvent.ekillcount;
	else X << ++g_pMain->pSpecialEvent.kkillcount;
	g_pMain->Send_All(&X, nullptr, (uint8)Nation::ALL, ZONE_SPBATTLE1);
}

void CUser::ZindanWarLogOut()
{
	if (!g_pMain->pSpecialEvent.opened || GetZoneID() != ZONE_SPBATTLE1)
		return;

	Packet X(XSafe, uint8(XSafeOpCodes::ZindanWar));
	X << (uint8)ZindanOp::logout;
	Send(&X);
}

void CGameServerDlg::SpecialEventFinish()
{
	std::string score = "";
	if (pSpecialEvent.ekillcount > pSpecialEvent.kkillcount)
		score = string_format("The Human Nationality race has won the %s.", pSpecialEvent.eventname.empty() ? "-" : pSpecialEvent.eventname.c_str());
	else if (pSpecialEvent.kkillcount > pSpecialEvent.ekillcount)
		score = string_format("The Karus Nationality race has won the %s.", pSpecialEvent.eventname.empty() ? "-" : pSpecialEvent.eventname.c_str());

	if (pSpecialEvent.zoneid == ZONE_SPBATTLE1)
	{
		ZindanWarStartTimeTickCount = 0;
		ZindanWarSummonCheck = false;
		ZindanLastSummonTime = 0;
		ZindanWarSummonStage = 1;
		ZindanWarisSummon = false;
		ZindanWarLastSummon = false;
		ZindanSpawnList.clear();
	}

	if (score.size()) SendAnnouncement(score.c_str());
	KickOutZoneUsers(pSpecialEvent.zoneid);
	pSpecialEvent.Initialize();
}

#if LISANS_SYSTEM  == 1
// 18.10.2020 Belirlenen Tarihe Lisanslama start
bool CGameServerDlg::LicenseSystem()
{
	DateTime now(&g_localTime);

#define GUN 0
#define AY 1
#define YIL 2
#define SAAT 3
#define DAKIKA 4

	static uint32 lisansTarih[] = { 01, 30, 2026, 23, 59 };
	bool kapat = false;

	uint32 yil = now.GetYear();

	if (yil > lisansTarih[YIL])
		kapat = true;
	else if (now.GetMonth() > lisansTarih[AY] && yil == lisansTarih[YIL])
		kapat = true;
	else if (now.GetDay() > lisansTarih[GUN] && now.GetMonth() == lisansTarih[AY] && yil == lisansTarih[YIL])
		kapat = true;
	else if (now.GetHour() > lisansTarih[SAAT] && now.GetDay() == lisansTarih[GUN] && now.GetMonth() == lisansTarih[AY] && yil == lisansTarih[YIL])
		kapat = true;
	else if (now.GetMinute() > lisansTarih[DAKIKA] && now.GetHour() == lisansTarih[SAAT] && now.GetDay() == lisansTarih[GUN] && now.GetMonth() == lisansTarih[AY] && yil == lisansTarih[YIL])
		kapat = true;

	if (kapat) //true ise
	{
		printf("Lisans Bitti\n");
		ExitProcess(1); // DeaFSezo lisans bittiği gibi kapatma
		return false;
	}
	return true;
}
// 18.10.2020 Belirlenen Tarihe Lisanslama end
#endif
bool CGameServerDlg::getTwoPoint(const std::string& s, const std::string& start_delim, const std::string& stop_delim, std::string& out)
{
	size_t first_delim_pos = s.find(start_delim);
	if (first_delim_pos == std::string::npos) return false;
	size_t end_pos_of_first_delim = first_delim_pos + start_delim.length();
	size_t last_delim_pos = s.find_first_of(stop_delim, end_pos_of_first_delim);
	if (last_delim_pos == std::string::npos) return false;
	out = s.substr(end_pos_of_first_delim, last_delim_pos - end_pos_of_first_delim);
	return true;
}

bool CGameServerDlg::startsWithCaseInsensitive(std::string mainStr, std::string toMatch)
{
	std::transform(mainStr.begin(), mainStr.end(), mainStr.begin(), ::tolower);
	std::transform(toMatch.begin(), toMatch.end(), toMatch.begin(), ::tolower);
	return mainStr.find(toMatch) == 0;
}

static std::string readBuffer;

size_t CGameServerDlg::WriteCallback(void* contents, size_t size, size_t nmemb, void* userp)
{
	((std::string*)userp)->append((char*)contents, size * nmemb);
	return size * nmemb;
}


void CGameServerDlg::UserCallback(StreamUser u, uint32 multipier)	// Trovo
{
	std::string userName = u.userName;
	uint32 kc = multipier * rewardKC;
	// ödül ver
	if (u.userName.empty() || u.userName.length() > 21)
		return;


	if (CUser* pUser = GetUserPtr(u.userName, NameType::TYPE_CHARACTER))
	{
		if (!pUser->m_strAccountID.empty() && !pUser->isInGame())
			return;

		pUser->GiveBalance(kc);
	}
}

void CGameServerDlg::TrovoLive()			// Trovo
{
	CURL* curl;
	curl = curl_easy_init();

	CURLcode res;

	if (curl)
	{

		readBuffer.clear();
		curl_easy_setopt(curl, CURLOPT_URL, api.c_str());
		curl_easy_setopt(curl, CURLOPT_HEADER, FALSE);
		curl_easy_setopt(curl, CURLOPT_POST, TRUE);
		curl_easy_setopt(curl, CURLOPT_POSTFIELDS, query.c_str());
		curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, FALSE);

		curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, WriteCallback);
		curl_easy_setopt(curl, CURLOPT_WRITEDATA, &readBuffer);
		curl_easy_setopt(curl, CURLOPT_USERAGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/83.0.4103.106 Safari/537.36");
		curl_easy_setopt(curl, CURLOPT_VERBOSE, 0L);

		res = curl_easy_perform(curl);

		if (res != CURLE_OK)
		{
			printf("Trovo API : CURL Error . \n");
			return;
		}
		curl_easy_cleanup(curl);
		auto j = json::parse(readBuffer, nullptr, false);

		if (j[0].contains("data"))
		{
			auto a = j[0]["data"];
			if (a.contains("getCategoryDetail"))
			{

				auto b = a["getCategoryDetail"];
				if (b.contains("liveList"))
				{
					auto c = b["liveList"];
					for (auto f : c)
					{
						if (f.contains("channelInfo"))
						{

							auto v = f["programInfo"];
							std::string title = v["title"].get<std::string>();
							if (startsWithCaseInsensitive(title, ""))
							{
								std::string displayName = v["id"].get<std::string>();

								auto it = std::find_if(streamUsers.begin(), streamUsers.end(), find_user(displayName));
								if (it == streamUsers.end())
								{
									StreamUser nUser;
									if (getTwoPoint(title, "[", "]", nUser.userName))
									{
										auto itr = std::find_if(streamUsers.begin(), streamUsers.end(), find_user_nick(nUser.userName));
										if (itr == streamUsers.end())
										{
											uint32 min = 1;
											nUser.displayName = displayName;
											nUser.streamMinute = min;
											streamUsers.push_back(nUser);

										}
									}
								}

							}
						}
					}
				}
			}
		}




		j.clear();

	}
}

uint32 CGameServerDlg::Timer_BotSessions(void* lpParam)
{
	while (true)
	{
		/*g_pMain->m_BotcharacterNameLock.lock();
		foreach(itr, g_pMain->m_BotcharacterNameMap) {
			CBot* pBot = itr->second;
			if (pBot == nullptr || !pBot->isInGame()) continue;

			if (!pBot->MerchantChat.empty() && pBot->m_lastmerchanttime - UNIXTIME > (4 * MINUTE))
				pBot->SendMerchantChat();
		}
		g_pMain->m_BotcharacterNameLock.unlock();*/
		Sleep(30 * SECOND);
	}
	return 0;
}

SIZE_T CGameServerDlg::GetProcessMemoryUsage()
{
	PROCESS_MEMORY_COUNTERS_EX pmc;
	if (GetProcessMemoryInfo(GetCurrentProcess(), (PROCESS_MEMORY_COUNTERS*)&pmc, sizeof(pmc)))
	{
		SIZE_T physMemUsedByMe = pmc.WorkingSetSize / 1024 / 1024; // MB cinsinden
		return physMemUsedByMe;
	}
	return 0;
}

void CGameServerDlg::JstKOAddBotMiningStart()
{
	int Minute = 9999, Restie = 1, minlevel = 83, sCount = 1;

	//////////////////////////////////////////////////////////////////////////////////////////////////////
	for (int i = 0; i < sCount; i++) //Moradon
	{
		float Bonc = myrand(1, 15) * 1.0f;
		float Bonc2 = myrand(1, 15) * 1.0f;

		g_pMain->SpawnUserBot(Minute, 21, 642.700012f + Bonc, 11.500000f, 374.899994f + Bonc2, Restie, minlevel);
	}
	//////////////////////////////////////////////////////////////////////////////////////////////////////
	for (int i = 0; i < sCount; i++) //EMC - 1
	{
		float Bonc = myrand(1, 15) * 1.0f;
		float Bonc2 = myrand(1, 15) * 1.0f;

		g_pMain->SpawnUserBot(Minute, 2, 1427.300049f + Bonc, 12.400000f, 379.500000f + Bonc2, Restie, minlevel);
	}
	for (int i = 0; i < sCount; i++) //EMC - 2
	{
		float Bonc = myrand(1, 15) * 1.0f;
		float Bonc2 = myrand(1, 15) * 1.0f;

		g_pMain->SpawnUserBot(Minute, 2, 1697.300049f + Bonc, 23.400000f, 570.200012f + Bonc2, Restie, minlevel);
	}
	//////////////////////////////////////////////////////////////////////////////////////////////////////
	for (int i = 0; i < sCount; i++) //LUFER - 1
	{
		float Bonc = myrand(1, 15) * 1.0f;
		float Bonc2 = myrand(1, 15) * 1.0f;

		g_pMain->SpawnUserBot(Minute, 1, 644.099976f + Bonc, 11.400000f, 1653.900024f + Bonc2, Restie, minlevel);
	}
	for (int i = 0; i < sCount; i++) //LUFER - 2
	{
		float Bonc = myrand(1, 15) * 1.0f;
		float Bonc2 = myrand(1, 15) * 1.0f;

		g_pMain->SpawnUserBot(Minute, 1, 347.100006f + Bonc, 18.100000f, 1471.199951f + Bonc2, Restie, minlevel);
	}
}

#pragma region JstKO Moranker Rank Display System
uint16 CGameServerDlg::GetMorankerDisplayID(uint8 slotID)
{
	if (slotID < 1 || slotID > 6)
		return 0;
	return uint16(MAX_USER + 600 + slotID);
}

bool CGameServerDlg::IsMorankerDisplayID(uint16 id)
{
	return id >= GetMorankerDisplayID(1) && id <= GetMorankerDisplayID(6);
}

void CGameServerDlg::BroadcastMorankerBotToZone(CBot* pBot)
{
	if (pBot == nullptr || !pBot->isInGame())
		return;

	Packet inout;
	pBot->GetInOut(inout, INOUT_IN);
	Send_Zone(&inout, pBot->GetZoneID());

	Packet req(WIZ_REQ_USERIN);
	req << uint16(1) << uint8(0) << pBot->GetID();
	pBot->GetUserInfo(req);
	Send_Zone(&req, pBot->GetZoneID());
}

void CGameServerDlg::ClearMorankerRankBots()
{
	for (uint8 slot = 1; slot <= 6; slot++)
	{
		uint16 botID = GetMorankerDisplayID(slot);
		CBot* pBot = GetBotPtr(botID);
		if (pBot == nullptr)
			continue;

		if (pBot->isInGame())
			pBot->UserInOut(INOUT_OUT);

		RemoveMapBotList(botID, pBot->GetName());
		delete pBot;
	}
}

bool CGameServerDlg::SpawnMorankerRankBots()
{
	std::vector<_BOT_DATA*> botList;
	if (!g_DBAgent.LoadMorankerBotTable(botList))
		return false;

	uint8 spawned = 0;
	for (auto itr = botList.begin(); itr != botList.end(); ++itr)
	{
		_BOT_DATA* pBotInfo = *itr;
		if (pBotInfo == nullptr)
			continue;

		uint16 botID = pBotInfo->m_sSid;
		if (!IsMorankerDisplayID(botID))
		{
			printf("Moranker: invalid display id %d skipped.\n", botID);
			delete pBotInfo;
			continue;
		}

		CBot* pOldBot = GetBotPtr(botID);
		if (pOldBot != nullptr)
		{
			if (pOldBot->isInGame())
				pOldBot->UserInOut(INOUT_OUT);
			RemoveMapBotList(botID, pOldBot->GetName());
			delete pOldBot;
		}

		C3DMap* pMap = GetZoneByID(pBotInfo->m_bZone);
		if (pMap == nullptr)
		{
			printf("Moranker: zone %d not found for %s.\n", pBotInfo->m_bZone, pBotInfo->m_strUserID.c_str());
			delete pBotInfo;
			continue;
		}

		CBot* pBot = new CBot();
		pBot->Initialize();
		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;
		pBot->m_bZone = pBotInfo->m_bZone;
		pBot->m_pMap = pMap;
		pBot->m_sSid = botID;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;
		pBot->m_bKnightsRank = pBotInfo->m_bKareli;
		pBot->m_bPersonalRank = pBotInfo->m_bKaresiz;
		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->m_BotState = BOT_AFK;
		pBot->m_bAuthority = 1;
		pBot->m_bNeedParty = 0;
		pBot->m_bPartyLeader = false;
		pBot->m_bInParty = false;
		pBot->m_bGenieStatus = false;
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = true;
		pBot->LastWarpTime = 0;
		pBot->m_bAbnormalType = ABNORMAL_NORMAL;
		pBot->m_nOldAbnormalType = ABNORMAL_NORMAL;
		pBot->m_bResHpType = USER_STANDING;
		pBot->m_sDirection = (botID <= GetMorankerDisplayID(3)) ? 90 : -90;

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->SetBotAbility(false);
		pBot->SetPosition(pBotInfo->m_curx, pBotInfo->m_cury, pBotInfo->m_curz);
		pBot->m_oldx = pBotInfo->m_curx;
		pBot->m_oldy = pBotInfo->m_cury;
		pBot->m_oldz = pBotInfo->m_curz;
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());

		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		pBot->SendMoveResult(pBot->GetX(), pBot->GetY(), pBot->GetZ(), 0, 0, pBot->GetID());
		pBot->StateChangeServerDirect(1, USER_STANDING);
		BroadcastMorankerBotToZone(pBot);

		printf("Moranker CLEAN: sid=%d name=%s nation=%d pos=%.2f/%.2f/%.2f dir=%d spawned\n",
			pBot->GetID(), pBot->GetName().c_str(), pBot->GetNation(), pBot->GetX(), pBot->GetY(), pBot->GetZ(), pBot->m_sDirection);

		spawned++;
		delete pBotInfo;
	}

	printf("Moranker CLEAN: %d rank bot spawned.\n", spawned);
	return spawned > 0;
}
#pragma endregion
