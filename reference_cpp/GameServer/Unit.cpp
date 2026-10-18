#include "stdafx.h"
#include "GameServerDlg.h"
#include "MagicInstance.h"
#include "KnightsManager.h"

#include <cfloat>

Unit::Unit(UnitType unitType)
	: m_pMap(nullptr), m_pRegion(nullptr), m_sRegionX(0), m_sRegionZ(0), m_unitType(unitType)
{
	Initialize();
}

void Unit::Initialize()
{
	m_pMap = nullptr;
	m_pRegion = nullptr;

	SetPosition(0.0f, 0.0f, 0.0f);
	m_bLevel = 0;
	m_bNation = 0;

	if (this->isNPC()) InitType4(true);
	InitType3();

	AbsorbedAmmount = 0;
	m_sTotalHit = 0;
	m_sTotalAc = 0;
	m_fTotalHitrate = 0.0f;
	m_fTotalEvasionrate = 0.0f;

	m_bResistanceBonus = 0;
	m_sFireR = m_sColdR = m_sLightningR = m_sMagicR = m_sDiseaseR = m_sPoisonR = 0;
	m_sDaggerR = m_sSwordR = m_sJamadarR = m_sAxeR = m_sClubR = m_sSpearR = m_sBowR = 0;
	m_byDaggerRAmount = m_byBowRAmount = 100;

	//for (size_t i = 0; i < 8; i++)
	//	MilanSkillID[i] = 0;

	m_bCanStealth = true;
	m_bReflectArmorType = 0;
	m_bIsBlinded = false;
	m_bCanUseSkills = m_bCanUsePotions = m_bCanTeleport = m_bGearSkills = true;
	m_bInstantCast = false;
	m_bIsUndead = m_bIsKaul = m_bIsDevil = false;

	m_bBlockPhysical = m_bBlockMagic = false;
	m_bBlockCurses = m_bReflectCurses = false;
	m_bMirrorDamage = false;
	m_bMirrorDamageType = false;
	m_byMirrorAmount = 0;

	m_transformationType = TransformationType::TransformationNone;
	m_sTransformID = 0;
	m_sTransformSkillID = 0;
	m_sTransformMpchange = false;
	m_sTransformHpchange = false;
	m_tTransformationStartTime = 0;
	m_sTransformationDuration = 0;

	m_sAttackSpeedAmount = 100;		// this is for the duration spells Type 4
	m_bSpeedAmount = 100;
	m_sACAmount = 0;
	m_sACSourAmount = 0;
	m_sACPercent = 100;
	m_bAttackAmount = 100;
	m_sMagicAttackAmount = 0;
	m_sMaxHPAmount = m_sMaxMPAmount = 0;
	m_bHitRateAmount = 100;
	m_sAvoidRateAmount = 100;
	m_bAddFireR = m_bAddColdR = m_bAddLightningR = 0;
	m_bAddMagicR = m_bAddDiseaseR = m_bAddPoisonR = 0;
	m_bPctFireR = m_bPctColdR = m_bPctLightningR = 100;
	m_bPctMagicR = m_bPctDiseaseR = m_bPctPoisonR = 100;
	m_bMagicDamageReduction = 100;
	m_bManaAbsorb = 0;
	AbsorbCount = 0;
	m_bRadiusAmount = 0;
	m_buffCount = 0;

	m_oSocketID = m_oBarrackID = -1;
	m_bEventRoom = 0;
}

/*
NOTE: Due to KO messiness, we can really only calculate a 2D distance/
There are a lot of instances where the y (height level, in this case) coord isn't set,
which understandably screws things up a lot.
*/
// Calculate the distance between 2 2D points.
float Unit::GetDistance(float fx, float fz)
{
	return (float)GetDistance(GetX(), GetZ(), fx, fz);
}

float Unit::Get3Distance(float fx, float fz, float fy)
{
	return Get3Distance(GetX(), GetZ(), GetY(), fx, fz, fy);
}

// Calculate the 2D distance between Units.
float Unit::GetDistance(Unit* pTarget)
{
	if (pTarget == nullptr)
		return -FLT_MAX;

	if (GetZoneID() != pTarget->GetZoneID())
		return -FLT_MAX;

	return GetDistance(pTarget->GetX(), pTarget->GetZ());
}

float Unit::GetDistanceSqrt(Unit* pTarget)
{
	if (pTarget == nullptr)
		return -FLT_MAX;

	if (GetZoneID() != pTarget->GetZoneID())
		return -FLT_MAX;

	return sqrtf(GetDistance(pTarget->GetX(), pTarget->GetZ()));
}

// Check to see if the Unit is in 2D range of another Unit.
// Range MUST be squared already.
bool Unit::isInRange(Unit* pTarget, float fSquaredRange)
{
	return (GetDistance(pTarget) <= fSquaredRange);
}

bool Unit::isInRange3D(float fx, float fz, float fy, float fSquaredRange)
{
	return (Get3Distance(fx, fz, fy) <= fSquaredRange);
}

// Check to see if we're in the 2D range of the specified coordinates.
// Range MUST be squared already.
bool Unit::isInRange(float fx, float fz, float fSquaredRange)
{
	return (GetDistance(fx, fz) <= fSquaredRange);
}

// Check to see if the Unit is in 2D range of another Unit.
// Range must NOT be squared already.
// This is less preferable to the more common precalculated range.
bool Unit::isInRangeSlow(Unit* pTarget, float fNonSquaredRange)
{
	return isInRange(pTarget, pow(fNonSquaredRange, 2.0f));
}

// Check to see if the Unit is in 2D range of the specified coordinates.
// Range must NOT be squared already.
// This is less preferable to the more common precalculated range.
bool Unit::isInRangeSlow(float fx, float fz, float fNonSquaredRange)
{
	return isInRange(fx, fz, pow(fNonSquaredRange, 2.0f));
}

float Unit::GetDistance(float fStartX, float fStartZ, float fEndX, float fEndZ)
{
	return pow(fStartX - fEndX, 2.0f) + pow(fStartZ - fEndZ, 2.0f);
}

float Unit::Get3Distance(float fStartX, float fStartZ, float fStartY, float fEndX, float fEndZ, float fEndY)
{
	return pow(fStartX - fEndX, 2.0f) + pow(fStartZ - fEndZ, 2.0f) + pow(fStartY - fEndY, 2.0f);
}

bool Unit::isInRange(float fStartX, float fStartZ, float fEndX, float fEndZ, float fSquaredRange)
{
	return (GetDistance(fStartX, fStartZ, fEndX, fEndZ) <= fSquaredRange);
}

bool Unit::isInRangeSlow(float fStartX, float fStartZ, float fEndX, float fEndZ, float fNonSquaredRange)
{
	return isInRange(fStartX, fStartZ, fEndX, fEndZ, pow(fNonSquaredRange, 2.0f));
}

void Unit::SetRegion(uint16 x /*= -1*/, uint16 z /*= -1*/)
{
	m_sRegionX = x; m_sRegionZ = z;
	m_pRegion = m_pMap->GetRegion(x, z); // TODO: Clean this up
}

bool Unit::RegisterRegion()
{
	uint16
		new_region_x = GetNewRegionX(), new_region_z = GetNewRegionZ(),
		old_region_x = GetRegionX(), old_region_z = GetRegionZ();

	if (GetRegion() == nullptr || (old_region_x == new_region_x && old_region_z == new_region_z))
		return false;
	//
	AddToRegion(new_region_x, new_region_z);

	RemoveRegion(old_region_x - new_region_x, old_region_z - new_region_z);
	InsertRegion(new_region_x - old_region_x, new_region_z - old_region_z);

	return true;
}

void Unit::RemoveRegion(int16 del_x, int16 del_z)
{
	if (GetMap() == nullptr)
		return;

	Packet result;
	GetInOut(result, INOUT_OUT);
	g_pMain->Send_OldRegions(&result, del_x, del_z, GetMap(), GetRegionX(), GetRegionZ(), nullptr, GetEventRoom());
}

void Unit::InsertRegion(int16 insert_x, int16 insert_z)
{
	if (GetMap() == nullptr)
		return;

	Packet result;
	GetInOut(result, INOUT_IN);
	g_pMain->Send_NewRegions(&result, insert_x, insert_z, GetMap(), GetRegionX(), GetRegionZ(), nullptr, GetEventRoom());
}

