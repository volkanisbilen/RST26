#pragma once

#include "../shared/database/OdbcConnection.h"
#include "KingSystem.h"

enum class UserUpdateType
{
	UPDATE_LOGOUT,
	UPDATE_ALL_SAVE,
	UPDATE_PACKET_SAVE,
};

enum RentalType
{
	RENTAL_TYPE_IN_LIST		= 1,
	RENTAL_TYPE_LENDER		= 2,
	RENTAL_TYPE_BORROWER	= 3
};

struct _USER_RENTAL_ITEM
{
	std::string strUserID;
	uint64 nSerialNum;
	uint32 nRentalIndex, nItemID, nRentalMoney;
	uint16 sDurability, sRentalTime;
	int16 sMinutesRemaining;
	uint8 byRentalType, byRegType;
	char szTimeRental[30];

	_USER_RENTAL_ITEM()
	{
		memset(&szTimeRental, 0, sizeof(szTimeRental));
	}
};

typedef std::map<uint64, _USER_RENTAL_ITEM *> UserRentalMap;

class Packet;
class CUser;
struct _ITEM_DATA;
class CDBAgent  
{
public:
	CDBAgent();

	bool Startup(bool bMarsEnabled,
		tstring& strAccountDSN, tstring& strAccountUID, tstring& strAccountPWD,
		tstring& strGameDSN, tstring& strGameUID, tstring& strGamePWD,
		tstring& strLogDSN, tstring& strLogUID, tstring& strLogPWD);

	bool Connect(bool bMarsEnabled,
		tstring& strAccountDSN, tstring& strAccountUID, tstring& strAccountPWD,
		tstring& strGameDSN, tstring& strGameUID, tstring& strGamePWD,
		tstring& strLogDSN, tstring& strLogUID, tstring& strLogPWD);

	void ReportSQLError(OdbcError *pError);
	int8 AccountLogin(std::string & strAccountID, std::string & strPasswd);
	uint8 NationSelect(std::string & strAccountID, uint8 bNation);
	bool GetAllCharID(std::string & strAccountID, std::string & strCharID1, std::string & strCharID2, std::string & strCharID3, std::string & strCharID4);
	bool SetAllCharID(std::string & strAccountID, std::string & strCharID1, std::string & strCharID2, std::string & strCharID3, std::string & strCharID4);
	void LoadCharInfo(std::string & strCharID, ByteBuffer & result);
	void LoadNewCharSet(std::string& strCharID, uint16 sClass);
	void LoadNewCharValue(std::string& strCharID, uint16 sClass);
	int8 ChangeHair(std::string & strAccountID, std::string & strCharID, uint8 bOpcode, uint8 bFace, uint32 nHair);
	int8 CreateNewChar(std::string & strAccountID, int index, std::string & strCharID, uint8 bRace, uint16 sClass, uint32 nHair, uint8 bFace, uint8 bStr, uint8 bSta, uint8 bDex, uint8 bInt, uint8 bCha);
	int8 DeleteChar(std::string & strAccountID, int index, std::string & strCharID, std::string & strSocNo);
	void LoadRentalData(std::string & strAccountID, std::string & strCharID, UserRentalMap & rentalData);
	void LoadItemSealData(std::string & strAccountID, std::string & strCharID, UserItemSealMap & itemSealData);
	bool LoadUserData(std::string & strAccountID, std::string & strCharID, CUser *pUser);
	bool LoadTrashItemList(CUser *pUser);
	void InsertTrashItemList(CUser *pUser, uint32 itemid,uint32 itemcount, uint16 duration, uint32 deleteime, uint64 serial, uint8 bflag);
	void DeleteTrashItem(CUser *pUser, uint64 serialnum);
	bool LoadPusRefund(CUser *pUser);

