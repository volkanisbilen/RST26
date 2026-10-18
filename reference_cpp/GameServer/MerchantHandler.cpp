#include "stdafx.h"
#include "../shared/DateTime.h"
using std::string;
enum MerchantOpenResponseCodes
{
	MERCHANT_OPEN_SUCCESS = 1,
	MERCHANT_OPEN_NO_SESSION = -1,
	MERCHANT_OPEN_DEAD = -2,
	MERCHANT_OPEN_TRADING = -3,
	MERCHANT_OPEN_MERCHANTING = -4,
	MERCHANT_OPEN_INVALID_ZONE = -5,
	MERCHANT_OPEN_SHOPPING = -6,
	MERCHANT_OPEN_UNDERLEVELED = 30
};

#define ITEM_MENICIAS_LIST 810166000

void CUser::MerchantProcess(Packet& pkt)
{
	if (!isInGame())
		return;

	if (pCindWar.isEventUser() && g_pMain->isCindirellaZone(GetZoneID()))
		return;

	uint8 opcode = pkt.read<uint8>();
	switch (opcode)
	{
		// Regular merchants
	case MERCHANT_OPEN:
		MerchantOpen();
		break;

	case MERCHANT_CLOSE:
		MerchantClose();
		break;

	case MERCHANT_ITEM_ADD:
#if (!HOOKACTIVE)
		MerchantItemAdd(pkt);
#endif
		break;
	case MERCHANT_ITEM_CANCEL:
		MerchantItemCancel(pkt);
		break;

	case MERCHANT_ITEM_LIST:  // 6805F9FF
		MerchantItemList(pkt);
		break;

	case MERCHANT_ITEM_BUY:
		MerchantItemBuy(pkt);
		break;

	case MERCHANT_INSERT:
		MerchantInsert(pkt);
		break;

	case MERCHANT_TRADE_CANCEL:
		CancelMerchant();
		break;

		// Buying merchants
	case MERCHANT_BUY_OPEN:
		BuyingMerchantOpen(pkt);
		break;

	case MERCHANT_BUY_CLOSE:
		BuyingMerchantClose();
		break;

	case MERCHANT_BUY_LIST:
		BuyingMerchantList(pkt);
		break;

	case MERCHANT_BUY_INSERT:
		BuyingMerchantInsert(pkt);
		break;

	case MERCHANT_BUY_BUY: // seeya!
		BuyingMerchantBuy(pkt);
		break;

	case MERCHANT_MENISIA_LIST:
		MerchantOfficialList(pkt);
		break;
	case MERCHANT_LIST:
		MerchantList(pkt);
		break;
	default:
		printf("Merchant unhandled packets %d \n", opcode);
		TRACE("Merchant unhandled packets %d \n", opcode);
		break;
	}
}

void CUser::MerchantOpen()
{
	if (isBuyingMerchantingPreparing())
		return;

	if (SlaveTotalTime > 0) {
		Packet Update(XSafe, uint8(XSafeOpCodes::PL_USER_TOOLS));
		Update << uint8(1) << uint8(4);
		Send(&Update);
	}

	int16 errorCode = 0;
	if (isDead())
		errorCode = MERCHANT_OPEN_DEAD;
	else if (isStoreOpen())
		errorCode = MERCHANT_OPEN_SHOPPING;
	else if (isTrading())
		errorCode = MERCHANT_OPEN_TRADING;
	else if (GetZoneID() != ZONE_MORADON)
		errorCode = MERCHANT_OPEN_INVALID_ZONE; // DeaFSezo maradon d��� pazar kurma engellendi
	else if (GetLevel() < g_pMain->pServerSetting.MerchantLevel)
		errorCode = MERCHANT_OPEN_UNDERLEVELED;
	else if (isMerchanting())
		errorCode = MERCHANT_OPEN_MERCHANTING;
	else
		errorCode = MERCHANT_OPEN_SUCCESS;

	if (isSlaveMerchant())
	{
		errorCode = MERCHANT_OPEN_MERCHANTING;
#if 0
		Packet result;
		std::string m_sAutosystem;

		m_sAutosystem = string_format("[Slave Merchant] Activated.");
		result.clear();
		result.Initialize(XSafeOpCodes::PL_INFOMESSAGE);
		result << m_sAutosystem;
		Send(&result);
#else
		SendBoxMessage(0, "", "[Slave Merchant] Activated.", 0, messagecolour::red);
#endif
	}

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_OPEN));
	result << errorCode;
	Send(&result);

	// If we're already merchanting, user may be desynced
	// so we need to close our current merchant first.
	if (errorCode == MERCHANT_OPEN_MERCHANTING)
	{
		if (SlaveTotalTime > 0)
			MerchantSlaveClose();
		else
			MerchantClose();
	}

	if (errorCode == MERCHANT_OPEN_SUCCESS)
		m_bSellingMerchantPreparing = true;
}


void CUser::MerchantClose()
{
	bool checking = isSellingMerchantingPreparing() || isSellingMerchant();
	if (!checking)
		return;

	bool prepare = m_bSellingMerchantPreparing;

	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		auto* pItem = GetItem(i);
		if (pItem && pItem->MerchItem) pItem->MerchItem = false;
	}

	level_merchant_time = 0;
	plookersocketid = -1;
	m_bSellingMerchantPreparing = false;
	m_bMerchantState = MERCHANT_STATE_NONE;
	memset(m_arMerchantItems, 0, sizeof(m_arMerchantItems));

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_CLOSE));
	result << GetSocketID();

	if (!prepare)
	{
		SendToRegion(&result, nullptr, GetEventRoom());
		SetOffCha(_choffstatus::DEACTIVE, offcharactertype::merchant);
	}
	else Send(&result);
}

void CUser::MerchantItemAdd(Packet& pkt)
{
	if (!isSellingMerchantingPreparing() || isBuyingMerchantingPreparing())
		return;

	_ITEM_TABLE pTable = _ITEM_TABLE();
	Packet result(WIZ_MERCHANT, uint8(MERCHANT_ITEM_ADD));
	uint32 nGold, nItemID;
	uint16 sCount;
	uint8 bSrcPos, bDstPos, bMode, isKC;

	pkt >> nItemID >> sCount >> nGold >> bSrcPos >> bDstPos >> bMode >> isKC;
	// TODO: Implement the possible error codes for these various error cases.
	if (!nItemID || !sCount || bSrcPos >= HAVE_MAX || bDstPos >= MAX_MERCH_ITEMS) goto fail_return;

	for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		if (m_arMerchantItems[i].nNum && m_arMerchantItems[i].bOriginalSlot == bSrcPos)
			goto fail_return;

	_ITEM_DATA* pSrcItem = GetItem(bSrcPos + SLOT_MAX);
	pTable = g_pMain->GetItemPtr(nItemID);
	if (pTable.isnull() || !pSrcItem)
		goto fail_return;

	if (nItemID >= ITEM_NO_TRADE_MIN && nItemID <= ITEM_NO_TRADE_MAX // Cannot be traded, sold or stored.
		|| pTable.m_bRace == RACE_UNTRADEABLE // Cannot be traded or sold.
		|| pTable.m_bCountable == 2
		|| (pTable.m_bKind == 255 && sCount != 1)) // Cannot be traded or sold.
		goto fail_return;

	if (pTable.m_bCountable == 1 && sCount > MAX_ITEM_COUNT) goto fail_return;

	if (pSrcItem->nNum != nItemID
		|| pSrcItem->sCount == 0
		|| sCount == 0
		|| pSrcItem->sCount < sCount
		|| pSrcItem->isRented()
		|| pSrcItem->isSealed()
		|| pSrcItem->isBound()
		|| pSrcItem->isDuplicate()
		|| pSrcItem->isExpirationTime())
		goto fail_return;

	_MERCH_DATA* pMerch = &m_arMerchantItems[bDstPos];
	if (pMerch == nullptr || pMerch->nNum != 0)
		goto fail_return;

	pSrcItem->MerchItem = true;

	pMerch->nNum = nItemID;
	pMerch->nPrice = nGold;
	pMerch->sCount = sCount; // Selling Count
	pMerch->orgcount = pSrcItem->sCount;

	pMerch->sDuration = pSrcItem->sDuration;
	pMerch->nSerialNum = pSrcItem->nSerialNum; // NOTE: Stackable items will have an issue with this.
	pMerch->bOriginalSlot = bSrcPos;
	pMerch->IsSoldOut = false;
	pMerch->isKC = isKC;

	if (pTable.m_bKind == 255 && !pTable.m_bCountable) pMerch->sCount = 1;

	result << uint16(1) << nItemID << sCount << pMerch->sDuration << nGold << bSrcPos << bDstPos << isKC;
	Send(&result);
	return;

fail_return:
	result << uint16(0) << nItemID << sCount << (uint16)bSrcPos + bDstPos << nGold << bSrcPos << bDstPos << isKC;
	Send(&result);
}

void CUser::MerchantItemCancel(Packet& pkt)
{
	if (!isSellingMerchantingPreparing() || isBuyingMerchantingPreparing())
		return;

	uint8 bSrcPos = pkt.read<uint8>();

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_ITEM_CANCEL));

	if (bSrcPos >= MAX_MERCH_ITEMS)
	{
		result << int16(-2);
		Send(&result);
		return;
	}

	_MERCH_DATA* pMerch = &m_arMerchantItems[bSrcPos];
	if (pMerch == nullptr || !pMerch->nNum
		|| !pMerch->sCount || pMerch->sCount > ITEMCOUNT_MAX
		|| pMerch->bOriginalSlot >= HAVE_MAX)
	{
		result << int16(-3);
		Send(&result);
		return;
	}

	_ITEM_DATA* pItem = GetItem(pMerch->bOriginalSlot);
	if (pItem == nullptr || pItem->nNum != pMerch->nNum
		|| pItem->sCount != pMerch->orgcount
		|| !pItem->isMerchantItem())
	{
		result << int16(-3);
		Send(&result);
		return;
	}

	pItem->MerchItem = false;
	memset(pMerch, 0, sizeof(_MERCH_DATA));
	result << int16(1) << bSrcPos;
	Send(&result);
}

