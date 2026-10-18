#include "stdafx.h"
#include "DBAgent.h"
#include "../shared/DateTime.h"
#include "../shared/Ini.h"

using std::string;

ServerCommandTable CGameServerDlg::s_commandTable;
ChatCommandTable CUser::s_commandTable;



static bool FarmBotCastSkill(CBot* pBot, Unit* pTarget, const uint32* pSkillList, size_t skillCount)
{
	if (pBot == nullptr || pTarget == nullptr || skillCount == 0)
		return false;

	if (pBot->GetBotState() != BOT_FARMER
		|| !pBot->isInGame()
		|| pBot->isDead()
		|| pBot->GetZoneID() != pTarget->GetZoneID())
		return false;

	// Komutla atilan destek skillerinde bot uzakta kaldiysa once hedefe yaklastir.
	if (pBot->GetDistanceSqrt(pTarget) > 18.0f)
	{
		C3DMap* pMap = g_pMain->GetZoneByID(pTarget->GetZoneID());
		if (pMap == nullptr)
			return false;

		pBot->UserInOut(INOUT_OUT);
		pBot->m_pMap = pMap;
		pBot->m_bZone = pTarget->GetZoneID();
		pBot->SetPosition(pTarget->GetX() + myrand(-3, 3), pTarget->GetY(), pTarget->GetZ() + myrand(-3, 3));
		pBot->m_oldx = pBot->m_curx;
		pBot->m_oldy = pBot->m_cury;
		pBot->m_oldz = pBot->m_curz;
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->UserInOut(INOUT_IN);
	}

	for (size_t i = 0; i < skillCount; i++)
	{
		uint32 skillID = pSkillList[i];

		pBot->StateChangeServerDirect(1, USER_STANDING);
		pBot->MagicPacket(MAGIC_CASTING, skillID, pBot->GetID(), pTarget->GetID(), (uint16)pBot->GetX(), (uint16)pBot->GetY(), (uint16)pBot->GetZ());
		pBot->MagicPacket(MAGIC_EFFECTING, skillID, pBot->GetID(), pTarget->GetID(), (uint16)pBot->GetX(), (uint16)pBot->GetY(), (uint16)pBot->GetZ());
		pBot->m_sSkillCoolDown = (uint32)UNIXTIME + 2;
		return true;
	}

	return false;
}


static std::string NormalizeFarmBotPartyCommand(std::string command)
{
	STRTOLOWER(command);
	std::string out;
	for (size_t i = 0; i < command.size(); i++)
	{
		unsigned char ch = (unsigned char)command[i];
		if (ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' || ch == '.' || ch == ',' || ch == '!' || ch == '?')
			continue;
		out.push_back((char)ch);
	}
	return out;
}

static bool IsPlusOnlyCommand(const std::string& cmd)
{
	if (cmd.empty())
		return false;
	for (size_t i = 0; i < cmd.size(); i++)
	{
		if (cmd[i] != '+')
			return false;
	}
	return true;
}

static bool IsFarmBotBuffCommand(const std::string& cmd)
{
	return IsPlusOnlyCommand(cmd)
		|| cmd == "+buff" || cmd == "+buf" || cmd == "buff" || cmd == "buf"
		|| cmd == "+pls" || cmd == "pls" || cmd == "+please" || cmd == "please";
}

static bool IsFarmBotAcCommand(const std::string& cmd)
{
	return cmd == "ac" || cmd == "+ac" || cmd == "ac+" || cmd == "acpls" || cmd == "+acpls" || cmd == "acplease";
}

static bool IsFarmBotWolfCommand(const std::string& cmd)
{
	return cmd.find("wolf") != std::string::npos || cmd.find("wolfff") != std::string::npos;
}

static bool IsFarmBotSwiftCommand(const std::string& cmd)
{
	return cmd == "sw" || cmd == "+sw" || cmd == "swift" || cmd == "+swift"
		|| cmd.find("swsw") != std::string::npos || cmd.find("sww") != std::string::npos
		|| cmd.find("swift") != std::string::npos;
}

static bool FarmBotCastPartyCommand(CUser* pUser, const std::string& rawCommand)
{
	if (pUser == nullptr || !pUser->isInParty())
		return false;

	std::string command = NormalizeFarmBotPartyCommand(rawCommand);
	if (command.empty())
		return false;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(pUser->GetPartyID());
	if (pParty == nullptr)
		return false;

	std::vector<Unit*> targets;
	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] < 0)
			continue;

		Unit* pMember = g_pMain->GetUnitPtr(pParty->uid[i], pUser->GetZoneID());
		if (pMember != nullptr && !pMember->isDead() && pMember->GetZoneID() == pUser->GetZoneID())
			targets.push_back(pMember);
	}

	if (targets.empty())
		return false;

	CBot* pPriestBot = nullptr;
	CBot* pRogueBot = nullptr;
	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] < 0)
			continue;

		CBot* pBot = g_pMain->GetBotPtr(pParty->uid[i]);
		if (pBot == nullptr || pBot->GetBotState() != BOT_FARMER || !pBot->isInGame() || pBot->isDead())
			continue;

		if (pBot->GetZoneID() != pUser->GetZoneID())
			continue;

		if (pPriestBot == nullptr && pBot->isPriest())
			pPriestBot = pBot;
		else if (pRogueBot == nullptr && pBot->isRogue())
			pRogueBot = pBot;
	}

	bool casted = false;
	if (IsFarmBotWolfCommand(command) && pRogueBot != nullptr)
	{
		const uint32 karusWolf[] = { 108725, 108002 };
		const uint32 elmoWolf[]  = { 208725, 208002 };
		const uint32* skills = (pRogueBot->GetNation() == ELMORAD ? elmoWolf : karusWolf);
		size_t count = (pRogueBot->GetNation() == ELMORAD ? _countof(elmoWolf) : _countof(karusWolf));
		foreach(itr, targets)
			casted = FarmBotCastSkill(pRogueBot, *itr, skills, count) || casted;
	}
	else if (IsFarmBotSwiftCommand(command) && pRogueBot != nullptr)
	{
		const uint32 karusSwift[] = { 108010, 108002 };
		const uint32 elmoSwift[]  = { 208010, 208002 };
		const uint32* skills = (pRogueBot->GetNation() == ELMORAD ? elmoSwift : karusSwift);
		size_t count = (pRogueBot->GetNation() == ELMORAD ? _countof(elmoSwift) : _countof(karusSwift));
		foreach(itr, targets)
			casted = FarmBotCastSkill(pRogueBot, *itr, skills, count) || casted;
	}
	else if (IsFarmBotAcCommand(command) && pPriestBot != nullptr)
	{
		const uint32 karusAc[] = { 112674, 112673, 112660, 112651, 112639, 112630 };
		const uint32 elmoAc[]  = { 212674, 212673, 212660, 212651, 212639, 212630 };
		const uint32* skills = (pPriestBot->GetNation() == ELMORAD ? elmoAc : karusAc);
		size_t count = (pPriestBot->GetNation() == ELMORAD ? _countof(elmoAc) : _countof(karusAc));
		foreach(itr, targets)
			casted = FarmBotCastSkill(pPriestBot, *itr, skills, count) || casted;
	}
	else if (IsFarmBotBuffCommand(command) && pPriestBot != nullptr)
	{
		const uint32 karusBuff[] = { 112675, 112672, 112670, 112657, 112656, 112655, 112654 };
		const uint32 elmoBuff[]  = { 212675, 212672, 212670, 212657, 212656, 212655, 212654 };
		const uint32* skills = (pPriestBot->GetNation() == ELMORAD ? elmoBuff : karusBuff);
		size_t count = (pPriestBot->GetNation() == ELMORAD ? _countof(elmoBuff) : _countof(karusBuff));
		foreach(itr, targets)
			casted = FarmBotCastSkill(pPriestBot, *itr, skills, count) || casted;
	}

	return casted;
}

static void WarpFarmPartyBotsToUser(CUser* pUser)
{
	if (pUser == nullptr || !pUser->isInParty())
		return;

	_PARTY_GROUP* pParty = g_pMain->GetPartyPtr(pUser->GetPartyID());
	if (pParty == nullptr)
		return;

	CBot* pMageBot = nullptr;
	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] < 0)
			continue;

		CBot* pBot = g_pMain->GetBotPtr(pParty->uid[i]);
		if (pBot != nullptr && pBot->GetBotState() == BOT_FARMER && pBot->isMage())
		{
			pMageBot = pBot;
			break;
		}
	}

	// Partyde mage farm bot yoksa tp komutu bosuna calismasin.
	if (pMageBot == nullptr)
		return;

	C3DMap* pMap = g_pMain->GetZoneByID(pUser->GetZoneID());
	if (pMap == nullptr)
		return;

	const float sideX[8] = { 0.0f, 5.0f, -5.0f, 8.0f, -8.0f, 11.0f, -11.0f, 0.0f };
	const float sideZ[8] = { 6.0f, 5.0f, 5.0f, 3.5f, 3.5f, 2.0f, 2.0f, 9.0f };

	for (uint8 i = 0; i < MAX_PARTY_USERS; i++)
	{
		if (pParty->uid[i] < 0)
			continue;

		CBot* pBot = g_pMain->GetBotPtr(pParty->uid[i]);
		if (pBot == nullptr || pBot->GetBotState() != BOT_FARMER)
			continue;

		uint8 slot = i >= 8 ? 1 : i;
		pBot->UserInOut(INOUT_OUT);
		pBot->m_pMap = pMap;
		pBot->m_bZone = pUser->GetZoneID();
		pBot->SetPosition(pUser->GetX() + sideX[slot], pUser->GetY(), pUser->GetZ() + sideZ[slot]);
		pBot->m_oldx = pBot->m_curx;
		pBot->m_oldy = pBot->m_cury;
		pBot->m_oldz = pBot->m_curz;
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bGenieStatus = true;
		pBot->StateChangeServerDirect(1, USER_STANDING);
		pBot->UserInOut(INOUT_IN);
	}
}


#pragma region CGameServerDlg::HandleMorankerCommand
COMMAND_HANDLER(CGameServerDlg::HandleMorankerCommand)
{
	std::string sub = "reload";
	if (!vargs.empty())
	{
		sub = vargs.front();
		vargs.pop_front();
		STRTOLOWER(sub);
	}

	if (sub == "clear")
	{
		ClearMorankerRankBots();
		printf("Moranker: rank botlari temizlendi.\n");
		return true;
	}

	if (sub == "status")
	{
		uint8 active = 0;
		for (uint8 i = 1; i <= 6; i++)
		{
			CBot* pBot = GetBotPtr(GetMorankerDisplayID(i));
			if (pBot != nullptr && pBot->isInGame())
				active++;
		}
		printf("Moranker: aktif rank bot sayisi %d/6\n", active);
		return true;
	}

	ClearMorankerRankBots();
	bool ok = SpawnMorankerRankBots();
	printf("%s\n", ok ? "Moranker: rank botlari yenilendi." : "Moranker: rank botlari yuklenemedi. LOAD_MORANKER_BOTS2369 sonucunu kontrol et.");
	return ok;
}
#pragma endregion

