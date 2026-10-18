#include "stdafx.h"
#include "DBAgent.h"
/**
* @brief	Handles requests related to VIP Storage.
*/
void CUser::ReqVipStorageProcess(Packet& pkt)
{
	Packet result(WIZ_VIPWAREHOUSE);
	uint8 bOpcode = pkt.read<uint8>();

	if (isDead() || !isInGame())
	{
		result << bOpcode << uint8(2);
		Send(&result);
		return;
	}

	switch (bOpcode)
	{
	case VIP_UseVault: // Use Vault Item
	{
		uint16 sNpcId;
		uint32 nItemID;
		uint8 page, bSrcPos, bDstPos;
		_ITEM_TABLE pTable = _ITEM_TABLE();
		_ITEM_DATA* pSrcItem = nullptr;
		CNpc* pNpc = nullptr;
		uint32 Days;

		pkt >> sNpcId >> nItemID >> page >> bSrcPos >> bDstPos;
		pNpc = g_pMain->GetNpcPtr(sNpcId, GetZoneID());
		if (pNpc == nullptr
			|| pNpc->GetType() != NPC_WAREHOUSE
			|| !isInRange(pNpc, MAX_NPC_RANGE))
		{
			result << uint8(VIP_UseVault) << uint8(2);
			Send(&result);
			return;
		}

		if (nItemID != VIP_VAULT_KEY
			&& nItemID != VIP_SAFE_KEY_1
			&& nItemID != VIP_SAFE_KEY_7)
		{
			result << uint8(VIP_UseVault) << uint8(2);
			Send(&result);
			return;
		}

		pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull())
		{
			result << uint8(VIP_UseVault) << uint8(2);
			Send(&result);
			return;
		}

		pSrcItem = GetItem(SLOT_MAX + bSrcPos);
		if (pSrcItem == nullptr)
		{
			result << uint8(VIP_UseVault) << uint8(2);
			Send(&result);
			return;
		}

		if (nItemID == VIP_VAULT_KEY)
			Days = 30;
		else if (nItemID == VIP_SAFE_KEY_1)
			Days = 1;
		else if (nItemID == VIP_SAFE_KEY_7)
			Days = 7;
		else
		{
			result << uint8(VIP_UseVault) << uint8(2);
			Send(&result);
			return;
		}

		// Check for invalid slot IDs.
		if (bSrcPos > HAVE_MAX
			// Cannot be traded, sold or stored (note: don't check the race, as these items CAN be stored).
			|| (nItemID >= ITEM_NO_TRADE_MIN && nItemID <= ITEM_NO_TRADE_MAX)
			// Check that the source item we're moving is what the client says it is.
			|| pSrcItem->nNum != nItemID
			// Rented items cannot be placed in the inn.
			|| pSrcItem->isRented()
			|| pSrcItem->isDuplicate())
		{
			result << uint8(VIP_UseVault) << uint8(2);
			Send(&result);
			return;
		}

		uint32 iexpiration = uint32(UNIXTIME + 60 * 60 * 24 * Days);
		uint8 bSealResult = g_DBAgent.VIPStorageUseVaultKey(GetAccountName(), iexpiration);
		if (bSealResult != 1)
		{
			result << uint8(VIP_UseVault) << uint8(2);
			Send(&result);
			return;
		}

		memset(pSrcItem, 0, sizeof(_ITEM_DATA));

		SetUserAbility(false);
		SendItemWeight();
		m_bVIPStorageVaultExpiration = iexpiration;
		result << uint8(VIP_UseVault) << uint8(1) << uint8(1) << m_bVIPStorageVaultExpiration;
		Send(&result);
	}
	break;
	case VIP_SetPassword: // Set Password
	{
		std::string strSealPasswd;
		pkt >> strSealPasswd;

		uint8 bSealResult = g_DBAgent.VIPStorageSetPassword(strSealPasswd, GetAccountName(), 1);
		if (bSealResult != 1)
		{
			result << uint8(VIP_SetPassword) << uint8(2);
			Send(&result);
			return;
		}

		m_bVIPStoragePassword = strSealPasswd;
		m_bWIPStorePassowrdRequest = 1;
		result << uint8(VIP_SetPassword) << bSealResult;
		Send(&result);
	}
	break;
	case VIP_CancelPassword: // cancel password
	{
		uint8 bCancelResult = g_DBAgent.VIPStorageCancelPassword(GetAccountName(), 0);
		if (bCancelResult != 1)
		{
			result << uint8(VIP_CancelPassword) << uint8(2);
			Send(&result);
			return;
		}

		m_bWIPStorePassowrdRequest = 0;
		result << uint8(VIP_CancelPassword) << bCancelResult;
		Send(&result);
	}
	break;
	case VIP_ChangePassword: // Change Password
	{
		std::string strSealPasswd;
		pkt >> strSealPasswd;
		//yeni db komut yazýlacak.
		uint8 bSealResult = g_DBAgent.VIPStorageSetPassword(strSealPasswd, GetAccountName(), m_bWIPStorePassowrdRequest);

		if (bSealResult != 1)
		{
			result << uint8(VIP_ChangePassword) << uint8(2);
			Send(&result);
			return;
		}

		m_bVIPStoragePassword = strSealPasswd;
		result << uint8(VIP_ChangePassword) << bSealResult;
		Send(&result);
	}
	break;
	}
}

