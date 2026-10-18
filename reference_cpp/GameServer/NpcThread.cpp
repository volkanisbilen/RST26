#include "stdafx.h"
#include <sstream>
#include <iostream>
#include "MagicInstance.h"

#pragma region CNpcThread::GetFreeID()
uint16 CNpcThread::GetFreeID()
{
	Guard lock(m_FreeNpcListLock);
	if (m_FreeNpcList.size() <= 0)
	{
		printf("Max number of NPC spawned. No ID available.\n");
		return -1;
	}
	uint16 sNpcID = *(m_FreeNpcList.begin());
	m_FreeNpcList.erase(m_FreeNpcList.begin());
	return sNpcID;
}
#pragma endregion

#pragma region CNpcThread::GiveIDBack(uint16 sNpcID) 
void CNpcThread::GiveIDBack(uint16 sNpcID)
{
	Guard lock(m_FreeNpcListLock);
	m_FreeNpcList.push_back(sNpcID);
}
#pragma endregion

#pragma region CNpcThread::CNpcThread(uint16 ZoneID)
CNpcThread::CNpcThread(uint16 ZoneID) { Initialize(ZoneID); }
#pragma endregion

#pragma region CNpcThread::Initialize(uint16 sZoneID)
void CNpcThread::Initialize(uint16 sZoneID)
{
	m_npclist.clear();
	_running = true;
	m_thread = std::thread(&CNpcThread::_Engine, this);
	m_sZoneID = sZoneID;
	m_FreeNpcList.clear();
	t_counter = 0;
	for (uint16 i = NPC_BAND; i < 32567; i++) m_FreeNpcList.push_back(i);
}
#pragma endregion

int CNpcThread::_Engine()
{
	try
	{
		time_t dwDiffTime = 0, dwTickTime = 0, fTime2 = 0;
		int duration_damage = 0;
		std::set<CNpc*> deleted;
		while (_running)
		{
			deleted.clear();
			if (g_pMain->npcthreadreload) { Sleep(200); continue; }

			fTime2 = getMSTime(); // the current time

			testlock.lock();
			std::set <CNpc*> copylist = m_npclist;
			testlock.unlock();

			foreach(itr, copylist)
			{
				try
				{
					auto* pNpc = *itr;
					if (pNpc == nullptr) continue;


					if (pNpc->m_bDelete && !pNpc->m_bDeleteRequested && (pNpc->m_tDeleteTime > 0 && uint32(UNIXTIME) - pNpc->m_tDeleteTime > 60))
					{
						pNpc->m_bDeleteRequested = true;
						deleted.insert(pNpc);
						continue;
					}

					if (pNpc->m_bDelete)
					{
						continue;
					}

					bool jumping = false;
					dwTickTime = fTime2 - pNpc->m_fDelayTime;
					if (pNpc->m_bForceReset && pNpc->GetNpcState() == (uint8)NpcState::NPC_DEAD)
					{
						jumping = true;
						pNpc->m_bForceReset = false;
					}

					if (!jumping && pNpc->m_Delay > dwTickTime && !pNpc->m_bFirstLive)
					{
						if (!pNpc->isDead() && pNpc->GetNpcState() == (uint8)NpcState::NPC_STANDING
							&& pNpc->CheckFindEnemy() && pNpc->FindEnemy())
						{
							pNpc->StateChange((uint8)NpcState::NPC_ATTACKING);
							pNpc->m_Delay = 0;
						}
						continue;
					}

					uint8 bState = pNpc->GetNpcState();
					time_t tDelay = -1;
					bool bIsDead = pNpc->isDead();

					if (pNpc->m_bDelete)
						continue;

					switch ((NpcState)bState)
					{
					case NpcState::NPC_LIVE:
						tDelay = pNpc->NpcLive();
						break;
					case NpcState::NPC_HPCHANGE:
						if (!bIsDead)
							tDelay = pNpc->HpChangeReq();
						break;
					case NpcState::NPC_STANDING:
						if (!bIsDead)
							tDelay = pNpc->NpcStanding();
						break;
					case NpcState::NPC_MOVING:
						if (!bIsDead)
							tDelay = pNpc->NpcMoving();
						break;
					case NpcState::NPC_ATTACKING:
						if (!bIsDead)
							tDelay = pNpc->NpcAttacking();
						break;
					case NpcState::NPC_TRACING:
						if (!bIsDead)
							tDelay = pNpc->NpcTracing();
						break;
					case NpcState::NPC_FIGHTING:
						if (!bIsDead)
							tDelay = pNpc->Attack();
						break;
					case NpcState::NPC_BACK:
						if (!bIsDead)
							tDelay = pNpc->NpcBack();
						break;
					case NpcState::NPC_DEAD:
						tDelay = pNpc->NpcDead();
						break;
					case NpcState::NPC_SLEEPING:
						if (!bIsDead)
							tDelay = pNpc->NpcSleeping();
						break;
					case NpcState::NPC_FAINTING:
						if (!bIsDead)
							tDelay = pNpc->NpcFainting();
						break;
					case NpcState::NPC_HEALING:
						if (!bIsDead)
							tDelay = pNpc->NpcHealing();
						break;
					case NpcState::NPC_CASTING:
						if (!bIsDead)
							tDelay = pNpc->NpcCasting();
						break;
					}

					if (bState != (uint8)NpcState::NPC_LIVE && bState != (uint8)NpcState::NPC_DEAD && pNpc->GetNpcState() != (uint8)NpcState::NPC_DEAD)
						pNpc->m_fDelayTime = getMSTime();

					if (tDelay >= 0)
						pNpc->m_Delay = tDelay;

					time_t dwTickTimeType4 = fTime2 - pNpc->m_fHPType4CheckTime;
					if (3 * SECOND < dwTickTimeType4 && pNpc->isAlive())
					{
						pNpc->m_fHPType4CheckTime = getMSTime();
						pNpc->Type4Duration();
					}

					time_t dwTickTimeType3 = fTime2 - pNpc->m_fHPType3CheckTime;
					if (1 * SECOND < dwTickTimeType3 && pNpc->isAlive())
					{
						pNpc->m_fHPType3CheckTime = getMSTime();
						pNpc->HPTimeChangeType3();
					}

					dwTickTime = fTime2 - pNpc->m_fHPChangeTime;
					if (15 * SECOND < dwTickTime)
						pNpc->HpChange();

					if (pNpc->isAlive() && pNpc->m_sDuration > 0 && pNpc->m_iSpawnedTime && (int32(UNIXTIME) - pNpc->m_iSpawnedTime > pNpc->m_sDuration))
					{
						pNpc->Dead();
						continue;
					}
				}
				catch (std::system_error& ex)
				{
					printf("NPC ERROR - _NpcThread:1 - %s\n", ex.what());
					continue;
				}
			}
			copylist.clear();
			foreach(itr, deleted)
			{
				auto* ptrNpc = *itr;
				if (ptrNpc == nullptr)
					continue;

				//printf("deleted: %s %d\n", ptrNpc->GetName().c_str(), ptrNpc->GetID());

				uint16 sNpcID = ptrNpc->GetID();
				GiveIDBack(sNpcID);
				_mNpcRemove(ptrNpc);
				m_arNpcArray.DeleteData(sNpcID);
			}
			Sleep(250);
		}
	}
	catch (std::system_error& ex)
	{
		printf("NPC ERROR - _NpcThread:2 - %s\n", ex.what());
	}
	return 0;
}

