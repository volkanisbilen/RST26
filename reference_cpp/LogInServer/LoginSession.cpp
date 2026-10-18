#pragma region include
#include "stdafx.h"
#include "../shared/DateTime.h"

LSPacketHandler PacketHandlers[NUM_LS_OPCODES];
#pragma endregion

#pragma region InitPacketHandlers
void InitPacketHandlers(void)
{
	memset(&PacketHandlers, 0, sizeof(LSPacketHandler) * NUM_LS_OPCODES);
	PacketHandlers[LS_VERSION_REQ]			= &LoginSession::HandleVersion;
	PacketHandlers[LS_DOWNLOADINFO_REQ]		= &LoginSession::HandlePatches;
	PacketHandlers[LS_LOGIN_REQ]			= &LoginSession::HandleLogin;
	PacketHandlers[LS_SERVERLIST]			= &LoginSession::HandleServerlist;
	PacketHandlers[LS_NEWS]					= &LoginSession::HandleNews;
	PacketHandlers[LS_CRYPTION]				= &LoginSession::HandleSetEncryptionPublicKey;
	PacketHandlers[LS_UNKF7]				= &LoginSession::HandleUnkF7;
	PacketHandlers[LS_OTP]					= &LoginSession::HandleOTP;
	PacketHandlers[LS_OTP_SYNC]				= &LoginSession::HandleOTPSync;


	PacketHandlers[LS_KOREAKOLAUNCHER_PING] = &LoginSession::HandleLauncherPing;
}
#pragma endregion

#pragma region Constructor
/**
* @brief The constructor.
*/
LoginSession::LoginSession(uint16 socketID, SocketMgr *mgr) : KOSocket(socketID, mgr, -1, 2048, 256), OTPTrials(0), isOTPPassed(false) {}

#pragma endregion

#pragma region LoginSession::OnConnect()
/**
* @brief	Executes the connect action.
*/
void LoginSession::OnConnect()
{
	KOSocket::OnConnect();
	Initialize();
}
#pragma endregion

#pragma region LoginSession::Initialize()
/**
* @brief	Initializes this object.
*/
void LoginSession::Initialize()
{
	OTPKey = "";
	OTPTrials = 0;
	isOTPPassed = false;
}
#pragma endregion

#pragma region LoginSession::OnDisconnect()
/**
* @brief	Executes the disconnect action.
*/
void LoginSession::OnDisconnect()
{
	KOSocket::OnDisconnect();
}
#pragma endregion

#pragma region LoginSession::HandlePacket(Packet & pkt)

/**
* @brief	Handles an incoming user packet.
*
* @param	pkt	The packet.
*
* @return	true if it succeeds, false if it fails.
*/
bool LoginSession::HandlePacket(Packet & pkt)
{
	uint8 opcode = pkt.GetOpcode();

	// Unknown packetWIZ
	if (opcode >= NUM_LS_OPCODES
		|| PacketHandlers[opcode] == nullptr)
		return false;

	(this->*PacketHandlers[opcode])(pkt);
	return true;
}
#pragma endregion

#pragma region LoginSession::HandleVersion(Packet & pkt)
void LoginSession::HandleVersion(Packet & pkt)
{
	Packet result(pkt.GetOpcode());
	result << g_pMain->GetVersion();
	Send(&result);
}
#pragma endregion

#pragma region LoginSession::HandlePatches(Packet & pkt)
void LoginSession::HandlePatches(Packet & pkt)
{
	/*Packet result(pkt.GetOpcode());
	std::set<std::string> downloadset;
	uint16 version;
	pkt >> version;

	foreach(itr, (*g_pMain->GetPatchList()))
	{
		_VERSION_INFO* pInfo = itr->second;
		if (pInfo->sVersion > version)
			downloadset.insert(pInfo->strFilename);
	}

	result << g_pMain->GetFTPUrl() << g_pMain->GetFTPPath();
	result << uint16(downloadset.size());
	foreach(itr, downloadset)
		result << (*itr);
	printf("IP: %s Client Version: %d < Will download %d patch(es).>\n", GetRemoteIP().c_str(), version, uint8(downloadset.size()));
	Send(&result);*/


	Packet result(pkt.GetOpcode());
	uint16 version;
	pkt >> version;

	std::vector<std::string> downloadset;
	foreach(itr, (*g_pMain->GetPatchList())) {
		auto* pInfo = itr->second;
		if (pInfo != nullptr && pInfo->sVersion > version)
			downloadset.push_back(pInfo->strFilename);
		//downloadset.insert(pInfo->strFilename);
	}

	result << g_pMain->GetFTPUrl() << g_pMain->GetFTPPath();

	if (downloadset.empty()) {
		result << uint16(0);
		Send(&result);
		return;
	}
		
	std::sort(downloadset.begin(), downloadset.end());
	result << uint16(downloadset.size());
	foreach(itr, downloadset) result << (*itr);
	printf ("IP: %s Client Version: %d < Will download %d patch(es).>\n", GetRemoteIP().c_str(), version,uint8(downloadset.size()));
	Send(&result);
}
#pragma endregion


