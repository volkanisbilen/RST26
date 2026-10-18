#include "stdafx.h"

#pragma region CGameServerDlg::ForgettenTempleManuelOpening()
void CGameServerDlg::ForgettenTempleManuelOpening(uint8 Type)
{
	if (pForgettenTemple.isActive) return;
	ForgettenTempleReset();
	auto& ppp = pForgettenTemple.ptimeopt;
	if (!ppp.PlayingTime || !ppp.SummonTime || !ppp.MinLevel || !ppp.MaxLevel) return;
	ForgettenTempleStart(1, ppp.MinLevel, ppp.MaxLevel);
}
#pragma endregion

#pragma region CGameServerDlg::ForgettenTempleManuelClosed()
void CGameServerDlg::ForgettenTempleManuelClosed()
{
	if (!pForgettenTemple.isActive)
		return ForgettenTempleReset();

	KickOutZoneUsers(ZONE_FORGOTTEN_TEMPLE);
	ForgettenTempleReset();
	Announcement(IDS_MONSTER_CHALLENGE_CLOSE);
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventManageRoom()
void CGameServerDlg::TempleEventManageRoom()
{
	uint8 nMaxUserCount = 0;

	switch ((EventOpCode)g_pMain->pTempleEvent.ActiveEvent)
	{
	case EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR:
#pragma region BDW Manage Room
	{
		std::queue<std::string> v_elmoradContainer, v_elmoradPriestContainer, v_karusContainer, v_karusPriestContainer;

		std::vector<std::string> m_userlist;
		m_TempleEventUserMap.m_lock.lock();
		foreach_stlmap_nolock(i, m_TempleEventUserMap)
			if (i->second) m_userlist.push_back(i->second->strUserID);
		m_TempleEventUserMap.m_lock.unlock();

		foreach(itr, m_userlist) {
			CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
			if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
				continue;

			std::string strUserID = pUser->GetName();
			if (pUser->JobGroupCheck(GROUP_WARRIOR) || pUser->JobGroupCheck(GROUP_ROGUE)
				|| pUser->JobGroupCheck(GROUP_MAGE) || pUser->JobGroupCheck(GROUP_PORTU_KURIAN))
				pUser->GetNation() == (uint8)Nation::KARUS ? v_karusContainer.push(strUserID) : v_elmoradContainer.push(strUserID);
			else if (pUser->JobGroupCheck(GROUP_CLERIC))
				pUser->GetNation() == (uint8)Nation::KARUS ? v_karusPriestContainer.push(strUserID) : v_elmoradPriestContainer.push(strUserID);
		}

		//containers for all classes. this might be turned into more generic process otherway.
		uint16 nElmoGeneCount = (uint16)v_elmoradContainer.size();
		uint16 nElmoPriestCount = (uint16)v_elmoradPriestContainer.size();

		uint16 nKarusGeneCount = (uint16)v_karusContainer.size();
		uint16 nKarusPriestCount = (uint16)v_karusPriestContainer.size();

		uint16 nElmoEventUserCount = nElmoGeneCount + nElmoPriestCount;
		uint16 nKarusEventUserCount = nKarusGeneCount + nKarusPriestCount;
		int nElmoPartyCount = (ceil(nElmoEventUserCount / 8) == 0 ? 1 : int(ceil(nElmoEventUserCount / 8)));
		int nKarusPartyCount = (ceil(nKarusEventUserCount / 8) == 0 ? 1 : int(ceil(nKarusEventUserCount / 8)));
		int nKarusPriestPerParty = (ceil(nKarusPriestCount / nKarusPartyCount) == 0 ? 1 : int(ceil(nKarusPriestCount / nKarusPartyCount)));
		int nElmoPriestPerParty = (ceil(nElmoPriestCount / nElmoPartyCount) == 0 ? 1 : int(ceil(nElmoPriestCount / nElmoPartyCount)));
		for (int bRoom = 1; bRoom <= MAX_TEMPLE_EVENT_ROOM; bRoom++) {
			auto* pRoomInfo = m_TempleEventBDWRoomList.GetData(bRoom);
			if (!pRoomInfo) continue;

			pRoomInfo->m_ElmoradUserListLock.lock();
			for (int x = 0; x < nElmoPriestPerParty; x++) {
				if (v_elmoradPriestContainer.empty() || pRoomInfo->m_ElmoradUserList.size() >= 8)
					break;

				std::string strUserID = v_elmoradPriestContainer.front();
				v_elmoradPriestContainer.pop();
				CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
					continue;

				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_ElmoradUserList.insert(make_pair(pEventUser.strUserID, pEventUser));
			}

			for (int x = (uint16)pRoomInfo->m_ElmoradUserList.size(); x < 8; x++) {
				if (v_elmoradContainer.empty())
					break;

				std::string strUserID = v_elmoradContainer.front();
				v_elmoradContainer.pop();

				CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
					continue;

				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_ElmoradUserList.insert(std::make_pair(pEventUser.strUserID, pEventUser));
			}
			pRoomInfo->m_ElmoradUserListLock.unlock();

			pRoomInfo->m_KarusUserListLock.lock();
			for (int x = 0; x < nKarusPriestPerParty; x++) {
				if (v_karusPriestContainer.empty() || pRoomInfo->m_KarusUserList.size() >= 8)
					break;

				std::string strUserID = v_karusPriestContainer.front();
				v_karusPriestContainer.pop();
				CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser()) continue;

				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_KarusUserList.insert(std::make_pair(pEventUser.strUserID, pEventUser));
			}

			for (int x = (uint16)pRoomInfo->m_KarusUserList.size(); x < 8; x++) {
				if (v_karusContainer.empty())
					break;

				std::string strUserID = v_karusContainer.front();
				v_karusContainer.pop();

				CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
					continue;

				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_KarusUserList.insert(std::make_pair(pEventUser.strUserID, pEventUser));
			}
			pRoomInfo->m_KarusUserListLock.unlock();
		}
	}
#pragma endregion
	break;
	case EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
#pragma region Juraid Manage Room
	{
		std::queue<std::string> v_elmoradContainer, v_elmoradPriestContainer, v_karusContainer, v_karusPriestContainer;

		std::vector<std::string> m_userlist;
		m_TempleEventUserMap.m_lock.lock();
		foreach_stlmap_nolock(i, m_TempleEventUserMap)
			if (i->second) m_userlist.push_back(i->second->strUserID);
		m_TempleEventUserMap.m_lock.unlock();

		foreach(itr, m_userlist) {

			CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
			if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser()) continue;
			std::string strUserID = pUser->GetName();
			if (pUser->JobGroupCheck(GROUP_WARRIOR) || pUser->JobGroupCheck(GROUP_ROGUE)
				|| pUser->JobGroupCheck(GROUP_MAGE) || pUser->JobGroupCheck(GROUP_PORTU_KURIAN))
				pUser->GetNation() == (uint8)Nation::KARUS ? v_karusContainer.push(strUserID) : v_elmoradContainer.push(strUserID);
			else if (pUser->JobGroupCheck(GROUP_CLERIC))
				pUser->GetNation() == (uint8)Nation::KARUS ? v_karusPriestContainer.push(strUserID) : v_elmoradPriestContainer.push(strUserID);
		}

		//containers for all classes. this might be turned into more generic process otherway.
		uint16 nElmoGeneCount = (uint16)v_elmoradContainer.size();
		uint16 nElmoPriestCount = (uint16)v_elmoradPriestContainer.size();

		uint16 nKarusGeneCount = (uint16)v_karusContainer.size();
		uint16 nKarusPriestCount = (uint16)v_karusPriestContainer.size();

		uint16 nElmoEventUserCount = nElmoGeneCount + nElmoPriestCount;
		uint16 nKarusEventUserCount = nKarusGeneCount + nKarusPriestCount;
		int nElmoPartyCount = (ceil(nElmoEventUserCount / 8) == 0 ? 1 : int(ceil(nElmoEventUserCount / 8)));
		int nKarusPartyCount = (ceil(nKarusEventUserCount / 8) == 0 ? 1 : int(ceil(nKarusEventUserCount / 8)));
		int nKarusPriestPerParty = (ceil(nKarusPriestCount / nKarusPartyCount) == 0 ? 1 : int(ceil(nKarusPriestCount / nKarusPartyCount)));
		int nElmoPriestPerParty = (ceil(nElmoPriestCount / nElmoPartyCount) == 0 ? 1 : int(ceil(nElmoPriestCount / nElmoPartyCount)));

		for (int bRoom = 1; bRoom <= MAX_TEMPLE_EVENT_ROOM; bRoom++) {
			auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(bRoom);
			if (!pRoomInfo)
				continue;

			pRoomInfo->m_ElmoradUserListLock.lock();
			for (int x = 0; x < nElmoPriestPerParty; x++) {
				if (v_elmoradPriestContainer.empty() || pRoomInfo->m_ElmoradUserList.size() >= 8) 
					break;
				
				std::string strUserID = v_elmoradPriestContainer.front();
				v_elmoradPriestContainer.pop();
				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_ElmoradUserList.insert(std::make_pair(pEventUser.strUserID, pEventUser));
			}

			for (int x = (uint16)pRoomInfo->m_ElmoradUserList.size(); x < 8; x++) {
				if (v_elmoradContainer.empty())
					break;
				
				std::string strUserID = v_elmoradContainer.front();
				v_elmoradContainer.pop();
				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_ElmoradUserList.insert(std::make_pair(pEventUser.strUserID, pEventUser));
			}
			pRoomInfo->m_ElmoradUserListLock.unlock();

			pRoomInfo->m_KarusUserListLock.lock();
			for (int x = 0; x < nKarusPriestPerParty; x++) {
				if (v_karusPriestContainer.empty() || pRoomInfo->m_KarusUserList.size() >= 8) 
					break;

				std::string strUserID = v_karusPriestContainer.front();
				v_karusPriestContainer.pop();
				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_KarusUserList.insert(std::make_pair(pEventUser.strUserID, pEventUser));
			}

			for (int x = (uint16)pRoomInfo->m_KarusUserList.size(); x < 8; x++) {
				if (v_karusContainer.empty())
					break;
				
				std::string strUserID = v_karusContainer.front();
				v_karusContainer.pop();
				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_KarusUserList.insert(std::make_pair(pEventUser.strUserID, pEventUser));
			}
			pRoomInfo->m_KarusUserListLock.unlock();
		}
	}
#pragma endregion
	break;
	case EventOpCode::TEMPLE_EVENT_CHAOS:
#pragma region Chaos Manage Room
	{
		nMaxUserCount = 18; // max 18 users
		std::queue<std::string> v_userContainer;// the continer for all users

		m_TempleEventUserMap.m_lock.lock();
		foreach_stlmap_nolock(itr, m_TempleEventUserMap) {
			auto* pEventUser = itr->second;
			if (pEventUser == nullptr) continue;
			std::string strUserID = pEventUser->strUserID;
			v_userContainer.push(strUserID);
		}
		m_TempleEventUserMap.m_lock.unlock();

		for (int bRoom = 1; bRoom <= MAX_TEMPLE_EVENT_ROOM; bRoom++) {
			auto* pRoomInfo = g_pMain->m_TempleEventChaosRoomList.GetData(bRoom);
			if (!pRoomInfo)
				continue;

			int counter = 0;
			pRoomInfo->UserListLock.lock();
			for (int x = 0; x < nMaxUserCount; x++) {
				if (v_userContainer.empty()) break;
				std::string strUserID = v_userContainer.front();
				v_userContainer.pop();
				_TEMPLE_STARTED_EVENT_USER pEventUser;
				pEventUser.strUserID = strUserID;
				pEventUser.isPrizeGiven = false;
				pEventUser.isLoqOut = false;
				pRoomInfo->m_UserList.insert(std::make_pair(pEventUser.strUserID, pEventUser));
			}
			pRoomInfo->UserListLock.unlock();
		}
	}
#pragma endregion
	break;
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventSendAccept()
void CGameServerDlg::TempleEventSendAccept()
{
	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT_COUNTER));
	result << uint16(pTempleEvent.ActiveEvent) << (uint16)pTempleEvent.AllUserCount << uint16(0);

	std::vector<std::string> m_userlist;
	m_TempleEventUserMap.m_lock.lock();
	foreach_stlmap_nolock(i, m_TempleEventUserMap)
		if (i->second) m_userlist.push_back(i->second->strUserID);
	m_TempleEventUserMap.m_lock.unlock();

	foreach(itr, m_userlist) {
		CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		pUser->Send(&result);
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventBridgeCheck()
void CGameServerDlg::TempleEventBridgeCheck(uint8 DoorNumber)
{
	if (!isJuraidMountainActive() || DoorNumber > 2)
		return;

	for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
		auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(i);
		if (!pRoomInfo || pRoomInfo->m_bFinished)
			continue;

		if (pRoomInfo->pkBridges[DoorNumber] && !pRoomInfo->m_sKarusBridges[DoorNumber]) {

			auto* pNpc = GetNpcPtr(pRoomInfo->pkBridges[DoorNumber], ZONE_JURAID_MOUNTAIN);
			if (pNpc && pNpc->m_byGateOpen != 2) {
				pNpc->m_byGateOpen = 2;
				pRoomInfo->m_sKarusBridges[DoorNumber] = true;

				Packet result;
				result.clear();
				result.Initialize(WIZ_NPC_INOUT);
				result << uint8(InOutType::INOUT_OUT) << pNpc->GetID();
				g_pMain->Send_All(&result, nullptr, (uint8)Nation::KARUS, ZONE_JURAID_MOUNTAIN, true, pNpc->GetEventRoom());
				result.clear();
				result.Initialize(WIZ_NPC_INOUT);
				result << uint8(InOutType::INOUT_IN) << pNpc->GetID();
				pNpc->GetNpcInfo(result);
				g_pMain->Send_All(&result, nullptr, (uint8)Nation::KARUS, ZONE_JURAID_MOUNTAIN, true, pNpc->GetEventRoom());
			}
		}

		if (pRoomInfo->peBridges[DoorNumber] && !pRoomInfo->m_sElmoBridges[DoorNumber]) {

			auto* pNpc = GetNpcPtr(pRoomInfo->peBridges[DoorNumber], ZONE_JURAID_MOUNTAIN);
			if (pNpc && pNpc->m_byGateOpen != 2) {
				pNpc->m_byGateOpen = 2;
				pRoomInfo->m_sElmoBridges[DoorNumber] = true;
				Packet result;
				result.clear();
				result.Initialize(WIZ_NPC_INOUT);
				result << uint8(InOutType::INOUT_OUT) << pNpc->GetID();
				g_pMain->Send_All(&result, nullptr, (uint8)Nation::ELMORAD, ZONE_JURAID_MOUNTAIN, true, pNpc->GetEventRoom());
				result.clear();
				result.Initialize(WIZ_NPC_INOUT);
				result << uint8(InOutType::INOUT_IN) << pNpc->GetID();
				pNpc->GetNpcInfo(result);
				g_pMain->Send_All(&result, nullptr, (uint8)Nation::ELMORAD, ZONE_JURAID_MOUNTAIN, true, pNpc->GetEventRoom());
			}
		}
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventRoomClose()
void CGameServerDlg::TempleEventRoomClose()
{
	switch ((EventOpCode)pTempleEvent.ActiveEvent)
	{
	case EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventBDWRoomList.GetData(i);
			if (!pRoomInfo || pRoomInfo->m_bFinished || !pRoomInfo->m_FinishPacketControl)
				continue;

			if (pRoomInfo->m_tFinishTimeCounter > 0 && pRoomInfo->m_tFinishTimeCounter <= UNIXTIME) {
				TempleEventFinish(i);
				pRoomInfo->m_bFinished = true;
			}
		}
	}
	break;
	case EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventJuraidRoomList.GetData(i);
			if (!pRoomInfo || pRoomInfo->m_bFinished || !pRoomInfo->m_FinishPacketControl)
				continue;

			if (pRoomInfo->m_tFinishTimeCounter > 0 && pRoomInfo->m_tFinishTimeCounter <= UNIXTIME) {
				TempleEventFinish(i);
				pRoomInfo->m_bFinished = true;
			}
		}
	}
	break;
	case EventOpCode::TEMPLE_EVENT_CHAOS:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventChaosRoomList.GetData(i);
			if (!pRoomInfo || pRoomInfo->m_bFinished || !pRoomInfo->m_FinishPacketControl)
				continue;

			if (pRoomInfo->m_tFinishTimeCounter > 0 && pRoomInfo->m_tFinishTimeCounter <= UNIXTIME) {
				TempleEventFinish(i);
				pRoomInfo->m_bFinished = true;
			}
		}
	}
	break;
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventSendWinnerScreen()
void CGameServerDlg::TempleEventSendWinnerScreen()
{
	switch ((EventOpCode)pTempleEvent.ActiveEvent)
	{
	case EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventBDWRoomList.GetData(i);
			if (!pRoomInfo || pRoomInfo->m_bFinished || pRoomInfo->m_FinishPacketControl)
				continue;

			pRoomInfo->m_FinishPacketControl = true;
			pRoomInfo->m_tFinishTimeCounter = UNIXTIME + 20;
			pRoomInfo->m_tAltarSpawn = false;
			pRoomInfo->m_tAltarSpawnTimed = 0;
			pRoomInfo->m_bWinnerNation = 0;

			if (pRoomInfo->m_KarusScoreBoard > pRoomInfo->m_ElmoScoreBoard)
				pRoomInfo->m_bWinnerNation = (uint8)Nation::KARUS;
			else if (pRoomInfo->m_ElmoScoreBoard > pRoomInfo->m_KarusScoreBoard)
				pRoomInfo->m_bWinnerNation = (uint8)Nation::ELMORAD;

			Packet newpkt1(WIZ_SELECT_MSG);
			newpkt1 << uint16(0) << uint8(7) << uint64(0);
			newpkt1 << uint32(8) << uint8(7) << uint32(500);
			Packet newpkt2(WIZ_EVENT);
			newpkt2 << uint8(TEMPLE_EVENT_FINISH)
				<< uint8(2) << uint8(0)
				<< uint8(pRoomInfo->m_bWinnerNation) << uint32(20);

			pRoomInfo->m_KarusUserListLock.lock();
			_BDW_ROOM_INFO::KarusUserList copykuser = pRoomInfo->m_KarusUserList;
			pRoomInfo->m_KarusUserListLock.unlock();
			foreach(itrUser, copykuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
					continue;

				pUser->Send(&newpkt1);
				pUser->Send(&newpkt2);
			}

			pRoomInfo->m_ElmoradUserListLock.lock();
			_BDW_ROOM_INFO::ElmoradUserList copyeuser = pRoomInfo->m_ElmoradUserList;
			pRoomInfo->m_ElmoradUserListLock.unlock();
			foreach(itrUser, copyeuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
					continue;

				pUser->Send(&newpkt1);
			}
		}
	}
	break;
	case EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventJuraidRoomList.GetData(i);
			if (!pRoomInfo || pRoomInfo->m_bFinished || pRoomInfo->m_FinishPacketControl)
				continue;

			pRoomInfo->m_FinishPacketControl = true;
			pRoomInfo->m_tFinishTimeCounter = UNIXTIME + 20;

			if (pRoomInfo->m_iElmoradKillCount > pRoomInfo->m_iKarusKillCount)
				pRoomInfo->m_bWinnerNation = (uint8)Nation::ELMORAD;
			else if (pRoomInfo->m_iKarusKillCount > pRoomInfo->m_iElmoradKillCount)
				pRoomInfo->m_bWinnerNation = (uint8)Nation::KARUS;
			else
				pRoomInfo->m_bWinnerNation = 0;

			Packet newpkt1(WIZ_SELECT_MSG);
			newpkt1 << uint16(0) << uint8(7) << uint64(0);
			newpkt1 << uint32(6) << uint8(11) << uint32(500);
			Packet newpkt2(WIZ_EVENT);
			newpkt2 << uint8(TEMPLE_EVENT_FINISH)
				<< uint8(2) << uint8(0)
				<< pRoomInfo->m_bWinnerNation << uint32(20);

			pRoomInfo->m_KarusUserListLock.lock();
			_JURAID_ROOM_INFO::KarusUserList copykuser = pRoomInfo->m_KarusUserList;
			pRoomInfo->m_KarusUserListLock.unlock();
			foreach(itrUser, copykuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_JURAID_MOUNTAIN)
					continue;

				pUser->Send(&newpkt1);
				pUser->Send(&newpkt2);
			}

			pRoomInfo->m_ElmoradUserListLock.lock();
			_JURAID_ROOM_INFO::ElmoradUserList copyeuser = pRoomInfo->m_ElmoradUserList;
			pRoomInfo->m_ElmoradUserListLock.unlock();
			foreach(itrUser, copyeuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_JURAID_MOUNTAIN)
					continue;

				pUser->Send(&newpkt1);
			}
		}
	}
	break;
	case EventOpCode::TEMPLE_EVENT_CHAOS:
	{
		Packet newpkt1(WIZ_SELECT_MSG);
		newpkt1 << uint16(0) << uint8(7) << uint64(0);
		newpkt1 << uint32(9) << uint8(24) << uint32(500);
		Packet newpkt2(WIZ_EVENT);
		newpkt2 << uint8(TEMPLE_EVENT_FINISH)
			<< uint8(2) << uint8(0)
			<< uint8(0) << uint32(20);

		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventChaosRoomList.GetData(i);
			if (!pRoomInfo || pRoomInfo->m_bFinished || pRoomInfo->m_FinishPacketControl)
				continue;

			pRoomInfo->m_FinishPacketControl = true;
			pRoomInfo->m_tFinishTimeCounter = UNIXTIME + 20;

			pRoomInfo->UserListLock.lock();
			_CHAOS_ROOM_INFO::UserList copyuser = pRoomInfo->m_UserList;
			pRoomInfo->UserListLock.unlock();

			foreach(itrUser, copyuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_CHAOS_DUNGEON)
					continue;

				pUser->StateChangeServerDirect(7, 0);
				pUser->Send(&newpkt1);
				pUser->Send(&newpkt2);
			}
		}
	}
	break;
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventStart()
void CGameServerDlg::TempleEventStart()
{
	int16 act = pTempleEvent.ActiveEvent;
	if (act != (int16)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR
		&& act != (int16)EventOpCode::TEMPLE_EVENT_CHAOS
		&& act != (int16)EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN)
		return;

	uint16 remtime = 0;
	if (pTempleEvent.SignRemainSeconds > UNIXTIME)
		remtime = uint16(pTempleEvent.SignRemainSeconds - UNIXTIME);

	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT));
	result << pTempleEvent.ActiveEvent << remtime;

	for (int i = 0; i < MAX_USER; i++){
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame() || pUser->isInTempleEventZone() || pUser->isInMonsterStoneZone())
			continue;

		if (pUser->GetZoneID() == ZONE_PRISON)
			continue;

		/*if (act == (int16)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR && pUser->GetLevel() > 80)
			continue;*/

		pUser->Send(&result);
	}

}
#pragma endregion
#pragma region CUser::EventPartyCreate()
void CUser::EventPartyCreate()
{
	m_sUserPartyType = 0;
	m_bPartyLeader = true;
	m_bPartyCommandLeader = true;
	StateChangeServerDirect(6, 1);
}
#pragma endregion

