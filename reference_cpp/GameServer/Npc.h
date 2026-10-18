#pragma once

#include "LuaEngine.h"
#include "Unit.h"
#include "PathFind.h"
#include "NpcTable.h"

class CNpc : public Unit
{
public:
	virtual uint16 GetID() { return m_sNid; }
	virtual std::string & GetName() { return m_strName; }
	std::string & GetNameUser() { return m_strUserName; }
	std::string & GetNamePet() { return m_strPetName; }
	bool	m_isNpcBeingUsed;
	bool	m_bForceReset;
	ULONGLONG m_MykoSoftDelay;
	short	m_sSize;				// MONSTER(NPC) Size
	int		m_iWeapon_1;
	int		m_iWeapon_2;
	std::string	m_strName;				// MONSTER(NPC) Name
	std::string	m_strUserName;
	std::string	m_strPetName;
	bool	isReturningSpawnSlot;
	int		m_iSellingGroup;		// ItemGroup
	uint16	m_sPid;
	Unit*	pKiller;
	Unit*	pAttackerReq;
	int     m_iHPChangeAmount;
	bool    m_bisDOT;
	bool	m_bSendDeathPacket;
	uint16  m_sDuration;
	int16	m_sPetId, m_sGuardID;
	int32   m_iSpawnedTime;
	bool	m_isTowerOwner;
public:
	//---------------------------------------------------------------
	//	PathFind Info
	//---------------------------------------------------------------
	short		m_min_x;
	short		m_min_y;
	short		m_max_x;
	short		m_max_y;

	typedef struct { long cx; long cy; } Size;
	Size	m_vMapSize;

	float m_fStartPoint_X, m_fStartPoint_Y;
	float m_fEndPoint_X, m_fEndPoint_Y;

	short m_sStepCount;

	CPathFind m_vPathFind;
	_PathNode	*m_pPath;

	int		m_nInitMinX;	// Initial min X position when spawning
	int		m_nInitMinY;	// Initial min Y(Z) position when spawning
	int		m_nInitMaxX;	// Initial max X position when spawning
	int		m_nInitMaxY;	// Initial max Y(Z) position when spawning

	time_t	m_fHPChangeTime; // last time that NPC/MONSTER gained hp from resting.
	time_t	m_fHPType4CheckTime; // last time that NPC/MONSTER Type4Checked
	time_t	m_fHPType3CheckTime; // last time that NPC/MONSTER Type4Checked
	time_t	m_tFaintingTime; // fainting! wtf?
	time_t	m_tSlowedTime;	 // the time NPC/MONSTER is slowed. This needs a better implementation. A skill over Unit class.
	int16	m_WoodOwnerID;

	bool	m_msummon, m_msummonsavepos;
public:
	_Target	m_Target;			// 
	short	m_ItemUserLevel;	//

	std::map<short, __NpcDamagedList> m_DamagedUserList;
	std::recursive_mutex m_damageuserListLock, m_damagenpcListLock;
	std::map<short, __NpcDamagedList> m_DamagedNpcList;

	int		m_TotalDamage;
	short   m_sMaxDamageUserid;	// the user who has damaged the most to that NPC/MOB
	std::string strMaxDamageUser;

	_PathList m_PathList;		// Npc's path list
	_PattenPos m_pPatternPos;	// Npc's previous pattern position

	short	m_iPatternFrame;		// the pattern frame

	uint8	m_byMoveType;		// NPC's Move Type
	uint8	m_byInitMoveType;	// NPC's Initial Move Type
	short	m_sPathCount;		// NPC's PathList Count
	short	m_sMaxPathCount;	// NPC's PathList Max Count

	bool	m_bFirstLive;		// NPC's first live denoting flag.

	_NPC_BOSS_SET pNpcBossInfo;
private:
	uint8	m_OldNpcState, m_NpcState; // NPC's old and current states

public:
	INLINE uint8 GetNpcState() { return m_NpcState; }
	INLINE uint8 GetNpcOldState() { return m_OldNpcState; }

