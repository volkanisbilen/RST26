#include "stdafx.h"
#include "KingSystem.h"
#include "DBAgent.h"
#include <iostream>
#include "MagicInstance.h"

uint32 CGameServerDlg::Timer_UpdateGameTime(void* lpParam)
{
	uint32 nWeekDay = g_localTime.tm_wday;
	while (g_bRunning)
	{
		g_pMain->UpdateGameTime();

		if (!g_pMain->pCollectionRaceEvent.isCRActive && UNIXTIME > g_pMain->autocrchecktime) {
			g_pMain->m_CollectionRaceListArray.m_lock.lock();
			auto copylist = g_pMain->m_CollectionRaceListArray.m_UserTypeMap;
			g_pMain->m_CollectionRaceListArray.m_lock.unlock();

			foreach(itr, copylist) {
				auto* p = itr->second;
				if (!p) continue;

				if (!p->autostart || p->autohour < 0 || p->autominute < 0)
					continue;

				std::list<std::string> nInGameEvent = StrSplit(p->days, ",");
				uint8 daysize = (uint8)nInGameEvent.size();
				if (!daysize)
					continue;

				uint8 nInGameDays = 0;
				for (int i = 0; i < daysize; i++)
				{
					nInGameDays = atoi(nInGameEvent.front().c_str());
					if (nInGameDays == nWeekDay && p->autohour == g_pMain->m_sHour && p->autominute == g_pMain->m_sMin && g_pMain->m_sSec == 0) {
						g_pMain->autocrchecktime = UNIXTIME + 5;
						g_pMain->CollectionRaceStart(p, itr->first, nullptr);
						printf("CollectionRace AutoStart - EventName[%s]-EventID[%d]-nInGameDays[%d]-nWeekDay[%d]-autohour[%d]-m_sHour[%d]-autominute[%d]-m_sMin[%d]-m_sSec[%d]\n",
							p->EventName.c_str(), p->m_EventID, nInGameDays, nWeekDay, p->autohour, g_pMain->m_sHour, p->autominute, g_pMain->m_sMin, g_pMain->m_sSec);
					}
					nInGameEvent.pop_front();
				}
			}
		}

		sleep(1 * SECOND);
	}
	return 0;
}

