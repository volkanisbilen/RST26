#pragma once

class CBotSaveDataSet : public OdbcRecordset
{
public:
	CBotSaveDataSet(OdbcConnection* dbConnection, BotSaveDataArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("BOT_MERCHANT_DATA"); }
	virtual tstring GetColumns()
	{
		return _T("nIndex, advertMessage, nNum1, nPrice1, sCount1, sDuration1,iskc1,"
			"nNum2, nPrice2, sCount2, sDuration2,iskc2,"
			"nNum3, nPrice3, sCount3, sDuration3,iskc3,"
			"nNum4, nPrice4, sCount4, sDuration4,iskc4," 
			"nNum5, nPrice5, sCount5, sDuration5,iskc5," 
			"nNum6, nPrice6, sCount6, sDuration6,iskc6," 
			"nNum7, nPrice7, sCount7, sDuration7,iskc7," 
			"nNum8, nPrice8, sCount8, sDuration8,iskc8," 
			"nNum9, nPrice9, sCount9, sDuration9,iskc9," 
			"nNum10, nPrice10, sCount10, sDuration10,iskc10," 
			"nNum11, nPrice11, sCount11, sDuration11,iskc11," 
			"nNum12, nPrice12, sCount12, sDuration12,iskc12, "
			"PX, PZ, PY, Minute, Zone, sDirection,MerchantType");
	}

	virtual bool Fetch()
	{
		_BOT_SAVE_DATA * pAuto = new _BOT_SAVE_DATA();
		int field = 1;

		_dbCommand->FetchUInt32(field++, pAuto->nAutoID);
		_dbCommand->FetchString(field++, pAuto->AdvertMessage);

		for (int i = 0; i < MAX_MERCH_ITEMS; i++)
		{
			_dbCommand->FetchUInt32(field++, pAuto->nNum[i]);
			_dbCommand->FetchUInt32(field++, pAuto->nPrice[i]);
			_dbCommand->FetchUInt16(field++, pAuto->sCount[i]);
			_dbCommand->FetchInt16(field++, pAuto->sDuration[i]);
			_dbCommand->Fetchtbool(field++, pAuto->isKc[i]);
			pAuto->IsSoldOut[i] = false;
			pAuto->nSerialNum[i] = g_pMain->GenerateItemSerial();
		}

		pAuto->fX = (float)(_dbCommand->FetchInt32(field++) / 100.0f);
		pAuto->fZ = (float)(_dbCommand->FetchInt32(field++) / 100.0f);
		pAuto->fY = (float)(_dbCommand->FetchInt32(field++) / 100.0f);
		_dbCommand->FetchUInt16(field++, pAuto->Minute);
		_dbCommand->FetchByte(field++, pAuto->byZone);
		_dbCommand->FetchInt16(field++, pAuto->Direction);
		_dbCommand->FetchByte(field++, pAuto->sMerchanType);

		if (!m_pMap->PutData(pAuto->nAutoID, pAuto))
			delete pAuto;

		return true;
	}

	BotSaveDataArray* m_pMap;
};

class CBotMerchantInfoSet : public OdbcRecordset
{
public:
	CBotMerchantInfoSet(OdbcConnection* dbConnection, _BOT_INFO* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("MERCHANT_BOT_INFO"); }
	virtual tstring GetColumns()
	{
		return _T("nIndex, type, setX, setY, setZ,bZoneID,direction,isbuy");
	}

	virtual bool Fetch()
	{
		int field = 1; uint32 index = 0;
		_MERCHANT_BOT_INFO* pAuto = new _MERCHANT_BOT_INFO();
		_dbCommand->FetchUInt32(field++, index);
		_dbCommand->FetchUInt16(field++, pAuto->type);
		_dbCommand->FetchUInt16(field++, pAuto->setX);
		_dbCommand->FetchInt16(field++, pAuto->setY);
		_dbCommand->FetchUInt16(field++, pAuto->setZ);
		_dbCommand->FetchByte(field++, pAuto->bZoneID);
		_dbCommand->FetchInt32(field++, pAuto->direction);
		_dbCommand->Fetchtbool(field++, pAuto->isBuy);
		pAuto->used = false;
		if (!m_pMap->mCoordinate.PutData(index, pAuto)) {
			delete pAuto;
			return false;
		}
		return true;
	}

	_BOT_INFO* m_pMap;
};

class CBotMerchantItemSet : public OdbcRecordset
{
public:
	CBotMerchantItemSet(OdbcConnection* dbConnection, _BOT_INFO* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("MERCHANT_BOT_ITEM"); }
	virtual tstring GetColumns()
	{
		return _T("itemid, minItemCount,maxItemCount, MinPrice,MaxPrice,MinKC,MaxKC,type,moneytype");
	}

	virtual bool Fetch()
	{
		int field = 1; 
		_MERCHANT_BOT_ITEM* pAuto = new _MERCHANT_BOT_ITEM();
		_dbCommand->FetchUInt32(field++, pAuto->itemid);
		_dbCommand->FetchUInt32(field++, pAuto->minItemCount);
		_dbCommand->FetchUInt32(field++, pAuto->maxItemCount);
		_dbCommand->FetchUInt32(field++, pAuto->minPrice);
		_dbCommand->FetchUInt32(field++, pAuto->maxPrice);
		_dbCommand->FetchUInt32(field++, pAuto->minKc);
		_dbCommand->FetchUInt32(field++, pAuto->maxKc);
		_dbCommand->FetchUInt16(field++, pAuto->type);
		_dbCommand->FetchByte(field++, pAuto->moneytype);
		if (!m_pMap->mItem.PutData(pAuto->itemid, pAuto)) {
			delete pAuto;
			return false;
		}
		return true;
	}

	_BOT_INFO* m_pMap;
};