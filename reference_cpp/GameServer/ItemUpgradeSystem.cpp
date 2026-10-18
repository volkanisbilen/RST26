#include "stdafx.h"

/**
* @brief	Checks whether the specified ItemID exists in the map passed by.
*			And returns the number of instance on success.
*
* @return	Returns the number of instance found in the map.
*/
static uint16 IsExistInMap(UpgradeItemsMap& map, int32 ItemID)
{
	uint16 temp = 0;
	foreach(itr, map)
	{
		if (itr->second.nItemID == ItemID)
			temp++;
	}
	return temp;
}

static UpgradeScrollType GetScrollType(uint32 ScrollNum)
{
	if (ScrollNum == 379221000
		|| ScrollNum == 379222000
		|| ScrollNum == 379223000
		|| ScrollNum == 379224000
		|| ScrollNum == 379225000
		|| ScrollNum == 379226000
		|| ScrollNum == 379227000
		|| ScrollNum == 379228000
		|| ScrollNum == 379229000
		|| ScrollNum == 379230000
		|| ScrollNum == 379231000
		|| ScrollNum == 379232000
		|| ScrollNum == 379233000
		|| ScrollNum == 379234000
		|| ScrollNum == 379235000
		|| ScrollNum == 379255000)
		return UpgradeScrollType::LowClassScroll;

	else if (ScrollNum == 379205000
		|| ScrollNum == 379206000
		|| ScrollNum == 379208000
		|| ScrollNum == 379209000
		|| ScrollNum == 379210000
		|| ScrollNum == 379211000
		|| ScrollNum == 379212000
		|| ScrollNum == 379213000
		|| ScrollNum == 379214000
		|| ScrollNum == 379215000
		|| ScrollNum == 379216000
		|| ScrollNum == 379217000
		|| ScrollNum == 379218000
		|| ScrollNum == 379219000
		|| ScrollNum == 379220000)
		return UpgradeScrollType::MiddleClassScroll;

	else if (ScrollNum == 379021000
		|| ScrollNum == 379022000
		|| ScrollNum == 379023000
		|| ScrollNum == 379024000
		|| ScrollNum == 379025000
		|| ScrollNum == 379030000
		|| ScrollNum == 379031000
		|| ScrollNum == 379032000
		|| ScrollNum == 379033000
		|| ScrollNum == 379034000
		|| ScrollNum == 379035000
		|| ScrollNum == 379138000
		|| ScrollNum == 379139000
		|| ScrollNum == 379140000
		|| ScrollNum == 379141000
		|| ScrollNum == 379016000
		|| ScrollNum == 379020000
		|| ScrollNum == 379018000
		|| ScrollNum == 379019000)
		return UpgradeScrollType::HighClassScroll;

	else if (ScrollNum == 379256000)
		return UpgradeScrollType::HighToRebirthScroll;

	else if (ScrollNum == 810322000)
		return UpgradeScrollType::RebirthToHighScroll;

	else if (ScrollNum == 379257000)
		return UpgradeScrollType::RebirthClassScroll;

	else if (ScrollNum == 379159000
		|| ScrollNum == 379160000
		|| ScrollNum == 379161000
		|| ScrollNum == 379162000
		|| ScrollNum == 379163000
		|| ScrollNum == 379164000)
		return UpgradeScrollType::AccessoriesClassScroll;

	else if (ScrollNum == 379152000)
		return UpgradeScrollType::ClassScroll;

	return UpgradeScrollType::InvalidScroll;
}

/**
* @brief	Packet handler for the standard item upgrade system.
*
* @param	pkt	The packet.
*/

