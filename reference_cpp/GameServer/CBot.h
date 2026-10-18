#pragma once
#include "Unit.h"
#include "User.h"

typedef	std::map<uint32, ULONGLONG>			BotSavedMagicMap;

class CBot : public Unit
{
public:
	~CBot() {}
	CBot();

	virtual void Initialize();

	short GetDamage(Unit* pTarget, _MAGIC_TABLE pSkill = _MAGIC_TABLE(), bool bPreviewOnly = false);
	bool isHostileTo(Unit* pTarget);
	bool isMoral2Checking(Unit* pTarget, _MAGIC_TABLE pSkill);
	void UserInOut(uint8 bType);
	void BotInOut(uint8 bType);
	void SendMoveResult(float fX, float fY, float fZ, uint8 echo /*= 0.0f*/, int16 speed, uint16 socketid);
	void GenieStart();
	void SendToRegion(Packet* pkt, CUser* pExceptUser = nullptr, uint16 nEventRoom = 0);
	void SendGenieStart(bool isToRegion = false);
	void BotsSurroundingUserRegionUpdate();
	void GetUserInfo(Packet& pkt);
	bool JobGroupCheck(short jobgroupid);
	void MoveProcess(float X, float Y, float Z, uint16 will_x, uint16 will_y, uint16 will_z, float sSpeed, uint8 echo);
	void MoveRegionProcess(float X, float Y, float Z, uint16 will_x, uint16 will_y, uint16 will_z, float sSpeed, uint8 echo);
	void MoveProcessGoDeahTown();
	void MoveProcessRonarkLandTown();
	void MoveProcessArdreamLandTown();
	void MoveProcessSpecialEventZindan();
	void MoveProcessSpecialMiniRonarkLand();
	void MoveProcessSpecialBaseWar();
	void MoveProcessSpecialMoradonWar(); 
	void MoveProcessSpecialSnowWar();
	void MoveProcessSpecialNightDelosWar();
	void MoveProcessSpecialNeoStarWar();
	void MoveProcessSpecialCastleWar();
	void MoveProcessSpecialMiniArdreamWar();
	void MoveProcessSpecialBifrost();
	void MoveProcessSpecialDelos();
	void WalkRegionCordinat(int16 m_sSocketID, float x, float y, float z, uint16 Delay = 0, bool isAttack = false);
	void WalkCordinat(float x, float y, float z, uint16 Delay = 0, bool isAttack = false);
	void SetBotAbility(bool bSendPacket = true);
	float SetCoefficient();
	void SetSlotItemValue();
	uint8 GetBaseClass();
	void ApplySetItemBonuses(_SET_ITEM* pItem);
	_ITEM_TABLE* GetItemPrototype(uint8 pos, _ITEM_DATA*& pItem);
	bool CheckSkillPoint(uint8 skillnum, uint8 min, uint8 max);
	void SetMaxHp(int iFlag = 0);
	void SetMaxMp();
	void SetMaxSp();
	void ShowEffect(uint32 nSkillID);
	void SpChange(int amount);
	void HpMpChange();
	bool	m_bSlaveMerchant;
	int16	m_bSlaveUserID;
	void RecastSavedMagic(uint8 buffType = 0);
	void Type4Change();
	void RegionFindAttackProcess();
	void RegionGetAssasinDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void RegionGetAssasinArrowDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void RegionGetAssasinDaggerDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void RegionGetWarriorDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void RegionGetFlameMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void RegionGetGlacierMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void RegionGetLightningMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void RegionGetPriestDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void SetRival(CBot* pRival);
	void SetRival(CUser* pRival);
	void RemoveRival();
	time_t  MagicPacket(uint8 opcode, uint32 nSkillID, int16 sCasterID, int16 sTargetID,
		int16 sData1 = 0, int16 sData2 = 0, int16 sData3 = 0);
	void BotMining();
	void BotFishing();
	void BotMerchant();
	void Regene(uint8 regene_type, uint32 magicid = 0);
	void FindMonsterAttackSlot();
	void FindMonsterAttack(int x, int z, C3DMap* pMap);
	void GetAssasinDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void GetAssasinArrowDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void GetAssasinDaggerDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void GetWarriorDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void GetFlameMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void GetGlacierMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void GetLightningMageDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void GetPriestDamageMagic(int16 tid, float X, float Y, float Z, int16 will_x, int16 will_y, int16 will_z, float sSpeed, int8 echo);
	void MerchantMoveProcess();
	void HPTimeChangeType3();
	void Type4Duration();
	void CheckSavedMagic();
	void SendUserStatusUpdate(UserStatus type, UserStatusBehaviour status);
	void SendToZone(Packet* result, float fRange = 0.0f);
	bool GetStartPosition(short& x, short& y, uint8 bZone = 0);
	bool GetStartPositionRandom(short& x, short& z, uint8 bZone = 0);
	void UpdateAngerGauge(uint8 byAngerGauge);
	void SendDeathNotice(Unit* pKiller, DeathNoticeType noticeType, bool isToZone = true);
	void AddBotRank(C3DMap* pMap);
	void StateChangeServerDirect(uint8 bType, uint32 nBuff);
	void InitOnDeath(Unit* pKiller);
	void OnDeathKilledPlayer(CUser* pKiller);
	void OnDeathKilledNpc(CNpc* pKiller);
	void OnDeathKilledBot(CBot* pKiller);
	void BotPartyInviteProcess();
	void FarmBotPartyFollowProcess();