#pragma region CUser::EventPartyInvitationCheck(uint16 EnterPartyGetID)
void CUser::EventPartyInvitationCheck(uint16 EnterPartyGetID)
{
	Packet party(WIZ_PARTY);
	int16 PartyMemberCount = 0;

	CUser* pUser = g_pMain->GetUserPtr(EnterPartyGetID);
	if (pUser == nullptr || pUser == this || pUser->isInParty() || !pUser->isInGame()
		|| GetNation() != pUser->GetNation())
		return;

	auto* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr) return;

	for (PartyMemberCount = 0; PartyMemberCount < MAX_PARTY_USERS; PartyMemberCount++) {
		if (pParty->uid[PartyMemberCount] < 0)
			break;
	}

	if (PartyMemberCount == MAX_PARTY_USERS) return;

	pUser->m_sPartyIndex = m_sPartyIndex;
	pUser->m_bInParty = true;
	pUser->m_sUserPartyType = 0;
	pUser->EventPartyInvitation();
}
#pragma endregion

#pragma region CUser::EventPartyInvitation()
void CUser::EventPartyInvitation()
{
	Packet result;
	uint8 byIndex = 0xFF;
	int leader_id = -1;

	auto* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
	{
		m_bInParty = false;
		m_bInEnterParty = false;
		m_sUserPartyType = 0;
		m_sPartyIndex = -1;
		return;
	}

	CUser* pLeader = g_pMain->GetUserPtr(pParty->uid[0]);
	if (pLeader == nullptr || !pLeader->isInGame())
		return;

	//bak
	if (pLeader->GetZoneID() != GetZoneID()
		|| GetPartyMemberAmount(pParty) == MAX_PARTY_USERS)
	{
		DoNotAcceptJoiningTheParty();
		return;
	}

	// make sure user isn't already in the array...
	// kind of slow, but it works for the moment
	foreach_array(i, pParty->uid)
	{
		if (pParty->uid[i] == GetSocketID())
		{
			m_bInParty = false;
			m_bInEnterParty = false;
			m_sUserPartyType = 0;
			m_sPartyIndex = -1;
			pParty->uid[i] = -1;
			return;
		}
	}

	int PartyNemberCount = 0;
	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] >= 0)
			PartyNemberCount++;
	}

	if (PartyNemberCount == 1)
		pLeader->m_bInEnterParty = true;

	// Send the player who just joined the existing party list
	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		// No player set?
		if (pParty->uid[i] < 0)
		{
			// If we're not in the list yet, add us.
			if (byIndex == 0xFF) {
				pParty->uid[i] = GetSocketID();
				byIndex = i;
			}
			continue;
		}

		// For everyone else, 
		CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		result.clear();
		result.Initialize(WIZ_PARTY);
		result << uint8(PARTY_INSERT) << pParty->uid[i]
			<< uint8(1) // success
			<< pUser->GetName()
			<< pUser->m_MaxHp << pUser->m_sHp
			<< pUser->GetLevel() << pUser->m_sClass
			<< pUser->m_MaxMp << pUser->m_sMp
			<< pUser->GetNation()
			<< pUser->m_sUserPartyType;
		Send(&result);
	}

	if (pLeader->m_bNeedParty == 2 || m_bNeedParty == 2)
	{
		if (pLeader->m_bNeedParty == 2)
		{
			g_pMain->m_SeekingPartyArrayLock.lock();
			// You don't need anymore seek
			foreach(itr, g_pMain->m_SeekingPartyArray) {
				if ((*itr) == nullptr) continue;
				if ((*itr)->m_strUserID == pLeader->GetName()) {
					g_pMain->m_SeekingPartyArray.erase(itr);
					break;
				}
			}
			g_pMain->m_SeekingPartyArrayLock.unlock();
		}
		if (m_bNeedParty == 2)
		{
			g_pMain->m_SeekingPartyArrayLock.lock();
			// You don't need anymore seek
			foreach(itr, g_pMain->m_SeekingPartyArray) {
				if ((*itr) == nullptr) continue;
				if ((*itr)->m_strUserID == GetName()) {
					g_pMain->m_SeekingPartyArray.erase(itr);
					break;
				}
			}
			g_pMain->m_SeekingPartyArrayLock.unlock();
		}
	}

	m_bInEnterParty = true;
	pLeader->m_bInEnterParty = true;

	if (pLeader->m_bNeedParty == 2 && pLeader->isInParty())
		pLeader->StateChangeServerDirect(2, 1);

	if (m_bNeedParty == 2 && isInParty())
		StateChangeServerDirect(2, 1);

	result.clear();
	result.Initialize(WIZ_PARTY);
	result << uint8(PARTY_INSERT) << GetSocketID()
		<< uint8(1)
		<< GetName()
		<< m_MaxHp << m_sHp
		<< GetLevel() << GetClass()
		<< m_MaxMp << m_sMp
		<< GetNation()
		<< m_sUserPartyType;
	g_pMain->Send_PartyMember(GetPartyID(), &result);
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventCreateParties()
void CGameServerDlg::TempleEventCreateParties()
{
	switch ((EventOpCode)pTempleEvent.ActiveEvent)
	{
	case EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventBDWRoomList.GetData(i);
			if (!pRoomInfo) 
				continue;

			CUser* pKarusLeader = nullptr;

			pRoomInfo->m_KarusUserListLock.lock();
			_BDW_ROOM_INFO::KarusUserList copykuser = pRoomInfo->m_KarusUserList;
			pRoomInfo->m_KarusUserListLock.unlock();
			foreach(itrUser, copykuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame()
					|| !pUser->isEventUser() || pUser->isInParty()
					|| pUser->GetZoneID() != pTempleEvent.ZoneID)
					continue;

				if (pRoomInfo->GetRoomKarusUserCount() > 1) {
					if (pKarusLeader == nullptr) {
						auto* pParty = g_pMain->CreateParty(pUser);
						if (pParty == nullptr) break;
						pKarusLeader = pUser;
						pKarusLeader->EventPartyCreate();
					}
					else pKarusLeader->EventPartyInvitationCheck(pUser->GetID());
				}
			}

			CUser* pElmoLeader = nullptr;

			pRoomInfo->m_ElmoradUserListLock.lock();
			_BDW_ROOM_INFO::ElmoradUserList copyeuser = pRoomInfo->m_ElmoradUserList;
			pRoomInfo->m_ElmoradUserListLock.unlock();
			foreach(itrUser, copyeuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->isInParty())
					continue;

				if (pRoomInfo->GetRoomElmoradUserCount() > 1) {
					if (pElmoLeader == nullptr) {
						auto* pParty2 = g_pMain->CreateParty(pUser);
						if (pParty2 == nullptr)  break;
						pElmoLeader = pUser;
						pUser->EventPartyCreate();
					}
					else pElmoLeader->EventPartyInvitationCheck(pUser->GetSocketID());
				}
			}
		}
	}
	break;
	case  EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventJuraidRoomList.GetData(i);
			if (!pRoomInfo)
				continue;

			CUser* pKarusLeader = nullptr;

			pRoomInfo->m_KarusUserListLock.lock();
			_JURAID_ROOM_INFO::KarusUserList copykuser = pRoomInfo->m_KarusUserList;
			pRoomInfo->m_KarusUserListLock.unlock();
			foreach(itrUser, copykuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->isInParty())
					continue;

				if (pRoomInfo->GetRoomKarusUserCount() > 1) {
					if (pKarusLeader == nullptr) {
						auto* pParty = g_pMain->CreateParty(pUser);
						if (pParty == nullptr)
							break;

						pKarusLeader = pUser;
						pKarusLeader->EventPartyCreate();
					}
					else {
						pKarusLeader->EventPartyInvitationCheck(pUser->GetSocketID());
					}
				}
			}

			CUser* pElmoLeader = nullptr;

			pRoomInfo->m_ElmoradUserListLock.lock();
			_JURAID_ROOM_INFO::ElmoradUserList copyeuser = pRoomInfo->m_ElmoradUserList;
			pRoomInfo->m_ElmoradUserListLock.unlock();
			foreach(itrUser, copyeuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->isInParty())
					continue;

				if (pRoomInfo->GetRoomElmoradUserCount() > 1) {
					if (pElmoLeader == nullptr) {
						auto* pParty = g_pMain->CreateParty(pUser);
						if (pParty == nullptr) break;
						pElmoLeader = pUser;
						pElmoLeader->EventPartyCreate();
					}
					else pElmoLeader->EventPartyInvitationCheck(pUser->GetSocketID());
				}
			}
		}
	}
	break;
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventTeleportUsers()
void CGameServerDlg::TempleEventTeleportUsers()
{
	uint32 time = 0;
	Packet newpkt1(WIZ_SELECT_MSG);
	newpkt1 << uint16(0) << uint8(7) << uint64(0);
	if (pTempleEvent.ActiveEvent == (int16)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR) {
		time = pEventTimeOpt.pvroomop[0].play * MINUTE;
		newpkt1 << uint32(8) << uint8(7) << time;
	}
	else if (pTempleEvent.ActiveEvent == (int16)EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN) {
		time = pEventTimeOpt.pvroomop[2].play * MINUTE;
		newpkt1 << uint32(6) << uint8(11) << time;
	}
	else if (pTempleEvent.ActiveEvent == (int16)EventOpCode::TEMPLE_EVENT_CHAOS) {
		time = pEventTimeOpt.pvroomop[1].play * MINUTE;
		newpkt1 << uint32(9) << uint8(24) << time;
	}

	Packet newpkt2(WIZ_BIFROST);
	newpkt2 << uint8(5) << uint16(time);

	switch ((EventOpCode)pTempleEvent.ActiveEvent)
	{
	case EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR:
	{
		Packet newpkt3(WIZ_EVENT, uint8(1));
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventBDWRoomList.GetData(i);
			if (!pRoomInfo)
				continue;

			pRoomInfo->m_KarusUserListLock.lock();
			_BDW_ROOM_INFO::KarusUserList copykuser = pRoomInfo->m_KarusUserList;
			pRoomInfo->m_KarusUserListLock.unlock();

			foreach(itrUser, copykuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
					continue;

				if (!pUser->ZoneChange(pTempleEvent.ZoneID, 0.0f, 0.0f, i))
					continue;

				pUser->m_isFinalJoinedEvent = true;
				pUser->BorderDefenceAddPlayerRank();
				pUser->Send(&newpkt1);
				pUser->Send(&newpkt2);
				pUser->Send(&newpkt3);
			}

			pRoomInfo->m_ElmoradUserListLock.lock();
			_BDW_ROOM_INFO::ElmoradUserList copyeuser = pRoomInfo->m_ElmoradUserList;
			pRoomInfo->m_ElmoradUserListLock.unlock();
			foreach(itrUser, copyeuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
					continue;

				if (!pUser->ZoneChange(pTempleEvent.ZoneID, 0.0f, 0.0f, i))
					continue;

				pUser->m_isFinalJoinedEvent = true;
				pUser->BorderDefenceAddPlayerRank();
				pUser->Send(&newpkt1);
				pUser->Send(&newpkt2);
				pUser->Send(&newpkt3);
			}

			pRoomInfo->m_bActive = true;
		}


		std::vector<std::string> m_userlist;
		m_TempleEventUserMap.m_lock.lock();
		foreach_stlmap_nolock(i, m_TempleEventUserMap)
			if (i->second) m_userlist.push_back(i->second->strUserID);
		m_TempleEventUserMap.m_lock.unlock();

		foreach(itr, m_userlist) {
			CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
			if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->m_isFinalJoinedEvent)
				continue;

			pUser->m_isFinalJoinedEvent = false;
			pUser->m_sJoinedEvent = -1;
		}
	}
	break;
	case EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventJuraidRoomList.GetData(i);
			if (!pRoomInfo)
				continue;

			uint8 h_size = pRoomInfo->GetRoomElmoradUserCount();
			uint8 k_size = pRoomInfo->GetRoomKarusUserCount();

			if (!h_size && !k_size)
				continue;

			pRoomInfo->m_bActive = true;

			pRoomInfo->m_KarusUserListLock.lock();
			_JURAID_ROOM_INFO::KarusUserList copykuser = pRoomInfo->m_KarusUserList;
			pRoomInfo->m_KarusUserListLock.unlock();
			foreach(itrUser, copykuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
					continue;

				if (!pUser->ZoneChange(pTempleEvent.ZoneID, 0.0f, 0.0f, i))
					continue;

				pUser->m_isFinalJoinedEvent = true;
				pUser->Send(&newpkt1);
				pUser->Send(&newpkt2);
			}

			pRoomInfo->m_ElmoradUserListLock.lock();
			_JURAID_ROOM_INFO::ElmoradUserList copyeuser = pRoomInfo->m_ElmoradUserList;
			pRoomInfo->m_ElmoradUserListLock.unlock();
			foreach(itrUser, copyeuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
					continue;

				if (!pUser->ZoneChange(pTempleEvent.ZoneID, 0.0f, 0.0f, i))
					continue;

				pUser->m_isFinalJoinedEvent = true;
				pUser->Send(&newpkt1);
				pUser->Send(&newpkt2);
			}
		}

		std::vector<std::string> m_userlist;
		m_TempleEventUserMap.m_lock.lock();
		foreach_stlmap_nolock(i, m_TempleEventUserMap)
			if (i->second) m_userlist.push_back(i->second->strUserID);
		m_TempleEventUserMap.m_lock.unlock();

		foreach(itr, m_userlist) {
			CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
			if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->m_isFinalJoinedEvent)
				continue;

			pUser->m_isFinalJoinedEvent = false;
			pUser->m_sJoinedEvent = -1;
		}
	}
	break;
	case EventOpCode::TEMPLE_EVENT_CHAOS:
	{
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventChaosRoomList.GetData(i);
			if (!pRoomInfo)
				continue;

			if (pRoomInfo->GetRoomTotalUserCount() == 0)
				continue;

			pRoomInfo->m_bActive = true;

			pRoomInfo->UserListLock.lock();
			_CHAOS_ROOM_INFO::UserList copyuser = pRoomInfo->m_UserList;
			pRoomInfo->UserListLock.unlock();

			foreach(itrUser, copyuser) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
					continue;

				CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser())
					continue;

				if (!pUser->ZoneChange(pTempleEvent.ZoneID, 0.0f, 0.0f, i))
					continue;

				pUser->RobItem(CHAOS_MAP, 1);

				pUser->m_isFinalJoinedEvent = true;
				pUser->ChaosExpansionAddPlayerRank();
				pUser->Send(&newpkt1);
				pUser->Send(&newpkt2);
			}
		}

		std::vector<std::string> m_userlist;
		m_TempleEventUserMap.m_lock.lock();
		foreach_stlmap_nolock(i, m_TempleEventUserMap)
			if (i->second) m_userlist.push_back(i->second->strUserID);
		m_TempleEventUserMap.m_lock.unlock();

		foreach(itr, m_userlist) {
			CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
			if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->m_isFinalJoinedEvent)
				continue;

			pUser->m_isFinalJoinedEvent = false;
			pUser->m_sJoinedEvent = -1;
		}
	}
	break;
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventFinish(int16 nRoom /* = -1 */)
void CGameServerDlg::TempleEventFinish(int16 nRoom /* = -1 */)
{
	uint16 local_id = 0;
	if (pTempleEvent.ActiveEvent == TEMPLE_EVENT_BORDER_DEFENCE_WAR)
		local_id = EventLocalID::BorderDefenceWar;
	else if (pTempleEvent.ActiveEvent == TEMPLE_EVENT_CHAOS)
		local_id = EventLocalID::ChaosExpansion;
	else if (pTempleEvent.ActiveEvent == TEMPLE_EVENT_JURAD_MOUNTAIN)
		local_id = EventLocalID::JuraidMountain;

	std::vector<_EVENT_REWARD> mreward;
	m_EventRewardArray.m_lock.lock();
	foreach_stlmap_nolock(itr, m_EventRewardArray)
		if (itr->second && local_id == itr->second->local_id && itr->second->status)
			mreward.push_back(*itr->second);
	m_EventRewardArray.m_lock.unlock();

	struct _itemlist {
		uint32 id, count, time;
		_itemlist(uint32 id, uint32 count, uint32 time) {
			this->id = id;
			this->count = count;
			this->time = time;
		}
	};

	uint32 w_cash = 0, w_exp = 0, w_loyalty = 0, w_noah = 0;
	uint32 l_cash = 0, l_exp = 0, l_loyalty = 0, l_noah = 0;
	std::vector<_itemlist> m_winner, m_loser;
	foreach(itr, mreward) {

		if (itr->iswinner) {
			if (itr->experience) w_exp += itr->experience;
			if (itr->cash) w_cash += itr->cash;
			if (itr->loyalty) w_loyalty += itr->loyalty;
			if (itr->noah) w_noah += itr->noah;
			for (int i = 0; i < 3; i++)
				if (itr->itemid[i])
					m_winner.push_back(_itemlist(itr->itemid[i], itr->itemcount[i], itr->itemexpiration[i]));
		}
		else
		{
			if (itr->experience) l_exp += itr->experience;
			if (itr->cash) l_cash += itr->cash;
			if (itr->loyalty) l_loyalty += itr->loyalty;
			if (itr->noah) l_noah += itr->noah;
			for (int i = 0; i < 3; i++)
				if (itr->itemid[i])
					m_loser.push_back(_itemlist(itr->itemid[i], itr->itemcount[i], itr->itemexpiration[i]));
		}
	}

	std::vector<CUser*> m_tpList;
	switch ((EventOpCode)pTempleEvent.ActiveEvent)
	{
	case EventOpCode::TEMPLE_EVENT_CHAOS:
#pragma region Chaos Finish
	{
		if (nRoom != -1) {
			auto* pRoomInfo = g_pMain->m_TempleEventChaosRoomList.GetData(nRoom);
			if (!pRoomInfo || pRoomInfo->m_bFinished)
				return;

			std::vector<std::string> m_UserList;
			int totalcount = pRoomInfo->GetRoomTotalUserCount() + 2;
			pRoomInfo->m_bFinished = true;

			pRoomInfo->UserListLock.lock();
			foreach(itrUser, pRoomInfo->m_UserList) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
					continue;

				pEventUser.isPrizeGiven = true;
				m_UserList.push_back(pEventUser.strUserID);
			}

			pRoomInfo->UserListLock.unlock();

			foreach(itr, m_UserList) {
				CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_CHAOS_DUNGEON)
					continue;

				int64 nChangeExp = -1, nGainedExp = int64(pow(pUser->GetLevel(), 3) * 0.15 * (5 * (int64)pUser->m_ChaosExpansionKillCount - (int64)pUser->m_ChaosExpansionDeadCount));
				int64 nPremiumGainedExp = nGainedExp * 2;

				if (nGainedExp < 0) nGainedExp = 0;
				if (nGainedExp > 8000000) nGainedExp = 8000000;
				if (nPremiumGainedExp > 10000000) nPremiumGainedExp = 10000000;

				nChangeExp = pUser->GetPremium() != 0 ? nPremiumGainedExp : nGainedExp;
				if (nChangeExp > 0) pUser->ExpChange("", nChangeExp, true);

				int32 nUserRank = pUser->GetPlayerRank(RANK_TYPE_CHAOS_DUNGEON);
				int rank = nUserRank - 1;
				if (rank >= 0 && rank < 18) {
					for (uint8 x = 0; x < 5; x++) 
						if (pChaosReward[rank].itemid[x]) 
							pUser->GiveItem("Chaos Event", pChaosReward[rank].itemid[x], pChaosReward[rank].itemcount[x], true);

					if (pChaosReward[rank].cash)
						pUser->GiveBalance(pChaosReward[rank].cash);
					if (pChaosReward[rank].experience)
						pUser->ExpChange("JR", pChaosReward[rank].experience, true);
					if (pChaosReward[rank].loyalty)
						pUser->SendLoyaltyChange("JR", pChaosReward[rank].loyalty);
					if (pChaosReward[rank].noah)
						pUser->GoldGain(pChaosReward[rank].noah, true, false);
				}

				/*if (nUserRank == 1) pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinChaos1, 0, nullptr);
				else if (nUserRank == 2) pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinChaos2, 0, nullptr);
				else if (nUserRank == 3) pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinChaos3, 0, nullptr);
				if (nUserRank == 1) pUser->pUserDailyRank.CWCounterWin++;

				if (totalcount < 5) totalcount = 5;
				int dilim = (nUserRank * 100) / totalcount;
				if (dilim <= 20) {
					pUser->GiveItem("Chaos Event", BLUE_TREASURE_CHEST);
					pUser->GiveItem("Chaos Event", VOUCHER_OF_ORACLE);
				}
				else if (dilim <= 40) { pUser->GiveItem("Chaos Event", GREEN_TREASURE_CHEST); }
				else if (dilim <= 60) { pUser->GiveItem("Chaos Event", RED_TREASURE_CHEST); }*/
				//pUser->GiveItem("Chaos Event", VOUCHER_OF_CHAOS);
				m_tpList.push_back(pUser);
			}
		}
		else {
			for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
				auto* pRoomInfo = g_pMain->m_TempleEventChaosRoomList.GetData(i);
				if (!pRoomInfo || pRoomInfo->m_bFinished)
					continue;

				std::vector<std::string> m_UserList;
				int totalcount = pRoomInfo->GetRoomTotalUserCount() + 2;
				pRoomInfo->m_bFinished = true;

				pRoomInfo->UserListLock.lock();
				foreach(itrUser, pRoomInfo->m_UserList) {
					auto& pEventUser = itrUser->second;
					if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
						continue;

					pEventUser.isPrizeGiven = true;
					m_UserList.push_back(pEventUser.strUserID);
				}
				pRoomInfo->UserListLock.unlock();

				foreach(itr, m_UserList) {
					CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
					if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_CHAOS_DUNGEON)
						continue;

					int64 nChangeExp = -1, nGainedExp = int64(pow(pUser->GetLevel(), 3) * 0.15 * (5 * (int64)pUser->m_ChaosExpansionKillCount - (int64)pUser->m_ChaosExpansionDeadCount));
					int64 nPremiumGainedExp = nGainedExp * 2;

					if (nGainedExp < 0) nGainedExp = 0;
					if (nGainedExp > 8000000) nGainedExp = 8000000;
					if (nPremiumGainedExp > 10000000) nPremiumGainedExp = 10000000;

					nChangeExp = pUser->GetPremium() != 0 ? nPremiumGainedExp : nGainedExp;
					if (nChangeExp > 0) pUser->ExpChange("",nChangeExp, true);

					int32 nUserRank = pUser->GetPlayerRank(RANK_TYPE_CHAOS_DUNGEON);
					int rank = nUserRank - 1;
					if (rank >= 0 && rank < 18) {
						for (uint8 x = 0; x < 5; x++)
							if (pChaosReward[rank].itemid[x])
								pUser->GiveItem("Chaos Event", pChaosReward[rank].itemid[x], pChaosReward[rank].itemcount[x], true);
						
						if (pChaosReward[rank].cash)
							pUser->GiveBalance(pChaosReward[rank].cash);
						if (pChaosReward[rank].experience)
							pUser->ExpChange("JR", pChaosReward[rank].experience, true);
						if (pChaosReward[rank].loyalty)
							pUser->SendLoyaltyChange("JR", pChaosReward[rank].loyalty);
						if (pChaosReward[rank].noah)
							pUser->GoldGain(pChaosReward[rank].noah, true, false);
					}

					/*if (nUserRank == 1) pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinChaos1, 0, nullptr);
					else if (nUserRank == 2) pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinChaos2, 0, nullptr);
					else if (nUserRank == 3) pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinChaos3, 0, nullptr);
					if (nUserRank == 1) pUser->pUserDailyRank.CWCounterWin++;

					if (totalcount < 5) totalcount = 5;
					int dilim = (nUserRank * 100) / totalcount;
					if (dilim <= 20) {
						pUser->GiveItem("Chaos Event", BLUE_TREASURE_CHEST);
						pUser->GiveItem("Chaos Event", VOUCHER_OF_ORACLE);
					}
					else if (dilim <= 40) { pUser->GiveItem("Chaos Event", GREEN_TREASURE_CHEST); }
					else if (dilim <= 60) { pUser->GiveItem("Chaos Event", RED_TREASURE_CHEST); }*/

					//pUser->GiveItem("Chaos Event", VOUCHER_OF_CHAOS);
					m_tpList.push_back(pUser);
				}
			}
		}
	}
