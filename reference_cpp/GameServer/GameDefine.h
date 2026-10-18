#pragma once

#define NPC_HAVE_ITEM_LIST	8
#define PERK_COUNT					13
#define MAX_MONSTER_STONE_ROOM	750
#define FORGETTEN_TEMPLE_MAX_USER	80
#define GAMEMASTER_NOT	"No Authorization."
// Classes
#define KARUWARRIOR			101	// Beginner Karus Warrior
#define KARUROGUE			102	// Beginner Karus Rogue
#define KARUWIZARD			103	// Beginner Karus Magician
#define KARUPRIEST			104	// Beginner Karus Priest
#define BERSERKER			105	// Skilled (after first job change) Karus Warrior
#define GUARDIAN			106	// Mastered Karus Warrior
#define HUNTER				107	// Skilled (after first job change) Karus Rogue
#define PENETRATOR			108	// Mastered Karus Rogue
#define SORSERER			109	// Skilled (after first job change) Karus Magician
#define NECROMANCER			110	// Mastered Karus Magician
#define SHAMAN				111	// Skilled (after first job change) Karus Priest
#define DARKPRIEST			112	// Mastered Karus Priest
#define KURIANSTARTER		113	// Kurian 
#define KURIANNOVICE		114	// Novice Kurian 
#define KURIANMASTER		115	// Mastered Kurian

#define ELMORWARRRIOR		201	// Beginner El Morad Warrior
#define ELMOROGUE			202	// Beginner El Morad Rogue
#define ELMOWIZARD			203	// Beginner El Morad Magician
#define ELMOPRIEST			204	// Beginner El Morad Priest
#define BLADE				205	// Skilled (after first job change) El Morad Warrior
#define PROTECTOR			206	// Mastered El Morad Warrior
#define RANGER				207	// Skilled (after first job change) El Morad Rogue
#define ASSASSIN			208	// Mastered El Morad Rogue
#define MAGE				209	// Skilled (after first job change) El Morad Magician
#define ENCHANTER			210	// Mastered El Morad Magician
#define CLERIC				211	// Skilled (after first job change) El Morad Priest
#define DRUID				212	// Mastered El Morad Priest
#define PORUTUSTARTER		213	// Kurian 
#define PORUTUNOVICE		214	// Novice PORUTU 
#define PORUTUMASTER		215	// Mastered PORUTU

// Races
#define KARUS_BIG			1	// Arch Tuarek (Karus Warriors - only!)
#define KARUS_MIDDLE		2	// Tuarek (Karus Rogues & Priests)
#define KARUS_SMALL			3	// Wrinkle Tuarek (Karus Magicians)
#define KARUS_WOMAN			4	// Puri Tuarek (Karus Priests)
#define KURIAN				6	// Kurian

#define BABARIAN			11	// Barbarian (El Morad Warriors - only!)
#define ELMORAD_MAN			12	// El Morad Male (El Morad - ALL CLASSES)
#define ELMORAD_WOMAN		13	// El Morad Female (El Morad - ALL CLASSES)
#define PORUTU				14	// Porutu

// Ÿ�ݺ� ����� //
#define GREAT_SUCCESS			0X01		// �뼺��
#define SUCCESS					0X02		// ����
#define NORMAL					0X03		// ����
#define	FAIL					0X04		// ���

#define ITEM_OREADS					700039768

enum class SealErrorCodes
{
	SealErrorNone = 0,
	SealNoError = 1,
	SealErrorFailed = 2,
	SealErrorNeedCoins = 3,
	SealErrorInvalidCode = 4,
	SealErrorPremiumOnly = 5,
	SealErrorFailed2 = 6,
	SealErrorTooSoon = 7,
};

enum class WarehouseError { NoAccess = 0, Access = 1, ReqMoney = 2, Invalid = 3 };

struct _type_cooldown
{
	ULONGLONG time;
	bool t_catch;
	_type_cooldown(ULONGLONG time, bool t_catch)
	{
		this->time = time;
		this->t_catch = t_catch;
	}
};

struct _botadd
{
	uint32 nIndex;
	ULONGLONG addtime;
	uint8 type;
	_botadd(uint32 nIndex, ULONGLONG addtime, uint8 type)
	{
		this->nIndex = nIndex;
		this->addtime = addtime;
		this->type = type;
	}
};

#define QUEST_MOB_GROUPS		4
struct _USER_QUEST_INFO
{
	uint8 QuestState;
	uint8 m_bKillCounts[QUEST_MOB_GROUPS];

	_USER_QUEST_INFO()
	{
		QuestState = 0;
		memset(&m_bKillCounts, 0, sizeof(m_bKillCounts));
	}
};

struct _cooldown
{
	ULONGLONG recasttime, time;
	_cooldown(ULONGLONG recasttime, ULONGLONG time)
	{
		this->recasttime = recasttime;
		this->time = time;
	}
};

struct _LEV_MERCH_INFO
{
	bool status;
	uint8 exprate;
	uint32 rewardtime;
	time_t finishtime;

	_LEV_MERCH_INFO()
	{
		Initialize();
	}

	void Initialize()
	{
		finishtime = 0;
		exprate = 0;
		rewardtime = 0;
		status = false;
	}
};

enum class CswOpStatus
{
	NotOperation,
	Preparation,
	War
};

enum class perksSub
{
	info,
	perkPlus,
	perkReset,
	perkUseItem,
	perkTargetInfo
};

enum class perksError
{
	remPerks,
	notFound,
	maxPerkCount,
	success
};

enum class perks
{
	weight,
	healt,
	mana,
	loyalty,
	percentDrop,
	percentExp,
	percentCoinsMon,
	percentCoinsSell,
	percentUpgradeChance,
	percentDamageMonster,
	percentDamagePlayer,
	defence,
	attack
};


struct _JACKPOT_SETTING
{
	uint8 iType;
	uint16 rate, _1000, _500, _100, _50, _10, _2;
	_JACKPOT_SETTING()
	{
		Initialize();
	}

	void Initialize()
	{
		iType = 0;
		rate = _1000 = _500 = _100 = _50 = _10 = _2 = 0;
	}
};

struct _CSW_TIMEOPT
{
	uint32 Preparing, wartime;
	uint32 money, tl, cash, loyalty;
	uint32 itemid[3], itemcount[3], itemtime[3];
	_CSW_TIMEOPT()
	{
		Initialize();
	}
	void Initialize()
	{
		money = tl = cash = loyalty = 0;
		Preparing = 10;
		wartime = 60;
		memset(itemid, 0, sizeof(itemid));
		memset(itemcount, 0, sizeof(itemcount));
		memset(itemtime, 0, sizeof(itemtime));
	}
};

enum class CswNotice
{
	Preparation,
	War,
	MonumentKilled,
	CswFinish,
};

struct _CSW_STATUS
{
	bool prepare_check, war_check;
	bool   Started;
	CswOpStatus Status;
	_CSW_TIMEOPT poptions;
	std::recursive_mutex mClanListLock;
	std::map<uint16, uint16> mClanList;
	time_t  CswTime, MonumentTime;
	_CSW_STATUS()
	{
		Initialize();
	}
	void Initialize()
	{
		prepare_check = war_check = false;
		Started = false;
		Status = CswOpStatus::NotOperation;
		poptions.Initialize();
		mClanListLock.lock();
		mClanList.clear();
		mClanListLock.unlock();
		CswTime = MonumentTime = 0;
	}
};

struct _TBDATA
{
	std::string sealpass, accountpass;
	_TBDATA() { Initialize(); }
	void Initialize()
	{
		sealpass.clear();
		accountpass.clear();
	}
};

struct _USER_INFOSTORAGE
{
	time_t flamebacktime;
	uint16  flamelevel;

	_USER_INFOSTORAGE() { Initialize(); }
	void Initialize()
	{
		flamebacktime = 0;
		flamelevel = 0;
	}
};

struct _iteminfo
{
	uint32 itemid[3], count[3];
	_iteminfo() { Initialize(); }
	void Initialize()
	{
		memset(itemid, 0, sizeof(itemid));
		memset(count, 0, sizeof(count));
	}
};

struct _DAMAGE_SETTING
{
	float priestTOwarriror
		, priestTOmage
		, priestTOpriest
		, priestTOrogue
		, priestTOkurian
		, warrirorTOrogue
		, warrirorTOmage
		, warrirorTOwarrior
		, warrirorTOpriest
		, warrirorTOkurian
		, rogueTOmage
		, rogueTOwarrior
		, rogueTOrogue
		, rogueTOpriest
		, rogueTOkurian
		, kurianTOmage
		, kurianTOwarrior
		, kurianTOrogue
		, kurianTOpriest
		, kurianTOkurian
		, mageTOwarriror
		, mageTOmage
		, mageTOpriest
		, mageTOrogue
		, mageTOkurian
		, mondef
		, montakedamage
		, magemagicdamage
		, uniqueitem
		, lowclassitem
		, middleclassitem
		, highclassitem
		, rareitem
		, magicitem
		, rdamage
		, kurianzehir;

	_DAMAGE_SETTING()
	{
		Initialize();
	}

	void Initialize()
	{
		priestTOwarriror
			= priestTOmage
			= priestTOpriest
			= priestTOrogue
			= priestTOkurian
			= warrirorTOrogue
			= warrirorTOmage
			= warrirorTOwarrior
			= warrirorTOpriest
			= warrirorTOkurian
			= rogueTOmage
			= rogueTOwarrior
			= rogueTOrogue
			= rogueTOpriest
			= rogueTOkurian
			= kurianTOmage
			= kurianTOwarrior
			= kurianTOrogue
			= kurianTOpriest
			= kurianTOkurian
			= mageTOwarriror
			= mageTOmage
			= mageTOpriest
			= mageTOrogue
			= mageTOkurian
			= mondef
			= montakedamage
			= magemagicdamage
			= uniqueitem
			= lowclassitem
			= middleclassitem
			= highclassitem
			= magicitem
			= rdamage = 1.0f;
	}
};

enum class cindopcode
{
	selectclass,
	nationchange,
	joinevent,
	starting,
	updatekda,
	finish,
	success,
	timewait,
	notchange,
	alreadyclass,
	alreadynation
};

enum class offcharactertype
{
	merchant,
	genie
};

enum class lettersendtype
{
	cindirella
};

struct _CHARACTER_SETVALID
{
	uint8 ClassIndex;
	uint8 NationValue;
	uint8 RaceValue;
	uint16 ClassValue;
	uint8 ReStat[(uint8)StatType::STAT_COUNT];

	INLINE bool null() { return ClassIndex == 0; }

	_CHARACTER_SETVALID()
	{
		ClassIndex = NationValue = RaceValue = 0;
		ClassValue = 0;
		memset(ReStat, 0, sizeof(ReStat));
	}
};

struct _LINFO
{
	uint32 itemid;
	uint16 scount, duration;
	uint32 expiration;
	std::string subject, message;
	_LINFO(uint32 a, uint16 b, uint16 c, uint32 d, std::string e, std::string f)
	{
		itemid = a;
		scount = b;
		duration = c;
		expiration = d;
		subject = e;
		message = f;
	}
};

struct _CINDWAR_SETTING
{
	uint32 playtime, preparetime, reqmoney, reqloyalty;
	uint16 maxuserlimit;
	uint8 minlevel, maxlevel, zoneid, beglevel;

	_CINDWAR_SETTING() { Initialize(); }
	void Initialize()
	{
		playtime = preparetime = 0;
		minlevel = maxlevel = 0;
		reqmoney = reqloyalty = maxuserlimit = 0;
		zoneid = beglevel = 0;
	}
};

struct _CINDWAR_REWARD
{
	uint16 rankid;
	uint32 expcount, cashcount, loyaltycount, moneycount;
	uint32 itemid[10], itemexpiration[10];
	uint16 itemcount[10], itemduration[10];
	_CINDWAR_REWARD() { Initialize(); }
	void Initialize()
	{
		rankid = 0;
		expcount = cashcount = loyaltycount = moneycount = 0;
		memset(itemid, 0, sizeof(itemid));
		memset(itemcount, 0, sizeof(itemcount));
		memset(itemduration, 0, sizeof(itemduration));
		memset(itemexpiration, 0, sizeof(itemexpiration));
	}
};


struct _CINDWARUSER
{
	bool eventuser, first_selected;
	_ITEM_DATA m_sItemArray[INVENTORY_TOTAL];
	uint16 m_sClass, m_sPoints;
	uint8 m_bstrSkill[10], m_bRace, myselectclass, m_eNation, m_oNation;
	uint8 m_bStats[(uint8)StatType::STAT_COUNT];
	uint8 m_bLevel;
	uint64 m_iExp;
	uint32 m_iGold;
	time_t myselectlasttime, myselectnationtime;
	uint16 killcount, deadcount;
	char m_strSkillData[320];
	uint16 m_strSkillCount;
	uint32 gainexp, gainnoah;
	typedef std::map <uint16, _USER_QUEST_INFO>	QuestMap;
	QuestMap m_QuestMap;
	typedef	std::map<uint32, ULONGLONG>			UserSavedMagicMap; //23.04.2024
	UserSavedMagicMap m_savedMagicMap; //23.04.2024

	INLINE bool isEventUser() { return eventuser; }

	_CINDWARUSER() { Initialize(); }
	void Initialize()
	{
		m_eNation = 0;
		gainexp = gainnoah = 0;
		memset(&m_sItemArray, 0, sizeof(m_sItemArray));
		memset(&m_bstrSkill, 0, sizeof(m_bstrSkill));
		memset(&m_bStats, 0, sizeof(m_bStats));
		memset(&m_strSkillData, 0, sizeof(m_strSkillData));
		memset(&myselectclass, 0, sizeof(myselectclass));
		eventuser = false;
		first_selected = true;
		m_sPoints = 0;
		m_sClass = 0;
		m_bRace = 0;
		m_bLevel = 0;
		m_iExp = 0;
		m_iGold = 0;
		m_strSkillCount = 0;
		myselectlasttime = myselectnationtime = 0;
		killcount = deadcount = 0;
		m_QuestMap.clear();
		m_savedMagicMap.clear(); //23.04.2024
		m_oNation = 0;
	}
};

struct _cindwarmycount
{
	uint16 killcount, deadcount;

	_cindwarmycount(uint16 killcount, uint16 deadcount)
	{
		this->killcount = killcount;
		this->deadcount = deadcount;
	}
};

struct _PERKS
{
	uint32 pIndex;
	uint16 perkCount, maxPerkCount;
	bool status, percentage;
	std::string strDescp;
};

struct _RANK_REWARD
{
	uint8 RankNumber;
	uint32 nItemID;
	uint16 sCount;
	_RANK_REWARD() {
		Initialize();
	}

	void Initialize() {
		RankNumber = 0;
		nItemID = 0;
		sCount = 0;
	}
};

struct _PERKS_DATA
{
	uint16 perkType[PERK_COUNT], remPerk;
	_PERKS_DATA()
	{
		Initialize();
	}

	void Initialize()
	{
		memset(perkType, 0, sizeof(perkType));
		remPerk = 0;
	}
};

enum class messagecolour
{
	red = 0xFF0000,
	green = 0x008000,
	blue = 0x0000FF,
	white = 0xFFFFFF,
	black = 0x000000
};

struct _CINDWARGAME
{
	bool start, prepare;
	time_t starttime, finishtime, preparetime;
	uint16 e_killcount, k_killcount; uint8 settingid;

	std::recursive_mutex e_listlock, k_listlock;
	//std::vector<uint16> e_list, k_list;
	std::map<std::string, _cindwarmycount> e_list, k_list;
	_CINDWAR_SETTING pSetting[5];
	_CINDWAR_REWARD pReward[200];
	_ITEM_DATA m_warrior[INVENTORY_TOTAL],
		m_rogue[INVENTORY_TOTAL], m_mage[INVENTORY_TOTAL],
		m_priest[INVENTORY_TOTAL], m_kurian[INVENTORY_TOTAL];

	INLINE bool isPrepara() { return prepare; }
	INLINE bool isStarted() { return start; }

	INLINE bool isON() { return prepare || start; }

	INLINE uint16 GetElmoradCount()
	{
		Guard lock(e_listlock);
		return (uint16)e_list.size();
	}
	INLINE uint16 GetKarusCount()
	{
		Guard lock(k_listlock);
		return (uint16)k_list.size();
	}
	INLINE uint16 GetTotalCount()
	{
		return GetElmoradCount() + GetKarusCount();
	}

	_CINDWARGAME() { Initialize(true); }
	void Initialize(bool gameopened)
	{
		start = prepare = false;
		starttime = finishtime = 0;
		e_killcount = k_killcount = 0;

		e_listlock.lock();
		e_list.clear();
		e_listlock.unlock();

		k_listlock.lock();
		k_list.clear();
		k_listlock.unlock();

		preparetime = 0;
		settingid = 0;

		if (gameopened)
		{

			memset(&m_warrior, 0, sizeof(m_warrior));
			memset(&m_rogue, 0, sizeof(m_rogue));
			memset(&m_mage, 0, sizeof(m_mage));
			memset(&m_priest, 0, sizeof(m_priest));
			memset(&m_kurian, 0, sizeof(m_kurian));
			for (int i = 0; i < 5; i++) pSetting[i].Initialize();
			for (int i = 0; i < 200; i++) pReward[i].Initialize();
		}
	}
};

struct _CINDWAR_ITEMS
{
	uint8 iclass, slotid, itemflag;
	uint32 id, itemid, itemexpire;
	uint16 itemcount, itemduration;
};

struct _CINDWAR_STATSET
{
	uint32 id;
	uint8 settindid;
	uint8 iClass, skill[4], stat[5];
	uint16 Skillfreepoint, Statfreepoint;
	_CINDWAR_STATSET()
	{
		id = 0;
		settindid = 0;
		iClass = 0;
		Skillfreepoint = 0;
		Statfreepoint = 0;
		memset(skill, 0, sizeof(skill));
		memset(stat, 0, sizeof(stat));
	}
};

