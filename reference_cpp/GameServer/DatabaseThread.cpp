#include "stdafx.h"
#include "../shared/Condition.h"
#include "KnightsManager.h"
#include "KingSystem.h"
#include "DBAgent.h"
#include <time.h>
#include "../shared/DateTime.h"
#include <sstream>

extern CDBAgent g_DBAgent;
using std::string;

#pragma region CGameServerDlg::ReqHandleAuthority(Packet& pkt, CUser* pUser)
void CGameServerDlg::ReqHandleAuthority(Packet& pkt, CUser* pUser)
{
	std::string strUserID, strGM, banReason;
	uint8 banType = 0; uint32 sPeriod = 0;
	pkt >> strGM >> strUserID >> banType >> sPeriod >> banReason;
	g_DBAgent.UpdateUserAuthority(strGM, strUserID, sPeriod, banType, banReason, pUser);
}
#pragma endregion

void CGameServerDlg::UserAuthorityUpdate(BanTypes s, CUser* pmaster, std::string targetid, std::string desc, uint32 period)
{
	if (targetid.empty()) return;

	std::string reqowner = "SERVER";
	if (pmaster) reqowner = pmaster->GetName();

	Packet result(WIZ_AUTHORITY_CHANGE);
	result << reqowner << targetid << uint8(s) << period << desc;
	g_pMain->AddDatabaseRequest(result, pmaster);

	/*DateTime time;
	g_pMain->WriteBanListLogFile(string_format("%d:%d:%d || Sender=%s,Command=%s,User=%s\n", time.GetHour(), time.GetMinute(), time.GetSecond(), bannedside.c_str(), Reason.c_str(), bannedid));*/
}

void CUser::ReqHandleEvent(Packet& pkt, CUser* pUser)
{
	uint8 Opcode;
	pkt >> Opcode;

	switch (Opcode)
	{
	case TEMPLE_DRAKI_TOWER_LIST:
	{
		Packet result(WIZ_EVENT, uint8(TEMPLE_DRAKI_TOWER_LIST));
		g_DBAgent.ReqDrakiTowerList(result, this);
		Send(&result);
	}
	break;
	case TEMPLE_DRAKI_TOWER_ENTER:
	{
		Packet result(WIZ_EVENT, uint8(TEMPLE_DRAKI_TOWER_ENTER));
		uint32 nItemID; uint8 EnterDungeon;
		pkt >> nItemID >> EnterDungeon;

		if (nItemID != 0 && nItemID != CERTIFIKAOFDRAKI)
		{
			result << uint32(5);
			Send(&result);
			return;
		}

		if (!m_bDrakiEnteranceLimit
			&& !CheckExistItem(CERTIFIKAOFDRAKI, 1))
		{
			result << uint32(7);
			Send(&result);
			return;
		}

		if (!g_DBAgent.DrakiDataCheck(this))
		{
			result << uint32(10);
			Send(&result);
			return;
		}

		if (!m_bDrakiEnteranceLimit && !RobItem(CERTIFIKAOFDRAKI, 1))
		{
			result << uint32(10);
			Send(&result);
			return;
		}

		uint16 SignRoom = 0;
		for (int i = 1; i <= EVENTMAXROOM; i++)
		{
			_DRAKI_TOWER_INFO* pRoomInfo = g_pMain->m_MonsterDrakiTowerList.GetData(i);
			if (pRoomInfo == nullptr || pRoomInfo->m_tDrakiTowerStart) continue;

			SignRoom = pRoomInfo->m_tDrakiRoomID;
			break;
		}

		_DRAKI_TOWER_INFO* pRoomInfo = g_pMain->m_MonsterDrakiTowerList.GetData(SignRoom);
		if (pRoomInfo == nullptr || pRoomInfo->m_tDrakiTowerStart)
		{
			result << uint32(6);
			Send(&result);
			return;
		}

		//pRoomInfo->m_tDrakiTimer = UNIXTIME - (pRoomInfo->m_sDrakiStage == 1 ? 0 : pRoomInfo->m_sDrakiStage * 80);
		pRoomInfo->m_tDrakiTimer = UNIXTIME;
		pRoomInfo->m_tDrakiSubTimer = UNIXTIME + 300;

		if (m_bDrakiEnteranceLimit > 0)
			m_bDrakiEnteranceLimit--;

		pRoomInfo->m_sDrakiTempStage = m_bDrakiStage;
		pRoomInfo->m_sDrakiStage = EnterDungeon;
		pRoomInfo->m_sDrakiSubStage = 1;
		pRoomInfo->m_tDrakiTowerStart = true;
		pRoomInfo->m_tDrakiRoomCloseTimer = 7200;

		_DRAKI_TOWER_ROOM_USER_LIST* pDrakiRoomUser = new  _DRAKI_TOWER_ROOM_USER_LIST;
		pDrakiRoomUser->m_DrakiRoomID = SignRoom;
		pDrakiRoomUser->strUserID = GetName();

		if (!pRoomInfo->m_UserList.PutData(pDrakiRoomUser->m_DrakiRoomID, pDrakiRoomUser))
			delete pDrakiRoomUser;

		if (pRoomInfo->m_sDrakiStage == 1) pRoomInfo->m_isDrakiStageChange = true;

		int POSX = 0, POSZ = 0;
		switch (pRoomInfo->m_sDrakiStage)
		{
		case 1:
			POSX = 40; POSZ = 451;
			break;
		case 2:
			POSX = 78; POSZ = 58;
			break;
		case 3:
			POSX = 315; POSZ = 439;
			break;
		case 4:
			POSX = 304; POSZ = 271;
			break;
		case 5:
			POSX = 71; POSZ = 195;
			break;
		}

		if (ZoneChange(ZONE_DRAKI_TOWER, (float)POSX, (float)POSZ, SignRoom))
		{
			UserInOut(InOutType::INOUT_OUT);
			g_DBAgent.UpdateDrakiTowerLimitLastUpdate(GetName(), m_bDrakiEnteranceLimit);
		}
	}
	break;
	}
}

void CGameServerDlg::ReqBotLoadSaveData(Packet& pkt) {

	uint8 type = pkt.read<uint8>();
	uint8 zoneid = pkt.read<uint8>();
	uint16 id = pkt.read<uint16>();

	if (type == 1)
	{
		int sCount = 1;

		g_pMain->m_BotSaveDataArray.m_lock.lock();
		auto m_sMapBotListArray = g_pMain->m_BotSaveDataArray.m_UserTypeMap;
		g_pMain->m_BotSaveDataArray.m_lock.unlock();

		foreach(itr, m_sMapBotListArray)
		{
			CBot* pBot = g_pMain->GetBotPtr(itr->first);
			if (pBot == nullptr
				|| !pBot->isInGame()
				|| pBot->GetZoneID() != zoneid
				|| !pBot->isMerchanting())
				continue;

			_BOT_SAVE_DATA* pAuto = new _BOT_SAVE_DATA();
			for (int i = 0; i < MAX_MERCH_ITEMS; i++)
			{
				_MERCH_DATA* pMerch = &pBot->m_arMerchantItems[i];
				pAuto->sCount[i] = pMerch->sCount;
				pAuto->nNum[i] = pMerch->nNum;
				pAuto->sDuration[i] = pMerch->sDuration;
				pAuto->nPrice[i] = pMerch->nPrice;
				pAuto->IsSoldOut[i] = false;
				pAuto->isKc[i] = pMerch->isKC;
				pAuto->nSerialNum[i] = g_pMain->GenerateItemSerial();
			}

			pAuto->byZone = pBot->m_bZone;
			pAuto->Direction = pBot->m_sDirection;
			pAuto->fX = pBot->GetX();
			pAuto->fY = pBot->GetY();
			pAuto->fZ = pBot->GetZ();
			pAuto->Minute = 9999;
			pAuto->AdvertMessage = pBot->MerchantChat;


			pAuto->sMerchanType = 0;

			if (pBot->isBuyingMerchant())
				pAuto->sMerchanType = 1;

			pAuto->nAutoID = g_DBAgent.InsertBotMerchant(pAuto);
			sCount++;
		}

		CUser* pUser = GetUserPtr(id);
		if (pUser)
			g_pMain->SendHelpDescription(pUser, string_format("(%d) Save Merchant Bot", sCount));
	}
	else if (type == 2)
	{
		g_pMain->m_BotSaveDataArray.DeleteAllData();
		g_pMain->LoadSaveBotDataTable();
		CUser* pUser = GetUserPtr(id);
		if (pUser == nullptr)
			return;

		g_pMain->m_BotSaveDataArray.m_lock.lock();
		auto m_bBotSaveDataArray = g_pMain->m_BotSaveDataArray.m_UserTypeMap;
		g_pMain->m_BotSaveDataArray.m_lock.unlock();

		if (m_bBotSaveDataArray.size() > 0)
		{
			int nAutoID = 0;
			foreach(itr, m_bBotSaveDataArray)
			{
				_BOT_SAVE_DATA* pAuto = itr->second;
				if (pAuto == nullptr)
				{
					if (pUser)g_pMain->SendHelpDescription(pUser, string_format("### Error Nullptr ###", nAutoID));
					return;
				}

				int16 Direction = pAuto->Direction;

				float fX = pAuto->fX;
				float fZ = pAuto->fZ;
				float fY = pAuto->fY;
				nAutoID = itr->first;

				if (!g_pMain->SpawnUserBot(9999, zoneid, fX, fY, fZ, 50, 1, Direction, nAutoID))
				{
					if (pUser)g_pMain->SendHelpDescription(pUser, "### Bot Bulunamadi ###");
					continue;
				}
			}
		}
	}
}