#pragma endregion
	break;

	case EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR:
#pragma region BDW Finish
	{
		if (nRoom != -1) {
			auto* pRoomInfo = m_TempleEventBDWRoomList.GetData(nRoom);
			if (!pRoomInfo || pRoomInfo->m_bFinished)
				return;

			pRoomInfo->m_bFinished = true;

			std::vector< std::string> m_KarusUserList, m_ElmoradUserList;

			pRoomInfo->m_KarusUserListLock.lock();
			foreach(itrUser, pRoomInfo->m_KarusUserList) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
					continue;

				pEventUser.isPrizeGiven = true;
				m_KarusUserList.push_back(pEventUser.strUserID);
			}
			pRoomInfo->m_KarusUserListLock.unlock();

			foreach(itr, m_KarusUserList) {
				CUser* pUser = GetUserPtr(*itr, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
					continue;
				
				uint32 nChangeExp = 0;
				uint8 bLevel = pUser->GetLevel();
				
				if (bLevel < 58)
				{
					if (bLevel < 20)
						bLevel = 20;
					nChangeExp = int64(((int64)bLevel - 20) * (3000 + 100/* Temp Score */ * 2000));
				}
				else
				{
					nChangeExp = int64(((int64)bLevel + 55) * (20000 + 100/* Temp Score */ * 1000));
				}

				if (pRoomInfo->m_bWinnerNation == pUser->GetNation()) {
					foreach(i, m_winner)
						pUser->GiveItem("BDW", i->id, i->count, true, i->time);

					if (nChangeExp)
						w_exp += nChangeExp;
					if (w_cash)
						pUser->GiveBalance(w_cash);
					if (w_exp)
						pUser->ExpChange("BDW", w_exp, true);
					if (w_loyalty)
						pUser->SendLoyaltyChange("BDW", w_loyalty);
					if (w_noah)
						pUser->GoldGain(w_noah, true, false);
				}
				else {
					foreach(i, m_loser)
						pUser->GiveItem("BDW", i->id, i->count, true, i->time);

					if (nChangeExp)
						l_exp += nChangeExp;
					if (l_cash)
						pUser->GiveBalance(l_cash);
					if (l_exp)
						pUser->ExpChange("BDW", l_exp, true);
					if (l_loyalty)
						pUser->SendLoyaltyChange("BDW", l_loyalty);
					if (l_noah)
						pUser->GoldGain(l_noah, true, false);
				}
				m_tpList.push_back(pUser);
			}

			pRoomInfo->m_ElmoradUserListLock.lock();
			foreach(itrUser, pRoomInfo->m_ElmoradUserList) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
					continue;

				pEventUser.isPrizeGiven = true;
				m_ElmoradUserList.push_back(pEventUser.strUserID);
			}
			pRoomInfo->m_ElmoradUserListLock.unlock();

			foreach(itr, m_ElmoradUserList) {
				CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
					continue;

				uint32 nChangeExp = 0;
				uint8 bLevel = pUser->GetLevel();

				if (bLevel < 58)
				{
					if (bLevel < 20)
						bLevel = 20;
					nChangeExp = int64(((int64)bLevel - 20) * (3000 + 100/* Temp Score */ * 2000));
				}
				else
				{
					nChangeExp = int64(((int64)bLevel + 55) * (20000 + 100/* Temp Score */ * 1000));
				}

				if (pRoomInfo->m_bWinnerNation == pUser->GetNation()) {
					foreach(i, m_winner)
						pUser->GiveItem("BDW", i->id, i->count, true, i->time);

					if (nChangeExp)
						w_exp += nChangeExp;
					if (w_cash)
						pUser->GiveBalance(w_cash);
					if (w_exp)
						pUser->ExpChange("BDW", w_exp, true);
					if (w_loyalty)
						pUser->SendLoyaltyChange("BDW", w_loyalty);
					if (w_noah)
						pUser->GoldGain(w_noah, true, false);
				}
				else {
					foreach(i, m_loser)
						pUser->GiveItem("BDW", i->id, i->count, true, i->time);

					if (nChangeExp)
						l_exp += nChangeExp;
					if (l_cash)
						pUser->GiveBalance(l_cash);
					if (l_exp)
						pUser->ExpChange("BDW", l_exp, true);
					if (l_loyalty)
						pUser->SendLoyaltyChange("BDW", l_loyalty);
					if (l_noah)
						pUser->GoldGain(l_noah, true, false);
				}
				m_tpList.push_back(pUser);
			}
		}
		else {
			for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
				auto* pRoomInfo = m_TempleEventBDWRoomList.GetData(i);
				if (!pRoomInfo || pRoomInfo->m_bFinished)
					continue;

				pRoomInfo->m_bFinished = true;

				std::vector< std::string> m_KarusUserList, m_ElmoradUserList;

				pRoomInfo->m_KarusUserListLock.lock();
				foreach(itrUser, pRoomInfo->m_KarusUserList) {
					auto& pEventUser = itrUser->second;
					if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
						continue;

					pEventUser.isPrizeGiven = true;
					m_KarusUserList.push_back(pEventUser.strUserID);
				}
				pRoomInfo->m_KarusUserListLock.unlock();

				foreach(itr, m_KarusUserList) {
					CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
					if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
						continue;

					uint32 nChangeExp = 0;
					uint8 bLevel = pUser->GetLevel();

					if (bLevel < 58)
					{
						if (bLevel < 20)
							bLevel = 20;
						nChangeExp = int64(((int64)bLevel - 20) * (3000 + 100/* Temp Score */ * 2000));
					}
					else
					{
						nChangeExp = int64(((int64)bLevel + 55) * (20000 + 100/* Temp Score */ * 1000));
					}

					if (pRoomInfo->m_bWinnerNation == pUser->GetNation()) {
						foreach(i, m_winner)
							pUser->GiveItem("BDW", i->id, i->count, true, i->time);

						if (nChangeExp)
							w_exp += nChangeExp;
						if (w_cash)
							pUser->GiveBalance(w_cash);
						if (w_exp)
							pUser->ExpChange("BDW", w_exp, true);
						if (w_loyalty)
							pUser->SendLoyaltyChange("BDW", w_loyalty);
						if (w_noah)
							pUser->GoldGain(w_noah, true, false);
					}
					else {
						foreach(i, m_loser)
							pUser->GiveItem("BDW", i->id, i->count, true, i->time);

						if (nChangeExp)
							l_exp += nChangeExp;
						if (l_cash)
							pUser->GiveBalance(l_cash);
						if (l_exp)
							pUser->ExpChange("BDW", l_exp, true);
						if (l_loyalty)
							pUser->SendLoyaltyChange("BDW", l_loyalty);
						if (l_noah)
							pUser->GoldGain(l_noah, true, false);
					}

					m_tpList.push_back(pUser);
				}

				pRoomInfo->m_ElmoradUserListLock.lock();
				foreach(itrUser, pRoomInfo->m_ElmoradUserList) {
					auto& pEventUser = itrUser->second;
					if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
						continue;

					pEventUser.isPrizeGiven = true;
					m_ElmoradUserList.push_back(pEventUser.strUserID);
				}
				pRoomInfo->m_ElmoradUserListLock.unlock();

				foreach(itr, m_ElmoradUserList) {
					CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
					if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
						continue;

					uint32 nChangeExp = 0;
					uint8 bLevel = pUser->GetLevel();

					if (bLevel < 58)
					{
						if (bLevel < 20)
							bLevel = 20;
						nChangeExp = int64(((int64)bLevel - 20) * (3000 + 100/* Temp Score */ * 2000));
					}
					else
					{
						nChangeExp = int64(((int64)bLevel + 55) * (20000 + 100/* Temp Score */ * 1000));
					}

					if (pRoomInfo->m_bWinnerNation == pUser->GetNation()) {
						foreach(i, m_winner)
							pUser->GiveItem("BDW", i->id, i->count, true, i->time);

						if (nChangeExp)
							w_exp += nChangeExp;
						if (w_cash)
							pUser->GiveBalance(w_cash);
						if (w_exp)
							pUser->ExpChange("BDW", w_exp, true);
						if (w_loyalty)
							pUser->SendLoyaltyChange("BDW", w_loyalty);
						if (w_noah)
							pUser->GoldGain(w_noah, true, false);
					}
					else {
						foreach(i, m_loser)
							pUser->GiveItem("BDW", i->id, i->count, true, i->time);

						if (nChangeExp)
							l_exp += nChangeExp;
						if (l_cash)
							pUser->GiveBalance(l_cash);
						if (l_exp)
							pUser->ExpChange("BDW", l_exp, true);
						if (l_loyalty)
							pUser->SendLoyaltyChange("BDW", l_loyalty);
						if (l_noah)
							pUser->GoldGain(l_noah, true, false);
					}

					m_tpList.push_back(pUser);
				}
			}
		}
	}
