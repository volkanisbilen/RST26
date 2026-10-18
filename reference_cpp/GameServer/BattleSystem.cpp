#include "stdafx.h"
#include "DBAgent.h"

#pragma region Battle System

#pragma region CGameServerDlg::GetBattleAndNationMonumentName(int16 TrapNumber, uint8 ZoneID)
std::string CGameServerDlg::GetBattleAndNationMonumentName(int16 TrapNumber, uint8 ZoneID)
{
	std::string sMonumentName = ZoneID == 0 ? "Nereid Monument" : "DECLARE_NATION_MONUMENT_STATUS";

	if (ZoneID == 0)
	{
		if (TrapNumber == 1)
			sMonumentName = "El Morad main territory";
		else if (TrapNumber == 2)
			sMonumentName = "El Morad provision line";
		else if (TrapNumber == 3)
			sMonumentName = "Lake of Life";
		else if (TrapNumber == 4)
			sMonumentName = "Foss Castle";
		else if (TrapNumber == 5)
			sMonumentName = "Karus main territory";
		else if (TrapNumber == 6)
			sMonumentName = "Karus provision line";
		else if (TrapNumber == 7)
			sMonumentName = "Swamp of Shadows";
	}
	else
	{
		if (ZoneID == ZONE_KARUS && TrapNumber == 1)
			TrapNumber = 2;
		else if (ZoneID == ZONE_KARUS && TrapNumber == 2)
			TrapNumber = 1;

		if (TrapNumber == 0)
			sMonumentName = string_format("%s Monument", ZoneID == ZONE_KARUS ? "Luferson" : "El Morad");
		else if (TrapNumber == 1)
			sMonumentName = string_format("%s Monument", ZoneID == ZONE_KARUS ? "Bellua" : "Asga Village");
		else if (TrapNumber == 2)
			sMonumentName = string_format("%s Monument", ZoneID == ZONE_KARUS ? "Linate" : "Raiba Village");
		else if (TrapNumber == 3)
			sMonumentName = string_format("%s Monument", ZoneID == ZONE_KARUS ? "Laon Camp" : "Dodo Camp");
	}

	return sMonumentName;
}
#pragma endregion

