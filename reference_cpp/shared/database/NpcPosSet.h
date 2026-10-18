#pragma once

class CNpcPosSet : public OdbcRecordset
{
public:
	CNpcPosSet(OdbcConnection* dbConnection, void* dummy)
		: OdbcRecordset(dbConnection) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("K_NPCPOS2369"); }
#elif GAME_SOURCE_VERSION  == 1098
virtual tstring GetTableName() { return _T("K_NPCPOS1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("K_NPCPOS1534"); }
#endif
	virtual tstring GetColumns() { return _T("ZoneID, NpcID, mon_summon, mon_savepos, ActType, RegenType, DungeonFamily, SpecialType, TrapNumber, LeftX, TopZ, NumNPC, RegTime, byDirection, DotCnt, path, Room,iRange"); }
	virtual bool Fetch() { return g_pMain->_LoaderSpawnCallBack(_dbCommand); }
};