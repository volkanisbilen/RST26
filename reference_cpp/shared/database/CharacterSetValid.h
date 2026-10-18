#pragma once

class CCharacterSetValid : public OdbcRecordset
{
public:
	CCharacterSetValid(OdbcConnection* dbConnection, CharacterSetValidArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("CHARACTER_SET_VALIDS"); }
	virtual tstring GetColumns()
	{
		return _T("ClassIndex, NationValue, RaceValue, ClassValue, ReStatStr,ReStatSta,ReStatDex,ReStatInt,ReStatCha");
	}

	virtual bool Fetch()
	{
		int i = 1;
		_CHARACTER_SETVALID pData;
		_dbCommand->FetchByte(i++, pData.ClassIndex);
		_dbCommand->FetchByte(i++, pData.NationValue);
		_dbCommand->FetchByte(i++, pData.RaceValue);
		_dbCommand->FetchUInt16(i++, pData.ClassValue);
		for (int x = 0; x < (int)StatType::STAT_COUNT; x++) _dbCommand->FetchByte(i++, pData.ReStat[x]);
		m_pMap->push_back(pData);
		return true;
	}
	CharacterSetValidArray* m_pMap;
};