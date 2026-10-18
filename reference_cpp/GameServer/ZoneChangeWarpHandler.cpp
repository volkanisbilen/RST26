#include "stdafx.h"
#include "MagicInstance.h"
bool CUser::ZoneChange(uint16 sNewZone, float x, float z, int16 eventroom /*= -1*/, bool check /*= true*/, bool arrest_summon /*= false*/, bool v_eventlogout)
{
	// Stability workaround: treat Delos Castellan as Delos map for client compatibility.
	if (sNewZone == ZONE_DELOS_CASTELLAN)
		sNewZone = ZONE_DELOS;

	C3DMap * pMap = g_pMain->GetZoneByID(sNewZone);
	if (pMap == nullptr || pMap->m_Status != 1)
		return false;

	WarpListResponse errorReason;

	uint8 oldzone = m_bZone; uint16 oldeventroom = m_bEventRoom;
	if (check && !CanChangeZone(pMap, errorReason))
	{
		Packet result(WIZ_WARP_LIST, uint8(2));
		result << uint8(errorReason);
		if (errorReason == WarpListResponse::WarpListMinLevel)
			result << pMap->GetMinLevelReq();

		Send(&result);
		if (m_bCheckWarpZoneChange)
			m_bCheckWarpZoneChange = false;
		return false;
	}

	bool cindrella = IsCindIn((uint8)sNewZone);
	bool specialev = g_pMain->pSpecialEvent.opened && isInSpecialEventZone((uint8)sNewZone);
	if (cindrella) {

#if 0
		if (GetZoneID() == sNewZone)
			return false;
#endif

		std::string warname = "Fun Class Event", explanation = "";

		auto pSet = g_pMain->pCindWar.pSetting[g_pMain->pCindWar.settingid];
		if (!GetLoyalty()) {
			explanation = string_format("Your loyalty points are insufficient.", pSet.reqloyalty);
			XSafe_SendMessageBox(warname.c_str(), explanation.c_str());
			return false;
		}

		if (GetLoyalty() < pSet.reqloyalty) {
			explanation = string_format("You must have a minimum of %d loyalty points to participate in the event.", pSet.reqloyalty);
			XSafe_SendMessageBox(warname.c_str(), explanation.c_str());
			return false;
		}

		if (GetCoins() < pSet.reqmoney) {
			explanation = string_format("You must have a minimum of %d coins to participate in the event.", pSet.reqmoney);
			XSafe_SendMessageBox(warname.c_str(), explanation.c_str());
			return false;
		}

		if (GetLevel() < pSet.minlevel) {
			explanation = string_format("You must be at least level %d to participate in the event.", pSet.minlevel);
			XSafe_SendMessageBox(warname.c_str(), explanation.c_str());
			return false;
		}

		if (GetLevel() > pSet.maxlevel) {
			explanation = string_format("Levels between a minimum of %d and a maximum of %d can participate in the event.", pSet.minlevel, pSet.maxlevel);
			XSafe_SendMessageBox(warname.c_str(), explanation.c_str());
			return false;
		}

		if (pSet.maxuserlimit && g_pMain->pCindWar.GetTotalCount() > pSet.maxuserlimit) {
			explanation = string_format("the maximum number of users has been reached. please try again later..", pSet.minlevel, pSet.maxlevel);
			XSafe_SendMessageBox(warname.c_str(), explanation.c_str());
			return false;
		}
	}

	if (GetZoneID() == ZONE_DELOS && g_pMain->m_byBattleSiegeWarOpen && sNewZone != ZONE_DELOS)
	{
		Packet result2(WIZ_BIFROST, uint8(5));
		result2 << uint16(1);
		Send(&result2);
	}

	if (sNewZone == ZONE_FORGOTTEN_TEMPLE && !isGM()) {

		if (!g_pMain->isForgettenTempleActive())
			return false;

		g_pMain->pForgettenTemple.UserListLock.lock();
		if (g_pMain->pForgettenTemple.UserList.size() >= FORGETTEN_TEMPLE_MAX_USER) {
			g_pMain->pForgettenTemple.UserListLock.unlock();
			return false;
		}
		g_pMain->pForgettenTemple.UserListLock.unlock();
	}

	// Random respawn position...
	if (sNewZone == ZONE_CHAOS_DUNGEON)
	{
		short sx, sz;
		GetStartPositionRandom(sx, sz, (uint8)sNewZone);
		x = (float)sx;
		z = (float)sz;
	}
	else if (sNewZone == ZONE_DELOS) {
		if (isInClan()) {
			if (g_pMain->pSiegeWar.sMasterKnights == GetClanID()) {
				if (GetNation() == (uint8)Nation::KARUS) {
					x = (float)(455 + myrand(0, 5));
					z = (float)(790 + myrand(0, 5));
				}
				else {
					x = (float)(555 + myrand(0, 5));
					z = (float)(790 + myrand(0, 5));
				}
			}
		}
	}

	if (!arrest_summon && (specialev || cindrella)) {
		if (cindrella && GetZoneID() != sNewZone)
		{
			x = 0.0f; z = 0.0f;
		}
	}

	if (x == 0.0f && z == 0.0f) {
		_START_POSITION * pStartPosition = g_pMain->GetStartPosition(sNewZone);
		if (pStartPosition != nullptr)
		{
			x = (float)(GetNation() == (uint8)Nation::KARUS ? pStartPosition->sKarusX : pStartPosition->sElmoradX + myrand(0, pStartPosition->bRangeX));
			z = (float)(GetNation() == (uint8)Nation::KARUS ? pStartPosition->sKarusZ : pStartPosition->sElmoradZ + myrand(0, pStartPosition->bRangeZ));
		}
	}

	m_bWarp = true;
	m_bZoneChangeFlag = true; 

	if (g_pMain->pCollectionRaceEvent.isCRActive && sNewZone != g_pMain->pCollectionRaceEvent.ZoneID)
		CollectionRaceHide();

	if (GetZoneID() == sNewZone) {
		m_bZoneChangeFlag = false;
		m_bWarp = false;
		Warp(uint16(x * 10), uint16(z * 10));
		PetHome(uint16(x * 10), 0, uint16(z * 10));
		return false;
	}

	if (sNewZone == ZONE_FORGOTTEN_TEMPLE) {
		g_pMain->pForgettenTemple.UserListLock.lock();
		auto itr = g_pMain->pForgettenTemple.UserList.find(GetID());
		if (itr != g_pMain->pForgettenTemple.UserList.end())
			itr->second = 0;
		else
			g_pMain->pForgettenTemple.UserList.emplace(std::make_pair(GetID(), 0));
		g_pMain->pForgettenTemple.UserListLock.unlock();
	}

	if (isTransformed() && (sNewZone != ZONE_ELMORAD
		&& sNewZone != ZONE_ELMORAD2
		&& sNewZone != ZONE_ELMORAD3
		&& sNewZone != ZONE_KARUS
		&& sNewZone != ZONE_KARUS2
		&& sNewZone != ZONE_KARUS3
		&& sNewZone != ZONE_ELMORAD_ESLANT
		&& sNewZone != ZONE_ELMORAD_ESLANT2
		&& sNewZone != ZONE_ELMORAD_ESLANT3
		&& sNewZone != ZONE_KARUS_ESLANT
		&& sNewZone != ZONE_KARUS_ESLANT2
		&& sNewZone != ZONE_KARUS_ESLANT3
		&& sNewZone != ZONE_MORADON
		&& sNewZone != ZONE_MORADON2
		&& sNewZone != ZONE_MORADON3
		&& sNewZone != ZONE_MORADON4
		&& sNewZone != ZONE_MORADON5))
	{
		if (isTransformed() && m_transformationType == Unit::TransformationType::TransformationMonster)
		{
			//printf("silindi\n");
			Packet result(WIZ_MAGIC_PROCESS, uint8(MagicOpcode::MAGIC_CANCEL_TRANSFORMATION));
			m_transformationType = Unit::TransformationType::TransformationNone;
			Send(&result);
			RemoveSavedMagic(m_bAbnormalType);
			StateChangeServerDirect(3, ABNORMAL_NORMAL);
		}
	}

	if (sNewZone == ZONE_CHAOS_DUNGEON)
		m_sHp = 10000;

	if (!isInSpecialEventZone(uint8(sNewZone)) && isInSpecialEventZone() && g_pMain->pSpecialEvent.opened)
		ZindanWarLogOut();

	m_LastX = x;
	m_LastZ = z;

	UserInOut(INOUT_OUT);

	SetZoneAbilityChange(sNewZone);
	g_pMain->KillNpc(GetSocketID(), GetZoneID(), this);
	if (isWantedUser()) NewWantedEventLoqOut();
	if (GetZoneID() == ZONE_BATTLE6 && GetTowerID() != -1) TowerExitsFunciton();
	// Reset the user's anger gauge when leaving the zone
	// Unknown if this is official behaviour, but it's logical.
	if (GetAngerGauge() > 0)
		UpdateAngerGauge(0);

	m_bZoneChangeSameZone = false;
	m_bZoneChangeControl = true;

	if (isInPKZone(sNewZone) || (isInPKZone() && !isInPKZone(sNewZone))) KA_ResetCheck(true);

	if (isInParty() && isPartyLeader()) {
		auto* pParty = g_pMain->GetPartyPtr(GetPartyID());
		if (pParty != nullptr) PartyLeaderPromote(pParty->uid[1]);
	}

	PartyNemberRemove(GetSocketID());

	if (hasRival())
		RemoveRival();

	PetOnDeath();
	ResetWindows();
	
	if (isInMonsterStoneZone() && m_ismsevent) TempleMonsterStoneItemExitRoom();

	/*Event is Disband.*/
	switch (GetZoneID())
	{
	case ZONE_SNOW_BATTLE:
	{
		if (sNewZone != ZONE_SNOW_BATTLE)
			SetMaxHp(1);
	}
	break;
	case ZONE_CHAOS_DUNGEON:
	{
		if (sNewZone != ZONE_CHAOS_DUNGEON)
		{
			if (isEventUser())
			{
				SetMaxHp(1);
				RobChaosSkillItems();
				m_bEventRoom = 0;
			}
		}
	}
	break;
	case ZONE_JURAID_MOUNTAIN:
	{
		if (sNewZone != ZONE_JURAID_MOUNTAIN)
		{
			if (isEventUser())
			{
				m_bEventRoom = 0;
			}
		}
	}
	break;
	case ZONE_BORDER_DEFENSE_WAR:
	{
		if (sNewZone != ZONE_BORDER_DEFENSE_WAR)
		{
			if (isEventUser())
			{
				m_bEventRoom = 0;
			}
		}
	}
	break;
	case ZONE_DUNGEON_DEFENCE:
	{
		if (sNewZone != ZONE_DUNGEON_DEFENCE)
		{
			DungeonDefenceRobItemSkills();
			SetMaxHp(1);
			m_bEventRoom = 0;
		}
	}
	break;
	case ZONE_DRAKI_TOWER:
	{
		if (sNewZone != ZONE_DRAKI_TOWER)
		{
			m_bEventRoom = 0;
		}
	}
	break;
	}

	/* Event is sign*/
	switch (sNewZone)
	{
	case ZONE_UNDER_CASTLE:
		g_pMain->m_nUnderTheCastleUsers.push_back(GetSocketID());
		break;
	}

	if (GetZoneID() == ZONE_FORGOTTEN_TEMPLE) {
		g_pMain->pForgettenTemple.UserListLock.lock();
		g_pMain->pForgettenTemple.UserList.erase(GetID());
		g_pMain->pForgettenTemple.UserListLock.unlock();
	}
	
	BottomUserLogOut();

	if (!v_eventlogout
		&& isInTempleEventZone()
		&& isEventUser()
		&& !isInTempleEventZone((uint8)sNewZone))
	{
		if (GetZoneID() == ZONE_BORDER_DEFENSE_WAR) {
			BDWUserLogOut();
			BorderDefenceRemovePlayerRank();

			if (m_bHasAlterOptained)
			{
				CMagicProcess::RemoveType4Buff((uint8)BuffType::BUFF_TYPE_FRAGMENT_OF_MANES, this, true, true);
				m_bHasAlterOptained = false;
			}
		}
		else if (GetZoneID() == ZONE_CHAOS_DUNGEON)
		{
			RobChaosSkillItems();
			ChaosExpansionRemovePlayerRank();
			SetMaxHp(1);
		}
		ResetTempleEventData();
	}

	if (eventroom == 0 && GetEventRoom() > 0)
		m_bEventRoom = 0;
	else if (eventroom > 0)
		m_bEventRoom = eventroom;

	if (GetZoneID() == ZONE_DRAKI_TOWER && isDead()) {
		Packet newpkt(WIZ_REGENE);
		newpkt << GetSPosX() << GetSPosZ() << GetSPosY();
		Send(&newpkt);
	}

	m_bZone = (uint8)sNewZone; // this is 2 bytes to support the warp data loaded from SMDs. It should not go above a byte, however.
	SetPosition(x, 0.0f, z);
	m_pMap = pMap;

	ZoneOnlineRewardChange();

	SetRegion(GetNewRegionX(), GetNewRegionZ());

	UserInOut(InOutType::INOUT_WARP);
	Packet result(WIZ_ZONE_CHANGE, uint8(ZoneChangeTeleport));
	result << uint16(GetZoneID()) << GetSPosX() << GetSPosZ() << GetSPosY() << GetNation() << g_pMain->m_byOldVictory;
	Send(&result);

	if (pCindWar.isEventUser() && g_pMain->isCindirellaZone((uint8)oldzone) && !g_pMain->isCindirellaZone((uint8)sNewZone))
		CindirellaLogOut();

	if (!m_bZoneChangeSameZone)
	{
		m_sWhoKilledMe = -1;
		m_iLostExp = 0;
		m_bRegeneType = 0;
		m_tLastRegeneTime = 0;
		m_sBind = -1;
		InitType3();
		InitType4(false); //23.04.2024
		Type9Duration(true);
		SetUserAbility();

		// You don't need anymore 
		if (m_bNeedParty == 2)
		{
			g_pMain->m_SeekingPartyArrayLock.lock();
			foreach(itr, g_pMain->m_SeekingPartyArray)
			{
				if ((*itr) == nullptr)
					continue;

				if ((*itr)->m_strUserID == GetName())
				{
					(*itr)->m_bZone = uint8(sNewZone);
					break;
				}
			}
			g_pMain->m_SeekingPartyArrayLock.unlock();
		}
	}

	g_pMain->TempleEventSendActiveEventTime(this);

	if (cindrella)
		CindirellaSign();

	if (sNewZone == ZONE_DELOS)
	{
		csw_sendwarflag();
		csw_rank_register();
	}
		
	return true;
}

