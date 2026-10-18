#include "stdafx.h"

static bool IsBotWarOrPkAttackTarget(Unit* pUnit)
{
	if (pUnit == nullptr)
		return false;

	C3DMap* pMap = pUnit->GetMap();
	if (pMap != nullptr && pMap->isWarZone())
		return true;

	if (pUnit->isPlayer())
		return TO_USER(pUnit)->isInPKZone();

	if (pUnit->isBot())
		return TO_BOT(pUnit)->isInPKZone();

	return false;
}
#include "CBot.h"
#include "Map.h"
#include "../shared/DateTime.h"
#include "DBAgent.h"
#include <random>   
#include <chrono>   
#include <algorithm>


// JstKO: Bot kesimlerinde de CZ/PK kill streak efekti ve PVP kill odulu calissin.
static void JstKO_SendPvpKillNotice(CUser* pUser)
{
	// JstKO: Eski yazili kill notice sistemi kapatildi.
	// Kill ekrani artik client tarafindaki re_killcount.uif + killnameall.dxt + killnamebackgall.dxt
	// dosyalarini kullanan WIZ_KILLASSIST paketi ile calisir.
	return;
}


static void JstKO_ApplyBotPvpKillReward(CUser* pUser, bool bPartyReward)
{
	if (pUser == nullptr || !pUser->isInGame() || pUser->isDead() || !pUser->isInPKZone())
		return;

	pUser->m_PvpKillCount++;
	pUser->m_PvpKill50Count++;
	pUser->m_PvpKill500Count++;

	JstKO_SendPvpKillNotice(pUser);

	const char* rewardTitle = bPartyReward ? "Pvp Party Kill odul" : "Pvp Kill odul";

	if (pUser->m_PvpKillCount == 5)
	{
		_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill�temGift);
		pUser->GoldGain(g_pMain->KillGoldGift);
		pUser->CashGain(g_pMain->KillCashGift);
		pUser->GiveWerehouseItem(g_pMain->Kill�temGift, g_pMain->KillCountGift);
		pUser->m_PvpKillCount = 0;
		// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
		// JstKO: kapatildi - ekstra player effect istemiyoruz.
		g_pMain->SendHelpDescription(pUser, string_format("[%s] = [ Para : %d ] | [ Cash : %d ] | [ Item Name : %s x%d ] | [ Kill Count : %d ]", rewardTitle, g_pMain->KillGoldGift, g_pMain->KillCashGift, pTable.m_sName.c_str(), g_pMain->KillCountGift, 5));
	}

	if (pUser->m_PvpKill50Count == 50)
	{
		_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill50�temGift);
		pUser->GoldGain(g_pMain->Kill50GoldGift);
		pUser->CashGain(g_pMain->Kill50CashGift);
		pUser->GiveWerehouseItem(g_pMain->Kill50�temGift, g_pMain->Kill50CountGift);
		pUser->m_PvpKillCount = 0;
		pUser->m_PvpKill50Count = 0;
		// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
		// JstKO: kapatildi - ekstra player effect istemiyoruz.
		g_pMain->SendHelpDescription(pUser, string_format("[%s 50] = [ Para : %d ] | [ Cash : %d ] | [ Item Name : %s x%d ] | [ Kill Count : %d ]", rewardTitle, g_pMain->Kill50GoldGift, g_pMain->Kill50CashGift, pTable.m_sName.c_str(), g_pMain->Kill50CountGift, 50));
	}

	if (pUser->m_PvpKill500Count == 500)
	{
		_ITEM_TABLE pTable = g_pMain->GetItemPtr(g_pMain->Kill500�temGift);
		pUser->GoldGain(g_pMain->Kill500GoldGift);
		pUser->CashGain(g_pMain->Kill500CashGift);
		pUser->GiveWerehouseItem(g_pMain->Kill500�temGift, g_pMain->Kill500CountGift);
		pUser->m_PvpKillCount = 0;
		pUser->m_PvpKill50Count = 0;
		pUser->m_PvpKill500Count = 0;
		// JstKO: kapatildi - odul confetti/skill efekti istemiyoruz, kill assist UI WIZ_KILLASSIST ile calisir.
		// JstKO: kapatildi - ekstra player effect istemiyoruz.
		g_pMain->SendHelpDescription(pUser, string_format("[%s 500] = [ Para : %d ] | [ Cash : %d ] | [ Item Name : %s x%d ] | [ Kill Count : %d ]", rewardTitle, g_pMain->Kill500GoldGift, g_pMain->Kill500CashGift, pTable.m_sName.c_str(), g_pMain->Kill500CountGift, 500));
	}
}

CBot::CBot() :Unit(UnitType::UnitBot)
{
	Initialize();
}

void CBot::Initialize()
{
	Unit::Initialize();
	ReplyChat = "";
	ReplyStatus = 0;
	ReplyTime = 0;
	ReplyID = 0;
	m_sTargetID = -1;
	m_TargetChanged = false;
	echo = -1;
	m_sRegionAttack = false;
	m_sMoveTime = m_sMoveRegionTime = 0;
	m_sCswAttackTime = 0;
	MerchantChat.clear();
	LastWarpTime = 0;
	m_tPartyInviteTime = 0;
	WalkStep = 0;
	m_sRegionAttackTime = m_sMoveRegionAttackTime = 0;
	m_strUserID.clear();
	m_Reverse = false;
	memset(&m_byAPClassBonusAmount, 0, sizeof(m_byAPClassBonusAmount));
	memset(&m_byAcClassBonusAmount, 0, sizeof(m_byAcClassBonusAmount));
	memset(&m_bStats, 0, sizeof(m_bStats));
	memset(&m_sStatItemBonuses, 0, sizeof(m_sStatItemBonuses));
	memset(&m_sStatAchieveBonuses, 0, sizeof(m_sStatAchieveBonuses));
	memset(&m_bStatBuffs, 0, sizeof(m_bStatBuffs));
	memset(&m_bRebStats, 0, sizeof(m_bRebStats));
	memset(&m_bstrSkill, 0, sizeof(m_bstrSkill));
	memset(&m_arMerchantItems, 0, sizeof(m_arMerchantItems));
	m_bPlayerAttackAmount = 100;
	m_sAchieveCoverTitle = 0;
	m_maxStop = 5000;
	LastMiningCheck = m_sMoveMerchantProcess = 0;
	m_fHPChangeTime = m_fHPType4CheckTime = m_fType4ChangeTime = getMSTime();

	m_bKnightsRank = -1;
	m_bPersonalRank = -1;
	m_bNPGainAmount = 100;

	m_bMerchantState = MERCHANT_STATE_NONE;
	m_sMerchantAreaType = 0;
	m_bMerchantIndex = 0;
	m_bAuthority = 1;
	m_sBind = -1;
	m_state = GameState::GAME_STATE_CONNECTED;
	m_bPartyLeader = false;
	m_bSlaveMerchant = false;
	m_bSlaveUserID = -1;
	//priest bot
	m_bUserPriestBotID = -1;
	memset(m_GenieOptions, 0, sizeof(m_GenieOptions));
	m_GenieTime = 0;
	m_sFirstUsingGenie = 0;

	m_bGenieStatus = 0;
	m_BotState = 0;
	m_bIsChicken = m_bWeaponsDisabled = false;
	m_bIsHidingHelmet = false;
	m_bIsHidingCospre = false;
	m_bIsHidingWings = false;

	m_bPremiumMerchant = false;
	m_bInParty = false;

	m_PlayerKillingLoyaltyDaily = 0;
	m_PlayerKillingLoyaltyPremiumBonus = 0;

	m_bInvisibilityType = INVIS_NONE;
	m_sDirection = ReturnSymbolisOK = 0;

	m_bAuthority = AUTHORITY_PLAYER;
	m_bLevel = m_MoveState = 1;
	m_iLoyalty = 100;
	m_iLoyaltyMonthly = m_sSkillCoolDown = 0;
	m_sHp = m_sMp = m_sSp = MAX_PLAYER_HP;

	m_sRivalID = -1;
	m_tRivalExpiryTime = 0;
	m_byAngerGauge = 0;

	m_bAddWeaponDamage = 0;
	m_bPctArmourAc = 100;
	m_sAddArmourAc = 0;
	m_sPartyIndex = 0;

	m_MaxHp = 0;
	m_MaxMp = 1;
	m_MaxSp = 250;
	m_bMerchantViewer = -1;
	m_bResHpType = USER_STANDING;
	m_bBlockPrivateChat = false;
	m_bNeedParty = 0x01;
	m_bAbnormalType = ABNORMAL_NORMAL;	// User starts out in normal size.
	m_nOldAbnormalType = m_bAbnormalType;
	m_teamColour = TeamColourNone;
	m_sSpeed = 0.0f;
}

void CBot::GetMerchantSlipList(_MERCH_DATA list[MAX_MERCH_ITEMS], CBot* pownermerch) {
	if (!pownermerch) return;
	for (int i = 0; i < MAX_MERCH_ITEMS; i++) {
		if (pownermerch->m_arMerchantItems[i].nNum) {
			for (int x = 0; x < MAX_MERCH_ITEMS; x++) {
				if (!list[x].nNum) { list[x] = pownermerch->m_arMerchantItems[i]; break; }
			}
		}
	}
}
void CBot::MerchantSlipRefList(CBot* pM, bool merchcrea) {
	if (!pM) return;

	if (!merchcrea) {
		Packet newpkt1(WIZ_MERCHANT_INOUT, uint8(1));
		newpkt1 << uint16(1) << pM->GetID() << pM->GetMerchantState() << (pM->GetMerchantState() == 1 ? false : pM->m_bPremiumMerchant);
		pM->SendToRegion(&newpkt1);
	}

	Packet newpkt2(WIZ_MERCHANT, uint8(MERCHANT_LIST));
	uint8 PremiumState = 0;
	newpkt2 << uint8(1) << pM->GetID() << pM->GetMerchantState() << PremiumState;
	_MERCH_DATA pnewlist[MAX_MERCH_ITEMS] = {};
	memset(pnewlist, 0, sizeof(pnewlist));
	GetMerchantSlipList(pnewlist, pM);
	for (int i = 0, x = PremiumState == 1 ? 8 : 4; i < x; i++) if (!pnewlist[i].IsSoldOut) newpkt2 << pnewlist[i].nNum;
	pM->SendToRegion(&newpkt2);

	for (int i = 0, x = PremiumState == 1 ? 8 : 4; i < x; i++) {
		if (!pnewlist[i].nNum) continue;
		Packet newpkt3(WIZ_MERCHANT, uint8(MERCHANT_ITEM_ADD));
		newpkt3 << uint16(1) << pnewlist[i].nNum << pnewlist[i].sCount << pnewlist[i].sDuration << pnewlist[i].nPrice << pnewlist[i].bOriginalSlot << uint8(i);
		pM->SendToRegion(&newpkt3);
	}
}

