#include "stdafx.h"
#include <sstream>

using namespace std;

#pragma region CUser::BundleOpenReq(Packet & pkt)
void CUser::BundleOpenReq(Packet & pkt)
{
	Packet result(WIZ_BUNDLE_OPEN_REQ);
	uint32 bundle_index = pkt.read<uint32>();
	C3DMap* pMap = GetMap();

	if (bundle_index == 0xFFFFFFFF || bundle_index < 1)
		return;

	if (pMap == nullptr || isDead() || isTrading() || isMerchanting()
		|| isSellingMerchantingPreparing() || isFishing() || isMining())
		return;

	result << bundle_index;

	_LOOT_BUNDLE* pBundle = pMap->m_RegionItemArray.GetData(bundle_index);
	if (pBundle == nullptr || !isInRange(pBundle->x, pBundle->z, RANGE_50M))
		return;

	CUser* pBundleUser = g_pMain->GetUserPtr(pBundle->pLooter);
	if (pBundleUser == nullptr) return;
	if (pBundleUser != this && !pBundleUser->isInParty())return;
	if (pBundleUser != this && !isInSameParty(pBundleUser))return;

	result << uint8(pBundle->ItemsCount > 0 ? 1 : 0);
	if (pBundle->ItemsCount < 1) { Send(&result); return; }

	uint8 LOOT_ITEMS = 8;
	int count = 0;
	for (int i = 0; i < LOOT_ITEMS; i++) if (pBundle->Items[i].nItemID) count++;
	for (int i = 0; i < LOOT_ITEMS; i++) result << pBundle->Items[i].nItemID << pBundle->Items[i].sCount;
	if (count < LOOT_ITEMS) for (uint32 i = count; i < LOOT_ITEMS; i++) result << uint32(0) << uint16(0);
	Send(&result);
}
#pragma endregion