void CGameServerDlg::InitServerCommands()
{
	static Command<CGameServerDlg> commandTable[] = 
	{
		{ "help",				&CGameServerDlg::HandleHelpCommand,					"Tum komutlari gosterir." },
		{ "resetloyalty",		&CGameServerDlg::HandleResetRLoyaltyCommand,		"Loyalty/NP degerini sifirlar." },
		{ "notice",				&CGameServerDlg::HandleNoticeCommand,				"Sunucu geneline duyuru gonderir." },
		{ "noticeall",			&CGameServerDlg::HandleNoticeallCommand,			"Tum sunucuya duyuru gonderir." },
		{ "kill",				&CGameServerDlg::HandleKillUserCommand,				"Belirtilen oyuncunun baglantisini keser." },
		{ "open1",				&CGameServerDlg::HandleWar1OpenCommand,				"1. savas bolgesini acar." },
		{ "open2",				&CGameServerDlg::HandleWar2OpenCommand,				"2. savas bolgesini acar." },
		{ "open3",				&CGameServerDlg::HandleWar3OpenCommand,				"3. savas bolgesini acar." },
		{ "open4",				&CGameServerDlg::HandleWar4OpenCommand,				"4. savas bolgesini acar." },
		{ "open5",				&CGameServerDlg::HandleWar5OpenCommand,				"5. savas bolgesini acar." },
		{ "open6",				&CGameServerDlg::HandleWar6OpenCommand,				"6. savas bolgesini acar." },
		{ "snow",				&CGameServerDlg::HandleSnowWarOpenCommand,			"Kar savasi bolgesini acar." },
		{ "csw",				&CGameServerDlg::HandleSiegeWarOpenCommand,			"Castle Siege War bolgesini acar. Kullanim: /csw" },
		{ "close",				&CGameServerDlg::HandleWarCloseCommand,				"Aktif savas bolgesini kapatir. Kullanim: /close" },
		{ "cswclose",			&CGameServerDlg::HandleCastleSiegeWarClose,			"Aktif CSW bolgesini kapatir. Kullanim: /cswclose" },
		{ "down",				&CGameServerDlg::HandleShutdownCommand,				"Sunucuyu kapatir." },
		{ "discount",			&CGameServerDlg::HandleDiscountCommand,				"Son savasi kazanan irk icin indirim etkinligini acar." },
		{ "alldiscount",		&CGameServerDlg::HandleGlobalDiscountCommand,		"Herkes icin indirim etkinligini acar." },
		{ "offdiscount",		&CGameServerDlg::HandleDiscountOffCommand,			"Indirim etkinligini kapatir." },
		{ "captain",			&CGameServerDlg::HandleCaptainCommand,				"Savas kaptanlarini/komutanlarini ayarlar." },
		{ "santa",				&CGameServerDlg::HandleSantaCommand,				"Ucan Santa Claus etkinligini acar." },
		{ "santaclose",			&CGameServerDlg::HandleSantaOffCommand,				"Ucan Santa Claus/angel etkinligini kapatir." },
		{ "angel",				&CGameServerDlg::HandleAngelCommand,				"Ucan angel etkinligini acar." },
		{ "angelclose",			&CGameServerDlg::HandleSantaOffCommand,				"Ucan Santa Claus/angel etkinligini kapatir." },
		{ "permanent",			&CGameServerDlg::HandlePermanentChatCommand,		"Kalici sohbet cubugu yazisini ayarlar." },
		{ "offpermanent",		&CGameServerDlg::HandlePermanentChatOffCommand,		"Kalici sohbet cubugu yazisini sifirlar." },
		{ "beefclose",			&CGameServerDlg::HandleBeefEventClose,				"Beef etkinligini kapatir. Kullanim: /beefclose" },
		{ "reloadnotice",		&CGameServerDlg::HandleReloadNoticeCommand,			"Oyun ici duyuru listesini yeniden yukler." },
		{ "reloadtables",		&CGameServerDlg::HandleReloadTablesCommand,			"Oyun ici tablolari yeniden yukler." },
		{ "reloadtables2",		&CGameServerDlg::HandleReloadTables2Command,		"Oyun ici tablolari yeniden yukler." },
		{ "reloadtables3",		&CGameServerDlg::HandleReloadTables3Command,		"Oyun ici tablolari yeniden yukler." },
		{ "reloadmagics",		&CGameServerDlg::HandleReloadMagicsCommand,			"Magic/skill tablolarini yeniden yukler." },
		{ "reloadquests",		&CGameServerDlg::HandleReloadQuestCommand,			"Gorev tablolarini yeniden yukler." },
		{ "reloaddailyquest",	&CGameServerDlg::HandleReloadDailyQuestCommand,		"Sadece gunluk gorev tablolarini yeniden yukler." },
		{ "reloadranks",		&CGameServerDlg::HandleReloadRanksCommand,			"Rank tablolarini yeniden yukler." },
		{ "reloaddrops",		&CGameServerDlg::HandleReloadDropsCommand,			"Drop tablolarini yeniden yukler." },
		{ "reloaddrops2",		&CGameServerDlg::HandleReloadDropsRandomCommand,		"Random drop tablolarini yeniden yukler." },
		{ "reloadkings",		&CGameServerDlg::HandleReloadKingsCommand,			"King tablolarini yeniden yukler." },
		{ "reloadtitle",		&CGameServerDlg::HandleReloadRightTopTitleCommand,	"Sag ust baslik tablolarini yeniden yukler." }, 
		{ "reloadpus",			&CGameServerDlg::HandleReloadPusItemCommand,		"PUS tablolarini yeniden yukler." }, 
		{ "reloaditems",		&CGameServerDlg::HandleReloadItemsCommand,			"Item tablolarini yeniden yukler." },
		{ "reloaddungeon",		&CGameServerDlg::HandleReloadDungeonDefenceTables,	"Dungeon Defence tablolarini yeniden yukler." },
		{ "reloaddraki",		&CGameServerDlg::HandleReloadDrakiTowerTables,		"Draki Tower tablolarini yeniden yukler." },
		{ "reloadevent",		&CGameServerDlg::HandleEventScheduleResetTable,		"Etkinlik zamanlayici tablolarini yeniden yukler." },
		{ "reloadpremium",		&CGameServerDlg::HandleReloadClanPremiumTable,		"Clan premium tablosunu yeniden yukler." },
		{ "reloadsocial",		&CGameServerDlg::HandleTopLeftCommand,				"SocialGroup ikonlarini yeniden yukler." },
		{ "reloadclanpnotice",	&CGameServerDlg::HandleReloadBonusNotice,			"Clan premium duyuru listesini yeniden yukler." },
		{ "reload_item",		&CGameServerDlg::HandleReloadItems,					"Item tablosunu yeniden yukler." },
		{ "reloadupgrade",		&CGameServerDlg::HandleReloadUpgradeCommand,		"Upgrade tablosunu yeniden yukler." },
		{ "reloadbug",			&CGameServerDlg::HandleReloadRankBugCommand,		"Upgrade bug/rank bug tablosunu yeniden yukler." },
		
		{ "reloadlreward",		&CGameServerDlg::HandleReloadLevelRewardCommand,		"Level odul tablosunu yeniden yukler." },
		{ "reloadmreward",		&CGameServerDlg::HandleReloadMerchantLevelRewardCommand,"Merchant level odul tablosunu yeniden yukler." },

		{ "reloadzoneon",		&CGameServerDlg::HandleReloadZoneOnlineRewardCommand,"Zone online odul tablosunu yeniden yukler." },

		{ "cindopen",			&CGameServerDlg::HandleCindirellaWarOpen,			"Fun Class/Cindirella etkinligini acar." },
		{ "cindclose",			&CGameServerDlg::HandleCindirellaWarClose,			"Fun Class/Cindirella etkinligini kapatir." },
		{ "ftopen",				&CGameServerDlg::HandleForgettenTempleEvent,		"Forgetten Temple etkinligini acar." },
		{ "ftclose",			&CGameServerDlg::HandleForgettenTempleEventClose,	"Forgetten Temple etkinligini kapatir." },
		{ "count",				&CGameServerDlg::HandleCountCommand,				"Online oyuncu sayisini gosterir." },
		{ "tpall",				&CGameServerDlg::HandleTeleportAllCommand,			"Oyunculari home zone bolgesine yollar." },
		{ "warresult",			&CGameServerDlg::HandleWarResultCommand,			"Savas sonucunu ayarlar." },
		{ "utc",				&CGameServerDlg::HandleEventUnderTheCastleCommand,	"Under the Castle etkinligini acar/kapatir." },
		{ "tournamentstart",	&CGameServerDlg::HandleTournamentStart,				"Clan Tournament etkinligini baslatir." },
		{ "tournamentclose",	&CGameServerDlg::HandleTournamentClose,				"Clan Tournament etkinligini kapatir." },		
		{ "chaosopen",			&CGameServerDlg::HandleChaosExpansionOpen,			"Chaos Expansion etkinligini acar. Kullanim: /chaosopen" },
		{ "borderopen",			&CGameServerDlg::HandleBorderDefenceWar,			"Border Defence War etkinligini acar. Kullanim: /borderopen" },
		{ "juraidopen",			&CGameServerDlg::HandleJuraidMountain,				"Juraid Mountain etkinligini acar. Kullanim: /juraidopen" },
		{ "beefopen",			&CGameServerDlg::HandleBeefEvent,					"Beef etkinligini acar. Kullanim: /beefopen" },		
		{ "chaosclose",			&CGameServerDlg::HandleChaosExpansionClose,			"Chaos Expansion etkinligini kapatir. Kullanim: /chaosclose" },
		{ "borderclose",		&CGameServerDlg::HandleBorderDefenceWarClose,		"Border Defence War etkinligini kapatir. Kullanim: /borderclose" },
		{ "juraidclose",		&CGameServerDlg::HandleJuraidMountainClose,			"Juraid Mountain etkinligini kapatir. Kullanim: /juraidclose" },		
		{ "lottery",			&CGameServerDlg::HandleLotteryStart,				"Lottery etkinligini baslatir. Kullanim: /lottery" },				
		{ "lotteryclose",		&CGameServerDlg::HandleLotteryClose,				"Lottery etkinligini kapatir. Kullanim: /lotteryclose" },
		{ "testing",			&CGameServerDlg::HandleServerGameTestCommand,		"Server test komutudur." },
		{ "aireset",			&CGameServerDlg::HandleAIResetCommand,				"AI sistemini sifirlar."	},
		{ "reloadspawn",		&CGameServerDlg::HandleAIResetCommand,				"Panelden guncellenen K_NPC/K_MONSTER ve K_NPCPOS dogus noktalarini oyuna anlik yeniden yukler." },
		{ "moranker",			&CGameServerDlg::HandleMorankerCommand,			"Moranker rank botlarini yonetir. Kullanim: /moranker reload|clear|status" },
		{ "npcreload",		&CGameServerDlg::HandleAIResetCommand,				"NPC/monster tablolarini ve dogus noktalarini yeniden yukler." },
		{ "block",				&CGameServerDlg::Handlebannedcommand,				"Oyuncuyu banlar. Kullanim: /block CharacterNick veya +block CharacterNick Sure" },
		{ "bug",				&CGameServerDlg::HandleBugdanKurtarCommand,			"Askida kalan karakteri kurtarir." },
		{ "reload_cind",		&CGameServerDlg::HandleReloadCindirellaCommand,		"Cindirella/Fun Class etkinlik tablolarini yeniden yukler." },
		{ "exp_add",			&CGameServerDlg::HandleExpAddCommand,				"Sunucu geneli EXP event ayarlar. 0 verilirse kapanir. Kullanim: bonusPercent" },
		{ "np_add",				&CGameServerDlg::HandleNPAddCommand,				"Sunucu geneli NP event ayarlar. 0 verilirse kapanir. Kullanim: bonusPercent" },
		{ "money_add",			&CGameServerDlg::HandleMoneyAddCommand,				"Sunucu geneli coin event ayarlar. 0 verilirse kapanir. Kullanim: bonusPercent" },
		{ "drop_add",			&CGameServerDlg::HandleDropAddCommand,				"Sunucu geneli drop event ayarlar. 0 verilirse kapanir. Kullanim: bonusPercent" },
		{ "event",				&CGameServerDlg::HandleSpecialEventOpenCommand,		"Ozel etkinlik komutudur." },
		{ "reloadatak",			&CGameServerDlg::HandleReloadCOEFFICIENTCommand,			"Atak/katsayi tablolarini yeniden yukler." },
		{ "pkbotmix",			&CGameServerDlg::HandleServerPkBotMixCommand,		"Karisik PK botlarini baslatir. Kullanim: /pkbotmix Dakika MinLevel [ZoneID] " },
		{ "pkboxsingle",		&CGameServerDlg::HandleServerPkBoxSingleCommand,	"Tek sinif PK botlarini baslatir. Kullanim: /pkboxsingle Sinif Dakika MinLevel ZoneID" },
		{ "farmbotmix",			&CGameServerDlg::HandleServerFarmBotMixCommand,		"Karisik farm partilerini baslatir. Kullanim: /farmbotmix Dakika PartyCount [ZoneID] " },
		{ "merchantbotmix",		&CGameServerDlg::HandleServerMerchantBotMixCommand,	"Merchant botlarini baslatir. Kullanim: /merchantbotmix Adet Dakika AlanTipi [ZoneID] " },
		{ "open_master",		&CGameServerDlg::HandleServerOpenMasterCommand,		"Hedef karakterin masterini acar. Kullanim: /open_master CharacterName " },
	};

	init_command_table(CGameServerDlg, commandTable, s_commandTable);
}

void CGameServerDlg::CleanupServerCommands() { free_command_table(s_commandTable); }

void CUser::InitChatCommands()
{
	static Command<CUser> commandTable[] = 
	{
		// Command				Handler											Help message
		{ "help",				&CUser::HandleHelpCommand,						"Tum komutlari gosterir." },
		{ "resetloyalty",		&CUser::HandleResetRLoyaltyCommand,				"Loyalty/NP degerini sifirlar." },
		{ "give",				&CUser::HandleGiveItemCommand,					"Oyuncuya item verir. Kullanim: +give CharacterNick ItemID Adet Sure" },
		{ "zone_give_item",		&CUser::HandleOnlineZoneGiveItemCommand,		"Belirtilen zone icindeki oyunculara item verir. Kullanim: +zone_give_item ZoneID ItemID Adet Sure" },
		{ "online_give_item",	&CUser::HandleOnlineGiveItemCommand,			"Online tum oyunculara item verir. Kullanim: +online_give_item ItemID Adet Sure" },
		{ "zone",				&CUser::HandleZoneChangeCommand,				"Belirtilen zone bolgesine isinlar. Kullanim: +zone ZoneID" },
		{ "mon",				&CUser::HandleMonsterSummonCommand,				"Belirtilen monsteri spawn eder, respawn olmaz. Kullanim: +mon MonsterID" },
		{ "npc",				&CUser::HandleNPCSummonCommand,					"Belirtilen NPCyi spawn eder, respawn olmaz. Kullanim: +npc NpcID" },
		{ "kill",				&CUser::HandleMonKillCommand,					"Belirtilen oyuncunun baglantisini keser." },
		{ "open1",				&CUser::HandleWar1OpenCommand,					"1. savas bolgesini acar." },
		{ "open2",				&CUser::HandleWar2OpenCommand,					"2. savas bolgesini acar." },
		{ "open3",				&CUser::HandleWar3OpenCommand,					"3. savas bolgesini acar." },
		{ "open4",				&CUser::HandleWar4OpenCommand,					"4. savas bolgesini acar." },
		{ "open5",				&CUser::HandleWar5OpenCommand,					"5. savas bolgesini acar." },
		{ "open6",				&CUser::HandleWar6OpenCommand,					"6. savas bolgesini acar." },
		{ "captain",			&CUser::HandleCaptainCommand,					"Savas kaptanlarini/komutanlarini ayarlar." },
		{ "snow",				&CUser::HandleSnowWarOpenCommand,				"Kar savasi bolgesini acar." },
		{ "csw",				&CUser::HandleSiegeWarOpenCommand,				"Castle Siege War bolgesini acar. Kullanim: +csw" },
		{ "close",				&CUser::HandleWarCloseCommand,					"Aktif savas bolgesini kapatir. Kullanim: +close" },
		{ "cswclose",			&CUser::HandleCastleSiegeWarClose,				"Aktif CSW bolgesini kapatir. Kullanim: +cswclose" },
		{ "np",					&CUser::HandleLoyaltyChangeCommand,				"Oyuncunun NP degerini degistirir. Kullanim: +np CharacterNick Miktar" },
		{ "exp",				&CUser::HandleExpChangeCommand,					"Oyuncunun EXP degerini degistirir. Kullanim: +exp CharacterNick Miktar" },
		{ "noah",				&CUser::HandleGoldChangeCommand,				"Oyuncunun Noah/coin degerini degistirir. Kullanim: +noah CharacterNick Miktar" },
		{ "kc",					&CUser::HandleKcChangeCommand,					"Oyuncunun KC degerini degistirir. Kullanim: +kc CharacterNick Miktar" },
		{ "tl",					&CUser::HandleTLBalanceCommand,					"Oyuncunun TL bakiyesini degistirir. Kullanim: +tl CharacterNick Miktar" },
		{ "tl_ekle",			&CUser::HandleTLBalanceCommand,					"Oyuncuya TL bakiye ekler. Kullanim: +tl_ekle CharacterNick Miktar" },
		{ "exp_add",			&CUser::HandleExpAddCommand,					"Sunucu geneli EXP event ayarlar. 0 verilirse kapanir. Kullanim: bonusPercent" },
		{ "np_add",				&CUser::HandleNPAddCommand,						"Sunucu geneli NP event ayarlar. 0 verilirse kapanir. Kullanim: bonusPercent" },
		{ "money_add",			&CUser::HandleMoneyAddCommand,					"Sunucu geneli coin event ayarlar. 0 verilirse kapanir. Kullanim: bonusPercent" },
		{ "drop_add",			&CUser::HandleDropAddCommand,					"Sunucu geneli drop event ayarlar. 0 verilirse kapanir. Kullanim: bonusPercent" },
		{ "tpall",				&CUser::HandleTeleportAllCommand,				"Oyunculari home zone bolgesine yollar." },
		{ "pmall",				&CUser::HandlePrivateAllCommand,				"Tum oyunculara ozel mesaj gonderir. Kullanim: +pmall Baslik Mesaj" },
		{ "summonknights",		&CUser::HandleKnightsSummonCommand,				"Belirtilen clani yanina ceker. Kullanim: +summonknights ClanAdi" },
		{ "warresult",			&CUser::HandleWarResultCommand,					"Savas sonucunu ayarlar."},
		{ "resetranking",		&CUser::HandleResetPlayerRankingCommand,		"Oyuncu rank listesini sifirlar. Kullanim: +resetranking ZoneID"},
		
		{ "nation_change",		&CUser::HandleNationChangeCommand,				"Oyuncunun irkini degistirir." },
		{ "item",				&CUser::HandleGiveItemSelfCommand,				"Komutu kullanan GM karakterine item verir. Kullanim: +item ItemID [Adet]" },
		{ "summonuser",			&CUser::HandleSummonUserCommand,				"Belirtilen oyuncuyu yanina ceker. Kullanim: +summonuser CharacterNick" },
		{ "tpon",				&CUser::HandleTpOnUserCommand,					"Belirtilen oyuncunun yanina isinlanir. Kullanim: +tpon CharacterNick" },
		{ "go",					&CUser::HandleGoUserCommand,					"Oyuncu veya bot konumuna isinlanir. Kullanim: +go CharacterName" },
		{ "goto",				&CUser::HandleLocationChange,					"Belirtilen koordinata isinlanir. Kullanim: +goto X Y" },
		{ "mute",				&CUser::HandleMuteCommand,						"Oyuncuyu susturur. Kullanim: +mute CharacterNick" },
		{ "unmute",				&CUser::HandleUnMuteCommand,					"Oyuncunun susturmasini kaldirir. Kullanim: +unmute CharacterNick" },
		
		{ "ftopen",				&CUser::HandleForgettenTempleEvent,				"Forgetten Temple etkinligini acar." },
		{ "ftclose",			&CUser::HandleForgettenTempleEventClose,		"Forgetten Temple etkinligini kapatir." },

		{ "clanchat",			&CUser::ClanChatFollowCommand,			         "GM icin klan sohbetini takip eder." }, // JstKO GameMaster Klan Sohbetini Oku Eklendi 01.01.2025
		{ "upfull",		&CUser::FullUpgradeStatusHandlers,			     "GM full upgrade oranini yuzde 100 yapar." }, // JstKO GM Full Upgrade %100 ?zel Eklendi 05.01.2025

		{ "allow",				&CUser::HandleAllowAttackCommand,				"Oyuncunun atak iznini acar." },
		{ "disable",			&CUser::HandleDisableCommand,					"Oyuncunun atak iznini kapatir." },
		{ "changeroom",			&CUser::HandleChangeRoom,						"Oyuncunun etkinlik odasini degistirir." },
		{ "hapis",				&CUser::HandleSummonPrison,						"Oyuncuyu hapis bolgesine gonderir. Kullanim: +hapis CharacterNick" },
		{ "afkbotspawn",		&CUser::HandleBotAfkSystem,						"AFK bot sistemini baslatir." },
		{ "miningbotspawn",		&CUser::HandleBotSpawnMining,					"Mining bot spawn eder." },
		{ "fishingbotspawn",	&CUser::HandleBotSpawnFishing,					"Fishing bot spawn eder." },
		{ "farmbotspawn",		&CUser::HandleBotSpawnFarm,						"Farm bot spawn eder." },
		{ "farmbotmix",			&CUser::HandleBotSpawnFarmMix,					"Karisik farm partilerini baslatir. Kullanim: +farmbotmix Dakika PartyCount [ZoneID]" },
		{ "pkbotspawn",			&CUser::HandleBotSpawnPk,						"PK bot spawn eder." },
		{ "pkbotmix",			&CUser::HandleBotSpawnPkMix,					"Karisik PK botlarini baslatir. Kullanim: +pkbotmix Dakika MinLevel [ZoneID]" },
		{ "pkboxsingle",		&CUser::HandlePkBoxSingleCommand,				"Tek sinif PK botlarini baslatir. Kullanim: +pkboxsingle Sinif Dakika MinLevel ZoneID" },
		{ "merchantbotmix",		&CUser::HandleMerchantBotMix,				"Merchant botlarini baslatir. Kullanim: +merchantbotmix Adet Dakika AlanTipi [ZoneID]" },
		{ "merchantbot",		&CUser::HandleMerchantBotCommand,				"Merchant bot ekler." },
		{ "merchantbotspawn",	&CUser::HandleBotSpawnMerchant,					"Merchant bot spawn eder." },
		{ "merchantmovebotspawn",&CUser::HandleBotSpawnMerchantMove,			"Hareketli merchant bot islemini baslatir." },
		{ "botkillone",		&CUser::HandleBotDisconnected,					"Secili/belirtilen tek botun baglantisini keser. Kullanim: +botkillone" },
		{ "killbot",			&CUser::HandleBotAllDisconnected,				"Tum botlarin baglantisini keser. Kullanim: +killbot" },
		{ "testing",			&CUser::HandleServerGameTestCommand,			"Server test komutudur." },
		{ "effect",				&CUser::HandleEffectTestCommand,			"GM effect test. Kullanim: +effect EffectID [show|player|both] [CharacterName]" },
		{ "chaosopen",			&CUser::HandleChaosExpansionOpen,				"Chaos Expansion etkinligini acar. Kullanim: +chaosopen" },
		{ "borderopen",			&CUser::HandleBorderDefenceWarOpen,				"Border Defence War etkinligini acar. Kullanim: +borderopen" },
		{ "juraidopen",			&CUser::HandleJuraidMountainOpen,				"Juraid Mountain etkinligini acar. Kullanim: +juraidopen" },
		{ "beefopen",			&CUser::HandleBeefEventOpen,					"Beef etkinligini acar. Kullanim: +beefopen" },		
		{ "chaosclose",			&CUser::HandleChaosExpansionClosed,				"Chaos Expansion etkinligini kapatir. Kullanim: +chaosclose" },
		{ "beefclose",			&CUser::HandleBeefEventClose,					"Beef etkinligini kapatir. Kullanim: +beefclose" },
		{ "borderclose",		&CUser::HandleBorderDefenceWarClosed,			"Border Defence War etkinligini kapatir. Kullanim: +borderclose" },
		{ "juraidclose",		&CUser::HandleJuraidMountainClosed,				"Juraid Mountain etkinligini kapatir. Kullanim: +juraidclose" },
		{ "drop",				&CUser::HandleNpcDropTester,					"NPC/monster drop test eder. Kullanim: hedef sec ve +drop MaksAdet" },
		{ "reload_table",		&CUser::HandleReloadTable,						"Tabloyu yeniden yukler." },
		{ "fishing",			&CUser::HandleFishingDropTester,				"Fishing drop test eder. Kullanim: +fishing" },
		{ "mining",				&CUser::HandleMiningDropTester,					"Mining drop test eder. Kullanim: +mining" },
		{ "clear",				&CUser::HandleInventoryClear,					"Oyuncunun envanterini temizler. Kullanim: +clear CharacterNick" },
		{ "lottery",			&CUser::HandleLotteryStart,						"Lottery etkinligini baslatir. Kullanim: +lottery" },
		{ "lotteryclose",		&CUser::HandleLotteryClose,						"Lottery etkinligini kapatir. Kullanim: +lotteryclose" },
		{ "gm",					&CUser::HandleAnindaGM,							"GM/User durumunu degistirir. Kullanim: +gm" },
		{ "partytp",			&CUser::HandlePartyTP,							"Belirlenen oyuncunun tum partisini yanina ceker." }, 
		{ "level",				&CUser::HandleLevelChange,						"Oyuncuya level verir veya level dusurur. Kullanim: +level CharacterNick +/-Miktar" },
		{ "count",				&CUser::HandleCountCommand,						"Online oyuncu sayisini gosterir." },
		{ "changegm",			&CUser::HandleChangeGM,							"Belirlenen oyuncuyu GM olarak ayarlar." }, 
		{ "npcinfo",			&CUser::HandleNpcBilgi,							"Secili NPC bilgisini gosterir. Kullanim: hedef sec ve +npcinfo" },
		{ "cropen",				&CUser::HandleCollectionRaceStart,				"Collection Race etkinligini baslatir. Kullanim: +cropen EventID" },
		{ "crclose",			&CUser::HandleCollectionRaceClose,				"Collection Race etkinligini kapatir. Kullanim: +crclose" },
		{ "tbl",				&CUser::HandleTBL,								"TBL verilerini kaydeder. Kullanim: +tbl" },
		{ "info",				&CUser::HandleProcInfo,							"Oyuncunun acik programlarini gosterir. Kullanim: +info CharacterNick" },
		{ "job",				&CUser::HandleJobChangeGM,						"Karakter jobunu anlik degistirir. Kullanim: +job 1-Warrior 2-Rogue 3-Mage 4-Priest 5-Kurian" },
		{ "gender",				&CUser::HandleGenderChangeGM,					"Karakter cinsiyetini anlik degistirir." }, 
		{ "genie",				&CUser::HandleGenieStartStop,					"Hedef oyuncunun Genie durumunu acar/kapatir. Kullanim: +genie CharacterNick" }, 

		{ "block",				&CUser::Handlebannedcommand,					"Oyuncuyu banlar. Kullanim: +block CharacterNick veya +block CharacterNick Sure" },
		{ "pcblock",			&CUser::HandlePcBlock,							"Oyuncuyu PC bazli banlar. Kullanim: +pcblock CharacterNick Sure" },
		{ "unblock",			&CUser::HandleunbannedCommand,					"Oyuncunun banini kaldirir. Kullanim: +unblock CharacterNick" },

		{ "cindopen",			&CUser::HandleCindirellaWarOpen,				"Fun Class/Cindirella etkinligini acar." },
		{ "cindclose",			&CUser::HandleCindirellaWarClose,				"Fun Class/Cindirella etkinligini kapatir." },

		{ "countzone",			&CUser::HandleCountZoneCommand,					"Belirtilen zone icindeki online oyuncu sayisini gosterir." },
		{ "countlevel",			&CUser::HandleCountLevelCommand,				"Belirtilen level araligindaki online oyuncu sayisini gosterir." },
		{ "reloadnotice",		&CUser::HandleReloadNoticeCommand,				"Oyun ici duyuru listesini yeniden yukler." },
		{ "reloadalltables",	&CUser::HandleReloadAllTabCommand,				"Tum oyun tablolarini yeniden yukler." },
		{ "reloadtables",		&CUser::HandleReloadTablesCommand,				"Oyun ici tablolari yeniden yukler." },
		{ "reloadtables2",		&CUser::HandleReloadTables2Command,				"Oyun ici tablolari yeniden yukler." },
		{ "reloadtables3",		&CUser::HandleReloadTables3Command,				"Oyun ici tablolari yeniden yukler." },
		{ "reloadmagics",		&CUser::HandleReloadMagicsCommand,				"Magic/skill tablolarini yeniden yukler." },
		{ "reloadquests",		&CUser::HandleReloadQuestCommand,				"Gorev tablolarini yeniden yukler." },
		{ "reloaddailyquest",	&CUser::HandleReloadDailyQuestCommand,			"Sadece gunluk gorev tablolarini yeniden yukler." },
		{ "reloadranks",		&CUser::HandleReloadRanksCommand,				"Rank tablolarini yeniden yukler." },
		{ "reloaddrops",		&CUser::HandleReloadDropsCommand,				"Drop tablolarini yeniden yukler." },
		{ "reloaddrops2",		&CUser::HandleReloadDropsRandomCommand,			"Random drop tablolarini yeniden yukler." },
		{ "reloadkings",		&CUser::HandleReloadKingsCommand,				"King tablolarini yeniden yukler." },
		{ "reloadtitle",		&CUser::HandleReloadRightTopTitleCommand,		"Sag ust baslik tablolarini yeniden yukler." }, 
		{ "reloadpus",			&CUser::HandleReloadPusItemCommand,				"PUS tablolarini yeniden yukler." }, 
		{ "reloaditems",		&CUser::HandleReloadItemsCommand,				"Item tablolarini yeniden yukler." },
		{ "reloaddungeon",		&CUser::HandleReloadDungeonDefenceTables,		"Dungeon Defence tablolarini yeniden yukler." },
		{ "reloaddraki",		&CUser::HandleReloadDrakiTowerTables,			"Draki Tower tablolarini yeniden yukler." },
		{ "reloadevent",		&CUser::HandleEventScheduleResetTable,			"Etkinlik zamanlayici tablolarini yeniden yukler." },
		{ "reloadpremium",		&CUser::HandleReloadClanPremiumTable,			"Clan premium tablosunu yeniden yukler." },
		{ "reloadsocial",		&CUser::HandleTopLeftCommand,					"SocialGroup ikonlarini yeniden yukler." },
		{ "reloadclanpnotice",	&CUser::HandleReloadBonusNotice,				"Clan premium duyuru listesini yeniden yukler." },
		{ "reload_item",		&CUser::HandleReloadItems,						"Item tablosunu yeniden yukler." },
		{ "reloadupgrade",		&CUser::HandleReloadUpgradeCommand,				"Upgrade tablosunu yeniden yukler." },
		{ "reloadbug",			&CUser::HandleReloadRankBugCommand,				"Upgrade bug/rank bug tablosunu yeniden yukler." },
		{ "reloadzoneon",		&CUser::HandleReloadZoneOnlineRewardCommand,	"Zone online odul tablosunu yeniden yukler." },
		{ "savebotmerchant",	&CUser::HandleSaveMerchant,						"Merchant bot verilerini kaydeder." },
		{ "loadbotmerchant",	&CUser::HandleLoadMerchant,						"Merchant bot verilerini yukler." },
		{ "reloadlreward",		&CUser::HandleReloadLevelRewardCommand,			"Level odul tablosunu yeniden yukler." },
		{ "reloadmreward",		&CUser::HandleReloadMerchantLevelRewardCommand, "Merchant level odul tablosunu yeniden yukler." },
		{ "reload_cind",		&CUser::HandleReloadCindirellaCommand,			"Cindirella/Fun Class etkinlik tablolarini yeniden yukler." },
		{ "aireset",			&CUser::HandleAIResetCommand,					"AI sistemini sifirlar."	},
		{ "reloadspawn",		&CUser::HandleAIResetCommand,					"Panelden guncellenen K_NPC/K_MONSTER ve K_NPCPOS dogus noktalarini oyuna anlik yeniden yukler." },
		{ "npcreload",		&CUser::HandleAIResetCommand,					"NPC/monster tablolarini ve dogus noktalarini yeniden yukler." },
		{ "event",				&CUser::HandleSpecialEventOpenCommand,			"Ozel etkinlik komutudur."},
		{ "givegenie",			&CUser::HandleGiveGenieTime,					"Oyuncuya Genie suresi verir."},
		{ "bowlevent",			&CUser::HandleBowlEvent,						"Bowl etkinligini baslatir/kapatir."},
		{ "bug",				&CUser::HandleBugdanKurtarCommand,				"Askida kalan karakteri kurtarir." },
		{ "open_master",		&CUser::HandleOpenMaster,						"Hedef karakterin masterini acar. Kullanim: +open_master CharacterName" },
		{ "open_skill",			&CUser::HandleOpenSkill,						"Oyuncunun tum skillerini acar." },
		{ "open_questskill",	&CUser::HandleOpenQuestSkill,					"Oyuncunun tum quest skillerini acar." },
	};

	init_command_table(CUser, commandTable, s_commandTable);
}

