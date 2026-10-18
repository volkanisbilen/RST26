#include "stdafx.h"
#include "MagicInstance.h"

using namespace std;

#pragma region CUser::SiegeTransformHpChange(int amount)
void CUser::SiegeTransformHpChange(int amount)
{
	if (amount < 0)
		return;

	Packet result(WIZ_HP_CHANGE);

	// Implement damage/HP cap.
	if (amount < -MAX_DAMAGE)
		amount = -MAX_DAMAGE;
	else if (amount > MAX_DAMAGE)
		amount = MAX_DAMAGE;

	if (amount < 0 && -amount >= m_sHp)
		m_sHp = 0;
	else if (amount >= 0 && m_sHp + amount > m_MaxHp)
		m_sHp = m_MaxHp;
	else
		m_sHp += amount;

	result << m_MaxHp << m_sHp << uint16(-1);
	Send(&result);

	if (isInParty() && GetZoneID() != ZONE_CHAOS_DUNGEON)
		SendPartyHPUpdate();
}
#pragma endregion

#pragma region CUser::HpChange(int amount, Unit *pAttacker /*= nullptr*/, bool isDOT /*= false*/)
/**
* @brief	Changes a user's HP.
*
* @param	amount   	The amount to change by.
* @param	pAttacker	The attacker.
* @param	bSendToAI	true to update the AI server.
*/
void CUser::HpChange(int amount, Unit* pAttacker /*= nullptr*/, bool isDOT /*= false*/)
{
	Packet result;
	uint16 tid = (pAttacker != nullptr ? pAttacker->GetID() : -1);
	int16 oldHP = m_sHp;
	int originalAmount = amount;
	int mirrorDamage = 0;

	// No cheats allowed
	if (pAttacker && pAttacker->GetZoneID() != GetZoneID())
		return;

	// Implement damage/HP cap.
	if (amount < -MAX_DAMAGE)
		amount = -MAX_DAMAGE;
	else if (amount > MAX_DAMAGE)
		amount = MAX_DAMAGE;

	//printf("%s \n",m_strUserID.c_str());

	// If we're taking damage...
	if (amount < 0)
	{
		if (isGM())
			return;

		if (!isDOT)
			RemoveStealth();

		bool NotUseZone = (GetZoneID() == ZONE_CHAOS_DUNGEON || GetZoneID() == ZONE_KNIGHT_ROYALE);

		// Handle the mirroring of damage.
		if (pAttacker && pAttacker->isPlayer() && m_bMirrorDamage && !NotUseZone)
		{
			CUser* pUser = TO_USER(pAttacker);
			if (m_bMirrorDamageType)
			{
				if (pUser && pUser->isInGame())
				{
					mirrorDamage = (m_byMirrorAmount * amount) / 100;
					amount -= mirrorDamage;
					pUser->HpChange(mirrorDamage);
				}
			}
			else if (isInParty())
			{
				mirrorDamage = (m_byMirrorAmount * amount) / 100;
				amount -= mirrorDamage;

				auto* pParty = g_pMain->GetPartyPtr(pUser->GetPartyID());
				if (pParty)
				{

					uint8 p_count = 0;
					for (int i = 0; i < 7; i++)
						if (pParty->uid[i] >= 0)
							p_count++;

					mirrorDamage = mirrorDamage / p_count < 2 ? 2 : p_count;
					for (int i = 0; i < MAX_PARTY_USERS; i++)
					{
						auto* p = g_pMain->GetUserPtr(pParty->uid[i]);
						if (p == nullptr || p == this)
							continue;

						p->HpChange(mirrorDamage);
					}
				}
			}
		}

		// Handle mastery passives
		if (isMastered() && !NotUseZone)
		{
			// Matchless: [Passive]Decreases all damages received by 15%
			if (CheckSkillPoint((uint8)SkillPointCategory::SkillPointMaster, 10, g_pMain->m_byMaxLevel))
				amount = (85 * amount) / 100;
			// Absoluteness: [Passive]Decrease 10 % demage of all attacks
			else if (CheckSkillPoint((uint8)SkillPointCategory::SkillPointMaster, 5, 9))
				amount = (90 * amount) / 100;
		}

		// Handle mana absorb skills
		if (m_bManaAbsorb > 0 && !NotUseZone)
		{
			if ((m_bManaAbsorb == 15 && AbsorbCount > 0) || m_bManaAbsorb != 0)
			{
				if (m_bManaAbsorb == 15) AbsorbCount--;
				int toBeAbsorbed = (originalAmount * m_bManaAbsorb) / 100;
				amount -= toBeAbsorbed;
				MSpChange(toBeAbsorbed);
			}
		}
	}
	// If we're receiving HP and we're undead, all healing must become damage.
	else if (m_bIsUndead)
	{
		amount = -amount;
		originalAmount = amount;
	}

	if (amount < 0 && -amount >= m_sHp)
		m_sHp = 0;
	else if (amount >= 0 && m_sHp + amount > m_MaxHp)
		m_sHp = m_MaxHp;
	else
		m_sHp += amount;

	bool NotUseZone2 = (GetZoneID() == ZONE_CHAOS_DUNGEON || GetZoneID() == ZONE_KNIGHT_ROYALE);

	if (pAttacker != nullptr
		&& pAttacker->isPlayer()
		&& isDevil())
	{
		if (amount < 0)
		{
			int32 Receive = int32(amount / 3.1);

			result.clear();
			result.Initialize(WIZ_KURIAN_SP_CHANGE);
			result << uint8(2) << uint8(1);
			result << Receive;
			Send(&result);

			AbsorbedAmmount += Receive;

			if (m_sHp > 0)
				m_sHp -= int16(Receive);

			if (AbsorbedAmmount <= ABSORBED_TOTAL)
				CMagicProcess::RemoveType4Buff(BUFF_TYPE_DEVIL_TRANSFORM, this);
		}
	}

	result.clear();
	result.Initialize(WIZ_HP_CHANGE);
	result << m_MaxHp << m_sHp << tid;

	if (GetHealth() > 0
		&& isMastered()
		&& !isMage() && !NotUseZone2)
	{
		const uint16 hp30Percent = (30 * GetMaxHealth()) / 100;
		if ((oldHP >= hp30Percent && m_sHp < hp30Percent)
			|| (m_sHp > hp30Percent))
		{
			//SetUserAbility();
			if (m_sHp < hp30Percent)
				ShowEffect(106800); // skill ID for "Boldness", shown when a player takes damage.
		}
	}
	Send(&result);

	if (isInParty() && !NotUseZone2)
	{
		SendPartyHPUpdate();
		SendPartyHpManager(PartyType::Send_All); // Partyde HP Degeri Yazi Olarak Gosterme
	}

	// Ensure we send the original damage (prior to passives) amount to the attacker 
	// as it appears to behave that way officially.
	if (pAttacker != nullptr
		&& pAttacker->isPlayer())
		TO_USER(pAttacker)->SendTargetHP(0, GetID(), originalAmount, false); //target Scroll Bar Eklemesi 03.02.2024 end
	//TO_USER(pAttacker)->SendTargetHP(0, GetID(), originalAmount);

	if (m_sHp <= 0)
		OnDeath(pAttacker);
}

