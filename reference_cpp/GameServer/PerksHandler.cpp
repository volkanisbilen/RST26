#include "StdAfx.h"

void CUser::Send_myPerks()
{
	Packet newpkt(XSafe, uint8(XSafeOpCodes::PERKS));
	newpkt << uint8(perksSub::info) << pPerks.remPerk << g_pMain->pServerSetting.m_perkCoins;

	g_pMain->m_PerksArray.m_lock.lock();
	newpkt << (uint16)g_pMain->m_PerksArray.GetSize(false);
	foreach_stlmap_nolock(itr, g_pMain->m_PerksArray)
	{
		auto* p = itr->second;
		if (!p) continue;
		newpkt.DByte();
		newpkt << p->pIndex << p->status << p->perkCount << p->maxPerkCount << p->strDescp << p->percentage;
	}
	g_pMain->m_PerksArray.m_lock.unlock();

	for (int i = 0; i < PERK_COUNT; i++)
		newpkt << pPerks.perkType[i];
	Send(&newpkt);
}

void CUser::HandlePerks(Packet& pkt)
{
	uint8 subcode = pkt.read<uint8>();
	switch ((perksSub)subcode)
	{
	case perksSub::perkPlus:
		PerkPlus(pkt);
		break;
	case perksSub::perkReset:
		PerkReset(pkt);
		break;
	case perksSub::perkTargetInfo:
		PerkTargetInfo(pkt);
		break;
	}
}

void CUser::SendPerkError(perksError error)
{
	Packet newpkt(XSafe, uint8(XSafeOpCodes::PERKS));
	newpkt << uint8(perksSub::perkPlus) << uint8(error);
	Send(&newpkt);
}

void CUser::PerkPlus(Packet& pkt)
{
	Packet newpkt(XSafe, uint8(XSafeOpCodes::PERKS));
	newpkt << uint8(perksSub::perkPlus);

	uint32 index = pkt.read<uint32>();
	if (index >= PERK_COUNT)
		return;

	if (!pPerks.remPerk)
		return SendPerkError(perksError::remPerks);

	auto* perks = g_pMain->m_PerksArray.GetData(index);
	if (!perks)
		return SendPerkError(perksError::notFound);

	if (pPerks.perkType[index] >= perks->maxPerkCount)
		return SendPerkError(perksError::maxPerkCount);

	pPerks.remPerk--;
	pPerks.perkType[index]++;
	newpkt << uint8(perksError::success) << index << pPerks.perkType[index] << pPerks.remPerk;
	Send(&newpkt);
	SetUserAbility(true);
}

void CUser::PerkReset(Packet& pkt)
{
	Packet newpkt(XSafe, uint8(XSafeOpCodes::PERKS));
	newpkt << uint8(perksSub::perkReset);
	uint32 point = 0;
	for (int i = 0; i < PERK_COUNT; i++)
		point += pPerks.perkType[i];

	if (point == 0)
		return SendBoxMessage(0, "", "There are no Perk points to reset.", 0, messagecolour::red);

	uint32 coins = g_pMain->pServerSetting.m_perkCoins;

	if (!GoldLose(coins, true))
		return SendBoxMessage(0, "", "You do not have enough money to reset the perk points.", 0, messagecolour::red);

	pPerks.remPerk += point;
	memset(pPerks.perkType, 0, sizeof(pPerks.perkType));
	newpkt << pPerks.remPerk;
	Send(&newpkt);
	SetUserAbility(true);
}

bool CUser::PerkUseItem(uint32 itemid, uint32 itemcount, uint16 perk)
{
	uint16 plus = perk;
	if (plus < 1) plus = 1;

	Packet newpkt(XSafe, uint8(XSafeOpCodes::PERKS));
	newpkt << uint8(perksSub::perkUseItem);

	if (itemid)
	{
		if (!CheckExistItem(itemid, itemcount)) {
			SendBoxMessage(0, "", "You also don't have perk item inventory. please check.", 0, messagecolour::red);
			return false;
		}
			
		if (!RobItem(itemid, itemcount, true)) {
			SendBoxMessage(0, "", "You also don't have perk item inventory. please check.", 0, messagecolour::red);
			return false;
		}
	}

	pPerks.remPerk += plus;
	newpkt << pPerks.remPerk;
	Send(&newpkt);
	return true;
}

void CUser::PerkTargetInfo(Packet& pkt)
{
	uint16 targetID = pkt.read<uint16>();
	if (!targetID)
		return;

	CUser* pUser = g_pMain->GetUserPtr(targetID);
	if (!pUser || !pUser->isInGame())
		return SendBoxMessage(0, "", "No player found or no such player.", 0, messagecolour::red);

	Packet newpkt(XSafe, uint8(XSafeOpCodes::PERKS));
	newpkt << uint8(perksSub::perkTargetInfo) << targetID;
	for (int i = 0; i < PERK_COUNT; i++)
		newpkt << pUser->pPerks.perkType[i];
	Send(&newpkt);
}

void CUser::SendBoxMessage(uint8 type, std::string title, std::string message, uint8 time, messagecolour a) {
	Packet newpkt(XSafe, (uint8)XSafeOpCodes::MESSAGE2);
	newpkt.DByte();

	newpkt << type;
	if (type == 0)
		newpkt << message << uint32(a);
	else
		newpkt << title << message << time;

	Send(&newpkt);
}