void CUser::MerchantInsert(Packet& pkt)
{
	if (!isSellingMerchantingPreparing() || isBuyingMerchantingPreparing())
		return;

	uint16 bResult = 0;
	bResult = 1;
	_MERCH_DATA	m_arNewItems[MAX_MERCH_ITEMS]{}; //What is this person selling? Stored in "_MERCH_DATA" structure.

	DateTime time;
	string advertMessage; // check here maybe to make sure they're not using it otherwise?
	pkt >> advertMessage;
	if (advertMessage.size() > MAX_MERCH_MESSAGE)
		return;

#pragma region SlaveMerchant
	if (SlaveTotalTime > 0 && isSlave == 1)
	{

		CBot* pSlaveUser = nullptr;
		uint16 m_bSlaveUsers = g_pMain->SpawnSlaveUserBot(86400, this);
		if (m_bSlaveUsers > 0)
		{
			pSlaveUser = g_pMain->m_MapBotList.GetData(m_bSlaveUsers);
			if (pSlaveUser == nullptr)
				return;

			pSlaveUser->m_bPremiumMerchant = true;
			Packet result(WIZ_MERCHANT, uint8(MERCHANT_INSERT));
			result << uint16(1)
				<< advertMessage
				<< pSlaveUser->GetID()
				<< pSlaveUser->m_bPremiumMerchant;

			std::string asdasd = advertMessage;

			if (!asdasd.empty())
				pSlaveUser->MerchantChat = string_format("%s(Location:%d,%d)", asdasd.c_str(), pSlaveUser->GetSPosX() / 10, pSlaveUser->GetSPosZ() / 10);
			else
				pSlaveUser->MerchantChat.clear();

			uint16 totalMerchItems = 0;
			for (int i = 0; i < MAX_MERCH_ITEMS; i++)
			{
				pSlaveUser->m_arMerchantItems[i] = m_arMerchantItems[i];

				SlaveItemID[i] = pSlaveUser->m_arMerchantItems[i].nNum;
				SlaveItemCount[i] = pSlaveUser->m_arMerchantItems[i].sCount;

				if (pSlaveUser->m_arMerchantItems[i].nNum != 0 &&
					(pSlaveUser->m_arMerchantItems[i].orgcount == 0
						|| pSlaveUser->m_arMerchantItems[i].orgcount < m_arMerchantItems[i].sCount))
					return;

				result << pSlaveUser->m_arMerchantItems[i].nNum;



				if (pSlaveUser->m_arMerchantItems[i].nNum > 0)
				{
					totalMerchItems++;
					//g_pMain->WriteMerchantLogFile(string_format("MERCHANT SLAVE SELLING-ITEMS - %d:%d:%d || Seller=%s,ItemID=%d,Count=%d/%d,Price=%d,Zone=%d,X=%d,Z=%d\n", time.GetHour(), time.GetMinute(), time.GetSecond(), pSlaveUser->GetName().c_str(), pSlaveUser->m_arMerchantItems[i].nNum, pSlaveUser->m_arMerchantItems[i].bCount, pSlaveUser->m_arMerchantItems[i].sCount, pSlaveUser->m_arMerchantItems[i].nPrice, pSlaveUser->GetZoneID(), uint16(pSlaveUser->GetX()), uint16(pSlaveUser->GetZ())));
				}
			}

			for (int i = 0; i < MAX_MERCH_ITEMS; i++)
			{
				auto pTable = g_pMain->GetItemPtr(pSlaveUser->m_arMerchantItems[i].nNum);
				if (pTable.isnull())
					continue;

				RobItem(pSlaveUser->m_arMerchantItems[i].bOriginalSlot, pTable, pSlaveUser->m_arMerchantItems[i].sCount, true, true);
			}

			if (totalMerchItems == 0)
			{
				if (pSlaveUser->isSellingMerchant())
				{
					if (isSlaveMerchant()) {
						m_bSlaveMerchant = false;
						m_bSlaveUserID = -1;
					}
					Packet result(WIZ_MERCHANT, uint8(MERCHANT_CLOSE));
					result << pSlaveUser->GetID();
					pSlaveUser->SendToRegion(&result);
					//g_pMain->WriteCheatLogFile(string_format("Merchant Slave hack - %d:%d:%d || %s is disconnected.\n", time.GetHour(), time.GetMinute(), time.GetSecond(), pSlaveUser->GetName().c_str()));
					pSlaveUser->UserInOut(INOUT_OUT);
					return;
				}

				Packet result(WIZ_MERCHANT, uint8(MERCHANT_CLOSE));
				result << pSlaveUser->GetID();
				pSlaveUser->SendToRegion(&result);

				//g_pMain->WriteCheatLogFile(string_format("Merchant Slave hack - %d:%d:%d || %s is disconnected.\n", time.GetHour(), time.GetMinute(), time.GetSecond(), pSlaveUser->GetName().c_str()));
				return;
			}
			pSlaveUser->m_bMerchantState = MERCHANT_STATE_SELLING;
			pSlaveUser->SendToRegion(&result);
		}
		MerchantSlaveClose();
		return;
	}
#pragma endregion

	if (isGM())
	{
		Packet result(WIZ_MERCHANT, uint8(MERCHANT_INSERT));

		_MERCH_DATA copy_m_arMerchantItems[MAX_MERCH_ITEMS];
		memcpy(copy_m_arMerchantItems, &m_arMerchantItems, sizeof(m_arMerchantItems));

		plookersocketid = -1;
		m_bSellingMerchantPreparing = false;
		m_bMerchantState = MERCHANT_STATE_NONE;
		memset(m_arMerchantItems, 0, sizeof(m_arMerchantItems));

		for (int i = 0; i < INVENTORY_TOTAL; i++)
		{
			auto* pItem = GetItem(i);
			if (pItem && pItem->MerchItem) 
				pItem->MerchItem = false;
		}

		uint8 nRandom = 2;
		float fX = GetX();
		float fZ = GetZ();

		fX = fX + myrand(0, nRandom);
		fZ = fZ + myrand(0, nRandom);

		CBot* myBot;
		uint16 ID = g_pMain->SpawnEventBotMerchant(3600, GetZoneID(), GetX(), GetY(), GetZ(), NULL, MIN_LEVEL_ARDREAM);
		if (ID)
		{
			myBot = g_pMain->GetBotPtr(ID);
			if (myBot == nullptr)
				return;

			if (myBot->GetRegion() == nullptr)
				return;

			std::string asdasd = advertMessage;

			if (!asdasd.empty())
				myBot->MerchantChat = string_format("%s(Location:%d,%d)", asdasd.c_str(), myBot->GetSPosX() / 10, myBot->GetSPosZ() / 10);
			else
				myBot->MerchantChat.clear();

			myBot->m_iLoyalty = myrand(100, 5000);

			memset(myBot->m_arMerchantItems, 0, sizeof(myBot->m_arMerchantItems));

			result << bResult << advertMessage << myBot->GetID()
				<< m_bPremiumMerchant;

			uint8 nreqcount = 0;
			for (int i = 0; i < MAX_MERCH_ITEMS; i++)
			{
				myBot->m_arMerchantItems[i] = copy_m_arMerchantItems[i];

				if (myBot->m_arMerchantItems[i].nNum != 0 &&
					(myBot->m_arMerchantItems[i].sCount == 0
						|| myBot->m_arMerchantItems[i].sCount < copy_m_arMerchantItems[i].sCount))
					return;

				result << myBot->m_arMerchantItems[i].nNum;

				if (myBot->m_arMerchantItems[i].nNum > 0)
					nreqcount++;
			}

			if (!nreqcount)
				return;

			myBot->m_bPremiumMerchant = m_bPremiumMerchant;
			myBot->m_bMerchantState = MERCHANT_STATE_SELLING;
			//myBot->m_bSellingMerchantPreparing = false;

			//myBot->SendMerchantChat();
		}

		if (myBot && ID)
		{
			//myBot->MerchantSlipRefList(myBot,true);
			myBot->SendToRegion(&result);
		}

		result.clear();
		result << uint8(MERCHANT_CLOSE) << GetSocketID();
		Send(&result);
		return;
	}

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_INSERT));
	result << uint16(0x01) << advertMessage << GetSocketID() << m_bPremiumMerchant;

	uint16 totalMerchItems = 0;
	for (int i = 0; i < MAX_MERCH_ITEMS; i++)
	{
		result << m_arMerchantItems[i].nNum;
		if (m_arMerchantItems[i].nNum > 0)  totalMerchItems++;
	}

	if (!totalMerchItems || totalMerchItems > 12)
		return goDisconnect(string_format("There is a problem in the number of parts put on the market. itemcount=%d", totalMerchItems), __FUNCTION__);

	if (g_pMain->pLevMercInfo.status)
		level_merchant_time = UNIXTIME + (g_pMain->pLevMercInfo.rewardtime * MINUTE);

	m_bMerchantState = MERCHANT_STATE_SELLING;
	m_bSellingMerchantPreparing = false;
	SendToRegion(&result, nullptr, GetEventRoom());
	SetOffCha(_choffstatus::ACTIVE, offcharactertype::merchant);
	MerchantCreationInsertLog(1);
	MerchantSlipRefList(this, true);
}

