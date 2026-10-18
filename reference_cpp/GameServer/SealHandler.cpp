#include "stdafx.h"
#include "DBAgent.h"

using std::string;

#define ITEM_SEAL_PRICE 1000000

enum class SealOpcodes
{
	ITEM_LOCK = 1,
	ITEM_UNLOCK = 2,
	ITEM_BOUND = 3,
	ITEM_UNBOUND = 4
};

void CUser::ItemSealProcess(Packet& pkt)
{
	uint8 opcode = pkt.read<uint8>();
	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_SEAL));
	result << opcode;

	if (!isInGame() || isTrading() || isMerchanting()
		|| isSellingMerchantingPreparing() || isBuyingMerchantingPreparing()
		|| isFishing() || isMining()) {
		result << uint8(SealErrorCodes::SealErrorFailed);
		Send(&result);
		return;
	}

	switch ((SealOpcodes)opcode)
	{
		// Used when sealing an item.
	case SealOpcodes::ITEM_LOCK:
	{
		uint8 bSrcPos = 0;
		string strPasswd; uint32 nItemID;  int16 unk0;
		pkt >> unk0 >> nItemID >> bSrcPos >> strPasswd;

		if (!nItemID)return;

		/*if (GetPremium() == 0) {
			result << uint8(SealErrorCodes::SealErrorPremiumOnly);
			Send(&result);
			return;
		}*/

		if (!CheckVipPassword(strPasswd)) {
			result << uint8(SealErrorCodes::SealErrorInvalidCode);
			Send(&result);
			return;
		}

		if (!hasCoins(ITEM_SEAL_PRICE)) {
			result << uint8(SealErrorCodes::SealErrorNeedCoins);
			Send(&result);
			return;
		}

		auto* pItem = GetItem(SLOT_MAX + bSrcPos);
		if (pItem == nullptr || bSrcPos >= HAVE_MAX
			|| pItem->nNum != nItemID || !pItem->sCount
			|| !pItem->nSerialNum || pItem->bFlag == (uint8)ItemFlag::ITEM_FLAG_SEALED
			|| pItem->bFlag == (uint8)ItemFlag::ITEM_FLAG_CHAR_SEAL
			|| pItem->isDuplicate() || pItem->isExpirationTime() || pItem->isRented()) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		auto pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || pTable.m_bCountable) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		result << nItemID << bSrcPos;
		g_pMain->AddDatabaseRequest(result, this);
	}
	break;
	// Used when unsealing an item.
	case SealOpcodes::ITEM_UNLOCK:
	{
		string strPasswd; uint32 nItemID;
		int16 unk0; uint8 bSrcPos;
		pkt >> unk0 >> nItemID >> bSrcPos >> strPasswd;

		if (!nItemID)return;

		/*if (GetPremium() == 0) {
			result << uint8(SealErrorCodes::SealErrorPremiumOnly);
			Send(&result);
			return;
		}*/

		if (!CheckVipPassword(strPasswd)) {
			result << uint8(SealErrorCodes::SealErrorInvalidCode);
			Send(&result);
			return;
		}

		auto* pItem = GetItem(SLOT_MAX + bSrcPos);
		if (pItem == nullptr || bSrcPos >= HAVE_MAX
			|| pItem->nNum != nItemID || !pItem->sCount || !pItem->nSerialNum
			|| pItem->bFlag != (uint8)ItemFlag::ITEM_FLAG_SEALED
			|| pItem->isDuplicate() || pItem->isExpirationTime() || pItem->isRented()) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		auto pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || pTable.m_bCountable) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		result << nItemID << bSrcPos;
		g_pMain->AddDatabaseRequest(result, this);
	}
	break;
	// Used when binding a Krowaz item (used to take it from not bound -> bound)
	case SealOpcodes::ITEM_BOUND:
	{
		uint32 nItemID;
		uint8 bSrcPos = 0, unk3;
		uint16 unk1; uint32 unk2;
		pkt >> unk1 >> nItemID >> bSrcPos >> unk3 >> unk2;

		if (!nItemID)return;

		auto* pItem = GetItem(SLOT_MAX + bSrcPos);
		if (pItem == nullptr || bSrcPos >= HAVE_MAX
			|| pItem->nNum != nItemID || !pItem->sCount
			|| pItem->isDuplicate() || pItem->bFlag == (uint8)SealOpcodes::ITEM_BOUND
			|| pItem->isRented() || !pItem->nSerialNum) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		auto pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || pTable.m_bCountable) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		result << nItemID << bSrcPos;
		g_pMain->AddDatabaseRequest(result, this);
	}
	break;
	// Used when binding a Old item (used to take it from bound -> not bound)
	case SealOpcodes::ITEM_UNBOUND:
	{
		string strPasswd; uint32 nItemID;
		int16 unk0; int8 bSrcPos = -1;
		pkt >> unk0 >> nItemID >> bSrcPos >> strPasswd;

		if (!nItemID) return;

		if (!CheckVipPassword(strPasswd)) {
			result << uint8(SealErrorCodes::SealErrorInvalidCode);
			Send(&result);
			return;
		}

		auto* pItem = GetItem(SLOT_MAX + bSrcPos);
		if (pItem == nullptr || bSrcPos >= HAVE_MAX
			|| pItem->nNum != nItemID || !pItem->sCount
			|| !pItem->nSerialNum || pItem->bFlag != (uint8)ItemFlag::ITEM_FLAG_BOUND
			|| pItem->isDuplicate() || pItem->isRented()) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		auto pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || pTable.m_bCountable) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		if (!CheckExistItem(810890000, pTable.m_Bound)) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		result << nItemID << bSrcPos << pTable.m_Bound;
		g_pMain->AddDatabaseRequest(result, this);
	}
	break;
	default:
		printf("ItemSealProcess unhandled opcode %d \n", opcode);
		break;
	}
}

