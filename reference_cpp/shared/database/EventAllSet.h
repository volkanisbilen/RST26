class CEventTimeDaySet : public OdbcRecordset
{
public:
	CEventTimeDaySet(OdbcConnection* dbConnection, EVENTMAINTIME* pData)
		: OdbcRecordset(dbConnection), m_pMap(pData) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("EVENT_SCHEDULEDAYLIST2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("EVENT_SCHEDULEDAYLIST1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("EVENT_SCHEDULEDAYLIST1534"); }
#endif
	virtual tstring GetColumns() { return _T("eventid,sunday,monday,tuesday,wednesday,thursday,friday,saturday"); }
	virtual bool Fetch() {

		int field = 1; uint8 eventid;
		EVENT_DAYLIST* pData = new EVENT_DAYLIST;
		_dbCommand->FetchByte(field++, eventid);
		for (uint8 i = 0; i < 7; i++)
			_dbCommand->Fetchtbool(field++, pData->day[i]);

		if (!m_pMap->mdaylist.PutData(eventid, pData)) { delete pData; return false; }
		return true;
	}
	EVENTMAINTIME* m_pMap;
};

class CEventTimeOptSet : public OdbcRecordset
{
public:
	CEventTimeOptSet(OdbcConnection* dbConnection, EVENTMAINTIME* pData)
		: OdbcRecordset(dbConnection), m_pMap(pData) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("EVENT_SCHEDULEMAINLIST2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("EVENT_SCHEDULEMAINLIST1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("EVENT_SCHEDULEMAINLIST1534"); }
#endif
	virtual tstring GetColumns() {
		return _T("eventid, name, type, zoneid,status,"
			"hour1, minute1, hour2, minute2, hour3, minute3, hour4, minute4, hour5, minute5,"
			"minLevel,maxLevel,reqLoyalty,reqMoney");
	}

	virtual bool Fetch() {

		int field = 1;
		EVENT_OPENTIMELIST* pData = new EVENT_OPENTIMELIST;
		_dbCommand->FetchByte(field++, pData->eventid);
		_dbCommand->FetchString(field++, pData->name);
		_dbCommand->FetchByte(field++, pData->type);
		_dbCommand->FetchByte(field++, pData->zoneid);
		_dbCommand->Fetchtbool(field++, pData->status);

		for (uint8 i = 0; i < 5; i++) {
			_dbCommand->FetchSByte(field++, pData->hour[i]);
			_dbCommand->FetchSByte(field++, pData->minute[i]);
			pData->timeactive[i] = false; pData->second[i] = 0;
		}

		_dbCommand->FetchByte(field++, pData->minLevel);
		_dbCommand->FetchByte(field++, pData->maxLevel);
		_dbCommand->FetchUInt32(field++, pData->reqLoyalty);
		_dbCommand->FetchUInt32(field++, pData->reqMoney);

		for (uint8 i = 0; i < 5; i++)
			if (pData->hour[i] >= 0)
				pData->timeactive[i] = true;

		auto* pdaylist = g_pMain->pEventTimeOpt.mdaylist.GetData(pData->eventid);
		if (!pdaylist) {
			printf("Event get day List Error:X-1 [Data evenid=%d is nullptr]\n", pData->eventid);
			delete pData;
			return false;
		}

		for (uint8 i = 0; i < 7; i++)
			pData->iday[i] = pdaylist->day[i];

		if (!m_pMap->mtimelist.PutData(pData->eventid, pData)) { delete pData; return false; }
		m_pMap->mtimeforloop.push_back(*pData);
		return true;
	}
	EVENTMAINTIME* m_pMap;
};

class CEventChaosGift : public OdbcRecordset
{
public:
	CEventChaosGift(OdbcConnection* dbConnection, void* dummy)
		: OdbcRecordset(dbConnection) {}

#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("EVENT_CHAOS_REWARDS2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("EVENT_CHAOS_REWARDS1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("EVENT_CHAOS_REWARDS1534"); }
#endif
	virtual tstring GetColumns() {
		return _T("rankid,"
			"itemid1,itemcount1,itemexpiration1,"
			"itemid2,itemcount2,itemexpiration2,"
			"itemid3,itemcount3,itemexpiration3,"
			"itemid4,itemcount4,itemexpiration4,"
			"itemid5,itemcount5,itemexpiration5,"
			"experience,loyalty,cash,noah");
	}

	virtual bool Fetch() {
		int field = 1; uint8 irank = 0;
		_dbCommand->FetchByte(field++, irank);

		if (irank > 18 || !irank) return false;
		for (uint8 x = 0; x < 5; x++) {
			_dbCommand->FetchUInt32(field++, g_pMain->pChaosReward[irank - 1].itemid[x]);
			_dbCommand->FetchUInt32(field++, g_pMain->pChaosReward[irank - 1].itemcount[x]);
			_dbCommand->FetchUInt32(field++, g_pMain->pChaosReward[irank - 1].itemtime[x]);
		}
		_dbCommand->FetchUInt32(field++, g_pMain->pChaosReward[irank - 1].experience);
		_dbCommand->FetchUInt32(field++, g_pMain->pChaosReward[irank - 1].loyalty);
		_dbCommand->FetchUInt32(field++, g_pMain->pChaosReward[irank - 1].cash);
		_dbCommand->FetchUInt32(field++, g_pMain->pChaosReward[irank - 1].noah);
		return true;
	}
};

