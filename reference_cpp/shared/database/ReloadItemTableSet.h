#pragma once
class CReloadItemTableSet : public OdbcRecordset
{
public:
	CReloadItemTableSet(OdbcConnection * dbConnection, ItemtableArray * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("ITEM"); }
	virtual tstring GetColumns() {
		return _T("Num, Extension, strName, Description, Kind, Slot, Race, Class, Damage,"
		"minDamage,maxDamage,"
		"Delay, Range, Weight, Duration, BuyPrice, SellPrice,SellNpcType,SellNpcPrice, Ac, Countable, Effect1, Effect2, ReqLevel, ReqLevelMax, ReqRank, ReqTitle, ReqStr, ReqSta, ReqDex, ReqIntel, ReqCha, SellingGroup, ItemType, Hitrate, Evasionrate, DaggerAc, JamadarAc, SwordAc, ClubAc, AxeAc, SpearAc, BowAc, FireDamage, IceDamage, LightningDamage, PoisonDamage, HPDrain, MPDamage, MPDrain, MirrorDamage, Droprate ,StrB, StaB, DexB, IntelB, ChaB, MaxHpB, MaxMpB, FireR, ColdR, LightningR, MagicR, PoisonR, CurseR, ItemClass, NPbuyPrice, Bound,byGrade,DropNotice,UpgradeNotice");
	}