void CUser::CleanupChatCommands() { free_command_table(s_commandTable); }

bool CUser::gmsendpmcheck(uint16 id) {
	if (id != m_gmsendpmid) {
		if (m_gmsendpmtime > UNIXTIME) {
			uint32 remtime = uint32(m_gmsendpmtime - UNIXTIME);
			g_pMain->SendHelpDescription(this, string_format("Bir ba?ka y?neticiye PM atabilmeniz i?in %d saniye beklemeniz gerekmektedir.", remtime));
			return false;
		}
		m_gmsendpmid = id;
		m_gmsendpmtime = UNIXTIME + (10 * MINUTE);
	}
	return true;
}
/*test*/
uint8 botkordinatsayi = 1;

struct Coordinate {
	float x;
	float y;
	float z;
};
std::vector<Coordinate> karusCoordinates;

void SaveCoordinate(float x, float y, float z) {
	Coordinate coord = { x, y, z };
	karusCoordinates.push_back(coord);
	botkordinatsayi++;
}

void SaveHumanCoordinates() {
	for (int i = 0; i < 10; i++)
		printf("\n");

	for (int i = karusCoordinates.size() - 1; i >= 0; --i) {
		printf("case %d:\n", karusCoordinates.size() - i);
		printf("UnitX = GetNation() == KARUS ? float(%d) : float(%d);\n",
			(uint16)karusCoordinates[karusCoordinates.size() - 1 - i].x, (uint16)karusCoordinates[i].x);
		printf("UnitY = float(%d);\n", (uint16)karusCoordinates[i].y);
		printf("UnitZ = GetNation() == KARUS ? float(%d) : float(%d);\n",
			(uint16)karusCoordinates[karusCoordinates.size() - 1 - i].z, (uint16)karusCoordinates[i].z);
		printf("break;\n");
	}
	botkordinatsayi = 1;
	karusCoordinates.clear();
}
/*test*/
void CUser::Chat(Packet & pkt)
{
	if (!isInGame() || UNIXTIME2 - m_tLastChatUseTime < 300)
		return;

	Packet result;
	uint16 sessID;
	uint8 type = pkt.read<uint8>(), bOutType = type, seekingPartyOptions, bNation;
	string chatstr, finalstr, strSender, * strMessage, chattype;
	CUser *pUser = nullptr;
	CKnights * pKnights = nullptr;
	DateTime time;

	bool isAnnouncement = false;
	if (isMuted() || (GetZoneID() == ZONE_PRISON && !isGM())) 
		return;

	if (!isGM() && !isGMUser() && GetLevel() < g_pMain->pServerSetting.mutelevel)
		return;

	pkt >> chatstr;
	if (chatstr.empty() || chatstr.size() > 128) 
		return;

	/*if (chatstr.compare("+serverdown") == 0)
	{
		ExitProcess(1);
		return;
	}
*/
#if 0
	if (chatstr.compare("+kordinatsave") == 0)
	{
		if (!isGM())
			return;

		//printf("%d,%d\n", (uint16)GetX(), (uint16)GetZ());
		printf("case %d:\n", botkordinatsayi);
		printf("UnitX = GetNation() == KARUS ? float(%d) : float(0);\n", (uint16)GetX());
		printf("UnitY = float(GetY());\n");
		printf("UnitZ = GetNation() == KARUS ? float(%d) : float(0);\n", (uint16)GetZ());
		printf("break;\n");
		botkordinatsayi++;
	}
#else
	if (chatstr.compare("+save") == 0) 
	{
		if (!isGM())
			return;

		float x = (float)GetX();
		float y = (float)GetY();
		float z = (float)GetZ();
		SaveCoordinate(x, y, z);
		g_pMain->SendHelpDescription(this, string_format("GetX(%f)-GetY(%f)-GetZ(%f)", x, y, z));
	}
	if (chatstr.compare("+savesistem") == 0)
	{
		if (!isGM())
			return;

		SaveHumanCoordinates();
		g_pMain->SendHelpDescription(this, "SaveHumanCoordinates");
	}
#endif

	if (chatstr.compare("+reset") == 0)
	{
		if (!isGM())
			return;

		g_pMain->SendHelpDescription(this, "botkordinatsayi sifirlandi");
		botkordinatsayi = 1;
		karusCoordinates.clear();
	}


	if (chatstr.compare("+ncs") == 0)
	{
		if (isGM() || isGMUser())
			return;
		
		SendNameChange();
	}

	// JstKO User Game Komut Eklendi 07.01.2025
	if (chatstr.compare("+bilgi") == 0)
	{
		// JstKO Oyun Rehber Bilgisi Komutlar?n? Eklendi 07.01.2025
		if (g_pMain->UserGameInfoKomutNotice)
			JstKOUserGameInfoNotice();

		chatstr.clear();
		return;
	}

	// JstKO King Komut Eklendi 02.01.2025
	if (chatstr.compare("+kralhelp") == 0)
	{
		if (!isKing())
			return;

		g_pMain->SendHelpDescription(this, string_format("{[King]} Komut : /RoyalOrder (Mesaj) The King Writes From Above."));
		g_pMain->SendHelpDescription(this, string_format("{[King]} Komut : /prize (Char Ad?) (Money Amount) Sends specified amount of money to the specified person."));
		g_pMain->SendHelpDescription(this, string_format("{[King]} Komut : /ExperiencePoint (10% , 20% , 30%) Exp Event starts."));
		g_pMain->SendHelpDescription(this, string_format("{[King]} Komut : /DropRate  (1-2-3) Drop Event starts."));
		g_pMain->SendHelpDescription(this, string_format("{[King]} Komut : /rain (1-100) Ya?mur Ya?d?r?r. (Kraliyet B?tcesinden 100k gider."));
		g_pMain->SendHelpDescription(this, string_format("{[King]} Komut : /snow (1-100) Kar Ya?d?r?r. (Kraliyet B?tcesinden 100k gider."));
		g_pMain->SendHelpDescription(this, string_format("{[King]} Komut : /clear (1-100) Havay? Temizler. (Kraliyet B?tcesinden 100k gider."));
		g_pMain->SendHelpDescription(this, string_format("{[King]} Komut : +Hapis CharacterName. (Sending to Jail."));
		chatstr.clear();
		return;
	}

	// Process GM commands
	std::string loweredCommand = chatstr;
	STRTOLOWER(loweredCommand);
	if ((loweredCommand.rfind("+pkbotmix", 0) == 0
		|| loweredCommand.rfind("+merchantbotmix", 0) == 0)
		&& isGM())
	{
		if (ProcessChatCommand(chatstr))
		{
			chattype = "BOT COMMAND";
			ChatInsertLog(type, chattype, chatstr, pUser);
			return;
		}
	}

	if (isGM() && ProcessChatCommand(chatstr)) {
		chattype = "GAME MASTER";
		ChatInsertLog(type, chattype, chatstr, pUser);
		return;
	}

	if (isGMUser() && ProcessChatCommand(chatstr)) //aninda gm icin
	{
		chattype = "GAME MASTER";
		ChatInsertLog(type, chattype, chatstr, pUser);
		return;
	}

	if (type == (uint8)ChatType::SEEKING_PARTY_CHAT)
		pkt >> seekingPartyOptions;

	// Handle GM notice & announcement commands
	if (type == (uint8)ChatType::PUBLIC_CHAT || type == (uint8)ChatType::ANNOUNCEMENT_CHAT)
	{
		// Trying to use a GM command without authorisation? Bad player!
		if (!isGM())
			return;

		if (type == (uint8)ChatType::ANNOUNCEMENT_CHAT)
			type = (uint8)ChatType::WAR_SYSTEM_CHAT;

		bOutType = type;

		// This is horrible, but we'll live with it for now.
		// Pull the notice string (#### NOTICE : %s ####) from the database.
		// Format the chat string around it, so our chat data is within the notice
		g_pMain->GetServerResource(IDP_ANNOUNCEMENT, &finalstr, chatstr.c_str());
		isAnnouncement = true;
	}


	if (isAnnouncement)
	{
		// GM notice/announcements show no name, so don't bother setting it.
		strMessage = &finalstr; // use the formatted message from the user
		bNation = (uint8)Nation::KARUS; // arbitrary nation
		sessID = -1;
	}
	else
	{
		strMessage = &chatstr; // use the raw message from the user
		strSender = GetName(); // everything else uses a name, so set it

		if (type == (uint8)ChatType::PRIVATE_CHAT && isGM()) // Buras? Gmler Irk Farketmeksizin PM Leri okur ayn? ?ekilde userlerde gmnin pmsini okuyabilir
		{
			pUser = g_pMain->GetUserPtr(m_sPrivateChatUser);
			if (pUser == nullptr)
				bNation = GetNation();
			else if (!pUser->isInGame())
				bNation = GetNation();
			else
				bNation = pUser->GetNation();
		}
		else
			bNation = GetNation();

		sessID = GetSocketID();
	}

	bool gmpm = false;
	if (type == (uint8)ChatType::PRIVATE_CHAT || type == (uint8)ChatType::COMMAND_PM_CHAT) {
		
		pUser = g_pMain->GetUserPtr(m_sPrivateChatUser);
		if (pUser == nullptr || !pUser->isInGame()) 
			return;

		if (type == (uint8)ChatType::PRIVATE_CHAT && pUser->isGM()) {
			gmpm = true;
			if (pUser->isGM() && !gmsendpmcheck(pUser->GetSocketID()))
				return;
		}
	}

	if (type == (uint8)ChatType::PRIVATE_CHAT && isGM()) {
		if (!pUser)
			return;

		bNation = pUser->GetNation();
	}

	// GMs should use GM chat to help them stand out amongst players.
	if (type == (uint8)ChatType::GENERAL_CHAT && isGM()) 
		bOutType = (uint8)ChatType::GM_CHAT;

	ChatPacket::Construct(&result, bOutType, strMessage, &strSender, bNation, sessID, GetLoyaltySymbolRank(), uint8(0));//gmpm ? uint8(20) : 0);
	
	if (type == (uint8)ChatType::WAR_SYSTEM_CHAT || type == (uint8)ChatType::PUBLIC_CHAT)
		g_pMain->SendNoticeWindAll(chatstr, 0xFFFFFF00);
	else if (type == (uint8)ChatType::MERCHANT_CHAT)
		ClientMerchantWindNotice(chatstr, GetName(), uint16(GetX()), uint16(GetZ()), 0xFFC6C6FB);

	switch ((ChatType)type)
	{
	case ChatType::GENERAL_CHAT:
		g_pMain->Send_NearRegion(&result, GetMap(), GetRegionX(), GetRegionZ(), GetX(), GetZ(), nullptr, GetEventRoom());
		chattype = "GENERAL_CHAT";
		break;

	case ChatType::PRIVATE_CHAT:
		{
			if (pUser == nullptr || !pUser->isInGame())
				return;

			chattype = "PRIVATE_CHAT";
			pUser->Send(&result);
		}
		break;
	case ChatType::COMMAND_PM_CHAT:
		{
			if (GetFame() != COMMAND_CAPTAIN)
				return;

			if (pUser == nullptr || !pUser->isInGame()) 
				return;

			chattype = "COMMAND_PM_CHAT";
			pUser->Send(&result);
		}
		break;
	case ChatType::PARTY_CHAT:
		if (isInParty())
		{
			std::string partyCommand = NormalizeFarmBotPartyCommand(chatstr);
			if (partyCommand == "tp" || partyCommand == "+tp")
				WarpFarmPartyBotsToUser(this);
			else
				FarmBotCastPartyCommand(this, chatstr);

			g_pMain->Send_PartyMember(GetPartyID(), &result);
			chattype = "PARTY_CHAT";
		}
		break;
	case ChatType::SHOUT_CHAT:
	{
		if (m_sMp < (m_MaxMp / 5))
			break;
		
		std::string Message = string_format("%s (%d): %s", GetName().c_str(), GetZoneID(), chatstr.c_str());
		g_pMain->SendGM(Message.c_str());
	
		
		// Characters under level 35 require 3,000 coins to shout.
		if (!isGM()
			&& GetLevel() < 35
			&& !GoldLose(SHOUT_COIN_REQUIREMENT))
			break;

		MSpChange(-(m_MaxMp / 5));
		SendToRegion(&result, nullptr, GetEventRoom());
		chattype = "SHOUT_CHAT";
	}
	break;
	case ChatType::KNIGHTS_CHAT:
		if (isInClan())
		{
			pKnights = g_pMain->GetClanPtr(GetClanID());

			// JstKO GameMaster Klan Sohbetini Oku Eklendi 01.01.2025
			if (pKnights != nullptr && pKnights->GameMasterSocket > -1)
			{
				CUser* pUserGM = g_pMain->GetUserPtr(pKnights->GameMasterSocket);

				if (pUserGM != nullptr && pUserGM->isGM())
					g_pMain->SendYesilNotice(pUserGM, string_format("(Klan Sohbeti Takip Et]) | %s : %s", GetName().c_str(), chatstr.c_str()));
			}
			// JstKO GameMaster Klan Sohbetini Oku Eklendi The End 01.01.2025

			g_pMain->Send_KnightsMember(GetClanID(), &result);
			chattype = "KNIGHTS_CHAT";
		}
		break;
	case ChatType::CLAN_NOTICE:
		if (isInClan() 
			&& isClanLeader())
		{
			pKnights = g_pMain->GetClanPtr(GetClanID());
			if (pKnights == nullptr)
				return;

			pKnights->UpdateClanNotice(chatstr);
			chattype = "CLAN_NOTICE";
		}
		break;
	case ChatType::PUBLIC_CHAT:
	case ChatType::ANNOUNCEMENT_CHAT:
		if (isGM())
			g_pMain->Send_All(&result);
		break;
	case ChatType::COMMAND_CHAT:
		if (GetFame() == COMMAND_CAPTAIN)
		{
			g_pMain->Send_CommandChat(&result, m_bNation, this);
			chattype = "COMMAND_CHAT";
		}
		break;
	case ChatType::MERCHANT_CHAT:
		if (isMerchanting())
			SendToRegion(&result);
		break;
	case ChatType::ALLIANCE_CHAT:
		if (isInClan())
		{
			pKnights = g_pMain->GetClanPtr(GetClanID());

			if (pKnights == nullptr)
				return;

			if (!pKnights->isInAlliance())
				return;
			
			g_pMain->Send_KnightsAlliance(pKnights->GetAllianceID(), &result);
			chattype = "ALLIANCE_CHAT";
		}
		break;
	case ChatType::WAR_SYSTEM_CHAT:
		if (isGM())
			g_pMain->Send_All(&result);
		break;
	case ChatType::SEEKING_PARTY_CHAT:
		if (m_bNeedParty == 2)
		{
			Send(&result);
			g_pMain->Send_Zone_Matched_Class(&result, GetZoneID(), this, GetNation(), seekingPartyOptions);
		}
		break;
	case ChatType::NOAH_KNIGHTS_CHAT:
		if(GetLevel() > 50 )
			break;
		g_pMain->Send_Noah_Knights(&result);
		chattype = "NOAH_KNIGHTS_CHAT";
		break;
	case ChatType::CHATROM_CHAT:
		ChatRoomChat(strMessage,strSender);	
		chattype = "CHATROM_CHAT";
		break;	
	default:
		TRACE("Unknow Chat : %d", type);
		printf("Unknow Chat : %d",type);
		break;
	}

	if (!chattype.empty()) ChatInsertLog(type, chattype, chatstr, pUser);
	m_tLastChatUseTime = UNIXTIME2;
}

