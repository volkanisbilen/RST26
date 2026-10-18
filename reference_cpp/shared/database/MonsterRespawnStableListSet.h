#pragma once

class CMonsterRespawnStableListSet : public OdbcRecordset
{
public:
	CMonsterRespawnStableListSet(OdbcConnection* dbConnection, MonsterRespawnLoopArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("MONSTER_RESPAWNLOOP_LIST"); }
	virtual tstring GetColumns() { return _T("idead, iborn, count, deadtime"); }

	virtual bool Fetch()
	{
		int field = 1;
		_MONSTER_RESPAWNLIST* pData = new _MONSTER_RESPAWNLIST;
		_dbCommand->FetchUInt16(field++, pData->idead);
		_dbCommand->FetchUInt16(field++, pData->iborn);
		_dbCommand->FetchUInt16(field++, pData->count);
		_dbCommand->FetchUInt16(field++, pData->deadtime);
		if (!m_pMap->PutData(pData->idead, pData, false)) delete pData;
		return true;
	}
	MonsterRespawnLoopArray* m_pMap;
};
