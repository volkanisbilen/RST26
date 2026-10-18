#pragma once

class CMonsterItemSet : public OdbcRecordset
{
public:
	CMonsterItemSet(OdbcConnection * dbConnection, MonsterItemArray * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("K_MONSTER_ITEM2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("K_MONSTER_ITEM1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("K_MONSTER_ITEM1534"); }
#endif

	/*virtual tstring GetColumns() { return _T("sIndex, iItem01, sPersent01, iItem02, sPersent02, iItem03, sPersent03, iItem04,sPersent04,"
		"iItem05, sPersent05,iItem06, sPersent06, iItem07, sPersent07,iItem08, sPersent08,"
		"iItem09, sPersent09, iItem10, sPersent10, iItem11, sPersent11, iItem12, sPersent12"); }*/
	virtual tstring GetColumns() {
		return _T("sIndex, iItem01, sPersent01, iItem02, sPersent02, iItem03, sPersent03, iItem04,sPersent04,"
			"iItem05, sPersent05,iItem06, sPersent06, iItem07, sPersent07,iItem08, sPersent08,"
		"iItem09, sPersent09, iItem10, sPersent10, iItem11, sPersent11, iItem12, sPersent12");
	}

	virtual bool Fetch()
	{
		_K_MONSTER_ITEM *pData = new _K_MONSTER_ITEM;

		_dbCommand->FetchUInt16(1, pData->sIndex);

		for (int i = 0, j = 2; i < LOOT_DROP_ITEMS; i++)
		{
			_dbCommand->FetchUInt32(j++, pData->iItem[i]);
			_dbCommand->FetchUInt16(j++, pData->sPercent[i]);
		}

		if (!m_pMap->PutData(pData->sIndex, pData))
			delete pData;

		return true;
	}

	MonsterItemArray*m_pMap;
};