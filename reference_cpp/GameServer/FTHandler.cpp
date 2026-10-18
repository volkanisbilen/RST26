#include "stdafx.h"

_FORGETTEN_TEMPLE_STAGES CGameServerDlg::ForgettenTempleGetStage()
{
	foreach_stlmap(ita, m_ForgettenTempleStagesArray) {
		auto* pStage = ita->second;
		if (pStage == nullptr || pStage->Type != pForgettenTemple.Type || pStage->Stage != pForgettenTemple.Stage)
			continue;

		return *pStage;
	}
	return _FORGETTEN_TEMPLE_STAGES();
}

std::vector< _FORGETTEN_TEMPLE_SUMMON> CGameServerDlg::ForgettenTempleLoadSpawn()
{
	std::vector< _FORGETTEN_TEMPLE_SUMMON> mVector;
	foreach_stlmap(ita, m_ForgettenTempleMonsterArray) {
		auto* pSummon = ita->second;
		if (pSummon == nullptr || pSummon->Type != pForgettenTemple.Type)
			continue;

		if (pSummon->Stage != pForgettenTemple.Stage)
			continue;

		mVector.push_back(*pSummon);
	}

	return mVector;
}

void CGameServerDlg::ForgettenTempleReset()
{
	RemoveAllEventNpc(ZONE_FORGOTTEN_TEMPLE);
	pForgettenTemple.Initialize(true);
}

void CGameServerDlg::ForgettenTempleStart(uint8 Type, uint8 MinLevel, uint8 MaxLevel)
{
	if (m_ForgettenTempleMonsterArray.IsEmpty() || m_ForgettenTempleStagesArray.IsEmpty() || pForgettenTemple.isActive) return;
	pForgettenTemple.isActive = true;
	pForgettenTemple.isJoin = true;
	pForgettenTemple.MinLevel = pForgettenTemple.ptimeopt.MinLevel;
	pForgettenTemple.MaxLevel = pForgettenTemple.ptimeopt.MaxLevel;
	pForgettenTemple.Stage = 1;
	pForgettenTemple.Type = Type;
	pForgettenTemple.StartTime = UNIXTIME;
	pForgettenTemple.FinishTime = UNIXTIME + (pForgettenTemple.ptimeopt.PlayingTime * MINUTE);
	Announcement(IDS_MONSTER_CHALLENGE_OPEN);
}

void CGameServerDlg::ForgettenTempleSendItem()
{
#define MIN_DAMAGE 50000
	ServerLog("FT_REWARD", "reward distribution started");

	std::vector<_EVENT_REWARD> mreward;
	m_EventRewardArray.m_lock.lock();
	foreach_stlmap_nolock(itr, m_EventRewardArray)
		if (itr->second && 13 == itr->second->local_id && itr->second->status)
			mreward.push_back(*itr->second);
	m_EventRewardArray.m_lock.unlock();

	pForgettenTemple.UserListLock.lock();
	std::map<uint16, uint64> copymap = pForgettenTemple.UserList;
	pForgettenTemple.UserListLock.unlock();

	const bool hasConfiguredReward = !mreward.empty();
	foreach(itr, copymap) {
		auto* pUser = g_pMain->GetUserPtr(itr->first);
		if (pUser == nullptr || !pUser->isInGame() || pUser->isDead() || pUser->GetZoneID() != ZONE_FORGOTTEN_TEMPLE)
			continue;

		if (!hasConfiguredReward)
		{
			// Fallback reward when DB reward row is missing/disabled.
			ServerLog("FT_REWARD", "fallback reward user=%s", pUser->GetName().c_str());
			pUser->GiveItem("FT Fallback", BLUE_TREASURE_CHEST, 1);
			pUser->GiveItem("FT Fallback", FORGETTEN_TEMPLE_COIN1, 1);
			continue;
		}

		foreach(itr2, mreward) {
			ServerLog("FT_REWARD", "configured reward user=%s local_id=%u", pUser->GetName().c_str(), itr2->local_id);

			for (int i = 0; i < 3; i++)
				if(itr2->itemid[i])
					pUser->GiveItem("FT", itr2->itemid[i], itr2->itemcount[i], true, itr2->itemexpiration[i]);

			if (itr2->cash)
				pUser->GiveBalance(itr2->cash);
			if (itr2->experience)
				pUser->ExpChange("FT", itr2->experience, true);
			if (itr2->loyalty)
				pUser->SendLoyaltyChange("FT", itr2->loyalty);
			if (itr2->noah)
				pUser->GoldGain(itr2->noah, true, false);
		}
	}

	/*struct _RANKING { uint16 ID = 0; uint64 TotalDamage = 0; uint16 Rank = 0; };
	std::vector<_RANKING> pRankingList;
	foreach(itr, pForgettenTemple.UserList) {
		auto* pUser = g_pMain->GetUserPtr(itr->first);
		if (pUser == nullptr || !pUser->isInGame() || pUser->isDead() || pUser->GetZoneID() != ZONE_FORGOTTEN_TEMPLE) continue;

		_RANKING pList;
		pList.ID = itr->first;
		pList.TotalDamage = itr->second;
		pRankingList.push_back(pList);
	}

	if (pRankingList.empty())
		return;

	uint16 TotalSing = (uint16)pRankingList.size(); int Counter = 1;
	std::sort(pRankingList.begin(), pRankingList.end(), [](_RANKING const& a, _RANKING const& b) { return a.TotalDamage > b.TotalDamage; });
	foreach(itr, pRankingList) {
		CUser* pUser = g_pMain->GetUserPtr(itr->ID);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		itr->Rank = Counter++;
	}

	foreach(itr, pRankingList) {
		CUser* pUser = g_pMain->GetUserPtr(itr->ID);
		if (pUser == nullptr || !pUser->isInGame() || itr->TotalDamage < MIN_DAMAGE)
			continue;

		double Dilim = ceil((itr->Rank * 100) / TotalSing);
		int test = (int)Dilim;
		if (test <= 50) pUser->GiveItem("FT Event",BLUE_TREASURE_CHEST,1);
		else  pUser->GiveItem("FT Event", GREEN_TREASURE_CHEST, 1);
		pUser->GiveItem("FT Event", FORGETTEN_TEMPLE_COIN1);
		printf("Ft Damage Test | Info: Name:%s Rank:%d Dilim:%d Damage:%I64d \n",pUser->GetName().c_str(),itr->Rank,test,itr->TotalDamage);
	}*/
}