#pragma endregion

#pragma region CUser::SetMaxHp(int iFlag)
/**
* @brief	Calculates & sets a player's maximum HP.
*
* @param	iFlag	If set to 1, additionally resets the HP to max.
* 					If set to 2, additionally resets the max HP to 100 (i.e. Snow war).
*/
void CUser::SetMaxHp(int iFlag)
{
	_CLASS_COEFFICIENT* p_TableCoefficient = nullptr;
	p_TableCoefficient = g_pMain->m_CoefficientArray.GetData(GetClass());

	if (!p_TableCoefficient)
		return;

	int temp_sta = getStatTotal(StatType::STAT_STA);

	if (GetZoneID() == ZONE_SNOW_BATTLE && iFlag == 0)
	{
		if (GetFame() == COMMAND_CAPTAIN || isKing())
			m_MaxHp = 300;
		else
			m_MaxHp = 100;
	}
	else if (GetZoneID() == ZONE_CHAOS_DUNGEON && iFlag == 0
		|| (GetZoneID() == ZONE_DUNGEON_DEFENCE && iFlag == 0))
		m_MaxHp = 10000 / 10;
	else
	{
		m_MaxHp = (short)(((p_TableCoefficient->HP * GetLevel() * GetLevel() * temp_sta)
			+ 0.1 * (GetLevel() * temp_sta) + (temp_sta / 5)) + m_sMaxHPAmount + m_sItemMaxHp + 20);

		if (pPerks.perkType[(int)perks::healt] > 0) {
			auto* perks = g_pMain->m_PerksArray.GetData((int)perks::healt);
			if (perks && perks->perkCount)
				m_MaxHp += perks->perkCount * pPerks.perkType[(int)perks::healt];
		}

		// Awakening Max Healt %20 Artt�rma.
		if (isMasteredKurianPortu())
		{
			if (GetZoneID() != ZONE_KNIGHT_ROYALE
				&& GetZoneID() != ZONE_CHAOS_DUNGEON)
			{
				if (CheckSkillPoint(PRO_SKILL4, 2, 23))
					m_MaxHp += m_MaxHp * 20 / 100;
			}
		}

		// A player's max HP should be capped at (currently) 14,000 HP.
		if (m_MaxHp > g_pMain->pServerSetting.maxplayerhp && !isGM())
			m_MaxHp = g_pMain->pServerSetting.maxplayerhp;

		if (iFlag == 1)
		{
			m_MaxHp = g_pMain->pServerSetting.maxplayerhp;
			HpChange(m_MaxHp);
		}
		else if (iFlag == 2)
			m_MaxHp = 100;
	}

	//Transformation stats need to be applied here
	if (GetZoneID() == ZONE_DELOS && isSiegeTransformation())
	{
		_MAGIC_TYPE6* pType = g_pMain->m_Magictype6Array.GetData(m_sTransformSkillID);

		if (pType != nullptr)
			m_MaxHp = (short)pType->sMaxHp;

		if (m_MaxHp > 0)
		{
			if (m_sTransformHpchange)
			{
				m_sTransformHpchange = false;
				SiegeTransformHpChange(m_MaxHp);
				return;
			}
		}
	}

	if (m_MaxHp < m_sHp)
	{
		m_sHp = m_MaxHp;
		HpChange(m_sHp);
	}
}

#pragma endregion

#pragma region CUser::SetMaxMp()
/**
* @brief	Calculates & sets a player's maximum MP.
*/
void CUser::SetMaxMp()
{
	_CLASS_COEFFICIENT* p_TableCoefficient = nullptr;
	p_TableCoefficient = g_pMain->m_CoefficientArray.GetData(GetClass());
	if (!p_TableCoefficient) return;

	int temp_intel = 0, temp_sta = 0;
	temp_intel = getStatTotal(StatType::STAT_INT) + 30;
	//	if( temp_intel > 255 ) temp_intel = 255;
	temp_sta = getStatTotal(StatType::STAT_STA);
	//	if( temp_sta > 255 ) temp_sta = 255;

	if (p_TableCoefficient->MP != 0)
	{
		m_MaxMp = (short)((p_TableCoefficient->MP * GetLevel() * GetLevel() * temp_intel)
			+ (0.1f * GetLevel() * 2 * temp_intel) + (temp_intel / 5) + m_sMaxMPAmount + m_sItemMaxMp + 20);

		if (pPerks.perkType[(int)perks::mana] > 0)
		{
			auto* perks = g_pMain->m_PerksArray.GetData((int)perks::mana);
			if (perks && perks->perkCount)
				m_MaxMp += perks->perkCount * pPerks.perkType[(int)perks::mana];
		}
	}
	else if (p_TableCoefficient->SP != 0)
	{
		m_MaxMp = (short)((p_TableCoefficient->SP * GetLevel() * GetLevel() * temp_sta)
			+ (0.1f * GetLevel() * temp_sta) + (temp_sta / 5) + m_sMaxMPAmount + m_sItemMaxMp);

		if (pPerks.perkType[(int)perks::mana] > 0)
		{
			auto* perks = g_pMain->m_PerksArray.GetData((int)perks::mana);
			if (perks && perks->perkCount)
				m_MaxMp += perks->perkCount * pPerks.perkType[(int)perks::mana];
		}
	}

	//Transformation stats need to be applied here
	if (GetZoneID() == ZONE_DELOS && isSiegeTransformation())
	{
		_MAGIC_TYPE6* pType = g_pMain->m_Magictype6Array.GetData(m_sTransformSkillID);

		if (pType != nullptr)
			m_MaxMp = (short)pType->sMaxMp;

		if (m_MaxMp > 0)
		{
			if (m_sTransformMpchange)
			{
				m_sTransformMpchange = false;
				MSpChange(m_MaxMp);
				return;
			}
		}
	}

	if (m_MaxMp < m_sMp)
	{
		m_sMp = m_MaxMp;
		MSpChange(m_sMp);
	}
}
#pragma endregion

