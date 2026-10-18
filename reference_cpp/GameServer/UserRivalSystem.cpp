#include "stdafx.h"

void CUser::SetRival(Unit * pRival)
{
	if (pRival == nullptr
		|| hasRival())
		return;

	m_sRivalID = pRival->GetID();
	m_tRivalExpiryTime = UNIXTIME + RIVALRY_DURATION;

	Packet result(WIZ_PVP, uint8(PVPOpcodes::PVPAssignRival));
	result << pRival->GetID() << GetCoins() << GetLoyalty();

	uint16 recvClanID = 0;
	if (pRival->isPlayer())
		recvClanID = TO_USER(pRival)->GetClanID();
	else
		recvClanID = TO_BOT(pRival)->GetClanID();

	CKnights* pKnights = g_pMain->GetClanPtr(recvClanID);
	if (pKnights != nullptr)
		result << pKnights->GetName();
	else
		result << uint16(0);

	result << pRival->GetName();
	Send(&result);
}

/**
* @brief	Removes our rivalry state.
*/
void CUser::RemoveRival()
{
	if (!hasRival())
		return;

	// Reset our rival data
	m_tRivalExpiryTime = 0;
	m_sRivalID = -1;

	// Send the packet to let the client know that our rivalry has ended
	Packet result(WIZ_PVP, uint8(PVPOpcodes::PVPRemoveRival));
	Send(&result);
}

/**
* @brief	Updates the player's anger gauge level, setting it to
* 			byAngerGauge.
*
* @param	byAngerGauge	The anger gauge level.
*/
void CUser::UpdateAngerGauge(uint8 byAngerGauge)
{
	Packet result(WIZ_PVP, uint8(byAngerGauge == 0 ? PVPOpcodes::PVPResetHelmet : PVPOpcodes::PVPUpdateHelmet));

	if (byAngerGauge > MAX_ANGER_GAUGE)
		byAngerGauge = MAX_ANGER_GAUGE;

	m_byAngerGauge = byAngerGauge;
	if (byAngerGauge > 0)
		result << uint8(byAngerGauge) << hasFullAngerGauge();

	Send(&result);
}