void CGameServerDlg::ReqGmCommandLetterGiveItem(Packet& pkt)
{

	uint32 nItemID, sExpirationTime;
	uint16 sCount;
	uint8 sZoneID;
	pkt >> nItemID >> sZoneID >> sCount >> sExpirationTime;

	auto pData = GetItemPtr(nItemID);
	if (pData.isnull())
		return;

	std::string Subject = "Item", Message = "Gift", SenderName = "Admin";
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame() || (pUser->GetZoneID() != sZoneID && sZoneID))
			continue;

		_ITEM_DATA pItem{};
		pItem.nNum = nItemID;
		pItem.nSerialNum = g_pMain->GenerateItemSerial();
		pItem.sCount = sCount;
		pItem.sDuration = pData.m_sDuration;
		pItem.nExpirationTime = sExpirationTime;
		pItem.sRemainingRentalTime = 0;
		pItem.bFlag = ITEM_FLAG_NONE;
		g_DBAgent.SendLetter(SenderName, pUser->GetName(), Subject, Message, 2, &pItem, 0);
	}
}
void CGameServerDlg::ReqHandleDatabasRequest(Packet& pkt)
{
	uint8 opcode;
	pkt >> opcode;

	switch ((ProcDbServerType)opcode)
	{
	case ProcDbServerType::CollectionRaceStart:
		ReqCollectionRaceStart(pkt);
		break;
	case ProcDbServerType::GmCommandGiveLetterItem:
		ReqGmCommandLetterGiveItem(pkt);
		break;
	case ProcDbServerType::BotSaveLoad:
		ReqBotLoadSaveData(pkt);
		break;
	case ProcDbServerType::LotteryReward:
		ReqLotteryReward(pkt);
		break;
	case ProcDbServerType::DrakiTowerLimitReset:
		g_DBAgent.CDBAgent::UpdateDaysDrakiTowerLimit();
		break;
	case ProcDbServerType::UpdateSiegeWarfareDb:
	{
		g_DBAgent.UpdateSiegeWarfareDB(g_pMain->pSiegeWar.nMoradonTax, g_pMain->pSiegeWar.nDellosTax, g_pMain->pSiegeWar.nDungeonCharge);
		//printf("Database UpdateSiegeWarfareDB Save Process Success \n");
	}
	break;
	case ProcDbServerType::UpdateKnights:
	{
		foreach_stlmap(itr, g_pMain->m_KnightsArray)
		{
			CKnights* pKnights = itr->second;
			if (pKnights == nullptr) continue;
			pKnights->CKnights::UpdateClanGrade();
			g_DBAgent.KnightsSave(pKnights->GetID(), pKnights->m_byFlag, pKnights->m_nClanPointFund);
		}
		//printf("Database UpdateKnights Save Process Success \n");
	}
	break;
	case ProcDbServerType::ResetKnights:
	{
		uint16 sClanID;
		pkt >> sClanID;

		CKnights* pKnights = g_pMain->GetClanPtr(sClanID);
		if (pKnights != nullptr)
		{
			pKnights->m_sCape = -1; memset(&pKnights->m_Image, 0, sizeof(pKnights->m_Image));
			pKnights->m_bCapeR = 0; pKnights->m_bCapeG = 0; pKnights->m_bCapeB = 0;
			pKnights->m_sMarkVersion = 0; pKnights->m_sMarkLen = 0;
			g_DBAgent.KnightsReset(pKnights->GetID());
			;
		}
	}
	break;
	case ProcDbServerType::UpdateKingSystemDb:
	{
		uint8 m_byNation; uint32 m_nNationalTreasury, m_nTerritoryTax;
		pkt >> m_byNation >> m_nNationalTreasury >> m_nTerritoryTax;

		CKingSystem* pKingDB = g_pMain->m_KingSystemArray.GetData(m_byNation);
		if (pKingDB != nullptr)
		{
			g_DBAgent.UpdateKingSystemDB(m_byNation, m_nNationalTreasury, m_nTerritoryTax);
			//printf("Database UpdateKingSystemDB Save Process Success \n");
		}
	}
	break;
	case ProcDbServerType::ReloadSymbolAndKnights:
		LoadUserRankings();
		m_KnightsRatingArray[KARUS_ARRAY].DeleteAllData();
		m_KnightsRatingArray[ELMORAD_ARRAY].DeleteAllData();
		LoadKnightsRankTable(false, true);
		break;
	case ProcDbServerType::ResetLoyalty:
		g_DBAgent.ResetLoyaltyMonthly();
		printf("Database ResetLoyaltyMonthly Save Process Success \n");
		break;
	case ProcDbServerType::BotRegion:
	{
		std::string accountid; uint8 type;
		pkt >> type >> accountid;

		if (accountid.empty())
			return;

		if (type)
			g_DBAgent.InsertCurrentUser(accountid, accountid);
		else
			g_DBAgent.RemoveCurrentUser(accountid);
	}
	break;
		case ProcDbServerType::king_selection:
	{
		ReloadKnightAndUserRanks(false);
		king_loyaltyselection();
		ResetLoyaltyMonthly();
		king_selectreq = false;
		printf("king_selection paketi geldi\n");
	}
	break;
	default:
		printf("ReqHandleDatabasRequest unhandled opcode %d \n", opcode);
		break;
	}
}

void CUser::Addletteritem(_LINFO pgift)
{
	_ITEM_DATA pItem{};
	pItem.nNum = pgift.itemid;
	pItem.nSerialNum = g_pMain->GenerateItemSerial();
	pItem.sCount = pgift.scount;
	pItem.sDuration = pgift.duration;
	pItem.nExpirationTime = pgift.expiration;
	g_DBAgent.SendLetter(GetName(), GetName(), pgift.subject, pgift.message, 2, &pItem, 0);
}

void CUser::ReqSendLetterItem(Packet& pkt)
{
	uint8 type = pkt.read<uint8>();

	bool lettercheck = false;
	std::vector<_LINFO> mlist;

	switch ((lettersendtype)type)
	{
	case lettersendtype::cindirella:
	{
		uint16 rankid = pkt.read<uint16>();
		if (rankid > 199) return;
		auto p = g_pMain->pCindWar.pReward[rankid];
		std::string sender = "Fun Class", subject = "Fun Class Event reward item.";
		for (int i = 0; i < 10; i++)
		{
			if (p.itemid[i] && p.itemcount[i])
			{
				mlist.emplace_back(_LINFO(p.itemid[i], p.itemcount[i], p.itemduration[i], p.itemexpiration[i], subject, sender));
			}
		}
		foreach(pgift, mlist) { Addletteritem(*pgift); if (!lettercheck) lettercheck = true; }
	}
	break;
	}

	if (lettercheck) ReqLetterUnread();
}


