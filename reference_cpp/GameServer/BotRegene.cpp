#include "StdAfx.h"

void CBot::Regene(uint8 regene_type, uint32 magicid /*= 0*/)
{
	if (GetMap() == nullptr)
		return;

	_OBJECT_EVENT* pEvent = nullptr;
	_START_POSITION* pStartPosition = nullptr;
	float x = 0.0f, z = 0.0f;

	if (!isDead())
		return;

	if (regene_type != 1 && regene_type != 2)
		regene_type = 1;

	// If we're in a home zone, we'll want the coordinates from there. Otherwise, assume our own home zone.
	pStartPosition = g_pMain->m_StartPositionArray.GetData(GetZoneID());
	if (pStartPosition == nullptr)
		return;

	BotInOut(INOUT_OUT);

	pEvent = GetMap()->GetObjectEvent(m_sBind);

	// If we're not using a spell to resurrect.
	if (magicid == 0)
	{
		// Resurrect at a bind/respawn point
		if (pEvent && pEvent->byLife == 1)
		{
			SetPosition(pEvent->fPosX + x, 0.0f, pEvent->fPosZ + z);
			x = pEvent->fPosX;
			z = pEvent->fPosZ;
		}
		// Are we trying to respawn in a home zone?
		// If we're in a war zone (aside from snow wars, which apparently use different coords), use BattleZone coordinates.
		else if ((GetZoneID() <= ZONE_ELMORAD) || (GetZoneID() != ZONE_SNOW_BATTLE && GetZoneID() == (ZONE_BATTLE_BASE + g_pMain->m_byBattleZone)))
		{
			// Use the proper respawn area for our nation, as the opposite nation can
			// enter this zone at a war's invasion stage.
			x = (float)((GetNation() == KARUS ? pStartPosition->sKarusX : pStartPosition->sElmoradX) + myrand(0, pStartPosition->bRangeX));
			z = (float)((GetNation() == KARUS ? pStartPosition->sKarusZ : pStartPosition->sElmoradZ) + myrand(0, pStartPosition->bRangeZ));
		}
		else
		{
			short sx, sz;
			// If we're in a war zone (aside from snow wars, which apparently use different coords), use BattleZone coordinates.
			if (isInMoradon())
			{
				x = (float)(MINI_ARENA_RESPAWN_X + myrand(-MINI_ARENA_RESPAWN_RADIUS, MINI_ARENA_RESPAWN_RADIUS));
				z = (float)(MINI_ARENA_RESPAWN_Z + myrand(-MINI_ARENA_RESPAWN_RADIUS, MINI_ARENA_RESPAWN_RADIUS));
			}
			else if (GetZoneID() == ZONE_CHAOS_DUNGEON)
			{
				GetStartPositionRandom(sx, sz);
				x = sx;
				z = sz;
			}
			else if (GetZoneID() == ZONE_JURAID_MOUNTAIN)
			{
				if (GetNation() == KARUS)
				{
					x = float(512 + myrand(0, 3));
					z = float(364 + myrand(0, 3));
				}
				else
				{
					x = float(512 + myrand(0, 3));
					z = float(659 + myrand(0, 3));
				}
			}
			// For all else, just grab the start position (/town coordinates) from the START_POSITION table.
			else
			{
				GetStartPosition(sx, sz);
				x = sx;
				z = sz;
			}
		}

		SetPosition(x, 0.0f, z);

		//m_LastX = x;
		//m_LastZ = z;

		m_bResHpType = USER_STANDING;
		m_BotState = BOT_MOVE;
	}
	else // we're respawning using a resurrect skill.
	{
		_MAGIC_TYPE5* pType = g_pMain->m_Magictype5Array.GetData(magicid);
		if (pType == nullptr)
			return;

		if (GetZoneID() != ZONE_UNDER_CASTLE)
			MSpChange(-((int32)m_MaxMp)); // reset us to 0 MP. 

		m_bResHpType = USER_STANDING;
		m_BotState = BOT_FARMER;
	}

	SetRegion(GetNewRegionX(), GetNewRegionZ());
	BotInOut(INOUT_RESPAWN);
	HpChange(GetMaxHealth());
	MSpChange(GetMaxMana());

	if (GetZoneID() == ZONE_UNDER_CASTLE)
		MSpChange(GetMaxMana());

	if (!isBlinking()
		&& GetZoneID() != ZONE_CHAOS_DUNGEON
		&& GetZoneID() != ZONE_DUNGEON_DEFENCE
		&& GetZoneID() != ZONE_KNIGHT_ROYALE)
	{
		InitType4();
		RecastSavedMagic();

		if (isInPKZone())
			Type4Change();
	}

	if (GetAngerGauge() > 0)
		UpdateAngerGauge(0);
}

bool CBot::GetStartPosition(short& x, short& z, uint8 bZone /*= 0 */)
{
	// Get start position data for current zone (unless we specified a zone).
	int nZoneID = (bZone == 0 ? GetZoneID() : bZone);
	_START_POSITION* pData = g_pMain->GetStartPosition(nZoneID);
	if (pData == nullptr)
		return false;

	// NOTE: This is how mgame does it.
	// This only allows for positive randomisation; we should really allow for the full range...
	if (GetNation() == KARUS)
	{
		x = pData->sKarusX + myrand(0, pData->bRangeX);
		z = pData->sKarusZ + myrand(0, pData->bRangeZ);
	}
	else
	{
		x = pData->sElmoradX + myrand(0, pData->bRangeX);
		z = pData->sElmoradZ + myrand(0, pData->bRangeZ);
	}

	return true;
}

bool CBot::GetStartPositionRandom(short& x, short& z, uint8 bZone)
{
	int nRandom = myrand(0, g_pMain->m_StartPositionRandomArray.GetSize() - 1);
	goto GetPosition;

GetPosition:
	{
		if (g_pMain->m_StartPositionRandomArray.GetData(nRandom)->ZoneID == (bZone == 0 ? GetZoneID() : bZone))
		{
			x = g_pMain->m_StartPositionRandomArray.GetData(nRandom)->PosX + myrand(0, g_pMain->m_StartPositionRandomArray.GetData(nRandom)->Radius);
			z = g_pMain->m_StartPositionRandomArray.GetData(nRandom)->PosZ + myrand(0, g_pMain->m_StartPositionRandomArray.GetData(nRandom)->Radius);
			return true;
		}

		nRandom = myrand(0, g_pMain->m_StartPositionRandomArray.GetSize() - 1);
		goto GetPosition;
	}

	return GetStartPosition(x, z);
}