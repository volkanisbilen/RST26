#include "stdafx.h"

void CUser::SkillPointChange(Packet & pkt)
{
	uint8 max_level = g_pMain->m_byMaxLevel;
	if (IsCindIn() && g_pMain->pCindWar.settingid < 5)
		max_level = g_pMain->pCindWar.pSetting[g_pMain->pCindWar.settingid].beglevel;

	uint8 type = pkt.read<uint8>();
	Packet result(WIZ_SKILLPT_CHANGE, type);
	// invalid type
	if (type < (uint8)SkillPointCategory::SkillPointCat1 || type >(uint8)SkillPointCategory::SkillPointMaster
		// not enough free skill points to allocate
		|| m_bstrSkill[0] < 1
		// restrict skill points per category to your level
		|| m_bstrSkill[type] + 1 > GetLevel()
		// we need our first job change to assign skill points
		|| (GetClass() % 100) <= 4
		// to set points in the mastery category, we need to be mastered.
		|| (type == (uint8)SkillPointCategory::SkillPointMaster && (!isMastered()
				// force a limit of MAX_LEVEL - 60 (the level you can do the mastery quest)
				// on the master skill category, so the limit's 23 skill points with a level 83 cap.
				|| m_bstrSkill[type] >= (max_level - 60)
				// allow only 1 point in the master category for every level above 60.
				|| m_bstrSkill[type] >= (GetLevel() - 60))))

	{
		result << m_bstrSkill[type]; // only send the packet on failure
		Send(&result);
		return;
	}

	--m_bstrSkill[0];
	++m_bstrSkill[type];
	SetUserAbility();
}

// Dialog
void CUser::SendStatSkillDistribute()
{
	Packet result(WIZ_CLASS_CHANGE, uint8(CLASS_CHANGE_REQ));
	Send(&result);
}

uint32 CUser::getskillpointreqmoney() {
	uint32 temp_value = 0;
	if (GetPremium() == 12 || CheckExistItem(RETURNTOKENS))
		temp_value = 0;
	else
		temp_value = (int)pow((GetLevel() * 2.0f), 3.4f);

	if (GetLevel() < 30)
		temp_value = (int)(temp_value * 0.4f);
	else if (GetLevel() >= 60)
		temp_value = (int)(temp_value * 1.5f);
	if (g_pMain->pServerSetting.FreeSkillandStat)
		temp_value = 0;
	return temp_value;
}

void CUser::SendPresetReqMoney() {
	Packet newpkt(XSafe, uint8(XSafeOpCodes::RESET));
	newpkt << getskillpointreqmoney();
	Send(&newpkt);
}

#pragma region CUser::AllSkillPointChange
void CUser::AllSkillPointChange(bool bIsFree)
{
	Packet result(WIZ_CLASS_CHANGE, uint8(ALL_SKILLPT_CHANGE));
	int index = 0, skill_point = 0, money = 0, temp_value = getskillpointreqmoney(), old_money = 0;
	uint8 type = 0;

	// Level too low.
	if (GetLevel() < 10)
		goto fail_return;

	// Get total skill points
	for (int i = 1; i < 9; i++)
		skill_point += m_bstrSkill[i];

	// If we don't have any skill points, there's no point resetting now is there.
	if (skill_point <= 0)
	{
		type = 2;
		goto fail_return;
	}

	if (g_pMain->pServerSetting.FreeSkillandStat)
		bIsFree = true;

	// Not enough money.
	if (!bIsFree && !GoldLose(temp_value, false))
		goto fail_return;

	// Reset skill points.
	m_bstrSkill[0] = (GetLevel() - 9) * 2;
	for (int i = 1; i < 9; i++)
		m_bstrSkill[i] = 0;

	//RobItem(700001000, 1);

	result << uint8(1) << GetCoins() << m_bstrSkill[0];
	SetUserAbility();
	Send(&result);
	return;

fail_return:
	result << type << temp_value;
	Send(&result);
}
#pragma endregion

