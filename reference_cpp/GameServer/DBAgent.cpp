#include "stdafx.h"
#include "../shared/database/OdbcConnection.h"
#include "KnightsManager.h"
#include "DBAgent.h"
#include "../shared/DateTime.h"

CDBAgent g_DBAgent;

using std::string;
using std::unique_ptr;

CDBAgent::CDBAgent() { m_GameDB = new OdbcConnection(); m_AccountDB = new OdbcConnection(); m_LogDB = new OdbcConnection(); }

CDBAgent::~CDBAgent() { delete m_GameDB; delete m_AccountDB; delete m_LogDB; }

bool CDBAgent::Startup(bool bMarsEnabled, tstring& strAccountDSN, tstring& strAccountUID, tstring& strAccountPWD,
	tstring& strGameDSN, tstring& strGameUID, tstring& strGamePWD, tstring& strLogDSN, tstring& strLogUID, tstring& strLogPWD)
{
	if (!Connect(bMarsEnabled, strAccountDSN, strAccountUID, strAccountPWD, strGameDSN, strGameUID, strGamePWD, strLogDSN, strLogUID, strLogPWD))
	{
		printf(_T("ERROR: Failed to connect to the database server.\n"));
		return false;
	}
	return true;
}

bool CDBAgent::Connect(bool bMarsEnabled, tstring& strAccountDSN, tstring& strAccountUID, tstring& strAccountPWD,
	tstring& strGameDSN, tstring& strGameUID, tstring& strGamePWD,
	tstring& strLogDSN, tstring& strLogUID, tstring& strLogPWD)
{
	if (!m_AccountDB->Connect(strAccountDSN, strAccountUID, strAccountPWD, bMarsEnabled)) { ReportSQLError(m_AccountDB->GetError()); return false; }
	if (!m_GameDB->Connect(strGameDSN, strGameUID, strGamePWD, bMarsEnabled)) { ReportSQLError(m_GameDB->GetError()); return false; }
	if (!m_LogDB->Connect(strLogDSN, strLogUID, strLogPWD, bMarsEnabled)) { ReportSQLError(m_LogDB->GetError()); return false; }
	return true;
}

void CDBAgent::ReportSQLError(OdbcError* pError)
{
	if (pError == nullptr)
		return;

	DateTime time;

	// This is *very* temporary.
	string errorMessage = string_format(_T("[ ODBC Error - %d.%d.%d %d:%d:%d ] ] Source: %s Error: %s Description: %s\n"),
		time.GetDay(), time.GetMonth(), time.GetYear(), time.GetHour(), time.GetMinute(), time.GetSecond(),
		pError->Source.c_str(), pError->ExtendedErrorMessage.c_str(), pError->ErrorMessage.c_str());

	Guard lock(m_lock);

	FILE* fp = fopen("./Logs/GameServer.log", "a");
	if (fp != nullptr)
	{
		fwrite(errorMessage.c_str(), errorMessage.length(), 1, fp);
		fclose(fp);
	}

	printf("Database error: %s\n", errorMessage.c_str());
	delete pError;
}

int8 CDBAgent::AccountLogin(string& strAccountID, string& strPasswd)
{
	int8 bRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strPasswd.c_str(), strPasswd.length());

	if (!dbCommand->Execute(_T("{? = CALL GAME_LOGIN_V2(?, ?)}")))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

uint8 CDBAgent::NationSelect(string& strAccountID, uint8 bNation)
{
	uint8 bRet = 0;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL NATION_SELECT(?, %d)}"), bNation)))
		ReportSQLError(m_GameDB->GetError());

	return (bRet > 0 ? bNation : 0);
}

#pragma region CDBAgent::GetAllCharID(string & strAccountID, string & strCharID1, string & strCharID2, string & strCharID3, string & strCharID4)
bool CDBAgent::GetAllCharID(string& strAccountID, string& strCharID1, string& strCharID2, string& strCharID3, string& strCharID4)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	int8 bRet = -2; // generic error

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("{? = CALL GET_ALL_CHARID(?)}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	dbCommand->FetchString(1, strCharID1);
	dbCommand->FetchString(2, strCharID2);
	dbCommand->FetchString(3, strCharID3);
	dbCommand->FetchString(4, strCharID4);

	return true;
}
#pragma endregion

void CDBAgent::LoadCharInfo(string& strCharID, ByteBuffer& result)
{
	uint32 nHair = 0;
	uint16 sClass = 0;
	uint8 bRace = 0, bLevel = 0, bFace = 0, bZone = 0;
	char strItem[INVENTORY_TOTAL * 8];
	char strItemExpiration[INVENTORY_TOTAL * 4];
	ByteBuffer itemData;

	// ensure it's all 0'd out initially.
	memset(strItem, 0x00, sizeof(strItem));
	memset(strItemExpiration, 0x00, sizeof(strItemExpiration));

	if (!strCharID.empty())
	{
		unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
		if (dbCommand.get() == nullptr)
			return;

		dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

		if (!dbCommand->Execute(_T("{CALL LOAD_CHAR_INFO(?)}")))
			ReportSQLError(m_GameDB->GetError());

		if (dbCommand->hasData())
		{
			dbCommand->FetchByte(1, bRace);
			dbCommand->FetchUInt16(2, sClass);
			dbCommand->FetchUInt32(3, nHair);
			dbCommand->FetchByte(4, bLevel);
			dbCommand->FetchByte(5, bFace);
			dbCommand->FetchByte(6, bZone);
			dbCommand->FetchBinary(7, strItem, sizeof(strItem));
			dbCommand->FetchBinary(8, strItemExpiration, sizeof(strItemExpiration));
		}
	}

	itemData.append(strItem, sizeof(strItem));

	result << strCharID << bRace << sClass << bLevel << bFace << nHair << bZone;
	for (int i = 0; i < SLOT_MAX; i++)
	{
		uint32 nItemID;
		uint16 sDurability, sCount;
		itemData >> nItemID >> sDurability >> sCount;
		if (i == HEAD || i == BREAST || i == SHOULDER || i == LEG || i == GLOVE || i == FOOT || i == RIGHTHAND || i == LEFTHAND)
			result << nItemID << sDurability;
	}
}

int8 CDBAgent::CreateNewChar(string& strAccountID, int index, string& strCharID, uint8 bRace, uint16 sClass, uint32 nHair, uint8 bFace, uint8 bStr, uint8 bSta, uint8 bDex, uint8 bInt, uint8 bCha)
{
	int8 bRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL CREATE_NEW_CHAR(?, %d, ?, %d, %d, %d, %d, %d, %d, %d, %d, %d)}"),
		index, bRace, sClass, nHair, bFace, bStr, bSta, bDex, bInt, bCha)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

void CDBAgent::LoadNewCharSet(std::string& strCharID, uint16 sClass)
{
	int8 bRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
#if GAME_SOURCE_VERSION  == 2369
	if (!dbCommand->Execute(string_format(_T("{CALL [LOAD_NEW_CHAR_SET2369](?,%d)}"), sClass)))
#elif GAME_SOURCE_VERSION  == 1098
	if (!dbCommand->Execute(string_format(_T("{CALL [LOAD_NEW_CHAR_SET1098](?,%d)}"), sClass)))
#elif GAME_SOURCE_VERSION  == 1534
	if (!dbCommand->Execute(string_format(_T("{CALL [LOAD_NEW_CHAR_SET1534](?,%d)}"), sClass)))
#endif
		ReportSQLError(m_GameDB->GetError());
}

void CDBAgent::LoadNewCharValue(std::string& strCharID, uint16 sClass)
{
	int8 bRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
#if GAME_SOURCE_VERSION  == 2369
	if (!dbCommand->Execute(string_format(_T("{CALL [LOAD_NEW_CHAR_VALUE2369](?,%d, %d)}"), sClass, g_pMain->m_nServerNo)))
#elif GAME_SOURCE_VERSION  == 1098
	if (!dbCommand->Execute(string_format(_T("{CALL [LOAD_NEW_CHAR_VALUE1098](?,%d, %d)}"), sClass, g_pMain->m_nServerNo)))
#elif GAME_SOURCE_VERSION  == 1534
	if (!dbCommand->Execute(string_format(_T("{CALL [LOAD_NEW_CHAR_VALUE1534](?,%d, %d)}"), sClass, g_pMain->m_nServerNo)))
#endif
		ReportSQLError(m_GameDB->GetError());
}

int8 CDBAgent::ChangeHair(std::string& strAccountID, std::string& strCharID, uint8 bOpcode, uint8 bFace, uint32 nHair)
{
	int8 bRet = 1; // failed
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL CHANGE_HAIR(?, ?, %d, %d, %d)}"),
		bOpcode, bFace, nHair)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

int8 CDBAgent::DeleteChar(string& strAccountID, int index, string& strCharID, string& strSocNo)
{
	int8 bRet = -2; // generic error
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strSocNo.c_str(), strSocNo.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL DELETE_CHAR(?, %d, ?, ?)}"), index)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

#pragma region CDBAgent::VIPStorageSetPassword(std::string &strSealPasswd, std::string & strAccountID, uint8 KeyCancel)
uint8 CDBAgent::VIPStorageSetPassword(std::string& strSealPasswd, std::string& strAccountID, uint8 KeyCancel)
{
	uint8 bRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 3;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strSealPasswd.c_str(), strSealPasswd.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL VIP_STORAGE_PASSWORD(?, ?, %d)}"), KeyCancel)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}
#pragma endregion

#pragma region CDBAgent::VIPStorageCancelPassword(std::string & strAccountID)
uint8 CDBAgent::VIPStorageCancelPassword(std::string& strAccountID, uint8 KeyCancel)
{
	uint8 bRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 2;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL VIP_STORAGE_CANCEL_PASSWORD(?,%d)}"), KeyCancel)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}
#pragma endregion

#pragma region CDBAgent::VIPStorageUseVaultKey(std::string &strAccountID, uint32 iExpirationTime)

uint8 CDBAgent::VIPStorageUseVaultKey(std::string& strAccountID, uint32 iExpirationTime)
{
	uint8 bRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 3;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL VIP_STORAGE_USE_VAULTKEY(?, %d)}"), iExpirationTime)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

#pragma endregion

void CDBAgent::LoadRentalData(string& strAccountID, string& strCharID, UserRentalMap& rentalData)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("{CALL LOAD_RENTAL_DATA(?)}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData())
		return;

	do
	{
		_USER_RENTAL_ITEM* pItem = new _USER_RENTAL_ITEM();
		_RENTAL_ITEM* pRentalItem = nullptr;

		dbCommand->FetchString(1, pItem->strUserID);
		if (STRCASECMP(pItem->strUserID.c_str(), strCharID.c_str()) != 0)
		{
			delete pItem;
			continue;
		}

		dbCommand->FetchByte(2, pItem->byRentalType);
		dbCommand->FetchByte(3, pItem->byRegType);
		dbCommand->FetchUInt32(4, pItem->nRentalIndex);
		dbCommand->FetchUInt32(5, pItem->nItemID);
		dbCommand->FetchUInt16(6, pItem->sDurability);
		dbCommand->FetchUInt64(7, pItem->nSerialNum);
		dbCommand->FetchUInt32(8, pItem->nRentalMoney);
		dbCommand->FetchUInt16(9, pItem->sRentalTime);
		dbCommand->FetchInt16(10, pItem->sMinutesRemaining);
		dbCommand->FetchString(11, pItem->szTimeRental, sizeof(pItem->szTimeRental));

		pRentalItem = g_pMain->m_RentalItemArray.GetData(pItem->nRentalIndex);
		if (pRentalItem == nullptr
			|| rentalData.find(pItem->nSerialNum) != rentalData.end())
			delete pItem;
		else
			rentalData.insert(std::make_pair(pItem->nSerialNum, pItem));

	} while (dbCommand->MoveNext());
}

void CDBAgent::LoadItemSealData(string& strAccountID, string& strCharID, UserItemSealMap& itemSealData)
{
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("SELECT nItemSerial, nItemID, bSealType, prelockstate FROM SEALED_ITEMS WHERE strAccountID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData())
		return;

	do
	{
		int field = 1;
		_USER_SEAL_ITEM* pItem = new _USER_SEAL_ITEM;
		dbCommand->FetchUInt64(field++, pItem->nSerialNum);
		dbCommand->FetchUInt32(field++, pItem->nItemID);
		dbCommand->FetchByte(field++, pItem->bSealType);
		dbCommand->FetchByte(field++, pItem->prelockstate);
		itemSealData.insert(std::make_pair(pItem->nSerialNum, pItem));
	} while (dbCommand->MoveNext());
}

uint8 CDBAgent::CharacterSelectLoginChecking(std::string& AccountID, std::string& CharID)
{
	if (CharID.length() > MAX_ID_SIZE)
		return 0;

	uint8 bRet = 0;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, AccountID.c_str(), AccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, CharID.c_str(), CharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL CHARACTER_LOGIN_CHECKS(?, ?)}"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return 0;
	}

	return bRet;
}

void CDBAgent::InsertTrashItemList(CUser* pUser, uint32 itemid, uint32 itemcount, uint16 duration, uint32 deleteime, uint64 serial, uint8 bflag)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return;
	if (pUser == nullptr) return;
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(string_format(_T("{CALL INSERT_TRASH_ITEMLIST(?,%d,%d,%d," I64FMTD ",%d,%d)}"), itemid, duration, deleteime, serial, bflag, itemcount)))
		ReportSQLError(m_GameDB->GetError());
}

void CDBAgent::DeleteTrashItem(CUser* pUser, uint64 serial)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return;
	if (pUser == nullptr) return;
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(string_format(_T("{CALL DELETE_TRASH_ITEM(?," I64FMTD ")}"), serial)))
		ReportSQLError(m_GameDB->GetError());
}

bool CDBAgent::LoadTrashItemList(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(_T("{CALL LOAD_TRASH_ITEMLIST(?)}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return true;

	do
	{
		uint32 itemid, deletetime, itemcount; uint16 itemduration; uint8 flag; uint64 serialnum;  int field = 1;
		dbCommand->FetchUInt32(field++, itemid);
		dbCommand->FetchUInt32(field++, deletetime);
		dbCommand->FetchUInt16(field++, itemduration);
		dbCommand->FetchUInt32(field++, itemcount);
		dbCommand->FetchByte(field++, flag);
		dbCommand->FetchUInt64(field++, serialnum);

		if (!itemid || (uint32)UNIXTIME >= deletetime || !itemcount) continue;

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(itemid);
		if (pTable.isnull()) continue;

		if ((pTable.m_bKind == 255 && !pTable.m_bCountable) || (!pTable.m_bCountable)) itemcount = 1;

		short RemoveID = pUser->GetRepurchaseFreeID();
		if (RemoveID == -1 || pUser->m_sDeletedItemMap.GetData(RemoveID) != nullptr) continue;

		_DELETED_ITEM* pDelete = new _DELETED_ITEM;
		pDelete->nNum = itemid;
		pDelete->sCount = itemcount;
		pDelete->iDeleteTime = deletetime;
		pDelete->serialnum = serialnum;
		pDelete->bflag = flag;
		pDelete->itemduration = itemduration;
		pDelete->status = false;
		if (!pUser->m_sDeletedItemMap.PutData(RemoveID, pDelete)) delete pDelete;
	} while (dbCommand->MoveNext());
	return true;
}

bool CDBAgent::LoadUserData(string& strAccountID, string& strCharID, CUser* pUser)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (pUser == nullptr || pUser->m_bLogout || !pUser->GetName().empty() || strCharID.length() > MAX_ID_SIZE)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(_T("{CALL LOAD_USER_DATA(?, ?)}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	char strItem[INVENTORY_TOTAL * 8]{ },
		strSerial[INVENTORY_TOTAL * 8]{ },
		strItemExpiration[INVENTORY_TOTAL * 4],
		strDeletedItem[40 * 12]{};

	char strLevel[83]{};

	memset(strItem, 0, sizeof(strItem));
	memset(strSerial, 0, sizeof(strSerial));
	memset(strItemExpiration, 0, sizeof(strItemExpiration));
	memset(strDeletedItem, 0, sizeof(strDeletedItem));
	memset(strLevel, 0, sizeof(strLevel));

	int field = 1;
	dbCommand->FetchByte(field++, pUser->m_bNation);
	dbCommand->FetchByte(field++, pUser->m_bRace);
	dbCommand->FetchUInt16(field++, pUser->m_sClass);
	dbCommand->FetchUInt32(field++, pUser->m_nHair);
	dbCommand->FetchByte(field++, pUser->m_bRank);
	dbCommand->FetchByte(field++, pUser->m_bTitle);
	dbCommand->FetchByte(field++, pUser->m_bLevel);
	dbCommand->FetchByte(field++, pUser->m_bRebirthLevel);
	dbCommand->FetchInt64(field++, pUser->m_iExp);
	dbCommand->FetchUInt32(field++, pUser->m_iLoyalty);
	dbCommand->FetchByte(field++, pUser->m_bFace);
	dbCommand->FetchByte(field++, pUser->m_bCity);
	dbCommand->FetchInt16(field++, pUser->m_bKnights);
	dbCommand->FetchByte(field++, pUser->m_bFame);
	dbCommand->FetchInt16(field++, pUser->m_sHp);
	dbCommand->FetchInt16(field++, pUser->m_sMp);
	dbCommand->FetchInt16(field++, pUser->m_sSp);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_STR]);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_STA]);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_DEX]);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_INT]);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_CHA]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_STR]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_STA]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_DEX]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_INT]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_CHA]);
	dbCommand->FetchByte(field++, pUser->m_bAuthority);
	dbCommand->FetchInt16(field++, pUser->m_sPoints);
	dbCommand->FetchUInt32(field++, pUser->m_iGold);
	dbCommand->FetchByte(field++, pUser->m_bZone);
	dbCommand->FetchInt16(field++, pUser->m_sBind);
	pUser->m_curx = (float)(dbCommand->FetchInt32(field++) / 100.0f);
	pUser->m_curz = (float)(dbCommand->FetchInt32(field++) / 100.0f);
	pUser->m_cury = (float)(dbCommand->FetchInt32(field++) / 100.0f);
	pUser->m_oldx = pUser->m_curx;
	pUser->m_oldy = pUser->m_cury;
	pUser->m_oldz = pUser->m_curz;
	pUser->m_dwTime = dbCommand->FetchUInt32(field++) + 1;
	dbCommand->FetchString(field++, (char*)pUser->m_bstrSkill, sizeof(pUser->m_bstrSkill));
	dbCommand->FetchBinary(field++, strItem, sizeof(strItem));
	dbCommand->FetchBinary(field++, strSerial, sizeof(strSerial));
	dbCommand->FetchBinary(field++, strItemExpiration, sizeof(strItemExpiration));
	dbCommand->FetchBinary(field++, strLevel, sizeof(strLevel));
	dbCommand->FetchUInt32(field++, pUser->m_iMannerPoint);
	dbCommand->FetchUInt32(field++, pUser->m_iLoyaltyMonthly);
	dbCommand->FetchInt32(field++, pUser->m_mutetimestatus);
	dbCommand->FetchInt32(field++, pUser->m_attacktimestatus);
	dbCommand->FetchString(field++, pUser->mytagname);
	dbCommand->FetchUInt32(field++, pUser->m_tagnamergb);
	dbCommand->FetchByte(field++, pUser->ChickenStatus);
	dbCommand->FetchUInt32(field++, pUser->m_flashtime);
	dbCommand->FetchByte(field++, pUser->m_flashcount);
	dbCommand->FetchByte(field++, pUser->m_flashtype);
	dbCommand->FetchUInt32(field++, pUser->m_EventDisandTime);

	pUser->m_strUserID = strCharID;
	pUser->m_lastSaveTime = UNIXTIME;
	pUser->m_lastBonusTime = UNIXTIME;
	pUser->m_TimeOnline = UNIXTIME;

	ByteBuffer ItemBuffer, DeletedItemBuffer, SerialBuffer, ItemTimeBuffer;
	ItemBuffer.append(strItem, sizeof(strItem));
	DeletedItemBuffer.append(strDeletedItem, sizeof(strDeletedItem));
	SerialBuffer.append(strSerial, sizeof(strSerial));
	ItemTimeBuffer.append(strItemExpiration, sizeof(strItemExpiration));

	ByteBuffer strLevelBuffer;
	strLevelBuffer.append(strLevel, sizeof(strLevel));

	memset(pUser->m_sItemArray, 0x00, sizeof(pUser->m_sItemArray));
	memset(pUser->m_sLevelArray, 0x00, sizeof(pUser->m_sLevelArray));

	pUser->m_sDeletedItemMap.DeleteAllData();

	if (pUser->m_sealedItemMap.size())
	{
		foreach(itr, pUser->m_sealedItemMap)
		{
			if (itr->second == nullptr)
				continue;

			delete itr->second;
		}
	}
	pUser->m_sealedItemMap.clear();

	LoadItemSealData(strAccountID, strCharID, pUser->m_sealedItemMap);
	LoadKnightCash(strAccountID, pUser);

	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		uint64 nSerialNum;
		uint32 nItemID;
		int16 sDurability, sCount;
		uint32 nItemTime;

		ItemBuffer >> nItemID >> sDurability >> sCount;
		SerialBuffer >> nSerialNum;			//�TEMLER KARARIRSA AKINA HABER VER BUFLARI BA�TAN YAZICAK 26.10.2020 02.05
		ItemTimeBuffer >> nItemTime;

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || sCount <= 0)
			continue;

		if (!pTable.m_bCountable && sCount > 1)
			sCount = 1;
		else if (sCount > ITEMCOUNT_MAX)
			sCount = ITEMCOUNT_MAX;

		if (nSerialNum == 0)
			nSerialNum = g_pMain->GenerateItemSerial();

		_ITEM_DATA* pItem = pUser->GetItem(i);
		if (pItem == nullptr)
			continue;

		pItem->nNum = nItemID;
		pItem->sDuration = pTable.isAccessory() ? pTable.m_sDuration : sDurability;
		pItem->sCount = sCount;
		pItem->nSerialNum = nSerialNum;
		pItem->nExpirationTime = nItemTime;

		if (i == SHOULDER
			&& (nItemID == ROBIN_LOOT_ITEM || nItemID == 850680000 || nItemID == 510000000 || nItemID == 520000000))
			pUser->m_autoloot = true;

		if (i == CFAIRY && nItemID == ITEM_OREADS)
			pUser->m_fairycheck = true;

		if (i == SHOULDER
			&& pItem->nNum == AUTOMATIC_UPGRADE)
			pUser->m_AutoUpgrade = true;

		if ((pTable.m_bKind == 255 && !pTable.m_bCountable) || (!pTable.m_bCountable)) pItem->sCount = 1;

		UserItemSealMap::iterator sealitr = pUser->m_sealedItemMap.find(nSerialNum);
		if (sealitr != pUser->m_sealedItemMap.end())
		{
			if (sealitr->second->bSealType == 1) pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_SEALED;
			else if (sealitr->second->bSealType == 3) pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_BOUND;
			else if (sealitr->second->bSealType == 4) pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_NOT_BOUND;
			pItem->oFlag = sealitr->second->prelockstate;
		}

		if (pUser->dupplicate_itemcheck(pItem->nSerialNum))
		{
			pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_DUPLICATE;
			auto pTable = g_pMain->GetItemPtr(pItem->nNum);
			if (!pTable.isnull())
			{
				printf("Dupplicate Item Inventory: AccountName:%s - UserName:%s -  [Item Name:%s, ItemNum:%d, SerialNum:%I64d, ItemSlot:%d] \n",
					pUser->GetAccountName().c_str(), pUser->GetName().c_str(), pTable.m_sName.c_str(), pTable.Getnum(), pItem->nSerialNum, i);
			}
		}
	}

	for (int i = 0; i < 83; i++)
	{
		bool sCheck = false;
		strLevelBuffer >> sCheck;
		pUser->m_sLevelArray[i] = sCheck;
	}

	if (pUser->m_bAuthority == (uint8)AuthorityTypes::AUTHORITY_GAME_MASTER || pUser->m_bAuthority == (uint8)AuthorityTypes::AUTHORITY_GM_USER)
	{
		if (!g_DBAgent.LoadGameMasterSetting(strCharID, pUser))
		{
			printf("�zinsiz Gm Giri� Yap�m� ! Gm AccountID: (%s) Character Name: (%s) \n", strAccountID.c_str(), strCharID.c_str());
			return false;
		}
	}

	pUser->m_bCharacterDataLoaded = true;
	return true;
}