void CGameServerDlg::UpdateGameTime()
{
	DateTime now(&g_localTime);
#if XTREME_LISANS  == 1
	g_pMain->LicenseSystem(); // 18.10.2020 Belirlenen Tarihe Lisanslama
#endif
	Packet result;

	if (m_sSec != now.GetSecond())
	{
		uint16 m_bOnlineCount = 0, m_bMerchantCount = 0, m_bMiningCount = 0, m_bGenieCount = 0, m_bFishingCount = 0;
		
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUser = g_pMain->GetUserPtr(i);
			if (pUser == nullptr || !pUser->isInGame())
				continue;

			if (pUser->isMerchanting())
				m_bMerchantCount++;

			if (pUser->isMining())
				m_bMiningCount++;

			if (pUser->isFishing())
				m_bFishingCount++;

			if (pUser->isGenieActive())
				m_bGenieCount++;

			m_bOnlineCount++;
		}
		g_pMain->m_checktime -= 1 * SECOND;

		if (g_pMain->m_checktime <= 0)
		{
			g_pMain->m_checktime = 60 * SECOND;

			MEMORYSTATUSEX statex;
			statex.dwLength = sizeof(statex);
			GlobalMemoryStatusEx(&statex);

			DWORDLONG totalRAMMB = statex.ullTotalPhys / (1024 * 1024);
			DWORDLONG availRAMMB = statex.ullAvailPhys / (1024 * 1024);
			SIZE_T physMemUsedByMe = GetProcessMemoryUsage(); // Bellek kullan�m�n� al
			std::string texts;
			texts = string_format("JstKO Systems [Online :%d - Merchant :%d - Mining :%d - Fishing :%d - Genie :%d] || RAM: %llu MB",
				m_bOnlineCount, m_bMerchantCount, m_bMiningCount, m_bFishingCount, m_bGenieCount, physMemUsedByMe);
			SetConsoleTitle(texts.c_str());
		}

		std::vector<CBot*> deleted;
		// Check timed King events.
		m_KingSystemArray.m_lock.lock();
		foreach_stlmap_nolock(itr, m_KingSystemArray)
		{
			if (itr->second == nullptr)
				continue;

			itr->second->CheckKingTimer();
		}
		m_KingSystemArray.m_lock.unlock();
		RLOCK(m_MapBotList);
		foreach_stlmap_nolock(itr, m_MapBotList)
		{
			CBot* pUser = itr->second;

			if (pUser == nullptr)
				continue;

			if (!pUser->isInGame())
				continue;

			if (pUser->LastWarpTime < UNIXTIME)
				deleted.push_back(pUser);

			if (pUser->restyping > 12 && pUser->restyping < 15)
				pUser->FindMonsterAttackSlot();

			if ((pUser->m_bResHpType == USER_MINING
				|| pUser->m_bResHpType == USER_FLASHING)
				&& pUser->m_for_bottime + 15 < uint32(UNIXTIME))
			{
				Packet result(WIZ_MINING, uint8(MiningAttempt));
				uint16 resultCode = MiningResultSuccess, Random = myrand(0, 10000);
				uint16 sEffect = 0;

				if (Random > 4000
					|| pUser->m_bResHpType == USER_SITDOWN) // EXP
					sEffect = 13082; // "XP" effect
				else
					sEffect = 13081; // "Item" effect

				result << resultCode << pUser->GetID() << sEffect;
				pUser->SendToRegion(&result);
				pUser->m_for_bottime = uint32(UNIXTIME);
			}
		}
		RULOCK(m_MapBotList);

		foreach(itr, deleted) {
			if ((*itr)->isSlaveMerchant())
			{
				CUser* pSlaveUser = g_pMain->GetUserPtr((*itr)->m_bSlaveUserID);
				if (pSlaveUser != nullptr && pSlaveUser->isSlave <= 0)
					pSlaveUser->GiveSlaveMerchantItems();
			}

			CUser* pSlaveUser = g_pMain->GetUserPtr((*itr)->m_bSlaveUserID);

			if (!(*itr)->isSlaveMerchant())
				(*itr)->UserInOut(INOUT_OUT);
			else if ((*itr)->isSlaveMerchant() && pSlaveUser->isSlave <= 0)
				pSlaveUser->UserInOut(INOUT_OUT);
		}
	}

	if (m_sMin != now.GetMinute())
	{
		m_ReloadKnightAndUserRanksMinute++;
		if (m_ReloadKnightAndUserRanksMinute == RELOAD_KNIGHTS_AND_USER_RATING) {
			m_ReloadKnightAndUserRanksMinute = 0;
			ReloadKnightAndUserRanks(false);
		}

		if (m_RankRewardSendStatus)
		{
			// Player Ranking Rewards
			std::list<std::string> vargs = StrSplit(m_sPlayerRankingsRewardZones, ",");
			uint8 nZones = (uint8)vargs.size();
			if (nZones > 0)
			{
				uint8 nZoneID = 0;
				for (int i = 0; i < nZones; i++)
				{
					nZoneID = atoi(vargs.front().c_str());
					SetPlayerRankingRewards(nZoneID);
					vargs.pop_front();
				}
			}
			m_RankRewardSendStatus = false;
		}
	}

	// Every hour
	if (m_sHour != now.GetHour())
	{
		ResetPlayerRankings();
		UpdateWeather();
		//SetGameTime();

		if (m_bSantaOrAngel)
			SendFlyingSantaOrAngel();

		result.clear();
		result.Initialize(WIZ_DB_SAVE);
		result << uint8(ProcDbServerType::UpdateKnights);
		g_pMain->AddDatabaseRequest(result);
	}

	// Every day
	if (m_sDate != now.GetDay())
	{
		EventTimerSet();
		g_pMain->UpdateFlagAndCape();
		//printf("G3 Ustu Klanlarin Pelerinleri Dusuruldu!\n");


		result.clear();
		result.Initialize(WIZ_DB_SAVE);
		result << uint8(ProcDbServerType::UpdateSiegeWarfareDb);
		g_pMain->AddDatabaseRequest(result);

		m_KingSystemArray.m_lock.lock();
		foreach_stlmap_nolock(itr, m_KingSystemArray)
		{
			if (itr->second == nullptr)
				continue;

			result.clear();
			result.Initialize(WIZ_DB_SAVE);
			result << uint8(ProcDbServerType::UpdateKingSystemDb);
			result << itr->second->m_byNation << itr->second->m_nNationalTreasury << itr->second->m_nTerritoryTax;
			g_pMain->AddDatabaseRequest(result);
		}
		m_KingSystemArray.m_lock.unlock();

		//akara 
		std::vector<AKARA_LIST> AkaraList;
		g_DBAgent.LoadAkaraList(AkaraList);

		if (!AkaraList.empty())
		{
			for (size_t i = 0; i < AkaraList.size(); i++)
			{
				for (int b = 0; b < 3; b++)
				{
					if (AkaraList.at(i).BetMax[b] > 0)
					{
						try
						{
							std::string name = "JstKO";
							std::string subject = "Akara's Altar";
							std::string message = "congratulations";

							_ITEM_DATA pItem{};
							_ITEM_TABLE pTable = g_pMain->GetItemPtr(AkaraList.at(i).RealItemNum[b]);

							if (pTable.isnull() || AkaraList.at(i).Owner[b].empty())
								continue;

							pItem.nNum = pTable.m_iNum;
							pItem.sCount = 1;
							pItem.sDuration = pTable.m_sDuration;
							pItem.nSerialNum = g_pMain->GenerateItemSerial();
							pItem.nExpirationTime = 0;

							g_DBAgent.SendLetter(name, AkaraList.at(i).Owner[b], subject, message, 2, &pItem, 0);
							g_DBAgent.UpdateUserAkaraHistory(i + 1, b, AkaraList, 3);
							g_DBAgent.UpdateAkaraHistory(i + 1, b, AkaraList, 3);
						}
						catch (const std::exception& e)
						{
							std::cerr << "An error occurred: " << e.what() << std::endl;
						}
					}
				}
			}

			for (int i = 0; i < 3; i++)
			{
				g_DBAgent.ClearAkaraList(i);
			}
		}
		//akara son
	}

	// Every month
	if (m_sMonth != now.GetMonth())
	{
		// Reset monthly NP.
		result.clear();
		result.Initialize(WIZ_DB_SAVE);
		result << uint8(ProcDbServerType::ResetLoyalty);
		g_pMain->AddDatabaseRequest(result);
	}

	if (!m_sSec)
	{
		foreach_stlmap(itr, g_pMain->m_AutomaticCommandArray)
		{
			auto* p = itr->second;
			if (!p || p->command.empty()) continue;

			if (p->hour != m_sHour || p->minute != m_sMin)
				continue;

			if (p->iDay != 7 && p->iDay != g_localTime.tm_wday)
				continue;

			ProcessServerCommand(p->command);
		}
	}

	if (g_localTime.tm_wday == 0 && now.GetHour() == 0 && now.GetMinute() == 0 && now.GetSecond() == 0 && !king_selectreq) {
		printf("king_selection paketi gonderildi\n");
		king_selectreq = true;
		Packet newpkt(WIZ_DB_SAVE, uint8(ProcDbServerType::king_selection));
		AddDatabaseRequest(newpkt);
	}

	// Update the server time
	m_sYear = now.GetYear();
	m_sMonth = now.GetMonth();
	m_sDate = now.GetDay();
	m_sHour = now.GetHour();
	m_sMin = now.GetMinute();
	m_sSec = now.GetSecond();

}


