#include "StdAfx.h"

void CBot::SendLoyaltyChange(int32 nChangeAmount /*= 0*/, bool bIsKillReward /* false */, bool bIsBonusReward /* false */, bool bIsAddLoyaltyMonthly /* true */)
{
	uint32 nClanLoyaltyAmount = 0;
	int32 nChangeAmountLoyaltyMonthly = m_bNPGainAmount * nChangeAmount / 100;

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

		if (m_iLoyalty <= 0)
			m_iLoyalty = myrand(1, RIVALRY_NP_BONUS);

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

				if (m_iLoyaltyMonthly <= 0)
					m_iLoyaltyMonthly = myrand(1, RIVALRY_NP_BONUS);
			}
		}
	}
	// We're simply adding NP here.
	else
	{
		// If you're using an NP modifying buff then add the bonus
		nChangeAmount = m_bNPGainAmount * nChangeAmount / 100;

		// Add on any additional NP earned because of a global NP event.
		// NOTE: They officially check to see if the NP is <= 100,000.
		nChangeAmount = nChangeAmount * (100 + g_pMain->m_byNPEventAmount) / 100;

		// We should only apply NP bonuses when NP was gained as a reward for killing a player.
		if (bIsKillReward)
		{
			// Add on any additional NP gained from items/skills.
			nChangeAmount += m_bItemNPBonus + m_bSkillNPBonus;

			// Add monument bonus.
			if (isInPKZone() && GetPVPMonumentNation() == GetNation())
				nChangeAmount += PVP_MONUMENT_NP_BONUS;
		}

		if (m_iLoyalty + nChangeAmount > LOYALTY_MAX)
			m_iLoyalty = LOYALTY_MAX;
		else
			m_iLoyalty += nChangeAmount;

		if ((isInPKZone() && !bIsBonusReward))
		{
			if (GetZoneID() == ZONE_ARDREAM || GetZoneID() == ZONE_RONARK_LAND_BASE)
				bIsAddLoyaltyMonthly = false;

			m_PlayerKillingLoyaltyPremiumBonus += myrand(1, 5);

			if (m_PlayerKillingLoyaltyDaily + nChangeAmount > LOYALTY_MAX)
				m_PlayerKillingLoyaltyDaily = LOYALTY_MAX;
			else
				m_PlayerKillingLoyaltyDaily += nChangeAmount;

			UpdatePlayerKillingRank();
		}

		//// We should only apply additional monthly NP when NP was gained as a reward for killing a player.
		if (!bIsBonusReward)
		{
			if (bIsAddLoyaltyMonthly)
			{
				if (nChangeAmountLoyaltyMonthly > 40)
					nChangeAmountLoyaltyMonthly -= 20;
				else if (nChangeAmountLoyaltyMonthly >= 20 && nChangeAmountLoyaltyMonthly < 40)
					nChangeAmountLoyaltyMonthly -= 10;

				if (m_iLoyaltyMonthly + nChangeAmountLoyaltyMonthly > LOYALTY_MAX)
					m_iLoyaltyMonthly = LOYALTY_MAX;
				else
					m_iLoyaltyMonthly += nChangeAmountLoyaltyMonthly;
			}
		}
	}
}

