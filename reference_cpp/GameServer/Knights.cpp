#include "stdafx.h"
#include "Knights.h"
#include "KingSystem.h"

using std::string;

#pragma region CKnights::CKnights()
CKnights::CKnights()
{
	m_sIndex = 0;
	m_byFlag = (uint8)ClanTypeFlag::ClanTypeNone;
	m_byNation = 0;
	m_byGrade = 5;
	m_byRanking = 0;
	m_sMembers = 1;
	m_sOnline = 0;
	memset(&m_Image, 0, sizeof(m_Image));
	m_sDomination = 0;
	m_nPoints = 0;
	m_nClanPointFund = 0;
	m_sCape = -1;
	m_sAlliance = 0;
	m_sAllianceReq = 0;
	m_sMarkLen = 0;
	m_sMarkVersion = 0;
	m_bCapeR = m_bCapeG = m_bCapeB = 0;
	m_sClanPointMethod = 0;
	ClanBuffExpBonus = 0;
	ClanBuffNPBonus = 0;
	m_dwTime = 0;
	m_nMoney = 0;
	memset(m_sClanWarehouseArray, 0x00, sizeof(m_sClanWarehouseArray));

	sPremiumTime = 0;

	castcape = false;
	m_CastTime = 0;
	m_castCapeID = -1;
	m_bCastCapeR = 0;
	m_bCastCapeG = 0;
	m_bCastCapeB = 0;

	m_ladlist.clear();
	m_ladlistime = 0;
	m_bOnlineNembers = 0;
	m_bOnlineNpCount = 0;
	m_bOnlineExpCount = 0;

	bySiegeFlag = 0;
	nLose = 0;
	nVictory = 0;
	sPremiumInUse = 0;
	GameMasterSocket = -1; // JstKO GameMaster Klan Sohbetini Oku Eklendi 01.01.2025
}
#pragma endregion 

#pragma region CKnights::OnLogin(CUser *pUser)

void CKnights::OnLogin(CUser *pUser)
{
	if(pUser == nullptr)
		return;

	std:: string userid= pUser->GetName();
	STRTOUPPER(userid);
	_KNIGHTS_USER *pKnightUser = m_arKnightsUser.GetData(userid);
	if(pKnightUser == nullptr)
		return;

	Packet result;
	pKnightUser->bLevel = pUser->m_bLevel;
	pKnightUser->sClass = pUser->m_sClass;
	pKnightUser->nLoyalty = pUser->m_iLoyalty;
	pKnightUser->LoyaltyMonthly = pUser->m_iLoyaltyMonthly;
	pUser->m_pKnightsUser = pKnightUser;

	// Send login notice
	if (GetID() != 1 && GetID() != 15001)
	{
		m_sOnline++;
		result.Initialize(WIZ_KNIGHTS_PROCESS);
		result << uint8(KnightsPacket::KNIGHTS_USER_ONLINE);
		result.SByte();
		result << pUser->GetName();
		Send(&result);
		
		/*Packet aresult(WIZ_KNIGHTS_PROCESS, uint8(0x62));
		aresult << uint8(m_sOnline);
		g_pMain->Send_KnightsMember(GetID(), &aresult);*/
	}
	// Construct the clan notice packet to send to the logged in player
	if (!m_strClanNotice.empty())
	{
		ConstructClanNoticePacket(&result);
		pUser->Send(&result);
	}
}


#pragma endregion 

#pragma region CKnights::OnLoginAlliance(CUser *pUser)

void CKnights::OnLoginAlliance(CUser *pUser)
{
	if(pUser == nullptr)
		return;

	std:: string userid= pUser->GetName();
	STRTOUPPER(userid);
	_KNIGHTS_USER *pKnightUser = m_arKnightsUser.GetData(userid);
	if(pKnightUser == nullptr)
		return;

	Packet result;
	pKnightUser->bLevel = pUser->m_bLevel;
	pKnightUser->sClass = pUser->m_sClass;
	pKnightUser->nLoyalty = pUser->m_iLoyalty;
	pKnightUser->LoyaltyMonthly = pUser->m_iLoyaltyMonthly;
	pUser->m_pKnightsUser = pKnightUser;

	// Construct the clan notice packet to send to the logged in player
	if (!m_strClanNotice.empty())
	{
		ConstructClanNoticePacket(&result);
		pUser->Send(&result);
	}
}

