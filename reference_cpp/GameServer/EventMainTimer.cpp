#include "stdafx.h"
#include "DBAgent.h"

#pragma region Event Manuel Opening
#pragma region CGameServerDlg::ChaosExpansionManuelOpening()
void CGameServerDlg::ChaosExpansionManuelOpening()
{
	if (!pEventTimeOpt.pvroomop[1].sign || !pEventTimeOpt.pvroomop[1].play) {
		printf("Room Event Timer Table is null or sign time : 0 or play time  : 0 \n");
		return;
	}

	if (pTempleEvent.ActiveEvent != -1) { printf("event is ongoing \n"); return; }

	uint32 nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;
	uint32 CloseTime = (pEventTimeOpt.pvroomop[1].sign + pEventTimeOpt.pvroomop[1].play) * MINUTE;

	TempleEventReset(EventOpCode::TEMPLE_EVENT_CHAOS, true);
	TrashChaosExpansionRanking();
	pTempleEvent.isAutomatic = false;
	pTempleEvent.isAttackable = false;
	pTempleEvent.bAllowJoin = true;
	pTempleEvent.ActiveEvent = (int16)EventOpCode::TEMPLE_EVENT_CHAOS;
	pTempleEvent.ZoneID = ZONE_CHAOS_DUNGEON;
	pTempleEvent.StartTime = UNIXTIME;
	pTempleEvent.ClosedTime = UNIXTIME + CloseTime;
	pTempleEvent.SignRemainSeconds = UNIXTIME + pEventTimeOpt.pvroomop[1].sign * MINUTE;

	TempleEventStart();
	printf("[Manuel Event] call for the %s Event is started at %02d:%02d:%02d \n", pEventTimeOpt.pvroomop[1].Name.c_str(), nHour, nMinute, nSeconds);
}
#pragma endregion

#pragma region CGameServerDlg::BorderDefenceWarManuelOpening()
void CGameServerDlg::BorderDefenceWarManuelOpening()
{
	if (!pEventTimeOpt.pvroomop[0].sign || !pEventTimeOpt.pvroomop[0].play) {
		printf("Room Event Timer Table is null or sign time : 0 or play time  : 0 \n");
		return;
	}

	if (pTempleEvent.ActiveEvent != -1) { printf("event is ongoing \n"); return; }

	uint32 nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;
	uint32 CloseTime = (pEventTimeOpt.pvroomop[0].sign + pEventTimeOpt.pvroomop[0].play) * MINUTE;

	TempleEventReset(EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR, true);
	TrashBorderDefenceWarRanking();
	pTempleEvent.isAttackable = false;
	pTempleEvent.bAllowJoin = true;
	pTempleEvent.ActiveEvent = (int16)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR;
	pTempleEvent.ZoneID = ZONE_BORDER_DEFENSE_WAR;
	pTempleEvent.StartTime = UNIXTIME;
	pTempleEvent.ClosedTime = UNIXTIME + CloseTime;
	pTempleEvent.SignRemainSeconds = UNIXTIME + pEventTimeOpt.pvroomop[0].sign * MINUTE;

	TempleEventStart();
	printf("[Manuel Event] call for the %s Event is started at %02d:%02d:%02d \n", pEventTimeOpt.pvroomop[0].Name.c_str(), nHour, nMinute, nSeconds);
}
#pragma endregion

#pragma region CGameServerDlg::JuraidMountainManuelOpening()
void CGameServerDlg::JuraidMountainManuelOpening()
{
	if (!pEventTimeOpt.pvroomop[2].sign || !pEventTimeOpt.pvroomop[2].play) {
		printf("Room Event Timer Table is null or sign time : 0 or play time  : 0 \n");
		return;
	}

	if (pTempleEvent.ActiveEvent != -1) { printf("event is ongoing \n"); return; }

	uint32 nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;
	uint32 CloseTime = (pEventTimeOpt.pvroomop[2].sign + pEventTimeOpt.pvroomop[2].play) * MINUTE;

	TempleEventReset(EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN, true);
	pTempleEvent.isAttackable = false;
	pTempleEvent.bAllowJoin = true;
	pTempleEvent.ActiveEvent = (int16)EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN;
	pTempleEvent.ZoneID = ZONE_JURAID_MOUNTAIN;
	pTempleEvent.StartTime = UNIXTIME;
	pTempleEvent.ClosedTime = UNIXTIME + CloseTime;
	pTempleEvent.bridge_active = true;
	pTempleEvent.bridge_start_min = (uint32)UNIXTIME;
	pTempleEvent.SignRemainSeconds = UNIXTIME + pEventTimeOpt.pvroomop[2].sign * MINUTE;

	TempleEventStart();
	SendAnnouncement("Gate Keeper of Islant mountain: 70 above level Warriors!! I Will let you enter the cave now.");
	printf("[Manuel Event] call for the %s Event is started at %02d:%02d:%02d \n", pEventTimeOpt.pvroomop[2].Name.c_str(), nHour, nMinute, nSeconds);
}
#pragma endregion

#pragma region CGameServerDlg::BeefEventManuelOpening()
void CGameServerDlg::BeefEventManuelOpening()
{
	_BEEF_EVENT_PLAY_TIMER* pPlayInfo = m_BeefEventPlayTimerArray.GetData((uint8)EventLocalID::BeefEvent);
	if (pPlayInfo == nullptr
		|| pPlayInfo->MonumentTime <= 0
		|| pPlayInfo->LoserSignTime <= 0
		|| pPlayInfo->FarmingTime <= 0)
	{
		printf("Beef Event Timer Table is null or monument time : 0 or loser time : 0 farming time : 0 \n");
		return;
	}

	if (pBeefEvent.isActive)
	{
		printf("event is ongoing \n");
		return;
	}

	pBeefEvent.isActive = true;
	pBeefEvent.isAttackable = true;
	pBeefEvent.BeefMonumentStartTime = (uint32)UNIXTIME;
	pBeefEvent.BeefSendTime = pPlayInfo->MonumentTime * MINUTE;
	pBeefEvent.isMonumentDead = false;
	pBeefEvent.WictoryNation = 0;
	BeefEventUpdateTime();
	BeefEventSendNotice(1);

	uint32 nHour = g_localTime.tm_hour;
	uint32 nMinute = g_localTime.tm_min;
	uint32 nSeconds = g_localTime.tm_sec;
	printf("[Manuel Event] call for the %s Event is started at %02d:%02d:%02d \n", "Beef Event", nHour, nMinute, nSeconds);
}
#pragma endregion
#pragma endregion