#pragma region CGameServerDlg::CheckNationMonumentRewards()
void CGameServerDlg::CheckNationMonumentRewards()
{
	std::vector<uint16> deleted;
	std::vector< _MONUMENT_INFORMATION> m_winnerlist, m_defeatlist;

	m_NationMonumentWinnerNationArray.m_lock.lock();
	foreach_stlmap_nolock(itr, m_NationMonumentWinnerNationArray) {
		_MONUMENT_INFORMATION* pWinnerMonu = itr->second;
		if (pWinnerMonu == nullptr || pWinnerMonu->RepawnedTime - int32(UNIXTIME) > 0)
			continue;

		pWinnerMonu->RepawnedTime = int32(UNIXTIME) + 300;
		m_winnerlist.push_back(*pWinnerMonu);
	}
	m_NationMonumentWinnerNationArray.m_lock.unlock();

	foreach(itr, m_winnerlist) {
		CNpc* pNpc = GetNpcPtr(itr->sNid, g_pMain->m_bVictory == (uint8)Nation::KARUS ? ZONE_ELMORAD : ZONE_KARUS);
		if (pNpc == nullptr)
			continue;

		std::vector<uint16> unitList;
		g_pMain->GetUnitListFromSurroundingRegions(pNpc, &unitList);

		foreach(itrx, unitList) {
			Unit* pTarget = g_pMain->GetUnitPtr(*itrx, pNpc->GetZoneID());
			if (pTarget == nullptr || pTarget->isNPC() || !pTarget->isPlayer()
				|| pTarget->GetNation() != pNpc->GetNation()
				|| !pTarget->isInRangeSlow(pNpc, 100))
				continue;

			if (itr->sSid == LUFERSON_MONUMENT_SID || itr->sSid == ELMORAD_MONUMENT_SID)
				TO_USER(pTarget)->SendLoyaltyChange("Nation Monument Reward", 200);
			else
				TO_USER(pTarget)->SendLoyaltyChange("Nation Monument Reward", 50);
		}
		Announcement(DECLARE_NATION_REWARD_STATUS, (uint8)Nation::ALL, itr->sSid, nullptr, pNpc);
		ShowNpcEffect(itr->sNid, 20100, pNpc->GetZoneID());
	}

	m_NationMonumentDefeatedNationArray.m_lock.lock();
	foreach_stlmap_nolock(itr, m_NationMonumentDefeatedNationArray) {
		_MONUMENT_INFORMATION* pDefeatedMonu = itr->second;
		if (pDefeatedMonu == nullptr || pDefeatedMonu->RepawnedTime - int32(UNIXTIME) > 0)
			continue;

		pDefeatedMonu->RepawnedTime = int32(UNIXTIME) + 10000;
		m_defeatlist.push_back(*pDefeatedMonu);
	}
	m_NationMonumentDefeatedNationArray.m_lock.unlock();

	foreach(itr, m_defeatlist) {
		CNpc* pNpc = GetNpcPtr(itr->sNid, g_pMain->m_bVictory == (uint8)Nation::KARUS ? ZONE_KARUS : ZONE_ELMORAD);
		if (pNpc == nullptr)
			continue;

		std::vector<uint16> unitList;
		g_pMain->GetUnitListFromSurroundingRegions(pNpc, &unitList);

		foreach(itrx, unitList) {
			Unit* pTarget = g_pMain->GetUnitPtr(*itrx, pNpc->GetZoneID());
			if (pTarget == nullptr || pTarget->isNPC() || !pTarget->isPlayer()
				|| pTarget->GetNation() != pNpc->GetNation()
				|| !pTarget->isInRangeSlow(pNpc, 100))
				continue;

			if (itr->sSid == LUFERSON_MONUMENT_SID || itr->sSid == ELMORAD_MONUMENT_SID)
				TO_USER(pTarget)->SendLoyaltyChange("Nation Monument Reward", 200);
			else
				TO_USER(pTarget)->SendLoyaltyChange("Nation Monument Reward", 50);
		}

		Announcement(DECLARE_NATION_REWARD_STATUS, (uint8)Nation::ALL, itr->sSid, nullptr, pNpc);
		ShowNpcEffect(itr->sNid, 20100, pNpc->GetZoneID());
		deleted.push_back(itr->sSid);
	}

	foreach(itr, deleted) g_pMain->m_NationMonumentDefeatedNationArray.DeleteData(*itr);
}
#pragma endregion

#pragma region CGameServerDlg::BattleZoneCurrentUsers()
/**
* @brief	Updates the number of users currently in the war zone
* 			and sends the user counts to all servers in this group.
*/
void CGameServerDlg::BattleZoneCurrentUsers()
{
	C3DMap* pMap = GetZoneByID(ZONE_BATTLE_BASE + m_byBattleZone);
	if (pMap == nullptr
		|| m_nServerNo != pMap->m_nServerNo)
		return;

	uint16 nKarusMan = 0, nElmoradMan = 0;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame() || pUser->GetZoneID() != pMap->GetID())
			continue;

		if (pUser->GetNation() == (uint8)Nation::KARUS)
			nKarusMan++;
		else
			nElmoradMan++;
	}

	m_sKarusCount = nKarusMan;
	m_sElmoradCount = nElmoradMan;
}
#pragma endregion

