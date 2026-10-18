#include "stdafx.h"

void CUser::MiningDropTester(uint8 WeaponType, uint8 Hour)
{
	if (!isGM())
		return;

	uint32 TotalExp = 0;

	std::vector<uint32> pList;
	g_pMain->m_MiningFishingItemArray.m_lock.lock();
	foreach_stlmap_nolock(itr, g_pMain->m_MiningFishingItemArray) {
		auto * pFarm = itr->second;
		if (pFarm == nullptr || pFarm->nWarStatus != 0 || pFarm->nTableType != 0)
			continue;

		if (pFarm->UseItemType != WeaponType)
			continue;

		pList.push_back(pFarm->nIndex);
	}
	g_pMain->m_MiningFishingItemArray.m_lock.unlock();

	struct GIVE_DROP_LIST
	{
		uint32 ItemID; uint16 ItemCount; std::string ItemName;
		GIVE_DROP_LIST(uint32 ItemID, uint16 ItemCount, std::string ItemName)
		{
			this->ItemID = ItemID;
			this->ItemCount = ItemCount;
			this->ItemName = ItemName;
		}
	};

	std::vector<GIVE_DROP_LIST> pDropList;

	int t1 = Hour * 60;
	int t2 = t1 * 60;
	int t3 = t2 / 5;

	if (t3 < 1)
		return;

	for (int i = 0; i < t3; i++)
	{
		uint32 bRandArray[10000], offset = 0; uint8 sItemSlot = 0;
		memset(bRandArray, 0, sizeof(bRandArray));
		foreach(itr, pList) {
			auto* p = g_pMain->m_MiningFishingItemArray.GetData(*itr);
			if (p == nullptr)
				continue;

			if (offset >= 9999)
				break;

			for (int i = 0; i < int(p->SuccessRate); i++) {
				if (i + offset >= 9999)
					break;

				bRandArray[offset + i] = p->nGiveItemID;
			}
			offset += int(p->SuccessRate);
		}

		if (offset > 9999) offset = 9999;
		uint32 bRandSlot = myrand(0, offset);
		uint32 nItemID = bRandArray[bRandSlot];

		_ITEM_TABLE pItem = g_pMain->GetItemPtr(nItemID);
		if (pItem.isnull())
			continue;

		if (nItemID == ITEM_EXP)
		{
			int Experience = 0;
			if (GetLevel() >= 1 && GetLevel() <= 34)
				Experience = 50;
			if (GetLevel() >= 35 && GetLevel() <= 59)
				Experience = 100;
			if (GetLevel() >= 60 && GetLevel() <= 69)
				Experience = 200;
			if (GetLevel() >= 70 && GetLevel() <= 83)
				Experience = 300;

			uint32 expAmount = Experience * (100 + g_pMain->m_byExpEventAmount) / 100;
			TotalExp += expAmount;
		}
		else
		{
			bool Available = false;
			foreach(itr, pDropList) {
				if (itr->ItemID == nItemID) {
					itr->ItemCount++;
					Available = true;
					break;
				}
			}
			if (!Available)
				pDropList.push_back(GIVE_DROP_LIST(nItemID, 1, pItem.m_sName));
		}
	}

	Packet result;

	std::string teeext = string_format("--------------------Mining--------------------");
	result.clear();
	result.Initialize(WIZ_CHAT);
	result << uint8(ChatType::GENERAL_CHAT) << uint8(0) << uint16(-1) << uint8(0) << teeext;
	Send(&result);

	ExpChange("Mining test", TotalExp);

	std::string ExpCommand = string_format("[Mining Test] Total Exp: %d", TotalExp);

	result.clear();
	result.Initialize(WIZ_CHAT);
	result << uint8(ChatType::GENERAL_CHAT) << uint8(0) << uint16(-1) << uint8(0) << ExpCommand;
	Send(&result);

	foreach(itr, pDropList)
	{
		std::string Command = string_format("[Mining Test] ItemName: %s, ItemCount: %d", itr->ItemName.c_str(), itr->ItemCount);

		result.clear();
		result.Initialize(WIZ_CHAT);
		result << uint8(ChatType::GENERAL_CHAT) << uint8(0) << uint16(-1) << uint8(0) << Command;
		Send(&result);
		GiveItem("Drop Test", itr->ItemID, itr->ItemCount, true, 1); // 29.10.2020 Mining Drop Testten Gelen Itemler Artýk 1 Gun Sureyle Gelecek
		//GiveItem(itr->ItemID, itr->ItemCount);
	}

	std::string teeext2 = string_format("------------------------------------------");
	result.clear();
	result.Initialize(WIZ_CHAT);
	result << uint8(ChatType::GENERAL_CHAT) << uint8(0) << uint16(-1) << uint8(0) << teeext2;
	Send(&result);
}

