#include "stdafx.h"

#pragma region CGameServerDlg::csw_maintimer
void CGameServerDlg::csw_maintimer() {
	if (!isCswActive())
		return;

	uint32 r_time = 0;
	if (pCswEvent.CswTime > UNIXTIME)
		r_time = uint32(pCswEvent.CswTime - UNIXTIME);

	if (pCswEvent.Status == CswOpStatus::Preparation && !pCswEvent.prepare_check) {
		if (r_time) {
			if (r_time == 10 * MINUTE) csw_remnotice((uint8)CswNotice::Preparation, 10);
			else if (r_time == 5 * MINUTE) csw_remnotice((uint8)CswNotice::Preparation, 5);
			else if (r_time == 4 * MINUTE) csw_remnotice((uint8)CswNotice::Preparation, 4);
			else if (r_time == 3 * MINUTE) csw_remnotice((uint8)CswNotice::Preparation, 3);
			else if (r_time == 2 * MINUTE) csw_remnotice((uint8)CswNotice::Preparation, 2);
			else if (r_time == 1 * MINUTE) csw_remnotice((uint8)CswNotice::Preparation, 1);
		}
		else {
			pCswEvent.prepare_check = true;
			csw_waropen();
		}
	}
	else if (pCswEvent.Status == CswOpStatus::War && !pCswEvent.war_check) {
		if (r_time) {
			if (r_time == 30 * MINUTE) csw_remnotice((uint8)CswNotice::War, 30);
			else if (r_time == 10 * MINUTE) csw_remnotice((uint8)CswNotice::War, 10);
			else if (r_time == 5 * MINUTE) csw_remnotice((uint8)CswNotice::War, 5);
			else if (r_time == 4 * MINUTE) csw_remnotice((uint8)CswNotice::War, 4);
			else if (r_time == 3 * MINUTE) csw_remnotice((uint8)CswNotice::War, 3);
			else if (r_time == 2 * MINUTE) csw_remnotice((uint8)CswNotice::War, 2);
			else if (r_time == 1 * MINUTE) csw_remnotice((uint8)CswNotice::War, 1);
		}
		else {
			pCswEvent.war_check = true;
			csw_close();
		}
	}
}
#pragma endregion

#pragma region CGameServerDlg::csw_close()
void CGameServerDlg::csw_close() {

	if (!isCswActive())
		return;

	m_byBattleOpen = m_byOldBattleOpen = NO_BATTLE;
	csw_reset();
	csw_usertools(true, CswNotice::CswFinish, true, false, false, true, true);
	ResetAllEventObject(ZONE_DELOS);
}
#pragma endregion

#pragma region CGameServerDlg::csw_reset()
void CGameServerDlg::csw_reset() {
	pCswEvent.Status = CswOpStatus::NotOperation;
	pCswEvent.CswTime = 0;
	pCswEvent.Started = false;
	pCswEvent.MonumentTime = 0;
	pCswEvent.war_check = pCswEvent.prepare_check = false;
}
#pragma endregion

#pragma region CGameServerDlg::csw_remnotice(uint8 NoticeType,int32 Time)
void CGameServerDlg::csw_remnotice(uint8 NoticeType, int32 Time)
{
	uint32 a = 0;
	 if ((CswNotice)NoticeType == CswNotice::Preparation)
		a = IDS_SIEGE_WAR_READY_TIME_NOTICE;
	else if ((CswNotice)NoticeType == CswNotice::War)
		a = IDS_SIEGE_WAR_TIME_NOTICE;

	if (!a)
		return;

	std::string notice = "";

	g_pMain->GetServerResource(a, &notice, Time);

	Packet pkt;
	g_pMain->GetServerResource(IDP_ANNOUNCEMENT, &notice, notice.c_str());
	ChatPacket::Construct(&pkt, (uint8)ChatType::WAR_SYSTEM_CHAT, &notice);

	for (int i = 0; i < MAX_USER; i++){
		CUser* pUser = GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		/*if (pUser->GetZoneID() != ZONE_DELOS 
			&& (CswNotice)NoticeType != CswNotice::Preparation)
			continue;*/

		pUser->Send(&pkt);
	}
}
#pragma endregion

