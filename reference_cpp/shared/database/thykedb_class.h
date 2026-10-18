#pragma once

class CLevelRewardsSet : public OdbcRecordset
{
public:
	CLevelRewardsSet(OdbcConnection* dbConnection, LevelRewardArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
	virtual tstring GetTableName() { return _T("LEVEL_REWARDS"); }
	virtual tstring GetColumns() { return _T("bLevel, bNotice, cash, tl,loyalty, money, "
		"itemid1,itemcount1,itemtime1,"
		"itemid2,itemcount2,itemtime2,"
		"itemid3,itemcount3,itemtime3"); }
	virtual bool Fetch() {
		int field = 1;

		uint8 bLevel = 0;
		_LEVEL_REWARDS* pData = new _LEVEL_REWARDS;
		_dbCommand->FetchByte(field++, bLevel);
		_dbCommand->FetchString(field++, pData->bNotice);
		_dbCommand->FetchUInt32(field++, pData->cash);
		_dbCommand->FetchUInt32(field++, pData->tl);
		_dbCommand->FetchUInt32(field++, pData->loyalty);
		_dbCommand->FetchUInt32(field++, pData->money);
		for (int i = 0; i < 3; i++)
		{
			_dbCommand->FetchUInt32(field++, pData->itemid[i]);
			_dbCommand->FetchUInt32(field++, pData->itemcount[i]);
			_dbCommand->FetchUInt32(field++, pData->itemtime[i]);
		}
		pData->isnull = false;
		if (!m_pMap->PutData(bLevel, pData)) {
			delete pData; return false;
		}
		return true;
	}
	LevelRewardArray* m_pMap;
};

class CLevelMerchantRewardsSet : public OdbcRecordset
{
public:
	CLevelMerchantRewardsSet(OdbcConnection* dbConnection, LevelMerchantRewardArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
	virtual tstring GetTableName() { return _T("LEVEL_MERCHANT_REWARDS"); }
	virtual tstring GetColumns() {
		return _T("sIndex, startHour, startMinute,finishtime, rate_experience,exp_minute");
	}
	virtual bool Fetch() {
		int field = 1;
		_LEVEL_MERCHANT_REWARDS* pData = new _LEVEL_MERCHANT_REWARDS;
		_dbCommand->FetchUInt16(field++, pData->index);
		_dbCommand->FetchUInt32(field++, pData->startHour);
		_dbCommand->FetchUInt32(field++, pData->startMinute);
		_dbCommand->FetchUInt32(field++, pData->finih_minute);
		_dbCommand->FetchByte(field++, pData->rate_experience);
		_dbCommand->FetchUInt32(field++, pData->exp_minute);
		if (!m_pMap->PutData(pData->index, pData)) {
			delete pData; return false;
		}
		return true;
	}
	LevelMerchantRewardArray* m_pMap;
};

class CJackPotSettingSet : public OdbcRecordset
{
public:
	CJackPotSettingSet(OdbcConnection* dbConnection, void* dummy)
		: OdbcRecordset(dbConnection) {}

	virtual tstring GetTableName() { return _T("JACKPOT_SETTINGS"); }
	virtual tstring GetColumns() {
		return _T("iType, rate,_1000,_500,_100,_50,_10,_2");
	}

	virtual bool Fetch() {
		int field = 1; uint8 iType = 0;
		_dbCommand->FetchByte(field++, iType);
		if (iType > 1) return false;
		_dbCommand->FetchUInt16(field++, g_pMain->pJackPot[iType].rate);
		_dbCommand->FetchUInt16(field++, g_pMain->pJackPot[iType]._1000);
		_dbCommand->FetchUInt16(field++, g_pMain->pJackPot[iType]._500);
		_dbCommand->FetchUInt16(field++, g_pMain->pJackPot[iType]._100);
		_dbCommand->FetchUInt16(field++, g_pMain->pJackPot[iType]._50);
		_dbCommand->FetchUInt16(field++, g_pMain->pJackPot[iType]._10);
		_dbCommand->FetchUInt16(field++, g_pMain->pJackPot[iType]._2);
		return true;
	}
};

class CPerksSet : public OdbcRecordset
{
public:
	CPerksSet(OdbcConnection* dbConnection, PerksArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("PERKS"); }
	virtual tstring GetColumns() { return _T("pIndex,status,strDescp, perkCount,perkMaxCount,percentage"); }

	virtual bool Fetch() {
		int field = 1;
		_PERKS* pData = new _PERKS;
		_dbCommand->FetchUInt32(field++, pData->pIndex);
		_dbCommand->Fetchtbool(field++, pData->status);
		_dbCommand->FetchString(field++, pData->strDescp);
		_dbCommand->FetchUInt16(field++, pData->perkCount);
		_dbCommand->FetchUInt16(field++, pData->maxPerkCount);
		_dbCommand->Fetchtbool(field++, pData->percentage);
		if (!m_pMap->PutData(pData->pIndex, pData)) {
			delete pData;
			return false;
		}
		return true;
	}
	PerksArray* m_pMap;
};

#pragma once

class CForgettenTMonsList : public OdbcRecordset
{
public:
	CForgettenTMonsList(OdbcConnection* dbConnection, ForgettenTempleMonsterArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("FT_SUMMON_LIST"); }
	virtual tstring GetColumns() { return _T("bIndex, Type, Stage, SidID, SidCount, PosX, PosZ, Range"); }

