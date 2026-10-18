#include "stdafx.h"
#include "GameEvent.h"
#include "User.h"

CGameEvent::CGameEvent() : m_bType(0)
{
	memset(&m_iCond, 0, sizeof(m_iCond));
	memset(&m_iExec, 0, sizeof(m_iExec));
	m_sIndex = 0;
}

void CGameEvent::RunEvent(CUser *pUser, bool isest)
{
	switch (m_bType)
	{
	case ZONE_CHANGE:
	{
		if (pUser->m_bWarp) break;

		int32 m_ex1 = m_iExec[0], m_ex2 = m_iExec[1], m_ex3 = m_iExec[2];
		if (isest && pUser->GetNation() == (uint8)Nation::ELMORAD) {
			m_ex1 = ZONE_ELMORAD; m_ex2 = 678; m_ex3 = 194;
		}
		else if (isest && pUser->GetNation() == (uint8)Nation::KARUS) {
			m_ex1 = ZONE_KARUS; m_ex2 = 1365; m_ex3 = 1841;
		}
		pUser->ZoneChange(m_ex1, (float)m_ex2, (float)m_ex3);
	}
		break;
	case ZONE_TRAP_DEAD:
		//	TRACE("&&& User - zone trap dead ,, name=%s\n", pUser->GetName().c_str());
		//	pUser->Dead();
		break;	
	case ZONE_TRAP_AREA:
		pUser->TrapProcess();
		break;
	}
}
