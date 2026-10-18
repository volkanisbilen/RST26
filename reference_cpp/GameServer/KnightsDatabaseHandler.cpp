#include "stdafx.h"
#include "../shared/Condition.h"
#include "KnightsManager.h"
#include "DBAgent.h"

using namespace std;

#pragma region CKnightsManager::ReqKnightsPacket(CUser* pUser, Packet & pkt)
/**
* @brief	Handles all clan packets subject to database manipulation.
*
*/
void CKnightsManager::ReqKnightsPacket(CUser* pUser, Packet & pkt)
{
	uint8 opcode;
	pkt >> opcode;
	switch ((KnightsPacket)opcode)
	{
	case KnightsPacket::KNIGHTS_CREATE:
		ReqCreateKnights(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_JOIN:
		ReqKnightsJoin(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_WITHDRAW:
		ReqKnightsLeave(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_REMOVE:
		ReqKnightsRemove(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_DESTROY:
		ReqKnightsDestroy(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_ADMIT:
		ReqKnightsAdmit(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_REJECT:
		ReqKnightsReject(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_PUNISH:
		ReqKnightsPunish(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_CHIEF:
		ReqKnightsChief(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_VICECHIEF:
		ReqKnightsViceChief(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_OFFICER:
		ReqKnightsOfficer(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_MEMBER_REQ:
		ReqKnightsAllMember(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_LIST_REQ:
		//ReqKnightsList(pkt);
		break;
	case KnightsPacket::KNIGHTS_ALLLIST_REQ:
		g_DBAgent.LoadKnightsAllList();
		break;
	case KnightsPacket::KNIGHTS_MARK_REGISTER:
		ReqKnightsRegisterSymbol(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_ALLY_CREATE:
		ReqKnightsAllianceCreate(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_ALLY_INSERT:
		ReqKnightsAllianceInsert(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_ALLY_REMOVE:
		ReqKnightsAllianceRemove(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_ALLY_PUNISH:
		ReqKnightsAlliancePunish(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_UPDATE_GRADE:
		ReqKnightsGradeUpdate(pkt);
		break;
	case KnightsPacket::KNIGHTS_DONATE_POINTS:
		ReqKnightsDonateNP(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_HANDOVER:
		ReqKnightsHandover(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_HANDOVER_REQ:
		break;
	case KnightsPacket::KNIGHTS_POINT_METHOD:
		ReqKnightsPointMethodChange(pUser, pkt);
		break;
	case KnightsPacket::KNIGHTS_UPDATENOTICE:
		ReqKnightsClanNoticeUpdate(pkt);
		break;
	case KnightsPacket::KNIGHTS_REFUND_POINTS:
		ReqKnightsRefundNP(pkt);
		break;
	case KnightsPacket::KNIGHTS_UPDATE_FUND:
		ReqKnightsUpdateNP(pkt);
		break;
	case KnightsPacket::KNIGHTS_UPDATEMEMO:
	{
		uint8 type;
		pkt >> type;
		if (type == 2)
			ReqKnightsAllianceNoticeUpdate(pkt);

		if (type == 3)
			ReqKnightsUserMemoUpdate(pkt);
	}
	break;
	case KnightsPacket::KNIGHTS_UPDATE_USER:
		break;
	case KnightsPacket::KNIGHTS_UPDATE_CAPE:
		ReqKnightsCapeUpdate(pkt);
		break;
	default:
		TRACE("Unhandled clan database opcode: %X\n", opcode);
		printf("Unhandled clan database opcode: %X\n", opcode);
		break;
	}
}
#pragma endregion

#pragma region CKnightsManager::ReqCreateKnights(CUser *pUser, Packet & pkt)
/**
* @brief	Handles clan creation request and all database processes.
*/
void CKnightsManager::ReqCreateKnights(CUser *pUser, Packet & pkt)
{

	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_CREATE));
	string strKnightsName, strChief;
	uint16 sClanID;
	uint8 bFlag, bNation;
	int8 bResult;

	pkt >> bFlag >> sClanID >> bNation >> strKnightsName >> strChief;
	if (pUser == nullptr || pUser->GetName() != strChief || pUser->isInClan())
		return;

	bResult = g_DBAgent.CreateKnights(sClanID, bNation, strKnightsName, strChief, bFlag);

	if (bResult > 0)
	{
		result << bResult;
		pUser->Send(&result);
		return;
	}

	CKnights *pKnights = new CKnights();
	pKnights->m_sIndex = sClanID;
	pKnights->m_byFlag = g_pMain->pServerSetting.AutoRoyalG1 ? (uint8)ClanTypeFlag::ClanTypeRoyal1 : bFlag;
	pKnights->m_byNation = bNation;
	pKnights->m_strName = strKnightsName;
	pKnights->m_strChief = pUser->GetName();
	if (!g_pMain->m_KnightsArray.PutData(pKnights->m_sIndex, pKnights)) {
		delete pKnights;
		return;
	}
	
	pUser->GoldLose(CLAN_COIN_REQUIREMENT, false);

	pKnights->AddUser(pUser);
	pKnights->m_sMembers = 1;
	pKnights->m_bOnlineNembers = 1;
	pUser->m_bFame = CHIEF;
	pUser->AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveBecomeKnightMember, 0, nullptr);

	bool autoroyal = false;
	if (g_pMain->pServerSetting.AutoRoyalG1) autoroyal = true;

	result << uint8(1) << pUser->GetSocketID()
		<< sClanID << strKnightsName
		<< (uint8(autoroyal ? 1 : pKnights->m_byGrade))  << pKnights->m_byRanking
		<< pUser->m_iGold;
	pUser->SendToRegion(&result, nullptr, pUser->GetEventRoom());

	if (autoroyal) {
		CKnightsManager::UpdateKnightsGrade(sClanID, (uint8)ClanTypeFlag::ClanTypePromoted);
		CKnightsManager::UpdateKnightsGrade(sClanID, (uint8)ClanTypeFlag::ClanTypeRoyal1);
		pKnights->m_nClanPointFund = 0;
		pKnights->m_byGrade = 1;
		pKnights->UpdateClanFund();
	}

	//Xtreme 05.02.2024 ekleme
	if (pUser->GetLevel() < 51)
		pUser->CharacterOlduguYerdeYenile();// 17.04.2024 clan paket düzenlemesi
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsJoin(CUser *pUser, Packet & pkt)
/**
* @brief	Handles joins to knights.
*/
void CKnightsManager::ReqKnightsJoin(CUser *pUser, Packet & pkt)
{
	string strRequesterID, strTarget;
	uint16 sClanID;

	pkt >> sClanID >> strRequesterID;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);

	if (pKnights == nullptr || pUser == nullptr || strRequesterID.empty() || pUser->GetName() != strRequesterID)
		return;

	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_JOIN));
	int8 bResult = int8(g_DBAgent.KnightsMemberJoin(strRequesterID, sClanID));
	if (bResult > 0)
	{
		result << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	std::string noticeText;
	g_pMain->GetServerResource(IDS_KNIGHTS_JOIN, &noticeText, pUser->GetName().c_str());
	if (!noticeText.empty()) pKnights->SendChat("%s", noticeText.c_str());

	pKnights->AddUser(pUser);
	pUser->AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveBecomeKnightMember, 0, nullptr);
	
	result << uint8(1) << pUser->GetSocketID() << pUser->GetClanID() << pUser->GetFame()
		<< pKnights->m_byFlag
		<< pKnights->GetAllianceID();

	if (pUser->isKing()) {
		if (pUser->GetNation() == (uint8)Nation::ELMORAD)
			result << uint16(KingCapeType::KNIGHTS_HUMAN_KING_CAPE) << uint32(0);
		else
			result << uint16(KingCapeType::KNIGHTS_KARUS_KING_CAPE) << uint32(0);
	}
	else {
		if (pKnights->isCastellanCape())
			result << pKnights->m_castCapeID << pKnights->m_bCastCapeR << pKnights->m_bCastCapeG << pKnights->m_bCastCapeB;
		else
			result << pKnights->GetCapeID() << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB;
		result << uint8(0);
	}

	result << int16(pKnights->m_sMarkVersion)
		<< pKnights->m_strName << pKnights->m_byGrade << pKnights->m_byRanking;
	pUser->SendToRegion(&result, nullptr, pUser->GetEventRoom());
	pUser->Send(&result);

	if (pKnights->isInPremium())pUser->SendClanPremium(pKnights, false);

	//xtreme ekleme 05.02.2024
	if (pUser->GetLevel() < 51)
		pUser->CharacterOlduguYerdeYenile();// 17.04.2024 clan paket düzenlemesi

}
#pragma endregion

#pragma region CKnightsManager::ReqKnightsLeave(CUser *pUser, Packet & pkt)
/**
* @brief	Handles leavings from knights.
*/
void CKnightsManager::ReqKnightsLeave(CUser *pUser, Packet & pkt)
{
	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_WITHDRAW));
	if (!pUser)
		return;

	if (!pUser->isInClan() || pUser->m_pKnightsUser == nullptr) {
		result << uint8(2);
		pUser->Send(&result);
		return;
	}

	std::string strRequesterID; uint16 sClanID;
	pkt >> sClanID >> strRequesterID;

	CKnights* pKnights = g_pMain->GetClanPtr(sClanID);
	if (pKnights == nullptr || pUser->GetName() != strRequesterID) {
		result << uint8(2);
		pUser->Send(&result);
		return;
	}

	uint8 bResult = g_DBAgent.KnightsMemberLeave(strRequesterID, sClanID);
	if (bResult != 1) {
		result << bResult;
		pUser->Send(&result);
		return;
	}

	if (!pKnights->RemoveUser(pUser)) {
		result << uint8(2);
		pUser->Send(&result);
		return;
	}

	result << uint8(1) << pUser->GetSocketID() << pUser->GetClanID() << pUser->GetFame();
	pUser->SendToRegion(&result, nullptr, pUser->GetEventRoom());

	std::string clanNotice;
	g_pMain->GetServerResource(IDS_KNIGHTS_WITHDRAW, &clanNotice, pUser->GetName().c_str());
	if (clanNotice.empty()) return;

	Packet result2;
	ChatPacket::Construct(&result2, (uint8)ChatType::KNIGHTS_CHAT, &clanNotice);
	pKnights->Send(&result2);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsDestroy(CUser *pUser, Packet & pkt)
/**
* @brief	Handles destroying knights.
*/
void CKnightsManager::ReqKnightsDestroy(CUser *pUser, Packet & pkt)
{
	if (pUser == nullptr || !pUser->isInClan())
		return;

	std::string strRequesterID; uint16 sClanID;
	pkt >> sClanID >> strRequesterID;

	CKnights* pKnights = g_pMain->GetClanPtr(sClanID);
	if (pKnights == nullptr || pUser->GetName() != strRequesterID)
		return;

	for (int i = 0; i < WAREHOUSE_MAX; i++) {
		auto* pItem = &pKnights->m_sClanWarehouseArray[i];
		if (pItem->nNum != 0)
		{
			g_pMain->SendHelpDescription(pUser, "Please empty the inventory in your clan bank.");
			return;
			break;
		}
	}

	int8 bResult = g_DBAgent.KnightsDestroy(sClanID);
	if (bResult != 0) 
		return;

	if (pKnights->isInAlliance()) {
		auto* pAlliance = g_pMain->GetAlliancePtr(pKnights->GetAllianceID());
		if (pAlliance) {
			Packet result;
			result << pUser->GetName() << pAlliance->sMainAllianceKnights << pKnights->GetID();
			ReqKnightsAllianceRemove(pUser, result);
		}
	}

	pKnights->Disband(pUser);
	//xtreme ekleme 05.02.2024
	if (pUser->GetLevel() < 51)
		pUser->CharacterOlduguYerdeYenile();// 17.04.2024 clan paket düzenlemesi

}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsRemove(CUser *pUser, Packet & pkt)
/**
* @brief	Handles joins to knights.
*/
void CKnightsManager::ReqKnightsRemove(CUser *pUser, Packet & pkt)
{
	if (pUser == nullptr)
		return;

	std::string strRequesterID, strTarget; uint16 sClanID;
	pkt >> sClanID >> strRequesterID >> strTarget;

	CKnights* pKnights = g_pMain->GetClanPtr(sClanID);
	if (pKnights == nullptr || pUser->GetName() != strRequesterID)
		return;

	int8 bResult = int8(g_DBAgent.KnightsMemberRemove(strTarget, sClanID));
	if (bResult > 0) {
		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_REMOVE));
		result << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	std::string clanNotice;


	CUser* pTUser = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
	if (pTUser != nullptr)
		pKnights->RemoveUser(pTUser);
	else
		pKnights->RemoveUser(strTarget);

	g_pMain->GetServerResource(IDS_KNIGHTS_REMOVE, &clanNotice, pTUser == nullptr ? strTarget.c_str() : pTUser->GetName().c_str());

	if (pTUser != nullptr)
		pTUser->SendClanUserStatusUpdate(true);

	if (clanNotice.empty()) return;

	// Construct the clan system chat packet
	Packet result;
	ChatPacket::Construct(&result, (uint8)ChatType::KNIGHTS_CHAT, &clanNotice);

	// If we've been removed from a clan, tell the user as well (since they're no longer in the clan)
	if (pTUser != nullptr) pTUser->Send(&result);

	// Otherwise, since we're actually in the clan, we don't need to be explicitly told what happened.
	pKnights->Send(&result);
}
#pragma endregion

#pragma region CKnightsManager::ReqKnightsAdmit(CUser* pUser, Packet & pkt)
/**
* @brief	Handles joins to knights.
*/
void CKnightsManager::ReqKnightsAdmit(CUser* pUser, Packet & pkt)
{
	string strRequesterID, strTarget;
	uint16 sClanID;

	pkt >> sClanID >> strRequesterID >> strTarget;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);

	if (pKnights == nullptr || pUser == nullptr || pUser->GetName() != strRequesterID)
		return;

	int8 bResult;

	bResult = int8(g_DBAgent.KnightsMemberAdmit(strTarget, sClanID));
	if (bResult > 0)
	{		
		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_ADMIT));
		result << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	CUser *pTUser = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
	if (pTUser != nullptr)
		pTUser->m_bFame = KNIGHT;

	if (pTUser != nullptr)
		pTUser->SendClanUserStatusUpdate(false);
}
#pragma endregion

#pragma region CKnightsManager::ReqKnightsReject(CUser* pUser, Packet & pkt)
/**
* @brief	Handles reject requests of knights.
*/
void CKnightsManager::ReqKnightsReject(CUser* pUser, Packet & pkt)
{
	string strRequesterID, strTarget;
	uint16 sClanID;

	pkt >> sClanID >> strRequesterID >> strTarget;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);

	if (pKnights == nullptr || pUser == nullptr || pUser->GetName() != strRequesterID)
		return;

	int8 bResult;

	bResult = int8(g_DBAgent.KnightsMemberReject(strTarget, sClanID));
	if (bResult > 0)
	{		
		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_REJECT));
		result << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	CUser *pTUser = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
	if (pTUser != nullptr)
	{
		pTUser->SetClanID(NO_CLAN);
		pTUser->m_bFame = 0;
		pKnights->RemoveUser(pTUser);
		pTUser->SendClanUserStatusUpdate(false);
	}
	else
		pKnights->RemoveUser(strTarget);
}
#pragma endregion

#pragma region CKnightsManager::ReqKnightsPunish(CUser* pUser, Packet & pkt)
/**
* @brief	Handles joins to knights.
*/
void CKnightsManager::ReqKnightsPunish(CUser* pUser, Packet & pkt)
{
	if (pUser == nullptr) return;

	std::string strRequesterID, strTarget; uint16 sClanID;
	pkt >> sClanID >> strRequesterID >> strTarget;

	CKnights* pKnights = g_pMain->GetClanPtr(sClanID);
	if (pKnights == nullptr || pUser->GetName() != strRequesterID)
		return;

	int8 bResult = int8(g_DBAgent.KnightsMemberPunish(strTarget, sClanID));
	if (bResult > 0)
	{
		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_PUNISH));
		result << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	//RecvModifyFame(pUser, result, command);

	CUser* pTUser = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
	if (pTUser != nullptr) {
		pTUser->m_bFame = PUNISH;
		pTUser->SendClanUserStatusUpdate(false);
	}
}
#pragma endregion

#pragma region CKnightsManager::ReqKnightsChief(CUser* pUser, Packet & pkt)
/**
* @brief	Handles joins to knights.
*/
void CKnightsManager::ReqKnightsChief(CUser* pUser, Packet & pkt)
{
	string strRequesterID, strTarget;
	uint16 sClanID;

	pkt >> sClanID >> strRequesterID >> strTarget;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);

	if (pKnights == nullptr || pUser == nullptr || pUser->GetName() != strRequesterID)
		return;

	int8 bResult;

	bResult = int8(g_DBAgent.KnightsMemberChief(strTarget, sClanID));
	if (bResult > 0)
	{		
		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_CHIEF));
		result << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	std::string clanNotice;
	CUser *pTUser = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
	if (pTUser != nullptr)
	{
		pTUser->m_bFame = CHIEF;
		pTUser->SendClanUserStatusUpdate(false);
	}

	g_pMain->GetServerResource(IDS_KNIGHTS_CHIEF, &clanNotice, strTarget.c_str());

	if (clanNotice.empty())
		return;

	// Construct the clan system chat packet
	Packet result;
	ChatPacket::Construct(&result, (uint8)ChatType::KNIGHTS_CHAT, &clanNotice);

	// Otherwise, since we're actually in the clan, we don't need to be explicitly told what happened.
	pKnights->Send(&result);
}
#pragma endregion

#pragma region CKnightsManager::ReqKnightsViceChief(CUser* pUser, Packet & pkt)
/**
* @brief	Handles joins to knights.
*/
void CKnightsManager::ReqKnightsViceChief(CUser* pUser, Packet & pkt)
{
	string strRequesterID, strTarget;
	uint16 sClanID;

	pkt >> sClanID >> strRequesterID >> strTarget;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);

	if (pKnights == nullptr || pUser == nullptr || pUser->GetName() != strRequesterID)
		return;

	int8 bResult;

	bResult = int8(g_DBAgent.KnightsMemberViceChief(strTarget, sClanID));
	if (bResult > 0)
	{		
		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_VICECHIEF));
		result << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	CUser *pTUser = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
	if (pTUser != nullptr)
	{
		pTUser->m_bFame = VICECHIEF;
		pTUser->SendClanUserStatusUpdate(false);
	}

	std::string clanNotice;
	g_pMain->GetServerResource(IDS_KNIGHTS_VICECHIEF, &clanNotice, strTarget.c_str());

	if (pKnights->m_strViceChief_1 == "")
		pKnights->m_strViceChief_1 = strTarget;
	else if (pKnights->m_strViceChief_2 == "")
		pKnights->m_strViceChief_2 = strTarget;
	else if (pKnights->m_strViceChief_3 == "")
		pKnights->m_strViceChief_3 = strTarget;

	if (clanNotice.empty())
		return;

	// Construct the clan system chat packet
	Packet result;
	ChatPacket::Construct(&result, (uint8)ChatType::KNIGHTS_CHAT, &clanNotice);
	// Otherwise, since we're actually in the clan, we don't need to be explicitly told what happened.
	pKnights->Send(&result);
}
#pragma endregion

#pragma region CKnightsManager::ReqKnightsOfficer(CUser* pUser, Packet & pkt)
/**
* @brief	Handles joins to knights.
*/
void CKnightsManager::ReqKnightsOfficer(CUser* pUser, Packet & pkt)
{
	string strRequesterID, strTarget;
	uint16 sClanID;
	pkt >> sClanID >> strRequesterID >> strTarget;
	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);

	if (pKnights == nullptr || pUser == nullptr || pUser->GetName() != strRequesterID)
		return;

	int8 bResult = int8(g_DBAgent.KnightsMemberOfficer(strTarget, sClanID));
	if (bResult > 0)
	{		
		Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_OFFICER));
		result << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	CUser *pTUser = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
	if (pTUser != nullptr)
	{
		pTUser->m_bFame = OFFICER;
		pTUser->SendClanUserStatusUpdate(false);
	}
}
#pragma endregion

#pragma region CKnightsManager::ReqKnightsRegisterSymbol(CUser *pUser, Packet & pkt)
/**
* @brief	Handles knights symbol registration on database.
*/
void CKnightsManager::ReqKnightsRegisterSymbol(CUser *pUser, Packet & pkt)
{
	if (pUser == nullptr)
		return;

	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_MARK_REGISTER));
	char clanSymbol[MAX_KNIGHTS_MARK];
	memset(clanSymbol, 0, sizeof(clanSymbol));
	uint16 sClanID, sSymbolSize, sErrorCode = 0, sNewVersion = 0;

	pkt >> sClanID >> sSymbolSize;

	if (!sSymbolSize || sSymbolSize > MAX_KNIGHTS_MARK)
	{
		sErrorCode = 2;
		goto fail_return;
	}


	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);
	if (pKnights == nullptr)
	{
		sErrorCode = 20;
		goto fail_return;
	}

	// Make sure they still have enough coins.
	if (!pUser->GoldLose(CLAN_SYMBOL_COST))
	{
		sErrorCode = 14;
		goto fail_return;
	}

	pkt.read(clanSymbol, sSymbolSize);

	int bResult = g_DBAgent.KnightsSymbolUpdate(sClanID, sSymbolSize, clanSymbol);
	if (bResult > 0)
	{
		sErrorCode = bResult;
		goto fail_return;
	}

	sNewVersion = ++pKnights->m_sMarkVersion;
	pKnights->m_sMarkLen = sSymbolSize;
	memcpy(pKnights->m_Image, clanSymbol, sSymbolSize);
	sErrorCode = 1;
	result << sErrorCode << sNewVersion;
	pKnights->Send(&result);
	return;