short CUser::GetDamage(Unit* pTarget, _MAGIC_TABLE pSkill /*= _MAGIC_TABLE()*/, bool bPreviewOnly /*= false*/)// gm damage düzenlemesi end short to int
{
	//int32 damage = 0;
	int damage = 0;// gm damage düzenlemesi end
	int random = 0;
	int32 temp_hit = 0, temp_ap = 0, temp_hit_B = 0;
	uint8 result;

	if (pTarget == nullptr || pTarget->isDead())
		return -1;

	// Trigger item procs
	if (pTarget->isPlayer() && !bPreviewOnly)
	{
		OnAttack(pTarget, AttackType::AttackTypePhysical);
		pTarget->OnDefend(this, AttackType::AttackTypePhysical);
	}

	if (m_sACAmount < 0) m_sACAmount = 0;
	if (m_sACSourAmount < 0) m_sACSourAmount = 0;

	bool disablearmorscroll = false;
	if (!pSkill.isnull() && (pSkill.iNum == 107640 ||
		pSkill.iNum == 108640 ||
		pSkill.iNum == 207640 ||
		pSkill.iNum == 208640 ||

		pSkill.iNum == 107620 ||
		pSkill.iNum == 108620 ||
		pSkill.iNum == 207620 ||
		pSkill.iNum == 208620 ||

		pSkill.iNum == 107600 ||
		pSkill.iNum == 108600 ||
		pSkill.iNum == 207600 ||
		pSkill.iNum == 208600 ||

		pSkill.iNum == 108670 ||
		pSkill.iNum == 208670))
		disablearmorscroll = true;

	int32 temp_ac = pTarget->m_sTotalAc + (!disablearmorscroll ? pTarget->m_sACAmount : 0);
	if (pTarget->m_sACSourAmount > 0) temp_ac -= pTarget->m_sACSourAmount;
	if (temp_ac < 0) temp_ac = 0;

	temp_ap = m_sTotalHit * m_bAttackAmount;

	// Apply player vs player AC/AP bonuses.
	if (pTarget->isPlayer())
	{
		CUser* pTUser = TO_USER(pTarget);	// NOTE: using a = a*v instead of a *= v because the compiler assumes different 
		// types being multiplied, which results in these calcs not behaving correctly.

		// adjust player AP by percent, for skills like "Critical Point"
		temp_ap = temp_ap * m_bPlayerAttackAmount / 100;

		// Now handle class-specific AC/AP bonuses.
		temp_ac = temp_ac * (100 + pTUser->m_byAcClassBonusAmount[pTUser->GetBaseClass() - 1]) / 100;
		temp_ap = temp_ap * (100 + m_byAPClassBonusAmount[pTUser->GetBaseClass() - 1]) / 100;
	}

	double acc = temp_ac;
	if (pTarget->isNPC() && temp_ac)
	{
		acc *= g_pMain->pDamageSetting.mondef;
		temp_ac = (int32)acc;
	}

	if (temp_ac < 1)
		temp_ac = 0;

	// Allow for complete physical damage blocks.
	// NOTE: Unsure if we need to count skill usage as magic damage or if we 
	// should only block that in explicit magic damage skills (CMagicProcess::GetMagicDamage()).
	if (pTarget->m_bBlockPhysical)
		return 0;

	temp_hit_B = (int)((temp_ap * 200 / 100) / (temp_ac + 240));

	// Skill/arrow hit.    
	if (!pSkill.isnull())
	{
		//skill düzenlemesi 03.10.2023 ekleme
		// SKILL HIT! YEAH!	                                
		if (pSkill.bType[0] == 1)
		{
			_MAGIC_TYPE1* pType1 = g_pMain->m_Magictype1Array.GetData(pSkill.iNum);
			if (pType1 == nullptr)
				return -1;

			// Non-relative hit.
			if (pType1->bHitType)
				result = (pType1->sHitRate <= myrand(0, 100) ? FAIL : SUCCESS);
			// Relative hit.
			else
				result = GetHitRate((m_fTotalHitrate / pTarget->m_fTotalEvasionrate) * (pType1->sHitRate / 100.0f));

			temp_hit = (int32)(temp_hit_B * (pType1->sHit / 100.0f));
		}
		// ARROW HIT! YEAH!
		else if (pSkill.bType[0] == 2)
		{
			_MAGIC_TYPE2* pType2 = g_pMain->m_Magictype2Array.GetData(pSkill.iNum);
			if (pType2 == nullptr)
				return -1;

			// Non-relative/Penetration hit.
			if (pType2->bHitType == 1 || pType2->bHitType == 2)
				result = (pType2->sHitRate <= myrand(0, 100) ? FAIL : SUCCESS);
			// Relative hit/Arc hit.
			else
				result = GetHitRate((m_fTotalHitrate / pTarget->m_fTotalEvasionrate) * (pType2->sHitRate / 100.0f));

			if (pType2->bHitType == 1)
				temp_hit = (int32)(m_sTotalHit * m_bAttackAmount * (pType2->sAddDamage / 100.0f) / 100);
			else
				temp_hit = (int32)(temp_hit_B * (pType2->sAddDamage / 100.0f));

			temp_hit = int32(temp_hit * 1.8); // okçu damage arttırma
		}
		//skill düzenlemesi 03.10.2023 ekleme end
	}
	// Normal hit (R attack)     
	else
	{
		temp_hit = temp_ap / 100;
		result = GetHitRate(m_fTotalHitrate / pTarget->m_fTotalEvasionrate + 1.0f);
	}
	bool rDamage = false;
	switch (result)
	{						// 1. Magical item damage....
	case GREAT_SUCCESS:
	case SUCCESS:
	case NORMAL:
	{
		if (!pSkill.isnull())
		{
			// Skill Hit.
			damage = temp_hit;
			random = myrand(0, damage / 10);
			if (pSkill.bType[0] == 1)
				damage = (short)((temp_hit + 0.3f * random) + 0.99f);
			else
				damage = (short)(((temp_hit * 0.6f) + 1.0f * random) + 0.99f);
		}
		else
		{
			rDamage = true;
			// Normal Hit.
			if (isGM() && !pTarget->isPlayer())
			{
				// gm damage düzenlemesi
				damage = 30000;
				//printf("amountGM 5: %d \n", damage);
				return damage;
				// gm damage düzenlemesi end
			}

			//skill düzenlemesi 03.10.2023 ekleme end
			damage = temp_hit_B;
			random = myrand(0, damage);
			damage = (short)((0.85f * temp_hit_B) + 0.3f * random);
			//skill düzenlemesi 03.10.2023 ekleme end
		}
	}
	break;
	case FAIL:
		if (!pSkill.isnull())
		{	 // Skill Hit.

		}
		else
		{ // Normal Hit.
			if (isGM() && !pTarget->isPlayer())
			{
				damage = 30000;
				//printf("amountGM 3: %d \n", damage);
				return damage;
				//damage = 30000;
				//return damage;
			}
		}
	}

	double daaa = damage;
	if (pTarget->isNPC() && damage) { daaa *= g_pMain->pDamageSetting.montakedamage; damage = (int32)daaa; }

	// Apply item bonuses
	damage = GetMagicDamage(damage, pTarget, bPreviewOnly);
	// These two only apply to players
	if (pTarget->isPlayer())
	{

		damage = GetACDamage(damage, pTarget); // 3. Additional AC calculation....	
		//23.06.2023 fr damage firedamage düzenlemesi
		if (g_pMain->m_bisDamage == false)
		{
			_CLASS_DAMAGE* pCoefficientDamage = g_pMain->m_CoefficientDamageArray.GetData(GetClass());
			if (pCoefficientDamage != nullptr)
			{
				double dm = (double)damage;

				if (TO_USER(pTarget)->isWarrior())
					dm *= pCoefficientDamage->sWarrior;
				else if (TO_USER(pTarget)->isMage())
					dm *= pCoefficientDamage->sMage;
				else if (TO_USER(pTarget)->isPriest())
					dm *= pCoefficientDamage->sPriest;
				else if (TO_USER(pTarget)->isRogue())
					dm *= pCoefficientDamage->sRogue;
				else if (TO_USER(pTarget)->isPortuKurian())
					dm *= pCoefficientDamage->sKurian;

				damage = (int32)dm;
			}
		}

		if (GetMap()->isWarZone())
			damage /= 3;
		else
			damage /= 2;
		//23.06.2023 fr damage firedamage düzenlemesi end
	}

	//TADOO unique item damage arttırma


	if (pTarget->isNPC() && damage)
	{
		uint32 perkDamage = 0;
		if (pPerks.perkType[(int)perks::percentDamageMonster] > 0)
		{
			auto* perks = g_pMain->m_PerksArray.GetData((int)perks::percentDamageMonster);
			if (perks && perks->perkCount)
				perkDamage += perks->perkCount * pPerks.perkType[(int)perks::percentDamageMonster];
		}
		if (perkDamage)
			damage += damage * perkDamage / 100;
	}
	else if (pTarget->isPlayer() && damage)
	{
		uint32 perkDamage = 0;
		if (pPerks.perkType[(int)perks::percentDamagePlayer] > 0)
		{
			auto* perks = g_pMain->m_PerksArray.GetData((int)perks::percentDamagePlayer);
			if (perks && perks->perkCount)
				perkDamage += perks->perkCount * pPerks.perkType[(int)perks::percentDamagePlayer];
		}
		if (perkDamage)
			damage += damage * perkDamage / 100;
	}

	/*if (g_pMain->pServerSetting.ActiveUniqueItemBonusDamage > 0)
	{
		double dm = (double)damage;

		if (damage && isMage())
			dm *= getplusdamage();

		damage = (int32)dm;

		if (!rDamage)
		{
			uint32 itemid[2]{}, minDamage[2]{}, maxDamage[2]{};
			memset(itemid, 0, sizeof(itemid));
			memset(minDamage, 0, sizeof(minDamage));
			memset(maxDamage, 0, sizeof(maxDamage));
			uint8 slot[] = { LEFTHAND, RIGHTHAND }, index = 0;
			foreach_array(i, slot)
			{
				if (index > 1) break;
				auto pWeapon = GetItemPrototype(slot[i]);
				if (!pWeapon.isnull()
					&& pWeapon.minDamage > 0
					&& pWeapon.maxDamage > 0)
				{

					itemid[index] = pWeapon.Getnum();
					minDamage[index] = pWeapon.minDamage;
					maxDamage[index] = pWeapon.maxDamage;
				}
				index++;
			}

			uint32 max_d = 0, min_d = 0;
			for (int i = 0; i < 2; i++)
			{
				if (!itemid[i] || maxDamage[i] < max_d) continue;
				max_d = maxDamage[i];
				min_d = minDamage[i];
			}
			if (min_d > max_d) max_d = min_d;

			if (min_d && max_d)
				damage += myrand(min_d, max_d);
		}
	}*/
	int32 eksidamage = 0;
	eksidamage = int32(damage * 0.1);
	//printf("damage1: %d \n", damage);

	if (eksidamage > 9)
		damage += myrand(-eksidamage, eksidamage);

	//printf("damage2: %d \n", damage);

	//printf("amountGM 154: %d \n", damage);
	if (damage > MAX_DAMAGE)
		damage = MAX_DAMAGE;
	else if (damage < -MAX_DAMAGE)
		damage = -MAX_DAMAGE;



	//loot düzenlemesi 19.03.2024
	if (pTarget->isNPC() && damage > 0)
	{
		if (pTarget->GetHealth() <= damage)
			damage = pTarget->GetHealth();

		//printf("loot düzenlemesi damage: %d \n", damage);
	}
	//loot düzenlemesi 19.03.2024 end

	//printf("amountGM 155: %d \n", damage);
	return damage;
}
//short CUser::GetDamage(Unit *pTarget, _MAGIC_TABLE pSkill /*= _MAGIC_TABLE()*/, bool bPreviewOnly /*= false*/)
//{
//	int32 damage = 0;
//	int random = 0;
//	int32 temp_hit = 0, temp_ap = 0, temp_hit_B = 0;
//	uint8 result;
//
//	if (pTarget == nullptr || pTarget->isDead())
//		return -1;
//
//	// Trigger item procs
//	if (pTarget->isPlayer() && !bPreviewOnly)
//	{
//		OnAttack(pTarget, AttackType::AttackTypePhysical);
//		pTarget->OnDefend(this, AttackType::AttackTypePhysical);
//	}
//
//	if (m_sACAmount < 0) m_sACAmount = 0;
//	if (m_sACSourAmount < 0) m_sACSourAmount = 0;
//
//	bool disablearmorscroll = false;
//	if (!pSkill.isnull() && (pSkill.iNum == 107640 ||
//		pSkill.iNum == 108640 ||
//		pSkill.iNum == 207640 ||
//		pSkill.iNum == 208640 ||
//
//		pSkill.iNum == 107620 ||
//		pSkill.iNum == 108620 ||
//		pSkill.iNum == 207620 ||
//		pSkill.iNum == 208620 ||
//
//		pSkill.iNum == 107600 ||
//		pSkill.iNum == 108600 ||
//		pSkill.iNum == 207600 ||
//		pSkill.iNum == 208600 ||
//
//		pSkill.iNum == 108670 ||
//		pSkill.iNum == 208670))
//		disablearmorscroll = true;
//
//	int32 temp_ac = pTarget->m_sTotalAc + (!disablearmorscroll ? pTarget->m_sACAmount : 0);
//	if(pTarget->m_sACSourAmount > 0) temp_ac -= pTarget->m_sACSourAmount;
//	if (temp_ac < 0) temp_ac = 0;
//
//	temp_ap = m_sTotalHit * m_bAttackAmount;
//
//	// Apply player vs player AC/AP bonuses.
//	if (pTarget->isPlayer())
//	{
//		CUser * pTUser = TO_USER(pTarget);	// NOTE: using a = a*v instead of a *= v because the compiler assumes different 
//		// types being multiplied, which results in these calcs not behaving correctly.
//
//		// adjust player AP by percent, for skills like "Critical Point"
//		temp_ap = temp_ap * m_bPlayerAttackAmount / 100;
//
//		// Now handle class-specific AC/AP bonuses.
//		temp_ac = temp_ac * (100 + pTUser->m_byAcClassBonusAmount[pTUser->GetBaseClass() - 1]) / 100;
//		temp_ap = temp_ap * (100 + m_byAPClassBonusAmount[pTUser->GetBaseClass() - 1]) / 100;
//	}
//
//	double acc = temp_ac;
//	if (pTarget->isNPC() && temp_ac) { 
//		acc *= g_pMain->pDamageSetting.mondef;
//		temp_ac = (int32)acc;
//	}
//
//	if (temp_ac < 1)
//		temp_ac = 0;
//
//	// Allow for complete physical damage blocks.
//	// NOTE: Unsure if we need to count skill usage as magic damage or if we 
//	// should only block that in explicit magic damage skills (CMagicProcess::GetMagicDamage()).
//	if (pTarget->m_bBlockPhysical)
//		return 0;
//
//	temp_hit_B = (int)((temp_ap * 200 / 100) / (temp_ac + 240));
//
//	// Skill/arrow hit.    
//	if (!pSkill.isnull())
//	{
//		// SKILL HIT! YEAH!	                                
//		if (pSkill.bType[0] == 1)
//		{
//			_MAGIC_TYPE1 *pType1 = g_pMain->m_Magictype1Array.GetData(pSkill.iNum);
//			if (pType1 == nullptr)
//				return -1;
//
//			// Non-relative hit.
//			if (pType1->bHitType)
//				result = (pType1->sHitRate <= myrand(0, 100) ? FAIL : SUCCESS);
//			// Relative hit.
//			else
//				result = GetHitRate((m_fTotalHitrate / pTarget->m_fTotalEvasionrate) * (pType1->sHitRate / 100.0f));
//
//			if (result == 4 && 25 > myrand(0, 100)) result = SUCCESS; //13.01.2021 Skill sekmesin diye kapatıldı bir altındaki kod hiç sekmemesi için eklendi
//			if (result == FAIL) result = SUCCESS; //hiç sekmemesi için olan ayar
//			temp_hit = (int32)(temp_hit_B * (pType1->sHit / 100.0f));
//		}
//		// ARROW HIT! YEAH!
//		else if (pSkill.bType[0] == 2)
//		{
//			_MAGIC_TYPE2 *pType2 = g_pMain->m_Magictype2Array.GetData(pSkill.iNum);
//			if (pType2 == nullptr)
//				return -1;
//
//			// Non-relative/Penetration hit.
//			if (pType2->bHitType == 1 || pType2->bHitType == 2)
//				result = (pType2->sHitRate <= myrand(0, 100) ? FAIL : SUCCESS);
//			// Relative hit/Arc hit.
//			else
//				result = GetHitRate((m_fTotalHitrate / pTarget->m_fTotalEvasionrate) * (pType2->sHitRate / 100.0f));
//
//			if (result == 4 && 25 > myrand(0, 100)) result = SUCCESS; //13.01.2021 Skill sekmesin diye kapatıldı bir altındaki kod hiç sekmemesi için eklendi
//			//if (result == FAIL) result = SUCCESS; //hiç sekmemesi için olan ayar
//			if (pType2->bHitType == 1)
//				temp_hit = (int32)(m_sTotalHit * m_bAttackAmount * (pType2->sAddDamage / 100.0f) / 100);
//			else
//				temp_hit = (int32)(temp_hit_B * (pType2->sAddDamage / 100.0f));
//		}
//	}
//	// Normal hit (R attack)     
//	else
//	{
//		temp_hit = temp_ap / 100;
//		result = GetHitRate(m_fTotalHitrate / pTarget->m_fTotalEvasionrate + 1.0f);
//	}
//	bool rDamage = false;
//	switch (result)
//	{						// 1. Magical item damage....
//	case GREAT_SUCCESS:
//	case SUCCESS:
//	case NORMAL:
//	{
//		if (!pSkill.isnull())
//		{
//			// Skill Hit.
//			damage = temp_hit;
//			random = myrand(0, damage / 10);
//			if (pSkill.bType[0] == 1)
//				damage = (short)((temp_hit + 0.3f * random) + 0.99f);
//			else
//				damage = (short)(((temp_hit * 0.6f) + 1.0f * random) + 0.99f);
//		}
//		else
//		{	
//			rDamage = true;
//			// Normal Hit.
//			if (isGM() && !pTarget->isPlayer())
//			{
//				if (g_pMain->m_nGameMasterRHitDamage != 0)
//				{
//					damage = g_pMain->m_nGameMasterRHitDamage;
//					return damage;
//				}
//			}
//			damage = temp_hit_B;
//			random = myrand(0, damage);
//
//			int32 fakedamage = 0;
//			if (isPriest()) {
//				fakedamage = (short)((0.15f * temp_hit_B) + 0.2f * random);
//			}
//			else {
//				fakedamage = (short)((0.75f * temp_hit_B) + 0.3f * random);
//			}
//			damage = fakedamage;
//
//			if (GetLevel() > 30 && !isPriest()) {
//				double dm = (double)damage;
//				dm *= g_pMain->pDamageSetting.rdamage;
//				damage = (int32)dm;
//			}
//		}
//	}
//	break;
//	case FAIL:
//		if (!pSkill.isnull())
//		{	 // Skill Hit.
//
//		}
//		else { // Normal Hit.
//			if (isGM() && !pTarget->isPlayer())
//			{
//				damage = 30000;
//				return damage;
//			}
//		}
//	}
//
//	double daaa = damage;
//	if (pTarget->isNPC() && damage) { daaa *= g_pMain->pDamageSetting.montakedamage; damage = (int32)daaa; }
//
//	// Apply item bonuses
//	damage = GetMagicDamage(damage, pTarget, bPreviewOnly);
//	if (pTarget->isPlayer()) {
//		damage = GetACDamage(damage, pTarget); // 3. Additional AC calculation....	
//		if (isPriest()
//			&& pTarget->isPlayer()
//			&& (TO_USER(pTarget)->isWarrior())) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.priestTOwarriror;//1.12f;
//			damage = (int32)dm;
//		}
//		else if (isPriest()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isMage()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.priestTOmage;//1.12f;
//			damage = (int32)dm;
//		}
//		else if (isPriest()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isPriest()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.priestTOpriest;//1.5f;
//			damage = (int32)dm;
//		}
//		else if (isPriest()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isRogue()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.priestTOrogue;//1.12f;
//			damage = (int32)dm;
//		}
//		else if (isPriest()
//			&& pTarget != nullptr
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isPortuKurian()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.priestTOkurian;//1.02f;
//			damage = (int32)dm;
//		}
//		else if (isWarrior()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isRogue()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.warrirorTOrogue;//0.88f;
//			damage = (int32)dm;
//		}
//		else if (isWarrior()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isMage()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.warrirorTOmage;//0.94f;
//			damage = (int32)dm;
//		}
//		else if (isWarrior()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isWarrior()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.warrirorTOwarrior;//0.92f;
//			damage = (int32)dm;
//		}
//		else if (isWarrior()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isPriest()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.warrirorTOpriest;//0.77f;
//			damage = (int32)dm;
//		}
//		else if (isWarrior()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isPortuKurian()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.warrirorTOkurian;//0.95f;
//			damage = (int32)dm;
//		}
//		else if (isRogue()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isMage()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.rogueTOmage;//0.85f;
//			damage = (int32)dm;
//		}
//		else if (isRogue()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isWarrior()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.rogueTOwarrior;//0.93f;
//			damage = (int32)dm;
//		}
//		else if (isRogue()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isRogue()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.rogueTOrogue;//0.88f;
//			damage = (int32)dm;
//		}
//		else if (isRogue()
//			&& pTarget != nullptr
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isPriest()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.rogueTOpriest;//0.75f;
//			damage = (int32)dm;
//		}
//		else if (isRogue()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isPortuKurian()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.rogueTOkurian;//0.87f;
//			damage = (int32)dm;
//		}
//		else if (isPortuKurian()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isMage()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.kurianTOmage;//0.97f;
//			damage = (int32)dm;
//		}
//		else if (isPortuKurian()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isWarrior()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.kurianTOwarrior;//0.93f;
//			damage = (int32)dm;
//		}
//		else if (isPortuKurian()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isRogue()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.kurianTOrogue;//0.90f;
//			damage = (int32)dm;
//		}
//		else if (isPortuKurian()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isPriest()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.kurianTOpriest;//0.70f;
//			damage = (int32)dm;
//		}
//		else if (isPortuKurian()
//			&& pTarget->isPlayer()
//			&& TO_USER(pTarget)->isPortuKurian()) {
//			double dm = (double)damage;
//			dm *= g_pMain->pDamageSetting.kurianTOkurian;//0.98f;
//			damage = (int32)dm;
//		}
//		else damage /= 3;
//	}
//	
//	//TADOO unique item damage arttırma
//	double dm = (double)damage;
//
//
//	if (pTarget->isNPC() && damage)
//	{
//		uint32 perkDamage = 0;
//		if (pPerks.perkType[(int)perks::percentDamageMonster] > 0) {
//			auto* perks = g_pMain->m_PerksArray.GetData((int)perks::percentDamageMonster);
//			if (perks && perks->perkCount)
//				perkDamage += perks->perkCount * pPerks.perkType[(int)perks::percentDamageMonster];
//		}
//		if (perkDamage)
//			damage += damage * perkDamage / 100;
//	}
//	else if (pTarget->isPlayer() && damage)
//	{
//		uint32 perkDamage = 0;
//		if (pPerks.perkType[(int)perks::percentDamagePlayer] > 0) {
//			auto* perks = g_pMain->m_PerksArray.GetData((int)perks::percentDamagePlayer);
//			if (perks && perks->perkCount)
//				perkDamage += perks->perkCount * pPerks.perkType[(int)perks::percentDamagePlayer];
//		}
//		if (perkDamage)
//			damage += damage * perkDamage / 100;
//	}
//
//	if (damage && isMage())
//		dm *= getplusdamage();
//	
//	damage = (int32)dm;
//
//	if (!rDamage)
//	{
//		uint32 itemid[2]{}, minDamage[2]{}, maxDamage[2]{};
//		memset(itemid, 0, sizeof(itemid));
//		memset(minDamage, 0, sizeof(minDamage));
//		memset(maxDamage, 0, sizeof(maxDamage));
//		uint8 slot[] = { LEFTHAND, RIGHTHAND }, index = 0;
//		foreach_array(i, slot) {
//			if (index > 1) break;
//			auto pWeapon = GetItemPrototype(slot[i]);
//			if (!pWeapon.isnull()
//				&& pWeapon.minDamage > 0
//				&& pWeapon.maxDamage > 0) {
//
//				itemid[index] = pWeapon.Getnum();
//				minDamage[index] = pWeapon.minDamage;
//				maxDamage[index] = pWeapon.maxDamage;
//			}
//			index++;
//		}
//
//		uint32 max_d = 0, min_d = 0;
//		for (int i = 0; i < 2; i++) {
//			if (!itemid[i] || maxDamage[i] < max_d) continue;
//			max_d = maxDamage[i];
//			min_d = minDamage[i];
//		}
//		if (min_d > max_d) max_d = min_d;
//
//		if (min_d && max_d)
//			damage += myrand(min_d, max_d);
//	}
//
//	if (damage > MAX_DAMAGE)
//		damage = MAX_DAMAGE;
//	else if (damage < -MAX_DAMAGE)
//		damage = -MAX_DAMAGE;
//
//	return damage;
//}