struct _LEVEL_REWARDS
{
	bool isnull;
	std::string bNotice;
	uint32 cash, tl, loyalty, money;
	uint32 itemid[3], itemcount[3], itemtime[3];
	_LEVEL_REWARDS()
	{
		isnull = true;
		bNotice.clear();
		cash = tl = loyalty = money = 0;
		memset(itemid, 0, sizeof(itemid));
		memset(itemcount, 0, sizeof(itemcount));
		memset(itemtime, 0, sizeof(itemtime));
	}
};

struct _LEVEL_MERCHANT_REWARDS
{
	uint16 index;
	uint32 startHour, startMinute, exp_minute, finih_minute;
	uint8 rate_experience;
	_LEVEL_MERCHANT_REWARDS()
	{
		index = 0;
		startHour = startMinute = exp_minute = 0;
		rate_experience = 0;
		finih_minute = 0;
	}
};

enum class SmashExchangeError
{
	SmashSuccess = 1,
	SmashInventory = 2,
	SmashBag = 3,
	SmashItem = 4,
	SmashNpc = 5,
};

struct _FORGETTEN_TEMPLE_STAGES
{
	int32 nIndex;
	uint8  Type, Stage;
	uint16 Time;

	_FORGETTEN_TEMPLE_STAGES()
	{
		nIndex = -1;
		Type = 0;
		Stage = 0;
		Time = 0;
	}
};

struct _AUTOMATIC_COMMAND
{
	uint32 sIndex;
	std::string command;
	uint32 hour, minute, iDay;
};

struct XFTTIMEOPT
{
	uint16 PlayingTime, SummonTime, SpawnMinTime, WaitingTime;
	uint8 MinLevel, MaxLevel;
	XFTTIMEOPT() { Initialize(); }
	void Initialize()
	{
		PlayingTime = SummonTime = SpawnMinTime = WaitingTime = 0;
		MinLevel = MaxLevel = 0;
	}
};

struct _UNDER_THE_CASTLE 
{
	std::recursive_mutex UserListLock;
	std::map<uint16, uint64> UserList;
	std::recursive_mutex m_UnderTheCastleMonsterListLock;
	std::map<uint16, uint16> m_UnderTheCastleMonsterList;

	bool  isActive;
	bool  isSummon;
	uint32 StartTime;
	uint32 StartMoveTime;

	uint8  minlevel;
	uint8  maxlevel;

	_UNDER_THE_CASTLE()
	{
		Initialize();
	}

	void Initialize()
	{
		UserListLock.lock();
		UserList.clear();
		UserListLock.unlock();

		m_UnderTheCastleMonsterListLock.lock();
		m_UnderTheCastleMonsterList.clear();
		m_UnderTheCastleMonsterListLock.unlock();

		isActive = false;
		isSummon = false;

		StartTime = NULL;
		StartMoveTime = NULL;

		minlevel = NULL;
		maxlevel = NULL;
	}
};

#if(GAME_SOURCE_VERSION == 2369) 
struct _CASTELLAN_WAR
{
	std::recursive_mutex UserListLock;
	std::map<uint16, uint64> UserList;

	bool  isActive;
	bool  isStard;
	uint32 StartTime;
	uint32 NoticeTime;

	uint8  minlevel;
	uint8  maxlevel;

	_CASTELLAN_WAR()
	{
		Initialize();
	}

	void Initialize()
	{
		UserListLock.lock();
		UserList.clear();
		UserListLock.unlock();

		isActive = false;
		isStard = false;
		NoticeTime = NULL;
		StartTime = NULL;
		minlevel = NULL;
		maxlevel = NULL;
	}
};
#endif

struct _FORGETTEN_TEMPLE
{
	XFTTIMEOPT ptimeopt;

	std::recursive_mutex UserListLock;
	std::map<uint16, uint64> UserList;
	uint16 monstercount;
	std::vector<uint32> SpawnList;

	bool  isActive;
	bool  isJoin;
	bool  isSummon;
	bool  isSummonCheck;
	bool  isLastSummon;
	bool  isFinished;
	bool  isWaiting;

	uint16 Stage;
	uint8  Type;
	uint8  MinLevel;
	uint8  MaxLevel;
	time_t StartTime;
	time_t FinishTime;
	time_t WaitingTime;
	time_t isLastSummonTime;
	uint32 NoticeStartHour;

	_FORGETTEN_TEMPLE()
	{
		Initialize();
	}

	void Initialize(bool init = false)
	{
		UserListLock.lock();
		UserList.clear();
		UserListLock.unlock();

		monstercount = 0;
		isActive = false;
		isJoin = false;
		isSummon = false;
		isSummonCheck = false;
		isLastSummon = false;
		isFinished = false;
		isWaiting = false;
		Stage = 1;
		Type = 0;
		MinLevel = 0;
		MaxLevel = 0;
		StartTime = 0;
		FinishTime = 0;
		WaitingTime = 0;
		isLastSummonTime = 0;
		NoticeStartHour = 0;

		if (!init) ptimeopt.Initialize();
	}
};

struct _FORGETTEN_TEMPLE_SUMMON
{
	uint32 bIndex;
	uint8  Type;
	uint8  Stage;
	uint16 SidID;
	uint16 SidCount;
	uint16 PosX;
	uint16 PosZ;
	uint16  Range;

	_FORGETTEN_TEMPLE_SUMMON()
	{
		Initialize();
	}

	void Initialize()
	{
		bIndex = 0;
		Type = 0;
		Stage = 0;
		SidID = 0;
		SidCount = 0;
		PosX = 0;
		PosZ = 0;
		Range = 0;
	}
};


struct _SPEVENT
{
	bool opened, finishcheck;
	time_t finishtime, starttime;
	uint8 zoneid;

	std::string ename, kname, eventname;
	uint16 kkillcount, ekillcount;

	_SPEVENT() { Initialize(); }

	void Initialize()
	{
		opened = finishcheck = false;
		finishtime = 0;
		zoneid = 0;
		ename = "HUMAN";
		kname = "KARUS";
		eventname.clear();
		kkillcount = ekillcount = 0;
	}
};


struct EVENT_VROOMOPLIST
{
	uint8 zoneid;
	std::string Name;
	uint32 sign, play, attackopen, attackclose, finish;
	EVENT_VROOMOPLIST()
	{
		Initialize();
	}
	void Initialize()
	{
		Name.clear();
		sign = play = attackopen = attackclose = finish = 0;
		zoneid = 0;
	}
};

struct EVENT_DAYLIST
{
	bool day[7];
};

struct EVENT_OPENTIMELIST
{
	uint8 eventid, type, zoneid;
	int8 hour[5], minute[5], second[5];
	bool status, iday[7], timeactive[5];
	std::string name;

	uint8 minLevel, maxLevel;
	uint32 reqLoyalty, reqMoney;

	EVENT_OPENTIMELIST()
	{
		eventid = type = zoneid = 0;
		status = false;
		memset(hour, 0, sizeof(hour));
		memset(minute, 0, sizeof(minute));
		memset(second, 0, sizeof(second));
		memset(iday, 0, sizeof(iday));
		memset(timeactive, 0, sizeof(timeactive));
		name.clear();
		minLevel = maxLevel = 0;
		reqLoyalty = reqMoney = 0;
	}
};

struct EVENT_CHAOSREWARD
{
	uint32 itemid[5], itemcount[5], itemtime[5];
	uint32 experience, loyalty, cash, noah;
	EVENT_CHAOSREWARD()
	{
		Initialize();
	}

	void Initialize()
	{
		memset(itemid, 0, sizeof(itemid));
		memset(itemcount, 0, sizeof(itemcount));
		memset(itemtime, 0, sizeof(itemtime));
		experience = loyalty = cash = noah = 0;
	}
};

struct EVENTMAINTIME
{
	std::recursive_mutex mtimeforlooplock;
	std::vector< EVENT_OPENTIMELIST> mtimeforloop;
	CSTLMap<EVENT_OPENTIMELIST, uint8> mtimelist;
	CSTLMap<EVENT_DAYLIST, uint8> mdaylist;
	EVENT_VROOMOPLIST pvroomop[3];

	EVENTMAINTIME()
	{
		Initialize();
	}

	void Initialize()
	{
		mtimeforloop.clear();
		mdaylist.DeleteAllData();
		mtimelist.DeleteAllData();
	}
};



enum class kaopcode { kill = 1, assist = 2 };

enum class cntscode
{
	Failed,
	SuccessReq,
	AlreadyReq,
	NoClanLeader,
	LowNember,
	OnlineNember,
	NoItem,
	isKing,
	isClan,
	Success
};

enum class BeefEffectType
{
	EffectNone = 0,
	EffectRed = 1,
	EffectGreen = 2,
	EffectWhite = 3
};

enum class BottomUserListOpcode
{
	Sign = 1,
	UserInfoDetail = 2,
	UserList = 3,
	RegionDelete = 4,
	unknow = 5
};

enum class ProcDbServerType
{
	UpdateSiegeWarfareDb = 0,
	UpdateKingSystemDb = 1,
	UpdateKnights = 2,
	ReloadSymbolAndKnights = 3,
	ResetLoyalty = 5,
	DrakiTowerLimitReset = 6,
	ResetKnights = 7,
	BotRegion,
	LotteryReward,
	BotSaveLoad,
	GmCommandGiveLetterItem,
	CollectionRaceStart,
	BotMerchantCoordSave,
	king_selection
};

enum class ProcDbType
{
	ReturnGiveLetterItem = 0,
	DrakiTowerUpdate = 1,
	SendKnightCash = 2,
	KickOutAllUser = 5,
	AccountInfoSave = 6,
	AccountInfoShow = 7,
	ClanBankSave = 8,
	ClanPremium = 9,
	UpdateKnightCash = 10,
	UpdateAccountKnightCash = 11,
	UserReportDbSave = 12,
	UpdateSheriffTable = 13,
	SaveSheriffReport = 14,
	PusGiftSendLetter = 15,
	ItemRemove = 16,
	ClanNts = 17,
	NtsCommand = 18,
	CheatLog,
	DailyQuestReward,
	CollectionRaceReward,
	Letter

};

struct BURNING_FEATURES
{
	uint8 nprate, moneyrate, exprate, droprate;
	BURNING_FEATURES() { Initialize(); }
	void Initialize()
	{
		nprate = moneyrate = exprate = droprate = 0;
	}
};

struct _MERCHANT_LIST
{
	std::string	strUserName;

	uint16	strUserID;
	uint32	ItemID[12];
	uint32	Price[12];
	uint8	Type;
};

enum class CastCheckType
{
	Failed = 0,
	Success = 1,
	Add = 2,
	NoValue = 3
};

enum class e_summontype
{
	m_none,
	m_MonsterStoneBoss,
	m_bjuriadmonster,
	m_ftmonster
};

enum class LootErrorCodes
{
	LootError = 0,
	LootSolo = 1,
	LootPartyCoinDistribution = 2,
	LootPartyNotification = 3,
	LootPartyItemGivenAway = 4,
	LootPartyItemGivenToUs = 5,
	LootNoWeight = 6,
	LootNoSlot = 7
};

enum PetModes
{
	MODE_ATTACK = 3,
	MODE_DEFENCE = 4,
	MODE_LOOTING = 8,
	MODE_CHAT = 9,
	MODE_SATISFACTION_UPDATE = 0x0F,
	MODE_FOOD = 0x10
};

enum ItemMovementType
{
	ITEM_INVEN_SLOT = 1,
	ITEM_SLOT_INVEN = 2,
	ITEM_INVEN_INVEN = 3,
	ITEM_SLOT_SLOT = 4,
	ITEM_INVEN_ZONE = 5,
	ITEM_ZONE_INVEN = 6,
	ITEM_INVEN_TO_COSP = 7,  // Inventory -> Cospre bag
	ITEM_COSP_TO_INVEN = 8,  // Cospre bag -> Inventory
	ITEM_INVEN_TO_MBAG = 9,  // Inventory -> Magic bag
	ITEM_MBAG_TO_INVEN = 10, // Magic bag -> Inventory
	ITEM_MBAG_TO_MBAG = 11,  // Magic bag -> Magic bag
	ITEM_INVEN_TO_PET = 12, // Inventory -> Pet Inventory
	ITEM_PET_TO_INVEN = 13  // Pet Inventory -> Inventory
};

enum SpawnEventType
{
	UnderTheCastleSummon = 1,
	ChaosStoneSummon = 2,
	DungeonDefencSummon = 4,
	DrakiTowerSummon = 5,
	MonsterBossSummon = 6,
	GuardSummon = 7,
	MonsterStone = 8,
	JuraidMonster,
	Loopstill,
	ForgettenTempleSummon
};

enum ItemSlotType
{
	ItemSlot1HEitherHand = 0,
	ItemSlot1HRightHand = 1,
	ItemSlot1HLeftHand = 2,
	ItemSlot2HRightHand = 3,
	ItemSlot2HLeftHand = 4,
	ItemSlotPauldron = 5,
	ItemSlotPads = 6,
	ItemSlotHelmet = 7,
	ItemSlotGloves = 8,
	ItemSlotBoots = 9,
	ItemSlotEarring = 10,
	ItemSlotNecklace = 11,
	ItemSlotRing = 12,
	ItemSlotShoulder = 13,
	ItemSlotBelt = 14,
	ItemSlotKaul = 20,
	ItemSlotBag = 25,
	ItemSlotCospreGloves = 100,
	ItemSlotCosprePauldron = 105,
	ItemSlotCospreHelmet = 107,
	ItemSlotCospreWings = 110,
	ItemSlotCospreFairy = 111,
	ItemSlotCospreTattoo = 112
};

// Item Weapon Kind
#define WEAPON_KIND_DAGGER		11
#define WEAPON_KIND_1H_SWORD	21
#define WEAPON_KIND_2H_SWORD	22
#define WEAPON_KIND_1H_AXE		31
#define WEAPON_KIND_2H_AXE		32
#define WEAPON_KIND_1H_CLUP		41
#define WEAPON_KIND_2H_CLUP		42
#define WEAPON_KIND_1H_SPEAR	51
#define WEAPON_KIND_2H_SPEAR	52
#define WEAPON_SHIELD			60
#define WEAPON_PICKAXE			61	// Unlike the others, this is just the Kind field as-is (not / 10).
#define WEAPON_MATLOCK			62	// Unlike the others, this is just the Kind field as-is (not / 10). ZONE_PRISON
#define WEAPON_FISHING			63	// Unlike the others, this is just the Kind field as-is (not / 10).
#define WEAPON_KIND_BOW			70
#define WEAPON_KIND_CROSSBOW	71
#define ACCESSORY_EARRING		91
#define ACCESSORY_NECKLACE		92
#define ACCESSORY_RING			93
#define ACCESSORY_BELT			94
#define WEAPON_KIND_STAFF		110
#define WEAPON_KIND_JAMADHAR	140
#define WEAPON_KIND_MACE		181
#define WEAPON_KIND_PORTU_AC	200
#define WEAPON_KIND_WARRIOR_AC	210
#define WEAPON_KIND_ROGUE_AC	220
#define WEAPON_KIND_MAGE_AC		230
#define WEAPON_KIND_PRIEST_AC	240
#define ITEM_KIND_COSPRE		252
#define ITEM_KIND_PET_ITEM		151

////////////////////////////////////////////////////////////
// User Status //
#define USER_STANDING			0X01		// �� �ִ�.
#define USER_SITDOWN			0X02		// �ɾ� �ִ�.
#define USER_DEAD				0x03		// ��Ŷ�
#define USER_BLINKING			0x04		// ��� ��Ƴ���!!!
#define USER_MONUMENT			0x06		// MonumentSystem	
#define USER_MINING				0x07		// MINING System
#define USER_FLASHING			0x08		// FLASHING System
////////////////////////////////////////////////////////////
// Durability Type
#define ATTACK				0x01
#define DEFENCE				0x02
#define REPAIR_ALL			0x03
#define ACID_ALL			0x04
#define UTC_ATTACK			0x05
#define UTC_DEFENCE			0x06
////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////
// Knights Authority Type
/*
#define CHIEF				0x06
#define VICECHIEF			0x05*/
#define OFFICER				0x04
#define KNIGHT				0x03
//#define TRAINEE				0x02
#define PUNISH				0x01	

#define CHIEF				0x01	// ����
#define VICECHIEF			0x02	// �δ���
#define TRAINEE				0x05	// ���
#define COMMAND_CAPTAIN		100		// ���ֱ���
////////////////////////////////////////////////////////////

#define CLAN_COIN_REQUIREMENT	500000
#define CLAN_LEVEL_REQUIREMENT	30

#define ITEM_INFINITYARC			800606000 // Infinity Arrow
#define ITEM_INFINITYCURE			346391000 // Infinity Cure
#define ITEM_CHAT					900012000 // Chat Quest
#define ITEM_GOLD					900000000 // Coins
#define ITEM_EXP					900001000
#define ITEM_COUNT					900002000
#define ITEM_LADDERPOINT			900003000
#define ITEM_SKILL					900007000
#define ITEM_HUNT					900005000 // Hunt for Quests
#define ITEM_NO_TRADE_MIN			900000001 // Cannot be traded, sold or stored.
#define ITEM_NO_TRADE_MAX			999999999 // Cannot be traded, sold or stored.
#define ITEM_SCROLL_LOW				379221000 // Upgrade Scroll (Low Class item)
#define ITEM_SCROLL_MIDDLE			379205000 // Upgrade Scroll (Middle Class item)
#define ITEM_SCROLL_HIGH			379021000 // Blessed Upgrade Scroll
#define ITEM_UPGRADE_SCROLL			379016000 // Upgrade Scroll
#define ITEM_SCROLL_REB				379256000 // Reverse Scroll
#define ITEM_SCROLL_REBSTR			379257000 // Reverse Item Strengthen Scroll
#define ITEM_TRINA					700002000 // Trina's Piece
#define ITEM_KARIVDIS				379258000 // Tears of Karivdis
#define ITEM_SHADOW_PIECE			700009000 // Shadow Piece
#define ITEM_DRAGON_SCALE			700043000 // Dragon Scale
#define ITEM_MAGE_EARRING			310310004
#define ITEM_WARRIOR_EARRING		310310005
#define ITEM_ROGUE_EARRING			310310006
#define ITEM_PRIEST_EARRING			310310007
#define SIEGEWARREWARD				914007000
#define VIP_VAULT_KEY				800442000
#define AUTOMATIC_MINING			501610000
#define AUTOMATIC_FISHING			501620000
#define ITEM_LOGOS					890092000
#define ITEM_NO_TRADE				900000001 // Cannot be traded, sold or stored. //clanbank

