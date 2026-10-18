#include "StdAfx.h"
#include "DBAgent.h"

int32 GetRemeaningtime(uint32 time) { return time - (uint32)UNIXTIME; }
void CUser::UpdateDailyQuestState(_DAILY_USERQUEST* pquest, uint8 newstate) { if (pquest)pquest->status = newstate; }

void CUser::DailyQuestFinished(_DAILY_USERQUEST* puserq, _DAILY_QUEST* pquest) {
	if (!puserq || !pquest) return;
	g_pMain->ServerLog("DAILYQ_FINISH", "user=%s quest=%u", GetName().c_str(), pquest->index);

	if (pquest->replaytime) puserq->replaytime = (uint32)UNIXTIME + (pquest->replaytime * HOUR);
	if (pquest->timetype == (uint8)DailyQuesttimetype::repeat) UpdateDailyQuestState(puserq, (uint8)DailyQuestStatus::ongoing);
	else if (pquest->timetype == (uint8)DailyQuesttimetype::single) UpdateDailyQuestState(puserq, (uint8)DailyQuestStatus::comp);
	else if (pquest->timetype == (uint8)DailyQuesttimetype::time) UpdateDailyQuestState(puserq, (uint8)DailyQuestStatus::timewait);
	memset(puserq->kcount, 0, sizeof(puserq->kcount));

	Packet newpkt(WIZ_DB_SAVE_USER, uint8(ProcDbType::DailyQuestReward));
	newpkt << pquest->index;
	g_pMain->AddDatabaseRequest(newpkt, this);
}


void CUser::ReqDailyQuestSendReward(Packet& pkt) {
	uint8 index = pkt.read<uint8>();
	_DAILY_QUEST* pquest = g_pMain->m_DailyQuestArray.GetData(index);
	if (!pquest) return;
	g_pMain->ServerLog("DAILYQ_REWARD", "user=%s quest=%u", GetName().c_str(), index);

	struct list { uint32 itemid; uint32 itemcount; list(uint32 a, uint32 b) { itemid = a; itemcount = b; } };

	std::map <uint8, list> mlist; uint8 counter = 0;
	std::vector<uint8> mdeleted;

	bool randitem = false;
	for (uint8 i = 0; i < 4; i++) {
		if (pquest->rewaditemid[i] && pquest->rewarditemcount[i]) {

			if (pquest->rewaditemid[i] == 900004000) {
				if (pquest->randomid)
					randitem = true;
			}
			else
				mlist.insert(std::make_pair(counter++, list(pquest->rewaditemid[i], pquest->rewarditemcount[i])));
		}
	}

	if (randitem) {
		std::vector<uint32> m_randlist;
		foreach(itr, g_pMain->m_RandomItemArray) {
			auto* pRandom = *itr;
			if (!pRandom || pRandom->SessionID != pquest->randomid) continue;
			m_randlist.push_back(pRandom->ItemID);
		}
		if (m_randlist.size()) {
			uint32 n_randitem = m_randlist[myrand(0, (int32)m_randlist.size() - 1)];
			if (n_randitem) mlist.insert(std::make_pair(counter++, list(n_randitem, 1)));
		}
	}

	if (mlist.empty()) return;

	QuestV2ShowGiveItem(pquest->rewaditemid[0], pquest->rewarditemcount[0],
		pquest->rewaditemid[1], pquest->rewarditemcount[1],
		pquest->rewaditemid[2], pquest->rewarditemcount[2],
		pquest->rewaditemid[3], pquest->rewarditemcount[3]);

	bool lettercheck = false;
	foreach(itr, mlist) {
		_ITEM_TABLE pTable = g_pMain->GetItemPtr(itr->second.itemid);
		if (pTable.isnull()) continue;

		if (pTable.Getnum() == ITEM_EXP) {
			ExpChange("daily quest", itr->second.itemcount, true);
			continue;
		}
		else if (pTable.Getnum() == ITEM_GOLD) {
			GoldGain(itr->second.itemcount, true, false);
			continue;
		}
		else if (pTable.Getnum() == ITEM_COUNT || pTable.Getnum() == ITEM_LADDERPOINT) {
			SendLoyaltyChange("Daily Quest", itr->second.itemcount);
			continue;
		}

		int8 pos = FindSlotForItem(itr->second.itemid, itr->second.itemcount);
		if (pos > 0) {
			GiveItem("Daily Quest", itr->second.itemid, itr->second.itemcount, true);
			continue;
		}

		std::string SenderName = "ADMIN", Subject = "REWARD", Message = "REWARD";
		_ITEM_DATA pItem{};
		pItem.nNum = itr->second.itemid;
		pItem.nSerialNum = g_pMain->GenerateItemSerial();
		pItem.sCount = itr->second.itemcount;
		pItem.sDuration = pTable.m_sDuration;
		pItem.nExpirationTime = 0;
		g_DBAgent.SendLetter(SenderName, GetName(), Subject, Message, 2, &pItem, 0);
		if (!lettercheck) lettercheck = true;
	}
	if (lettercheck) ReqLetterUnread();
}

