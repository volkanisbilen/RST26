#include "stdafx.h"

void CUser::SendMonsterStoneFail(uint8 errorid) {
	Packet msevent(WIZ_EVENT, uint8(MONSTER_STONE));
	msevent << errorid;
	Send(&msevent);
}

bool CUser::MonsterStoneQuestJoin(uint16 sQuestID, uint8 bTargetZoneID, uint8 bTargetFamily)
{
	if (!isInGame() || isTrading() || isMerchanting()
		|| isSellingMerchantingPreparing() || isFishing() || isMining())
		return false;

	if (!g_pMain->pServerSetting.monsterstone_status)
	{
		g_pMain->SendHelpDescription(this, "Monster Stone map is in maintenance mode.");
		SendMonsterStoneFail(1);
		return false;
	}

	if (sQuestID != 199)
	{
		g_pMain->SendHelpDescription(this, "Monster Stone quest join failed. Invalid quest id.");
		SendMonsterStoneFail(5);
		return false;
	}

	if (pCindWar.isEventUser() && g_pMain->isCindirellaZone(GetZoneID()))
	{
		SendMonsterStoneFail(1);
		return false;
	}

	if (GetHealth() < (GetMaxHealth() / 2))
	{
		SendMonsterStoneFail(9);
		return false;
	}

	if (GetZoneID() == ZONE_PRISON || isInMonsterStoneZone() || isInTempleEventZone())
	{
		SendMonsterStoneFail(1);
		return false;
	}

	if (m_ismsevent || GetEventRoom() > 0)
	{
		SendMonsterStoneFail(5);
		return false;
	}

	// Lua can now explicitly send target ZoneID + Family:
	// MonsterStoneQuestJoin(UID, 199, 82, 71)
	// If old Lua files call only MonsterStoneQuestJoin(UID, 199), keep the previous safe default.
	uint8 bZoneID = bTargetZoneID == 0 ? ZONE_STONE1 : bTargetZoneID;
	uint8 bFamily = bTargetFamily == 0 ? 4 : bTargetFamily;

	if (!isInMonsterStoneZone(bZoneID) || bFamily == 0)
	{
		g_pMain->SendHelpDescription(this, "Monster Stone quest join failed. Invalid ZoneID/Family.");
		SendMonsterStoneFail(5);
		return false;
	}

	int roomid = -1;
	for (int i = 0; i < MAX_MONSTER_STONE_ROOM; i++)
	{
		auto& pRoomInfo = g_pMain->m_TempleEventMonsterStoneRoomList[i];
		if (pRoomInfo.isnull() || pRoomInfo.Active || pRoomInfo.mUserList.size())
			continue;

		roomid = i;
		break;
	}

	if (roomid < 0 || roomid >= MAX_MONSTER_STONE_ROOM)
	{
		SendMonsterStoneFail(5);
		return false;
	}

	auto& pRoom = g_pMain->m_TempleEventMonsterStoneRoomList[roomid];
	if (pRoom.isnull() || pRoom.Active || pRoom.roomid != roomid)
	{
		SendMonsterStoneFail(5);
		return false;
	}

	std::vector<_MONSTER_STONE_LIST_INFORMATION> mlist;
	g_pMain->m_MonsterStoneListInformationArray.m_lock.lock();
	foreach_stlmap_nolock(itr, g_pMain->m_MonsterStoneListInformationArray)
	{
		if (itr->second == nullptr || itr->second->ZoneID != bZoneID || itr->second->Family != bFamily)
			continue;

		mlist.push_back(*itr->second);
	}
	g_pMain->m_MonsterStoneListInformationArray.m_lock.unlock();

	if (mlist.empty())
	{
		g_pMain->SendHelpDescription(this, "Monster Stone quest join failed. Spawn data not found for requested ZoneID/Family.");
		SendMonsterStoneFail(5);
		return false;
	}

	pRoom.Active = true;
	pRoom.StartTime = UNIXTIME;
	pRoom.FinishTime = pRoom.StartTime + MONSTER_STONE_TIME;
	pRoom.StartZoneID = GetZoneID();
	pRoom.zoneid = bZoneID;
	pRoom.MonsterFamily = bFamily;
	pRoom.isBossKilled = false;
	pRoom.WaitingTime = 0;
	pRoom.bPartyMonsterStone = false;
	pRoom.mUserList.push_back(GetSocketID());

	foreach(itr, mlist)
	{
		g_pMain->SpawnEventNpc(itr->sSid, itr->bType == 0 ? true : false, itr->ZoneID, itr->X, itr->Y, itr->Z, itr->sCount, 0,
			MONSTER_STONE_DEAD_TIME, 0, -1, pRoom.roomid + 1, itr->byDirection, 1, 0, (uint16)SpawnEventType::MonsterStone, itr->isBoss);
	}

	// Default Monster Stone support NPCs for Stone 1.
	g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 204, 0, 201, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
	g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 204, 0, 197, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
	g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 204, 0, 193, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);

	if (!ZoneChange(pRoom.zoneid, 0.0f, 0.0f, pRoom.roomid + 1))
	{
		pRoom.Active = false;
		pRoom.mUserList.clear();
		g_pMain->TempleMonsterStoneResetNpcs(pRoom.roomid, pRoom.zoneid);
		SendMonsterStoneFail(5);
		return false;
	}

	m_ismsevent = true;
	MonsterStoneTimerScreen(MONSTER_STONE_TIME);
	return true;
}

