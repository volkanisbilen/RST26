#include "stdafx.h"

void CUser::SurroundingAllInfo(bool first, bool change)
{

	if (!m_bZoneChangeFlag && first && isBottomSign)
		return;

	if (!first && !change && m_surallinfotime && m_surallinfotime > UNIXTIME2)
		return;

	if (!first)
		m_surallinfotime = UNIXTIME2 + 5000;

	if (!isBottomSign)
		isBottomSign = true;

	{
		Packet result(WIZ_USER_INFORMATIN, uint8(1));
		result.SByte();
		result << uint8(1) << GetZoneID() << uint8(0) << uint16(0);
		Send(&result);
	}

	struct _info
	{
		CUser* pUser;
		float distance;
		_info(CUser* pUser, float distance)
		{
			this->pUser = pUser;
			this->distance = distance;
		}
	};

	struct _info2
	{
		CBot* pUser;
		float distance;
		_info2(CBot* pUser, float distance)
		{
			this->pUser = pUser;
			this->distance = distance;
		}
	};

	std::vector < _info> mlist; std::vector < _info2> mlist2;
	mlist.push_back(_info(this, 0.0f));

	uint16 sCount = 0;
	Packet result(WIZ_USER_INFORMATIN, uint8(first ? 1 : 3));
	result << uint8(1) << GetZoneID() << uint8(0) << sCount;

	for (int i = 0; i < MAX_USER; i++)
	{
		if (sCount >= 800)
			break;

		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame() || pUser->GetZoneID() != GetZoneID())
			continue;

		if ((!isGM() && pUser->isGM()) || GetEventRoom() != pUser->GetEventRoom())
			continue;

		if (pUser->GetZoneID() == ZONE_CHAOS_DUNGEON || pUser->GetZoneID() == ZONE_PRISON
			|| pUser->GetZoneID() == ZONE_KNIGHT_ROYALE || pUser->GetZoneID() == ZONE_DUNGEON_DEFENCE)
			continue;

		if (pUser->GetSocketID() == GetSocketID())
			continue;

		float fDis = 0.0f;
		if (!isGM())
			fDis = Unit::GetDistanceSqrt(pUser);

		if (fDis > 0.0f && fDis > 300.0f)
			continue;

		mlist.push_back(_info(pUser, fDis));
	}

	g_pMain->m_MapBotList.m_lock.lock();
	auto m_sMapBotListArray = g_pMain->m_MapBotList.m_UserTypeMap;
	g_pMain->m_MapBotList.m_lock.unlock();

	foreach(itr, m_sMapBotListArray)
	{
		CBot* pUser = itr->second;
		if (pUser == nullptr || !pUser->isInGame() || pUser->GetZoneID() != GetZoneID())
			continue;

		if (pUser->GetZoneID() == ZONE_CHAOS_DUNGEON || pUser->GetZoneID() == ZONE_PRISON
			|| pUser->GetZoneID() == ZONE_KNIGHT_ROYALE || pUser->GetZoneID() == ZONE_DUNGEON_DEFENCE)
			continue;

		if (pUser->GetID() == GetSocketID())
			continue;

		float fDis = 0.0f;
		/*if (!isGM())
			fDis = Unit::GetDistanceSqrt(pUser);*/

		if (fDis > 0.0f && fDis > 300.0f)
			continue;

		mlist2.push_back(_info2(pUser, fDis));

	}

	if (mlist.empty() && mlist2.empty())
	{
		Send(&result);
		return;
	}

	std::sort(mlist.begin(), mlist.end(), [](auto const& a, auto const& b) { return a.distance < b.distance; });

	foreach(itr, mlist)
	{
		CUser* pUser = itr->pUser;
		if (!pUser) continue;

		result.SByte();
		result << pUser->GetName() << pUser->GetNation();
		result << uint16(1) << pUser->GetSPosX() << pUser->GetSPosZ() << pUser->GetClanID();

		CKnights* pKnights = nullptr;
		if (pUser->isInClan())
			pKnights = g_pMain->GetClanPtr(pUser->GetClanID());

		if (pKnights)
			result << pKnights->m_sMarkVersion << uint8(0) << uint8(0);
		else
			result << uint16(0) << uint16(0);

		result << uint16(1);
		sCount++;
	}

	foreach(itr, mlist2)
	{
		CBot* pUser = itr->pUser;
		if (!pUser)
			continue;

		result.SByte();
		result << pUser->GetName() << pUser->GetNation();
		result << uint16(1) << pUser->GetSPosX() << pUser->GetSPosZ() << pUser->GetClanID();
		result << uint16(0) << uint16(0);

		result << uint16(1);
		sCount++;
	}

	result.put(4, sCount);
	SendCompressed(&result);
}