////////////////////////////////////////////////////////////
// EVENT MISCELLANOUS DATA DEFINE
#define ZONE_TRAP_INTERVAL	   2		// Interval is one second (in seconds) right now.
#define ZONE_TRAP_DAMAGE	   500		// HP Damage is 10 for now :)

////////////////////////////////////////////////////////////

#define RIVALRY_DURATION		(300)	// 5 minutes
#define RIVALRY_NP_BONUS		(150)	// 150 additional NP on kill
#define MINIRIVALRY_NP_BONUS	(50)	// 150 additional NP on kill

#define MAX_ANGER_GAUGE			(5)		// Maximum of 5 deaths in a PVP zone to fill your gauge.

#define PVP_MONUMENT_NP_BONUS	(5)	// 5 additional NP on kill
#define EVENT_MONUMENT_NP_BONUS	(10)	// 10 additional NP on kill

#define MAX_ID_REPORT 15

#define PET_START_ITEM  610001000
#define PET_START_LEVEL	1

////////////////////////////////////////////////////////////
// SKILL POINT DEFINE
#define ORDER_SKILL			0x01
#define MANNER_SKILL		0X02
#define LANGUAGE_SKILL		0x03
#define BATTLE_SKILL		0x04
#define PRO_SKILL1			0x05
#define PRO_SKILL2			0x06
#define PRO_SKILL3			0x07
#define PRO_SKILL4			0x08

enum SkillPointCategory
{
	SkillPointFree = 0,
	SkillPointCat1 = 5,
	SkillPointCat2 = 6,
	SkillPointCat3 = 7,
	SkillPointMaster = 8
};

/////////////////////////////////////////////////////////////
// ITEM TYPE DEFINE
#define ITEM_TYPE_FIRE				0x01
#define ITEM_TYPE_COLD				0x02
#define ITEM_TYPE_LIGHTNING			0x03
#define ITEM_TYPE_POISON			0x04
#define ITEM_TYPE_HP_DRAIN			0x05
#define ITEM_TYPE_MP_DAMAGE			0x06
#define ITEM_TYPE_MP_DRAIN			0x07
#define ITEM_TYPE_MIRROR_DAMAGE		0x08

/////////////////////////////////////////////////////////////
// JOB GROUP TYPES
#define GROUP_WARRIOR				1
#define GROUP_ROGUE					2
#define GROUP_MAGE					3
#define GROUP_CLERIC				4
#define GROUP_ATTACK_WARRIOR		5
#define GROUP_DEFENSE_WARRIOR		6
#define GROUP_ARCHERER				7
#define GROUP_ASSASSIN				8
#define GROUP_ATTACK_MAGE			9
#define GROUP_PET_MAGE				10
#define GROUP_HEAL_CLERIC			11
#define GROUP_CURSE_CLERIC			12
#define GROUP_PORTU_KURIAN			13

#define MAX_SELECTING_USER 5

//////////////////////////////////////////////////////////////
// USER ABNORMAL STATUS TYPES
enum AbnormalType
{
	ABNORMAL_INVISIBLE = 0,	// Hides character completely (for GM visibility only).
	ABNORMAL_NORMAL = 1,	// Shows character. This is the default for players.
	ABNORMAL_GIANT = 2,	// Enlarges character.
	ABNORMAL_DWARF = 3,	// Shrinks character.
	ABNORMAL_BLINKING = 4,	// Forces character to start blinking.
	ABNORMAL_GIANT_TARGET = 6,	// Enlarges character and shows "Hit!" effect.
	ABNORMAL_CHAOS_NORMAL = 7,	// Returns the user to the Chaos nonblinkingin form.
	BOARDING = 8
};

//////////////////////////////////////////////////////////////
// Object Type
#define NORMAL_OBJECT		0
#define SPECIAL_OBJECT		1

//////////////////////////////////////////////////////////////
// REGENE TYPES
#define REGENE_NORMAL		0
#define REGENE_MAGIC		1
#define REGENE_ZONECHANGE	2
#define MAX_PARTY_USERS		8

struct _CLASS_COEFFICIENT
{
	uint16	sClassNum;
	float	ShortSword;
	float   Jamadar;
	float	Sword;
	float	Axe;
	float	Club;
	float	Spear;
	float	Pole;
	float	Staff;
	float	Bow;
	float	HP;
	float	MP;
	float	SP;
	float	AC;
	float	Hitrate;
	float	Evasionrate;
};

struct _CLASS_DAMAGE
{
	uint16	sClassNum;
	float	sRogue;
	float	sWarrior;
	float	sPriest;
	float	sMage;
	float	sKurian;
	_CLASS_DAMAGE()
	{
		initiliaze();
	}

	void initiliaze()
	{
		sClassNum = 0;
		sRogue = 0.0f;
		sWarrior = 0.0f;
		sPriest = 0.0f;
		sMage = 0.0f;
		sKurian = 0.0f;
	}
};

struct _ITEM_TABLE
{
	uint32	m_iNum;
	int16	Extension;
	std::string	m_sName;
	std::string	Description;
	//uint32	ItemIconID1;
	//uint32	ItemIconID2;
	uint8	m_bKind;
	uint8	m_bSlot;
	uint8	m_bRace;
	uint8	m_bClass;
	uint16	m_sDamage;
	uint16	m_sDelay;
	uint16	m_sRange;
	uint16	m_sWeight;
	uint16	m_sDuration;
	uint32	m_iBuyPrice;
	uint32	m_iSellPrice;
	uint8   m_iSellNpcType;
	uint32   m_iSellNpcPrice;
	int16	m_sAc;
	uint8	m_bCountable;
	uint32	m_iEffect1;
	uint32	m_iEffect2;
	uint8	m_bReqLevel;
	uint8	m_bReqLevelMax;
	uint8	m_bReqRank;
	uint8	m_bReqTitle;
	uint8	m_bReqStr;
	uint8	m_bReqSta;
	uint8	m_bReqDex;
	uint8	m_bReqIntel;
	uint8	m_bReqCha;
	uint8	m_isUpgradeNotice;
	uint8	m_isDropNotice;
	uint16	m_bSellingGroup;
	uint8	m_ItemType;
	uint16	m_sHitrate;
	uint16	m_sEvarate;
	uint16	m_sDaggerAc;
	uint16	m_JamadarAc;
	uint16	m_sSwordAc;
	uint16	m_sClubAc;
	uint16	m_sAxeAc;
	uint16	m_sSpearAc;
	uint16	m_sBowAc;
	uint8	m_bFireDamage;
	uint8	m_bIceDamage;
	uint8	m_bLightningDamage;
	uint8	m_bPoisonDamage;
	uint8	m_bHPDrain;
	uint8	m_bMPDamage;
	uint8	m_bMPDrain;
	uint8	m_bMirrorDamage;
	uint8	m_DropRate;
	int16	m_sStrB;
	int16	m_sStaB;
	int16	m_sDexB;
	uint8   m_byGrade;
	int16	m_sIntelB;
	int16	m_sChaB;
	int16	m_MaxHpB;
	int16	m_MaxMpB;
	int16	m_bFireR;
	int16	m_bColdR;
	int16	m_bLightningR;
	int16	m_bMagicR;
	int16	m_bPoisonR;
	int16	m_bCurseR;
	int16	ItemClass;

	uint16 minDamage, maxDamage;
	uint32	m_iNPBuyPrice;
	int16	m_Bound;


	_ITEM_TABLE()
	{
		m_iNum = 0;
		minDamage = 0;
		maxDamage = 0;
		Extension = -1;
		//ItemIconID1 = 0;
		//ItemIconID2 = 0;
		m_bKind = 0;
		m_bSlot = 0;
		m_isUpgradeNotice = 0;
		m_isDropNotice = 0;
		m_bRace = 0;
		m_bClass = 0;
		m_sDamage = 0;
		m_sDelay = 0;
		m_sRange = 0;
		m_sWeight = 0;
		m_sDuration = 0;
		m_iBuyPrice = 0;
		m_iSellPrice = 0;
		m_sAc = 0;
		m_bCountable = 0;
		m_iEffect1 = 0;
		m_iEffect2 = 0;
		m_bReqLevel = 0;
		m_bReqLevelMax = 0;
		m_bReqRank = 0;
		m_byGrade = 0;
		m_bReqTitle = 0;
		m_bReqStr = 0;
		m_bReqSta = 0;
		m_bReqDex = 0;
		m_bReqIntel = 0;
		m_bReqCha = 0;
		m_bSellingGroup = 0;
		m_ItemType = 0;
		m_sHitrate = 0;
		m_sEvarate = 0;
		m_sDaggerAc = 0;
		m_JamadarAc = 0;
		m_sSwordAc = 0;
		m_sClubAc = 0;
		m_sAxeAc = 0;
		m_sSpearAc = 0;
		m_sBowAc = 0;
		m_bFireDamage = 0;
		m_bIceDamage = 0;
		m_bLightningDamage = 0;
		m_bPoisonDamage = 0;
		m_bHPDrain = 0;
		m_bMPDamage = 0;
		m_bMPDrain = 0;
		m_bMirrorDamage = 0;
		m_DropRate = 0;
		m_sStrB = 0;
		m_sStaB = 0;
		m_sDexB = 0;
		m_sIntelB = 0;
		m_sChaB = 0;
		m_MaxHpB = 0;
		m_MaxMpB = 0;
		m_bFireR = 0;
		m_bColdR = 0;
		m_bLightningR = 0;
		m_bMagicR = 0;
		m_bPoisonR = 0;
		m_bCurseR = 0;
		ItemClass = 0;
		m_iNPBuyPrice = 0;
		m_Bound = 0;
		m_iSellNpcType = 0;
		m_iSellNpcPrice = 0;
	}

	INLINE bool isnull() { return m_iNum == 0; }
	INLINE bool isStackable() { return m_bCountable != 0; }

	INLINE uint8 GetKind() { return m_bKind; }
	INLINE uint8 Gettype() { return m_ItemType; }
	INLINE uint32 Getnum() { return m_iNum; }

	INLINE uint8 GetItemGroup() { return uint8(m_bKind); }
	INLINE bool isDagger() { return GetKind() == WEAPON_KIND_DAGGER; }
	INLINE bool isJamadar() { return GetKind() == WEAPON_KIND_JAMADHAR; }

	INLINE bool isSword() { return GetKind() == WEAPON_KIND_1H_SWORD || GetKind() == WEAPON_KIND_2H_SWORD; }
	INLINE bool is2HSword() { return GetKind() == WEAPON_KIND_2H_SWORD; }

	INLINE bool isAxe() { return GetKind() == WEAPON_KIND_1H_AXE || GetKind() == WEAPON_KIND_2H_AXE; }
	INLINE bool is2HAxe() { return GetKind() == WEAPON_KIND_2H_AXE; }

	INLINE bool isClub() { return GetKind() == WEAPON_KIND_1H_CLUP || GetKind() == WEAPON_KIND_2H_CLUP; }
	INLINE bool is2HClub() { return GetKind() == WEAPON_KIND_2H_CLUP; }

	INLINE bool isSpear() { return GetKind() == WEAPON_KIND_1H_SPEAR || GetKind() == WEAPON_KIND_2H_SPEAR; }
	INLINE bool is2HSpear() { return GetKind() == WEAPON_KIND_2H_SPEAR; }

	INLINE bool isShield() { return GetKind() == WEAPON_SHIELD; }
	INLINE bool isStaff() { return GetKind() == WEAPON_KIND_STAFF; }
	INLINE bool isBow() { return GetKind() == WEAPON_KIND_BOW || GetKind() == WEAPON_KIND_CROSSBOW; }
	INLINE bool isCrossBow() { return GetKind() == WEAPON_KIND_CROSSBOW; }
	INLINE bool isMace() { return GetKind() == WEAPON_KIND_MACE; }


	INLINE bool isPickaxe() { return GetKind() == WEAPON_PICKAXE; }
	INLINE bool isPetItem() { return GetKind() == ITEM_KIND_PET_ITEM; }
	INLINE bool isPunishmentStick() { return Getnum() == 900356000; }
	INLINE bool isCyhperRing() { return GetKind() == 160; }
	INLINE bool isTimingFlowTyon() { return Getnum() == 900335523; }
	INLINE bool isTimingFlowMeganthereon() { return Getnum() == 900336524; }
	INLINE bool isTimingFlowHellHound() { return Getnum() == 900337525; }

	//Xtreme 
	INLINE bool isMiningItem() { return GetKind() == WEAPON_PICKAXE; }
	INLINE bool isFishingItem() { return GetKind() == WEAPON_FISHING; }

	INLINE bool isTimingDelay()
	{
		return
			Getnum() == 900335523 ||
			Getnum() == 900336524 ||
			Getnum() == 900337525;
	}

	INLINE bool isWirinomUniqDelay()
	{
		return
			(Getnum() >= 127410731 && Getnum() <= 127410740) ||
			(Getnum() >= 127420741 && Getnum() <= 127420750) ||
			(Getnum() >= 127430751 && Getnum() <= 127430760) ||
			(Getnum() >= 127440761 && Getnum() <= 127440770) ||
			Getnum() == 127410284 ||
			Getnum() == 127420285 ||
			Getnum() == 127430286 ||
			Getnum() == 127440287;
	}

	INLINE bool isWirinomRebDelay()
	{
		return
			(Getnum() >= 127411181 && Getnum() <= 127411210) ||
			(Getnum() >= 127421211 && Getnum() <= 127421240) ||
			(Getnum() >= 127431241 && Getnum() <= 127431270) ||
			(Getnum() >= 127441271 && Getnum() <= 127441300);
	}

	INLINE bool isDarkKnightMace()
	{
		return
			(Getnum() >= 1111471741 && Getnum() <= 1111471750) ||
			(Getnum() >= 1111479901 && Getnum() <= 1111479930);
	}

	INLINE bool isGargesSwordDelay()
	{
		return
			(Getnum() >= 1110582731 && Getnum() <= 1110582740) ||
			Getnum() == 1110582451;
	}

	INLINE bool isFishing() { return GetKind() == WEAPON_FISHING; }
	INLINE bool isRON() { return Getnum() == 189401287 || Getnum() == 189401288 || Getnum() == 189401289; }
	INLINE bool isLigh() { return Getnum() == 189301277 || Getnum() == 189301278 || Getnum() == 189301279; }
	INLINE bool isNormal() { return Gettype() == 4 || Gettype() == 5; }
	INLINE bool isRebirth1() { return Gettype() == 11 || Gettype() == 12; }
	INLINE bool isAll() { return GetKind() > 0; }
	INLINE bool is2Handed() { return m_bSlot == ItemSlot2HLeftHand || m_bSlot == ItemSlot2HRightHand; }
	INLINE bool isAccessory() { return GetKind() == ACCESSORY_EARRING || GetKind() == ACCESSORY_NECKLACE || GetKind() == ACCESSORY_RING || GetKind() == ACCESSORY_BELT; }
	INLINE bool isEarring() { return GetKind() == ACCESSORY_EARRING; }
	INLINE bool isNecklace() { return GetKind() == ACCESSORY_NECKLACE; }
	INLINE bool isRing() { return GetKind() == ACCESSORY_RING; }
	INLINE bool isBelt() { return GetKind() == ACCESSORY_BELT; }
};

struct _LOOT_ITEM
{
	uint32 nItemID; uint16 sCount, slotid;
};

struct _REPURCHASE_ITEM
{
	uint32 nItemID;
	uint32 nPrice;
	time_t nLastTime;

	uint32 iLastDay()
	{
		return (uint32)((((UNIXTIME - nLastTime) / 60) / 60) / 24);
	}

	bool CheckRepurchase()
	{
		if (iLastDay() <= 3)
			return true;

		return false;
	}
};

enum class tagerror
{
	Open,
	List,
	newtag,
	success,
	noitem,
	already,
	error
};

struct _USER_LOOT_SETTING
{
	uint8 iWeapon;
	uint8 iArmor;
	uint8 iAccessory;
	uint8 iNormal;
	uint8 iUpgrade;
	uint8 iCraft;
	uint8 iRare;
	uint8 iMagic;
	uint8 iUnique;
	uint8 iConsumable;
	uint32 iPrice;

	_USER_LOOT_SETTING()
	{
		initiliaze();
	}

	void initiliaze()
	{
		iWeapon = 0;
		iArmor = 0;
		iAccessory = 0;
		iNormal = 0;
		iUpgrade = 0;
		iCraft = 0;
		iRare = 0;
		iMagic = 0;
		iUnique = 0;
		iConsumable = 0;
		iPrice = 0;
	}
};

struct _LOOT_BUNDLE
{
	uint32 nBundleID;
	uint8 ItemsCount;
	uint16 npcid;
	uint16 pLooter;
	std::string strname;
	float x, z, y;
	time_t tDropTime;

	_LOOT_ITEM Items[NPC_HAVE_ITEM_LIST];
	_LOOT_BUNDLE() { Initialize(); }

	void Initialize()
	{
		nBundleID = 0;
		tDropTime = 0;
		pLooter = 0xfff;
		ItemsCount = 0;
		x = 0;
		z = 0;
		y = 0;
		npcid = 0;
		memset(Items, 0, sizeof(Items));
	}
};

struct	_EXCHANGE_ITEM
{
	uint32	nItemID;
	uint32	nCount;
	uint16	sDurability;
	uint8	bSrcPos;
	uint8	bDstPos;
	uint64	nSerialNum;
};

struct _USER_ACHIEVE_SUMMARY
{
	uint32 PlayTime;
	uint32 MonsterDefeatCount;
	uint32 UserDefeatCount;
	uint32 UserDeathCount;
	uint32 TotalMedal;
	uint16 RecentAchieve[3];
	uint16 NormalCount;
	uint16 QuestCount;
	uint16 WarCount;
	uint16 AdvantureCount;
	uint16 ChallangeCount;
	_USER_ACHIEVE_SUMMARY()
	{
		Initialize();
	}

