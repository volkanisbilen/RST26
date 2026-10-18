#pragma once

class CRimaLottery : public OdbcRecordset
{
public:
	CRimaLottery(OdbcConnection * dbConnection, RimaLotteryArray *pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("LOTTERY_EVENT_SETTINGS"); }
	virtual tstring GetColumns() 
	{
		return _T("LNum ,"
			"reqItem1,reqItemCount1 ,reqItem2,reqItemCount2 ,reqItem3,reqItemCount3,reqItem4,reqItemCount4,reqItem5,reqItemCount5,"
			"rewardItem1,rewardItem2,rewardItem3,rewardItem4,"
			"userLimit,eventTime");
	}

	virtual bool Fetch()
	{
		int field = 1;
		_RIMA_LOTTERY_DB *pData = new _RIMA_LOTTERY_DB;
		_dbCommand->FetchUInt32(field++, pData->iNum);

		for (int x = 0; x < 5; x++) {
			_dbCommand->FetchUInt32(field++, pData->nReqItem[x]);
			_dbCommand->FetchUInt32(field++, pData->nReqItemCount[x]);
		}
			
		for (int s = 0; s < 4; s++)
			_dbCommand->FetchUInt32(field++, pData->nRewardItem[s]);

		_dbCommand->FetchUInt32(field++, pData->UserLimit);
		_dbCommand->FetchUInt32(field++, pData->EventTime);

		if (!m_pMap->PutData(pData->iNum, pData))
			delete pData;

		return true;
	}

	RimaLotteryArray * m_pMap;
};