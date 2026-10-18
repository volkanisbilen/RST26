#include "StdAfx.h"

float CBot::SetCoefficient()
{
	_CLASS_COEFFICIENT* pCoefficient = g_pMain->m_CoefficientArray.GetData(GetClass());
	if (pCoefficient == nullptr)
		return 0.0f;

	if (!isWeaponsDisabled())
	{
		_ITEM_TABLE* pRightHand = GetItemPrototype(GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_RIGHTHAND : RIGHTHAND);
		if (pRightHand != nullptr)
		{
			switch (pRightHand->m_bKind)
			{
			case WEAPON_KIND_DAGGER:
				return pCoefficient->ShortSword;
				break;
			case WEAPON_KIND_1H_SWORD:
			case WEAPON_KIND_2H_SWORD:
				return pCoefficient->Sword;
				break;
			case WEAPON_KIND_1H_AXE:
			case WEAPON_KIND_2H_AXE:
				return pCoefficient->Axe;
				break;
			case WEAPON_KIND_1H_CLUP:
			case WEAPON_KIND_2H_CLUP:
				return pCoefficient->Club;
				break;
			case WEAPON_KIND_1H_SPEAR:
			case WEAPON_KIND_2H_SPEAR:
				return pCoefficient->Spear;
				break;
			case WEAPON_KIND_BOW:
			case WEAPON_KIND_CROSSBOW:
				return pCoefficient->Bow;
				break;
			case WEAPON_KIND_STAFF:
				return pCoefficient->Staff;
				break;
			case WEAPON_KIND_JAMADHAR:
				return pCoefficient->Jamadar;
				break;
			case WEAPON_KIND_MACE:
				return pCoefficient->Pole;
				break;
			}
		}
		else
		{
			_ITEM_TABLE* pLeftHand = GetItemPrototype(GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_LEFTHAND : LEFTHAND);
			if (pLeftHand != nullptr)
			{
				switch (pLeftHand->m_bKind)
				{
				case WEAPON_KIND_DAGGER:
					return pCoefficient->ShortSword;
					break;
				case WEAPON_KIND_1H_SWORD:
				case WEAPON_KIND_2H_SWORD:
					return pCoefficient->Sword;
					break;
				case WEAPON_KIND_1H_AXE:
				case WEAPON_KIND_2H_AXE:
					return pCoefficient->Axe;
					break;
				case WEAPON_KIND_1H_CLUP:
				case WEAPON_KIND_2H_CLUP:
					return pCoefficient->Club;
					break;
				case WEAPON_KIND_1H_SPEAR:
				case WEAPON_KIND_2H_SPEAR:
					return pCoefficient->Spear;
					break;
				case WEAPON_KIND_BOW:
				case WEAPON_KIND_CROSSBOW:
					return pCoefficient->Bow;
					break;
				case WEAPON_KIND_STAFF:
					return pCoefficient->Staff;
					break;
				case WEAPON_KIND_JAMADHAR:
					return pCoefficient->Jamadar;
					break;
				case WEAPON_KIND_MACE:
					return pCoefficient->Pole;
					break;
				}
			}
		}
	}
	return 0.0f;
}

