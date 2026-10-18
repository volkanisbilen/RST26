#pragma once

class xCapeCastellanBonus : public OdbcRecordset
{
public:
	xCapeCastellanBonus(OdbcConnection * dbConnection, CapeCastellanBonus * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("KNIGHTS_CAPE_CASTELLAN_BONUS"); }
	virtual tstring GetColumns() 
	{
		return _T("Type, ACBonus, HPBonus, MPBonus, StrengthBonus, StaminaBonus, DexterityBonus, IntelBonus, CharismaBonus, "
			"FlameResistance, GlacierResistance, LightningResistance, PoisonResistance, MagicResistance, CurseResistance, "
			"XPBonusPercent, CoinBonusPercent, APBonusPercent, APBonusClassType, APBonusClassPercent, ACBonusClassType, ACBonusClassPercent, "
			"MaxWeightBonus, NPBonus");
	}

	virtual bool Fetch()
	{
		int i = 1;
		_CAPE_CASTELLAN_BONUS *pData = new _CAPE_CASTELLAN_BONUS;
		_dbCommand->FetchUInt32(i++, pData->Type);
		_dbCommand->FetchUInt16(i++, pData->ACBonus);
		_dbCommand->FetchUInt16(i++, pData->HPBonus);
		_dbCommand->FetchUInt16(i++, pData->MPBonus);
		_dbCommand->FetchUInt16(i++, pData->StrengthBonus);
		_dbCommand->FetchUInt16(i++, pData->StaminaBonus);
		_dbCommand->FetchUInt16(i++, pData->DexterityBonus);
		_dbCommand->FetchUInt16(i++, pData->IntelBonus);
		_dbCommand->FetchUInt16(i++, pData->CharismaBonus);
		_dbCommand->FetchUInt16(i++, pData->FlameResistance);
		_dbCommand->FetchUInt16(i++, pData->GlacierResistance);
		_dbCommand->FetchUInt16(i++, pData->LightningResistance);
		_dbCommand->FetchUInt16(i++, pData->PoisonResistance);
		_dbCommand->FetchUInt16(i++, pData->MagicResistance);
		_dbCommand->FetchUInt16(i++, pData->CurseResistance);
		_dbCommand->FetchUInt16(i++, pData->XPBonusPercent);
		_dbCommand->FetchUInt16(i++, pData->CoinBonusPercent);
		_dbCommand->FetchUInt16(i++, pData->APBonusPercent);
		_dbCommand->FetchUInt16(i++, pData->APBonusClassType);
		_dbCommand->FetchUInt16(i++, pData->APBonusClassPercent);
		_dbCommand->FetchUInt16(i++, pData->ACBonusClassType);
		_dbCommand->FetchUInt16(i++, pData->ACBonusClassPercent);
		_dbCommand->FetchUInt16(i++, pData->MaxWeightBonus);
		_dbCommand->FetchByte(i++, pData->NPBonus);
		if (!m_pMap->PutData(pData->Type, pData))
			delete pData;

		return true;
	}

	CapeCastellanBonus *m_pMap;
};