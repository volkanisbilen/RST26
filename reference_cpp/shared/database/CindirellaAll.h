#pragma once

class CCindirellaItemSet1 : public OdbcRecordset
{
public:
	CCindirellaItemSet1(OdbcConnection* dbConnection, CindirellaItemsArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS0_2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS0_1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS0_1534"); }
#endif
	virtual tstring GetColumns() { return _T("id, class, slotid, itemid, itemcount,itemduration,itemflag,itemexpire"); }
	virtual bool Fetch() {
		int field = 1;

		_CINDWAR_ITEMS* pData = new _CINDWAR_ITEMS;
		_dbCommand->FetchUInt32(field++, pData->id);
		_dbCommand->FetchByte(field++, pData->iclass);
		_dbCommand->FetchByte(field++, pData->slotid);
		_dbCommand->FetchUInt32(field++, pData->itemid);
		_dbCommand->FetchUInt16(field++, pData->itemcount);
		_dbCommand->FetchUInt16(field++, pData->itemduration);
		_dbCommand->FetchByte(field++, pData->itemflag);
		_dbCommand->FetchUInt32(field++, pData->itemexpire);
		if (!m_pMap->PutData(pData->id, pData)) { delete pData; return false; }
		return true;
	}
	CindirellaItemsArray* m_pMap;
};

class CCindirellaItemSet2 : public OdbcRecordset
{
public:
	CCindirellaItemSet2(OdbcConnection* dbConnection, CindirellaItemsArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS1_2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS1_1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS1_1534"); }
#endif
	virtual tstring GetColumns() { return _T("id, class, slotid, itemid, itemcount,itemduration,itemflag,itemexpire"); }
	virtual bool Fetch() {
		int field = 1;

		_CINDWAR_ITEMS* pData = new _CINDWAR_ITEMS;
		_dbCommand->FetchUInt32(field++, pData->id);
		_dbCommand->FetchByte(field++, pData->iclass);
		_dbCommand->FetchByte(field++, pData->slotid);
		_dbCommand->FetchUInt32(field++, pData->itemid);
		_dbCommand->FetchUInt16(field++, pData->itemcount);
		_dbCommand->FetchUInt16(field++, pData->itemduration);
		_dbCommand->FetchByte(field++, pData->itemflag);
		_dbCommand->FetchUInt32(field++, pData->itemexpire);
		if (!m_pMap->PutData(pData->id, pData)) { delete pData; return false; }
		return true;
	}
	CindirellaItemsArray* m_pMap;
};

class CCindirellaItemSet3 : public OdbcRecordset
{
public:
	CCindirellaItemSet3(OdbcConnection* dbConnection, CindirellaItemsArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS2_2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS2_1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS2_1534"); }
#endif
	virtual tstring GetColumns() { return _T("id, class, slotid, itemid, itemcount,itemduration,itemflag,itemexpire"); }
	virtual bool Fetch() {
		int field = 1;

		_CINDWAR_ITEMS* pData = new _CINDWAR_ITEMS;
		_dbCommand->FetchUInt32(field++, pData->id);
		_dbCommand->FetchByte(field++, pData->iclass);
		_dbCommand->FetchByte(field++, pData->slotid);
		_dbCommand->FetchUInt32(field++, pData->itemid);
		_dbCommand->FetchUInt16(field++, pData->itemcount);
		_dbCommand->FetchUInt16(field++, pData->itemduration);
		_dbCommand->FetchByte(field++, pData->itemflag);
		_dbCommand->FetchUInt32(field++, pData->itemexpire);
		if (!m_pMap->PutData(pData->id, pData)) { delete pData; return false; }
		return true;
	}
	CindirellaItemsArray* m_pMap;
};

class CCindirellaItemSet4 : public OdbcRecordset
{
public:
	CCindirellaItemSet4(OdbcConnection* dbConnection, CindirellaItemsArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS3_2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS3_1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS3_1534"); }
#endif
	virtual tstring GetColumns() { return _T("id, class, slotid, itemid, itemcount,itemduration,itemflag,itemexpire"); }
	virtual bool Fetch() {
		int field = 1;

		_CINDWAR_ITEMS* pData = new _CINDWAR_ITEMS;
		_dbCommand->FetchUInt32(field++, pData->id);
		_dbCommand->FetchByte(field++, pData->iclass);
		_dbCommand->FetchByte(field++, pData->slotid);
		_dbCommand->FetchUInt32(field++, pData->itemid);
		_dbCommand->FetchUInt16(field++, pData->itemcount);
		_dbCommand->FetchUInt16(field++, pData->itemduration);
		_dbCommand->FetchByte(field++, pData->itemflag);
		_dbCommand->FetchUInt32(field++, pData->itemexpire);
		if (!m_pMap->PutData(pData->id, pData)) { delete pData; return false; }
		return true;
	}
	CindirellaItemsArray* m_pMap;
};

class CCindirellaItemSet5 : public OdbcRecordset
{
public:
	CCindirellaItemSet5(OdbcConnection* dbConnection, CindirellaItemsArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS4_2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS4_1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CINDWAR_ITEMS4_1534"); }
#endif
	virtual tstring GetColumns() { return _T("id, class, slotid, itemid, itemcount,itemduration,itemflag,itemexpire"); }
	virtual bool Fetch() {
		int field = 1;

		_CINDWAR_ITEMS* pData = new _CINDWAR_ITEMS;
		_dbCommand->FetchUInt32(field++, pData->id);
		_dbCommand->FetchByte(field++, pData->iclass);
		_dbCommand->FetchByte(field++, pData->slotid);
		_dbCommand->FetchUInt32(field++, pData->itemid);
		_dbCommand->FetchUInt16(field++, pData->itemcount);
		_dbCommand->FetchUInt16(field++, pData->itemduration);
		_dbCommand->FetchByte(field++, pData->itemflag);
		_dbCommand->FetchUInt32(field++, pData->itemexpire);
		if (!m_pMap->PutData(pData->id, pData)) { delete pData; return false; }
		return true;
	}
	CindirellaItemsArray* m_pMap;
};

class CCindirellaStatSet : public OdbcRecordset
{
public:
	CCindirellaStatSet(OdbcConnection* dbConnection, CindirellaStatArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CINDWAR_STAT2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CINDWAR_STAT1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CINDWAR_STAT1534"); }
#endif
	virtual tstring GetColumns() { return _T("id, settingid, iClass, s_freepoint,s_page1, s_page2,s_page3,s_page4,StatStr,StatSta,StatDex,StatInt,StatCha,Statfreepoint"); }
	virtual bool Fetch() {
		int field = 1;

		_CINDWAR_STATSET* pData = new _CINDWAR_STATSET;
		_dbCommand->FetchUInt32(field++, pData->id);
		_dbCommand->FetchByte(field++, pData->settindid);
		_dbCommand->FetchByte(field++, pData->iClass);
		_dbCommand->FetchUInt16(field++, pData->Skillfreepoint);
		
		for (int i = 0 ; i < 4; i++)
			_dbCommand->FetchByte(field++, pData->skill[i]);

		for (int i = 0; i < 5; i++)
			_dbCommand->FetchByte(field++, pData->stat[i]);

		_dbCommand->FetchUInt16(field++, pData->Statfreepoint);

		if (!m_pMap->PutData(pData->id, pData)) { delete pData; return false; }
		return true;
	}
	CindirellaStatArray* m_pMap;
};

class CCindirellaSettingSet : public OdbcRecordset
{
public:
	CCindirellaSettingSet(OdbcConnection* dbConnection, void* dummy)
		: OdbcRecordset(dbConnection) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CINDWAR_SETTING2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CINDWAR_SETTING1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CINDWAR_SETTING1534"); }
#endif
	virtual tstring GetColumns() {
		return _T("settingid, playtime,preparetime, minlevel,maxlevel,reqmoney,reqloyalty,"
			"maxuserlimit,zoneid,beginnerlevel");
	}

