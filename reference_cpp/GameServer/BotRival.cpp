#include "StdAfx.h"

void CBot::SetRival(CBot* pRival)
{
	if (pRival == nullptr
		|| hasRival())
		return;

	m_sRivalID = pRival->GetID();
	m_tRivalExpiryTime = UNIXTIME + RIVALRY_DURATION;
}

void CBot::SetRival(CUser* pRival)
{
	if (pRival == nullptr
		|| hasRival())
		return;

	m_sRivalID = pRival->GetID();
	m_tRivalExpiryTime = UNIXTIME + RIVALRY_DURATION;
}

void CBot::RemoveRival()
{
	if (!hasRival())
		return;

	// Reset our rival data
	m_tRivalExpiryTime = 0;
	m_sRivalID = -1;
}

void CBot::UpdateAngerGauge(uint8 byAngerGauge)
{
	Packet result(WIZ_PVP, uint8(byAngerGauge == 0 ? PVPResetHelmet : PVPUpdateHelmet));

	if (byAngerGauge > MAX_ANGER_GAUGE)
		byAngerGauge = MAX_ANGER_GAUGE;

	m_byAngerGauge = byAngerGauge;
	if (byAngerGauge > 0)
		result << uint8(byAngerGauge) << hasFullAngerGauge();

	SendToRegion(&result);
}