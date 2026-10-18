#include "stdafx.h"
#include "DBAgent.h"
#include "FerihaQueque.h"
//lockkontrolyeni
#pragma region AdiniFerihaKoydum::tKnightLogger()
void AdiniFerihaKoydum::tKnightLogger()
{
	while (_running[(uint8)dbreqtype::Logger]) {
		Packet* p = nullptr;
		_lock[(uint8)dbreqtype::Logger].lock();
		if (_queue[(uint8)dbreqtype::Logger].size()) {
			p = _queue[(uint8)dbreqtype::Logger].front();
			_queue[(uint8)dbreqtype::Logger].pop();
		}
		_lock[(uint8)dbreqtype::Logger].unlock();

		if (!p) {
			if (!_running[(uint8)dbreqtype::Logger]) break;
			Wait((uint8)dbreqtype::Logger);
			continue;
		}

		Packet& pkt = *p;
		pkt.DByte();
		std::string tablename = "-", sqlinfo = "-", log = "-";
		pkt >> tablename >> sqlinfo;
		if (tablename.empty() || sqlinfo.empty()) goto retdelete;
		log = string_format("INSERT INTO %s VALUES (%s)", tablename.c_str(), sqlinfo.c_str());
		if (log.empty()) goto retdelete;
		g_DBAgent.InsertDatabaseLog(log);

	retdelete:
		/*if (p && p->size()) */delete p;
	}
}
#pragma endregion

#pragma region void CDBAgent::InsertDatabaseLog
void CDBAgent::InsertDatabaseLog(std::string sqlinfo) {
	if (sqlinfo.empty()) return;
	std::unique_ptr<OdbcCommand> dbCommand(m_LogDB->CreateCommand());
	if (dbCommand.get() == nullptr) return;
	if (!dbCommand->Execute(sqlinfo)) ReportSQLError(m_LogDB->GetError());
}
#pragma endregion