/**
* @brief	Changes the zone of all party members within the user's zone.
* 			If the user is not in a party, they should still be teleported.
*
* @param	sNewZone	ID of the new zone.
* @param	x			The x coordinate.
* @param	z			The z coordinate.
*/
void CUser::ZoneChangeParty(uint16 sNewZone, float x, float z)
{
	if (!isInParty()) { 
		ZoneChange(sNewZone, x, z); 
		return; 
	}

	auto* pParty = g_pMain->GetPartyPtr(GetPartyID());
	if (pParty == nullptr)
	{
		ZoneChange(sNewZone, x, z);
		return;
	}
		
	std::vector<CUser*> willTP;
	for (int i = 0; i < GetPartyMemberAmount(); i++) {
		CUser* pUser = g_pMain->GetUserPtr(pParty->uid[i]);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		willTP.push_back(pUser);
	}
	foreach(itr, willTP) if (*itr) (*itr)->ZoneChange(sNewZone, x, z);
}

/**
* @brief	Changes the zone of all clan members in home/neutral zones (including Eslant).
* 			If the user is not in a clan, they should not be teleported.
*
* @param	sNewZone	ID of the new zone.
* @param	x			The x coordinate.
* @param	z			The z coordinate.
*/
void CUser::ZoneChangeClan(uint16 sNewZone, float x, float z)
{
	CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
	if (pKnights == nullptr)
		return;

	std::vector<std::string> templist;
	pKnights->m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(itr, pKnights->m_arKnightsUser) {
		_KNIGHTS_USER* p = itr->second;
		if (p == nullptr)
			continue;

		templist.push_back(p->strUserName);
	}
	pKnights->m_arKnightsUser.m_lock.unlock();

	foreach(itr, templist) {
		CUser* pTUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (pTUser == nullptr || !pTUser->isInGame() || pTUser->isInTempleEventZone())
			continue;

		pTUser->ZoneChange(sNewZone, x, z);
	}
}

