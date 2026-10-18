#include "StdAfx.h"

void CBot::MoveProcess(float X, float Y, float Z, uint16 will_x, uint16 will_y, uint16 will_z, float sSpeed, uint8 echo)
{
	if (!isInGame() || GetMap() == nullptr)
		return;

	if (isDead())
		return;

	if (!GetMap()->IsValidPosition(X, Z, Y))
		return;

	if (m_oldx != GetX()
		|| m_oldz != GetZ())
	{
		m_oldx = GetX();
		m_oldy = GetY();
		m_oldz = GetZ();
	}

	// TODO: Ensure this is checked properly to prevent speedhacking
	SetPosition(X, Y, Z); RegisterRegion();
	m_sRegionAttackTime = UNIXTIME2;

	/*if (m_bInvisibilityType == INVIS_DISPEL_ON_MOVE)
		CMagicProcess::RemoveStealth(this, INVIS_DISPEL_ON_MOVE);*/

	Packet result(WIZ_MOVE);
	result << GetID() << will_x << will_z << will_y << sSpeed << echo;
	SendToRegion(&result);
}

void CBot::MoveRegionProcess(float X, float Y, float Z, uint16 will_x, uint16 will_y, uint16 will_z, float sSpeed, uint8 echo)
{
	if (!isInGame() || GetMap() == nullptr)
		return;

	if (isDead())
		return;

	if (!GetMap()->IsValidPosition(X, Z, Y))
		return;

	if (m_oldx != GetX()
		|| m_oldz != GetZ())
	{
		m_oldx = GetX();
		m_oldy = GetY();
		m_oldz = GetZ();
	}

	// TODO: Ensure this is checked properly to prevent speedhacking
	SetPosition(X, Y, Z); RegisterRegion();

	//if (m_bInvisibilityType == INVIS_DISPEL_ON_MOVE)
	//	CMagicProcess::RemoveStealth(this, INVIS_DISPEL_ON_MOVE);

	/*if (GetBotState() == BOT_MOVE)
		m_sMoveRegionAttackTime = UNIXTIME2;*/

	Packet result(WIZ_MOVE);
	result << GetID() << will_x << will_z << will_y << sSpeed << echo;
	SendToRegion(&result);
}

void CBot::MoveProcessGoDeahTown()
{
	if (isPriest() || isMage())
	{
		if (m_sSkillCoolDown > (uint32)UNIXTIME)
			return;
	}

	switch (GetZoneID())
	{
	case ZONE_RONARK_LAND:
		MoveProcessRonarkLandTown();
		break;
	case ZONE_ARDREAM:
	case ZONE_SPBATTLE2:
		MoveProcessArdreamLandTown();
		break;
	case ZONE_RONARK_LAND_BASE:
		break;
	case ZONE_SPBATTLE1:
		MoveProcessSpecialEventZindan();
		break;
	case ZONE_SPBATTLE3:
		MoveProcessSpecialMiniRonarkLand();
		break;
	case ZONE_SPBATTLE4:
		MoveProcessSpecialBaseWar();
		break;
	case ZONE_SPBATTLE5:
		MoveProcessSpecialMoradonWar();
		break; 
	case ZONE_SPBATTLE6:
		MoveProcessSpecialSnowWar();
		break;
	case ZONE_SPBATTLE7:
		MoveProcessSpecialNightDelosWar();
		break;
	case ZONE_SPBATTLE9:
		MoveProcessSpecialNeoStarWar();
		break;
	case ZONE_SPBATTLE10:
		MoveProcessSpecialCastleWar();
		break;
	case ZONE_SPBATTLE11:
		MoveProcessSpecialMiniArdreamWar();
		break;
	case ZONE_BIFROST:
		MoveProcessSpecialBifrost();
		break;
	case ZONE_DELOS:
		MoveProcessSpecialDelos();
		break;
	}
}

void CBot::WalkRegionCordinat(int16 m_sSocketID, float x, float y, float z, uint16 Delay, bool isAttack)
{
	if (m_sMoveRegionTime > UNIXTIME2)
		return;

	m_sMoveRegionTime = UNIXTIME2 + 1000;

	float Mesafe = 0.0f, EnYakinMesafe = 0.0f;
	__Vector3 vBot, vUser, vDistance, vRealDistance;

	Mesafe = pow(x - GetX(), 2.0f) + pow(z - GetZ(), 2.0f);
	if (Mesafe == EnYakinMesafe)
	{
		if (Delay > 0)
			Delay = myrand(1, Delay);

		m_sMoveRegionTime = UNIXTIME2 + (Delay * 1000);
		if (WalkStep == 15)
			m_Reverse = true;

		if (WalkStep == 0)
			m_Reverse = false;

		if (m_Reverse)
			WalkStep--;
		else
			WalkStep++;

		float(x + myrand(1, 15) * 2.0f);
		float(z + myrand(1, 15) * 2.0f);
	}

	if (isAttack)
	{
		if (!isRegionTargetUp())
			m_sRegionAttack = true;
	}

	vBot.Set(GetX(), GetY(), GetZ());
	vUser.Set(x + ((myrand(0, 2000) - 1000.0f) / 500.0f), y, z + ((myrand(0, 2000) - 1000.0f) / 500.0f));

	if (m_sTargetID != m_sSocketID)
	{
		m_TargetChanged = true;
		m_sTargetID = m_sSocketID;
		m_oldx = vUser.x + ((myrand(0, 2000) - 1000.0f) / 500.0f);
		m_oldz = vUser.z + ((myrand(0, 2000) - 1000.0f) / 500.0f);
		m_oldy = vUser.y;
	}

	vDistance = vUser - vBot;
	vRealDistance = vDistance;
	vDistance.Normalize();

	float speed = m_sSpeed;
	uint8 KosuSuresi = 1;
	bool sRunFinish = false;
	vDistance *= speed / 10.0f;

	if (echo == uint8(0)
		&& vDistance.Magnitude() < vRealDistance.Magnitude()
		&& (vDistance * 100.0f).Magnitude() < vRealDistance.Magnitude())
	{
		vDistance *= 100.0f;
		KosuSuresi = 100;
	}
	else if (vDistance.Magnitude() > vRealDistance.Magnitude()
		|| vDistance.Magnitude() == vRealDistance.Magnitude())
	{
		sRunFinish = true;
		vDistance = vRealDistance;
	}

	if (m_TargetChanged)
	{
		m_TargetChanged = false;
		echo = uint8(1);
	}
	else if (sRunFinish)
		echo = uint8(0);
	else
		echo = uint8(3);

	uint16 will_x, will_z, will_y;
	will_x = uint16((vBot + vDistance).x * 10.0f);
	will_y = uint16(vUser.y * 10.0f);
	will_z = uint16((vBot + vDistance).z * 10.0f);
	MoveRegionProcess((vBot + vDistance).x, vUser.y, (vBot + vDistance).z, will_x, will_y, will_z, speed, echo);
}

void CBot::WalkCordinat(float x, float y, float z, uint16 Delay, bool isAttack)
{
	if (m_sMoveTime > UNIXTIME2)
		return;

	m_sMoveTime = UNIXTIME2 + 900;

	float Mesafe = 0.0f, EnYakinMesafe = 0.0f;
	__Vector3 vBot, vUser, vDistance, vRealDistance;

	Mesafe = pow(x - GetX(), 2.0f) + pow(z - GetZ(), 2.0f);
	if (Mesafe == EnYakinMesafe)
	{
		if (Delay > 0)
			Delay = myrand(1, Delay);

		//m_sMoveTime = UNIXTIME + Delay;
		if (WalkStep == 15)
			m_Reverse = true;

		if (WalkStep == 0)
			m_Reverse = false;

		if (m_Reverse)
			WalkStep--;
		else
			WalkStep++;

		float(x + myrand(1, 15) * 2.0f);
		float(z + myrand(1, 15) * 2.0f);

		if (GetZoneID() == ZONE_SPBATTLE1)
		{
			switch (s_MoveProcess)
			{
			case 1:
				if (m_MoveState++ > 18)
					return;
				break;
			case 2:
				if (m_MoveState++ > 12)
					return;
				break;
			case 3:
				if (m_MoveState++ > 8)
					return;
				break;
			case 4:
				if (m_MoveState++ > 11)
					return;
				break;
			case 5:
				if (m_MoveState++ > 19)
					return;
				break;
			case 6:
				if (m_MoveState++ > 9)
					return;
				break;
			case 7:
				if (m_MoveState++ > 17)
					return;
				break;
			case 8:
				if (m_MoveState++ > 11)
					return;
				break;
			case 9:
				if (m_MoveState++ > 20)
					return;
				break;
			case 10:
				if (m_MoveState++ > 7)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_SPBATTLE3)
		{
			switch (s_MoveProcess)
			{
			case 1:
				if (m_MoveState++ > 15)
					return;
				break;
			case 2:
				if (m_MoveState++ > 19)
					return;
				break;
			case 3:
				if (m_MoveState++ > 17)
					return;
				break;
			case 4:
				if (m_MoveState++ > 16)
					return;
				break;
			case 5:
				if (m_MoveState++ > 24)
					return;
				break;
			case 6:
				if (m_MoveState++ > 17)
					return;
				break;
			case 7:
				if (m_MoveState++ > 17)
					return;
				break;
			case 8:
				if (m_MoveState++ > 16)
					return;
				break;
			case 9:
				if (m_MoveState++ > 29)
					return;
				break;
			case 10:
				if (m_MoveState++ > 24)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_SPBATTLE4)
		{
			switch (s_MoveProcess)
			{
			case 1:
				if (m_MoveState++ > 10)
					return;
				break;
			case 2:
				if (m_MoveState++ > 14)
					return;
				break;
			case 3:
				if (m_MoveState++ > 11)
					return;
				break;
			case 4:
				if (m_MoveState++ > 21)
					return;
				break;
			case 5:
				if (m_MoveState++ > 22)
					return;
				break;
			case 6:
				if (m_MoveState++ > 27)
					return;
				break;
			case 7:
				if (m_MoveState++ > 29)
					return;
				break;
			case 8:
				if (m_MoveState++ > 30)
					return;
				break;
			case 9:
				if (m_MoveState++ > 36)
					return;
				break;
			case 10:
				if (m_MoveState++ > 49)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_SPBATTLE5)
		{
			switch (s_MoveProcess)
			{
			case 1:
				if (m_MoveState++ > 10)
					return;
				break;
			case 2:
				if (m_MoveState++ > 10)
					return;
				break;
			case 3:
				if (m_MoveState++ > 11)
					return;
				break;
			case 4:
				if (m_MoveState++ > 9)
					return;
				break;
			case 5:
				if (m_MoveState++ > 23)
					return;
				break;
			case 6:
				if (m_MoveState++ > 11)
					return;
				break;
			case 7:
				if (m_MoveState++ > 28)
					return;
				break;
			case 8:
				if (m_MoveState++ > 11)
					return;
				break;
			case 9:
				if (m_MoveState++ > 18)
					return;
				break;
			case 10:
				if (m_MoveState++ > 11)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_SPBATTLE6)
		{
			switch (s_MoveProcess)
			{
			case 1:
				if (m_MoveState++ > 6)
					return;
				break;
			case 2:
				if (m_MoveState++ > 6)
					return;
				break;
			case 3:
				if (m_MoveState++ > 7)
					return;
				break;
			case 4:
				if (m_MoveState++ > 13)
					return;
				break;
			case 5:
				if (m_MoveState++ > 13)
					return;
				break;
			case 6:
				if (m_MoveState++ > 13)
					return;
				break;
			case 7:
				if (m_MoveState++ > 14)
					return;
				break;
			case 8:
				if (m_MoveState++ > 13)
					return;
				break;
			case 9:
				if (m_MoveState++ > 14)
					return;
				break;
			case 10:
				if (m_MoveState++ > 11)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_SPBATTLE7)
		{
			switch (s_MoveProcess)
			{
			case 1: // 21 alt case
				if (m_MoveState++ > 20)
					return;
				break;
			case 2: // 22 alt case
				if (m_MoveState++ > 21)
					return;
				break;
			case 3: // 22 alt case
				if (m_MoveState++ > 21)
					return;
				break;
			case 4: // 21 alt case
				if (m_MoveState++ > 20)
					return;
				break;
			case 5: // 20 alt case
				if (m_MoveState++ > 19)
					return;
				break;
			case 6: // 19 alt case
				if (m_MoveState++ > 18)
					return;
				break;
			case 7: // 18 alt case
				if (m_MoveState++ > 17)
					return;
				break;
			case 8: // 17 alt case
				if (m_MoveState++ > 16)
					return;
				break;
			case 9: // 18 alt case
				if (m_MoveState++ > 17)
					return;
				break;
			case 10: // 21 alt case
				if (m_MoveState++ > 20)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_SPBATTLE9)
		{
			switch (s_MoveProcess)
			{
			case 1: // 8 alt case
				if (m_MoveState++ > 7)
					return;
				break;
			case 2: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 3: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 4: // 8 alt case
				if (m_MoveState++ > 7)
					return;
				break;
			case 5: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 6: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 7: // 8 alt case
				if (m_MoveState++ > 7)
					return;
				break;
			case 8: // 10 alt case
				if (m_MoveState++ > 9)
					return;
				break;
			case 9: // 10 alt case
				if (m_MoveState++ > 9)
					return;
				break;
			case 10: // 5 alt case
				if (m_MoveState++ > 4)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_SPBATTLE10)
		{
			switch (s_MoveProcess)
			{
			case 1: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 2: // 5 alt case
				if (m_MoveState++ > 4)
					return;
				break;
			case 3: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 4: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 5: // 5 alt case
				if (m_MoveState++ > 4)
					return;
				break;
			case 6: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 7: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 8: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 9: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 10: // 4 alt case
				if (m_MoveState++ > 3)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_SPBATTLE11)
		{
			switch (s_MoveProcess)
			{
			case 1: // 8 alt case
				if (m_MoveState++ > 7)
					return;
				break;
			case 2: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 3: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 4: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 5: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 6: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 7: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 8: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 9: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 10: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			}
		}
		
		if (GetZoneID() == ZONE_BIFROST)
		{
			switch (s_MoveProcess)
			{
			case 1: // 14 alt case
				if (m_MoveState++ > 13)
					return;
				break;
			case 2: // 12 alt case
				if (m_MoveState++ > 11)
					return;
				break;
			case 3: // 16 alt case
				if (m_MoveState++ > 15)
					return;
				break;
			case 4: // 9 alt case
				if (m_MoveState++ > 8)
					return;
				break;
			case 5: // 21 alt case
				if (m_MoveState++ > 20)
					return;
				break;
			case 6: // 14 alt case
				if (m_MoveState++ > 13)
					return;
				break;
			case 7: // 12 alt case
				if (m_MoveState++ > 11)
					return;
				break;
			case 8: // 16 alt case
				if (m_MoveState++ > 15)
					return;
				break;
			case 9: // 9 alt case
				if (m_MoveState++ > 8)
					return;
				break;
			case 10: // 21 alt case
				if (m_MoveState++ > 20)
					return;
				break;
			}
		}
		
		if (GetZoneID() == ZONE_DELOS)
		{
			switch (s_MoveProcess)
			{
			case 1: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 2: // 7 alt case
				if (m_MoveState++ > 6)
					return;
				break;
			case 3: // 5 alt case
				if (m_MoveState++ > 4)
					return;
				break;
			case 4: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 5: // 9 alt case
				if (m_MoveState++ > 8)
					return;
				break;
			case 6: // 8 alt case
				if (m_MoveState++ > 7)
					return;
				break;
			case 7: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 8: // 6 alt case
				if (m_MoveState++ > 5)
					return;
				break;
			case 9: // 9 alt case
				if (m_MoveState++ > 8)
					return;
				break;
			case 10: // 9 alt case
				if (m_MoveState++ > 8)
					return;
				break;
			}
		}

		if (GetZoneID() == ZONE_RONARK_LAND || GetZoneID() == ZONE_ARDREAM)
		{
			switch (s_MoveProcess)
			{
			case 1:
				if (m_MoveState++ > 19)
					return;
				break;
			case 2:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 34)
						return;
				}
				else
				{
					if (m_MoveState++ > 29)
						return;
				}
			}break;
			case 3:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 26)
						return;
				}
				else
				{
					if (m_MoveState++ > 31)
						return;
				}
			}break;
			case 4:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 19)
						return;
				}
				else
				{
					if (m_MoveState++ > 34)
						return;
				}
			}break;
			case 5:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 21)
						return;
				}
				else
				{
					if (m_MoveState++ > 39)
						return;
				}
			}break;
			case 6:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 17)
						return;
				}
				else
				{
					if (m_MoveState++ > 34)
						return;
				}
			}break;
			case 7:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 22)
						return;
				}
				else
				{
					if (m_MoveState++ > 25)
						return;
				}
			}break;
			case 8:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 16)
						return;
				}
				else
				{
					if (m_MoveState++ > 24)
						return;
				}
			}break;
			case 9:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 21)
						return;
				}
				else
				{
					if (m_MoveState++ > 24)
						return;
				}
			}break;
			case 10:
			{
				if (GetNation() == KARUS)
				{
					if (m_MoveState++ > 23)
						return;
				}
				else
				{
					if (m_MoveState++ > 30)
						return;
				}
			}break;
			}
		}
	}

	if (isAttack)
	{
		if (!isRegionTargetUp())
			m_sRegionAttack = true;
	}

	vBot.Set(GetX(), GetY(), GetZ());
	vUser.Set(x, y, z);

	vDistance = vUser - vBot;
	vRealDistance = vDistance;
	vDistance.Normalize();

	float speed = m_sSpeed;
	uint8 KosuSuresi = 1;
	bool KosuBitirme = false;
	vDistance *= speed / 10.0f;

	if (echo == uint8(0)
		&& vDistance.Magnitude() < vRealDistance.Magnitude()
		&& (vDistance * 100.0f).Magnitude() < vRealDistance.Magnitude())
	{
		vDistance *= 100.0f;
		KosuSuresi = 100;
	}
	else if (vDistance.Magnitude() > vRealDistance.Magnitude()
		|| vDistance.Magnitude() == vRealDistance.Magnitude())
	{
		KosuBitirme = true;
		vDistance = vRealDistance;
	}

	uint16 will_x, will_z, will_y;
	echo = uint8(3);
	will_x = uint16((vBot + vDistance).x * 10.0f);
	will_y = uint16(vUser.y * 10.0f);
	will_z = uint16((vBot + vDistance).z * 10.0f);
	MoveRegionProcess((vBot + vDistance).x, vUser.y, (vBot + vDistance).z, will_x, will_y, will_z, speed, echo);
}

