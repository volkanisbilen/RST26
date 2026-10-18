#include "stdafx.h"

#pragma region CUser::BifrostPieceSendFail(uint8 errorid)
void CUser::BifrostPieceSendFail(uint8 errorid) {
	Packet result(WIZ_ITEM_UPGRADE, (uint8)ItemUpgradeOpcodes::ITEM_BIFROST_EXCHANGE);
	result << errorid;
	Send(&result);
}
#pragma endregion

void CUser::BifrostPieceProcess(Packet & pkt)
{
	uint16 sNpcID; uint32 nExchangeItemID; int8 bSrcPos = -1, errorcode = 2;
	pkt >> sNpcID >> nExchangeItemID >> bSrcPos;
	Packet result(WIZ_ITEM_UPGRADE, (uint8)ItemUpgradeOpcodes::ITEM_BIFROST_EXCHANGE);

	uint32 coinsreq = g_pMain->pServerSetting.chaoticcoins;
	if (coinsreq && !hasCoins(coinsreq)) {
		g_pMain->SendHelpDescription(this, string_format("You need %d coins to do the exchange process.", coinsreq));
		return BifrostPieceSendFail(errorcode);
	}

	if (m_BeefExchangeTime > UNIXTIME2 || m_sItemWeight >= m_sMaxWeight)
		return BifrostPieceSendFail(errorcode);

	m_BeefExchangeTime = UNIXTIME2 + 1500;

	CNpc* pNpc = g_pMain->GetNpcPtr(sNpcID, GetZoneID());
	if (pNpc == nullptr || !isInGame()
		|| !isInRange(pNpc, MAX_NPC_RANGE)
		|| isTrading() || isMerchanting()
		|| isSellingMerchantingPreparing() || isMining()
		|| isFishing() || isDead() || !isInMoradon())
		return BifrostPieceSendFail(errorcode);

	if (pNpc->GetType() != NPC_CHAOTIC_GENERATOR && pNpc->GetType() != NPC_CHAOTIC_GENERATOR2)
		return BifrostPieceSendFail(errorcode);

	auto pTable = g_pMain->GetItemPtr(nExchangeItemID);
	if (pTable.isnull() || !pTable.m_bCountable || pTable.m_iEffect2 != 251)
		return BifrostPieceSendFail(errorcode);

	_ITEM_DATA* pItem = GetItem(SLOT_MAX + bSrcPos);
	if (pItem == nullptr || bSrcPos < 0 || bSrcPos >= HAVE_MAX
		|| pItem->nNum != nExchangeItemID
		|| pItem->sCount <= 0 || pItem->isRented()
		|| pItem->isSealed() || pItem->isDuplicate())
		return BifrostPieceSendFail(errorcode);

	uint8 bFreeSlots = 0;
	for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
		if (GetItem(i) && GetItem(i)->nNum == 0 && ++bFreeSlots >= ITEMS_IN_EXCHANGE_GROUP) break;

	if (bFreeSlots < 1)
		return BifrostPieceSendFail(errorcode);

	std::vector<_ITEM_EXCHANGE> mlist;
	foreach_stlmap(itr, g_pMain->m_ItemExchangeArray) {
		if (itr->second == nullptr) continue;
		if (itr->second->bRandomFlag != 1 && itr->second->bRandomFlag != 2 && itr->second->bRandomFlag != 3) continue;
		if (itr->second->nOriginItemNum[0] == nExchangeItemID) mlist.push_back(*itr->second);
	}

	if (mlist.empty())
		return BifrostPieceSendFail(errorcode);

	int bRandArray[10000]{ 0 }; int offset = 0;
	memset(&bRandArray, 0, sizeof(bRandArray));
	foreach(itr, mlist) {
		if (!BifrostCheckExchange(itr->nIndex))
			return BifrostPieceSendFail(errorcode);

		if (itr->bRandomFlag >= 101
			|| !CheckExistItemAnd(itr->nOriginItemNum[0], itr->sOriginItemCount[0], 0, 0, 0, 0, 0, 0, 0, 0))
			continue;

		if (offset >= 9999) break;
		for (int i = 0; i < int(itr->sExchangeItemCount[0] / 5); i++) {
			if (i + offset >= 9999) break;
			bRandArray[offset + i] = itr->nExchangeItemNum[0];
		}
		offset += int(itr->sExchangeItemCount[0] / 5);
	}

	if (offset > 9999) offset = 9999;
	uint32 bRandSlot = myrand(0, offset);
	uint32 nItemID = bRandArray[bRandSlot];

	auto pGiveTable = g_pMain->GetItemPtr(nItemID);
	if (pGiveTable.isnull() || pGiveTable.m_sWeight + m_sItemWeight >= m_sMaxWeight)
		return BifrostPieceSendFail(errorcode);

	uint8 sItemSlot = FindSlotForItem(nItemID, 1);
	if (sItemSlot < 1)
		return BifrostPieceSendFail(errorcode);

	if (!RobItem(SLOT_MAX + bSrcPos, pTable, 1))
		return BifrostPieceSendFail(errorcode);

	int8 SlotCheck = sItemSlot - SLOT_MAX;
	if (!GiveItem("Bifrost Exchange", nItemID, 1)) {
		errorcode = 0;
		return BifrostPieceSendFail(errorcode);
	}

	BeefEffectType beefEffectType = BeefEffectType::EffectNone;

	if(coinsreq)
		GoldLose(coinsreq);

	if (pGiveTable.m_ItemType == 4) beefEffectType = BeefEffectType::EffectWhite;
	else if (pGiveTable.m_ItemType == 5) beefEffectType = BeefEffectType::EffectGreen;
	else beefEffectType = BeefEffectType::EffectRed;

	result << uint8(1) << nItemID << SlotCheck << nExchangeItemID << bSrcPos << (uint8)beefEffectType;
	Send(&result);
	{
		Packet newpkt(WIZ_OBJECT_EVENT, (uint8)OBJECT_ARTIFACT);
		newpkt << (uint8)beefEffectType << sNpcID;
		SendToRegion(&newpkt, nullptr, GetEventRoom());
	}

	if (pGiveTable.m_ItemType == 4 || pGiveTable.Getnum() == 379068000) LogosItemNotice(pGiveTable);
}