	void Initialize()
	{
		PlayTime = 0;
		MonsterDefeatCount = 0;
		UserDefeatCount = 0;
		UserDeathCount = 0;
		TotalMedal = 0;
		NormalCount = 0;
		QuestCount = 0;
		WarCount = 0;
		AdvantureCount = 0;
		ChallangeCount = 0;
		memset(&RecentAchieve, 0, sizeof(RecentAchieve));
	}
};
struct _UPGRADE_SETTINGS
{
	uint32		nReqItem1;
	uint32		nReqItem2;
	uint8		ItemType;
	uint8		ItemRate;
	uint8		ItemGrade;
	uint32		nReqCoins;
	uint32		SuccesRate;
	uint8		ItemUpgradeStatus;
};
struct _UPGRADE_LOAD
{
	uint32		ItemNumber;
	uint32		NewItemNumber;
	uint32		ScrollNumber;

};
struct _USER_ACHIEVE_INFO
{
	uint8 bStatus;
	uint32 iCount[2];
	_USER_ACHIEVE_INFO()
	{
		bStatus = 1;
		memset(iCount, 0, sizeof(iCount));
	}
};

struct _USER_ACHIEVE_TIMED_INFO
{
	uint32 iExpirationTime;
	_USER_ACHIEVE_TIMED_INFO()
	{
		iExpirationTime = 0;
	}
};

struct _COMMANDER
{
	std::string	strName;
	uint8	bNation;
	uint16  sWarzone;

	_COMMANDER()
	{
		Initialize();
	}

	void Initialize()
	{
		bNation = 0;
		sWarzone = 0;
		strName = "";
	}
};

enum ItemRace
{
	RACE_TRADEABLE_IN_48HR = 19, // These items can't be traded until 48 hours from the time of creation
	RACE_UNTRADEABLE = 20  // Cannot be traded or sold.
};

enum SellType
{
	SellTypeNormal = 0, // sell price is 1/4 of the purchase price
	SellTypeFullPrice = 1, // sell price is the same as the purchase price
	SellTypeNoRepairs = 2  // sell price is 1/4 of the purchase price, item cannot be repaired.
};

struct _BOTTOM_USER_INFO
{
	uint32 SignTime;
	_BOTTOM_USER_INFO()
	{
		Initialize();
	}

	void Initialize()
	{
		SignTime = 0;
	}
};

struct _BOTTOM_USER_LIST
{
	std::recursive_mutex UserListLock;
	std::map <uint16, _BOTTOM_USER_INFO*> m_UserList;

	uint8 GetUserCount()
	{
		UserListLock.lock();
		uint8 size = (uint8)m_UserList.size();
		UserListLock.unlock();
		return size;
	}

	_BOTTOM_USER_LIST()
	{
		Initialize();
	}

	void Initialize()
	{
		UserListLock.lock();
		m_UserList.clear();
		UserListLock.unlock();
	}
};

struct _giveitempusinfo
{
	bool pusrefund;
	uint8 buytype;
	uint32 price;

	_giveitempusinfo(bool pusrefund, uint8 buytype, uint32 price)
	{
		this->pusrefund = pusrefund;
		this->buytype = buytype;
		this->price = price;
	}
};


struct _ZONE_SERVERINFO
{
	short		sServerNo;
	std::string	strServerIP;

	_ZONE_SERVERINFO()
	{
		sServerNo = 0;
	}
};

struct _KNIGHTS_CAPE
{
	uint16	sCapeIndex;
	uint32	nReqCoins;
	uint32	nReqClanPoints;	// clan point requirement
	uint8	byGrade;		// clan grade requirement
	uint8	byRanking;		// clan rank requirement (e.g. royal, accredited, etc)
	std::string	Description;
	uint8 Type;
	uint8 Ticket;
	uint8 BonusType;

	_KNIGHTS_CAPE()
	{
		Initialize();
	}

	void Initialize()
	{
		sCapeIndex = 0;
		nReqCoins = 0;
		nReqClanPoints = 0;
		byGrade = 0;
		byRanking = 0;
		Type = 0;
		Ticket = 0;
		BonusType = 0;
	}
};

struct _KNIGHTS_SIEGE_WARFARE
{
	uint16	sCastleIndex;
	uint16	sMasterKnights;
	uint8	bySiegeType;
	uint8	byWarDay;
	uint8	byWarTime;
	uint8	byWarMinute;
	uint16	sChallengeList_1;
	uint16	sChallengeList_2;
	uint16	sChallengeList_3;
	uint16	sChallengeList_4;
	uint16	sChallengeList_5;
	uint16	sChallengeList_6;
	uint16	sChallengeList_7;
	uint16	sChallengeList_8;
	uint16	sChallengeList_9;
	uint16	sChallengeList_10;
	uint8	byWarRequestDay;
	uint8	byWarRequestTime;
	uint8	byWarRequestMinute;
	uint8	byGuerrillaWarDay;
	uint8	byGuerrillaWarTime;
	uint8	byGuerrillaWarMinute;
	std::string	strChallengeList;
	uint16	sMoradonTariff;
	uint16	sDellosTariff;
	int32	nDungeonCharge;
	int32	nMoradonTax;
	int32	nDellosTax;
	uint16	sRequestList_1;
	uint16	sRequestList_2;
	uint16	sRequestList_3;
	uint16	sRequestList_4;
	uint16	sRequestList_5;
	uint16	sRequestList_6;
	uint16	sRequestList_7;
	uint16	sRequestList_8;
	uint16	sRequestList_9;
	uint16	sRequestList_10;

	_KNIGHTS_SIEGE_WARFARE()
	{
		Initialize();
	}

	void Initialize()
	{
		sCastleIndex = 0;
		sMasterKnights = 0;
		bySiegeType = 0;
		byWarDay = 0;
		byWarTime = 0;
		byWarMinute = 0;
		sChallengeList_1 = 0;
		sChallengeList_2 = 0;
		sChallengeList_3 = 0;
		sChallengeList_4 = 0;
		sChallengeList_5 = 0;
		sChallengeList_6 = 0;
		sChallengeList_7 = 0;
		sChallengeList_8 = 0;
		sChallengeList_9 = 0;
		sChallengeList_10 = 0;
		byWarRequestDay = 0;
		byWarRequestTime = 0;
		byWarRequestMinute = 0;
		byGuerrillaWarDay = 0;
		byGuerrillaWarTime = 0;
		byGuerrillaWarMinute = 0;
		sMoradonTariff = 0;
		sDellosTariff = 0;
		nDungeonCharge = 0;
		nMoradonTax = 0;
		nDellosTax = 0;
		sRequestList_1 = 0;
		sRequestList_2 = 0;
		sRequestList_3 = 0;
		sRequestList_4 = 0;
		sRequestList_5 = 0;
		sRequestList_6 = 0;
		sRequestList_7 = 0;
		sRequestList_8 = 0;
		sRequestList_9 = 0;
		sRequestList_10 = 0;
	}
};

struct _KNIGHTS_ALLIANCE
{
	uint16	sMainAllianceKnights;
	uint16	sSubAllianceKnights;
	uint16	sMercenaryClan_1;
	uint16	sMercenaryClan_2;
	std::string strAllianceNotice;
	_KNIGHTS_ALLIANCE()
	{
		strAllianceNotice = "";
		sMainAllianceKnights = 0;
		sSubAllianceKnights = 0;
		sMercenaryClan_1 = 0;
		sMercenaryClan_2 = 0;
	}
};

struct _START_POSITION
{
	uint16	ZoneID;
	uint16	sKarusX;
	uint16	sKarusZ;
	uint16	sElmoradX;
	uint16	sElmoradZ;
	uint16	sKarusGateX;
	uint16	sKarusGateZ;
	uint16	sElmoradGateX;
	uint16	sElmoradGateZ;
	uint8	bRangeX;
	uint8	bRangeZ;
};

struct _NPC_BOSS_SET
{
	bool isBoss;
	int16 StageID;
	_NPC_BOSS_SET() { Initialize(); }
	void Initialize()
	{
		isBoss = false;
		StageID = 0;
	}
};

struct _MONSTER_BOSS_RANDOM_SPAWN
{
	uint16 nIndex, Stage, MonsterID, PosX, PosZ, Range, ReloadTime;
	uint8  MonsterZone;

	INLINE bool isnull() { return nIndex == 0; }

	_MONSTER_BOSS_RANDOM_SPAWN()
	{
		nIndex = MonsterID = Stage = PosX = PosZ = Range = ReloadTime = 0;
		MonsterZone = 0;
	}
};

struct _MONSTER_BOSS_RANDOM_STAGES
{
	uint16 Stage;
	uint16 MonsterID;
	uint8 MonsterZone;
};

struct _KNIGHTS_RATING
{
	uint32 nRank;
	uint16 sClanID;
	uint32 nPoints;
};

struct _USER_RANK
{
	uint16	nRank;  // shIndex for USER_KNIGHTS_RANK
	uint16	sKnights;
	std::string strUserName;
	std::string strClanName;
	std::string strRankName;
	uint32	nSalary; // nMoney for USER_KNIGHTS_RANK
	uint32	nLoyalty; // nKarusLoyaltyMonthly/nElmoLoyaltyMonthly for USER_PERSONAL_RANK

	_USER_RANK()
	{
		Initialize();
	}

	void Initialize()
	{
		nRank = 0;
		sKnights = 0;
		nSalary = 0;
		nLoyalty = 0;
	}
};

struct _PREMIUM_DATA
{
	uint8	bPremiumType;
	uint32	iPremiumTime;

	_PREMIUM_DATA()
	{
		bPremiumType = 0;
		iPremiumTime = 0;
	}
};

struct _DRAKI_TOWER_ROOM_USER_LIST
{
	uint16 m_DrakiRoomID;
	std::string strUserID;
	_DRAKI_TOWER_ROOM_USER_LIST()
	{
		Initialize();
	}

	void Initialize()
	{
		m_DrakiRoomID = 0;
	}
};

struct _DRAKI_TOWER_INFO
{
	CSTLMap <_DRAKI_TOWER_ROOM_USER_LIST, uint16> m_UserList;
	uint16  m_tDrakiRoomID;
	time_t  m_tDrakiTimer;
	time_t	m_tDrakiSubTimer;
	time_t	m_tDrakiOutTimer;
	time_t  m_tDrakiTownOutTimer;
	bool	m_bOutTimer;
	bool	m_bTownOutTimer;
	bool	m_isDrakiStageChange;
	bool    m_tDrakiTowerStart;
	uint8	m_bSavedDrakiMaxStage;
	uint8	m_bSavedDrakiStage;
	bool    m_bTownRequest;
	uint32	m_bSavedDrakiTime;
	uint8	m_bSavedDrakiLimit;
	uint16	m_sDrakiStage, m_sDrakiSubStage, m_sDrakiTempStage, m_sDrakiTempSubStage;
	uint16  m_bDrakiMonsterKill;
	uint32	m_tDrakiSpareTimer;
	uint32  m_tDrakiRoomCloseTimer;

	_DRAKI_TOWER_INFO()
	{
		Initialize();
	}

	uint8 GetRoomuserCount()
	{
		return m_UserList.GetSize();
	}

	void Initialize()
	{
		m_tDrakiRoomID = 0;
		m_tDrakiTimer = UNIXTIME;
		m_tDrakiSubTimer = UNIXTIME;
		m_tDrakiOutTimer = UNIXTIME;
		m_tDrakiTownOutTimer = UNIXTIME;
		m_bSavedDrakiMaxStage = 0;
		m_bSavedDrakiStage = 0;
		m_bTownRequest = false;
		m_bSavedDrakiTime = 0;
		m_bSavedDrakiLimit = 0;
		m_sDrakiStage = 0;
		m_sDrakiSubStage = 0;
		m_sDrakiTempStage = 0;
		m_sDrakiTempSubStage = 0;
		m_bDrakiMonsterKill = 0;
		m_tDrakiSpareTimer = 0;
		m_tDrakiRoomCloseTimer = 7200;
		m_tDrakiTowerStart = false;
		m_bOutTimer = false;
		m_bTownOutTimer = false;
		m_isDrakiStageChange = false;
	}
};

struct DRAKI_MONSTER_LIST
{
	uint32 nIndex;
	uint8  bDrakiStage;
	uint16 sMonsterID;
	uint16 sPosX;
	uint16 sPosZ;
	uint16 sDirection;
	uint8  bMonster;
};

struct DRAKI_ROOM_LIST
{
	uint8 bIndex;
	uint8 bDrakiStage;
	uint8 bDrakiSubStage;
	uint8 bDrakiNpcStage;
	uint8 MonsterKillCount;

	void Initialize()
	{
		MonsterKillCount = 0;
	}
};

struct DRAKI_ROOM_RANK
{
	uint32 bIndex;
	std::string strUserID;
	uint8 bStage;
	uint32 FinishTime;
	uint8 MaxStage;
	uint8 EnteranceLimit;
};

// Achiement System
struct _ACHIEVE_TITLE
{
	uint32	sIndex;
	std::string strName;
	uint16 Str;
	uint16 Hp;
	uint16 Dex;
	uint16 Int;
	uint16 Mp;
	uint16 Attack;
	int16 Defence;
	uint16 sLoyaltyBonus;
	uint16 sExpBonus;
	uint16 sShortSwordAC;
	uint16 sJamadarAC;
	uint16 sSwordAC;
	uint16 sBlowAC;
	uint16 sAxeAC;
	uint16 sSpearAC;
	uint16 sArrowAC;
	uint16 sFireBonus;
	uint16 sIceBonus;
	uint16 sLightBonus;
	uint16 sFireResist;
	uint16 sIceResist;
	uint16 sLightResist;
	uint16 sMagicResist;
	uint16 sCurseResist;
	uint16 sPoisonResist;

	_ACHIEVE_TITLE()
	{
		Initialize();
	}

	void Initialize()
	{
		sIndex = 0;
		Str = 0;
		Hp = 0;
		Dex = 0;
		Int = 0;
		Mp = 0;
		Attack = 0;
		Defence = 0;
		sLoyaltyBonus = 0;
		sExpBonus = 0;
		sShortSwordAC = 0;
		sJamadarAC = 0;
		sSwordAC = 0;
		sBlowAC = 0;
		sAxeAC = 0;
		sSpearAC = 0;
		sArrowAC = 0;
		sFireBonus = 0;
		sIceBonus = 0;
		sLightBonus = 0;
		sFireResist = 0;
		sIceResist = 0;
		sLightResist = 0;
		sMagicResist = 0;
		sCurseResist = 0;
		sPoisonResist = 0;
	}
};

struct _ACHIEVE_MAIN
{
	uint32	sIndex;
	uint8	Type;
	uint16 TitleID;
	uint16 Point;
	uint32 ItemNum;
	uint32 iCount;
	uint8	ZoneID;
	uint8	unknown2;
	uint8	AchieveType;
	uint16	ReqTime;
	std::string strName;
	std::string strDescription;
	uint8 byte1;
	uint8 byte2;

	_ACHIEVE_MAIN()
	{
		Initialize();
	}

	void Initialize()
	{
		sIndex = 0;
		Type = 0;
		TitleID = 0;
		Point = 0;
		ItemNum = 0;
		iCount = 0;
		ZoneID = 0;
		unknown2 = 0;
		AchieveType = 0;
		ReqTime = 0;
		byte1 = 0;
		byte2 = 0;
	}
};

#define ACHIEVE_MOB_GROUPS		2
#define  ACHIEVE_MOBS_PER_GROUP	4
struct _ACHIEVE_MONSTER
{
	uint32	sIndex;
	uint8 Type;
	uint8 byte;
	uint32	sNum[ACHIEVE_MOB_GROUPS][ACHIEVE_MOBS_PER_GROUP];
	uint32	sCount[ACHIEVE_MOB_GROUPS];
};

struct _ACHIEVE_WAR
{
	uint32	sIndex;
	uint8	Type;
	uint32 Count;
};

struct _ACHIEVE_NORMAL
{
	uint32	sIndex;
	uint16	Type;
	uint32 Count;
};

struct _ACHIEVE_COM
{
	uint32	sIndex;
	uint8	Type;
	uint16  Required1;
	uint16  Required2;
};
// Finish Achiement System

struct _RIMA_LOTTERY_DB
{
	uint32  iNum;
	uint32  nReqItem[5];
	uint32  nReqItemCount[5];
	uint32  nRewardItem[4];
	uint32  UserLimit;
	uint32  EventTime;

	_RIMA_LOTTERY_DB()
	{
		void Initialize();
	}

	void Initialize()
	{
		memset(&nReqItem, 0, sizeof(nReqItem));
		memset(&nReqItemCount, 0, sizeof(nReqItemCount));
		memset(&nRewardItem, 0, sizeof(nRewardItem));
		iNum = 0;
		UserLimit = 0;
		EventTime = 0;
	}
};

#define ITEMS_RIGHT_CLICK_EXCHANGE_GROUP 25
struct _RIGHT_CLICK_EXCHANGE
{
	uint32 nBaseItemID;
	uint32 nItemID[ITEMS_RIGHT_CLICK_EXCHANGE_GROUP];
	uint16 sDurability[ITEMS_RIGHT_CLICK_EXCHANGE_GROUP];
	uint32 nCount[ITEMS_RIGHT_CLICK_EXCHANGE_GROUP];
	uint32 nRentalTime[ITEMS_RIGHT_CLICK_EXCHANGE_GROUP];
	uint8  bFlag[ITEMS_RIGHT_CLICK_EXCHANGE_GROUP];
	uint8 bStatus;
	uint8 bSelection;
	_RIGHT_CLICK_EXCHANGE()
	{
		memset(&nItemID, 0, sizeof(nItemID));
		memset(&sDurability, 0, sizeof(sDurability));
		memset(&nCount, 0, sizeof(nCount));
		memset(&nRentalTime, 0, sizeof(nRentalTime));
		memset(&bFlag, 0, sizeof(bFlag));
		nBaseItemID = 0;
		bSelection = 0;
		bStatus = 0;
	}
};

struct _RIMA_LOOTERY_USER_INFO
{
	int16 GetID;
	std::string Name;
	uint32 JoinCounterID;
	uint32 TicketCount;
	bool isGift;
	_RIMA_LOOTERY_USER_INFO()
	{
		void Initialize();
	}