#pragma endregion
	break;

	case EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
#pragma region Juraid Finish
	{
		if (nRoom != -1) {
			auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(nRoom);
			if (!pRoomInfo || pRoomInfo->m_bFinished)
				return;

			pRoomInfo->m_bFinished = true;

			std::vector< _TEMPLE_STARTED_EVENT_USER> mlist;

			pRoomInfo->m_KarusUserListLock.lock();
			foreach(itrUser, pRoomInfo->m_KarusUserList) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
					continue;

				pEventUser.isPrizeGiven = true;
				mlist.push_back(itrUser->second);
			}
			pRoomInfo->m_KarusUserListLock.unlock();

			foreach(itr, mlist) {
				CUser* pUser = g_pMain->GetUserPtr(itr->strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser()
					|| pUser->GetZoneID() != ZONE_JURAID_MOUNTAIN)
					continue;

				if (pRoomInfo->m_bWinnerNation == pUser->GetNation()) {
					foreach(i, m_winner)
						pUser->GiveItem("JR", i->id, i->count, true, i->time);

					if (w_cash)
						pUser->GiveBalance(w_cash);
					if (w_exp)
						pUser->ExpChange("JR", w_exp, true);
					if (w_loyalty)
						pUser->SendLoyaltyChange("JR", w_loyalty);
					if (w_noah)
						pUser->GoldGain(w_noah, true, false);

					pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinJuraid, 0, nullptr);
				}
				else {
					foreach(i, m_loser)
						pUser->GiveItem("JR", i->id, i->count, true, i->time);

					if (l_cash)
						pUser->GiveBalance(l_cash);
					if (l_exp)
						pUser->ExpChange("JR", l_exp, true);
					if (l_loyalty)
						pUser->SendLoyaltyChange("JR", l_loyalty);
					if (l_noah)
						pUser->GoldGain(l_noah, true, false);
				}

				m_tpList.push_back(pUser);
			}

			mlist.clear();

			pRoomInfo->m_ElmoradUserListLock.lock();
			foreach(itrUser, pRoomInfo->m_ElmoradUserList) {
				auto& pEventUser = itrUser->second;
				if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
					continue;

				pEventUser.isPrizeGiven = true;
				mlist.push_back(itrUser->second);
			}
			pRoomInfo->m_ElmoradUserListLock.unlock();

			foreach(itr, mlist) {
				CUser* pUser = g_pMain->GetUserPtr(itr->strUserID, NameType::TYPE_CHARACTER);
				if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_JURAID_MOUNTAIN)
					continue;

				if (pRoomInfo->m_bWinnerNation == pUser->GetNation()) {
					foreach(i, m_winner)
						pUser->GiveItem("JR", i->id, i->count, true, i->time);

					if (w_cash)
						pUser->GiveBalance(w_cash);
					if (w_exp)
						pUser->ExpChange("JR", w_exp, true);
					if (w_loyalty)
						pUser->SendLoyaltyChange("JR", w_loyalty);
					if (w_noah)
						pUser->GoldGain(w_noah, true, false);

					pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinJuraid, 0, nullptr);
				}
				else {
					foreach(i, m_loser)
						pUser->GiveItem("JR", i->id, i->count, true, i->time);

					if (l_cash)
						pUser->GiveBalance(l_cash);
					if (l_exp)
						pUser->ExpChange("JR", l_exp, true);
					if (l_loyalty)
						pUser->SendLoyaltyChange("JR", l_loyalty);
					if (l_noah)
						pUser->GoldGain(l_noah, true, false);
				}

				m_tpList.push_back(pUser);
			}
		}
		else {
			for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
				auto* pRoomInfo = m_TempleEventJuraidRoomList.GetData(i);
				if (!pRoomInfo || pRoomInfo->m_bFinished)
					continue;

				pRoomInfo->m_bFinished = true;

				std::vector< _TEMPLE_STARTED_EVENT_USER> mlist;

				pRoomInfo->m_KarusUserListLock.lock();
				foreach(itrUser, pRoomInfo->m_KarusUserList) {
					auto& pEventUser = itrUser->second;
					if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
						continue;

					pEventUser.isPrizeGiven = true;
					mlist.push_back(itrUser->second);
				}
				pRoomInfo->m_KarusUserListLock.unlock();

				foreach(itr, mlist) {
					CUser* pUser = g_pMain->GetUserPtr(itr->strUserID, NameType::TYPE_CHARACTER);
					if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_JURAID_MOUNTAIN) 
						continue;

					if (pRoomInfo->m_bWinnerNation == pUser->GetNation()) {
						foreach(i, m_winner)
							pUser->GiveItem("JR", i->id, i->count, true, i->time);

						if (w_cash)
							pUser->GiveBalance(w_cash);
						if (w_exp)
							pUser->ExpChange("JR", w_exp, true);
						if (w_loyalty)
							pUser->SendLoyaltyChange("JR", w_loyalty);
						if (w_noah)
							pUser->GoldGain(w_noah, true, false);

						pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinJuraid, 0, nullptr);
					}
					else {
						foreach(i, m_loser)
							pUser->GiveItem("JR", i->id, i->count, true, i->time);

						if (l_cash)
							pUser->GiveBalance(l_cash);
						if (l_exp)
							pUser->ExpChange("JR", l_exp, true);
						if (l_loyalty)
							pUser->SendLoyaltyChange("JR", l_loyalty);
						if (l_noah)
							pUser->GoldGain(l_noah, true, false);
					}
					m_tpList.push_back(pUser);
				}

				mlist.clear();

				pRoomInfo->m_ElmoradUserListLock.lock();
				foreach(itrUser, pRoomInfo->m_ElmoradUserList) {
					auto& pEventUser = itrUser->second;
					if (pEventUser.strUserID.empty() || pEventUser.isPrizeGiven)
						continue;

					pEventUser.isPrizeGiven = true;
					mlist.push_back(itrUser->second);
				}
				pRoomInfo->m_ElmoradUserListLock.unlock();

				foreach(itr, mlist) {
					CUser* pUser = g_pMain->GetUserPtr(itr->strUserID, NameType::TYPE_CHARACTER);
					if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_JURAID_MOUNTAIN) continue;

					if (pRoomInfo->m_bWinnerNation == pUser->GetNation()) {
						foreach(i, m_winner)
							pUser->GiveItem("JR", i->id, i->count, true, i->time);

						if (w_cash)
							pUser->GiveBalance(w_cash);
						if (w_exp)
							pUser->ExpChange("JR", w_exp, true);
						if (w_loyalty)
							pUser->SendLoyaltyChange("JR", w_loyalty);
						if (w_noah)
							pUser->GoldGain(w_noah, true, false);

						pUser->AchieveWarCountAdd(UserAchieveWarTypes::AchieveWinJuraid, 0, nullptr);
					}
					else {
						foreach(i, m_loser)
							pUser->GiveItem("JR", i->id, i->count, true, i->time);

						if (l_cash)
							pUser->GiveBalance(l_cash);
						if (l_exp)
							pUser->ExpChange("JR", l_exp, true);
						if (l_loyalty)
							pUser->SendLoyaltyChange("JR", l_loyalty);
						if (l_noah)
							pUser->GoldGain(l_noah, true, false);
					}
					m_tpList.push_back(pUser);
				}
			}
		}
	}
