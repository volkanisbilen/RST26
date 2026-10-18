#include "stdafx.h"

#pragma region CGameServerDlg::Send_CommandChat(Packet *pkt, int nation, CUser* pExceptUser)

void CGameServerDlg::Send_CommandChat(Packet* pkt, int nation, CUser* pExceptUser)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (nation == 0 || nation == pUser->GetNation())
			pUser->Send(pkt);
	}
}
#pragma endregion

#pragma region CGameServerDlg::UserInOutForMe(CUser *pSendUser)
void CGameServerDlg::UserInOutForMe(CUser* pSendUser)
{
	if (pSendUser == nullptr || pSendUser->m_strUserID.empty())
		return;

	Packet result(WIZ_REQ_USERIN);

	C3DMap* pMap = pSendUser->GetMap();
	if (pMap == nullptr)
		return;

	uint16 user_count = 0;
	result << uint16(0); // placeholder for the user count
	std::vector<CUser*> mList;
	int16 rx = pSendUser->GetRegionX(), rz = pSendUser->GetRegionZ();
	foreach_region(x, z)
	{
		CRegion* pRegion = pMap->GetRegion(rx + x, rz + z);
		if (pRegion == nullptr)
			continue;

		pRegion->m_lockUserArray.lock();
		if (pRegion->m_RegionUserArray.empty())
		{
			pRegion->m_lockUserArray.unlock();
			continue;
		}

		ZoneUserArray cm_RegionUserArray = pRegion->m_RegionUserArray;
		pRegion->m_lockUserArray.unlock();

		foreach(itr, cm_RegionUserArray)
		{
			CUser* pUser = GetUserPtr(*itr);
			if (pUser == nullptr
				|| !pUser->isInGame()
				|| pUser->m_bZone != pSendUser->m_bZone)
				continue;

			if (pSendUser->GetSocketID() == pUser->GetSocketID())
				continue;

			if (!pSendUser->isGM() && pSendUser->GetEventRoom() > 0 && pSendUser->GetEventRoom() != pUser->GetEventRoom())
				continue;

			if (pUser->m_bAbnormalType == ABNORMAL_INVISIBLE) // we should not see GMs if they "unview"
				continue;

			if (!pSendUser->CheckInvisibility(pUser))
				continue;

			result << uint8(0) << pUser->GetSocketID();

			pUser->GetUserInfo(result);

			mList.push_back(pUser);
			user_count++;
		}
	}

	result.put(0, uint16(user_count));

	pSendUser->SendCompressed(&result);
	g_pMain->MerchantUserInOutForMe(pSendUser);
	if (!mList.empty()) pSendUser->UserInOutTag(mList);

	BotInOutForMe(pSendUser);
}
#pragma endregion
void CGameServerDlg::BotInOutForMe(CUser* pSendUser)
{
	if (pSendUser == nullptr || pSendUser->m_strUserID.empty())
		return;

	Packet result(WIZ_REQ_USERIN);

	C3DMap* pMap = pSendUser->GetMap();
	if (pMap == nullptr)
		return;

	uint16 user_count = 0;
	result << uint16(10); // placeholder for the user count

	int16 rx = pSendUser->GetRegionX(), rz = pSendUser->GetRegionZ();
	foreach_region(x, z)
	{
		CRegion* pRegion = pMap->GetRegion(rx + x, rz + z);
		if (pRegion == nullptr)
			continue;

		pRegion->m_lockBotArray.lock();
		if (pRegion->m_RegionBotArray.size() <= 0)
		{
			pRegion->m_lockBotArray.unlock();
			continue;
		}

		ZoneUserArray cm_RegionUserArray = pRegion->m_RegionBotArray;
		pRegion->m_lockBotArray.unlock();

		foreach(itr, cm_RegionUserArray)
		{
			CBot* pUser = (g_pMain->GetBotPtr(*itr));

			if (pUser == nullptr)
				continue;

			if (!pUser->isInGame() || pUser->m_bZone != pSendUser->m_bZone)
				continue;

			if (pUser->m_bAbnormalType == ABNORMAL_INVISIBLE) // we should not see GMs if they "unview"
				continue;

			result << uint8(0) << pUser->GetID();
			pUser->GetUserInfo(result);
			user_count++;
		}
	}

	result.put(0, uint16(user_count));
	pSendUser->SendCompressed(&result);
}
#pragma region CGameServerDlg::RegionUserInOutForMe(CUser *pSendUser)

