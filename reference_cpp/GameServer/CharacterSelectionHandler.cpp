#include "stdafx.h"
#include "DBAgent.h"
#include "KingSystem.h"
#include "../GameServer/GameDefine.h"

#define ARRANGE_LINE	800444000

using std::string;

void CUser::SelNationToAgent(Packet & pkt)
{
	Packet result(WIZ_SEL_NATION);
	uint8 nation = pkt.read<uint8>();
	if (nation != (uint8)Nation::KARUS && nation != (uint8)Nation::ELMORAD)
	{
		result << uint8(0);
		Send(&result);
		return;
	}

	result << nation;
	g_pMain->AddDatabaseRequest(result, this);
}

#pragma region
void CUser::AllCharInfo(Packet &pkt)
{
	uint8 opcode = pkt.read<uint8>();
	switch (opcode)
	{
	case 1:
		AllCharInfoToAgent();
		break;
	case 2:
	/*	AllCharNameChangeInfo(pkt); // DeaFSezo nick değiştirme fix 28.08.2024
		break;*/
	case 3:
		AllCharLocationSendToAgent(pkt);
		break;
	case 4:
		AllCharLocationRecvToAgent(pkt);
		break;
	default:
		TRACE("Unhandle AllCharInfo Opecode %X", opcode);
		printf("Unhandle AllCharInfo Opecode %X", opcode);
		break;
	}
}
#pragma endregion

#pragma region
void CUser::AllCharInfoToAgent()
{
	Packet result(WIZ_ALLCHAR_INFO_REQ, uint8(AllCharInfoOpcode));
	g_pMain->AddDatabaseRequest(result, this);
}
#pragma endregion

#pragma region
void CUser::AllCharNameChangeInfo(Packet & pkt)
{
	uint16 CharRanking;
	string sOldCharID, sNewCharID;
	pkt >> CharRanking >> sOldCharID >> sNewCharID;

	Packet result(WIZ_ALLCHAR_INFO_REQ, uint8(AlreadyCharName));
	if (sNewCharID.empty()
		|| sNewCharID.length() > MAX_ID_SIZE
		|| sOldCharID.empty())
	{
		result << uint8(0);
		Send(&result);
		return;
	}

	result << sOldCharID << sNewCharID;
	g_pMain->AddDatabaseRequest(result, this);
}
#pragma endregion

#pragma region
void CUser::AllCharLocationSendToAgent(Packet & pkt)
{
	Packet result(WIZ_ALLCHAR_INFO_REQ, uint8(ArrangeOpen));
	string strCharID1, strCharID2, strCharID3, strCharID4;

	//g_DBAgent.GetAllCharID(m_strAccountID, strCharID1, strCharID2, strCharID3, strCharID4);

	return;

	if (!g_DBAgent.GetAllCharID(m_strAccountID, strCharID1, strCharID2, strCharID3, strCharID4))
		return;

	if (!isInGame() || isMining() || isFishing()
		|| isMerchanting() || isBuyingMerchant()
		|| isSellingMerchant() || isStoreOpen()
		|| isTrading())
		return;

	if (strCharID1.length() == 0 && strCharID2.length() == 0 && strCharID3.length() == 0 && strCharID4.length() == 0)
		return;

	//g_pMain->AddDatabaseRequest(result, this);

	if (strCharID1.length() != 0)
		result << strCharID1;
	else
		result << "";
	if (strCharID2.length() != 0)
		result << strCharID2;
	else
		result << "";
	if (strCharID3.length() != 0)
		result << strCharID3;
	else
		result << "";
	if (strCharID4.length() != 0)
		result << strCharID4;
	else
		result << "";

	Send(&result);
}
#pragma endregion