void CUser::FishingDropTester(uint8 WeaponType, uint8 Hour)
{
	if (!isGM())
		return;

	uint32 TotalExp = 0;

	std::vector<uint32> pFishingList;
	foreach_stlmap(itr, g_pMain->m_MiningFishingItemArray)
	{
		_MINING_FISHING_ITEM *pFarm = itr->second;
		if (pFarm == nullptr
			|| pFarm->nWarStatus != 0
			|| pFarm->nTableType != 0)
			continue;

		if (pFarm->UseItemType == WeaponType)
			pFishingList.push_back(pFarm->nIndex);
	}

	struct GIVE_DROP_LIST
	{
		uint32 ItemID = 0; uint16 ItemCount = 0; std::string ItemName = "";
	};

	std::vector<GIVE_DROP_LIST> pDropList;

	int t1 = Hour * 60;
	int t2 = t1 * 60;
	int t3 = t2 / 5;

	if (t3 <= 0) return;

	for (int i = 0; i < t3; i++)
	{
		int offset = 0;
		uint32 bRandArray[10000];
		memset(&bRandArray, 0, sizeof(bRandArray));
		uint8 sItemSlot = 0;
		foreach(itr, pFishingList)
		{
			_MINING_FISHING_ITEM *pFarming = g_pMain->m_MiningFishingItemArray.GetData(*itr);
			if (pFarming == nullptr)
				continue;

			if (offset >= 10000)
				break;

			for (int i = 0; i < int(pFarming->SuccessRate / 5); i++)
			{
				if (i + offset >= 10000)
					break;

				bRandArray[offset + i] = pFarming->nGiveItemID;
			}

			offset += int(pFarming->SuccessRate / 5);
		}

		uint32 bRandSlot = myrand(0, offset);
		uint32 nItemID = bRandArray[bRandSlot];
		_ITEM_TABLE pItem = g_pMain->GetItemPtr(nItemID);
		if (pItem.isnull())
			continue;

		if (nItemID == ITEM_EXP)
		{
			int Experience = 0;
			if (GetLevel() >= 1 && GetLevel() <= 34)
				Experience = 50;
			if (GetLevel() >= 35 && GetLevel() <= 59)
				Experience = 100;
			if (GetLevel() >= 60 && GetLevel() <= 69)
				Experience = 200;
			if (GetLevel() >= 70 && GetLevel() <= 83)
				Experience = 300;

			uint32 expAmount = Experience * (100 + g_pMain->m_byExpEventAmount) / 100;
			TotalExp += expAmount;
		}
		else
		{
			bool Available = false;

			foreach(itr, pDropList)
			{
				if (itr->ItemID == nItemID)
				{
					itr->ItemCount++;
					Available = true;
					break;
				}
			}

			if (!Available)
			{
				GIVE_DROP_LIST pList;
				pList.ItemID = nItemID;
				pList.ItemCount = 1;
				pList.ItemName = pItem.m_sName;
				pDropList.push_back(pList);
			}
		}
	}

	Packet result;

	std::string teeext = string_format("--------------------Fishing--------------------");
	result.clear();
	result.Initialize(WIZ_CHAT);
	result << uint8(ChatType::GENERAL_CHAT) << uint8(0) << uint16(-1) << uint8(0) << teeext;
	Send(&result);

	ExpChange("Mining test",TotalExp);

	std::string ExpCommand = string_format("[Fishing Test] Total Exp: %d", TotalExp);

	result.clear();
	result.Initialize(WIZ_CHAT);
	result << uint8(ChatType::GENERAL_CHAT) << uint8(0) << uint16(-1) << uint8(0) << ExpCommand;
	Send(&result);

	foreach(itr, pDropList)
	{
		std::string Command = string_format("[Fishing Test] ItemName: %s, ItemCount: %d", itr->ItemName.c_str(), itr->ItemCount);

		result.clear();
		result.Initialize(WIZ_CHAT);
		result << uint8(ChatType::GENERAL_CHAT) << uint8(0) << uint16(-1) << uint8(0) << Command;
		Send(&result);
		GiveItem("Fishing Drop Test", itr->ItemID, itr->ItemCount, true, 1); // 29.10.2020 Fishing Drop Testten Gelen Itemler Artýk 1 Gun Sureyle Gelecek
		//GiveItem(itr->ItemID, itr->ItemCount);
	}

	std::string teeext2 = string_format("------------------------------------------");
	result.clear();
	result.Initialize(WIZ_CHAT);
	result << uint8(ChatType::GENERAL_CHAT) << uint8(0) << uint16(-1) << uint8(0) << teeext2;
	Send(&result);
}