#pragma endregion 

#pragma region CKnights::ConstructClanNoticePacket(Packet *result)

void CKnights::ConstructClanNoticePacket(Packet *result)
{
	if(!m_strClanNotice.empty())
	{
		string clan = "Clan Notice";
		result->Initialize(WIZ_NOTICE);
		result->DByte();
		*result	<< uint8(4)			// type
			<< uint8(1)				// total blocks
			<< clan			// header
			<< m_strClanNotice;
	}
	else
	{
		string clan = "Clan Notice";
		result->Initialize(WIZ_NOTICE);
		result->DByte();
		*result	<< uint8(4)		// type
			<< uint8(2)			// total blocks
			<< clan;			// header
	}
}

#pragma endregion 

#pragma region CKnights::UpdateClanNotice(std::string & clanNotice)

/**
* @brief	Updates this clan's notice with clanNotice
* 			and informs logged in clan members.
*
* @param	clanNotice	The clan notice.
*/
void CKnights::UpdateClanNotice(std::string & clanNotice)
{
	if (clanNotice.length() > MAX_CLAN_NOTICE_LENGTH)
		return;

	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_UPDATENOTICE));;
	// Tell the database to update the clan notice.
	result << GetID() << clanNotice;
	g_pMain->AddDatabaseRequest(result);
}


#pragma endregion 

#pragma region CKnights::UpdateClanFund()

/**
* @brief	Sends a request to update the clan's fund in the database.
*/
void CKnights::UpdateClanFund()
{
	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_UPDATE_FUND));
	result << GetID() << uint32(m_nClanPointFund);
	g_pMain->AddDatabaseRequest(result);
}

#pragma endregion 

#pragma region CKnights::UpdateClanGrade()

/**
* @brief	Sends a request to update the clan's fund in the database.
*/
void CKnights::UpdateClanGrade()
{
	uint32 mtotal = 0;
	foreach_stlmap (i, m_arKnightsUser)
	{
		_KNIGHTS_USER *p = i->second;

		if(p == nullptr)
			continue;

		CUser* pUser = g_pMain->GetUserPtr(p->strUserName, NameType::TYPE_CHARACTER);
		if(pUser != nullptr && pUser->isInGame())
			p->nLoyalty = pUser->m_iLoyalty;

		mtotal += p->nLoyalty;
	}

	m_nPoints = mtotal;
	if (m_byFlag >= (uint8)ClanTypeFlag::ClanTypeAccredited5) 
		m_byGrade = 1;
	else
		m_byGrade = g_pMain->GetKnightsGrade(m_nPoints);

	/*m_byGrade = g_pMain->GetKnightsGrade(m_nPoints);
	if (m_byFlag == (uint8)ClanTypeFlag::ClanTypePromoted && m_byGrade > 3)
	{
		m_byFlag = (uint8)ClanTypeFlag::ClanTypeTraining;
		m_sCape = -1;
		m_bCapeR = m_bCapeG = m_bCapeB = 0;
	}*/
}

#pragma endregion 

#pragma region CKnights::OnLogout(CUser *pUser) 

void CKnights::OnLogout(CUser *pUser)
{
	if (pUser == nullptr)
		return;
	if (GetID() != 1 && GetID() != 15001)
	{
		if (m_sOnline)
			m_sOnline--;


		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_USER_OFFLINE));
		result.SByte();
		result << pUser->GetName();
		Send(&result);
	}


}

#pragma endregion 

#pragma region CKnights::OnLogoutAlliance(CUser *pUser)

