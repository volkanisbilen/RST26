 #pragma once

class CKnightsSiegeWarfare : public OdbcRecordset
{
public:
	CKnightsSiegeWarfare(OdbcConnection * dbConnection, _KNIGHTS_SIEGE_WARFARE* pMap)
		: OdbcRecordset(dbConnection), pSiegeData(pMap) {}

	virtual tstring GetTableName() { return _T("KNIGHTS_SIEGE_WARFARE"); }
	virtual tstring GetColumns() { return _T("sCastleIndex, sMasterKnights, bySiegeType, byWarDay, byWarTime, byWarMinute, sChallengeList_1, sChallengeList_2, sChallengeList_3, sChallengeList_4, sChallengeList_5, sChallengeList_6, sChallengeList_7, sChallengeList_8, sChallengeList_9, sChallengeList_10, byWarRequestDay, byWarRequestTime, byWarRequestMinute, byGuerrillaWarDay, byGuerrillaWarTime, byGuerrillaWarMinute, strChallengeList, sMoradonTariff, sDellosTariff, nDungeonCharge, nMoradonTax, nDellosTax, sRequestList_1, sRequestList_2, sRequestList_3, sRequestList_4, sRequestList_5, sRequestList_6, sRequestList_7, sRequestList_8, sRequestList_9, sRequestList_10"); }

	virtual bool Fetch()
	{
		int field = 1;
		_dbCommand->FetchUInt16(field++, pSiegeData->sCastleIndex);
		_dbCommand->FetchUInt16(field++, pSiegeData->sMasterKnights);
		_dbCommand->FetchByte(field++, pSiegeData->bySiegeType);
		_dbCommand->FetchByte(field++, pSiegeData->byWarDay);
		_dbCommand->FetchByte(field++, pSiegeData->byWarTime);
		_dbCommand->FetchByte(field++, pSiegeData->byWarMinute);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_1);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_2);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_3);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_4);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_5);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_6);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_7);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_8);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_9);
		_dbCommand->FetchUInt16(field++, pSiegeData->sChallengeList_10);
		_dbCommand->FetchByte(field++, pSiegeData->byWarRequestDay);
		_dbCommand->FetchByte(field++, pSiegeData->byWarRequestTime);
		_dbCommand->FetchByte(field++, pSiegeData->byWarRequestMinute);
		_dbCommand->FetchByte(field++, pSiegeData->byGuerrillaWarDay);
		_dbCommand->FetchByte(field++, pSiegeData->byGuerrillaWarTime);
		_dbCommand->FetchByte(field++, pSiegeData->byGuerrillaWarMinute);
		_dbCommand->FetchString(field++, pSiegeData->strChallengeList);
		_dbCommand->FetchUInt16(field++, pSiegeData->sMoradonTariff);
		_dbCommand->FetchUInt16(field++, pSiegeData->sDellosTariff);
		_dbCommand->FetchInt32(field++, pSiegeData->nDungeonCharge);
		_dbCommand->FetchInt32(field++, pSiegeData->nMoradonTax);
		_dbCommand->FetchInt32(field++, pSiegeData->nDellosTax);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_1);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_2);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_3);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_4);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_5);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_6);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_7);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_8);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_9);
		_dbCommand->FetchUInt16(field++, pSiegeData->sRequestList_10);

		C3DMap* pMap = g_pMain->GetZoneByID(ZONE_MORADON);
		if (pMap != nullptr) pMap->SetTariff(uint8(pSiegeData->sMoradonTariff));
		pMap = g_pMain->GetZoneByID(ZONE_MORADON2);
		if (pMap != nullptr) pMap->SetTariff(uint8(pSiegeData->sMoradonTariff));
		pMap = g_pMain->GetZoneByID(ZONE_MORADON3);
		if (pMap != nullptr) pMap->SetTariff(uint8(pSiegeData->sMoradonTariff));
		pMap = g_pMain->GetZoneByID(ZONE_MORADON4);
		if (pMap != nullptr) pMap->SetTariff(uint8(pSiegeData->sMoradonTariff));
		pMap = g_pMain->GetZoneByID(ZONE_MORADON5);
		if (pMap != nullptr) pMap->SetTariff(uint8(pSiegeData->sMoradonTariff));
		pMap = g_pMain->GetZoneByID(ZONE_DELOS);
		if (pMap != nullptr) pMap->SetTariff(uint8(pSiegeData->sDellosTariff));
		return true;
	}

	_KNIGHTS_SIEGE_WARFARE* pSiegeData;
};