#pragma region CGameServerDlg::BanishLosers()
/**
* @brief	Kicks invaders out of the invaded nation after a war
*			and resets captains.
**/
void CGameServerDlg::BanishLosers()
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		// Reset captains
		if (m_byBattleOpen == NO_BATTLE)
		{
			if (pUser->GetFame() == COMMAND_CAPTAIN)
				pUser->ChangeFame(CHIEF);
		}

		if (m_byBattleOpen == NATION_BATTLE)
		{
			// Kick out losers
			if (pUser->GetMap()->isWarZone() && m_bVictory != pUser->GetNation())
				pUser->KickOutZoneUser(true);
		}
		else if (m_byBattleOpen == NO_BATTLE)
		{
			// Kick out invaders
			if ((pUser->GetZoneID() <= (uint8)Nation::ELMORAD && pUser->GetZoneID() != pUser->GetNation())
				|| pUser->GetMap()->isWarZone())
				pUser->KickOutZoneUser(true);
		}
	}
	
	if (g_pMain->m_byBattleZone + ZONE_BATTLE_BASE == ZONE_BATTLE2)
		BattleZoneRemnantSpawn();
}
#pragma endregion

#pragma region CGameServerDlg::BattleZoneRemnantSpawn()
void CGameServerDlg::BattleZoneRemnantSpawn()
{
	foreach_stlmap(itr, m_WarBanishOfWinnerArray)
	{
		_BANISH_OF_WINNER* pBanish = itr->second;
		if (pBanish == nullptr)
			continue;

		if (pBanish->sNationID != m_bVictory)
			continue;

		//spawnevent kaldýrýlacak.
		SpawnEventNpc(pBanish->sSid, true, pBanish->sZoneID, pBanish->sPosX, 0, pBanish->sPosZ, pBanish->sCount, pBanish->sRadius, pBanish->sDeadTime * MINUTE, 0, -1, 0, 0, 1, 0, 0, 0);
	}
	Announcement(IDS_REMNANT_SUMMON_INFO);
}
#pragma endregion

#pragma region CGameServerDlg::ResetBattleZone()
void CGameServerDlg::ResetBattleZone(uint8 bType)
{
	m_bCommanderSelected = false;
	m_bVictory = 0;

	m_sBanishDelay = 0;
	m_byBanishFlag = false;

	m_sBattleResultDelay = 0;
	m_bResultDelay = false;

	m_bKarusFlag = 0;
	m_bElmoradFlag = 0;

	m_byKarusOpenFlag = false;
	m_byElmoradOpenFlag = false;

	m_byBattleSave = false;

	m_byBattleZone = 0;
	m_byBattleZoneType = 0;

	m_byBattleOpen = NO_BATTLE;
	m_byOldBattleOpen = NO_BATTLE;

	m_byBattleNoticeTime = 0;
	m_byBattleOpenedTime = 0;
	m_byBattleRemainingTime = 0;
	m_sBattleTimeDelay = 0;
	m_byNereidsIslandRemainingTime = 0;

	m_sKarusDead = 0;
	m_sElmoradDead = 0;

	m_sKarusCount = 0;
	m_sElmoradCount = 0;

	m_sKilledKarusNpc = 0;
	m_sKilledElmoNpc = 0;

	m_sKarusMonumentPoint = 0;
	m_sElmoMonumentPoint = 0;
	m_sKarusMonuments = 0;
	m_sElmoMonuments = 0;

	memset(m_sNereidsIslandMonuArray, 0, sizeof(m_sNereidsIslandMonuArray));

	m_NationMonumentWinnerNationArray.DeleteAllData();
	m_NationMonumentDefeatedNationArray.DeleteAllData();

	m_bMiddleStatueNation = 0;

	if (bType == BATTLEZONE_OPEN || bType == BATTLEZONE_CLOSE) {
		foreach_stlmap(itr, m_arNpcThread) {
			if (itr->first != ZONE_KARUS && itr->first != ZONE_ELMORAD
				&& itr->first != ZONE_BATTLE && itr->first != ZONE_BATTLE2
				&& itr->first != ZONE_BATTLE3 && itr->first != ZONE_BATTLE4
				&& itr->first != ZONE_BATTLE5 && itr->first != ZONE_BATTLE6)
				continue;

			CNpcThread* zoneitrThread = itr->second;
			if (zoneitrThread == nullptr) continue;
			zoneitrThread->ChangeAbilityAllNPCs(bType);
		}
	}
}
#pragma endregion