void CUser::JstKOAutoLooter(_LOOT_BUNDLE* pBundle) {
	if (m_fairycheck || !pBundle)
		return;
#pragma region CindWar bölgesinde oto kutu çalışması kapatıldı. 11.04.2024
	bool isCind = g_pMain->pCindWar.isStarted() && g_pMain->isCindirellaZone(GetZoneID()) && pCindWar.isEventUser();
	if (isCind)
		return;
#pragma endregion
	/*for (int i = 0; i < 8; i++)printf("%d %d %d %d\n", pBundle->ItemsCount, pBundle->Items[i].nItemID, pBundle->Items[i].sCount, pBundle->Items[i].slotid);
	printf("\n");*/

	if(g_pMain->pServerSetting.LootandGeniePremium && GetPremium()==0)
		return;

	_PARTY_GROUP* pParty = nullptr;
	C3DMap* pMap = GetMap();
	if (pMap == nullptr || !pMap->m_bAutoLoot || !pBundle->ItemsCount || (!m_autoloot && !isInParty())) 
		return;
	
	if (isInParty()) pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (!isInParty() && !isInRange(pBundle->x, pBundle->z, RANGE_50M))
		return;

	CUser* pLooter = nullptr;
	if (!m_autoloot) {
		if (!pParty) 
			return;
		
		for (int i = 0; i < MAX_PARTY_USERS; i++) {
			CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
			if (pUser == nullptr || !pUser->m_autoloot || pUser->isDead() || !isInRange(pBundle->x, pBundle->z, RANGE_50M)) continue;
			pLooter = pUser;
			break;
		}
		if (!pLooter) 
			return;
	}
	else {}

	pLooter = this;
	if (!pLooter)
		return;

	Packet result(WIZ_ITEM_GET);
	for (int i = 0; i < NPC_HAVE_ITEM_LIST; i++) {
		if (!pBundle->Items[i].nItemID || !pBundle->Items[i].sCount) 
			continue;

		if (pBundle->Items[i].nItemID == ITEM_GOLD) {
			uint32 pGold = 0;
			if (!isInParty() || pParty == nullptr) {
				if (!isInGame() || isMerchanting() || isSellingMerchantingPreparing() || isDead()) return;

				if (GetPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent) > 0)
					pGold = pBundle->Items[i].sCount * (100 + GetPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent)) / 100;
				else
					pGold = pBundle->Items[i].sCount;

				if (!JackPotNoah(pGold))
					GoldGain(pGold, false, true);

				result << uint8(LootErrorCodes::LootSolo) << pBundle->nBundleID << int8(-1) << pBundle->Items[i].nItemID << pBundle->Items[i].sCount << GetCoins() << pBundle->Items[i].slotid;
				pLooter->Send(&result);
			}
			else {
				if (!pParty) 
					return;
				
				std::vector<CUser*> partyUsers;
				for (int i = 0; i < MAX_PARTY_USERS; i++) {
					CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
					if (pUser == nullptr) 
						continue;
					
					if (!pUser->isInRange(pBundle->x, pBundle->z, RANGE_50M) || pUser->isDead())
						continue;
					
					partyUsers.push_back(pUser);
				}

				if (partyUsers.empty())
					return;

				foreach(itr, partyUsers) {
					int coins = (int)(pBundle->Items[i].sCount / (float)partyUsers.size());
					if ((*itr)->GetPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent) > 0)
						pGold = coins * (100 + (*itr)->GetPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent)) / 100;
					else
						pGold = coins;

					if (!(*itr)->JackPotNoah(pGold))
						(*itr)->GoldGain(pGold, false, true);

					result.clear();
					result << uint8(LootErrorCodes::LootPartyCoinDistribution) << pBundle->nBundleID << uint8(-1) << pBundle->Items[i].nItemID << (*itr)->GetCoins() << pBundle->Items[i].slotid;
					(*itr)->Send(&result);
				}
				result.clear();
				result << uint8(LootErrorCodes::LootPartyItemGivenAway);
				Send(&result);
			}
		}
		else {
			auto * pReceiver = GetLootUser(pBundle, pBundle->Items[i]);
			if (!pReceiver) 
				return;

			if (pReceiver->isDead() || pReceiver->isMerchanting()) pReceiver = GetLootUser(pBundle, pBundle->Items[i]);

			if (pReceiver == nullptr) 
				return;
			
			// Ensure there's enough room in this user's inventory.
			if (!pReceiver->CheckWeight(pBundle->Items[i].nItemID, pBundle->Items[i].sCount)) 
				continue;

			// Retrieve the position for this item.
			int8 bDstPos = pReceiver->FindSlotForItem(pBundle->Items[i].nItemID, pBundle->Items[i].sCount);

			if (bDstPos < 0 || bDstPos >= INVENTORY_TOTAL) 
				continue;

			if (CheckChestBlockItem(pBundle->Items[i].nItemID))
				continue;

			auto pTable = g_pMain->GetItemPtr(pBundle->Items[i].nItemID);
			_ITEM_DATA* pDstItem = &pReceiver->m_sItemArray[bDstPos];
			if (pTable.isnull() || pDstItem == nullptr)
				continue;

			if (pUserLootSetting.iConsumable == 0 && pTable.GetKind() == 95)
				continue;

			if (pUserLootSetting.iPrice != 0) {
				uint32 Price = 0;
				if (pTable.m_iSellPrice != SellTypeFullPrice)
					Price = ((pTable.m_iBuyPrice / (GetPremiumProperty(PremiumPropertyOpCodes::PremiumItemSellPercent) > 0 ? 4 : 6)) * pBundle->Items[i].sCount);
				else
					Price = (pTable.m_iBuyPrice * pBundle->Items[i].sCount);

				if (pUserLootSetting.iPrice > Price)
					continue;
			}

			uint8 WeaponType = isLootingWeaponCheck(pTable.GetKind());
			if (WeaponType == 1) {
				if (pUserLootSetting.iWeapon == 0) 
					continue;

				if (pTable.m_ItemType == 0 && pUserLootSetting.iNormal == 0) 
					continue;
				else if (pTable.m_ItemType == 1 && pUserLootSetting.iMagic == 0)
					continue;
				else if (pTable.m_ItemType == 2 && pUserLootSetting.iRare == 0) 
					continue;
				else if (pTable.m_ItemType == 3 && pUserLootSetting.iCraft == 0) 
					continue;
				else if (pTable.m_ItemType == 4 && pUserLootSetting.iUnique == 0)
					continue;
				else if (pTable.m_ItemType == 5 && pUserLootSetting.iUpgrade == 0)
					continue;
			}
			else if (WeaponType == 2) {
				if (pUserLootSetting.iArmor == 0)
					continue;
				if (pTable.m_ItemType == 0 && pUserLootSetting.iNormal == 0)
					continue;
				else if (pTable.m_ItemType == 1 && pUserLootSetting.iMagic == 0)
					continue;
				else if (pTable.m_ItemType == 2 && pUserLootSetting.iRare == 0)
					continue;
				else if (pTable.m_ItemType == 3 && pUserLootSetting.iCraft == 0)
					continue;
				else if (pTable.m_ItemType == 4 && pUserLootSetting.iUnique == 0) 
					continue;
				else if (pTable.m_ItemType == 5 && pUserLootSetting.iUpgrade == 0) 
					continue;
			}
			else if (WeaponType == 3) {
				if (pUserLootSetting.iAccessory == 0)
					continue;
				if (pTable.m_ItemType == 0 && pUserLootSetting.iNormal == 0) 
					continue;
				else if (pTable.m_ItemType == 1 && pUserLootSetting.iMagic == 0)
					continue;
				else if (pTable.m_ItemType == 2 && pUserLootSetting.iRare == 0)
					continue;
				else if (pTable.m_ItemType == 3 && pUserLootSetting.iCraft == 0)
					continue;
				else if (pTable.m_ItemType == 4 && pUserLootSetting.iUnique == 0) 
					continue;
				else if (pTable.m_ItemType == 5 && pUserLootSetting.iUpgrade == 0) 
					continue;
			}

			pDstItem->nNum = pBundle->Items[i].nItemID;
			pDstItem->sCount += pBundle->Items[i].sCount;

			if (pDstItem->sCount == pBundle->Items[i].sCount) {
				pDstItem->nSerialNum = g_pMain->GenerateItemSerial();
				pDstItem->sDuration = pTable.m_sDuration;
			}

			if (pDstItem->sCount > MAX_ITEM_COUNT) pDstItem->sCount = MAX_ITEM_COUNT;

			result.clear();
			result << uint8(pReceiver == this ? LootErrorCodes::LootSolo : LootErrorCodes::LootPartyItemGivenToUs)
				<< pBundle->nBundleID << uint8(bDstPos - SLOT_MAX) << pDstItem->nNum << pDstItem->sCount << pReceiver->GetCoins() << pBundle->Items[i].slotid;

			pReceiver->Send(&result);
			pReceiver->SetUserAbility(false);
			pReceiver->SendItemWeight();

			bool HasObtained = false;
			if (pTable.ItemClass == 4 || pTable.ItemClass == 5
				|| pTable.ItemClass == 8 || pTable.ItemClass == 21
				|| pTable.ItemClass == 22 || pTable.ItemClass == 33
				|| pTable.ItemClass == 34 || pTable.ItemClass == 35
				|| pTable.Getnum() == 379068000)
				HasObtained = true;
			
			// JstKO Yeni İtem Türüne Göre Renk Atama Cok GüzeL Eklendi. 24.04.2025
			if (pTable.m_isDropNotice && g_pMain->pServerSetting.DropNotice && !isGM())
			{
				std::ostringstream oss;
				oss << pReceiver->GetName() << " Has Obtained [" << pTable.m_sName << "] From [" << pBundle->strname << "] İn Zone [" << pMap->m_nZoneName << "]";

				std::string message = oss.str();

				uint8 r = 255, g = 255, b = 255;  // Varsayılan olarak beyaz (Genel Öğeler İçin)

				// JstKO İtem Türüne Göre Renk Atama Eklendi. 24.04.2025
				switch (pTable.ItemClass)
				{
				case 1: // Common Item Ezik :(
					r = 255; g = 255; b = 255; // Beyaz
					break;
				case 2: // Rare Item Normal :l
					r = 0; g = 255; b = 0; // Yeşil
					break;
				case 3: // Epic Item İyi :)
					r = 0; g = 191; b = 255; // Mavi
					break;
				case 4: // Legendary Item ooo Uniq :D
					r = 255; g = 215; b = 0; // Altın
					break;
				default:
					r = 255; g = 165; b = 0; // Turuncu
					break;
				}

				// JstKO LogosYolla Fonksiyonunu Çağırıyoruz, Renkle Mesajı Gönderiyoruz
				g_pMain->LogosYolla(" Drop Notice", message, r, g, b);
			}
			
			// Now notify the party that we've looted, if applicable.
			if (isInParty()) {
				result.clear();
				result << uint8(LootErrorCodes::LootPartyNotification) << pBundle->nBundleID << pBundle->Items[i].nItemID << pReceiver->GetName() << pBundle->Items[i].slotid;
				g_pMain->Send_PartyMember(GetPartyID(), &result);
				if (pReceiver != this) { result.clear(); result << uint8(LootErrorCodes::LootPartyItemGivenAway); Send(&result); }
			}

			pReceiver->NpcDropReceivedInsertLog(pBundle->Items[i].nItemID, pBundle->ItemsCount, pBundle->npcid);
		}
		pMap->RegionItemRemove(pBundle, i);
	}
}

