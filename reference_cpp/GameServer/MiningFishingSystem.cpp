#include "stdafx.h"

/**
* @brief	Handles packets related to the mining system.
* 			Also handles soccer-related packets (yuck).
*
* @param	pkt	The packet.
*/
void CUser::HandleMiningSystem(Packet & pkt)
{
	uint8 opcode;
	pkt >> opcode;

	switch (opcode)
	{
	case MiningStart:
		HandleMiningStart(pkt);
		break;

	case MiningAttempt:
		HandleMiningAttempt(pkt);
		break;

	case MiningStop:
		HandleMiningStop();
		break;

	case BettingGame:
		HandleBettingGame(pkt);
		break;

	case FishingStart:
		HandleFishingStart(pkt);
		break;

	case FishingAttempt:
		HandleFishingAttempt(pkt);
		break;

	case FishingStop:
		HandleFishingStop(pkt);
		break;

	case MiningSoccer:
		HandleSoccer(pkt);
		break;

	default:
		TRACE("[SID=%d] Unknown packet %X\n", GetSocketID(), opcode);
		printf("[SID=%d] Unknown packet %X\n", GetSocketID(), opcode);
		return;
	}
}

/**
* @brief	Handles users Betting Game for 10 k coins.
* @param	pkt	The packet.
*/
void CUser::HandleBettingGame(Packet & pkt)
{
	if (isDead() || !isInGame())
		return;

	enum class ResultMessages
	{
		Won = 1, // %s won 10000
		EndinginaTie = 2, // Received 5000 Noah for ending in a tie
		Lose = 3, // %s lost Noah
		EnougCoins = 4, // %s You don't Enough coins.
	};

	Packet result(WIZ_MINING, uint8(BettingGame));
	ResultMessages resultsopcode = ResultMessages::EnougCoins;

	if (!hasCoins(5000))
	{
		resultsopcode = ResultMessages::EnougCoins;
		goto fail_return;
	}

	GoldLose(5000);

	uint8 PlayerRand = 0;  uint8 NpcRand = 0;
	PlayerRand = myrand(1, 5); NpcRand = myrand(1, 5);

	if (PlayerRand > NpcRand)
		resultsopcode = ResultMessages::Won;
	else if (PlayerRand < NpcRand)
		resultsopcode = ResultMessages::Lose;
	else if (PlayerRand == NpcRand)
		resultsopcode = ResultMessages::EndinginaTie;

	if (resultsopcode == ResultMessages::Won)
		GoldGain(10000);

fail_return:
	result << uint16(resultsopcode) << uint16(0) << uint8(0) << uint8(0) << uint8(PlayerRand) << uint8(NpcRand) << uint16(0);
	SendToRegion(&result);
}

bool CUser::MiningFishingAreaCheck(uint8 Type) {
	switch (Type)
	{
		//mining
	case 1:
	{
		C3DMap* pMap = GetMap();
		if (pMap == nullptr || !pMap->m_bMiningZone)
			return false;
		
		
		if (isInMoradon())
		{
			if (GetX() >= 600 && GetX() <= 666 && GetZ() >= 348 && GetZ() <= 399)
				return true;
		}
		else if (isInElmoradCastle())
		{
			if (GetX() >= 1408 && GetX() <= 1488 && GetZ() >= 354 && GetZ() <= 440)
				return true;

			if (GetX() >= 1653 && GetX() <= 1733 && GetZ() >= 526 && GetZ() <= 625)
				return true;
		}
		else if (isInLufersonCastle())
		{
			if (GetX() >= 597 && GetX() <= 720 && GetZ() >= 1625 && GetZ() <= 1705)
				return true;

			if (GetX() >= 315 && GetX() <= 415 && GetZ() >= 1435 && GetZ() <= 1500)
				return true;
		}
	}
	break;
	//fishing
	case 2:
	{
		if (isInElmoradCastle())
		{
			if (GetX() >= 850 && GetX() <= 935 && GetZ() >= 1080 && GetZ() <= 1115)
				return true;
		}
		else if (isInLufersonCastle())
		{
			if (GetX() >= 1106 && GetX() <= 1209 && GetZ() >= 915 && GetZ() <= 944)
				return true;
		}
	}
	break;
	}
	return false;
}