#pragma region CGameServerDlg::MonsterStoneProcess(Packet & pkt)
void CUser::MonsterStoneProcess(Packet& pkt)
{
	if (!isInGame() || isTrading() || isMerchanting()
		|| isSellingMerchantingPreparing() || isFishing() || isMining())
		return;

	if (pCindWar.isEventUser() && g_pMain->isCindirellaZone(GetZoneID()))
		return SendMonsterStoneFail(1);

	uint32 itemid; pkt >> itemid;

	if (!g_pMain->pServerSetting.monsterstone_status) {
		g_pMain->SendHelpDescription(this, "Monster Stone map is in maintenance mode.");
		//g_pMain->SendHelpDescription(this, "Monster Stone haritası bakım modundadır.");
		return SendMonsterStoneFail(1);
	}

	Packet msevent(WIZ_EVENT, uint8(MONSTER_STONE));
	msevent << itemid;

	if (itemid != 300144036 && itemid != 300145037 && itemid != 300146038 && itemid != 900144023 && itemid != 300145039)
		return;

	if (GetHealth() < (GetMaxHealth() / 2))
		return SendMonsterStoneFail(9);

	if (GetZoneID() == ZONE_PRISON || isInMonsterStoneZone() || isInTempleEventZone())
		return SendMonsterStoneFail(1);

	if (m_ismsevent || GetEventRoom() > 0 || !CheckExistItem(itemid, 1))
		return SendMonsterStoneFail(5);

	int roomid = -1;
	for (int i = 0; i < MAX_MONSTER_STONE_ROOM; i++) {
		auto& pRoomInfo = g_pMain->m_TempleEventMonsterStoneRoomList[i];
		if (pRoomInfo.isnull() || pRoomInfo.Active || pRoomInfo.mUserList.size())
			continue;

		roomid = i;
		break;
	}

	if (roomid < 0 || roomid >= MAX_MONSTER_STONE_ROOM)
		return;

	auto& pRoom = g_pMain->m_TempleEventMonsterStoneRoomList[roomid];
	if (pRoom.isnull() || pRoom.Active || pRoom.roomid != roomid)
		return;

	if (itemid == 300145039)
	{

		if (!isInParty() || !isPartyLeader())
			return SendMonsterStoneFail(5);
		uint8 bZoneID = ZONE_STONE3, bFamilly = 0;
		_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
		if (pParty == nullptr)
			return SendMonsterStoneFail(5);

		for (int i = 0; i < GetPartyMemberAmount(); i++) {
			CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
			if (pUser == nullptr || !pUser->isInGame())
				continue;

			if (GetNation() != pUser->GetNation())
				return SendMonsterStoneFail(5);
		}
		std::vector< _MONSTER_STONE_LIST_INFORMATION > mlist;
		g_pMain->m_MonsterStoneListInformationArray.m_lock.lock();
		foreach_stlmap_nolock(itr, g_pMain->m_MonsterStoneListInformationArray) {
			if (itr->second == nullptr || itr->second->ZoneID != bZoneID || itr->second->Family != bFamilly)
				continue;

			mlist.push_back(*itr->second);
		}
		g_pMain->m_MonsterStoneListInformationArray.m_lock.unlock();

		if (/*mlist.empty() ||*/ !RobItem(itemid, 1))
			return SendMonsterStoneFail(5);
		pRoom.Active = true;
		pRoom.StartTime = UNIXTIME;
		pRoom.FinishTime = pRoom.StartTime + MONSTER_STONE_TIME;
		pRoom.StartZoneID = GetZoneID();
		pRoom.zoneid = bZoneID;
		pRoom.MonsterFamily = bFamilly;
		pRoom.isBossKilled = false;
		pRoom.WaitingTime = 0;
		pRoom.bPartyMonsterStone = true;
		foreach(aaa, mlist)
			g_pMain->SpawnEventNpc(aaa->sSid, aaa->bType == 0 ? true : false, aaa->ZoneID, aaa->X, aaa->Y, aaa->Z, aaa->sCount, 0,
				MONSTER_STONE_DEAD_TIME, 0, -1, pRoom.roomid + 1, aaa->byDirection, 1, 0, (uint16)SpawnEventType::MonsterStone, aaa->isBoss);
		for (int i = 0; i < MAX_PARTY_USERS; i++)
		{
			CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
			if (pUser == nullptr || pUser->GetSocketID() == GetSocketID()) continue;

			pRoom.mUserList.push_back(pUser->GetSocketID());
			pUser->ZoneChange(pRoom.zoneid, 0.0f, 0.0f, pRoom.roomid + 1);
			pUser->MonsterStoneTimerScreen(MONSTER_STONE_TIME);

		}
		pRoom.mUserList.push_back(GetSocketID());
		ZoneChange(pRoom.zoneid, 0.0f, 0.0f, pRoom.roomid + 1);
		auto* pPartyb = g_pMain->CreateParty(this);
		if (pPartyb == nullptr)
			return SendMonsterStoneFail(5);

		this->EventPartyCreate();
		foreach(itr, pRoom.mUserList)
		{
			if (*itr != this->GetSocketID())
				this->EventPartyInvitationCheck(*itr);
		}

	}

	else

		if (g_pMain->pServerSetting.new_monsterstone) {
			if (itemid == 300144036 || itemid == 300145037 || itemid == 300146038) {
				uint8 bZoneID = 0, bFamilly = 0;
				if (itemid == 300144036) bZoneID = ZONE_STONE1;
				else if (itemid == 300145037) bZoneID = ZONE_STONE2;
				else if (itemid == 300146038) bZoneID = ZONE_STONE3;

				if (!isInMonsterStoneZone(bZoneID))
					return SendMonsterStoneFail(5);

				if (bZoneID == ZONE_STONE1) bFamilly = myrand(1, 4);
				else if (bZoneID == ZONE_STONE2) bFamilly = myrand(5, 9);
				else if (bZoneID == ZONE_STONE3) bFamilly = myrand(10, 13);

				if (bZoneID == ZONE_STONE1) {
					if (bFamilly != 1 && bFamilly != 2 && bFamilly != 3
						&& bFamilly != 4) {
						printf("Monster Stone Zone Fail: Stone 1\n");
						return SendMonsterStoneFail(5);
					}
				}
				else if (bZoneID == ZONE_STONE2) {
					if (bFamilly != 5 && bFamilly != 6 && bFamilly != 7 &&
						bFamilly != 8 && bFamilly != 9) {
						printf("Monster Stone Zone Fail: Stone 2\n");
						return SendMonsterStoneFail(5);
					}
				}
				else if (bZoneID == ZONE_STONE3) {
					if (bFamilly != 10 && bFamilly != 11 && bFamilly != 12
						&& bFamilly != 13) {
						printf("Monster Stone Zone Fail: Stone 3\n");
						return SendMonsterStoneFail(5);
					}
				}

				std::vector< _MONSTER_STONE_LIST_INFORMATION > mlist;
				g_pMain->m_MonsterStoneListInformationArray.m_lock.lock();
				foreach_stlmap_nolock(itr, g_pMain->m_MonsterStoneListInformationArray) {
					if (itr->second == nullptr || itr->second->ZoneID != bZoneID || itr->second->Family != bFamilly)
						continue;

					mlist.push_back(*itr->second);
				}
				g_pMain->m_MonsterStoneListInformationArray.m_lock.unlock();

				if (mlist.empty() || !RobItem(itemid, 1))
					return SendMonsterStoneFail(5);

				pRoom.Active = true;
				pRoom.StartTime = UNIXTIME;
				pRoom.FinishTime = pRoom.StartTime + MONSTER_STONE_TIME;
				pRoom.StartZoneID = GetZoneID();
				pRoom.zoneid = bZoneID;
				pRoom.MonsterFamily = bFamilly;
				pRoom.isBossKilled = false;
				pRoom.WaitingTime = 0;
				pRoom.mUserList.push_back(GetSocketID());

				foreach(aaa, mlist)
					g_pMain->SpawnEventNpc(aaa->sSid, aaa->bType == 0 ? true : false, aaa->ZoneID, aaa->X, aaa->Y, aaa->Z, aaa->sCount, 0,
						MONSTER_STONE_DEAD_TIME, 0, -1, pRoom.roomid + 1, aaa->byDirection, 1, 0, (uint16)SpawnEventType::MonsterStone, aaa->isBoss);

				if (pRoom.zoneid == ZONE_STONE1) {
					g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 204, 0, 201, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 204, 0, 197, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 204, 0, 193, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
				}
				else if (pRoom.zoneid == ZONE_STONE2) {
					g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 203, 0, 202, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 203, 0, 197, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 203, 0, 193, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
				}
				else if (pRoom.zoneid == ZONE_STONE3) {
					g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 204, 0, 207, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 204, 0, 200, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 204, 0, 194, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
				}

				if (!ZoneChange(pRoom.zoneid, 0.0f, 0.0f, pRoom.roomid + 1))
					return SendMonsterStoneFail(5);
			}
			else return;
		}
		else {
			if (itemid == 900144023) {

				uint8 bZoneID = 0, bFamilly = 0;
				if (GetLevel() >= 20 && GetLevel() <= 29)
				{
					bZoneID = ZONE_STONE1;
					bFamilly = 1;
				}
				else if (GetLevel() >= 30 && GetLevel() <= 35)
				{
					bZoneID = ZONE_STONE1;
					bFamilly = 2;
				}
				else if (GetLevel() >= 36 && GetLevel() <= 40)
				{
					bZoneID = ZONE_STONE1;
					bFamilly = 3;
				}
				else if (GetLevel() >= 41 && GetLevel() <= 46)
				{
					bZoneID = ZONE_STONE1;
					bFamilly = 4;
				}
				else if (GetLevel() >= 47 && GetLevel() <= 55)
				{
					bZoneID = (uint8)myrand(ZONE_STONE1, ZONE_STONE2);

					if (bZoneID == ZONE_STONE1)
						bFamilly = 4;
					else
						bFamilly = 5;
				}
				else if (GetLevel() >= 56 && GetLevel() <= 60)
				{
					bZoneID = ZONE_STONE2;
					bFamilly = (uint8)myrand(6, 8);
				}
				else if (GetLevel() >= 61 && GetLevel() <= 66)
				{
					bZoneID = ZONE_STONE2;
					bFamilly = (uint8)myrand(8, 9);
				}
				else if (GetLevel() >= 67 && GetLevel() <= 70)
				{
					bFamilly = (uint8)myrand(9, 10);

					if (bFamilly == 9)
						bZoneID = ZONE_STONE2;
					else
						bZoneID = ZONE_STONE3;
				}
				else if (GetLevel() >= 71 && GetLevel() <= 74)
				{
					bZoneID = ZONE_STONE3;
					bFamilly = (uint8)myrand(10, 12);
				}
				else if (GetLevel() >= 75 && GetLevel() <= g_pMain->m_byMaxLevel)
				{
					bZoneID = ZONE_STONE3;
					bFamilly = 13;
				}

				if (!bFamilly)
					return;

				if (!isInMonsterStoneZone(bZoneID))
					return SendMonsterStoneFail(5);

				if (bZoneID == ZONE_STONE1) {
					if (bFamilly != 1 && bFamilly != 2 && bFamilly != 3
						&& bFamilly != 4) {
						printf("Monster Stone Zone Fail: Stone 1\n");
						return SendMonsterStoneFail(5);
					}
				}
				else if (bZoneID == ZONE_STONE2) {
					if (bFamilly != 5 && bFamilly != 6 && bFamilly != 7 &&
						bFamilly != 8 && bFamilly != 9) {
						printf("Monster Stone Zone Fail: Stone 2\n");
						return SendMonsterStoneFail(5);
					}
				}
				else if (bZoneID == ZONE_STONE3) {
					if (bFamilly != 10 && bFamilly != 11 && bFamilly != 12
						&& bFamilly != 13) {
						printf("Monster Stone Zone Fail: Stone 3\n");
						return SendMonsterStoneFail(5);
					}
				}

				std::vector< _MONSTER_STONE_LIST_INFORMATION > mlist;
				g_pMain->m_MonsterStoneListInformationArray.m_lock.lock();
				foreach_stlmap_nolock(itr, g_pMain->m_MonsterStoneListInformationArray) {
					if (itr->second == nullptr || itr->second->ZoneID != bZoneID || itr->second->Family != bFamilly)
						continue;

					mlist.push_back(*itr->second);
				}
				g_pMain->m_MonsterStoneListInformationArray.m_lock.unlock();

				if (mlist.empty() || !RobItem(900144023, 1))
					return SendMonsterStoneFail(5);

				pRoom.Active = true;
				pRoom.StartTime = UNIXTIME;
				pRoom.FinishTime = pRoom.StartTime + MONSTER_STONE_TIME;
				pRoom.StartZoneID = GetZoneID();
				pRoom.zoneid = bZoneID;
				pRoom.MonsterFamily = bFamilly;
				pRoom.isBossKilled = false;
				pRoom.WaitingTime = 0;
				pRoom.mUserList.push_back(GetSocketID());

				foreach(aaa, mlist)
					g_pMain->SpawnEventNpc(aaa->sSid, aaa->bType == 0 ? true : false, aaa->ZoneID, aaa->X, aaa->Y, aaa->Z, aaa->sCount, 0,
						MONSTER_STONE_DEAD_TIME, 0, -1, pRoom.roomid + 1, aaa->byDirection, 1, 0, (uint16)SpawnEventType::MonsterStone, aaa->isBoss);

				if (pRoom.zoneid == ZONE_STONE1) {
					g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 204, 0, 201, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 204, 0, 197, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 204, 0, 193, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
				}
				else if (pRoom.zoneid == ZONE_STONE2) {
					g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 203, 0, 202, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 203, 0, 197, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 203, 0, 193, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
				}
				else if (pRoom.zoneid == ZONE_STONE3) {
					g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 204, 0, 207, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 204, 0, 200, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
					g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 204, 0, 194, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
				}

				if (!ZoneChange(pRoom.zoneid, 0.0f, 0.0f, pRoom.roomid + 1))
					return SendMonsterStoneFail(5);
			}
			else return;
		}

	m_ismsevent = true;
	MonsterStoneTimerScreen(MONSTER_STONE_TIME);
}
#pragma endregion


