#pragma once

class CGiftEventArray : public OdbcRecordset
{
public:
	CGiftEventArray(OdbcConnection * dbConnection,RandomItemArray * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("ITEM_RANDOM2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("ITEM_RANDOM1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("ITEM_RANDOM1534"); }
#endif
	virtual tstring GetColumns() { return _T("nIndex,ItemID,ItemCount,nRentalTime,nSessionID,nStatus"); }

	virtual bool Fetch()
	{
		_RANDOM_ITEM*pData = new _RANDOM_ITEM;

		_dbCommand->FetchUInt16(1, pData->Number);
		_dbCommand->FetchUInt32(2, pData->ItemID);
		_dbCommand->FetchUInt32(3, pData->ItemCount);
		_dbCommand->FetchUInt16(4, pData->nRentalTime);
		_dbCommand->FetchByte(5, pData->SessionID);
		_dbCommand->FetchByte(6, pData->Status);
	
		m_pMap->push_back(pData);
		
		return true;
	}

	RandomItemArray* m_pMap;
};