#pragma region Event Manuel Close
#pragma region CGameServerDlg::ChaosExpansionManuelClosed()
void CGameServerDlg::ChaosExpansionManuelClosed()
{
	if (pTempleEvent.ActiveEvent != -1) {
		if (pTempleEvent.ActiveEvent != (int16)EventOpCode::TEMPLE_EVENT_CHAOS) {
			printf("Another event is open\n");
			return;
		}

		uint16 remtime = 0;
		if (pTempleEvent.SignRemainSeconds > UNIXTIME)
			remtime = uint16(pTempleEvent.SignRemainSeconds - UNIXTIME);

		if (!pTempleEvent.isActive) {
			printf("The event has %d seconds to start\n", remtime);
			return;
		}
		if (pTempleEvent.EventManuelClose) {
			printf("Application already submitted\n");
			return;
		}
		TempleEventSendWinnerScreen();
		pTempleEvent.EventManuelClose = true;
		pTempleEvent.EventTimerFinishControl = true;
		pTempleEvent.ManuelClosedTime = (uint32)UNIXTIME;
		return;
	}

	printf("Chaos Expansion Event is not open\n");
}
#pragma endregion

void CGameServerDlg::BeefEventManuelClosed() {
	BeefEventSendNotice(4);
	ResetBeefEvent();
	KickOutZoneUsers(ZONE_BIFROST, ZONE_RONARK_LAND);
}

#pragma region CGameServerDlg::BorderDefenceWarManuelClosed()
void CGameServerDlg::BorderDefenceWarManuelClosed()
{
	if (pTempleEvent.ActiveEvent != -1) {
		if (pTempleEvent.ActiveEvent != (int16)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR) {
			printf("Another event is open\n");
			return;
		}

		uint16 remtime = 0;
		if (pTempleEvent.SignRemainSeconds > UNIXTIME)
			remtime = uint16(pTempleEvent.SignRemainSeconds - UNIXTIME);

		if (!pTempleEvent.isActive) {
			printf("The event has %d seconds to start\n", remtime);
			return;
		}
		if (pTempleEvent.EventManuelClose) {
			printf("Application already submitted\n");
			return;
		}
		TempleEventSendWinnerScreen();
		pTempleEvent.EventManuelClose = true;
		pTempleEvent.EventTimerFinishControl = true;
		pTempleEvent.ManuelClosedTime = (uint32)UNIXTIME;
		return;
	}
	printf("Border Defence War Event is not open\n");
}
#pragma endregion

#pragma region CGameServerDlg::JuraidMountainManuelClosed()
void CGameServerDlg::JuraidMountainManuelClosed()
{
	if (pTempleEvent.ActiveEvent != -1) {
		if (pTempleEvent.ActiveEvent != (int16)EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN) {
			printf("Another event is open\n");
			return;
		}

		uint16 remtime = 0;
		if (pTempleEvent.SignRemainSeconds > UNIXTIME)
			remtime = uint16(pTempleEvent.SignRemainSeconds - UNIXTIME);

		if (!pTempleEvent.isActive) {
			printf("The event has %d seconds to start\n", remtime);
			return;
		}
		if (pTempleEvent.EventManuelClose) {
			printf("Application already submitted\n");
			return;
		}
		TempleEventSendWinnerScreen();
		pTempleEvent.EventManuelClose = true;
		pTempleEvent.EventTimerFinishControl = true;
		pTempleEvent.ManuelClosedTime = (uint32)UNIXTIME;
		return;
	}
	printf("Juraid Mountain Event is not open\n");
}
#pragma endregion
#pragma endregion

#pragma region CGameServerDlg::EventMainTimer()
void CGameServerDlg::EventMainTimer()
{
	level_merchant_timer();
	csw_maintimer();
	VirtualEventTimer();
	SingleLunarEventTimer();
	SingleOtherEventTimer();
	HandleCzSantaEventTimer();
	if (pBeefEvent.isMonumentDead && pBeefEvent.BeefFarmingPlayTime < UNIXTIME)
	{
		BeefEventSendNotice(4);
		ResetBeefEvent();
		KickOutZoneUsers(ZONE_BIFROST,ZONE_RONARK_LAND);
	}
}
#pragma endregion