void CUser::BottomLeftRegionUserList(Packet& pkt)
{
	uint8 opcode; pkt >> opcode;
	if (opcode == (uint8)BottomUserListOpcode::UserInfoDetail && (isTrading() || isMerchanting()
		|| isFishing() || isMining() || isSellingMerchantingPreparing()))
		return;

#if(GAME_SOURCE_VERSION == 1098)
	if (opcode != (uint8)BottomUserListOpcode::UserInfoDetail
		&& opcode != (uint8)BottomUserListOpcode::unknow) return;
#endif

	switch ((BottomUserListOpcode)opcode)
	{
	case BottomUserListOpcode::Sign:
		SurroundingAllInfo(true);
		break;
	case BottomUserListOpcode::UserInfoDetail:
		HandleBottomUserInfoDetail(pkt);
		break;
	case BottomUserListOpcode::UserList:
		SurroundingAllInfo();
		break;
	case BottomUserListOpcode::RegionDelete:
		BottomUserLogOut();
		break;
	case BottomUserListOpcode::unknow:
		HandleGetUserInfoView(pkt);
		break;
	}
}

void CUser::BottomUserLogOut()
{
#if(GAME_SOURCE_VERSION == 1098)
	return;
#endif
	Packet result(WIZ_USER_INFORMATIN, uint8(BottomUserListOpcode::RegionDelete));
	result.SByte(); result << GetName();
	g_pMain->Send_Zone(&result, GetZoneID(), this, (uint8)Nation::ALL, GetEventRoom());
}

