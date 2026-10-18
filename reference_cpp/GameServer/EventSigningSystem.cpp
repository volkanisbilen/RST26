#include "stdafx.h"
#include "DBAgent.h"
using std::string;

void CGameServerDlg::SendEventRemainingTime(bool bSendAll, CUser* pUser, uint8 ZoneID)
{
	Packet result(WIZ_BIFROST, uint8(BIFROST_EVENT));
	uint16 nRemainingTime = 0;

	if (ZoneID == ZONE_BATTLE4)
		nRemainingTime = m_byNereidsIslandRemainingTime / 2;

	result << nRemainingTime;

	if (pUser != nullptr)
		pUser->Send(&result);
	else if (bSendAll)
	{
		if (ZoneID == ZONE_BATTLE4)
			Send_All(&result, nullptr, 0, ZONE_BATTLE4);
		else
		{
			Send_All(&result, nullptr, 0, ZONE_RONARK_LAND);
			Send_All(&result, nullptr, 0, ZONE_BIFROST);
		}
	}

	if (ZoneID == ZONE_BATTLE4)
	{
		result.clear();
		result.Initialize(WIZ_MAP_EVENT);
		result << uint8(0) << uint8(7);

		for (int i = 0; i < 7; i++)
			result << m_sNereidsIslandMonuArray[i];
		if (pUser) pUser->Send(&result);
		result.clear();
		result.Initialize(WIZ_MAP_EVENT);
		result << uint8(2) << uint16(m_sElmoMonumentPoint) << uint16(m_sKarusMonumentPoint);
		if (pUser) pUser->Send(&result);
	}
}

void CUser::TempleProcess(Packet& pkt)
{
	uint8 opcode = pkt.read<uint8>();

	switch (opcode)
	{
	case MONSTER_STONE:
		MonsterStoneProcess(pkt);
		break;
	case TEMPLE_EVENT_JOIN:
		TempleJoinEvent(g_pMain->pTempleEvent.ActiveEvent);
		break;
	case TEMPLE_EVENT_DISBAND:
		TempleDisbandEvent(g_pMain->pTempleEvent.ActiveEvent);
		break;
	case TEMPLE_DRAKI_TOWER_ENTER:
		DrakiTowerTempleEnter(pkt);
		break;
	case TEMPLE_DRAKI_TOWER_LIST:
		DrakiTowerList();
		break;
	case TEMPLE_DRAKI_TOWER_TOWN:
		DrakiTowerTown();
	case TEMPLE_EVENT_DUNGEON_SIGN:
		DungeonDefenceSign();
		break;
		//Xtreme Monster Stone Recive Exp vermesi
	case TEMPLE_EVENT_GET_EXP:
		TempleGetExp(pkt);
		break;
	default:
		printf("Temple Process unhandled packets %d \n", opcode);
		TRACE("Temple Process unhandled packets %d \n", opcode);
		break;

	}
}

//Xtreme Monster Stone Recive Exp vermesi
bool CUser::TempleGetExp(Packet& pkt) {

	uint32 MonsterStoneItemID;
	pkt >> MonsterStoneItemID;

	if (CheckExistItem(MonsterStoneItemID, 1))
	{
		uint32 TotalExp = 25000 * GetLevel();
		std::string descp = "Temple Experience Reward"; // Xtreme Açıklama metni
		ExpChange(descp, TotalExp, true); // Xtreme TotalExp'i gönderiyoruz
		RobItem(MonsterStoneItemID);
	}

	return true;
}