void CKnights::OnLogoutAlliance(CUser *pUser)
{
	return;

	if (pUser == nullptr)
		return;

	Packet logoutNotice;
	// TODO: Shift this to SERVER_RESOURCE
	std::string buffer = string_format("%s is offline.", pUser->GetName().c_str()); 
	ChatPacket::Construct(&logoutNotice, (uint8)ChatType::ALLIANCE_CHAT, &buffer);
	CKnights * pKnights = g_pMain->GetClanPtr(pUser->GetClanID());
	
	if(pKnights == nullptr)
		Send(&logoutNotice);
	else
		g_pMain->Send_KnightsAlliance(pKnights->GetAllianceID(), &logoutNotice);
}

#pragma endregion 

#pragma region CKnights::AddUser(std::string & strAccountID,std::string & strUserID, uint8 bFame, uint16 sClass, uint8 bLevel, int32 lastlogin, uint32 Loyalty)

bool CKnights::AddUser(std::string& strAccountID, std::string & strUserID, uint8 bFame, uint16 sClass, uint8 bLevel, int32 lastlogin, uint32 Loyalty, uint32 LoyaltyMonthly)
{
	if (m_sIndex != 1
		&& m_sIndex != 15001
		&& m_arKnightsUser.GetSize() >= MAX_CLAN_USERS)
		return false;

	std:: string userid= strUserID;
	STRTOUPPER(userid);
	if(m_arKnightsUser.GetData(userid) != nullptr)
		return false;

	_KNIGHTS_USER * pKnightUser = new _KNIGHTS_USER();

	pKnightUser->strUserName = strUserID;
	pKnightUser->strAccountName = strAccountID;
	pKnightUser->bLevel = bLevel;
	pKnightUser->sClass = sClass;
	pKnightUser->nLastLogin = lastlogin;
	pKnightUser->bFame = bFame;
	pKnightUser->nLoyalty = Loyalty;
	pKnightUser->LoyaltyMonthly = LoyaltyMonthly;

	CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser != nullptr)
		pUser->m_pKnightsUser = pKnightUser;

	m_arKnightsUser.PutData(userid, pKnightUser);

	return true;
}

#pragma endregion 

#pragma region CKnights::AddUser(CUser *pUser)

bool CKnights::AddUser(CUser *pUser)
{
	if (pUser == nullptr
		|| !AddUser(pUser->GetAccountName(),pUser->GetName(), TRAINEE, pUser->GetClass(), pUser->GetLevel(), uint32(UNIXTIME), pUser->m_iLoyalty, pUser->m_iLoyaltyMonthly))
		return false;

	pUser->SetClanID(m_sIndex);
	pUser->m_bFame = TRAINEE;
	m_sMembers++;
	pUser->KnightsClanBuffUpdate(true, this);
	// xtreme ekleme 05.02.2024
	//pUser->CharacterOlduguYerdeYenile();
	if (pUser->GetLevel() < 51)
		pUser->CharacterOlduguYerdeYenile();// 17.04.2024 clan paket d�zenlemesi
	return true;
}

#pragma endregion 

#pragma region CKnights::RemoveUser(std::string & strUserID)

/**
* @brief	Removes the specified user from the clan array.
*
* @param	strUserID	Identifier for the user.
*
* @return	.
*/
bool CKnights::RemoveUser(std::string & strUserID)
{
	std:: string userid= strUserID;
	STRTOUPPER(userid);
	_KNIGHTS_USER *pKnightUser = m_arKnightsUser.GetData(userid);
	if(pKnightUser == nullptr)
		return false;

	CUser* pRemoveUser = g_pMain->GetUserPtr(pKnightUser->strUserName, NameType::TYPE_CHARACTER);
	if (pRemoveUser != nullptr)
		pRemoveUser->KnightsClanBuffUpdate(false, this);

	// If they're not logged in (note: logged in users being removed have their NP refunded in the other handler)
	// but they've donated NP, ensure they're refunded for the next time they login.
	if (pKnightUser->nDonatedNP > 0)
		RefundDonatedNP(pKnightUser->nDonatedNP, nullptr, pKnightUser->strUserName.c_str());

	if (STRCASECMP(m_strViceChief_1.c_str(), strUserID.c_str()) == 0)
		m_strViceChief_1 = "";
	else if (STRCASECMP(m_strViceChief_2.c_str(), strUserID.c_str()) == 0)
		m_strViceChief_2 = "";
	else if (STRCASECMP(m_strViceChief_3.c_str(), strUserID.c_str()) == 0)
		m_strViceChief_3 = "";

	m_arKnightsUser.DeleteData(userid);

	if (m_sMembers > 1)
		m_sMembers--;
	
	return true;
}