void CUser::MerchantItemList(Packet& pkt)
{
	uint16 uid = pkt.read<uint16>();
	CBot* pUserBot = g_pMain->GetBotPtr(uid);

	if (pUserBot != nullptr)
	{
		if (!pUserBot->isMerchanting())
			return;

		if (pUserBot->m_bMerchantViewer >= 0)
		{
			auto* pLooker = g_pMain->GetUserPtr(pUserBot->m_bMerchantViewer);
			if (pLooker && pLooker->isInGame())
			{
				Packet newpkt(WIZ_MERCHANT, uint8(MERCHANT_ITEM_LIST));
				newpkt.SByte();
				newpkt << int16(-7) << pLooker->GetName();
				Send(&newpkt);
				return;
			}
			else pUserBot->m_bMerchantViewer = -1;
		}

		pUserBot->m_bMerchantViewer = GetID();
		m_sMerchantsSocketID = uid;

		Packet result(WIZ_MERCHANT, uint8(MERCHANT_ITEM_LIST));
		result << uint16(0x01) << uint16(uid);
		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		{
			_MERCH_DATA* pMerch = &pUserBot->m_arMerchantItems[i];
			if (pMerch == nullptr)
				continue;

			result << pMerch->nNum << pMerch->sCount << pMerch->sDuration << pMerch->nPrice;

			if (pMerch->nNum == ITEM_CYPHER_RING)
				ShowCyperRingItemInfo(result, pMerch->nSerialNum);
			else
				result << uint32(0); // Item Unique ID
		}

		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		{
			_MERCH_DATA* pMerch = &pUserBot->m_arMerchantItems[i];
			uint8 isKC = pMerch->isKC ? 1 : 0;
			result << isKC;
		}

		Send(&result);
		return;
	}

	CUser* pUserMerchant = g_pMain->GetUserPtr(uid);
	if (!pUserMerchant)
		return;

	if (!isInRange(pUserMerchant, MAX_NPC_RANGE)
		|| !pUserMerchant->isMerchanting()
		|| pUserMerchant->isSellingMerchantingPreparing()
		|| pUserMerchant->isBuyingMerchantingPreparing())
		return;

	if (pUserMerchant->plookersocketid >= 0)
	{
		CUser* pLooker = g_pMain->GetUserPtr(pUserMerchant->plookersocketid);
		if (pLooker && pLooker->isInGame())
		{
			Packet newpkt(WIZ_MERCHANT, uint8(MERCHANT_ITEM_LIST));
			newpkt.SByte();
			newpkt << int16(-7) << pLooker->GetName();
			Send(&newpkt);
			return;
		}
		else pUserMerchant->plookersocketid = -1;
	}

	pUserMerchant->plookersocketid = GetID();
	m_sMerchantsSocketID = uid;

	if (g_pMain->pServerSetting.MerchantView)
	{
		if (pUserMerchant != nullptr)
		{
			DateTime timemachine;
			Packet merchanwiew(XSafe);
			merchanwiew << uint8(XSafeOpCodes::MERC_WIEWER_INFO) << uint8(1);
			merchanwiew.SByte();
			merchanwiew << GetName() << timemachine.GetHour() << timemachine.GetMinute();
			pUserMerchant->Send(&merchanwiew);
		}
	}
	Packet result(WIZ_MERCHANT, uint8(MERCHANT_ITEM_LIST));
	result << uint16(1) << uint16(uid);
	for (int i = 0; i < MAX_MERCH_ITEMS; i++)
	{
		_MERCH_DATA* pMerch = &pUserMerchant->m_arMerchantItems[i];
		if (pMerch == nullptr)
			continue;

		result << pMerch->nNum
			<< pMerch->sCount
			<< pMerch->sDuration
			<< pMerch->nPrice;

		_ITEM_TABLE pItemTable = g_pMain->GetItemPtr(pMerch->nNum);
		if (!pItemTable.isnull())
		{
			if (pItemTable.isPetItem())
				ShowPetItemInfo(result, pMerch->nSerialNum);
			else if (pItemTable.Getnum() == ITEM_CYPHER_RING)
				ShowCyperRingItemInfo(result, pMerch->nSerialNum);
			else
				result << uint32(0); // Item Unique ID
		}
		else
			result << uint32(0); // Item Unique ID*/
	}

	for (int i = 0; i < MAX_MERCH_ITEMS; i++)
	{
		_MERCH_DATA* pMerch = &pUserMerchant->m_arMerchantItems[i];
		if (!pMerch) continue;
		uint8 isKC = pMerch->isKC ? 1 : 0;
		result << isKC;
	}

	Send(&result);
}

