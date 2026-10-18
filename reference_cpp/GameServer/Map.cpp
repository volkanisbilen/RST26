#include "stdafx.h"
#include <set>
#include "../shared/SMDFile.h"

/* passthru methods */
/* passthru methods */
int C3DMap::GetXRegionMax() { return m_smdFile->GetXRegionMax(); }
int C3DMap::GetZRegionMax() { return m_smdFile->GetZRegionMax(); }
bool C3DMap::IsValidPosition(float x, float z, float y) { return m_smdFile->IsValidPosition(x, z, y); }
_REGENE_EVENT * C3DMap::GetRegeneEvent(int objectindex) { return m_smdFile->GetRegeneEvent(objectindex); }
_WARP_INFO * C3DMap::GetWarp(int warpID) { return m_smdFile->GetWarp(warpID); }
void C3DMap::GetWarpList(int warpGroup, std::set<_WARP_INFO *> & warpEntries) { m_smdFile->GetWarpList(warpGroup, warpEntries); }
int C3DMap::GetMapSize() { return m_smdFile->GetMapSize(); }
float C3DMap::GetUnitDistance() { return m_smdFile->GetUnitDistance(); }

C3DMap::C3DMap() { Initialize2(); }

void C3DMap::Initialize2()
{
	m_smdFile = nullptr;
	m_ppRegion = nullptr;
	m_nZoneNumber = 0;
	m_sMaxUser = MAX_USER;
	m_wBundle = 1;
	m_nServerNo = 0;
	m_Status = 0;
	m_MinLevel = 0;
	m_MaxLevel = 0;
	m_ZoneType = 0;
	m_kTradeOtherNation = 0;
	m_kTalkOtherNation = 0;
	m_kAttackOtherNation = 0;
	m_kAttackSameNation = 0;
	m_kFriendlyNpc = 0;
	m_kWarZone = 0;
	m_kClanUpdates = 0;
	m_bTeleport = 0;
	m_bGate = 0;
	m_bEscape = 0;
	m_bCallingFriend = 0;
	m_bTeleportFriend = 0;
	m_bBlink = 0;
	m_bPetSpawn = 0;
	m_bExpLost = 0;
	m_bGiveLoyalty = 0;
	m_bGuardSummon = 0;
	m_bMenissiahList = 0;
	m_bMilitaryZone = 0;
	m_fInitX = 0.0f;
	m_fInitY = 0.0f;
	m_fInitZ = 0.0f;
}

bool C3DMap::Initialize(_ZONE_INFO *pZone)
{
	if (!pZone)
		return false;

	m_nServerNo = pZone->m_nServerNo;
	m_nZoneNumber = pZone->m_nZoneNumber;
	m_nZoneName = pZone->m_ZoneName;
	m_Status = pZone->m_Status;
	m_MinLevel = pZone->m_MinLevel;
	m_MaxLevel = pZone->m_MaxLevel;
	m_ZoneType = pZone->m_ZoneType;
	m_kTradeOtherNation = pZone->m_kTradeOtherNation;
	m_kTalkOtherNation = pZone->m_kTalkOtherNation;
	m_kAttackOtherNation = pZone->m_kAttackOtherNation;
	m_kAttackSameNation = pZone->m_kAttackSameNation;
	m_kFriendlyNpc = pZone->m_kFriendlyNpc;
	m_kWarZone = pZone->m_kWarZone;
	m_kClanUpdates = pZone->m_kClanUpdates;
	m_bTeleport = pZone->m_bTeleport;
	m_bGate = pZone->m_bGate;
	m_bEscape = pZone->m_bEscape;
	m_bCallingFriend = pZone->m_bCallingFriend;
	m_bTeleportFriend = pZone->m_bTeleportFriend;
	m_bBlink = pZone->m_bBlink;
	m_bPetSpawn = pZone->m_bPetSpawn;
	m_bExpLost = pZone->m_bExpLost;
	m_bGiveLoyalty = pZone->m_bGiveLoyalty;
	m_bGuardSummon = pZone->m_bGuardSummon;
	m_bMenissiahList = pZone->m_bMenissiahList;
	m_bMilitaryZone = pZone->m_bMilitaryZone;
	m_bMiningZone = pZone->m_bMiningZone;
	m_bBlinkZone = pZone->m_bBlinkZone;
	m_bAutoLoot = pZone->m_bAutoLoot;
	m_bGoldLose = pZone->m_bGoldLose;
	m_fInitX = pZone->m_fInitX;
	m_fInitY = pZone->m_fInitY;
	m_fInitZ = pZone->m_fInitZ;

	m_smdFile = SMDFile::Load(pZone->m_MapName, true /* load warps & regene events */);

	if (m_smdFile != nullptr)
	{
		SetZoneAttributes(pZone); // 
		m_ppRegion = new CRegion*[m_smdFile->m_nXRegion];
		for (int i = 0; i < m_smdFile->m_nXRegion; i++)
			m_ppRegion[i] = new CRegion[m_smdFile->m_nZRegion];
	}

	return (m_smdFile != nullptr);
}

