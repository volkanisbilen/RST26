#include "stdafx.h"
#include "GameEvent.h"

#include "KnightsManager.h"
#include "DBAgent.h"
#include "KingSystem.h"

#include "../shared/database/OdbcRecordset.h"
#include "../shared/database/ItemTableSet.h"
#include "../shared/database/ItemSellTableSet.h"
#include "../shared/database/SetItemTableSet.h"
#include "../shared/database/ItemExchangeSet.h"
#include "../shared/database/ItemExchangeExpSet.h"
#include "../shared/database/ItemSpecialExchangeSet.h"
#include "../shared/database/ItemExchangeCrashSet.h"
#include "../shared/database/ItemUpgradeSet.h"
#include "../shared/database/ItemOpSet.h"
#include "../shared/database/MagicTableSet.h"
#include "../shared/database/MagicType1Set.h"
#include "../shared/database/MagicType2Set.h"
#include "../shared/database/MagicType3Set.h"
#include "../shared/database/MagicType4Set.h"
#include "../shared/database/MagicType5Set.h"
#include "../shared/database/MagicType6Set.h"
#include "../shared/database/MagicType7Set.h"
#include "../shared/database/MagicType8Set.h"
#include "../shared/database/MagicType9Set.h"
#include "../shared/database/ObjectPosSet.h"
#include "../shared/database/ZoneInfoSet.h"
#include "../shared/database/EventSet.h"
#include "../shared/database/CoefficientSet.h"
#include "../shared/database/CoefficientDamageSet.h"
#include "../shared/database/LevelUpTableSet.h"
#include "../shared/database/ServerResourceSet.h"
#include "../shared/database/MonsterResourceSet.h"
#include "../shared/database/QuestHelperSet.h"
#include "../shared/database/QuestMonsterSet.h"
#include "../shared/database/KnightsAllianceSet.h"
#include "../shared/database/KnightsRankSet.h"
#include "../shared/database/KnightsCapeSet.h"
#include "../shared/database/UserPersonalRankSet.h"
#include "../shared/database/UserKnightsRankSet.h"
#include "../shared/database/CharacterSealSet.h"
#include "../shared/database/StartPositionSet.h"
#include "../shared/database/EventAllSet.h"
#include "../shared/database/StartPositionRandomSet.h"
#include "../shared/database/BattleSet.h"
#include "../shared/database/CapeCastellanBonusSet.h"
#include "../shared/database/BurningFeatureSet.h"
#include "../shared/database/thykedb_class.h"
#include "../shared/database/RentalItemSet.h"
#include "../shared/database/KingSystemSet.h"
#include "../shared/database/KingCandidacyNoticeBoardSet.h"
#include "../shared/database/KingElectionListSet.h"
#include "../shared/database/KingNominationListSet.h"
#include "../shared/database/SheriffReportListSet.h"
#include "../shared/database/EventTriggerSet.h"
#include "../shared/database/MonsterChallenge.h"
#include "../shared/database/MonsterChallengeSummonList.h"
#include "../shared/database/MonsterSummonListSet.h"
#include "../shared/database/MonsterUnderTheCastleSet.h"
#include "../shared/database/MonsterStoneListInformationSet.h"
#include "../shared/database/JuraidMountionListInformationSet.h"
#include "../shared/database/MiningFishingTableSet.h"
#include "../shared/database/PremiumItemSet.h"
#include "../shared/database/PremiumItemExpSet.h"
#include "../shared/database/UserDailyOpSet.h"
#include "../shared/database/UserItemSet.h"
#include "../shared/database/KnightsSiegeWar.h"
#include "../shared/database/AchieveTitle.h"
#include "../shared/database/AchieveMain.h"
#include "../shared/database/AchieveMonster.h"
#include "../shared/database/AchieveNormal.h"
#include "../shared/database/AchieveWar.h"
#include "../shared/database/AchieveCom.h"
#include "../shared/database/PremiumBonusItemSet.h"
#include "../shared/database/MiningExchange.h"
#include "../shared/database/ChaosStoneSummonList.h"
#include "../shared/database/ChaosStoneRespawn.h"
#include "../shared/database/BanishOfWinnerNpc.h"
#include "../shared/database/GiveItemLuaFunctionSet.h"
#include "../shared/database/CharacterSetValid.h"
#include "../shared/database/CindirellaAll.h"
#include "../shared/database/CswAll.h"
#include "../shared/database/ItemRightClickExchange.h"
#include "../shared/database/BotSaveData.h"

#include "../shared/database/EventBeefSchedulePlayTimerSet.h"

#include "../shared/database/DungeonDefenceMonsterListSet.h"
#include "../shared/database/DungeonDefenceStageListSet.h"
#include "../shared/database/ReturnLetterGiftItemSet.h"
#include "../shared/database/PetStatsInfo.h"
#include "../shared/database/PetTransformSet.h"
#include "../shared/database/Gift_Random_Item_Event.h" 
#include "../shared/database/SendMessagesSet.h"		
#include "../shared/database/RightTopTitleSet.h"	
#include "../shared/database/TopLeftSet.h"
#include "../shared/database/MonsterRespawnStableListSet.h"
#include "../shared/database/UserLootSettingsSet.h"
#include "../shared/database/RandomBossSet.h"
#include "../shared/database/RandomBossStageSet.h"
#include "../shared/database/ZindanWarStageSet.h"	
#include "../shared/database/ZindanWarSummonList.h"
#include "../shared/database/WheelSetData.h"
#include "../shared/database/RimaLotterySet.h"
#include "../shared/database/AntiAfkListSet.h"
#include "../shared/database/ZoneKillReward.h"

