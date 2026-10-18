#include "stdafx.h"

using namespace std;

/**
* @brief	Changes the player's experience points by iExp.
*
* @param	iExp	The amount of experience points to adjust by.
*/
void CUser::ExpChange(std::string descp, int64 iExp, bool bIsBonusReward)
{
	int64 baseexpamount = iExp;

	// GetMap Nullptr crash control
	if (GetMap() == nullptr)
		return;

	// Stop players level 5 or under from losing XP on death.
	if ((GetLevel() < 6 && iExp < 0)
		// Stop players in the war zone (TODO: Add other war zones) from losing XP on death.
		|| (GetMap()->isWarZone() && iExp < 0))
		return;

	// Despite being signed, we don't want m_iExp ever going below 0.
	// If this happens, we need to investigate why -- not sweep it under the rug.
	if (m_iExp < 0)
		return;

	if (iExp > 0)
	{
		if (!bIsBonusReward)
		{
			uint64 TempExp = iExp, FinalExp = iExp;
			
			if (m_bItemExpGainAmount > 0)
				FinalExp += (TempExp * m_bItemExpGainAmount) / 100;

			if (m_sExpGainbuff11Amount > 0)
				FinalExp += (TempExp * m_sExpGainbuff11Amount) / 100;

			if (m_sExpGainbuff33Amount > 0)
				FinalExp += (TempExp * m_sExpGainbuff33Amount) / 100;

			if (g_pMain->m_byExpEventAmount > 0)
				FinalExp += (TempExp * g_pMain->m_byExpEventAmount) / 100;

			if (m_flameilevel > 0 && m_flameilevel <= 3 && g_pMain->pBurningFea[m_flameilevel - 1].exprate)
				FinalExp += (TempExp * g_pMain->pBurningFea[m_flameilevel - 1].exprate) / 100;

			uint16 prem_count = 0;
			if (GetPremium() && !isDead()) {
				prem_count = GetPremiumProperty(PremiumPropertyOpCodes::PremiumExpPercent);
				if (prem_count > 0)
					FinalExp += (TempExp * prem_count) / 100;
			}

			prem_count = 0;
			if (GetPremium() && !isDead()) {
				prem_count = GetClanPremiumProperty(PremiumPropertyOpCodes::PremiumExpPercent);
				if (prem_count > 0)
					FinalExp += (TempExp * prem_count) / 100;
			}

			if (m_FlashExpBonus > 0)
				FinalExp += (TempExp * m_FlashExpBonus) / 100;

			if (isInClan()) {
				auto *pmyknight = g_pMain->GetClanPtr(GetClanID());
				if (pmyknight && pmyknight->m_bOnlineExpCount > 0)
					FinalExp += (TempExp * pmyknight->m_bOnlineExpCount) / 100;
			}

			uint32 perkExperience = 0;
			if (pPerks.perkType[(int)perks::percentExp] > 0) {
				auto* perks = g_pMain->m_PerksArray.GetData((int)perks::percentExp);
				if (perks && perks->perkCount)
					perkExperience += perks->perkCount * pPerks.perkType[(int)perks::percentExp];
			}

			if (perkExperience)
				FinalExp += (TempExp * perkExperience) / 100;

			iExp = FinalExp;
		}
	}

	ExpChangeInsertLog(baseexpamount, iExp, descp);

	bool bLevel = true;
	if (iExp < 0 && (m_iExp + iExp) < 0)
		bLevel = false;
	else
	{
		if (bExpSealStatus)
			ExpSealChangeExp(iExp);
		else
			m_iExp += iExp;
	}

	// If we need to delevel...
	if (!bLevel)
	{
		// Drop us back a level.
		m_bLevel--;

		// Get the excess XP (i.e. below 0), so that we can take it off the max XP of the previous level
		int64 diffXP = m_iExp + OnDeathLostExpCalc(g_pMain->GetExpByLevel(m_bLevel, GetRebirthLevel()));

		// Now reset our XP to max for the former level.
		m_iExp = g_pMain->GetExpByLevel(GetLevel(), GetRebirthLevel());

		// Get new stats etc.
		LevelChange(GetLevel(), false);

		// Take the remainder of the XP off (and delevel again if necessary).
		ExpChange("abc1", - diffXP);
		return;
	}
	// If we've exceeded our XP requirement, we've leveled.
	else if (m_iExp >= m_iMaxExp)
	{
		if (GetLevel() < g_pMain->m_byMaxLevel) {
			// Reset our XP, level us up.
			m_iExp -= m_iMaxExp;
			LevelChange(++m_bLevel);
			return;
		}

		// Hit the max level? Can't level any further. Cap the XP.
		m_iExp = m_iMaxExp;
	}

	if (GetLevel() < GetMap()->GetMinLevelReq()
		|| GetLevel() > GetMap()->GetMaxLevelReq()) {
		KickOutZoneUser();
	}

	if (m_bIsChicken && GetLevel() > 29) 
		m_bIsChicken = false;

	// Tell the client our new XP
	Packet result(WIZ_EXP_CHANGE);
	result << uint8(4) << m_iExp; // NOTE: Use proper flag
	Send(&result);
}

