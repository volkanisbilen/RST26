
#include "stdafx.h"
#include "../shared/KOSocketMgr.h"
#include "MagicProcess.h"
#include "MagicInstance.h"

using std::string;
using std::vector;

void MagicInstance::Run()
{
	if (pSkill.isnull())
		pSkill = g_pMain->GetMagicPtr(nSkillID);

	if (pSkill.isnull())
		return SendSkillFailed();

	if (pSkillCaster == nullptr)
		pSkillCaster = g_pMain->GetUnitPtr(sCasterID, sSkillCasterZoneID);

	/*if (pSkillCaster->isPlayer())
		printf("%d: %s: %d %d\n", bOpcode, pSkill.krname.c_str(), TO_USER(pSkillCaster)->GetTargetID(), pSkillCaster->GetID());*/

	if (pSkillCaster == nullptr)
		return SendSkillFailed();

	if (pSkillCaster->isPlayer())
	{

		CUser* pUser = TO_USER(pSkillCaster);
		if ((MagicOpcode)bOpcode == MagicOpcode::MAGIC_CASTING)
		{
			pUser->pUserMagicUsed.castID = pSkill.iNum;
			pUser->pUserMagicUsed.castX = pUser->GetSPosX();
			pUser->pUserMagicUsed.castZ = pUser->GetSPosZ();
		}

		if (pSkill.bMoral == 7 || pSkill.bMoral == 10)
		{

			if (pSkillCaster->isInSpecialEventZone()
				&& !g_pMain->pSpecialEvent.opened
				&& !g_pMain->pCindWar.isON())
				return SendSkillFailed();

			if (g_pMain->isCindirellaZone(pUser->GetZoneID())
				&& g_pMain->pCindWar.isON()
				&& !g_pMain->pCindWar.isStarted()
				&& !g_pMain->pSpecialEvent.opened)
				return SendSkillFailed();
		}

		_MykoSoftinfo& p = pUser->pUserMagicUsed.status;
		if (AnimatedSkill() && pSkill.bType[0] == 2 && pSkill.bCastTime && (MagicOpcode)bOpcode == MagicOpcode::MAGIC_CASTING)
		{

			if (p.skillid && p.skillid != pSkill.iNum && UNIXTIME2 - p.m_castusetime < 500)
				return SendSkillFailed();

			p.m_castusetime = UNIXTIME2;
			p.skillid = nSkillID;
		}
	}

	if (sTargetID != -1 && !pSkillTarget)
		pSkillTarget = g_pMain->GetUnitPtr(sTargetID, sSkillCasterZoneID);

	if (sTargetID != -1 && !pSkillTarget)
		return SendSkillFailed();

	/*25.05.2024*/
	if (pSkill.iNum && pSkillCaster->isPlayer())
	{
		CBot* pUserBot = nullptr;
		pUserBot = g_pMain->m_MapBotList.GetData(sCasterID);
		if (pUserBot != nullptr) // bot ise
		{
			if (pUserBot == nullptr)
			{
				CUser* pUser = g_pMain->GetUserPtr(sCasterID);
				if (pUser && pUser->isGM())
					g_pMain->SendHelpDescription(pUser, string_format("[Skill] ID -> : %d || [Successrate] -> : %d || [Type] -> : %d", pSkill.iNum, pSkill.bSuccessRate, pSkill.bType[0]));
			}

		}
		else
		{
			CUser* pUser = g_pMain->GetUserPtr(sCasterID);
			if (pUser && pUser->isGM())
				g_pMain->SendHelpDescription(pUser, string_format("[Skill] ID -> : %d || [Successrate] -> : %d || [Type] -> : %d", pSkill.iNum, pSkill.bSuccessRate, pSkill.bType[0]));
		}
	}
	/*25.05.2024*/

	if (pSkillTarget)
	{
		if (pSkillTarget->isNPC())
		{
			if (TO_NPC(pSkillTarget)->isDead())
			{

				return;
			}

			if ((pSkill.bMoral == MORAL_ENEMY || pSkill.bMoral == MORAL_AREA_ENEMY) && (pSkillCaster->isPlayer() && TO_NPC(pSkillTarget)->m_OrgNation == 3))
				return SendSkillFailed();
		}
	}

	if (pSkillCaster->isPlayer())
	{
		if ((pSkill.bMoral == MORAL_SELF && !bIsRunProc)
			|| (pSkill.bMoral == MORAL_FRIEND_WITHME && pSkillTarget && pSkillTarget->isNPC() && pSkillTarget != pSkillCaster))
		{
			pSkillTarget = pSkillCaster;
			sTargetID = pSkillCaster->GetID();
		}

		if (sTargetID == -1)
		{
			float mesafe = sqrtf(pSkillCaster->GetDistance(float(sData[0]), float(sData[2])));
			uint16 SkillRange = pSkill.sRange + 4;
			if (isStompSkills()) SkillRange += 5;


			bool noblock = (pSkill.t_1 != -1 && pSkill.t_1 != 0
				&& pSkill.bCastTime && bOpcode == (uint8)MagicOpcode::MAGIC_EFFECTING)
				|| bOpcode == (uint8)MagicOpcode::MAGIC_FAIL;

			if (!noblock && mesafe > (pSkill.iUseItem == 391010000 ? (float)500 : (float)SkillRange))
				return SendSkillFailed();
		}

		CUser* pUser = TO_USER(pSkillCaster);

		if (pSkill.bMoral == MORAL_SELF || pSkill.bMoral == MORAL_ENEMY || pSkill.bMoral == MORAL_AREA_ENEMY)
		{
			bool isnpc = false; uint16 protoid = 0;
			if (pSkillTarget && pSkillTarget->isNPC())
			{
				isnpc = true; protoid = TO_NPC(pSkillTarget)->GetProtoID();
			}

			if ((!pUser->isGM() && pUser->GetZoneID() == ZONE_JURAID_MOUNTAIN) && (!g_pMain->isJuraidMountainActive() || !pUser->isEventUser()))
				return SendSkillFailed();

			if (!pUser->isGM() && protoid == DEVA_BIRD_SSID && pUser->GetZoneID() == ZONE_JURAID_MOUNTAIN
				&& !pUser->CheckDevaAttack(isnpc, protoid))
				return SendSkillFailed();
		}

		if (pUser->isAttackDisabled())
			return SendSkillFailed();

		if (bOpcode == (uint8)MagicOpcode::MAGIC_CASTING || bOpcode == (uint8)MagicOpcode::MAGIC_FLYING
			|| bOpcode == (uint8)MagicOpcode::MAGIC_EFFECTING)
		{
			CastCheckType cast = CheckCastTime();
			if (cast == CastCheckType::NoValue)
				return;

			if (cast != CastCheckType::Add && cast != CastCheckType::Success)
				return SendSkillCasting();

			if (bOpcode == (uint8)MagicOpcode::MAGIC_FLYING)
			{
				bool caught = (pUser->pUserMagicUsed.lastmultiple == 1 && isArcher3liOK()) || (pUser->pUserMagicUsed.lastmultiple == 2 && isArcher5liOK());
				if (pUser->pUserMagicUsed.lastmultipletime > UNIXTIME2 && caught)
					return;

				if (isArcher3liOK() || isArcher5liOK())
				{
					if (isArcher3liOK())
						pUser->pUserMagicUsed.lastmultiple = 1;
					else
						pUser->pUserMagicUsed.lastmultiple = 2;
					pUser->pUserMagicUsed.lastmultipletime = UNIXTIME2 + 1150;
				}
			}
		}

		if (pSkillCaster->isPlayer() && AnimatedSkill())
		{
			bool returnfail = false;
			CUser* pUser = TO_USER(pSkillCaster);

			//printf("%s :opcode=%d,speed=%d\n", pSkill.krname.c_str(), bOpcode, pUser->m_sSpeed);
			if (pUser->pUserMagicUsed.bCastFailed && bOpcode != (uint8)MagicOpcode::MAGIC_CASTING)
				returnfail = true;
			else
				pUser->pUserMagicUsed.bCastFailed = false;

			if (!returnfail && bOpcode == (uint8)MagicOpcode::MAGIC_CASTING && pUser->m_sSpeed != 0)
				pUser->pUserMagicUsed.bCastFailed = true;

			bool pos_check = pUser->pUserMagicUsed.castX == pUser->GetSPosX() && pUser->pUserMagicUsed.castZ == pUser->GetSPosZ();
			if (!pos_check && pSkill.iNum == pUser->pUserMagicUsed.castID)
			{
				uint8 op_code = 0;
				if (pSkill.bFlyingEffect)
					op_code = (uint8)MagicOpcode::MAGIC_FLYING;
				else
					op_code = (uint8)MagicOpcode::MAGIC_EFFECTING;

				if (bOpcode == op_code)
					returnfail = true;
			}

			if (returnfail)
				return SendSkillFailed();
		}

		if (!isArcher3liOK() && !isArcher5liOK()
			&& bOpcode == (uint8)MagicOpcode::MAGIC_FAIL && pSkill.bType[0] == 2
			&& TO_USER(pSkillCaster)->isRogue() && TO_USER(pSkillCaster)->pUserMagicUsed.archerfailreturn)
		{

			TO_USER(pSkillCaster)->pUserMagicUsed.archerfailreturn = false;
			SendSkillNoFunction();
			return;
		}
	}

	if (pSkillCaster->isPlayer() && pSkillTarget)
	{
		bool checkout = false;
		if (TO_USER(pSkillCaster)->isGenieActive()
			&& pSkillTarget->isPlayer()
			&& pSkillCaster->GetNation() != pSkillTarget->GetNation()
			&& TO_USER(pSkillCaster)->isInPVPZone())
			return SendSkillFailed();

		if (TO_USER(pSkillCaster)->isGenieActive()
			&& pSkillTarget->isBot()
			&& pSkillCaster->GetNation() != pSkillTarget->GetNation()
			&& TO_USER(pSkillCaster)->isInPVPZone())
			return SendSkillFailed();

		if (bOpcode == (uint8)MagicOpcode::MAGIC_FAIL || bOpcode == (uint8)MagicOpcode::MAGIC_FLYING)
			checkout = true;

		if (!checkout)
		{
			float mesafe = pSkillCaster->GetDistanceSqrt(pSkillTarget);
			int SkillRange = pSkill.sRange;

			if ((pSkill.bType[0] == 1 || pSkill.bType[1] == 1) && pSkill.bCastTime == 0 && !isStaffSkill())
			{
				if (TO_USER(pSkillCaster)->m_sSpeed)
					SkillRange = 18;
				else
					SkillRange = 12;
			}
			else if (isDrainSkills())
				SkillRange += 5;
			else if (isStaffSkill() && TO_USER(pSkillCaster)->m_sSpeed)
				SkillRange += 17;
			else if (isStaffSkill() && !TO_USER(pSkillCaster)->m_sSpeed)
				SkillRange += 10;
			else
				SkillRange += 9;

			if (pSkillTarget->isNPC() && isDrainSkills() && (TO_NPC(pSkillTarget)->GetPID() == 6200))
				SkillRange = 37;
			else if (pSkill.bType[0] == 8 && pSkill.t_1 == 154 && pSkill.sRange == 1)
				SkillRange = 6;

			if (!isStaffSkill() && pSkill.t_1 != -1
				&& pSkill.t_1 != 0 && pSkill.bCastTime /*&& TO_USER(pSkillCaster)->isMage()*/
				&& bOpcode == (uint8)MagicOpcode::MAGIC_EFFECTING)
				SkillRange = pSkill.sRange * 2;

			if (pSkill.bMoral == SkillMoral::MORAL_PARTY && pSkill.bType[0] == 4
				&& pSkillTarget && pSkillCaster != pSkillTarget && pSkillTarget->isPlayer()
				&& TO_USER(pSkillTarget)->isInParty() && TO_USER(pSkillCaster)->isInParty()
				&& TO_USER(pSkillCaster)->GetPartyID() == TO_USER(pSkillTarget)->GetPartyID())
				checkout = true;

			if (!checkout && SkillRange && mesafe > (pSkill.iUseItem == 391010000 ? 55 : SkillRange))
			{
				if (!isStaffSkill())
					SendSkillFailed();

				return;
			}
		}
	}

	bool no_casting_failed = false;
	if (pSkillCaster->isPlayer()
		&& (MagicOpcode)bOpcode == MagicOpcode::MAGIC_CASTING
		&& TO_USER(pSkillCaster)->isPriest()
		&& TO_USER(pSkillCaster)->isGenieActive()
		&& !pSkill.pType4.isnull()
		&& pSkill.bIsBuff)
		no_casting_failed = true;

	if (pSkillCaster->isPlayer())
	{
		std::string str = "Massa";
		TRACE("%s", str.c_str());
	}

	bool Failed = false, NoFunction = false;

	if (!no_casting_failed)
	{
		SkillUseResult ab = CheckSkillPrerequisites();
		switch (ab)
		{
		case SkillUseResult::SkillUseFail:
			Failed = true;
			break;
		case SkillUseResult::SkillUseOnlyUse:
			return;
		case SkillUseResult::SkillUseNoFunction:
			NoFunction = true;
			break;
		}
	}

	if (Failed && !no_casting_failed)
		return SendSkillFailed();

	if (NoFunction && !no_casting_failed)
		return SendSkillNoFunction();

	if (!no_casting_failed)
	{
		Failed = NoFunction = false;
		SkillUseResult ac = UserCanCast();
		switch (ac)
		{
		case SkillUseResult::SkillUseFail:
			Failed = true;
			break;
		case SkillUseResult::SkillUseOnlyUse:
			return;
		case SkillUseResult::SkillUseNoFunction:
			NoFunction = true;
			break;
		}
	}

	if (Failed && !no_casting_failed)
		return SendSkillFailed();

	if (NoFunction && !no_casting_failed)
		return SendSkillNoFunction();

	switch ((MagicOpcode)bOpcode)
	{
	case MagicOpcode::MAGIC_CASTING:
	{
		/*if (pSkillCaster->isPlayer()) {
			CUser* pUser = TO_USER(pSkillCaster);
			if (pSkill.bCastTime) {
				pUser->pUserMagicUsed.status.bOpcode = MagicOpcode::MAGIC_CASTING;
				pUser->pUserMagicUsed.status.m_castskill = true;
				pUser->pUserMagicUsed.status.skillid = true;
			}
		}*/
		SendSkill();
	}
	break;
	case MagicOpcode::MAGIC_FAIL:
		SendSkill(false);
		break;
	case MagicOpcode::MAGIC_FLYING:
	{
		// Handle arrow & mana checking/removals.
		if (pSkillCaster && pSkillCaster->isPlayer())
		{
			CUser* pCaster = TO_USER(pSkillCaster);
			_MAGIC_TYPE2* pType = g_pMain->m_Magictype2Array.GetData(nSkillID);

			// NOTE: Not all skills that use MAGIC_FLYING are type 2 skills.
			// Some type 3 skills use it (such as "Vampiric Fire"). 
			// For these we should really apply the same flying logic, but for now we'll just ignore it.
			if (pType != nullptr)
			{
				// Throwing knives are differentiated by the fact "NeedArrow" is set to 0.
				// We still need to check for & take 1 throwing knife in this case however.
				uint8 bCount = pType->bNeedArrow;

				if (!bCount)
					bCount = 1;
				if ((!pCaster->CheckExistItem(pSkill.iUseItem, bCount))
					// Ensure user has enough mana for this skill
					|| pSkillCaster && pSkill.sMsp > pSkillCaster->GetMana()
					|| pSkill.sSp > pCaster->GetStamina())
				{
					if (!pCaster->CheckExistItem(ITEM_INFINITYARC) && pSkill.iUseItem == 391010000)
					{
						SendSkillFailed();
						return;
					}
				}


				if (TO_USER(pSkillCaster)->pUserMagicUsed.ArrowUseTime > UNIXTIME2)
				{
					pCaster->pUserMagicUsed.archerfailreturn = true;
					return;
				}

				if (pSkillCaster && pSkillCaster->isPlayer())
				{
					if (TO_USER(pSkillCaster)->isPortuKurian())
					{
						if (pSkill.sSp > TO_USER(pSkillCaster)->GetStamina())
						{
							SendSkillFailed();
							return;
						}
					}
				}

				pCaster->m_arrowLock.lock();
				for (size_t i = 0; i < bCount; i++)
					pCaster->m_flyingArrows.push_back(Arrow(pType->iNum, UNIXTIME2));

				pCaster->m_arrowLock.unlock();
				pCaster->RobItem(pSkill.iUseItem, bCount);
			}

			else if (pSkillCaster && pSkill.sMsp > pSkillCaster->GetMana())
			{
				SendSkillFailed();
				return;
			}
			if (pSkillCaster && pSkillCaster->isPlayer())
			{
				if (TO_USER(pSkillCaster)->isPortuKurian())
				{
					if (pSkill.sSp > TO_USER(pSkillCaster)->GetStamina())
					{
						SendSkillFailed();
						return;
					}
				}
			}

			if (!pCaster->isBlinking())
			{
				if (pSkill.bType[0] == 2)
					pCaster->MSpChange(-(pSkill.sMsp));

				if (pCaster->isPortuKurian())
					pCaster->SpChange(-(pSkill.sSp));
			}
		}
		if (pSkill.bType[0] != 2 && pSkill.bType[1] != 2)
			SendSkill(true); // send this to the region
	}
	break;
	case MagicOpcode::MAGIC_EFFECTING:
	{
		if (!bIsRecastingSavedMagic && (pSkill.bType[0] == 0 && pSkill.bType[1] != 0 && pSkill.iUseItem != 0 && (pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->CheckExistItem(pSkill.iUseItem))) && !TO_USER(pSkillCaster)->isInPKZone())
		{
			SendTransformationList();
			return;
		}

		if (pSkill.bType[0] == 0 && pSkill.iNum == 500127 && pSkillCaster->isPlayer())
		{
			CUser* pCaster = TO_USER(pSkillCaster);
			pCaster->MerchantEye();
			return;
		}

		if (pSkillCaster->isPlayer() && !bIsRecastingSavedMagic)
		{
			CUser* pMagicCaster = TO_USER(pSkillCaster);
			if (pMagicCaster->m_bResHpType == USER_SITDOWN && (pSkill.bMoral == 7 || pSkill.bMoral == 10))
				return SendSkillFailed();
		}

		if (!ExecuteSkill(pSkill.bType[0]))
			return;


		if (pSkillCaster->isPlayer())
		{
			CUser* pCaster = TO_USER(pSkillCaster);

			if (!pCaster)
				return;

			bool colldownadd = false;
			if (nSkillID == 110820 || nSkillID == 210820)
				colldownadd = true;

			bool is_buff = pSkillCaster->hasBuff((uint8)BuffType::BUFF_TYPE_INSTANT_MAGIC);

			if (!is_buff)
			{

				if (pSkill.bType[0] != 0)
				{

					if (pSkill.bType[0] == 3 && pSkill.t_1 == -1)
					{

						pCaster->m_MagicTypeCooldownListLock.lock();
						auto find = pCaster->m_MagicTypeCooldownList.find(10);
						if (find != pCaster->m_MagicTypeCooldownList.end())
							find->second.time = UNIXTIME2;
						else
							pCaster->m_MagicTypeCooldownList.emplace(std::make_pair(10, _type_cooldown(UNIXTIME2, false)));
						pCaster->m_MagicTypeCooldownListLock.unlock();
					}
					else
					{
						pCaster->m_MagicTypeCooldownListLock.lock();
						auto find = pCaster->m_MagicTypeCooldownList.find(pSkill.bType[0]);
						if (find != pCaster->m_MagicTypeCooldownList.end())
							find->second.time = UNIXTIME2;
						else
							pCaster->m_MagicTypeCooldownList.emplace(std::make_pair(pSkill.bType[0], _type_cooldown(UNIXTIME2, false)));
						pCaster->m_MagicTypeCooldownListLock.unlock();
					}
				}

				if (pSkill.bType[1] != 0)
				{
					pCaster->m_MagicTypeCooldownListLock.lock();
					auto find = pCaster->m_MagicTypeCooldownList.find(pSkill.bType[1]);
					if (find != pCaster->m_MagicTypeCooldownList.end())
						find->second.time = UNIXTIME2;
					else
						pCaster->m_MagicTypeCooldownList.emplace(std::make_pair(pSkill.bType[1], _type_cooldown(UNIXTIME2, false)));
					pCaster->m_MagicTypeCooldownListLock.unlock();
				}

				if (pSkill.sReCastTime)
					colldownadd = true;
			}

			bool is_minor = pSkill.iNum == 107705 || pSkill.iNum == 207705 || pSkill.iNum == 108705 || pSkill.iNum == 208705;
			if (colldownadd || is_minor)
			{

				uint16 recasttime = pSkill.sReCastTime;
				ULONGLONG time = recasttime * 90;

				if (is_minor)
				{
					if (GAME_SOURCE_VERSION == 1098)
						time = 50;
					else
						time = 90;
				}

				time += UNIXTIME2;
				pCaster->m_lockCoolDownList.lock();
				auto itr = pCaster->m_CoolDownList.find(nSkillID);
				if (itr == pCaster->m_CoolDownList.end())
					pCaster->m_CoolDownList.emplace(std::make_pair(nSkillID, _cooldown(time, UNIXTIME2)));
				else
				{
					itr->second.recasttime = time;
					itr->second.time = UNIXTIME2;
				}

				pCaster->m_lockCoolDownList.unlock();
			}
		}

		ExecuteSkill(pSkill.bType[1]);
		if (pSkill.bType[0] != 2) ConsumeItem();
		if (pSkill.bType[0] != 2 && bInstantCast) CMagicProcess::RemoveType4Buff((uint8)BuffType::BUFF_TYPE_INSTANT_MAGIC, pSkillCaster);
	}
	break;
	case MagicOpcode::MAGIC_CANCEL:
	case MagicOpcode::MAGIC_CANCEL2:
		Type3Cancel();	//Damage over Time skills. 
		Type4Cancel();	//Buffs
		Type6Cancel();	//Transformations
		Type9Cancel();	//Stealth, lupine etc.
		break;

	case MagicOpcode::MAGIC_TYPE4_EXTEND:
		Type4Extend();
		break;

	default:
		printf("MagicInstance Unhandled packets %d \n", bOpcode);
		TRACE("MagicInstance Unhandled packets %d \n", bOpcode);
		break;
	}
}

bool MagicInstance::CheckTbl()
{
	if (pSkill.isnull() || pSkillCaster == nullptr)
		return false;

	if ((pSkill.t_1 == -1 || pSkill.t_1 == 0) || pSkill.bItemGroup == 255 || !pSkill.bCastTime || isMageArmorSkill()) return true;
	//printf("[Magic Cast:0]= iNum=%d,krname=%s, bCastTime=%d, t_1=%d , strUserID=%s \n", pSkill.iNum, pSkill.krname.c_str(), pSkill.bCastTime, pSkill.t_1, TO_USER(pSkillCaster)->GetName().c_str());
	return false;
}

bool MagicInstance::AnimatedSkill()
{
	if (pSkill.isnull() || pSkillCaster == nullptr) return false;
	if ((pSkill.t_1 == -1 || pSkill.t_1 == 0) || pSkill.bItemGroup == 255 || !pSkill.bCastTime || isMageArmorSkill()) return false;
	return true;
}

CastCheckType MagicInstance::CheckCastTime()
{
	if (pSkill.isnull() || pSkillCaster == nullptr)
		return CastCheckType::Failed;

	if (!pSkill.bCastTime || !pSkillCaster->isPlayer())
		return CastCheckType::Success;

	if (pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isGenieActive()) // geine acikken el kaldırma kontrlu kapalı
		return CastCheckType::Success;

	CUser* pUser = TO_USER(pSkillCaster);

	MagicOpcode bOp = (MagicOpcode)bOpcode;
	if (bOp == MagicOpcode::MAGIC_CASTING)
	{
		auto itr = pUser->m_SkillCastList.find(nSkillID);
		if (itr != pUser->m_SkillCastList.end())
		{
			itr->second.time = UNIXTIME2;
			itr->second.castcheck = false;
			itr->second.flyfail = false;
		}
		else pUser->m_SkillCastList.insert(std::make_pair(nSkillID, _castlist(UNIXTIME2, false, false)));
		return CastCheckType::Add;
	}

	if (bOp == MagicOpcode::MAGIC_EFFECTING && pSkill.bType[0] == 2) return CastCheckType::Success;

	if (bOp == MagicOpcode::MAGIC_FLYING || bOp == MagicOpcode::MAGIC_EFFECTING)
	{
		auto itr = pUser->m_SkillCastList.find(nSkillID);
		if (pUser->m_SkillCastList.empty() || itr == pUser->m_SkillCastList.end())
		{
			if (!CheckTbl())
				return CastCheckType::NoValue;

			return CastCheckType::Success;
		}
		ULONGLONG diff = UNIXTIME2 - itr->second.time, time = itr->second.time;

		if (bOp == MagicOpcode::MAGIC_FLYING)
		{

			double katsayi = 88.30;  //uçma kaçma skillerinde çarpan sayısı düşürülürse tolerans artart
			if (pSkill.bCastTime > 49) katsayi = 80;

			if (pUser->GetFame() == COMMAND_CAPTAIN)
			{ //at üzerinde uçma kaçma skilleri
				int ar = 1050;
				if (pSkill.bType[0] == 2) ar = 400;
				if (pSkill.iNum == 490033) ar = 550;
				if (diff < ar)
				{
					//printf("FLYING FAIL CAPTAIN: iNum=%d - %I64d - %d : %d - userid=%s\n", pSkill.iNum, diff, pSkill.bCastTime, pSkill.bCastTime * 100, pSkillCaster->GetName().c_str());
					if (pSkill.sReCastTime > 0) SendSkillNoFunction();
					itr->second.flyfail = true;
					return CastCheckType::NoValue;
				}
			}
			else if (pUser->isMonsterTransformation())
			{
				if (pUser->m_sTransformID == 100) katsayi = 55;
				else if (pUser->m_sTransformID == 200) katsayi = 55;
				else if (pUser->m_sTransformID == 400) katsayi = 55;
				else if (pUser->m_sTransformID == 500) katsayi = 80;
				else if (pUser->m_sTransformID == 900) katsayi = 75;
				else if (pUser->m_sTransformID == 1000) katsayi = 35;
				else if (pUser->m_sTransformID == 1100) katsayi = 45;
				else if (pUser->m_sTransformID == 1200) katsayi = 90;
				else if (pUser->m_sTransformID == 1636) katsayi = 60;
				else if (pUser->m_sTransformID == 1631) katsayi = 40;
				else if (pUser->m_sTransformID == 1701) katsayi = 85;
				else if (pUser->m_sTransformID == 2010) katsayi = 55;
				else if (pUser->m_sTransformID == 2200) katsayi = 50;
				else if (pUser->m_sTransformID == 2400) katsayi = 55;
				else if (pUser->m_sTransformID == 3400) katsayi = 75;
				else if (pUser->m_sTransformID == 3700) katsayi = 45;

				if (katsayi > 11) katsayi -= 10;
				if (diff < (ULONGLONG)pSkill.bCastTime * katsayi || diff >(ULONGLONG)(pSkill.bCastTime * 100) + 900)
				{
					if (pSkill.sReCastTime > 0) SendSkillNoFunction();
					itr->second.flyfail = true;
					//printf("FLYING FAIL TRANS: id=%d, iNum=%d - %I64d - %d : %d - userid=%s\n", pUser->m_sTransformID, pSkill.iNum, diff, pSkill.bCastTime, pSkill.bCastTime * 100, pSkillCaster->GetName().c_str());
					return CastCheckType::NoValue;
				}
				//printf("0:tid=%d - iNum=%d - %I64d - %d : %d\n", pUser->m_sTransformID, pSkill.iNum, UNIXTIME64 - time, pSkill.bCastTime, pSkill.bCastTime * 100);
			}
			else
			{
				if (pSkill.bType[0] == 2 && pSkill.iUseItem == 391010000)
				{ // okçu bölümü
					if (CounterStrike()) diff += 50;
					if (diff < 312 || diff > 1250)
					{
						//printf("FLYING FAIL ARCHER: iNum=%d - %I64d - %d : %d - userid=%s\n", pSkill.iNum, diff, pSkill.bCastTime, pSkill.bCastTime * 100, pSkillCaster->GetName().c_str());
						if (pSkill.sReCastTime > 0) SendSkillNoFunction();
						itr->second.flyfail = true;
						return CastCheckType::NoValue;
					}
				}
				else
				{
					ULONGLONG ar = static_cast<ULONGLONG>(pSkill.bCastTime * katsayi);
					if (pSkill.iNum == 490033) ar = 500;
					if (diff < ar /*|| diff >(ULONGLONG)(pSkill.bCastTime * 100) + 500*/)
					{
						//printf("FLYING FAIL: iNum=%d - %I64d - %d : %d - userid=%s\n", pSkill.iNum, UNIXTIME2 - time, pSkill.bCastTime, pSkill.bCastTime * 100, pSkillCaster->GetName().c_str());
						if (pSkill.sReCastTime > 0) SendSkillNoFunction();
						itr->second.flyfail = true;
						return CastCheckType::NoValue;
					}
				}
			}
			itr->second.castcheck = true;
		}
		else if (bOp == MagicOpcode::MAGIC_EFFECTING)
		{
			double katsayi = 85.20;
			if (pSkill.bCastTime > 49) katsayi = 80;

			bool flyfail = itr->second.flyfail, castcheck = itr->second.castcheck;
			pUser->m_SkillCastList.erase(nSkillID);

			if (flyfail)
				return CastCheckType::NoValue;
			if (castcheck)
				return CastCheckType::Success;

			if (pUser->GetFame() == COMMAND_CAPTAIN)
			{
				int ar = 1050;
				if (pSkill.bType[0] == 2) ar = 675;
				else if (pSkill.iNum == 490033) ar = 550;
				else if (pSkill.iNum == 106781 || pSkill.iNum == 206781) ar = 600;
				if (diff < ar || diff > 1700)
				{
					//printf("EFFECT FAIL CAPTAIN: iNum=%d - %I64d - %d : %d - userid=%s\n", pSkill.iNum, UNIXTIME64 - time, pSkill.bCastTime, pSkill.bCastTime * 100, pSkillCaster->GetName().c_str());
					if (pSkill.sReCastTime > 0) SendSkillNoFunction();
					return CastCheckType::NoValue;
				}
			}
			else if (pUser->isMonsterTransformation())
			{

				if (pUser->m_sTransformID == 100) katsayi = 55;
				else if (pUser->m_sTransformID == 200) katsayi = 55;
				else if (pUser->m_sTransformID == 400) katsayi = 55;
				else if (pUser->m_sTransformID == 500) katsayi = 80;
				else if (pUser->m_sTransformID == 900) katsayi = 75;
				else if (pUser->m_sTransformID == 1000) katsayi = 35;
				else if (pUser->m_sTransformID == 1100) katsayi = 45;
				else if (pUser->m_sTransformID == 1200) katsayi = 90;
				else if (pUser->m_sTransformID == 1636) katsayi = 60;
				else if (pUser->m_sTransformID == 1631) katsayi = 40;
				else if (pUser->m_sTransformID == 1701) katsayi = 85;
				else if (pUser->m_sTransformID == 2010) katsayi = 55;
				else if (pUser->m_sTransformID == 2200) katsayi = 50;
				else if (pUser->m_sTransformID == 2400) katsayi = 55;
				else if (pUser->m_sTransformID == 3400) katsayi = 75;
				else if (pUser->m_sTransformID == 3700) katsayi = 45;

				if (katsayi > 11) katsayi -= 10;
				if (diff < (ULONGLONG)pSkill.bCastTime * katsayi || diff >(ULONGLONG)(pSkill.bCastTime * 100) + 900)
				{
					if (pSkill.sReCastTime > 0) SendSkillNoFunction();
					//printf("EFFECT FAIL TRANS: id=%d, iNum=%d - %I64d - %d : %d - userid=%s\n", pUser->m_sTransformID, pSkill.iNum, UNIXTIME2 - time, pSkill.bCastTime, pSkill.bCastTime * 100, pSkillCaster->GetName().c_str());
					return CastCheckType::NoValue;
				}
				//printf("1:tid=%d - iNum=%d - %I64d - %d : %d\n", pUser->m_sTransformID, pSkill.iNum, UNIXTIME64 - time, pSkill.bCastTime, pSkill.bCastTime * 100);
			}
			else
			{
				if (diff < (ULONGLONG)pSkill.bCastTime * katsayi/* || diff >(ULONGLONG)(pSkill.bCastTime * 100) + 800*/)
				{
					if (pSkill.sReCastTime > 0) SendSkillNoFunction();
					//printf("EFFECT FAIL: iNum=%d - %I64d - %d : %d - userid=%s\n", pSkill.iNum, UNIXTIME2 - time, pSkill.bCastTime, pSkill.bCastTime * 100, pSkillCaster->GetName().c_str());
					return CastCheckType::NoValue;
				}
			}
		}
	}
	return CastCheckType::Success;
}