#include "../shared/database/TimedNoticeList.h"
#include "../shared/database/ClickRankBugSet.h"
#include "../shared/database/EventTimerShowListSet.h"
#include "../shared/database/ServerSettingSet.h"
#include "../shared/database/DamageSettingSet.h"

// AI Server Related Includes
#include "../shared/database/NpcItemSet.h"
#include "../shared/database/MonsterItemSet.h"
#include "../shared/database/MakeItemGroupSet.h"
#include "../shared/database/MakeWeaponTableSet.h"
#include "../shared/database/MakeDefensiveTableSet.h"
#include "../shared/database/MakeGradeItemTableSet.h"
#include "../shared/database/MakeLareItemTableSet.h"
#include "../shared/database/NpcTableSet.h"
#include "../shared/database/NpcPosSet.h"
#include "PusItemSet.h"
#include "PusCategorty.h"
#include "../shared/database/DailyQuestSet.h"
//Collection Race
#include "../shared/database/CollectionRaceEventSet.h"
#include "../shared/database/ReloadItemTableSet.h"

// NPC Related Load Methods

bool CGameServerDlg::LoadNpcItemTable()
{
	LOAD_TABLE(CNpcItemSet, g_DBAgent.m_GameDB, &m_NpcItemArray, false, false);
}
bool CGameServerDlg::LoadZoneKillReward()
{
	LOAD_TABLE(CZoneKillReward, g_DBAgent.m_GameDB, &m_ZoneKillReward, false, false);
}

bool CGameServerDlg::LoadAntiAfkListTable()
{
	Guard lock(m_AntiAfkListLock);
	m_AntiAfkList.clear();
	LOAD_TABLE(CAntifAfkListSet, g_DBAgent.m_GameDB, nullptr, true, false);
}

bool CGameServerDlg::LoadEventTimerShowTable()
{
	LOAD_TABLE(CEventTimerShowSet, g_DBAgent.m_GameDB, &m_EventTimerShowArray, true, false);
}

bool CGameServerDlg::LoadDamageSettingTable()
{
	LOAD_TABLE(CDamageSettingSet, g_DBAgent.m_GameDB, &pDamageSetting, true, false);
}

bool CGameServerDlg::LoadMonsterItemTable()
{
	LOAD_TABLE(CMonsterItemSet, g_DBAgent.m_GameDB, &m_MonsterItemArray, false, false);
}

bool CGameServerDlg::LoadCharacterSetValidTable()
{
	LOAD_TABLE(CCharacterSetValid, g_DBAgent.m_GameDB, &m_CharacterSetValidArray, true, false);
}

bool CGameServerDlg::BurningFeatureTable()
{
	LOAD_TABLE(CBurningFeatureSet, g_DBAgent.m_GameDB, nullptr, false, false);
}

bool CGameServerDlg::LoadMakeItemGroupTable()
{
	LOAD_TABLE(CMakeItemGroupSet, g_DBAgent.m_GameDB, &m_MakeItemGroupArray, false, false);
}

bool CGameServerDlg::LoadMakeItemGroupRandomTable()
{
	LOAD_TABLE(CMakeItemGroupRandomSet, g_DBAgent.m_GameDB, &m_MakeItemGroupRandomArray, false, false);
}

bool CGameServerDlg::LoadMakeWeaponItemTableData()
{
	LOAD_TABLE(CMakeWeaponTableSet, g_DBAgent.m_GameDB, &m_MakeWeaponItemArray, true, false);
}

bool CGameServerDlg::LoadItemRightClickExchangeTable()
{
	LOAD_TABLE(CItemRightClickExchange, g_DBAgent.m_GameDB, &m_LoadRightClickExchange, true, false);
}

bool CGameServerDlg::LoadMakeDefensiveItemTableData()
{
	LOAD_TABLE(CMakeDefensiveTableSet, g_DBAgent.m_GameDB, &m_MakeDefensiveItemArray, true, false);
}

bool CGameServerDlg::LoadMakeGradeItemTableData()
{
	LOAD_TABLE(CMakeGradeItemTableSet, g_DBAgent.m_GameDB, &m_MakeGradeItemArray, false, false);
}

bool CGameServerDlg::LoadMakeLareItemTableData()
{
	LOAD_TABLE(CMakeLareItemTableSet, g_DBAgent.m_GameDB, &m_MakeLareItemArray, false, false);
}

bool CGameServerDlg::LoadNpcTableData(bool bNpcData /*= true*/)
{
	if (bNpcData) { LOAD_TABLE(CNpcTableSet, g_DBAgent.m_GameDB, &m_arNpcTable, false, false); }
	else { LOAD_TABLE(CMonTableSet, g_DBAgent.m_GameDB, &m_arMonTable, false, false); }
}

bool CGameServerDlg::LoadNpcPosTable()
{
	LOAD_TABLE(CNpcPosSet, g_DBAgent.m_GameDB, nullptr, false, false);
}

bool CGameServerDlg::CreateBottomZoneMap()
{
	Sleep(1000);
	foreach_stlmap(itr, m_ZoneArray)
	{
		_BOTTOM_USER_LIST* pBottom = new _BOTTOM_USER_LIST();
		pBottom->m_UserList.clear();
		if (!m_BottomUserListArray.PutData(itr->first, pBottom))
			return false;
	} return true;
}

bool CGameServerDlg::LoadItemTable()
{
	LOAD_TABLE(CItemTableSet, g_DBAgent.m_GameDB, &m_ItemtableArray, false, false);
}
bool CGameServerDlg::LoadWheelDataSetTable()
{
	LOAD_TABLE(CWheelData, g_DBAgent.m_GameDB, &m_ItemWheelArray, true, false);
}
bool CGameServerDlg::ReLoadItemTable()
{
	LOAD_TABLE(CReloadItemTableSet, g_DBAgent.m_GameDB, &m_ItemtableArray, false, true);
}

