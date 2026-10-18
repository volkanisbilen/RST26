#include "StdAfx.h"
#include "DBAgent.h"
#include "md5.h"
#include "MagicInstance.h"

void CBot::HpChange(int amount, Unit* pAttacker /*= nullptr*/, bool isDOT /*= false*/)
{
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

	// If we're taking damage...
	if (amount < 0)
	{
		/*if (!isDOT)
			RemoveStealth();*/

		bool NotUseZone = (GetZoneID() == ZONE_CHAOS_DUNGEON || GetZoneID() == ZONE_KNIGHT_ROYALE);

		// Handle the mirroring of damage.
		if (m_bMirrorDamage && !NotUseZone)
		{
			if (m_bMirrorDamageType)
			{
				CUser* pUserAttacker = g_pMain->GetUserPtr(pAttacker->GetID());;
				if (pUserAttacker != nullptr)
				{
					mirrorDamage = (m_byMirrorAmount * amount) / 100;
					amount -= mirrorDamage;
					pUserAttacker->HpChange(mirrorDamage);
				}
			}
		}

		// Handle mastery passives
		if (isMastered() && !NotUseZone)
		{
			// Matchless: [Passive]Decreases all damages received by 15%
			if (CheckSkillPoint(SkillPointMaster, 10, g_pMain->m_byMaxLevel))
				amount = (85 * amount) / 100;
			// Absoluteness: [Passive]Decrease 10 % demage of all attacks
			else if (CheckSkillPoint(SkillPointMaster, 5, 9))
				amount = (90 * amount) / 100;
		}

		if (m_bManaAbsorb > 0 && !NotUseZone)
		{
			int toBeAbsorbed = 0, absortedmana = 0;
			toBeAbsorbed = (originalAmount * m_bManaAbsorb) / 100;
			amount -= toBeAbsorbed;

			if (amount > 0)
				amount = 0;

			absortedmana = toBeAbsorbed;
			MSpChange(absortedmana);
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
			AbsorbedAmmount += Receive;

			if (m_sHp > 0)
				m_sHp -= int16(Receive);

			if (AbsorbedAmmount <= ABSORBED_TOTAL)
				CMagicProcess::RemoveType4Buff(BUFF_TYPE_DEVIL_TRANSFORM, this);
		}
	}

	if (pAttacker != nullptr
		&& pAttacker->isPlayer()
		&& m_sHp > 0 && amount < 0
		&& !NotUseZone2)
	{
		if (isWarrior() && isMastered())
		{
			if (CheckSkillPoint(PRO_SKILL4, 10, 23))
			{
				int16 NewHP = oldHP - m_sHp;
				m_sHp += (15 * NewHP) / 100;
			}
		}
		else if ((isRogue() || isMage() || isPriest()) && isMastered())
		{
			if (CheckSkillPoint(PRO_SKILL4, 5, 9))
			{
				int16 NewHP = oldHP - m_sHp;
				m_sHp += (10 * NewHP) / 100;
			}
			else if (CheckSkillPoint(PRO_SKILL4, 10, 23))
			{
				int16 NewHP = oldHP - m_sHp;
				m_sHp += (15 * NewHP) / 100;
			}
		}
	}

	if (GetHealth() > 0
		&& isMastered()
		&& !isMage() && !NotUseZone2)
	{
		const uint16 hp30Percent = (30 * GetMaxHealth()) / 100;
		if ((oldHP >= hp30Percent && m_sHp < hp30Percent)
			|| (m_sHp > hp30Percent))
		{
			SetBotAbility();

			if (m_sHp < hp30Percent)
				ShowEffect(106800); // skill ID for "Boldness", shown when a player takes damage.
		}
	}

	// Ensure we send the original damage (prior to passives) amount to the attacker 
	// as it appears to behave that way officially.
	if (pAttacker != nullptr
		&& pAttacker->isPlayer())
		TO_USER(pAttacker)->SendTargetHP(0, GetID(), originalAmount);

	if (m_sHp == 0)
		OnDeath(pAttacker);
}

void CBot::MSpChange(int amount)
{
	int16 oldMP = m_sMp;

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
}

void CBot::SpChange(int amount)
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
}

