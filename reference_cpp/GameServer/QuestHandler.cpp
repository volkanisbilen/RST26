#include "stdafx.h"
#include "KnightsManager.h"
#include "../shared/DateTime.h"

void CUser::_getEtcList(std::vector<uint16>& metclist, bool all)
{
	if (isWarrior() || all) {
		metclist.emplace_back(334); //70 skill
		metclist.emplace_back(359); //75 skill
		metclist.emplace_back(365); //80 skill
		//metclist.emplace_back(273); //master
		metclist.emplace_back(390); //yere vurma skilleri
	}
	if (isRogue() || all) {
		metclist.emplace_back(335);
		metclist.emplace_back(347);
		metclist.emplace_back(360);
		metclist.emplace_back(366);
		//metclist.emplace_back(273);
	}
	if (isMage() || all) {
		metclist.emplace_back(336);
		metclist.emplace_back(348);
		metclist.emplace_back(361);
		metclist.emplace_back(367);
		//metclist.emplace_back(273);
	}
	if (isPriest() || all) {
		metclist.emplace_back(337);
		metclist.emplace_back(349);
		metclist.emplace_back(357);
		metclist.emplace_back(362);
		metclist.emplace_back(363);
		metclist.emplace_back(364);
		metclist.emplace_back(368);
		//metclist.emplace_back(273);
	}
	if (isPortuKurian() || all) {
		metclist.emplace_back(1377);
		metclist.emplace_back(1378);
		//metclist.emplace_back(273);
	}
}

void CUser::OpenEtcSkill(bool bAuto/*=true*/)
{
	if (bAuto && !g_pMain->pServerSetting.AutoQuestSkill)
		return;

	std::vector<uint16> metclist;
	_getEtcList(metclist, false);
	if (metclist.empty()) return;

	foreach(itr, metclist)
	{
		SaveEvent(*itr, 0);
		if (!CheckExistEvent(*itr, 2))
			SaveEvent(*itr, 2);
	}
}

void CUser::QuestDataRequest(bool gamestarted)
{
	DateTime date;

	Packet packtime(WIZ_QUEST, uint8(8));
	packtime << uint16(date.GetYear()) << uint8(date.GetMonth())
		<< uint8(date.GetDay()) << uint8(date.GetHour())
		<< uint8(date.GetMinute()) << uint8(date.GetSecond());
	Send(&packtime);

	std::vector<uint16> questlist; uint16 QuestCount = 0;
	Packet packquest(WIZ_QUEST, uint8(1));
	packquest << QuestCount;

	foreach_stlmap(itr, m_questMap)
	{
		if (itr->second == nullptr)
			continue;

		if (itr->second->QuestState == 1
			|| itr->second->QuestState == 3)
			questlist.push_back(itr->first);

		QuestCount++;
		packquest << itr->first << itr->second->QuestState;

		if (itr->first == 390 && itr->second->QuestState == 2)
		{
			Packet test(XSafe, uint8(0xDF));
			test << itr->second->QuestState;
			Send(&test);

			{
				Packet result(WIZ_PRESET);
				result << uint8(2) << uint8(1);
				for (int i = 0; i < 4; i++) result << m_bstrSkill[i + 5];
				result << m_bstrSkill[0];
				Send(&result);

			}
		}

	}
	packquest.put(1, QuestCount);
	Send(&packquest);

	foreach(a, questlist)
	{
		_USER_QUEST_INFO* pInfo = m_questMap.GetData(*a);
		if (pInfo == nullptr)
			continue;

		QuestV2MonsterDataRequest(*a);
	}
}

