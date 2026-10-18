#pragma once

class CLuaGiveItemExchangeSet : public OdbcRecordset
{
public:
	CLuaGiveItemExchangeSet(OdbcConnection * dbConnection, LuaGiveItemExchangeArray *pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("ITEM_GIVE_EXCHANGE"); }
	virtual tstring GetColumns()
	{
		return _T("nExchangeIndex, "
			"nRobItemID1, nRobItemID2, "
			"nRobItemID3, nRobItemID4, "
			"nRobItemID5, nRobItemID6, "
			"nRobItemID7, nRobItemID8, "
			"nRobItemID9, nRobItemID10, "
			"nRobItemID11, nRobItemID12, "
			"nRobItemID13, nRobItemID14, "
			"nRobItemID15, nRobItemID16, "
			"nRobItemID17, nRobItemID18, "
			"nRobItemID19, nRobItemID20, "
			"nRobItemID21, nRobItemID22, "
			"nRobItemID23, nRobItemID24, "
			"nRobItemID25, "
			"nRobItemCount1, nRobItemCount2, "
			"nRobItemCount3, nRobItemCount4, "
			"nRobItemCount5, nRobItemCount6, "
			"nRobItemCount7, nRobItemCount8, "
			"nRobItemCount9, nRobItemCount10, "
			"nRobItemCount11, nRobItemCount12, "
			"nRobItemCount13, nRobItemCount14, "
			"nRobItemCount15, nRobItemCount16, "
			"nRobItemCount17, nRobItemCount18, "
			"nRobItemCount19, nRobItemCount20, "
			"nRobItemCount21, nRobItemCount22, "
			"nRobItemCount23, nRobItemCount24, "
			"nRobItemCount25, "
			"nGiveItemID1, nGiveItemID2, "
			"nGiveItemID3, nGiveItemID4, "
			"nGiveItemID5, nGiveItemID6, "
			"nGiveItemID7, nGiveItemID8, "
			"nGiveItemID9, nGiveItemID10, "
			"nGiveItemID11, nGiveItemID12, "
			"nGiveItemID13, nGiveItemID14, "
			"nGiveItemID15, nGiveItemID16, "
			"nGiveItemID17, nGiveItemID18, "
			"nGiveItemID19, nGiveItemID20, "
			"nGiveItemID21, nGiveItemID22, "
			"nGiveItemID23, nGiveItemID24, "
			"nGiveItemID25, "
			"nGiveItemCount1, nGiveItemCount2, "
			"nGiveItemCount3, nGiveItemCount4, "
			"nGiveItemCount5, nGiveItemCount6, "
			"nGiveItemCount7, nGiveItemCount8, "
			"nGiveItemCount9, nGiveItemCount10, "
			"nGiveItemCount11, nGiveItemCount12, "
			"nGiveItemCount13, nGiveItemCount14, "
			"nGiveItemCount15, nGiveItemCount16, "
			"nGiveItemCount17, nGiveItemCount18, "
			"nGiveItemCount19, nGiveItemCount20, "
			"nGiveItemCount21, nGiveItemCount22, "
			"nGiveItemCount23, nGiveItemCount24, "
			"nGiveItemCount25, "
			"nGiveItemTime1, nGiveItemTime2,   "
			"nGiveItemTime3, nGiveItemTime4,   "
			"nGiveItemTime5, nGiveItemTime6,   "
			"nGiveItemTime7, nGiveItemTime8,   "
			"nGiveItemTime9, nGiveItemTime10   "
			"nGiveItemTime11, nGiveItemTime12, "
			"nGiveItemTime13, nGiveItemTime14, "
			"nGiveItemTime15, nGiveItemTime16, "
			"nGiveItemTime17, nGiveItemTime18, "
			"nGiveItemTime19, nGiveItemTime20  "
			"nGiveItemTime21, nGiveItemTime22, "
			"nGiveItemTime23, nGiveItemTime24, "
			"nGiveItemTime25");
	}

	virtual bool Fetch()
	{
		_GIVE_LUA_ITEM_EXCHANGE *pData = new _GIVE_LUA_ITEM_EXCHANGE;

		int i = 1;
		_dbCommand->FetchUInt32(i++, pData->nExchangeID);

		for (int x = 0; x < ITEMS_IN_ROB_ITEM_GROUP_LUA; x++)
			_dbCommand->FetchUInt32(i++, pData->nRobItemID[x]);

		for (int x = 0; x < ITEMS_IN_ROB_ITEM_GROUP_LUA; x++)
			_dbCommand->FetchUInt32(i++, pData->nRobItemCount[x]);

		for (int j = 0; j < ITEMS_IN_GIVE_ITEM_GROUP_LUA; j++)
			_dbCommand->FetchUInt32(i++, pData->nGiveItemID[j]);

		for (int j = 0; j < ITEMS_IN_GIVE_ITEM_GROUP_LUA; j++)
			_dbCommand->FetchUInt32(i++, pData->nGiveItemCount[j]);

		for (int j = 0; j < ITEMS_IN_GIVE_ITEM_GROUP_LUA; j++)
			_dbCommand->FetchUInt32(i++, pData->nGiveItemTime[j]);

		if (!m_pMap->PutData(pData->nExchangeID, pData))
			delete pData;

		return true;
	}

	LuaGiveItemExchangeArray * m_pMap;
};