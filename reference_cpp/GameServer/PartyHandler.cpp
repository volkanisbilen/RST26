#include "stdafx.h"
#include "CBot.h"
#include "../shared/KOSocketMgr.h"
using namespace std;

static uint8 GetBotPartyRank(CBot* pBot)
{
	if (pBot == nullptr)
		return uint8(-1);

	return (pBot->m_bKnightsRank <= pBot->m_bPersonalRank ? pBot->m_bKnightsRank : pBot->m_bPersonalRank <= pBot->m_bKnightsRank ? pBot->m_bPersonalRank : int8(-1));
}

static void BuildPartyMemberInsert(Packet& pkt, CUser* pUser, _PARTY_GROUP* pParty, uint8 byIndex)
{
	pkt.clear();
	pkt << uint8(PARTY_INSERT) << pUser->GetSocketID()
		<< byIndex << pUser->GetName() << pUser->m_MaxHp
		<< pUser->m_sHp << pUser->GetLevel() << pUser->GetClass()
		<< pUser->m_MaxMp << pUser->m_sMp
		<< pUser->GetNation() << uint8(0)
		<< pParty->NumberTargetID << pUser->m_sUserPartyType << pUser->GetLoyaltySymbolRank();
}

static void BuildPartyMemberInsert(Packet& pkt, CBot* pBot, _PARTY_GROUP* pParty, uint8 byIndex)
{
	pkt.clear();
	pkt << uint8(PARTY_INSERT) << pBot->GetID()
		<< byIndex << pBot->GetName() << pBot->m_MaxHp
		<< pBot->m_sHp << pBot->GetLevel() << pBot->GetClass()
		<< pBot->m_MaxMp << pBot->m_sMp
		<< pBot->GetNation() << uint8(0)
		<< pParty->NumberTargetID << uint8(0) << GetBotPartyRank(pBot);
}

static bool IsPartyRestrictedZone(uint8 zoneID)
{
	return zoneID == ZONE_CHAOS_DUNGEON
		|| zoneID == ZONE_PRISON
		|| zoneID == ZONE_DUNGEON_DEFENCE;
}

static bool IsBotPartyLevelValid(CUser* pUser, CBot* pBot)
{
	if (pUser == nullptr || pBot == nullptr)
		return false;

	// Farm botlar normal oyuncunun davetini level farkina takilmadan kabul etsin.
	if (pBot->GetBotState() == BOT_FARMER)
		return true;

	if (pUser->GetZoneID() == ZONE_BORDER_DEFENSE_WAR || pUser->GetZoneID() == ZONE_JURAID_MOUNTAIN)
		return true;

	if (pUser->m_bIsChicken || pBot->m_bIsChicken || (pUser->isInClan() && pUser->GetClanID() == pBot->GetClanID()))
		return true;

	return ((pBot->GetLevel() <= (int)(pUser->GetLevel() * 1.5) && pBot->GetLevel() >= (int)(pUser->GetLevel() * 2 / 3))
		|| (pBot->GetLevel() <= (pUser->GetLevel() + 8) && pBot->GetLevel() >= ((int)(pUser->GetLevel()) - 8)));
}

static bool IsBotPartyLevelValid(CBot* pLeader, CBot* pBot)
{
	if (pLeader == nullptr || pBot == nullptr)
		return false;

	if (pLeader->GetZoneID() == ZONE_BORDER_DEFENSE_WAR || pLeader->GetZoneID() == ZONE_JURAID_MOUNTAIN)
		return true;

	if (pLeader->m_bIsChicken || pBot->m_bIsChicken || (pLeader->isInClan() && pLeader->GetClanID() == pBot->GetClanID()))
		return true;

	return ((pBot->GetLevel() <= (int)(pLeader->GetLevel() * 1.5) && pBot->GetLevel() >= (int)(pLeader->GetLevel() * 2 / 3))
		|| (pBot->GetLevel() <= (pLeader->GetLevel() + 8) && pBot->GetLevel() >= ((int)(pLeader->GetLevel()) - 8)));
}

static bool ValidateBotPartyInvite(CUser* pUser, CBot* pBot, Packet& party)
{
	if (pUser == nullptr || pBot == nullptr || pBot->isInParty())
	{
		party << uint8(PARTY_INSERT) << int16(-7);
		pUser->Send(&party);
		return false;
	}

	if (pUser->GetNation() != pBot->GetNation() && !pUser->isPartnerPartyZone())
	{
		party << uint8(PARTY_INSERT) << int16(-3);
		pUser->Send(&party);
		return false;
	}

	if (pUser->GetZoneID() != pBot->GetZoneID())
	{
		party << uint8(PARTY_INSERT) << int16(-5);
		pUser->Send(&party);
		return false;
	}

	if (IsPartyRestrictedZone(pUser->GetZoneID()))
	{
		party << uint8(PARTY_INSERT) << int16(-9);
		pUser->Send(&party);
		return false;
	}

	if (!pBot->isInGame())
	{
		party << uint8(PARTY_INSERT) << int16(-6);
		pUser->Send(&party);
		return false;
	}

	if (!IsBotPartyLevelValid(pUser, pBot))
	{
		party << uint8(PARTY_INSERT) << int16(-2);
		pUser->Send(&party);
		return false;
	}

	return true;
}

static void AddBotToParty(CUser* pLeader, CBot* pBot, _PARTY_GROUP* pParty)
{
	if (pLeader == nullptr || pBot == nullptr || pParty == nullptr)
		return;

	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] >= 0)
			continue;

		pParty->uid[i] = pBot->GetID();
		pBot->m_sPartyIndex = pParty->wIndex;
		pBot->m_bInParty = true;
		pBot->m_bPartyLeader = false;

		pLeader->m_bInEnterParty = true;
		Packet result(WIZ_PARTY);
		BuildPartyMemberInsert(result, pBot, pParty, uint8(1));
		g_pMain->Send_PartyMember(pParty->wIndex, &result);
		return;
	}
}

static bool AddBotToBotParty(CBot* pLeader, CBot* pBot, _PARTY_GROUP* pParty)
{
	if (pLeader == nullptr || pBot == nullptr || pParty == nullptr)
		return false;

	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] >= 0)
			continue;

		pParty->uid[i] = pBot->GetID();
		pBot->m_sPartyIndex = pParty->wIndex;
		pBot->m_bInParty = true;
		pBot->m_bPartyLeader = false;

		Packet result(WIZ_PARTY);
		BuildPartyMemberInsert(result, pBot, pParty, uint8(1));
		g_pMain->Send_PartyMember(pParty->wIndex, &result);
		return true;
	}

	return false;
}

static int GetPartyMemberCount(_PARTY_GROUP* pParty)
{
	if (pParty == nullptr)
		return 0;

	int count = 0;
	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] >= 0)
			count++;
	}

	return count;
}

enum BotPartyRole : uint8
{
	PARTY_ROLE_UNKNOWN = 0,
	PARTY_ROLE_WARRIOR = 1,
	PARTY_ROLE_ROGUE = 2,
	PARTY_ROLE_MAGE = 3,
	PARTY_ROLE_PRIEST = 4,
	PARTY_ROLE_KURIAN = 5
};

static BotPartyRole GetBotPartyRole(CBot* pBot)
{
	if (pBot == nullptr)
		return PARTY_ROLE_UNKNOWN;
	if (pBot->isWarrior()) return PARTY_ROLE_WARRIOR;
	if (pBot->isRogue()) return PARTY_ROLE_ROGUE;
	if (pBot->isMage()) return PARTY_ROLE_MAGE;
	if (pBot->isPriest()) return PARTY_ROLE_PRIEST;
	if (pBot->isPortuKurian()) return PARTY_ROLE_KURIAN;
	return PARTY_ROLE_UNKNOWN;
}