void CUser::MerchantItemBuy(Packet& pkt)
{
	uint32 itemid, req_gold;
	uint16 item_count, leftover_count;
	uint8 item_slot, dest_slot;
	Packet result(WIZ_MERCHANT);

	if (m_sMerchantsSocketID < MAX_USER)
	{
		CUser* pMerchUser = g_pMain->GetUserPtr(m_sMerchantsSocketID);

		if (!pMerchUser)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		if (pMerchUser->GetSocketID() == GetSocketID() || pMerchUser->GetAccountName() == GetAccountName())
			return goDisconnect("trying to buy parts from its own market.", __FUNCTION__);

		if (!isInGame() || isDead() || isMining() || isFishing()
			/*|| isStoreOpen()*/
			|| isMerchanting() || isSellingMerchantingPreparing()
			|| isBuyingMerchant() || m_sMerchantsSocketID < 0
			|| m_sMerchantsSocketID > MAX_USER || isTrading())
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		if (!isInRange(pMerchUser, 35.0f) || !pMerchUser->isMerchanting() || pMerchUser->isSellingMerchantingPreparing()
			|| pMerchUser->GetMerchantState() != MERCHANT_STATE_SELLING)
		{

			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		pkt >> itemid >> item_count >> item_slot >> dest_slot;

		if (item_slot >= MAX_MERCH_ITEMS || dest_slot >= HAVE_MAX || !item_count)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		// Grab pointers to the items.
		_MERCH_DATA* pMerch = &pMerchUser->m_arMerchantItems[item_slot];
		if (pMerch == nullptr || pMerch->IsSoldOut || !pMerch->sCount || !pMerch->nPrice)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		_ITEM_DATA* pBuyerItem = GetItem(SLOT_MAX + dest_slot);
		if (pBuyerItem == nullptr)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		if (pMerch->nNum != itemid || pMerch->sCount < item_count || pMerch->orgcount < item_count)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		_ITEM_TABLE pSellingItem = g_pMain->GetItemPtr(itemid);
		if (pSellingItem.isnull() || (!pSellingItem.m_bCountable && item_count != 1))
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;

		}

		if (pSellingItem.m_bKind == 255 && item_count != 1 && !pSellingItem.m_bCountable)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		uint32 nReqWeight = pSellingItem.m_sWeight * item_count;
		if ((nReqWeight + m_sItemWeight > m_sMaxWeight))
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		if (pBuyerItem->nNum != 0 && (pBuyerItem->nNum != itemid || !pSellingItem.m_bCountable))
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		_ITEM_DATA* pSellerItem = pMerchUser->GetItem(pMerch->bOriginalSlot);
		if (pSellerItem == nullptr || pSellerItem->nNum != pMerch->nNum
			|| !pSellerItem->isMerchantItem() || pSellerItem->sCount < item_count
			|| !pSellerItem->sCount)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		int8 PosXZ = FindSlotForItem(pMerch->nNum, pMerch->sCount);
		if (PosXZ < 0)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		// Do we have enough coins?
		req_gold = pMerch->nPrice * item_count;
		if (pMerch->isKC)
		{
			if (m_nKnightCash < req_gold)
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}
		}
		else
		{
			if (m_iGold < req_gold)
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}
		}

		if (pMerchUser && pMerchUser->m_iGold + req_gold > COIN_MAX)
		{
			result << uint8(6) << uint16(-18);
			Send(&result);
			return;
		}

		if (pMerch->isKC)
		{
			if (!CashLose(req_gold))
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}
			if (pMerchUser) pMerchUser->CashGain(req_gold);

			// JstKO Yeni Pazarlar Para&KC List �zleme Pm Notice Eklendi. 25.04.2025
			std::string PMName = "Pazarlar Coins&KC List";
			std::string Mesaj = "----------------[Pazar KC]-------------------";
			Packet result;
			ChatPacket::Construct(&result, ChatType::PRIVATE_CHAT, &Mesaj, &PMName);
			g_pMain->Send_All(&result); // JstKO Pm Notice Herkes G�r�yoruz. Eklendi. 25.04.2025

			ShowEffect(490092); // JstKO Y�lba�i Ya�iyor Effect Eklendi. 25.04.2025
			{
				if (item_count > 1) // �tem KC Adet Eklendi.
					Mesaj = string_format("%s -> %s [%s x%d] KC : [-%d] Ald�n�z Tebrikler!", GetName().c_str(), pMerchUser->GetName().c_str(), pSellingItem.m_sName.c_str(), item_count, req_gold);
				else
					Mesaj = string_format("%s -> %s [%s] KC : [-%d] Ald�n�z Tebrikler!", GetName().c_str(), pMerchUser->GetName().c_str(), pSellingItem.m_sName.c_str(), req_gold);
			}

			ChatPacket::Construct(&result, ChatType::PRIVATE_CHAT, &Mesaj, &PMName);
			g_pMain->Send_All(&result); // JstKO Pm Notice Herkes G�r�yoruz. Eklendi. 25.04.2025
		}

		else
		{
			if (!GoldLose(req_gold))
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}
			if (pMerchUser) pMerchUser->GoldGain(req_gold);

			// JstKO Yeni Pazarlar Para&KC List �zleme Pm Notice Eklendi. 25.04.2025
			std::string PMName = "Pazarlar Coins&KC List";
			std::string Mesaj = "----------------[Pazar Coins]----------------";
			Packet result;
			ChatPacket::Construct(&result, ChatType::PRIVATE_CHAT, &Mesaj, &PMName);
			g_pMain->Send_All(&result); // JstKO Pm Notice Herkes G�r�yoruz. Eklendi. 25.04.2025

			ShowEffect(490092); // JstKO Y�lba�i Ya�iyor Effect Eklendi. 25.04.2025
			{
				if (item_count > 1) // �tem Coins Adet Eklendi.
					Mesaj = string_format("%s -> %s [%s x%d] Coins : [-%d] Ald�n�z Tebrikler!", GetName().c_str(), pMerchUser->GetName().c_str(), pSellingItem.m_sName.c_str(), item_count, req_gold);
				else
					Mesaj = string_format("%s -> %s [%s] Coins : [-%d] Ald�n�z Tebrikler!", GetName().c_str(), pMerchUser->GetName().c_str(), pSellingItem.m_sName.c_str(), req_gold);
			}

			ChatPacket::Construct(&result, ChatType::PRIVATE_CHAT, &Mesaj, &PMName);
			g_pMain->Send_All(&result); // JstKO Pm Notice Herkes G�r�yoruz. Eklendi. 25.04.2025
		}

		if (!pMerch->isKC)
			pMerchUser->pUserDailyRank.GMTotalSold += req_gold;

		leftover_count = pMerch->sCount - item_count;
		pBuyerItem->sCount += item_count;

		SendStackChange(itemid, pBuyerItem->sCount, pBuyerItem->sDuration, dest_slot, (pBuyerItem->sCount == item_count)); // is it a new item?

		pMerch->sCount -= item_count;
		pSellerItem->sCount -= item_count;

		uint64 serial = pSellerItem->nSerialNum;
		if (!serial) serial = g_pMain->GenerateItemSerial();

		if (pSellingItem.isStackable())
		{
			if (!pSellerItem->sCount && !pBuyerItem->nNum)
				pBuyerItem->nSerialNum = serial;
			else if (!pBuyerItem->nNum)
				pBuyerItem->nSerialNum = g_pMain->GenerateItemSerial();
		}
		else pBuyerItem->nSerialNum = serial;

		pBuyerItem->nNum = pMerch->nNum;
		pBuyerItem->sDuration = pMerch->sDuration;

		if (!pSellingItem.isStackable() || item_count == pMerch->sCount) pBuyerItem->nSerialNum = pSellerItem->nSerialNum;
		if (!pSellingItem.isStackable() && pBuyerItem->nSerialNum == 0) pBuyerItem->nSerialNum = g_pMain->GenerateItemSerial();
		pBuyerItem->MerchItem = false;

		MerchantShoppingDetailInsertLog(false, 1, itemid, item_count, pMerch->nPrice, pMerchUser);

		if (!pSellerItem->sCount || (pSellingItem.m_bKind == 255 && pSellingItem.m_bCountable == 0))  memset(pSellerItem, 0, sizeof(_ITEM_DATA));
		pMerchUser->SendStackChange(pSellerItem->nNum, pSellerItem->sCount, pSellerItem->sDuration, pMerch->bOriginalSlot - SLOT_MAX);
		if (!pMerch->sCount || (pSellingItem.m_bCountable == 0 && pSellingItem.m_bKind == 255)) { memset(pMerch, 0, sizeof(_MERCH_DATA));  pMerch->IsSoldOut = true; }

		result.clear();
		result.Initialize(WIZ_MERCHANT);
		result << uint8(MERCHANT_ITEM_PURCHASED) << itemid << GetName();
		pMerchUser->Send(&result);

		result.clear();
		result.Initialize(WIZ_MERCHANT);
		result << uint8(MERCHANT_ITEM_BUY) << uint16(1)
			<< itemid << leftover_count
			<< item_slot << dest_slot;
		Send(&result);

		if (item_slot < 4 && leftover_count == 0)
		{
			result.clear();
			result.Initialize(WIZ_MERCHANT_INOUT);
			result << uint8(2) << m_sMerchantsSocketID << uint8(1) << uint8(0) << item_slot;
			pMerchUser->SendToRegion(&result, nullptr, GetEventRoom());
		}

		int nItemsRemaining = 0;
		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		{
			if (pMerchUser->m_arMerchantItems[i].nNum != 0 && !pMerchUser->m_arMerchantItems[i].IsSoldOut)
				nItemsRemaining++;
		}

		if (nItemsRemaining == 0 && pMerchUser) pMerchUser->MerchantClose();
		if (nItemsRemaining > 0) MerchantSlipRefList(pMerchUser);
	}
	else
	{
		CBot* pUserBot = g_pMain->GetBotPtr(m_sMerchantsSocketID);
		if (pUserBot != nullptr)
		{
			if (pUserBot->GetMerchantState() != MERCHANT_STATE_SELLING)
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}

			pkt >> itemid >> item_count >> item_slot >> dest_slot;

			// Make sure the slots are correct and that we're actually buying at least 1 item.
			if (item_slot >= MAX_MERCH_ITEMS || dest_slot >= HAVE_MAX || !item_count)
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}

			// Grab pointers to the items.
			_MERCH_DATA* pMerch = &pUserBot->m_arMerchantItems[item_slot];
			if (pMerch == nullptr || pMerch->IsSoldOut || !pMerch->sCount || !pMerch->nPrice)
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}

			_ITEM_DATA* pBuyerItem = GetItem(SLOT_MAX + dest_slot);
			if (pBuyerItem == nullptr)
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}

			if (pMerch->nNum != itemid || pMerch->sCount < item_count || pMerch->orgcount < item_count)
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}

			auto pSellingItem = g_pMain->GetItemPtr(itemid);
			if (pSellingItem.isnull() || pSellingItem.m_bCountable == 2 || (!pSellingItem.m_bCountable && item_count != 1))
			{
				result << uint16(-18);
				Send(&result);
				return;
			}

			if (pSellingItem.m_bKind == 255 && item_count != 1 && !pSellingItem.m_bCountable)
			{
				result << uint16(-18);
				Send(&result);
				return;
			}

			uint32 nReqWeight = pSellingItem.m_sWeight * item_count;
			if (nReqWeight + m_sItemWeight > m_sMaxWeight)
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}

			req_gold = pMerch->nPrice * item_count;

			if (pMerch->isKC)
			{
				if (m_nKnightCash < req_gold)
				{
					result << uint8(6) << uint16(-18);
					Send(&result);
					return;
				}
			}
			else
			{
				if (m_iGold < req_gold)
				{
					result << uint8(6) << uint16(-18);
					Send(&result);
					return;
				}
			}

			if (pBuyerItem->nNum != 0 && (pBuyerItem->nNum != itemid || !pSellingItem.m_bCountable))
			{
				result << uint8(6) << uint16(-18);
				Send(&result);
				return;
			}

			if (!pMerch->isKC && pUserBot->m_iGold + req_gold > COIN_MAX)
				return;

			if (pMerch->isKC)
			{
				if (pUserBot->isSlaveMerchant())
				{
					if (!CashLose(req_gold)) {
						result << uint8(6) << uint16(-18);
						Send(&result);
						return;
					}

					CUser* pSlaveUser = g_pMain->GetUserPtr(pUserBot->m_bSlaveUserID);

					if (pSlaveUser != nullptr)
					{

						for (int i = 0; i < 12; i++) {
							if (pSlaveUser->SlaveItemID[i] == itemid)
							{
								pSlaveUser->SlaveItemID[i] = 0;
								pSlaveUser->SlaveItemCount[i] -= item_count;

							}
						}


						std::string m_sAutosystem, gold, gold2, total;
						m_sAutosystem = string_format("[Slave Merchant] You Received ");
						gold = std::to_string(req_gold);
						gold2 = " cash.";
						total = m_sAutosystem + gold + gold2;
#if 0
						Packet result(XSafeOpCodes::PL_INFOMESSAGE);
						result << total;
						pSlaveUser->Send(&result);
#else
						pSlaveUser->SendBoxMessage(0, "", "[Slave Merchant] You Received ", 0, messagecolour::red);
#endif					

						g_DBAgent.UpdateSlaveKC(pSlaveUser->GetName(), req_gold);

						for (int i = 0; i < 12; i++)
							SlaveItemID[i] = 0;
						for (int i = 0; i < 12; i++)
							SlaveItemCount[i] = 0;

					}
				}
				else
				{
					if (!CashLose(req_gold)) {
						result << uint8(6) << uint16(-18);
						Send(&result);
						return;
					}
				}
			}
			else
			{
				if (pUserBot->isSlaveMerchant())
				{
					if (!GoldLose(req_gold))
					{
						result << uint8(6) << uint16(-18);
						Send(&result);
						return;
					}

					CUser* pSlaveUser = g_pMain->GetUserPtr(pUserBot->m_bSlaveUserID);

					if (pSlaveUser != nullptr)
					{

						for (int i = 0; i < 12; i++) {
							if (pSlaveUser->SlaveItemID[i] == itemid)
							{
								pSlaveUser->SlaveItemID[i] = 0;
								pSlaveUser->SlaveItemCount[i] -= item_count;

							}
						}


						if (pSlaveUser->m_iGold + req_gold > COIN_MAX)
						{
							result << uint8(6) << uint16(-18);
							Send(&result);
							return;
						}
						std::string m_sAutosystem, gold, gold2, total;
						m_sAutosystem = string_format("[Slave Merchant] You Received ");
						gold = std::to_string(req_gold);
						gold2 = " gold.";
						total = m_sAutosystem + gold + gold2;
#if 0
						Packet result(XSafeOpCodes::PL_INFOMESSAGE);
						result << total;
						pSlaveUser->Send(&result);
#else
						pSlaveUser->SendBoxMessage(0, "", "[Slave Merchant] You Received ", 0, messagecolour::red);
#endif
						g_DBAgent.UpdateSlaveCoins(pSlaveUser->GetName(), req_gold);

						for (int i = 0; i < 12; i++)
							SlaveItemID[i] = 0;
						for (int i = 0; i < 12; i++)
							SlaveItemCount[i] = 0;
					}
				}
				else
				{
					if (!GoldLose(req_gold)) {
						result << uint8(6) << uint16(-18);
						Send(&result);
						return;
					}

					if (pUserBot->m_iGold + req_gold > COIN_MAX)
						pUserBot->m_iGold = COIN_MAX;
					else
						pUserBot->m_iGold += req_gold;
				}
			}

			leftover_count = pMerch->sCount - item_count;
			pBuyerItem->sCount += item_count;
			pMerch->sCount -= item_count;
			pBuyerItem->nSerialNum = g_pMain->GenerateItemSerial();
			pBuyerItem->nNum = pMerch->nNum;
			pBuyerItem->sDuration = pMerch->sDuration;

			if (!pSellingItem.isStackable() || item_count == pMerch->sCount) pBuyerItem->nSerialNum = g_pMain->GenerateItemSerial();
			if (!pSellingItem.isStackable() && pBuyerItem->nSerialNum == 0) pBuyerItem->nSerialNum = g_pMain->GenerateItemSerial();
			pBuyerItem->MerchItem = false;

			SendStackChange(itemid, pBuyerItem->sCount, pBuyerItem->sDuration, dest_slot, (pBuyerItem->sCount == item_count)); // is it a new item?
			MerchantShoppingDetailInsertLog(true, 1, itemid, item_count, pMerch->nPrice, nullptr);

			if (!pMerch->sCount || (pSellingItem.m_bCountable == 0 && pSellingItem.m_bKind == 255))
			{ 
				memset(pMerch, 0, sizeof(_MERCH_DATA)); 
				pMerch->IsSoldOut = true;
			}

			result.clear();
			result.Initialize(WIZ_MERCHANT);
			result << uint8(MERCHANT_ITEM_PURCHASED) << itemid << GetName();

			if (pUserBot->isSlaveMerchant())
			{
				CUser* pSlaveUser = g_pMain->GetUserPtr(pUserBot->GetSlaveGetID());
				if (pSlaveUser != nullptr)
				{
					if (pSlaveUser->isSlaveMerchant())
						pSlaveUser->Send(&result);
				}
			}

			result.clear();
			result.Initialize(WIZ_MERCHANT);
			result << uint8(MERCHANT_ITEM_BUY) << uint16(1) << itemid << leftover_count << item_slot << dest_slot;
			Send(&result);

			if (item_slot < 4 && leftover_count == 0)
			{
				result.clear();
				result.Initialize(WIZ_MERCHANT_INOUT);
				result << uint8(2) << m_sMerchantsSocketID << uint8(1) << uint8(0) << item_slot;
				pUserBot->SendToRegion(&result);
			}

			int nItemsRemaining = 0;
			for (int i = 0; i < MAX_MERCH_ITEMS; i++)
			{
				if (pUserBot->m_arMerchantItems[i].nNum != 0 && !pUserBot->m_arMerchantItems[i].IsSoldOut)
					nItemsRemaining++;
			}

			if (nItemsRemaining == 0)
			{
				if (pUserBot->isSlaveMerchant())
				{
					CUser* pSlaveUser = g_pMain->GetUserPtr(pUserBot->GetSlaveGetID());
					if (pSlaveUser != nullptr)
					{
						if (pSlaveUser->isSlaveMerchant())
						{
							pSlaveUser->m_bSlaveMerchant = false;
							pSlaveUser->m_bSlaveUserID = -1;
						}
					}
					result.clear();
					result.Initialize(WIZ_MERCHANT);
					result << uint8(MERCHANT_CLOSE) << pUserBot->GetID();
					pUserBot->SendToRegion(&result);

					pUserBot->UserInOut(INOUT_OUT);
					return;
				}
				pUserBot->LastWarpTime = UNIXTIME + 10;

				pUserBot->m_bMerchantViewer = -1;
				//pUserBot->m_bSellingMerchantPreparing = false;
				pUserBot->m_bMerchantState = MERCHANT_STATE_NONE;

				Packet res(WIZ_MERCHANT, uint8(MERCHANT_CLOSE));
				res << pUserBot->GetID();
				pUserBot->SendToRegion(&res);
			}
		}
	}
}