SkillUseResult MagicInstance::UserCanCast()
{
	if (pSkill.isnull() || !pSkillCaster)
		return SkillUseResult::SkillUseFail;

	if (pSkillCaster->isBlinking() && !bIsRecastingSavedMagic)
		return SkillUseResult::SkillUseFail;


	if (pSkill.iNum >= 300000 && pSkill.iNum <= 400000)
	{
		if (!pSkillCaster->isNPC())
		{
			if (TO_USER(pSkillCaster)->GetZoneID() == ZONE_CHAOS_DUNGEON && nSkillID != 490221 || TO_USER(pSkillCaster)->GetZoneID() == ZONE_CHAOS_DUNGEON && nSkillID != 490332)
				return SkillUseResult::SkillUseFail;
		}
	}

	if (pSkillCaster->isInTempleEventZone() && !g_pMain->pTempleEvent.isAttackable && TO_USER(pSkillCaster)->isEventUser())
		return SkillUseResult::SkillUseFail;

	// We don't need to check anything as we're just canceling our buffs.
	if (bOpcode == (int8)MagicOpcode::MAGIC_CANCEL || bOpcode == (int8)MagicOpcode::MAGIC_CANCEL2
		// Also don't need to check anything if we're forwarding a fail packet.
		|| bOpcode == (int8)MagicOpcode::MAGIC_FAIL
		// Or are reapplying persistent buffs.
		|| bIsRecastingSavedMagic)
		return SkillUseResult::SkillUseOK;

	if (!pSkillCaster->canUseSkills() || (pSkillCaster->isDead() && pSkill.bType[0] != 5))
		return SkillUseResult::SkillUseFail;

	if (!pSkillCaster->GearSkills())
		return SkillUseResult::SkillUseFail;

	if ((pSkill.bMoral >= MORAL_AREA_ENEMY && pSkill.bMoral <= MORAL_SELF_AREA) && sTargetID != -1)
	{
		if (!bIsItemProc)
			return SkillUseResult::SkillUseFail;

		sTargetID = -1;
	}

	if (pSkillCaster->isPlayer())
	{
		if (pSkill.sSkill != 0 && !bIsRunProc)
		{
			if (pSkillCaster->GetZoneID() != ZONE_CHAOS_DUNGEON && pSkillCaster->GetZoneID() != ZONE_DUNGEON_DEFENCE && pSkillCaster->GetZoneID() != ZONE_KNIGHT_ROYALE)
			{
				if (TO_USER(pSkillCaster)->m_sClass != (pSkill.sSkill / 10) || pSkillCaster && pSkillCaster->GetLevel() < pSkill.sSkillLevel)
					return SkillUseResult::SkillUseFail;
			}
		}

		if (pSkillCaster->isInTempleEventZone() && !g_pMain->pTempleEvent.isAttackable && TO_USER(pSkillCaster)->isEventUser())
			return SkillUseResult::SkillUseFail;

		if (pSkillTarget != nullptr)
		{
			if (pSkillTarget->isNPC())
			{
				if (TO_NPC(pSkillTarget)->GetType() == NPC_TREE)
					return SkillUseResult::SkillUseFail;

				else if (TO_NPC(pSkillTarget)->GetType() == NPC_FOSIL)
					return SkillUseResult::SkillUseFail;

				else if (TO_NPC(pSkillTarget)->m_OrgNation == 3) // Skill Vurma Kapatıldı 27.09.2020
					return SkillUseResult::SkillUseFail;

				else if (TO_NPC(pSkillTarget)->GetType() == NPC_OBJECT_FLAG && TO_NPC(pSkillTarget)->GetProtoID() == 511)
					return SkillUseResult::SkillUseFail;

				else if (TO_NPC(pSkillTarget)->GetType() == NPC_REFUGEE)
					return SkillUseResult::SkillUseFail;

				else if (TO_NPC(pSkillTarget)->GetType() == NPC_PRISON)
					return SkillUseResult::SkillUseFail;

				else if (TO_NPC(pSkillTarget)->GetType() == (uint8)Nation::NONE &&
					TO_NPC(pSkillTarget)->GetType() == NPC_PARTNER_TYPE)
					return SkillUseResult::SkillUseFail;

				else if (TO_NPC(pSkillTarget)->GetType() == NPC_BORDER_MONUMENT)
					return SkillUseResult::SkillUseFail;

				else if (TO_NPC(pSkillTarget)->GetType() == NPC_DESTROYED_ARTIFACT || TO_NPC(pSkillTarget)->isCswDoors())
				{
					if (!TO_USER(pSkillCaster)->isInClan() || !g_pMain->isCswActive() || !g_pMain->isCswWarActive())
						return SkillUseResult::SkillUseFail;
					if (TO_USER(pSkillCaster)->GetClanID() == g_pMain->pSiegeWar.sMasterKnights)
						return SkillUseResult::SkillUseFail;
				}

				if (pSkillCaster->GetZoneID() == ZONE_UNDER_CASTLE && isUnderTheCastleDisableSkill())
					return SkillUseResult::SkillUseFail;
			}

			if (pSkillCaster->isPlayer())
			{
				if (pSkillCaster->isInTempleQuestEventZone() && (!pSkillCaster->isSameEventRoom(pSkillTarget) && TO_USER(pSkillCaster)->m_ismsevent))
					return SkillUseResult::SkillUseFail;
			}
		}

		if (pSkillCaster && TO_USER(pSkillCaster)->isTrading())
			return SkillUseResult::SkillUseFail;

		if (pSkillCaster && TO_USER(pSkillCaster)->isMerchanting())
			return SkillUseResult::SkillUseFail;

		if (pSkillCaster && TO_USER(pSkillCaster)->isStoreOpen())
			return SkillUseResult::SkillUseFail;

		if (pSkillCaster->isPlayer()
			&& bOpcode != (uint8)MagicOpcode::MAGIC_TYPE4_EXTEND
			&& bOpcode != (uint8)MagicOpcode::MAGIC_CANCEL
			&& bOpcode != (uint8)MagicOpcode::MAGIC_DURATION_EXPIRED
			&& bOpcode != (uint8)MagicOpcode::MAGIC_FAIL)
		{
			bool no_check = false;
			if ((pSkill.bType[0] == 2 || pSkill.bType[1] == 2) && bOpcode != (uint8)MagicOpcode::MAGIC_CASTING)
				no_check = true;

			if (!no_check && (pSkillCaster->GetMana() - pSkill.sMsp) < 0)
				return SkillUseResult::SkillUseFail;
		}

		if (pSkillCaster && pSkillCaster->isPlayer())
		{
			if (TO_USER(pSkillCaster)->isPortuKurian())
			{
				if ((TO_USER(pSkillCaster)->GetStamina() - pSkill.sSp) < 0)
					return SkillUseResult::SkillUseFail;
			}
		}

		if (pSkillCaster && pSkillCaster->GetZoneID() == ZONE_SNOW_BATTLE && g_pMain->m_byBattleOpen == SNOW_BATTLE && nSkillID != SNOW_EVENT_SKILL)
			return SkillUseResult::SkillUseFail;

		if (pSkillTarget != nullptr && pSkill.bMoral == MORAL_ENEMY && pSkill.bType[0] == 3 && pSkill.bType[1] == 0 && pSkillTarget->isDead())
		{
			_MAGIC_TYPE3* pType3 = g_pMain->m_Magictype3Array.GetData(pSkill.iNum);
			if (pType3 == nullptr)
				return SkillUseResult::SkillUseFail;

			if (pType3->bDirectType == 0 && pType3->sFirstDamage == 0 && pType3->sTimeDamage == 0)
			{
				bOpcode = (int8)MagicOpcode::MAGIC_EFFECTING;
				sData[1] = 1;
				SendSkill();
				return SkillUseResult::SkillUseOnlyUse;
			}
		}

		if (pSkill.bType[0] == 1 && pSkill.bType[1] == 0)
		{
			_MAGIC_TYPE1* pType1 = g_pMain->m_Magictype1Array.GetData(pSkill.iNum);

			if (pType1 == nullptr)
				return SkillUseResult::SkillUseFail;

			//if (pSkill && pSkill.sUseStanding == 0 && pSkill.sRange > 0 && !pSkillCaster->isInRangeSlow(pSkillTarget, float(pSkill.sRange)))
			//	return SkillUseResult::SkillUseFail;
		}

		if (pSkill.bMoral == MORAL_PARTY && pSkill.bType[0] == 3 && pSkill.bType[1] == 0)
		{
			if (pSkillTarget == nullptr)
				return SkillUseResult::SkillUseFail;

			_MAGIC_TYPE3* pType3 = g_pMain->m_Magictype3Array.GetData(pSkill.iNum);
			if (pType3 == nullptr)
				return SkillUseResult::SkillUseFail;

			if (pSkillTarget->GetID() == pSkillCaster->GetID())
				return SkillUseResult::SkillUseFail;

			//if (pSkill && pSkill.sUseStanding == 0 && pSkill.sRange > 0 && !pSkillCaster->isInRangeSlow(pSkillTarget, float(pSkill.sRange)))
			//	return SkillUseResult::SkillUseFail;
		}

		//if (pSkillCaster && nSkillID == 490214 && TO_USER(pSkillCaster)->isInPVPZone()) // Abyss Fire are disabled in PVP Zones
		//	return SkillUseResult::SkillUseFail;

		_MAGIC_TYPE5* pType5;

		uint8 bType = 0;
		uint16 sNeedStone = 0;

		if (pSkill.bType[0] == 5)
		{
			pType5 = g_pMain->m_Magictype5Array.GetData(pSkill.iNum);

			if (pType5)
			{
				bType = pType5->bType;
				sNeedStone = pType5->sNeedStone;
			}
		}

		if (pSkill.iUseItem != 391010000 && (pSkill.bType[0] != 2 && pSkill.bType[0] != 6)
			&& (pSkill.iUseItem != 0 && !TO_USER(bType == RESURRECTION ? pSkillTarget : pSkillCaster)->CanUseItem(pSkill.iUseItem, (bType == RESURRECTION ? sNeedStone : 1))))
		{

			if (pSkill.iUseItem == 370005000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isRogue() && isRogueCureSkills() && TO_USER(pSkillCaster)->CheckExistItem(ITEM_INFINITYCURE))
			{
				/*conti2nue*/
			}
			else if (pSkill.iUseItem == 379059000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isWarrior() && TO_USER(pSkillCaster)->CheckExistItem(479059000))
			{
				/*continue*/
			}
			else if (pSkill.iUseItem == 379060000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isRogue() && TO_USER(pSkillCaster)->CheckExistItem(479060000))
			{
				/*continue*/
			}
			else if (pSkill.iUseItem == 379061000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isMage() && TO_USER(pSkillCaster)->CheckExistItem(479061000))
			{
				/*continue*/
			}
			else if (pSkill.iUseItem == 379062000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isPriest() && TO_USER(pSkillCaster)->CheckExistItem(479062000))
			{
				/*continue*/
			}
			else
				return SkillUseResult::SkillUseFail;
		}

		if (pSkill.nBeforeAction >= (uint32)ClassType::ClassWarrior && pSkill.nBeforeAction <= (uint32)ClassType::ClassPriest)
			nConsumeItem = CLASS_STONE_BASE_ID + (pSkill.nBeforeAction * 1000);
		else if (pSkill.nBeforeAction != 0 && (pSkill.nBeforeAction == 379090000 || pSkill.nBeforeAction == 379093000))
			nConsumeItem = pSkill.iUseItem;
		else if (pSkill.nBeforeAction == 381001000)
			nConsumeItem = pSkill.nBeforeAction;
		else
			nConsumeItem = pSkill.iUseItem;

		if (pSkill.iUseItem != 391010000 && (pSkill.bType[0] != 2
			&& pSkill.bType[0] != 6)
			&& (pSkill.iUseItem != 0 && !TO_USER(pSkillCaster)->CanUseItem(nConsumeItem)) && bType != RESURRECTION)
		{

			if (pSkill.iUseItem == 370005000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isRogue() && isRogueCureSkills() && TO_USER(pSkillCaster)->CheckExistItem(ITEM_INFINITYCURE))
			{
				/*continue*/
			}
			else if (pSkill.iUseItem == 379059000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isWarrior() && TO_USER(pSkillCaster)->CheckExistItem(479059000))
			{
				/*continue*/
			}
			else if (pSkill.iUseItem == 379060000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isRogue() && TO_USER(pSkillCaster)->CheckExistItem(479060000))
			{
				/*continue*/
			}
			else if (pSkill.iUseItem == 379061000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isMage() && TO_USER(pSkillCaster)->CheckExistItem(479061000))
			{
				/*continue*/
			}
			else if (pSkill.iUseItem == 379062000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isPriest() && TO_USER(pSkillCaster)->CheckExistItem(479062000))
			{
				/*continue*/
			}
			else if (nConsumeItem == 379062000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isPriest() && TO_USER(pSkillCaster)->CheckExistItem(479062000))
			{
				/*continue*/
			}
			else if (nConsumeItem == 379059000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isWarrior() && TO_USER(pSkillCaster)->CheckExistItem(479059000))
			{
				/*continue*/
			}
			else if (nConsumeItem == 379060000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isRogue() && TO_USER(pSkillCaster)->CheckExistItem(479060000))
			{
				/*continue*/
			}
			else if (nConsumeItem == 379061000 && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isMage() && TO_USER(pSkillCaster)->CheckExistItem(479061000))
			{
				/*continue*/
			}
			else
				return SkillUseResult::SkillUseFail;
		}

#if (GAME_SOURCE_VERSION == 1098)
		if (pSkillCaster
			&& !TO_USER(pSkillCaster)->isGM()
			&& !TO_USER(pSkillCaster)->isMastered()
			&& pSkill.sEtc != 0
			&& !TO_USER(pSkillCaster)->CheckExistEvent(pSkill.sEtc, 2))
			return SkillUseResult::SkillUseFail;
#endif // (GAME_SOURCE_VERSION != 1098)

		/*if (bOpcode == (int8)MagicOpcode::MAGIC_CASTING && pSkill && pSkill.bType[0] < 4 && pSkillTarget != nullptr && pSkillCaster && !pSkillCaster->isInAttackRange(pSkillTarget, pSkill))
			return SkillUseResult::SkillUseFail;
		else*/ /*if (bOpcode == (int8)MagicOpcode::MAGIC_EFFECTING
			&& pSkill && (pSkill.bType[0] == 1 || pSkill.bType[0] == 2) && pSkillTarget != nullptr && !pSkillCaster->isInAttackRange(pSkillTarget, pSkill))
			return SkillUseResult::SkillUseFail;*/

		if (bOpcode == (uint8)MagicOpcode::MAGIC_CASTING
			&& pSkill.bType[0] < 4
			&& pSkillTarget != nullptr
			&& !pSkillCaster->isInAttackRange(pSkillTarget, pSkill))
			return SkillUseResult::SkillUseFail;
		else if (bOpcode == (uint8)MagicOpcode::MAGIC_EFFECTING
			&& (pSkill.bType[0] == 1 || pSkill.bType[0] == 2)
			&& pSkillTarget != nullptr
			&& !pSkillCaster->isInAttackRange(pSkillTarget, pSkill))
			return SkillUseResult::SkillUseFail;

		if (pSkill.iNum == 210004 || pSkill.iNum == 209004 || pSkill.iNum == 109004 || pSkill.iNum == 110004)
		{
			if (pSkillTarget == nullptr
				|| (pSkillTarget && pSkillCaster == pSkillTarget)
				|| pSkillTarget && pSkillTarget->isDead()
				|| pSkillTarget && pSkillTarget->isBlinking())
				return SkillUseResult::SkillUseFail;

			if (pSkillTarget && pSkillTarget->isNPC())
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster && pSkillCaster->isPlayer() && !TO_USER(pSkillCaster)->isInParty())
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->GetPartyID() != TO_USER(pSkillTarget)->GetPartyID())
				return SkillUseResult::SkillUseFail;
		}

		if (pSkill.iNum == 500306 || pSkill.iNum == 500038 || pSkill.iNum == 500106)
		{
			if (pSkillTarget == nullptr)
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster == nullptr)
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster == pSkillTarget || pSkillTarget->isDead() || pSkillTarget->isBlinking())
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster->GetZoneID() != pSkillTarget->GetZoneID()
				|| pSkillCaster->GetNation() != pSkillTarget->GetNation()
				|| pSkillCaster->GetZoneID() == ZONE_RONARK_LAND
				|| pSkillCaster->GetZoneID() == ZONE_ARDREAM
				|| pSkillCaster->GetZoneID() == ZONE_RONARK_LAND_BASE
				|| pSkillCaster->GetZoneID() == ZONE_BIFROST)
				return SkillUseResult::SkillUseFail;

			if (pSkillTarget->isNPC())
				return SkillUseResult::SkillUseFail;
		}

		if (pSkill.iNum == 490301
			|| pSkill.iNum == 490302
			|| pSkill.iNum == 490303
			|| pSkill.iNum == 490304
			|| pSkill.iNum == 490305
			|| pSkill.iNum == 490306
			|| pSkill.iNum == 490307)
		{
			if (pSkillCaster && pSkillCaster->isDead() || pSkillCaster && pSkillCaster->isBlinking())
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster && pSkillCaster->GetZoneID() != ZONE_BIFROST && pSkillCaster->GetZoneID() != ZONE_BATTLE2)
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster && !pSkillCaster->isPlayer())
				return SkillUseResult::SkillUseFail;
		}

		if (pSkill.iNum == 610090 || pSkill.iNum == 610091 || pSkill.iNum == 610100 || pSkill.iNum == 610124)
		{

			if (pSkillCaster && pSkillCaster == pSkillTarget || pSkillCaster && pSkillCaster->isDead() || pSkillCaster && pSkillCaster->isBlinking())
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster && pSkillCaster->GetNation() == (uint8)Nation::KARUS && pSkillCaster->GetZoneID() != ZONE_KARUS)
				return SkillUseResult::SkillUseFail;
			else if (pSkillCaster && pSkillCaster->GetNation() == (uint8)Nation::ELMORAD && pSkillCaster->GetZoneID() != ZONE_ELMORAD)
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster && pSkillCaster->GetLevel() < 60)
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster && !pSkillCaster->isPlayer())
				return SkillUseResult::SkillUseFail;
		}

		if (pSkill.bType[0] == 6)
		{
			auto* pType6 = g_pMain->m_Magictype6Array.GetData(pSkill.iNum);
			if (pType6 == nullptr)
				return SkillUseResult::SkillUseFail;

			if (pType6->bUserSkillUse == (uint8)TransformationSkillUse::TransformationSkillUseMonster)
			{
				if (!pSkillCaster->isTransformationMonsterInZone() /*|| pSkillCaster->isBuffed(true)*/)
					return SkillUseResult::SkillUseFail;
			}

			if (pType6->bUserSkillUse == (uint8)TransformationSkillUse::TransformationSkillUseSiege)
			{
				if (pSkillCaster->isBuffed(true))
					return SkillUseResult::SkillUseFail;

				if (pSkillCaster->GetZoneID() != ZONE_DELOS)
					return SkillUseResult::SkillUseFail;

				/*if (!g_pMain->pCswEvent.Started
					|| g_pMain->pCswEvent.Status != (uint8)CswOpStatus::CastellanWar)
					return SkillUseResult::Failed;*/
			}

			if (pType6->bUserSkillUse == (uint8)TransformationSkillUse::TransformationSkillMovingTower)
			{
				/*if (!g_pMain->pCswEvent.Started
					|| g_pMain->pCswEvent.Status != (uint8)CswOpStatus::CastellanWar)
					return SkillUseResult::Failed;*/

				if (pSkillCaster->isBuffed(true) || pSkillCaster->GetZoneID() != ZONE_DELOS)
					return SkillUseResult::SkillUseFail;
			}
		}

		_MAGIC_TYPE8* pType8;
		if (pSkill.bType[0] == 8)
		{
			pType8 = g_pMain->m_Magictype8Array.GetData(pSkill.iNum);
			if (pType8 == nullptr)
				return SkillUseResult::SkillUseFail;

			if (pSkillTarget != nullptr)
			{
				if (pSkillCaster && pSkillCaster->isPlayer())
				{
					if (pType8->bWarpType == 25)
					{
						if (pSkillTarget == pSkillCaster)
							return SkillUseResult::SkillUseFail;
					}

					if (pType8->sRadius > 0)
					{
						if (pType8->bWarpType == 25)
						{
							if (pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)pType8->sRadius / 2)
								return SkillUseResult::SkillUseFail;
						}
						else
						{
							if (pSkillCaster && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)pType8->sRadius)
								return SkillUseResult::SkillUseFail;
						}
					}
					else
					{
						if (pType8 && (pType8->iNum == 114509
							|| pType8->iNum == 115509
							|| pType8->iNum == 214509
							|| pType8->iNum == 215509
							|| pType8->iNum == 115570
							|| pType8->iNum == 215570))
						{
							if (pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)pSkill.sRange + 5)
								return SkillUseResult::SkillUseFail;
						}
						else
						{
							if (pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)pSkill.sRange)
								return SkillUseResult::SkillUseFail;
						}
					}
				}
			}
		}

		_MAGIC_TYPE9* pType9 = nullptr;

		if (pSkill.bType[0] == 9)
		{
			pType9 = g_pMain->m_Magictype9Array.GetData(pSkill.iNum);
			if (pType9 == nullptr)
				return SkillUseResult::SkillUseFail;

			if (pType9->bStateChange <= 2 && pSkillCaster && !pSkillCaster->canStealth() && pSkillCaster->GetZoneID() != ZONE_LOST_TEMPLE && pSkillCaster->GetZoneID() != ZONE_FORGOTTEN_TEMPLE)
				return SkillUseResult::SkillUseFail;

			if (pSkillCaster->isPlayer() && nSkillID != 208680 && nSkillID != 108680)
			{
				pSkillCaster->m_buff9lock.lock();
				Type9BuffMap buffMap = pSkillCaster->m_type9BuffMap;
				pSkillCaster->m_buff9lock.unlock();
				if (buffMap.find(pType9->bStateChange) != buffMap.end())
					return SkillUseResult::SkillUseFail;
			}
		}
	}

	// priest warrior asas range kontrolleri 31.05.2021
	//priest debug skill range
	if (TO_USER(pSkillCaster)->GetClass() == 212 || TO_USER(pSkillCaster)->GetClass() == 211 || TO_USER(pSkillCaster)->GetClass() == 204
		|| TO_USER(pSkillCaster)->GetClass() == 104 || TO_USER(pSkillCaster)->GetClass() == 111 || TO_USER(pSkillCaster)->GetClass() == 112)
	{
		if (nSkillID == 111703 || nSkillID == 211703 || nSkillID == 112703 || nSkillID == 212703
			|| nSkillID == 111709 || nSkillID == 211709 || nSkillID == 112709 || nSkillID == 212709
			|| nSkillID == 111715 || nSkillID == 211715 || nSkillID == 112715 || nSkillID == 212715
			|| nSkillID == 111724 || nSkillID == 211724 || nSkillID == 112724 || nSkillID == 212724
			|| nSkillID == 111727 || nSkillID == 211727 || nSkillID == 112727 || nSkillID == 212727
			|| nSkillID == 111730 || nSkillID == 211730 || nSkillID == 112730 || nSkillID == 212730
			|| nSkillID == 111736 || nSkillID == 211736 || nSkillID == 112736 || nSkillID == 212736
			|| nSkillID == 111745 || nSkillID == 211745 || nSkillID == 112745 || nSkillID == 212745
			|| nSkillID == 111760 || nSkillID == 211760 || nSkillID == 112760 || nSkillID == 212760
			|| nSkillID == 112771 || nSkillID == 212771)
		{
			if (pSkillTarget != nullptr && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)40)
				TO_USER(pSkillCaster)->priesterrorcount++;
			else
				TO_USER(pSkillCaster)->priesterrorcount = 0;

			if (TO_USER(pSkillCaster)->priesterrorcount >= 2)
				return SkillUseResult::SkillUseFail;
		}
	}

	//piest attack range skill
	if (TO_USER(pSkillCaster)->GetClass() == 212 || TO_USER(pSkillCaster)->GetClass() == 211 || TO_USER(pSkillCaster)->GetClass() == 204
		|| TO_USER(pSkillCaster)->GetClass() == 104 || TO_USER(pSkillCaster)->GetClass() == 111 || TO_USER(pSkillCaster)->GetClass() == 112)
	{

		if (pSkillTarget != nullptr && pSkill.bType[0] == 1 && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)19)
			return SkillUseResult::SkillUseFail;

	}

	if (pSkillCaster->isPlayer())
	{
		//warrior attack  range check
		if (TO_USER(pSkillCaster)->GetClass() == 201 || TO_USER(pSkillCaster)->GetClass() == 101 || TO_USER(pSkillCaster)->GetClass() == 205
			|| TO_USER(pSkillCaster)->GetClass() == 105 || TO_USER(pSkillCaster)->GetClass() == 206 || TO_USER(pSkillCaster)->GetClass() == 106)
		{

			if (nSkillID == 105725 || nSkillID == 205725 || nSkillID == 106725 || nSkillID == 206725
				|| nSkillID == 105735 || nSkillID == 205735 || nSkillID == 106735 || nSkillID == 206735
				|| nSkillID == 105760 || nSkillID == 205760 || nSkillID == 106760 || nSkillID == 206760
				|| nSkillID == 106775 || nSkillID == 206775)
			{
				//if (pSkill.bType[0] == 1 && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)50)
				//  return SkillUseFail;
			}
			else
			{
				if (pSkillTarget != nullptr && pSkill.bType[0] == 1 && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)19)
					return SkillUseResult::SkillUseFail;
			}
		}
	}

	//mage staf 42-72 skills range control
	if (TO_USER(pSkillCaster)->GetClass() == 203 || TO_USER(pSkillCaster)->GetClass() == 103 || TO_USER(pSkillCaster)->GetClass() == 209
		|| TO_USER(pSkillCaster)->GetClass() == 109 || TO_USER(pSkillCaster)->GetClass() == 210 || TO_USER(pSkillCaster)->GetClass() == 110)
	{
		if (nSkillID == 109542 || nSkillID == 209542 || nSkillID == 110542 || nSkillID == 210542
			|| nSkillID == 109642 || nSkillID == 209642 || nSkillID == 110642 || nSkillID == 210642
			|| nSkillID == 109742 || nSkillID == 209742 || nSkillID == 110742 || nSkillID == 210742
			|| nSkillID == 110572 || nSkillID == 210572 || nSkillID == 110672 || nSkillID == 210672
			|| nSkillID == 110772 || nSkillID == 210772)
		{
			if (pSkillTarget != nullptr && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)19)
				return SkillUseResult::SkillUseFail;
		}
	}

	if (TO_USER(pSkillCaster)->GetClass() == 102 || TO_USER(pSkillCaster)->GetClass() == 202 || TO_USER(pSkillCaster)->GetClass() == 107
		|| TO_USER(pSkillCaster)->GetClass() == 108 || TO_USER(pSkillCaster)->GetClass() == 207 || TO_USER(pSkillCaster)->GetClass() == 208)
	{
		//rogue attack skill range check
		if (pSkillTarget != nullptr && nSkillID != 208575 && nSkillID != 108575 && pSkill.bType[0] == 1 && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)23)//19
			return SkillUseResult::SkillUseFail;

		//blow arrow skill range check
		if (nSkillID == 208575 || nSkillID == 108575)
		{
			if (pSkillTarget != nullptr && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)17) // 20
				return SkillUseResult::SkillUseFail;
		}

		if (nSkillID == 108820 || nSkillID == 208820 || nSkillID == 108821 || nSkillID == 208821)
		{
			if (pSkillTarget != nullptr && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)19)
				return SkillUseResult::SkillUseFail;
		}

		//Source Marking
		if (nSkillID == 108815 || nSkillID == 208815)
		{
			if (pSkillTarget != nullptr && pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)34)
				return SkillUseResult::SkillUseFail;
		}
	}

	// priest warrior asas range kontrolleri 31.05.2021

	if (pSkillCaster->isNPC())
	{
		if (pSkillCaster->GetMana() - pSkill.sMsp < 0)
			return SkillUseResult::SkillUseFail;

		pSkillCaster->MSpChange(-(pSkill.sMsp));
	}

	if ((bOpcode == (int8)MagicOpcode::MAGIC_EFFECTING || bOpcode == (int8)MagicOpcode::MAGIC_CASTING) && !IsAvailable() && !bIsRunProc /*&& !pSkillCaster->isTransformed()*/)
		return SkillUseResult::SkillUseFail;

	if (bOpcode == (int8)MagicOpcode::MAGIC_EFFECTING && pSkillCaster->canInstantCast())
		bInstantCast = true;

	return SkillUseResult::SkillUseOK;
}

#pragma region MagicInstance::CheckSkillClass(int skillclass)
bool MagicInstance::CheckSkillClass(int iskillclass)
{

	CUser* pCaster = TO_USER(pSkillCaster);
	if (!pCaster) return false;

	switch (iskillclass)
	{
	case KARUWARRIOR:
	case ELMORWARRRIOR:
		if (!pCaster->isBeginnerWarrior()) return false;
		break;
	case BERSERKER:
	case BLADE:
		if (!pCaster->isNoviceWarrior()) return false;
		break;
	case GUARDIAN:
	case PROTECTOR:
		if (!pCaster->isMasteredWarrior()) return false;
		break;
	case KARUROGUE:
	case ELMOROGUE:
		if (!pCaster->isBeginnerRogue()) return false;
		break;
	case HUNTER:
	case RANGER:
		if (!pCaster->isNoviceRogue()) return false;
		break;
	case PENETRATOR:
	case ASSASSIN:
		if (!pCaster->isMasteredRogue()) return false;
		break;
	case KARUWIZARD:
	case ELMOWIZARD:
		if (!pCaster->isBeginnerMage()) return false;
		break;
	case SORSERER:
	case MAGE:
		if (!pCaster->isNoviceMage()) return false;
		break;
	case NECROMANCER:
	case ENCHANTER:
		if (!pCaster->isMasteredMage()) return false;
		break;
	case KARUPRIEST:
	case ELMOPRIEST:
		if (!pCaster->isBeginnerPriest()) return false;
		break;
	case SHAMAN:
	case CLERIC:
		if (!pCaster->isNovicePriest()) return false;
		break;
	case DARKPRIEST:
	case DRUID:
		if (!pCaster->isMasteredPriest()) return false;
		break;
	case KURIANSTARTER:
	case PORUTUSTARTER:
		if (!pCaster->isBeginnerKurianPortu()) return false;
		break;
	case KURIANNOVICE:
	case PORUTUNOVICE:
		if (!pCaster->isNoviceKurianPortu()) return false;
		break;
	case KURIANMASTER:
	case PORUTUMASTER:
		if (!pCaster->isMasteredKurianPortu()) return false;
		break;
	}
	return true;
}
#pragma endregion