void CUser::QuestV2PacketProcess(Packet& pkt)
{
	uint8 opcode = pkt.read<uint8>();
	uint32 nQuestID = pkt.read<uint32>();

	CNpc* pNpc = g_pMain->GetNpcPtr(m_sEventNid, GetZoneID());
	_QUEST_HELPER* pQuestHelper = g_pMain->m_QuestHelperArray.GetData(nQuestID);

	//TRACE("opcode = %d, nQuestID = %d \n", opcode, nQuestID);
	if (opcode == 5 || opcode == 4 || opcode == 12)
	{
		if (pQuestHelper == nullptr
			// Is this quest for this player's nation? NOTE: 3 indicates both (why not 0, I don't know)
			|| (pQuestHelper->bNation != 3 && pQuestHelper->bNation != GetNation())
			// Is the player's level high enough to do this quest?
			|| (pQuestHelper->bLevel > GetLevel())
			// Are we the correct class? NOTE: 5 indicates any class.
			|| ((isRogue() || isMage() || isPriest() || isWarrior()) && (pQuestHelper->bClass != 5 && !JobGroupCheck(pQuestHelper->bClass)))
			// Are we the correct class? Krain & Porto class.
			|| (isPortuKurian() && !JobGroupCheck((short)ClassType::ClassPortuKurian)))
			return;

		goto QuestClientPacket;
	}

	// Does this quest helper exist?
	if (pQuestHelper == nullptr
		// Does the quest NPC exist, and is it alive? 
		|| (pQuestHelper->sEventDataIndex != 500 && (pNpc == nullptr || pNpc->isDead()))
		// Are we even talking to this NPC?
		|| (pQuestHelper->sEventDataIndex != 500 && (pQuestHelper->sNpcId != pNpc->GetProtoID()))
		// Are we in range of this NPC?
		|| (pQuestHelper->sEventDataIndex != 500 && !isInRange(pNpc, MAX_NPC_RANGE))
		// Is this quest for this player's nation? NOTE: 3 indicates both (why not 0, I don't know)
		|| (pQuestHelper->bNation != 3 && pQuestHelper->bNation != GetNation())
		// Is the player's level high enough to do this quest?
		|| (pQuestHelper->bLevel > GetLevel())
		// Are we the correct class? NOTE: 5 indicates any class.
		|| ((isRogue() || isMage() || isPriest() || isWarrior()) && (pQuestHelper->bClass != 5 && !JobGroupCheck(pQuestHelper->bClass)))
		// Are we the correct class? Krain & Porto class.
		|| (isPortuKurian() && !JobGroupCheck((short)ClassType::ClassPortuKurian)))
		return;

QuestClientPacket:
	{
		switch (opcode)
		{
		case 3:
		case 7:
			QuestV2ExecuteHelper(pQuestHelper);
			break;
		case 4:
			QuestV2CheckFulfill(pQuestHelper);
			break;
		case 5:
		{
			_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(pQuestHelper->sEventDataIndex);
			if (pQuestInfo == nullptr)
				break;

			SaveEvent(pQuestHelper->sEventDataIndex, 4);

			//// Kick the user out of the quest zone.
			//// Monster suppression squad is the only zone I'm aware of that this should apply to.
			if (GetZoneID() >= 81 && GetZoneID() <= 83)
				KickOutZoneUser(true);
		}
		break;
		case 12:
			if (!CheckExistEvent(pQuestHelper->sEventDataIndex, 1))
				SaveEvent(pQuestHelper->sEventDataIndex, 1);
			break;
		default:
			printf("Uncatched Num: %d\n", opcode);
			TRACE("Uncatched Num: %d\n", opcode);
			break;
		}
	}
}