bool CGameServerDlg::LoadItemSellTable()
{
	LOAD_TABLE(CItemSellTableSet, g_DBAgent.m_GameDB, &m_ItemSellTableArray, false, false);


}bool CGameServerDlg::LoadZoneOnlineRewardTable()
{
	// Zone Online / Pre Reward sistemi kapali.
	// Tablo yuklenmez; startup/reload bu sistem yuzunden odul/efekt/mesaj baslatmaz.
	m_ZoneOnlineRewardArrayLock.lock();
	m_ZoneOnlineRewardArray.clear();
	m_ZoneOnlineRewardArrayLock.unlock();
	return true;
}

bool CGameServerDlg::LoadItemPremiumGiftTable()
{
	LOAD_TABLE(CItemPremiumGift, g_DBAgent.m_GameDB, &m_ItemPremiumGiftArray, false, false);
}

bool CGameServerDlg::LoadSetItemTable()
{
	LOAD_TABLE(CSetItemTableSet, g_DBAgent.m_GameDB, &m_SetItemArray, true, false);
}

bool CGameServerDlg::LoadItemExchangeTable()
{
	LOAD_TABLE(CItemExchangeSet, g_DBAgent.m_GameDB, &m_ItemExchangeArray, true, false);
}

bool CGameServerDlg::LoadItemExchangeExpTable()
{
	LOAD_TABLE(CItemExchangeExpSet, g_DBAgent.m_GameDB, &m_ItemExchangeExpArray, true, false);
}

bool CGameServerDlg::LoadItemSpecialExchangeTable()
{
	LOAD_TABLE(CItemSpecialExchangeSet, g_DBAgent.m_GameDB, &m_ItemSpecialExchangeArray, true, false);
}

bool CGameServerDlg::LoadItemExchangeCrashTable()
{
	LOAD_TABLE(CItemExchangeCreashSet, g_DBAgent.m_GameDB, &m_ItemExchangeCrashArray, true, false);
}

bool CGameServerDlg::LoadItemUpgradeTable()
{
	LOAD_TABLE(CItemUpgradeSet, g_DBAgent.m_GameDB, &m_ItemUpgradeArray, false, false);
}

bool CGameServerDlg::LoadItemOpTable()
{
	LOAD_TABLE(CItemOpSet, g_DBAgent.m_GameDB, &m_ItemOpArray, true, false);
}

bool CGameServerDlg::LoadServerResourceTable()
{
	LOAD_TABLE(CServerResourceSet, g_DBAgent.m_GameDB, &m_ServerResourceArray, false, false);
}

bool CGameServerDlg::LoadQuestHelperTable()
{
	m_questNpcLock.lock();
	m_QuestNpcList.clear();
	m_questNpcLock.unlock();
	LOAD_TABLE(CQuestHelperSet, g_DBAgent.m_GameDB, &m_QuestHelperArray, true, false);
}

bool CGameServerDlg::LoadQuestMonsterTable()
{
	LOAD_TABLE(CQuestMonsterSet, g_DBAgent.m_GameDB, &m_QuestMonsterArray, true, false);
}

bool CGameServerDlg::LoadMagicTable()
{
	LOAD_TABLE(CMagicTableSet, g_DBAgent.m_GameDB, &m_MagictableArray, false, false);
}

bool CGameServerDlg::LoadMagicType1()
{
	LOAD_TABLE(CMagicType1Set, g_DBAgent.m_GameDB, &m_Magictype1Array, false, false);
}

bool CGameServerDlg::LoadMagicType2()
{
	LOAD_TABLE(CMagicType2Set, g_DBAgent.m_GameDB, &m_Magictype2Array, false, false);
}

bool CGameServerDlg::LoadMagicType3()
{
	LOAD_TABLE(CMagicType3Set, g_DBAgent.m_GameDB, &m_Magictype3Array, false, false);
}

bool CGameServerDlg::LoadMagicType4()
{
	LOAD_TABLE(CMagicType4Set, g_DBAgent.m_GameDB, &m_Magictype4Array, false, false);
}

bool CGameServerDlg::LoadMagicType5()
{
	LOAD_TABLE(CMagicType5Set, g_DBAgent.m_GameDB, &m_Magictype5Array, false, false);
}

bool CGameServerDlg::LoadMagicType6()
{
	LOAD_TABLE(CMagicType6Set, g_DBAgent.m_GameDB, &m_Magictype6Array, false, false);
}

bool CGameServerDlg::LoadMagicType7()
{
	LOAD_TABLE(CMagicType7Set, g_DBAgent.m_GameDB, &m_Magictype7Array, false, false);
}

bool CGameServerDlg::LoadMagicType8()
{
	LOAD_TABLE(CMagicType8Set, g_DBAgent.m_GameDB, &m_Magictype8Array, false, false);
}

bool CGameServerDlg::LoadMagicType9()
{
	LOAD_TABLE(CMagicType9Set, g_DBAgent.m_GameDB, &m_Magictype9Array, false, false);
}

bool CGameServerDlg::LoadRentalList()
{
	LOAD_TABLE(CRentalItemSet, g_DBAgent.m_GameDB, &m_RentalItemArray, true, false);
}
bool CGameServerDlg::LoadCoefficientDamageTable()
{
	LOAD_TABLE(CCoefficientDamageSet, g_DBAgent.m_GameDB, &m_CoefficientDamageArray, false, false);
}
bool CGameServerDlg::LoadCoefficientTable()
{
	LOAD_TABLE(CCoefficientSet, g_DBAgent.m_GameDB, &m_CoefficientArray, false, false);
}