void CGameServerDlg::HandleCzSantaEventTimer()
{
	const uint16 sSantaProtoID = 10055;
	const uint8 sEventZone = ZONE_RONARK_LAND;
	const uint32 sEventDuration = 6 * HOUR;
	const uint32 sRespawnDelay = 2 * MINUTE;

	uint32 nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min;
	if ((nHour % 6) == 0 && nMinute == 0)
	{
		if (m_CzSantaLastOpenHour != nHour)
		{
			m_bCzSantaEventActive = true;
			m_CzSantaEventEndTime = (uint32)UNIXTIME + sEventDuration;
			m_CzSantaNextSpawnTime = (uint32)UNIXTIME;
			m_CzSantaLastOpenHour = nHour;
			SendAnnouncement("CZ Santa event started. Santa appears every 2 minutes after being killed.", Nation::ALL);
		}
	}

	if (!m_bCzSantaEventActive)
		return;

	if ((uint32)UNIXTIME >= m_CzSantaEventEndTime)
	{
		m_bCzSantaEventActive = false;
		m_CzSantaNextSpawnTime = 0;
		SendAnnouncement("CZ Santa event ended.", Nation::ALL);
		return;
	}

	bool bSantaAliveInCz = false;
	CNpcThread* pThread = m_arNpcThread.GetData(sEventZone);
	if (pThread != nullptr)
	{
		pThread->m_arNpcArray.m_lock.lock();
		foreach_stlmap_nolock(itr, pThread->m_arNpcArray)
		{
			CNpc* pNpc = TO_NPC(itr->second);
			if (pNpc == nullptr || pNpc->isDead())
				continue;

			if (pNpc->GetProtoID() == sSantaProtoID)
			{
				bSantaAliveInCz = true;
				break;
			}
		}
		pThread->m_arNpcArray.m_lock.unlock();
	}

	if (bSantaAliveInCz || (uint32)UNIXTIME < m_CzSantaNextSpawnTime)
		return;

	_START_POSITION* pStart = GetStartPosition(sEventZone);
	if (pStart == nullptr)
		return;

	// Spawn Santas near both nations' CZ town/teleport sides to guarantee visibility.
	float karusX = (float)pStart->sKarusX + (float)myrand(-10, 10);
	float karusZ = (float)pStart->sKarusZ + (float)myrand(-10, 10);
	float elmoX = (float)pStart->sElmoradX + (float)myrand(-10, 10);
	float elmoZ = (float)pStart->sElmoradZ + (float)myrand(-10, 10);

	// Santa event must be spawned as a normal event monster.
	// Do not use SpawnEventType::MonsterBossSummon here; that path expects MonsterBossRandom data
	// and can silently remove the spawn when no special boss index exists.
	SpawnEventNpc(sSantaProtoID, true, sEventZone, karusX, 0.0f, karusZ, 1, 0, 0, 0, -1, 0, 0, 1, 0, 0, 0);
	SpawnEventNpc(sSantaProtoID, true, sEventZone, elmoX, 0.0f, elmoZ, 1, 0, 0, 0, -1, 0, 0, 1, 0, 0, 0);

	SendAnnouncement("Santa Claus monsters have spawned in Ronark Land town areas.", Nation::ALL);
	m_CzSantaNextSpawnTime = (uint32)UNIXTIME + sRespawnDelay;
}

void CGameServerDlg::level_merchant_timer()
{
	if (tar_levelmercreward)
		return;

	std::string notice;
	Packet newpkt;

	if (pLevMercInfo.status) {
		if (UNIXTIME > pLevMercInfo.finishtime) {
			pLevMercInfo.Initialize();
			notice = "Mercant Exp Event has ended. Enjoy your stay.";
			g_pMain->GetServerResource(IDP_ANNOUNCEMENT, &notice, notice.c_str());
			ChatPacket::Construct(&newpkt, (uint8)ChatType::WAR_SYSTEM_CHAT, &notice);
			Send_All(&newpkt);
		}
		return;
	}

	uint32 nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;
	_LEVEL_MERCHANT_REWARDS pReward = _LEVEL_MERCHANT_REWARDS();

	m_LevelMerchantRewardArray.m_lock.lock();
	foreach_stlmap_nolock(itr, m_LevelMerchantRewardArray) {
		auto* p = itr->second;
		if (!p)
			continue;

		if (p->startHour == nHour && p->startMinute == nMinute && !nSeconds) {
			pReward = *p;
			break;
		}
	}
	m_LevelMerchantRewardArray.m_lock.unlock();

	if (!pReward.index)
		return;

	notice = string_format("Mercant Exp Event has activated. Enjoy your stay. "
		"If you set up a Merchant between %d:%d - %d minute, you will gain %d% EXP every %d minutes.", nHour, nMinute, pReward.finih_minute,pReward.rate_experience,pReward.exp_minute);

	pLevMercInfo.exprate = pReward.rate_experience;
	pLevMercInfo.rewardtime = pReward.exp_minute;
	for (int i = 0; i < MAX_USER; i++) {
		CUser* pUser = GetUserPtr(i);
		if (pUser) pUser->level_merchant_time = UNIXTIME + (pLevMercInfo.rewardtime * MINUTE);
	}
	pLevMercInfo.finishtime = UNIXTIME + (pReward.finih_minute * MINUTE);
	pLevMercInfo.status = true;

	g_pMain->GetServerResource(IDP_ANNOUNCEMENT, &notice, notice.c_str());
	ChatPacket::Construct(&newpkt, (uint8)ChatType::WAR_SYSTEM_CHAT, &notice);
	Send_All(&newpkt);
}

#pragma region CGameServerDlg::VirtualEventLocalTimer()
void CGameServerDlg::SingleOtherEventLocalTimer()
{
	if (m_nUnderTheCastleEventTime > 0)
		m_nUnderTheCastleEventTime--;

	if (pBeefEvent.BeefSendTime > 0)
		pBeefEvent.BeefSendTime--;
}
#pragma endregion

int32 CGameServerDlg::ZindanWarStageSelect()
{
	foreach_stlmap(ita, m_ZindanWarStageListArray)
	{
		_ZINDAN_WAR_STAGES* pStage = ita->second;
		if (pStage == nullptr)
			continue;

		if (pStage->Stage == ZindanWarSummonStage) {
			return pStage->nIndex;
		}
	}
	return -1;
}

bool CGameServerDlg::ZindanWarLoadSpawn()
{
	ZindanSpawnList.clear();
	foreach_stlmap(ita, m_ZindanWarSummonListArray)
	{
		_ZINDAN_WAR_SUMMON* pSummon = ita->second;
		if (pSummon == nullptr )
			continue;

		if (pSummon->Stage == ZindanWarSummonStage) {
			ZindanSpawnList.push_back(pSummon->bIndex);
		}
	}

	return (uint32)ZindanSpawnList.empty() ? false : true;
}

bool CGameServerDlg::isgetonday(bool mday[7]) { return mday[g_localTime.tm_wday]; }

