#pragma once

class CBurningFeatureSet : public OdbcRecordset
{
public:
	CBurningFeatureSet(OdbcConnection* dbConnection, void* dummy)
		: OdbcRecordset(dbConnection) {}

	virtual tstring GetTableName() { return _T("BURNING_FEATURES"); }
	virtual tstring GetColumns() { return _T("blevel, nprate, moneyrate, exprate, droprate"); }

	virtual bool Fetch() {
		int i = 1;

		uint8 blev = 0;
		_dbCommand->FetchByte(i++, blev);
		if (blev > 3)
			return false;

		blev -= 1;

		_dbCommand->FetchByte(i++, g_pMain->pBurningFea[blev].nprate);
		_dbCommand->FetchByte(i++, g_pMain->pBurningFea[blev].moneyrate);
		_dbCommand->FetchByte(i++, g_pMain->pBurningFea[blev].exprate);
		_dbCommand->FetchByte(i++, g_pMain->pBurningFea[blev].droprate);
		return true;
	}
};