	INLINE void SetNpcState(uint8 newState) { m_NpcState = newState; }
	INLINE void SetNpcOldState(uint8 oldState) { m_OldNpcState = oldState; }

	INLINE int16 GetWoodUserID() { return m_WoodOwnerID; }
	uint16	m_sNid;				// NPC ID

	INLINE bool isCswDoors()
	{
		return (GetProtoID() == 561
			|| GetProtoID() == 562
			|| GetProtoID() == 563)
			&& GetType() == NPC_GATE;
	}

	float	m_nInitX;		// NPC's Initial Position X
	float	m_nInitY;		// NPC's Initial Position Y
	float	m_nInitZ;		// NPC's Initial Position Z

	float	m_fPrevX;		// Prev X Pos;
	float	m_fPrevY;		// Prev Y Pos;
	float	m_fPrevZ;		// Prev Z Pos;

	uint8	m_byActType;		// the action type of the NPC/MONSTER

	uint8	m_byRank;			// the rank
	uint8	m_byTitle;			// the title

	uint16	m_sAttack;			// 
	uint16	m_sAttackDelay;		// the time that a monster awaits till next attack
	uint16	m_sSpeed;			// monster speed

	uint16	m_sUtcSecond;		// UnderTheCastle Skill Attack.

	float   m_fSpeed_1;			// moving speed of Monster (random move)
	float   m_fSpeed_2;			// moving speed of monster when either attacking or tracing (assuming no slow is taken)

	float   m_fSecForMetor;		// 

	uint16	m_sStandTime;		// the stand time

	uint8	m_bySearchRange;	// the search range of the NPC/MOB for enemies
	uint8	m_byAttackRange;	// the attack range of the NPC/MOB for enemies
	uint8	m_byTracingRange;	// the tracing range of the NPC/MOB for enemies

	int		m_iMoney;			// the amount of coins to be dropped when owned
	int		m_iItem;			// the drop index of monster/npc (K_MONSTER_ITEM table sIndex)

	CNpcTable *m_proto;
public:
	//----------------------------------------------------------------
	//	NPC Path Related Parameters
	//----------------------------------------------------------------
	_NpcPosition	m_pPoint[MAX_PATH_LINE]; // 

	short	m_iAniFrameIndex;
	short	m_iAniFrameCount;
	uint8	m_byPathCount;				// ÆĞ½º¸¦ µû¶ó ÀÌµ¿ÇÏ´Â ¸ó½ºÅÍ ³¢¸® °ãÄ¡Áö ¾Êµµ·Ï,, 
	bool	m_bStopFollowingTarget;		// when set, indicates that an NPC should stop following its target
	uint8	m_byActionFlag;				// Çàµ¿º¯È­ ÇÃ·¡±× ( 0 : Çàµ¿º¯È­ ¾øÀ½, 1 : °ø°İ¿¡¼­ Ãß°İ)

	bool	m_bTracing;
	float	m_fTracingStartX, m_fTracingStartZ;

	short	m_iFind_X[4];		// find enemy¿¡¼­ Ã£À» Region°Ë»ç¿µ¿ª
	short	m_iFind_Y[4];

	float   m_fOldSpeed_1;		// placeholder for the NPC/MOBs initial speeds
	float   m_fOldSpeed_2;		// placeholder for the NPC/MOBs initial speeds

	bool	m_bMonster; // are we a monster or an NPC?

	uint32	m_nActiveSkillID;	// ID of skill currently being cast
	int16	m_sActiveTargetID;	// ID of the target of the skill currently being cast
	uint16	m_sActiveCastTime;	// Cast time of the skill currently being cast (in seconds)
	int16   m_sSkillX;
	int16   m_sSkillY;
	int16   m_sSkillZ;