int CBot::FindSlotForItem(uint32 nItemID, uint16 sCount /*= 1*/)
{
	int result = -1;
	_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
	if (pTable.isnull())
		return result;

	_ITEM_DATA* pItem = nullptr;
	if (pTable.m_bCountable > 0) {
		for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++) {
			pItem = GetItem(i);
			if (pItem == nullptr)
				continue;

			if (pItem->nNum == nItemID && pItem->sCount + sCount <= ITEMCOUNT_MAX)
				return i;

			// Found a free slot, we'd prefer to stack it though
			// so store the first free slot, and ignore it.
			if (pItem->nNum == 0 && result < 0)
				result = i;
		}
		// If we didn't find a slot countaining our stackable item, it's possible we found
		// an empty slot. So return that (or -1 if it none was found; no point searching again).
		return result;
	}

	// If it's not stackable, don't need any additional logic.
	// Just find the first free slot.
	return GetEmptySlot();
}

int CBot::GetEmptySlot()
{
	_ITEM_DATA* pItem = nullptr;
	for (int i = SLOT_MAX; i < SLOT_MAX + HAVE_MAX; i++)
	{
		pItem = GetItem(i);
		if (pItem == nullptr)
			continue;

		if (pItem->nNum == 0)
			return i;
	}
	return -1;
}

void CBot::SetZoneAbilityChange(uint16 sNewZone)
{
	C3DMap* pMap = g_pMain->GetZoneByID(sNewZone);
	CKingSystem* pKingSystem = g_pMain->m_KingSystemArray.GetData(GetNation());

	if (pMap == nullptr)
		return;

	switch (sNewZone)
	{
	case ZONE_KARUS:
	case ZONE_KARUS2:
	case ZONE_KARUS3:
	case ZONE_ELMORAD:
	case ZONE_ELMORAD2:
	case ZONE_ELMORAD3:
	case ZONE_KARUS_ESLANT:
	case ZONE_KARUS_ESLANT2:
	case ZONE_KARUS_ESLANT3:
	case ZONE_ELMORAD_ESLANT:
	case ZONE_ELMORAD_ESLANT2:
	case ZONE_ELMORAD_ESLANT3:
	case ZONE_BIFROST:
	case ZONE_BATTLE:
	case ZONE_BATTLE2:
	case ZONE_BATTLE3:
	case ZONE_BATTLE4:
	case ZONE_BATTLE5:
	case ZONE_BATTLE6:
	case ZONE_SNOW_BATTLE:
	case ZONE_RONARK_LAND:
	case ZONE_ARDREAM:
	case ZONE_RONARK_LAND_BASE:
	case ZONE_KROWAZ_DOMINION:
	case ZONE_STONE1:
	case ZONE_STONE2:
	case ZONE_STONE3:
	case ZONE_BORDER_DEFENSE_WAR:
	case ZONE_UNDER_CASTLE:
	case ZONE_JURAID_MOUNTAIN:
	case ZONE_PARTY_VS_1:
	case ZONE_PARTY_VS_2:
	case ZONE_PARTY_VS_3:
	case ZONE_PARTY_VS_4:
	case ZONE_CLAN_WAR_ARDREAM:
	case ZONE_CLAN_WAR_RONARK:
	case ZONE_KNIGHT_ROYALE:
	case ZONE_CHAOS_DUNGEON:
		if (pKingSystem != nullptr)
			pMap->SetTariff(10 + pKingSystem->m_nTerritoryTariff);
		else
			pMap->SetTariff(10);
		break;
	case ZONE_MORADON:
	case ZONE_MORADON2:
	case ZONE_MORADON3:
	case ZONE_MORADON4:
	case ZONE_MORADON5:
	case ZONE_ARENA:
		pMap->SetTariff((uint8)g_pMain->pSiegeWar.sMoradonTariff);
		break;
	case ZONE_DELOS:
	case ZONE_DESPERATION_ABYSS:
	case ZONE_HELL_ABYSS:
	case ZONE_DELOS_CASTELLAN:
		pMap->SetTariff((uint8)g_pMain->pSiegeWar.sDellosTariff);
		break;
	default:
		//printf("King and Deos Tariff unhandled zone %d \n", sNewZone);
		TRACE("King and Deos Tariff unhandled zone %d \n", sNewZone);
		break;
	}

}

void CBot::GetInOut(Packet& result, uint8 bType)
{
	result.Initialize(WIZ_USER_INOUT);
	result << bType << uint8(0) << GetID();
	if (bType != INOUT_OUT)
		GetUserInfo(result);
}

// Fonksiyonu: Bot bir bölgeye eklendiğinde bölge notice gösterimi eklenmiştir.
void CBot::AddToRegion(int16 new_region_x, int16 new_region_z)
{
	// Eski bölgeden kaldır
	if (GetRegion())
		GetRegion()->Remove(this);

	// Yeni bölgeyi ayarla
	SetRegion(new_region_x, new_region_z);

	// Yeni bölgeye ekle
	if (GetRegion())
		GetRegion()->Add(this);

	// ***** Bölge Notice Ekleniyor *****
	// Harita pointer
	C3DMap* pMap = g_pMain->GetZoneByID(GetZoneID());
	if (!pMap)
		return;

	// Notice metni: Bot ismi ve yeni bölge koordinatlarını gösterir
	std::string notice = string_format("%s Adlı Bot (%d, %d) Bölgesine Eklendi.", GetName(), new_region_x, new_region_z);

	// Sadece yeni bölgedeki oyunculara gönder
	//pMap->NoticeToRegion(notice.c_str());
}

void CBot::BotInOut(uint8 bType)
{
	if (GetRegion() == nullptr)
		return;

	Packet result;
	GetInOut(result, bType);

	if (bType == INOUT_OUT)
	{
		if (GetRegion())
			GetRegion()->Remove(this);
	}
	else
	{
		GetRegion()->Add(this);
	}

	SendToRegion(&result);
}

void CBot::UserInOut(uint8 bType)
{
	// Oyun dışına çıkma durumunda gerekli kontrolleri yap
	if (bType == INOUT_OUT)
	{
		if (!isInGame() || GetRegion() == nullptr || g_pMain->GetZoneByID(GetZoneID()) == nullptr)
		{
			TRACE("UserInOut: Not in game or region/zone is nullptr.\n");
			return;
		}
	}

	Packet result;
	GetInOut(result, bType);

	if (bType == INOUT_OUT)
	{
		// Oyuncu oyun dışına çıkıyor
		C3DMap* pMap = g_pMain->GetZoneByID(GetZoneID());
		if (pMap == nullptr)
		{
			TRACE("UserInOut: Invalid zone ID.\n");
			return;
		}

		// Belirli haritalarda özel işlemler yapılıyor
		if (pMap->GetID() == ZONE_ARDREAM ||
			pMap->GetID() == ZONE_RONARK_LAND ||
			pMap->GetID() == ZONE_BORDER_DEFENSE_WAR)
		{
			// Loyalty sıfırlama ve oyuncu öldürme bölgesi sıralamasından silme
			m_iLoyaltyDaily = 0;
			m_iLoyaltyPremiumBonus = 0;

			for (int nation = KARUS_ARRAY; nation <= ELMORAD_ARRAY; nation++)
			{
				if (g_pMain->m_UserPlayerKillingZoneRankingArray[nation].GetSize() > 0)
				{
					g_pMain->m_UserPlayerKillingZoneRankingArray[nation].DeleteData(GetID());
				}
			}
		}
		else if (pMap->GetID() == ZONE_CHAOS_DUNGEON)
		{
			// Kaos genişleme sıralamasından silme
			if (g_pMain->m_UserChaosExpansionRankingArray.GetSize() > 0)
			{
				g_pMain->m_UserChaosExpansionRankingArray.DeleteData(GetID());
			}
		}

		TRACE("%s Bot has left the game. Removing from Player Ranked ZoneID %d\n", GetName().c_str(), GetZoneID());

		// Bölgedeki botları güncelle
		BotsSurroundingUserRegionUpdate();

		// Bölgeden kaldırma işlemi
		if (GetRegion())
		{
			GetRegion()->Remove(this);
		}

		// Oyun durumu güncelleme
		m_state = GameState::GAME_STATE_CONNECTED;

		// Veritabanından kullanıcıyı kaldırma
		g_DBAgent.RemoveCurrentUser(GetName());

		// Bot kullanıcısını güncelleme
		//g_DBAgent.UpdateBotUser(this);
	}
	else
	{
		// Oyuncu oyun içine giriyor
		if (GetRegion())
		{
			GetRegion()->Add(this);
		}

		// Oyun durumu güncelleme
		m_state = GameState::GAME_STATE_INGAME;

		// Veritabanına kullanıcı ekleme
		g_DBAgent.InsertCurrentUser(GetName(), GetName());

		// Bot kullanıcısını yükleme
		//g_DBAgent.GetLoadBotUser(this);

		// Oyun başlangıç zamanını kaydetme
		m_tGameStartTimeSavedMagic = UNIXTIME;
	}

	// Sonucu bölgeye gönderme
	SendToRegion(&result);
}

