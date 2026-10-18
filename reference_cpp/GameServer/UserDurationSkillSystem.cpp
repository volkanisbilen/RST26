#include "stdafx.h"
#include "MagicInstance.h"

void CUser::SpTimeChange()
{
	m_tSpLastTimeNormal = UNIXTIME;

	if (!isPortuKurian())
		return;

	if (m_MaxSp != m_sSp)
	{
		if (isBeginnerKurianPortu())
		{
			SpChange(5);
		}
		else if (isNoviceKurianPortu())
		{
			SpChange(7);
		}
		else if (isMasteredKurianPortu())
		{
			uint16 sptotalpoint = 7;

			if (CheckSkillPoint(PRO_SKILL4, 1, 23))
				sptotalpoint++;
			if (CheckSkillPoint(PRO_SKILL4, 2, 23))
				sptotalpoint++;
			if (CheckSkillPoint(PRO_SKILL4, 3, 23))
				sptotalpoint++;
			if (CheckSkillPoint(PRO_SKILL4, 4, 23))
				sptotalpoint++;
			if (CheckSkillPoint(PRO_SKILL4, 5, 23))
				sptotalpoint++;

			SpChange(sptotalpoint);
		}
	}
}

void CUser::HPTimeChange()
{
	m_tHPLastTimeNormal = UNIXTIME;
	Packet result;

	if (isDead())
		return;

	int mpPercent = 100;

	if (GetZoneID() == ZONE_SNOW_BATTLE
		&& g_pMain->m_byBattleOpen == SNOW_BATTLE)
	{
		HpChange(5);
		return;
	}

	// For mages with under 30% of HP
	if (CheckClass(110, 210)
		&& m_sMp < (30 * m_MaxMp / 100))
		mpPercent = 120;

	if (m_bResHpType == USER_STANDING)
	{
		if (isTransformed()) {
			// TODO: Apply transformation rates
		}

		if (m_MaxMp != m_sMp)
			MSpChange((int)(((GetLevel() * (1 + GetLevel() / 60.0) + 1) * 0.2) + 3) * mpPercent / 100);
	}
	else if (m_bResHpType == USER_SITDOWN)
	{
		if (isGM())
		{
			if (m_MaxHp != m_sHp)
				HpChange(m_MaxHp);

			if (m_MaxMp != m_sMp)
				MSpChange(m_MaxMp);
			
			if (m_MaxSp != m_sSp)
			{
				if (isPortuKurian())
					SpChange(m_MaxSp);
			}
		}
		else
		{
			if (m_MaxHp != m_sHp)
				HpChange((int)(GetLevel() * (1 + GetLevel() / 30.0)) + 3);

			if (m_MaxMp != m_sMp)
			{
				if (GetZoneID() == ZONE_PRISON && GetLevel() > 1)
					MSpChange(+(m_MaxMp * 5 / 100));
				else
					MSpChange((int)(((m_MaxMp * 5) / ((GetLevel() - 1) + 30)) + 3) * mpPercent / 100);
			}
		}
	}
}

void CUser::TrainingProcess()
{
	Packet result;

	if (GetLevel() >= 10)
	{
		if (m_lastTrainingTime == 0)
			m_lastTrainingTime = UNIXTIME;

		if (m_lastTrainingTime + PLAYER_TRAINING_INTERVAL <= UNIXTIME)
		{
			m_lastTrainingTime = UNIXTIME;

			uint8 level;
			uint32 iExp = 0;

			// calculation to give
			level = GetLevel();
			if (level >= 10 && level <= 20)
				iExp = 50;
			else if (level > 20 && level <= 40)
				iExp = 200;
			else if (level > 40 && level <= 60)
				iExp = 1000;
			else if (level > 60 && level <= 70)
				iExp = 2000;
			else if (level > 70 && level <= 80)
				iExp = 2500;
			else if (level > 80)
				iExp = 5000;

			m_iTotalTrainingExp += iExp;

			ExpChange("training",iExp, true);
			result.clear();
			result.Initialize(WIZ_MINING);
			result << uint8(18) << uint8(3) << m_iTotalTrainingExp;
			Send(&result);
		}
	}
}

