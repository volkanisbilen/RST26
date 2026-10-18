#pragma once

class CChaosStoneCoordinate : public OdbcRecordset
{
public:
	CChaosStoneCoordinate(OdbcConnection * dbConnection, ChaosStoneRespawnCoordinateArray *pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CHAOS_STONE_SPAWN2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CHAOS_STONE_SPAWN1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CHAOS_STONE_SPAWN1534"); }
#endif
	virtual tstring GetColumns() { return _T("sIndex, sZoneID,isOpen, sRank, sChaosID, sCount, sSpawnX, sSpawnZ, sSpawnTime, sDirection, sRadiusRange"); }

	virtual bool Fetch()
	{
		_CHAOS_STONE_RESPAWN *pData = new _CHAOS_STONE_RESPAWN;

		int i = 1;
		_dbCommand->FetchByte(i++, pData->sIndex);
		_dbCommand->FetchUInt16(i++, pData->sZoneID);
		_dbCommand->FetchByte(i++, pData->isOpen);
		_dbCommand->FetchByte(i++, pData->sRank);
		_dbCommand->FetchUInt16(i++, pData->sChaosID);
		_dbCommand->FetchUInt16(i++, pData->sCount);
		_dbCommand->FetchUInt16(i++, pData->sSpawnX);
		_dbCommand->FetchUInt16(i++, pData->sSpawnZ);
		_dbCommand->FetchByte(i++, pData->sSpawnTime);
		_dbCommand->FetchUInt16(i++, pData->sDirection);
		_dbCommand->FetchUInt16(i++, pData->sRadiusRange);

		if (!m_pMap->PutData(pData->sIndex, pData))
			delete pData;

		return true;
	}

	ChaosStoneRespawnCoordinateArray * m_pMap;
};