class CEventVroomListSet : public OdbcRecordset
{
public:
	CEventVroomListSet(OdbcConnection* dbConnection, EVENTMAINTIME* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("EVENT_OPT_VROOM"); }
	virtual tstring GetColumns() { return _T("zoneid, name, sign, play, attackopen, attackclose, finish"); }

	int dizi = 0;
	virtual bool Fetch() {
		int field = 1;
		_dbCommand->FetchByte(field++, m_pMap->pvroomop[dizi].zoneid);
		_dbCommand->FetchString(field++, m_pMap->pvroomop[dizi].Name);
		_dbCommand->FetchUInt32(field++, m_pMap->pvroomop[dizi].sign);
		_dbCommand->FetchUInt32(field++, m_pMap->pvroomop[dizi].play);
		_dbCommand->FetchUInt32(field++, m_pMap->pvroomop[dizi].attackopen);
		_dbCommand->FetchUInt32(field++, m_pMap->pvroomop[dizi].attackclose);
		_dbCommand->FetchUInt32(field++, m_pMap->pvroomop[dizi].finish);
		if (++dizi > 3) return false;
		return true;
	}
	EVENTMAINTIME* m_pMap;
};

class CEventRewardSet : public OdbcRecordset
{
public:
	CEventRewardSet(OdbcConnection* dbConnection, EventRewardArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("EVENT_REWARDS"); }
	virtual tstring GetColumns()
	{
		return _T("sIndex, status, local_id, iswinner,"
			"itemid1,itemcount1,itemexpiration1,"
			"itemid2,itemcount2,itemexpiration2,"
			"itemid3,itemcount3,itemexpiration3,"
			"experience,loyalty,cash,noah");
	}

	virtual bool Fetch()
	{
		int field = 1;
		_EVENT_REWARD* pData = new _EVENT_REWARD;
		_dbCommand->FetchUInt32(field++, pData->index);
		_dbCommand->Fetchtbool(field++, pData->status);
		_dbCommand->FetchUInt16(field++, pData->local_id);
		_dbCommand->Fetchtbool(field++, pData->iswinner);
		for (int i = 0; i < 3; i++) {
			_dbCommand->FetchUInt32(field++, pData->itemid[i]);
			_dbCommand->FetchUInt32(field++, pData->itemcount[i]);
			_dbCommand->FetchUInt32(field++, pData->itemexpiration[i]);
		}
		_dbCommand->FetchUInt32(field++, pData->experience);
		_dbCommand->FetchUInt32(field++, pData->loyalty);
		_dbCommand->FetchUInt32(field++, pData->cash);
		_dbCommand->FetchUInt32(field++, pData->noah);

		if (!m_pMap->PutData(pData->index, pData)) {
			delete pData;
			return false;
		}
		return true;
	}

	EventRewardArray* m_pMap;
};

class CZoneOnlineRewardSet : public OdbcRecordset
{
public:
	CZoneOnlineRewardSet(OdbcConnection* dbConnection, ZoneOnlineRewardArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("ZONE_ONLINE_REWARD"); }
	virtual tstring GetColumns()
	{
		return _T("bZoneId,"
			"itemid, itemcount, itemtime, minute, loyalty, cash,tl,"
			"pre_itemid, pre_itemcount, pre_itemtime, pre_minute, pre_loyalty, pre_cash, pre_tl");
	}

	virtual bool Fetch()
	{
		uint32 bZoneId = 0; int field = 1;
		_ZONE_ONLINE_REWARD pData{};
		_dbCommand->FetchUInt32(field++, bZoneId);
		_dbCommand->FetchUInt32(field++, pData.itemid);
		_dbCommand->FetchUInt32(field++, pData.itemcount);
		_dbCommand->FetchUInt32(field++, pData.itemtime);
		_dbCommand->FetchUInt32(field++, pData.minute);
		_dbCommand->FetchUInt32(field++, pData.loyalty);
		_dbCommand->FetchUInt32(field++, pData.cash);
		_dbCommand->FetchUInt32(field++, pData.tl);

		_dbCommand->FetchUInt32(field++, pData.pre_itemid);
		_dbCommand->FetchUInt32(field++, pData.pre_itemcount);
		_dbCommand->FetchUInt32(field++, pData.pre_itemtime);
		_dbCommand->FetchUInt32(field++, pData.pre_minute);
		_dbCommand->FetchUInt32(field++, pData.pre_loyalty);
		_dbCommand->FetchUInt32(field++, pData.pre_cash);
		_dbCommand->FetchUInt32(field++, pData.pre_tl);

		pData.bZoneID = bZoneId;
		pData.usingtime = UNIXTIME;
		m_pMap->push_back(pData);
		return true;
	}
	ZoneOnlineRewardArray* m_pMap;
};