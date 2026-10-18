#include "StdAfx.h"

#define CLAN_NTS_ITEM 800086000  // Clan NTS item ID. 900144023 Monster Stone ile cakismamali.

void CUser::ClanNtsHandler()
{

	if (!isInGame()) return;

	Packet newpkt(WIZ_DB_SAVE_USER, uint8(ProcDbType::ClanNts));
	g_pMain->AddDatabaseRequest(newpkt, this);

	cntscode errorcode = cntscode::Failed;
	if (m_clanntsreq) { errorcode = cntscode::AlreadyReq; goto send_result; }
	if (!isInClan() || !isClanLeader()) { errorcode = cntscode::NoClanLeader; goto send_result; }
	if (!CheckExistItem(CLAN_NTS_ITEM)) { errorcode = cntscode::NoItem; goto send_result; }

	g_pMain->AddDatabaseRequest(newpkt, this);
	m_clanntsreq = true;
	errorcode = cntscode::SuccessReq;

send_result:
	ClanNtsSendMsg(errorcode);
}

void CUser::ReqClanNts()
{

	std::vector<_KNIGHTS_USER> m_userlist;
	m_clanntsreq = false;
	std::string strUserID = "";
	cntscode errorcode = cntscode::Failed;

	struct _nationdata { _NATION_DATA pdata[4]; };
	std::map< std::string, _nationdata> mlist;
	uint16 myclanid = GetClanID();

	if (!isInClan() || !isClanLeader())
	{
		errorcode = cntscode::NoClanLeader;
		goto send_result;
	}

	if (!CheckExistItem(CLAN_NTS_ITEM))
	{
		errorcode = cntscode::NoItem;
		goto send_result;
	}

	auto* pKnights = g_pMain->GetClanPtr(GetClanID());
	if (!pKnights)
		goto send_result;

	CKingSystem* pKingData = g_pMain->m_KingSystemArray.GetData(GetNation());

	pKnights->m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(i, pKnights->m_arKnightsUser)
		if (i->second)
			m_userlist.push_back(*i->second);
	pKnights->m_arKnightsUser.m_lock.unlock();

	foreach(itr, m_userlist)
	{
		CUser* pUser = g_pMain->GetUserPtr(itr->strUserName, NameType::TYPE_CHARACTER);
		if (pUser && pUser != this)
		{
			errorcode = cntscode::OnlineNember;
			strUserID = pUser->GetName();
			goto send_result;
		}

		bool isking = false;
		std::string strCharID[4];
		for (int i = 0; i < 4; i++)strCharID[i] = "";
		if (!g_DBAgent.GetAllCharID(itr->strAccountName, strCharID[0], strCharID[1], strCharID[2], strCharID[3]))
			goto send_result;

		if (pKingData)
		{
			for (int i = 0; i < 4; i++)
			{
				if (strCharID[i].length() > 0 && STRCASECMP(pKingData->m_strKingName.c_str(), strCharID[i].c_str()) == 0)
				{
					strUserID = strCharID[i];
					isking = true;
				}
			}

			if (isking)
			{
				errorcode = cntscode::isKing;
				goto send_result;
			}
		}

		uint8 nnum = 0, nnum2 = 0;
		_NATION_DATA data[4]{};

		for (int i = 0; i < 4; i++)
		{
			if (strCharID[i].length() > 0 && !g_DBAgent.GetAllCharInfo(strCharID[i], data[nnum++]))
				goto send_result;
		}

		if (!nnum)
			goto send_result;

		for (int i = 0; i < 4; i++)
		{
			if (data[i].isnull())
				continue;

			if (data[i].bCharName.empty())
				goto send_result;

			CUser* pUser = g_pMain->GetUserPtr(data[i].bCharName, NameType::TYPE_CHARACTER);
			if (pUser && pUser != this)
			{
				errorcode = cntscode::OnlineNember;
				strUserID = pUser->GetName();
				goto send_result;
			}

			if (data[i].sClanID > 0 && data[i].sClanID != myclanid)
			{
				errorcode = cntscode::isClan;
				strUserID = data[i].bCharName;
				goto send_result;
			}

			nnum2++;

			_nationdata pnew;
			pnew.pdata[i] = data[i];

			auto it = mlist.find(itr->strAccountName);
			if (it != mlist.end())
				it->second.pdata[i] = pnew.pdata[i];
			else
				mlist.insert(std::make_pair(itr->strAccountName, pnew));
		}

		if (!nnum2)
			goto send_result;
	}

	Nation newnation = Nation::KARUS;
	if (GetNation() == (uint8)Nation::KARUS)
		newnation = Nation::ELMORAD;

	if (!g_DBAgent.SaveCNTSKnights(GetClanID(), newnation))
		goto send_result;

	foreach(itr, mlist)
	{
		for (int i = 0; i < 4; i++)
		{
			if (itr->second.pdata[i].isnull())
				continue;

			uint8 newclass = newnation == Nation::ELMORAD ? itr->second.pdata[i].sClass + 100 : itr->second.pdata[i].sClass - 100;
			uint8 newrace = g_pMain->GetCntNewRace(newclass, newnation);
			if (newrace)
				g_DBAgent.SaveClanNationTransferUser(itr->second.pdata[i].bCharName, itr->first, myclanid, newnation, newrace, newclass);
		}
	}

	m_bNation = (uint8)newnation;
	m_sClass = newnation == Nation::ELMORAD ? GetClass() + 100 : GetClass() - 100;
	m_bRace = g_pMain->GetCntNewRace(m_sClass, newnation);

	//goDisconnect();

	errorcode = cntscode::Success;
send_result:
	ClanNtsSendMsg(errorcode, strUserID);
}