void CUser::UpdateDailyQuestCount(uint16 monsterid) {
	if (m_dailyquestmap.IsEmpty() || !monsterid) return;
	foreach_stlmap(itr, m_dailyquestmap) {
		if (!itr->second || itr->second->status == (uint8)DailyQuestStatus::comp) continue;
		if (itr->second->status == (uint8)DailyQuestStatus::timewait && GetRemeaningtime(itr->second->replaytime) > 0) continue;

		auto* pmain = g_pMain->m_DailyQuestArray.GetData(itr->first);
		if (!pmain) continue;

		if (GetLevel() < pmain->minlevel || GetLevel() > pmain->maxlevel)
			continue;

		if (pmain->premiumstatus == 1 && GetPremium() == 0)
			continue;

		if (pmain->zoneid) {
			if (pmain->zoneid == ZONE_MORADON && !isInMoradon()) continue;
			else if (pmain->zoneid == ZONE_KARUS && !isInLufersonCastle()) continue;
			else if (pmain->zoneid == ZONE_ELMORAD && !isInElmoradCastle()) continue;

			else if (pmain->zoneid == ZONE_KARUS_ESLANT && !isInKarusEslant()) continue;
			else if (pmain->zoneid == ZONE_ELMORAD_ESLANT && !isInElmoradEslant()) continue;
			else if (pmain->zoneid != GetZoneID()) continue;
		}

		if ((pmain->killtype == 0 && isInParty()) || (pmain->killtype == 1 && !isInParty())) continue;

		bool killcheck = false;
		for (uint8 i = 0; i < 4; i++) {
			if (monsterid != pmain->Mobid[i]) continue;
			killcheck = true;
		}
		if (!killcheck) continue;

		for (int group = 0; group < QUEST_MOB_GROUPS; group++) {
			if (pmain->Mobid[group] != monsterid)
				continue;

			if ((itr->second->kcount[group] + 1) > pmain->kcount[group])
				continue;

			itr->second->kcount[group]++;
			Packet pkt(XSafe, uint8(XSafeOpCodes::DailyQuest));
			pkt << uint8(DailyQuestOp::killupdate) << itr->first << monsterid;
			Send(&pkt);

			{
				Packet newpkt(XSafe, uint8(0xDC));
				newpkt.SByte();
				newpkt << pmain->strQuestName << uint16(itr->second->kcount[group]) << uint16(pmain->kcount[group]) << monsterid;

				Send(&newpkt);
			}
		}

		if (itr->second->kcount[0] >= pmain->kcount[0] &&
			itr->second->kcount[1] >= pmain->kcount[1] &&
			itr->second->kcount[2] >= pmain->kcount[2] &&
			itr->second->kcount[3] >= pmain->kcount[3])
			DailyQuestFinished(itr->second, pmain);
	}
}

void CUser::DailyQuestSendList() {
	uint16 counter = 0;
	Packet list(XSafe, uint8(XSafeOpCodes::DailyQuest));
	list << uint8(DailyQuestOp::sendlist) << counter;
	g_pMain->m_DailyQuestArray.m_lock.lock();
	foreach_stlmap_nolock(itr, g_pMain->m_DailyQuestArray) {
		auto* p = itr->second;
		if (!p) continue;

		if (p->premiumstatus == 1 && GetPremium() == 0)
			continue;

		list << p->index << p->timetype << p->killtype;
		for (uint8 i = 0; i < 4; i++)
			list << p->Mobid[i] << p->kcount[i] << p->rewaditemid[i] << p->rewarditemcount[i];

		list << p->zoneid << p->replaytime << p->minlevel << p->maxlevel << p->strQuestName << p->premiumstatus;
		counter++;
	}
	g_pMain->m_DailyQuestArray.m_lock.unlock();
	list.put(2, counter);
	Send(&list);

	counter = 0;
	Packet userlist(XSafe, uint8(XSafeOpCodes::DailyQuest));
	userlist << uint8(DailyQuestOp::userinfo) << counter;
	m_dailyquestmap.m_lock.lock();
	foreach_stlmap_nolock(itr, m_dailyquestmap)
	{
		if (!itr->second)
			continue;

		int remtime = GetRemeaningtime(itr->second->replaytime);
		if (remtime < 0) {
			remtime = 0;
			if (itr->second->status == (uint8)DailyQuestStatus::timewait) itr->second->status = (uint8)DailyQuestStatus::ongoing;
		}
		userlist << itr->first << itr->second->status;
		for (int i = 0; i < 4; i++)
			userlist << itr->second->kcount[i];

		userlist << (uint32)remtime;
		counter++;
	}
	m_dailyquestmap.m_lock.unlock();
	userlist.put(2, counter);
	Send(&userlist);
}