void CUser::ReqHandleUserDatabaseRequest(Packet& pkt)
{
	uint8 opcode;
	pkt >> opcode;
	switch ((ProcDbType)opcode)
	{
	case ProcDbType::Letter:
		ReqSendLetterItem(pkt);
		break;
	case ProcDbType::DailyQuestReward:
		ReqDailyQuestSendReward(pkt);
		break;
	case ProcDbType::SaveSheriffReport:
	{
		string strReportedName, strReason;
		SheriffArray::iterator itr = g_pMain->m_SheriffArray.find(strReportedName);
		if (itr == g_pMain->m_SheriffArray.end())
		{
			tm Days;
			std::stringstream Date;
			time_t timer = time(0);
			Days = *localtime(&timer);
			Date << Days.tm_year - 100 << "/" << 1 + Days.tm_mon << "/" << Days.tm_mday << " "
				<< Days.tm_hour << ":" << Days.tm_min << ":" << Days.tm_sec;

			_SHERIFF_STUFF* pSheriffReport = new _SHERIFF_STUFF();
			pSheriffReport->reportedName = strReportedName;
			pSheriffReport->ReportSheriffName = GetName();
			pSheriffReport->crime = strReason;
			pSheriffReport->m_VoteYes++;
			pSheriffReport->SheriffYesA = GetName();
			pSheriffReport->m_Date = Date.str().c_str();

			g_pMain->m_SheriffArray.insert(std::make_pair(strReportedName, pSheriffReport));
			g_DBAgent.SaveSheriffReports(pSheriffReport->reportedName, pSheriffReport->ReportSheriffName, pSheriffReport->crime, pSheriffReport->m_VoteYes, pSheriffReport->SheriffYesA, pSheriffReport->m_VoteNo, pSheriffReport->m_Date);
		}
	}
	break;
	case ProcDbType::UpdateSheriffTable:
	{
		uint8 Type, Yes, No; string reportedName, IsEmptySheriffName;
		pkt >> Type >> reportedName >> Yes >> No >> IsEmptySheriffName;
		switch (Type)
		{
		case 1:
			g_DBAgent.UpdateSheriffTable(1, reportedName, Yes, No, IsEmptySheriffName);
			break;
		case 2:
			g_DBAgent.UpdateSheriffTable(2, reportedName, Yes, No, IsEmptySheriffName);
			break;
		}
	}
	break;
	case ProcDbType::UserReportDbSave:
	{
		std::string UserID, ReportType, SubJect, Message;
		pkt >> UserID >> ReportType >> SubJect >> Message;
		if (UserID.empty() || ReportType.empty() || SubJect.empty() || Message.empty()) return;
		g_DBAgent.UpdateReportUser(UserID, ReportType, SubJect, Message);
	}
	break;
	case ProcDbType::UpdateAccountKnightCash:
	{
		int32 cashamount = pkt.read<int32>();
		int32 tlamount = pkt.read<int32>();
		if (!cashamount && !tlamount) return;

		short id;
		pkt >> id;

		std::string log = ""; bool twon = false;
		if (cashamount && tlamount)
		{
			log.append(string_format("%d TL and %d Cash has been loaded into your account.", tlamount, cashamount));
			twon = true;
		}

		if (!twon && cashamount)
			log.append(string_format("%d Cash ", cashamount));
		else if (!twon && tlamount)
			log.append(string_format("%d TL ", tlamount));

		if (!twon) log.append("has been loaded into your account.");

		g_pMain->SendHelpDescription(this, log.c_str());

		m_nKnightCash += cashamount;
		m_nTLBalance += tlamount;

		g_DBAgent.UpdateAccountKnightCash(this);

		Packet newpkt(XSafe, uint8(XSafeOpCodes::KCUPDATE));
		newpkt << m_nKnightCash << m_nTLBalance;
		Send(&newpkt);
		SendItemMove(1, 2);

		if (id != -1)
		{
			auto* pUser = g_pMain->GetUserPtr(id);
			if (pUser) g_pMain->SendHelpDescription(pUser, string_format("Current Player Cash: %d - TL: %d", m_nKnightCash, m_nTLBalance));
		}
	}
	break;
	case ProcDbType::UpdateKnightCash:
		g_DBAgent.UpdateKnightCash(this);
		break;
	case ProcDbType::ClanPremium:
		g_DBAgent.UpdateClanPremiumServiceUser(this);
		break;
	case ProcDbType::ClanBankSave:
	{
		std::string ClanName;
		pkt >> ClanName;
		g_DBAgent.UpdateClanWarehouseData(ClanName, this);
	}
	break;
	case ProcDbType::AccountInfoSave:
	{
		std::string txt_email, txt_phone, txt_seal, txt_otp;
		pkt >> txt_email >> txt_phone >> txt_seal >> txt_otp;

		uint8 Result = g_DBAgent.AccountInfoSave(GetAccountName(), GetName(), txt_email, txt_phone, txt_otp, txt_seal);
		Packet test(XSafe, uint8(XSafeOpCodes::ACCOUNT_INFO_SAVE));
		test << uint8(2);
		if (Result != 1)
		{
			test << uint8(0);
			Send(&test);
			return;
		}
		test << uint8(1);
		Send(&test);
	}
	break;
	case ProcDbType::AccountInfoShow:
	{
		uint8 Active = g_DBAgent.AccountInfoShow(GetAccountName());
		if (Active != 1)
		{
			Packet send(XSafe, uint8(XSafeOpCodes::ACCOUNT_INFO_SAVE));
			send << uint8(1);
			Send(&send);
		}
	}
	break;
	case ProcDbType::ReturnGiveLetterItem:
	{
		_ITEM_TABLE pTable = _ITEM_TABLE();
		foreach_stlmap(itr, g_pMain->m_KnightReturnLetterGiftListArray)
		{
			if (itr->second == nullptr)
				continue;

			_ITEM_DATA pItem;
			pItem.nNum = itr->second->m_iItemNum;
			pTable = g_pMain->GetItemPtr(pItem.nNum);
			if (pTable.isnull())
				continue;

			pItem.nSerialNum = g_pMain->GenerateItemSerial();
			pItem.sCount = itr->second->sCount;
			pItem.sDuration = pTable.m_sDuration;
			g_DBAgent.SendLetter(itr->second->strSender, GetName(), itr->second->strSubject, itr->second->strMessage, 2, &pItem, 0);
		}
		ReqLetterUnread();
	}
	break;
	case ProcDbType::CollectionRaceReward:
	{
		uint32 itemid; uint16 itemcount;
		pkt >> itemid >> itemcount;

		_ITEM_TABLE itemdata = g_pMain->GetItemPtr(itemid);
		if (itemdata.isnull())
			return;

		WareHouseAddItemProcess(itemid, itemcount);

		/*std::string SendName = "Collection Race", Subejct = "Collection Race Hediyesi", message = "Collection Race eventinden kazanılan itemleriniz Inventory'nizde yer olmadığı için kalan ödüller letterden gönderilmiştir.";
		_ITEM_DATA pItem{};
		pItem.nNum = itemid;
		pItem.sCount = itemcount;
		pItem.sDuration = itemdata.m_sDuration;
		pItem.nSerialNum = g_pMain->GenerateItemSerial();
		g_DBAgent.SendLetter(SendName, GetName(), Subejct, message, 2, &pItem, 0);
		ReqLetterUnread();*/
	}
	break;
	case ProcDbType::PusGiftSendLetter:
	{
		uint32 itemid, itemprice; std::string name;
		pkt >> itemid >> itemprice >> name;

		_ITEM_TABLE itemdata = g_pMain->GetItemPtr(itemid);
		if (itemdata.isnull())
		{
			g_pMain->SendHelpDescription(this, "Your request to send a gift failed. Invalid exist item.");
			return;
		}

		CUser* pUser = g_pMain->GetUserPtr(name, NameType::TYPE_CHARACTER);
		if (!pUser || !pUser->isInGame())
		{
			g_pMain->SendHelpDescription(this, "The player is not in the game.");
			return;
		}

		m_nKnightCash -= itemprice;
		g_pMain->SendHelpDescription(this, string_format("Congratulations, You have gift purchased %s for %d Knight Cash.", itemdata.m_sName.c_str(), itemprice).c_str());

		Packet result(XSafe);
		result << uint8(XSafeOpCodes::CASHCHANGE) << uint32(m_nKnightCash);
		Send(&result);

		g_DBAgent.UpdateKnightCash(this);

		_ITEM_DATA pItem{};
		pItem.nNum = itemid;
		pItem.nSerialNum = g_pMain->GenerateItemSerial();
		pItem.sCount = 1;
		pItem.sDuration = itemdata.m_sDuration;
		g_DBAgent.SendLetter(GetName(), name, std::string("Pus Gift"), std::string("Pus Gift"), 2, &pItem, 0);
	}
	break;
	case ProcDbType::DrakiTowerUpdate:
	{
		uint16 Room;  uint32 Time; uint8 bIndex;
		pkt >> Room >> Time >> bIndex;
		g_DBAgent.UpdateDrakiTowerData(Time, bIndex, GetName());
	}
	break;
	case ProcDbType::KickOutAllUser:
		Disconnect();
		break;
	case ProcDbType::ItemRemove:
	{
		uint8 subcode = pkt.read<uint8>();
		if (subcode)
		{
			uint64 serialnum = pkt.read<uint64>();
			if (!serialnum) return;
			g_DBAgent.DeleteTrashItem(this, serialnum);
		}
		else
		{
			uint32 itemid, itemcount, itemdeletetime; uint16 duration; uint8 bflag; uint64 serialnum;
			pkt >> itemid >> itemcount >> duration >> serialnum >> bflag >> itemdeletetime;
			if (!itemid || !itemcount) return;
			g_DBAgent.InsertTrashItemList(this, itemid, itemcount, duration, itemdeletetime, serialnum, bflag);
		}
	}
	break;
	case ProcDbType::ClanNts:
		ReqClanNts();
		break;
	case ProcDbType::NtsCommand:
		ReqNtsCommand(pkt);
		break;
	case ProcDbType::CheatLog:
		g_DBAgent.InsertCheatLog(this);
		break;
	default:
		printf("ReqHandleUserDatabaseRequest unhandled opcode %d\n", opcode);
		break;
	}
}

