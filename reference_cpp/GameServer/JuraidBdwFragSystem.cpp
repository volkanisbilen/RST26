#include "stdafx.h"
#include "MagicInstance.h"

#define ALTAR_OF_MANES	9840

#pragma region CGameServerDlg::BDWMonumentAltarTimer()
void CGameServerDlg::BDWMonumentAltarTimer()
{
	if (!isBorderDefenceWarActive())
		return;

	for (int i = 1; i <= MAX_TEMPLE_EVENT_ROOM; i++) {
		auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(i);
		if (!pRoomInfo || pRoomInfo->m_bFinished || !pRoomInfo->m_tAltarSpawn)
			continue;

		if (!pRoomInfo->m_tAltarSpawnTimed || UNIXTIME < pRoomInfo->m_tAltarSpawnTimed)
			continue;

		pRoomInfo->m_tAltarSpawnTimed = 0;
		BDWMonumentAltarRespawn(i);
	}
}
#pragma endregion

#pragma region CGameServerDlg::BDWMonumentAltarRespawn(uint16 bRoom)
void CGameServerDlg::BDWMonumentAltarRespawn(uint16 bRoom)
{
	if (!isBorderDefenceWarActive() || bRoom > MAX_TEMPLE_EVENT_ROOM)
		return;

	auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(bRoom);
	if (!pRoomInfo || pRoomInfo->m_bFinished || !pRoomInfo->m_tAltarSpawn || !pRoomInfo->pAltar)
		return;

	auto* pAltar = GetNpcPtr(pRoomInfo->pAltar, ZONE_BORDER_DEFENSE_WAR);
	if (!pAltar || !pAltar->isDead() || pAltar->GetEventRoom() != bRoom || pAltar->GetProtoID() != ALTAR_OF_MANES)
		return;

	Packet result(WIZ_EVENT, uint8(2));
	result << uint8(2);
	g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL, ZONE_BORDER_DEFENSE_WAR, true, bRoom);

	pRoomInfo->m_tAltarSpawn = false;

	pAltar->m_bForceReset = true;
	
	if (pAltar->isAlive())
		pAltar->HpChange(pAltar->GetMaxHealth(), nullptr);
}
#pragma endregion

#pragma region CUser::BDWMonumentPointProcess()
void CUser::BDWMonumentPointProcess()
{
	if (!isInValidRoom(0) || !isEventUser())
		return;

	bool inarea = false;
	if (GetNation() == (uint8)Nation::KARUS && (GetX() >= 28.0f && GetX() <= 35.0f && GetZ() >= 128.0f && GetZ() <= 135.0f))
		inarea = true;
	else if (GetNation() == (uint8)Nation::ELMORAD && (GetX() >= 220.0f && GetX() <= 227.0f && GetZ() >= 127.0f && GetZ() <= 135.0f))
		inarea = true;

	if (!inarea)
		return;

	auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
	if (!pRoomInfo || !g_pMain->isBorderDefenceWarActive() || !m_bHasAlterOptained)
		return;

	Packet result(WIZ_EVENT);
	result << uint8(TEMPLE_EVENT_ALTAR_TIMER) << uint16(60);
	g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL, ZONE_BORDER_DEFENSE_WAR, true, GetEventRoom());

	pRoomInfo->m_tAltarSpawnTimed = UNIXTIME + 60;
	pRoomInfo->m_tAltarSpawn = true;
	BDWAltarScreenAndPlayerPointChange();
	CMagicProcess::RemoveType4Buff((uint8)BuffType::BUFF_TYPE_FRAGMENT_OF_MANES, this, true, true);
	m_bHasAlterOptained = false;
}
#pragma endregion