void CNpcThread::_mNpcAdd(CNpc* pnpc)
{
	Guard lock(testlock);
	m_npclist.insert(pnpc);
}

void CNpcThread::_mNpcRemove(CNpc* pnpc)
{
	Guard lock(testlock);
	m_npclist.erase(pnpc);
}

#pragma region CGameServerDlg::_CreateNpcThread()
bool CGameServerDlg::_CreateNpcThread()
{
	m_TotalNPC = 0; m_CurrentNPC = 0;
	//printf("\nNPC THREAD SYSTEM CREATOR STARTING\n"); 

	uint16 counter = 0;
	std::vector<C3DMap*> mZoneList;
	foreach_stlmap_nolock(itr, m_ZoneArray)
	{
		C3DMap* pInfo = itr->second;
		if (pInfo == nullptr || !pInfo->m_Status) continue;
		mZoneList.push_back(pInfo);
	}

	if (mZoneList.empty()) { printf("\nNpc thread starting failed.\n"); return false; }
	std::sort(mZoneList.begin(), mZoneList.end(), [](C3DMap* const& a, C3DMap* const& b) { return a->m_nZoneNumber < b->m_nZoneNumber; });

	foreach(mykosoftjanekdominik, mZoneList)
	{
		CNpcThread* pNpcThread = new CNpcThread((*mykosoftjanekdominik)->m_nZoneNumber);
		if (!m_arNpcThread.PutData((*mykosoftjanekdominik)->m_nZoneNumber, pNpcThread)) { delete pNpcThread; return false; }

		std::string threadid = std::to_string((*mykosoftjanekdominik)->m_nZoneNumber);
		if ((*mykosoftjanekdominik)->m_nZoneNumber < 10) threadid = string_format("0%d", (*mykosoftjanekdominik)->m_nZoneNumber);
		++counter;

		pNpcThread->_LoadAllObjects();
	}
	printf("\n");
	if (!LoadNpcPosTable()) return false;
	printf("Npc Threads created for all zones. Total NPC Num = %d, threads = %d\n", (uint16)m_TotalNPC, (uint32)m_arNpcThread.GetSize());
	return true;
}
#pragma endregion

