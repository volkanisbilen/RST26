#include "stdafx.h"
#include "MagicInstance.h"

int8 CGameServerDlg::wantedgetroom(uint8 zoneid) {
	if (zoneid == ZONE_RONARK_LAND) return (int8)0;
	else if (zoneid == ZONE_ARDREAM) return (int8)1;
	else if (zoneid == ZONE_RONARK_LAND_BASE) return (int8)2;
	return (int8)-1;
}

int8 CGameServerDlg::wantedgetzone(uint8 room) {
	if (room == 0) return (int8)ZONE_RONARK_LAND;
	else if (room == 1) return (int8)ZONE_ARDREAM;
	else if (room == 2) return (int8)ZONE_RONARK_LAND_BASE;
	return (int8)-1;
}

bool invalidmap[3] = { true,true,true }, check = false;

void invcheckk() {
	for (int i = 0; i < 3; i++) {
		int8 zoneid = g_pMain->wantedgetzone(i);
		if (zoneid == -1) continue;
		auto* itr = g_pMain->GetZoneByID(zoneid);
		if (itr && !itr->m_Status) invalidmap[i] = false;
	}
}

void CGameServerDlg::NewWantedEventMainTimer()
{
	if (!g_pMain->pServerSetting.AutoWanted) return;

	if (!check) { check = true; invcheckk(); }

	for (int i = 0; i < 3; i++) {
		if (!invalidmap[i]) continue;
		auto& pMain = pWantedMain[i];

		switch (pMain.status)
		{
		case wantedstatus::disable:
			if (pMain.nextselecttime && UNIXTIME > pMain.nextselecttime) NewWantedEventSelecting(i);
			break;
		case wantedstatus::invitation:
			if (pMain.invitationtime && UNIXTIME > pMain.invitationtime) {
				if (pMain.m_elmolist.empty() && pMain.m_karuslist.empty()) {
					pMain.status = wantedstatus::disable;
					NewWantedEventResetData(i);
					continue;
				}
				pMain.status = wantedstatus::list;
				pMain.listtime = UNIXTIME + 10;
				pMain.finishtime = UNIXTIME + (10 * MINUTE);
			}
			break;
		case wantedstatus::list:
			if (pMain.listtime && UNIXTIME > pMain.listtime) { pMain.status = wantedstatus::running; NewWantedEventUserListSend(i); }
			break;
		case wantedstatus::running:
			if (pMain.finishtime && UNIXTIME > pMain.finishtime) {
				pMain.status = wantedstatus::disable;
				NewWantedEventFinishing(i);
				NewWantedEventResetData(i);
			}
			break;
		}
	}
}

void CGameServerDlg::NewWantedEventFinishing(uint8 room)
{
	int8 zoneid = wantedgetzone(room);
	if (zoneid == (int8)-1) goto reset;

	auto& pMain = pWantedMain[room];

	foreach(itr, pMain.m_elmolist) {
		if (!itr->second.wanted || itr->second.pUser == nullptr || !itr->second.pUser->isInGame()) continue;
		itr->second.pUser->GiveItem("Wanted Event", 914052000, 1);
		itr->second.pUser->SendLoyaltyChange("Wanted Event", 300);
		itr->second.pUser->m_iswanted = false;
		itr->second.pUser->m_iswantedtime = 0;
		//itr->second.pUser->AchieveWarCountAdd(UserAchieveWarTypes::WantedRunRunRun, 0, nullptr);
		CMagicProcess::RemoveType4Buff((uint8)BuffType::BUFF_TYPE_REWARD_MASK, itr->second.pUser, false);
	}
	foreach(itr, pMain.m_karuslist) {
		if (!itr->second.wanted || itr->second.pUser == nullptr || !itr->second.pUser->isInGame()) continue;
		itr->second.pUser->GiveItem("Wanted Event", 914052000, 1);
		itr->second.pUser->SendLoyaltyChange("Wanted Event", 300);
		itr->second.pUser->m_iswanted = false;
		itr->second.pUser->m_iswantedtime = 0;
		//itr->second.pUser->AchieveWarCountAdd(UserAchieveWarTypes::WantedRunRunRun, 0, nullptr);
		CMagicProcess::RemoveType4Buff((uint8)BuffType::BUFF_TYPE_REWARD_MASK, itr->second.pUser, false);
	}
reset:
	CGameServerDlg::NewWantedEventResetData(room);
}

void CGameServerDlg::NewWantedEventUserListSend(uint8 room)
{
	int8 zoneid = wantedgetzone(room);
	if (zoneid == (int8)-1) return;

	auto& pMain = pWantedMain[room];
	if (pMain.status != wantedstatus::list) return;

	Packet result; uint8 counter = 0;
	result.Initialize(WIZ_VANGUARD);
	result << uint8(0x02) << uint8(0x01) << counter;

	foreach(itr, pMain.m_elmolist) {
		if (itr->second.pUser == nullptr) continue;
		result << itr->second.pUser->GetName();
		counter++;
	}
	result.put(2, counter);
	CGameServerDlg::Send_Zone(&result, zoneid, nullptr, (uint8)Nation::KARUS);

	counter = 0;
	result.clear();
	result.Initialize(WIZ_VANGUARD);
	result << uint8(0x02) << uint8(0x01) << counter;

	foreach(itr, pMain.m_karuslist) {
		if (itr->second.pUser == nullptr) continue;
		result << itr->second.pUser->GetName();
		counter++;
	}
	result.put(2, counter);
	CGameServerDlg::Send_Zone(&result, zoneid, nullptr, (uint8)Nation::ELMORAD);
}