void CUser::BotUsingSkill(uint16 sTargetID, uint32 nSkillID)
{
	MagicInstance instance;
	instance.bIsRunProc = true;
	instance.sCasterID = sTargetID;
	instance.sTargetID = GetSocketID();
	instance.nSkillID = nSkillID;
	instance.sSkillCasterZoneID = GetZoneID();
	instance.Run();
}

uint32 CGameServerDlg::Timer_t_1(void* lpParam)
{
#if(SKILLTEST)

	return 0;

	while (g_bRunning)
	{
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUser = g_pMain->GetUserPtr(i);
			if (pUser == nullptr || !pUser->isInGame()) continue;

			CNpcThread* zoneitrThread = g_pMain->m_arNpcThread.GetData(pUser->GetZoneID());
			if (!zoneitrThread)
				continue;

			zoneitrThread->m_arNpcArray.m_lock.lock();
			auto copymap = zoneitrThread->m_arNpcArray.m_UserTypeMap;
			zoneitrThread->m_arNpcArray.m_lock.unlock();

			foreach(ax, copymap)
			{
				if (ax->second && ax->second->GetProtoID() == 13013)
					pUser->ClientEvent(ax->second->GetID());
			}
		}

		sleep(1000);
	}
#endif
	return (uint32)0;
}