void CUser::CharacterSealProcess(Packet& pkt)
{
	uint8 bOpcode = pkt.read<uint8>();

	switch (bOpcode)
	{
	case CharacterSealOpcodes::ShowList:
		CharacterSealShowList(pkt);
		break;
	case CharacterSealOpcodes::UseScroll:
		CharacterSealUseScroll(pkt);
		break;
	case CharacterSealOpcodes::UseRing:
		CharacterSealUseRing(pkt);
		break;
	case CharacterSealOpcodes::Preview:
		CharacterSealPreview(pkt);
		break;
	case CharacterSealOpcodes::AchieveList:
		CharacterSealAchieveList(pkt);
		break;
	case 0x05:
	{
		Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_CHARACTER_SEAL));
		result << uint8(0x05);
		Send(&result);
	}
	break;
	default:
		TRACE("Unhandled Character Seal Opcode: %X\n", bOpcode);
		printf("Unhandled Character Seal Opcode: %X\n", bOpcode);
		break;
	}
}

void CUser::CharacterSealShowList(Packet& pkt)
{
	if (!CheckExistItem(ITEM_SEAL_SCROLL) && !CheckExistItem(ITEM_CYPHER_RING))
		return;

	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_CHARACTER_SEAL));
	result << uint8(CharacterSealOpcodes::ShowList);
	g_pMain->AddDatabaseRequest(result, this);
}

void CUser::CharacterSealUseScroll(Packet& pkt)
{
	/*09 02 FFFF 12 98B9B02F 0400 52494E47 0400 31323334*/
	uint16 unknow; uint32 nItemID;
	string userName, strPasswd; uint8 bSrcPos = 0;
	pkt >> unknow >> bSrcPos >> nItemID >> userName >> strPasswd;

	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_CHARACTER_SEAL));
	result << uint8(CharacterSealOpcodes::UseScroll);

	if (userName.empty() ||
		STRCASECMP(userName.c_str(), GetName().c_str()) == 0) {
		result << uint16(0);
		Send(&result);
		return;
	}

	if (!CheckVipPassword(strPasswd)) {
		result << uint16(0);
		Send(&result);
		return;
	}

	_ITEM_DATA* pItem = GetItem(SLOT_MAX + bSrcPos);
	if (pItem == nullptr || bSrcPos >= HAVE_MAX
		|| pItem->bFlag != (uint8)ItemFlag::ITEM_FLAG_NONE
		|| pItem->nNum != ITEM_SEAL_SCROLL || nItemID != ITEM_SEAL_SCROLL
		|| pItem->sCount < 1 || pItem->isDuplicate()) {
		result << uint16(0);
		Send(&result);
		return;
	}

	result << bSrcPos << userName << strPasswd;
	g_pMain->AddDatabaseRequest(result, this);
}

