#include "stdafx.h"

#pragma region CUser::AddToRegion(int16 new_region_x, int16 new_region_z)
void CUser::AddToRegion(int16 new_region_x, int16 new_region_z)
{
	if (GetRegion())
		GetRegion()->Remove(this);

	SetRegion(new_region_x, new_region_z);

	if (GetRegion())
		GetRegion()->Add(this);
}
#pragma endregion

void CUser::GetInOut(Packet & result, uint8 bType)
{
	if (bType != InOutType::INOUT_OUT && isGM() && m_bAbnormalType == (uint32)AbnormalType::ABNORMAL_INVISIBLE) return;
	result.Initialize(WIZ_USER_INOUT);
	result << uint16(bType) << GetID();
	if (bType != INOUT_OUT) GetUserInfo(result);
}

void CUser::UserInOut(uint8 bType)
{
	if (GetRegion() == nullptr)
		return;

	Packet result;

	GetInOut(result, bType);
	if (bType == InOutType::INOUT_OUT) GetRegion()->Remove(this);
	else GetRegion()->Add(this);

	if (m_bAbnormalType != (uint32)AbnormalType::ABNORMAL_INVISIBLE) SendToRegion(&result, this, GetEventRoom());

	if (bType == INOUT_RESPAWN && !mytagname.empty() && mytagname != "-") {
		Packet newpkt(XSafe, (uint8)XSafeOpCodes::TagInfo);
		uint8 r = GetRValue(GetTagNameColour()), g = GetGValue(GetTagNameColour()), b = GetBValue(GetTagNameColour());
		newpkt << (uint8)tagerror::List << uint16(1) << GetSocketID() << mytagname << r << g << b;
		SendToRegion(&newpkt, this, GetEventRoom());
	}
}

void CUser::GmInOut(uint8 bType)
{
	if (GetRegion() == nullptr) return;
	ResetGMVisibility();
	Packet result;
	GmGetInOut(result, bType);
	SendToRegion(&result, this, GetEventRoom());
}

void CUser::GmGetInOut(Packet& result, uint8 bType)
{
	result.Initialize(WIZ_USER_INOUT);
	result << uint16(bType) << GetID();
	GetUserInfo(result);
}