#pragma region CUser::MiningExchange(uint16 ID)
void CUser::MiningExchange(uint16 ID)
{
	uint32 bRandArray[10000];
	memset(&bRandArray, 0, sizeof(bRandArray));
	int offset = 0;
	_MINING_EXCHANGE * pMiningExchange;
	std::vector<uint32> MiningExchangeList;
	uint8 sItemSlot = 0;
	uint16 sEffect = 0;
	uint16 MiningÝndex = 0;

	if (g_pMain->m_MiningExchangeArray.GetSize() > 0)
	{
		g_pMain->m_MiningExchangeArray.m_lock.lock();
		foreach_stlmap_nolock(itr, g_pMain->m_MiningExchangeArray)
		{
			_MINING_EXCHANGE* pMining = itr->second;
			if (pMining == nullptr)
				continue;

			if (std::find(MiningExchangeList.begin(), MiningExchangeList.end(), itr->second->nIndex) == MiningExchangeList.end())
			{
				switch (ID)
				{
				case 1:
					if (itr->second->OreType == 1 && itr->second->sNpcID == 31511)
						MiningExchangeList.push_back(itr->second->nIndex);
					break;
				case 2:
					if (itr->second->OreType == 2 && itr->second->sNpcID == 31511)
						MiningExchangeList.push_back(itr->second->nIndex);
					break;
				default:
					printf("Mining Exchange unhandled OreType %d \n", itr->second->OreType);
					TRACE("Mining Exchange unhandled OreType %d \n", itr->second->OreType);
					break;
				}
			}
		}
		g_pMain->m_MiningExchangeArray.m_lock.unlock();
	}

	if (MiningExchangeList.size() > 0)
	{
		foreach(itr, MiningExchangeList)
		{
			pMiningExchange = g_pMain->m_MiningExchangeArray.GetData(*itr);

			if (pMiningExchange == nullptr)
				return;

			if (offset >= 10000)
				break;

			for (int i = 0; i < int(pMiningExchange->SuccesRate / 5); i++)
			{
				if (i + offset >= 10000)
					break;

				bRandArray[offset + i] = pMiningExchange->nGiveItemNum;
			}

			offset += int(pMiningExchange->SuccesRate / 5);
		}

		uint32 bRandSlot = myrand(0, offset);
		uint32 nItemID = bRandArray[bRandSlot];

		int32 nIndex = -1;
		g_pMain->m_MiningExchangeArray.m_lock.lock();
		foreach_stlmap_nolock(str, g_pMain->m_MiningExchangeArray)
		{
			if (str->second == nullptr)
				continue;

			if (str->second->nGiveItemNum == nItemID)
			{
				nIndex = str->second->nIndex;
				break;
			}
		}
		g_pMain->m_MiningExchangeArray.m_lock.unlock();

		_ITEM_TABLE pItem = g_pMain->GetItemPtr(nItemID);
		if (pItem.isnull())
			return;

		uint8 bFreeSlots = 0;
		for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
		{
			if (GetItem(i)->nNum == 0 && ++bFreeSlots >= ITEMS_IN_EXCHANGE_GROUP)
				break;
		}

		if (bFreeSlots < 1)
			return;

		int SlotForItem = FindSlotForItem(pItem.m_iNum, 1);
		if (SlotForItem == -1)
			return;

		_MINING_EXCHANGE * pMiningExchangeEffect = g_pMain->m_MiningExchangeArray.GetData(nIndex);
		if (pMiningExchangeEffect == nullptr)
			return;

		sItemSlot = GetEmptySlot() - SLOT_MAX;
		RobItem(pMiningExchange->nOriginItemNum, 1);
		GiveItem("Mining Exchange", nItemID, 1);

		if (pMiningExchangeEffect->GiveEffect == 1)
		{
			sEffect = 13081; // "Item" effect
			uint16 resultCode = MiningResultSuccess;
			Packet x(WIZ_MINING, uint8(MiningAttempt));
			x << resultCode << GetID() << sEffect;
			Send(&x);
		}
	}
}
#pragma endregion