#pragma region CUser::MSpChange(int amount)
/**
* @brief	Changes a user's mana points.
*
* @param	amount	The amount to adjust by.
*/
void CUser::MSpChange(int amount)
{
	Packet result(WIZ_MSP_CHANGE);
	int16 oldMP = m_sMp;

	if (isGM() && amount < 0)
		return;

	// TODO: Make this behave unsigned.
	m_sMp += amount;
	if (m_sMp < 0)
		m_sMp = 0;
	else if (m_sMp > m_MaxMp)
		m_sMp = m_MaxMp;

	if (isMasteredMage())
	{
		const uint16 mp30Percent = (30 * GetMaxMana()) / 100;
		if (oldMP >= mp30Percent
			&& GetMana() < mp30Percent)
			ShowEffect(106800); // skill ID for "Boldness", shown when a player loses mana.
	}

	result << m_MaxMp << m_sMp;
	Send(&result);

	if (isInParty() && GetZoneID() != ZONE_CHAOS_DUNGEON)
		SendPartyHPUpdate(); // handles MP too
}

#pragma endregion

void CUser::OnDeath(Unit* pKiller)
{
	if (m_bResHpType == USER_DEAD)
		return;

	m_bResHpType = USER_DEAD;

	// JstKO User Deatch Effect Eklendi 01.01.2025
	if (g_pMain->UserDeathEffect)
		ShowEffect(492027);

	OnDeathitDoesNotMatter(pKiller);
	if (pKiller)
	{
		if (pKiller->isPlayer() && pKiller != this)
			OnDeathKilledPlayer(TO_USER(pKiller));
		else if (pKiller->isBot())
			OnDeathKilledBot(TO_BOT(pKiller));
		else if (pKiller->isNPC())
			OnDeathKilledNpc(TO_NPC(pKiller));
	}
	InitOnDeath(pKiller);
}

void CUser::OnDeathitDoesNotMatter(Unit* pKiller)
{
	switch (GetZoneID())
	{
	case ZONE_CHAOS_DUNGEON:
		if (isInTempleEventZone(GetZoneID()))
		{
			if (isEventUser())
				RobChaosSkillItems();
		}
		break;
	case ZONE_BORDER_DEFENSE_WAR:
		if (isInTempleEventZone(GetZoneID()) && isEventUser()) BDWUserHasObtainedLoqOut();
		break;
	case ZONE_BATTLE6:
		if (isTowerOwner())
			TowerExitsFunciton(true);
		break;
	case ZONE_DRAKI_TOWER:
		DrakiTowerKickOuts();
		break;
	case ZONE_FORGOTTEN_TEMPLE:
	case ZONE_DELOS_CASTELLAN:
	case ZONE_DUNGEON_DEFENCE:
		KickOutZoneUser(true, ZONE_MORADON);
		break;
	case ZONE_UNDER_CASTLE:
		ItemWoreOut(UTC_ATTACK, -MAX_DAMAGE);
		ItemWoreOut(UTC_DEFENCE, -MAX_DAMAGE);
		break;
	}
	if (isWantedUser()) NewWantedEventLoqOut(pKiller);
}