static BotPartyRole GetUserPartyRole(CUser* pUser)
{
	if (pUser == nullptr)
		return PARTY_ROLE_UNKNOWN;
	if (pUser->isWarrior()) return PARTY_ROLE_WARRIOR;
	if (pUser->isRogue()) return PARTY_ROLE_ROGUE;
	if (pUser->isMage()) return PARTY_ROLE_MAGE;
	if (pUser->isPriest()) return PARTY_ROLE_PRIEST;
	if (pUser->isPortuKurian()) return PARTY_ROLE_KURIAN;
	return PARTY_ROLE_UNKNOWN;
}

static bool IsRoleAllowedInParty(_PARTY_GROUP* pParty, CBot* pCandidate)
{
	if (pParty == nullptr || pCandidate == nullptr)
		return true;

	BotPartyRole candidateRole = GetBotPartyRole(pCandidate);
	if (candidateRole == PARTY_ROLE_UNKNOWN)
		return true;

	uint8 sameRoleCount = 0;
	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] < 0)
			continue;

		if (CBot* pBotMember = g_pMain->GetBotPtr(pParty->uid[i]))
		{
			if (GetBotPartyRole(pBotMember) == candidateRole)
				sameRoleCount++;
		}
		else if (CUser* pUserMember = g_pMain->GetUserPtr(pParty->uid[i]))
		{
			if (GetUserPartyRole(pUserMember) == candidateRole)
				sameRoleCount++;
		}
	}

	// Priest should be unique in a party; other roles can be at most 2.
	if (candidateRole == PARTY_ROLE_PRIEST)
		return sameRoleCount < 1;

	return sameRoleCount < 2;
}

static bool IsBotValidForBotParty(CBot* pLeader, CBot* pBot, _PARTY_GROUP* pParty)
{
	return pLeader != nullptr
		&& pBot != nullptr
		&& pBot != pLeader
		&& pBot->isInGame()
		&& !pBot->isDead()
		&& !pBot->isInParty()
		&& pBot->m_BotState == pLeader->m_BotState
		&& pBot->GetNation() == pLeader->GetNation()
		&& pBot->GetZoneID() == pLeader->GetZoneID()
		&& pLeader->isInRangeSlow(pBot, RANGE_100M)
		&& IsRoleAllowedInParty(pParty, pBot)
		&& IsBotPartyLevelValid(pLeader, pBot);
}

static _PARTY_GROUP* FillBotPartyWithBots(CBot* pLeader, _PARTY_GROUP* pParty)
{
	if (pLeader == nullptr)
		return pParty;

	if (pLeader->isInParty())
	{
		pParty = g_pMain->GetPartyPtr(pLeader->GetPartyID());
		if (pParty == nullptr || pParty->uid[0] != pLeader->GetID())
			return pParty;
	}

	if (pParty != nullptr && GetPartyMemberCount(pParty) >= MAX_PARTY_USERS)
		return pParty;

	foreach_stlmap_nolock(itr, g_pMain->m_MapBotList)
	{
		CBot* pBot = itr->second;
		if (!IsBotValidForBotParty(pLeader, pBot, pParty))
			continue;

		if (pParty == nullptr)
			pParty = g_pMain->CreateParty(pLeader);

		if (pParty == nullptr)
			return nullptr;

		AddBotToBotParty(pLeader, pBot, pParty);

		if (GetPartyMemberCount(pParty) >= MAX_PARTY_USERS)
			break;
	}

	return pParty;
}

static bool IsUserSeekingParty(CUser* pUser)
{
	if (pUser == nullptr || pUser->m_bNeedParty != 2)
		return false;

	bool isSeeking = false;
	g_pMain->m_SeekingPartyArrayLock.lock();
	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		if ((*itr)->m_sSid == pUser->GetID()
			&& (*itr)->m_bZone == pUser->GetZoneID()
			&& (*itr)->m_bNation == pUser->GetNation())
		{
			isSeeking = true;
			break;
		}
	}
	g_pMain->m_SeekingPartyArrayLock.unlock();

	return isSeeking;
}

/*
1. The invitation to the party has been declined.
2. You cannot form a party because of the Level difference.
3. You cannot form a party with a user from the other nation.
4. A clan party cannot invite users from a different clan.
5. You cannot form a party with a user in a different zone.
6. You cannot form a party with a user that's not online.
7. You cannot form a party with this user.
8. Not a same clan
9. You cannot form a party in this region.
10. You cannot invite this player to a party. Invitee does not have required item
*/

#pragma region void CUser::PartySystemProcess(Packet & pkt)
void CUser::PartySystemProcess(Packet& pkt)
{
	uint8 opcode;
	pkt >> opcode;
	switch (opcode)
	{
	case PARTY_CREATE:
		PartyCreateRequest(pkt);
		break;
	case PARTY_INSERT:
		PartyInvitationRequest(pkt);
		break;
	case PARTY_PERMIT:
		PartyInsertOrCancel(pkt);
		break;
	case PARTY_PROMOTE:
		PartyLeaderPromote(pkt.read<uint16>());
		break;
	case PARTY_REMOVE:
		PartyNemberRemove(pkt.read<uint16>());
		break;
	case PARTY_DELETE:
		PartyisDelete();
		break;
	case PARTY_TARGET_NUMBER:
		PartyTargetNumber(pkt);
		break;
	case PARTY_ALERT:
		PartyAlert(pkt);
		break;
	case PARTY_COMMAND_PROMATE:
		PartyCommand(pkt);
		break;
	default:
		printf("Party unhandled packets %x \n", opcode);
		TRACE("Party unhandled packets %x \n", opcode);
		break;
	}
}
#pragma endregion

#pragma region void CUser::PartyCreateRequest(Packet & pkt)
void CUser::PartyCreateRequest(Packet& pkt)
{
	std::string strUserID = ""; int8 PartyType;
	pkt >> strUserID >> PartyType;

	/*01 0D00 48756D616E4D6167656369616E FF*/
	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE)
		return;

	PartyCreateCheck(strUserID, PartyType);
}
#pragma endregion