SkillUseResult MagicInstance::CheckSkillPrerequisites()
{
	if (pSkill.isnull() || pSkillCaster == nullptr)
		return SkillUseResult::SkillUseFail;

	if (pSkill.iNum > 299999 && pSkill.iNum < 400000 && pSkill.sSkillLevel != 255)
	{
		if (pSkillCaster->isPlayer() && pSkill.iNum != 302166)
			return SkillUseResult::SkillUseFail;
	}

	if (pSkill.iNum > 399999 && pSkill.iNum < 500000)
	{
		if (pSkill.bMoral == MORAL_FRIEND_WITHME)
		{
			if (pSkillTarget == nullptr || pSkillCaster != pSkillTarget)
				return SkillUseResult::SkillUseFail;
		}
	}

	if (pSkillCaster->isPlayer()
		&& TO_USER(pSkillCaster)->isGenieActive()
		&& pSkillTarget && pSkillTarget->isNPC()
		&& TO_NPC(pSkillTarget)->isMonster())
	{
		g_pMain->m_AntiAfkListLock.lock();
		AntiAfkList copylist = g_pMain->m_AntiAfkList;
		g_pMain->m_AntiAfkListLock.unlock();

		if (copylist.empty() || std::find(copylist.begin(), copylist.end(), TO_NPC(pSkillTarget)->GetProtoID()) != copylist.end())
			return SkillUseResult::SkillUseFail;
	}

	/*bool aaaa = (pSkill.iNum == 500287 ||
		pSkill.iNum == 500610 ||
		pSkill.iNum == 500611 ||
		pSkill.iNum == 500612 ||
		pSkill.iNum == 510525 ||
		pSkill.iNum == 510530 ||
		pSkill.iNum == 510531 ||
		pSkill.iNum == 510532 ||
		pSkill.iNum == 510533) && !bIsRunProc;

	if (aaaa) return SkillUseResult::SkillUseFail;*/

	if (pSkill.iNum > 499999 && !pSkill.iUseItem && !pSkillCaster->isNPC() && pSkill.t_1 != 0 && pSkill.t_1 != -1)
		return SkillUseResult::SkillUseFail;

	if (pSkill.iNum == 492060)
		return SkillUseResult::SkillUseFail;
	if (pSkill.iNum == 490026 && pSkillCaster->isPlayer() && !TO_USER(pSkillCaster)->isPriest())
		return SkillUseResult::SkillUseFail;
	if (pSkillTarget && pSkillTarget->isNPC() && TO_NPC(pSkillTarget)->isGuard() && pSkillCaster->isInMoradon())
		return SkillUseResult::SkillUseFail;
	if (pSkill.iNum == 492063 && pSkillCaster->GetZoneID() != ZONE_BORDER_DEFENSE_WAR)
		return SkillUseResult::SkillUseFail;
	if (pSkill.iNum == 510544 || pSkill.iNum == 492023 || pSkill.iNum == 492024 || pSkill.iNum == 500271)
		return SkillUseResult::SkillUseFail;

	/*if (pSkill.sSkillLevel == 255 &&  (int)BuffType::BUFF_TYPE_VARIOUS_EFFECTS && !bIsItemProc)
		return SkillUseResult::SkillUseFail;*/

	if (pSkillCaster->isPlayer()
		&& pSkillCaster->isInTempleEventZone()
		&& (pSkill.bMoral == MORAL_ENEMY || pSkill.bMoral == MORAL_ALL || pSkill.bMoral == MORAL_AREA_ALL)
		&& !TO_USER(pSkillCaster)->virt_eventattack_check())
		return SkillUseResult::SkillUseFail;

	if (pSkillCaster->isPlayer()
		&& bOpcode != (int8)MagicOpcode::MAGIC_CANCEL
		&& bOpcode != (int8)MagicOpcode::MAGIC_CANCEL2
		&& bOpcode != (int8)MagicOpcode::MAGIC_CANCEL_TRANSFORMATION
		&& pSkill.iNum < 300000 && (TO_USER(pSkillCaster)->GetNation() != pSkill.iNum / 100000))
		return SkillUseResult::SkillUseFail;

	if (bOpcode != (uint8)MagicOpcode::MAGIC_FLYING && bOpcode != (uint8)MagicOpcode::MAGIC_EFFECTING)
	{
		if (bOpcode == (uint8)MagicOpcode::MAGIC_CASTING && pSkillTarget && (pSkill.sRange > 0 && pSkill.sUseStanding == 1 && !((pSkill.sRange + 10) >= pSkillCaster->GetDistanceSqrt(pSkillTarget))))
			return SkillUseResult::SkillUseFail;
		else
			return SkillUseResult::SkillUseOK;
	}

	if (pSkillCaster->isSiegeTransformation() && (pSkill.iNum == 490035 || pSkill.iNum == 490034))
		return SkillUseResult::SkillUseFail;

	if ((isKrowazWeaponSkills() || isDarkKnightWeaponSkill()) && !bIsItemProc)
		return SkillUseResult::SkillUseFail;

	if (pSkill.iNum == 492060 || (isKrowazWeaponSkills() && !bIsItemProc) || pSkill.iNum == 490026 && pSkillCaster->isPlayer() && !TO_USER(pSkillCaster)->isPriest())
		return SkillUseResult::SkillUseFail;

	bool Arrow = nSkillID == 107555 || nSkillID == 108555 || nSkillID == 207555 || nSkillID == 208555 || nSkillID == 107515 || nSkillID == 108515 || nSkillID == 208515 || nSkillID == 207515;

	//if (bOpcode != (int8)MagicOpcode::MAGIC_FLYING && bOpcode != (int8)MagicOpcode::MAGIC_EFFECTING)
	//{
	//	//if (bOpcode == (uint8)MagicOpcode::MAGIC_CASTING && pSkillTarget && (pSkill && pSkill.sRange > 0 && pSkill.sUseStanding == 1 && !(pSkillCaster && pSkillCaster->isInRangeSlow(pSkillTarget, (float)pSkill.sRange))))
	//	//	return SkillUseResult::SkillUseFail;
	//	//else if (!Arrow)
	//		return SkillUseResult::SkillUseOK;
	//}

	if (pSkillCaster->isPlayer())
	{
		CUser* pCaster = TO_USER(pSkillCaster);
		if (pCaster)
		{
			if (pSkill.sUseStanding == 1 && pCaster->m_sSpeed != 0)
				return SkillUseResult::SkillUseFail;

			{
				int iclass = pSkill.sSkill / 10;
				if (bOpcode != (int8)MagicOpcode::MAGIC_CANCEL
					&& bOpcode != (int8)MagicOpcode::MAGIC_CANCEL2
					&& bOpcode != (int8)MagicOpcode::MAGIC_CANCEL_TRANSFORMATION
					&& pSkill.sSkill && iclass && !CheckSkillClass(iclass))
					return SkillUseResult::SkillUseNoFunction;
			}

			if (pSkill.bType[0] == 1)
			{
				_MAGIC_TYPE1* pType1 = g_pMain->m_Magictype1Array.GetData(nSkillID);
				if (pType1 == nullptr)
					return SkillUseResult::SkillUseFail;
			}

			if (pSkill.bType[0] == 3 || pSkill.bType[1] == 3)
			{
				_MAGIC_TYPE3* pType = g_pMain->m_Magictype3Array.GetData(nSkillID);
				if (pType == nullptr)
					return SkillUseResult::SkillUseFail;

				if (pType->bAttribute == (uint8)AttributeType::AttributeNone && !isStompSkills())
				{
					if (pSkill.bType[0] != 0)
					{
						pCaster->m_MagicTypeCooldownListLock.lock();
						auto find = pCaster->m_MagicTypeCooldownList.find(pSkill.bType[0]);
						if (find != pCaster->m_MagicTypeCooldownList.end())
						{
							find->second.time = 0;
							find->second.t_catch = false;
						}
						pCaster->m_MagicTypeCooldownListLock.unlock();
					}

					if (pSkill.bType[1] != 0)
					{
						pCaster->m_MagicTypeCooldownListLock.lock();
						auto find = pCaster->m_MagicTypeCooldownList.find(pSkill.bType[1]);
						if (find != pCaster->m_MagicTypeCooldownList.end())
						{
							find->second.time = 0;
							find->second.t_catch = false;
						}
						pCaster->m_MagicTypeCooldownListLock.unlock();
					}
				}
			}

			_MAGIC_TYPE4* pType4 = nullptr;
			if (pSkill.bType[0] == 4 && pSkill.bType[1] == 0)
			{
				pType4 = g_pMain->m_Magictype4Array.GetData(nSkillID);
				if (pType4 == nullptr)
					return SkillUseResult::SkillUseFail;

				if (pType4->bBuffType == BUFF_TYPE_FISHING)
				{
					if (pSkillCaster && !pSkillCaster->isPlayer())
						return SkillUseResult::SkillUseFail;

					if (TO_USER(pSkillCaster)->GetPremium() == 7)
					{
						if (pType4->sSpecialAmount == 1 && TO_USER(pSkillCaster)->m_FlashDcBonus >= 100)
							return SkillUseResult::SkillUseFail;

						if (pType4->sSpecialAmount == 2 && TO_USER(pSkillCaster)->m_FlashExpBonus >= 100)
							return SkillUseResult::SkillUseFail;

						if (pType4->sSpecialAmount == 3 && TO_USER(pSkillCaster)->m_FlashWarBonus >= 10)
							return SkillUseResult::SkillUseFail;
					}
					else
					{
						if (pSkillCaster && TO_USER(pSkillCaster)->m_FlashDcBonus >= 100)
							return SkillUseResult::SkillUseFail;

						if (pSkillCaster && TO_USER(pSkillCaster)->m_FlashExpBonus >= 100)
							return SkillUseResult::SkillUseFail;

						if (pSkillCaster && TO_USER(pSkillCaster)->m_FlashWarBonus >= 10)
							return SkillUseResult::SkillUseFail;

						if (TO_USER(pSkillCaster)->GetPremium() != 10 && pType4->sSpecialAmount == 1)
							return SkillUseResult::SkillUseFail;

						if (pSkillCaster && TO_USER(pSkillCaster)->GetPremium() != 11 && pType4->sSpecialAmount == 2)
							return SkillUseResult::SkillUseFail;

						if (pSkillCaster && TO_USER(pSkillCaster)->GetPremium() != 12 && pType4->sSpecialAmount == 3)
							return SkillUseResult::SkillUseFail;
					}
				}

				if (pType4->bBuffType == (int)BuffType::BUFF_TYPE_HELL_FIRE_DRAGON && (!pSkillCaster->isNPC() || (pSkillCaster->isNPC() && TO_NPC(pSkillCaster)->GetProtoID() != 9850)))
					return SkillUseResult::SkillUseFail;
			}

			if ((pSkillTarget != nullptr && bOpcode != (int8)MagicOpcode::MAGIC_EFFECTING) || nSkillID == 108575 || nSkillID == 208575)
			{
				if ((pSkillCaster != pSkillTarget
					&& !pSkillCaster->isInAttackRange(pSkillTarget, pSkill))  // Skill Range Check
					|| (pCaster->isInEnemySafetyArea() && nSkillID < 400000) // Disable Skill in Safety Area 
					|| (pCaster->isInTempleEventZone() && !pCaster->isSameEventRoom(pSkillTarget)) // BDW CHAOS JR If EventRoom is not same disable Skill.
					|| (pCaster->isInTempleQuestEventZone() && !pCaster->isSameEventRoom(pSkillTarget))) // Monster Squard 81 / 82 / 83 If EventRoom is not same disable Skill.
					return SkillUseResult::SkillUseFail;
			}

			bool is_minor = pSkill.iNum == 107705 || pSkill.iNum == 207705 || pSkill.iNum == 108705 || pSkill.iNum == 208705;
			if (pSkill.bType[0] != 9
				&& (pSkill.sReCastTime || is_minor)
				&& !bIsRecastingSavedMagic
				&& bOpcode != (uint8)MagicOpcode::MAGIC_TYPE4_EXTEND
				&& bOpcode != (uint8)MagicOpcode::MAGIC_CANCEL
				&& bOpcode != (uint8)MagicOpcode::MAGIC_CANCEL2
				&& bOpcode != (uint8)MagicOpcode::MAGIC_FAIL)
			{

				pCaster->m_lockCoolDownList.lock();
				auto itr = pCaster->m_CoolDownList.find(nSkillID);
				if (itr != pCaster->m_CoolDownList.end())
				{


					if (itr->second.recasttime > UNIXTIME2)
					{
						pCaster->m_lockCoolDownList.unlock();

						if (is_minor)
							return SkillUseResult::SkillUseOnlyUse;
						else
							return SkillUseResult::SkillUseFail;
					}

					if ((pCaster->isWarrior() || pCaster->isPriest() || pCaster->GetZoneID() == ZONE_CHAOS_DUNGEON)
						&& pSkill.bType[0] == 1 && pSkill.bMoral == MORAL_ENEMY && !pSkill.bCastTime)
					{

						if (itr->second.time > 0 && pSkill.sReCastTime * 102 > (UNIXTIME2 - itr->second.time))
						{
							//itr->second.time = UNIXTIME2;
							pCaster->m_lockCoolDownList.unlock();
							return SkillUseResult::SkillUseNoFunction;
						}
					}

					itr->second.recasttime = itr->second.time = 0;
					//pCaster->m_CoolDownList.erase(nSkillID);
				}
				pCaster->m_lockCoolDownList.unlock();
			}

			if (pCaster->isRogue() && pSkill.iUseItem != 370007000 && pCaster->GetZoneID() != ZONE_CHAOS_DUNGEON) {
				if (pSkill.bType[0] == 2 || (pSkill.bType[0] == 1 && !isEskrimaSkills())) {
					_ITEM_TABLE pLeftHand = TO_USER(pSkillCaster)->GetItemPrototype(LEFTHAND);
					_ITEM_TABLE pRightHand = TO_USER(pSkillCaster)->GetItemPrototype(RIGHTHAND);

					if (pSkill.bType[0] == 2) {
						if ((pLeftHand.isnull() && pRightHand.isnull()) || ((!pLeftHand.isnull() && !pLeftHand.isBow()) || (!pRightHand.isnull() && !pRightHand.isBow())))
							return SkillUseResult::SkillUseFail;
					}
					else if (pSkill.bType[0] == 1 && !isEskrimaSkills())
					{
						if (isBlowArrowSkills()) {
							if ((pLeftHand.isnull() && pRightHand.isnull()) || ((!pLeftHand.isnull() && !pLeftHand.isBow()) || (!pRightHand.isnull() && !pRightHand.isBow())))
								return SkillUseResult::SkillUseFail;
						}
						else if ((pLeftHand.isnull() && pRightHand.isnull()) || ((!pLeftHand.isnull() && pLeftHand.isBow()) || (!pRightHand.isnull() && pRightHand.isBow())))
							return SkillUseResult::SkillUseFail;
					}
					if (!pLeftHand.isnull()) {
						if (pLeftHand.isDagger() && pSkill.bType[0] == 2)
							return SkillUseResult::SkillUseFail;

						if (!isBlowArrowSkills() && pLeftHand.isBow() && (pSkill.bType[0] == 1 || pSkill.bType[1] == 1) && pSkill.bCastTime == 0)
							return SkillUseResult::SkillUseFail;
					}
					if (!pRightHand.isnull()) {
						if (pRightHand.isDagger() && pSkill.bType[0] == 2)
							return SkillUseResult::SkillUseFail;
						if (!isBlowArrowSkills() && pRightHand.isBow() && (pSkill.bType[0] == 1 || pSkill.bType[1] == 1) && pSkill.bCastTime == 0)
							return SkillUseResult::SkillUseFail;
					}
				}

				if (bOpcode == (uint8)MagicOpcode::MAGIC_EFFECTING && pSkill.bType[0] == 2)
					return SkillUseResult::SkillUseOK;
			}

			if (pSkill.iNum > 400000
				&& pSkill.bType[0] == 3
				&& pSkill.bType[1] == 0
				&& pSkill.bMoral == MORAL_SELF
				&& pSkill.bItemGroup == 9)
			{
				_MAGIC_TYPE3* pType3 = g_pMain->m_Magictype3Array.GetData(pSkill.iNum);

				if (pType3 == nullptr)
					return SkillUseResult::SkillUseFail;

				// Skill mustn't do any damage or such.
				if ((pType3->bDirectType == 1 || pType3->bDirectType == 2) && pType3->sFirstDamage > 0)
				{
					//printf("%I64d\n", UNIXTIME2 - pCaster->t_timeLastPotionUse);
					if (pCaster && (UNIXTIME2 - pCaster->t_timeLastPotionUse) < PLAYER_POTION_REQUEST_INTERVAL)
						return SkillUseResult::SkillUseFail;
				}
			}

			if (pSkill.iNum == 208685 || pSkill.iNum == 108685)
				return SkillUseResult::SkillUseOK;

			/*if (pSkill && (pSkill.bType[0] == 2 || pSkill.bType[1] == 2)
				&& nSkillID < 400000 && (nSkillID != 107555 && nSkillID != 108555 && nSkillID != 207555 && nSkillID != 208555 && nSkillID != 107515 && nSkillID != 108515 && nSkillID != 208515 && nSkillID != 207515))
			{
				if (pCaster && UNIXTIME2 - pCaster->m_tLastArrowUse < 300)
				{
					DateTime time;
					g_pMain->WriteCheatLogFile(string_format("SkillHack - %d:%d:%d || %s Disconnected for SkillHack[Archer].\n", time.GetHour(), time.GetMinute(), time.GetSecond(), pCaster->GetName().c_str()));
					return SkillUseResult::SkillUseFail;
				}
			}*/

			bool validtype = pSkill.bType[0] == 1 || pSkill.bType[0] == 3 || pSkill.bType[0] == 4
				|| pSkill.bType[0] == 5 || pSkill.bType[0] == 6 || pSkill.bType[0] == 7;

			if (pSkillCaster->isPlayer()
				&& validtype
				&& nSkillID < 400000
				&& pSkill.bItemGroup != 255
				&& (MagicOpcode)bOpcode != MagicOpcode::MAGIC_FAIL)
			{

				pCaster->m_MagicTypeCooldownListLock.lock();
				auto itr = pCaster->m_MagicTypeCooldownList.find(pSkill.bType[0]);
				if (itr != pCaster->m_MagicTypeCooldownList.end())
				{
					bool existspeed = (pType4 && (BuffType)pType4->bBuffType == BuffType::BUFF_TYPE_ATTACK_SPEED) || (MagicOpcode)bOpcode == MagicOpcode::MAGIC_TYPE4_EXTEND;
					if (!existspeed && itr->second.time > 0)
					{

						int a_count = 550;
						if ((pSkill.bType[0] == 1 || pSkill.bType[1] == 1) && (pSkill.bCastTime == 0 && !isStaffSkill()))
						{
							a_count = 600;//ilk yakalanma
							if (itr->second.t_catch)
								a_count = 380;//yakalandıktan sonra skili serbest bırakma.
						}
						if (pSkill.bType[0] == 4 && !isStaffSkill() && !isMageArmorSkill())
						{
							a_count = 0;
						}
						ULONGLONG diff = 0;
						if (UNIXTIME2 > itr->second.time) diff = UNIXTIME2 - itr->second.time;

						if (isStaffSkill() && diff < (PLAYER_SKILL_REQUEST_INTERVAL))
						{
							pCaster->m_MagicTypeCooldownListLock.unlock();
							return SkillUseResult::SkillUseOnlyUse;
						}
						else if (!isMageArmorSkill() && diff < a_count)
						{
							pCaster->m_MagicTypeCooldownListLock.unlock();
							itr->second.t_catch = true;
							itr->second.time = UNIXTIME2;

							if (isStompSkills())
								return SkillUseResult::SkillUseNoFunction;

							return pCaster->isRogue() ? SkillUseResult::SkillUseNoFunction : SkillUseResult::SkillUseOnlyUse;
						}
					}

					itr->second.time = 0;
					itr->second.t_catch = false;
					//pCaster->m_MagicTypeCooldownList.erase(pSkill.bType[0]);
				}
				pCaster->m_MagicTypeCooldownListLock.unlock();
			}

			if (pSkillCaster->isPlayer()
				&& validtype
				&& nSkillID < 400000
				&& pSkill.bItemGroup != 255
				&& (MagicOpcode)bOpcode != MagicOpcode::MAGIC_FAIL)
			{

				pCaster->m_MagicTypeCooldownListLock.lock();
				auto itr = pCaster->m_MagicTypeCooldownList.find(pSkill.bType[1]);
				if (itr != pCaster->m_MagicTypeCooldownList.end())
				{

					if (itr->second.time > 0)
					{
						int a_count = 550;
						if ((pSkill.bType[0] == 1 || pSkill.bType[1] == 1) && (pSkill.bCastTime == 0 && !isStaffSkill()))
						{
							a_count = 600;
							if (itr->second.t_catch)
								a_count = 380;
						}
						if (pSkill.bType[1] == 4 && !isStaffSkill() && !isMageArmorSkill())
						{
							a_count = 0;
						}

						ULONGLONG diff = 0;
						if (UNIXTIME2 > itr->second.time) diff = UNIXTIME2 - itr->second.time;

						if (isStaffSkill() && diff < PLAYER_SKILL_REQUEST_INTERVAL)
						{
							pCaster->m_MagicTypeCooldownListLock.unlock();
							return SkillUseResult::SkillUseOnlyUse;
						}
						else if (!isMageArmorSkill() && diff < a_count)
						{
							pCaster->m_MagicTypeCooldownListLock.unlock();
							itr->second.t_catch = true;
							itr->second.time = UNIXTIME2;

							if (isStompSkills())
								return SkillUseResult::SkillUseNoFunction;

							return pCaster->isRogue() ? SkillUseResult::SkillUseNoFunction : SkillUseResult::SkillUseOnlyUse;
						}
					}

					itr->second.time = 0;
					itr->second.t_catch = false;
					//pCaster->m_MagicTypeCooldownList.erase(pSkill.bType[1]);
				}
				pCaster->m_MagicTypeCooldownListLock.unlock();
			}

			if (pCaster && pCaster->GetZoneID() != ZONE_DRAKI_TOWER)
			{
				if (pSkill.iNum == 492070 || pSkill.iNum == 492071)
					return SkillUseResult::SkillUseFail;
			}
		}
	}

	if (pSkillCaster->isNPC() && TO_NPC(pSkillCaster) != nullptr && TO_NPC(pSkillCaster)->isPet())
	{
		auto* pUser = g_pMain->GetUserPtr(TO_NPC(pSkillCaster)->GetPetID());
		if (pUser != nullptr)
		{
			if (pUser->m_PetSkillCooldownList.find(nSkillID) != pUser->m_PetSkillCooldownList.end() && pSkill.sReCastTime > 0)
			{
				PetSkillCooldownList::iterator itr = pUser->m_PetSkillCooldownList.find(nSkillID);
				int16_t DiffrentMilliseconds = int16_t((UNIXTIME)-itr->second);
				if (DiffrentMilliseconds > 0 && DiffrentMilliseconds < pSkill.sReCastTime / 3)
					return SkillUseResult::SkillUseFail;
				else
					pUser->m_PetSkillCooldownList.erase(nSkillID);
			}
		}
	}

	if (pSkillTarget != nullptr)
	{
		if (pSkillTarget->isNPC())
		{
			if (pSkillCaster && pSkillCaster->GetMap()->isWarZone())
			{
				if (TO_NPC(pSkillTarget)->GetType() == NPC_GATE && g_pMain->m_byBattleOpen != NATION_BATTLE)
					return SkillUseResult::SkillUseOnlyUse;

				if (TO_NPC(pSkillTarget)->GetType() == NPC_GATE && TO_NPC(pSkillTarget)->m_byGateOpen == 1 && g_pMain->m_byBattleOpen == NATION_BATTLE)
					return SkillUseResult::SkillUseOnlyUse;
			}

			if (pSkillCaster && pSkillCaster->GetZoneID() == ZONE_DELOS)
			{
				if (TO_NPC(pSkillTarget)->GetType() == NPC_GATE && g_pMain->m_byBattleOpen != SIEGE_BATTLE)
					return SkillUseResult::SkillUseOnlyUse;

				if (TO_NPC(pSkillTarget)->GetType() == NPC_GATE && TO_NPC(pSkillTarget)->m_byGateOpen == 1 && g_pMain->m_byBattleOpen == SIEGE_BATTLE)
					return SkillUseResult::SkillUseOnlyUse;
			}
		}

		if (pSkillTarget->isNPC() && pSkillCaster && !pSkillCaster->isAttackable(pSkillTarget))
			return SkillUseResult::SkillUseFail;
		if (pSkillTarget->isBlinking() && !bIsRecastingSavedMagic)
			return SkillUseResult::SkillUseFail;
		else
		{
			if (pSkillTarget->isPlayer() && TO_USER(pSkillTarget)->hasBuff(BUFF_TYPE_FREEZE))
				return SkillUseResult::SkillUseFail;
		}
	}

	return SkillUseResult::SkillUseOK;
}

/**
* @brief	Checks primary type 3 (DOT/HOT) skill prerequisites before executing the skill.
*
* @return	true if it succeeds, false if it fails.
*/
bool MagicInstance::CheckType3Prerequisites()
{
	_MAGIC_TYPE3* pType = g_pMain->m_Magictype3Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	if (!pSkillCaster)
		return false;

	// Handle AOE prerequisites
	if (sTargetID == -1)
	{
		// No need to handle any prerequisite logic for NPCs/mobs casting AOEs.
		if (!pSkillCaster->isPlayer())
			return true;

		// Player can attack other players in the safety area.
		if (TO_USER(pSkillCaster)->isInEnemySafetyArea())
			return false;

		if (pSkill.bMoral == MORAL_PARTY_ALL && pType->sTimeDamage > 0)
		{
			// Players may not cast group healing spells whilst transformed
			// into a monster (skills with IDs of 45###). 
			if (TO_USER(pSkillCaster)->isMonsterTransformation())
				return false;

			// Official behaviour means we cannot cast a group healing spell
			// if we currently have an active restoration spells on us.
			// This behaviour seems fairly illogical, but it's how it works.
			for (int i = 0; i < MAX_TYPE3_REPEAT; i++)
			{
				if (pSkillCaster->m_durationalSkills[i].m_sHPAmount > 0)
					return false;
			}
		}

		return true;
	}

	else if (sTargetID >= NPC_BAND)
	{
		if (pSkillTarget == nullptr)
			return false;

		if (pSkillCaster->GetZoneID() != 30 || (pType->sFirstDamage <= 0 && pType->sTimeDamage <= 0))
			return true;

		if (TO_NPC(pSkillTarget)->GetType() == NPC_GATE)
			return false;

		return true;
	}
	else
	{
		if (pSkill.bMoral > MORAL_PARTY)
			return true;

		if (pSkillTarget == nullptr)
			return false;

		if (pSkillTarget->isNPC() || pSkillTarget->isDead())
			return false;

		if (pType && pType->sTimeDamage > 0)
		{
			for (int i = 0; i < MAX_TYPE3_REPEAT; i++)
			{
				if (pSkillTarget->m_durationalSkills[i].m_sHPAmount > 0)
					return false;
			}
		}

		if (TO_USER(pSkillTarget)->isSiegeTransformation() && pSkillCaster && !pSkillCaster->CanAttack(pSkillTarget))
			return false;

		return true;
	}
}

/**
* @brief	Checks primary type 4 (buff/debuff) skill prerequisites before executing the skill.
*
* @return	true if it succeeds, false if it fails.
*/
bool MagicInstance::CheckType4Prerequisites()
{
	if (pSkill.isnull() || !pSkillCaster)
		return false;

	_MAGIC_TYPE4* pType = g_pMain->m_Magictype4Array.GetData(nSkillID);
	if (pType == nullptr)
		return (pSkill.bType[0] == 6);

	if (pType->bBuffType == BUFF_TYPE_FISHING)
	{
		if (pSkillCaster == nullptr || !pSkillCaster->isPlayer() || !TO_USER(pSkillCaster)->GetPremium())
			return false;

		if (pType->sSpecialAmount == 1
			&& (TO_USER(pSkillCaster)->GetPremium() == 10 || TO_USER(pSkillCaster)->GetPremium() == 7))
		{
			if (TO_USER(pSkillCaster)->m_FlashDcBonus >= 100)
				return false;

			return true;
		}
		else if (pType->sSpecialAmount == 2
			&& (TO_USER(pSkillCaster)->GetPremium() == 11 || TO_USER(pSkillCaster)->GetPremium() == 7))
		{
			if (TO_USER(pSkillCaster)->m_FlashExpBonus >= 100)
				return false;

			return true;
		}
		else if (pType->sSpecialAmount == 3
			&& (TO_USER(pSkillCaster)->GetPremium() == 12 || TO_USER(pSkillCaster)->GetPremium() == 7))
		{
			if (TO_USER(pSkillCaster)->m_FlashWarBonus >= 10)
				return false;

			return true;
		}
		return false;
	}

	if ((sTargetID < 0 || sTargetID >= MAX_USER))
	{
		if (sTargetID < NPC_BAND || pType->bBuffType != BUFF_TYPE_HP_MP || pType->sMaxHPPct != 99)
			return true;

		return false;
	}

	if (pSkillTarget == nullptr || !pSkillTarget->isPlayer())
		return false;


	//if (TO_USER(pSkillTarget)->isTransformed()) {
	//	// Can't use buff scrolls/pots when transformed into anything but NPCs.
	//	if (!TO_USER(pSkillTarget)->isNPCTransformation() && (nSkillID >= 500000 || pType->bBuffType == (uint8)BuffType::BUFF_TYPE_SIZE)) {
	//		SendSkillFailed();
	//		return false;
	//	}
	//}

	if (pType->bBuffType == (int)BuffType::BUFF_TYPE_HELL_FIRE_DRAGON && (!pSkillCaster->isNPC() || (pSkillCaster->isNPC() && TO_NPC(pSkillCaster)->GetProtoID() != 9850)))
		return false;

	if (pSkill.sSkillLevel == 255 && pType->bBuffType == (int)BuffType::BUFF_TYPE_VARIOUS_EFFECTS && !bIsItemProc)
		return false;

	if (pSkill.iUseItem == 0 && pType->bBuffType == (int)BuffType::BUFF_TYPE_VARIOUS_EFFECTS && pSkill.bMoral != MORAL_AREA_ALL && !bIsRunProc) return false;

	if (bOpcode != 1 && pType && pType->isBuff() && &pSkillTarget)
	{
		pSkillTarget->m_buffLock.lock();
		if (pSkillTarget->m_buffMap.find(pType->bBuffType) != pSkillTarget->m_buffMap.end())
		{
			pSkillTarget->m_buffLock.unlock();
			return false;
		}

		if (pSkillTarget->isPlayer())
		{
			if (pType->bBuffType == BUFF_TYPE_SPEED)
			{
				if (pSkillTarget->m_buffMap.find(BUFF_TYPE_SPEED2) != pSkillTarget->m_buffMap.end())
				{
					pSkillTarget->m_buffLock.unlock();
					return false;
				}
			}
		}
		pSkillTarget->m_buffLock.unlock();
	}

	return true;
}

bool MagicInstance::CheckType6Prerequisites()
{
	if (!pSkillCaster)
		return false;

	if (!pSkillCaster->isPlayer())
		return true;

	_MAGIC_TYPE6* pType = g_pMain->m_Magictype6Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	CUser* pCaster = TO_USER(pSkillCaster);

	if (!pCaster)
		return false;

	switch (pType->bUserSkillUse)
	{
	case TransformationSkillUseMonster:
	{
		uint32 itemid = 381001000;
		if (pType->iNum == 501560 || pType->iNum == 501561 || pType->iNum == 501562 || pType->iNum == 501563) itemid = pSkill.nBeforeAction;
		if ((!pCaster->CanUseItem(pSkill.nBeforeAction) || !pCaster->CanUseItem(pSkill.iUseItem)) && !pCaster->CheckExistItem(itemid))
			return false;
	}
	break;
	case TransformationSkillOreadsGuard:
		if (!pCaster->isPlayer())
			return false;
		break;

	default:
		if (!pCaster->CanUseItem(pSkill.iUseItem))
			return false;
		break;
	}

	bool bAllowedClass = (pType->sClass == 0);
	if (bAllowedClass)
		return true;

	switch (pCaster->GetBaseClassType())
	{
	case ClassType::ClassWarrior:
	case ClassType::ClassWarriorNovice:
	case ClassType::ClassWarriorMaster:
		bAllowedClass = ((pType->sClass / 1000)) == 1;
		break;

	case ClassType::ClassRogue:
	case ClassType::ClassRogueNovice:
	case ClassType::ClassRogueMaster:
		bAllowedClass = ((pType->sClass % 1000) / 100) == 1;
		break;

	case ClassType::ClassMage:
	case ClassType::ClassMageNovice:
	case ClassType::ClassMageMaster:
		bAllowedClass = (((pType->sClass % 1000) % 100) / 10) == 1;
		break;

	case ClassType::ClassPriest:
	case ClassType::ClassPriestNovice:
	case ClassType::ClassPriestMaster:
		bAllowedClass = (((pType->sClass % 1000) % 100) % 10) == 1;
		break;

	case ClassType::ClassPortuKurian:
	case ClassType::ClassPortuKurianNovice:
	case ClassType::ClassPortuKurianMaster:
		bAllowedClass = (((pType->sClass % 1000) % 100) % 10) == 1;
		break;
	}

	return bAllowedClass;
}

bool MagicInstance::ExecuteSkill(uint8 bType)
{
	if (!pSkillCaster)
		return false;

	if (bType == 0 || (pSkillCaster->isBlinking() && bType != 4 && pSkill.iNum < 300000))
		return false;

	// Implement player-specific logic before skills are executed.
	if (pSkillCaster->isPlayer())
	{
		if ((bType >= 1 && bType <= 3) || (bType == 7))
			TO_USER(pSkillCaster)->RemoveStealth();
	}

	switch (bType)
	{
	case 1: return ExecuteType1();
	case 2: return ExecuteType2();
	case 3: return ExecuteType3();
	case 4: return ExecuteType4();
	case 5: return ExecuteType5();
	case 6: return ExecuteType6();
	case 7: return ExecuteType7();
	case 8: return ExecuteType8();
	case 9: return ExecuteType9();
	}

	return false;
}

void MagicInstance::SendTransformationList()
{

	if (pSkillCaster == nullptr)
		return;

	if (!pSkillCaster->isPlayer())
		return;

	Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_TRANSFORM_LIST));
	result << nSkillID;
	TO_USER(pSkillCaster)->Send(&result);
}