bool CDBAgent::LoadWarehouseData(string& strAccountID, CUser* pUser)
{
	char strItem[WAREHOUSE_MAX * 8], strSerial[WAREHOUSE_MAX * 8], strItemExpiration[WAREHOUSE_MAX * 4];

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (pUser == nullptr
		|| pUser->m_bLogout)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("SELECT nMoney, WarehouseData, strSerial, WarehouseDataTime FROM WAREHOUSE WHERE strAccountID = ?")))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return false;

	memset(strItem, 0x00, sizeof(strItem));
	memset(strSerial, 0x00, sizeof(strSerial));
	memset(strItemExpiration, 0x00, sizeof(strItemExpiration));

	dbCommand->FetchUInt32(1, pUser->m_iBank);
	dbCommand->FetchBinary(2, strItem, sizeof(strItem));
	dbCommand->FetchBinary(3, strSerial, sizeof(strSerial));
	dbCommand->FetchBinary(4, strItemExpiration, sizeof(strItemExpiration));

	ByteBuffer ItemBuffer, SerialBuffer, ItemTimeBuffer;
	ItemBuffer.append(strItem, sizeof(strItem));
	SerialBuffer.append(strSerial, sizeof(strSerial));
	ItemTimeBuffer.append(strItemExpiration, sizeof(strItemExpiration));

	memset(pUser->m_sWarehouseArray, 0x00, sizeof(pUser->m_sWarehouseArray));

	for (int i = 0; i < WAREHOUSE_MAX; i++)
	{
		uint64 nSerialNum;
		uint32 nItemID, nItemTime;
		int16 sDurability, sCount;

		ItemBuffer >> nItemID >> sDurability >> sCount;
		SerialBuffer >> nSerialNum;
		ItemTimeBuffer >> nItemTime;

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || sCount <= 0)
			continue;

		if (!pTable.m_bCountable && sCount > 1)
			sCount = 1;
		else if (sCount > ITEMCOUNT_MAX)
			sCount = ITEMCOUNT_MAX;

		pUser->m_sWarehouseArray[i].nNum = nItemID;
		pUser->m_sWarehouseArray[i].sDuration = sDurability;
		pUser->m_sWarehouseArray[i].sCount = sCount;
		pUser->m_sWarehouseArray[i].nSerialNum = nSerialNum;
		pUser->m_sWarehouseArray[i].nExpirationTime = nItemTime;

		if ((pTable.m_bKind == 255 && !pTable.m_bCountable) || (!pTable.m_bCountable))
			pUser->m_sWarehouseArray[i].sCount = 1;

		UserItemSealMap::iterator sealitr = pUser->m_sealedItemMap.find(nSerialNum);
		if (sealitr != pUser->m_sealedItemMap.end())
		{
			if (sealitr->second->bSealType == 1) pUser->m_sWarehouseArray[i].bFlag = (uint8)ItemFlag::ITEM_FLAG_SEALED;
			else if (sealitr->second->bSealType == 3) pUser->m_sWarehouseArray[i].bFlag = (uint8)ItemFlag::ITEM_FLAG_BOUND;
			else if (sealitr->second->bSealType == 4) pUser->m_sWarehouseArray[i].bFlag = (uint8)ItemFlag::ITEM_FLAG_NOT_BOUND;
			pUser->m_sWarehouseArray[i].oFlag = sealitr->second->prelockstate;
		}

		if (pUser->dupplicate_itemcheck(pUser->m_sWarehouseArray[i].nSerialNum))
		{
			pUser->m_sWarehouseArray[i].bFlag = (uint8)ItemFlag::ITEM_FLAG_DUPLICATE;
			auto pTable = g_pMain->GetItemPtr(pUser->m_sWarehouseArray[i].nNum);
			if (!pTable.isnull())
			{
				printf("Dupplicate Item Warehouse: AccountName:%s - UserName:%s -  [Item Name:%s, ItemNum:%d, SerialNum:%I64d, ItemSlot:%d] \n",
					pUser->GetAccountName().c_str(), pUser->GetName().c_str(), pTable.m_sName.c_str(), pTable.Getnum(), pUser->m_sWarehouseArray[i].nSerialNum, i);
			}
		}
	}

	return true;
}

#pragma region CDBAgent::UpdateClanWarehouseData(string strAccountID, CUser *puser)
bool CDBAgent::UpdateClanWarehouseData(std::string strClanID, CUser* pUser)
{
	if (pUser == nullptr || !pUser->isInClan())
		return false;

	CKnights* pKnights = g_pMain->GetClanPtr(pUser->GetClanID());
	if (strClanID.length() == 0 || !pKnights)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	// This *should* be padded like the database field is (unnecessarily), but I want to see how MSSQL responds.
	ByteBuffer itemBuffer, serialBuffer;
	for (int i = 0; i < WAREHOUSE_MAX; i++)
	{
		_ITEM_DATA* pItem = &pKnights->m_sClanWarehouseArray[i];  //pKnights
		if (pItem == nullptr) continue;
		itemBuffer << pItem->nNum << pItem->sDuration << pItem->sCount/* << pItem->nExpirationTime*/; // item remaining infos
		serialBuffer << pItem->nSerialNum;
	}

	//***sorunsuz94*** 27.03.2020 clana para koyma fix 0 lan yer  pUser->m_ClaniBank eklendi
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)itemBuffer.contents(), itemBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)serialBuffer.contents(), serialBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strClanID.c_str(), strClanID.length());
	if (!dbCommand->Execute(string_format(_T("UPDATE KNIGHTS SET Gold=%d, dwTime=%d, WarehouseData=?, strSerial=? WHERE IDName=?"),
		pKnights->m_nMoney, pUser->m_dwTime)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}
	return true;
}
#pragma endregion

bool CDBAgent::LoadVipWarehouseData(string& strAccountID, CUser* pUser)
{
	char strItem[VIPWAREHOUSE_MAX * 8],
		strItemExpiration[VIPWAREHOUSE_MAX * 4],
		strSerial[VIPWAREHOUSE_MAX * 8];

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (pUser == nullptr
		|| pUser->m_bLogout)
		return false;

	pUser->m_bVIPStoragePassword = "";
	pUser->m_bVIPStorageVaultExpiration = 0;
	pUser->m_bWIPStorePassowrdRequest = 0;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("SELECT strPassword,strExpirationDate,WipPasswordRequest, VIPWarehouseData, strSerial, VIPWarehouseDataTime FROM VIP_WAREHOUSE WHERE strAccountID = ?")))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return false;

	memset(strItem, 0x00, sizeof(strItem));
	memset(strSerial, 0x00, sizeof(strSerial));
	memset(strItemExpiration, 0x00, sizeof(strItemExpiration));

	dbCommand->FetchString(1, pUser->m_bVIPStoragePassword);
	dbCommand->FetchUInt32(2, pUser->m_bVIPStorageVaultExpiration);
	dbCommand->FetchByte(3, pUser->m_bWIPStorePassowrdRequest);
	dbCommand->FetchBinary(4, strItem, sizeof(strItem));
	dbCommand->FetchBinary(5, strSerial, sizeof(strSerial));
	dbCommand->FetchBinary(6, strItemExpiration, sizeof(strItemExpiration));

	ByteBuffer vItemBuffer, vSerialBuffer, vItemExBuffer;
	vItemBuffer.append(strItem, sizeof(strItem));
	vSerialBuffer.append(strSerial, sizeof(strSerial));
	vItemExBuffer.append(strItemExpiration, sizeof(strItemExpiration));

	memset(pUser->m_sVIPWarehouseArray, 0x00, sizeof(pUser->m_sVIPWarehouseArray));

	for (int i = 0; i < VIPWAREHOUSE_MAX; i++)
	{
		uint64 nSerialNum;
		uint32 nItemID, nItemEx;
		int16 sDurability, sCount;

		vItemBuffer >> nItemID >> sDurability >> sCount;
		vSerialBuffer >> nSerialNum;
		vItemExBuffer >> nItemEx;

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || sCount <= 0)
			continue;

		if (!pTable.m_bCountable && sCount > 1)
			sCount = 1;
		else if (sCount > ITEMCOUNT_MAX)
			sCount = ITEMCOUNT_MAX;

		pUser->m_sVIPWarehouseArray[i].nNum = nItemID;
		pUser->m_sVIPWarehouseArray[i].sDuration = sDurability;
		pUser->m_sVIPWarehouseArray[i].sCount = sCount;
		pUser->m_sVIPWarehouseArray[i].nSerialNum = nSerialNum;
		pUser->m_sVIPWarehouseArray[i].nExpirationTime = nItemEx;

		if ((pTable.m_bKind == 255 && !pTable.m_bCountable) || (!pTable.m_bCountable))
			pUser->m_sVIPWarehouseArray[i].sCount = 1;

		UserItemSealMap::iterator sealitr = pUser->m_sealedItemMap.find(nSerialNum);
		if (sealitr != pUser->m_sealedItemMap.end())
		{
			if (sealitr->second->bSealType == 1) pUser->m_sVIPWarehouseArray[i].bFlag = (uint8)ItemFlag::ITEM_FLAG_SEALED;
			else if (sealitr->second->bSealType == 3) pUser->m_sVIPWarehouseArray[i].bFlag = (uint8)ItemFlag::ITEM_FLAG_BOUND;
			else if (sealitr->second->bSealType == 4) pUser->m_sVIPWarehouseArray[i].bFlag = (uint8)ItemFlag::ITEM_FLAG_NOT_BOUND;
			pUser->m_sVIPWarehouseArray[i].oFlag = sealitr->second->prelockstate;
		}

		if (pUser->dupplicate_itemcheck(pUser->m_sVIPWarehouseArray[i].nSerialNum))
		{
			pUser->m_sVIPWarehouseArray[i].bFlag = (uint8)ItemFlag::ITEM_FLAG_DUPLICATE;
			auto pTable = g_pMain->GetItemPtr(pUser->m_sVIPWarehouseArray[i].nNum);
			if (!pTable.isnull())
			{
				printf("Dupplicate Item WipWarehouse: AccountName:%s - UserName:%s -  [Item Name:%s, ItemNum:%d, SerialNum:%I64d, ItemSlot:%d] \n",
					pUser->GetAccountName().c_str(), pUser->GetName().c_str(), pTable.m_sName.c_str(), pTable.Getnum(), pUser->m_sVIPWarehouseArray[i].nSerialNum, i);
			}
		}
	}
	return true;
}

bool CDBAgent::LoadPremiumServiceUser(string& strAccountID, CUser* pUser)
{
	if (pUser == nullptr) return false;

	std::unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("{CALL LOAD_PREMIUM_SERVICE_USER(?)}")))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	char strPremium[30];
	memset(strPremium, 0, sizeof(strPremium));
	uint8 bPremiumCount = 0;

	dbCommand->FetchByte(1, pUser->m_bPremiumInUse);
	dbCommand->FetchByte(2, bPremiumCount);
	dbCommand->FetchBinary(3, strPremium, sizeof(strPremium));

	for (int i = 0, index = 0; i < bPremiumCount; i++, index += 5)
	{
		uint8	bPremiumType = *(uint8*)(strPremium + index);
		uint32	iPremiumTime = *(uint32*)(strPremium + index + 1);
		if (iPremiumTime < (uint32)UNIXTIME) continue;

		_PREMIUM_DATA* pPremium = new _PREMIUM_DATA;
		pPremium->bPremiumType = bPremiumType;
		pPremium->iPremiumTime = iPremiumTime;
		if (!pUser->m_PremiumMap.PutData(bPremiumType, pPremium))
			delete pPremium;
	}

	if (pUser->m_PremiumMap.GetData(pUser->m_bPremiumInUse) == nullptr)
	{
		pUser->m_bPremiumInUse = NO_PREMIUM;
		foreach_stlmap(itr, pUser->m_PremiumMap)
		{
			_PREMIUM_DATA* uPrem = itr->second;
			if (uPrem == nullptr || uPrem->iPremiumTime == 0) continue;
			pUser->m_bPremiumInUse = uPrem->bPremiumType;
			break;
		}
	}

	// this is hardcoded because we don't really care about the other mode
	if (pUser->m_bPremiumInUse != NO_PREMIUM)
		pUser->m_bAccountStatus = 1; // normal premium with expiry time
	else
		pUser->m_bAccountStatus = 0;

	return true;
}

#pragma region CDBAgent::LoadSavedMagic(CUser *pUser)

bool CDBAgent::LoadSavedMagic(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());
	if (!dbCommand->Execute(_T("SELECT "
		"nSkill1, nDuring1, nSkill2, nDuring2, nSkill3, nDuring3, nSkill4, nDuring4, nSkill5, nDuring5, "
		"nSkill6, nDuring6, nSkill7, nDuring7, nSkill8, nDuring8, nSkill9, nDuring9, nSkill10, nDuring10 "
		"FROM USER_SAVED_MAGIC WHERE strCharID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}


	if (!dbCommand->hasData())
		return false;

	Guard lock(pUser->m_savedMagicLock);
	pUser->m_savedMagicMap.clear();
	for (int i = 1; i <= 20; i += 2)
	{
		uint32 nSkillID = 0, nExpiry = 0;
		dbCommand->FetchUInt32(i, nSkillID);
		dbCommand->FetchUInt32(i + 1, nExpiry);
		if (nSkillID > 0 && nExpiry > 5 && nExpiry < 28800)
			pUser->m_savedMagicMap[nSkillID] = (static_cast<unsigned long long>(nExpiry) * 1000 + UNIXTIME2);
	}

	return true;
}
#pragma endregion

#pragma region CDBAgent::UpdateSavedMagic(CUser *pUser)
bool CDBAgent::UpdateSavedMagic(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	pUser->m_savedMagicLock.lock();
	uint32 nSkillID[10] = { 0 }, tExpiryTime[10] = { 0 }, i = 0;
	foreach(itr, pUser->m_savedMagicMap)
	{
		if (!itr->second) continue;
		int32 sctime = 0;
		if (itr->first > 0 && (itr->second - UNIXTIME2 > 1000)) sctime = (uint32)((itr->second - UNIXTIME2) / 1000);
		if (itr->first > 0 && sctime > 0) { nSkillID[i] = itr->first; tExpiryTime[i] = sctime; }
		if (++i >= 10) break;
	}
	pUser->m_savedMagicLock.unlock();

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());
	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_SAVED_MAGIC(?, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d)}"),
		nSkillID[0], tExpiryTime[0], nSkillID[1], tExpiryTime[1], nSkillID[2], tExpiryTime[2], nSkillID[3], tExpiryTime[3], nSkillID[4], tExpiryTime[4],
		nSkillID[5], tExpiryTime[5], nSkillID[6], tExpiryTime[6], nSkillID[7], tExpiryTime[7], nSkillID[8], tExpiryTime[8], nSkillID[9], tExpiryTime[9])))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}
#pragma endregion

bool CDBAgent::SetLogInInfo(string& strAccountID, string& strCharID, string& strServerIP, short sServerNo, string& strClientIP, uint8 bInit)
{
	uint8 result = 0;
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &result);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strServerIP.c_str(), strServerIP.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strClientIP.c_str(), strClientIP.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL SET_LOGIN_INFO(?, ?, %d, ?, ?, %d)}"), sServerNo, bInit)))
		ReportSQLError(m_AccountDB->GetError());

	return (bool)(result == 0 ? false : true);
}

#pragma region CDBAgent::RemoveCurrentUser(string & strAccountID)

bool CDBAgent::RemoveCurrentUser(string& strAccountID)
{
	if (strAccountID.empty())
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("DELETE FROM CURRENTUSER WHERE strAccountID = ?")))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	return true;
}
#pragma endregion
#pragma region CDBAgent::InsertCurrentUser(string & strAccountID)
void CDBAgent::InsertCheatLog(CUser* pUser)
{

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetRemoteIP().c_str(), pUser->GetRemoteIP().length());
	if (!dbCommand->Execute(_T("insert into KO_LOG.dbo.LOGS_CHEAT (strAccountID,strCharID,strClientIP,strCheatLog) values (?,?,?,'Cheat Decated BullCode !')")))
	{
		ReportSQLError(m_AccountDB->GetError());
		return;
	}

	return;
}
#pragma endregion
#pragma region CDBAgent::InsertCurrentUser(string & strAccountID)
bool CDBAgent::InsertCurrentUser(string& strAccountID, string& strCharID)
{
	if (strAccountID.empty() || strCharID.empty())
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(_T("INSERT INTO CURRENTUSER VALUES (?,?,1,'5.9.173.118','127.0.0.2')")))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	return true;
}
#pragma endregion

void CDBAgent::DestroyPusSessions(CUser* pUser)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr || pUser == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strAccountID.c_str(), pUser->m_strAccountID.length());

	// TODO: Add an arg for the free slot count so we only need to pull/delete what we can hold.
	if (!dbCommand->Execute(string_format(_T("DELETE FROM PUS_SESSION WHERE [strAccountID] = ?"))))
	{
		ReportSQLError(m_AccountDB->GetError());
		return;
	}

	return;
}

bool CDBAgent::CreatePusSession(uint16 sFreeCount, CUser* pUser)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr || pUser == nullptr)
		return false;

	std::string strClass = "~Unknown~";

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strAccountID.c_str(), pUser->m_strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strClass.c_str(), strClass.length());


	// TODO: Add an arg for the free slot count so we only need to pull/delete what we can hold.
	if (!dbCommand->Execute(string_format(_T("INSERT INTO PUS_SESSION([strAccountID],[strUserID],[strClass],[strLevel],[strParam],[strData],[FreeSlotCount],[SessionKey],[CreatedTime]) VALUES (?, ?, ?, %d, %d, '', GetDate())"), pUser->GetLevel(), sFreeCount)))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	return true;
}



bool CDBAgent::LoadWebItemMall(std::vector<_ITEM_DATA>& itemList, CUser* pUser)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());

	// TODO: Add an arg for the free slot count so we only need to pull/delete what we can hold.
	if (!dbCommand->Execute(_T("{CALL LOAD_WEB_ITEMMALL(?)}")))
		ReportSQLError(m_AccountDB->GetError());

	if (dbCommand->hasData())
	{
		do
		{
			_ITEM_DATA item;
			dbCommand->FetchUInt32(2, item.nNum); // 1 is the account name, which we don't need to use unless we're logging	
			dbCommand->FetchUInt16(3, item.sCount);
			dbCommand->FetchUInt32(5, item.nExpirationTime);
			itemList.push_back(item);
		} while (dbCommand->MoveNext());
	}

	return true;
}

#pragma region CDBAgent::LoadSkillShortcut(Packet & result, CUser *pUser)

bool CDBAgent::LoadSkillShortcut(Packet& result, CUser* pUser)
{
	if (pUser == nullptr) return false;

	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;


	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(_T("SELECT nCount, strSkillData FROM USERDATA_SKILLSHORTCUT WHERE strCharID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}
	if (!dbCommand->hasData()) return false;

	int field = 1;
	dbCommand->FetchUInt16(field++, pUser->m_strSkillCount);
	dbCommand->FetchBinary(field++, (char*)pUser->m_strSkillData, sizeof(pUser->m_strSkillData));

	result << pUser->m_strSkillCount;
	for (uint32 i = 0; i < pUser->m_strSkillCount; i++) result << *(uint32*)(pUser->m_strSkillData + (i * sizeof(uint32)));
	return true;
}
#pragma endregion

#pragma region CDBAgent::SaveSkillShortcut(CUser *pUser)
void CDBAgent::SaveSkillShortcut(CUser* pUser)
{
	if (pUser == nullptr) return;

	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)pUser->m_strSkillData, sizeof(pUser->m_strSkillData), SQL_BINARY);
	if (!dbCommand->Execute(string_format(_T("{CALL SKILLSHORTCUT_SAVE(?,%d,?)}"), pUser->m_strSkillCount)))
		ReportSQLError(m_GameDB->GetError());
}
#pragma endregion

bool CDBAgent::UpdateClanPremiumServiceUser(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	CKnights* pKnights = g_pMain->GetClanPtr(pUser->GetClanID());
	if (pKnights == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (!dbCommand->Execute(string_format(_T("{CALL SAVE_PREMIUM_SERVICE_CLAN(%d, %d, %d)}"),
		pKnights->GetID(), pKnights->sPremiumInUse, pKnights->sPremiumTime)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}

bool CDBAgent::UpdatePremiumServiceUser(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	int8 bRet = -2; // generic error

	char strPremium[30];
	int index = 0;
	memset(strPremium, 0, sizeof(strPremium));
	int counter = 0;

	uint32 time = 0;
	foreach_stlmap(itr, pUser->m_PremiumMap)
	{
		if (itr->second == nullptr
			|| itr->second->iPremiumTime < UNIXTIME)
			continue;

		if (time < itr->second->iPremiumTime)
			time = itr->second->iPremiumTime;

		*(uint8*)(strPremium + index) = itr->first;
		*(uint32*)(strPremium + 1 + index) = itr->second->iPremiumTime;
		index += 5;
		counter++;
	}

	if (time > uint32(UNIXTIME))
		time = uint32(time - UNIXTIME); // total premium seconds left

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)strPremium, sizeof(strPremium), SQL_BINARY);

	if (!dbCommand->Execute(string_format(_T("{? = CALL SAVE_PREMIUM_SERVICE_USER_V2(?, ?, %d, %d, %d)}"),
		counter, pUser->m_bPremiumInUse, time)))
		ReportSQLError(m_AccountDB->GetError());

	return true;
}

uint8 CDBAgent::SealItem(string strSealPasswd, uint64 nItemSerial, uint32 nItemID, uint8 bSealType, CUser* pUser)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 3;

	uint8 bRet = 1;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strSealPasswd.c_str(), strSealPasswd.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL USER_ITEM_SEAL(?, ?, ?, " I64FMTD ", %d, %d)}"), nItemSerial, nItemID, bSealType)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

void CDBAgent::RequestFriendList(std::vector<string>& friendList, CUser* pUser)
{
	if (pUser == nullptr)
		return;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(_T("SELECT * FROM FRIEND_LIST WHERE strUserID = ?")))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return;

	string strCharID;
	for (int i = 2; i <= 25; i++)
	{
		if (dbCommand->FetchString(i, strCharID)
			&& strCharID.length())
			friendList.push_back(strCharID);
	}
}

FriendAddResult CDBAgent::AddFriend(short sid, short tid)
{
	CUser* pSrcUser = g_pMain->GetUserPtr(sid), * pTargetUser = g_pMain->GetUserPtr(tid);
	if (pSrcUser == nullptr)
		return FriendAddResult::FRIEND_ADD_ERROR;

	if (pTargetUser == nullptr)
	{
		CBot* pTargetUser = g_pMain->GetBotPtr(tid);

		if (pTargetUser == nullptr)
			return FriendAddResult::FRIEND_ADD_ERROR;

		unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
		if (dbCommand.get() == nullptr)
			return FriendAddResult::FRIEND_ADD_ERROR;

		int16 sRet = (int16)FriendAddResult::FRIEND_ADD_ERROR;

		dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
		dbCommand->AddParameter(SQL_PARAM_INPUT, pSrcUser->GetName().c_str(), pSrcUser->GetName().length());
		dbCommand->AddParameter(SQL_PARAM_INPUT, pTargetUser->GetName().c_str(), pTargetUser->GetName().length());

		if (!dbCommand->Execute(_T("{? = CALL INSERT_FRIEND_LIST(?, ?)}")))
			ReportSQLError(m_GameDB->GetError());

		return (FriendAddResult)sRet;
	}

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return FriendAddResult::FRIEND_ADD_ERROR;

	int16 sRet = (int16)FriendAddResult::FRIEND_ADD_ERROR;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pSrcUser->GetName().c_str(), pSrcUser->GetName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pTargetUser->GetName().c_str(), pTargetUser->GetName().length());

	if (!dbCommand->Execute(_T("{? = CALL INSERT_FRIEND_LIST(?, ?)}")))
		ReportSQLError(m_GameDB->GetError());

	return (FriendAddResult)sRet;
}

FriendRemoveResult CDBAgent::RemoveFriend(string& strCharID, CUser* pUser)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return FriendRemoveResult::FRIEND_REMOVE_ERROR;

	int16 sRet = (int16)FriendRemoveResult::FRIEND_REMOVE_ERROR;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(_T("{? = CALL DELETE_FRIEND_LIST(?, ?)}")))
		ReportSQLError(m_GameDB->GetError());

	return (FriendRemoveResult)sRet;
}

