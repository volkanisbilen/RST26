#pragma once

class CTopLeftSet : public OdbcRecordset
{
public:
	CTopLeftSet(OdbcConnection * dbConnection, TopLeftArray * pMap) : OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("TOPLEFT"); }
	virtual tstring GetColumns() { return _T("sIndex, Facebook, FacebookUrl, Discord, DiscordURL, Live, LiveURL, ResellerURL"); }

	virtual bool Fetch()
	{
		auto * pData = new _TOPLEFT();
		
		_dbCommand->FetchUInt32(1, pData->sIndex);
		
		_dbCommand->FetchByte(2, pData->Facebook);
		_dbCommand->FetchString(3, pData->FacebookURL);
		
		_dbCommand->FetchByte(4, pData->Discord);
		_dbCommand->FetchString(5, pData->DiscordURL);

		_dbCommand->FetchByte(6, pData->Live);
		_dbCommand->FetchString(7, pData->LiveURL);

		_dbCommand->FetchString(8, pData->ResellerURL);

		if (!m_pMap->PutData(pData->sIndex, pData))
		{
			delete pData;
			return false;
		}
		return true;
	}

	TopLeftArray * m_pMap;
};