void CGameServerDlg::VirtualEventOpen(uint8 id, EVENT_OPENTIMELIST ptime, int timei, int nhour, int nminute) {

	if (!pEventTimeOpt.pvroomop[id].sign || !pEventTimeOpt.pvroomop[id].play)
		return;

	uint32 c_time = (pEventTimeOpt.pvroomop[id].sign + pEventTimeOpt.pvroomop[id].play) * MINUTE;

	if (id == 0) {
		TempleEventReset(EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR, true);
		TrashBorderDefenceWarRanking();
	}
	else if (id == 1) {
		TempleEventReset(EventOpCode::TEMPLE_EVENT_CHAOS, true);
		TrashChaosExpansionRanking();
	}
	else if (id == 2)
		TempleEventReset(EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN, true);

	pTempleEvent.isAttackable = false;
	pTempleEvent.bAllowJoin = true;
	if (id == 0) pTempleEvent.ActiveEvent = (int16)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR;
	else if (id == 1) pTempleEvent.ActiveEvent = (int16)EventOpCode::TEMPLE_EVENT_CHAOS;
	else if (id == 2) pTempleEvent.ActiveEvent = (int16)EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN;
	pTempleEvent.ZoneID = (int8)ptime.zoneid;
	pTempleEvent.StartTime = UNIXTIME;
	pTempleEvent.ClosedTime = UNIXTIME + c_time;
	pTempleEvent.SignRemainSeconds = UNIXTIME + pEventTimeOpt.pvroomop[0].sign * MINUTE;
	pTempleEvent.pTime = ptime;
	pTempleEvent.isAutomatic = true;
	if (id == 2) {
		pTempleEvent.bridge_active = true;
		pTempleEvent.bridge_start_min = (uint32)UNIXTIME;
	}

	TempleEventStart();

	if (id == 2)
		SendAnnouncement("Gate Keeper of Islant mountain: 70 above level Warriors!! I Will let you enter the cave now.");

	printf("%d. call for the %s Event is started at %02d:%02d:%02d \n", timei, ptime.name.c_str(), nhour, nminute, 0);
}
void CGameServerDlg::CloseBowlEvent()
{
	g_pMain->isBowlEventActive = false;
	std::string sEventMessages2 = string_format("Bowl Event etkinliği sona erdi.");
	g_pMain->tBowlEventTime = 0;
	g_pMain->tBowlEventZone = 0;
	if (!sEventMessages2.empty())
		g_pMain->SendAnnouncement(sEventMessages2.c_str(), (uint8)Nation::ALL);
}

#pragma region CGameServerDlg::VirtualEventTimer()