/**
* @brief	Handles users requesting to start mining.
* 			NOTE: This is a mock-up, so be warned that it does not
* 			handle checks such as identifying if the user is allowed
* 			to mine in this area.
*
* @param	pkt	The packet.
*/

void CUser::HandleMiningStart(Packet & pkt)
{
	Packet result(WIZ_MINING, uint8(MiningStart));
	uint16 resultCode = MiningResultSuccess;

	DateTime time;

	// Are we mining already?
	if (isMining())
		resultCode = MiningResultMiningAlready;

	if (!MiningFishingAreaCheck(1))
		resultCode = MiningResultNotMiningArea;

	// Do we have a pickaxe? Is it worn?
	_ITEM_DATA * pItem;
	_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
	if (pItem == nullptr || pTable.isnull()
		|| pItem->sDuration <= 0
		|| !pTable.isPickaxe())
		resultCode = MiningResultNotPickaxe;

	result << resultCode;

	// If nothing went wrong, allow the user to start mining.
	// Be sure to let everyone know we're mining.
	if (resultCode == MiningResultSuccess)
	{
		m_bResHpType = USER_MINING;
		m_bMining = true;
		result << GetID();
		SendToRegion(&result);
	}
	else
	{
		Send(&result);
	}

	Packet JstKO; // JstKO Public Chat Renk Sari Notice Eklendi 22.02.2025
	std::string PubliChat_Notice = string_format("Nick : %s | Mining Start : %d Ba�ladi. - Saat : %d:%d:%d", GetName().c_str(), m_bMining, time.GetHour(), time.GetMinute(), time.GetSecond());
	JstKO.clear();
	ChatPacket::Construct(&JstKO, ChatType::KNIGHTS_CHAT, PubliChat_Notice.c_str(), "[Mining Start]", GetNation());
	Send(&JstKO);
}


std::vector<_MINING_FISHING_ITEM> CUser::MiningItemList(_ITEM_TABLE pTable) {

	std::vector<_MINING_FISHING_ITEM> pMiningList;
	if (pTable.isnull()) 
		return pMiningList;

	foreach_stlmap(itr, g_pMain->m_MiningFishingItemArray) {
		auto& pMining = (*itr->second);
		if (pMining.isnull() || pMining.nTableType != 0) 
			continue;

		if (pTable.m_iNum == GOLDEN_MATTOCK && pMining.UseItemType == 1) {
			if (g_pMain->m_byBattleOpen == NATION_BATTLE) {
				if (GetZoneID() == ZONE_KARUS) {
					if (GetWarVictory() == (uint8)Nation::ELMORAD && GetNation() == (uint8)Nation::ELMORAD && pMining.nWarStatus == 2) pMiningList.push_back(pMining);
					else if (GetWarVictory() == (uint8)Nation::ELMORAD && GetNation() == (uint8)Nation::KARUS && pMining.nWarStatus == 1) pMiningList.push_back(pMining);
					else if (pMining.nWarStatus == 0)pMiningList.push_back(pMining);
				}
				else if (GetZoneID() == ZONE_ELMORAD) {
					if (GetWarVictory() == (uint8)Nation::KARUS && GetNation() == (uint8)Nation::KARUS && pMining.nWarStatus == 2) pMiningList.push_back(pMining);
					else if (GetWarVictory() == (uint8)Nation::KARUS && GetNation() == (uint8)Nation::ELMORAD && pMining.nWarStatus == 1) pMiningList.push_back(pMining);
					else if (pMining.nWarStatus == 0) pMiningList.push_back(pMining);
				}
				else if (pMining.nWarStatus == 0) pMiningList.push_back(pMining);
			}
			else if (pMining.nWarStatus == 0) pMiningList.push_back(pMining);
		}
		else if (pTable.m_iNum == MATTOCK && pMining.UseItemType == 0) {
			if (g_pMain->m_byBattleOpen == NATION_BATTLE) {
				if (GetZoneID() == ZONE_KARUS) {
					if (GetWarVictory() == (uint8)Nation::ELMORAD && GetNation() == (uint8)Nation::ELMORAD && pMining.nWarStatus == 2) pMiningList.push_back(pMining);
					else if (GetWarVictory() == (uint8)Nation::ELMORAD && GetNation() == (uint8)Nation::KARUS && pMining.nWarStatus == 1) pMiningList.push_back(pMining);
					else if (pMining.nWarStatus == 0) pMiningList.push_back(pMining);
				}
				else if (GetZoneID() == ZONE_ELMORAD) {
					if (GetWarVictory() == (uint8)Nation::KARUS && GetNation() == (uint8)Nation::KARUS && pMining.nWarStatus) pMiningList.push_back(pMining);
					else if (GetWarVictory() == (uint8)Nation::KARUS && GetNation() == (uint8)Nation::ELMORAD && pMining.nWarStatus == 1) pMiningList.push_back(pMining);
					else if (pMining.nWarStatus == 0) pMiningList.push_back(pMining);
				}
				else if (pMining.nWarStatus == 0) pMiningList.push_back(pMining);
			}
			else if (pMining.nWarStatus == 0) pMiningList.push_back(pMining);
		}
	}
	return pMiningList;
}