#pragma region CUser::ItemGet(Packet & pkt)
void CUser::ItemGet(Packet& pkt)
{
	Packet result(WIZ_ITEM_GET);
	uint32 nBundleID, nItemID; uint16 SlotID;
	pkt >> nBundleID >> nItemID >> SlotID;
	LootErrorCodes pc = LootErrorCodes::LootError;

	C3DMap* pMap = GetMap();
	if (pMap == nullptr)
		return;

	if (isTrading() || isDead() || isMerchanting() || isMining()
		|| isFishing() || SlotID >= NPC_HAVE_ITEM_LIST)
		goto failed;

	_LOOT_BUNDLE* pBundle = pMap->m_RegionItemArray.GetData(nBundleID, false);
	if (pBundle == nullptr || !pBundle->ItemsCount || !pBundle->Items[SlotID].nItemID || !pBundle->Items[SlotID].sCount) 
		goto failed;

	CUser* pBundleUser = g_pMain->GetUserPtr(pBundle->pLooter);
	if (pBundleUser == nullptr || (pBundleUser != this && !pBundleUser->isInParty())) 
		goto failed;
	
	if (pBundleUser != this && !isInSameParty(pBundleUser))
		goto failed;

	CUser* pReceiver = GetLootUser(pBundle, pBundle->Items[SlotID]);
	if (pReceiver == nullptr) goto failed;

	if (nItemID == ITEM_GOLD) {
		uint32 pGold = 0;
		_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
		if (!isInParty() || pParty == nullptr) {
			if (!isInGame() || isMerchanting() || isSellingMerchantingPreparing() || isDead()) {
				goto failed;
			}
				
			if (!isInRange(pBundle->x, pBundle->z, RANGE_50M)) {
				goto failed;
			}

			if (GetPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent) > 0)
				pGold = pBundle->Items[SlotID].sCount * (100 + GetPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent)) / 100;
			else
				pGold = pBundle->Items[SlotID].sCount;

			if(!JackPotNoah(pGold))
				GoldGain(pGold, false, true);

			result << uint8(LootErrorCodes::LootSolo) << nBundleID << int8(-1) << nItemID << pBundle->Items[SlotID].sCount << GetCoins() << SlotID;
			pReceiver->Send(&result);
		}
		else {
			std::vector<CUser*> partyUsers;
			for (int i = 0; i < MAX_PARTY_USERS; i++) {
				CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (pUser == nullptr || !pUser->isInGame() || pUser->isDead())
					continue;
				
				if (!pUser->isInRange(pBundle->x, pBundle->z, RANGE_50M))
					continue;
					
				partyUsers.push_back(pUser);
			}

			if (partyUsers.empty()) {
				result << uint8(LootErrorCodes::LootError);
				Send(&result);

				return;
			}

			foreach(itr, partyUsers) {
				int coins = (int)(pBundle->Items[SlotID].sCount / (float)partyUsers.size());
				if ((*itr)->GetPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent) > 0)
					pGold = coins * (100 + (*itr)->GetPremiumProperty(PremiumPropertyOpCodes::PremiumNoahPercent)) / 100;
				else
					pGold = coins;

				if (!(*itr)->JackPotNoah(pGold))
					(*itr)->GoldGain(pGold, false, true);

				result.clear();
				result << uint8(LootErrorCodes::LootPartyCoinDistribution) << nBundleID << uint8(-1) << nItemID << (*itr)->GetCoins() << SlotID;
				(*itr)->Send(&result);
			}
			result.clear();
			result << uint8(LootErrorCodes::LootPartyItemGivenAway);
			Send(&result);
		}
	}
	else {
		if (pReceiver->isDead() || pReceiver->isMerchanting()
			|| !pReceiver->isInRange(pBundle->x, pBundle->z, RANGE_50M)) {
			pReceiver = GetLootUser(pBundle, pBundle->Items[SlotID]);
		}

		if (!pReceiver)
			return;

		// Ensure there's enough room in this user's inventory.
		if (!pReceiver->CheckWeight(pBundle->Items[SlotID].nItemID, pBundle->Items[SlotID].sCount)) {
			result << uint8(LootErrorCodes::LootNoWeight);
			pReceiver->Send(&result);
			return;
		}

		// Retrieve the position for this item.
		int8 bDstPos = pReceiver->FindSlotForItem(pBundle->Items[SlotID].nItemID, pBundle->Items[SlotID].sCount);

		if (bDstPos < 0 || bDstPos >= INVENTORY_TOTAL) {
			result << uint8(LootErrorCodes::LootError);
			Send(&result);
			return;
		}

		auto pTable = g_pMain->GetItemPtr(nItemID);
		if (pCindWar.isEventUser() && g_pMain->isCindirellaZone(pReceiver->GetZoneID())) //11.04.2024
		{
			uint32 GiveWarehouseItemID = pBundle->Items[SlotID].nItemID;
			uint16 GiveWarehouseCount = pBundle->Items[SlotID].sCount;
			uint8 bFreeSlots = 0;
			for (int i = 0; i < WAREHOUSE_MAX; i++) {
				if (pReceiver->GetWerehouseItem(i)->nNum == 0 && ++bFreeSlots >= ITEMS_IN_EXCHANGE_GROUP)
					break;
			}
			int SlotForItem = pReceiver->FindWerehouseSlotForItem(GiveWarehouseItemID, 1);
			if (bFreeSlots > 0 && SlotForItem >= 0)
			{
				if (pReceiver->GiveWerehouseItem(GiveWarehouseItemID, 1, false, true))
					pReceiver->XSafe_ItemNotice(4, GiveWarehouseItemID);

				result << uint8(LootErrorCodes::LootPartyItemGivenAway);
			}
			else
			{
				result << uint8(LootErrorCodes::LootError);
				Send(&result);
				return;
			}
		}
		else
		{
			_ITEM_DATA* pDstItem = &pReceiver->m_sItemArray[bDstPos];
			if (pTable.isnull() || pDstItem == nullptr) {
				result << uint8(LootErrorCodes::LootError);
				Send(&result);
				return;
			}

			pDstItem->nNum = pBundle->Items[SlotID].nItemID;
			pDstItem->sCount += pBundle->Items[SlotID].sCount;

			if (pDstItem->sCount == pBundle->Items[SlotID].sCount) {
				pDstItem->nSerialNum = g_pMain->GenerateItemSerial();
				pDstItem->sDuration = pTable.m_sDuration;
			}

			if (pDstItem->sCount > MAX_ITEM_COUNT) pDstItem->sCount = MAX_ITEM_COUNT;

			result << uint8(pReceiver == this ? LootErrorCodes::LootSolo : LootErrorCodes::LootPartyItemGivenToUs)
				<< nBundleID << uint8(bDstPos - SLOT_MAX) << pDstItem->nNum << pDstItem->sCount << pReceiver->GetCoins() << SlotID;
		}

		pReceiver->Send(&result);
		pReceiver->SetUserAbility(false);
		pReceiver->SendItemWeight();

		// Now notify the party that we've looted, if applicable.
		if (isInParty()) {
			result.clear();
			result << uint8(LootErrorCodes::LootPartyNotification) << nBundleID << nItemID << pReceiver->GetName() << SlotID;
			g_pMain->Send_PartyMember(GetPartyID(), &result);
			if (pReceiver != this) { result.clear(); result << uint8(LootErrorCodes::LootPartyItemGivenAway); Send(&result); }
		}

			// JstKO Yeni İtem Türüne Göre Renk Atama Cok GüzeL Eklendi. 24.04.2025
			if (pTable.m_isDropNotice && g_pMain->pServerSetting.DropNotice && !isGM())
			{
				std::ostringstream oss;
				oss << pReceiver->GetName() << " Has Obtained [" << pTable.m_sName << "] From [" << pBundle->strname << "] İn Zone [" << pMap->m_nZoneName << "]";

				std::string message = oss.str();

				uint8 r = 255, g = 255, b = 255;  // Varsayılan olarak beyaz (Genel Öğeler İçin)

				// JstKO İtem Türüne Göre Renk Atama Eklendi. 24.04.2025
				switch (pTable.ItemClass)
				{
				case 1: // Common Item Ezik :(
					r = 255; g = 255; b = 255; // Beyaz
					break;
				case 2: // Rare Item Normal :l
					r = 0; g = 255; b = 0; // Yeşil
					break;
				case 3: // Epic Item İyi :)
					r = 0; g = 191; b = 255; // Mavi
					break;
				case 4: // Legendary Item ooo Uniq :D
					r = 255; g = 215; b = 0; // Altın
					break;
				default:
					r = 255; g = 165; b = 0; // Turuncu
					break;
				}

				// JstKO LogosYolla Fonksiyonunu Çağırıyoruz, Renkle Mesajı Gönderiyoruz
				g_pMain->LogosYolla(" Drop Notice", message, r, g, b);
			}

		pReceiver->NpcDropReceivedInsertLog(nItemID, pBundle->ItemsCount, pBundle->npcid);
	}
	pMap->RegionItemRemove(pBundle, SlotID);
	return;