#pragma region
void CUser::AllCharLocationRecvToAgent(Packet & pkt)
{
	return;

	/*
	1 = Success.
	1 = Failed.
	3 = Required item on iventory is not available.
	4 = 2 locations have set on one character.
	*/
	enum ResultOpCodes
	{
		Success = 1,
		Failed = 2,
		ItemIsNot = 3,
		TwoLocations = 4,
	};

	Packet result(WIZ_ALLCHAR_INFO_REQ, uint8(ArrangeRecvSend));
	uint8 CharRankingI, CharRankingII, CharRankingIII, CharRankingIIII;
	string strCharID1, strCharID2, strCharID3, strCharID4;
	string ID1, ID2, ID3, ID4;
	pkt >> CharRankingI >> CharRankingII >> CharRankingIII >> CharRankingIIII;

	if (CharRankingI == CharRankingII || CharRankingI == CharRankingIII || CharRankingI == CharRankingIIII)
	{
		result << uint8(TwoLocations);
		Send(&result);
		return;
	}

	if (CharRankingII == CharRankingI || CharRankingII == CharRankingIII || CharRankingII == CharRankingIIII)
	{
		result << uint8(TwoLocations);
		Send(&result);
		return;
	}

	if (CharRankingIII == CharRankingI || CharRankingIII == CharRankingII || CharRankingIII == CharRankingIIII)
	{
		result << uint8(TwoLocations);
		Send(&result);
		return;
	}

	if (CharRankingIIII == CharRankingI || CharRankingIIII == CharRankingII || CharRankingIIII == CharRankingIII)
	{
		result << uint8(TwoLocations);
		Send(&result);
		return;
	}

	if (!g_DBAgent.GetAllCharID(m_strAccountID, strCharID1, strCharID2, strCharID3, strCharID4))
	{
		result << uint8(Failed);
		Send(&result);
		return;
	}

	if (!isInGame()
		|| isTrading() 
		|| isMerchanting() 
		|| isSellingMerchant() 
		|| isBuyingMerchant() 
		|| isStoreOpen() 
		|| isMining() 
		|| isDead() 
		|| isFishing())
	{
		result << uint8(Failed);
		Send(&result);
		return;
	}

	if (strCharID1.length() == 0 
		&& strCharID2.length() == 0 
		&& strCharID3.length() == 0 
		&& strCharID4.length() == 0)
	{
		result << uint8(Failed);
		Send(&result);
		return;
	}

	if (!CheckExistItem(ARRANGE_LINE, 1))
	{
		result << uint8(ItemIsNot);
		Send(&result);
		return;
	}

	if (strCharID1.length() != 0)
	{
		switch (CharRankingI)
		{
		case 1:
			ID1 = strCharID1;
			break;
		case 2:
			ID2 = strCharID1;
			break;
		case 3:
			ID3 = strCharID1;
			break;
		case 4:
			ID4 = strCharID1;
			break;
		}
	}
	else
	{
		switch (CharRankingI)
		{
		case 1:
		case 2:
		case 3:
		case 4:
			ID1 = "";
			break;
		}
	}
	if (strCharID2.length() != 0)
	{
		switch (CharRankingII)
		{
		case 1:
			ID1 = strCharID2;
			break;
		case 2:
			ID2 = strCharID2;
			break;
		case 3:
			ID3 = strCharID2;
			break;
		case 4:
			ID3 = strCharID2;
			break;
		}
	}
	else
	{
		switch (CharRankingII)
		{
		case 1:
		case 2:
		case 3:
		case 4:
			ID2 = "";
			break;
		}
	}
	if (strCharID3.length() != 0)
	{
		switch (CharRankingIII)
		{
		case 1:
			ID1 = strCharID3;
			break;
		case 2:
			ID2 = strCharID3;
			break;
		case 3:
			ID3 = strCharID3;
			break;
		case 4:
			ID4 = strCharID3;
			break;
		}
	}
	else
	{
		switch (CharRankingIII)
		{
		case 1:
		case 2:
		case 3:
		case 4:
			ID3 = "";
			break;
		}
	}

	if (strCharID4.length() != 0)
	{
		switch (CharRankingIIII)
		{
		case 1:
			ID1 = strCharID4;
			break;
		case 2:
			ID2 = strCharID4;
			break;
		case 3:
			ID3 = strCharID4;
			break;
		case 4:
			ID4 = strCharID4;
			break;
		}
	}
	else
	{
		switch (CharRankingIIII)
		{
		case 1:
		case 2:
		case 3:
		case 4:
			ID4 = "";
			break;
		}
	}

	if (!g_DBAgent.SetAllCharID(m_strAccountID, ID1, ID2, ID3, ID4))
	{
		result << uint8(Failed);
		Send(&result);
		return;
	}

	if (!RobItem(ARRANGE_LINE, 1))
	{
		result << uint8(Failed);
		Send(&result);
		return;
	}

	result << uint8(Succes);
	Send(&result);
}
#pragma endregion

void CUser::ChangeHair(Packet & pkt)
{
	std::string strUserID;
	uint32 nHair;
	uint8 bOpcode, bFace;
	Packet result(WIZ_CHANGE_HAIR);

	// The opcode:
	// 0 seems to be an in-game implementation for converting from old -> new hair data
	// 2 seems to be used with the hair change item(?).

	// Another note: there's 4 bytes at the end of the packet data that I can't account for (maybe a[nother] checksum?)

	pkt.SByte();
	pkt >> bOpcode >> strUserID >> bFace >> nHair;
	result.SByte();
	if(bOpcode == 1)
		result << bOpcode << m_strUserID << bFace << nHair;
	else
		result << bOpcode << strUserID << bFace << nHair;

	g_pMain->AddDatabaseRequest(result, this);
}

void CUser::NewCharToAgent(Packet & pkt)
{
	Packet result(WIZ_NEW_CHAR);
	uint32 nHair;
	uint16 sClass;
	uint8 bCharIndex, bRace, bFace, str, sta, dex, intel, cha, errorCode = 0;
	std::string strUserID;

	pkt >> bCharIndex >> strUserID >> bRace >> sClass >> bFace >> nHair
		>> str >> sta >> dex >> intel >> cha;

	_CLASS_COEFFICIENT* p_TableCoefficient = g_pMain->m_CoefficientArray.GetData(sClass);

	if (bCharIndex > 3)
		errorCode = NEWCHAR_NO_MORE;
	else if (p_TableCoefficient == nullptr
		|| (str + sta + dex + intel + cha) > 300
		|| !NewCharValid(bRace, sClass))
		errorCode = NEWCHAR_INVALID_DETAILS;
	else if (!NewCharClassVaid(sClass))
		errorCode = NEWCHAR_INVALID_CLASS;
	else if (!NewCharRaceVaid(bRace))
		errorCode = NEWCHAR_INVALID_RACE;
	else if ((str + sta + dex + intel + cha) < 300)
		errorCode = NEWCHAR_POINTS_REMAINING;
	else if (str < 50 || sta < 50 || dex < 50 || intel < 50 || cha < 50)
		errorCode = NEWCHAR_STAT_TOO_LOW;
	else if (strUserID.empty() || strUserID.length() > MAX_ID_SIZE)
		errorCode = NEWCHAR_INVALID_NAME;

	//if (bRace == 6 || bRace == 14)
	//errorCode = NEWCHAR_NOT_SUPPORTED_RACE;

	if (errorCode != 0)
	{
		result << errorCode;
		Send(&result);
		return;
	}

	result << bCharIndex
		<< strUserID << bRace << sClass << bFace << nHair
		<< str << sta << dex << intel << cha;
	g_pMain->AddDatabaseRequest(result, this);
}

/**
* @brief	Checks whether incoming create character request details
*			are correct.
*/
bool CUser::NewCharRaceVaid(uint16 bRace)
{
	bool Result = false;

	switch (bRace)
	{
	case 1:
	case 2:
	case 3:
	case 4:
	case 6:
	case 11:
	case 12:
	case 13:
	case 14:
		Result = true;
		break;
	default:
		Result = false;
		break;
	}
	return Result;
}

