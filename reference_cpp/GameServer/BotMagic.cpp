#include "StdAfx.h"

static float GetNormalizedBotSkillRange(CBot* pBot, _MAGIC_TABLE* pSkill, float fallbackRange)
{
	if (pSkill == nullptr || pSkill->sRange <= 0)
		return fallbackRange;

	float range = (float)pSkill->sRange;
	if (range > 25.0f)
		range /= 10.0f;

	float hardCap = 12.0f;
	if (pBot != nullptr)
	{
		if (pBot->isMage() || pBot->isPriest())
			hardCap = 20.0f;
		else if (pBot->isRogue())
			hardCap = 18.0f;
	}

	if (range > hardCap)
		range = hardCap;

	// Farm bots should keep attacking from longer distance to avoid idle/stuck behavior around monster slots.
	if (pBot != nullptr && pBot->GetBotState() == BOT_FARMER)
		range *= 0.90f;
	else
		range *= 0.375f;

	if (range < 1.0f)
		range = 1.0f;

	return range;
}

time_t CBot::MagicPacket(uint8 opcode, uint32 nSkillID, int16 sCasterID, int16 sTargetID, int16 sData1, int16 sData2, int16 sData3)
{
	_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(nSkillID);
	if (pSkill == nullptr)
		return -1;

	Packet result(WIZ_MAGIC_PROCESS, opcode); // here we emulate a skill packet to be handled.
	result << nSkillID << sCasterID << sTargetID << sData1 << sData2 << sData3;
	CMagicProcess::MagicPacketBot(result, this);
	return -1;
}