	void SendLoyaltyChange(int32 nChangeAmount = 0, bool bIsKillReward = false, bool bIsBonusTime = false, bool bIsAddLoyaltyMonthly = true);
	void LoyaltyChange(int16 tid, uint16 bonusNP = 0);
	void LoyaltyDivide(int16 tid, uint16 bonusNP = 0);
	void LoyaltyBotChange(int16 tid, uint16 bonusNP = 0);
	void LoyaltyBotDivide(int16 tid, uint16 bonusNP = 0);
	uint8 GetPVPMonumentNation();
	void UpdatePlayerKillingRank();
	int16 GetLoyaltyDivideSource(uint8 totalmember = 0);
	int16 GetLoyaltyDivideTarget();
	void SetZoneAbilityChange(uint16 sNewZone);
	int8	m_bMerchantState;
	int16	m_sMerchantAreaType;

public:

	char	m_GenieOptions[100];
	
	
	std::string	m_strUserID;
	std::string MerchantChat;
	std::string ReplyChat;

	bool	m_bPremiumMerchant;
	bool	m_bInParty;
	bool	m_bPartyLeader;
	bool	m_bIsHidingHelmet;
	bool	m_bIsHidingCospre;
	bool	m_bIsHidingWings;
	bool	m_bIsChicken;
	bool    m_Reverse;
	bool	m_sRegionAttack;
	bool	m_TargetChanged;
	bool	m_bWeaponsDisabled;
	bool	m_bBlockPrivateChat;
	
	
	int8	m_bPersonalRank;
	int8	m_bKnightsRank;
	int8	m_bStatBuffs[STAT_COUNT];
	uint8	m_bPlayerAttackAmount;
	uint8	m_bRace;
	uint8	m_bFace;
	uint8   m_sFirstUsingGenie;
	uint8	m_bFame;
	uint8	m_reblvl;
	uint8   m_bstrSkill[10];
	uint8	m_bStats[STAT_COUNT];
	uint8	m_bResHpType;
	uint8 restyping;
	uint8	m_bRank;
	uint8	m_bInvisibilityType;
	uint8	m_bNeedParty;
	uint8	m_bAuthority;
	uint8	m_bGenieStatus;
	uint8   m_bWalkingState;
	uint8	s_MoveProcess;
	uint8	m_MoveState;
	uint8	WalkStep;
	uint8	echo;
	uint8	m_bAddWeaponDamage;
	uint8	m_bRebStats[STAT_COUNT];
	uint8	m_bMaxWeightAmount;
	uint8	m_byAPBonusAmount;
	uint8	m_bPctArmourAc;
	uint8	m_byAPClassBonusAmount[4];
	uint8	m_byAcClassBonusAmount[4];
	uint8	m_bItemExpGainAmount;
	uint8	m_bItemNPBonus;
	uint8	m_bItemNoahGainAmount;
	uint8	m_bSkillNPBonus;
	uint8	m_MaxSp;
	uint8	ReplyStatus;
	uint8	m_BotState;
	uint8	m_byAngerGauge;
	uint8	m_bNPGainAmount;
	uint8 GetSymbol();