void CGameServerDlg::NewWantedEventResetData(uint8 ZoneID) { pWantedMain[ZoneID].Initialize(); }

void CGameServerDlg::NewWantedEventSelecting(uint8 room)
{
	int8 zoneid = wantedgetzone(room);
	if (zoneid == (int8)-1) goto reset;
	auto& pMain = pWantedMain[room];
	pMain.m_elmolist.clear();
	pMain.m_karuslist.clear();

	{
		uint16 HumanIndex = 0, KarusIndex = 0;
		struct WantedList { uint16 nIndex; CUser* pUser; };
		std::vector<WantedList> SelectHumanList, SelectKarusList;

		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pUser = g_pMain->GetUserPtr(i);
			if (!pUser || !pUser->isInGame())
				continue;

			if (!pUser->isInPKZone() || pUser->GetZoneID() != zoneid || pUser->isDead()) continue;
			pUser->m_iswanted = false; pUser->m_iswantedtime = 0;
			if (pUser->GetNation() == (uint8)Nation::ELMORAD) SelectHumanList.push_back({ HumanIndex++, pUser });
			else SelectKarusList.push_back({ KarusIndex++, pUser });
		}

		if (SelectHumanList.empty() && SelectKarusList.empty()) goto reset;

		int SelectingElmoradUser[MAX_SELECTING_USER] = { 0 };
		if (!SelectHumanList.empty()) {
			for (int i = 0; i < MAX_SELECTING_USER; i++) {
				if (HumanIndex < MAX_SELECTING_USER) {
					if (HumanIndex <= i) break;
				}

				SelectingElmoradUser[i] = myrand(1, HumanIndex);
				for (int kontrol = 0; kontrol < i; kontrol++) {
					if (SelectingElmoradUser[kontrol] == SelectingElmoradUser[i]) {
						i--;
						break;
					}
				}
			}

			foreach(itr, SelectHumanList) {
				if (itr->pUser == nullptr || itr->pUser->GetZoneID() != zoneid || itr->pUser->isDead()) continue;
				for (int i = 0; i < MAX_SELECTING_USER; i++) {
					if (SelectingElmoradUser[i] == itr->nIndex) {
						Packet result(WIZ_VANGUARD, uint8(0x01));
						result << uint8(0x01) << uint8(0x01);
						itr->pUser->Send(&result);
					}
				}
			}
		}

		if (!SelectKarusList.empty()) {
			int RLandSelectingKarusUser[MAX_SELECTING_USER] = { 0 };
			for (int i = 0; i < MAX_SELECTING_USER; i++) {
				if (KarusIndex < MAX_SELECTING_USER) {
					if (KarusIndex <= i) break;
				}
				RLandSelectingKarusUser[i] = myrand(1, KarusIndex);
				for (int kontrol = 0; kontrol < i; kontrol++) {
					if (RLandSelectingKarusUser[kontrol] == RLandSelectingKarusUser[i]) {
						i--;
						break;
					}
				}
			}

			foreach(itr, SelectKarusList) {
				if (itr->pUser == nullptr || itr->pUser->GetZoneID() != zoneid || itr->pUser->isDead())continue;
				for (int i = 0; i < MAX_SELECTING_USER; i++) {
					if (RLandSelectingKarusUser[i] == itr->nIndex) {
						Packet result(WIZ_VANGUARD, uint8(0x01));
						result << uint8(0x01) << uint8(0x01);
						itr->pUser->Send(&result);
					}
				}
			}
		}
	}
	pMain.status = wantedstatus::invitation;
	pMain.invitationtime = UNIXTIME + 11;
	return;
reset:
	NewWantedEventResetData(room);
}