/**
* @brief	Saves user quest information.
*
* @param	sQuestID	the sEventDataIndex.
* @param	bQuestState	the bEventStatus or the quest state.
*	bQuestState :
*	1) ongoing
*	2) completed
*	3) Before complition
*	4) remove
*/
void CUser::SaveEvent(uint16 sQuestID, uint8 bQuestState)
{
	Packet result;
	switch (bQuestState)
	{
	case 1:
	{
		_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);
		if (pQuestInfo == nullptr)
		{
			_USER_QUEST_INFO* pUserQuest = new _USER_QUEST_INFO();

			pUserQuest->QuestState = bQuestState;

			for (int i = 0; i < QUEST_MOBS_PER_GROUP; i++)
				pUserQuest->m_bKillCounts[i] = 0;

			if (!m_questMap.PutData(sQuestID, pUserQuest))
			{
				delete pUserQuest;
				return;
			}
			result.Initialize(WIZ_QUEST);
			result << uint8(2) << sQuestID << bQuestState;
			Send(&result);

			QuestV2MonsterDataRequest(sQuestID);
		}
		else
		{
			pQuestInfo->QuestState = bQuestState;
			result.Initialize(WIZ_QUEST);
			result << uint8(2) << sQuestID << bQuestState;
			Send(&result);
			QuestV2MonsterDataRequest(sQuestID);
		}
	}
	case 2:
	{
		_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);
		if (pQuestInfo == nullptr)
		{
			_USER_QUEST_INFO* pUserQuest = new _USER_QUEST_INFO();

			pUserQuest->QuestState = bQuestState;

			for (int i = 0; i < QUEST_MOBS_PER_GROUP; i++)
				pUserQuest->m_bKillCounts[i] = 0;

			if (!m_questMap.PutData(sQuestID, pUserQuest))
			{
				delete pUserQuest;
				return;
			}
			result.Initialize(WIZ_QUEST);
			result << uint8(2) << sQuestID << bQuestState;
			Send(&result);
			{
				if (sQuestID == 390)
				{
					Packet test(XSafe, uint8(0xDF));
					test << sQuestID;
					Send(&test);
					{
						Packet result(WIZ_PRESET);
						result << uint8(2) << uint8(1);
						for (int i = 0; i < 4; i++) result << m_bstrSkill[i + 5];
						result << m_bstrSkill[0];
						Send(&result);

					}
				}
			}
			AchieveComCountAdd(UserAchieveComTypes::AchieveRequireQuest);
		}
		else
		{
			pQuestInfo->QuestState = bQuestState;
			Packet result(WIZ_QUEST, uint8(2));
			result << sQuestID << bQuestState;
			Send(&result);

			if (sQuestID == 390 && bQuestState == 2)
			{
				Packet test(XSafe, uint8(0xDF));
				test << sQuestID;
				Send(&test);
				{
					Packet result(WIZ_PRESET);
					result << uint8(2) << uint8(1);
					for (int i = 0; i < 4; i++) result << m_bstrSkill[i + 5];
					result << m_bstrSkill[0];
					Send(&result);

				}
			}
			AchieveComCountAdd(UserAchieveComTypes::AchieveRequireQuest);
		}
	}
	break;
	case 3:
	{
		_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);
		if (pQuestInfo != nullptr)
		{
			pQuestInfo->QuestState = bQuestState;
			Packet result(WIZ_QUEST, uint8(2));
			result << sQuestID << bQuestState;
			Send(&result);
		}
	}
	break;
	case 4:
	{
		_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);
		if (pQuestInfo != nullptr)
		{
			pQuestInfo->QuestState = bQuestState;
			result.Initialize(WIZ_QUEST);
			result << uint8(2) << sQuestID << bQuestState;
			Send(&result);

			QuestV2RemoveEvent(sQuestID);
		}
	}
	break;
	default:
	{
		_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);
		if (pQuestInfo != nullptr)
		{
			pQuestInfo->QuestState = bQuestState;
			result.Initialize(WIZ_QUEST);
			result << uint8(2) << sQuestID << bQuestState;
			Send(&result);
		}
	}
	break;
	}
}

// this method is reduntant for now.
void CUser::QuestV2RemoveEvent(uint16 sQuestID)
{
	DeleteEvent(sQuestID);
}

void CUser::DeleteEvent(uint16 sQuestID)
{
	m_questMap.DeleteData(sQuestID);
}

bool CUser::CheckExistEvent(uint16 sQuestID, uint8 bQuestState)
{
	// Attempt to find a quest with that ID in the map
	_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);

	// If it doesn't exist, it doesn't exist. 
	// Unless of course, we wanted it to not exist, in which case we're right!
	// (this does seem silly, but this behaviour is STILL expected, so do not remove it.)
	if (pQuestInfo == nullptr)
		return bQuestState == 0;

	return pQuestInfo->QuestState == bQuestState;
}