#pragma region CUser::NpcShoppingLog
void CUser::NpcShoppingLog(std::vector<ITEMS> p, uint16 protoid, uint8 Type) {
	Packet newpkt(0);
	std::string log = string_format("'%s','%s',GETDATE(),%d,%d,'%s'", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), GetZoneID(), protoid, Type == 2 ? "SELLING" : "BUYING");

	int counter = 0;
	foreach(itr, p) {
		if (counter >= 14) break;
		log.append(string_format(",%d,%d,%d", itr->ITEMID, itr->_ICOUNT, itr->price));
		counter++;
	}

	if (counter < 14)
		for (int i = 0; i < 14 - counter; i++)
			log.append(string_format(",%d,%d,%d", 0, 0, 0));

	newpkt << std::string("NPC_SHOPPING") << log;
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::ChatInsertLog
void CUser::ChatInsertLog(uint8 chattype, std::string descp, std::string message, CUser* ptarget) {
	if (descp.empty() || message.empty()) return;

	Packet newpkt(0);
	std::string taccid = ptarget ? ptarget->GetAccountName() : std::string("-"), tusid = ptarget ? ptarget->GetName() : std::string("-");
	newpkt << std::string("CHAT");
	newpkt << string_format("'%s','%s','%s',GETDATE(),%d,'%s','%s','%s','%s',%d,%d,%d",
		regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(GetRemoteIP()).c_str(), chattype, regex_quotes(descp).c_str(), regex_quotes(message).c_str(), regex_quotes(taccid).c_str(),
		regex_quotes(tusid).c_str(), GetZoneID(), (uint16)GetX(), (uint16)GetZ());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::LoginInsertLog()
void CUser::LoginInsertLog() {
	Packet newpkt(0);
	newpkt << std::string("LOGIN");
	newpkt << string_format("'%s','%s','%s',GETDATE(),%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(GetRemoteIP()).c_str(), GetZoneID());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::LogoutInsertLog()
void CUser::LogoutInsertLog() {
	Packet newpkt(0);
	newpkt << std::string("LOGOUT");
	newpkt << string_format("'%s','%s','%s',GETDATE(),%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(GetRemoteIP()).c_str(), GetZoneID());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::Disconnectprintfwriter
void CUser::Disconnectprintfwriter(std::string fonkname, std::string descp, int dccode) {
	Packet newpkt(0);
	newpkt << std::string("DISCONNECT");
	newpkt << string_format("'%s','%s','%s',GETDATE(),'%s','%s',%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(GetRemoteIP()).c_str(), regex_quotes(fonkname).c_str(), regex_quotes(descp).c_str(), dccode);
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::GiveItemInsertLog
void CUser::GiveItemInsertLog(std::string placeowned, uint32 itemid, uint16 itemcount, CUser* powned) {
	if (!powned) return;
	Packet newpkt(0);
	newpkt << std::string("ITEMS_RECEIVED");
	newpkt << string_format("'%s','%s',GETDATE(),'%s',%d,%d,%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(placeowned).c_str(), itemid, itemcount, GetZoneID(), (uint16)GetX(), (uint16)GetZ());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::RobItemInsertLog
void CUser::RobItemInsertLog(uint32 itemid, uint16 itemcount, uint8 pos, CUser* powned) {
	if (!powned) return;
	Packet newpkt(0);
	newpkt << std::string("ITEMS_LOST");
	newpkt << string_format("'%s','%s',GETDATE(),%d,%d,%d,%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), itemid, itemcount, pos, GetZoneID(), (uint16)GetX(), (uint16)GetZ());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::MerchantCreationInsertLog
void CUser::MerchantCreationInsertLog(uint8 merchtype) {

	std::string merctype = "SELL";
	if (merchtype == 2) merctype = "BUY";

	Packet newpkt(0);
	newpkt << std::string("MERCHANT_CREATION");

	std::string sqlinfo = string_format("'%s','%s','%s',GETDATE(),'%s'", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(GetRemoteIP()).c_str(), regex_quotes(merctype).c_str());
	for (int i = 0; i < MAX_MERCH_ITEMS; i++) sqlinfo.append(string_format(",%d,%d,%d", m_arMerchantItems[i].nNum, m_arMerchantItems[i].sCount, m_arMerchantItems[i].nPrice));
	newpkt << sqlinfo;
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::ItemRemoveInsertLog
void CUser::ItemRemoveInsertLog(uint32 itemid) {
	if (!itemid)
		return;

	Packet newpkt(0);
	newpkt << std::string("ITEMREMOVE");
	newpkt << string_format("'%s','%s','%s',GETDATE(),'%d'", regex_quotes(GetAccountName()).c_str(),
		regex_quotes(GetName()).c_str(), regex_quotes(GetRemoteIP()).c_str(), itemid);
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::MerchantShoppingDetailInsertLog
void CUser::MerchantShoppingDetailInsertLog(bool bot, uint8 merchtype, uint32 itemid, uint16 itemcount, uint32 itemprice, CUser* pCostumer) {

	std::string accountid = "bot", struserid = "bot";
	if (!bot) {
		if (!pCostumer || !pCostumer || pCostumer->GetAccountName().empty() || pCostumer->GetName().empty()) return;
		accountid = pCostumer->GetAccountName();
		struserid = pCostumer->GetName();
	}

	std::string merctype = "SELL";
	if (merchtype == 2) merctype = "BUY";
	Packet newpkt(0);
	newpkt << std::string("MERCHANT_SHOPPING");
	newpkt << string_format("'%s','%s','%s',GETDATE(),'%s',%d,%d,%d,%d,'%s','%s'",
		regex_quotes(GetAccountName()).c_str(),
		regex_quotes(GetName()).c_str(),
		regex_quotes(GetRemoteIP()).c_str(),
		regex_quotes(merctype).c_str(), itemid, itemcount, itemprice, itemprice * itemcount,
		regex_quotes(accountid).c_str(),
		regex_quotes(struserid).c_str());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::KillingNpcInsertLog
void CUser::KillingNpcInsertLog(CNpc* pdeadnpc) {
	if (!pdeadnpc) return;
	Packet newpkt(0);
	newpkt << std::string("KILLING_NPCS");
	newpkt << std::string(string_format("'%s','%s',GETDATE(),%d,'%s',%d,%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), pdeadnpc->GetProtoID(),
		regex_quotes(pdeadnpc->GetName()).c_str(), pdeadnpc->isMonster(), pdeadnpc->GetZoneID(), (uint16)pdeadnpc->GetX(), (uint16)pdeadnpc->GetZ()));
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

void CUser::ExpChangeInsertLog(int64 amount, int64 bmount, std::string descp) {
	if (!amount || amount < 500000)
		return;

	Packet newpkt(0);
	newpkt << std::string("EXP_CHANGE");
	newpkt << std::string(string_format("'%s','%s',GETDATE(),'%s',%d, %d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(),
		regex_quotes(descp).c_str(), amount, bmount));
	g_pMain->AddLogRequest(newpkt);
}
#pragma region CUser::UpgradeInsertLog2
void CUser::UpgradeInsertLog2(uint32 itemid, uint32 money, uint8 uptype, bool status, uint32 reqitemid, uint16 reqitemcount) {

	std::string type = "Auto Upgrade", stat = "FAILED", ItemName = "-";

	if (uptype == 1 || uptype == 2 || uptype == 3) type = "Auto Upgrade";
	else if (uptype == 4 || uptype == 15) type = "Auto Upgrade";
	else if (uptype == 8) type = "Auto Upgrade";

	if (status) stat = "SUCCESS";

	Packet newpkt(0);
	newpkt << std::string("UPGRADE");
	std::string sqlinfo = string_format("'%s','%s',GETDATE(),%d,%d,'%s','%s','%s'", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), money, itemid, regex_quotes(ItemName).c_str(), regex_quotes(type).c_str(), regex_quotes(stat).c_str());
	for (int i = 0; i < 9; i++)
		sqlinfo.append(string_format(",%d,%d", reqitemid, reqitemcount));
	newpkt << sqlinfo;
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion
#pragma region CUser::UpgradeInsertLog
void CUser::UpgradeInsertLog(uint32 itemid, uint32 money, uint8 uptype, bool status, uint32 reqitemid[9], uint16 reqitemcount[9]) {

	std::string type = "Upgrade", stat = "FAILED", ItemName = "-";

	if (uptype == 1 || uptype == 2 || uptype == 3) type = "Plus Upgrade";
	else if (uptype == 4 || uptype == 15) type = "RebirthUpgrade";
	else if (uptype == 8) type = "Accessory Upgrade";

	if (status) stat = "SUCCESS";

	Packet newpkt(0);
	newpkt << std::string("UPGRADE");
	std::string sqlinfo = string_format("'%s','%s',GETDATE(),%d,%d,'%s','%s','%s'", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), money, itemid, regex_quotes(ItemName).c_str(), regex_quotes(type).c_str(), regex_quotes(stat).c_str());
	for (int i = 0; i < 9; i++) sqlinfo.append(string_format(",%d,%d", reqitemid[i], reqitemcount[i]));
	newpkt << sqlinfo;
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::KillingUserInsertLog
void CUser::KillingUserInsertLog(CUser* pdeaduser) {
	if (!pdeaduser) return;

	Packet newpkt(0);
	newpkt << std::string("KILLING_USERS");
	newpkt << string_format("'%s','%s',GETDATE(),'%s','%s',%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(),
		regex_quotes(pdeaduser->GetAccountName()).c_str(), regex_quotes(pdeaduser->GetName()).c_str(), pdeaduser->GetZoneID(), (uint16)pdeaduser->GetX(), (uint16)pdeaduser->GetZ());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::UserNameChangeInsertLog
void CUser::UserNameChangeInsertLog(std::string oldname, std::string newname) {
	if (oldname.empty() || newname.empty()) return;

	Packet newpkt(0);
	newpkt << std::string("USER_NAME_CHANGE");
	newpkt << string_format("'%s','%s','%s',GETDATE()", regex_quotes(GetAccountName()).c_str(), regex_quotes(oldname).c_str(), regex_quotes(newname).c_str());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::ClanNameChangeInsertLog
void CUser::ClanNameChangeInsertLog(std::string oldname, std::string newname) {
	if (oldname.empty() || newname.empty()) return;

	Packet newpkt(0);
	newpkt << std::string("CLAN_NAME_CHANGE");
	newpkt << string_format("'%s','%s','%s',GETDATE()",
		regex_quotes(GetAccountName()).c_str(),
		regex_quotes(oldname).c_str(),
		regex_quotes(newname).c_str());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::NationTransferInsertLog
void CUser::NationTransferInsertLog(uint8 oldnation, uint8 newnation) {
	Packet newpkt(0);
	newpkt << std::string("NATION_TRANSFER");
	newpkt << string_format("'%s','%s',GETDATE(),%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), oldnation, newnation);
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

//#pragma region CUser::NationTransferInsertLog
//void CUser::JobChangeInsertLog(uint16 oldClass, uint16 newClass, uint16 oldRace, uint16 newRace) {
//	Packet newpkt(0);
//	newpkt << std::string("JOB_CHANGE");
//	newpkt << string_format("'%s','%s',GETDATE(),%d,%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), oldClass, newClass, oldRace, newRace);
//	g_pMain->AddLogRequest(newpkt);
//}
//#pragma endregion

#pragma region CUser::NationTransferInsertLog
void CUser::JobChangeInsertLog(uint16 oldClass, uint16 newClass, uint16 oldRace, uint16 newRace) {
	Packet newpkt(0);
	newpkt << std::string("JOB_CHANGE");
	newpkt << string_format("'%s','%s',GETDATE(),%d,%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), oldClass, newClass, oldRace, newRace);
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion


#pragma region CUser::PusShoppingInsertLog
void CUser::PusShoppingInsertLog(uint32 itemid, uint16 itemcount, uint32 itemcashcount) {
	Packet newpkt(0);
	newpkt << std::string("PUS_SHOPPING");
	newpkt << string_format("'%s','%s','%s', GETDATE(),%d,%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(GetRemoteIP()).c_str(), itemid, itemcount, itemcashcount, GetZoneID());
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::NpcDropReceivedInsertLog
void CUser::NpcDropReceivedInsertLog(uint32 itemid, uint16 itemcount, uint16 npcid) {
	Packet newpkt(0);
	newpkt << std::string("NPC_DROP_RECEIVED");
	std::string sqlinfo = string_format("'%s','%s',GETDATE(),%d,%d,%d,%d,%d,%d", regex_quotes(GetAccountName()).c_str(),
		regex_quotes(GetName()).c_str(),
		itemid,
		itemcount,
		GetZoneID(),
		(uint16)GetX(),
		(uint16)GetZ(),
		npcid);
	for (int i = 0; i < MAX_PARTY_USERS; i++) sqlinfo.append(string_format(",'%s'", "-"));
	newpkt << sqlinfo;
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::PremiumInsertLog
void CUser::PremiumInsertLog(uint8 premiumtype, uint32 premiumtime) {
	Packet newpkt(0);
	newpkt << std::string("PREMIUM");
	newpkt << string_format("'%s','%s',GETDATE(),%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), premiumtype, premiumtime);
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::TradeInsertLog
void CUser::TradeInsertLog(CUser* ptarget, uint32 money, uint32 tmoney, uint32 itemid[9], uint16 itemcount[9], uint32 titemid[9], uint16 titemcount[9]) {
	if (!ptarget) return;

	Packet newpkt(0);
	newpkt << std::string("TRADE");
	std::string sqlinfo = string_format("'%s','%s','%s',GETDATE(),%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(GetRemoteIP()).c_str(), money);
	for (int i = 0; i < 9; i++) sqlinfo.append(string_format(",%d,%d", itemid[i], itemcount[i]));
	sqlinfo.append(string_format(",'%s','%s','%s',%d", regex_quotes(ptarget->GetAccountName()).c_str(), regex_quotes(ptarget->GetName()).c_str(), regex_quotes(ptarget->GetRemoteIP()).c_str(), tmoney));
	for (int i = 0; i < 9; i++) sqlinfo.append(string_format(",%d,%d", titemid[i], titemcount[i]));
	newpkt << sqlinfo;
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::LoyaltyChangeInsertLog
void CUser::LoyaltyChangeInsertLog(std::string placeowned, uint32 currentloyalty, uint32 amount, uint32 amountend, uint32 finalloyalty) {
	Packet newpkt(0);
	newpkt << std::string("LOYALTY_CHANGE");
	newpkt << string_format("'%s','%s',GETDATE(),'%s',%d,%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(placeowned).c_str(), currentloyalty, amount, amountend, finalloyalty);
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion

#pragma region CUser::LoyaltyChangeInsertLog
void CUser::ClanBankInsertLog(CKnights* pKnights, uint32 itemid, uint32 count, uint32 money, bool add) {
	if (!pKnights) return;

	std::string type = add ? "ADD" : "DELETE";

	Packet newpkt(0);
	newpkt << std::string("CLAN_BANK");
	newpkt << string_format("'%s','%s',GETDATE(),'%s',%d,'%s',%d,%d,%d", regex_quotes(GetAccountName()).c_str(), regex_quotes(GetName()).c_str(), regex_quotes(type).c_str(), pKnights->GetID(), pKnights->GetName().empty() ? std::string("-").c_str() : pKnights->GetName().c_str(), itemid, count, money);
	g_pMain->AddLogRequest(newpkt);
}
#pragma endregion