#pragma region CGameServerDlg::_LoaderSpawnCallBack(OdbcCommand* _dbCommand)
bool CGameServerDlg::_LoaderSpawnCallBack(OdbcCommand* _dbCommand)
{
	if (_dbCommand == nullptr)
		return false;

	int nRandom = 0;
	int field = 1; static CNpcTable* pNpcTable = nullptr;
	uint8  bZoneID, bActType, bRegenType, bDungeonFamily, bSpecialType, bTrapNumber, bDotCnt;
	uint16 sSid, sRegTime, sRoom, bNumNpc; uint8 bPathSerial = 1;
	int16  bDirection; static char szPath[500];
	int32  iLeftX, iTopZ;
	int nRandomX = 0, nRandomZ = 0;
	float fRandom_X = 0.0f, fRandom_Z = 0.0f;
	uint16 iRange = 0;
	bool monsummon = false, monsummonsave = false;

	_dbCommand->FetchByte(field++, bZoneID);
	_dbCommand->FetchUInt16(field++, sSid);
	_dbCommand->Fetchtbool(field++, monsummon);
	_dbCommand->Fetchtbool(field++, monsummonsave);
	_dbCommand->FetchByte(field++, bActType);
	_dbCommand->FetchByte(field++, bRegenType);
	_dbCommand->FetchByte(field++, bDungeonFamily);
	_dbCommand->FetchByte(field++, bSpecialType);
	_dbCommand->FetchByte(field++, bTrapNumber);
	_dbCommand->FetchInt32(field++, iLeftX);
	_dbCommand->FetchInt32(field++, iTopZ);
	_dbCommand->FetchUInt16(field++, bNumNpc);
	_dbCommand->FetchUInt16(field++, sRegTime);
	_dbCommand->FetchInt16(field++, bDirection);
	_dbCommand->FetchByte(field++, bDotCnt);
	_dbCommand->FetchString(field++, szPath, sizeof(szPath));
	_dbCommand->FetchUInt16(field++, sRoom);
	_dbCommand->FetchUInt16(field++, iRange);

	KOMap* pZone = g_pMain->GetZoneByID(bZoneID);
	if (pZone == nullptr || !pZone->m_Status)
		return true;

	CNpcThread* pnpcthread = m_arNpcThread.GetData(bZoneID, false);
	if (pnpcthread == nullptr)
		return true;

	if (bZoneID != pnpcthread->GetZoneID())
		return true;

	for (uint16 j = 0; j < bNumNpc; j++)
	{
		CNpc* pNpc = new CNpc();
		pNpc->m_byMoveType = bActType;
		pNpc->m_byInitMoveType = bActType;

		bool bMonster = (bActType < 100);
		if (bMonster)
		{
			pNpcTable = g_pMain->m_arMonTable.GetData(sSid, false);
		}
		else
		{
			pNpc->m_byMoveType = bActType - 100;
			pNpcTable = g_pMain->m_arNpcTable.GetData(sSid, false);
		}

		if (pNpcTable == nullptr)
		{
			printf("[K_NPCPOS] Error: sNPC %d not found in %s table.\n", sSid, bMonster ? "K_MONSTER" : "K_NPC");
			delete pNpc;
			continue;
		}

		uint16 sNpcID = pnpcthread->GetFreeID();
		if (sNpcID == uint16(-1))
		{
			printf("Npc PutData Fail - ID = %d Zone = %d\n", pNpc->GetID(), pNpc->GetZoneID());
			delete pNpc;
			continue;
		}

		pNpc->m_bZone = bZoneID;

		pNpc->Load(sNpcID, pNpcTable, bMonster, pNpcTable->m_byGroup);
		pNpc->m_byBattlePos = 0;
		if (pNpc->m_byMoveType >= 2)
		{
			pNpc->m_byBattlePos = myrand(1, 3);
			pNpc->m_byPathCount = bPathSerial++;
		}

		pNpc->InitPos();

		nRandom = iRange;
		if (nRandom <= 0)
			fRandom_X = (float)iLeftX;
		else
			fRandom_X = (float)(myrand((iLeftX - nRandom) * 10, (iLeftX + nRandom) * 10) / 10);

		nRandom = iRange;
		if (nRandom <= 0)
			fRandom_Z = (float)iTopZ;
		else
			fRandom_Z = (float)(myrand((iTopZ - nRandom) * 10, (iTopZ + nRandom) * 10) / 10);

		pNpc->m_bEventRoom = sRoom;
		pNpc->SetPosition(fRandom_X, 0.0f, fRandom_Z);

		pNpc->m_sRegenTime = sRegTime * SECOND;
		pNpc->m_byDirection = (uint8)bDirection;
		pNpc->m_sMaxPathCount = bDotCnt;

		pNpc->pNpcBossInfo.isBoss = false;
		pNpc->pNpcBossInfo.StageID = -1;

		pNpc->m_sPetId = -1;
		pNpc->m_strPetName = "";
		pNpc->m_strUserName = "";

		if ((pNpc->m_byMoveType == 2 || pNpc->m_byMoveType == 3 || pNpc->m_byMoveType == 5) && bDotCnt == 0)
		{
			pNpc->m_byMoveType = 1;
			std::string npcName = pNpc->GetName();
			TRACE("##### ServerDlg:CreateNpcThread - Path type Error :  nid=%d, sid=%d, name=%s, acttype=%d, path=%d #####\n",
				pNpc->GetID(), pNpc->GetProtoID(), npcName.c_str(), pNpc->m_byMoveType, pNpc->m_sMaxPathCount);

		}

		if (bDotCnt > 0)
		{
			int index = 0;
			for (int l = 0; l < bDotCnt; l++)
			{
				static char szX[5], szZ[5];
				memset(szX, 0, sizeof(szX));
				memset(szZ, 0, sizeof(szZ));
				memcpy(szX, szPath + index, 4);
				index += 4;
				memcpy(szZ, szPath + index, 4);
				index += 4;
				pNpc->m_PathList.pPatternPos[l].x = atoi(szX);
				pNpc->m_PathList.pPatternPos[l].z = atoi(szZ);
			}
		}

		pNpc->m_nInitMinX = pNpc->m_nLimitMinX = (int)fRandom_X;//pNpcPos->iLeftX -nRandomX;
		pNpc->m_nInitMinY = pNpc->m_nLimitMinZ = (int)fRandom_Z;//pNpcPos->iTopZ -nRandomZ;
		pNpc->m_nInitMaxX = pNpc->m_nLimitMaxX = (int)fRandom_X;//pNpcPos->iRightX +nRandomX;
		pNpc->m_nInitMaxY = pNpc->m_nLimitMaxZ = (int)fRandom_Z;//pNpcPos->iBottomZ +nRandomZ;

		// dungeon work
		pNpc->m_byDungeonFamily = bDungeonFamily;
		pNpc->m_bySpecialType = (NpcSpecialType)bSpecialType;
		pNpc->m_byRegenType = bRegenType;
		pNpc->m_byTrapNumber = bTrapNumber;

		pNpc->m_pMap = g_pMain->GetZoneByID(pNpc->GetZoneID());
		if (pNpc->GetMap() == nullptr)
		{
			TRACE(_T("ERROR: NPC %d in zone %d that does not exist."), sSid, bZoneID);
			delete pNpc;
			continue;
		}


		if (pNpc->GetZoneID() == ZONE_JURAID_MOUNTAIN)
		{
			if (pNpc->GetEventRoom() > MAX_TEMPLE_EVENT_ROOM || !pNpc->GetEventRoom())
			{
				printf("juraid monster load eventroom error.\n");
				delete pNpc;
				return false;
			}

			auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(pNpc->GetEventRoom());
			if (!pRoomInfo)
			{
				printf("juraid monster load bridges error.\n");
				delete pNpc;
				return false;
			}

			if (pNpc->m_byTrapNumber == 1 && !pRoomInfo->pkBridges[0]) pRoomInfo->pkBridges[0] = pNpc->GetID();
			else if (pNpc->m_byTrapNumber == 2 && !pRoomInfo->pkBridges[1]) pRoomInfo->pkBridges[1] = pNpc->GetID();
			else if (pNpc->m_byTrapNumber == 3 && !pRoomInfo->pkBridges[2]) pRoomInfo->pkBridges[2] = pNpc->GetID();
			else if (pNpc->m_byTrapNumber == 4 && !pRoomInfo->peBridges[0]) pRoomInfo->peBridges[0] = pNpc->GetID();
			else if (pNpc->m_byTrapNumber == 5 && !pRoomInfo->peBridges[1]) pRoomInfo->peBridges[1] = pNpc->GetID();
			else if (pNpc->m_byTrapNumber == 6 && !pRoomInfo->peBridges[2]) pRoomInfo->peBridges[2] = pNpc->GetID();
			if (pNpc->GetProtoID() >= 8100 && pNpc->GetProtoID() <= 8106) pNpc->e_stype = e_summontype::m_bjuriadmonster;
		}
		else if (pNpc->GetZoneID() == ZONE_BORDER_DEFENSE_WAR)
		{
			if (pNpc->GetEventRoom() > 0 && pNpc->GetEventRoom() <= MAX_TEMPLE_EVENT_ROOM)
			{
				auto* pRoomInfo = g_pMain->m_TempleEventBDWRoomList.GetData(pNpc->GetEventRoom());
				if (pRoomInfo && pNpc->GetProtoID() == 9840 && !pRoomInfo->pAltar)
					pRoomInfo->pAltar = pNpc->GetID();
			}
		}

		pNpc->m_msummon = monsummon;
		pNpc->m_msummonsavepos = monsummonsave;

		if (!pnpcthread->m_arNpcArray.PutData(pNpc->GetID(), pNpc))
		{
			TRACE("Npc PutData Fail - ID = %d Zone = %d\n", pNpc->GetID(), pNpc->GetZoneID());
			delete pNpc;
			continue;
		}
		pnpcthread->_mNpcAdd(pNpc);
		++g_pMain->m_TotalNPC;
	}
	return true;
}
#pragma endregion

#pragma region CNpcThread::_LoadAllObjects
void CNpcThread::_LoadAllObjects()
{
	foreach_stlmap_nolock(itr, g_pMain->m_ObjectEventArray)
	{
		auto* pEvent = itr->second;
		if (pEvent == nullptr || pEvent->sZoneID != m_sZoneID) continue;
		if (pEvent->sType == OBJECT_GATE || pEvent->sType == OBJECT_GATE2
			|| pEvent->sType == OBJECT_GATE_LEVER || pEvent->sType == OBJECT_ANVIL
			|| pEvent->sType == OBJECT_ARTIFACT || pEvent->sType == OBJECT_WALL
			|| pEvent->sType == OBJECT_WOOD || pEvent->sType == OBJECT_WOOD_LEVER
			|| pEvent->sType == OBJECT_EFECKT || pEvent->sType == OBJECT_BIND
			|| pEvent->sType == OBJECT_POISONGAS || pEvent->sType == OBJECT_KROWASGATE)
			_AddObjectEventNpc(pEvent);
	}
}
#pragma endregion