/**
* @brief	Checks whether incoming create character request details
*			are correct.
*/
bool CUser::NewCharClassVaid(uint16 bClass)
{
	bool Result = false;

	switch (bClass)
	{
	case 101:
	case 102:
	case 103:
	case 104:
	case 105:
	case 107:
	case 109:
	case 111:
	case 106:
	case 108:
	case 110:
	case 112:
	case 201:
	case 202:
	case 203:
	case 204:
	case 205:
	case 207:
	case 209:
	case 211:
	case 206:
	case 208:
	case 210:
	case 212:
	case 113:
	case 114:
	case 115:
	case 213:
	case 214:
	case 215:
		Result = true;
		break;
	default:
		Result = false;
		break;
	}

	return Result;
}

/**
* @brief	Checks whether incoming create character request details
*			are correct.
*/
bool CUser::NewCharValid(uint8 bRace, uint16 bClass)
{
	bool Result = false;
	switch (bRace)
	{
	case 1:
		if (bClass == 101)
			Result = true;
		break;
	case 2:
		if (bClass == 102 || bClass == 104)
			Result = true;
		break;
	case 3:
		if (bClass == 103)
			Result = true;
		break;
	case 4:
		if (bClass == 103 || bClass == 104)
			Result = true;
		break;
	case 6:
		if (bClass == 113)
			Result = true;
		break;
	case 11:
		if (bClass == 201)
			Result = true;
		break;
	case 12:
	case 13:
		if (bClass == 201
			|| bClass == 202
			|| bClass == 203
			|| bClass == 204)
			Result = true;
		break;
	case 14:
		if (bClass == 213)
			Result = true;
		break;
	default:
		Result = false;
		break;
	}
	return Result;
#if 0
	bool Result = false;
	switch (bRace)
	{
	case 1:
		if (bClass == 101 || bClass == 105 || bClass == 106)
			Result = true;
		break;
	case 2:
		if (bClass == 102 || bClass == 104 || bClass == 107
			|| bClass == 111 || bClass == 108 || bClass == 112)
			Result = true;
		break;
	case 3:
		if (bClass == 103 || bClass == 109 || bClass == 110)
			Result = true;
		break;
	case 4:
		if (bClass == 103 || bClass == 104 || bClass == 109
			|| bClass == 111 || bClass == 110 || bClass == 112)
			Result = true;
		break;
	case 6:
		if (bClass == 113 || bClass == 114 || bClass == 115)
			Result true;
		break;
	case 14:
		if (bClass == 213 || bClass == 214 || bClass == 215)
			Result true;
		break;
	case 11:
		if (bClass == 201 || bClass == 205 || bClass == 206)
			Result = true;
		break;
	case 12:
	case 13:
		if (bClass == 201 || bClass == 205 || bClass == 206
			|| bClass == 202 || bClass == 207 || bClass == 208
			|| bClass == 203 || bClass == 209 || bClass == 210
			|| bClass == 204 || bClass == 211 || bClass == 212)
			Result = true;
		break;
	default:
		Result = false;
		break;
	}
	return Result;
#endif
}

void CUser::DelCharToAgent(Packet & pkt)
{
	Packet result(WIZ_DEL_CHAR);
	std::string strUserID, strSocNo;
	uint8 bCharIndex;
	pkt >> bCharIndex >> strUserID >> strSocNo; 

	if (bCharIndex > 2
		|| strUserID.empty() || strUserID.size() > MAX_ID_SIZE
		|| strSocNo.empty() || strSocNo.size() > 15
		|| isClanLeader())
	{
		result << uint8(0) << uint8(-1);
		Send(&result);
		return;
	}

	// Process the deletion request in the database
	result	<< bCharIndex << strUserID << strSocNo;
	g_pMain->AddDatabaseRequest(result, this);
}

void CUser::SelCharToAgent(Packet & pkt)
{
	Packet result(WIZ_SEL_CHAR);
	std::string strUserID, strAccountID;
	uint8 bInit;

	pkt >> strAccountID >> strUserID >> bInit;
	//printf("Character Selection Sign accountid=%s - charactername= %s\n", strAccountID.c_str(), strUserID.c_str());
	if (strAccountID.empty() || strAccountID.size() > MAX_ID_SIZE
		|| strUserID.empty() || strUserID.size() > MAX_ID_SIZE
		|| strAccountID != m_strAccountID) 
		return goDisconnect("invalid account name or player name.", __FUNCTION__);

	if (g_pMain->m_Shutdownstart) { result << uint8(0); Send(&result); return; }

	CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser && (pUser->GetSocketID() != GetSocketID() && GetAccountName() == pUser->GetAccountName()))
		return pUser->goDisconnect("The account is already in the game, kicking the person in the game from the game.", __FUNCTION__);
	else if (pUser && (GetAccountName() != pUser->GetAccountName() && strUserID == pUser->GetName()))
		return goDisconnect("different account name but same player name.", __FUNCTION__);

	result << strAccountID << strUserID << bInit;
	g_pMain->AddDatabaseRequest(result, this);
}