#pragma endregion 

#pragma region CKnights::RemoveUser(CUser *pUser)
bool CKnights::RemoveUser(CUser * pUser)
{
	if (pUser == nullptr || pUser->m_pKnightsUser == nullptr) 
		return false;

	pUser->KnightsClanBuffUpdate(false, this);

	// Ensure users are refunded when they leave/are removed from a clan.
	// NOTE: If we bring back multiserver setups, this is going to cause synchronisation issues.
	uint32 nDonatedNP = pUser->m_pKnightsUser->nDonatedNP;
	if (nDonatedNP > 0)
		RefundDonatedNP(nDonatedNP, pUser);

	if (STRCASECMP(m_strViceChief_1.c_str(), pUser->GetName().c_str()) == 0)
		m_strViceChief_1 = "";
	else if (STRCASECMP(m_strViceChief_2.c_str(), pUser->GetName().c_str()) == 0)
		m_strViceChief_2 = "";
	else if (STRCASECMP(m_strViceChief_3.c_str(), pUser->GetName().c_str()) == 0)
		m_strViceChief_3 = "";

	if (isInPremium())pUser->SendClanPremium(this, true);

	pUser->SetClanID(NO_CLAN);
	pUser->m_bFame = 0;
	pUser->m_pKnightsUser = nullptr;

	std::string userid= pUser->GetName();
	STRTOUPPER(userid);
	m_arKnightsUser.DeleteData(userid);

	if (m_sMembers > 1)
		m_sMembers--;

	if (!pUser->isClanLeader())
		pUser->SendClanUserStatusUpdate();

	// xtreme ekleme 05.02.2024
	CheckKnightBuffSystem();
	pUser->SendCapeBonusNotice();
	pUser->SetUserAbility(true);
	pUser->UserInOut(INOUT_OUT);
	pUser->UserInOut(INOUT_IN);

	// xtreme ekleme 05.02.2024
	//pUser->CharacterOlduguYerdeYenile();

	if (pUser->GetLevel() < 51)
		pUser->CharacterOlduguYerdeYenile();// 17.04.2024 clan paket d�zenlemesi

	return true;
}

#pragma endregion 

#pragma region CKnights::LoadUserMemo(std::string & strUserID, std::string & strMemo)

bool CKnights::LoadUserMemo(std::string & strUserID, std::string & strMemo)
{
	std:: string userid= strUserID;
	STRTOUPPER(userid);
	_KNIGHTS_USER * pKnightUser = m_arKnightsUser.GetData(userid);
	if(pKnightUser == nullptr)
		return false;

	pKnightUser->strMemo = strMemo;
	return true;
}

#pragma endregion 

#pragma region CKnights::RefundDonatedNP(uint32 nDonatedNP, CUser * pUser /*= nullptr*/, const char * strUserID /*= nullptr*/)

