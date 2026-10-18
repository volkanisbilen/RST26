#pragma once

class CRandomBossStageSet : public OdbcRecordset
{
public:
	CRandomBossStageSet(OdbcConnection * dbConnection, MonsterBossRandomStage*pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("MONSTER_BOSS_RANDOM_STAGES"); }
	virtual tstring GetColumns() {return _T("Stage, MonsterID, MonsterZone");}

	virtual bool Fetch()
	{
		int i = 1;
		_MONSTER_BOSS_RANDOM_STAGES *pData = new _MONSTER_BOSS_RANDOM_STAGES;
		_dbCommand->FetchUInt16(i++, pData->Stage);
		_dbCommand->FetchUInt16(i++, pData->MonsterID);
		_dbCommand->FetchByte(i++, pData->MonsterZone);
		if (!m_pMap->PutData(pData->Stage, pData))
			delete pData;

		return true;
	}
	MonsterBossRandomStage* m_pMap;
};