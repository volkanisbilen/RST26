#pragma once

class CZindanWarMonsList : public OdbcRecordset
{
public:
	CZindanWarMonsList(OdbcConnection * dbConnection, ZindanWarSummonListArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("ZINDAN_WAR_SUMMON_LIST"); }
	virtual tstring GetColumns() { return _T("bIndex, Type, Stage, SidID, SidCount, PosX, PosZ, Range"); }

	virtual bool Fetch()
	{
		int field = 1;
		_ZINDAN_WAR_SUMMON* pData = new _ZINDAN_WAR_SUMMON;
		_dbCommand->FetchUInt32(field++, pData->bIndex);
		_dbCommand->FetchByte(field++, pData->Type);
		_dbCommand->FetchByte(field++, pData->Stage);
		_dbCommand->FetchUInt16(field++, pData->SidID);
		_dbCommand->FetchUInt16(field++, pData->SidCount);
		_dbCommand->FetchUInt16(field++, pData->PosX);
		_dbCommand->FetchUInt16(field++, pData->PosZ);
		_dbCommand->FetchUInt16(field++, pData->Range);
		if (!m_pMap->PutData(pData->bIndex, pData))
			delete pData;

		return true;
	}
	ZindanWarSummonListArray*m_pMap;
};