void CBot::SetBotAbility(bool bSendPacket)
{
	_CLASS_COEFFICIENT* pCoefficient = g_pMain->m_CoefficientArray.GetData(GetClass());
	if (pCoefficient == nullptr)
		return;

	float Coefficient = SetCoefficient();

	uint16 righthandattackpower = 0, lefthandattackpower = 0;

	if (!isWeaponsDisabled())
	{
		_ITEM_TABLE* pRightHand = GetItemPrototype(GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_RIGHTHAND : RIGHTHAND);
		if (pRightHand != nullptr)
		{
			_ITEM_DATA* pRightData = nullptr;
			if (GetZoneID() == ZONE_KNIGHT_ROYALE)
				pRightData = GetKnightRoyaleItem(KNIGHT_ROYAL_RIGHTHAND);
			else
				pRightData = GetItem(RIGHTHAND);

			if (pRightData != nullptr)
			{
				if (pRightData->sDuration == 0)
					righthandattackpower += (pRightHand->m_sDamage + m_bAddWeaponDamage) / 2;
				else
					righthandattackpower += pRightHand->m_sDamage + m_bAddWeaponDamage;
			}
		}

		_ITEM_TABLE* pLeftHand = GetItemPrototype(GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_LEFTHAND : LEFTHAND);
		if (pLeftHand != nullptr)
		{
			_ITEM_DATA* pLeftData = nullptr;
			if (GetZoneID() == ZONE_KNIGHT_ROYALE)
				pLeftData = GetKnightRoyaleItem(KNIGHT_ROYAL_LEFTHAND);
			else
				pLeftData = GetItem(LEFTHAND);

			if (pLeftData != nullptr)
			{
				if (pLeftHand->isBow())
				{
					if (pLeftData->sDuration == 0)
						lefthandattackpower += (pLeftHand->m_sDamage + m_bAddWeaponDamage) / 2;
					else
						lefthandattackpower += pLeftHand->m_sDamage + m_bAddWeaponDamage;
				}
				else
				{
					if (pLeftData->sDuration == 0)
						lefthandattackpower += ((pLeftHand->m_sDamage + m_bAddWeaponDamage) / 2) / 2;
					else
						lefthandattackpower += (pLeftHand->m_sDamage + m_bAddWeaponDamage) / 2;
				}
			}
		}
	}

	if (righthandattackpower < 0) righthandattackpower = 0;
	if (lefthandattackpower < 0) lefthandattackpower = 0;

	uint16 Totalattackpower = righthandattackpower + lefthandattackpower;

	if (Totalattackpower < 3)
		Totalattackpower = 3;

	// Update stats based on item data
	SetSlotItemValue();

	int mainstr = GetStat(STAT_STR), maindex = GetStat(STAT_DEX), mainint = GetStat(STAT_INT);

	uint32 BaseAp = 0, ApStat = 0;

	if (mainstr > 150)
		BaseAp = mainstr - 150;

	if (mainstr == 160)
		BaseAp--;

	if (mainint > 100)
		BaseAp = mainint - 100;

	if (mainint == 110)
		BaseAp--;

	int totalstr = mainstr + GetStatBonusTotal(STAT_STR), totaldex = maindex + GetStatBonusTotal(STAT_DEX), totalint = mainint + GetStatBonusTotal(STAT_INT);

	uint32 tempMaxWeight = m_sMaxWeight;
	m_sMaxWeight = ((((GetStatWithItemBonus(STAT_STR) + GetLevel()) * 50) + m_sMaxWeightBonus) * (m_bMaxWeightAmount <= 0 ? 1 : m_bMaxWeightAmount / 100)) / 2;

	//if (m_sACAmount < 0) m_sACAmount = 0;
	m_sTotalHit = 0;

	float BonusAp = 1.0;
	BonusAp = (m_byAPBonusAmount + 100) / 100.0f;

	if (isRogue())
		m_sTotalHit = (uint16)(((0.005f * Totalattackpower * (totaldex + 40)) + (Coefficient * Totalattackpower * GetLevel() * totaldex) + 3) * BonusAp);
	else if (isPriest())
	{
		if (mainstr > mainint)
			m_sTotalHit = (uint16)(((0.005f * Totalattackpower * (totalstr + 40)) + (Coefficient * Totalattackpower * GetLevel() * totalstr) + 3) * BonusAp) + BaseAp;
		else if (mainint > mainstr)
			m_sTotalHit = (uint16)(((0.005f * Totalattackpower * (totalint + 40)) + (Coefficient * Totalattackpower * GetLevel() * totalint) + 3) * BonusAp) + BaseAp;
		else
			m_sTotalHit = (uint16)(((0.005f * Totalattackpower * (totalint + 40)) + (Coefficient * Totalattackpower * GetLevel() * totalint) + 3) * BonusAp) + BaseAp;
	}
	else if (isWarrior())
	{
		if (mainstr > mainint)
			m_sTotalHit = (uint16)(((0.005f * Totalattackpower * (totalstr + 40)) + (Coefficient * Totalattackpower * GetLevel() * totalstr) + 3) * BonusAp) + BaseAp;
		else if (mainint > mainstr)
			m_sTotalHit = (uint16)(((0.005f * Totalattackpower * (totalint + 40)) + (Coefficient * Totalattackpower * GetLevel() * totalint) + 3) * BonusAp) + BaseAp;
		else
			m_sTotalHit = (uint16)(((0.005f * Totalattackpower * (totalstr + 40)) + (Coefficient * Totalattackpower * GetLevel() * totalstr) + 3) * BonusAp) + BaseAp;
	}
	else
		m_sTotalHit = (uint16)(((0.005f * Totalattackpower * (totalstr + 40)) + (Coefficient * Totalattackpower * GetLevel() * totalstr) + 3) * BonusAp) + BaseAp;;

	m_sTotalAc = 0;
	m_sTotalAc = (short)(pCoefficient->AC * (GetLevel() + m_sItemAc));

	if (m_sACPercent <= 0) m_sACPercent = 100;

	uint8 bDefenseBonus = 0, bResistanceBonus = 0;

	// Reset resistance bonus
	m_bResistanceBonus = 0;

	if (isWarrior())
	{
		if (isNoviceWarrior())
		{
			//Hinder: [Passive]Increase defense by 10%. If a shield is not equipped, the effect will decrase by half.
			if (CheckSkillPoint(PRO_SKILL2, 5, 14))
				bDefenseBonus = 5;
			//Arrest: Passive]Increase defense by 15 % .If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 15, 34))
				bDefenseBonus = 8;
			//Bulwark: [Passive]Increase defense by 20%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 35, 54))
				bDefenseBonus = 10;
			//Evading: [Passive]Increase defense by 25%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 55, 69))
				bDefenseBonus = 13;

			// Resist: [Passive]Increase all resistance by 30. If a shield is not equipped, the effect will decrease by half.
			if (CheckSkillPoint(PRO_SKILL2, 10, 19))
				bResistanceBonus = 15;
			// Endure: [Passive]Increase all resistance by 60. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 20, 39))
				bResistanceBonus = 30;
			//Immunýty: [Passive]Increase all resistance by 90. If a shield is not equipped, the effect will decrase by half.
			else if (CheckSkillPoint(PRO_SKILL2, 40, 83))
				bResistanceBonus = 45;

			m_bResistanceBonus += bResistanceBonus;
			m_sTotalAc += bDefenseBonus * m_sTotalAc / 100;
		}
		else if (isMasteredWarrior())
		{
			//Hinder: [Passive]Increase defense by 10%. If a shield is not equipped, the effect will decrase by half.
			if (CheckSkillPoint(PRO_SKILL2, 5, 14))
				bDefenseBonus = 20;
			//Arrest: Passive]Increase defense by 15 % .If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 15, 34))
				bDefenseBonus = 34;
			//Bulwark: [Passive]Increase defense by 20%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 35, 54))
				bDefenseBonus = 40;
			//Evading: [Passive]Increase defense by 25%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 55, 69))
				bDefenseBonus = 50;
			// Iron Skin: [Passive]Increase defense by 30%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 70, 79))
				bDefenseBonus = 60;
			//Iron Body: [Passive]Increase defense by 40%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 80, 83))
				bDefenseBonus = 80;

			// Resist: [Passive]Increase all resistance by 30. If a shield is not equipped, the effect will decrease by half.
			if (CheckSkillPoint(PRO_SKILL2, 10, 19))
				bResistanceBonus = 30;
			// Endure: [Passive]Increase all resistance by 60. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 20, 39))
				bResistanceBonus = 60;
			//Immunýty: [Passive]Increase all resistance by 90. If a shield is not equipped, the effect will decrase by half.
			else if (CheckSkillPoint(PRO_SKILL2, 40, 83))
				bResistanceBonus = 90;

			_ITEM_TABLE* pLeftHand = GetItemPrototype(GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_LEFTHAND : LEFTHAND);
			if (pLeftHand == nullptr || !pLeftHand->isShield())
			{
				bDefenseBonus /= 2;	bResistanceBonus /= 2;
			}

			if (bDefenseBonus < 0)
				bDefenseBonus = 0;

			if (bResistanceBonus < 0)
				bResistanceBonus = 0;

			m_bResistanceBonus += bResistanceBonus;
			m_sTotalAc += bDefenseBonus * m_sTotalAc / 100;
		}
	}
	else if (isPortuKurian())
	{
		if (isNoviceKurianPortu())
		{
			//Hinder: [Passive]Increase defense by 10%. If a shield is not equipped, the effect will decrase by half.
			if (CheckSkillPoint(PRO_SKILL2, 5, 14))
				bDefenseBonus = 5;
			//Arrest: Passive]Increase defense by 15 % .If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 15, 34))
				bDefenseBonus = 8;
			//Bulwark: [Passive]Increase defense by 20%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 35, 54))
				bDefenseBonus = 10;
			//Evading: [Passive]Increase defense by 25%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 55, 69))
				bDefenseBonus = 13;

			// Resist: [Passive]Increase all resistance by 30. If a shield is not equipped, the effect will decrease by half.
			if (CheckSkillPoint(PRO_SKILL2, 10, 19))
				bResistanceBonus = 15;
			// Endure: [Passive]Increase all resistance by 60. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 20, 39))
				bResistanceBonus = 30;
			//Immunýty: [Passive]Increase all resistance by 90. If a shield is not equipped, the effect will decrase by half.
			else if (CheckSkillPoint(PRO_SKILL2, 40, 83))
				bResistanceBonus = 45;

			m_bResistanceBonus += bResistanceBonus;
			m_sTotalAc += bDefenseBonus * m_sTotalAc / 100;
		}
		else if (isMasteredKurianPortu())
		{
			//Hinder: [Passive]Increase defense by 10%. If a shield is not equipped, the effect will decrase by half.
			if (CheckSkillPoint(PRO_SKILL2, 5, 14))
				bDefenseBonus = 5;
			//Arrest: Passive]Increase defense by 15 % .If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 15, 34))
				bDefenseBonus = 8;
			//Bulwark: [Passive]Increase defense by 20%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 35, 54))
				bDefenseBonus = 10;
			//Evading: [Passive]Increase defense by 25%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 55, 69))
				bDefenseBonus = 13;
			// Iron Skin: [Passive]Increase defense by 30%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 70, 79))
				bDefenseBonus = 15;
			//Iron Body: [Passive]Increase defense by 40%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 80, 83))
				bDefenseBonus = 20;

			// Resist: [Passive]Increase all resistance by 30. If a shield is not equipped, the effect will decrease by half.
			if (CheckSkillPoint(PRO_SKILL2, 10, 19))
				bResistanceBonus = 15;
			// Endure: [Passive]Increase all resistance by 60. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 20, 39))
				bResistanceBonus = 30;
			//Immunýty: [Passive]Increase all resistance by 90. If a shield is not equipped, the effect will decrase by half.
			else if (CheckSkillPoint(PRO_SKILL2, 40, 83))
				bResistanceBonus = 45;

			if (bDefenseBonus < 0)
				bDefenseBonus = 0;

			if (bResistanceBonus < 0)
				bResistanceBonus = 0;

			m_bResistanceBonus += bResistanceBonus;
			m_sTotalAc += bDefenseBonus * m_sTotalAc / 100;
		}
	}

	if (isMasteredPriest()
		|| isMasteredWarrior())
	{
		// Boldness/Daring [Passive]Increase your defense by 20% when your HP is down to 30% or lower.
		if (m_sHp < 30 * m_MaxHp / 100)
			m_sTotalAc += 20 * m_sTotalAc / 100;
	}
	else if (isMasteredRogue())
	{
		// Valor: [Passive]Increase your resistance by 50 when your HP is down to 30% or below.
		if (m_sHp < 30 * m_MaxHp / 100)
			m_bResistanceBonus += 50;
	}
	else if (isMasteredKurianPortu())
	{
		//Axid Break: [Passive]When HP less than 30% Attack increases by 20%.
		if (CheckSkillPoint(PRO_SKILL4, 15, 23))
		{
			if (m_sHp < 30 * m_MaxHp / 100)
				m_sTotalHit += 20 * m_sTotalHit / 100;
		}
	}

	if (m_bAddWeaponDamage > 0)
		++m_sTotalHit;

	if (m_sAddArmourAc > 0 || m_bPctArmourAc > 100)
		++m_sTotalAc;

	uint8 bSta = GetStat(STAT_STA);
	if (bSta > 100)
		m_sTotalAc += bSta - 100;

	uint8 bInt = GetStat(STAT_INT);
	if (bInt > 100)
		m_bResistanceBonus += (bInt - 100) / 2;

	if (m_sACPercent <= 0) m_sACPercent = 100;

	m_sTotalAc = m_sTotalAc * m_sACPercent / 100;

	m_fTotalHitrate = ((1 + pCoefficient->Hitrate * GetLevel() * totaldex) * m_sItemHitrate / 100) * (m_bHitRateAmount / 100);
	m_fTotalEvasionrate = ((1 + pCoefficient->Evasionrate * GetLevel() * totaldex) * m_sItemEvasionrate / 100) * (m_sAvoidRateAmount / 100);

	SetMaxHp();
	SetMaxMp();

	if (isPortuKurian())
		SetMaxSp();

	// TODO: Transformation stats need to be applied here
	if (GetZoneID() == ZONE_DELOS && isSiegeTransformation())
	{
		_MAGIC_TYPE6* pType = g_pMain->m_Magictype6Array.GetData(m_sTransformSkillID);
		if (pType != nullptr)
		{
			m_sTotalHit = pType->sTotalHit;
			m_sTotalAc = pType->sTotalAc;
			m_MaxHp = pType->sMaxHp;
			m_MaxMp = pType->sMaxMp;
			m_sFireR = pType->sTotalFireR;
			m_sColdR = pType->sTotalColdR;
			m_sLightningR = pType->sTotalLightningR;
			m_sMagicR = pType->sTotalMagicR;
			m_sDiseaseR = pType->sTotalDiseaseR;
			m_sPoisonR = pType->sTotalPoisonR;
		}
	}
}