#pragma region void CUser::PartyCreateCheck(uint16 UserGetSocketID, uint8 PartyType
void CUser::PartyCreateCheck(std::string strUserID, int8 PartyType)
{
	Packet party(WIZ_PARTY);
	CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	CBot* pBot = nullptr;

	if (pUser == nullptr)
	{
		pBot = g_pMain->GetBotPtr(strUserID, NameType::TYPE_CHARACTER);
		if (!ValidateBotPartyInvite(this, pBot, party))
			return;

		if (isInParty() || isInApprovedParty()) {
			party << uint8(PARTY_INSERT) << int16(-7);
			Send(&party);
			return;
		}

		auto* pParty = g_pMain->CreateParty(this);
		if (pParty == nullptr) {
			party << uint8(PARTY_INSERT) << int16(-7);
			Send(&party);
			return;
		}

		m_sUserPartyType = PartyType;
		m_bPartyLeader = true;
		m_bPartyCommandLeader = true;
		m_bInEnterParty = true;
		StateChangeServerDirect(6, 1);
		AddBotToParty(this, pBot, pParty);
		return;
	}

	if (pUser == nullptr || pUser == this || pUser->isInParty()
		|| pUser->isInApprovedParty()) {
		party << uint8(PARTY_INSERT) << int16(-7);
		Send(&party);
		return;
	}

	if (GetNation() != pUser->GetNation() && !isPartnerPartyZone()) {
		party << uint8(PARTY_INSERT) << int16(-3);
		Send(&party);
		return;
	}

	if (GetZoneID() != pUser->GetZoneID()) {
		party << uint8(PARTY_INSERT) << int16(-5);
		Send(&party);
		return;
	}

	if (GetZoneID() == ZONE_CHAOS_DUNGEON
		|| GetZoneID() == ZONE_PRISON
		|| GetZoneID() == ZONE_DUNGEON_DEFENCE) {
		party << uint8(PARTY_INSERT) << int16(-9);
		Send(&party);
		return;
	}

	if (!pUser->isInGame()) {
		party << uint8(PARTY_INSERT) << int16(-6);
		Send(&party);
		return;
	}

	if (PartyType == 2) {
		if (!pUser->CheckExistItem(914057000, 1)) {
			party << uint8(PARTY_INSERT) << int16(-10);
			Send(&party);
			return;
		}

		bool seekchecking = false;
		if (g_pMain->m_SeekingPartyArray.size() != 0) {
			g_pMain->m_SeekingPartyArrayLock.lock();
			foreach(itr, g_pMain->m_SeekingPartyArray) {
				if ((*itr) == nullptr || (*itr)->m_sSid != GetID())
					continue;

				if ((*itr)->m_bloginType != 2) {
					party << uint8(PARTY_INSERT) << int16(-7);
					Send(&party);
					g_pMain->m_SeekingPartyArrayLock.unlock();
					return;
				}
				seekchecking = true;
			}
			g_pMain->m_SeekingPartyArrayLock.unlock();
		}
		if (!seekchecking) {
			party << uint8(PARTY_INSERT) << int16(-7);
			Send(&party);
			return;
		}
	}

	if (g_pMain->m_byBattleSiegeWarOpen && GetZoneID() == ZONE_DELOS)
	{
		CKnights * pKnights = g_pMain->GetClanPtr(pUser->GetClanID());
		if (pKnights != nullptr)
		{ 
			if (!pKnights->isInAlliance() && GetClanID() != pUser->GetClanID())
			{
				party << uint8(PARTY_INSERT) << int16(-3);
				Send(&party);
				return;
			}
			
			_KNIGHTS_ALLIANCE * pAlliance = g_pMain->GetAlliancePtr(pKnights->GetAllianceID());
			if (pAlliance != nullptr)
			{
				if (pKnights->isInAlliance() && pKnights->GetAllianceID() != pAlliance->sMainAllianceKnights)
				{
					party << uint8(PARTY_INSERT) << int16(-3);
					Send(&party);
					return;
				}
			}
		}
	}

	if (GetZoneID() != ZONE_BORDER_DEFENSE_WAR && GetZoneID() != ZONE_JURAID_MOUNTAIN) {
		if (!m_bIsChicken && !pUser->m_bIsChicken && (!isInClan() || GetClanID() != pUser->GetClanID())) {
			if (!((pUser->GetLevel() <= (int)(GetLevel() * 1.5) && pUser->GetLevel() >= (int)(GetLevel() * 2 / 3))
				|| (pUser->GetLevel() <= (GetLevel() + 8) && pUser->GetLevel() >= ((int)(GetLevel()) - 8)))) {
				party << uint8(PARTY_INSERT) << int16(-2);
				Send(&party);
				return;
			}
		}
	}

	if (isInParty() || isInApprovedParty()) {
		party << uint8(PARTY_INSERT) << int16(-7);
		Send(&party);
		return;
	}

	auto* pParty = g_pMain->CreateParty(this);
	if (pParty == nullptr) {
		party << uint8(PARTY_INSERT) << int16(-7);
		Send(&party);
		return;
	}

	m_sUserPartyType = PartyType;
	pUser->m_sUserPartyType = PartyType;
	m_bPartyLeader = true;
	m_bPartyCommandLeader = true;
	StateChangeServerDirect(6, 1); // give party leader the 'P' symbol
	pUser->m_sPartyIndex = m_sPartyIndex;
	pUser->m_bInParty = true;
	party << uint8(PARTY_PERMIT) << GetSocketID() << GetName();
	pUser->Send(&party);
}
#pragma endregion

#pragma region void CUser::PartyInvitationRequest(Packet & pkt)
void CUser::PartyInvitationRequest(Packet& pkt)
{
	std::string strUserID = "";
	int8 PartyType = 0;
	pkt >> strUserID >> PartyType;

	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE) 
		return;

	auto* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser == nullptr)
	{
		CBot* pBot = g_pMain->GetBotPtr(strUserID, NameType::TYPE_CHARACTER);
		Packet party(WIZ_PARTY);
		if (!ValidateBotPartyInvite(this, pBot, party))
			return;

		_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
		if (pParty == nullptr) {
			party << uint8(PARTY_INSERT) << int16(-1);
			Send(&party);
			return;
		}

		if (GetPartyMemberAmount(pParty) == MAX_PARTY_USERS) {
			party << uint8(PARTY_INSERT) << int16(-1);
			Send(&party);
			return;
		}

		AddBotToParty(this, pBot, pParty);
		return;
	}

	if (!pUser->isInGame())
		return;
	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());

	if (pUser->m_bUserPriestBotID > 0)
	{
		for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
		{
			auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
			if (PartyUser != nullptr && PartyUser->m_bUserPriestBotID > 0)
			{
				Packet result(PL_MESSAGE);
				result << uint8(0) << "Error" << "This character has priest and your party has a priest!";
				pUser->Send(&result);

				Packet result2(PL_MESSAGE);
				result2 << uint8(0) << "Error" << "You can not join party because the party has a priest.";
				pUser->Send(&result2);
				return;
			}

		}
	}
	
	PartyInvitationCheck(pUser->GetSocketID(), PartyType);
}
#pragma endregion