/**
* @brief	Handles stat updates after a level change.
* 			It does not change the level.
*
* @param	level   	The level we've changed to.
* @param	bLevelUp	true to level up, false for deleveling.
*/
//void CUser::LevelChange(uint8 level, bool bLevelUp)
//{
//	if (level < 1 || level > g_pMain->m_byMaxLevel)
//		return;
//
//	if (bLevelUp && level > GetLevel() + 1)
//	{
//		int16 nStatTotal = 300 + (level - 1) * 3;
//		uint8 nSkillTotal = (level - 9) * 2;
//
//		if (level > 60)
//			nStatTotal += 2 * (level - 60);
//
//		m_sPoints += nStatTotal - GetStatTotal();
//		m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree] += nSkillTotal - GetTotalSkillPoints();
//		m_bLevel = level;
//	}
//	else if (bLevelUp)
//	{
//		// On each level up, we should give 3 stat points for levels 1-60.
//		// For each level above that, we give an additional 2 stat points (so 5 stat points per level).
//		int levelsAfter60 = (level > 60 ? level - 60 : 0);
//		if ((m_sPoints + GetStatTotal()) < int32(297 + (3 * level) + (2 * levelsAfter60)))
//			m_sPoints += (levelsAfter60 == 0 ? 3 : 5);
//
//		if (level >= 10 && GetTotalSkillPoints() < 2 * (level - 9))
//			m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree] += 2;
//		
//		AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveReachLevel, 0, nullptr);
//
//		if (level > 1 && level < 84 && !m_sLevelArray[level-1])
//		{
//			auto pLev = g_pMain->GetLevelRewards(level);
//			if (!pLev.isnull) {
//				for (uint8 i = 0; i < 3; i++)
//					if (pLev.itemid[i] && pLev.itemcount[i])
//						GiveItem("level reward", pLev.itemid[i], pLev.itemcount[i], true, pLev.itemtime[i]);
//
//				if (pLev.cash || pLev.tl)
//					GiveBalance(pLev.cash, pLev.tl);
//
//				if (pLev.money)
//					GoldGain(pLev.money, true);
//
//				if (pLev.loyalty)
//					SendLoyaltyChange("level reward", pLev.loyalty, false, false, true);
//
//				std::string notice = GetName() + " has reached " + string_format("%d Level!.", level).c_str() + " " + pLev.bNotice;
//				if (notice.size())
//					g_pMain->SendGameNotice(ChatType::GENERAL_CHAT, notice, "", true, nullptr, true);
//			}
//			m_sLevelArray[level - 1] = true;
//		}
//	}
//
//	if (!bLevelUp)
//		m_bLevel = level;
//
//	m_iMaxExp = g_pMain->GetExpByLevel(level, GetRebirthLevel());
//
//	SetUserAbility();
//	m_sMp = m_MaxMp;
//	HpChange(GetMaxHealth());
//
//	if (!m_flashtime && (m_FlashExpBonus > 0 || m_FlashDcBonus > 0 || m_FlashWarBonus > 0))
//		SetFlashTimeNote(true);
//
//	Packet result(WIZ_LEVEL_CHANGE);
//	result << GetSocketID()
//		<< GetLevel() << m_sPoints << m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree]
//		<< m_iMaxExp << m_iExp
//		<< m_MaxHp << m_sHp
//		<< m_MaxMp << m_sMp
//		<< m_sMaxWeight << m_sItemWeight;
//
//	g_pMain->Send_Region(&result, GetMap(), GetRegionX(), GetRegionZ(), nullptr, GetEventRoom());
//	if (isInParty())
//	{
//		// TODO: Move this to party specific code
//		result.Initialize(WIZ_PARTY);
//		result << uint8(PARTY_LEVELCHANGE) << GetSocketID() << GetLevel();
//		g_pMain->Send_PartyMember(GetPartyID(), &result);
//
//		if (m_bIsChicken)
//			GrantChickenManner();
//	}
//
//	// We should kick players out of the zone if their level no longer matches the requirements for this zone.
//	if (GetLevel() < GetMap()->GetMinLevelReq() || GetLevel() > GetMap()->GetMaxLevelReq())
//		KickOutZoneUser();
//
//#if(GAME_SOURCE_VERSION == 1098)
//	if (level == 30 && ChickenStatus == 0)
//	{
//		SendMyInfo();
//		UserInOut(INOUT_OUT);
//		SetRegion(GetNewRegionX(), GetNewRegionZ());
//		UserInOut(INOUT_WARP);
//		g_pMain->UserInOutForMe(this);
//		NpcInOutForMe();
//		g_pMain->MerchantUserInOutForMe(this);
//
//		ResetWindows();
//		InitType4();
//		RecastSavedMagic();
//
//		GoldGain(100000);  // civciv patlay�nca verilen para �d�l� 500K
//		ChickenStatus = 1;
//		ShowEffect(490092);  // Civciv patlay�nca kafada konfeti patl�yor.
//
//		bool genie = false;
//		if (isGenieActive()) {
//			GenieStop(); genie = true;
//		}
//
//		if (genie) GenieStart();
//	}
//	
//#endif
//	SendPresetReqMoney();
//}