/**
* @brief	Sends the skill failed packet to the caster.
*
* @param	sTargetID	Target ID to override with, as some use cases
* 						require.
*/
void MagicInstance::SendSkillFailed(int16 sTargetID)
{
	if (pSkillCaster == nullptr)
		return;

	Packet result;
	sData[3] = (bOpcode == (int8)MagicOpcode::MAGIC_CASTING ? (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_CASTING : (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_NOEFFECT);
	BuildSkillPacket(result, sCasterID, sTargetID == -1 ? this->sTargetID : sTargetID, (int8)MagicOpcode::MAGIC_FAIL, nSkillID, sData);

	// No need to proceed if we're not sending fail packets.
	if (!bSendFail || !pSkillCaster->isPlayer())
		return;

	TO_USER(pSkillCaster)->Send(&result);
}

void MagicInstance::SendSkillCasting(int16 sTargetID)
{
	if (pSkillCaster == nullptr)
		return;

	Packet result;
	sData[3] = (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_CASTING;
	BuildSkillPacket(result, sCasterID, sTargetID == -1 ? this->sTargetID : sTargetID, (uint8)MagicOpcode::MAGIC_FAIL, nSkillID, sData);

	// No need to proceed if we're not sending fail packets.
	if (!bSendFail || !pSkillCaster->isPlayer())
		return;

	TO_USER(pSkillCaster)->Send(&result);
}

void MagicInstance::SendSkillAttackZero(int16 sTargetID)
{
	if (pSkillCaster == nullptr)
		return;

	Packet result;
	sData[3] = (bOpcode == (uint8)MagicOpcode::MAGIC_CASTING ? (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_CASTING : (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_ATTACKZERO);
	BuildSkillPacket(result, sCasterID, sTargetID == -1 ? this->sTargetID : sTargetID, (uint8)MagicOpcode::MAGIC_FAIL, nSkillID, sData);

	// No need to proceed if we're not sending fail packets.
	if (!bSendFail || !pSkillCaster->isPlayer()) return;

	TO_USER(pSkillCaster)->Send(&result);
}

void MagicInstance::SendSkillNoFunction(int16 sTargetID)
{
	if (pSkillCaster == nullptr)
		return;

	Packet result;
	sData[3] = (bOpcode == (uint8)MagicOpcode::MAGIC_CASTING ? (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_CASTING : (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_ENDCOMBO);
	BuildSkillPacket(result, sCasterID, sTargetID == -1 ? this->sTargetID : sTargetID, (uint8)MagicOpcode::MAGIC_FAIL, nSkillID, sData);

	// No need to proceed if we're not sending fail packets.
	if (!bSendFail || !pSkillCaster->isPlayer())
		return;

	TO_USER(pSkillCaster)->Send(&result);
}

void MagicInstance::SendSkillNotEffect()
{
	if (pSkillCaster == nullptr)
		return;

	Packet result;
	sData[3] = (bOpcode == (int8)MagicOpcode::MAGIC_CASTING ? (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_CASTING : (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_NOEFFECT);
	BuildSkillPacket(result, sCasterID, sTargetID, (int8)MagicOpcode::MAGIC_EFFECTING, nSkillID, sData);

	// No need to proceed if we're not sending fail packets.
	if (!bSendFail || !pSkillCaster->isPlayer())
		return;

	TO_USER(pSkillCaster)->Send(&result);
}

/**
* @brief	Builds skill packet using the specified data.
*
* @param	result			Buffer to store the packet in.
* @param	sSkillCaster	Skill caster's ID.
* @param	sSkillTarget	Skill target's ID.
* @param	opcode			Skill system opcode.
* @param	nSkillID		Identifier for the skill.
* @param	sData			Array of additional misc skill data.
*/
void MagicInstance::BuildSkillPacket(Packet& result, int16 sSkillCaster, int16 sSkillTarget, int8 opcode, uint32 nSkillID, int16 sData[7])
{
	// On skill failure, flag the skill as failed.
	if (opcode == (int8)MagicOpcode::MAGIC_FAIL)
	{
		bSkillSuccessful = false;

		if (!bSendFail)
			return;
	}
	/*printf("%d :", opcode);
	for (int i = 0; i < 7; i++)printf(" %d",sData[i]);
	printf("\n");*/

	Unit* pUnit = g_pMain->GetUnitPtr(sSkillCaster, sSkillCasterZoneID);
	if (pUnit != nullptr)
	{
		if (pUnit->isBot())
		{
			if (!TO_BOT(pUnit)->isMage())
				sData[1] = 1;
		}
	}

	result.Initialize(WIZ_MAGIC_PROCESS);
	result << opcode << nSkillID << sSkillCaster << sSkillTarget << sData[0] << sData[1] << sData[2] << sData[3] << sData[4] << sData[5] << sData[6];
}

/**
* @brief	Builds and sends skill packet using the specified data to pUnit.
*
* @param	pUnit			The unit to send the packet to. If an NPC is specified,
* 							bSendToRegion is assumed true.
* @param	bSendToRegion	true to send the packet to pUser's region.
* @param	sSkillCaster	Skill caster's ID.
* @param	sSkillTarget	Skill target's ID.
* @param	opcode			Skill system opcode.
* @param	nSkillID		Identifier for the skill.
* @param	sData			Array of additional misc skill data.
*/
void MagicInstance::BuildAndSendSkillPacket(Unit* pUnit, bool bSendToRegion, int16 sSkillCaster, int16 sSkillTarget, int8 opcode, uint32 nSkillID, int16 sData[7])
{
	if (!pUnit)
		return;

	Packet result;
	BuildSkillPacket(result, sSkillCaster, sSkillTarget, opcode, nSkillID, sData);

	// No need to proceed if we're not sending fail packets.
	if (opcode == (int8)MagicOpcode::MAGIC_FAIL && !bSendFail)
		return;

	if (/*bSendToRegion || */!pUnit->isPlayer())
		pUnit->SendToRegion(&result);
	else
		TO_USER(pUnit)->SendToRegion(&result, nullptr, pUnit->GetEventRoom());
}

/**
* @brief	Sends the skill data in the current context to pUnit.
* 			If pUnit is nullptr, it will assume the caster.
*
* @param	bSendToRegion	true to send the packet to pUnit's region.
* 							If pUnit is an NPC, this is assumed true.
* @param	pUser		 	The unit to send the packet to.
* 							If pUnit is an NPC, it will send to pUnit's region regardless.
*/
void MagicInstance::SendSkill(bool bSendToRegion, Unit* pUnit)
{
	// If pUnit is nullptr, it will assume the caster.
	if (pUnit == nullptr)
		pUnit = pSkillCaster;

	// Build the skill packet using the skill data in the current context.
	BuildAndSendSkillPacket(pUnit, bSendToRegion,
		this->sCasterID, this->sTargetID, this->bOpcode, this->nSkillID, this->sData);
}

bool MagicInstance::isSkillClassAvailable(uint32 nSkillID)
{
	switch (nSkillID)
	{
	case 190573:
	case 290573:
		return true;
		break;
	case 190673:
	case 290673:
		return true;
		break;
	case 190773:
	case 290773:
		return true;
		break;
	case 188566:
	case 288566:
		return true;
		break;
	case 189742:
	case 190742:
	case 289742:
	case 290742:
		return true;
		break;
	case 189642:
	case 190642:
	case 289642:
	case 290642:
		return true;
		break;
	case 194509:
	case 195509:
	case 294509:
	case 295509:
		return true;
		break;
	}

	return false;
}

bool MagicInstance::IsAvailable()
{
	int modulator = 0, Class = 0, skill_mod = 0;

	if (!pSkillCaster)
		return false;

	if (pSkill.isnull() || (pSkillCaster->isInTempleEventZone() && !g_pMain->pTempleEvent.isAttackable))
		goto fail_return;

	switch (pSkill.bMoral)
	{
	case MORAL_SELF:
		if (nSkillID == 492032)
		{
			if (!pSkillCaster->isPlayer())
				goto fail_return;
		}
		else
			if (pSkillTarget && pSkillCaster->isPlayer() && pSkillCaster != pSkillTarget)
				goto fail_return;
		break;

	case MORAL_FRIEND_WITHME:
		if (pSkillTarget && pSkillTarget != pSkillCaster)
		{
			if (pSkillCaster->isMoral2Checking(pSkillTarget, pSkill))
			{
				bool SlavePriestControl = false; //17.06.2024
				if (pSkillTarget && pSkillTarget->isPlayer())
				{
					if (TO_USER(pSkillTarget)->m_bUserPriestBotID != -1)
						SlavePriestControl = true;
				}
				if (SlavePriestControl == false)
				{
					pSkillTarget = pSkillCaster;
					sTargetID = pSkillCaster->GetID();
				}
			}
		}
		break;

	case MORAL_FRIEND_EXCEPTME:
		if (pSkillTarget && pSkillCaster == pSkillTarget || pSkillCaster->isHostileTo(pSkillTarget))
			goto fail_return;
		break;

	case MORAL_PARTY:
	{
		if (pSkillCaster->isNPC() || (pSkillTarget != nullptr && pSkillTarget->isNPC()))
			goto fail_return;

		if (pSkillCaster->isBot() || (pSkillTarget != nullptr && pSkillTarget->isBot()))
			return true;

		CUser* pCaster = TO_USER(pSkillCaster);

		if (!pCaster)
			return false;

		if (pSkill.bType[0] != 4)
		{
			if (pSkillTarget == nullptr)
				return false;

			if (!pCaster->isInParty() || !TO_USER(pSkillTarget)->isInParty() || pSkillTarget == pCaster || TO_USER(pSkillTarget)->GetPartyID() != pCaster->GetPartyID())
				return false;
		}
		else
		{
			if ((!pCaster->isInParty() && pSkillCaster != pSkillTarget) || (pSkillTarget != nullptr && TO_USER(pSkillTarget)->isInParty() && TO_USER(pSkillTarget)->GetPartyID() != pCaster->GetPartyID()))
				goto fail_return;
		}
	}
	break;
	case MORAL_NPC:
		if (pSkillTarget == nullptr || !pSkillCaster->isHostileTo(pSkillTarget))
			goto fail_return;

		if (nSkillID == 490214)
		{
			if (pSkillTarget == nullptr || !pSkillTarget->isNPC() || pSkillCaster->isHostileTo(pSkillTarget) && pSkill.iUseItem != 379105000)
				goto fail_return;
		}
		break;
	case MORAL_ENEMY:
		// Allow for archery skills with no defined targets (e.g. an arrow from 'multiple shot' not hitting any targets). 
		// These should be ignored, not fail.
		if (pSkillTarget != nullptr && pSkillCaster && !pSkillCaster->isHostileTo(pSkillTarget))
			goto fail_return;
		break;

	case MORAL_CORPSE_FRIEND:
		// We need to revive *something*.
		if (pSkillTarget == nullptr
			|| pSkillCaster && pSkillCaster->isHostileTo(pSkillTarget)
			|| pSkillCaster && pSkillCaster == pSkillTarget
			|| pSkillTarget && pSkillTarget->isAlive())
			goto fail_return;
		break;

	case MORAL_CLAN:
	{
		// NPCs cannot be in clans.
		if (pSkillCaster && pSkillCaster->isNPC() || (pSkillTarget != nullptr && pSkillTarget->isNPC()))
			goto fail_return;

		// We're definitely a user, so....
		CUser* pCaster = TO_USER(pSkillCaster);

		if (!pCaster)
			return false;

		if ((!pCaster->isInClan() && pSkillCaster != pSkillTarget) || (pSkillTarget != nullptr && TO_USER(pSkillTarget)->GetClanID() != pCaster->GetClanID()))
			goto fail_return;
	}
	break;

	case MORAL_SIEGE_WEAPON:
		if (pSkillCaster && pSkillCaster->isNPC() || (pSkillCaster && pSkillCaster->isPlayer() && !TO_USER(pSkillCaster)->isSiegeTransformation()))
			goto fail_return;
		break;

	case MORAL_DRAWBRIDGE:  // CSW Ramkey hilesi fixlendi. CSW Kapısını kırması için kontrol eklendi.
		if (pSkillTarget && !pSkillTarget->isNPC() || (pSkillCaster && pSkillCaster->isPlayer() && !TO_USER(pSkillCaster)->isSiegeTransformation()) || pSkillCaster && pSkillCaster->GetZoneID() != ZONE_DELOS)
			goto fail_return;

		if (pSkillTarget && pSkillTarget->isNPC() && TO_NPC(pSkillTarget)->GetType() != NPC_GATE)
			goto fail_return;

		if (pSkillTarget && pSkillTarget->isNPC() && TO_NPC(pSkillTarget)->GetProtoID() != 561 && TO_NPC(pSkillTarget)->GetProtoID() != 562 && TO_NPC(pSkillTarget)->GetProtoID() != 563) 
			goto fail_return;

		break;
	}

	// Check skill prerequisites
	for (int i = 0; i < 2; i++)
	{
		switch (pSkill.bType[i])
		{
		case 3:
			if (!CheckType3Prerequisites())
				return false;
			break;

		case 4:
			if (!CheckType4Prerequisites())
				return false;
			break;

		case 6:
			if (!CheckType6Prerequisites())
				return false;
			break;
		}
	}

	// This only applies to users casting skills. NPCs are fine and dandy (we can trust THEM at least).
	if (pSkillCaster && pSkillCaster->isPlayer())
	{
		if (pSkill.bType[0] == 3)
		{
			_MAGIC_TYPE3* pType3 = g_pMain->m_Magictype3Array.GetData(pSkill.iNum);
			if (pType3 == nullptr)
				goto fail_return;

			auto pTable = g_pMain->GetItemPtr(pSkill.iUseItem);
			// Allow for skills that block potion use.
			if (!pSkillCaster->canUsePotions() && pType3->bDirectType == 1 && pType3->sFirstDamage > 0 && pSkill.iUseItem != 0 && !pTable.isnull() && pTable.m_bClass == 0)
				goto fail_return;
		}

		modulator = pSkill.sSkill % 10;  // Hacking prevention!
		if (modulator != 0)
		{
			_MAGIC_TABLE* pMagicT = nullptr;
			pMagicT = g_pMain->m_MagictableArray.GetData(pSkill.iNum);

			if (pMagicT == nullptr)
				return false;

			uint32 skill_packet = pMagicT->iNum;
			if (isSkillClassAvailable(pSkill.iNum))
				skill_packet = pMagicT->iNum - SKILLPACKET;

			pMagicT = g_pMain->m_MagictableArray.GetData(skill_packet);

			if (pMagicT == nullptr)
				return false;

			Class = pMagicT->sSkill / 10;
			if (pSkillCaster && Class != TO_USER(pSkillCaster)->GetClass())
				goto fail_return;

			if (pSkillCaster && (pMagicT->sSkillLevel > TO_USER(pSkillCaster)->m_bstrSkill[modulator]) && TO_USER(pSkillCaster)->GetFame() != COMMAND_CAPTAIN)
				goto fail_return;
		}

		else if (pSkill.sSkillLevel > pSkillCaster->GetLevel() && pSkill.sSkillLevel != 30010 && pSkill.sSkillLevel != 30050)
			goto fail_return;

		if (pSkill.bType[0] == 1)
		{
			if (pSkill.sSkill == 1055 || pSkill.sSkill == 2055)
			{
				if (pSkillCaster && TO_USER(pSkillCaster)->isWeaponsDisabled())
					return false;

				_ITEM_TABLE	pLeftHand = TO_USER(pSkillCaster)->GetItemPrototype(LEFTHAND);
				_ITEM_TABLE	pRightHand = TO_USER(pSkillCaster)->GetItemPrototype(RIGHTHAND);

				if (!pLeftHand.isnull() && !pLeftHand.isSword() && !pLeftHand.isAxe() && !pLeftHand.isMace() && !pLeftHand.isSpear() && !pLeftHand.isMace() && !pLeftHand.isShield() && !pLeftHand.isClub())
					return false;

				if (!pRightHand.isnull() && !pRightHand.isSword() && !pRightHand.isAxe() && !pRightHand.isMace() && !pRightHand.isSpear() && !pRightHand.isMace() && !pRightHand.isShield() && !pRightHand.isClub())
					return false;
			}
			else if (pSkill.sSkill == 1056 || pSkill.sSkill == 2056)
			{	// Weapons verification in case of 2 handed attacks !
				if (pSkillCaster && TO_USER(pSkillCaster)->isWeaponsDisabled())
					return false;

				_ITEM_TABLE	pRightHand = TO_USER(pSkillCaster)->GetItemPrototype(RIGHTHAND);

				if (pSkillCaster && TO_USER(pSkillCaster)->GetItem(LEFTHAND)->nNum != 0 || (!pRightHand.isnull() && !pRightHand.isSword() && !pRightHand.isAxe() && !pRightHand.isMace() && !pRightHand.isSpear() && !pRightHand.isMace() && !pRightHand.isShield()))
					return false;
			}
		}

		// Handle MP/HP/item loss.
		if (bOpcode == (int8)MagicOpcode::MAGIC_EFFECTING)
		{
			CUser* pCaster = TO_USER(pSkillCaster);

			if (pCaster == nullptr)
				return false;

			int total_hit = pSkillCaster->m_sTotalHit;

			if (pSkill.bType[0] == 2 && pSkill.bFlyingEffect != 0) // Type 2 related...
				return true;		// Do not reduce MP/SP when flying effect is not 0.

			if (pSkill.sMsp > pSkillCaster->GetMana())
				goto fail_return;

			if (pSkillCaster->isPlayer())
			{
				if (TO_USER(pSkillCaster)->isPortuKurian() && pSkill.sSp > TO_USER(pSkillCaster)->GetStamina())
					goto fail_return;
			}

			// If the PLAYER uses an item to cast a spell.
			if (pSkillCaster->isPlayer() && (pSkill.bType[0] == 3 || pSkill.bType[0] == 4))
			{
				if (pSkill.iUseItem != 0)
				{
					auto pItem = g_pMain->GetItemPtr(pSkill.iUseItem);
					if (pItem.isnull())
						return false;

					if ((pItem.m_bClass != 0 && !TO_USER(pSkillCaster)->JobGroupCheck(pItem.m_bClass)) || (pItem.m_bReqLevel != 0 && TO_USER(pSkillCaster)->GetLevel() < pItem.m_bReqLevel))
						return false;
				}
			}

			if ((pSkill.bType[0] != 4 || (pSkill.bType[0] == 4 && sTargetID == -1)) && pSkillCaster && !pSkillCaster->isBlinking())
			{
				CUser* pSkilled = TO_USER(pSkillCaster);

				if (!pSkilled)
					return false;

				if (pSkilled->isDevil() && pSkilled->CheckSkillPoint(PRO_SKILL3, 45, 83) && pSkilled->isPlayer())
				{
					int16 nDivide = pSkill.sMsp / 2;
					pSkillCaster->MSpChange(-(nDivide));
				}
				else
					pSkillCaster->MSpChange(-(pSkill.sMsp));

				if (pCaster != nullptr)
				{
					if (pCaster->isPortuKurian())
						pCaster->SpChange(-(pSkill.sSp));
				}
			}

			// Skills that require HP rather than MP.
			if (pSkill.sHP > 0 && pSkill.sMsp == 0 && pSkill.sHP < 10000)
			{
				if (pSkillCaster && pSkill.sHP > pSkillCaster->GetHealth())
					goto fail_return;

				if (pSkillCaster)
					pSkillCaster->HpChange(-pSkill.sHP);
			}
			if (pSkill.sHP >= 10000)
			{
				// Can't cast this on ourself.
				if (pSkillCaster == pSkillTarget)
					return false;

				if (pSkillCaster)
					pSkillCaster->HpChange(-10000);
			}
		}
	}
	return true;

fail_return:
	//SendSkillFailed();
	return false;
}

bool MagicInstance::ExecuteType1()
{
	if (!pSkillCaster || pSkill.isnull())
		return false;

	_MAGIC_TYPE1* pType = g_pMain->m_Magictype1Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	int damage = 0;
	bool bResult = false;

	if (pSkillTarget != nullptr)
	{
		if ((pSkillTarget->GetEventRoom() != pSkillCaster->GetEventRoom()))
			return false;

		if (pSkillTarget->isDead() || pSkillTarget->GetZoneID() != pSkillCaster->GetZoneID())
			return false;

		/*if (pSkill.sRange > 0)
		{
			if (isStaffSkill())
			{
				if (pSkillCaster->GetDistanceSqrt(pSkillTarget) > (float)pSkill.sRange * 3)
				{
					SendSkill();
					return bResult;
				}
			}
			else
			{
				if (pSkillCaster->isPlayer()
					&& pSkillCaster->GetDistanceSqrt(pSkillTarget) >= (float)pSkill.sRange)
				{
					SendSkill();
					return bResult;
				}
			}
		}*/

		bResult = true;
		uint16 sAdditionalDamage = pType->sAddDamage;

		// Give increased damage in war zones (as per official 1.298~1.325)
		// This may need to be revised to use the last nation to win the war, or more accurately,
		// the nation that last won the war 3 times in a row (whether official behaves this way now is unknown).
		if (pSkillCaster->isPlayer() && pSkillTarget->isPlayer())
		{
			if (pSkillCaster->GetMap()->isWarZone())
				sAdditionalDamage /= 2;
			else
				sAdditionalDamage /= 3;
		}

		damage = pSkillCaster->GetDamage(pSkillTarget, pSkill);

		// Only add additional damage if the target's not currently blocking it.
		// NOTE: Not sure whether to count this as physical or magic damage.
		// Using physical damage, due to the nature of these skills.
		if (!pSkillTarget->m_bBlockPhysical)
			damage += sAdditionalDamage;

		if (pSkillTarget->isPlayer())
		{
			if (pType->iADPtoUser)
				damage = (damage * (int)pType->iADPtoUser) / 100;
		}
		else if (pType->iADPtoNPC)
			damage = (damage * (int)pType->iADPtoNPC) / 100;

		if (pSkillCaster->GetZoneID() == ZONE_CHAOS_DUNGEON && nSkillID != 490226)
			damage = 100;
		else if (pSkillCaster->GetZoneID() == ZONE_CHAOS_DUNGEON && nSkillID == 490226)
			damage = 1000;

		pSkillTarget->HpChange(-damage, pSkillCaster);

		if (pSkillTarget->m_bReflectArmorType != 0 && pSkillCaster != pSkillTarget)
			ReflectDamage(damage, pSkillTarget);
	}
	else if (sTargetID == -1)
	{
		std::vector<Unit*> casted_member;
		std::vector<uint16> unitList;
		g_pMain->GetUnitListFromSurroundingRegions(pSkillCaster, &unitList);
		uint16 sAdditionalDamage = pType->sAddDamage;
		bResult = true;

		int sRange = pType->sRange;
		if (isStompSkills()) sRange += 5;
		// DeaFSezo yere vurma düzenlemesi 28.06.2024
		Unit* pTUsersx = nullptr;

		if (pSkillCaster->isPlayer())
			pTUsersx = g_pMain->GetUnitPtr(TO_USER(pSkillCaster)->GetTargetID(), sSkillCasterZoneID);

		if (pTUsersx && isStompSkills())
			casted_member.push_back(pTUsersx);
		
		foreach(itr, unitList) {
			Unit* pTarget = g_pMain->GetUnitPtr(*itr, sSkillCasterZoneID);
			if (pTarget == nullptr || pTarget->isDead()) continue;
			if (pTarget->isPlayer() && TO_USER(pTarget)->isGM()) continue;

			if (pSkillCaster != pTarget && !pTarget->isDead() && !pTarget->isBlinking() && pTarget->isAttackable()
				&& CMagicProcess::UserRegionCheck(pSkillCaster, pTarget, pSkill, sRange, sData[0], sData[2])) {
				
				if (pTUsersx)
				{
					if (pTarget != pTUsersx)
						casted_member.push_back(pTarget);
				}
				else
					casted_member.push_back(pTarget);
				// DeaFSezo yere vurma düzenlemesi 28.06.2024 end

			}
		}// DeaFSezo yere vurma düzenlemesi 28.06.2024

		// If you managed to not hit anything with your AoE, you're still gonna have a cooldown (You should l2aim)
		if (casted_member.empty()) { SendSkill(); return true; }

		sData[1] = 1;
		foreach(itr, casted_member)
		{
			Unit* pTarget = *itr; // it's checked above, not much need to check it again
			if (pTarget == nullptr)
				continue;

			//printf("%s\n", pTarget->GetName().c_str());
			if (pTarget->isPlayer() && !TO_USER(pTarget)->isInGame())
				continue;

			if (pTarget->GetEventRoom() != pSkillCaster->GetEventRoom())
				continue;

			if (pSkill.sRange > 0 && pSkillCaster->isPlayer()) {
				float SkillRange = (float)pSkill.sRange + 5;
				if (isStompSkills()) SkillRange += 3;
				// // DeaFSezo warrior skill vurma fix yere vurma düzenlemesi 28.06.2024
				if (isStompSkills())
				{
					if (!pTarget->isInRange(pTUsersx->GetX(), pTUsersx->GetZ(), RANGE_15M))
						continue;
					//
				}
				else if (pSkillCaster->GetDistanceSqrt(pTarget) >= SkillRange)
					continue;
				// DeaFSezo yere vurma düzenlemesi 28.06.2024 end
			}

			uint16 sAdditionalDamage = pType->sAddDamage;
			damage = pSkillCaster->GetDamage(pTarget, pSkill, true);

			if (!pTarget->m_bBlockPhysical)
				damage += sAdditionalDamage;

			if (pSkillCaster->GetZoneID() == ZONE_CHAOS_DUNGEON && nSkillID != 490226)
				damage = 100;
			else if (pSkillCaster->GetZoneID() == ZONE_CHAOS_DUNGEON && nSkillID == 490226)
				damage = 1000;

			pTarget->HpChange(-damage, pSkillCaster);

			if (pTarget->isPlayer())
				TO_USER(pTarget)->ItemWoreOut(DEFENCE, damage);

			if (pTarget->m_bReflectArmorType != 0 && pSkillCaster != pTarget)
				ReflectDamage(damage, pTarget);
		}
	}

	if (pSkillTarget != nullptr)
	{
		if (pSkillTarget->isNPC() && (pSkill.iNum == 108821 || pSkill.iNum == 208821))
			bOpcode = (int8)MagicOpcode::MAGIC_CASTING;
	}

	sData[3] = (damage == 0 ? (int8)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_ATTACKZERO : 0);

	if (pSkill.iNum == 106520 || pSkill.iNum == 106802 || pSkill.iNum == 206520
		|| pSkill.iNum == 105520 || pSkill.iNum == 205520 || pSkill.iNum == 206802
		|| pSkill.iNum == 106820 || pSkill.iNum == 206820 || pSkill.iNum == 490220)
		return bResult;

	SendSkill();
	return bResult;
}

bool MagicInstance::ExecuteType2()
{
	/*
	NOTE:
	Archery skills work differently to most other skills.

	When an archery skill is used, the client sends MAGIC_FLYING (instead of MAGIC_CASTING)
	to show the arrows flying in the air to their targets.

	The client chooses the target(s) to be hit by the arrows.

	When an arrow hits a target, it will send MAGIC_EFFECTING which triggers this handler.
	An arrow sent may not necessarily hit a target.

	As such, for archery skills that use multiple arrows, not all n arrows will necessarily
	hit their target, and thus they will not necessarily call this handler all n times.

	What this means is, we must remove all n arrows from the user in MAGIC_FLYING, otherwise
	it is not guaranteed all arrows will be hit and thus removed.
	(and we can't just go and take all n items each time an arrow hits, that could potentially
	mean 25 arrows are removed [5 each hit] when using "Arrow Shower"!)

	However, via the use of hacks, this MAGIC_FLYING step can be skipped -- so we must also check
	to ensure that there arrows are indeed flying, to prevent users from spamming the skill
	without using arrows.
	*/

	if (!pSkillCaster || pSkill.isnull())
		return false;

	_MAGIC_TYPE2* pType = g_pMain->m_Magictype2Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	if (!pSkillCaster)
		return false;

	int damage = 0;
	bool bResult = false;
	float total_range = 0.0f;	// These variables are used for range verification!
	int sx, sz, tx, tz;
	int range = 0;

	// If we need arrows, then we require a bow.
	// This check is needed to allow for throwing knives (the sole exception at this time.
	// In this case, 'NeedArrow' is set to 0 -- we do not need a bow to use throwing knives, obviously.

	if (pType->iNum != 107656
		&& pType->iNum != 108656
		&& pType->iNum != 207656
		&& pType->iNum != 208656)
	{
		if (pType->bNeedArrow > 0)
		{
			_ITEM_TABLE pTable = _ITEM_TABLE();
			if (pSkillCaster->isPlayer())
			{
				if (TO_USER(pSkillCaster)->isWeaponsDisabled())
					return false;

				pTable = TO_USER(pSkillCaster)->GetItemPrototype(LEFTHAND);

				if (pTable.isnull() || !pTable.isBow())
				{
					pTable = TO_USER(pSkillCaster)->GetItemPrototype(RIGHTHAND);

					if (pTable.isnull() || !pTable.isBow())
						return false;
				}
			}
			else
			{
				pTable = g_pMain->GetItemPtr(TO_NPC(pSkillCaster)->m_iWeapon_1);
				if (pTable.isnull())
					return false;
			}

			if (!pTable.isnull())
				range = pTable.m_sRange;
		}
		else
		{
			range = pSkill.sRange;
		}
	}
	else
	{
		range = pSkill.sRange * 2;
	}

	/*auto * pTargetNpc = g_pMain->GetNpcPtr(TO_USER(pSkillCaster)->GetTargetID(), TO_USER(pSkillCaster)->GetZoneID());
	if (pTargetNpc && pTargetNpc->GetProtoID() == 541 && pTargetNpc->GetZoneID() == ZONE_DELOS && g_pMain->m_byBattleSiegeWarOpen)
		pSkillTarget = pTargetNpc;
	else
		pSkillTarget = pTargetNpc;*/  // bu kod yüzünden okçular userlara vurmuyor.

	if (pSkillTarget == nullptr || pSkillTarget->isDead())
		goto packet_send;

	total_range = pow(((pType->sAddRange * range) / 100.0f), 2.0f);
	sx = (int)pSkillCaster->GetX(); tx = (int)pSkillTarget->GetX();
	sz = (int)pSkillCaster->GetZ(); tz = (int)pSkillTarget->GetZ();
	float distance = pow((float)(sx - tx), 2.0f) + pow((float)(sz - tz), 2.0f);

	if (distance > total_range)
		return false;

	uint8 successivecount = 0;

	if (pSkillCaster->isPlayer())
	{
		TO_USER(pSkillCaster)->m_arrowLock.lock();

		CUser* pUser = TO_USER(pSkillCaster);
		for (auto itr = pUser->m_flyingArrowsSuccess.begin(); itr != pUser->m_flyingArrowsSuccess.end();)
		{
			if (UNIXTIME2 >= itr->tFlyingTime + 500)
				itr = pUser->m_flyingArrowsSuccess.erase(itr);
			else
			{
				if (itr->nSkillID == nSkillID)
					successivecount++;

				itr++;
			}
		}
		//
		TO_USER(pSkillCaster)->m_arrowLock.unlock();
		// okcu range kontrolleri 31.05.2021
		if (pSkillCaster->isPlayer())
		{
			////5 'li 3 'lu Mesafe Kontrol
			//if (nSkillID == 107555 || nSkillID == 107515 || nSkillID == 108515 || nSkillID == 108555 || nSkillID == 207555 || nSkillID == 207515 || nSkillID == 208515 || nSkillID == 208555)
			//{
			//	if (pSkillTarget && pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 135.0f))
			//		goto packet_send;
			//}
			//else
			//{
				//Okcu Diger Skilleri icin
			if (pSkillTarget && pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 2400.0F))
				goto packet_send;
			//}
		}
		// okcu range kontrolleri 31.05.2021

		if (pUser->m_flyingArrows.empty())
			goto packet_send;

		ArrowList::iterator arrowItr;
		bool bFoundArrow = false;
		for (auto itr = pUser->m_flyingArrows.begin(); itr != pUser->m_flyingArrows.end();)
		{
			if (UNIXTIME2 >= itr->tFlyingTime + ARROW_EXPIRATION_TIME)
			{
				itr = pUser->m_flyingArrows.erase(itr);
			}
			else
			{
				if (itr->nSkillID == nSkillID)
				{
					arrowItr = itr;
					bFoundArrow = true;
				}
				++itr;
			}
		}

		if (!bFoundArrow)
			goto packet_send;

		pUser->m_flyingArrows.erase(arrowItr);
	}

	if (distance > 40 && successivecount > 0) {}
	else
	{
		damage = pSkillCaster->GetDamage(pSkillTarget, pSkill);
		if (pSkillCaster->isPlayer())
		{
			CUser* pUser = TO_USER(pSkillCaster);

			if (pSkillTarget->isPlayer())
			{
				if (pType->iADPtoUser)
					damage = (damage * (int)pType->iADPtoUser) / 100;
			}
			else if (pType->iADPtoNPC)
				damage = (damage * (int)pType->iADPtoNPC) / 100;

			pSkillTarget->HpChange(-damage, pSkillCaster);
			if (pSkillTarget->m_bReflectArmorType != 0 && pSkillCaster != pSkillTarget)
				ReflectDamage(damage, pSkillTarget);

			pUser->ItemWoreOut(ATTACK, damage);

			ULONGLONG useTime = UNIXTIME2;
			if (pUser->isGenieActive())
				useTime += 150;
			else
				useTime += 250; //önceki hali 400 tarih : 21.04.2024
			pUser->pUserMagicUsed.ArrowUseTime = useTime;
			pUser->m_flyingArrowsSuccess.push_back(Arrow(pType->iNum, UNIXTIME2));
		}
	}

	bResult = true;
packet_send:
	sData[3] = (damage == 0 ? (int16)e_SkillMagicFailMsg::SKILLMAGIC_FAIL_ATTACKZERO : 0);
	if (pSkill.bType[0] == 2 && (pSkill.bType[1] != 3 && pSkill.bType[1] != 4))
	{
		if (pSkillTarget == nullptr)
			SendSkill();
		else
		{
			sData[1] = 1;
			BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, pSkillTarget->GetID(), bOpcode, nSkillID, sData);
		}
	}
	return bResult;
}

// Applied when a magical attack, healing, and mana restoration is done.
bool MagicInstance::ExecuteType3()
{
	if (pSkill.isnull() || pSkillCaster == nullptr)
		return false;

	_MAGIC_TYPE3* pType = g_pMain->m_Magictype3Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	int damage = 0, duration_damage = 0;
	vector<Unit*> casted_member;

	// If the target's a group of people...
	if (sTargetID == -1)
	{
		std::vector<uint16> unitList;

		g_pMain->GetUnitListFromSurroundingRegions(pSkillCaster, &unitList);
		if (pType->sFirstDamage > 0 || pType->sTimeDamage > 0)
			casted_member.push_back(pSkillCaster);

		foreach(itr, unitList)
		{
			Unit* pTarget = g_pMain->GetUnitPtr(*itr, sSkillCasterZoneID);
			if (pTarget == nullptr)
				continue;

			if (pTarget->isPlayer() && (TO_USER(pTarget)->isGM() || TO_USER(pTarget)->isBlinking()))
				continue;

			if (pTarget->GetEventRoom() != pSkillCaster->GetEventRoom())
				continue;

			if (pSkillCaster != pTarget
				&& !pTarget->isDead()
				&& !pTarget->isBlinking()
				&& pTarget->isAttackable()
				&& CMagicProcess::UserRegionCheck(pSkillCaster, pTarget, pSkill, pType->bRadius, sData[0], sData[2]))
				casted_member.push_back(pTarget);
		}

		// If you managed to not hit anything with your AoE, you're still gonna have a cooldown (You should l2aim)
		if (casted_member.empty())
		{
			if (pSkill.bMoral == MORAL_SIEGE_WEAPON)
				casted_member.push_back(pSkillCaster);
			else
			{
				SendSkill();
				return true;
			}
		}
	}
	else
	{	// If the target was a single unit.
		if (pSkillTarget == nullptr
			|| pSkillTarget->isDead()
			|| (pSkillTarget->isPlayer() && TO_USER(pSkillTarget)->isBlinking()))
			return false;

		casted_member.push_back(pSkillTarget);
	}

	// Anger explosion requires the caster be a player
	// and a full anger gauge (which will be reset after use).
	if (pType && pType->bDirectType == 18)
	{
		// Only players can cast this skill.
		if (!pSkillCaster->isPlayer() || pSkillCaster && !TO_USER(pSkillCaster)->hasFullAngerGauge())
			return false;

		// Reset the anger gauge
		TO_USER(pSkillCaster)->UpdateAngerGauge(0);
	}

	sData[1] = 1;
	foreach(itr, casted_member)
	{
		Unit* pTarget = *itr; // it's checked above, not much need to check it again

		if (pTarget == nullptr)
			continue;

		if (pTarget->isPlayer()
			&& !TO_USER(pTarget)->isInGame())
			continue;

		if (pTarget->isPlayer() && pTarget->m_bType3Flag)
		{
			if (g_pMain->pTempleEvent.ActiveEvent && !g_pMain->pTempleEvent.isAttackable && pTarget->isInTempleEventZone()) continue;
		}

		//if (pSkill && pSkill.bCastTime == 0 && pSkill.sUseStanding == 1 && pSkill.sRange > 0)
		//{
		//	if (pSkill.sRange == 10000)
		//	{
		//		if (!pSkillCaster->isInRangeSlow(pTarget, float(pSkill.sRange)))
		//			continue;
		//	}
		//}
		// mage range kontrolleri 31.05.2021
		if (pSkillCaster->isPlayer())
		{
			//mage 43-56 skills range control
			if (nSkillID == 110556 || nSkillID == 210556 || nSkillID == 109556 || nSkillID == 209556
				|| nSkillID == 109656 || nSkillID == 209656 || nSkillID == 110656 || nSkillID == 210656
				|| nSkillID == 109756 || nSkillID == 209756 || nSkillID == 110756 || nSkillID == 210756
				|| nSkillID == 109543 || nSkillID == 209543 || nSkillID == 110543 || nSkillID == 210543
				|| nSkillID == 109643 || nSkillID == 209643 || nSkillID == 110643 || nSkillID == 210643
				|| nSkillID == 109743 || nSkillID == 209743 || nSkillID == 110743 || nSkillID == 210743)
			{
				if (pSkillTarget && pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 135.0f))
					continue;
			}

			//mage 51-54-57 skills range control
			if (nSkillID == 109551 || nSkillID == 209551 || nSkillID == 110551 || nSkillID == 210551
				|| nSkillID == 109554 || nSkillID == 209554 || nSkillID == 110554 || nSkillID == 210554
				|| nSkillID == 109557 || nSkillID == 209557 || nSkillID == 110557 || nSkillID == 210557
				|| nSkillID == 109651 || nSkillID == 209651 || nSkillID == 110651 || nSkillID == 210651
				|| nSkillID == 109657 || nSkillID == 209657 || nSkillID == 110657 || nSkillID == 210657
				|| nSkillID == 109751 || nSkillID == 209751 || nSkillID == 110751 || nSkillID == 210751
				|| nSkillID == 109754 || nSkillID == 209754 || nSkillID == 110754 || nSkillID == 210754
				|| nSkillID == 109757 || nSkillID == 209757 || nSkillID == 110757 || nSkillID == 210757)
			{
				//if (pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 2250.0F))
				if (pSkillTarget && pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 5250.0F))
					continue;
			}

			//mage 80 ice fire range control
			if (nSkillID == 110674 || nSkillID == 210674 || nSkillID == 110575 || nSkillID == 210575)
			{
				//if (pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 2250.0F))
				if (pSkillTarget && pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 5250.0F))
					continue;
			}

			//mage 18 skills range control
			if (nSkillID == 109518 || nSkillID == 209518 || nSkillID == 110518 || nSkillID == 210518
				|| nSkillID == 109618 || nSkillID == 209618 || nSkillID == 110618 || nSkillID == 210618
				|| nSkillID == 109718 || nSkillID == 209718 || nSkillID == 110718 || nSkillID == 210718)
			{
				//if (pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 1150.0f))
				if (pSkillTarget && pSkillTarget->GetX() > 0 && !pSkillCaster->isInRange(pSkillTarget->GetX(), pSkillTarget->GetZ(), 4150.0f))
					continue;
			}

			//Mage alan skills range control
			if (nSkillID == 109533 || nSkillID == 209533 || nSkillID == 110533 || nSkillID == 210533
				|| nSkillID == 109633 || nSkillID == 209633 || nSkillID == 110633 || nSkillID == 210633
				|| nSkillID == 109733 || nSkillID == 209733 || nSkillID == 110733 || nSkillID == 210733
				|| nSkillID == 109545 || nSkillID == 209545 || nSkillID == 110545 || nSkillID == 210545
				|| nSkillID == 109645 || nSkillID == 209645 || nSkillID == 110645 || nSkillID == 210645
				|| nSkillID == 109745 || nSkillID == 209745 || nSkillID == 110745 || nSkillID == 210745
				|| nSkillID == 109560 || nSkillID == 209560 || nSkillID == 110560 || nSkillID == 210560
				|| nSkillID == 109660 || nSkillID == 209660 || nSkillID == 110660 || nSkillID == 210660
				|| nSkillID == 109760 || nSkillID == 209760 || nSkillID == 110760 || nSkillID == 210760
				|| nSkillID == 110571 || nSkillID == 210571 || nSkillID == 110671 || nSkillID == 210671
				|| nSkillID == 110771 || nSkillID == 210771 || nSkillID == 110762 || nSkillID == 210762)
			{
				if (pTarget->GetX() > 0 && !pSkillCaster->isInRange(pTarget->GetX(), pTarget->GetZ(), 2250.0F))
					continue;
			}
		}
		// mage range kontrolleri 31.05.2021

		/*if ((isStaffSkill() || nSkillID == 210572 || nSkillID == 110572))
		{
			if (pSkill && pSkill.sUseStanding == 0 && pSkill.sRange > 0 && !pSkillCaster->isInRangeSlow(pTarget, float(pSkill.sRange * 2.0f)))
				continue;
		}

		else if (pSkill && pSkill.sUseStanding == 0 && pSkill.sRange > 0)
		{
			if (pSkill.iNum == 120011 || pSkill.iNum == 220011)
			{
				if (pSkillCaster && !pSkillCaster->isInRangeSlow(pTarget, float(7 * 1.5f)))
					continue;
			}
			else
			{
				if (pSkillCaster && !pSkillCaster->isInRangeSlow(pTarget, float(pSkill.sRange * 1.5f)))
					continue;
			}
		}*/

		/*02.10.2020 Zamanlı Mage Zehir Skili ile priestin Restore atması fixlendi.*/
		// If you are casting an attack spell.
		if (pType && (pType->sFirstDamage < 0) && (pType->bDirectType == 1 || pType->bDirectType == 8) && (nSkillID < 400000) && (pType->bDirectType != 11 && pType->bDirectType != 13))
			damage = GetMagicDamage(pTarget, pType->sFirstDamage, pType->bAttribute);
		else
			damage = pType->sFirstDamage;

		// Allow for complete magic damage blocks.
		if (damage < 0 && pTarget->m_bBlockMagic)
			continue;

		if (pSkillCaster && pSkillCaster->isPlayer())
		{
			if (pSkillCaster && pSkillCaster->GetZoneID() == ZONE_SNOW_BATTLE && g_pMain->m_byBattleOpen == SNOW_BATTLE)
				damage = -10;

			// hacking prevention for potions
			if (pSkill.iNum > 400000 // using hp and mana potion at the same time!!! not at all!
				&& pSkill.bType[1] == 0
				&& pSkill.bMoral == MORAL_SELF
				&& pSkill.bItemGroup == 9
				&& (pType->bDirectType == 1 || pType->bDirectType == 2)
				&& pType->sFirstDamage > 0)
				TO_USER(pSkillCaster)->t_timeLastPotionUse = UNIXTIME2;
		}

		bool bSendLightningStunSkill = true;

		if (pType && pType->bDirectType < 2 && nSkillID < 400000 && pTarget && pTarget->isPlayer() && (pType->bAttribute == (uint8)AttributeType::AttributeLightning || pType->bAttribute == (uint8)AttributeType::AttributeIce))
			bSendLightningStunSkill = false;

		if (pTarget->isPlayer())
		{
			if (pType->iADPtoUser)
				damage = (damage * (int)pType->iADPtoUser) / 100;
		}
		else if (pType->iADPtoNPC)
			damage = (damage * (int)pType->iADPtoNPC) / 100;

		// Non-durational spells.
		if (pType->bDuration == 0)
		{
			switch (pType->bDirectType)
			{
				// Affects target's HP
			case 1:
			{
				if (pSkillCaster && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isPriest())
				{
					if (TO_USER(pSkillCaster)->isHealDamageChecking(pSkillTarget, pSkill))
					{
						damage = pType->sFirstDamage / 2;
						damage = -damage;

						if (damage > 0)
							break;

						pTarget->HpChangeMagic(damage, pSkillCaster, (AttributeType)pType->bAttribute);

						if (pTarget->m_bReflectArmorType != 0 && pTarget != pSkillCaster && damage < 0)
							ReflectDamage(damage, pTarget);
						break;
					}
				}

				// Disable to heal or minor NPC.
				if ((pTarget->isNPC() && pType->sTimeDamage > 0) || (pTarget->isNPC() && pType->sFirstDamage > 0))
				{
					if (TO_NPC(pTarget)->isPet())
						pTarget->HpChangeMagic(damage, pSkillCaster, (AttributeType)pType->bAttribute);

					return false;
				}

				if (pSkillCaster->isPlayer() && pTarget->isPlayer() && TO_USER(pSkillCaster)->isPriest() && pSkillCaster != pSkillTarget && damage > 0)
				{
					int hpfark = pTarget->GetMaxHealth() - pTarget->GetHealth();
					if (hpfark > 0)
					{
						if (float asec = static_cast<float>(hpfark) / pTarget->GetMaxHealth())
						{
							int percent = static_cast<int>(asec * 100); uint8 assistype = 0;
							if (percent > 74) assistype = 1;
							else if (percent > 49) assistype = 2;
							else if (percent > 24) assistype = 3;
							if (assistype) TO_USER(pSkillCaster)->KA_AssistScreenSend(assistype);
						}
					}
				}

				if (pTarget->isPlayer() && damage > 0 && pTarget->isDevil())
				{
					CUser* pSkilled = TO_USER(pTarget);
					if (pSkilled != nullptr)
					{
						if (pSkilled->CheckSkillPoint(PRO_SKILL3, 45, MAX_LEVEL))
							damage += 50 * damage / 100;
					}
				}

				if (pSkill.iNum == 490231)
					damage = 1000;

				pTarget->HpChangeMagic(damage, pSkillCaster, (AttributeType)pType->bAttribute);

				if (pTarget->m_bReflectArmorType != 0 && pTarget != pSkillCaster && damage < 0)
					ReflectDamage(damage, pTarget);
			}
			break;
			case 2:
				if (pTarget->isPlayer())
				{
					if (!pTarget->isDead())
						pTarget->MSpChange(pType->sFirstDamage);
				}
				else
				{
					if (TO_NPC(pTarget)->isPet())
						pTarget->MSpChange(pType->sFirstDamage);
					else
						pTarget->HpChange(pType->sFirstDamage, pSkillCaster);
				}
				break;
			case 3:
				damage = pTarget->GetMana() / pType->sFirstDamage;
				pTarget->MSpChange(damage);
				break;

				// "Magic Hammer" repairs equipped items.
			case 4:
				if (pTarget->isPlayer())
				{
					if (damage > 0)
						TO_USER(pTarget)->ItemWoreOut(REPAIR_ALL, damage);
					else
						TO_USER(pTarget)->ItemWoreOut(ACID_ALL, -damage);
				}
				break;

				// Increases/decreases target's HP by a percentage
			case 5:
				if (pType->sFirstDamage < 100)
					damage = (pType->sFirstDamage * pTarget->GetHealth()) / -100;
				else
					damage = (pTarget->GetMaxHealth() * (pType->sFirstDamage - 100)) / 100;

				pTarget->HpChangeMagic(damage, pSkillCaster);
				break;

				// Caster absorbs damage based on percentage of target's HP. Players only.
			case 8:
				if (pType->sFirstDamage > 0)
				{
					if (pType->sFirstDamage < 100)
						damage = (pTarget->GetHealth() * 100) / pType->sFirstDamage;
					else
						damage = (pTarget->GetMaxHealth() - 100 * 100) / pType->sFirstDamage;
				}

				if (!pTarget->isDead() && pTarget->isPlayer())
				{
					pTarget->HpChangeMagic(damage, pSkillCaster);
					pSkillCaster->HpChangeMagic(-(damage));
				}
				else
					pTarget->HpChange(damage, pSkillCaster);

				break;

				// Caster absorbs damage based on percentage of target's max HP
			case 9:
				if (pType->sFirstDamage < 100)
					damage = (pType->sFirstDamage * pTarget->GetHealth()) / -100;
				else
					damage = (pTarget->GetMaxHealth() * (pType->sFirstDamage - 100)) / 100;

				pTarget->HpChangeMagic(damage, pSkillCaster);
				if (pTarget->isPlayer())
					pSkillCaster->HpChangeMagic(-(damage));
				break;

				// Inflicts true damage (i.e. disregards Ac/resistances/buffs, etc).
			case 11:
				pTarget->HpChange(damage, pSkillCaster);
				break;

				// Used by "Destination scroll" (whatever that is)
			case 12:
				continue;

				// Chance (how often?) to reduce the opponent's armor and weapon durability by sFirstDamage
			case 13:
				if (pTarget->isPlayer() && CheckPercent(500)) // using 50% for now.
				{
					TO_USER(pTarget)->ItemWoreOut(ATTACK, damage);
					TO_USER(pTarget)->ItemWoreOut(DEFENCE, damage);
				}
				break;

				// Drains target's MP, gives half of it to the caster as HP. Players only.
				// NOTE: Non-durational form (as in 1.8xx). This was made durational later (database configured).
			case 16:
				if (pTarget->isPlayer())
				{
					pTarget->MSpChange(pType->sFirstDamage);
					pSkillCaster->HpChangeMagic(-(pType->sFirstDamage));
				}
				break;
			case 17:
				if (!pSkillCaster->isDead() && !pTarget->isDead())
				{
					if (!pTarget->isNPC())
					{
						pTarget->HpChangeMagic(pType->sFirstDamage, pSkillCaster, (AttributeType)pType->bAttribute);
					}
					else
					{
						if (pSkillCaster->isNPC() && TO_NPC(pSkillCaster)->isGuardSummon())
							pTarget->HpChangeMagic(pType->sFirstDamage, pSkillCaster, (AttributeType)pType->bAttribute);
					}
					//pTarget->HpChangeMagic(pType->sFirstDamage, pSkillCaster, (AttributeType)pType->bAttribute);
				}
				break;
			case 19: // Chaos Dungeon Skills
				if (pTarget->isPlayer() || pTarget->isNPC())
				{
					if (pSkill.iNum == 490221)
						damage = (-3000);
					else if (pSkill.iNum == 490227)
						damage = (-2000);
					else if (pSkill.iNum == 490232)
						damage = (-100);

					if (pTarget != pSkillCaster)
					{
						pTarget->HpChangeMagic(damage / 10, pSkillCaster, (AttributeType)pType->bAttribute);
						ReflectDamage(damage, pTarget);
					}
				}
				break;

			case 20:
				if (!pTarget->isPlayer() || pTarget->isDead())
					return false;

				// Disable to heal or minor NPC.
				if ((pTarget->isNPC() && pType->sTimeDamage > 0) || (pTarget->isNPC() && pType->sFirstDamage > 0))
					return false;

				if (!pTarget->isDead() && pTarget->isPlayer())
				{
					_ITEM_TABLE pItem = g_pMain->GetItemPtr(810117000);
					if (pItem.isnull()) return false;

					uint32 Noah = pItem.m_iBuyPrice;
					if (Noah > 0 && !TO_USER(pTarget)->hasCoins(Noah))
						return false;

					if (Noah > 0 && !TO_USER(pSkillCaster)->GoldLose(Noah))
						return false;

					pTarget->HpChange(pType->sFirstDamage, pSkillCaster);
				}
				else return false;
				break;
			case 21:
				if (!pTarget->isPlayer() || pTarget->isDead())
					return false;

				// Disable to heal or minor NPC.
				if ((pTarget->isNPC() && pType->sTimeDamage > 0) || (pTarget->isNPC() && pType->sFirstDamage > 0))
					return false;

				if (!pTarget->isDead() && pTarget->isPlayer())
				{
					_ITEM_TABLE pItem = g_pMain->GetItemPtr(810118000);
					if (pItem.isnull())
						return false;

					uint32 Noah = pItem.m_iBuyPrice;
					if (Noah > 0 && !TO_USER(pTarget)->hasCoins(Noah))
						return false;
					if (Noah > 0 && !TO_USER(pSkillCaster)->GoldLose(Noah))
						return false;
					pTarget->MSpChange(pType->sFirstDamage);
				}
				else return false;
				break;
			case 255:// Stat Scroll - MagicNum = 501011
				if (TO_USER(pSkillCaster)->isPlayer())
				{

				}
				break;
			}
		}
		// Durational spells! Durational spells only involve HP.
		else if (pType->bDuration != 0)
		{
			if (pType->bDirectType == 18)
				damage = (int)(pType->sTimeDamage);

			if (damage != 0)		// In case there was first damage......
				pTarget->HpChangeMagic(damage, pSkillCaster);			// Initial damage!!!

			if (pTarget->isAlive())
			{
				CUser* pUser = TO_USER(pSkillCaster);

				if (pUser != nullptr)
				{
					// HP booster (should this actually just be using sFirstDamage as the percent of max HP, i.e. 105 being 5% of max HP each increment?)
					if (pType->bDirectType == 14)
						duration_damage = pType->bDuration / 2 * ((int)(pSkillCaster->GetLevel() * (1 + pSkillCaster->GetLevel() / 30.0)) + 3);
					else if (pType->bDirectType == 19)
					{
						duration_damage = pType->sTimeDamage;

						if (pType->iNum == 490224)
							duration_damage = (-5000 / 10);
					}
					else if (pSkillCaster->isPlayer() && pType->iNum == 492029 && (pUser->isPortuKurian()))
						duration_damage = (pType->sTimeDamage * 2);
					else if (pType->sTimeDamage < 0 && pType->bAttribute != 4)
						duration_damage = GetMagicDamage(pTarget, pType->sTimeDamage, pType->bAttribute);
					else
						duration_damage = pType->sTimeDamage;

					// Allow for complete magic damage blocks.
					if (duration_damage < 0 && pTarget->m_bBlockMagic)
						continue;

					if (pType->bDirectType == 18)
						duration_damage = -(int)(-(pType->sTimeDamage) * (pType->bDuration / 2));

					// Setup DOT (damage over time)
					for (int k = 0; k < MAX_TYPE3_REPEAT; k++)
					{
						Unit::MagicType3* pEffect = &pTarget->m_durationalSkills[k];

						if (pEffect == nullptr)
							continue;

						if (pEffect->m_byUsed)
							continue;

						pEffect->m_byUsed = true;
						pEffect->m_tHPLastTime = 0;
						pEffect->m_bHPInterval = 2;					// interval of 2s between each damage loss/HP gain 

						// number of ticks required at a rate of 2s per tick over the total duration of the skill
						float tickCount = (float)pType->bDuration / (float)pEffect->m_bHPInterval;

						if (pSkillCaster->GetZoneID() == ZONE_CHAOS_DUNGEON)
							tickCount *= 2;

						// amount of HP to be given/taken every tick at a rate of 2s per tick
						pEffect->m_sHPAmount = (int16)(duration_damage / tickCount);

						pEffect->m_bTickCount = 0;
						pEffect->m_bTickLimit = (uint8)tickCount;
						pEffect->m_sSourceID = sCasterID;
						pEffect->m_byAttribute = pType->bAttribute;
						pEffect->m_skill = nSkillID; // target Scroll Bar Eklemesi 03.02.2024 end
						break;
					}
					pTarget->m_bType3Flag = true;
				}
			}

			// Send the status updates (i.e. DOT or position indicators) to the party/user
			if (pTarget->isPlayer() && pType->sTimeDamage < 0)
			{
				if (bSendLightningStunSkill)
					TO_USER(pTarget)->SendUserStatusUpdate(pType->bAttribute == (uint8)ResistanceTypes::POISON_R ? UserStatus::USER_STATUS_POISON : UserStatus::USER_STATUS_DOT, UserStatusBehaviour::USER_STATUS_INFLICT);
			}
		}

		if (!bSendLightningStunSkill)
		{
			if (isStaffSkill(false) || isExtendedArcherSkill(false))
			{
				nSkillID += SKILLPACKET;
				pSkill = g_pMain->GetMagicPtr(nSkillID);
				if (pSkill.isnull())
					return false;

				ExecuteType4();
			}
		}
		else
		{
			// Send the skill data in the current context to the caster's region, with the target explicitly set.
			// In the case of AOEs, this will mean the AOE will show the AOE's effect on the user (not show the AOE itself again).
			if (pSkill.bType[1] == 0 || pSkill.bType[1] == 3)
				BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, pTarget->GetID(), bOpcode, nSkillID, sData);
		}

		if (pSkillCaster && pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isMage())
			TO_USER(pSkillCaster)->ItemWoreOut(ATTACK, -damage);

		// we're healing someone (so that they can choose to pick on the healer!)
		if (pSkillCaster && pSkillCaster->isPlayer()
			&& pType->bDirectType == 1
			&& damage > 0
			&& sCasterID != sTargetID)
		{
			TO_USER(pSkillCaster)->HealMagic();
		}
	}

	// Allow for AOE effects.
	if (sTargetID == -1 && pSkill.bType[0] == 3)
		SendSkill();

	return true;
}

bool MagicInstance::CheckIceLightSpeed(int rate, int rand)
{
	if (pSkill.isnull() || !pSkill.icelightrate)
		return false;

	int newrand = rand + (rate * 3);
	if (pSkill.icelightrate > newrand)
		return true;

	return false;
}


bool MagicInstance::ExecuteType4()
{
	if (pSkill.isnull() || pSkillCaster == nullptr)
		return false;
	
	CUser* pUser = g_pMain->GetUserPtr(TO_USER(pSkillCaster)->GetID());


	if (pUser != nullptr)
	{
		if (nSkillID == 495146)
		{
			Packet result2(XSafe, uint8(XSafeOpCodes::PL_SLAVEPRIEST));
			result2 << uint8(1);
			pUser->Send(&result2);

		}
	}

	_MAGIC_TYPE4* pType = g_pMain->m_Magictype4Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	int damage = 0;
	vector<Unit*> casted_member;

	if (sTargetID >= 0 && pType->bBuffType == (uint8)BuffType::BUFF_TYPE_FREEZE && (pSkillTarget && pSkillTarget->isNPC()))
		return false;

	if (!bIsRecastingSavedMagic && sTargetID >= 0 && (pSkillTarget && pSkillTarget->HasSavedMagic(nSkillID)))
	{
		if (pType->bBuffType == 55 && pSkill.bType[0] == 6 && pSkill.bType[1] == 4)
		{
			pSkillTarget->m_buffLock.lock();
			if (pSkillTarget->m_buffMap.find(pType->bBuffType) != pSkillTarget->m_buffMap.end())
			{
				pSkillTarget->m_buffLock.unlock();
				return false;
			}
			pSkillTarget->m_buffLock.unlock();
		}
		else return false;
	}

	if (sTargetID == -1)
	{
		std::vector<uint16> unitList;

		g_pMain->GetUnitListFromSurroundingRegions(pSkillCaster, &unitList);
		foreach(itr, unitList)
		{
			Unit* pTarget = g_pMain->GetUnitPtr(*itr, sSkillCasterZoneID);
			if (pTarget == nullptr) continue;
			if (pTarget->isPlayer() && (TO_USER(pTarget)->isGM() || !TO_USER(pTarget)->isInGame() || TO_USER(pTarget)->isBlinking()) || pTarget->isDead()) continue;
			if (!pTarget->isAttackable()) continue;

			int added = 0;
			if (pTarget->isNPC() && (pSkill.bMoral == 7 || pSkill.bMoral == 10) && (pType->bBuffType == 40 || pType->bBuffType == 6 || pType->bBuffType == 47)) added = 3;
			if (CMagicProcess::UserRegionCheck(pSkillCaster, pTarget, pSkill, pType->bRadius + added, sData[0], sData[2]))
			{
				casted_member.push_back(pTarget);
				if (pTarget->isPlayer() && pSkillCaster->GetNation() != pTarget->GetNation())TO_USER(pTarget)->RemoveStealth();
			}
		}

		if (casted_member.empty() && pSkillCaster->isPlayer())
		{
			if (pSkill.bMoral == MORAL_PARTY_ALL) casted_member.push_back(pSkillCaster);
			else if (pSkill.bMoral == MORAL_ENEMY_PARTY) casted_member.push_back(pSkillCaster);
			else if (pSkill.bMoral == MORAL_SIEGE_WEAPON) casted_member.push_back(pSkillCaster);
			else if (pSkill.bMoral == MORAL_SELF) casted_member.push_back(pSkillCaster);
			else { SendSkill(); return true; }
		}
	}
	else
	{
		if (pSkillTarget == nullptr || (pSkillTarget && pSkillTarget->isPlayer() && pSkillTarget->isDead()) || (pSkillTarget && pSkillTarget->isPlayer() && TO_USER(pSkillTarget)->isBlinking() && !bIsRecastingSavedMagic))
			return false;

		casted_member.push_back(pSkillTarget);
	}

	foreach(itr, casted_member)
	{
		Unit* pTarget = *itr;

		if (pTarget == nullptr)
			continue;

		// DEBUFF BILGI SISTEMI 27.09.2020 start
		CUser* pUser = TO_USER(pSkillCaster);

		if (pUser == nullptr)
			continue;

		if (pTarget->isPlayer())
		{
			if (nSkillID == 112703 || nSkillID == 111703)
			{
				std::string DebuffNotice = string_format("%s Adli Karektere Malice Atildi", pTarget->GetName().c_str());
				std::string Nick = pUser->GetName();
				Packet DBInfoPacket;
				ChatPacket::Construct(&DBInfoPacket, (uint8)ChatType::PARTY_CHAT, &DebuffNotice, &Nick);
				g_pMain->Send_PartyMember(pUser->GetPartyID(), &DBInfoPacket);
			}
			if (nSkillID == 212703 || nSkillID == 211703)
			{
				std::string DebuffNotice = string_format("%s Adli Karektere Malice Atildi", pTarget->GetName().c_str());
				std::string Nick = pUser->GetName();
				Packet DBInfoPacket;
				ChatPacket::Construct(&DBInfoPacket, (uint8)ChatType::PARTY_CHAT, &DebuffNotice, &Nick);
				g_pMain->Send_PartyMember(pUser->GetPartyID(), &DBInfoPacket);
			}
			if (nSkillID == 112745 || nSkillID == 111745)
			{
				std::string DebuffNotice = string_format("%s Adli Karektere Parasite Atildi", pTarget->GetName().c_str());
				std::string Nick = pUser->GetName();
				Packet DBInfoPacket;
				ChatPacket::Construct(&DBInfoPacket, (uint8)ChatType::PARTY_CHAT, &DebuffNotice, &Nick);
				g_pMain->Send_PartyMember(pUser->GetPartyID(), &DBInfoPacket);
			}
			if (nSkillID == 212745 || nSkillID == 211745)
			{
				std::string DebuffNotice = string_format("%s Adli Karektere Parasite Atildi", pTarget->GetName().c_str());
				std::string Nick = pUser->GetName();
				Packet DBInfoPacket;
				ChatPacket::Construct(&DBInfoPacket, (uint8)ChatType::PARTY_CHAT, &DebuffNotice, &Nick);
				g_pMain->Send_PartyMember(pUser->GetPartyID(), &DBInfoPacket);
			}
			if (nSkillID == 112771)
			{
				std::string DebuffNotice = string_format("%s Adli Karektere Superior Parasite Atildi", pTarget->GetName().c_str());
				std::string Nick = pUser->GetName();
				Packet DBInfoPacket;
				ChatPacket::Construct(&DBInfoPacket, (uint8)ChatType::PARTY_CHAT, &DebuffNotice, &Nick);
				g_pMain->Send_PartyMember(pUser->GetPartyID(), &DBInfoPacket);
			}
			if (nSkillID == 212771)
			{
				std::string DebuffNotice = string_format("%s Adli Karektere Superior Parasite Atildi", pTarget->GetName().c_str());
				std::string Nick = pUser->GetName();
				Packet DBInfoPacket;
				ChatPacket::Construct(&DBInfoPacket, (uint8)ChatType::PARTY_CHAT, &DebuffNotice, &Nick);
				g_pMain->Send_PartyMember(pUser->GetPartyID(), &DBInfoPacket);
			}
		}
		// DEBUFF BILGI SISTEMI 27.09.2020 end

		//if (pSkill && pSkill.bCastTime == 0
		//	&& pSkill.sRange > 0
		//	&& (pSkillCaster->GetDistanceSqrt(pTarget) >= (float)pSkill.sRange)
		//	&& pType->bBuffType != BUFF_TYPE_HP_MP
		//	&& pType->bBuffType != BUFF_TYPE_AC)
		//	continue;

		uint8 bResult = 1;
		_BUFF_TYPE4_INFO pBuffInfo;
		bool bAllowCastOnSelf = false;
		uint16 sDuration = pType->sDuration;

		// Speed Skills
		bool bSetSpeed = true;
		uint8 nTargetSpeedAmount = pType->bSpeed;

		// A handful of skills (Krowaz, currently) should use the caster as the target.
		// As such, we should correct this before any other buff/debuff logic is handled.
		switch (pType->bBuffType)
		{
			//case BUFF_TYPE_UNDEAD:
			//case BUFF_TYPE_UNSIGHT:
		case BUFF_TYPE_BLOCK_PHYSICAL_DAMAGE:
		case BUFF_TYPE_BLOCK_MAGICAL_DAMAGE:
		case BUFF_TYPE_ARMORED:
			if (pSkillCaster && !pSkillCaster->isPlayer())
				continue;

			pTarget = pSkillCaster;
			bAllowCastOnSelf = true;
			break;

		case BUFF_TYPE_DISABLE_TARGETING:
		{
			if (pSkillCaster && !pSkillCaster->isPlayer())
				continue;

			if (pSkill.iNum == 108780 || pSkill.iNum == 208780)
				bAllowCastOnSelf = true;
		}
		break;
		}

		bool bBlockingDebuffs = pTarget->m_bBlockCurses;

		// Skill description: Blocks all curses and has a chance to reflect the curse back onto the caster.
		// NOTE: the exact rate is undefined, so we'll have to guess and improve later.
		if (pType->isDebuff() && pTarget->m_bReflectCurses)
		{
			const short reflectChance = 25; // % chance to reflect.
			if (CheckPercent(reflectChance * 10))
			{
				pTarget = pSkillCaster; // skill has been reflected, the target is now the caster.
				bBlockingDebuffs = (pTarget->m_bBlockCurses || pTarget->m_bReflectCurses);
				bAllowCastOnSelf = true;
			}
			// Didn't reflect, so we'll just block instead.
			else
			{
				bBlockingDebuffs = true;
			}
		}

		if (pTarget->isPlayer())
		{
			if ((pSkill.bMoral == MORAL_ENEMY || pSkill.bMoral == MORAL_AREA_ENEMY || isKurianStuns2()) // 15.02.2024 eklemesi
				&& (pType->bBuffType == BUFF_TYPE_SPEED2 || pType->bBuffType == BUFF_TYPE_SPEED || pType->bBuffType == BUFF_TYPE_STUN))
			{
				bool te = (isRushSkill() || isKurianStuns2() || pSkill.iNum == 492027 || pSkill.iNum == 190773 || pSkill.iNum == 290773
					|| pSkill.iNum == 190673 || pSkill.iNum == 290673);
				//printf("kurian skill test1 : %d \n", pSkill.iNum);
				if (!te)
				{
					int targetres = pType->bBuffType == (uint8)BuffType::BUFF_TYPE_SPEED2 ? pTarget->m_sColdR : pTarget->m_sLightningR, maxres = 250;
					targetres += pTarget->m_bResistanceBonus;
					if (targetres > maxres)
						targetres = maxres;

					//printf("Other freeze skill used\n");
					int percentagerate = (targetres * 100) / maxres, rand = myrand(0, 10000);
					//printf("kurian skill test2 : %d | percentagerate: %d | rand: %d | targetres: %d \n", pSkill.iNum, percentagerate, rand, targetres);
					if (!CheckIceLightSpeed(percentagerate, rand))
					{
						//printf("kurian skill test3 : %d | percentagerate: %d | rand: %d | targetres: %d \n", pSkill.iNum, percentagerate, rand, targetres);
						//printf("Other freeze skill is success\n");
						bSetSpeed = false; // kitlemez olur
					}
				}
				else
				{
					int rand2 = myrand(0, 10000);
					//printf("kurian skill test4 : %d | rand2: %d | pSkill.icelightrate: %d \n", pSkill.iNum, rand2, pSkill.icelightrate);
					if (pSkill.icelightrate <= rand2)
					{
						//printf("kurian skill test5 : %d | rand2: %d | pSkill.icelightrate: %d \n", pSkill.iNum, rand2, pSkill.icelightrate);
						//printf("Kurian stun skill is success\n");
						bSetSpeed = false; // kitlemez olur
					}
				}
				if (pSkill.iNum == 106802
					|| pSkill.iNum == 206802)

				{
					Packet result(WIZ_EFFECT);
					result << pTarget->GetID() << nSkillID;
					pTarget->SendToRegion(&result);
				}
			}
		}

		pTarget->m_buffLock.lock();
		Type4BuffMap::iterator buffItr = pTarget->m_buffMap.find(pType->bBuffType);
		bool bSkillTypeAlreadyOnTarget = (!bIsRecastingSavedMagic && buffItr != pTarget->m_buffMap.end());
		pTarget->m_buffLock.unlock();

		if (bSkillTypeAlreadyOnTarget && pType->isDebuff())
		{
			if (pType->bBuffType == BUFF_TYPE_STATS)
			{
				if (CMagicProcess::RemoveType4Buff(BUFF_TYPE_BATTLE_CRY, pTarget, false) && pTarget->isPlayer())
				{
					Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_DURATION_EXPIRED));
					result << uint8(BUFF_TYPE_BATTLE_CRY);
					TO_USER(pTarget)->Send(&result);
				}

				CMagicProcess::RemoveType4Buff(BUFF_TYPE_STATS, pTarget, false);
				bSkillTypeAlreadyOnTarget = false;


			}
			else if (pType->bBuffType != BUFF_TYPE_SPEED2)
			{
				CMagicProcess::RemoveType4Buff(pType->bBuffType, pTarget, false);
				bSkillTypeAlreadyOnTarget = false;
			}
			else if (pType->bBuffType == BUFF_TYPE_SPEED2 && bSetSpeed)
			{
				CMagicProcess::RemoveType4Buff(pType->bBuffType, pTarget, true);
				bSkillTypeAlreadyOnTarget = false;
			}
		}

		// If this skill is a debuff, and we are in the crossfire, 
		// we should not bother debuffing ourselves (that would be bad!)
		// Note that we should allow us if there's an explicit override (i.e. with Krowaz self-debuffs)
		if (!bAllowCastOnSelf && pType->isDebuff() && pTarget == pSkillCaster)
			continue;

		// If the user already has this buff type cast on them (debuffs should just reset the duration)
		if ((bSkillTypeAlreadyOnTarget && pType->isBuff()) || (pType->isDebuff() && bBlockingDebuffs) || !CMagicProcess::GrantType4Buff(pSkill, pType, pSkillCaster, pTarget, bIsRecastingSavedMagic))
		{
			if (sTargetID != -1 && (pType->isBuff() || (pType->isDebuff() && bBlockingDebuffs)))
			{
				bResult = 0;
				goto fail_return;
			}

			continue;
		}

		if (nSkillID == 495146 && !pUser->isPriestUseZone())
			pUser->InsertSavedMagic(nSkillID, pType->sDuration);

		// Only players can store persistent skills.
		if (nSkillID > 500000 && pTarget->isPlayer())
		{
			// Persisting effects will already exist in the map if we're recasting it. 
			if (!bIsRecastingSavedMagic)
				pTarget->InsertSavedMagic(nSkillID, pType->sDuration);
			else
				sDuration = pTarget->GetSavedMagicDuration(nSkillID);
		}

		if (pSkillCaster->isPlayer()
			&& (sTargetID != -1 && pSkill.bType[0] == 4))
		{
			CUser* pCaster = TO_USER(pSkillCaster);

			if (pCaster != nullptr)
			{
				if (pCaster->isPortuKurian())
					pCaster->SpChange(-(pSkill.sSp));
			}
		}

		if (!isStaffSkill(true) && !isExtendedArcherSkill(true))
		{
			if (pSkillCaster->isPlayer() && (sTargetID != -1 && pSkill.bType[0] == 4) && !pSkillCaster->isBlinking())
				pSkillCaster->MSpChange(-(pSkill.sMsp));
		}

		// We do not want to reinsert debuffs into the map (which had their expiry times reset above).
		if (!bSkillTypeAlreadyOnTarget
			&& pType->bBuffType != BUFF_TYPE_SPEED2
			&& pType->bBuffType != BUFF_TYPE_FISHING)
		{
			pBuffInfo.m_nSkillID = nSkillID;
			pBuffInfo.m_bBuffType = pType->bBuffType;
			pBuffInfo.m_bIsBuff = pType->bIsBuff;

			pBuffInfo.m_bDurationExtended = false;
			pBuffInfo.m_tEndTime = UNIXTIME + sDuration;

			// Add the buff into the buff map.
			if (pSkillCaster->isPlayer()) pBuffInfo.pownskill = TO_USER(pSkillCaster);
			pTarget->AddType4Buff(pType->bBuffType, pBuffInfo);
		}
		else if (pType->bBuffType == BUFF_TYPE_SPEED2 && bSetSpeed)
		{
			pBuffInfo.m_nSkillID = nSkillID;
			pBuffInfo.m_bBuffType = pType->bBuffType;
			pBuffInfo.m_bIsBuff = pType->bIsBuff;

			pBuffInfo.m_bDurationExtended = false;
			pBuffInfo.m_tEndTime = UNIXTIME + sDuration;

			// Add the buff into the buff map.
			if (pSkillCaster->isPlayer()) pBuffInfo.pownskill = TO_USER(pSkillCaster);
			pTarget->AddType4Buff(pType->bBuffType, pBuffInfo);
		}

		// Update character stats.
		if (pTarget->isPlayer())
		{
			TO_USER(pTarget)->SetUserAbility();

			if (pType->isBuff() && pType->bBuffType == BUFF_TYPE_HP_MP && bIsRecastingSavedMagic)
				TO_USER(pTarget)->HpChange(pTarget->m_sMaxHPAmount);

		}
	fail_return:
		if (pSkill.bType[1] == 0 || pSkill.bType[1] == 4)
		{
			Unit* pTmp = nullptr;
			if (pType->bBuffType == BUFF_TYPE_HP_MP)
			{
				if (!pSkillCaster->isInRangeSlow(pTarget, pType->bRadius))
					pTmp = pTarget;
				else
					pTmp = (pSkillCaster->isPlayer() ? pSkillCaster : pTarget);
			}
			else
				pTmp = (pSkillCaster->isPlayer() ? pSkillCaster : pTarget);

			int16 sDataCopy[] = { sData[0], bResult, sData[2], (int16)sDuration, sData[4], pType->bSpeed, sData[6] };

			if (bSetSpeed)
			{
				if (pTarget->isPlayer() && pType->bBuffType == BUFF_TYPE_SPEED2)
				{
					pTarget->m_buffLock.lock();
					bool isSpeedBuffExist = false;
					foreach(sk, pTarget->m_buffMap)
					{
						if (sk->second.m_nSkillID == 490230
							|| sk->second.m_nSkillID == 208725
							|| sk->second.m_nSkillID == 207725
							|| sk->second.m_nSkillID == 107725
							|| sk->second.m_nSkillID == 108725
							|| sk->second.m_nSkillID == 490223)
						{
							isSpeedBuffExist = true;
							break;
						}
					}
					pTarget->m_buffLock.unlock();
					if (isSpeedBuffExist)
					{
						CMagicProcess::RemoveType4Buff(BUFF_TYPE_SPEED, pTarget, false);

						Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_DURATION_EXPIRED));
						result << BUFF_TYPE_SPEED;
						TO_USER(pTarget)->Send(&result);
					}
				}

				BuildAndSendSkillPacket(pTmp, true, sCasterID, pTarget->GetID(), bOpcode, nSkillID, sDataCopy);
				if ((pSkill.bMoral >= MORAL_ENEMY) && pTarget->isPlayer())
				{
					UserStatus status = UserStatus::USER_STATUS_POISON;
					if (pType->bBuffType == (uint8)BuffType::BUFF_TYPE_SPEED)
					{
						status = UserStatus::USER_STATUS_SPEED;
						TO_USER(pTarget)->SendUserStatusUpdate(status, UserStatusBehaviour::USER_STATUS_INFLICT);
					}
					else if (pType->bBuffType == (uint8)BuffType::BUFF_TYPE_SPEED2)
					{
						status = UserStatus::USER_STATUS_SPEED;
						TO_USER(pTarget)->SendUserStatusUpdate(status, UserStatusBehaviour::USER_STATUS_INFLICT);
					}
					else
						TO_USER(pTarget)->SendUserStatusUpdate(status, UserStatusBehaviour::USER_STATUS_INFLICT);
				}
			}
			if (pType->bBuffType == BUFF_TYPE_DECREASE_RESIST || pType->bBuffType == BUFF_TYPE_RESIS_AND_MAGIC_DMG)
				SendSkill();
		}

		if (bResult == 0 && pSkillCaster && pSkillCaster->isPlayer())
		{
			if (pType && pType->isDebuff())
				return false;
			else
				SendSkillFailed((*itr)->GetID());
		}
	}

	if (pSkill.bMoral >= MORAL_ALL)
		MagicInstance::SendSkill();

	return true;
}