#pragma region void CUser::PartyInvitationCheck(uint16 UserGetSocketID, uint8 PartyType)
void CUser::PartyInvitationCheck(uint16 UserGetSocketID, uint8 PartyType)
{
	Packet party(WIZ_PARTY);
	CUser* pUser = g_pMain->GetUserPtr(UserGetSocketID);
	_PARTY_GROUP* pParty = nullptr;
	int16 PartyMemberCount = 0;

	if (pUser == nullptr || pUser == this || pUser->isInParty() || pUser->isInApprovedParty()) {
		party << uint8(PARTY_INSERT) << int16(-7);
		Send(&party);
		return;
	}

	if (GetNation() != pUser->GetNation()
		&& !isPartnerPartyZone())
	{
		party << uint8(PARTY_INSERT) << int16(-3);
		Send(&party);
		return;
	}

	if (GetZoneID() != pUser->GetZoneID())
	{
		party << uint8(PARTY_INSERT) << int16(-5);
		Send(&party);
		return;
	}

	if (GetZoneID() == ZONE_CHAOS_DUNGEON
		|| GetZoneID() == ZONE_PRISON
		|| GetZoneID() == ZONE_DUNGEON_DEFENCE)
	{
		party << uint8(PARTY_INSERT) << int16(-9);
		Send(&party);
		return;
	}

	if (!pUser->isInGame()) {
		party << uint8(PARTY_INSERT) << int16(-6);
		Send(&party);
		return;
	}

	if (PartyType == 2 || m_sUserPartyType == 2) {
		if (!pUser->CheckExistItem(914057000, 1)) {
			party << uint8(PARTY_INSERT) << int16(-10);
			Send(&party);
			return;
		}

		g_pMain->m_SeekingPartyArrayLock.lock();
		foreach(itr, g_pMain->m_SeekingPartyArray) {
			if ((*itr) == nullptr) continue;
			if ((*itr)->m_sSid == GetID()
				&& (*itr)->m_bloginType != 2) {
				party << uint8(PARTY_INSERT) << int16(-7);
				Send(&party);
				g_pMain->m_SeekingPartyArrayLock.unlock();
				return;
			}
		}
		g_pMain->m_SeekingPartyArrayLock.unlock();
	}

	if (GetZoneID() != ZONE_BORDER_DEFENSE_WAR && GetZoneID() != ZONE_JURAID_MOUNTAIN) {
		if (!m_bIsChicken && !pUser->m_bIsChicken && (!isInClan() || GetClanID() != pUser->GetClanID())) {
			if (!((pUser->GetLevel() <= (int)(GetLevel() * 1.5) && pUser->GetLevel() >= (int)(GetLevel() * 2 / 3))
				|| (pUser->GetLevel() <= (GetLevel() + 8) && pUser->GetLevel() >= ((int)(GetLevel()) - 8)))) {
				party << uint8(PARTY_INSERT) << int16(-2);
				Send(&party);
				return;
			}
		}
	}

	pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr) {
		party << uint8(PARTY_INSERT) << int16(-1);
		Send(&party);
		return;
	}

	for (PartyMemberCount = 0; PartyMemberCount < MAX_PARTY_USERS; PartyMemberCount++) {
		if (pParty->uid[PartyMemberCount] < 0)
			break;
	}

	if (PartyMemberCount == MAX_PARTY_USERS)
	{
		party << uint8(PARTY_INSERT) << int16(-1);
		Send(&party);
		return;
	}

	pUser->m_sPartyIndex = m_sPartyIndex;
	pUser->m_bInParty = true;

	if (m_sUserPartyType == 2)
		pUser->m_sUserPartyType = m_sUserPartyType;
	else
		pUser->m_sUserPartyType = PartyType;

	party << uint8(PARTY_PERMIT) << GetSocketID() << GetName();
	pUser->Send(&party);
}
#pragma endregion

#pragma region void CUser::PartyInsertOrCancel(Packet & pkt)
void CUser::PartyInsertOrCancel(Packet& pkt)
{
	uint8 OkorCancel;
	pkt >> OkorCancel;
	switch (OkorCancel)
	{
	case 0:
		DoNotAcceptJoiningTheParty();
		break;
	case 1:
		AgreeToJoinTheParty();
		break;
	default:
		printf("Party System Ok or Cansel unhandled opcode %d \n", OkorCancel);
		TRACE("Party System Ok or Cansel unhandled opcode %d \n", OkorCancel);
		break;
	}
}
#pragma endregion

#pragma region void CUser::AgreeToJoinTheParty()
void CUser::AgreeToJoinTheParty()
{
	Packet result(WIZ_PARTY);
	_PARTY_GROUP* pParty = nullptr;
	uint8 byIndex = 0xFF;
	int leader_id = -1;

	if (!isInApprovedParty())
		return;

	pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
	{
		m_bInParty = false;
		m_bInEnterParty = false;
		m_sUserPartyType = 0;
		m_sPartyIndex = -1;
		return;
	}

	CUser* pLeader = g_pMain->GetUserPtr(pParty->uid[0]);
	CBot* pLeaderBot = pLeader == nullptr ? g_pMain->GetBotPtr(pParty->uid[0]) : nullptr;
	if ((pLeader == nullptr || !pLeader->isInGame()) && (pLeaderBot == nullptr || !pLeaderBot->isInGame())) {
		m_bInParty = false;
		m_bInEnterParty = false;
		m_sUserPartyType = 0;
		m_sPartyIndex = -1;
		return;
	}

	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		CUser* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (PartyUser != nullptr)
		{
			if (PartyUser->m_bUserPriestBotID != -1)
			{

				if (m_bUserPriestBotID != -1)
				{
					Packet result(PL_MESSAGE);
					result << uint8(0) << "Error" << "This character has priest and your party has a priest!";
					if (pLeader != nullptr)
						pLeader->Send(&result);

					Packet result2(PL_MESSAGE);
					result2 << uint8(0) << "Error" << "You can not join party because the party has a priest.";
					Send(&result2);

					DoNotAcceptJoiningTheParty();

					return;
				}
			}
		}
	}

	uint8 leaderZone = pLeader != nullptr ? pLeader->GetZoneID() : pLeaderBot->GetZoneID();
	uint16 leaderPartyID = pLeader != nullptr ? pLeader->m_sPartyIndex : pLeaderBot->m_sPartyIndex;
	if (leaderZone != GetZoneID()
		|| GetPartyMemberAmount(pParty) == MAX_PARTY_USERS) {
		DoNotAcceptJoiningTheParty();
		return;
	}

	// make sure user isn't already in the array...
	// kind of slow, but it works for the moment
	foreach_array(i, pParty->uid)
	{
		if (pParty->uid[i] == GetSocketID())
		{
			m_bInParty = false;
			m_bInEnterParty = false;
			m_sUserPartyType = 0;
			m_sPartyIndex = -1;
			pParty->uid[i] = -1;
			return;
		}
	}

	int PartyNemberCount = 0;
	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] >= 0)
			PartyNemberCount++;
	}

	if (PartyNemberCount == 1 && pLeader != nullptr)
		pLeader->m_bInEnterParty = true;

	// Send the player who just joined the existing party list
	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		// No player set?
		if (pParty->uid[i] < 0)
		{
			// If we're not in the list yet, add us.
			if (byIndex == 0xFF) {
				pParty->uid[i] = GetSocketID();
				byIndex = i;
			}
			continue;
		}

		// For everyone else,
		CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUser != nullptr && pUser->isInGame())
		{
			BuildPartyMemberInsert(result, pUser, pParty, uint8(1));
			Send(&result);
			continue;
		}

		CBot* pBot = g_pMain->GetBotPtr(pParty->uid[i]);
		if (pBot == nullptr || !pBot->isInGame()) continue;

		BuildPartyMemberInsert(result, pBot, pParty, uint8(1));
		Send(&result);
	}

	if (pLeader != nullptr && (pLeader->m_bNeedParty == 2 || m_bNeedParty == 2)) {
		if (pLeader->m_bNeedParty == 2) {
			g_pMain->m_SeekingPartyArrayLock.lock();
			// You don't need anymore seek
			foreach(itr, g_pMain->m_SeekingPartyArray) {
				if ((*itr) == nullptr) continue;
				if ((*itr)->m_sSid == pLeader->GetID()) {
					g_pMain->m_SeekingPartyArray.erase(itr);
					break;
				}
			}
			g_pMain->m_SeekingPartyArrayLock.unlock();
		}
		if (m_bNeedParty == 2) {
			g_pMain->m_SeekingPartyArrayLock.lock();
			// You don't need anymore seek
			foreach(itr, g_pMain->m_SeekingPartyArray) {
				if ((*itr) == nullptr) continue;
				if ((*itr)->m_sSid == GetID()) {
					g_pMain->m_SeekingPartyArray.erase(itr);
					break;
				}
			}
			g_pMain->m_SeekingPartyArrayLock.unlock();
		}
	}

	m_bInEnterParty = true;
	if (pLeader != nullptr)
		pLeader->m_bInEnterParty = true;
	m_sPartyIndex = leaderPartyID;

	if (pLeader != nullptr && pLeader->m_bNeedParty == 2 && pLeader->isInParty())
		pLeader->StateChangeServerDirect(2, 1);

	if (m_bNeedParty == 2 && isInParty())
		StateChangeServerDirect(2, 1);

	BuildPartyMemberInsert(result, this, pParty, uint8(1));
	g_pMain->Send_PartyMember(GetPartyID(), &result);

	CUser* pUserr = nullptr;  //17.12.2020 Partyde HP Degeri Yazi Olarak Gosterme
	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		pUserr = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUserr == nullptr)
			continue;

		pUserr->SendPartyHpManager(PartyType::Send_All); 
	}
}
#pragma endregion