	void Initialize()
	{
		isGift = false;
		GetID = -1;
		JoinCounterID = 0;
		Name.clear();
		TicketCount = 0;
	}
};

struct _RIMA_LOTTERY_PROCESS
{
	bool LotteryStart;
	bool TimerControl;
	bool SendGitfActivate;
	uint32 nReqItem[5];
	uint32 nReqItemCount[5];
	uint32 nRewardItem[4];
	uint32 UserLimit;
	uint32 EventStartTime;
	uint32 EventProcessTime;
	uint32 EventTime;
	uint32 UserJoinCounter;

	_RIMA_LOTTERY_PROCESS()
	{
		void Initialize();
	}

	void Initialize()
	{
		LotteryStart = TimerControl = SendGitfActivate = false;
		memset(nReqItem, 0, sizeof(nReqItem));
		memset(nReqItemCount, 0, sizeof(nReqItemCount));
		memset(nReqItemCount, 0, sizeof(nReqItemCount));
		UserLimit = 0;
		EventStartTime = 0;
		EventProcessTime = 0;
		EventTime = 0;
		UserJoinCounter = 0;
	}
};


struct _DUNGEON_DEFENCE_MONSTER_LIST
{
	uint32	ID;
	uint8   Diffuculty;
	uint16  MonsterID;
	uint16  sCount;
	uint16  PosX;
	uint16  PosZ;
	uint16  sDirection;
	uint8   isMonster;
	uint16  sRadiusRange;
};

struct _DUNGEON_DEFENCE_STAGE_LIST
{
	uint32 ID;
	uint8 Difficulty;
	std::string DifficultyName;
	uint8 sStageID;

	_DUNGEON_DEFENCE_STAGE_LIST()
	{
		Initialize();
	}

	void Initialize()
	{
		ID = 0;
		Difficulty = 0;
		sStageID = 0;
	}
};

struct _DUNGEON_DEFENCE_ROOM_USER_LIST
{
	uint16 m_DefenceRoomID;
	std::string strUserID[MAX_PARTY_USERS];

	_DUNGEON_DEFENCE_ROOM_USER_LIST()
	{
		Initialize();
	}

	void Initialize()
	{
		m_DefenceRoomID = 0;
	}
}; //CTSLMApv2

struct _DUNGEON_DEFENCE_ROOM_INFO
{
	CSTLMap <_DUNGEON_DEFENCE_ROOM_USER_LIST, uint16> m_UserList;
	uint16 m_DefenceRoomID;
	uint16 m_DefenceKillCount;
	uint32 m_DefenceSpawnTime;
	uint32 m_DefenceRoomClose;
	uint32 m_DefenceOutTime;
	uint8  m_DefenceStageID;
	uint8  m_DefenceDifficulty;
	bool   m_DefenceisStarted;
	bool   m_DefenceMonsterBeginnerSpawned;
	bool   m_DefenceMonsterSpawned;
	bool   m_DefenceisFinished;


	_DUNGEON_DEFENCE_ROOM_INFO()
	{
		Initialize();
	}

	uint8 GetRoomuserCount()
	{
		return m_UserList.GetSize();
	}

	void Initialize()
	{
		m_UserList.DeleteAllData();
		m_DefenceRoomID = 0;
		m_DefenceKillCount = 0;
		m_DefenceSpawnTime = 60;
		m_DefenceRoomClose = 7200;
		m_DefenceOutTime = 0;
		m_DefenceStageID = 0;
		m_DefenceDifficulty = 0;
		m_DefenceisStarted = false;
		m_DefenceMonsterSpawned = false;
		m_DefenceMonsterBeginnerSpawned = false;
		m_DefenceisFinished = false;
	}
};

// TODO: Rewrite this system to be less script dependent for exchange logic.
// Coin requirements should be in the database, and exchanges should be grouped.
#define ITEMS_IN_ORIGIN_GROUP 5
#define ITEMS_IN_EXCHANGE_GROUP 5

#define ITEMS_IN_ROB_ITEM_GROUP_LUA 25
#define ITEMS_IN_GIVE_ITEM_GROUP_LUA 25

#define ITEMS_SPECIAL_EXCHANGE_GROUP 10
#define EVENT_START_TIMES 5

struct _MINING_EXCHANGE
{
	uint16  nIndex;
	uint16  sNpcID;
	uint8   GiveEffect;
	uint8   OreType;
	std::string nOriginItemName;
	uint32 nOriginItemNum;
	std::string nGiveItemName;
	uint32 nGiveItemNum;
	uint16 nGiveItemCount;
	uint32 SuccesRate;

	_MINING_EXCHANGE()
	{
		Initialize();
	}

	void Initialize()
	{
		nIndex = 0;
		sNpcID = 0;
		GiveEffect = 0;
		OreType = 0;
		nOriginItemNum = 0;
		nGiveItemNum = 0;
		nGiveItemCount = 0;
		SuccesRate = 0;
	}
};

enum EventType
{
	LunarWar = 1,
	VirtualRoom = 2,
	SingleRoom = 3
};

enum EventLocalID
{
	CastleSiegeWar = 1,
	NapiesGorge,
	AlseidsPrairie,
	NiedsTriangle,
	NereidsIsland,
	Zipang,
	Oreads,
	SnowWar,
	BorderDefenceWar,
	ChaosExpansion,
	JuraidMountain,
	UnderTheCastle,
	ForgettenTemple,
	BeefEvent
};

struct _EVENTTIMER_SHOWLIST
{
	std::string name, days;
	uint32 hour, minute, id;
	bool status;
};

struct _BEEF_EVENT_PLAY_TIMER
{
	uint8 TableEventLocalID;
	uint16 EventZoneID;
	std::string EventName;
	uint32 MonumentTime;
	uint32 LoserSignTime;
	uint32 FarmingTime;

	_BEEF_EVENT_PLAY_TIMER()
	{
		Initialize();
	}

	void Initialize()
	{
		TableEventLocalID = 0;
		EventZoneID = 0;
		MonumentTime = 0;
		LoserSignTime = 0;
		FarmingTime = 0;
	}
};

struct _EVENT_BRIDGE
{
	bool  isBridgeSystemActive, isBridgeTimerControl1, isBridgeTimerControl2, isBridgeTimerControl3;
	uint32 isBridgeSystemStartMinutes;

	_EVENT_BRIDGE()
	{
		Initialize();
	}
	void Initialize()
	{
		isBridgeSystemActive = isBridgeTimerControl1 = isBridgeTimerControl2 = isBridgeTimerControl3 = false;
		isBridgeSystemStartMinutes = 0;
	}
};

struct _BEEF_EVENT_STATUS
{
	bool    isActive;
	bool    isAttackable;
	bool    isMonumentDead;
	bool    isFarmingPlay;
	uint8   WictoryNation;
	uint32  BeefSendTime;
	uint32  BeefMonumentStartTime;
	uint32  BeefFarmingPlayTime;
	uint32  LoserNationSignTime;
	bool    isLoserSign;
};

struct _GIVE_LUA_ITEM_EXCHANGE
{
	uint32	nExchangeID;

	uint32	nRobItemID[25];
	uint32	nRobItemCount[25];

	uint32	nGiveItemID[25];
	uint32	nGiveItemCount[25];

	uint32  nGiveItemTime[25];
};

// 17.05.2020 start
struct _RANDOM_ITEM
{
	uint16 Number;
	uint32 ItemID;
	uint32 ItemCount;
	uint16 nRentalTime;
	uint8 SessionID;
	uint8 Status;




};
// 17.05.2020 end

// 17.05.2020 [SendMessage]
struct _SEND_MESSAGE
{
	int id;
	std::string strMessage;
	std::string strSender;
	uint16 bChatType;
	uint16 SendType;

	_SEND_MESSAGE()
	{
		Initialize();
	}

	void Initialize()
	{
		id = 0;
		bChatType = 0;
		SendType = 0;
	}
};
// 17.05.2020 [SendMessage]

struct _RIGHT_TOP_TITLE_MSG
{
	int id;
	std::string strMessage;
	std::string strTitle;
	_RIGHT_TOP_TITLE_MSG()
	{
		Initialize();
	}

	void Initialize()
	{
		id = 0;
	}

};

struct _TOPLEFT
{
	uint32_t sIndex;
	uint8_t Facebook, Discord, Live;
	std::string FacebookURL, DiscordURL, LiveURL, ResellerURL;

	_TOPLEFT()
	{
		Initialize();
	}

	void Initialize()
	{
		sIndex = 0;
		Facebook = Discord = Live = 0;
		FacebookURL.clear();
		ResellerURL.clear();
		DiscordURL.clear();
		LiveURL.clear();
	}
};

struct _ITEM_EXCHANGE
{
	uint32	nIndex;
	uint8	bRandomFlag;

	uint32	nOriginItemNum[ITEMS_IN_ORIGIN_GROUP];
	uint32	sOriginItemCount[ITEMS_IN_ORIGIN_GROUP];

	uint32	nExchangeItemNum[ITEMS_IN_EXCHANGE_GROUP];
	uint32	sExchangeItemCount[ITEMS_IN_EXCHANGE_GROUP];
	uint8	sExchangeItemTime[ITEMS_IN_EXCHANGE_GROUP];
};

struct _DKM_MONSTER_DEAD_GIFT
{
	uint8  iNum;
	uint32 nGiveItemNum;
	std::string nGiveItemName;
	uint16 nGiveItemCount;
	uint32 nGiveItemPercent;

	_DKM_MONSTER_DEAD_GIFT()
	{
		Initialize();
	}

	void Initialize()
	{
		iNum = 0;
		nGiveItemNum = 0;
		nGiveItemCount = 0;
		nGiveItemPercent = 0;
	}
};

struct _ITEM_EXCHANGE_EXP
{
	uint32	nIndex;
	uint8	bRandomFlag;
	uint32	nExchangeItemNum[ITEMS_IN_EXCHANGE_GROUP];
	uint32	sExchangeItemCount[ITEMS_IN_EXCHANGE_GROUP];
	uint8	sExchangeItemTime[ITEMS_IN_EXCHANGE_GROUP];
};

struct SPECIAL_PART_SEWING_EXCHANGE
{
	uint32	nIndex;
	uint16	sNpcNum;
	uint32	nReqItemID[ITEMS_SPECIAL_EXCHANGE_GROUP];
	uint32	nReqItemCount[ITEMS_SPECIAL_EXCHANGE_GROUP];
	uint32	nGiveItemID;
	uint32	nGiveItemCount;
	uint32  iSuccessRate;
	uint8	isNotice;
	uint8   isShadowSucces;

	INLINE bool isnull() { return nIndex == 0 || nGiveItemID == 0; }

	SPECIAL_PART_SEWING_EXCHANGE()
	{
		nIndex = 0;
		sNpcNum = 0;
		memset(nReqItemID, 0, sizeof(nReqItemID));
		memset(nReqItemCount, 0, sizeof(nReqItemCount));
		nGiveItemID = 0;
		nGiveItemCount = 0;
		iSuccessRate = 0;
		isNotice = 0;
		isShadowSucces = 0;
	}
};

struct _ITEM_EXCHANGE_CRASH
{
	uint32 nIndex;
	uint32 nItemID;
	uint8  nCount;
	uint16 sRate;

	INLINE bool isnull() { return nIndex == 0; }
	_ITEM_EXCHANGE_CRASH()
	{
		nIndex = nItemID = 0;
		nCount = 0;
		sRate = 0;
	}
};

struct _ITEM_PREMIUM_GIFT
{
	uint32	m_iItemNum;
	uint16	sCount;
	std::string strMessage;
	std::string strSubject;
	std::string strSender;

	_ITEM_PREMIUM_GIFT()
	{
		Initialize();
	}

	void Initialize()
	{
		m_iItemNum = 0;
		sCount = 0;
	}
};

struct _ITEM_SELLTABLE
{
	uint32_t nIndex, iSellingGroup, ItemIDs[24];

	_ITEM_SELLTABLE()
	{
		nIndex = iSellingGroup = 0;
		memset(ItemIDs, 0, sizeof(ItemIDs));
	}
};

struct _ZONE_ONLINE_REWARD
{
	uint8 bZoneID;
	uint32 itemid, itemcount, itemtime, minute, loyalty, cash, tl;
	time_t usingtime;

	uint32 pre_itemid, pre_itemcount, pre_itemtime, pre_minute, pre_loyalty, pre_cash, pre_tl;
};

#define MAX_ITEMS_REQ_FOR_UPGRADE 8
struct _ITEM_UPGRADE
{
	uint32	nIndex;
	uint16	sNpcNum;
	int8	bOriginType;
	uint32	sOriginItem;
	uint32	nReqItem[MAX_ITEMS_REQ_FOR_UPGRADE];
	uint32	nReqNoah;
	uint8	bRateType;
	uint16	sGenRate;
	uint16	sTrinaRate, sKarivdisRate; //sTrinaRate, sKarivdisRate yeni eklendi 22.04.2020
	int32	nGiveItem;

	INLINE uint32 Getscroll() { return nReqItem[MAX_ITEMS_REQ_FOR_UPGRADE]; }
	INLINE bool isReverse() { return Getscroll() == (uint32)379257000; }
	INLINE bool isTransform() { return Getscroll() == (uint32)379256000; }
};



enum class ItemTriggerType
{
	TriggerTypeAttack = 3,
	TriggerTypeDefend = 13
};

struct _ITEM_OP
{
	uint32	nItemID;
	uint8	bTriggerType;
	uint32	nSkillID;
	uint8	bTriggerRate;
};

struct _CAPE_CASTELLAN_BONUS
{
	uint32 Type;
	uint16 HPBonus;
	uint16 MPBonus;
	uint16 StrengthBonus;
	uint16 StaminaBonus;
	uint16 DexterityBonus;
	uint16 IntelBonus;
	uint16 CharismaBonus;
	uint16 FlameResistance;
	uint16 GlacierResistance;
	uint16 LightningResistance;
	uint16 PoisonResistance;
	uint16 MagicResistance;
	uint16 CurseResistance;
	uint16 XPBonusPercent;
	uint16 CoinBonusPercent;
	uint16 APBonusPercent;
	uint16 APBonusClassType;
	uint16 APBonusClassPercent;
	uint16 ACBonus;
	uint16 ACBonusClassType;
	uint16 ACBonusClassPercent;
	uint16 MaxWeightBonus;
	uint8 NPBonus;
};
struct _WHEEL_DATA
{
	uint32	nIndex;
	uint32	nItemID;
	uint16	nItemCount;
	uint16	nRentalTime;
	uint8	nFlag;
	uint16	ExchangeRate;

};
struct _SET_ITEM
{
	uint32 SetIndex;

	uint16 HPBonus;
	uint16 MPBonus;
	uint16 StrengthBonus;
	uint16 StaminaBonus;
	uint16 DexterityBonus;
	uint16 IntelBonus;
	uint16 CharismaBonus;
	uint16 FlameResistance;
	uint16 GlacierResistance;
	uint16 LightningResistance;
	uint16 PoisonResistance;
	uint16 MagicResistance;
	uint16 CurseResistance;

	uint16 XPBonusPercent;
	uint16 CoinBonusPercent;

	uint16 APBonusPercent;		// +AP% for all classes
	uint16 APBonusClassType;	// defines a specific class for +APBonusClassPercent% to be used against
	uint16 APBonusClassPercent;	// +AP% for APBonusClassType only

	uint16 ACBonus;				// +AC amount for all classes
	uint16 ACBonusClassType;	// defines a specific class for +ACBonusClassPercent% to be used against
	uint16 ACBonusClassPercent;	// +AC% for ACBonusClassType only

	uint16 MaxWeightBonus;
	uint8 NPBonus;
};

struct _QUEST_HELPER
{
	uint32	nIndex;
	uint8	bMessageType;
	uint8	bLevel;
	uint32	nExp;
	uint8	bClass;
	uint8	bNation;
	uint8	bQuestType;
	uint8	bZone;
	uint16	sNpcId;
	uint16	sEventDataIndex;
	int8	bEventStatus;
	uint32	nEventTriggerIndex;
	uint32	nEventCompleteIndex;
	uint32	nExchangeIndex;
	uint32	nEventTalkIndex;
	std::string strLuaFilename;
	uint32	sQuestMenu;
	uint32	sNpcMain;
	uint8	sQuestSolo;

	_QUEST_HELPER()
	{
		Initialize();
	}

	void Initialize()
	{
		nIndex = 0;
		bMessageType = 0;
		bLevel = 0;
		nExp = 0;
		bClass = 0;
		bNation = 0;
		bQuestType = 0;
		bZone = 0;
		sNpcId = 0;
		sEventDataIndex = 0;
		bEventStatus = 0;
		nEventTriggerIndex = 0;
		nEventCompleteIndex = 0;
		nExchangeIndex = 0;
		nEventTalkIndex = 0;
		sQuestMenu = 0;
		sNpcMain = 0;
		sQuestSolo = 0;
	}
};

struct _QUEST_HELPER_NATION
{
	uint8	bNation;
	uint16	sEventDataIndex;
};

struct _ZINDAN_WAR_STAGES
{
	uint32 nIndex;
	uint8  Type;
	uint8  Stage;
	uint16 Time;
};

struct _ZINDAN_WAR_SUMMON
{
	uint32 bIndex;
	uint8  Type;
	uint8  Stage;
	uint16 SidID;
	uint16 SidCount;
	uint16 PosX;
	uint16 PosZ;
	uint16  Range;

	_ZINDAN_WAR_SUMMON()
	{
		Initialize();
	}

	void Initialize()
	{
		bIndex = 0;
		Type = 0;
		Stage = 0;
		SidID = 0;
		SidCount = 0;
		PosX = 0;
		PosZ = 0;
		Range = 0;
	}
};


struct _USER_SEAL_ITEM
{
	uint64 nSerialNum;
	uint32 nItemID;
	uint8 bSealType, prelockstate;
};

#define QUEST_MOB_GROUPS		4
#define QUEST_MOBS_PER_GROUP	4
struct _QUEST_MONSTER
{
	uint16	sQuestNum;
	uint16	sNum[QUEST_MOB_GROUPS][QUEST_MOBS_PER_GROUP];
	uint16	sCount[QUEST_MOB_GROUPS];