	bool PusRefundItemDelete(CUser *pUser, uint64 serial, uint32 itemid);
	bool PusRefundItemAdd(CUser *pUser, uint64 serial, uint32 itemid, uint16 itemcount, uint16 itemduration, uint32 kcprice, uint32 buyingtime, uint8 buytype);
	uint8 CharacterSelectLoginChecking(std::string& strAccountID, std::string& strCharID);
	bool PusGiftCheck(CUser *pUser, std::string targetname);
	bool LoadWarehouseData(std::string & strAccountID, CUser *pUser);
	bool LoadVipWarehouseData(std::string & strAccountID, CUser *pUser);
	bool LoadPremiumServiceUser(std::string & strAccountID, CUser *pUser);
	bool LoadSavedMagic(CUser *pUser);
	bool LoadAchieveData(CUser *pUser);
	bool LoadGameMasterSetting(std::string strCharID,CUser *pUser);
	bool GameStartClearRemainUser();
	bool CheckTrashItemListTime();
	uint32 CreateNewPet(uint64 nSerialID, uint8 bLevel, std::string& strPetName);
	void UpdatingUserPet(uint64 nSerial);
	void LoadPetData(uint64 nSerial);
	bool LoadUserSealExpData(CUser *pUser);
	bool UpdateUserSealExpData(CUser *pUser);
	bool LoadDrakiTowerTables();
	bool LoadUserDrakiTowerData(CUser *pUser);
	void ReqDrakiTowerList(Packet & pkt, CUser* pUser);
	void UpdateDrakiTowerData(uint32 sTime, uint8 bStage, std::string & strUserID);
	bool UpdateDrakiTowerLimitLastUpdate(std::string & strUserID, uint8 EnteranceLimit);
	bool UpdateDaysDrakiTowerLimit();
	uint8 DrakiDataCheck(CUser* pUser);
	bool LoadUserReturnData(CUser *pUser);
	bool UpdateUserPerks(CUser* pUser);
	bool UpdateUserReturnData(std::string & strCharID, CUser *pUser);
	bool LoadChaosStoneFamilyStage();
	bool LoadQuestData(std::string & strCharID, CUser *pUser);
	bool UpdateQuestData(std::string & strCharID, CUser *pUser);
	bool CharacterSelectPrisonCheck(std::string & strCharID, uint8 & ZoneID);
	int8 SelectCharacterChecking(std::string & strAccountID, std::string & strCharID);
	bool SaveSheriffReports(std::string ReportedUser, std::string ReportedSheriff, std::string Crime, uint8 Yes, std::string SheriffA, uint8 No, std::string ReportDateTime);
	bool UpdateSheriffTable(uint8 Type, std::string ReportedUser, uint8 Yes, uint8 No, std::string Sheriff);
	bool SetLogInInfo(std::string & strAccountID, std::string & strCharID, std::string & strServerIP, short sServerNo, std::string & strClientIP, uint8 bInit);
	bool RemoveCurrentUser(std::string & strAccountID);
	void DestroyPusSessions(CUser * pUser);
	void InsertCheatLog(CUser * pUser);
	bool CreatePusSession(uint16 sFreeCount, CUser * pUser);
	bool UpdateClanPremiumServiceUser(CUser* pUser);
	bool UpdatePremiumServiceUser(CUser *pUser);
	bool LoadWebItemMall(std::vector<_ITEM_DATA> & itemList, CUser *pUser);
	bool InsertCurrentUser(std::string & strAccountID, std::string & strCharID);

	bool LoadPerksData(std::string strUserID, CUser* pUser);

	bool LoadSkillShortcut(Packet & result, CUser *pUser);
	void SaveSkillShortcut(CUser *pUser);
	void RequestFriendList(std::vector<std::string> & friendList, CUser *pUser);
	FriendAddResult AddFriend(short sid, short tid);
	FriendRemoveResult RemoveFriend(std::string & strCharID, CUser *pUser);
	uint8 VIPStorageSetPassword(std::string &strSealPasswd, std::string & strAccountID, uint8 KeyCancel);
	uint8 VIPStorageCancelPassword(std::string & strAccountID, uint8 KeyCancel);
	uint8 VIPStorageUseVaultKey(std::string & strAccountID, uint32 iExpirationTime);
	bool UpdateUser(std::string & strCharID, UserUpdateType type, CUser *pUser);
	bool UpdateWarehouseData(std::string & strAccountID, UserUpdateType type, CUser *pUser);
	bool UpdateVipWarehouseData(std::string & strAccountID, UserUpdateType type, CUser *pUser);
	bool UpdateSavedMagic(CUser *pUser);
	bool UpdateAchieveData(CUser *pUser);
	uint8 SealItem(std::string strSealPasswd, uint64 nItemSerial, uint32 nItemID, uint8 bSealType, CUser *pUser);

	bool LoadDrakiTowerDailyRank(std::vector<_DRAKI_TOWER_FORDAILY_RANKING>& List);
	bool LoadUserDailyRank(std::string& strCharID, CUser* pUser);
	bool LoadDailyRank();
	bool SaveDailyRank(std::string &CharID, CUser *pUser);