bool CGameServerDlg::LoadLevelUpTable()
{
	LOAD_TABLE(CLevelUpTableSet, g_DBAgent.m_GameDB, &m_LevelUpArray, false, false);
}

bool CGameServerDlg::LoadAllKnights()
{
	return g_DBAgent.LoadAllKnights(m_KnightsArray);
}

bool CGameServerDlg::XCodeLoadEventTables()
{
	pEventTimeOpt.mtimeforloop.clear();
	LOAD_TABLE_ERROR_ONLY(CEventTimeDaySet, g_DBAgent.m_GameDB, &pEventTimeOpt, true, false);
	LOAD_TABLE_ERROR_ONLY(CEventTimeOptSet, g_DBAgent.m_GameDB, &pEventTimeOpt, true, false);
	return true;
}

bool CGameServerDlg::XCodeLoadEventVroomTables()
{
	LOAD_TABLE(CEventVroomListSet, g_DBAgent.m_GameDB, &pEventTimeOpt, true, false);
}

bool CGameServerDlg::LoadKnightsAllianceTable(bool bIsSlient)
{
	LOAD_TABLE(CKnightsAllianceSet, g_DBAgent.m_GameDB, &m_KnightsAllianceArray, true, bIsSlient);
}

bool CGameServerDlg::LoadMiningExchangeListTable()
{
	LOAD_TABLE(CMiningExchange, g_DBAgent.m_GameDB, &m_MiningExchangeArray, true, false);
}

bool CGameServerDlg::LoadEventRewardTable()
{
	for (int i = 0; i < 18; i++)
		g_pMain->pChaosReward[i].Initialize();

	LOAD_TABLE_ERROR_ONLY(CEventRewardSet, g_DBAgent.m_GameDB, &m_EventRewardArray, true, false);
	LOAD_TABLE_ERROR_ONLY(CEventChaosGift, g_DBAgent.m_GameDB, nullptr, true, false);
	return true;
}

bool CGameServerDlg::LoadBeefEventPlayTimerTable()
{
	LOAD_TABLE(CEventBeefSchedulePlayTimer, g_DBAgent.m_GameDB, &m_BeefEventPlayTimerArray, true, false);
}

bool CGameServerDlg::LoadFtEventPlayTimerTable()
{
	LOAD_TABLE(CFtEventTimeSet, g_DBAgent.m_GameDB, &pForgettenTemple.ptimeopt, true, false);
}

bool CGameServerDlg::LoadAutomaticCommand()
{
	LOAD_TABLE(CAutomaticCommandSet, g_DBAgent.m_GameDB, &g_pMain->m_AutomaticCommandArray, true, false);
}

bool CGameServerDlg::LoadAchieveTitleTable()
{
	LOAD_TABLE(CAchieveTitleSet, g_DBAgent.m_GameDB, &m_AchieveTitleArray, true, false);
}

bool CGameServerDlg::LoadAchieveWarTable()
{
	LOAD_TABLE(CAchieveWarSet, g_DBAgent.m_GameDB, &m_AchieveWarArray, true, false);
}

bool CGameServerDlg::LoadAchieveMainTable()
{
	LOAD_TABLE(CAchieveMainSet, g_DBAgent.m_GameDB, &m_AchieveMainArray, true, false);
}

bool CGameServerDlg::LoadAchieveMonsterTable()
{
	LOAD_TABLE(CAchieveMonsterSet, g_DBAgent.m_GameDB, &m_AchieveMonsterArray, true, false);
}

bool CGameServerDlg::LoadAchieveNormalTable()
{
	LOAD_TABLE(CAchieveNormalSet, g_DBAgent.m_GameDB, &m_AchieveNormalArray, true, false);
}

bool CGameServerDlg::LoadRandomBossTable()
{
	LOAD_TABLE(CRandomBossSet, g_DBAgent.m_GameDB, &m_MonsterBossRandom, true, false);
}

bool CGameServerDlg::LoadRandomBoosStageTable()
{
	LOAD_TABLE(CRandomBossStageSet, g_DBAgent.m_GameDB, &m_MonsterBossStage, true, false);
}

bool CGameServerDlg::LoadAchieveComTable()
{
	LOAD_TABLE(CAchieveComSet, g_DBAgent.m_GameDB, &m_AchieveComArray, true, false);
}

bool CGameServerDlg::LoadDungeonDefenceMonsterTable()
{
	LOAD_TABLE(CDungeonDefenceMonsterListSet, g_DBAgent.m_GameDB, &m_DungeonDefenceMonsterListArray, true, false);
}

bool CGameServerDlg::LoadDungeonDefenceStageTable()
{
	LOAD_TABLE(CDungeonDefenceStageListSet, g_DBAgent.m_GameDB, &m_DungeonDefenceStageListArray, true, false);
}

bool CGameServerDlg::LoadKnightReturnLetterGiftItemTable()
{
	LOAD_TABLE(CUserReturnLetterGiftItem, g_DBAgent.m_GameDB, &m_KnightReturnLetterGiftListArray, true, false);
}

bool CGameServerDlg::LoadKnightsSiegeWarsTable(bool bIsSlient)
{
	LOAD_TABLE(CKnightsSiegeWarfare, g_DBAgent.m_GameDB, &pSiegeWar, true, false);
}