#pragma region CNpcThread::_AddObjectEventNpc(_OBJECT_EVENT* pEvent, uint16 nEventRoom /* = 0 */, bool isEventObject /* false */)
bool CNpcThread::_AddObjectEventNpc(_OBJECT_EVENT* pEvent, uint16 nEventRoom /* = 0 */, bool isEventObject /* false */)
{
	C3DMap* pMap = g_pMain->GetZoneByID(m_sZoneID);
	if (!pEvent || pMap == nullptr) return false;

	int sSid = 0;

	bool object = { pEvent->sType == OBJECT_GATE || pEvent->sType == OBJECT_GATE2 || pEvent->sType == OBJECT_GATE_LEVER
		|| pEvent->sType == OBJECT_ANVIL || pEvent->sType == OBJECT_ARTIFACT || pEvent->sType == OBJECT_WALL
		|| pEvent->sType == OBJECT_WOOD || pEvent->sType == OBJECT_WOOD_LEVER || pEvent->sType == OBJECT_EFECKT || pEvent->sType == OBJECT_BIND };


	if (object) sSid = pEvent->sIndex;
	else sSid = pEvent->sControlNpcID;
	if (sSid <= 0)  return false;

	CNpcTable* pNpcTable = g_pMain->m_arNpcTable.GetData(sSid, false);
	if (pNpcTable == nullptr)  return false;

	CNpc* pNpc = new CNpc();

	pNpc->m_bIsEventNpc = isEventObject;
	if (pEvent->sType != 5) { pNpc->m_byMoveType = 104; pNpc->m_byInitMoveType = 104; }
	else { pNpc->m_byMoveType = 0; pNpc->m_byInitMoveType = 0; }
	pNpc->m_byBattlePos = 0;
	pNpc->m_byObjectType = SPECIAL_OBJECT;
	pNpc->m_byGateOpen = uint8(pEvent->sStatus);
	pNpc->m_bZone = pMap->m_nZoneNumber;
	pNpc->m_bEventRoom = nEventRoom;
	pNpc->SetPosition(pEvent->fPosX, pEvent->fPosY, pEvent->fPosZ);

	pNpc->m_nInitMinX = (int)pEvent->fPosX - 1;
	pNpc->m_nInitMinY = (int)pEvent->fPosZ - 1;
	pNpc->m_nInitMaxX = (int)pEvent->fPosX + 1;
	pNpc->m_nInitMaxY = (int)pEvent->fPosZ + 1;
	pNpc->m_strPetName.clear();

	uint16 sNpcID = GetFreeID();
	if (sNpcID == uint16(-1)) { delete pNpc; return nullptr; }
	pNpc->Load(sNpcID, pNpcTable, false);
	pNpc->m_pMap = pMap;
	if (pNpc->GetMap() == nullptr || !m_arNpcArray.PutData(pNpc->GetID(), pNpc, false))
	{
		delete pNpc;
		return false;
	}
	_mNpcAdd(pNpc);
	if (sSid == 5001)
	{
		if (pNpc->m_bZone == ZONE_MORADON)
		{
			g_pMain->m1anvilid = sNpcID;
			//printf(" \n");
			//printf("Auto Upgrade System Auto ID has been set for Moradon I.\n");
		}
		if (pNpc->m_bZone == ZONE_MORADON2)
		{
			g_pMain->m2anvilid = sNpcID;
			//printf("Auto Upgrade System Auto ID has been set for Moradon II.\n");
		}
		if (pNpc->m_bZone == ZONE_MORADON3)
		{
			g_pMain->m3anvilid = sNpcID;
			//printf("Auto Upgrade System Auto ID has been set for Moradon III.\n");
		}
		if (pNpc->m_bZone == ZONE_MORADON4)
		{
			g_pMain->m4anvilid = sNpcID;
			//printf("Auto Upgrade System Auto ID has been set for Moradon IV.\n");
		}
		if (pNpc->m_bZone == ZONE_MORADON5)
		{
			g_pMain->m5anvilid = sNpcID;
			//printf("Auto Upgrade System Auto ID has been set for Moradon V.\n");
		}
	}
	++g_pMain->m_TotalNPC;
	return true;
}
#pragma endregion

#pragma region CGameServerDlg::SpawnEventNpc
void CGameServerDlg::SpawnEventNpc(uint16 sSid, bool bIsMonster, uint8 byZone, float fX, float fY, float fZ,
	uint16 sCount /*= 1*/, uint16 sRadius /*= 0*/, uint16 sDuration /*= 0*/, uint8 nation /*= 0*/, int16 socketID /*= -1*/, uint16 nEventRoom /* = 0 */,
	uint8 byDirection /* = 0 */, uint8 bMoveType /* = 0 */, uint8 bGateOpen/* = 0 */, uint16 nSummonSpecialType/* = 0 */, uint32 nSummonSpecialID/* = 0 */)
{
	CNpcThread* zoneitrThread = m_arNpcThread.GetData(byZone);
	if (zoneitrThread) zoneitrThread->_AddNPC(sSid, bIsMonster, byZone, fX, fY, fZ, sCount, sRadius, sDuration,
		nation, socketID, nEventRoom, byDirection, bMoveType, bGateOpen, nSummonSpecialType, nSummonSpecialID);
}
#pragma endregion