void CUser::SelectCharacter(Packet & pkt)
{
	Packet result(WIZ_SEL_CHAR);
	uint8 bResult, bInit;

	pkt >> bResult >> bInit;

	if (m_bSelectedCharacter)
		return goDisconnect("He has made a character selection before.", __FUNCTION__);

	CUser* pUser = g_pMain->GetUserPtr(GetAccountName(), NameType::TYPE_ACCOUNT);
	if (pUser && (pUser->GetSocketID() != GetSocketID())) {
		goDisconnect("select character ch ingame diff sockid.", __FUNCTION__);
		pUser->goDisconnect("select character ch ingame diff sockid.", __FUNCTION__);
		return;
	}

	if (m_bSelectedCharacter && (pUser && pUser->GetState() != GameState::GAME_STATE_CONNECTED)) {
		goDisconnect("select character ch ingame diff sockid.", __FUNCTION__);
		pUser->goDisconnect("select character ch ingame diff sockid.", __FUNCTION__);
		return;
	}

	if ((pUser && pUser->m_bSelectedCharacter))
		return goDisconnect("JstKO | DC [1].", __FUNCTION__);

	switch (bResult)
	{
	case 0:
		printf("Cannot Select Character Case 0\n");
		result << uint8(CannotSelectingCharacter);
		Send(&result);
		break;
	case 1:
	{
		m_pMap = g_pMain->GetZoneByID(GetZoneID());
		if (GetMap() == nullptr) { result << uint8(CannotSelectingCharacter); Send(&result); return; }
		if (GetLevel() > g_pMain->m_byMaxLevel)
			return goDisconnect("own level higher than system level.", __FUNCTION__);

		bool ishome = false; uint8 zoneid = 0;
		if (GetZoneID() == ZONE_ELMORAD && !g_pMain->m_byElmoradOpenFlag && GetNation() == (uint8)Nation::KARUS) { ishome = true; zoneid = ZONE_MORADON; }

		if (!ishome && GetZoneID() == ZONE_KARUS && !g_pMain->m_byKarusOpenFlag && GetNation() == (uint8)Nation::ELMORAD) { ishome = true; zoneid = ZONE_MORADON; }
		if (!ishome && GetMap()->isWarZone() && !g_pMain->isWarOpen()) { ishome = true; zoneid = ZONE_MORADON; }
		if (!ishome && GetMap()->isWarZone() && g_pMain->isWarOpen() && g_pMain->m_bVictory != 0 && g_pMain->m_bVictory != GetNation()) { ishome = true; zoneid = ZONE_MORADON; }
		if (!ishome && (g_pMain->pCindWar.isON() || isInTotalTempleEventZone() || (isOpenWarZoneKickOutOtherZone() && g_pMain->isWarOpen()))) { ishome = true; zoneid = ZONE_MORADON; }
		if (!ishome && GetZoneID() == ZONE_DELOS && g_pMain->m_byBattleSiegeWarOpen && !GetClanID()) { ishome = true; zoneid = ZONE_MORADON; }
		if (!ishome && isInSpecialEventZone() && !g_pMain->pSpecialEvent.opened) { ishome = true; zoneid = ZONE_MORADON; }
		if (!ishome && (GetZoneID() == ZONE_STONE1 || GetZoneID() == ZONE_STONE2 || GetZoneID() == ZONE_STONE3)) { ishome = true; zoneid = ZONE_MORADON; }

		if (!ishome && g_pMain->pSpecialEvent.opened && isInPKZone()) { 
			ishome = true; zoneid = ZONE_MORADON; 
		}

		if (!ishome && (isPartyTournamentinZone() || isClanTournamentinZone())) {
			auto* TournamentClanInfo = g_pMain->m_ClanVsDataList.GetData(GetZoneID());
			if ((TournamentClanInfo == nullptr) || (TournamentClanInfo->aTournamentClanNum[0] != GetClanID()
				&& TournamentClanInfo->aTournamentClanNum[1] != GetClanID())) {
				ishome = true; zoneid = ZONE_MORADON;
			}
		}

		if (!ishome && GetZoneID() == ZONE_BIFROST) {
			if (!g_pMain->pBeefEvent.isActive || !g_pMain->pBeefEvent.isMonumentDead) { ishome = true; zoneid = ZONE_MORADON; }
			if (g_pMain->pBeefEvent.isActive && !g_pMain->pBeefEvent.isFarmingPlay) { ishome = true; zoneid = ZONE_MORADON; }
			if (g_pMain->pBeefEvent.isActive && g_pMain->pBeefEvent.isFarmingPlay && g_pMain->pBeefEvent.WictoryNation != GetNation() && !g_pMain->pBeefEvent.isLoserSign) { ishome = true; zoneid = ZONE_MORADON; }
		}

		if (!ishome && g_pMain->isCindirellaZone(GetZoneID())) { ishome = true; zoneid = ZONE_MORADON; }

		if (!ishome && GetZoneID() == ZONE_DELOS && !csw_canenterdelos()) {
			ishome = true; zoneid = ZONE_MORADON;
		}

		if (ishome) {
			if (zoneid) m_bZone = zoneid;
			else m_bZone = ZONE_MORADON;
			short x = 0, z = 0;
			bool test = GetStartPosition(x, z, zoneid);
			if (test) { m_curx = (float)x; m_curz = (float)z; }
			else {
#if(GAME_SOURCE_VERSION == 2369)
				m_bZone = ZONE_MORADON; 
				m_curx = (float)81400 / 100.0f; 
				m_cury = (float)469 / 100.0f;
				m_curz = (float)43750 / 100.0f;
#else
				m_bZone = ZONE_MORADON;
				m_curx = (float)26700 / 100.0f;
				m_cury = (float)790 / 100.0f;
				m_curz = (float)30300 / 100.0f;
#endif
			}
			m_pMap = g_pMain->GetZoneByID(GetZoneID());
			if (GetMap() == nullptr) { result << uint8(CannotSelectingCharacter); Send(&result); return; }
		}
		
		if (!g_pMain->isWarOpen() && GetFame() == COMMAND_CAPTAIN)
			m_bFame = CHIEF;

		SetLogInInfoToDB(bInit);
		SetUserAbility(false);

		m_iMaxExp = g_pMain->GetExpByLevel(GetLevel(), GetRebirthLevel());

		SetRegion(GetNewRegionX(), GetNewRegionZ());

		result << bResult << GetZoneID() << GetSPosX() << GetSPosZ() << GetSPosY() << GetNation();
		m_bSelectedCharacter = true;
		Send(&result);

		if (GetClanID() <= NO_CLAN)
		{
			SetClanID(NO_CLAN);
			m_bFame = 0;
			return;
		}
		else if (GetClanID() != 0 && GetZoneID() > 2)
		{
			result.clear();
			result.Initialize(WIZ_KNIGHTS_PROCESS);
			result << uint8(KnightsPacket::KNIGHTS_LIST_REQ) << GetClanID();
			g_pMain->AddDatabaseRequest(result, this);
		}
	}	
	break;
	case 2:
		result << uint8(AccountBlocked);
		Send(&result);
		break;
	case 3:
		result << uint8(AlreadyCharacterName);
		Send(&result);
		break;
	case 4:
	{
		string strCharID1, strCharID2, strCharID3, strCharID4, strPrısonID;
		uint8 ZoneID = 0;

		if (!g_DBAgent.GetAllCharID(m_strAccountID, strCharID1, strCharID2, strCharID3, strCharID4)) {
			printf("Cannot Select Character Case 4\n");
			result << uint8(CannotSelectingCharacter);
			Send(&result);
			return;
		}

		result.SByte();
		
		if (strCharID1.length() != 0)
		{
			if (g_DBAgent.CharacterSelectPrisonCheck(strCharID1, ZoneID))
			{
				if (ZoneID == ZONE_PRISON)
				{
					result << uint8(PrisonCharacter) << strCharID1 << "JstKO - Keeper";
					Send(&result);
					return;
				}
			}
		}
		if (strCharID2.length() != 0)
		{
			if (g_DBAgent.CharacterSelectPrisonCheck(strCharID2, ZoneID))
			{
				if (ZoneID == ZONE_PRISON)
				{
					result << uint8(PrisonCharacter) << strCharID2 << "JstKO - Keeper";
					Send(&result);
					return;
				}
			}
		}
		if (strCharID3.length() != 0)
		{
			if (g_DBAgent.CharacterSelectPrisonCheck(strCharID3, ZoneID))
			{
				if (ZoneID == ZONE_PRISON)
				{
					result << uint8(PrisonCharacter) << strCharID3 << "JstKO - Keeper";
					Send(&result);
					return;
				}
			}
		}
		if (strCharID4.length() != 0)
		{
			if (g_DBAgent.CharacterSelectPrisonCheck(strCharID4, ZoneID))
			{
				if (ZoneID == ZONE_PRISON)
				{
					result << uint8(PrisonCharacter) << strCharID4 << "JstKO - Keeper";
					Send(&result);
					return;
				}
			}
		}
	}
	break;
	case 5:
		result << uint8(MaintanenceMode);
		Send(&result);
		break;
	case 6:
	{
		Disconnect();
		return;
	}
	break;
	default:
		printf("Unkown case switch SelectCharacter %d\n", bResult);
		//result << uint8(CannotSelectingCharacter);
		//Send(&result);
		break;
	}
}

