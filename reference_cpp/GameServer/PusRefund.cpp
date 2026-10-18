#include "StdAfx.h"
#include "DBAgent.h"

#pragma region CUser::ReqHandleXSafe(Packet &pkt)
void CUser::ReqHandleXSafe(Packet &pkt) {
	uint8 opcode = pkt.read<uint8>();
	switch ((XSafeOpCodes)opcode)
	{
	case XSafeOpCodes::PusRefund:
	{
		uint8 pusopc = pkt.read<uint8>();
		switch ((pusrefunopcode)pusopc)
		{
		case pusrefunopcode::itemreurnsucces:
		{
			uint64 serial = pkt.read<uint64>();
			uint32 itemid = pkt.read<uint32>();
			g_DBAgent.PusRefundItemDelete(this, serial, itemid);
		}
		break;
		case pusrefunopcode::listadd:
		{
			uint64 serial = 0; uint32 itemid, itemprice, buyingtime; uint16 itemcount, duration; uint8 buytype = 0;
			pkt >> serial >> itemid >> itemcount >> itemprice >> duration >> buyingtime;
			if (serial != 0) g_DBAgent.PusRefundItemAdd(this, serial, itemid, itemcount, duration, itemprice, buyingtime, buytype);
		}
		break;
		}
	}
	break;
	}
}
#pragma endregion

#pragma region CUser::HandleItemReturn(Packet &pkt)
void CUser::HandleItemReturn(Packet &pkt) {
	uint8 opcode = pkt.read<uint8>();
	if ((pusrefunopcode)opcode != pusrefunopcode::ireturn) 
		return;

	uint64 serial = pkt.read<uint64>();
	uint8 slotid = pkt.read<uint8>();

	if (m_pusrefundtime > UNIXTIME) 
		return ItemReturnSendErrorOpcode(pusrefunopcode::procestime);

	m_pusrefundtime = UNIXTIME + 5;

	auto *pre = m_PusRefundMap.GetData(serial);
	if (!pre)
		return ItemReturnSendErrorOpcode(pusrefunopcode::itemnotfound);
	
	if (UNIXTIME > pre->expiredtime)
		return ItemReturnSendErrorOpcode(pusrefunopcode::timeexpired);

	_ITEM_DATA *pItem = nullptr; int8 pos = -1;
	for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++) {
		if (auto *pitem = GetItem(i)) {
			if (pitem->nSerialNum == serial && pitem->nNum == pre->itemid) { pos = i; pItem = pitem; break; }
		}
	}

	if (!pItem)
		return ItemReturnSendErrorOpcode(pusrefunopcode::notinventory);

	auto  getitem = g_pMain->GetItemPtr(pItem->nNum);
	if (getitem.isnull()) 
		return ItemReturnSendErrorOpcode(pusrefunopcode::itemnotfound);
	
	bool scralable = getitem.GetKind() == 255 && !getitem.m_bCountable;
	if (pItem->sCount != pre->itemcount || (scralable && pItem->sDuration != pre->itemduration))
		return ItemReturnSendErrorOpcode(pusrefunopcode::itemused);
	
	pItem = GetItem(pos);
	if (pItem == nullptr)
		return ItemReturnSendErrorOpcode(pusrefunopcode::itemnotfound);

	pItem->sCount = 0; pItem->sDuration = 0;

	SendStackChange(getitem.m_iNum, pItem->sCount, pItem->sDuration, pos - SLOT_MAX, false, pItem->nExpirationTime);

	memset(pItem, 0, sizeof(_ITEM_DATA));

	/*int tax = (pre->itemprice * 10) / 100;
	int finalprice = pre->itemprice - tax;
	if (finalprice > 0)GiveBalance(finalprice);*/

	if (pre->itemprice > 0) GiveBalance(!pre->buytype ? pre->itemprice : 0, pre->buytype ? pre->itemprice : 0);
	Packet newpkt(XSafe, uint8(XSafeOpCodes::PusRefund));
	newpkt << (uint8)pusrefunopcode::itemreurnsucces << serial << pre->itemid;
	Send(&newpkt);
	g_pMain->AddDatabaseRequest(newpkt, this);
	m_PusRefundMap.DeleteData(serial);
}
#pragma endregion