#pragma region CGameServerDlg::csw_prepareopen
void CGameServerDlg::csw_prepareopen() {

	if (isCswActive())
		return;

	ResetAllEventObject(ZONE_DELOS);
	g_pMain->pCswEvent.mClanListLock.lock();
	g_pMain->pCswEvent.mClanList.clear();
	g_pMain->pCswEvent.mClanListLock.unlock();
	pCswEvent.Status = CswOpStatus::Preparation;
	pCswEvent.CswTime = UNIXTIME + (pCswEvent.poptions.Preparing * MINUTE);
	pCswEvent.Started = true;
	csw_usertools(true, CswNotice::Preparation, true, true, false, false);
}
#pragma endregion

#pragma region CGameServerDlg::csw_waropen
void CGameServerDlg::csw_waropen() {

	if (!isCswActive())
		return;

	m_byBattleOpen = m_byOldBattleOpen = SIEGE_BATTLE;
	pCswEvent.Status = CswOpStatus::War;
	pCswEvent.CswTime = UNIXTIME + (pCswEvent.poptions.wartime * MINUTE);
	csw_usertools(true, CswNotice::War, true, false, true, true);
}
#pragma endregion

#pragma region CGameServerDlg::csw_usertools(bool Notice, CswNotice s, bool Flag, bool KickOutPreapre, bool KickOutWar, bool town, bool reward)
void CGameServerDlg::csw_usertools(bool Notice, CswNotice s, bool Flag, bool KickOutPreapre, bool KickOutWar, bool town, bool reward)
{
	CKnights* pOwner = nullptr;
	if (Flag && g_pMain->pSiegeWar.sMasterKnights)
		pOwner = g_pMain->GetClanPtr(g_pMain->pSiegeWar.sMasterKnights);

	for (int i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (reward && pUser->GetZoneID() == ZONE_DELOS)
		{
			if (g_pMain->pSiegeWar.sMasterKnights == pUser->GetClanID())
			{
				for (uint8 i = 0; i < 3; i++) {
					if (!pCswEvent.poptions.itemid[i] || !pCswEvent.poptions.itemcount[i])
						continue;

					pUser->GiveItem("Csw winner knight", pCswEvent.poptions.itemid[i],
						pCswEvent.poptions.itemcount[i], true, pCswEvent.poptions.itemtime[i]);
				}

				if (pCswEvent.poptions.cash || pCswEvent.poptions.tl)
					pUser->GiveBalance(pCswEvent.poptions.cash, pCswEvent.poptions.tl);

				if (pCswEvent.poptions.money)
					pUser->GoldGain(pCswEvent.poptions.money, true);

				if (pCswEvent.poptions.loyalty)
					pUser->SendLoyaltyChange("csw winner knight", pCswEvent.poptions.loyalty, false, false, true);
			}
		}

		if (Notice && s == CswNotice::MonumentKilled && pUser->GetZoneID() == ZONE_DELOS)
			pUser->csw_notice(s);
		else if (Notice && s == CswNotice::Preparation)
			pUser->csw_notice(s);
		else if (Notice && s == CswNotice::War && pUser->GetZoneID() == ZONE_DELOS)
			pUser->csw_notice(s);
		else if (Notice && s == CswNotice::CswFinish)
			pUser->csw_notice(s);

		if (Flag && pUser->GetZoneID() == ZONE_DELOS) 
			pUser->csw_sendwarflag(pOwner);

		if (KickOutPreapre
			&& (pUser->GetZoneID() == ZONE_DELOS || pUser->GetZoneID() == ZONE_HELL_ABYSS || pUser->GetZoneID() == ZONE_DESPERATION_ABYSS))
			pUser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f);

		if (KickOutWar && pUser->isInPKZone())
			pUser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f);

		if (town && pUser->GetZoneID() == ZONE_DELOS)
			pUser->csw_town();
	}
}
#pragma endregion

