#include "stdafx.h"

bool CUser::CheckDevaAttack(bool isnpc, uint16 protoid) {

	if (!g_pMain->isJuraidMountainActive() || !isEventUser() || !isInValidRoom(0))
		return false;

	auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(GetEventRoom());
	if (!pRoomInfo)
		return false;

	if (isnpc && protoid == DEVA_BIRD_SSID) {
		bool gate = true;
		if (GetNation() == (uint8)Nation::KARUS) {
			for (int i = 0; i < 3; i++) {
				if (!pRoomInfo->m_sKarusBridges[i]) { gate = false; break; }
			}
		}
		else if (GetNation() == (uint8)Nation::ELMORAD) {
			for (int i = 0; i < 3; i++) {
				if (!pRoomInfo->m_sElmoBridges[i]) { gate = false; break; }
			}
		}

		if (!gate)
			return false;
	}

	return true;
}

void CUser::Attack(Packet & pkt)
{
	if (!isInGame())
		return;

	int16 sid = -1, tid = -1, damage, delaytime, distance;
	uint8 bType, bResult = 0, unknown;
	Unit * pTarget = nullptr;

	pkt >> bType >> bResult >> tid >> delaytime >> distance >> unknown;

	if (m_bResHpType == USER_SITDOWN || isIncapacitated() || isInEnemySafetyArea())
		return;

	if (isInSpecialEventZone()
		&& !g_pMain->pSpecialEvent.opened 
		&& !g_pMain->pCindWar.isON())
		return;

	if (!g_pMain->pSpecialEvent.opened 
		&& g_pMain->isCindirellaZone(GetZoneID()) 
		&& g_pMain->pCindWar.isON()
		&& !g_pMain->pCindWar.isStarted())
		return;

	RemoveStealth();

	// If you're holding a weapon, do a client-based (ugh, do not trust!) delay check.
	_ITEM_TABLE pRightTable = GetItemPrototype(RIGHTHAND), pLeftTable = GetItemPrototype(LEFTHAND);

	bool nocheck = false;
	if ((isGM() || isGMUser()) && (!pRightTable.isnull() && pRightTable.Getnum() == 389158000) || (!pLeftTable.isnull() && pLeftTable.Getnum() == 389158000)) nocheck = true;

	if ((!pRightTable.isnull() && pRightTable.isBow()) || (!pLeftTable.isnull() && pLeftTable.isBow())) return;

	if (!nocheck && m_lastrattacktime > UNIXTIME2) return;
	m_lastrattacktime = UNIXTIME2 + PLAYER_R_HIT_REQUEST_INTERVAL;

	uint32 m_range = 0;
	if (!pRightTable.isnull() && !isMage())
	{
		if (pRightTable.isTimingDelay())
		{
			if (delaytime < (pRightTable.m_sDelay + 9) || distance > pRightTable.m_sRange)
				return;
		}
		else if (pRightTable.isWirinomUniqDelay() || pRightTable.isWirinomRebDelay() || pRightTable.isGargesSwordDelay())
		{
			if (delaytime < (pRightTable.m_sDelay - 4) || distance > pRightTable.m_sRange)
				return;
		}
		else
		{
			if (delaytime < pRightTable.m_sDelay || distance > pRightTable.m_sRange)
				return;
		}
		m_range = pRightTable.m_sRange;
	}
	// Empty handed.
	else if (delaytime < 100 && !nocheck)
		return;

	pTarget = g_pMain->GetUnitPtr(tid, GetZoneID());
	if (!pTarget)
		return;

	if (GetZoneID() == ZONE_JURAID_MOUNTAIN
		&& pTarget->isNPC()
		&& !CheckDevaAttack(pTarget->isNPC(), pTarget->isNPC() ? TO_NPC(pTarget)->GetProtoID() : 0))
		return;

	if (pTarget->isNPC()) {

		uint8 type = TO_NPC(pTarget)->GetType();
		if (type == NPC_GUARD_TOWER1 || type == NPC_GUARD_TOWER2 || type == NPC_SOCCER_BAAL)
			return;

		if (GetZoneID() == ZONE_DELOS) {
			if (!TO_NPC(pTarget)->isMonster() 
				&& type != NPC_DESTROYED_ARTIFACT 
				&& type != NPC_OBJECT_FLAG 
				&& type != NPC_GATE)
				return;

			if (type == NPC_DESTROYED_ARTIFACT
				&& (!g_pMain->isCswActive() || !g_pMain->isCswWarActive() || !isInClan()
					|| g_pMain->pSiegeWar.sMasterKnights == GetClanID()))
				return;
		}
	}

	if (!isAttackable(pTarget))
		return;

	if (!isInAttackRange(pTarget))
		return;

	if (!CanAttack(pTarget))
		return;

	if (pTarget->isNPC() && TO_NPC(pTarget)->m_OrgNation == 3)
		return;

	bResult = ATTACK_FAIL;

	if (isInTempleEventZone() && (!isSameEventRoom(pTarget) || !g_pMain->pTempleEvent.isAttackable))
		return;

	if (isInTempleQuestEventZone() && (!isSameEventRoom(pTarget) && m_ismsevent))
		return;

	if (pTarget->isPlayer() && pTarget->hasBuff(BUFF_TYPE_FREEZE))
		return;

	if (m_range && pTarget) 
	{
		float mesafe = GetDistanceSqrt(pTarget);
		if (mesafe + 1.0f > (float)m_range)
			return;
	}

	damage = GetDamage(pTarget);

	// Can't use R attacks in the Snow War.
	if (GetZoneID() == ZONE_SNOW_BATTLE && g_pMain->m_byBattleOpen == SNOW_BATTLE)
		damage = 0;
	else if (GetZoneID() == ZONE_CHAOS_DUNGEON && g_pMain->pTempleEvent.isAttackable)
		damage = 500 / 10;

	if (GetZoneID() == ZONE_DUNGEON_DEFENCE) {
		_DUNGEON_DEFENCE_ROOM_INFO* pRoomBilgi = g_pMain->m_DungeonDefenceRoomListArray.GetData(GetEventRoom());
		if (pRoomBilgi != nullptr) {
			if (pRoomBilgi->m_DefenceisStarted == true)
				damage = 500 / 10;
		}
	}

	if (isInTempleEventZone() && !virt_eventattack_check())
		return;

	if (isInTempleEventZone() && (!isSameEventRoom(pTarget) || !g_pMain->pTempleEvent.isAttackable))
		return;

	if (pTarget->isNPC())
	{
		if (TO_NPC(pTarget)->GetType() == NPC_PRISON)
		{
			if (GetMana() < ((int32)m_MaxMp * 5 / 100))
				return;

			_ITEM_DATA* pItem;
			_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
			if (pItem == nullptr || pTable.isnull()
				|| pItem->sDuration <= 0 // are we supposed to wear the pickaxe on use? Need to verify.
				|| !pTable.isPunishmentStick())
				damage = 0;
			else
			{
				damage = 1;
				MSpChange(-((int32)m_MaxMp * 5 / 100));
			}
		}
		else if (TO_NPC(pTarget)->GetType() == NPC_FOSIL)
		{
			_ITEM_DATA* pItem;
			_ITEM_TABLE pTable = GetItemPrototype(RIGHTHAND, pItem);
			if (pItem == nullptr || pTable.isnull()
				|| pItem->sDuration <= 0 // are we supposed to wear the pickaxe on use? Need to verify.
				|| !pTable.isPickaxe())
				damage = 0;
			else
				damage = 1;
		}
		else if (TO_NPC(pTarget)->m_OrgNation == 3) // R Atack Kapat�ld� 27.09.2020 
		{
			damage = 0;
		}
		else if (TO_NPC(pTarget)->GetType() == NPC_OBJECT_FLAG && TO_NPC(pTarget)->GetProtoID() == 511)
			damage = 1;
		else if (TO_NPC(pTarget)->GetType() == NPC_REFUGEE)
		{
			if (TO_NPC(pTarget)->isMonster())
			{
				if (TO_NPC(pTarget)->GetProtoID() == 3202 || TO_NPC(pTarget)->GetProtoID() == 3203
					|| TO_NPC(pTarget)->GetProtoID() == 3252 || TO_NPC(pTarget)->GetProtoID() == 3253)
					damage = 20;
				else
					damage = 10;
			}
			else
				damage = 10;
		}
		else if (TO_NPC(pTarget)->GetType() == NPC_TREE /*|| TO_NPC(pTarget)->GetType() == NPC_SANTA*/)
			damage = 20;
		else if (TO_NPC(pTarget)->GetNation() == (uint8)Nation::NONE && TO_NPC(pTarget)->GetType() == NPC_PARTNER_TYPE)
			damage = 0;
		else if (VaccuniAttack())
			damage = 30000;
		else if (TO_NPC(pTarget)->GetType() == NPC_BORDER_MONUMENT)
			damage = 10;
	}	

	if (damage > 0)
	{
		pTarget->HpChange(-damage, this);
		if (pTarget->isDead())
			bResult = ATTACK_TARGET_DEAD;
		else
			bResult = ATTACK_SUCCESS;

		// Every attack takes a little of your weapon's durability.
		ItemWoreOut(ATTACK, damage);

		// Every hit takes a little of the defender's armour durability.
		if (pTarget->isPlayer())
			TO_USER(pTarget)->ItemWoreOut(DEFENCE, damage);
	}
	
	Packet result(WIZ_ATTACK, bType);
	result << bResult << GetSocketID() << tid << unknown;
	SendToRegion(&result, nullptr, GetEventRoom());
}