void CBot::LoyaltyChange(int16 tid, uint16 bonusNP /*= 0*/)
{
	short loyalty_source = 0, loyalty_target = 0;

	CUser* pTUser = g_pMain->GetUserPtr(tid);
	if (pTUser == nullptr || !pTUser->isInGame())
		return;

	if (GetZoneID() == ZONE_DELOS)
	{
		if (pTUser->GetNation() == GetNation())
		{
			if (!GetMap()->canAttackSameNation())
				return;
		}
		else
		{
			if (!GetMap()->canAttackOtherNation())
				return;
		}
	}
	else
	{
		// TODO: Rewrite this out, it shouldn't handle all cases so generally like this
		if ((!GetMap()->isNationPVPZone())
			|| GetZoneID() == ZONE_DESPERATION_ABYSS
			|| GetZoneID() == ZONE_HELL_ABYSS
			|| GetZoneID() == ZONE_DRAGON_CAVE
			|| GetZoneID() == ZONE_CAITHAROS_ARENA)
			return;
	}

	if (pTUser->GetNation() != GetNation())
	{
		if (pTUser->GetLoyalty() == 0)
		{
			int64 nExpLost = 0;
			loyalty_source = 0;
			loyalty_target = 0;

			if (GetMap()->m_bExpLost != 0)
			{
				bool isNationZoneExpLost = ((GetNation() == KARUS && pTUser->GetZoneID() == ZONE_ELMORAD) || (GetNation() == ELMORAD && pTUser->GetZoneID() == ZONE_KARUS));

				if (isNationZoneExpLost)
					nExpLost = pTUser->m_iMaxExp / 100;
				else
					nExpLost = pTUser->m_iMaxExp / 20;

				pTUser->ExpChange("Exp Lost", -nExpLost);
				goto fail_return;
			}
		}
		// Ardream
		else if (pTUser->GetZoneID() == ZONE_ARDREAM)
		{
			loyalty_source = g_pMain->m_Loyalty_Ardream_Source;
			loyalty_target = g_pMain->m_Loyalty_Ardream_Target;
		}
		// Ronark Land Base
		else if (pTUser->GetZoneID() == ZONE_RONARK_LAND_BASE)
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Base_Source;
			loyalty_target = g_pMain->m_Loyalty_Ronark_Land_Base_Target;
		}
		else if (pTUser->GetZoneID() == ZONE_RONARK_LAND)
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Source;
			loyalty_target = g_pMain->m_Loyalty_Ronark_Land_Target;
		}
		else if (pTUser->GetZoneID() == ZONE_KARUS
			|| pTUser->GetZoneID() == ZONE_ELMORAD
			|| (pTUser->GetZoneID() >= ZONE_BATTLE && pTUser->GetZoneID() <= ZONE_BATTLE6))
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Source;
			loyalty_target = g_pMain->m_Loyalty_Other_Zone_Target;
		}
		// Other zones
		else
		{
			loyalty_source = g_pMain->m_Loyalty_Other_Zone_Source;
			loyalty_target = g_pMain->m_Loyalty_Other_Zone_Target;
		}
	}

	// Include any bonus NP (e.g. rival NP bonus)
	loyalty_source += bonusNP;
	loyalty_target -= bonusNP;

	SendLoyaltyChange(loyalty_source, true, false, pTUser->GetMonthlyLoyalty() > 0 ? true : false);
	pTUser->SendLoyaltyChange("PlayerKill", loyalty_target, true, false, pTUser->GetMonthlyLoyalty() > 0 ? true : false);
fail_return:
	// TODO: Move this to a better place (death handler, preferrably)
	// If a war's running, and we died/killed in a war zone... (this method should NOT be so tied up in specifics( 
	if (g_pMain->isWarOpen() && GetMap()->isWarZone())
	{
		// Update the casualty count
		if (pTUser->GetNation() == KARUS)
			g_pMain->m_sKarusDead++;
		else
			g_pMain->m_sElmoradDead++;
	}
}

void CBot::LoyaltyDivide(int16 tid, uint16 bonusNP /*= 0*/)
{
	int16 loyalty_source = 0, loyalty_target = 0;
	uint8 total_member = 0;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
		return;

	CUser* pTUser = g_pMain->GetUserPtr(tid);
	if (pTUser == nullptr || !pTUser->isInGame())
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
		if (pTUser->GetNation() == KARUS)
			g_pMain->m_sKarusDead++;
		else
			g_pMain->m_sElmoradDead++;
	}

	if (pTUser->GetNation() != GetNation())
	{
		if (pTUser->GetLoyalty() == 0) // No cheats allowed...
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
			pUser->SendLoyaltyChange("Loyalty Change", loyalty_source + bonusNP, true, false, pTUser->GetMonthlyLoyalty() > 0 ? true : false);
	}

	pTUser->SendLoyaltyChange("Loyalty Change", loyalty_target, true, false, pTUser->GetMonthlyLoyalty() > 0 ? true : false);
}