#pragma region CUser::AllPointChange
void CUser::AllPointChange(bool bIsFree)
{
	Packet result(WIZ_CLASS_CHANGE, uint8(ALL_POINT_CHANGE));
	int temp_money = getskillpointreqmoney();
	uint16 statTotal;

	uint16 byStr, bySta, byDex, byInt, byCha;
	uint8 bResult = 0;

	if (!IsCindIn() && GetLevel() > g_pMain->m_byMaxLevel)
		goto fail_return;
	
	for (int i = 0; i < SLOT_MAX; i++)
	{
		if (m_sItemArray[i].nNum) {
			bResult = 4;
			g_pMain->SendHelpDescription(this, "For stat point reset, take off your clothes.");
			goto fail_return;
		}
	}
	if (g_pMain->pServerSetting.FreeSkillandStat)
		bIsFree = true;

	//// It's 300-10 for clarity (the 10 3being the stat points assigned on char creation)
	//if (GetStatTotal() == 290)
	//{
	//	bResult = 2; // don't need to reallocate stats, it has been done already...
	//	goto fail_return;
	//}

	// Not enough coins
	if (!bIsFree && !GoldLose(temp_money, false))
		goto fail_return;

	// TODO: Pull this from the database.
	switch (m_bRace)
	{

	case KARUS_BIG:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
	case KURIAN:
		if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		break;
	case KARUS_MIDDLE:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
	case KARUS_SMALL:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
	case KARUS_WOMAN:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
	case BABARIAN:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
	case ELMORAD_MAN:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		break;
	case ELMORAD_WOMAN:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		break;
	case PORUTU:
		if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		break;

	}

	// Players gain 3 stats points for each level up to and including 60.
	// They also received 10 free stat points on creation. 
	m_sPoints = 10 + (GetLevel() - 1) * 3;

	// For every level after 60, we add an additional two points.
	if (GetLevel() > 60)
		m_sPoints += 2 * (GetLevel() - 60);

	statTotal = GetStatTotal();
	//ASSERT(statTotal == 290);

	if (statTotal != 290) // hata packet msg yolla
		return;

	SetUserAbility();

	byStr = GetStat(StatType::STAT_STR);
	bySta = GetStat(StatType::STAT_STA),
	byDex = GetStat(StatType::STAT_DEX);
	byInt = GetStat(StatType::STAT_INT),
	byCha = GetStat(StatType::STAT_CHA);

	result << uint8(1) // result (success)
		<< GetCoins()
		<< byStr << bySta << byDex << byInt << byCha
		<< m_MaxHp << m_MaxMp << m_sTotalHit << m_sMaxWeight << m_sPoints;
	Send(&result);
	return;

fail_return:
	result << bResult << temp_money;
	Send(&result);
}
#pragma endregion

#pragma region CUser::AllSkillPointChange2
void CUser::AllSkillPointChange2(bool bIsFree)
{
	Packet result(WIZ_CLASS_CHANGE, uint8(ALL_SKILLPT_CHANGE));
	int index = 0, skill_point = 0, money = 0, temp_value = getskillpointreqmoney(), old_money = 0;
	uint8 type = 0;

	// Level too low.
	if (GetLevel() < 10)
		goto fail_return;

	// Get total skill points
	for (int i = 1; i < 9; i++)
		skill_point += m_bstrSkill[i];

	// If we don't have any skill points, there's no point resetting now is there.
	if (skill_point <= 0)
	{
		type = 2;
		goto fail_return;
	}
	// Reset skill points.
	m_bstrSkill[0] = (GetLevel() - 9) * 2;
	for (int i = 1; i < 9; i++)
		m_bstrSkill[i] = 0;

	RobItem(700001000, 1);

	result << uint8(1) << GetCoins() << m_bstrSkill[0];
	SetUserAbility();
	Send(&result);
	return;

fail_return:
	result << type << temp_value;
	Send(&result);
}
#pragma endregion

