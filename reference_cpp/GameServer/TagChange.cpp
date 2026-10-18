#include "StdAfx.h"

void CUser::HandleTagChange(Packet &pkt) {
	uint8 subcode = pkt.read<uint8>();
	switch ((tagerror)subcode)
	{
	case tagerror::newtag:
		TagChange(pkt);
		break;
	}
}

void CUser::UserInOutTag(std::vector<CUser*> mlist)
{
	if (mlist.empty()) return;

	uint16 counter = 0;
	Packet newpkt(XSafe, (uint8)XSafeOpCodes::TagInfo);
	newpkt << (uint8)tagerror::List << counter;
	foreach(itr, mlist) {
		CUser *puser = *itr;
		if (!puser || puser->mytagname.empty() || puser->mytagname == "-") continue;
		uint8 r = GetRValue(puser->GetTagNameColour()), g = GetGValue(puser->GetTagNameColour()), b = GetBValue(puser->GetTagNameColour());
		newpkt << puser->GetSocketID() << puser->mytagname << r << g << b;
		counter++;
	}
	newpkt.put(2, counter);
	Send(&newpkt);
}

void CUser::SendTagNameChangePanel() {
	Packet newpkt(XSafe, (uint8)XSafeOpCodes::TagInfo);
	newpkt << uint8(tagerror::Open);
	Send(&newpkt);
}

void CUser::TagChange(Packet &pkt) {

	std::string newtag; uint8 r, g, b;
	pkt >> newtag >> r >> g >> b;
	Packet mytag(XSafe, (uint8)XSafeOpCodes::TagInfo);
	if (newtag.empty()/* || !g_pMain->WordGuardSystem(newtag, (uint8)newtag.length())*/) { //Özel Karekter Kontrolü Kapatýldý
		mytag << (uint8)tagerror::error;
		Send(&mytag);
		return;
	}

	if (newtag == mytagname) {
		mytag << (uint8)tagerror::already;
		Send(&mytag);
		return;
	}

	if (newtag.size() > MAX_ID_SIZE) {
		mytag << (uint8)tagerror::error;
		Send(&mytag);
		return;
	}

	if (!CheckExistItem(800099000, 1)) {
		mytag << (uint8)tagerror::noitem;
		Send(&mytag);
		return;
	}

	if (!RobItem(800099000, 1)) {
		mytag << (uint8)tagerror::error;
		Send(&mytag);
		return;
	}

	DWORD colour = RGB(r, g, b);

	mytagname = newtag; m_tagnamergb = colour;
	mytag << (uint8)tagerror::success << uint8(0) << GetSocketID() << mytagname << r << g << b;
	Send(&mytag);
	
	mytag.clear();
	mytag << (uint8)XSafeOpCodes::TagInfo << (uint8)tagerror::success << uint8(1) << GetSocketID() << mytagname << r << g << b;
	SendToRegion(&mytag,this,GetEventRoom());
}
