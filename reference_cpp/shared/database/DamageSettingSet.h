#pragma once

class CDamageSettingSet : public OdbcRecordset
{
public:
	CDamageSettingSet(OdbcConnection* dbConnection, _DAMAGE_SETTING* pData)
		: OdbcRecordset(dbConnection), pDamageData(pData) {}

	virtual tstring GetTableName() { return _T("DAMAGE_SETTINGS"); }
	virtual tstring GetColumns() { 
		return _T( "[priest-warriror], [priest-mage],[priest-priest],[priest-rogue],[priest-kurian],"
			"[warriror-rogue], [warriror-mage],[warriror-warrior],[warriror-priest],[warriror-kurian],"
			"[rogue-mage], [rogue-warrior], [rogue-rogue],[rogue-priest],[rogue-kurian],"
			"[kurian-mage],[kurian-warrior],[kurian-rogue],[kurian-priest],[kurian-kurian],"
			"[mage-warriror], [mage-mage], [mage-priest],[mage-rogue],[mage-kurian],"
			"mondef,montakedamage,magemagicdamage,uniqueitem,lowclassitem,middleclassitem,highclassitem,"
			"rareitem,magicitem, rdamage, kurianzehir"); 
	}

	virtual bool Fetch() {
		int field = 1;
		_dbCommand->FetchSingle(field++, pDamageData->priestTOwarriror);
		_dbCommand->FetchSingle(field++, pDamageData->priestTOmage);
		_dbCommand->FetchSingle(field++, pDamageData->priestTOpriest);
		_dbCommand->FetchSingle(field++, pDamageData->priestTOrogue);
		_dbCommand->FetchSingle(field++, pDamageData->priestTOkurian);

		_dbCommand->FetchSingle(field++, pDamageData->warrirorTOrogue);
		_dbCommand->FetchSingle(field++, pDamageData->warrirorTOmage);
		_dbCommand->FetchSingle(field++, pDamageData->warrirorTOwarrior);
		_dbCommand->FetchSingle(field++, pDamageData->warrirorTOpriest);
		_dbCommand->FetchSingle(field++, pDamageData->warrirorTOkurian);

		_dbCommand->FetchSingle(field++, pDamageData->rogueTOmage);
		_dbCommand->FetchSingle(field++, pDamageData->rogueTOwarrior);
		_dbCommand->FetchSingle(field++, pDamageData->rogueTOrogue);
		_dbCommand->FetchSingle(field++, pDamageData->rogueTOpriest);
		_dbCommand->FetchSingle(field++, pDamageData->rogueTOkurian);

		_dbCommand->FetchSingle(field++, pDamageData->kurianTOmage);
		_dbCommand->FetchSingle(field++, pDamageData->kurianTOwarrior);
		_dbCommand->FetchSingle(field++, pDamageData->kurianTOrogue);
		_dbCommand->FetchSingle(field++, pDamageData->kurianTOpriest);
		_dbCommand->FetchSingle(field++, pDamageData->kurianTOkurian);

		_dbCommand->FetchSingle(field++, pDamageData->mageTOwarriror);
		_dbCommand->FetchSingle(field++, pDamageData->mageTOmage);
		_dbCommand->FetchSingle(field++, pDamageData->mageTOpriest);
		_dbCommand->FetchSingle(field++, pDamageData->mageTOrogue);
		_dbCommand->FetchSingle(field++, pDamageData->mageTOkurian);

		_dbCommand->FetchSingle(field++, pDamageData->mondef);
		_dbCommand->FetchSingle(field++, pDamageData->montakedamage);
		_dbCommand->FetchSingle(field++, pDamageData->magemagicdamage);

		_dbCommand->FetchSingle(field++, pDamageData->uniqueitem);
		_dbCommand->FetchSingle(field++, pDamageData->lowclassitem);
		_dbCommand->FetchSingle(field++, pDamageData->middleclassitem);
		_dbCommand->FetchSingle(field++, pDamageData->highclassitem);
		_dbCommand->FetchSingle(field++, pDamageData->rareitem);
		_dbCommand->FetchSingle(field++, pDamageData->magicitem);
		_dbCommand->FetchSingle(field++, pDamageData->rdamage);
		_dbCommand->FetchSingle(field++, pDamageData->kurianzehir);
		return true;
	}
	_DAMAGE_SETTING* pDamageData;
};