void CGameServerDlg::RegionUserInOutForMe(CUser* pSendUser)
{
	if (pSendUser == nullptr)
		return;

	Packet result(WIZ_REGIONCHANGE, uint8(0));
	pSendUser->SendCompressed(&result);

	Packet result1(WIZ_REGIONCHANGE, uint8(1));
	C3DMap* pMap = pSendUser->GetMap();

	if (pMap == nullptr)
		return;

	uint16 user_count = 0;
	result1 << uint16(0); // placeholder for the user count

	int16 rx = pSendUser->GetRegionX(), rz = pSendUser->GetRegionZ();

	foreach_region(x, z)
	{
		GetRegionUserList(pMap, rx + x, rz + z, result1, user_count, pSendUser->GetEventRoom(), false, pSendUser);
		GetRegionBotList(pMap, rx + x, rz + z, result1, user_count, pSendUser->GetEventRoom(), false);
	}

	result1.put(1, user_count);
	pSendUser->SendCompressed(&result1);

	Packet result2(WIZ_REGIONCHANGE, uint8(2));
	pSendUser->SendCompressed(&result2);
}

#pragma endregion

#pragma region CGameServerDlg::GetRegionBotIn(C3DMap *pMap, uint16 region_x, uint16 region_z, Packet & pkt, uint16 & t_count, uint16 nEventRoom)
void CGameServerDlg::GetRegionBotIn(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom)
{
	if (pMap == nullptr)
		return;

	CRegion* pRegion = pMap->GetRegion(region_x, region_z);
	if (pRegion == nullptr)
		return;

	int16 sMaxCount = 150;
	int16 sTotalCount = 0;

	pRegion->m_lockBotArray.lock();

	if (pRegion->m_RegionBotArray.size() <= 0)
	{
		pRegion->m_lockBotArray.unlock();
		return;
	}

	ZoneBotArray cm_RegionUserBotArray = pRegion->m_RegionBotArray;
	pRegion->m_lockBotArray.unlock();

	std::vector<CBot*> deleted;
	foreach(itr, cm_RegionUserBotArray)
	{
		CBot* pUser = GetBotPtr(*itr);

		if (pUser == nullptr
			|| !pUser->isInGame())
			continue;

		if (pUser->LastWarpTime > 0 && pUser->LastWarpTime < UNIXTIME)
			deleted.push_back(pUser);

		if (sTotalCount > sMaxCount)
			continue;

		pkt << uint8(0) << pUser->GetID();
		pUser->GetUserInfo(pkt);

		t_count++;
		sTotalCount++;
	}

	foreach(itr, deleted)
		(*itr)->UserInOut(INOUT_OUT);
}
#pragma endregion

#pragma region  CGameServerDlg::GetRegionBotList(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet & pkt, uint16 & t_count, uint16 nEventRoom, bool SurUpdate /* = false */)
void CGameServerDlg::GetRegionBotList(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom, bool SurUpdate /* = false */)
{
	if (pMap == nullptr)
		return;

	CRegion* pRegion = pMap->GetRegion(region_x, region_z);
	if (pRegion == nullptr)
		return;

	pRegion->m_lockBotArray.lock();
	if (pRegion->m_RegionBotArray.size() <= 0)
	{
		pRegion->m_lockBotArray.unlock();
		return;
	}

	ZoneBotArray cm_RegionUserBotArray = pRegion->m_RegionBotArray;
	pRegion->m_lockBotArray.unlock();

	std::vector<CBot*> deleted;
	foreach(itr, cm_RegionUserBotArray)
	{
		CBot* pUser = GetBotPtr(*itr);

		if (pUser == nullptr
			|| !pUser->isInGame())
			continue;

		if (pUser->LastWarpTime > 0 && pUser->LastWarpTime < UNIXTIME)
			deleted.push_back(pUser);

		pkt << pUser->GetID();
		t_count++;
	}

	foreach(itr, deleted)
		(*itr)->UserInOut(INOUT_OUT);
}
#pragma endregion

#pragma region  CGameServerDlg::GetRegionUserList(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet & pkt, uint16 & t_count, uint16 nEventRoom, bool SurUpdate /* = false */)
void CGameServerDlg::GetRegionUserList(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom, bool SurUpdate /* = false */, CUser* pExcept /* = nullptr*/)
{
	if (pMap == nullptr)
		return;

	CRegion* pRegion = pMap->GetRegion(region_x, region_z);
	if (pRegion == nullptr)
		return;

	pRegion->m_lockUserArray.lock();
	if (pRegion->m_RegionUserArray.size() <= 0)
	{
		pRegion->m_lockUserArray.unlock();
		return;
	}

	std::vector<CUser*> mList;
	ZoneUserArray cm_RegionUserArray = pRegion->m_RegionUserArray;
	pRegion->m_lockUserArray.unlock();
	foreach(itr, cm_RegionUserArray)
	{
		CUser* pUser = GetUserPtr(*itr);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (nEventRoom >= 0 && nEventRoom != pUser->GetEventRoom())
			continue;

		if (pExcept && pExcept->GetSocketID() == pUser->GetSocketID())
			continue;

		if (pUser->m_bAbnormalType == ABNORMAL_INVISIBLE) // we should not see GMs if they "unview"
			continue;

		/*if (sTotalCount > sMaxCount)
			continue;*/

		pkt << pUser->GetSocketID();
		mList.push_back(pUser);
		t_count++;
	}

}

