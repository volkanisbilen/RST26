#pragma once

class CItemSellTableSet : public OdbcRecordset
{
public:
	CItemSellTableSet(OdbcConnection * dbConnection, ItemSellTableArray * pMap) 
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("ITEM_SELLTABLE2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("ITEM_SELLTABLE1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("ITEM_SELLTABLE1534"); }
#endif
	virtual tstring GetColumns() { return _T("nIndex, iSellingGroup, Item1, Item2, Item3, Item4, Item5, Item6, Item7, Item8, Item9, Item10,\
											 Item11, Item12, Item13, Item14, Item15, Item16, Item17, Item18, Item19, Item20, Item21,\
											 Item22, Item23, Item24"); }

	virtual bool Fetch()
	{
		auto * pData = new _ITEM_SELLTABLE();
		uint32_t Column = 1;

		_dbCommand->FetchUInt32(Column++, pData->nIndex);
		_dbCommand->FetchUInt32(Column++, pData->iSellingGroup);

		for (int32_t i = 0; i < 24; i++)
			_dbCommand->FetchUInt32(Column++, pData->ItemIDs[i]);
		
		if (!m_pMap->PutData(pData->nIndex, pData))
			delete pData;

		return true;
	}
	ItemSellTableArray *m_pMap;
};