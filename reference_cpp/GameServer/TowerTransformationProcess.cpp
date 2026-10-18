#include "stdafx.h"
#include "MagicInstance.h"

void CUser::TowerExitsFunciton(bool Dead)
{
	if (!isTowerOwner()) return;

	CNpc* pNpc = g_pMain->GetNpcPtr(GetTowerID(), GetZoneID());
	if (GetTowerID() == -1 || pNpc == nullptr || pNpc->GetType() != 191) return;
	if (!pNpc->m_isTowerOwner) return;

	pNpc->StateChange((uint8)NpcState::NPC_SHOW);
	pNpc->m_isTowerOwner = false;
	m_TowerOwnerID = -1;

	if (Dead) {
		Packet result(WIZ_MOVING_TOWER, uint8(17));
		result << uint8(1) << GetSocketID() << pNpc->GetID();
		Send(&result);

		AbnormalType abtype = AbnormalType::ABNORMAL_NORMAL;
		if (isGM() && m_bAbnormalType == (uint32)AbnormalType::ABNORMAL_INVISIBLE) abtype = AbnormalType::ABNORMAL_INVISIBLE;
		StateChangeServerDirect(3, (uint32)abtype);
	}
}

#pragma region CUser::HandleTowerPackets(Packet & pkt)
void CUser::HandleTowerPackets(Packet & pkt)
{
	uint8 command = pkt.read<uint8>();
	Packet result(WIZ_MOVING_TOWER);
	switch (command)
	{
	case 1:
	{
		if (!isInClan() || GetZoneID() != ZONE_DELOS || !g_pMain->m_byBattleSiegeWarOpen) return;

		CUser * pTUser = g_pMain->GetUserPtr(GetTargetID());
		if (pTUser == nullptr || pTUser->m_transformSkillUseid != TransformationSkillUse::TransformationSkillMovingTower) return;

		StateChangeServerDirect(11, (uint32)AbnormalType::BOARDING);
		UserInOut(InOutType::INOUT_OUT);

		//Warp(uint16(pTUser->GetX() * 10), uint16(pTUser->GetZ() * 10));
		result << command << uint8(1);
		Send(&result);
	}
	break;
	case 2:
	{
		if (!isInClan() || GetZoneID() != ZONE_DELOS || !g_pMain->m_byBattleSiegeWarOpen) return;

		uint16 POSX = pkt.read<uint16>();
		uint16 POSZ = pkt.read<uint16>();
		result << command << uint8(1);
		StateChangeServerDirect(11, (uint32)AbnormalType::ABNORMAL_NORMAL);
		Warp(POSX, POSZ);
		Send(&result);
	}
	break;
	case 16:
	{
		uint16 NpcID = pkt.read<uint16>();
		if (GetTowerID() != -1 || GetZoneID() != ZONE_BATTLE6 || !g_pMain->isWarOpen()) return;

		CNpc* pNpc = g_pMain->GetNpcPtr(NpcID, GetZoneID());
		if (pNpc == nullptr || pNpc->GetType() != 191 || pNpc->m_isTowerOwner || !isInRange(pNpc, MAX_NPC_RANGE)) return;

		pNpc->StateChange((uint8)NpcState::NPC_HIDE);
		m_TowerOwnerID = pNpc->GetID(); pNpc->m_isTowerOwner = true;

		Warp((uint16)pNpc->GetX() * 10, (uint16)pNpc->GetZ() * 10);
		StateChangeServerDirect(3, 450018);
		result << command << uint8(1) << GetID() << pNpc->GetID()
			<< (uint16)pNpc->GetSPosX() << (uint16)pNpc->GetSPosZ() << (uint16)pNpc->GetSPosY();
		Send(&result);
	}
	break;
	case 17:
	{
		if (GetTowerID() == -1 || GetZoneID() != ZONE_BATTLE6 || !g_pMain->isWarOpen()) return;
		CNpc* pNpc = g_pMain->GetNpcPtr(GetTowerID(), GetZoneID());
		if (pNpc == nullptr || pNpc->GetType() != 191 || !pNpc->m_isTowerOwner) return;

		pNpc->StateChange((uint8)NpcState::NPC_SHOW);
		pNpc->m_isTowerOwner = false;
		m_TowerOwnerID = -1;

		AbnormalType abtype = AbnormalType::ABNORMAL_NORMAL;
		if (isGM() && m_bAbnormalType == (uint32)AbnormalType::ABNORMAL_INVISIBLE) abtype = AbnormalType::ABNORMAL_INVISIBLE;
		StateChangeServerDirect(3, (uint32)abtype);

		InitType4();
		RecastSavedMagic();
		result << command << uint8(1) << GetID();
		Send(&result);
	}
	break;
	default:
		printf("Moving Tower unhandled packets %d \n", command);
		TRACE("Moving Tower unhandled packets %d \n", command);
		break;
	}
}
#pragma endregion

/**
* @brief	Changes a player's fame.
*
* @param	bFame	The fame.
*/
void CUser::ChangeFame(uint8 bFame)
{
	Packet result(WIZ_AUTHORITY_CHANGE, uint8(COMMAND_AUTHORITY));

	m_bFame = bFame;
	result << GetSocketID() << GetFame();
	SendToRegion(&result);
}