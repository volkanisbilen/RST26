#pragma once

class CItemRightClickExchange : public OdbcRecordset
{
public:
	CItemRightClickExchange(OdbcConnection* dbConnection, LoadRightClickExchange* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("RIGHT_CLICK_EXCHANGE"); }
	virtual tstring GetColumns()
	{
		return _T("nOriginItem,"
			"ExchangeItem1, ExchangeItemCount1, ExchangeItemDuration1, ExchangeItemFlag1, ExchangeItemTime1, "
			"ExchangeItem2, ExchangeItemCount2, ExchangeItemDuration2, ExchangeItemFlag2, ExchangeItemTime2, "
			"ExchangeItem3, ExchangeItemCount3, ExchangeItemDuration3, ExchangeItemFlag3, ExchangeItemTime3, "
			"ExchangeItem4, ExchangeItemCount4, ExchangeItemDuration4, ExchangeItemFlag4, ExchangeItemTime4, "
			"ExchangeItem5, ExchangeItemCount5, ExchangeItemDuration5, ExchangeItemFlag5, ExchangeItemTime5, "
			"ExchangeItem6, ExchangeItemCount6, ExchangeItemDuration6, ExchangeItemFlag6, ExchangeItemTime6, "
			"ExchangeItem7, ExchangeItemCount7, ExchangeItemDuration7, ExchangeItemFlag7, ExchangeItemTime7, "
			"ExchangeItem8, ExchangeItemCount8, ExchangeItemDuration8, ExchangeItemFlag8, ExchangeItemTime8, "
			"ExchangeItem9, ExchangeItemCount9, ExchangeItemDuration9, ExchangeItemFlag9, ExchangeItemTime9, "
			"ExchangeItem10, ExchangeItemCount10, ExchangeItemDuration10, ExchangeItemFlag10, ExchangeItemTime10, "
			"ExchangeItem11, ExchangeItemCount11, ExchangeItemDuration11, ExchangeItemFlag11, ExchangeItemTime11, "
			"ExchangeItem12, ExchangeItemCount12, ExchangeItemDuration12, ExchangeItemFlag12, ExchangeItemTime12, "
			"ExchangeItem13, ExchangeItemCount13, ExchangeItemDuration13, ExchangeItemFlag13, ExchangeItemTime13, "
			"ExchangeItem14, ExchangeItemCount14, ExchangeItemDuration14, ExchangeItemFlag14, ExchangeItemTime14, "
			"ExchangeItem15, ExchangeItemCount15, ExchangeItemDuration15, ExchangeItemFlag15, ExchangeItemTime15, "
			"ExchangeItem16, ExchangeItemCount16, ExchangeItemDuration16, ExchangeItemFlag16, ExchangeItemTime16, "
			"ExchangeItem17, ExchangeItemCount17, ExchangeItemDuration17, ExchangeItemFlag17, ExchangeItemTime17, "
			"ExchangeItem18, ExchangeItemCount18, ExchangeItemDuration18, ExchangeItemFlag18, ExchangeItemTime18, "
			"ExchangeItem19, ExchangeItemCount19, ExchangeItemDuration19, ExchangeItemFlag19, ExchangeItemTime19, "
			"ExchangeItem20, ExchangeItemCount20, ExchangeItemDuration20, ExchangeItemFlag20, ExchangeItemTime20, "
			"ExchangeItem21, ExchangeItemCount21, ExchangeItemDuration21, ExchangeItemFlag21, ExchangeItemTime21, "
			"ExchangeItem22, ExchangeItemCount22, ExchangeItemDuration22, ExchangeItemFlag22, ExchangeItemTime22, "
			"ExchangeItem23, ExchangeItemCount23, ExchangeItemDuration23, ExchangeItemFlag23, ExchangeItemTime23, "
			"ExchangeItem24, ExchangeItemCount24, ExchangeItemDuration24, ExchangeItemFlag24, ExchangeItemTime24, "
			"ExchangeItem25, ExchangeItemCount25, ExchangeItemDuration25, ExchangeItemFlag25, ExchangeItemTime25, "
			"Status, Selection");

	}

	virtual bool Fetch()
	{
		_RIGHT_CLICK_EXCHANGE* pData = new _RIGHT_CLICK_EXCHANGE;

		int i = 1;
		_dbCommand->FetchUInt32(i++, pData->nBaseItemID);
		for (int x = 0; x < ITEMS_RIGHT_CLICK_EXCHANGE_GROUP; x++)
		{
			_dbCommand->FetchUInt32(i++, pData->nItemID[x]);
			_dbCommand->FetchUInt32(i++, pData->nCount[x]);
			_dbCommand->FetchUInt16(i++, pData->sDurability[x]);
			_dbCommand->FetchByte(i++, pData->bFlag[x]);
			_dbCommand->FetchUInt32(i++, pData->nRentalTime[x]);

		}

		_dbCommand->FetchByte(i++, pData->bStatus);
		_dbCommand->FetchByte(i++, pData->bSelection);

		if (!m_pMap->PutData(pData->nBaseItemID, pData))
			delete pData;

		g_pMain->RightClickExchangeItemList.push_back(pData->nBaseItemID);
		return true;
	}

	LoadRightClickExchange* m_pMap;
};