#pragma once

struct _LEVEL_UP
{
	uint8   ID;
	uint8	Level;
	int64	Exp;
	uint8	RebithLevel;
};

struct _MAGIC_TYPE1
{
	uint32	iNum;
	uint8	bHitType;
	uint16	sHitRate;
	uint16	sHit;
	uint16	sAddDamage;
	uint16	sRange;
	uint8	bComboType;
	uint8	bComboCount;
	uint16	sComboDamage;
	uint8	bDelay;
	uint16  iADPtoUser;
	uint16  iADPtoNPC;
};

struct _MAGIC_TYPE2
{
	uint32	iNum;
	uint8	bHitType;
	uint16	sHitRate;
	uint16	sAddDamage;
	uint16	sAddRange;
	uint8	bNeedArrow;
	uint16  iADPtoUser;
	uint16  iADPtoNPC;
};

struct _MAGIC_TYPE3
{
	uint32	iNum;
	uint8	bDirectType;
	int16	sFirstDamage;
	int16	sTimeDamage;
	uint8	bRadius;
	uint8	bDuration;  // duration, in seconds
	uint8	bAttribute;
	uint16  iADPtoUser;
	uint16  iADPtoNPC;
};

struct _MAGIC_TYPE4
{
	uint32	iNum;
	uint8	bBuffType;
	uint8	bRadius;
	uint16	sDuration;  // duration, in seconds
	uint8	bAttackSpeed;
	uint8	bSpeed;
	int16	sAC;
	uint16	sACPct;
	uint8	bAttack;
	uint8	bMagicAttack;
	uint16	sMaxHP;
	uint16	sMaxHPPct;
	uint16	sMaxMP;
	uint16	sMaxMPPct;
	int8	bStr;
	int8	bSta;
	int8	bDex;
	int8	bIntel;
	int8	bCha;
	uint8	bFireR;
	uint8	bColdR;
	uint8	bLightningR;
	uint8	bMagicR;
	uint8	bDiseaseR;
	uint8	bPoisonR;
	uint16	sExpPct;
	uint16	sSpecialAmount;
	uint8	bHitRate;
	uint16	sAvoidRate;

	bool	bIsBuff; // true if buff, false if debuff

	INLINE bool isnull() { return iNum == 0; }
	INLINE bool isBuff() { return bIsBuff; }
	INLINE bool isDebuff() { return !bIsBuff; }

	_MAGIC_TYPE4() {
		iNum = 0;
		bBuffType = 0;
		bRadius = 0;
		sDuration = 0;
		bAttackSpeed = 0;
		bSpeed = 0;
		sAC = 0;
		sACPct = 0;
		bAttack = 0;
		bMagicAttack = 0;
		sMaxHP = 0;
		sMaxHPPct = 0;
		sMaxMP = 0;
		sMaxMPPct = 0;
		bStr = 0;
		bSta = 0;
		bDex = 0;
		bIntel = 0;
		bCha = 0;
		bFireR = 0;
		bColdR = 0;
		bLightningR = 0;
		bMagicR = 0;
		bDiseaseR = 0;
		bPoisonR = 0;
		sExpPct = 0;
		sSpecialAmount = 0;
		bHitRate = 0;
		sAvoidRate = 0;
		bIsBuff = 0;
	}
};

struct _MAGIC_TYPE5
{
	uint32	iNum;
	uint8	bType;
	uint8	bExpRecover;
	uint16	sNeedStone;
};

struct _MAGIC_TYPE6
{
	uint32	iNum;
	uint16	sSize;
	uint16	sTransformID;
	uint16	sDuration; // duration, in seconds
	uint16	sMaxHp;
	uint16	sMaxMp;
	uint8	bSpeed;
	uint16	sAttackSpeed;
	uint16	sTotalHit;
	uint16	sTotalAc;
	uint16	sTotalHitRate;
	uint16	sTotalEvasionRate;
	uint16	sTotalFireR;
	uint16	sTotalColdR;
	uint16	sTotalLightningR;
	uint16	sTotalMagicR;
	uint16	sTotalDiseaseR;
	uint16	sTotalPoisonR;
	uint16	sClass;
	uint8	bUserSkillUse;
	uint8	bNeedItem;
	uint8	bSkillSuccessRate;
	uint8	bMonsterFriendly;
	uint8	bNation;
};