uint32 CUser::GetMiningExpCount() {
	if (GetLevel() >= 1 && GetLevel() <= 34) return 1000;
	else if (GetLevel() >= 35 && GetLevel() <= 59) return 5000;
	else if (GetLevel() >= 60 && GetLevel() <= 69) return 15000;
	else if (GetLevel() >= 70 && GetLevel() <= 83) return 50000;
	return 0;
}

void CUser::HandleMiningAttempt(Packet & pkt)
{
	if (!isMining() || isFishing() || isTrading() || isMerchanting() || isSellingMerchantingPreparing())
		return;
	
	if (m_tLastMiningAttempt > UNIXTIME2)
		return;

	_ITEM_TABLE pGiveItem = _ITEM_TABLE();
	std::vector<_MINING_FISHING_ITEM> pMiningList;

	m_tLastMiningAttempt = UNIXTIME2 + MINING_DELAY;
	MiningErrors bResult = MiningErrors::MiningResultError;

	Packet result(WIZ_MINING, uint8(MiningAttempt));

	_ITEM_DATA* pItem;
	auto pTable = GetItemPrototype(RIGHTHAND, pItem);
	if (pItem == nullptr || pTable.isnull()
		|| (pTable.m_iNum != GOLDEN_MATTOCK && pTable.m_iNum != MATTOCK)
		|| pItem->sDuration <= 0 || pItem->isDuplicate()
		|| pItem->isRented() || pItem->isSealed() || !pTable.isPickaxe()) {
		bResult = MiningErrors::MiningResultNotPickaxe;
		goto fail_return;
	}

	if (!isInMoradon()) {
		if (g_pMain->m_byBattleOpen == NATION_BATTLE && GetZoneID() != ZONE_KARUS && GetZoneID() != ZONE_ELMORAD) 
			goto fail_return;
		
		if (g_pMain->m_byBattleOpen != NATION_BATTLE && (GetNation() == (uint8)Nation::ELMORAD && !isInElmoradCastle()) || (GetNation() == (uint8)Nation::KARUS && !isInLufersonCastle())) 
			goto fail_return;
	}

	pMiningList = MiningItemList(pTable);
	if (pMiningList.empty()) 
		goto fail_return;

	uint32 bRandArray[10000], offset = 0; uint8 sItemSlot = 0;
	memset(bRandArray, 0, sizeof(bRandArray));
	foreach(itr, pMiningList) {
		if (itr->isnull()) 
			continue;
		if (offset >= 9999)
			break;
		
		for (int i = 0; i < int(itr->SuccessRate); i++) {
			if (i + offset >= 9999) 
				break;
			
			bRandArray[offset + i] = itr->nGiveItemID;
		}
		offset += int(itr->SuccessRate);
	}

	if (offset > 9999) offset = 9999;
	uint32 bRandSlot = myrand(0, offset);
	uint32 nItemID = bRandArray[bRandSlot];

	pGiveItem = g_pMain->GetItemPtr(nItemID);
	if (pGiveItem.isnull() || m_sItemWeight + pGiveItem.m_sWeight > m_sMaxWeight)
		return;

	int SlotForItem = FindSlotForItem(pGiveItem.m_iNum, 1);
	if (SlotForItem < 0)
		goto fail_return;

	uint16 sEffect = 0;
	if (nItemID == ITEM_EXP) // JstKO Maden, Golden ve Mattock Effect ve Exp Notice Eklendi. 22.02.2025
	{
		uint32 expcount = GetMiningExpCount();
		if (expcount) ExpChange("Mining Exp",expcount);
		sEffect = 13082;  // EXP EFFECT
		ShowEffect(490093); // Kar Mavi Paltama Effect Eklendi. 22.02.2025

		Packet JstKO; // JstKO Public Chat Renk Sari Notice Eklendi 22.02.2025
		std::string PubliChat_Notice = string_format("Nick : %s | Mining Exp : %d Aldin Tebrikler", GetName().c_str(), expcount);
		JstKO.clear();
		ChatPacket::Construct(&JstKO, ChatType::PUBLIC_CHAT, PubliChat_Notice.c_str(), "[Mining Exp Bilgi]", GetNation());
		Send(&JstKO);
	}
	else if (nItemID > 0) // JstKO Maden, Golden ve Mattock Effect ve �tem Notice Eklendi. 22.02.2025
	{
		_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);

		if (pTable.isnull())
			return;

		sItemSlot = GetEmptySlot() - SLOT_MAX;
		GiveItem("Mining Item", nItemID, 1);
		sEffect = 13081; // �TEM EFFECT
		ShowEffect(490092); // Y�lba�i Ya�iyor Effect Eklendi. 22.02.2025

		Packet JstKO; // JstKO Public Chat Renk K�rm�z� Notice Eklendi 22.02.2025
		std::string PubliChat_Notice = string_format("Nick : %s | Mining �tem Name : %s Aldin Tebrikler", GetName().c_str(), pTable.m_sName.c_str());
		JstKO.clear();
		ChatPacket::Construct(&JstKO, ChatType::SHOUT_CHAT, PubliChat_Notice.c_str(), "[Mining �tem Bilgi]", GetNation());
		Send(&JstKO);
	}

	ItemWoreOut(ATTACK, 150);
	result << uint16(MiningErrors::MiningResultSuccess) << GetSocketID() << sEffect;
	SendToRegion(&result, nullptr, GetEventRoom());

	return;