#pragma region CUser::HandleMonsterStoneTestCommand
bool CUser::HandleMonsterStoneTestCommand(CommandArgs& vargs, const char* args, const char* description)
{
	if (!isGM())
		return false;

	if (vargs.empty())
	{
		g_pMain->SendHelpDescription(this, "Monster Stone GM Test Kullanimi:");
		g_pMain->SendHelpDescription(this, "+ms list");
		g_pMain->SendHelpDescription(this, "+ms reset");
		g_pMain->SendHelpDescription(this, "+ms stone1 <family> veya +ms 81 <family>");
		g_pMain->SendHelpDescription(this, "+ms stone2 <family> veya +ms 82 <family>");
		g_pMain->SendHelpDescription(this, "+ms stone3 <family> veya +ms 83 <family>");
		return true;
	}

	std::string firstArg = vargs.front();
	STRTOLOWER(firstArg);

	if (firstArg == "list")
	{
		g_pMain->SendHelpDescription(this, "Monster Stone Family Listesi:");
		g_pMain->SendHelpDescription(this, "stone1 / Zone 81 : DBde olan herhangi Family (normal item akisi 1-4)");
		g_pMain->SendHelpDescription(this, "stone2 / Zone 82 : DBde olan herhangi Family (normal item akisi 5-9, quest testleri 59/71 olabilir)");
		g_pMain->SendHelpDescription(this, "stone3 / Zone 83 : DBde olan herhangi Family (normal item akisi 10-13)");
		return true;
	}

	if (firstArg == "reset")
	{
		if (!isInMonsterStoneZone() || GetEventRoom() < 1 || GetEventRoom() > MAX_MONSTER_STONE_ROOM)
		{
			g_pMain->SendHelpDescription(this, "Aktif Monster Stone odasinda degilsiniz.");
			return true;
		}

		auto& pRoomInfo = g_pMain->m_TempleEventMonsterStoneRoomList[GetEventRoom() - 1];
		if (pRoomInfo.isnull() || !pRoomInfo.Active)
		{
			g_pMain->SendHelpDescription(this, "Bu Monster Stone odasi aktif degil.");
			return true;
		}

		g_pMain->TempleMonsterStoneAutoResetRoom(pRoomInfo);
		g_pMain->SendHelpDescription(this, "Monster Stone odasi resetlendi.");
		return true;
	}

	if (vargs.size() < 2)
	{
		g_pMain->SendHelpDescription(this, "Eksik parametre. Ornek: +ms stone1 1");
		return true;
	}

	uint8 bZoneID = 0, bFamily = 0;

	if (firstArg == "stone1" || firstArg == "ms1")
		bZoneID = ZONE_STONE1;
	else if (firstArg == "stone2" || firstArg == "ms2")
		bZoneID = ZONE_STONE2;
	else if (firstArg == "stone3" || firstArg == "ms3")
		bZoneID = ZONE_STONE3;
	else
		bZoneID = (uint8)atoi(firstArg.c_str());

	vargs.pop_front();
	bFamily = (uint8)atoi(vargs.front().c_str());

	// GM test komutunda family araligi bilincli olarak serbest birakildi.
	// Normal Monster Stone item akisi stone1=1-4, stone2=5-9, stone3=10-13 kullanir;
	// fakat quest/Lua testlerinde veritabaninda farkli Family degerleri (or: 59, 71) bulunabilir.
	// Bu nedenle burada sadece ZoneID'nin Monster Stone zone'u olup olmadigini kontrol ediyoruz.
	// Asil dogrulama asagida MONSTER_STONE_RESPAWN_LIST icinde ZoneID + Family kaydi aranarak yapilir.
	if (!isInMonsterStoneZone(bZoneID))
	{
		g_pMain->SendHelpDescription(this, "Hatali Monster Stone zone degeri. Kullan: stone1/81, stone2/82 veya stone3/83");
		return true;
	}

	if (bFamily == 0)
	{
		g_pMain->SendHelpDescription(this, "Family 0 kullanilamaz. Ornek: +ms stone2 71");
		return true;
	}

	if (isTrading() || isMerchanting() || isSellingMerchantingPreparing() || isFishing() || isMining())
	{
		g_pMain->SendHelpDescription(this, "Bu durumdayken Monster Stone test odasi acilamaz.");
		return true;
	}

	if (isInMonsterStoneZone() || GetEventRoom() > 0)
	{
		g_pMain->SendHelpDescription(this, "Once mevcut event/Monster Stone odasindan cikin veya +ms reset kullanin.");
		return true;
	}

	int roomid = -1;
	for (int i = 0; i < MAX_MONSTER_STONE_ROOM; i++)
	{
		auto& pRoomInfo = g_pMain->m_TempleEventMonsterStoneRoomList[i];
		if (pRoomInfo.isnull() || pRoomInfo.Active || pRoomInfo.mUserList.size())
			continue;

		roomid = i;
		break;
	}

	if (roomid < 0 || roomid >= MAX_MONSTER_STONE_ROOM)
	{
		g_pMain->SendHelpDescription(this, "Bos Monster Stone odasi bulunamadi.");
		return true;
	}

	std::vector<_MONSTER_STONE_LIST_INFORMATION> mlist;
	g_pMain->m_MonsterStoneListInformationArray.m_lock.lock();
	foreach_stlmap_nolock(itr, g_pMain->m_MonsterStoneListInformationArray)
	{
		if (itr->second == nullptr || itr->second->ZoneID != bZoneID || itr->second->Family != bFamily)
			continue;

		mlist.push_back(*itr->second);
	}
	g_pMain->m_MonsterStoneListInformationArray.m_lock.unlock();

	if (mlist.empty())
	{
		g_pMain->SendHelpDescription(this, string_format("MONSTER_STONE_RESPAWN_LIST kaydi bulunamadi. Zone=%d Family=%d", bZoneID, bFamily));
		return true;
	}

	auto& pRoom = g_pMain->m_TempleEventMonsterStoneRoomList[roomid];
	if (pRoom.isnull() || pRoom.Active || pRoom.roomid != roomid)
	{
		g_pMain->SendHelpDescription(this, "Monster Stone odasi hazir degil.");
		return true;
	}

	pRoom.Active = true;
	pRoom.StartTime = UNIXTIME;
	pRoom.FinishTime = pRoom.StartTime + MONSTER_STONE_TIME;
	pRoom.StartZoneID = GetZoneID();
	pRoom.zoneid = bZoneID;
	pRoom.MonsterFamily = bFamily;
	pRoom.isBossKilled = false;
	pRoom.WaitingTime = 0;
	pRoom.bPartyMonsterStone = false;
	pRoom.mUserList.push_back(GetSocketID());

	foreach(aaa, mlist)
		g_pMain->SpawnEventNpc(aaa->sSid, aaa->bType == 0 ? true : false, aaa->ZoneID, aaa->X, aaa->Y, aaa->Z, aaa->sCount, 0,
			MONSTER_STONE_DEAD_TIME, 0, -1, pRoom.roomid + 1, aaa->byDirection, 1, 0, (uint16)SpawnEventType::MonsterStone, aaa->isBoss);

	if (pRoom.zoneid == ZONE_STONE1)
	{
		g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 204, 0, 201, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
		g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 204, 0, 197, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
		g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 204, 0, 193, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
	}
	else if (pRoom.zoneid == ZONE_STONE2)
	{
		g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 203, 0, 202, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
		g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 203, 0, 197, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
		g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 203, 0, 193, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
	}
	else if (pRoom.zoneid == ZONE_STONE3)
	{
		g_pMain->SpawnEventNpc(16062, false, (uint8)pRoom.zoneid, 204, 0, 207, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
		g_pMain->SpawnEventNpc(12117, false, (uint8)pRoom.zoneid, 204, 0, 200, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
		g_pMain->SpawnEventNpc(31508, false, (uint8)pRoom.zoneid, 204, 0, 194, 1, 0, MONSTER_STONE_DEAD_TIME, 3, -1, pRoom.roomid + 1, 0, 0, 0);
	}

	if (!ZoneChange(pRoom.zoneid, 0.0f, 0.0f, pRoom.roomid + 1))
	{
		g_pMain->TempleMonsterStoneResetNpcs(pRoom.roomid, pRoom.zoneid);
		pRoom.reset();
		g_pMain->SendHelpDescription(this, "ZoneChange basarisiz. Monster Stone odasi iptal edildi.");
		return true;
	}

	m_ismsevent = true;
	MonsterStoneTimerScreen(MONSTER_STONE_TIME);
	g_pMain->SendHelpDescription(this, string_format("Monster Stone test baslatildi. Zone=%d Family=%d Room=%d Spawn=%d", bZoneID, bFamily, pRoom.roomid + 1, (int)mlist.size()));
	return true;
}
#pragma endregion

#pragma region CGameServerDlg::TempleMonsterStoneTimer()
void CGameServerDlg::TempleMonsterStoneTimer()
{
	for (int i = 0; i < MAX_MONSTER_STONE_ROOM; i++) {
		auto& pRoomInfo = g_pMain->m_TempleEventMonsterStoneRoomList[i];
		if (pRoomInfo.isnull() || !pRoomInfo.Active)
			continue;

		uint32 remtime = 0, waittime = 0;
		if (pRoomInfo.FinishTime > UNIXTIME)
			remtime = uint32(pRoomInfo.FinishTime - UNIXTIME);

		if (pRoomInfo.WaitingTime > UNIXTIME)
			waittime = uint32(pRoomInfo.WaitingTime - UNIXTIME);

		if ((!pRoomInfo.isBossKilled && !remtime) || (pRoomInfo.isBossKilled && !waittime)) {
			pRoomInfo.Active = false;
			TempleMonsterStoneAutoResetRoom(pRoomInfo);
		}
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleMonsterStoneAutoResetRoom(_MONSTER_STONE_INFO &pRoom)
void CGameServerDlg::TempleMonsterStoneAutoResetRoom(_MONSTER_STONE_INFO& pRoom)
{
	if (pRoom.isnull())
		return;

	foreach(aaakiki, pRoom.mUserList) {
		CUser* pUser = g_pMain->GetUserPtr(*aaakiki);
		if (pUser == nullptr || !pUser->isInGame() || !pUser->isInMonsterStoneZone())
			continue;

		pUser->m_ismsevent = false;
		pUser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f, 0);
	}

	TempleMonsterStoneResetNpcs(pRoom.roomid, pRoom.zoneid);
	pRoom.reset();
}
#pragma endregion

#pragma region CGameServerDlg::TempleMonsterStoneResetNpcs(int16 roomid, uint8 zoneid)
void CGameServerDlg::TempleMonsterStoneResetNpcs(int16 roomid, uint8 zoneid)
{
	if (roomid == -1)
		return;

	CNpcThread* zoneitrThread = g_pMain->m_arNpcThread.GetData(zoneid);
	if (zoneitrThread == nullptr)
		return;

	foreach_stlmap(itr, zoneitrThread->m_arNpcArray) {
		CNpc* pNpc = TO_NPC(itr->second);
		if (pNpc == nullptr || pNpc->isDead() || pNpc->GetZoneID() != zoneid || pNpc->GetEventRoom() != (roomid + 1))
			continue;

		pNpc->Dead();
	}
}
#pragma endregion

#pragma region CGameServerDlg::TempleMonsterStoneItemExitRoom()
void CUser::TempleMonsterStoneItemExitRoom()
{
	if (GetEventRoom() < 1 || GetEventRoom() > MAX_TEMPLE_QUEST_EVENT_ROOM)
		return;

	auto& pRoomInfo = g_pMain->m_TempleEventMonsterStoneRoomList[GetEventRoom() - 1];
	if (pRoomInfo.isnull() || !pRoomInfo.Active)
		return;

	m_bEventRoom = 0;
	m_ismsevent = false;
	g_pMain->TempleMonsterStoneResetNpcs(pRoomInfo.roomid, pRoomInfo.zoneid);
	pRoomInfo.reset();
}
#pragma endregion


#pragma region CNpc::MonsterStoneKillProcess(CUser *pUser)
void CNpc::MonsterStoneKillProcess(CUser* pUser)
{
	if (pUser == nullptr || e_stype != e_summontype::m_MonsterStoneBoss || GetEventRoom() < 1 || GetEventRoom() > MAX_MONSTER_STONE_ROOM)
		return;

	auto& pRoomInfo = g_pMain->m_TempleEventMonsterStoneRoomList[GetEventRoom() - 1];
	if (pRoomInfo.isnull() || !pRoomInfo.Active || pRoomInfo.isBossKilled)
		return;

	pRoomInfo.isBossKilled = true;
	pRoomInfo.WaitingTime = UNIXTIME + MONSTER_STONE_FINISH_TIME;
	Packet result(WIZ_EVENT);
	result << uint8(TEMPLE_EVENT_FINISH) << uint8(0x11) << uint8(0x00) << uint8(0x65) << uint8(0x14) << uint32(0x00);
	pUser->Send(&result);

	result.clear();
	result.Initialize(WIZ_QUEST);
	result << uint8(2) << uint16(209) << uint8(0);
	pUser->Send(&result);
}
#pragma endregion

#pragma region CUser::MonsterStoneTimerScreen(uint16 Time)
void CUser::MonsterStoneTimerScreen(uint16 Time)
{
	bool bIsNeutralZone = (GetZoneID() >= ZONE_STONE1 && GetZoneID() <= ZONE_STONE3);

	if (!bIsNeutralZone || !isInGame())
		return;

	Packet result;

	result.Initialize(WIZ_BIFROST);
	result << uint8(MONSTER_SQUARD) << uint16(Time);
	Send(&result);

	result.clear();
	result.Initialize(WIZ_SELECT_MSG);
	result << uint16(0) << uint8(7) << uint64(0) << uint8(9) << uint16(0) << uint8(0) << uint8(11) << uint16(Time) << uint16(0);
	Send(&result);
}
#pragma endregion