#pragma region void CUser::DoNotAcceptJoiningTheParty()
void CUser::DoNotAcceptJoiningTheParty()
{
	int LeaderGetID = -1, PartyNemberCount = 0;
	if (!isInApprovedParty())
		return;

	m_sUserPartyType = 0;
	m_bInParty = false;
	m_bInEnterParty = false;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr) {
		m_sPartyIndex = -1;
		return;
	}

	LeaderGetID = pParty->uid[0];
	CUser* pLeader = g_pMain->GetUserPtr(pParty->uid[0]);
	CBot* pLeaderBot = pLeader == nullptr ? g_pMain->GetBotPtr(pParty->uid[0]) : nullptr;
	if (pLeader == nullptr && pLeaderBot == nullptr) return;

	for (int i = 0; i < MAX_PARTY_USERS; i++) {
		if (pParty->uid[i] >= 0)
			PartyNemberCount++;
	}

	if (PartyNemberCount == 1 && pLeader != nullptr)
		pLeader->PartyisDelete();
	else if (PartyNemberCount == 1 && pLeaderBot != nullptr)
	{
		pLeaderBot->m_bInParty = false;
		pLeaderBot->m_bPartyLeader = false;
		pLeaderBot->m_sPartyIndex = 0;
		g_pMain->DeleteParty(pParty->wIndex);
	}

	Packet result(WIZ_PARTY, uint8(PARTY_INSERT));
	result << int16(-1);
	if (pLeader != nullptr)
		pLeader->Send(&result);
}
#pragma endregion

#pragma region void CUser::PartyisDelete()
void CUser::PartyisDelete()
{
	if (!isInApprovedParty())
		return;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr) {
		m_sUserPartyType = 0;
		m_bInEnterParty = false;
		m_bInParty = false;
		m_sPartyIndex = -1;
		m_bPartyCommandLeader = false;
		return;
	}

	for (int i = 0; i < MAX_PARTY_USERS; i++) {
		CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUser != nullptr) {
			pUser->m_sUserPartyType = 0;
			pUser->m_bInEnterParty = false;
			pUser->m_bInParty = false;
			pUser->m_sPartyIndex = -1;
			pUser->m_bPartyCommandLeader = false;
			if (pUser->GetZoneID() == ZONE_STONE3)
				pUser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f, 0);
		}

		CBot* pBot = g_pMain->GetBotPtr(pParty->uid[i]);
		if (pBot != nullptr) {
			pBot->m_bInParty = false;
			pBot->m_bPartyLeader = false;
			pBot->m_sPartyIndex = 0;
		}
	}

	if (m_bNeedParty == 3) {
		// You don't need anymore seek
		g_pMain->m_SeekingPartyArrayLock.lock();
		foreach(itr, g_pMain->m_SeekingPartyArray) {
			if ((*itr) == nullptr) continue;
			if ((*itr)->m_sSid == GetID()) {
				g_pMain->m_SeekingPartyArray.erase(itr);
				break;
			}
		}
		g_pMain->m_SeekingPartyArrayLock.unlock();
		StateChangeServerDirect(2, 1);
	}

	Packet result(WIZ_PARTY, uint8(PARTY_DELETE));
	g_pMain->Send_PartyMember(pParty->wIndex, &result);

	m_bPartyLeader = false;
	StateChangeServerDirect(6, 0); // remove 'P' symbol from party leader
	g_pMain->DeleteParty(pParty->wIndex);
}
#pragma endregion

#pragma region void CUser::PartyLeaderPromote(uint16 GetLeaderID)
void CUser::PartyLeaderPromote(uint16 GetLeaderID, _PARTY_GROUP* pmyparty)
{
	if (!isPartyLeader())
		return;

	_PARTY_GROUP* pParty = nullptr;
	if (!pmyparty) 
		pParty = g_pMain->GetPartyPtr(GetPartyID());
	else 
		pParty = pmyparty;

	if (pParty == nullptr) 
		return;

	uint8 pos = 0;
	for (uint8 i = 1; i < MAX_PARTY_USERS; i++) {
		if (pParty->uid[i] != GetLeaderID)
			continue;
		pos = i;
		break;
	}

	// Didn't find it? (leader's always position 0, no need to check there)
	if (pos == 0 || pos >= MAX_PARTY_USERS)
		return;

	// Ensure this user exists and that they're in our party already.
	CUser* pUser = g_pMain->GetUserPtr(pParty->uid[pos]);
	if (pUser == nullptr
		|| pUser->GetPartyID() != GetPartyID())
		return;

	// Swap the IDs around, so they have the leadership position.
	std::swap(pParty->uid[0], pParty->uid[pos]);

	// Swap the seeking party & leader flags
	std::swap(m_bNeedParty, pUser->m_bNeedParty);
	std::swap(m_bPartyLeader, pUser->m_bPartyLeader);

	// Remove our leadership state from the client
	StateChangeServerDirect(6, 0); // remove 'P' symbol from old party leader	
	StateChangeServerDirect(2, m_bNeedParty); // seeking a party

	// Make them leader.
	pUser->StateChangeServerDirect(6, 1); // assign 'P' symbol to new party leader
	pUser->StateChangeServerDirect(2, pUser->m_bNeedParty); // seeking a party

	Packet result(WIZ_PARTY, uint8(PARTY_INSERT));
	result << pUser->GetSocketID()
		<< uint8(100) // reset position to leader
		<< pUser->GetName() << pUser->m_MaxHp << pUser->m_sHp
		<< pUser->GetLevel() << pUser->GetClass()
		<< pUser->m_MaxMp << pUser->m_sMp
		<< pUser->GetNation() << uint8(0)
		<< pParty->NumberTargetID
		<< pUser->m_sUserPartyType << pUser->m_bPersonalRank;
	g_pMain->Send_PartyMember(GetPartyID(), &result);

	for (int i = 0; i < MAX_PARTY_USERS; i++) //17.12.2020 Partyde HP Degeri Yazi Olarak Gosterme
	{
		pUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUser == nullptr)
			continue;

		pUser->SendPartyHpManager(PartyType::Send_All);
	}
}
#pragma endregion

