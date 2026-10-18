#pragma once

class CCswTimeOptSet : public OdbcRecordset
{
public:
	CCswTimeOptSet(OdbcConnection* dbConnection, _CSW_TIMEOPT* pData)
		: OdbcRecordset(dbConnection), ptimedata(pData) {}

	virtual tstring GetTableName() { return _T("KNIGHTS_CSW_OPT"); }
	virtual tstring GetColumns() { return _T("Preparing,WarTime,money,tl,cash,loyalty,"
		"itemid1,itemcount1,itemtime1,"
		"itemid2,itemcount2,itemtime2,"
		"itemid3,itemcount3,itemtime3"); }

	virtual bool Fetch() {
		int field = 1;
		_dbCommand->FetchUInt32(field++, ptimedata->Preparing);
		_dbCommand->FetchUInt32(field++, ptimedata->wartime);
		
		_dbCommand->FetchUInt32(field++, ptimedata->money);
		_dbCommand->FetchUInt32(field++, ptimedata->tl);
		_dbCommand->FetchUInt32(field++, ptimedata->cash);
		_dbCommand->FetchUInt32(field++, ptimedata->loyalty);
		for (int i = 0; i < 3; i++) {
			_dbCommand->FetchUInt32(field++, ptimedata->itemid[i]);
			_dbCommand->FetchUInt32(field++, ptimedata->itemcount[i]);
			_dbCommand->FetchUInt32(field++, ptimedata->itemtime[i]);
		}
		return true;
	}
	_CSW_TIMEOPT* ptimedata;
};