#pragma region CGameServerDlg::BattleEventGiveItem()
void CGameServerDlg::BattleEventGiveItem(int nType)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		Packet result(WIZ_MAP_EVENT);
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (m_byBattleZoneType == BATTLEZONE_OPEN)
		{
			if (nType == BATTLEZONE_OPEN)
				result << uint8(GOLDSHELL) << uint8(1) << pUser->GetSocketID();
			else
				result << uint8(GOLDSHELL) << uint8(0) << pUser->GetSocketID();
		}
		else
			result << uint8(GOLDSHELL) << uint8(0) << pUser->GetSocketID();

		pUser->Send(&result);
	}
}
#pragma endregion

//#pragma region CGameServerDlg::BattleZoneResult()
//void CGameServerDlg::BattleZoneResult(uint8 nation)
//{
//	m_bVictory = nation;
//	Announcement(DECLARE_WINNER, m_bVictory);
//	Announcement(DECLARE_LOSER, nation == (int)Nation::KARUS ? (int)Nation::ELMORAD : (int)Nation::KARUS);
//
//	if (m_byBattleOpen == NATION_BATTLE)
//	{
//		for (uint16 i = 0; i < MAX_USER; i++)
//		{
//			CUser* pUser = g_pMain->GetUserPtr(i);
//			if (pUser == nullptr
//				|| !pUser->isInGame()
//				|| !pUser->GetMap()->isWarZone())
//				continue;
//
//			if (m_bVictory == pUser->GetNation())
//			{
//				pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinWar, 0, nullptr);
//				if (pUser->GetFame() == COMMAND_CAPTAIN)
//					pUser->SendLoyaltyChange("Battle Result", 500);
//			}
//		}
//	}
//
//	if (g_pMain->m_byBattleZoneType == ZONE_ARDREAM)
//	{
//		BattleZoneClose();
//		return;
//	}
//
//	m_byKarusOpenFlag = nation == (uint8)Nation::ELMORAD ? true : false;
//	m_byElmoradOpenFlag = nation == (uint8)Nation::KARUS ? true : false;
//	m_byBanishFlag = true;
//	m_sBanishDelay = 0;
//}
//#pragma endregion

#define OLD_STYLE 0 
#pragma region CGameServerDlg::BattleZoneResult()
void CGameServerDlg::BattleZoneResult(uint8 nation)
{
	m_bVictory = nation;
	Announcement(DECLARE_WINNER, m_bVictory);
	Announcement(DECLARE_LOSER, nation == (int)Nation::KARUS ? (int)Nation::ELMORAD : (int)Nation::KARUS);

	if (m_byBattleOpen == NATION_BATTLE)
	{
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUser = g_pMain->GetUserPtr(i);
			if (pUser == nullptr
				|| !pUser->isInGame()
				|| !pUser->GetMap()->isWarZone())
				continue;

			if (m_bVictory == pUser->GetNation())
			{
				pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinWar, 0, nullptr);
#if OLD_STYLE == 1
				if (pUser->GetFame() == COMMAND_CAPTAIN)
					pUser->SendLoyaltyChange("Battle Result", 500);
#else
				//pUser->GiveItem("WarWinner", 379068000, 1); 
				pUser->SendLoyaltyChange("Battle Result", 1500); 
#endif
			}
#if OLD_STYLE == 0
			else
			{
				//pUser->GiveItem("WarLosers", 379067000, 1); 
				pUser->SendLoyaltyChange("Battle Result", 250); 
			}
#endif
		}
	}

	if (g_pMain->m_byBattleZoneType == ZONE_ARDREAM)
	{
		BattleZoneClose();
		return;
	}

	m_byKarusOpenFlag = nation == (uint8)Nation::ELMORAD ? true : false;
	m_byElmoradOpenFlag = nation == (uint8)Nation::KARUS ? true : false;
	m_byBanishFlag = true;
	m_sBanishDelay = 0;
