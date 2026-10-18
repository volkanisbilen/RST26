#include "stdafx.h"

void CUser::VersionCheck(Packet & pkt)
{
	Packet result(WIZ_VERSION_CHECK);
	result << uint8(0) << uint16(__VERSION) << uint8_t(0) << uint64(0) << uint64(0) << uint8(0); // 0 = success, 1 = prem error
	Send(&result);
}

void CUser::KickOutProcess(Packet & pkt)
{
	//printf("KickOut Paketi Geldi\n");
	Packet result(WIZ_LOGOUT);
	std::string strAccountID = "";

	pkt >> strAccountID;
	if (strAccountID.empty() || strAccountID.size() > MAX_ID_SIZE)
		return;

	CUser * pUser = g_pMain->GetUserPtr(strAccountID, NameType::TYPE_ACCOUNT);

	if (pUser)
		pUser->Disconnect();
}

void CUser::LoginProcess(Packet & pkt)
{
	// Enforce only one login request per session
	// It's common for players to spam this at the server list when a response isn't received immediately.
	if (!m_strAccountID.empty())
		return;

	if(!string_is_valid(m_strAccountID))
		return;

	Packet result(WIZ_LOGIN);
	std::string strAccountID, strPasswd;
	pkt >> strAccountID >> strPasswd;

	/*
	 01 0031 0700 47575655564930 000000010001000000000000
	*/

	if (strAccountID.empty() || strAccountID.size() > MAX_ID_SIZE
		|| strPasswd.empty() || strPasswd.size() > MAX_PW_SIZE) {
		result << int8(-1);
		Send(&result);
		return;
	}

	m_strAccountID = strAccountID;
	result << strPasswd << strAccountID;
	g_pMain->AddDatabaseRequest(result, this);
}