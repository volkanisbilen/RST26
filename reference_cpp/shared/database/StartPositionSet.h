#pragma once

class CStartPositionSet : public OdbcRecordset
{
public:
	CStartPositionSet(OdbcConnection * dbConnection, StartPositionArray * pMap) 
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("START_POSITION2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("START_POSITION1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("START_POSITION1534"); }
#endif
	virtual tstring GetColumns() { return _T("ZoneID, sKarusX, sKarusZ, sElmoradX, sElmoradZ, bRangeX, bRangeZ, sKarusGateX, sKarusGateZ, sElmoGateX, sElmoGateZ"); }

	virtual bool Fetch()
	{
		_START_POSITION *pData = new _START_POSITION;

		_dbCommand->FetchUInt16(1, pData->ZoneID);
		_dbCommand->FetchUInt16(2, pData->sKarusX);
		_dbCommand->FetchUInt16(3, pData->sKarusZ);
		_dbCommand->FetchUInt16(4, pData->sElmoradX);
		_dbCommand->FetchUInt16(5, pData->sElmoradZ);
		_dbCommand->FetchByte (6, pData->bRangeX);
		_dbCommand->FetchByte (7, pData->bRangeZ);
		_dbCommand->FetchUInt16(8, pData->sKarusGateX);
		_dbCommand->FetchUInt16(9, pData->sKarusGateZ);
		_dbCommand->FetchUInt16(10, pData->sElmoradGateX);
		_dbCommand->FetchUInt16(11, pData->sElmoradGateZ);


		if (!m_pMap->PutData(pData->ZoneID, pData))
			delete pData;

		return true;
	}

	StartPositionArray *m_pMap;
};