#pragma region CUser::csw_sendwarflag(CKnights* pOwner,bool finish)
void CUser::csw_sendwarflag(CKnights* pKnights, bool finish)
{
	CKnights* pOwner = pKnights;
	if (g_pMain->pSiegeWar.sMasterKnights)
		pOwner = g_pMain->GetClanPtr(g_pMain->pSiegeWar.sMasterKnights);

	Packet result(WIZ_SIEGE, uint8(2));
	result.SByte();
	result << uint8(0);
	if (pOwner)
		result << pOwner->GetID() << pOwner->m_sMarkVersion << pOwner->m_byFlag << pOwner->m_byGrade;
	else
		result << uint32(0) << uint16(0);
	Send(&result);



	uint32 maintime = 0;
	if (g_pMain->pCswEvent.CswTime > UNIXTIME)
		maintime = uint32(g_pMain->pCswEvent.CswTime - UNIXTIME);

	enum class csw_flags
	{
		timer,
		finish
	};

	Packet newpkt(XSafe, uint8(XSafeOpCodes::CSW));
	newpkt.SByte();
	if (!finish && maintime)
	{
		newpkt << uint8(csw_flags::timer) << maintime << std::string(pOwner ? pOwner->GetName() : "") << (uint8)g_pMain->pCswEvent.Status;
		if (g_pMain->pCswEvent.Status == CswOpStatus::Preparation)
			newpkt << g_pMain->pCswEvent.poptions.Preparing;
		else
			newpkt << g_pMain->pCswEvent.poptions.wartime;
	}

	else
		newpkt << uint8(csw_flags::finish);
	Send(&newpkt);

	//if (g_pMain->isCswActive() || finish)
	//{
	//	Packet newpkt(XSafe, uint8(XSafeOpCodes::CSW));
	//	newpkt << (finish ? uint8(0) : uint8(1));

	//	if (!finish) {
	//		uint32 maintime = 0;
	//		if (g_pMain->pCswEvent.CswTime > UNIXTIME)
	//			maintime = uint32(g_pMain->pCswEvent.CswTime - UNIXTIME);
	//		if (maintime)
	//			newpkt << maintime << (pOwner ? pOwner->GetName() : "");
	//	}
	//	Send(&newpkt);
	//}
}
#pragma endregion

#pragma region CUser::csw_notice(CswNotice p)
void CUser::csw_notice(CswNotice p)
{
	std::string notice = ""; Packet pkt;

	CKnights* pKnights = nullptr;
	if ((p == CswNotice::MonumentKilled || p == CswNotice::CswFinish) && g_pMain->pSiegeWar.sMasterKnights)
		pKnights = g_pMain->GetClanPtr(g_pMain->pSiegeWar.sMasterKnights);

	switch (p)
	{
	case CswNotice::Preparation:
		g_pMain->GetServerResource(IDS_SIEGE_WAR_READY_TIME_NOTICE, &notice, g_pMain->pCswEvent.poptions.Preparing);
		break;
	case CswNotice::MonumentKilled:
		g_pMain->GetServerResource(IDS_NPC_GUIDON_DESTORY, &notice, pKnights == nullptr ? "***" : pKnights->GetName().c_str());
		break;
	case CswNotice::War:
		g_pMain->GetServerResource(IDS_SIEGE_WAR_TIME_NOTICE, &notice, g_pMain->pCswEvent.poptions.wartime);
		break;
	case CswNotice::CswFinish:
		if (pKnights)
			g_pMain->GetServerResource(IDS_SIEGE_WAR_VICTORY, &notice, pKnights->GetName().c_str());
		else
			g_pMain->GetServerResource(IDS_SIEGE_WAR_END, &notice);
		break;
	}
	
	if (notice.empty()) 
		return;

	g_pMain->GetServerResource(IDP_ANNOUNCEMENT, &notice, notice.c_str());
	ChatPacket::Construct(&pkt, (uint8)ChatType::WAR_SYSTEM_CHAT, &notice);
	Send(&pkt);
}
#pragma endregion