#pragma region void CUser::PartyNemberRemove(uint16 UserID)
void CUser::PartyNemberRemove(uint16 UserID, _PARTY_GROUP* pmyparty)
{
	if (!isInApprovedParty())
		return;

	CUser* pUser = g_pMain->GetUserPtr(UserID);
	CBot* pBot = pUser == nullptr ? g_pMain->GetBotPtr(UserID) : nullptr;
	if (pUser == nullptr && pBot == nullptr)
		return;

	_PARTY_GROUP* pParty = nullptr;
	if (!pmyparty) pParty = g_pMain->GetPartyPtr(GetPartyID());
	else pParty = pmyparty;

	if (pParty == nullptr) {
		m_bInParty = false;
		m_bInEnterParty = false;
		m_sPartyIndex = -1;
		m_sUserPartyType = 0;
		if (pUser != nullptr) {
			pUser->m_bInParty = false;
			pUser->m_bInEnterParty = false;
			pUser->m_sPartyIndex = -1;
			pUser->m_sUserPartyType = 0;
		}
		if (pBot != nullptr) {
			pBot->m_bInParty = false;
			pBot->m_bPartyLeader = false;
			pBot->m_sPartyIndex = 0;
		}
		return;
	}

	uint16 targetID = pUser != nullptr ? pUser->GetSocketID() : pBot->GetID();
	if (targetID != GetSocketID()) {
		if (pParty->uid[0] != GetSocketID())
			return;
	}
	else {
		if (pParty->uid[0] == targetID) {
			PartyisDelete();
			return;
		}
	}

	int count = 0, memberPos = -1;
	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] < 0)
			continue;

		else if (pParty->uid[i] == targetID)
		{
			memberPos = i;
			continue;
		}
		count++;
	}

	if (count == 1)
	{
		CUser* pPartyLeader = g_pMain->GetUserPtr(pParty->uid[0]);
		if (pPartyLeader == nullptr)
			return;
		else
			pPartyLeader->PartyisDelete();
		return;
	}

	Packet result(WIZ_PARTY, uint8(PARTY_REMOVE));
	result << targetID;
	g_pMain->Send_PartyMember(m_sPartyIndex, &result);

	if (memberPos >= 0) {
		if (pUser != nullptr) {
			pUser->m_sUserPartyType = 0;
			pUser->m_bInParty = false;
			pUser->m_bInEnterParty = false;
			pUser->m_sPartyIndex = -1;
		}
		if (pBot != nullptr) {
			pBot->m_bInParty = false;
			pBot->m_bPartyLeader = false;
			pBot->m_sPartyIndex = 0;
		}
		pParty->uid[memberPos] = -1;
	}

	for (int i = 0; i < MAX_PARTY_USERS; i++) //17.12.2020 Partyde HP Degeri Yazi Olarak Gosterme
	{
		pUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUser == nullptr)
			continue;

		pUser->SendPartyHpManager(PartyType::Send_All);
	}
}
#pragma endregion

#pragma region void CUser::PartyAlert(Packet & pkt)
void CUser::PartyAlert(Packet& pkt)
{
	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
		return;

	uint8 SubOpCode = 0; uint32 EffectID = 0;
	pkt >> SubOpCode >> EffectID;

	if (m_lasttargetnumbertime > UNIXTIME2) return;
	m_lasttargetnumbertime = UNIXTIME2 + 850;

	if (!isInGame() || isDead() || !isInParty() || !isPartyCommandLeader()) return;

	Packet result(WIZ_PARTY, uint8(PARTY_ALERT));
	result << SubOpCode;
	g_pMain->Send_PartyMember(GetPartyID(), &result);
}
#pragma endregion

#pragma region void CUser::PartyCommand(Packet & pkt)
void CUser::PartyCommand(Packet& pkt)
{
	int16 NenberID = -1; pkt >> NenberID;

	if (m_lasttargetnumbertime > UNIXTIME2) return;
	m_lasttargetnumbertime = UNIXTIME2 + 850;

	if (!isInGame() || !isInParty() || !isPartyCommandLeader()) return;
	CUser* pUser = g_pMain->GetUserPtr(NenberID);
	if (pUser == nullptr || pUser->GetPartyID() != GetPartyID()) return;
	std::swap(m_bPartyCommandLeader, pUser->m_bPartyCommandLeader);
	Packet result(WIZ_PARTY, uint8(PARTY_COMMAND_PROMATE));
	result.SByte();
	result << pUser->GetSocketID() << pUser->GetName();
	g_pMain->Send_PartyMember(GetPartyID(), &result);
}
#pragma endregion

#pragma region void CUser::PartyTargetNumber(Packet & pkt)
void CUser::PartyTargetNumber(Packet& pkt)
{
	int8 Succes = -1; int16 TargetID = -1; uint32 EffectID = 0;

	if (m_lasttargetnumbertime > UNIXTIME2) return;
	m_lasttargetnumbertime = UNIXTIME2 + 850;

	pkt >> TargetID >> EffectID >> Succes;
	if (!isInGame() || isDead() || !isInParty() || !isPartyCommandLeader()) return;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
		return;

	pParty->NumberTargetID = TargetID;
	Packet result(WIZ_PARTY, uint8(PARTY_TARGET_NUMBER));
	result << TargetID << Succes;
	g_pMain->Send_PartyMember(GetPartyID(), &result);
}
#pragma endregion

#pragma region CGameServerDlg::DeleteParty(uint16 sIndex)
void CGameServerDlg::DeleteParty(uint16 sIndex)
{
	m_PartyArray.DeleteData(sIndex);
}
#pragma endregion

void CUser::PartyBBS(Packet& pkt)
{
	uint8 Type; uint8 opcode;
	pkt >> Type >> opcode;

	if (Type != 0 || !isInGame() || isSellingMerchantingPreparing())
		return;

	switch (opcode)
	{
	case PARTY_BBS_REGISTER:
		PartyBBSRegister(pkt);
		break;
	case PARTY_BBS_DELETE:
		PartyBBSDelete(pkt);
		break;
	case PARTY_BBS_NEEDED:
		PartyBBSNeeded(pkt);
		break;
	case PARTY_BBS_WANTED:
		PartyBBSWanted(pkt);
		break;
	case PARTY_BBS_PARTY_CHANGE:
		PartyBBSPartyChange(pkt);
		break;
	case PARTY_BBS_PARTY_DELETE:
		PartyBBSPartyDelete(pkt);
		break;
	case PARTY_BBS_LIST:
		SendPartyBBSNeeded(pkt);
		break;
	default:
		printf("Party Seeking System Type 0 unhandled Opcode %d \n", opcode);
		break;
	}
}

void CUser::PartyBBSPartyDelete(Packet& pkt)
{
	if (m_bNeedParty == 1
		|| !isPartyLeader())
		return;

	// You don't need anymore 
	g_pMain->m_SeekingPartyArrayLock.lock();
	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		if ((*itr)->m_sSid == GetID())
		{
			g_pMain->m_SeekingPartyArray.erase(itr);
			break;
		}
	}
	g_pMain->m_SeekingPartyArrayLock.unlock();

	StateChangeServerDirect(2, 1); // not looking for a party
	SendPartyBBSNeeded(pkt);
}

void CUser::PartyBBSPartyChange(Packet& pkt)
{
	uint16 page_index = 0;
	if (!isPartyLeader())
		return;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
		return;

	Packet result(WIZ_PARTY_BBS, uint8(PARTY_TYPE_SEEKING));
	pkt.DByte();
	pkt >> pParty->sWantedClass >> page_index >> pParty->WantedMessage;
	result << uint8(PARTY_BBS_WANTED) << uint8(1);

	_SEEKING_PARTY_USER* pPartyUser = nullptr;
	uint16 seeking_index = 0; // holds the index where we located at in the vector
	g_pMain->m_SeekingPartyArrayLock.lock();

	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		seeking_index++;
		if ((*itr)->m_sSid == GetID())
		{
			pPartyUser = (*itr);
			pPartyUser->m_strSeekingNote = pParty->WantedMessage;
			pPartyUser->m_sClass = pParty->sWantedClass;
			break;
		}
	}

	SendPartyBBSNeeded(pkt);

	uint16 counter = 0;
	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		counter++;
		if (counter < seeking_index)
			continue;

		result << (*itr)->m_strUserID << uint8((*itr)->m_sLevel) << (*itr)->m_sClass;
		if (counter >= 12)
			break;
	}

	g_pMain->m_SeekingPartyArrayLock.unlock();

	uint16 s = counter - seeking_index;
	// You still need to fill up ten slots.
	if (s < MAX_BBS_PAGE)
	{
		for (int j = s; j < MAX_BBS_PAGE; j++)
			result << uint16(0) << uint16(0)
			<< uint16(0) << uint8(0)
			<< uint8(0) << uint8(0)
			<< uint16(0)
			<< uint8(0);
	}
	result << uint16(0) << uint8(0) << uint16(s);
	Send(&result);
}