	_QUEST_MONSTER()
	{
		sQuestNum = 0;
		memset(&sCount, 0, sizeof(sCount));
		memset(&sNum, 0, sizeof(sNum));
	}
};

enum SpecialQuestIDs
{
	QUEST_KILL_GROUP1 = 32001,
	QUEST_KILL_GROUP2 = 32002,
	QUEST_KILL_GROUP3 = 32003,
	QUEST_KILL_GROUP4 = 32004,
	QUEST_KILL_GROUP5 = 32005,
	QUEST_KILL_GROUP6 = 32006,
	QUEST_KILL_GROUP7 = 32007,
};

struct _RENTAL_ITEM
{
	uint32	nRentalIndex;
	uint32	nItemID;
	uint16	sDurability;
	uint64	nSerialNum;
	uint8	byRegType;
	uint8	byItemType;
	uint8	byClass;
	uint16	sRentalTime;
	uint32	nRentalMoney;
	std::string strLenderCharID;
	std::string strBorrowerCharID;

	_RENTAL_ITEM()
	{
		Initialize();
	}

	void Initialize()
	{
		nRentalIndex = 0;
		nItemID = 0;
		sDurability = 0;
		nSerialNum = 0;
		byRegType = 0;
		byItemType = 0;
		byClass = 0;
		nRentalMoney = 0;
	}
};

struct _PREMIUM_ITEM
{
	uint8	Type;
	float	ExpRestorePercent;
	uint16	NoahPercent;
	uint16	DropPercent;
	uint32	BonusLoyalty;
	uint16	RepairDiscountPercent;
	uint16	ItemSellPercent;
};

struct _KILLASSISTINFO
{
	uint32 totalkillcount, serikillcount, totalassistcount;
	time_t lastkilltime, lastassisttime;
	_KILLASSISTINFO() { Initialize(); }

	void Initialize()
	{
		totalkillcount = serikillcount = totalassistcount = 0;
		lastkilltime = lastassisttime = 0;
	}
};

struct _CLAN_PREMIUM_ITEM
{
	uint8	Type;
	uint16	ExpRestorePercent;
	uint16	NoahPercent;
	uint16	DropPercent;
	uint32	BonusLoyalty;
	uint16	RepairDiscountPercent;
	uint16	ItemSellPercent;
};

struct _CHAOS_EXPANSION_RANKING
{
	uint16 c_SocketID;
	int16  c_EventRoom;
	uint16 c_KillCount;
	uint16 c_DeathCount;
};

struct _BORDER_DEFENCE_WAR_RANKING
{
	uint16 b_SocketID;
	int16  b_EventRoom;
	uint8  b_Nation;
	uint32 b_UserPoint;
};

struct _PLAYER_KILLING_ZONE_RANKING
{
	uint16 p_SocketID;
	uint16 p_Zone;
	uint8  P_Nation;
	uint32 P_LoyaltyDaily;
	uint16 P_LoyaltyPremiumBonus;
};

struct _ZINDAN_WAR_RANKING
{
	uint16 z_SocketID;
	uint16 z_Zone;
	uint8  z_Nation;
	uint32 z_LoyaltyDaily;
	uint16 z_LoyaltyPremiumBonus;
};

struct _PREMIUM_ITEM_EXP
{
	uint16	nIndex;
	uint8	Type;
	uint8	MinLevel;
	uint8	MaxLevel;
	uint16	sPercent;
};

struct _EVENT_REWARD
{
	uint32 index;
	uint16 local_id;
	bool iswinner, status;
	uint32 itemid[3], itemcount[3], itemexpiration[3];
	uint32 experience, loyalty, cash, noah;

};

struct _SEEKING_PARTY_USER
{
	uint16 m_sSid;
	uint16 m_sClass;
	uint8	isPartyLeader;
	int16	m_sLevel;
	uint8	m_bZone;
	std::string	m_strSeekingNote;
	std::string	m_strUserID;
	uint8	m_bNation;
	uint16 m_sPartyID;
	uint8 m_bSeekType;
	uint8 m_bloginType;

	_SEEKING_PARTY_USER()
	{
		Initialize();
	}

	void Initialize()
	{
		m_sSid = 0;
		m_sClass = 0;
		isPartyLeader = 0;
		m_sLevel = 0;
		m_bZone = 0;
		m_bNation = 0;
		m_sPartyID = 0;
		m_bSeekType = 0;
		m_bloginType = 0;
	}
};

struct _TEMPLE_SOCCER_EVENT_USER
{
	uint16 m_socketID;
	uint8 m_teamColour;
};

struct _KNIGHT_RETURN_GIFT_ITEM
{
	uint8 ID;
	uint32	m_iItemNum;
	uint16	sCount;
	std::string strMessage;
	std::string strSubject;
	std::string strSender;

	_KNIGHT_RETURN_GIFT_ITEM()
	{
		Initialize();
	}

	void Initialize()
	{
		ID = 0;
		m_iItemNum = 0;
		sCount = 0;
	}
};

struct _LOOT_SETTINGS
{
	std::string UserName;
	uint8 iWarrior;
	uint8 iRogue;
	uint8 iMage;
	uint8 iPriest;
	uint8 iWeapon;
	uint8 iArmor;
	uint8 iAccessory;
	uint8 iNormal;
	uint8 iUpgrade;
	uint8 iCraft;
	uint8 iRare;
	uint8 iMagic;
	uint8 iUnique;
	uint8 iConsumable;
	uint32 iPrice;

	_LOOT_SETTINGS()
	{
		Initialize();
	}

	void Initialize()
	{
		iWarrior = 0;
		iRogue = 0;
		iMage = 0;
		iPriest = 0;
		iWeapon = 0;
		iArmor = 0;
		iAccessory = 0;
		iNormal = 0;
		iUpgrade = 0;
		iCraft = 0;
		iRare = 0;
		iMagic = 0;
		iUnique = 0;
		iConsumable = 0;
		iPrice = 0;
	}
};

struct _EVENT_STATUS
{
	EVENT_OPENTIMELIST pTime;
	int16	ActiveEvent;
	int8	ZoneID;
	uint8	LastEventRoom;
	time_t	StartTime, ClosedTime, SignRemainSeconds;
	uint16	AllUserCount, ElMoradUserCount, KarusUserCount;
	uint32  m_tLastWinnerCheck, ManuelClosedTime;
	bool	isAttackable, isActive, bAllowJoin;
	bool    EventTimerStartControl;
	bool    EventTimerAttackOpenControl;
	bool    EventTimerAttackCloseControl;
	bool    EventTimerFinishControl;
	bool    EventTimerResetControl;
	bool    EventCloseMainControl;
	bool    EventManuelClose;
	bool	isAutomatic;
	bool    bridge_active, bridge_t_check[3];
	uint32  bridge_start_min;

	_EVENT_STATUS()
	{
		Initialize();
	}
	void Initialize()
	{
		isAutomatic = 0;
		pTime = EVENT_OPENTIMELIST();
		bridge_active = false;
		memset(bridge_t_check, 0, sizeof(bridge_t_check));
		m_tLastWinnerCheck = 0;
		bridge_start_min = 0;
		ActiveEvent = -1;
		ZoneID = 0;
		LastEventRoom = 1;
		StartTime = 0;
		ClosedTime = 0;
		AllUserCount = 0;
		KarusUserCount = 0;
		ElMoradUserCount = 0;
		ManuelClosedTime = 0;
		SignRemainSeconds = 0;
		isAttackable = false;
		isActive = false;
		bAllowJoin = false;
		EventTimerStartControl = false;
		EventTimerAttackOpenControl = false;
		EventTimerAttackCloseControl = false;
		EventTimerFinishControl = false;
		EventTimerResetControl = false;
		EventCloseMainControl = false;
		EventManuelClose = false;
	}

};

struct _TEMPLE_EVENT_USER
{
	uint32 m_iJoinOrder;
	std::string strUserID;
	int64 reg_hwID;
	_TEMPLE_EVENT_USER()
	{
		void Initialize();
	}

	void Initialize()
	{
		m_iJoinOrder = 0;
		strUserID.clear();
	}
};

struct _TEMPLE_STARTED_EVENT_USER
{
	std::string strUserID;
	bool isPrizeGiven;
	bool isLoqOut;

	_TEMPLE_STARTED_EVENT_USER()
	{
		void initiliaze();
	}

	void initiliaze()
	{
		isPrizeGiven = false;
		isLoqOut = false;
	}
};

struct _JURAID_ROOM_INFO
{
	std::recursive_mutex m_ElmoradUserListLock, m_KarusUserListLock;
	typedef std::map<std::string, _TEMPLE_STARTED_EVENT_USER>  ElmoradUserList, KarusUserList;

	ElmoradUserList m_ElmoradUserList;
	KarusUserList m_KarusUserList;
	uint32 m_iKarusDeathCount, m_iKarusKillCount, m_iElmoradDeathCount, m_iElmoradKillCount;
	uint8  m_iElmoMainMonsterKill, m_iKarusMainMonsterKill, m_iElmoSubMonsterKill, m_iKarusSubMonsterKill;
	uint8  m_bWinnerNation;
	bool   m_bFinished, m_bActive, m_FinishPacketControl, m_sElmoBridges[3], m_sKarusBridges[3];
	time_t  m_tFinishTimeCounter;
	bool MainCloseCheck;

	uint16 pkBridges[3], peBridges[3];

	_JURAID_ROOM_INFO() { Initialize(); }

	uint8 GetRoomKarusUserCount()
	{
		Guard lock(m_KarusUserListLock);
		return (uint8)m_KarusUserList.size();
	}

	uint8 GetRoomElmoradUserCount()
	{
		Guard lock(m_ElmoradUserListLock);
		return (uint8)m_ElmoradUserList.size();
	}

	uint8 GetRoomTotalUserCount()
	{
		return GetRoomKarusUserCount() + GetRoomElmoradUserCount();
	}

	void Initialize(bool reset = false)
	{
		m_ElmoradUserListLock.lock();
		m_ElmoradUserList.clear();
		m_ElmoradUserListLock.unlock();

		m_KarusUserListLock.lock();
		m_KarusUserList.clear();
		m_KarusUserListLock.unlock();

		m_bWinnerNation = 0;
		m_iKarusDeathCount = 0;
		m_iKarusKillCount = 0;
		m_iElmoradDeathCount = 0;
		m_iElmoradKillCount = 0;
		m_iElmoMainMonsterKill = 0;
		m_iKarusMainMonsterKill = 0;
		m_iElmoSubMonsterKill = 0;
		m_iKarusSubMonsterKill = 0;
		m_bFinished = false;
		m_bActive = false;
		MainCloseCheck = false;
		m_tFinishTimeCounter = 0;
		memset(m_sElmoBridges, 0, sizeof(m_sElmoBridges));
		memset(m_sKarusBridges, 0, sizeof(m_sKarusBridges));
		m_FinishPacketControl = false;

		if (!reset)
		{
			memset(pkBridges, 0, sizeof(pkBridges));
			memset(peBridges, 0, sizeof(peBridges));
		}
	}
};

struct _BDW_ROOM_INFO
{
	std::recursive_mutex m_KarusUserListLock, m_ElmoradUserListLock;
	typedef std::map<std::string, _TEMPLE_STARTED_EVENT_USER> KarusUserList, ElmoradUserList;
	KarusUserList m_KarusUserList;
	ElmoradUserList m_ElmoradUserList;
	uint32 m_iKarusKillCount, m_iElmoradKillCount, m_iKarusMonuCount, m_iElmoMonuCount, m_ElmoScoreBoard, m_KarusScoreBoard;
	uint8 m_bWinnerNation, m_iKarusUserCount, m_iElmoUserCount;
	uint16 pAltar;
	bool m_bFinished, m_bActive, m_FinishPacketControl, m_tAltarSpawn;
	time_t m_tFinishTimeCounter, m_tAltarSpawnTimed;

	_BDW_ROOM_INFO() { Initialize(); }

	uint8 GetRoomKarusUserCount()
	{
		Guard lock(m_KarusUserListLock);
		return (uint8)m_KarusUserList.size();
	}

	uint8 GetRoomElmoradUserCount()
	{
		Guard lock(m_ElmoradUserListLock);
		return (uint8)m_ElmoradUserList.size();
	}

	uint8 GetRoomTotalUserCount()
	{
		return GetRoomKarusUserCount() + GetRoomElmoradUserCount();
	}

	void Initialize(bool reset = false)
	{

		m_KarusUserListLock.lock();
		m_KarusUserList.clear();
		m_KarusUserListLock.unlock();

		m_ElmoradUserListLock.lock();
		m_ElmoradUserList.clear();
		m_ElmoradUserListLock.unlock();

		m_bWinnerNation = 0;
		m_iKarusKillCount = 0;
		m_iElmoradKillCount = 0;
		m_iKarusMonuCount = 0;
		m_iElmoMonuCount = 0;
		m_bFinished = false;
		m_bActive = false;
		m_tFinishTimeCounter = 0;
		m_tAltarSpawnTimed = UNIXTIME;
		m_tAltarSpawn = false;
		m_FinishPacketControl = false;
		m_ElmoScoreBoard = 0;
		m_KarusScoreBoard = 0;
		m_iKarusUserCount = 0;
		m_iElmoUserCount = 0;

		if (!reset) { pAltar = 0; }
	}
};

struct _CHAOS_ROOM_INFO
{
	bool icanbeused;
	std::recursive_mutex UserListLock;
	typedef std::map<std::string, _TEMPLE_STARTED_EVENT_USER> UserList;
	UserList m_UserList;
	bool   m_bFinished, m_bActive, m_FinishPacketControl;
	time_t m_tFinishTimeCounter;

	_CHAOS_ROOM_INFO() { Initialize(); }

	uint8 GetRoomTotalUserCount()
	{
		Guard lock(UserListLock);
		return (uint8)m_UserList.size();
	}

	void Initialize()
	{
		UserListLock.lock();
		m_UserList.clear();
		UserListLock.unlock();
		m_bActive = false;
		m_bFinished = false;
		m_tFinishTimeCounter = 0;
		m_FinishPacketControl = false;
	}
};


struct _UNDER_THE_CASTLE_INFO
{
	int16 m_sUtcGateID[3];

	_UNDER_THE_CASTLE_INFO()
	{
		Initialize();
	}

	void Initialize()
	{
		m_sUtcGateID[0] = -1;
		m_sUtcGateID[1] = -1;
		m_sUtcGateID[2] = -1;
	}
};

struct _TOURNAMENT_DATA
{
	uint8 aTournamentZoneID;
	uint16 aTournamentClanNum[2];
	uint16 aTournamentScoreBoard[2];
	uint32 aTournamentTimer;
	uint8 aTournamentMonumentKilled;
	time_t aTournamentOutTimer;
	bool aTournamentisAttackable;
	bool aTournamentisStarted;
	bool aTournamentisFinished;

	_TOURNAMENT_DATA()
	{
		Initialize();
	}

	void Initialize()
	{
		aTournamentZoneID = 0;
		aTournamentClanNum[0] = 0;
		aTournamentClanNum[1] = 0;
		aTournamentScoreBoard[0] = 0;
		aTournamentScoreBoard[1] = 0;
		aTournamentTimer = 0;
		aTournamentMonumentKilled = 0;
		aTournamentOutTimer = UNIXTIME;
		aTournamentisAttackable = false;
		aTournamentisStarted = false;
		aTournamentisFinished = false;
	}
};

struct _CHAOS_STONE_INFO
{
	uint8  sChaosIndex;
	uint16 sChaosID;
	uint16 sZoneID;
	uint8  sRank;
	uint32 sSpawnTime;
	uint8  sMonsterFamily;
	uint16 sBoosID[10];
	uint8  sBoosKilledCount;
	bool isChaosStoneKilled;
	bool isTotalKilledMonster;
	bool isOnResTimer;
	bool ChaosStoneON;

	_CHAOS_STONE_INFO()
	{
		Initialize();
	}

	void Initialize()
	{
		memset(&sBoosID, 0, sizeof(sBoosID));
		sChaosIndex = 0;
		sChaosID = 0;
		sZoneID = 0;
		sRank = 1;
		sSpawnTime = 30;
		sMonsterFamily = 0;
		sBoosKilledCount = 0;
		isChaosStoneKilled = false;
		isTotalKilledMonster = false;
		isOnResTimer = false;
		ChaosStoneON = false;
	}
};

struct _CHAOS_STONE_SUMMON_LIST
{
	uint32	nIndex;
	uint16	ZoneID;
	uint16	sSid;
	uint8	MonsterSpawnFamily;
};

struct _CHAOS_STONE_RESPAWN
{
	uint8	sIndex;
	uint16  sZoneID;
	uint8   isOpen;
	uint8	sRank;
	uint16  sChaosID;
	uint16  sCount;
	uint16  sSpawnX;
	uint16  sSpawnZ;
	uint8   sSpawnTime;
	uint16  sDirection;
	uint16  sRadiusRange;
};

struct _CHAOS_STONE_STAGE
{
	uint8 nIndex;
	uint16 ZoneID;
	uint8 nIndexFamily;
};