fail_return:
	result << uint16(MiningErrors::MiningResultError);
	Send(&result);
	HandleMiningStop();
}

/**
* @brief	Handles when a user stops mining.
*
* @param	pkt	The packet.
*/
void CUser::HandleMiningStop()
{
	if (!isMining())
		return;

	DateTime time;

	Packet result(WIZ_MINING, uint8(MiningStop));
	result << uint16(1) << GetID();
	m_bMining = false;
	m_bResHpType = USER_STANDING;
	SendToRegion(&result);
	result.clear();
	result.SetOpcode(WIZ_MINING);
	result << uint8(MiningStop) << uint16(2);
	Send(&result);

	Packet JstKO; // JstKO Public Chat Renk Sari Notice Eklendi 22.02.2025
	std::string PubliChat_Notice = string_format("Nick : %s | Mining Stop : %d Durdur. - Saat : %d:%d:%d", GetName().c_str(), m_bMining, time.GetHour(), time.GetMinute(), time.GetSecond());
	JstKO.clear();
	ChatPacket::Construct(&JstKO, ChatType::SHOUT_CHAT, PubliChat_Notice.c_str(), "[Mining Stop]", GetNation());
	Send(&JstKO);
}

/// Fish Stard
void CUser::HandleFishingStart(Packet & pkt)
{
	if (isBlinking()
		|| isTrading()
		|| isMerchanting()
		|| isDead()
		|| !isInGame()
		|| isBuyingMerchantingPreparing()
		|| isSellingMerchantingPreparing())
		return;

	DateTime time;

	Packet result(WIZ_MINING, uint8(FishingStart));
	uint16 resultCode = MiningResultSuccess;

	// Are we mining already?
	if (isFishing())
		resultCode = MiningResultMiningAlready;

	if (!MiningFishingAreaCheck(2)) {
		resultCode = MiningResultNotMiningArea;
	}

	// Do we have a pickaxe? Is it worn?
	_ITEM_DATA * pItem;
	_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
	if (pItem == nullptr || pTable.isnull()
		|| pItem->sDuration <= 0
		|| !pTable.isFishing())
		resultCode = MiningResultNotPickaxe;
	
	auto item = GetItem(RIGHTHAND);
	if (!item) return;

	if (item->nNum != GOLDEN_FISHING) //[] 06.05.2020 Golden Fishing Rod Disindakilerde RainWorm kontrolu yapacak.
	{
		if (GetItemCount(RAINWORM) <= 0)
			resultCode = MiningResultNoEarthWorm;
	}

	result << resultCode;

	// If nothing went wrong, allow the user to start mining.
	// Be sure to let everyone know we're mining.
	if (resultCode == MiningResultSuccess)
	{
		m_bResHpType = USER_FLASHING;
		m_bFishing = true;
		result << GetID();
		SendToRegion(&result);
	}
	else
	{
		Send(&result);
	}

	Packet JstKO; // JstKO Public Chat Renk Sari Notice Eklendi 22.02.2025
	std::string PubliChat_Notice = string_format("Nick : %s | Fishing Start : %d Ba�ladi. - Saat : %d:%d:%d", GetName().c_str(), m_bMining, time.GetHour(), time.GetMinute(), time.GetSecond());
	JstKO.clear();
	ChatPacket::Construct(&JstKO, ChatType::KNIGHTS_CHAT, PubliChat_Notice.c_str(), "[Fishing Start]", GetNation());
	Send(&JstKO);
}

