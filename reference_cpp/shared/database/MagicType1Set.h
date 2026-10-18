#pragma once

class CMagicType1Set : public OdbcRecordset
{
public:
	CMagicType1Set(OdbcConnection * dbConnection, Magictype1Array * pMap) 
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("MAGIC_TYPE1"); }
	virtual tstring GetColumns() { return _T("iNum, Type, HitRate, Hit, AddDamage, ComboType, ComboCount, ComboDamage,Range, Delay, AddDmgPercToUser, AddDmgPercToNpc"); }

	virtual bool Fetch()
	{
		int field = 1;
		_MAGIC_TYPE1 *pData = new _MAGIC_TYPE1;
		_dbCommand->FetchUInt32(field++, pData->iNum);
		_dbCommand->FetchByte(field++, pData->bHitType);
		_dbCommand->FetchUInt16(field++, pData->sHitRate);
		_dbCommand->FetchUInt16(field++, pData->sHit);
		_dbCommand->FetchUInt16(field++, pData->sAddDamage);
		_dbCommand->FetchByte(field++, pData->bComboType);
		_dbCommand->FetchByte(field++, pData->bComboCount);
		_dbCommand->FetchUInt16(field++, pData->sComboDamage);
		_dbCommand->FetchUInt16(field++, pData->sRange);
		_dbCommand->FetchByte(field++, pData->bDelay);
		_dbCommand->FetchUInt16(field++, pData->iADPtoUser);
		_dbCommand->FetchUInt16(field++, pData->iADPtoNPC);

		if (!m_pMap->PutData(pData->iNum, pData))
			delete pData;

		return true;
	}

	Magictype1Array *m_pMap;
};