uint8 CUser::GetQuestStatus(uint16 QuestID)
{
	// Attempt to find a quest with that ID in the map
	_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(QuestID);

	// If it doesn't exist, it doesn't exist. 
	// Unless of course, we wanted it to not exist, in which case we're right!
	// (this does seem silly, but this behaviour is STILL expected, so do not remove it.)
	if (pQuestInfo == nullptr)
		return 0;

	return pQuestInfo->QuestState;
}

void CUser::QuestV2MonsterCountAdd(uint16 sNpcID)
{
	if (m_questMap.GetSize() == 0)
		return;

	uint16 sQuestNum = 0;
	_QUEST_MONSTER* pQuestMonster = nullptr;

	foreach_stlmap(itr, m_questMap)
	{
		if (itr->second == nullptr
			|| itr->second->QuestState != 1)
			continue;

		sQuestNum = itr->first; // placeholder so that we can implement logic mockup
		pQuestMonster = g_pMain->m_QuestMonsterArray.GetData(sQuestNum);

		if (pQuestMonster == nullptr)
			continue;

		bool StatusCheck[4];
		memset(StatusCheck, false, sizeof(StatusCheck));

		for (int group = 0; group < QUEST_MOB_GROUPS; group++)
		{
			for (int i = 0; i < QUEST_MOBS_PER_GROUP; i++)
			{
				if (pQuestMonster->sNum[group][i] != sNpcID) // are we killing da monster for this quest?
					continue;

				// do we exceed the total number of monster kill for this event?
				if ((itr->second->m_bKillCounts[group] + 1) > pQuestMonster->sCount[group])
					continue;

				itr->second->m_bKillCounts[group]++; // well, congrats ya, you've killed a quest monster so lets increase it up
				Packet result(WIZ_QUEST, uint8(9)); // update user kill count screen
				result << uint8(2) << uint16(sQuestNum) << uint8(group + 1) << uint16(itr->second->m_bKillCounts[group]);
				Send(&result);
			}
		}

		for (int v = 0; v < QUEST_MOB_GROUPS; v++)
		{
			if (itr->second->m_bKillCounts[v] >= pQuestMonster->sCount[v])
				StatusCheck[v] = true;
		}

		if ((pQuestMonster->sCount[0] <= 0 || StatusCheck[0])
			&& (pQuestMonster->sCount[1] <= 0 || StatusCheck[1])
			&& (pQuestMonster->sCount[2] <= 0 || StatusCheck[2])
			&& (pQuestMonster->sCount[3] <= 0 || StatusCheck[3]))
			SaveEvent(sQuestNum, 3);
	}
}

/*
* @brief	Checks if the specified quest is finished or not.
*
* @param	sQuestID	the Quest ID to be checked.
*/
uint8 CUser::QuestV2CheckQuestFinished(uint16 sQuestID)
{
	// Attempt to find a quest with that ID in the map
	_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);
	//// If it doesn't exist, it doesn't exist. 
	if (pQuestInfo == nullptr)
		return 0;

	return  pQuestInfo->QuestState == 3; // quest finished?
}

void CUser::QuestV2MonsterDataRequest(uint16 sQuestID)
{
	_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);
	if (pQuestInfo == nullptr)
		return;

	Packet result(WIZ_QUEST, uint8(9));
	result << uint8(1) << sQuestID;

	for (int i = 0; i < QUEST_MOBS_PER_GROUP; i++)
		result << (uint16)pQuestInfo->m_bKillCounts[i];

	Send(&result);
}

uint8 CUser::QuestV2CheckMonsterCount(uint16 sQuestID, uint8 sRate)
{
	_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(sQuestID);
	if (pQuestInfo == nullptr)
		return 0;

	return (uint8)pQuestInfo->m_bKillCounts[sRate - 1];
}

