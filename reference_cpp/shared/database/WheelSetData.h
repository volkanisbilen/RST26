#pragma once

class CWheelData : public OdbcRecordset
{
public:
	CWheelData(OdbcConnection * dbConnection, ItemWheelArray * pMap) : OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("WHEEL_SETTINGS"); }
	virtual tstring GetColumns() { return _T("nIndex, nItemID, nItemCount, nRentalTime,nFlag,DropRate"); }
	virtual tstring GetOrderBy() { return _T("DropRate"); }

	virtual bool Fetch()
	{
		auto pData = new _WHEEL_DATA;

		auto i = 1;
		_dbCommand->FetchUInt32(i++, pData->nIndex);
		_dbCommand->FetchUInt32(i++, pData->nItemID);
		_dbCommand->FetchUInt16(i++, pData->nItemCount);
		_dbCommand->FetchUInt16(i++, pData->nRentalTime);
		_dbCommand->FetchByte(i++, pData->nFlag);
		_dbCommand->FetchUInt16(i++, pData->ExchangeRate);
		if (!m_pMap->PutData(pData->nIndex, pData))
			delete pData;

		return true;
	}

	ItemWheelArray* m_pMap;
};