bool CUser::TriggerProcZoneChecking(uint8 TargetZoneID, uint8 OwnerZoneID)
{
	if (!TargetZoneID
		|| TargetZoneID <= 0
		|| !OwnerZoneID
		|| OwnerZoneID <= 0)
		return false;

	switch (TargetZoneID)
	{
	case ZONE_CHAOS_DUNGEON:
	case ZONE_DUNGEON_DEFENCE:
	case ZONE_KNIGHT_ROYALE:
		return false;
		break;
	}

	switch (OwnerZoneID)
	{
	case ZONE_CHAOS_DUNGEON:
	case ZONE_DUNGEON_DEFENCE:
	case ZONE_KNIGHT_ROYALE:
		return false;
		break;
	}

	return true;
}

void CUser::OnAttack(Unit* pTarget, AttackType attackType)
{
	if (pTarget == nullptr
		|| !pTarget->isPlayer()
		|| attackType == AttackType::AttackTypeMagic)
		return;

	if (!TriggerProcZoneChecking(pTarget->GetZoneID(), GetZoneID()))
		return;

	// Trigger weapon procs for the attacker on attack
	static const uint8 itemSlots[] = { RIGHTHAND, LEFTHAND };

	foreach_array(i, itemSlots)
	{
		// If we hit an applicable weapon, don't try proc'ing the other weapon. 
		// It is only ever meant to proc on the dominant weapon.
		if (TriggerProcItem(itemSlots[i], pTarget, ItemTriggerType::TriggerTypeAttack))
			break;
	}
}

void CUser::OnDefend(Unit* pAttacker, AttackType attackType)
{
	if (pAttacker == nullptr || !pAttacker->isPlayer())
		return;

	if (!TriggerProcZoneChecking(pAttacker->GetZoneID(), GetZoneID()))
		return;

	// Trigger defensive procs for the defender when being attacked
	static const uint8 itemSlots[] = { RIGHTHAND, LEFTHAND };
	foreach_array(i, itemSlots)
	{
		// If we hit an applicable weapon, don't try proc'ing the other weapon. 
		// It is only ever meant to proc on the dominant weapon.
		if (TriggerProcItem(itemSlots[i], pAttacker, ItemTriggerType::TriggerTypeDefend))
			break;
	}
}

/**
* @brief	Trigger item procs.
*
* @param	bSlot	   	Slot of item to attempt to proc.
* @param	pTarget	   	Target of the skill (attacker/defender depending on the proc type).
* @param	triggerType	Which type of item to proc (offensive/defensive).
*
* @return	true if there's an applicable item to proc, false if not.
*/
bool CUser::TriggerProcItem(uint8 bSlot, Unit* pTarget, ItemTriggerType triggerType)  // Royal Event için fixlendi 23.04.2020 21:10
{
	// Don't proc weapon skills if our weapon is disabled.
	if (triggerType == ItemTriggerType::TriggerTypeAttack && isWeaponsDisabled())
		return false;

	_ITEM_DATA* pItem = GetItem(bSlot);
	if (pItem == nullptr
		// and that it doesn't need to be repaired.
		|| pItem->sDuration == 0)
		return false; // not an applicable item

	// Ensure that this item has an attached proc skill in the table
	_ITEM_OP* pData = g_pMain->m_ItemOpArray.GetData(pItem->nNum);
	if (pData == nullptr // no skill to proc
		|| pData->bTriggerType != (uint8)triggerType) // don't use an offensive proc when we're defending (or vice versa)
		return false; // not an applicable item

	// At this point, we have determined there is an applicable item in the slot.
	// Should it proc now? (note: CheckPercent() checks out of 1000)
	if (!CheckPercent(pData->bTriggerRate * 10))
		return true; // it is an applicable item, it just didn't proc. No need to proc subsequent items.

	MagicInstance instance;

	instance.bIsRunProc = true;
	instance.bIsItemProc = true;
	instance.sCasterID = GetID();
	instance.nSkillID = pData->nSkillID;
	instance.sSkillCasterZoneID = pTarget->GetZoneID();

	auto pTable = g_pMain->GetMagicPtr(pData->nSkillID);
	if (pTable.isnull())
		return false;

	// For AOE skills such as "Splash", the AOE should be focus on the target.
	switch (pTable.bMoral)
	{
	case MORAL_ENEMY:
	case MORAL_NPC:
		instance.sTargetID = pTarget->GetID();
		break;
	case MORAL_SELF:
		instance.sTargetID = GetID();
		break;
	case MORAL_AREA_ENEMY:
		instance.sTargetID = -1;
		instance.sData[0] = (uint16)pTarget->GetX();
		instance.sData[2] = (uint16)pTarget->GetZ();
		break;
	}

	instance.Run();
	return true; // it is an applicable item, and it proc'd. No need to proc subsequent items.
}

short CNpc::GetDamage(Unit* pTarget, _MAGIC_TABLE pSkill /*= _MAGIC_TABLE()*/, bool bPreviewOnly /*= false*/)
{
	if (pTarget->isPlayer())
		return GetDamage(TO_USER(pTarget), pSkill);

	return GetDamage(TO_NPC(pTarget), pSkill);
}