uint32 CGameServerDlg::Timer_t_2(void* lpParam)
{
#if(SKILLTEST)


	while (g_bRunning)
	{
		/*g_pMain->m_arNpcThread.m_lock.lock();
		foreach_stlmap_nolock(iyt, g_pMain->m_arNpcThread) {
			if (!iyt->second) continue;
			iyt->second->m_arNpcArray.m_lock.lock();
			foreach_stlmap_nolock(a, iyt->second->m_arNpcArray) {
				if (a->second) {
					a->second->IsNoPathFind(10.0f);
					a->second->Type4Duration();
				}
			}
			iyt->second->m_arNpcArray.m_lock.unlock();
		}
		g_pMain->m_arNpcThread.m_lock.unlock();*/


		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUser = g_pMain->GetUserPtr(i);
			if (pUser == nullptr || !pUser->isInGame())
				continue;

			/*pUser->SendPremiumInfo();

			pUser->InitType4();
			pUser->Type4Duration();
			pUser->Update();
			pUser->m_bInvisibilityType = (uint8)InvisibilityType::INVIS_DISPEL_ON_MOVE;
			pUser->Type9Duration(true);
			pUser->RemoveStealth();*/

			if (pUser->isWarrior())
			{
				if (pUser->isNoviceWarrior())
				{
					pUser->BotUsingSkill(pUser->GetSocketID(), 105002); //sprint

				}
				else if (pUser->isMasteredWarrior())
				{
					if (pUser->GetNation() == 1)
					{
						pUser->BotUsingSkill(pUser->GetSocketID(), 106002); //sprint
					}
					else
					{
						pUser->BotUsingSkill(pUser->GetSocketID(), 206002); //sprint
					}
				}
			}

			if (UNIXTIME > pUser->testskillusetime)
			{
				pUser->BotUsingSkill(pUser->GetSocketID(), 500354);//814678000
				pUser->BotUsingSkill(pUser->GetSocketID(), 500512);//800220000
				pUser->BotUsingSkill(pUser->GetSocketID(), 500508);//800130000
				pUser->BotUsingSkill(pUser->GetSocketID(), 490160);//800127000

				if (!pUser->CheckExistItem(814678000)) pUser->GiveItem("tst", 814678000);
				if (!pUser->CheckExistItem(800220000)) pUser->GiveItem("tst", 800220000);
				if (!pUser->CheckExistItem(800130000)) pUser->GiveItem("tst", 800130000);
				if (!pUser->CheckExistItem(800127000)) pUser->GiveItem("tst", 800127000);
			}

			if (UNIXTIME > pUser->testskillusetime2)
			{
				std::vector<uint8> willDel;
				pUser->m_buffLock.lock();
				foreach(itr, pUser->m_buffMap) willDel.push_back(itr->first);
				pUser->m_buffLock.unlock();
				foreach(itr, willDel)
					CMagicProcess::RemoveType4Buff((*itr), pUser, true, pUser->isLockableScroll((*itr)));
				pUser->testskillusetime2 = UNIXTIME + myrand(120, 240);
			}
		}
		sleep(1 * SECOND);
	}
#endif
	return (uint32)0;
}