void CUser::PartyBBSRegister(Packet& pkt)
{
	_SEEKING_PARTY_USER* pUser = nullptr;
	string seeking_msg = "";
	Packet result(WIZ_PARTY_BBS, uint8(PARTY_TYPE_SEEKING));
	pkt.SByte();
	pkt >> seeking_msg;

	result << uint8(PARTY_BBS_REGISTER) << uint8(1);
	StateChangeServerDirect(2, 2); // seeking a party

	uint16 seeking_index = 0; // holds the index where we located at in the vector
	g_pMain->m_SeekingPartyArrayLock.lock();

	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		seeking_index++;
		if ((*itr)->m_sSid == GetID())
		{
			pUser = (*itr);
			pUser->m_strSeekingNote = seeking_msg;
			break;
		}
	}

	if (pUser == nullptr)
	{
		pUser = new _SEEKING_PARTY_USER;
		pUser->m_bNation = GetNation();
		pUser->m_strUserID = GetName();
		pUser->m_sClass = GetClass();
		pUser->m_bSeekType = 0;
		pUser->m_bZone = GetZoneID();
		pUser->m_strSeekingNote = seeking_msg;
		pUser->m_sLevel = GetLevel();
		pUser->m_sSid = GetID();
		pUser->m_sPartyID = GetPartyID();
		pUser->isPartyLeader = isPartyLeader();
		pUser->m_bloginType = 0;
		g_pMain->m_SeekingPartyArray.push_back(pUser);
	}

	SendPartyBBSNeeded(pkt);

	uint16 counter = 0;

	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		counter++;
		if (counter < seeking_index)
			continue;

		result << (*itr)->m_strUserID << uint8((*itr)->m_sLevel) << (*itr)->m_sClass;
		if (counter >= 12)
			break;
	}

	g_pMain->m_SeekingPartyArrayLock.unlock();

	uint16 s = counter - seeking_index;
	// You still need to fill up ten slots.
	if (s < MAX_BBS_PAGE)
	{
		for (int j = s; j < MAX_BBS_PAGE; j++)
			result << uint16(0) << uint16(0)
			<< uint16(0) << uint8(0)
			<< uint8(0) << uint8(0)
			<< uint16(0)
			<< uint8(0);
	}
	result << uint16(0) << uint8(0) << uint16(s);

	Send(&result);
}

void CUser::PartyBBSDelete(Packet& pkt)
{
	if (m_bNeedParty == 1)
		return;

	g_pMain->m_SeekingPartyArrayLock.lock();
	// You don't need anymore 
	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		if ((*itr)->m_sSid == GetID())
		{
			g_pMain->m_SeekingPartyArray.erase(itr);
			break;
		}
	}
	g_pMain->m_SeekingPartyArrayLock.unlock();
	StateChangeServerDirect(2, 1); // not looking for a party
	SendPartyBBSNeeded(pkt);
}

void CUser::PartyBBSNeeded(Packet& pkt)
{
	/*uint16 page_index = 0;
	uint8 typefilter = 0, locationFilter = 0, levelFilter = 0;
	pkt >> page_index >> typefilter >> locationFilter >> levelFilter;
	SendPartyBBSNeeded(page_index,type,typefilter, locationFilter, levelFilter);*/
}

void CUser::SendPartyBBSNeeded(Packet& pkt)
{
	Packet result(WIZ_PARTY_BBS);
	uint16 page_index = 0;
	uint8 typefilter = 0, locationFilter = 0, levelFilter = 0;
	pkt >> page_index >> typefilter >> locationFilter >> levelFilter;
	uint16 start_counter = 0, BBS_Counter = 0;
	start_counter = page_index * MAX_BBS_PAGE;

	if (start_counter >= MAX_USER) {
		result << uint8(PARTY_TYPE_SEEKING) << uint8(PARTY_BBS_NEEDED) << uint8(0);
		Send(&result);
		return;
	}


	int MykoSoft_counter = 0;
	result << uint8(PARTY_TYPE_SEEKING) << uint8(11) << uint8(1) << page_index << uint16(BBS_Counter);
	Guard lock(g_pMain->m_SeekingPartyArrayLock);
	foreach(itr, g_pMain->m_SeekingPartyArray) {
		_SEEKING_PARTY_USER* pUser = (*itr);
		if (pUser == nullptr) continue;
		uint8 PartyMembers = 0;
		uint16 sClass = pUser->m_sClass;

		MykoSoft_counter++;
		int x_size = MykoSoft_counter > 0 ? MykoSoft_counter - 1 : MykoSoft_counter;
		if (x_size < page_index * MAX_BBS_PAGE) continue;

		if ((typefilter == 2 && pUser->isPartyLeader == 1)
			|| (typefilter == 3 && pUser->isPartyLeader == 0)
			|| (locationFilter > 0 && locationFilter != pUser->m_bZone)
			|| (pUser->m_bloginType == 2)
			|| (levelFilter > 0 && levelFilter == 1 && pUser->m_sLevel > 11)
			|| (levelFilter > 0 && levelFilter == 2 && (pUser->m_sLevel < 11 || pUser->m_sLevel > 20))
			|| (levelFilter > 0 && levelFilter == 3 && (pUser->m_sLevel < 21 || pUser->m_sLevel > 30))
			|| (levelFilter > 0 && levelFilter == 4 && (pUser->m_sLevel < 31 || pUser->m_sLevel > 40))
			|| (levelFilter > 0 && levelFilter == 5 && (pUser->m_sLevel < 41 || pUser->m_sLevel > 50))
			|| (levelFilter > 0 && levelFilter == 6 && (pUser->m_sLevel < 51 || pUser->m_sLevel > 60))
			|| (levelFilter > 0 && levelFilter == 7 && (pUser->m_sLevel < 61 || pUser->m_sLevel > 70))
			|| (levelFilter > 0 && levelFilter == 8 && (pUser->m_sLevel < 71 || pUser->m_sLevel > 80))
			|| (levelFilter > 0 && levelFilter == 9 && pUser->m_sLevel < 81))
			continue;

		BBS_Counter++;
		if (BBS_Counter >= MAX_BBS_PAGE) break;

		if (pUser->isPartyLeader) {
			auto* pParty = g_pMain->GetPartyPtr(pUser->m_sPartyID);
			if (pParty == nullptr) return;
			pUser->m_strSeekingNote = pParty->WantedMessage;
			PartyMembers = GetPartyMemberAmount(pParty);
			sClass = pParty->sWantedClass;
		}

		result.DByte();
		result << pUser->m_bNation << uint8(pUser->m_bSeekType) << pUser->m_strUserID << sClass
			<< uint16(0) << pUser->m_sLevel << uint8(pUser->isPartyLeader ? 3 : 2);
		result.SByte();
		result << pUser->m_strSeekingNote << pUser->m_bZone << PartyMembers;
	}

	// You still need to fill up ten slots.
	if (BBS_Counter < MAX_BBS_PAGE) {
		for (int j = BBS_Counter; j < MAX_BBS_PAGE; j++) result << uint16(0) << uint16(0) << uint16(0) << uint8(0) << uint8(0) << uint8(0) << uint16(0) << uint8(0);
	}

	int page_number = (int)g_pMain->m_SeekingPartyArray.size() / MAX_BBS_PAGE;
	if (page_number % MAX_BBS_PAGE != 0) page_number++;
	result << page_index << page_number;
	result.put(5, BBS_Counter);
	Send(&result);
}
void CUser::PartyBBSWanted(Packet& pkt)
{
	uint16 page_index = 0;
	if (!isPartyLeader())
		return;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
		return;

	Packet result(WIZ_PARTY_BBS, uint8(PARTY_TYPE_SEEKING));
	pkt.DByte();
	pkt >> pParty->sWantedClass >> page_index >> pParty->WantedMessage;
	result << uint8(PARTY_BBS_WANTED) << uint8(1);

	_SEEKING_PARTY_USER* pPartyUser = nullptr;
	uint16 seeking_index = 0; // holds the index where we located at in the vector

	g_pMain->m_SeekingPartyArrayLock.lock();

	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		seeking_index++;
		if ((*itr)->m_sSid == GetID())
		{
			pPartyUser = (*itr);
			pPartyUser->m_strSeekingNote = pParty->WantedMessage;
			pPartyUser->m_sClass = pParty->sWantedClass;
			break;
		}
	}
	
	
	if (pPartyUser == nullptr)
	{
		pPartyUser = new _SEEKING_PARTY_USER;
		pPartyUser->m_bNation = GetNation();
		pPartyUser->m_strUserID = GetName();
		pPartyUser->m_sClass = pParty->sWantedClass;
		pPartyUser->m_bSeekType = 0;
		pPartyUser->m_bZone = GetZoneID();
		pPartyUser->m_strSeekingNote = pParty->WantedMessage;
		pPartyUser->m_sLevel = GetLevel();
		pPartyUser->m_sSid = GetID();
		pPartyUser->m_sPartyID = GetPartyID();
		pPartyUser->isPartyLeader = isPartyLeader();
		pPartyUser->m_bloginType = 0;
		g_pMain->m_SeekingPartyArray.push_back(pPartyUser);
	}

	StateChangeServerDirect(2, 3); // Looking for party nember
	SendPartyBBSNeeded(pkt);

	uint16 counter = 0;

	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
		if ((*itr) == nullptr)
			continue;

		counter++;
		if (counter < seeking_index)
			continue;

		result << (*itr)->m_strUserID << uint8((*itr)->m_sLevel) << (*itr)->m_sClass;
		if (counter >= 12)
			break;
	}

	g_pMain->m_SeekingPartyArrayLock.unlock();

	uint16 s = counter - seeking_index;
	// You still need to fill up ten slots.
	if (s < MAX_BBS_PAGE)
	{
		for (int j = s; j < MAX_BBS_PAGE; j++)
			result << uint16(0) << uint16(0)
			<< uint16(0) << uint8(0)
			<< uint8(0) << uint8(0)
			<< uint16(0)
			<< uint8(0);
	}
	result << uint16(0) << uint8(0) << uint16(s);
	Send(&result);
}