struct _MAGIC_TYPE7
{
	uint32	iNum;
	uint8	bValidGroup;
	uint8	bNationChange;
	uint16	sMonsterNum;
	uint8	bTargetChange;
	uint8	bStateChange;
	uint8	bRadius;
	uint16	sHitRate;
	uint16	sDuration;
	uint16	sDamage;
	uint8	bVision;
	uint32	nNeedItem;
};

struct _MAGIC_TYPE8
{
	uint32	iNum;
	uint8	bTarget;
	uint16	sRadius;
	uint8	bWarpType;
	uint16	sExpRecover;
	uint16	sKickDistance; // used exclusively by soccer ball-control skills.
};

struct _MAGIC_TYPE9
{
	uint32	iNum;
	uint8	bValidGroup;
	uint8	bNationChange;
	uint16	sMonsterNum;
	uint8	bTargetChange;
	uint8	bStateChange;
	uint16	sRadius;
	uint16	sHitRate;
	uint16	sDuration;
	uint16	sDamage;
	uint16	sVision;
	uint32	nNeedItem;
};

struct _MAGIC_TABLE
{
	uint32	iNum;
	int32   t_1;
	uint32	nBeforeAction;
	uint8	bTargetAction;
	uint8	bSelfEffect;
	uint16	bFlyingEffect;
	uint16	iTargetEffect;
	uint8	bMoral;
	uint16	sSkillLevel;	
	uint16	sSkill;
	uint16	sMsp;
	uint16	sHP;
	uint16  sSp;
	uint8	bItemGroup;
	uint32	iUseItem;
	uint8	bCastTime;
	uint16	sReCastTime, icelightrate;
	uint8	bSuccessRate;
	uint8	bType[2];
	uint16	sRange;
	uint16  sEtc;
	uint8	sUseStanding;
	uint8	sSkillCheck;
	std::string krname;

	_MAGIC_TYPE4 pType4;
	bool bIsBuff;

	INLINE bool isnull() { return iNum == 0; }

	_MAGIC_TABLE() {
		bIsBuff = false;
		pType4 = _MAGIC_TYPE4();
		iNum = 0;
		t_1 = 0;
		nBeforeAction = 0;
		bTargetAction = 0;
		bSelfEffect = 0;
		bFlyingEffect = 0;
		iTargetEffect = 0;
		bMoral = 0;
		sSkillLevel = 0;
		sSkill = 0;
		sMsp = 0;
		sHP = 0;
		sSp = 0;
		bItemGroup = 0;
		iUseItem = 0;
		bCastTime = 0;
		sReCastTime = 0;
		bSuccessRate = 0;
		sRange = 0;
		sEtc = 0;
		sUseStanding = 0;
		sSkillCheck = 0;
		icelightrate = 0;
		memset(bType, 0, sizeof(bType));
	}
};

enum TransformationSkillUse
{
	TransformationSkillUseSiege = 0,
	TransformationSkillUseMonster = 1,
	TransformationSkillUseNPC = 3,
	TransformationSkillUseSpecial = 4, // e.g. Snowman transformations
	TransformationSkillMovingTower = 6,
	TransformationSkillOreadsGuard = 5,
	TransformationSkillUseMamaPag = 7,  // e.g. Moving Towers
	TransformationSkillUseNone = 0xFF
};

struct _SERVER_RESOURCE
{
	uint32 nResourceID;
	std::string strResource;

	_SERVER_RESOURCE() {
		void Initialize();
	}
	void Initialize() {
		nResourceID = 0;
	}
};

