#include "stdafx.h"

void CUser::SendItemDisassembleFail(SmashExchangeError errorid) {
	Packet result(WIZ_ITEM_UPGRADE, (uint8)ItemUpgradeOpcodes::ITEM_OLDMAN_EXCHANGE);
	result << (uint16)errorid;
	Send(&result);
}

void CUser::ItemDisassemble(Packet & pkt)
{
	uint32 nitemid; uint16 snpcid, rollcount = 1;
	uint8 npos;
	pkt >> nitemid >> npos >> snpcid;
	std::vector<_ITEM_EXCHANGE_CRASH> mList;
	SmashExchangeError errorid = SmashExchangeError::SmashNpc;

	Packet result(WIZ_ITEM_UPGRADE, (uint8)ItemUpgradeOpcodes::ITEM_OLDMAN_EXCHANGE);

	if (m_SmashingTime > UNIXTIME2 || snpcid < MAX_USER)
		return SendItemDisassembleFail(SmashExchangeError::SmashNpc);

	m_SmashingTime = UNIXTIME2 + 1000;

	if (/*g_pMain->preload.itemsmash ||*/ !isInGame() || isDead() || isSellingMerchantingPreparing()
		|| isBuyingMerchantingPreparing() || isTrading() || isMerchanting()
		|| isMining() || isFishing() || !isInMoradon())
		return SendItemDisassembleFail(SmashExchangeError::SmashNpc);

	auto pTable = g_pMain->GetItemPtr(nitemid);
	if (pTable.isnull() || pTable.m_bCountable
		|| pTable.m_bKind == 255 || npos >= HAVE_MAX)
		return SendItemDisassembleFail(SmashExchangeError::SmashNpc);

	int16 iclas = pTable.ItemClass;
	if (iclas != 3 && iclas != 4 && iclas != 5 && iclas != 8 &&
		iclas != 31 && iclas != 32 && iclas != 33 && iclas != 34 &&
		iclas != 35 && iclas != 37 && iclas != 38 && iclas != 21 && iclas != 22)
		return SendItemDisassembleFail(SmashExchangeError::SmashItem);

	uint32 nreqcoins = 10000;
	if (pTable.m_ItemType == 4 || pTable.m_ItemType == 12) nreqcoins = 100000;

	if (!hasCoins(nreqcoins))
		return SendItemDisassembleFail(SmashExchangeError::SmashItem);

	auto* pNpc = g_pMain->GetNpcPtr(snpcid, GetZoneID());
	if (pNpc == nullptr || pNpc->GetType() != NPC_OLD_MAN_NPC
		|| !isInRange(pNpc, MAX_NPC_RANGE))
		return SendItemDisassembleFail(SmashExchangeError::SmashNpc);

	auto* pOrgItem = GetItem(npos + SLOT_MAX);
	if (pOrgItem == nullptr || pOrgItem->nNum != nitemid
		|| pOrgItem->sCount != 1 || pOrgItem->isDuplicate() || pOrgItem->isRented()
		|| pOrgItem->isExpirationTime())
		return SendItemDisassembleFail(SmashExchangeError::SmashItem);

	g_pMain->m_ItemExchangeCrashArray.m_lock.lock();
	foreach_stlmap_nolock(itr, g_pMain->m_ItemExchangeCrashArray) {
		auto* p = itr->second;
		if (p == nullptr) continue;

		bool added = false;
		if ((iclas == 2 || iclas == 7) && p->nIndex >= 1000000 && p->nIndex < 2000000) added = true;
		else if ((iclas == 3 || iclas == 4 || iclas == 5 || iclas == 8) && p->nIndex >= 2000000 && p->nIndex < 3000000) added = true;
		else if ((iclas == 32 || iclas == 33 || iclas == 34 || iclas == 35 || iclas == 37 || iclas == 38) && p->nIndex >= 3000000 && p->nIndex < 4000000) added = true;
		else if (iclas == 21 && p->nIndex >= 4000000 && p->nIndex < 5000000) added = true;
		else if ((iclas == 31 || iclas == 22) && p->nIndex >= 5000000 && p->nIndex < 6000000) added = true;
		mList.push_back(*p);
	}
	g_pMain->m_ItemExchangeCrashArray.m_lock.unlock();

	if (mList.empty())
		return SendItemDisassembleFail(SmashExchangeError::SmashNpc);

	if (iclas == 31 || iclas == 21 || iclas == 22)
		rollcount = 1;
	else if (iclas == 3 || iclas == 4 || iclas == 5 || iclas == 8)
		rollcount = 2;
	else if (iclas == 32 || iclas == 33 || iclas == 34 || iclas == 35 || iclas == 37 || iclas == 38)
		rollcount = 3;

	if (!CheckGiveSlot((uint8)rollcount))
		return SendItemDisassembleFail(SmashExchangeError::SmashInventory);

	uint32 bRandArray[10000]{ 0 }, bRandSlot[3]{ 0 }, nExchangeID[3]{ 0 };
	for (int i = 0; i < rollcount; i++) {
		int offset[3]{ 0 };
		memset(&bRandArray, 0, sizeof(bRandArray));

		foreach(itr, mList) {
			if (itr->isnull()) continue;

			if (offset[i] >= 9999) break;
			for (int z = 0; z < int(itr->sRate / 5); z++) {
				if (z + offset[i] >= 9999) break;
				bRandArray[offset[i] + z] = itr->nIndex;
			}
			offset[i] += int(itr->sRate / 5);
		}
		if (offset[i] > 9999) offset[i] = 9999;

		bRandSlot[i] = myrand(0, offset[i]);
		nExchangeID[i] = bRandArray[bRandSlot[i]];
	}

	struct itemlist { uint32 id = 0; uint16 count = 0; _ITEM_TABLE pItem = _ITEM_TABLE(); };
	itemlist list[3]{};

	int nReqWeight = 0;
	for (int i = 0; i < rollcount; i++) {

		auto pCrashItem = _ITEM_EXCHANGE_CRASH();

		auto* pmykosoft = g_pMain->m_ItemExchangeCrashArray.GetData(nExchangeID[i]);
		if (pmykosoft) pCrashItem = *pmykosoft;

		if (pCrashItem.isnull())
			return SendItemDisassembleFail(SmashExchangeError::SmashItem);

		auto pTable = g_pMain->GetItemPtr(pCrashItem.nItemID);
		if (pTable.isnull())
			return SendItemDisassembleFail(SmashExchangeError::SmashItem);

		list[i].id = pCrashItem.nItemID;
		list[i].count = pCrashItem.nCount;
		list[i].pItem = pTable;

		nReqWeight += pTable.m_sWeight * pCrashItem.nCount;
	}

	if (!ItemSmashSystemRobItemCheck(SLOT_MAX + npos, nitemid, 1))
		return SendItemDisassembleFail(SmashExchangeError::SmashItem);

	if (m_sItemWeight + nReqWeight > m_sMaxWeight || !GoldLose(nreqcoins, true))
		return SendItemDisassembleFail(SmashExchangeError::SmashItem);

	memset(pOrgItem, 0, sizeof(_ITEM_DATA));

	result << (uint16)SmashExchangeError::SmashSuccess << nitemid << npos << rollcount;
	for (int i = 0; i < rollcount; i++) {
		if (!list[i].id || !list[i].count) continue;

		int8 pos = FindSlotForItem(list[i].id, list[i].count);
		if (pos < SLOT_MAX) continue;

		GiveItem("Item Crash Exchange", list[i].id, list[i].count, true);
		result << list[i].id << uint8(pos - SLOT_MAX) << list[i].count;
		if (list[i].pItem.m_ItemType == 4 || list[i].id == 379068000) LogosItemNotice(list[i].pItem);
	}
	Send(&result);
}

bool CUser::ItemSmashSystemRobItemCheck(uint8 bPos, uint32 nItemID, uint16 sCount)
{
	_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
	if (pTable.isnull())
		return false;

	_ITEM_DATA* pItem = nullptr;
	pItem = GetItem(bPos);

	if (pItem == nullptr)
		return false;

	if (pItem->nNum != pTable.m_iNum
		|| pItem->sCount < sCount)
		return false;

	pItem->sCount -= sCount;

	// Delete the item if the stack's now 0
	if (pItem->sCount <= 0)
		memset(pItem, 0, sizeof(_ITEM_DATA));

	SendStackChange(pTable.m_iNum, pItem->sCount, pItem->sDuration, bPos - SLOT_MAX);
	return true;
}