void CGameServerDlg::VirtualEventTimer()
{
	if (g_pMain->isBowlEventActive) {
		
		if (g_pMain->tBowlEventTime)
			g_pMain->tBowlEventTime--;

		if (!g_pMain->tBowlEventTime)
			g_pMain->CloseBowlEvent();
	}

	if (pSpecialEvent.opened) {
		if (!ZindanWarLastSummon && pSpecialEvent.zoneid == ZONE_SPBATTLE1) {
			if (!ZindanWarSummonCheck && ZindanWarStartTimeTickCount != 0) {
				if ((uint32)UNIXTIME >= ZindanWarStartTimeTickCount + 10) {
					ZindanWarSummonStage = 1;
					ZindanLastSummonTime = (uint32)UNIXTIME + 30;
					ZindanWarSummonCheck = true;
					ZindanWarisSummon = true;
				}
			}
			if (ZindanWarisSummon && !ZindanWarLastSummon) {
				int32 Index = ZindanWarStageSelect();
				if (!Index || Index == -1) { ZindanWarLastSummon = true; ZindanWarisSummon = false; }

				_ZINDAN_WAR_STAGES* pStages = m_ZindanWarStageListArray.GetData(Index);
				if (!ZindanWarLastSummon && pStages != nullptr) {
					if ((uint32)UNIXTIME >= ZindanWarStartTimeTickCount + pStages->Time + 10) {
						if (ZindanWarLoadSpawn()) {
							foreach(rx, ZindanSpawnList) {
								_ZINDAN_WAR_SUMMON* pSummon = m_ZindanWarSummonListArray.GetData(*rx);
								if (pSummon == nullptr)continue;
								SpawnEventNpc(pSummon->SidID, pSummon->Type == 0 ? true : false, ZONE_SPBATTLE1, (float)pSummon->PosX, 0, (float)pSummon->PosZ, pSummon->SidCount, pSummon->Range, 0, 0, -1, 0);
							}
							ZindanWarSummonStage++;
							ZindanLastSummonTime = (uint32)UNIXTIME;
						}
						else { ZindanWarLastSummon = true; ZindanWarisSummon = false; }
					}
				}
			}
		}

		if (!pSpecialEvent.finishcheck && UNIXTIME >= pSpecialEvent.finishtime) {
			pSpecialEvent.finishcheck = true;
			SpecialEventFinish();
		}
	}

	uint32 nWeekDay = g_localTime.tm_wday, nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;
	
	pEventTimeOpt.mtimeforlooplock.lock();
	auto copy_map = pEventTimeOpt.mtimeforloop;
	pEventTimeOpt.mtimeforlooplock.unlock();

	if (copy_map.empty())
		return;

	if (pTempleEvent.ActiveEvent == -1)
	{
		foreach(ptime, copy_map) {
			if (!ptime->status || ptime->type != EventType::VirtualRoom || !isgetonday(ptime->iday))
				continue;

			bool successid = ptime->eventid == EventLocalID::BorderDefenceWar
				|| ptime->eventid == EventLocalID::ChaosExpansion
				|| ptime->eventid == EventLocalID::JuraidMountain;

			if (!successid)
				continue;

			for (int timei = 0; timei < EVENT_START_TIMES; timei++) {
				if (!ptime->timeactive[timei])
					continue;

				if (nHour == ptime->hour[timei] && nMinute == ptime->minute[timei] && nSeconds == 0) {

					int8 id = -1;
					if (ptime->eventid == EventLocalID::BorderDefenceWar)
						id = 0;
					else if (ptime->eventid == EventLocalID::ChaosExpansion)
						id = 1;
					else if (ptime->eventid == EventLocalID::JuraidMountain)
						id = 2;

					if (id >= 0)
						VirtualEventOpen(id, *ptime, timei, nHour, nMinute);
				}
			}
		}
	}
	else {
		if (pTempleEvent.ActiveEvent == (int8)EventOpCode::TEMPLE_EVENT_CHAOS) {
			if (!pTempleEvent.EventTimerStartControl && !pTempleEvent.isActive) { //Event Start
				uint32 EventSignFinishTime = (pEventTimeOpt.pvroomop[1].sign * MINUTE);
				if (UNIXTIME >= (pTempleEvent.StartTime + EventSignFinishTime)) {
					pTempleEvent.bAllowJoin = false;
					pTempleEvent.SignRemainSeconds = 0;
					pTempleEvent.LastEventRoom = 1;
					pTempleEvent.isActive = true;
					pTempleEvent.EventTimerStartControl = true;
					TempleEventManageRoom();
					TempleEventTeleportUsers();
				}
			}

			if (!pTempleEvent.isActive)
				return;

			//if (!pTempleEvent.EventCloseMainControl) {

			if (!pTempleEvent.EventTimerAttackOpenControl && !pTempleEvent.isAttackable) {//Event Attack Start
				uint32 EventAttackOpenTime = (pEventTimeOpt.pvroomop[1].sign + pEventTimeOpt.pvroomop[1].attackopen);
				EventAttackOpenTime = EventAttackOpenTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventAttackOpenTime)) {
					pTempleEvent.isAttackable = true;
					pTempleEvent.EventTimerAttackOpenControl = true;
				}
			}

			if (!pTempleEvent.EventTimerAttackCloseControl && pTempleEvent.isAttackable) {//Event Attack Close
				uint32 EventAttackStopTime = (pEventTimeOpt.pvroomop[1].sign + pEventTimeOpt.pvroomop[1].attackclose);
				EventAttackStopTime = EventAttackStopTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventAttackStopTime)) {
					pTempleEvent.isAttackable = false;
					pTempleEvent.EventTimerAttackCloseControl = true;
				}
			}

			if (pTempleEvent.EventTimerStartControl) {//Event Finish Control
				uint32 EventFinishTime = (pEventTimeOpt.pvroomop[1].sign + pEventTimeOpt.pvroomop[1].play);
				EventFinishTime = EventFinishTime * MINUTE;
				if (!pTempleEvent.EventTimerFinishControl) {
					if (UNIXTIME >= (pTempleEvent.StartTime + EventFinishTime)) {
						TempleEventSendWinnerScreen();
						pTempleEvent.EventTimerFinishControl = true;
					}
				}
				else if (UNIXTIME >= (pTempleEvent.StartTime + EventFinishTime))
					TempleEventRoomClose();
			}
			//}

			//if (pTempleEvent.EventCloseMainControl) {
			if (pTempleEvent.EventManuelClose) {
				if ((uint32)UNIXTIME >= pTempleEvent.ManuelClosedTime + pEventTimeOpt.pvroomop[1].finish) {
					TempleEventFinish();
					TempleEventReset(EventOpCode::TEMPLE_EVENT_CHAOS);
					pTempleEvent.EventTimerResetControl = true;
				}
			}
			TempleEventRoomClose();
			//}

			if (!pTempleEvent.EventTimerResetControl) {//Event Reset 
				uint32 EventResetTime = (pEventTimeOpt.pvroomop[1].sign + pEventTimeOpt.pvroomop[1].play);
				EventResetTime = EventResetTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventResetTime + pEventTimeOpt.pvroomop[1].finish)) {
					TempleEventFinish();
					TempleEventReset(EventOpCode::TEMPLE_EVENT_CHAOS);
					pTempleEvent.EventTimerResetControl = true;
				}
			}
		}
		else if (pTempleEvent.ActiveEvent == (int8)EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR) {

			if (!pTempleEvent.EventTimerStartControl && !pTempleEvent.isActive) {//Event Start
				if (UNIXTIME > (pTempleEvent.StartTime + (pEventTimeOpt.pvroomop[0].sign * MINUTE))) {
					pTempleEvent.EventTimerStartControl = true;
					pTempleEvent.SignRemainSeconds = 0;
					pTempleEvent.LastEventRoom = 1;
					pTempleEvent.isActive = true;
					pTempleEvent.bAllowJoin = false;
					TempleEventManageRoom();
					TempleEventTeleportUsers();
					TempleEventCreateParties();
				}
			}

			if (!pTempleEvent.isActive)
				return;

			//if (!pTempleEvent.EventCloseMainControl) {
			if (!pTempleEvent.EventTimerAttackOpenControl && !pTempleEvent.isAttackable) {//Event Attack Start
				uint32 EventAttackOpenTime = (pEventTimeOpt.pvroomop[0].sign + pEventTimeOpt.pvroomop[0].attackopen);
				EventAttackOpenTime = EventAttackOpenTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventAttackOpenTime)) {
					pTempleEvent.isAttackable = true;
					pTempleEvent.EventTimerAttackOpenControl = true;
				}
			}

			if (!pTempleEvent.EventTimerAttackCloseControl && pTempleEvent.isAttackable) {//Event Attack Close
				uint32 EventAttackStopTime = (pEventTimeOpt.pvroomop[0].sign + pEventTimeOpt.pvroomop[0].attackclose);
				EventAttackStopTime = EventAttackStopTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventAttackStopTime)) {
					pTempleEvent.isAttackable = false;
					pTempleEvent.EventTimerAttackCloseControl = true;
				}
			}

			if (pTempleEvent.EventTimerStartControl) {//Event Finish Control
				uint32 EventFinishTime = (pEventTimeOpt.pvroomop[0].sign + pEventTimeOpt.pvroomop[0].play);
				EventFinishTime = EventFinishTime * MINUTE;
				if (!pTempleEvent.EventTimerFinishControl) {
					if (UNIXTIME >= (pTempleEvent.StartTime + EventFinishTime)) {
						TempleEventSendWinnerScreen();
						pTempleEvent.EventTimerFinishControl = true;
					}
				}
				else if (UNIXTIME >= (pTempleEvent.StartTime + EventFinishTime))
					TempleEventRoomClose();
			}
			//}

			//if (pTempleEvent.EventCloseMainControl) {
			if (pTempleEvent.EventManuelClose) {
				if ((uint32)UNIXTIME >= pTempleEvent.ManuelClosedTime + pEventTimeOpt.pvroomop[0].finish) {
					TempleEventFinish();
					TempleEventReset(EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR);
					pTempleEvent.EventTimerResetControl = true;
				}
			}
			TempleEventRoomClose();
			//}

			if (!pTempleEvent.EventTimerResetControl) {//Event Reset
				uint32 EventResetTime = (pEventTimeOpt.pvroomop[0].sign + pEventTimeOpt.pvroomop[0].play + 1);
				EventResetTime = EventResetTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventResetTime + pEventTimeOpt.pvroomop[0].finish)) {
					TempleEventFinish();
					TempleEventReset(EventOpCode::TEMPLE_EVENT_BORDER_DEFENCE_WAR);
					pTempleEvent.EventTimerResetControl = true;
				}
			}
		}
		else if (pTempleEvent.ActiveEvent == (int8)EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN) {
			if (!pTempleEvent.EventTimerStartControl && !pTempleEvent.isActive) {//Event Start
				uint32 EventSignFinishTime = (pEventTimeOpt.pvroomop[2].sign * MINUTE);
				if (UNIXTIME >= (pTempleEvent.StartTime + EventSignFinishTime)) {
					pTempleEvent.SignRemainSeconds = 0;
					pTempleEvent.LastEventRoom = 1;
					pTempleEvent.isActive = true;
					pTempleEvent.bAllowJoin = false;
					TempleEventManageRoom();
					TempleEventTeleportUsers();
					TempleEventCreateParties();
					pTempleEvent.EventTimerStartControl = true;
				}
			}

			if (!pTempleEvent.isActive)
				return;

			//if (!pTempleEvent.EventCloseMainControl) { 

			if (!pTempleEvent.EventTimerAttackOpenControl && !pTempleEvent.isAttackable) {//Event Attack Start
				uint32 EventAttackOpenTime = (pEventTimeOpt.pvroomop[2].sign + pEventTimeOpt.pvroomop[2].attackopen);
				EventAttackOpenTime = EventAttackOpenTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventAttackOpenTime)) {
					pTempleEvent.isAttackable = true;
					pTempleEvent.EventTimerAttackOpenControl = true;
				}
			}

			if (!pTempleEvent.EventTimerAttackCloseControl && pTempleEvent.isAttackable) {//Event Attack Close
				uint32 EventAttackStopTime = (pEventTimeOpt.pvroomop[2].sign + pEventTimeOpt.pvroomop[2].attackclose);
				EventAttackStopTime = EventAttackStopTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventAttackStopTime)) {
					pTempleEvent.isAttackable = false;
					pTempleEvent.EventTimerAttackCloseControl = true;
				}
			}

			if (pTempleEvent.bridge_active) {//Bridge System
				if (!pTempleEvent.bridge_t_check[0]) {
					if ((uint32)UNIXTIME >= (pTempleEvent.bridge_start_min + 1200)) {
						TempleEventBridgeCheck(0);
						pTempleEvent.bridge_t_check[0] = true;
					}
				}

				if (!pTempleEvent.bridge_t_check[1]) {
					if ((uint32)UNIXTIME >= (pTempleEvent.bridge_start_min + 1800)) {
						TempleEventBridgeCheck(1);
						pTempleEvent.bridge_t_check[1] = true;
					}
				}

				if (!pTempleEvent.bridge_t_check[2]) {
					if ((uint32)UNIXTIME >= (pTempleEvent.bridge_start_min + 2400)) {
						TempleEventBridgeCheck(2);
						pTempleEvent.bridge_t_check[2] = true;
					}
				}
			}

			//Event Finish Control
			if (pTempleEvent.EventTimerStartControl) {
				uint32 EventFinishTime = (pEventTimeOpt.pvroomop[2].sign + pEventTimeOpt.pvroomop[2].play);
				EventFinishTime = EventFinishTime * MINUTE;
				if (!pTempleEvent.EventTimerFinishControl) {
					if (UNIXTIME >= (pTempleEvent.StartTime + EventFinishTime)) {
						TempleEventSendWinnerScreen();
						pTempleEvent.EventTimerFinishControl = true;
					}
				}
				else if (UNIXTIME >= (pTempleEvent.StartTime + EventFinishTime))
					TempleEventRoomClose();
			}
			//}

			//if (pTempleEvent.EventCloseMainControl) {
			if (pTempleEvent.EventManuelClose) {
				if ((uint32)UNIXTIME >= pTempleEvent.ManuelClosedTime + pEventTimeOpt.pvroomop[2].finish) {
					TempleEventFinish();
					TempleEventReset(EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN);
					pTempleEvent.EventTimerResetControl = true;
				}
			}
			TempleEventRoomClose();
			//}

			if (!pTempleEvent.EventTimerResetControl) {//Event Reset
				uint32 EventResetTime = pEventTimeOpt.pvroomop[2].sign + pEventTimeOpt.pvroomop[2].play + 1;
				EventResetTime = EventResetTime * MINUTE;
				if (UNIXTIME >= (pTempleEvent.StartTime + EventResetTime + pEventTimeOpt.pvroomop[2].finish)) {
					TempleEventFinish();
					TempleEventReset(EventOpCode::TEMPLE_EVENT_JURAD_MOUNTAIN);
					pTempleEvent.EventTimerResetControl = true;
				}
			}
		}
	}
}
#pragma endregion