#if OLD_STYLE == 0
	g_pMain->BattleZoneClose(); 
#endif
}
#pragma endregion


#pragma region CGameServerDlg::BattleZoneSelectCommanders()

void CGameServerDlg::BattleZoneSelectCommanders()
{
	if (m_bCommanderSelected)
		return;

	m_CommanderArray.clear();
	m_KnightsRatingArray[KARUS_ARRAY].DeleteAllData();
	m_KnightsRatingArray[ELMORAD_ARRAY].DeleteAllData();
	LoadUserRankings();
	LoadKnightsRankTable(true, true);

	m_CommanderArrayLock.lock();
	CommanderArray mcopyarray = m_CommanderArray;
	m_CommanderArrayLock.unlock();

	foreach(itr, mcopyarray) {
		if (itr->second == nullptr) continue;
		CUser* pUser = GetUserPtr(itr->second->strName, NameType::TYPE_CHARACTER);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		pUser->ChangeFame(COMMAND_CAPTAIN);
	}

	m_bCommanderSelected = true;
}

#pragma endregion

#pragma region CGameServerDlg::BattleZoneResetCommanders()

void CGameServerDlg::BattleZoneResetCommanders()
{
	m_CommanderArrayLock.lock();
	m_bCommanderSelected = false;
	foreach(itr, m_CommanderArray)
	{
		CUser* pUser = GetUserPtr(itr->second->strName, NameType::TYPE_CHARACTER);

		if (pUser == nullptr)
			continue;

		if (pUser->isKing() && pUser->GetFame() == COMMAND_CAPTAIN)
		{
			CKnights* pKnight = g_pMain->GetClanPtr(pUser->GetClanID());
			if (pKnight != nullptr)
			{
				if ((STRCASECMP(pKnight->m_strChief.c_str(), pUser->GetName().c_str()) == 0))
					pUser->ChangeFame(CHIEF);
				else if ((STRCASECMP(pKnight->m_strViceChief_1.c_str(), pUser->GetName().c_str()) == 0)
					|| (STRCASECMP(pKnight->m_strViceChief_2.c_str(), pUser->GetName().c_str()) == 0)
					|| (STRCASECMP(pKnight->m_strViceChief_3.c_str(), pUser->GetName().c_str()) == 0))
					pUser->ChangeFame(VICECHIEF);
				else
					pUser->ChangeFame(TRAINEE);
			}
			else
				pUser->ChangeFame(0);
		}
		else if (!pUser->isKing() && pUser->GetFame() == COMMAND_CAPTAIN)
			pUser->ChangeFame(CHIEF);

	}
	
	if (m_CommanderArray.empty())
	{
		m_CommanderArrayLock.unlock();
		return;
	}
	//x3x0x0
	foreach(itr, m_CommanderArray)
	{
		if (itr->second == nullptr)
			continue;

		delete itr->second;
	}

	m_CommanderArray.clear();
	m_CommanderArrayLock.unlock();
}

#pragma endregion