#pragma region CNpcThread::_AddNNPC
void CNpcThread::_AddNPC(uint16 m_sSid, bool m_bIsMonster, uint8 m_byZone, float m_fX, float m_fY, float m_fZ,
	uint16 m_sCount /*= 1*/, uint16 sRadius /*= 0*/, uint16 m_sDuration /*= 0*/, uint8 m_nation /*= 0*/, int16 m_socketID /*= -1*/, uint16 m_nEventRoom /* = 0 */,
	uint8 m_byDirection /* = 0 */, uint8 m_bMoveType /* = 0 */, uint8 m_bGateOpen/* = 0 */, uint16 m_nSummonSpecialType/* = 0 */, uint32 m_nSummonSpecialID/* = 0 */)
{

	if (!m_sCount || m_fX < 0.0f || m_fZ < 0.0f)
		return;

	for (int i = 0; i < m_sCount; i++)
	{

		CNpcTable* proto = nullptr;
		KOMap* pZone = g_pMain->GetZoneByID(m_byZone);
		if (pZone == nullptr || !pZone->m_Status)
			return;

		if (m_bIsMonster) proto = g_pMain->m_arMonTable.GetData(m_sSid);
		else proto = g_pMain->m_arNpcTable.GetData(m_sSid);

		if (proto == nullptr)
		{
			g_pMain->IsSummon = false; // 29.10.2020 Boss At�nca Notice Gecmesi Engellendi
			return;
		}

		int Random = 0;
		CNpc* pNpc = new CNpc();
		if (m_nSummonSpecialType == (uint16)SpawnEventType::MonsterBossSummon)
			pNpc->m_bIsEventNpc = false;
		else
			pNpc->m_bIsEventNpc = true;

		pNpc->m_byMoveType = (m_bMoveType > 0 ? m_bMoveType : (m_bIsMonster ? 1 : 0));
		pNpc->m_byInitMoveType = (m_bMoveType > 0 ? m_bMoveType : (m_bIsMonster ? 1 : 0));
		pNpc->m_byBattlePos = 0;

		pNpc->m_bZone = m_byZone;
		pNpc->m_bNation = m_nation;
		pNpc->m_OrgNation = m_nation;
		float x = 0.0f, z = 0.0f;

		if (m_sCount > 1 || m_nSummonSpecialType == SpawnEventType::ChaosStoneSummon)
		{
			if (m_nSummonSpecialType == SpawnEventType::ChaosStoneSummon)
			{
				x = static_cast<float>(myrand(-10, 30));
				z = static_cast<float>(myrand(-10, 30));
			}
			else
			{
				x = static_cast<float>(myrand(-10, 10));
				z = static_cast<float>(myrand(-10, 10));
			}
		}

		pNpc->SetPosition((float)(m_fX + x), m_fY, m_fZ + z);
		pNpc->m_nInitMinX = pNpc->m_nLimitMinX = int(m_fX + x);
		pNpc->m_nInitMinY = pNpc->m_nLimitMinZ = int(m_fZ + z);
		pNpc->m_nInitMaxX = pNpc->m_nLimitMaxX = int(m_fX + x);
		pNpc->m_nInitMaxY = pNpc->m_nLimitMaxZ = int(m_fZ + z);
		pNpc->m_pMap = pZone;

		if (m_nSummonSpecialType == SpawnEventType::Loopstill && m_nSummonSpecialID)
		{
			pNpc->m_msummon = true;
			pNpc->m_msummonsavepos = true;
		}

		pNpc->m_bEventRoom = m_nEventRoom;
		pNpc->m_sDuration = m_sDuration;

		if (m_byDirection > 0)
			pNpc->m_byDirection = uint8(m_byDirection);

		uint16 sNpcID = GetFreeID();
		if (sNpcID == uint16(-1))
		{
			delete pNpc;
			return;
		}

		pNpc->Load(sNpcID, proto, m_bIsMonster, m_nation);
		pNpc->InitPos();

		if (!pNpc->m_bIsEventNpc && m_nSummonSpecialType == (uint16)SpawnEventType::MonsterBossSummon)
		{
			auto* pRandomBoss = g_pMain->m_MonsterBossRandom.GetData(m_nSummonSpecialID);
			if (pRandomBoss == nullptr)
			{
				delete pNpc;
				return;

			}
			pNpc->pNpcBossInfo.isBoss = true;
			pNpc->pNpcBossInfo.StageID = pRandomBoss->Stage;
			pNpc->m_sRegenTime = pRandomBoss->ReloadTime * SECOND;
		}
		else
		{
			pNpc->pNpcBossInfo.isBoss = false;
			pNpc->pNpcBossInfo.StageID = -1;
			pNpc->m_sRegenTime = 0;
		}

		pNpc->m_oSocketID = m_socketID;
		pNpc->m_byGateOpen = m_bGateOpen;

		if (pNpc->GetZoneID() == ZONE_DELOS)
		{
			if (pNpc->GetProtoID() == 511)
			{
				CUser* pUser = g_pMain->GetUserPtr(pNpc->m_oSocketID);
				if (pUser == nullptr)
				{
					delete pNpc;
					return;
				}
				else
				{
					if (!pUser->isInClan())
					{
						delete pNpc;
						return;
					}
					pNpc->m_oBarrackID = pUser->GetClanID();
				}
			}
			pNpc->m_oSocketID = -1;
		}

		if (m_nSummonSpecialID == 1) pNpc->e_stype = e_summontype::m_MonsterStoneBoss;

		switch (m_nSummonSpecialType)
		{
		case SpawnEventType::UnderTheCastleSummon:
		{
			if (pNpc->GetZoneID() == ZONE_UNDER_CASTLE)
			{
				_UNDER_THE_CASTLE_INFO* pRoomInfo = g_pMain->m_MonsterUnderTheCastleList.GetData(1);
				if (pRoomInfo != nullptr)
				{
					if (m_nSummonSpecialID == 1)
						pRoomInfo->m_sUtcGateID[0] = pNpc->GetID();
					else if (m_nSummonSpecialID == 2)
						pRoomInfo->m_sUtcGateID[1] = pNpc->GetID();
					else if (m_nSummonSpecialID == 3)
						pRoomInfo->m_sUtcGateID[2] = pNpc->GetID();
				}
			}
		}
		break;
		case SpawnEventType::ChaosStoneSummon:
		{
			if (pNpc->GetZoneID() == ZONE_RONARK_LAND
				|| pNpc->GetZoneID() == ZONE_RONARK_LAND_BASE
				|| pNpc->GetZoneID() == ZONE_ARDREAM)
			{
				if (m_nSummonSpecialID > 0)
				{
					_CHAOS_STONE_INFO* pChaosInfo = g_pMain->m_ChaosStoneInfoArray.GetData(m_nSummonSpecialID);
					if (pChaosInfo != nullptr)
					{
						if (pNpc->GetZoneID() == pChaosInfo->sZoneID && pChaosInfo->isChaosStoneKilled == true && pNpc->isMonster())
						{
							if (pChaosInfo->sBoosID[0] == 0)
								pChaosInfo->sBoosID[0] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[1] == 0)
								pChaosInfo->sBoosID[1] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[2] == 0)
								pChaosInfo->sBoosID[2] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[3] == 0)
								pChaosInfo->sBoosID[3] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[4] == 0)
								pChaosInfo->sBoosID[4] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[5] == 0)
								pChaosInfo->sBoosID[5] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[6] == 0)
								pChaosInfo->sBoosID[6] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[7] == 0)
								pChaosInfo->sBoosID[7] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[8] == 0)
								pChaosInfo->sBoosID[8] = pNpc->GetID();
							else if (pChaosInfo->sBoosID[9] == 0)
								pChaosInfo->sBoosID[9] = pNpc->GetID();

							pChaosInfo->sBoosKilledCount++;
						}
					}
				}
			}
		}
		break;
		case SpawnEventType::DungeonDefencSummon:
		{
			if (pNpc->GetZoneID() == ZONE_DUNGEON_DEFENCE)
			{
				_DUNGEON_DEFENCE_ROOM_INFO* pRoomInfo = g_pMain->m_DungeonDefenceRoomListArray.GetData(m_nSummonSpecialID);
				if (pRoomInfo != nullptr)
				{
					if (pRoomInfo->m_DefenceisStarted == true)
					{
						if (pNpc->isMonster())
							pRoomInfo->m_DefenceKillCount++;
					}
				}
			}
		}
		break;
		case SpawnEventType::DrakiTowerSummon:
		{
			if (pNpc->GetZoneID() == ZONE_DRAKI_TOWER)
			{
				_DRAKI_TOWER_INFO* pRoomInfo = g_pMain->m_MonsterDrakiTowerList.GetData(m_nSummonSpecialID);

				if (pRoomInfo != nullptr)
				{
					if (pRoomInfo->m_tDrakiTowerStart == true)
					{
						if (pNpc->isMonster())
							pRoomInfo->m_bDrakiMonsterKill++;
					}
				}

			}
		}
		break;
		case SpawnEventType::JuraidMonster:
			if (pNpc->GetZoneID() == ZONE_JURAID_MOUNTAIN)
			{
				if (pNpc->GetProtoID() == 2152 || pNpc->GetProtoID() == 8007 || pNpc->GetProtoID() == 1772)
					pNpc->e_stype = e_summontype::m_bjuriadmonster;
			}
			break;
		case SpawnEventType::GuardSummon:
			pNpc->m_sGuardID = m_socketID;
			break;
		case SpawnEventType::ForgettenTempleSummon:
			if (g_pMain->isForgettenTempleActive() && pNpc->GetZoneID() == ZONE_FORGOTTEN_TEMPLE)
			{
				pNpc->e_stype = e_summontype::m_ftmonster;
				g_pMain->pForgettenTemple.monstercount++;
			}
			break;
		default:
			break;
		}

		if (!m_arNpcArray.PutData(pNpc->GetID(), pNpc))
		{
			printf("[AddNPC Fail] Cannot PutData | ID = %d Zone = %d\n", pNpc->GetID(), m_byZone);
			delete pNpc;
			return;
		}

		_mNpcAdd(pNpc);
		pNpc->RegisterRegion();
	}
}
#pragma endregion