void CUser::ItemUpgrade2(Packet& pkt, uint8 nUpgradeType)
{
	Packet hooked(XSafe, uint8(XSafeOpCodes::RPGUARD_AUTO_UPGRADE));
	Packet result(WIZ_ITEM_UPGRADE, nUpgradeType);
	_ITEM_DATA* pOriginItem;
	_ITEM_TABLE  proto = _ITEM_TABLE();
	//CNpc* pNpc;
	UpgradeScrollType m_ScrollType = UpgradeScrollType::InvalidScroll;; // the Scroll type that shall be used for this upgrade.
	//UpgradeItemsMap nItems; // the items sent by the client 
	//UpgradeIterator OriginItem;
	//uint16 sNpcID;
	uint32 nItemID; uint8 bPos;
	uint32 ScrollID = 0;
	UpgradeScrollType pUserScroll = UpgradeScrollType::InvalidScroll;
	/*eneskcbs*/
	uint8 byGrade = 0;
	uint32 byGenRate = 0;
	uint32 NewItemID = 0;
	uint32 nReqCoins = 0;
	uint8 /*bType = UpgradeType::UpgradeTypeNormal,*/ bResult = UpgradeErrorCodes::UpgradeNoMatch, ItemClass = 0;
	std::string sNoticeMessage3 = "Unknown";
	//	bool isrecycle = false, logositemac = false;

	if (isDead() || isTrading() || isStoreOpen() || isMerchanting() || isSellingMerchant() || isBuyingMerchant() || isMining())
	{
		bResult = UpgradeTrading;
		{

			//sNoticeMessage3 = string_format("Upgrading up to a maximum of grade [%d] is currently allowed.", g_pMain->pServerSetting.maxBlessingUp);
			sNoticeMessage3 = string_format("You cannot upgrade items while shopping or trading.");
			//printf("Failed 1 bResult: %d \n", bResult);
			goto fail_return;
		}
	}

	//int32 snItemID[10]; //int8 sbPos[10];
	int counter = 0;
	//for (int i = 0; i < 10; i++) // client sends 10 items in a row, starting with the item to be upgraded and latter the required items
	//{
	//	
	//	// Invalid slot ID
	//	if (bPos != -1 && bPos >= HAVE_MAX)
	//	{
	//		//printf("Failed 7 bResult: %d \n", bResult);
	//		goto fail_return;
	//	}

	//	snItemID[i] = nItemID;

	//	// We do not care where the user put the scroll in his screen. We do only care
	//	// is that whether the received Item IDs and POSs are valid.
	//	if (nItemID > 0 && bPos >= 0 && bPos < HAVE_MAX)
	//	{

	//		ITEM_UPGRADE_ITEM pUpgradeItem;
	//		pUpgradeItem.nItemID = nItemID;
	//		pUpgradeItem.bPos = bPos;
	//		nItems.insert(std::make_pair(counter++, pUpgradeItem));
	//	}
	//}

	//if (nItems.empty()) // if no items are available, then it should fail.
	//{
	//	//printf("Failed 8 bResult: %d \n", bResult);
	//	goto fail_return;
	//}
	pkt >> nItemID >> bPos;

	//printf("nItemID  %d >> bPos %d \n", nItemID , bPos);

	if (nItemID > 0 && bPos < HAVE_MAX)// we shall check if the items sent by the client do really exist on the user.
	{
		_ITEM_DATA* pItem = GetItem(SLOT_MAX + bPos);
		_ITEM_TABLE  pTable = g_pMain->GetItemPtr(nItemID);

		// Here we check and iterate through the users inventory and check
		// whether the item ids and positions are valid in the users inventory.
		// The reason why we do that is that we never trust a player.
		if (pItem->nNum != pTable.m_iNum)
		{
			//printf("Failed 93 bResult: %d \n", bResult);
			sNoticeMessage3 = string_format("Valid item data was not found in the database.");
			goto fail_return;
		}
		if (pItem == nullptr)
		{
			//printf("Failed 91 bResult: %d \n", bResult);
			sNoticeMessage3 = string_format("Valid item data was not found.");
			goto fail_return;
		}
		if (pTable.isnull())
		{
			//printf("Failed 92 bResult: %d \n", bResult);
			sNoticeMessage3 = string_format("There is no such item in the database.");
			goto fail_return;
		}
		// We shall allow no user to upgrade items with protected types like bounded, sealed etc. 
		// Even worse trying to upgrade a duplicate item cant be accepted.
		// The official behavious is to check the origin item only but c'mon
		// a user shouldnt upgrade an item by using a sealed scroll too.
		if (/*pItem->isBound()
			|| */pItem->isDuplicate()
			|| pItem->isRented()
			|| pItem->isSealed())
		{
			bResult = UpgradeErrorCodes::UpgradeRental;
			{
				//printf("Failed 10 bResult: %d \n", bResult);
				sNoticeMessage3 = string_format("You cannot upgrade a temporary, duplicate or rental item.");
				goto fail_return;
			}
		}
	}
	else
	{
		//printf("Failed 1011 bResult: %d \n", bResult);
		sNoticeMessage3 = string_format("You cannot upgrade an item that does not exist on your character.");
		goto fail_return;
	}


	// Now we are about to put the data on our pointers, iterators that
	// we are going to use for the rest of upgrade process.
	//OriginItem = nItems.begin(); // this is our first item in the map which is going to be upgraded.
	//if (OriginItem == nItems.end())
	//{
	//	//printf("Failed 11 bResult: %d \n", bResult);
	//	goto fail_return;
	//}

	proto = g_pMain->GetItemPtr(nItemID);
	pOriginItem = GetItem(SLOT_MAX + bPos);
	//printf("Failed 122 nItemID: %d \n", nItemID);
	if (pOriginItem == nullptr)
	{
		sNoticeMessage3 = string_format("You cannot upgrade an item that does not exist on your character..");
		//printf("Failed 120 bResult: %d \n", bResult);
		goto fail_return;
	}
	//if (proto.isnull())
	//{
	//	//printf("Failed 121 bResult: %d \n", bResult);
	//	goto fail_return;
	//}
	uint32 scroll_id = 0;
	//foreach(itr, nItems)
	//{
	//	pUserScroll = GetScrollType(itr->second.nItemID);
	//	scroll_id = itr->second.nItemID;
	//	if (pUserScroll != UpgradeScrollType::InvalidScroll)
	//		break;
	//}

	//if (pUserScroll == UpgradeScrollType::InvalidScroll)
	//{
	//	//printf("Failed 13 bResult: %d \n", bResult);
	//	goto fail_return;
	//}
	pUserScroll = (UpgradeScrollType)autoupscrolltype;

	if (autoupscrolltype == 4)
		pUserScroll = UpgradeScrollType::HighClassScroll;

	if (autoupscrolltype == 1)
		scroll_id = ITEM_SCROLL_LOW;
	else if (autoupscrolltype == 2)
		scroll_id = ITEM_SCROLL_MIDDLE;
	else if (autoupscrolltype == 3)
		scroll_id = ITEM_SCROLL_HIGH;
	else if (autoupscrolltype == 4)
		scroll_id = ITEM_UPGRADE_SCROLL;

	// Auto-upgrade path does not use the special 379152000 scroll directly.
	bool bUniversalSixToSevenScroll = false;

	int nReqOriginItem = nItemID;
	{
		foreach(itr, g_pMain->m_loadUpgrade) {

			if (nReqOriginItem != itr->second->ItemNumber)
				continue;

			if (itr->second->ScrollNumber != scroll_id)
				continue;

			// upgrade limit koyma 18.12.2023
			////printf("Failed 13 snItemID[1]: %d | snItemID[2]: %d | snItemID[3]: %d | snItemID[4]: %d | snItemID[5]: %d | snItemID[6]: %d \n"
			//	, snItemID[1], snItemID[2], snItemID[3], snItemID[4], snItemID[5], snItemID[6]);
			if ((scroll_id == ITEM_SCROLL_LOW
				|| scroll_id == ITEM_SCROLL_MIDDLE
				|| scroll_id == ITEM_SCROLL_HIGH) && proto.m_byGrade >= g_pMain->pServerSetting.maxBlessingUp)
			{
				std::string sNoticeMessage3 = string_format("Upgrading up to a maximum of grade [%d] is currently allowed.", g_pMain->pServerSetting.maxBlessingUp);
				SendAcsMessage(sNoticeMessage3);
				//XSafe_SendMessageBox("Upgrade Failed", string_format("Upgrading up to a maximum of grade [%d] is currently allowed.", g_pMain->pServerSetting.maxBlessingUp));
				//printf("Failed 13 m_ScrollType: %d \n", m_ScrollType);
				goto fail_return;
			}

			if ((scroll_id == ITEM_SCROLL_REBSTR) && proto.m_byGrade >= g_pMain->pServerSetting.maxBlessingUpReb)
			{
				std::string sNoticeMessage3 = string_format("Rebirth Upgrading up to a maximum of grade [%d] is currently allowed.", g_pMain->pServerSetting.maxBlessingUpReb);
				SendAcsMessage(sNoticeMessage3);
				//XSafe_SendMessageBox("Rebirth Upgrade Failed", string_format("Rebirth Upgrading up to a maximum of grade [%d] is currently allowed.", g_pMain->pServerSetting.maxBlessingUpReb));
				goto fail_return;
			}

			


			bool isValidMatch = false;
			m_ScrollType = UpgradeScrollType::InvalidScroll;
			//printf("Failed 1 m_ScrollType: %d \n", m_ScrollType);
			if (proto.ItemClass == 1) // low class scroll
			{
				m_ScrollType = UpgradeScrollType::LowClassScroll;
				//printf("Failed 2 m_ScrollType: %d \n", m_ScrollType);
			}
			else if (proto.ItemClass == 2) // middle class scroll
			{
				m_ScrollType = UpgradeScrollType::MiddleClassScroll;
				//printf("Failed 3 m_ScrollType: %d \n", m_ScrollType);
			}
			// aakinci upgrade d�zenlemesi 28.11.2023
			else if (proto.ItemClass == 3 || (proto.ItemClass == 4 && proto.m_ItemType != 11 && proto.m_ItemType != 12)) // high class scroll
			{
				m_ScrollType = UpgradeScrollType::HighClassScroll;
				//printf("Failed 4 m_ScrollType: %d \n", m_ScrollType);
			}
			else if ((proto.ItemClass == 4 && proto.m_ItemType == 11) || (proto.ItemClass == 4 && proto.m_ItemType == 12)) // rebirth scroll
			{
				m_ScrollType = UpgradeScrollType::RebirthClassScroll;
				////printf("Failed 5 m_ScrollType: %d \n", m_ScrollType);
			}
			else if (proto.ItemClass == 22) // accessories class scroll 8 to 22
			{
				m_ScrollType = UpgradeScrollType::AccessoriesClassScroll;
				//printf("Failed 6 m_ScrollType: %d \n", m_ScrollType);
			}
			// aakinci upgrade d�zenlemesi 28.11.2023 end

			if (m_ScrollType == UpgradeScrollType::LowClassScroll && !bUniversalSixToSevenScroll &&
				(pUserScroll != UpgradeScrollType::LowClassScroll
					&& pUserScroll != UpgradeScrollType::MiddleClassScroll
					&& pUserScroll != UpgradeScrollType::HighClassScroll))
			{
				//printf("Failed 7 m_ScrollType: %d \n", m_ScrollType);
				sNoticeMessage3 = string_format("You do not have the required scroll type for the item upgrade.");
				goto fail_return;
			}
			// Middle Class Scroll Match
			else if (m_ScrollType == UpgradeScrollType::MiddleClassScroll && !bUniversalSixToSevenScroll &&
				(pUserScroll != UpgradeScrollType::MiddleClassScroll
					&& pUserScroll != UpgradeScrollType::HighClassScroll && pUserScroll != UpgradeScrollType::HighToRebirthScroll
					&& pUserScroll != UpgradeScrollType::RebirthToHighScroll))// rebtohight sc eklemesi end
			{
				//printf("Failed 8 m_ScrollType: %d \n", m_ScrollType);
				sNoticeMessage3 = string_format("You do not have the required scroll type for the item upgrade..");
				goto fail_return;
			}
			// High Class Scroll Match && High to Rebirth Upgrade Class Scroll Match
			else if (m_ScrollType == UpgradeScrollType::HighClassScroll && !bUniversalSixToSevenScroll &&
				(pUserScroll != UpgradeScrollType::HighClassScroll
					&& pUserScroll != UpgradeScrollType::HighToRebirthScroll
					&& pUserScroll != UpgradeScrollType::RebirthToHighScroll))// rebtohight sc eklemesi end
			{
				//printf("Failed 9 m_ScrollType: %d \n", m_ScrollType);
				sNoticeMessage3 = string_format("You do not have the required scroll type for the item upgrade...");
				goto fail_return;
			}
			// Rebirth Upgrade Class Scroll Match
			else if (m_ScrollType == UpgradeScrollType::RebirthClassScroll && pUserScroll != UpgradeScrollType::RebirthClassScroll) // rebirth + basma
			{
				//printf("Failed 10 m_ScrollType: %d \n", m_ScrollType);
				sNoticeMessage3 = string_format("You do not have the required scroll type for the item upgrade....");
				goto fail_return;
			}
			else if (m_ScrollType == UpgradeScrollType::AccessoriesClassScroll && pUserScroll != UpgradeScrollType::AccessoriesClassScroll) // tak� basma
			{
				//printf("Failed 11 m_ScrollType: %d \n", m_ScrollType);
				sNoticeMessage3 = string_format("You do not have the required scroll type for the item upgrade.....");
				goto fail_return;
			}

			NewItemID = itr->second->NewItemNumber;
			bResult = UpgradeSucceeded;
			break;
		}

		// If we ran out of upgrades to search through, it failed.
		if (bResult != UpgradeSucceeded)
		{
			//printf("Failed 16 m_ScrollType: %d \n", m_ScrollType);
			goto fail_return;
		}

		uint32 ReqItem1 = 0;
		uint32 ReqItem2 = 0;
		uint8 rtCount = 0;

		foreach(itr, g_pMain->m_UpgradeSettings)
		{
			////printf("Failed 171 itr->second->nReqItem1: %d  || itr->second->nReqItem2: %d || nItemID: %d || scroll_id: %d || itr->second->ItemGrade: %d || proto.m_byGrade: %d\n"
			//	, itr->second->nReqItem1, itr->second->nReqItem2, nItemID, scroll_id, itr->second->ItemGrade, proto.m_byGrade);

			if (itr->second->nReqItem1 != scroll_id
				&& itr->second->nReqItem2 != scroll_id)
				continue;

			if (itr->second->nReqItem1 != 0
				&& itr->second->nReqItem2 != 0)
				continue;

			if (itr->second->ItemGrade != proto.m_byGrade && itr->second->ItemGrade != 99)
				continue;

			if (proto.m_ItemType != itr->second->ItemType)
				continue;

			ReqItem1 = itr->second->nReqItem1;
			ReqItem2 = itr->second->nReqItem2;
			byGenRate = itr->second->SuccesRate;
			nReqCoins = itr->second->nReqCoins;
			rtCount++;
		}

		if (g_pMain->pServerSetting.AutoUpFasterKatlama > 0 && m_AutoUpgradeisFast > 0)
			nReqCoins = uint32(nReqCoins * g_pMain->pServerSetting.AutoUpFasterKatlama);

		if (!hasCoins(nReqCoins))
		{
			bResult = UpgradeNeedCoins;
			{
				//printf("Failed 17 m_ScrollType: %d \n", m_ScrollType);
				sNoticeMessage3 = string_format("You do not have the coins needed to upgrade the item.");
				goto fail_return;
			}
		}

		if (!CheckExistItem(ReqItem1, 1) && ReqItem1 > 0) {
			bResult = UpgradeNoMatch;
			{
				sNoticeMessage3 = string_format("You do not have the scroll required for the item upgrade!!");
				//printf("Failed 20 m_ScrollType: %d \n", m_ScrollType);
				goto fail_return;
			}
		}
		if (!CheckExistItem(ReqItem2, 1) && ReqItem2 > 0) {
			bResult = UpgradeNoMatch;
			{
				sNoticeMessage3 = string_format("You do not have the scroll required for the item upgrade!");
				//printf("Failed 21 m_ScrollType: %d \n", m_ScrollType);
				goto fail_return;
			}
		}

		if (rtCount == 0) {
			bResult = UpgradeNoMatch;
			{
				sNoticeMessage3 = string_format("The information required for the item upgrade was not found in the database.");
				//printf("Failed 22 m_ScrollType: %d \n", m_ScrollType);
				goto fail_return;
			}
		}

		// Generate a random number, test if the item burned.
		int rand = myrand(0, 10000/* myrand(9000, 10000)*/), GenRate;

		GenRate = byGenRate;


		if (GenRate < rand)
		{
			memset(pOriginItem, 0, sizeof(_ITEM_DATA));

			bResult = UpgradeFailed;
			ItemUpgradeNotice(proto, UpgradeFailed, scroll_id);

			std::string UP = "ItemUpgrade";
			std::string STA = "FAILED";

			if (isGM())
				g_pMain->SendHelpDescription(this, string_format("Item Upgrade Failed : (+%d -> +%d) SuccesRate : %d ItemID: %d - ScrollID[1] : %d",
					proto.m_byGrade, proto.m_byGrade + 1, byGenRate, nItemID, scroll_id));

			hooked << uint8(2) << uint32(nReqCoins);
			Send(&hooked);

			GoldLose(nReqCoins, true);
		}
		else
		{
			// Generate the new item ID
			int nNewItemID = NewItemID;

			int nNewItemLevel = nNewItemID % 10;

			/*if((m_ScrollType != UpgradeScrollType::RebirthClassScroll && nNewItemLevel > 8)
			|| (m_ScrollType == UpgradeScrollType::RebirthClassScroll && nNewItemLevel > 5))
			goto fail_return; */

			// Does this new item exist?
			_ITEM_TABLE  newProto = g_pMain->GetItemPtr(nNewItemID);

			if (newProto.isnull())
			{ // if not, just say it doesn't match. No point removing the item anyway (like official :/).
				bResult = UpgradeNoMatch;
				{
					sNoticeMessage3 = string_format("The item you will obtain after the item upgrade was not found in the database.");
					//printf("Failed 24 m_ScrollType: %d \n", m_ScrollType);
					goto fail_return;
				}
			}

			// Update the user's item in their inventory with the new item
			pOriginItem->nNum = nNewItemID;
			//_ITEM_DATA pItemx{};
			//pItemx.nNum = nNewItemID;
			//pItemx.bFlag = ITEM_FLAG_NONE;
			//printf("ITEM_FLAG_NONE pItemx.nNum: %d \n", pItemx.nNum);
			//if (pOriginItem->bFlag == ITEM_FLAG_BOUND)
			//{
			//	printf("ITEM_FLAG_NONE pItemx.nNum: %d \n", pOriginItem->nNum);
			//	pOriginItem->bFlag = ITEM_FLAG_NOT_BOUND; // 23.03.2024 eklemesi
			//}
			if (pOriginItem->bFlag == ITEM_FLAG_BOUND)
			{
				Packet resultxx(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_SEAL));
				resultxx << uint8(4);

				uint8 bSealResult = g_DBAgent.UpdateUserSealItem(pOriginItem->nSerialNum, pOriginItem->nNum, uint8(4), pOriginItem->bFlag, this);
				//printf("bSealResult: %d | pOriginItem->nNum:%d | pOriginItem->bFlag: %d | bPos: %d \n", bSealResult, snItemID[0], pOriginItem->bFlag, bPosxx);
				if (bSealResult != 1) {
					resultxx << bSealResult << pOriginItem->nNum << bPos;
					Send(&resultxx);
				}
				else
				{
					pOriginItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_NOT_BOUND;
					resultxx << bSealResult << pOriginItem->nNum << bPos;
					Send(&resultxx);
				}
				//uint8 bSealResult = g_DBAgent.UpdateUserSealItem(pOriginItem->nSerialNum, pOriginItem->nNum, 3, pOriginItem->bFlag, this);
				//resultxx << bSealResult << pOriginItem->nNum << bPos;
				//Send(&resultxx);
			}
			// Reset the durability also, to the new cap.
			pOriginItem->sDuration = newProto.m_sDuration;

			// Send upgrade notice.
			ItemUpgradeNotice(newProto, UpgradeSucceeded, scroll_id);
			std::string UP = "ItemUpgrade";
			// Rob gold upgrade noah
			GoldLose(nReqCoins, true);
			if (isGM())
				g_pMain->SendHelpDescription(this, string_format("Item Upgrade Succesfully : (+%d -> +%d) SuccesRate : %d ItemID: %d - ScrollID[1] : %d", proto.m_byGrade, newProto.m_byGrade, byGenRate, nItemID, scroll_id));

			hooked << uint8(3) << uint32(nReqCoins);
			Send(&hooked);
			// Replace the item ID in the list for the packet
			nItemID = nNewItemID;
		}

		//	uint32 logitemid[9]{ 0 }; uint16 logitemcount[9]{ 0 };
			// Remove the source item 
			// Remove all required items, if applicable.
		{
			RobItem(scroll_id, 1);
			//_ITEM_DATA* pItem = GetItem(SLOT_MAX + bPos);
			//if (pItem != nullptr
			//	&& pItem->nNum != 0
			//	&& pItem->sCount != 0)
			//{
			//	RobItem(scroll_id, 1);

			//	if (pItem->sCount == 0 && pItem->nNum == nItemID) {
			//		pItem->nExpirationTime = 0;
			//		memset(pItem, 0, sizeof(_ITEM_DATA));
			//	}
			//}
		}

		UpgradeInsertLog2(proto.Getnum(), nReqCoins, (uint8)pUserScroll, bResult, scroll_id, 1);
	}
	result << int8(1) << bResult;
	if (bResult == UpgradeFailed && nUpgradeType != 3)
		result << uint8(0); // aakinci false

	result << nItemID << bPos;
	Send(&result);

	// Send the result to everyone in the area
	// (i.e. make the anvil do its burned/upgraded indicator thing)
	uint16 sNpcID = g_pMain->m1anvilid;
	if (GetZoneID() == ZONE_MORADON)
		sNpcID = g_pMain->m1anvilid;
	else if (GetZoneID() == ZONE_MORADON2)
		sNpcID = g_pMain->m2anvilid;
	else if (GetZoneID() == ZONE_MORADON3)
		sNpcID = g_pMain->m3anvilid;
	else if (GetZoneID() == ZONE_MORADON4)
		sNpcID = g_pMain->m4anvilid;
	else if (GetZoneID() == ZONE_MORADON5)
		sNpcID = g_pMain->m5anvilid;
	else
		sNpcID = g_pMain->m1anvilid;

	result.Initialize(WIZ_OBJECT_EVENT);
	result << uint8(OBJECT_ANVIL) << bResult << uint16(sNpcID);//sNpcID aakinci
	SendToRegion(&result, nullptr, GetEventRoom());
	return;
