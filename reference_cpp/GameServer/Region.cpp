#include "stdafx.h"
#include "Region.h"
#include "User.h"
#include "Npc.h"

void CRegion::Add(CUser* pUser)
{
	if (pUser == nullptr || !pUser->isInGame()) return;

	Guard lock(m_lockUserArray);
	m_RegionUserArray.insert(pUser->GetID());

	m_byMoving = !m_RegionUserArray.empty();
}

void CRegion::Remove(CUser* pUser)
{
	if (pUser == nullptr) return;
	Guard lock(m_lockUserArray);
	m_RegionUserArray.erase(pUser->GetID());
	m_byMoving = !m_RegionUserArray.empty();
}

void CRegion::Add(CNpc* pNpc)
{
	if (pNpc == nullptr) return;
	Guard lock(m_lockNpcArray);
	m_RegionNpcArray.insert(pNpc->GetID());
	if (pNpc->GetPetID() >= 0 || pNpc->GetGuardID() >= 0)
		m_byMoving = !m_RegionNpcArray.empty();
}

void CRegion::Remove(CNpc* pNpc)
{
	if (pNpc == nullptr) return;
	Guard lock(m_lockNpcArray);
	m_RegionNpcArray.erase(pNpc->GetID());
	if (pNpc->GetPetID() >= 0 || pNpc->GetGuardID() >= 0)
		m_byMoving = !m_RegionNpcArray.empty();
}


void CRegion::Add(CBot * pBot)
{
	if (pBot == nullptr)
		return;

	Guard lock(m_lockBotArray);
	m_RegionBotArray.insert(pBot->GetID());
	m_byMoving = !m_RegionBotArray.empty();
}

void CRegion::Remove(CBot * pBot)
{
	if (pBot == nullptr)
		return;

	Guard lock(m_lockBotArray);
	m_RegionBotArray.erase(pBot->GetID());
	m_byMoving = !m_RegionBotArray.empty();
}