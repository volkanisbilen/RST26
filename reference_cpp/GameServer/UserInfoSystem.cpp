#include "stdafx.h"
#include "KingSystem.h"
#include "DBAgent.h"

using namespace std;

/**
* @brief	Sends the player's information on initial login.
*/
void CUser::SendMyInfo(bool after /*after = false*/)
{
	C3DMap* pMap = GetMap();
	if (pMap == nullptr)
		return;

	CKnights* pKnights = nullptr;

	if (!pMap->IsValidPosition(GetX(), GetZ(), 0.0f))
	{
		short x = 0, z = 0;
		GetStartPosition(x, z);

		m_curx = (float)x;
		m_curz = (float)z;
	}

	if (!after)
		AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveBecomeKing, 0, nullptr);

	Packet result(WIZ_MYINFO);

	// Load up our user rankings (for our NP symbols).
	g_pMain->GetUserRank(this);

	// Are we the King? Let's see, shall we?
	CKingSystem* pData = g_pMain->m_KingSystemArray.GetData(GetNation());
	if (pData != nullptr
		&& STRCASECMP(pData->m_strKingName.c_str(), m_strUserID.c_str()) == 0)
		m_bRank = 1; // We're da King, man.
	else
		m_bRank = 0; // totally not da King.

	result.SByte(); // character name has a single byte length
	result << GetSocketID()
		<< GetName()
		<< GetSPosX()
		<< GetSPosZ()
		<< GetSPosY()
		<< GetNation()
		<< m_bRace
		<< m_sClass
		<< m_bFace
		<< m_nHair
		<< m_bRank
		<< m_bTitle
		<< uint8(0) << uint8(0)
		/*	<< m_bIsHidingHelmet
			<< m_bIsHidingCospre*/
		<< GetLevel()
		<< m_sPoints
		<< m_iMaxExp
		<< m_iExp
		<< GetLoyalty()
		<< GetMonthlyLoyalty()
		<< GetClanID()
		<< GetFame();

	if (isInClan())
		pKnights = g_pMain->GetClanPtr(GetClanID());

	if (pKnights == nullptr)
	{
		if (isKing())
		{
			if (GetNation() == (uint8)Nation::ELMORAD)
				result << uint64(0) << uint16(KingCapeType::KNIGHTS_HUMAN_KING_CAPE) << uint32(0);
			else
				result << uint64(0) << uint16(KingCapeType::KNIGHTS_KARUS_KING_CAPE) << uint32(0);
		}
		else
			result << uint64(0) << uint16(-1) << uint32(0);
	}
	else
	{
		result << pKnights->GetAllianceID() << pKnights->m_byFlag << pKnights->GetName() << pKnights->m_byGrade << pKnights->m_byRanking << pKnights->m_sMarkVersion;

		CKnights* pMainClan = g_pMain->GetClanPtr(pKnights->GetAllianceID());
		_KNIGHTS_ALLIANCE* pAlliance = g_pMain->GetAlliancePtr(pKnights->GetAllianceID());

		if (!after)
			pKnights->OnLogin(this);

		if (pKnights->isInAlliance() && pMainClan != nullptr && pAlliance != nullptr)
		{
			if (isKing())
			{
				if (GetNation() == (uint8)Nation::ELMORAD)
					result << uint16(KingCapeType::KNIGHTS_HUMAN_KING_CAPE) << uint32(0);
				else
					result << uint16(KingCapeType::KNIGHTS_KARUS_KING_CAPE) << uint32(0);
			}
			else
			{
				if (pMainClan->isCastellanCape()) {
					if (pAlliance->sMainAllianceKnights == pKnights->GetID())
						result << pMainClan->m_castCapeID << pMainClan->m_bCastCapeR << pMainClan->m_bCastCapeG << pMainClan->m_bCastCapeB << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
					else if (pAlliance->sSubAllianceKnights == pKnights->GetID())
						result << pMainClan->m_castCapeID << pKnights->m_bCastCapeR << pKnights->m_bCastCapeG << pKnights->m_bCastCapeB << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
					else if (pAlliance->sMercenaryClan_1 == pKnights->GetID() || pAlliance->sMercenaryClan_2 == pKnights->GetID())
						result << pMainClan->m_castCapeID << uint32(0); // only the cape will be present
				}
				else {
					if (pAlliance->sMainAllianceKnights == pKnights->GetID())
						result << pMainClan->GetCapeID() << pMainClan->m_bCapeR << pMainClan->m_bCapeG << pMainClan->m_bCapeB << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
					else if (pAlliance->sSubAllianceKnights == pKnights->GetID())
						result << pMainClan->GetCapeID() << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
					else if (pAlliance->sMercenaryClan_1 == pKnights->GetID() || pAlliance->sMercenaryClan_2 == pKnights->GetID())
						result << pMainClan->GetCapeID() << uint32(0); // only the cape will be present
				}
			}
		}
		else
		{
			if (isKing()) {
				if (GetNation() == (uint8)Nation::ELMORAD) result << uint16(KingCapeType::KNIGHTS_HUMAN_KING_CAPE) << uint32(0);
				else result << uint16(KingCapeType::KNIGHTS_KARUS_KING_CAPE) << uint32(0);
			}
			else {
				if (pKnights->isCastellanCape())
					result << pKnights->m_castCapeID << pKnights->m_bCastCapeR << pKnights->m_bCastCapeG << pKnights->m_bCastCapeB << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
				else
					result << pKnights->GetCapeID() << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
			}
		}
	}

	result << uint8(2) << uint8(3) << uint8(4) << uint8(5) // unknown
		//result << uint8(0) << uint8(0) << uint8(0) << uint8(0) // unknown
		<< m_MaxHp << m_sHp
		<< m_MaxMp << m_sMp
		<< m_sMaxWeight << m_sItemWeight
		<< GetStat(StatType::STAT_STR) << uint8(GetStatItemBonus(StatType::STAT_STR))
		<< GetStat(StatType::STAT_STA) << uint8(GetStatItemBonus(StatType::STAT_STA))
		<< GetStat(StatType::STAT_DEX) << uint8(GetStatItemBonus(StatType::STAT_DEX))
		<< GetStat(StatType::STAT_INT) << uint8(GetStatItemBonus(StatType::STAT_INT))
		<< GetStat(StatType::STAT_CHA) << uint8(GetStatItemBonus(StatType::STAT_CHA))
		<< m_sTotalHit << m_sTotalAc
		<< uint8(m_sFireR) << uint8(m_sColdR) << uint8(m_sLightningR)
		<< uint8(m_sMagicR) << uint8(m_sDiseaseR) << uint8(m_sPoisonR)
		<< m_iGold
		<< m_bAuthority;

	if (GetLevel() < 30 && !ChickenStatus && GAME_SOURCE_VERSION == 1098)		// V2 için burasý aktif olucak.
		result << int8(50) << int8(250);
	else
		result << (m_bKnightsRank <= m_bPersonalRank ? m_bKnightsRank : int8(-1)) << (m_bPersonalRank <= m_bKnightsRank ? m_bPersonalRank : int8(-1));

	result.append(m_bstrSkill, 9);

	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = GetItem(i);
		if (pItem == nullptr)
			continue;

		result << pItem->nNum
			<< pItem->sDuration
			<< pItem->sCount
			<< pItem->bFlag
			<< pItem->sRemainingRentalTime;	// remaining time

		_ITEM_TABLE pItemTable = g_pMain->GetItemPtr(pItem->nNum);
		if (!pItemTable.isnull())
		{
			/*			if (pItemTable->isPetItem()) //  istedi
							ShowPetItemInfo(result, pItem->nSerialNum);
						else*/ if (pItemTable.Getnum() == ITEM_CYPHER_RING)
	ShowCyperRingItemInfo(result, pItem->nSerialNum);
						else
	result << uint32(0); // Item Unique ID
		}
		else
			result << uint32(0); // Item Unique ID*/

		/*if (pItem->nNum == ITEM_CYPHER_RING)
			ShowCyperRingItemInfo(result, pItem->nSerialNum);
		else
			result << uint32(0); // Item Unique ID*/

		result << pItem->nExpirationTime; // expiration date in unix time
	}

	result.SByte();
	m_bIsChicken = GetLevel() < 30 ? true : false;// CheckExistEvent(50, 1);
	result << uint8(m_bAccountStatus)	// account status (0 = none, 1 = normal prem with expiry in hours, 2 = pc room)
		<< uint8(m_PremiumMap.GetSize());

	foreach_stlmap(itr, m_PremiumMap)
	{
		_PREMIUM_DATA* pPreData = itr->second;
		if (pPreData == nullptr || pPreData->iPremiumTime == 0)
			continue;

		uint32 TimeRest;
		uint16 TimeShow;
		TimeRest = uint32(pPreData->iPremiumTime - UNIXTIME);

		if (TimeRest >= 1 && TimeRest <= 3600)
			TimeShow = 1;
		else
			TimeShow = TimeRest / 3600;

		result << pPreData->bPremiumType << TimeShow;
	}

	result << m_bPremiumInUse
		<< uint8(m_bIsChicken)	// chicken /beginner flag
		<< m_iMannerPoint
		<< g_pMain->m_sKarusMilitary
		<< g_pMain->m_sHumanMilitary
		<< g_pMain->m_sKarusEslantMilitary
		<< g_pMain->m_sHumanEslantMilitary
		<< g_pMain->m_sMoradonMilitary
		<< uint8(0)
		<< uint16(GetGenieTime())
		<< GetRebirthLevel()
		<< GetRebStatBuff(StatType::STAT_STR)
		<< GetRebStatBuff(StatType::STAT_STA)
		<< GetRebStatBuff(StatType::STAT_DEX)
		<< GetRebStatBuff(StatType::STAT_INT)
		<< GetRebStatBuff(StatType::STAT_CHA)
		<< uint64(m_iSealedExp)
		<< uint16(m_sCoverTitle)
		<< uint16(m_sSkillTitle)
		<< ReturnSymbolisOK
		<< uint32(0) << uint8(0);

	SendCompressed(&result);
	SetZoneAbilityChange(GetZoneID());

	if (g_pMain->ClanPrePazar && sClanPremStatus)
		m_bPremiumMerchant = true; //

	// Renkli pm için geliþtirildi krala özel renk felan 27.09.2020 start
	if (isGM())
	{
		m_bAuthorityColor = 20;
	}
	/*else if (isGMUser())
	{
		m_bAuthorityColor = 21;
	}*/
	else if (m_bRank == 1)
	{
		m_bAuthorityColor = 22;
	}
	else
	{
		m_bAuthorityColor = 1;
	}
	// Renkli pm için geliþtirildi krala özel renk felan 27.09.2020 end
	m_bGearSkills = true;
}