#pragma endregion

#pragma region CGameServerDlg::MerchantUserInOutForMe(CUser *pSendUser)
void CGameServerDlg::MerchantUserInOutForMe(CUser* pSendUser)
{
	if (pSendUser == nullptr)
		return;

	C3DMap* pMap = pSendUser->GetMap();
	if (pMap == nullptr) return;

	Packet result(WIZ_MERCHANT_INOUT, uint8(1));
	uint16 user_count = 0;

	result << uint16(0);
	int16 rx = pSendUser->GetRegionX(), rz = pSendUser->GetRegionZ();
	foreach_region(x, z)
	{
		g_pMain->GetRegionMerchantUserIn(pMap, rx + x, rz + z, result, user_count, pSendUser->GetEventRoom());
		g_pMain->GetRegionMerchantBotIn(pMap, rx + x, rz + z, result, user_count, pSendUser->GetEventRoom());
	}
	result.put(1, user_count);
	pSendUser->SendCompressed(&result);
}
#pragma endregion

#pragma region  CGameServerDlg::GetRegionMerchantUserIn(C3DMap *pMap, uint16 region_x, uint16 region_z, Packet & pkt, uint16 & t_count, uint16 nEventRoom)
void CGameServerDlg::GetRegionMerchantUserIn(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom)
{
	if (pMap == nullptr)
		return;

	CRegion* pRegion = pMap->GetRegion(region_x, region_z);
	if (pRegion == nullptr || pRegion->m_RegionUserArray.size() <= 0)
		return;

	pRegion->m_lockUserArray.lock();
	ZoneUserArray cm_RegionUserArray = pRegion->m_RegionUserArray;
	pRegion->m_lockUserArray.unlock();
	foreach(itr, cm_RegionUserArray)
	{
		CUser* pUser = GetUserPtr(*itr);
		if (pUser == nullptr || !pUser->isInGame() || !pUser->isMerchanting())
			continue;

		if (nEventRoom != pUser->GetEventRoom())
			continue;

		if (pUser->m_bAbnormalType == AbnormalType::ABNORMAL_INVISIBLE)
			continue;

		pkt << pUser->GetSocketID() 
			<< pUser->GetMerchantState() 
			<< (pUser->GetMerchantState() == 1 ? false : pUser->m_bPremiumMerchant);
		t_count++;
	}
}
#pragma endregion

#pragma region  CGameServerDlg::GetRegionMerchantBotIn(C3DMap *pMap, uint16 region_x, uint16 region_z, Packet & pkt, uint16 & t_count, uint16 nEventRoom)
void CGameServerDlg::GetRegionMerchantBotIn(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom)
{
	if (pMap == nullptr)
		return;

	CRegion* pRegion = pMap->GetRegion(region_x, region_z);
	if (pRegion == nullptr || pRegion->m_RegionBotArray.size() <= 0)
		return;

	pRegion->m_lockBotArray.lock();
	ZoneBotArray cm_RegionUserArrays = pRegion->m_RegionBotArray;
	pRegion->m_lockBotArray.unlock();

	std::vector<CBot*> deleted;
	foreach(itr, cm_RegionUserArrays)
	{
		CBot* pUser = (g_pMain->GetBotPtr(*itr));
		if (pUser == nullptr || !pUser->isInGame() || !pUser->isMerchanting())
			continue;

		if (pUser->LastWarpTime > 0 && pUser->LastWarpTime < UNIXTIME)
			deleted.push_back(pUser);

		if (pUser->m_bAbnormalType == AbnormalType::ABNORMAL_INVISIBLE)
			continue;

		pkt << pUser->GetID() << pUser->GetMerchantState() << (pUser->GetMerchantState() == 1 ? false : pUser->m_bPremiumMerchant);
		t_count++;
	}

	foreach(itr, deleted)
		(*itr)->UserInOut(INOUT_OUT);
}

#pragma endregion

#pragma region CGameServerDlg::NpcInOutForMe(CUser* pSendUser)

void CUser::NpcInOutForMe()
{
	Packet result(WIZ_REQ_NPCIN);
	C3DMap* pMap = GetMap();
	if (pMap == nullptr) return;
	uint16 npc_count = 0;
	result << uint16(0); // placeholder for NPC count
	int16 rx = GetRegionX(), rz = GetRegionZ();
	foreach_region(x, z)
	{
		CRegion* pRegion = pMap->GetRegion(rx + x, rz + z);
		if (pRegion == nullptr) continue;

		pRegion->m_lockNpcArray.lock();
		if (pRegion->m_RegionNpcArray.empty())
		{
			pRegion->m_lockNpcArray.unlock();
			continue;
		}

		ZoneNpcArray cm_RegionNpcArray = pRegion->m_RegionNpcArray;
		pRegion->m_lockNpcArray.unlock();

		foreach(itr, cm_RegionNpcArray)
		{
			CNpc* pNpc = g_pMain->GetNpcPtr(*itr, GetZoneID());
			if (pNpc == nullptr || pNpc->isDead() || pNpc->GetZoneID() != GetZoneID() || pNpc->GetEventRoom() != GetEventRoom())
				continue;

			result << pNpc->GetID();
			pNpc->GetNpcInfo(result, GetClanID());
			npc_count++;

			if (result.size() > 500)
			{
				result.put(0, npc_count);
				SendCompressed(&result);
				npc_count = 0;
				result.Initialize(WIZ_REQ_NPCIN);
				result << uint16(0); // placeholder for NPC count
			}
		}
	}
	result.put(0, npc_count);
	SendCompressed(&result);
}