void CUser::HPTimeChangeType3()
{
	if (isDead()
		|| !m_bType3Flag)
		return;

	uint16	totalActiveDurationalSkills = 0,
		totalActiveDOTSkills = 0;

	bool bIsDOT = false;
	for (int i = 0; i < MAX_TYPE3_REPEAT; i++)
	{
		MagicType3 * pEffect = &m_durationalSkills[i];
		if (pEffect == nullptr || !pEffect->m_byUsed)
			continue;

		// Has the required interval elapsed before using this skill?
		if ((UNIXTIME - pEffect->m_tHPLastTime) >= pEffect->m_bHPInterval)
		{
			Unit * pUnit = g_pMain->GetUnitPtr(pEffect->m_sSourceID, GetZoneID());
			if (pUnit == nullptr)
				continue;

			// Reduce the HP 
			HpChange(pEffect->m_sHPAmount, pUnit); // do we need to specify the source of the DOT?
			pEffect->m_tHPLastTime = UNIXTIME;

			if (pEffect->m_sHPAmount < 0)
				bIsDOT = true;

			// Has the skill expired yet?
			if (++pEffect->m_bTickCount == pEffect->m_bTickLimit)
			{
				Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_DURATION_EXPIRED));

				// Healing-over-time skills require the type 100
				if (pEffect->m_sHPAmount > 0)
					result << uint8(100);
				else // Damage-over-time requires 200.
					result << uint8(200);

				Send(&result);

				// If the skill inflicts poison damage, remember to remove the poison indicator!
				if (pEffect->m_byAttribute == (uint8)ResistanceTypes::POISON_R)
					SendUserStatusUpdate(UserStatus::USER_STATUS_POISON, UserStatusBehaviour::USER_STATUS_CURE);

				pEffect->Reset();
			}
		}

		if (pEffect->m_byUsed)
		{
			totalActiveDurationalSkills++;
			if (pEffect->m_sHPAmount < 0)
				totalActiveDOTSkills++;
		}
	}

	// Have all the skills expired?
	if (totalActiveDurationalSkills == 0)
		m_bType3Flag = false;

	// If there was DOT skills when we started, but none anymore... revert the HP bar.
	if (bIsDOT && totalActiveDOTSkills == 0)
		SendUserStatusUpdate(UserStatus::USER_STATUS_DOT,UserStatusBehaviour::USER_STATUS_CURE);
}

void CUser::Type4Duration()
{
	std::vector<uint8> willDel;
	m_buffLock.lock();
	foreach(itr, m_buffMap) {
		if (itr->second.m_tEndTime > UNIXTIME)
			continue;

		willDel.push_back(itr->first);
	}
	m_buffLock.unlock();
	foreach(itr, willDel)
		CMagicProcess::RemoveType4Buff((*itr), this, true, isLockableScroll((*itr)));

	if (!isDebuffed() && !m_bType3Flag)
		SendUserStatusUpdate(UserStatus::USER_STATUS_POISON, UserStatusBehaviour::USER_STATUS_CURE);
}

void CUser::Type9Duration(bool bForceExpiration) {

	std::vector< _BUFF_TYPE9_INFO> mlist;
	m_buff9lock.lock();
	foreach(itr, m_type9BuffMap) {
		if (itr->second.tEndTime != -1 && (bForceExpiration || (int32)UNIXTIME >= itr->second.tEndTime))
			mlist.push_back(itr->second);
	}
	m_buff9lock.unlock();

	foreach(itr, mlist) {
		MagicInstance instance;
		instance.pSkillCaster = this;
		instance.nSkillID = itr->nSkillID;
		instance.Type9Cancel(true);
	}
}