bool CDBAgent::UpdateUser(string& strCharID, UserUpdateType type, CUser* pUser)
{
	if (pUser == nullptr || strCharID != pUser->GetName())
		return false;

	if (pUser->pCindWar.isEventUser()/* && g_pMain->isCindirellaZone(pUser->GetZoneID())*/)
		return true;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (type == UserUpdateType::UPDATE_PACKET_SAVE)
		pUser->m_dwTime++;
	else if (type == UserUpdateType::UPDATE_LOGOUT
		|| type == UserUpdateType::UPDATE_ALL_SAVE)
		pUser->m_dwTime = 0;

	ByteBuffer ItemBuffer, SerialBuffer, ItemTimeBuffer;
	ByteBuffer strLevelBuffer;
	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = pUser->GetItem(i);
		if (!pItem)
			continue;

		ItemBuffer << pItem->nNum << pItem->sDuration << pItem->sCount;
		SerialBuffer << pItem->nSerialNum;
		ItemTimeBuffer << pItem->nExpirationTime;
	}

	for (int i = 0; i < 83; i++)
		strLevelBuffer << pUser->m_sLevelArray[i];

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)pUser->m_bstrSkill, sizeof(pUser->m_bstrSkill));
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)ItemBuffer.contents(), ItemBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)SerialBuffer.contents(), SerialBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)ItemTimeBuffer.contents(), ItemTimeBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)strLevelBuffer.contents(), strLevelBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->mytagname.c_str(), pUser->mytagname.length());

	if (type == UserUpdateType::UPDATE_LOGOUT) 
	{
		if (pUser->isInMoradon())
			pUser->m_bZone = ZONE_MORADON;
		else if (pUser->isInLufersonCastle())
			pUser->m_bZone = ZONE_KARUS;
		else if (pUser->isInElmoradCastle())
			pUser->m_bZone = ZONE_ELMORAD;

		if (pUser->isInKarusEslant())
			pUser->m_bZone = ZONE_KARUS_ESLANT;
		else if (pUser->isInElmoradEslant())
			pUser->m_bZone = ZONE_ELMORAD_ESLANT;
	}

	uint32 nDonatedNP = 0;
	CKnights* pKnights = g_pMain->GetClanPtr(pUser->GetClanID());
	if (pKnights != nullptr)
	{
		std::string userid = pUser->GetName();
		STRTOUPPER(userid);
		_KNIGHTS_USER* pKnightUser = pKnights->m_arKnightsUser.GetData(userid);
		if (pKnightUser != nullptr)
		{
			pKnightUser->bFame = pUser->GetFame();
			pKnightUser->bLevel = pUser->GetLevel();
			pKnightUser->sClass = pUser->GetClass();
			pKnightUser->nLastLogin = int32(UNIXTIME);
			pKnightUser->LoyaltyMonthly = pUser->m_iLoyaltyMonthly;
			nDonatedNP = pKnightUser->nDonatedNP;
		}
	}

	if (pUser->isKing())
	{
		CKingSystem* pKingSystem = g_pMain->m_KingSystemArray.GetData(pUser->GetNation());
		if (pKingSystem != nullptr)
			pKingSystem->m_sKingClanID = pUser->GetClanID();
	}

	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_USER_DATA("
		"?, "								// strCharID 
		"%d, %d, %d, %d, %d, "				// nation, race, class, hair, rank
		"%d, %d, %d, " I64FMTD ", %d, %d, "	// title, level, levelreb, exp, loyalty, face
		"%d, %d, %d, "						// city, knights, fame
		"%d, %d, %d, "						// hp, mp, sp
		"%d, %d, %d, %d, %d, "				// str, sta, dex, int, cha
		"%d, %d, %d, %d, %d, "				// rebstr, rebsta, rebdex, rebint, rebcha
		"%d, %d, %d, %d, %d, "				// authority, free points, gold, zone, bind
		"%d, %d, %d, %d, "					// x, z, y, dwTime
		"?, ?, ?, ?, ?, "					// strSkill, strItem, strSerial, strItemExpiration, strLevelCheck
		"%d, %d, "							// manner points, monthly NP
		"%d, %d, %d, %d, ?, %d, %d, "		// LastLogin, nDonatedNP,m_mutetimestatus,m_attacktimestatus,mytagname,chickenstatus,m_tagnamergb
		"%d, %d, %d,%d )}"),						// flashtime, flashcount,flashtype,m_EventDisandTime
		pUser->m_bNation, pUser->m_bRace, pUser->m_sClass, pUser->m_nHair, pUser->m_bRank,
		pUser->m_bTitle, pUser->m_bLevel, pUser->m_bRebirthLevel, pUser->m_iExp /* temp hack, database needs to support it */, pUser->m_iLoyalty, pUser->m_bFace,
		pUser->m_bCity, pUser->m_bKnights, pUser->m_bFame,
		pUser->m_sHp, pUser->m_sMp, pUser->m_sSp,
		pUser->m_bStats[(uint8)StatType::STAT_STR], pUser->m_bStats[(uint8)StatType::STAT_STA], pUser->m_bStats[(uint8)StatType::STAT_DEX], pUser->m_bStats[(uint8)StatType::STAT_INT], pUser->m_bStats[(uint8)StatType::STAT_CHA],
		pUser->m_bRebStats[(uint8)StatType::STAT_STR], pUser->m_bRebStats[(uint8)StatType::STAT_STA], pUser->m_bRebStats[(uint8)StatType::STAT_DEX], pUser->m_bRebStats[(uint8)StatType::STAT_INT], pUser->m_bRebStats[(uint8)StatType::STAT_CHA],
		pUser->m_bAuthority, pUser->m_sPoints, pUser->m_iGold, pUser->m_bZone, pUser->m_sBind,
		(int)(pUser->m_curx * 100), (int)(pUser->m_curz * 100), (int)(pUser->m_cury * 100), pUser->m_dwTime,
		pUser->m_iMannerPoint, pUser->m_iLoyaltyMonthly,
		int32(UNIXTIME), nDonatedNP, pUser->m_mutetimestatus, pUser->m_attacktimestatus, pUser->ChickenStatus, pUser->m_tagnamergb,
		pUser->m_flashtime, pUser->m_flashcount, pUser->m_flashtype, pUser->m_EventDisandTime)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	pUser->m_lastSaveTime = UNIXTIME;
	pUser->m_lastBonusTime = UNIXTIME;
	return true;
}

bool CDBAgent::UpdateWarehouseData(string& strAccountID, UserUpdateType type, CUser* pUser)
{
	if (strAccountID.length() == 0)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (type == UserUpdateType::UPDATE_LOGOUT
		|| type == UserUpdateType::UPDATE_ALL_SAVE)
		pUser->m_dwTime = 0;

	// This *should* be padded like the database field is (unnecessarily), but I want to see how MSSQL responds.
	ByteBuffer ItemBuffer, SerialBuffer, ItemTimeBuffer;
	for (int i = 0; i < WAREHOUSE_MAX; i++)
	{
		_ITEM_DATA* pItem = &pUser->m_sWarehouseArray[i];
		ItemBuffer << pItem->nNum << pItem->sDuration << pItem->sCount;
		SerialBuffer << pItem->nSerialNum;
		ItemTimeBuffer << pItem->nExpirationTime;
	}

	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)ItemBuffer.contents(), ItemBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)SerialBuffer.contents(), SerialBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)ItemTimeBuffer.contents(), ItemTimeBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());

	if (!dbCommand->Execute(string_format(_T("UPDATE WAREHOUSE SET nMoney=%d, dwTime=%d, WarehouseData=?, strSerial=?, WarehouseDataTime=? WHERE strAccountID=?"),
		pUser->m_iBank, pUser->m_dwTime)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}

bool CDBAgent::UpdateVipWarehouseData(string& strAccountID, UserUpdateType type, CUser* pUser)
{
	if (strAccountID.length() == 0)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	uint64 wait = 0;
	while (dbCommand->isOpen())
	{
		if (!dbCommand->isOpen())
			break;

		wait++;
	}

	if (type == UserUpdateType::UPDATE_LOGOUT
		|| type == UserUpdateType::UPDATE_ALL_SAVE)
		pUser->m_dwTime = 0;

	// This *should* be padded like the database field is (unnecessarily), but I want to see how MSSQL responds.
	ByteBuffer vItemBuffer, vSerialBuffer, vItemExBuffer;

	for (int i = 0; i < VIPWAREHOUSE_MAX; i++)
	{
		_ITEM_DATA* pItem = &pUser->m_sVIPWarehouseArray[i];
		vItemBuffer << pItem->nNum << pItem->sDuration << pItem->sCount;
		vSerialBuffer << pItem->nSerialNum;
		vItemExBuffer << pItem->nExpirationTime;
	}

	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)vItemBuffer.contents(), vItemBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)vSerialBuffer.contents(), vSerialBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)vItemExBuffer.contents(), vItemExBuffer.size(), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());

	if (!dbCommand->Execute(string_format(_T("UPDATE VIP_WAREHOUSE SET VIPWarehouseData=?, strSerial=?, VIPWarehouseDataTime=?  WHERE strAccountID=?"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}

#pragma region CDBAgent::UpdateUserAuthority(std::string GameMaster, string strUserID, uint32 iPeriod, uint8 banType, string reason, CUser *pUser/* = nullptr*/)
void CDBAgent::UpdateUserAuthority(std::string GameMaster, string strUserID, uint32 iPeriod, uint8 banType, string reason, CUser* pUser/* = nullptr*/)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return;

	uint32 day_period = iPeriod;
	if (iPeriod) iPeriod = (uint32)UNIXTIME + (60 * 60 * 24 * iPeriod);

	int sRet = -1; if (reason.empty())reason = "-";
	std::string strRealUserID = "", strAccountID = "";
	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, GameMaster.c_str(), GameMaster.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, reason.c_str(), reason.length());
	if (!dbCommand->Execute(string_format(_T("{CALL AUTHORITY_CHANGE(?, ?, ?, %d, %d)}"), iPeriod, banType)))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData()) { return; }

	int field = 1;
	dbCommand->FetchInt32(field++, sRet);
	dbCommand->FetchString(field++, strRealUserID);
	dbCommand->FetchString(field++, strAccountID);

	if (sRet == 0)
	{
		std::string sNoticeMessage = "";

		CUser* pUserAccount = g_pMain->GetUserPtr(strAccountID, NameType::TYPE_ACCOUNT);
		CUser* pUserCharacter = g_pMain->GetUserPtr(strRealUserID, NameType::TYPE_CHARACTER);

		switch ((BanTypes)banType)
		{
		case BanTypes::BANNED:
			if (pUserAccount != nullptr)
				pUserAccount->Disconnect();

			sNoticeMessage = string_format("%s is currently blocked for using illegal software.", strRealUserID.c_str());
			break;
		case BanTypes::MUTE:
			if (pUserCharacter != nullptr)
			{
				if (day_period)
					pUserCharacter->m_mutetimestatus = iPeriod;
				else
					pUserCharacter->m_mutetimestatus = -1;

				sNoticeMessage = string_format("%s has been muted for %s days.", strRealUserID.c_str(), (day_period > 0 ? std::to_string(day_period).c_str() : "forever"));
			}
			break;
		case BanTypes::UNMUTE:
			if (pUserCharacter != nullptr)
				pUserCharacter->m_mutetimestatus = 0;
			break;
		case BanTypes::DIS_ATTACK:
			if (pUserCharacter != nullptr)
			{
				if (iPeriod)
					pUserCharacter->m_attacktimestatus = iPeriod;
				else
					pUserCharacter->m_attacktimestatus = -1;
			}
			break;
		case BanTypes::ALLOW_ATTACK:
			if (pUserCharacter != nullptr)
				pUserCharacter->m_attacktimestatus = 0;
			break;
		}
		if (!sNoticeMessage.empty()) g_pMain->SendNotice(sNoticeMessage.c_str(), (uint8)Nation::ALL);
		return;
	}

	if (sRet == 1)
	{
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "Invalid player name!");
		else printf("Invalid player name! \n");
		return;
	}

	if (sRet == 2)
	{
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "You cannot change a GM's authority!");
		else printf("You cannot change a GM's authority! \n");
		return;
	}

	if (sRet == 3)
	{
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "No operation was made since user already is of that authority!");
		else printf("No operation was made since user already is of that authority! \n");
		return;
	}

	if (sRet == 6)
	{
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "Some error has occured during database process. Please try again!");
		else printf("Some error has occured during database process. Please try again! \n");
		return;
	}
}
#pragma endregion


uint8 CDBAgent::UpdateCharacterName(std::string& strAccountID, std::string& strUserID, std::string& strNewUserID)
{
	uint8 sRet = 1;
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return sRet;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strNewUserID.c_str(), strNewUserID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL CHANGE_NEW_ID(?, ?, ?)}"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return 1;
	}
	return sRet;
}

uint8 CDBAgent::UpdateCharacterClanName(std::string strClanID, std::string strNewClanID, CUser* puser)
{
	if (puser == nullptr
		|| puser->GetAccountName().empty()
		|| puser->GetAccountName().length() > MAX_ID_SIZE
		|| puser->GetName().empty()
		|| puser->GetName().length() > MAX_ID_SIZE)
		return (uint8)0;

	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)	return (uint8)0;

	uint8 sRet = 1;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, puser->GetAccountName().c_str(), puser->GetAccountName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, puser->GetName().c_str(), puser->GetName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strClanID.c_str(), strClanID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strNewClanID.c_str(), strNewClanID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL CHANGE_NEW_CLANID(?, ?, ?, ?)}"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return (uint8)0;
	}
	return sRet;
}

void CDBAgent::UpdateBattleEvent(string& strCharID, uint8 bNation)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("UPDATE BATTLE SET byNation=%d, strUserName=? WHERE sIndex=%d"), bNation, g_pMain->m_nServerNo)))
		ReportSQLError(m_GameDB->GetError());
}

void CDBAgent::AccountLogout(string& strAccountID)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("{CALL ACCOUNT_LOGOUT(?)}")))
		ReportSQLError(m_AccountDB->GetError());
}

void CDBAgent::UpdateConCurrentUserCount(int nServerNo, int nZoneNo, int nCount)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("UPDATE CONCURRENT SET zone%d_count = %d WHERE serverid = %d"), nZoneNo, nCount, nServerNo)))
		ReportSQLError(m_AccountDB->GetError());
}

// This is what everything says it should do, 
// but the client doesn't seem to care if it's over 1
uint8 CDBAgent::GetUnreadLetterCount(string& strCharID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	uint8 bCount = 0;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bCount);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(_T("{? = CALL MAIL_BOX_CHECK_COUNT(?)}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return 0;
	}

	return bCount;
}

bool CDBAgent::GetLetterList(string& strCharID, Packet& result, bool bNewLettersOnly /* = true*/)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	int8 bCount = 0;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bCount);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL MAIL_BOX_REQUEST_LIST(?, %d)}"), bNewLettersOnly)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	result << uint8(1);
	int offset = (int)result.wpos();
	result << bCount; // placeholder for count

	if (!dbCommand->hasData())
		return true;

	result.SByte();
	do
	{
		string strSubject, strSender;
		uint32 nLetterID, nItemID, nCoins, nDate;
		uint16 sCount, sDaysRemaining;
		uint8 bStatus, bType;

		dbCommand->FetchUInt32(1, nLetterID);
		dbCommand->FetchByte(2, bStatus);
		dbCommand->FetchByte(3, bType);
		dbCommand->FetchString(4, strSubject);
		dbCommand->FetchString(5, strSender);
		dbCommand->FetchByte(6, bType);
		dbCommand->FetchUInt32(7, nItemID);
		dbCommand->FetchUInt16(8, sCount);
		dbCommand->FetchUInt32(9, nCoins);
		dbCommand->FetchUInt32(10, nDate);
		dbCommand->FetchUInt16(11, sDaysRemaining);

		result << nLetterID // letter ID
			<< bStatus  // letter status, doesn't seem to affect anything
			<< strSubject << strSender
			<< bType;

		if (bType == 2)
			result << nItemID << sCount << nCoins;

		result << nDate // date (yy*10000 + mm*100 + dd)
			<< sDaysRemaining;

	} while (dbCommand->MoveNext());

	result.put(offset, bCount); // set count now that the result set's been read

	return true;
}

int8 CDBAgent::SendLetter(string& strSenderID, string& strRecipientID, string& strSubject, string& strMessage, uint8 bType, _ITEM_DATA* pItem, int32 nCoins)
{
	uint64 nSerialNum = 0;
	uint32 nItemID = 0, nItemExpiration = 0;
	uint16 sCount = 0, sDurability = 0;
	int8 bRet = 0;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	// This is a little bit redundant, but best to be sure.
	if (bType == 2
		&& pItem != nullptr)
	{
		nItemID = pItem->nNum;
		sCount = pItem->sCount;
		sDurability = pItem->sDuration;
		nSerialNum = pItem->nSerialNum;
		nItemExpiration = pItem->nExpirationTime;
	}

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strSenderID.c_str(), strSenderID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strRecipientID.c_str(), strRecipientID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strSubject.c_str(), strSubject.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strMessage.c_str(), strMessage.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL MAIL_BOX_SEND(?, ?, ?, ?, %d, %d, %d, %d, " I64FMTD ",%d, %d)}"),
		bType, nItemID, sCount, sDurability, nSerialNum, nItemExpiration, nCoins)))
	{
		ReportSQLError(m_GameDB->GetError());
		return 0;
	}
	return bRet;
}

bool CDBAgent::ReadLetter(string& strCharID, uint32 nLetterID, string& strMessage)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{CALL MAIL_BOX_READ(?, %d)}"), nLetterID)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	dbCommand->FetchString(1, strMessage);
	return true;
}

int8 CDBAgent::GetItemFromLetter(string& strCharID, uint32 nLetterID, uint32& nItemID, uint16& sCount, uint16& sDurability, uint32& nCoins, uint64& nSerialNum, uint32& nExpiTime)
{
	int8 bRet = -1; // error
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL MAIL_BOX_GET_ITEM(?, %d)}"), nLetterID)))
		ReportSQLError(m_GameDB->GetError());

	if (dbCommand->hasData())
	{
		dbCommand->FetchUInt32(1, nItemID);
		dbCommand->FetchUInt16(2, sCount);
		dbCommand->FetchUInt16(3, sDurability);
		dbCommand->FetchUInt32(4, nCoins);
		dbCommand->FetchUInt64(5, nSerialNum);
		dbCommand->FetchUInt32(6, nExpiTime);
	}

	return bRet = 1;
}

void CDBAgent::DeleteLetter(string& strCharID, uint32 nLetterID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	// NOTE: The official implementation passes all 5 letter IDs.
	if (!dbCommand->Execute(string_format(_T("UPDATE MAIL_BOX SET bDeleted = 1 WHERE nLetterID = %d AND strRecipientID = ?"), nLetterID)))
		ReportSQLError(m_GameDB->GetError());
}

#pragma region King System Related Functions

#pragma region CDBAgent::UpdateElectionStatus(uint8 byType, uint8 byNation)

/**
* @brief	Updates the election status.
*
* @param	byType  	Election status.
* @param	byNation	Electoral nation.
*/
void CDBAgent::UpdateElectionStatus(uint8 byType, uint8 byNation)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("{CALL KING_UPDATE_ELECTION_STATUS(%d, %d)}"), byType, byNation)))
		ReportSQLError(m_GameDB->GetError());

	if (byType == ELECTION_TYPE_ELECTION)
	{
		CKingSystem* pKingSystem = g_pMain->m_KingSystemArray.GetData(byNation);
		if (pKingSystem != nullptr)
		{
			if (dbCommand->hasData())
			{
				dbCommand->FetchString(1, pKingSystem->m_strKingName);
				dbCommand->FetchUInt16(2, pKingSystem->m_sYear);
				dbCommand->FetchByte(3, pKingSystem->m_byMonth);
				dbCommand->FetchByte(4, pKingSystem->m_byDay);
				dbCommand->FetchByte(5, pKingSystem->m_byHour);
				dbCommand->FetchByte(6, pKingSystem->m_byMinute);

				CUser* pUser = g_pMain->GetUserPtr(pKingSystem->m_strKingName, NameType::TYPE_CHARACTER);

				if (pUser != nullptr && pUser->isInGame())
				{
					pUser->SendMyInfo();

					pUser->UserInOut(INOUT_OUT);
					pUser->SetRegion(pUser->GetNewRegionX(), pUser->GetNewRegionZ());
					pUser->UserInOut(INOUT_WARP);

					g_pMain->UserInOutForMe(pUser);
					pUser->NpcInOutForMe();
					g_pMain->MerchantUserInOutForMe(pUser);

					pUser->ResetWindows();

					pUser->InitType4();
					pUser->RecastSavedMagic();
				}
			}
		}
	}
}
#pragma endregion

#pragma region CDBAgent::UpdateElectionList(uint8 byDBType, uint8 byType, uint8 byNation, uint16 sKnights, uint32 nAmount, string & strNominee)

/**
* @brief	Updates the election list.
*
* @param	byDBType  	Procedure-specific database action.
* 						If 1, insert. If 2, delete.
* @param	byType	  	Flag to specify what the user's in the election list for (election, impeachment, and thereof).
* @param	byNation  	Electoral nation.
* @param	sKnights  	The nominee's clan ID.
* @param	nAmount		Vote count.
* @param	strNominee	The nominee's name.
*/
void CDBAgent::UpdateElectionList(uint8 byDBType, uint8 byType, uint8 byNation, uint16 sKnights, uint32 nAmount, string& strNominee)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strNominee.c_str(), strNominee.length());
	if (!dbCommand->Execute(string_format(_T("{CALL KING_UPDATE_ELECTION_LIST(%d, %d, %d, %d, %d, ?)}"),
		byDBType, byType, byNation, sKnights, nAmount)))
		ReportSQLError(m_GameDB->GetError());
}

#pragma endregion

#pragma region CDBAgent::UpdateCandidacyRecommend(std::string & strNominator, std::string & strNominee, uint8 byNation)

/**
* @brief	Nominates/recommends strNominee as King.
*
* @param	strNominator	The nominator.
* @param	strNominee  	The nominee.
* @param	byNation		Their nation.
*
* @return	.
*/
int16 CDBAgent::UpdateCandidacyRecommend(std::string& strNominator, std::string& strNominee, uint8 byNation)
{
	int16 sRet = -6;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strNominator.c_str(), strNominator.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strNominee.c_str(), strNominee.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KING_CANDIDACY_RECOMMEND(?, ?, %d)}"), byNation)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::UpdateElectionVoteList(uint8 byNation, std::string & strVoterAccountID, std::string & strVoterUserID, std::string & strNominee)
/**
* @brief	Updates the election vote list.
*
* @param	byNation  	Electoral nation.
* @param	strVoterAccountID  	Voter User Account ID
* @param	strVoterUserID		Voter User ID
* @param	strNominee	The nominee's name.
*/
int16 CDBAgent::UpdateElectionVoteList(uint8 byNation, std::string& strVoterAccountID, std::string& strVoterUserID, std::string& strNominee)
{
	int16 bRet = -6; // error
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strVoterAccountID.c_str(), strVoterAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strVoterUserID.c_str(), strVoterUserID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strNominee.c_str(), strNominee.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL KING_ELECTION_PROC(?, ?, %d, ?)}"), byNation)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

#pragma endregion

#pragma region CDBAgent::GetElectionResults(uint8 byNation)
/**
* @brief	Updates the election vote list.
*
* @param	byNation  	Electoral nation.
*/
void CDBAgent::GetElectionResults(uint8 byNation)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("{CALL KING_ELECTION_RESULTS(%d)}"), byNation)))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData())
		return;

	CKingSystem* pKingSystem = g_pMain->m_KingSystemArray.GetData(byNation);
	if (pKingSystem == nullptr)
		return;

	pKingSystem->m_bElectionUnderProgress = true;
	// All db processes are done.
	uint16 count = 0;
	do
	{
		string strUserID;
		uint8 bRank, bMonth;
		uint16 sClanID, sYear;
		uint32 kingvotes, totalvotes;
		// update the king system respectively.
		dbCommand->FetchString(1, strUserID);
		dbCommand->FetchByte(2, bRank);
		dbCommand->FetchUInt16(3, sClanID);
		dbCommand->FetchByte(4, bMonth);
		dbCommand->FetchUInt16(5, sYear);
		dbCommand->FetchUInt32(6, kingvotes);
		dbCommand->FetchUInt32(7, totalvotes);

		pKingSystem->m_byNextMonth = bMonth;
		pKingSystem->m_sNextYear = sYear;

		if (bRank == 2)
		{
			_KING_ELECTION_LIST* pEntry = new _KING_ELECTION_LIST;
			pEntry->sKnights = sClanID;
			pEntry->nVotes = 0;
			pKingSystem->m_senatorList.insert(make_pair(strUserID, pEntry));
		}
		else if (bRank == 1) // this is the king!
		{
			pKingSystem->m_strNewKingName = strUserID;
			pKingSystem->m_sKingClanID = sClanID;
			pKingSystem->m_KingVotes = kingvotes;
			pKingSystem->m_TotalVotes = totalvotes;
		}
	} while (dbCommand->MoveNext());

	pKingSystem->m_bElectionUnderProgress = false;
}

#pragma endregion

#pragma region CDBAgent::UpdateCandidacyNoticeBoard(string & strCharID, uint8 byNation, string & strNotice)

/**
* @brief	Updates the candidacy notice board.
*
* @param	strCharID	Candidate's name.
* @param	byNation 	Candidate's nation.
* @param	strNotice	The notice.
*/
void CDBAgent::UpdateCandidacyNoticeBoard(string& strCharID, uint8 byNation, string& strNotice)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strNotice.c_str(), strNotice.length());
	if (!dbCommand->Execute(string_format(_T("{CALL KING_CANDIDACY_NOTICE_BOARD_PROC(?, %d, ?)}"), byNation)))
		ReportSQLError(m_GameDB->GetError());
}
#pragma endregion