#pragma region CGameServerDlg::BattleWinnerResult()
void CGameServerDlg::BattleWinnerResult(BattleWinnerTypes winnertype)
{
	uint8 winner_nation = 0;
	uint8 nBattleZone = m_byBattleZone + ZONE_BATTLE_BASE;

	if (winnertype == BattleWinnerTypes::BATTLE_WINNER_NPC)
	{
		if (m_sKilledKarusNpc > m_sKilledElmoNpc)
			winner_nation = (uint8)Nation::KARUS;
		else if (m_sKilledElmoNpc > m_sKilledKarusNpc)
			winner_nation = (uint8)Nation::ELMORAD;

		if (winner_nation == 0
			&& (nBattleZone == ZONE_BATTLE
				|| nBattleZone == ZONE_BATTLE2
				|| nBattleZone == ZONE_BATTLE3))
		{
			BattleWinnerResult(BattleWinnerTypes::BATTLE_WINNER_KILL);
			return;
		}
	}
	else if (winnertype == BattleWinnerTypes::BATTLE_WINNER_MONUMENT)
	{
		if (m_sKarusMonumentPoint > m_sElmoMonumentPoint)
			winner_nation = (uint8)Nation::KARUS;
		else if (m_sElmoMonumentPoint > m_sKarusMonumentPoint)
			winner_nation = (uint8)Nation::ELMORAD;

		if (winner_nation == 0)
		{
			BattleWinnerResult(BattleWinnerTypes::BATTLE_WINNER_KILL);
			return;
		}
	}
	else if (winnertype == BattleWinnerTypes::BATTLE_WINNER_KILL)
	{
		if (m_sKarusDead > m_sElmoradDead)
			winner_nation = (uint8)Nation::ELMORAD;
		else if (m_sElmoradDead > m_sKarusDead)
			winner_nation = (uint8)Nation::KARUS;

		if (winner_nation == 0
			&& (nBattleZone == ZONE_BATTLE4
				|| nBattleZone == ZONE_BATTLE5
				|| nBattleZone == ZONE_BATTLE6))
		{
			BattleWinnerResult(BattleWinnerTypes::BATTLE_WINNER_NPC);
			return;
		}
	}

	if (winner_nation == 0) // Draw
	{
		TRACE("### WARNING : Battle is Draw Zone=%d, Karus Dead=%d, El Morad Dead=%d, Killed Karus NPC=%d, Killed Human NPC=%d, Karus Monument Point=%d, Karus Monument Point=%d ###\n", m_byBattleZone, m_sKarusDead, m_sElmoradDead, m_sKilledKarusNpc, m_sKilledElmoNpc, m_sKarusMonumentPoint, m_sKarusMonumentPoint);
		BattleZoneClose();
	}
	else
		BattleZoneResult(winner_nation);
}
#pragma endregion

#pragma region CGameServerDlg::BattleZoneOpen(int nType, uint8 bZone /*= 0*/)
void CGameServerDlg::BattleZoneOpen(int nType, uint8 bZone /*= 0*/)
{
	if ((nType == BATTLEZONE_OPEN || nType == SNOW_BATTLE) && !g_pMain->isWarOpen())
	{
		ResetBattleZone(BATTLEZONE_OPEN);
		m_byBattleOpen = nType == BATTLEZONE_OPEN ? NATION_BATTLE : SNOW_BATTLE;
		m_byOldBattleOpen = nType == BATTLEZONE_OPEN ? NATION_BATTLE : SNOW_BATTLE;
		m_byBattleZone = bZone;
		m_byBattleOpenedTime = int32(UNIXTIME);
		m_byBattleRemainingTime = m_byBattleTime;

		if (bZone + ZONE_BATTLE_BASE == ZONE_BATTLE4) {
			m_byNereidsIslandRemainingTime = m_byBattleTime;
			SendEventRemainingTime(true, nullptr, ZONE_BATTLE4);
		}
		else if (bZone + ZONE_BATTLE_BASE == ZONE_BATTLE5) {
			m_byNereidsIslandRemainingTime = m_byBattleTime;
			SendEventRemainingTime(true, nullptr, ZONE_BATTLE5);
		}
		if (m_byBattleZoneType == 0)
		{
			KickOutZoneUsers(ZONE_RONARK_LAND_BASE);
			KickOutZoneUsers(ZONE_RONARK_LAND);
			KickOutZoneUsers(ZONE_BIFROST);
			KickOutZoneUsers(ZONE_KROWAZ_DOMINION);
		}
		else if (m_byBattleZoneType == ZONE_ARDREAM)
			KickOutZoneUsers(ZONE_ARDREAM);

		if (nType == SNOW_BATTLE)
			Announcement(SNOW_BATTLEZONE_OPEN);	// Send an announcement out that the battlezone is open/closed.
		else
			Announcement(nType);	// Send an announcement out that the battlezone is open/closed.

		if (nType == BATTLEZONE_OPEN
			&& m_byBattleZoneType != ZONE_ARDREAM)
			BattleEventGiveItem(nType);
	}

	else if (nType == BATTLEZONE_CLOSE
		&& isWarOpen())
		Announcement(BATTLEZONE_CLOSE);
	else if (nType == SNOW_BATTLEZONE_CLOSE
		&& isWarOpen())
		Announcement(SNOW_BATTLEZONE_CLOSE);
	else
		return;
}
#pragma endregion