	bool	m_bDelete;			// when set, will remove the NPC from the server after execution.
	bool	m_bDeleteRequested;
	uint32  m_tDeleteTime;

public:
	//----------------------------------------------------------------
	//	MONSTER AI 
	//----------------------------------------------------------------
	uint8	m_tNpcAttType;		// Passive[TENDER_ATTACK_TYPE = 0], Aggressive [ATROCITY_ATTACK_TYPE = 1] 

	uint8	m_byAttackPos;		// User
	uint8	m_byBattlePos;		// 
	uint8	m_byMaxDamagedNation;	// the nation of maximum damaged user's nation (1:KARUS, 2:ELMORAD)

	bool	m_bHasFriends;		// When set, monsters behave in groups (defined by their family type) and will seek out help from nearby similar mobs.
	uint8	m_byGateOpen;		// Gate Npc Status -> 1,2 : open 0 : close
	uint8	m_byObjectType;     // The Object Type . NORMAL_OBJECT = 0, SPECIAL_OBJECT =  1
	uint8	m_byDungeonFamily;		// ´øÁ¯¿¡¼­ °°Àº ÆĞ¹Ğ¸® ¹­À½ (°°Àº ¹æ)
	NpcSpecialType	m_bySpecialType;
	// 1:
	// 2:
	// 3:
	// 4:(m_sControlSid)
	// 5:
	// 6:
	// 100:

	uint8	m_byTrapNumber;	// ´øÁ¯¿¡¼­ Æ®·¦ÀÇ ¹øÈ£,,
	uint8	m_byChangeType;	// 0:Á¤»ó»óÅÂ, 1:º¯ÇÏ±â À§ÇÑ ÁØºñ, 2:´Ù¸¥¸ó½ºÅÍ·Î º¯ÇÔ, 3:¸ó½ºÅÍÀÇ ÃâÇö, 100:¸ó½ºÅÍÀÇ Á×À½
	uint8	m_byRegenType;	// 0:Á¤»óÀûÀ¸·Î ¸®Á¨ÀÌ µÊ.. , 1:ÇÑ¹ø Á×À¸¸é ¸®Á¨ÀÌ ¾ÈµÇ´Â Æ¯¼ö ¸ö, 2:¸®Á¨ÀÌ ¾ÈµÊ
	uint8   m_byDeadType;	// 0:self death(kinda suicide), 100: another killed
	int WoodCooldownClose;

							//----------------------------------------------------------------
							//	MONSTER_POS DB AI Server Related
							//----------------------------------------------------------------
	time_t	m_Delay;			// this doesn't really need to be time_t, but we'll use it (at least for now) for consistency
	time_t	m_fDelayTime;		// Npc Thread waits until delay time overs.

	uint8	m_byType;
	int		m_sRegenTime;		// NPC regeneration period of time

	uint8	m_byDirection;

	int		m_nLimitMinX;		// È°µ¿ ¿µ¿ª
	int		m_nLimitMinZ;
	int		m_nLimitMaxX;
	int		m_nLimitMaxZ;

	bool	m_bIsEventNpc;

	float	m_fAdd_x;
	float	m_fAdd_z;

	float	m_fBattlePos_x;
	float	m_fBattlePos_z;

	float	m_fSecForRealMoveMetor;		// ÃÊ´ç °¥ ¼ö ÀÖ´Â °Å¸®..(½ÇÁ¦ Å¬¶óÀÌ¾ğÆ®¿¡ º¸³»ÁÖ´Â °Å¸®)

	bool	m_bPathFlag;		// the path flag if path found
public:
	CNpc();

	void	Load(uint16 sNpcID, CNpcTable * proto, bool bMonster, uint8 nation = 0);
	void	SendMoveResult(float fX, float fY, float fZ, float fSpeed = 0.0f);
	void	SendExpToUserList();
	bool	isShowBox();
	void	GiveNpcHaveItem(Unit *pKiller);
	void    DropTesterHaveItem(CUser * pTester, uint16 Count);


	void	InitPos();
	void	Init();
	void	InitTarget();

	void	Initialize();
	void	AddToRegion(int16 new_region_x, int16 new_region_z);

