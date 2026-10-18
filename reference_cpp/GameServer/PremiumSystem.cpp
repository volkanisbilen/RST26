#include "stdafx.h"
#include "DBAgent.h"

void CUser::HandlePremium(Packet & pkt)
{
	uint8 opcode, pType;
	uint32 bResult = 0;
	pkt >> opcode >> pType;
	if (opcode != 4) return;

	Packet result(WIZ_PREMIUM, uint8(opcode));

	if (m_bPremiumInUse == pType) {
		result << int8(0) << int16(-1);
		Send(&result);
		return;
	}

	_PREMIUM_DATA * pPremium = m_PremiumMap.GetData(pType);
	if (pPremium == nullptr || pPremium->iPremiumTime < UNIXTIME) {
		result << int8(0) << int16(-1);
		Send(&result);
		return;
	}

	m_bPremiumInUse = pType;

	SendPremiumInfo();
	result.clear();
	g_pMain->AddDatabaseRequest(result, this);

	if (m_FlashExpBonus > 0 || m_FlashDcBonus > 0 || m_FlashWarBonus > 0) SetFlashTimeNote(true);
}

void CUser::SendPremiumInfo()
{
	Packet result(WIZ_PREMIUM, uint8(1));
	result << uint8(m_PremiumMap.GetSize());  // Premium count not expired
	uint8 counter = 0;
	foreach_stlmap(itr, m_PremiumMap)
	{
		_PREMIUM_DATA * uPrem = itr->second;
		if (uPrem == nullptr || uPrem->iPremiumTime == 0)
			continue;

		uint32 TimeRest = 0;
		uint16 TimeShow = 0;
		TimeRest = uint32(uPrem->iPremiumTime - UNIXTIME);
		if (TimeRest >= 1 && TimeRest <= 3600)
			TimeShow = 1;
		else
			TimeShow = TimeRest / 3600;

		result << uPrem->bPremiumType << TimeShow;
		counter++;

		if (m_bPremiumInUse == NO_PREMIUM)
			m_bPremiumInUse = uPrem->bPremiumType;
	}

	result << m_bPremiumInUse << uint32(0);
	Send(&result);
}

/**
* @brief Gives user the premium bonus items.
*/
void CUser::GivePremiumItem(uint8 bPremiumType)
{
	Packet result(WIZ_SHOPPING_MALL, uint8(STORE_LETTER));
	result << uint8(LetterOpcodes::LETTER_GIVE_PRE_ITEM) << bPremiumType;
	g_pMain->AddDatabaseRequest(result, this);
}

void CUser::SendClanPremium(CKnights* pknight, bool exits /*= false*/)
{
	m_bClanPremiumInUse = 0;
	sClanPremStatus = false;
	if (exits)
		return SendClanPremiumPkt(pknight, exits);

	if (!pknight || !pknight->isInPremium())
		return;

	m_bClanPremiumInUse = 13;
	sClanPremStatus = true;
	SendClanPremiumPkt(pknight, false);
	ClanPremiumBonusNotice(exits);
}

void CUser::SendClanPremiumPkt(CKnights* pknights, bool exits)
{
	Packet result(WIZ_PREMIUM, uint8(PremiumTypes::CLAN_PREMIUM));
	if (exits) result << uint8(0) << uint32(0) << uint16(0) << uint8(2);
	else result << uint8(4) << GetClanPremiumTime(pknights) << uint16(0) << uint8(2);
	Send(&result);
}

uint32 CUser::GetClanPremiumTime(CKnights* pknights)
{
	uint32 remtime = 0;
	if (pknights == nullptr) return remtime;
	return remtime = (pknights->sPremiumTime - (uint32)UNIXTIME) / 60;
}

uint16 CUser::GetPremiumProperty(PremiumPropertyOpCodes type)
{
	if (GetPremium() <= 0)
		return 0;

	_PREMIUM_ITEM * pPremiumItem = g_pMain->m_PremiumItemArray.GetData(GetPremium());
	if (pPremiumItem == nullptr)
		return 0;

	switch (type)
	{

	case PremiumPropertyOpCodes::PremiumNoahPercent:
		return pPremiumItem->NoahPercent;
	case PremiumPropertyOpCodes::PremiumDropPercent:
		return pPremiumItem->DropPercent;
	case PremiumPropertyOpCodes::PremiumBonusLoyalty:
		return pPremiumItem->BonusLoyalty;
	case PremiumPropertyOpCodes::PremiumRepairDiscountPercent:
		return pPremiumItem->RepairDiscountPercent;
	case PremiumPropertyOpCodes::PremiumItemSellPercent:
		return pPremiumItem->ItemSellPercent;
	case PremiumPropertyOpCodes::PremiumExpPercent:
		{
			foreach_stlmap(itr, g_pMain->m_PremiumItemExpArray)
			{
				_PREMIUM_ITEM_EXP *pPremiumItemExp = g_pMain->m_PremiumItemExpArray.GetData(itr->first);
				if (pPremiumItemExp == nullptr)
					continue;

				if (GetPremium() == pPremiumItemExp->Type
					&& GetLevel() >= pPremiumItemExp->MinLevel 
					&& GetLevel() <= pPremiumItemExp->MaxLevel)
					return pPremiumItemExp->sPercent;
			}
		}
		return 0;
	}
	return 0;
}