	virtual bool Fetch() {
		int field = 1;
		uint8 settingid = 0;
		_dbCommand->FetchByte(field++, settingid);
		_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pSetting[settingid].playtime);
		_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pSetting[settingid].preparetime);
		_dbCommand->FetchByte(field++, g_pMain->pCindWar.pSetting[settingid].minlevel);
		_dbCommand->FetchByte(field++, g_pMain->pCindWar.pSetting[settingid].maxlevel);
		_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pSetting[settingid].reqmoney);
		_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pSetting[settingid].reqloyalty);
		_dbCommand->FetchUInt16(field++, g_pMain->pCindWar.pSetting[settingid].maxuserlimit);
		_dbCommand->FetchByte(field++, g_pMain->pCindWar.pSetting[settingid].zoneid);
		_dbCommand->FetchByte(field++, g_pMain->pCindWar.pSetting[settingid].beglevel);
		return true;
	}
};

class CCindirellaRewardSet : public OdbcRecordset
{
public:
	CCindirellaRewardSet(OdbcConnection* dbConnection, void* dummy)
		: OdbcRecordset(dbConnection) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CINDWAR_REWARD2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CINDWAR_REWARD1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CINDWAR_REWARD1534"); }
#endif
	virtual tstring GetColumns() {
		return _T("irankd, expcount, cashcount,loyaltycount, moneycount,"
			"itemid1,itemcount1,itemduration1,itemexpiration1,"
			"itemid2,itemcount2,itemduration2,itemexpiration2,"
			"itemid3,itemcount3,itemduration3,itemexpiration3,"
			"itemid4,itemcount4,itemduration4,itemexpiration4,"
			"itemid5,itemcount5,itemduration5,itemexpiration5,"
			"itemid6,itemcount6,itemduration6,itemexpiration6,"
			"itemid7,itemcount7,itemduration7,itemexpiration7,"
			"itemid8,itemcount8,itemduration8,itemexpiration8,"
			"itemid9,itemcount9,itemduration9,itemexpiration9,"
			"itemid10,itemcount10,itemduration10,itemexpiration10");
	}

	virtual bool Fetch() {
		int field = 1;

		uint16 rankid = 0;
		_dbCommand->FetchUInt16(field++, rankid);
		if (rankid > 0) rankid -= 1;
		_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pReward[rankid].expcount);
		_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pReward[rankid].cashcount);
		_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pReward[rankid].loyaltycount);
		_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pReward[rankid].moneycount);
		for (int i = 0; i < 10; i++) {
			_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pReward[rankid].itemid[i]);
			_dbCommand->FetchUInt16(field++, g_pMain->pCindWar.pReward[rankid].itemcount[i]);
			_dbCommand->FetchUInt16(field++, g_pMain->pCindWar.pReward[rankid].itemduration[i]);
			_dbCommand->FetchUInt32(field++, g_pMain->pCindWar.pReward[rankid].itemexpiration[i]);
		}
		return true;
	}
};