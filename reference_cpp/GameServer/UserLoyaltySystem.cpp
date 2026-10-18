#include "stdafx.h"
#include "KnightsManager.h"

#pragma region CUser::LoyaltyDivide(int16 tid, uint16 bonusNP /*= 0*/)
/**
* @brief	Takes a target's loyalty points (NP)
* 			and rewards some/all to the killer's party (current user).
*
* @param	tid		The target's ID.
* @param	bonusNP Bonus NP to be awarded to the killer's party as-is.
*/
void CUser::LoyaltyDivide(std::string placeowned, Unit* pTUser, uint16 bonusNP /*= 0*/)
{
	if (pTUser == nullptr
		|| (pTUser->isPlayer() && !TO_USER(pTUser)->isInGame()))
		return;

	int16 loyalty_source = 0, loyalty_target = 0;
	uint8 total_member = 0;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
		return;

	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUser == nullptr)
			continue;

		total_member++;
	}

	if (total_member <= 0)
		return;

	//	This is for the Event Battle on Wednesday :(
	if (g_pMain->isWarOpen()
		&& GetZoneID() == (ZONE_BATTLE_BASE + g_pMain->m_byBattleZone))
	{
		if (pTUser->GetNation() == (uint8)Nation::KARUS)
			g_pMain->m_sKarusDead++;
		else
			g_pMain->m_sElmoradDead++;
	}

	if (pTUser->GetZoneID() == ZONE_DELOS && GetZoneID() == ZONE_DELOS)
	{
		if (g_pMain->m_byBattleSiegeWarOpen)
		{
			uint32 recvLoyalty = 0;
			if (pTUser->isPlayer())
				recvLoyalty = TO_USER(pTUser)->GetLoyalty();
			else
				recvLoyalty = TO_BOT(pTUser)->GetLoyalty();

			if (recvLoyalty == 0) // No cheats allowed...
			{
				loyalty_source = 0;
				loyalty_target = 0;
			}
			else
			{
				loyalty_source = GetLoyaltyDivideSource(total_member);
				loyalty_target = GetLoyaltyDivideTarget();

				if (loyalty_source == 0)
				{
					loyalty_source = 0;
					loyalty_target = 0;
				}
			}
		}
	}
	else if (pTUser->GetNation() != GetNation())
	{
		uint32 recvLoyalty = 0;
		if (pTUser->isPlayer())
			recvLoyalty = TO_USER(pTUser)->GetLoyalty();
		else
			recvLoyalty = TO_BOT(pTUser)->GetLoyalty();

		if (recvLoyalty == 0) // No cheats allowed...
		{
			loyalty_source = 0;
			loyalty_target = 0;
		}
		else
		{
			loyalty_source = GetLoyaltyDivideSource(total_member);
			loyalty_target = GetLoyaltyDivideTarget();

			if (loyalty_source == 0)
			{
				loyalty_source = 0;
				loyalty_target = 0;
			}
		}
	}
	else
		return;

	for (int j = 0; j < MAX_PARTY_USERS; j++) // Distribute loyalty amongst party members.
	{
		bonusNP = 0;
		CUser* pUser = g_pMain->GetUserPtr(pParty->uid[j]);
		if (pUser == nullptr)
			continue;

		if (pUser->isPriest()
			&& pUser->GetID() != GetID()
			&& pUser->hasRival()
			&& !pUser->hasRivalryExpired()
			&& pUser->GetRivalID() == pTUser->GetID())
		{
			bonusNP = RIVALRY_NP_BONUS;
			pUser->RemoveRival();
		}
		else if (pUser->GetID() == GetID()
			&& pUser->hasRival()
			&& !pUser->hasRivalryExpired()
			&& pUser->GetRivalID() == pTUser->GetID())
		{
			bonusNP = RIVALRY_NP_BONUS;
			pUser->RemoveRival();
		}

		if (pUser->isAlive())
		{
			if (pTUser->isPlayer())
				pUser->SendLoyaltyChange(placeowned, loyalty_source + bonusNP, true, false, TO_USER(pTUser)->GetMonthlyLoyalty() > 0 ? true : false);
			else
				pUser->SendLoyaltyChange(placeowned, loyalty_source + bonusNP, true, false, TO_BOT(pTUser)->GetMonthlyLoyalty() > 0 ? true : false);
		}
	}

	if (pTUser->isPlayer())
		TO_USER(pTUser)->SendLoyaltyChange(placeowned, loyalty_target, true, false, TO_USER(pTUser)->GetMonthlyLoyalty() > 0 ? true : false);
	else
		TO_BOT(pTUser)->SendLoyaltyChange(loyalty_target, true, false, TO_BOT(pTUser)->GetMonthlyLoyalty() > 0 ? true : false);
}
#pragma endregion