#pragma endregion

#pragma region CGameServerDlg::GetRegionNpcIn(C3DMap *pMap, uint16 region_x, uint16 region_z, Packet & pkt, uint16 & t_count, uint16 nEventRoom, CUser* pSendUser)

void CGameServerDlg::GetRegionNpcIn(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom, CUser* pSendUser)
{
	if (pMap == nullptr)
		return;

	uint16 sZoneID = pMap->GetID();

	CRegion* pRegion = pMap->GetRegion(region_x, region_z);
	if (pRegion == nullptr)
		return;

	pRegion->m_lockNpcArray.lock();
	if (pRegion->m_RegionNpcArray.size() <= 0)
	{
		pRegion->m_lockNpcArray.unlock();
		return;
	}

	ZoneNpcArray cm_RegionNpcArray = pRegion->m_RegionNpcArray;
	pRegion->m_lockNpcArray.unlock();

	foreach(itr, cm_RegionNpcArray)
	{
		CNpc* pNpc = GetNpcPtr(*itr, pMap->GetID());
		if (pNpc == nullptr
			|| pNpc->isDead())
			continue;

		if (nEventRoom >= 0 && nEventRoom != pNpc->GetEventRoom())
			continue;

		pkt << pNpc->GetID();
		pNpc->GetNpcInfo(pkt, pSendUser->GetClanID());
		t_count++;
	}
}

#pragma endregion

#pragma region CGameServerDlg::RegionNpcInfoForMe(CUser *pSendUser)

void CGameServerDlg::RegionNpcInfoForMe(CUser* pSendUser)
{
	if (pSendUser == nullptr)
		return;

	Packet result(WIZ_NPC_REGION);
	C3DMap* pMap = pSendUser->GetMap();
	if (pMap == nullptr)
		return;
	uint16 npc_count = 0;
	result << uint16(0); // placeholder for NPC count

	int16 rx = pSendUser->GetRegionX(), rz = pSendUser->GetRegionZ();
	foreach_region(x, z)
		GetRegionNpcList(pMap, rx + x, rz + z, result, npc_count, pSendUser->GetEventRoom());


	result.put(0, npc_count);
	pSendUser->SendCompressed(&result);
}

#pragma endregion

#pragma region CGameServerDlg::GetRegionNpcList(C3DMap *pMap, uint16 region_x, uint16 region_z, Packet & pkt, uint16 & t_count, uint16 nEventRoom)

void CGameServerDlg::GetRegionNpcList(C3DMap* pMap, uint16 region_x, uint16 region_z, Packet& pkt, uint16& t_count, uint16 nEventRoom)
{
	if (pMap == nullptr)
		return;

	CRegion* pRegion = pMap->GetRegion(region_x, region_z);
	if (pRegion == nullptr)
		return;

	pRegion->m_lockNpcArray.lock();
	ZoneNpcArray cm_RegionNpcArray = pRegion->m_RegionNpcArray;
	if (cm_RegionNpcArray.empty())
	{
		pRegion->m_lockNpcArray.unlock();
		return;
	}
	pRegion->m_lockNpcArray.unlock();

	foreach(itr, cm_RegionNpcArray)
	{
		CNpc* pNpc = GetNpcPtr(*itr, pMap->GetID());
		if (pNpc == nullptr || pNpc->isDead())
			continue;

		if (nEventRoom > 0
			&& nEventRoom != pNpc->m_bEventRoom)
			continue;

		pkt << pNpc->GetID();
		t_count++;
	}
}


#pragma endregion

#pragma region CGameServerDlg::GetUnitListFromSurroundingRegions(Unit * pOwner, std::vector<uint16> * pList)