/**
* @brief	Calculates damage for monsters/NPCs attacking players.
*
* @param	pTarget			Target player.
* @param	pSkill			The skill (not used here).
* @param	bPreviewOnly	true to preview the damage only and not apply any item bonuses (not used here).
*
* @return	The damage.
*/
short CNpc::GetDamage(CUser* pTarget, _MAGIC_TABLE pSkill /*= _MAGIC_TABLE*/, bool bPreviewOnly /*= false*/)
{
	if (pTarget == nullptr)
		return 0;

	int32 damage = 0, HitB;

	if (pTarget->m_sACAmount <= 0) pTarget->m_sACAmount = 0;
	if (pTarget->m_sACSourAmount <= 0) pTarget->m_sACSourAmount = 0;

	int32 Ac = (pTarget->m_sTotalAc + pTarget->m_sACAmount) - pTarget->m_sACSourAmount;
	// A unit's total AC shouldn't ever go below 0.
	if (Ac < 0)
		Ac = 0;

	HitB = (int)((m_sTotalHit * m_bAttackAmount * 200 / 100) / (Ac + 240));

	if (HitB <= 0)
		return 0;

	uint8 result = GetHitRate(m_fTotalHitrate / pTarget->m_fTotalEvasionrate);
	if (isGuard())
		result = GREAT_SUCCESS;

	switch (result)
	{
	case GREAT_SUCCESS:
		damage = (int)(0.3f * myrand(0, HitB));
		damage += (short)(0.85f * (float)HitB);
		damage = (damage * 3) / 2;
		break;

	case SUCCESS:
	case NORMAL:
		damage = (int)(0.3f * myrand(0, HitB));
		damage += (short)(0.85f * (float)HitB);
		break;
	}

	int nMaxDamage = (int)(2.6 * m_sTotalHit);
	if (damage > nMaxDamage)
		damage = nMaxDamage;

	// Enforce overall damage cap
	if (damage > MAX_DAMAGE)
		damage = MAX_DAMAGE;

	return (short)damage;
}

/**
* @brief	Calculates damage for monsters/NPCs attacking other monsters/NPCs.
*
* @param	pTarget			Target NPC/monster.
* @param	pSkill			The skill (not used here).
* @param	bPreviewOnly	true to preview the damage only and not apply any item bonuses (not used here).
*
* @return	The damage.
*/
short CNpc::GetDamage(CNpc* pTarget, _MAGIC_TABLE pSkill /*= _MAGIC_TABLE*/, bool bPreviewOnly /*= false*/)
{
	if (pTarget == nullptr)
		return 0;

	short damage = 0, Hit = m_sTotalHit, Ac = pTarget->m_sTotalAc;
	uint8 result = GetHitRate(m_fTotalHitrate / pTarget->m_fTotalEvasionrate);

	if (isPet()
		|| pTarget->isPet()
		|| isGuardSummon()
		|| pTarget->isGuardSummon())
		result = GREAT_SUCCESS;

	switch (result)
	{
	case GREAT_SUCCESS:
		damage = (short)(0.6 * Hit);
		if (damage <= 0)
		{
			damage = 0;
			break;
		}
		damage = myrand(0, damage);
		damage += (short)(0.7 * Hit);
		break;

	case SUCCESS:
	case NORMAL:
		if (Hit - Ac > 0)
		{
			damage = (short)(0.6 * (Hit - Ac));
			if (damage <= 0)
			{
				damage = 0;
				break;
			}
			damage = myrand(0, damage);
			damage += (short)(0.7 * (Hit - Ac));
		}
		break;
	}

	// Enforce damage cap
	if (damage > MAX_DAMAGE)
		damage = MAX_DAMAGE;

	return damage;
}

short CBot::GetDamage(Unit* pTarget, _MAGIC_TABLE pSkill /*= nullptr*/, bool bPreviewOnly /*= false*/)
{
	int32 damage = 0;
	int random = 0;
	int32 temp_hit = 0, temp_ac = 0, temp_ap = 0, temp_hit_B = 0;
	uint8 result;

	if (pTarget == nullptr || pTarget->isDead())
		return -1;

	// Trigger item procs
	if (pTarget->isPlayer() && !bPreviewOnly)
	{
		OnAttack(pTarget, AttackType::AttackTypePhysical);
		pTarget->OnDefend(this, AttackType::AttackTypePhysical);
	}

	temp_ac = pTarget->m_sTotalAc + pTarget->m_sACAmount;
	// A unit's total AC shouldn't ever go below 0.
	if (temp_ac < 0)
		temp_ac = 0;

	temp_ap = m_sTotalHit * m_bAttackAmount;
	// Apply player vs player AC/AP bonuses.
	if (pTarget->isPlayer())
	{
		CUser* pTUser = TO_USER(pTarget);
		// NOTE: using a = a*v instead of a *= v because the compiler assumes different 
		// types being multiplied, which results in these calcs not behaving correctly.											
		// adjust player AP by percent, for skills like "Critical Point"
		temp_ap = temp_ap * m_bPlayerAttackAmount / 100;
		// Now handle class-specific AC/AP bonuses.
		temp_ac = temp_ac * (100 + pTUser->m_byAcClassBonusAmount[pTUser->GetBaseClass() - 1]) / 100;
		temp_ap = temp_ap * (100 + m_byAPClassBonusAmount[pTUser->GetBaseClass() - 1]) / 100;
	}
	else if (pTarget->isBot())
	{
		CBot* pTUser = TO_BOT(pTarget);
		// NOTE: using a = a*v instead of a *= v because the compiler assumes different 
		// types being multiplied, which results in these calcs not behaving correctly.											
		// adjust player AP by percent, for skills like "Critical Point"
		temp_ap = temp_ap * m_bPlayerAttackAmount / 100;
		// Now handle class-specific AC/AP bonuses.
		temp_ac = temp_ac * (100 + pTUser->m_byAcClassBonusAmount[pTUser->GetBaseClass() - 1]) / 100;
		temp_ap = temp_ap * (100 + m_byAPClassBonusAmount[pTUser->GetBaseClass() - 1]) / 100;
	}
	// Allow for complete physical damage blocks.
	// NOTE: Unsure if we need to count skill usage as magic damage or if we 
	// should only block that in explicit magic damage skills (CMagicProcess::GetMagicDamage()).
	if (pTarget->m_bBlockPhysical)
		return 0;

	temp_hit_B = (int)((temp_ap * 200 / 100) / (temp_ac + 240));
	// Skill/arrow hit.    
	if (!pSkill.isnull())
	{
		// SKILL HIT! YEAH!	                                
		if (pSkill.bType[0] == 1)
		{
			_MAGIC_TYPE1* pType1 = g_pMain->m_Magictype1Array.GetData(pSkill.iNum);
			if (pType1 == nullptr)
				return -1;

			// Non-relative hit.
			if (pType1->bHitType == 2)
				result = pType1->sHitRate <= myrand(0, 100) ? FAIL : SUCCESS;
			else if (pType1->bHitType == 1)
				result = SUCCESS;
			else
				result = GetHitRate((m_fTotalHitrate / pTarget->m_fTotalEvasionrate) * (pType1->sHitRate / 100.0f));

			temp_hit = (int32)(temp_hit_B * (pType1->sHit / 100.0f));
		}
		// ARROW HIT! YEAH!
		else if (pSkill.bType[0] == 2)
		{
			_MAGIC_TYPE2* pType2 = g_pMain->m_Magictype2Array.GetData(pSkill.iNum);
			if (pType2 == nullptr)
				return -1;

			// Non-relative/Penetration hit.
			if (pType2->bHitType == 0)
				result = (pType2->sHitRate <= myrand(0, 100) ? FAIL : SUCCESS);
			else
				result = GetHitRate((m_fTotalHitrate / pTarget->m_fTotalEvasionrate) * (pType2->sHitRate / 100.0f));

			if (pType2->bHitType == 1)
				temp_hit = (int32)(m_sTotalHit * m_bAttackAmount * (pType2->sAddDamage / 100.0f) / 100);
			else
				temp_hit = (int32)(temp_hit_B * (pType2->sAddDamage / 100.0f));
		}
	}
	// Normal hit (R attack)     
	else
	{
		temp_hit = temp_ap / 100;
		result = GetHitRate(m_fTotalHitrate / pTarget->m_fTotalEvasionrate + 1.0f);
	}

	switch (result)
	{
	case GREAT_SUCCESS:
	case SUCCESS:
	case NORMAL:
	{
		if (!pSkill.isnull())
		{	 // Skill Hit.
			damage = temp_hit;
			random = myrand(0, damage);
			if (pSkill.bType[0] == 1)
				damage = (short)((temp_hit + 0.3f * random) + 0.99f);
			else
				damage = (short)(((temp_hit * 0.6f) + 1.0f * random) + 0.99f);
		}
		else
		{	// Normal Hit.

			if (isGM() && pTarget->isNPC())
			{
				if (g_pMain->m_nGameMasterRHitDamage != 0)
				{
					damage = g_pMain->m_nGameMasterRHitDamage;
					return damage;
				}
			}
			damage = temp_hit_B;
			random = myrand(0, damage);
			damage = (short)((0.85f * temp_hit_B) + 0.3f * random);
		}
	}
	break;
	case FAIL:
	{
		if (!pSkill.isnull())
		{
			// Skill Hit.
		}
		else
		{
			// Normal Hit.
			if (isGM() && pTarget->isNPC())
			{
				damage = 30000;
				return damage;
			}
		}
	}break;
	}

	// Apply item bonuses
	damage = GetMagicDamage(damage, pTarget, bPreviewOnly);
	// These two only apply to players
	if (pTarget->isPlayer())
	{
		damage = GetACDamage(damage, pTarget);
		// 3. Additional AC calculation....	
		// Give increased damage in war zones (as per official 1.298~1.325)
		// This may need to be revised to use the last nation to win the war, or more accurately, 
		// the nation that last won the war 3 times in a row (whether official behaves this way now is unknown).
#if 0
		if (g_pMain->m_bisDamage == false)
		{
			_CLASS_DAMAGE* pCoefficientDamage = g_pMain->m_CoefficientDamageArray.GetData(GetClass());
			if (pCoefficientDamage != nullptr)
			{
				double dm = (double)damage;

				if (TO_USER(pTarget)->isWarrior())
					dm *= pCoefficientDamage->sWarrior;
				else if (TO_USER(pTarget)->isMage())
					dm *= pCoefficientDamage->sMage;
				else if (TO_USER(pTarget)->isPriest())
					dm *= pCoefficientDamage->sPriest;
				else if (TO_USER(pTarget)->isRogue())
					dm *= pCoefficientDamage->sRogue;
				else if (TO_USER(pTarget)->isPortuKurian())
					dm *= pCoefficientDamage->sKurian;

				damage = (int32)dm;
			}
		}
#endif
		if (GetMap()->isWarZone())
			damage /= 3;
		else
			damage /= 2;
	}
	else if (pTarget->isBot())// These two only apply to Bots
	{
		damage = GetACDamage(damage, pTarget);
		// 3. Additional AC calculation....	
		// Give increased damage in war zones (as per official 1.298~1.325)
		// This may need to be revised to use the last nation to win the war, or more accurately, 
		// the nation that last won the war 3 times in a row (whether official behaves this way now is unknown).
#if 0
		if (g_pMain->m_bisDamage == false)
		{
			_CLASS_DAMAGE* pCoefficientDamage = g_pMain->m_CoefficientDamageArray.GetData(GetClass());
			if (pCoefficientDamage != nullptr)
			{
				double dm = (double)damage;

				if (TO_BOT(pTarget)->isWarrior())
					dm *= pCoefficientDamage->sWarrior;
				else if (TO_BOT(pTarget)->isMage())
					dm *= pCoefficientDamage->sMage;
				else if (TO_BOT(pTarget)->isPriest())
					dm *= pCoefficientDamage->sPriest;
				else if (TO_BOT(pTarget)->isRogue())
					dm *= pCoefficientDamage->sRogue;
				else if (TO_BOT(pTarget)->isPortuKurian())
					dm *= pCoefficientDamage->sKurian;

				damage = (int32)dm;
			}
		}
#endif
		if (GetMap()->isWarZone())
			damage /= 3;
		else
			damage /= 2;
	}

	// Enforce damage cap
	if (damage > MAX_DAMAGE)
		damage = MAX_DAMAGE;
	else if (damage < -MAX_DAMAGE)
		damage = -MAX_DAMAGE;

	if (pTarget->GetID() > NPC_BAND)
	{
		if (!isGM())
		{
			switch (TO_NPC(pTarget)->GetType())
			{
			case NPC_FOSIL:
				damage = 0;
				break;
			case NPC_PRISON:
				damage = 0;
				break;
			case NPC_TREE:
				damage = 0;
				break;
			case NPC_EXP_EVENT_R:
				damage = 0;
				break;
			}
		}
	}
	if (damage > 1000)
		damage = 1000;

	return damage;
}

