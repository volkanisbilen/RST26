#pragma once

INLINE bool blooddrain(uint32 skillid) {
	return skillid == 107610 || skillid == 108610 || skillid == 207610 || skillid == 208610;
}
INLINE bool vampirictouch(uint32 skillid) {
	return skillid == 107650 || skillid == 108650 || skillid == 207650 || skillid == 208650;
}
INLINE bool illusion(uint32 skillid) {
	return skillid == 107630 || skillid == 108630 || skillid == 207630 || skillid == 208630;
}
INLINE bool throwingKnife(uint32 skillid) {
	return skillid == 107656 || skillid == 108656 || skillid == 207656 || skillid == 208656;
}
INLINE bool stealth(uint32 skillid) {
	return skillid == 107645 || skillid == 108645 || skillid == 208645 || skillid == 207645;
}

INLINE bool provoke(uint32 skillid) {
	return skillid == 105645 || skillid == 106645 || skillid == 114645 || skillid == 115645
		|| skillid == 205645 || skillid == 206645 || skillid == 214645 || skillid == 215645;
}

INLINE bool battlecry(uint32 skillid) { return skillid == 106781 || skillid == 206781; }
INLINE bool smokescreen(uint32 skillid) { return skillid == 108780 || skillid == 208780; }

class CMagicTableSet : public OdbcRecordset
{
public:
	CMagicTableSet(OdbcConnection * dbConnection, MagictableArray * pMap) 
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("MAGIC"); }
	virtual tstring GetColumns() { return _T("MagicNum, krName, t_1, BeforeAction, TargetAction, SelfEffect, FlyingEffect, TargetEffect, Moral, SkillLevel, Skill, Msp, HP, sSp, ItemGroup, UseItem, CastTime, ReCastTime, SuccessRate, Type1, Type2, Range, Etc, UseStanding, SkillCheck, icelightrate"); }

	virtual bool Fetch()
	{
		int field = 1;
		_MAGIC_TABLE *pData = new _MAGIC_TABLE;
		_dbCommand->FetchUInt32(field++, pData->iNum);
		_dbCommand->FetchString(field++, pData->krname);
		_dbCommand->FetchInt32(field++, pData->t_1);
		_dbCommand->FetchUInt32(field++, pData->nBeforeAction);
		_dbCommand->FetchByte(field++, pData->bTargetAction);
		_dbCommand->FetchByte(field++, pData->bSelfEffect);
		_dbCommand->FetchUInt16(field++, pData->bFlyingEffect);
		_dbCommand->FetchUInt16(field++, pData->iTargetEffect);
		_dbCommand->FetchByte(field++, pData->bMoral);
		_dbCommand->FetchUInt16(field++, pData->sSkillLevel);
		_dbCommand->FetchUInt16(field++, pData->sSkill);
		_dbCommand->FetchUInt16(field++, pData->sMsp);
		_dbCommand->FetchUInt16(field++, pData->sHP);
		_dbCommand->FetchUInt16(field++, pData->sSp);
		_dbCommand->FetchByte(field++, pData->bItemGroup);
		_dbCommand->FetchUInt32(field++, pData->iUseItem);
		_dbCommand->FetchByte(field++, pData->bCastTime);
		_dbCommand->FetchUInt16(field++, pData->sReCastTime);
		_dbCommand->FetchByte(field++, pData->bSuccessRate);
		_dbCommand->FetchByte(field++, pData->bType[0]);
		_dbCommand->FetchByte(field++, pData->bType[1]);
		_dbCommand->FetchUInt16(field++, pData->sRange);
		_dbCommand->FetchUInt16(field++, pData->sEtc);
		_dbCommand->FetchByte(field++, pData->sUseStanding);
		_dbCommand->FetchByte(field++, pData->sSkillCheck);
		_dbCommand->FetchUInt16(field++, pData->icelightrate);

		if (provoke(pData->iNum)) pData->bCastTime = 4;
		else if (battlecry(pData->iNum)) pData->bCastTime = 6;
		else if (blooddrain(pData->iNum)) pData->bCastTime = 9;
		else if (vampirictouch(pData->iNum)) pData->bCastTime = 6;
		else if (illusion(pData->iNum)) pData->bCastTime = 7;
		else if (throwingKnife(pData->iNum)) pData->bCastTime = 6;
		else if (smokescreen(pData->iNum)) pData->bCastTime = 7;
		else if (stealth(pData->iNum)) pData->bCastTime = 25;
		else if (pData->iNum == 490221) pData->bCastTime = 6;
		else if (pData->iNum == 490232 || pData->iNum == 490228 || pData->iNum == 490224) pData->bCastTime = 3;

		pData->pType4 = _MAGIC_TYPE4();

		if (!m_pMap->PutData(pData->iNum, pData))
			delete pData;

		return true;
	}

	MagictableArray *m_pMap;
};