void CBot::LoyaltyBotChange(int16 tid, uint16 bonusNP /*= 0*/)
{
	short loyalty_source = 0, loyalty_target = 0;

	CBot* pTUser = g_pMain->GetBotPtr(tid);
	if (pTUser == nullptr || !pTUser->isInGame())
		return;

	if (GetZoneID() == ZONE_DELOS)
	{
		if (pTUser->GetNation() == GetNation())
		{
			if (!GetMap()->canAttackSameNation())
				return;
		}
		else
		{
			if (!GetMap()->canAttackOtherNation())
				return;
		}
	}
	else
	{
		// TODO: Rewrite this out, it shouldn't handle all cases so generally like this
		if ((!GetMap()->isNationPVPZone())
			|| GetZoneID() == ZONE_DESPERATION_ABYSS
			|| GetZoneID() == ZONE_HELL_ABYSS
			|| GetZoneID() == ZONE_DRAGON_CAVE
			|| GetZoneID() == ZONE_CAITHAROS_ARENA)
			return;
	}

	if (pTUser->GetNation() != GetNation())
	{
		// Ardream
		if (pTUser->GetZoneID() == ZONE_ARDREAM)
		{
			loyalty_source = g_pMain->m_Loyalty_Ardream_Source;
			loyalty_target = g_pMain->m_Loyalty_Ardream_Target;
		}
		// Ronark Land Base
		else if (pTUser->GetZoneID() == ZONE_RONARK_LAND_BASE)
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Base_Source;
			loyalty_target = g_pMain->m_Loyalty_Ronark_Land_Base_Target;
		}
		else if (pTUser->GetZoneID() == ZONE_RONARK_LAND)
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Source;
			loyalty_target = g_pMain->m_Loyalty_Ronark_Land_Target;
		}
		else if (pTUser->GetZoneID() == ZONE_KARUS
			|| pTUser->GetZoneID() == ZONE_ELMORAD
			|| (pTUser->GetZoneID() >= ZONE_BATTLE && pTUser->GetZoneID() <= ZONE_BATTLE6))
		{
			loyalty_source = g_pMain->m_Loyalty_Ronark_Land_Source;
			loyalty_target = g_pMain->m_Loyalty_Other_Zone_Target;
		}
		// Other zones
		else
		{
			loyalty_source = g_pMain->m_Loyalty_Other_Zone_Source;
			loyalty_target = g_pMain->m_Loyalty_Other_Zone_Target;
		}
	}

	// Include any bonus NP (e.g. rival NP bonus)
	loyalty_source += bonusNP;
	loyalty_target -= bonusNP;

	SendLoyaltyChange(loyalty_source, true, false, pTUser->GetMonthlyLoyalty() > 0 ? true : false);
	pTUser->SendLoyaltyChange(loyalty_target, true, false, pTUser->GetMonthlyLoyalty() > 0 ? true : false);
	// TODO: Move this to a better place (death handler, preferrably)
	// If a war's running, and we died/killed in a war zone... (this method should NOT be so tied up in specifics( 
	if (g_pMain->isWarOpen() && GetMap()->isWarZone())
	{
		// Update the casualty count
		if (pTUser->GetNation() == KARUS)
			g_pMain->m_sKarusDead++;
		else
			g_pMain->m_sElmoradDead++;
	}
}

void CBot::LoyaltyBotDivide(int16 tid, uint16 bonusNP /*= 0*/)
{
	int16 loyalty_source = 0, loyalty_target = 0;
	uint8 total_member = 0;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
		return;

	CBot* pTBot = g_pMain->GetBotPtr(tid);
	if (pTBot == nullptr || !pTBot->isInGame())
		return;

	for (int i = 0; i < MAX_PARTY_USERS; i++)
	{
		CBot* pBot = g_pMain->GetBotPtr(pParty->uid[i]);
		if (pBot == nullptr)
			continue;

		total_member++;
	}

	if (total_member <= 0)
		return;

	//	This is for the Event Battle on Wednesday :(
	if (g_pMain->isWarOpen()
		&& GetZoneID() == (ZONE_BATTLE_BASE + g_pMain->m_byBattleZone))
	{
		if (pTBot->GetNation() == KARUS)
			g_pMain->m_sKarusDead++;
		else
			g_pMain->m_sElmoradDead++;
	}

	if (pTBot->GetNation() != GetNation())
	{
		if (pTBot->GetLoyalty() == 0) // No cheats allowed...
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
		CBot* pUser = g_pMain->GetBotPtr(pParty->uid[j]);
		if (pUser == nullptr)
			continue;

		if (pUser->isPriest()
			&& pUser->GetID() != GetID()
			&& pUser->hasRival()
			&& !pUser->hasRivalryExpired()
			&& pUser->GetRivalID() == pTBot->GetID())
		{
			bonusNP = RIVALRY_NP_BONUS;
			pUser->RemoveRival();
		}
		else if (pUser->GetID() == GetID()
			&& pUser->hasRival()
			&& !pUser->hasRivalryExpired()
			&& pUser->GetRivalID() == pTBot->GetID())
		{
			bonusNP = RIVALRY_NP_BONUS;
			pUser->RemoveRival();
		}

		if (pUser->isAlive())
			pUser->SendLoyaltyChange(loyalty_source + bonusNP, true, false, pTBot->GetMonthlyLoyalty() > 0 ? true : false);
	}

	pTBot->SendLoyaltyChange(loyalty_target, true, false, pTBot->GetMonthlyLoyalty() > 0 ? true : false);
}

int16 CBot::GetLoyaltyDivideSource(uint8 totalmember)
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

int16 CBot::GetLoyaltyDivideTarget()
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