#pragma region CUser::AllPointChange2
void CUser::AllPointChange2(bool bIsFree)
{
	Packet result(WIZ_CLASS_CHANGE, uint8(ALL_POINT_CHANGE));
	int temp_money = 0;
	uint16 statTotal;

	uint16 byStr, bySta, byDex, byInt, byCha;
	uint8 bResult = 0;

	if (GetLevel() > g_pMain->m_byMaxLevel)
		goto fail_return;

	for (int i = 0; i < SLOT_MAX; i++)
	{
		if (m_sItemArray[i].nNum) {
			bResult = 4;
			goto fail_return;
		}
	}

	// It's 300-10 for clarity (the 10 being the stat points assigned on char creation)
	if (GetStatTotal() == 290)
	{
		bResult = 2; // don't need to reallocate stats, it has been done already...
		goto fail_return;
	}

	// TODO: Pull this from the database.
	switch (m_bRace)
	{

	case KARUS_BIG:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
	case KURIAN:
		if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		break;
	case KARUS_MIDDLE:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
	case KARUS_SMALL:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
	case KARUS_WOMAN:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
	case BABARIAN:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
	case ELMORAD_MAN:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		break;
	case ELMORAD_WOMAN:
		if (isWarrior())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isRogue())
		{
			SetStat(StatType::STAT_STR, 60);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 70);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPriest())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isMage())
		{
			SetStat(StatType::STAT_STR, 50);
			SetStat(StatType::STAT_STA, 60);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 70);
			SetStat(StatType::STAT_CHA, 50);
		}
		else if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		break;
	case PORUTU:
		if (isPortuKurian())
		{
			SetStat(StatType::STAT_STR, 65);
			SetStat(StatType::STAT_STA, 65);
			SetStat(StatType::STAT_DEX, 60);
			SetStat(StatType::STAT_INT, 50);
			SetStat(StatType::STAT_CHA, 50);
		}
		break;
	}

	statTotal = GetStatTotal();
	//ASSERT(statTotal == 290);

	// Players gain 3 stats points for each level up to and including 60.
	// They also received 10 free stat points on creation. 
	m_sPoints = 10 + (GetLevel() - 1) * 3;

	// For every level after 60, we add an additional two points.
	if (GetLevel() > 60)
		m_sPoints += 2 * (GetLevel() - 60);

	SetUserAbility();

	byStr = GetStat(StatType::STAT_STR);
	bySta = GetStat(StatType::STAT_STA),
	byDex = GetStat(StatType::STAT_DEX);
	byInt = GetStat(StatType::STAT_INT),
	byCha = GetStat(StatType::STAT_CHA);


	RobItem(700001000, 1);

	result << uint8(1) // result (success)
		<< GetCoins()
		<< byStr << bySta << byDex << byInt << byCha
		<< m_MaxHp << m_MaxMp << m_sTotalHit << m_sMaxWeight << m_sPoints;
	Send(&result);
	return;

fail_return:
	result << bResult << temp_money;
	Send(&result);
}
#pragma endregion

void CUser::StatPresetHandle(Packet& pkt)	//2370 Skill stat preset paket voidi
{
	/*
	1:You have successfully apply the Stat
	2:Available after Stat Redistribution
	3:You have failed to apply the Stat
	4:Stat Preset Error:
	1:You have successfully apply the Skill
	*/

	Packet Preset(WIZ_PRESET);
	uint8 Type = pkt.read<uint8>();
	if (!isInGame() || isMerchanting() || isMining() || isDead()
		|| GetZoneID() == ZONE_CHAOS_DUNGEON
		|| GetZoneID() == ZONE_KNIGHT_ROYALE
		|| GetZoneID() == ZONE_DUNGEON_DEFENCE
		|| GetZoneID() == ZONE_PRISON) {
		Preset << uint8(1) << uint8(3);
		Send(&Preset);
		return;
	}

	switch (Type)
	{
	case 1:
		PresetMyStatReset(pkt);
		break;
	case 2:
		PresetMySkillReset(pkt);
		break;
	default:
		printf("PresetProcess unhandled type %d \n", Type);
		break;
	}
}