	bool LoadBotTable();
	bool LoadMorankerBotTable(std::vector<_BOT_DATA*> & botList);

	// Character Seal Related Methods
	bool LoadAllCharacterSealItems(CharacterSealItemArray & m_CharacterSealArray);
	bool LoadCharacterSealUserData(std::string & strAccountID, std::string & strCharID, _CHARACTER_SEAL_ITEM *pUser);
	int8 CharacterSealProcess(std::string & strAcountID, std::string & strUserID, std::string & strPassword, uint64 nItemSerial);
	int8 CharacterUnSealProcess(std::string & strAcountID, uint8 bSelectedIndex, uint64 nItemSerial);
	int8 CharacterSealItemCheck(std::string& strUserID);
	// Clan System
#pragma region Clan System Methods
	bool LoadAllKnights(KnightsArray &m_KnightsArray);
	int KnightsSave(uint16 sClanID, uint8 bFlag, uint32 nClanFund);
	void KnightsReset(uint16 sClanID);
	int8 CreateKnights(uint16 sClanID, uint8 bNation, std::string & strKnightsName, std::string & strChief, uint8 bFlag = 1);
	int KnightsMemberJoin(std::string & strCharID, uint16 sClanID);
	int KnightsMemberRemove(std::string & strCharID, uint16 sClanID);
	int KnightsMemberLeave(std::string & strCharID, uint16 sClanID);
	int KnightsMemberAdmit(std::string & strCharID, uint16 sClanID);
	int KnightsMemberReject(std::string & strCharID, uint16 sClanID);
	int KnightsMemberChief(std::string & strCharID, uint16 sClanID);
	int KnightsMemberViceChief(std::string & strCharID, uint16 sClanID);
	int KnightsMemberOfficer(std::string & strCharID, uint16 sClanID);
	int KnightsMemberPunish(std::string & strCharID, uint16 sClanID);
	int KnightsPointMethodChange(uint16 sClanID, uint8 bDomination);
	int KnightsSymbolUpdate(uint16 sClanID, uint16 sSymbolSize, char *clanSymbol);
	int KnightsCapeUpdate(uint16 sClanID, uint16 sCapeID, uint8 r, uint8 g, uint8 b);
	int KnightsCastellanCapeUpdate(uint16 sClanID, uint16 sCapeID, uint8 r, uint8 g, uint8 b, uint32 time);
	int KnightsHandover(std::string & strCharID, uint16 sClanID);
	int KnightsFundUpdate(uint16 sClanID, uint32 nClanPointFund);
	int KnightsGradeUpdate(uint16 sClanID, uint8 byFlag, uint16 sCapeID);
	int KnightsClanNoticeUpdate(uint16 sClanID, std::string & strClanNotice);
	int KnightsUserMemoUpdate(std::string strUsedID, std::string strMemo);
	int KnightsUserDonate(CUser * pUser, uint32 amountNP, uint32 UserAmountNP);
	int KnightsRefundNP(std::string & strUserID, uint32 nRefundNP);
	// Alliance related methods
	int KnightsAllianceCreate(uint16 sMainClanID, uint16 sSubClanID, uint8  byEmptyIndex); 
	int KnightsAllianceInsert(uint16 sMainClanID, uint16 sSubClanID, uint8  byEmptyIndex); 
	int KnightsAllianceDestroy(uint16 sAllianceID); 
	int KnightsAllianceRemove(uint16 sMainClanID, uint16 sSubClanID); 
	int KnightsAlliancePunish(uint16 sMainClanID, uint16 sSubClanID); 
	int KnightsAllianceNoticeUpdate(uint16 sClanID, std::string & strClanNotice);

	int KnightsDestroy(uint16 sClanID);
	uint16 LoadKnightsAllMembers(uint16 sClanID, Packet & result);
	int8 UpdateAlliance(uint8 byType, uint16 shAlliancIndex, uint16 shKnightsIndex, uint8  byEmptyIndex, uint8 bySiegeFlag); 
	void LoadKnightsAllList();
#pragma endregion
	