void CGameServerDlg::FtFinish() {
	pForgettenTemple.isActive = false;
	KickOutZoneUsers(ZONE_FORGOTTEN_TEMPLE, ZONE_MORADON);
	ForgettenTempleReset();
	Announcement(IDS_MONSTER_CHALLENGE_CLOSE);
}

void CGameServerDlg::ForgettenTempleTimerProc()
{
	if (!pForgettenTemple.isActive)
		return;

	if (pForgettenTemple.isFinished && pForgettenTemple.isWaiting && UNIXTIME > pForgettenTemple.WaitingTime)
		return FtFinish();

	if (pForgettenTemple.isFinished)
		return;

	if (UNIXTIME > pForgettenTemple.FinishTime) {
		if (!pForgettenTemple.FinishTime) printf("Forgetten Temple Starting Error | Info:Finish time is zero. ! \n");
		return FtFinish();
	}

	if (!pForgettenTemple.isSummonCheck && UNIXTIME > (pForgettenTemple.StartTime + pForgettenTemple.ptimeopt.SummonTime)) {

		/*if (pForgettenTemple.UserList.size() < 20)
			return FtFinish();*/

		pForgettenTemple.isSummonCheck = true;
		pForgettenTemple.Stage = 1;
		pForgettenTemple.isLastSummonTime = UNIXTIME + 30;
		pForgettenTemple.isJoin = false;
		pForgettenTemple.isSummon = true;
		Announcement(IDS_MONSTER_CHALLENGE_START, (uint8)Nation::ALL, (uint8)ChatType::PUBLIC_CHAT);
	}

	if (pForgettenTemple.isSummon && !pForgettenTemple.isLastSummon) {
		_FORGETTEN_TEMPLE_STAGES pStages = ForgettenTempleGetStage();
		if (pStages.nIndex != -1) {

			if (!pForgettenTemple.isLastSummon)
			{
				uint32 ntime = (uint32)pForgettenTemple.StartTime + (pStages.Time + pForgettenTemple.ptimeopt.SummonTime);

				if (UNIXTIME > ntime)
				{
					auto summonlist = ForgettenTempleLoadSpawn();
					if (summonlist.size())
					{
						foreach(pSummon, summonlist)
							SpawnEventNpc(pSummon->SidID, true, ZONE_FORGOTTEN_TEMPLE, (float)pSummon->PosX, 0, (float)pSummon->PosZ, pSummon->SidCount, pSummon->Range, 0, 0, -1, 0, 0, 1, 0, (uint16)SpawnEventType::ForgettenTempleSummon);

						pForgettenTemple.Stage++;
						pForgettenTemple.isLastSummonTime = UNIXTIME;
					}
					else
					{
						pForgettenTemple.isLastSummon = true;
						pForgettenTemple.isSummon = false;
					}
				}
			}
		}
		else {
			pForgettenTemple.isLastSummon = true;
			pForgettenTemple.isSummon = false;
		}
	}

	if (pForgettenTemple.isLastSummon && !pForgettenTemple.isSummon && !g_pMain->pForgettenTemple.monstercount) {
		pForgettenTemple.WaitingTime = UNIXTIME + pForgettenTemple.ptimeopt.WaitingTime;
		pForgettenTemple.isWaiting = true;
		pForgettenTemple.isFinished = true;
		ForgettenTempleSendItem();
		Announcement(IDS_MONSTER_CHALLENGE_VICTORY);
	}
}

void CNpc::ForgettenTempleMonsterDead() 
{
	if (!g_pMain->isForgettenTempleActive() || GetZoneID() != ZONE_FORGOTTEN_TEMPLE || e_stype != e_summontype::m_ftmonster)
		return;

	if (g_pMain->pForgettenTemple.monstercount > 0) g_pMain->pForgettenTemple.monstercount--;

	uint32 nSkillID = 0;
	if (GetProtoID() == 9816) nSkillID = 492059;
	else if (GetProtoID() == 9817) nSkillID = 492060;
	else if (GetProtoID() == 9818) nSkillID = 492061;
	else if (GetProtoID() == 9819) nSkillID = 492060;
	else if (GetProtoID() == 9820) nSkillID = 492062;
	if (!nSkillID)
		return;

	bool r_result = false;

	MagicInstance instance;
	instance.bIsRunProc = true;
	instance.bIsItemProc = true;
	instance.sCasterID = GetID();
	instance.sTargetID = -1;
	instance.nSkillID = nSkillID;
	instance.sSkillCasterZoneID = GetZoneID();
	instance.sData[0] = (uint16)127;
	instance.sData[2] = (uint16)127;
	instance.Run();
}
