#pragma once

class CZoneKillReward : public OdbcRecordset
{
public:
	CZoneKillReward(OdbcConnection * dbConnection, ZoneKillReward *pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("ZONE_KILL_REWARD"); }
	virtual tstring GetColumns() 
	{
		return _T("ZoneID,Nation,Party,AllPartyReward,ItemID,ItemDuration,ItemCount,ItemFlag,ItemExpirationTime,DropRate,GivedItemWarehouse,Status,nIndex,KillCount,isPriest,PriestRate");
	}


	virtual bool Fetch()
	{
		auto pData = new _ZONE_KILL_REWARD;

		auto i = 1;
		_dbCommand->FetchByte(i++, pData->ZoneID);
		_dbCommand->FetchByte(i++, pData->Nation);
		_dbCommand->FetchByte(i++, pData->Party);
		_dbCommand->Fetchtbool(i++, pData->isPartyItem);
		_dbCommand->FetchUInt32(i++, pData->ItemID);
		_dbCommand->FetchUInt16(i++, pData->Duration);
		_dbCommand->FetchUInt32(i++, pData->sCount);
		_dbCommand->FetchByte(i++, pData->sFlag);
		_dbCommand->FetchUInt32(i++, pData->Time);
		_dbCommand->FetchUInt16(i++, pData->sRate);
		_dbCommand->FetchByte(i++, pData->isBank);
		_dbCommand->FetchByte(i++, pData->Status);
		_dbCommand->FetchUInt32(i++, pData->ID);
		_dbCommand->FetchByte(i++, pData->KillCount);
		_dbCommand->Fetchtbool(i++, pData->isPriest);
		_dbCommand->FetchByte(i++, pData->priestrate);
		
		if (pData->priestrate > 100)
			pData->priestrate = 100;
		
		g_pMain->m_ZoneKillReward.insert(std::make_pair(pData->ID, pData));
		return true;
	}

	ZoneKillReward * m_pMap;
};