void CUser::LevelChange(uint8 level, bool bLevelUp)
{
	if (level < 1 || level > g_pMain->m_byMaxLevel)
		return;

	if (bLevelUp && level > GetLevel() + 1)
	{
		int16 nStatTotal = 300 + (level - 1) * 3;
		uint8 nSkillTotal = (level - 9) * 2;

		if (level > 60)
			nStatTotal += 2 * (level - 60);

		m_sPoints += nStatTotal - GetStatTotal();
		m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree] += nSkillTotal - GetTotalSkillPoints();
		m_bLevel = level;
	}
	else if (bLevelUp)
	{
		// On each level up, we should give 3 stat points for levels 1-60.
		// For each level above that, we give an additional 2 stat points (so 5 stat points per level).
		int levelsAfter60 = (level > 60 ? level - 60 : 0);
		if ((m_sPoints + GetStatTotal()) < int32(297 + (3 * level) + (2 * levelsAfter60)))
			m_sPoints += (levelsAfter60 == 0 ? 3 : 5);

		if (level >= 10 && GetTotalSkillPoints() < 2 * (level - 9))
			m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree] += 2;

		AchieveNormalCountAdd(UserAchieveNormalTypes::AchieveReachLevel, 0, nullptr);

		if (level > 1 && level < 84 && !m_sLevelArray[level - 1])
		{
			auto pLev = g_pMain->GetLevelRewards(level);
			if (!pLev.isnull) {
				for (uint8 i = 0; i < 3; i++)
					if (pLev.itemid[i] && pLev.itemcount[i])
						GiveItem("Level Reward", pLev.itemid[i], pLev.itemcount[i], true, pLev.itemtime[i]);

				if (pLev.cash || pLev.tl)
					GiveBalance(pLev.cash, pLev.tl);

				if (pLev.money)
					GoldGain(pLev.money, true);

				if (pLev.loyalty)
					SendLoyaltyChange("Level Reward", pLev.loyalty, false, false, true);

				std::string notice = GetName() + " has reached " + string_format("%d Level!.", level).c_str() + " " + pLev.bNotice;
				if (notice.size())
					g_pMain->SendGameNotice(ChatType::GENERAL_CHAT, notice, "", true, nullptr, true);
			}
			m_sLevelArray[level - 1] = true;
		}
	}

	//######################################################################################################

    // JstKO Yeni Level Y�ksek K&H List �zleme Pm Notice Eklendi. 22.04.2025
	std::map<uint8, std::string> levelNotices =
	{
		{ 5,  "Level 5 oldun! Ba�lang�� art�k bitti." },
		{ 10, "Level 10 oldun! Yolun ba��ndas�n." },
		{ 15, "Level 15 oldun! G��leniyorsun." },
		{ 20, "Level 20 oldun! Art�k tehlikeler seni bekliyor." },
		{ 25, "Level 25 oldun! Harika gidiyorsun." },
		{ 30, "Level 30 oldun! Orta seviyelere ula�t�n." },
		{ 35, "Level 35 oldun! G��l� bir sava���s�n." },
		{ 40, "Level 40 oldun! Seni kimse tutamaz." },
		{ 45, "Level 45 oldun! Tehlikeli d��manlara haz�r ol." },
		{ 50, "Level 50 oldun! Efsane olma yolundas�n." },
		{ 55, "Level 55 oldun! Harikalar diyar�na ho� geldin." },
		{ 60, "Level 60 oldun! Art�k bir ustas�n." },
		{ 65, "Level 65 oldun! Sayg� duyulan birisin." },
		{ 70, "Level 70 oldun! Destan yazmaya az kald�." },
		{ 75, "Level 75 oldun! Zafere yakla�t�n." },
		{ 80, "Level 80 oldun! Efsaneler aras�nda yerini al." },
		{ 83, "Level 83 oldun! Zirvedesin, tebrikler!" },
		{ 85, "Level 85 oldun! Maksimum seviyeye ula�t�n!" }
	};

	auto it = levelNotices.find(level);
	if (it != levelNotices.end())
	{
		std::string PMName = "Level K&H List";
		std::string nationName = (GetNation() == KARUS) ? "Karus" : "Human";

		Packet result;

		std::string lineMsg = "----------------------------------------------";
		std::string levelMsg = string_format("%s : [%s] %s", nationName.c_str(), GetName().c_str(), it->second.c_str());

		// �izgi + mesaj g�nderimi
		ChatPacket::Construct(&result, ChatType::PRIVATE_CHAT, &lineMsg, &PMName);
		g_pMain->Send_All(&result);

		ChatPacket::Construct(&result, ChatType::PRIVATE_CHAT, &levelMsg, &PMName);
		g_pMain->Send_All(&result);
	}

	//######################################################################################################

	if (!bLevelUp)
		m_bLevel = level;

	m_iMaxExp = g_pMain->GetExpByLevel(level, GetRebirthLevel());

	SetUserAbility();
	m_sMp = m_MaxMp;
	HpChange(GetMaxHealth());

	if (!m_flashtime && (m_FlashExpBonus > 0 || m_FlashDcBonus > 0 || m_FlashWarBonus > 0))
		SetFlashTimeNote(true);

	Packet result(WIZ_LEVEL_CHANGE);
	result << GetSocketID()
		<< GetLevel() << m_sPoints << m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree]
		<< m_iMaxExp << m_iExp
		<< m_MaxHp << m_sHp
		<< m_MaxMp << m_sMp
		<< m_sMaxWeight << m_sItemWeight;

	g_pMain->Send_Region(&result, GetMap(), GetRegionX(), GetRegionZ(), nullptr, GetEventRoom());
	if (isInParty())
	{
		// TODO: Move this to party specific code
		result.Initialize(WIZ_PARTY);
		result << uint8(PARTY_LEVELCHANGE) << GetSocketID() << GetLevel();
		g_pMain->Send_PartyMember(GetPartyID(), &result);

		if (m_bIsChicken)
			GrantChickenManner();
	}

	// We should kick players out of the zone if their level no longer matches the requirements for this zone.
	if (GetLevel() < GetMap()->GetMinLevelReq() || GetLevel() > GetMap()->GetMaxLevelReq())
		KickOutZoneUser();