void CUser::UseVipKeyError(uint8 subcode, uint8 errorid)
{
	Packet result(WIZ_VIPWAREHOUSE, subcode);
	result << errorid;
	Send(&result);
}

#pragma region CUser::VIPhouseProcess(Packet & pkt)
void CUser::VIPhouseProcess(Packet& pkt)
{
	if (!isInGame() || m_bIsLoggingOut)
		return;

	Packet result(WIZ_VIPWAREHOUSE);
	uint8 opcode = pkt.read<uint8>();



	if (pCindWar.isEventUser() && g_pMain->isCindirellaZone(GetZoneID()))
		return UseVipKeyError(opcode, 2);

	if (isDead() || isTrading()
		|| isMining() || isMerchanting()
		|| isSellingMerchant() || isBuyingMerchant()
		|| isBuyingMerchantingPreparing() || isSellingMerchantingPreparing()
		|| isFishing())
		return UseVipKeyError(opcode, 2);

	if ((m_bVIPStorageVaultExpiration == 0
		|| m_bVIPStorageVaultExpiration < UNIXTIME)
		&& (opcode == VIP_InvenToStorage
			|| opcode == VIP_StorageToStore))
	{
		result << opcode << uint8(2);
		Send(&result);
		return;
	}

	if (opcode == VIP_Open) // VIP Storage Open
	{
		uint16 npcid;
		pkt >> npcid;

		std::string password;
		pkt.SByte();
		pkt >> password;

		bool isKey = false;
		if (m_bVIPStorageVaultExpiration > (uint32)UNIXTIME)
			isKey = true;
		else
			m_bVIPStorageVaultExpiration = 0;

		if (!npcid && m_bVIPStorageVaultExpiration < (uint32)UNIXTIME)
			return UseVipKeyError(opcode, 21);

		auto* pNpc = g_pMain->GetNpcPtr(npcid, GetZoneID());
		if (!pNpc && m_bVIPStorageVaultExpiration < (uint32)UNIXTIME)
			return UseVipKeyError(opcode, 21);

		if (pNpc && (pNpc->GetType() != NPC_WAREHOUSE || !isInRange(pNpc, MAX_NPC_RANGE)))
			return UseVipKeyError(opcode, 0);

		if (m_bWIPStorePassowrdRequest && !m_bVIPStoragePassword.empty())
			return UseVipKeyError(11, 1);

		result << opcode << uint8(1) << uint8(m_bWIPStorePassowrdRequest)
			<< uint8(isKey) << uint32(m_bVIPStorageVaultExpiration)
			<< uint32(VIP_VAULT_KEY) << uint16(1) << uint16(1) << uint8(0) << uint32(0) << uint16(0);

		for (int i = 0; i < VIPWAREHOUSE_MAX; i++) {
			_ITEM_DATA* pItem = &m_sVIPWarehouseArray[i];
			if (pItem == nullptr) continue;

			result << pItem->nNum << pItem->sDuration << pItem->sCount << pItem->bFlag;
			_ITEM_TABLE pItemTable = g_pMain->GetItemPtr(pItem->nNum);
			if (!pItemTable.isnull()) {
				if (pItemTable.isPetItem())
					ShowPetItemInfo(result, pItem->nSerialNum);
				else if (pItemTable.Getnum() == ITEM_CYPHER_RING)
					ShowCyperRingItemInfo(result, pItem->nSerialNum);
				else result << uint32(0); // Item Unique ID
			}
			else result << uint32(0); // Item Unique ID*/
			result << pItem->nExpirationTime;
		}
		Send(&result);
	}
	else if (opcode == VIP_InvenToStorage) // Inven to VIP Storage
	{
		if (m_bVIPStorageVaultExpiration < (uint32)UNIXTIME)
			return UseVipKeyError(opcode, 2);

		uint16 sNpcId, nCount; uint32 nItemID;
		uint8 page, bSrcPos, bDstPos;
		pkt >> sNpcId >> nItemID >> page >> bSrcPos >> bDstPos >> nCount;

		uint16 reference_pos = 48 * page;
		if (page > 1 || !nCount)
			return UseVipKeyError(opcode, 2);

		if (nItemID == ITEM_GOLD
			|| (nItemID >= ITEM_NO_TRADE_MIN && nItemID <= ITEM_NO_TRADE_MAX))
			return UseVipKeyError(opcode, 2);

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull() || pTable.m_bCountable == 2)
			return UseVipKeyError(opcode, 2);

		_ITEM_DATA* pSrcItem = GetItem(SLOT_MAX + bSrcPos);
		if (pSrcItem == nullptr || pSrcItem->nNum != nItemID
			|| pSrcItem->isRented() || pSrcItem->isDuplicate())
			return UseVipKeyError(opcode, 2);

		if (bSrcPos >= HAVE_MAX || reference_pos + bDstPos > VIPWAREHOUSE_MAX)
			return UseVipKeyError(opcode, 2);

		auto* pDstItem = &m_sVIPWarehouseArray[bDstPos];

		if (pDstItem->nNum != 0 && pTable.m_bKind == 255 && !pTable.m_bCountable)
			return UseVipKeyError(opcode, 2);

		if ((!pTable.isStackable() && pDstItem->nNum != 0)
			|| (pTable.isStackable() && pDstItem->nNum != 0 && pDstItem->nNum != pSrcItem->nNum)
			|| pSrcItem->sCount < nCount)
			return UseVipKeyError(opcode, 2);

		if (!pDstItem->sCount || pDstItem->nNum == 0)
			memset(pDstItem, 0, sizeof(_ITEM_DATA));

		if (pTable.isStackable())
			pDstItem->sCount += (uint16)nCount;
		else
			pDstItem->sCount = (uint16)nCount;

		if (pTable.isStackable())
			pSrcItem->sCount -= (uint16)nCount;
		else
			pSrcItem->sCount = 0;

		uint64 serial = pSrcItem->nSerialNum;
		if (!serial) serial = g_pMain->GenerateItemSerial();

		if (pTable.isStackable())
		{
			if (!pSrcItem->sCount && !pDstItem->nNum)
				pDstItem->nSerialNum = serial;
			else if (!pDstItem->nNum)
				pDstItem->nSerialNum = g_pMain->GenerateItemSerial();
		}
		else pDstItem->nSerialNum = serial;

		pDstItem->sDuration = pSrcItem->sDuration;
		pDstItem->bFlag = pSrcItem->bFlag;
		pDstItem->sRemainingRentalTime = pSrcItem->sRemainingRentalTime;
		pDstItem->nExpirationTime = pSrcItem->nExpirationTime;
		pDstItem->nNum = pSrcItem->nNum;

		if (pDstItem->sCount > MAX_ITEM_COUNT)
			pDstItem->sCount = MAX_ITEM_COUNT;

		if (!pSrcItem->sCount || (pTable.GetKind() == 255 && !pTable.m_bCountable))
			memset(pSrcItem, 0, sizeof(_ITEM_DATA));

		SetUserAbility(false);
		SendItemMove(1, 1);
		SendItemWeight();

		result << opcode << uint8(1);
		Send(&result);
	}
	else if (opcode == VIP_StorageToInven)
	{
		uint16 sNpcId, nCount; 	uint32 nItemID;
		uint8 page, bSrcPos, bDstPos;

		pkt >> sNpcId >> nItemID >> page >> bSrcPos >> bDstPos >> nCount;
		if (page > 1 || !nCount) return UseVipKeyError(opcode, 2);
		if (nItemID == ITEM_GOLD) return UseVipKeyError(opcode, 1);

		uint16 reference_pos = 48 * page;
		if (reference_pos + bSrcPos > VIPWAREHOUSE_MAX)
			return UseVipKeyError(opcode, 2);

		_ITEM_TABLE pTable = g_pMain->GetItemPtr(nItemID);
		if (pTable.isnull()) return UseVipKeyError(opcode, 2);

		_ITEM_DATA* pSrcItem = &m_sVIPWarehouseArray[reference_pos + bSrcPos];
		if (pSrcItem == nullptr
			|| bDstPos > HAVE_MAX || pSrcItem->nNum != nItemID
			|| !CheckWeight(pTable, nItemID, (uint16)nCount))
			return UseVipKeyError(opcode, 2);

		_ITEM_DATA* pDstItem = GetItem(SLOT_MAX + bDstPos);
		if (pDstItem == nullptr)
			return UseVipKeyError(opcode, 2);

		if ((!pTable.isStackable() && pDstItem->nNum != 0)
			|| (pTable.isStackable() && pDstItem->nNum != 0 && pDstItem->nNum != pSrcItem->nNum)
			|| pSrcItem->sCount < nCount)
			return UseVipKeyError(opcode, 2);

		if (!pSrcItem->sCount || pSrcItem->nNum == 0)
			memset(pSrcItem, 0, sizeof(_ITEM_DATA));

		if (pTable.isStackable())
			pDstItem->sCount += nCount;
		else
			pDstItem->sCount = nCount;

		if (pTable.isStackable())
			pSrcItem->sCount -= nCount;
		else
			pSrcItem->sCount = 0;

		uint64 serial = pSrcItem->nSerialNum;
		if (!serial) serial = g_pMain->GenerateItemSerial();

		if (pTable.isStackable())
		{
			if (!pSrcItem->sCount && !pDstItem->nNum)
				pDstItem->nSerialNum = serial;
			else if (!pDstItem->nNum)
				pDstItem->nSerialNum = g_pMain->GenerateItemSerial();
		}
		else pDstItem->nSerialNum = serial;

		pDstItem->sDuration = pSrcItem->sDuration;
		pDstItem->bFlag = pSrcItem->bFlag;
		pDstItem->sRemainingRentalTime = pSrcItem->sRemainingRentalTime;
		pDstItem->nExpirationTime = pSrcItem->nExpirationTime;
		pDstItem->nNum = pSrcItem->nNum;

		if (pDstItem->sCount > MAX_ITEM_COUNT)
			pDstItem->sCount = MAX_ITEM_COUNT;

		if (!pSrcItem->sCount || (pTable.GetKind() == 255 && !pTable.m_bCountable))
			memset(pSrcItem, 0, sizeof(_ITEM_DATA));

		SetUserAbility(false);
		SendItemMove(1, 1);
		SendItemWeight();

		result << opcode << uint8(1);
		Send(&result);
	}
	else if (opcode == VIP_StorageToStore) // VIP Storage to VIP Storage
	{
		if (m_bVIPStorageVaultExpiration < (uint32)UNIXTIME)
			return UseVipKeyError(opcode, 2);

		uint16 sNpcId, nCount; uint32 nItemID;
		uint8 page, bSrcPos, bDstPos;
		pkt >> sNpcId >> nItemID >> page >> bSrcPos >> bDstPos >> nCount;

		uint16 reference_pos = 48 * page;
		if (page > 1 || bSrcPos > reference_pos + VIPWAREHOUSE_MAX
			|| bDstPos > reference_pos + VIPWAREHOUSE_MAX)
			return UseVipKeyError(opcode, 2);

		_ITEM_DATA* pSrcItem = &m_sVIPWarehouseArray[bSrcPos];
		_ITEM_DATA* pDstItem = &m_sVIPWarehouseArray[bDstPos];
		if (pSrcItem == nullptr || pDstItem == nullptr
			|| pSrcItem->nNum != nItemID || pDstItem->nNum != 0)
			return UseVipKeyError(opcode, 2);

		memcpy(pDstItem, pSrcItem, sizeof(_ITEM_DATA));
		memset(pSrcItem, 0, sizeof(_ITEM_DATA));
		result << opcode << uint8(1);
		Send(&result);
	}
	else if (opcode == VIP_InvenToInven) // Inven to Inven Storage
	{
		uint16 sNpcId, nCount; uint32 nItemID;
		uint8 page, bSrcPos, bDstPos;
		pkt >> sNpcId >> nItemID >> page >> bSrcPos >> bDstPos >> nCount;

		if (page > 1 || bSrcPos > HAVE_MAX || bDstPos > HAVE_MAX)
			return UseVipKeyError(opcode, 2);

		_ITEM_DATA* pSrcItem = GetItem(SLOT_MAX + bSrcPos);
		_ITEM_DATA* pDstItem = GetItem(SLOT_MAX + bDstPos);
		if (pSrcItem == nullptr || pDstItem == nullptr)
			return UseVipKeyError(opcode, 2);

		//_ITEM_DATA* tmpItem;
		//memcpy(&tmpItem, pDstItem, sizeof(_ITEM_DATA)); // Temporarily store the target item
		//memcpy(pDstItem, pSrcItem, sizeof(_ITEM_DATA)); // Replace the target item with the source
		//memcpy(pSrcItem, &tmpItem, sizeof(_ITEM_DATA)); // Now replace the source with the old target (swapping them)
		//result << opcode << uint8(1);
		//Send(&result);
		//DeaFSezo buradaki _ITEM_DATA* yazýyordu * kaldýrýldý
		_ITEM_DATA tmpItem;
		memcpy(&tmpItem, pDstItem, sizeof(_ITEM_DATA)); // Hedef öðeyi geçici olarak sakla
		memcpy(pDstItem, pSrcItem, sizeof(_ITEM_DATA)); // Hedef öðeyi kaynak ile deðiþtir
		memcpy(pSrcItem, &tmpItem, sizeof(_ITEM_DATA)); // Þimdi kaynak öðeyi eski hedefle deðiþtir (deðiþ tokuþ)
		result << opcode << uint8(1);
		Send(&result);

	}
	else if (opcode == VIP_UseVault) // Use Vault Key
	{
		if (m_bVIPStoragePassword.length() != 4)
		{
			result << opcode << uint8(2);
			Send(&result);
			return;
		}

		if (m_bVIPStorageVaultExpiration - UNIXTIME > 60 * 60 * 24 * 30)
		{
			result << opcode << uint8(5);
			Send(&result);
			return;
		}

		uint16 sNpcId;
		uint32 nItemID;
		uint8 page, bSrcPos, bDstPos;
		pkt >> sNpcId >> nItemID >> page >> bSrcPos >> bDstPos;

		if (nItemID != VIP_VAULT_KEY
			&& nItemID != VIP_SAFE_KEY_1
			&& nItemID != VIP_SAFE_KEY_7) {
			result << uint8(VIP_UseVault) << uint8(2);
			Send(&result);
			return;
		}

		if (!CheckExistItem(nItemID)) {
			result << opcode << uint8(2);
			Send(&result);
			return;
		}

		result << opcode << sNpcId << nItemID << page << bSrcPos << bDstPos;
		g_pMain->AddDatabaseRequest(result, this);
	}
	else if (opcode == VIP_SetPassword) // Set Password
	{
		std::string password;
		pkt.SByte();
		pkt >> password;

		if (password.length() != 4)
		{
			result << opcode << uint8(2);
			Send(&result);
			return;
		}

		if (!std::all_of(password.begin(), password.end(), ::isdigit))
		{
			result << opcode << uint8(2);
			Send(&result);
			return;
		}

		result << opcode << password;
		g_pMain->AddDatabaseRequest(result, this);
	}
	else if (opcode == VIP_CancelPassword) // Cancel password
	{
		result << opcode;
		g_pMain->AddDatabaseRequest(result, this);
	}
	else if (opcode == VIP_ChangePassword) // Change Password
	{
		std::string password;
		pkt.SByte();
		pkt >> password;

		if (password.length() != 4)
		{
			result << opcode << uint8(2);
			Send(&result);
			return;
		}

		if (!std::all_of(password.begin(), password.end(), ::isdigit))
		{
			result << opcode << uint8(2);
			Send(&result);
			return;
		}

		result << opcode << password;
		g_pMain->AddDatabaseRequest(result, this);
	}
	else if (opcode == VIP_EnterPassword) // Pass login
	{
		std::string password;
		pkt.SByte();
		pkt >> password;

		if (password.length() != 4)
		{
			result << opcode << uint8(2);
			Send(&result);
			return;
		}

		if (!std::all_of(password.begin(), password.end(), ::isdigit))
		{
			result << opcode << uint8(2);
			Send(&result);
			return;
		}

		if (password != m_bVIPStoragePassword)
		{
			result << opcode << uint8(2);
			Send(&result);
			return;
		}
		result << opcode << uint8(1) << uint8(0);
		Send(&result);

		result.clear();
		result.Initialize(WIZ_VIPWAREHOUSE);
		result << uint8(1) << uint8(1) << uint8(m_bVIPStoragePassword.length() == 4);

		if (m_bVIPStorageVaultExpiration < UNIXTIME)
			result << uint8(0) << uint32(0);
		else
			result << uint8(1) << uint32(m_bVIPStorageVaultExpiration);

		result << uint32(VIP_VAULT_KEY) << uint16(1) << uint16(1) << uint8(0) << uint32(0) << uint16(0);

		for (int i = 0; i < VIPWAREHOUSE_MAX; i++)
		{
			_ITEM_DATA* pItem = &m_sVIPWarehouseArray[i];

			if (pItem == nullptr)
				continue;

			result << pItem->nNum
				<< pItem->sDuration
				<< pItem->sCount
				<< pItem->bFlag;

			_ITEM_TABLE pItemTable = g_pMain->GetItemPtr(pItem->nNum);
			if (!pItemTable.isnull())
			{
				if (pItemTable.isPetItem())
					ShowPetItemInfo(result, pItem->nSerialNum);
				else if (pItemTable.Getnum() == ITEM_CYPHER_RING)
					ShowCyperRingItemInfo(result, pItem->nSerialNum);
				else
					result << uint32(0); // Item Unique ID
			}
			else
				result << uint32(0); // Item Unique ID*/

			/*if (pItem->nNum == ITEM_CYPHER_RING)
				ShowCyperRingItemInfo(result, pItem->nSerialNum);
			else
				result << uint32(0); // Item Unique ID*/

			result << pItem->nExpirationTime;
		}
		Send(&result);
	}
}

#pragma endregion