void CUser::OnDeathKilledPlayer(CUser* pKiller)
{
	if (!pKiller)
		return;

	CheckRespawnScroll();

	if (pKiller != this)
	{
		bool m_party_check = false;
		bool removerival = false;
		bool specialevent = pKiller->isInSpecialEventZone() && isInSpecialEventZone() && g_pMain->pSpecialEvent.opened;
		bool cindireallaw = g_pMain->pCindWar.isStarted()
			&& pCindWar.isEventUser()
			&& pKiller->pCindWar.isEventUser()
			&& g_pMain->isCindirellaZone(GetZoneID())
			&& g_pMain->isCindirellaZone(pKiller->GetZoneID());

		uint16 bonusNP = 0;
		switch (pKiller->GetZoneID())
		{
		case ZONE_CHAOS_DUNGEON:
			if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
			{
				if (pKiller->isEventUser())
				{
					m_ChaosExpansionDeadCount++;
					UpdateChaosExpansionRank();

					pKiller->m_ChaosExpansionKillCount++;
					pKiller->UpdateChaosExpansionRank();

					pKiller->AchieveWarCountAdd(UserAchieveWarTypes::AchieveKillCountChaos, 0, this);
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
			}
			break;
		case ZONE_BORDER_DEFENSE_WAR:
			if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
			{
				if (pKiller->isEventUser())
				{
					pKiller->BDWUpdateRoomKillCount();
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
			}
			break;
		case ZONE_JURAID_MOUNTAIN:
			if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
			{
				if (pKiller->isEventUser())
				{
					pKiller->JRUpdateRoomKillCount();
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
			}
			break;
		case ZONE_DELOS:
		{
			if (g_pMain->isCswActive() && pKiller->isInClan() && g_pMain->isCswWarActive())
			{
				pKiller->csw_rank_killupdate();
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				m_party_check = true;
			}
		}
		break;
		case ZONE_MORADON:
		case ZONE_MORADON2:
		case ZONE_MORADON3:
		case ZONE_MORADON4:
		case ZONE_MORADON5:
			SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
			break;
		case ZONE_CLAN_WAR_RONARK:
		case ZONE_CLAN_WAR_ARDREAM:
		case ZONE_PARTY_VS_1:
		case ZONE_PARTY_VS_2:
		case ZONE_PARTY_VS_3:
		case ZONE_PARTY_VS_4:
			if (pKiller->isClanTournamentinZone() || pKiller->isPartyTournamentinZone())
			{
				g_pMain->UpdateClanTournamentScoreBoard(pKiller);
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
			}
			break;
		case ZONE_ARDREAM:
		case ZONE_RONARK_LAND:
		case ZONE_RONARK_LAND_BASE:
		{
			m_party_check = true;
			pKiller->KA_KillUpdate();
			KA_AssistDebufUpdate(pKiller);
			bool bKilledByRival = false;

			// PK System icin
			if (!pKiller->isInParty())
			{
				pKiller->killmoney++;
				pKiller->killuser++;

				if (pKiller->killmoney >= 3)
				{
					//pKiller->GoldGain(1000000);
					pKiller->killmoney = 0;
				}

				if (pKiller->killuser >= 5)
				{
					//pKiller->GiveItem("VanGuard", SILVERY_GEM, 1);
					pKiller->killuser = 0;
				}
			}
			// PK System icin end

			// If the killer has us set as their rival, reward them & remove the rivalry.
			bKilledByRival = (!pKiller->hasRivalryExpired() && pKiller->GetRivalID() == GetID());

			if (!cindireallaw && !specialevent && !bKilledByRival)
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);

			if (bKilledByRival)
			{
				if (!cindireallaw && !specialevent)
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeRival, true);

				// Apply bonus NP for rival kills
				bonusNP += RIVALRY_NP_BONUS;

				removerival = true;
				pKiller->AchieveWarCountAdd(UserAchieveWarTypes::AchieveTakeRevange, 0, this);
			}

			if (!hasFullAngerGauge())
				UpdateAngerGauge(++m_byAngerGauge);

			// If we don't have a rival, this player is now our rival for 3 minutes.
			if (!hasRival()) SetRival(pKiller);
		}
		break;
		case ZONE_ELMORAD:
		case ZONE_KARUS:
			if (g_pMain->isWarOpen())
			{
				m_party_check = true;
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
			}
			break;
		case ZONE_BATTLE:
		case ZONE_BATTLE2:
		case ZONE_BATTLE3:
		case ZONE_BATTLE4:
		case ZONE_BATTLE5:
		case ZONE_BATTLE6:
			if (g_pMain->isWarOpen())
			{
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
				m_party_check = true;
			}
			break;
		case ZONE_BIFROST:
			m_party_check = true;
			SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
			break;
		case ZONE_SNOW_BATTLE:
			if (g_pMain->m_byBattleOpen == SNOW_BATTLE)
			{
				pKiller->GoldGain(SNOW_EVENT_MONEY);
				GetNation() == (uint8)Nation::KARUS ? g_pMain->m_sKarusDead++ : g_pMain->m_sElmoradDead++;;
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
			}
			break;
		}
		
		CUser* pUser = TO_USER(pKiller);
		if (pUser && GetZoneID() == ZONE_STAR_NEO_VS && GetEventRoom() > 2000)
			VSEventProcess(1); //vs event

		if (pKiller->isInSpecialEventZone() && isInSpecialEventZone())
			m_party_check = true;

		if (m_party_check)
		{

			_PARTY_GROUP* pParty = nullptr;
			if (pKiller->isInParty() && (pParty = g_pMain->GetPartyPtr(pKiller->GetPartyID())) != nullptr)
			{

				for (int i = 0; i < MAX_PARTY_USERS; i++)
				{
					CUser* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
					if (PartyUser == nullptr || !isInRangeSlow(PartyUser, RANGE_50M)
						|| PartyUser->isDead() || !PartyUser->isInGame() || !PartyUser->isPlayer())
						continue;

					PartyUser->killmoney++;
					PartyUser->killuser++;

					if (PartyUser->killmoney >= 3)
					{
						//PartyUser->GoldGain(1000000);
						PartyUser->killmoney = 0;
					}

					if (PartyUser->killuser >= 5)
					{
						//PartyUser->GiveItem("VanGuard", SILVERY_GEM, 1);
						PartyUser->killuser = 0;
					}

					if (PartyUser->GetNation() == (uint8)Nation::KARUS)
						PartyUser->QuestV2MonsterCountAdd((uint8)Nation::KARUS);
					else if (PartyUser->GetNation() == (uint8)Nation::ELMORAD)
						PartyUser->QuestV2MonsterCountAdd((uint8)Nation::ELMORAD);

					PartyUser->UpdateDailyQuestCount(1);
					if (g_pMain->pCollectionRaceEvent.isCRActive) //15.05.2021
						g_pMain->CollectionRaceSendDead(PartyUser, 0x01);
				}
			}
			else
			{
				if (GetNation() == (uint8)Nation::KARUS)
					pKiller->QuestV2MonsterCountAdd((uint8)Nation::KARUS);
				else if (GetNation() == (uint8)Nation::ELMORAD)
					pKiller->QuestV2MonsterCountAdd((uint8)Nation::ELMORAD);

				pKiller->UpdateDailyQuestCount(1);
				if (g_pMain->pCollectionRaceEvent.isCRActive) //15.05.2021
					g_pMain->CollectionRaceSendDead(pKiller, 0x01);
			}
		}

		// Fallback: some zones do not set m_party_check, but enemy player kills
		// should still progress daily PK quests.
		if (!m_party_check && pKiller->GetNation() != GetNation())
		{
			if (pKiller->isInParty())
			{
				_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(pKiller->GetPartyID());
				if (pParty != nullptr)
				{
					for (int i = 0; i < MAX_PARTY_USERS; i++)
					{
						CUser* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
						if (PartyUser == nullptr
							|| !isInRangeSlow(PartyUser, RANGE_50M)
							|| PartyUser->isDead()
							|| !PartyUser->isInGame()
							|| !PartyUser->isPlayer())
							continue;

						PartyUser->UpdateDailyQuestCount(1);
					}
				}
			}
			else
			{
				pKiller->UpdateDailyQuestCount(1);
			}
		}

		if (specialevent || cindireallaw)
		{
			SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
			if (cindireallaw && pKiller->pCindWar.isEventUser() && pCindWar.isEventUser())
				CindireallaUpdateKDA(pKiller);
			else if (specialevent)
				pKiller->ZindanWarUpdateScore();
		}

		if (pKiller->isInPKZone())
		{
			pKiller->m_KillCount++;
			pKiller->GiveKillReward();
		}


		if (pKiller->GetMap())
		{
			if (pKiller->GetMap()->m_bGiveLoyalty)
			{
				if (pKiller->isInParty()) pKiller->LoyaltyDivide("user kill", this, bonusNP);
				else pKiller->LoyaltyChange(this, bonusNP);
			}

			if (pKiller->GetMap()->m_bGoldLose && !cindireallaw)
				pKiller->GoldChange(GetID(), 0);
		}

		if (removerival && pKiller->isInGame() && pKiller->hasRival())
			pKiller->RemoveRival();

		pKiller->AchieveWarCountAdd(UserAchieveWarTypes::AchieveDefeatEnemy, 0, this);
		pKiller->KillingUserInsertLog(this);
		m_sWhoKilledMe = pKiller->GetID();
	}
	else
		m_sWhoKilledMe = -1;

		// JstKO Yeni Cz User Pvp Kill �d�l Notice Eklendi. 16.04.2025
		if (!pKiller->isInParty())
		{
			pKiller->m_PvpKillCount++;
			pKiller->m_PvpKill50Count++;
			pKiller->m_PvpKill500Count++;
		}

		// JstKO Yeni Pvp Kill Ekran Notice Eklendi.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.

		if (pKiller->m_PvpKillCount == 5)
		{
			// JstKO Yeni Kill�temGift �tem Name Notice Eklendi. 16.04.2025
			_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill�temGift);

			pKiller->GoldGain(g_pMain->KillGoldGift);
			pKiller->CashGain(g_pMain->KillCashGift);
			//pKiller->SendLoyaltyChange("Pvp Kill �d�l", g_pMain->KillNpGift); // Np Gerek Yok.
			pKiller->GiveItem("Pvp Kill �d�l", g_pMain->Kill�temGift, g_pMain->KillCountGift);
			pKiller->m_PvpKillCount = 0;
			// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
			// JstKO: kapatildi - ekstra player effect istemiyoruz.
			SendChat((uint8)ChatType::SHOUT_CHAT, string_format("Vs �ld�n T�h! o �ntikam Bu Nick : [ %s ] Saldirsana.", pKiller->GetName().c_str()), "[Pvp Kill �d�l Kaybetsin]");
			g_pMain->SendHelpDescription(pKiller, string_format("[Pvp Kill �d�l Kazandin] = [ Para : %d ] | [ Cash : %d ] | [ �tem Name : %s x%d ] | [ Kill Count : %d ] �ld�rme G�revi Bitirdin �d�l �tem G�nderildin Tebrikler. ", g_pMain->KillGoldGift, g_pMain->KillCashGift, pTable.m_sName.c_str(), g_pMain->KillCountGift, pKiller->m_PvpKillCount+5));
		}

		if (pKiller->m_PvpKill50Count == 50)
		{
			// JstKO Yeni Kill�50temGift �tem Name Notice Eklendi. 16.04.2025
			_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill50�temGift);

			pKiller->GoldGain(g_pMain->Kill50GoldGift);
			pKiller->CashGain(g_pMain->Kill50CashGift);
			//pKiller->SendLoyaltyChange("Pvp Kill �d�l", g_pMain->Kill50NpGift);
			pKiller->GiveItem("Pvp Kill �d�l", g_pMain->Kill50�temGift, g_pMain->Kill50CountGift);
			pKiller->m_PvpKillCount = 0;
			pKiller->m_PvpKill50Count = 0;
			// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
			// JstKO: kapatildi - ekstra player effect istemiyoruz.
			SendChat((uint8)ChatType::SHOUT_CHAT, string_format("Vs �ld�n T�h! o �ntikam Bu Nick : [ %s ] Saldirsana.", pKiller->GetName().c_str()), "[Pvp Kill �d�l Kaybetsin]");
			g_pMain->SendHelpDescription(pKiller, string_format("[Pvp Kill �d�l Kazandin] = [ Para : %d ] | [ Cash : %d ] | [ �tem Name : %s x%d ] | [ Kill Count : %d ] �ld�rme G�revi Bitirdin �d�l �tem G�nderildin Tebrikler. ", g_pMain->Kill50GoldGift, g_pMain->Kill50CashGift, pTable.m_sName.c_str(), g_pMain->Kill50CountGift, pKiller->m_PvpKill50Count+50));
		}

		if (pKiller->m_PvpKill500Count == 500)
		{
			// JstKO Yeni Kill500�temGift �tem Name Notice Eklendi. 16.04.2025
			_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill500�temGift);

			pKiller->GoldGain(g_pMain->Kill500GoldGift);
			pKiller->CashGain(g_pMain->Kill500CashGift);
			//pKiller->SendLoyaltyChange("Pvp Kill �d�l", g_pMain->Kill500NpGift);
			pKiller->GiveWerehouseItem(g_pMain->Kill500�temGift, g_pMain->Kill500CountGift);
			pKiller->m_PvpKillCount = 0;
			pKiller->m_PvpKill50Count = 0;
			pKiller->m_PvpKill500Count = 0;
			// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
			// JstKO: kapatildi - ekstra player effect istemiyoruz.
			SendChat((uint8)ChatType::SHOUT_CHAT, string_format("Vs �ld�n T�h! o �ntikam Bu Nick : [ %s ] Saldirsana.", pKiller->GetName().c_str()), "[Pvp Kill �d�l Kaybetsin]");
			g_pMain->SendHelpDescription(pKiller, string_format("[Pvp Kill �d�l Kazandin] = [ Para : %d ] | [ Cash : %d ] | [ �tem Name : %s x%d ] | [ Kill Count : %d ] �ld�rme G�revi Bitirdin �d�l �tem G�nderildin Tebrikler. ", g_pMain->Kill500GoldGift, g_pMain->Kill500CashGift, pTable.m_sName.c_str(), g_pMain->Kill500CountGift, pKiller->m_PvpKill500Count+500));
		}

		if (pKiller->isInPKZone() && (pKiller->isInParty() || isInParty()))
		{
			CUser* pPartyUser = nullptr;

			_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(pKiller->GetPartyID());
			Guard looock(g_pMain->m_PartyArray.m_lock);
			if (pParty != nullptr)
			{
				for (int i = 0; i < MAX_PARTY_USERS; i++)
				{
					pPartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
					if (pPartyUser != nullptr)
					{

						pPartyUser->m_PvpKillCount++;
						pPartyUser->m_PvpKill50Count++;
						pPartyUser->m_PvpKill500Count++;

						// JstKO Yeni Pvp Kill Ekran Notice Eklendi.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.
						// JstKO: eski yazili kill notice kapatildi; re_killcount.uif WIZ_KILLASSIST kullaniliyor.

						if (pPartyUser->m_PvpKillCount == 5)
						{
							// JstKO Yeni Kill�temGift �tem Name Notice Eklendi. 16.04.2025
							_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill�temGift);

							pPartyUser->GoldGain(g_pMain->KillGoldGift);
							pPartyUser->CashGain(g_pMain->KillCashGift);
							//pPartyUser->SendLoyaltyChange("Pvp Kill �d�l", g_pMain->KillNpGift);
							pPartyUser->GiveWerehouseItem(g_pMain->Kill�temGift, g_pMain->KillCountGift);
							pPartyUser->m_PvpKillCount = 0;
							// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
							// JstKO: kapatildi - ekstra player effect istemiyoruz.
							SendChat((uint8)ChatType::SHOUT_CHAT, string_format("Vs �ld�n T�h! o �ntikam Bu Nick : [ %s ] Saldirsana.", pKiller->GetName().c_str()), "[Pvp Kill �d�l Kaybetsin]");
							g_pMain->SendHelpDescription(pPartyUser, string_format("[Pvp Party Kill �d�l Kazandin] = [ Para : %d ] | [ Cash : %d ] | [ �tem Name : %s x%d ] | [ Kill Count : %d ] �ld�rme G�revi Bitirdin �d�l �tem G�nderildin Tebrikler. ", g_pMain->KillGoldGift, g_pMain->KillCashGift, pTable.m_sName.c_str(), g_pMain->KillCountGift, pPartyUser->m_PvpKillCount+5));
						}

						if (pPartyUser->m_PvpKill50Count == 50)
						{
							// JstKO Yeni Kill50�temGift �tem Name Notice Eklendi. 16.04.2025
							_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill50�temGift);

							pPartyUser->GoldGain(g_pMain->Kill50GoldGift);
							pPartyUser->CashGain(g_pMain->Kill50CashGift);
							//pPartyUser->SendLoyaltyChange("Pvp Kill �d�l", g_pMain->Kill50NpGift);
							pPartyUser->GiveWerehouseItem(g_pMain->Kill50�temGift, g_pMain->Kill50CountGift);
							pPartyUser->m_PvpKillCount = 0;
							pPartyUser->m_PvpKill50Count = 0;
							// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
							// JstKO: kapatildi - ekstra player effect istemiyoruz.
							SendChat((uint8)ChatType::SHOUT_CHAT, string_format("Vs �ld�n T�h! o �ntikam Bu Nick : [ %s ] Saldirsana.", pKiller->GetName().c_str()), "[Pvp Kill �d�l Kaybetsin]");
							g_pMain->SendHelpDescription(pPartyUser, string_format("[Pvp Party Kill �d�l Kazandin] = [ Para : %d ] | [ Cash : %d ] | [ �tem Name : %s x%d ] | [ Kill Count : %d ] �ld�rme G�revi Bitirdin �d�l �tem G�nderildin Tebrikler. ", g_pMain->Kill50GoldGift, g_pMain->Kill50CashGift, pTable.m_sName.c_str(), g_pMain->Kill50CountGift, pPartyUser->m_PvpKill50Count+50));
						}

						if (pPartyUser->m_PvpKill500Count == 500)
						{
							// JstKO Yeni Kill500�temGift �tem Name Notice Eklendi. 16.04.2025
							_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill500�temGift);

							pPartyUser->GoldGain(g_pMain->Kill500GoldGift);
							pPartyUser->CashGain(g_pMain->Kill500CashGift);
							//pPartyUser->SendLoyaltyChange("Pvp Kill �d�l", g_pMain->Kill500NpGift);
							pPartyUser->GiveWerehouseItem(g_pMain->Kill500�temGift, g_pMain->Kill500CountGift);
							pPartyUser->m_PvpKillCount = 0;
							pPartyUser->m_PvpKill50Count = 0;
							pPartyUser->m_PvpKill500Count = 0;
							// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
							// JstKO: kapatildi - ekstra player effect istemiyoruz.
							SendChat((uint8)ChatType::SHOUT_CHAT, string_format("Vs �ld�n T�h! o �ntikam Bu Nick : [ %s ] Saldirsana.", pKiller->GetName().c_str()), "[Pvp Kill �d�l Kaybetsin]");
							g_pMain->SendHelpDescription(pPartyUser, string_format("[Pvp Party Kill �d�l] = [ Para : %d ] | [ Cash : %d ] | [ �tem Name : %s x%d ] | [ Kill Count : %d ] �ld�rme G�revi Bitirdin �d�l �tem G�nderildin Tebrikler. ", g_pMain->Kill500GoldGift, g_pMain->Kill500CashGift, pTable.m_sName.c_str(), g_pMain->Kill500CountGift, pPartyUser->m_PvpKill500Count+500));
						}
					}
				}
			}
		}
		// JstKO Yeni Cz User Pvp Kill �d�l Notice Eklendi. The End. 16.04.2025
}