void CGameServerDlg::GetUnitListFromSurroundingRegions(Unit* pOwner, std::vector<uint16>* pList)
{
	if (pOwner == nullptr)
		return;

	C3DMap* pMap = pOwner->GetMap();
	if (pMap == nullptr)
		return;

	int16 rx = pOwner->GetRegionX(), rz = pOwner->GetRegionZ();

	foreach_region(x, z)
	{
		uint16 region_x = rx + x, region_z = rz + z;
		CRegion* pRegion = pMap->GetRegion(region_x, region_z);
		if (pRegion == nullptr)
			continue;

		pRegion->m_lockNpcArray.lock();
		if (pRegion->m_RegionNpcArray.size() <= 0)
		{
			pRegion->m_lockNpcArray.unlock();
			continue;
		}

		ZoneNpcArray cm_RegionNpcArray = pRegion->m_RegionNpcArray;
		pRegion->m_lockNpcArray.unlock();
		foreach(itr, cm_RegionNpcArray)
		{
			CNpc* pNpc = GetNpcPtr(*itr, pMap->GetID());
			if (pNpc == nullptr || pNpc->isDead())
				continue;

			if (pOwner->GetEventRoom() >= 0 && pOwner->GetEventRoom() != pNpc->GetEventRoom())
				continue;

			pList->push_back(*itr);
		}
	}

	foreach_region(x, z)
	{
		uint16 region_x = rx + x, region_z = rz + z;
		CRegion* pRegion = pMap->GetRegion(region_x, region_z);
		if (pRegion == nullptr)
			continue;

		// Add all potential users to list
		pRegion->m_lockUserArray.lock();
		if (pRegion->m_RegionUserArray.size() <= 0)
		{
			pRegion->m_lockUserArray.unlock();
			continue;
		}

		ZoneUserArray cm_RegionUserArray = pRegion->m_RegionUserArray;
		pRegion->m_lockUserArray.unlock();
		foreach(itr, cm_RegionUserArray)
		{
			CUser* pUser = GetUserPtr(*itr);
			if (pUser == nullptr
				|| !pUser->isInGame())
				continue;

			if (pOwner->GetEventRoom() >= 0 && pOwner->GetEventRoom() != pUser->GetEventRoom())
				continue;

			pList->push_back(*itr);

		}
	}

	foreach_region(x, z)
	{
		uint16 region_x = rx + x, region_z = rz + z;
		CRegion* pRegion = pMap->GetRegion(region_x, region_z);
		if (pRegion == nullptr)
			continue;

		// Add all potential users to list
		pRegion->m_lockBotArray.lock();
		if (pRegion->m_RegionBotArray.size() <= 0)
		{
			pRegion->m_lockBotArray.unlock();
			continue;
		}

		ZoneUserArray cm_RegionBotArray = pRegion->m_RegionBotArray;
		pRegion->m_lockBotArray.unlock();
		foreach(itr, cm_RegionBotArray)
		{
			CBot* pBot = GetBotPtr(*itr);
			if (pBot == nullptr
				|| !pBot->isInGame()
				|| pBot->isDead())
				continue;

			if (pOwner->GetEventRoom() >= 0 && pOwner->GetEventRoom() != pBot->GetEventRoom())
				continue;

			pList->push_back(*itr);
		}
	}
}

#pragma endregion

#pragma region CGameServerDlg::Send_Zone_Matched_Class(Packet *pkt, uint8 bZoneID, CUser* pExceptUser, uint8 nation, uint8 seekingPartyOptions, uint16 nEventRoom)

/**
* @brief	Sends a packet to all users in the zone matching the specified class types.
*
* @param	pkt				   	The packet.
* @param	bZoneID			   	Identifier for the zone.
* @param	pExceptUser		   	The except user.
* @param	nation			   	The nation.
* @param	seekingPartyOptions	Bitmask of classes to send to.
*/
void CGameServerDlg::Send_Zone_Matched_Class(Packet* pkt, uint8 bZoneID, CUser* pExceptUser, uint8 nation, uint8 seekingPartyOptions, uint16 nEventRoom)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr)
			continue;

		if (pUser == pExceptUser
			|| !pUser->isInGame()
			|| pUser->GetZoneID() != bZoneID
			|| pUser->isInParty()) // looking for users to join the party
			continue;

		if (nEventRoom >= 0 && nEventRoom != pUser->GetEventRoom())
			continue;

		// If we're in the neutral zone (Moradon), it doesn't matter which nation we party with.
		// For all other zones, we must party with a player of the same nation.
		if (pUser->isInMoradon()
			|| pUser->GetNation() == nation)
		{
			if (((seekingPartyOptions & 1) && pUser->JobGroupCheck((short)ClassType::ClassWarrior))
				|| ((seekingPartyOptions & 2) && pUser->JobGroupCheck((short)ClassType::ClassRogue))
				|| ((seekingPartyOptions & 4) && pUser->JobGroupCheck((short)ClassType::ClassMage))
				|| ((seekingPartyOptions & 8) && pUser->JobGroupCheck((short)ClassType::ClassPriest))
				|| ((seekingPartyOptions & 10) && pUser->JobGroupCheck((short)ClassType::ClassPortuKurian)))
				pUser->Send(pkt);
		}
	}
}

#pragma endregion

