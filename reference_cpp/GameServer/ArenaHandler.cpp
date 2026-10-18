#include "stdafx.h"

using std::string;

enum
{
	CHALLENGE_PVP_REQUEST	= 1,
	CHALLENGE_PVP_CANCEL	= 2,
	CHALLENGE_PVP_ACCEPT	= 3,
	CHALLENGE_PVP_REJECT	= 4,
	CHALLENGE_PVP_REQ_SENT	= 5,
	CHALLENGE_CVC_REQUEST	= 6,
	CHALLENGE_CVC_CANCEL	= 7,
	CHALLENGE_CVC_ACCEPT	= 8,
	CHALLENGE_CVC_REJECT	= 9,
	CHALLENGE_CVC_REQ_SENT	= 10,
	CHALLENGE_GENERIC_ERROR	= 11,
	CHALLENGE_ZONE_ERROR	= 12,
	CHALLENGE_CLAN_ERROR	= 13
};

// also known as CUser::BattleFields()
void CUser::HandleChallenge(Packet & pkt)
{
	if (isDead())
		return;

	uint8 opcode = pkt.read<uint8>();
	switch (opcode)
	{
	case CHALLENGE_PVP_REQUEST:
		HandleChallengeRequestPVP(pkt);
		break;

	case CHALLENGE_PVP_ACCEPT:
		HandleChallengeAcceptPVP(pkt);
		break;

	case CHALLENGE_CVC_REQUEST:
		HandleChallengeRequestCVC(pkt);
		break;

	case CHALLENGE_CVC_ACCEPT:
		HandleChallengeAcceptCVC(pkt);
		break;

	case CHALLENGE_PVP_CANCEL:
	case CHALLENGE_CVC_CANCEL:
		HandleChallengeCancelled(opcode);
		break;

	case CHALLENGE_PVP_REJECT:
	case CHALLENGE_CVC_REJECT:
		HandleChallengeRejected(opcode);
		break;
	}
}

// This is sent when the challenger requests a PVP duel with a challengee.
void CUser::HandleChallengeRequestPVP(Packet & pkt)
{
	Packet result(WIZ_CHALLENGE);
	string strUserID;

	if (m_bRequestingChallenge
		|| m_bChallengeRequested
		|| GetZoneID() == ZONE_ARENA
		|| isInParty()
		|| isTrading()
		|| isMerchanting()
		|| isStoreOpen())
	{
		result << uint8(CHALLENGE_GENERIC_ERROR);
		Send(&result);
		return;
	}

	if (GetMap()->isNationPVPZone()
		|| GetZoneID() == ZONE_DELOS
		|| GetMap()->isWarZone())
	{
		result << uint8(CHALLENGE_ZONE_ERROR);
		Send(&result);
		return;
	}

	pkt.SByte();
	result.SByte();
	pkt >> strUserID;

	CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser == nullptr)
	{
		result << uint8(CHALLENGE_GENERIC_ERROR);
		Send(&result);
		return;
	}

	if (!pUser->isInGame()
		|| pUser->isInParty()
		|| pUser->isDead()
		|| pUser->m_bRequestingChallenge
		|| pUser->m_bChallengeRequested
		|| pUser->isMerchanting()
		|| pUser->isTrading()
		|| pUser->isStoreOpen()
		|| pUser->GetZoneID() != GetZoneID())
	{
		result << uint8(CHALLENGE_GENERIC_ERROR);
		Send(&result);
		return;
	}

	m_bRequestingChallenge = (uint8)CHALLENGE_PVP_CANCEL;
	m_sChallengeUser = pUser->GetID();
	pUser->m_bChallengeRequested = (uint8)CHALLENGE_PVP_REJECT;
	pUser->m_sChallengeUser = GetID();

	result << uint8(CHALLENGE_PVP_REQUEST) << GetName();
	pUser->Send(&result);

	result.clear();
	result << uint8(CHALLENGE_PVP_REQ_SENT) << pUser->GetName();
	Send(&result);
}