void CUser::CharacterSealPreview(Packet& pkt)
{
	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_CHARACTER_SEAL));
	uint32 nUniqueID = 0;
	pkt >> nUniqueID;

	_CHARACTER_SEAL_ITEM_MAPPING* pDataMapping = g_pMain->m_CharacterSealItemMapping.GetData(nUniqueID);
	if (pDataMapping == nullptr)
	{
		result << uint8(CharacterSealOpcodes::Preview) << uint8(2);
		Send(&result);
		return;
	}

	/*bool isExist = false;
	for (int i = 0; i < INVENTORY_TOTAL; i++) {
		_ITEM_DATA *pItem = GetItem(i);
		if (pItem == nullptr) continue;

		if (pItem->nSerialNum == pDataMapping->nItemSerial) {
			isExist = true;
			break;
		}
	}

	if (isExist == false)
	{
		result << uint8(CharacterSealOpcodes::Preview) << uint8(2);
		Send(&result);
		return;
	}*/

	_CHARACTER_SEAL_ITEM* pData = g_pMain->m_CharacterSealItemArray.GetData(pDataMapping->nItemSerial);
	if (pData == nullptr)
	{
		result << uint8(CharacterSealOpcodes::Preview) << uint8(2);
		Send(&result);
		return;
	}

	result.SByte();
	result << uint8(CharacterSealOpcodes::Preview) << uint8(1)
		<< pData->GetName()
		<< uint8(1)
		<< pData->GetRace()
		<< pData->GetClass()
		<< pData->GetLevel()
		<< pData->m_iLoyalty
		<< pData->m_bStats[(uint8)StatType::STAT_STR]
		<< pData->m_bStats[(uint8)StatType::STAT_STA]
		<< pData->m_bStats[(uint8)StatType::STAT_DEX]
		<< pData->m_bStats[(uint8)StatType::STAT_INT]
		<< pData->m_bStats[(uint8)StatType::STAT_CHA]
		<< pData->m_iGold // Weird Sending this here hm
		<< pData->m_bstrSkill[(uint8)SkillPointCategory::SkillPointFree]
		<< uint32(1) // -> Reading Which Skill has how many pointd
		<< pData->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat1]
		<< pData->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat2]
		<< pData->m_bstrSkill[(uint8)SkillPointCategory::SkillPointCat3]
		<< pData->m_bstrSkill[(uint8)SkillPointCategory::SkillPointMaster];

	/*Time to Send the inventory Itemz and the Equipped itemz ??*/
	for (int i = 0; i < INVENTORY_COSP; i++)
	{
		_ITEM_DATA* pItem = pData->GetItem(i);
		if (pItem == nullptr)
			continue;

		result << pItem->nNum
			<< pItem->sDuration
			<< pItem->sCount
			<< pItem->bFlag;// item type flag (e.g. rented)
	}
	Send(&result);
}

void CUser::CharacterSealAchieveList(Packet& pkt)
{
	uint8 test;
	pkt >> test;
	uint32 sCount = 0;

	Packet result(WIZ_ITEM_UPGRADE, uint8(ITEM_CHARACTER_SEAL));
	result << uint8(CharacterSealOpcodes::AchieveList) << uint8(1);
	//size_t wpos = result.wpos();
	result << sCount;
	foreach_stlmap(itr, m_AchieveMap)
	{
		_USER_ACHIEVE_INFO* pAchieveData = itr->second;
		if (pAchieveData == nullptr
			|| (pAchieveData->bStatus != UserAchieveStatus::AchieveFinished
				&& pAchieveData->bStatus != UserAchieveStatus::AchieveCompleted))
			continue;

		_ACHIEVE_MAIN* pAchieveMain = g_pMain->m_AchieveMainArray.GetData(itr->first);
		if (pAchieveMain == nullptr
			|| pAchieveMain->TitleID == 0)
			continue;

		result << pAchieveMain->sIndex;
		sCount++;
	}
	result.put(4, sCount);
	Send(&result);
}

