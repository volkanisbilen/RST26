#include "stdafx.h"
#include "DBAgent.h"
#include "../shared/Ini.h"

//##################################################################################################
//# Merhaba Arkada�lar Benim Sezer ��itme Engelliyim Duymuyorum ve Konu�ma Bana Destek Ol L�ften Siz Seviyoruz.  #
//##################################################################################################

// JstKO SendInfoNotice Chat Notice Eklendi 02.01.2025
#pragma region CUser::JstKOSendInfoNotice()
void CUser::JstKOSendInfoNotice()
{
	Packet JstKO; // JstKO Chat Notice Herkes G�rebilirisiniz Eklendi 02.01.2025
	std::string Chat_Notice = string_format("#### Hosgeldiniz JstKO Server Online. Hepinize Iyi Oyunlar. ####");
	JstKO.clear();
	ChatPacket::Construct(&JstKO, ChatType::PUBLIC_CHAT, Chat_Notice.c_str(), "[JstKO]", GetNation());
	Send(&JstKO);
	//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
	Packet JstKO1; // JstKO1 Chat Notice Herkes G�rebilirisiniz Eklendi 02.01.2025
	std::string Chat_Notice1 = string_format("#### Guvenliginiz Icin Sifrenizi Kimseyle Paylasmayin Dolandirilmamak Adina Aracilik ile Islem Yapmanizi Oneririz. ####");
	JstKO1.clear();
	ChatPacket::Construct(&JstKO1, ChatType::ALLIANCE_CHAT, Chat_Notice1.c_str(), "[SYSTEM]", GetNation());
	Send(&JstKO1);	
}
#pragma endregion
// JstKO SendInfoNotice Chat Notice Eklendi The End 02.01.2025

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO GmInfoKomutNotice GM&KING Info Komutlar Eklendi 02.01.2025
#pragma region CUser::JstKOGmInfoKomutNotice()
void CUser::JstKOGmInfoKomutNotice()
{
	if (isGM() && g_pMain->GmInfoKomutNotice) // JstKO GM Info Komutlar Eklendi 02.01.2025
	{
		g_pMain->SendHelpDescription(this, string_format("{[GM]} : +help Yazarak GameMaster Komutlar�n� G�rebilirsiniz."));
	}
	if (isKing() && g_pMain->GmInfoKomutNotice) // JstKO King Info Komutlar Eklendi 02.01.2025
		g_pMain->SendHelpDescription(this, string_format("{[KING]} : +kralhelp Yazarak Kral&Patron Komutlar�n� G�rebilirsiniz."));
}
#pragma endregion
// JstKO GmInfoKomutNotice GM&KING Info Komutlar Eklendi The End 02.01.2025

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO ServerSettings Ayarlar Eklendi 01.01.2025
#pragma region CGameServerDlg::JstKOServerSettings()
void CGameServerDlg::JstKOServerSettings()
{
	CIni JstKO(CONF_DEAFSOFT); // JstKO Files x64 JstKOSettings.ini Ayarlar Eklendi 01.01.2025 

	// JstKO Detayli Release Log Sistemi. 1 = Aktif, 0 = Pasif.
	JstKODetailedLogSystem = JstKO.GetBool("JSTKO_LOG_SYSTEM", "STATUS", false);
	ServerLog("SERVER", "JstKO detailed log system enabled. ini=%s", CONF_DEAFSOFT);

	// JstKO Clan Create Notice Settings Eklendi 01.01.2025
	ClanCreateMinNationalPoints = JstKO.GetInt("CLAN_CREATE_SETTINGS", "MIN_NP", 1000); // JstKO Yeni Klan Kurabilmek Np Ayarlar
	ClanCreateMinLevel = JstKO.GetInt("CLAN_CREATE_SETTINGS", "MIN_LEVEL", CLAN_LEVEL_REQUIREMENT); // JstKO Yeni Klan Kurabilmek Level Ayarlar
	ClanCreateCoins = JstKO.GetInt("CLAN_CREATE_SETTINGS", "COINS", CLAN_COIN_REQUIREMENT); // JstKO Yeni Klan Kurabilmek Para Ayarlar

	// JstKO Yeni Cz User Pvp Kill �d�l Notice Eklendi. 12.04.2025
	KillGoldGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_GOLD_GIFT", 250000); // Para : 250,000 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	KillCashGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_CASH_GIFT", 25); // Cash : 25 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill�temGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_ITEM_GIFT", 389205000); // Black Gem Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	KillCountGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_COUNT_GIFT", 1); // Adet : 1 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill50GoldGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_50GOLD_GIFT", 500000); // Para : 500,000 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill50CashGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_50CASH_GIFT", 50); // Cash : 50 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill50�temGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_50ITEM_GIFT", 389199000); // Blue Gem Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill50CountGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_50COUNT_GIFT", 5); // Adet : 5 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill500GoldGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_500GOLD_GIFT", 50000000); // Para : 50,000,000 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill500CashGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_500CASH_GIFT", 500); // Cash : 500 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill500�temGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_500ITEM_GIFT", 389201000); // Green Gem Eklendi. Files x64 JstKOSettings.ini. 12.04.2025
	Kill500CountGift = JstKO.GetInt("PVP_KILL_GIFT_SETTINGS", "KILL_500COUNT_GIFT", 10); // Adet : 10 Eklendi. Files x64 JstKOSettings.ini. 12.04.2025

	// JstKO Oyun Rehber Bilgisi Komutlar�n� Settings Eklendi 07.01.2025
	UserGameInfoKomutNotice = JstKO.GetBool("USER_GAME_INFO_KOMUT_NOTICE", "STATUS", true); // Files x64 JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO Gm&K�ng Info Komut Settings Eklendi 02.01.2025
	GmInfoKomutNotice = JstKO.GetBool("GM_INFO_KOMUT_NOTICE", "STATUS", true); // Files x64  JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO Auto Online Count Notice Settings Eklendi 01.01.2025
	AutoOnlineCountNotice = JstKO.GetBool("AUTO_ONLINE_COUNT_NOTICE", "STATUS", true); // Files x64  JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO User Login �sOnline Notice Settings Eklendi 01.01.2025
	GirisUserisOnlineNotice = JstKO.GetBool("USER_LOGIN_ISONLINE_NOTICE", "STATUS", true); // Files x64  JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO Giri� Gate Zone RgB Notice Settings Eklendi 04.01.2025
	Giri�GateZoneNotice = JstKO.GetBool("GIRIS_GATE_ZONE_NOTICE", "STATUS", true); // Files x64  JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO User Login Effect Settings Eklendi 01.01.2025
	UserLoginEffect = JstKO.GetBool("USER_LOGIN_EFFECT", "STATUS", true); // Files x64 JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO User Deatch Effect Settings Eklendi 01.01.2025
	UserDeathEffect = JstKO.GetBool("USER_DEATH_EFFECT", "STATUS", true); // Files x64 JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO User Town Effect Settings Eklendi 01.01.2025
	UserTownEffect = JstKO.GetBool("USER_TOWN_EFFECT", "STATUS", true); // Files x64 JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO User Rise Effect Settings Eklendi 01.01.2025
	UserRiseEffect = JstKO.GetBool("USER_RISE_EFFECT", "STATUS", true); // Files x64 JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO User Zone Change Settings Effect Eklendi 01.01.2025
	UserZoneChangeEffect = JstKO.GetBool("USER_ZONE_CHANGE_EFFECT", "STATUS", true); // Files x64 JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

	// JstKO Auto King Name Update Settings Eklendi 01.01.2025
	AutoKingUpdateServerList = JstKO.GetBool("AUTO_KING_UPDATE_SERVER_LIST", "STATUS", true); // Files x64 JstKOSettings.ini 1 A�ip ve 0 Kapatma Ayarlar

}
#pragma endregion
// JstKO ServerSettings Ayarlar Eklendi The End 01.01.2025

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO GmisOnlineNotice Ayarlar Eklendi 30.12.2024
#pragma region CUser::JstKOGmisOnlineNotice()
void CUser::JstKOGmisOnlineNotice()
{
	// JstKO Giri� GM Online Notice Eklendi 30.12.2024
	if (isGM() && g_pMain->pServerSetting.GmisOnlineNotice) // Gm Sadece | Tablo Server_Settings GmisOnlineNotice Eklendi 1 A�ip ve 0 Kapatma Ayarlar Yeter :)
	{
		g_pMain->SendRgbNotice(string_format("GameMaster : %s is Online. Iyi Oyunlar Dileriz.", GetName().c_str()), 1, 255, 1); // Herkes User Chat Notice
		return; // Geri D�nmek Demek 
	}
	// JstKO Giri� GM Online Notice The End 30.12.2024
}
#pragma endregion
// JstKO GmisOnlineNotice Ayarlar Eklendi The End 30.12.2024

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO GmisOfflineNotice Ayarlar Eklendi 30.12.2024
#pragma region CUser::JstKOGmisOfflineNotice()
void CUser::JstKOGmisOfflineNotice()
{
	// JstKO ��kt� GM Offline Notice Eklendi 30.12.2024
	if (isGM() && g_pMain->pServerSetting.GmisOnlineNotice) // Gm Sadece | Tablo Server_Settings GmisOnlineNotice Eklendi 1 A�ip ve 0 Kapatma Ayarlar Yeter :)
	{
		g_pMain->SendRgbNotice(string_format("GameMaster : %s is Offline ��kt� Tekrar G�r�s�r�z.", GetName().c_str()), 255, 1, 1); // Herkes User Chat Notice Rgb K�rm�z� G�r�yor�z.
		return; // Geri D�nmek Demek 
	}
	// JstKO ��kt� GM Offline Notice The End 30.12.2024
}
#pragma endregion
// JstKO GmisOfflineNotice Ayarlar Eklendi The End 30.12.2024

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO LoginUserisOnlineNotice Ayarlar Eklendi 01.01.2025
#pragma region CUser::JstKOLoginUserisOnlineNotice()
void CUser::JstKOLoginUserisOnlineNotice()
{
	std::string sZoneName;
	switch (GetZoneID())
	{
	case ZONE_KARUS: sZoneName = "Luferson"; break;
	case ZONE_KARUS2: sZoneName = "Luferson"; break;
	case ZONE_KARUS3: sZoneName = "Luferson"; break;
	case ZONE_ELMORAD: sZoneName = "El Morad"; break;
	case ZONE_ELMORAD2: sZoneName = "El Morad"; break;
	case ZONE_ELMORAD3: sZoneName = "El Morad"; break;
	case ZONE_KARUS_ESLANT: sZoneName = "Karus Eslant"; break;
	case ZONE_KARUS_ESLANT2: sZoneName = "Karus Eslant"; break;
	case ZONE_KARUS_ESLANT3: sZoneName = "Karus Eslant"; break;
	case ZONE_ELMORAD_ESLANT: sZoneName = "El Morad Eslant"; break;
	case ZONE_ELMORAD_ESLANT2: sZoneName = "El Morad Eslant"; break;
	case ZONE_ELMORAD_ESLANT3: sZoneName = "El Morad Eslant"; break;
	case ZONE_MORADON: sZoneName = "Moradon"; break;
	case ZONE_MORADON2: sZoneName = "Moradon"; break;
	case ZONE_MORADON3: sZoneName = "Moradon"; break;
	case ZONE_DELOS: sZoneName = "Delos"; break;
	case ZONE_BIFROST: sZoneName = "Bifrost"; break;
	case ZONE_DESPERATION_ABYSS: sZoneName = "Desperation Abyss"; break;
	case ZONE_HELL_ABYSS: sZoneName = "Hell Abyss"; break;
	case ZONE_DRAGON_CAVE: sZoneName = "Dragon Cave"; break;
	case ZONE_ARENA: sZoneName = "Arena"; break;
	case ZONE_ORC_ARENA: sZoneName = "Orc Arena"; break;
	case ZONE_GOBLIN_ARENA: sZoneName = "Goblin Arena"; break;
	case ZONE_CAITHAROS_ARENA: sZoneName = "Caitharos Arena"; break;
	case ZONE_FORGOTTEN_TEMPLE: sZoneName = "Forgotten Temple"; break;
	case ZONE_RONARK_LAND: sZoneName = "Ronark Land"; break;
	default:
		break;
	}

	if (!isGM() && g_pMain->GirisUserisOnlineNotice) //JstKO Login User �sOnline Notice 01.01.2025
	{
		std::string UserisOnlineNotice;
		ShowEffect(490161); // JstKO Oyun Giri� Char Karus Effect Eklendi. 24.02.2025
		ShowEffect(490092); // JstKO Oyun Giri� Char Y�lba�i Ya�iyor Effect Eklendi. 24.02.2025
		if (GetNation() == KARUS)
			UserisOnlineNotice = string_format("KARUS Oyuncu: [%s] is Online [%s] - Hosgeldiniz JstKO, Iyi Oyunlar.", GetName().c_str(), sZoneName.c_str());
		else
		ShowEffect(490090); // JstKO Oyun Giri� Char Karus Effect Eklendi. 24.02.2025
		ShowEffect(490092); // JstKO Oyun Giri� Char Y�lba�i Ya�iyor Effect Eklendi. 24.02.2025
			UserisOnlineNotice = string_format("HUMAN Oyuncu: [%s] is Online [%s] - Hosgeldiniz JstKO, Iyi Oyunlar.", GetName().c_str(), sZoneName.c_str());
		g_pMain->SendChat<COMMAND_CHAT>(UserisOnlineNotice.c_str());
	}
	if (!isGM() && g_pMain->GirisUserisOnlineNotice) //JstKO Login User �sOnline Notice 01.01.2025
	{
		std::string UserisOnlineNotice1;
		if (GetNation() == KARUS)
			UserisOnlineNotice1 = string_format("KARUS Oyuncu: [%s] is Online [%s] - Hosgeldiniz JstKO, Iyi Oyunlar.", GetName().c_str(), sZoneName.c_str());
		else
			UserisOnlineNotice1 = string_format("HUMAN Oyuncu: [%s] is Online [%s] - Hosgeldiniz JstKO, Iyi Oyunlar.", GetName().c_str(), sZoneName.c_str());
		g_pMain->SendChat<KNIGHTS_CHAT>(UserisOnlineNotice1.c_str());
	}
}
#pragma endregion
// JstKO LoginUserisOnlineNotice Ayarlar Eklendi The End 01.01.2025

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO GateZoneNotice Ayarlar Eklendi 04.01.2025
#pragma region CUser::JstKOGateZoneNotice()
void CUser::JstKOGateZoneNotice()
{
	std::string sZoneName;
	switch (GetZoneID())
	{
	case ZONE_KARUS: sZoneName = "Luferson"; break;
	case ZONE_KARUS2: sZoneName = "Luferson"; break;
	case ZONE_KARUS3: sZoneName = "Luferson"; break;
	case ZONE_ELMORAD: sZoneName = "El Morad"; break;
	case ZONE_ELMORAD2: sZoneName = "El Morad"; break;
	case ZONE_ELMORAD3: sZoneName = "El Morad"; break;
	case ZONE_KARUS_ESLANT: sZoneName = "Karus Eslant"; break;
	case ZONE_KARUS_ESLANT2: sZoneName = "Karus Eslant"; break;
	case ZONE_KARUS_ESLANT3: sZoneName = "Karus Eslant"; break;
	case ZONE_ELMORAD_ESLANT: sZoneName = "El Morad Eslant"; break;
	case ZONE_ELMORAD_ESLANT2: sZoneName = "El Morad Eslant"; break;
	case ZONE_ELMORAD_ESLANT3: sZoneName = "El Morad Eslant"; break;
	case ZONE_MORADON: sZoneName = "Moradon"; break;
	case ZONE_MORADON2: sZoneName = "Moradon"; break;
	case ZONE_MORADON3: sZoneName = "Moradon"; break;
	case ZONE_DELOS: sZoneName = "Delos"; break;
	case ZONE_BIFROST: sZoneName = "Bifrost"; break;
	case ZONE_DESPERATION_ABYSS: sZoneName = "Desperation Abyss"; break;
	case ZONE_HELL_ABYSS: sZoneName = "Hell Abyss"; break;
	case ZONE_DRAGON_CAVE: sZoneName = "Dragon Cave"; break;
	case ZONE_ARENA: sZoneName = "Arena"; break;
	case ZONE_ORC_ARENA: sZoneName = "Orc Arena"; break;
	case ZONE_GOBLIN_ARENA: sZoneName = "Goblin Arena"; break;
	case ZONE_CAITHAROS_ARENA: sZoneName = "Caitharos Arena"; break;
	case ZONE_FORGOTTEN_TEMPLE: sZoneName = "Forgotten Temple"; break;
	case ZONE_RONARK_LAND: sZoneName = "Ronark Land"; break;
	default:
		break;
	}

	if (!isGM() && g_pMain->Giri�GateZoneNotice) //JstKO Gmsiz Herkes User G�r�yoruz Giris Gate Zone RGB Notice Eklendi. 04.01.2025
	{
		if (GetNation() == KARUS)
		    g_pMain->SendRgbNotice(string_format("KARUS Oyuncu: [%s] Gate Zone [%s] giris yapti.", GetName().c_str(), sZoneName.c_str()), 55, 55, 255); // Gate Giris Karus
		else
			g_pMain->SendRgbNotice(string_format("HUMAN Oyuncu: [%s] Gate Zone [%s] giris yapti.", GetName().c_str(), sZoneName.c_str()), 255, 102, 102); // Gate Giris Human
	}
}
#pragma endregion
// JstKO GateZoneNotice Ayarlar Eklendi The End 04.01.2025

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO OyunOnlineNotice Ayarlar Eklendi 31.12.2024
#pragma region CUser::JstKOOyunOnlineNotice()
void CUser::JstKOOyunOnlineNotice()
{
	uint16 UserTotal = 0; // User Online.
	uint16 KarusCount = 0; // User Karus Online.
	uint16 ElmoradCount = 0; // User Human Online.
	for (int i = 0; i < MAX_USER; i++) // User Max 5000 Son.
	{
		CUser* pUser = g_pMain->GetUserPtr(i); // User �nsan Ger�ek.

		if (pUser == nullptr || !pUser->isInGame()) // User Oyunda Giriyor.
			continue; // Devam Etmek Demek.

		UserTotal++; // User Online.

		pUser->GetNation() == Nation::KARUS ? KarusCount++ : ElmoradCount++; // User Karus ve Human Oyunda Online.
	}

	uint16 BotTotal = 0; // Bot Online.
	uint16 BotKarusCount = 0; // Bot Karus Online.
	uint16 BotElmoradCount = 0; // Bot Human Online.
	for (int i = 0; i < MAX_BOT; i++) // Bot Max 5000 Son.
	{
		CBot* pBot = g_pMain->GetBotPtr(i); // Bot Zeka Ger�ek.

		if (pBot == nullptr || !pBot->isInGame()) // Bot Oyunda Giriyor.
			continue; // Devam Etmek Demek.

		BotTotal++; // Bot Online.

		pBot->GetNation() == Nation::KARUS ? BotKarusCount++ : BotElmoradCount++; // Bot Karus ve Human Oyunda Online.
	}

	if (g_pMain->pServerSetting.OyunUserBotOnlineNotice) // User&Bot Oyunda Online | Tablo Server_Settings OyunUserBotOnlineNotice Eklendi 1 A�ip ve 0 Kapatma Ayarlar Yeter :)
	g_pMain->SendYesilNotice(this, string_format("Oyunda : [%d] (Karus : [%d] | [%d] : Human) Oyuncu Aktif", UserTotal+BotTotal, KarusCount+BotKarusCount, ElmoradCount+BotElmoradCount));
	return; // Geri D�nmek Demek 

}
#pragma endregion
// JstKO OyunOnlineNotice Ayarlar Eklendi The End 31.12.2024

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO AutoOnlineCount Ayarlar Eklendi 01.01.2025
#pragma region CGameServerDlg::JstKOAutoOnlineCount()
void CGameServerDlg::JstKOAutoOnlineCount() 
{
	uint32 nHour = g_localTime.tm_hour;
	uint32 nMinute = g_localTime.tm_min;
	uint32 nSecond = g_localTime.tm_sec;

	if (g_pMain->AutoOnlineCountNotice) // JstKO Yeni Auto Online Count Notice Eklendi 01.01.2025
	{
		if ((nMinute % 15 == 0 || nMinute == 0) && nSecond == 0)
		{
			uint16 usercount = 0;
			SessionMap sessMap = g_pMain->m_socketMgr.GetActiveSessionMap();

			for (auto itr = sessMap.begin(); itr != sessMap.end(); ++itr)
			{
				if (TO_USER(itr->second)->isInGame())
					usercount++;
			}

			Guard lock(g_pMain->m_BotcharacterNameLock);
			usercount += (uint16)g_pMain->m_BotcharacterNameMap.size(); 

			std::string sNoticeMessage = string_format("User&Bot Online List : [%d] Bizimle Oldu�unuz ��in Te�ekk�r Ederiz.", usercount);

			if (!sNoticeMessage.empty())
				g_pMain->SendNotice(sNoticeMessage.c_str(), Nation::ALL);
		}
	}
}
#pragma endregion
// JstKO AutoOnlineCount Ayarlar Eklendi The End 01.01.2025

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO User Game Info Komut Eklendi 07.01.2025
#pragma region CUser::JstKOUserGameInfoNotice()
void CUser::JstKOUserGameInfoNotice()
{
	if (g_pMain->UserGameInfoKomutNotice) // JstKO Oyun Rehber Bilgisi Komutlar�n� JstKOSettings.ini Eklendi 07.01.2025
	{
		Packet JstKO2; // JstKO Chat Bilgisi Notice Eklendi 07.01.2025
		std::string Bilgisi_Notice = string_format("Oyun Rehber Bilgilerini Almak I�in Chat Kismindan +bilgi Yazabilirsiniz.");
		JstKO2.clear();
		ChatPacket::Construct(&JstKO2, ChatType::FORCE_CHAT, Bilgisi_Notice.c_str(), "[OYUN B�LG�S�]", GetNation());
		Send(&JstKO2);
	}

	// JstKO Oyun Rehber Bilgisi Komutlar�n� Eklendi 07.01.2025

	SendChat(ChatType::GENERAL_CHAT, string_format("JstKO Anti Cheat Koruma Hilesiz ve Koxp %100 Yenilmez.", m_strUserID.c_str()), "{[Koruma Sistemi]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Rehber Bilgileri Asagi Komutlar�n� G�rebilirsiniz.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("NP ile PUS Itemi Alabilirsiniz.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("CR'lerden PUS Itemi Kazanabilirsiniz.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Cz'deki �zel Bosslardan Tak� Kasabilirsiniz.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("G�nl�k Giri� �d�l Sistemi Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Monster Stone ve Event Haritalar� Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Skill ve Stat S�f�rlama �cretsizdir.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Colony Zone'de Chaos Stone ve Mini Chaos Stone Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Manner Point ile PUS Itemi Alma Sistemi Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Daily Quest Sistemi Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Bah�e Farm�ndan KC Drop Sistemi Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Exp: [ZOR] | �tem: [ORTA].", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Tekrarl� ve G�nl�k Quest Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("CZ ve Eventlerde Botlar Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("�ron Necklace, �ron Belt ve Chitin Shield Droplar� Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Eslant ve CZ Ge�i� Level S�n�r�: 60.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("�slion ilk G�nden Itibaren Aktif.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Multi Client S�n�r�: 3 | IP S�n�r�: 9.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("TL Merchant Aktif!", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Upgrade +8 Sondur. Tak�larda +1 Sondur.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Trade ve Merchant I�in Level S�n�r� Yoktur.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Merchant KC Item Sat�� Minimum 1.000 KC Olarak Belirlenmi�tir.", m_strUserID.c_str()), "{[�zellikleri]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("YEN� CHAOT�C GENERATOR SAYES�NDE DAHA DETAYLI VE HIZLI B�R S�STEM S�Z� BEKL�YOR", m_strUserID.c_str()), "{[CHAOT�C GENERATOR S�STEM�]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("Maradon ve Cz Bolgesinde Online Kalip Otomatik olarak Cash ve Np Kazanabilirsiniz", m_strUserID.c_str()), "{[Online Hediye Sistemi]}");
	SendChat(ChatType::GENERAL_CHAT, string_format("JstKO Haydi Sen de Kat�l ve Efsane Maceraya Ba�la!", m_strUserID.c_str()), "{[Oyun Gel]}");
	return;
}
#pragma endregion
// JstKO User Game Info Komut Eklendi The End 07.01.2025

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO Yeni S�re �temler List Notice Eklendi. 06.02.2025 
#pragma region CUser::JstKOExprationItemList()
void CUser::JstKOExprationItemList()
{
	// JstKO Yeni S�re �temler Info Notice Eklendi. 06.02.2025 
	std::string PMName = "Gecici Gun Item List";
	uint8 ItemCount = 0;

	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = GetItem(i);

		if (pItem == nullptr)
			continue;

		if ((pItem->nExpirationTime - UNIXTIME) / 86400 > 300)
			continue;

		if (pItem->nExpirationTime != 0)
		{
			_ITEM_TABLE pTable = g_pMain->GetItemPtr(pItem->nNum);

			if (pTable.isnull())
				continue;

			ItemCount++;

			// JstKO Yeni Pm Mesaj Otomatik S�resi Notice Eklendi. 06.02.2025
			if (ItemCount == 1)
			{
				std::string Mesaj = "Suresi Bitecek Itemlerin Listesi";
				Packet result;
				ChatPacket::Construct(&result, ChatType::PRIVATE_CHAT, &Mesaj, &PMName, GetNation());
				Send(&result);
			}

			// JstKO Yeni Pm Mesaj Otomatik S�resi Notice Eklendi. 06.02.2025
			std::string Mesaj = string_format("Item Name : %s | (Gun : %d Kalan) Sonra Silinecek. Iyi Oyunlar.", pTable.m_sName.c_str(), (pItem->nExpirationTime - UNIXTIME) / 86400);
			Packet result;
			ChatPacket::Construct(&result, ChatType::PRIVATE_CHAT, &Mesaj, &PMName, GetNation());
			Send(&result);

			//------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

			Packet JstKO; // JstKO Public Chat Renk Sari Notice Eklendi 06.02.2025
			std::string PubliChat_Notice = string_format("Item Name : %s | (Gun : %d Kalan) Sonra Silinecek. Iyi Oyunlar.", pTable.m_sName.c_str(), (pItem->nExpirationTime - UNIXTIME) / 86400);
			JstKO.clear();
			ChatPacket::Construct(&JstKO, ChatType::PUBLIC_CHAT, PubliChat_Notice.c_str(), "[Gecici Gun Item List]", GetNation());
			Send(&JstKO);
		}
	}
}
#pragma endregion
// JstKO Yeni S�re �temler List Notice Eklendi. The End 06.02.2025 

//-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

// JstKO Yeni S�re �temler Deleted Notice Eklendi. 06.02.2025 
#pragma region CUser::JstKOExprationItemDeleted()
void CUser::JstKOExprationItemDeleted()
{
	// JstKO Yeni S�re �temler Deleted Notice Eklendi. 06.02.2025 
	std::string PMName = "[Gun Suresi Silindi]";

	for (int i = 0; i < INVENTORY_TOTAL; i++)
	{
		_ITEM_DATA* pItem = GetItem(i);

		if (pItem == nullptr || pItem->nNum == 0)
			continue;

		if (pItem->nExpirationTime != 0 && pItem->nExpirationTime < (uint32)UNIXTIME)
		{
			_ITEM_TABLE pTable = g_pMain->GetItemPtr(pItem->nNum);

			if (pTable.isnull())
				continue;

			// JstKO Yeni Pm Mesaj Otomatik S�resi Notice Eklendi. 06.02.2025
			std::string Mesaj = string_format("Nick : %s | Item Name : %s | (Gun : %d Kalan) | Suresi dolan item silindi.", GetName().c_str(), pTable.m_sName.c_str(), 0);
			Packet result;
			ChatPacket::Construct(&result, PRIVATE_CHAT, &Mesaj, &PMName, GetNation());
			Send(&result);

			//------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

			Packet JstKO; // JstKO Public Chat Renk Sari Notice Eklendi 06.02.2025
			std::string PubliChat_Notice = string_format("Nick : %s | Item Name : %s | (Gun : %d Kalan) | Suresi dolan item silindi.", GetName().c_str(), pTable.m_sName.c_str(), 0);
			JstKO.clear();
			ChatPacket::Construct(&JstKO, ChatType::PUBLIC_CHAT, PubliChat_Notice.c_str(), "[Gun Suresi Silindi]", GetNation());
			Send(&JstKO);
		}
	}
}
#pragma endregion
// JstKO Yeni S�re �temler Deleted Notice Eklendi. The End 06.02.2025 