void CUser::HandleBottomUserInfoDetail(Packet& pkt)
{
	Packet result(WIZ_USER_INFORMATIN, uint8(UserInfoDetail));
	Packet resultPlayer(WIZ_ITEM_UPGRADE, uint8_t(9));
	std::string strCharName;
	pkt.SByte();
	pkt >> strCharName;
	CKnights* pKnights = nullptr;

	CUser* pUser = g_pMain->GetUserPtr(strCharName, NameType::TYPE_CHARACTER);
	if (pUser != nullptr)
	{
		if (!isGM() && pUser->isGM())
			return;

		resultPlayer.SByte();
		resultPlayer << uint8_t(4) << uint8_t(1)
			<< pUser->GetName() << pUser->GetNation() << pUser->GetRace() << pUser->GetClass() << pUser->GetLevel() << uint32_t(pUser->GetLoyalty())
			<< pUser->GetStat(StatType::STAT_STR) << pUser->GetStat(StatType::STAT_STA) << pUser->GetStat(StatType::STAT_DEX) << pUser->GetStat(StatType::STAT_INT) << pUser->GetStat(StatType::STAT_CHA)
			<< pUser->GetCoins()
			<< pUser->m_sPoints
			<< uint8_t(0) << uint16_t(0)
			<< pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat1] << pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat2] << pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat3] << pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointMaster];

		for (int i = 0; i < SLOT_MAX + HAVE_MAX; i++)
		{
			_ITEM_DATA* pItem = pUser->GetItem(i);
			resultPlayer << pItem->nNum << pItem->sDuration << pItem->sCount << pItem->bFlag;
		}

		resultPlayer << pUser->GetRebirthLevel();
		Send(&resultPlayer);

		// JstKO Bottom User List View Notice Eklendi. 02.02.2025
		if (isGM() || isGMUser()) // Gm View �zle �zin Yok Yasak Bakma Sanane Aq :D
		{
			std::string strCharID1 = "", strCharID2 = "", strCharID3 = "", strCharID4 = "";
			g_DBAgent.GetAllCharID(pUser->m_strAccountID, strCharID1, strCharID2, strCharID3, strCharID4);

			if (strCharID1.length() < 2)
				strCharID1 = "Bos Char";

			if (strCharID2.length() < 2)
				strCharID2 = "Bos Char";

			if (strCharID3.length() < 2)
				strCharID3 = "Bos Char";

			if (strCharID4.length() < 2)
				strCharID4 = "Bos Char";

			// JstKO UserInfoView Notice Eklendi. 02.02.2025
			SendChat(PUBLIC_CHAT, string_format("<< { %s } Isimli Kullanici Bilgileri Asagidaki Gibidir. >>", pUser->GetName().c_str()));
			SendChat(GENERAL_CHAT, string_format("<< Account Name : [ %s ] >>", pUser->GetAccountName().c_str()));
			SendChat(GM_CHAT, string_format("<< IP Adress : [ %s ] >>", pUser->GetRemoteIP().c_str()));
			SendChat(MERCHANT_CHAT, string_format("<< Total Char : [ %s ] | [ %s ] | [ %s ] | [ %s ] >>", strCharID1.c_str(), strCharID2.c_str(), strCharID3.c_str(), strCharID4.c_str()));
			SendChat(SHOUT_CHAT, string_format("<< HP : [ %d ] | MP : [ %d ] >>", pUser->GetMaxHealth(), pUser->GetMaxMana()));
			SendChat(FORCE_CHAT, string_format("<< Sol Np : [ %d ] | Sa� Np : [ %d ] >>", pUser->GetLoyalty(), pUser->GetMonthlyLoyalty()));
			SendChat(KNIGHTS_CHAT, string_format("<< Cash Point : [ %d ] | TL : [ %d ] | Para : [ %d ] >>", pUser->GetCash(), pUser->m_nTLBalance, pUser->GetCoins()));

			// -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

			DateTime time;
			g_pMain->SendHelpDescription(this, string_format("<< Kullan�c�n�n �ld�rme ve �lme Sayisi 0 Oldugu Icin Kd Orani Hesaplanamadi >>"));
			g_pMain->SendHelpDescription(this, string_format("<< Toplam �ld�rme Say�s� : [ %d ] | Toplam �lme Say�s� : [ %d ] >>", pUser->m_AchieveInfo.UserDefeatCount, pUser->m_AchieveInfo.UserDeathCount));
			g_pMain->SendHelpDescription(this, string_format("<< Monster �ld�rme Say�s� : [ %d ] >>", pUser->m_AchieveInfo.MonsterDefeatCount));
			g_pMain->SendHelpDescription(this, string_format("<< Server Time : [ %04d-%02d-%02d at %02d:%02d ] >>", time.GetYear(), time.GetMonth(), time.GetDay(), time.GetHour(), time.GetMinute()));
			
		}
		// JstKO Bottom User List View Notice Eklendi. The End 02.02.2025

		/*
		Warrior -> 334,359,365
		Rogue -> 335,347,360,366
		Mage -> 336,348,361,367,516
		Priest -> 337,349,357,362,363,364,368
		*/
		if (true)
		{
			Packet result(XSafe, uint8_t(XSafeOpCodes::ShowQuestList));
			result << uint16(334) << (pUser->GetQuestStatus(334) == 2 ? uint8(2) : uint8(1));
			result << uint16(359) << (pUser->GetQuestStatus(359) == 2 ? uint8(2) : uint8(1));
			result << uint16(365) << (pUser->GetQuestStatus(365) == 2 ? uint8(2) : uint8(1));
			result << uint16(335) << (pUser->GetQuestStatus(335) == 2 ? uint8(2) : uint8(1));
			result << uint16(347) << (pUser->GetQuestStatus(347) == 2 ? uint8(2) : uint8(1));
			result << uint16(360) << (pUser->GetQuestStatus(360) == 2 ? uint8(2) : uint8(1));
			result << uint16(366) << (pUser->GetQuestStatus(366) == 2 ? uint8(2) : uint8(1));
			result << uint16(336) << (pUser->GetQuestStatus(336) == 2 ? uint8(2) : uint8(1));
			result << uint16(348) << (pUser->GetQuestStatus(348) == 2 ? uint8(2) : uint8(1));
			result << uint16(361) << (pUser->GetQuestStatus(361) == 2 ? uint8(2) : uint8(1));
			result << uint16(367) << (pUser->GetQuestStatus(367) == 2 ? uint8(2) : uint8(1));
			result << uint16(516) << (pUser->GetQuestStatus(516) == 2 ? uint8(2) : uint8(1));
			result << uint16(337) << (pUser->GetQuestStatus(337) == 2 ? uint8(2) : uint8(1));
			result << uint16(349) << (pUser->GetQuestStatus(349) == 2 ? uint8(2) : uint8(1));
			result << uint16(357) << (pUser->GetQuestStatus(357) == 2 ? uint8(2) : uint8(1));
			result << uint16(362) << (pUser->GetQuestStatus(362) == 2 ? uint8(2) : uint8(1));
			result << uint16(363) << (pUser->GetQuestStatus(363) == 2 ? uint8(2) : uint8(1));
			result << uint16(364) << (pUser->GetQuestStatus(364) == 2 ? uint8(2) : uint8(1));
			result << uint16(368) << (pUser->GetQuestStatus(368) == 2 ? uint8(2) : uint8(1));
			for (int i = 0; i < PERK_COUNT; i++)
				result << pUser->pPerks.perkType[i];
			Send(&result);
		}
		// YOLLAT
	}
	else
	{
		CBot* pBot = g_pMain->GetBotPtr(strCharName, NameType::TYPE_CHARACTER);
		if (pBot != nullptr)
		{
			resultPlayer.SByte();
			resultPlayer << uint8_t(4) << uint8_t(1)
				<< pBot->GetName() << pBot->GetNation() << pBot->GetRace() << pBot->GetClass() << pBot->GetLevel() << pBot->GetLoyalty()
				<< pBot->GetStat(StatType::STAT_STR) << pBot->GetStat(StatType::STAT_STA) << pBot->GetStat(StatType::STAT_DEX) << pBot->GetStat(StatType::STAT_INT) << pBot->GetStat(StatType::STAT_CHA)
				<< pBot->GetCoins()
				<< pBot->m_sPoints
				<< uint8_t(0) << uint16_t(0)
				<< pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat1] << pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat2] << pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat3] << pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointMaster];

			for (int i = 0; i < SLOT_MAX + HAVE_MAX; i++)
			{
				_ITEM_DATA* pItem = pBot->GetItem(i);
				resultPlayer << pItem->nNum << pItem->sDuration << pItem->sCount << pItem->bFlag;
			}

			resultPlayer << pBot->GetRebirthLevel();
			Send(&resultPlayer);

			// JstKO Bottom Bot List View Notice Eklendi. 02.02.2025
			if (isGM() || isGMUser()) // Gm View �zle �zin Yok Yasak Bakma Sanane Aq :D
			{
				// JstKO BotInfoView Notice Eklendi. 02.02.2025
				SendChat(PUBLIC_CHAT, string_format("<< { %s } Bot Kullanici Bilgileri Asagidaki Gibidir. >>", pBot->GetName().c_str()));
				SendChat(SHOUT_CHAT, string_format("<< HP : [ %d ] | MP : [ %d ] >>", pBot->GetMaxHealth(), pBot->GetMaxMana()));
				SendChat(FORCE_CHAT, string_format("<< Sol Np : [ %d ] | Sa� Np : [ %d ] >>", pBot->GetLoyalty(), pBot->GetMonthlyLoyalty()));
				SendChat(KNIGHTS_CHAT, string_format("<< Para : [ %d ] >>", pBot->GetCoins()));
				SendChat(PUBLIC_CHAT, string_format("<< Zaman Online : [ Sn : %d ] >>", (uint32(UNIXTIME) - m_bAchieveLoginTime)));
			}
			// JstKO Bottom Bot List View Notice Eklendi. The End 02.02.2025
		}
	}
}