void CUser::PresetMySkillReset(Packet& pkt)
{
	uint8 SkillPoint[4] = { 0 };
	for (int i = 0; i < 4; i++) pkt >> SkillPoint[i];

	Packet Preset;
	uint8 CeilPoint[4] = { 83,83,83,23 }; bool NotChanges = true;
	for (int i = 0; i < 4; i++)
	{
		if (SkillPoint[i] > CeilPoint[i] || SkillPoint[i] < 0)
		{
			Preset.clear();
			Preset.Initialize(WIZ_PRESET);
			Preset << uint8(2) << uint8(3);
			Send(&Preset);
			return;
		}
		if (SkillPoint[i] > 0) NotChanges = false;
	}

	if (NotChanges)
	{
		Preset.clear();
		Preset.Initialize(WIZ_CLASS_CHANGE);
		Preset << uint8(ALL_SKILLPT_CHANGE) << uint8(2);
		Send(&Preset);
		return;
	}

	uint16 usedPoints = 0, maxPoints = (GetLevel() - 9) * 2;
	for (int i = 0; i < 4; i++)
		usedPoints += SkillPoint[i];

	if (usedPoints > maxPoints || GetLevel() < 10)
	{
		Preset.clear();
		Preset.Initialize(WIZ_PRESET);
		Preset << uint8(2) << uint8(3);
		Send(&Preset);
		return;
	}

	int skill_point = 0;
	for (int i = 1; i < 9; i++)
		skill_point += m_bstrSkill[i];

	if (skill_point <= 0)
	{
		Preset.clear();
		Preset.Initialize(WIZ_CLASS_CHANGE);
		Preset << uint8(ALL_SKILLPT_CHANGE) << uint8(2);
		Send(&Preset);
		return;
	}

	int temp_money = (int)pow((GetLevel() * 2.0f), 3.4f);
	if (GetLevel() < 30)
		temp_money = (int)(temp_money * 0.4f);
	else if (GetLevel() >= 60)
		temp_money = (int)(temp_money * 1.5f);

	if ((g_pMain->m_sDiscount == 1 && g_pMain->m_byOldVictory == GetNation()) || g_pMain->m_sDiscount == 2)
		temp_money /= 2;

	if (temp_money > 0 && !GoldLose(temp_money, true))
	{
		Preset.clear();
		Preset.Initialize(WIZ_CLASS_CHANGE);
		Preset << uint8(ALL_SKILLPT_CHANGE) << uint8(0) << temp_money;
		Send(&Preset);
		return;
	}

	m_bstrSkill[0] = maxPoints - usedPoints;
	for (int i = 1; i < 9; i++)
		m_bstrSkill[i] = 0;

	for (int i = 0; i < 4; i++)
		m_bstrSkill[i + 5] = SkillPoint[i];

	Preset.clear();
	Preset.Initialize(WIZ_PRESET);
	Preset << uint8(2) << uint8(1);
	for (int i = 0; i < 4; i++) Preset << m_bstrSkill[i + 5];
	Preset << m_bstrSkill[0];
	Send(&Preset);
	SetUserAbility();
}

void CUser::PresetMyStatReset(Packet& pkt)
{
	Packet Preset;

	uint8 point[5]{};
	for (int i = 0; i < 5; i++)
		pkt >> point[i];

	uint8 basePoint[5]{};
	basePoint[0] = 50;
	basePoint[1] = 60;
	basePoint[2] = 60;
	basePoint[3] = 50;
	basePoint[4] = 50;

	if (isPriest())
		basePoint[4] += 20;
	else if (isWarrior())
	{
		basePoint[0] += 15;
		basePoint[1] += 5;
	}
	else if (isMage())
	{
		basePoint[2] += 10;
		basePoint[4] += 20;
		basePoint[1] -= 10;
	}
	else if (isRogue())
	{
		basePoint[0] += 10;
		basePoint[2] += 10;
	}
	else if (isPortuKurian())
	{
		basePoint[0] += 15;
		basePoint[1] += 5;
	}

	uint16 totalMinus = 0;
	for (int i = 0; i < 5; i++)
	{
		if (point[i] < basePoint[i])
			point[i] = basePoint[i];
		totalMinus += point[i] - basePoint[i];
	}

	uint16 FreePoint = 10 + (GetLevel() - 1) * 3;
	if (GetLevel() > 60) FreePoint += 2 * (GetLevel() - 60);

	if (totalMinus > FreePoint || GetLevel() > MAX_LEVEL) {
		Preset.clear();
		Preset.Initialize(WIZ_PRESET);
		Preset << uint8(1) << uint8(3);
		Send(&Preset);
		return;
	}

	FreePoint -= totalMinus;

	for (int i = 0; i < SLOT_MAX; i++) {
		if (m_sItemArray[i].nNum) {
			Preset.clear();
			Preset.Initialize(WIZ_CLASS_CHANGE);
			Preset << uint8(ALL_POINT_CHANGE) << uint8(4);
			Send(&Preset);
			return;
		}
	}

	bool NotChanges = true;
	for (int j = 0; j < 5; j++) {
		if (point[j] != basePoint[j]) NotChanges = false;
	}

	if (NotChanges || GetStatTotal() == 290) {
		Preset.clear();
		Preset.Initialize(WIZ_CLASS_CHANGE);
		Preset << uint8(ALL_POINT_CHANGE) << uint8(2);
		Send(&Preset);
		return;
	}

	int temp_money = (int)pow((GetLevel() * 2.0f), 3.4f);
	if (GetLevel() < 30)
		temp_money = (int)(temp_money * 0.4f);
	else if (GetLevel() >= 60)
		temp_money = (int)(temp_money * 1.5f);

	if ((g_pMain->m_sDiscount == 1 && g_pMain->m_byOldVictory == GetNation())
		|| g_pMain->m_sDiscount == 2)
		temp_money /= 2;

	if (GetPremium() == 12)
		temp_money = 0;

	if (temp_money > 0 && !GoldLose(temp_money, true)) {
		Preset.clear();
		Preset.Initialize(WIZ_CLASS_CHANGE);
		Preset << uint8(ALL_POINT_CHANGE) << uint8(0) << temp_money;
		Send(&Preset);
		return;
	}

	for (int s = 0; s < 3; s++) SetStat((StatType)s, point[s]);

	m_bStats[3] = point[4];
	m_bStats[4] = point[3];

	m_sPoints = FreePoint;

	Preset.clear();
	Preset.Initialize(WIZ_PRESET);
	Preset << uint8(1) << uint8(1);
	for (int c = 0; c < 5; c++) Preset << GetStat((StatType)c);
	Preset << m_sPoints;
	Send(&Preset);

	SetUserAbility();
}