#if(GAME_SOURCE_VERSION == 1098)
	if (level == 30 && ChickenStatus == 0)
	{
		SendMyInfo();
		UserInOut(INOUT_OUT);
		SetRegion(GetNewRegionX(), GetNewRegionZ());
		UserInOut(INOUT_WARP);
		g_pMain->UserInOutForMe(this);
		NpcInOutForMe();
		g_pMain->MerchantUserInOutForMe(this);

		ResetWindows();
		InitType4();
		RecastSavedMagic();

		GoldGain(100000);  // civciv patlay�nca verilen para �d�l� 500K
		ChickenStatus = 1;
		// Disabled festive explosion effect on rank/level visuals in Moradon.
		// ShowEffect(490092);

		bool genie = false;
		if (isGenieActive()) {
			GenieStop(); genie = true;
		}

		if (genie) GenieStart();
	}

#endif
	SendPresetReqMoney();
}

void CGameServerDlg::SendGameNotice(ChatType chattype,
	std::string n, std::string sender, bool send_all,
	CUser* pUser, bool anontip, uint8 zoneid)
{
	if (n.empty())
		return;

	Packet result;
	if (anontip)
		g_pMain->GetServerResource(IDP_ANNOUNCEMENT, &n, n.c_str());

	if (pUser)
	{
		uint8 bNation = pUser->GetNation();
		ChatPacket::Construct(&result, (uint8)chattype, &n, &sender, bNation);
		pUser->Send(&result);
		return;
	}

	g_pMain->m_socketMgr.GetLock().lock();
	SessionMap sessMap = g_pMain->m_socketMgr.GetActiveSessionMap();
	g_pMain->m_socketMgr.GetLock().unlock();
	foreach(itr, sessMap) {
		CUser* user = TO_USER(itr->second);
		if (user == nullptr || !user->isInGame())
			continue;

		uint8 bNation = user->GetNation(), bZoneID = user->GetZoneID();
		if (zoneid && zoneid != bZoneID)
			continue;

		ChatPacket::Construct(&result, (uint8)chattype, &n, &sender, bNation);
		user->Send(&result);
	}
}