bool MagicInstance::ExecuteType5()
{
	// Disallow anyone that isn't a player.
	if (!pSkillCaster || !pSkillCaster->isPlayer() || pSkill.isnull())
		return false;

	_MAGIC_TYPE5* pType = g_pMain->m_Magictype5Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	vector<CUser*> casted_member;

	// Targeting a group of people (e.g. party)
	if (sTargetID == -1)
	{
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			CUser* pTUser = g_pMain->GetUserPtr(i);
			if (pTUser == nullptr || !pTUser->isInGame())
				continue;

			if (!pTUser->isInGame() || pTUser->hasBuff((uint8)BuffType::BUFF_TYPE_FREEZE))
				continue;

			// If the target's dead, only allow resurrection/self-resurrection spells.
			if (pTUser->isDead() && (pType->bType != RESURRECTION && pType->bType != RESURRECTION_SELF && pType->bType != LIFE_CRYSTAL))
				continue;

			// If the target's alive, we don't need to resurrect them.
			if (!pTUser->isDead() && (pType->bType == RESURRECTION || pType->bType == RESURRECTION_SELF || pType->bType == LIFE_CRYSTAL))
				continue;

			// Ensure the target's applicable for this skill.
			if (CMagicProcess::UserRegionCheck(pSkillCaster, pTUser, pSkill, pSkill.sRange, sData[0], sData[2]))
				casted_member.push_back(pTUser);
		}
	}
	// Targeting a single person
	else
	{
		if (pSkillTarget == nullptr)
			return false;

		if (!pSkillTarget->isPlayer())
			return false;

		// If the target's dead, only allow resurrection/self-resurrection spells.
		if (pSkillTarget->isDead() && (pType->bType != RESURRECTION && pType->bType != RESURRECTION_SELF && pType->bType != LIFE_CRYSTAL))
			return false;

		// If the target's alive, we don't need to resurrect them.
		if (!pSkillTarget->isDead() && (pType->bType == RESURRECTION || pType->bType == RESURRECTION_SELF || pType->bType == LIFE_CRYSTAL))
			return false;

		casted_member.push_back(TO_USER(pSkillTarget));
	}


	foreach(itr, casted_member)
	{
		Type4BuffMap::iterator buffIterator;
		CUser* pTUser = (*itr);

		if (pTUser == nullptr)
			continue;

		int skillCount = 0;
		bool bRemoveDOT = false;

		switch (pType->bType)
		{
			// Remove all DOT skills
		case REMOVE_TYPE3:
			for (int i = 0; i < MAX_TYPE3_REPEAT; i++)
			{
				Unit::MagicType3* pEffect = &pTUser->m_durationalSkills[i];

				if (pEffect == nullptr)
					continue;

				if (!pEffect->m_byUsed)
					continue;

				// Ignore healing-over-time skills
				if (pEffect->m_sHPAmount >= 0)
				{
					skillCount++;
					continue;
				}

				pEffect->Reset();
				// TODO: Wrap this up (ugh, I feel so dirty)
				Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_DURATION_EXPIRED));
				result << uint8(200); // removes DOT skill
				pTUser->Send(&result);
				bRemoveDOT = true;
			}

			if (skillCount == 0)
			{
				pTUser->m_bType3Flag = false;
				if (bRemoveDOT)
					pTUser->SendUserStatusUpdate(UserStatus::USER_STATUS_DOT, UserStatusBehaviour::USER_STATUS_CURE);
			}
			break;

		case REMOVE_TYPE4: // Remove type 4 debuffs
		{

			pTUser->m_buffLock.lock();
			std::vector<uint8> willDel;
			std::vector<uint8> willDel2;
			foreach(itr, pTUser->m_buffMap)
			{
				if (itr->second.isDebuff())
				{
					willDel.push_back(itr->first);
					willDel2.push_back(itr->second.m_bBuffType);
				}
			}
			pTUser->m_buffLock.unlock();
			foreach(itr, willDel) CMagicProcess::RemoveType4Buff(*itr, pTUser);
			foreach(itr, willDel2) if (pTUser->isLockableScroll(*itr)) pTUser->RecastLockableScrolls(*itr);
			pTUser->SendUserStatusUpdate(UserStatus::USER_STATUS_POISON, UserStatusBehaviour::USER_STATUS_CURE);
		}
		break;
		case RESURRECTION_SELF:
		{
			if (pSkillCaster != pTUser)
				continue;

			bool succes = false;
			if (nSkillID == 480003 ||
				nSkillID == 480004 ||
				nSkillID == 480006 ||
				nSkillID == 480010 ||
				nSkillID == 480017 ||
				nSkillID == 480002 ||
				nSkillID == 488031)
			{
				succes = true;
			}
			if (!succes) continue;
			pTUser->Regene(INOUT_IN, nSkillID);
		}
		break;
		case LIFE_CRYSTAL:
			if (pSkillCaster != pTUser)
				continue;

			pTUser->Regene(INOUT_IN, nSkillID);
			pTUser->ItemWoreOut(REPAIR_ALL, MAX_DAMAGE);
			break;

		case RESURRECTION:
			if (pTUser->CheckExistItem(pSkill.iUseItem, pType->sNeedStone))
			{
				if (pTUser->RobItem(pSkill.iUseItem, pType->sNeedStone))
				{
					TO_USER(pSkillCaster)->GiveItem("Resurrection Priest Item", pSkill.iUseItem, (pType->sNeedStone / 2) + 1);
					pTUser->Regene(INOUT_IN, nSkillID);
				}
			}
			break;

		case REMOVE_BLESS:
		{
			if (CMagicProcess::RemoveType4Buff(BUFF_TYPE_HP_MP, pTUser))
			{
				if (!pTUser->isDebuffed())
					pTUser->SendUserStatusUpdate(UserStatus::USER_STATUS_POISON, UserStatusBehaviour::USER_STATUS_CURE);
			}
		}
		break;
		}

		if (pSkill.bType[1] == 0 || pSkill.bType[1] == 5)
		{
			// Send the packet to the caster.
			sData[1] = 1;
			BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
		}
	}

	return true;
}