failed:
	result << (uint8)pc;
	Send(&result);
}
#pragma endregion

#pragma region CUser::GetLootUser(_LOOT_BUNDLE * pBundle, _LOOT_ITEM pItem)
CUser * CUser::GetLootUser(_LOOT_BUNDLE * pBundle, _LOOT_ITEM pItem)
{
	CUser * pReceiver = nullptr;
	if (pBundle == nullptr || !pItem.nItemID || !pItem.sCount) return nullptr;
	if (pItem.nItemID == ITEM_GOLD) {
		if (!isInParty()) {
			if ((GetCoins() + pItem.sCount) > COIN_MAX) return nullptr;
		}
		return this;
	}
	if (isInParty()) {
		CUser* ptr = GetItemRoutingUser(pItem.nItemID, pItem.sCount);
		return ptr == nullptr ? this : ptr;
	}
	else pReceiver = this;
	return pReceiver;
}
#pragma endregion

uint8 CUser::isLootingWeaponCheck(uint8 Kind)
{
	switch (Kind)
	{
	// Item Weapon Kind
	case WEAPON_KIND_DAGGER:
	case WEAPON_KIND_1H_SWORD:
	case WEAPON_KIND_2H_SWORD:
	case WEAPON_KIND_1H_AXE:
	case WEAPON_KIND_2H_AXE:
	case WEAPON_KIND_1H_CLUP:
	case WEAPON_KIND_2H_CLUP:
	case WEAPON_KIND_1H_SPEAR:
	case WEAPON_KIND_2H_SPEAR:
	case WEAPON_SHIELD:
	case WEAPON_KIND_BOW:
	case WEAPON_KIND_CROSSBOW:
		return 1;
	break;
	case WEAPON_KIND_STAFF:
	case WEAPON_KIND_JAMADHAR:
	case WEAPON_KIND_MACE:
	case WEAPON_KIND_PORTU_AC:
	case WEAPON_KIND_WARRIOR_AC:
	case WEAPON_KIND_ROGUE_AC:
	case WEAPON_KIND_MAGE_AC:
	case WEAPON_KIND_PRIEST_AC:
		return 2;
	break;
	case ACCESSORY_EARRING:
	case ACCESSORY_NECKLACE:
	case ACCESSORY_RING:
	case ACCESSORY_BELT:
		return 3;
	break;
	}

	return 0;
}

