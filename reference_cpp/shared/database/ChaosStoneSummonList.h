#pragma once

class CChaosStoneSummonListSet : public OdbcRecordset
{
public:
	CChaosStoneSummonListSet(OdbcConnection * dbConnection, ChaosStoneSummonListArray * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("CHAOS_STONE_SUMMON_LIST2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("CHAOS_STONE_SUMMON_LIST1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("CHAOS_STONE_SUMMON_LIST1534"); }
#endif

	virtual tstring GetColumns() { return _T("nIndex, ZoneID, sSid, MonsterSpawnFamily"); }

	virtual bool Fetch()
	{
		_CHAOS_STONE_SUMMON_LIST * pData = new _CHAOS_STONE_SUMMON_LIST;

		_dbCommand->FetchUInt32(1, pData->nIndex);
		_dbCommand->FetchUInt16(2, pData->ZoneID);
		_dbCommand->FetchUInt16(3, pData->sSid);
		_dbCommand->FetchByte(4, pData->MonsterSpawnFamily);

		pData->nIndex = m_pMap->GetSize();

		if (!m_pMap->PutData(pData->nIndex, pData))
			delete pData;

		return true;
	}

	ChaosStoneSummonListArray *m_pMap;
};