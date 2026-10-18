#include "stdafx.h"

#pragma region CUser::DelosCasttellanZoneOut()
void CUser::DelosCasttellanZoneOut()
{
	CKnights * pKnight = g_pMain->GetClanPtr(g_pMain->pSiegeWar.sMasterKnights);
	if (pKnight == nullptr)
		return;

	for (uint16 i = 0; i < MAX_USER; i++)
	{
		CUser* pUser = g_pMain->GetUserPtr(i);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		if (pUser->GetZoneID() != ZONE_DELOS_CASTELLAN)
			continue;

		CKnights* pTKnight = g_pMain->GetClanPtr(pUser->GetClanID());
		if (pTKnight == nullptr)
			continue;

		if (pKnight->GetID() == pUser->GetClanID())
			continue;

		if (pTKnight->GetAllianceID() == g_pMain->pSiegeWar.sMasterKnights)
			continue;

		pUser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f);
	}
}
#pragma endregion

#pragma region CUser::isCswWinnerNembers()
bool CUser::isCswWinnerNembers()
{
	if (g_pMain->pSiegeWar.sMasterKnights == 0)
		return false;

	CKnights * pKnight = g_pMain->GetClanPtr(g_pMain->pSiegeWar.sMasterKnights);
	if (pKnight == nullptr)
		return false;

	CKnights * pTKnight = g_pMain->GetClanPtr(GetClanID());
	if (pTKnight == nullptr)
		return false;

	if (pKnight->GetID() == GetClanID())
	{
		ZoneChange(ZONE_DELOS, 458.0f, 113.0f);
		return true;
	}

	if (pTKnight->GetAllianceID() == g_pMain->pSiegeWar.sMasterKnights)
	{
		ZoneChange(ZONE_DELOS, 458.0f, 113.0f);
		return true;
	}

	return false;
}
#pragma endregion