#pragma region CGameServerDlg::Send_Zone(Packet *pkt, uint8 bZoneID, CUser* pExceptUser /*= nullptr*/, uint8 nation /*= 0*/, uint16 nEventRoom /*= 0*/, float fRange /*= 0.0f*/)

void CGameServerDlg::Send_Zone(Packet* pkt, uint8 bZoneID, CUser* pExceptUser /*= nullptr*/, uint8 nation /*= 0*/, uint16 nEventRoom /*= 0*/, float fRange /*= 0.0f*/)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr)
			continue;

		if (!pUser->isInGame()
			|| pUser->GetZoneID() != bZoneID
			|| (nation != (uint8)Nation::ALL && nation != pUser->GetNation()))
		{
			if (pExceptUser != nullptr)
			{
				if (pUser == pExceptUser
					|| (fRange > 0.0f && pUser->isInRange(pExceptUser, fRange)))
					continue;
			}
			continue;
		}

		if (nEventRoom >= 0 && nEventRoom != pUser->GetEventRoom())
			continue;

		pUser->Send(pkt);
	}
}

#pragma endregion

void CGameServerDlg::Send_Merchant(Packet* pkt, uint8 bZoneID, CUser* pExceptUser /*= nullptr*/, uint8 nation /*= 0*/, uint16 nEventRoom /*= 0*/, float fRange /*= 0.0f*/)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);

		if (pUser == nullptr)
			continue;

		if (!pUser->isInGame() || pUser->GetZoneID() != bZoneID || (nation != (uint8)Nation::ALL && nation != pUser->GetNation()))
		{
			if (pExceptUser != nullptr)
			{
				if (pUser == pExceptUser)
					continue;
			}
			continue;
		}

		pUser->Send(pkt);
	}
}


#pragma region CGameServerDlg::Send_GM

void CGameServerDlg::Send_GM(Packet* pkt)
{
	foreach(itr, m_GMNameMap)
	{
		CUser* pUser = TO_USER(itr->second);
		if (pUser == nullptr
			|| !pUser->isInGame()
			|| !pUser->isGM())
			continue;

		pUser->Send(pkt);
	}
}
#pragma endregion
// end

#pragma region CGameServerDlg::Send_All(Packet *pkt, CUser* pExceptUser /*= nullptr*/, uint8 nation /*= 0*/,uint8 ZoneID /*= 0*/, bool isSendEventUsers /* false */, uint16 nEventRoom /*= -1*/)
void CGameServerDlg::Send_All(Packet* pkt, CUser* pExceptUser /*= nullptr*/, uint8 nation /*= 0*/,
	uint8 ZoneID /*= 0*/, bool isSendEventUsers /* false */, uint16 nEventRoom /*= -1*/)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr
			|| pUser == pExceptUser
			|| !pUser->isInGame())
			continue;

		if ((nation != (uint8)Nation::ALL && nation != pUser->GetNation()))
			continue;

		if (ZoneID != 0 && pUser->GetZoneID() != ZoneID)
			continue;

		if (nEventRoom && nEventRoom != pUser->GetEventRoom())
			continue;

		if (isSendEventUsers && !pUser->isEventUser())
			continue;

		pUser->Send(pkt);
	}
}
#pragma endregion

#pragma region  CGameServerDlg::Send_Region(Packet *pkt, C3DMap *pMap, int x, int z, CUser* pExceptUser, uint16 nEventRoom)

void CGameServerDlg::Send_Region(Packet* pkt, C3DMap* pMap, int x, int z, CUser* pExceptUser, uint16 nEventRoom)
{
	foreach_region(rx, rz)
		Send_UnitRegion(pkt, pMap, rx + x, rz + z, pExceptUser, nEventRoom);
}

#pragma endregion

void CGameServerDlg::SendNoticeWindAll(std::string notice, uint32 color, uint8 ZoneID)
{
	Packet result(WIZ_ADD_MSG, uint8(2));
	result << notice << color << color;

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame() || pUser && ZoneID > 0 && ZoneID != pUser->GetZoneID())
			continue;

		pUser->Send(&result);
	}
}

#pragma region CGameServerDlg::Send_UnitRegion(Packet *pkt, C3DMap *pMap, int x, int z, CUser *pExceptUser, uint16 nEventRoom)

void CGameServerDlg::Send_UnitRegion(Packet* pkt, C3DMap* pMap, int x, int z, CUser* pExceptUser, uint16 nEventRoom)
{
	if (pMap == nullptr || pMap->m_nZoneNumber == 0)
		return;

	CRegion* pRegion = pMap->GetRegion(x, z);

	if (pRegion == nullptr)
		return;

	pRegion->m_lockUserArray.lock();

	if (pRegion->m_RegionUserArray.size() <= 0)
	{
		pRegion->m_lockUserArray.unlock();
		return;
	}

	ZoneUserArray cm_RegionUserArray = pRegion->m_RegionUserArray;
	pRegion->m_lockUserArray.unlock();
	foreach(itr, cm_RegionUserArray)
	{
		if (cm_RegionUserArray.size() <= 0)
		{
			printf("cm_RegionUserArray [10] size = 0\n");
			break;
		}
		CUser* pUser = GetUserPtr(*itr);
		if (pUser == nullptr
			|| pUser == pExceptUser
			|| !pUser->isInGame())
			continue;

		if (nEventRoom >= 0 && nEventRoom != pUser->GetEventRoom())
			continue;

		pUser->Send(pkt);
	}
}