#pragma region CGameServerDlg::SpawnEventPet
void CGameServerDlg::SpawnEventPet(uint16 sSid, bool bIsMonster, uint8 byZone, float fX, float fY, float fZ,
	uint16 sCount /*= 1*/, uint16 sRadius /*= 0*/, uint16 sDuration /*= 0*/, uint8 nation /*= 0*/, int16 socketID /*= -1*/, uint16 nEventRoom /* = 0 */, uint8 nType /* = 0 */, uint32 nSkillID /* = 0 */)
{
	CNpcThread* zoneitrThread = m_arNpcThread.GetData(byZone);
	if (zoneitrThread == nullptr) return;
	zoneitrThread->_AddPetNPC(sSid, bIsMonster, byZone, fX, fY, fZ, sCount, sRadius, sDuration,
		nation, socketID, nEventRoom, nType, nSkillID);
}
#pragma endregion

#pragma region CNpcThread::_AddPetNPC
void CNpcThread::_AddPetNPC(uint16 m_sSid, bool m_bIsMonster, uint8 m_byZone, float m_fX, float m_fY, float m_fZ,
	uint16 m_sCount /*= 1*/, uint16 m_sRadius /*= 0*/, uint16 m_sDuration /*= 0*/, uint8 m_nation /*= 0*/, int16 m_socketID /*= -1*/, uint16 m_nEventRoom /* = 0 */, uint8 m_Type /* = 0 */, uint32 m_SkillID /* = 0 */)
{
	if (!m_sCount || m_fX < 0 || m_fZ < 0) return;

	for (int i = 0; i < m_sCount; i++)
	{
		int16 minRange = -(m_sRadius / 2);
		float fX_temp = m_fX, fZ_temp = m_fZ;

		m_fX = fX_temp + (float)(myrand(minRange, m_sRadius));
		m_fZ = fZ_temp + (float)(myrand(minRange, m_sRadius));

		CNpcTable* proto = nullptr;
		KOMap* pZone = g_pMain->GetZoneByID(m_byZone);
		if (pZone == nullptr || !pZone->m_Status) return;

		if (m_bIsMonster) proto = g_pMain->m_arMonTable.GetData(m_sSid);
		else proto = g_pMain->m_arNpcTable.GetData(m_sSid);
		if (proto == nullptr) return;

		CNpc* pNpc = new CNpc();

		pNpc->m_bIsEventNpc = true;
		pNpc->m_byMoveType = (m_bIsMonster ? 1 : 0);

		pNpc->m_byInitMoveType = 1;
		pNpc->m_byBattlePos = 0;

		pNpc->m_bZone = m_byZone;

		pNpc->SetPosition(m_fX, m_fY, m_fZ);
		pNpc->m_nInitMinX = pNpc->m_nLimitMinX = int(m_fX);
		pNpc->m_nInitMinY = pNpc->m_nLimitMinZ = int(m_fZ);
		pNpc->m_nInitMaxX = pNpc->m_nLimitMaxX = int(m_fX);
		pNpc->m_nInitMaxY = pNpc->m_nLimitMaxZ = int(m_fZ);
		pNpc->m_pMap = pZone;

		pNpc->m_bEventRoom = m_nEventRoom;
		pNpc->m_sDuration = m_sDuration;

		pNpc->m_byDirection = 0;

		uint16 sNpcID = GetFreeID();
		if (sNpcID == uint16(-1))
		{
			printf("[AddNPC Fail] No Available ID | ID = %d Zone = %d\n", pNpc->GetID(), m_byZone);
			delete pNpc;
			return;
		}

		pNpc->Load(sNpcID, proto, m_bIsMonster, m_nation);
		pNpc->InitPos();

		if (pNpc->m_bIsEventNpc)
			pNpc->m_sRegenTime = 0;

		pNpc->m_oSocketID = m_socketID;
		pNpc->m_byGateOpen = 0;

		if (m_Type > 0)
		{
			enum OpCodes
			{
				PetHandler = 1,
				GuardHandler = 2
			};

			if (m_Type == PetHandler)
			{
				CUser* pUser = g_pMain->GetUserPtr(m_socketID);
				if (pUser == nullptr)
				{
					delete pNpc;
					return;
				}

				if (pUser->m_PettingOn == nullptr)
				{
					delete pNpc;
					return;
				}
				pNpc->m_sPetId = m_socketID;
				pNpc->m_strPetName = pUser->m_PettingOn->strPetName;
				pNpc->m_strUserName = pUser->m_strUserID;
				pNpc->m_iHP = pUser->m_PettingOn->sHP;
				pNpc->m_MaxHP = pUser->m_PettingOn->info->PetMaxHP;
				pNpc->m_sMP = pUser->m_PettingOn->sMP;
				pNpc->m_MaxMP = pUser->m_PettingOn->info->PetMaxMP;
				pNpc->m_bLevel = pUser->m_PettingOn->bLevel;
				pNpc->m_sAttack = pUser->m_PettingOn->info->PetAttack;
				pNpc->m_sTotalAc = pUser->m_PettingOn->info->PetDefense;
				pNpc->m_sTotalAcTemp = pUser->m_PettingOn->info->PetDefense;
				pNpc->m_fTotalHitrate = pUser->m_PettingOn->info->PetRes;
				pNpc->m_fTotalEvasionrate = pUser->m_PettingOn->info->PetRes;
				pNpc->m_sTotalHit = pUser->m_PettingOn->info->PetAttack;
				pNpc->m_sPid = pUser->m_PettingOn->sPid;
				pNpc->m_sSize = pUser->m_PettingOn->sSize;
				pNpc->m_bNation = m_nation;
				pUser->m_PettingOn->sNid = pNpc->GetID();
				// Ensure pet health is synchronized after the NPC is fully registered to avoid
				// immediate-death caused by race conditions where HpChange is processed
				// before the NPC stats are set on the client/player.
				if (pNpc->m_iHP <= 0)
					pNpc->m_iHP = pNpc->GetMaxHealth();

				// Debug log: pet NPC added
				printf("[PET_DEBUG] _AddPetNPC: user=%s socket=%d zone=%d petIndex=%d assignedNid=%d HP=%d MaxHP=%d Sat=%d\n",
					pUser->GetName().c_str(), pUser->GetSocketID(), pUser->GetZoneID(), pUser->m_PettingOn->nIndex, pUser->m_PettingOn->sNid, pNpc->m_iHP, pNpc->GetMaxHealth(), pUser->m_PettingOn->sSatisfaction);
				g_pMain->ServerLog("PET_DEBUG", "_AddPetNPC user=%s socket=%d zone=%d petIndex=%d assignedNid=%d HP=%d MaxHP=%d Sat=%d", pUser->GetName().c_str(), pUser->GetSocketID(), pUser->GetZoneID(), pUser->m_PettingOn->nIndex, pUser->m_PettingOn->sNid, pNpc->m_iHP, pNpc->GetMaxHealth(), pUser->m_PettingOn->sSatisfaction);

				if (pUser->m_PettingOn->sSatisfaction <= 0)
					pUser->m_PettingOn->sSatisfaction = 500;

				if (m_SkillID > 0)
				{
					Packet result;
					pUser->PetSpawnProcess();
					pUser->m_lastPetSatisfaction = UNIXTIME;
					MagicInstance instance;
					instance.sData[0] = 0;
					instance.sData[1] = 1;
					instance.sData[2] = 0;
					instance.sData[3] = -1;
					instance.sData[4] = pUser->m_PettingOn->sNid;
					instance.sData[5] = 0;
					instance.sData[6] = 0;

					instance.BuildSkillPacket(result, pUser->GetSocketID(), pUser->GetSocketID(), (int8)MagicOpcode::MAGIC_EFFECTING, m_SkillID, instance.sData);
					pUser->Send(&result);
				}
			}

			if (m_Type == GuardHandler) pNpc->m_sGuardID = m_socketID;
			pNpc->m_oSocketID = -1;
		}

		if (!m_arNpcArray.PutData(pNpc->GetID(), pNpc))
		{
			printf("[Add PET NPC Fail] Cannot PutData | ID = %d Zone = %d\n", pNpc->GetID(), m_byZone);
			delete pNpc;
			return;
		}
		_mNpcAdd(pNpc);
	}
}
#pragma endregion

