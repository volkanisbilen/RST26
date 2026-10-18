#include "StdAfx.h"

float CUser::SetCoefficient()
{
	_CLASS_COEFFICIENT* pCoefficient = g_pMain->m_CoefficientArray.GetData(GetClass());
	if (pCoefficient == nullptr)
		return 0.0f;

	_ITEM_TABLE pRightHand = GetItemPrototype(RIGHTHAND);
	if (!pRightHand.isnull())
	{
		switch (pRightHand.m_bKind)
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
		_ITEM_TABLE pLeftHand = GetItemPrototype(LEFTHAND);
		if (!pLeftHand.isnull())
		{
			switch (pLeftHand.m_bKind)
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
	return 0.0f;
}

void CUser::SetUserAbility(bool bSendPacket)
{
	_CLASS_COEFFICIENT* pCoefficient = g_pMain->m_CoefficientArray.GetData(GetClass());
	if (pCoefficient == nullptr) return;
	float Coefficient = SetCoefficient();

	uint16 rightpower = 0, leftpower = 0;
	_ITEM_TABLE pRightHand = GetItemPrototype(RIGHTHAND);
	_ITEM_DATA* pRightData = GetItem(RIGHTHAND);
	if (!pRightHand.isnull() && pRightData) {
		if (pRightData->sDuration == 0) rightpower += (pRightHand.m_sDamage + m_bAddWeaponDamage) / 2;
		else rightpower += pRightHand.m_sDamage + m_bAddWeaponDamage;
	}

	_ITEM_TABLE pLeftHand = GetItemPrototype(LEFTHAND);
	_ITEM_DATA* pLeftData = GetItem(LEFTHAND);
	if (!pLeftHand.isnull() && pLeftData) {
		if (pLeftHand.isBow()) {
			if (pLeftData->sDuration == 0) leftpower += (pLeftHand.m_sDamage + m_bAddWeaponDamage) / 2;
			else leftpower += pLeftHand.m_sDamage + m_bAddWeaponDamage;
		}
		else {
			if (pLeftData->sDuration == 0) leftpower += ((pLeftHand.m_sDamage + m_bAddWeaponDamage) / 2) / 2;
			else leftpower += (pLeftHand.m_sDamage + m_bAddWeaponDamage) / 2;
		}
	}

	if (!rightpower) rightpower = 0; if (!leftpower) leftpower = 0;
	uint16 totalpower = rightpower + leftpower;
	if (totalpower < 3) totalpower = 3;

	// Update stats based on item data
	SetSlotItemValue();

	int mainstr = GetStat(StatType::STAT_STR), maindex = GetStat(StatType::STAT_DEX), mainint = GetStat(StatType::STAT_INT);
	uint32 BaseAp = 0, ApStat = 0;
	//bool intbp = isPriest() && !pRightHand.isnull() && pRightHand.GetKind() == WEAPON_KIND_MACE;
	//if (intbp) { // int bp için ekleme yapayým mý, sende ki nasýlsa öyle kalsýn kanka, bunu kullanabilir miyim?, elbette
	//	if (mainint > 150)
	//		BaseAp = mainint - 150;
	//	if (mainint == 160) BaseAp--;
	//}
	//else {
	//	if (mainstr > 150) BaseAp = mainstr - 150;
	//	if (mainstr == 160) BaseAp--;
	//}
		// int atak priest eklemesi 29.03.2024
	if (isPriest() && mainint > 240)
	{
		if (mainint > 240)
			BaseAp = mainint - 150;

		if (mainint == 160)
			BaseAp--;
	}
	else
		// int atak priest eklemesi 29.03.2024 end
	{
		if (mainstr > 150)
			BaseAp = mainstr - 150;

		if (mainstr == 160)
			BaseAp--;
	}
	int totalstr = mainstr + GetStatBonusTotal(StatType::STAT_STR),
		totaldex = maindex + GetStatBonusTotal(StatType::STAT_DEX)/*,
		totalint = mainint + GetStatBonusTotal(StatType::STAT_INT)*/;
		// int atak priest eklemesi 29.03.2024
	int totalint;
	totalint = mainint + GetStatBonusTotal(StatType::STAT_INT);
	// int atak priest eklemesi 29.03.2024 end

	uint32 tempMaxWeight = m_sMaxWeight; uint16 maxweightbonus = m_sMaxWeightBonus;
	m_sMaxWeight = (((GetStatWithItemBonus(StatType::STAT_STR) + GetLevel()) * 50) + maxweightbonus);

	if (pPerks.perkType[(int)perks::weight] > 0) {
		auto* perks = g_pMain->m_PerksArray.GetData((int)perks::weight);
		if (perks && perks->perkCount && perks->status)
			m_sMaxWeight += (perks->perkCount * pPerks.perkType[(int)perks::weight]) * 10;
	}

	if (tempMaxWeight != m_sMaxWeight) SendItemWeight();

	m_sTotalHit = 0;
	float BonusAp = (m_byAPBonusAmount + 100) / 100.0f;

	int16 achieveattack = m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_ATTACK];
	int16 achievedefens = m_sStatAchieveBonuses[(int16)UserAchieveStatTypes::ACHIEVE_STAT_DEFENCE];

	uint16 power = totalpower;

	if (isRogue())
		m_sTotalHit = (uint16)(((0.005f * power * (totaldex + 40)) + (Coefficient * power * GetLevel() * totaldex) + 3) * BonusAp);
	// int atak priest eklemesi 29.03.2024
	else if (isPriest() && mainint > 240)
		m_sTotalHit = (uint16)(((0.005f * power * (totalint + 40)) + ((Coefficient * 1.7) * power * GetLevel() * totalint) + 3) * BonusAp) + BaseAp;
	// int atak priest eklemesi 29.03.2024 end
	else
		m_sTotalHit = (uint16)(((0.005f * power * (totalstr + 40)) + (Coefficient * power * GetLevel() * totalstr) + 3) * BonusAp) + BaseAp;
	//uint16 power = intbp ? rightpower : totalpower;

	//if (isRogue())
	//	m_sTotalHit = (uint16)(((0.005f * power * (totaldex + 40)) + (Coefficient * power * GetLevel() * totaldex) + 3) * BonusAp);
	//else if (isPriest()) {
	//	int totalstat = intbp ? totalint : totalstr;
	//	m_sTotalHit = (uint16)(((0.005f * power * (totalstat + 40)) + (Coefficient * power * GetLevel() * totalstat) + 3) * BonusAp) + BaseAp;
	//}
	//else
	//	m_sTotalHit = (uint16)(((0.005f * power * (totalstr + 40)) + (Coefficient * power * GetLevel() * totalstr) + 3) * BonusAp) + BaseAp;


	if (achieveattack > 0) m_sTotalHit += achieveattack;

	if (m_sACAmount < 0)
		m_sACAmount = 0;

	if (m_sACSourAmount < 0)
		m_sACSourAmount = 0;

	m_sTotalAc = (short)(pCoefficient->AC * (GetLevel() + m_sItemAc));
	
	if (pPerks.perkType[(int)perks::defence] > 0) {
		auto* perks = g_pMain->m_PerksArray.GetData((int)perks::defence);
		if (perks && perks->perkCount && perks->status)
			m_sTotalAc += perks->perkCount * pPerks.perkType[(int)perks::defence];
	}

	if (pPerks.perkType[(int)perks::attack] > 0) {
		auto* perks = g_pMain->m_PerksArray.GetData((int)perks::attack);
		if (perks && perks->perkCount && perks->status)
			m_sTotalHit += perks->perkCount * pPerks.perkType[(int)perks::attack];
	}

	if (m_sACPercent <= 0) m_sACPercent = 100;

	uint8 bDefenseBonus = 0, bResistanceBonus = 0;

	// Reset resistance bonus
	m_bResistanceBonus = 0;

	if (isWarrior()) {
		if (isNoviceWarrior()) {
			//Hinder: [Passive]Increase defense by 10%. If a shield is not equipped, the effect will decrase by half.
			if (CheckSkillPoint(PRO_SKILL2, 5, 14)) bDefenseBonus = 5;
			//Arrest: Passive]Increase defense by 15 % .If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 15, 34)) bDefenseBonus = 8;
			//Bulwark: [Passive]Increase defense by 20%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 35, 54)) bDefenseBonus = 10;
			//Evading: [Passive]Increase defense by 25%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 55, 69)) bDefenseBonus = 13;

			// Resist: [Passive]Increase all resistance by 30. If a shield is not equipped, the effect will decrease by half.
			if (CheckSkillPoint(PRO_SKILL2, 10, 19)) bResistanceBonus = 15;
			// Endure: [Passive]Increase all resistance by 60. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 20, 39)) bResistanceBonus = 30;
			//Immunity: [Passive]Increase all resistance by 90. If a shield is not equipped, the effect will decrase by half.
			else if (CheckSkillPoint(PRO_SKILL2, 40, 83)) bResistanceBonus = 45;

			m_bResistanceBonus += bResistanceBonus;
			m_sTotalAc += bDefenseBonus * m_sTotalAc / 100;
		}
		else if (isMasteredWarrior()) {
			//Hinder: [Passive]Increase defense by 10%. If a shield is not equipped, the effect will decrase by half.
			if (CheckSkillPoint(PRO_SKILL2, 5, 14)) bDefenseBonus = 20;
			//Arrest: Passive]Increase defense by 15 % .If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 15, 34)) bDefenseBonus = 34;
			//Bulwark: [Passive]Increase defense by 20%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 35, 54)) bDefenseBonus = 40;
			//Evading: [Passive]Increase defense by 25%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 55, 69)) bDefenseBonus = 50;
			// Iron Skin: [Passive]Increase defense by 30%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 70, 79)) bDefenseBonus = 60;
			//Iron Body: [Passive]Increase defense by 40%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 80, 83)) bDefenseBonus = 80;

			// Resist: [Passive]Increase all resistance by 30. If a shield is not equipped, the effect will decrease by half.
			if (CheckSkillPoint(PRO_SKILL2, 10, 19)) bResistanceBonus = 30;
			// Endure: [Passive]Increase all resistance by 60. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 20, 39)) bResistanceBonus = 60;
			//Immunity: [Passive]Increase all resistance by 90. If a shield is not equipped, the effect will decrase by half.
			else if (CheckSkillPoint(PRO_SKILL2, 40, 83)) bResistanceBonus = 90;

			if (pLeftHand.isnull() || !pLeftHand.isShield()) {
				if (bDefenseBonus) bDefenseBonus /= 2;
				if (bResistanceBonus) bResistanceBonus /= 2;
			}

			m_bResistanceBonus += bResistanceBonus;
			m_sTotalAc += bDefenseBonus * m_sTotalAc / 100;
		}
	}
	else if (isPortuKurian()) {
		if (isNoviceKurianPortu()) {
			//Hinder: [Passive]Increase defense by 10%. If a shield is not equipped, the effect will decrase by half.
			if (CheckSkillPoint(PRO_SKILL2, 5, 14)) bDefenseBonus = 5;
			//Arrest: Passive]Increase defense by 15 % .If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 15, 34)) bDefenseBonus = 8;
			//Bulwark: [Passive]Increase defense by 20%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 35, 54)) bDefenseBonus = 10;
			//Evading: [Passive]Increase defense by 25%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 55, 69)) bDefenseBonus = 13;

			// Resist: [Passive]Increase all resistance by 30. If a shield is not equipped, the effect will decrease by half.
			if (CheckSkillPoint(PRO_SKILL2, 10, 19)) bResistanceBonus = 15;
			// Endure: [Passive]Increase all resistance by 60. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 20, 39)) bResistanceBonus = 30;
			//Immunity: [Passive]Increase all resistance by 90. If a shield is not equipped, the effect will decrase by half.
			else if (CheckSkillPoint(PRO_SKILL2, 40, 83)) bResistanceBonus = 45;

			m_bResistanceBonus += bResistanceBonus;
			m_sTotalAc += bDefenseBonus * m_sTotalAc / 100;
		}
		else if (isMasteredKurianPortu()) {
			//Hinder: [Passive]Increase defense by 10%. If a shield is not equipped, the effect will decrase by half.
			if (CheckSkillPoint(PRO_SKILL2, 5, 14)) bDefenseBonus = 5;
			//Arrest: Passive]Increase defense by 15 % .If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 15, 34)) bDefenseBonus = 8;
			//Bulwark: [Passive]Increase defense by 20%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 35, 54)) bDefenseBonus = 10;
			//Evading: [Passive]Increase defense by 25%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 55, 69)) bDefenseBonus = 13;
			// Iron Skin: [Passive]Increase defense by 30%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 70, 79)) bDefenseBonus = 15;
			//Iron Body: [Passive]Increase defense by 40%. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 80, 83)) bDefenseBonus = 20;

			// Resist: [Passive]Increase all resistance by 30. If a shield is not equipped, the effect will decrease by half.
			if (CheckSkillPoint(PRO_SKILL2, 10, 19)) bResistanceBonus = 15;
			// Endure: [Passive]Increase all resistance by 60. If a shield is not equipped, the effect will decrease by half.
			else if (CheckSkillPoint(PRO_SKILL2, 20, 39)) bResistanceBonus = 30;
			//Immunity: [Passive]Increase all resistance by 90. If a shield is not equipped, the effect will decrase by half.
			else if (CheckSkillPoint(PRO_SKILL2, 40, 83)) bResistanceBonus = 45;

			m_bResistanceBonus += bResistanceBonus;
			m_sTotalAc += bDefenseBonus * m_sTotalAc / 100;
		}
	}

	if (isMasteredPriest() || isMasteredWarrior()) {
		// Boldness/Daring [Passive]Increase your defense by 20% when your HP is down to 30% or lower.
		if (m_sHp < 30 * ((int32)m_MaxHp / 100))
			m_sTotalAc += 20 * m_sTotalAc / 100;
	}
	else if (isMasteredRogue()) {
		// Valor: [Passive]Increase your resistance by 50 when your HP is down to 30% or below.
		if (m_sHp < 30 * m_MaxHp / 100)
			m_bResistanceBonus += 50;
	}
	else if (isMasteredKurianPortu()) {
		//Axid Break: [Passive]When HP less than 30% Attack increases by 20%.
		if (CheckSkillPoint(PRO_SKILL4, 15, 23)) {
			if (m_sHp < 30 * m_MaxHp / 100)
				m_sTotalHit += 20 * m_sTotalHit / 100;
		}
	}

	if (m_bAddWeaponDamage > 0)
		++m_sTotalHit;

	if (m_sAddArmourAc > 0 || m_bPctArmourAc > 100)
		++m_sTotalAc;

	uint8 bSta = GetStat(StatType::STAT_STA);
	if (bSta > 100)
		m_sTotalAc += bSta - 100;

	uint8 bInt = GetStat(StatType::STAT_INT);
	if (bInt > 100)
		m_bResistanceBonus += (bInt - 100) / 2;

	if (m_sACPercent < 1) m_sACPercent = 100;

	if (achievedefens > 0) m_sTotalAc += achievedefens;
	m_sTotalAc = m_sTotalAc * m_sACPercent / 100;

	m_fTotalHitrate = ((1 + pCoefficient->Hitrate * GetLevel() * totaldex) * m_sItemHitrate / 100) * (m_bHitRateAmount / 100);
	m_fTotalEvasionrate = ((1 + pCoefficient->Evasionrate * GetLevel() * totaldex) * m_sItemEvasionrate / 100) * (m_sAvoidRateAmount / 100);

	if (GetZoneID() == ZONE_DELOS && isSiegeTransformation()) {
		auto *pType = g_pMain->m_Magictype6Array.GetData(m_sTransformSkillID);
		if (pType) {
			m_sTotalHit = pType->sTotalHit;
			m_sTotalAc = pType->sTotalAc;
			m_MaxHp = pType->sMaxHp;
			m_MaxMp = pType->sMaxMp;
			m_sSpeed = pType->bSpeed;
			m_sFireR = pType->sTotalFireR;
			m_sColdR = pType->sTotalColdR;
			m_sLightningR = pType->sTotalLightningR;
			m_sMagicR = pType->sTotalMagicR;
			m_sDiseaseR = pType->sTotalDiseaseR;
			m_sPoisonR = pType->sTotalPoisonR;
		}
	}

	SetMaxHp();
	SetMaxMp();

	if (isPortuKurian())
		SetMaxSp();

	if (bSendPacket)
		SendItemMove(1, 2);
}