void CBot::BotsSurroundingUserRegionUpdate()
{
	Packet result(WIZ_USER_INFORMATIN, uint8(UserInfoShow));
	result.SByte();
	result << GetName();
	g_pMain->Send_Zone(&result, GetZoneID());
}
#if 0
void CBot::GetUserInfo(Packet& pkt)
{
	pkt.SByte();
	pkt << GetName() << GetNation();
	pkt << uint8(0) << uint8(0);
	pkt << GetClanID() << uint8(isInPKZone() == true ? uint8(0) : GetFame());

	CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
	if (pKnights == nullptr)
	{
		if (isKing())
			pkt << uint32(0) << uint16(0) << uint8(0) << uint16(GetNation() == (uint8)Nation::KARUS ? 97 : 98) << uint32(0) << uint8(0);
		else
			pkt << uint32(0) << uint16(0) << uint8(0) << uint16(-1) << uint32(0) << uint8(0);
	}
	else
	{
		pkt << pKnights->GetAllianceID() << pKnights->GetName() << pKnights->m_byGrade << pKnights->m_byRanking << pKnights->m_sMarkVersion; // symbol/mark version

		CKnights* pMainClan = g_pMain->GetClanPtr(pKnights->GetAllianceID());
		_KNIGHTS_ALLIANCE* pAlliance = g_pMain->GetAlliancePtr(pKnights->GetAllianceID());

		if (pKnights->isInAlliance() && pMainClan != nullptr && pAlliance != nullptr)
		{
			if (!isKing())
			{
				if (pAlliance->sMainAllianceKnights == pKnights->GetID())
					pkt << pMainClan->GetCapeID() << pMainClan->m_bCapeR << pMainClan->m_bCapeG << pMainClan->m_bCapeB << uint8(0);
				else if (pAlliance->sSubAllianceKnights == pKnights->GetID())
					pkt << pMainClan->GetCapeID() << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB << uint8(0);
				else if (pAlliance->sMercenaryClan_1 == pKnights->GetID() || pAlliance->sMercenaryClan_2 == pKnights->GetID())
					pkt << pMainClan->GetCapeID() << uint32(0); // only the cape will be present
				else
					pkt << pMainClan->GetCapeID() << pMainClan->m_bCapeR << pMainClan->m_bCapeG << pMainClan->m_bCapeB << uint8(0); // this is stored in 4 bytes after all.
			}
			else
				pkt << uint16(GetNation() == (uint8)Nation::KARUS ? 97 : 98) << uint32(0); // cape ID

			// not sure what this is, but it (just?) enables the clan symbol on the cape 
			// value in dump was 9, but everything tested seems to behave as equally well...
			// we'll probably have to implement logic to respect requirements.
			pkt << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
		}
		else
		{
			if (!isKing())
				pkt << pKnights->GetCapeID() << pKnights->m_bCapeR << pKnights->m_bCapeG << pKnights->m_bCapeB << uint8(0); // this is stored in 4 bytes after all.
			else
				pkt << uint16(GetNation() == (uint8)Nation::KARUS ? 97 : 98) << uint32(0); // cape ID

			// not sure what this is, but it (just?) enables the clan symbol on the cape 
			// value in dump was 9, but everything tested seems to behave as equally well...
			// we'll probably have to implement logic to respect requirements.
			pkt << ((pKnights->m_byFlag > 1 && pKnights->m_byGrade < 3) ? uint8(1) : uint8(0));
		}
	}

	// There are two event-driven invisibility states; dispel on attack, and dispel on move.
	// These are handled primarily server-side; from memory the client only cares about value 1 (which we class as 'dispel on move').
	// As this is the only place where this flag is actually sent to the client, we'll just convert 'dispel on attack' 
	// back to 'dispel on move' as the client expects.
	uint8 bInvisibilityType = m_bInvisibilityType;
	if (bInvisibilityType != (uint8)InvisibilityType::INVIS_NONE)
		bInvisibilityType = (uint8)InvisibilityType::INVIS_DISPEL_ON_MOVE;

	pkt << GetLevel() << m_bRace << m_sClass
		<< GetSPosX() << GetSPosZ() << GetSPosY()
		<< m_bFace << m_nHair
		<< m_bResHpType << uint32(m_bAbnormalType)
		<< m_bNeedParty
		<< m_bAuthority
		<< m_bPartyLeader
		<< bInvisibilityType
		<< uint8(m_teamColour)
		<< m_bIsHidingHelmet
		<< m_bIsHidingCospre
		<< m_sDirection
		<< m_bIsDevil
		<< m_bIsHidingWings
		<< m_bIsChicken
		<< m_bRank
		<< (m_bKnightsRank <= m_bPersonalRank ? m_bKnightsRank : int8(-1))
		<< (m_bPersonalRank <= m_bKnightsRank ? m_bPersonalRank : int8(-1));

	uint8 equippedItems[] =
	{
		BREAST, LEG, HEAD, GLOVE, FOOT, SHOULDER, RIGHTHAND, LEFTHAND,
		CWING, CHELMET, CLEFT, CRIGHT, CTOP, CFAIRY, CTATTOO
	};

	bool isRoyaleSignEvent = (GetZoneID() == ZONE_KNIGHT_ROYALE);
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
		<< m_bGenieStatus
		<< GetRebLevel() /*// is reb exp 83+ thing << uint8(m_bLevel == 83) // is reb exp 83+ thing*/
		<< uint16(m_sAchieveCoverTitle)
		<< uint32(0)//ReturnSymbolisOK // R symbol after name returned?
		<< uint32(0) // face time system
		<< uint16(0); // New Packet
}
#else
void CBot::GetUserInfo(Packet& pkt)
{
	pkt.SByte();
	pkt << GetName() << GetNation() << uint8(0) << uint8(0);
	pkt << GetClanID() << uint8(isInPKZone() == true ? uint8(0) : GetFame()); // -> [uint8(isInPKZone() == true ? uint8(0) : GetFame()] <- soldaki kod pk bölgelerinde clan baskanı normal clan uyesi gibi gözükür

	CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
	if (pKnights == nullptr)
	{
		if (isKing())
			//pkt << uint32(0) << uint16(0) << uint8(0) << uint16(KNIGHTS_KNIG_CAPE) << uint32(0) << uint8(0);//King White Cape 20.05.2020	
			pkt << uint32(0) << uint16(0) << uint8(0) << uint16(GetNation() == (uint8)Nation::KARUS ? 97 : 98) << uint32(0) << uint8(0); //King New Cape 20.05.2020	
		else
			//pkt << uint32(0) << uint16(0) << uint8(0) << uint16(GetNation() == KARUS ? 97 : 98) << uint32(0) << uint8(0); //Userlerin ırk pelerini göstermesi kaldırıldı
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

		pkt << m_bKnightsRank;
		pkt << m_bPersonalRank;

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
		<< m_bGenieStatus
		<< GetRebLevel() /*// is reb exp 83+ thing << uint8(m_bLevel == 83) // is reb exp 83+ thing*/
		<< uint16(m_sAchieveCoverTitle)
		<< ReturnSymbolisOK // R symbol after name returned?
		<< uint32(0) // face time system
		<< uint16(0); //2364 new uint
}
#endif
bool CBot::JobGroupCheck(short jobgroupid)
{
	if (jobgroupid > 100)
		return GetClass() == jobgroupid;

	ClassType subClass = GetBaseClassType();
	switch (jobgroupid)
	{
	case GROUP_WARRIOR: return (subClass == ClassWarrior || subClass == ClassWarriorNovice || subClass == ClassWarriorMaster);
	case GROUP_PORTU_KURIAN: return (subClass == ClassPortuKurian || subClass == ClassPortuKurianNovice || subClass == ClassPortuKurianMaster);
	case GROUP_ROGUE: return (subClass == ClassRogue || subClass == ClassRogueNovice || subClass == ClassRogueMaster);
	case GROUP_MAGE: return (subClass == ClassMage || subClass == ClassMageNovice || subClass == ClassMageMaster);
	case GROUP_CLERIC: return (subClass == ClassPriest || subClass == ClassPriestNovice || subClass == ClassPriestMaster);
	}
	return (subClass == jobgroupid);
}
#if 0
uint16 CGameServerDlg::SpawnPriestBot(int Minute, CUser* pUser)
{
	foreach_stlmap(itr, m_MapBotList)
	{
		CBot* pPriest = itr->second;

		if (pPriest->m_state == GameState::GAME_STATE_INGAME)
			continue;

		if (pPriest->GetNation() != pUser->GetNation())
			continue;

		std::string userid = "priest";




		pPriest->Initialize();
		pPriest->LastWarpTime = UNIXTIME + (Minute);
		pPriest->m_pMap = GetZoneByID(pUser->GetZoneID());
		pPriest->m_bZone = pUser->GetZoneID();
		pPriest->m_sClass = 212;//212
		pPriest->m_bRace = 13;
		pPriest->m_curx = pUser->m_curx;
		pPriest->m_curz = pUser->m_curz;
		pPriest->m_cury = pUser->m_cury;
		pPriest->m_oldx = pUser->m_oldx;
		pPriest->m_oldy = pUser->m_oldy;
		pPriest->m_oldz = pUser->m_oldz;
		pPriest->m_nHair = 53890920;
		pPriest->m_bFace = 4;
		pPriest->m_bFame = 0;
		pPriest->m_bLevel = 83;

		memset(pPriest->m_sItemArray, 0x00, sizeof(pPriest->m_sItemArray));
		memset(pPriest->m_arMerchantItems, 0x00, sizeof(pPriest->m_arMerchantItems));




		pPriest->m_sItemArray[0].nNum = 910156114;
		pPriest->m_sItemArray[1].nNum = 926007598;
		pPriest->m_sItemArray[2].nNum = 910156114;
		pPriest->m_sItemArray[3].nNum = 910156114;
		pPriest->m_sItemArray[4].nNum = 926005598;
		pPriest->m_sItemArray[5].nNum = 610001000;
		pPriest->m_sItemArray[6].nNum = 140210168;
		pPriest->m_sItemArray[7].nNum = 910160118;
		pPriest->m_sItemArray[8].nNum = 171510008;
		pPriest->m_sItemArray[9].nNum = 910162272;
		pPriest->m_sItemArray[10].nNum = 926006598;
		pPriest->m_sItemArray[11].nNum = 910162272;
		pPriest->m_sItemArray[12].nNum = 926008598;
		pPriest->m_sItemArray[13].nNum = 926009598;

		for (int i = 0; i < 14; i++)
			pPriest->m_sItemArray[i].sCount = 1;

		for (int i = 0; i < 14; i++)
			pPriest->m_sItemArray[i].nSerialNum = GenerateItemSerial();

		for (int i = 0; i < 14; i++)
			pPriest->m_sItemArray[i].sDuration = 1;



		pPriest->m_reblvl = pUser->m_bRebirthLevel;


		pPriest->m_bstrSkill[0] = 0;
		pPriest->m_bstrSkill[1] = 0;
		pPriest->m_bstrSkill[2] = 0;
		pPriest->m_bstrSkill[3] = 0;
		pPriest->m_bstrSkill[4] = 0;
		pPriest->m_bstrSkill[5] = 68;
		pPriest->m_bstrSkill[6] = 80;
		pPriest->m_bstrSkill[7] = 0;
		pPriest->m_bstrSkill[8] = 0;
		pPriest->m_bstrSkill[9] = 0;

		pPriest->m_iGold = pUser->m_iGold;
		pPriest->m_sPoints = pUser->m_sPoints;

		pPriest->m_bStats[0] = 167;
		pPriest->m_bStats[1] = 60;
		pPriest->m_bStats[2] = 60;
		pPriest->m_bStats[3] = 255;
		pPriest->m_bStats[4] = 50;

		pPriest->m_strUserID = "[" + pUser->GetName() + "'s Priest]";
		pPriest->m_bKnightsRank = pUser->m_bKnightsRank;
		pPriest->m_bPersonalRank = pUser->m_bPersonalRank;
		pPriest->m_sAchieveCoverTitle = pUser->m_sCoverTitle;
		pPriest->ReturnSymbolisOK = pUser->ReturnSymbolisOK;
		pPriest->m_bResHpType = USER_STANDING;

		pPriest->SetPosition(pUser->GetX(), pUser->GetY(), pUser->GetZ());
		pPriest->SetRegion(pUser->GetNewRegionX(), pUser->GetNewRegionZ());

		pPriest->m_bUserPriestBotID = pUser->GetSocketID();

		pUser->m_bUserPriestBotID = pPriest->GetID();

		pUser->hasPriestBot = true;

		if (pUser->GetNation() == KARUS)
			pPriest->m_teamColour = TeamColourBlue;
		else
			pPriest->m_teamColour = TeamColourRed;

		g_DBAgent.LoadPriestBotGenieData(userid, pUser);

		pPriest->SetMaxHp(1);
		pPriest->SetMaxMp();
		pPriest->UserInOut(INOUT_IN);

		//pPriest->GenieStart();

		return pPriest->GetID();
	}//
	return 0;
}
#else
/*test*/
uint16 CGameServerDlg::SpawnPriestBot(int Minute, CUser* pUser)
{
	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (pUser->GetZoneID() <= ZONE_ELMORAD && pUser->GetZoneID() != pBotInfo->m_bNation
			|| (pUser->GetZoneID() >= ZONE_KARUS_ESLANT
				&& pUser->GetZoneID() <= ZONE_ELMORAD_ESLANT
				&& pUser->GetZoneID() != (pBotInfo->m_bNation + 10)))
			continue;

		/*if (pBotInfo->m_bLevel < pUser->GetLevel())
			continue;*/

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = "[" + pUser->GetName() + "'s Priest]";//"[Slave]" + pUser->GetName();
		pBot->m_bNation = pUser->m_bNation;
		pBot->m_bRace = 13;//pUser->m_bRace;
		pBot->m_sClass = 212;//pUser->m_sClass;

		pBot->m_curx = pUser->m_curx;
		pBot->m_curz = pUser->m_curz;
		pBot->m_cury = pUser->m_cury;
		pBot->m_oldx = pUser->m_oldx;
		pBot->m_oldy = pUser->m_oldy;
		pBot->m_oldz = pUser->m_oldz;
		/*new start*/
		pBot->m_nHair = 53890920;//pUser->m_nHair;
		pBot->m_bLevel = 83;//pUser->m_bLevel;
		pBot->m_bFace = 4;//pUser->m_bFace;
		pBot->m_bKnights = pUser->m_bKnights;
		pBot->m_bFame = 0;//pUser->m_bFame;
		pBot->m_bKnightsRank = pUser->m_bKnightsRank;
		pBot->m_bPersonalRank = pUser->m_bPersonalRank;
		/*new end*/
		memcpy(pBot->m_sItemArray, pUser->m_sItemArray, sizeof(pBot->m_sItemArray));
		memset(pBot->m_arMerchantItems, 0x00, sizeof(pBot->m_arMerchantItems));

		/*new start*/
		pBot->m_sItemArray[0].nNum = 910156114;
		pBot->m_sItemArray[1].nNum = 926007598;
		pBot->m_sItemArray[2].nNum = 910156114;
		pBot->m_sItemArray[3].nNum = 910156114;
		pBot->m_sItemArray[4].nNum = 926005598;
		pBot->m_sItemArray[5].nNum = 610001000;
		pBot->m_sItemArray[6].nNum = 140210168;
		pBot->m_sItemArray[7].nNum = 910160118;
		pBot->m_sItemArray[8].nNum = 171510008;
		pBot->m_sItemArray[9].nNum = 910162272;
		pBot->m_sItemArray[10].nNum = 926006598;
		pBot->m_sItemArray[11].nNum = 910162272;
		pBot->m_sItemArray[12].nNum = 926008598;
		pBot->m_sItemArray[13].nNum = 926009598;

		for (int i = 0; i < 14; i++)
			pBot->m_sItemArray[i].sCount = 1;

		for (int i = 0; i < 14; i++)
			pBot->m_sItemArray[i].nSerialNum = GenerateItemSerial();

		for (int i = 0; i < 14; i++)
			pBot->m_sItemArray[i].sDuration = 1;

		pBot->m_bstrSkill[0] = 0;
		pBot->m_bstrSkill[1] = 0;
		pBot->m_bstrSkill[2] = 0;
		pBot->m_bstrSkill[3] = 0;
		pBot->m_bstrSkill[4] = 0;
		pBot->m_bstrSkill[5] = 68;
		pBot->m_bstrSkill[6] = 80;
		pBot->m_bstrSkill[7] = 0;
		pBot->m_bstrSkill[8] = 0;
		pBot->m_bstrSkill[9] = 0;

		pBot->m_bStats[0] = 167;
		pBot->m_bStats[1] = 60;
		pBot->m_bStats[2] = 60;
		pBot->m_bStats[3] = 255;
		pBot->m_bStats[4] = 50;

		pBot->ReturnSymbolisOK = pUser->ReturnSymbolisOK;
		/*new end*/

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pUser->m_sCoverTitle;
		pBot->m_reblvl = pUser->m_bRebirthLevel;
		pBot->m_iGold = pUser->m_iGold;
		pBot->m_sPoints = pUser->m_sPoints;
		pBot->m_iLoyalty = pUser->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pUser->m_iLoyaltyMonthly;

		int Random = myrand(0, 10000);
		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute);//UNIXTIME + (Minute * 60);
		pBot->m_pMap = GetZoneByID(pUser->GetZoneID());
		pBot->m_bZone = pUser->GetZoneID();
		pBot->m_sDirection = pUser->m_sDirection;
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->SetBotAbility();
		pBot->m_BotState = BOT_MERCHANT;
		pBot->SetPosition(pUser->GetX(), pUser->GetY(), pUser->GetZ());
		//pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		pBot->SetRegion(pUser->GetNewRegionX(), pUser->GetNewRegionZ()); //new

		/*new start*/
		pBot->m_bUserPriestBotID = pUser->GetSocketID();

		pUser->m_bUserPriestBotID = pBot->GetID();

		pUser->hasPriestBot = true;

		if (pUser->GetNation() == KARUS)
			pBot->m_teamColour = TeamColourBlue;
		else
			pBot->m_teamColour = TeamColourRed;

		g_DBAgent.LoadPriestBotGenieData(std::string("priest"), pUser);
		/*new end*/

		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		pBot->StateChangeServerDirect(1, Random > 5000 ? USER_STANDING : USER_SITDOWN);
		return pBot->GetID();
	}
	return 0;
}
/*test*/
#endif