void CBot::SetMaxHp(int iFlag)
{
	_CLASS_COEFFICIENT* p_TableCoefficient = nullptr;
	p_TableCoefficient = g_pMain->m_CoefficientArray.GetData(GetClass());

	if (!p_TableCoefficient)
		return;

	int temp_sta = GetStatTotal(STAT_STA);

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

		if (iFlag == 1)
		{
			m_MaxHp = MAX_PLAYER_HP;
			HpChange(m_MaxHp);
		}
		else if (iFlag == 2)
			m_MaxHp = 100;
	}
	// Awakening Max Healt %20 Arttýrma.
	if (isMasteredKurianPortu())
	{
		if (GetZoneID() != ZONE_KNIGHT_ROYALE
			&& GetZoneID() != ZONE_CHAOS_DUNGEON)
		{
			if (CheckSkillPoint(PRO_SKILL4, 2, 23))
				m_MaxHp += m_MaxHp * 20 / 100;
		}
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

void CBot::SetMaxMp()
{
	_CLASS_COEFFICIENT* p_TableCoefficient = nullptr;
	p_TableCoefficient = g_pMain->m_CoefficientArray.GetData(GetClass());
	if (!p_TableCoefficient) return;

	int temp_intel = 0, temp_sta = 0;
	temp_intel = GetStatTotal(STAT_INT) + 30;
	temp_sta = GetStatTotal(STAT_STA);

	if (p_TableCoefficient->MP != 0)
	{
		m_MaxMp = (short)((p_TableCoefficient->MP * GetLevel() * GetLevel() * temp_intel)
			+ (0.1f * GetLevel() * 2 * temp_intel) + (temp_intel / 5) + m_sMaxMPAmount + m_sItemMaxMp + 20);
	}
	else if (p_TableCoefficient->SP != 0)
	{
		m_MaxMp = (short)((p_TableCoefficient->SP * GetLevel() * GetLevel() * temp_sta)
			+ (0.1f * GetLevel() * temp_sta) + (temp_sta / 5) + m_sMaxMPAmount + m_sItemMaxMp);
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

void CBot::SetMaxSp()
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

void CBot::HpMpChange()
{
	m_fHPChangeTime = getMSTime();

	if (isDead())
		return;

	const uint16 hp30Percent = (90 * GetMaxHealth()) / 100;
	if (uint16(GetHealth()) > hp30Percent)
		return;

	int32 sSkillID = 0;
	if (isRogue())
		sSkillID = 490014;
	else if (isMage())
		sSkillID = 490014;
	else if (isPortuKurian())
		sSkillID = 490014;
	else if (isPriest())
		sSkillID = 112545;
	else
		sSkillID = 490014;

	if (sSkillID <= 0)
		return;

	if (isPriest())
	{
		if (GetNation() == ELMORAD)
			sSkillID += 100000;
	}

	if (sSkillID == 112545
		|| sSkillID == 212545)
		MagicPacket(MAGIC_CASTING, sSkillID, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
	MagicPacket(MAGIC_EFFECTING, sSkillID, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
	m_sSpeed = 0;
	m_sSkillCoolDown = (uint32)UNIXTIME + 2;
}
void CUser::CyberACS_BotPriestSystem()
{
	uint32 s_SwiftSkills[] = { 208002 , 108010 , 207010 , 208010 , 500265 , 107725 , 108725 , 207725 , 208725 , 490230 , 490336, 101002, 102002, 105002, 106002, 107002, 108002, 201002 , 202002, 205002, 206002, 207002, 208002, 490223, 490334, 500316 };

	uint32 s_HpSkills[] = { 111606, 491009, 500102, 491011 , 500103 , 500304 , 500315 , 500054 , 112606 , 211606 , 212606 , 111615 , 112615 , 211615 , 212615 , 111624 , 112624 , 211624 , 212624 , 111633 , 112633 , 211633 , 212633 , 111642 , 112642 , 211642 , 212642 , 111654 , 112654 , 211654 , 212654 , 500054 , 500354 , 111655 , 112655 , 211655 , 212655 , 111656 , 112656 , 211656 , 212656 , 111657 , 112657 , 211657 , 212657 , 112670 , 212670 , 112672 , 212672 , 112675 , 212675,500053 };

	uint32 s_AcSkills[] = { 491006, 491007, 500029, 500030, 500055, 500056, 111603, 112603, 211603, 212603, 111612, 112612, 211612, 212612, 111621, 112621, 211621, 212621, 111630, 112630, 211630, 212630, 111639, 112639, 211639, 212639,	111651 , 112651 , 211651 , 212651 , 111660 , 112660 , 211660 , 212660 , 112673 , 212673 , 112674 , 212674 };

	uint32 s_ResisSkills[] = { 111609, 112609, 211609, 212609, 111627, 112627, 211627, 212627, 111636, 112636, 211636, 212636, 111645, 112645, 211645, 212645 };

	uint32 s_ResSkills[] = { 111503, 112503, 211503, 212503, 111512, 112512, 211512, 212512, 111521, 112521, 211521, 212521, 111530, 112530, 211530, 212530, 111539, 112539, 211539 , 212539 , 111548 , 112548 , 211548 , 212548 , 112570 , 212570 , 112575 , 212575 , 112580 , 212580 };


	bool Partybuff = false;

	bool buff = false;
	bool buffacc = false;

	CBot* pPriest = nullptr;
	pPriest = g_pMain->m_MapBotList.GetData(m_bUserPriestBotID);

	/*HealYuzde
	Buffbool
	AccBool
	HealBool */

	if (pPriest != nullptr && m_bUserPriestBotID > 0)
	{
		_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(GetPartyID());

		if (pParty != nullptr)//toplu 10k eðer 2 kiþinin hp si %80 altýna düþerse toplu 10k at
		{
			uint8 sayi;
			sayi = 0;
			for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
			{
				auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (PartyUser != nullptr && PartyUser->m_sHp < (PartyUser->m_MaxHp * HealYuzde) / 100) // %80 in altýndaysa 
				{
					sayi++; //%80 altýnda kiþi sayýsý
				}
			}
			if (sayi >= 2)
			{
				for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
				{
					auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
					if (PartyUser != nullptr)
					{
						MagicInstance instance;
						instance.bOpcode = 1;
						instance.nSkillID = 212560;

						//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
						instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


						instance.sCasterID = pPriest->GetID();
						instance.sTargetID = PartyUser->GetSocketID();
						instance.sData[0] = 0;
						instance.sData[1] = 0;
						instance.sData[2] = 0;
						instance.sData[3] = 0;
						instance.sData[4] = 0;
						instance.sData[5] = 0;
						instance.sData[6] = 0;


						instance.sSkillCasterZoneID = PartyUser->GetZoneID();
						instance.bIsRecastingSavedMagic = false;
						instance.Run(); //priest animasyonu


						//heal kýsmý
						MagicInstance instance2;
						instance2.bOpcode = 3;
						instance2.nSkillID = 212560;

						//instance2.pSkill = g_pMain->m_MagictableArray.GetData(instance2.nSkillID);
						instance2.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


						instance2.sCasterID = pPriest->GetID();
						instance2.sTargetID = PartyUser->GetSocketID();
						instance2.sData[0] = 0;
						instance2.sData[1] = 0;
						instance2.sData[2] = 0;
						instance2.sData[3] = 0;
						instance2.sData[4] = 0;
						instance2.sData[5] = 0;
						instance2.sData[6] = 0;


						instance2.sSkillCasterZoneID = PartyUser->GetZoneID();
						instance2.bIsRecastingSavedMagic = false;
						instance2.Run(); //priest animasyonu


					}
				}
			}
		}

		if (pParty != nullptr)//tekli 1920 
		{
			for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
			{
				auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (PartyUser != nullptr && PartyUser->m_sHp < (PartyUser->m_MaxHp * HealYuzde) / 100) // %80 in altýndaysa 
				{
					MagicInstance instance;
					instance.bOpcode = 1;
					instance.nSkillID = 212545;//1920 heal

					//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
					instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


					instance.sCasterID = pPriest->GetID();
					instance.sTargetID = PartyUser->GetSocketID();
					instance.sData[0] = 0;
					instance.sData[1] = 0;
					instance.sData[2] = 0;
					instance.sData[3] = 0;
					instance.sData[4] = 0;
					instance.sData[5] = 0;
					instance.sData[6] = 0;


					instance.sSkillCasterZoneID = PartyUser->GetZoneID();
					instance.bIsRecastingSavedMagic = false;
					instance.Run();//priest animasyonu


					MagicInstance instance2;
					//heal basma kýsmý
					instance2.bOpcode = 3;
					instance2.nSkillID = 212545;//1920 heal

					//instance2.pSkill = g_pMain->m_MagictableArray.GetData(instance2.nSkillID);
					instance2.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


					instance2.sCasterID = pPriest->GetID();
					instance2.sTargetID = PartyUser->GetSocketID();
					instance2.sData[0] = 0;
					instance2.sData[1] = 0;
					instance2.sData[2] = 0;
					instance2.sData[3] = 0;
					instance2.sData[4] = 0;
					instance2.sData[5] = 0;
					instance2.sData[6] = 0;


					instance2.sSkillCasterZoneID = PartyUser->GetZoneID();
					instance2.bIsRecastingSavedMagic = false;
					instance2.Run();//priest animasyonu
					break;
				}
			}
		}
		else if (m_sHp < (m_MaxHp * HealYuzde) / 100) // %80 in altýndaysa 
		{
			MagicInstance instance;
			instance.bOpcode = 1;
			instance.nSkillID = 212545;//1920 heal

			//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
			instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


			instance.sCasterID = pPriest->GetID();
			instance.sTargetID = GetSocketID();
			instance.sData[0] = 0;
			instance.sData[1] = 0;
			instance.sData[2] = 0;
			instance.sData[3] = 0;
			instance.sData[4] = 0;
			instance.sData[5] = 0;
			instance.sData[6] = 0;


			instance.sSkillCasterZoneID = GetZoneID();
			instance.bIsRecastingSavedMagic = false;
			instance.Run();//priest animasyonus

			MagicInstance instance2;
			//heal basma kýsmý
			instance2.bOpcode = 3;
			instance2.nSkillID = 212545;//1920 heal

			//instance2.pSkill = g_pMain->m_MagictableArray.GetData(instance2.nSkillID);
			instance2.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


			instance2.sCasterID = pPriest->GetID();
			instance2.sTargetID = GetSocketID();
			instance2.sData[0] = 0;
			instance2.sData[1] = 0;
			instance2.sData[2] = 0;
			instance2.sData[3] = 0;
			instance2.sData[4] = 0;
			instance2.sData[5] = 0;
			instance2.sData[6] = 0;


			instance2.sSkillCasterZoneID = GetZoneID();
			instance2.bIsRecastingSavedMagic = false;
			instance2.Run();//priest animasyonus

		}


		foreach(itr, m_buffMap)
		{
			for (int i = 0; i < 52; i++)
				if (itr->second.m_nSkillID == s_HpSkills[i])
					buff = true;

			for (int i = 0; i < 38; i++)
				if (itr->second.m_nSkillID == s_AcSkills[i])
					buffacc = true;
		}



		if (pParty != nullptr)//buff
		{
			uint8 PartyMember, PartyUsedSkill;
			PartyMember = 0;
			PartyUsedSkill = 0;

			for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
			{
				auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (PartyUser != nullptr)
				{
					PartyMember++;
				}
			}

			for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
			{
				auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (PartyUser != nullptr)
				{

					foreach(itr, PartyUser->m_buffMap)
					{
						if (itr->second.m_nSkillID == 212675)
						{
							PartyUsedSkill++;
						}
					}
				}
			}

			if (PartyMember == PartyUsedSkill)
				Partybuff = true;

			for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
			{
				auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (PartyUser != nullptr && !Partybuff && Buffbool == 1 && buff == false)
				{
					MagicInstance instance;
					instance.bOpcode = 1;
					instance.nSkillID = 212675;//buff

					//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
					instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


					instance.sCasterID = pPriest->GetID();
					instance.sTargetID = PartyUser->GetSocketID();
					instance.sData[0] = 0;
					instance.sData[1] = 0;
					instance.sData[2] = 0;
					instance.sData[3] = 0;
					instance.sData[4] = 0;
					instance.sData[5] = 0;
					instance.sData[6] = 0;


					instance.sSkillCasterZoneID = PartyUser->GetZoneID();
					instance.bIsRecastingSavedMagic = false;
					instance.Run();//priest animasyonu

					//heal basma kýsmý
					instance.bOpcode = 3;
					instance.nSkillID = 212675;//buff

					//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
					instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


					instance.sCasterID = pPriest->GetID();
					instance.sTargetID = PartyUser->GetSocketID();
					instance.sData[0] = 0;
					instance.sData[1] = 0;
					instance.sData[2] = 0;
					instance.sData[3] = 0;
					instance.sData[4] = 0;
					instance.sData[5] = 0;
					instance.sData[6] = 0;


					instance.sSkillCasterZoneID = PartyUser->GetZoneID();
					instance.bIsRecastingSavedMagic = false;
					instance.Run();//priest animasyonu
				}
			}
		}
		else if (pParty == nullptr && !buff && Buffbool == 1 && buff == false)//buff
		{


			MagicInstance instance;
			instance.bOpcode = 1;
			instance.nSkillID = 212675;//buf

			//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
			instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


			instance.sCasterID = pPriest->GetID();
			instance.sTargetID = GetSocketID();
			instance.sData[0] = 0;
			instance.sData[1] = 0;
			instance.sData[2] = 0;
			instance.sData[3] = 0;
			instance.sData[4] = 0;
			instance.sData[5] = 0;
			instance.sData[6] = 0;


			instance.sSkillCasterZoneID = GetZoneID();
			instance.bIsRecastingSavedMagic = false;
			instance.Run();//priest animasyonus


			//iþlem
			instance.bOpcode = 3;
			instance.nSkillID = 212675;//buf

			//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
			instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


			instance.sCasterID = pPriest->GetID();
			instance.sTargetID = GetSocketID();
			instance.sData[0] = 0;
			instance.sData[1] = 0;
			instance.sData[2] = 0;
			instance.sData[3] = 0;
			instance.sData[4] = 0;
			instance.sData[5] = 0;
			instance.sData[6] = 0;


			instance.sSkillCasterZoneID = GetZoneID();
			instance.bIsRecastingSavedMagic = false;
			instance.Run();//priest animasyonus
		}
		if (pParty != nullptr)
		{
			bool Partybuffacc = false;
			uint8 PartyMember, PartyUsedSkill;
			PartyMember = 0;
			PartyUsedSkill = 0;

			for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
			{
				auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (PartyUser != nullptr)
				{
					PartyMember++;
				}
			}

			for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
			{
				auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (PartyUser != nullptr)
				{

					foreach(itr, PartyUser->m_buffMap)
					{
						if (itr->second.m_nSkillID == 212674)
						{
							PartyUsedSkill++;
						}
					}
				}
			}

			if (PartyMember == PartyUsedSkill)
				Partybuffacc = true;

			for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
			{
				auto* PartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (PartyUser != nullptr && !Partybuffacc && AccBool == 1 && buffacc == false)
				{
					MagicInstance instance;
					instance.bOpcode = 1;
					instance.nSkillID = 212674;//buff

					//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
					instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


					instance.sCasterID = pPriest->GetID();
					instance.sTargetID = PartyUser->GetSocketID();
					instance.sData[0] = 0;
					instance.sData[1] = 0;
					instance.sData[2] = 0;
					instance.sData[3] = 0;
					instance.sData[4] = 0;
					instance.sData[5] = 0;
					instance.sData[6] = 0;


					instance.sSkillCasterZoneID = PartyUser->GetZoneID();
					instance.bIsRecastingSavedMagic = false;
					instance.Run();//priest animasyonu

					//heal basma kýsmý
					instance.bOpcode = 3;
					instance.nSkillID = 212674;//buff

					//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
					instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


					instance.sCasterID = pPriest->GetID();
					instance.sTargetID = PartyUser->GetSocketID();
					instance.sData[0] = 0;
					instance.sData[1] = 0;
					instance.sData[2] = 0;
					instance.sData[3] = 0;
					instance.sData[4] = 0;
					instance.sData[5] = 0;
					instance.sData[6] = 0;


					instance.sSkillCasterZoneID = PartyUser->GetZoneID();
					instance.bIsRecastingSavedMagic = false;
					instance.Run();//priest animasyonu
				}
			}
		}
		else if (pParty == nullptr && !buffacc && AccBool == 1 && buffacc == false)//buff  acccc
		{


			MagicInstance instance;
			instance.bOpcode = 1;
			instance.nSkillID = 212674;//buf

			//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
			instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


			instance.sCasterID = pPriest->GetID();
			instance.sTargetID = GetSocketID();
			instance.sData[0] = 0;
			instance.sData[1] = 0;
			instance.sData[2] = 0;
			instance.sData[3] = 0;
			instance.sData[4] = 0;
			instance.sData[5] = 0;
			instance.sData[6] = 0;


			instance.sSkillCasterZoneID = GetZoneID();
			instance.bIsRecastingSavedMagic = false;
			foreach(itr, m_buffMap)
			{
				if (itr->second.m_nSkillID == 212674)
					return;
			}
			instance.Run();//priest animasyonus


			//iþlem
			instance.bOpcode = 3;
			instance.nSkillID = 212674;//buf

			//instance.pSkill = g_pMain->m_MagictableArray.GetData(instance.nSkillID);
			instance.pSkill = g_pMain->GetMagicPtr(instance.nSkillID);


			instance.sCasterID = pPriest->GetID();
			instance.sTargetID = GetSocketID();
			instance.sData[0] = 0;
			instance.sData[1] = 0;
			instance.sData[2] = 0;
			instance.sData[3] = 0;
			instance.sData[4] = 0;
			instance.sData[5] = 0;
			instance.sData[6] = 0;


			instance.sSkillCasterZoneID = GetZoneID();
			instance.bIsRecastingSavedMagic = false;
			instance.Run();//priest animasyonus
		}
	}
}