#pragma region CUser::PusRefundSendList()
void CUser::PusRefundSendList() {
	if (m_PusRefundMap.IsEmpty()) return;

	uint16 count = 0;
	Packet newpkt(XSafe, uint8(XSafeOpCodes::PusRefund));
	newpkt << uint8(pusrefunopcode::listsend) << count;
	foreach_stlmap(itr, m_PusRefundMap) {
		auto* p = itr->second;
		if (!p || UNIXTIME > p->expiredtime) continue;
		newpkt << itr->first << itr->second->itemid << itr->second->itemprice << itr->second->expiredtime;
		count++;
	}
	newpkt.ByteBuffer::put(2, count);
	Send(&newpkt);
}
#pragma endregion

bool CUser::PusRefundPurchase(uint64 serial, uint32 itemid, uint16 itemcount, uint32 itemprice, _ITEM_TABLE ptable, uint8 buytype) {
	if (ptable.isnull() || !serial) return false;
	auto *pre = m_PusRefundMap.GetData(serial);
	if (pre) return false;

	uint32 buyingtime = (uint32)UNIXTIME + (60 * 60);//(uint32)UNIXTIME + 30 * 60 * 24 * 1;

	_PUS_REFUND *pref = new _PUS_REFUND();
	pref->accountid = GetAccountName();
	pref->itemid = itemid;
	pref->itemprice = itemprice;
	pref->itemduration = ptable.m_sDuration;
	pref->itemcount = itemcount;
	pref->expiredtime = buyingtime;
	pref->buytype = buytype;
	if (!m_PusRefundMap.PutData(serial, pref)) { delete pref; return false; }

	Packet newpkt(XSafe, uint8(XSafeOpCodes::PusRefund));
	newpkt << (uint8)pusrefunopcode::listadd << serial << itemid << itemcount << itemprice << ptable.m_sDuration << buyingtime << buytype;
	Send(&newpkt);
	g_pMain->AddDatabaseRequest(newpkt, this);
	return true;
}

void CUser::ItemReturnSendErrorOpcode(pusrefunopcode opcode) {
	Packet newpkt(XSafe, uint8(XSafeOpCodes::PusRefund));
	newpkt << (uint8)opcode;
	Send(&newpkt);
}

void CUser::PUSGiftPurchase(Packet &pkt) {

	if (pCindWar.isEventUser() && g_pMain->isCindirellaZone(GetZoneID())) {
		g_pMain->SendHelpDescription(this, "You can't use power-up-store while Fun Class event.");
		return;
	}

	uint32 pusid; std::string name;
	pkt >> pusid >> name;

	g_pMain->SendHelpDescription(this, "the system is currently down.");
	return;

	if (m_pusgifttime > UNIXTIME2) {
		g_pMain->SendHelpDescription(this, "Please wait for a while before sending a gift.");
		return;
	}
	m_pusgifttime = UNIXTIME2 + 5000;

	if (name.empty() || name.length() > MAX_ID_SIZE) {
		g_pMain->SendHelpDescription(this, "Username is wrong.");
		return;
	}

	if (isMerchanting() || isTrading()) {
		g_pMain->SendHelpDescription(this, "You cannot send gifts while shopping.");
		return;
	}

	if (isDead()) {
		g_pMain->SendHelpDescription(this, "You cannot send gifts while you are dead.");
		return;
	}

	_PUS_ITEM* item = g_pMain->m_PusItemArray.GetData(pusid);
	if (item == nullptr) {
		g_pMain->SendHelpDescription(this, "server no1.");
		return;
	}

	uint32 totalCash = item->Price;
	if (m_nKnightCash < totalCash) {
		g_pMain->SendHelpDescription(this, "Your cash amount is not enough.");
		return;
	}

	CUser* pUser = g_pMain->GetUserPtr(name, NameType::TYPE_CHARACTER);
	if (!pUser || !pUser->isInGame()) {
		g_pMain->SendHelpDescription(this, "The player is not in the game.");
		return;
	}

	g_pMain->SendHelpDescription(this, "Your gift request has been added to the system for processing.");
	Packet newpkt(WIZ_DB_SAVE_USER, uint8(ProcDbType::PusGiftSendLetter));
	newpkt << item->ItemID << item->Price << name;
	g_pMain->AddDatabaseRequest(newpkt, this);
}