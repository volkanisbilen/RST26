#pragma once

class CSpecialStone : public OdbcRecordset
{
public:
	CSpecialStone(OdbcConnection* dbConnection, SpecialStonearray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("K_SPECIAL_STONE"); }
	virtual tstring GetColumns() { return _T("nIndex,ZoneID,MainNPC,SummonNPC,MonsterName,SummonCount"); }

	virtual bool Fetch()
	{
		SPECIAL_STONE* pData = new SPECIAL_STONE;

	

		if (m_pMap.insert(std::make_pair(uint32(0), pData)))
			delete pData;

		return true;
	}

	SpecialStonearray* m_pMap;
};