void CUser::ReqNtsCommand(Packet& pkt)
{

	std::string uId;
	pkt >> uId;

	CUser* pUser = g_pMain->GetUserPtr(uId, NameType::TYPE_CHARACTER);
	if (pUser == nullptr || !pUser->isInGame())
		return g_pMain->SendHelpDescription(this, "Error Nts: User is not online");

	if (pUser->isTransformed())
		return g_pMain->SendHelpDescription(this, "Error Nts: delete for nation change in transform scroll");

	if (pUser->isInClan())
		return g_pMain->SendHelpDescription(this, string_format("Error Nts: %s is in Clan!", pUser->GetName().c_str()));

	_NATION_DATA pdata[4];
	for (int i = 0; i < 4; i++) pdata[i] = _NATION_DATA();

	std::string strCharID[4];
	for (int i = 0; i < 4; i++) strCharID[i] = "";
	if (!g_DBAgent.GetAllCharID(pUser->GetAccountName(), strCharID[0], strCharID[1], strCharID[2], strCharID[3]))
		return g_pMain->SendHelpDescription(this, "Error Nts: 0!");

	CKingSystem* pKingSystem = g_pMain->m_KingSystemArray.GetData(pUser->GetNation());
	if (!pKingSystem) return;

	for (int i = 0; i < 4; i++)
	{
		if (strCharID[i].length() > 0 && STRCASECMP(pKingSystem->m_strKingName.c_str(), strCharID[i].c_str()) == 0)
			return g_pMain->SendHelpDescription(this, string_format("Error Nts: %s is King!", strCharID[i].c_str()));
	}

	uint8 nnum = 0;
	for (int i = 0; i < 4; i++)
	{
		if (strCharID[i].length() > 0 && !g_DBAgent.GetAllCharInfo(strCharID[i], pdata[nnum++]))
			return;
	}

	if (!nnum)
		return;

	nnum = 0;
	for (int i = 0; i < 4; i++)
	{
		if (pdata[i].isnull() || pdata[i].bCharName.empty())
			continue;

		if (pdata[i].sClanID > 0 && pdata[i].sClanID != GetClanID())
			return g_pMain->SendHelpDescription(this, string_format("Nts: %s is in clan", pdata[i].bCharName.c_str()));

		nnum++;
	}

	if (!nnum)
		return;

	Nation newnation = Nation::KARUS;
	if (pUser->GetNation() == (uint8)Nation::KARUS) newnation = Nation::ELMORAD;

	for (int i = 0; i < 4; i++)
	{
		if (pdata[i].isnull())
			continue;

		uint8 newclass = newnation == Nation::ELMORAD ? pdata[i].sClass + 100 : pdata[i].sClass - 100;
		uint8 newrace = g_pMain->GetCntNewRace(newclass, newnation);

		if (!newrace)
			continue;

		if (pUser->GetName() == pdata[i].bCharName)
		{
			pUser->m_bTitle = 0;
			pUser->m_bRace = newrace;
			pUser->m_sClass = newclass;
			pUser->m_bNation = (uint8)newnation;
			if (pUser->GetHealth() < (pUser->GetMaxHealth() / 2)) HpChange(pUser->GetMaxHealth());
			pUser->SendMyInfo();
			pUser->UserInOut(InOutType::INOUT_OUT);
			pUser->SetRegion(GetNewRegionX(), GetNewRegionZ());
			pUser->UserInOut(InOutType::INOUT_WARP);
			g_pMain->UserInOutForMe(pUser);
			NpcInOutForMe();
			g_pMain->MerchantUserInOutForMe(pUser);
			pUser->ResetWindows();
			pUser->InitType4();
			pUser->RecastSavedMagic();
		}
		g_DBAgent.UpdateNtsCommand(pUser->GetAccountName(), pdata[i].bCharName, newrace, newclass, (uint8)newnation);
	}
	g_pMain->SendHelpDescription(this, string_format("nts operation successful for person %s !", pUser->GetName().c_str()));
}

void CUser::ReqItemUpgrade(Packet& pkt)
{
	uint8 bOpcode = pkt.read<uint8>();

	switch (ItemUpgradeOpcodes(bOpcode))
	{
	case ITEM_SEAL:
		ReqSealItem(pkt);
		break;
	case ITEM_CHARACTER_SEAL:
		ReqCharacterSealProcess(pkt);
		break;
	case ItemUpgradeOpcodes::PET_HATCHING_TRANSFROM:
		ReqHatchingTransforms(pkt);
		break;
	default:
		printf("ReqItemUpgrade unhandled database packet %d \n", bOpcode);
		TRACE("ReqItemUpgrade unhandled database packet %d \n", bOpcode);
		break;
	}
}

