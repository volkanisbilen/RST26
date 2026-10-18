#pragma once

class ClanPremiumItemSet : public OdbcRecordset
{
public:
	ClanPremiumItemSet(OdbcConnection * dbConnection, ClanPremiumItemArray * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("CLAN_PREMIUM_ITEM"); }
	virtual tstring GetColumns() { return _T("Type, ExpRestorePercent, NoahPercent, DropPercent, BonusLoyalty, RepairDiscountPercent, ItemSellPercent"); }

	virtual bool Fetch()
	{
		int i = 1;
		_CLAN_PREMIUM_ITEM * pData = new _CLAN_PREMIUM_ITEM;
		_dbCommand->FetchByte(i++, pData->Type);
		_dbCommand->FetchUInt16(i++, pData->ExpRestorePercent);
		_dbCommand->FetchUInt16(i++, pData->NoahPercent);
		_dbCommand->FetchUInt16(i++, pData->DropPercent);
		_dbCommand->FetchUInt32(i++, pData->BonusLoyalty);
		_dbCommand->FetchUInt16(i++, pData->RepairDiscountPercent);
		_dbCommand->FetchUInt16(i++, pData->ItemSellPercent);

		if (!m_pMap->PutData(pData->Type, pData))
			delete pData;

		return true;
	}

	ClanPremiumItemArray *m_pMap;
};