	void UpdateUserAuthority(std::string GameMaster, std::string strUserID, uint32 iPeriod, uint8 banTypes, std::string reason, CUser *pUser = nullptr);
	uint8 UpdateCharacterName(std::string & strAccountID, std::string & strUserID, std::string & strNewUserID);
	uint8 UpdateCharacterClanName(std::string strClanID, std::string strNewClanID, CUser* puser);
	ServerUniteSelectingCharNameErrorOpcode UpdateSelectingCharacterName(std::string & strAccountID, std::string & strUserID, std::string & strNewUserID);
	void UpdateBattleEvent(std::string & strCharID, uint8 bNation);
	void AccountLogout(std::string & strAccountID);

	void UpdateConCurrentUserCount(int nServerNo, int nZoneNo, int nCount);

	uint8 AccountInfoSave(std::string& AccountID, std::string UserID, std::string& email, std::string &phone, std::string &otp, std::string& seal);

	uint8 AccountInfoShow(std::string& AccountID);

	uint8 GetUnreadLetterCount(std::string & strCharID);
	bool GetLetterList(std::string & strCharID, Packet & result, bool bNewLettersOnly = true);
	int8 SendLetter(std::string & strSenderID, std::string & strRecipientID, std::string & strSubject, std::string & strMessage, uint8 bType, _ITEM_DATA * pItem, int32 nCoins);
	bool ReadLetter(std::string & strCharID, uint32 nLetterID, std::string & strMessage);
	int8 GetItemFromLetter(std::string & strCharID, uint32 nLetterID, uint32 & nItemID, uint16 & sCount, uint16 & sDurability, uint32 & nCoins, uint64 & nSerialNum, uint32 &nExpiTime); 
	void DeleteLetter(std::string & strCharID, uint32 nLetterID);	
	void UpdateElectionStatus(uint8 byType, uint8 byNation);
	void UpdateElectionList(uint8 byDBType, uint8 byType, uint8 byNation, uint16 sKnights, uint32 nAmount, std::string & strNominee);
	int16 UpdateElectionVoteList(uint8 byNation, std::string & strVoterAccountID, std::string & strVoterUserID, std::string & strNominee);
	void GetElectionResults(uint8 byNation);
	int16 UpdateCandidacyRecommend(std::string & strNominator, std::string & strNominee, uint8 byNation);
	void UpdateCandidacyNoticeBoard(std::string & strCharID, uint8 byNation, std::string & strNotice);

	void UpdateNoahOrExpEvent(uint8 byType, uint8 byNation, uint8 byAmount, uint8 byDay, uint8 byHour, uint8 byMinute, uint16 sDuration, uint32 nCoins);
	void InsertPrizeEvent(uint8 byType, uint8 byNation, uint32 nCoins, std::string & strCharID);
	void UpdateKingSystemDB(uint8 byNation, uint32 nNationalTreasury, uint32 KingTax);
	void InsertTaxEvent(uint8 opcode, uint8 KingNationTax, uint8 Nation, uint32 TerritoryTax = 0);
	void UpdateNationIntro(uint32 iServerNo, uint8 nation, std::string & strKingName, std::string &strKingNotice);
	void UpdateKingList(std::string& strCharID, uint8 Nation); // JstKO Auto King Name Update Eklendi 01.01.2025
	void LoadJstKOWelcomeLogosNotice(); // JstKO Welcome Logos Notice Eklendi 01.01.2025
	void UpdateVSEventLog(std::string WinNames, std::string LoseNames, uint32 NpPoint);
	void ResetLoyaltyMonthly();
	void UpdateSlaveCoins(std::string& strAccountID, uint32 Coins);
	void UpdateSlaveKC(std::string& strAccountID, uint32 KC);
	void RemoveSlaveCoins(std::string& strAccountID, uint32 Coins);
	void RemoveSlaveKC(std::string& strAccountID, uint32 KC);
	uint8 CheckSlave(std::string& strAccountID);
	void LoadSlave(std::string& strAccountID);
	void KingAddAndDelete(std::string nstrUserID, uint8 Type, uint32 CashPoint); //Write Gkhn
	bool GetLoadBotUser(CBot* pUser);
	bool UpdateBotUser(CBot* pUser);

	void SpecialStone();
	void LoadItemUpgradeSettings();
	void LoadUpgrade();
	void LoadCollectionReward(uint16 sEventID);
	void ClearRemainUsers();
	void InsertUserDailyOp(_USER_DAILY_OP * pUserDailyOp);
	void UpdateUserDailyOp(std::string strUserId, uint8 type, int32 sUnixTime);
	void UpdateRanks();