struct _SERVER_SETTING
{
	uint16 premiumID, premiumTime;
	uint32 m_perkCoins;
	uint8 MaximumLevelChange, AutoWanted, mutelevel;
	uint8 DropNotice;
	uint8 UpgradeNotice;
	uint8 GmisOnlineNotice; // JstKO GmisOnline ve GmisOffline Gm Giriş Logos Notice Ayarlar Eklendi 30.12.2024
	uint8 OyunUserBotOnlineNotice; // JstKO User&Bot Oyunda Online | Tablo Server_Settings OyunUserBotOnlineNotice Eklendi 1 Açip ve 0 Kapatma Ayarlar 30.12.2024
	uint8 MerchantView;
	uint8 UserMaxUpgradeCount;
	uint8 AutoQuestSkill;
	uint8 AutoUpFasterKatlama;
	uint8 ClanBankWithPremium; //JstKO Clan Bankası Premiumlu veya Premiumsuz Acilsin 15.07.2020
	uint8 AutoRoyalG1; //JstKO 15.08.2020
	uint8 LootandGeniePremium; //JstKO 15.08.2020
	uint8 TradeLevel; //JstKO 15.08.2020
	uint8 MerchantLevel; //JstKO 15.08.2020
	std::string WelcomeMsg;
	uint32 etrafa_itemverid[3], etrafa_itemvercount[3];
	uint8 new_monsterstone;
	bool monsterstone_status;
	short maxplayerhp;
	bool  trashitem, onlinecash;
	uint32 onlinecashtime, flashtime;
	uint16 MinKnightCash;
	uint32 chaoticcoins;
	bool FreeSkillandStat;
	uint8 maxBlessingUp, maxBlessingUpReb;
	uint8 giveGenieHour;
	std::string srcversion;
	_SERVER_SETTING()
	{
		Initialize();
	}
	void Initialize()
	{
		giveGenieHour = 0;
		maxBlessingUp = maxBlessingUpReb = 0;
		premiumID = premiumTime = 0;
		m_perkCoins = 0;
		memset(etrafa_itemverid, 0, sizeof(etrafa_itemverid));
		memset(etrafa_itemvercount, 0, sizeof(etrafa_itemvercount));
		new_monsterstone = 0;
		monsterstone_status = false;
		chaoticcoins = 0;
		maxplayerhp = 0;
		MaximumLevelChange = 83;
		trashitem = onlinecash = false;
		onlinecashtime = flashtime = 0;
		AutoWanted = 0;
		mutelevel = 1;
		MerchantLevel = 1;
		TradeLevel = 1;
		DropNotice = 0;
		UpgradeNotice = 0;
		GmisOnlineNotice = 0; //JstKO GmisOnline ve GmisOffline Gm Giriş Logos Notice Ayarlar Eklendi 30.12.2024 
		OyunUserBotOnlineNotice = 0; //JstKO User&Bot Oyunda Online | Tablo Server_Settings OyunUserBotOnlineNotice Eklendi 1 Açip ve 0 Kapatma Ayarlar 30.12.2024
		FreeSkillandStat = false;
		MerchantView = 0;
		UserMaxUpgradeCount = 30;
		AutoQuestSkill = 0;
		ClanBankWithPremium = 0;
		AutoUpFasterKatlama = 0;
		AutoRoyalG1 = 0;
		LootandGeniePremium = 0;
		MinKnightCash = 0;
		srcversion = "";
		WelcomeMsg.clear();
	}
};
//struct SPECIAL_STONE
//{
//	uint32		nIndex;
//	uint8		ZoneID;
//	uint16      MainNpcID;
//	uint16      NpcID;
//	uint8		sCount;
//	std::string MonsterName;
//};

struct SPECIAL_STONE
{
	uint32		nIndex;
	uint8		ZoneID;
	uint16      MainNpcID;
	uint16      NpcID;
	uint8		sCount;
	std::string MonsterName;
	uint8		NpcMonster;
};

struct _EVENT_TRIGGER
{
	uint32 nIndex;
	uint16 bNpcType;
	uint32 sNpcID;
	uint32 nTriggerNum;
};

struct _USER_DAILY_OP
{
	std::string strUserId;
	int32 ChaosMapTime;
	int32 UserRankRewardTime;
	int32 PersonalRankRewardTime;
	int32 KingWingTime;
	int32 WarderKillerTime1;
	int32 WarderKillerTime2;
	int32 KeeperKillerTime;
	int32 UserLoyaltyWingRewardTime;

	_USER_DAILY_OP()
	{
		Initialize();
	}

	void Initialize()
	{
		ChaosMapTime = 0;
		UserRankRewardTime = 0;
		PersonalRankRewardTime = 0;
		KingWingTime = 0;
		WarderKillerTime1 = 0;
		WarderKillerTime2 = 0;
		KeeperKillerTime = 0;
		UserLoyaltyWingRewardTime = 0;
	}
};
struct _ZONE_KILL_REWARD
{
	uint32		ID;
	uint8		ZoneID;
	uint8		Nation;
	uint8		Party;
	uint32		ItemID;
	uint16		Duration;
	uint32		sCount;
	uint8		sFlag;
	uint8		ItemType;
	uint32		Time;
	uint16		sRate;
	uint8		isBank;
	uint8		Status;
	uint8		KillCount;
	bool		isPartyItem, isPriest;
	uint8 priestrate;
	_ZONE_KILL_REWARD()
	{
		isPriest = false;
		priestrate = 0;
		ID = 0;
		ZoneID = 0;
		Nation = 0;
		Party = 0;
		ItemID = 0;
		Duration = 0;
		sCount = 0;
		sFlag = 0;
		ItemType = 0;
		Time = 0;
		sRate = 0;
		isBank = 0;
		Status = 0;
		KillCount = 0;
		isPartyItem = false;
	}
};
struct _MONUMENT_INFORMATION
{
	uint16 sSid;
	uint16 sNid;
	int32 RepawnedTime;
};

struct _MONSTER_CHALLENGE
{
	uint16 sIndex;
	uint8 bStartTime1;
	uint8 bStartTime2;
	uint8 bStartTime3;
	uint8 bLevelMin;
	uint8 bLevelMax;
};

struct _MINING_FISHING_ITEM
{
	uint32 nIndex;
	uint32 nTableType;
	uint32 nWarStatus;
	uint8 UseItemType;
	std::string nGiveItemName;
	uint32 nGiveItemID;
	uint32 nGiveItemCount;
	uint32 SuccessRate;

	INLINE bool isnull() { return nIndex == 0; }

	_MINING_FISHING_ITEM()
	{
		nIndex = 0;
		nTableType = 0;
		nWarStatus = 0;
		UseItemType = 0;
		nGiveItemID = 0;
		nGiveItemCount = 0;
		SuccessRate = 0;
	}
};

struct _MONSTER_CHALLENGE_SUMMON_LIST
{
	uint16 sIndex;
	uint8 bLevel;
	uint8 bStage;
	uint8 bStageLevel;
	uint16 sTime;
	uint16 sSid;
	uint16 sCount;
	uint16 sPosX;
	uint16 sPosZ;
	uint8 bRange;
};

struct _SOCCER_STATUS
{
	bool	m_SoccerActive;
	bool	m_SoccerTimer;
	int16	m_SoccerSocketID;
	uint16	m_SoccerTimers;
	uint16	m_SoccerTime;
	uint8	m_SoccerRedColour;
	uint8	m_SoccerBlueColour;
	uint8	m_SoccerBlueGool;
	uint8	m_SoccerRedGool;

	bool	isSoccerAktive() { return m_SoccerActive; }
	bool	isSoccerTime() { return m_SoccerTimer; }
};

struct _CR_USER_LIST
{
	uint16 KillCount[3];
	bool isFinish;
	uint8 sStage;
	_CR_USER_LIST()
	{
		isFinish = false;
		for (int i = 0; i < 3; i++)
		{
			KillCount[i] = 0;
		}
		sStage = 0;
	}
};

struct _COLLECTION_RACE_EVENT
{
	CSTLMap < _CR_USER_LIST, std::string> m_userlit;
	bool isCRActive, openrequest;

	uint32 RewardItemID[10], RewardItemCount[10], RewardItemTime[10];
	uint8 RewardItemRate[10];
	uint8 RewardSession[10];

	uint32 EventTime, rankbug;

	uint16 ProtoID[3], KillCount[3], UserLimit, TotalCount;
	uint8 ZoneID, MinLevel, MaxLevel, isRepeat, isClassStatus;
	std::string EventName;
	uint16 KillCountWarrior[3], KillCountRogue[3], KillCountMage[3], KillCountPriest[3];
	_COLLECTION_RACE_EVENT()
	{
		Initialize();
	}

	void Initialize()
	{
		m_userlit.DeleteAllData();
		isCRActive = openrequest = false;
		EventTime = 0;
		memset(&RewardItemID, 0, sizeof(RewardItemID));
		memset(&RewardItemCount, 0, sizeof(RewardItemCount));
		memset(&ProtoID, 0, sizeof(ProtoID));
		memset(&KillCount, 0, sizeof(KillCount));
		memset(&RewardItemRate, 0, sizeof(RewardItemRate));
		memset(&RewardItemTime, 0, sizeof(RewardItemTime));
		memset(&KillCountWarrior, 0, sizeof(KillCountWarrior));
		memset(&KillCountRogue, 0, sizeof(KillCountRogue));
		memset(&KillCountMage, 0, sizeof(KillCountMage));
		memset(&KillCountPriest, 0, sizeof(KillCountPriest));

		for (int i = 0; i < 10; i++)
		{
			RewardItemRate[i] = 0;
			RewardItemTime[i] = 0;
			RewardItemID[i] = 0;
			RewardItemCount[i] = 0;
		}

		for (int i = 0; i < 3; i++)
		{
			ProtoID[i] = 0;
			KillCount[i] = 0;
			KillCountWarrior[i] = 0;
			KillCountRogue[i] = 0;
			KillCountMage[i] = 0;
			KillCountPriest[i] = 0;
		}

		UserLimit = TotalCount = 0;
		ZoneID = MinLevel = MaxLevel = 0;
		EventName = "";
		rankbug = 1;
		isRepeat = isClassStatus = 0;
	}
};

struct _COLLECTION_RACE_EVENT_USER
{
	uint16 KillCount[3];
	bool CheckFinish;

	_COLLECTION_RACE_EVENT_USER()
	{
		Initialize();
	}

	void Initialize()
	{
		for (int i = 0; i < 3; i++)
			KillCount[i] = 0;

		CheckFinish = false;
	}
};

struct _COLLECTION_RACE_EVENT_LIST
{
	uint32_t EventTime, m_EventID;
	uint16_t ProtoID[3], KillCount[3], UserLimit;
	uint32_t RewardItemID[10], RewardItemCount[10];
	uint8_t ZoneID, MinLevel, MaxLevel;
	std::string EventName;
	uint8 RepeatStatus, ClassStatus;
	bool autostart;
	int autohour, autominute;
	std::string days, warriorkill, roguekill, magekill, priestkill;
	_COLLECTION_RACE_EVENT_LIST()
	{
		Initialize();
	}

	void Initialize()
	{
		autostart = false;
		autohour = autominute = -1;
		EventTime = 0;
		memset(&RewardItemID, 0, sizeof(RewardItemID));
		memset(&RewardItemCount, 0, sizeof(RewardItemCount));
		memset(&ProtoID, 0, sizeof(ProtoID));
		memset(&KillCount, 0, sizeof(KillCount));
		for (int i = 0; i < 10; i++)
		{
			RewardItemID[i] = 0;
			RewardItemCount[i] = 0;
		}

		for (int i = 0; i < 3; i++)
		{
			ProtoID[i] = 0;
			KillCount[i] = 0;
		}
		UserLimit = 0;
		ZoneID = MinLevel = MaxLevel = 0;
		EventName = "";
		RepeatStatus = ClassStatus = 0;
		m_EventID = 0;
		days = warriorkill = roguekill = magekill = priestkill = "";
	}
};

struct _START_POSITION_RANDOM
{
	uint16 sIndex;
	uint8 ZoneID;
	uint16 PosX;
	uint16 PosZ;
	uint8 Radius;
};

struct _USER_ITEM
{
	uint32 nItemID;
	std::vector<uint64> nItemSerial;

	_USER_ITEM()
	{
		void Initialize();
	}

	void Initialize()
	{
		nItemID = 0;
		nItemSerial.clear();
	}
};

struct _NATION_DATA
{
	std::string	bCharName;
	uint8		bRace;
	uint8		bFace;
	uint16		sClass;
	uint16		nNum;
	uint32		nHair;
	uint16		sClanID;

	INLINE bool isnull() { return bCharName.length() == 0; }

	_NATION_DATA()
	{
		bRace = 0;
		bFace = 0;
		sClass = 0;
		nNum = 0;
		nHair = 0;
		sClanID = 0;
	}
};


struct _DELETED_ITEM
{
	uint32 nNum, sCount, iDeleteTime;
	uint16 itemduration;
	uint64 serialnum;
	uint8 bflag;
	bool status;
};

struct _BANISH_OF_WINNER
{
	uint32 sira;
	uint16 sSid;
	uint8  sNationID;
	uint8  sZoneID;
	uint16 sPosX;
	uint16 sPosZ;
	uint16 sCount;
	uint16 sRadius;
	uint16 sDeadTime;
};

enum BuffType
{
	BUFF_TYPE_NONE = 0,
	BUFF_TYPE_HP_MP = 1,
	BUFF_TYPE_AC = 2,
	BUFF_TYPE_SIZE = 3,
	BUFF_TYPE_DAMAGE = 4,
	BUFF_TYPE_ATTACK_SPEED = 5,
	BUFF_TYPE_SPEED = 6,
	BUFF_TYPE_STATS = 7,
	BUFF_TYPE_RESISTANCES = 8,
	BUFF_TYPE_ACCURACY = 9,
	BUFF_TYPE_MAGIC_POWER = 10,
	BUFF_TYPE_EXPERIENCE = 11,
	BUFF_TYPE_WEIGHT = 12,
	BUFF_TYPE_WEAPON_DAMAGE = 13,
	BUFF_TYPE_WEAPON_AC = 14,
	BUFF_TYPE_LOYALTY = 15,
	BUFF_TYPE_NOAH_BONUS = 16,
	BUFF_TYPE_PREMIUM_MERCHANT = 17,
	BUFF_TYPE_ATTACK_SPEED_ARMOR = 18,  // Berserker
	BUFF_TYPE_DAMAGE_DOUBLE = 19,  // Critical Point
	BUFF_TYPE_DISABLE_TARGETING = 20,  // Smoke Screen / Light Shock
	BUFF_TYPE_BLIND = 21,  // Blinding (Strafe)
	BUFF_TYPE_FREEZE = 22,  // Freezing Distance
	BUFF_TYPE_INSTANT_MAGIC = 23,  // Instantly Magic
	BUFF_TYPE_DECREASE_RESIST = 24,  // Minor resist
	BUFF_TYPE_MAGE_ARMOR = 25,  // Fire / Ice / Lightning Armor
	BUFF_TYPE_PROHIBIT_INVIS = 26,  // Source Marking
	BUFF_TYPE_RESIS_AND_MAGIC_DMG = 27,  // Elysian Web
	BUFF_TYPE_TRIPLEAC_HALFSPEED = 28,  // Wall of Iron
	BUFF_TYPE_BLOCK_CURSE = 29,  // Counter Curse
	BUFF_TYPE_BLOCK_CURSE_REFLECT = 30,  // Curse Refraction
	BUFF_TYPE_MANA_ABSORB = 31,  // Outrage / Frenzy
	BUFF_TYPE_IGNORE_WEAPON = 32,  // Weapon cancellation
	BUFF_TYPE_VARIOUS_EFFECTS = 33,  // ... whatever the event item grants.
	BUFF_TYPE_PASSION_OF_SOUL = 35,  // Passion of the Soul
	BUFF_TYPE_FIRM_DETERMINATION = 36,  // Firm Determination
	BUFF_TYPE_SPEED2 = 40,  // Cold Wave
	BUFF_TYPE_ARMORED = 41,  // Armored Skin
	BUFF_TYPE_UNK_EXPERIENCE = 42,  // unknown buff type, used for something relating to XP.
	BUFF_TYPE_ATTACK_RANGE_ARMOR = 43,  // Inevitable Murderous
	BUFF_TYPE_MIRROR_DAMAGE_PARTY = 44,  // Minak's Thorn
	BUFF_TYPE_DAGGER_BOW_DEFENSE = 45,  // Eskrima
	BUFF_TYPE_STUN = 47,  // Lighting Skill 
	BUFF_TYPE_FISHING = 48,  // Fishing Skill
	BUFF_TYPE_LOYALTY_AMOUNT = 55,  // Santa's Present
	BUFF_TYPE_NO_RECALL = 150, // "Cannot use against the ones to be summoned"
	BUFF_TYPE_REDUCE_TARGET = 151, // "Reduction" (reduces target's stats, but enlarges their character to make them easier to attack)
	BUFF_TYPE_SILENCE_TARGET = 152, // Silences the target to prevent them from using any skills (or potions)
	BUFF_TYPE_NO_POTIONS = 153, // "No Potion" prevents target from using potions.
	BUFF_TYPE_KAUL_TRANSFORMATION = 154, // Transforms the target into a Kaul (a pig thing), preventing you from /town'ing or attacking, but increases defense.
	BUFF_TYPE_UNDEAD = 155, // User becomes undead, increasing defense but preventing the use of potions and converting all health received into damage.
	BUFF_TYPE_UNSIGHT = 156, // Blocks the caster's sight (not the target's).
	BUFF_TYPE_BLOCK_PHYSICAL_DAMAGE = 157, // Blocks all physical damage.
	BUFF_TYPE_BLOCK_MAGICAL_DAMAGE = 158, // Blocks all magical/skill damage.
	BUFF_TYPE_UNK_POTION = 159, // unknown potion, "Return of the Warrior", "Comeback potion", perhaps some sort of revive?
	BUFF_TYPE_SLEEP = 160, // Zantman (Sandman)
	BUFF_TYPE_INVISIBILITY_POTION = 163, // "Unidentified potion"
	BUFF_TYPE_GODS_BLESSING = 164, // Increases your defense/max HP 
	BUFF_TYPE_HELP_COMPENSATION = 165, // Compensation for using the help system (to help, ask for help, both?)
	BUFF_TYPE_BATTLE_CRY = 171,