CRegion * C3DMap::GetRegion(uint16 regionX, uint16 regionZ)
{
	if (regionX > GetXRegionMax()
		|| regionZ > GetZRegionMax())
		return nullptr;

	Guard lock(m_lock);
	return &m_ppRegion[regionX][regionZ];
}

bool C3DMap::RegionItemAdd(_LOOT_BUNDLE * pBundle)
{
	if (pBundle == nullptr)
		return false;

	m_wBundle++;

	if (m_RegionItemArray.IsExist(m_wBundle))
	{
		/*_LOOT_BUNDLE* pBundlex = m_RegionItemArray.GetData(m_wBundle, false);
		if (pBundlex == pBundle)
			printf("%d m_wBundle value is stacked! : %s\n", m_wBundle, pBundle->pLooter->GetName().c_str());
		else
			printf("%d m_wBundle value is stacked!\n", m_wBundle);*/
		return false;
	}

	pBundle->nBundleID = m_wBundle;
	m_RegionItemArray.PutData(pBundle->nBundleID, pBundle);

	if (m_wBundle > ZONEITEM_MAX)
		m_wBundle = 1;

	return true;
}


void C3DMap::RegionItemRemove(_LOOT_BUNDLE * pBundle, uint16 slotid)
{

	if(!pBundle || slotid >= NPC_HAVE_ITEM_LIST) return;

	if (pBundle->ItemsCount > 0)pBundle->ItemsCount--;
	pBundle->Items[slotid].nItemID = 0;
	pBundle->Items[slotid].sCount = 0;
	pBundle->Items[slotid].slotid = 0;

	if (pBundle->ItemsCount > 0) return;
	m_RegionItemArray.DeleteData(pBundle->nBundleID);
}