void CBot::SetSlotItemValue()
{
	_ITEM_TABLE* pTable = nullptr;
	int item_hit = 0, item_ac = 0;

	m_sItemMaxHp = m_sItemMaxMp = 0;
	m_sItemAc = 0;
	m_sItemWeight = m_sMaxWeightBonus = 0;
	m_sItemHitrate = m_sItemEvasionrate = 100;

	memset(m_sStatItemBonuses, 0, sizeof(uint16) * STAT_COUNT);
	m_sFireR = m_sColdR = m_sLightningR = m_sMagicR = m_sDiseaseR = m_sPoisonR = 0;
	m_sDaggerR = m_sSwordR = m_sJamadarR = m_sAxeR = m_sClubR = m_sSpearR = m_sBowR = 0;

	m_byAPBonusAmount = 0;
	memset(&m_byAPClassBonusAmount, 0, sizeof(m_byAPClassBonusAmount));
	memset(&m_byAcClassBonusAmount, 0, sizeof(m_byAcClassBonusAmount));

	m_bItemExpGainAmount = m_bItemNPBonus = m_bItemNoahGainAmount = m_bSkillNPBonus = 0;

	Guard lock(m_equippedItemBonusLock);
	m_equippedItemBonuses.clear();

	std::map<uint16, uint32> setItems;

	// Apply stat bonuses from all equipped & cospre items.
	// Total up the weight of all items.
	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = nullptr;
		pTable = GetItemPrototype(i, pItem);

		if (pTable == nullptr)
			continue;

		// Bags increase max weight, they do not weigh anything.
		if (i == INVENTORY_COSP + COSP_BAG1 + MBAG_COUNT
			|| i == INVENTORY_COSP + COSP_BAG2 + MBAG_COUNT)
		{
			m_sMaxWeightBonus += pTable->m_sDuration;
		}
		// All other items are attributed to the total weight of items in our inventory.
		else
		{
			if (!pTable->m_bCountable)
				m_sItemWeight += pTable->m_sWeight;
			else
				// Non-stackable items should have a count of 1. If not, something's broken.
				m_sItemWeight += pTable->m_sWeight * pItem->sCount;
		}

		// Do not apply stats to unequipped items
		if ((i >= SLOT_MAX && i < INVENTORY_COSP)
			// or disabled weapons.
			|| (isWeaponsDisabled()
				&& (i == (GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_RIGHTHAND : RIGHTHAND)
					|| i == (GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_LEFTHAND : LEFTHAND))
				&& !pTable->isShield())
			// or items in magic bags.
			|| i >= INVENTORY_MBAG
			|| pItem->isDuplicate())
			continue;

		item_ac = pTable->m_sAc;
		if (pItem->sDuration == 0)
			item_ac /= 10;

		m_sItemMaxHp += pTable->m_MaxHpB;
		m_sItemMaxMp += pTable->m_MaxMpB;
		m_sItemAc += item_ac;
		m_sStatItemBonuses[STAT_STR] += pTable->m_sStrB;
		m_sStatItemBonuses[STAT_STA] += pTable->m_sStaB;
		m_sStatItemBonuses[STAT_DEX] += pTable->m_sDexB;
		m_sStatItemBonuses[STAT_INT] += pTable->m_sIntelB;
		m_sStatItemBonuses[STAT_CHA] += pTable->m_sChaB;
		m_sItemHitrate += pTable->m_sHitrate;
		m_sItemEvasionrate += pTable->m_sEvarate;

		m_sFireR += pTable->m_bFireR;
		m_sColdR += pTable->m_bColdR;
		m_sLightningR += pTable->m_bLightningR;
		m_sMagicR += pTable->m_bMagicR;
		m_sDiseaseR += pTable->m_bCurseR;
		m_sPoisonR += pTable->m_bPoisonR;

		m_sDaggerR += pTable->m_sDaggerAc;
		m_sJamadarR += pTable->m_JamadarAc;
		m_sSwordR += pTable->m_sSwordAc;
		m_sAxeR += pTable->m_sAxeAc;
		m_sClubR += pTable->m_sClubAc;
		m_sSpearR += pTable->m_sSpearAc;
		m_sBowR += pTable->m_sBowAc;

		ItemBonusMap bonusMap;
		if (pTable->m_bFireDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_FIRE, pTable->m_bFireDamage));

		if (pTable->m_bIceDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_COLD, pTable->m_bIceDamage));

		if (pTable->m_bLightningDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_LIGHTNING, pTable->m_bLightningDamage));

		if (pTable->m_bPoisonDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_POISON, pTable->m_bPoisonDamage));

		if (pTable->m_bHPDrain)
			bonusMap.insert(std::make_pair(ITEM_TYPE_HP_DRAIN, pTable->m_bHPDrain));

		if (pTable->m_bMPDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_MP_DAMAGE, pTable->m_bMPDamage));

		if (pTable->m_bMPDrain)
			bonusMap.insert(std::make_pair(ITEM_TYPE_MP_DRAIN, pTable->m_bMPDrain));

		if (pTable->m_bMirrorDamage)
			bonusMap.insert(std::make_pair(ITEM_TYPE_MIRROR_DAMAGE, pTable->m_bMirrorDamage));

		// If we have bonuses to apply, store them.
		if (!bonusMap.empty())
			m_equippedItemBonuses[i] = bonusMap;

		// Apply cospre item stats
		if (pTable->GetKind() == ITEM_KIND_COSPRE)
		{
			// If this item exists in the set table, it has bonuses to be applied.
			_SET_ITEM* pSetItem = g_pMain->m_SetItemArray.GetData(pTable->m_iNum);

			if (pSetItem != nullptr)
				ApplySetItemBonuses(pSetItem);
		}

		// All set items start with race over 100
		if (pTable->m_bRace < 100)
			continue;

		// Each set is uniquely identified by item's race
		auto itr = setItems.find(pTable->m_bRace);

		// If the item doesn't exist in our map yet...
		if (itr == setItems.end())
		{
			// Generate the base set ID and insert it into our map
			setItems.insert(std::make_pair(pTable->m_bRace, pTable->m_bRace * 10000));
			itr = setItems.find(pTable->m_bRace);
		}

		// Update the final set ID depending on the equipped set item 
		switch (pTable->m_bSlot)
		{
		case ItemSlotHelmet:
			itr->second += 2;
			break;
		case ItemSlotPauldron:
			itr->second += 16;
			break;
		case ItemSlotPads:
			itr->second += 512;
			break;
		case ItemSlotGloves:
			itr->second += 2048;
			break;
		case ItemSlotBoots:
			itr->second += 4096;
			break;
		}
	}

	// Now we can add up all the set bonuses, if any.
	foreach(itr, setItems)
	{
		// Test if this set item exists (if we're not using at least 2 items from the set, this will fail)
		_SET_ITEM* pItem = g_pMain->m_SetItemArray.GetData(itr->second);

		if (pItem == nullptr)
			continue;

		ApplySetItemBonuses(pItem);
	}

	if (m_sAddArmourAc > 0)
		m_sItemAc += m_sAddArmourAc;
	else
		m_sItemAc = m_sItemAc * m_bPctArmourAc / 100;
}

