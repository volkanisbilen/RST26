#pragma once

class CEventTimerShowSet : public OdbcRecordset
{
public:
	CEventTimerShowSet(OdbcConnection * dbConnection, EventTimerShowArray *pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("EVENT_TIMER_SHOWLIST2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("EVENT_TIMER_SHOWLIST1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("EVENT_TIMER_SHOWLIST1534"); }
#endif
	virtual tstring GetColumns() 
	{
		return _T("name, status, hour, minute, days, id");
	}

	virtual bool Fetch()
	{
		int i = 1;
		_EVENTTIMER_SHOWLIST *pData = new _EVENTTIMER_SHOWLIST;
		_dbCommand->FetchString(i++, pData->name);
		_dbCommand->Fetchtbool(i++, pData->status);
		_dbCommand->FetchUInt32(i++, pData->hour);
		_dbCommand->FetchUInt32(i++, pData->minute);
		_dbCommand->FetchString(i++, pData->days);
		_dbCommand->FetchUInt32(i++, pData->id);
		if (!m_pMap->PutData(pData->id, pData)) {
			delete pData;
			return false;
		}
		return true;
	}

	EventTimerShowArray* m_pMap;
};