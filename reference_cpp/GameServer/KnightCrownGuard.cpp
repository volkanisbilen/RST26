#include "stdafx.h"

#define __GUARD_VERSION 2229

void CUser::SpeedHackTime(Packet & pkt)
{
	if (!isInGame() || isGM() || isGMUser())
		return;

	float nSpeed = 45.0f;

	if (GetFame() == COMMAND_CAPTAIN || isRogue())
		nSpeed = 90.0f;

	else if (isWarrior() || isMage() || isPriest())
		nSpeed = 67.0f;
	bool isBack = false;
	nSpeed += 17.0f;

	float nRange = (pow(GetX() - m_LastX, 2.0f) + pow(GetZ() - m_LastZ, 2.0f)) / 100.0f;

	if (nRange >= nSpeed)
		isBack = true;
	if (isBack)
	{
		Warp(uint16(m_LastX) * 10, uint16(m_LastZ) * 10);
	}
	else
	{

		m_LastX = GetX();
		m_LastZ = GetZ();
	}

}