void CUser::CharacterSealUseRing(Packet& pkt)
{
	uint16 unknow;
	uint32 nItemID;
	uint8 bSrcPos = 0, bSelectedCharIndex;
	pkt >> unknow >> bSrcPos >> nItemID >> bSelectedCharIndex;

	if (bSrcPos >= HAVE_MAX
		|| nItemID != ITEM_CYPHER_RING
		|| GetItem(SLOT_MAX + bSrcPos) == nullptr
		|| GetItem(SLOT_MAX + bSrcPos)->bFlag != (uint8)ItemFlag::ITEM_FLAG_NONE
		|| GetItem(SLOT_MAX + bSrcPos)->nNum != ITEM_CYPHER_RING
		|| GetItem(SLOT_MAX + bSrcPos)->nNum != nItemID
		|| bSelectedCharIndex >= 40)
		return;

	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_CHARACTER_SEAL));
	result << uint8(CharacterSealOpcodes::UseRing) << bSrcPos << bSelectedCharIndex;
	g_pMain->AddDatabaseRequest(result, this);
}

#pragma region Item Upgrade Related Database Methods
void CUser::ReqSealItem(Packet& pkt)
{
	uint8 bSrcPos, bSealType, bSealResult = 3;
	uint32 nItemID; uint16 bound_count = 0;
	pkt >> bSealType >> nItemID >> bSrcPos;

	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_SEAL));
	result << bSealType;

	if ((SealOpcodes)bSealType == SealOpcodes::ITEM_UNBOUND)
		pkt >> bound_count;

	auto* pItem = GetItem(SLOT_MAX + bSrcPos);
	if (pItem == nullptr || pItem->nNum != nItemID) {
		result << uint8(SealErrorCodes::SealErrorFailed);
		Send(&result);
		return;
	}

	if ((SealOpcodes)bSealType == SealOpcodes::ITEM_LOCK
		&& !GoldLose(ITEM_SEAL_PRICE)) {
		result << bSealResult << nItemID << bSrcPos;
		Send(&result);
		return;
	}

	if ((SealOpcodes)bSealType == SealOpcodes::ITEM_UNBOUND
		&& !CheckExistItem(810890000, bound_count)) {
		result << bSealResult << nItemID << bSrcPos;
		Send(&result);
		return;
	}

	bSealResult = g_DBAgent.UpdateUserSealItem(pItem->nSerialNum, nItemID, bSealType, pItem->bFlag, this);
	if (bSealResult != 1) {
		result << bSealResult << nItemID << bSrcPos;
		Send(&result);
		return;
	}

	if ((SealOpcodes)bSealType == SealOpcodes::ITEM_LOCK) {
		pItem->oFlag = pItem->bFlag;
		pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_SEALED;
	}
	else if ((SealOpcodes)bSealType == SealOpcodes::ITEM_UNLOCK) {

		if (pItem->oFlag == 7 || pItem->oFlag == 8)
			pItem->bFlag = pItem->oFlag;
		else
			pItem->bFlag = 0;
	}
	else if ((SealOpcodes)bSealType == SealOpcodes::ITEM_BOUND)
		pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_BOUND;
	else if ((SealOpcodes)bSealType == SealOpcodes::ITEM_UNBOUND) {
		pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_NOT_BOUND;
		RobItem(810890000, bound_count);
	}

	result << bSealResult << nItemID << bSrcPos;
	Send(&result);

	return;

	switch ((SealOpcodes)bSealType)
	{
	case SealOpcodes::ITEM_LOCK:
	{
		if (!GoldLose(ITEM_SEAL_PRICE)) {
			result << bSealResult << nItemID << bSrcPos;
			Send(&result);
			return;
		}

		bSealResult = g_DBAgent.UpdateUserSealItem(pItem->nSerialNum, nItemID, bSealType, pItem->bFlag, this);
		if (bSealResult != 1) {
			result << bSealResult << nItemID << bSrcPos;
			Send(&result);
			return;
		}

		pItem->oFlag = pItem->bFlag;
		pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_SEALED;
		result << bSealResult << nItemID << bSrcPos;
		Send(&result);
	}
	break;
	case SealOpcodes::ITEM_UNLOCK:
	{
		bSealResult = g_DBAgent.UpdateUserSealItem(pItem->nSerialNum, nItemID, bSealType, pItem->bFlag, this);
		if (bSealResult != 1) {
			result << bSealResult << nItemID << bSrcPos;
			Send(&result);
			return;
		}

		pItem->bFlag = 0;
		result << bSealResult << nItemID << bSrcPos;
		Send(&result);
	}
	break;
	case SealOpcodes::ITEM_BOUND:
	{
		bSealResult = g_DBAgent.UpdateUserSealItem(pItem->nSerialNum, nItemID, bSealType, pItem->bFlag, this);
		if (bSealResult != 1) {
			result << bSealResult << nItemID << bSrcPos;
			Send(&result);
			return;
		}
		pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_BOUND;
		result << bSealResult << nItemID << bSrcPos;
		Send(&result);
	}
	break;
	case SealOpcodes::ITEM_UNBOUND:
	{
		bSealResult = g_DBAgent.UpdateUserSealItem(pItem->nSerialNum, nItemID, bSealType, pItem->bFlag, this);
		if (bSealResult != 1) {
			result << bSealResult << nItemID << bSrcPos;
			Send(&result);
			return;
		}

		auto pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull()) {
			result << uint8(SealErrorCodes::SealErrorFailed);
			Send(&result);
			return;
		}

		if (!RobItem(810890000, pTable.m_Bound)) {
			result << bSealResult << nItemID << bSrcPos;
			Send(&result);
			return;
		}

		pItem->bFlag = (uint8)ItemFlag::ITEM_FLAG_NOT_BOUND;
		result << bSealResult << nItemID << bSrcPos;
		Send(&result);
	}
	break;
	default:
		printf("ReqSealItem db req unhandled opcode %d \n", bSealType);
		break;
	}
}