#pragma region CGameServerDlg::SingleLunarEventTimer()
void CGameServerDlg::SingleLunarEventTimer()
{
	uint32 nWeekDay = g_localTime.tm_wday, nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;
	if (g_pMain->pEventTimeOpt.mtimeforloop.empty())
		goto final_time;

	if (!isWarOpen() /*&& !isCswActive()*/) {

		pEventTimeOpt.mtimeforlooplock.lock();
		auto copy_map = pEventTimeOpt.mtimeforloop;
		pEventTimeOpt.mtimeforlooplock.unlock();
		if (copy_map.empty())
			goto final_time;

		foreach(ptime, copy_map) {
			if (!ptime->status || ptime->type != EventType::LunarWar || !isgetonday(ptime->iday))
				continue;

			if (ptime->eventid != EventLocalID::AlseidsPrairie && ptime->eventid != EventLocalID::NapiesGorge
				&& ptime->eventid != EventLocalID::NereidsIsland && ptime->eventid != EventLocalID::NiedsTriangle
				&& ptime->eventid != EventLocalID::Oreads && ptime->eventid != EventLocalID::SnowWar
				&& ptime->eventid != EventLocalID::Zipang && ptime->eventid != EventLocalID::CastleSiegeWar)
				continue;

			for (int timei = 0; timei < EVENT_START_TIMES; timei++) {
				if (!ptime->timeactive[timei]) continue;

				if (ptime->type == EventType::LunarWar && ptime->eventid != EventLocalID::CastleSiegeWar) {
					for (int NoticeCount = 1; NoticeCount <= 5; NoticeCount++) {
						if ((ptime->minute[timei] - NoticeCount) < 0) {
							int16 sNoticeMinutes = ptime->minute[timei] - NoticeCount;
							if (sNoticeMinutes < 0) {
								sNoticeMinutes = 60 + sNoticeMinutes;
								int16 sNoticeHour = ptime->hour[timei] - 1;
								if (sNoticeHour < 0) sNoticeHour = 23;
								if (sNoticeHour == nHour && sNoticeMinutes == nMinute && nSeconds == 0) {
									printf("War Open Notice (1) left %d Minutes \n", NoticeCount);
									m_byBattleNoticeTime = NoticeCount;
									Announcement(IDS_PRE_BATTLE_ANNOUNCEMENT);
								}
							}
						}
						else if (ptime->hour[timei] == nHour && (ptime->minute[timei] - NoticeCount == nMinute) && nSeconds == 0) {
							printf("War Open Notice (2) left %d Minutes \n", NoticeCount);
							m_byBattleNoticeTime = NoticeCount;
							Announcement(IDS_PRE_BATTLE_ANNOUNCEMENT);
						}
					}
				}

				if (nHour == ptime->hour[timei] && nMinute == ptime->minute[timei] && nSeconds == 0) {
					if (ptime->type == EventType::LunarWar) {
						if (ptime->eventid == EventLocalID::CastleSiegeWar) {
							printf("%d. call for the %s Event is started at %02d:%02d:%02d \n", timei, ptime->name.c_str(), nHour, nMinute, nSeconds);
						}
						else {
							BattleZoneOpen(BATTLEZONE_OPEN, (uint8)ptime->zoneid - ZONE_BATTLE_BASE);
							printf("%d. call for the %s Event is started at %02d:%02d:%02d \n", timei, ptime->name.c_str(), nHour, nMinute, nSeconds);
						}
					}
				}
			}
		}
	}
	else if (m_byBattleOpen == NATION_BATTLE /*&& !isCswActive()*/) {
		BattleZoneCurrentUsers();
		int32 WarElapsedTime = int32(UNIXTIME) - m_byBattleOpenedTime;
		m_byBattleRemainingTime = m_byBattleTime - WarElapsedTime;
		uint8 nBattleZone = g_pMain->m_byBattleZone + ZONE_BATTLE_BASE;

		if (m_byNereidsIslandRemainingTime > 0 && nBattleZone == ZONE_BATTLE4) m_byNereidsIslandRemainingTime--;

		if (m_bVictory == 0) {
			if (m_bCommanderSelected == false && WarElapsedTime >= (m_byBattleTime / 24)) // Select captain
				BattleZoneSelectCommanders();
			else if ((WarElapsedTime == (m_byBattleTime / 4) && nBattleZone == ZONE_BATTLE4)) {
				if (m_sKarusMonuments >= 7 && m_sElmoMonuments <= 0)
					BattleZoneResult((uint8)Nation::KARUS);
				else if (m_sKarusMonuments <= 0 && m_sElmoMonuments >= 7)
					BattleZoneResult((uint8)Nation::ELMORAD);
			}
			else if (WarElapsedTime == (m_byBattleTime / 2)) {// War half time.
				if (nBattleZone == ZONE_BATTLE || nBattleZone == ZONE_BATTLE2 || nBattleZone == ZONE_BATTLE3)
					BattleWinnerResult(BattleWinnerTypes::BATTLE_WINNER_NPC);
				else if (nBattleZone == ZONE_BATTLE4) // Nereid's Island
					BattleWinnerResult(BattleWinnerTypes::BATTLE_WINNER_MONUMENT);
				else if (nBattleZone == ZONE_BATTLE6) // Oreads
					BattleWinnerResult(BattleWinnerTypes::BATTLE_WINNER_KILL);
			}
			m_sBattleTimeDelay++;
			if (m_sBattleTimeDelay >= (nBattleZone == ZONE_BATTLE4 ? (m_byBattleTime / 48) : (m_byBattleTime / 24))) {
				m_sBattleTimeDelay = 0;
				Announcement(DECLARE_BATTLE_ZONE_STATUS);
			}
		}
		else {
			if (WarElapsedTime < m_byBattleTime) {// Won the war.
				m_sBattleTimeDelay++;
				if (m_sBattleTimeDelay >= (m_byBattleTime / 24)) {
					m_sBattleTimeDelay = 0;
					Announcement(UNDER_ATTACK_NOTIFY);
				}
			}
		}
		if (m_bResultDelay) {
			m_sBattleResultDelay++;
			if (m_sBattleResultDelay == (m_byBattleTime / (m_byBattleTime / 10))) {
				m_bResultDelay = false;
				BattleZoneResult(m_bResultDelayVictory);
			}
		}
		if (WarElapsedTime >= m_byBattleTime) CGameServerDlg::BattleZoneClose(); // War is over.
	}

final_time:
	if (m_byBanishFlag) {
		m_sBanishDelay++;
		if (m_sBanishDelay == (m_byBattleTime / 360))
			Announcement(DECLARE_BAN);
		else if (m_sBanishDelay == (m_byBattleTime / 120)) {
			m_byBanishFlag = false;
			m_sBanishDelay = 0;
			BanishLosers();
		}
	}
}
#pragma endregion