void CBot::ApplySetItemBonuses(_SET_ITEM* pItem)
{
	m_sItemAc += pItem->ACBonus;
	m_sItemMaxHp += pItem->HPBonus;
	m_sItemMaxMp += pItem->MPBonus;

	m_sStatItemBonuses[STAT_STR] += pItem->StrengthBonus;
	m_sStatItemBonuses[STAT_STA] += pItem->StaminaBonus;
	m_sStatItemBonuses[STAT_DEX] += pItem->DexterityBonus;
	m_sStatItemBonuses[STAT_INT] += pItem->IntelBonus;
	m_sStatItemBonuses[STAT_CHA] += pItem->CharismaBonus;

	m_sFireR += pItem->FlameResistance;
	m_sColdR += pItem->GlacierResistance;
	m_sLightningR += pItem->LightningResistance;
	m_sMagicR += pItem->MagicResistance;
	m_sDiseaseR += pItem->CurseResistance;
	m_sPoisonR += pItem->PoisonResistance;

	m_bItemExpGainAmount += pItem->XPBonusPercent;
	m_bItemNoahGainAmount += pItem->CoinBonusPercent;
	m_bItemNPBonus += pItem->NPBonus;

	m_sMaxWeightBonus += pItem->MaxWeightBonus;

	// NOTE: The following percentages use values such as 3 to indicate +3% (not the typical 103%).
	// Also note that at this time, there are no negative values used, so we can assume it's always a bonus.
	m_byAPBonusAmount += pItem->APBonusPercent;
	if (pItem->APBonusClassType >= 1 && pItem->APBonusClassType <= 4)
		m_byAPClassBonusAmount[pItem->APBonusClassType - 1] += pItem->APBonusClassPercent;

	if (pItem->ACBonusClassType >= 1 && pItem->ACBonusClassType <= 4)
		m_byAcClassBonusAmount[pItem->ACBonusClassType - 1] += pItem->ACBonusClassPercent;
}