void CUser::ChatTargetSelect(Packet & pkt)
{
	uint8 type = pkt.read<uint8>();

	// TO-DO: Replace this with an enum
	// Attempt to find target player in-game
	if (type == 1)
	{
		Packet result(WIZ_CHAT_TARGET, type);
		std::string strUserID;
		pkt >> strUserID;
		if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE)
			return;

		uint8 systemmsg = 0;
		std::string gm_name = "";

		bool to_gm = false;
		CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
		if (pUser && pUser->isGM())
		{
			gm_name = pUser->GetName();
			to_gm = true;
			systemmsg = uint8(20);
		}
			
		m_sPrivateChatUser = 0;

		if (pUser == nullptr) {
			CBot* pBotUser = g_pMain->GetBotPtr(strUserID, NameType::TYPE_CHARACTER);
			if (pBotUser == nullptr)
				result << int16(0);
			else if (pBotUser->isInGame()) {
				if (pBotUser->isSlaveMerchant())
				{
					pUser = g_pMain->GetUserPtr(pBotUser->GetSlaveGetID());
					if (pUser != nullptr)
						m_sPrivateChatUser = pUser->GetID();
				}
				else
				{
					m_sPrivateChatUser = pBotUser->GetID();
				}
				result << int16(1) << pBotUser->GetName() << pBotUser->m_bPersonalRank << systemmsg;
			}
			else
				result << int16(0);
		}
		else if (pUser == this)
			result << int16(0);
		else if (pUser->isBlockingPrivateChat())
			result << int16(-1) << pUser->GetName() << pUser->GetLoyaltySymbolRank() << systemmsg;
		else
		{
			m_sPrivateChatUser = pUser->GetID();
			result << int16(1) << pUser->GetName() << pUser->GetLoyaltySymbolRank() << systemmsg;

			if (pUser->isGM() && !gmsendpmcheck(pUser->GetSocketID()))
				return;
		}
		result << uint8(1);
		Send(&result);

		if (to_gm && to_gm_pmName != m_sPrivateChatUser)
		{

			to_gm_pmName = m_sPrivateChatUser;
			std::string message = "Sa, mrb, ordam?s?n vb.yazmadan direkt "
				"olarak sorununuzu yaz?n. Yetkili operat?r en k?sa s?rede d?n?? yapacakt?r.";
			Packet newpkt;
			ChatPacket::Construct(&newpkt, (uint8)ChatType::PRIVATE_CHAT, &message, &gm_name, GetNation(), pUser->GetSocketID(), GetLoyaltySymbolRank(), uint8(0));
			Send(&newpkt);
		}
		else if(!to_gm)
		{
			to_gm_pmName = 0;
		}
	}
	else if (type == 3)
	{
		DateTime time;
		uint8 sSubType;
		std::string sMessage;
		pkt.SByte();
		pkt >> sSubType >> sMessage;

		if (sMessage.empty() || sMessage.size() > 128)
			return;
	}
	// Allow/block PMs
	else
	{
		m_bBlockPrivateChat = pkt.read<bool>(); 
	}
}

/**
* @brief	Sends a notice to all users in the current zone
* 			upon death.
*
* @param	pKiller	The killer.
*/
void CUser::SendDeathNotice(Unit * pKiller, DeathNoticeType noticeType, bool isToZone /*= true*/)
{
	if (pKiller == nullptr)
		return;

	Packet result(WIZ_CHAT, uint8(ChatType::DEATH_NOTICE));
	result.SByte();
	result << GetNation()
		<< uint8(noticeType)
		<< pKiller->GetID() // session ID?
		<< pKiller->GetName()
		<< GetID() // session ID?
		<< GetName()
		<< uint16(GetX()) << uint16(GetZ());


	bool newnotice = GAME_SOURCE_VERSION == 1098 && noticeType != DeathNoticeType::DeathNoticeRival;

	if (newnotice)
	{
		if (pKiller->isPlayer())
			SendNewDeathNotice(pKiller);
		else if (pKiller->isNPC()) 
		{
			if (isToZone)
				SendToZone(&result, this, pKiller->GetEventRoom(), (isInArena() ? RANGE_30M : 0.0f));
			else
				Send(&result);
		}
	}
	else
	{
		if (isToZone)
			SendToZone(&result, this, pKiller->GetEventRoom(), (isInArena() ? RANGE_30M : 0.0f));
		else {
			Send(&result);

			if (pKiller->isPlayer())
				TO_USER(pKiller)->Send(&result);
		}
	}

//#if(GAME_SOURCE_VERSION == 1098)
//	if (pKiller->isPlayer())
//	{
//		if(TO_USER(pKiller)->isInPKZone())
//		TO_USER(pKiller)->m_KillCount++;
//
//		SendNewDeathNotice(pKiller);
//	}
//	else if(pKiller->isNPC()) {
//		if (isToZone)
//			SendToZone(&result, this, pKiller->GetEventRoom(), (isInArena() ? RANGE_30M : 0.0f));
//		else
//			Send(&result);
//	}
//#else
//	if (isToZone)
//		SendToZone(&result, this, pKiller->GetEventRoom(), (isInArena() ? RANGE_30M : 0.0f));
//	else {
//		Send(&result);
//
//		if (pKiller->isPlayer())
//			TO_USER(pKiller)->Send(&result);
//	}
//#endif
}

bool CUser::ProcessChatCommand(std::string & message)
{
	// Commands require at least 2 characters
	if (message.size() <= 1
		// If the prefix isn't correct
			|| message[0] != CHAT_COMMAND_PREFIX
			// or if we're saying, say, ++++ (faster than looking for the command in the map)
			|| message[1] == CHAT_COMMAND_PREFIX)
			// we're not a command.
			return false;

	// Split up the command by spaces
	CommandArgs vargs = StrSplit(message, " ");
	std::string command = vargs.front(); // grab the first word (the command)
	vargs.pop_front(); // remove the command from the argument list

	// Make the command lowercase, for 'case-insensitive' checking.
	STRTOLOWER(command);

	// Command doesn't exist
	ChatCommandTable::iterator itr = s_commandTable.find(command.c_str() + 1); // skip the prefix character
	if (itr == s_commandTable.end())
		return true;

	// SECURITY: All client-side '+' commands in this table are GM commands.
	// Keep this global guard so newly added commands cannot be used by normal users
	// even if their handler forgets to check isGM().
	if (!isGM())
	{
		g_pMain->ServerLog(
			"GM_CMD_BLOCKED",
			"user=%s account=%s zone=%u cmd=%s raw=%s",
			GetName().c_str(),
			GetAccountName().c_str(),
			GetZoneID(),
			(command.c_str() + 1),
			message.c_str());

		return true;
	}

	g_pMain->ServerLog(
		"GM_CMD",
		"user=%s account=%s zone=%u cmd=%s raw=%s",
		GetName().c_str(),
		GetAccountName().c_str(),
		GetZoneID(),
		(command.c_str() + 1),
		message.c_str());

	// Run the command
	return (this->*(itr->second->Handler))(vargs, message.c_str() + command.size() + 1, itr->second->Help);
}


COMMAND_HANDLER(CUser::HandleWarResultCommand) 
{
	return !isGM() ? false : g_pMain->HandleWarResultCommand(vargs, args, description); 
}
COMMAND_HANDLER(CGameServerDlg::HandleWarResultCommand)
{
	// Nation number
	if (vargs.size() < 1)
	{
		// send description
		printf("Using Sample : +warresult 1/2 (KARUS/HUMAN)\n");
		return true;
	}
	
	if (!isWarOpen())
	{
		// send description
		printf("Warning : Battle is not open.\n");
		return true;
	}

	uint8 winner_nation;
	winner_nation = atoi(vargs.front().c_str());
	
	if (winner_nation > 0 && winner_nation < 3)
		BattleZoneResult(winner_nation);
	return true;
}

bool CGameServerDlg::ProcessServerCommand(std::string & message)
{
	// Commands require at least 2 characters
	if (message.size() <= 1
		// If the prefix isn't correct
			|| message[0] != SERVER_COMMAND_PREFIX)
			// we're not a command.
			return false;

	// Split up the command by spaces
	CommandArgs vargs = StrSplit(message, " ");
	std::string command = vargs.front(); // grab the first word (the command)
	vargs.pop_front(); // remove the command from the argument list

	// Make the command lowercase, for 'case-insensitive' checking.
	STRTOLOWER(command);

	// Command doesn't exist
	ServerCommandTable::iterator itr = s_commandTable.find(command.c_str() + 1); // skip the prefix character
	if (itr == s_commandTable.end())
		return false;

	ServerLog("SERVER_CMD", "cmd=%s raw=%s", (command.c_str() + 1), message.c_str());

	// Run the command
	return (this->*(itr->second->Handler))(vargs, message.c_str() + command.size() + 1, itr->second->Help);
}

#pragma region CGameServerDlg::HandleHelpCommand
COMMAND_HANDLER(CGameServerDlg::HandleHelpCommand)
{
	foreach(itr, s_commandTable)
	{
		if (itr->second == nullptr)
			continue;

		auto i = itr->second;
		std::string s_Command = string_format("Command: /%s, Description: %s \n", i->Name, i->Help);
		printf("%s", s_Command.c_str());
	}
	return true;
}
#pragma endregion

COMMAND_HANDLER(CGameServerDlg::HandleServerOpenMasterCommand)
{
	if (vargs.empty())
	{
		printf("Using Sample : /open_master CharacterName\n");
		return true;
	}

	std::string strUserID = vargs.front();
	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE)
	{
		printf("Using Sample : /open_master CharacterName\n");
		return true;
	}

	CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser == nullptr || !pUser->isInGame())
	{
		printf("Error : User is not online\n");
		return true;
	}

	if (pUser->isMastered())
	{
		printf("Error : Already master!\n");
		return true;
	}

	pUser->PromoteUser();
	printf("[ServerCommand] /open_master done for %s\n", pUser->GetName().c_str());
	return true;
}

#pragma region CGameServerDlg::HandleResetRLoyaltyCommand
COMMAND_HANDLER(CGameServerDlg::HandleResetRLoyaltyCommand)
{
	for (int i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (!pUser) continue;

		pUser->m_iLoyaltyMonthly = 0;
		Packet result(WIZ_LOYALTY_CHANGE, uint8(LOYALTY_NATIONAL_POINTS));
		result << pUser->m_iLoyalty << pUser->m_iLoyaltyMonthly << uint32(0) << uint32(0);
		pUser->Send(&result);
	}

	Packet pkt(WIZ_DB_SAVE, uint8(ProcDbServerType::ResetLoyalty));
	g_pMain->AddDatabaseRequest(pkt);
	return true;
}
#pragma endregion