void CUser::PartyBBSUserLoqOut()
{
	g_pMain->m_SeekingPartyArrayLock.lock();

	if (g_pMain->m_SeekingPartyArray.size() == 0)
	{
		g_pMain->m_SeekingPartyArrayLock.unlock();
		return;
	}

	foreach(itr, g_pMain->m_SeekingPartyArray)
	{
			if ((*itr) == nullptr)
				continue;

			if ((*itr)->m_sSid == GetID())
			{
				g_pMain->m_SeekingPartyArray.erase(itr);
				break;
			}
	}
	g_pMain->m_SeekingPartyArrayLock.unlock();

}

uint8 CUser::GetPartyMemberAmount(_PARTY_GROUP* pParty)
{
	if (pParty == nullptr)
		pParty = g_pMain->GetPartyPtr(GetPartyID());

	if (pParty == nullptr)
		return 0;

	uint8 PartyMembers = 0;
	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] >= 0)
			PartyMembers++;
	}
	return PartyMembers;
}

void CUser::SendPartyStatusUpdate(uint8 bStatus, uint8 bResult /*= 0*/)
{
	if (!isInParty()) return;

	Packet result(WIZ_PARTY, uint8(PARTY_STATUSCHANGE));
	result << GetSocketID() << bStatus << bResult;
	g_pMain->Send_PartyMember(GetPartyID(), &result);
}

void CUser::GrantChickenManner()
{
	uint8 bLevel = GetLevel(), bManner = 0;
	// No manner points if you're not a chicken anymore nor when you're not in a party.
	if (!m_bIsChicken || !isInParty())
		return;

	_PARTY_GROUP* pParty = nullptr;
	pParty = g_pMain->GetPartyPtr(GetPartyID());

	if (pParty == nullptr)
		return;

	for (int i = 0; i < MAX_PARTY_USERS; i++) {
		CUser* pTargetUser = nullptr;
		if (pParty->uid[i] != GetSocketID()) pTargetUser = g_pMain->GetUserPtr(pParty->uid[i]);

		if (pTargetUser == nullptr || pTargetUser->isDead() || pTargetUser->m_bIsChicken) continue;
		if (!isInRange(pTargetUser, RANGE_50M)) continue;

		if (pTargetUser->GetLevel() > 20 && pTargetUser->GetLevel() < 40) bManner = pTargetUser->GetLevel() / 10;
		else bManner = 1;
		pTargetUser->SendMannerChange(bManner);
	}
}

void CBot::BotPartyInviteProcess()
{
	if (!isInGame()
		|| isDead()
		|| IsPartyRestrictedZone(GetZoneID())
		|| (m_BotState != BOT_MOVE && m_BotState != BOT_FARMER)
		|| (m_BotState == BOT_FARMER && !m_bPartyLeader)
		|| m_tPartyInviteTime > UNIXTIME)
		return;

	if (isInParty() && !isPartyLeader())
		return;

	_PARTY_GROUP* pParty = nullptr;
	if (isInParty())
	{
		pParty = g_pMain->GetPartyPtr(GetPartyID());
		if (pParty == nullptr || pParty->uid[0] != GetID())
			return;

		int memberCount = 0;
		for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
		{
			if (pParty->uid[i] >= 0)
				memberCount++;
		}

		if (memberCount >= MAX_PARTY_USERS)
			return;
	}

	// Priority: farm/move bot lideri yakindaki gercek kullaniciyi davet eder, sonra bos slotlari botlarla tamamlar.

	CUser* pInvitee = nullptr;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr
			|| !pUser->isInGame()
			|| pUser->isDead()
			|| pUser->isInApprovedParty()
			|| (m_BotState == BOT_MOVE && !IsUserSeekingParty(pUser))
			|| pUser->GetNation() != GetNation()
			|| pUser->GetZoneID() != GetZoneID()
			|| !isInRangeSlow(pUser, RANGE_50M))
			continue;

		if (!IsBotPartyLevelValid(pUser, this))
			continue;

		pInvitee = pUser;
		break;
	}

	m_tPartyInviteTime = UNIXTIME + 15;
	if (pInvitee == nullptr)
	{
		pParty = FillBotPartyWithBots(this, pParty);
		return;
	}

	if (pParty == nullptr)
		pParty = g_pMain->CreateParty(this);

	if (pParty == nullptr)
		return;

	pInvitee->m_sUserPartyType = 0;
	pInvitee->m_sPartyIndex = m_sPartyIndex;
	pInvitee->m_bInParty = true;
	pInvitee->m_bInEnterParty = false;

	Packet party(WIZ_PARTY);
	party << uint8(PARTY_PERMIT) << GetID() << GetName();
	pInvitee->Send(&party);

	// Top up remaining slots with bots after user invite.
	pParty = FillBotPartyWithBots(this, pParty);
}