#pragma region CDBAgent::UpdateNoahOrExpEvent(uint8 byType, uint8 byNation, uint8 byAmount, uint8 byDay, uint8 byHour, uint8 byMinute, uint16 sDuration, uint32 nCoins)

void CDBAgent::UpdateNoahOrExpEvent(uint8 byType, uint8 byNation, uint8 byAmount, uint8 byDay, uint8 byHour, uint8 byMinute, uint16 sDuration, uint32 nCoins)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("{CALL KING_UPDATE_NOAH_OR_EXP_EVENT(%d, %d, %d, %d, %d, %d, %d %d)}"),
		byType, byNation, byAmount, byDay, byHour, byMinute, sDuration, nCoins)))
		ReportSQLError(m_GameDB->GetError());
}
#pragma endregion

#pragma region CDBAgent::UpdateKingSystemDB(uint8 byNation, uint32 nNationalTreasury, uint32 KingTax)

void CDBAgent::UpdateKingSystemDB(uint8 byNation, uint32 nNationalTreasury, uint32 KingTax)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	int16 sRet = -2;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KING_UPDATE_DATABASE(%d, %d, %d)}"), byNation, nNationalTreasury, KingTax)))
		ReportSQLError(m_GameDB->GetError());
}

#pragma endregion

#pragma region CDBAgent::InsertPrizeEvent(uint8 byType, uint8 byNation, uint32 nCoins, std::string & strCharID)

void CDBAgent::InsertPrizeEvent(uint8 byType, uint8 byNation, uint32 nCoins, std::string& strCharID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	int16 sRet = -2;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL KING_INSERT_PRIZE_EVENT(%d, %d, %d, ?)}"),
		byType, byNation, nCoins)))
		ReportSQLError(m_GameDB->GetError());
}

#pragma endregion

#pragma region CDBAgent::InsertTaxEvent(uint8 opcode, uint8 TerritoryTariff, uint8 Nation, uint32 TerritoryTax)
void CDBAgent::InsertTaxEvent(uint8 opcode, uint8 TerritoryTariff, uint8 Nation, uint32 TerritoryTax)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	int16 sRet = -2;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KING_CHANGE_TAX (%d, %d, %d, %d)}"), opcode, Nation, TerritoryTariff, TerritoryTax)))
		ReportSQLError(m_GameDB->GetError());
}
#pragma endregion

#pragma region CDBAgent::UpdateNationIntro(uint32 iServerNo, uint8 nation, std::string & strKingName, std::string &strKingNotice)
void CDBAgent::UpdateNationIntro(uint32 iServerNo, uint8 nation, std::string& strKingName, std::string& strKingNotice)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strKingName.c_str(), strKingName.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strKingNotice.c_str(), strKingNotice.length());


	if (nation == 1)
	{
		if (!dbCommand->Execute(string_format(_T("UPDATE GAME_SERVER_LIST SET strKarusKingName = ?, strKarusNotice = ? WHERE sGroupID = %d"), iServerNo)))
		{
			ReportSQLError(m_AccountDB->GetError());
			return;
		}
	}
	else if (nation == 2)
	{
		if (!dbCommand->Execute(string_format(_T("UPDATE GAME_SERVER_LIST SET strElMoradKingName = ?, strElMoradNotice = ? WHERE sGroupID = %d"), iServerNo)))
		{
			ReportSQLError(m_AccountDB->GetError());
			return;
		}
	}
}

// JstKO Auto King Name Update Eklendi 01.01.2025
void CDBAgent::UpdateKingList(std::string& strCharID, uint8 Nation)
{

	if (strCharID.empty())
		return;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
	{
		return;
	}

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (Nation == ELMORAD)
	{
		if (!dbCommand->Execute(_T("UPDATE GAME_SERVER_LIST SET strElMoradKingName = ?")))
			ReportSQLError(m_GameDB->GetError());
	}
	else if (Nation == KARUS)
	{
		if (!dbCommand->Execute(_T("UPDATE GAME_SERVER_LIST SET strKarusKingName = ?")))
			ReportSQLError(m_GameDB->GetError());
	}
	else
		return;

}
// JstKO Auto King Name Update Eklendi The End 01.01.2025

#pragma endregion

void CDBAgent::LoadUpgrade()
{


	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;



	if (!dbCommand->Execute(_T("select nIndex,nOriginNumber,nNewNumber,nReqItem from NEW_UPGRADE2369")))
		ReportSQLError(m_GameDB->GetError());

	g_pMain->m_loadUpgrade.clear();

	uint32 rCount = 1;
	uint32 nIndex;

	if (dbCommand->hasData())
	{
		do
		{
			_UPGRADE_LOAD* pEvent = new _UPGRADE_LOAD;
			dbCommand->FetchUInt32(1, nIndex);
			dbCommand->FetchUInt32(2, pEvent->ItemNumber);
			dbCommand->FetchUInt32(3, pEvent->NewItemNumber);
			dbCommand->FetchUInt32(4, pEvent->ScrollNumber);



			g_pMain->m_loadUpgrade.insert(std::make_pair(nIndex, pEvent));
			rCount++;


		} while (dbCommand->MoveNext());
	}


}


#pragma endregion
void CDBAgent::LoadItemUpgradeSettings()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

#if GAME_SOURCE_VERSION  == 2369
	if (!dbCommand->Execute(_T("SELECT ReqItemID1,ReqItemID2,ItemType,ItemRate,ItemGrade,ItemReqCoins,SuccessRate,ItemUpgradeStatus FROM ITEM_UPGRADE_SETTINGS2369")))
#elif GAME_SOURCE_VERSION  == 1098
	if (!dbCommand->Execute(_T("SELECT ReqItemID1,ReqItemID2,ItemType,ItemRate,ItemGrade,ItemReqCoins,SuccessRate,ItemUpgradeStatus FROM ITEM_UPGRADE_SETTINGS1098")))
#elif GAME_SOURCE_VERSION  == 1534
	if (!dbCommand->Execute(_T("SELECT ReqItemID1,ReqItemID2,ItemType,ItemRate,ItemGrade,ItemReqCoins,SuccessRate,ItemUpgradeStatus FROM ITEM_UPGRADE_SETTINGS1534")))
#endif
		ReportSQLError(m_GameDB->GetError());

	g_pMain->m_UpgradeSettings.clear();
	uint32 rCount = 1;


	if (dbCommand->hasData())
	{
		do
		{
			_UPGRADE_SETTINGS* pUpgrade = new _UPGRADE_SETTINGS;
			dbCommand->FetchUInt32(1, pUpgrade->nReqItem1);
			dbCommand->FetchUInt32(2, pUpgrade->nReqItem2);
			dbCommand->FetchByte(3, pUpgrade->ItemType);
			dbCommand->FetchByte(4, pUpgrade->ItemRate);
			dbCommand->FetchByte(5, pUpgrade->ItemGrade);
			dbCommand->FetchUInt32(6, pUpgrade->nReqCoins);
			dbCommand->FetchUInt32(7, pUpgrade->SuccesRate);
			dbCommand->FetchByte(8, pUpgrade->ItemUpgradeStatus);




			g_pMain->m_UpgradeSettings.insert(std::make_pair(rCount, pUpgrade));
			rCount++;


		} while (dbCommand->MoveNext());
	}

}

void CDBAgent::LoadCollectionReward(uint16 sEventID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;
#if GAME_SOURCE_VERSION  == 2369
	if (!dbCommand->Execute(string_format("select nItemID,nItemCount,nItemTime,nRate, nItemSession from COLLECTION_RACE_EVENT_REWARD2369 where EventID=%d", sEventID)))
#elif GAME_SOURCE_VERSION  == 1098
	if (!dbCommand->Execute(string_format("select nItemID,nItemCount,nItemTime,nRate, nItemSession from COLLECTION_RACE_EVENT_REWARD1098 where EventID=%d", sEventID)))
#elif GAME_SOURCE_VERSION  == 1534
	if (!dbCommand->Execute(string_format("select nItemID,nItemCount,nItemTime,nRate, nItemSession from COLLECTION_RACE_EVENT_REWARD1534 where EventID=%d", sEventID)))
#endif
		ReportSQLError(m_GameDB->GetError());

	if (dbCommand->hasData())
	{
		int i = 0;
		do
		{
			if (i > 10) return;

			int field = 1;
			dbCommand->FetchUInt32(field++, g_pMain->pCollectionRaceEvent.RewardItemID[i]);
			dbCommand->FetchUInt32(field++, g_pMain->pCollectionRaceEvent.RewardItemCount[i]);
			dbCommand->FetchUInt32(field++, g_pMain->pCollectionRaceEvent.RewardItemTime[i]);
			dbCommand->FetchByte(field++, g_pMain->pCollectionRaceEvent.RewardItemRate[i]);
			dbCommand->FetchByte(field++, g_pMain->pCollectionRaceEvent.RewardSession[i]);

			if (g_pMain->pCollectionRaceEvent.RewardItemRate[i] > 100)
				g_pMain->pCollectionRaceEvent.RewardItemRate[i] = 100;

			i++;


		} while (dbCommand->MoveNext());
	}
}
void CDBAgent::SpecialStone()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(_T("SELECT nIndex,ZoneID,MainNPC,SummonNPC,MonsterName,SummonCount FROM K_SPECIAL_STONE where Status=1")))
		ReportSQLError(m_GameDB->GetError());

	g_pMain->m_SpecialStonearray.clear();

	if (dbCommand->hasData())
	{
		do
		{
			SPECIAL_STONE* rStone = new SPECIAL_STONE;
			dbCommand->FetchUInt32(1, rStone->nIndex);
			dbCommand->FetchByte(2, rStone->ZoneID);
			dbCommand->FetchUInt16(3, rStone->MainNpcID);
			dbCommand->FetchUInt16(4, rStone->NpcID);
			dbCommand->FetchString(5, rStone->MonsterName);
			dbCommand->FetchByte(6, rStone->sCount);




			g_pMain->m_SpecialStonearray.insert(std::make_pair(rStone->nIndex, rStone));

		} while (dbCommand->MoveNext());
	}
	printf("Load K_SPECIAL Tables data size %d \n", (int)g_pMain->m_SpecialStonearray.size());
}
/**
* @brief	Resets the monthly NP total accumulated in the last month.
*/
void CDBAgent::ResetLoyaltyMonthly()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(_T("{CALL RESET_LOYALTY_MONTHLY}")))
		ReportSQLError(m_GameDB->GetError());
}

/**
* @brief	Clears the remaining users who were connected to this server
from the logged in user list that may still be there as the
result of an improper shutdown.
*/
void CDBAgent::ClearRemainUsers()
{
	_ZONE_SERVERINFO* pInfo = g_pMain->m_ServerArray.GetData(g_pMain->m_nServerNo);
	if (pInfo == nullptr)
		return;

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pInfo->strServerIP.c_str(), pInfo->strServerIP.length());
	if (!dbCommand->Execute(_T("{CALL CLEAR_REMAIN_USERS(?)}")))
		ReportSQLError(m_AccountDB->GetError());
}

void CDBAgent::InsertUserDailyOp(_USER_DAILY_OP* pUserDailyOp)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUserDailyOp->strUserId.c_str(), pUserDailyOp->strUserId.length());
	if (!dbCommand->Execute(string_format(_T("{CALL INSERT_USER_DAILY_OP(?, %d, %d, %d, %d, %d, %d, %d, %d)}"),
		pUserDailyOp->ChaosMapTime, pUserDailyOp->UserRankRewardTime, pUserDailyOp->PersonalRankRewardTime, pUserDailyOp->KingWingTime)))
		ReportSQLError(m_GameDB->GetError());
}

void CDBAgent::UpdateUserDailyOp(std::string strUserId, uint8 type, int32 sUnixTime)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserId.c_str(), strUserId.length());
	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_USER_DAILY_OP(?, %d, %d)}"), type, sUnixTime)))
		ReportSQLError(m_GameDB->GetError());
}

void CDBAgent::UpdateRanks()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(_T("{CALL UPDATE_RANKS}")))
		ReportSQLError(m_GameDB->GetError());
}

int8 CDBAgent::NationTransfer(std::string strAccountID)
{
	int8 bRet = 3;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());

	if (!dbCommand->Execute(_T("{? = CALL ACCOUNT_NATION_TRANSFER(?)}")))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

bool CDBAgent::GetAllCharInfo(string& strCharID, uint16& nNum, _NATION_DATA* m_NationTransfer)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(_T("SELECT USERDATA.strUserID, Race, Class, Face, HairRGB, Knights FROM USERDATA \
							   INNER JOIN USER_KNIGHTDATA ON USER_KNIGHTDATA.strUserID = USERDATA.strUserID \
							   WHERE USERDATA.strUserID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	string dCharName;
	uint8 dRace, bFace;
	uint16 dClass, sClanID;
	uint32 nHair;

	if (dbCommand->hasData())
	{
		dbCommand->FetchString(1, dCharName);
		dbCommand->FetchByte(2, dRace);
		dbCommand->FetchUInt16(3, dClass);
		dbCommand->FetchByte(4, bFace);
		dbCommand->FetchUInt32(5, nHair);
		dbCommand->FetchUInt16(6, sClanID);
	}
	else
		return false;

	m_NationTransfer->nNum = nNum;
	m_NationTransfer->bCharName = dCharName;
	m_NationTransfer->bRace = dRace;
	m_NationTransfer->sClass = dClass;
	m_NationTransfer->bFace = bFace;
	m_NationTransfer->nHair = nHair;
	m_NationTransfer->sClanID = sClanID;

	nNum++;
	return true;
}

bool CDBAgent::GetAllCharInfo(std::string strCharID, _NATION_DATA& m_NationTransfer)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(_T("SELECT strUserID, Race, Class, Face, HairRGB, Knights FROM USERDATA WHERE strUserID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData()) return false;

	std::string dCharName;
	uint8 dRace, bFace;
	uint16 dClass, sClanID;
	uint32 nHair;

	dbCommand->FetchString(1, dCharName);
	dbCommand->FetchByte(2, dRace);
	dbCommand->FetchUInt16(3, dClass);
	dbCommand->FetchByte(4, bFace);
	dbCommand->FetchUInt32(5, nHair);
	dbCommand->FetchUInt16(6, sClanID);

	m_NationTransfer.bCharName = dCharName;
	m_NationTransfer.bRace = dRace;
	m_NationTransfer.sClass = dClass;
	m_NationTransfer.bFace = bFace;
	m_NationTransfer.nHair = nHair;
	m_NationTransfer.sClanID = sClanID;
	return true;
}

int8 CDBAgent::NationTransferSaveUser(std::string strAccountID, std::string strCharID, uint16 nNum, uint8 bnewRace, uint16 bNewClass, uint8 bNewNation, uint8 bNewFace, uint32 iNewHair)
{
	int8 bRet = 3;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL NATION_TRANSFER_SAVEUSER(?, ?, %d, %d, %d, %d, %d, %d)}"), nNum, bnewRace, bNewClass, bNewNation, bNewFace, iNewHair)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

#pragma region CDBAgent::UpdateSiegeWarfareDB(uint32 nMoradonTax, uint32 nDelosTax, uint32 nDungeonCharge)

void CDBAgent::UpdateSiegeWarfareDB(uint32 nMoradonTax, uint32 nDelosTax, uint32 nDungeonCharge)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	int16 sRet = -2;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL UPDATE_SIEGE_DATABASE(%d, %d, %d)}"), nMoradonTax, nDelosTax, nDungeonCharge)))
		ReportSQLError(m_GameDB->GetError());
}

void CDBAgent::UpdateSiege(int16 m_sCastleIndex, int16 m_sMasterKnights, int16 m_bySiegeType, int16 m_byWarDay, int16 m_byWarTime, int16 m_byWarMinute)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_SIEGE (%d, %d, %d, %d, %d, %d)}"), m_sCastleIndex, m_sMasterKnights, m_bySiegeType, m_byWarDay, m_byWarTime, m_byWarMinute)))
		ReportSQLError(m_AccountDB->GetError());
}

void CDBAgent::UpdateSiegeTax(uint8 Zone, int16 ZoneTarrif)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return;

	if (Zone == ZONE_DELOS)
	{
		if (!dbCommand->Execute(string_format(_T("UPDATE KNIGHTS_SIEGE_WARFARE SET sDellosTariff = %d"), ZoneTarrif)))
			ReportSQLError(m_GameDB->GetError());
	}
	else if (Zone >= ZONE_MORADON && Zone <= ZONE_MORADON5)
	{
		if (!dbCommand->Execute(string_format(_T("UPDATE KNIGHTS_SIEGE_WARFARE SET sMoradonTariff = %d"), ZoneTarrif)))
			ReportSQLError(m_GameDB->GetError());
	}
	else if (Zone == 0)
	{
		if (!dbCommand->Execute(string_format(_T("UPDATE KNIGHTS_SIEGE_WARFARE SET nDungeonCharge = 0, nMoradonTax = 0, nDellosTax = 0"))))
			ReportSQLError(m_GameDB->GetError());
	}
}
#pragma endregion


uint16 CDBAgent::GetClanIDWithCharID(string& strCharID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(_T("SELECT Knights FROM USERDATA WHERE strUserId = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return -1;
	}

	if (!dbCommand->hasData())
		return -1;

	uint16 sKnights = -1;
	dbCommand->FetchUInt16(1, sKnights);

	return sKnights;
}

void CDBAgent::LoadLadderRankList(Packet& result, uint8 bNation, uint8& count)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("SELECT TOP(10)USERDATA.strUserID,  ISNULL(Knights.IDNum, 0 ), ISNULL(Knights.IDName, '' ), ISNULL(Knights.sMarkVersion, '' ) FROM USERDATA FULL OUTER JOIN Knights ON Knights.IDNum = USERDATA.Knights where USERDATA.strUserID IS NOT NULL and USERDATA.Nation = %d and USERDATA.Authority = 1 order by USERDATA.LoyaltyMonthly desc"), bNation)))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData())
		return;

	int i = 1101;
	do
	{
		uint16 sKnightsIndex, sMarkVersion;
		string strKnightsName, strChief;

		dbCommand->FetchString(1, strChief);
		dbCommand->FetchUInt16(2, sKnightsIndex);
		dbCommand->FetchString(3, strKnightsName);
		dbCommand->FetchUInt16(4, sMarkVersion);

		result << uint16(i++)
			<< strChief
			<< uint16(0)
			<< sKnightsIndex
			<< sMarkVersion
			<< strKnightsName
			<< uint16(0);
		count++;
		if (i >= 1110)
			break;
	} while (dbCommand->MoveNext());
}

void CDBAgent::LoadKnightsLeaderList(Packet& result, uint8 bNation, uint8& count)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(_T("SELECT IDNum, Chief, IDName, sMarkVersion, Nation FROM KNIGHTS WHERE Chief IN (SELECT strUserID FROM USERDATA WHERE Authority = 1) ORDER BY Flag DESC, ClanPointFund DESC, Points DESC")))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData())
		return;

	int i = 1201;
	do
	{
		uint16 sKnightsIndex, sMarkVersion;
		string strKnightsName, strChief;
		uint8 Nation;

		dbCommand->FetchUInt16(1, sKnightsIndex);
		dbCommand->FetchString(2, strChief);
		dbCommand->FetchString(3, strKnightsName);
		dbCommand->FetchUInt16(4, sMarkVersion);
		dbCommand->FetchByte(5, Nation);

		if (bNation != Nation) continue;

		result << uint16(i++)
			<< strChief
			<< uint16(0)
			<< sKnightsIndex
			<< sMarkVersion
			<< strKnightsName
			<< uint16(0);

		count++;
		if (i >= 1210)
			break;
	} while (dbCommand->MoveNext());
}

#pragma region Knights Related Database Functions
#include "Knights.h"
#include "KnightsManager.h"

