#pragma once

class CRandomBossSet : public OdbcRecordset
{
public:
	CRandomBossSet(OdbcConnection * dbConnection, MonsterBossRandomSpawn*pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("MONSTER_BOSS_RANDOM_SPAWN"); }
	virtual tstring GetColumns() {return _T("nIndex,Stage, MonsterID, MonsterZone,PosX,PosZ,Range,ReloadTime");}

	virtual bool Fetch()
	{
		int i = 1;
		_MONSTER_BOSS_RANDOM_SPAWN *pData = new _MONSTER_BOSS_RANDOM_SPAWN;
		_dbCommand->FetchUInt16(i++, pData->nIndex);
		_dbCommand->FetchUInt16(i++, pData->Stage);
		_dbCommand->FetchUInt16(i++, pData->MonsterID);
		_dbCommand->FetchByte(i++, pData->MonsterZone);
		_dbCommand->FetchUInt16(i++, pData->PosX);
		_dbCommand->FetchUInt16(i++, pData->PosZ);
		_dbCommand->FetchUInt16(i++, pData->Range);
		_dbCommand->FetchUInt16(i++, pData->ReloadTime);
		if (!m_pMap->PutData(pData->nIndex, pData))
			delete pData;

		return true;
	}

	MonsterBossRandomSpawn* m_pMap;
};