#pragma region CUser::ExpSealHandler(Packet & pkt)
void CUser::ExpSealHandler(Packet & pkt)
{
	uint8 bOpcode;

	pkt >> bOpcode;
	
	if (bOpcode == 1)
		ExpSealUpdateStatus(1);
	else if (bOpcode == 2)
		ExpSealUpdateStatus(0);
	else if (bOpcode == 3)
		ExpSealSealedPotion(pkt);
}
#pragma endregion

#pragma region CUser::ExpSealUpdateStatus(uint8 status)
void CUser::ExpSealUpdateStatus(uint8 status)
{
	Packet result(WIZ_EXP_SEAL);
	bExpSealStatus = status;
	result << uint8(status == 1 ? 1 : 2)
		<< uint8(1);
	Send(&result);
}
#pragma endregion

//npckill
void CUser::SendNpcKillID(uint32 sNpcID)
{
	/*
		The Spirit Of logos
	*/

	CNpc *pNpc = g_pMain->GetNpcPtr(sNpcID, GetZoneID());

	if (pNpc)
		//g_pMain->KillNpc(sNpcID, GetZoneID(), this);
		g_pMain->KillNpc(sNpcID, GetZoneID());
}

#pragma endregion


#pragma region CUser::ExpSealSealedPotion(Packet & pkt)
#define SEALED_JAR		810354000
#define SEALED_JAR_500M 810355000
#define SEALED_JAR_1B	810356000
#define SEALED_JAR_100M 810357000

