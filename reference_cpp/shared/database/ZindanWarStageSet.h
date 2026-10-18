#pragma once

class CZindanWarStagesList : public OdbcRecordset
{
public:
	CZindanWarStagesList(OdbcConnection * dbConnection, ZindanWarStageListArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("ZINDAN_WAR_STAGES"); }
	virtual tstring GetColumns() { return _T("nIndex, Type, Stage, Time"); }

	virtual bool Fetch()
	{
		int field = 1;
		_ZINDAN_WAR_STAGES* pData = new _ZINDAN_WAR_STAGES;
		_dbCommand->FetchUInt32(field++, pData->nIndex);
		_dbCommand->FetchByte(field++, pData->Type);
		_dbCommand->FetchByte(field++, pData->Stage);
		_dbCommand->FetchUInt16(field++, pData->Time);
		if (!m_pMap->PutData(pData->nIndex, pData))
			delete pData;

		return true;
	}
	ZindanWarStageListArray*m_pMap;
};