#pragma endregion

#pragma region CGameServerDlg::Send_OldRegions(Packet *pkt, int old_x, int old_z, C3DMap *pMap, int x, int z, CUser* pExceptUser, uint16 nEventRoom)

// TODO: Move the following two methods into a base CUser/CNpc class
void CGameServerDlg::Send_OldRegions(Packet* pkt, int old_x, int old_z, C3DMap* pMap, int x, int z, CUser* pExceptUser, uint16 nEventRoom)
{
	if (old_x != 0)
	{
		Send_UnitRegion(pkt, pMap, x + old_x * 2, z + old_z - 1, pExceptUser, nEventRoom);
		Send_UnitRegion(pkt, pMap, x + old_x * 2, z + old_z, pExceptUser, nEventRoom);
		Send_UnitRegion(pkt, pMap, x + old_x * 2, z + old_z + 1, pExceptUser, nEventRoom);
	}

	if (old_z != 0)
	{
		Send_UnitRegion(pkt, pMap, x + old_x, z + old_z * 2, pExceptUser, nEventRoom);
		if (old_x < 0)
			Send_UnitRegion(pkt, pMap, x + old_x + 1, z + old_z * 2, pExceptUser, nEventRoom);
		else if (old_x > 0)
			Send_UnitRegion(pkt, pMap, x + old_x - 1, z + old_z * 2, pExceptUser, nEventRoom);
		else
		{
			Send_UnitRegion(pkt, pMap, x + old_x - 1, z + old_z * 2, pExceptUser, nEventRoom);
			Send_UnitRegion(pkt, pMap, x + old_x + 1, z + old_z * 2, pExceptUser, nEventRoom);
		}
	}
}

#pragma endregion

#pragma region CGameServerDlg::Send_NewRegions(Packet *pkt, int new_x, int new_z, C3DMap *pMap, int x, int z, CUser* pExceptUser, uint16 nEventRoom)

void CGameServerDlg::Send_NewRegions(Packet* pkt, int new_x, int new_z, C3DMap* pMap, int x, int z, CUser* pExceptUser, uint16 nEventRoom)
{
	if (new_x != 0)
	{
		Send_UnitRegion(pkt, pMap, x + new_x, z - 1, pExceptUser, nEventRoom);
		Send_UnitRegion(pkt, pMap, x + new_x, z, pExceptUser, nEventRoom);
		Send_UnitRegion(pkt, pMap, x + new_x, z + 1, pExceptUser, nEventRoom);
	}

	if (new_z != 0)
	{
		Send_UnitRegion(pkt, pMap, x, z + new_z, pExceptUser, nEventRoom);

		if (new_x < 0)
			Send_UnitRegion(pkt, pMap, x + 1, z + new_z, pExceptUser, nEventRoom);
		else if (new_x > 0)
			Send_UnitRegion(pkt, pMap, x - 1, z + new_z, pExceptUser, nEventRoom);
		else
		{
			Send_UnitRegion(pkt, pMap, x - 1, z + new_z, pExceptUser, nEventRoom);
			Send_UnitRegion(pkt, pMap, x + 1, z + new_z, pExceptUser, nEventRoom);
		}
	}
}

#pragma endregion

#pragma region CGameServerDlg::Send_NearRegion(Packet *pkt, C3DMap *pMap, int region_x, int region_z, float curx, float curz, CUser* pExceptUser, uint16 nEventRoom)