void CUser::QuestV2ExecuteHelper(_QUEST_HELPER* pQuestHelper)
{
	if (pQuestHelper == nullptr
		|| CheckExistEvent(pQuestHelper->sEventDataIndex, 2))
		return;

	QuestV2RunEvent(pQuestHelper, pQuestHelper->nEventTriggerIndex, -1); // NOTE: Fulfill will use nEventCompleteIndex
}

void CUser::QuestV2CheckFulfill(_QUEST_HELPER* pQuestHelper)
{
	if (pQuestHelper == nullptr)
		return;

	// Attempt to find a quest with that ID in the map
	_USER_QUEST_INFO* pQuestInfo = m_questMap.GetData(pQuestHelper->sEventDataIndex);
	if (pQuestInfo == nullptr)
		return;

	if (pQuestInfo->QuestState != 1)
		return;

	uint16 sQuestNum = pQuestHelper->sEventDataIndex;
	_QUEST_MONSTER* pQuestMonster = g_pMain->m_QuestMonsterArray.GetData(pQuestHelper->sEventDataIndex);
	if (pQuestMonster != nullptr)
	{
		if (pQuestMonster->sQuestNum != 812)
		{
			for (int group = 0; group < QUEST_MOB_GROUPS; group++)
			{
				for (int i = 0; i < QUEST_MOBS_PER_GROUP; i++)
				{
					if (pQuestInfo->m_bKillCounts[group] != pQuestMonster->sCount[group])
						return;
				}
			}
		}
	}
	QuestV2RunEvent(pQuestHelper, pQuestHelper->nEventCompleteIndex, -1);
}

bool CUser::QuestV2RunEvent(_QUEST_HELPER* pQuestHelper, uint32 nEventID, int8 bSelectedReward /*= -1*/)
{
	bool result = false;
	if (pQuestHelper == nullptr)
		return false;

	// Make sure the NPC exists and is not dead (we should also check if it's in range)
	if (pQuestHelper->sEventDataIndex != 500) {
		// Lookup the corresponding NPC. 
		CNpc* pNpc = g_pMain->GetNpcPtr(m_sEventNid, GetZoneID());
		if (pNpc == nullptr /*|| pNpc->isDead()*/)
			return result;

		// Increase the NPC's reference count to ensure it doesn't get freed while executing a script
		pNpc->IncRef();

		m_nQuestHelperID = pQuestHelper->nIndex;
		result = g_pMain->GetLuaEngine()->ExecuteScript(this, pNpc, nEventID, bSelectedReward,
			pQuestHelper->strLuaFilename.c_str());

		// Decrease it now that we've finished with it + free if necessary
		pNpc->DecRef();
	}
	else {
		m_nQuestHelperID = pQuestHelper->nIndex;
		result = g_pMain->GetLuaEngine()->ExecuteScript(this, nullptr, nEventID, bSelectedReward,
			pQuestHelper->strLuaFilename.c_str());
	}
	return result;
}

/*
These are called by quest scripts.
*/

void CUser::QuestV2SaveEvent(uint16 sQuestID)
{
	_QUEST_HELPER* pQuestHelper = g_pMain->m_QuestHelperArray.GetData(sQuestID);
	if (pQuestHelper == nullptr)
		return;

	SaveEvent(pQuestHelper->sEventDataIndex, pQuestHelper->bEventStatus);
}

void CUser::QuestV2SendNpcMsg(uint32 nQuestID, uint16 sNpcID)
{
	Packet result(WIZ_QUEST, uint8(7));
	result << nQuestID << sNpcID;
	Send(&result);
}