void CUser::ReqCharacterSealProcess(Packet& pkt)
{
	if (isTrading()
		|| isMining()
		|| isFishing()
		|| isSellingMerchantingPreparing()
		|| isBuyingMerchantingPreparing()
		|| isMerchanting()
		|| !isInGame())
		return;

	uint8 bOpcode = pkt.read<uint8>();

	switch (CharacterSealOpcodes(bOpcode))
	{
	case ShowList:
		ReqCharacterSealShowList(pkt);
		break;
	case UseScroll:
		ReqCharacterSealUseScroll(pkt);
		break;
	case UseRing:
		ReqCharacterSealUseRing(pkt);
		break;
	case Preview:
		break;
	default:
		printf("Character Seal Unhandled packet %d \n", bOpcode);
		TRACE("Character Seal Unhandled packet %d \n", bOpcode);
		break;
	}
}

void CUser::ReqCharacterSealShowList(Packet& pkt)
{
	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_CHARACTER_SEAL));

	if ((!CheckExistItem(ITEM_SEAL_SCROLL) && !CheckExistItem(ITEM_CYPHER_RING))
		|| GetAccountName().length() == 0)
	{
		result << uint8(CharacterSealOpcodes::UseScroll) << uint16(0);
		Send(&result);
		return;
	}

	result << uint8(CharacterSealOpcodes::ShowList) << uint8(1);
	string strCharID1, strCharID2, strCharID3, strCharID4;

	g_DBAgent.GetAllCharID(m_strAccountID, strCharID1, strCharID2, strCharID3, strCharID4);
	g_DBAgent.LoadCharInfo(strCharID1, result);
	g_DBAgent.LoadCharInfo(strCharID2, result);
	g_DBAgent.LoadCharInfo(strCharID3, result);
	g_DBAgent.LoadCharInfo(strCharID4, result);
	Send(&result);
}

