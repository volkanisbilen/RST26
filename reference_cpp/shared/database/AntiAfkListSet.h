#pragma once

class CAntifAfkListSet : public OdbcRecordset
{
public:
	CAntifAfkListSet(OdbcConnection * dbConnection, uint16 *pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("ANTIAFKLIST2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("ANTIAFKLIST1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("ANTIAFKLIST1534"); }
#endif
	virtual tstring GetColumns() {return _T("NpcID");}
	virtual bool Fetch() {
		/*uint32 index; */uint16 NpcID; int field = 1;
		//_dbCommand->FetchUInt32(field++, index);
		_dbCommand->FetchUInt16(field++, NpcID);
		g_pMain->m_AntiAfkList.push_back(NpcID);
		return true;
	}
	uint16 * m_pMap;
};