bool CGameServerDlg::LoadCollectionRaceEventTable()
{
	LOAD_TABLE(CollectionRaceEventListSet, g_DBAgent.m_GameDB, &m_CollectionRaceListArray, true, false);
}

bool CGameServerDlg::LoadRimaLotteryEventTable()
{
	LOAD_TABLE(CRimaLottery, g_DBAgent.m_GameDB, &m_RimaLotteryArray, true, false);
}

bool CGameServerDlg::LoadDailyQuestListTable()
{
	LOAD_TABLE_ERROR_ONLY(CDailyQyestSet, g_DBAgent.m_GameDB, &m_DailyQuestArray, true, false);
	return true;
}

//Pet System
bool CGameServerDlg::LoadPetStatsInfoTable()
{
	LOAD_TABLE(CPetStatsInfoSet, g_DBAgent.m_GameDB, &m_PetInfoSystemArray, true, false);
}

bool CGameServerDlg::LoadPetImageChangeTable()
{
	LOAD_TABLE(CPetImageChane, g_DBAgent.m_GameDB, &m_PetTransformSystemArray, true, false);
}

bool CGameServerDlg::LoadZindanWarStageListTable()
{
	LOAD_TABLE(CZindanWarStagesList, g_DBAgent.m_GameDB, &m_ZindanWarStageListArray, true, false);
}

bool CGameServerDlg::LoadZindanWarSummonListTable()
{
	LOAD_TABLE(CZindanWarMonsList, g_DBAgent.m_GameDB, &m_ZindanWarSummonListArray, true, false);
}

bool CGameServerDlg::LoadUserRankings()
{
	g_DBAgent.UpdateRanks();
	CUserPersonalRankSet UserPersonalRankSet(g_DBAgent.m_GameDB, &m_UserKarusPersonalRankMap, &m_UserElmoPersonalRankMap);
	CUserKnightsRankSet  UserKnightsRankSet(g_DBAgent.m_GameDB, &m_UserKarusKnightsRankMap, &m_UserElmoKnightsRankMap);
	TCHAR* szError = nullptr;

	// Cleanup first, in the event it's already loaded (we'll have this automatically reload in-game)
	CleanupUserRankings();

	szError = UserPersonalRankSet.Read(true);
	if (szError != nullptr)
	{
		printf("ERROR: Failed to load personal rankings, error:\n%s\n", szError);
		return false;
	}

	szError = UserKnightsRankSet.Read(true);
	if (szError != nullptr)
	{
		printf("ERROR: Failed to load user knights rankings, error:\n%s\n", szError);
		return false;
	}

	std::vector< _USER_RANK> m_persokarus, m_persoelmorad, m_knightkarus, m_knightelmorad;

	m_userRankingsLock.lock();
	foreach(itr, m_UserKarusPersonalRankMap)
		if (itr->second) m_persokarus.push_back(*itr->second);

	foreach(itr, m_UserElmoPersonalRankMap)
		if (itr->second) m_persoelmorad.push_back(*itr->second);

	foreach(itr, m_UserKarusKnightsRankMap)
		if (itr->second) m_knightkarus.push_back(*itr->second);

	foreach(itr, m_UserElmoKnightsRankMap)
		if (itr->second) m_knightelmorad.push_back(*itr->second);
	m_userRankingsLock.unlock();

	foreach(itr, m_persokarus)
	{
		CUser* pUser = GetUserPtr(itr->strUserName, NameType::TYPE_CHARACTER);
		if (pUser)
			pUser->m_bPersonalRank = uint8(itr->nRank);
	}

	foreach(itr, m_persoelmorad)
	{
		CUser* pUser = GetUserPtr(itr->strUserName, NameType::TYPE_CHARACTER);
		if (pUser)
			pUser->m_bPersonalRank = uint8(itr->nRank);
	}

	foreach(itr, m_knightkarus)
	{
		CUser* pUser = GetUserPtr(itr->strUserName, NameType::TYPE_CHARACTER);
		if (pUser)
			pUser->m_bKnightsRank = uint8(itr->nRank);
	}

	foreach(itr, m_knightelmorad)
	{
		CUser* pUser = GetUserPtr(itr->strUserName, NameType::TYPE_CHARACTER);
		if (pUser)
			pUser->m_bKnightsRank = uint8(itr->nRank);
	}

	return true;
}

bool CGameServerDlg::LoadCharacterSealItemTable()
{
	LOAD_TABLE(CCharacterSealSet, g_DBAgent.m_GameDB, &m_CharacterSealItemArray, true, false);
}

/**
* @brief	Loads the user item table from the database.
*/
bool CGameServerDlg::LoadCharacterSealItemTableAll()
{
	return g_DBAgent.LoadAllCharacterSealItems(m_CharacterSealItemArray);
}