struct _BOT_SAVE_DATA
{
	uint32 nAutoID;
	uint32 nNum[12];
	int16 sDuration[12];
	uint16 sCount[12];
	uint64 nSerialNum[12];
	uint32 nPrice[12];
	bool IsSoldOut[12];
	std::string AdvertMessage;
	uint16 Minute;
	uint8 byZone;
	float fX;
	float fY;
	float fZ;
	bool isKc[12];
	int16 Direction;
	uint8 sMerchanType;
};
struct _OBJECT_EVENT
{
	int nIndex;
	uint16 sZoneID;
	int sBelong;
	short sIndex;
	short sType;
	short sControlNpcID;
	short sStatus;
	float fPosX;
	float fPosY;
	float fPosZ;
	uint8 byLife;
};

#pragma pack(push, 1)
struct _REGENE_EVENT
{
	float fRegenePosX;
	float fRegenePosY;
	float fRegenePosZ;
	float fRegeneAreaZ;
	float fRegeneAreaX;
	int	  sRegenePoint;
};

struct _WARP_INFO
{
	short	sWarpID;
	char	strWarpName[32];
	char	strAnnounce[256];
	uint16	sUnk0; // padding?
	uint32	dwPay;
	short	sZone;
	uint16	sUnk1; // padding?
	float	fX;
	float	fY;
	float	fZ;
	float	fR;
	short	sNation;
	uint16	sUnk2; // padding?

	_WARP_INFO() { memset(this, 0, sizeof(_WARP_INFO)); };
};
#pragma pack(pop)

struct _ZONE_INFO
{
	uint16	m_nServerNo;
	uint16	m_nZoneNumber;
	std::string	m_MapName;
	std::string	m_ZoneName;
	uint8 m_Status;
	uint8 m_ZoneType;
	uint8 m_MinLevel;
	uint8 m_MaxLevel;
	uint8 m_kTradeOtherNation;
	uint8 m_kTalkOtherNation;
	uint8 m_kAttackOtherNation;
	uint8 m_kAttackSameNation;
	uint8 m_kFriendlyNpc;
	uint8 m_kWarZone;
	uint8 m_kClanUpdates;
	uint8 m_bBlink;
	uint8 m_bTeleport;
	uint8 m_bGate;
	uint8 m_bEscape;
	uint8 m_bCallingFriend;
	uint8 m_bTeleportFriend;
	uint8 m_bPetSpawn;
	uint8 m_bExpLost;
	uint8 m_bGiveLoyalty;
	uint8 m_bGuardSummon;
	uint8 m_bMenissiahList;
	uint8 m_bMilitaryZone;
	uint8 m_bMiningZone, m_bBlinkZone, m_bAutoLoot, m_bGoldLose;

	float m_fInitX, m_fInitY, m_fInitZ;

	_ZONE_INFO()
	{
		Initialize();
	}

	void Initialize()
	{
		m_nServerNo = 0;
		m_nZoneNumber = 0;
		m_Status = 0;
		m_ZoneType = 0;
		m_MinLevel = 0;
		m_MaxLevel = 0;
		m_kTradeOtherNation = 0;
		m_kTalkOtherNation = 0;
		m_kAttackOtherNation = 0;
		m_kAttackSameNation = 0;
		m_kFriendlyNpc = 0;
		m_kWarZone = 0;
		m_kClanUpdates = 0;
		m_bBlink = 0;
		m_bTeleport = 0;
		m_bGate = 0;
		m_bEscape = 0;
		m_bCallingFriend = 0;
		m_bTeleportFriend = 0;
		m_bPetSpawn = 0;
		m_bExpLost = 0;
		m_bGiveLoyalty = 0;
		m_bGuardSummon = 0;
		m_bMilitaryZone = 0;
		m_bMenissiahList = 0;
		m_bMiningZone = 0;
		m_bBlinkZone = 0;
		m_bAutoLoot = 0;
		m_bGoldLose = 0;
		m_fInitX = 0.0f;
		m_fInitY = 0.0f;
		m_fInitZ = 0.0f;
	}
};

struct _MONSTER_SUMMON_LIST
{
	uint16	sSid;
	uint16	sLevel;
	uint16	sProbability;
	uint8	bType;
};