void CUser::Regene(uint8 regene_type, uint32 magicid /*= 0*/)
{
	if(GetMap() == nullptr)
		return;

	_OBJECT_EVENT* pEvent = nullptr;
	_START_POSITION* pStartPosition = nullptr;
	float x = 0.0f, z = 0.0f;

	if (!isDead())
		return;

	// 1vs1 Event S�f�rlama //

	if (GetZoneID() == ZONE_STAR_NEO_VS && GetEventRoom() > 1 && m_VsDurumu == false)
		VSEventProcess(2); // event

	if (regene_type != 1 && regene_type != 2)
		regene_type = 1;

	if (regene_type == 2) 
	{
		// Is our level high enough to be able to resurrect using this skill?
		if (GetLevel() <= 5
			// Do we have enough resurrection stones?
				|| !RobItem(379006000, 3 * GetLevel()))
				return;
	}

	// If we're in a home zone, we'll want the coordinates from there. Otherwise, assume our own home zone.
	pStartPosition = g_pMain->m_StartPositionArray.GetData(GetZoneID());
	if (pStartPosition == nullptr)
		return;

	UserInOut(INOUT_OUT);

	pEvent = GetMap()->GetObjectEvent(m_sBind);	

	bool isCind = g_pMain->pCindWar.isStarted() && g_pMain->isCindirellaZone(GetZoneID()) && pCindWar.isEventUser();

	// If we're not using a spell to resurrect.
	if (magicid == 0) 
	{
		// Resurrect at a bind/respawn point
		if (pEvent && pEvent->byLife == 1)
		{
			SetPosition(pEvent->fPosX + x, 0.0f, pEvent->fPosZ + z);
			x = pEvent->fPosX;
			z = pEvent->fPosZ;
		}
		// Are we trying to respawn in a home zone?
		// If we're in a war zone (aside from snow wars, which apparently use different coords), use BattleZone coordinates.
		else if ((GetZoneID() <= ZONE_ELMORAD) || (GetZoneID() != ZONE_SNOW_BATTLE && GetZoneID() == (ZONE_BATTLE_BASE + g_pMain->m_byBattleZone))) 
		{
			// Use the proper respawn area for our nation, as the opposite nation can
			// enter this zone at a war's invasion stage.
			x = (float)((GetNation() == (uint8)Nation::KARUS ? pStartPosition->sKarusX :  pStartPosition->sElmoradX) + myrand(0, pStartPosition->bRangeX));
			z = (float)((GetNation() == (uint8)Nation::KARUS ? pStartPosition->sKarusZ :  pStartPosition->sElmoradZ) + myrand(0, pStartPosition->bRangeZ));
		}
		else
		{
			short sx, sz;
			// If we're in a war zone (aside from snow wars, which apparently use different coords), use BattleZone coordinates.
			if (isInMoradon() && isInArena())
			{
				x = (float)(MINI_ARENA_RESPAWN_X + myrand(-MINI_ARENA_RESPAWN_RADIUS, MINI_ARENA_RESPAWN_RADIUS));
				z = (float)(MINI_ARENA_RESPAWN_Z + myrand(-MINI_ARENA_RESPAWN_RADIUS, MINI_ARENA_RESPAWN_RADIUS));
			}
			else if (GetZoneID() == ZONE_CHAOS_DUNGEON  || (g_pMain->tBowlEventZone == GetZoneID() && g_pMain->isBowlEventActive))
			{
				GetStartPositionRandom(sx, sz);
				x = sx;
				z = sz;
			}
			else if (GetZoneID() == ZONE_JURAID_MOUNTAIN && isInValidRoom(0)) {
				auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(GetEventRoom());
				if (pRoomInfo) {
					if (isDevaStage(pRoomInfo)) {
						if ((Nation)GetNation() == Nation::KARUS) {
							x = float(511 + myrand(0, 3)), z = float(738 + myrand(0, 3));
						}
						else {
							x = float(511 + myrand(0, 3)), z = float(281 + myrand(0, 3));
						}
					}
					else if (isBridgeStage2(pRoomInfo)) {
						if ((Nation)GetNation() == Nation::KARUS) {
							x = float(336 + myrand(0, 3)), z = float(848 + myrand(0, 3));
						}
						else {
							x = float(695 + myrand(0, 3)), z = float(171 + myrand(0, 3));
						}
					}
					else if (isBridgeStage1(pRoomInfo)) {
						if ((Nation)GetNation() == Nation::KARUS) {
							x = float(224 + myrand(0, 3)), z = float(671 + myrand(0, 3));
						}
						else {
							x = float(800 + myrand(0, 3)), z = float(349 + myrand(0, 3));
						}
					}
				}
			}
			// For all else, just grab the start position (/town coordinates) from the START_POSITION table.
			else
			{

				GetStartPosition(sx, sz, 0,isCind);
				x = sx;
				z = sz;
			}

			if (x == 0 && z == 0)
			{
				GetStartPosition(sx, sz, 0, isCind);
				x = sx;
				z = sz;
			}
		}

		SetPosition(x, 0.0f, z);

		m_LastX = x;
		m_LastZ = z;

		m_bResHpType = USER_STANDING;	
		m_bRegeneType = REGENE_NORMAL;

		if (isCind)
			CindirellaChaModify(true);
	}
	else // we're respawning using a resurrect skill.
	{
		_MAGIC_TYPE5 * pType = g_pMain->m_Magictype5Array.GetData(magicid);     
		if (pType == nullptr)
			return;

		if (GetZoneID() != ZONE_UNDER_CASTLE)
		{
			MSpChange(-((int32)m_MaxMp)); // reset us to 0 MP. 

			if (m_sWhoKilledMe == -1)
				ExpChange("restore back", (m_iLostExp * pType->bExpRecover) / 100, true); // Restore 
		}

		m_bResHpType = USER_STANDING;
		m_bRegeneType = REGENE_MAGIC;
	}

	Packet result(WIZ_REGENE);
	result << GetSPosX() << GetSPosZ() << GetSPosY();
	Send(&result);

	m_tLastRegeneTime = UNIXTIME;
	m_sWhoKilledMe = -1;
	m_iLostExp = 0;
	m_bCity = 0;

	SetRegion(GetNewRegionX(), GetNewRegionZ());

	UserInOut(INOUT_RESPAWN);		

	g_pMain->RegionUserInOutForMe(this);
	g_pMain->RegionNpcInfoForMe(this);

	InitializeStealth();
	SendUserStatusUpdate(UserStatus::USER_STATUS_DOT, UserStatusBehaviour::USER_STATUS_CURE);
	SendUserStatusUpdate(UserStatus::USER_STATUS_POISON, UserStatusBehaviour::USER_STATUS_CURE);

	if (isInArena())
		SendUserStatusUpdate(UserStatus::USER_STATUS_SPEED,UserStatusBehaviour::USER_STATUS_CURE);

	HpChange(GetMaxHealth());

	if (GetZoneID() == ZONE_UNDER_CASTLE)
		MSpChange(GetMaxMana());

	if (!isBlinking()
		&& GetZoneID() != ZONE_CHAOS_DUNGEON
		&& GetZoneID() != ZONE_DUNGEON_DEFENCE
		&& GetZoneID() != ZONE_KNIGHT_ROYALE)
	{
		InitType4();
		RecastSavedMagic();
	}

	if(GetZoneID() == ZONE_CHAOS_DUNGEON)
		BlinkStart(-10);
	else if (magicid == 0 && !isNPCTransformation())
		BlinkStart();

	ZoneOnlineRewardChange();

	// If we actually respawned (i.e. we weren't resurrected by a skill)...
	if (magicid == 0)
	{
		bool cindirella = g_pMain->pCindWar.isStarted() && pCindWar.isEventUser() && g_pMain->isCindirellaZone(GetZoneID());
		// In PVP zones (not war zones), we must kick out players if they no longer
		// have any national points.
		if (GetLoyalty() == 0 && (GetMap()->isWarZone() || isInSpecialEventZone() || isInPKZone() || cindirella || BIFROST_EVENT)) // DeaFSezo np 0 kald� ise bifrost haritas�ndan atma Eklendim 28.12.2024
			KickOutZoneUser();
	}

	// JstKO User Rise Effect Eklendi 01.01.2025
	if (g_pMain->UserRiseEffect)
		ShowEffect(492032);
}