void CUser::ReqHatchingTransforms(Packet& pkt)
{
	enum results
	{
		Failed = 1,
		InvalidName = 2,
		Familiarcannot = 3,
		LimitExceeded = 4,
	};

	//return;//Pet Fonksiyonu kapatıldı.
	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::PET_HATCHING_TRANSFROM));

	int32 nItemID; int8 bPos; std::string strKaulName;
	pkt >> nItemID >> bPos >> strKaulName;

	_ITEM_TABLE pTable = g_pMain->GetItemPtr(PET_START_ITEM);
	if (pTable.isnull())
	{
		result << uint8(0) << uint8(Failed);
		Send(&result);
		return;
	}

	_ITEM_DATA* pItem = &m_sItemArray[SLOT_MAX + bPos];
	if (pItem == nullptr
		|| pItem->nNum != nItemID
		|| pItem->isBound()
		|| pItem->isDuplicate()
		|| pItem->isRented()
		|| pItem->isSealed())
	{
		result << uint8(0) << uint8(Failed);
		Send(&result);
		return;
	}

	PET_INFO* pPetInfo = g_pMain->m_PetInfoSystemArray.GetData(PET_START_LEVEL);
	if (pPetInfo == nullptr)
	{
		result << uint8(0) << uint8(Failed);
		Send(&result);
		return;
	}

	uint64 nSerialID = g_pMain->GenerateItemSerial();
	uint32 nStatus = g_DBAgent.CreateNewPet(nSerialID, PET_START_LEVEL, strKaulName);
	if (nStatus < 10)
	{
		result << uint8(nStatus)
			<< pTable.m_iNum
			<< bPos << uint32(0)
			<< strKaulName
			<< uint8(pTable.m_sDamage)
			<< uint8(1)
			<< uint16(0)
			<< uint16(9000);
		Send(&result);
		return;
	}

	memset(pItem, 0x00, sizeof(_ITEM_DATA));
	pItem->nNum = pTable.m_iNum;
	pItem->nSerialNum = nSerialID;
	pItem->sCount = 1;
	pItem->sDuration = pTable.m_sDuration;

	PET_DATA* pPet = new PET_DATA();
	pPet->bLevel = PET_START_LEVEL;
	pPet->nIndex = (uint32)nStatus;
	pPet->sHP = pPetInfo->PetMaxHP;
	pPet->sMP = pPetInfo->PetMaxMP;
	pPet->sSatisfaction = 9000;
	pPet->nExp = 0;
	pPet->strPetName = strKaulName;
	memset(pPet->sItem, 0x00, sizeof(pPet->sItem));
	pPet->info = pPetInfo;

	//g_pMain->m_PetSystemlock.lock();
	g_pMain->m_PetDataSystemArray.insert(std::make_pair(pItem->nSerialNum, pPet));
	//g_pMain->m_PetSystemlock.unlock();
	result << uint8(1)
		<< pTable.m_iNum
		<< bPos
		<< pPet->nIndex
		<< strKaulName
		<< uint8(pTable.m_sDamage)
		<< uint8(1)
		<< uint16(0)
		<< pPet->sSatisfaction;
	Send(&result);
}

void CUser::ReqRemoveCurrentUser(Packet& pkt)
{
	string m_strAccountID;

	pkt >> m_strAccountID;
	// if there was no error deleting to CURRENTUSER...
	if (!g_DBAgent.RemoveCurrentUser(m_strAccountID))
		printf("An error has occurred during deletion of user from CURRENTUSER table.\n");
}

void CUser::ReqAccountLogIn(Packet& pkt)
{
	bool disconnect;
	string strPasswd, strAccountID;
	pkt >> strPasswd >> strAccountID >> disconnect;

	if (strPasswd.empty() || strAccountID.empty()
		|| !m_strAccountID.empty() || m_bSelectedCharacter)
		return;

	if (strAccountID.size() > MAX_ID_SIZE || strPasswd.size() > MAX_PW_SIZE)
		return;

	if (!g_DBAgent.LoadTBUserData(strAccountID, this))
		return goDisconnect("I was thrown when tb_user was getting my info.", __FUNCTION__);

	if (pUserTbInfo.accountpass.empty() || pUserTbInfo.accountpass != strPasswd)
		return goDisconnect(string_format("My account password is incorrect or does not match. strAccountID=%s", strAccountID.c_str()), __FUNCTION__);

	std::string AccountID = strAccountID;
	STRTOUPPER(AccountID);

	if (disconnect)
	{
		SendLoginFailed(-1);
		CUser* pUser = g_pMain->GetUserPtr(strAccountID, NameType::TYPE_ACCOUNT);
		if (pUser && pUser->GetSocketID() != GetSocketID())
		{
			pUser->goDisconnect(string_format("the process of dismissing the account from the game while choosing a server. strAccountID=%s", strAccountID.c_str()), __FUNCTION__);

			g_pMain->m_accountNameLock.lock();
			auto find = g_pMain->m_accountNameMap.find(AccountID);
			if (find != g_pMain->m_accountNameMap.end())
				g_pMain->m_accountNameMap.erase(AccountID);
			g_pMain->m_accountNameLock.unlock();
			return;
		}
	}

	Packet result(WIZ_LOGIN);
	int8 dbresult = g_DBAgent.AccountLogin(strAccountID, strPasswd);
	result << dbresult;

	if (dbresult >= 0 && dbresult <= 2)
	{
		CUser* pUser = g_pMain->GetUserPtr(strAccountID, NameType::TYPE_ACCOUNT);
		if (pUser)
		{

			std::string mapAccountID = pUser->m_strAccountID;
			STRTOUPPER(mapAccountID);

			if (pUser->GetSocketID() != GetSocketID() && AccountID == mapAccountID)
			{
				SendLoginFailed(-1);
				pUser->goDisconnect(string_format("the process of dismissing the account from the game while choosing a server. strAccountID=%s", strAccountID.c_str()), __FUNCTION__);

				g_pMain->m_accountNameLock.lock();
				auto find = g_pMain->m_accountNameMap.find(AccountID);
				if (find != g_pMain->m_accountNameMap.end())
					g_pMain->m_accountNameMap.erase(AccountID);
				g_pMain->m_accountNameLock.unlock();
				return;
			}
		}

		m_strAccountID = strAccountID;
		g_pMain->AddAccountName(this);

		result << uint32(0);
		Send(&result);
		return;
	}

	m_strAccountID.clear();
	Send(&result);
}

void CUser::ReqSelectNation(Packet& pkt)
{
	Packet result(WIZ_SEL_NATION);
	uint8 bNation = pkt.read<uint8>(), bResult;

	bResult = g_DBAgent.NationSelect(m_strAccountID, bNation) ? bNation : 0;
	result << bResult;
	Send(&result);
}

void CUser::ReqAllCharInfo(Packet& pkt)
{
	uint8 oPcode;
	pkt >> oPcode;
	Packet result(WIZ_ALLCHAR_INFO_REQ);
	switch (oPcode)
	{
	case AllCharInfoOpcode:
	{
		string strCharID1, strCharID2, strCharID3, strCharID4;
		result << uint8(oPcode) << uint8(1);

		g_DBAgent.GetAllCharID(m_strAccountID, strCharID1, strCharID2, strCharID3, strCharID4);
		g_DBAgent.LoadCharInfo(strCharID1, result);
		g_DBAgent.LoadCharInfo(strCharID2, result);
		g_DBAgent.LoadCharInfo(strCharID3, result);
		g_DBAgent.LoadCharInfo(strCharID4, result);
		Send(&result);
	}
	break;
	case AlreadyCharName:
	{
		string sOldCharID, sNewCharID;
		pkt >> sOldCharID >> sNewCharID;

		ServerUniteSelectingCharNameErrorOpcode NameChangeResult;
		uint8 Result = g_DBAgent.CharacterSelectLoginChecking(GetAccountName(), sOldCharID);
		if (Result == 1)
			NameChangeResult = g_DBAgent.UpdateSelectingCharacterName(GetAccountName(), sOldCharID, sNewCharID);
		else
			printf("OldNick %s - NewNick %s paket acigiyla nick degismeye calisiyor\n", sOldCharID.c_str(), sNewCharID.c_str());

		result << uint8(AlreadyCharName);

		switch (NameChangeResult)
		{
		case ServerUniteSelectingCharNameErrorOpcode::CannotCharacterID:
			result << uint8(ServerUniteSelectingCharNameErrorOpcode::CannotCharacterID);
			Send(&result);
			break;
		case ServerUniteSelectingCharNameErrorOpcode::CharacterIDSuccesfull:
			result << uint8(ServerUniteSelectingCharNameErrorOpcode::CharacterIDSuccesfull);
			Send(&result);
		default:
			result << uint8(ServerUniteSelectingCharNameErrorOpcode::CannotCharacterID);
			Send(&result);
			break;
		}
	}
	break;
	default:
		break;
	}
}

void CUser::ReqChangeHair(Packet& pkt)
{
	Packet result(WIZ_CHANGE_HAIR);
	string strUserID;
	uint32 nHair;
	uint8 bOpcode, bFace;
	pkt.SByte();
	pkt >> bOpcode >> strUserID >> bFace >> nHair;

	if (bOpcode == 0)
	{
		int8 bResult = g_DBAgent.ChangeHair(m_strAccountID, strUserID, bOpcode, bFace, nHair);

		result << uint8(bResult);
		Send(&result);
	}
	else if (bOpcode == 1)
	{
		if (!isInGame()
			|| !CheckExistItem(ITEM_MAKE_OVER))
		{
			result << uint8(1);
			Send(&result);
			return;
		}

		int8 bResult = g_DBAgent.ChangeHair(m_strAccountID, strUserID, bOpcode, bFace, nHair);
		if (bResult == 0)
		{
			RobItem(ITEM_MAKE_OVER);

			m_bFace = bFace;
			m_nHair = nHair;
		}

		result << bResult;
		Send(&result);
	}
}

