#include "stdafx.h"


#pragma region CUser::ShozinSendFail(CraftingErrorCode erroid)
void CUser::ShozinSendFail(CraftingErrorCode erroid)
{
	Packet newpkt(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::SPECIAL_PART_SEWING));
	newpkt << (uint8)erroid;
	Send(&newpkt);
}
#pragma endregion

#pragma region CUser::ShozinExchange(Packet & pkt)
void CUser::ShozinExchange(Packet& pkt)
{
	uint16 sNpcID; uint32 nShadowPiece; uint8 nShadowPieceSlot, nMaterialCount, nDownFlag;
	pkt >> sNpcID >> nShadowPiece >> nShadowPieceSlot >> nMaterialCount;

	std::vector<SPECIAL_PART_SEWING_EXCHANGE> m_exchangelist;
	std::vector<uint32> ExchangingItems;
	std::vector<uint8> m_Slot;
	uint32 nItemID[ITEMS_SPECIAL_EXCHANGE_GROUP]{};
	uint16 nItemCount[ITEMS_SPECIAL_EXCHANGE_GROUP]{};
	int8 nItemSlot[ITEMS_SPECIAL_EXCHANGE_GROUP]{};

	memset(nItemID, 0, sizeof(nItemID));
	memset(nItemCount, 0, sizeof(nItemCount));
	memset(nItemSlot, -1, sizeof(nItemSlot));

	_ITEM_DATA* pShadowItem = nullptr;
	bool isShadowPiece = false;

	Packet newpkt(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::SPECIAL_PART_SEWING));
	if (nMaterialCount > 10 || m_CraftingTime > UNIXTIME2)
		return ShozinSendFail();

	m_CraftingTime = UNIXTIME2 + 850;

	if (!isInGame() || isDead()
		|| isSellingMerchantingPreparing()
		|| isTrading() || isMerchanting()
		|| isMining() || isFishing() || !isInMoradon()
		|| nShadowPieceSlot >= HAVE_MAX)
		return ShozinSendFail();

	auto* pNpc = g_pMain->GetNpcPtr(sNpcID, GetZoneID());
	if (pNpc == nullptr || (pNpc->GetType() != NPC_CRAFTSMAN && pNpc->GetType() != NPC_JEWELY))
		return ShozinSendFail();

	if (nShadowPiece == ITEM_SHADOW_PIECE) {
		pShadowItem = GetItem(SLOT_MAX + nShadowPieceSlot);
		if (pShadowItem == nullptr || pShadowItem->nNum != ITEM_SHADOW_PIECE || pShadowItem->sCount < 1)
			return ShozinSendFail();
		if (pShadowItem->isBound() || pShadowItem->isDuplicate() || pShadowItem->isRented() || pShadowItem->isSealed())
			return ShozinSendFail();
		if (pNpc->GetType() != NPC_CRAFTSMAN)
			return ShozinSendFail();

		isShadowPiece = true;
	}

	if (isShadowPiece && !pShadowItem)
		return ShozinSendFail();

	for (int i = 0; i < nMaterialCount; i++) {
		pkt >> nItemSlot[i];
		if (nItemSlot[i] < 0 || nItemSlot[i] >= HAVE_MAX)
			return ShozinSendFail();
		m_Slot.push_back(nItemSlot[i]);
	}

	for (int i = 0; i < nMaterialCount; i++) {
		if (nItemSlot[i] != -1) {
			uint16 Counter = 0;
			for (auto test = m_Slot.begin(); test != m_Slot.end(); ++test) if (nItemSlot[i] == *test)Counter++;

			if (Counter > 1)
				return ShozinSendFail();
		}
	}

	for (int i = 0; i < nMaterialCount; i++) {
		for (int j = 0; j < nMaterialCount; j++)
			if (nItemSlot[i] == nItemSlot[j] && i != j)
				return ShozinSendFail();
	}

	pkt >> nDownFlag;
	for (int i = 0; i < nMaterialCount; i++) {
		auto* pItem = GetItem(SLOT_MAX + nItemSlot[i]);
		if (pItem == nullptr) continue;
		if (pItem->isDuplicate() || pItem->isRented() || pItem->isExpirationTime() || !pItem->sCount)
			return ShozinSendFail();

		int nDigit = 100000000;
		int ItemNumLenght = 9;
		if (pItem->nNum > 999999999) {
			nDigit = 1000000000;
			ItemNumLenght = 10;
		}
		else {
			nDigit = 100000000;
			ItemNumLenght = 9;
		}

		uint8 nReadByte;
		nItemID[i] = 0;

		for (int x = 0; x < ItemNumLenght; x++) {
			pkt >> nReadByte;
			uint8 decimal = nReadByte - 48;
			if (decimal > 9) return ShozinSendFail();
			nItemID[i] += decimal * nDigit;
			nDigit = nDigit / 10;
		}

		uint8 nCount[3] = { 0, 0, 0 };
		pkt >> nCount[0] >> nCount[1] >> nCount[2];
		uint8 decimal1 = nCount[0] - 48;
		uint8 decimal2 = nCount[1] - 48;
		uint8 decimal3 = nCount[2] - 48;
		if (decimal1 > 9 || decimal2 > 9 || decimal3 > 9)
			return ShozinSendFail();
		uint16 nCountFinish = 0;
		nCountFinish += decimal1 * 100;
		nCountFinish += decimal2 * 10;
		nCountFinish += decimal3 * 1;
		nItemCount[i] = nCountFinish;
		ExchangingItems.push_back(nItemID[i]);
	}

	for (int i = 0; i < nMaterialCount; i++) {
		auto* pItem = GetItem(SLOT_MAX + nItemSlot[i]);
		auto pTable = g_pMain->GetItemPtr(nItemID[i]);
		if (pItem == nullptr || pTable.isnull() /*|| pTable.m_bRace == 20*/
			|| pItem->nNum != nItemID[i] || pItem->sCount < nItemCount[i] || !pItem->sCount)
			return ShozinSendFail();

		if (pItem->isDuplicate() || pItem->isRented() || (nItemCount[i] > 1 && !pTable.m_bCountable))
			return ShozinSendFail();
	}

	g_pMain->m_ItemSpecialExchangeArray.m_lock.lock();
	foreach_stlmap_nolock(itr, g_pMain->m_ItemSpecialExchangeArray) {
		auto* pSpecialExchange = itr->second;
		if (pSpecialExchange == nullptr)
			continue;

		uint16 tablesize = 0;
		for (int x = 0; x < ITEMS_SPECIAL_EXCHANGE_GROUP; x++)
			if (pSpecialExchange->nReqItemID[x] != 0) tablesize++;

		if (tablesize != nMaterialCount)
			continue;

		uint8 nMatchCount = 0;
		for (int i = 0; i < nMaterialCount; i++) {
			if (nItemID[i]) {
				for (int x = 0; x < ITEMS_SPECIAL_EXCHANGE_GROUP; x++) {
					if (pSpecialExchange->nReqItemID[x] != 0
						&& nItemID[i] == pSpecialExchange->nReqItemID[x]
						&& nItemCount[i] == pSpecialExchange->nReqItemCount[x]) {
						nMatchCount++;
						break;
					}
				}
			}
		}

		if (nMaterialCount != nMatchCount)
			continue;

		bool added = false;
		if (pNpc->GetType() == NPC_JEWELY && itr->second->sNpcNum == 31402) added = true;
		else if (pNpc->GetType() == NPC_CRAFTSMAN && itr->second->sNpcNum == 19073) added = true;
		else if (pNpc->GetType() == 500 && itr->second->sNpcNum == 31510) added = true;
		if (added)m_exchangelist.push_back(*itr->second);
	}
	g_pMain->m_ItemSpecialExchangeArray.m_lock.unlock();

	if (m_exchangelist.empty())
		return ShozinSendFail();

	SPECIAL_PART_SEWING_EXCHANGE pExchange = m_exchangelist[myrand(0, (int32)m_exchangelist.size() - 1)];
	if (pExchange.isnull())
		return ShozinSendFail();

	if (!CheckExistSpacialItemAnd(pExchange.nReqItemID[0], pExchange.nReqItemCount[0],
		pExchange.nReqItemID[1], pExchange.nReqItemCount[1],
		pExchange.nReqItemID[2], pExchange.nReqItemCount[2],
		pExchange.nReqItemID[3], pExchange.nReqItemCount[3],
		pExchange.nReqItemID[4], pExchange.nReqItemCount[4],
		pExchange.nReqItemID[5], pExchange.nReqItemCount[5],
		pExchange.nReqItemID[6], pExchange.nReqItemCount[6],
		pExchange.nReqItemID[7], pExchange.nReqItemCount[7],
		pExchange.nReqItemID[8], pExchange.nReqItemCount[8],
		pExchange.nReqItemID[9], pExchange.nReqItemCount[9]))
		return ShozinSendFail();

	uint8 MatchingCase = 0, NeedCount = 0;
	for (int x = 0; x < ITEMS_SPECIAL_EXCHANGE_GROUP; x++) {
		if (!pExchange.nReqItemID[x]) continue;
		NeedCount++;
		foreach(itr, ExchangingItems) {
			if ((*itr) == pExchange.nReqItemID[x]) {
				MatchingCase++;
				*itr = 0;
			}
		}
	}
	if (NeedCount != MatchingCase)
		return ShozinSendFail();

	CraftingErrorCode resultOpCode = CraftingErrorCode::WrongMaterial;
	uint8 sItemSlot = 0; uint32 nItemNumber = 0; uint16 sCount = 0;

	uint32 rand = myrand(0, 10000);
	uint32 upgradeRate = pExchange.iSuccessRate;
	if (isShadowPiece) {
		if (pExchange.isShadowSucces)
			upgradeRate = 10000;
		else
			upgradeRate += (upgradeRate * 40) / 100;
	}
	if (upgradeRate > 10000) upgradeRate = 10000;

	if (upgradeRate < rand) {
		for (int i = 0; i < nMaterialCount; i++) {
			auto* pDatItemCountable = GetItem(SLOT_MAX + nItemSlot[i]);
			if (pDatItemCountable == nullptr)
				continue;

			auto pTable = g_pMain->GetItemPtr(pDatItemCountable->nNum);
			if (pTable.isnull())
				continue;

			if (!RobItem(SLOT_MAX + nItemSlot[i], pTable, nItemCount[i], false))
				continue;
		}
		if (isShadowPiece && pShadowItem) memset(pShadowItem, 0, sizeof(_ITEM_DATA));
		resultOpCode = CraftingErrorCode::Failed;
	}
	else {

		uint8 bFreeSlots = 0;
		for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++) {
			auto* pItem = GetItem(i);
			if (pItem && !pItem->nNum && ++bFreeSlots >= ITEMS_IN_EXCHANGE_GROUP)
				break;
		}

		if (bFreeSlots < 1)
			return ShozinSendFail();

		int SlotForItem = FindSlotForItem(pExchange.nGiveItemID, 1);
		if (SlotForItem < 0)
			return ShozinSendFail();

		sItemSlot = GetEmptySlot() - SLOT_MAX;
		nItemNumber = pExchange.nGiveItemID;
		sCount = pExchange.nGiveItemCount;

		auto pTable = g_pMain->GetItemPtr(nItemNumber);
		if (pTable.isnull())
			return ShozinSendFail();

		if (pTable.m_bKind == 255 && pTable.m_bCountable == 0) 
			sCount = 1;
		
		int32 nReqWeight = pTable.m_sWeight * sCount;

		if (m_sItemWeight + nReqWeight > m_sMaxWeight)
			return ShozinSendFail();

		if (!GiveItem("Shozin Exchange", nItemNumber, sCount))
			return ShozinSendFail();

		for (int i = 0; i < nMaterialCount; i++) {
			auto* pDatItemCountable = GetItem(SLOT_MAX + nItemSlot[i]);
			if (pDatItemCountable == nullptr) 
				continue;

			auto pTable = g_pMain->GetItemPtr(pDatItemCountable->nNum);
			if (pTable.isnull())
				continue;

			if (!RobItem(SLOT_MAX + nItemSlot[i], pTable, nItemCount[i], false))
				continue;
		}
		if (isShadowPiece && pShadowItem) 
			memset(pShadowItem, 0, sizeof(_ITEM_DATA));

		resultOpCode = CraftingErrorCode::Success;
		if (pExchange.isNotice) LogosItemNotice(pTable);
	}

	pUserDailyRank.SHTotalExchange++;

	newpkt << (uint8)resultOpCode << sNpcID;
	if (resultOpCode == CraftingErrorCode::Success) newpkt << nItemNumber << sItemSlot;
	Send(&newpkt);

	if (resultOpCode == CraftingErrorCode::Success) 
		ShowNpcEffect(31033, true);
	else if (resultOpCode == CraftingErrorCode::Failed)
		ShowNpcEffect(31034, true);
}
#pragma endregion