	BUFF_TYPE_IMIR_ROARS = 167, // Imirs scream-  Physical damage-free zone is created for 20 seconds within a range of 10m from the user.
	BUFF_TYPE_LOGOS_HORNS = 168, // Logos horns- Magical damage-free zone is created for 20 seconds within a range of 10m from the user.
	BUFF_TYPE_NP_DROP_NOAH = 169, // Increase rates by some value ( NP, Drop, Noah etc.)
	BUFF_TYPE_SNOWMAN_TITI = 170, // SnowMan titi
	BUFF_TYPE_INCREASE_ATTACK = 172, // Attack increased by 5%
	BUFF_TYPE_REWARD_MASK = 180,
	BUFF_TYPE_MAGIC_SPELL = 183,
	BUFF_TYPE_INCREASE_NP = 162, // Np Increase Event Items
	BUFF_TYPE_DRAKEY = 50,  // Drakey's Nemesis 
	BUFF_TYPE_DEVIL_TRANSFORM = 49,  // Kurian/Porutu Devil Transformation
	BUFF_TYPE_GM_BUFF = 46,  // 30 minutes 10 all stats rise
	BUFF_TYPE_BLESS_OF_TEMPLE = 39,  // Bless of Temple (Attack Increase).
	BUFF_TYPE_FRAGMENT_OF_MANES = 52,  // Lost Speed
	BUFF_TYPE_HELL_FIRE_DRAGON = 37,  // Advent of Hellfire Dragon
	BUFF_TYPE_DIVIDE_ARMOR = 53,
	BUFF_TYPE_JACKPOT = 77,
};

enum FlyingSantaOrAngel
{
	FLYING_NONE = 0,
	FLYING_SANTA = 1,
	FLYING_ANGEL = 2
};

enum SurroundingUserOpCode
{
	UserInfo = 1,
	UserInfoDetail = 2,
	UserInfoAll = 3,
	UserInfoShow = 4,
	UserInfoView = 5
};

// Start Achieve System

enum UserAchieveOpcodes
{
	AchieveError = 0,
	AchieveSuccess = 1,
	AchieveGiftRequest = 2,
	AchieveDetailShow = 3,
	AchieveSummary = 4,
	AchieveFailed = 5,
	AchieveStart = 6,
	AchieveStop = 7,
	AchieveChalengeFailed = 8,
	AchieveCountScreen = 9,
	AchieveCoverTitle = 16,
	AchieveSkillTitle = 17,
	AchieveCoverTitleReset = 18,
	AchieveSkillTitleReset = 19

};

enum UserAchieveStatTypes
{
	ACHIEVE_STAT_STR = 0,
	ACHIEVE_STAT_STA = 1,
	ACHIEVE_STAT_DEX = 2,
	ACHIEVE_STAT_INT = 3,
	ACHIEVE_STAT_CHA = 4,
	ACHIEVE_STAT_ATTACK = 5,
	ACHIEVE_STAT_DEFENCE = 6,
	ACHIEVE_STAT_COUNT = 7
};

enum class UserAchieveMainTypes
{
	AchieveWar = 1,
	AchieveMonster = 2,
	AchieveCom = 3,
	AchieveNormal = 4
};

enum class UserAchieveNormalTypes
{
	AchieveBecomeKing = 1,
	AchieveReachNP = 2,
	AchieveReachLevel = 3,
	AchieveBecomeSpecial = 4,
	AchieveKnightContribution = 5,
	AchieveDefeatMons = 8,
	AchieveBecomeKnightMember = 10,
};

enum class UserAchieveComTypes
{
	AchieveRequireQuest = 1,
	AchieveRequireAchieve = 2
};

enum class UserAchieveMonsterTypes
{
	AchieveDefeatMonster = 1,
	AchieveDefeatNpc = 2
};

enum class UserAchieveWarTypes
{
	AchieveDefeatEnemy = 1,
	AchieveTakeRevange = 3,
	AchieveWinWar = 4,
	AchieveWinNationGuardBattle = 5,
	AchieveWinJuraid = 6,
	AchieveWinChaos1 = 7,
	AchieveWinChaos2 = 8,
	AchieveWinChaos3 = 9,
	AchieveWinCSW = 10,
	AchieveKillCountChaos = 11,
	AchieveDefeatMonsterRonarkL = 20,
	AchieveDrakiTowerFinished = 21
};

enum UserAchieveStatus
{
	AchieveChalenngeInComplete = 0,
	AchieveIncomplete = 1,
	AchieveFinished = 4,
	AchieveCompleted = 5
};

// Finish Achieve System

enum ExpSealOpcode
{
	SealExp_ON = 1,
	SealExp_OFF = 2
};

enum NationTransferOpcode
{
	NationWarStatus = 1,
	NationOpenBox = 2,
	NationSuccessTransfer = 3
};

struct _DRAKI_TOWER_FORDAILY_RANKING
{
	std::string strUserID;
	uint32 Class, DrakiTime;
	uint8 DrakiStage;
};

struct _USER_DAILY_RANK
{
	uint64 GMTotalSold, MHTotalKill, SHTotalExchange, CWCounterWin, UPCounterBles;
	_USER_DAILY_RANK() { Initialize(); }

	void Initialize()
	{
		GMTotalSold = MHTotalKill = SHTotalExchange = CWCounterWin = UPCounterBles = 0;
	}
};

struct _DAILY_RANK
{
	std::string strUserID;
	uint32 GmRank[3], MhRank[3], ShRank[3], AkRank[3], CwRank[3], UpRank[3], FkRank[3];
	uint64 GMTotalSold, MHTotalKill, SHTotalExchange;
	uint64 AKLoyaltyMonthly, CWCounterWin, UPCounterBles;
	uint64 Felokillcount;
	_DAILY_RANK() { Initialize(); }
	void Initialize()
	{
		strUserID.clear();
		memset(GmRank, 0, sizeof(GmRank)); GMTotalSold = 0;
		memset(MhRank, 0, sizeof(MhRank)); MHTotalKill = 0;
		memset(ShRank, 0, sizeof(ShRank)); SHTotalExchange = 0;
		memset(AkRank, 0, sizeof(AkRank)); AKLoyaltyMonthly = 0;
		memset(CwRank, 0, sizeof(CwRank)); CWCounterWin = 0;
		memset(UpRank, 0, sizeof(UpRank)); UPCounterBles = 0;
		memset(FkRank, 0, sizeof(FkRank)); Felokillcount = 0;
	}
};

struct _TIMED_NOTICE
{
	uint32 nIndex, time;
	uint8 noticetype;
	std::string notice;
	uint16 zoneid;
	uint32 usingtime;
	_TIMED_NOTICE()
	{
		Initialize();
	}
	void Initialize()
	{
		nIndex = 0;
		noticetype = 0;
		notice.clear();
		zoneid = 0;
		time = 0;
		usingtime = 0;
	}
};

enum class DailyRankType
{
	GRAND_MERCHANT = 0x00,
	KNIGHT_ADONIS = 0x01,
	HERO_OF_CHAOS = 0x02,
	MONSTER_HUNDER = 0x03,
	SHOZIN = 0x04,
	DRAKI_RANK = 0x05,
	DISCIPLE_KERON = 0x06,
	KING_OF_FELANKOR = 0x08,
	COUNT
};

enum SheriffOpcode
{
	ReportFailed = 0,
	SendPrison = 2,
	Question = 4,
	QuestionAnswer = 5,
	QuestionNotAnswered = 6,
	Macro = 7,
	ReportSuccess = 9,
	VotingFailed = 10,
	ReportAgree = 12,
	ReportDisagree = 13,
	ListOpen = 14,
	Unknown = 15,
	GiveAccolade = 16,
	Unknown2 = 17,
	KingsInspector = 18,
	RemoveAccolade = 20
};

enum CharAllInfoOpcodes
{
	AllCharInfoOpcode = 1,
	AlreadyCharName = 2,
	ArrangeOpen = 3,
	ArrangeRecvSend = 4
};

struct _SHERIFF_STUFF
{
	std::string reportedName, ReportSheriffName, crime, SheriffYesA, SheriffYesB, SheriffYesC, SheriffNoA, SheriffNoB, SheriffNoC, m_Date;
	uint8  m_VoteYes;
	uint8  m_VoteNo;

	_SHERIFF_STUFF()
	{
		reportedName.clear();
		ReportSheriffName.clear();
		crime.clear();
		SheriffYesA.clear();
		SheriffYesB.clear();
		SheriffYesC.clear();
		SheriffNoA.clear();
		SheriffNoB.clear();
		SheriffNoC.clear();
		m_Date.clear();
		m_VoteYes = m_VoteNo = 0;
	}
};

enum ClanBattle
{
	MatchStarted = 1,
	MatchLose = 2
};

enum GenieStatus
{
	GenieStatusInactive = 0,
	GenieStatusActive = 1
};

enum GenieSystemInfo
{
	GenieInfoRequest = 1,
	GenieUpdateRequest = 2
};

enum GenieNonAttackType
{
	GenieUseSpiringPotion = 1,
	GenieLoadOptions = 2,
	GenielSaveOptions = 3,
	GenieStartHandle = 4,
	GenieStopHandle = 5,
	GenieRemainingTime = 6,
	GenieActivated = 7,
};

enum GenieAttackHandle
{
	GenieMove = 1,
	GenieRotate = 2,
	GenieMainAttack = 3,
	GenieMagic = 4
};
struct _PUS_CATEGORY
{
	uint32 ID;
	std::string CategoryName;
	uint8 CategoryID;
	uint8 Status;
};
struct _PUS_ITEM
{
	uint32 ID;
	uint32 ItemID;
	uint32 Price;
	uint32 BuyCount;
	uint8 Cat;
	uint8 PriceType;
};

struct _PUS_REFUND
{
	uint32 itemid, itemprice, expiredtime;
	uint16 itemcount, itemduration;
	std::string accountid;
	uint8 buytype;
};

enum class moveop { finish, start, nott, move, };
struct _MOVE
{
	moveop status;
	ULONGLONG caughttime;
	int8  oldecho;
	int16 oldspeed;
	int caughtcount;

	_MOVE() { Initialize(); }
	void Initialize()
	{
		status = moveop::nott;
		oldspeed = 0;
		oldecho = -1;
		caughtcount = 0;
		caughttime = UNIXTIME2;
	}
};

struct _MykoSoftinfo
{
	ULONGLONG m_castusetime;
	uint32 skillid;
	MagicOpcode bOpcode;
	_MykoSoftinfo()
	{
		Initialize();
	}
	void Initialize()
	{
		skillid = 0;
		m_castusetime = 0;
		bOpcode = MagicOpcode::MAGIC_CASTING;
	}
};

struct _bot_merchant_it
{
	uint32 itemid, price, count; bool iskc;
	_ITEM_TABLE pTable;
	_bot_merchant_it()
	{
		itemid = price = count = 0;
		iskc = false;
		pTable = _ITEM_TABLE();
	}
};
struct _bot_merchant
{
	uint32 index;
	uint16 areaType;
	_bot_merchant_it merc[MAX_MERCH_ITEMS];
	bool isBuy;
};

struct _MERCHANT_BOT_INFO
{
	uint16 type;
	uint16 setX, setZ;
	int16 setY;
	uint8 bZoneID;
	int32 direction;
	bool used, isBuy;
	_MERCHANT_BOT_INFO()
	{
		setY = 0;
		direction = 0;
		isBuy = false;
		used = false;
		bZoneID = 0;
		type = setX = setZ = 0;
	}
};

struct _MERCHANT_BOT_ITEM
{
	uint32 itemid, minItemCount, maxItemCount;
	uint32 minPrice, maxPrice, minKc, maxKc;
	uint16 type;
	uint8 moneytype;
	_MERCHANT_BOT_ITEM()
	{
		itemid = minItemCount = maxItemCount = minPrice = maxPrice = minKc = maxKc = 0;
		moneytype = 0;
	}
};

struct _BOT_INFO
{
	CSTLMap<_MERCHANT_BOT_INFO> mCoordinate;
	CSTLMap<_MERCHANT_BOT_ITEM> mItem;

	_BOT_INFO() { Initialize(); }
	void Initialize()
	{
		mCoordinate.DeleteAllData();
		mItem.DeleteAllData();
	}
};

struct _USER_LAST_MAGIC_USED
{
	ULONGLONG ArrowUseTime;
	bool archerfailreturn, bCastFailed;

	ULONGLONG lastmultipletime;
	uint8 lastmultiple;
	short castX, castZ;
	uint32 castID;
	_MykoSoftinfo status;

	_USER_LAST_MAGIC_USED()
	{
		Initialize();
	}

	void Initialize()
	{
		status.Initialize();
		lastmultiple = 0;
		castID = 0;
		lastmultipletime = UNIXTIME2;
		archerfailreturn = bCastFailed = false;
		ArrowUseTime = 0;
		castX = castZ = -1;
	}
};

struct _BUFF_TYPE4_INFO
{
	uint32	m_nSkillID;
	uint8  m_bBuffType;
	bool	m_bIsBuff; // Is it a buff or a debuff?
	bool	m_bDurationExtended;
	time_t	m_tEndTime;
	CUser* pownskill;

	INLINE bool isBuff() { return m_bIsBuff; }
	INLINE bool isDebuff() { return !m_bIsBuff; }

	_BUFF_TYPE4_INFO()
		: m_nSkillID(0), m_bBuffType(0), m_bIsBuff(false), m_bDurationExtended(false), m_tEndTime(0), pownskill(nullptr)
	{
	}
};

struct _BUFF_TYPE9_INFO
{
	uint32	nSkillID;
	time_t	tEndTime;

	_BUFF_TYPE9_INFO(uint32 nSkillID, time_t tEndTime)
	{
		this->nSkillID = nSkillID;
		this->tEndTime = tEndTime;
	}
};

struct ZINDAN_WARINFO
{
	bool start; bool finishcheck;
	std::string ename, kname;
	uint16 kkillcount, ekillcount;
	time_t starttime, finishtime;


	ZINDAN_WARINFO()
	{
		Initiliaze();
	}
	void Initiliaze()
	{
		start = false;
		finishcheck = false;
		ename = "HUMAN";
		kname = "KARUS";
		kkillcount = ekillcount = 0;
		starttime = finishtime = 0;
	}
};

struct ITEMS
{
	int ITEMID;
	uint8 IPOS, ILINE, INDEX;
	uint16 _ICOUNT;
	_ITEM_TABLE pItem;
	uint32 price;
}
pItems;

struct _WANTED_USER
{
	CUser* pUser; bool wanted;
	_WANTED_USER(CUser* a, bool b) { pUser = a; wanted = b; }
};

enum class wantedstatus { disable, invitation, list, running };

struct _WANTED_MAIN
{
	std::recursive_mutex UserListLock;
	std::map<uint16, _WANTED_USER> m_elmolist, m_karuslist;

	wantedstatus status;
	time_t invitationtime, nextselecttime, listtime, finishtime;
	_WANTED_MAIN()
	{
		Initialize();
	}

	void Initialize()
	{
		UserListLock.lock();
		m_elmolist.clear();
		m_karuslist.clear();
		UserListLock.unlock();
		nextselecttime = UNIXTIME + 60;
		invitationtime = 0;
		listtime = 0;
		finishtime = 0;
		status = wantedstatus::disable;
	}
};


struct _DAILY_QUEST {
	uint8 index;
	uint16 Mobid[4], kcount[4]; uint32 rewaditemid[4], rewarditemcount[4];
	uint8 zoneid, minlevel, maxlevel, timetype, killtype, premiumstatus;
	uint32 replaytime;
	uint16 questid;
	uint8 randomid;
	std::string strQuestName;

	_DAILY_QUEST()
	{
		strQuestName = "<Enemy>";
	}
};

struct _DAILY_USERQUEST {
	uint8 status;
	uint16 kcount[4];
	uint32 replaytime;

	/*_DAILY_USERQUEST(uint8 a, uint8 b, uint16 c, uint32 d) {
		index = a;
		status = b;
		kcount = c;
		replaytime = d;
	}*/
	_DAILY_USERQUEST()
	{
		for (uint8 i = 0; i < 4; i++)
		{
			kcount[i] = 0;
		}
	}
};

//struct _ANTIAFKLIST {
//	uint32 index;
//	uint16 NpcID;
//	_ANTIAFKLIST(uint32 a, uint16 b) { index = a; NpcID = b; }
//};

struct RANKBUG
{
	uint32 BorderJoin, ChaosJoin, JuraidJoin, CzRank, CrMinComp, CrMaxComp, LotteryJoin;
	RANKBUG() { Initiliaze(); }
	void Initiliaze()
	{
		BorderJoin = ChaosJoin = JuraidJoin = CzRank = CrMinComp = CrMaxComp = 0;
		LotteryJoin = 0;
	}
};

struct _MONSTER_STONE_INFO
{
	bool icanbeused;
	std::vector<uint16> mUserList;
	bool Active, isBossKilled;
	uint8 zoneid;
	uint16 StartZoneID, MonsterFamily;
	time_t StartTime, FinishTime, WaitingTime;
	int16 roomid;
	bool bPartyMonsterStone;

	INLINE bool isnull() { return !icanbeused; }
	_MONSTER_STONE_INFO() { Initialize(); }

	void Initialize()
	{
		mUserList.clear();
		Active = isBossKilled = icanbeused = false;
		StartZoneID = 0;
		zoneid = 0;
		StartTime = 0;
		FinishTime = 0;
		WaitingTime = 0;
		MonsterFamily = 0;
		roomid = -1;
		bPartyMonsterStone = false;
	}

	void reset()
	{
		mUserList.clear();
		Active = isBossKilled = false;
		StartZoneID = 0;
		zoneid = 0;
		StartTime = 0;
		FinishTime = 0;
		WaitingTime = 0;
		MonsterFamily = 0;
	}
};

enum class CraftingErrorCode
{
	WrongMaterial = 0,
	Success = 1,
	Failed = 2
};


struct _MONSTER_RESPAWNLIST
{
	uint16 idead, iborn, count, deadtime;
};

struct timershow
{
	std::string name; uint32 hour, minute;
	timershow(std::string name, uint32 hour, uint32 minute) { this->name = name; this->hour = hour; this->minute = minute; }
};

enum class DailyQuestOp { sendlist, userinfo, killupdate };
enum class DailyQuesttimetype { repeat, time, single };
enum class DailyQuestStatus { timewait, comp, ongoing };
enum class pusrefunopcode { ireturn, listsend, itemnotfound, timeexpired, procestime, notinventory, itemused, itemreurnsucces, listadd };

#define GAMESERVER_CONSOLE_PASSWORD "123"
#define TOURNAMENT_MIN_CLAN_COUNT 1
#define TOURNAMENT_MAX_CLAN_COUNT 50
#define TOURNAMENT_MIN_PARTY_COUNT 1
#define TOURNAMENT_MAX_PARTY_COUNT 8

#include "../shared/database/structs.h"