#pragma endregion
	break;
	}

	foreach(usr, m_tpList) {
		if (*usr == nullptr || !(*usr)->isInGame())
			continue;

		TempleEventKickOutUser((*usr));
		if ((*usr)->m_bAbnormalType != AbnormalType::ABNORMAL_NORMAL)
			(*usr)->StateChangeServerDirect(3, AbnormalType::ABNORMAL_NORMAL);
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventGetActiveEventTime(CUser *pUser)
void CGameServerDlg::TempleEventGetActiveEventTime(CUser* pUser)
{
	if (pUser == nullptr)
		return;

	uint16 remtime = 0;
	if (pTempleEvent.SignRemainSeconds > UNIXTIME)
		remtime = uint16(pTempleEvent.SignRemainSeconds - UNIXTIME);

	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT));
	result << pTempleEvent.ActiveEvent << remtime;
	pUser->Send(&result);
}

#pragma endregion

#pragma region CGameServerDlg::TempleEventSendActiveEventTime(CUser *pUser)
void CGameServerDlg::TempleEventSendActiveEventTime(CUser* pUser)
{
	if (pUser == nullptr || pTempleEvent.ActiveEvent == -1)
		return;

	if (!pUser->isEventUser())
		return TempleEventGetActiveEventTime(pUser);

	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT_JOIN));
	result << uint8(1) << uint16(pTempleEvent.ActiveEvent);
	pUser->Send(&result);

	switch ((EventOpCode)pTempleEvent.ActiveEvent)
	{
	case EventOpCode::TEMPLE_EVENT_CHAOS:
		TemplEventChaosSendJoinScreenUpdate(pUser);
		break;
	case EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR:
		TemplEventBDWSendJoinScreenUpdate(pUser);
		break;
	case EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
		TemplEventJuraidSendJoinScreenUpdate(pUser);
		break;
	}
}