void CUser::SendServerChange(std::string & ip, uint8 bInit)
{
	Packet result(WIZ_SERVER_CHANGE);
	result << ip << uint16(g_pMain->m_GameServerPort) << bInit << GetZoneID() << GetNation();
	Send(&result);
}

// happens on character selection
void CUser::SetLogInInfoToDB(uint8 bInit)
{
	_ZONE_SERVERINFO *pInfo = g_pMain->m_ServerArray.GetData(g_pMain->m_nServerNo);
	if (pInfo == nullptr)
		return goDisconnect("serverno not exits", __FUNCTION__);

	Packet result(WIZ_LOGIN_INFO);
	result	<< GetName() 
		<< pInfo->strServerIP << uint16(g_pMain->m_GameServerPort) << GetRemoteIP() 
		<< bInit;
	g_pMain->AddDatabaseRequest(result, this);
}

#pragma region CUser::RecvLoginInfo(Packet & pkt)
void CUser::RecvLoginInfo(Packet & pkt)
{
	int8 bResult = pkt.read<uint8>();
	if (bResult < 0)
		return goDisconnect("b result is false.", __FUNCTION__);
}
#pragma endregion
#pragma region CUser::SendCrRandomList()
void CUser::SendCrRandomList() {

	uint16 counter = 0;
	uint8 randomid = 0;
	Packet test(XSafe, uint8(XSafeOpCodes::SendAntiAfk));
	test << uint8(4) << counter;

	std::map <uint16, _RANDOM_ITEM*> nRandom;

	g_pMain->m_CrRandomListLock.lock();
	foreach(itr, g_pMain->m_RandomItemArray)
	{
		_RANDOM_ITEM* pRandom = *itr;

		if (pRandom == nullptr)
		{
			continue;
		}

		counter++;
		test << pRandom->SessionID << pRandom->ItemID;
		//if (pRandom->SessionID == 3)
		//	printf("SendSoloMobList pRandom->SessionID: %d || pRandom->ItemID: %d \n", pRandom->SessionID, pRandom->ItemID);

	}
	g_pMain->m_CrRandomListLock.unlock();

	//foreach(itr, copylist) {
	//	//printf("SendSoloMobList *itr: %d \n", *itr);
	//	test << (*itr);
	//	counter++;
	//}
	test.put(2, counter);
	Send(&test);
}
#pragma endregion
void CUser::SendLists() {
	SendAntiAfkList();
	SendWheelData();
	DailyQuestSendList();
	PusRefundSendList();
	SendEventTimerList();
	SendRightClickExchange();
	SendCrRandomList();
}

void CUser::SendEventTimerList() {
	
	if (g_pMain->timershowreset) return;

	uint32 nHour = g_localTime.tm_hour, nMinute = g_localTime.tm_min, nSeconds = g_localTime.tm_sec;
	
	Packet newpkt(XSafe, uint8(XSafeOpCodes::EventShowList));

	g_pMain->m_timershowlistLock.lock();
	newpkt << nHour << nMinute << nSeconds << (uint16)g_pMain->m_timershowlist.size();
	foreach(itr, g_pMain->m_timershowlist) newpkt << itr->name << itr->hour << itr->minute;
	g_pMain->m_timershowlistLock.unlock();
	Send(&newpkt);
}

void CUser::SetFlashTimeNote(bool remove) {

	if (remove) {
		if (m_FlashExpBonus > 0 || m_FlashDcBonus > 0 || m_FlashWarBonus > 0) {
			m_flashtime = 0; 
			m_flashcount = 0;
			m_flashtype = 0;
			SendFlashNotice(true);
			if (m_FlashExpBonus > 0) m_FlashExpBonus = 0;
			if (m_FlashDcBonus > 0) m_FlashDcBonus = 0;
			if (m_FlashWarBonus > 0) m_FlashWarBonus = 0;
			CMagicProcess::RemoveType4Buff(BUFF_TYPE_FISHING, this, true);
		}
		return;
	}

	bool validprem = GetPremium() >= 10 && GetPremium() <= 12 || GetPremium() == 7;
	if (!m_flashtime || !validprem /*|| !g_pMain->pServerSetting.flashtime*/)
		return;


	if (m_flashcount > 10) 
		m_flashcount = 10;

	if (GetPremium() == 10 || m_flashtype == 2)
		m_FlashDcBonus = m_flashcount * 10;
	else if (GetPremium() == 11 || m_flashtype == 1)
		m_FlashExpBonus = m_flashcount * 10;
	else if (GetPremium() == 12 || m_flashtype == 3)
		m_FlashWarBonus = m_flashcount;

	SendFlashNotice();
}