void CUser::LogosItemNotice(_ITEM_TABLE ptable) {
	if (ptable.isnull() || !g_pMain->pServerSetting.DropNotice) 
		return;

	Packet newpkt(WIZ_LOGOSSHOUT, uint8(0x02));
	newpkt.SByte();
	newpkt << uint8(0x04) << GetName() << ptable.m_iNum << m_bPersonalRank;
	g_pMain->Send_All(&newpkt);
}

#pragma region CUser::BifrostCheckExchange(int nExchangeID)
bool CUser::BifrostCheckExchange(int nExchangeID)
{
	// Does the exchange exist?
	_ITEM_EXCHANGE* pExchange = g_pMain->m_ItemExchangeArray.GetData(nExchangeID);
	if (pExchange == nullptr) return false;

	// Find free slots in the inventory, so that we can check against this later.
	uint8 bFreeSlots = 0;
	for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++) {
		if (GetItem(i) && GetItem(i)->nNum == 0 && ++bFreeSlots >= ITEMS_IN_EXCHANGE_GROUP) break;
	}

	// Add up the rates for this exchange to obtain a total percentage
	int nTotalPercent = 0;
	for (int i = 0; i < ITEMS_IN_EXCHANGE_GROUP; i++)
		nTotalPercent += pExchange->sExchangeItemCount[i];

	if (nTotalPercent > 9000)
		return (bFreeSlots > 0);

	int nTotalGold = 0;
	for (int i = 0; i < ITEMS_IN_EXCHANGE_GROUP; i++) {
		if (pExchange->nExchangeItemNum[i] == ITEM_GOLD)
			nTotalGold += pExchange->sExchangeItemCount[i];
	}

	// Can we hold all of these items? If we can't, we have a problem.
	uint8 bReqSlots = 0;
	uint32 nReqWeight = 0;
	for (int i = 0; i < ITEMS_IN_EXCHANGE_GROUP; i++) {
		uint32 nItemID = pExchange->nExchangeItemNum[i];

		// Does the item exist? If not, we'll ignore it (NOTE: not official behaviour).
		auto pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || nItemID == 0)
			continue;

		// Try to find a slot for the item.
		// If we can't find an applicable slot with our inventory as-is,
		// there's no point even checking further.
		int pos;
		if ((pos = FindSlotForItem(nItemID, 1)) < 0)
			return false;

		// Now that we have our slot, see if it's in use (i.e. if adding a stackable item)
		// If it's in use, then we don't have to worry about requiring an extra slot for this item.
		// The only caveat here is with having multiple of the same stackable item: 
		// theoretically we could give them OK, but when it comes time to adding them, we'll find that
		// there's too many of them and they can't fit in the same slot. 
		// As this isn't an issue with real use cases, we can ignore it.
		_ITEM_DATA* pItem = GetItem(pos);
		if (pItem == nullptr)
			return false;

		if (pItem->nNum == 0)
			bReqSlots++; // new item? new required slot.

		 // Also take into account weight (not official behaviour)
		nReqWeight += pTable.m_sWeight;
	}

	// Holding too much already?
	if (m_sItemWeight + nReqWeight > m_sMaxWeight)
		return false;

	if (bFreeSlots < bReqSlots)
		return false;

	if (nTotalGold + GetCoins() > COIN_MAX)
		return false;

	// Do we have enough slots?
	return (bFreeSlots >= bReqSlots);
}
#pragma endregion