#pragma region CUser::BDWUserLogOut()
void CUser::BDWUserLogOut() 
{
	if (!isInValidRoom(0))
		return;

	if (!isEventUser() || !g_pMain->isBorderDefenceWarActive() || GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
		return;

	auto *pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
	if (!pRoomInfo)
		return;

	bool Finished = false;
	if (!pRoomInfo->m_FinishPacketControl && pRoomInfo->m_bWinnerNation == 0) {
		if (GetNation() == (uint8)Nation::ELMORAD) {
			uint8 bcount = pRoomInfo->GetRoomElmoradUserCount();
			if (bcount > 0) bcount--;

			if (!bcount) {
				Finished = true;
				pRoomInfo->m_bWinnerNation = (uint8)Nation::KARUS;
			}

			pRoomInfo->m_ElmoradUserListLock.lock();
			pRoomInfo->m_ElmoradUserList.erase(GetName());
			pRoomInfo->m_ElmoradUserListLock.unlock();
		}
		else {
			uint8 bcount = pRoomInfo->GetRoomKarusUserCount();
			if (bcount > 0) bcount--;

			if (!bcount) {
				Finished = true;
				pRoomInfo->m_bWinnerNation = (uint8)Nation::ELMORAD;
			}

			pRoomInfo->m_KarusUserListLock.lock();
			pRoomInfo->m_KarusUserList.erase(GetName());
			pRoomInfo->m_KarusUserListLock.unlock();
		}
	}

	if (Finished) {
		pRoomInfo->m_FinishPacketControl = true;
		pRoomInfo->m_tFinishTimeCounter = UNIXTIME + 20;
		pRoomInfo->m_tAltarSpawn = false;
		pRoomInfo->m_tAltarSpawnTimed = 0;

		Packet newpkt1(WIZ_SELECT_MSG);
		newpkt1 << uint16(0) << uint8(7) << uint64(0);
		newpkt1 << uint32(8) << uint8(7) << uint32(500);
		Packet newpkt2(WIZ_EVENT);
		newpkt2 << uint8(TEMPLE_EVENT_FINISH)
			<< uint8(2) << uint8(0)
			<< uint8(pRoomInfo->m_bWinnerNation) << uint32(20);

		BDWSendPacket(newpkt1);
		BDWSendPacket(newpkt2);
	}
	if (m_bHasAlterOptained) BDWUserHasObtainedLoqOut();
}
#pragma endregion

#pragma region CUser::BDWUserHasObtainedLoqOut()
void CUser::BDWUserHasObtainedLoqOut()
{
	if (!isInValidRoom(0) || !g_pMain->isBorderDefenceWarActive() || !m_bHasAlterOptained)
		return;

	auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
	if (!pRoomInfo)
		return;

	pRoomInfo->m_tAltarSpawnTimed = UNIXTIME + 60;
	pRoomInfo->m_tAltarSpawn = true;
	m_bHasAlterOptained = false;

	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT_ALTAR_TIMER));
	result << uint16(60);

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

		pUser->Send(&result);
	}

	pRoomInfo->m_ElmoradUserListLock.lock();
	_BDW_ROOM_INFO::ElmoradUserList copyeuser = pRoomInfo->m_ElmoradUserList;
	pRoomInfo->m_ElmoradUserListLock.unlock();
	foreach(itrUser1, copyeuser) {
		auto& pEventUser = itrUser1->second;
		if (pEventUser.strUserID.empty() || pEventUser.isLoqOut)
			continue;

		CUser* pUser = g_pMain->GetUserPtr(pEventUser.strUserID, NameType::TYPE_CHARACTER);
		if (pUser == nullptr || !pUser->isInGame() || !pUser->isEventUser() || pUser->GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
			continue;

		pUser->Send(&result);
	}
}
#pragma endregion

#pragma region CUser::BDWSendPacket(Packet& pkt)
void CUser::BDWSendPacket(Packet& pkt)
{
	if (!isInValidRoom(0))
		return;

	auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
	if (!pRoomInfo)
		return;

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

		pUser->Send(&pkt);
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

		pUser->Send(&pkt);
	}
}
#pragma endregion