void CUser::CancelMerchant()
{
	if (m_sMerchantsSocketID < 0) return;

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_TRADE_CANCEL));
	result << uint16(1);
	Send(&result);
	RemoveFromMerchantLookers();
}

void CUser::BuyingMerchantOpen(Packet& pkt)
{
	if (isSellingMerchantingPreparing())
		return;

	int16 errorCode = 0;
	if (isDead())
		errorCode = MERCHANT_OPEN_DEAD;
	else if (isStoreOpen())
		errorCode = MERCHANT_OPEN_SHOPPING;
	else if (isTrading())
		errorCode = MERCHANT_OPEN_TRADING;
	else if (GetZoneID() != ZONE_MORADON) // DeaFSezo maradon d��� pazar kurma engellendi
		errorCode = MERCHANT_OPEN_INVALID_ZONE;
	else if (GetLevel() < g_pMain->pServerSetting.MerchantLevel)
		errorCode = MERCHANT_OPEN_UNDERLEVELED;
	else if (isMerchanting())
		errorCode = MERCHANT_OPEN_MERCHANTING;
	else
		errorCode = MERCHANT_OPEN_SUCCESS;

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_OPEN));
	result << errorCode;
	Send(&result);

	if (errorCode == MERCHANT_OPEN_MERCHANTING)
		BuyingMerchantClose();

	if (errorCode == MERCHANT_OPEN_SUCCESS)
		m_bBuyingMerchantPreparing = true;

	memset(&m_arMerchantItems, 0, sizeof(m_arMerchantItems));
	DateTime time;
}

void CUser::BuyingMerchantClose()
{
	if (isSellingMerchantingPreparing() || isSellingMerchant())
		return;

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_CLOSE));
	result << GetSocketID();

	if (isBuyingMerchant())
	{

		if (plookersocketid >= 0)
		{
			if (CUser* pUser = g_pMain->GetUserPtr(plookersocketid))
			{
				pUser->m_sMerchantsSocketID = -1;
				Packet newpkt(WIZ_MERCHANT, uint8(29));
				newpkt << uint16(1);
				pUser->Send(&newpkt);
			}
		}
		level_merchant_time = 0;
		plookersocketid = -1;
		m_bBuyingMerchantPreparing = false;
		m_bMerchantState = MERCHANT_STATE_NONE;
		memset(m_arMerchantItems, 0, sizeof(m_arMerchantItems));
		Send(&result);
		SendToRegion(&result, nullptr, GetEventRoom());
		SetOffCha(_choffstatus::DEACTIVE, offcharactertype::merchant);
	}
	else if (m_sMerchantsSocketID >= 0)
	{
		RemoveFromMerchantLookers();
		Send(&result);
	}
}

void CUser::BuyingMerchantInsert(Packet& pkt)
{
	if (isDead() || isTrading() || isMining() || isFishing())
		return;

	if (!isBuyingMerchantingPreparing() || isSellingMerchantingPreparing() || isMerchanting())
		return;

	if (GetLevel() < g_pMain->pServerSetting.MerchantLevel)
		return;

	uint32 totalamount = 0;
	uint8 t_itemcount = pkt.read<uint8>(), n_itemcount = 0;
	if (!t_itemcount || t_itemcount > MAX_MERCH_ITEMS)
		return;

	for (int i = 0; i < t_itemcount; i++)
	{
		uint32 itemid, buying_price; uint16 item_count;
		pkt >> itemid >> item_count >> buying_price;
		auto pItem = g_pMain->GetItemPtr(itemid);
		if (pItem.isnull())
			return;

		if (!item_count || !buying_price
			|| (!pItem.m_bCountable && item_count != 1)
			|| (itemid >= ITEM_NO_TRADE_MIN && itemid <= ITEM_NO_TRADE_MAX)
			|| pItem.m_bRace == RACE_UNTRADEABLE || pItem.m_bCountable == 2)
		{
			return;
		}

		if (pItem.m_bKind == 255 && !pItem.m_bCountable && item_count != 1)
			return;

		m_arMerchantItems[i].nNum = itemid;
		m_arMerchantItems[i].sCount = item_count;
		m_arMerchantItems[i].nPrice = buying_price;
		m_arMerchantItems[i].sDuration = pItem.m_sDuration;
		totalamount += buying_price;
		n_itemcount++;
	}

	if (isGM())
	{
		CBot* myBot = nullptr;
		uint16 sCount = myrand(1, 10);
		for (int i = 0; i < 1; i++)
		{
			float Bonc = myrand(1, 15) * 1.0f;
			float Bonc2 = myrand(1, 15) * 1.0f;

			uint16 mSBotID = g_pMain->SpawnEventBotMerchant(3600, GetZoneID(), GetX(), GetY(), GetZ(), NULL, MIN_LEVEL_ARDREAM);
			if (mSBotID)
			{
				myBot = g_pMain->GetBotPtr(mSBotID);
				if (myBot == nullptr)
					return;

				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
					myBot->m_arMerchantItems[i] = m_arMerchantItems[i];

				if (!hasCoins(totalamount))
					return;

				{
					int count = 0, count2 = 0;
					for (int i = 0; i < MAX_MERCH_ITEMS; i++)
						if (myBot->m_arMerchantItems[i].nNum)
							count++;

					for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
						if (!myBot->GetItem(i)->nNum)
							count2++;

					if (count > count2)
					{
						count2 = 0;
						for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
						{
							auto* pItem = myBot->GetItem(i);
							if (pItem)
							{
								memset(pItem, 0, sizeof(_ITEM_DATA));
								count2++;
							}
							if (count2 >= count)
								break;
						}
					}
				}

				myBot->m_iLoyalty = myrand(100, 5000);
				myBot->m_bMerchantState = MERCHANT_STATE_BUYING;
				Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_INSERT));
				result << uint8(1);
				//myBot->SendToRegion(&result);

				result.clear();
				result.Initialize(WIZ_MERCHANT);
				result << uint8(MERCHANT_BUY_REGION_INSERT) << myBot->GetID();
				for (int i = 0; i < 4; i++)
				{
					result << myBot->m_arMerchantItems[i].nNum;
				}
				myBot->SendToRegion(&result);

				if (myBot->m_iGold < totalamount)
					myBot->m_iGold = myrand(totalamount, totalamount + 5000000);
			}
		}

		plookersocketid = -1;
		m_bBuyingMerchantPreparing = false;
		m_bMerchantState = MERCHANT_STATE_NONE;
		memset(m_arMerchantItems, 0, sizeof(m_arMerchantItems));

		Packet result;
		result.clear();
		result.Initialize(WIZ_MERCHANT);
		result << uint8(MERCHANT_BUY_CLOSE) << GetSocketID();
		SendToRegion(&result, nullptr, GetEventRoom());
		return;
	}

	if (!hasCoins(totalamount) || n_itemcount != t_itemcount)
		return;

	if (g_pMain->pLevMercInfo.status)
		level_merchant_time = UNIXTIME + (g_pMain->pLevMercInfo.rewardtime * MINUTE);

	m_bMerchantState = MERCHANT_STATE_BUYING;
	m_bBuyingMerchantPreparing = false;
	Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_INSERT));
	result << uint8(1);
	Send(&result);

	BuyingMerchantInsertRegion();
	SetOffCha(_choffstatus::ACTIVE, offcharactertype::merchant);
	MerchantCreationInsertLog(2);
}