uint32 CGameServerDlg::Timer_t_3(void* lpParam)
{
#if(SKILLTEST)
	return 0;
	while (g_bRunning)
	{
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUser = g_pMain->GetUserPtr(i);
			if (pUser == nullptr || !pUser->isInGame())
				continue;

			CNpcThread* zoneitrThread = g_pMain->m_arNpcThread.GetData(pUser->GetZoneID());
			if (!zoneitrThread)
				continue;

			zoneitrThread->m_arNpcArray.m_lock.lock();
			auto copymap = zoneitrThread->m_arNpcArray.m_UserTypeMap;
			zoneitrThread->m_arNpcArray.m_lock.unlock();

			foreach(ax, copymap)
			{
				if (ax->second && ax->second->GetProtoID() == 13013)
					pUser->ClientEvent(ax->second->GetID());
			}

		}
		sleep(myrand(1250, 2000));
	}
#endif
	return (uint32)0;
}

#pragma region CGameServerDlg::Timer_UpdateSessions(void * lpParam)
uint32 CGameServerDlg::Timer_UpdateSessions(void* lpParam)
{
	while (g_bRunning)
	{
		g_pMain->m_socketMgr.GetLock().lock();
		SessionMap sessMap = g_pMain->m_socketMgr.GetActiveSessionMap();
		g_pMain->m_socketMgr.GetLock().unlock();
		foreach(itr, sessMap)
		{
			CUser* pUser = TO_USER(itr->second);
			if (pUser == nullptr)
				continue;

			if (!pUser->m_strAccountID.empty() && !pUser->isInGame())
			{
				ULONGLONG timeout = KOSOCKET_LOADING_TIMEOUT, nDifference = (UNIXTIME2 - pUser->GetLastResponseTime());
				if (nDifference >= timeout)
				{
					pUser->goDisconnect("time out", __FUNCTION__);
					continue;
				}
			}

			if (pUser->isInGame())
			{
				pUser->Update();
				if (UNIXTIME2 > g_pMain->DelayedTime)
				{
					g_pMain->DelayedTime = UNIXTIME2 + (30 * SECOND);
					pUser->CheckDelayedTime();
				}
			}
		}
		sleep(1 * SECOND);
	}
	return uint32(0);
}
#pragma endregion 

#pragma region CGameServerDlg::Timer_UpdateConcurrent(void * lpParam)
uint32 CGameServerDlg::Timer_UpdateConcurrent(void* lpParam)
{
	while (true)
	{
		g_pMain->ReqUpdateConcurrent();
		sleep(120 * SECOND);
	}
	return 0;
}
#pragma endregion 

#pragma region CGameServerDlg::Timer_TimedNotice(void * lpParam)
uint32 CGameServerDlg::Timer_TimedNotice(void* lpParam)
{
	while (true)
	{
		foreach_stlmap_nolock(itr, g_pMain->m_TimedNoticeArray)
		{
			_TIMED_NOTICE* ptimed = itr->second;
			if (ptimed == nullptr || ptimed->usingtime > UNIXTIME) continue;
			if (ptimed->time < 1) ptimed->time = 1;
			ptimed->usingtime = (uint32)UNIXTIME + ptimed->time * MINUTE;
			Packet result;
			std::string notice = ptimed->notice;
			g_pMain->GetServerResource(IDP_ANNOUNCEMENT, &notice, notice.c_str());
			ChatPacket::Construct(&result, (uint8)ptimed->noticetype, &notice);
			if (ptimed->zoneid != 0)g_pMain->Send_Zone(&result, (uint8)ptimed->zoneid, nullptr, (uint8)Nation::ALL);
			else g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL);
		} sleep(60 * SECOND);


	} 
	
	return uint32(0);
}
#pragma endregion 

uint32 CGameServerDlg::Timer_BotMoving(void* lpParam)
{
	while (true)
	{
		g_pMain->BotHandlerMainTimer();
		Sleep(100);
	}

	return uint32(0);
}

#pragma region CGameServerDlg::ReqUpdateConcurrent()
void CGameServerDlg::ReqUpdateConcurrent()
{
	uint32 sCount = 0;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (!pUser || !pUser->isInGame())
			continue;

		sCount++;
	}
	sCount += (uint32)g_pMain->m_BotcharacterNameMap.size();

	Packet result(WIZ_ZONE_CONCURRENT);
	result << uint32(m_nServerNo)
		<< sCount;
	AddDatabaseRequest(result);
}
#pragma endregion