short Unit::GetMagicDamage(int damage, Unit* pTarget, bool bPreviewOnly)
{
	if (pTarget == nullptr || pTarget->isDead() || pTarget->isBlinking())
		return 0;

	//return damage;

	int16 sReflectDamage = 0;

	int totalhp = 0, totalmana = 0;
	if (isPlayer())
	{
		CUser* pUser = TO_USER(this);
		pUser->m_equippedItemBonusLock.lock();
		foreach(itr, pUser->m_equippedItemBonuses)
		{
			foreach(bonusItr, itr->second)
			{
				short total_r = 0, temp_damage = 0;
				uint8 bType = bonusItr->first;
				int16 sAmount = bonusItr->second;

				bool bIsDrain = (bType >= ITEM_TYPE_HP_DRAIN && bType <= ITEM_TYPE_MP_DRAIN);
				if (bIsDrain) temp_damage = damage * sAmount / 100;

				switch (bType)
				{
				case ITEM_TYPE_FIRE:
					total_r = (pTarget->m_sFireR + pTarget->m_bAddFireR) * pTarget->m_bPctFireR / 100;
					break;
				case ITEM_TYPE_COLD:
					total_r = (pTarget->m_sColdR + pTarget->m_bAddColdR) * pTarget->m_bPctColdR / 100;
					break;
				case ITEM_TYPE_LIGHTNING:
					total_r = (pTarget->m_sLightningR + pTarget->m_bAddLightningR) * pTarget->m_bPctLightningR / 100;
					break;
				case ITEM_TYPE_HP_DRAIN:
					totalhp += temp_damage;
					break;
				case ITEM_TYPE_MP_DAMAGE:
					pTarget->MSpChange(-temp_damage);
					break;
				case ITEM_TYPE_MP_DRAIN:
					totalmana += temp_damage;
					break;
				}
				total_r += pTarget->m_bResistanceBonus;
				if (!bIsDrain)
				{
					if (total_r > 200) total_r = 200;
					temp_damage = sAmount - sAmount * total_r / 200;
					damage += temp_damage;
				}
			}
		}
		pUser->m_equippedItemBonusLock.unlock();
	}

	if (totalhp > 0)
		HpChange(totalhp);

	if (totalmana > 0)
		MSpChange(totalmana);

	if (pTarget->isPlayer())
	{
		CUser* pUser = TO_USER(pTarget);
		pUser->m_equippedItemBonusLock.lock();
		foreach(itr, pUser->m_equippedItemBonuses)
		{
			foreach(bonusItr, itr->second)
				if (bonusItr->first == ITEM_TYPE_MIRROR_DAMAGE)
					sReflectDamage += bonusItr->second;
		}
		pUser->m_equippedItemBonusLock.unlock();
	}

	if (sReflectDamage > 0)
	{
		short temp_damage = damage * sReflectDamage / 100;
		HpChange(-temp_damage);
	}

	return damage;

}

short Unit::GetACDamage(int damage, Unit* pTarget)
{
	// This isn't applicable to NPCs.
	if (!isPlayer() || !pTarget->isPlayer())
		return damage;

	if (pTarget->isDead())
		return 0;

	CUser* pUser = TO_USER(this);
	if (pUser == nullptr) return damage;

	if (pUser->isWeaponsDisabled())
		return damage;

	uint8 weaponSlots[] = { LEFTHAND, RIGHTHAND };
	int firstdamage = damage;

	foreach_array(slot, weaponSlots)
	{
		_ITEM_TABLE pWeapon = pUser->GetItemPrototype(weaponSlots[slot]);
		if (pWeapon.isnull())
			continue;

		if (pWeapon.isDagger())
			damage -= damage * (pTarget->m_sDaggerR * pTarget->m_byDaggerRAmount / 100) / 250;
		else if (pWeapon.isSword())
			damage -= damage * pTarget->m_sSwordR / 250;
		else if (pWeapon.isAxe())
			damage -= damage * pTarget->m_sAxeR / 250;
		else if (pWeapon.isClub())
			damage -= damage * pTarget->m_sClubR / 250;
		else if (pWeapon.isSpear())
			damage -= damage * pTarget->m_sSpearR / 250;
		else if (pWeapon.isBow())
			damage -= damage * (pTarget->m_sBowR * pTarget->m_byBowRAmount / 100) / 250;
		else if (pWeapon.isJamadar())
			damage -= damage * pTarget->m_sJamadarR / 250;
	}
	return damage;
}

uint8 Unit::GetHitRate(float rate)
{
	int random = myrand(1, 10000);
	if (rate >= 5.0f)
	{
		if (random >= 1 && random <= 3500)
			return GREAT_SUCCESS;
		else if (random >= 3501 && random <= 7500)
			return SUCCESS;
		else if (random >= 7501 && random <= 9800)
			return NORMAL;
	}
	else if (rate < 5.0f && rate >= 3.0f)
	{
		if (random >= 1 && random <= 2500)
			return GREAT_SUCCESS;
		else if (random >= 2501 && random <= 6000)
			return SUCCESS;
		else if (random >= 6001 && random <= 9600)
			return NORMAL;
	}
	else if (rate < 3.0f && rate >= 2.0f)
	{
		if (random >= 1 && random <= 2000)
			return GREAT_SUCCESS;
		else if (random >= 2001 && random <= 5000)
			return SUCCESS;
		else if (random >= 5001 && random <= 9400)
			return NORMAL;
	}
	else if (rate < 2.0f && rate >= 1.25f)
	{
		if (random >= 1 && random <= 1500)
			return GREAT_SUCCESS;
		else if (random >= 1501 && random <= 4000)
			return SUCCESS;
		else if (random >= 4001 && random <= 9200)
			return NORMAL;
	}
	else if (rate < 1.25f && rate >= 0.8f)
	{
		if (random >= 1 && random <= 1000)
			return GREAT_SUCCESS;
		else if (random >= 1001 && random <= 3000)
			return SUCCESS;
		else if (random >= 3001 && random <= 9000)
			return NORMAL;
	}
	else if (rate < 0.8f && rate >= 0.5f)
	{
		if (random >= 1 && random <= 800)
			return GREAT_SUCCESS;
		else if (random >= 801 && random <= 2500)
			return SUCCESS;
		else if (random >= 2501 && random <= 8000)
			return NORMAL;
	}
	else if (rate < 0.5f && rate >= 0.33f)
	{
		if (random >= 1 && random <= 600)
			return GREAT_SUCCESS;
		else if (random >= 601 && random <= 2000)
			return SUCCESS;
		else if (random >= 2001 && random <= 7000)
			return NORMAL;
	}
	else if (rate < 0.33f && rate >= 0.2f)
	{
		if (random >= 1 && random <= 400)
			return GREAT_SUCCESS;
		else if (random >= 401 && random <= 1500)
			return SUCCESS;
		else if (random >= 1501 && random <= 6000)
			return NORMAL;
	}
	else
	{
		if (random >= 1 && random <= 200)
			return GREAT_SUCCESS;
		else if (random >= 201 && random <= 1000)
			return SUCCESS;
		else if (random >= 1001 && random <= 5000)
			return NORMAL;
	}

	return FAIL;
}

void Unit::SendToRegion(Packet* result)
{
	g_pMain->Send_Region(result, GetMap(), GetRegionX(), GetRegionZ(), nullptr, GetEventRoom());
}

void Unit::InitType3()
{
	for (int i = 0; i < MAX_TYPE3_REPEAT; i++)
		m_durationalSkills[i].Reset();

	m_bType3Flag = false;
}

void Unit::InitType4(bool bRemoveSavedMagic /*= false*/, uint8 buffType /* = 0 */)
{
	m_buffLock.lock();
	Type4BuffMap buffMap = m_buffMap; // copy the map
	if (buffMap.empty())
	{
		m_buffLock.unlock();
		return;
	}
	std::vector<uint8> RemoveBuffMap;
	foreach(itr, buffMap)
	{
		if (buffType > 0 && itr->second.m_bBuffType != buffType)
			continue;

		RemoveBuffMap.push_back(itr->first);
	}
	m_buffLock.unlock();
	foreach(itr, RemoveBuffMap)
		CMagicProcess::RemoveType4Buff(*itr, this, bRemoveSavedMagic, buffType > 0 ? true : false);
}

/**
* @brief	Determine if this unit is basically able to attack the specified unit.
* 			This should only be called to handle the minimal shared logic between
* 			NPCs and players.
*
* 			You should use the more appropriate CUser or CNpc specialization.
*
* @param	pTarget	Target for the attack.
*
* @return	true if we can attack, false if not.
*/
bool Unit::CanAttack(Unit* pTarget)
{
	if (pTarget == nullptr)
		return false;

	// Units cannot attack units in different event room.
	if (GetEventRoom() != pTarget->GetEventRoom())
		return false;

	// Units cannot attack units in different zones.
	if (GetZoneID() != pTarget->GetZoneID())
		return false;

	// We cannot attack our target if we are incapacitated 
	// (should include debuffs & being blinded)
	if (isIncapacitated()
		// or if our target is in a state in which
			// they should not be allowed to be attacked
		|| pTarget->isDead()
		|| pTarget->isBlinking())
		return false;

	// Finally, we can only attack the target if we are hostile towards them.
	return isHostileTo(pTarget);
}

/**
* @brief	Determine if this unit is basically able to attack the specified unit.
* 			This should only be called to handle the minimal shared logic between
* 			NPCs and players.
*
* 			You should use the more appropriate CUser or CNpc specialization.
*
* @param	pTarget	Target for the attack.
*
* @return	true if we attackable, false if not.
*/
bool Unit::isAttackable(Unit* pTarget)
{
	if (pTarget == nullptr)
		pTarget = this;

	if (pTarget)
	{
		if (pTarget->isNPC())
		{
			CNpc* pNpc = TO_NPC(pTarget);
			if (pNpc != nullptr)
			{

				if (pNpc->GetType() == NPC_BIFROST_MONUMENT)
				{
					if (!g_pMain->pBeefEvent.isActive || g_pMain->pBeefEvent.isMonumentDead)
						return false;
				}
				else if (pNpc->GetType() == NPC_PVP_MONUMENT)
				{
					if ((GetNation() == (uint8)Nation::KARUS && pNpc->GetPID() == MONUMENT_KARUS_SPID)
						|| (GetNation() == (uint8)Nation::ELMORAD && pNpc->GetPID() == MONUMENT_ELMORAD_SPID))
						return false;
				}
				//Tarih : 14.05.2020 ##START##
				else if (pNpc->GetType() == NPC_CLAN_WAR_MONUMENT)
				{
					if ((GetNation() == (uint8)Nation::KARUS && pNpc->GetPID() == CLAN_MONUMENT_SPID)
						|| (GetNation() == (uint8)Nation::ELMORAD && pNpc->GetPID() == CLAN_MONUMENT_SPID))
						return false;
				}
				//Tarih : 14.05.2020 ##END##
				else if (pNpc->GetType() == NPC_GUARD_TOWER1
					|| pNpc->GetType() == NPC_GUARD_TOWER2
					|| pNpc->GetType() == NPC_GATE2
					|| pNpc->GetType() == NPC_VICTORY_GATE
					|| pNpc->GetType() == NPC_PHOENIX_GATE
					|| pNpc->GetType() == NPC_SPECIAL_GATE
					|| pNpc->GetType() == NPC_GATE_LEVER)
					return false;
				else if (TO_NPC(pTarget)->m_OrgNation == 3) // Atack Kapatıldı 27.09.2020 
				{
					//printf("ByGroup 3 Npc Atack Blocked\n");
					return false;
				}
			}
		}
	}

	return true;
}

void Unit::OnDeath(Unit* pKiller)
{
	SendDeathAnimation(pKiller);
}

void Unit::SendDeathAnimation(Unit* pKiller /*= nullptr*/)
{
	Packet result(WIZ_DEAD);
	result << GetID();
	SendToRegion(&result);
}

void Unit::AddType4Buff(uint8 bBuffType, _BUFF_TYPE4_INFO& pBuffInfo)
{
	Guard lock(m_buffLock);
	m_buffMap.insert(std::make_pair(bBuffType, pBuffInfo));
	if (pBuffInfo.isBuff()) m_buffCount++;
}