fail_return:
	result << sErrorCode << sNewVersion;
	pUser->Send(&result);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsCapeUpdate(Packet & pkt)
/**
* @brief	Handles knights cape update process on database.
*/
void CKnightsManager::ReqKnightsCapeUpdate(Packet & pkt)
{
	uint16 sClanID, sCapeID;
	uint8 r, g, b;
	pkt >> sClanID >> sCapeID >> r >> g >> b;

	g_DBAgent.KnightsCapeUpdate(sClanID, sCapeID, r, g, b);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsHandover(CUser *pUser, Packet & pkt)

void CKnightsManager::ReqKnightsHandover(CUser *pUser, Packet & pkt)
{
	string strRequesterID, strTarget;
	uint16 sClanID;
	pkt >> sClanID >> strRequesterID >> strTarget;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);
	if (pKnights == nullptr || pUser == nullptr || strTarget.empty() || pUser->GetName() != strRequesterID)
		return;

	Packet result(WIZ_KNIGHTS_PROCESS);
	int8 bResult;

	bResult = int8(g_DBAgent.KnightsHandover(strTarget, sClanID));
	if (bResult > 0)
	{
		result << (uint8)KnightsPacket::KNIGHTS_HANDOVER << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	if (STRCASECMP(pKnights->m_strViceChief_1.c_str(), strTarget.c_str()) == 0)
		pKnights->m_strViceChief_1 = "";
	else if (STRCASECMP(pKnights->m_strViceChief_2.c_str(), strTarget.c_str()) == 0)
		pKnights->m_strViceChief_2 = "";
	else if (STRCASECMP(pKnights->m_strViceChief_3.c_str(), strTarget.c_str()) == 0)
		pKnights->m_strViceChief_3 = "";

	pKnights->m_strChief = strTarget;

	result << (uint8)KnightsPacket::KNIGHTS_HANDOVER << pUser->GetName() << strTarget;
	pKnights->Send(&result);
	pUser->ChangeFame(TRAINEE);

	CUser *pTUser = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
	if(pTUser != nullptr)
		pTUser->ChangeFame(CHIEF);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsPointMethodChange(CUser *pUser, Packet & pkt)

void CKnightsManager::ReqKnightsPointMethodChange(CUser *pUser, Packet & pkt)
{
	if (pUser == nullptr)
		return;

	Packet result(WIZ_KNIGHTS_PROCESS);
	string strCharID;
	uint16 sClanID;
	int8 bRemoveFlag, bResult;

	pkt >> strCharID >> sClanID  >> bRemoveFlag;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);
	if(pKnights == nullptr)
		return;

	bResult = int8(g_DBAgent.KnightsPointMethodChange(sClanID, bRemoveFlag));

	if (bResult > 0)
	{
		result << (uint8)(KnightsPacket::KNIGHTS_POINT_METHOD) << uint8(bResult);
		pUser->Send(&result);
		return;
	}

	pKnights->m_sClanPointMethod  = bRemoveFlag;
	result << (uint8)(KnightsPacket::KNIGHTS_POINT_METHOD);
	result << uint8(1) << pKnights->GetClanPointMethod();
	pUser->Send(&result);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsGradeUpdate(Packet & pkt)

/**
* @brief	Request a clan's grade (and cape) be updated
* 			in the database.
*
* @param	pkt	The packet.
*/
void CKnightsManager::ReqKnightsGradeUpdate(Packet & pkt)
{
	uint16 sClanID, sCapeID;
	uint8 byFlag;

	pkt >> sClanID >> byFlag >> sCapeID;
	g_DBAgent.KnightsGradeUpdate(sClanID, byFlag, sCapeID);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsClanNoticeUpdate(Packet & pkt)
/**
* @brief	Requests a clan's notice be updated in the database.
*
* @param	pkt	The packet.
*/
void CKnightsManager::ReqKnightsClanNoticeUpdate(Packet & pkt)
{
	uint16 sClanID;
	string strClanNotice;

	pkt >> sClanID >> strClanNotice;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);
	if (pKnights == nullptr)
		return;

	int8 bResult = int8(g_DBAgent.KnightsClanNoticeUpdate(sClanID, strClanNotice));
	if (bResult > 0)
		return;

	Packet result;

	// Update the stored clan notice
	pKnights->m_strClanNotice = strClanNotice;

	// Construct the update notice packet to inform players the clan notice has changed
	std::string updateNotice = string_format("### %s updated the clan notice. ###", pKnights->m_strChief.c_str()); 
	ChatPacket::Construct(&result, (uint8)ChatType::KNIGHTS_CHAT, &updateNotice);

	if(!updateNotice.empty()) pKnights->Send(&result);

	// Construct the new clan notice packet
	pKnights->ConstructClanNoticePacket(&result);
	pKnights->Send(&result);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsUserMemoUpdate(Packet & pkt)
/**
* @brief	Handles the database request to update the
* 			specified user's memo.
*
* @param	pkt	The packet.
*/
void CKnightsManager::ReqKnightsUserMemoUpdate(Packet & pkt)
{
	uint16 strClanID;
	std::string strUserID, strMemo;
	pkt >> strClanID >> strUserID >> strMemo;
	g_DBAgent.KnightsUserMemoUpdate(strUserID, strMemo);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsAllianceNoticeUpdate(Packet & pkt)
/**
* @brief	Handles the database request to update the
* 			specified Knight's AllianceNotice.
*
* @param	pkt	The packet.
*/
void CKnightsManager::ReqKnightsAllianceNoticeUpdate(Packet & pkt)
{
	uint16 strClanID;
	std::string strUserID, strAllianceNotice;
	pkt >> strClanID >> strUserID >> strAllianceNotice;
	g_DBAgent.KnightsAllianceNoticeUpdate(strClanID, strAllianceNotice);
}

#pragma endregion

#pragma region CKnightsManager::ReqKnightsUpdateNP(Packet & pkt)

/**
* @brief	Handles the database request to update the
* 			specified clan's clan point fund.
*
* @param	pkt	The packet.
*/
void CKnightsManager::ReqKnightsUpdateNP(Packet & pkt)
{
	uint16 sClanID;
	uint32 nClanPointFund;
	pkt >> sClanID >> nClanPointFund;

	CKnights *pKnights = g_pMain->GetClanPtr(sClanID);
	if(pKnights == nullptr)
		return;

	int8 bResult = int8(g_DBAgent.KnightsFundUpdate(sClanID, nClanPointFund));
}

#pragma endregion

/**
* @brief	Handles the clan NP donations database request.
* 			It is essentially the client packet's real handler
* 			as the packet is simply forwarded here.
*
* @param	pUser	The user.
* @param	pkt  	The packet.
*/
void CKnightsManager::ReqKnightsDonateNP(CUser * pUser, Packet & pkt)
{
	if (pUser == nullptr || !pUser->isInClan())
		return;

	uint32 amountNP;
	pkt >> amountNP;

	// Ensure the user has enough NP to donate to the clan.
	if (amountNP > pUser->GetLoyalty()
		// Users must have at least MIN_NP_TO_DONATE to donate.
			|| (pUser->GetLoyalty() - amountNP) < MIN_NP_TO_DONATE)
			return;

	// Ensure the clan exists
	CKnights * pKnights = g_pMain->GetClanPtr(pUser->GetClanID());
	if (pKnights == nullptr)
		return;

	// Take player's donated NP. Don't affect monthly NP. 
	if (g_DBAgent.KnightsUserDonate(pUser, pKnights->m_nClanPointFund + amountNP, amountNP) == 0)
	{
		// Update the user's donated NP
		AddUserDonatedNP(pUser->GetClanID(), pUser->m_strUserID, amountNP, true);

		// Take the NP from the user and update the client.
		pUser->m_iLoyalty -= amountNP;
		pUser->SendLoyaltyChange("Loyalty Donate Clan",0, false, false, false);
		pUser->AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveKnightContribution, 0, nullptr);
	}
}

/**
* @brief	Handles the database request to refund the specified
* 			logged out character's donated NP.
* 			
* 			NOTE: Logged in players don't need to be handled as their NP is
* 			refunded in-game.
*
* @param	pkt	The packet.
*/
void CKnightsManager::ReqKnightsRefundNP(Packet & pkt)
{
	string strUserID;
	uint32 nRefundNP;
	pkt >> strUserID >> nRefundNP;
	g_DBAgent.KnightsRefundNP(strUserID, nRefundNP);
}