void CUser::FlashUpdateTime(uint32 time) {

	SendFlashNotice(true);
	if (!time) {
		if (m_FlashExpBonus > 0) m_FlashExpBonus = 0;
		if (m_FlashDcBonus > 0) m_FlashDcBonus = 0;
		if (m_FlashWarBonus > 0) m_FlashWarBonus = 0;
		CMagicProcess::RemoveType4Buff(BUFF_TYPE_FISHING, this, true);
	}
	SendFlashNotice();
}

void CUser::UserInfoStorage(bool login) {

	std::string strUserID = GetName();
	STRTOUPPER(strUserID);

	Guard lock(g_pMain->m_InfoStorMapLock);
	auto itr = g_pMain->m_InfoStorMap.find(strUserID);
	if (itr == g_pMain->m_InfoStorMap.end()) {
		if (login)
			g_pMain->m_InfoStorMap.insert(std::make_pair(strUserID, _USER_INFOSTORAGE()));

		return;
	}

	if (login) {
		if (itr->second.flamebacktime > UNIXTIME)
			m_flameilevel = itr->second.flamelevel;
		return;
	}

	itr->second.flamelevel = m_flameilevel;
	itr->second.flamebacktime = UNIXTIME + 300;
}


void CUser::GameStart(Packet& pkt)
{
	if (isInGame())
		return;

	uint8 opcode = pkt.read<uint8>();
	if (opcode != 1 && opcode != 2)
		return goDisconnect("game start diffrent opcode.", __FUNCTION__);

	std::string recvUserID, strUserID = GetName();

	pkt.SByte();
	pkt >> recvUserID;

	if (opcode == 1)
	{
		if (m_state != GameState::GAME_STATE_CONNECTED)
			return goDisconnect("m_state not none.", __FUNCTION__);

		m_state = GameState::GAME_STATE_CONNECTED;

		if (newChar && g_pMain->pServerSetting.giveGenieHour)
			GenieExchange(0, g_pMain->pServerSetting.giveGenieHour, true);

		UserInfoStorage(true);
		SendMyInfo();

		m_flametime = UNIXTIME + (1 * HOUR);

		if (isGM()) m_bAbnormalType = ABNORMAL_INVISIBLE;
		m_bAchieveLoginTime = (uint32)UNIXTIME;

		HookUStatePanelRequest();

		g_pMain->UserInOutForMe(this);
		NpcInOutForMe();
		g_pMain->MerchantUserInOutForMe(this);

		Packet newpkt(WIZ_STORY);
		newpkt << uint32(0) << uint16(0);
		Send(&newpkt);

		SendNotice();
		TopSendNotice();
		SendCapeBonusNotice();
		SendTime();
		SendWeather();
		QuestDataRequest();
		OpenEtcSkill();
		JstKOSendInfoNotice();
		SendLists();
		RightTopTitleMsg();

		newpkt.clear();
		newpkt.Initialize(WIZ_GAMESTART);
		Send(&newpkt);

		XSafe_ReqUserData();
		xSafe_SendAccountRegister();
		if (isInClan())
		{
			CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
			if (pKnights != nullptr) {
				SendClanPremium(pKnights, false);
				/*std::string userid = GetName();
				STRTOUPPER(userid);

				_KNIGHTS_USER *pKnightUser = pKnights->m_arKnightsUser.GetData(userid);
				if (pKnightUser != nullptr) {
					Packet zaresult(WIZ_KNIGHTS_PROCESS, uint8(0x62));
					zaresult << uint8(pKnights->m_sOnline);
					Send(&zaresult);
				}*/
			}
		}
		auto* TopLeft = g_pMain->m_TopLeftArray.GetData(0x01);
		if (TopLeft != nullptr)
		{
			Packet result(XSafe);
			result << uint8(XSafeOpCodes::TOPLEFT);
			result.DByte();
			result << TopLeft->Facebook << TopLeft->FacebookURL << TopLeft->Discord << TopLeft->DiscordURL << TopLeft->Live << TopLeft->LiveURL;
			result << TopLeft->ResellerURL;
			Send(&result);
		}
	}
	else if (opcode == 2)
	{
		if (m_state != GameState::GAME_STATE_CONNECTED)
			return goDisconnect("m_state not connected.", __FUNCTION__);

		ZoneOnlineRewardStart();
		m_state = GameState::GAME_STATE_INGAME;
		g_pMain->AddCharacterName(this);
		UserInOut(INOUT_RESPAWN);

		SetUserAbility(true);
		GmListProcess(false);
		Send_myPerks();
		//SendTagUserList(this);

		// JstKO Sistem Mesajları Renk Kırmızı Eklendi 02.01.2025
		std::vector< _SEND_MESSAGE> m_messagelist;
		g_pMain->m_SendMessageArray.m_lock.lock();
		foreach_stlmap_nolock(itr, g_pMain->m_SendMessageArray) {
			_SEND_MESSAGE* msg = itr->second;
			if (msg == nullptr || msg->SendType != 1)
				continue;

			uint8 type = (uint8)msg->bChatType; //ChatType 2 Private Chat.
			m_messagelist.push_back(*itr->second);
		}
		g_pMain->m_SendMessageArray.m_lock.unlock();

		foreach(itr, m_messagelist)
		{
			Packet result;
			ChatPacket::Construct(&result, (uint8)itr->bChatType, &(itr->strMessage), &(itr->strSender), GetNation(), -1, GetLoyaltySymbolRank(), (uint8)21); // Sistem Mesajları Renkli Kırmızı Eklendi 
			Send(&result);
		}
		{ // JstKO Sistem Mesajları Renk Sari Eklendi 02.01.2025
			std::vector< _SEND_MESSAGE> m_messagelist;
			g_pMain->m_SendMessageArray.m_lock.lock();
			foreach_stlmap_nolock(itr, g_pMain->m_SendMessageArray) {
				_SEND_MESSAGE* msg = itr->second;
				if (msg == nullptr || msg->SendType != 2)
					continue;

				uint8 type = (uint8)msg->bChatType; //ChatType 2 Private Chat.
				m_messagelist.push_back(*itr->second);
			}
			g_pMain->m_SendMessageArray.m_lock.unlock();

			foreach(itr, m_messagelist)
			{
				Packet result;
				ChatPacket::Construct(&result, (uint8)itr->bChatType, &(itr->strMessage), &(itr->strSender), GetNation(), -1, GetLoyaltySymbolRank(), (uint8)22); // Sistem Mesajları Renkli Sari Eklendi
				Send(&result);
			}
		}
		{ // JstKO Sistem Mesajları Renk Yeşil Eklendi 02.01.2025
			std::vector< _SEND_MESSAGE> m_messagelist;
			g_pMain->m_SendMessageArray.m_lock.lock();
			foreach_stlmap_nolock(itr, g_pMain->m_SendMessageArray) {
				_SEND_MESSAGE* msg = itr->second;
				if (msg == nullptr || msg->SendType != 3)
					continue;

				uint8 type = (uint8)msg->bChatType; //ChatType 2 Private Chat.
				m_messagelist.push_back(*itr->second);
			}
			g_pMain->m_SendMessageArray.m_lock.unlock();

			foreach(itr, m_messagelist)
			{
				Packet result;
				ChatPacket::Construct(&result, (uint8)itr->bChatType, &(itr->strMessage), &(itr->strSender), GetNation(), -1, GetLoyaltySymbolRank(), (uint8)23); // Sistem Mesajları Renkli Yeşil Eklendi
				Send(&result);
			}
		}
		{ // JstKO Sistem Mesajları Renk Pembe Eklendi 02.01.2025
			std::vector< _SEND_MESSAGE> m_messagelist;
			g_pMain->m_SendMessageArray.m_lock.lock();
			foreach_stlmap_nolock(itr, g_pMain->m_SendMessageArray) {
				_SEND_MESSAGE* msg = itr->second;
				if (msg == nullptr || msg->SendType != 4)
					continue;

				uint8 type = (uint8)msg->bChatType; //ChatType 2 Private Chat.
				m_messagelist.push_back(*itr->second);
			}
			g_pMain->m_SendMessageArray.m_lock.unlock();

			foreach(itr, m_messagelist)
			{
				Packet result;
				ChatPacket::Construct(&result, (uint8)itr->bChatType, &(itr->strMessage), &(itr->strSender), GetNation(), -1, GetLoyaltySymbolRank(), (uint8)24); // Sistem Mesajları Renkli Pembe Eklendi
				Send(&result);
			}
		}
		{ // JstKO Sistem Mesajları Renk Mavi Eklendi 02.01.2025
			std::vector< _SEND_MESSAGE> m_messagelist;
			g_pMain->m_SendMessageArray.m_lock.lock();
			foreach_stlmap_nolock(itr, g_pMain->m_SendMessageArray) {
				_SEND_MESSAGE* msg = itr->second;
				if (msg == nullptr || msg->SendType != 5)
					continue;

				uint8 type = (uint8)msg->bChatType; //ChatType 2 Private Chat.
				m_messagelist.push_back(*itr->second);
			}
			g_pMain->m_SendMessageArray.m_lock.unlock();

			foreach(itr, m_messagelist)
			{
				Packet result;
				ChatPacket::Construct(&result, (uint8)itr->bChatType, &(itr->strMessage), &(itr->strSender), GetNation(), -1, GetLoyaltySymbolRank(), (uint8)25); // Sistem Mesajları Renkli Mavi Eklendi
				Send(&result);
			}
		}
		{ // JstKO Sistem Mesajları Renk Mavi Açık Eklendi 02.01.2025
			std::vector< _SEND_MESSAGE> m_messagelist;
			g_pMain->m_SendMessageArray.m_lock.lock();
			foreach_stlmap_nolock(itr, g_pMain->m_SendMessageArray) {
				_SEND_MESSAGE* msg = itr->second;
				if (msg == nullptr || msg->SendType != 6)
					continue;

				uint8 type = (uint8)msg->bChatType; //ChatType 2 Private Chat.
				m_messagelist.push_back(*itr->second);
			}
			g_pMain->m_SendMessageArray.m_lock.unlock();

			foreach(itr, m_messagelist)
			{
				Packet result;
				ChatPacket::Construct(&result, (uint8)itr->bChatType, &(itr->strMessage), &(itr->strSender), GetNation(), -1, GetLoyaltySymbolRank(), (uint8)26); // Sistem Mesajları Renkli Mavi Açık Eklendi
				Send(&result);
			}
		}

		RobChaosSkillItems();// Chaos skilleri inventoryde kaldiginda oyuna girince siler otomatikmen

		if (GetLevel() >= 1 && GetLevel() <= g_pMain->m_byMaxLevel)
			AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveReachLevel, 0, nullptr);

		if (isPortuKurian())
			SpChange(m_MaxSp);

		if (isInClan())
			KnightsClanBuffUpdate(true);

		// If we've relogged while dead, we need to make sure the client 
		// is still given the option to revive.
		if (isDead() && m_bCity) { m_iLostExp = OnDeathLostExpCalc(m_iMaxExp); CheckRespawnScroll(); }
		if (isDead()) SendDeathAnimation();

		g_pMain->TempleEventGetActiveEventTime(this);

		if (g_pMain->pCollectionRaceEvent.isCRActive)
			CollectionGetActiveTime();

		if (GetItem(SHOULDER)->nNum > 0 && GetItem(SHOULDER)->nNum == AUTOMATIC_UPGRADE)
		{
			m_AutoUpgrade = true;
			XSafe_ItemNotice(23, 1);
		}

		InitType4();
		RecastSavedMagic();
		BlinkStart();

		// JstKO Login Effect 01.01.2025
		if (g_pMain->UserLoginEffect)
		PlayerEffect(15005);

		// JstKO Welcome Notice Status Eklendi 01.01.2025
		if (g_pMain->JstKOWelcomeStatus)
			JstKOWelcomeLogos("[JstKO]", g_pMain->WelcomeNoticeYeni, g_pMain->WelcomeNoticeR, g_pMain->WelcomeNoticeG, g_pMain->WelcomeNoticeB);

		if (g_pMain->pServerSetting.OyunUserBotOnlineNotice) // JstKO User&Bot Oyunda Online Notice Eklendi 31.12.2024
			JstKOOyunOnlineNotice();

		if (g_pMain->pServerSetting.GmisOnlineNotice) // JstKO Giriş Gm Online Notice Eklendi 30.12.2024
			JstKOGmisOnlineNotice();

		//JstKO Login User İsOnline Notice 01.01.2025
		if (g_pMain->GirisUserisOnlineNotice)
			JstKOLoginUserisOnlineNotice();

		//JstKO Gm Info Komut Notice 02.01.2025
		if (g_pMain->GmInfoKomutNotice)
			JstKOGmInfoKomutNotice();

		// JstKO Yeni Süre İtemler Info Notice Eklendi. 06.02.2025 
		JstKOExprationItemList();

		m_LastX = GetX();
		m_LastZ = GetZ();

		if (m_flashtime && GetPremium())
			SetFlashTimeNote();

		if (GetFame() == COMMAND_CAPTAIN && isClanLeader()) ChangeFame(CHIEF);

		if (isInSpecialEventZone() && g_pMain->pSpecialEvent.opened) ZindanWarSendScore();

#if(CLIENTLESS)
		int x = myrand(100, 1000);
		int y = myrand(100, 1000);
		Warp(x * 10, y * 10);
#endif

		SendPresetReqMoney();

		if (GetZoneID() == ZONE_DELOS)
			csw_sendwarflag();
	}

	m_tHPLastTimeNormal = UNIXTIME;
	m_tSpLastTimeNormal = UNIXTIME;
	CollectionRaceFirstLoad();
	SendLoginNotice();

	// JstKO Auto King Name Update Eklendi 01.01.2025
	if (isKing() && g_pMain->AutoKingUpdateServerList)

	{
		g_DBAgent.UpdateKingList(m_strUserID, GetNation());

		if (GetNation() == KARUS)
			printf("[King Update] = Karus King Updated : %s\n", m_strUserID.c_str());
		else
			printf("[King Update] = Human King Updated : %s\n", m_strUserID.c_str());

	}
	// JstKO Auto King Name Update Eklendi The End 01.01.2025

	if (g_pMain->isWarOpen())
	{
		g_pMain->m_CommanderArrayLock.lock();
		foreach(itr, g_pMain->m_CommanderArray)
		{
			if (itr->second == nullptr)
				continue;

			if (GetName() == itr->second->strName && GetFame() != COMMAND_CAPTAIN)
			{
				ChangeFame(COMMAND_CAPTAIN);
				break;
			}
		}
		g_pMain->m_CommanderArrayLock.unlock();
	}
}