void LoginSession::HandleLauncherPing(Packet & pkt)
{
	Packet result(pkt.GetOpcode());
	Send(&result);
}

#pragma region LoginSession::HandleLogin(Packet & pkt)
void LoginSession::HandleLogin(Packet& pkt)
{
	enum LoginErrorCode
	{
		AUTH_SUCCESS = 0x01,
		AUTH_NOT_FOUND = 0x02,
		AUTH_INVALID = 0x03,
		AUTH_BANNED = 0x04,
		AUTH_IN_GAME = 0x05,
		AUTH_ERROR = 0x06,
		AUTH_AGREEMENT = 0xF,
		AUTH_OTP = 0x10,
		AUTH_FAILED = 0xFF
	};

	Packet result(pkt.GetOpcode());
	uint16 resultCode = 0;
	std::string account, password, OTP_Key;
	DateTime time;

	pkt >> account >> password;

	if (account.size() == 0
		|| account.size() > MAX_ID_SIZE
		|| password.size() == 0
		|| password.size() > MAX_PW_SIZE
		|| !string_is_valid(account))
		resultCode = AUTH_NOT_FOUND;
	else
		resultCode = g_pMain->m_DBProcess.AccountLogin(account, password, OTP_Key);

	std::string sAuthMessage;

	switch (resultCode)
	{
	case AUTH_SUCCESS:
		sAuthMessage = "SUCCESS";
		break;
	case AUTH_NOT_FOUND:
		sAuthMessage = "NOT FOUND";
		break;
	case AUTH_INVALID:
		sAuthMessage = "INVALID";
		break;
	case AUTH_BANNED:
		sAuthMessage = "BANNED";
		break;
	case AUTH_IN_GAME:
		sAuthMessage = "IN GAME";
		break;
	case AUTH_ERROR:
		sAuthMessage = "ERROR";
		break;
	case AUTH_AGREEMENT:
		sAuthMessage = "USER AGREEMENT";
		break;
	case AUTH_FAILED:
		sAuthMessage = "FAILED";
		break;
	case AUTH_OTP:
	{
		sAuthMessage = "OTP VALIDATION";
		OTPKey = OTP_Key;
	}
	break;
	default:
		sAuthMessage = string_format("UNKNOWN (%d)", resultCode);
		break;
	}

	printf("[ LOGIN - %d:%d:%d ] ID=%s Authentication=%s IP=%s\n",
		time.GetHour(), time.GetMinute(), time.GetSecond(),
		account.c_str(), sAuthMessage.c_str(), GetRemoteIP().c_str());

	result << uint16(0) << uint8(resultCode);// << uint8(0) << uint8(0);
	if (resultCode == AUTH_SUCCESS)
	{
		result << g_pMain->m_DBProcess.AccountPremium(account);
		result << account;
	}
	if (resultCode == AUTH_OTP)
	{
		result << uint32(0);
	}
	else if (resultCode == AUTH_IN_GAME)
	{
		/*Packet newpkt(LOGIN_ADD_ACCOUNT);
		newpkt << account << password;
		g_pMain->m_GameServerSocketMgr.SendAll(&newpkt);*/

		std::string iTarIP = g_pMain->m_DBProcess.GetConnectedServerIP(account);
		if (!iTarIP.empty())
		{
			std::string cryp = account + "+//" + password;

			result << iTarIP;
			result << uint16(15572); //Xtreme eski port 15001
			
			result << account;
		}
	}
	else if (resultCode == AUTH_AGREEMENT)
	{

	}

	//g_pMain->WriteUserLogFile(string_format("LOGIN - %d:%d:%d || ID=%s,PDW=%s,Authentication=%s,IP=%s\n", time.GetHour(), time.GetMinute(), time.GetSecond(), account.c_str(), password.c_str(), sAuthMessage.c_str(), GetRemoteIP().c_str()));

	Send(&result);
}
#pragma endregion