uint8 CGameServerDlg::GetCntNewRace(uint16 newclass, Nation newnation)
{
	uint8 classtype = newclass % 100, newrace = 0;
	if ((classtype == 1 || classtype == 5 || classtype == 6) && newnation == Nation::ELMORAD)	newrace = ELMORAD_MAN;
	else if ((classtype == 1 || classtype == 5 || classtype == 6) && newnation == Nation::KARUS)		newrace = KARUS_BIG;
	else if ((classtype == 2 || classtype == 7 || classtype == 8) && newnation == Nation::ELMORAD)	newrace = ELMORAD_MAN;
	else if ((classtype == 2 || classtype == 7 || classtype == 8) && newnation == Nation::KARUS)		newrace = KARUS_MIDDLE;
	else if ((classtype == 3 || classtype == 9 || classtype == 10) && newnation == Nation::ELMORAD)	newrace = ELMORAD_WOMAN;
	else if ((classtype == 3 || classtype == 9 || classtype == 10) && newnation == Nation::KARUS)		newrace = KARUS_SMALL;
	else if ((classtype == 4 || classtype == 11 || classtype == 12) && newnation == Nation::ELMORAD)	newrace = ELMORAD_WOMAN;
	else if ((classtype == 4 || classtype == 11 || classtype == 12) && newnation == Nation::KARUS)		newrace = KARUS_WOMAN;
	else if ((classtype == 13 || classtype == 14 || classtype == 15) && newnation == Nation::ELMORAD)	newrace = PORUTU;
	else if ((classtype == 13 || classtype == 14 || classtype == 15) && newnation == Nation::KARUS)		newrace = KURIAN;
	return newrace;
}

void CUser::ClanNtsSendMsg(cntscode p, std::string strUserID)
{
	std::string title = "Clan Nation Transfer", message = "";
	if (p == cntscode::Failed) message = "something went wrong.";
	else if (p == cntscode::SuccessReq) message = "The request has been successfully discarded.";
	else if (p == cntscode::AlreadyReq) message = "You have already made a request.";
	else if (p == cntscode::NoClanLeader) message = "You are not in a clan or a leader.";
	else if (p == cntscode::OnlineNember) message = string_format("Online User %s", strUserID.empty() ? "-" : strUserID.c_str());
	else if (p == cntscode::NoItem) message = "You do not have the required parts on you.";
	else if (p == cntscode::isKing) message = string_format("King User %s", strUserID.empty() ? "-" : strUserID.c_str());
	else if (p == cntscode::isClan) message = string_format("is in clan User %s", strUserID.empty() ? "-" : strUserID.c_str());
	else if (p == cntscode::Success) message = "Success Process";
	if (!title.empty() && !message.empty()) XSafe_SendMessageBox(title, message);
}