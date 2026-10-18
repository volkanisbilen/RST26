#include "StdAfx.h"

CGameSocket::~CGameSocket() {}

void CGameSocket::OnConnect()
{
	KOSocket::OnConnect();
	Initialize();
}

void CGameSocket::Initialize()
{

}

void CGameSocket::OnDisconnect()
{
	TRACE("*** CloseProcess - socketID=%d ... server=%s *** \n", GetSocketID(), GetRemoteIP().c_str());
}

bool CGameSocket::HandlePacket(Packet & pkt)
{
	switch (pkt.GetOpcode())
	{
	case LOGIN_LOGIN_CONNECT:
		RecvServerConnect(pkt);
		break;
	case LOGIN_REMOVE_ACCOUNT:
		RecvRemoveAccount(pkt);
		break;
	default:
		break;
	}

	return true;

}

void CGameSocket::RecvServerConnect(Packet & pkt)
{
	uint8 bReconnect = 0;
	pkt >> bReconnect;

	if(bReconnect==0)
		printf("GameServer Connected! IP:%s\n", GetRemoteIP().c_str());
	else
		printf("GameServer reConnected! IP:%s\n", GetRemoteIP().c_str());

	Packet result(LOGIN_LOGIN_CONNECT);
	result << bReconnect;
	Send(&result);
}

void CGameSocket::RecvRemoveAccount(Packet & pkt)
{
	std::string StrAccountID = "";
	pkt >> StrAccountID;
	
	//gerek yok ama dursun..
}