/**
* @brief	Refunds 30% of the user's donated NP.
* 			If the user has the item "CONT Recovery", refund ALL of the user's 
* 			donated NP.
*
* @param	nDonatedNP	The donated NP.
* @param	pUser	  	The user's session, when refunding the user in-game.
* 						Set to nullptr to indicate the use of the character's name
* 						and consequently a database update instead.
* @param	strUserID 	Logged out character's name. 
* 						Used to refund logged out characters' national points 
* 						when pUser is set to nullptr.
*/
void CKnights::RefundDonatedNP(uint32 nDonatedNP, CUser * pUser /*= nullptr*/, const char * strUserID /*= nullptr*/)
{
	uint32 m_totalDonationChange = nDonatedNP;

	if(m_byFlag >= (uint8)ClanTypeFlag::ClanTypeAccredited5)
	{
		int counter = 0;
		while(m_totalDonationChange != 0)
		{
			if(m_nClanPointFund < m_totalDonationChange)
			{
				switch ((ClanTypeFlag)m_byFlag)
				{
				case ClanTypeFlag::ClanTypeAccredited5:
					m_nClanPointFund = 0;
					m_totalDonationChange = 0;
					break;
				case ClanTypeFlag::ClanTypeAccredited4: // 7000 * 36 = 252000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeAccredited5;
					m_nClanPointFund += 252000;  
					break;
				case ClanTypeFlag::ClanTypeAccredited3: // 10000 * 36 = 360000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeAccredited4;
					m_nClanPointFund += 360000;   
					break;
				case ClanTypeFlag::ClanTypeAccredited2: // 15000 * 36 = 540000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeAccredited3;
					m_nClanPointFund += 540000; 
					break;
				case ClanTypeFlag::ClanTypeAccredited1: // 20000 * 36 = 720000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeAccredited2;
					m_nClanPointFund += 720000; 
					break;
				case ClanTypeFlag::ClanTypeRoyal5: // 25000 * 36 = 900000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeAccredited1;
					m_nClanPointFund += 900000; 
					break;
				case ClanTypeFlag::ClanTypeRoyal4: // 30000 * 36 = 1080000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeRoyal5;
					m_nClanPointFund += 1080000; 
					break;
				case ClanTypeFlag::ClanTypeRoyal3: // 35000 * 36 = 1260000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeRoyal4;
					m_nClanPointFund += 1260000; 
					break;
				case ClanTypeFlag::ClanTypeRoyal2: // 40000 * 36 = 1440000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeRoyal3;
					m_nClanPointFund += 1440000; 
					break;
				case ClanTypeFlag::ClanTypeRoyal1: // 45000 * 36 = 1620000
					m_byFlag = (uint8)ClanTypeFlag::ClanTypeRoyal2;
					m_nClanPointFund += 1620000; 
					break;
				default:
					TRACE("A clan with unknow clantype IDNum = %d ClanPoints = %d", GetID(), (uint32)m_nClanPointFund);
					printf("A clan with unknow clantype IDNum = %d ClanPoints = %d", GetID(), (uint32)m_nClanPointFund);
					break;
				}

				if(m_nClanPointFund < m_totalDonationChange)
				{
					m_totalDonationChange -= m_nClanPointFund;
					m_nClanPointFund = 0;
				}
				else
				{
					m_nClanPointFund -= m_totalDonationChange;
					m_totalDonationChange = 0;
					break;
				}
			}
			else
			{
				m_nClanPointFund -= m_totalDonationChange;
				m_totalDonationChange = 0;
				break;
			}
		}
	}
	else
	{
		if(m_nClanPointFund < nDonatedNP)
			m_nClanPointFund = 0;
		else
			m_nClanPointFund -= nDonatedNP;
	}

	_KNIGHTS_CAPE *pCape = g_pMain->m_KnightsCapeArray.GetData(m_sCape);
	if (pCape != nullptr)
	{
		if ((pCape->byGrade && m_byGrade > pCape->byGrade)
			// not sure if this should use another error, need to confirm
				|| m_byFlag < pCape->byRanking)
		{
			m_sCape = 0; m_bCapeR = 0;  m_bCapeG = 0; m_bCapeB = 0;

			// Now save (we don't particularly care whether it was able to do so or not).
			Packet db(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_UPDATE_CAPE));
			db	<< GetID() << GetCapeID()
				<< m_bCapeR << m_bCapeG << m_bCapeB;
			g_pMain->AddDatabaseRequest(db);
			SendUpdate();
		}
	}

	// If the player's logged in, just adjust their national points in-game.
	if (pUser != nullptr)
	{
		// Refund 30% of NP unless the user has the item "CONT Recovery".
		// In this case, ALL of the donated NP will be refunded.
		if (!pUser->RobItem(ITEM_CONT_RECOVERY)
			&& !pUser->RobItem(ITEM_CONT_RESTORE_CERT))
		{
			nDonatedNP = (nDonatedNP * 30) / 100;
		}



		if (pUser->m_iLoyalty + nDonatedNP > LOYALTY_MAX)
			pUser->m_iLoyalty = LOYALTY_MAX;
		else
			pUser->m_iLoyalty += nDonatedNP;

		Packet result1(WIZ_LOYALTY_CHANGE, uint8(LOYALTY_NATIONAL_POINTS));
		result1 << pUser->m_iLoyalty << pUser->m_iLoyaltyMonthly
			<< uint32(0) // Clan donations(? Donations made by this user? For the clan overall?)
			<< uint32(0); // Premium NP(? Additional NP gained?)

		pUser->Send(&result1);
		return;
	}

	nDonatedNP = (nDonatedNP * 30) / 100;

	// For logged out players, we must update the player's national points in the database.
	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_REFUND_POINTS));
	result << strUserID << nDonatedNP;
	g_pMain->AddDatabaseRequest(result);
}