void CUser::ReqCreateNewChar(Packet& pkt)
{
	string strCharID;
	uint32 nHair;
	uint16 sClass;
	uint8 bCharIndex, bRace, bFace, bStr, bSta, bDex, bInt, bCha;
	pkt >> bCharIndex >> strCharID >> bRace >> sClass >> bFace >> nHair >> bStr >> bSta >> bDex >> bInt >> bCha;

	uint8 sonuc = g_DBAgent.CreateNewChar(m_strAccountID, bCharIndex, strCharID, bRace, sClass, nHair, bFace, bStr, bSta, bDex, bInt, bCha);
	Packet result(WIZ_NEW_CHAR);
	result << sonuc;
	if (sonuc == 0)
	{
		uint16 newClass = 0, newClanID = 0; uint8 newLevel = 0, newFame = 0; uint32 newLoyalty = 0, newMontlyLoyalty = 0;;
		bool ismykodominik = g_DBAgent.LoadAutoClanInfo(strCharID, newClass, newLevel, newClanID, newFame, newLoyalty, newMontlyLoyalty);
		if ((ismykodominik && newFame == 5) && (newClanID == 1 || newClanID == 15001))
		{
			CKnightsManager::AddKnightsUser(newClanID, m_strAccountID, strCharID, std::string(""), GetFame(), sClass, newLevel, (uint32)UNIXTIME, newLoyalty, newMontlyLoyalty);
			CKnightsManager::AddUserDonatedNP(newClanID, strCharID, 0, false);
		}
		g_DBAgent.LoadNewCharSet(strCharID, sClass);
		g_DBAgent.LoadNewCharValue(strCharID, sClass);

		if (g_pMain->pServerSetting.premiumID && g_pMain->pServerSetting.premiumTime)
			GivePremium(static_cast<uint8>(g_pMain->pServerSetting.premiumID), g_pMain->pServerSetting.premiumTime, true);
		newChar = true;
	}
	Send(&result);
}

void CUser::ReqDeleteChar(Packet& pkt)
{
	string strCharID, strSocNo;
	uint8 bCharIndex;
	pkt >> bCharIndex >> strCharID >> strSocNo;

	Packet result(WIZ_DEL_CHAR);
	int8 retCode = g_DBAgent.DeleteChar(m_strAccountID, bCharIndex, strCharID, strSocNo);
	result << retCode << uint8(retCode ? bCharIndex : -1);
	Send(&result);
}