float CUser::GetPremiumPropertyExp(PremiumPropertyOpCodes type)
{
	if (GetPremium() <= 0)
		return 0.0f;

	_PREMIUM_ITEM * pPremiumItem = g_pMain->m_PremiumItemArray.GetData(GetPremium());
	if (pPremiumItem == nullptr)
		return 0.0f;

	switch (type)
	{
		case PremiumPropertyOpCodes::PremiumExpRestorePercent:
			return pPremiumItem->ExpRestorePercent;
	}
	return 0.0f;
}
/**
* @brief	Get premium properties.
*/
uint16 CUser::GetClanPremiumProperty(PremiumPropertyOpCodes type)
{
	if (GetClanPremium() <= 0)
		return 0;

	_PREMIUM_ITEM * pPremiumItem = g_pMain->m_PremiumItemArray.GetData(GetClanPremium());
	if (pPremiumItem == nullptr)
		return 0;

	switch (type)
	{
	case PremiumPropertyOpCodes::PremiumNoahPercent:
		return pPremiumItem->NoahPercent;
	case PremiumPropertyOpCodes::PremiumDropPercent:
		return pPremiumItem->DropPercent;
	case PremiumPropertyOpCodes::PremiumBonusLoyalty:
		return pPremiumItem->BonusLoyalty;
	case PremiumPropertyOpCodes::PremiumRepairDiscountPercent:
		return pPremiumItem->RepairDiscountPercent;
	case PremiumPropertyOpCodes::PremiumItemSellPercent:
		return pPremiumItem->ItemSellPercent;
	case PremiumPropertyOpCodes::PremiumExpPercent:
		{
			foreach_stlmap(itr, g_pMain->m_PremiumItemExpArray)
			{
				_PREMIUM_ITEM_EXP* pPremiumItemExp = g_pMain->m_PremiumItemExpArray.GetData(itr->first);
				if (pPremiumItemExp == nullptr)
					continue;

				if (GetClanPremium() == pPremiumItemExp->Type
					&& GetLevel() >= pPremiumItemExp->MinLevel
					&& GetLevel() <= pPremiumItemExp->MaxLevel)
					return pPremiumItemExp->sPercent;
			}
		}
		return 0;
	}
	return 0;
}

float CUser::GetClanPremiumPropertyExp(PremiumPropertyOpCodes type)
{
	if (GetPremium() <= 0)
		return 0.0f;

	_PREMIUM_ITEM * pPremiumItem = g_pMain->m_PremiumItemArray.GetData(GetPremium());
	if (pPremiumItem == nullptr)
		return 0.0f;

	switch (type)
	{
		case PremiumPropertyOpCodes::PremiumExpRestorePercent:
			return pPremiumItem->ExpRestorePercent;
	}
	return 0.0f;
}