/**************************************************************************
* The following methods should not be here, but it's necessary to avoid
* code duplication between AI and GameServer until they're better merged.
**************************************************************************/
/**
* @brief	Sets zone attributes for the loaded zone.
*
* @param	zoneNumber	The zone number.
*/
void KOMap::SetZoneAttributes(_ZONE_INFO* pZone)
{
	m_zoneFlags = 0;
	m_byTariff = 10; // defaults to 10 officially for zones that don't use it.
	m_byMinLevel = pZone->m_MinLevel;
	m_byMaxLevel = pZone->m_MaxLevel;
	m_zoneType = (ZoneAbilityType)pZone->m_ZoneType;

	if (pZone->m_kTradeOtherNation)
		m_zoneFlags |= ZF_TRADE_OTHER_NATION;
	if (pZone->m_kTalkOtherNation)
		m_zoneFlags |= ZF_TALK_OTHER_NATION;
	if (pZone->m_kAttackOtherNation)
		m_zoneFlags |= ZF_ATTACK_OTHER_NATION;
	if (pZone->m_kAttackSameNation)
		m_zoneFlags |= ZF_ATTACK_SAME_NATION;
	if (pZone->m_kFriendlyNpc)
		m_zoneFlags |= ZF_FRIENDLY_NPCS;
	if (pZone->m_kWarZone)
		m_zoneFlags |= ZF_WAR_ZONE;
	if (pZone->m_kClanUpdates)
		m_zoneFlags |= ZF_CLAN_UPDATE;

	m_bBlink = pZone->m_bBlink;
	m_bTeleport = pZone->m_bTeleport;
	m_bGate = pZone->m_bGate;
	m_bEscape = pZone->m_bEscape;
	m_bCallingFriend = pZone->m_bCallingFriend;
	m_bTeleportFriend = pZone->m_bTeleportFriend;
	m_bPetSpawn = pZone->m_bPetSpawn;
	m_bExpLost = pZone->m_bExpLost;
	m_bGiveLoyalty = pZone->m_bGiveLoyalty;
	m_bGuardSummon = pZone->m_bGuardSummon;
	m_bMenissiahList = pZone->m_bMenissiahList;
	m_bMilitaryZone = pZone->m_bMilitaryZone;
	m_bMiningZone = pZone->m_bMiningZone;
	m_bBlinkZone = pZone->m_bBlinkZone;
	m_bAutoLoot = pZone->m_bAutoLoot;
	m_bGoldLose = pZone->m_bGoldLose;
}

void KOMap::UpdateDelosDuringCSW(bool Open, ZoneAbilityType type)
{
	if (Open)
	{
		m_zoneType = type;
		m_zoneFlags = ZF_ATTACK_OTHER_NATION | ZF_ATTACK_SAME_NATION | ZF_FRIENDLY_NPCS;
	}
	else
	{
		m_zoneType = ZoneAbilityType::ZoneAbilitySiegeDisabled;
		m_zoneFlags = ZF_TRADE_OTHER_NATION | ZF_TALK_OTHER_NATION | ZF_FRIENDLY_NPCS;
	}
}

/**
* @brief	Determines if an NPC is hostile to a unit.
* 			Non-hostile units cannot be attacked.
*
* @param	pTarget	Target unit
*
* @return	true if hostile to, false if not.
*/
bool CNpc::isHostileTo(Unit* pTarget)
{
	if (pTarget == nullptr)
		return false;

	// Units cannot attack units in different event room.
	if (GetEventRoom() != pTarget->GetEventRoom())
		return false;

	// Only players can attack these targets.
	if (pTarget->isPlayer())
	{
		// Scarecrows are NPCs that the client allows us to attack
		// however, since they're not a monster, and all NPCs in neutral zones
		// are friendly, we need to override to ensure we can attack them server-side.

		switch (GetType())
		{
		case NPC_SCARECROW:
			return true;
			break;
		case NPC_BIFROST_MONUMENT:
		{
			if (g_pMain->pBeefEvent.isActive && !g_pMain->pBeefEvent.isMonumentDead)
				return true;
		}
		break;
		case NPC_BORDER_MONUMENT:
		{
			if (g_pMain->isBorderDefenceWarActive() && isInTempleEventZone())
				return true;
		}
		break;
		}

		if (isGuardSummon())
		{

			if (isInMoradon())
			{
				if (TO_USER(pTarget)->isInPartyArena())
				{
					CUser* pUser = g_pMain->GetUserPtr(GetGuardID());
					if (pUser == nullptr) return false;

					if (pUser->isInParty() && pTarget->isPlayer()
						&& pUser->GetPartyID() == TO_USER(pTarget)->GetPartyID())
						return false;
					else if (pUser->GetName() == TO_USER(pTarget)->GetName())
						return false;
					else
						return true;
				}
				else if (TO_USER(pTarget)->isInMeleeArena())
				{
					CUser* pUser = g_pMain->GetUserPtr(GetGuardID());
					if (pUser == nullptr) return false;

					if (pUser->isInParty() && pTarget->isPlayer()
						&& pUser->GetPartyID() == TO_USER(pTarget)->GetPartyID())
						return false;
					else if (pUser->GetName() == TO_USER(pTarget)->GetName())
						return false;
					else
						return true;
				}
				else
					return false;
			}
			else
			{
				if (TO_USER(pTarget)->isInEnemySafetyArea() || GetNation() == pTarget->GetNation())
					return false;

				CUser* pUser = g_pMain->GetUserPtr(GetGuardID());
				if (pUser == nullptr) return false;
				if (pUser->isInParty() && pTarget->isPlayer()
					&& pUser->GetPartyID() == TO_USER(pTarget)->GetPartyID())
					return false;
				else if (pUser->GetName() == TO_USER(pTarget)->GetName())
					return false;
				else
					return true;
			}
		}
	}

	if (GetZoneID() == ZONE_DELOS && pTarget->isPlayer())
	{
		if (GetType() == NPC_DESTROYED_ARTIFACT || isCswDoors())
		{
			if (!TO_USER(pTarget)->isInClan() || !g_pMain->isCswActive() || !g_pMain->isCswWarActive())
				return false;

			if (TO_USER(pTarget)->GetClanID() == g_pMain->pSiegeWar.sMasterKnights)
				return false;
		}
		return true;
	}

	if (g_pMain->m_byBattleOpen == NATION_BATTLE
		&& GetMap() && GetMap()->isWarZone())
	{
		if (GetProtoID() > 1100 && GetProtoID() <= 2203
			|| GetProtoID() > 2301 && GetProtoID() <= 2306)
			return true;
	}

	if (pTarget->isNPC())
	{
		// Scarecrows are NPCs that the client allows us to attack
		// however, since they're not a monster, and all NPCs in neutral zones
		// are friendly, we need to override to ensure we can attack them server-side.

		switch (TO_NPC(pTarget)->GetType())
		{
		case NPC_SCARECROW:
			return false;
			break;
		case NPC_BIFROST_MONUMENT:
			return false;
			break;
		case NPC_BORDER_MONUMENT:
			return false;
			break;
		case NPC_OBJECT_FLAG:
			CNpc* pNpc = g_pMain->GetPetPtr(TO_NPC(pTarget)->GetPetID(), TO_NPC(pTarget)->GetZoneID());
			if (pNpc != nullptr)
			{
				switch (GetType())
				{
				case NPC_GUARD:
					return false;
					break;
				case NPC_PATROL_GUARD:
					return false;
					break;
				case NPC_STORE_GUARD:
					return false;
					break;
				default:
					break;
				}

				if (!pNpc->isDead())
					return true;
			}
			break;
		}

		if (isGuardSummon())
		{
			if (TO_NPC(pTarget)->isMonster()) return true;
			else return false;
		}

		if (TO_NPC(pTarget)->isGuardSummon())
			return false;

		if (isMonster())
		{
			if (TO_NPC(pTarget)->isMonster())
				return false;
		}
	}

	if (GetNation() == (uint8)Nation::ALL)
		return false;

	if (pTarget->isPlayer() && isGuard() && isInMoradon())
		return false;

	if (!isMonster() && GetMap() && GetMap()->areNPCsFriendly())
	{
		if (isGuard() && TO_NPC(pTarget)->isMonster())
			return true;

		return false;
	}

	/*// A nation of 0 indicates friendliness to all
	if (GetNation() == Nation::ALL
		// Also allow for cases when all NPCs in this zone are inherently friendly.
		|| (!isMonster() && GetMap()->areNPCsFriendly()))
		return false;*/

		// A nation of 3 indicates hostility to all (or friendliness to none)
	if (GetNation() == (uint8)Nation::NONE && pTarget->GetNation() != (uint8)Nation::NONE)
		return true;

	if (GetMap()->isWarZone()
		&& g_pMain->m_bResultDelay == true
		&& g_pMain->m_bResultDelayVictory != pTarget->GetNation()
		&& GetNation() != pTarget->GetNation())
		return false;

	// An NPC cannot attack a unit of the same nation
	return (GetNation() != pTarget->GetNation());
}

bool CNpc::isMoral2Checking(Unit* pTarget, _MAGIC_TABLE pSkill)
{
	if (pTarget == nullptr || pSkill.isnull()
		|| GetEventRoom() != pTarget->GetEventRoom()
		|| !isMonster())
		return true;

	if (pSkill.bMoral == MORAL_FRIEND_WITHME)
	{
		if (TO_USER(pTarget)->isPriest())
		{
			if (pSkill.iNum == 111554 ||
				pSkill.iNum == 112554 ||
				pSkill.iNum == 211554 ||
				pSkill.iNum == 212554)
			{
				_ITEM_TABLE pTable = TO_USER(pTarget)->GetItemPrototype(RIGHTHAND);
				if (!pTable.isnull() && pTable.isDarkKnightMace())
					return false;
			}
		}
		if (isHealer())
			return false;
	}

	return true;
}

bool CUser::isMoral2Checking(Unit* pTarget, _MAGIC_TABLE pSkill)
{
	if (pTarget == nullptr || pSkill.isnull()
		|| GetEventRoom() != pTarget->GetEventRoom())
		return true;

	// For non-player hostility checks, refer to the appropriate override.
	if (!pTarget->isPlayer())
		return pTarget->isMoral2Checking(this, pSkill);

	_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND);
	if (isPriest() && isDKMToUserDamageSkills(pSkill.iNum) && !pTable.isnull() && pTable.isDarkKnightMace())
	{
		if (isInArena() && TO_USER(pTarget)->isInArena())
		{
			if (isInMeleeArena() && TO_USER(pTarget)->isInMeleeArena())
			{
				return false;
			}
			else if (isInPartyArena() && TO_USER(pTarget)->isInPartyArena())
			{
				if (isInParty() && GetPartyID() != TO_USER(pTarget)->GetPartyID())
					return false;

				if (!isInParty())
					return false;
			}
			else if (GetZoneID() == ZONE_ARENA)
			{
				if (isInClanArena() && TO_USER(pTarget)->isInClanArena())
				{
					if (isInClan() && TO_USER(pTarget)->isInClan())
						return false;
				}
				if (!isInEnemySafetyArea() && !TO_USER(pTarget)->isInEnemySafetyArea())
					return false;
			}
			return true;
		}

		if (GetZoneID() == ZONE_DELOS)
		{
			if (g_pMain->m_byBattleSiegeWarOpen && g_pMain->m_byBattleSiegeWarAttack)
			{
				if (isInOwnSafetyArea())
					return true;

				CKnights* pClan1 = g_pMain->GetClanPtr(GetClanID());
				CKnights* pClan2 = g_pMain->GetClanPtr(TO_USER(pTarget)->GetClanID());
				if (pClan1 == nullptr || pClan2 == nullptr)
					return true;

				std::vector<std::string> pKnights1, pKnights2;
				pClan1->m_arKnightsUser.m_lock.lock();
				foreach_stlmap_nolock(itr, pClan1->m_arKnightsUser)
				{
					auto* KnightUser = itr->second;
					if (KnightUser == nullptr)
						continue;

					pKnights1.push_back(KnightUser->strUserName);
				}
				pClan1->m_arKnightsUser.m_lock.unlock();

				foreach(itr, pKnights1)
				{
					auto* pClanMember = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
					if (pClanMember == nullptr)
						continue;

					if (pClanMember->GetZoneID() == TO_USER(pTarget)->GetZoneID()
						&& pClanMember->GetClanID() != TO_USER(pTarget)->GetClanID()
						&& !pClanMember->isStoreOpen()
						&& !pClanMember->isMerchanting()
						&& !pClanMember->isTrading()
						&& !CKnightsManager::CheckAlliance(pClan1, pClan2))
						return false;
				}

				pClan2->m_arKnightsUser.m_lock.lock();
				foreach_stlmap_nolock(itr, pClan2->m_arKnightsUser)
				{
					auto* KnightUser = itr->second;
					if (KnightUser == nullptr)
						continue;

					pKnights2.push_back(KnightUser->strUserName);
				}
				pClan2->m_arKnightsUser.m_lock.unlock();

				foreach(itr, pKnights2)
				{
					auto* pTargetClanMember = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
					if (pTargetClanMember == nullptr)
						continue;

					if (pTargetClanMember->GetZoneID() == GetZoneID()
						&& pTargetClanMember->GetClanID() != GetClanID()
						&& !pTargetClanMember->isStoreOpen()
						&& !pTargetClanMember->isMerchanting()
						&& !pTargetClanMember->isTrading()
						&& !CKnightsManager::CheckAlliance(pClan1, pClan2))
						return false;
				}
			}
		}

		if (GetNation() != pTarget->GetNation())
		{
			if (isInPVPZone())
				return false;

			if ((GetZoneID() == ZONE_DESPERATION_ABYSS
				|| GetZoneID() == ZONE_HELL_ABYSS
				|| GetZoneID() == ZONE_DRAGON_CAVE))
				return false;
		}
		return true;
	}

	if (!isInArena() && GetZoneID() != ZONE_DELOS)
	{
		if (GetNation() != pTarget->GetNation()
			&& TO_USER(pTarget)->isInOwnSafetyArea())
			return true;
	}

	// Players can attack other players in the Moradon arena.
	if (isInArena() && TO_USER(pTarget)->isInArena())
	{
		if (isInPartyArena())
		{
			if (isInParty() && GetPartyID() == TO_USER(pTarget)->GetPartyID())
				return false;
			else
				return true;
		}
		else if (isInMeleeArena() && TO_USER(pTarget)->isInMeleeArena())
			return true;

		// Players can attack other players in the arena.
		if (GetZoneID() == ZONE_ARENA)
		{
			if (isInClanArena() && TO_USER(pTarget)->isInClanArena())
			{
				if (isInClan() && TO_USER(pTarget)->isInClan()
					&& GetClanID() == TO_USER(pTarget)->GetClanID())
					return false;
			}

			if (isInEnemySafetyArea() && TO_USER(pTarget)->isInEnemySafetyArea())
				return false;

			return true;
		}
	}

	if (!isInArena() && GetZoneID() != ZONE_DELOS)
	{
		if (GetNation() != pTarget->GetNation()
			&& TO_USER(pTarget)->isInOwnSafetyArea())
			return true;
	}

	if (GetZoneID() == ZONE_DELOS)
	{
		CKnights* pKnightsSource = g_pMain->GetClanPtr(GetClanID());
		CKnights* pKnightsTarget = g_pMain->GetClanPtr(TO_USER(pTarget)->GetClanID());
		if (pKnightsSource != nullptr && pKnightsTarget != nullptr)
		{
			if (pKnightsSource->GetID() == pKnightsTarget->GetID())
				return false;
		}
		return true;
	}

	if (GetNation() != pTarget->GetNation())
	{
		if (isInPVPZone())
			return true;

		if ((GetZoneID() == ZONE_DESPERATION_ABYSS
			|| GetZoneID() == ZONE_HELL_ABYSS
			|| GetZoneID() == ZONE_DRAGON_CAVE))
			return true;
	}
	return false;
}