void CUser::OnDeathKilledBot(CBot* pKiller)
{
	if (!pKiller)
		return;

	CheckRespawnScroll();

	bool m_party_check = false;
	bool removerival = false;
	bool specialevent = pKiller->isInSpecialEventZone() && isInSpecialEventZone() && g_pMain->pSpecialEvent.opened;
	bool cindireallaw = false;
	// Bot normal CZ/PK alaninda user oldurdugunde Cindirella gecici level/class
	// akisi kesinlikle tetiklenmemeli. Cindirella user KDA sadece user-vs-user
	// event akisi icin kullaniliyor.

	uint16 bonusNP = 0;
	switch (pKiller->GetZoneID())
	{
	case ZONE_MORADON:
	case ZONE_MORADON2:
	case ZONE_MORADON3:
	case ZONE_MORADON4:
	case ZONE_MORADON5:
		SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
		break;
	case ZONE_ARDREAM:
	case ZONE_RONARK_LAND:
	case ZONE_RONARK_LAND_BASE:
	{
		m_party_check = true;
		bool bKilledByRival = false;

		// If the killer has us set as their rival, reward them & remove the rivalry.
		bKilledByRival = (!pKiller->hasRivalryExpired() && pKiller->GetRivalID() == GetID());

		if (!cindireallaw && !specialevent && !bKilledByRival)
			SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);

		if (bKilledByRival)
		{
			if (!cindireallaw && !specialevent)
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeRival, true);

			// Apply bonus NP for rival kills
			bonusNP += RIVALRY_NP_BONUS;

			removerival = true;
		}

		if (!hasFullAngerGauge())
			UpdateAngerGauge(++m_byAngerGauge);

		// If we don't have a rival, this player is now our rival for 3 minutes.
		if (!hasRival())
			SetRival(pKiller);
	}
	break;
	case ZONE_ELMORAD:
	case ZONE_KARUS:
		if (g_pMain->isWarOpen())
		{
			m_party_check = true;
			SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
		}
		break;
	case ZONE_BATTLE:
	case ZONE_BATTLE2:
	case ZONE_BATTLE3:
	case ZONE_BATTLE4:
	case ZONE_BATTLE5:
	case ZONE_BATTLE6:
		if (g_pMain->isWarOpen())
		{
			SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
			m_party_check = true;
		}
		break;
	case ZONE_BIFROST:
		m_party_check = true;
		SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
		break;
	case ZONE_SNOW_BATTLE:
		if (g_pMain->m_byBattleOpen == SNOW_BATTLE)
		{
			GetNation() == (uint8)Nation::KARUS ? g_pMain->m_sKarusDead++ : g_pMain->m_sElmoradDead++;;
			SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
		}
		break;
	}

	if (pKiller->GetMap())
	{
		if (pKiller->GetMap()->m_bGiveLoyalty)
		{
			if (pKiller->isInParty())
				pKiller->LoyaltyDivide(GetID(), bonusNP);
			else
				pKiller->LoyaltyChange(GetID(), bonusNP);

			// Mixed party support: bot killer + real users in same party should also
			// grant NP to nearby real users.
			if (pKiller->isInParty())
			{
				_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(pKiller->GetPartyID());
				if (pParty != nullptr)
				{
					for (int i = 0; i < MAX_PARTY_USERS; i++)
					{
						CUser* pPartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
						if (pPartyUser == nullptr
							|| !pPartyUser->isInGame()
							|| pPartyUser->isDead()
							|| !pKiller->isInRangeSlow(pPartyUser, RANGE_50M))
							continue;

						pPartyUser->LoyaltyDivide("BotKill", this, bonusNP);
						break; // LoyaltyDivide handles distribution to user party members.
					}
				}
			}
		}
	}

	if (removerival && pKiller->isInGame() && pKiller->hasRival())
		pKiller->RemoveRival();

	m_sWhoKilledMe = pKiller->GetID();
}