void CUser::ReqCharacterSealUseScroll(Packet& pkt)
{
	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_CHARACTER_SEAL));
	result << uint8(CharacterSealOpcodes::UseScroll);

	if (!CheckExistItem(ITEM_SEAL_SCROLL) || GetAccountName().length() == 0) {
		result << uint16(0);
		Send(&result);
		return;
	}

	uint8 bSrcPos = 0;
	string strRequestedChar, strPassword, strCharID[4];
	pkt >> bSrcPos >> strRequestedChar >> strPassword;

	/*int8 ItemCheckResult = g_DBAgent.CharacterSealItemCheck(strRequestedChar);
	if (ItemCheckResult != 1) {
		result << uint16(0);
		Send(&result);
		return;
	}*/

	if (STRCASECMP(strRequestedChar.c_str(), GetName().c_str()) == 0) {
		result << uint16(0);
		Send(&result);
		return;
	}

	g_DBAgent.GetAllCharID(m_strAccountID, strCharID[0], strCharID[1], strCharID[2], strCharID[3]);

	uint8 bCharRequested = 5;
	if (STRCASECMP(strRequestedChar.c_str(), strCharID[0].c_str()) == 0) bCharRequested = 0;
	else if (STRCASECMP(strRequestedChar.c_str(), strCharID[1].c_str()) == 0) bCharRequested = 1;
	else if (STRCASECMP(strRequestedChar.c_str(), strCharID[2].c_str()) == 0) bCharRequested = 2;
	else if (STRCASECMP(strRequestedChar.c_str(), strCharID[3].c_str()) == 0) bCharRequested = 3;

	if (bCharRequested == 5) {
		result << uint16(0);
		Send(&result);
		return;
	}

	_ITEM_DATA* pSrcItem = GetItem(SLOT_MAX + bSrcPos);
	if (pSrcItem == nullptr
		|| pSrcItem->bFlag != (uint8)ItemFlag::ITEM_FLAG_NONE
		|| pSrcItem->nNum != ITEM_SEAL_SCROLL
		|| pSrcItem->isRented()
		|| pSrcItem->isDuplicate()) {
		result << uint16(0);
		Send(&result);
		return;
	}

	uint64 nItemSerial = pSrcItem->nSerialNum;

	int8 bResult = g_DBAgent.CharacterSealProcess(GetAccountName(), strCharID[bCharRequested], strPassword, nItemSerial);
	if (bResult != 1) {
		result << uint16(bResult) << pSrcItem->nNum << bSrcPos;
		Send(&result);
		return;
	}

	_CHARACTER_SEAL_ITEM* pData = new _CHARACTER_SEAL_ITEM;
	pData->nSerialNum = nItemSerial;
	pData->m_strUserID = strCharID[bCharRequested];
	pData->strAccountID = GetAccountName();

	if (!g_DBAgent.LoadCharacterSealUserData(pData->strAccountID, pData->m_strUserID, pData)) {
		delete pData;
		result << uint16(0);
		Send(&result);
		return;
	}

	if (!g_pMain->m_CharacterSealItemArray.PutData(pData->nSerialNum, pData)) {
		delete pData;
		result << uint16(0);
		Send(&result);
		return;
	}

	_CHARACTER_SEAL_ITEM_MAPPING* pDataMap = new _CHARACTER_SEAL_ITEM_MAPPING;
	pDataMap->nItemSerial = nItemSerial;
	pDataMap->nUniqueID = pData->nUniqueID;

	if (!g_pMain->m_CharacterSealItemMapping.PutData(pData->nUniqueID, pDataMap)) {
		delete pData;
		delete pDataMap;
		result << uint16(0);
		Send(&result);
		return;
	}

	pSrcItem->nNum = ITEM_CYPHER_RING;
	SetUserAbility(false);
	SendItemWeight();

	result << uint8(1) << uint8(bSrcPos) << uint32(ITEM_CYPHER_RING)
		<< uint32(pData->nUniqueID)
		<< pData->GetName()
		<< uint8(pData->GetClass())
		<< pData->GetLevel()
		<< uint16(0)
		<< pData->GetRace() << uint8(0);
	Send(&result);
}