	void	GetInOut(Packet & result, uint8 bType);
	void	SendInOut(uint8 bType, float fx, float fz, float fy);
	void	GetNpcInfo(Packet & pkt, uint16 userClanID = 0);

	void	SendGateFlag(uint8 bFlag = -1);
	void	SendJuraidBridgeFlag(uint8 bFlag = -1);

	void	HpChange(int amount, Unit *pAttacker = nullptr, bool isDOT = false);
	time_t	HpChangeReq();
	void	RecvPetExp(CNpc* pNpc, int32 iDamage, int iTotalDamage, int iNpcExp, int iNpcLoyalty);
	void	HpChangeMagic(int amount, Unit *pAttacker = nullptr, AttributeType attributeType = AttributeType::AttributeNone);
	void	MSpChange(int amount);
	virtual void StateChangeServerDirect(uint8 bType, uint32 nBuff);
	bool	CastSkill(Unit * pTarget, uint32 nSkillID);

	uint8 isInSoccerEvent();

	void	OnDeath(Unit *pKiller);
	void	OnDeathProcess(Unit *pKiller);
	void	ChaosStoneDeath(CUser *pUser);
	void	SpecialStoneDeath(CUser* pUser);
	void	SantaDeath(CUser *pUser);
	void	ChaosStoneBossKilledBy();
	uint8	ChaosStoneSelectStage(uint8 sRank);
	void    ChaosStoneDeathRespawnMonster(uint8 ChaosGetIndex);
	void	PVPMonumentProcess(CUser *pUser);
	void	BattleMonumentProcess(CUser *pUser);
	void	KarusNationMonumentProcess(CUser *pUser);
	void	HumanNationMonumentProcess(CUser *pUser);
	void	MonsterStoneKillProcess(CUser *pUser);
	void	UnderTheCastleProcess(CUser *pUser);
	void    DungeonDefenceProcess(CUser *pUser);
	void	OnRespawn();

	void	csw_momumentprocess(CUser *pKiller);

	void BifrostMonumentProcess(CUser* pUser);
	void TournamentMonumentKillProcess(CUser* puser);
	void ForgettenTempleMonsterDead();
	//bdw Monument
	void	BDWMonumentAltarSystem(CUser *pUser);

protected:
	void	ClearPathFindData(void);

public:
	void Dead(Unit* pKiller = nullptr, bool open_bundle = true);
	void	NpcStrategy(uint8 type);
	int		FindFriend(MonSearchType type = MonSearchType::MonSearchSameFamily);
	void	FindFriendRegion(int x, int z, C3DMap* pMap, _TargetHealer* pHealer, MonSearchType type = MonSearchType::MonSearchSameFamily);
	bool	IsCloseTarget(CUser *pUser, int nRange);
	void	ChangeTarget(int nAttackType, CUser *pUser);
	void	ChangeNTarget(CNpc *pNpc);
	void	RecvAttackReq(int nDamage, uint16 sAttackerID, AttributeType attributeType = AttributeType::AttributeNone);
	bool	ResetPath();
	bool	GetTargetPos(float& x, float& z);
	bool	IsChangePath();
	void    SendAttackRequest(int16 tid);
	void	TracingAttack();
	int		GetTargetPath(int option = 0);
	CloseTargetResult IsCloseTarget(int nRange, AttackType attackType);
	bool	StepMove();
	bool	StepNoPathMove();
	bool	IsMovingEnd();
	int		IsSurround(CUser* pUser);
	bool	IsDamagedUserList(CUser *pUser);
	bool	IsPathFindCheck(float fDistance);
	void	IsNoPathFind(float fDistance);