int64 CUser::OnDeathLostExpCalc(int64 maxexp)
{
	float nExpLostFloat = 0.0f;
	int64 nExpLost = maxexp / 20;

	if (GetPremiumPropertyExp(PremiumPropertyOpCodes::PremiumExpRestorePercent) > 0)
		nExpLostFloat = maxexp * (GetPremiumPropertyExp(PremiumPropertyOpCodes::PremiumExpRestorePercent)) / 100;
	else
		nExpLostFloat = 0.0f;

	if (nExpLostFloat) nExpLost = static_cast<int64>(nExpLostFloat);

	return nExpLost;
}

void CUser::OnDeathKilledNpc(CNpc* pKiller)
{
	if (pKiller == nullptr) return;

	int64 nExpLost = 0;
	if (GetMap() && GetMap()->m_bExpLost != 0)
	{
		nExpLost = OnDeathLostExpCalc(m_iMaxExp);
		if (nExpLost > 0
			&& pKiller->GetType() != NPC_ROLLINGSTONE
			&& pKiller->GetProtoID() != SAW_BLADE_SSID
			&& pKiller->GetProtoID() != GUARD_SUMMON)
		{
			ExpChange("killed npc lost exp", -nExpLost, false);
			m_iLostExp = nExpLost;
			m_bCity = 1;

			CheckRespawnScroll();
		}
	}

	Packet result(WIZ_ATTACK);
	result << uint8_t(LONG_ATTACK) << uint8_t(AttackResult::ATTACK_TARGET_DEAD) << pKiller->GetID() << GetID();
	Send(&result);

	switch (pKiller->GetZoneID())
	{
	case ZONE_ARDREAM:
	case ZONE_RONARK_LAND:
	case ZONE_RONARK_LAND_BASE:
		if (pKiller->GetType() == NPC_GUARD_TOWER1 || pKiller->GetType() == NPC_GUARD_TOWER2)
			SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
		break;
	case ZONE_KROWAZ_DOMINION:
		if (pKiller->GetType() == NPC_ROLLINGSTONE || pKiller->GetProtoID() == SAW_BLADE_SSID)
			SendLoyaltyChange("Krowaz Dominion", -100);
		break;
	case ZONE_BATTLE6:
		if (pKiller->GetType() == NPC_OBJECT_WOOD)
		{
			CUser* pWoodOwner = g_pMain->GetUserPtr(pKiller->GetWoodUserID());
			if (pWoodOwner != nullptr)
			{
				if (pWoodOwner->GetMap()->m_bGiveLoyalty != 0)
				{
					if (pWoodOwner->isInParty())
						pWoodOwner->LoyaltyDivide("Oreads war wood", this, 0);
					else
						pWoodOwner->LoyaltyChange(this, 0);
				}
			}
		}
		break;
	}
}