#pragma region CUser::BDWAltarScreenAndPlayerPointChange()
void CUser::BDWAltarScreenAndPlayerPointChange()
{
	if (!isInValidRoom(0) || !g_pMain->isBorderDefenceWarActive())
		return;

	auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
	if (!pRoomInfo || pRoomInfo->m_FinishPacketControl)
		return;

	Nation bNation = (Nation)GetNation();
	uint16 e_count = pRoomInfo->GetRoomElmoradUserCount(), k_count = pRoomInfo->GetRoomKarusUserCount();
	uint16 t_count = e_count + k_count;

	if (bNation == Nation::ELMORAD && e_count) 
		pRoomInfo->m_ElmoScoreBoard += 10 * e_count;
	else if (bNation == Nation::KARUS && k_count)
		pRoomInfo->m_KarusScoreBoard += 10 * k_count;

	bool finished = false;
	bNation == Nation::ELMORAD ? pRoomInfo->m_iElmoMonuCount += 1 : pRoomInfo->m_iKarusMonuCount += 1;

	if (pRoomInfo->m_bWinnerNation == 0) {
		uint16 TotalPoint = 0;
		if (t_count >= 16) TotalPoint = 400;
		else if (t_count >= 10) TotalPoint = 200;
		else if (t_count >= 5) TotalPoint = 140;
		else TotalPoint = 50;

		if (pRoomInfo->m_ElmoScoreBoard >= TotalPoint)
			pRoomInfo->m_bWinnerNation = (uint8)Nation::ELMORAD;
		else if (pRoomInfo->m_KarusScoreBoard >= TotalPoint)
			pRoomInfo->m_bWinnerNation = (uint8)Nation::KARUS;

		if (pRoomInfo->m_bWinnerNation != 0) finished = true;
	}

	if (finished) {
		pRoomInfo->m_FinishPacketControl = true;
		pRoomInfo->m_tFinishTimeCounter = UNIXTIME + 20;
		pRoomInfo->m_tAltarSpawn = false;
		pRoomInfo->m_tAltarSpawnTimed = 0;
	}

	Packet newpkt1(WIZ_SELECT_MSG);
	newpkt1 << uint16(0) << uint8(7) << uint64(0);
	newpkt1 << uint32(8) << uint8(7) << uint32(500);
	Packet newpkt2(WIZ_EVENT);
	newpkt2 << uint8(TEMPLE_EVENT_FINISH)
		<< uint8(2) << uint8(0)
		<< uint8(pRoomInfo->m_bWinnerNation) << uint32(20);

	Packet pointpacket(WIZ_EVENT, uint8(TEMPLE_SCREEN));
	pointpacket << uint32(pRoomInfo->m_KarusScoreBoard) << uint32(pRoomInfo->m_ElmoScoreBoard);

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

		if (bNation == Nation::KARUS) {
			pUser->m_BorderDefenceWarUserPoint += 10;
			pUser->UpdateBorderDefenceWarRank();
		}
		pUser->Send(&pointpacket);
		if (finished) {
			pUser->Send(&newpkt1);
			pUser->Send(&newpkt2);
		}
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

		if (bNation == Nation::ELMORAD) {
			pUser->m_BorderDefenceWarUserPoint += 10;
			pUser->UpdateBorderDefenceWarRank();
		}
		pUser->Send(&pointpacket);
		if (finished) {
			pUser->Send(&newpkt1);
			pUser->Send(&newpkt2);
		}
	}
}
#pragma endregion

#pragma region CNpc::BDWMonumentAltarSystem(CUser *pUser)
void CNpc::BDWMonumentAltarSystem(CUser *pUser)
{
	if (pUser == nullptr || !pUser->isInValidRoom(0) || !g_pMain->isBorderDefenceWarActive())
		return;

	if (GetZoneID() != ZONE_BORDER_DEFENSE_WAR || pUser->GetEventRoom() != GetEventRoom())
		return;

	auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(pUser->GetEventRoom());
	if (!pRoomInfo)
		return;

	pUser->m_bHasAlterOptained = true;
	MagicInstance instance;
	instance.sCasterID = pUser->GetSocketID();
	instance.sTargetID = pUser->GetSocketID();
	instance.sSkillCasterZoneID = pUser->GetZoneID();
	instance.nSkillID = 492063;
	instance.bIsRunProc = true;
	instance.Run();

	Packet result(WIZ_EVENT, uint8(TEMPLE_EVENT_ALTAR_FLAG));
	result.SByte();
	result << pUser->GetName() << pUser->GetNation();
	g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL, ZONE_BORDER_DEFENSE_WAR, true, GetEventRoom());
}
#pragma endregion

