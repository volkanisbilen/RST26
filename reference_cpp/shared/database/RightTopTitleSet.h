#pragma once
class CRightTopTitleSet : public OdbcRecordset
{
public:
	CRightTopTitleSet(OdbcConnection * dbConnection, RightTopTitleArray * pMap)
		: OdbcRecordset(dbConnection), m_pMap(pMap) {}

	virtual tstring GetTableName() { return _T("RIGHT_TOP_TITLE"); }
	virtual tstring GetColumns() { return _T("id, strMessage, strTitle"); }

	virtual bool Fetch()
	{
		_RIGHT_TOP_TITLE_MSG *pData = new _RIGHT_TOP_TITLE_MSG;


		_dbCommand->FetchInt32(1, pData->id);

		_dbCommand->FetchString(2, pData->strMessage);
		_dbCommand->FetchString(3, pData->strTitle);

		if (!m_pMap->PutData(pData->id, pData))
		{
			delete pData;
			return false;
		}
		return true;
	}

	RightTopTitleArray *m_pMap;
};