#pragma region CUser::HookUStatePanelRequest()
void CUser::HookUStatePanelRequest()
{
	XSafe_UIRequest(Packet(XSafe, uint8(XSafeOpCodes::UIINFO)));
}
#pragma endregion

#pragma region CUser::SendRightClickExchange()
void CUser::SendRightClickExchange()
{
	std::vector<uint32> itemList;
	g_pMain->m_RightClickItemListLock.lock();
	itemList = g_pMain->RightClickExchangeItemList;
	g_pMain->m_RightClickItemListLock.unlock();

	const uint32 monsterStones[] = { 300144000, 300145000, 300146000, 300147000, 300148000 };
	for (uint32 stoneId : monsterStones)
	{
		if (std::find(itemList.begin(), itemList.end(), stoneId) == itemList.end())
			itemList.push_back(stoneId);
	}

	if (itemList.empty())
		return;

	Packet pRightClickExchange(XSafe, uint8(XSafeOpCodes::RIGHT_CLICK_EXCHANGE));
	pRightClickExchange << uint8(RightClickExchangeSubcode::SendItemExchangeList) << uint16(itemList.size());
	foreach(itr, itemList)
		pRightClickExchange << uint32(*itr);
	Send(&pRightClickExchange);
}
#pragma endregion


#pragma region CUser::SendAntiAfkList()
void CUser::SendAntiAfkList() {

	g_pMain->m_AntiAfkListLock.lock();
	AntiAfkList copylist = g_pMain->m_AntiAfkList;
	g_pMain->m_AntiAfkListLock.unlock();

	if (copylist.empty())
		return;

	uint16 counter = 0;
	Packet test(XSafe, uint8(XSafeOpCodes::SendAntiAfk));
	test << uint8(1) << counter;
	foreach(itr, copylist) {
		test << (*itr);
		counter++;
	}
	test.put(2, counter);
	Send(&test);
}
#pragma endregion

void CUser::LoadingLogin(Packet &pkt)
{
	uint16 count = 0, remainCount = 0;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		count++;
	}
	
	if (count >= MAX_USER)
	{
		if (g_pMain->m_sLoginingPlayer > 1000)
			g_pMain->m_sLoginingPlayer = 0;

		g_pMain->m_sLoginingPlayer++;
		m_bWaitingOrder = true;
		m_tOrderRemain = UNIXTIME + 10;

		Packet result(WIZ_LOADING_LOGIN, uint8(1));
		result << int32(g_pMain->m_sLoginingPlayer);
		Send(&result);
		return;
	}

	if (g_pMain->m_sLoginingPlayer > 0)
		g_pMain->m_sLoginingPlayer--;

	// Online User Counts
	Packet result(WIZ_LOADING_LOGIN, uint8(1));
	result << int32(g_pMain->m_sLoginingPlayer);
	Send(&result);
}