bool MagicInstance::ExecuteType6()
{
	if (pSkill.isnull() || pSkillCaster && !pSkillCaster->isPlayer())
		return false;

	_MAGIC_TYPE6* pType = g_pMain->m_Magictype6Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	CUser* pCaster = TO_USER(pSkillCaster);

	if (!pCaster)
		return false;

	uint16 sDuration = 0;

	if (pType->bUserSkillUse == (uint8)TransformationSkillUse::TransformationSkillOreadsGuard) return false;

	if (pType->bUserSkillUse == TransformationSkillUseMonster) // ts buga girip kitleniyordu kapatıldı!
	{
		if ((pSkillCaster && pSkillCaster->GetMap()->canAttackOtherNation() && !(pSkillCaster->isTransformationMonsterInZone()))
			|| (!bIsRecastingSavedMagic && pCaster->isTransformed())
			/*|| (pSkillCaster && pSkillCaster->isBuffed(true))*/
			|| (pType->bNation != 0 && pType->bNation != pCaster->GetNation()))
		{
			if (bIsRecastingSavedMagic)
				Type6Cancel(true);
			return false;
		}
	}
	else if (pType->bUserSkillUse == TransformationSkillUseNPC || pType->bUserSkillUse == TransformationSkillUseMamaPag || pType->bUserSkillUse == TransformationSkillUseSpecial)
	{
		if (pType->bNation != pCaster->GetNation() || (!bIsRecastingSavedMagic && pCaster->isTransformed()))
		{
			if (bIsRecastingSavedMagic)
				Type6Cancel(true);
			return false;
		}
	}
	else if (pType->bUserSkillUse == TransformationSkillUseSiege)
	{
		if (nSkillID == 450001)
		{
			if (pSkillCaster && pSkillCaster->GetZoneID() != ZONE_DELOS && pSkillCaster->GetZoneID() != ZONE_BATTLE2 && pSkillCaster->GetZoneID() != ZONE_BATTLE3)
				return false;
		}
		else if (nSkillID == 450003)
		{
			if (pSkillCaster && pSkillCaster->GetZoneID() != ZONE_BATTLE2)
				return false;

		}
		else
			if (pSkillCaster && pSkillCaster->GetZoneID() != ZONE_DELOS)
				return false;

		//if (pSkillCaster && pSkillCaster->GetMap()->canAttackOtherNation() && (!bIsRecastingSavedMagic && pCaster->isTransformed()) || (pSkillCaster && pSkillCaster->isBuffed(true)) || (pType->bNation != 0 && pType->bNation != pCaster->GetNation()))
		//{
		//	if (bIsRecastingSavedMagic)
		//		Type6Cancel(true);

		//	return false;
		//}
	}
	else if (pType->bUserSkillUse == TransformationSkillMovingTower)
	{
		if (pSkillCaster && pSkillCaster->GetZoneID() != ZONE_DELOS)
			return false;

		if ((pSkillCaster && pSkillCaster->GetMap()->canAttackOtherNation() && !(pSkillCaster->GetZoneID() == ZONE_DELOS))
			|| (!bIsRecastingSavedMagic && pCaster->isTransformed())
			|| (pSkillCaster && pSkillCaster->isBuffed(true))
			|| (pType->bNation != 0 && pType->bNation != pCaster->GetNation()))
		{
			if (bIsRecastingSavedMagic)
				Type6Cancel(true);
			return false;
		}
	}
	else if (pType->bUserSkillUse == TransformationSkillOreadsGuard)
	{
		if (pSkillCaster && pSkillCaster->GetZoneID() != ZONE_BATTLE6) return false;

		if ((!bIsRecastingSavedMagic && pCaster->isTransformed())
			|| (pSkillCaster && pSkillCaster->isBuffed(true))
			|| (pType->bNation != 0 && pType->bNation != pCaster->GetNation()))
		{
			if (bIsRecastingSavedMagic) Type6Cancel(true);
			return false;
		}
	}

	// We can ignore all these checks if we're just recasting on relog.
	if (!bIsRecastingSavedMagic)
	{
		if (pSkillTarget->HasSavedMagic(nSkillID))
			return false;

		// User's casting a new skill. Use the full duration.
		sDuration = pType->sDuration;
	}
	else
	{
		// Server's recasting the skill (kept on relog, zone change, etc.)
		int16 tmp = pSkillCaster->GetSavedMagicDuration(nSkillID);

		// Has it expired (or not been set?) -- just in case.
		if (tmp <= 0)
			return false;

		// it's positive, so no harm here in casting.
		sDuration = tmp;
	}

	switch (pType->bUserSkillUse)
	{
	case TransformationSkillUseMonster:
		pCaster->m_transformationType = Unit::TransformationType::TransformationMonster;
		break;

	case TransformationSkillUseNPC:
	case TransformationSkillUseMamaPag:
		pCaster->m_transformationType = Unit::TransformationType::TransformationNPC;
		break;

	case TransformationSkillUseSiege:
	case TransformationSkillMovingTower:
	case TransformationSkillOreadsGuard:
		pCaster->m_transformationType = Unit::TransformationType::TransformationSiege;
		break;

	default: // anything 
		return false;
	}

	// TODO : Save duration, and obviously end
	pCaster->m_sTransformID = pType->sTransformID;
	pCaster->m_sTransformSkillID = pType->iNum;
	pCaster->m_tTransformationStartTime = UNIXTIME2; // in milliseconds
	pCaster->m_sTransformationDuration = ULONGLONG(sDuration) * 1000; // in milliseconds
	pCaster->m_sTransformHpchange = true;
	pCaster->m_sTransformMpchange = true;
	pCaster->m_transformSkillUseid = (TransformationSkillUse)pType->bUserSkillUse;
	pSkillCaster->StateChangeServerDirect(3, nSkillID);

	sData[1] = 1;
	sData[3] = sDuration;
	SendSkill();

	// TODO : Give the users ALL TEH BONUSES!!
	if (pType->bUserSkillUse == TransformationSkillUseSiege || pType->bUserSkillUse == TransformationSkillMovingTower /*|| pType->bUserSkillUse == TransformationSkillOreadsGuard*/)
	{
		pCaster->m_sTotalHit = pType->sTotalHit;
		pCaster->m_sTotalAc = pType->sTotalAc;
		pCaster->m_MaxHp = pType->sMaxHp;
		pCaster->m_MaxMp = pType->sMaxMp;
		pCaster->m_sSpeed = pType->bSpeed;
		pCaster->m_sFireR = pType->sTotalFireR;
		pCaster->m_sColdR = pType->sTotalColdR;
		pCaster->m_sLightningR = pType->sTotalLightningR;
		pCaster->m_sMagicR = pType->sTotalMagicR;
		pCaster->m_sDiseaseR = pType->sTotalDiseaseR;
		pCaster->m_sPoisonR = pType->sTotalPoisonR;
		pCaster->SetUserAbility();
	}

	pSkillCaster->InsertSavedMagic(nSkillID, sDuration);
	return true;
}

bool MagicInstance::ExecuteType7()
{
	if (!pSkillCaster || pSkill.isnull())
		return false;

	_MAGIC_TYPE7* pType = g_pMain->m_Magictype7Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	vector<Unit*> casted_member;

	int damage = pType->sDamage;

	if (sTargetID == -1)
	{
		std::vector<uint16> unitList;

		g_pMain->GetUnitListFromSurroundingRegions(pSkillCaster, &unitList);
		foreach(itr, unitList)
		{
			Unit* pTarget = g_pMain->GetUnitPtr(*itr, sSkillCasterZoneID);

			if (pTarget == nullptr)
				continue;

			if (pTarget->isPlayer() && TO_USER(pTarget)->isGM())
				continue;

			if (pSkillCaster != pTarget && !pTarget->isDead() && !pTarget->isBlinking() && pTarget->isAttackable()
				&& CMagicProcess::UserRegionCheck(pSkillCaster, pTarget, pSkill, pType->bRadius, sData[0], sData[2]))
				casted_member.push_back(pTarget);
		}

		// If you managed to not hit anything with your AoE, you're still gonna have a cooldown (You should l2aim)
		if (casted_member.empty() || (sTargetID == -1 && casted_member.empty()))
		{
			SendSkill();
			return false;
		}
	}
	else
	{
		// If the target was a single unit.
		if (pSkillTarget == nullptr || pSkillTarget->isDead() || (pSkillTarget->isPlayer() && TO_USER(pSkillTarget)->isBlinking()))
			return false;

		casted_member.push_back(pSkillTarget);
	}

	sData[1] = 0x01;

	foreach(itr, casted_member)
	{
		Unit* pTarget = *itr;

		if (pTarget == nullptr || pTarget->isDead())
			continue;

		if (pTarget->isDead())
			continue;

		//if (pSkill && pSkill.sRange > 0 && (pSkillCaster->GetDistanceSqrt(pTarget) >= (float)pSkill.sRange))
		//	continue;

		if (pType && pType->bTargetChange == 0)
		{
			if (pSkillCaster && pSkillCaster->isNPC())
			{
				CUser* pUser = g_pMain->GetUserPtr(pTarget->GetID());
				if (pUser == nullptr)
				{
					CNpc* pNpc = g_pMain->GetNpcPtr(pTarget->GetID(), pTarget->GetZoneID());
					if (pNpc == nullptr)
						return false;

					TO_NPC(pSkillCaster)->ChangeNTarget(pNpc);
				}
				else
					TO_NPC(pSkillCaster)->ChangeTarget(0, pUser);
			}
		}

		if (pType && pType->bTargetChange == 1)
		{
			if (pSkillCaster && pSkillCaster->isNPC())
			{
				if (TO_NPC(pSkillCaster)->isPet())
				{
					uint8 bResult = ATTACK_FAIL;

					damage = pSkillCaster->GetDamage(pTarget, pSkill);

					if (damage > 0)
					{
						pTarget->HpChange(-(damage), pSkillCaster);
						if (pTarget->isDead())
							bResult = ATTACK_TARGET_DEAD;
						else
							bResult = ATTACK_SUCCESS;
					}

					Packet result(WIZ_ATTACK, uint8(LONG_ATTACK));
					result << bResult << TO_NPC(pSkillCaster)->GetID() << pTarget->GetID();
					TO_NPC(pSkillCaster)->SendToRegion(&result);
					BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, pTarget->GetID(), bOpcode, nSkillID, sData);
					return true;
				}
			}

			if (damage < 0)
				continue;

			if (pTarget->isPlayer())
				continue;

			pTarget->HpChange(-damage, pSkillCaster);
		}

		if (pType->bTargetChange == 2)
		{
			if (!pTarget->isNPC())
				return false;

			pTarget->StateChangeServerDirect(1, 4);
			TO_NPC(pTarget)->m_tFaintingTime = UNIXTIME + pType->sDuration;  // pSkillTarget yazınca Priest uyutma skili patlıyor.
		}
	}
	return true;
}