#pragma region CDBAgent::LoadAllKnights(KnightsArray &m_KnightsArray)
/**
* @brief Loads all Knights data on server start.
*
* @param m_KnightsArray	the Knights array used by the GameServerDLG.
*/
bool CDBAgent::LoadAllKnights(KnightsArray& m_KnightsArray)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (!dbCommand->Execute(_T("{CALL KNIGHTS_LOAD}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return true;

	do
	{
		int i = 1;
		CKnights* pData = new CKnights();
		char strItem[WAREHOUSE_MAX * 8], strSerial[WAREHOUSE_MAX * 8];//new
		dbCommand->FetchUInt16(i++, pData->m_sIndex);
		dbCommand->FetchByte(i++, pData->m_byFlag);
		dbCommand->FetchByte(i++, pData->m_byNation);
		dbCommand->FetchByte(i++, pData->m_byRanking);
		dbCommand->FetchString(i++, pData->m_strName);
		dbCommand->FetchUInt16(i++, pData->m_sDomination);
		dbCommand->FetchBinary(i++, pData->m_Image, sizeof(pData->m_Image));
		dbCommand->FetchUInt16(i++, pData->m_sMarkVersion);
		dbCommand->FetchUInt16(i++, pData->m_sMarkLen);
		dbCommand->FetchUInt16(i++, pData->m_sCape);
		dbCommand->FetchByte(i++, pData->m_bCapeR);
		dbCommand->FetchByte(i++, pData->m_bCapeG);
		dbCommand->FetchByte(i++, pData->m_bCapeB);
		dbCommand->FetchInt16(i++, pData->m_castCapeID);
		dbCommand->FetchByte(i++, pData->m_bCastCapeR);
		dbCommand->FetchByte(i++, pData->m_bCastCapeG);
		dbCommand->FetchByte(i++, pData->m_bCastCapeB);
		dbCommand->FetchUInt32(i++, pData->m_CastTime);
		dbCommand->FetchUInt16(i++, pData->m_sAlliance);
		dbCommand->FetchUInt32(i++, (uint32&)pData->m_nClanPointFund);
		dbCommand->FetchString(i++, pData->m_strClanNotice);
		dbCommand->FetchByte(i++, pData->m_sClanPointMethod);
		dbCommand->FetchByte(i++, pData->bySiegeFlag);
		dbCommand->FetchUInt16(i++, pData->nLose);
		dbCommand->FetchUInt16(i++, pData->nVictory);
		dbCommand->FetchUInt64(i++, pData->m_nMoney);
		dbCommand->FetchInt32(i++, pData->m_dwTime);
		dbCommand->FetchBinary(i++, strItem, sizeof(strItem));
		dbCommand->FetchBinary(i++, strSerial, sizeof(strSerial));
		dbCommand->FetchUInt32(i++, pData->sPremiumTime);

		ByteBuffer itemBuffer, serialBuffer;
		itemBuffer.append(strItem, sizeof(strItem));
		serialBuffer.append(strSerial, sizeof(strSerial));

		memset(pData->m_sClanWarehouseArray, 0x00, sizeof(pData->m_sClanWarehouseArray));

		int serialcounter = 0;

		for (int i = 0; i < WAREHOUSE_MAX; i++)
		{
			uint64 nSerialNum;
			uint32 nItemID;
			int16 sDurability, sCount;

			itemBuffer >> nItemID >> sDurability >> sCount;
			serialBuffer >> nSerialNum;

			_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
			if (pTable.isnull() || sCount <= 0) continue;

			if (!pTable.m_bCountable && sCount > 1) sCount = 1;
			else if (sCount > ITEMCOUNT_MAX) sCount = ITEMCOUNT_MAX;

			pData->m_sClanWarehouseArray[i].nNum = nItemID;
			pData->m_sClanWarehouseArray[i].sDuration = sDurability;
			pData->m_sClanWarehouseArray[i].sCount = sCount;
			pData->m_sClanWarehouseArray[i].nSerialNum = nSerialNum;
			pData->m_sClanWarehouseArray[i].nExpirationTime = 0;

			/*if (pUser->dupplicate_itemcheck(pData->m_sClanWarehouseArray[i].nSerialNum))
			{
				pData->m_sClanWarehouseArray[i].bFlag = (uint8)ItemFlag::DUPLICATE;
				auto pTable = g_pMain->GetItemPtr(pData->m_sClanWarehouseArray[i].nNum);
				if (!pTable.isnull()) {
					printf("Dupplicate Item WipWarehouse: AccountName:%s - UserName:%s -  [Item Name:%s, ItemNum:%d, SerialNum:%I64d, ItemSlot:%d] \n",
						pUser->GetAccountName().c_str(), pUser->GetName().c_str(), pTable.m_sName.c_str(), pTable.Getnum(), pData->m_sClanWarehouseArray[i].nSerialNum, i);
				}
			}*/
		}

		if (pData->m_CastTime != 0 && (uint32)UNIXTIME < pData->m_CastTime)pData->castcape = true;
		if (!m_KnightsArray.PutData(pData->m_sIndex, pData))
			delete pData;



	} while (dbCommand->MoveNext());

	if (!dbCommand->Execute(_T("{CALL KNIGHTS_LOAD_MEMBERSDATA}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return true;

	uint16 sIDNum, sClass;
	std::string strUserID, strMemo, strAccountID;
	uint32 nDonatedNP, Loyalty, LoyaltyMonthly;
	uint8 fame, level;
	int32 lastLogin;
	do
	{
		int field = 1;
		dbCommand->FetchUInt16(field++, sIDNum);
		dbCommand->FetchString(field++, strAccountID);
		dbCommand->FetchString(field++, strUserID);
		dbCommand->FetchUInt32(field++, nDonatedNP);
		dbCommand->FetchString(field++, strMemo);
		dbCommand->FetchByte(field++, fame);
		dbCommand->FetchUInt16(field++, sClass);
		dbCommand->FetchByte(field++, level);
		dbCommand->FetchInt32(field++, lastLogin);
		dbCommand->FetchUInt32(field++, Loyalty);
		dbCommand->FetchUInt32(field++, LoyaltyMonthly);

		CKnightsManager::AddKnightsUser(sIDNum, strAccountID, strUserID, strMemo, fame, sClass, level, lastLogin, Loyalty, LoyaltyMonthly);
		CKnightsManager::AddUserDonatedNP(sIDNum, strUserID, nDonatedNP, false);

	} while (dbCommand->MoveNext());

	//CKnightsManager::GameLoadUpdateKnights();
	return true;
}
#pragma endregion

void CKnightsManager::GameLoadUpdateKnights()  // Clan ile ilgili t�m g�ncellemeler bu void i�ersinde olu�turulacak! OYUN A�ILIRKEN
{
	foreach_stlmap(itr, g_pMain->m_KnightsArray)
	{
		CKnights* pKnights = itr->second;
		if (pKnights == nullptr)
			continue;

		if (pKnights->m_byFlag < (uint8)ClanTypeFlag::ClanTypeAccredited5)
		{
			if (pKnights->m_byGrade > 3)
			{
				Packet reset(WIZ_DB_SAVE, uint8(ProcDbServerType::ResetKnights));
				reset << pKnights->GetID();
				g_pMain->AddDatabaseRequest(reset);
			}
		}
	}
}

void CDBAgent::KnightsReset(uint16 sClanID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("{CALL KNIGHTS_RESET(%d)}"), sClanID)))
	{
		ReportSQLError(m_GameDB->GetError());
	}
}

#pragma region CDBAgent::CreateKnights(uint16 sClanID, uint8 bNation, string & strKnightsName, string & strChief, uint8 bFlag)

int8 CDBAgent::CreateKnights(uint16 sClanID, uint8 bNation, string& strKnightsName, string& strChief, uint8 bFlag)
{
	int8 bRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strKnightsName.c_str(), strKnightsName.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strChief.c_str(), strChief.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL CREATE_KNIGHTS(%d, %d, %d, ?, ?)}"), sClanID, bNation, bFlag)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsMemberJoin(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberJoin(string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_JOIN(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsMemberRemove(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberRemove(string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_REMOVE(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsMemberLeave(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberLeave(string& strCharID, uint16 sClanID)
{
	uint8 sRet = 2;
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_LEAVE(?, %d)}"), sClanID)))
	{
		ReportSQLError(m_GameDB->GetError());
		return sRet;
	}

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsMemberAdmit(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberAdmit(string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_ADMIT(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsMemberReject(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberReject(string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_REJECT(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsMemberChief(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberChief(string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_CHIEF(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsMemberViceChief(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberViceChief(string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_VICECHIEF(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsMemberOfficer(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberOfficer(string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_OFFICER(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsMemberPunish(string & strCharID, uint16 sClanID)

int CDBAgent::KnightsMemberPunish(string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_PUNISH(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsPointMethodChange(uint16 sClanID, uint8 bDomination)

int CDBAgent::KnightsPointMethodChange(uint16 sClanID, uint8 bDomination)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_POINT_METHOD_CHANGE(%d, %d)}"), sClanID, bDomination)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsDestroy(uint16 sClanID)

int CDBAgent::KnightsDestroy(uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_DESTROY (%d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsSymbolUpdate(uint16 sClanID, uint16 sSymbolSize, char *clanSymbol)

int CDBAgent::KnightsSymbolUpdate(uint16 sClanID, uint16 sSymbolSize, char* clanSymbol)
{
	int16 sRet = 0;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, clanSymbol, MAX_KNIGHTS_MARK, SQL_BINARY);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_UPDATE_MARK(%d, ?, %d)}"), sClanID, sSymbolSize)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsCapeUpdate(uint16 sClanID, uint16 sCapeID, uint8 r, uint8 g, uint8 b)
/**
* @brief	Handles clan cape database updates.
*
* @param	sClanID	Identifier for the clan.
* @param	sCapeID	Identifier for the cape.
* @param	r 	Red colour component.
* @param	g 	Green colour component.
* @param	b 	Blue colour component.
*
* @return	true if it succeeds, false if it fails.
*/
int CDBAgent::KnightsCapeUpdate(uint16 sClanID, uint16 sCapeID, uint8 r, uint8 g, uint8 b)
{
	int16 sRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_UPDATE_CAPE (%d, %d, %d, %d, %d)}"), sClanID, sCapeID, r, g, b)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsCastellanCapeUpdate(uint8 type, uint16 sClanID, uint16 sCapeID, uint8 r, uint8 g, uint8 b, uint32 time)
/**
* @brief	Handles clan cape database updates.
*
* @param	sClanID	Identifier for the clan.
* @param	sCapeID	Identifier for the cape.
* @param	r 	Red colour component.
* @param	g 	Green colour component.
* @param	b 	Blue colour component.
* @param	time 	time
*
* @return	true if it succeeds, false if it fails.
*/
int CDBAgent::KnightsCastellanCapeUpdate(uint16 sClanID, uint16 sCapeID, uint8 r, uint8 g, uint8 b, uint32 time)
{
	int16 sRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_CASTELLAN_UPDATE_CAPE (%d, %d, %d, %d, %d, %d)}"), sClanID, sCapeID, r, g, b, time)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsHandover(std::string & strCharID, uint16 sClanID)

int CDBAgent::KnightsHandover(std::string& strCharID, uint16 sClanID)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_HANDOVER(?, %d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsGradeUpdate(uint16 sClanID, uint8 byFlag, uint16 sCapeID)
/**
* @brief	Updates the clan grade.
*
* @param	sClanID	Identifier for the clan.
* @param	byFlag 	The clan type (training, promoted, etc).
* @param	sCapeID	Identifier for the cape.
*/
int CDBAgent::KnightsGradeUpdate(uint16 sClanID, uint8 byFlag, uint16 sCapeID)
{
	int16 sRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_UPDATE_GRADE(%d, %d, %d)}"), sClanID, sCapeID, byFlag)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsClanNoticeUpdate(uint16 sClanID, std::string & strClanNotice)

/**
* @brief	Updates the clan notice.
*
* @param	sClanID		 	Identifier for the clan.
* @param	strClanNotice	The clan notice.
*/
int CDBAgent::KnightsClanNoticeUpdate(uint16 sClanID, std::string& strClanNotice)
{
	int16 sRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (strClanNotice.empty())
	{
		if (!dbCommand->Execute(string_format(_T("UPDATE KNIGHTS SET strClanNotice = NULL WHERE IDNum=%d"), sClanID)))
		{
			ReportSQLError(m_GameDB->GetError());
			return sRet;
		}
		return 0;
	}
	else
	{
		dbCommand->AddParameter(SQL_PARAM_INPUT, strClanNotice.c_str(), strClanNotice.length());
		if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_UPDATE_CLANNOTICE(%d,?)}"), sClanID)))
			ReportSQLError(m_GameDB->GetError());
	}
	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsAllianceNoticeUpdate(uint16 sClanID, std::string & strClanNotice)

/**
* @brief	Updates the clan notice.
*
* @param	sClanID		 	Identifier for the clan.
* @param	strClanNotice	The clan notice.
*/
int CDBAgent::KnightsAllianceNoticeUpdate(uint16 sClanID, std::string& strClanNotice)
{
	int16 sRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strClanNotice.c_str(), strClanNotice.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_ALLIANCE_UPDATE_ALLIANCENOTICE(%d,?)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsFundUpdate(uint16 sClanID, uint32 nClanPointFund)
/**
* @brief	Updates the clan fund.
*
* @param	sClanID		  	Identifier for the clan.
* @param	nClanPointFund	The current clan point fund.
*/
int CDBAgent::KnightsFundUpdate(uint16 sClanID, uint32 nClanPointFund)
{
	int16 sRet = -1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);

	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_UPDATE_FUND(%d, %d)}"), sClanID, nClanPointFund)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsUserMemoUpdate(std::string strUserID, std::string strMemo)
/**
* @brief	Updates the user memo
*
* @param	strUserID	  	the Identifier.
* @param	strMemo			the Memo to be saved..
*/
int CDBAgent::KnightsUserMemoUpdate(std::string strUserID, std::string strMemo)
{
	int16 sRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strMemo.c_str(), strMemo.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_UPDATE_USERMEMO(?,?)}"))))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsUserDonate(CUser * pUser,  uint32 amountNP, uint32 UserAmountNP)
/**
* @brief	Donates (clanPoints) clan points to the specified user's clan.
* 			Also increases the user's total NP donated.
*
* @param	pUser	  	The donor user.
* @param	amountNP  	The number of national points being donated by the user.
*
* @return	true if it succeeds, false if it fails.
*/
int CDBAgent::KnightsUserDonate(CUser* pUser, uint32 amountNP, uint32 UserAmountNP)
{
	int16 sRet = 1;
	if (!pUser) return sRet;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_DONATE(?, %d, %d, %d)}"), pUser->GetClanID(), amountNP, UserAmountNP)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::LoadKnightsAllMembers(uint16 sClanID, Packet & result)

uint16 CDBAgent::LoadKnightsAllMembers(uint16 sClanID, Packet& result)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	int16 sRet = -2;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);

	//if (!dbCommand->Execute(string_format(_T("SELECT strUserID, Fame, [Level], Class FROM USERDATA WHERE Knights = %d"), sClanID)))
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_LOAD_MEMBERS (%d)}"), sClanID)))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return 0;

	uint16 count = 0;
	do
	{
		string strCharID; uint16 sClass; uint8 bFame, bLevel;
		dbCommand->FetchString(1, strCharID);
		dbCommand->FetchByte(2, bFame);
		dbCommand->FetchByte(3, bLevel);
		dbCommand->FetchUInt16(4, sClass);

		result << strCharID << bFame << bLevel << sClass
			// check if user's logged in (i.e. grab logged in state)
			<< uint8(g_pMain->GetUserPtr(strCharID, NameType::TYPE_CHARACTER) == nullptr ? 0 : 1);
		count++;
	} while (dbCommand->MoveNext());

	return count;
}
#pragma endregion

#pragma region CDBAgent::LoadKnightsAllList()

void CDBAgent::LoadKnightsAllList()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	const tstring szSQL = _T("SELECT IDNum, Points, Ranking FROM KNIGHTS WHERE Points != 0 ORDER BY Points DESC");

	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(szSQL))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return;

	Packet result(WIZ_KNIGHTS_PROCESS);
	uint8 bCount = 0;
	int offset;

	do
	{
		if (bCount == 0)
		{
			result.clear();
			offset = (int)result.wpos();
			result << uint8(0);
		}

		uint32 nPoints; uint16 sClanID; uint8 bRanking;
		dbCommand->FetchUInt16(1, sClanID);
		dbCommand->FetchUInt32(2, nPoints);
		dbCommand->FetchByte(3, bRanking);

		result << sClanID << nPoints << bRanking;

		// only send 100 clans at a time (no shared memory limit, yay!)
		if (++bCount >= 100)
		{
			// overwrite the count
			result.put(offset, bCount);

			CKnightsManager::RecvKnightsAllList(result);
			bCount = 0;
		}
	} while (dbCommand->MoveNext());

	// didn't quite make it in the last batch (if any)? send the remainder.
	if (bCount < 100)
	{
		result.put(offset, bCount);
		CKnightsManager::RecvKnightsAllList(result);
	}
}
#pragma endregion

#pragma region CDBAgent::KnightsSave(uint16 sClanID, uint8 bFlag, uint32 nClanFund)
/**
* @brief	Donates (clanPoints) clan points to the specified user's clan.
* 			Also increases the user's total NP donated.
*
* @param	sClanID	  	The donor user.
* @param	bFlag  		The clan flag.
* @param	nClanFund  	The total clan fund.
*
* @return	true if it succeeds, false if it fails.
*/
int CDBAgent::KnightsSave(uint16 sClanID, uint8 bFlag, uint32 nClanFund)
{
	int16 sRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_SAVE(%d, %d, %d)}"), sClanID, bFlag, nClanFund)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}

#pragma endregion

#pragma region CDBAgent::KnightsRefundNP(string & strUserID, uint32 nRefundNP)
/**
* @brief	Refunds the specified amount of NP to a logged out character.
*
* @param	strUserID	Character's name.
* @param	nRefundNP	The amount of NP to refund.
*/
int CDBAgent::KnightsRefundNP(string& strUserID, uint32 nRefundNP)
{
	int16 sRet = 1;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return sRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_MEMBER_REFUNDNP(?, %d)}"), nRefundNP)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma endregion
/////////////////////////////////

/////////////////////////////////
#pragma region Knight Alliance Related Database Methods

#pragma region CDBAgent::KnightsAllianceCreate(uint16 sMainClanID, uint16 sSubClanID, uint8  byEmptyIndex)
/**
* @brief	Initiates a new alliance creation progress.
*
* @param	sMainClanID	the alliance index.
* @param	sSubClanID the knights index.
* @param	byEmptyIndex
*/
int CDBAgent::KnightsAllianceCreate(uint16 sMainClanID, uint16 sSubClanID, uint8  byEmptyIndex)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	int16 sRet = 1;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_ALLIANCE_CREATE(%d, %d, %d)}"), sMainClanID, sSubClanID, byEmptyIndex)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsAllianceInsert(uint16 sMainClanID, uint16 sSubClanID, uint8  byEmptyIndex)
/**
* @brief	Inserts a knight to an existing alliance.
*
* @param	sMainClanID	the alliance index.
* @param	sSubClanID the knights index.
* @param	byEmptyIndex
*/
int CDBAgent::KnightsAllianceInsert(uint16 sMainClanID, uint16 sSubClanID, uint8  byEmptyIndex)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	int16 sRet = 1;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_ALLIANCE_INSERT(%d, %d, %d)}"), sMainClanID, sSubClanID, byEmptyIndex)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsAllianceDestroy(uint16 sAllianceID)
/**
* @brief	Destroys the alliance.
*
* @param	sAllianceID	the alliance index.
*/
int CDBAgent::KnightsAllianceDestroy(uint16 sAllianceID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	int16 sRet = 1;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_ALLIANCE_DESTROY(%d)}"), sAllianceID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsAllianceRemove(uint16 sMainClanID, uint16 sSubClanID)
/**
* @brief	Removes a knight from the alliance.
*
* @param	sMainClanID	the alliance index.
* @param	sSubClanID the knights index.
*/
int CDBAgent::KnightsAllianceRemove(uint16 sMainClanID, uint16 sSubClanID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	int16 sRet = 1;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_ALLIANCE_REMOVE(%d, %d)}"), sMainClanID, sSubClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

#pragma region CDBAgent::KnightsAlliancePunish(uint16 sMainClanID, uint16 sSubClanID)
/**
* @brief	Punishes a knight from the alliance.
*
* @param	sMainClanID	the alliance index.
* @param	sSubClanID the knights index.
*/
int CDBAgent::KnightsAlliancePunish(uint16 sMainClanID, uint16 sSubClanID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	int16 sRet = 1;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL KNIGHTS_ALLIANCE_PUNISH(%d, %d)}"), sMainClanID, sSubClanID)))
		ReportSQLError(m_GameDB->GetError());

	return sRet;
}
#pragma endregion

bool CDBAgent::SaveSheriffReports(std::string ReportedUser, std::string ReportedSheriff, std::string Crime, uint8 Yes, std::string SheriffA, uint8 No, std::string ReportDateTime)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, ReportedUser.c_str(), ReportedUser.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, ReportedSheriff.c_str(), ReportedSheriff.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, Crime.c_str(), Crime.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, SheriffA.c_str(), SheriffA.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, ReportDateTime.c_str(), ReportDateTime.length());


	if (!dbCommand->Execute(string_format(_T("{CALL SHERIF_REPORTS_SAVE(?, ?, ?, %d, ?, %d, ?)}"), Yes, No)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}

bool CDBAgent::UpdateSheriffTable(uint8 Type, std::string ReportedUser, uint8 Yes, uint8 No, std::string Sheriff)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, ReportedUser.c_str(), ReportedUser.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, Sheriff.c_str(), Sheriff.length());

	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_SHERIFF_REPORTS(%d, ?, %d, %d, ?)}"), Type, Yes, No)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (Yes > 2)
	{
		CUser* pSelected = g_pMain->GetUserPtr(ReportedUser, NameType::TYPE_CHARACTER);
		if (pSelected
			&& pSelected->isInGame()) // In game or Not we send him to the prison anyhow :)
			pSelected->ZoneChange(ZONE_PRISON, 167, 125); // Random for now

		g_pMain->m_SheriffArray.erase(ReportedUser);
	}
	else if (No >= 2)
	{
		g_pMain->m_SheriffArray.erase(ReportedUser);
	}
	return true;
}


#pragma endregion

#pragma region CDBAgent::SetAllCharID(string & strAccountID, string & strCharID1, string & strCharID2, string & strCharID3, string & strCharID4)
bool CDBAgent::SetAllCharID(string& strAccountID, string& strCharID1, string& strCharID2, string& strCharID3, string& strCharID4)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	string ID1, ID2, ID3, ID4;

	if (strCharID1 == "")
		ID1 = "";
	else
		ID1 = strCharID1;

	if (strCharID2 == "")
		ID2 = "";
	else
		ID2 = strCharID2;

	if (strCharID3 == "")
		ID3 = "";
	else
		ID3 = strCharID3;

	if (strCharID4 == "")
		ID4 = "";
	else
		ID4 = strCharID4;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, ID1.c_str(), ID1.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, ID2.c_str(), ID2.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, ID3.c_str(), ID3.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, ID4.c_str(), ID4.length());

	if (!dbCommand->Execute(_T("{CALL UPDATE_ALL_CHAR_ID("
		"?, ?, ?, ?, ?)}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}
#pragma endregion

#pragma region CDBAgent::UpdateAchieveData(CUser *pUser)
bool CDBAgent::UpdateAchieveData(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	// achieve system
	char strAchieve[ACHIEVE_ARRAY_SIZE];
	memset(strAchieve, 0, sizeof(strAchieve));

	int index2 = 36;

	*(uint32*)(strAchieve) = pUser->m_AchieveInfo.PlayTime;
	*(uint32*)(strAchieve + 4) = pUser->m_AchieveInfo.MonsterDefeatCount;
	*(uint32*)(strAchieve + 8) = pUser->m_AchieveInfo.UserDefeatCount;
	*(uint32*)(strAchieve + 12) = pUser->m_AchieveInfo.UserDeathCount;
	*(uint32*)(strAchieve + 16) = pUser->m_AchieveInfo.TotalMedal;
	*(uint16*)(strAchieve + 20) = pUser->m_AchieveInfo.RecentAchieve[0];
	*(uint16*)(strAchieve + 22) = pUser->m_AchieveInfo.RecentAchieve[1];
	*(uint16*)(strAchieve + 24) = pUser->m_AchieveInfo.RecentAchieve[2];
	*(uint16*)(strAchieve + 26) = pUser->m_AchieveInfo.NormalCount;
	*(uint16*)(strAchieve + 28) = pUser->m_AchieveInfo.QuestCount;
	*(uint16*)(strAchieve + 30) = pUser->m_AchieveInfo.WarCount;
	*(uint16*)(strAchieve + 32) = pUser->m_AchieveInfo.AdvantureCount;
	*(uint16*)(strAchieve + 34) = pUser->m_AchieveInfo.ChallangeCount;

	foreach_stlmap(itr, pUser->m_AchieveMap)
	{
		if (itr->second == nullptr)
			continue;

		*(uint16*)(strAchieve + index2) = itr->first;
		*(uint8*)(strAchieve + index2 + 2) = itr->second->bStatus;
		*(uint32*)(strAchieve + index2 + 3) = itr->second->iCount[0];
		*(uint32*)(strAchieve + index2 + 7) = itr->second->iCount[1];
		index2 += 11;
	}

	// end of achieve system loading
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)strAchieve, sizeof(strAchieve), SQL_BINARY);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());

	if (!dbCommand->Execute(string_format(_T("UPDATE USER_ACHIEVE_DATA SET sCoverID=%d, sSkillID=%d, sAchieveCount=%d, strAchieve=? WHERE strUserID=?"),
		pUser->m_sCoverID, pUser->m_sSkillID, pUser->m_AchieveMap.GetSize())))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}
#pragma endregion

#pragma region CDBAgent::LoadAchieveData(CUser *pUser)
bool CDBAgent::LoadAchieveData(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	if (pUser->m_AchieveMap.GetSize() > 0)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());
	if (!dbCommand->Execute(_T("SELECT sCoverID, sSkillID, sAchieveCount, strAchieve FROM USER_ACHIEVE_DATA WHERE strUserID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	char strAchieve[ACHIEVE_ARRAY_SIZE];
	memset(strAchieve, 0, sizeof(strAchieve));
	uint16 sAchieveCount = 0;

	dbCommand->FetchUInt16(1, pUser->m_sCoverID);
	dbCommand->FetchUInt16(2, pUser->m_sSkillID);
	dbCommand->FetchUInt16(3, sAchieveCount);
	dbCommand->FetchBinary(4, strAchieve, sizeof(strAchieve));

	// Load User Achieve Data
	pUser->m_AchieveInfo.PlayTime = *(uint32*)(strAchieve);
	pUser->m_AchieveInfo.MonsterDefeatCount = *(uint32*)(strAchieve + 4);
	pUser->m_AchieveInfo.UserDefeatCount = *(uint32*)(strAchieve + 8);
	pUser->m_AchieveInfo.UserDeathCount = *(uint32*)(strAchieve + 12);
	pUser->m_AchieveInfo.TotalMedal = *(uint32*)(strAchieve + 16);
	pUser->m_AchieveInfo.RecentAchieve[0] = *(uint16*)(strAchieve + 20);
	pUser->m_AchieveInfo.RecentAchieve[1] = *(uint16*)(strAchieve + 22);
	pUser->m_AchieveInfo.RecentAchieve[2] = *(uint16*)(strAchieve + 24);
	pUser->m_AchieveInfo.NormalCount = 0;
	pUser->m_AchieveInfo.QuestCount = 0;
	pUser->m_AchieveInfo.WarCount = 0;
	pUser->m_AchieveInfo.AdvantureCount = 0;
	pUser->m_AchieveInfo.ChallangeCount = 0;

	if (sAchieveCount == AchieveChalenngeInComplete)
	{
		foreach_stlmap(itr, g_pMain->m_AchieveMainArray)
		{
			_USER_ACHIEVE_INFO* pAchieve = new _USER_ACHIEVE_INFO;
			_ACHIEVE_MAIN* pAchieveMain = g_pMain->m_AchieveMainArray.GetData(itr->second->sIndex);

			if (pAchieveMain == nullptr)
				continue;

			if (pAchieveMain->byte2 == 41
				|| pAchieveMain->byte2 == 42)
				pAchieve->bStatus = AchieveChalenngeInComplete;
			else
				pAchieve->bStatus = AchieveIncomplete;

			pAchieve->iCount[0] = AchieveChalenngeInComplete;
			pAchieve->iCount[1] = AchieveChalenngeInComplete;
			pUser->m_AchieveMap.PutData(itr->second->sIndex, pAchieve);
		}
	}
	else
	{
		for (int i = 0, index = 36; i < sAchieveCount; i++, index += 11)
		{
			uint16	sAchieveID = *(uint16*)(strAchieve + index);
			uint8	bStatus = *(uint8*)(strAchieve + index + 2);
			uint32	iCount1 = *(uint32*)(strAchieve + index + 3);
			uint32	iCount2 = *(uint32*)(strAchieve + index + 7);

			_USER_ACHIEVE_INFO* pAchieve = new _USER_ACHIEVE_INFO;
			_ACHIEVE_MAIN* pAchieveMain = g_pMain->m_AchieveMainArray.GetData(sAchieveID);

			if (pAchieveMain != nullptr)
			{
				if (bStatus == UserAchieveStatus::AchieveCompleted
					|| bStatus == UserAchieveStatus::AchieveFinished)
				{
					switch (pAchieveMain->AchieveType)
					{
					case 0:
						pUser->m_AchieveInfo.NormalCount++;
						break;
					case 1:
						pUser->m_AchieveInfo.QuestCount++;
						break;
					case 2:
						pUser->m_AchieveInfo.WarCount++;
						break;
					case 3:
						pUser->m_AchieveInfo.AdvantureCount++;
						break;
					case 4:
						pUser->m_AchieveInfo.ChallangeCount++;
						break;
					default:
						break;
					}
				}

				if (pAchieveMain->AchieveType == 4
					&& bStatus == UserAchieveStatus::AchieveIncomplete)
					pAchieve->bStatus = AchieveChalenngeInComplete;
				else
					pAchieve->bStatus = bStatus;

				pAchieve->iCount[0] = iCount1;
				pAchieve->iCount[1] = iCount2;
				pUser->m_AchieveMap.PutData(sAchieveID, pAchieve);
			}
		}
	}

	_ACHIEVE_MAIN* pAchieveMain = g_pMain->m_AchieveMainArray.GetData(pUser->m_sCoverID);
	if (pAchieveMain != nullptr)
		pUser->m_sCoverTitle = pAchieveMain->TitleID;

	pAchieveMain = g_pMain->m_AchieveMainArray.GetData(pUser->m_sSkillID);
	if (pAchieveMain)
	{
		pUser->m_sSkillTitle = pAchieveMain->TitleID;
		_ACHIEVE_TITLE* pAchieveTitle = g_pMain->m_AchieveTitleArray.GetData(pUser->m_sSkillTitle);
		if (pAchieveTitle != nullptr)
		{
			pUser->m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_STR] = pAchieveTitle->Str;
			pUser->m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_STA] = pAchieveTitle->Hp;
			pUser->m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_DEX] = pAchieveTitle->Dex;
			pUser->m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_INT] = pAchieveTitle->Int;
			pUser->m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_CHA] = pAchieveTitle->Mp;
			pUser->m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_ATTACK] = pAchieveTitle->Attack;
			pUser->m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_DEFENCE] = pAchieveTitle->Defence;
		}
	}

	// end of achieve loading
	return true;
}
#pragma endregion

#pragma region int8 CDBAgent::CharacterSealProcess(std::string & strAcountID, std::string & strUserID, std::string & strPassword, uint64 nItemSerial)
/**
* @brief	Proceeds character seal process.
*
* @param	strAccountID	Character's Account name.
* @param	strUserID		the Character to be sealed within the Cypher Ring
* @param	strPassword		the Account Seal Password
*/
int8 CDBAgent::CharacterSealProcess(std::string& strAccountID, std::string& strUserID, std::string& strPassword, uint64 nItemSerial)
{
	uint8 bRet = 1;
	unique_ptr<OdbcCommand> dbCommand2(m_GameDB->CreateCommand());
	if (dbCommand2.get() == nullptr)
		return 3;

	dbCommand2->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand2->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand2->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	if (!dbCommand2->Execute(string_format(_T("{? = CALL CHARACTER_SEAL_PROCEED(?, ?, " I64FMTD ")}"), nItemSerial)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}
#pragma endregion 

int8 CDBAgent::CharacterSealItemCheck(std::string& strUserID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	uint8 bRet = 1;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	if (!dbCommand->Execute(_T("{? = CALL CHARACTER_SEAL_ITEM_CHECK(?)}")))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

std::string CDBAgent::getmypassword(std::string accountid)
{
	std::string my_password = "";
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return my_password;


	dbCommand->AddParameter(SQL_PARAM_INPUT, accountid.c_str(), accountid.length());
	if (!dbCommand->Execute(_T("SELECT strPasswd FROM TB_USER WHERE strAccountID = ?")))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return my_password;

	dbCommand->FetchString(1, my_password);
	return my_password;
}

#pragma region int8 CDBAgent::CharacterUnSealProcess(std::string & strAcountID, uint8 bSelectedIndex, uint64 nItemSerial)
/**
* @brief	Proceeds character seal process.
*
* @param	strAccountID	Character's Account name.
* @param	strUserID		the Character to be sealed within the Cypher Ring
* @param	strPassword		the Account Seal Password
*/
int8 CDBAgent::CharacterUnSealProcess(std::string& strAccountID, uint8 bSelectedIndex, uint64 nItemSerial)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return 3;

	uint8 bRet = 1;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL CHARACTER_UNSEAL_PROCEED(?, %d,  " I64FMTD ")}"), bSelectedIndex, nItemSerial)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

#pragma endregion

#pragma region CDBAgent::LoadAllCharacterSealItems(CharacterSealItemArray & m_CharacterSealArray)
/**
* @brief Loads all Character Seal Items data on server start.
*
* @param m_CharacterSealArray	the Character Seal array used by the GameServerDLG.
*/
bool  CDBAgent::LoadAllCharacterSealItems(CharacterSealItemArray& m_CharacterSealArray)
{
	foreach_stlmap(itr, g_pMain->m_CharacterSealItemArray)
	{
		_CHARACTER_SEAL_ITEM* pData = itr->second;
		if (pData == nullptr)
			continue;

		if (!LoadCharacterSealUserData(pData->strAccountID, pData->m_strUserID, pData))
			return false;

		_CHARACTER_SEAL_ITEM_MAPPING* pDataMapping = new _CHARACTER_SEAL_ITEM_MAPPING;
		pDataMapping->nItemSerial = pData->nSerialNum;
		pDataMapping->nUniqueID = pData->nUniqueID;

		if (!g_pMain->m_CharacterSealItemMapping.PutData(pDataMapping->nUniqueID, pDataMapping))
			return false;
	}

	return true;
}
#pragma endregion

#pragma region CDBAgent::LoadCharacterSealUserData(std::string & strAccountID, std::string & strCharID, _CHARACTER_SEAL_ITEM *pUser)

bool CDBAgent::LoadCharacterSealUserData(std::string& strAccountID, std::string& strCharID, _CHARACTER_SEAL_ITEM* pUser)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	DateTime time;

	if (dbCommand.get() == nullptr)
		return false;

	int8 bRet = -2; // generic error

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(_T("{? = CALL CHARACTER_SEAL_LOAD_USER_DATA(?, ?)}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	char strItem[INVENTORY_TOTAL * 8] = { 0 }, strSerial[INVENTORY_TOTAL * 8] = { 0 }, strItemExpiration[INVENTORY_TOTAL * 4] = { 0 };

	int field = 1;
	pUser->nUniqueID = g_pMain->GenerateItemUniqueID();
	dbCommand->FetchByte(field++, pUser->m_bRace);
	dbCommand->FetchUInt16(field++, pUser->m_sClass);
	dbCommand->FetchUInt32(field++, pUser->m_nHair);
	dbCommand->FetchByte(field++, pUser->m_bRank);
	dbCommand->FetchByte(field++, pUser->m_bTitle);
	dbCommand->FetchByte(field++, pUser->m_bLevel);
	dbCommand->FetchByte(field++, pUser->m_bRebirthLevel);
	dbCommand->FetchInt64(field++, pUser->m_iExp);
	dbCommand->FetchUInt32(field++, pUser->m_iLoyalty);
	dbCommand->FetchByte(field++, pUser->m_bFace);
	dbCommand->FetchByte(field++, pUser->m_bCity);
	dbCommand->FetchInt16(field++, pUser->m_bKnights);
	dbCommand->FetchByte(field++, pUser->m_bFame);
	dbCommand->FetchInt16(field++, pUser->m_sHp);
	dbCommand->FetchInt16(field++, pUser->m_sMp);
	dbCommand->FetchInt16(field++, pUser->m_sSp);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_STR]);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_STA]);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_DEX]);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_INT]);
	dbCommand->FetchByte(field++, pUser->m_bStats[(uint8)StatType::STAT_CHA]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_STR]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_STA]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_DEX]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_INT]);
	dbCommand->FetchByte(field++, pUser->m_bRebStats[(uint8)StatType::STAT_CHA]);
	dbCommand->FetchByte(field++, pUser->m_bAuthority);
	dbCommand->FetchInt16(field++, pUser->m_sPoints);
	dbCommand->FetchUInt32(field++, pUser->m_iGold);
	dbCommand->FetchByte(field++, pUser->m_bZone);
	dbCommand->FetchString(field++, (char*)pUser->m_bstrSkill, sizeof(pUser->m_bstrSkill));
	dbCommand->FetchBinary(field++, strItem, sizeof(strItem));
	dbCommand->FetchBinary(field++, strSerial, sizeof(strSerial));
	dbCommand->FetchBinary(field++, strItemExpiration, sizeof(strItemExpiration));
	dbCommand->FetchUInt32(field++, pUser->m_iMannerPoint);
	dbCommand->FetchUInt32(field++, pUser->m_iLoyaltyMonthly);

	ByteBuffer itemBuffer, serialBuffer, itemexpirationbuffer;
	itemBuffer.append(strItem, sizeof(strItem));
	serialBuffer.append(strSerial, sizeof(strSerial));
	itemexpirationbuffer.append(strItemExpiration, sizeof(strItemExpiration));

	memset(pUser->m_sItemArray, 0x00, sizeof(pUser->m_sItemArray));

	UserRentalMap rentalData;

	// Until this statement is cleaned up, 
	// no other statements can be processed.
	delete dbCommand.release();

	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		uint64 nSerialNum;
		uint32 nItemID;
		uint32 nExpirationTime;
		int16 sDurability, sCount;
		itemBuffer >> nItemID >> sDurability >> sCount;
		serialBuffer >> nSerialNum;
		itemexpirationbuffer >> nExpirationTime;

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || sCount <= 0)
			continue;

		if (!pTable.m_bCountable && sCount > 1)
			sCount = 1;
		else if (sCount > ITEMCOUNT_MAX)
			sCount = ITEMCOUNT_MAX;

		if (nSerialNum == 0)
			nSerialNum = g_pMain->GenerateItemSerial();

		_ITEM_DATA* pItem = pUser->GetItem(i);
		if (pItem == nullptr)
			continue;

		pItem->nNum = nItemID;
		pItem->sDuration = pTable.isAccessory() ? pTable.m_sDuration : sDurability;
		pItem->sCount = sCount;
		pItem->nExpirationTime = nExpirationTime; // set this to 0 for now till the expiration system completed
		pItem->nSerialNum = nSerialNum;
	}
	return true;
}
#pragma endregion