void CUser::BuyingMerchantInsertRegion()
{
	Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_REGION_INSERT));
	result << GetSocketID();
	DateTime time;

	for (int i = 0; i < 4; i++)
		result << m_arMerchantItems[i].nNum;

	SendToRegion(&result, nullptr, GetEventRoom());
}

void CUser::BuyingMerchantList(Packet& pkt)
{
	//if (m_sMerchantsSocketID >= 0)
	//	RemoveFromMerchantLookers(); //This check should never be hit...

	uint16 uid = pkt.read<uint16>();

	CBot* pUserBot = g_pMain->GetBotPtr(uid);
	if (pUserBot != nullptr)
	{
		if (!pUserBot->isMerchanting())
			return;

		if (pUserBot->m_bMerchantViewer >= 0)
		{
			CUser* pLooker = g_pMain->GetUserPtr(pUserBot->m_bMerchantViewer);
			if (pLooker && pLooker->isInGame())
			{
				Packet newpkt(WIZ_MERCHANT, uint8(MERCHANT_ITEM_LIST));
				newpkt.SByte();
				newpkt << int16(-7) << pLooker->GetName();
				Send(&newpkt);
				return;
			}
			else pUserBot->m_bMerchantViewer = -1;
		}

		pUserBot->m_bMerchantViewer = GetID();
		m_sMerchantsSocketID = uid;

		Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_LIST));
		result << uint8(1) << uint16(uid);

		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		{
			_MERCH_DATA* pMerch = &pUserBot->m_arMerchantItems[i];
			if (!pMerch) continue;
			result << pMerch->nNum << pMerch->sCount
				<< pMerch->sDuration << pMerch->nPrice;
		}
		Send(&result);
		return;
	}

	CUser* pMerchant = g_pMain->GetUserPtr(uid);
	if (pMerchant == nullptr || !pMerchant->isMerchanting() || pMerchant->isSellingMerchantingPreparing() || !pMerchant->isBuyingMerchant())
		return;

	if (pMerchant->plookersocketid >= 0)
	{
		CUser* pLooker = g_pMain->GetUserPtr(pMerchant->plookersocketid);
		if (pLooker && pLooker->isInGame())
		{
			Packet newpkt(WIZ_MERCHANT, uint8(MERCHANT_ITEM_LIST));
			newpkt.SByte();
			newpkt << int16(-7) << pLooker->GetName();
			Send(&newpkt);
		}
		else pMerchant->plookersocketid = -1;
	}

	pMerchant->plookersocketid = GetID();
	m_sMerchantsSocketID = uid;

	if (g_pMain->pServerSetting.MerchantView)
	{
		if (pMerchant != nullptr)
		{
			DateTime timemachine;
			Packet merchanwiew(XSafe);
			merchanwiew << uint8(XSafeOpCodes::MERC_WIEWER_INFO) << uint8(1);
			merchanwiew.SByte();
			merchanwiew << GetName() << timemachine.GetHour() << timemachine.GetMinute();
			pMerchant->Send(&merchanwiew);
		}
	}
	Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_LIST));
	result << uint8(1) << uint16(uid);

	for (int i = 0; i < MAX_MERCH_ITEMS; i++)
	{
		_MERCH_DATA* pMerch = &pMerchant->m_arMerchantItems[i];
		if (pMerch == nullptr)
			continue;

		result << pMerch->nNum << pMerch->sCount
			<< pMerch->sDuration << pMerch->nPrice;
	}
	Send(&result);
}

void CUser::BuyingMerchantBuy(Packet& pkt)
{
	DateTime time;
	uint32 nPrice;
	uint16 sStackSize, sRemainingStackSize;
	uint8 bSellerSrcSlot, bMerchantListSlot;

	CBot* pMerchant = g_pMain->GetBotPtr(m_sMerchantsSocketID);
	if (pMerchant != nullptr)
	{
		if (pMerchant->GetMerchantState() != MERCHANT_STATE_BUYING)
			return;

		pkt >> bSellerSrcSlot >> bMerchantListSlot >> sStackSize;

		if (sStackSize == 0
			|| bSellerSrcSlot >= HAVE_MAX
			|| bMerchantListSlot >= MAX_MERCH_ITEMS)
			return;

		_MERCH_DATA* pWantedItem = &pMerchant->m_arMerchantItems[bMerchantListSlot];
		_ITEM_DATA* pSellerItem = GetItem(SLOT_MAX + bSellerSrcSlot);

		// Make sure the merchant actually has that item in that slot
		// and that they want enough, and the selling user has enough
		if (pWantedItem == nullptr
			|| pSellerItem == nullptr
			|| pWantedItem->nNum != pSellerItem->nNum
			|| pWantedItem->sCount < sStackSize
			|| pSellerItem->sCount < sStackSize
			// For scrolls, this will ensure you can only sell a full stack of scrolls.
			// For everything else, this will ensure you cannot sell items that need repair.
			|| pSellerItem->sDuration != pWantedItem->sDuration
			|| pSellerItem->isDuplicate()
			|| pSellerItem->isSealed()
			|| pSellerItem->isBound()
			|| pSellerItem->isRented())
			return;

		// If it's not stackable, and we're specifying something other than 1
		// we really don't care to handle this request...
		_ITEM_TABLE proto = g_pMain->GetItemPtr(pWantedItem->nNum);
		if (proto.isnull() || (!proto.m_bCountable && sStackSize != 1))
			return;

		// Do they have enough coins?
		nPrice = pWantedItem->nPrice * sStackSize;

		// and give them all to me, me, me!
		GoldGain(nPrice);

		// Get the remaining stack size after purchase.
		sRemainingStackSize = pSellerItem->sCount - sStackSize;

		// Update how many items the buyer still needs.
		pSellerItem->sCount -= sStackSize;
		pWantedItem->sCount -= sStackSize;

		// If the buyer needs no more, remove this item from the wanted list.
		if (pWantedItem->sCount == 0)
			memset(pWantedItem, 0, sizeof(_MERCH_DATA));

		// If the seller's all out, remove their item.
		if (pSellerItem->sCount <= 0)
			memset(pSellerItem, 0, sizeof(_ITEM_DATA));

		// Update players
		SendStackChange(pSellerItem->nNum, pSellerItem->sCount, pSellerItem->sDuration, bSellerSrcSlot);

		Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_BOUGHT));
		result << bMerchantListSlot << uint16(0) << GetName();
		//pMerchant->SendToRegion(&result);

		result.clear();
		result << uint8(MERCHANT_BUY_SOLD) << uint8(1) << bMerchantListSlot << pWantedItem->sCount << bSellerSrcSlot << pSellerItem->sCount;
		Send(&result);

		result.clear();
		result << uint8(MERCHANT_BUY_BUY) << uint8(1);
		Send(&result);

		if (bMerchantListSlot < 4 && pWantedItem->sCount == 0)
		{
			result.Initialize(WIZ_MERCHANT_INOUT);
			result << uint8(2) << m_sMerchantsSocketID << uint8(1) << uint8(0) << bMerchantListSlot;
			pMerchant->SendToRegion(&result);
		}

		int nItemsRemaining = 0;
		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		{
			if (pMerchant->m_arMerchantItems[i].nNum != 0)
				nItemsRemaining++;
		}

		if (nItemsRemaining == 0)
		{
			if (pMerchant->isSlaveMerchant())
			{
				CUser* pSlaveUser = g_pMain->GetUserPtr(pMerchant->GetSlaveGetID());
				if (pSlaveUser != nullptr)
				{
					if (pSlaveUser->isSlaveMerchant())
					{
						pSlaveUser->m_bSlaveMerchant = false;
						pSlaveUser->m_bSlaveUserID = -1;
					}
				}

				Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_CLOSE));
				result << pMerchant->GetID();
				pMerchant->SendToRegion(&result);
				pMerchant->UserInOut(INOUT_OUT);
				return;
			}
			pMerchant->LastWarpTime = UNIXTIME + 10;

			pMerchant->m_bMerchantViewer = -1;
			//pMerchant->m_bBuyingMerchantPreparing = false;
			pMerchant->m_bMerchantState = MERCHANT_STATE_NONE;
			memset(pMerchant->m_arMerchantItems, 0, sizeof(pMerchant->m_arMerchantItems));

			Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_CLOSE));
			result << pMerchant->GetID();
			pMerchant->SendToRegion(&result);
		}
		MerchantShoppingDetailInsertLog(true, 2, pWantedItem->nNum, pWantedItem->sCount, pWantedItem->nPrice, nullptr);
	}
	else
	{
		CUser* pMerchant = g_pMain->GetUserPtr(m_sMerchantsSocketID);

		if (pMerchant == nullptr)
			return;

		if (pMerchant->GetMerchantState() != MERCHANT_STATE_BUYING || !pMerchant->isBuyingMerchant() || pMerchant->isSellingMerchantingPreparing())
			return;

		pkt >> bSellerSrcSlot >> bMerchantListSlot >> sStackSize;

		if (sStackSize == 0
			|| bSellerSrcSlot >= HAVE_MAX
			|| bMerchantListSlot >= MAX_MERCH_ITEMS)
			return;

		_MERCH_DATA* pWantedItem = &pMerchant->m_arMerchantItems[bMerchantListSlot];
		_ITEM_DATA* pSellerItem = GetItem(SLOT_MAX + bSellerSrcSlot);

		if (pWantedItem == nullptr || pSellerItem == nullptr)
			return;

		if (pWantedItem->nNum != pSellerItem->nNum
			|| pWantedItem->sCount < sStackSize
			|| pSellerItem->sCount < sStackSize
			// For scrolls, this will ensure you can only sell a full stack of scrolls.
			// For everything else, this will ensure you cannot sell items that need repair.
			|| pSellerItem->sDuration != pWantedItem->sDuration
			|| pSellerItem->isDuplicate()
			|| pSellerItem->isSealed()
			|| pSellerItem->isBound()
			|| pSellerItem->isRented())
			return;

		// If it's not stackable, and we're specifying something other than 1
		// we really don't care to handle this request...
		_ITEM_TABLE proto = g_pMain->GetItemPtr(pWantedItem->nNum);
		if (proto.isnull() || (!proto.m_bCountable && sStackSize != 1))
			return;

		// Do they have enough coins?
		nPrice = pWantedItem->nPrice * sStackSize;
		if (pWantedItem->isKC && pMerchant->m_nKnightCash < nPrice)
			return;
		else if (!pMerchant->hasCoins(nPrice))
			return;

		// Now find the buyer a home for their item
		int8 bDstPos = pMerchant->FindSlotForItem(pWantedItem->nNum, sStackSize);
		if (bDstPos < 0)
			return;

		_ITEM_DATA* pMerchantItem = pMerchant->GetItem(bDstPos);
		if (pMerchantItem == nullptr)
			return;

		// Take coins off the buying merchant
		if (pWantedItem->isKC)
		{
			if (!pMerchant->CashLose(nPrice))
				return;

			CashGain(nPrice);
		}
		else
		{
			if (!pMerchant->GoldLose(nPrice))
				return;

			GoldGain(nPrice);
			pUserDailyRank.GMTotalSold += nPrice;
		}

		// Get the remaining stack size after purchase.
		sRemainingStackSize = pSellerItem->sCount - sStackSize;

		// Now we give the buying merchant their wares.
		pMerchantItem->nNum = pSellerItem->nNum;
		pMerchantItem->sDuration = pSellerItem->sDuration;
		pSellerItem->sCount -= sStackSize;
		pMerchantItem->sCount += sStackSize;

		// Update how many items the buyer still needs.
		pWantedItem->sCount -= sStackSize;

		// If the buyer needs no more, remove this item from the wanted list.
		if (pWantedItem->sCount == 0)
			memset(pWantedItem, 0, sizeof(_MERCH_DATA));

		// If the seller's all out, remove their item.
		if (pSellerItem->sCount <= 0)
			memset(pSellerItem, 0, sizeof(_ITEM_DATA));

		// TODO : Proper checks for the removal of the items in the array, we're now assuming everything gets bought

		// Update players
		SendStackChange(pSellerItem->nNum, pSellerItem->sCount, pSellerItem->sDuration, bSellerSrcSlot);
		pMerchant->SendStackChange(pMerchantItem->nNum, pMerchantItem->sCount, pMerchantItem->sDuration, bDstPos - SLOT_MAX,
			pMerchantItem->sCount == sStackSize); 	// if the buying merchant only has what they wanted, it's a new item.
		// (otherwise it was a stackable item that was merged into an existing slot)

		Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_BOUGHT));
		result << bMerchantListSlot << uint16(0) << GetName();
		pMerchant->Send(&result);

		result.clear();
		result << uint8(MERCHANT_BUY_SOLD) << uint8(1) << bMerchantListSlot << pWantedItem->sCount << bSellerSrcSlot << pSellerItem->sCount;
		Send(&result);

		result.clear();
		result << uint8(MERCHANT_BUY_BUY) << uint8(1);
		Send(&result);

		if (bMerchantListSlot < 4 && pWantedItem->sCount == 0)
		{
			result.Initialize(WIZ_MERCHANT_INOUT);
			result << uint8(2) << m_sMerchantsSocketID << uint8(1) << uint8(0) << bMerchantListSlot;
			pMerchant->SendToRegion(&result, nullptr, GetEventRoom());
		}

		int nItemsRemaining = 0;
		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		{
			if (pMerchant->m_arMerchantItems[i].nNum != 0)
				nItemsRemaining++;
		}

		MerchantShoppingDetailInsertLog(false, 2, pWantedItem->nNum, pWantedItem->sCount, pWantedItem->nPrice, pMerchant);
		if (nItemsRemaining == 0) pMerchant->BuyingMerchantClose();
	}
}