void CUser::ExpSealSealedPotion(Packet & pkt)
{
	uint32 JarItemId;
	uint64 SealedExp;
	uint8 bPos;
	pkt >> JarItemId >> bPos >> SealedExp;
	_ITEM_TABLE pTItem = _ITEM_TABLE();
	Packet result(WIZ_EXP_SEAL, uint8(0x03));

	if (JarItemId != SEALED_JAR)
		goto fail_return;

	_ITEM_DATA* pItem = GetItem(bPos + SLOT_MAX);

	if (pItem == nullptr || pItem->nNum != JarItemId)
		goto fail_return;

	if (SealedExp > m_iSealedExp)
		goto fail_return;

	memset(pItem, 0x00, sizeof(_ITEM_DATA));
	SendStackChange(0, 0, 0, bPos + SLOT_MAX, true, 0, 0);

	if (SealedExp == 100000000)
		JarItemId = SEALED_JAR_100M;
	else if (SealedExp == 500000000)
		JarItemId = SEALED_JAR_500M;
	else if (SealedExp == 1000000000)
		JarItemId = SEALED_JAR_1B;
	else
		goto fail_return;

	pTItem = g_pMain->GetItemPtr(JarItemId);
	if (pTItem.isnull())
		goto fail_return;

	uint8 pos = GiveItem("Seal Experience Item", JarItemId, 1, true);
	m_iSealedExp -= (uint32) SealedExp;
	ExpSealChangeExp(0);

	result << JarItemId << pos << uint32(SealedExp) << uint32(0);
fail_return:
	Send(&result);
}
#pragma endregion

#pragma region CUser::ExpSealChangeExp(uint64 amount)
void CUser::ExpSealChangeExp(uint64 amount)
{
	if (m_iSealedExp >= 1000000000 && amount > 0)
	{
		ExpSealUpdateStatus(0);
		ExpChange("exp seal", amount);
		return;
	}

	m_iSealedExp += (uint32) amount;
	if (m_iSealedExp >= 1000000000)
		m_iSealedExp = 1000000000;
	else if (m_iSealedExp <= 0)
		m_iSealedExp = 0;


	Packet result(WIZ_EXP_SEAL, uint8(0x04));
	result << m_iSealedExp << uint32(0);
	Send(&result);

	result.clear();
	result.Initialize(WIZ_EXP_CHANGE);
	result << uint8(1) << m_iExp; // NOTE: Use proper flag
	Send(&result);
}
#pragma endregion

void CUser::ExpFlash()
{
	if (m_FlashExpBonus >= 100)
		return;

	if (GetPremium() == 11 || GetPremium() == 7) {
		SendFlashNotice(true);
		m_FlashWarBonus = 0;
		m_FlashDcBonus = 0;
		m_FlashExpBonus += 10;
		
		if (m_FlashExpBonus > 100) 
			m_FlashExpBonus = 100;

		if (g_pMain->pServerSetting.flashtime && !m_flashtime || (m_flashtype != 1))
			m_flashtime = g_pMain->pServerSetting.flashtime;

		if (g_pMain->pServerSetting.flashtime && m_flashcount < 10)
		

		m_flashtype = 1;
		SendFlashNotice();
	}
}

void CUser::DcFlash()
{
	if (m_FlashDcBonus >= 100)
		return;

	if (GetPremium() == 10 || GetPremium() == 7) {
		SendFlashNotice(true);
		m_FlashExpBonus = 0;
		m_FlashWarBonus = 0;
		m_FlashDcBonus += 10;
		if (m_FlashDcBonus > 100)  m_FlashDcBonus = 100;

		if (g_pMain->pServerSetting.flashtime && !m_flashtime || (m_flashtype != 2))
			m_flashtime = g_pMain->pServerSetting.flashtime;

		if (g_pMain->pServerSetting.flashtime && m_flashcount < 10)
			m_flashcount++;

		m_flashtype = 2;
		SendFlashNotice();
	}
}

void CUser::WarFlash()
{
	if (m_FlashWarBonus >= 10)
		return;

	if (GetPremium() == 12 || GetPremium() == 7) {
		SendFlashNotice(true);
		m_FlashExpBonus = 0;
		m_FlashDcBonus = 0;
		m_FlashWarBonus += 1;
		if (m_FlashWarBonus > 10) m_FlashWarBonus = 10;

		if (g_pMain->pServerSetting.flashtime && !m_flashtime || (m_flashtype != 3))
			m_flashtime = g_pMain->pServerSetting.flashtime;

		if (g_pMain->pServerSetting.flashtime && m_flashcount < 10)
			m_flashcount++;
		
		m_flashtype = 3;
		SendFlashNotice();
	}
}