void CGameServerDlg::CleanupUserRankings()
{
	std::set<_USER_RANK*> deleteSet, deleteSet2, deleteSet3, deleteSet4;
	std::vector< std::string> m_prank, m_krank;

	m_userRankingsLock.lock();
	foreach(itr, m_UserElmoPersonalRankMap)
	{
		if (itr->second == nullptr) continue;
		m_prank.push_back(itr->first);
		deleteSet.insert(itr->second);
	}

	foreach(itr, m_UserKarusPersonalRankMap)
	{
		if (itr->second == nullptr) continue;
		m_prank.push_back(itr->first);
		deleteSet2.insert(itr->second);
	}

	foreach(itr, m_UserElmoKnightsRankMap)
	{
		if (itr->second == nullptr) continue;
		m_krank.push_back(itr->first);
		deleteSet3.insert(itr->second);
	}

	foreach(itr, m_UserKarusKnightsRankMap)
	{
		if (itr->second == nullptr) continue;
		m_krank.push_back(itr->first);
		deleteSet4.insert(itr->second);
	}

	m_UserElmoPersonalRankMap.clear();
	m_UserKarusPersonalRankMap.clear();
	m_UserElmoKnightsRankMap.clear();
	m_UserKarusKnightsRankMap.clear();

	// Free the memory used by the _USER_RANK structs
	foreach(itr, deleteSet)  delete* itr;
	foreach(itr, deleteSet2) delete* itr;
	foreach(itr, deleteSet3) delete* itr;
	foreach(itr, deleteSet4) delete* itr;

	// These only store pointers to memory that was already freed by the primary rankings maps.
	m_playerRankings[KARUS_ARRAY].clear();
	m_playerRankings[ELMORAD_ARRAY].clear();
	m_userRankingsLock.unlock();

	foreach(itr, m_prank)
	{
		CUser* pUser = GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (pUser != nullptr) pUser->m_bPersonalRank = -1;
	}

	foreach(itr, m_krank)
	{
		CUser* pUser = GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (pUser != nullptr) pUser->m_bKnightsRank = -1;
	}
}

bool CGameServerDlg::LoadKnightsCapeTable()
{
	LOAD_TABLE(CKnightsCapeSet, g_DBAgent.m_GameDB, &m_KnightsCapeArray, false, false);
}

bool CGameServerDlg::LoadKnightsRankTable(bool bWarTime /*= false*/, bool bIsSlient /*= false*/)
{
	std::string strKarusCaptainNames, strElmoCaptainNames;
	LOAD_TABLE_ERROR_ONLY(CKnightsRankSet, g_DBAgent.m_GameDB, nullptr, true, bIsSlient);

	if (!bWarTime)
		return true;

	CKnightsRankSet& pSet = _CKnightsRankSet; // kind ugly generic naming

	if (pSet.nKarusCount > 0)
	{
		Packet result;
		GetServerResource(IDS_KARUS_CAPTAIN, &strKarusCaptainNames,
			pSet.strKarusCaptain[0], pSet.strKarusCaptain[1], pSet.strKarusCaptain[2],
			pSet.strKarusCaptain[3], pSet.strKarusCaptain[4]);
		ChatPacket::Construct(&result, (uint8)ChatType::WAR_SYSTEM_CHAT, &strKarusCaptainNames);
		Send_All(&result, nullptr, (uint8)Nation::KARUS);
	}

	if (pSet.nElmoCount > 0)
	{
		Packet result;
		GetServerResource(IDS_ELMO_CAPTAIN, &strElmoCaptainNames,
			pSet.strElmoCaptain[0], pSet.strElmoCaptain[1], pSet.strElmoCaptain[2],
			pSet.strElmoCaptain[3], pSet.strElmoCaptain[4]);
		ChatPacket::Construct(&result, (uint8)ChatType::WAR_SYSTEM_CHAT, &strElmoCaptainNames);
		Send_All(&result, nullptr, (uint8)Nation::ELMORAD);
	}

	return true;
}

bool CGameServerDlg::LoadStartPositionTable()
{
	LOAD_TABLE(CStartPositionSet, g_DBAgent.m_GameDB, &m_StartPositionArray, false, false);
}

bool CGameServerDlg::LoadBattleTable()
{
	LOAD_TABLE(CBattleSet, g_DBAgent.m_GameDB, &m_byOldVictory, true, false);
}

bool CGameServerDlg::LoadCapeCastellanBonus()
{
	LOAD_TABLE(xCapeCastellanBonus, g_DBAgent.m_GameDB, &m_CapeCastellanBonus, true, false);
}

bool CGameServerDlg::LoadKingSystem()
{
	LOAD_TABLE_ERROR_ONLY(CKingSystemSet, g_DBAgent.m_GameDB, &m_KingSystemArray, true, false);
	LOAD_TABLE_ERROR_ONLY(CKingCandidacyNoticeBoardSet, g_DBAgent.m_GameDB, &m_KingSystemArray, true, false);
	LOAD_TABLE_ERROR_ONLY(CKingNominationListSet, g_DBAgent.m_GameDB, &m_KingSystemArray, true, false);
	LOAD_TABLE(CKingElectionListSet, g_DBAgent.m_GameDB, &m_KingSystemArray, true, false);
}

bool CGameServerDlg::LoadSheriffTable()
{
	LOAD_TABLE(CSheriffReportList, g_DBAgent.m_GameDB, &m_SheriffArray, true, false);
}

bool CGameServerDlg::LoadMonsterResourceTable()
{
	LOAD_TABLE(CMonsterResourceSet, g_DBAgent.m_GameDB, &m_MonsterResourceArray, false, false);
}

bool CGameServerDlg::LoadForgettenTempleSummonListTable()
{
	LOAD_TABLE(CForgettenTMonsList, g_DBAgent.m_GameDB, &m_ForgettenTempleMonsterArray, true, false);
}

bool CGameServerDlg::LoadForgettenTempleStagesListTable()
{
	LOAD_TABLE(CForgettenStagesList, g_DBAgent.m_GameDB, &m_ForgettenTempleStagesArray, true, false);
}

bool CGameServerDlg::LoadMonsterChallengeTable()
{
	LOAD_TABLE(CMonsterChallenge, g_DBAgent.m_GameDB, &m_MonsterChallengeArray, true, false);
}

bool CGameServerDlg::LoadMonsterChallengeSummonListTable()
{
	LOAD_TABLE(CMonsterChallengeSummonList, g_DBAgent.m_GameDB, &m_MonsterChallengeSummonListArray, true, false);
}