#pragma region CDBAgent::LoadUserSealExpData(CUser *pUser)
bool CDBAgent::LoadUserSealExpData(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());

	if (!dbCommand->Execute(_T("{CALL LOAD_USER_SEAL_EXP(?)}")))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return false;

	dbCommand->FetchUInt32(1, pUser->m_iSealedExp);

	return true;
}
#pragma endregion

#pragma region CDBAgent::UpdateUserSealExpData(CUser *pUser)
bool CDBAgent::UpdateUserSealExpData(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());

	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_USER_SEAL_EXP(?, %d)}"), pUser->m_iSealedExp)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}
#pragma endregion

#pragma region CDBAgent::DrakiSignCreateDatas(std::string & strCharID, CUser *pUser)
uint8 CDBAgent::DrakiDataCheck(CUser* pUser)
{
	uint8 bRet = 0;
	if (pUser == nullptr) return 0;
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return bRet;

	uint8 Class = 0;
	if (pUser->isWarrior()) Class = 1;
	else if (pUser->isRogue()) Class = 2;
	else if (pUser->isMage()) Class = 3;
	else if (pUser->isPriest()) Class = 4;
	else if (pUser->isPortuKurian()) Class = 13;
	if (Class == 0) return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL CHECK_DRAKI_USER(?, %d)}"), Class)))
	{
		ReportSQLError(m_GameDB->GetError());
		return bRet;
	}
	if (!dbCommand->hasData()) return bRet;
	return bRet;
}
#pragma endregion

#pragma region CDBAgent::LoadUserDrakiTowerData(CUser *pUser)
bool CDBAgent::LoadUserDrakiTowerData(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());

	if (!dbCommand->Execute(_T("{CALL LOAD_DRAKI_TOWER(?)}")))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
	{
		pUser->m_iDrakiTime = 3600;
		pUser->m_bDrakiStage = 1;
		pUser->m_bDrakiEnteranceLimit = 3;
	}
	else
	{
		dbCommand->FetchInt32(1, pUser->m_iDrakiTime);
		dbCommand->FetchByte(2, pUser->m_bDrakiStage);
		dbCommand->FetchByte(3, pUser->m_bDrakiEnteranceLimit);
	}

	return true;
}
#pragma endregion

#pragma region CDBAgent::UpdateDaysDrakiTowerLimit()
bool CDBAgent::UpdateDaysDrakiTowerLimit()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;

	if (!dbCommand->Execute(_T("{CALL UPDATE_DRAKI_LIMIT_DAY}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}
#pragma endregion

#pragma region CDBAgent::UpdateDrakiTowerLimitLastUpdate(std::string & strUserID, uint8 EnteranceLimit)
bool CDBAgent::UpdateDrakiTowerLimitLastUpdate(std::string& strUserID, uint8 EnteranceLimit)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_DRAKI_LIMIT(?, %d)}"), EnteranceLimit)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	return true;
}
#pragma endregion


#pragma region CDBAgent::LoadDrakiTowerTables()
bool CDBAgent::LoadDrakiTowerTables()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;

	if (!dbCommand->Execute(_T("{CALL LOAD_DRAKI_MONSTER_LIST}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	do
	{
		DRAKI_MONSTER_LIST* pData = new DRAKI_MONSTER_LIST;

		dbCommand->FetchUInt32(1, pData->nIndex);
		dbCommand->FetchByte(2, pData->bDrakiStage);
		dbCommand->FetchUInt16(3, pData->sMonsterID);
		dbCommand->FetchUInt16(4, pData->sPosX);
		dbCommand->FetchUInt16(5, pData->sPosZ);
		dbCommand->FetchUInt16(6, pData->sDirection);
		dbCommand->FetchByte(7, pData->bMonster);


		if (!g_pMain->m_DrakiMonsterListArray.PutData(pData->nIndex, pData))
			delete pData;

	} while (dbCommand->MoveNext());

	if (!dbCommand->Execute(_T("{CALL LOAD_DRAKI_TOWER_STAGES}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	do
	{
		DRAKI_ROOM_LIST* pData = new DRAKI_ROOM_LIST;

		dbCommand->FetchByte(1, pData->bIndex);
		dbCommand->FetchByte(2, pData->bDrakiStage);
		dbCommand->FetchByte(3, pData->bDrakiSubStage);
		dbCommand->FetchByte(4, pData->bDrakiNpcStage);

		if (!g_pMain->m_DrakiRoomListArray.PutData(pData->bIndex, pData))
			delete pData;

	} while (dbCommand->MoveNext());

	return true;
}
#pragma endregion

#pragma region CDBAgent::UpdateDrakiTowerData(uint32 sTime, uint8 bStage, std::string & strUserID)
void CDBAgent::UpdateDrakiTowerData(uint32 sTime, uint8 bStage, std::string& strUserID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());

	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_DRAKI_TOWER_DATA(?, %d, %d)}"), sTime, bStage)))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return;
}
#pragma endregion

#pragma region CDBAgent::ReqDrakiTowerList(Packet & pkt, CUser* pUser)
void CDBAgent::ReqDrakiTowerList(Packet& pkt, CUser* pUser)
{
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr || pUser == nullptr)
		return;

	if (!dbCommand->Execute(_T("{CALL LOAD_DRAKI_RANK}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	pkt.SByte();
	uint8 Counter = 0;
	if (!dbCommand->hasData())
	{
		for (int i = 0; i < 5; i++)
		{
			Counter++;
			pkt << uint8(++Counter) << "" << uint32(3600) << uint32(1);
		}
		pkt << uint8(-1) << pUser->m_strUserID << uint32(3600) << uint32(1) << uint32(1) << uint32(pUser->m_bDrakiEnteranceLimit);
		return;
	}

	std::string strRankID;
	uint32 pFinishTime, iRank, sIndex, Class;
	uint8  bStage;
	uint16 Counter2 = 0;
	do
	{
		int field = 1;
		dbCommand->FetchUInt32(field++, sIndex);
		dbCommand->FetchUInt32(field++, Class);
		dbCommand->FetchUInt32(field++, iRank);
		dbCommand->FetchString(field++, strRankID);
		dbCommand->FetchByte(field++, bStage);
		dbCommand->FetchUInt32(field++, pFinishTime);

		if (pUser->isWarrior())
		{
			if (Class == 1)
			{
				Counter2++;
				pkt << uint8(iRank) << strRankID << pFinishTime << uint32(bStage);
			}
		}
		else if (pUser->isRogue())
		{
			if (Class == 2)
			{
				Counter2++;
				pkt << uint8(iRank) << strRankID << pFinishTime << uint32(bStage);
			}
		}
		else if (pUser->isMage())
		{
			if (Class == 3)
			{
				Counter2++;
				pkt << uint8(iRank) << strRankID << pFinishTime << uint32(bStage);
			}
		}
		else if (pUser->isPriest())
		{
			if (Class == 4)
			{
				Counter2++;
				pkt << uint8(iRank) << strRankID << pFinishTime << uint32(bStage);
			}
		}
		else if (pUser->isPortuKurian())
		{
			if (Class == 13)
			{
				Counter2++;
				pkt << uint8(iRank) << strRankID << pFinishTime << uint32(bStage);
			}
		}
	} while (dbCommand->MoveNext());

	int8 thyke = (5 - Counter2);
	if (thyke < 0) thyke = 0;

	if (thyke > 0)
	{
		for (int i = 0; i < thyke; i++)
			pkt << uint8(++Counter2) << "" << uint32(3600) << uint32(1);
	}

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->m_strUserID.c_str(), pUser->m_strUserID.length());
	if (!dbCommand->Execute(_T("SELECT * FROM DRAKI_TOWER_RIFT_RANK WHERE strUserID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData())
	{
		pkt << uint8(-1) << pUser->GetName() << uint32(3600) << uint32(1) << uint32(1) << uint32(pUser->m_bDrakiEnteranceLimit);
		return;
	}
	dbCommand->FetchUInt32(1, sIndex);
	dbCommand->FetchUInt32(2, Class);
	dbCommand->FetchUInt32(4, iRank);
	dbCommand->FetchString(5, strRankID);
	dbCommand->FetchByte(6, bStage);
	dbCommand->FetchUInt32(7, pFinishTime);

	if (iRank > 254) iRank = -1;

	uint8 kk = bStage % 6;
	uint32 MaxStages = (bStage - kk) / 6 + (kk > 0 ? 1 : 0);
	pkt << uint8(iRank) << pUser->GetName() << pFinishTime << uint32(bStage) << uint32(MaxStages == 0 ? 1 : MaxStages) << uint32(pUser->m_bDrakiEnteranceLimit);
}
#pragma endregion

#pragma region CDBAgent::CharacterSelectPrisonCheck(std::string & strCharID, uint8 & ZoneID)
bool CDBAgent::CharacterSelectPrisonCheck(std::string& strCharID, uint8& ZoneID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(_T("SELECT Zone FROM USERDATA WHERE strUserID = ?")))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return false;

	dbCommand->FetchByte(1, ZoneID);
	return true;
}
#pragma endregion

#pragma region CDBAgent::SelectCharacterChecking(string & strAccountID, string & strCharID)
int8 CDBAgent::SelectCharacterChecking(string& strAccountID, string& strCharID)
{
	int8 bRet = 0;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(string_format(_T("{? = CALL SELECT_CHARACTER(?, ?)}"))))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}
#pragma endregion

ServerUniteSelectingCharNameErrorOpcode CDBAgent::UpdateSelectingCharacterName(std::string& strAccountID, std::string& strUserID, std::string& strNewUserID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return ServerUniteSelectingCharNameErrorOpcode::CannotCharacterID;

	int16 bRet = 0;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strNewUserID.c_str(), strNewUserID.length());


	if (!dbCommand->Execute(string_format(_T("{? = CALL CHARACTER_SELECT_NAME_CHANGE(?, ?, ?)}"))))
		ReportSQLError(m_GameDB->GetError());

	return ServerUniteSelectingCharNameErrorOpcode(bRet);
}

void CDBAgent::LoadPasswd(std::string AccountID, std::string& strPasswd)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return;

	char strPass[8] = { 0 };

	dbCommand->AddParameter(SQL_PARAM_INPUT, AccountID.c_str(), AccountID.length());

	if (!dbCommand->Execute(_T("{CALL LOAD_OLD_ITEM_PASSWD(?)}")))
		ReportSQLError(m_GameDB->GetError());

	dbCommand->FetchString(1, strPasswd);
}

bool CDBAgent::LoadGameMasterSetting(string strCharID, CUser* pUser)
{
	if (!pUser || strCharID.empty())
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{CALL LOAD_GAMEMASTER_SETTINGS(?)}"))))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return false;

	int field = 1; std::string charid;
	dbCommand->FetchString(field++, charid);
	dbCommand->FetchByte(field++, pUser->m_GameMastersMute);
	dbCommand->FetchByte(field++, pUser->m_GameMastersUnMute);
	dbCommand->FetchByte(field++, pUser->m_GameMastersUnBan);
	dbCommand->FetchByte(field++, pUser->m_GameMastersBanPermit);
	dbCommand->FetchByte(field++, pUser->m_GameMastersBanUnder);
	dbCommand->FetchByte(field++, pUser->m_GameMastersBanDays);
	dbCommand->FetchByte(field++, pUser->m_GameMastersBanCheating);
	dbCommand->FetchByte(field++, pUser->m_GameMastersBanScamming);
	dbCommand->FetchByte(field++, pUser->m_GameMastersAllowAttack);
	dbCommand->FetchByte(field++, pUser->m_GameMastersDisabledAttack);
	dbCommand->FetchByte(field++, pUser->m_GameMastersNpAdd);
	dbCommand->FetchByte(field++, pUser->m_GameMastersExpAdd);
	dbCommand->FetchByte(field++, pUser->m_GameMastersMoneyAdd);
	dbCommand->FetchByte(field++, pUser->m_GameMastersDropAdd);
	dbCommand->FetchByte(field++, pUser->m_GameMastersLoyaltyChange);
	dbCommand->FetchByte(field++, pUser->m_GameMastersExpChange);
	dbCommand->FetchByte(field++, pUser->m_GameMastersMoneyChange);
	dbCommand->FetchByte(field++, pUser->m_GameMastersGiveItem);
	dbCommand->FetchByte(field++, pUser->m_GameMastersGiveItemSelf);
	dbCommand->FetchByte(field++, pUser->m_GameMastersSummonUser);
	dbCommand->FetchByte(field++, pUser->m_GameMastersTpOnUser);
	dbCommand->FetchByte(field++, pUser->m_GameMastersZoneChange);
	dbCommand->FetchByte(field++, pUser->m_GameMastersLocationChange);
	dbCommand->FetchByte(field++, pUser->m_GameMastersMonsterSummon);
	dbCommand->FetchByte(field++, pUser->m_GameMastersNpcSummon);
	dbCommand->FetchByte(field++, pUser->m_GameMastersMonKilled);
	dbCommand->FetchByte(field++, pUser->m_GameMastersTeleportAllUser);
	dbCommand->FetchByte(field++, pUser->m_GameMastersClanSummon);
	dbCommand->FetchByte(field++, pUser->m_GameMastersResetPlayerRanking);
	dbCommand->FetchByte(field++, pUser->m_GameMastersChangeEventRoom);
	dbCommand->FetchByte(field++, pUser->m_GameMastersWarOpen);
	dbCommand->FetchByte(field++, pUser->m_GameMastersWarClose);
	dbCommand->FetchByte(field++, pUser->m_GameMastersCaptainElection);
	dbCommand->FetchByte(field++, pUser->m_GameMastersSendPrsion);
	dbCommand->FetchByte(field++, pUser->m_GameMastersKcChange);
	dbCommand->FetchByte(field++, pUser->m_GameMastersReloadTable);
	dbCommand->FetchByte(field++, pUser->m_GameMastersDropTest);
	return true;
}

bool CDBAgent::GameStartClearRemainUser()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
	{
		printf("Clear Remain Users Error\n");
		return false;
	}

	if (!dbCommand->Execute(_T("{CALL GAME_START_CLEAR_REMAIN_USERS}")))
	{
		ReportSQLError(m_GameDB->GetError());
		printf("Clear Remain Users Error\n");
		return false;
	}
	return true;
}

bool CDBAgent::CheckTrashItemListTime()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
	{
		printf("CheckTrashItemListTime Error\n");
		return false;
	}

	if (!dbCommand->Execute(_T("{CALL UPDATE_TRASH_ITEMLIST}")))
	{
		ReportSQLError(m_GameDB->GetError());
		printf("CheckTrashItemListTime\n");
		return false;
	}
	return true;
}
#pragma endregion


void CDBAgent::LoadPetData(uint64 nSerial)
{
	return;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("{CALL LOAD_PET_DATA(" I64FMTD ")}"), nSerial)))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData())
		return;

	char strItem[PET_INVENTORY_TOTAL * 23];
	memset(strItem, 0x00, sizeof(strItem));

	PET_DATA* p = new PET_DATA();
	dbCommand->FetchString(1, p->strPetName);
	dbCommand->FetchByte(2, p->bLevel);
	dbCommand->FetchUInt16(3, p->sHP);
	dbCommand->FetchUInt16(4, p->sMP);
	dbCommand->FetchUInt32(5, p->nIndex);
	dbCommand->FetchInt16(6, p->sSatisfaction);
	dbCommand->FetchUInt32(7, p->nExp);
	dbCommand->FetchUInt16(8, p->sPid);
	dbCommand->FetchUInt16(9, p->sSize);
	dbCommand->FetchBinary(10, strItem, sizeof(strItem));

	ByteBuffer PetItemBuffer;
	PetItemBuffer.append(strItem, sizeof(strItem));

	PET_INFO* pInfo = g_pMain->m_PetInfoSystemArray.GetData(p->bLevel);
	if (pInfo == nullptr)
	{
		PET_INFO* pInfo2 = g_pMain->m_PetInfoSystemArray.GetData(PET_START_LEVEL);
		if (pInfo2 == nullptr)
		{
			printf("LoadPetData Warning!!! \n");
			TRACE("LoadPetData Warning!!! \n");
			return;
		}
	}

	memset(p->sItem, 0x00, sizeof(p->sItem));

	p->info = pInfo;
	int index = 0;
	for (int i = 0; i < PET_INVENTORY_TOTAL; i++)
	{
		uint64 nSerialNum;
		uint32 nItemID, nExpirationtime;
		int16 sDurability, sCount, PetRentTime;
		uint8 bFlag;

		PetItemBuffer >> nItemID >> sDurability >> sCount >> bFlag >> PetRentTime >> nExpirationtime >> nSerialNum;

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || sCount <= 0)
			continue;

		if (!pTable.m_bCountable && sCount > 1)
			sCount = 1;
		else if (sCount > ITEMCOUNT_MAX)
			sCount = ITEMCOUNT_MAX;

		if (nSerialNum == 0)
			nSerialNum = g_pMain->GenerateItemSerial();

		_ITEM_DATA* pItem = &p->sItem[i];
		pItem->nNum = nItemID;
		pItem->sDuration = sDurability;
		pItem->sCount = sCount;
		pItem->bFlag = bFlag;
		pItem->sRemainingRentalTime = PetRentTime;
		pItem->nExpirationTime = nExpirationtime;
		pItem->nSerialNum = nSerialNum;
	}

	//g_pMain->m_PetSystemlock.lock();
	g_pMain->m_PetDataSystemArray.insert(std::make_pair(nSerial, p));
	//g_pMain->m_PetSystemlock.unlock();
}