void CUser::ReqCharacterSealUseRing(Packet& pkt)
{
	uint8 bSrcPos = 0, m_charindex;
	pkt >> bSrcPos >> m_charindex;

	Packet result(WIZ_ITEM_UPGRADE, uint8(ItemUpgradeOpcodes::ITEM_CHARACTER_SEAL));
	result << uint8(CharacterSealOpcodes::UseRing);

	if (bSrcPos >= HAVE_MAX || m_charindex >= 4 || !CheckExistItem(ITEM_CYPHER_RING)) {
		result << uint16(0);
		Send(&result);
		return;
	}

	string strCharID[4];
	for (int i = 0; i < 4; i++) strCharID[i] = "";

	_ITEM_DATA* pSrcItem = GetItem(SLOT_MAX + bSrcPos);
	if (pSrcItem == nullptr || pSrcItem->bFlag != (uint8)ItemFlag::ITEM_FLAG_NONE
		|| pSrcItem->nNum != ITEM_CYPHER_RING || pSrcItem->isRented()
		|| pSrcItem->isDuplicate()) {
		result << uint16(0);
		Send(&result);
		return;
	}

	uint64 nItemSerial = pSrcItem->nSerialNum;
	_CHARACTER_SEAL_ITEM* pData = g_pMain->m_CharacterSealItemArray.GetData(nItemSerial);
	if (pData == nullptr)
		return;

	if ((pData->GetClass() / 100) != GetNation()) {
		result << uint16(0);
		Send(&result);
		return;
	}

	if (!g_DBAgent.GetAllCharID(m_strAccountID, strCharID[0], strCharID[1], strCharID[2], strCharID[3])) {
		result << uint16(0);
		Send(&result);
		return;
	}

	if (strCharID[m_charindex].length() != 0) {
		result << uint16(3);
		Send(&result);
		return;
	}

	uint32 nItemUniqueID = pData->nUniqueID;
	int8 bResult = g_DBAgent.CharacterUnSealProcess(GetAccountName(), m_charindex, nItemSerial);
	if (bResult != 1) {
		result << uint8(0) << bSrcPos << pSrcItem->nNum;
		Send(&result);
		return;
	}

	g_pMain->m_CharacterSealItemArray.DeleteData(nItemSerial);
	g_pMain->m_CharacterSealItemMapping.DeleteData(nItemUniqueID);

	memset(pSrcItem, 0, sizeof(_ITEM_DATA));
	SetUserAbility(false);
	SendItemWeight();

	result << uint8(bResult) << bSrcPos << uint32(0);
	Send(&result);
}
#pragma endregion

void CUser::ShowCyperRingItemInfo(Packet& pkt, uint64 nSerialNum)
{
	_CHARACTER_SEAL_ITEM* pCharSealItem = g_pMain->m_CharacterSealItemArray.GetData(nSerialNum);
	if (pCharSealItem != nullptr)
	{
		int64 ExpRate = 0;
		ExpRate = ((pCharSealItem->m_iExp * 50) / (g_pMain->GetExpByLevel(pCharSealItem->GetLevel(), GetRebirthLevel())) * 100);

		if (ExpRate > 10000)
			ExpRate = 10000;

		if (ExpRate < 0)
			ExpRate = 0;

		pkt.DByte();
		pkt << uint32(pCharSealItem->nUniqueID)
			<< pCharSealItem->GetName()
			<< uint8(pCharSealItem->GetClass())
			<< pCharSealItem->GetLevel()
			<< uint16(ExpRate)
			<< pCharSealItem->GetRace() << uint8(0) << uint8(0);
	}
	else
		pkt << uint32(0);
}

bool CUser::CheckVipPassword(std::string password)
{
	if (password.empty() || password.length() != 8) return false;
	if (pUserTbInfo.sealpass.empty() || pUserTbInfo.sealpass.length() != 8) return false;
	return password == pUserTbInfo.sealpass;
}