#pragma region CUser::Warp(uint16 sPosX, uint16 sPosZ)
void CUser::Warp(uint16 sPosX, uint16 sPosZ)
{
	if (GetMap() == nullptr)
		return;

	if (m_bWarp)
		return;

	float real_x = sPosX / 10.0f, real_z = sPosZ / 10.0f;
	if (!GetMap()->IsValidPosition(real_x, real_z, 0.0f))
	{
		TRACE("Invalid position %f,%f\n", real_x, real_z);
		return;
	}

	m_LastX = real_x;
	m_LastZ = real_z;

	Packet result(WIZ_WARP);
	result << sPosX << sPosZ;
	Send(&result);

	UserInOut(INOUT_OUT);

	m_curx = real_x;
	m_curz = real_z;
	curX1 = sPosX;
	curZ1 = sPosZ;
	m_oldwillx = sPosX;
	m_oldwillz = sPosZ;

	SetRegion(GetNewRegionX(), GetNewRegionZ());

	UserInOut(INOUT_WARP);
	g_pMain->UserInOutForMe(this);
	NpcInOutForMe();
	g_pMain->MerchantUserInOutForMe(this);
	ResetWindows();
}
#pragma endregion

#pragma region CUser:: RecvWarp(Packet & pkt)
void CUser::RecvWarp(Packet & pkt)
{
	if (isDead() 
		|| !isGM())
		return;

	uint16 PosX, PosZ;
	pkt >> PosX >> PosZ;
	Warp(PosX, PosZ);
}
#pragma endregion