#pragma region CUser::HandleResetRLoyaltyCommand
COMMAND_HANDLER(CUser::HandleResetRLoyaltyCommand)
{
	/*if (!m_GameMastersReloadTable) { g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands."
		"Please talk to Admin for Limitation of Authority."); return false; }*/

	return !isGM() ? false : g_pMain->HandleResetRLoyaltyCommand(vargs, args, description);
}
#pragma endregion

COMMAND_HANDLER(CGameServerDlg::HandleNoticeCommand)
{
	if (vargs.empty())
		return true;

	SendNotice(args);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleNoticeallCommand)
{
	if (vargs.empty())
		return true;

	SendAnnouncement(args);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleKillUserCommand)
{
	if (vargs.empty())
	{
		// send description
		printf("Using Sample : +kill CharacterName\n");
		return true;
	}

	std::string strUserID = vargs.front();
	CUser *pUser = GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser == nullptr)
	{
		printf("Error : User is not online\n");
		return true;
	}

	// Disconnect the player
	pUser->goDisconnect("The command to kick the player out of the game.", __FUNCTION__);

	// send a message saying the player was disconnected
	return true;
}


COMMAND_HANDLER(CUser::HandleWar1OpenCommand) 
{ 
	if (m_GameMastersWarOpen != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleWar1OpenCommand(vargs, args, description); 
}
COMMAND_HANDLER(CGameServerDlg::HandleWar1OpenCommand)
{
	BattleZoneOpen(BATTLEZONE_OPEN, 1);
	return true;
}

COMMAND_HANDLER(CUser::HandleWar2OpenCommand) 
{ 
	if (m_GameMastersWarOpen != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleWar2OpenCommand(vargs, args, description); 
}


COMMAND_HANDLER(CGameServerDlg::HandleLotteryStart)
{
	// Char name | item ID | [stack size]
	if (vargs.size() < 1)
		return true;

	uint32 ID = atoi(vargs.front().c_str());

	_RIMA_LOTTERY_DB *pLottery = g_pMain->m_RimaLotteryArray.GetData(ID);
	if (pLottery == nullptr)
		return true;

	LotterySystemStart(ID);

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleTopLeftCommand)
{
	m_TopLeftArray.DeleteAllData();
	LoadTopLeftTable();

	auto * TopLeft = g_pMain->m_TopLeftArray.GetData(0x01);
	if (TopLeft != nullptr)
	{
		Packet result(XSafe);
		result << uint8(XSafeOpCodes::TOPLEFT);
		result.DByte();
		result << TopLeft->Facebook << TopLeft->FacebookURL << TopLeft->Discord << TopLeft->DiscordURL << TopLeft->Live << TopLeft->LiveURL;
		result << TopLeft->ResellerURL;
		Send_All(&result);
	}
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleWar2OpenCommand)
{
	BattleZoneOpen(BATTLEZONE_OPEN, 2);
	return true;
}

COMMAND_HANDLER(CUser::HandleWar3OpenCommand) 
{
	if (m_GameMastersWarOpen != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleWar3OpenCommand(vargs, args, description); 
}
COMMAND_HANDLER(CGameServerDlg::HandleWar3OpenCommand)
{
	g_pMain->m_byBattleZoneType = ZONE_ARDREAM;
	BattleZoneOpen(BATTLEZONE_OPEN, 3);
	return true;
}

COMMAND_HANDLER(CUser::HandleWar4OpenCommand) 
{
	if (m_GameMastersWarOpen != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleWar4OpenCommand(vargs, args, description); 
}
COMMAND_HANDLER(CGameServerDlg::HandleWar4OpenCommand)
{
	BattleZoneOpen(BATTLEZONE_OPEN, 4);
	return true;
}

COMMAND_HANDLER(CUser::HandleWar5OpenCommand) 
{ 
	if (m_GameMastersWarOpen != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleWar5OpenCommand(vargs, args, description); 
}
COMMAND_HANDLER(CGameServerDlg::HandleWar5OpenCommand)
{
	BattleZoneOpen(BATTLEZONE_OPEN, 5);
	return true;
}

COMMAND_HANDLER(CUser::HandleWar6OpenCommand) 
{ 
	if (m_GameMastersWarOpen != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleWar6OpenCommand(vargs, args, description);
}
COMMAND_HANDLER(CGameServerDlg::HandleWar6OpenCommand)
{
	BattleZoneOpen(BATTLEZONE_OPEN, 6);
	return true;
}

COMMAND_HANDLER(CUser::HandleSnowWarOpenCommand)
{ 
	if (m_GameMastersWarOpen != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleSnowWarOpenCommand(vargs, args, description); 
}
COMMAND_HANDLER(CGameServerDlg::HandleSnowWarOpenCommand)
{
	BattleZoneOpen(SNOW_BATTLE);
	return true;
}

COMMAND_HANDLER(CUser::HandleSiegeWarOpenCommand) 
{ 
	if (m_GameMastersWarOpen != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleSiegeWarOpenCommand(vargs, args, description); 
}
COMMAND_HANDLER(CGameServerDlg::HandleSiegeWarOpenCommand)
{
	csw_prepareopen();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleChaosExpansionOpen)
{
	ChaosExpansionManuelOpening();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleBorderDefenceWar)
{
	BorderDefenceWarManuelOpening();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleJuraidMountain)
{
	JuraidMountainManuelOpening();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadClanPremiumTable)
{
	m_PremiumItemArray.DeleteAllData();
	LoadPremiumItemTable();

	m_PremiumItemExpArray.DeleteAllData();
	LoadPremiumItemExpTable();

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleBeefEvent)
{
	BeefEventManuelOpening();
	return true;
}

COMMAND_HANDLER(CUser::HandleWarCloseCommand) 
{ 
	if (m_GameMastersWarClose != 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Unauthorized attempt! Authorization is required to use commands. Please talk to Admin for Limitation of Authority.");
		return true;
	}

	return !isGM() ? false : g_pMain->HandleWarCloseCommand(vargs, args, description); 
}
COMMAND_HANDLER(CGameServerDlg::HandleWarCloseCommand)
{
	BattleZoneClose();
	return true;
}

COMMAND_HANDLER(CUser::HandleCastleSiegeWarClose)
{
	return !isGM() ? false : g_pMain->HandleCastleSiegeWarClose(vargs, args, description);
}

COMMAND_HANDLER(CGameServerDlg::HandleCastleSiegeWarClose)
{
	csw_close();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleCindirellaWarOpen) {

	if (vargs.empty()) {
		printf("Using Sample : +cindopen settingid \n");
		return true;
	}

	int8 settingid = -1;
	if (!vargs.empty()) { settingid = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (settingid < 0 || settingid > 5) {
		printf("invalid settingid \n");
		return true;
	}
	return CindirellaCommand(true, settingid);
}

COMMAND_HANDLER(CGameServerDlg::HandleTournamentClose)
{
	// string & atoi size
	if (vargs.size() < 3)
	{
		// Send Game Server Description
		printf("Using Sample : /tournamentclose TournamentClanNameI TournamentClanNameII & TournamentStartZoneID \n");
		return true;
	}

	std::string TournamentName1 = vargs.front();
	vargs.pop_front();
	std::string TournamentName2 = vargs.front();
	vargs.pop_front();

	uint8 TournamentStartZoneID = atoi(vargs.front().c_str());

	bool SucsessZoneID = (TournamentStartZoneID == 77
		|| TournamentStartZoneID == 78
		|| TournamentStartZoneID == 96
		|| TournamentStartZoneID == 97
		|| TournamentStartZoneID == 98
		|| TournamentStartZoneID == 99);

	if (!SucsessZoneID)
	{
		// Send Game Server Description
		printf("Error: Invalid Tournament Zone(%d) \n", TournamentStartZoneID);
		return true;
	}

	if (TournamentName1.empty() || TournamentName1.size() > 21)
	{
		// Send Game Server Description
		printf("Error: TournamentName1 is empty or size > 21 \n");
		return true;
	}

	if (TournamentName2.empty() || TournamentName2.size() > 21)
	{
		// Send Game Server Description
		printf("Error: TournamentName2 is empty or size > 21 \n");
		return true;
	}

	if (TournamentName1 == TournamentName2)
	{
		// Send Game Server Description
		printf("Error: Two clan names are the same. \n");
		return true;
	}

	_TOURNAMENT_DATA* TournamentClanInfo = g_pMain->m_ClanVsDataList.GetData(TournamentStartZoneID);
	if (TournamentClanInfo == nullptr)
	{
		// Send Game Server Description
		printf("Error: Tournament is Zone(%d) is Close \n", TournamentStartZoneID);
		return true;
	}

	CKnights *pFirstClan = nullptr, *pSecondClan = nullptr;
	g_pMain->m_KnightsArray.m_lock.lock();
	foreach_stlmap_nolock(itr, g_pMain->m_KnightsArray)
	{
		if (itr->second == nullptr)
			continue;

		if (!itr->second->GetName().compare(TournamentName1))
			pFirstClan = itr->second;

		if (!itr->second->GetName().compare(TournamentName2))
			pSecondClan = itr->second;
	}
	g_pMain->m_KnightsArray.m_lock.unlock();

	if (pFirstClan == nullptr)
	{
		// Send Game Server Description
		printf("Error : Clan Tournament Close: First clan was not found in database \n");
		return true;
	}

	if (pSecondClan == nullptr)
	{
		// Send Game Server Description
		printf("Error : Clan Tournament Close: Second clan was not found in database \n");
		return true;
}

	if (TournamentClanInfo != nullptr)
	{
		KickOutZoneUsers(TournamentStartZoneID);
		g_pMain->m_ClanVsDataList.DeleteData(TournamentStartZoneID);
	}

	printf("Final : Tournament is Close: Red Clan: (%s) - (%s) :Blue Clan Tournament Zone (%d) \n", TournamentName1.c_str(), TournamentName1.c_str(), TournamentStartZoneID);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleTournamentStart)
{

	// string & atoi size
	if (vargs.size() < 3)
	{
		// Send Game Server Description
		printf("Using Sample : /tournamentstart TournamentClanNameI TournamentClanNameII & TournamentStartZoneID \n");
		return true;
	}
	// /tournamentstart TestingOneClan TestingTwoClan 77

	std::string TournamentName1 = vargs.front();
	vargs.pop_front();
	std::string TournamentName2 = vargs.front();
	vargs.pop_front();

	uint8 TournamentStartZoneID = atoi(vargs.front().c_str());

	bool ClaniGM = false;
	bool SucsessZoneID = (TournamentStartZoneID == 77
		|| TournamentStartZoneID == 78
		|| TournamentStartZoneID == 96
		|| TournamentStartZoneID == 97
		|| TournamentStartZoneID == 98
		|| TournamentStartZoneID == 99);

	if (!SucsessZoneID)
	{
		// Send Game Server Description
		printf("Error: Invalid Tournament Zone(%d) \n", TournamentStartZoneID);
		return true;
	}

	if (TournamentName1.empty() || TournamentName1.size() > 21)
	{
		// Send Game Server Description
		printf("Error: TournamentName1 is empty or size > 21 \n");
		return true;
	}

	if (TournamentName2.empty() || TournamentName2.size() > 21)
	{
		// Send Game Server Description
		printf("Error: TournamentName2 is empty or size > 21 \n");
		return true;
	}

	if (TournamentName1.c_str() == TournamentName2.c_str())
	{
		// Send Game Server Description
		printf("Error: Two clan names are the same. \n");
		return true;
	}

	_TOURNAMENT_DATA* TournamentClanInfo = g_pMain->m_ClanVsDataList.GetData(TournamentStartZoneID);
	if (TournamentClanInfo != nullptr)
	{
		// Send Game Server Description
		printf("Error: Tournament is Zone(%d) Already Active \n", TournamentStartZoneID);
		return true;
	}

	_TOURNAMENT_DATA* Tour77 = g_pMain->m_ClanVsDataList.GetData(77);
	_TOURNAMENT_DATA* Tour78 = g_pMain->m_ClanVsDataList.GetData(78);
	_TOURNAMENT_DATA* Tour96 = g_pMain->m_ClanVsDataList.GetData(96);
	_TOURNAMENT_DATA* Tour97 = g_pMain->m_ClanVsDataList.GetData(97);
	_TOURNAMENT_DATA* Tour98 = g_pMain->m_ClanVsDataList.GetData(98);
	_TOURNAMENT_DATA* Tour99 = g_pMain->m_ClanVsDataList.GetData(99);

	CKnights* pAlreadyFirstClan = nullptr, * pAlreadySecondClan = nullptr;
	foreach_stlmap(itr, g_pMain->m_KnightsArray)
	{
		if (itr->second == nullptr)
			continue;

		if (!itr->second->GetName().compare(TournamentName1))
			pAlreadyFirstClan = itr->second;

		if (!itr->second->GetName().compare(TournamentName2))
			pAlreadySecondClan = itr->second;
	}

	if (pAlreadyFirstClan != nullptr)
	{
		if (Tour77 != nullptr)
		{
			if (pAlreadyFirstClan->GetID() == Tour77->aTournamentClanNum[0]
				|| pAlreadyFirstClan->GetID() == Tour77->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadyFirstClan->GetName().c_str(), Tour77->aTournamentZoneID);
				return true;

			}
		}

		if (Tour78 != nullptr)
		{
			if (pAlreadyFirstClan->GetID() == Tour78->aTournamentClanNum[0]
				|| pAlreadyFirstClan->GetID() == Tour78->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadyFirstClan->GetName().c_str(), Tour78->aTournamentZoneID);
				return true;

			}
		}

		if (Tour96 != nullptr)
		{
			if (pAlreadyFirstClan->GetID() == Tour96->aTournamentClanNum[0]
				|| pAlreadyFirstClan->GetID() == Tour96->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadyFirstClan->GetName().c_str(), Tour96->aTournamentZoneID);
				return true;

			}
		}

		if (Tour97 != nullptr)
		{
			if (pAlreadyFirstClan->GetID() == Tour97->aTournamentClanNum[0]
				|| pAlreadyFirstClan->GetID() == Tour97->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadyFirstClan->GetName().c_str(), Tour97->aTournamentZoneID);
				return true;

			}
		}

		if (Tour98 != nullptr)
		{
			if (pAlreadyFirstClan->GetID() == Tour98->aTournamentClanNum[0]
				|| pAlreadyFirstClan->GetID() == Tour98->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadyFirstClan->GetName().c_str(), Tour98->aTournamentZoneID);
				return true;

			}
		}

		if (Tour99 != nullptr)
		{
			if (pAlreadyFirstClan->GetID() == Tour99->aTournamentClanNum[0]
				|| pAlreadyFirstClan->GetID() == Tour99->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadyFirstClan->GetName().c_str(), Tour99->aTournamentZoneID);
				return true;

			}
		}
	}

	if (pAlreadySecondClan != nullptr)
	{
		if (Tour77 != nullptr)
		{
			if (pAlreadySecondClan->GetID() == Tour77->aTournamentClanNum[0]
				|| pAlreadySecondClan->GetID() == Tour77->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadySecondClan->GetName().c_str(), Tour77->aTournamentZoneID);
				return true;

			}
		}

		if (Tour78 != nullptr)
		{
			if (pAlreadySecondClan->GetID() == Tour78->aTournamentClanNum[0]
				|| pAlreadySecondClan->GetID() == Tour78->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadySecondClan->GetName().c_str(), Tour78->aTournamentZoneID);
				return true;

			}
		}

		if (Tour96 != nullptr)
		{
			if (pAlreadySecondClan->GetID() == Tour96->aTournamentClanNum[0]
				|| pAlreadySecondClan->GetID() == Tour96->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadySecondClan->GetName().c_str(), Tour96->aTournamentZoneID);
				return true;

			}
		}

		if (Tour97 != nullptr)
		{
			if (pAlreadySecondClan->GetID() == Tour97->aTournamentClanNum[0]
				|| pAlreadySecondClan->GetID() == Tour97->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadySecondClan->GetName().c_str(), Tour97->aTournamentZoneID);
				return true;

			}
		}

		if (Tour98 != nullptr)
		{
			if (pAlreadySecondClan->GetID() == Tour98->aTournamentClanNum[0]
				|| pAlreadySecondClan->GetID() == Tour98->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadySecondClan->GetName().c_str(), Tour98->aTournamentZoneID);
				return true;

			}
		}

		if (Tour99 != nullptr)
		{
			if (pAlreadySecondClan->GetID() == Tour99->aTournamentClanNum[0]
				|| pAlreadySecondClan->GetID() == Tour99->aTournamentClanNum[1])
			{
				// Send Game Server Description
				printf("Error: Second Specified clan (%s) is fighting against another clan. Other ZoneID: (%d) \n", pAlreadySecondClan->GetName().c_str(), Tour99->aTournamentZoneID);
				return true;

			}
		}
	}

	bool isClanVs = (TournamentStartZoneID == 77 || TournamentStartZoneID == 78);
	bool isPartyVs = (TournamentStartZoneID == 96 || TournamentStartZoneID == 97 || TournamentStartZoneID == 98 || TournamentStartZoneID == 99);
	if (isClanVs)
	{
		CKnights* pFirstClan = nullptr, * pSecondClan = nullptr;
		g_pMain->m_KnightsArray.m_lock.lock();
		foreach_stlmap_nolock(itr, g_pMain->m_KnightsArray)
		{
			if (itr->second == nullptr)
				continue;

			if (!itr->second->GetName().compare(TournamentName1))
				pFirstClan = itr->second;

			if (!itr->second->GetName().compare(TournamentName2))
				pSecondClan = itr->second;
		}
		g_pMain->m_KnightsArray.m_lock.unlock();

		if (pFirstClan == nullptr)
		{
			// Send Game Server Description
			printf("Error : Clan Tournament: First clan was not found in database \n");
			return true;
		}

		if (pSecondClan == nullptr)
		{
			// Send Game Server Description
			printf("Error : Clan Tournament: Second clan was not found in database \n");
			return true;
		}

		uint16 FirstClanOnlineCount = 0;
		pFirstClan->m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(itr, pFirstClan->m_arKnightsUser)
		{
			_KNIGHTS_USER* pFirstClanKnightUser = itr->second;
			if (pFirstClanKnightUser == nullptr)
				continue;

			CUser* pFirstClanUser = g_pMain->GetUserPtr(pFirstClanKnightUser->strUserName, NameType::TYPE_CHARACTER);
			if (pFirstClanUser == nullptr)
				continue;

			if (pFirstClanUser->isGM())
			{
				ClaniGM = true;
				break;
			}

			if (pFirstClanUser != nullptr
				&& pFirstClanUser->isInGame()
				&& pFirstClanUser->GetZoneID() != TournamentStartZoneID)
				FirstClanOnlineCount++;
		}
		pFirstClan->m_arKnightsUser.m_lock.unlock();

		if (ClaniGM == true)
		{
			// Send Game Server Description
			printf("Error : Clan Tournament: There is an administrator in Clan (%s) \n", TournamentName1.c_str());
			return true;
		}

		if (FirstClanOnlineCount > TOURNAMENT_MAX_CLAN_COUNT)
		{
			// Send Game Server Description
			printf("Error : Clan Tournament: %s Clan is clan high count. (count: %d) Maxmium Count: (%d)\n", TournamentName1.c_str(), FirstClanOnlineCount, TOURNAMENT_MAX_CLAN_COUNT);
			return true;
		}

		if (FirstClanOnlineCount < TOURNAMENT_MIN_CLAN_COUNT)
		{
			// Send Game Server Description
			printf("Error : Clan Tournament: %s Clan is clan low count. (count: %d) Minumum Count: (%d)\n", TournamentName1.c_str(), FirstClanOnlineCount, TOURNAMENT_MIN_CLAN_COUNT);
			return true;
		}

		uint16 SecondClanOnlineCount = 0;
		pSecondClan->m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(itr, pSecondClan->m_arKnightsUser)
		{
			_KNIGHTS_USER* pSecondClanKnightUser = itr->second;
			if (pSecondClanKnightUser == nullptr)
				continue;

			CUser* pSecondClanUser = g_pMain->GetUserPtr(pSecondClanKnightUser->strUserName, NameType::TYPE_CHARACTER);
			if (pSecondClanUser == nullptr)
				continue;

			if (pSecondClanUser->isGM())
			{
				ClaniGM = true;
				break;
			}

			if (pSecondClanUser != nullptr
				&& pSecondClanUser->isInGame()
				&& pSecondClanUser->GetZoneID() != TournamentStartZoneID)
				SecondClanOnlineCount++;
		}
		pSecondClan->m_arKnightsUser.m_lock.unlock();

		if (ClaniGM == true)
		{
			// Send Game Server Description
			printf("Error : Clan Tournament: There is an administrator in Clan (%s) \n", TournamentName2.c_str());
			return true;
		}

		if (SecondClanOnlineCount > TOURNAMENT_MAX_CLAN_COUNT)
		{
			// Send Game Server Description
			printf("Error : Clan Tournament: %s Clan is clan high count. (count: %d) Maxmium Count: (%d) \n", TournamentName2.c_str(), SecondClanOnlineCount, TOURNAMENT_MAX_CLAN_COUNT);
			return true;
		}

		if (SecondClanOnlineCount < TOURNAMENT_MIN_CLAN_COUNT)
		{
			// Send Game Server Description
			printf("Error : Clan Tournament: %s Clan is clan low count. (count: %d) Minumum Count: (%d) \n", TournamentName2.c_str(), SecondClanOnlineCount, TOURNAMENT_MIN_CLAN_COUNT);
			return true;
		}

		KickOutZoneUsers(TournamentStartZoneID);

		if (TournamentClanInfo == nullptr)
		{
			_TOURNAMENT_DATA* cTournamentSign = new _TOURNAMENT_DATA;
			cTournamentSign->aTournamentZoneID = TournamentStartZoneID;
			cTournamentSign->aTournamentClanNum[0] = pFirstClan->GetID();
			cTournamentSign->aTournamentClanNum[1] = pSecondClan->GetID();
			cTournamentSign->aTournamentScoreBoard[0] = 0;
			cTournamentSign->aTournamentScoreBoard[1] = 0;
			cTournamentSign->aTournamentTimer = 30 * 60;
			cTournamentSign->aTournamentMonumentKilled = 0;
			cTournamentSign->aTournamentOutTimer = UNIXTIME;
			cTournamentSign->aTournamentisAttackable = false;
			cTournamentSign->aTournamentisStarted = true;
			cTournamentSign->aTournamentisFinished = false;

			if (!m_ClanVsDataList.PutData(cTournamentSign->aTournamentZoneID, cTournamentSign))
				delete cTournamentSign;
		}

		pFirstClan->m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(itr, pFirstClan->m_arKnightsUser)
		{
			_KNIGHTS_USER* pFirstClanKnightUser = itr->second;
			if (pFirstClanKnightUser == nullptr)
				continue;

			CUser* pFirstClanUser = g_pMain->GetUserPtr(pFirstClanKnightUser->strUserName, NameType::TYPE_CHARACTER);
			if (pFirstClanUser == nullptr
				|| !pFirstClanUser->isInGame()
				|| pFirstClanUser->GetZoneID() == TournamentStartZoneID)
				continue;

			pFirstClanUser->ZoneChange(TournamentStartZoneID, 0.0f, 0.0f);
		}
		pFirstClan->m_arKnightsUser.m_lock.unlock();

		pSecondClan->m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(itr, pSecondClan->m_arKnightsUser)
		{
			_KNIGHTS_USER* pSecondClanKnightUser = itr->second;
			if (pSecondClanKnightUser == nullptr)
				continue;

			CUser* pSecondClanUser = g_pMain->GetUserPtr(pSecondClanKnightUser->strUserName, NameType::TYPE_CHARACTER);
			if (pSecondClanUser == nullptr
				|| !pSecondClanUser->isInGame()
				|| pSecondClanUser->GetZoneID() == TournamentStartZoneID)
				continue;

			pSecondClanUser->ZoneChange(TournamentStartZoneID, 0.0f, 0.0f);
		}
		pSecondClan->m_arKnightsUser.m_lock.unlock();
	}
	else if (isPartyVs)
	{
		CKnights* pFirstClanParty = nullptr, * pSecondClanParty = nullptr;
		g_pMain->m_KnightsArray.m_lock.lock();
		foreach_stlmap(itr, g_pMain->m_KnightsArray)
		{
			if (itr->second == nullptr)
				continue;

			if (!itr->second->GetName().compare(TournamentName1))
				pFirstClanParty = itr->second;

			if (!itr->second->GetName().compare(TournamentName2))
				pSecondClanParty = itr->second;
		}
		g_pMain->m_KnightsArray.m_lock.unlock();

		if (pFirstClanParty == nullptr)
		{
			// Send Game Server Description
			printf("Error :Party Tournament: First clan was not found in database \n");
			return true;
		}

		if (pSecondClanParty == nullptr)
		{
			// Send Game Server Description
			printf("Error :Party Tournament: Second clan was not found in database \n");
			return true;
		}

		uint16 FirstClanOnlineCount = 0;
		pFirstClanParty->m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(itr, pFirstClanParty->m_arKnightsUser)
		{
			_KNIGHTS_USER* pFirstClanPartyKnightUser = itr->second;
			if (pFirstClanPartyKnightUser == nullptr)
				continue;

			CUser* pFirstClanUser = g_pMain->GetUserPtr(pFirstClanPartyKnightUser->strUserName, NameType::TYPE_CHARACTER);
			if (pFirstClanUser != nullptr
				&& pFirstClanUser->isInGame()
				&& pFirstClanUser->GetZoneID() != TournamentStartZoneID)
				FirstClanOnlineCount++;
		}
		pFirstClanParty->m_arKnightsUser.m_lock.unlock();

		if (FirstClanOnlineCount > TOURNAMENT_MAX_PARTY_COUNT)
		{
			// Send Game Server Description
			printf("Error : Party Tournament: %s Clan is clan high count. (count: %d) Maximum Count (%d) \n", TournamentName1.c_str(), FirstClanOnlineCount, TOURNAMENT_MAX_PARTY_COUNT);
			return true;
		}

		if (FirstClanOnlineCount < TOURNAMENT_MIN_PARTY_COUNT)
		{
			// Send Game Server Description
			printf("Error : Party Tournament: %s Clan is clan low count. (count: %d) Minumum Count (%d)\n", TournamentName1.c_str(), FirstClanOnlineCount, TOURNAMENT_MIN_PARTY_COUNT);
			return true;
		}

		uint16 SecondClanOnlineCount = 0;
		pSecondClanParty->m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(itr, pSecondClanParty->m_arKnightsUser)
		{
			_KNIGHTS_USER* pSecondClanPartyKnightUser = itr->second;
			if (pSecondClanPartyKnightUser == nullptr)
				continue;

			CUser* pSecondClanUser = g_pMain->GetUserPtr(pSecondClanPartyKnightUser->strUserName, NameType::TYPE_CHARACTER);
			if (pSecondClanUser != nullptr
				&& pSecondClanUser->isInGame()
				&& pSecondClanUser->GetZoneID() != TournamentStartZoneID)
				SecondClanOnlineCount++;
		}
		pSecondClanParty->m_arKnightsUser.m_lock.unlock();

		if (SecondClanOnlineCount > TOURNAMENT_MAX_PARTY_COUNT)
		{
			// Send Game Server Description
			printf("Error : Party Tournament: %s Clan is clan high count. (count: %d) Maximum Count (%d) \n", TournamentName2.c_str(), SecondClanOnlineCount, TOURNAMENT_MAX_PARTY_COUNT);
			return true;
		}

		if (SecondClanOnlineCount < TOURNAMENT_MIN_PARTY_COUNT)
		{
			// Send Game Server Description
			printf("Error : Party Tournament: %s Clan is clan low count. (count: %d) Minumum Count (%d)\n", TournamentName2.c_str(), SecondClanOnlineCount, TOURNAMENT_MIN_PARTY_COUNT);
			return true;
		}

		KickOutZoneUsers(TournamentStartZoneID);

		if (TournamentClanInfo == nullptr)
		{
			_TOURNAMENT_DATA* cTournamentSign = new _TOURNAMENT_DATA;
			cTournamentSign->aTournamentZoneID = TournamentStartZoneID;
			cTournamentSign->aTournamentClanNum[0] = pFirstClanParty->GetID();
			cTournamentSign->aTournamentClanNum[1] = pSecondClanParty->GetID();
			cTournamentSign->aTournamentScoreBoard[0] = 0;
			cTournamentSign->aTournamentScoreBoard[1] = 0;
			cTournamentSign->aTournamentTimer = 0;
			cTournamentSign->aTournamentMonumentKilled = 0;
			cTournamentSign->aTournamentOutTimer = UNIXTIME;
			cTournamentSign->aTournamentisAttackable = false;
			cTournamentSign->aTournamentisStarted = true;
			cTournamentSign->aTournamentisFinished = false;

			if (!m_ClanVsDataList.PutData(cTournamentSign->aTournamentZoneID, cTournamentSign))
				delete cTournamentSign;
		}

		pFirstClanParty->m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(itr, pFirstClanParty->m_arKnightsUser)
		{
			_KNIGHTS_USER* pFirstClanPartyKnightUser = itr->second;
			if (pFirstClanPartyKnightUser == nullptr)
				continue;

			CUser* pFirstClanUser = g_pMain->GetUserPtr(pFirstClanPartyKnightUser->strUserName, NameType::TYPE_CHARACTER);
			if (pFirstClanUser == nullptr
				|| !pFirstClanUser->isInGame()
				|| pFirstClanUser->GetZoneID() == TournamentStartZoneID)
				continue;

			pFirstClanUser->ZoneChange(TournamentStartZoneID, 0.0f, 0.0f);
		}
		pFirstClanParty->m_arKnightsUser.m_lock.unlock();

		pSecondClanParty->m_arKnightsUser.m_lock.lock();
		foreach_stlmap_nolock(itr, pSecondClanParty->m_arKnightsUser)
		{
			_KNIGHTS_USER* pSecondClanPartyKnightUser = itr->second;
			if (pSecondClanPartyKnightUser == nullptr)
				continue;

			CUser* pSecondClanUser = g_pMain->GetUserPtr(pSecondClanPartyKnightUser->strUserName, NameType::TYPE_CHARACTER);
			if (pSecondClanUser == nullptr
				|| !pSecondClanUser->isInGame()
				|| pSecondClanUser->GetZoneID() == TournamentStartZoneID)
				continue;

			pSecondClanUser->ZoneChange(TournamentStartZoneID, 0.0f, 0.0f);
		}
		pSecondClanParty->m_arKnightsUser.m_lock.unlock();

	}
	printf("Final : Tournament is Start: Red Clan: (%s) - (%s) :Blue Clan Tournament Zone (%d) \n", TournamentName1.c_str(), TournamentName1.c_str(), TournamentStartZoneID);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleEventUnderTheCastleCommand)
{
	// Nation number
	if (vargs.size() < 1)
	{
		// send description
		printf("Using Sample : +underthecastle 1/2 (Open/Close)\n");
		return true;
	}
	string chatstr;
	uint8 type;
	type = atoi(vargs.front().c_str());

	if (type == 1)
	{
		m_bUnderTheCastleIsActive = true;
		m_bUnderTheCastleMonster = true;
		m_nUnderTheCastleEventTime = 180 * MINUTE;
		GetServerResource(IDS_UNDER_THE_CASTLE_OPEN, &chatstr);
		g_pMain->SendAnnouncement(chatstr.c_str());
		//g_pMain->SendAnnouncement("Under the Castle has opened. You may now enter Under the Castle"); // UTC Acilis Notice D?zeltildi.
		printf("Under The Castle Stard\n");
	}

	if (type == 2)
	{
		m_nUnderTheCastleEventTime = 10;
		GetServerResource(IDS_UNDER_THE_CASTLE_VICTORY, &chatstr);
		g_pMain->SendAnnouncement(chatstr.c_str());
		printf("Under The Castle Closed\n");
	}
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleTeleportAllCommand)
{
	// Zone number
	if (vargs.size() < 1)
	{
		// send description
		printf("Using Sample : /tp_all ZoneNumber | /tp_all ZoneNumber TargetZoneNumber.\n");
		return true;
	}

	int nZoneID = 0;
	int nTargetZoneID = 0;

	if (vargs.size() == 1)
		nZoneID = atoi(vargs.front().c_str());

	if (vargs.size() == 2)
	{
		nZoneID = atoi(vargs.front().c_str());
		vargs.pop_front();
		nTargetZoneID = atoi(vargs.front().c_str());
	}

	if (nZoneID > 0 || nTargetZoneID > 0)
		g_pMain->KickOutZoneUsers(nZoneID,nTargetZoneID);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleShutdownCommand)
{
	if (m_Shutdownstart) { printf("Server Shut Down in process..!!\n"); return true; }
	m_Shutdownfinishtime = UNIXTIME + 1;
	m_Shutdownstart = true; m_ShutdownKickStart = false;

	printf("Server Shut Down in 1 minutes \n");
	Packet result(WIZ_LOGOSSHOUT, uint8(2));
	result << uint8(6) << uint8(1);
	Send_All(&result);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleDiscountCommand)
{
	m_sDiscount = 1;
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleGlobalDiscountCommand)
{
	m_sDiscount = 2;
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleDiscountOffCommand)
{
	m_sDiscount = 0;
	return true;
}