void CUser::SendFlashNotice(bool isRemove)
{
	string header = "", descption = "";
	if (m_FlashExpBonus > 0)
		header = string_format("Exp +%d%%", m_FlashExpBonus);
	else if (m_FlashDcBonus > 0)
		header = string_format("Item Drop +%d%%", m_FlashDcBonus);
	else if (m_FlashWarBonus > 0)
		header = string_format("Cont +%d", m_FlashWarBonus);

	descption = header;
	if (!descption.empty() && m_flashtime) descption.append(string_format(" .remaining time %d", m_flashtime));

	if (header.empty()) 
		return;

	Packet result(WIZ_NOTICE);
	result.DByte();
	result << uint8(4) << uint8(isRemove == true ? 2 : 1) << header << descption;
	Send(&result);
}

bool CUser::JackPotNoah(uint32 gold)
{
	if (m_jackpotype != 2 || !gold)
		return false;

	auto jack = g_pMain->pJackPot[1];
	if (!jack.rate)
		return false;

	if (myrand(0, 10000) > jack.rate)
		return false;

	Packet newpkt(WIZ_GOLD_CHANGE, uint8(CoinEvent));

	int rand_ = myrand(0, 10000), xrand = 0;
	if (rand_ < jack._1000) xrand = 1000;
	else if (rand_ < jack._500) xrand = 500;
	else if (rand_ < jack._100) xrand = 100;
	else if (rand_ < jack._50) xrand = 50;
	else if (rand_ < jack._10) xrand = 10;
	else if (rand_ < jack._2) xrand = 2;
	if (xrand == 0)
		return false;

	uint32 m_bGoldEvent = gold * xrand;
	newpkt << uint16(740) << uint16(0) << uint32(0) << uint16(xrand) << GetSocketID();
	SendToRegion(&newpkt, nullptr, GetEventRoom());
	GoldGain(m_bGoldEvent, false, true);

	if (rand_ == 500 || rand_ == 1000) {
		// JstKO Jackpot Coin Patlad� Notice Eklendi. 04.01.2025
		std::string notice = string_format("Oyuncu : %s Adl� Karakter ��in JackPot COIN Patlad� %dx Hayirli Olsun.", GetName().c_str(), xrand);
		Packet result;
		ChatPacket::Construct(&result, (uint8)ChatType::GENERAL_CHAT, &notice);
		g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL);
	}

	return true;
}

bool CUser::JackPotExp(int64 iExp)
{
	if (GetLevel() >= g_pMain->m_byMaxLevel && m_iExp >= m_iMaxExp)
		return true;

	if (m_jackpotype != 1 || iExp == 0)
		return false;

	auto jack = g_pMain->pJackPot[0];
	if (!jack.rate)
		return false;

	if (myrand(0, 10000) > jack.rate)
		return false;

	Packet newpkt(WIZ_EXP_CHANGE, uint8(2));

	int rand_ = myrand(0, 10000), xrand = 0;
	if (rand_ < jack._1000) xrand = 1000;
	else if (rand_ < jack._500) xrand = 500;
	else if (rand_ < jack._100) xrand = 100;
	else if (rand_ < jack._50) xrand = 50;
	else if (rand_ < jack._10) xrand = 10;
	else if (rand_ < jack._2) xrand = 2;
	if (xrand == 0)
		return false;

	uint64 m_bExpEvent = iExp * xrand;

	newpkt << GetSocketID() << xrand << m_bExpEvent;
	SendToRegion(&newpkt, nullptr, GetEventRoom());
	ExpChange("jack pot", m_bExpEvent);

	if (xrand == 500 || xrand == 1000) {
		// JstKO Jackpot EXP Patlad� Notice Eklendi. 04.01.2025
		std::string notice = string_format("Oyuncu : %s Adl� Karakter ��in JackPot EXP Patlad� %dx Hayirli Olsun.", GetName().c_str(), xrand);
		Packet result;
		ChatPacket::Construct(&result, (uint8)ChatType::GENERAL_CHAT, &notice);
		g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL);
	}
	
	return true;
}