void CUser::RemoveFromMerchantLookers()
{
	int16 psockid = m_sMerchantsSocketID;

	m_sMerchantsSocketID = -1;

	CBot* pBot = g_pMain->GetBotPtr(psockid);
	if (pBot)
	{
		pBot->m_bMerchantViewer = -1;
		return;
	}

	CUser* pUser = g_pMain->GetUserPtr(psockid);
	if (!pUser) return;

	pUser->plookersocketid = -1;

	if (g_pMain->pServerSetting.MerchantView)
	{
		DateTime timemachine;
		Packet merchanwiew(XSafe);
		merchanwiew << uint8(XSafeOpCodes::MERC_WIEWER_INFO) << uint8(2); // Merchant Kapatildi Yazisi D�zeltildi.
		merchanwiew.SByte();
		merchanwiew << GetName() << timemachine.GetHour() << timemachine.GetMinute();
		pUser->Send(&merchanwiew);
	}
}

void CUser::MerchantOfficialList(Packet& pkt)
{
	if (!isInGame() || isDead() || isTrading() || isMerchanting()
		|| !isInMoradon() || isMining() || isFishing())
		return;

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_MENISIA_LIST));
	if (!GetMap() || GetMap()->m_bMenissiahList != 1)
		return;

	uint8 OpCode = pkt.read<uint8>();
	switch (OpCode)
	{
	case 1:
		MerchantListSend(pkt);
		break;
	case 2:
		MerchantListMoveProcess(pkt);
		break;
	default:
		//printf("MerchantOfficialList unhandled packets %d \n", OpCode);
		break;
	}
}

#define MAX_MERCHANT_RANGE  (35.0f)



void CUser::MerchantList(Packet& pkt)  //2369 Merchant list paket void
{
	uint16 Owner = pkt.read<uint16>();

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_LIST));
	if (Owner < MAX_USER) {
		CUser* pMerchUser = g_pMain->GetUserPtr(Owner);
		if (pMerchUser == nullptr || !pMerchUser->isInGame()
			|| pMerchUser->GetEventRoom() != GetEventRoom()
			|| pMerchUser->GetZoneID() != GetZoneID()
			|| !pMerchUser->isMerchanting())
			goto fail_result;
		_MERCH_DATA pnewlist[MAX_MERCH_ITEMS] = {};
		memset(pnewlist, 0, sizeof(pnewlist));
		if (pMerchUser->isSellingMerchant()) {
			uint8 PremiumState = pMerchUser->GetMerchantPremiumState() ? 1 : 0;
			result << uint8(1) << pMerchUser->GetID() << pMerchUser->GetMerchantState() << PremiumState;


			GetMerchantSlipList(pnewlist, pMerchUser);
			for (int i = 0, x = PremiumState == 1 ? 8 : 4; i < x; i++) {
				if (!pnewlist[i].IsSoldOut) result << pnewlist[i].nNum;
				else result << uint32(0);
			}
		}
		else {
			result << uint8(1) << pMerchUser->GetID() << pMerchUser->GetMerchantState() << uint8(0);
			for (int i = 0; i < 4; i++) result << pMerchUser->m_arMerchantItems[i].nNum;
		}

		if (true)
		{
			Packet result(WIZ_MERCHANT);
			result << uint8(0x32)
				<< uint8(1)
				<< pMerchUser->GetID()
				<< uint8(pMerchUser->GetMerchantState() == MERCHANT_STATE_SELLING ? false : true)
				<< (pMerchUser->GetMerchantState() == 1 ? false : pMerchUser->m_bPremiumMerchant); // Type of merchant [normal - gold] // bool*/

			for (int i = 0, listCount = (pMerchUser->GetMerchantState() == 1 ? 4 : (pMerchUser->m_bPremiumMerchant ? 8 : 4)); i < listCount; i++)
				result << (pMerchUser->GetMerchantState() == MERCHANT_STATE_SELLING ? pnewlist[i].nPrice : pMerchUser->m_arMerchantItems[i].nPrice) << (pMerchUser->GetMerchantState() == MERCHANT_STATE_SELLING ? uint8(pnewlist[i].isKC) : uint8(pMerchUser->m_arMerchantItems[i].isKC));

			Send(&result);
		}
	}
	else {
		CBot* pMerchUser = g_pMain->GetBotPtr(Owner);
		if (pMerchUser == nullptr || !pMerchUser->isInGame()
			|| pMerchUser->GetZoneID() != GetZoneID() || !pMerchUser->isMerchanting())
			goto fail_result;
		_MERCH_DATA pnewlist[MAX_MERCH_ITEMS] = {};
		memset(pnewlist, 0, sizeof(_MERCH_DATA));
		if (pMerchUser->isSellingMerchant()) {
			uint8 PremiumState = pMerchUser->m_bPremiumMerchant;
			result << uint8(1) << pMerchUser->GetID() << pMerchUser->GetMerchantState() << PremiumState;


			pMerchUser->GetMerchantSlipList(pnewlist, pMerchUser);
			for (int i = 0, x = PremiumState == 1 ? 8 : 4; i < x; i++) {
				if (!pnewlist[i].IsSoldOut) result << pnewlist[i].nNum;
				else result << uint32(0);
			}
		}
		else {
			result << uint8(1) << pMerchUser->GetID() << pMerchUser->GetMerchantState() << uint8(0);
			for (int i = 0; i < 4; i++) result << pMerchUser->m_arMerchantItems[i].nNum;
		}
		if (true)
		{
			Packet result(WIZ_MERCHANT);
			result << uint8(0x32)
				<< uint8(1)
				<< pMerchUser->GetID()
				<< uint8(pMerchUser->GetMerchantState() == MERCHANT_STATE_SELLING ? false : true)
				<< (pMerchUser->GetMerchantState() == 1 ? false : pMerchUser->m_bPremiumMerchant); // Type of merchant [normal - gold] // bool*/

			for (int i = 0, listCount = (pMerchUser->GetMerchantState() == 1 ? 4 : (pMerchUser->m_bPremiumMerchant ? 8 : 4)); i < listCount; i++)
				result << (pMerchUser->GetMerchantState() == MERCHANT_STATE_SELLING ? pnewlist[i].nPrice : pMerchUser->m_arMerchantItems[i].nPrice) << (pMerchUser->GetMerchantState() == MERCHANT_STATE_SELLING ? uint8(pnewlist[i].isKC) : uint8(pMerchUser->m_arMerchantItems[i].isKC));

			Send(&result);
		}
	}
	Send(&result);

	return;