void CUser::WantedEventUserisMove()
{
	if (!isWantedUser() || isDead() || !isInPKZone()) return;
	int8 zoneid = g_pMain->wantedgetroom(GetZoneID());
	if (zoneid == (int8)-1 || zoneid < 0 || zoneid > 2) return;

	auto& pMain = g_pMain->pWantedMain[zoneid];
	if (pMain.status != wantedstatus::running) return;

	Guard lock(pMain.UserListLock);
	if ((Nation)GetNation() == Nation::ELMORAD) {
		auto itr = pMain.m_elmolist.find(GetSocketID());
		if (itr == pMain.m_elmolist.end() || !itr->second.pUser || !itr->second.wanted) return;
	}
	else {
		auto itr = pMain.m_karuslist.find(GetSocketID());
		if (itr == pMain.m_karuslist.end() || !itr->second.pUser || !itr->second.wanted) return;
	}

	if (g_pMain->m_WantedSystemMapShowTime != 0) {
		if (UNIXTIME - g_pMain->m_WantedSystemMapShowTime > 60) {
			Packet Move(WIZ_VANGUARD, uint8(0x02));
			Move << uint8(0x02) << uint8(0x01) << uint8(0x00) << uint16(GetX()) << uint16(GetZ()) << GetName();
			if (GetNation() == (uint8)Nation::ELMORAD)
				g_pMain->Send_Zone(&Move, GetZoneID(), nullptr, (uint8)Nation::KARUS);
			else
				g_pMain->Send_Zone(&Move, GetZoneID(), nullptr, (uint8)Nation::ELMORAD);
			g_pMain->m_WantedSystemMapShowTime = (uint32)UNIXTIME;
		}
		if (UNIXTIME - g_pMain->m_WantedSystemTownControlTime > 15) {
			if (GetNation() == (uint8)Nation::ELMORAD) {
				if (((GetX() < 699.0f && GetX() > 540.0f) && ((GetZ() < 984.0f && GetZ() > 875.0f)))) Warp(10080, 10103);
			}
			else {
				if (((GetX() < 1444.0f && GetX() > 1314.0f) && ((GetZ() < 1140.0f && GetZ() > 1020.0f)))) Warp(10080, 10103);
			}
			g_pMain->m_WantedSystemTownControlTime = (uint32)UNIXTIME;
		}
	}
}

void CUser::NewWantedEventLoqOut(Unit* pKiller)
{
	m_iswanted = false; m_iswantedtime = 0;
	CMagicProcess::RemoveType4Buff((uint8)BuffType::BUFF_TYPE_REWARD_MASK, this, false);
	int8 zoneid = g_pMain->wantedgetroom(GetZoneID());
	if (zoneid == (int8)-1 || zoneid < 0 || zoneid > 2) return;

	auto& pMain = g_pMain->pWantedMain[zoneid];
	if (pMain.status == wantedstatus::disable) return;

	Guard lock(pMain.UserListLock);
	if ((Nation)GetNation() == Nation::ELMORAD) {
		auto itr = pMain.m_elmolist.find(GetSocketID());
		if (itr == pMain.m_elmolist.end()) return;
		pMain.m_elmolist.erase(GetID());
	}
	else {
		auto itr = pMain.m_karuslist.find(GetSocketID());
		if (itr == pMain.m_karuslist.end()) return;
		pMain.m_karuslist.erase(GetID());
	}

	if (pKiller && pKiller->isPlayer() && TO_USER(pKiller)->isInGame()) {
		TO_USER(pKiller)->GiveItem("Wanted Event", 914052000, 1);
		TO_USER(pKiller)->SendLoyaltyChange("Wanted Event", isInParty() ? 160 : 80);
		//TO_USER(pKiller)->AchieveWarCountAdd(UserAchieveWarTypes::WantedKillKill, 0, nullptr);
	}
}

void CUser::WantedProcess(Packet& pkt)
{
	enum class VanguardOpcode { Register = 1 };
	uint8 opcode = pkt.read<uint8>();
	switch ((VanguardOpcode)opcode)
	{
	case VanguardOpcode::Register:
		WantedRegister(pkt);
		break;
	default:
		TRACE("VanGuard unhandled Packet %d", opcode);
		printf("VanGuard unhandled Packet %d", opcode);
		break;
	}
}

void CUser::WantedRegister(Packet& pkt)
{
	if (isWantedUser() || isDead()) return;
	int8 zoneid = g_pMain->wantedgetroom(GetZoneID());
	if (zoneid == (int8)-1 || zoneid < 0 || zoneid > 2) return;

	auto& pMain = g_pMain->pWantedMain[zoneid];
	if (pMain.status != wantedstatus::invitation) return;

	Guard lock(pMain.UserListLock);
	if ((Nation)GetNation() == Nation::ELMORAD) {
		auto itr = pMain.m_elmolist.find(GetSocketID());
		if (itr != pMain.m_elmolist.end()) return;
	}
	else {
		auto itr = pMain.m_karuslist.find(GetSocketID());
		if (itr != pMain.m_karuslist.end()) return;
	}

	if ((Nation)GetNation() == Nation::ELMORAD) pMain.m_elmolist.insert(std::make_pair(GetID(), _WANTED_USER(this, true)));
	else pMain.m_karuslist.insert(std::make_pair(GetID(), _WANTED_USER(this, true)));

	MagicInstance instance;
	instance.sCasterID = GetID();
	instance.sTargetID = GetID();
	instance.nSkillID = 302166;
	instance.bIsItemProc = true;
	instance.bIsRunProc = true;
	instance.sSkillCasterZoneID = GetZoneID();
	instance.Run();


	m_iswanted = true;
	m_iswantedtime = (uint32)UNIXTIME + 600;
}