// This is sent when the challenger requests a clan vs clan battle with a challengee.
void CUser::HandleChallengeRequestCVC(Packet & pkt)
{
	Packet result(WIZ_CHALLENGE);
	string strUserID;

	if (m_bRequestingChallenge
		|| m_bChallengeRequested
		|| GetZoneID() == ZONE_ARENA
		|| isInParty()
		|| isTrading()
		|| isMerchanting()
		|| isStoreOpen())
	{
		result << uint8(CHALLENGE_GENERIC_ERROR);
		Send(&result);
		return;
	}

	// Are we entitled to speak for the clan?
	if (!isClanLeader())
	{
		result << uint8(CHALLENGE_CLAN_ERROR);
		Send(&result);
		return;
	}

	if (GetMap()->isNationPVPZone()
		|| GetZoneID() == ZONE_DELOS
		|| GetMap()->isWarZone())
	{
		result << uint8(CHALLENGE_ZONE_ERROR);
		Send(&result);
		return;
	}

	pkt.SByte();
	result.SByte();
	pkt >> strUserID;

	CUser *pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser == nullptr)
	{
		result << uint8(CHALLENGE_GENERIC_ERROR);
		Send(&result);
		return;
	}

	if (!pUser->isInGame()
		|| pUser->isInParty()
		|| pUser->isDead()
		|| pUser->m_bRequestingChallenge
		|| pUser->m_bChallengeRequested
		|| pUser->isMerchanting()
		|| pUser->isTrading()
		|| pUser->isStoreOpen()
		|| pUser->GetZoneID() != GetZoneID())
	{
		result << uint8(CHALLENGE_GENERIC_ERROR);
		Send(&result);
		return;
	}

	// Is the target entitled to speak for the clan?
	if (!pUser->isClanLeader())
	{
		result << uint8(CHALLENGE_CLAN_ERROR);
		Send(&result);
		return;
	}

	m_bRequestingChallenge = (uint8)CHALLENGE_CVC_CANCEL;
	m_sChallengeUser = pUser->GetID();
	pUser->m_bChallengeRequested = (uint8)CHALLENGE_CVC_REJECT;
	pUser->m_sChallengeUser = GetID();

	result << uint8(CHALLENGE_CVC_REQUEST) << GetName();
	pUser->Send(&result);

	result.clear();
	result << uint8(CHALLENGE_CVC_REQ_SENT) << pUser->GetName();
	Send(&result);
}

// This is sent when the challengee accepts a challenger's PVP request
void CUser::HandleChallengeAcceptPVP(Packet & pkt)
{
	if (!m_bChallengeRequested)
		return;

	CUser *pUser = g_pMain->GetUserPtr(m_sChallengeUser);

	m_sChallengeUser = -1;
	m_bChallengeRequested = 0;

	if (pUser == nullptr)
	{
		Packet result(WIZ_CHALLENGE);
		result << uint8(CHALLENGE_GENERIC_ERROR);
		Send(&result);
		return;
	}

	pUser->m_sChallengeUser = -1;
	pUser->m_bRequestingChallenge = 0;

	// Glorious magic numbers!
	ZoneChange(ZONE_ARENA, 135.0f, 115.0f);
	pUser->ZoneChange(ZONE_ARENA, 120.0f, 115.0f);
}

