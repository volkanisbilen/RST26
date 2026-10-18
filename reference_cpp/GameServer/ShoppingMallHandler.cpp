#include "stdafx.h"
#include "DBAgent.h"

void CUser::ShoppingMall(Packet & pkt)
{
	uint8 opcode = pkt.read<uint8>();

	switch (opcode)
	{
	case STORE_OPEN:
		HandleStoreOpen(pkt);
		break;

	case STORE_CLOSE:
		HandleStoreClose();
		break;

	case STORE_BUY:
	case STORE_MINI: // not sure what this is
	case STORE_PROCESS:
		/* fairly certain there's no need to emulate these as they're unused */
		break;

	case STORE_LETTER:
		LetterSystem(pkt);
		break;

	default:
		TRACE("Unknown shoppingmall packet: %X\n", opcode);
		printf("Unknown shoppingmall packet: %X\n", opcode);
	}
}

// We're opening the PUS...
void CUser::HandleStoreOpen(Packet & pkt)
{
	Packet result(WIZ_SHOPPING_MALL, uint8(STORE_OPEN));
	int16 sFreeSlot = 0;

	if (isDead()
		|| isTrading()
		|| isMerchanting()
		/*|| isStoreOpen()*/)
	{
		result << uint16(-9);
		Send(&result);
		return;
	}

	// Not allowed in private arenas
	if (GetZoneID() >= 40 && GetZoneID() <= 45)
	{
		result << uint16(-9);
		Send(&result);
		return;
	}

	for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
	{
		if (GetItem(i)->nNum == 0)
			sFreeSlot++;
	}

	if (sFreeSlot <= 0)
	{
		result << uint16(-8);
		Send(&result);
		return;
	}

	m_bStoreOpen = true;
	UserDataSaveToAgent();

	result << uint16(1) << sFreeSlot;
	Send(&result);
}

// We're closing the PUS so that we can call LOAD_WEB_ITEMMALL and load the extra items.
void CUser::HandleStoreClose()
{
	Packet result(WIZ_SHOPPING_MALL, uint8(STORE_CLOSE));
	m_bStoreOpen = false;
	g_pMain->AddDatabaseRequest(result, this);
}

void CUser::ReqPusSessionCreate(Packet &pkt)
{
	return;
	uint16 sFreeCount = pkt.read<uint16>();
	int16 sErrorCode = 1;

	g_DBAgent.DestroyPusSessions(this);
	if (!g_DBAgent.CreatePusSession(sFreeCount, this))
		sErrorCode = -8;

	Packet result(WIZ_SHOPPING_MALL, uint8(STORE_OPEN));
	result << sErrorCode << sFreeCount;
	Send(&result);

}

void CUser::ReqLoadWebItemMall()
{
	//g_DBAgent.DestroyPusSessions(this);
	Packet result(WIZ_SHOPPING_MALL, uint8(STORE_CLOSE));
	std::vector<_ITEM_DATA> itemList;

	if (!g_DBAgent.LoadWebItemMall(itemList, this))
		return;

	// reuse the GiveItem() method for giving them the item, just don't send the packet
	// as it's handled by STORE_CLOSE.

	foreach(itr, itemList)
		GiveItem("WebItemMall", itr->nNum, itr->sCount, true, itr->nExpirationTime);

	for (int i = SLOT_MAX; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA * pItem = GetItem(i);
		if (pItem == nullptr)
			continue;

		result << pItem->nNum
			<< pItem->sDuration
			<< pItem->sCount
			<< pItem->bFlag	// item type flag (e.g. rented)
			<< pItem->sRemainingRentalTime
			<< pItem->nExpirationTime;
	}
	Send(&result);
}