bool MagicInstance::ExecuteType8()
{
	if (pSkill.isnull() || pSkillCaster == nullptr || pSkillCaster->GetMap() == nullptr)
		return false;

	_MAGIC_TYPE8* pType = g_pMain->m_Magictype8Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	vector<Unit*> casted_member;

	if (sTargetID == -1)
	{
		for (uint16 i = 0; i < MAX_USER; i++)
		{
			Unit* pTUser = g_pMain->GetUserPtr(i);

			if (pTUser == nullptr || (pTUser->isPlayer() && !TO_USER(pTUser)->isInGame()))
				continue;

			if ((pTUser->isPlayer() && !TO_USER(pTUser)->isInGame()))
				continue;

			if (CMagicProcess::UserRegionCheck(pSkillCaster, pTUser, pSkill, pType->sRadius, sData[0], sData[2]))
				casted_member.push_back(pTUser);
		}

		if (casted_member.empty())
			return false;
	}
	else
	{	// If the target was another single player.
		Unit* pTUser = g_pMain->GetUnitPtr(sTargetID, sSkillCasterZoneID);

		if (pTUser == nullptr)
			return false;

		if (pTUser->isPlayer() && !TO_USER(pTUser)->isInGame())
			return false;

		if (pSkill.bMoral == MORAL_PARTY && pSkillCaster == pTUser)
			casted_member.push_back(pTUser);
		else if (pSkill.bMoral == MORAL_PARTY_ALL && CMagicProcess::UserRegionCheck(pSkillCaster, pTUser, pSkill, pType->sRadius, sData[0], sData[2]))
			casted_member.push_back(pTUser);
		else
			casted_member.push_back(pTUser);
	}

	foreach(itr, casted_member)
	{
		Unit* pTUser = *itr;

		if (pTUser == nullptr || (pTUser->isPlayer() && !TO_USER(pTUser)->isInGame()))
			continue;

		uint8 bResult = 0;
		_OBJECT_EVENT* pEvent = nullptr;

		if (pType && pType->bWarpType != 11)
		{   // Warp or summon related: targets CANNOT be dead.
			if (pTUser->isDead() || !pTUser->canTeleport())
			{
				sData[1] = 0;
				BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
				continue;
			}
		}
		// Resurrection related: we're reviving DEAD targets.
		else if (!pTUser->isDead())
		{
			sData[1] = 0;
			BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
			continue;
		}

		// Is target already warping?			
		if (pTUser->isPlayer() && TO_USER(pTUser)->m_bWarp)
		{
			sData[1] = 0;
			BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
			continue;
		}

		switch (pType->bWarpType)
		{
		case 1:	// Send source to resurrection point. Disable gate escape etc for forgetten temple and pvp zones...
		{
			if (!pSkillCaster->isPlayer() || !pTUser->isPlayer()) { SendSkillFailed(); return false; }
			CUser* pCaster = TO_USER(pSkillCaster), * pTarget = TO_USER(pTUser);
			if (pTarget->GetMap() == nullptr)
				continue;

			if (pSkill.bMoral != 1 && pSkillCaster->GetMap()->isEscapeZone() != 1)
			{
				SendSkillFailed();
				return false;
			}

			if (pSkill.iNum == 109035 || pSkill.iNum == 110035 || pSkill.iNum == 209035 || pSkill.iNum == 210035)
			{
				if (g_pMain->m_byBattleOpen == NATION_BATTLE && (pSkillCaster->isInLufersonCastle() || pSkillCaster->isInElmoradCastle()))
				{
					SendSkillFailed();
					return false;
				}

				if (pCaster->isInParty() && (!pTarget->isInParty() || pCaster->GetPartyID() != pTarget->GetPartyID()))
					continue;
			}

			if (pSkill.iNum == 109015 || pSkill.iNum == 110015 || pSkill.iNum == 111700 || pSkill.iNum == 112700 ||
				pSkill.iNum == 209015 || pSkill.iNum == 210015 || pSkill.iNum == 211700 || pSkill.iNum == 212700)
			{
				if (pSkillCaster->GetMap()->isGateZone() != 1)
				{
					SendSkillFailed();
					return false;
				}
			}

			sData[1] = 1;
			BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);

			if (pTUser->GetMap() == nullptr)
				continue;

			pEvent = pTUser->GetMap()->GetObjectEvent(TO_USER(pTUser)->m_sBind);

			if (pEvent != nullptr)
				TO_USER(pTUser)->Warp(uint16(pEvent->fPosX * 10), uint16(pEvent->fPosZ * 10));
			else
			{
				if (pTUser->GetZoneID() == ZONE_JURAID_MOUNTAIN)
					TO_USER(pTUser)->JuraidMountainWarp();
				else
				{
					if (pTUser->GetZoneID() <= (uint8)Nation::ELMORAD || pTUser->GetMap()->isWarZone() || pTUser->GetMap()->canAttackOtherNation())
					{
						_START_POSITION* pStartPosition = g_pMain->m_StartPositionArray.GetData(pTUser->GetZoneID());
						if (pStartPosition)
							TO_USER(pTUser)->Warp((uint16)((pTUser->GetNation() == (uint8)Nation::KARUS ? pStartPosition->sKarusX : pStartPosition->sElmoradX) + myrand(0, pStartPosition->bRangeX)) * 10, (uint16)((pTUser->GetNation() == (uint8)Nation::KARUS ? pStartPosition->sKarusZ : pStartPosition->sElmoradZ) + myrand(0, pStartPosition->bRangeZ)) * 10);
						else return false;
					}
					else
					{
						_START_POSITION* pStartPosition = g_pMain->m_StartPositionArray.GetData(pTUser->GetZoneID());
						if (pStartPosition)
							TO_USER(pTUser)->Warp((uint16)((pTUser->GetNation() == (uint8)Nation::KARUS ? pStartPosition->sKarusX : pStartPosition->sElmoradX) + myrand(0, pStartPosition->bRangeX)) * 10, (uint16)((pTUser->GetNation() == (uint8)Nation::KARUS ? pStartPosition->sKarusZ : pStartPosition->sElmoradZ) + myrand(0, pStartPosition->bRangeZ)) * 10);

					}
				}
			}
		}
		break;
		case 2: // Chamber of Teleport Skills / BATTLE & BIFROST
		{
			_MAGIC_TYPE8* pType = g_pMain->m_Magictype8Array.GetData(nSkillID);
			if (pType == nullptr)
				return false;

			if (pSkillCaster && pSkillCaster->GetZoneID() != ZONE_BIFROST && pSkillCaster->GetZoneID() != ZONE_BATTLE2)
				return false;

			if (pSkillCaster && pSkillCaster->isPlayer() && pSkillCaster->isAlive())
			{
				if (pSkillCaster->GetZoneID() == ZONE_BIFROST)
				{
					if (nSkillID == 490301) //Chamber of Arrogance Transport Scroll
					{
						uint16 x, z;
						x = 227;
						z = 819;

						TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
					}
					else if (nSkillID == 490302) //Chamber of Gluttony Transport Scroll
					{
						uint16 x, z;
						x = 770;
						z = 818;

						TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
					}
					else if (nSkillID == 490303) //Chamber of Rage Transport Scroll
					{
						uint16 x, z;
						x = 671;
						z = 355;

						TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
					}
					else if (nSkillID == 490304) //Chamber of Sloth Transport Scroll
					{
						uint16 x, z;
						x = 498;
						z = 396;

						TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
					}
					else if (nSkillID == 490305) //Chamber of Lechery Transport Scroll
					{
						uint16 x, z;
						x = 102;
						z = 140;

						TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
					}
					else if (nSkillID == 490306) //Chamber of Jealousy Transport Scroll
					{
						uint16 x, z;
						x = 440;
						z = 188;

						TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
					}
					else if (nSkillID == 490307) //Chamber of Avarice Transport Scroll
					{
						uint16 x, z;
						x = 712;
						z = 183;

						TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
					}
				}
				else if (pSkillCaster->GetZoneID() == ZONE_BATTLE2)
				{
					if (nSkillID == 490301 || nSkillID == 490302 || nSkillID == 490303 || nSkillID == 490304 || nSkillID == 490305 || nSkillID == 490306 || nSkillID == 490307)
					{
						if (pSkillCaster->GetNation() == (uint8)Nation::ELMORAD)
						{
							uint16 x, z;
							x = 600;
							z = 340;

							TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
						}
						else if (pSkillCaster->GetNation() == (uint8)Nation::KARUS)
						{
							uint16 x, z;
							x = 394;
							z = 632;

							TO_USER(pSkillCaster)->Warp(x * 10, z * 10);
						}
					}
				}
				else
				{
					sData[1] = 0;
					BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
					return false;
				}
			}
			else
			{
				sData[1] = 0;
				BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
				return false;
			}
		}
		break;
		case 3:
			//CASUSLUK
			break;
		case 5:		// Send target to a hidden zone.
			// LATER!!!
			break;
		case 11:// Resurrect a dead player.
		{
			if (pSkillTarget && !pSkillTarget->isAlive())
			{
				if (pSkillCaster->isPlayer() && pSkillTarget->isPlayer())
				{
					// Send the packet to the target.
					sData[1] = 1;
					BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);

					TO_USER(pTUser)->m_bResHpType = USER_STANDING;     // Target status is officially alive now.
					TO_USER(pTUser)->HpChange(TO_USER(pTUser)->m_MaxHp, pSkillCaster);	 // Refill HP.	
					TO_USER(pTUser)->ExpChange("exp restore back", pType->sExpRecover / 100, true);    // Increase target experience.
				}
				else
					return false;
			}
			else
				return false;
		}
		break;
		case 12:// Summon a target within the zone.	Disable telepert for forgetten temple...
		{
			if (!pSkillCaster->isPlayer() || !pTUser->isPlayer()) { SendSkillFailed(); return false; }
			CUser* pCaster = TO_USER(pSkillCaster), * pTarget = TO_USER(pTUser);

			if (nSkillID == 490042 || (nSkillID == 490050 && pCaster->GetMap()->isCallingFriendZone() != 1))
			{
				SendSkillFailed();
				return false;
			}

			if ((nSkillID == 109004 || nSkillID == 110004 || nSkillID == 209004 || nSkillID == 210004)
				&& (pCaster->GetMap()->isTeleportFriendZone() != 1 || pCaster == pTarget))
			{
				SendSkillFailed();
				return false;
			}

			if ((pSkillCaster->isInElmoradCastle() || pSkillCaster->isInLufersonCastle()) && (nSkillID == 490050 || nSkillID == 490042))
			{
				if (g_pMain->m_byBattleOpen == NATION_BATTLE || pSkillCaster->GetMap()->isCallingFriendZone() != 1)
				{
					SendSkillFailed();
					return false;
				}
			}

			if (pTarget->GetZoneID() == ZONE_FORGOTTEN_TEMPLE && (nSkillID == 490050 || nSkillID == 490042))
			{
				SendSkillFailed();
				return false;
			}

			// Cannot teleport users from other zones.
			if (pSkillCaster->GetZoneID() != pTarget->GetZoneID() || !pTarget->isInParty()
				|| pCaster->GetPartyID() != pTarget->GetPartyID() || pTarget->isBlinking())
			{
				SendSkillFailed();
				return false;
			}

			if (pTarget->m_bHasAlterOptained)
			{
				SendSkillFailed();
				continue;
			}

			sData[1] = 1;
			BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
			TO_USER(pTarget)->Warp(pSkillCaster->GetSPosX(), pSkillCaster->GetSPosZ());
		}
		break;
		case 13:// Summon a target outside the zone.		
			// Different zone? 
			if (pSkillCaster && pSkillCaster->GetZoneID() == pTUser->GetZoneID())
			{
				sData[1] = 0;
				BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
				return false;
			}

			// Send the packet to the target.
			sData[1] = 1;
			BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);

			TO_USER(pTUser)->ZoneChange(pSkillCaster->GetZoneID(), pSkillCaster->GetX(), pSkillCaster->GetZ());
			TO_USER(pTUser)->UserInOut(INOUT_RESPAWN);
			break;

		case 20:	// Teleport the source (radius) meters forward
		{
			float warp_x = float(sData[0]), warp_z = float(sData[2]), warp_y = float(sData[1]);
			TO_USER(pTUser)->Warp(uint16_t(warp_x), uint16_t(warp_z));
			BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
		}
		break;

		case 21:// Summon a monster within a zone with monster staff.
		{
			MonsterSummonList* pMonsterSummonListStaff = g_pMain->m_MonsterSummonList.GetData(1);

			//bType = 1 : Monster summon Item
			if (nSkillID == 490088
				|| nSkillID == 490093
				|| nSkillID == 490096
				|| nSkillID == 490097)
			{
				if (pMonsterSummonListStaff)
				{
					int nCurrentMonster = 0;
					int nRandom = myrand(0, (int32)pMonsterSummonListStaff->size());

					for (std::vector<_MONSTER_SUMMON_LIST>::iterator itr = pMonsterSummonListStaff->begin(); itr != pMonsterSummonListStaff->end(); ++itr)
					{
						if (nCurrentMonster == nRandom)
						{
							g_pMain->SpawnEventNpc(itr->sSid, true, pSkillCaster->GetZoneID(), pSkillCaster->GetX(), pSkillCaster->GetY(), pSkillCaster->GetZ(), 1, (pType->sRadius / 1000));
							break;
						}
						nCurrentMonster++;
					}
				}
			}
			//bType = 2: Monster Summon Special Item
			if (nSkillID == 492015)
			{
				MonsterSummonList* pMonsterSummonListSpecialStaff = g_pMain->m_MonsterSummonList.GetData(2);
				if (pMonsterSummonListSpecialStaff)
				{
					int nCurrentMonster = 0;
					int nRandom = myrand(0, (int32)pMonsterSummonListSpecialStaff->size());

					for (std::vector<_MONSTER_SUMMON_LIST>::iterator itr = pMonsterSummonListSpecialStaff->begin(); itr != pMonsterSummonListSpecialStaff->end(); ++itr)
					{
						if (nCurrentMonster == nRandom)
						{
							g_pMain->SpawnEventNpc(itr->sSid, true, pSkillCaster->GetZoneID(), pSkillCaster->GetX(), pSkillCaster->GetY(), pSkillCaster->GetZ(), 1, (pType->sRadius / 1000));
							break;
						}
						nCurrentMonster++;
					}
				}
			}
			//bType = 3: Monster Summon Higher Staff
			if (nSkillID == 500202)
			{
				MonsterSummonList* pMonsterSummonListHigherStaff = g_pMain->m_MonsterSummonList.GetData(3);
				if (pMonsterSummonListHigherStaff)
				{
					int nCurrentMonster = 0;
					int nRandom = myrand(0, (int32)pMonsterSummonListHigherStaff->size());

					for (std::vector<_MONSTER_SUMMON_LIST>::iterator itr = pMonsterSummonListHigherStaff->begin(); itr != pMonsterSummonListHigherStaff->end(); ++itr)
					{
						if (nCurrentMonster == nRandom)
						{
							g_pMain->SpawnEventNpc(itr->sSid, true, pSkillCaster->GetZoneID(), pSkillCaster->GetX(), pSkillCaster->GetY(), pSkillCaster->GetZ(), 1, (pType->sRadius / 1000));
							break;
						}
						nCurrentMonster++;
					}
				}
			}
		}
		break;
		case 25:// This is used by Wild Advent (70 rogue skill) and Descent, teleport the user to the target user (Moral check to distinguish between the 2 skills)
		{
			//float dest_x, dest_z = 0.0f;
			//// If we're not even in the same zone, I can't teleport to you!
			//if ((pSkill.bMoral != MORAL_ENEMY && pSkill.bMoral != MORAL_PARTY)
			//	|| (pSkill.iNum > 500000 && pSkillCaster->GetZoneID() > ZONE_MORADON5))
			//	return false;

			//if ( pTUser->GetZoneID() != pSkillCaster->GetZoneID()
			//	|| (pSkill.bMoral < MORAL_ENEMY && pSkillCaster->isHostileTo(pTUser)))
			//	return false;

			//if (pSkill.iNum != 108770 && pSkill.iNum != 208770) {
			//	if (pSkillCaster->GetMap()->isTeleportZone() != 1)
			//		return false;
			//}

			//dest_x = pTUser->GetX();
			//dest_z = pTUser->GetZ();

			//if (pSkillCaster->isPlayer() && (pSkillCaster->GetDistanceSqrt(pSkillTarget) <= (float)pType->sRadius))
			//	TO_USER(pSkillCaster)->Warp(uint16(dest_x * 10), uint16(dest_z * 10));
			//else
			//	SendSkillFailed();

			if (!pSkillCaster->isPlayer() || !pTUser->isPlayer()) { SendSkillFailed(); return false; }
			CUser* pCaster = TO_USER(pSkillCaster), * pTarget = TO_USER(pTUser);
			if (!pCaster || !pTarget) continue;

			if (pCaster->GetDistanceSqrt(pTarget) > (float)pType->sRadius)
			{
				SendSkillFailed();
				return false;
			}

			// If we're not even in the same zone, I can't teleport to you!
			if ((pSkill.bMoral != MORAL_ENEMY && pSkill.bMoral != MORAL_PARTY) || (pSkill.iNum > 500000 && pCaster->GetZoneID() > ZONE_MORADON5))
			{
				SendSkillFailed();
				return false;
			}

			if (pTarget->GetZoneID() != pCaster->GetZoneID() || (pSkill.bMoral < MORAL_ENEMY && pCaster->isHostileTo(pTarget)))
			{
				SendSkillFailed();
				return false;
			}

			if (pTarget->m_bHasAlterOptained)
			{
				SendSkillFailed();
				return false;
			}

			/*bool nocheck = false;
			nocheck = pTarget->isInPKZone() && (pSkill.iNum == 105650 || pSkill.iNum == 106650 || pSkill.iNum == 114650 ||
				pSkill.iNum == 115650 || pSkill.iNum == 205650 || pSkill.iNum == 206650 || pSkill.iNum == 214650 || pSkill.iNum == 215650);*/

			if (/*!nocheck && */pSkill.iNum != 108770 && pSkill.iNum != 208770 && pCaster->GetMap()->isTeleportZone() != 1)
			{
				SendSkillFailed();
				return false;
			}

			float dest_x = (pTarget->curX1 / 10.0f + pTarget->m_oldwillx / 10.0f) / 2;
			float dest_z = (pTarget->curZ1 / 10.0f + pTarget->m_oldwillz / 10.0f) / 2;
			pCaster->Warp(uint16(dest_x * 10), uint16(dest_z * 10));
		}
		break;
		case 26:// Teleport the source (radius) meters forward
		{
			if (pSkillCaster->GetMap()->isBlinkZone() != 1)
				return false;

			float warp_x = (float(sData[0])) / 10, warp_z = (float(sData[2])) / 10;

			sData[1] = 1;
			BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
			TO_USER(pTUser)->Warp(uint16(warp_x * 10), uint16(warp_z * 10));
		}
		break;
		case 27:
		{
			if (pSkillCaster->GetMap()->isTeleportZone() != 1)
				return false;

			if (pSkill.iNum == 500038 && pSkillCaster->GetZoneID() >= ZONE_SPBATTLE1 && pSkillCaster->GetZoneID() <= ZONE_SPBATTLE12)
			{
				SendSkillFailed();
				return false;
			}

			float dest_x, dest_z = 0.0f;
			// If we're not even in the same zone, I can't teleport to you!

			if (pSkillCaster->GetZoneID() != pSkillTarget->GetZoneID()
				|| pSkillCaster->GetNation() != pSkillTarget->GetNation())
			{
				SendSkillFailed();
				return false;
			}

			dest_x = (TO_USER(pTUser)->curX1 / 10.0f + TO_USER(pTUser)->m_oldwillx / 10.0f) / 2;
			dest_z = (TO_USER(pTUser)->curZ1 / 10.0f + TO_USER(pTUser)->m_oldwillz / 10.0f) / 2;

			if (pSkillCaster->isPlayer())
				TO_USER(pSkillCaster)->Warp(uint16(dest_x * 10), uint16(dest_z * 10));
			else
				SendSkillFailed();
		}
		break;
		case 28:
			break;
		case 29:
		{
			float warp_x = pTUser->GetX() - pSkillCaster->GetX(), warp_z = pTUser->GetZ() - pSkillCaster->GetZ();

			// Unable to work out orientation, so we'll just fail (won't be necessary with m_sDirection).
			float	distance = sqrtf(warp_x * warp_x + warp_z * warp_z);
			if (distance == 0.0f)
			{
				sData[1] = 0;
				BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
				return false;
			}

			g_pMain->pSoccerEvent.m_SoccerSocketID = TO_USER(pSkillCaster)->GetSocketID();
			warp_x /= distance; warp_z /= distance;
			warp_x *= pType->sRadius; warp_z *= pType->sRadius;
			warp_x += TO_USER(pSkillCaster)->m_oldx; warp_z += TO_USER(pSkillCaster)->m_oldz;

			sData[1] = 1;
			BuildAndSendSkillPacket(*itr, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);

			TO_NPC(pTUser)->SendMoveResult(warp_x, 0, warp_z, distance);
		}
		break;
		case 30: // For Kurian Rush
		{
			if (pSkillTarget->isPlayer() && pSkillTarget->isAlive() && pSkillCaster->isPlayer())
			{
				if (isRushSkill(false) && pSkillTarget->isPlayer())
				{
					nSkillID += SKILLPACKET;
					pSkill = g_pMain->GetMagicPtr(nSkillID);
					if (pSkill.isnull())
						return false;

					ExecuteType4();

					_MAGIC_TYPE4* pType2 = g_pMain->m_Magictype4Array.GetData(nSkillID);
					nSkillID -= SKILLPACKET;

					if (pType2 == nullptr)
						return false;

					sData[1] = 1;
					BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
				}
			}
			else
				return false;
		}
		break;
		case 31:// For Kurian Pull
		{
			// kurian güncellemesi 16.12.2023
			if (pSkillCaster == nullptr || pSkillTarget == nullptr)
				return false;

			if (pSkillTarget->isPlayer() && pSkillTarget->isAlive() && pSkillCaster->isPlayer() && pSkillCaster != pSkillTarget)
			{
				bool luck = false;
				_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(nSkillID);
				if (pSkill == nullptr)
					return false;

				if (5 < myrand(0, 10))
					luck = false;
				else
					luck = true;

				sData[1] = luck;
				BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
			}
			else
				return false;
			// kurian güncellemesi 16.12.2023 end
		}
		break;
		//case 30: // For Kurian Rush
		//{
		//	if (pSkillTarget->isPlayer() && pSkillTarget->isAlive() && pSkillCaster->isPlayer())
		//	{
		//		if (pSkillTarget->isPlayer() && TO_USER(pSkillCaster)->CheckSkillPoint(PRO_SKILL1, 45, 83))
		//		{
		//			_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(nSkillID);
		//			if (pSkill == nullptr)
		//				return false;

		//			nSkillID += SKILLPACKET;
		//			pSkill = g_pMain->m_MagictableArray.GetData(nSkillID);
		//			if (pSkill == nullptr)
		//				return false;

		//			ExecuteType4();

		//			_MAGIC_TYPE4* pType2 = g_pMain->m_Magictype4Array.GetData(nSkillID);
		//			nSkillID -= SKILLPACKET;

		//			if (pType2 == nullptr)
		//				return false;

		//			sData[1] = 1;
		//			BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
		//		}
		//		else
		//		{
		//			sData[1] = 1;
		//			BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
		//		}
		//	}
		//	else
		//		return false;
		//}
		//break;
		//case 31:// For Kurian Pull
		//{
		//	if (pSkillTarget->isPlayer() && pSkillTarget->isAlive() && pSkillCaster->isPlayer())
		//	{
		//		bool luck = false;
		//		_MAGIC_TABLE* pSkill = g_pMain->m_MagictableArray.GetData(nSkillID);
		//		if (pSkill == nullptr)
		//			return false;

		//		if (75 <= myrand(0, 100))
		//			luck = false;
		//		else
		//			luck = true;

		//		sData[1] = luck;
		//		BuildAndSendSkillPacket(pSkillCaster, true, sCasterID, (*itr)->GetID(), bOpcode, nSkillID, sData);
		//	}
		//	else
		//		return false;
		//}
		//break;
		default:
			printf("Magic Instance ExecuteType8 Not Warp Type %d \n", pType->bWarpType);
			break;
		}
	}
	return true;
}

bool MagicInstance::ExecuteType9()
{
	if (pSkill.isnull() || !pSkillCaster || pSkillCaster->GetMap() == nullptr)
		return false;

	_MAGIC_TYPE9* pType = g_pMain->m_Magictype9Array.GetData(nSkillID);
	if (pType == nullptr)
		return false;

	Unit* pCaster = pSkillCaster;

	if (pCaster == nullptr)
		return false;

	sData[1] = 0;

	Guard lock(pSkillCaster->m_buff9lock);
	Type9BuffMap& buffMap = pSkillCaster->m_type9BuffMap;
	if (pCaster->isPlayer() && buffMap.find(pType->bStateChange) != buffMap.end())
		return false;

	sData[1] = 1;

	if (pType->bStateChange == 1 || pType->bStateChange == 2)
	{
		if (!pSkillTarget->isPlayer())
			return false;

		if (pSkillCaster->GetZoneID() == ZONE_FORGOTTEN_TEMPLE || pSkillCaster->GetZoneID() == ZONE_DUNGEON_DEFENCE)
			return false;

		if (pSkillCaster->isPlayer() && pSkillCaster->canStealth())
		{

			if (TO_USER(pSkillCaster)->m_bInvisibilityType != (uint8)InvisibilityType::INVIS_NONE)
			{
				SendSkillFailed();
				return false;
			}

			pSkillCaster->StateChangeServerDirect(7, pType->bStateChange); // Update the client to be invisible
			buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, (uint32)UNIXTIME + pType->sDuration)));
			pSkillTarget->m_type9BuffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
			sData[3] = pType->sDuration;

			// Ensure every user in the party is given the skill icon in the corner of the screen.
			BuildAndSendSkillPacket(pSkillCaster, false, sCasterID, TO_USER(pSkillCaster)->GetID(), bOpcode, nSkillID, sData);
			return true;
		}

		return false;
	}
	else if (pType->bStateChange >= 3 && pType->bStateChange <= 4)
	{
		Packet result(WIZ_STEALTH, uint8(1));
		result << pType->sRadius;

		// If the player's in a party, apply this skill to all members of the party.
		if (TO_USER(pCaster)->isInParty() && pType->bStateChange == 4)
		{
			_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(TO_USER(pCaster)->GetPartyID());
			if (pParty == nullptr)
				return false;

			for (int i = 0; i < MAX_PARTY_USERS; i++)
			{
				CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
				if (pUser == nullptr)
					continue;

				// If this user already has this skill active, we don't need to reapply it.
				if (pUser->m_type9BuffMap.find(pType->bStateChange) != pUser->m_type9BuffMap.end())
					continue;

				pUser->m_type9BuffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				pUser->Send(&result);

				// Ensure every user in the party is given the skill icon in the corner of the screen.
				BuildAndSendSkillPacket(pUser, false, sCasterID, pUser->GetID(), bOpcode, nSkillID, sData);
			}
		}
		else // not in a party, so just apply this skill to us.
		{
			buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
			TO_USER(pCaster)->Send(&result);

			// Ensure we are given the skill icon in the corner of the screen.
			SendSkill(false); // only update us, as only we need to know that we can see invisible players.
		}
	}
	else if (pType->bStateChange == 9)
	{
		if (pType->sMonsterNum == 8850)
		{
			if (pCaster->GetMap()->isGuardSummonZone() != 1)
				return false;
		}

		buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
		g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, 2, pType->sDuration, pCaster->GetNation(), pCaster->GetID(), pCaster->GetEventRoom(), 0, 0, 0, SpawnEventType::GuardSummon);
		SendSkill();
	}
	else if (pType->bStateChange == 7)
	{
		buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
		g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, 2, pType->sDuration, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
	}
	else if (pType->bStateChange == 8)
	{
		if (pCaster->GetMap()->m_bPetSpawn != 1)
		{
			sData[1] = 0;
			SendSkillFailed();
			return false;
		}
		//Pet Fonksiyonu kapatıldı.
		//return false;
		g_pMain->m_PetSystemlock.lock();
		PetDataSystemArray::iterator itr = g_pMain->m_PetDataSystemArray.find(TO_USER(pCaster)->GetItem(SHOULDER)->nSerialNum);
		if (itr == g_pMain->m_PetDataSystemArray.end() || itr->second->info == nullptr)
		{
			sData[1] = 0;
			SendSkillFailed();
			g_pMain->m_PetSystemlock.unlock();
			return false;
		}
		g_pMain->m_PetSystemlock.unlock();
		CNpc* pPet = g_pMain->GetPetPtr(TO_USER(pCaster)->GetSocketID(), TO_USER(pCaster)->GetZoneID());
		if (pPet != nullptr)
		{
			sData[1] = 0;
			SendSkillFailed();
			return false;
		}

		TO_USER(pCaster)->m_PettingOn = itr->second;
		g_pMain->SpawnEventPet(MONSTER_KAUL_PET_SYSTEM, false, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, 0, -1, TO_USER(pCaster)->GetNation(), TO_USER(pCaster)->GetSocketID(), 0, 1, nSkillID);
		buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, (UNIXTIME + (pType->sDuration)))));
		TO_USER(pSkillCaster)->m_petrespawntime = UNIXTIME + 1;
	}
	else if (pType->bStateChange == 10
		|| pType->bStateChange == 11
		|| pType->bStateChange == 12
		|| pType->bStateChange == 13)
	{
		switch (pType->bStateChange)
		{
		case 10:
			switch (pType->iNum)
			{
			case 502022:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 4, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502013:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 4, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502014:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 5, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502026:
			case 502027:
			case 502017:
			case 502018:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			default:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, pType->sDuration, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			}
			break;
		case 11:
			switch (pType->iNum)
			{
			case 502024:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 4, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502025:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 3, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502021:
			case 502031:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502019:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 5, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502028:
			case 502029:
			case 502030:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			default:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, pType->sDuration, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			}
			break;
		case 12:
			switch (pType->iNum)
			{
			case 502016:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			default:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, pType->sDuration, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			}
			break;
		case 13:
			switch (pType->iNum)
			{
			case 502023:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 4, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502015:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 5, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			case 502020:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, 0, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			default:
				buffMap.insert(std::make_pair(pType->bStateChange, _BUFF_TYPE9_INFO(nSkillID, UNIXTIME + pType->sDuration)));
				g_pMain->SpawnEventNpc(pType->sMonsterNum, true, pCaster->GetZoneID(), pCaster->GetX(), pCaster->GetY(), pCaster->GetZ(), 1, pType->sRadius, pType->sDuration, pType->bNationChange, pCaster->GetID(), pCaster->GetEventRoom());
				break;
			}
			break;
		default:
			break;
		}
	}
	return true;
}
#if 0
short MagicInstance::GetMagicDamage(Unit* pTarget, int total_hit, int attribute)
{
	int32 temp_hit = 0, damage = 0;
	short righthand_damage = 0, attribute_damage = 0, lastdamage = 0;
	int32 temp_ac = 0, temp_ap = 0, temp_hit_B = 0;
	int random = 0, total_r = 0;
	uint8 result; bool isChaSkill = true;
	uint16 bStat = 1;

	if (pTarget == nullptr || pSkillCaster == nullptr || pTarget->isDead() || pSkillCaster->isDead())
		return 0;

	// Trigger item procs
	if (pTarget->isPlayer() && pSkillCaster->isPlayer())
	{
		pSkillCaster->OnAttack(pTarget, AttackType::AttackTypeMagic);
		pTarget->OnDefend(pSkillCaster, AttackType::AttackTypeMagic);
	}

	if (pTarget->m_bBlockMagic)
		return 0;

	int16 sMagicAmount = 0;
	if (pSkillCaster->isNPC()) result = pSkillCaster->GetHitRate(pTarget->m_fTotalHitrate / pSkillCaster->m_fTotalEvasionrate + 2.0f);
	else
	{
		if (pSkillCaster->isPlayer())
		{
			CUser* pUser = TO_USER(pSkillCaster);
			if (pUser == nullptr) return 0;

			if (pUser->isMage())
				bStat = pTarget->isPlayer() ? pUser->GetStat(StatType::STAT_CHA) : pUser->GetStatWithItemBonus(StatType::STAT_CHA);
			else
			{
				isChaSkill = false;
				bStat = pUser->GetStatWithItemBonus(StatType::STAT_STR);
				temp_ap = pUser->m_sTotalHit * pUser->m_bAttackAmount / 5;
				temp_ap = temp_ap * pUser->m_bPlayerAttackAmount / 100;
				temp_ac = pTarget->m_sTotalAc + pTarget->m_sACAmount;
				// A unit's total AC shouldn't ever go below 0.
				if (temp_ac < 0) temp_ac = 0;
			}

			sMagicAmount = pUser->m_sMagicAttackAmount + 100;
			if (isChaSkill) total_hit = (int)ceil((total_hit * bStat / 102.5f));
			total_hit = (total_hit * sMagicAmount) / 100;
			result = SUCCESS;
		}
		else
		{
			CBot* pBot = TO_BOT(pSkillCaster);

			if (pBot->isMage())
				bStat = pBot->GetStatWithItemBonus(STAT_CHA);
			else
			{
				isChaSkill = false;
				bStat = pBot->GetStatWithItemBonus(STAT_STR);
				temp_ap = pBot->m_sTotalHit * pBot->m_bAttackAmount / 5;
				temp_ap = temp_ap * 10 / 100;
				temp_ac = pTarget->m_sTotalAc + pTarget->m_sACAmount;
				// A unit's total AC shouldn't ever go below 0.
				if (temp_ac < 0)
					temp_ac = 0;
			}

			// 255 mp + 51 takı mp
			// 306 mp statı var
			// 306-86 = 220 smagicamountu var
			// novva 400 vuruyor ise 
			// 306 mp stat sayesinde 658 damage yaptı direkt

			sMagicAmount = pBot->m_sMagicAttackAmount + 100;
			if (isChaSkill)
				total_hit = (int)ceil(total_hit * bStat / 102.5f);

			total_hit = (total_hit * sMagicAmount) / 100;
			result = SUCCESS;
		}
	}

	if (result != FAIL)
	{
		// In case of SUCCESS.... 
		switch (attribute)
		{
		case FIRE_R:
			total_r = (pTarget->m_sFireR + pTarget->m_bAddFireR) * pTarget->m_bPctFireR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctFireR / 100;
			break;
		case COLD_R:
			total_r = (pTarget->m_sColdR + pTarget->m_bAddColdR) * pTarget->m_bPctColdR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctColdR / 100;
			break;
		case LIGHTNING_R:
			total_r = (pTarget->m_sLightningR + pTarget->m_bAddLightningR) * pTarget->m_bPctLightningR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctLightningR / 100;
			break;
		case MAGIC_R:
			total_r = (pTarget->m_sMagicR + pTarget->m_bAddMagicR) * pTarget->m_bPctMagicR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctMagicR / 100;
			break;
		case DISEASE_R:
			total_r = (pTarget->m_sDiseaseR + pTarget->m_bAddDiseaseR) * pTarget->m_bPctDiseaseR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctDiseaseR / 100;
			break;
		case POISON_R:
			total_r = (pTarget->m_sPoisonR + pTarget->m_bAddPoisonR) * pTarget->m_bPctPoisonR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctPoisonR / 100;
			break;
		default:
			total_r += pTarget->m_bResistanceBonus;
			break;
		}

		if (pSkillCaster->isPlayer())
		{
			CUser* pUser = TO_USER(pSkillCaster);
			if (pUser == nullptr) return 0;

			auto pRightHand = pUser->GetItemPrototype(RIGHTHAND);
			if (!pUser->isWeaponsDisabled() && !pRightHand.isnull() && pRightHand.isStaff() && pUser->GetItemPrototype(LEFTHAND).isnull())
			{
				righthand_damage = pRightHand.m_sDamage + pUser->m_bAddWeaponDamage;
				pUser->m_equippedItemBonusLock.lock();
				auto itr = pUser->m_equippedItemBonuses.find(RIGHTHAND);
				if (itr != pUser->m_equippedItemBonuses.end())
				{
					auto bonusItr = itr->second.find(attribute);
					if (bonusItr != itr->second.end())
						attribute_damage = short(bonusItr->second);
				}
				pUser->m_equippedItemBonusLock.unlock();
			}
			else righthand_damage = 0;

			int16 sAmount = 0;
			Guard lock(pUser->m_equippedItemBonusLock);
			foreach(itr, pUser->m_equippedItemBonuses)
			{
				uint8 bSlot = itr->first;
				auto bonusAtt = itr->second.find(attribute);
				if (bonusAtt == itr->second.end() || bSlot == RIGHTHAND || bSlot == LEFTHAND) continue;
				sAmount = int16(bonusAtt->second);
				if (attribute >= ITEM_TYPE_FIRE && attribute <= ITEM_TYPE_POISON)
				{
					attribute_damage += sAmount;// - sAmount * sTempResist / 200;
				}
			}

			auto pLeftHand = pUser->GetItemPrototype(LEFTHAND);
			if (pUser->isPortuKurian() && !pUser->isWeaponsDisabled() && !pLeftHand.isnull())
			{
				if (righthand_damage == 0)
					righthand_damage = pLeftHand.m_sDamage + pUser->m_bAddWeaponDamage;
				else
					righthand_damage += pLeftHand.m_sDamage + pUser->m_bAddWeaponDamage;
			}

			if (pUser->isPortuKurian() && !pUser->isWeaponsDisabled() && !pRightHand.isnull())
			{
				if (righthand_damage == 0)
					righthand_damage = pRightHand.m_sDamage + pUser->m_bAddWeaponDamage;
				else
					righthand_damage += pRightHand.m_sDamage + pUser->m_bAddWeaponDamage;
			}

			if (pUser->isWarrior() && !pUser->isWeaponsDisabled() && !pRightHand.isnull())
			{
				if (righthand_damage == 0)
					righthand_damage = pRightHand.m_sDamage + pUser->m_bAddWeaponDamage;
				else
					righthand_damage += pRightHand.m_sDamage + pUser->m_bAddWeaponDamage;
			}
		}

		if (pTarget->isPlayer())
		{
			if (TO_USER(pTarget)->isWarrior()) damage = (445 * total_hit / (total_r + 510));
			else if (TO_USER(pTarget)->isPriest()) damage = (400 * total_hit / (total_r + 510));
			else if (TO_USER(pTarget)->isRogue()) damage = (485 * total_hit / (total_r + 510));
			else if (TO_USER(pTarget)->isMage()) damage = (455 * total_hit / (total_r + 515));
			else damage = (485 * total_hit / (total_r + 510));

			if (!isChaSkill)
			{
				// Now handle class-specific AC/AP bonuses.
				temp_ac = temp_ac * (100 + TO_USER(pTarget)->m_byAcClassBonusAmount[TO_USER(pTarget)->GetBaseClass() - 1]) / 100;
				temp_ap = temp_ap * (100 + TO_USER(pSkillCaster)->m_byAPClassBonusAmount[TO_USER(pTarget)->GetBaseClass() - 1]) / 100;

				temp_hit_B = (int)((temp_ap * 20 / 10) / (temp_ac + 240));
				temp_hit = (int32)(temp_hit_B * (damage / 80));
				damage = temp_hit;
			}
		}
		else
		{
			damage = (555 * total_hit / (total_r + 515));
			if (!isChaSkill)
			{
				// Now handle class-specific AC/AP bonuses
				temp_hit_B = (int)((temp_ap * 20 / 10) / (temp_ac + 240));
				temp_hit = (int32)(temp_hit_B * (damage / 80.0f));
				damage = temp_hit;
			}
		}

		random = myrand(0, damage / 2);
		damage = (int32(random * 0.1f + (damage * 0.85f)) - sMagicAmount);

		if (pSkillCaster->isPlayer() && pTarget->isNPC() && TO_USER(pSkillCaster)->isWarrior())
			damage *= int32(0.50f);

		if (pSkillCaster->isNPC())
			damage -= ((3 * righthand_damage) + (3 * attribute_damage));
		else if (attribute != MAGIC_R)
			damage -= (int32)(((righthand_damage * 0.8f) + (righthand_damage * pSkillCaster->GetLevel()) / 60) + ((attribute_damage * 0.8f) + (attribute_damage * pSkillCaster->GetLevel()) / 60));

		if (pTarget->m_bMagicDamageReduction < 100) damage = damage * pTarget->m_bMagicDamageReduction / 100;
	}

	// Apply boost for skills matching weather type.
	// This isn't actually used officially, but I think it's neat...
	//GetWeatherDamage(damage, attribute);

	if (pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isWarrior() && righthand_damage == 0) damage /= 2;
	if (pTarget->isPlayer()) damage /= 3;
	if (pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isWarrior() && pTarget->isPlayer() && pTarget->m_sACAmount < 100)  damage += (int)((damage * 30 / 100));

	double baa = damage;
	if (pSkillCaster->isPlayer() && damage && TO_USER(pSkillCaster)->isMage())
	{
		baa *= g_pMain->pDamageSetting.magemagicdamage;
		baa *= TO_USER(pSkillCaster)->getplusdamage();
	}

	damage = (int32)baa;
	if (damage > MAX_DAMAGE) damage = MAX_DAMAGE;
	return damage;
}
#else
short MagicInstance::GetMagicDamage(Unit* pTarget, int total_hit, int attribute)
{
	int32 temp_hit = 0, damage = 0;
	short righthand_damage = 0, attribute_damage = 0, lastdamage = 0;
	int32 temp_ac = 0, temp_ap = 0, temp_hit_B = 0;
	int random = 0, total_r = 0;
	uint8 result;
	uint16 bStat = 1;
	bool isChaSkill = true;

	if (pTarget == nullptr
		|| pSkillCaster == nullptr
		|| pTarget->isDead()
		|| pSkillCaster->isDead())
		return 0;

	// Trigger item procs
	if (pTarget->GetID() < MAX_USER && pSkillCaster->GetID() < MAX_USER)
	{
		pSkillCaster->OnAttack(pTarget, AttackType::AttackTypeMagic);
		pTarget->OnDefend(pSkillCaster, AttackType::AttackTypeMagic);
	}

	if (pTarget->m_bBlockMagic)
		return 0;

	int16 sMagicAmount = 0;
	if (pSkillCaster->GetID() >= NPC_BAND)
		result = pSkillCaster->GetHitRate(pTarget->m_fTotalHitrate / pSkillCaster->m_fTotalEvasionrate + 2.0f);
	else
	{
		if (pSkillCaster->isPlayer())
		{
			CUser* pUser = TO_USER(pSkillCaster);

			if (pUser->isMage())
				bStat = pUser->GetStatWithItemBonus(StatType::STAT_CHA);
			else
			{
				isChaSkill = false;
				bStat = pUser->GetStatWithItemBonus(StatType::STAT_STR);
				temp_ap = pUser->m_sTotalHit * pUser->m_bAttackAmount / 5;
				temp_ap = temp_ap * pUser->m_bPlayerAttackAmount / 100;
				temp_ac = pTarget->m_sTotalAc + pTarget->m_sACAmount;
				// A unit's total AC shouldn't ever go below 0.
				if (temp_ac < 0)
					temp_ac = 0;
			}

			// 255 mp + 51 takı mp
			// 306 mp statı var
			// 306-86 = 220 smagicamountu var
			// novva 400 vuruyor ise 
			// 306 mp stat sayesinde 658 damage yaptı direkt

			if (pUser->isMage())
			{
				if (pTarget->GetID() < NPC_BAND && pTarget->GetID() > MAX_USER) // magelerin botlara olan damagesini düşürme 07.04.2024
					sMagicAmount = pUser->m_sMagicAttackAmount + 80;// mage damage düşürme 100 to 80
				else
					sMagicAmount = pUser->m_sMagicAttackAmount + 170; // mage damage arttırma 100 to 170
			}
			else
				sMagicAmount = pUser->m_sMagicAttackAmount + short(g_pMain->pDamageSetting.kurianzehir); // kurian vs. için zehir kısma

			if (isChaSkill)
				total_hit = (int)ceil(total_hit * bStat / 102.5f);

			total_hit = (total_hit * sMagicAmount) / 100;
			result = SUCCESS;
		}
		else
		{
			CBot* pBot = TO_BOT(pSkillCaster);

			if (pBot->isMage())
				bStat = pBot->GetStatWithItemBonus(StatType::STAT_CHA);
			else
			{
				isChaSkill = false;
				bStat = pBot->GetStatWithItemBonus(StatType::STAT_STR);
				temp_ap = pBot->m_sTotalHit * pBot->m_bAttackAmount / 5;
				temp_ap = temp_ap * 10 / 100;
				temp_ac = pTarget->m_sTotalAc + pTarget->m_sACAmount;
				// A unit's total AC shouldn't ever go below 0.
				if (temp_ac < 0)
					temp_ac = 0;
			}

			// 255 mp + 51 takı mp
			// 306 mp statı var
			// 306-86 = 220 smagicamountu var
			// novva 400 vuruyor ise 
			// 306 mp stat sayesinde 658 damage yaptı direkt

			if (pBot->isMage())
			{
				if (pTarget->GetID() < NPC_BAND && pTarget->GetID() > MAX_USER) // magelerin botlara olan damagesini düşürme 07.04.2024
					sMagicAmount = pBot->m_sMagicAttackAmount + 80;// mage damage düşürme 100 to 80
				else
					sMagicAmount = pBot->m_sMagicAttackAmount + 170; // mage damage arttırma 100 to 170
			}
			else
				sMagicAmount = pBot->m_sMagicAttackAmount + short(g_pMain->pDamageSetting.kurianzehir); // kurian vs. için zehir kısma

			if (isChaSkill)
				total_hit = (int)ceil(total_hit * bStat / 102.5f);

			total_hit = (total_hit * sMagicAmount) / 100;
			result = SUCCESS;
		}
	}

	if (result != FAIL)
	{
		// In case of SUCCESS.... 
		switch ((ResistanceTypes)attribute)
		{
		case ResistanceTypes::FIRE_R:
			total_r = (pTarget->m_sFireR + pTarget->m_bAddFireR) * pTarget->m_bPctFireR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctFireR / 100;
			break;
		case ResistanceTypes::COLD_R:
			total_r = (pTarget->m_sColdR + pTarget->m_bAddColdR) * pTarget->m_bPctColdR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctColdR / 100;
			break;
		case ResistanceTypes::LIGHTNING_R:
			total_r = (pTarget->m_sLightningR + pTarget->m_bAddLightningR) * pTarget->m_bPctLightningR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctLightningR / 100;
			break;
		case ResistanceTypes::MAGIC_R:
			total_r = (pTarget->m_sMagicR + pTarget->m_bAddMagicR) * pTarget->m_bPctMagicR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctMagicR / 100;
			break;
		case ResistanceTypes::DISEASE_R:
			total_r = (pTarget->m_sDiseaseR + pTarget->m_bAddDiseaseR) * pTarget->m_bPctDiseaseR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctDiseaseR / 100;
			break;
		case ResistanceTypes::POISON_R:
			total_r = (pTarget->m_sPoisonR + pTarget->m_bAddPoisonR) * pTarget->m_bPctPoisonR / 100;
			total_r += pTarget->m_bResistanceBonus * pTarget->m_bPctPoisonR / 100;
			break;
		default:
			total_r += pTarget->m_bResistanceBonus;
			break;
		}

		if (/*pSkillCaster->isPlayer()*/pSkillCaster->GetID() < MAX_USER)
		{
			CUser* pUser = TO_USER(pSkillCaster);

			// double the staff's damage when using a skill of the same attribute as the staff
			_ITEM_TABLE pRightHand = pUser->GetItemPrototype(pUser->GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_RIGHTHAND : RIGHTHAND);
			if (!pUser->isWeaponsDisabled()
				&& !pRightHand.isnull() && pRightHand.isStaff()
				&& pUser->GetItemPrototype(LEFTHAND).isnull())
			{
				//Guard lock(pSkillCaster->m_equippedItemBonusLock);
				//Guard lock(pSkillCaster->m_equippedItemBonusLock);
				pUser->m_equippedItemBonusLock.lock();
				righthand_damage = pRightHand.m_sDamage + pUser->m_bAddWeaponDamage;// viper staff damagesi
				auto itr = pUser->m_equippedItemBonuses.find(pUser->GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_RIGHTHAND : RIGHTHAND);
				if (itr != pUser->m_equippedItemBonuses.end())
				{
					auto bonusItr = itr->second.find(attribute);
					if (bonusItr != itr->second.end())
						attribute_damage = short(bonusItr->second);// fire damage vs.
				}
				pUser->m_equippedItemBonusLock.unlock();
			}
			else
			{
				righthand_damage = 0;
			}

			//Guard lock21(pSkillCaster->m_equippedItemBonusLock);
			//Guard lock(pSkillCaster->m_equippedItemBonusLock);
			pUser->m_equippedItemBonusLock.lock();
			int16 sAmount = 0;
			uint8 b_Slot = pUser->GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_RIGHTHAND : RIGHTHAND;
			uint8 c_Slot = pUser->GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_LEFTHAND : LEFTHAND;

			foreach(itr, pUser->m_equippedItemBonuses)
			{
				uint8 bSlot = itr->first;
				auto bonusAtt = itr->second.find(attribute);
				if (bonusAtt == itr->second.end()
					|| bSlot == b_Slot
					|| bSlot == c_Slot)
					continue;

				sAmount = int16(bonusAtt->second);

				if (attribute >= ITEM_TYPE_FIRE
					&& attribute <= ITEM_TYPE_POISON)
				{
					// add attribute damage amount to right-hand damage instead of attribute
					// so it doesn't bother taking into account caster level (as it would with the staff attributes).
					attribute_damage += sAmount;// - sAmount * sTempResist / 200;
				}
			}
			pUser->m_equippedItemBonusLock.unlock();
			_ITEM_TABLE pLeftHand = pUser->GetItemPrototype(pUser->GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_LEFTHAND : LEFTHAND);
			if (pUser->isPortuKurian() && !pUser->isWeaponsDisabled() && !pLeftHand.isnull())
			{
				if (righthand_damage == 0)
					righthand_damage = pLeftHand.m_sDamage + pUser->m_bAddWeaponDamage;
				else
					righthand_damage += pLeftHand.m_sDamage + pUser->m_bAddWeaponDamage;
			}

			if (pUser->isPortuKurian() && !pUser->isWeaponsDisabled() && !pRightHand.isnull())
			{
				if (righthand_damage == 0)
					righthand_damage = pRightHand.m_sDamage + pUser->m_bAddWeaponDamage;
				else
					righthand_damage += pRightHand.m_sDamage + pUser->m_bAddWeaponDamage;
			}
		}

		damage = (485 * total_hit / (total_r + 510));

		if (pTarget->GetID() < MAX_USER)
		{
			if (g_pMain->m_bisDamage == false)
			{
				_CLASS_DAMAGE* pCoefficientDamage = g_pMain->m_CoefficientDamageArray.GetData(TO_USER(pSkillCaster)->GetClass());
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

			if (!isChaSkill)
			{
				// Now handle class-specific AC/AP bonuses.
				temp_ac = temp_ac * (100 + TO_USER(pTarget)->m_byAcClassBonusAmount[TO_USER(pTarget)->GetBaseClass() - 1]) / 100;
				temp_ap = temp_ap * (100 + TO_USER(pSkillCaster)->m_byAPClassBonusAmount[TO_USER(pTarget)->GetBaseClass() - 1]) / 100;

				temp_hit_B = (int)((temp_ap * 20 / 10) / (temp_ac + 240));
				temp_hit = (int32)(temp_hit_B * (damage / 80));
				damage = temp_hit;
			}
		}
		else
		{
			if (!isChaSkill)
			{
				// Now handle class-specific AC/AP bonuses
				temp_hit_B = (int)((temp_ap * 20 / 10) / (temp_ac + 240));
				temp_hit = (int32)(temp_hit_B * (damage / 80.0f));
				damage = temp_hit;
			}
		}
		random = myrand(0, damage / 2);
		damage = (int32(random * 0.1f + (damage * 0.85f)) - sMagicAmount);

		if (pSkillCaster->GetID() >= NPC_BAND)
			damage -= ((3 * righthand_damage) + (3 * attribute_damage));
		else if (pSkillCaster->GetID() < MAX_USER)
		{
			if (TO_USER(pSkillCaster)->isPortuKurian() && pSkillCaster->isDevil())
			{
				CUser* pUser = TO_USER(pSkillCaster);
				// double the staff's damage when using a skill of the same attribute as the staff
				_ITEM_TABLE pRightHand = pUser->GetItemPrototype(pUser->GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_RIGHTHAND : RIGHTHAND);

				if (!pRightHand.isnull() && (pRightHand.isSword() || pRightHand.isAxe() || pRightHand.isMace() || pRightHand.isSpear()) && !pUser->GetItemPrototype(pUser->GetZoneID() == ZONE_KNIGHT_ROYALE ? KNIGHT_ROYAL_LEFTHAND : LEFTHAND).isnull())	// Only kurian			
					damage -= (int32)(((righthand_damage * 0.8f) + (righthand_damage * pSkillCaster->GetLevel()) / 60) + ((bStat * 0.8f) + (bStat * pSkillCaster->GetLevel()) / 5.5));
			}
			else if ((ResistanceTypes)attribute != ResistanceTypes::MAGIC_R)	// Only if the staff has an attribute.
				damage -= (int32)(((righthand_damage * 0.8f) + (righthand_damage * pSkillCaster->GetLevel()) / 60) + ((attribute_damage * 0.8f) + (attribute_damage * pSkillCaster->GetLevel()) / 60));
		}

		if (pTarget->m_bMagicDamageReduction < 100)
			damage = damage * pTarget->m_bMagicDamageReduction / 100;
	}

	// Apply boost for skills matching weather type.
	// This isn't actually used officially, but I think it's neat...
	//GetWeatherDamage(damage, attribute);

	if (pSkillCaster->GetID() < MAX_USER)
	{
		if (TO_USER(pSkillCaster)->isWarrior()
			&& righthand_damage == 0)
			damage /= 2;
	}


	if (pTarget->GetID() < MAX_USER)
		damage /= 3;

	if (pSkillCaster->GetID() < MAX_USER)
	{
		if (pSkillCaster->isPlayer() && TO_USER(pSkillCaster)->isWarrior())
		{
			if (pTarget->isPlayer() && pTarget->m_sACAmount < 100)
				damage += (int)((damage * 30 / 100));
		}


		//if (g_pMain->pServerSetting.ActiveUniqueItemBonusDamage > 0)
		//{
		//	double baa = damage;
		//	if (pSkillCaster->isPlayer() && damage && TO_USER(pSkillCaster)->isMage())
		//	{
		//		baa *= g_pMain->pDamageSetting.magemagicdamage;
		//		baa *= TO_USER(pSkillCaster)->getplusdamage();
		//	}
		//	//printf("Magic Damage 2 damage: %d \n", damage);
		//	damage = (int32)baa;
		//}

	}
	//printf("Magic Damage 1 damage: %d \n", damage);


	// Implement damage cap.
	if (damage > MAX_DAMAGE)
		damage = MAX_DAMAGE;
	else if (damage < -MAX_DAMAGE)
		damage = -MAX_DAMAGE;

	//printf("Magic Damage 3 damage: %d \n", damage);
	lastdamage = damage;

	// aakinci ekleme 02.12.2022
	if (pTarget->GetID() >= NPC_BAND)
	{
		if (TO_NPC(pTarget)->isPet())
			return lastdamage;

		// Santa R damage duzenlemesi
		if (TO_NPC(pTarget)->GetType() == NPC_SANTA
			|| TO_NPC(pTarget)->GetType() == NPC_REFUGEE
			|| TO_NPC(pTarget)->GetType() == NPC_FOSIL
			|| TO_NPC(pTarget)->GetType() == NPC_TREE
			|| TO_NPC(pTarget)->GetType() == NPC_BORDER_MONUMENT)
			lastdamage = 0;


		CUser* pUser = g_pMain->GetUserPtr(pSkillCaster->GetID());

		//Tarih : 14.05.2020 ##START##
		if (pUser) {
			if (TO_NPC(pTarget)->GetType() == NPC_PVP_MONUMENT)
			{
				if (pUser->GetZoneID() == ZONE_BATTLE5)
				{
					if ((!g_pMain->isWarOpen()) || (g_pMain->isWarOpen() && g_pMain->m_sElmoradDead != g_pMain->m_sKarusDead))
						lastdamage = 0;

					if ((TO_NPC(pTarget)->GetNation() == (uint8)Nation::KARUS && pUser->GetNation() == (uint8)Nation::KARUS)
						|| (TO_NPC(pTarget)->GetNation() == (uint8)Nation::ELMORAD && pUser->GetNation() == (uint8)Nation::ELMORAD))
						lastdamage = 0;
				}
			}
		}
	}
	//printf("1-lastdamage %d\n",lastdamage);
	uint16	BotDamageControl = lastdamage;;
	// aakinci ekleme 02.12.2022 end
	if (pSkillCaster->isBot() && TO_BOT(pSkillCaster)->isMage() && (pTarget->isBot())
		|| pSkillCaster->isBot() && TO_BOT(pSkillCaster)->isMage() && (pTarget->isPlayer()))
	{
		if (BotDamageControl > 1000)
			lastdamage = -1000;
	}
	//printf("2-lastdamage %d\n", lastdamage);
	return lastdamage;
}
#endif
float CUser::getplusdamage()
{
	auto pLeftHand = GetItemPrototype(LEFTHAND);
	if (!pLeftHand.isnull())
	{
		if (pLeftHand.m_ItemType == 4 || pLeftHand.m_ItemType == 12) return g_pMain->pDamageSetting.uniqueitem;
		else if (pLeftHand.m_ItemType == 2) return g_pMain->pDamageSetting.rareitem;
		else if (pLeftHand.m_ItemType == 1) return g_pMain->pDamageSetting.magicitem;
		else if (pLeftHand.m_ItemType == 5 && (pLeftHand.ItemClass == 0 || pLeftHand.ItemClass == 1)) return g_pMain->pDamageSetting.lowclassitem;
		else if (pLeftHand.m_ItemType == 5 && (pLeftHand.ItemClass == 2 || pLeftHand.ItemClass == 7 || pLeftHand.ItemClass == 33)) return g_pMain->pDamageSetting.middleclassitem;
		else if (pLeftHand.m_ItemType == 5 && (pLeftHand.ItemClass == 3 || pLeftHand.ItemClass == 4 || pLeftHand.ItemClass == 8 || pLeftHand.ItemClass == 34)) return g_pMain->pDamageSetting.highclassitem;
		else if (pLeftHand.m_ItemType == 11) return g_pMain->pDamageSetting.highclassitem;
	}
	else
	{
		auto pRightHand = GetItemPrototype(RIGHTHAND);
		if (!pRightHand.isnull())
		{
			if (pRightHand.m_ItemType == 4 || pRightHand.m_ItemType == 12) return g_pMain->pDamageSetting.uniqueitem;
			else if (pRightHand.m_ItemType == 2) return g_pMain->pDamageSetting.rareitem;
			else if (pRightHand.m_ItemType == 1) return g_pMain->pDamageSetting.magicitem;
			else if (pRightHand.m_ItemType == 5 && (pRightHand.ItemClass == 0 || pRightHand.ItemClass == 1)) return g_pMain->pDamageSetting.lowclassitem;
			else if (pRightHand.m_ItemType == 5 && (pRightHand.ItemClass == 2 || pRightHand.ItemClass == 7 || pRightHand.ItemClass == 33)) return g_pMain->pDamageSetting.middleclassitem;
			else if (pRightHand.m_ItemType == 5 && (pRightHand.ItemClass == 3 || pRightHand.ItemClass == 4 || pRightHand.ItemClass == 8 || pRightHand.ItemClass == 34)) return g_pMain->pDamageSetting.highclassitem;
			else if (pRightHand.m_ItemType == 11) return g_pMain->pDamageSetting.highclassitem;
		}
	}
	return 1.0f;
}

int32 MagicInstance::GetWeatherDamage(int32 damage, int attribute)
{
	// Give a 10% damage output boost based on weather (and skill's elemental attribute)
	if ((g_pMain->m_byWeather == WEATHER_FINE && attribute == (uint8)AttributeType::AttributeFire)
		|| (g_pMain->m_byWeather == WEATHER_RAIN && attribute == (uint8)AttributeType::AttributeLightning)
		|| (g_pMain->m_byWeather == WEATHER_SNOW && attribute == (uint8)AttributeType::AttributeIce))
		damage = (damage * 110) / 100;

	return damage;
}

void MagicInstance::Type6Cancel(bool bForceRemoval)
{
	// NPCs cannot transform.
	if (pSkillCaster == nullptr
		|| !pSkillCaster->isPlayer()
		|| (!bForceRemoval && !TO_USER(pSkillCaster)->isTransformed()))
		return;

	CUser* pUser = TO_USER(pSkillCaster);

	_MAGIC_TYPE6* pType = g_pMain->m_Magictype6Array.GetData(nSkillID);
	if (pType == nullptr)
		return;

	if (pUser->isSiegeTransformation())
		pUser->m_bRegeneType = REGENE_NORMAL;

	Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_CANCEL_TRANSFORMATION));
	// TODO: Reset stat changes, recalculate stats.
	pUser->m_transformationType = Unit::TransformationType::TransformationNone;
	pUser->m_sTransformHpchange = false;
	pUser->m_sTransformMpchange = false;
	pUser->Send(&result);
	pUser->SetUserAbility();
	pUser->RemoveSavedMagic(pUser->m_bAbnormalType);
	pUser->StateChangeServerDirect(3, ABNORMAL_NORMAL);
	if (pUser->m_transformSkillUseid == TransformationSkillMovingTower) pUser->m_transformSkillUseid = TransformationSkillUseNone;
}