void CUser::HandleGetUserInfoView(Packet& pkt)
{
	uint16 charid;
	pkt >> charid;

	CBot* pBot = g_pMain->GetBotPtr(charid);
	if (pBot != nullptr)
	{
		Packet result(WIZ_ITEM_UPGRADE);
		result << uint8(ITEM_CHARACTER_SEAL) << uint8(CharacterSealOpcodes::Preview) << uint8(0x01);
		result.SByte();
		result << pBot->GetName()
			<< pBot->GetNation()
			<< pBot->GetRace()
			<< pBot->GetClass()
			<< pBot->GetLevel()
			<< pBot->GetLoyalty()
			<< pBot->m_bStats[(uint8)StatType::STAT_STR] << pBot->m_bStats[(uint8)StatType::STAT_STA]
			<< pBot->m_bStats[(uint8)StatType::STAT_DEX] << pBot->m_bStats[(uint8)StatType::STAT_INT] << pBot->m_bStats[(uint8)StatType::STAT_CHA]
			<< pBot->m_iGold
			<< pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree]
			<< uint32(0x01)
			<< pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat1] << pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat2]
			<< pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat3] << pBot->m_bstrSkill[(uint8)SkillPointCategory::SkillPointMaster];

		for (int i = 0; i < INVENTORY_COSP; i++)
		{
			auto* pItem = pBot->GetItem(i);
			if (pItem == nullptr)
				continue;

			result << pItem->nNum << pItem->sDuration << pItem->sCount << pItem->bFlag;
		}

		result << pBot->GetRebirthLevel();

		Send(&result);
	}

	auto* pUser = g_pMain->GetUserPtr(charid);
	if (pUser == nullptr)
		return;

	Packet result(WIZ_ITEM_UPGRADE);
	result << uint8(ITEM_CHARACTER_SEAL) << uint8(CharacterSealOpcodes::Preview) << uint8(0x01);
	result.SByte();
	result << pUser->GetName()
		<< pUser->GetNation()
		<< pUser->GetRace()
		<< pUser->GetClass()
		<< pUser->GetLevel()
		<< pUser->GetLoyalty()
		<< pUser->m_bStats[(uint8)StatType::STAT_STR] << pUser->m_bStats[(uint8)StatType::STAT_STA]
		<< pUser->m_bStats[(uint8)StatType::STAT_DEX] << pUser->m_bStats[(uint8)StatType::STAT_INT] << pUser->m_bStats[(uint8)StatType::STAT_CHA]
		<< pUser->m_iGold
		<< pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree]
		<< uint32(0x01)
		<< pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat1] << pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat2]
		<< pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat3] << pUser->m_bstrSkill[(uint8)SkillPointCategory::SkillPointMaster];

	for (int i = 0; i < INVENTORY_COSP; i++)
	{
		auto* pItem = pUser->GetItem(i);
		if (pItem == nullptr)
			continue;

		result << pItem->nNum << pItem->sDuration << pItem->sCount << pItem->bFlag;
	}

	result << pUser->GetRebirthLevel();

	Send(&result);
}