COMMAND_HANDLER(CUser::HandleCaptainCommand) { return !isGM() ? false : g_pMain->HandleCaptainCommand(vargs, args, description); }
COMMAND_HANDLER(CGameServerDlg::HandleCaptainCommand)
{
	BattleZoneSelectCommanders();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleSantaCommand)
{
	m_bSantaOrAngel = FLYING_SANTA;
	m_bCzSantaEventActive = true;
	m_CzSantaEventEndTime = (uint32)UNIXTIME + (6 * HOUR);
	m_CzSantaNextSpawnTime = (uint32)UNIXTIME;
	SendAnnouncement("CZ Santa event manually started by server command.", Nation::ALL);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleSantaOffCommand)
{
	m_bSantaOrAngel = FLYING_NONE;
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleAngelCommand)
{
	m_bSantaOrAngel = FLYING_ANGEL;
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandlePermanentChatCommand)
{
	if (vargs.empty())
	{
		// send error saying we need args (unlike the previous implementation of this command)
		return true;
	}

	SetPermanentMessage("%s", args);
	return true;
}

void CGameServerDlg::SendHelpDescription(CUser *pUser, std::string sHelpMessage)
{
	if (pUser == nullptr || sHelpMessage == "")
		return;

	Packet result(WIZ_CHAT, (uint8)ChatType::PUBLIC_CHAT);
	result << pUser->GetNation() << pUser->GetSocketID() << uint8(0) << sHelpMessage;
	pUser->Send(&result);
}

void CGameServerDlg::SendInfoMessage(CUser *pUser, std::string sHelpMessage, uint8 type)
{
	if (pUser == nullptr || sHelpMessage == "")
		return;

	Packet result(WIZ_CHAT, type);
	result << pUser->GetNation() << pUser->GetSocketID() << uint8(0) << sHelpMessage;
	pUser->Send(&result);
}