bool CBot::isMoral2Checking(Unit* pTarget, _MAGIC_TABLE pSkill)
{
	return true;
}

bool CUser::isDKMToMonsterDamageSkills(uint32 iSkillID)
{
	if (iSkillID == 111554 ||
		iSkillID == 112554 ||
		iSkillID == 211554 ||
		iSkillID == 212554)
		return true;

	return false;
}


bool CUser::isDKMToUserDamageSkills(uint32 iSkillID)
{
	if (iSkillID == 111545 ||
		iSkillID == 112545 ||
		iSkillID == 211545 ||
		iSkillID == 212545)
		return true;

	return false;
}

bool CUser::isHealDamageChecking(Unit* pTarget, _MAGIC_TABLE pSkill)
{
	if (pTarget == nullptr || pSkill.isnull())
		return false;

	_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND);
	if (!pTable.isnull() && pTable.isDarkKnightMace() && this != pTarget)
	{
		if (pTarget->isNPC() && isDKMToMonsterDamageSkills(pSkill.iNum))
		{
			if (TO_NPC(pTarget)->isMonster())
			{
				return true;
			}
		}
		else if (pTarget->isPlayer() && isDKMToUserDamageSkills(pSkill.iNum))
		{
			if (isInArena() && TO_USER(pTarget)->isInArena())
			{
				if ((isInMeleeArena() && TO_USER(pTarget)->isInMeleeArena())
					|| (isInPartyArena() && TO_USER(pTarget)->isInPartyArena()))
				{
					if (isInParty() && GetPartyID() != TO_USER(pTarget)->GetPartyID())
					{
						return true;
					}
				}

				if (!isInParty())
				{
					return true;
				}

				if (GetZoneID() == ZONE_ARENA && !isInEnemySafetyArea())
				{
					if (!isInEnemySafetyArea())
					{
						if (isInClanArena() && TO_USER(pTarget)->isInClanArena())
						{
							if (isInClan() && TO_USER(pTarget)->isInClan()
								&& GetClanID() != TO_USER(pTarget)->GetClanID())
							{
								return true;
							}
						}
						else if (!TO_USER(pTarget)->isInEnemySafetyArea() && !TO_USER(pTarget)->isInClanArena())
						{
							return true;
						}
					}
				}

				if (GetZoneID() == ZONE_DELOS && g_pMain->m_byBattleSiegeWarOpen && g_pMain->m_byBattleSiegeWarAttack)
				{
					if (isInOwnSafetyArea())
						return false;

					if (isInClan())
					{
						CKnights* pClan1 = g_pMain->GetClanPtr(GetClanID());
						CKnights* pClan2 = g_pMain->GetClanPtr(TO_USER(pTarget)->GetClanID());
						if (pClan1 == nullptr || pClan2 == nullptr)
							return true;

						if (!CKnightsManager::CheckAlliance(pClan1, pClan2) || GetClanID() != TO_USER(pTarget)->GetClanID())
							return true;
					}
					else
					{
						return true;
					}
				}
			}
		}

		if (GetNation() != pTarget->GetNation())
		{
			if (isInPVPZone())
				return true;

			if ((GetZoneID() == ZONE_DESPERATION_ABYSS
				|| GetZoneID() == ZONE_HELL_ABYSS
				|| GetZoneID() == ZONE_DRAGON_CAVE))
				return true;
		}
	}
	return false;
}

bool CUser::isInClanArena()
{
	if (GetZoneID() != ZONE_ARENA)
		return false;

	return (isInRangeSlow(64.0f, 178.0f, 60)
		|| isInRangeSlow(192.0f, 178.0f, 60));
}

/**
* @brief	Determines if a player is hostile to a unit.
* 			Non-hostile units cannot be attacked.
*
* @param	pTarget	Target unit
*
* @return	true if hostile to, false if not.
*/
bool CUser::isHostileTo(Unit* pTarget)
{
	if (pTarget == nullptr)
		return false;

	// Units cannot attack units in different event room.
	if (GetEventRoom() != pTarget->GetEventRoom())
		return false;

	// For non-player hostility checks, refer to the appropriate override.
	if (!pTarget->isPlayer())
		return pTarget->isHostileTo(this);

	// Players can attack other players in the Moradon arena.
	if (isInArena() && TO_USER(pTarget)->isInArena())
	{
		if ((GetX() < 735.0f && GetX() > 684.0f)
			&& (GetZ() < 411.0f && GetZ() > 360.0f)) // party arena in moradon
		{
			if (isInParty() && pTarget->isPlayer()
				&& GetPartyID() == TO_USER(pTarget)->GetPartyID())
				return false;
			else
				return true;
		}
		else if ((GetX() < 735.0f && GetX() > 684.0f)
			&& (GetZ() < 491.0f && GetZ() > 440.0f)) // melee arena in moradon
		{
			if (GetName() == pTarget->GetName())
				return false;
			else
				return true;
		}
	}

	// Players can attack other players in the arena.
	if (GetZoneID() == ZONE_ARENA)
	{
		if (isInRangeSlow(64.0f, 178.0f, 60)
			|| isInRangeSlow(192.0f, 178.0f, 60)) // we are at the rose clan arena
		{
			if (!isInClan() || !TO_USER(pTarget)->isInClan())
				return false;

			if (isInClan() && TO_USER(pTarget)->isInClan()
				&& GetClanID() == TO_USER(pTarget)->GetClanID())
				return false;
		}

		if (isInEnemySafetyArea())
			return false;

		return true;
	}

	if (GetZoneID() == ZONE_STAR_NEO_VS)
	{
		if (!m_VsDurumu)
			return true;
	}

	// Players can attack other players in the safety area.
	if (GetNation() != pTarget->GetNation()
		&& TO_USER(pTarget)->isInOwnSafetyArea())
		return false;

	// Players can attack opposing nation players when they're in PVP zones.
	if (GetNation() != pTarget->GetNation()
		&& isInPVPZone())
		return true;

	// JstKO: Battle/war maps must behave like normal R attack for every skill check.
	// Without this, R attack may pass through the battle attack flow, but skill moral/hostile
	// checks reject GM/user/bot targets in ZONE_BATTLE..ZONE_BATTLE6.
	if (GetNation() != pTarget->GetNation()
		&& GetMap() != nullptr
		&& pTarget->GetMap() != nullptr
		&& GetMap()->isWarZone()
		&& pTarget->GetMap()->isWarZone())
		return true;

	// Players can attack opposing nation players when they're in PVP zones.
	if (GetNation() != pTarget->GetNation()
		&& (GetZoneID() == ZONE_DESPERATION_ABYSS
			|| GetZoneID() == ZONE_HELL_ABYSS
			|| GetZoneID() == ZONE_DRAGON_CAVE))
		return true;

	if (ChaosTempleEventZone())
		return true;

	if (GetZoneID() == ZONE_DELOS && pTarget->isPlayer())
	{
		CswOpStatus cswstatus = g_pMain->pCswEvent.Status;
		if (cswstatus != CswOpStatus::War || GetClanID() == TO_USER(pTarget)->GetClanID())
			return false;

		if (!g_pMain->isCswActive() || !isInClan() || !TO_USER(pTarget)->isInClan())
			return false;

		if (isInOwnSafetyArea() || TO_USER(pTarget)->isInOwnSafetyArea())
			return false;

		return true;
	}

	// Players can attack opposing nation players during wars.
	if (GetNation() != pTarget->GetNation()
		&& (isInElmoradCastle() || isInLufersonCastle())
		&& (g_pMain->m_byElmoradOpenFlag || g_pMain->m_byKarusOpenFlag))
		return true;

	if (g_pMain->isCindirellaZone(GetZoneID()))
	{
		if (!g_pMain->pCindWar.isStarted() || !pTarget->isPlayer() || GetNation() == pTarget->GetNation())
			return false;

		if (GetNation() != pTarget->GetNation() && pCindWar.isEventUser() && TO_USER(pTarget)->pCindWar.isEventUser())
			return true;
	}

	if (isGM())
	{
		// Players can attack opposing nation players during wars.
		if (GetNation() != pTarget->GetNation()
			&& (isInElmoradCastle() || isInLufersonCastle()))
			return true;
	}

	// Players cannot attack other players in any other circumstance.
	return false;
}

bool CBot::isHostileTo(Unit* pTarget)
{
	if (pTarget == nullptr)
		return false;

	// Units cannot attack units in different event room.
	if (GetEventRoom() != pTarget->GetEventRoom())
		return false;

	// For non-player hostility checks, refer to the appropriate override.
	if (pTarget->isNPC())
		return pTarget->isHostileTo(this);

	// Players can attack other players in the safety area.
	if (GetNation() != pTarget->GetNation()
		&& pTarget->isPlayer()
		&& TO_USER(pTarget)->isInOwnSafetyArea())
		return false;

	// Players can attack opposing nation players when they're in PVP zones.
	if (GetNation() != pTarget->GetNation()
		&& isInPKZone())
		return true;

	// JstKO: Battle/war maps must behave like normal R attack for every skill check.
	// This allows PK bots / GM bots / normal users to use all attack skills in war zones
	// with the same enemy-nation rule used by R attacks.
	if (GetNation() != pTarget->GetNation()
		&& GetMap() != nullptr
		&& pTarget->GetMap() != nullptr
		&& GetMap()->isWarZone()
		&& pTarget->GetMap()->isWarZone())
		return true;

	// Players can attack opposing nation players when they're in PVP zones.
	if (GetNation() != pTarget->GetNation()
		&& (GetZoneID() == ZONE_DESPERATION_ABYSS
			|| GetZoneID() == ZONE_HELL_ABYSS
			|| GetZoneID() == ZONE_DRAGON_CAVE))
		return true;

	if (GetZoneID() == ZONE_DELOS && pTarget->isPlayer())
	{
		CswOpStatus cswstatus = g_pMain->pCswEvent.Status;
		if (cswstatus != CswOpStatus::War || GetClanID() == TO_USER(pTarget)->GetClanID())
			return false;

		if (!g_pMain->isCswActive() || !isInClan() || !TO_USER(pTarget)->isInClan())
			return false;

		if (isInOwnSafetyArea() || TO_USER(pTarget)->isInOwnSafetyArea())
			return false;

		return true;
	}

	if (GetZoneID() == ZONE_DELOS && pTarget->isBot())
	{
		CswOpStatus cswstatus = g_pMain->pCswEvent.Status;
		if (cswstatus != CswOpStatus::War || GetClanID() == TO_BOT(pTarget)->GetClanID())
			return false;

		if (!g_pMain->isCswActive() || !isInClan() || !TO_BOT(pTarget)->isInClan())
			return false;

		if (isInOwnSafetyArea() || TO_BOT(pTarget)->isInOwnSafetyArea())
			return false;

		return true;
	}

	if (ChaosTempleEventZone())
		return true;

	// Players can attack opposing nation players during wars.
	if (GetNation() != pTarget->GetNation()
		&& (isInElmoradCastle() || isInLufersonCastle())
		&& (g_pMain->m_byElmoradOpenFlag || g_pMain->m_byKarusOpenFlag))
		return true;

	// Players cannot attack other players in any other circumstance.
	return false;
}