	int16	m_bKnights;
	int16	m_sPoints;
	int16	m_sHp;
	int16	m_sMp;
	int16	m_sSp;
	int16	m_bMerchantViewer;
	int16	m_sDirection;
	int16	m_sTargetID;
	int16	m_sStatItemBonuses[STAT_COUNT];
	int16	m_sStatAchieveBonuses[ACHIEVE_STAT_COUNT];
	int16	m_sItemAc;
	uint16	m_GenieTime;
	int16	m_sItemHitrate;
	int16	m_sItemEvasionrate;
	int16	m_sItemMaxHp;
	int16	m_sItemMaxMp;
	int16	m_sRivalID;
	int16	m_sBind;
	int16	m_MaxHp;
	int16	m_MaxMp;
	int16 m_bUserPriestBotID;
	uint16	m_sClass;
	uint16	m_sSid;
	uint16	m_sAchieveCoverTitle;
	uint16  m_iLoyaltyPremiumBonus;
	uint16	m_sMaxWeightBonus;
	uint16	m_sAddArmourAc;
	uint16	m_PlayerKillingLoyaltyPremiumBonus;
	uint16	m_sPartyIndex;

	void GetMerchantSlipList(_MERCH_DATA list[MAX_MERCH_ITEMS], CBot* pownermerch);
	void MerchantSlipRefList(CBot* pM, bool merchcrea);
	_MERCH_DATA	m_arMerchantItems[MAX_MERCH_ITEMS]; //What is this person selling? Stored in "_MERCH_DATA" structure.

	int FindSlotForItem(uint32 nItemID, uint16 sCount = 1);
	int GetEmptySlot();

	uint32	m_nHair, m_bMerchantIndex;
	uint32	m_iGold;
	uint32	m_iLoyalty;
	uint32	m_iLoyaltyMonthly;
	uint32  m_iLoyaltyDaily;
	uint32	m_bAbnormalType;
	uint32	m_nOldAbnormalType, ReturnSymbolisOK;
	uint32	m_sMaxWeight;
	uint32	m_sItemWeight;
	uint32	ReplyID;
	uint32	m_maxStop;
	uint32	m_sSkillCoolDown;
	uint32	m_PlayerKillingLoyaltyDaily;

	DWORD	ReplyTime;

	time_t	LastWarpTime;
	time_t	m_tPartyInviteTime;
	time_t	m_tGameStartTimeSavedMagic;
	ULONGLONG	m_sMoveRegionTime;
	ULONGLONG	m_sMoveTime;
	ULONGLONG	m_sCswAttackTime;
	time_t	m_fHPChangeTime;
	time_t	m_tRivalExpiryTime;
	time_t	m_fType4ChangeTime;
	time_t	LastMiningCheck;
	time_t	m_sMoveMerchantProcess;
	time_t	m_fDelayTime;
	time_t	m_fHPType4CheckTime;

	float	m_oldx;
	float	m_oldy;
	float	m_oldz;
	float	m_sSpeed;

	ULONGLONG  m_sRegionAttackTime;
	ULONGLONG  m_sMoveRegionAttackTime;

	_ITEM_DATA m_sItemArray[INVENTORY_TOTAL];
	time_t  m_for_bottime;
	
	_ITEM_DATA m_sItemKnightRoyalArray[INVENTORY_TOTAL];

	GameState	m_state;
	TeamColour	m_teamColour;
	EquippedItemBonuses m_equippedItemBonuses;
	BotSavedMagicMap m_savedMagicMap;

	std::recursive_mutex m_equippedItemBonusLock;
	std::recursive_mutex m_savedMagicLock;;

	virtual uint16 GetID() { return m_sSid; }
	virtual std::string& GetName() { return m_strUserID; }
	virtual int32 GetHealth() { return m_sHp; }
	virtual int32 GetMaxHealth() { return m_MaxHp; }
	virtual int32 GetMana() { return m_sMp; }
	virtual int32 GetMaxMana() { return m_MaxMp; }
	virtual bool isDead() { return m_bResHpType == USER_DEAD || m_sHp <= 0; }
	virtual void GetInOut(Packet& result, uint8 bType);
	virtual void AddToRegion(int16 new_region_x, int16 new_region_z);
	virtual void HpChange(int amount, Unit* pAttacker = nullptr, bool isDOT = false);
	virtual void MSpChange(int amount);
	virtual void OnDeath(Unit* pKiller);