#pragma region CUser::csw_canenterdelos()
bool CUser::csw_canenterdelos()
{
	bool isWinnerClan = false;
	if (isInClan() && g_pMain->pSiegeWar.sMasterKnights > 0)
	{
		auto* pClan = g_pMain->GetClanPtr(GetClanID());
		if (pClan != nullptr)
		{
			isWinnerClan = (GetClanID() == g_pMain->pSiegeWar.sMasterKnights
				|| pClan->GetAllianceID() == g_pMain->pSiegeWar.sMasterKnights);
		}
	}

	if (g_pMain->isCswActive()) {
		if (!isInClan() || isInAutoClan())
			return false;

		auto* pClan = g_pMain->GetClanPtr(GetClanID());
		if (pClan == nullptr || pClan->m_byGrade > 3)
			return false;
	}

	// CSW winner clan/alliance members can always access Delos regardless of NP.
	if (!isWinnerClan && GetLoyalty() == 0)
		return false;

	return true;
}
#pragma endregion

#pragma region CUser::csw_town()
void CUser::csw_town()
{
	/*if (g_pMain->pSiegeWar.sMasterKnights == GetClanID())
		return;*/

	Home(false);
}
#pragma endregion

#pragma region CUser::csw_rank()
void CUser::csw_rank()
{
	if (!isInClan() || !g_pMain->isCswActive())
		return;

	uint16 sCount = 0;
	Packet result(WIZ_SIEGE, uint8(5));
	result << uint8(2) << sCount;
	result.DByte();

	struct _list {
		uint16 clanid, killcount;
		_list(uint16 clanid, uint16 killcount) {
			this->clanid = clanid;
			this->killcount = killcount;
		}
	};


	std::vector<_list> mlist;
	g_pMain->pCswEvent.mClanListLock.lock();
	foreach(itr, g_pMain->pCswEvent.mClanList)
		mlist.push_back(_list(itr->first, itr->second));
	g_pMain->pCswEvent.mClanListLock.unlock();

	std::sort(mlist.begin(), mlist.end(), [](auto const& a, auto const& b) { return a.killcount > b.killcount; });

	uint8 sReaminCount = 0;
	foreach(itr, mlist) {
		CKnights* pknights = g_pMain->GetClanPtr(itr->clanid);
		if (!pknights)
			continue;

		sReaminCount++;
		result << itr->clanid << pknights->m_sMarkVersion << pknights->GetName() << uint8(0) << itr->killcount << uint8(1) << sReaminCount;
		sCount++;
		if (sCount >= 50)
			break;
	}
	result.ByteBuffer::put(2, sCount);
	Send(&result);
}
#pragma endregion

#pragma region CUser::csw_rank_register()
void CUser::csw_rank_register()
{
	if (!isInClan() || !g_pMain->isCswActive())
		return;

	Guard lock(g_pMain->pCswEvent.mClanListLock);
	auto itr = g_pMain->pCswEvent.mClanList.find(GetClanID());
	if (itr == g_pMain->pCswEvent.mClanList.end())
	{
		g_pMain->pCswEvent.mClanList.insert(std::make_pair(GetClanID(), 0));
	}
}
#pragma endregion

#pragma region CUser::csw_rank_killupdate()
void CUser::csw_rank_killupdate()
{
	if (!isInClan())
		return;

	Guard lock(g_pMain->pCswEvent.mClanListLock);
	auto itr = g_pMain->pCswEvent.mClanList.find(GetClanID());
	if (itr != g_pMain->pCswEvent.mClanList.end())
		itr->second++;
}
#pragma endregion

#pragma region CNpc::csw_momumentprocess(CUser* pUser)
void CNpc::csw_momumentprocess(CUser* pUser) {
	if (pUser == nullptr || !pUser->isInClan() || !g_pMain->isCswActive() || !g_pMain->isCswWarActive())
		return;

	auto* pknights = g_pMain->GetClanPtr(pUser->GetClanID());
	if (!pknights || pknights->m_byGrade > 3)
		return;

	g_pMain->pSiegeWar.sMasterKnights = pUser->GetClanID();
	g_pMain->UpdateSiege(g_pMain->pSiegeWar.sCastleIndex, g_pMain->pSiegeWar.sMasterKnights, g_pMain->pSiegeWar.bySiegeType, 0, 0, 0);
	g_pMain->csw_usertools(true, CswNotice::MonumentKilled, true, false, false, true);
	g_pMain->ResetAllEventObject(ZONE_DELOS);
}
#pragma endregion