void CBot::ShowEffect(uint32 nSkillID)
{
	Packet result(WIZ_EFFECT);
	result << GetID() << nSkillID;
	SendToRegion(&result);
}

void CBot::SendMoveResult(float fX, float fY, float fZ, uint8 echo /*= 0.0f*/, int16 speed, uint16 socketid)
{
	CBot* pPriest = nullptr;
	pPriest = g_pMain->m_MapBotList.GetData(GetID());

	SetPosition(fX, fY, fZ);
	RegisterRegion();

	Packet result(WIZ_MOVE);
	result << socketid << GetSPosX() << GetSPosZ() << GetSPosY() << speed << echo;

	if (GetEventRoom() > 0)
		SendToRegion(&result, nullptr, GetEventRoom());
	else
		SendToRegion(&result);
		
}

#pragma region CUser::GenieStart()
void CBot::GenieStart()
{

	Packet result(WIZ_GENIE, uint8(GenieInfoRequest));
	result << uint8(GenieActivated) << uint16(GetID()) << uint8(1);

	m_bGenieStatus = true;
	if (GetEventRoom() > 0)
		SendToRegion(&result, nullptr, GetEventRoom());
	else
		SendToRegion(&result);


}
#pragma endregion

#pragma region CUser::SendGenieStart(bool isToRegion /* = false */)
void CBot::SendGenieStart(bool isToRegion /* = false */)
{
	Packet result(WIZ_GENIE, uint8(GenieInfoRequest));
	result << uint8(GenieStartHandle) << uint16(1) << uint16(3600);
	SendToRegion(&result);

	Packet result2(WIZ_GENIE, uint8(GenieInfoRequest));
	result2 << uint8(GenieActivated) << uint16(GetID()) << uint8(1);

	SendToRegion(&result2);

}
#pragma endregion


void CBot::RecastSavedMagic(uint8 buffType /* = 0*/)
{
	Guard lock(m_savedMagicLock);
	BotSavedMagicMap castSet;
	foreach(itr, m_savedMagicMap)
	{
		if (itr->first != 0 || itr->second != 0)
			castSet.insert(std::make_pair(itr->first, itr->second));
	}

	if (castSet.empty())
		return;

	foreach(itr, castSet)
	{
		if (buffType > 0)
		{
			_MAGIC_TYPE4* pType = g_pMain->m_Magictype4Array.GetData(itr->first);

			if (pType == nullptr)
				continue;

			if (pType->bBuffType != buffType)
				continue;
		}

		if (isSiegeTransformation())
			continue;

		MagicInstance instance;
		instance.sCasterID = GetID();
		instance.sTargetID = GetID();
		instance.nSkillID = itr->first;
		instance.sSkillCasterZoneID = GetZoneID();
		instance.bIsRecastingSavedMagic = true;
		instance.Run();
	}
}

void CBot::CheckSavedMagic()
{
	Guard lock(m_savedMagicLock);
	if (m_savedMagicMap.empty())
		return;

	std::set<uint32> deleteSet;
	foreach(itr, m_savedMagicMap)
	{
		if (itr->second <= UNIXTIME2)
			deleteSet.insert(itr->first);
	}

	foreach(itr, deleteSet)
		m_savedMagicMap.erase(*itr);
}

