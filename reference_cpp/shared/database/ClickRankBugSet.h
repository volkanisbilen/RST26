#pragma once
 
class CRankBugSet : public OdbcRecordset
{
public:
	CRankBugSet(OdbcConnection * dbConnection, void * pMap)
    : OdbcRecordset(dbConnection) {}
	
	virtual tstring GetTableName() { return _T("CLICK_RANK_BUG"); }
	virtual tstring GetColumns() { return _T("BorderJoin, ChaosJoin, JuraidJoin, CzRank, CrMinComp,CrMaxComp, LotteryJoin"); }
 
	virtual bool Fetch()
	{
		int field = 1;
		_dbCommand->FetchUInt32(field++, g_pMain->pRankBug.BorderJoin);
		_dbCommand->FetchUInt32(field++, g_pMain->pRankBug.ChaosJoin);
		_dbCommand->FetchUInt32(field++, g_pMain->pRankBug.JuraidJoin);
		_dbCommand->FetchUInt32(field++, g_pMain->pRankBug.CzRank);
		_dbCommand->FetchUInt32(field++, g_pMain->pRankBug.CrMinComp);
		_dbCommand->FetchUInt32(field++, g_pMain->pRankBug.CrMaxComp);
		_dbCommand->FetchUInt32(field++, g_pMain->pRankBug.LotteryJoin);
     return true;
   }
 };