	void UpdateSiege(int16 m_sCastleIndex, int16 m_sMasterKnights, int16 m_bySiegeType, int16 m_byWarDay, int16 m_byWarTime, int16 m_byWarMinute);
	void UpdateSiegeTax(uint8 Zone, int16 ZoneTarrif);
	void UpdateSiegeWarfareDB(uint32 nMoradonTax, uint32 nDelosTax, uint32 nDungeonCharge);

	//clanbank
	bool UpdateClanWarehouseData(std::string strClanID, CUser * pser);
	// Clan bank //

	uint16 GetClanIDWithCharID(std::string & strCharID);
	void LoadLadderRankList(Packet & result, uint8 bNation, uint8& count);
	void LoadKnightsLeaderList(Packet & result, uint8 bNation, uint8& count);

	bool UpdateGenieData(std::string & strCharID, CUser *pUser);
	bool LoadGenieData(std::string & strCharID, CUser *pUser);
	bool LoadPriestBotGenieData(std::string& strCharID, CUser* pUser);
	int8 NationTransfer(std::string strAccountID);
	bool GetAllCharInfo(std::string & strCharID, uint16 & nNum, _NATION_DATA* m_NationTransferArray);
	bool GetAllCharInfo(std::string strCharID, _NATION_DATA& m_NationTransferArray);
	int8 NationTransferSaveUser(std::string strAccountID, std::string strCharID, uint16 nNum, uint8 bnewRace, uint16 bNewClass, uint8 bNewNation, uint8 bNewFace, uint32 iNewHair);
	
	int8 SetItemFromLetter(std::string& strCharID, uint32 nLetterID);
	// PPCard Serial DBAgent Methods
	uint8 LoadPPCard(std::string PPCardKeys, CUser *pUser);

	// Promotion Key Methods
	uint8 LoadPromotionKey(std::string& PPCardKeys, CUser* pUser);
	// BotMerchantData
	bool LoadBotHandlerMerchantTable();
	bool InsertBotMerchant(_BOT_SAVE_DATA* pData);

	void LoadPasswd(std::string AccountID, std::string & strPasswd);
	bool LoadAutoClanInfo(std::string &strCharID, uint16 &Class, uint8 &Level, uint16 &ClanID, uint8 &Fame, uint32 &Loyalty, uint32& monltyLoyalty);

	bool SaveCNTSKnights(uint16 ClanID, Nation newnation);
	bool SaveClanNationTransferUser(std::string strUserID, std::string strAccountID, uint16 ClanID, Nation newnation, uint8 newrace, uint8 newclass);

	//ACS
	bool LoadKnightCash(std::string & strAccountID, CUser * pUser);
	bool UpdateKnightCash(CUser * pUser);
	bool UpdateAccountKnightCash(CUser *pUser);
	bool UpdateReportUser(std::string strCharID, std::string ReportType, std::string SubJect, std::string Message);
	uint8 UpdateNtsCommand(std::string strAccountID, std::string strCharID, uint8 bnewRace, uint16 bNewClass, uint8 bNewNation);
	void InsertDatabaseLog(std::string sqlinfo);
	std::string getmypassword(std::string accountid);
	bool LoadTBUserData(std::string strAccountID, CUser* pUser);
	uint8 UpdateUserSealItem(uint64 nItemSerial, uint32 nItemID, uint8 bSealType, uint8 prelockstate, CUser* pUser);

	//Akara List
	uint8 LoadAkaraList(std::vector<AKARA_LIST>& List);
	uint8 UpdateAkaraList(uint16 Uid, uint32 ItemID, uint32 Price, uint8 sira, uint8 listeNum);
	uint8 UpdateUserAkaraHistory(uint8 listeNum, uint8 sira, std::vector<AKARA_LIST>& List, uint8 sonuc);
	uint8 LoadUserAkaraHistory(std::vector<ByteBuffer>& List, std::string& UserName);
	uint8 ClearAkaraList(uint8 sira);
	uint8 UpdateAkaraHistory(uint8 listeNum, uint8 sira, std::vector<AKARA_LIST>& List, uint8 sonuc);
	uint8 LoadAkaraHistory(std::vector<ByteBuffer>& List);

	~CDBAgent();

private:
	OdbcConnection *m_GameDB, *m_AccountDB, *m_LogDB;
	std::recursive_mutex m_lock;

	friend class CGameServerDlg;
};

extern CDBAgent g_DBAgent;