#pragma region LoginSession::HandleServerlist(Packet & pkt)
void LoginSession::HandleServerlist(Packet & pkt)
{
	Packet result(pkt.GetOpcode());

	uint16 echo;
	pkt >> echo;
	result << echo;

	g_pMain->GetServerList(result);
	Send(&result);
}
#pragma endregion

#pragma region LoginSession::HandleNews(Packet & pkt)
void LoginSession::HandleNews(Packet & pkt)
{
	Packet result(pkt.GetOpcode());
	News *pNews = g_pMain->GetNews();

	if (pNews->Size)
	{
		result << "Login Notice" << uint16(pNews->Size);
		result.append(pNews->Content, pNews->Size);
	}
	else // dummy news, will skip past it
	{
		result << "Login Notice" << "<empty>";
	}
	Send(&result);
}
#pragma endregion

#pragma region LoginSession::HandleSetEncryptionPublicKey(Packet & pkt)
void LoginSession::HandleSetEncryptionPublicKey(Packet & pkt)
{
	Packet result(pkt.GetOpcode());
	result << m_crypto.GenerateKey();
	Send(&result);
	EnableCrypto();
}
#pragma endregion

#pragma region LoginSession::HandleUnkF7(Packet & pkt)
void LoginSession::HandleUnkF7(Packet & pkt)
{
	Packet result(pkt.GetOpcode());
	result << uint16(0);
	Send(&result);
}
#pragma endregion

#pragma region LoginSession::HandleOTP(Packet & pkt)
void LoginSession::HandleOTP(Packet & pkt)
{
	enum LoginErrorCode
	{
		AUTH_SUCCESS = 0x01,
		AUTH_NOT_FOUND = 0x02,
		AUTH_INVALID = 0x03,
		AUTH_BANNED = 0x04,
		AUTH_IN_GAME = 0x05,
		AUTH_ERROR = 0x06,
		AUTH_AGREEMENT = 0xF,
		AUTH_FAILED = 0xFF
	};

	Packet result(pkt.GetOpcode());
	uint16 resultCode = 0;
	std::string account, password, OTPCode;
	DateTime time;

	pkt >> account >> password >> OTPCode;

	if (account.size() == 0
		|| account.size() > MAX_ID_SIZE
		|| password.size() == 0
		|| password.size() > MAX_PW_SIZE
		|| !string_is_valid(account))
		return;
	else
		resultCode = g_pMain->m_DBProcess.AccountLoginOTP(account, password);

	std::string sAuthMessage;

	if (resultCode == AUTH_NOT_FOUND
		|| resultCode == AUTH_INVALID
		|| resultCode == AUTH_BANNED
		|| resultCode == AUTH_ERROR
		|| resultCode == AUTH_FAILED)
		return;

	if (OTPKey == OTPCode)
	{
		isOTPPassed = true;
		sAuthMessage = "SUCCESS";
	}
	else if (OTPKey != OTPCode)
	{
		isOTPPassed = false;
		OTPTrials++;
		sAuthMessage = "INVALID OTP";
	}

	printf("[ LOGIN OTP - %d:%d:%d ] ID=%s Authentication=%s IP=%s\n",
		time.GetHour(), time.GetMinute(), time.GetSecond(),
		account.c_str(), sAuthMessage.c_str(), GetRemoteIP().c_str());

	if (isOTPPassed)
	{
		result.clear();
		result.Initialize(LS_LOGIN_REQ);
		result << uint16(0) << uint8(resultCode);// << uint8(0) << uint8(0);
		
		if (resultCode == AUTH_SUCCESS)
		{
			result << g_pMain->m_DBProcess.AccountPremium(account);
			result << account;
		}
	}
	else
	{
		if (OTPTrials > 2)
		{
			result << uint16(0) << OTPCode;
			Send(&result);
			Disconnect();
			return;
		}

		result << uint16(2) << OTPCode;
	}

//	g_pMain->WriteUserLogFile(string_format("LOGIN OTP - %d:%d:%d || ID=%s,PDW=%s,Authentication=%s,IP=%s\n", time.GetHour(), time.GetMinute(), time.GetSecond(), account.c_str(), password.c_str(), sAuthMessage.c_str(), GetRemoteIP().c_str()));

	Send(&result);
}
#pragma endregion

#pragma region LoginSession::HandleOTPSync(Packet & pkt)
void LoginSession::HandleOTPSync(Packet & pkt)
{
	Packet result(pkt.GetOpcode());
	result << uint16(0);
	Send(&result);
}
#pragma endregion