void CUser::GetUserInfo(Packet& pkt)
{
	pkt.SByte();
	pkt << GetName() << GetNation() << uint8(0) << uint8(0);
	pkt << GetClanID() << uint8(isInPKZone() == true ? uint8(0) : GetFame()); // -> [uint8(isInPKZone() == true ? uint8(0) : GetFame()] <- soldaki kod pk bölgelerinde clan baskaný normal clan uyesi gibi gözükür

	CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
	if (pKnights == nullptr)
	{
		if (isKing())
			//pkt << uint32(0) << uint16(0) << uint8(0) << uint16(KNIGHTS_KNIG_CAPE) << uint32(0) << uint8(0);//King White Cape 20.05.2020	
			pkt << uint32(0) << uint16(0) << uint8(0) << uint16(GetNation() == (uint8)Nation::KARUS ? 97 : 98) << uint32(0) << uint8(0); //King New Cape 20.05.2020	
		else
			//pkt << uint32(0) << uint16(0) << uint8(0) << uint16(GetNation() == KARUS ? 97 : 98) << uint32(0) << uint8(0); //Userlerin ýrk pelerini göstermesi kaldýrýldý
			pkt << uint32(0) << uint16(0) << uint8(0) << uint16(-1) << uint32(0) << uint8(0);
	}
	else
	{
		pkt << pKnights->GetAllianceID() << pKnights->GetName() << pKnights->m_byGrade << pKnights->m_byRanking << pKnights->m_sMarkVersion;

		CKnights* pMainClan = g_pMain->GetClanPtr(pKnights->GetAllianceID());
		_KNIGHTS_ALLIANCE* pAlliance = g_pMain->GetAlliancePtr(pKnights->GetAllianceID());

		if (pKnights->isInAlliance() && pMainClan != nullptr && pAlliance != nullptr)
		{
			if (!isKing())
			{
				if (pMainClan->isCastellanCape() && pMainClan->m_CastTime >= (uint32)UNIXTIME)
				{
					if (pAlliance->sMainAllianceKnights == pKnights->GetID())
						pkt << pMainClan->m_castCapeID << pMainClan->m_bCastCapeR << pMainClan->m_bCastCapeG << pMainClan->m_bCastCapeB << uint8(0);
					else if (pAlliance->sSubAllianceKnights == pKnights->GetID())
						pkt << pMainClan->m_castCapeID << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB << uint8(0);
					else if (pAlliance->sMercenaryClan_1 == pKnights->GetID() || pAlliance->sMercenaryClan_2 == pKnights->GetID())
						pkt << pMainClan->m_castCapeID << uint32(0);
					else
						pkt << pMainClan->m_castCapeID << pMainClan->m_bCapeR << pMainClan->m_bCapeG << pMainClan->m_bCapeB << uint8(0);
				}
				else {
					if (pAlliance->sMainAllianceKnights == pKnights->GetID())
						pkt << pMainClan->GetCapeID() << pMainClan->m_bCapeR << pMainClan->m_bCapeG << pMainClan->m_bCapeB << uint8(0);
					else if (pAlliance->sSubAllianceKnights == pKnights->GetID())
						pkt << pMainClan->GetCapeID() << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB << uint8(0);
					else if (pAlliance->sMercenaryClan_1 == pKnights->GetID() || pAlliance->sMercenaryClan_2 == pKnights->GetID())
						pkt << pMainClan->GetCapeID() << uint32(0);
					else
						pkt << pMainClan->GetCapeID() << pMainClan->m_bCapeR << pMainClan->m_bCapeG << pMainClan->m_bCapeB << uint8(0);
				}
			}
			else
				//pkt << uint16(KNIGHTS_KNIG_CAPE) << uint32(0); // cape ID //King White Cape 20.05.2020	
				pkt << uint16(GetNation() == (uint8)Nation::KARUS ? 97 : 98) << uint32(0); //King New Cape	20.05.2020	

			// not sure what this is, but it (just?) enables the clan symbol on the cape 
			// value in dump was 9, but everything tested seems to behave as equally well...
			// we'll probably have to implement logic to respect requirements.
			//pkt << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
			pkt << pKnights->m_byFlag;
		}
		else
		{
			if (!isKing()) {
				if (pKnights->isCastellanCape() && pKnights->m_CastTime >= (uint32)UNIXTIME)
					pkt << pKnights->m_castCapeID << pKnights->m_bCastCapeR << pKnights->m_bCastCapeG << pKnights->m_bCastCapeB << uint8(0);
				else
					pkt << pKnights->GetCapeID() << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB << uint8(0);
			}
			else {
				if (GetNation() == (uint8)Nation::ELMORAD) pkt << uint16(KingCapeType::KNIGHTS_HUMAN_KING_CAPE) << uint32(0); // cape ID
				else pkt << uint16(KingCapeType::KNIGHTS_KARUS_KING_CAPE) << uint32(0); // cape ID
			}
			pkt << pKnights->m_byFlag;
		}
	}
	// There are two event-driven invisibility states; dispel on attack, and dispel on move.
	// These are handled primarily server-side; from memory the client only cares about value 1 (which we class as 'dispel on move').
	// As this is the only place where this flag is actually sent to the client, we'll just convert 'dispel on attack' 
	// back to 'dispel on move' as the client expects.
	uint8 bInvisibilityType = m_bInvisibilityType;
	if (bInvisibilityType != (uint8)InvisibilityType::INVIS_NONE)
		bInvisibilityType = (uint8)InvisibilityType::INVIS_DISPEL_ON_MOVE;
	//if (IsOffCharacter()) bInvisibilityType = (uint8)InvisibilityType::INVIS_DISPEL_ON_ATTACK;
	pkt << GetLevel() << m_bRace << m_sClass
		<< GetSPosX() << GetSPosZ() << GetSPosY()
		<< m_bFace << m_nHair
		<< m_bResHpType << uint32(m_bAbnormalType)
		<< m_bNeedParty
		<< m_bAuthority
		<< m_bPartyLeader
		// kurian güncellemesi 16.12.2023
		<< bInvisibilityType << uint8(m_teamColour); // visibility state // team colour (i.e. in soccer, 0=none, 1=blue, 2=red)

	// Devil transformation status.
	if (isDevil())
		pkt << uint16(1);
	else
		pkt << uint16(0);

	pkt << m_sDirection // direction
		<< m_bIsChicken // chicken/beginner flag
		<< uint8(9) // king flag
		<< uint16(0); // unknown
	// kurian güncellemesi 16.12.2023 end
		/*<< bInvisibilityType
		<< uint8(m_teamColour)

		<< m_bIsHidingHelmet
		<< m_bIsHidingCospre
		<< m_sDirection
		<< m_bIsDevil
		<< m_bIsHidingWings
		<< m_bIsChicken
		<< m_bRank;*/

		//bool civicv = GetLevel() < 30 && !ChickenStatus && V2TAHT == 1;									// v2myko için burasý aktif olucak.
		//bool civicv = GetLevel() < 30 && !ChickenStatus && GAME_SOURCE_VERSION == 1098;				// 1098 için burasý aktif olucak.
	if (GetLevel() < 30 && !ChickenStatus && GAME_SOURCE_VERSION == 1098)		// V2 için burasý aktif olucak.
		pkt << int8(50) << int8(250);
	else
		pkt << (m_bKnightsRank <= m_bPersonalRank ? m_bKnightsRank : int8(-1)) << (m_bPersonalRank <= m_bKnightsRank ? m_bPersonalRank : int8(-1));

	uint8 equippedItems[] =
	{
		BREAST, LEG, HEAD, GLOVE, FOOT, SHOULDER, RIGHTHAND, LEFTHAND,
		CWING, CHELMET, CLEFT, CRIGHT, CTOP, CFAIRY, CTATTOO
	};

	bool isWarOpen = (g_pMain->m_byBattleOpen == NATION_BATTLE && g_pMain->m_byBattleZone + ZONE_BATTLE_BASE != ZONE_BATTLE3);
	_ITEM_DATA* pItem = nullptr;

	foreach_array(i, equippedItems)
	{
		pItem = GetItem(equippedItems[i]);
		if (pItem == nullptr)
			continue;

		if (isWarOpen)
		{
			if (isWarrior())
			{
				if (i == RIGHTEAR)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_PAULDRON << pItem->sDuration << pItem->bFlag;
				else if (i == HEAD)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_PAD << pItem->sDuration << pItem->bFlag;
				else if (i == LEFTEAR)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_HELMET << pItem->sDuration << pItem->bFlag;
				else if (i == NECK)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_GAUNTLET << pItem->sDuration << pItem->bFlag;
				else if (i == BREAST)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_BOOTS << pItem->sDuration << pItem->bFlag;
				else
					pkt << pItem->nNum << pItem->sDuration << pItem->bFlag;
			}
			else if (isRogue())
			{
				if (i == RIGHTEAR)
					pkt << (uint32)ROGUE_DRAGON_ARMOR_PAULDRON << pItem->sDuration << pItem->bFlag;
				else if (i == HEAD)
					pkt << (uint32)ROGUE_DRAGON_ARMOR_PAD << pItem->sDuration << pItem->bFlag;
				else if (i == LEFTEAR)
					pkt << (uint32)ROGUE_DRAGON_ARMOR_HELMET << pItem->sDuration << pItem->bFlag;
				else if (i == NECK)
					pkt << (uint32)ROGUE_DRAGON_ARMOR_GAUNTLET << pItem->sDuration << pItem->bFlag;
				else if (i == BREAST)
					pkt << (uint32)ROGUE_DRAGON_ARMOR_BOOTS << pItem->sDuration << pItem->bFlag;
				else
					pkt << pItem->nNum << pItem->sDuration << pItem->bFlag;
			}
			else if (isMage())
			{
				if (i == RIGHTEAR)
					pkt << (uint32)MAGE_DRAGON_ARMOR_PAULDRON << pItem->sDuration << pItem->bFlag;
				else if (i == HEAD)
					pkt << (uint32)MAGE_DRAGON_ARMOR_PAD << pItem->sDuration << pItem->bFlag;
				else if (i == LEFTEAR)
					pkt << (uint32)MAGE_DRAGON_ARMOR_HELMET << pItem->sDuration << pItem->bFlag;
				else if (i == NECK)
					pkt << (uint32)MAGE_DRAGON_ARMOR_GAUNTLET << pItem->sDuration << pItem->bFlag;
				else if (i == BREAST)
					pkt << (uint32)MAGE_DRAGON_ARMOR_BOOTS << pItem->sDuration << pItem->bFlag;
				else
					pkt << pItem->nNum << pItem->sDuration << pItem->bFlag;
			}
			else if (isPriest())
			{
				if (i == RIGHTEAR)
					pkt << (uint32)PRIEST_DRAGON_ARMOR_PAULDRON << pItem->sDuration << pItem->bFlag;
				else if (i == HEAD)
					pkt << (uint32)PRIEST_DRAGON_ARMOR_PAD << pItem->sDuration << pItem->bFlag;
				else if (i == LEFTEAR)
					pkt << (uint32)PRIEST_DRAGON_ARMOR_HELMET << pItem->sDuration << pItem->bFlag;
				else if (i == NECK)
					pkt << (uint32)PRIEST_DRAGON_ARMOR_GAUNTLET << pItem->sDuration << pItem->bFlag;
				else if (i == BREAST)
					pkt << (uint32)PRIEST_DRAGON_ARMOR_BOOTS << pItem->sDuration << pItem->bFlag;
				else
					pkt << pItem->nNum << pItem->sDuration << pItem->bFlag;
			}
			else if (isPortuKurian())
			{
				if (i == RIGHTEAR)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_PAULDRON << pItem->sDuration << pItem->bFlag;
				else if (i == HEAD)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_PAD << pItem->sDuration << pItem->bFlag;
				else if (i == LEFTEAR)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_HELMET << pItem->sDuration << pItem->bFlag;
				else if (i == NECK)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_GAUNTLET << pItem->sDuration << pItem->bFlag;
				else if (i == BREAST)
					pkt << (uint32)WARRIOR_DRAGON_ARMOR_BOOTS << pItem->sDuration << pItem->bFlag;
				else
					pkt << pItem->nNum << pItem->sDuration << pItem->bFlag;
			}
		}
		else
		{
			pItem = GetItem(equippedItems[i]);
			if (pItem == nullptr)
				continue;

			pkt << pItem->nNum << pItem->sDuration << pItem->bFlag;
		}
	}
	pkt << GetZoneID()

		<< uint8(-1)
		<< uint8(-1)
		<< uint32(0)
		<< m_bIsHidingWings
		<< m_bIsHidingHelmet
		<< m_bIsHidingCospre
		<< isGenieActive()
		<< GetRebirthLevel() /*// is reb exp 83+ thing << uint8(m_bLevel == 83) // is reb exp 83+ thing*/
		<< uint16(m_sCoverTitle)
		<< ReturnSymbolisOK // R symbol after name returned?
		<< uint32(0) // face time system
		<< uint16(0); //2364 new uint
}

void CUser::RightTopTitleMsg()
{
	foreach_stlmap(itr, g_pMain->m_RightTopTitleArray) {
		_RIGHT_TOP_TITLE_MSG* msg = itr->second;
		if (msg == nullptr) continue;

		Packet notice2(WIZ_NOTICE);
		notice2.DByte();
		notice2 << uint8(4) << uint8(1) << msg->strTitle << msg->strMessage;
		Send(&notice2);
	}
}

void CUser::RightTopTitleMsgDelete()
{
	foreach_stlmap(itr, g_pMain->m_RightTopTitleArray)
	{
		_RIGHT_TOP_TITLE_MSG* msg = itr->second;
		if (msg == nullptr)
			continue;

		Packet notice2(WIZ_NOTICE);
		notice2.DByte();
		notice2 << uint8(4) << uint8(2) << msg->strTitle << msg->strMessage;
		Send(&notice2);
	}
}