void CBot::Type4Change()
{
	m_fType4ChangeTime = getMSTime();

	if (isDead())
		return;

	//MagicPacket(MAGIC_EFFECTING, 500048, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ()); //HP 1000 sc
	MagicPacket(MAGIC_EFFECTING, 500049, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ()); //Weapon Enchant Scroll 
	MagicPacket(MAGIC_EFFECTING, 500050, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ()); //Armor Enchant
	MagicPacket(MAGIC_EFFECTING, 500054, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ()); //Undying
	MagicPacket(MAGIC_EFFECTING, 492018, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ()); //400 AC
	MagicPacket(MAGIC_EFFECTING, 491014, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ()); //SW SC

	if (isRogue())
		MagicPacket(MAGIC_EFFECTING, 500042, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ()); //Scroll of Dexterity(Power up
	else if (isMage())
		MagicPacket(MAGIC_EFFECTING, 500040, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());// str sc
	else if (isPriest())
	{
		MagicPacket(MAGIC_EFFECTING, 500040, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());//str sc
		MagicPacket(MAGIC_EFFECTING, 212020, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());//priest kitap
	}
	else
		MagicPacket(MAGIC_EFFECTING, 500040, GetID(), GetID(), (uint16)GetX(), (uint16)GetY(), (uint16)GetZ());//str sc
}

void CBot::BotMining()
{
	if (LastMiningCheck + (2 * MINUTE) > UNIXTIME)
		return;

	LastMiningCheck = UNIXTIME;

	Packet result(WIZ_MINING, uint8(MiningAttempt));
	uint16 resultCode = MiningResultSuccess, Random = myrand(0, 10000);
	uint16 sEffect = 0;

	if (Random > 4000)
		sEffect = 13082; // "XP" effect
	else
		sEffect = 13081; // "Item" effect

	result << resultCode << GetID() << sEffect;
	SendToRegion(&result);
}

void CBot::BotFishing()
{
	if (LastMiningCheck + (2 * MINUTE) > UNIXTIME)
		return;

	LastMiningCheck = UNIXTIME;

	Packet result(WIZ_MINING, uint8(FishingAttempt));
	uint16 resultCode = MiningResultSuccess, Random = myrand(0, 10000);
	uint16 sEffect = 0;

	if (Random > 4000)
		sEffect = 13082; // "XP" effect
	else
		sEffect = 13081; // "Item" effect

	result << resultCode << GetID() << sEffect;
	SendToRegion(&result);
}

void CBot::BotMerchant()
{
	if (LastMiningCheck + (1 * MINUTE) > UNIXTIME)
		return;

	LastMiningCheck = UNIXTIME;

	if (MerchantChat.empty())
		return;

	Packet result(WIZ_CHAT);
	ChatPacket::Construct(&result, MERCHANT_CHAT, &MerchantChat, &GetName(), GetNation(), GetID());
	SendToRegion(&result);
}

void CBot::FindMonsterAttackSlot()
{
	if (isDead())
		return;

	CNpc* pNpc = nullptr;
	KOMap* pMap = GetMap();
	if (pMap == nullptr)
		return;

	foreach_region(rx, rz)
		FindMonsterAttack(rx + GetRegionX(), rz + GetRegionZ(), pMap);
}

void CBot::FindMonsterAttack(int x, int z, C3DMap* pMap)
{
	if (m_sRegionAttackTime > UNIXTIME2) 
		return;

	std::vector<Unit*> casted_member;
	std::vector<uint16> unitList;
	g_pMain->GetUnitListFromSurroundingRegions(this, &unitList);

	foreach(itr, unitList)
	{
		Unit* pTarget = g_pMain->GetUnitPtr(*itr, GetZoneID());

		if (pTarget == nullptr)
			continue;

		if (this != pTarget
			&& !pTarget->isDead()
			&& !pTarget->isBlinking()
			&& pTarget->isAttackable())
			casted_member.push_back(pTarget);
	}

	if (pMap == nullptr)
		return;

	int iValue = 0;
	float UnitX, UnitY, UnitZ;

	CRegion* pRegion = pMap->GetRegion(x, z);
	if (pRegion == nullptr)
		return;

	int16 sTargetID = -1;
	std::vector<uint16> willDel;
	__Vector3 vBot, vpUnit, vDistance, vRealDistance;

	foreach(itr, casted_member)
	{
		Unit* pTarget = *itr; // it's checked above, not much need to check it again
		float fSearchRange = 18.0f;

		if (pTarget == nullptr)
			continue;

		if (pTarget->isDead()
			|| pTarget->isPlayer() && TO_USER(pTarget)->GetNation() == GetNation()
			|| pTarget->isPlayer() && !IsBotWarOrPkAttackTarget(pTarget)
			|| pTarget->GetZoneID() != GetZoneID()
			|| pTarget->isNPC() && !TO_NPC(pTarget)->isMonster()
			|| pTarget->isPlayer() && TO_USER(pTarget)->isGM()
			|| pTarget->isBot() && TO_BOT(pTarget)->GetNation() == GetNation()
			|| pTarget->isBot() && !IsBotWarOrPkAttackTarget(pTarget))
			continue;

		float fDis = GetDistanceSqrt(pTarget);
		if (fDis > fSearchRange)
			continue;

		if (sTargetID > -1)
			continue;

		UnitX = pTarget->GetX();
		UnitZ = pTarget->GetZ();
		UnitY = pTarget->GetY();

		vBot.Set(GetX(), pTarget->GetY(), GetZ());
		vpUnit.Set(pTarget->GetX() + ((myrand(0, 2000) - 1000.0f) / 500.0f), pTarget->GetY(), pTarget->GetZ() + ((myrand(0, 2000) - 1000.0f) / 500.0f));
		sTargetID = pTarget->GetID();
	}

	if (sTargetID == int16(-1))
	{
		m_sRegionAttackTime = ULONGLONG(UNIXTIME2 + (5 * SECOND));
		return;
	}

	if (m_sTargetID != sTargetID)
	{
		m_TargetChanged = true;
		m_sTargetID = sTargetID;
		m_oldx = vpUnit.x + ((myrand(0, 2000) - 1000.0f) / 500.0f);
		m_oldz = vpUnit.z + ((myrand(0, 2000) - 1000.0f) / 500.0f);
		m_oldy = vpUnit.y;
	}

	vDistance = vpUnit - vBot;
	vRealDistance = vDistance;
	vDistance.Normalize();

	float sSpeed = m_sSpeed;
	bool sRunFinish = false;
	vDistance *= sSpeed / 10.0f;

	uint16 sRunTime = 1000;
	if (isWarrior() || isRogue())
		sRunTime = 700;

	if (echo == uint8(0)
		&& vDistance.Magnitude() < vRealDistance.Magnitude()
		&& (vDistance * 2.0f).Magnitude() < vRealDistance.Magnitude())
	{
		vDistance *= 2.0f;
		sRunTime = 2;
	}
	else if (vDistance.Magnitude() > vRealDistance.Magnitude()
		|| vDistance.Magnitude() == vRealDistance.Magnitude())
	{
		sRunFinish = true;
		vDistance = vRealDistance;
	}

	if (m_TargetChanged)
	{
		m_TargetChanged = false;
		echo = uint8(1);
		m_sRegionAttackTime = ULONGLONG(UNIXTIME2 + sRunTime);
	}
	else if (sRunFinish)
	{
		echo = uint8(0);
		m_sRegionAttackTime = ULONGLONG(UNIXTIME2 + sRunTime);
	}
	else
	{
		echo = uint8(3);
		m_sRegionAttackTime = ULONGLONG(UNIXTIME2 + sRunTime);
	}

	uint16 will_x, will_z, will_y;
	will_x = uint16((vBot + vDistance).x * 10.0f);
	will_y = uint16(vpUnit.y * 10.0f);
	will_z = uint16((vBot + vDistance).z * 10.0f);
	//m_sRegionAttackTime = ULONGLONG(UNIXTIME2 + (2 * SECOND));

	if (isRogue())
		GetAssasinDamageMagic(sTargetID, (vBot + vDistance).x, vpUnit.y, (vBot + vDistance).z, will_x, will_y, will_z, sSpeed, echo);
	else if (isWarrior())
		GetWarriorDamageMagic(sTargetID, (vBot + vDistance).x, vpUnit.y, (vBot + vDistance).z, will_x, will_y, will_z, sSpeed, echo);
	else if (isMage())
	{
		int nRandom = myrand(1, 3);
		switch (nRandom)
		{
		case 1:
			GetFlameMageDamageMagic(sTargetID, (vBot + vDistance).x, vpUnit.y, (vBot + vDistance).z, will_x, will_y, will_z, sSpeed, echo);
			break;
		case 2:
			GetGlacierMageDamageMagic(sTargetID, (vBot + vDistance).x, vpUnit.y, (vBot + vDistance).z, will_x, will_y, will_z, sSpeed, echo);
			break;
		default:
			GetLightningMageDamageMagic(sTargetID, (vBot + vDistance).x, vpUnit.y, (vBot + vDistance).z, will_x, will_y, will_z, sSpeed, echo);
			break;
		}
	}
	else if (isPriest())
		GetPriestDamageMagic(sTargetID, (vBot + vDistance).x, vpUnit.y, (vBot + vDistance).z, will_x, will_y, will_z, sSpeed, echo);
}