#pragma region CGameServerDlg::BattleZoneClose()
void CGameServerDlg::BattleZoneClose()
{
	if (isWarOpen())
	{
		if (m_byBattleOpen == SNOW_BATTLE)
		{
			BattleZoneOpen(SNOW_BATTLEZONE_CLOSE);
			ResetBattleZone(BATTLEZONE_NONE);
			m_byBanishFlag = true;
		}
		if (m_byBattleOpen == NATION_BATTLE)
		{
			BattleZoneResetCommanders();
			BattleZoneOpen(BATTLEZONE_CLOSE);
			ResetBattleZone(BATTLEZONE_CLOSE);
			BattleEventGiveItem(BATTLEZONE_CLOSE);
			m_byBanishFlag = true;
		}
	}
}
#pragma endregion

#pragma region CGameServerDlg::BattleZoneVictoryCheck()
void CGameServerDlg::BattleZoneVictoryCheck()
{
	if (m_bKarusFlag >= NUM_FLAG_VICTORY)
		m_bVictory = (uint8)Nation::KARUS;
	else if (m_bElmoradFlag >= NUM_FLAG_VICTORY)
		m_bVictory = (uint8)Nation::ELMORAD;
	else
		return;

	Announcement(DECLARE_WINNER);

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pTUser = g_pMain->GetUserPtr(i);
		if (pTUser == nullptr)
			continue;

		if (!pTUser->isInGame()
			|| pTUser->GetZoneID() != pTUser->GetNation()
			|| pTUser->GetNation() != m_bVictory)
			continue;

		pTUser->GoldGain(AWARD_GOLD);
		pTUser->ExpChange("battle award exp", AWARD_EXP);

		if (pTUser->GetFame() == COMMAND_CAPTAIN)
		{
			if (pTUser->isKing())
				pTUser->SendLoyaltyChange("battle victory captain", 500);
			else
				pTUser->SendLoyaltyChange("battle victory user", 300);
		}

		if (m_bVictory == (uint8)Nation::KARUS)
		{
			if (pTUser->GetFame() != COMMAND_CAPTAIN && !pTUser->isKing())
				pTUser->SendLoyaltyChange("battle victory", 100);
			else if (pTUser->GetFame() != COMMAND_CAPTAIN && pTUser->isKing())
				pTUser->SendLoyaltyChange("battle victory", 200);
		}
		else if (m_bVictory == (uint8)Nation::ELMORAD)
		{
			if (pTUser->GetFame() != COMMAND_CAPTAIN && !pTUser->isKing())
				pTUser->SendLoyaltyChange("battle victory", 100);
			else if (pTUser->GetFame() != COMMAND_CAPTAIN && pTUser->isKing())
				pTUser->SendLoyaltyChange("battle victory", 200);
		}

		// Make the winning nation use a victory emotion (yay!)
		pTUser->StateChangeServerDirect(4, 12);
	}
}
#pragma endregion

#pragma endregion