	virtual bool Fetch()
	{
		int i = 1;
		uint32 myIndex = -1;
		_dbCommand->FetchUInt32(i,myIndex);
		_ITEM_TABLE* pData = m_pMap->GetData(myIndex);

		if (pData)
		{
			_dbCommand->FetchUInt32(i++, pData->m_iNum);
			_dbCommand->FetchInt16(i++, pData->Extension);
			_dbCommand->FetchString(i++, pData->m_sName);
			_dbCommand->FetchString(i++, pData->Description);
			//_dbCommand->FetchUInt32(i++, pData->ItemIconID1);
			//_dbCommand->FetchUInt32(i++, pData->ItemIconID2);
			_dbCommand->FetchByte(i++, pData->m_bKind);
			_dbCommand->FetchByte(i++, pData->m_bSlot);
			_dbCommand->FetchByte(i++, pData->m_bRace);
			_dbCommand->FetchByte(i++, pData->m_bClass);
			_dbCommand->FetchUInt16(i++, pData->m_sDamage);
			_dbCommand->FetchUInt16(i++, pData->minDamage);
			_dbCommand->FetchUInt16(i++, pData->maxDamage);
			_dbCommand->FetchUInt16(i++, pData->m_sDelay);
			_dbCommand->FetchUInt16(i++, pData->m_sRange);
			_dbCommand->FetchUInt16(i++, pData->m_sWeight);
			_dbCommand->FetchUInt16(i++, pData->m_sDuration);
			_dbCommand->FetchUInt32(i++, pData->m_iBuyPrice);
			_dbCommand->FetchUInt32(i++, pData->m_iSellPrice);
			_dbCommand->FetchByte(i++, pData->m_iSellNpcType);
			_dbCommand->FetchUInt32(i++, pData->m_iSellNpcPrice);
			_dbCommand->FetchInt16(i++, pData->m_sAc);
			_dbCommand->FetchByte(i++, pData->m_bCountable);
			_dbCommand->FetchUInt32(i++, pData->m_iEffect1);
			_dbCommand->FetchUInt32(i++, pData->m_iEffect2);
			_dbCommand->FetchByte(i++, pData->m_bReqLevel);
			_dbCommand->FetchByte(i++, pData->m_bReqLevelMax);
			_dbCommand->FetchByte(i++, pData->m_bReqRank);
			_dbCommand->FetchByte(i++, pData->m_bReqTitle);
			_dbCommand->FetchByte(i++, pData->m_bReqStr);
			_dbCommand->FetchByte(i++, pData->m_bReqSta);
			_dbCommand->FetchByte(i++, pData->m_bReqDex);
			_dbCommand->FetchByte(i++, pData->m_bReqIntel);
			_dbCommand->FetchByte(i++, pData->m_bReqCha);
			_dbCommand->FetchUInt16(i++, pData->m_bSellingGroup);
			_dbCommand->FetchByte(i++, pData->m_ItemType);
			_dbCommand->FetchUInt16(i++, pData->m_sHitrate);
			_dbCommand->FetchUInt16(i++, pData->m_sEvarate);
			_dbCommand->FetchUInt16(i++, pData->m_sDaggerAc);
			_dbCommand->FetchUInt16(i++, pData->m_JamadarAc);
			_dbCommand->FetchUInt16(i++, pData->m_sSwordAc);
			_dbCommand->FetchUInt16(i++, pData->m_sClubAc);
			_dbCommand->FetchUInt16(i++, pData->m_sAxeAc);
			_dbCommand->FetchUInt16(i++, pData->m_sSpearAc);
			_dbCommand->FetchUInt16(i++, pData->m_sBowAc);
			_dbCommand->FetchByte(i++, pData->m_bFireDamage);
			_dbCommand->FetchByte(i++, pData->m_bIceDamage);
			_dbCommand->FetchByte(i++, pData->m_bLightningDamage);
			_dbCommand->FetchByte(i++, pData->m_bPoisonDamage);
			_dbCommand->FetchByte(i++, pData->m_bHPDrain);
			_dbCommand->FetchByte(i++, pData->m_bMPDamage);
			_dbCommand->FetchByte(i++, pData->m_bMPDrain);
			_dbCommand->FetchByte(i++, pData->m_bMirrorDamage);
			_dbCommand->FetchByte(i++, pData->m_DropRate);
			_dbCommand->FetchInt16(i++, pData->m_sStrB);
			_dbCommand->FetchInt16(i++, pData->m_sStaB);
			_dbCommand->FetchInt16(i++, pData->m_sDexB);
			_dbCommand->FetchInt16(i++, pData->m_sIntelB);
			_dbCommand->FetchInt16(i++, pData->m_sChaB);
			_dbCommand->FetchInt16(i++, pData->m_MaxHpB);
			_dbCommand->FetchInt16(i++, pData->m_MaxMpB);
			_dbCommand->FetchInt16(i++, pData->m_bFireR);
			_dbCommand->FetchInt16(i++, pData->m_bColdR);
			_dbCommand->FetchInt16(i++, pData->m_bLightningR);
			_dbCommand->FetchInt16(i++, pData->m_bMagicR);
			_dbCommand->FetchInt16(i++, pData->m_bPoisonR);
			_dbCommand->FetchInt16(i++, pData->m_bCurseR);
			_dbCommand->FetchInt16(i++, pData->ItemClass);
			_dbCommand->FetchUInt32(i++, pData->m_iNPBuyPrice);
			_dbCommand->FetchInt16(i++, pData->m_Bound);
			_dbCommand->FetchByte(i++, pData->m_byGrade);
			_dbCommand->FetchByte(i++, pData->m_isDropNotice);
			_dbCommand->FetchByte(i++, pData->m_isUpgradeNotice);
		}
		else
		{
			_ITEM_TABLE *pData = new _ITEM_TABLE;

			_dbCommand->FetchUInt32(i++, pData->m_iNum);
			_dbCommand->FetchInt16(i++, pData->Extension);
			_dbCommand->FetchString(i++, pData->m_sName);
			_dbCommand->FetchString(i++, pData->Description);
			//_dbCommand->FetchUInt32(i++, pData->ItemIconID1);
			//_dbCommand->FetchUInt32(i++, pData->ItemIconID2);
			_dbCommand->FetchByte(i++, pData->m_bKind);
			_dbCommand->FetchByte(i++, pData->m_bSlot);
			_dbCommand->FetchByte(i++, pData->m_bRace);
			_dbCommand->FetchByte(i++, pData->m_bClass);
			_dbCommand->FetchUInt16(i++, pData->m_sDamage);
			_dbCommand->FetchUInt16(i++, pData->minDamage);
			_dbCommand->FetchUInt16(i++, pData->maxDamage);
			_dbCommand->FetchUInt16(i++, pData->m_sDelay);
			_dbCommand->FetchUInt16(i++, pData->m_sRange);
			_dbCommand->FetchUInt16(i++, pData->m_sWeight);
			_dbCommand->FetchUInt16(i++, pData->m_sDuration);
			_dbCommand->FetchUInt32(i++, pData->m_iBuyPrice);
			_dbCommand->FetchUInt32(i++, pData->m_iSellPrice);
			_dbCommand->FetchByte(i++, pData->m_iSellNpcType);
			_dbCommand->FetchUInt32(i++, pData->m_iSellNpcPrice);
			_dbCommand->FetchInt16(i++, pData->m_sAc);
			_dbCommand->FetchByte(i++, pData->m_bCountable);
			_dbCommand->FetchUInt32(i++, pData->m_iEffect1);
			_dbCommand->FetchUInt32(i++, pData->m_iEffect2);
			_dbCommand->FetchByte(i++, pData->m_bReqLevel);
			_dbCommand->FetchByte(i++, pData->m_bReqLevelMax);
			_dbCommand->FetchByte(i++, pData->m_bReqRank);
			_dbCommand->FetchByte(i++, pData->m_bReqTitle);
			_dbCommand->FetchByte(i++, pData->m_bReqStr);
			_dbCommand->FetchByte(i++, pData->m_bReqSta);
			_dbCommand->FetchByte(i++, pData->m_bReqDex);
			_dbCommand->FetchByte(i++, pData->m_bReqIntel);
			_dbCommand->FetchByte(i++, pData->m_bReqCha);
			_dbCommand->FetchUInt16(i++, pData->m_bSellingGroup);
			_dbCommand->FetchByte(i++, pData->m_ItemType);
			_dbCommand->FetchUInt16(i++, pData->m_sHitrate);
			_dbCommand->FetchUInt16(i++, pData->m_sEvarate);
			_dbCommand->FetchUInt16(i++, pData->m_sDaggerAc);
			_dbCommand->FetchUInt16(i++, pData->m_JamadarAc);
			_dbCommand->FetchUInt16(i++, pData->m_sSwordAc);
			_dbCommand->FetchUInt16(i++, pData->m_sClubAc);
			_dbCommand->FetchUInt16(i++, pData->m_sAxeAc);
			_dbCommand->FetchUInt16(i++, pData->m_sSpearAc);
			_dbCommand->FetchUInt16(i++, pData->m_sBowAc);
			_dbCommand->FetchByte(i++, pData->m_bFireDamage);
			_dbCommand->FetchByte(i++, pData->m_bIceDamage);
			_dbCommand->FetchByte(i++, pData->m_bLightningDamage);
			_dbCommand->FetchByte(i++, pData->m_bPoisonDamage);
			_dbCommand->FetchByte(i++, pData->m_bHPDrain);
			_dbCommand->FetchByte(i++, pData->m_bMPDamage);
			_dbCommand->FetchByte(i++, pData->m_bMPDrain);
			_dbCommand->FetchByte(i++, pData->m_bMirrorDamage);
			_dbCommand->FetchByte(i++, pData->m_DropRate);
			_dbCommand->FetchInt16(i++, pData->m_sStrB);
			_dbCommand->FetchInt16(i++, pData->m_sStaB);
			_dbCommand->FetchInt16(i++, pData->m_sDexB);
			_dbCommand->FetchInt16(i++, pData->m_sIntelB);
			_dbCommand->FetchInt16(i++, pData->m_sChaB);
			_dbCommand->FetchInt16(i++, pData->m_MaxHpB);
			_dbCommand->FetchInt16(i++, pData->m_MaxMpB);
			_dbCommand->FetchInt16(i++, pData->m_bFireR);
			_dbCommand->FetchInt16(i++, pData->m_bColdR);
			_dbCommand->FetchInt16(i++, pData->m_bLightningR);
			_dbCommand->FetchInt16(i++, pData->m_bMagicR);
			_dbCommand->FetchInt16(i++, pData->m_bPoisonR);
			_dbCommand->FetchInt16(i++, pData->m_bCurseR);
			_dbCommand->FetchInt16(i++, pData->ItemClass);
			_dbCommand->FetchUInt32(i++, pData->m_iNPBuyPrice);
			_dbCommand->FetchInt16(i++, pData->m_Bound);
			_dbCommand->FetchByte(i++, pData->m_byGrade);
			_dbCommand->FetchByte(i++, pData->m_isDropNotice);
			_dbCommand->FetchByte(i++, pData->m_isUpgradeNotice);
			if (!m_pMap->PutData(pData->m_iNum, pData))
				delete pData;
		}

		return true;
	}

	ItemtableArray *m_pMap;
};