void CUser::LuaGiveItemShowGiveItem(uint32 nUnk1, uint32 sUnk1,
	uint32 nUnk2, uint32 sUnk2,
	uint32 nUnk3, uint32 sUnk3,
	uint32 nUnk4, uint32 sUnk4,
	uint32 nUnk5, uint32 sUnk5,
	uint32 nUnk6, uint32 sUnk6,
	uint32 nUnk7, uint32 sUnk7,
	uint32 nUnk8, uint32 sUnk8,
	uint32 nUnk9, uint32 sUnk9,
	uint32 nUnk10, uint32 sUnk10)
{
	Packet result(WIZ_QUEST, uint8(10));
	result << nUnk1 << sUnk1
		<< nUnk2 << sUnk2
		<< nUnk3 << sUnk3
		<< nUnk4 << sUnk4
		<< nUnk5 << sUnk5
		<< nUnk6 << sUnk6
		<< nUnk7 << sUnk7
		<< nUnk8 << sUnk8
		<< nUnk9 << sUnk9
		<< nUnk10 << sUnk10;
	Send(&result);
}

void CUser::QuestV2ShowGiveItem(uint32 ItemID1, uint32 ItemCount1,
	uint32 ItemID2, uint32 ItemCount2,
	uint32 ItemID3, uint32 ItemCount3,
	uint32 ItemID4, uint32 ItemCount4,
	uint32 ItemID5, uint32 ItemCount5,
	uint32 ItemID6, uint32 ItemCount6,
	uint32 ItemID7, uint32 ItemCount7,
	uint32 ItemID8, uint32 ItemCount8)
{
	Packet result(WIZ_QUEST, uint8(10));
	result << ItemID1 << ItemCount1
		<< ItemID2 << ItemCount2
		<< ItemID3 << ItemCount3
		<< ItemID4 << ItemCount4
		<< ItemID5 << ItemCount5
		<< ItemID6 << ItemCount6
		<< ItemID7 << ItemCount7
		<< ItemID8 << ItemCount8;
	Send(&result);
}

uint16 CUser::QuestV2SearchEligibleQuest(uint16 sNpcID)
{
	g_pMain->m_questNpcLock.lock();
	QuestNpcList::iterator itr = g_pMain->m_QuestNpcList.find(sNpcID);
	g_pMain->m_questNpcLock.unlock();
	if (itr == g_pMain->m_QuestNpcList.end() || itr->second.empty())
		return 0;

	// Loop through all the QuestHelper instances attached to that NPC.
	foreach(itr2, itr->second)
	{
		_QUEST_HELPER* pHelper = (*itr2);
		if (pHelper->bLevel > GetLevel()
			|| (pHelper->bLevel == GetLevel() && pHelper->nExp > m_iExp)
			|| ((isRogue() || isMage() || isPriest() || isWarrior()) && (pHelper->bClass != 5 && !JobGroupCheck(pHelper->bClass)))
			|| (isPortuKurian() && !JobGroupCheck((short)ClassType::ClassPortuKurian))
			|| (pHelper->bNation != 3 && pHelper->bNation != GetNation())
			|| (pHelper->sEventDataIndex == 0)
			|| (pHelper->bEventStatus < 0 || CheckExistEvent(pHelper->sEventDataIndex, 2))
			|| !CheckExistEvent(pHelper->sEventDataIndex, pHelper->bEventStatus))
			continue;

		return 2;
	}
	return 0;
}

void CUser::QuestV2ShowMap(uint32 nQuestHelperID)
{
	Packet result(WIZ_QUEST, uint8(11));
	result << nQuestHelperID;
	Send(&result);
}

