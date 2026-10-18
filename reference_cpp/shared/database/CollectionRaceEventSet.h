#pragma once

class CollectionRaceEventListSet : public OdbcRecordset
{
public:
	CollectionRaceEventListSet(OdbcConnection* dbConnection, CollectionRaceEventListArray* pMap) : OdbcRecordset(dbConnection), m_pMap(pMap) {}
#if GAME_SOURCE_VERSION  == 2369
	virtual tstring GetTableName() { return _T("COLLECTION_RACE_EVENT_SETTINGS2369"); }
#elif GAME_SOURCE_VERSION  == 1098
	virtual tstring GetTableName() { return _T("COLLECTION_RACE_EVENT_SETTINGS1098"); }
#elif GAME_SOURCE_VERSION  == 1534
	virtual tstring GetTableName() { return _T("COLLECTION_RACE_EVENT_SETTINGS1534"); }
#endif
	virtual tstring GetColumns() {
		return _T("sEventIndex,EventName, Unit1,UnitCount1,Unit2,UnitCount2,Unit3,UnitCount3,MinLevel,Maxlevel,EventZone,EventTime,UserLimit,isRepeatStatus,"
			"AutoStart,AutoHour,AutoMinute,AutoDay,WarriorKill,RogueKill,MageKill,PriestKill,ClassStatus");
	}

	virtual bool Fetch()
	{
		_COLLECTION_RACE_EVENT_LIST* pData = new _COLLECTION_RACE_EVENT_LIST;

		auto i = 1, sEventIndex = 0;

		_dbCommand->FetchUInt32(i++, pData->m_EventID);
		_dbCommand->FetchString(i++, pData->EventName);

		for (uint32 j = 0; j < 3; j++)
		{
			_dbCommand->FetchUInt16(i++, pData->ProtoID[j]);
			_dbCommand->FetchUInt16(i++, pData->KillCount[j]);

		}

		_dbCommand->FetchByte(i++, pData->MinLevel);
		_dbCommand->FetchByte(i++, pData->MaxLevel);
		_dbCommand->FetchByte(i++, pData->ZoneID);
		_dbCommand->FetchUInt32(i++, pData->EventTime);
		_dbCommand->FetchUInt16(i++, pData->UserLimit);
		_dbCommand->FetchByte(i++, pData->RepeatStatus);
		_dbCommand->Fetchtbool(i++, pData->autostart);
		_dbCommand->FetchInt32(i++, pData->autohour);
		_dbCommand->FetchInt32(i++, pData->autominute);
		_dbCommand->FetchString(i++, pData->days);
		_dbCommand->FetchString(i++, pData->warriorkill);
		_dbCommand->FetchString(i++, pData->roguekill);
		_dbCommand->FetchString(i++, pData->magekill);
		_dbCommand->FetchString(i++, pData->priestkill);
		_dbCommand->FetchByte(i++, pData->ClassStatus);
		if (!m_pMap->PutData(pData->m_EventID, pData))
			delete pData;

		return true;
	}

	CollectionRaceEventListArray* m_pMap;
};