void CBot::MerchantMoveProcess()
{
	if (m_sMoveMerchantProcess > UNIXTIME)
		return;

	std::vector<Unit*> casted_member;
	std::vector<uint16> unitList;
	g_pMain->GetUnitListFromSurroundingRegions(this, &unitList);

	foreach(itr, unitList)
	{
		Unit* pTarget = g_pMain->GetUnitPtr(*itr, GetZoneID());

		if (pTarget == nullptr)
			continue;

		if (this != pTarget
			&& !pTarget->isDead()
			&& !pTarget->isBlinking()
			&& pTarget->isAttackable())
			casted_member.push_back(pTarget);
	}

	int iValue = 0;
	int16 sTargetID = -1;
	float sRange = 0.0f, sRangeSlow = 0.0f;
	__Vector3 vBot, vpUnit, vDistance, vRealDistance;
	foreach(itr, casted_member)
	{
		Unit* pTarget = *itr; // it's checked above, not much need to check it again
		if (pTarget == nullptr)
			continue;

		if (pTarget->isDead()
			|| pTarget->GetZoneID() != GetZoneID()
			|| pTarget->isNPC()
			|| pTarget->isPlayer() && !TO_USER(pTarget)->isMerchanting()
			|| pTarget->isBot() && !TO_BOT(pTarget)->isMerchanting())
			continue;

		if ((echo == uint8(1)
			&& pTarget->GetID() != m_sTargetID)
			|| (echo == 0
				&& m_sTargetID == pTarget->GetID()))
			continue;

		sRange = pow(pTarget->GetX() + ((myrand(0, 2000) - 1000.0f) / 500.0f) - GetX(), 2.0f) + pow(pTarget->GetZ() + ((myrand(0, 2000) - 1000.0f) / 500.0f) - GetZ(), 2.0f);
		if (sRangeSlow != 0.0f && sRange > sRangeSlow)
			continue;

		sRangeSlow = sRange;
		vBot.Set(GetX(), pTarget->GetY(), GetZ());
		vpUnit.Set(pTarget->GetX() + ((myrand(0, 2000) - 1000.0f) / 500.0f), pTarget->GetY(), pTarget->GetZ() + ((myrand(0, 2000) - 1000.0f) / 500.0f));
		sTargetID = pTarget->GetID();
	}

	if (sTargetID == -1)
		return;

	if (m_sTargetID != sTargetID)
	{
		m_TargetChanged = true;
		m_sTargetID = sTargetID;
		m_oldx = vpUnit.x + ((myrand(0, 2000) - 1000.0f) / 500.0f);
		m_oldz = vpUnit.z + ((myrand(0, 2000) - 1000.0f) / 500.0f);
		m_oldy = vpUnit.y;
	}

	vDistance = vpUnit - vBot;
	vRealDistance = vDistance;
	vDistance.Normalize();

	float sSpeed = m_sSpeed;
	uint8 sRuntime = 1;
	bool sRunTimeFinish = false;
	vDistance *= sSpeed / 10.0f;

	if (echo == uint8(0)
		&& vDistance.Magnitude() < vRealDistance.Magnitude()
		&& (vDistance * 2.0f).Magnitude() < vRealDistance.Magnitude())
	{
		vDistance *= 2.0f;
		sRuntime = 2;
	}
	else if (vDistance.Magnitude() > vRealDistance.Magnitude()
		|| vDistance.Magnitude() == vRealDistance.Magnitude())
	{
		sRunTimeFinish = true;
		vDistance = vRealDistance;
	}

	if (m_TargetChanged)
	{
		m_TargetChanged = false;
		echo = uint8(1);
		m_sMoveMerchantProcess = UNIXTIME + (sRuntime);
	}
	else if (sRunTimeFinish)
	{
		echo = uint8(0);
		m_sMoveMerchantProcess = UNIXTIME + (myrand(7, 17));
	}
	else
	{
		echo = uint8(3);
		m_sMoveMerchantProcess = UNIXTIME + (sRuntime);
	}

	uint16 will_x, will_z, will_y;
	will_x = uint16((vBot + vDistance).x * 10.0f);
	will_y = uint16(vpUnit.y * 10.0f);
	will_z = uint16((vBot + vDistance).z * 10.0f);
	MoveRegionProcess((vBot + vDistance).x, vpUnit.y, (vBot + vDistance).z, will_x, will_y, will_z, sSpeed, echo);
}

