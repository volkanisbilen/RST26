#pragma once

class CTimedNotice : public OdbcRecordset
{
public:
	CTimedNotice(OdbcConnection * dbConnection, TimedNoticeArray*pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("TIMED_NOTICE"); }
	virtual tstring GetColumns() { return _T("nIndex, noticetype, notice, zoneid, time"); }

	virtual bool Fetch()
	{
		int field = 1;
		_TIMED_NOTICE *pData = new _TIMED_NOTICE;
		_dbCommand->FetchUInt32(field++, pData->nIndex);
		_dbCommand->FetchByte(field++, pData->noticetype);
		_dbCommand->FetchString(field++, pData->notice);
		_dbCommand->FetchUInt16(field++, pData->zoneid);
		_dbCommand->FetchUInt32(field++, pData->time);
		if (pData->time < 1) pData->time = 1;
		pData->usingtime = (uint32)UNIXTIME + pData->time * MINUTE;
		if (!m_pMap->PutData(pData->nIndex, pData))
			delete pData;
		return true;
	}
	TimedNoticeArray* m_pMap;
};