#pragma endregion 

void CKnights::ClanPremiumNoticeExits(CUser* pUser)
{
	if (pUser == nullptr)
		return;

	string header = "Clan Bonus";
	Packet result(WIZ_NOTICE);
	result.DByte();
	result << uint8(4) << uint8(2) << header;
	pUser->Send(&result);
}

void CKnights::CastellanCapeNoticeExits(CUser *pUser)
{
	if (pUser == nullptr)
		return;

	string header = "Cape Bonus";
	Packet result(WIZ_NOTICE);
	result.DByte();
	result << uint8(4) << uint8(2) << header;
	pUser->Send(&result);

	pUser->SetUserAbility(true);
}

#pragma region CKnights::Disband(CUser *pLeader /*= nullptr*/)

void CKnights::Disband(CUser *pLeader /*= nullptr*/)
{

 string clanNotice;
	g_pMain->GetServerResource(m_byFlag == (uint8)ClanTypeFlag::ClanTypeTraining ? IDS_CLAN_DESTROY : IDS_KNIGHTS_DESTROY, &clanNotice, m_strName.c_str());
	SendChat(clanNotice.c_str());

	std::vector<std::string> m_temparKnightsUser;

	m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(i, m_arKnightsUser)
	{
		if (i->second == nullptr)
			continue;

		m_temparKnightsUser.push_back(i->second->strUserName);
	}
	m_arKnightsUser.m_lock.unlock();

	foreach(i, m_temparKnightsUser)
	{
		CUser * pUser = g_pMain->GetUserPtr((*i), NameType::TYPE_CHARACTER);
		// If the user's logged in, handle the player data removal in-game.
		// It will be saved to the database when they log out.
		if (pUser != nullptr)
			RemoveUser(pUser);
		else
			RemoveUser((*i));

		if (pUser != nullptr)
		{
			ClanPremiumNoticeExits(pUser);
			CastellanCapeNoticeExits(pUser);

			// xtreme ekleme 05.02.2024
			//pUser->CharacterOlduguYerdeYenile();
			if (pUser->GetLevel() < 51)
				pUser->CharacterOlduguYerdeYenile();// 17.04.2024 clan paket d�zenlemesi

		}
	}

	if (pLeader != nullptr)
	{
		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_DESTROY));
		result << uint8(1);
		pLeader->Send(&result);

		if (pLeader->isKing()) {
			Packet capes(WIZ_CAPE);
			capes << uint16(1) << uint16(0) << uint16(0);
			if (pLeader->GetNation() == (uint8)Nation::ELMORAD) capes << uint16(KingCapeType::KNIGHTS_HUMAN_KING_CAPE);
			else capes << uint16(KingCapeType::KNIGHTS_KARUS_KING_CAPE);
			capes << uint16(0) << uint16(0);
			pLeader->Send(&capes);
		}
	}

	if (g_pMain->pSiegeWar.sMasterKnights == m_sIndex) {
		g_pMain->pSiegeWar.sMasterKnights = 0;
		g_pMain->UpdateSiege(g_pMain->pSiegeWar.sCastleIndex, g_pMain->pSiegeWar.sMasterKnights, g_pMain->pSiegeWar.bySiegeType, 0, 0, 0);
	}
		
	g_pMain->m_KnightsArray.DeleteData(m_sIndex);
}
#pragma endregion 