void CBot::MoveProcessRonarkLandTown()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess)
	{
	case 1:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1375) : float(623);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1099) : float(902);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1276) : float(668);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1056) : float(917);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1212) : float(731);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(901) : float(938);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1088) : float(770);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(771) : float(1007);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(966) : float(820);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(876) : float(1078);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(745) : float(863);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(937) : float(1140);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(726) : float(910);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(921) : float(1166);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(734) : float(939);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1016) : float(1201);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(574) : float(1019);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1056) : float(1207);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(463) : float(1067);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(910) : float(1206);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(517) : float(1098);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(880) : float(1215);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(538) : float(1127);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(792) : float(1182);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(693) : float(1201);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(794) : float(1145);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(772) : float(1229);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(941) : float(1094);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(725) : float(1267);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(946) : float(1059);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(717) : float(1275);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(918) : float(1034);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(775) : float(1243);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(944) : float(1041);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(735) : float(1227);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(956) : float(1100);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(718) : float(1267);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(928) : float(978);
			break;
		}
	}break;
	case 2:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1377) : float(623);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1100) : float(902);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1350) : float(647);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1087) : float(920);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1311) : float(689);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1035) : float(960);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1205) : float(728);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(916) : float(989);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1164) : float(776);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(968) : float(1014);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(994) : float(801);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(973) : float(1080);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(949) : float(818);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1047) : float(1142);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(871) : float(876);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1049) : float(1196);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(843) : float(922);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1103) : float(1214);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(613) : float(952);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1055) : float(1244);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(432) : float(1024);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1118) : float(1251);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(374) : float(1010);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1042) : float(1207);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(392) : float(1054);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(957) : float(1200);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(322) : float(1138);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(896) : float(1233);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(143) : float(1155);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(887) : float(1174);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(282) : float(1204);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(695) : float(1145);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(364) : float(1226);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(604) : float(1097);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(279) : float(1247);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(518) : float(1057);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(193) : float(1286);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(590) : float(1066);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(129) : float(1246);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(674) : float(1055);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(182) : float(1267);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(595) : float(1044);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(303) : float(1309);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(513) : float(989);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(403) : float(1316);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(645) : float(942);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(506) : float(1381);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(735) : float(952);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(586) : float(1435);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(782) : float(968);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(709) : float(1466);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(811) : float(988);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(764) : float(1484);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(933) : float(1066);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(726) : float(1508);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(945) : float(1133);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(732) : float(1460);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(923) : float(1108);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(776) : float(0);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(951) : float(0);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(712) : float(0);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(937) : float(0);
			break;
		case 32:
			UnitX = GetNation() == KARUS ? float(746) : float(0);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(928) : float(0);
			break;
		case 33:
			UnitX = GetNation() == KARUS ? float(703) : float(0);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(949) : float(0);
			break;
		case 34:
			UnitX = GetNation() == KARUS ? float(776) : float(0);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(937) : float(0);
			break;
		}
	}break;
	case 3:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1380) : float(624);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1102) : float(901);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1269) : float(661);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1058) : float(906);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1165) : float(685);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1173) : float(899);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1034) : float(710);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1196) : float(917);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(952) : float(747);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1209) : float(923);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(799) : float(770);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1067) : float(952);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(738) : float(795);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(935) : float(1022);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(715) : float(836);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(915) : float(1091);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(740) : float(862);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(955) : float(1077);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(776) : float(876);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(944) : float(1052);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(726) : float(897);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(946) : float(1079);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(768) : float(927);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(934) : float(1063);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(717) : float(963);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(919) : float(1081);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(749) : float(992);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(966) : float(1112);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(708) : float(1031);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1022) : float(1144);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(573) : float(1075);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1065) : float(1151);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(469) : float(1117);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(909) : float(1160);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(541) : float(1137);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(807) : float(1158);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(688) : float(1179);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(795) : float(1150);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(773) : float(1209);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(930) : float(1125);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(731) : float(1223);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(941) : float(1088);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(723) : float(1244);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(923) : float(1059);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(742) : float(1251);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(947) : float(984);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(724) : float(1257);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(938) : float(942);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(722) : float(1301);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(921) : float(936);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(750) : float(1344);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(934) : float(931);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(0) : float(1308);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(972);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(0) : float(1270);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1052);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(0) : float(1282);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1070);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(0) : float(1217);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1100);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(0) : float(1266);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1062);
			break;
		}
	}break;
	case 4:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1380) : float(626);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1103) : float(901);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1460) : float(638);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1081) : float(912);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1589) : float(653);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1067) : float(919);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1626) : float(675);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1011) : float(949);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1794) : float(710);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1185) : float(930);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1521) : float(762);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1573) : float(919);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1368) : float(795);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1483) : float(912);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1136) : float(830);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1265) : float(843);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(1060) : float(869);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1201) : float(819);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(869) : float(892);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1210) : float(806);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(793) : float(929);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1069) : float(806);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(764) : float(989);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(937) : float(795);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(737) : float(1060);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(923) : float(798);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(714) : float(1142);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(934) : float(815);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(758) : float(1186);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(960) : float(843);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(742) : float(1194);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(923) : float(888);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(726) : float(1223);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(955) : float(906);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(802) : float(1227);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(903) : float(933);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(732) : float(1250);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(935) : float(982);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(0) : float(1274);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1007);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(0) : float(1266);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1060);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(0) : float(1204);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1129);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(0) : float(1178);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1170);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(0) : float(1119);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1185);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(0) : float(1102);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1216);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(0) : float(1154);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1240);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(0) : float(1176);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1301);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(0) : float(1221);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1349);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(0) : float(1264);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1307);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(0) : float(1347);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1449);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(0) : float(1381);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1488);
			break;
		case 32:
			UnitX = GetNation() == KARUS ? float(0) : float(1428);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1530);
			break;
		case 33:
			UnitX = GetNation() == KARUS ? float(0) : float(1491);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1532);
			break;
		case 34:
			UnitX = GetNation() == KARUS ? float(0) : float(1425);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1572);
			break;

		}
	}break;
	case 5:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1375) : float(627);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1102) : float(903);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1257) : float(636);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1045) : float(913);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1218) : float(623);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(897) : float(893);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1166) : float(628);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(965) : float(896);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1030) : float(634);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(972) : float(925);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1020) : float(647);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1023) : float(925);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(866) : float(672);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1062) : float(921);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(838) : float(713);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1098) : float(927);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(721) : float(728);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(925) : float(910);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(773) : float(777);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(931) : float(915);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(723) : float(802);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(953) : float(892);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(918) : float(833);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(767) : float(844);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(1017) : float(861);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(805) : float(831);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(1056) : float(886);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(923) : float(803);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(1001) : float(906);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(972) : float(776);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(944) : float(952);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1051) : float(764);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(872) : float(996);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1056) : float(769);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(836) : float(1036);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1096) : float(750);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(726) : float(1088);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(943) : float(744);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(771) : float(1118);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(938) : float(751);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(716) : float(1125);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(928) : float(763);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(0) : float(1032);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(793);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(0) : float(1177);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(840);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(0) : float(1233);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(865);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(0) : float(1301);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(918);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(0) : float(1270);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(970);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(0) : float(1257);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1011);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(0) : float(1272);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1059);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(0) : float(1282);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1076);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(0) : float(1269);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1119);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(0) : float(1281);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1165);
			break;
		case 32:
			UnitX = GetNation() == KARUS ? float(0) : float(1316);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1180);
			break;
		case 33:
			UnitX = GetNation() == KARUS ? float(0) : float(1353);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1198);
			break;
		case 34:
			UnitX = GetNation() == KARUS ? float(0) : float(1417);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1196);
			break;
		case 35:
			UnitX = GetNation() == KARUS ? float(0) : float(1469);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1187);
			break;
		case 36:
			UnitX = GetNation() == KARUS ? float(0) : float(1499);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1143);
			break;
		case 37:
			UnitX = GetNation() == KARUS ? float(0) : float(1505);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1085);
			break;
		case 38:
			UnitX = GetNation() == KARUS ? float(0) : float(1460);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1105);
			break;
		case 39:
			UnitX = GetNation() == KARUS ? float(0) : float(1380);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1091);
			break;

		}
	}break;
	case 6:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1375) : float(625);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1102) : float(895);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1389) : float(605);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1069) : float(896);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1245) : float(605);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1081) : float(925);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1100) : float(586);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1209) : float(898);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1024) : float(511);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1189) : float(880);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1043) : float(485);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1032) : float(902);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1050) : float(485);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(971) : float(933);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(979) : float(533);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(961) : float(1010);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(935) : float(558);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1045) : float(1046);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(868) : float(630);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1066) : float(1052);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(838) : float(703);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1054) : float(1073);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(733) : float(769);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(953) : float(1083);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(745) : float(819);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(920) : float(1086);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(730) : float(842);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(939) : float(1104);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(706) : float(874);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(938) : float(1054);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(762) : float(932);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(958) : float(1050);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(736) : float(972);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(969) : float(1037);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(0) : float(972);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1037);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(0) : float(1059);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(986);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(0) : float(1099);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(959);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(0) : float(1146);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(965);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(0) : float(1175);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(971);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(0) : float(1190);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(927);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(0) : float(1215);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(915);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(0) : float(1243);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(959);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(0) : float(1248);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1003);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(0) : float(1290);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1065);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(0) : float(1257);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1060);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(0) : float(1264);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1055);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(0) : float(1286);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1081);
			break;
		case 32:
			UnitX = GetNation() == KARUS ? float(0) : float(1254);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1045);
			break;
		case 33:
			UnitX = GetNation() == KARUS ? float(0) : float(1300);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1054);
			break;
		case 34:
			UnitX = GetNation() == KARUS ? float(0) : float(1267);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1060);
			break;

		}
	}break;
	case 7:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1376) : float(626);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1102) : float(890);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1272) : float(634);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1048) : float(923);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1206) : float(635);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(896) : float(911);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1124) : float(693);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(777) : float(927);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(982) : float(741);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(794) : float(911);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(832) : float(791);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(728) : float(910);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(794) : float(844);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(923) : float(840);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(708) : float(890);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1023) : float(795);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(578) : float(911);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1065) : float(782);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(526) : float(960);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1006) : float(783);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(439) : float(978);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1013) : float(798);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(334) : float(1015);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(900) : float(809);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(175) : float(1037);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(895) : float(780);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(288) : float(1180);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(689) : float(838);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(464) : float(1230);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(514) : float(867);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(611) : float(1253);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(400) : float(937);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(706) : float(1251);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(541) : float(990);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(843) : float(1269);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(667) : float(1024);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(774) : float(1269);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(943) : float(1060);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(729) : float(1253);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(954) : float(1067);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(770) : float(1250);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(996) : float(1055);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(729) : float(1275);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(936) : float(1047);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(0) : float(1254);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1035);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(0) : float(1291);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1073);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(0) : float(1242);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1055);
			break;
		}
	}break;

	case 8:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1375) : float(622);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1091) : float(901);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1487) : float(626);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1111) : float(902);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1463) : float(641);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(970) : float(916);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1267) : float(708);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(901) : float(923);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1129) : float(751);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(803) : float(997);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(836) : float(800);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(773) : float(1044);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(728) : float(827);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(839) : float(1080);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(754) : float(842);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(930) : float(1092);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(728) : float(860);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(942) : float(1073);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(768) : float(887);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(930) : float(1040);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(735) : float(936);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(951) : float(1042);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(765) : float(971);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(977) : float(1043);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(768) : float(993);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(936) : float(1014);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(730) : float(1023);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(947) : float(986);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(783) : float(1055);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(946) : float(972);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(731) : float(1130);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(950) : float(958);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(0) : float(1172);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(974);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(0) : float(1186);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(916);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(0) : float(1128);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(907);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(0) : float(1253);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(925);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(0) : float(1247);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1005);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(0) : float(1268);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1025);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(0) : float(1260);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1061);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(0) : float(1244);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1053);
			break;
		}
	}break;

	case 9:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1376) : float(625);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1103) : float(902);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1255) : float(654);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1040) : float(915);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1241) : float(692);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(897) : float(886);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1190) : float(709);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(930) : float(877);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1177) : float(750);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(966) : float(863);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1070) : float(768);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(968) : float(855);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(980) : float(809);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1045) : float(845);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(885) : float(875);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1035) : float(818);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(847) : float(893);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1095) : float(810);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(726) : float(931);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(939) : float(803);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(805) : float(979);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(914) : float(795);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(761) : float(1012);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(949) : float(813);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(736) : float(1025);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(920) : float(850);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(743) : float(1052);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(980) : float(887);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(776) : float(1037);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(951) : float(914);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(753) : float(1060);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(946) : float(928);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(720) : float(1102);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(971) : float(951);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(769) : float(1138);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(974) : float(962);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(758) : float(1173);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(941) : float(965);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(712) : float(1189);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(800) : float(940);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(769) : float(1199);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(939) : float(922);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(0) : float(1219);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(917);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(0) : float(1256);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(961);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(0) : float(1271);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1059);
			break;
		}
	}break;

	case 10:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(1378) : float(626);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1102) : float(903);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1340) : float(597);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1159) : float(915);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1137) : float(563);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1220) : float(890);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(940) : float(480);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1197) : float(877);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(800) : float(473);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1063) : float(921);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(508) : float(524);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1084) : float(994);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(377) : float(567);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1066) : float(1047);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(389) : float(691);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(956) : float(1070);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(337) : float(777);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(900) : float(1085);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(180) : float(787);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(888) : float(1121);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(287) : float(825);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(694) : float(1148);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(359) : float(894);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(610) : float(1198);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(468) : float(949);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(693) : float(1204);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(547) : float(994);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(794) : float(1201);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(694) : float(1026);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(789) : float(1165);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(777) : float(1047);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(940) : float(1118);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(738) : float(1062);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(925) : float(1096);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(770) : float(1106);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(956) : float(1113);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(740) : float(1144);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(962) : float(1139);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(751) : float(1161);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(935) : float(1151);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(651) : float(1183);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(1061) : float(1130);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(772) : float(1208);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(969) : float(1121);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(765) : float(1248);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(940) : float(1076);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(0) : float(1241);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1060);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(0) : float(1255);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1045);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(0) : float(1250);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1065);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(0) : float(1264);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1051);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(0) : float(1266);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1060);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(0) : float(1257);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1057);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(0) : float(1272);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(0) : float(1062);
			break;
		}
	}break;
	}

	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessArdreamLandTown()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess)
	{
	case 1:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(856) : float(195);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(138) : float(901);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(809) : float(186);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(156) : float(875);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(739) : float(214);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(297) : float(762);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(828) : float(230);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(533) : float(692);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(908) : float(216);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(638) : float(581);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(864) : float(292);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(777) : float(544);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(772) : float(469);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(838) : float(534);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(712) : float(524);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(889) : float(545);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(548) : float(614);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(895) : float(507);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(467) : float(801);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(928) : float(447);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(327) : float(775);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(883) : float(197);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(234) : float(686);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(824) : float(238);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(194) : float(767);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(816) : float(247);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(205) : float(816);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(796) : float(176);
			break;
		}
	}break;
	case 2:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(853) : float(188);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(141) : float(899);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(768) : float(203);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(223) : float(732);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(668) : float(235);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(172) : float(626);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(490) : float(132);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(148) : float(447);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(368) : float(215);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(100) : float(335);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(325) : float(218);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(144) : float(195);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(212) : float(336);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(190) : float(132);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(206) : float(359);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(270) : float(114);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(219) : float(519);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(337) : float(154);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(100) : float(637);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(474) : float(211);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(210) : float(760);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(595) : float(249);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(227) : float(768);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(779) : float(203);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(189) : float(799);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(803) : float(220);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(212) : float(750);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(816) : float(205);
			break;
		}
	}break;
	case 3:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(856) : float(191);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(140) : float(900);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(800) : float(126);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(136) : float(876);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(757) : float(135);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(207) : float(766);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(753) : float(117);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(463) : float(694);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(626) : float(267);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(539) : float(540);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(550) : float(396);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(621) : float(491);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(394) : float(534);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(560) : float(378);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(249) : float(715);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(542) : float(497);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(221) : float(761);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(744) : float(379);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(195) : float(769);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(807) : float(206);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(250) : float(788);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(817) : float(220);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(208) : float(716);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(784) : float(197);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(192) : float(791);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(849) : float(206);
			break;
		}
	}break;
	case 4:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(852) : float(192);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(141) : float(899);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(831) : float(186);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(181) : float(821);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(770) : float(272);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(226) : float(842);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(810) : float(396);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(453) : float(915);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(645) : float(557);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(524) : float(915);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(537) : float(720);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(436) : float(883);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(478) : float(789);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(423) : float(822);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(314) : float(864);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(542) : float(788);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(221) : float(875);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(577) : float(596);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(216) : float(812);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(697) : float(502);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(193) : float(772);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(804) : float(212);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(239) : float(772);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(823) : float(241);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(198) : float(789);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(819) : float(215);
			break;
		}
	}break;
	case 5:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(855) : float(195);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(140) : float(901);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(806) : float(210);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(157) : float(888);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(762) : float(223);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(209) : float(813);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(593) : float(333);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(161) : float(885);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(513) : float(460);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(155) : float(919);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(371) : float(723);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(106) : float(880);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(305) : float(823);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(153) : float(801);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(128) : float(871);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(253) : float(651);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(160) : float(756);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(391) : float(453);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(134) : float(767);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(585) : float(219);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(238) : float(806);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(716) : float(241);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(225) : float(669);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(808) : float(168);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(187) : float(722);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(816) : float(246);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(223) : float(778);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(833) : float(226);
			break;
		}
	}break;
	case 6:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(853) : float(189);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(139) : float(902);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(807) : float(202);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(158) : float(735);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(798) : float(241);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(269) : float(624);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(824) : float(376);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(400) : float(586);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(819) : float(464);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(554) : float(589);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(874) : float(546);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(659) : float(620);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(852) : float(702);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(784) : float(491);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(768) : float(779);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(842) : float(419);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(727) : float(747);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(877) : float(306);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(467) : float(786);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(931) : float(200);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(323) : float(743);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(879) : float(214);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(200) : float(794);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(795) : float(225);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(209) : float(779);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(819) : float(188);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(188) : float(791);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(818) : float(197);
			break;
		}
	}break;
	case 7:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(856) : float(189);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(137) : float(907);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(834) : float(189);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(144) : float(839);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(770) : float(220);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(232) : float(741);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(780) : float(217);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(344) : float(639);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(730) : float(100);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(484) : float(478);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(576) : float(135);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(520) : float(331);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(539) : float(190);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(538) : float(197);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(463) : float(325);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(534) : float(145);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(346) : float(360);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(604) : float(110);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(243) : float(525);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(609) : float(156);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(208) : float(606);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(740) : float(154);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(194) : float(776);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(821) : float(227);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(235) : float(755);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(823) : float(189);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(195) : float(802);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(792) : float(210);
			break;
		}
	}break;
	case 8:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(856) : float(176);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(137) : float(938);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(812) : float(182);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(121) : float(910);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(768) : float(192);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(184) : float(853);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(673) : float(249);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(178) : float(824);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(554) : float(407);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(177) : float(925);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(367) : float(598);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(97) : float(908);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(316) : float(719);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(157) : float(877);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(215) : float(780);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(275) : float(832);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(238) : float(863);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(383) : float(790);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(199) : float(862);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(476) : float(579);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(209) : float(795);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(528) : float(472);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(205) : float(779);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(691) : float(265);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(231) : float(769);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(774) : float(188);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(188) : float(794);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(815) : float(213);
			break;
		}
	}break;
	case 9:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(853) : float(194);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(140) : float(899);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(808) : float(185);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(157) : float(842);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(750) : float(230);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(329) : float(710);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(828) : float(198);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(544) : float(561);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(896) : float(133);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(611) : float(445);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(889) : float(211);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(782) : float(341);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(783) : float(209);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(825) : float(197);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(720) : float(323);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(884) : float(143);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(452) : float(363);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(927) : float(105);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(337) : float(519);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(885) : float(154);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(194) : float(627);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(787) : float(215);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(208) : float(765);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(813) : float(242);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(240) : float(799);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(770) : float(209);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(197) : float(770);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(826) : float(194);
			break;
		}
	}break;
	case 10:
	{
		switch (m_MoveState)
		{
		case 1:
			UnitX = GetNation() == KARUS ? float(858) : float(166);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(117) : float(899);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(784) : float(169);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(122) : float(856);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(757) : float(208);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(253) : float(782);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(755) : float(225);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(457) : float(698);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(668) : float(239);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(504) : float(622);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(548) : float(367);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(365) : float(602);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(401) : float(455);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(478) : float(465);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(328) : float(529);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(541) : float(457);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(250) : float(694);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(562) : float(500);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(212) : float(752);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(633) : float(426);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(227) : float(773);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(777) : float(196);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(190) : float(769);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(823) : float(197);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(289) : float(797);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(841) : float(209);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(208) : float(783);
			UnitY = float(GetY());
			UnitZ = GetNation() == KARUS ? float(798) : float(182);
			break;
		}
	}break;
	}

	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialEventZindan()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(175) : float(441);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(376) : float(129);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(130) : float(400);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(372) : float(112);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(82) : float(376);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(370) : float(102);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(99) : float(356);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(255) : float(101);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(101) : float(313);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(248) : float(108);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(102) : float(265);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(231) : float(114);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(111) : float(230);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(116) : float(122);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(110) : float(202);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(111) : float(119);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(145) : float(158);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(62) : float(113);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(158) : float(145);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(113) : float(62);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(202) : float(110);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(119) : float(111);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(230) : float(111);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(122) : float(116);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(265) : float(102);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(114) : float(231);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(313) : float(101);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(108) : float(248);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(356) : float(99);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(101) : float(255);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(376) : float(82);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(102) : float(370);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(400) : float(130);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(112) : float(372);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(441) : float(175);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(129) : float(376);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(232) : float(443);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(379) : float(128);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(241) : float(373);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(297) : float(103);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(251) : float(314);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(245) : float(101);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(193) : float(273);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(239) : float(120);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(98) : float(197);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(230) : float(120);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(116) : float(122);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(127) : float(105);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(122) : float(116);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(105) : float(127);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(197) : float(98);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(120) : float(230);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(273) : float(193);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(120) : float(239);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(314) : float(251);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(101) : float(245);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(373) : float(241);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(103) : float(297);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(443) : float(232);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(128) : float(379);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(236) : float(436);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(382) : float(182);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(239) : float(445);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(266) : float(206);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(239) : float(417);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(257) : float(262);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(285) : float(321);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(254) : float(243);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(321) : float(285);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(243) : float(254);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(417) : float(239);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(262) : float(257);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(445) : float(239);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(206) : float(266);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(436) : float(236);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(182) : float(382);
			break;

		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(236) : float(441);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(381) : float(128);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(241) : float(388);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(300) : float(106);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(243) : float(351);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(250) : float(102);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(294) : float(309);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(242) : float(110);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(275) : float(280);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(193) : float(111);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(277) : float(277);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(139) : float(139);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(280) : float(275);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(111) : float(193);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(309) : float(294);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(110) : float(242);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(351) : float(243);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(102) : float(250);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(388) : float(241);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(106) : float(300);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(441) : float(236);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(128) : float(381);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(234) : float(435);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(379) : float(206);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(241) : float(432);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(229) : float(267);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(192) : float(421);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(243) : float(265);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(136) : float(434);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(234) : float(261);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(49) : float(330);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(231) : float(244);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(106) : float(273);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(179) : float(243);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(117) : float(276);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(135) : float(241);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(134) : float(280);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(70) : float(119);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(187) : float(248);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(121) : float(118);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(209) : float(209);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(124) : float(124);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(248) : float(187);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(118) : float(121);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(280) : float(134);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(119) : float(70);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(276) : float(117);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(241) : float(135);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(273) : float(106);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(243) : float(179);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(330) : float(49);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(244) : float(231);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(434) : float(136);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(261) : float(234);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(421) : float(192);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(265) : float(243);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(432) : float(241);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(267) : float(229);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(435) : float(234);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(206) : float(379);
			break;
		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(234) : float(442);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(381) : float(132);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(238) : float(364);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(293) : float(98);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(240) : float(334);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(245) : float(107);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(276) : float(272);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(226) : float(94);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(275) : float(275);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(147) : float(147);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(272) : float(276);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(94) : float(226);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(334) : float(240);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(107) : float(245);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(364) : float(238);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(98) : float(293);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(442) : float(234);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(132) : float(381);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(226) : float(439);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(385) : float(129);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(150) : float(387);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(377) : float(105);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(62) : float(343);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(370) : float(102);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(108) : float(290);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(341) : float(102);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(100) : float(281);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(305) : float(126);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(95) : float(221);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(260) : float(120);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(90) : float(176);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(229) : float(120);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(109) : float(110);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(116) : float(101);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(113) : float(113);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(114) : float(114);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(110) : float(109);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(101) : float(116);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(176) : float(90);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(120) : float(229);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(221) : float(95);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(120) : float(260);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(281) : float(100);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(126) : float(305);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(290) : float(108);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(102) : float(341);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(343) : float(62);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(102) : float(370);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(387) : float(150);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(105) : float(377);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(439) : float(226);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(129) : float(385);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(229) : float(451);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(379) : float(133);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(243) : float(396);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(294) : float(112);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(244) : float(368);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(249) : float(100);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(277) : float(332);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(236) : float(103);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(274) : float(275);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(194) : float(93);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(275) : float(275);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(137) : float(137);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(275) : float(274);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(93) : float(194);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(332) : float(277);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(103) : float(236);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(368) : float(244);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(100) : float(249);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(396) : float(243);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(112) : float(294);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(451) : float(229);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(133) : float(379);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(235) : float(444);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(384) : float(131);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(244) : float(398);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(362) : float(112);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(235) : float(381);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(344) : float(95);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(236) : float(328);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(313) : float(99);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(242) : float(308);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(304) : float(121);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(246) : float(272);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(261) : float(119);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(172) : float(246);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(243) : float(117);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(117) : float(194);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(232) : float(124);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(93) : float(162);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(178) : float(110);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(101) : float(116);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(185) : float(117);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(116) : float(101);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(117) : float(185);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(162) : float(93);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(110) : float(178);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(194) : float(117);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(124) : float(232);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(246) : float(172);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(117) : float(243);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(272) : float(246);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(119) : float(261);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(308) : float(242);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(121) : float(304);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(328) : float(236);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(99) : float(313);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(381) : float(235);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(95) : float(344);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(398) : float(244);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(112) : float(362);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(444) : float(235);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(131) : float(384);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(234) : float(437);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(379) : float(175);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(241) : float(436);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(276) : float(226);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(305) : float(451);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(248) : float(287);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(356) : float(356);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(251) : float(251);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(451) : float(305);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(287) : float(248);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(436) : float(241);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(226) : float(276);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(437) : float(234);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(175) : float(379);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialMiniRonarkLand()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1299) : float(708);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1069) : float(943);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1246) : float(748);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1056) : float(950);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1198) : float(810);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(1034) : float(946);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1156) : float(845);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1044) : float(941);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1116) : float(885);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1029) : float(936);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1079) : float(920);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(1019) : float(948);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1026) : float(978);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1007) : float(978);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(959) : float(959);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(945) : float(945);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(978) : float(1026);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(978) : float(1007);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(920) : float(1079);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(948) : float(1019);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(885) : float(1116);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(936) : float(1029);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(845) : float(1156);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(941) : float(1044);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(810) : float(1198);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(946) : float(1034);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(748) : float(1246);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(950) : float(1056);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(708) : float(1299);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(943) : float(1069);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1331) : float(713);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1081) : float(921);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1296) : float(743);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1082) : float(942);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1269) : float(786);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1075) : float(945);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1161) : float(837);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1053) : float(947);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1192) : float(867);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(1061) : float(940);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1142) : float(897);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1043) : float(945);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1109) : float(927);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1037) : float(963);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1067) : float(952);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(1034) : float(981);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(960) : float(950);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1030) : float(955);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(998) : float(998);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1014) : float(1014);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(950) : float(960);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(955) : float(1030);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(952) : float(1067);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(981) : float(1034);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(927) : float(1109);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(963) : float(1037);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(897) : float(1142);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(945) : float(1043);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(867) : float(1192);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(940) : float(1061);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(837) : float(1161);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(947) : float(1053);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(786) : float(1269);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(945) : float(1075);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(743) : float(1296);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(942) : float(1082);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(713) : float(1331);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(921) : float(1081);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1333) : float(703);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1065) : float(930);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1308) : float(762);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(1041) : float(942);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1284) : float(812);
			UnitY = float(4);
			UnitZ = GetNation() == KARUS ? float(1041) : float(956);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1235) : float(844);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1033) : float(947);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1197) : float(881);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1031) : float(948);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1100) : float(906);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(1023) : float(942);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1133) : float(972);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(1026) : float(971);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1073) : float(1023);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1021) : float(1002);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(1070) : float(1070);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1021) : float(1021);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(1023) : float(1073);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1002) : float(1021);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(972) : float(1133);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(971) : float(1026);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(906) : float(1100);
			UnitY = float(4);
			UnitZ = GetNation() == KARUS ? float(942) : float(1023);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(881) : float(1197);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(948) : float(1031);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(844) : float(1235);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(947) : float(1033);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(812) : float(1284);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(956) : float(1041);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(762) : float(1308);
			UnitY = float(22);
			UnitZ = GetNation() == KARUS ? float(942) : float(1041);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(703) : float(1333);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(930) : float(1065);
			break;

		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1269) : float(717);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1060) : float(920);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1265) : float(780);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1007) : float(914);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1236) : float(806);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(909) : float(904);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1198) : float(822);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(916) : float(924);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1186) : float(878);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(942) : float(932);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1172) : float(911);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(971) : float(933);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1156) : float(968);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(986) : float(934);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1129) : float(1024);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(959) : float(937);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(1071) : float(1071);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(971) : float(971);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(1024) : float(1129);
			UnitY = float(18);
			UnitZ = GetNation() == KARUS ? float(937) : float(959);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(968) : float(1156);
			UnitY = float(21);
			UnitZ = GetNation() == KARUS ? float(934) : float(986);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(911) : float(1172);
			UnitY = float(19);
			UnitZ = GetNation() == KARUS ? float(933) : float(971);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(878) : float(1186);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(932) : float(942);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(822) : float(1198);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(924) : float(916);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(806) : float(1236);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(904) : float(909);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(780) : float(1265);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(914) : float(1007);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(717) : float(1269);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(920) : float(1060);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1301) : float(706);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1070) : float(933);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1252) : float(736);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1066) : float(961);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1203) : float(768);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1063) : float(988);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1130) : float(793);
			UnitY = float(6552);
			UnitZ = GetNation() == KARUS ? float(1054) : float(1046);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1091) : float(819);
			UnitY = float(6552);
			UnitZ = GetNation() == KARUS ? float(1048) : float(1080);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1028) : float(841);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1040) : float(1096);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(923) : float(861);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(1047) : float(1067);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(925) : float(878);
			UnitY = float(19);
			UnitZ = GetNation() == KARUS ? float(1050) : float(1048);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(878) : float(925);
			UnitY = float(14);
			UnitZ = GetNation() == KARUS ? float(1048) : float(1050);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(861) : float(923);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1067) : float(1047);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(841) : float(1028);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1096) : float(1040);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(819) : float(1091);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1080) : float(1048);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(793) : float(1130);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1046) : float(1054);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(768) : float(1203);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(988) : float(1063);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(736) : float(1252);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(961) : float(1066);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(706) : float(1301);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(933) : float(1070);
			break;

		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1300) : float(723);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(1070) : float(918);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1249) : float(764);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(1065) : float(931);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1204) : float(821);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(1063) : float(953);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1156) : float(852);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1054) : float(945);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1121) : float(889);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1049) : float(945);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1051) : float(948);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(956) : float(945);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1068) : float(976);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(983) : float(935);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1034) : float(1005);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(939) : float(933);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(1005) : float(1034);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(933) : float(939);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(976) : float(1068);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(935) : float(983);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(948) : float(1051);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(945) : float(956);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(889) : float(1121);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(945) : float(1049);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(852) : float(1156);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(945) : float(1054);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(821) : float(1204);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(953) : float(1063);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(764) : float(1249);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(931) : float(1065);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(723) : float(1300);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(918) : float(1070);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1287) : float(712);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(1065) : float(937);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1239) : float(739);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(1063) : float(938);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1199) : float(780);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1069) : float(943);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1101) : float(833);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1032) : float(943);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1137) : float(870);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(1044) : float(943);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1090) : float(895);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1050) : float(948);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1056) : float(928);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1060) : float(967);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1022) : float(955);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(1057) : float(981);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(962) : float(978);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(999) : float(1011);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(978) : float(962);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1011) : float(999);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(955) : float(1022);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(981) : float(1057);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(928) : float(1056);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(967) : float(1060);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(895) : float(1090);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(948) : float(1050);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(870) : float(1137);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(943) : float(1044);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(833) : float(1101);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(943) : float(1032);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(780) : float(1199);
			UnitY = float(4);
			UnitZ = GetNation() == KARUS ? float(943) : float(1069);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(739) : float(1239);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(938) : float(1063);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(712) : float(1287);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(937) : float(1065);
			break;

		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1296) : float(706);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1066) : float(932);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1268) : float(751);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1056) : float(930);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1264) : float(779);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1006) : float(941);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1254) : float(768);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(964) : float(967);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1232) : float(768);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(938) : float(986);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1209) : float(781);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(913) : float(1011);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1190) : float(793);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(929) : float(1040);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1185) : float(823);
			UnitY = float(6552);
			UnitZ = GetNation() == KARUS ? float(956) : float(1058);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(1135) : float(849);
			UnitY = float(4);
			UnitZ = GetNation() == KARUS ? float(1017) : float(1073);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(1138) : float(850);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(961) : float(1081);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(1102) : float(867);
			UnitY = float(13);
			UnitZ = GetNation() == KARUS ? float(962) : float(1061);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(1073) : float(879);
			UnitY = float(19);
			UnitZ = GetNation() == KARUS ? float(972) : float(1050);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(998) : float(913);
			UnitY = float(16);
			UnitZ = GetNation() == KARUS ? float(1036) : float(1048);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(1007) : float(951);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(1026) : float(1046);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(983) : float(983);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1037) : float(1037);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(951) : float(1007);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1046) : float(1026);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(913) : float(998);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1048) : float(1036);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(879) : float(1073);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1050) : float(972);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(867) : float(1102);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(1061) : float(962);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(850) : float(1138);
			UnitY = float(19);
			UnitZ = GetNation() == KARUS ? float(1081) : float(961);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(849) : float(1135);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1073) : float(1017);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(823) : float(1185);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(1058) : float(956);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(793) : float(1190);
			UnitY = float(4);
			UnitZ = GetNation() == KARUS ? float(1040) : float(929);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(781) : float(1209);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(1011) : float(913);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(768) : float(1232);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(986) : float(938);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(768) : float(1254);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(967) : float(964);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(779) : float(1264);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(941) : float(1006);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(751) : float(1268);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(930) : float(1056);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(706) : float(1296);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(932) : float(1066);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1296) : float(706);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1066) : float(932);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1268) : float(751);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1056) : float(930);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1264) : float(779);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1006) : float(941);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1254) : float(768);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(964) : float(967);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1232) : float(768);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(938) : float(986);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1209) : float(781);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(913) : float(1011);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1190) : float(793);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(929) : float(1040);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1185) : float(823);
			UnitY = float(6552);
			UnitZ = GetNation() == KARUS ? float(956) : float(1058);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(1135) : float(849);
			UnitY = float(4);
			UnitZ = GetNation() == KARUS ? float(1017) : float(1073);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(1138) : float(850);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(961) : float(1081);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(1102) : float(867);
			UnitY = float(13);
			UnitZ = GetNation() == KARUS ? float(962) : float(1061);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(1073) : float(879);
			UnitY = float(19);
			UnitZ = GetNation() == KARUS ? float(972) : float(1050);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(998) : float(913);
			UnitY = float(16);
			UnitZ = GetNation() == KARUS ? float(1036) : float(1048);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(1007) : float(951);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(1026) : float(1046);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(983) : float(983);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1037) : float(1037);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(951) : float(1007);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1046) : float(1026);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(913) : float(998);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1048) : float(1036);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(879) : float(1073);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1050) : float(972);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(867) : float(1102);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(1061) : float(962);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(850) : float(1138);
			UnitY = float(19);
			UnitZ = GetNation() == KARUS ? float(1081) : float(961);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(849) : float(1135);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1073) : float(1017);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(823) : float(1185);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(1058) : float(956);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(793) : float(1190);
			UnitY = float(4);
			UnitZ = GetNation() == KARUS ? float(1040) : float(929);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(781) : float(1209);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(1011) : float(913);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(768) : float(1232);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(986) : float(938);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(768) : float(1254);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(967) : float(964);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(779) : float(1264);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(941) : float(1006);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(751) : float(1268);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(930) : float(1056);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(706) : float(1296);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(932) : float(1066);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1289) : float(709);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1067) : float(932);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1200) : float(756);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(1068) : float(941);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1194) : float(769);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1061) : float(965);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1176) : float(779);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(1059) : float(1005);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1145) : float(784);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(1051) : float(1040);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1108) : float(810);
			UnitY = float(6551);
			UnitZ = GetNation() == KARUS ? float(1042) : float(1071);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1068) : float(828);
			UnitY = float(6553);
			UnitZ = GetNation() == KARUS ? float(1039) : float(1095);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1026) : float(855);
			UnitY = float(5);
			UnitZ = GetNation() == KARUS ? float(1037) : float(1077);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(1012) : float(879);
			UnitY = float(20);
			UnitZ = GetNation() == KARUS ? float(1063) : float(1043);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(995) : float(887);
			UnitY = float(19);
			UnitZ = GetNation() == KARUS ? float(1100) : float(1048);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(987) : float(913);
			UnitY = float(22);
			UnitZ = GetNation() == KARUS ? float(1118) : float(1087);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(996) : float(926);
			UnitY = float(22);
			UnitZ = GetNation() == KARUS ? float(1137) : float(1113);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(971) : float(944);
			UnitY = float(20);
			UnitZ = GetNation() == KARUS ? float(1146) : float(1135);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(944) : float(971);
			UnitY = float(15);
			UnitZ = GetNation() == KARUS ? float(1135) : float(1146);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(926) : float(996);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1113) : float(1137);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(913) : float(987);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1087) : float(1118);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(887) : float(995);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1048) : float(1100);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(879) : float(1012);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1043) : float(1063);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(855) : float(1026);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1077) : float(1037);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(828) : float(1068);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1095) : float(1039);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(810) : float(1108);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(1071) : float(1042);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(784) : float(1145);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1040) : float(1051);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(779) : float(1176);
			UnitY = float(5);
			UnitZ = GetNation() == KARUS ? float(1005) : float(1059);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(769) : float(1194);
			UnitY = float(3);
			UnitZ = GetNation() == KARUS ? float(965) : float(1061);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(756) : float(1200);
			UnitY = float(1);
			UnitZ = GetNation() == KARUS ? float(941) : float(1068);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(709) : float(1289);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(932) : float(1067);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialBaseWar()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(510) : float(508);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(161) : float(720);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(513) : float(505);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(215) : float(703);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(511) : float(504);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(273) : float(662);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(507) : float(469);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(341) : float(606);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(506) : float(433);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(384) : float(563);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(474) : float(410);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(418) : float(586);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(444) : float(407);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(442) : float(472);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(407) : float(444);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(472) : float(442);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(410) : float(474);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(586) : float(418);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(433) : float(506);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(563) : float(384);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(469) : float(507);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(606) : float(341);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(504) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(662) : float(273);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(505) : float(513);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(703) : float(215);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(508) : float(510);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(720) : float(161);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(511) : float(528);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(142) : float(707);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(513) : float(508);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(181) : float(694);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(513) : float(505);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(231) : float(661);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(517) : float(519);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(253) : float(647);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(513) : float(540);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(312) : float(610);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(518) : float(558);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(427) : float(589);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(542) : float(587);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(402) : float(554);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(587) : float(601);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(456) : float(572);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(592) : float(615);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(461) : float(503);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(615) : float(592);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(503) : float(461);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(601) : float(587);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(572) : float(456);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(587) : float(542);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(554) : float(402);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(558) : float(518);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(589) : float(427);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(540) : float(513);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(610) : float(312);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(519) : float(517);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(647) : float(253);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(505) : float(513);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(661) : float(231);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(508) : float(513);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(694) : float(181);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(528) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(707) : float(142);
			break;

		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(534) : float(525);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(132) : float(707);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(541) : float(508);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(147) : float(681);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(585) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(159) : float(651);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(593) : float(525);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(189) : float(616);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(595) : float(554);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(229) : float(593);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(597) : float(572);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(265) : float(573);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(598) : float(599);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(306) : float(546);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(609) : float(619);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(419) : float(518);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(700) : float(628);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(359) : float(489);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(658) : float(645);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(375) : float(457);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(676) : float(665);
			UnitY = float(19);
			UnitZ = GetNation() == KARUS ? float(470) : float(439);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(665) : float(676);
			UnitY = float(22);
			UnitZ = GetNation() == KARUS ? float(439) : float(470);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(645) : float(658);
			UnitY = float(24);
			UnitZ = GetNation() == KARUS ? float(457) : float(375);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(628) : float(700);
			UnitY = float(27);
			UnitZ = GetNation() == KARUS ? float(489) : float(359);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(619) : float(609);
			UnitY = float(21);
			UnitZ = GetNation() == KARUS ? float(518) : float(419);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(599) : float(598);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(546) : float(306);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(572) : float(597);
			UnitY = float(23);
			UnitZ = GetNation() == KARUS ? float(573) : float(265);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(554) : float(595);
			UnitY = float(23);
			UnitZ = GetNation() == KARUS ? float(593) : float(229);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(525) : float(593);
			UnitY = float(23);
			UnitZ = GetNation() == KARUS ? float(616) : float(189);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(511) : float(585);
			UnitY = float(20);
			UnitZ = GetNation() == KARUS ? float(651) : float(159);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(508) : float(541);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(681) : float(147);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(525) : float(534);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(707) : float(132);
			break;

		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(484) : float(491);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(121) : float(766);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(451) : float(501);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(145) : float(661);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(439) : float(503);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(155) : float(623);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(424) : float(504);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(200) : float(589);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(432) : float(486);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(238) : float(564);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(435) : float(469);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(278) : float(543);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(424) : float(450);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(308) : float(504);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(409) : float(467);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(346) : float(478);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(434) : float(498);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(355) : float(452);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(450) : float(501);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(375) : float(414);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(461) : float(482);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(385) : float(396);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(482) : float(461);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(396) : float(385);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(501) : float(450);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(414) : float(375);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(498) : float(434);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(452) : float(355);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(467) : float(409);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(478) : float(346);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(450) : float(424);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(504) : float(308);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(469) : float(435);
			UnitY = float(18);
			UnitZ = GetNation() == KARUS ? float(543) : float(278);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(486) : float(432);
			UnitY = float(23);
			UnitZ = GetNation() == KARUS ? float(564) : float(238);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(504) : float(424);
			UnitY = float(24);
			UnitZ = GetNation() == KARUS ? float(589) : float(200);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(503) : float(439);
			UnitY = float(21);
			UnitZ = GetNation() == KARUS ? float(623) : float(155);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(501) : float(451);
			UnitY = float(15);
			UnitZ = GetNation() == KARUS ? float(661) : float(145);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(491) : float(484);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(766) : float(121);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(503) : float(508);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(146) : float(909);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(492) : float(517);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(168) : float(880);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(454) : float(512);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(235) : float(921);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(480) : float(507);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(211) : float(883);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(486) : float(515);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(250) : float(776);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(494) : float(519);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(291) : float(738);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(507) : float(530);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(367) : float(712);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(511) : float(507);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(392) : float(666);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(515) : float(509);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(429) : float(622);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(537) : float(523);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(463) : float(575);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(574) : float(530);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(509) : float(562);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(566) : float(566);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(525) : float(525);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(530) : float(574);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(562) : float(509);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(523) : float(537);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(575) : float(463);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(509) : float(515);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(622) : float(429);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(507) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(666) : float(392);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(530) : float(507);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(712) : float(367);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(519) : float(494);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(738) : float(291);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(515) : float(486);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(776) : float(250);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(507) : float(480);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(883) : float(211);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(512) : float(454);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(921) : float(235);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(517) : float(492);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(880) : float(168);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(508) : float(503);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(909) : float(146);
			break;

		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(506) : float(502);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(133) : float(898);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(478) : float(524);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(207) : float(915);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(494) : float(492);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(174) : float(878);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(505) : float(485);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(189) : float(874);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(510) : float(487);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(221) : float(818);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(512) : float(488);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(240) : float(791);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(510) : float(491);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(309) : float(762);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(509) : float(495);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(337) : float(724);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(512) : float(504);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(397) : float(679);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(516) : float(505);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(434) : float(639);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(535) : float(506);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(458) : float(603);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(585) : float(502);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(531) : float(581);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(571) : float(476);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(505) : float(553);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(552) : float(456);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(511) : float(533);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(534) : float(450);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(509) : float(508);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(518) : float(435);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(489) : float(491);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(504) : float(489);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(493) : float(518);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(474) : float(521);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(521) : float(532);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(530) : float(530);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(578) : float(578);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(521) : float(474);
			UnitY = float(14);
			UnitZ = GetNation() == KARUS ? float(532) : float(521);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(489) : float(504);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(518) : float(493);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(435) : float(518);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(491) : float(489);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(450) : float(534);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(508) : float(509);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(456) : float(552);
			UnitY = float(14);
			UnitZ = GetNation() == KARUS ? float(533) : float(511);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(476) : float(571);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(553) : float(505);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(502) : float(585);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(581) : float(531);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(506) : float(535);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(603) : float(458);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(505) : float(516);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(639) : float(434);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(504) : float(512);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(679) : float(397);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(495) : float(509);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(724) : float(337);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(491) : float(510);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(762) : float(309);
			break;
		case 32:
			UnitX = GetNation() == KARUS ? float(488) : float(512);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(791) : float(240);
			break;
		case 33:
			UnitX = GetNation() == KARUS ? float(487) : float(510);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(818) : float(221);
			break;
		case 34:
			UnitX = GetNation() == KARUS ? float(485) : float(505);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(874) : float(189);
			break;
		case 35:
			UnitX = GetNation() == KARUS ? float(492) : float(494);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(878) : float(174);
			break;
		case 36:
			UnitX = GetNation() == KARUS ? float(524) : float(478);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(915) : float(207);
			break;
		case 37:
			UnitX = GetNation() == KARUS ? float(502) : float(506);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(898) : float(133);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(510) : float(509);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(141) : float(919);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(511) : float(507);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(177) : float(904);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(513) : float(512);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(190) : float(798);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(516) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(237) : float(739);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(513) : float(509);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(296) : float(693);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(512) : float(508);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(331) : float(631);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(514) : float(512);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(363) : float(603);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(515) : float(531);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(405) : float(567);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(512) : float(556);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(435) : float(538);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(484) : float(571);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(463) : float(515);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(455) : float(545);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(490) : float(508);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(452) : float(580);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(513) : float(505);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(480) : float(480);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(508) : float(508);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(580) : float(452);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(505) : float(513);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(545) : float(455);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(508) : float(490);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(571) : float(484);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(515) : float(463);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(556) : float(512);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(538) : float(435);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(531) : float(515);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(567) : float(405);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(512) : float(514);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(603) : float(363);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(508) : float(512);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(631) : float(331);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(509) : float(513);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(693) : float(296);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(511) : float(516);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(739) : float(237);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(512) : float(513);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(798) : float(190);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(507) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(904) : float(177);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(509) : float(510);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(919) : float(141);
			break;

		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(511) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(145) : float(916);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(509) : float(513);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(229) : float(892);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(512) : float(508);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(184) : float(850);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(515) : float(506);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(227) : float(828);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(514) : float(497);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(278) : float(791);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(511) : float(492);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(322) : float(765);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(511) : float(493);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(377) : float(739);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(453) : float(505);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(397) : float(683);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(426) : float(506);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(402) : float(648);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(406) : float(505);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(418) : float(611);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(388) : float(507);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(442) : float(585);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(376) : float(504);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(465) : float(575);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(373) : float(463);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(493) : float(537);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(378) : float(448);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(528) : float(516);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(382) : float(465);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(567) : float(478);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(386) : float(500);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(594) : float(438);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(421) : float(517);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(628) : float(408);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(441) : float(540);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(633) : float(384);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(517) : float(573);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(646) : float(383);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(501) : float(612);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(636) : float(412);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(553) : float(629);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(627) : float(440);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(566) : float(639);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(630) : float(468);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(600) : float(634);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(616) : float(505);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(617) : float(633);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(598) : float(534);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(638) : float(638);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(559) : float(559);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(633) : float(617);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(534) : float(598);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(634) : float(600);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(505) : float(616);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(639) : float(566);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(468) : float(630);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(629) : float(553);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(440) : float(627);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(612) : float(501);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(412) : float(636);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(573) : float(517);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(383) : float(646);
			break;
		case 32:
			UnitX = GetNation() == KARUS ? float(540) : float(441);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(384) : float(633);
			break;
		case 33:
			UnitX = GetNation() == KARUS ? float(517) : float(421);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(408) : float(628);
			break;
		case 34:
			UnitX = GetNation() == KARUS ? float(500) : float(386);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(438) : float(594);
			break;
		case 35:
			UnitX = GetNation() == KARUS ? float(465) : float(382);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(478) : float(567);
			break;
		case 36:
			UnitX = GetNation() == KARUS ? float(448) : float(378);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(516) : float(528);
			break;
		case 37:
			UnitX = GetNation() == KARUS ? float(463) : float(373);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(537) : float(493);
			break;
		case 38:
			UnitX = GetNation() == KARUS ? float(504) : float(376);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(575) : float(465);
			break;
		case 39:
			UnitX = GetNation() == KARUS ? float(507) : float(388);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(585) : float(442);
			break;
		case 40:
			UnitX = GetNation() == KARUS ? float(505) : float(406);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(611) : float(418);
			break;
		case 41:
			UnitX = GetNation() == KARUS ? float(506) : float(426);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(648) : float(402);
			break;
		case 42:
			UnitX = GetNation() == KARUS ? float(505) : float(453);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(683) : float(397);
			break;
		case 43:
			UnitX = GetNation() == KARUS ? float(493) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(739) : float(377);
			break;
		case 44:
			UnitX = GetNation() == KARUS ? float(492) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(765) : float(322);
			break;
		case 45:
			UnitX = GetNation() == KARUS ? float(497) : float(514);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(791) : float(278);
			break;
		case 46:
			UnitX = GetNation() == KARUS ? float(506) : float(515);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(828) : float(227);
			break;
		case 47:
			UnitX = GetNation() == KARUS ? float(508) : float(512);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(850) : float(184);
			break;
		case 48:
			UnitX = GetNation() == KARUS ? float(513) : float(509);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(892) : float(229);
			break;
		case 49:
			UnitX = GetNation() == KARUS ? float(511) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(916) : float(145);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(506) : float(502);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(133) : float(898);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(478) : float(524);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(207) : float(915);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(494) : float(492);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(174) : float(878);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(505) : float(485);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(189) : float(874);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(510) : float(487);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(221) : float(818);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(512) : float(488);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(240) : float(791);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(510) : float(491);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(309) : float(762);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(509) : float(495);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(337) : float(724);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(512) : float(504);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(397) : float(679);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(516) : float(505);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(434) : float(639);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(535) : float(506);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(458) : float(603);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(585) : float(502);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(531) : float(581);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(571) : float(476);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(505) : float(553);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(552) : float(456);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(511) : float(533);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(534) : float(450);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(509) : float(508);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(518) : float(435);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(489) : float(491);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(504) : float(489);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(493) : float(518);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(474) : float(521);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(521) : float(532);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(530) : float(530);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(578) : float(578);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(521) : float(474);
			UnitY = float(14);
			UnitZ = GetNation() == KARUS ? float(532) : float(521);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(489) : float(504);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(518) : float(493);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(435) : float(518);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(491) : float(489);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(450) : float(534);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(508) : float(509);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(456) : float(552);
			UnitY = float(14);
			UnitZ = GetNation() == KARUS ? float(533) : float(511);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(476) : float(571);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(553) : float(505);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(502) : float(585);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(581) : float(531);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(506) : float(535);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(603) : float(458);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(505) : float(516);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(639) : float(434);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(504) : float(512);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(679) : float(397);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(495) : float(509);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(724) : float(337);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(491) : float(510);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(762) : float(309);
			break;
		case 32:
			UnitX = GetNation() == KARUS ? float(488) : float(512);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(791) : float(240);
			break;
		case 33:
			UnitX = GetNation() == KARUS ? float(487) : float(510);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(818) : float(221);
			break;
		case 34:
			UnitX = GetNation() == KARUS ? float(485) : float(505);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(874) : float(189);
			break;
		case 35:
			UnitX = GetNation() == KARUS ? float(492) : float(494);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(878) : float(174);
			break;
		case 36:
			UnitX = GetNation() == KARUS ? float(524) : float(478);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(915) : float(207);
			break;
		case 37:
			UnitX = GetNation() == KARUS ? float(502) : float(506);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(898) : float(133);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {

		case 1:
			UnitX = GetNation() == KARUS ? float(511) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(145) : float(916);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(509) : float(513);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(229) : float(892);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(512) : float(508);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(184) : float(850);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(515) : float(506);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(227) : float(828);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(514) : float(497);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(278) : float(791);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(511) : float(492);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(322) : float(765);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(511) : float(493);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(377) : float(739);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(453) : float(505);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(397) : float(683);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(426) : float(506);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(402) : float(648);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(406) : float(505);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(418) : float(611);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(388) : float(507);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(442) : float(585);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(376) : float(504);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(465) : float(575);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(373) : float(463);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(493) : float(537);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(378) : float(448);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(528) : float(516);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(382) : float(465);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(567) : float(478);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(386) : float(500);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(594) : float(438);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(421) : float(517);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(628) : float(408);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(441) : float(540);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(633) : float(384);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(517) : float(573);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(646) : float(383);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(501) : float(612);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(636) : float(412);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(553) : float(629);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(627) : float(440);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(566) : float(639);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(630) : float(468);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(600) : float(634);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(616) : float(505);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(617) : float(633);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(598) : float(534);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(638) : float(638);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(559) : float(559);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(633) : float(617);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(534) : float(598);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(634) : float(600);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(505) : float(616);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(639) : float(566);
			UnitY = float(8);
			UnitZ = GetNation() == KARUS ? float(468) : float(630);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(629) : float(553);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(440) : float(627);
			break;
		case 30:
			UnitX = GetNation() == KARUS ? float(612) : float(501);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(412) : float(636);
			break;
		case 31:
			UnitX = GetNation() == KARUS ? float(573) : float(517);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(383) : float(646);
			break;
		case 32:
			UnitX = GetNation() == KARUS ? float(540) : float(441);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(384) : float(633);
			break;
		case 33:
			UnitX = GetNation() == KARUS ? float(517) : float(421);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(408) : float(628);
			break;
		case 34:
			UnitX = GetNation() == KARUS ? float(500) : float(386);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(438) : float(594);
			break;
		case 35:
			UnitX = GetNation() == KARUS ? float(465) : float(382);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(478) : float(567);
			break;
		case 36:
			UnitX = GetNation() == KARUS ? float(448) : float(378);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(516) : float(528);
			break;
		case 37:
			UnitX = GetNation() == KARUS ? float(463) : float(373);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(537) : float(493);
			break;
		case 38:
			UnitX = GetNation() == KARUS ? float(504) : float(376);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(575) : float(465);
			break;
		case 39:
			UnitX = GetNation() == KARUS ? float(507) : float(388);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(585) : float(442);
			break;
		case 40:
			UnitX = GetNation() == KARUS ? float(505) : float(406);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(611) : float(418);
			break;
		case 41:
			UnitX = GetNation() == KARUS ? float(506) : float(426);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(648) : float(402);
			break;
		case 42:
			UnitX = GetNation() == KARUS ? float(505) : float(453);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(683) : float(397);
			break;
		case 43:
			UnitX = GetNation() == KARUS ? float(493) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(739) : float(377);
			break;
		case 44:
			UnitX = GetNation() == KARUS ? float(492) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(765) : float(322);
			break;
		case 45:
			UnitX = GetNation() == KARUS ? float(497) : float(514);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(791) : float(278);
			break;
		case 46:
			UnitX = GetNation() == KARUS ? float(506) : float(515);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(828) : float(227);
			break;
		case 47:
			UnitX = GetNation() == KARUS ? float(508) : float(512);
			UnitY = float(9);
			UnitZ = GetNation() == KARUS ? float(850) : float(184);
			break;
		case 48:
			UnitX = GetNation() == KARUS ? float(513) : float(509);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(892) : float(229);
			break;
		case 49:
			UnitX = GetNation() == KARUS ? float(511) : float(511);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(916) : float(145);
			break;

		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialMoradonWar()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(146) : float(137);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(596) : float(865);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(138) : float(137);
			UnitY = float(6537);
			UnitZ = GetNation() == KARUS ? float(621) : float(875);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(125) : float(135);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(655) : float(791);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(119) : float(131);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(682) : float(776);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(117) : float(122);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(717) : float(747);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(122) : float(117);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(747) : float(717);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(131) : float(119);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(776) : float(682);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(135) : float(125);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(791) : float(655);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(137) : float(138);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(875) : float(621);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(137) : float(146);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(865) : float(596);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(161) : float(165);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(597) : float(853);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(160) : float(170);
			UnitY = float(6535);
			UnitZ = GetNation() == KARUS ? float(621) : float(798);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(163) : float(169);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(627) : float(767);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(166) : float(172);
			UnitY = float(6534);
			UnitZ = GetNation() == KARUS ? float(663) : float(751);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(169) : float(175);
			UnitY = float(6535);
			UnitZ = GetNation() == KARUS ? float(695) : float(725);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(175) : float(169);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(725) : float(695);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(172) : float(166);
			UnitY = float(6534);
			UnitZ = GetNation() == KARUS ? float(751) : float(663);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(169) : float(163);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(767) : float(627);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(170) : float(160);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(798) : float(621);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(165) : float(161);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(853) : float(597);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {

		case 1:
			UnitX = GetNation() == KARUS ? float(152) : float(148);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(595) : float(839);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(154) : float(147);
			UnitY = float(6534);
			UnitZ = GetNation() == KARUS ? float(671) : float(812);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(154) : float(147);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(633) : float(792);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(156) : float(151);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(642) : float(768);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(160) : float(160);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(678) : float(731);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(161) : float(161);
			UnitY = float(6534);
			UnitZ = GetNation() == KARUS ? float(714) : float(714);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(160) : float(160);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(731) : float(678);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(151) : float(156);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(768) : float(642);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(147) : float(154);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(792) : float(633);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(147) : float(154);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(812) : float(671);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(148) : float(152);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(839) : float(595);
			break;
		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(150) : float(155);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(599) : float(847);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(149) : float(149);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(618) : float(832);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(146) : float(141);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(636) : float(794);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(140) : float(140);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(682) : float(738);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(139) : float(139);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(714) : float(714);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(140) : float(140);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(738) : float(682);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(141) : float(146);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(794) : float(636);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(149) : float(149);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(832) : float(618);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(155) : float(150);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(847) : float(599);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(150) : float(155);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(599) : float(847);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(149) : float(149);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(618) : float(832);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(146) : float(141);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(636) : float(794);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(140) : float(140);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(682) : float(738);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(139) : float(139);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(714) : float(714);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(140) : float(140);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(738) : float(682);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(141) : float(146);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(794) : float(636);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(149) : float(149);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(832) : float(618);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(155) : float(150);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(847) : float(599);
			break;
		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(132) : float(142);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(601) : float(841);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(126) : float(139);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(628) : float(832);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(115) : float(126);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(650) : float(814);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(112) : float(111);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(680) : float(786);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(104) : float(104);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(706) : float(748);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(103) : float(103);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(734) : float(734);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(104) : float(104);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(748) : float(706);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(111) : float(112);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(786) : float(680);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(126) : float(115);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(814) : float(650);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(139) : float(126);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(832) : float(628);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(142) : float(132);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(841) : float(601);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(148) : float(181);
			UnitY = float(6538);
			UnitZ = GetNation() == KARUS ? float(593) : float(819);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(144) : float(190);
			UnitY = float(6536);
			UnitZ = GetNation() == KARUS ? float(613) : float(782);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(143) : float(189);
			UnitY = float(6538);
			UnitZ = GetNation() == KARUS ? float(622) : float(775);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(156) : float(204);
			UnitY = float(6539);
			UnitZ = GetNation() == KARUS ? float(657) : float(746);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(169) : float(204);
			UnitY = float(6539);
			UnitZ = GetNation() == KARUS ? float(691) : float(725);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(174) : float(174);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(703) : float(703);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(204) : float(169);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(725) : float(691);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(204) : float(156);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(746) : float(657);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(189) : float(143);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(775) : float(622);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(190) : float(144);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(782) : float(613);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(181) : float(148);
			UnitY = float(6533);
			UnitZ = GetNation() == KARUS ? float(819) : float(593);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(180) : float(159);
			UnitY = float(6538);
			UnitZ = GetNation() == KARUS ? float(543) : float(915);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(214) : float(212);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(544) : float(914);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(238) : float(239);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(544) : float(913);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(293) : float(289);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(539) : float(909);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(327) : float(341);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(536) : float(910);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(352) : float(359);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(542) : float(912);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(377) : float(401);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(553) : float(911);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(408) : float(368);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(565) : float(895);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(414) : float(435);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(586) : float(918);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(419) : float(459);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(627) : float(921);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(423) : float(464);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(678) : float(913);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(433) : float(472);
			UnitY = float(6552);
			UnitZ = GetNation() == KARUS ? float(718) : float(880);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(439) : float(474);
			UnitY = float(6551);
			UnitZ = GetNation() == KARUS ? float(738) : float(858);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(459) : float(471);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(799) : float(829);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(471) : float(459);
			UnitY = float(6546);
			UnitZ = GetNation() == KARUS ? float(829) : float(799);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(474) : float(439);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(858) : float(738);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(472) : float(433);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(880) : float(718);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(464) : float(423);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(913) : float(678);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(459) : float(419);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(921) : float(627);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(435) : float(414);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(918) : float(586);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(368) : float(408);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(895) : float(565);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(401) : float(377);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(911) : float(553);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(359) : float(352);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(912) : float(542);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(341) : float(327);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(910) : float(536);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(289) : float(293);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(909) : float(539);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(239) : float(238);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(913) : float(544);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(212) : float(214);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(914) : float(544);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(159) : float(180);
			UnitY = float(6538);
			UnitZ = GetNation() == KARUS ? float(915) : float(543);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(180) : float(159);
			UnitY = float(6538);
			UnitZ = GetNation() == KARUS ? float(543) : float(915);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(214) : float(212);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(544) : float(914);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(238) : float(239);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(544) : float(913);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(293) : float(289);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(539) : float(909);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(327) : float(341);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(536) : float(910);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(352) : float(359);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(542) : float(912);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(377) : float(401);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(553) : float(911);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(408) : float(368);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(565) : float(895);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(414) : float(435);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(586) : float(918);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(419) : float(459);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(627) : float(921);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(423) : float(464);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(678) : float(913);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(433) : float(472);
			UnitY = float(6552);
			UnitZ = GetNation() == KARUS ? float(718) : float(880);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(439) : float(474);
			UnitY = float(6551);
			UnitZ = GetNation() == KARUS ? float(738) : float(858);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(459) : float(471);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(799) : float(829);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(471) : float(459);
			UnitY = float(6546);
			UnitZ = GetNation() == KARUS ? float(829) : float(799);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(474) : float(439);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(858) : float(738);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(472) : float(433);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(880) : float(718);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(464) : float(423);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(913) : float(678);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(459) : float(419);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(921) : float(627);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(435) : float(414);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(918) : float(586);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(368) : float(408);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(895) : float(565);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(401) : float(377);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(911) : float(553);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(359) : float(352);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(912) : float(542);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(341) : float(327);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(910) : float(536);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(289) : float(293);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(909) : float(539);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(239) : float(238);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(913) : float(544);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(212) : float(214);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(914) : float(544);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(159) : float(180);
			UnitY = float(6538);
			UnitZ = GetNation() == KARUS ? float(915) : float(543);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(180) : float(159);
			UnitY = float(6538);
			UnitZ = GetNation() == KARUS ? float(543) : float(915);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(214) : float(212);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(544) : float(914);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(238) : float(239);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(544) : float(913);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(293) : float(289);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(539) : float(909);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(327) : float(341);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(536) : float(910);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(352) : float(359);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(542) : float(912);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(377) : float(401);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(553) : float(911);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(408) : float(368);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(565) : float(895);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(414) : float(435);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(586) : float(918);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(419) : float(459);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(627) : float(921);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(423) : float(464);
			UnitY = float(6550);
			UnitZ = GetNation() == KARUS ? float(678) : float(913);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(433) : float(472);
			UnitY = float(6552);
			UnitZ = GetNation() == KARUS ? float(718) : float(880);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(439) : float(474);
			UnitY = float(6551);
			UnitZ = GetNation() == KARUS ? float(738) : float(858);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(459) : float(471);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(799) : float(829);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(471) : float(459);
			UnitY = float(6546);
			UnitZ = GetNation() == KARUS ? float(829) : float(799);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(474) : float(439);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(858) : float(738);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(472) : float(433);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(880) : float(718);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(464) : float(423);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(913) : float(678);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(459) : float(419);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(921) : float(627);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(435) : float(414);
			UnitY = float(6548);
			UnitZ = GetNation() == KARUS ? float(918) : float(586);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(368) : float(408);
			UnitY = float(6549);
			UnitZ = GetNation() == KARUS ? float(895) : float(565);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(401) : float(377);
			UnitY = float(2);
			UnitZ = GetNation() == KARUS ? float(911) : float(553);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(359) : float(352);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(912) : float(542);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(341) : float(327);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(910) : float(536);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(289) : float(293);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(909) : float(539);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(239) : float(238);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(913) : float(544);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(212) : float(214);
			UnitY = float(6547);
			UnitZ = GetNation() == KARUS ? float(914) : float(544);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(159) : float(180);
			UnitY = float(6538);
			UnitZ = GetNation() == KARUS ? float(915) : float(543);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialSnowWar()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(252) : float(241);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(142) : float(349);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(244) : float(243);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(160) : float(269);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(242) : float(242);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(185) : float(225);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(242) : float(242);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(225) : float(185);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(243) : float(244);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(269) : float(160);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(241) : float(252);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(349) : float(142);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(259) : float(255);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(126) : float(371);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(256) : float(254);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(167) : float(343);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(254) : float(253);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(249) : float(279);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(253) : float(254);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(279) : float(249);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(254) : float(256);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(343) : float(167);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(255) : float(259);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(371) : float(126);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(266) : float(264);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(144) : float(372);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(265) : float(264);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(166) : float(333);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(266) : float(266);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(199) : float(330);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(266) : float(266);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(239) : float(239);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(266) : float(266);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(330) : float(199);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(264) : float(265);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(333) : float(166);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(264) : float(266);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(372) : float(144);
			break;
		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(261) : float(244);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(128) : float(372);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(259) : float(237);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(163) : float(397);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(258) : float(246);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(187) : float(328);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(251) : float(251);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(200) : float(305);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(222) : float(219);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(219) : float(280);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(206) : float(214);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(238) : float(276);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(202) : float(202);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(265) : float(265);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(214) : float(206);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(276) : float(238);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(219) : float(222);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(280) : float(219);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(251) : float(251);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(305) : float(200);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(246) : float(258);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(328) : float(187);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(237) : float(259);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(397) : float(163);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(244) : float(261);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(372) : float(128);
			break;

		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(268) : float(271);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(121) : float(373);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(268) : float(271);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(148) : float(338);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(270) : float(272);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(164) : float(365);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(280) : float(275);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(208) : float(291);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(322) : float(281);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(263) : float(284);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(308) : float(297);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(238) : float(306);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(309) : float(309);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(268) : float(268);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(297) : float(308);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(306) : float(238);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(281) : float(322);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(284) : float(263);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(275) : float(280);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(291) : float(208);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(272) : float(270);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(365) : float(164);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(271) : float(268);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(338) : float(148);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(271) : float(268);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(373) : float(121);
			break;
		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(268) : float(271);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(121) : float(373);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(268) : float(271);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(148) : float(338);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(270) : float(272);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(164) : float(365);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(280) : float(275);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(208) : float(291);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(322) : float(281);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(263) : float(284);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(308) : float(297);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(238) : float(306);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(309) : float(309);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(268) : float(268);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(297) : float(308);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(306) : float(238);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(281) : float(322);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(284) : float(263);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(275) : float(280);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(291) : float(208);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(272) : float(270);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(365) : float(164);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(271) : float(268);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(338) : float(148);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(271) : float(268);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(373) : float(121);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(268) : float(271);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(121) : float(373);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(268) : float(271);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(148) : float(338);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(270) : float(272);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(164) : float(365);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(280) : float(275);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(208) : float(291);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(322) : float(281);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(263) : float(284);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(308) : float(297);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(238) : float(306);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(309) : float(309);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(268) : float(268);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(297) : float(308);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(306) : float(238);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(281) : float(322);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(284) : float(263);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(275) : float(280);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(291) : float(208);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(272) : float(270);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(365) : float(164);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(271) : float(268);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(338) : float(148);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(271) : float(268);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(373) : float(121);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(261) : float(262);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(66) : float(443);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(249) : float(247);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(67) : float(442);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(247) : float(244);
			UnitY = float(16);
			UnitZ = GetNation() == KARUS ? float(97) : float(417);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(250) : float(243);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(117) : float(394);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(250) : float(242);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(145) : float(348);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(249) : float(241);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(178) : float(284);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(247) : float(245);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(218) : float(242);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(245) : float(247);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(242) : float(218);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(241) : float(249);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(284) : float(178);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(242) : float(250);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(348) : float(145);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(243) : float(250);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(394) : float(117);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(244) : float(247);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(417) : float(97);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(247) : float(249);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(442) : float(67);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(262) : float(261);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(443) : float(66);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(265) : float(245);
			UnitY = float(13);
			UnitZ = GetNation() == KARUS ? float(78) : float(398);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(261) : float(247);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(68) : float(340);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(247) : float(240);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(67) : float(276);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(242) : float(229);
			UnitY = float(11);
			UnitZ = GetNation() == KARUS ? float(158) : float(239);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(245) : float(240);
			UnitY = float(12);
			UnitZ = GetNation() == KARUS ? float(122) : float(271);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(204) : float(215);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(151) : float(180);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(214) : float(214);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(166) : float(166);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(215) : float(204);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(180) : float(151);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(240) : float(245);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(271) : float(122);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(229) : float(242);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(239) : float(158);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(240) : float(247);
			UnitY = float(16);
			UnitZ = GetNation() == KARUS ? float(276) : float(67);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(247) : float(261);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(340) : float(68);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(245) : float(265);
			UnitY = float(16);
			UnitZ = GetNation() == KARUS ? float(398) : float(78);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(265) : float(268);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(127) : float(373);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(301) : float(273);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(151) : float(338);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(294) : float(266);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(145) : float(368);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(305) : float(278);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(166) : float(296);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(299) : float(287);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(195) : float(262);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(295) : float(295);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(230) : float(230);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(287) : float(299);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(262) : float(195);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(278) : float(305);
			UnitY = float(17);
			UnitZ = GetNation() == KARUS ? float(296) : float(166);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(266) : float(294);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(368) : float(145);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(273) : float(301);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(338) : float(151);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(268) : float(265);
			UnitY = float(10);
			UnitZ = GetNation() == KARUS ? float(373) : float(127);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialNightDelosWar()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(433) : float(551);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(413) : float(860);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(457) : float(544);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(424) : float(837);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(470) : float(544);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(443) : float(876);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(474) : float(535);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(478) : float(786);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(485) : float(520);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(494) : float(768);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(501) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(526) : float(747);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(505) : float(511);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(548) : float(776);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(508) : float(503);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(574) : float(703);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(491) : float(511);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(584) : float(717);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(464) : float(481);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(616) : float(680);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(463) : float(463);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(622) : float(622);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(481) : float(464);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(680) : float(616);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(511) : float(491);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(717) : float(584);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(503) : float(508);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(703) : float(574);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(511) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(776) : float(548);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(507) : float(501);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(747) : float(526);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(520) : float(485);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(768) : float(494);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(535) : float(474);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(786) : float(478);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(544) : float(470);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(876) : float(443);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(544) : float(457);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(837) : float(424);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(551) : float(433);
			UnitY = float(87);
			UnitZ = GetNation() == KARUS ? float(860) : float(413);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(456) : float(565);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(414) : float(851);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(476) : float(557);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(422) : float(848);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(485) : float(555);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(427) : float(891);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(543) : float(541);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(461) : float(825);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(536) : float(538);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(457) : float(806);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(538) : float(542);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(528) : float(797);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(528) : float(523);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(499) : float(767);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(516) : float(542);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(517) : float(797);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(500) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(543) : float(737);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(506) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(550) : float(718);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(537) : float(501);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(575) : float(718);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(533) : float(501);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(574) : float(722);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(545) : float(543);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(597) : float(674);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(586) : float(536);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(671) : float(697);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(562) : float(562);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(630) : float(630);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(536) : float(586);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(697) : float(671);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(543) : float(545);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(674) : float(597);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(501) : float(533);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(722) : float(574);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(501) : float(537);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(718) : float(575);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(505) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(718) : float(550);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(506) : float(500);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(737) : float(543);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(542) : float(516);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(797) : float(517);
			break;
		case 23:
			UnitX = GetNation() == KARUS ? float(523) : float(528);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(767) : float(499);
			break;
		case 24:
			UnitX = GetNation() == KARUS ? float(542) : float(538);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(797) : float(528);
			break;
		case 25:
			UnitX = GetNation() == KARUS ? float(538) : float(536);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(806) : float(457);
			break;
		case 26:
			UnitX = GetNation() == KARUS ? float(541) : float(543);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(825) : float(461);
			break;
		case 27:
			UnitX = GetNation() == KARUS ? float(555) : float(485);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(891) : float(427);
			break;
		case 28:
			UnitX = GetNation() == KARUS ? float(557) : float(476);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(848) : float(422);
			break;
		case 29:
			UnitX = GetNation() == KARUS ? float(565) : float(456);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(851) : float(414);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(449) : float(554);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(414) : float(848);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(460) : float(531);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(416) : float(843);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(470) : float(513);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(443) : float(854);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(475) : float(487);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(478) : float(839);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(485) : float(478);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(493) : float(854);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(507) : float(475);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(539) : float(810);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(505) : float(470);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(565) : float(804);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(506) : float(492);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(590) : float(769);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(512) : float(508);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(663) : float(757);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(506) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(630) : float(716);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(506) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(671) : float(705);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(506) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(705) : float(671);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(506) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(716) : float(630);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(508) : float(512);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(757) : float(663);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(492) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(769) : float(590);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(470) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(804) : float(565);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(475) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(810) : float(539);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(478) : float(485);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(854) : float(493);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(487) : float(475);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(839) : float(478);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(513) : float(470);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(854) : float(443);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(531) : float(460);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(843) : float(416);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(554) : float(449);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(848) : float(414);
			break;
		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(457) : float(603);
			UnitY = float(81);
			UnitZ = GetNation() == KARUS ? float(425) : float(875);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(475) : float(564);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(437) : float(889);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(475) : float(540);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(461) : float(818);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(473) : float(541);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(528) : float(799);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(475) : float(526);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(478) : float(771);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(512) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(536) : float(739);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(505) : float(503);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(525) : float(708);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(505) : float(499);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(556) : float(695);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(498) : float(508);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(568) : float(727);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(490) : float(492);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(601) : float(661);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(488) : float(486);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(616) : float(629);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(486) : float(488);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(629) : float(616);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(492) : float(490);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(661) : float(601);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(508) : float(498);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(727) : float(568);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(499) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(695) : float(556);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(503) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(708) : float(525);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(507) : float(512);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(739) : float(536);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(526) : float(475);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(771) : float(478);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(541) : float(473);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(799) : float(528);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(540) : float(475);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(818) : float(461);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(564) : float(475);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(889) : float(437);
			break;
		case 22:
			UnitX = GetNation() == KARUS ? float(603) : float(457);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(875) : float(425);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(454) : float(549);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(412) : float(860);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(470) : float(549);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(425) : float(888);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(473) : float(539);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(444) : float(801);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(473) : float(534);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(454) : float(788);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(480) : float(520);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(518) : float(766);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(489) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(503) : float(728);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(516) : float(515);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(534) : float(691);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(507) : float(523);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(558) : float(659);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(509) : float(529);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(577) : float(626);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(523) : float(527);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(603) : float(610);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(527) : float(523);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(610) : float(603);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(529) : float(509);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(626) : float(577);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(523) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(659) : float(558);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(515) : float(516);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(691) : float(534);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(506) : float(489);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(728) : float(503);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(520) : float(480);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(766) : float(518);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(534) : float(473);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(788) : float(454);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(539) : float(473);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(801) : float(444);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(549) : float(470);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(888) : float(425);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(549) : float(454);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(860) : float(412);
			break;
		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(452) : float(555);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(422) : float(849);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(469) : float(538);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(433) : float(829);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(475) : float(538);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(458) : float(787);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(478) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(518) : float(762);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(487) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(497) : float(708);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(505) : float(504);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(538) : float(709);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(512) : float(443);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(589) : float(679);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(481) : float(420);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(584) : float(658);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(429) : float(400);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(594) : float(639);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(419) : float(419);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(601) : float(601);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(400) : float(429);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(639) : float(594);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(420) : float(481);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(658) : float(584);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(443) : float(512);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(679) : float(589);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(504) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(709) : float(538);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(506) : float(487);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(708) : float(497);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(505) : float(478);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(762) : float(518);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(538) : float(475);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(787) : float(458);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(538) : float(469);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(829) : float(433);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(555) : float(452);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(849) : float(422);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(452) : float(555);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(422) : float(849);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(469) : float(538);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(433) : float(829);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(475) : float(538);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(458) : float(787);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(478) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(518) : float(762);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(487) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(497) : float(708);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(505) : float(504);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(538) : float(709);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(512) : float(443);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(589) : float(679);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(481) : float(420);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(584) : float(658);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(429) : float(400);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(594) : float(639);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(419) : float(419);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(601) : float(601);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(400) : float(429);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(639) : float(594);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(420) : float(481);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(658) : float(584);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(443) : float(512);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(679) : float(589);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(504) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(709) : float(538);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(506) : float(487);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(708) : float(497);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(505) : float(478);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(762) : float(518);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(538) : float(475);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(787) : float(458);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(538) : float(469);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(829) : float(433);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(555) : float(452);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(849) : float(422);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(447) : float(540);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(422) : float(848);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(474) : float(535);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(445) : float(785);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(473) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(487) : float(756);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(507) : float(509);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(529) : float(703);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(504) : float(504);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(572) : float(572);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(509) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(703) : float(529);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(507) : float(473);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(756) : float(487);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(535) : float(474);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(785) : float(445);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(540) : float(447);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(848) : float(422);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(451) : float(536);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(410) : float(847);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(473) : float(537);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(423) : float(795);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(536) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(460) : float(759);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(536) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(494) : float(718);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(504) : float(512);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(533) : float(672);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(505) : float(509);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(563) : float(602);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(509) : float(505);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(602) : float(563);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(512) : float(504);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(672) : float(533);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(507) : float(536);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(718) : float(494);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(507) : float(536);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(759) : float(460);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(537) : float(473);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(795) : float(423);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(536) : float(451);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(847) : float(410);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(457) : float(542);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(427) : float(841);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(472) : float(546);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(446) : float(877);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(473) : float(530);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(494) : float(781);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(506) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(518) : float(684);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(506) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(546) : float(658);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(506) : float(506);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(599) : float(599);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(507) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(658) : float(546);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(506) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(684) : float(518);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(530) : float(473);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(781) : float(494);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(546) : float(472);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(877) : float(446);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(542) : float(457);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(841) : float(427);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialNeoStarWar()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(107) : float(105);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(224) : float(34);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(108) : float(109);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(201) : float(53);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(113) : float(114);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(178) : float(92);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(115) : float(117);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(165) : float(135);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(117) : float(115);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(135) : float(165);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(114) : float(113);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(92) : float(178);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(109) : float(108);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(53) : float(201);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(105) : float(107);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(34) : float(224);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(49) : float(45);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(226) : float(31);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(44) : float(45);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(207) : float(81);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(43) : float(49);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(175) : float(120);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(50) : float(50);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(149) : float(149);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(49) : float(43);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(120) : float(175);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(45) : float(44);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(81) : float(207);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(45) : float(49);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(31) : float(226);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(108) : float(45);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(218) : float(35);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(103) : float(47);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(177) : float(63);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(85) : float(48);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(152) : float(86);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(67) : float(67);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(132) : float(132);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(48) : float(85);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(86) : float(152);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(47) : float(103);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(63) : float(177);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(45) : float(108);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(35) : float(218);
			break;
		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(45) : float(109);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(224) : float(34);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(47) : float(111);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(180) : float(75);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(65) : float(86);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(158) : float(73);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(81) : float(81);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(140) : float(119);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(81) : float(81);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(119) : float(140);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(86) : float(65);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(73) : float(158);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(111) : float(47);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(75) : float(180);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(109) : float(45);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(34) : float(224);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(107) : float(45);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(222) : float(35);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(108) : float(45);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(179) : float(79);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(106) : float(77);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(159) : float(96);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(99) : float(99);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(118) : float(118);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(77) : float(106);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(96) : float(159);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(45) : float(108);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(79) : float(179);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(45) : float(107);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(35) : float(222);
			break;

		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(84) : float(105);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(232) : float(32);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(39) : float(111);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(225) : float(52);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(51) : float(104);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(133) : float(86);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(48) : float(52);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(147) : float(112);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(52) : float(48);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(112) : float(147);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(104) : float(51);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(86) : float(133);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(111) : float(39);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(52) : float(225);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(105) : float(84);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(32) : float(232);
			break;

		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(46) : float(110);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(226) : float(26);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(45) : float(108);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(173) : float(78);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(78) : float(78);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(141) : float(91);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(78) : float(78);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(91) : float(141);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(108) : float(45);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(78) : float(173);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(110) : float(46);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(26) : float(226);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(107) : float(46);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(224) : float(30);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(115) : float(46);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(135) : float(56);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(78) : float(48);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(176) : float(92);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(61) : float(50);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(164) : float(109);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(77) : float(75);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(140) : float(95);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(75) : float(77);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(95) : float(140);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(50) : float(61);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(109) : float(164);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(48) : float(78);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(92) : float(176);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(46) : float(115);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(56) : float(135);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(46) : float(107);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(30) : float(224);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(44) : float(109);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(226) : float(32);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(46) : float(108);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(182) : float(87);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(73) : float(47);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(178) : float(98);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(132) : float(49);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(142) : float(128);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(101) : float(80);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(135) : float(135);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(80) : float(101);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(135) : float(135);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(49) : float(132);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(128) : float(142);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(47) : float(73);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(98) : float(178);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(108) : float(46);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(87) : float(182);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(109) : float(44);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(32) : float(226);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(110) : float(109);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(223) : float(28);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(101) : float(110);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(133) : float(108);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(106) : float(106);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(138) : float(138);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(110) : float(101);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(108) : float(133);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(109) : float(110);
			UnitY = float(0);
			UnitZ = GetNation() == KARUS ? float(28) : float(223);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialCastleWar()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(994) : float(1002);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(860) : float(1131);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(977) : float(1006);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(898) : float(1100);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(947) : float(1010);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(988) : float(1086);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1010) : float(947);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1086) : float(988);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1006) : float(977);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1100) : float(898);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1002) : float(994);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1131) : float(860);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1042) : float(1027);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(861) : float(1135);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1076) : float(1019);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(956) : float(1101);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1066) : float(1066);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1042) : float(1042);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1019) : float(1076);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1101) : float(956);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1027) : float(1042);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1135) : float(861);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1007) : float(1027);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(871) : float(1131);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(994) : float(1013);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(920) : float(1080);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1015) : float(1015);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(955) : float(1017);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1015) : float(1009);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(962) : float(1015);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(998) : float(998);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(975) : float(975);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1009) : float(1015);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1015) : float(962);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1015) : float(1015);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1017) : float(955);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(1013) : float(994);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1080) : float(920);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(1027) : float(1007);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1131) : float(871);
			break;
		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1038) : float(1033);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(866) : float(1124);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1038) : float(1079);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(866) : float(1093);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1034) : float(1063);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(961) : float(1047);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1069) : float(1069);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(967) : float(967);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1063) : float(1034);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1047) : float(961);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1079) : float(1038);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1093) : float(866);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1033) : float(1038);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1124) : float(866);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1005) : float(994);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(876) : float(1128);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(983) : float(939);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(917) : float(1027);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(951) : float(951);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(973) : float(973);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(939) : float(983);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1027) : float(917);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(994) : float(1005);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1128) : float(876);
			break;
		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1038) : float(1034);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(852) : float(1131);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1102) : float(1107);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(949) : float(1048);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1107) : float(1107);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1014) : float(1014);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1107) : float(1102);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1048) : float(949);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1034) : float(1038);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(1131) : float(852);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1003) : float(1051);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(864) : float(1120);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1016) : float(1036);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(956) : float(1060);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1020) : float(1015);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(965) : float(1020);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1035) : float(1035);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1002) : float(1002);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1015) : float(1020);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1020) : float(965);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1036) : float(1016);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1060) : float(956);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(1051) : float(1003);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1120) : float(864);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1001) : float(1028);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(862) : float(1128);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1003) : float(1008);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(907) : float(1077);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(971) : float(945);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(962) : float(1027);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(945) : float(971);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1027) : float(962);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(1008) : float(1003);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1077) : float(907);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(1028) : float(1001);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1128) : float(862);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1006) : float(1001);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(862) : float(1122);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(993) : float(934);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(907) : float(1040);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(934) : float(993);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1040) : float(907);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1001) : float(1006);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1122) : float(862);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(1048) : float(1026);
			UnitY = float(6);
			UnitZ = GetNation() == KARUS ? float(865) : float(1128);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(1080) : float(1024);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(990) : float(1088);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(1024) : float(1080);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1088) : float(990);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(1026) : float(1048);
			UnitY = float(7);
			UnitZ = GetNation() == KARUS ? float(1128) : float(865);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialMiniArdreamWar()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(676) : float(298);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(492) : float(554);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(639) : float(350);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(453) : float(550);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(593) : float(401);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(420) : float(488);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(526) : float(464);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(435) : float(420);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(464) : float(526);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(420) : float(435);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(401) : float(593);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(488) : float(420);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(350) : float(639);
			UnitY = float(76);
			UnitZ = GetNation() == KARUS ? float(550) : float(453);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(298) : float(676);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(554) : float(492);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(720) : float(310);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(498) : float(553);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(694) : float(362);
			UnitY = float(70);
			UnitZ = GetNation() == KARUS ? float(515) : float(563);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(597) : float(486);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(575) : float(593);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(542) : float(542);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(622) : float(622);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(486) : float(597);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(593) : float(575);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(362) : float(694);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(563) : float(515);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(310) : float(720);
			UnitY = float(73);
			UnitZ = GetNation() == KARUS ? float(553) : float(498);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(681) : float(294);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(493) : float(553);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(598) : float(341);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(518) : float(552);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(567) : float(464);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(519) : float(536);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(521) : float(521);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(512) : float(512);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(464) : float(567);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(536) : float(519);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(341) : float(598);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(552) : float(518);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(294) : float(681);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(553) : float(493);
			break;
		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(689) : float(319);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(492) : float(554);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(582) : float(405);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(519) : float(571);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(533) : float(517);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(544) : float(600);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(517) : float(533);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(600) : float(544);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(405) : float(582);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(571) : float(519);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(319) : float(689);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(554) : float(492);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(714) : float(291);
			UnitY = float(68);
			UnitZ = GetNation() == KARUS ? float(491) : float(535);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(607) : float(331);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(516) : float(535);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(565) : float(424);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(521) : float(462);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(518) : float(510);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(486) : float(456);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(510) : float(518);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(456) : float(486);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(424) : float(565);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(462) : float(521);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(331) : float(607);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(535) : float(516);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(291) : float(714);
			UnitY = float(74);
			UnitZ = GetNation() == KARUS ? float(535) : float(491);
			break;
		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(729) : float(253);
			UnitY = float(70);
			UnitZ = GetNation() == KARUS ? float(517) : float(567);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(670) : float(331);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(516) : float(586);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(588) : float(462);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(580) : float(535);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(531) : float(515);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(624) : float(540);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(515) : float(531);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(540) : float(624);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(462) : float(588);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(535) : float(580);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(331) : float(670);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(586) : float(516);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(253) : float(729);
			UnitY = float(72);
			UnitZ = GetNation() == KARUS ? float(567) : float(517);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(715) : float(282);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(498) : float(554);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(671) : float(327);
			UnitY = float(75);
			UnitZ = GetNation() == KARUS ? float(530) : float(553);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(576) : float(473);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(521) : float(536);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(540) : float(540);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(544) : float(544);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(473) : float(576);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(536) : float(521);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(327) : float(671);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(553) : float(530);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(282) : float(715);
			UnitY = float(74);
			UnitZ = GetNation() == KARUS ? float(554) : float(498);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(685) : float(268);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(492) : float(557);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(633) : float(321);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(579) : float(581);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(623) : float(348);
			UnitY = float(71);
			UnitZ = GetNation() == KARUS ? float(643) : float(597);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(603) : float(322);
			UnitY = float(89);
			UnitZ = GetNation() == KARUS ? float(691) : float(641);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(571) : float(474);
			UnitY = float(81);
			UnitZ = GetNation() == KARUS ? float(706) : float(678);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(520) : float(520);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(689) : float(689);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(474) : float(571);
			UnitY = float(73);
			UnitZ = GetNation() == KARUS ? float(678) : float(706);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(322) : float(603);
			UnitY = float(73);
			UnitZ = GetNation() == KARUS ? float(641) : float(691);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(348) : float(623);
			UnitY = float(76);
			UnitZ = GetNation() == KARUS ? float(597) : float(643);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(321) : float(633);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(581) : float(579);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(268) : float(685);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(557) : float(492);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(691) : float(256);
			UnitY = float(70);
			UnitZ = GetNation() == KARUS ? float(492) : float(553);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(626) : float(315);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(482) : float(578);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(577) : float(358);
			UnitY = float(72);
			UnitZ = GetNation() == KARUS ? float(468) : float(618);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(512) : float(405);
			UnitY = float(72);
			UnitZ = GetNation() == KARUS ? float(429) : float(707);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(472) : float(419);
			UnitY = float(75);
			UnitZ = GetNation() == KARUS ? float(410) : float(607);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(424) : float(416);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(462) : float(518);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(381) : float(381);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(550) : float(550);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(416) : float(424);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(518) : float(462);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(419) : float(472);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(607) : float(410);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(405) : float(512);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(707) : float(429);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(358) : float(577);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(618) : float(468);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(315) : float(626);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(578) : float(482);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(256) : float(691);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(553) : float(492);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(686) : float(244);
			UnitY = float(72);
			UnitZ = GetNation() == KARUS ? float(492) : float(552);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(599) : float(307);
			UnitY = float(69);
			UnitZ = GetNation() == KARUS ? float(518) : float(552);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(566) : float(378);
			UnitY = float(72);
			UnitZ = GetNation() == KARUS ? float(520) : float(556);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(494) : float(465);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(516) : float(535);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(465) : float(494);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(535) : float(516);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(378) : float(566);
			UnitY = float(78);
			UnitZ = GetNation() == KARUS ? float(556) : float(520);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(307) : float(599);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(552) : float(518);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(244) : float(686);
			UnitY = float(77);
			UnitZ = GetNation() == KARUS ? float(552) : float(492);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialBifrost()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(73) : float(226);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(744) : float(918);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(90) : float(211);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(756) : float(888);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(106) : float(195);
			UnitY = float(102);
			UnitZ = GetNation() == KARUS ? float(765) : float(873);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(138) : float(231);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(762) : float(816);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(161) : float(251);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(781) : float(792);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(209) : float(267);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(775) : float(759);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(212) : float(279);
			UnitY = float(126);
			UnitZ = GetNation() == KARUS ? float(767) : float(737);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(279) : float(212);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(737) : float(767);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(267) : float(209);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(759) : float(775);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(251) : float(161);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(792) : float(781);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(231) : float(138);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(816) : float(762);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(195) : float(106);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(873) : float(765);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(211) : float(90);
			UnitY = float(116);
			UnitZ = GetNation() == KARUS ? float(888) : float(756);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(226) : float(73);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(918) : float(744);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(68) : float(231);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(743) : float(925);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(105) : float(212);
			UnitY = float(110);
			UnitZ = GetNation() == KARUS ? float(766) : float(900);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(138) : float(197);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(762) : float(857);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(164) : float(207);
			UnitY = float(108);
			UnitZ = GetNation() == KARUS ? float(786) : float(834);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(205) : float(255);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(770) : float(793);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(238) : float(267);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(743) : float(761);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(267) : float(238);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(761) : float(743);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(255) : float(205);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(793) : float(770);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(207) : float(164);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(834) : float(786);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(197) : float(138);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(857) : float(762);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(212) : float(105);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(900) : float(766);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(231) : float(68);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(925) : float(743);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(69) : float(242);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(743) : float(946);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(91) : float(221);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(757) : float(910);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(102) : float(201);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(767) : float(870);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(131) : float(197);
			UnitY = float(106);
			UnitZ = GetNation() == KARUS ? float(759) : float(878);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(162) : float(258);
			UnitY = float(124);
			UnitZ = GetNation() == KARUS ? float(782) : float(825);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(196) : float(209);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(778) : float(771);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(235) : float(224);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(790) : float(758);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(262) : float(253);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(774) : float(746);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(253) : float(262);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(746) : float(774);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(224) : float(235);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(758) : float(790);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(209) : float(196);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(771) : float(778);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(258) : float(162);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(825) : float(782);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(197) : float(131);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(878) : float(759);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(201) : float(102);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(870) : float(767);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(221) : float(91);
			UnitY = float(116);
			UnitZ = GetNation() == KARUS ? float(910) : float(757);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(242) : float(69);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(946) : float(743);
			break;
		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(66) : float(235);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(744) : float(943);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(103) : float(223);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(763) : float(908);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(191) : float(198);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(764) : float(862);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(164) : float(228);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(784) : float(816);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(195) : float(195);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(777) : float(777);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(228) : float(164);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(816) : float(784);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(198) : float(191);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(862) : float(764);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(223) : float(103);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(908) : float(763);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(235) : float(66);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(943) : float(744);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(69) : float(245);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(742) : float(949);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(92) : float(219);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(758) : float(919);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(106) : float(197);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(766) : float(860);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(134) : float(248);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(761) : float(791);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(163) : float(269);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(785) : float(764);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(189) : float(311);
			UnitY = float(110);
			UnitZ = GetNation() == KARUS ? float(775) : float(763);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(268) : float(358);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(729) : float(777);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(281) : float(388);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(761) : float(782);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(329) : float(392);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(768) : float(811);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(370) : float(413);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(780) : float(847);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(425) : float(425);
			UnitY = float(107);
			UnitZ = GetNation() == KARUS ? float(840) : float(840);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(413) : float(370);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(847) : float(780);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(392) : float(329);
			UnitY = float(110);
			UnitZ = GetNation() == KARUS ? float(811) : float(768);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(388) : float(281);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(782) : float(761);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(358) : float(268);
			UnitY = float(123);
			UnitZ = GetNation() == KARUS ? float(777) : float(729);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(311) : float(189);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(763) : float(775);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(269) : float(163);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(764) : float(785);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(248) : float(134);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(791) : float(761);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(197) : float(106);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(860) : float(766);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(219) : float(92);
			UnitY = float(115);
			UnitZ = GetNation() == KARUS ? float(919) : float(758);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(245) : float(69);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(949) : float(742);
			break;
		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(73) : float(226);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(744) : float(918);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(90) : float(211);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(756) : float(888);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(106) : float(195);
			UnitY = float(102);
			UnitZ = GetNation() == KARUS ? float(765) : float(873);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(138) : float(231);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(762) : float(816);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(161) : float(251);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(781) : float(792);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(209) : float(267);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(775) : float(759);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(212) : float(279);
			UnitY = float(126);
			UnitZ = GetNation() == KARUS ? float(767) : float(737);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(279) : float(212);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(737) : float(767);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(267) : float(209);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(759) : float(775);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(251) : float(161);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(792) : float(781);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(231) : float(138);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(816) : float(762);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(195) : float(106);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(873) : float(765);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(211) : float(90);
			UnitY = float(116);
			UnitZ = GetNation() == KARUS ? float(888) : float(756);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(226) : float(73);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(918) : float(744);
			break;

		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(68) : float(231);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(743) : float(925);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(105) : float(212);
			UnitY = float(110);
			UnitZ = GetNation() == KARUS ? float(766) : float(900);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(138) : float(197);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(762) : float(857);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(164) : float(207);
			UnitY = float(108);
			UnitZ = GetNation() == KARUS ? float(786) : float(834);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(205) : float(255);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(770) : float(793);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(238) : float(267);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(743) : float(761);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(267) : float(238);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(761) : float(743);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(255) : float(205);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(793) : float(770);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(207) : float(164);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(834) : float(786);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(197) : float(138);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(857) : float(762);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(212) : float(105);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(900) : float(766);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(231) : float(68);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(925) : float(743);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(69) : float(242);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(743) : float(946);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(91) : float(221);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(757) : float(910);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(102) : float(201);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(767) : float(870);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(131) : float(197);
			UnitY = float(106);
			UnitZ = GetNation() == KARUS ? float(759) : float(878);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(162) : float(258);
			UnitY = float(124);
			UnitZ = GetNation() == KARUS ? float(782) : float(825);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(196) : float(209);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(778) : float(771);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(235) : float(224);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(790) : float(758);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(262) : float(253);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(774) : float(746);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(253) : float(262);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(746) : float(774);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(224) : float(235);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(758) : float(790);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(209) : float(196);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(771) : float(778);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(258) : float(162);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(825) : float(782);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(197) : float(131);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(878) : float(759);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(201) : float(102);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(870) : float(767);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(221) : float(91);
			UnitY = float(116);
			UnitZ = GetNation() == KARUS ? float(910) : float(757);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(242) : float(69);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(946) : float(743);
			break;
		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(66) : float(235);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(744) : float(943);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(103) : float(223);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(763) : float(908);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(191) : float(198);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(764) : float(862);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(164) : float(228);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(784) : float(816);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(195) : float(195);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(777) : float(777);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(228) : float(164);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(816) : float(784);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(198) : float(191);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(862) : float(764);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(223) : float(103);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(908) : float(763);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(235) : float(66);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(943) : float(744);
			break;

		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(69) : float(245);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(742) : float(949);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(92) : float(219);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(758) : float(919);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(106) : float(197);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(766) : float(860);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(134) : float(248);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(761) : float(791);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(163) : float(269);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(785) : float(764);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(189) : float(311);
			UnitY = float(110);
			UnitZ = GetNation() == KARUS ? float(775) : float(763);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(268) : float(358);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(729) : float(777);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(281) : float(388);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(761) : float(782);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(329) : float(392);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(768) : float(811);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(370) : float(413);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(780) : float(847);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(425) : float(425);
			UnitY = float(107);
			UnitZ = GetNation() == KARUS ? float(840) : float(840);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(413) : float(370);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(847) : float(780);
			break;
		case 13:
			UnitX = GetNation() == KARUS ? float(392) : float(329);
			UnitY = float(110);
			UnitZ = GetNation() == KARUS ? float(811) : float(768);
			break;
		case 14:
			UnitX = GetNation() == KARUS ? float(388) : float(281);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(782) : float(761);
			break;
		case 15:
			UnitX = GetNation() == KARUS ? float(358) : float(268);
			UnitY = float(123);
			UnitZ = GetNation() == KARUS ? float(777) : float(729);
			break;
		case 16:
			UnitX = GetNation() == KARUS ? float(311) : float(189);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(763) : float(775);
			break;
		case 17:
			UnitX = GetNation() == KARUS ? float(269) : float(163);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(764) : float(785);
			break;
		case 18:
			UnitX = GetNation() == KARUS ? float(248) : float(134);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(791) : float(761);
			break;
		case 19:
			UnitX = GetNation() == KARUS ? float(197) : float(106);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(860) : float(766);
			break;
		case 20:
			UnitX = GetNation() == KARUS ? float(219) : float(92);
			UnitY = float(115);
			UnitZ = GetNation() == KARUS ? float(919) : float(758);
			break;
		case 21:
			UnitX = GetNation() == KARUS ? float(245) : float(69);
			UnitY = float(109);
			UnitZ = GetNation() == KARUS ? float(949) : float(742);
			break;

		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}