#pragma region CGameServerDlg::ResetAllEventObject(uint8 byZone)
void CGameServerDlg::ResetAllEventObject(uint8 byZone)
{
	auto* zoneitrThread = m_arNpcThread.GetData(byZone);
	if (zoneitrThread) zoneitrThread->ResetAllNPCs();
}
#pragma endregion

#pragma region CNpcThread::ResetAllNPCs()
void CNpcThread::ResetAllNPCs()
{
	std::vector<CNpc*> mlist;
	m_arNpcArray.m_lock.lock();
	foreach_stlmap_nolock(itr, m_arNpcArray)
	{
		CNpc* pNpc = itr->second;
		if (pNpc == nullptr || pNpc->GetType() == NPC_DESTROYED_ARTIFACT)
			continue;

		mlist.push_back(pNpc);
	}
	m_arNpcArray.m_lock.unlock();

	foreach(itr, mlist)
	{
		if (!(*itr))
			continue;

		(*itr)->Dead();
		(*itr)->m_bForceReset = true;

		if ((*itr)->GetNpcState() == (uint8)NpcState::NPC_LIVE)
			(*itr)->HpChange((*itr)->GetMaxHealth(), nullptr);
	}
}
#pragma endregion

#pragma region CGameServerDlg::RemoveAllEventNpc
void CGameServerDlg::RemoveAllEventNpc(uint8 byZone)
{
	CNpcThread* zoneitrThread = m_arNpcThread.GetData(byZone);
	if (zoneitrThread == nullptr) return;
	zoneitrThread->RemoveAllNPCs();

	if (byZone == ZONE_BORDER_DEFENSE_WAR || byZone == ZONE_JURAID_MOUNTAIN || byZone == ZONE_UNDER_CASTLE)
		zoneitrThread->ResetAllNPCs();
}
#pragma endregion

#pragma region CNpcThread::RemoveAllNPCs(uint8 eventroom)
void CNpcThread::RemoveAllNPCs(uint8 eventroom /* = 0*/)
{
	foreach_stlmap(itr, m_arNpcArray)
	{
		CNpc* pNpc = itr->second;
		if (pNpc == nullptr || !pNpc->m_bIsEventNpc || pNpc->isDead() || (eventroom > 0 && pNpc->GetEventRoom() != eventroom))
			continue;

		pNpc->Dead();
	}
}
#pragma endregion

#pragma region CNpcThread::ChangeAbilityAllNPCs(uint8 bType)
void CNpcThread::ChangeAbilityAllNPCs(uint8 bType)
{
	foreach_stlmap(itr, m_arNpcArray)
	{
		CNpc* pNpc = itr->second;
		if (pNpc == nullptr) continue;
		if (pNpc->GetType() > 10 && (pNpc->GetNation() == (uint8)Nation::KARUS || pNpc->GetNation() == (uint8)Nation::ELMORAD))
			pNpc->ChangeAbility(bType);
	}
}
#pragma endregion