void CUser::RecvZoneChange(Packet & pkt)
{
	uint8 opcode = pkt.read<uint8>();
	if (opcode == ZoneChangeLoading)
	{
		if (!m_bZoneChangeControl) // we also need to make sure we're actually waiting on this request...
			return;

		// JstKO Giris Gate Zone Notice Eklendi. 04.01.2025
		if (g_pMain->Giri�GateZoneNotice)
			JstKOGateZoneNotice();

		g_pMain->RegionUserInOutForMe(this);
		g_pMain->RegionNpcInfoForMe(this);
		g_pMain->MerchantUserInOutForMe(this);
		
		Packet result(WIZ_ZONE_CHANGE, uint8(ZoneChangeLoaded)); // finalise the zone change
		Send(&result);
	}
	else if (opcode == ZoneChangeLoaded)
	{
		if (!m_bZoneChangeControl) // we also need to make sure we're actually waiting on this request...
			return;

		UserInOut(INOUT_RESPAWN);
		m_bZoneChangeFlag = false;

		if (!m_bZoneChangeSameZone) {
			if (GetZoneID() == ZONE_CHAOS_DUNGEON || GetZoneID() == ZONE_KNIGHT_ROYALE || GetZoneID() == ZONE_DUNGEON_DEFENCE) BlinkStart(45);
			else if (!isNPCTransformation()) BlinkStart();
		}

		if (hasPriestBot == true)
		{
			CBot* pPriest = nullptr;
			pPriest = g_pMain->m_MapBotList.GetData(m_bUserPriestBotID);
			if (pPriest)
				pPriest->UserInOut(INOUT_OUT);

			m_bUserPriestBotID = -1;
			hasPriestBot = false;
			RemoveSavedMagic(495146);// sleave priest aynadan ge�ince fix DeaFSezo
			
			if (!isPriestUseZone())
			{
				CUser* pUsers = g_pMain->GetUserPtr(GetID());
				uint16 m_bUserPriestBotID = g_pMain->SpawnPriestBot(static_cast<int>(pPriest->LastWarpTime - UNIXTIME), pUsers);
			}

		}


		if (GetZoneID() != ZONE_CHAOS_DUNGEON && !IsCindIn())
		{
			InitType4();
			RecastSavedMagic();
		}

		if (isDead())
			SendDeathAnimation();

		if (GetZoneID() == ZONE_DELOS && g_pMain->m_byBattleSiegeWarOpen)
		{
			Packet result(WIZ_SELECT_MSG);
			result << uint16(0) << uint8(7)
				<< uint64(0) << uint32(9)
				<< uint8(11) << uint32(g_pMain->m_CSWTimer - UNIXTIME);
			Packet bresult(WIZ_BIFROST);
			bresult << uint8(5) << uint16(g_pMain->m_CSWTimer - UNIXTIME);// <<  uint8(0x07); /* BIFROST */
			Send(&result);
			Send(&bresult);
		}
		if (GetZoneID() == ZONE_DRAKI_TOWER) SendDrakiTempleDetail(true);
		if (isInSpecialEventZone() && g_pMain->pSpecialEvent.opened) ZindanWarSendScore();

		if (GetZoneID() == ZONE_CLAN_WAR_ARDREAM
			|| GetZoneID() == ZONE_CLAN_WAR_RONARK
			|| GetZoneID() == ZONE_PARTY_VS_1
			|| GetZoneID() == ZONE_PARTY_VS_2
			|| GetZoneID() == ZONE_PARTY_VS_3
			|| GetZoneID() == ZONE_PARTY_VS_4)
			TournamentSendTimer();
		
		SetZoneAbilityChange((uint16)GetZoneID());
		g_pMain->KillNpc(GetSocketID(), GetZoneID(), this);
		if (g_pMain->pCollectionRaceEvent.isCRActive /*&& GetZoneID() == g_pMain->pCollectionRaceEvent.ZoneID*/)
			CollectionGetActiveTime();
	
		m_bWarp = false;
		m_bZoneChangeControl = false;
		m_bCheckWarpZoneChange = false;

		// JstKO User Zone Change Effect Eklendi 01.01.2025
		if (g_pMain->UserZoneChangeEffect)
			ShowEffect(492028);
	}
	else if (opcode == ZoneMilitaryCamp)
	{
		uint16 ZoneID = pkt.read<uint16>();
		if (m_sMilitaryTime > UNIXTIME) return;
		m_sMilitaryTime = UNIXTIME + 7;

		if (GetMap() == nullptr || !GetMap()->m_Status || GetMap()->m_bMilitaryZone != 1 || GetZoneID() == ZoneID) return;
		if (!isInElmoradCastle() && !isInMoradon() && !isInLufersonCastle() && !isInKarusEslant() && !isInElmoradEslant()) return;

		if (isInMoradon() && ZoneID != ZONE_MORADON && ZoneID != ZONE_MORADON2 && ZoneID != ZONE_MORADON3 &&
			ZoneID != ZONE_MORADON4 && ZoneID != ZONE_MORADON5) return;

		if (isInLufersonCastle() && ZoneID != 1 && ZoneID != 5 && ZoneID != 6) 
			return;
		else if (isInElmoradCastle() && ZoneID != 2 && ZoneID != 7 && ZoneID != 8)
			return;
		else if (isInKarusEslant() && ZoneID != 11 && ZoneID != 13 && ZoneID != 14)
			return;
		else if (isInElmoradEslant() && ZoneID != 12 && ZoneID != 15 && ZoneID != 16)
			return;

		if (g_pMain->isWarOpen() && (isInLufersonCastle() || isInElmoradCastle())) 
			return;

		if (isInLufersonCastle() && g_pMain->m_sKarusMilitary < 2) 
			return;
		else if (isInElmoradCastle() && g_pMain->m_sHumanMilitary < 2)
			return;
		else if (isInKarusEslant() && g_pMain->m_sKarusEslantMilitary < 2)
			return;
		else if (isInElmoradEslant() && g_pMain->m_sHumanEslantMilitary < 2)
			return;
		else if (isInMoradon() && g_pMain->m_sMoradonMilitary < 2)
			return;

		ZoneChange(ZoneID, 0.0f, 0.0f);
	}
}