/**
* @brief Handles join requests from user to the ongoing event.
*
*/
bool CUser::TempleJoinEvent(int16 sEvent)
{
	int16 nActiveEvent = g_pMain->pTempleEvent.ActiveEvent;
	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT_JOIN));

	if (isEventUser() || GetZoneID() == ZONE_PRISON || isInTempleEventZone())
	{
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return false;
	}

	if (nActiveEvent == -1 || sEvent == -1 || nActiveEvent != sEvent || !g_pMain->pTempleEvent.bAllowJoin)
	{
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return false;
	}

	if (g_pMain->pTempleEvent.isActive)
	{
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return false;
	}

	if (g_pMain->pTempleEvent.isAutomatic
		&& GetLevel() < g_pMain->pTempleEvent.pTime.minLevel) {
		g_pMain->SendHelpDescription(this, string_format("You must be at least level %d to participate in the event.", g_pMain->pTempleEvent.pTime.minLevel));
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return false;
	}

	if (g_pMain->pTempleEvent.isAutomatic
		&& GetLevel() > g_pMain->pTempleEvent.pTime.maxLevel) {
		g_pMain->SendHelpDescription(this, string_format("Levels between a minimum of %d and a maximum of %d can participate in the event.",
			g_pMain->pTempleEvent.pTime.minLevel, g_pMain->pTempleEvent.pTime.maxLevel));
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return false;
	}

	if (g_pMain->pTempleEvent.isAutomatic
		&& !hasLoyalty(g_pMain->pTempleEvent.pTime.reqLoyalty)) {
		g_pMain->SendHelpDescription(this, string_format("You must have a minimum of %d loyalty points to participate in the event.",
			g_pMain->pTempleEvent.pTime.reqLoyalty));
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return false;
	}

	if (g_pMain->pTempleEvent.isAutomatic
		&& !hasCoins(g_pMain->pTempleEvent.pTime.reqMoney)) {
		g_pMain->SendHelpDescription(this, string_format("You must have a minimum of %d loyalty points to participate in the event.",
			g_pMain->pTempleEvent.pTime.reqMoney));
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return false;
	}

	switch (sEvent)
	{
	case EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
		if (!g_pMain->AddEventUser(this))
		{
			result << uint8(4) << nActiveEvent;
			Send(&result);
			return false;
		}

		result << uint8(1) << nActiveEvent;
		Send(&result);
		GetNation() == (uint8)Nation::KARUS ? g_pMain->pTempleEvent.KarusUserCount++ : g_pMain->pTempleEvent.ElMoradUserCount++;
		g_pMain->pTempleEvent.AllUserCount = (g_pMain->pTempleEvent.KarusUserCount + g_pMain->pTempleEvent.ElMoradUserCount);
		m_sJoinedEvent = (int16)EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN;
		g_pMain->TemplEventJuraidSendJoinScreenUpdate();
		return true;
	case TEMPLE_EVENT_BORDER_DEFENCE_WAR:
	{
		if (!g_pMain->AddEventUser(this))
		{
			result << uint8(4) << nActiveEvent;
			Send(&result);
			return false;
		}

		result << uint8(1) << nActiveEvent;
		Send(&result);

		GetNation() == (uint8)Nation::KARUS ? g_pMain->pTempleEvent.KarusUserCount++ : g_pMain->pTempleEvent.ElMoradUserCount++;
		g_pMain->pTempleEvent.AllUserCount = (g_pMain->pTempleEvent.KarusUserCount + g_pMain->pTempleEvent.ElMoradUserCount);
		m_sJoinedEvent = (int16)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR;
		g_pMain->TemplEventBDWSendJoinScreenUpdate();
		return true;
	}
	break;
	case TEMPLE_EVENT_CHAOS:
	{
		auto* pItem = GetItem(RIGHTHAND);
		if (isMining() || isFishing() || pItem == nullptr
			|| pItem->nNum == MATTOCK
			|| pItem->nNum == GOLDEN_MATTOCK
			|| pItem->nNum == FISHING
			|| pItem->nNum == GOLDEN_FISHING) {
			result << uint8(4) << nActiveEvent;
			Send(&result);
			return false;
		}

		if (!g_pMain->AddEventUser(this)) {
			result << uint8(4) << nActiveEvent;
			Send(&result);
			return false;
		}

		result << uint8(1) << nActiveEvent;
		Send(&result);

		GetNation() == (uint8)Nation::KARUS ? g_pMain->pTempleEvent.KarusUserCount++ : g_pMain->pTempleEvent.ElMoradUserCount++;
		g_pMain->pTempleEvent.AllUserCount = (g_pMain->pTempleEvent.KarusUserCount + g_pMain->pTempleEvent.ElMoradUserCount);
		m_sJoinedEvent = (int16)EventOpCode::TEMPLE_EVENT_CHAOS;
		g_pMain->TemplEventChaosSendJoinScreenUpdate();
		return true;
	}
	break;
	default:
		break;
	}

	return false;
}