// First job change; you're a [novice], Harry!
bool CUser::PromoteUserNovice()
{
	uint8 bNewClasses[] = { (uint8)ClassType::ClassWarriorNovice, (uint8)ClassType::ClassRogueNovice, (uint8)ClassType::ClassMageNovice, (uint8)ClassType::ClassPriestNovice, (uint8)ClassType::ClassPortuKurianNovice };
	uint8 bOldClass = 0;

	if (isPortuKurian())
		bOldClass = 4; // convert base class 1,2,3,4,5 to 0,1,2,3,4 to align with bNewClasses
	else
		bOldClass = GetClassType() - 1; // convert base class 1,2,3,4,5 to 0,1,2,3,4 to align with bNewClasses

	// Make sure it's a beginner class.
	if (!isBeginner())
		return false;

	Packet result(WIZ_CLASS_CHANGE, uint8(6));

	// Build the new class.
	uint16 sNewClass = (GetNation() * 100) + bNewClasses[bOldClass];
	result << sNewClass << GetID();
	SendToRegion(&result);

	// Change the class & update party.
	result.clear();
	result << uint8(2) << sNewClass;
	ClassChange(result, false); // TODO: Clean this up. Shouldn't need to build a packet for this.

	// Update the clan.
	result.clear();
	result << uint16(0);
	CKnightsManager::KnightsCurrentMember(this, result); // TODO: Clean this up too.
	return true;
}

// From novice to master.
bool CUser::PromoteUser()
{
	/* unlike the official, the checks & item removal should be handled in the script, not here */
	uint8 bOldClass = GetClassType();

	// We must be a novice before we can be promoted to master.
	if (!isNovice())
		return false;

	Packet result(WIZ_CLASS_CHANGE, uint8(6));

	// Build the new class.
	uint16 sNewClass = (GetNation() * 100) + bOldClass + 1;
	result << sNewClass << GetID();
	SendToRegion(&result);

	// Change the class & update party.
	result.clear();
	result << uint8(2) << sNewClass;
	ClassChange(result, false); // TODO: Clean this up. Shouldn't need to build a packet for this.

	// use integer division to get from 5/7/9/11 (novice classes) to 1/2/3/4 (base classes)
	uint8 bBaseClass = (bOldClass / 2) - 1;

	// this should probably be moved to the script
	SaveEvent(bBaseClass, 2);

	// Update the clan.
	result.clear();
	result << uint16(0);
	CKnightsManager::KnightsCurrentMember(this, result); // TODO: Clean this up too.
	return true;
}

void CUser::PromoteClan(ClanTypeFlag byFlag)
{
	if (!isInClan())
		return;

	CKnightsManager::UpdateKnightsGrade(GetClanID(), (uint8)byFlag);
}

void CUser::SendClanPointChange(int32 nChangeAmount)
{
	if (!isInClan())
		return;

	CKnightsManager::UpdateClanPoint(GetClanID(), nChangeAmount);
}

uint8 CUser::GetClanGrade()
{
	if (!isInClan())
		return 0;

	CKnights* pClan = g_pMain->GetClanPtr(GetClanID());
	if (pClan == nullptr)
		return 0;

	return pClan->m_byGrade;
}

uint32 CUser::GetClanPoint()
{
	if (!isInClan())
		return 0;

	CKnights* pClan = g_pMain->GetClanPtr(GetClanID());
	if (pClan == nullptr)
		return 0;

	return pClan->m_nClanPointFund;
}

uint8 CUser::GetClanRank()
{
	if (!isInClan())
		return (uint8)ClanTypeFlag::ClanTypeNone;

	CKnights* pClan = g_pMain->GetClanPtr(GetClanID());
	if (pClan == nullptr)
		return (uint8)ClanTypeFlag::ClanTypeNone;

	return pClan->m_byFlag;
}


uint8 CUser::GetWarVictory() { return g_pMain->m_bVictory; }

uint8 CUser::CheckMiddleStatueCapture() { return g_pMain->m_bMiddleStatueNation == GetNation() ? 1 : 0; }

void CUser::MoveMiddleStatue() { Warp((uint16)((GetNation() == (uint8)Nation::KARUS ? DODO_CAMP_WARP_X : LAON_CAMP_WARP_X) + myrand(0, DODO_LAON_WARP_RADIUS)) * 10, (uint16)((GetNation() == (uint8)Nation::KARUS ? DODO_CAMP_WARP_Z : LAON_CAMP_WARP_Z) + myrand(0, DODO_LAON_WARP_RADIUS)) * 10); }

uint8 CUser::GetPVPMonumentNation() { return g_pMain->m_nPVPMonumentNation[GetZoneID()]; }