void CUser::Rotate(Packet & pkt)
{
	if (isDead())
		return;

	pkt >> m_sDirection;

	Packet result(WIZ_ROTATE);
	result << GetSocketID() << m_sDirection;
	SendToRegion(&result, this, GetEventRoom());
}

bool CUser::CanChangeZone(C3DMap * pTargetMap, WarpListResponse & errorReason)
{
	// While unofficial, game masters should be allowed to teleport anywhere.
	if (isGM())
		return true;

	CKnights * pClan = nullptr;

	// Generic error reason; this should only be checked when the method returns false.
	errorReason = WarpListResponse::WarpListGenericError;

	if (GetLevel() < pTargetMap->GetMinLevelReq())
	{
		errorReason = WarpListResponse::WarpListMinLevel;
		return false;
	}

	if (GetLevel() > pTargetMap->GetMaxLevelReq() || !CanLevelQualify(pTargetMap->GetMaxLevelReq()))
	{
		errorReason = WarpListResponse::WarpListDoNotQualify;
		return false;
	}

	switch (pTargetMap->GetID())
	{
	case ZONE_KARUS:
	case ZONE_KARUS2:
	case ZONE_KARUS3:
	{
		// Users may enter Luferson (1)/El Morad (2) if they are that nation, 
		if (GetNation() == (uint8)Nation::KARUS && pTargetMap->GetID() >= ZONE_KARUS && pTargetMap->GetID() <= ZONE_KARUS3)
			return true;

		// Users may also enter if there's a war invasion happening in that zone.
		if (GetNation() == (uint8)Nation::ELMORAD)
			return g_pMain->m_byKarusOpenFlag;
		else
			return g_pMain->m_byElmoradOpenFlag;
	}
	break;
	case ZONE_ELMORAD:
	case ZONE_ELMORAD2:
	case ZONE_ELMORAD3:
	{
		// Users may enter Luferson (1)/El Morad (2) if they are that nation, 
		if (GetNation() == (uint8)Nation::ELMORAD && pTargetMap->GetID() >= ZONE_ELMORAD && pTargetMap->GetID() <= ZONE_ELMORAD3)
			return true;

		// Users may also enter if there's a war invasion happening in that zone.
		if (GetNation() == (uint8)Nation::KARUS)
			return g_pMain->m_byElmoradOpenFlag;
		else
			return g_pMain->m_byKarusOpenFlag;
	}
	break;
	case ZONE_KARUS_ESLANT:
	case ZONE_KARUS_ESLANT2:
	case ZONE_KARUS_ESLANT3:
	{
		if (GetNation() == (uint8)Nation::KARUS && pTargetMap->GetID() >= ZONE_KARUS_ESLANT && pTargetMap->GetID() <= ZONE_KARUS_ESLANT3)
			return true;
	}
	break;
	case ZONE_ELMORAD_ESLANT:
	case ZONE_ELMORAD_ESLANT2:
	case ZONE_ELMORAD_ESLANT3:
	{
		if (GetNation() == (uint8)Nation::ELMORAD && pTargetMap->GetID() >= ZONE_ELMORAD_ESLANT && pTargetMap->GetID() <= ZONE_ELMORAD_ESLANT3)
			return true;
	}
	break;
	case ZONE_DELOS: // TODO: implement CSW logic.
	{
		bool isWinnerClan = false;
		if (isInClan() && g_pMain->pSiegeWar.sMasterKnights > 0)
		{
			CKnights* pUserClan = g_pMain->GetClanPtr(GetClanID());
			if (pUserClan != nullptr)
			{
				isWinnerClan = (GetClanID() == g_pMain->pSiegeWar.sMasterKnights
					|| pUserClan->GetAllianceID() == g_pMain->pSiegeWar.sMasterKnights);
			}
		}

		if (g_pMain->isCswActive()) {

			if (!isInClan() || isInAutoClan()) {
				errorReason = WarpListResponse::WarpListNotDuringCSW;
				return false;
			}

			CKnights* pClan = g_pMain->GetClanPtr(GetClanID());
			if (pClan == nullptr) {
				errorReason = WarpListResponse::WarpListNotDuringCSW;
				return false;
			}
			if (pClan->m_byGrade > 3) {
				errorReason = WarpListResponse::WarpListDoNotQualify;
				return false;
			}
		}

		if (!isWinnerClan && GetLoyalty() <= 0)
		{
			errorReason = WarpListResponse::WarpListNeedNP;
			return false;
		}

		return true;
	}
	break;
	case ZONE_BIFROST:
		return true;
	case ZONE_ARDREAM:
		if (g_pMain->isWarOpen())
		{
			errorReason = WarpListResponse::WarpListNotDuringWar;
			return false;
		}

		if (GetLoyalty() <= 0)
		{
			errorReason = WarpListResponse::WarpListNeedNP;
			return false;
		}
		return true;

	case ZONE_RONARK_LAND_BASE:
		if (g_pMain->isWarOpen() && g_pMain->m_byBattleZoneType != ZONE_ARDREAM)
		{
			errorReason = WarpListResponse::WarpListNotDuringWar;
			return false;
		}

		if (GetLoyalty() <= 0)
		{
			errorReason = WarpListResponse::WarpListNeedNP;
			return false;
		}
		return true;


	case ZONE_RONARK_LAND:
		if (g_pMain->isWarOpen() && g_pMain->m_byBattleZoneType != ZONE_ARDREAM)
		{
			errorReason = WarpListResponse::WarpListNotDuringWar;
			return false;
		}

		if (GetLoyalty() <= 0)
		{
			errorReason = WarpListResponse::WarpListNeedNP;
			return false;
		}
		return true;

		case ZONE_SPBATTLE1:
		case ZONE_SPBATTLE2:
		case ZONE_SPBATTLE3:
		case ZONE_SPBATTLE4:
		case ZONE_SPBATTLE5:
		case ZONE_SPBATTLE6:
		case ZONE_SPBATTLE7:
		case ZONE_SPBATTLE8:
		case ZONE_SPBATTLE9:
		case ZONE_SPBATTLE10:
		case ZONE_SPBATTLE11:
		case ZONE_SPBATTLE12:
		{
			if (!GetLoyalty())
			{
				errorReason = WarpListResponse::WarpListNeedNP;
				return false;
			}
		}
		return true;
	default:
		// War zones may only be entered if that war zone is active.
		// Eski kontrolde kendi irkinin open flag'i aciksa giris engelleniyordu.
		// Bu durum ozellikle Lunar Valley savasinda El Morad/Human oyuncularin
		// savas alanina girememesine sebep oluyordu. Savas zone girisinde
		// nation invasion flag'i degil, aktif battle zone eslesmesi kontrol edilir.
		if (pTargetMap->isWarZone())
		{
			if (!g_pMain->isWarOpen())
				return false;

			if (pTargetMap->GetID() == ZONE_SNOW_BATTLE)
			{
				if (g_pMain->m_byBattleOpen != SNOW_BATTLE)
					return false;
			}
			else if ((pTargetMap->GetID() - ZONE_BATTLE_BASE) != g_pMain->m_byBattleZone)
				return false;
		}
	}
	return true;
}

bool CUser::CanLevelQualify(uint8 sLevel)
{
	int16 nStatTotal = 300 + (sLevel - 1) * 3;
	uint8 nSkillTotal = (sLevel - 9) * 2;

	if (sLevel > 60)
		nStatTotal += 2 * (sLevel - 60);

	if ((m_sPoints + GetStatTotal()) > nStatTotal || GetTotalSkillPoints() > nSkillTotal)
		return false;

	return true;
}