#pragma region CUser::InitOnDeath(Unit *pKiller)
/**
* @brief	Inits the user after ondeath is proceeded.
*
*/
void CUser::InitOnDeath(Unit* pKiller)
{
	Unit::OnDeath(pKiller);

	// Player is dead stop other process.
	ResetWindows();
	InitType3();
	InitType4();

	Type9Duration(true);

	if (isTransformed() && !isNPCTransformation())
	{
		MagicInstance instance;
		instance.pSkillCaster = this;
		instance.nSkillID = m_sTransformSkillID; // TS bas�l�yken �l�nce silinmemesi fixlendi 
		instance.Type6Cancel(true);
	}
}
#pragma endregion

#pragma region CUser::CheckRespawnScroll()
/**
* @brief	Checks if any respawn scroll exists on the user. If so, sends the
*			emulating packets to the user.
*
*/
void CUser::CheckRespawnScroll()
{
	// Search for the existance of all items in the player's inventory storage and onwards (includes magic bags)
	for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
	{
		_ITEM_DATA* pItem = GetItem(i);
		if (pItem == nullptr)
			continue;

		// This implementation fixes a bug where it ignored the possibility for multiple stacks.
		if ((pItem->nNum == 800036000 && pItem->sCount >= 1)
			|| (pItem->nNum == 800039000 && pItem->sCount >= 1)
			|| (pItem->nNum == 910022000 && pItem->sCount >= 1)
			|| (pItem->nNum == 900699000 && pItem->sCount >= 1)
			|| (pItem->nNum == 810036000 && pItem->sCount >= 1)
			|| (pItem->nNum == 900136000 && pItem->sCount >= 1)
			|| (pItem->nNum == 910948000 && pItem->sCount >= 1))
		{
			Packet result2(WIZ_DEAD);
			result2 << GetID() << uint32(18483) << uint64(0);
			Send(&result2);
			break;
		}
	}
}
#pragma endregion

#pragma region CUser::SpChange(int amount)
void CUser::SpChange(int amount)
{
	Packet result(WIZ_KURIAN_SP_CHANGE);

	if (isBeginnerKurianPortu())
		m_MaxSp = 100;
	else if (isNoviceKurianPortu())
		m_MaxSp = 150;
	else if (isMasteredKurianPortu())
	{
		if (CheckSkillPoint(PRO_SKILL4, 0, 2))
			m_MaxSp = 200;
		else if (CheckSkillPoint(PRO_SKILL4, 3, 23))
			m_MaxSp = 250;
		else
			m_MaxSp = 200;
	}
	else
		m_MaxSp = 200;

	m_sSp += amount;

	if (m_sSp < 0)
		m_sSp = 0;
	else if (m_sSp >= m_MaxSp)
		m_sSp = m_MaxSp;

	result << uint8(1) << uint8(1);
	result << uint8(m_MaxSp) << uint8(m_sSp);
	Send(&result);
}
#pragma endregion

void CUser::HealMagic()
{
	C3DMap* pMap = GetMap();

	if (pMap == nullptr)
		return;

	int16 rx = GetRegionX(), rz = GetRegionZ();

	foreach_region(x, z)
		HealAreaCheck(pMap, rx + x, rz + z);
}