#pragma region CUser::BDWUpdateRoomKillCount()
void CUser::BDWUpdateRoomKillCount()
{
	if (!g_pMain->isBorderDefenceWarActive() || !isInValidRoom(0))
		return;

	auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
	if (!pRoomInfo || pRoomInfo->m_FinishPacketControl)
		return;

	Nation bNation = (Nation)GetNation();
	uint16 e_count = pRoomInfo->GetRoomElmoradUserCount(), k_count = pRoomInfo->GetRoomKarusUserCount();
	uint16 t_count = e_count + k_count;

	if (bNation == Nation::ELMORAD && e_count)
		pRoomInfo->m_ElmoScoreBoard += 1 * e_count;
	else if (bNation == Nation::KARUS && k_count)
		pRoomInfo->m_KarusScoreBoard += 1 * k_count;

	bNation == Nation::ELMORAD ? pRoomInfo->m_iElmoradKillCount += 1 : pRoomInfo->m_iKarusKillCount += 1;
	m_BorderDefenceWarUserPoint += 1;
	UpdateBorderDefenceWarRank();

	bool finished = false;
	if (pRoomInfo->m_bWinnerNation == 0) {

		uint16 TotalPoint = 0;
		if (t_count >= 16) TotalPoint = 400;
		else if (t_count >= 10) TotalPoint = 200;
		else if (t_count >= 5) TotalPoint = 140;
		else TotalPoint = 50;

		if (pRoomInfo->m_ElmoScoreBoard >= TotalPoint)
			pRoomInfo->m_bWinnerNation = (uint8)Nation::ELMORAD;
		else if (pRoomInfo->m_KarusScoreBoard >= TotalPoint)
			pRoomInfo->m_bWinnerNation = (uint8)Nation::KARUS;

		if (pRoomInfo->m_bWinnerNation != 0) finished = true;
	}

	if (finished) {

		pRoomInfo->m_FinishPacketControl = true;
		pRoomInfo->m_tFinishTimeCounter = UNIXTIME + 20;
		pRoomInfo->m_tAltarSpawn = false;
		pRoomInfo->m_tAltarSpawnTimed = 0;
	}

	Packet newpkt1(WIZ_SELECT_MSG);
	newpkt1 << uint16(0) << uint8(7) << uint64(0);
	newpkt1 << uint32(8) << uint8(7) << uint32(500);
	Packet newpkt2(WIZ_EVENT);
	newpkt2 << uint8(TEMPLE_EVENT_FINISH)
		<< uint8(2) << uint8(0)
		<< uint8(pRoomInfo->m_bWinnerNation) << uint32(20);

	Packet pointpacket(WIZ_EVENT, uint8(TEMPLE_SCREEN));
	pointpacket << uint32(pRoomInfo->m_KarusScoreBoard) << uint32(pRoomInfo->m_ElmoScoreBoard);

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

		pUser->Send(&pointpacket);
		if (finished) {
			pUser->Send(&newpkt1);
			pUser->Send(&newpkt2);
		}
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

		pUser->Send(&pointpacket);
		if (finished) {
			pUser->Send(&newpkt1);
			pUser->Send(&newpkt2);
		}
	}
}
#pragma endregion

#pragma region CUser::JRUpdateRoomKillCount()
void CUser::JRUpdateRoomKillCount()
{
	if (!isInValidRoom(0) || !g_pMain->isJuraidMountainActive())
		return;

	auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(GetEventRoom());
	if (!pRoomInfo || pRoomInfo->m_FinishPacketControl)
		return;

	uint8 nation = 0;
	GetNation() == (uint8)Nation::ELMORAD ? nation = 2 : nation = 1;

	nation == (uint8)Nation::ELMORAD ? pRoomInfo->m_iElmoradKillCount++ : pRoomInfo->m_iKarusKillCount++;

	// Send the new score to the room users.
	Packet result(WIZ_EVENT);
	result << uint8(TEMPLE_SCREEN) << uint32(pRoomInfo->m_iKarusKillCount) << uint32(pRoomInfo->m_iElmoradKillCount);
	g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL, ZONE_JURAID_MOUNTAIN, true, GetEventRoom());
}
#pragma endregion

bool CUser::virt_eventattack_check() {

	if (!isInTempleEventZone() || !isInValidRoom(0))
		return true;

	if (GetZoneID() == ZONE_BORDER_DEFENSE_WAR) {
		if (!g_pMain->isBorderDefenceWarActive())
			return false;

		auto* pRoom = g_pMain->m_TempleEventBDWRoomList.GetData(GetEventRoom());
		if (!pRoom || pRoom->m_FinishPacketControl || pRoom->m_bFinished)// DeaFSezo bdw bittiði anda birbirine atack yapýlýyordu ve bunun sonucunda yenen taraftaki adam ölürse ödül alamýyordu bitti anda atack yapýlmasý kapatýl 28.12.2024
			return false;
	}
	else if (GetZoneID() == ZONE_JURAID_MOUNTAIN) {
		if (!g_pMain->isJuraidMountainActive())
			return false;

		auto* pRoom = g_pMain->m_TempleEventJuraidRoomList.GetData(GetEventRoom());
		if (!pRoom || pRoom->m_bFinished)
			return false;
	}
	else if (GetZoneID() == ZONE_CHAOS_DUNGEON) {
		if (!g_pMain->isChaosExpansionActive())
			return false;

		auto* pRoom = g_pMain->m_TempleEventChaosRoomList.GetData(GetEventRoom());
		if (!pRoom || pRoom->m_bFinished)
			return false;
	}

	return true;
}

