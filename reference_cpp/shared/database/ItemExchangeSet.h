#pragma once

class CItemExchangeSet : public OdbcRecordset
{
public:
	CItemExchangeSet(OdbcConnection * dbConnection, ItemExchangeArray *pMap) 
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("ITEM_EXCHANGE2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("ITEM_EXCHANGE1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("ITEM_EXCHANGE1534"); }
#endif

	virtual tstring GetColumns() 

	{
		return _T("nIndex, bRandomFlag, "
			"nOriginItemNum1, nOriginItemCount1, "
			"nOriginItemNum2, nOriginItemCount2, "
			"nOriginItemNum3, nOriginItemCount3, "
			"nOriginItemNum4, nOriginItemCount4, "
			"nOriginItemNum5, nOriginItemCount5, "
			"nExchangeItemNum1, nExchangeItemCount1,nExchangeItemTime1, "
			"nExchangeItemNum2, nExchangeItemCount2,nExchangeItemTime2, "
			"nExchangeItemNum3, nExchangeItemCount3,nExchangeItemTime3, "
			"nExchangeItemNum4, nExchangeItemCount4,nExchangeItemTime4, "
			"nExchangeItemNum5, nExchangeItemCount5,nExchangeItemTime5");
	}

	virtual bool Fetch()
	{
		_ITEM_EXCHANGE *pData = new _ITEM_EXCHANGE;

		int i = 1;
		_dbCommand->FetchUInt32(i++, pData->nIndex);
		_dbCommand->FetchByte(i++, pData->bRandomFlag);

		for (int x = 0; x < ITEMS_IN_ORIGIN_GROUP; x++)
		{
			_dbCommand->FetchUInt32(i++, pData->nOriginItemNum[x]);
			_dbCommand->FetchUInt32(i++, pData->sOriginItemCount[x]);
		}

		for (int j = 0; j < ITEMS_IN_EXCHANGE_GROUP; j++)
		{
			_dbCommand->FetchUInt32(i++, pData->nExchangeItemNum[j]);
			_dbCommand->FetchUInt32(i++, pData->sExchangeItemCount[j]);
			_dbCommand->FetchByte(i++, pData->sExchangeItemTime[j]);
		}

		if (!m_pMap->PutData(pData->nIndex, pData))
			delete pData;

		return true;
	}

	ItemExchangeArray * m_pMap;
};