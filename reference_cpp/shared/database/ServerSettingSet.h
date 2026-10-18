#pragma once

class CServerSettingSet : public OdbcRecordset
{
public:
	CServerSettingSet(OdbcConnection* dbConnection, _SERVER_SETTING* pData)
		: OdbcRecordset(dbConnection), pMap(pData) {}

	virtual tstring GetTableName() { return _T("SERVER_SETTINGS"); }
	virtual tstring GetColumns() {
		return _T("MaximumLevelChange,DropNotice,UpgradeNotice,GmisOnlineNotice,OyunUserBotOnlineNotice,UserMaxUpgradeCount,AutoQuestSkill,MerchantView,ClanBankWithPremium,"
			"AutoRoyalG1,AutoWanted,LootandGeniePremium,MerchantMinKnightCash, trashitem, OnlineGiveCash, OnlineCashTime,"
			"flashtime,FreeSkillandStat,MerchantLevel,TradeLevel,chaoticcoins,mutelevel, new_monsterstone,monsterstone_status,"
			"etrafa_itemver1,etrafa_itemvercount1,etrafa_itemver2,etrafa_itemvercount2,etrafa_itemver3,etrafa_itemvercount3, maxplayerhp,"
			"WelcomeMsg, perkCoins, premiumID, premiumTime, maxBlessingUp,maxBlessingUpReb,giveGenieHour,AutoUpFasterKatlama,srcversion");
	}

	virtual bool Fetch() {
		int field = 1;
		_dbCommand->FetchByte(field++, pMap->MaximumLevelChange);
		_dbCommand->FetchByte(field++, pMap->DropNotice);
		_dbCommand->FetchByte(field++, pMap->UpgradeNotice);
		_dbCommand->FetchByte(field++, pMap->GmisOnlineNotice); // JstKO Gm Sadece | Tablo Server_Settings GmisOnlineNotice Eklendi 1 A�ip ve 0 Kapatma Ayarlar 30.12.2024
		_dbCommand->FetchByte(field++, pMap->OyunUserBotOnlineNotice); // JstKO User&Bot Oyunda Online | Tablo Server_Settings OyunUserBotOnlineNotice Eklendi 1 A�ip ve 0 Kapatma Ayarlar 31.12.2024
		_dbCommand->FetchByte(field++, pMap->UserMaxUpgradeCount);
		_dbCommand->FetchByte(field++, pMap->AutoQuestSkill);
		_dbCommand->FetchByte(field++, pMap->MerchantView);
		_dbCommand->FetchByte(field++, pMap->ClanBankWithPremium); //Clan Bankas� Premiumlu veya Premiumsuz Acilsin
		_dbCommand->FetchByte(field++, pMap->AutoRoyalG1); 
		_dbCommand->FetchByte(field++, pMap->AutoWanted);
		_dbCommand->FetchByte(field++, pMap->LootandGeniePremium);
		_dbCommand->FetchUInt16(field++, pMap->MinKnightCash);
		_dbCommand->Fetchtbool(field++, pMap->trashitem);
		_dbCommand->Fetchtbool(field++, pMap->onlinecash);
		_dbCommand->FetchUInt32(field++, pMap->onlinecashtime);
		_dbCommand->FetchUInt32(field++, pMap->flashtime);
		_dbCommand->Fetchtbool(field++, pMap->FreeSkillandStat);
		_dbCommand->FetchByte(field++, pMap->MerchantLevel);
		_dbCommand->FetchByte(field++, pMap->TradeLevel);
		_dbCommand->FetchUInt32(field++, pMap->chaoticcoins);
		_dbCommand->FetchByte(field++, pMap->mutelevel);
		_dbCommand->FetchByte(field++, pMap->new_monsterstone);
		_dbCommand->Fetchtbool(field++, pMap->monsterstone_status);

		for (int i = 0; i < 3; i++) {
			_dbCommand->FetchUInt32(field++, pMap->etrafa_itemverid[i]);
			_dbCommand->FetchUInt32(field++, pMap->etrafa_itemvercount[i]);
		}

		_dbCommand->FetchInt16(field++, pMap->maxplayerhp);
		_dbCommand->FetchString(field++, pMap->WelcomeMsg);
		_dbCommand->FetchUInt32(field++, pMap->m_perkCoins);
		_dbCommand->FetchUInt16(field++, pMap->premiumID);
		_dbCommand->FetchUInt16(field++, pMap->premiumTime);
		_dbCommand->FetchByte(field++, pMap->maxBlessingUp);
		_dbCommand->FetchByte(field++, pMap->maxBlessingUpReb);
		_dbCommand->FetchByte(field++, pMap->giveGenieHour);
		_dbCommand->FetchByte(field++, pMap->AutoUpFasterKatlama);
		_dbCommand->FetchString(field++, pMap->srcversion);
		if (pMap->maxplayerhp < 100)
			pMap->maxplayerhp = 100;

		if (pMap->maxplayerhp > 32500)
			pMap->maxplayerhp = 32500;

		return true;
	}
	_SERVER_SETTING* pMap;
};

#pragma once