bool C3DMap::CheckEvent(float x, float z, CUser* pUser)
{
	if (pUser == nullptr || !pUser->isInGame())
		return false;

	CGameEvent *pEvent = nullptr;

	int event_index = m_smdFile->GetEventID((int)(x / m_smdFile->GetUnitDistance()), (int)(z / m_smdFile->GetUnitDistance()));
	if (event_index < 2)
	{
		if (g_pMain->m_byBattleOpen == NATION_BATTLE 
			&& pUser->GetMap()->isWarZone() 
			&& g_pMain->m_byBattleZoneType == 0)
		{
			pEvent = m_EventArray.GetData(1010 + (pUser->GetNation() == (uint8)Nation::ELMORAD ? 1 : 2));

			if (pEvent != nullptr)
			{
				if ((x > pEvent->m_iCond[0] && x < pEvent->m_iCond[1]) 
					&& (z > pEvent->m_iCond[2] && z < pEvent->m_iCond[3]))
					pUser->ZoneChange(pEvent->m_iExec[0],(float)pEvent->m_iExec[1],(float)pEvent->m_iExec[2]);
				else
				{
					pEvent = m_EventArray.GetData(1010 + pUser->GetNation());

					if (pEvent != nullptr)
					{
						if ((x > pEvent->m_iCond[0] && x < pEvent->m_iCond[1]) 
							&& (z > pEvent->m_iCond[2] && z < pEvent->m_iCond[3]))
							if (g_pMain->m_bVictory == pUser->GetNation())
								pUser->ZoneChange(pEvent->m_iExec[0],(float)pEvent->m_iExec[1],(float)pEvent->m_iExec[2]);
					}
				}
			}

			return false;
		}
		else
			return false;
	}

	if (!pUser->isInKarusEslant() && !pUser->isInElmoradEslant())
	{
		if (g_pMain->m_byBattleOpen == NATION_BATTLE)
			event_index += g_pMain->m_byBattleZone - 1;
		else if (g_pMain->m_byBattleOpen == SNOW_BATTLE)
			event_index += g_pMain->m_byBattleZone + 10;
	}

	pEvent = m_EventArray.GetData(event_index);

	if (pEvent == nullptr)
		return false;

	if (pEvent->m_bType == 1)
	{
		if (pEvent->m_iExec[0] > ZONE_BATTLE_BASE && pEvent->m_iExec[0] <= ZONE_BATTLE6)
		{
			if (g_pMain->m_byBattleOpen != NATION_BATTLE)
				return false;
		}

		if (pEvent->m_iExec[0] == ZONE_SNOW_BATTLE)
		{
			if (g_pMain->m_byBattleOpen != SNOW_BATTLE)
				return false;
		}

	}
	else if (pEvent->m_iExec[0] > ZONE_BATTLE_BASE && pEvent->m_iExec[0] <= ZONE_BATTLE6)
	{
		if ((pUser->GetNation() == (uint8)Nation::KARUS && g_pMain->m_sKarusCount > m_sMaxUser)
			|| (pUser->GetNation() == (uint8)Nation::ELMORAD && g_pMain->m_sElmoradCount > m_sMaxUser))
		{
			//TRACE("%s cannot enter war zone %d, too many users.\n", pUser->GetName().c_str(), pEvent->m_iExec[0]);
			return false;
		}

		if (g_pMain->m_byBattleZoneType == ZONE_ARDREAM
			&& (pUser->GetLevel() < MIN_LEVEL_NIEDS_TRIANGLE
				|| pUser->GetLevel() > MAX_LEVEL_NIEDS_TRIANGLE
				|| !pUser->CanLevelQualify(MAX_LEVEL_NIEDS_TRIANGLE)))
			return false;
	}

	bool ist = false;
	if (pEvent->m_sIndex == 1231 || pEvent->m_sIndex == 1131)ist = true;
	pEvent->RunEvent(pUser, ist);
	return true;
}

_OBJECT_EVENT * C3DMap::GetObjectEvent(int objectindex) 
{ 
	foreach_stlmap(itr, g_pMain->m_ObjectEventArray)
	{
		_OBJECT_EVENT* pEvent = itr->second;
		if (pEvent == nullptr)
			continue;

		if (pEvent->sZoneID == m_nZoneNumber && pEvent->sIndex == objectindex)
			return pEvent;
	}

	return nullptr;
}

bool  C3DMap::IsMovable(int dest_x, int dest_y)
{
	return m_smdFile->GetEventID(dest_x, dest_y) == 0;
}

C3DMap::~C3DMap()
{
	m_EventArray.DeleteAllData();

	if (m_ppRegion != nullptr)
	{
		for (int i = 0; i <= GetXRegionMax(); i++)
			delete [] m_ppRegion[i];

		delete [] m_ppRegion;
		m_ppRegion = nullptr;
	}

	if (m_smdFile != nullptr)
		m_smdFile->DecRef();
}