bool CGameServerDlg::LoadMonsterSummonListTable()
{
	LOAD_TABLE(CMonsterSummonListSet, g_DBAgent.m_GameDB, &m_MonsterSummonList, true, false);
}

bool CGameServerDlg::LoadChaosStoneMonsterListTable()
{
	LOAD_TABLE(CChaosStoneSummonListSet, g_DBAgent.m_GameDB, &m_ChaosStoneSummonListArray, true, false);
}

bool CGameServerDlg::LoadChaosStoneCoordinateTable()
{
	LOAD_TABLE(CChaosStoneCoordinate, g_DBAgent.m_GameDB, &m_ChaosStoneRespawnCoordinateArray, true, false);
}

bool CGameServerDlg::LoadMonsterRespawnLoopListTable()
{
	LOAD_TABLE(CMonsterRespawnStableListSet, g_DBAgent.m_GameDB, &m_MonsterRespawnLoopArray, true, false);
}

bool CGameServerDlg::LoadMonsterUnderTheCastleTable()
{
	LOAD_TABLE(CMonsterUnderTheCastleSet, g_DBAgent.m_GameDB, &m_MonsterUnderTheCastleArray, true, false);
}

bool CGameServerDlg::LoadMonsterStoneListInformationTable()
{
	LOAD_TABLE(CMonsterStoneListInformationSet, g_DBAgent.m_GameDB, &m_MonsterStoneListInformationArray, true, false);
}

bool CGameServerDlg::LoadJuraidMountionListInformationTable()
{
	LOAD_TABLE(CJuraidMountionListInformationSet, g_DBAgent.m_GameDB, &m_JuraidMountionListInformationArray, true, false);
}

bool CGameServerDlg::LoadMiningFishingItemTable()
{
	LOAD_TABLE(CMiningFishingTableSet, g_DBAgent.m_GameDB, &m_MiningFishingItemArray, true, false);
}

bool CGameServerDlg::LoadPremiumItemTable()
{
	LOAD_TABLE(CPremiumItemSet, g_DBAgent.m_GameDB, &m_PremiumItemArray, true, false);
}

bool CGameServerDlg::LoadPremiumItemExpTable()
{
	LOAD_TABLE(CPremiumItemExpSet, g_DBAgent.m_GameDB, &m_PremiumItemExpArray, true, false);
}

bool CGameServerDlg::LoadUserDailyOpTable()
{
	LOAD_TABLE(CUserDailyOpSet, g_DBAgent.m_GameDB, &m_UserDailyOpMap, true, false);
}

bool CGameServerDlg::LoadEventTriggerTable()
{
	LOAD_TABLE(CEventTriggerSet, g_DBAgent.m_GameDB, &m_EventTriggerArray, true, false);
}

bool CGameServerDlg::LoadStartPositionRandomTable()
{
	LOAD_TABLE(CStartPositionRandomSet, g_DBAgent.m_GameDB, &m_StartPositionRandomArray, true, false);
}

bool CGameServerDlg::LoadGiftEventArray()
{
	LOAD_TABLE(CGiftEventArray, g_DBAgent.m_GameDB, &m_RandomItemArray, true, false);
}

bool CGameServerDlg::LoadSendMessageTable()
{
	LOAD_TABLE(CSendMessageSet, g_DBAgent.m_GameDB, &m_SendMessageArray, true, false);
}

bool CGameServerDlg::LoadRightTopTitleTable()
{
	LOAD_TABLE(CRightTopTitleSet, g_DBAgent.m_GameDB, &m_RightTopTitleArray, true, false);
}

bool CGameServerDlg::LoadTopLeftTable()
{
	LOAD_TABLE(CTopLeftSet, g_DBAgent.m_GameDB, &m_TopLeftArray, true, false);
}

bool CGameServerDlg::LoadBanishWinnerTable()
{
	LOAD_TABLE(CBanishWinner, g_DBAgent.m_GameDB, &m_WarBanishOfWinnerArray, true, false);
}

bool CGameServerDlg::LoadGiveItemExchangeTable()
{
	LOAD_TABLE(CLuaGiveItemExchangeSet, g_DBAgent.m_GameDB, &m_LuaGiveItemExchangeArray, true, false);
}

bool CGameServerDlg::LoadUserItemTable()
{
	LOAD_TABLE(CUserItemSet, g_DBAgent.m_GameDB, &m_UserItemArray, true, false);
}

bool CGameServerDlg::LoadSaveBotDataTable() 

{ 
	LOAD_TABLE(CBotSaveDataSet, g_DBAgent.m_GameDB, &m_BotSaveDataArray, true, false); 
}

bool CGameServerDlg::LoadPusCategoryTable()
{
	LOAD_TABLE(CPusItemCategory, g_DBAgent.m_GameDB, &m_PusCategoryArray, true, false);

}

bool CGameServerDlg::LoadObjectPosTable()
{
	LOAD_TABLE(CObjectPosSet, g_DBAgent.m_GameDB, &m_ObjectEventArray, true, false);
}

bool CGameServerDlg::LoadPusItemsTable()
{
	LOAD_TABLE(CPusItemSet, g_DBAgent.m_GameDB, &m_PusItemArray, false, false);
}

bool CGameServerDlg::LoadTimedNoticeTable()
{
	LOAD_TABLE(CTimedNotice, g_DBAgent.m_GameDB, &m_TimedNoticeArray, true, false);
}

bool CGameServerDlg::LoadRankBugTable()
{
	LOAD_TABLE(CRankBugSet, g_DBAgent.m_GameDB, nullptr, true, false);
}