void CBot::HPTimeChangeType3()
{
	if (isDead()
		|| !m_bType3Flag)
		return;

	uint16	totalActiveDurationalSkills = 0, totalActiveDOTSkills = 0;
	bool bIsDOT = false;
	for (int i = 0; i < MAX_TYPE3_REPEAT; i++)
	{
		MagicType3* pEffect = &m_durationalSkills[i];
		if (!pEffect->m_byUsed)
			continue;

		// Has the required interval elapsed before using this skill?
		if ((UNIXTIME - pEffect->m_tHPLastTime) >= pEffect->m_bHPInterval)
		{
			Unit* pUnit = g_pMain->GetUnitPtr(pEffect->m_sSourceID, GetZoneID());

			// Reduce the HP 
			HpChange(pEffect->m_sHPAmount, pUnit); // do we need to specify the source of the DOT?
			pEffect->m_tHPLastTime = UNIXTIME;

			if (pEffect->m_sHPAmount < 0)
				bIsDOT = true;

			// Has the skill expired yet?
			if (++pEffect->m_bTickCount == pEffect->m_bTickLimit)
			{
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
}

void CBot::Type4Duration()
{
	Guard lock(m_buffLock);
	if (m_buffMap.empty())
		return;

	foreach(itr, m_buffMap)
	{
		if (itr->second.m_tEndTime > UNIXTIME)
			continue;

		CMagicProcess::RemoveType4Buff(itr->first, this, true, isLockableScroll(itr->second.m_bBuffType));
		break; // only ever handle one at a time with the current logic
	}

	if (!isDebuffed())
		SendUserStatusUpdate(USER_STATUS_POISON, USER_STATUS_CURE);
}

void CBot::SendUserStatusUpdate(UserStatus type, UserStatusBehaviour status)
{
	Packet result(WIZ_ZONEABILITY, uint8(2));
	result << uint8(type) << uint8(status);
	/*
	1				, 0 = Cure damage over time
	1				, 1 = Damage over time
	2				, 0 = Cure poison
	2				, 1 = poison (purple)
	3				, 0 = Cure disease
	3				, 1 = disease (green)
	4				, 1 = blind
	5				, 0 = Cure grey HP
	5				, 1 = HP is grey (not sure what this is)
	*/
	SendToZone(&result);
}

void CBot::SendToZone(Packet* result, float fRange)
{
	g_pMain->Send_Zone(result, GetZoneID(), nullptr, 0, 0, fRange);
}

void CBot::AddBotRank(C3DMap* pMap)
{
	m_PlayerKillingLoyaltyDaily = 0; m_PlayerKillingLoyaltyPremiumBonus = 0;
	if (isInSpecialEventZone((uint8)GetZoneID()) == false)
	{
		if (GetNation() == KARUS)
		{
			_PLAYER_KILLING_ZONE_RANKING* pKarusRanking = g_pMain->m_UserPlayerKillingZoneRankingArray[0].GetData(GetID());
			if (pKarusRanking == nullptr)
			{
				_PLAYER_KILLING_ZONE_RANKING* pKillinZoneRank = new _PLAYER_KILLING_ZONE_RANKING;
				pKillinZoneRank->p_SocketID = GetID();
				pKillinZoneRank->P_Nation = GetNation();
				pKillinZoneRank->p_Zone = GetZoneID();
				//pKillinZoneRank->P_PersonelRank = m_bPersonalRank;
				pKillinZoneRank->P_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pKillinZoneRank->P_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
				if (!g_pMain->m_UserPlayerKillingZoneRankingArray[0].PutData(pKillinZoneRank->p_SocketID, pKillinZoneRank))
					delete pKillinZoneRank;
			}
			else
			{
				pKarusRanking->p_SocketID = GetID();
				pKarusRanking->P_Nation = GetNation();
				pKarusRanking->p_Zone = GetZoneID();
				//pKarusRanking->P_PersonelRank = m_bPersonalRank;
				pKarusRanking->P_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pKarusRanking->P_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
			}
			g_pMain->m_UserPlayerKillingZoneRankingArray[1].DeleteData(GetID());
		}
		else
		{
			_PLAYER_KILLING_ZONE_RANKING* pHumanRanking = g_pMain->m_UserPlayerKillingZoneRankingArray[1].GetData(GetID());
			if (pHumanRanking == nullptr)
			{
				_PLAYER_KILLING_ZONE_RANKING* pKillinZoneRank = new _PLAYER_KILLING_ZONE_RANKING;
				pKillinZoneRank->p_SocketID = GetID();
				pKillinZoneRank->P_Nation = GetNation();
				pKillinZoneRank->p_Zone = GetZoneID();
				//pKillinZoneRank->P_PersonelRank = m_bPersonalRank;
				pKillinZoneRank->P_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pKillinZoneRank->P_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
				if (!g_pMain->m_UserPlayerKillingZoneRankingArray[1].PutData(pKillinZoneRank->p_SocketID, pKillinZoneRank))
					delete pKillinZoneRank;
			}
			else
			{
				pHumanRanking->p_SocketID = GetID();
				pHumanRanking->P_Nation = GetNation();
				pHumanRanking->p_Zone = GetZoneID();
				//pHumanRanking->P_PersonelRank = m_bPersonalRank;
				pHumanRanking->P_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pHumanRanking->P_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
			}
			g_pMain->m_UserPlayerKillingZoneRankingArray[0].DeleteData(GetID());
		}
	}
	else
	{
		if (GetNation() == (uint8)Nation::KARUS)
		{
			_ZINDAN_WAR_RANKING* pKarusRanking = g_pMain->m_ZindanWarZoneRankingArray[0].GetData(GetID());
			if (pKarusRanking == nullptr)
			{
				_ZINDAN_WAR_RANKING* ZindanWarZoneRank = new _ZINDAN_WAR_RANKING;
				ZindanWarZoneRank->z_SocketID = GetID();
				ZindanWarZoneRank->z_Nation = GetNation();
				ZindanWarZoneRank->z_Zone = GetZoneID();
				ZindanWarZoneRank->z_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				ZindanWarZoneRank->z_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
				if (!g_pMain->m_ZindanWarZoneRankingArray[0].PutData(ZindanWarZoneRank->z_SocketID, ZindanWarZoneRank))
					delete ZindanWarZoneRank;
			}
			else
			{
				pKarusRanking->z_SocketID = GetID();
				pKarusRanking->z_Nation = GetNation();
				pKarusRanking->z_Zone = GetZoneID();
				pKarusRanking->z_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pKarusRanking->z_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
			}
			g_pMain->m_ZindanWarZoneRankingArray[1].DeleteData(GetID());
		}
		else
		{
			_ZINDAN_WAR_RANKING* pHumanRanking = g_pMain->m_ZindanWarZoneRankingArray[1].GetData(GetID());
			if (pHumanRanking == nullptr)
			{
				_ZINDAN_WAR_RANKING* ZindanWarZoneRank = new _ZINDAN_WAR_RANKING;
				ZindanWarZoneRank->z_SocketID = GetID();
				ZindanWarZoneRank->z_Nation = GetNation();
				ZindanWarZoneRank->z_Zone = GetZoneID();
				ZindanWarZoneRank->z_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				ZindanWarZoneRank->z_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
				if (!g_pMain->m_ZindanWarZoneRankingArray[1].PutData(ZindanWarZoneRank->z_SocketID, ZindanWarZoneRank))
					delete ZindanWarZoneRank;
			}
			else
			{
				pHumanRanking->z_SocketID = GetID();
				pHumanRanking->z_Nation = GetNation();
				pHumanRanking->z_Zone = GetZoneID();
				pHumanRanking->z_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pHumanRanking->z_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
			}
			g_pMain->m_ZindanWarZoneRankingArray[0].DeleteData(GetID());
		}
	}
}

void CBot::StateChangeServerDirect(uint8 bType, uint32 nBuff)
{
	uint8 buff = *(uint8*)&nBuff; // don't ask
	switch (bType)
	{
	case 1:
		m_bResHpType = buff;
		break;

	case 2:
		m_bNeedParty = buff;
		break;

	case 3:
		m_nOldAbnormalType = m_bAbnormalType;
		m_bAbnormalType = nBuff;
		break;

	case 5:
		m_bAbnormalType = nBuff;
		break;

	case 6:
		nBuff = m_bPartyLeader; // we don't set this here.
		break;

	case 7:
		break;

	case 8: // beginner quest
		break;

	case 14:
		break;
	}

	Packet result(WIZ_STATE_CHANGE);
	result << GetID() << bType << uint64(nBuff);
	SendToRegion(&result);
}

void CBot::OnDeath(Unit* pKiller)
{
	if (m_bResHpType == USER_DEAD)
		return;

	m_bResHpType = USER_DEAD;
	m_BotState = BOT_DEAD;
	if (isDead()) SendDeathAnimation(); // bot kill effecti
	isReset(false);

	if (pKiller != nullptr)
	{
		if (pKiller->isPlayer())
		{
			CUser* pUserKiller = TO_USER(pKiller);
			{
				if (pUserKiller != nullptr)
					OnDeathKilledPlayer(pUserKiller);
			}
		}
		else if (pKiller->isNPC())
		{
			CNpc* pNpcKiller = TO_NPC(pKiller);
			if (pNpcKiller != nullptr)
				OnDeathKilledNpc(pNpcKiller);
		}
		else if (pKiller->isBot())
		{
			CBot* pBotKiller = TO_BOT(pKiller);
			if (pBotKiller != nullptr)
				OnDeathKilledBot(pBotKiller);
		}
		else
		{
			printf("OnDeath warning \n");
			TRACE("OnDeath warning \n");
		}
	}

	InitOnDeath(pKiller);
}

void CBot::InitOnDeath(Unit* pKiller)
{
	Unit::OnDeath(pKiller);

	// Player is dead stop other process.
	InitType3();
	InitType4();

	if (pKiller->isBot())
		TO_BOT(pKiller)->isReset(true);
}

void CBot::OnDeathKilledPlayer(CUser* pKiller)
{
	int16 m_sWhoKilledMe;
	if (pKiller != nullptr)
	{
		if (pKiller->GetName() != GetName())
		{
			bool m_party_check = false;
			uint16 bonusNP = 0;
			switch (pKiller->GetZoneID())
			{
			case ZONE_CHAOS_DUNGEON:
				if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
				{
					if (pKiller->isEventUser())
					{

						pKiller->m_ChaosExpansionKillCount++;
						pKiller->UpdateChaosExpansionRank();

						//pKiller->AchieveWarCountAdd(UserAchieveWarTypes::AchieveKillCountChaos, 0, this);
						SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
					}
				}
				break;
			case ZONE_BORDER_DEFENSE_WAR:
				if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
				{
					if (pKiller->isEventUser())
					{
						pKiller->BDWUpdateRoomKillCount();
						SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
					}
				}
				break;
			case ZONE_JURAID_MOUNTAIN:
				if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
				{
					if (pKiller->isEventUser())
					{
						pKiller->JRUpdateRoomKillCount();
						SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
					}
				}
				break;
			case ZONE_DELOS:
			{
				if (g_pMain->isCswActive() && pKiller->isInClan() && g_pMain->isCswWarActive())
				{
					pKiller->csw_rank_killupdate();
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
					m_party_check = true;
				}
			}
			break;
			case ZONE_MORADON:
			case ZONE_MORADON2:
			case ZONE_MORADON3:
			case ZONE_MORADON4:
			case ZONE_MORADON5:
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				break;
			case ZONE_CLAN_WAR_RONARK:
			case ZONE_CLAN_WAR_ARDREAM:
			case ZONE_PARTY_VS_1:
			case ZONE_PARTY_VS_2:
			case ZONE_PARTY_VS_3:
			case ZONE_PARTY_VS_4:
				if (pKiller->isClanTournamentinZone() || pKiller->isPartyTournamentinZone())
				{
					g_pMain->UpdateClanTournamentScoreBoard(pKiller);
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
				break;
			case ZONE_ARDREAM:
			case ZONE_RONARK_LAND:
			case ZONE_RONARK_LAND_BASE:
				if (pKiller->isInPKZone())
				{
					m_party_check = true;
					pKiller->KA_KillUpdate(); // re_killcount.uif kill-count UI
					bool bKilledByRival = false;

					// Show death notices in PVP zones
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);

					// If the killer has us set as their rival, reward them & remove the rivalry.
					bKilledByRival = (!pKiller->hasRivalryExpired() && pKiller->GetRivalID() == GetID());
					if (bKilledByRival)
					{
						// If we are our killer's rival, use the rival notice instead.
						SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeRival, true);

						// Apply bonus NP for rival kills
						bonusNP += RIVALRY_NP_BONUS;

						// This player is no longer our rival
						if (pKiller->isInGame())
							pKiller->RemoveRival();
					}
					// The anger gauge is increased on each death.
					// When your anger gauge is full (5 deaths), you can use the "Anger Explosion" skill.
					if (!hasFullAngerGauge())
						UpdateAngerGauge(++m_byAngerGauge);

					// If we don't have a rival, this player is now our rival for 3 minutes.
					if (!hasRival())
						SetRival(pKiller);
				}
				break;
			case ZONE_ELMORAD:
			case ZONE_KARUS:
				if (g_pMain->isWarOpen())
				{
					m_party_check = true;
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
				}
				break;

			case ZONE_BATTLE:
			case ZONE_BATTLE2:
			case ZONE_BATTLE3:
			case ZONE_BATTLE4:
			case ZONE_BATTLE5:
			case ZONE_BATTLE6:
				if (g_pMain->isWarOpen())
				{
					m_party_check = true;
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
				}
				break;
			case ZONE_BIFROST:
				m_party_check = true;
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
				break;
			case ZONE_SNOW_BATTLE:
				if (g_pMain->m_byBattleOpen == SNOW_BATTLE)
				{
					pKiller->GoldGain(SNOW_EVENT_MONEY);
					GetNation() == KARUS ? g_pMain->m_sKarusDead++ : g_pMain->m_sElmoradDead++;;
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
				break;
			}

			bool specialevent = pKiller->isInSpecialEventZone() && isInSpecialEventZone() && g_pMain->pSpecialEvent.opened;
			bool cindireallaw = g_pMain->pCindWar.isStarted()
				&& pKiller->pCindWar.isEventUser()
				&& g_pMain->isCindirellaZone(GetZoneID())
				&& g_pMain->isCindirellaZone(pKiller->GetZoneID());
			if (specialevent || cindireallaw)
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);


			if (pKiller->isInPKZone())
			{
				if (pKiller->isInParty())
				{
					_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(pKiller->GetPartyID());
					if (pParty != nullptr)
					{
						Guard lock(g_pMain->m_PartyArray.m_lock);
						for (int i = 0; i < MAX_PARTY_USERS; i++)
						{
							CUser* pPartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
							if (pPartyUser == nullptr
								|| !isInRangeSlow(pPartyUser, RANGE_50M)
								|| pPartyUser->GetZoneID() != pKiller->GetZoneID())
								continue;

							JstKO_ApplyBotPvpKillReward(pPartyUser, true);
						}
					}
				}
				else
				{
					JstKO_ApplyBotPvpKillReward(pKiller, false);
				}
			}

			/**/
			if (m_party_check)
			{
				if (pKiller->isInParty())
				{
					_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(pKiller->GetPartyID());
					if (pParty == nullptr)
						return;

					short partyUsers[MAX_PARTY_USERS];
					for (int i = 0; i < MAX_PARTY_USERS; i++)
						partyUsers[i] = pParty->uid[i];

					for (int i = 0; i < MAX_PARTY_USERS; i++)
					{
						CUser* PartyUser = g_pMain->GetUserPtr(partyUsers[i]);
						if (PartyUser == nullptr
							|| !isInRangeSlow(PartyUser, RANGE_50M)
							|| PartyUser->isDead()
							|| !PartyUser->isInGame()
							|| !PartyUser->isPlayer())
							continue;

						if (PartyUser->GetNation() == KARUS)
							PartyUser->QuestV2MonsterCountAdd(KARUS);
						else if (PartyUser->GetNation() == ELMORAD)
							PartyUser->QuestV2MonsterCountAdd(ELMORAD);

						// Daily quest user kill counter (bot victim).
						PartyUser->UpdateDailyQuestCount(1);

						if (g_pMain->pCollectionRaceEvent.isCRActive)
							g_pMain->CollectionRaceSendDead(PartyUser, 0x01);
					}
				}
				else
				{
					if (GetNation() == KARUS)
						pKiller->QuestV2MonsterCountAdd(KARUS);
					else if (GetNation() == ELMORAD)
						pKiller->QuestV2MonsterCountAdd(ELMORAD);

					// Daily quest user kill counter (bot victim).
					pKiller->UpdateDailyQuestCount(1);

					if (g_pMain->pCollectionRaceEvent.isCRActive)
						g_pMain->CollectionRaceSendDead(pKiller, 0x01);
				}
			}
			/**/

			if (pKiller->GetMap()->m_bGiveLoyalty != 0)
			{
				if (pKiller->isInParty())
					pKiller->LoyaltyDivide("UserKill", this, bonusNP);
				else
					pKiller->LoyaltyChange(this, bonusNP);
			}

			m_sWhoKilledMe = pKiller->GetID();
		}
		else
			m_sWhoKilledMe = -1;
	}
}

void CBot::OnDeathKilledBot(CBot* pKiller)
{
	int16 m_sWhoKilledMe;
	if (pKiller != nullptr)
	{
		if (pKiller->GetName() != GetName())
		{
			uint16 bonusNP = 0;
			switch (pKiller->GetZoneID())
			{
			case ZONE_CHAOS_DUNGEON:
				if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
				{/*
					m_ChaosExpansionDeadCount++;
					UpdateChaosExpansionRank();*/
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
				break;
			case ZONE_BORDER_DEFENSE_WAR:
				if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
				{
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
				break;
			case ZONE_JURAID_MOUNTAIN:
				if (pKiller->isInTempleEventZone(pKiller->GetZoneID()))
				{
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
				break;
			case ZONE_MORADON:
			case ZONE_MORADON2:
			case ZONE_MORADON3:
			case ZONE_MORADON4:
			case ZONE_MORADON5:
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				break;
			case ZONE_KNIGHT_ROYALE:
				break;
			case ZONE_CLAN_WAR_RONARK:
			case ZONE_CLAN_WAR_ARDREAM:
			case ZONE_PARTY_VS_1:
			case ZONE_PARTY_VS_2:
			case ZONE_PARTY_VS_3:
			case ZONE_PARTY_VS_4:
				if (pKiller->isClanTournamentinZone() || pKiller->isPartyTournamentinZone())
				{
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
				break;
			case ZONE_ARDREAM:
			case ZONE_RONARK_LAND:
			case ZONE_RONARK_LAND_BASE:
				if (pKiller->isInPKZone())
				{
					bool bKilledByRival = false;

					// Show death notices in PVP zones
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);

					// If the killer has us set as their rival, reward them & remove the rivalry.
					bKilledByRival = (!pKiller->hasRivalryExpired() && pKiller->GetRivalID() == GetID());
					if (bKilledByRival)
					{
						// If we are our killer's rival, use the rival notice instead.
						SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeRival, true);

						// Apply bonus NP for rival kills
						bonusNP += RIVALRY_NP_BONUS;

						// This player is no longer our rival
						if (pKiller->isInGame())
							pKiller->RemoveRival();
					}
					// The anger gauge is increased on each death.
					// When your anger gauge is full (5 deaths), you can use the "Anger Explosion" skill.
					if (!hasFullAngerGauge())
						UpdateAngerGauge(++m_byAngerGauge);

					// If we don't have a rival, this player is now our rival for 3 minutes.
					if (!hasRival())
						SetRival(pKiller);
				}
				break;
			case ZONE_ELMORAD:
			case ZONE_KARUS:
				if (g_pMain->isWarOpen())
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
				break;

			case ZONE_BATTLE:
			case ZONE_BATTLE2:
			case ZONE_BATTLE3:
			case ZONE_BATTLE4:
			case ZONE_BATTLE5:
			case ZONE_BATTLE6:
				if (g_pMain->isWarOpen())
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, false);
				break;
			case ZONE_SNOW_BATTLE:
				if (g_pMain->m_byBattleOpen == SNOW_BATTLE)
				{
					GetNation() == KARUS ? g_pMain->m_sKarusDead++ : g_pMain->m_sElmoradDead++;;
					SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
				}
				break;
			}

			bool specialevent = pKiller->isInSpecialEventZone() && isInSpecialEventZone() && g_pMain->pSpecialEvent.opened;
			bool cindireallaw = false;
			if (specialevent || cindireallaw)
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);

			if (pKiller->GetMap()->m_bGiveLoyalty != 0)
			{
				if (pKiller->isInParty())
					pKiller->LoyaltyBotDivide(GetID(), bonusNP);
				else
					pKiller->LoyaltyBotChange(GetID(), bonusNP);

				// Mixed party support: if killer bot is grouped with real users,
				// trigger user-side party NP distribution as well.
				if (pKiller->isInParty())
				{
					_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(pKiller->GetPartyID());
					if (pParty != nullptr)
					{
						for (int i = 0; i < MAX_PARTY_USERS; i++)
						{
							CUser* pPartyUser = g_pMain->GetUserPtr(pParty->uid[i]);
							if (pPartyUser == nullptr
								|| !pPartyUser->isInGame()
								|| pPartyUser->isDead()
								|| !pKiller->isInRangeSlow(pPartyUser, RANGE_50M))
								continue;

							pPartyUser->LoyaltyDivide("BotKill", this, bonusNP);
							break; // LoyaltyDivide already distributes to user party.
						}
					}
				}
			}

			m_sWhoKilledMe = pKiller->GetID();
		}
		else
			m_sWhoKilledMe = -1;
	}
}

void CBot::OnDeathKilledNpc(CNpc* pKiller)
{
	if (pKiller != nullptr)
	{
		int64 nExpLost = 0;

		switch (pKiller->GetZoneID())
		{
		case ZONE_ARDREAM:
		case ZONE_RONARK_LAND:
		case ZONE_RONARK_LAND_BASE:
			if (pKiller->GetType() == NPC_GUARD_TOWER1 || pKiller->GetType() == NPC_GUARD_TOWER2)
				SendDeathNotice(pKiller, DeathNoticeType::DeathNoticeCoordinates, true);
			break;
		}
	}
}

uint8 CBot::GetPVPMonumentNation() { return g_pMain->m_nPVPMonumentNation[GetZoneID()]; }

void CBot::UpdatePlayerKillingRank()
{
	if (isInSpecialEventZone((uint8)GetZoneID()) == false)
	{
		if (GetNation() == KARUS)
		{
			_PLAYER_KILLING_ZONE_RANKING* pKarusRanking = g_pMain->m_UserPlayerKillingZoneRankingArray[0].GetData(GetID());
			if (pKarusRanking != nullptr)
			{
				pKarusRanking->P_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pKarusRanking->P_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
			}
		}
		else
		{
			_PLAYER_KILLING_ZONE_RANKING* pKarusRanking = g_pMain->m_UserPlayerKillingZoneRankingArray[1].GetData(GetID());
			if (pKarusRanking != nullptr)
			{
				pKarusRanking->P_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pKarusRanking->P_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
			}
		}
	}
	else
	{
		if (GetNation() == (uint8)Nation::KARUS)
		{
			_ZINDAN_WAR_RANKING* pKarusRanking = g_pMain->m_ZindanWarZoneRankingArray[0].GetData(GetID());;
			if (pKarusRanking != nullptr)
			{
				pKarusRanking->z_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pKarusRanking->z_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
			}
		}
		else
		{
			_ZINDAN_WAR_RANKING* pHumanRanking = g_pMain->m_ZindanWarZoneRankingArray[1].GetData(GetID());;
			if (pHumanRanking != nullptr)
			{
				pHumanRanking->z_LoyaltyDaily = m_PlayerKillingLoyaltyDaily;
				pHumanRanking->z_LoyaltyPremiumBonus = m_PlayerKillingLoyaltyPremiumBonus;
			}
		}
	}
}
uint16 CGameServerDlg::SpawnSlaveUserBot(int Minute, CUser* pUser)
{
	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (pUser->GetZoneID() <= ZONE_ELMORAD && pUser->GetZoneID() != pBotInfo->m_bNation
			|| (pUser->GetZoneID() >= ZONE_KARUS_ESLANT
				&& pUser->GetZoneID() <= ZONE_ELMORAD_ESLANT
				&& pUser->GetZoneID() != (pBotInfo->m_bNation + 10)))
			continue;

		/*if (pBotInfo->m_bLevel < pUser->GetLevel())
			continue;*/

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = "[Slave]" + pUser->GetName();
		pBot->m_bNation = pUser->m_bNation;
		pBot->m_bRace = pUser->m_bRace;
		pBot->m_sClass = pUser->m_sClass;
		pBot->m_nHair = pUser->m_nHair;
		pBot->m_bLevel = pUser->m_bLevel;
		pBot->m_bFace = pUser->m_bFace;
		pBot->m_bKnights = pUser->m_bKnights;
		pBot->m_bFame = pUser->m_bFame;

		memcpy(pBot->m_sItemArray, pUser->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pUser->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pUser->m_bStats, sizeof(pBot->m_bStats));
		memset(pBot->m_arMerchantItems, 0x00, sizeof(pBot->m_arMerchantItems));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pUser->m_sCoverTitle;
		pBot->m_reblvl = pUser->m_bRebirthLevel;
		pBot->m_iGold = pUser->m_iGold;
		pBot->m_sPoints = pUser->m_sPoints;
		pBot->m_iLoyalty = pUser->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pUser->m_iLoyaltyMonthly;

		int Random = myrand(0, 10000);
		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute * 60);
		pBot->m_pMap = GetZoneByID(pUser->GetZoneID());
		pBot->m_bZone = pUser->GetZoneID();
		pBot->m_sDirection = pUser->m_sDirection;
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->SetBotAbility();
		pBot->m_BotState = BOT_MERCHANT;
		pBot->SetPosition(pUser->GetX(), pUser->GetY(), pUser->GetZ());
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());

		pBot->m_bSlaveMerchant = true;
		pBot->m_bSlaveUserID = pUser->GetSocketID();


		pUser->m_bSlaveMerchant = true;
		pUser->m_bSlaveUserID = pBot->GetID();

		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		pBot->StateChangeServerDirect(1, Random > 5000 ? USER_STANDING : USER_SITDOWN);
		return pBot->GetID();
	}
	return 0;
}