#pragma region CKnights::SendChat(const char * format, ...)
void CKnights::SendChat(const char * format, ...)
{
	char buffer[128];
	va_list ap;
	va_start(ap, format);
	vsnprintf(buffer, 128, format, ap);
	va_end(ap);

	Packet result;
	ChatPacket::Construct(&result, (uint8)ChatType::KNIGHTS_CHAT, buffer);
	Send(&result);
}

#pragma endregion 

#pragma region CKnights::SendChatAlliance(const char * format, ...)

void CKnights::SendChatAlliance(const char * format, ...)
{
	char buffer[128];
	va_list ap;
	va_start(ap, format);
	vsnprintf(buffer, 128, format, ap);
	va_end(ap);

	Packet result;
	ChatPacket::Construct(&result, (uint8)ChatType::ALLIANCE_CHAT, buffer);
	Send(&result);
}

#pragma endregion 

#pragma region CKnights::Send(Packet *pkt)

void CKnights::Send(Packet *pkt)
{
	m_arKnightsUser.m_lock.lock();
	auto m_temparKnightsUser = m_arKnightsUser.m_UserTypeMap;
	m_arKnightsUser.m_lock.unlock();

	foreach(user, m_temparKnightsUser) {
		if (!user->second)
			continue;

		CUser* pUser = g_pMain->GetUserPtr(user->second->strUserName, NameType::TYPE_CHARACTER);
		if (pUser == nullptr || !pUser->isInGame() || GetID() != pUser->GetClanID())
			continue;

		pUser->Send(pkt);
	}
}

#pragma endregion 

#pragma region CKnights::GetKnightsAllMembers(uint16 sClanID, Packet & result, uint16 & pktSize, bool bClanLeader)
/**
*
*/
uint16 CKnights::GetKnightsAllMembers(Packet & result, uint16 & pktSize, CUser *pUser)
{
	uint16 count = 0;
	result.DByte();
	if (pUser && pUser->isInAutoClan()) {
		std::string CharName = pUser->GetName();
		STRTOUPPER(CharName);
		_KNIGHTS_USER* pKnightUser = m_arKnightsUser.GetData(CharName);
		if (pKnightUser) {
			result << pUser->GetName() << pUser->GetFame() << uint8(0)
				<< pUser->GetLevel() << pUser->GetClass() << uint8(1) << pKnightUser->strMemo
				<< uint32(0);
			count++;
		}
	}
	else {
		std::vector<_KNIGHTS_USER> m_userlist;
		m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(i, m_arKnightsUser) {
			if (count >= MAX_CLAN_USERS)
				break;

			if (i->second)
				m_userlist.push_back(*i->second);

			count++;
		}
		m_arKnightsUser.m_lock.unlock();

		foreach(itr, m_userlist) {
			CUser* pTUser = g_pMain->GetUserPtr(itr->strUserName, NameType::TYPE_CHARACTER);
			if (pTUser != nullptr)
				result << pTUser->GetName() << pTUser->GetFame() << uint8(0) << pTUser->GetLevel() << pTUser->m_sClass << uint8(1) << itr->strMemo << uint32(0);
			else// normally just clan leaders see this, but we can be generous now.
				result << itr->strUserName << uint8(itr->bFame) << uint8(0) << uint8(itr->bLevel) << uint16(itr->sClass) << uint8(0) << itr->strMemo << (int32(UNIXTIME) - itr->nLastLogin) / 3600;
		}
	}
	return count;
}
#pragma endregion 

#pragma region CKnights::~CKnights()

CKnights::~CKnights()
{
}

#pragma endregion 