uint32 CDBAgent::CreateNewPet(uint64 nSerialID, uint8 bLevel, std::string& strPetName)
{
	uint32 bRet = 3;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strPetName.c_str(), strPetName.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL CREATE_NEW_PET(" I64FMTD ",%d, ?)}"), nSerialID, bLevel)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

void CDBAgent::UpdatingUserPet(uint64 nSerial)
{
	return;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	g_pMain->m_PetSystemlock.lock();
	PetDataSystemArray::iterator itr = g_pMain->m_PetDataSystemArray.find(nSerial);
	if (itr == g_pMain->m_PetDataSystemArray.end())
	{
		g_pMain->m_PetSystemlock.unlock();
		return;
	}
	g_pMain->m_PetSystemlock.unlock();
	PET_DATA* pPet = itr->second;
	if (pPet == nullptr)
		return;

	ByteBuffer PetItemBuffer;
	for (int i = 0; i < PET_INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = &pPet->sItem[i];
		PetItemBuffer << pItem->nNum << pItem->sDuration << pItem->sCount << pItem->bFlag << pItem->sRemainingRentalTime << pItem->nExpirationTime << pItem->nSerialNum;
	}

	dbCommand->AddParameter(SQL_PARAM_INPUT, pPet->strPetName.c_str(), pPet->strPetName.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)PetItemBuffer.contents(), PetItemBuffer.size(), SQL_BINARY);
	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_USER_PET(" I64FMTD ", ?, %d, %d, %d, %d, %d, %d, %d, ?)}"), nSerial, pPet->bLevel, pPet->sHP, pPet->sMP, pPet->sSatisfaction, pPet->nExp, pPet->sPid, pPet->sSize)))
		ReportSQLError(m_GameDB->GetError());
}

#pragma region CDBAgent::LoadBotTable()
bool CDBAgent::LoadBotTable()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;
#if GAME_SOURCE_VERSION  == 2369
	if (!dbCommand->Execute(_T("{CALL LOAD_USER_BOTS2369}")))
#elif GAME_SOURCE_VERSION  == 1098
	if (!dbCommand->Execute(_T("{CALL LOAD_USER_BOTS1098}")))
#elif GAME_SOURCE_VERSION  == 1534
	if (!dbCommand->Execute(_T("{CALL LOAD_USER_BOTS1534}")))
#endif
	{
		printf(" Error LOAD_USER_BOTS. \n");
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
	{
		printf(" Error LOAD_USER_BOTS. \n");
		return false;
	}

	do
	{
		_BOT_DATA* pBot = new _BOT_DATA();
		int field = 1;

		char strItem[INVENTORY_TOTAL * 8];
		memset(strItem, 0x00, sizeof(strItem));

		dbCommand->FetchString(field++, pBot->m_strUserID);
		dbCommand->FetchByte(field++, pBot->m_bNation);
		dbCommand->FetchByte(field++, pBot->m_bRace);
		dbCommand->FetchUInt16(field++, pBot->m_sClass);
		dbCommand->FetchUInt32(field++, pBot->m_nHair);
		dbCommand->FetchByte(field++, pBot->m_bLevel);
		dbCommand->FetchByte(field++, pBot->m_bFace);
		dbCommand->FetchInt16(field++, pBot->m_bKnights);
		dbCommand->FetchByte(field++, pBot->m_bFame);
		dbCommand->FetchByte(field++, pBot->m_bZone);
		pBot->m_curx = (float)(dbCommand->FetchInt32(field++) / 100.0f);
		pBot->m_curz = (float)(dbCommand->FetchInt32(field++) / 100.0f);
		pBot->m_cury = (float)(dbCommand->FetchInt32(field++) / 100.0f);
		pBot->m_oldx = pBot->m_curx;
		pBot->m_oldy = pBot->m_cury;
		pBot->m_oldz = pBot->m_curz;

		memset(pBot->m_sItemArray, 0x00, sizeof(pBot->m_sItemArray));

		dbCommand->FetchBinary(field++, strItem, sizeof(strItem));
		ByteBuffer itemData;
		itemData.append(strItem, sizeof(strItem));

		for (uint8 i = 0; i < INVENTORY_TOTAL; i++)
		{
			uint32 nItemID;
			uint16 sDurability, sCount;
			itemData >> nItemID >> sDurability >> sCount;
			_ITEM_DATA* pItem = &pBot->m_sItemArray[i];
			pItem->nNum = nItemID;
			pItem->sDuration = sDurability;
			pItem->sCount = sCount;
		}

		dbCommand->FetchUInt16(field++, pBot->m_sSid);
		dbCommand->FetchUInt16(field++, pBot->m_sAchieveCoverTitle);
		dbCommand->FetchByte(field++, pBot->m_reblvl);
		dbCommand->FetchBinary(field++, (char*)pBot->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		dbCommand->FetchUInt32(field++, pBot->m_iGold);
		dbCommand->FetchInt16(field++, pBot->m_sPoints);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_STR]);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_STA]);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_DEX]);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_INT]);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_CHA]);
		dbCommand->FetchByte(field++, pBot->m_bKareli); //x6
		dbCommand->FetchByte(field++, pBot->m_bKaresiz); //x6
		/*dbCommand->FetchUInt32(field++, pBot->m_iLoyalty);
		dbCommand->FetchUInt32(field++, pBot->m_iLoyaltyMonthly);*///DeaFSezo buray� a�t�m 28.12.2024

		pBot->m_iLoyalty = myrand(5000, 10000);
		pBot->m_iLoyaltyMonthly = myrand(2000, 5000);

		if (pBot->m_bKareli == 0)
			pBot->m_bKareli = -1;

		if (pBot->m_bKaresiz == 0)
			pBot->m_bKaresiz = -1;

		pBot->m_sSid += MAX_USER;

		if (!g_pMain->m_ArtificialIntelligenceArray.PutData(pBot->m_sSid, pBot))
			delete pBot;

	} while (dbCommand->MoveNext());

	return true;
}
#pragma endregion


bool CDBAgent::InsertBotMerchant(_BOT_SAVE_DATA* pData)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	int16 sRet = 1;//nNum1, nPrice1, sCount1, sDuration1
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &sRet);
	if (!dbCommand->Execute(string_format(_T("{? = CALL INSERT_BOT_MERCHANT_DATA(%d, %d, %d, %d, %d, %d, %d, %d, %d, %d,%d, %d, %d, %d, %d,%d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d,%d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d, %d,%d, %d, %d, %d, %d,%d, %d, %d, %d, %d,%d, %d, %d, %d, %d, %d, %d)}"),
		pData->nNum[0], pData->nPrice[0], pData->sCount[0], pData->sDuration[0], pData->isKc[0],
		pData->nNum[1], pData->nPrice[1], pData->sCount[1], pData->sDuration[1], pData->isKc[1],
		pData->nNum[2], pData->nPrice[2], pData->sCount[2], pData->sDuration[2], pData->isKc[2],
		pData->nNum[3], pData->nPrice[3], pData->sCount[3], pData->sDuration[3], pData->isKc[3],
		pData->nNum[4], pData->nPrice[4], pData->sCount[4], pData->sDuration[4], pData->isKc[4],
		pData->nNum[5], pData->nPrice[5], pData->sCount[5], pData->sDuration[5], pData->isKc[5],
		pData->nNum[6], pData->nPrice[6], pData->sCount[6], pData->sDuration[6], pData->isKc[6],
		pData->nNum[7], pData->nPrice[7], pData->sCount[7], pData->sDuration[7], pData->isKc[7],
		pData->nNum[8], pData->nPrice[8], pData->sCount[8], pData->sDuration[8], pData->isKc[8],
		pData->nNum[9], pData->nPrice[9], pData->sCount[9], pData->sDuration[9], pData->isKc[9],
		pData->nNum[10], pData->nPrice[10], pData->sCount[10], pData->sDuration[10], pData->isKc[10],
		pData->nNum[11], pData->nPrice[11], pData->sCount[11], pData->sDuration[11], pData->isKc[11],
		(int)(pData->fX * 100), (int)(pData->fZ * 100), (int)(pData->fY * 100), pData->Minute, pData->byZone, pData->Direction, pData->sMerchanType)))
	{
		ReportSQLError(m_GameDB->GetError());
		return 0;
	}

	return true;
}

bool CDBAgent::LoadBotHandlerMerchantTable()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
	{
		printf("Load Open LOAD_BOT_HANDLER_MERCHANT Error: 1\n");
		return false;
	}

	if (!dbCommand->Execute(_T("{CALL LOAD_BOT_HANDLER_MERCHANT}")))
	{
		ReportSQLError(m_GameDB->GetError());
		printf("Load Open LOAD_BOT_HANDLER_MERCHANT Error: 2\n");
		return false;
	}

	if (!dbCommand->hasData())
	{
		printf("Load Open LOAD_BOT_HANDLER_MERCHANT Error: 3\n");
		return false;
	}
	do
	{
		_BOT_MERCHANT_ITEM* pBotMerchant = new _BOT_MERCHANT_ITEM();
		int field = 1;
		dbCommand->FetchUInt32(field++, pBotMerchant->sIndex);
		dbCommand->FetchByte(field++, pBotMerchant->BotMerchantType);
		dbCommand->FetchString(field++, pBotMerchant->BotItemNum);
		dbCommand->FetchString(field++, pBotMerchant->BotItemCount);
		dbCommand->FetchString(field++, pBotMerchant->BotItemPrice);
		dbCommand->FetchString(field++, pBotMerchant->BotMerchantMessage);

		memset(&pBotMerchant->m_MerchantItems, 0, sizeof(pBotMerchant->m_MerchantItems));

		std::list<std::string> sItemNum = StrSplit(pBotMerchant->BotItemNum, ",");
		std::list<std::string> sItemCount = StrSplit(pBotMerchant->BotItemCount, ",");
		std::list<std::string> sItemPrice = StrSplit(pBotMerchant->BotItemPrice, ",");

		uint8 nItemNum = (uint8)sItemNum.size();
		uint8 nItemCount = (uint8)sItemCount.size();
		uint8 nItemPrice = (uint8)sItemPrice.size();

		if (nItemNum > 0 && nItemCount > 0 && nItemPrice > 0)
		{
			uint32 bItemNum = 0;
			uint16 bItemCount = 0;
			uint32 bItemPrice = 0;
			for (int i = 0; i < nItemNum; i++)
			{
				bItemNum = atoi(sItemNum.front().c_str());
				bItemCount = atoi(sItemCount.front().c_str());
				bItemPrice = atoi(sItemPrice.front().c_str());
				if (bItemNum > 0 && bItemCount > 0 && bItemPrice > 0)
				{
					_ITEM_TABLE pTable = g_pMain->GetItemPtr(bItemNum);
					if (pTable.isnull()
						|| bItemNum >= ITEM_NO_TRADE_MIN && bItemNum <= ITEM_NO_TRADE_MAX // Cannot be traded, sold or stored.
						|| pTable.m_bRace == RACE_UNTRADEABLE // Cannot be traded or sold.
						|| (pTable.m_bKind == 255 && bItemCount != 1)) // Cannot be traded or sold.
						continue;

					pBotMerchant->m_MerchantItems[i].nNum = bItemNum;
					pBotMerchant->m_MerchantItems[i].nPrice = bItemPrice;
					pBotMerchant->m_MerchantItems[i].sCount = bItemCount; // Selling Count

					if (pTable.m_bKind == 255)
						pBotMerchant->m_MerchantItems[i].bCount = bItemCount; // Original Count ( INVENTORY )
					else
						pBotMerchant->m_MerchantItems[i].bCount = bItemCount; // Original Count ( INVENTORY )

					if (pTable.m_bCountable > 0)
						pBotMerchant->m_MerchantItems[i].bCount = bItemCount; // Original Count ( INVENTORY )
					else
						pBotMerchant->m_MerchantItems[i].bCount = 1; // Original Count ( INVENTORY )

					pBotMerchant->m_MerchantItems[i].sDuration = pTable.m_sDuration;
					pBotMerchant->m_MerchantItems[i].nSerialNum = g_pMain->GenerateItemSerial(); // NOTE: Stackable items will have an issue with this.
					pBotMerchant->m_MerchantItems[i].bOriginalSlot = i;
					pBotMerchant->m_MerchantItems[i].IsSoldOut = false;
				}
				sItemNum.pop_front();
				sItemCount.pop_front();
				sItemPrice.pop_front();
			}
		}

		if (!g_pMain->m_ArtificialMerchantArray.PutData(pBotMerchant->sIndex, pBotMerchant))
			delete pBotMerchant;

	} while (dbCommand->MoveNext());

	return true;
}

bool CDBAgent::UpdateBotUser(CBot* pUser)
{
	if (pUser == nullptr)
		return false;

	if (pUser->GetName().empty())
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	uint32 m_iCashPoint = pUser->m_iLoyalty;
	uint32 m_iCashBonusPoint = pUser->m_iLoyaltyMonthly;

	if (m_iCashPoint <= 0)
	{
		m_iCashPoint = myrand(1, RIVALRY_NP_BONUS);;
		pUser->m_iLoyalty = m_iCashPoint;
	}

	if (m_iCashBonusPoint <= 0)
	{
		m_iCashBonusPoint = myrand(1, RIVALRY_NP_BONUS);;
		pUser->m_iLoyaltyMonthly = m_iCashBonusPoint;
	}

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length()); 
	if (!dbCommand->Execute(string_format(_T("UPDATE BOT_HANDLER_FARM SET Loyalty = %d, LoyaltyMonthly = %d WHERE strUserID = ?"), m_iCashPoint, m_iCashBonusPoint)))
		ReportSQLError(m_AccountDB->GetError());

	return true;
}

bool CDBAgent::GetLoadBotUser(CBot* pUser)
{
	if (pUser == nullptr)
		return false;

	if (pUser->GetName().empty())
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(_T("SELECT Loyalty, LoyaltyMonthly FROM BOT_HANDLER_FARM WHERE strUserID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	dbCommand->FetchUInt32(1, pUser->m_iLoyalty); dbCommand->FetchUInt32(2, pUser->m_iLoyaltyMonthly);
	return true;
}

uint8 CDBAgent::LoadPromotionKey(std::string& PPCardKeys, CUser* pUser)
{
	uint8 bRet = 0; // failed key

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, PPCardKeys.c_str(), PPCardKeys.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL LOAD_PROMOTION_SYSTEM(?, ?, ?)}"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return 3;
	}


	return bRet;

}

#pragma region CDBAgent::LoadPPCard(std::string PPCardKeys, std::string strCharID)
uint8 CDBAgent::LoadPPCard(std::string PPCardKeys, CUser* pUser)
{
	if (PPCardKeys.empty() || !pUser || pUser->GetName().empty()) return 0;

	uint32 Cash = 0, tlcount = 0; uint8 cashtype = 0;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	dbCommand->AddParameter(SQL_PARAM_INPUT, PPCardKeys.c_str(), PPCardKeys.length());
	if (!dbCommand->Execute(_T("SELECT KnightCash,TLBalance, CashType FROM PPCARD_LIST WHERE PPKeyCode = ? AND [STATUS] = 0")))
	{
		ReportSQLError(m_GameDB->GetError());
		return 0;
	}

	if (!dbCommand->hasData())
		return 0;

	dbCommand->FetchUInt32(1, Cash);
	dbCommand->FetchUInt32(2, tlcount);
	dbCommand->FetchByte(3, cashtype);

	if ((cashtype != 3 && Cash == 0 && tlcount == 0) || cashtype < 1 || cashtype > 3)
		return 0;

	if (cashtype == 3)
	{
		uint8 pRet = LoadPromotionKey(PPCardKeys, pUser);
		if (pRet != 4)
			return 0;

		if (Cash || tlcount)pUser->GiveBalance(Cash, tlcount);
		return 1;
	}

	dbCommand->AddParameter(SQL_PARAM_INPUT, PPCardKeys.c_str(), PPCardKeys.length());
	if (!dbCommand->Execute(string_format(_T("UPDATE PPCARD_LIST SET [STATUS] = 1 , strAccountID = '%s' , strUserID = '%s' , UpdateDate = convert(varchar, getdate(), 120) FROM PPCARD_LIST WHERE PPKeyCode = ? AND [STATUS] = 0"),
		pUser->GetAccountName().c_str(), pUser->GetName().c_str())))
	{
		ReportSQLError(m_GameDB->GetError());
		return 0;
	}

	pUser->GiveBalance(Cash, tlcount);
	return 1;
}

#pragma endregion

bool CDBAgent::LoadKnightCash(std::string& strAccountID, CUser* pUser)
{
	if (strAccountID.empty() || pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());

	if (!dbCommand->Execute(_T("SELECT CashPoint,BonusCashPoint FROM TB_USER WHERE strAccountID = ?")))
		ReportSQLError(m_AccountDB->GetError());

	if (!dbCommand->hasData())
		return false;

	dbCommand->FetchUInt32(1, pUser->m_nKnightCash);
	dbCommand->FetchUInt32(2, pUser->m_nTLBalance);
	return true;
}

bool CDBAgent::UpdateKnightCash(CUser* pUser)
{
	if (pUser == nullptr)
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	uint32 KnightCash = pUser->m_nKnightCash;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());

	if (!dbCommand->Execute(string_format(_T("UPDATE TB_USER SET CashPoint = %d,BonusCashPoint=%d WHERE strAccountID = ?"), KnightCash, pUser->m_nTLBalance)))
		ReportSQLError(m_AccountDB->GetError());

	return true;
}

bool CDBAgent::UpdateAccountKnightCash(CUser* pUser)
{
	if (!pUser || pUser->GetAccountName().empty()) return false;
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());
	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_BALANCE (?, %d, %d)}"), pUser->m_nKnightCash, pUser->m_nTLBalance)))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}
	return true;
}


int8 CDBAgent::SetItemFromLetter(string& strCharID, uint32 nLetterID)
{
	int8 bRet = -1; // error
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL MAIL_BOX_SET_ITEM(?, %d)}"), nLetterID)))
	{
		ReportSQLError(m_GameDB->GetError());
		return -1;
	}
	return bRet;
}

bool CDBAgent::UpdateReportUser(std::string strCharID, std::string ReportType, std::string SubJect, std::string Message)
{
	if (strCharID.empty() || ReportType.empty() || SubJect.empty() || Message.empty()) return false;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, ReportType.c_str(), ReportType.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, SubJect.c_str(), SubJect.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, Message.c_str(), Message.length());
	if (!dbCommand->Execute(string_format(_T("INSERT INTO BUG_REPORT VALUES(?,?,?,?)"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}
	return true;
}

uint8 CDBAgent::AccountInfoShow(std::string& AccountID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	dbCommand->AddParameter(SQL_PARAM_INPUT, AccountID.c_str(), AccountID.length());
	if (!dbCommand->Execute(_T("SELECT AccountCheck FROM TB_USER WHERE strAccountID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return 0;
	}

	if (!dbCommand->hasData())
		return 0;

	uint8 Active = 0;
	dbCommand->FetchByte(1, Active);
	return Active;
}

uint8 CDBAgent::AccountInfoSave(std::string& AccountID, std::string UserID, std::string& email, std::string& phone, std::string& otp, std::string& seal)
{
	uint8 bRet = 0;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, AccountID.c_str(), AccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, UserID.c_str(), UserID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, email.c_str(), email.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, phone.c_str(), phone.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, otp.c_str(), otp.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, seal.c_str(), seal.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL ACCOUNT_INFO_SAVE(?,?,?,?,?,?)}"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return bRet;
	}

	return bRet;
}

bool CDBAgent::LoadAutoClanInfo(std::string& strCharID, uint16& Class, uint8& Level, uint16& ClanID, uint8& Fame, uint32& Loyalty, uint32& MontlyLoyalty)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(_T("SELECT Class, Level, Knights, Fame, Loyalty, LoyaltyMonthly FROM USERDATA WHERE strUserID = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	int field = 1;
	dbCommand->FetchUInt16(field++, Class);
	dbCommand->FetchByte(field++, Level);
	dbCommand->FetchUInt16(field++, ClanID);
	dbCommand->FetchByte(field++, Fame);
	dbCommand->FetchUInt32(field++, Loyalty);
	dbCommand->FetchUInt32(field++, MontlyLoyalty);
	return true;
}

bool CDBAgent::LoadDrakiTowerDailyRank(std::vector<_DRAKI_TOWER_FORDAILY_RANKING>& List)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (!dbCommand->Execute(_T("{CALL LOAD_DRAKI_TOWER_DAILY_RANK}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	do
	{
		_DRAKI_TOWER_FORDAILY_RANKING pList;
		dbCommand->FetchString(1, pList.strUserID);
		dbCommand->FetchUInt32(2, pList.Class);
		dbCommand->FetchUInt32(3, pList.DrakiTime);
		dbCommand->FetchByte(4, pList.DrakiStage);
		List.push_back(pList);
	} while (dbCommand->MoveNext());

	return true;
}


bool CDBAgent::LoadUserDailyRank(std::string& strCharID, CUser* pUser)
{
	if (pUser == nullptr || strCharID != pUser->GetName())
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{CALL LOAD_DAILY_RANK_USER(?)}"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	int field = 1;
	dbCommand->FetchUInt64(field++, pUser->pUserDailyRank.GMTotalSold);
	dbCommand->FetchUInt64(field++, pUser->pUserDailyRank.MHTotalKill);
	dbCommand->FetchUInt64(field++, pUser->pUserDailyRank.SHTotalExchange);
	dbCommand->FetchUInt64(field++, pUser->pUserDailyRank.CWCounterWin);
	dbCommand->FetchUInt64(field++, pUser->pUserDailyRank.UPCounterBles);

	return true;
}

bool CDBAgent::LoadDailyRank()
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;
	if (!dbCommand->Execute(_T("{CALL LOAD_DAILY_RANK}"))) { ReportSQLError(m_GameDB->GetError()); return false; }

	do
	{
		int field = 1, diff = 0;
		_DAILY_RANK* pData = new _DAILY_RANK();
		dbCommand->FetchString(field++, pData->strUserID);
		for (int i = 0; i < 3; i++) dbCommand->FetchUInt32(field++, pData->GmRank[i]);
		dbCommand->FetchUInt64(field++, pData->GMTotalSold);
		for (int i = 0; i < 3; i++) dbCommand->FetchUInt32(field++, pData->MhRank[i]);
		dbCommand->FetchUInt64(field++, pData->MHTotalKill);
		for (int i = 0; i < 3; i++) dbCommand->FetchUInt32(field++, pData->ShRank[i]);
		dbCommand->FetchUInt64(field++, pData->SHTotalExchange);
		for (int i = 0; i < 3; i++) dbCommand->FetchUInt32(field++, pData->AkRank[i]);
		dbCommand->FetchUInt64(field++, pData->AKLoyaltyMonthly);
		for (int i = 0; i < 3; i++) dbCommand->FetchUInt32(field++, pData->CwRank[i]);
		dbCommand->FetchUInt64(field++, pData->CWCounterWin);
		for (int i = 0; i < 3; i++) dbCommand->FetchUInt32(field++, pData->UpRank[i]);
		dbCommand->FetchUInt64(field++, pData->UPCounterBles);
		if (!g_pMain->m_DailyRank.PutData(pData->strUserID, pData)) { delete pData; return false; }
	} while (dbCommand->MoveNext());

	return true;
}

bool CDBAgent::SaveDailyRank(std::string& strCharID, CUser* pUser)
{
	if (pUser == nullptr || strCharID != pUser->GetName()) return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_USER_DAILY_RANK(?, %d, %d, %d, %d, %d)}"),
		pUser->pUserDailyRank.GMTotalSold, pUser->pUserDailyRank.MHTotalKill, pUser->pUserDailyRank.SHTotalExchange,
		pUser->pUserDailyRank.CWCounterWin, pUser->pUserDailyRank.UPCounterBles)))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}
	return true;
}