void CGameServerDlg::SetPermanentMessage(const char * format, ...)
{
	char buffer[128];
	va_list ap;
	va_start(ap, format);
	vsnprintf(buffer, 128, format, ap);
	va_end(ap);

	m_bPermanentChatMode = true;
	m_strPermanentChat = buffer;

	Packet result;
	ChatPacket::Construct(&result, (uint8)ChatType::PERMANENT_CHAT, &m_strPermanentChat);
	Send_All(&result);
}

COMMAND_HANDLER(CGameServerDlg::HandlePermanentChatOffCommand)
{
	Packet result;
	ChatPacket::Construct(&result, (uint8)ChatType::END_PERMANENT_CHAT);
	m_bPermanentChatMode = false;
	Send_All(&result);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadBonusNotice)
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr
			|| !pUser->isInGame()
			|| !pUser->isInClan())
			continue;

		CKnights* pKnights = g_pMain->GetClanPtr(pUser->GetClanID());
		if (pKnights == nullptr)
			continue;

		if (pKnights->isCastellanCape()) {
			auto* pKnightCape = g_pMain->m_KnightsCapeArray.GetData(pKnights->m_castCapeID);
			if (pKnightCape && pKnightCape->BonusType > 0) pUser->SendCapeBonusNotice();
		}
		else {
			auto* pKnightCape = g_pMain->m_KnightsCapeArray.GetData(pKnights->GetCapeID());
			if (pKnightCape && pKnightCape->BonusType > 0) pUser->SendCapeBonusNotice();
		}
	}

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadNoticeCommand)
{
	// Reload the notice data
	LoadNoticeData();
	LoadNoticeUpData();

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		pUser->SendNotice();
		pUser->TopSendNotice();
	}

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadDrakiTowerTables)
{
	m_DrakiMonsterListArray.DeleteAllData();
	m_DrakiRoomListArray.DeleteAllData();
	LoadDrakiTowerTables();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadDungeonDefenceTables)
{
	m_DungeonDefenceMonsterListArray.DeleteAllData();
	LoadDungeonDefenceMonsterTable();

	m_DungeonDefenceStageListArray.DeleteAllData();
	LoadDungeonDefenceStageTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadUpgradeCommand) {
	// JstKO 2369: active upgrade data comes from NEW_UPGRADE2369 and ITEM_UPGRADE_SETTINGS2369.
	// Do not reload legacy ITEM_UPGRADE here.
	g_DBAgent.LoadUpgrade();
	g_DBAgent.LoadItemUpgradeSettings();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadRankBugCommand) {
	LoadRankBugTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadLevelRewardCommand) {
	
	tar_levelreward = true;
	m_LevelRewardArray.DeleteAllData();
	LoadLevelRewardTable();
	tar_levelreward = false;
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadMerchantLevelRewardCommand) {
	tar_levelmercreward = true;
	m_LevelMerchantRewardArray.DeleteAllData();
	LoadLevelMerchantRewardTable();
	tar_levelmercreward = false;
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadZoneOnlineRewardCommand) {

	m_ZoneOnlineRewardReload = true;
	m_ZoneOnlineRewardArrayLock.lock();
	m_ZoneOnlineRewardArray.clear();
	LoadZoneOnlineRewardTable();
	std::vector<_ZONE_ONLINE_REWARD> copymap = g_pMain->m_ZoneOnlineRewardArray;
	m_ZoneOnlineRewardArrayLock.unlock();

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;
		
		pUser->m_ZoneOnlineRewardLock.lock();
		pUser->m_ZoneOnlineReward.clear();
		pUser->m_ZoneOnlineReward = copymap;
		pUser->m_ZoneOnlineRewardLock.unlock();
	}
	m_ZoneOnlineRewardReload = false;
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadTablesCommand)
{
	m_StartPositionArray.DeleteAllData();
	LoadStartPositionTable();

	m_StartPositionRandomArray.DeleteAllData();
	LoadStartPositionRandomTable();

	m_ItemExchangeArray.DeleteAllData();
	LoadItemExchangeTable();

	m_ItemExchangeExpArray.DeleteAllData();
	LoadItemExchangeExpTable();

	m_ItemSpecialExchangeArray.DeleteAllData();
	LoadItemSpecialExchangeTable();

	m_ItemExchangeCrashArray.DeleteAllData();
	LoadItemExchangeCrashTable();

	m_EventTriggerArray.DeleteAllData();
	LoadEventTriggerTable();

	m_ServerResourceArray.DeleteAllData();
	LoadServerResourceTable();

	m_MonsterResourceArray.DeleteAllData();
	LoadMonsterResourceTable();

	m_MonsterChallengeArray.DeleteAllData();
	LoadMonsterChallengeTable();

	m_MonsterChallengeSummonListArray.DeleteAllData();
	LoadMonsterChallengeSummonListTable();

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadTables2Command)
{
	m_RimaLotteryArray.DeleteAllData();
	LoadRimaLotteryEventTable();

	m_WarBanishOfWinnerArray.DeleteAllData();
	LoadBanishWinnerTable();

	m_DungeonDefenceMonsterListArray.DeleteAllData();
	LoadDungeonDefenceMonsterTable();

	m_DungeonDefenceStageListArray.DeleteAllData();
	LoadDungeonDefenceStageTable();

	m_DrakiMonsterListArray.DeleteAllData();
	m_DrakiRoomListArray.DeleteAllData();
	LoadDrakiTowerTables();

	m_LuaGiveItemExchangeArray.DeleteAllData();
	LoadGiveItemExchangeTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadAllTabCommand) {
	g_pMain->HandleReloadNoticeCommand(vargs, args, description);
	g_pMain->HandleReloadTablesCommand(vargs, args, description);
	g_pMain->HandleReloadTables2Command(vargs, args, description);
	g_pMain->HandleReloadTables3Command(vargs, args, description);
	g_pMain->HandleReloadMagicsCommand(vargs, args, description);
	g_pMain->HandleReloadQuestCommand(vargs, args, description);
	g_pMain->HandleReloadRanksCommand(vargs, args, description);
	g_pMain->HandleReloadDropsCommand(vargs, args, description);
	g_pMain->HandleReloadDropsRandomCommand(vargs, args, description);
	g_pMain->HandleReloadKingsCommand(vargs, args, description);
	g_pMain->HandleReloadRightTopTitleCommand(vargs, args, description);
	g_pMain->HandleReloadPusItemCommand(vargs, args, description);
	g_pMain->HandleReloadDungeonDefenceTables(vargs, args, description);
	g_pMain->HandleReloadDrakiTowerTables(vargs, args, description);
	g_pMain->HandleEventScheduleResetTable(vargs, args, description);
	g_pMain->HandleTopLeftCommand(vargs, args, description);
	g_pMain->HandleReloadBonusNotice(vargs, args, description);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadTables3Command)
{
	m_MonsterRespawnLoopArray.DeleteAllData();
	LoadMonsterRespawnLoopListTable();

	m_MonsterSummonList.DeleteAllData();
	LoadMonsterSummonListTable();

	m_MonsterUnderTheCastleArray.DeleteAllData();
	LoadMonsterUnderTheCastleTable();

	m_MonsterStoneListInformationArray.DeleteAllData();
	LoadMonsterStoneListInformationTable();

	m_JuraidMountionListInformationArray.DeleteAllData();
	LoadJuraidMountionListInformationTable();

	m_ChaosStoneSummonListArray.DeleteAllData();
	LoadChaosStoneMonsterListTable();

	m_ChaosStoneRespawnCoordinateArray.DeleteAllData();
	LoadChaosStoneCoordinateTable();

	m_ChaosStoneStageArray.DeleteAllData();
	LoadChaosStoneStage();

	m_MiningExchangeArray.DeleteAllData();
	LoadMiningExchangeListTable();

	m_MiningFishingItemArray.DeleteAllData();
	LoadMiningFishingItemTable();

	m_ItemOpArray.DeleteAllData();
	LoadItemOpTable();

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadPusItemCommand) 
{
	m_PusItemArray.DeleteAllData();
	LoadPusItemsTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadMagicsCommand)
{
	m_IsMagicTableInUpdateProcess = true;
	m_MagictableArray.DeleteAllData();
	m_Magictype1Array.DeleteAllData();
	m_Magictype2Array.DeleteAllData();
	m_Magictype3Array.DeleteAllData();
	m_Magictype4Array.DeleteAllData();
	m_Magictype5Array.DeleteAllData();
	m_Magictype6Array.DeleteAllData();
	m_Magictype8Array.DeleteAllData();
	m_Magictype9Array.DeleteAllData();
	LoadMagicTable();
	LoadMagicType1();
	LoadMagicType2();
	LoadMagicType3();
	LoadMagicType4();
	LoadMagicType5();
	LoadMagicType6();
	LoadMagicType7();
	LoadMagicType8();
	LoadMagicType9();
	m_IsMagicTableInUpdateProcess = false;

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleEventScheduleResetTable)
{
	if (pTempleEvent.ActiveEvent != -1 || m_byBattleOpen != NO_BATTLE || pBeefEvent.isActive)
	{
		printf("ongoing event Warning\n");
		return true;
	}

	if (pTempleEvent.ActiveEvent != -1 || m_byBattleOpen != NO_BATTLE
		|| pBeefEvent.isActive || pForgettenTemple.isActive) {
		printf("ongoing event Warning\n");
		return true;
	}

	g_pMain->pEventTimeOpt.Initialize();
	XCodeLoadEventTables();
	XCodeLoadEventVroomTables();

	m_BeefEventPlayTimerArray.DeleteAllData();
	LoadBeefEventPlayTimerTable();

	m_EventTimerShowArray.DeleteAllData();
	LoadEventTimerShowTable();
	EventTimerSet();
	
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr)
			continue;
		if (!pUser->isInGame())
			continue;

		pUser->SendEventTimerList();
	}
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadQuestCommand)
{
	m_QuestHelperArray.DeleteAllData();
	LoadQuestHelperTable();
	m_QuestMonsterArray.DeleteAllData();
	LoadQuestMonsterTable();

	m_DailyQuestArray.DeleteAllData();
	LoadDailyQuestListTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadDailyQuestCommand)
{
	m_DailyQuestArray.DeleteAllData();
	LoadDailyQuestListTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadRanksCommand)
{
	ReloadKnightAndUserRanks(true);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadDropsCommand)
{
	m_MakeItemGroupArray.DeleteAllData();
	LoadMakeItemGroupTable();
	m_NpcItemArray.DeleteAllData();
	LoadNpcItemTable();
	m_MonsterItemArray.DeleteAllData();
	LoadMonsterItemTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadDropsRandomCommand)
{
	m_randomtable_reload = true;
	m_MakeItemGroupRandomArray.DeleteAllData();
	LoadMakeItemGroupRandomTable();
	m_randomtable_reload = false;
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadKingsCommand)
{
	m_KingSystemArray.DeleteAllData();
	LoadKingSystem();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadRightTopTitleCommand) 
{
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame()) continue;
		pUser->RightTopTitleMsgDelete();
	}

	m_RightTopTitleArray.DeleteAllData();
	LoadRightTopTitleTable();

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		pUser->RightTopTitleMsg();
	}

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadItemsCommand)
{
	m_ItemtableArray.DeleteAllData();
	LoadItemTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadItems)
{
	ReLoadItemTable();
	printf("[ITEM] Table Reload\n");
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleCountCommand)
{
	uint16 count = 0;
	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		count++;
	}
	
	m_BotcharacterNameLock.lock();
	count += (uint16)g_pMain->m_BotcharacterNameMap.size();
	m_BotcharacterNameLock.unlock();

	printf("Online User Count : %d\n", count);
	return true;
 }

void CGameServerDlg::SendFormattedResource(uint32 nResourceID, uint8 byNation, bool bIsNotice, ...)
{
	_SERVER_RESOURCE *pResource = m_ServerResourceArray.GetData(nResourceID);
	if (pResource == nullptr)
		return;

	string buffer;
	va_list ap;
	va_start(ap, bIsNotice);
	_string_format(pResource->strResource, &buffer, ap);
	va_end(ap);

	if (bIsNotice)
		SendNotice(buffer.c_str(), byNation);
	else
		SendAnnouncement(buffer.c_str(), byNation);
}


COMMAND_HANDLER(CUser::HandleEffectTestCommand)
{
	if (vargs.empty())
	{
		g_pMain->SendHelpDescription(this, "Kullanim: +effect EffectID [show|player|both] [CharacterName]");
		g_pMain->SendHelpDescription(this, "Ornek: +effect 31088 both | +effect 490092 show | +effect 15005 player NxWiLe");
		return true;
	}

	uint32 nEffectID = uint32(atoi(vargs.front().c_str()));
	if (nEffectID == 0)
	{
		g_pMain->SendHelpDescription(this, "EffectID gecersiz. Ornek: +effect 31088 both");
		return true;
	}

	vargs.pop_front();

	std::string mode = "both";
	if (!vargs.empty())
	{
		mode = vargs.front();
		vargs.pop_front();
	}

	CUser* pTarget = this;
	if (!vargs.empty())
	{
		std::string strTarget = vargs.front();
		pTarget = g_pMain->GetUserPtr(strTarget, NameType::TYPE_CHARACTER);
		if (pTarget == nullptr)
		{
			g_pMain->SendHelpDescription(this, string_format("Effect target online degil: %s", strTarget.c_str()));
			return true;
		}
	}

	bool bShowEffect = (mode == "show" || mode == "both" || mode == "all");
	bool bPlayerEffect = (mode == "player" || mode == "both" || mode == "all");

	if (!bShowEffect && !bPlayerEffect)
	{
		g_pMain->SendHelpDescription(this, "Mod gecersiz. Kullan: show, player veya both");
		return true;
	}

	if (bShowEffect)
		pTarget->ShowEffect(nEffectID);

	if (bPlayerEffect)
	{
		if (nEffectID > 65535)
		{
			g_pMain->SendHelpDescription(this, string_format("PlayerEffect uint16 destekler. %u icin player atlandi, show gonderildi.", nEffectID));
		}
		else
		{
			pTarget->PlayerEffect(uint16(nEffectID));
		}
	}

	g_pMain->SendHelpDescription(this, string_format("Effect test gonderildi. Target=%s EffectID=%u Mode=%s", pTarget->GetName().c_str(), nEffectID, mode.c_str()));
	return true;
}

COMMAND_HANDLER(CUser::HandleServerGameTestCommand)
{
	if (!isGM()) return false;
	
	SendItemMove(1, 1);

	return true;
}

#include "MagicInstance.h"

COMMAND_HANDLER(CGameServerDlg::HandleServerGameTestCommand)
{


	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleChaosExpansionClose)
{
	ChaosExpansionManuelClosed();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleBeefEventClose)
{
	BeefEventManuelClosed();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleBorderDefenceWarClose)
{
	BorderDefenceWarManuelClosed();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleJuraidMountainClose)
{
	JuraidMountainManuelClosed();
	return true;
}

bool CUser::GetLevelChangeStat()
{
	Packet result(WIZ_CLASS_CHANGE, uint8(ALL_POINT_CHANGE));
	for (int i = 0; i < SLOT_MAX; i++) {
		_ITEM_DATA* pItem = GetItem(i);
		if (pItem && pItem->nNum > 0)return false;
	}

	uint8 basePoint[5];
	basePoint[0] = 50;
	basePoint[1] = 60;
	basePoint[2] = 60;
	basePoint[3] = 50;
	basePoint[4] = 50;

	if (isPriest())
		basePoint[4] += 20;
	else if (isWarrior())
	{
		basePoint[0] += 15;
		basePoint[1] += 5;
	}
	else if (isMage())
	{
		basePoint[2] += 10;
		basePoint[4] += 20;
		basePoint[1] -= 10;
	}
	else if (isRogue())
	{
		basePoint[0] += 10;
		basePoint[2] += 10;
	}
	else if (isPortuKurian())
	{
		basePoint[0] += 15;
		basePoint[1] += 5;
	}

	uint16 ReStatTotal = 0;
	for (int x = 0; x < (int)StatType::STAT_COUNT; x++) {
		if (basePoint[x] <= 0)return false;
		ReStatTotal += basePoint[x];
	}

	if (ReStatTotal != 290) return false;

	for (int x = 0; x < (int)StatType::STAT_COUNT; x++)
		SetStat((StatType)x, basePoint[x]);

	uint8 UserLevel = GetLevel();
	if (UserLevel > 83) UserLevel = 83;

	// Players gain 3 stats points for each level up to and including 60.
	// They also received 10 free stat points on creation. 
	m_sPoints = 10 + (UserLevel - 1) * 3;

	// For every level after 60, we add an additional two points.
	if (UserLevel > 60)
		m_sPoints += 2 * (UserLevel - 60);

	uint16 statTotal = GetStatTotal();
	if (statTotal != 290) return false;

	SetUserAbility();

	uint16 byStr, bySta, byDex, byInt, byCha;
	byStr = GetStat(StatType::STAT_STR),
		bySta = GetStat(StatType::STAT_STA),
		byDex = GetStat(StatType::STAT_DEX),
		byInt = GetStat(StatType::STAT_INT),
		byCha = GetStat(StatType::STAT_CHA);

	result << uint8(1)
		<< GetCoins()
		<< byStr << bySta << byDex << byInt << byCha
		<< m_MaxHp << m_MaxMp << m_sTotalHit << m_sMaxWeight << m_sPoints;
	Send(&result);
	return true;
}

//bool CUser::GetLevelChangeSkill()
//{
//	Packet result(WIZ_CLASS_CHANGE, uint8(ALL_SKILLPT_CHANGE));
//	int index = 0, skill_point = 0, money = 0, temp_value = 0, old_money = 0;
//	uint8 type = 0;
//
//	if (GetLevel() < 10)
//		return false;
//
//	// Get total skill points
//	for (int i = 1; i < 9; i++)
//		skill_point += m_bstrSkill[i];
//
//	// Reset skill points.
//	m_bstrSkill[0] = (GetLevel() - 9) * 2;
//	for (int i = 1; i < 9; i++)
//		m_bstrSkill[i] = 0;
//
//	result << uint8(1) << GetCoins() << m_bstrSkill[0];
//	Send(&result);
//	return true;
//}