// Clan Premium
void CKnights::CheckKnightBuffSystem()
{

	if (!g_pMain->ClanBuffSystemStatus)
		return;

	uint8 Members = GetOnlineMembers();

	if (Members < 5)
	{
		ClanBuffExpBonus = 0;
		ClanBuffNPBonus = 0;
	}

	if (Members >= 5 && Members < 10)
	{
		ClanBuffExpBonus = g_pMain->ClanBuff5UserOnlineExp;
		ClanBuffNPBonus = g_pMain->ClanBuff5UserOnlineNP;
	}
	else if (Members >= 10 && Members < 15)
	{
		ClanBuffExpBonus = g_pMain->ClanBuff10UserOnlineExp;
		ClanBuffNPBonus = g_pMain->ClanBuff10UserOnlineNP;
	}
	else if (Members >= 15 && Members < 20)
	{
		ClanBuffExpBonus = g_pMain->ClanBuff15UserOnlineExp;
		ClanBuffNPBonus = g_pMain->ClanBuff15UserOnlineNP;
	}
	else if (Members >= 20 && Members < 25)
	{
		ClanBuffExpBonus = g_pMain->ClanBuff20UserOnlineExp;
		ClanBuffNPBonus = g_pMain->ClanBuff20UserOnlineNP;
	}
	else if (Members >= 25 && Members < 30)
	{
		ClanBuffExpBonus = g_pMain->ClanBuff25UserOnlineExp;
		ClanBuffNPBonus = g_pMain->ClanBuff25UserOnlineNP;
	}
	else if (Members >= 30)
	{
		ClanBuffExpBonus = g_pMain->ClanBuff30UserOnlineExp;
		ClanBuffNPBonus = g_pMain->ClanBuff30UserOnlineNP;
	}

	std::vector<std::string> mlist;

	m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(i, m_arKnightsUser)
	{
		_KNIGHTS_USER *p = i->second;
		if (p == nullptr)
			continue;

		mlist.push_back(p->strUserName);
	}
	m_arKnightsUser.m_lock.unlock();

	foreach(itr, mlist)
	{
		CUser* Oyuncu = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);

		if (Oyuncu == nullptr)
			continue;

		Oyuncu->m_ClanExpBonus = ClanBuffExpBonus;
		Oyuncu->m_ClanNPBonus = ClanBuffNPBonus;
	}

	std::string header = string_format("EXP +%d%%|NP +%d%%", ClanBuffExpBonus, ClanBuffNPBonus);
	Packet notice(WIZ_NOTICE);
	notice.DByte();
	notice << uint8(4) << uint8(1) << "Clan Buff" << header;
	Send(&notice);

}

uint8 CKnights::GetOnlineMembers()
{
	uint8 OnlinePlayer = 0;
	std::vector<std::string> mlist;
	m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(i, m_arKnightsUser)
	{
		_KNIGHTS_USER *p = i->second;
		if (p == nullptr)
			continue;

		mlist.push_back(p->strUserName);
	}
	m_arKnightsUser.m_lock.unlock();

	foreach(itr, mlist)
	{
		CUser* Oyuncu = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (Oyuncu == nullptr || !Oyuncu->isInGame())
			continue;

		OnlinePlayer++;
	}

	if (OnlinePlayer == 0)
		return 0;

	return OnlinePlayer;
}

void CGameServerDlg::UpdateFlagAndCape()
{
	foreach_stlmap(itr, m_KnightsArray) {
		if (itr->second->m_byFlag >= (uint8)ClanTypeFlag::ClanTypeAccredited5) itr->second->m_byGrade = 1;
		else itr->second->m_byGrade = g_pMain->GetKnightsGrade(itr->second->m_nPoints);
		if (itr->second->m_byFlag == (uint8)ClanTypeFlag::ClanTypePromoted && itr->second->m_byGrade > 3) {
			itr->second->m_byFlag = (uint8)ClanTypeFlag::ClanTypeTraining;
			itr->second->m_sCape = -1;

			Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_UPDATE_GRADE));
			result << itr->second->m_sIndex << itr->second->m_byFlag << itr->second->m_sCape;
			g_pMain->AddDatabaseRequest(result);

		}
	}
}