fail_return:

	hooked << uint8(4) << uint32(0);
	Send(&hooked);

	if (bResult == UpgradeFailed)
		result << uint8(0);// aakinci false

	result << int8(1) << bResult;
	result << nItemID << bPos;
	Send(&result);
	SendAcsMessage(sNoticeMessage3);
}
void CUser::ItemUpgrade(Packet& pkt, uint8 nUpgradeType)
{
#define ITEM_LOW_CLASS_TRINA 353000000
#define ITEM_MIDDLE_CLASS_TRINA 352900000
	Packet result(WIZ_ITEM_UPGRADE, nUpgradeType);
	_ITEM_DATA* pOriginItem;
	_ITEM_TABLE  proto = _ITEM_TABLE();
	CNpc* pNpc;
	UpgradeScrollType m_ScrollType = UpgradeScrollType::InvalidScroll;; // the Scroll type that shall be used for this upgrade.
	UpgradeItemsMap nItems; // the items sent by the client 
	UpgradeIterator OriginItem;
	uint16 sNpcID;
	int32 nItemID; int8 bPos;
	uint32 ScrollID = 0;
	UpgradeScrollType pUserScroll = UpgradeScrollType::InvalidScroll;
	/*eneskcbs*/
	uint8 byGrade = 0;
	uint32 byGenRate = 0;
	uint32 NewItemID = 0;
	uint32 nReqCoins = 0;
	uint8 nItemUpgradeStatus = 1;
	int8 bType = UpgradeType::UpgradeTypeNormal, bResult = UpgradeErrorCodes::UpgradeNoMatch, ItemClass = 0;
	bool trina = false, karivdis = false, Accessories = false, isLowClassTrina = false, isMiddleClassTrina = false;
	bool ringtrina = false;
	DWORD CheckTime = GetTickCount();

	bool isrecycle = false, logositemac = false;

	if (isDead() || isTrading() || isStoreOpen() || isMerchanting() || isSellingMerchant() || isBuyingMerchant() || isMining())
	{
		bResult = UpgradeTrading;
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	if (bType != UpgradeTypeNormal && bType != UpgradeTypePreview) {
		bResult = UpgradeNoMatch;
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	if (m_sUserUpgradeCount >= g_pMain->pServerSetting.UserMaxUpgradeCount)
	{
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	if ((UNIXTIME - m_tUpgradeDelay) < UPGRADE_DELAY) //30.08.2020 Seri Upgrade Fix
	{
		Packet newpkt(XSafe, uint8(XSafeOpCodes::ERRORMSG));
		newpkt << uint8(1) << uint8(1) << std::string("Magic Anvil") << std::string("You are doing item upgrade very fast");
		Send(&newpkt);
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	m_tUpgradeDelay = UNIXTIME;
	pkt >> bType; // either preview or upgrade, need to allow for these types
	pkt >> sNpcID;

	if (bType > 2 || bType < 1)
	{
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	pNpc = g_pMain->GetNpcPtr(sNpcID, GetZoneID());

	if (pNpc == nullptr
		|| pNpc->GetType() != NPC_ANVIL)
	{
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	int32 snItemID[10];
	int counter = 0;
	for (int i = 0; i < 10; i++) // client sends 10 items in a row, starting with the item to be upgraded and latter the required items
	{
		pkt >> nItemID >> bPos;

		// Invalid slot ID
		if (bPos != -1 && bPos >= HAVE_MAX)
		{
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}

		snItemID[i] = nItemID;

		// We do not care where the user put the scroll in his screen. We do only care
		// is that whether the received Item IDs and POSs are valid.
		if (nItemID > 0 && bPos >= 0 && bPos < HAVE_MAX)
		{

			ITEM_UPGRADE_ITEM pUpgradeItem;
			pUpgradeItem.nItemID = nItemID;
			pUpgradeItem.bPos = bPos;
			nItems.insert(std::make_pair(counter++, pUpgradeItem));
		}
	}

	if (nItems.empty()) // if no items are available, then it should fail.
	{
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	foreach(itr, nItems) // we shall check if the items sent by the client do really exist on the user.
	{
		_ITEM_DATA* pItem = GetItem(SLOT_MAX + itr->second.bPos);
		_ITEM_TABLE  pTable = g_pMain->GetItemPtr(itr->second.nItemID);

		// Here we check and iterate through the users inventory and check
		// whether the item ids and positions are valid in the users inventory.
		// The reason why we do that is that we never trust a player.
		if (pItem == nullptr
			|| pTable.isnull()
			|| pItem->nNum != itr->second.nItemID)
		{
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}
		// We shall allow no user to upgrade items with protected types like bounded, sealed etc. 
		// Even worse trying to upgrade a duplicate item cant be accepted.
		// The official behavious is to check the origin item only but c'mon
		// a user shouldnt upgrade an item by using a sealed scroll too.
		else if (pItem->isBound()
			|| pItem->isDuplicate()
			|| pItem->isRented()
			|| pItem->isSealed())
		{
			bResult = UpgradeErrorCodes::UpgradeRental;
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}
	}


	// Now we are about to put the data on our pointers, iterators that
	// we are going to use for the rest of upgrade process.
	OriginItem = nItems.begin(); // this is our first item in the map which is going to be upgraded.
	if (OriginItem == nItems.end())
	{
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	proto = g_pMain->GetItemPtr(OriginItem->second.nItemID);
	pOriginItem = GetItem(SLOT_MAX + OriginItem->second.bPos);
	if (proto.isnull() || pOriginItem == nullptr)
	{
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	uint32 scroll_id = 0;
	foreach(itr, nItems)
	{
		pUserScroll = GetScrollType(itr->second.nItemID);
		scroll_id = itr->second.nItemID;
		if (pUserScroll != UpgradeScrollType::InvalidScroll)
			break;
	}

	if (pUserScroll == UpgradeScrollType::InvalidScroll)
	{
		printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
		goto fail_return;
	}

	// JstKO: 379152000 is the special universal +6 -> +7 scroll.
	// It is stored as ClassScroll, so allow it through class validation only when
	// the origin item is exactly +6. NEW_UPGRADE2369 still decides the real target item.
	bool bUniversalSixToSevenScroll = (scroll_id == 379152000 && proto.m_byGrade == 6);

	int nReqOriginItem = OriginItem->second.nItemID;
	{
		foreach(itr, g_pMain->m_loadUpgrade) {

			if (nReqOriginItem != itr->second->ItemNumber)
				continue;
			if (pUserScroll != UpgradeScrollType::AccessoriesClassScroll) {
				if (snItemID[1] > 0
					&& snItemID[1] != ITEM_TRINA
					&& snItemID[1] != ITEM_KARIVDIS
					&& snItemID[1] != ITEM_MIDDLE_CLASS_TRINA
					&& snItemID[1] != ITEM_LOW_CLASS_TRINA
					&& snItemID[1] != 890092000
					&& itr->second->ScrollNumber != snItemID[1])
					continue;

				if (snItemID[2] > 0
					&& snItemID[2] != ITEM_TRINA
					&& snItemID[2] != ITEM_KARIVDIS
					&& snItemID[2] != ITEM_MIDDLE_CLASS_TRINA
					&& snItemID[2] != ITEM_LOW_CLASS_TRINA
					&& snItemID[2] != 890092000
					&& itr->second->ScrollNumber != snItemID[2])
					continue;
			}

			if (pUserScroll == UpgradeScrollType::AccessoriesClassScroll)
			{
				if (snItemID[3] != 354000000
					&& itr->second->ScrollNumber != snItemID[3])
					continue;
			}

			if (IsExistInMap(nItems, ITEM_TRINA)) // Trina Piece 
				trina = true;

			if (IsExistInMap(nItems, ITEM_KARIVDIS)) // Karvis Rebirth Trina
				karivdis = true;

			if (IsExistInMap(nItems, 890092000)) //ITEM_BLESSINGLOGOS
				logositemac = true;

			if (IsExistInMap(nItems, ITEM_MIDDLE_CLASS_TRINA)) // Karvis Rebirth Trina
				isMiddleClassTrina = true;

			if (IsExistInMap(nItems, ITEM_LOW_CLASS_TRINA)) // Karvis Rebirth Trina
				isLowClassTrina = true;

			if (IsExistInMap(nItems, 354000000))
				ringtrina = true;

			if (ringtrina && IsExistInMap(nItems, 354000000) > 1)
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}

			if ((trina && karivdis)
				|| IsExistInMap(nItems, ITEM_TRINA) > 1
				|| IsExistInMap(nItems, ITEM_KARIVDIS) > 1)
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}

			if ((isLowClassTrina && isMiddleClassTrina)
				|| IsExistInMap(nItems, ITEM_MIDDLE_CLASS_TRINA) > 1
				|| IsExistInMap(nItems, ITEM_LOW_CLASS_TRINA) > 1)
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}

			if (logositemac && (trina || karivdis || isLowClassTrina || isMiddleClassTrina))
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}

			if (logositemac
				&& proto.m_ItemType != 11 && proto.m_ItemType != 12
				&& proto.m_ItemType != 4 && proto.m_ItemType != 5)
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}

			bool isValidMatch = false;

			m_ScrollType = UpgradeScrollType::InvalidScroll;
			if (proto.ItemClass == 1) // low class scroll
				m_ScrollType = UpgradeScrollType::LowClassScroll;
			else if (proto.ItemClass == 2) // middle class scroll
				m_ScrollType = UpgradeScrollType::MiddleClassScroll;
			else if (proto.ItemClass == 3) // high class scroll
				m_ScrollType = UpgradeScrollType::HighClassScroll;
			else if (proto.ItemClass == 4) // rebirth scroll
				m_ScrollType = UpgradeScrollType::RebirthClassScroll;
			else if (proto.ItemClass == 8) // accessories class scroll
				m_ScrollType = UpgradeScrollType::AccessoriesClassScroll;

			if (m_ScrollType == UpgradeScrollType::LowClassScroll && !bUniversalSixToSevenScroll &&
				(pUserScroll != UpgradeScrollType::LowClassScroll
					&& pUserScroll != UpgradeScrollType::MiddleClassScroll
					&& pUserScroll != UpgradeScrollType::HighClassScroll))
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}
			// Middle Class Scroll Match
			else if (m_ScrollType == UpgradeScrollType::MiddleClassScroll && !bUniversalSixToSevenScroll &&
				(pUserScroll != UpgradeScrollType::MiddleClassScroll
					&& pUserScroll != UpgradeScrollType::HighClassScroll && pUserScroll != UpgradeScrollType::HighToRebirthScroll
					&& pUserScroll != UpgradeScrollType::RebirthToHighScroll))
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}
			// High Class Scroll Match && High to Rebirth Upgrade Class Scroll Match
			else if (m_ScrollType == UpgradeScrollType::HighClassScroll && !bUniversalSixToSevenScroll &&
				(pUserScroll != UpgradeScrollType::HighClassScroll
					&& pUserScroll != UpgradeScrollType::HighToRebirthScroll
					&& pUserScroll != UpgradeScrollType::RebirthToHighScroll))
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}
			// Rebirth Upgrade Class Scroll Match
			else if (m_ScrollType == UpgradeScrollType::RebirthClassScroll && pUserScroll != UpgradeScrollType::RebirthClassScroll && pUserScroll != UpgradeScrollType::RebirthToHighScroll) // rebirth + basma
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}
			else if (m_ScrollType == UpgradeScrollType::AccessoriesClassScroll && pUserScroll != UpgradeScrollType::AccessoriesClassScroll) // tak� basma
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}

			if (logositemac
				&& (pUserScroll == UpgradeScrollType::HighToRebirthScroll
					|| pUserScroll == UpgradeScrollType::AccessoriesClassScroll
					|| pUserScroll == UpgradeScrollType::RebirthToHighScroll))
			{
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}

			if (logositemac)
			{
				if ((proto.m_ItemType == 4 || proto.m_ItemType == 5) && proto.m_byGrade >= g_pMain->pServerSetting.maxBlessingUp)
				{
					printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
					goto fail_return;
				}

				if ((proto.m_ItemType == 11 || proto.m_ItemType == 12) && proto.m_byGrade >= g_pMain->pServerSetting.maxBlessingUpReb)
				{
					printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
					goto fail_return;
				}
			}

			/*	if (!(proto.ItemClass <= ItemClass)
					&& m_ScrollType == UpgradeScrollType::AccessoriesClassScroll)
					continue;*/

			if (pUserScroll == UpgradeScrollType::AccessoriesClassScroll) {
				bool itemsAreDifferent =
					(snItemID[0] != snItemID[1]) ||
					(snItemID[0] != snItemID[2]) ||
					(snItemID[1] != snItemID[0]) ||
					(snItemID[1] != snItemID[2]) ||
					(snItemID[2] != snItemID[0]) ||
					(snItemID[2] != snItemID[1]);

				bool invalidItemIDExists =
					(snItemID[0] <= 0) ||
					(snItemID[1] <= 0) ||
					(snItemID[2] <= 0) ||
					(snItemID[3] <= 0);

				if (itemsAreDifferent || invalidItemIDExists) {
					printf("[%s]-[%d]-%d-%d\n", __FUNCTION__, __LINE__, itemsAreDifferent, invalidItemIDExists);
					goto fail_return;
				}
			}

			NewItemID = itr->second->NewItemNumber;
			bResult = UpgradeSucceeded;
			break;
		}

		// If we ran out of upgrades to search through, it failed.
		if (bResult != UpgradeSucceeded)
		{
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}

		uint32 ReqItem1 = 0;
		uint32 ReqItem2 = 0;
		uint8 rtCount = 0;

			// JstKO: 379152000 is a universal +6 -> +7 scroll.
			// Some ITEM_UPGRADE_SETTINGS2369 rows store ItemGrade as source grade (+6),
			// some as target grade (+7), and some as 99 wildcard.
			uint8 byUniversalTargetGrade = 0;
			if (bUniversalSixToSevenScroll && NewItemID > 0)
			{
				_ITEM_TABLE newProtoForSettings = g_pMain->GetItemPtr(NewItemID);
				if (!newProtoForSettings.isnull())
					byUniversalTargetGrade = newProtoForSettings.m_byGrade;
			}
		if (logositemac)
		{
			ReqItem1 = 379257000;
			ReqItem1 = 890092000;
			rtCount++;
		}
		else
		{
			foreach(itr, g_pMain->m_UpgradeSettings)
			{
				if (m_ScrollType == UpgradeScrollType::AccessoriesClassScroll) {
					if (itr->second->nReqItem1 != snItemID[4]
						&& itr->second->nReqItem2 != snItemID[4])
						continue;
					if (itr->second->nReqItem1 != snItemID[3]
						&& itr->second->nReqItem2 != snItemID[3])
						continue;
					if (itr->second->ItemGrade != proto.m_byGrade && itr->second->ItemGrade != 99)
						continue;
				}
					else {
						if (bUniversalSixToSevenScroll)
						{
							// 379152000 may be stored in ReqItemID1 or ReqItemID2.
							if (itr->second->nReqItem1 != scroll_id
								&& itr->second->nReqItem2 != scroll_id)
								continue;

							if (itr->second->ItemGrade != proto.m_byGrade
								&& itr->second->ItemGrade != byUniversalTargetGrade
								&& itr->second->ItemGrade != 99)
								continue;
						}
						else
						{
							if (itr->second->nReqItem1 != snItemID[1]
								&& itr->second->nReqItem2 != snItemID[1])
								continue;
							if (itr->second->nReqItem1 != snItemID[2]
								&& itr->second->nReqItem2 != snItemID[2])
								continue;
							if (itr->second->ItemGrade != proto.m_byGrade && itr->second->ItemGrade != 99)
								continue;
						}
					}

					if (bUniversalSixToSevenScroll)
					{
						if (itr->second->ItemType != proto.m_ItemType
							&& itr->second->ItemType != 0
							&& itr->second->ItemType != 99)
							continue;
					}
					else if (proto.m_ItemType != itr->second->ItemType)
						continue;

				ReqItem1 = itr->second->nReqItem1;
				ReqItem2 = itr->second->nReqItem2;
				byGenRate = itr->second->SuccesRate;
				nReqCoins = itr->second->nReqCoins;
				nItemUpgradeStatus = itr->second->ItemUpgradeStatus;
				rtCount++;
			}
		}

		if (!hasCoins(nReqCoins))
		{
			bResult = UpgradeNeedCoins;
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}
		if (!IsExistInMap(nItems, ReqItem2) && ReqItem2 > 0) {
			bResult = UpgradeNoMatch;
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}
		if (!IsExistInMap(nItems, ReqItem1) && ReqItem1 > 0) {
			bResult = UpgradeNoMatch;
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}
		if (!CheckExistItem(ReqItem1, 1) && ReqItem1 > 0) {
			bResult = UpgradeNoMatch;
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}
		if (!CheckExistItem(ReqItem2, 1) && ReqItem2 > 0) {
			bResult = UpgradeNoMatch;
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}

		if (rtCount == 0) {
			bResult = UpgradeNoMatch;
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}

		if (nItemUpgradeStatus == 0) {
			bResult = UpgradeNoMatch;
			printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
			goto fail_return;
		}

		// Generate a random number, test if the item burned.
		int rand = myrand(0, 10000/* myrand(9000, 10000)*/), GenRate;
		if (trina || karivdis || isLowClassTrina || isMiddleClassTrina)
		{
			GenRate = byGenRate;
			if (GenRate > 10000)
				GenRate = 10000;
		}
		else
			GenRate = byGenRate;

		if (logositemac)
			GenRate = 33 * 100;

		// JstKO GM Full Upgrade %100 �zel Eklendi 05.01.2025
		if (isGM() && GMUpgradeThis)
			GenRate = 10000;

		if (bType == UpgradeTypeNormal && GenRate < rand)
		{
			if (logositemac)
			{
				uint32 rand_id = myrand(0, 6700);
				if (static_cast<int>(rand_id) < rand && proto.m_byGrade > 1)
				{
					uint32 newid = pOriginItem->nNum -= 1;
					_ITEM_TABLE newProto = g_pMain->GetItemPtr(newid);
					if (newProto.isnull()) {
						bResult = UpgradeNoMatch;
						printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
						goto fail_return;
					}

					pOriginItem->nNum = newid;
					OriginItem->second.nItemID = pOriginItem->nNum;
				}
			}
			else
			{
				memset(pOriginItem, 0, sizeof(_ITEM_DATA));
			}

			bResult = UpgradeFailed;
			ItemUpgradeNotice(proto, UpgradeFailed, scroll_id);

			std::string UP = "ItemUpgrade";
			std::string STA = "FAILED";

			if (isGM()) g_pMain->SendHelpDescription(this, string_format("Item Upgrade Failed : (+%d -> +%d) SuccesRate : %d ScrollID : %d , %d ",
				proto.m_byGrade, proto.m_byGrade + 1, byGenRate, snItemID[1], snItemID[2]));
			GoldLose(nReqCoins, true);
		}
		else
		{
			// Generate the new item ID
			int nNewItemID = NewItemID;

			int nNewItemLevel = nNewItemID % 10;

			/*if((m_ScrollType != UpgradeScrollType::RebirthClassScroll && nNewItemLevel > 8)
			|| (m_ScrollType == UpgradeScrollType::RebirthClassScroll && nNewItemLevel > 5))
			goto fail_return; */

			// Does this new item exist?
			_ITEM_TABLE  newProto = g_pMain->GetItemPtr(nNewItemID);

			if (newProto.isnull())
			{ // if not, just say it doesn't match. No point removing the item anyway (like official :/).
				bResult = UpgradeNoMatch;
				printf("[%s]-[%d]\n", __FUNCTION__, __LINE__);
				goto fail_return;
			}

			if (bType != UpgradeTypePreview)
			{
				// Update the user's item in their inventory with the new item
				pOriginItem->nNum = nNewItemID;

				// Reset the durability also, to the new cap.
				pOriginItem->sDuration = newProto.m_sDuration;

				// Send upgrade notice.
				ItemUpgradeNotice(newProto, UpgradeSucceeded, scroll_id);
				std::string UP = "ItemUpgrade";
				// Rob gold upgrade noah
				GoldLose(nReqCoins, true);
				if (isGM())
					g_pMain->SendHelpDescription(this, string_format("Item Upgrade Succesfully : (+%d -> +%d) SuccesRate : %d ScrollID : %d , %d ", proto.m_byGrade, newProto.m_byGrade, byGenRate, snItemID[1], snItemID[2]));
			}

			// Replace the item ID in the list for the packet
			OriginItem->second.nItemID = nNewItemID;
		}

		uint32 logitemid[9]{ 0 }; uint16 logitemcount[9]{ 0 };
		// Remove the source item 
		if (bType != UpgradeTypePreview)
		{
			// Remove all required items, if applicable.
			for (int i = 1; i < MAX_ITEMS_REQ_FOR_UPGRADE; i++)
			{
				auto xitr = nItems.find(i);
				if (xitr == nItems.end())
					continue;

				_ITEM_DATA* pItem = GetItem(SLOT_MAX + xitr->second.bPos);
				if (pItem == nullptr
					|| pItem->nNum == 0
					|| pItem->sCount == 0)
					continue;

				logitemid[i] = xitr->second.nItemID;
				logitemcount[i] = 1;

				pItem->sCount--;
				if (pItem->sCount == 0 && pItem->nNum == xitr->second.nItemID) {
					pItem->nExpirationTime = 0;
					memset(pItem, 0, sizeof(_ITEM_DATA));
				}
			}
		}

		if (bType != UpgradeTypePreview)
			UpgradeInsertLog(proto.Getnum(), nReqCoins, (uint8)pUserScroll, bResult, logitemid, logitemcount);
	}
	result << bType << bResult;
	if (bResult == UpgradeFailed && nUpgradeType != 3)
		result << logositemac;

	foreach(i, nItems)
		result << i->second.nItemID << i->second.bPos;
	Send(&result);

	// Send the result to everyone in the area
	// (i.e. make the anvil do its burned/upgraded indicator thing)
	if (bType != UpgradeTypePreview) {
		result.Initialize(WIZ_OBJECT_EVENT);
		result << uint8(OBJECT_ANVIL) << bResult << sNpcID;
		SendToRegion(&result, nullptr, GetEventRoom());
	}



	return;