void CBot::GetAssasinDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo)
{
	if (tid < 0)
		return;

	int32 sSkillID = 0, bSkillID = 0;
	uint8 weaponSlots[] = { GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_LEFTHAND : LEFTHAND, GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_RIGHTHAND : RIGHTHAND };
	bool ArrowMagics = false, DaggersMagics = false;

	foreach_array(slot, weaponSlots)
	{
		_ITEM_TABLE* pWeapon = GetItemPrototype(weaponSlots[slot]);
		if (pWeapon == nullptr)
			continue;

		if (pWeapon->isDagger())
		{
			GetAssasinDaggerDamageMagic(tid, X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
			return;
		}
		else if (pWeapon->isBow())
		{
			GetAssasinArrowDamageMagic(tid, X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
			return;
		}
		else
			return;
	}
}

void CBot::GetAssasinArrowDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo)
{
	if (tid < 0)
		return;

	int nRandom = myrand(0, 20);
	uint32 sSkillID;
	switch (nRandom)
	{
	case 0:
		sSkillID = 108500;
		break;
	case 1:
		sSkillID = 108505;
		break;
	case 2:
		sSkillID = 108510;
		break;
	case 3:
		sSkillID = 108515;
		break;
	case 4:
		sSkillID = 108520;
		break;
	case 5:
		sSkillID = 108525;
		break;
	case 6:
		sSkillID = 108530;
		break;
	case 7:
		sSkillID = 108535;
		break;
	case 8:
		sSkillID = 108540;
		break;
	case 9:
		sSkillID = 108545;
		break;
	case 10:
		sSkillID = 108550;
		break;
	case 11:
		sSkillID = 108552;
		break;
	case 12:
		sSkillID = 108555;
		break;
	case 13:
		sSkillID = 108557;
		break;
	case 14:
		sSkillID = 108560;
		break;
	case 15:
		sSkillID = 108562;
		break;
	case 16:
		sSkillID = 108566;
		break;
	case 17:
		sSkillID = 108570;
		break;
	case 18:
		sSkillID = 108575;
		break;
	case 19:
		sSkillID = 108580;
		break;
	default:
		sSkillID = 108500;
		break;
	}

	if (GetNation() == ELMORAD)
		sSkillID += 100000;

	_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(sSkillID);
	if (pSkill == nullptr)
		return;

	if (GetClass() != (pSkill->sSkill / 10)
		|| GetLevel() < pSkill->sSkillLevel)
	{
		if (GetNation() == KARUS)
			sSkillID = 108500;
		else
			sSkillID = 208500;
	}

	Unit* pUnit = g_pMain->GetUnitPtr(tid, GetZoneID());
	if (pUnit == nullptr
		|| pUnit->isDead()
		|| pUnit->isPlayer() && TO_USER(pUnit)->isGM())
		return;

	float sRange = GetNormalizedBotSkillRange(this, pSkill, 10.0f);
	float fDis = GetDistanceSqrt(pUnit);
	if (fDis > sRange)
		MoveProcess(X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
	else
	{
		if ((uint32)UNIXTIME >= (m_sSkillCoolDown))
		{
			MagicPacket(MAGIC_CASTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			MagicPacket(MAGIC_EFFECTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			m_sSkillCoolDown = (uint32)UNIXTIME + 2;
		}
	}
}

void CBot::GetAssasinDaggerDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo)
{
	if (tid < 0)
		return;

	int nRandom = myrand(0, 14);
	uint32 bSkillID;
	switch (nRandom)
	{
	case 0:
		bSkillID = 108600;
		break;
	case 1:
		bSkillID = 108610;
		break;
	case 2:
		bSkillID = 108615;
		break;
	case 3:
		bSkillID = 108620;
		break;
	case 4:
		bSkillID = 108635;
		break;
	case 5:
		bSkillID = 108600;
		break;
	case 6:
		bSkillID = 108645;
		break;
	case 7:
		bSkillID = 108650;
		break;
	case 8:
		bSkillID = 108655;
		break;
	case 9:
		bSkillID = 108656;
		break;
	case 10:
		bSkillID = 108670;
		break;
	case 11:
		bSkillID = 108675;
		break;
	case 12:
		bSkillID = 108680;
		break;
	case 13:
		bSkillID = 108685;
		break;
	default:
		bSkillID = 108600;
		break;
	}

	if (GetNation() == ELMORAD)
		bSkillID += 100000;

	_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(bSkillID);
	if (pSkill == nullptr)
		return;

	if (GetClass() != (pSkill->sSkill / 10)
		|| GetLevel() < pSkill->sSkillLevel)
	{
		if (GetNation() == KARUS)
			bSkillID = 108600;
		else
			bSkillID = 208600;
	}

	Unit* pUnit = g_pMain->GetUnitPtr(tid, GetZoneID());
	if (pUnit == nullptr
		|| pUnit->isDead()
		|| pUnit->isPlayer() && TO_USER(pUnit)->isGM())
		return;

	float sRange = GetNormalizedBotSkillRange(this, pSkill, 10.0f);
	float fDis = GetDistanceSqrt(pUnit);
	if (fDis > sRange)
		MoveProcess(X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
	else
	{
		if ((uint32)UNIXTIME >= (m_sSkillCoolDown))
		{
			MagicPacket(MAGIC_CASTING, bSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			MagicPacket(MAGIC_EFFECTING, bSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			m_sSkillCoolDown = (uint32)UNIXTIME + 2;
		}
	}
}

void CBot::GetWarriorDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo)
{
	if (tid < 0)
		return;

	int nRandom = myrand(0, 17);
	uint32 sSkillID;
	switch (nRandom)
	{
	case 0:
		sSkillID = 106500;
		break;
	case 1:
		sSkillID = 106505;
		break;
	case 2:
		sSkillID = 106510;
		break;
	case 3:
		sSkillID = 106515;
		break;
	case 4:
		sSkillID = 106520;
		break;
	case 5:
		sSkillID = 106525;
		break;
	case 6:
		sSkillID = 106530;
		break;
	case 7:
		sSkillID = 106535;
		break;
	case 8:
		sSkillID = 106540;
		break;
	case 9:
		sSkillID = 106545;
		break;
	case 10:
		sSkillID = 106550;
		break;
	case 11:
		sSkillID = 106555;
		break;
	case 12:
		sSkillID = 106557;
		break;
	case 13:
		sSkillID = 106560;
		break;
	case 14:
		sSkillID = 106570;
		break;
	case 15:
		sSkillID = 106575;
		break;
	case 16:
		sSkillID = 106580;
		break;
	default:
		sSkillID = 106500;
		break;
	}

	if (GetNation() == ELMORAD)
		sSkillID += 100000;

	_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(sSkillID);
	if (pSkill == nullptr)
		return;

	if (GetClass() != (pSkill->sSkill / 10)
		|| GetLevel() < pSkill->sSkillLevel)
	{
		if (GetNation() == KARUS)
			sSkillID = 106500;
		else
			sSkillID = 206500;
	}

	Unit* pUnit = g_pMain->GetUnitPtr(tid, GetZoneID());
	if (pUnit == nullptr
		|| pUnit->isDead()
		|| pUnit->isPlayer() && TO_USER(pUnit)->isGM())
		return;

	float sRange = GetNormalizedBotSkillRange(this, pSkill, 10.0f);
	float fDis = GetDistanceSqrt(pUnit);
	if (fDis > sRange)
		MoveProcess(X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
	else
	{
		if ((uint32)UNIXTIME >= (m_sSkillCoolDown))
		{
			MagicPacket(MAGIC_CASTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			MagicPacket(MAGIC_EFFECTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			m_sSkillCoolDown = (uint32)UNIXTIME + 2;
		}
	}
}

void CBot::GetFlameMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo)
{
	if (tid < 0)
		return;

	int nRandom = myrand(0, 19);
	uint32 sSkillID;
	switch (nRandom)
	{
	case 0:
		sSkillID = 110503;
		break;
	case 1:
		sSkillID = 110509;
		break;
	case 2:
		sSkillID = 110515;
		break;
	case 3:
		sSkillID = 110518;
		break;
	case 4:
		sSkillID = 110527;
		break;
	case 5:
		sSkillID = 110533;
		break;
	case 6:
		sSkillID = 110535;
		break;
	case 7:
		sSkillID = 110539;
		break;
	case 8:
		sSkillID = 110542;
		break;
	case 9:
		sSkillID = 110543;
		break;
	case 10:
		sSkillID = 110545;
		break;
	case 11:
		sSkillID = 110551;
		break;
	case 12:
		sSkillID = 110554;
		break;
	case 13:
		sSkillID = 110556;
		break;
	case 14:
		sSkillID = 110557;
		break;
	case 15:
		sSkillID = 110560;
		break;
	case 16:
		sSkillID = 110570;
		break;
	case 17:
		sSkillID = 110571;
		break;
	case 18:
		sSkillID = 110572;
		break;
	case 19:
		sSkillID = 110575;
		break;
	default:
		sSkillID = 110503;
		break;
	}

	if (GetNation() == ELMORAD)
		sSkillID += 100000;

	_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(sSkillID);
	if (pSkill == nullptr)
		return;

	if (GetClass() != (pSkill->sSkill / 10)
		|| GetLevel() < pSkill->sSkillLevel)
	{
		if (GetNation() == KARUS)
			sSkillID = 110002;
		else
			sSkillID = 210002;
	}

	Unit* pUnit = g_pMain->GetUnitPtr(tid, GetZoneID());
	if (pUnit == nullptr
		|| pUnit->isDead()
		|| pUnit->isPlayer() && TO_USER(pUnit)->isGM())
		return;

	float sRange = GetNormalizedBotSkillRange(this, pSkill, 10.0f);
	float fDis = GetDistanceSqrt(pUnit);
	if (fDis > sRange)
		MoveProcess(X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
	else
	{
		if ((uint32)UNIXTIME >= (m_sSkillCoolDown))
		{
			MagicPacket(MAGIC_CASTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			MagicPacket(MAGIC_EFFECTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			m_sSkillCoolDown = (uint32)UNIXTIME + 2;
		}
	}
}

void CBot::GetGlacierMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo)
{
	if (tid < 0)
		return;

	int nRandom = myrand(0, 18);
	uint32 sSkillID;
	switch (nRandom)
	{
	case 0:
		sSkillID = 110503;
		break;
	case 1:
		sSkillID = 110509;
		break;
	case 2:
		sSkillID = 110515;
		break;
	case 3:
		sSkillID = 110518;
		break;
	case 4:
		sSkillID = 110527;
		break;
	case 5:
		sSkillID = 110533;
		break;
	case 6:
		sSkillID = 110535;
		break;
	case 7:
		sSkillID = 110539;
		break;
	case 8:
		sSkillID = 110542;
		break;
	case 9:
		sSkillID = 110543;
		break;
	case 10:
		sSkillID = 110545;
		break;
	case 11:
		sSkillID = 110551;
		break;
	case 12:
		sSkillID = 110554;
		break;
	case 13:
		sSkillID = 110556;
		break;
	case 14:
		sSkillID = 110557;
		break;
	case 15:
		sSkillID = 110560;
		break;
	case 16:
		sSkillID = 110570;
		break;
	case 17:
		sSkillID = 110571;
		break;
	case 18:
		sSkillID = 110572;
		break;
	default:
		sSkillID = 110503;
		break;
	}

	if (GetNation() == ELMORAD)
		sSkillID += 100000;

	_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(sSkillID);
	if (pSkill == nullptr)
		return;

	if (GetClass() != (pSkill->sSkill / 10)
		|| GetLevel() < pSkill->sSkillLevel)
	{
		if (GetNation() == KARUS)
			sSkillID = 110002;
		else
			sSkillID = 210002;
	}

	if (sSkillID != 110002 && sSkillID != 210002)
		sSkillID += 100;

	Unit* pUnit = g_pMain->GetUnitPtr(tid, GetZoneID());
	if (pUnit == nullptr
		|| pUnit->isDead()
		|| pUnit->isPlayer() && TO_USER(pUnit)->isGM())
		return;

	float sRange = GetNormalizedBotSkillRange(this, pSkill, 10.0f);
	float fDis = GetDistanceSqrt(pUnit);
	if (fDis > sRange)
		MoveProcess(X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
	else
	{
		if ((uint32)UNIXTIME >= (m_sSkillCoolDown))
		{
			MagicPacket(MAGIC_CASTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			MagicPacket(MAGIC_EFFECTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			m_sSkillCoolDown = (uint32)UNIXTIME + 2;
		}
	}
}

void CBot::GetLightningMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo)
{
	if (tid < 0)
		return;

	int nRandom = myrand(0, 18);
	uint32 sSkillID;
	switch (nRandom)
	{
	case 0:
		sSkillID = 110503;
		break;
	case 1:
		sSkillID = 110509;
		break;
	case 2:
		sSkillID = 110515;
		break;
	case 3:
		sSkillID = 110518;
		break;
	case 4:
		sSkillID = 110527;
		break;
	case 5:
		sSkillID = 110533;
		break;
	case 6:
		sSkillID = 110535;
		break;
	case 7:
		sSkillID = 110539;
		break;
	case 8:
		sSkillID = 110542;
		break;
	case 9:
		sSkillID = 110543;
		break;
	case 10:
		sSkillID = 110545;
		break;
	case 11:
		sSkillID = 110551;
		break;
	case 12:
		sSkillID = 110554;
		break;
	case 13:
		sSkillID = 110556;
		break;
	case 14:
		sSkillID = 110557;
		break;
	case 15:
		sSkillID = 110560;
		break;
	case 16:
		sSkillID = 110570;
		break;
	case 17:
		sSkillID = 110571;
		break;
	case 18:
		sSkillID = 110572;
		break;
	default:
		sSkillID = 110503;
		break;
	}

	if (GetNation() == ELMORAD)
		sSkillID += 100000;

	_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(sSkillID);
	if (pSkill == nullptr)
		return;

	if (GetClass() != (pSkill->sSkill / 10)
		|| GetLevel() < pSkill->sSkillLevel)
	{
		if (GetNation() == KARUS)
			sSkillID = 110002;
		else
			sSkillID = 210002;
	}

	if (sSkillID != 110002 && sSkillID != 210002)
		sSkillID += 200;

	Unit* pUnit = g_pMain->GetUnitPtr(tid, GetZoneID());
	if (pUnit == nullptr
		|| pUnit->isDead()
		|| pUnit->isPlayer() && TO_USER(pUnit)->isGM())
		return;

	float sRange = GetNormalizedBotSkillRange(this, pSkill, 10.0f);
	float fDis = GetDistanceSqrt(pUnit);
	if (fDis > sRange)
		MoveProcess(X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
	else
	{
		if ((uint32)UNIXTIME >= (m_sSkillCoolDown))
		{
			MagicPacket(MAGIC_CASTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			MagicPacket(MAGIC_EFFECTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			m_sSkillCoolDown = (uint32)UNIXTIME + 2;
		}
	}
}

void CBot::GetPriestDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo)
{
	if (tid < 0)
		return;

	int nRandom = myrand(0, 17);
	uint32 sSkillID;
	switch (nRandom)
	{
	case 0:
		sSkillID = 112709;
		break;
	case 1:
		sSkillID = 112739;
		break;
	case 2:
		sSkillID = 112724;
		break;
	case 3:
		sSkillID = 112715;
		break;
	case 4:
		sSkillID = 112727;
		break;
	case 5:
		sSkillID = 112712;
		break;
	case 6:
		sSkillID = 112721;
		break;
	case 7:
		sSkillID = 112703;
		break;
	case 8:
		sSkillID = 112739;
		break;
	case 9:
		sSkillID = 112750;
		break;
	case 10:
		sSkillID = 112745;
		break;
	case 11:
		sSkillID = 112757;
		break;
	case 12:
		sSkillID = 112760;
		break;
	case 13:
		sSkillID = 112770;
		break;
	case 14:
		sSkillID = 112771;
		break;
	case 15:
		sSkillID = 112772;
		break;
	case 16:
		sSkillID = 112815;
		break;
	default:
		sSkillID = 112001;
		break;
	}

	if (GetNation() == ELMORAD)
		sSkillID += 100000;

	_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(sSkillID);
	if (pSkill == nullptr)
		return;

	if (GetClass() != (pSkill->sSkill / 10)
		|| GetLevel() < pSkill->sSkillLevel)
	{
		if (GetNation() == KARUS)
			sSkillID = 110002;
		else
			sSkillID = 210002;
	}

	Unit* pUnit = g_pMain->GetUnitPtr(tid, GetZoneID());
	if (pUnit == nullptr
		|| pUnit->isDead()
		|| pUnit->isPlayer() && TO_USER(pUnit)->isGM())
		return;

	float sRange = GetNormalizedBotSkillRange(this, pSkill, 10.0f);
	float fDis = GetDistanceSqrt(pUnit);
	if (fDis > sRange)
		MoveProcess(X, Y, Z, will_x, will_y, will_z, sSpeed, echo);
	else
	{
		if ((uint32)UNIXTIME >= (m_sSkillCoolDown))
		{
			MagicPacket(MAGIC_CASTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			MagicPacket(MAGIC_EFFECTING, sSkillID, GetID(), pUnit->GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());
			m_sSkillCoolDown = (uint32)UNIXTIME + 2;
		}
	}
}
