#pragma once
typedef std::set <CNpc*> NpcSet;

class CNpc;
class CNpcThread
{
public:
	CNpcThread(uint16 sZoneID);
	virtual ~CNpcThread();
	void Shutdown();
	void Initialize(uint16 sZoneID);

	bool _AddObjectEventNpc(_OBJECT_EVENT* pEvent, uint16 nEventRoom = 0, bool isEventObject = false);
	void _LoadAllObjects();
	void _mNpcAdd(CNpc* pnpc);
	void _mNpcRemove(CNpc* pnpc);
	void _AddNPC(uint16 sSid, bool bIsMonster, uint8 byZone, float fX, float fY, float fZ, uint16 sCount = 1, uint16 sRadius = 0, uint16 sDuration = 0, uint8 nation = 0, int16 socketID = -1, uint16 nEventRoom = 0, uint8 byDirection = 0, uint8 bMoveType = 0, uint8 bGateOpen = 0, uint16 nSummonSpecialType = 0, uint32 nSummonSpecialID = 0);
	void _AddPetNPC(uint16 sSid, bool bIsMonster, uint8 byZone, float fX, float fY, float fZ,
		uint16 sCount /*= 1*/, uint16 sRadius /*= 0*/, uint16 sDuration /*= 0*/, uint8 nation /*= 0*/, int16 socketID /*= -1*/, uint16 nEventRoom /* = 0 */, uint8 nType /* = 0 */, uint32 nSkillID /* = 0 */);
	INLINE uint16 GetZoneID() { return m_sZoneID; };

	void ResetAllNPCs();
	void RemoveAllNPCs(uint8 eventroom = 0);
	void ChangeAbilityAllNPCs(uint8 bType);
	int _Engine();

	int t_counter;

	std::vector<uint16> m_FreeNpcList;
	std::recursive_mutex m_FreeNpcListLock;
	uint16 GetFreeID();
	void GiveIDBack(uint16 sNpcID);
	typedef CSTLMap <CNpc>	NpcArray;
	NpcArray m_arNpcArray;
	std::thread m_thread;
	NpcSet m_npclist;
	std::recursive_mutex testlock;
private:
	uint16 m_sZoneID;
	bool _running;
};