/**
* @brief	Handles a user's mining attempt by finding a random reward (or none at all).
* 			This is sent automatically by the client every MINING_DELAY (5) seconds.
*
* @param	pkt	The packet.
*/
void CUser::HandleFishingAttempt(Packet & pkt)
{
	if (!isFishing())
		return;

	Packet result(WIZ_MINING, uint8(FishingAttempt));

	uint16 resultCode = MiningResultSuccess;

	// Do we have a pickaxe? Is it worn?
	_ITEM_DATA * pItem;
	_MINING_FISHING_ITEM * pFishing = nullptr;
	std::vector<uint32> pFishingList;
	_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
	if (pItem == nullptr || pTable.isnull()
		|| pItem->sDuration <= 0 // are we supposed to wear the pickaxe on use? Need to verify.
		|| !pTable.isFishing())
		resultCode = MiningResultNotPickaxe;


	auto item = GetItem(RIGHTHAND);
	if (!item)
		return;

	if (item->nNum != GOLDEN_FISHING)
	{
		if (GetItemCount(RAINWORM) <= 0)
			resultCode = MiningResultNoEarthWorm;
	}

	// Check to make sure we're not spamming the packet...
	if ((UNIXTIME2 - m_tLastMiningAttempt) < MINING_DELAY) //16.09.2020 <- Seri fishing yapma fix.
	{
		resultCode = MiningResultMiningAlready; // as close an error as we're going to get...
		Packet newpkt(XSafe, uint8(XSafeOpCodes::ERRORMSG));
		newpkt << uint8(1) << uint8(1) << std::string("Fishing") << std::string("Cok Hizli Mining Yapiyorsunuz!");
		Send(&newpkt);
	}

	uint16 sEffect = 0; // Effect to show to clients

	// This is just a mock-up based on another codebase's implementation.
	// Need to log official data to get a proper idea of how it behaves, rate-wise,
	// so that we can then implement it more dynamically.
	if (resultCode == MiningResultSuccess && g_pMain->m_MiningFishingItemArray.GetSize() > 0)
	{
		foreach_stlmap(itr, g_pMain->m_MiningFishingItemArray)
		{
			pFishing = itr->second;

			if (pFishing == nullptr)
				continue;

			if (pFishing->nTableType != 1)
				continue;

			if (pTable.m_iNum == GOLDEN_FISHING)
			{
				if (pFishing->UseItemType == 3)
				{
					if (!g_pMain->isWarOpen())
					{
						if (pFishing->nWarStatus == MiningResultError)
							pFishingList.push_back(pFishing->nIndex);
						else
							continue;
					}
					else
					{
						if (GetZoneID() == ZONE_KARUS)
						{
							if (GetWarVictory() == (uint8)Nation::ELMORAD && GetNation() == (uint8)Nation::ELMORAD)
							{
								if (pFishing->nWarStatus == 2)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
							else if (GetWarVictory() == (uint8)Nation::ELMORAD && GetNation() == (uint8)Nation::KARUS)
							{
								if (pFishing->nWarStatus == 1)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
							else
							{
								if (pFishing->nWarStatus == 0)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
						}
						else if (GetZoneID() == ZONE_ELMORAD)
						{
							if (GetWarVictory() == (uint8)Nation::KARUS && GetNation() == (uint8)Nation::KARUS)
							{
								if (pFishing->nWarStatus == 2)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
							else if (GetWarVictory() == (uint8)Nation::KARUS && GetNation() == (uint8)Nation::ELMORAD)
							{
								if (pFishing->nWarStatus == 1)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
							else
							{
								if (pFishing->nWarStatus == 0)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
						}
						else if (GetZoneID() >= ZONE_MORADON && GetZoneID() <= ZONE_MORADON5)
						{
							if (pFishing->nWarStatus == 0)
								pFishingList.push_back(pFishing->nIndex);
							else
								continue;
						}
					}
				}
			}
			else
			{
				if (pFishing->UseItemType == 2)
				{
					if (!g_pMain->isWarOpen())
					{
						if (pFishing->nWarStatus == MiningResultError)
							pFishingList.push_back(pFishing->nIndex);
						else
							continue;
					}
					else
					{
						if (GetZoneID() == ZONE_KARUS)
						{
							if (GetWarVictory() == (uint8)Nation::ELMORAD && GetNation() == (uint8)Nation::ELMORAD)
							{
								if (pFishing->nWarStatus == 2)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
							else if (GetWarVictory() == (uint8)Nation::ELMORAD && GetNation() == (uint8)Nation::KARUS)
							{
								if (pFishing->nWarStatus == 1)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
							else
							{
								if (pFishing->nWarStatus == 0)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
						}
						else if (GetZoneID() == ZONE_ELMORAD)
						{
							if (GetWarVictory() == (uint8)Nation::KARUS && GetNation() == (uint8)Nation::KARUS)
							{
								if (pFishing->nWarStatus == 2)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
							else if (GetWarVictory() == (uint8)Nation::KARUS && GetNation() == (uint8)Nation::ELMORAD)
							{
								if (pFishing->nWarStatus == 1)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
							else
							{
								if (pFishing->nWarStatus == 0)
									pFishingList.push_back(pFishing->nIndex);
								else
									continue;
							}
						}
						else if (GetZoneID() >= ZONE_MORADON && GetZoneID() <= ZONE_MORADON5)
						{
							if (pFishing->nWarStatus == 0)
								pFishingList.push_back(pFishing->nIndex);
							else
								continue;
						}
					}
				}
			}
		}

		if (pFishingList.size() > 0)
		{
			int offset = 0;
			uint32 bRandArray[10000];
			uint8 sItemSlot = 0;
			foreach(itr, pFishingList)
			{
				pFishing = g_pMain->m_MiningFishingItemArray.GetData(*itr);

				if (pFishing == nullptr)
					return;

				if (offset >= 10000)
					break;

				for (int i = 0; i < int(pFishing->SuccessRate / 5); i++)
				{
					if (i + offset >= 10000)
						break;

					bRandArray[offset + i] = pFishing->nGiveItemID;
				}

				offset += int(pFishing->SuccessRate / 5);
			}

			uint32 bRandSlot = myrand(0, offset);
			uint32 nItemID = bRandArray[bRandSlot];
			_ITEM_TABLE pItem = g_pMain->GetItemPtr(nItemID);

			if (pItem.isnull())
				return;

			uint8 bFreeSlots = 0;
			for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
			{
				auto item = GetItem(i);
				if (!item || item->nNum == 0 && ++bFreeSlots >= ITEMS_IN_EXCHANGE_GROUP)
					break;
			}

			if (bFreeSlots < 1)
				return;

			int SlotForItem = FindSlotForItem(pItem.m_iNum, 1);
			if (SlotForItem == -1)
				return;

			if (nItemID == ITEM_EXP)
			{
				if (GetLevel() >= 1 && GetLevel() <= 34)
					ExpChange("Mining Exp", 50);
				if (GetLevel() >= 35 && GetLevel() <= 59)
					ExpChange("Mining Exp", 100);
				if (GetLevel() >= 60 && GetLevel() <= 69)
					ExpChange("Mining Exp", 200);
				if (GetLevel() >= 70 && GetLevel() <= 83)
					ExpChange("Mining Exp", 300);
				sEffect = 13082; // "XP" effect
			}
			else if (nItemID > 0)
			{
				_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);

				if (pTable.isnull())
					return;

				sItemSlot = GetEmptySlot() - SLOT_MAX;
				GiveItem("Fishing �tem", nItemID, 1);
				sEffect = 30730; // "Item" effect

				Packet JstKO; // JstKO Public Chat Renk Sari Notice Eklendi 06.02.2025
				std::string PubliChat_Notice = string_format("Nick : %s | Fishing �tem Name : %s Aldin Tebrikler", GetName().c_str(), pTable.m_sName.c_str());
				JstKO.clear();
				ChatPacket::Construct(&JstKO, ChatType::PUBLIC_CHAT, PubliChat_Notice.c_str(), "[Fishing �tem Bilgi]", GetNation());
				Send(&JstKO);
			}
		}
		m_tLastMiningAttempt = UNIXTIME2; //16.09.2020 <- Seri mining&fishing yapma fix.
	}

	result << resultCode << GetID() << sEffect;

	if (GetItem(RIGHTHAND)->nNum != GOLDEN_FISHING)
		RobItem(RAINWORM, 1);

	ItemWoreOut(ATTACK, 100);

	if (resultCode != MiningResultSuccess
		&& resultCode != MiningResultNothingFound)
	{
		// Tell us the error first
		Send(&result);

		// and then tell the client to stop mining
		HandleFishingStop(pkt);
		return;
	}

	if (resultCode != MiningResultNothingFound)
		SendToRegion(&result);
	else if (resultCode == MiningResultNothingFound)
		Send(&result);
}

/**
* @brief	Handles when a user stops mining.
*
* @param	pkt	The packet.
*/
void CUser::HandleFishingStop(Packet & pkt)
{
	if (!isFishing())
		return;

	DateTime time;

	Packet result(WIZ_MINING, uint8(FishingStop));
	result << uint16(1) << GetID();
	m_bResHpType = USER_STANDING;
	m_bFishing = false;
	SendToRegion(&result);
	result.clear();
	result.SetOpcode(WIZ_MINING);
	result << uint8(FishingStop) << uint8(2);
	Send(&result);

	Packet JstKO; // JstKO Public Chat Renk Sari Notice Eklendi 22.02.2025
	std::string PubliChat_Notice = string_format("Nick : %s | Fishing Stop : %d Durdur. - Saat : %d:%d:%d", GetName().c_str(), m_bMining, time.GetHour(), time.GetMinute(), time.GetSecond());
	JstKO.clear();
	ChatPacket::Construct(&JstKO, ChatType::SHOUT_CHAT, PubliChat_Notice.c_str(), "[Fishing Stop]", GetNation());
	Send(&JstKO);
}