void CUser::HealAreaCheck(C3DMap* pMap, int rx, int rz)
{
	static const float fRadius = 10.0f; // 30m

	if (pMap == nullptr)
		return;

	CRegion* pRegion = pMap->GetRegion(rx, rz);
	if (pRegion == nullptr)
		return;

	pRegion->m_lockNpcArray.lock();
	if (pRegion->m_RegionNpcArray.size() <= 0)
	{
		pRegion->m_lockNpcArray.unlock();
		return;
	}

	ZoneNpcArray cm_RegionNpcArray = pRegion->m_RegionNpcArray;
	pRegion->m_lockNpcArray.unlock();

	foreach(itr, cm_RegionNpcArray)
	{
		CNpc* pNpc = g_pMain->GetNpcPtr(*itr, GetZoneID());

		if (pNpc == nullptr
			|| pNpc->isDead()
			|| !pNpc->isHostileTo(this)
			|| (GetEventRoom() >= 0 && GetEventRoom() != pNpc->GetEventRoom())
			|| pNpc->GetProtoID() == SAW_BLADE_SSID)
			continue;

		if (isInRangeSlow(pNpc, fRadius))
			pNpc->ChangeTarget(1004, this);
	}
}

void CUser::SendMannerChange(int32 iMannerPoints)
{
	//Make sure we don't have too many or too little manner points!
	if (m_iMannerPoint + iMannerPoints > LOYALTY_MAX)
		m_iMannerPoint = LOYALTY_MAX;
	else if (m_iMannerPoint + iMannerPoints < 0)
		m_iMannerPoint = 0;
	else
		m_iMannerPoint += iMannerPoints;

	Packet pkt(WIZ_LOYALTY_CHANGE, uint8(LOYALTY_MANNER_POINTS));
	pkt << m_iMannerPoint;
	Send(&pkt);
}

void CUser::SetMaxSp()
{
	if (isBeginnerKurianPortu())
		m_MaxSp = 100;
	else if (isNoviceKurianPortu())
		m_MaxSp = 150;
	else if (isMasteredKurianPortu())
	{
		if (CheckSkillPoint(PRO_SKILL4, 0, 2))
			m_MaxSp = 200;
		else if (CheckSkillPoint(PRO_SKILL4, 3, 23))
			m_MaxSp = 250;
		else
			m_MaxSp = 200;
	}
	else
		m_MaxSp = 200;

	if (m_MaxSp < m_sSp)
	{
		m_sSp = m_MaxSp;
		SpChange(m_sSp);
	}
}

void CUser::KA_KillUpdate()
{
	if (!isInPKZone())
		return;

	// JstKO Kill Count UI:
	// Client tarafinda re_killcount.uif, killnameall.dxt ve killnamebackgall.dxt dosyalari
	// WIZ_KILLASSIST paketini dinleyerek 1 Kill, 2 Kill, Legendary vb. gorseli basar.
	// Eski ShowEffect/PlayerEffect sistemi kullanilmaz. Sayac sadece olum/zone resetinde sifirlanir.
	uint32 nRealSerialKillCount = ++pAssistKill.serikillcount;
	uint8 nKillSoundStage = (nRealSerialKillCount >= 12 ? uint8(12) : uint8(nRealSerialKillCount));

	// JstKO: re_killcount.uif kill count gorselini tetikler.
	// Client, SND klasorundeki narration_kill_f3_01.ogg - narration_kill_f3_12.ogg
	// seslerini bu son stage degerine gore oynatir. 12 ve uzeri Legendary/final ses olarak kalir.
	Packet newpkt(WIZ_KILLASSIST, uint8(kaopcode::kill));
	newpkt << uint8(1) << uint32(1) << GetName() << uint8(1) << nKillSoundStage;
	Send(&newpkt);

	pAssistKill.totalkillcount++;
	pAssistKill.lastkilltime = UNIXTIME;

	// Her 10 killde partiye/oyuncuya toplam kill bilgisini de gonder.
	// Bu kisim da ayni re_killcount.uif kill-count arayuzunu kullanir.
	if (pAssistKill.totalkillcount % 10 != 0)
		return;

	Packet newpkt2(WIZ_KILLASSIST, uint8(kaopcode::kill));
	newpkt2 << uint8(3) << uint32(1) << GetName() << GetNation() << pAssistKill.totalkillcount;
	if (isInParty())
		g_pMain->Send_PartyMember(GetPartyID(), &newpkt2);
	else
		Send(&newpkt2);
}


void CUser::KA_ResetCheck(bool zonechange /*= false*/)
{

	_KILLASSISTINFO& pin = pAssistKill;
	bool killreset = false, assistreset = false;
	if (zonechange && pin.totalkillcount > 0) killreset = true;
	if (zonechange && pin.totalassistcount > 0) assistreset = true;

	if (pin.totalkillcount > 0 && pin.lastkilltime && UNIXTIME - pin.lastkilltime > 15 * MINUTE) killreset = true;
	if (pin.totalassistcount > 0 && pin.lastassisttime && UNIXTIME - pin.lastassisttime > 15 * MINUTE) assistreset = true;

	Packet newpkt(WIZ_KILLASSIST);
	if (killreset) { pin.serikillcount = 0; pin.totalkillcount = 0; pin.lastkilltime = 0; newpkt << uint8(kaopcode::kill) << uint8(2); Send(&newpkt); }
	if (assistreset) { pin.totalassistcount = 0; pin.lastassisttime = 0; newpkt.clear(); newpkt << uint8(kaopcode::assist) << uint8(2); Send(&newpkt); }
}

void CUser::KA_AssistDebufUpdate(CUser* pkiller)
{
	if (!pkiller || !pkiller->isInParty()) return;

	std::vector<CUser*> mpriestlist;

	m_buffLock.lock();
	foreach(sk, m_buffMap) { if (sk->second.isDebuff() && sk->second.pownskill) mpriestlist.push_back(sk->second.pownskill); }
	m_buffLock.unlock();

	foreach(itr, mpriestlist)
	{
		if (!(*itr) || !(*itr)->isInParty() || !(*itr)->isPriest() || (*itr)->GetPartyID() != pkiller->GetPartyID()) continue;
		(*itr)->KA_AssistScreenSend(1);
	}
}

void CUser::KA_AssistScreenSend(uint8 type)
{
	if (UNIXTIME - pAssistKill.lastassisttime > 29) pAssistKill.totalassistcount = 0;
	pAssistKill.lastassisttime = UNIXTIME;
	Packet newpkt(WIZ_KILLASSIST, uint8(kaopcode::assist));
	newpkt << uint8(1) << uint16(1) << uint8(1) << (uint16)++pAssistKill.totalassistcount << type;
	Send(&newpkt);
}