#pragma endregion

#pragma region CGameServerDlg::TempleEventKickOutUser(CUser *pUser)
void CGameServerDlg::TempleEventKickOutUser(CUser* pUser) {
	if (pUser == nullptr || !pUser->isInGame())
		return;

	uint8 nZoneID = 0;
	if (pUser->GetZoneID() == ZONE_BORDER_DEFENSE_WAR || pUser->GetZoneID() == ZONE_CHAOS_DUNGEON) {
		if (pUser->GetLevel() >= 35 && pUser->GetNation() == (uint8)Nation::ELMORAD)
			nZoneID = ZONE_ELMORAD;
		else if (pUser->GetLevel() >= 35 && pUser->GetNation() == (uint8)Nation::KARUS)
			nZoneID = ZONE_KARUS;
		else
			nZoneID = ZONE_MORADON;
	}
	else if (pUser->GetZoneID() == ZONE_JURAID_MOUNTAIN) {
		if (pUser->GetMap() && pUser->GetLevel() >= pUser->GetMap()->GetMinLevelReq())
			nZoneID = ZONE_RONARK_LAND;
		else
			nZoneID = ZONE_MORADON;
	}

	if (nZoneID == 0)
		nZoneID = ZONE_MORADON;

	if (pUser->GetZoneID() == ZONE_BORDER_DEFENSE_WAR && pTempleEvent.ActiveEvent == (int16)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR) {
		if (pUser->m_bHasAlterOptained) CMagicProcess::RemoveType4Buff((uint8)BuffType::BUFF_TYPE_FRAGMENT_OF_MANES, pUser, true, true);
		pUser->m_bHasAlterOptained = false;
		pUser->BorderDefenceRemovePlayerRank();
	}
	else if (pUser->GetZoneID() == ZONE_CHAOS_DUNGEON && pTempleEvent.ActiveEvent == (int16)EventOpCode::TEMPLE_EVENT_CHAOS) {
		pUser->SetMaxHp(1);
		pUser->RobChaosSkillItems();
		pUser->ChaosExpansionRemovePlayerRank();
	}
	pUser->ResetTempleEventData();
	pUser->ZoneChange(nZoneID, 0.0f, 0.0f, 0, false, false, true);
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventBridgesClose(uint16 room)
void CGameServerDlg::TempleEventBridgesClose(uint16 room) {
	if (room > MAX_TEMPLE_EVENT_ROOM)
		return;

	auto* pRoomInfo = m_TempleEventJuraidRoomList.GetData(room);
	if (!pRoomInfo)
		return;

	for (int x = 0; x < 3; x++) {
		
		if (pRoomInfo->pkBridges[x]) {
			auto* pNpc = GetNpcPtr(pRoomInfo->pkBridges[x], ZONE_JURAID_MOUNTAIN);
			if (pNpc) 
				pNpc->m_byGateOpen = 0;
		}
		
		if (pRoomInfo->peBridges[x]) {
			auto* pNpc = GetNpcPtr(pRoomInfo->peBridges[x], ZONE_JURAID_MOUNTAIN);
			if (pNpc) 
				pNpc->m_byGateOpen = 0;
		}
	}
}

#pragma region CGameServerDlg::TempleEventReset(EventOpCode s, bool opened )
void CGameServerDlg::TempleEventReset(EventOpCode s, bool opened )
{
	if (opened && pTempleEvent.ActiveEvent != -1) { printf("TempleEventReset:[0] = 0x001\n"); return; }

	if (s == EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR) {
		RemoveAllEventNpc(ZONE_BORDER_DEFENSE_WAR);
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventBDWRoomList.GetData(i);
			if (pRoomInfo)
				pRoomInfo->Initialize(true);
		}
		TrashBorderDefenceWarRanking();
	}
	else if (s == EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN) {
		RemoveAllEventNpc(ZONE_JURAID_MOUNTAIN);
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventJuraidRoomList.GetData(i);
			if (!pRoomInfo)
				continue;

			for (int x = 0; x < 3; x++) {
				if (pRoomInfo->pkBridges[x]) {
					auto* pNpc = GetNpcPtr(pRoomInfo->pkBridges[x], ZONE_JURAID_MOUNTAIN);
					if (pNpc) 
						pNpc->m_byGateOpen = 0;
				}
				if (pRoomInfo->peBridges[x]) {
					auto* pNpc = GetNpcPtr(pRoomInfo->peBridges[x], ZONE_JURAID_MOUNTAIN);
					if (pNpc) 
						pNpc->m_byGateOpen = 0;
				}
			}
			pRoomInfo->Initialize(true);
		}
	}
	else if (s == EventOpCode::TEMPLE_EVENT_CHAOS) {
		RemoveAllEventNpc(ZONE_CHAOS_DUNGEON);
		for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
			auto* pRoomInfo = m_TempleEventChaosRoomList.GetData(i);
			if (pRoomInfo)
				pRoomInfo->Initialize();
		}
		TrashChaosExpansionRanking();
	}
	m_TempleEventLastUserOrder = 1;
	m_TempleEventUserMap.DeleteAllData();
	pTempleEvent.Initialize();
}
#pragma endregion

#pragma region CGameServerDlg::TempleEventChaosSendWinnerCounter()
void CGameServerDlg::TempleEventChaosShowCounterScreen()
{
	if (!isChaosExpansionActive()) return;

	Packet bresult(WIZ_EVENT, uint8(10));
	bresult << uint16(24) << uint8(104) << uint16(20) << uint16(0);
	Packet aresult(WIZ_RANK, uint8(RANK_TYPE_CHAOS_DUNGEON));
	for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
		auto *pRoomInfo = m_TempleEventChaosRoomList.GetData(i);
		if (!pRoomInfo)
			continue;

		pRoomInfo->UserListLock.lock();
		_CHAOS_ROOM_INFO::UserList copyUserList = pRoomInfo->m_UserList;
		pRoomInfo->UserListLock.unlock();

		foreach(itrUser, pRoomInfo->m_UserList) {
			auto& pEventUser = itrUser->second;
			if (pEventUser.strUserID.empty())
				continue;

			CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
			if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_CHAOS_DUNGEON)
				continue;

			pUser->StateChangeServerDirect(7, 0);
			pUser->Send(&bresult);
		}
	}
}
#pragma endregion

#pragma endregion