	INLINE uint8 GetRebLevel() { return m_reblvl; }
	INLINE uint16 GetCoverTitle() { return m_sAchieveCoverTitle; }
	INLINE bool isMerchanting() { return (GetMerchantState() != MERCHANT_STATE_NONE); }
	INLINE bool isSellingMerchant() { return GetMerchantState() == MERCHANT_STATE_SELLING; }
	INLINE bool isBuyingMerchant() { return GetMerchantState() == MERCHANT_STATE_BUYING; }
	INLINE int8 GetMerchantState() { return m_bMerchantState; }
	INLINE bool isInGame() { return GetState() == GameState::GAME_STATE_INGAME; }
	INLINE GameState GetState() { return m_state; }
	INLINE bool isInClan() { return GetClanID() > 0; }
	INLINE int16 GetClanID() { return m_bKnights; }
	INLINE uint8 GetFame() { return m_bFame; }
	INLINE bool isKing() { return m_bRank == 1; }
	INLINE bool isWarrior() { return JobGroupCheck(ClassWarrior); }
	INLINE bool isRogue() { return JobGroupCheck(ClassRogue); }
	INLINE bool isMage() { return JobGroupCheck(ClassMage); }
	INLINE bool isPriest() { return JobGroupCheck(ClassPriest); }
	INLINE bool isPortuKurian() { return JobGroupCheck(ClassPortuKurian); }
	INLINE bool isBeginnerWarrior() { return GetClassType() == ClassWarrior; }
	INLINE bool isBeginnerRogue() { return GetClassType() == ClassRogue; }
	INLINE bool isBeginnerMage() { return GetClassType() == ClassMage; }
	INLINE bool isBeginnerPriest() { return GetClassType() == ClassPriest; }
	INLINE bool isBeginnerKurianPortu() { return GetClassType() == ClassPortuKurian; }
	INLINE bool isNoviceWarrior() { return GetClassType() == ClassWarriorNovice; }
	INLINE bool isNoviceRogue() { return GetClassType() == ClassRogueNovice; }
	INLINE bool isNoviceMage() { return GetClassType() == ClassMageNovice; }
	INLINE bool isNovicePriest() { return GetClassType() == ClassPriestNovice; }
	INLINE bool isNoviceKurianPortu() { return GetClassType() == ClassPortuKurianNovice; }
	INLINE bool isMasteredWarrior() { return GetClassType() == ClassWarriorMaster; }
	INLINE bool isMasteredRogue() { return GetClassType() == ClassRogueMaster; }
	INLINE bool isMasteredMage() { return GetClassType() == ClassMageMaster; }
	INLINE bool isMasteredPriest() { return GetClassType() == ClassPriestMaster; }
	INLINE bool isMasteredKurianPortu() { return GetClassType() == ClassPortuKurianMaster; }
	INLINE uint16 GetClass() { return m_sClass; }
	INLINE uint8 GetRace() { return m_bRace; }
	INLINE uint32 GetLoyalty() { return m_iLoyalty; }
	INLINE uint32 GetMonthlyLoyalty() { return m_iLoyaltyMonthly; }
	INLINE uint32 GetCoins() { return m_iGold; }
	INLINE uint8 GetRebirthLevel() { return m_reblvl; }
	INLINE bool isRegionTargetUp() { return m_sRegionAttack; }
	INLINE bool isWeaponsDisabled() { return m_bWeaponsDisabled; }
	INLINE bool hasRival() { return GetRivalID() >= 0; }
	INLINE int16 GetRivalID() { return m_sRivalID; }
	INLINE bool hasRivalryExpired() { return UNIXTIME >= m_tRivalExpiryTime; }
	INLINE uint8 GetBotState() { return m_BotState; }
	INLINE uint8 GetAngerGauge() { return m_byAngerGauge; }
	INLINE bool hasFullAngerGauge() { return GetAngerGauge() >= MAX_ANGER_GAUGE; }
	INLINE uint16 GetPartyID() { return m_sPartyIndex; }
	INLINE bool isInParty() { return m_bInParty && m_sPartyIndex > 0; }
	INLINE bool isPartyLeader() { return isInParty() && m_bPartyLeader; }
	INLINE uint8 GetAuthority() { return m_bAuthority; }
	INLINE bool isGM() { return GetAuthority() == (uint8)AuthorityTypes::AUTHORITY_GAME_MASTER; }
	INLINE bool isGMUser() { return GetAuthority() == (uint8)AuthorityTypes::AUTHORITY_GM_USER; }
	INLINE bool isSlaveMerchant() { return m_bSlaveMerchant; }
	INLINE int16 GetSlaveGetID() { return m_bSlaveUserID; }