	virtual bool Fetch()
	{
		int field = 1;
		_FORGETTEN_TEMPLE_SUMMON* pData = new _FORGETTEN_TEMPLE_SUMMON;
		_dbCommand->FetchUInt32(field++, pData->bIndex);
		_dbCommand->FetchByte(field++, pData->Type);
		_dbCommand->FetchByte(field++, pData->Stage);
		_dbCommand->FetchUInt16(field++, pData->SidID);
		_dbCommand->FetchUInt16(field++, pData->SidCount);
		_dbCommand->FetchUInt16(field++, pData->PosX);
		_dbCommand->FetchUInt16(field++, pData->PosZ);
		_dbCommand->FetchUInt16(field++, pData->Range);
		if (!m_pMap->PutData(pData->bIndex, pData))
			delete pData;

		return true;
	}
	ForgettenTempleMonsterArray* m_pMap;
};

#pragma once

class CForgettenStagesList : public OdbcRecordset
{
public:
	CForgettenStagesList(OdbcConnection* dbConnection, ForgettenTempleStagesArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("FT_STAGES"); }
	virtual tstring GetColumns() { return _T("nIndex, Type, Stage, Time"); }

	virtual bool Fetch()
	{
		int field = 1;
		_FORGETTEN_TEMPLE_STAGES* pData = new _FORGETTEN_TEMPLE_STAGES;
		_dbCommand->FetchInt32(field++, pData->nIndex);
		_dbCommand->FetchByte(field++, pData->Type);
		_dbCommand->FetchByte(field++, pData->Stage);
		_dbCommand->FetchUInt16(field++, pData->Time);
		if (!m_pMap->PutData(pData->nIndex, pData))
			delete pData;

		return true;
	}
	ForgettenTempleStagesArray* m_pMap;
};

#pragma once

class CFtEventTimeSet : public OdbcRecordset
{
public:
	CFtEventTimeSet(OdbcConnection* dbConnection, XFTTIMEOPT* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("EVENT_OPT_FT"); }
	virtual tstring GetColumns() { return _T("PlayingTime, SummonTime, SpawnMinTime,WaitingTime,MinLevel,MaxLevel"); }

	virtual bool Fetch() {
		int field = 1;
		_dbCommand->FetchUInt16(field++, m_pMap->PlayingTime);
		_dbCommand->FetchUInt16(field++, m_pMap->SummonTime);
		_dbCommand->FetchUInt16(field++, m_pMap->SpawnMinTime);
		_dbCommand->FetchUInt16(field++, m_pMap->WaitingTime);
		_dbCommand->FetchByte(field++, m_pMap->MinLevel);
		_dbCommand->FetchByte(field++, m_pMap->MaxLevel);
		return true;
	}
	XFTTIMEOPT* m_pMap;
};

class CAutomaticCommandSet : public OdbcRecordset
{
public:
	CAutomaticCommandSet(OdbcConnection* dbConnection, AutomaticCommandArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("AUTOMATIC_COMMAND2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("AUTOMATIC_COMMAND1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("AUTOMATIC_COMMAND1534"); }
#endif
	virtual tstring GetColumns() { return _T("sIndex, command, hour,minute,iDay"); }

	virtual bool Fetch()
	{
		int field = 1;
		_AUTOMATIC_COMMAND* pData = new _AUTOMATIC_COMMAND;
		_dbCommand->FetchUInt32(field++, pData->sIndex);
		_dbCommand->FetchString(field++, pData->command);
		_dbCommand->FetchUInt32(field++, pData->hour);
		_dbCommand->FetchUInt32(field++, pData->minute);
		_dbCommand->FetchUInt32(field++, pData->iDay);
		if (!m_pMap->PutData(pData->sIndex, pData))
			delete pData;

		return true;
	}
	AutomaticCommandArray* m_pMap;
};


class CRankRewardSet : public OdbcRecordset
{
public:
	CRankRewardSet(OdbcConnection* dbConnection, RankRewardArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("RANK_REWARDS"); }
	virtual tstring GetColumns() { return _T("RankNumber, nItemID, sCount"); }

	virtual bool Fetch() {
		int field = 1;
		_RANK_REWARD* pData = new _RANK_REWARD;
		_dbCommand->FetchByte(field++, pData->RankNumber);
		_dbCommand->FetchUInt32(field++, pData->nItemID);
		_dbCommand->FetchUInt16(field++, pData->sCount);
		if (!m_pMap->PutData(pData->RankNumber, pData)) {
			delete pData;
			return false;
		}
		return true;
	}
	RankRewardArray* m_pMap;
};