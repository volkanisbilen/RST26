#pragma once

class CUserLootSettingsSet : public OdbcRecordset
{
public:
	CUserLootSettingsSet(OdbcConnection * dbConnection, UserLootSettingsArray * pMap) : OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("USER_LOOT_SETTINGS"); }
	virtual tstring GetColumns() { return _T("UserID, iWarrior, iRogue, iMage, iPriest, iWeapon, iArmor, iAccessory, iNormal, iUpgrade, iCraft, iRare, iMagic, iUnique, iConsumable, iPrice"); }

	virtual bool Fetch()
	{
		uint32_t i = 1;

		auto * pData = new _LOOT_SETTINGS;

		_dbCommand->FetchString(i++, pData->UserName);
		_dbCommand->FetchByte(i++, pData->iWarrior);
		_dbCommand->FetchByte(i++, pData->iRogue);
		_dbCommand->FetchByte(i++, pData->iMage);
		_dbCommand->FetchByte(i++, pData->iPriest);
		_dbCommand->FetchByte(i++, pData->iWeapon);
		_dbCommand->FetchByte(i++, pData->iArmor);
		_dbCommand->FetchByte(i++, pData->iAccessory);
		_dbCommand->FetchByte(i++, pData->iUpgrade);
		_dbCommand->FetchByte(i++, pData->iCraft);
		_dbCommand->FetchByte(i++, pData->iRare);
		_dbCommand->FetchByte(i++, pData->iMagic);
		_dbCommand->FetchByte(i++, pData->iUnique);
		_dbCommand->FetchByte(i++, pData->iConsumable);
		_dbCommand->FetchUInt32(i++, pData->iPrice);

		if (!m_pMap->insert(std::make_pair(pData->UserName, pData)).second)
			delete pData;

		return true;
	}

	UserLootSettingsArray * m_pMap;
};