struct _MONSTER_RESOURCE
{
	uint16		sSid;
	std::string strSidName;
	std::string strResource;
	uint16		sNoticeZone;
	uint16		sNoticeType;
	
	_MONSTER_RESOURCE() {
		void init();
	}
	void init() {
		sSid = 0;
		sNoticeZone = 0;
		sNoticeType = 0;
	}
};

struct _MONSTER_RESPAWN_VERIABLE_LIST
{
	uint16	sIndex;
	uint16	sSid;
	uint8   sType;
	uint8   sZoneID;
	uint16	sCount;
	uint16  sRadius;
	uint32  isDeadTime;
};

struct _MONSTER_RESPAWN_STABLE_LIST
{
	uint16	sIndex;
	uint16  GroupNumber;
	uint16	sSid;
	uint8   isNpc;
	uint8   sZoneID;
	uint16	sCount;
	uint8  sRadius;
	uint32  isDeadTime;


};

struct _MONSTER_RESPAWN_STABLE_SIGN_LIST
{
	uint16 GetID;
	uint16 GetZoneID;
	uint16 GetSsid;
	uint8  GetTrapNumber;
	uint16 CurrentSsid;
	uint16 CurrentGetID;
	float GetX;
	float GetY;
	float GetZ;
};

typedef std::map<uint16, std::string> RoomUserList;

struct _CHAT_ROOM
{
	int nIndex;

	std::string  strRoomName;
	std::string  strAdministrator;
	std::string  strPassword;

	bool isPassword(){return STRCASECMP(strPassword.c_str(), "") != 0; };
	uint8 isAdministrator(std::string strUserID){ return STRCASECMP(strAdministrator.c_str(), strUserID.c_str()) != 0 ? 1 : 2; };
	
	RoomUserList m_UserList;

	uint16 m_sMaxUser;
	uint16 m_sCurrentUser;

	uint8 m_bRoomNation;

	bool AddUser(std::string strUserID)
	{
		m_UserList.insert(std::make_pair(++m_sCurrentUser, strUserID));	
		return true;
	}

	_CHAT_ROOM() { Initialize(); }

	void Initialize()
	{
		nIndex = 0;
		m_sMaxUser = 0;
		m_sCurrentUser = 0;
		m_bRoomNation = 0;
		m_UserList.clear();
	}
};

struct _MONSTER_UNDER_THE_CASTLE
{
	uint16 sIndex;
	uint16 sSid;
	uint8 bType;
	uint8 bTrapNumber;
	uint16 X;
	uint16 Y;
	uint16 Z;
	uint8 byDirection;
	uint16  sCount;
	uint8 bRadius;
};

struct _MONSTER_STONE_LIST_INFORMATION
{
	uint16	sIndex;
	uint16	sSid;
	uint8	bType;
	uint16	sPid;
	uint8	ZoneID;
	uint8	isBoss;
	uint16	Family;
	uint8	byDirection;
	uint8	sCount;
	uint16	X;
	uint16	Y;
	uint16	Z;
};

struct _MONSTER_JURAID_MOUNTAIN_RESPAWN_LIST
{
	uint16	sIndex;
	uint16	sSid;
	uint8	bType;
	uint16	sPid;
	uint8	ZoneID;
	uint16	Family;
	uint8	sCount;
	uint16	X;
	uint16	Y;
	uint16	Z;
	uint8	byDirection;
	uint8	bRadius;
};

#define MAX_PARTY_USERS		8
struct	_PARTY_GROUP
{
	WORD wIndex;
	short uid[MAX_PARTY_USERS];
	uint8 bItemRouting;
	int16 NumberTargetID;
	std::string	WantedMessage;
	uint16 sWantedClass;
	_PARTY_GROUP()
	{
		Initialize();
	}

	void Initialize()
	{
		for (int i = 0; i < MAX_PARTY_USERS; i++)
			uid[i] = -1;

		wIndex = 0;
		bItemRouting = 0;
		WantedMessage = "";
		NumberTargetID = -1;
	}
};