void CUser::ClassChangeReq()
{
	Packet result(WIZ_CLASS_CHANGE, uint8(CLASS_CHANGE_RESULT));
	if (GetLevel() < 10) // if we haven't got our first job change
		result << uint8(2);
	else if ((m_sClass % 100) > 4)// if we've already got our job change
	{ 
		if ((isPortuKurian()) && (m_sClass % 100) == 13)
			result << uint8(1);
		else
			result << uint8(3);
	}
	else // otherwise
		result << uint8(1);
	Send(&result);
}

bool CUser::CheckSkillPoint(uint8 skillnum, uint8 min, uint8 max)
{
	if (skillnum < 5 || skillnum > 8)
		return false;

	return (m_bstrSkill[skillnum] >= min && m_bstrSkill[skillnum] <= max);
}

bool CUser::CheckClass(short class1, short class2, short class3, short class4, short class5, short class6)
{
	return (JobGroupCheck(class1) || JobGroupCheck(class2) || JobGroupCheck(class3) || JobGroupCheck(class4) || JobGroupCheck(class5) || JobGroupCheck(class6));
}

bool CUser::JobGroupCheck(short jobgroupid)
{
	if (jobgroupid > 100)
		return GetClass() == jobgroupid;

	ClassType subClass = GetBaseClassType();
	switch (jobgroupid)
	{
	case GROUP_WARRIOR:
		return (subClass == ClassType::ClassWarrior || subClass == ClassType::ClassWarriorNovice || subClass == ClassType::ClassWarriorMaster);

	case GROUP_ROGUE:
		return (subClass == ClassType::ClassRogue || subClass == ClassType::ClassRogueNovice || subClass == ClassType::ClassRogueMaster);

	case GROUP_MAGE:
		return (subClass == ClassType::ClassMage || subClass == ClassType::ClassMageNovice || subClass == ClassType::ClassMageMaster);

	case GROUP_CLERIC:
		return (subClass == ClassType::ClassPriest || subClass == ClassType::ClassPriestNovice || subClass == ClassType::ClassPriestMaster);

	case GROUP_PORTU_KURIAN:
		return (subClass == ClassType::ClassPortuKurian || subClass == ClassType::ClassPortuKurianNovice || subClass == ClassType::ClassPortuKurianMaster);
	}

	return (subClass == (ClassType)jobgroupid);
}

/**
* @brief	Handles player stat assignment.
*
* @param	pkt	The packet.
*/
void CUser::PointChange(Packet & pkt)
{
	uint8 type = pkt.read<uint8>();
	StatType statType = (StatType)(type - 1);

	if (statType < StatType::STAT_STR || statType >= StatType::STAT_COUNT
		|| m_sPoints < 1
		|| GetStat(statType) >= STAT_MAX)
		return;

	Packet result(WIZ_POINT_CHANGE, type);

	m_sPoints--; // remove a free point
	result << uint16(++m_bStats[(uint8)statType]); // assign the free point to a stat
	SetUserAbility();
	result << m_MaxHp << m_MaxMp << m_sTotalHit << m_sMaxWeight << uint16(m_sHp) << uint16(m_sMp);
	Send(&result);
	SendItemMove(1, 1);
}

#pragma region CUser::GetBaseClass()
uint8 CUser::GetBaseClass()
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

#pragma endregion 