void CGameServerDlg::Send_NearRegion(Packet* pkt, C3DMap* pMap, int region_x, int region_z, float curx, float curz, CUser* pExceptUser, uint16 nEventRoom)
{
	int left_border = region_x * VIEW_DISTANCE, top_border = region_z * VIEW_DISTANCE;
	Send_FilterUnitRegion(pkt, pMap, region_x, region_z, curx, curz, pExceptUser, nEventRoom);
	if (((curx - left_border) > (VIEW_DISTANCE / 2.0f)))
	{			// RIGHT
		if (((curz - top_border) > (VIEW_DISTANCE / 2.0f)))
		{	// BOTTOM
			Send_FilterUnitRegion(pkt, pMap, region_x + 1, region_z, curx, curz, pExceptUser, nEventRoom);
			Send_FilterUnitRegion(pkt, pMap, region_x, region_z + 1, curx, curz, pExceptUser, nEventRoom);
			Send_FilterUnitRegion(pkt, pMap, region_x + 1, region_z + 1, curx, curz, pExceptUser, nEventRoom);
		}
		else
		{													// TOP
			Send_FilterUnitRegion(pkt, pMap, region_x + 1, region_z, curx, curz, pExceptUser, nEventRoom);
			Send_FilterUnitRegion(pkt, pMap, region_x, region_z - 1, curx, curz, pExceptUser, nEventRoom);
			Send_FilterUnitRegion(pkt, pMap, region_x + 1, region_z - 1, curx, curz, pExceptUser, nEventRoom);
		}
	}
	else
	{														// LEFT
		if (((curz - top_border) > (VIEW_DISTANCE / 2.0f)))
		{	// BOTTOM
			Send_FilterUnitRegion(pkt, pMap, region_x - 1, region_z, curx, curz, pExceptUser, nEventRoom);
			Send_FilterUnitRegion(pkt, pMap, region_x, region_z + 1, curx, curz, pExceptUser, nEventRoom);
			Send_FilterUnitRegion(pkt, pMap, region_x - 1, region_z + 1, curx, curz, pExceptUser, nEventRoom);
		}
		else
		{													// TOP
			Send_FilterUnitRegion(pkt, pMap, region_x - 1, region_z, curx, curz, pExceptUser, nEventRoom);
			Send_FilterUnitRegion(pkt, pMap, region_x, region_z - 1, curx, curz, pExceptUser, nEventRoom);
			Send_FilterUnitRegion(pkt, pMap, region_x - 1, region_z - 1, curx, curz, pExceptUser, nEventRoom);
		}
	}
}

#pragma endregion

#pragma region CGameServerDlg::Send_FilterUnitRegion(Packet *pkt, C3DMap *pMap, int x, int z, float ref_x, float ref_z, CUser *pExceptUser, uint16 nEventRoom)

void CGameServerDlg::Send_FilterUnitRegion(Packet* pkt, C3DMap* pMap, int x, int z, float ref_x, float ref_z, CUser* pExceptUser, uint16 nEventRoom)
{
	if (pMap == nullptr || pMap->m_nZoneNumber == 0)
		return;

	CRegion* pRegion = pMap->GetRegion(x, z);
	if (pRegion == nullptr)
		return;

	pRegion->m_lockUserArray.lock();

	if (pRegion->m_RegionUserArray.size() <= 0)
	{
		pRegion->m_lockUserArray.unlock();
		return;
	}

	ZoneUserArray cm_RegionUserArray = pRegion->m_RegionUserArray;
	pRegion->m_lockUserArray.unlock();
	foreach(itr, cm_RegionUserArray)
	{
		CUser* pUser = GetUserPtr(*itr);
		if (pUser == nullptr
			|| pUser == pExceptUser
			|| !pUser->isInGame())
			continue;

		if (nEventRoom >= 0 && nEventRoom != pUser->GetEventRoom())
			continue;

		if (sqrt(pow((pUser->m_curx - ref_x), 2) + pow((pUser->m_curz - ref_z), 2)) < 32)
			pUser->Send(pkt);
	}
}

#pragma endregion

#pragma region CGameServerDlg::Send_PartyMember(int party, Packet *result)

void CGameServerDlg::Send_PartyMember(int party, Packet* result)
{
	_PARTY_GROUP* pParty = GetPartyPtr(party);
	if (pParty == nullptr)
		return;

	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		CUser* pUser = GetUserPtr(pParty->uid[i]);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		pUser->Send(result);
	}
}

#pragma endregion

#pragma region CGameServerDlg::Send_KnightsMember(int index, Packet *pkt)

void CGameServerDlg::Send_KnightsMember(int index, Packet* pkt)
{
	CKnights* pKnights = GetClanPtr(index);
	if (pKnights == nullptr)
		return;

	pKnights->Send(pkt);
}

#pragma endregion

#pragma region CGameServerDlg::Send_KnightsAlliance(uint16 sAllianceID, Packet *pkt)

void CGameServerDlg::Send_KnightsAlliance(uint16 sAllianceID, Packet* pkt)
{
	_KNIGHTS_ALLIANCE* pAlliance = GetAlliancePtr(sAllianceID);
	if (pAlliance == nullptr)
		return;

	Send_KnightsMember(pAlliance->sMainAllianceKnights, pkt);
	Send_KnightsMember(pAlliance->sSubAllianceKnights, pkt);
	Send_KnightsMember(pAlliance->sMercenaryClan_1, pkt);
	Send_KnightsMember(pAlliance->sMercenaryClan_2, pkt);
}
#pragma endregion

#pragma region CGameServerDlg::Send_Noah_Knights(Packet *pkt)
/**
* @brief Sends Noah Chat Packets to all Noah Knights.
*/
void CGameServerDlg::Send_Noah_Knights(Packet* pkt)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr
			|| !pUser->isInGame()
			|| pUser->GetLevel() > 50)
			continue;

		pUser->Send(pkt);
	}
}
#pragma endregion