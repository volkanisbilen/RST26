#pragma once
#include "LoadServerData.h"

class CCoefficientDamageSet : public OdbcRecordset
{
public:
	CCoefficientDamageSet(OdbcConnection* dbConnection, CoefficientDamageArray* pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("COEFFICIENT_DAMAGE"); }
	virtual tstring GetColumns() { return _T("sClass, sRogue, sWarrior, sPriest, sMage, sKurian"); }

	virtual bool Fetch()
	{
		_CLASS_DAMAGE* pData = new _CLASS_DAMAGE;

		_dbCommand->FetchUInt16(1, pData->sClassNum);
		_dbCommand->FetchSingle(2, pData->sRogue);
		_dbCommand->FetchSingle(3, pData->sWarrior);
		_dbCommand->FetchSingle(4, pData->sPriest);
		_dbCommand->FetchSingle(5, pData->sMage);
		_dbCommand->FetchSingle(6, pData->sKurian);

		if (!m_pMap->PutData(pData->sClassNum, pData))
			delete pData;

		return true;
	}

	CoefficientDamageArray* m_pMap;
};