bool CUser::GetLevelChangeSkill()
{
	Packet result(WIZ_CLASS_CHANGE, uint8(ALL_SKILLPT_CHANGE));
	int index = 0, skill_point = 0, money = 0, temp_value = 0, old_money = 0;
	uint8 type = 0;

	if (GetLevel() < 10)
		return false;

	// Get total skill points
	for (int i = 1; i < 9; i++)
		skill_point += m_bstrSkill[i];

	// Reset skill points.
	m_bstrSkill[0] = (GetLevel() - 9) * 2;
	for (int i = 1; i < 9; i++)
		m_bstrSkill[i] = 0;

	result << uint8(1) << GetCoins() << m_bstrSkill[0];
	Send(&result);
	return true;
}

#pragma region CUser::HandleCountCommand
COMMAND_HANDLER(CUser::HandleCountCommand)
{
	int user = 0, bot = 0, merchantuser = 0,merchantbot=0;

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (pUser->isMerchanting())
			merchantuser++;

		user++;
	}
	
	g_pMain->m_MapBotList.m_lock.lock();
	auto m_sMapBotListArray = g_pMain->m_MapBotList.m_UserTypeMap;
	g_pMain->m_MapBotList.m_lock.unlock();

	foreach(itr, m_sMapBotListArray)
	{
		CBot *pBot = itr->second;
		if (pBot == nullptr)
			continue;
		
		if (!pBot->isInGame())
			continue;

		if (pBot->isMerchanting())
			merchantbot++;

		bot++;
	}

	g_pMain->SendHelpDescription(this, string_format("Online User : %d/%d Merchant User : %d - Bot User :%d   Bot Merchant : %d", user,int(MAX_USER - user), merchantuser, bot, merchantbot));
	return true;
}

#pragma region CUser::HandleLevelChange
COMMAND_HANDLER(CUser::HandleLevelChange)
{
	if (vargs.size() < 2) {
		g_pMain->SendHelpDescription(this, "Using Sample : +level CharacterName Level");
		return true;
	}

	std::string strUserID = vargs.front();
	vargs.pop_front();
	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE) {
		g_pMain->SendHelpDescription(this, "Using Sample : +level CharacterName Level");
		return true;
	}

	CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser == nullptr || !pUser->isInGame()) {
		g_pMain->SendHelpDescription(this, "Error : User is not online");
		return true;
	}

	uint8 Level = atoi(vargs.front().c_str());
	vargs.pop_front();
	if (Level < 10 || Level > 83) {
		g_pMain->SendHelpDescription(this, "Error : Minumum 10 - Maximum 83");
		return true;
	}

	for (int i = 0; i < SLOT_MAX; i++) {
		_ITEM_DATA* pItem = pUser->GetItem(i);
		if (pItem && pItem->nNum > 0) {
			g_pMain->SendHelpDescription(this, "Error :Please take off all your clothes");
			return true;
		}
	}

	pUser->LevelChange(Level, false);
	pUser->AllSkillPointChange(true);
	pUser->AllPointChange(true);
	g_pMain->SendHelpDescription(this, "Level Change Process Success!");
	return true;
}
#pragma endregion

COMMAND_HANDLER(CUser::HandlePartyTP) 
{

	if (vargs.empty())
	{
		g_pMain->SendHelpDescription(this, "+partytp bosluk nick");
		return true;
	}

	std::string strUserID = vargs.front();
	vargs.pop_front();
	CUser *pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);

	if (pUser == nullptr)
	{
		g_pMain->SendHelpDescription(this, "Error : User is not online");
		return true;
	}
	pUser->ZoneChangeParty(GetZoneID(), uint16(GetX()), uint16(GetZ()));
	return true;
}


COMMAND_HANDLER(CUser::HandleNpcBilgi) 
{
	{
	CNpc* pNpc = g_pMain->GetNpcPtr(GetTargetID(), GetZoneID());
	if (pNpc == nullptr)
		return false;

	std::string strNpcName = pNpc->GetName();
	if (strNpcName.length() == 0)
		strNpcName = "<NoName>";
	g_pMain->SendHelpDescription(this, string_format("[Npc Name]  = %s | [Npc ID]  = %d | [Npc Proto ID] = %d",
		strNpcName.c_str(), pNpc->GetID(), pNpc->GetProtoID()));

	}

	return true;
}

COMMAND_HANDLER(CUser::HandleProcInfo)
{
	if (!isGM())
		return false;
	
	if (vargs.empty())
	{
		g_pMain->SendHelpDescription(this, "+info bosluk nick");
		return true;
	}

	std::string strUserID = vargs.front();
	
	CUser * pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser != nullptr && pUser->isInGame()) 
	{
		pUser->XSafe_SendProcessInfoRequest(this);
		//Packet result(XSafe);
		//result << uint8(PROCINFO) << uint16(pUser->GetID());
		//pUser->Send(&result);
	}
	else 
		g_pMain->SendHelpDescription(this, "Boyle bir user bulunamadi.");

	return true;
}

// gm tbl kaydetme komutu  AA11BB22CC99
COMMAND_HANDLER(CUser::HandleTBL)
{
	if (!isGM())
	{
		return false;
	}

	/*ini.GetString("TBL_HASH", "ITEM_ORG", "", server_itemorg);
	ini.GetString("TBL_HASH", "MAGIC_MAIN", "", server_skillmagic);
	ini.GetString("TBL_HASH", "ZONES", "", server_zones);
	ini.GetString("TBL_HASH", "MAGIC_MAIN_TK", "", server_skillmagictk);*/
	printf("tbl verileri kaydedildi.\n");
	CIni ini(CONF_GAME_SERVER);
	ini.SetString("TBL_HASH", "ITEM_ORG", itemorg.c_str());
	ini.SetString("TBL_HASH", "MAGIC_MAIN", skillmagic.c_str());
	ini.SetString("TBL_HASH", "ZONES", zones.c_str());
	ini.SetString("TBL_HASH", "MAGIC_MAIN_TK", skillmagictk.c_str());

	g_pMain->server_itemorg = itemorg;
	g_pMain->server_skillmagic = skillmagic;
	g_pMain->server_zones = zones;
	g_pMain->server_skillmagictk = skillmagictk;

	return true;
}

COMMAND_HANDLER(CUser::HandleChangeGM) 
{
	if (vargs.empty())
	{
		g_pMain->SendHelpDescription(this, "+changegm bosluk nick");
		return true;
	}

	std::string strUserID = vargs.front();
	vargs.pop_front();

	CUser *pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);

	if (pUser == nullptr)
	{
		g_pMain->SendHelpDescription(this, "Error : User is not online");
		return true;
	}

	if (pUser->isInGame())
	{
		pUser->m_bAuthority = (uint8)AuthorityTypes::AUTHORITY_GAME_MASTER;
		pUser->SendMyInfo();
		pUser->UserInOut(INOUT_OUT);
		pUser->SetRegion(pUser->GetNewRegionX(), pUser->GetNewRegionZ());
		pUser->UserInOut(INOUT_WARP);
		g_pMain->UserInOutForMe(pUser);
		NpcInOutForMe();
		g_pMain->MerchantUserInOutForMe(pUser);
		pUser->ResetWindows();
		pUser->InitType4();
		pUser->RecastSavedMagic();
		g_pMain->SendHelpDescription(pUser, "Congratulations, you've become a gamemaster.");
	}




	return true;
}

COMMAND_HANDLER(CUser::HandleGenieStartStop) 

{
	if (vargs.empty())
	{
		// send description
		g_pMain->SendHelpDescription(this, "Select user once and then; '+genie 1' genie starts or '+genie 2' genie stops.");
		return true;
	}

	uint8 type;
	type = atoi(vargs.front().c_str());

	CUser * pUser = g_pMain->GetUserPtr(GetTargetID());

	if (pUser == nullptr)
	{
		g_pMain->SendHelpDescription(this, "Error : User is not online");
		return true;
	}

	if (pUser->isInGame())
	{
		if (type == 1)
		{
			pUser->GenieStart();
		}

		if (type == 2)
		{
			pUser->GenieStop();
		}
	}

	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleAIResetCommand)
{
	npcthreadreload = true;

	// Panel reload oncesi oyunda aktif duran eski NPC/monster instance'larini
	// clientlardan ve region listelerinden cikarmak zorundayiz. Aksi halde DB'den
	// silinen monster yeniden yuklenmese bile memory/harita uzerinde gorunmeye devam eder.
	{
		foreach_stlmap(itrDespawnThread, m_arNpcThread)
		{
			auto* pthread = itrDespawnThread->second;
			if (pthread == nullptr)
				continue;

			std::vector<CNpc*> removeList;
			pthread->m_arNpcArray.m_lock.lock();
			foreach_stlmap_nolock(itrDespawnNpc, pthread->m_arNpcArray)
			{
				CNpc* pNpc = itrDespawnNpc->second;
				if (pNpc == nullptr)
					continue;
				removeList.push_back(pNpc);
			}
			pthread->m_arNpcArray.m_lock.unlock();

			foreach(itrRemoveNpc, removeList)
			{
				CNpc* pNpc = *itrRemoveNpc;
				if (pNpc == nullptr)
					continue;

				pNpc->SendInOut(InOutType::INOUT_OUT, pNpc->GetX(), pNpc->GetZ(), pNpc->GetY());
			}
		}
	}

	m_arNpcTable.m_lock.lock();
	m_arNpcTable.DeleteAllData(false);
	LoadNpcTableData(true);
	m_arNpcTable.m_lock.unlock();

	m_arMonTable.m_lock.lock();
	m_arMonTable.DeleteAllData(false);
	LoadNpcTableData(false);
	m_arMonTable.m_lock.unlock();

	m_TotalNPC = 0;
	m_CurrentNPC = 0;

	{
		foreach_stlmap(itrReloadThread, m_arNpcThread)
		{
			auto* pthread = itrReloadThread->second;
			if (pthread == nullptr)
				continue;

			pthread->m_npclist.clear();
			pthread->m_arNpcArray.DeleteAllData(false);
			pthread->m_FreeNpcList.clear();
			for (uint16 i = NPC_BAND; i < 32567; i++)
				pthread->m_FreeNpcList.push_back(i);

			pthread->_LoadAllObjects();
		}
	}

	if (!CGameServerDlg::LoadNpcPosTable())
	{
		npcthreadreload = false;
		return true;
	}

	// Yeni yuklenen NPC/monsterlari bolgedeki oyunculara tekrar goster.
	{
		foreach_stlmap(itrShowThread, m_arNpcThread)
		{
			auto* pthread = itrShowThread->second;
			if (pthread == nullptr)
				continue;

			std::vector<CNpc*> inList;
			pthread->m_arNpcArray.m_lock.lock();
			foreach_stlmap_nolock(itrShowNpc, pthread->m_arNpcArray)
			{
				CNpc* pNpc = itrShowNpc->second;
				if (pNpc == nullptr)
					continue;
				inList.push_back(pNpc);
			}
			pthread->m_arNpcArray.m_lock.unlock();

			foreach(itrInNpc, inList)
			{
				CNpc* pNpc = *itrInNpc;
				if (pNpc == nullptr)
					continue;
				pNpc->SendInOut(InOutType::INOUT_IN, pNpc->GetX(), pNpc->GetZ(), pNpc->GetY());
			}
		}
	}

	ChaosStoneRespawnOkey = true;
	RandomBossSystemLoad();
	ChaosStoneLoad();
	npcthreadreload = false;
	return true;
}

#pragma region CUser::Handlebannedcommand
COMMAND_HANDLER(CUser::Handlebannedcommand)
{
	if (!isGM()) return false;

	if (vargs.size() < 1) {
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +block 'CharacterName' 'day'");
		return true;
	}

	std::string strUserID = vargs.front();
	vargs.pop_front();

	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE) {
		g_pMain->SendHelpDescription(this, "Error name!");
		return true;
	}

	uint32 period = 0;
	if (!vargs.empty()) { period = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (period && period > 1095) { g_pMain->SendHelpDescription(this, "day error!"); return true; }

	int vargsize = (int)vargs.size();
	std::string desc[250] = { "" }, finaldesc = "";
	for (int i = 0; i <= vargsize; i++) {
		if (vargs.empty()) continue;
		desc[i] = vargs.front();
		finaldesc += desc[i] + ' ';
		vargs.pop_front();
		if (desc[i].size() > 500) return true;
	}

	if (finaldesc.empty())finaldesc = "-";
	g_pMain->UserAuthorityUpdate(BanTypes::BANNED, this, strUserID, finaldesc, period);
	return true;
}
#pragma endregion

#pragma region CUser::HandlePcBlock
COMMAND_HANDLER(CUser::HandlePcBlock)
{
	if (!isGM()) return false;

	if (vargs.size() < 1) {
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +pcblock CharacterName day");
		return true;
	}

	std::string strUserID = vargs.front();
	vargs.pop_front();

	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE) {
		g_pMain->SendHelpDescription(this, "Error name!");
		return true;
	}

	CUser* pUser = g_pMain->GetUserPtr(strUserID, NameType::TYPE_CHARACTER);
	if (pUser == nullptr)
	{
		g_pMain->SendHelpDescription(this, "Error : User is not online");
		return true;
	}

	uint32 period = 999;
	std::string desc[250] = { "" }, finaldesc = "";
	if (finaldesc.empty())finaldesc = "-";

	g_pMain->UserAuthorityUpdate(BanTypes::BANNED, this, strUserID, finaldesc, period);
	Packet result(XSafe, uint8(XSafeOpCodes::LIFESKILL));
	result << uint8(1);
	pUser->Send(&result);
	

	return true;
}
#pragma endregion
#pragma region CUser::HandleunbannedCommand
COMMAND_HANDLER(CUser::HandleunbannedCommand)
{
	if (!isGM()) return false;

	if (vargs.size() < 1) {
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +unblock CharacterName");
		return true;
	}

	std::string strUserID = vargs.front();
	vargs.pop_front();

	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE) {
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +unblock CharacterName");
		return true;
	}

	g_pMain->UserAuthorityUpdate(BanTypes::UNBAN, this, strUserID);
	return true;
}
#pragma endregion

COMMAND_HANDLER(CGameServerDlg::Handlebannedcommand)
{
	if (vargs.size() < 1) { printf("Using Sample : +block CharacterName\n"); return true; }
	std::string strUserID = vargs.front();
	vargs.pop_front();
	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE) { printf("character name error!\n"); return true; }

	UserAuthorityUpdate(BanTypes::BANNED, nullptr, strUserID);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::Handleunbannedcommand)
{
	if (vargs.size() < 1) { printf("Using Sample : +unblock CharacterName\n"); return true; }
	std::string strUserID = vargs.front();
	vargs.pop_front();
	if (strUserID.empty() || strUserID.size() > MAX_ID_SIZE) { printf("character name error!\n"); return true; }
	UserAuthorityUpdate(BanTypes::UNBAN, nullptr, strUserID);
	return true;
}


COMMAND_HANDLER(CGameServerDlg::HandleCindirellaWarClose) { return CindirellaCommand(false, -1); }

COMMAND_HANDLER(CUser::HandleCindirellaWarClose)
{
	if (!isGM()) return false;
	return g_pMain->CindirellaCommand(false, -1, this);
}

COMMAND_HANDLER(CGameServerDlg::HandleReloadCindirellaCommand) {
	//ReqSendReloadTable(e_reloadpingtype::cindtables);

	if (pCindWar.isStarted() && pCindWar.isPrepara()) 
		return true;

	for (int i = 0; i < 5; i++)
		m_CindirellaItemsArray[i].DeleteAllData();

	m_CindirellaStatArray.DeleteAllData();

	memset(&pCindWar.m_warrior, 0, sizeof(pCindWar.m_warrior));
	memset(&pCindWar.m_rogue, 0, sizeof(pCindWar.m_rogue));
	memset(&pCindWar.m_mage, 0, sizeof(pCindWar.m_mage));
	memset(&pCindWar.m_priest, 0, sizeof(pCindWar.m_priest));
	memset(&pCindWar.m_kurian, 0, sizeof(pCindWar.m_kurian));

	LoadCindirellaItemsTable();
	LoadCindirellaStatSetTable();
	LoadCindirellaSettingTable();
	LoadCindirellaRewardsTable();
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleForgettenTempleEvent)
{
	if (vargs.size() < 1) { printf("Using Sample : /ftopen Type\n"); return true; }

	uint8 Type = 0;
	if (!vargs.empty()) { Type = atoi(vargs.front().c_str()); vargs.pop_front(); }
	ForgettenTempleManuelOpening(Type);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleForgettenTempleEventClose) {
	ForgettenTempleManuelClosed();
	return true;
}
COMMAND_HANDLER(CGameServerDlg::HandleReloadCOEFFICIENTCommand)
{
	g_pMain->m_bisDamage = true;
	m_CoefficientArray.DeleteAllData();
	LoadCoefficientTable();
	//23.06.2023 fr damage firedamage d?zenlemesi
	g_pMain->m_CoefficientDamageArray.DeleteAllData();
	g_pMain->LoadCoefficientDamageTable();
	//23.06.2023 fr damage firedamage d?zenlemesi end
	if (g_pMain->m_CoefficientArray.GetSize() > 0 && g_pMain->m_CoefficientDamageArray.GetSize() > 0)
	{
		printf("HandleReloadCOEFFICIENTCommand is succesfuly. \n");
		g_pMain->m_bisDamage = false;
	}
	return true;
}