#pragma region CUser::JuraidMountainWarp()
void CUser::JuraidMountainWarp()
{
	if (GetZoneID() != ZONE_JURAID_MOUNTAIN || !isEventUser() || !isInValidRoom(0))
		goto table_pos;

	auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(GetEventRoom());
	if (!pRoomInfo)
		return;

	float x = 0.0f, z = 0.0f; bool test = false;
	if (isDevaStage(pRoomInfo)) {
		if ((Nation)GetNation() == Nation::KARUS)
		{
			x = float(511 + myrand(0, 3)), z = float(738 + myrand(0, 3));
		}
		else
		{
			x = float(511 + myrand(0, 3)), z = float(281 + myrand(0, 3));
		}
		test = true;
	}
	else if (isBridgeStage2(pRoomInfo)) {
		if ((Nation)GetNation() == Nation::KARUS)
		{
			x = float(336 + myrand(0, 3)), z = float(848 + myrand(0, 3));
		}
		else
		{
			x = float(695 + myrand(0, 3)), z = float(171 + myrand(0, 3));
		}
		test = true;
	}
	else if (isBridgeStage1(pRoomInfo)) {
		if ((Nation)GetNation() == Nation::KARUS)
		{
			x = float(224 + myrand(0, 3)), z = float(671 + myrand(0, 3));
		}
		else {
			x = float(800 + myrand(0, 3)), z = float(349 + myrand(0, 3));
		}
		test = true;
	}
	else
		goto table_pos;

	if (test)
		Warp(uint16(x * 10), uint16(z * 10));

	return;
table_pos:
	auto* pos = g_pMain->m_StartPositionArray.GetData(GetZoneID());
	if (pos) Warp((uint16)((GetNation() == (uint8)Nation::KARUS ? pos->sKarusX : pos->sElmoradX) + myrand(0, pos->bRangeX)) * 10,
		(uint16)((GetNation() == (uint8)Nation::KARUS ? pos->sKarusZ : pos->sElmoradZ) + myrand(0, pos->bRangeZ)) * 10);
}
#pragma endregion

#pragma region 	CUser::isDevaStage(_JURAID_ROOM_INFO* pRoomInfo)
bool CUser::isDevaStage(_JURAID_ROOM_INFO* pRoomInfo)
{
	if (!pRoomInfo ||!isEventUser())
		return false;
	
	if (GetNation() == (uint8)Nation::KARUS && pRoomInfo->m_iKarusMainMonsterKill >= 12 && pRoomInfo->m_iKarusSubMonsterKill >= 60)
		return true;
	else if (GetNation() == (uint8)Nation::ELMORAD && pRoomInfo->m_iElmoMainMonsterKill >= 12 && pRoomInfo->m_iElmoSubMonsterKill >= 60)
		return true;
	
	return false;
}
#pragma endregion

#pragma region 	CUser::isBridgeStage1(_JURAID_ROOM_INFO* pRoomInfo)
bool CUser::isBridgeStage1(_JURAID_ROOM_INFO* pRoomInfo)
{
	if (!pRoomInfo ||!isEventUser()) 
		return false;

	if (GetNation() == (uint8)Nation::KARUS
		&& pRoomInfo->m_iKarusMainMonsterKill >= 4 && pRoomInfo->m_iKarusSubMonsterKill >= 20
		&& pRoomInfo->m_iKarusMainMonsterKill <= 8 && pRoomInfo->m_iKarusSubMonsterKill < 40)
		return true;
	else if (GetNation() == (uint8)Nation::ELMORAD
		&& pRoomInfo->m_iElmoMainMonsterKill >= 4 && pRoomInfo->m_iElmoSubMonsterKill >= 20
		&& pRoomInfo->m_iElmoMainMonsterKill <= 8 && pRoomInfo->m_iElmoSubMonsterKill < 40)
		return true;
	return false;
}
#pragma endregion

#pragma region 	CUser::isBridgeStage2(_JURAID_ROOM_INFO* pRoomInfo)
bool CUser::isBridgeStage2(_JURAID_ROOM_INFO* pRoomInfo)
{
	if (!pRoomInfo || !isEventUser())
		return false;

	if (GetNation() == (uint8)Nation::KARUS
		&& pRoomInfo->m_iKarusMainMonsterKill >= 8 && pRoomInfo->m_iKarusSubMonsterKill >= 40
		&& pRoomInfo->m_iKarusMainMonsterKill <= 12 && pRoomInfo->m_iKarusSubMonsterKill < 60)
		return true;
	else if (GetNation() == (uint8)Nation::ELMORAD
		&& pRoomInfo->m_iElmoMainMonsterKill >= 8 && pRoomInfo->m_iElmoSubMonsterKill >= 40
		&& pRoomInfo->m_iElmoMainMonsterKill <= 12 && pRoomInfo->m_iElmoSubMonsterKill < 60)
		return true;
	return false;
}
#pragma endregion