void CUser::GivePremium(uint8 bPremiumType, uint16 sPremiumTime, bool minute)
{
	if (!bPremiumType || bPremiumType > 13 || sPremiumTime <= 0 || m_PremiumMap.GetSize() > PREMIUM_TOTAL)
		return;

	_PREMIUM_DATA * pPremium = m_PremiumMap.GetData(bPremiumType);
	if (pPremium == nullptr)
	{
		pPremium = new _PREMIUM_DATA;
		pPremium->bPremiumType = bPremiumType;

		if (minute)
			pPremium->iPremiumTime = uint32(UNIXTIME) + 60 * sPremiumTime;
		else
			pPremium->iPremiumTime = uint32(UNIXTIME) + 24 * 60 * 60 * sPremiumTime;

		m_PremiumMap.PutData(bPremiumType, pPremium);
	}
	else
	{
		pPremium->bPremiumType = bPremiumType;
		if (minute)
			pPremium->iPremiumTime = pPremium->iPremiumTime + 60 * sPremiumTime;
		else
			pPremium->iPremiumTime = pPremium->iPremiumTime + 24 * 60 * 60 * sPremiumTime;
	}

	m_bPremiumInUse = bPremiumType;
	m_bAccountStatus = 1;

	// JstKO Premium Notice Eklendi 01.01.2025
	if (bPremiumType == EXP_Premium)
		g_pMain->SendYesilNotice(this, "[Premium] : Tebrikler EXP Premium Aktif Edildi");
	else if (bPremiumType == DISC_Premium)
		g_pMain->SendYesilNotice(this, "[Premium] : Tebrikler DC Premium Aktif Edildi");
	else if (bPremiumType == WAR_Premium)
		g_pMain->SendYesilNotice(this, "[Premium] : Tebrikler WAR Premium Aktif Edildi");

	Packet pkt(WIZ_PREMIUM);
	g_pMain->AddDatabaseRequest(pkt, this);
	SendPremiumInfo();
	PremiumInsertLog(bPremiumType, sPremiumTime);
	//Disconnect(); //JstKO Premium Yukeyince Dc etme Kapatildi 17.12.2020

	//JstKO pre yükletince dc etmeden yenile karakteri işi garantiye aldık 
	CharacterOlduguYerdeYenile();
}

void CUser::GiveSwitchPremium(uint8 bPremiumType, uint16 sPremiumTime)
{
	if (bPremiumType <= 0
		|| bPremiumType > 13
		|| sPremiumTime <= 0
		|| m_PremiumMap.GetSize() > PREMIUM_TOTAL)
		return;

	_PREMIUM_DATA * pPremium = m_PremiumMap.GetData(bPremiumType);
	if (pPremium == nullptr)
	{
		pPremium = new _PREMIUM_DATA;
		pPremium->bPremiumType = bPremiumType;
		pPremium->iPremiumTime = uint32(UNIXTIME) + 24 * 60 * 60 * sPremiumTime;
		m_PremiumMap.PutData(bPremiumType, pPremium);
	}
	else
	{
		pPremium->bPremiumType = bPremiumType;
		pPremium->iPremiumTime = pPremium->iPremiumTime + 24 * 60 * 60 * sPremiumTime;
	}

	m_bPremiumInUse = bPremiumType;
	m_bAccountStatus = 1;

	// JstKO SWITCH Premium Notice Eklendi 01.01.2025
	if (bPremiumType == SWITCH_PREMIUM)
		g_pMain->SendYesilNotice(this, "[Premium] : Tebrikler SWITCH Premium Aktif Edildi");

	Packet pkt(WIZ_PREMIUM);
	g_pMain->AddDatabaseRequest(pkt, this);
	m_switchPremiumCount++;

	if (m_switchPremiumCount > 2)
		SendPremiumInfo();

	//JstKO pre yükletince dc etmeden yenile karakteri işi garantiye aldık 
	CharacterOlduguYerdeYenile();
}

void CUser::GiveClanPremium(uint8 bPremiumType, uint32 sPremiumDay)
{
	if (!isInClan() || !isClanLeader())
		return;

	CKnights* pKnights = g_pMain->GetClanPtr(GetClanID());
	if (pKnights == nullptr)
		return;

	int remtime = pKnights->sPremiumTime > (uint32)UNIXTIME ? pKnights->sPremiumTime - (uint32)UNIXTIME : 0;
	pKnights->sPremiumTime = ((uint32)UNIXTIME + 24 * 60 * 60 * sPremiumDay) + remtime;
	pKnights->sPremiumInUse = PremiumTypes::CLAN_PREMIUM;

	std::vector<std::string> mlist;
	pKnights->m_arKnightsUser.m_lock.lock();
	foreach_stlmap_nolock(i, pKnights->m_arKnightsUser) {
		_KNIGHTS_USER* p = i->second;
		if (p == nullptr) continue;

		mlist.push_back(p->strUserName);
	}
	pKnights->m_arKnightsUser.m_lock.unlock();

	foreach(itr, mlist)
	{
		CUser* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (pUser == nullptr || !pUser->isInGame())
			continue;

		pUser->SendClanPremium(pKnights);
	}

	// JstKO CLAN Premium Notice Eklendi 01.01.2025
	if (bPremiumType == CLAN_PREMIUM)
		g_pMain->SendYesilNotice(this, "[Premium] : Tebrikler CLAN Premium Aktif Edildi");

	Packet newpkt(WIZ_DB_SAVE_USER, uint8(ProcDbType::ClanPremium));
	g_pMain->AddDatabaseRequest(newpkt, this);

	//SendClanPremiumNotice();
}
