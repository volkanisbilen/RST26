#include "StdAfx.h"

void CUser::SetOffCha(_choffstatus s, offcharactertype type)
{
	uint32 itemid = 0;

	if (type == offcharactertype::merchant)
		itemid = 924041913;
	else if (type == offcharactertype::genie)
		itemid = 824041931;
	else
		return;

	if (s == _choffstatus::ACTIVE) {
		_ITEM_DATA * pItem = GetItem(CFAIRY);
		if (!pItem || pItem->nNum != itemid)
			return;

		m_offmerchanttime = 1400;
	}

	SetOffCharacter(s);
	std::string message = "offline";

	if (type == offcharactertype::genie)
		message.append(" genie");
	else
		message.append(" merchant");

	if (s == _choffstatus::ACTIVE) {
		off_merchantcheck = UNIXTIME + 60;
		message.append(" ready");
	}
	else
		message.append(" has been cancelled");
	SendAcsMessage(message);
}