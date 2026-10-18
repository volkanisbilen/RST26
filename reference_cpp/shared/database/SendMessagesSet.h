#pragma once

class CSendMessageSet : public OdbcRecordset
{
public:
	CSendMessageSet(OdbcConnection * dbConnection, SendMessageArray * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("SEND_MESSAGES"); }
	virtual tstring GetColumns() { return _T("id, strMessage, strSender, bChatType, SendType"); }

	virtual bool Fetch()
	{
		_SEND_MESSAGE *pData = new _SEND_MESSAGE;
		

		_dbCommand->FetchInt32(1, pData->id);
		
		_dbCommand->FetchString(2, pData->strMessage);
		_dbCommand->FetchString(3, pData->strSender);

		_dbCommand->FetchUInt16(4, pData->bChatType);
		_dbCommand->FetchUInt16(5, pData->SendType);

		if (!m_pMap->PutData(pData->id, pData))
		{
			delete pData;
			return false;
		}
		return true;
	}

	SendMessageArray *m_pMap;
};