	void HandleJuraidKill(CUser* pKiller);
	void HandleJuraidGateOpen(uint16 pBrigdes, uint8 door, _JURAID_ROOM_INFO* proominfo, Nation bNation);
public:
	time_t	Attack();
	time_t	LongAndMagicAttack();
	time_t	NpcLive();
	time_t	NpcTracing();
	time_t	NpcAttacking();
	time_t	NpcMoving();
	time_t	NpcSleeping();
	time_t	NpcFainting();
	time_t	NpcHealing();
	time_t	NpcCasting();
	time_t	NpcStanding();
	time_t	NpcBack();
	time_t  NpcDead();
	bool	SetLive();

public:
	bool	isInSpawnRange(int nX, int nZ);
	bool	RandomMove();
	bool	RandomBackMove();
	bool	IsInPathRange();
	int		GetNearPathPoint();
	bool	FindEnemy();
	bool	CheckFindEnemy();
	int		FindEnemyRegion();
	float	FindEnemyExpand(int nRX, int nRZ, float fCompDis, UnitType unitType);
	int		GetMyField();
	void	InitUserList();

	bool	isInMeleeArena();
	bool	isInPartyArena();

	int		GetDir(float x1, float z1, float x2, float z2);
	void	NpcMoveEnd();

	void	GetVectorPosition(__Vector3 & vOrig, __Vector3 & vDest, float fDis, __Vector3 * vResult);
	void	CalcAdaptivePosition(__Vector3 & vPosOrig, __Vector3 & vPosDest, float fAttackDistance, __Vector3 * vResult);
	void	ComputeDestPos(__Vector3 & vCur, float fDegree, float fDistance, __Vector3 * vResult);
	void	Yaw2D(float fDirX, float fDirZ, float& fYawResult);
	float	GetDistance(__Vector3 & vOrig, __Vector3 & vDest);
	int		PathFind(CPoint start, CPoint end, float fDistance);
	bool	GetUserInView();
	bool	GetUserInViewRange(int x, int z);
	void	MoveAttack();
	int		ItemProdution(int item_number);
	int		GetItemGrade(int item_grade);
	int		GetItemCodeNumber(int level, int item_type);
	int		GetWeaponItemCodeNumber(bool bWeapon);
	int		GetPartyExp(int party_level, int man, int nNpcExp);
	void	ChangeAbility(int iChangeType);
	void	HpChange();
	//bool	Teleport();
	void	HPTimeChangeType3();
	void	Type4Duration();
	time_t  MagicPacket(uint8 opcode, uint32 nSkillID, int16 sCasterID, int16 sTargetID,
		int16 sData1 = 0, int16 sData2 = 0, int16 sData3 = 0);

	bool	RegisterRegionNpc(float x, float z);
	void	NoticeRespawn(std::string & Monster); // 29.10.2020 Boss Atınca Notice Gecmesi Engellendi

	e_summontype e_stype;
public:

	INLINE bool isHealer() { return GetType() == NPC_HEALER; }

	INLINE bool isGuard()
	{
		return GetType() == NPC_GUARD || GetType() == NPC_PATROL_GUARD || GetType() == NPC_STORE_GUARD;
	}

	INLINE bool isGuardSummon()
	{
		return GetProtoID() == GUARD_SUMMON;
	}

	INLINE bool isGuardTower()
	{
		return GetType() == NPC_GUARD_TOWER1 || GetType() == NPC_GUARD_TOWER2;
	}

	INLINE bool isArtifact()
	{
		return GetType() == NPC_ARTIFACT
			|| GetType() == NPC_DESTROYED_ARTIFACT
			|| GetType() == NPC_ARTIFACT1
			|| GetType() == NPC_ARTIFACT2
			|| GetType() == NPC_ARTIFACT3
			|| GetType() == NPC_ARTIFACT4;
	}

	INLINE bool hasTarget() { return m_Target.bSet; }
	INLINE bool isDead() { return GetNpcState() == (uint8)NpcState::NPC_DEAD || m_iHP <= 0 || m_bDelete == true; }
	INLINE bool isAlive() { return !isDead(); }
	INLINE bool isMonster() { return m_bMonster; }