bool CGameServerDlg::LoadUserLootSettingsTable()
{
	LOAD_TABLE(CUserLootSettingsSet, g_DBAgent.m_GameDB, &UserLootSettings, false, false);
}

bool CGameServerDlg::LoadServerSettingsData()
{
	LOAD_TABLE(CServerSettingSet, g_DBAgent.m_GameDB, &pServerSetting, false, false);
}

bool CGameServerDlg::LoadCindirellaItemsTable()
{
	LOAD_TABLE_ERROR_ONLY(CCindirellaItemSet1, g_DBAgent.m_GameDB, &m_CindirellaItemsArray[0], true, false);
	LOAD_TABLE_ERROR_ONLY(CCindirellaItemSet2, g_DBAgent.m_GameDB, &m_CindirellaItemsArray[1], true, false);
	LOAD_TABLE_ERROR_ONLY(CCindirellaItemSet3, g_DBAgent.m_GameDB, &m_CindirellaItemsArray[2], true, false);
	LOAD_TABLE_ERROR_ONLY(CCindirellaItemSet4, g_DBAgent.m_GameDB, &m_CindirellaItemsArray[3], true, false);
	LOAD_TABLE_ERROR_ONLY(CCindirellaItemSet5, g_DBAgent.m_GameDB, &m_CindirellaItemsArray[4], true, false);
	return true;
}

bool CGameServerDlg::LoadCindirellaSettingTable()
{
	LOAD_TABLE(CCindirellaSettingSet, g_DBAgent.m_GameDB, nullptr, true, false);
}

bool CGameServerDlg::LoadCindirellaRewardsTable()
{
	LOAD_TABLE(CCindirellaRewardSet, g_DBAgent.m_GameDB, nullptr, true, false);
}

bool CGameServerDlg::LoadCindirellaStatSetTable()
{
	LOAD_TABLE(CCindirellaStatSet, g_DBAgent.m_GameDB, &m_CindirellaStatArray, true, false);
}

bool CGameServerDlg::LoadLevelRewardTable()
{
	LOAD_TABLE(CLevelRewardsSet, g_DBAgent.m_GameDB, &m_LevelRewardArray, true, false);
}

bool CGameServerDlg::LoadLevelMerchantRewardTable()
{
	LOAD_TABLE(CLevelMerchantRewardsSet, g_DBAgent.m_GameDB, &m_LevelMerchantRewardArray, true, false);
}

bool CGameServerDlg::LoadJackPotSettingTable()
{
	LOAD_TABLE(CJackPotSettingSet, g_DBAgent.m_GameDB, nullptr, true, false);
}

bool CGameServerDlg::LoadPerksTable()
{
	LOAD_TABLE(CPerksSet, g_DBAgent.m_GameDB, &m_PerksArray, true, false);
}

bool CGameServerDlg::LoadRankRewardTable() {
	LOAD_TABLE(CRankRewardSet, g_DBAgent.m_GameDB, &m_RankRewardArray, true, false);
}

bool CGameServerDlg::LoadCSWTables()
{
	LOAD_TABLE(CCswTimeOptSet, g_DBAgent.m_GameDB, &pCswEvent.poptions, true, false);
}

bool CGameServerDlg::MapFileLoad()
{
	ZoneInfoMap zoneMap;
	LOAD_TABLE_ERROR_ONLY(CZoneInfoSet, g_DBAgent.m_GameDB, &zoneMap, false, false);

	foreach(itr, zoneMap)
	{
		C3DMap* pMap = new C3DMap();
		_ZONE_INFO* pZone = itr->second;
		if (pZone == nullptr || !pMap->Initialize(pZone))
		{
			if (pZone != nullptr)
				printf("ERROR: Unable to load SMD - %s\n", pZone->m_MapName.c_str());

			delete pZone;
			delete pMap;
			m_ZoneArray.DeleteAllData();
			return false;
		}

		delete pZone;
		m_ZoneArray.PutData(pMap->m_nZoneNumber, pMap);
	}

	LOAD_TABLE_ERROR_ONLY(CEventSet, g_DBAgent.m_GameDB, &m_ZoneArray, true, false);
	return true;
}

bool CGameServerDlg::LoadDrakiTowerTables() { return g_DBAgent.LoadDrakiTowerTables(); }

bool CGameServerDlg::LoadChaosStoneStage() { return g_DBAgent.LoadChaosStoneFamilyStage(); }
bool CGameServerDlg::GameStartClearRemainUser() { return g_DBAgent.GameStartClearRemainUser(); }
bool CGameServerDlg::CheckTrashItemListTime() { return g_DBAgent.CheckTrashItemListTime(); }
bool CGameServerDlg::LoadDailyRankTable() { return g_DBAgent.LoadDailyRank(); }
bool CGameServerDlg::LoadBotTable() { return g_DBAgent.LoadBotTable(); }
bool CGameServerDlg::LoadBotMerchantTable()
{
	// Legacy merchant bot template list
	g_pMain->m_ArtificialMerchantArray.DeleteAllData();
	bool bLegacyLoaded = g_DBAgent.LoadBotHandlerMerchantTable();

	// New merchant mix system data (coordinate + item pools)
	g_pMain->pBotInfo.mCoordinate.DeleteAllData();
	g_pMain->pBotInfo.mItem.DeleteAllData();

	LOAD_TABLE_ERROR_ONLY(CBotMerchantInfoSet, g_DBAgent.m_GameDB, &g_pMain->pBotInfo, true, false);
	LOAD_TABLE_ERROR_ONLY(CBotMerchantItemSet, g_DBAgent.m_GameDB, &g_pMain->pBotInfo, true, false);

	return bLegacyLoaded;
}
