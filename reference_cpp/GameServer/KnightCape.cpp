#include "stdafx.h"

enum class capetype { normal = 0, ticket = 1 };

void CUser::SendCapeFail(int16 errorcode) {
	Packet result(WIZ_CAPE);
	result << errorcode;
	Send(&result);
}

void CUser::HandleCapeChange(Packet& pkt)
{
	int16 sCapeID; uint8 r, g, b, opcode;
	pkt >> opcode >> sCapeID >> r >> g >> b;
	if (opcode != 1 && opcode != 0)
		return SendCapeFail(-1);

	Packet result(WIZ_CAPE);
	if (!isInGame() || isDead() || !isClanLeader() || isTrading()
		|| isMerchanting() || isMining() || isFishing() || !isInClan())
		return SendCapeFail(-1);

	CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
	if (pKnights == nullptr)
		return SendCapeFail(-2);

	if (!pKnights->isPromoted())
		return SendCapeFail(-1);

	CKnights* pMainKnights = nullptr;
	_KNIGHTS_ALLIANCE* pAlliance = nullptr;
	uint32 nReqClanPoints = 0, nReqCoins = 0;

	bool castnewcape = false;
	if (pKnights->isInAlliance()) {
		pMainKnights = g_pMain->GetClanPtr(pKnights->GetAllianceID());
		pAlliance = g_pMain->GetAlliancePtr(pKnights->GetAllianceID());
		if (pMainKnights == nullptr || pAlliance == nullptr
			|| (pAlliance->sMainAllianceKnights != pKnights->GetID() && pAlliance->sSubAllianceKnights != pKnights->GetID())
			|| (pAlliance->sSubAllianceKnights == pKnights->GetID() && sCapeID >= 0))
			return SendCapeFail(-1);
	}

	if (sCapeID >= 0) {
		auto* pCape = g_pMain->m_KnightsCapeArray.GetData(sCapeID);
		if (pCape == nullptr || sCapeID == 97 || sCapeID == 98 || sCapeID == 99)
			return SendCapeFail(-5);

		if (pCape->Type == 3) castnewcape = true;

		if (opcode == 1 && (pCape->Ticket != opcode || pCape->Type != 3))
			return SendCapeFail(-1);

		if (opcode == 1 && !CheckExistItem(914006000, 1))
			return SendCapeFail(-10);

		if (pKnights->isCastellanCape() && pCape->Type != 3)
			return SendCapeFail(-10);

		if (opcode == 1 && pKnights->m_byGrade > 3)
			return SendCapeFail(-6);

		if (opcode == 0 && ((pCape->byGrade && pKnights->m_byGrade > pCape->byGrade)
			|| pKnights->m_byFlag < pCape->byRanking))
			return SendCapeFail(-6);

		if (opcode == 0 && !hasCoins(pCape->nReqCoins))
			return SendCapeFail(-7);

		nReqCoins = pCape->nReqCoins;
		nReqClanPoints = pCape->nReqClanPoints;
	}

	uint32 nReq1098ClanPrice = 0;
	bool bApplyingPaint = false;
	if (GAME_SOURCE_VERSION == 1098)
	{
		if (r != 0 || g != 0 || b != 0) {

			if (pKnights->m_byGrade > 3)
				return SendCapeFail(-1);

			if (sCapeID == -1)
				nReq1098ClanPrice += 100000000;

			bApplyingPaint = true;
		}
	}
	else
	{
		if (r != 0 || g != 0 || b != 0) {
			if (pKnights->m_byFlag < (uint8)ClanTypeFlag::ClanTypeAccredited5)
				return SendCapeFail(-1);

			bApplyingPaint = true;
			nReqClanPoints += 36000;
		}
	}

	if (opcode == 0 && GetCoins() < nReqCoins)
		return SendCapeFail(-7);

	if (GAME_SOURCE_VERSION == 1098)
	{
		if (opcode == 0 && GetCoins() < nReq1098ClanPrice)
			return SendCapeFail(-9);
	}
	else
	{
		if (opcode == 0 && pKnights->m_nClanPointFund < nReqClanPoints)
			return SendCapeFail(-9);
	}


	if (sCapeID >= 0 && opcode == 1 && !RobItem(914006000, 1))
		return;

	if (opcode == 0 && nReqCoins > 0 && !GoldLose(nReqCoins))
		return;

	if (sCapeID >= 0) {
		if (castnewcape) {
			pKnights->m_castCapeID = sCapeID;
			pKnights->castcape = true;
			uint32 CapeTime = opcode == 1 ? 14 : 15;
			pKnights->m_CastTime = uint32(UNIXTIME + uint32(60 * 60 * 24 * CapeTime));
		}
		else pKnights->m_sCape = sCapeID;
	}

	if (nReqClanPoints && opcode == 0) {
		pKnights->m_nClanPointFund -= nReqClanPoints;
		pKnights->UpdateClanFund();
	}

	if (nReq1098ClanPrice)
		GoldLose(nReq1098ClanPrice);

	if (pKnights->isCastellanCape()) {
		pKnights->m_bCastCapeR = bApplyingPaint ? r : 0;
		pKnights->m_bCastCapeG = bApplyingPaint ? g : 0;
		pKnights->m_bCastCapeB = bApplyingPaint ? b : 0;
	}
	else {
		pKnights->m_bCapeR = bApplyingPaint ? r : 0;
		pKnights->m_bCapeG = bApplyingPaint ? g : 0;
		pKnights->m_bCapeB = bApplyingPaint ? b : 0;
	}

	result << uint16(1) << pKnights->GetAllianceID() << pKnights->GetID();
	if (!pKnights->isInAlliance()) {
		if (isKing()) {
			if (GetNation() == (uint8)Nation::ELMORAD)
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
	}
	else if (pKnights->isInAlliance()) {
		if (isKing()) {
			if (GetNation() == (uint8)Nation::ELMORAD)
				result << uint16(KingCapeType::KNIGHTS_HUMAN_KING_CAPE) << uint32(0);
			else
				result << uint16(KingCapeType::KNIGHTS_KARUS_KING_CAPE) << uint32(0);
		}
		else {
			if (pMainKnights != nullptr && pMainKnights->isCastellanCape())
				result << pMainKnights->m_castCapeID << pKnights->m_bCastCapeR << pKnights->m_bCastCapeG << pKnights->m_bCastCapeB;
			else
				result << pKnights->m_sCape << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB;

			result << uint8(0);
		}
	}
	Send(&result);

	if (!pKnights->isInAlliance() || (pKnights->isInAlliance() && !pKnights->isAllianceLeader()))
		pKnights->SendUpdate();
	else if (pKnights->isInAlliance() && pKnights->isAllianceLeader())
		pKnights->SendAllianceUpdate();

	// Now tell Aujard to save (we don't particularly care whether it was able to do so or not).
	result.clear();
	result.Initialize(WIZ_CAPE);
	if (pKnights->isCastellanCape())
	{
		result << uint8(1) << pKnights->GetID() << pKnights->m_castCapeID << pKnights->m_bCastCapeR << pKnights->m_bCastCapeG << pKnights->m_bCastCapeB << pKnights->m_CastTime;
		pKnights->ReqKnightCapeBonus();
	}
	else
	{
		result << uint8(2) << pKnights->GetID() << pKnights->GetCapeID() << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB << uint32(0);
	}
	g_pMain->AddDatabaseRequest(result, this);
}

void CKnights::ReqKnightCapeBonus()
{
	std::vector<std::string> mlist;

	m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(user, m_arKnightsUser)
	{
		_KNIGHTS_USER *p = user->second;
		if (p == nullptr)
			continue;

		mlist.push_back(p->strUserName);
	}
	m_arKnightsUser.m_lock.unlock();

	foreach(itr, mlist)
	{
		CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (pUser == nullptr
			|| !pUser->isInGame()
			|| !pUser->isInClan())
			continue;

		CKnights* pKnight = g_pMain->GetClanPtr(pUser->GetClanID());
		if (pKnight == nullptr)
			continue;

		if (isCastellanCape()) {
			auto* pCape = g_pMain->m_KnightsCapeArray.GetData(m_castCapeID);
			if (pCape && pCape->BonusType > 0) {
				pUser->SendCapeBonusNotice();
				pUser->SetUserAbility(true);
			}
		}
		else {
			auto* pCape = g_pMain->m_KnightsCapeArray.GetData(pKnight->GetCapeID());
			if (pCape && pCape->BonusType > 0) {
				pUser->SendCapeBonusNotice();
				pUser->SetUserAbility(true);
			}
		}
	}
}

#pragma region CKnights::SendUpdate()
void CKnights::SendUpdate()
{
	Packet result(WIZ_KNIGHTS_PROCESS, uint8(KnightsPacket::KNIGHTS_UPDATE));
	if (isInAlliance()) {
		CKnights* pMainClan = g_pMain->GetClanPtr(GetAllianceID());
		_KNIGHTS_ALLIANCE* pAlliance = g_pMain->GetAlliancePtr(GetAllianceID());
		if (pMainClan == nullptr || pAlliance == nullptr) return;

		if (pMainClan->isCastellanCape()) {
			if (pAlliance->sMainAllianceKnights == GetID())
				result << GetID() << m_byFlag << pMainClan->m_castCapeID << pMainClan->m_bCastCapeR << pMainClan->m_bCastCapeG << pMainClan->m_bCastCapeB << uint8(0);
			else if (pAlliance->sSubAllianceKnights == GetID())
				result << GetID() << m_byFlag << pMainClan->m_castCapeID << m_bCastCapeR << m_bCastCapeG << m_bCastCapeB << uint8(0);
			else if (pAlliance->sMercenaryClan_1 == GetID() || pAlliance->sMercenaryClan_2 == GetID())
				result << GetID() << m_byFlag << pMainClan->m_castCapeID << uint32(0); // only the cape will be present
		}
		else {
			if (pAlliance->sMainAllianceKnights == GetID())
				result << GetID() << m_byFlag << pMainClan->GetCapeID() << pMainClan->m_bCapeR << pMainClan->m_bCapeG << pMainClan->m_bCapeB << uint8(0);
			else if (pAlliance->sSubAllianceKnights == GetID())
				result << GetID() << m_byFlag << pMainClan->GetCapeID() << m_bCapeR << m_bCapeG << m_bCapeB << uint8(0);
			else if (pAlliance->sMercenaryClan_1 == GetID() || pAlliance->sMercenaryClan_2 == GetID())
				result << GetID() << m_byFlag << pMainClan->GetCapeID() << uint32(0); // only the cape will be present
		}
	}
	else {
		if (isCastellanCape())
			result << GetID() << m_byFlag << m_castCapeID << m_bCastCapeR << m_bCastCapeG << m_bCastCapeB << uint8(0);
		else
			result << GetID() << m_byFlag << GetCapeID() << m_bCapeR << m_bCapeG << m_bCapeB << uint8(0);
	}
	Send(&result);
}
#pragma endregion 

#pragma region CKnights::SendAllianceUpdate()
void CKnights::SendAllianceUpdate()
{
	_KNIGHTS_ALLIANCE * pAlliance = g_pMain->GetAlliancePtr(GetAllianceID());

	if (pAlliance == nullptr)
		return;

	CKnights * pKnights1 = g_pMain->GetClanPtr(pAlliance->sMercenaryClan_1),
		*pKnights2 = g_pMain->GetClanPtr(pAlliance->sMercenaryClan_2),
		*pKnights3 = g_pMain->GetClanPtr(pAlliance->sSubAllianceKnights);

	if (pKnights1 != nullptr)
		pKnights1->SendUpdate();

	if (pKnights2 != nullptr)
		pKnights2->SendUpdate();

	if (pKnights3 != nullptr)
		pKnights3->SendUpdate();
}
#pragma endregion 