	INLINE ClassType GetBaseClassType()
	{
		static const ClassType classTypes[] =
		{
			ClassWarrior, ClassRogue, ClassMage, ClassPriest,
			ClassWarriorNovice, ClassWarriorMaster,		// job changed / mastered
			ClassRogueNovice, ClassRogueMaster,			// job changed / mastered
			ClassMageNovice, ClassMageMaster,			// job changed / mastered
			ClassPriestNovice, ClassPriestMaster,		// job changed / mastered
			ClassPortuKurian,ClassPortuKurianNovice,	// job changed / mastered 
			ClassPortuKurianMaster,						// job changed / mastered
		};

		uint8 classType = GetClassType();

		if (classType < 1 && classType > 19)
			return ClassType(0);

		return classTypes[classType - 1];
	}
	INLINE uint8 GetClassType()
	{
		return GetClass() % 100;
	}
	INLINE _ITEM_DATA* GetItem(uint8 pos)
	{
		if (pos > INVENTORY_TOTAL)
			return false;

		ASSERT(pos < INVENTORY_TOTAL);
		return &m_sItemArray[pos];
	}
	INLINE bool isInPKZone() {
		return GetZoneID() == ZONE_ARDREAM
			|| GetZoneID() == ZONE_RONARK_LAND
			|| GetZoneID() == ZONE_RONARK_LAND_BASE
			|| isInSpecialEventZone()
			|| isInPKBotZone();
	}
	INLINE bool isInPKBotZone() {
		return GetZoneID() == ZONE_BIFROST
			|| GetZoneID() == ZONE_DELOS;
	}
	INLINE uint8 GetStat(StatType type)
	{
		if (type >= StatType::STAT_COUNT)
			return 0;

		return m_bStats[(uint8)type];
	}
	INLINE _ITEM_TABLE* GetItemPrototype(uint8 pos)
	{
		_ITEM_DATA* pItem;
		ASSERT(pos < INVENTORY_TOTAL);
		return GetItemPrototype(pos, pItem);
	}
	INLINE _ITEM_DATA* GetKnightRoyaleItem(uint8 pos)
	{
		if (pos > INVENTORY_TOTAL)
			return false;

		ASSERT(pos < INVENTORY_TOTAL);
		return &m_sItemKnightRoyalArray[pos];

	}
	INLINE int16 GetStatBonusTotal(StatType type)
	{
		return GetStatBuff(type)
			+ GetRebStatBuff(type)
			+ GetStatItemBonus(type)
			+ GetStatAchieveBonus((UserAchieveStatTypes)type);
	}
	INLINE int8 GetStatBuff(StatType type)
	{
		ASSERT(type < STAT_COUNT);
		return m_bStatBuffs[type];
	}
	INLINE uint8 GetRebStatBuff(StatType type)
	{
		ASSERT(type < STAT_COUNT);
		return m_bRebStats[type];
	}
	INLINE int16 GetStatItemBonus(StatType type)
	{
		ASSERT(type < STAT_COUNT);
		return m_sStatItemBonuses[type];
	}
	INLINE int16 GetStatAchieveBonus(UserAchieveStatTypes type)
	{
		ASSERT(type < ACHIEVE_STAT_COUNT);
		return m_sStatAchieveBonuses[type];
	}
	INLINE int16 GetStatWithItemBonus(StatType type)
	{
		return GetStat(type) + GetStatItemBonus(type);
	}
	INLINE int16 GetStatTotal(StatType type)
	{
		return GetStat(type) + GetStatBonusTotal(type);
	}
	INLINE bool isMastered()
	{
		uint16 sClass = GetClassType();
		return (sClass == ClassWarriorMaster
			|| sClass == ClassRogueMaster
			|| sClass == ClassMageMaster
			|| sClass == ClassPriestMaster
			|| sClass == ClassPortuKurianMaster);
	}
	INLINE bool isLockableScroll(uint8 buffType)
	{
		return (buffType == BUFF_TYPE_HP_MP
			|| buffType == BUFF_TYPE_AC
			|| buffType == BUFF_TYPE_FISHING
			|| buffType == BUFF_TYPE_DAMAGE
			|| buffType == BUFF_TYPE_SPEED
			|| buffType == BUFF_TYPE_STATS
			|| buffType == BUFF_TYPE_BATTLE_CRY);
	}
	INLINE void isReset(bool isOndeath)
	{
		if (isOndeath == false)
		{
			if (GetZoneID() == ZONE_SPBATTLE1)
				s_MoveProcess = myrand(1, 10);
			else
				s_MoveProcess = myrand(1, 10);

			m_MoveState = 1;
			m_sCswAttackTime = UNIXTIME2 + 30000;
		}

		m_TargetChanged = /*m_sRegionAttack = */false;
		m_sTargetID = -1;
		m_oldx = m_oldz = m_oldy = float(0.0f);
	};
};