bool CDBAgent::LoadPusRefund(CUser* pthis)
{
	if (!pthis || pthis->GetAccountName().empty()) return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;
	dbCommand->AddParameter(SQL_PARAM_INPUT, pthis->GetAccountName().c_str(), pthis->GetAccountName().length());
	if (!dbCommand->Execute(_T("{CALL LOAD_USER_PUSREFUND_LIST(?)}"))) { ReportSQLError(m_GameDB->GetError()); return false; }
	if (!dbCommand->hasData()) return true;

	do
	{
		int field = 1; uint64 serial = 0;
		_PUS_REFUND* pref = new _PUS_REFUND();
		dbCommand->FetchUInt64(field++, serial);
		dbCommand->FetchString(field++, pref->accountid);
		dbCommand->FetchUInt32(field++, pref->itemid);
		dbCommand->FetchUInt16(field++, pref->itemcount);
		dbCommand->FetchUInt32(field++, pref->itemprice);
		dbCommand->FetchUInt32(field++, pref->expiredtime);
		dbCommand->FetchUInt16(field++, pref->itemduration);
		dbCommand->FetchByte(field++, pref->buytype);
		if (UNIXTIME > pref->expiredtime) { delete pref; continue; }
		if (!pthis->m_PusRefundMap.PutData(serial, pref)) { delete pref; return false; }
	} while (dbCommand->MoveNext());
	return true;
}

bool CDBAgent::PusRefundItemDelete(CUser* pthis, uint64 serial, uint32 itemid)
{
	if (!pthis) return false;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;
	dbCommand->AddParameter(SQL_PARAM_INPUT, pthis->GetAccountName().c_str(), pthis->GetAccountName().length());
	if (!dbCommand->Execute(string_format(_T("{CALL PUSREFUND_ITEM_DELETE(?, %I64d, %d)}"), serial, itemid))) { ReportSQLError(m_GameDB->GetError()); return false; }
	return true;
}

bool CDBAgent::PusRefundItemAdd(CUser* pthis, uint64 serial, uint32 itemid, uint16 itemcount, uint16 itemduration, uint32 kcprice, uint32 buyingtime, uint8 buytype)
{
	if (!pthis || pthis->GetAccountName().empty()) return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;
	dbCommand->AddParameter(SQL_PARAM_INPUT, pthis->GetAccountName().c_str(), pthis->GetAccountName().length());
	if (!dbCommand->Execute(string_format(_T("{CALL PUSREFUND_ITEM_ADD(?, %I64d,%d,%d,%d,%d,%d,%d)}"), serial, itemid, itemcount, itemduration, kcprice, buyingtime, buytype))) { ReportSQLError(m_GameDB->GetError()); return false; }
	return true;
}

bool CDBAgent::PusGiftCheck(CUser* pUser, std::string targetname)
{
	bool bRet = false;
	if (!pUser || targetname.empty()) return bRet;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return false;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, targetname.c_str(), targetname.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL PUSGIFT_CHECK(?)}")))) { ReportSQLError(m_GameDB->GetError()); return false; }
	return true;
}

bool CDBAgent::SaveClanNationTransferUser(std::string strUserID, std::string strAccountID, uint16 ClanID, Nation newnation, uint8 newrace, uint8 newclass)
{
	int bRet = 0;
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL NATION_CLANTRANSFER_SAVEUSER (?, ?, %d, %d, %d, %d)}"), ClanID, (uint8)newnation, newrace, newclass)))
	{ 
		ReportSQLError(m_GameDB->GetError()); 
		return bRet;
	}

	return true;
}

bool CDBAgent::SaveCNTSKnights(uint16 ClanID, Nation newnation)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (!dbCommand->Execute(string_format(_T("UPDATE KNIGHTS SET Nation = %d WHERE IDNum= %d"), (uint8)newnation, ClanID)))
	{ 
		ReportSQLError(m_GameDB->GetError());
		return false;
	}
	return true;
}

uint8 CDBAgent::UpdateNtsCommand(std::string strAccountID, std::string strCharID, uint8 bnewRace, uint16 bNewClass, uint8 bNewNation)
{
	int8 bRet = 0;
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());


	if (!dbCommand->Execute(string_format(_T("{? = CALL UPDATE_NTSCOMMAND(?, ?, %d, %d, %d)}"), bnewRace, bNewClass, bNewNation)))
		ReportSQLError(m_GameDB->GetError());

	return bRet;
}

bool CDBAgent::LoadTBUserData(std::string strAccountID, CUser* pUser)
{
	if (pUser == nullptr)  return false;

	std::unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)  return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(_T("{CALL LOAD_TBUSER(?)}")))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())  return false;

	int field = 1; uint8 lregister = 0;
	dbCommand->FetchString(field++, pUser->pUserTbInfo.sealpass);
	dbCommand->FetchString(field++, pUser->pUserTbInfo.accountpass);
	return true;
}

uint8 CDBAgent::UpdateUserSealItem(uint64 nItemSerial, uint32 nItemID, uint8 bSealType, uint8 prelockstate, CUser* pUser)
{
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) return 3;

	uint8 bRet = 1;
	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL UPDATE_USER_ITEM_SEAL(?, ?, " I64FMTD ", %d, %d, %d)}"), nItemSerial, nItemID, bSealType, prelockstate)))
	{
		ReportSQLError(m_GameDB->GetError());
		return 3;
	}
	return bRet;
}

bool CDBAgent::LoadPerksData(std::string strUserID, CUser* pUser)
{
	if (pUser == nullptr || strUserID.empty())
		return false;

	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strUserID.c_str(), strUserID.length());
	if (!dbCommand->Execute(_T("{CALL LOAD_PERKS_DATA(?)}")))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
		return false;

	int field = 1;
	std::string name;
	dbCommand->FetchString(field++, name);
	for (int i = 0; i < PERK_COUNT; i++)
		dbCommand->FetchUInt16(field++, pUser->pPerks.perkType[i]);
	dbCommand->FetchUInt16(field++, pUser->pPerks.remPerk);
	return true;
}

bool CDBAgent::UpdateUserPerks(CUser* pUser)
{
	if (!pUser || pUser->GetName().empty())
		return false;
	std::unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetName().c_str(), pUser->GetName().length());
	if (!dbCommand->Execute(string_format(_T("{CALL UPDATE_USER_PERKS(?,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d)}"), pUser->pPerks.remPerk,
		pUser->pPerks.perkType[0], pUser->pPerks.perkType[1], pUser->pPerks.perkType[2], pUser->pPerks.perkType[3],
		pUser->pPerks.perkType[4], pUser->pPerks.perkType[5], pUser->pPerks.perkType[6], pUser->pPerks.perkType[7],
		pUser->pPerks.perkType[8], pUser->pPerks.perkType[9], pUser->pPerks.perkType[10], pUser->pPerks.perkType[11],
		pUser->pPerks.perkType[12])))
	{
		ReportSQLError(m_GameDB->GetError());
		return false;
	}
	return true;
}
void CDBAgent::KingAddAndDelete(std::string nstrUserID, uint8 Type, uint32 CashPoint) //Write Gkhn
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	if (CashPoint < 0)
		CashPoint = 0;

	if (Type < 0 && Type > 1)
		return;

	dbCommand->AddParameter(SQL_PARAM_INPUT, nstrUserID.c_str(), nstrUserID.length());

	if (Type == 0)
	{
		if (!dbCommand->Execute(string_format(_T("EXEC KRAL_SIL ?"))))
			ReportSQLError(m_GameDB->GetError());
	}
	else if (Type == 1)
	{/*
		if (!dbCommand->Execute(string_format(_T("EXEC KRAL_EKLE ?,%d"), CashPoint)))
			ReportSQLError(m_GameDB->GetError());*/
		if (!dbCommand->Execute(string_format(_T("EXEC KRAL_EKLE ?"))))
			ReportSQLError(m_GameDB->GetError());
	}
}

void CDBAgent::LoadSlave(std::string& strAccountID)
{
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;


	uint8 Check = CheckSlave(strAccountID);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (Check == 0) {
		if (!dbCommand->Execute(string_format(_T("INSERT INTO SLAVE VALUES(?, 0, 0) "))))
		{
			ReportSQLError(m_GameDB->GetError());
		}
	}


	if (!dbCommand->Execute(_T("SELECT KC, Gold FROM SLAVE WHERE strUser = ?")))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	uint32 kc, gold;

	if (dbCommand->hasData())
	{
		dbCommand->FetchUInt32(1, kc);
		dbCommand->FetchUInt32(2, gold);

	}
	else
		return;

	CUser* pUser = g_pMain->GetUserPtr(strAccountID, NameType::TYPE_CHARACTER);




	pUser->SlaveTotalKC = kc;
	pUser->SlaveTotalCoins = gold;

	return;
}



void CDBAgent::UpdateSlaveCoins(std::string& strAccountID, uint32 Coins)
{

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	uint8 Check = CheckSlave(strAccountID);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (Check == 0) {
		if (!dbCommand->Execute(string_format(_T("INSERT INTO SLAVE VALUES(?, 0, 0) "))))
		{
			ReportSQLError(m_GameDB->GetError());
		}
	}

	if (!dbCommand->Execute(string_format(_T("UPDATE SLAVE SET Gold += %d WHERE strUser = ?"), Coins)))
		ReportSQLError(m_AccountDB->GetError());
}

void CDBAgent::UpdateSlaveKC(std::string& strAccountID, uint32 KC)
{

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	uint8 Check = CheckSlave(strAccountID);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (Check == 0) {
		if (!dbCommand->Execute(string_format(_T("INSERT INTO SLAVE VALUES(?, 0, 0) "))))
		{
			ReportSQLError(m_GameDB->GetError());
		}
	}


	if (!dbCommand->Execute(string_format(_T("UPDATE SLAVE SET KC += %d WHERE strUser = ?"), KC)))
		ReportSQLError(m_AccountDB->GetError());
}

void CDBAgent::RemoveSlaveCoins(std::string& strAccountID, uint32 Coins)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	uint8 Check = CheckSlave(strAccountID);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (Check == 0) {
		if (!dbCommand->Execute(string_format(_T("INSERT INTO SLAVE VALUES(?, 0, 0) "))))
		{
			ReportSQLError(m_GameDB->GetError());
		}
	}
	if (!dbCommand->Execute(string_format(_T("UPDATE SLAVE SET Gold = %d WHERE strUser = ?"), Coins)))
		ReportSQLError(m_AccountDB->GetError());
}

void CDBAgent::RemoveSlaveKC(std::string& strAccountID, uint32 KC)
{

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return;

	uint8 Check = CheckSlave(strAccountID);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (Check == 0) {
		if (!dbCommand->Execute(string_format(_T("INSERT INTO SLAVE VALUES(?, 0, 0) "))))
		{
			ReportSQLError(m_GameDB->GetError());
		}
	}
	if (!dbCommand->Execute(string_format(_T("UPDATE SLAVE SET KC = %d WHERE strUser = ?"), KC)))
		ReportSQLError(m_AccountDB->GetError());
}


uint8 CDBAgent::CheckSlave(std::string& strAccountID)
{
	int8 bRet = -1;
	bool isOkey = false;
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return bRet;

	dbCommand->AddParameter(SQL_PARAM_OUTPUT, &bRet);
	dbCommand->AddParameter(SQL_PARAM_INPUT, strAccountID.c_str(), strAccountID.length());
	if (!dbCommand->Execute(string_format(_T("{? = CALL HAS_SLAVE_CHECK(?)}"))))
		ReportSQLError(m_GameDB->GetError());

	return bRet;


}

uint8 CDBAgent::LoadAkaraList(std::vector<AKARA_LIST>& AkaraList)
{

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;


	if (!dbCommand->Execute(_T("SELECT * FROM AKARA_LIST")))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}


	std::vector<AKARA_LIST> List;

	int IsOpen = 0;
	uint8 sCount = 0;

	if (dbCommand->hasData())
	{

		do
		{
			uint8 nothing;
			uint32 ItemNum[3];
			uint32 RealItemNum[3];
			uint32 Bet[3];
			uint32 BetMax[3];
			memset(ItemNum, 0, sizeof(ItemNum));
			memset(Bet, 0, sizeof(Bet));
			memset(BetMax, 0, sizeof(BetMax));
			memset(RealItemNum, 0, sizeof(RealItemNum));
			std::string Owner[3];

			int i = 1;

			dbCommand->FetchByte(1, nothing);
			dbCommand->FetchUInt32(2, ItemNum[0]);
			dbCommand->FetchUInt32(3, RealItemNum[0]);
			dbCommand->FetchUInt32(4, Bet[0]);
			dbCommand->FetchUInt32(5, BetMax[0]);
			dbCommand->FetchString(6, Owner[0]);
			dbCommand->FetchUInt32(7, ItemNum[1]);
			dbCommand->FetchUInt32(8, RealItemNum[1]);
			dbCommand->FetchUInt32(9, Bet[1]);
			dbCommand->FetchUInt32(10, BetMax[1]);
			dbCommand->FetchString(11, Owner[1]);
			dbCommand->FetchUInt32(12, ItemNum[2]);
			dbCommand->FetchUInt32(13, RealItemNum[2]);
			dbCommand->FetchUInt32(14, Bet[2]);
			dbCommand->FetchUInt32(15, BetMax[2]);
			dbCommand->FetchString(16, Owner[2]);

			List.push_back(AKARA_LIST(ItemNum, RealItemNum, Bet, BetMax, Owner));

		} while (dbCommand->MoveNext());
	}

	AkaraList = List;


	return 0;
}
#pragma endregion
uint8 CDBAgent::ClearAkaraList(uint8 sira)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;


	if (sira == 0)
	{
		if (!dbCommand->Execute(_T("UPDATE AKARA_LIST	SET Maxbet = 0 , Owner = ''")))
		{
			ReportSQLError(m_AccountDB->GetError());
			return false;
		}
	}
	else
	{
		if (!dbCommand->Execute(string_format(_T("UPDATE AKARA_LIST	SET Maxbet%d = 0, Owner%d = ''"), sira, sira)))
		{
			ReportSQLError(m_AccountDB->GetError());
			return false;
		}
	}



	return 0;

}

uint8 CDBAgent::UpdateAkaraList(uint16 Uid, uint32 ItemID, uint32 Price, uint8 sira, uint8 listeNum)
{
	CUser* pUser = g_pMain->GetUserPtr(Uid);

	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	if (pUser != nullptr)
	{
		if (sira == 0)
		{
			if (!dbCommand->Execute(string_format(_T("UPDATE AKARA_LIST	SET Maxbet = %d , Owner = '%s' where ID = %d"), Price, pUser->GetName().c_str(), listeNum)))
			{
				ReportSQLError(m_AccountDB->GetError());
				return false;
			}
		}
		else
		{
			if (!dbCommand->Execute(string_format(_T("UPDATE AKARA_LIST	SET Maxbet%d = %d , Owner%d = '%s' where ID = %d"), sira, Price, sira, pUser->GetName().c_str(), listeNum)))
			{
				ReportSQLError(m_AccountDB->GetError());
				return false;
			}
		}


	}
	return 0;
}
uint8 CDBAgent::UpdateUserAkaraHistory(uint8 listeNum, uint8 sira, std::vector<AKARA_LIST>& List, uint8 sonuc)
{
	if (listeNum - 1 >= List.size() || sira >= 3)
		return 0;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	ByteBuffer AkaraList;

	AkaraList << listeNum << sira << uint8(sonuc) << List.at(listeNum - 1).BetMax[sira] << List.at(listeNum - 1).ItemNum[sira];

	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)AkaraList.contents(), AkaraList.size(), SQL_BINARY);


	if (!dbCommand->Execute(string_format(_T("INSERT INTO AKARA_USER_HISTORY (Data,UserName) VALUES(?,'%s')"), List.at(listeNum - 1).Owner[sira].c_str())))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	return 0;
}

uint8 CDBAgent::LoadUserAkaraHistory(std::vector<ByteBuffer>& List, std::string& UserName)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;


	if (!dbCommand->Execute(string_format(_T("SELECT * FROM AKARA_USER_HISTORY WHERE UserName = '%s'"), UserName.c_str())))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	int IsOpen = 0;
	uint8 sCount = 0;

	if (dbCommand->hasData())
	{

		do
		{
			std::string Name;
			char strHistory[15] = { 0 };

			dbCommand->FetchString(1, Name);
			dbCommand->FetchBinary(2, strHistory, sizeof(strHistory));

			ByteBuffer Buffer;
			Buffer.append(strHistory, sizeof(strHistory));
			List.push_back(Buffer);

		} while (dbCommand->MoveNext());
	}

	return 0;
}

uint8 CDBAgent::UpdateAkaraHistory(uint8 listeNum, uint8 sira, std::vector<AKARA_LIST>& List, uint8 sonuc)
{
	if (listeNum >= List.size() || sira >= 3)
		return 0;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return 0;

	ByteBuffer AkaraList;

	// listeNum is 1-based in caller side; vector is 0-based.
	AkaraList << listeNum << sira << uint8(sonuc) << List.at(listeNum - 1).ItemNum[sira] << List.at(listeNum - 1).BetMax[sira];

	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)AkaraList.contents(), AkaraList.size(), SQL_BINARY);


	if (!dbCommand->Execute(string_format(_T("INSERT INTO AKARA_HISTORY (Data) VALUES(?)"))))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	return 0;
}

uint8 CDBAgent::LoadAkaraHistory(std::vector<ByteBuffer>& List)
{
	unique_ptr<OdbcCommand> dbCommand(m_AccountDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;


	if (!dbCommand->Execute(_T("SELECT * FROM AKARA_HISTORY")))
	{
		ReportSQLError(m_AccountDB->GetError());
		return false;
	}

	int IsOpen = 0;
	uint8 sCount = 0;

	if (dbCommand->hasData())
	{

		do
		{
			char strHistory[15] = { 0 };

			dbCommand->FetchBinary(1, strHistory, sizeof(strHistory));

			ByteBuffer Buffer;
			Buffer.append(strHistory, sizeof(strHistory));
			List.push_back(Buffer);

		} while (dbCommand->MoveNext());
	}

	return 0;
}

// JstKO Welcome Logos Notice Eklendi 01.01.2025
#pragma region CDBAgent::LoadJstKOWelcomeLogosNotice
void CDBAgent::LoadJstKOWelcomeLogosNotice()
{

	uint8 Status = 0;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return;

	if (!dbCommand->Execute(string_format(_T("select *from WELCOME_LOGIN_NOTICE"))))
	{
		ReportSQLError(m_GameDB->GetError());
		return;
	}

	if (!dbCommand->hasData())
		return;


	if (dbCommand->hasData())
	{
		dbCommand->FetchString(1, g_pMain->WelcomeNoticeYeni);
		dbCommand->FetchByte(2, g_pMain->WelcomeNoticeR);
		dbCommand->FetchByte(3, g_pMain->WelcomeNoticeG);
		dbCommand->FetchByte(4, g_pMain->WelcomeNoticeB);
		dbCommand->FetchByte(5, Status);

		if (Status == 1)
			g_pMain->JstKOWelcomeStatus = true;
		else
			g_pMain->JstKOWelcomeStatus = false;

	}

	return;

}
#pragma endregion
// JstKO Welcome Logos Notice Eklendi The End 01.01.2025

#pragma region CDBAgent::LoadMorankerBotTable()
bool CDBAgent::LoadMorankerBotTable(std::vector<_BOT_DATA*> & botList)
{
	botList.clear();
	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());

	if (dbCommand.get() == nullptr)
		return false;

#if GAME_SOURCE_VERSION == 2369
	if (!dbCommand->Execute(_T("{CALL LOAD_MORANKER_BOTS2369}")))
#else
	if (!dbCommand->Execute(_T("{CALL LOAD_MORANKER_BOTS2369}")))
#endif
	{
		printf("Moranker: LOAD_MORANKER_BOTS2369 SQL error.\n");
		ReportSQLError(m_GameDB->GetError());
		return false;
	}

	if (!dbCommand->hasData())
	{
		printf("Moranker: LOAD_MORANKER_BOTS2369 returned no rows.\n");
		return false;
	}

	do
	{
		_BOT_DATA* pBot = new _BOT_DATA();
		int field = 1;

		char strItem[INVENTORY_TOTAL * 8];
		memset(strItem, 0x00, sizeof(strItem));
		memset(pBot->m_sItemArray, 0x00, sizeof(pBot->m_sItemArray));
		memset(pBot->m_bstrSkill, 0x00, sizeof(pBot->m_bstrSkill));
		memset(pBot->m_bStats, 0x00, sizeof(pBot->m_bStats));

		dbCommand->FetchString(field++, pBot->m_strUserID);
		dbCommand->FetchByte(field++, pBot->m_bNation);
		dbCommand->FetchByte(field++, pBot->m_bRace);
		dbCommand->FetchUInt16(field++, pBot->m_sClass);
		dbCommand->FetchUInt32(field++, pBot->m_nHair);
		dbCommand->FetchByte(field++, pBot->m_bLevel);
		dbCommand->FetchByte(field++, pBot->m_bFace);
		dbCommand->FetchInt16(field++, pBot->m_bKnights);
		dbCommand->FetchByte(field++, pBot->m_bFame);
		dbCommand->FetchByte(field++, pBot->m_bZone);
		pBot->m_curx = (float)(dbCommand->FetchInt32(field++) / 100.0f);
		pBot->m_curz = (float)(dbCommand->FetchInt32(field++) / 100.0f);
		pBot->m_cury = (float)(dbCommand->FetchInt32(field++) / 100.0f);
		pBot->m_oldx = pBot->m_curx;
		pBot->m_oldy = pBot->m_cury;
		pBot->m_oldz = pBot->m_curz;

		dbCommand->FetchBinary(field++, strItem, sizeof(strItem));
		ByteBuffer itemData;
		itemData.append(strItem, sizeof(strItem));

		for (uint8 i = 0; i < INVENTORY_TOTAL; i++)
		{
			uint32 nItemID = 0;
			uint16 sDurability = 0, sCount = 0;
			itemData >> nItemID >> sDurability >> sCount;
			_ITEM_DATA* pItem = &pBot->m_sItemArray[i];
			pItem->nNum = nItemID;
			pItem->sDuration = sDurability;
			pItem->sCount = sCount;
		}

		dbCommand->FetchUInt16(field++, pBot->m_sSid);
		dbCommand->FetchUInt16(field++, pBot->m_sAchieveCoverTitle);
		dbCommand->FetchByte(field++, pBot->m_reblvl);
		dbCommand->FetchBinary(field++, (char*)pBot->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		dbCommand->FetchUInt32(field++, pBot->m_iGold);
		dbCommand->FetchInt16(field++, pBot->m_sPoints);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_STR]);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_STA]);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_DEX]);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_INT]);
		dbCommand->FetchByte(field++, pBot->m_bStats[(uint8)StatType::STAT_CHA]);
		dbCommand->FetchByte(field++, pBot->m_bKareli);
		dbCommand->FetchByte(field++, pBot->m_bKaresiz);
		dbCommand->FetchUInt32(field++, pBot->m_iLoyalty);
		dbCommand->FetchUInt32(field++, pBot->m_iLoyaltyMonthly);

		if (pBot->m_bKareli == 0)
			pBot->m_bKareli = uint8(-1);
		if (pBot->m_bKaresiz == 0)
			pBot->m_bKaresiz = uint8(-1);

		// LOAD_MORANKER_BOTS2369 returns isolated Moranker slots 601-606.
		// Convert to client-visible bot IDs MAX_USER+601 .. MAX_USER+606.
		pBot->m_sSid += MAX_USER;
		botList.push_back(pBot);

	} while (dbCommand->MoveNext());

	return !botList.empty();
}
#pragma endregion