#pragma region CUser::SiegeWarFareProcess(Packet & pkt)
void CUser::SiegeWarFareProcess(Packet & pkt)
{
	uint8 opcode, type;
	uint16 tarrif;
	pkt >> opcode >> type >> tarrif;

	CKnights * pKnight = g_pMain->GetClanPtr(g_pMain->pSiegeWar.sMasterKnights);
	
	Packet result(WIZ_SIEGE);
	switch (opcode)
	{
	case 1:
	{
		result << opcode << type;
		switch (type)
		{
		case 1:
			//CastleSiegeWarBaseCreate
			break;
		default:
			printf("Siege Npc Base Created unhandlec packets %d \n", type);
			TRACE("Siege Npc Base Created unhandlec packets %d \n", type);
			break;
		}
	}
	break;
	case 5:
		result << opcode << type;
		switch (type)
		{
		case 1:
			//CastleSiegeWarRegisterClanShow
			break;
		case 2:
			csw_rank();
			break;
		default:
			printf("Siege Rank unhandlec packets %d \n", type);
			TRACE("Siege Rank Created unhandlec packets %d \n", type);
			break;
		}
		break;
	case 3: //moradon npc
	{
		result << opcode << type;
		switch (type)
		{
		case 2:
			result << g_pMain->pSiegeWar.sCastleIndex
				<< uint16(g_pMain->pSiegeWar.bySiegeType)
				<< g_pMain->pSiegeWar.byWarDay
				<< g_pMain->pSiegeWar.byWarTime
				<< g_pMain->pSiegeWar.byWarMinute;
			Send(&result);
			break;
		case 4:
			if (pKnight == nullptr)
				return;

			result.SByte();
			result
				<< g_pMain->pSiegeWar.sCastleIndex
				<< uint8(1)
				<< pKnight->GetName()
				<< pKnight->m_byNation
				<< pKnight->m_sMembers
				<< g_pMain->pSiegeWar.byWarRequestDay
				<< g_pMain->pSiegeWar.byWarRequestTime
				<< g_pMain->pSiegeWar.byWarRequestMinute;
			Send(&result);
			break;
		case 5:
			if (pKnight == nullptr)
				return;

			result.SByte();
			result
				<< g_pMain->pSiegeWar.sCastleIndex
				<< g_pMain->pSiegeWar.bySiegeType
				<< pKnight->GetName()
				<< pKnight->m_byNation
				<< pKnight->m_sMembers;
			Send(&result);
			break;

		default:
			printf("Siege Npc Maradon unhandled packets %d \n", type);
			TRACE("Siege Npc Maradon unhandled packets %d \n", type);
			break;
		}
	}
	break;
	case 4: //delos npc
	{
		result << opcode << type;
		switch (type)
		{
		case 2: // get all coins from the fund
			if (isKing())
			{
				if (g_pMain->pSiegeWar.nDellosTax + g_pMain->pSiegeWar.nMoradonTax + GetCoins() > COIN_MAX)
				{
					result.Initialize(WIZ_QUEST);
					result << uint8(13) << uint8(2);
					Send(&result);
					return;
				}
				else
				{
					uint32 gold = 0;
					gold = g_pMain->pSiegeWar.nDellosTax + g_pMain->pSiegeWar.nMoradonTax;
					GoldGain(gold, true);
					g_pMain->pSiegeWar.nDellosTax = 0;
					g_pMain->pSiegeWar.nMoradonTax = 0;
				}
			}
			else
			{
				if (g_pMain->pSiegeWar.nDungeonCharge + GetCoins() > COIN_MAX)
				{
					result.Initialize(WIZ_QUEST);
					result << uint8(13) << uint8(2);
					Send(&result);
					return;
				}
				else
				{
					GoldGain(g_pMain->pSiegeWar.nDungeonCharge, true);
					g_pMain->pSiegeWar.nDungeonCharge = 0;
				}
			}
			// Tell database to update
			g_pMain->UpdateSiegeTax(0, 0);
			break;
		case 3:
			if (isKing())
				return;

			result << g_pMain->pSiegeWar.sCastleIndex
				<< g_pMain->pSiegeWar.sMoradonTariff
				<< g_pMain->pSiegeWar.sDellosTariff
				<< g_pMain->pSiegeWar.nDungeonCharge;
			Send(&result);
			break;
		case 4:
		{
			if (tarrif > 20 || isKing())
				return;

			g_pMain->pSiegeWar.sMoradonTariff = tarrif;

			C3DMap * pMap = g_pMain->GetZoneByID(ZONE_MORADON);
			if (pMap != nullptr)
				pMap->SetTariff(uint8(g_pMain->pSiegeWar.sMoradonTariff));

			pMap = g_pMain->GetZoneByID(ZONE_MORADON2);
			if (pMap != nullptr)
				pMap->SetTariff(uint8(g_pMain->pSiegeWar.sMoradonTariff));

			pMap = g_pMain->GetZoneByID(ZONE_MORADON3);
			if (pMap != nullptr)
				pMap->SetTariff(uint8(g_pMain->pSiegeWar.sMoradonTariff));

			pMap = g_pMain->GetZoneByID(ZONE_MORADON4);
			if (pMap != nullptr)
				pMap->SetTariff(uint8(g_pMain->pSiegeWar.sMoradonTariff));

			pMap = g_pMain->GetZoneByID(ZONE_MORADON5);
			if (pMap != nullptr)
				pMap->SetTariff(uint8(g_pMain->pSiegeWar.sMoradonTariff));

			result << uint16(1) << tarrif << uint8(ZONE_MORADON);
			g_pMain->Send_All(&result);

			// Tell database to update
			g_pMain->UpdateSiegeTax(ZONE_MORADON, g_pMain->pSiegeWar.sMoradonTariff);
		}
		break;
		case 5:
		{
			if (tarrif > 20 || isKing())
				return;

			g_pMain->pSiegeWar.sDellosTariff = tarrif;

			C3DMap * pMap = g_pMain->GetZoneByID(ZONE_DELOS);
			if (pMap != nullptr)
				pMap->SetTariff(uint8(g_pMain->pSiegeWar.sDellosTariff));

			pMap = g_pMain->GetZoneByID(ZONE_DESPERATION_ABYSS);
			if (pMap != nullptr)
				pMap->SetTariff(uint8(g_pMain->pSiegeWar.sDellosTariff));

			pMap = g_pMain->GetZoneByID(ZONE_HELL_ABYSS);
			if (pMap != nullptr)
				pMap->SetTariff(uint8(g_pMain->pSiegeWar.sDellosTariff));

			result << uint16(1) << tarrif << uint8(ZONE_DELOS);
			g_pMain->Send_All(&result);

			// Tell database to update
			g_pMain->UpdateSiegeTax(ZONE_DELOS, g_pMain->pSiegeWar.sDellosTariff);
		}
		break;
		default:
			printf("Siege Npc Dels unhandled packets %d \n", type);
			TRACE("Siege Npc Dels unhandled packets %d \n", type);
			break;
		}
	}
	break;
	default:
		printf("Siege Npc - unhandled packets type: %d opcode: %d\n", type, opcode);
		TRACE("Siege Npc - unhandled packets type: %d opcode: %d\n", type, opcode);
		break;
	}
}
#pragma endregion