#pragma region CGameServerDlg::SingleOtherEventTimer()
void CGameServerDlg::SingleOtherEventTimer()
{
	SingleOtherEventLocalTimer();

	if (pForgettenTemple.isActive)
		ForgettenTempleTimerProc();

	uint32 nWeekDay = g_localTime.tm_wday, nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;
	pEventTimeOpt.mtimeforlooplock.lock();
	auto copy_map = pEventTimeOpt.mtimeforloop;
	pEventTimeOpt.mtimeforlooplock.unlock();

	if (copy_map.empty())
		return;

	foreach(ptime, copy_map) {

		if (ptime->eventid != EventLocalID::UnderTheCastle && ptime->eventid != EventLocalID::ForgettenTemple && ptime->eventid != EventLocalID::BeefEvent)
			continue;

		if ((!ptime->status || ptime->type != EventType::SingleRoom || !isgetonday(ptime->iday))
			|| (ptime->eventid == EventLocalID::UnderTheCastle && m_bUnderTheCastleIsActive)
			|| (ptime->eventid == EventLocalID::ForgettenTemple && pForgettenTemple.isActive)
			|| (ptime->eventid == EventLocalID::BeefEvent && pBeefEvent.isActive))
			continue;

		for (int timei = 0; timei < EVENT_START_TIMES; timei++) {
			if (!ptime->timeactive[timei])
				continue;

			if (ptime->eventid == EventLocalID::ForgettenTemple) {

				for (int count = 1; count <= 5; count++) {
					if (count != 1 && count != 5)
						continue;

					if ((ptime->minute[timei] - count) < 0) {
						int16 sNoticeMinutes = ptime->minute[timei] - count;
						if (sNoticeMinutes > 0)
							continue;

						sNoticeMinutes = 60 + sNoticeMinutes; int16 sNoticeHour = ptime->hour[timei] - 1;

						if (sNoticeHour < 0)
							sNoticeHour = 23;

						if (sNoticeHour == nHour && sNoticeMinutes == nMinute && nSeconds == 0) {
							pForgettenTemple.NoticeStartHour = ptime->hour[timei];
							pForgettenTemple.MinLevel = pForgettenTemple.ptimeopt.MinLevel;
							pForgettenTemple.MaxLevel = pForgettenTemple.ptimeopt.MaxLevel;
							Announcement(IDS_MONSTER_CHALLENGE_ANNOUNCEMENT);
							printf("Forgetten Temple Open Notice (low) left %d Minutes \n", count);
						}
					}
					else if (ptime->hour[timei] == nHour && (ptime->minute[timei] - count == nMinute) && nSeconds == 0) {
						pForgettenTemple.NoticeStartHour = ptime->hour[timei];
						pForgettenTemple.MinLevel = pForgettenTemple.ptimeopt.MinLevel;
						pForgettenTemple.MaxLevel = pForgettenTemple.ptimeopt.MaxLevel;
						Announcement(IDS_MONSTER_CHALLENGE_ANNOUNCEMENT);
						printf("Forgetten Temple Open Notice (low) left %d Minutes \n", count);
					}
				}
			}

			if (nHour == ptime->hour[timei] && nMinute == ptime->minute[timei] && nSeconds == 0) {
				if (ptime->eventid == EventLocalID::ForgettenTemple) {
					if (m_ForgettenTempleMonsterArray.IsEmpty())
						continue;

					if (!pForgettenTemple.ptimeopt.PlayingTime || !pForgettenTemple.ptimeopt.SummonTime)
						continue;

					ForgettenTempleReset();
					ForgettenTempleStart(1, pForgettenTemple.ptimeopt.MinLevel, pForgettenTemple.ptimeopt.MaxLevel);
				}
				else if (ptime->eventid == EventLocalID::BeefEvent) {
					_BEEF_EVENT_PLAY_TIMER* pPlayInfo = m_BeefEventPlayTimerArray.GetData((uint8)EventLocalID::BeefEvent);
					if (pPlayInfo == nullptr || !pPlayInfo->MonumentTime || !pPlayInfo->LoserSignTime || !pPlayInfo->FarmingTime)
						continue;

					pBeefEvent.isActive = true;
					pBeefEvent.isAttackable = true;
					pBeefEvent.BeefMonumentStartTime = (uint32)UNIXTIME;
					pBeefEvent.BeefSendTime = pPlayInfo->MonumentTime * MINUTE;
					pBeefEvent.isMonumentDead = false;
					pBeefEvent.WictoryNation = 0;
					BeefEventUpdateTime();
					BeefEventSendNotice(1);

					printf("%d. call for the %s Event is started at %02d:%02d:%02d \n", timei, ptime->name.c_str(), nHour, nMinute, nSeconds);
				}
			}
		}
	}
}
#pragma endregion