void MagicInstance::Type9Cancel(bool bRemoveFromMap)
{
	if (pSkillCaster == nullptr || !pSkillCaster->isPlayer())
		return;

	_MAGIC_TYPE9* pType = g_pMain->m_Magictype9Array.GetData(nSkillID);
	if (pType == nullptr)
		return;

	uint8 bResponse = 0;
	CUser* pCaster = TO_USER(pSkillCaster);
	if (pCaster == nullptr)
		return;

	TO_USER(pSkillCaster)->m_buff9lock.lock();
	Type9BuffMap& buffMap = pCaster->m_type9BuffMap;

	if (buffMap.find(pType->bStateChange) == buffMap.end())
	{
		TO_USER(pSkillCaster)->m_buff9lock.unlock();
		return;
	}

	if (bRemoveFromMap)
		buffMap.erase(pType->bStateChange);

	TO_USER(pSkillCaster)->m_buff9lock.unlock();

	// Stealths
	if (pType->bStateChange <= 2 || (pType->bStateChange >= 5 && pType->bStateChange < 7))
	{
		pCaster->StateChangeServerDirect(7, (uint8)InvisibilityType::INVIS_NONE);
		bResponse = 91;
	}
	// Lupine etc.
	else if (pType->bStateChange >= 3 && pType->bStateChange <= 4)
	{
		pCaster->InitializeStealth();
		bResponse = 92;
	}
	// Guardian pet related
	else if (pType->bStateChange >= 7 && pType->bStateChange <= 8)
	{
		if (pType->bStateChange == 8)
		{
			if (pCaster->m_PettingOn != nullptr)
				pCaster->PetOnDeath();
		}
		else
		{
			g_pMain->KillNpc(pCaster->GetSocketID(), pCaster->GetZoneID(), pCaster);
		}
		bResponse = 93;
	}
	else if (pType->bStateChange == 9)
	{
		g_pMain->KillNpc(pCaster->GetSocketID(), pCaster->GetZoneID());
		bResponse = 9;
	}

	Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_DURATION_EXPIRED));
	result << bResponse;
	pCaster->Send(&result);
}

void MagicInstance::Type4Cancel()
{
	if (pSkill.isnull() || pSkillCaster == nullptr)
		return;

	if (pSkillTarget != pSkillCaster)
		return;

	_MAGIC_TYPE4* pType = g_pMain->m_Magictype4Array.GetData(nSkillID);
	if (pType == nullptr || pType->isDebuff())
		return;

	if (nSkillID > 500000 && TO_USER(pSkillCaster)->isLockableScroll(pType->bBuffType) && pSkillCaster->hasDebuff(pType->bBuffType))
		return;

	if (!CMagicProcess::RemoveType4Buff(pType->bBuffType, TO_USER(pSkillCaster)))
		return;

	TO_USER(pSkillCaster)->RemoveSavedMagic(nSkillID);
	
	if (nSkillID == 495146)
	{
		CBot* pPriest = nullptr;
		pPriest = g_pMain->m_MapBotList.GetData(TO_USER(pSkillCaster)->m_bUserPriestBotID);
		if (pPriest)
			pPriest->UserInOut(INOUT_OUT);

		TO_USER(pSkillCaster)->m_bUserPriestBotID = -1;
		TO_USER(pSkillCaster)->hasPriestBot = false;
	}
}

void MagicInstance::Type3Cancel()
{
	if (pSkill.isnull() || pSkillCaster == nullptr)
		return;

	if (pSkillTarget != pSkillCaster)
		return;

	// Should this take only the specified skill? I'm thinking so.
	_MAGIC_TYPE3* pType = g_pMain->m_Magictype3Array.GetData(nSkillID);
	if (pType == nullptr)
		return;

	for (int i = 0; i < MAX_TYPE3_REPEAT; i++)
	{
		Unit::MagicType3* pEffect = &pSkillCaster->m_durationalSkills[i];

		if (pEffect == nullptr)
			continue;

		if (!pEffect->m_byUsed || pEffect->m_sHPAmount <= 0)
			continue;

		pEffect->Reset();
		break;
	}

	if (pSkillCaster->isPlayer())
	{
		Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_DURATION_EXPIRED));
		result << uint8(100); // remove the healing-over-time skill.
		TO_USER(pSkillCaster)->Send(&result);
	}

	int buff_test = 0;
	for (int j = 0; j < MAX_TYPE3_REPEAT; j++)
	{
		if (pSkillCaster->m_durationalSkills[j].m_byUsed)
		{
			buff_test++;
			break;
		}
	}

	if (buff_test == 0)
		pSkillCaster->m_bType3Flag = false;

	if (pSkillCaster->isPlayer() && !pSkillCaster->m_bType3Flag)
		TO_USER(pSkillCaster)->SendUserStatusUpdate(UserStatus::USER_STATUS_DOT, UserStatusBehaviour::USER_STATUS_CURE);
}

void MagicInstance::Type4Extend()
{
	if (pSkill.isnull() || pSkillCaster == nullptr)
		return;

	if (!pSkillCaster->isPlayer() || nSkillID >= 500000)
		return;

	_MAGIC_TYPE4* pType = g_pMain->m_Magictype4Array.GetData(nSkillID);
	if (pType == nullptr || pType->isDebuff())
		return;

	pSkillTarget->m_buffLock.lock();
	Type4BuffMap::iterator itr = pSkillTarget->m_buffMap.find(pType->bBuffType);

	if (itr == pSkillCaster->m_buffMap.end() || itr->second.m_bDurationExtended)
	{
		pSkillTarget->m_buffLock.unlock();
		return;
	}

	// Require the "Duration Item" for buff duration extension.
	// The things we must do to future-proof things...
	bool bItemFound = false;
	for (int i = SLOT_MAX; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = nullptr;
		_ITEM_TABLE pTable = TO_USER(pSkillCaster)->GetItemPrototype(i, pItem);

		if (pTable.isnull())
			continue;

		if (pTable.m_bKind != 255 || pTable.m_iEffect1 == 0)
			continue;

		auto pEffect = g_pMain->GetMagicPtr(pTable.m_iEffect1);

		if (pEffect.isnull())
			continue;

		if (pEffect.bMoral != MORAL_EXTEND_DURATION || !TO_USER(pSkillCaster)->RobItem(i, pTable))
			continue;

		bItemFound = true;
		break;
	}

	// No "Duration Item" was found.
	if (!bItemFound)
	{
		pSkillTarget->m_buffLock.unlock();
		return;
	}

	// Extend the duration of a buff.
	itr->second.m_tEndTime += pType->sDuration;
	itr->second.m_bDurationExtended = true;
	//pSkillCaster->m_buffLock.unlock();
	pSkillTarget->m_buffLock.unlock();//23.04.2024 eklemesi
	Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_TYPE4_EXTEND));
	result << uint32(nSkillID);
	TO_USER(pSkillTarget)->Send(&result);
}

void MagicInstance::ReflectDamage(int32 damage, Unit* pTarget)
{
	if (pSkillCaster == nullptr || pTarget == nullptr)
		return;

	if (!pSkillCaster->isPlayer())
		return;

	if (damage < 0)
		damage *= -1;

	int16 total_resistance_caster = 0;
	int32 reflect_damage = 0, sSkillID = 0;

	switch (pTarget->m_bReflectArmorType)
	{
	case FIRE_DAMAGE:
		pTarget->GetNation() == (uint8)Nation::KARUS ? sSkillID = 190573 : sSkillID = 290573;
		break;

	case ICE_DAMAGE:
		pTarget->GetNation() == (uint8)Nation::KARUS ? sSkillID = 190673 : sSkillID = 290673;
		break;

	case LIGHTNING_DAMAGE:
		pTarget->GetNation() == (uint8)Nation::KARUS ? sSkillID = 190773 : sSkillID = 290773;
		break;
	}

	if (!sSkillID)
		return;

	MagicInstance instance;

	instance.bIsRunProc = true;
	instance.sCasterID = pTarget->GetID();
	instance.sTargetID = pSkillCaster->GetID();
	instance.nSkillID = sSkillID;
	instance.sSkillCasterZoneID = pTarget->GetZoneID();
	// For AOE skills such as "Splash", the AOE should be focus on the target.
	instance.sData[0] = (uint16)pTarget->GetX();
	instance.sData[2] = (uint16)pTarget->GetZ();
	instance.Run();

	CMagicProcess::RemoveType4Buff(BUFF_TYPE_MAGE_ARMOR, pTarget, true);
}

void MagicInstance::ConsumeItem()
{
	if (pSkillCaster == nullptr)
		return;

	if (nConsumeItem != 0 && pSkillCaster->isPlayer())
	{
		CUser * pUser = TO_USER(pSkillCaster);
		const bool bHasWarriorInfiniteStone = (nConsumeItem == 379059000 && pUser->isWarrior() && pUser->CheckExistItem(479059000));
		const bool bHasRogueInfiniteStone = (nConsumeItem == 379060000 && pUser->isRogue() && pUser->CheckExistItem(479060000));
		const bool bHasMageInfiniteStone = (nConsumeItem == 379061000 && pUser->isMage() && pUser->CheckExistItem(479061000));
		const bool bHasPriestInfiniteStone = (nConsumeItem == 379062000 && pUser->isPriest() && pUser->CheckExistItem(479062000));
		const bool bSkipConsumeForInfiniteStone = (bHasWarriorInfiniteStone || bHasRogueInfiniteStone || bHasMageInfiniteStone || bHasPriestInfiniteStone);

		if (!bSkipConsumeForInfiniteStone)
		{
			if (nConsumeItem == 370001000 ||
				nConsumeItem == 370002000 ||
				nConsumeItem == 370003000 ||
				nConsumeItem == 379069000 ||
				nConsumeItem == 379070000 ||
				nConsumeItem == 379063000 ||
				nConsumeItem == 379064000 ||
				nConsumeItem == 379065000 ||
				nConsumeItem == 379066000)
				pUser->RobItem(0);
			else
				pUser->RobItem(nConsumeItem);
		}
	}

	if (bInstantCast)
		CMagicProcess::RemoveType4Buff(BUFF_TYPE_INSTANT_MAGIC, pSkillCaster);
}

bool MagicInstance::isStaffSkill(bool isExtended)
{
	uint32 iSkillID = nSkillID;

	if (isExtended)
		iSkillID -= SKILLPACKET;

	if (iSkillID == 109742 //Lr Staff
		|| iSkillID == 110742
		|| iSkillID == 209742
		|| iSkillID == 210742
		|| iSkillID == 110772
		|| iSkillID == 210772
		|| iSkillID == 109542 //Fr Staff
		|| iSkillID == 110542
		|| iSkillID == 209542
		|| iSkillID == 210542
		|| iSkillID == 110572
		|| iSkillID == 210572
		|| iSkillID == 109642 //Ice Staff
		|| iSkillID == 110642
		|| iSkillID == 209642
		|| iSkillID == 210642
		|| iSkillID == 110672
		|| iSkillID == 210672
		//new mastersiz START 43 ve 56 mage skilleri
		|| iSkillID == 109556
		|| iSkillID == 209556
		|| iSkillID == 109543
		|| iSkillID == 209543
		|| iSkillID == 109656
		|| iSkillID == 209656
		|| iSkillID == 109643
		|| iSkillID == 209643
		|| iSkillID == 109756
		|| iSkillID == 209756
		|| iSkillID == 109743
		|| iSkillID == 209743
		//new mastersiz END 43 ve 56 mage skilleri
		//new mastersiz START 43 ve 56 mage skilleri
		|| iSkillID == 110556
		|| iSkillID == 210556
		|| iSkillID == 110543
		|| iSkillID == 210543
		|| iSkillID == 110656
		|| iSkillID == 210656
		|| iSkillID == 110643
		|| iSkillID == 210643
		|| iSkillID == 110756
		|| iSkillID == 210756
		|| iSkillID == 110743
		|| iSkillID == 210743)
		//new mastersiz END 43 ve 56 mage skilleri
		return true;

	return false;
}

bool MagicInstance::isArcherMultipleSkill()
{
	uint32 iSkillID = nSkillID;

	if (iSkillID == 107555 // Arrow Shower
		|| iSkillID == 108555
		|| iSkillID == 207555
		|| iSkillID == 208555
		|| iSkillID == 107515 //multiple shot
		|| iSkillID == 108515
		|| iSkillID == 207515
		|| iSkillID == 208515)
		return true;

	return false;
}


bool MagicInstance::isRushSkill(bool isExtended)
{
	uint32 iSkillID = nSkillID;

	if (isExtended)
		iSkillID -= SKILLPACKET;

	if (iSkillID == 114509 || iSkillID == 115509 || iSkillID == 214509 || iSkillID == 215509)
		return true;

	return false;
}

bool MagicInstance::isMageArmorSkill(bool isExtended)
{
	uint32 iSkillID = nSkillID;

	if (isExtended)
		iSkillID -= SKILLPACKET;

	if (iSkillID == 190573
		|| iSkillID == 290573
		|| iSkillID == 190673
		|| iSkillID == 290673
		|| iSkillID == 190773
		|| iSkillID == 290773)
		return true;

	return false;
}

bool MagicInstance::isExtendedArcherSkill(bool isExtended)
{
	uint32 iSkillID = nSkillID;

	if (isExtended)
		iSkillID -= SKILLPACKET;

	if (iSkillID == 108566
		|| iSkillID == 208566)
		return true;

	return false;
}

bool MagicInstance::isUnderTheCastleDisableSkill()
{
	uint32 iSkillID = nSkillID;

	if (iSkillID == 107650
		|| iSkillID == 108650
		|| iSkillID == 207650
		|| iSkillID == 208650 // Vampiric touch
		|| iSkillID == 107610
		|| iSkillID == 108610
		|| iSkillID == 207610
		|| iSkillID == 208610 //Blood drain
		|| iSkillID == 109554
		|| iSkillID == 110554
		|| iSkillID == 209554
		|| iSkillID == 210554 // Fire Thorn
		|| iSkillID == 109754
		|| iSkillID == 110754
		|| iSkillID == 209754
		|| iSkillID == 210754 // Static Thorn
		|| iSkillID == 111745
		|| iSkillID == 112745
		|| iSkillID == 211745
		|| iSkillID == 212745 //Parasite
		|| iSkillID == 112771
		|| iSkillID == 212771) // Super Parasite
		return true;

	return false;
}

bool MagicInstance::isKrowazWeaponSkills()
{
	if (nSkillID == 491058	// Cannot use against the ones to be summoned.                                                                                                                                                                                                               
		|| nSkillID == 491059	// Reduction                                                                                                                                                                                                                                                 
		|| nSkillID == 491060	// Full Skill Gear                                                                                                                                                                                                                                           
		|| nSkillID == 491061	// Sweet Kiss                                                                                                                                                                                                                                                
		|| nSkillID == 491062	// No Potion                                                                                                                                                                                                                                                 
		|| nSkillID == 491063	// Splash                                                                                                                                                                                                                                                    
		|| nSkillID == 491064	// Strike Wrist                                                                                                                                                                                                                                              
		|| nSkillID == 491065	// Kaul Transformation                                                                                                                                                                                                                                       
		|| nSkillID == 491066	// Deadmans Call                                                                                                                                                                                                                                             
		|| nSkillID == 491067	// Unsight                                                                                                                                                                                                                                                   
		|| nSkillID == 491068	// Adamant                                                                                                                                                                                                                                                   
		|| nSkillID == 491069)	// Divine Protection   
		return true;

	return false;
}

bool MagicInstance::isDarkKnightWeaponSkill()
{
	if (nSkillID == 492029
		|| nSkillID == 492028
		|| nSkillID == 492027
		|| nSkillID == 492030
		|| nSkillID == 492031
		|| nSkillID == 492032) return true;
	return false;
}

bool MagicInstance::isDrainSkills(uint32 iskillid /*= 0*/)
{
	if (iskillid)
	{
		if (iskillid == 107650 ||
			iskillid == 108650 ||
			iskillid == 207650 ||
			iskillid == 208650 ||
			iskillid == 107610 ||
			iskillid == 108610 ||
			iskillid == 207610 ||
			iskillid == 208610)
			return true;
	}
	else
	{
		if (nSkillID == 107650 ||
			nSkillID == 108650 ||
			nSkillID == 207650 ||
			nSkillID == 208650 ||
			nSkillID == 107610 ||
			nSkillID == 108610 ||
			nSkillID == 207610 ||
			nSkillID == 208610)
			return true;
	}
	return false;
}

bool MagicInstance::isStompSkills()
{
	if (nSkillID == 105725 ||
		nSkillID == 105735 ||
		nSkillID == 106725 ||
		nSkillID == 106735 ||
		nSkillID == 205725 ||
		nSkillID == 205735 ||
		nSkillID == 206725 ||
		nSkillID == 206735 ||
		nSkillID == 105760 ||
		nSkillID == 106760 ||
		nSkillID == 205760 ||
		nSkillID == 206760 ||
		nSkillID == 106775 ||
		nSkillID == 206775)
		return true;

	return false;
}

bool MagicInstance::isKurianStuns2(bool isExtended)
{
	uint32 iSkillID = nSkillID;

	if (isExtended)
		iSkillID -= SKILLPACKET;

	if (iSkillID == 115810 || iSkillID == 215810)
		return true;

	return false;
}
// skill ve kurian için şuanda sorun kalmamış olması lazım, siz gene de test edin, int priesti vs. sorun görürseniz bakarız. Tamamdır
// iyi günler. sizede