_ITEM_TABLE* CBot::GetItemPrototype(uint8 pos, _ITEM_DATA*& pItem)
{
	if (pos >= INVENTORY_TOTAL)
		return nullptr;

	pItem = GetItem(pos);
	return pItem->nNum == 0 ? nullptr : &g_pMain->GetItemPtr(pItem->nNum);
}

bool CBot::CheckSkillPoint(uint8 skillnum, uint8 min, uint8 max)
{
	if (skillnum < 5 || skillnum > 8)
		return false;

	return (m_bstrSkill[skillnum] >= min && m_bstrSkill[skillnum] <= max);
}

uint8 CBot::GetBaseClass()
{
	switch (GetBaseClassType())
	{
	case ClassType::ClassWarrior:
	case ClassType::ClassWarriorNovice:
	case ClassType::ClassWarriorMaster:
		return GROUP_WARRIOR;

	case ClassType::ClassRogue:
	case ClassType::ClassRogueNovice:
	case ClassType::ClassRogueMaster:
		return GROUP_ROGUE;

	case ClassType::ClassMage:
	case ClassType::ClassMageNovice:
	case ClassType::ClassMageMaster:
		return GROUP_MAGE;

	case ClassType::ClassPriest:
	case ClassType::ClassPriestNovice:
	case ClassType::ClassPriestMaster:
		return GROUP_CLERIC;

	case ClassType::ClassPortuKurian:
	case ClassType::ClassPortuKurianNovice:
	case ClassType::ClassPortuKurianMaster:
		return GROUP_PORTU_KURIAN;
	}

	return 0;
}