#pragma region CUser::GetLoyaltyDivideSource(uint8 totalmember)

int16 CUser::GetLoyaltyDivideSource(uint8 totalmember)
{
	int16 nBaseLoyalty = 0;

	if (GetZoneID() == ZONE_ARDREAM)
		nBaseLoyalty = g_pMain->m_Loyalty_Ardream_Source;
	else if (GetZoneID() == ZONE_RONARK_LAND_BASE)
		nBaseLoyalty = g_pMain->m_Loyalty_Ronark_Land_Base_Source;
	else if (GetZoneID() == ZONE_RONARK_LAND)
		nBaseLoyalty = g_pMain->m_Loyalty_Ronark_Land_Source;
	else if (GetZoneID() == ZONE_KROWAZ_DOMINION)
		nBaseLoyalty = (g_pMain->m_Loyalty_Other_Zone_Source / 100) * 20;
	else if (GetZoneID() == ZONE_KARUS
		|| GetZoneID() == ZONE_ELMORAD
		|| (GetZoneID() >= ZONE_BATTLE && GetZoneID() <= ZONE_BATTLE6))
		nBaseLoyalty = g_pMain->m_Loyalty_Ronark_Land_Source;
	else
		nBaseLoyalty = g_pMain->m_Loyalty_Other_Zone_Source;

	int16 nMaxLoyalty = (nBaseLoyalty * 3) - 2;
	int16 nMinLoyalty = nMaxLoyalty / MAX_PARTY_USERS;
	int16 nLoyaltySource = nMinLoyalty;

	if (nLoyaltySource > 0)
	{
		for (int i = 0; i < (MAX_PARTY_USERS - totalmember); i++)
			nLoyaltySource += 2;
	}

	return nLoyaltySource - 1;
}
#pragma endregion

#pragma region CUser::GetLoyaltyDivideTarget()

int16 CUser::GetLoyaltyDivideTarget()
{
	if (GetZoneID() == ZONE_ARDREAM)
		return g_pMain->m_Loyalty_Ardream_Target;
	else if (GetZoneID() == ZONE_RONARK_LAND_BASE)
		return g_pMain->m_Loyalty_Ronark_Land_Base_Target;
	else if (GetZoneID() == ZONE_RONARK_LAND)
		return g_pMain->m_Loyalty_Ronark_Land_Target;
	else if (GetZoneID() == ZONE_KROWAZ_DOMINION)
		return (g_pMain->m_Loyalty_Other_Zone_Target / 100) * 20;
	else
		return g_pMain->m_Loyalty_Other_Zone_Target;

	return 0;
}

#pragma endregion