fail_result:
	result << uint8(4);
	Send(&result);
}

//  Pazar arama SC'si

void CUser::MerchantEye()  //2369 Merchant list paket void
{
	// �TEMKOTNROLU EKLENECEK
	struct MerchantEyeData
	{
		uint8 nIndex;
		std::string strMerchantItem[12];
		MerchantEyeData()
		{
			for (int i = 0; i < 12; i++)
				strMerchantItem[i] = "";
			nIndex = 0;
		}
	};


	if (!CheckExistItem(810168000))
		return;

	uint16 nCount = 2;
	std::map<uint32, _MERCHANT_LIST*> filteredDamageList;

	int16 nSize = 0;

	std::map<uint16, uint16> TestMerchant;
	std::map<uint16, MerchantEyeData> willTP;

	g_pMain->m_MapBotList.m_lock.lock();
	auto m_sMapBotListArray = g_pMain->m_MapBotList.m_UserTypeMap;
	g_pMain->m_MapBotList.m_lock.unlock();

	foreach(itr, m_sMapBotListArray)
	{
		CBot* pUser = itr->second;
		if (pUser == nullptr)
			continue;

		if (!pUser->isInGame() || !pUser->isMerchanting() || pUser->GetZoneID() != GetZoneID())
			continue;
		MerchantEyeData item;
		item.nIndex = 1;

		for (int a = 0; a < MAX_MERCH_ITEMS; a++)
		{
			if (pUser->m_arMerchantItems[a].nNum == 0)
				continue;

			_ITEM_TABLE proto = g_pMain->GetItemPtr(pUser->m_arMerchantItems[a].nNum);
			if (proto.isnull())
				continue;


			item.strMerchantItem[a] = proto.m_sName.c_str();

		}
		willTP.insert(std::pair<uint16, MerchantEyeData>(pUser->GetID(), item));
	}

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);

		if (pUser == nullptr)
			continue;

		if (!pUser->isInGame() || !pUser->isMerchanting() || pUser->GetZoneID() != GetZoneID())
			continue;
		MerchantEyeData item;
		item.nIndex = 1;
		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		{
			if (pUser->m_arMerchantItems[i].nNum == 0)
				continue;

			_ITEM_TABLE proto = g_pMain->GetItemPtr(pUser->m_arMerchantItems[i].nNum);
			if (proto.isnull())
				continue;

			item.strMerchantItem[i] = proto.m_sName.c_str();

		}
		willTP.insert(std::pair<uint16, MerchantEyeData>(pUser->GetID(), item));
	}
	if (willTP.size() == 0)
		return;

	Packet error(XSafe, uint8(0xDD));
	error << uint16(willTP.size());
	error.SByte();
	foreach(itr, willTP)
	{

		error << uint16(itr->first);
		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
			error << itr->second.strMerchantItem[i].c_str();
	}
	SendCompressed(&error);
}
void CUser::MerchantListSend(Packet& pkt)
{
	uint32 RecvItemID = pkt.read<uint32>();
	Packet result(WIZ_MERCHANT, uint8(MERCHANT_MENISIA_LIST));

	std::vector<uint32> MerchantOfficalListen;
	int8 nCount = 0;

	if (RecvItemID != ITEM_MENICIAS_LIST)
	{
		result << uint8(1) << uint8(0) << uint8(3);
		Send(&result);
		return;
	}

	if (!CheckExistItem(ITEM_MENICIAS_LIST))
	{
		result << uint8(1) << uint8(0) << uint8(3);
		Send(&result);
		return;
	}

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);

		if (!pUser)
			continue;

		if (!pUser->isInGame() || !pUser->isMerchanting())
			continue;

		if (pUser->GetZoneID() == GetZoneID())
			MerchantOfficalListen.push_back(pUser->GetSocketID());
	}

	if (MerchantOfficalListen.size() > 0)
	{
		int16 nSize = (int16)MerchantOfficalListen.size();
		while (nSize > 0)
		{
			nCount++;
			result << uint8(1) << uint8(2) << nCount;

			if (nSize > 50)
				result << uint8(50);
			else
				result << uint8(nSize);

			result.SByte();
			foreach(itr, MerchantOfficalListen)
			{
				CUser* pUser = g_pMain->GetUserPtr(*itr);

				if (pUser == nullptr)
					continue;

				if (!pUser->isInGame() || !pUser->isMerchanting())
					continue;


				result << pUser->GetID() << pUser->GetName() << pUser->GetMerchantState();

				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
				{
					if (pUser->m_arMerchantItems[i].nNum == 3452816845)
						result << uint32(0) << uint32(0);
					else
						result << pUser->m_arMerchantItems[i].nNum << pUser->m_arMerchantItems[i].nPrice;
				}
			}
			Send(&result);
			nSize = nSize - 50;
		}
	}
	else
	{
		result << uint8(1) << uint8(2) << int8(0);
		Send(&result);
	}
}

void CUser::MerchantListMoveProcess(Packet& pkt)
{
	uint16 TargetID = pkt.read<uint16>();

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_MENISIA_LIST));
	if (isDead()
		|| isTrading()
		|| isMerchanting()
		|| isStoreOpen()
		|| isMining()
		|| isFishing())
	{
		result << uint8(1) << uint8(0) << uint8(3);
		Send(&result);
		return;
	}

	CUser* pUser = g_pMain->GetUserPtr(TargetID);
	if (pUser == nullptr)
	{
		result << uint8(1) << uint8(0) << uint8(3);
		Send(&result);
		return;
	}

	if (!pUser->isInGame() || !pUser->isMerchanting() || pUser->GetZoneID() != GetZoneID())
	{
		result << uint8(1) << uint8(0) << uint8(3);
		Send(&result);
		return;
	}

	ZoneChange(pUser->GetZoneID(), pUser->GetX(), pUser->GetZ());
}

void CUser::GetMerchantSlipList(_MERCH_DATA list[MAX_MERCH_ITEMS], CUser* pownermerch)
{
	if (!pownermerch) return;
	for (int i = 0; i < MAX_MERCH_ITEMS; i++)
	{
		if (pownermerch->m_arMerchantItems[i].nNum)
		{
			for (int x = 0; x < MAX_MERCH_ITEMS; x++)
			{
				if (!list[x].nNum) { list[x] = pownermerch->m_arMerchantItems[i]; break; }
			}
		}
	}
}

void CUser::MerchantSlipRefList(CUser* pM, bool merchcrea)
{
	if (!pM) return;

	if (!merchcrea)
	{
		Packet newpkt1(WIZ_MERCHANT_INOUT, uint8(1));
		newpkt1 << uint16(1) << pM->GetSocketID() << pM->GetMerchantState() << (pM->GetMerchantState() == 1 ? false : pM->m_bPremiumMerchant);
		pM->SendToRegion(&newpkt1, pM, GetEventRoom());
	}

	Packet newpkt2(WIZ_MERCHANT, uint8(MERCHANT_LIST));
	uint8 PremiumState = pM->GetMerchantPremiumState() ? 1 : 0;
	newpkt2 << uint8(1) << pM->GetID() << pM->GetMerchantState() << PremiumState;
	_MERCH_DATA pnewlist[MAX_MERCH_ITEMS] = {};
	memset(pnewlist, 0, sizeof(pnewlist));
	GetMerchantSlipList(pnewlist, pM);
	for (int i = 0, x = PremiumState == 1 ? 8 : 4; i < x; i++) if (!pnewlist[i].IsSoldOut) newpkt2 << pnewlist[i].nNum;
	pM->Send(&newpkt2);

	for (int i = 0, x = PremiumState == 1 ? 8 : 4; i < x; i++)
	{
		if (!pnewlist[i].nNum) continue;
		Packet newpkt3(WIZ_MERCHANT, uint8(MERCHANT_ITEM_ADD));
		newpkt3 << uint16(1) << pnewlist[i].nNum << pnewlist[i].sCount << pnewlist[i].sDuration << pnewlist[i].nPrice << pnewlist[i].bOriginalSlot << uint8(i);
		pM->Send(&newpkt3);
	}
}
void CUser::GiveSlaveMerchantItems()
{
	CBot* pSlaveUser = g_pMain->m_MapBotList.GetData(m_bSlaveUserID);
	if (pSlaveUser != nullptr)
	{
		if (pSlaveUser->isMerchanting())
		{
			for (int i = 0; i < MAX_MERCH_ITEMS; i++)
			{
				_MERCH_DATA* pMerch = &pSlaveUser->m_arMerchantItems[i];

				if (pMerch->nNum == 0)
					continue;

				int SlotForItem = FindSlotForItem(pMerch->nNum, pMerch->sCount);

				if (SlotForItem < 0)
					GiveWerehouseItem(pMerch->nNum, pMerch->sCount);
				else
					GiveItem("GiveSlaveMerc", pMerch->nNum, pMerch->sCount);
			}
			Packet result;
			// remove the items from the array now that they've been restored to the user
			memset(&pSlaveUser->m_arMerchantItems, 0, sizeof(pSlaveUser->m_arMerchantItems));

			result.clear();
			result.Initialize(WIZ_MERCHANT);
			result << uint8(MERCHANT_CLOSE) << pSlaveUser->GetID();
			pSlaveUser->SendToRegion(&result);

			m_bSlaveMerchant = false;
			m_bSlaveUserID = -1;
		}
	}
}

void CUser::MerchantSlaveClose()
{
	if (SlaveTotalTime > 0)
	{
		plookersocketid = -1;
		m_bSellingMerchantPreparing = false;
		m_bMerchantState = MERCHANT_STATE_NONE;
		Packet result(WIZ_MERCHANT, uint8(MERCHANT_CLOSE));
		result << GetSocketID();

		if (GetEventRoom() > 0)
			SendToRegion(&result, nullptr, GetEventRoom());
		else
			SendToRegion(&result);
	}
	memset(&m_arMerchantItems, 0, sizeof(m_arMerchantItems));
}