void CUser::ReqSelectCharacter(Packet& pkt)
{
	uint8 bInit;
	string strCharID, StrAccountID;
	pkt >> StrAccountID >> strCharID >> bInit;
	if (StrAccountID.empty() || strCharID.empty() || m_strAccountID.empty() || StrAccountID != m_strAccountID)
		return goDisconnect("The account name or account password is incorrect or invalid.", __FUNCTION__);

	if (!GetName().empty()) return;

	Packet result(WIZ_SEL_CHAR);
	bool dberror = false; std::string DatabaseProcName = "";

	if (!g_DBAgent.LoadUserData(m_strAccountID, strCharID, this))
	{
		dberror = true; DatabaseProcName = "LoadUserData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadTrashItemList(this))
	{
		dberror = true; DatabaseProcName = "LoadTrashItemList";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadPerksData(strCharID, this))
	{
		dberror = true; DatabaseProcName = "LoadPerksData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadPusRefund(this))
	{
		dberror = true; DatabaseProcName = "LoadPusRefund";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadWarehouseData(m_strAccountID, this))
	{
		dberror = true; DatabaseProcName = "LoadWarehouseData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadVipWarehouseData(m_strAccountID, this))
	{
		dberror = true; DatabaseProcName = "LoadVipWarehouseData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadPremiumServiceUser(m_strAccountID, this))
	{
		dberror = true; DatabaseProcName = "LoadPremiumServiceUser";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadSavedMagic(this))
	{
		dberror = true; DatabaseProcName = "LoadSavedMagic";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadGenieData(strCharID, this))
	{
		dberror = true; DatabaseProcName = "LoadGenieData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadQuestData(strCharID, this))
	{
		dberror = true; DatabaseProcName = "LoadQuestData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadAchieveData(this))
	{
		dberror = true; DatabaseProcName = "LoadAchieveData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadUserDailyRank(strCharID, this))
	{
		dberror = true; DatabaseProcName = "LoadUserDailyRank";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadUserSealExpData(this))
	{
		dberror = true; DatabaseProcName = "LoadUserSealExpData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadUserDrakiTowerData(this))
	{
		dberror = true; DatabaseProcName = "LoadUserDrakiTowerData";
		goto fail_continue;
	}

	if (!g_DBAgent.LoadUserReturnData(this))
	{
		dberror = true; DatabaseProcName = "LoadUserReturnData";
		goto fail_continue;
	}

fail_continue:
	if (dberror)
	{
		TRACE("Notice::[Character Selection Error Database:%s] %s | %s | %d \n", DatabaseProcName.c_str(), m_strAccountID.c_str(), strCharID.c_str(), GetSocketID());
		printf("Notice::[Character Selection Error Database:%s] %s | %s | %d \n", DatabaseProcName.c_str(), m_strAccountID.c_str(), strCharID.c_str(), GetSocketID());
		result << int8(CannotSelectingCharacter);
		Send(&result);
		return;
	}

	result << int8(g_DBAgent.SelectCharacterChecking(m_strAccountID, GetName())) << bInit;
	SelectCharacter(result);
}

void CUser::ReqShoppingMall(Packet& pkt)
{
	switch (pkt.read<uint8>())
	{
	case STORE_CLOSE:
		ReqLoadWebItemMall();
		break;
	case STORE_LETTER:
		ReqLetterSystem(pkt);
		break;
	case STORE_OPEN:
		ReqPusSessionCreate(pkt);
		break;
	}
}

void CUser::ReqSkillDataProcess(Packet& pkt)
{
	uint8 opcode = pkt.read<uint8>();
	if (opcode == SKILL_DATA_LOAD)
		ReqSkillDataLoad(pkt);
	else if (opcode == SKILL_DATA_SAVE)
		ReqSkillDataSave(pkt);
}

void CUser::ReqSkillDataLoad(Packet& pkt)
{
	Packet result(WIZ_SKILLDATA, uint8(SKILL_DATA_LOAD));
	if (!g_DBAgent.LoadSkillShortcut(result, this))
		result << uint16(0);

	Send(&result);
}

void CUser::ReqSkillDataSave(Packet& pkt)
{
	// Initialize our buffer (not all skills are likely to be saved, we need to store the entire 260 bytes).
	char buff[320]{};
	//memset(buff, 0, sizeof(buff));
	RtlSecureZeroMemory(buff, sizeof(buff));
	short sCount;

	// Read in our skill count
	pkt >> sCount;

	// Make sure we're not going to copy too much (each skill is 1 uint32).
	if ((sCount * sizeof(uint32)) > sizeof(buff))
		return;

	// Copy the skill data directly in from where we left off reading in the packet buffer
	memcpy(m_strSkillData, (char*)(pkt.contents() + pkt.rpos()), sCount * sizeof(uint32));
	m_strSkillCount = sCount;
	g_DBAgent.SaveSkillShortcut(this);
}

void CUser::ReqFriendProcess(Packet& pkt)
{
	uint8 opcode = 0;
	pkt >> opcode;

	switch (opcode)
	{
	case FRIEND_REQUEST:
		ReqRequestFriendList(pkt);
		break;
	case FRIEND_ADD:
		ReqAddFriend(pkt);
		break;
	case FRIEND_REMOVE:
		ReqRemoveFriend(pkt);
		break;
	default:
		printf("Friend Process unhandled packet %d", opcode);
		TRACE("Friend Process unhandled packet %d", opcode);
		break;
	}
}

void CUser::ReqRequestFriendList(Packet& pkt)
{
	Packet result(WIZ_FRIEND_PROCESS);
	std::vector<string> friendList;

	g_DBAgent.RequestFriendList(friendList, this);

	result << uint16(friendList.size());
	foreach(itr, friendList)
		result << (*itr);

	FriendReport(result);
}

void CUser::ReqAddFriend(Packet& pkt)
{
	Packet result(WIZ_FRIEND_PROCESS);
	string strCharID;
	int16 tid;

	pkt.SByte();
	pkt >> tid >> strCharID;

	FriendAddResult resultCode = g_DBAgent.AddFriend(GetSocketID(), tid);
	result.SByte();
	result << tid << uint8(resultCode) << strCharID;

	RecvFriendModify(result, FRIEND_ADD);
}

void CUser::ReqRemoveFriend(Packet& pkt)
{
	Packet result(WIZ_FRIEND_PROCESS);
	string strCharID;

	pkt.SByte();
	pkt >> strCharID;

	FriendRemoveResult resultCode = g_DBAgent.RemoveFriend(strCharID, this);
	result.SByte();
	result << uint8(resultCode) << strCharID;

	RecvFriendModify(result, FRIEND_REMOVE);
}

/**
* @brief	Handles clan cape update requests.
*
* @param	pkt	The packet.
*/
void CUser::ReqChangeCape(Packet& pkt)
{
	uint16 sClanID, sCapeID; uint32 Time;
	uint8 r, g, b, type;
	pkt >> type >> sClanID >> sCapeID >> r >> g >> b >> Time;

	switch (type)
	{
	case 1:
		g_DBAgent.KnightsCastellanCapeUpdate(sClanID, sCapeID, r, g, b, Time);
		break;
	case 2:
		g_DBAgent.KnightsCapeUpdate(sClanID, sCapeID, r, g, b);
		break;
	}
}

void CUser::ReqUserLogOut()
{
	if (m_bLevel == 0 || !m_deleted)
	{

		m_deleted = false;
		m_bCharacterDataLoaded = false;
		return;
	}

	if (isInGame())
	{
		// Cindirella/Fun Class icindeyken once orijinal karakter bilgisini geri yukle.
		// Eski sirada UserInfoStorage() once calistigi icin event class/skill/level bilgisi
		// veritabanina kaydedilebiliyordu.
		if (pCindWar.isEventUser() && g_pMain->isCindirellaZone(GetZoneID()))
			CindirellaLogOut(true);

		UserInfoStorage(false);
		UserInOut(InOutType::INOUT_OUT);
		BottomUserLogOut();
		PetOnDeath();
		GmListProcess(true);

		if (isInParty())
		{
			auto* pParty = g_pMain->GetPartyPtr(GetPartyID());
			if (pParty && isPartyLeader())
				PartyLeaderPromote(pParty->uid[1], pParty);

			PartyNemberRemove(GetSocketID(), pParty);
		}

		if (isInClan())
		{
			CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
			if (pKnights != nullptr)
			{

				pKnights->OnLogout(this);
				if (pKnights->isInAlliance())
					pKnights->OnLogoutAlliance(this);

				KnightsClanBuffUpdate(false, pKnights);
				g_DBAgent.UpdateClanWarehouseData(pKnights->GetName(), this);
			}
		}

		PlayerKillingRemovePlayerRank();
		ZindanWarKillingRemovePlayerRank();
		BorderDefenceRemovePlayerRank();
		ChaosExpansionRemovePlayerRank();
		BDWUserLogOut();
		TowerExitsFunciton();
		UpdateAchievePlayTime();
		PartyBBSUserLoqOut();

		if (m_ismsevent)
			TempleMonsterStoneItemExitRoom();

		if (isWantedUser())
			NewWantedEventLoqOut();

		if (g_pMain->pTempleEvent.ActiveEvent != -1)
		{
			TempleDisbandEvent(GetJoinedEvent());
			if (!g_pMain->pTempleEvent.isActive) TempleDisbandEvent(g_pMain->pTempleEvent.ActiveEvent);
		}

		if (isSoccerEventUser())
		{
			isEventSoccerEnd();
			if (g_pMain->m_TempleSoccerEventUserArray.GetData(GetSocketID()) != nullptr)
				g_pMain->m_TempleSoccerEventUserArray.DeleteData(GetSocketID());
		}

		if (isMeChatRoom(m_ChatRoomIndex)) ChatRoomKickOut(GetSocketID());
		if (hasRival()) RemoveRival();
		ResetWindows();
	}

	g_pMain->KillNpc(GetSocketID(), GetZoneID(), this);
	g_DBAgent.UpdateUser(GetName(), UserUpdateType::UPDATE_LOGOUT, this);
	g_DBAgent.UpdateQuestData(GetName(), this);
	g_DBAgent.UpdateGenieData(GetName(), this);
	g_DBAgent.UpdatePremiumServiceUser(this);
	g_DBAgent.UpdateAchieveData(this);
	g_DBAgent.UpdateUserSealExpData(this);
	g_DBAgent.UpdateUserReturnData(GetName(), this);
	g_DBAgent.UpdateWarehouseData(GetAccountName(), UserUpdateType::UPDATE_LOGOUT, this);
	g_DBAgent.UpdateVipWarehouseData(GetAccountName(), UserUpdateType::UPDATE_LOGOUT, this);
	g_DBAgent.UpdateSavedMagic(this);
	g_DBAgent.SaveDailyRank(GetName(), this);
	g_DBAgent.UpdateUserPerks(this);
	LogoutInsertLog();

	g_DBAgent.AccountLogout(GetAccountName());

	g_pMain->RemoveSessionNames(this);

	// this session can be used again.
	m_bCharacterDataLoaded = false;
	m_deleted = false;
}

void CUser::ReqSaveCharacter()
{
	g_DBAgent.UpdateUser(GetName(), UserUpdateType::UPDATE_PACKET_SAVE, this);
	g_DBAgent.UpdateQuestData(GetName(), this);
	g_DBAgent.UpdateGenieData(GetName(), this);
	g_DBAgent.UpdatePremiumServiceUser(this);
	g_DBAgent.UpdateAchieveData(this);
	g_DBAgent.UpdateUserSealExpData(this);
	g_DBAgent.UpdateUserReturnData(GetName(), this);
	g_DBAgent.UpdateWarehouseData(GetAccountName(), UserUpdateType::UPDATE_PACKET_SAVE, this);
	g_DBAgent.UpdateVipWarehouseData(GetAccountName(), UserUpdateType::UPDATE_PACKET_SAVE, this);
	g_DBAgent.UpdateSavedMagic(this);
	g_DBAgent.SaveDailyRank(GetName(), this);
	g_DBAgent.UpdateUserPerks(this);
}

void CKnightsManager::ReqKnightsAllMember(CUser* pUser, Packet& pkt)
{
	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_MEMBER_REQ));
	int nOffset;
	uint16 sClanID, sCount;

	pkt >> sClanID;

	CKnights* pKnights = g_pMain->GetClanPtr(pUser->GetClanID());
	if (pKnights == nullptr)
		return;

	result << uint8(1);
	nOffset = (int)result.wpos(); // store offset
	result << uint16(0) // placeholder for packet length 
		<< uint16(0); // placeholder for user count

	sCount = g_DBAgent.LoadKnightsAllMembers(sClanID, result);
	if (sCount > MAX_CLAN_USERS)
		return;

	pkt.put(nOffset, uint16(result.size() - 3));
	pkt.put(nOffset + 2, sCount);

	pUser->Send(&result);
}

void CUser::ReqSetLogInInfo(Packet& pkt)
{
	DateTime time;
	string strCharID, strServerIP, strClientIP;
	uint16 sServerNo;
	uint8 bInit;

	pkt >> strCharID >> strServerIP >> sServerNo >> strClientIP >> bInit;
	// if there was an error inserting to CURRENTUSER...
	if (!g_DBAgent.SetLogInInfo(m_strAccountID, strCharID, strServerIP, sServerNo, strClientIP, bInit))
		return goDisconnect("error.", __FUNCTION__);

	LoginInsertLog();
}

void CUser::BattleEventResult(Packet& pkt)
{
	string strMaxUserName;
	uint8 bType, bNation;

	pkt >> bType >> bNation >> strMaxUserName;
	g_DBAgent.UpdateBattleEvent(strMaxUserName, bNation);
}

/**
* @brief	Handles database requests for the King system.
*
* @param	pUser	The user making the request, if applicable.
* 					nullptr if not.
* @param	pkt  	The packet.
*/
void CKingSystem::HandleDatabaseRequest(CUser* pUser, Packet& pkt)
{
	switch (pkt.read<uint8>())
	{
	case KING_ELECTION:
		HandleDatabaseRequest_Election(pUser, pkt);
		break;

	case KING_TAX:
		HandleDatabaseRequest_Tax(pUser, pkt);
		break;

	case KING_EVENT:
		HandleDatabaseRequest_Event(pUser, pkt);
		break;

	case KING_IMPEACHMENT:
		break;

	case KING_NPC:
		break;

	case KING_NATION_INTRO:
		HandleDatabaseRequest_NationIntro(pUser, pkt);
		break;
	}
}


/**
* @brief	Handles database requests for King commands.
*
* @param	pUser	The user making the request, if applicable.
* 					nullptr if not.
* @param	pkt  	The packet.
*/
void CKingSystem::HandleDatabaseRequest_NationIntro(CUser* pUser, Packet& pkt)
{
	if (pUser == nullptr)
		return;

	std::string strKingName, strKingNotice;
	uint8 byNation = 0;
	uint32 ServerNo = 0;
	pkt >> ServerNo >> byNation >> strKingName >> strKingNotice;
	g_DBAgent.UpdateNationIntro(ServerNo, byNation, strKingName, strKingNotice);
}

/**
* @brief	Handles database requests for the election system.
*
* @param	pUser	The user making the request, if applicable.
* 					nullptr if not.
* @param	pkt  	The packet.
*/
void CKingSystem::HandleDatabaseRequest_Election(CUser* pUser, Packet& pkt)
{
	uint8 opcode;
	pkt >> opcode;

	switch (opcode)
	{
		// Special king system/election database requests
	case KING_ELECTION:
	{
		uint8 byNation, byType;
		pkt >> opcode >> byNation >> byType;
		switch (opcode)
		{
		case KING_ELECTION_UPDATE_STATUS: // 7
			g_DBAgent.UpdateElectionStatus(byType, byNation);
			break;

		case KING_ELECTION_UPDATE_LIST: // 6
		{
			bool bDelete;
			uint16 sKnights;
			uint32 nVotes = 0;
			string strNominee;

			pkt >> bDelete >> sKnights >> strNominee;
			g_DBAgent.UpdateElectionList(bDelete ? 2 : 1, byType, byNation, sKnights, nVotes, strNominee);
		}
		break;
		case KING_ELECTION_VOTE:
		{
			Packet result(WIZ_KING, uint8(KING_ELECTION));
			result << uint8(KING_ELECTION_POLL) << uint8(2);
			string strVoterAccountID, strVoterUserID, strNominee;
			int16 bResult = -3;
			pkt >> strVoterAccountID >> strVoterUserID >> strNominee;
			bResult = g_DBAgent.UpdateElectionVoteList(byNation, strVoterAccountID, strVoterUserID, strNominee);
			result << uint16(bResult);
			CUser* pVoter = g_pMain->GetUserPtr(strVoterUserID, NameType::TYPE_CHARACTER);
			if (pVoter != nullptr && pVoter->isInGame())
				pVoter->Send(&result);
		} return;
		case KING_ELECTION_GET_VOTE_RESULTS:
		{
			g_DBAgent.GetElectionResults(byNation);
		} return;
		}
	}
	break;
	case KING_ELECTION_NOMINATE:
	{
		if (pUser == nullptr)
			return;

		Packet result(WIZ_KING, uint8(KING_ELECTION));
		std::string strNominee;
		int16 resultCode;
		pkt >> strNominee;
		resultCode = g_DBAgent.UpdateCandidacyRecommend(pUser->m_strUserID, strNominee, pUser->GetNation());

		// On success, we need to sync the local list.
		if (resultCode >= 0)
		{
			CKingSystem* pData = g_pMain->m_KingSystemArray.GetData(pUser->GetNation());
			if (pData == nullptr)
				return;

			pData->InsertNominee(pUser->m_strUserID, strNominee, resultCode);
			result << opcode << int16(1);
			pUser->Send(&result);
			return;
		}
		result << opcode << resultCode;
		pUser->Send(&result);
	}
	break;

	case KING_ELECTION_NOTICE_BOARD:
	{
		pkt >> opcode;
		if (pUser == nullptr)
			return;

		if (opcode == KING_CANDIDACY_BOARD_WRITE)
		{
			string strNotice;
			pkt >> strNotice;
			g_DBAgent.UpdateCandidacyNoticeBoard(pUser->m_strUserID, pUser->GetNation(), strNotice);
		}
	}
	break;
	}
}

/**
* @brief	Handles database requests for King commands.
*
* @param	pUser	The user making the request, if applicable.
* 					nullptr if not.
* @param	pkt  	The packet.
*/
void CKingSystem::HandleDatabaseRequest_Event(CUser* pUser, Packet& pkt)
{
	uint8 opcode, byNation;
	pkt >> opcode >> byNation;

	switch (opcode)
	{
	case KING_EVENT_NOAH:
	case KING_EVENT_EXP:
	{
		uint8 byAmount, byDay, byHour, byMinute;
		uint16 sDuration;
		uint32 nCoins;
		pkt >> byAmount >> byDay >> byHour >> byMinute >> sDuration >> nCoins;

		g_DBAgent.UpdateNoahOrExpEvent(opcode, byNation, byAmount, byDay, byHour, byMinute, sDuration, nCoins);
	}
	break;

	case KING_EVENT_PRIZE:
	{
		uint32 nCoins;
		string strUserID;
		pkt >> nCoins >> strUserID;

		g_DBAgent.InsertPrizeEvent(opcode, byNation, nCoins, strUserID);
	}
	break;
	}
}

/**
* @brief	Handles database requests for King commands.
*
* @param	pUser	The user making the request, if applicable.
* 					nullptr if not.
* @param	pkt  	The packet.
*/
void CKingSystem::HandleDatabaseRequest_Tax(CUser* pUser, Packet& pkt)
{
	if (pUser == nullptr)
		return;

	uint8 opcode = 0, byNation = 0, TerritoryTariff = 0;
	uint16 dummy;
	uint32 TerritoryTax = 0;
	pkt >> opcode;

	switch (opcode)
	{
	case 2: // Collec Kings Fund
		pkt >> TerritoryTax >> byNation;
		break;
	case 4: // Update tax
	{
		pkt >> dummy >> TerritoryTariff >> byNation;
	}
	break;
	}

	g_DBAgent.InsertTaxEvent(opcode, TerritoryTariff, byNation, TerritoryTax);
}

#pragma region CUser::ReqConcurrentProcess(Packet & pkt)
void CUser::ReqConcurrentProcess(Packet& pkt)
{
	uint32 serverNo, count;
	pkt >> serverNo >> count;
	g_DBAgent.UpdateConCurrentUserCount(serverNo, 1, count);
}
#pragma endregion

#pragma region CUser::DailyOpProcess(Packet & pkt)
void CUser::DailyOpProcess(Packet& pkt)
{
	uint8 process = pkt.read<uint8>();
	if (process == 1)
	{
		uint8 type;
		int32 time;
		std::string name;
		pkt >> name >> type >> time;
		g_DBAgent.UpdateUserDailyOp(name, type, time);
	}
	else if (process == 2)
	{
		_USER_DAILY_OP pData;
		pkt >> pData.strUserId;
		pkt >> pData.ChaosMapTime;
		pkt >> pData.UserRankRewardTime;
		pkt >> pData.PersonalRankRewardTime;
		pkt >> pData.KingWingTime;
		pkt >> pData.WarderKillerTime1;
		pkt >> pData.WarderKillerTime2;
		pkt >> pData.KeeperKillerTime;
		pkt >> pData.UserLoyaltyWingRewardTime;
		g_DBAgent.InsertUserDailyOp(&pData);
	}
}
#pragma endregion