/**
* @brief	Determine if this user is in an arena area.
*
* @return	true if in arena, false if not.
*/
bool Unit::isInArena()
{
	/*
	All of this needs to be handled more generically
	(i.e. bounds loaded from the database, or their existing SMD method).
	*/

	// If we're in the Arena zone, assume combat is acceptable everywhere.
	// NOTE: This is why we need generic bounds checks, to ensure even attacks in the Arena zone are in one of the 4 arena locations.
	if (GetZoneID() == ZONE_ARENA)
		return true;

	bool bIsNeutralZone = (GetZoneID() >= ZONE_MORADON && GetZoneID() <= ZONE_MORADON5);

	// The only other arena is located in Moradon. If we're not in Moradon, then it's not an Arena.
	if (!bIsNeutralZone)
		return false;

	// Moradon outside arena spawn bounds.
	return ((GetX() < 735.0f && GetX() > 684.0f)
		&& ((GetZ() < 491.0f && GetZ() > 440.0f)
			|| (GetZ() < 411.0f && GetZ() > 360.0f)));
}

/**
* @brief	Determine if this user is in a normal PVP zone.
* 			That is, they're in an PK zone that allows combat
* 			against the opposite nation.
*
* @return	true if in PVP zone, false if not.
*/
bool Unit::isInPVPZone()
{
	if (GetZoneID() == ZONE_RONARK_LAND
		|| GetZoneID() == ZONE_RONARK_LAND_BASE
		|| GetZoneID() == ZONE_ARDREAM
		|| GetZoneID() == ZONE_SNOW_BATTLE
		|| GetZoneID() == ZONE_BATTLE
		|| GetZoneID() == ZONE_BATTLE2
		|| GetZoneID() == ZONE_BATTLE3
		|| GetZoneID() == ZONE_BATTLE4
		|| GetZoneID() == ZONE_BATTLE5
		|| GetZoneID() == ZONE_BATTLE6
		|| GetZoneID() == ZONE_JURAID_MOUNTAIN
		|| GetZoneID() == ZONE_BORDER_DEFENSE_WAR
		|| GetZoneID() == ZONE_CLAN_WAR_ARDREAM
		|| GetZoneID() == ZONE_CLAN_WAR_RONARK
		|| GetZoneID() == ZONE_BIFROST
		|| GetZoneID() == ZONE_PARTY_VS_1
		|| GetZoneID() == ZONE_PARTY_VS_2
		|| GetZoneID() == ZONE_PARTY_VS_3
		|| GetZoneID() == ZONE_PARTY_VS_4
		|| isInSpecialEventZone())
		return true;
	else
		return false;
}

bool CUser::isPriestUseZone()
{
	if (GetZoneID() == ZONE_RONARK_LAND
		|| GetZoneID() == ZONE_RONARK_LAND_BASE
		|| GetZoneID() == ZONE_ARDREAM
		|| GetZoneID() == ZONE_SNOW_BATTLE
		|| GetZoneID() == ZONE_BATTLE
		|| GetZoneID() == ZONE_BATTLE2
		|| GetZoneID() == ZONE_BATTLE3
		|| GetZoneID() == ZONE_BATTLE4
		|| GetZoneID() == ZONE_BATTLE5
		|| GetZoneID() == ZONE_BATTLE6
		|| GetZoneID() == ZONE_JURAID_MOUNTAIN
		|| GetZoneID() == ZONE_BORDER_DEFENSE_WAR
		|| GetZoneID() == ZONE_CLAN_WAR_ARDREAM
		|| GetZoneID() == ZONE_CLAN_WAR_RONARK
		|| GetZoneID() == ZONE_PARTY_VS_1
		|| GetZoneID() == ZONE_PARTY_VS_2
		|| GetZoneID() == ZONE_PARTY_VS_3
		|| GetZoneID() == ZONE_PARTY_VS_4
		|| GetZoneID() == ZONE_BIFROST
		|| GetZoneID() == ZONE_KROWAZ_DOMINION
		|| GetZoneID() == ZONE_ARENA
		|| GetZoneID() == ZONE_DELOS
		|| GetZoneID() == ZONE_DELOS_CASTELLAN
		|| GetZoneID() == ZONE_DESPERATION_ABYSS
		|| GetZoneID() == ZONE_HELL_ABYSS
		|| GetZoneID() == ZONE_DRAGON_CAVE
		|| GetZoneID() == ZONE_ORC_ARENA
		|| GetZoneID() == ZONE_BLOOD_DON_ARENA
		|| GetZoneID() == ZONE_GOBLIN_ARENA
		|| GetZoneID() == ZONE_CAITHAROS_ARENA
		|| GetZoneID() == ZONE_FORGOTTEN_TEMPLE
		|| GetZoneID() == ZONE_LOST_TEMPLE
		|| GetZoneID() == ZONE_STONE1
		|| GetZoneID() == ZONE_STONE2
		|| GetZoneID() == ZONE_STONE3
		|| GetZoneID() == ZONE_UNDER_CASTLE
		|| GetZoneID() == ZONE_ISILOON_ARENA
		|| GetZoneID() == ZONE_FELANKOR_ARENA
		|| GetZoneID() == ZONE_DUNGEON_DEFENCE)
		return true;
	else
		return false;
}


/**
* @brief	Determine if this user is in an safety area.
*
* @return	true if in safety area, false if not.
*/
bool Unit::isInEnemySafetyArea()
{
	switch (GetZoneID())
	{
	case ZONE_DELOS:
		return isInRangeSlow(500.0f, 180.0f, 115);

	case ZONE_BIFROST:
		if (GetNation() == (uint8)Nation::ELMORAD)
			return ((GetX() < 124.0f && GetX() > 56.0f) && ((GetZ() < 840.0f && GetZ() > 700.0f)));
		else
			return ((GetX() < 270.0f && GetX() > 190.0f) && ((GetZ() < 970.0f && GetZ() > 870.0f)));

	case ZONE_ARENA:
		return isInRangeSlow(127.0f, 113.0f, 36);

	case ZONE_ELMORAD:
	case ZONE_ELMORAD2:
	case ZONE_ELMORAD3:
		if (GetNation() == (uint8)Nation::ELMORAD) // 210 , 1853 emc
			return isInRangeSlow(float(210.0f), float(1853.0f), 50);// (GetX() < 244.0f && GetX() > 176.0f) && ((GetZ() < 1880.0f && GetZ() > 1820.0f)));
		break;

	case ZONE_KARUS:
	case ZONE_KARUS2:
	case ZONE_KARUS3:
		if (GetNation() == (uint8)Nation::KARUS)
			return isInRangeSlow(float(1860.0f), float(174.0f), 50);
		break;

	case ZONE_BATTLE:
		if (GetNation() == (uint8)Nation::KARUS)
			return ((GetX() < 125.0f && GetX() > 98.0f) && ((GetZ() < 780.0f && GetZ() > 755.0f)));
		else if (GetNation() == (uint8)Nation::ELMORAD)
			return ((GetX() < 831.0f && GetX() > 805.0f) && ((GetZ() < 110.0f && GetZ() > 85.0f)));

	case ZONE_BATTLE2:
		if (GetNation() == (uint8)Nation::KARUS)
			return ((GetX() < 977.0f && GetX() > 942.0f) && ((GetZ() < 904.0f && GetZ() > 863.0f)));
		else if (GetNation() == (uint8)Nation::ELMORAD)
			return ((GetX() < 80.0f && GetX() > 46.0f) && ((GetZ() < 174.0f && GetZ() > 142.0f)));

	case ZONE_BATTLE4:
		if (GetNation() == (uint8)Nation::KARUS)
			return isInRangeSlow(float(235.0f), float(228.0f), 80)
			|| isInRangeSlow(float(846.0f), float(362.0f), 20)
			|| isInRangeSlow(float(338.0f), float(807.0f), 20);
		else if (GetNation() == (uint8)Nation::ELMORAD)
			return isInRangeSlow(float(809.0f), float(783.0f), 80)
			|| isInRangeSlow(float(182.0f), float(668.0f), 20)
			|| isInRangeSlow(float(670.0f), float(202.0f), 20);
	}
	return false;
}

/**
* @brief	Determine if this user is in an safety area.
*
* @return	true if in safety area, false if not.
*/
bool Unit::isInOwnSafetyArea()
{
	switch (GetZoneID())
	{
	case ZONE_DELOS:
		return isInRangeSlow(500.0f, 180.0f, 115);

	case ZONE_BIFROST:
		if (GetNation() == (uint8)Nation::KARUS)
			return ((GetX() < 124.0f && GetX() > 56.0f) && ((GetZ() < 840.0f && GetZ() > 700.0f)));
		else
			return ((GetX() < 270.0f && GetX() > 190.0f) && ((GetZ() < 970.0f && GetZ() > 870.0f)));

	case ZONE_ARENA:
		return isInRangeSlow(127.0f, 113.0f, 36);

	case ZONE_ELMORAD:
	case ZONE_ELMORAD2:
	case ZONE_ELMORAD3:
		if (GetNation() == (uint8)Nation::KARUS) // 210 , 1853 emc
			return isInRangeSlow(float(210.0f), float(1853.0f), 50);// (GetX() < 244.0f && GetX() > 176.0f) && ((GetZ() < 1880.0f && GetZ() > 1820.0f)));
		break;

	case ZONE_KARUS:
	case ZONE_KARUS2:
	case ZONE_KARUS3:
		if (GetNation() == (uint8)Nation::ELMORAD) // 1860 , 174 emc
			return isInRangeSlow(float(1860.0f), float(174.0f), 50);
		break;

	case ZONE_BATTLE:
		if (GetNation() == (uint8)Nation::ELMORAD)
			return ((GetX() < 125.0f && GetX() > 98.0f) && ((GetZ() < 780.0f && GetZ() > 755.0f)));
		else if (GetNation() == (uint8)Nation::KARUS)
			return ((GetX() < 831.0f && GetX() > 805.0f) && ((GetZ() < 110.0f && GetZ() > 85.0f)));

	case ZONE_BATTLE2:
		if (GetNation() == (uint8)Nation::ELMORAD)
			return ((GetX() < 977.0f && GetX() > 942.0f) && ((GetZ() < 904.0f && GetZ() > 863.0f)));
		else if (GetNation() == (uint8)Nation::KARUS)
			return ((GetX() < 80.0f && GetX() > 46.0f) && ((GetZ() < 174.0f && GetZ() > 142.0f)));

	case ZONE_BATTLE4:
		if (GetNation() == (uint8)Nation::ELMORAD)
			return isInRangeSlow(float(235.0f), float(228.0f), 80)
			|| isInRangeSlow(float(846.0f), float(362.0f), 20)
			|| isInRangeSlow(float(338.0f), float(807.0f), 20);
		else if (GetNation() == (uint8)Nation::KARUS)
			return isInRangeSlow(float(809.0f), float(783.0f), 80)
			|| isInRangeSlow(float(182.0f), float(668.0f), 20)
			|| isInRangeSlow(float(670.0f), float(202.0f), 20);
	}
	return false;
}

bool Unit::isSameEventRoom(Unit* pTarget)
{
	if (pTarget == nullptr)
		return false;

	if (pTarget->isNPC())
	{
		if (GetEventRoom() == ((CNpc*)pTarget)->GetEventRoom())
			return true;
	}
	else
	{
		if (GetEventRoom() == ((CUser*)pTarget)->GetEventRoom())
			return true;
	}
	return false;
}

/**
* @brief	Calculates and resets the player's stats/resistances.
*
* @param	bSendPacket	true to send a subsequent item movement packet
* 						which is almost always required in addition to
* 						using this method.
*/
void Unit::SetNpcAbility(bool bSendPacket /*= true*/)
{
	if (m_sACAmount < 0)
		m_sACAmount = 0;

	if (m_sACSourAmount < 0)
		m_sACSourAmount = 0;

	m_sTotalAc = m_sTotalAcTemp;

	if (m_sACPercent <= 0)
		m_sTotalAc = 0;
	else
		m_sTotalAc = m_sTotalAc * m_sACPercent / 100;

	m_MaxHP = m_MaxHPTemp + m_sMaxHPAmount;
	if ((uint32)m_iHP > m_MaxHP)
		m_iHP = m_MaxHP;
}

bool CUser::SendPrisonZoneArea()
{
	if (!isGM())
	{
		switch (GetZoneID())
		{
		case ZONE_ELMORAD:
		case ZONE_ELMORAD2:
		case ZONE_ELMORAD3:
			if (GetNation() == (uint8)Nation::ELMORAD)
			{
				return false;
			}
			else if (GetNation() == (uint8)Nation::KARUS)
			{
				return false;
			}
			break;
		}
	}
	return false;
}