uint8 CBot::GetSymbol()
{
	if (m_bPersonalRank < 1 && m_bKnightsRank < 1)
		return 0;
	uint8 bRaking = 0;
	if (m_bPersonalRank < m_bKnightsRank)
	{
		if (m_bPersonalRank == 1)
			bRaking = 30;
		else if (m_bPersonalRank > 1 && m_bPersonalRank <= 4)
			bRaking = 31;
		else if (m_bPersonalRank > 4 && m_bPersonalRank <= 10)
			bRaking = 32;
		else if (m_bPersonalRank > 10 && m_bPersonalRank <= 40)
			bRaking = 33;
		else if (m_bPersonalRank > 40 && m_bPersonalRank <= 100)
			bRaking = 34;
		else if (m_bPersonalRank > 100 && m_bPersonalRank <= 200)
			bRaking = 35;
	}
	else if (m_bPersonalRank > m_bKnightsRank)
	{
		if (m_bKnightsRank == 1)
			bRaking = 20;
		else if (m_bKnightsRank > 1 && m_bKnightsRank <= 4)
			bRaking = 21;
		else if (m_bKnightsRank > 4 && m_bKnightsRank <= 10)
			bRaking = 22;
		else if (m_bKnightsRank > 10 && m_bKnightsRank <= 40)
			bRaking = 23;
		else if (m_bKnightsRank > 40 && m_bKnightsRank <= 100)
			bRaking = 24;
		else if (m_bKnightsRank > 100 && m_bKnightsRank <= 200)
			bRaking = 25;
	}
	else if (m_bPersonalRank == m_bKnightsRank)
	{
		if (m_bKnightsRank == 1)
			bRaking = 20;
		else if (m_bKnightsRank > 1 && m_bKnightsRank <= 4)
			bRaking = 21;
		else if (m_bKnightsRank > 4 && m_bKnightsRank <= 10)
			bRaking = 22;
		else if (m_bKnightsRank > 10 && m_bKnightsRank <= 40)
			bRaking = 23;
		else if (m_bKnightsRank > 40 && m_bKnightsRank <= 100)
			bRaking = 24;
		else if (m_bKnightsRank > 100 && m_bKnightsRank <= 200)
			bRaking = 25;
	}

	//g_pMain->SendHelpDescription(this, string_format("return %d", bRaking));
	return bRaking;
}

void CBot::SendToRegion(Packet* pkt, CUser* pExceptUser /*= nullptr*/, uint16 nEventRoom /*-1*/)
{
	g_pMain->Send_Region(pkt, GetMap(), GetRegionX(), GetRegionZ(), pExceptUser, nEventRoom);
}