	INLINE bool isNonAttackingObject()
	{
		return isGate() ||
			GetType() == NPC_GATE_LEVER ||
			isArtifact() ||
			GetType() == NPC_SCARECROW ||
			GetType() == NPC_TREE ||
			(GetNation() == (uint8)Nation::NONE &&
			GetType() == NPC_PARTNER_TYPE);
	}

	INLINE bool isNonAttackableObject()
	{
		return isGate() ||
			GetType() == NPC_GATE_LEVER ||
			(GetNation() == (uint8)Nation::NONE &&
			GetType() == NPC_PARTNER_TYPE);
	}

	INLINE bool isGate()
	{
		return GetType() == NPC_GATE
			|| GetType() == NPC_GATE2
			|| GetType() == NPC_PHOENIX_GATE
			|| GetType() == NPC_SPECIAL_GATE
			|| GetType() == NPC_VICTORY_GATE
			|| GetType() == NPC_GATE_LEVER
			|| GetType() == NPC_KARUS_MONUMENT
			|| GetType() == NPC_HUMAN_MONUMENT
			|| GetType() == NPC_OBJECT_WOOD
			|| GetType() == NPC_KROWAZ_GATE;
	}

	INLINE bool isPet() { return GetType() == NPC_OBJECT_FLAG; }
	INLINE bool isGateOpen() { return(m_byGateOpen == 1 || m_byGateOpen == 2); }
	INLINE bool isGateClosed() { return !isGateOpen(); }
	INLINE short GetProtoID() { return GetProto()->m_sSid; }
	INLINE short GetPetID() { return m_sPetId; }
	INLINE short GetGuardID() { return m_sGuardID; }
	INLINE CNpcTable* GetProto() { return m_proto; }
	INLINE uint8 GetType()
	{
		if (GetProto() == nullptr)
			return 0;

		return GetProto()->m_tNpcType;
	}

	INLINE int8 GetByGruop()
	{
		if (GetProto() == nullptr)
			return -1;

		return GetProto()->m_byGroup;
	};
	

	INLINE uint8 GetState() { return m_NpcState; }
	INLINE uint16 GetPID() { return m_sPid; }
	
	void StateChange(uint8 bState);
	virtual int32 GetHealth() { return m_iHP; }
	virtual int32 GetMaxHealth() { return m_MaxHP; }
	virtual int32 GetMana() { return m_sMP; }
	virtual int32 GetMaxMana() { return m_MaxMP; }
	bool isHostileTo(Unit * pTarget);
	short GetDamage(Unit *pTarget, _MAGIC_TABLE pSkill = _MAGIC_TABLE(), bool bPreviewOnly = false);
	short GetDamage(CUser *pTarget, _MAGIC_TABLE pSkill = _MAGIC_TABLE(), bool bPreviewOnly = false);
	short GetDamage(CNpc *pTarget, _MAGIC_TABLE pSkill = _MAGIC_TABLE(), bool bPreviewOnly = false);
	float GetRewardModifier(uint8 byLevel);
	float GetPartyRewardModifier(uint32 nPartyLevel, uint32 nPartyMembers);
	virtual ~CNpc();

	bool isMoral2Checking(Unit* pTarget, _MAGIC_TABLE pSkill);

	// NPC State Handlers
	DECLARE_LUA_CLASS(CNpc);
	DECLARE_LUA_GETTER(GetID)
	DECLARE_LUA_GETTER(GetProtoID)
	DECLARE_LUA_GETTER(GetName)
	DECLARE_LUA_GETTER(GetNation)
	DECLARE_LUA_GETTER(GetType)
	DECLARE_LUA_GETTER(GetZoneID)
	DECLARE_LUA_GETTER(GetX)
	DECLARE_LUA_GETTER(GetY)
	DECLARE_LUA_GETTER(GetZ)
	
	DECLARE_LUA_FUNCTION(CastSkill) {
		LUA_RETURN(LUA_GET_INSTANCE()->CastSkill(
			reinterpret_cast<Unit *>(LUA_ARG(CUser *, 2)),
			LUA_ARG(uint32, 3)
		));
	}
};