void CUser::TempleDisbandEvent(int16 sEvent)
{
	uint16 nActiveEvent = g_pMain->pTempleEvent.ActiveEvent;
	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT_DISBAND));
	if (!isEventUser() || GetZoneID() == ZONE_PRISON)
	{
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return;
	}

	if (nActiveEvent == -1 || sEvent == -1 || nActiveEvent != sEvent)
	{
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return;
	}

	if (g_pMain->pTempleEvent.isActive)
	{
		result << uint8(4) << nActiveEvent;
		Send(&result);
		return;
	}

	switch ((EventOpCode)sEvent)
	{
	case EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN:
	{
		if (GetEventRoom() > 0) {

			if (GetEventRoom() > MAX_TEMPLE_EVENT_ROOM)
				return;

			auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(GetEventRoom());
			if (!pRoomInfo)
				return;

			if (GetNation() == (uint8)Nation::KARUS) {
				pRoomInfo->m_KarusUserListLock.lock();
				pRoomInfo->m_KarusUserList.erase(GetName());
				pRoomInfo->m_KarusUserListLock.unlock();
			}
			else {
				pRoomInfo->m_ElmoradUserListLock.lock();
				pRoomInfo->m_ElmoradUserList.erase(GetName());
				pRoomInfo->m_ElmoradUserListLock.unlock();
			}
		}
		else {
			GetNation() == (uint8)Nation::KARUS ? g_pMain->pTempleEvent.KarusUserCount-- : g_pMain->pTempleEvent.ElMoradUserCount--;
			g_pMain->pTempleEvent.AllUserCount = g_pMain->pTempleEvent.KarusUserCount + g_pMain->pTempleEvent.ElMoradUserCount;
			g_pMain->RemoveEventUser(this);
			result << uint8(1) << nActiveEvent;
			Send(&result);
			g_pMain->TemplEventJuraidSendJoinScreenUpdate();
			g_pMain->TemplEventJuraidSendJoinScreenUpdate(this);
		}
	}
	break;
	case EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR:
	{
		if (GetEventRoom() > 0) {
			auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
			if (!pRoomInfo)
				return;

			if (GetNation() == (uint8)Nation::KARUS) {
				pRoomInfo->m_KarusUserListLock.lock();
				pRoomInfo->m_KarusUserList.erase(GetName());
				pRoomInfo->m_KarusUserListLock.unlock();
			}
			else {
				pRoomInfo->m_ElmoradUserListLock.lock();
				pRoomInfo->m_ElmoradUserList.erase(GetName());
				pRoomInfo->m_ElmoradUserListLock.unlock();
			}
		}
		else {
			GetNation() == (uint8)Nation::KARUS ? g_pMain->pTempleEvent.KarusUserCount-- : g_pMain->pTempleEvent.ElMoradUserCount--;
			g_pMain->pTempleEvent.AllUserCount = g_pMain->pTempleEvent.KarusUserCount + g_pMain->pTempleEvent.ElMoradUserCount;
			g_pMain->RemoveEventUser(this);


			result << uint8(1) << nActiveEvent;
			Send(&result);
			g_pMain->TemplEventBDWSendJoinScreenUpdate();
			g_pMain->TemplEventBDWSendJoinScreenUpdate(this);
		}
	}
	break;
	case EventOpCode::TEMPLE_EVENT_CHAOS:
	{
		if (GetEventRoom() > 0) {
			auto* pRoomInfo = g_pMain->m_TempleEventChaosRoomList.GetData(GetEventRoom());
			if (!pRoomInfo)
				return;

			Guard lock(pRoomInfo->UserListLock);
			pRoomInfo->m_UserList.erase(GetName());
		}
		else {
			GetNation() == (uint8)Nation::KARUS ? g_pMain->pTempleEvent.KarusUserCount-- : g_pMain->pTempleEvent.ElMoradUserCount--;
			g_pMain->pTempleEvent.AllUserCount = g_pMain->pTempleEvent.KarusUserCount + g_pMain->pTempleEvent.ElMoradUserCount;
			g_pMain->RemoveEventUser(this);

			result << uint8(1) << nActiveEvent;
			Send(&result);
			g_pMain->TemplEventChaosSendJoinScreenUpdate();
			g_pMain->TemplEventChaosSendJoinScreenUpdate(this);
		}
	}
	break;
	default:
		break;
	}
}

bool CGameServerDlg::AddEventUser(CUser* pUser)
{
	if (pUser == nullptr || !pUser->isInGame())
		return false;

	if (pUser->m_EventDisandTime > UNIXTIME)
	{
		g_pMain->SendHelpDescription(pUser, string_format("Event katýlýmýnýz A-F-K kaldýgýnýz için 12 saat boyunca yasaklanmýþtýr."));
		g_pMain->SendHelpDescription(pUser, string_format("Ceza Bitimine Kalan Süre : %d saat.", time_t(pUser->m_EventDisandTime - UNIXTIME) / 60 / 60));
		return false;
	}
	foreach_stlmap(itr, m_TempleEventUserMap)
	{
		if (itr->second->reg_hwID > 0 && itr->second->reg_hwID == pUser->nHardwareID)
		{
			g_pMain->SendHelpDescription(pUser, string_format("Event katýlýmýný yalnýzca [1] Client'den saðlayabilirsiniz."));
			return false;
		}
	}
	_TEMPLE_EVENT_USER* pEventUser = new _TEMPLE_EVENT_USER();
	pEventUser->strUserID = pUser->GetName();
	pEventUser->m_iJoinOrder = m_TempleEventLastUserOrder++;
	pEventUser->reg_hwID = pUser->nHardwareID;
	if (!g_pMain->m_TempleEventUserMap.PutData(pEventUser->m_iJoinOrder, pEventUser)) {
		delete pEventUser;
		return false;
	}

	pUser->m_iEventJoinOrder = pEventUser->m_iJoinOrder;
	pUser->m_sJoinedEvent = g_pMain->pTempleEvent.ActiveEvent;
	return true;
}