// This is sent when the challengee accepts a challenger's clan vs clan request
void CUser::HandleChallengeAcceptCVC(Packet & pkt)
{
	if (!m_bChallengeRequested)
		return;

	Packet result(WIZ_CHALLENGE, uint8(CHALLENGE_GENERIC_ERROR));

	if (!isInClan())
		return;

	m_sChallengeUser = -1; m_bChallengeRequested = 0;
	CUser* pUser = g_pMain->GetUserPtr(m_sChallengeUser);
	if (pUser == nullptr) {
		Send(&result);
		return;
	}

	if (!pUser->isInClan())
		return;

	pUser->m_sChallengeUser = -1;
	pUser->m_bRequestingChallenge = 0;

	CKnights* pClan1 = g_pMain->GetClanPtr(GetClanID());
	if (!pClan1) {
		Send(&result);
		pUser->Send(&result);
		return;
	}

	CKnights* pClan2 = g_pMain->GetClanPtr(pUser->GetClanID());
	if (pClan2 == nullptr) {
		Send(&result);
		pUser->Send(&result);
		return;
	}

	std::vector< _KNIGHTS_USER> pKnights1, pKnights2;
	pClan1->m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(i, pClan1->m_arKnightsUser) {
		_KNIGHTS_USER* p = i->second;
		if (p == nullptr)
			continue;

		pKnights1.push_back(*p);
	}
	pClan1->m_arKnightsUser.m_lock.unlock();

	pClan2->m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(i, pClan2->m_arKnightsUser) {
		_KNIGHTS_USER* p = i->second;
		if (p == nullptr)
			continue;

		pKnights2.push_back(*p);
	}
	pClan2->m_arKnightsUser.m_lock.unlock();

	foreach(itr, pKnights1) {
		CUser* pTUser = g_pMain->GetUserPtr(itr->strUserName, NameType::TYPE_CHARACTER);
		if (pTUser == nullptr || !pTUser->isInGame() || pTUser->isInTempleEventZone())
			continue;

		if (pTUser->GetZoneID() == GetZoneID() /*&& !pTUser->isStoreOpen()*/
			&& !pTUser->isMerchanting() && !pTUser->isTrading()
			&& !pTUser->isMining() && !pTUser->isFishing() && !pTUser->isInParty()
			&& !pTUser->m_bRequestingChallenge && pTUser->m_sChallengeUser < 0)

			pTUser->ZoneChange(ZONE_ARENA, 128.0f, 125.0f);
	}

	foreach(itr, pKnights2) {
		CUser* pTUser = g_pMain->GetUserPtr(itr->strUserName, NameType::TYPE_CHARACTER);
		if (pTUser == nullptr || !pTUser->isInGame() || pTUser->isInTempleEventZone())
			continue;

		if (pTUser->GetZoneID() == GetZoneID() /*&& !pTUser->isStoreOpen()*/
			&& !pTUser->isMerchanting() && !pTUser->isTrading()
			&& !pTUser->isMining() && !pTUser->isFishing() && !pTUser->isInParty()
			&& !pTUser->m_bRequestingChallenge && pTUser->m_sChallengeUser < 0)
			pTUser->ZoneChange(ZONE_ARENA, 135.0f, 120.0f);
	}
}

// This is sent when the challenger cancels their request to the challengee.
void CUser::HandleChallengeCancelled(uint8 opcode)
{
	if (!m_bRequestingChallenge)
		return;

	Packet result(WIZ_CHALLENGE);

	CUser *pUser = g_pMain->GetUserPtr(m_sChallengeUser);
	if (pUser == nullptr || pUser->m_sChallengeUser != GetID())
	{
		result << uint8(CHALLENGE_GENERIC_ERROR);
		Send(&result);
	}
	else
	{
		pUser->m_sChallengeUser = -1;
		pUser->m_bChallengeRequested = 0;
		result << opcode;
		pUser->Send(&result);
	}

	m_sChallengeUser = -1;
	m_bRequestingChallenge = 0;
}

// This is sent when the challengee rejects the challenger's request.
void CUser::HandleChallengeRejected(uint8 opcode)
{
	if (!m_bChallengeRequested)
		return;

	Packet result(WIZ_CHALLENGE);

	CUser *pUser = g_pMain->GetUserPtr(m_sChallengeUser);
	if (pUser == nullptr || pUser->m_sChallengeUser != GetID())
	{
		result << uint8(CHALLENGE_GENERIC_ERROR);
	}
	else
	{
		pUser->m_sChallengeUser = -1;
		pUser->m_bRequestingChallenge = 0;
		result << opcode;
		pUser->Send(&result);
	}

	m_sChallengeUser = -1;
	m_bChallengeRequested = 0;
	Send(&result);
}