#pragma once

class CBotLoadData : public OdbcRecordset
{
public:
	CBotLoadData(OdbcConnection * dbConnection, BotSaveDataArray * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("BOT_MERCHANT_DATA"); }
	virtual tstring GetColumns() { return _T("nIndex, strMerchantChat,strMerchantData,MerchantType,Zone,X,Y"); }

	virtual bool Fetch()
	{
		_BOT_SAVE_DATA *pData = new _BOT_SAVE_DATA;

		_dbCommand->FetchUInt32(1, pData->nIndex);
		_dbCommand->FetchString(2, pData->strResource);

		if (!m_pMap->PutData(pData->nResourceID, pData))
			delete pData;

		return true;
	}

	BotSaveDataArray *m_pMap;
};