#pragma region CUser::SendLoyaltyChange(int32 nChangeAmount /*= 0*/, bool bIsKillReward /* false */, bool bIsBonusReward /* false */, bool bIsAddLoyaltyMonthly /* true */)
/**
* @brief	Adjusts a player's loyalty (NP) and sends the loyalty
* 			change packet.
*
* @param	nChangeAmount	The amount to adjust the loyalty points by.
* @param	bIsKillReward	When set to true, enables the use of NP-modifying buffs
*							and includes monthly NP gains.
*/
void CUser::SendLoyaltyChange(std::string placeowned, int32 nChangeAmount /*= 0*/, bool bIsKillReward /* false */, bool bIsBonusReward /* false */, bool bIsAddLoyaltyMonthly /* true */)
{
	Packet result(WIZ_LOYALTY_CHANGE, uint8(LOYALTY_NATIONAL_POINTS));

	uint32 xlog_currentloyalty = GetLoyalty(), x_logbonuamount = 0, nClanLoyaltyAmount = 0;

	CKnights* pmyknight = nullptr;
	if (isInClan()) pmyknight = g_pMain->GetClanPtr(GetClanID());

	int32 nChangeAmountLoyaltyMonthly = m_bNPGainAmount * nChangeAmount / 100, x_logcurrentchangemaount = nChangeAmount;

	uint32 perkLoyalty = 0;
	// If we're taking NP, we need to prevent us from hitting values below 0.
	if (nChangeAmount < 0)
	{
		// Negate the value so it becomes positive (i.e. -50 -> 50)
		// so we can determine if we're trying to take more NP than we have.
		uint32 amt = -nChangeAmount; /* avoids unsigned/signed comparison warning */

		if (amt > m_iLoyalty)
			m_iLoyalty = 0;
		else
			m_iLoyalty -= amt;

		// We should only adjust monthly NP when NP was lost when killing a player.
		if (bIsKillReward)
		{
			if (GetZoneID() == ZONE_ARDREAM || GetZoneID() == ZONE_RONARK_LAND_BASE)
				bIsAddLoyaltyMonthly = false;

			if (bIsAddLoyaltyMonthly)
			{
				if (amt > m_iLoyaltyMonthly)
					m_iLoyaltyMonthly = 0;
				else
					m_iLoyaltyMonthly -= amt;
			}
		}
	}
	// We're simply adding NP here.
	else
	{
		if (bIsKillReward)
		{
			AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveReachNP, 0, nullptr);

			if (pPerks.perkType[(int)perks::loyalty] > 0)
			{
				auto* perks = g_pMain->m_PerksArray.GetData((int)perks::loyalty);
				if (perks && perks->perkCount)
					perkLoyalty += perks->perkCount * pPerks.perkType[(int)perks::loyalty];
			}

			// If you're using an NP modifying buff then add the bonus
			nChangeAmount = m_bNPGainAmount * nChangeAmount / 100;

			// Add on any additional NP earned because of a global NP event.
			// NOTE: They officially check to see if the NP is <= 100,000.
			nChangeAmount = nChangeAmount * (100 + g_pMain->m_byNPEventAmount) / 100;

			if (m_flameilevel > 0 && m_flameilevel <= 3 && g_pMain->pBurningFea[m_flameilevel - 1].droprate)
				nChangeAmount = nChangeAmount * (100 + g_pMain->pBurningFea[m_flameilevel - 1].droprate) / 100;

			// Add on any additional NP gained from items/skills.
			nChangeAmount += m_bItemNPBonus + m_bSkillNPBonus;

			// Add monument bonus.
			if (isInPKZone() && GetPVPMonumentNation() == GetNation())
				nChangeAmount += PVP_MONUMENT_NP_BONUS;

			if (m_FlashWarBonus > 0)
			{
				nChangeAmount += m_FlashWarBonus;
				m_PlayerKillingLoyaltyPremiumBonus += m_FlashWarBonus;
			}

			if (GetPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty) > 0)
			{
				nChangeAmount += (GetPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty));
				m_PlayerKillingLoyaltyPremiumBonus += (GetPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty));
			}

			if (GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty) > 0)
			{
				nChangeAmount += (GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty));
				m_PlayerKillingLoyaltyPremiumBonus += (GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty));
			}

			if (pmyknight && pmyknight->m_bOnlineNpCount > 0)
			{ 
				nChangeAmount += pmyknight->m_bOnlineNpCount;
				m_PlayerKillingLoyaltyPremiumBonus += pmyknight->m_bOnlineNpCount; 
			}

			nChangeAmount += perkLoyalty;
		}

		if (m_iLoyalty + nChangeAmount > LOYALTY_MAX)
			m_iLoyalty = LOYALTY_MAX;
		else
			m_iLoyalty += nChangeAmount;

		if (((isInPKZone() || isInWarZone()) && !bIsBonusReward) || GetZoneID() == ZONE_BORDER_DEFENSE_WAR || GetZoneID() == ZONE_BIFROST)
		{
			if (GetZoneID() == ZONE_ARDREAM || GetZoneID() == ZONE_RONARK_LAND_BASE)
				bIsAddLoyaltyMonthly = false;

			if (m_PlayerKillingLoyaltyDaily + nChangeAmount > LOYALTY_MAX)
				m_PlayerKillingLoyaltyDaily = LOYALTY_MAX;
			else
				m_PlayerKillingLoyaltyDaily += nChangeAmount;

			UpdatePlayerKillingRank();
		}

		if (GetZoneID() == ZONE_SPBATTLE1)
		{
			bIsAddLoyaltyMonthly = false;

			if (m_PlayerKillingLoyaltyDaily + nChangeAmount > LOYALTY_MAX)
				m_PlayerKillingLoyaltyDaily = LOYALTY_MAX;
			else
				m_PlayerKillingLoyaltyDaily += nChangeAmount;

			UpdateZindanWarKillingRank();
		}

		if (bIsAddLoyaltyMonthly && !bIsBonusReward)
		{
			if (nChangeAmountLoyaltyMonthly > 40)
				nChangeAmountLoyaltyMonthly -= 20;
			else if (nChangeAmountLoyaltyMonthly >= 20 && nChangeAmountLoyaltyMonthly < 40)
				nChangeAmountLoyaltyMonthly -= 10;

			if (m_FlashWarBonus > 0)
				nChangeAmountLoyaltyMonthly += m_FlashWarBonus;

			nChangeAmountLoyaltyMonthly += perkLoyalty;

			if (GetPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty) > 0)
				nChangeAmountLoyaltyMonthly += (GetPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty));

			if (GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty) > 0)
				nChangeAmountLoyaltyMonthly += (GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumBonusLoyalty));

			if (pmyknight && pmyknight->m_bOnlineNpCount > 0) { nChangeAmountLoyaltyMonthly += pmyknight->m_bOnlineNpCount; }

			if (m_iLoyaltyMonthly + nChangeAmountLoyaltyMonthly > LOYALTY_MAX)
				m_iLoyaltyMonthly = LOYALTY_MAX;
			else
				m_iLoyaltyMonthly += nChangeAmountLoyaltyMonthly;
		}

		CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
		if (pKnights && pKnights->m_byFlag >= (uint8)ClanTypeFlag::ClanTypeAccredited5 && pKnights->GetClanPointMethod() == 0 && !bIsBonusReward && pKnights->GetID() != 15100 && pKnights->GetID() != 1)   // HUMAN VE KARUS BAÞLANGIÇ CLANLARI ÝÇÝN CLANA NP BAÐIÞI KAPATILDI.
		{
			if (pKnights->m_sMembers > 0 && pKnights->m_sMembers <= MAX_CLAN_USERS)
			{
				if (pKnights->m_sMembers <= 5)
					nClanLoyaltyAmount = 1;
				else if (pKnights->m_sMembers <= 10)
					nClanLoyaltyAmount = 2;
				else if (pKnights->m_sMembers <= 15)
					nClanLoyaltyAmount = 3;
				else if (pKnights->m_sMembers <= 20)
					nClanLoyaltyAmount = 4;
				else if (pKnights->m_sMembers <= 25)
					nClanLoyaltyAmount = 5;
				else if (pKnights->m_sMembers <= 30)
					nClanLoyaltyAmount = 6;
				else if (pKnights->m_sMembers <= 35)
					nClanLoyaltyAmount = 7;
				else if (pKnights->m_sMembers <= 40)
					nClanLoyaltyAmount = 8;
				else if (pKnights->m_sMembers <= 45)
					nClanLoyaltyAmount = 9;
				else if (pKnights->m_sMembers > 45)
					nClanLoyaltyAmount = 10;

				m_iLoyalty -= nClanLoyaltyAmount;
				CKnightsManager::AddUserDonatedNP(GetClanID(), m_strUserID, nClanLoyaltyAmount, true);
				AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveKnightContribution, 0, nullptr);
			}
		}
	}

	// Player is give first np, second exp and third meat dumpling etc.
	if (bIsKillReward && nChangeAmount > 0)
	{
		uint16 sEffect = 0;
	}
	result << m_iLoyalty << m_iLoyaltyMonthly
		<< uint32(0) // Clan donations(? Donations made by this user? For the clan overall?)
		<< nClanLoyaltyAmount; // Premium NP(? Additional NP gained?)
	Send(&result);

	// give first np, second exp and third meat dumpling etc.
	if (bIsKillReward && nChangeAmount > 0)
	{

		if (isInPKZone() /*|| GetMap()->isWarZone()*/)
		{
			/*if (++m_testkillcount > 0) {
				GiveItem("kill dks", 346250000, 1, true);
				m_testkillcount = 0;
			}*/
		}
	}

	if (x_logcurrentchangemaount > (int)120) LoyaltyChangeInsertLog(placeowned, xlog_currentloyalty, x_logcurrentchangemaount, nChangeAmount + x_logbonuamount, GetLoyalty());
}

#pragma endregion

bool CUser::LoyaltyLose(uint32 Loyalty)
{
	if (!hasLoyalty(Loyalty))
		return false;

	m_iLoyalty -= Loyalty;

	Packet result(WIZ_LOYALTY_CHANGE, uint8(LOYALTY_NATIONAL_POINTS));
	result << m_iLoyalty << m_iLoyaltyMonthly
		<< uint32(0)
		<< uint32(0);
	Send(&result);

	return true;
}