void CGameServerDlg::RemoveEventUser(CUser* pUser)
{
	if (pUser == nullptr)
		return;

	g_pMain->m_TempleEventUserMap.DeleteData(pUser->m_iEventJoinOrder);
	pUser->ResetTempleEventData();
}

#pragma region 	CUser::isEventUser()
/**
* @brief Returns true if the user joined an event.
*/
bool CUser::isEventUser()
{
	return m_sJoinedEvent > 0;
}
#pragma endregion

#pragma region 	Event System Lua Kontrol
uint8 CUser::GetMonsterChallengeTime() {
	auto& aa = g_pMain->pForgettenTemple;
	if (!aa.isActive || !aa.isJoin) return 0;
	if ((aa.MinLevel && GetLevel() < aa.MinLevel) || (aa.MaxLevel && GetLevel() > aa.MaxLevel)) return 0;
	return 1;
}

uint8 CUser::GetMonsterChallengeUserCount() {
	Guard lock(g_pMain->pForgettenTemple.UserListLock);
	return (uint8)g_pMain->pForgettenTemple.UserList.size();
}

bool CUser::GetUnderTheCastleOpen() { return g_pMain->m_bUnderTheCastleIsActive; }

uint16 CUser::GetUnderTheCastleUserCount() { return (uint16)g_pMain->m_nUnderTheCastleUsers.size(); }

bool CUser::GetJuraidMountainTime()
{
	if (g_pMain->pTempleEvent.ActiveEvent == TEMPLE_EVENT_JURAD_MOUNTAIN)
	{
		if (g_pMain->pTempleEvent.bAllowJoin)
			return true;
	}
	return false;
}
#pragma endregion

#pragma region 	Juraid , Border , Chaos Kontrol 
/**
* @brief Sends the Join Screen counter update.
*
*/
void CGameServerDlg::TemplEventJuraidSendJoinScreenUpdate(CUser* pUser /*= nullptr*/)
{
	uint16 remtime = 0;
	if (pTempleEvent.SignRemainSeconds > UNIXTIME)
		remtime = uint16(pTempleEvent.SignRemainSeconds - UNIXTIME);

	/*Packet result(WIZ_SELECT_MSG);
	result << uint16(0x00) << uint8(0x07) << uint64(0x00) << uint32(0x06)
		<< pTempleEvent.KarusUserCount << uint16(0x00) << pTempleEvent.ElMoradUserCount
		<< uint16(0x00) << remtime << uint16(0x00);*/

	Packet result(XSafe, uint8(XSafeOpCodes::JURAID));
	result << pTempleEvent.KarusUserCount << pTempleEvent.ElMoradUserCount << remtime;

	if (pUser)
		pUser->Send(&result);
	else
		TempleEventJoinScreenUpdateSend(result);
}

void CGameServerDlg::TemplEventBDWSendJoinScreenUpdate(CUser* pUser /*= nullptr*/)
{
	uint16 kcount = g_pMain->pTempleEvent.KarusUserCount, ecount = g_pMain->pTempleEvent.ElMoradUserCount;
	if (kcount && g_pMain->pRankBug.BorderJoin) kcount *= g_pMain->pRankBug.BorderJoin;
	if (ecount && g_pMain->pRankBug.BorderJoin) ecount *= g_pMain->pRankBug.BorderJoin;

	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT_COUNTER));
	result << uint16(TEMPLE_EVENT_BORDER_DEFENCE_WAR) << kcount << ecount;

	if (pUser) pUser->Send(&result);
	else TempleEventJoinScreenUpdateSend(result);
}

void CGameServerDlg::TemplEventChaosSendJoinScreenUpdate(CUser* pUser /*= nullptr*/)
{
	uint16 count = g_pMain->pTempleEvent.AllUserCount;
	if (count && g_pMain->pRankBug.ChaosJoin) count *= g_pMain->pRankBug.ChaosJoin;

	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT_COUNTER));
	result << uint16(TEMPLE_EVENT_CHAOS) << count;

	if (pUser) pUser->Send(&result);
	else TempleEventJoinScreenUpdateSend(result);
}

void CGameServerDlg::TempleEventJoinScreenUpdateSend(Packet& pkt)
{
	std::vector<std::string> m_userlist;
	m_TempleEventUserMap.m_lock.lock();
	foreach_stlmap_nolock(i, m_TempleEventUserMap)
		if (i->second)
			m_userlist.push_back(i->second->strUserID);
	m_TempleEventUserMap.m_lock.unlock();

	foreach(itr, m_userlist) {
		CUser* pSendUser = GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (pSendUser == nullptr || !pSendUser->isInGame() || !pSendUser->isEventUser())
			continue;

		pSendUser->Send(&pkt);
	}
}
#pragma endregion