#pragma region CNpcThread::HandleJuraidKill(CUser* pUser)
void CNpc::HandleJuraidKill(CUser* pKiller)
{
	if (!pKiller || !pKiller->isInValidRoom(0) || e_stype != e_summontype::m_bjuriadmonster)
		return;

	if (GetProtoID() != LEECH_KING_SSID && GetProtoID() != KOCATRIS_SSID
		&& GetProtoID() != LILIME_SSID && GetProtoID() != MINOTAUR_SSID
		&& GetProtoID() != BONE_DRAGON_SSID && GetProtoID() != RED_DRAGON_HATCHLING_SSID
		&& GetProtoID() != DEVA_BIRD_SSID && GetProtoID() != MONSTER_APOSTLE_SSID
		&& GetProtoID() != MONSTER_DOOM_SOLDIER_SSID && GetProtoID() != MONSTER_TROLL_CAPTAIN_SSID)
		return;

	uint8 bNation = pKiller->GetNation();
	auto* pRoomInfo = g_pMain->m_TempleEventJuraidRoomList.GetData(pKiller->GetEventRoom());
	if (!pRoomInfo || pRoomInfo->m_FinishPacketControl)
		return;

	if (GetProtoID() == MONSTER_APOSTLE_SSID || GetProtoID() == MONSTER_DOOM_SOLDIER_SSID || GetProtoID() == MONSTER_TROLL_CAPTAIN_SSID)
	{
		if (bNation == (uint8)Nation::KARUS)
		{
			pRoomInfo->m_iKarusSubMonsterKill++;
			uint32 mainMobKill = pRoomInfo->m_iKarusMainMonsterKill;
			uint8 subMobKill = pRoomInfo->m_iKarusSubMonsterKill;
			if (mainMobKill == 4 && subMobKill == 20) HandleJuraidGateOpen(pRoomInfo->pkBridges[0], 0, pRoomInfo, Nation::KARUS);
			else if (mainMobKill == 8 && subMobKill == 40) HandleJuraidGateOpen(pRoomInfo->pkBridges[1], 1, pRoomInfo, Nation::KARUS);
			else if (mainMobKill == 12 && subMobKill == 60) HandleJuraidGateOpen(pRoomInfo->pkBridges[2], 2, pRoomInfo, Nation::KARUS);
		}
		else if (bNation == (uint8)Nation::ELMORAD)
		{
			pRoomInfo->m_iElmoSubMonsterKill++;
			uint32 mainMobKill = pRoomInfo->m_iElmoMainMonsterKill;
			uint8 subMobKill = pRoomInfo->m_iElmoSubMonsterKill;
			if (mainMobKill == 4 && subMobKill == 20) HandleJuraidGateOpen(pRoomInfo->peBridges[0], 0, pRoomInfo, Nation::ELMORAD);
			else if (mainMobKill == 8 && subMobKill == 40) HandleJuraidGateOpen(pRoomInfo->peBridges[1], 1, pRoomInfo, Nation::ELMORAD);
			else if (mainMobKill == 12 && subMobKill == 60) HandleJuraidGateOpen(pRoomInfo->peBridges[2], 2, pRoomInfo, Nation::ELMORAD);
		} return;
	}

	bNation == (uint8)Nation::KARUS ? pRoomInfo->m_iKarusMainMonsterKill++ : pRoomInfo->m_iElmoMainMonsterKill++;
	uint32 mainMobKill = bNation == (uint8)Nation::KARUS ? pRoomInfo->m_iKarusMainMonsterKill : pRoomInfo->m_iElmoMainMonsterKill;
	uint8 subMobKill = bNation == (uint8)Nation::KARUS ? pRoomInfo->m_iKarusSubMonsterKill : pRoomInfo->m_iElmoSubMonsterKill;

	bool door = true;
	if (bNation == (uint8)Nation::KARUS)
	{
		for (int i = 0; i < 3; i++)
		{
			if (!pRoomInfo->m_sKarusBridges[i]) { door = false; break; }
		}
	}
	else if (bNation == (uint8)Nation::ELMORAD)
	{
		for (int i = 0; i < 3; i++)
		{
			if (!pRoomInfo->m_sElmoBridges[i]) { door = false; break; }
		}
	}

	if (!pRoomInfo->m_FinishPacketControl && ((mainMobKill == 13 && subMobKill == 60) || (GetProtoID() == DEVA_BIRD_SSID && door)))
	{

		pRoomInfo->m_FinishPacketControl = true;
		pRoomInfo->m_tFinishTimeCounter = UNIXTIME + 20;
		pRoomInfo->m_bWinnerNation = bNation;

		Packet newpkt1(WIZ_SELECT_MSG);
		newpkt1 << uint16(0) << uint8(7) << uint64(0);
		newpkt1 << uint32(6) << uint8(11) << uint32(500);
		Packet newpkt2(WIZ_EVENT);
		newpkt2 << uint8(TEMPLE_EVENT_FINISH)
			<< uint8(2) << uint8(0)
			<< pRoomInfo->m_bWinnerNation << uint32(20);

		g_pMain->Send_All(&newpkt1, nullptr, (uint8)Nation::ALL, ZONE_JURAID_MOUNTAIN, true, GetEventRoom());
		g_pMain->Send_All(&newpkt2, nullptr, (uint8)Nation::ALL, ZONE_JURAID_MOUNTAIN, true, GetEventRoom());
		return;
	}

	uint16 sSid = 0, sCount = 0, sRadius = 0, sDuration = 0, sEventRoom = GetEventRoom();
	int16 sSocketID = -1; uint8 ZoneID = GetZoneID(), nation = GetNation();
	float curX = GetX(), curY = GetY(), curZ = GetZ();

	int random_variable = myrand(0, 2);
	if (random_variable == 0) sSid = MONSTER_APOSTLE_SSID;
	else if (random_variable == 1) sSid = MONSTER_DOOM_SOLDIER_SSID;
	else if (random_variable == 2) sSid = MONSTER_TROLL_CAPTAIN_SSID;
	sCount = 5; sRadius = 6;

	g_pMain->SpawnEventNpc(sSid, true, ZoneID, curX, curY, curZ, sCount, sRadius, 0, (uint8)Nation::ALL, -1, GetEventRoom(), 0, 0, 0, (uint16)SpawnEventType::JuraidMonster);
	if (bNation == (uint8)Nation::KARUS)
	{
		if (mainMobKill == 4 && subMobKill == 20) HandleJuraidGateOpen(pRoomInfo->pkBridges[0], 0, pRoomInfo, Nation::KARUS);
		else if (mainMobKill == 8 && subMobKill == 40) HandleJuraidGateOpen(pRoomInfo->pkBridges[1], 1, pRoomInfo, Nation::KARUS);
		else if (mainMobKill == 12 && subMobKill == 60) HandleJuraidGateOpen(pRoomInfo->pkBridges[2], 2, pRoomInfo, Nation::KARUS);
	}
	else if (bNation == (uint8)Nation::ELMORAD)
	{
		if (mainMobKill == 4 && subMobKill == 20) HandleJuraidGateOpen(pRoomInfo->peBridges[0], 0, pRoomInfo, Nation::ELMORAD);
		else if (mainMobKill == 8 && subMobKill == 40) HandleJuraidGateOpen(pRoomInfo->peBridges[1], 1, pRoomInfo, Nation::ELMORAD);
		else if (mainMobKill == 12 && subMobKill == 60) HandleJuraidGateOpen(pRoomInfo->peBridges[2], 2, pRoomInfo, Nation::ELMORAD);
	}
}
#pragma endregion

#pragma region CNpc::HandleJuraidGateOpen(CNpc* pBridges, uint8 door, _JURAID_ROOM_INFO* pRoomInfo, Nation bNation)
void CNpc::HandleJuraidGateOpen(uint16 pBridgeid, uint8 door, _JURAID_ROOM_INFO* pRoomInfo, Nation bNation)
{
	if (door > 2 || !pBridgeid)
		return;

	auto* pBridges = g_pMain->GetNpcPtr(pBridgeid, ZONE_JURAID_MOUNTAIN);
	if (!pBridges) return;

	if (bNation == Nation::ELMORAD && pRoomInfo->m_sElmoBridges[door])
		return;
	else if (bNation == Nation::KARUS && pRoomInfo->m_sKarusBridges[door])
		return;

	if (bNation == Nation::ELMORAD)
		pRoomInfo->m_sElmoBridges[door] = true;
	else
		pRoomInfo->m_sKarusBridges[door] = true;

	pBridges->m_byGateOpen = 2;

	Packet result(WIZ_NPC_INOUT, uint8(InOutType::INOUT_OUT));
	result << pBridges->GetID();
	g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL, ZONE_JURAID_MOUNTAIN, true, pBridges->GetEventRoom());

	result.clear();
	result.Initialize(WIZ_NPC_INOUT);
	result << uint8(InOutType::INOUT_IN) << pBridges->GetID();
	pBridges->GetNpcInfo(result);
	g_pMain->Send_All(&result, nullptr, (uint8)Nation::ALL, ZONE_JURAID_MOUNTAIN, true, pBridges->GetEventRoom());
}
#pragma endregion

#pragma region CNpcThread::~CNpcThread()
CNpcThread::~CNpcThread()
{
	_running = false;

	m_npclist.clear();
	m_arNpcArray.DeleteAllData();
}
#pragma endregion

#pragma region CNpcThread::Shutdown()
void CNpcThread::Shutdown()
{
	_running = false;

	Sleep(1000);
	m_arNpcArray.DeleteAllData();
	m_npclist.clear();

	try
	{
		if (m_thread.joinable())
			m_thread.join();
	}
	catch (std::exception& ex)
	{
		printf("Caught thread exception: %s\n", ex.what());
	}
}
#pragma endregion