#pragma region CGameServerDlg::king_loyaltyselection()
void CGameServerDlg::king_loyaltyselection() //Write Gkhn
{
	printf("king_loyaltyselection voidine girdi\n");
	std::vector<std::string> mlist;
	auto* pKing1 = g_pMain->m_KingSystemArray.GetData((uint8)Nation::KARUS);
	if (pKing1 && !pKing1->m_strKingName.empty())
		mlist.emplace_back(pKing1->m_strKingName);

	auto* pKing2 = g_pMain->m_KingSystemArray.GetData((uint8)Nation::ELMORAD);
	if (pKing2 && !pKing2->m_strKingName.empty())
		mlist.emplace_back(pKing2->m_strKingName);

	for (auto it : mlist) {
		g_DBAgent.KingAddAndDelete(it, 0, 0);
		auto* pUser = g_pMain->GetUserPtr(it, NameType::TYPE_CHARACTER);
		if (pUser) pUser->CharacterOlduguYerdeYenile();
	}

	std::string k_king = "", e_king = "";
	foreach(itr, m_UserKarusPersonalRankMap)
		if (itr->second && itr->second->nRank == 1)
			k_king = itr->second->strUserName;

	foreach(itr, m_UserElmoPersonalRankMap)
		if (itr->second && itr->second->nRank == 1)
			e_king = itr->second->strUserName;

	CUser* pK_king = nullptr, * pE_king = nullptr;
	if (!k_king.empty()) pK_king = GetUserPtr(k_king, NameType::TYPE_CHARACTER);
	if (!e_king.empty()) pE_king = GetUserPtr(e_king, NameType::TYPE_CHARACTER);

	std::string notice = "";

	if (pK_king) {
		g_DBAgent.KingAddAndDelete(pK_king->GetName(), 1, 0);
		notice = string_format("%s Haftalik Sag Np Siralamasinda 1. Oldugu icin Kral Olmustur...", pK_king->GetName().c_str());
		SendChat<ChatType::WAR_SYSTEM_CHAT>(notice.c_str(), (uint8)Nation::KARUS, true);
		pK_king->GiveBalance(0, KingRewardKC);
		pK_king->m_bRank = 1;
		pK_king->CharacterOlduguYerdeYenile();
		printf("%s\n", notice.c_str());
	}
	else if (!k_king.empty()) {
		g_DBAgent.KingAddAndDelete(k_king, 1, KingRewardKC);
		notice = string_format("%s Haftalik Sag Np Siralamasinda 1. Oldugu icin Kral Olmustur...", k_king.c_str());
		SendChat<ChatType::WAR_SYSTEM_CHAT>(notice.c_str(), (uint8)Nation::KARUS, true);
		printf("%s\n", notice.c_str());
	}

	if (pE_king) {
		g_DBAgent.KingAddAndDelete(pE_king->GetName(), 1, 0);
		notice = string_format("%s Haftalik Sag Np Siralamasinda 1. Oldugu icin Kral Olmustur...", pE_king->GetName().c_str());
		SendChat<ChatType::WAR_SYSTEM_CHAT>(notice.c_str(), (uint8)Nation::ELMORAD, true);
		pE_king->GiveBalance(0, KingRewardKC);
		pE_king->m_bRank = 1;
		pE_king->CharacterOlduguYerdeYenile();
		printf("%s\n", notice.c_str());
	}
	else if (!e_king.empty()) {
		g_DBAgent.KingAddAndDelete(e_king, 1, KingRewardKC);
		notice = string_format("%s Haftalik Sag Np Siralamasinda 1. Oldugu icin Kral Olmustur...", e_king.c_str());
		SendChat<ChatType::WAR_SYSTEM_CHAT>(notice.c_str(), (uint8)Nation::ELMORAD, true);
		printf("%s\n", notice.c_str());
	}

	m_KingSystemArray.DeleteAllData();
	LoadKingSystem();
}
#pragma endregion