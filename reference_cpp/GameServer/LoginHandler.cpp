#include "stdafx.h"

void CUser::VersionCheck(Packet & pkt)
{
	if (m_bSelectedCharacter || m_bIsLoggingOut)
		return;

	Packet result(WIZ_VERSION_CHECK);
	result << uint8(0) << uint16(__VERSION) << uint8_t(0) << uint64(0) << uint64(0) << uint8(0); // 0 = success, 1 = prem error
	Send(&result);
}

void CUser::KickOutProcess(Packet & pkt)
{
	std::string account;
	pkt >> account;
	if (account.empty() || account.size() > MAX_ID_SIZE)
		return;

	CUser* pUser = g_pMain->GetUserPtr(account, NameType::TYPE_ACCOUNT);
	if (pUser)
		pUser->goDisconnect();

	return;
	/*std::string crpyto = "", account = "", password = "";
	pkt >> crpyto;
	if (crpyto.empty() || crpyto.size() < 4)
		return;

	size_t start = 0, end = crpyto.find("+//");
	if (end != std::string::npos)
		account = crpyto.substr(start, end - start);
	else return;

	if (account.empty() || account.size() > MAX_ID_SIZE)
		return;

	start = end + 3;
	password = crpyto.substr(start);
	if (password.empty() || !g_pMain->WordGuardSystem(password, password.size()))
		return;

	CUser* pUser = g_pMain->GetUserPtr(account, NameType::TYPE_ACCOUNT);
	if (!pUser)
		return;

	Packet newpkt(WIZ_KICKOUT);
	newpkt << account << password;
	g_pMain->AddDatabaseRequest(newpkt, this);*/
}

void CUser::ReqKickOutProcess(Packet& pkt)
{
	std::string accountid, passid;
	pkt >> accountid >> passid;

	if (accountid.empty() || accountid.size() > MAX_ID_SIZE
		|| passid.empty())
		return;

	std::string db_password = g_DBAgent.getmypassword(accountid);
	if (db_password.empty() || db_password != passid)
		return;

	CUser* pUser = g_pMain->GetUserPtr(accountid, NameType::TYPE_ACCOUNT);
	if (pUser)
		pUser->goDisconnect();
}

void CUser::SendLoginFailed(int8 errorid) {

	Packet result(WIZ_LOGIN);
	result << int8(-1);
	Send(&result);
}

void CUser::LoginProcess(Packet & pkt)
{
	if (!m_strAccountID.empty() || !string_is_valid(m_strAccountID))
		return;

	if (m_bSelectedCharacter || m_bIsLoggingOut || !m_strAccountID.empty())
		return SendLoginFailed(-1);

	std::string strAccountID, strPasswd;
	pkt >> strAccountID >> strPasswd;
	if (strAccountID.empty() || strAccountID.size() > MAX_ID_SIZE
		|| strPasswd.empty() || strPasswd.size() > MAX_PW_SIZE)
		return;
	
	bool disconnect = false;
	CUser* pUser = g_pMain->GetUserPtr(strAccountID, NameType::TYPE_ACCOUNT);
	if (pUser && pUser->IsDeleted())
		return SendLoginFailed(-1);

	if (pUser)
		disconnect = true;

	Packet result(WIZ_LOGIN);
	result << strPasswd << strAccountID << disconnect;
	g_pMain->AddDatabaseRequest(result, this);
}