void CBot::MoveProcessSpecialDelos()
{
	float UnitX = 0, UnitY = 0, UnitZ = 0;
	float sRange = 45.0f;

	switch (s_MoveProcess) {
	case 1:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(505) : float(504);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(284) : float(716);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(498) : float(501);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(327) : float(599);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(497) : float(500);
			UnitY = float(81);
			UnitZ = GetNation() == KARUS ? float(432) : float(517);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(499) : float(499);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(443) : float(443);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(500) : float(497);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(517) : float(432);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(501) : float(498);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(599) : float(327);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(504) : float(505);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(716) : float(284);
			break;
		}
		break;
	}
	case 2:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(464) : float(504);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(257) : float(706);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(424) : float(436);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(318) : float(616);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(421) : float(423);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(370) : float(500);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(423) : float(423);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(447) : float(447);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(423) : float(421);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(500) : float(370);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(436) : float(424);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(616) : float(318);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(504) : float(464);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(706) : float(257);
			break;
		}
		break;
	}
	case 3:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(553) : float(507);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(270) : float(712);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(604) : float(590);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(352) : float(582);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(592) : float(592);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(388) : float(388);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(590) : float(604);
			UnitY = float(81);
			UnitZ = GetNation() == KARUS ? float(582) : float(352);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(507) : float(553);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(712) : float(270);
			break;
		}
		break;
	}
	case 4:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(504) : float(491);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(285) : float(693);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(503) : float(472);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(330) : float(503);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(485) : float(465);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(358) : float(431);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(465) : float(485);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(431) : float(358);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(472) : float(503);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(503) : float(330);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(491) : float(504);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(693) : float(285);
			break;
		}
		break;
	}
	case 5:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(551) : float(511);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(264) : float(704);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(571) : float(515);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(322) : float(587);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(534) : float(513);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(370) : float(542);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(527) : float(514);
			UnitY = float(83);
			UnitZ = GetNation() == KARUS ? float(476) : float(532);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(514) : float(527);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(532) : float(476);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(513) : float(534);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(542) : float(370);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(515) : float(571);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(587) : float(322);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(511) : float(551);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(704) : float(264);
			break;
		}
		break;
	}
	case 6:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(505) : float(498);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(291) : float(706);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(476) : float(408);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(388) : float(547);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(433) : float(363);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(327) : float(493);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(343) : float(294);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(340) : float(385);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(292) : float(292);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(359) : float(359);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(294) : float(343);
			UnitY = float(81);
			UnitZ = GetNation() == KARUS ? float(385) : float(340);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(363) : float(433);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(493) : float(327);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(408) : float(476);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(547) : float(388);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(498) : float(505);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(706) : float(291);
			break;
		}
		break;
	}
	case 7:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(552) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(272) : float(694);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(614) : float(664);
			UnitY = float(81);
			UnitZ = GetNation() == KARUS ? float(348) : float(680);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(664) : float(723);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(412) : float(680);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(733) : float(774);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(424) : float(656);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(736) : float(798);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(482) : float(614);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(797) : float(784);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(506) : float(564);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(784) : float(797);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(564) : float(506);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(798) : float(736);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(614) : float(482);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(774) : float(733);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(656) : float(424);
			break;
		case 10:
			UnitX = GetNation() == KARUS ? float(723) : float(664);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(680) : float(412);
			break;
		case 11:
			UnitX = GetNation() == KARUS ? float(664) : float(614);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(680) : float(348);
			break;
		case 12:
			UnitX = GetNation() == KARUS ? float(506) : float(552);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(694) : float(272);
			break;
		}
		break;
	}
	case 8:
	{
		switch (m_MoveState) {

		case 1:
			UnitX = GetNation() == KARUS ? float(505) : float(504);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(290) : float(717);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(472) : float(501);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(392) : float(701);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(490) : float(490);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(405) : float(659);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(489) : float(483);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(479) : float(525);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(483) : float(489);
			UnitY = float(83);
			UnitZ = GetNation() == KARUS ? float(525) : float(479);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(490) : float(490);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(659) : float(405);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(501) : float(472);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(701) : float(392);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(504) : float(505);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(717) : float(290);
			break;

		}
		break;
	}
	case 9:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(504) : float(509);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(287) : float(699);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(506) : float(524);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(336) : float(532);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(533) : float(529);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(389) : float(450);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(529) : float(533);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(450) : float(389);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(524) : float(506);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(532) : float(336);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(509) : float(504);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(699) : float(287);
			break;
		}
		break;
	}
	case 10:
	{
		switch (m_MoveState) {
		case 1:
			UnitX = GetNation() == KARUS ? float(506) : float(504);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(286) : float(701);
			break;
		case 2:
			UnitX = GetNation() == KARUS ? float(477) : float(462);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(332) : float(659);
			break;
		case 3:
			UnitX = GetNation() == KARUS ? float(427) : float(444);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(432) : float(604);
			break;
		case 4:
			UnitX = GetNation() == KARUS ? float(429) : float(424);
			UnitY = float(80);
			UnitZ = GetNation() == KARUS ? float(450) : float(522);
			break;
		case 5:
			UnitX = GetNation() == KARUS ? float(423) : float(423);
			UnitY = float(83);
			UnitZ = GetNation() == KARUS ? float(481) : float(481);
			break;
		case 6:
			UnitX = GetNation() == KARUS ? float(424) : float(429);
			UnitY = float(83);
			UnitZ = GetNation() == KARUS ? float(522) : float(450);
			break;
		case 7:
			UnitX = GetNation() == KARUS ? float(444) : float(427);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(604) : float(432);
			break;
		case 8:
			UnitX = GetNation() == KARUS ? float(462) : float(477);
			UnitY = float(79);
			UnitZ = GetNation() == KARUS ? float(659) : float(332);
			break;
		case 9:
			UnitX = GetNation() == KARUS ? float(504) : float(506);
			UnitY = float(82);
			UnitZ = GetNation() == KARUS ? float(701) : float(286);
			break;
		}
		break;
	}
	}
	Unit* pUnit = g_pMain->GetUnitPtr(m_sTargetID, GetZoneID());
	if (pUnit != nullptr)
	{
		if (!pUnit->isDead())
		{
			float fDis = GetDistanceSqrt(pUnit);
			if (fDis < sRange)
			{
				WalkRegionCordinat(pUnit->GetID(), pUnit->GetX(), pUnit->GetY(), pUnit->GetZ(), 5, true);
				return;
			}
		}
	}

	if (UnitX == 0 && UnitY == 0 && UnitZ == 0)
		return;

	if (UnitX == 0 && UnitZ == 0)
		return;

	WalkCordinat(UnitX, UnitY, UnitZ, 5, true);
}