fail_return:
	{
		std::string upgradeItemsLog;
		foreach(i, nItems)
			upgradeItemsLog += string_format("[%u:%d]", i->second.nItemID, i->second.bPos);

		g_pMain->ServerLog("ITEM_UPGRADE_FAIL",
			"user=%s zone=%u npc=%u type=%d result=%d originItem=%d newItem=%u scroll=%u userScroll=%d reqScrollType=%d itemClass=%d itemType=%d grade=%u genRate=%u reqCoins=%u coins=%lld items=%s",
			GetName().c_str(), GetZoneID(), sNpcID, bType, bResult,
			OriginItem != nItems.end() ? OriginItem->second.nItemID : 0, NewItemID, scroll_id,
			(int)pUserScroll, (int)m_ScrollType, proto.ItemClass, proto.m_ItemType, proto.m_byGrade,
			byGenRate, nReqCoins, GetCoins(), upgradeItemsLog.c_str());
	}
	if (bResult == UpgradeFailed)
		result << logositemac;
	result << bType << bResult;
	foreach(i, nItems)
		result << i->second.nItemID << i->second.bPos;
	Send(&result);
}

void CUser::ItemUpgradeNotice(_ITEM_TABLE pItem, uint8 UpgradeResult, uint32 scroll_id)
{
	if (pItem.isnull() || isGM() || !g_pMain->pServerSetting.UpgradeNotice)
		return;

	if (!pItem.m_isUpgradeNotice)
		return;

	bool is_blue = (scroll_id == 379222000 || scroll_id == 379224000 || scroll_id == 379223000 || scroll_id == 379225000 ||
		scroll_id == 379226000 || scroll_id == 379206000 || scroll_id == 379209000 || scroll_id == 379208000 ||
		scroll_id == 379210000 || scroll_id == 379211000 || scroll_id == 379017000 || scroll_id == 379027000 ||
		scroll_id == 379026000 || scroll_id == 379028000 || scroll_id == 379029000 || scroll_id == 379022000 ||
		scroll_id == 379031000 || scroll_id == 379030000 || scroll_id == 379032000 || scroll_id == 379033000);

	bool is_orange = (scroll_id == 379020000 || scroll_id == 379229000 || scroll_id == 379214000 || scroll_id == 379025000);
	bool is_upgrade = (scroll_id == 379205000 || scroll_id == 379016000 || scroll_id == 379221000 || scroll_id == 379021000);

	bool i_staff_trans = (
		scroll_id == 379230000 || scroll_id == 379231000 || scroll_id == 379232000 || scroll_id == 379233000 ||
		scroll_id == 379234000 || scroll_id == 379235000 || scroll_id == 379215000 || scroll_id == 379216000 ||
		scroll_id == 379217000 || scroll_id == 379218000 || scroll_id == 379219000 || scroll_id == 379220000 ||
		scroll_id == 379034000 || scroll_id == 379035000 || scroll_id == 379138000 || scroll_id == 379139000 ||
		scroll_id == 379140000 || scroll_id == 379141000);

	// JstKO Yeni Upgrade Mesaj� Olu�tur Eklendi. 03.05.2025
	std::string resultText = (UpgradeResult == 1 ? "Succeeded" : "Failed");
	std::string message = string_format("%s has %s to upgrade %s", GetName().c_str(), resultText.c_str(), pItem.m_sName.c_str());

	// JstKO Yeni Efekt Eklendi. 03.05.2025
	ShowEffect(UpgradeResult == 1 ? 490092 : false);

	// JstKO Yeni SendLogos LOGOSSHOUT G�nderilecek Eklendi 03.05.2025
	bool SendLogos = (is_upgrade || pItem.m_ItemType == 11 || pItem.m_ItemType == 12 || scroll_id == 379159000);

	if (SendLogos) // JstKO Yeni SendLogos Eklendi. 03.05.2025
	{
		Packet result(WIZ_LOGOSSHOUT, uint8(0x02));
		result.SByte();
		result << uint8(0x05) << uint8(UpgradeResult) << GetName() << pItem.m_iNum << GetLoyaltySymbolRank();
		g_pMain->Send_All(&result);
		return;
	}

	// Yeni Chat tipi belirleme
	uint8 chatType = 0;
	if (is_blue)
		chatType = (uint8)ChatType::ALLIANCE_NOTICE;
	else if (is_orange)
		chatType = (uint8)ChatType::SHOUT_CHAT;
	else if (i_staff_trans)
		chatType = (uint8)ChatType::MERCHANT_CHAT;
	else
		chatType = (uint8)ChatType::KNIGHTS_CHAT;

	if (!chatType)
		chatType = (uint8)ChatType::GENERAL_CHAT; // fallback Yeni

	// Mesaj g�nder
	Packet newpkt;
	ChatPacket::Construct(&newpkt, chatType, &message, nullptr, GetNation());
	g_pMain->SendGameNotice((ChatType)chatType, message, "", true, nullptr, false);
	return;
}
/**
* @brief	Packet handler for the accessory upgrade system.
*
* @param	pkt	The packet.
*/
void CUser::ItemUpgradeAccessories(Packet& pkt)
{
	ItemUpgrade(pkt, ITEM_ACCESSORIES);
}

/**
* @brief	Packet handler for the upgrading of 'rebirthed' items.
*
* @param	pkt	The packet.
*/
void CUser::ItemUpgradeRebirth(Packet& pkt)
{
	ItemUpgrade(pkt, ITEM_UPGRADE_REBIRTH);
}