#include "StdAfx.h"

COMMAND_HANDLER(CUser::HandleBotDisconnected)
{
	if (!isGM())
		return false;

	if (vargs.size() > 0)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Kullanim: +botkillone (hedefteki tek botu siler)");
		return true;
	}

	CBot* pBot = g_pMain->GetBotPtr(GetTargetID());
	if (pBot == nullptr)
	{
		g_pMain->SendHelpDescription(this, "Such a bots does not exist in the game.");
		return true;
	}

	pBot->UserInOut(INOUT_OUT);
	g_pMain->RemoveMapBotList(pBot->GetID(), pBot->GetName());
	return true;
}

COMMAND_HANDLER(CUser::HandleBotAfkSystem)
{
	if (!isGM())
		return false;

	if (vargs.size() < 2)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +afkbotspawn count minute level");
		return true;
	}

	int sTime = 0, sLevel = 0, sCount = 0;
	sCount = atoi(vargs.front().c_str());
	vargs.pop_front();
	sTime = atoi(vargs.front().c_str());
	vargs.pop_front();
	sLevel = atoi(vargs.front().c_str());

	if (sCount > 100)
		sCount = 100;

	for (int i = 0; i < sCount; i++)
	{
		float BonX = myrand(1, 5) * 1.0f;
		float BonZ = myrand(1, 5) * 1.0f;
		g_pMain->SpawnEventAfkBotHandler(sTime, GetZoneID(), GetX() + BonX, GetY(), GetZ() + BonZ, sLevel);
	}
	return true;
}

COMMAND_HANDLER(CUser::HandleBotAllDisconnected)
{
	if (!isGM())
		return false;

	std::vector<uint32> HandleBotAllDisconnected;
	if (vargs.size() > 0)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Kullanim: +killbot (tum botlari siler)");
		return true;
	}

	g_pMain->m_MapBotList.m_lock.lock();
	auto m_sMapBotListArray = g_pMain->m_MapBotList.m_UserTypeMap;
	g_pMain->m_MapBotList.m_lock.unlock();

	foreach(itr, m_sMapBotListArray)
	{
		CBot* pBot = g_pMain->GetBotPtr(itr->first);
		if (pBot == nullptr)
			continue;

		HandleBotAllDisconnected.push_back(pBot->GetID());
	}

	if (HandleBotAllDisconnected.size() > 0)
	{
		foreach(itr, HandleBotAllDisconnected)
		{
			CBot* pBot = g_pMain->GetBotPtr(*itr);
			if (pBot == nullptr)
				continue;

			pBot->UserInOut(INOUT_OUT);
			g_pMain->RemoveMapBotList(pBot->GetID(), pBot->GetName());
		}
	}
	return true;
}

COMMAND_HANDLER(CUser::HandleBotSpawnMining)
{
	if (!isGM())
		return false;

	if (vargs.size() < 3)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +miningbotspawn Count Minute MinLevel");
		return true;
	}

	int sTime = 0, sLevel = 0, sCount = 0;
	sCount = atoi(vargs.front().c_str());
	vargs.pop_front();
	sTime = atoi(vargs.front().c_str());
	vargs.pop_front();
	sLevel = atoi(vargs.front().c_str());

	if (sCount > 100)
		sCount = 100;

	for (int i = 0; i < sCount; i++)
	{
		float BonX = myrand(1, 5) * 1.0f;
		float BonZ = myrand(1, 5) * 1.0f;
		g_pMain->SpawnEventBotMining(sTime, GetZoneID(), GetX() + BonX, GetY(), GetZ() + BonZ, sLevel);
	}
	return true;
}

COMMAND_HANDLER(CUser::HandleBotSpawnMerchant)
{
	if (!isGM())
		return false;

	if (vargs.size() < 1)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +merchantbotspawn sIndex");
		return true;
	}

	DateTime time;
	int sIndex = 0;
	sIndex = atoi(vargs.front().c_str());

	uint16 m_sSocketID = g_pMain->SpawnEventBotMerchant(3600, GetZoneID(), GetX(), GetY(), GetZ(), 0, MIN_LEVEL_ARDREAM);
	CBot* pBot = g_pMain->GetBotPtr(m_sSocketID);
	if (pBot == nullptr)
		return true;

	_BOT_MERCHANT_ITEM* pBotMerchantTable = g_pMain->m_ArtificialMerchantArray.GetData(sIndex);
	if (pBotMerchantTable == nullptr)
	{
		pBot->UserInOut(INOUT_OUT);
		g_pMain->RemoveMapBotList(pBot->GetID(), pBot->GetName());
		return true;
	}

	pBot->m_bPremiumMerchant = (bool)pBotMerchantTable->BotMerchantType;

	Packet result(WIZ_MERCHANT, uint8(MERCHANT_INSERT));
	result << uint16(1) << pBotMerchantTable->BotMerchantMessage << pBot->GetID()
		<< pBot->m_bPremiumMerchant;

	if (!pBotMerchantTable->BotMerchantMessage.empty())
		pBot->MerchantChat = string_format("%s(Location:%d,%d)", pBotMerchantTable->BotMerchantMessage.c_str(), pBot->GetSPosX() / 10, pBot->GetSPosZ() / 10);
	else
		pBot->MerchantChat.clear();

	uint16 totalMerchItems = 0;
	for (int t = 0; t < MAX_MERCH_ITEMS; t++)
	{
		pBot->m_arMerchantItems[t].bCount = pBotMerchantTable->m_MerchantItems[t].bCount;
		pBot->m_arMerchantItems[t].orgcount = pBotMerchantTable->m_MerchantItems[t].bCount;
		pBot->m_arMerchantItems[t].bOriginalSlot = pBotMerchantTable->m_MerchantItems[t].bOriginalSlot;
		pBot->m_arMerchantItems[t].IsSoldOut = pBotMerchantTable->m_MerchantItems[t].IsSoldOut;
		pBot->m_arMerchantItems[t].nNum = pBotMerchantTable->m_MerchantItems[t].nNum;
		pBot->m_arMerchantItems[t].nPrice = pBotMerchantTable->m_MerchantItems[t].nPrice;
		pBot->m_arMerchantItems[t].nSerialNum = pBotMerchantTable->m_MerchantItems[t].nSerialNum;
		pBot->m_arMerchantItems[t].sCount = pBotMerchantTable->m_MerchantItems[t].sCount;
		pBot->m_arMerchantItems[t].sDuration = pBotMerchantTable->m_MerchantItems[t].sDuration;

		if (pBot->m_arMerchantItems[t].nNum != 0 &&
			(pBot->m_arMerchantItems[t].bCount == 0
				|| pBot->m_arMerchantItems[t].bCount < pBotMerchantTable->m_MerchantItems[t].sCount))
			continue;

		result << pBot->m_arMerchantItems[t].nNum;

		if (pBot->m_arMerchantItems[t].nNum > 0)
			totalMerchItems++;
	}

	if (totalMerchItems == 0)
	{
		result.clear();
		result.Initialize(WIZ_MERCHANT);
		result << uint8(MERCHANT_CLOSE) << pBot->GetID();
		pBot->SendToRegion(&result);

		pBot->UserInOut(INOUT_OUT);
		g_pMain->RemoveMapBotList(pBot->GetID(), pBot->GetName());
		return true;
	}

	pBot->m_bMerchantState = MERCHANT_STATE_SELLING;
	pBot->SendToRegion(&result);
	MerchantClose();
	return true;
}

COMMAND_HANDLER(CUser::HandleBotSpawnFishing)
{
	if (!isGM())
		return false;

	if (vargs.size() < 3)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +fishingbotspawn Count Minute MinLevel");
		return true;
	}

	int sTime = 0, sLevel = 0, sCount = 0;
	sCount = atoi(vargs.front().c_str());
	vargs.pop_front();
	sTime = atoi(vargs.front().c_str());
	vargs.pop_front();
	sLevel = atoi(vargs.front().c_str());

	if (sCount > 100)
		sCount = 100;

	for (int i = 0; i < sCount; i++)
	{
		float BonX = myrand(1, 5) * 1.0f;
		float BonZ = myrand(1, 5) * 1.0f;
		g_pMain->SpawnEventBotFishing(sTime, GetZoneID(), GetX() + BonX, GetY(), GetZ() + BonZ, sLevel);
	}
	return true;
}

COMMAND_HANDLER(CUser::HandleBotSpawnFarm)
{
	if (!isGM())
		return false;

	if (vargs.size() < 6)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +farmbotspawn Count Minute MinLevel PartyLider Genie Class");
		return true;
	}

	int sTime = 0, sLevel = 0, sCount = 0, sPartyLider = 0, sGenie = 0, sClass = 0;
	sCount = atoi(vargs.front().c_str());
	vargs.pop_front();
	sTime = atoi(vargs.front().c_str());
	vargs.pop_front();
	sLevel = atoi(vargs.front().c_str());
	vargs.pop_front();
	sPartyLider = atoi(vargs.front().c_str());
	vargs.pop_front();
	sGenie = atoi(vargs.front().c_str());
	vargs.pop_front();
	sClass = atoi(vargs.front().c_str());

	if (sCount > 100)
		sCount = 100;

	for (int i = 0; i < sCount; i++)
	{
		float BonX = myrand(1, 5) * 1.0f;
		float BonZ = myrand(1, 5) * 1.0f;
		g_pMain->SpawnEventBotFarm(sTime, GetZoneID(), GetX() + BonX, GetY(), GetZ() + BonZ, sLevel, sPartyLider, sGenie, sClass);
	}
	return true;
}

COMMAND_HANDLER(CUser::HandleBotSpawnMerchantMove)
{
	if (!isGM())
		return false;

	if (vargs.size() < 3)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +merchantmovebotspawn Count Minute MinLevel");
		return true;
	}

	int sTime = 0, sLevel = 0, sCount = 0;
	sCount = atoi(vargs.front().c_str());
	vargs.pop_front();
	sTime = atoi(vargs.front().c_str());
	vargs.pop_front();
	sLevel = atoi(vargs.front().c_str());

	if (sCount > 100)
		sCount = 100;

	for (int i = 0; i < sCount; i++)
	{
		float BonX = myrand(1, 15) * 1.0f;
		float BonZ = myrand(1, 15) * 1.0f;
		g_pMain->SpawnEventBotMoveProcess(sTime, GetZoneID(), GetX() + BonX, GetY(), GetZ() + BonZ, sLevel);
	}
	return true;
}

COMMAND_HANDLER(CUser::HandleBotSpawnPk)
{
	if (!isGM())
		return false;

	if (vargs.size() < 5)
	{
		// send description
		g_pMain->SendHelpDescription(this, "Using Sample : +pkbotspawn Count Minute MinLevel Nation Class");
		return true;
	}

	int sTime = 0, sLevel = 0, sCount = 0, sNation = 0, sClass = 0;
	sCount = atoi(vargs.front().c_str());
	vargs.pop_front();
	sTime = atoi(vargs.front().c_str());
	vargs.pop_front();
	sLevel = atoi(vargs.front().c_str());
	vargs.pop_front();
	sNation = atoi(vargs.front().c_str());
	vargs.pop_front();
	sClass = atoi(vargs.front().c_str());

	if (sCount > 100)
		sCount = 100;

	for (int i = 0; i < sCount; i++)
	{
		float BonX = myrand(1, 15) * 1.0f;
		float BonZ = myrand(1, 15) * 1.0f;
		g_pMain->SpawnEventBotPk(sTime, GetZoneID(), GetX() + BonX, GetY(), GetZ() + BonZ, sLevel, sNation, sClass, this);
	}
	return true;
}


namespace
{
	static uint8 GetPkBoxSingleClassCode(const std::string & classToken)
	{
		std::string token = classToken;
		STRTOLOWER(token);

		if (token == "warrior" || token == "warr" || token == "savasci" || token == "savasci" || token == "1")
			return 1;

		if (token == "rogue" || token == "asas" || token == "archer" || token == "okcu" || token == "2")
			return 2;

		if (token == "mage" || token == "magician" || token == "wizard" || token == "buyucu" || token == "3")
			return 3;

		if (token == "priest" || token == "pr" || token == "4")
			return 4;

		if (token == "kurian" || token == "portu" || token == "porutu" || token == "14")
			return 14;

		return 0;
	}

	static const char * GetPkBoxSingleClassName(uint8 classCode)
	{
		switch (classCode)
		{
		case 1:  return "Warrior";
		case 2:  return "Rogue";
		case 3:  return "Mage";
		case 4:  return "Priest";
		case 14: return "Kurian";
		default: return "Bilinmeyen";
		}
	}
}

COMMAND_HANDLER(CUser::HandleBotSpawnPkMix)
{
	if (!isGM())
		return false;

	if (vargs.size() > 3)
	{
		g_pMain->SendHelpDescription(this, "Kullanim: +pkbotmix Dakika MinLevel [ZoneID]");
		return true;
	}

	int sTime = 60;
	int sLevel = GetMap() == nullptr ? 1 : GetMap()->GetMinLevelReq();
	uint8 zoneID = GetZoneID();

	if (!vargs.empty())
	{
		sTime = atoi(vargs.front().c_str());
		vargs.pop_front();
	}

	if (!vargs.empty())
	{
		sLevel = atoi(vargs.front().c_str());
		vargs.pop_front();
	}

	if (!vargs.empty())
		zoneID = (uint8)atoi(vargs.front().c_str());

	if (sTime <= 0)
		sTime = 60;

	if (sLevel <= 0)
		sLevel = 1;

	C3DMap* pZone = g_pMain->GetZoneByID(zoneID);
	if (pZone == nullptr)
	{
		g_pMain->SendHelpDescription(this, "Gecersiz zone. Ornek: +pkbotmix 500 83 61");
		return true;
	}

	const uint8 classes[] = { 1, 2, 3, 14, 1, 2, 3, 4 };
	const uint8 nations[] = { (uint8)Nation::KARUS, (uint8)Nation::ELMORAD };
	const int classCount = int(_countof(classes));
	const int nationCount = int(_countof(nations));
	uint16 spawned = 0;
	bool spawnAtGM = (zoneID == GetZoneID());

	for (int n = 0; n < nationCount; n++)
	{
		int classOffset = myrand(0, classCount - 1);

		for (int i = 0; i < 100; i++)
		{
			uint8 sClass = classes[(i + classOffset) % classCount];
			uint16 botID = 0;

			if (spawnAtGM)
			{
				float BonX = (float)myrand(-18, 18);
				float BonZ = (float)myrand(-18, 18);
				botID = g_pMain->SpawnEventBotPk(sTime, zoneID, GetX() + BonX, GetY(), GetZ() + BonZ, sLevel, nations[n], sClass, this);
			}
			else
			{
				botID = g_pMain->SpawnEventBotPk(sTime, zoneID, 0.0f, 0.0f, 0.0f, sLevel, nations[n], sClass, this, true);
			}

			if (botID != 0)
				spawned++;
		}
	}

	char buff[128];
	sprintf(buff, "PK mixed bots spawned: %u bot, zone=%u.", spawned, zoneID);
	g_pMain->SendHelpDescription(this, buff);
	return true;
}

namespace
{
	struct _FARM_POINT
	{
		float x;
		float y;
		float z;
		uint8 minLevel;
		uint8 maxLevel;
	};

	static uint8 ClampFarmLevel(uint8 level)
	{
		if (level < 1)
			return 1;
		if (level > 83)
			return 83;
		return level;
	}

	static std::vector<_FARM_POINT> BuildTownPointsRaw(float x, float z, uint8 minLevel, uint8 maxLevel, float y = 0.0f)
	{
		// HOME/START_POSITION tablosundaki koordinatlar oyun ici koordinat formatindadir.
		// Botlar tek noktaya binmesin diye LX/LZ mantigina benzer sekilde etrafa dagitilir.
		return {
			{x + 0.0f,  y, z + 0.0f,  minLevel, maxLevel},
			{x + 10.0f, y, z + 8.0f,  minLevel, maxLevel},
			{x - 10.0f, y, z + 8.0f,  minLevel, maxLevel},
			{x + 18.0f, y, z - 4.0f,  minLevel, maxLevel},
			{x - 18.0f, y, z - 4.0f,  minLevel, maxLevel},
			{x + 26.0f, y, z + 15.0f, minLevel, maxLevel},
			{x - 26.0f, y, z + 15.0f, minLevel, maxLevel},
			{x + 34.0f, y, z - 12.0f, minLevel, maxLevel},
			{x - 34.0f, y, z - 12.0f, minLevel, maxLevel},
			{x + 0.0f,  y, z + 28.0f, minLevel, maxLevel}
		};
	}

	static std::vector<_FARM_POINT> BuildTownPointsFromZonesTable(float initX, float initY, float initZ, uint8 minLevel, uint8 maxLevel)
	{
		// zones tablosundaki InitX/InitZ/InitY degerleri 100 katli tutuluyor.
		return BuildTownPointsRaw(initX / 100.0f, initZ / 100.0f, minLevel, maxLevel, initY / 100.0f);
	}

	static std::vector<_FARM_POINT> GetFarmPointsByZone(uint8 zoneID)
	{
		switch (zoneID)
		{
		case ZONE_KARUS:
		case 5:
		case 6:
			// HOME tablosu Nation=1 KarusZoneX/Z: Luferson spawn.
			return BuildTownPointsRaw(441.0f, 1625.0f, 35, 83);

		case ZONE_ELMORAD:
		case 7:
		case 8:
			// HOME tablosu Nation=2 ElmoZoneX/Z: EMC spawn.
			return BuildTownPointsRaw(1595.0f, 412.0f, 35, 83);

		case ZONE_KARUS_ESLANT:
		case 13:
		case 14:
			// HOME tablosu Nation=1 FreeZoneX/Z: Karus Eslant tarafi.
			return BuildTownPointsRaw(1380.0f, 1090.0f, 60, 83);

		case ZONE_ELMORAD_ESLANT:
		case 15:
		case 16:
			// HOME tablosu Nation=2 FreeZoneX/Z: Elmorad Eslant tarafi.
			return BuildTownPointsRaw(630.0f, 920.0f, 60, 83);

		case 21: case 22: case 23: case 24: case 25:
			// Moradon zones.xls InitX/InitZ: 81590/53079 -> 815.90/530.79
			return BuildTownPointsFromZonesTable(81590.0f, 469.0f, 53079.0f, 1, 83);

		default:
			return {};
		}
	}


	static bool IsFarmBotTownZone(uint8 zoneID)
	{
		switch (zoneID)
		{
		case ZONE_KARUS:
		case ZONE_ELMORAD:
		case ZONE_KARUS_ESLANT:
		case ZONE_ELMORAD_ESLANT:
		case 5:
		case 6:
		case 7:
		case 8:
		case 13:
		case 14:
		case 15:
		case 16:
		case 21:
		case 22:
		case 23:
		case 24:
		case 25:
			return true;
		default:
			return false;
		}
	}


static uint16 SpawnMixedFarmBots(CGameServerDlg* pMain, int minute, uint8 zoneID, uint16 botCount, float baseX, float baseY, float baseZ, bool usePresetPoints)
{
	if (pMain == nullptr || pMain->GetZoneByID(zoneID) == nullptr)
		return 0;

	if (botCount == 0)
		botCount = 1;

	// Yeni farmbotmix sistemi:
	// Botlar EMC/Luferson/Eslant town noktalarinda tek tek, oturur vaziyette bekler.
	// Kendileri party atmaz; normal user davet ederse PartyHandler tarafinda otomatik kabul eder.
	const uint8 classPool[5] = { 1, 2, 3, 4, 14 }; // warrior, rogue, mage, priest, kurian
	auto points = GetFarmPointsByZone(zoneID);
	uint16 spawned = 0;
	std::vector<size_t> pointOrder;

	if (usePresetPoints && !points.empty())
	{
		for (size_t i = 0; i < points.size(); i++)
			pointOrder.push_back(i);

		for (size_t i = pointOrder.size(); i > 1; --i)
		{
			size_t j = (size_t)myrand(0, (int32)i - 1);
			std::swap(pointOrder[i - 1], pointOrder[j]);
		}
	}

	for (uint16 p = 0; p < botCount; p++)
	{
		float px = baseX, py = baseY, pz = baseZ;
		uint8 minLevel = 45, maxLevel = 83;

		if (usePresetPoints && !points.empty())
		{
			const _FARM_POINT& pt = points[pointOrder[p % pointOrder.size()]];
			px = pt.x;
			py = pt.y;
			pz = pt.z;
			minLevel = ClampFarmLevel(pt.minLevel);
			maxLevel = ClampFarmLevel(pt.maxLevel);

			// Ayni town noktasi tekrar kullanilirsa daha genis, dogal bir dagilim ver.
			int repeat = int(p / pointOrder.size());
			float spread = 4.0f + float((repeat % 8) * 3);
			px += float(myrand(-int(spread), int(spread)));
			pz += float(myrand(-int(spread), int(spread)));
		}

		uint8 sClass = classPool[p % _countof(classPool)];
		uint16 botID = pMain->SpawnEventBotFarm(minute, zoneID, px, py, pz, minLevel, 0, 0, sClass, maxLevel);
		if (botID != 0)
		{
			CBot* pBot = pMain->GetBotPtr(botID);
			if (pBot != nullptr)
			{
				pBot->m_bPartyLeader = false;
				pBot->m_bGenieStatus = false;
				pBot->m_sTargetID = int16(-1);
				pBot->StateChangeServerDirect(1, USER_SITDOWN);
			}
			spawned++;
		}
	}

	return spawned;
}

} // namespace

COMMAND_HANDLER(CUser::HandleBotSpawnFarmMix)
{
	if (!isGM())
		return false;

	// +farmbotmix Minute BotCount [ZoneID]
	if (vargs.size() < 2)
	{
		g_pMain->SendHelpDescription(this, "Using Sample : +farmbotmix Minute BotCount [ZoneID]");
		return true;
	}

	int minute = atoi(vargs.front().c_str()); vargs.pop_front();
	int botCount = atoi(vargs.front().c_str()); vargs.pop_front();
	uint8 zoneID = GetZoneID();
	if (!vargs.empty())
		zoneID = (uint8)atoi(vargs.front().c_str());

	if (minute <= 0) minute = 60;
	if (botCount <= 0) botCount = 10;
	if (botCount > 300) botCount = 300;

	if (g_pMain->GetZoneByID(zoneID) == nullptr)
	{
		g_pMain->SendHelpDescription(this, "Invalid ZoneID. Allowed: 1/5/6(Karus), 2/7/8(Elmorad), 11/12/13/14/15/16(Eslant).");
		return true;
	}

	bool usePresetPoints = IsFarmBotTownZone(zoneID);
	uint16 spawned = SpawnMixedFarmBots(g_pMain, minute, zoneID, (uint16)botCount, GetX(), GetY(), GetZ(), usePresetPoints);
	g_pMain->SendHelpDescription(this, string_format("Farm bots spawned: %u (zone=%u, botcount=%d, minute=%d)", spawned, zoneID, botCount, minute));
	return true;
}

static bool BuildMerchantBotData(uint32 coordIndex, _MERCHANT_BOT_INFO* pCoord, _bot_merchant& merchant)
{
	if (pCoord == nullptr)
		return false;

	merchant = _bot_merchant();
	merchant.index = coordIndex;
	merchant.areaType = pCoord->type;
	merchant.isBuy = pCoord->isBuy;

	std::vector<_MERCHANT_BOT_ITEM*> items;
	g_pMain->pBotInfo.mItem.m_lock.lock();
	auto merchantItems = g_pMain->pBotInfo.mItem.m_UserTypeMap;
	g_pMain->pBotInfo.mItem.m_lock.unlock();

	foreach(itr, merchantItems)
	{
		_MERCHANT_BOT_ITEM* pItem = itr->second;
		if (pItem == nullptr
			|| pItem->type != pCoord->type
			|| pItem->itemid == 0)
			continue;

		auto pTable = g_pMain->GetItemPtr(pItem->itemid);
		if (pTable.isnull())
			continue;

		items.push_back(pItem);
	}

	if (items.empty())
		return false;

	uint8 slot = 0;
	uint8 attempts = 0;
	while (slot < MAX_MERCH_ITEMS && attempts < MAX_MERCH_ITEMS * 4)
	{
		attempts++;
		_MERCHANT_BOT_ITEM* pItem = items[myrand(0, (int32)items.size() - 1)];
		if (pItem == nullptr)
			continue;

		auto pTable = g_pMain->GetItemPtr(pItem->itemid);
		if (pTable.isnull())
			continue;

		uint32 minCount = std::max<uint32>(1, pItem->minItemCount);
		uint32 maxCount = std::max<uint32>(minCount, pItem->maxItemCount);
		// Force merchant bot prices to normal cashpoint(noah/cash channel), not bonus KC channel.
		uint32 minPrice = (pItem->moneytype == 0 ? pItem->minPrice : pItem->minKc);
		uint32 maxPrice = (pItem->moneytype == 0 ? pItem->maxPrice : pItem->maxKc);
		maxPrice = std::max<uint32>(minPrice, maxPrice);

		if (minPrice == 0 && maxPrice == 0)
			continue;

		merchant.merc[slot].itemid = pItem->itemid;
		merchant.merc[slot].count = myrand(minCount, maxCount);
		merchant.merc[slot].price = myrand(minPrice, maxPrice);
		merchant.merc[slot].iskc = false;
		merchant.merc[slot].pTable = pTable;
		slot++;
	}

	return slot > 0;
}

static bool IsMerchantSpawnOccupied(uint8 zoneID, float x, float z, float radius = 2.5f)
{
	g_pMain->m_MapBotList.m_lock.lock();
	auto botMap = g_pMain->m_MapBotList.m_UserTypeMap;
	g_pMain->m_MapBotList.m_lock.unlock();

	foreach(itr, botMap)
	{
		CBot* pBot = itr->second;
		if (pBot == nullptr
			|| !pBot->isInGame()
			|| pBot->GetZoneID() != zoneID)
			continue;

		if (pBot->isInRangeSlow(x, z, radius))
			return true;
	}

	return false;
}

COMMAND_HANDLER(CUser::HandleMerchantBotMix)
{
	if (!isGM())
		return false;

	uint16 botCount = 100;
	uint32 botMinute = 9999;
	uint16 areaType = 0;

	if (!vargs.empty()) { botCount = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { botMinute = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { areaType = atoi(vargs.front().c_str()); vargs.pop_front(); }

	if (botCount == 0 || botCount > 100)
		botCount = 100;

	if (botMinute == 0)
		botMinute = 9999;

	// Refresh merchant bot sources from DB each run to avoid stale/empty pools.
	g_pMain->LoadBotMerchantTable();

	uint16 spawnedCount = 0;
	uint32 coordTotal = 0, coordNull = 0, coordUsed = 0, coordWrongZone = 0, coordWrongType = 0, coordOccupied = 0;
	uint32 merchantBuildFail = 0, spawnFail = 0;
	uint32 itemTotal = 0, itemMatchingType = 0;
	uint32 botTemplateTotal = 0, botTemplateInUse = 0, botTemplateLevelFail = 0;

	g_pMain->pBotInfo.mItem.m_lock.lock();
	auto merchantItemsDebug = g_pMain->pBotInfo.mItem.m_UserTypeMap;
	g_pMain->pBotInfo.mItem.m_lock.unlock();
	foreach(itrItemDebug, merchantItemsDebug)
	{
		_MERCHANT_BOT_ITEM* pItem = itrItemDebug->second;
		if (pItem == nullptr)
			continue;

		itemTotal++;
		if (areaType == 0 || pItem->type == areaType)
			itemMatchingType++;
	}

	g_pMain->m_ArtificialIntelligenceArray.m_lock.lock();
	auto botTemplatesDebug = g_pMain->m_ArtificialIntelligenceArray.m_UserTypeMap;
	g_pMain->m_ArtificialIntelligenceArray.m_lock.unlock();
	foreach(itrBotDebug, botTemplatesDebug)
	{
		_BOT_DATA* pBotInfo = itrBotDebug->second;
		if (pBotInfo == nullptr)
			continue;

		botTemplateTotal++;
		if (pBotInfo->m_bLevel < 1)
			botTemplateLevelFail++;
		else if (g_pMain->GetBotPtr(pBotInfo->m_sSid) != nullptr)
			botTemplateInUse++;
	}

	g_pMain->pBotInfo.mCoordinate.m_lock.lock();
	auto merchantCoords = g_pMain->pBotInfo.mCoordinate.m_UserTypeMap;
	g_pMain->pBotInfo.mCoordinate.m_lock.unlock();

	foreach(itr, merchantCoords)
	{
		if (spawnedCount >= botCount)
			break;

		coordTotal++;
		uint32 coordIndex = itr->first;
		_MERCHANT_BOT_INFO* pCoord = itr->second;
		if (pCoord == nullptr)
		{
			coordNull++;
			continue;
		}

		if (pCoord->used)
		{
			coordUsed++;
			continue;
		}

		if (pCoord->bZoneID != ZONE_MORADON)
		{
			coordWrongZone++;
			continue;
		}

		if (areaType != 0 && pCoord->type != areaType)
		{
			coordWrongType++;
			continue;
		}

		if (IsMerchantSpawnOccupied(pCoord->bZoneID, (float)pCoord->setX, (float)pCoord->setZ))
		{
			coordOccupied++;
			continue;
		}

		_bot_merchant merchant;
		if (!BuildMerchantBotData(coordIndex, pCoord, merchant))
		{
			merchantBuildFail++;
			continue;
		}

		pCoord->used = true;
		uint16 botID = g_pMain->SpawnUserBot(botMinute, pCoord->bZoneID, (float)pCoord->setX, (float)pCoord->setY, (float)pCoord->setZ, 6, 1, pCoord->direction, 0, 0, merchant);
		if (botID == 0)
		{
			pCoord->used = false;
			spawnFail++;
			continue;
		}

		spawnedCount++;
	}

	g_pMain->SendHelpDescription(this, string_format("Moradon merchant bots spawned: %d", spawnedCount));
	if (spawnedCount == 0)
	{
		g_pMain->SendHelpDescription(this, string_format("MerchantMix debug: coords=%u null=%u used=%u zoneFail=%u typeFail=%u occupied=%u buildFail=%u spawnFail=%u",
			coordTotal, coordNull, coordUsed, coordWrongZone, coordWrongType, coordOccupied, merchantBuildFail, spawnFail));
		g_pMain->SendHelpDescription(this, string_format("MerchantMix debug: items=%u matchingType=%u botTemplates=%u inUse=%u levelFail=%u",
			itemTotal, itemMatchingType, botTemplateTotal, botTemplateInUse, botTemplateLevelFail));
	}
	return true;
}


COMMAND_HANDLER(CUser::HandlePkBoxSingleCommand)
{
	if (!isGM())
		return false;

	if (vargs.size() < 4)
	{
		g_pMain->SendHelpDescription(this, "Kullanim: +pkboxsingle Sinif Dakika MinLevel ZoneID");
		g_pMain->SendHelpDescription(this, "Siniflar: warrior, rogue, mage, priest, kurian");
		g_pMain->SendHelpDescription(this, "Ornek: +pkboxsingle warrior 500 83 71");
		return true;
	}

	uint8 sClass = GetPkBoxSingleClassCode(vargs.front());
	vargs.pop_front();
	int sTime = atoi(vargs.front().c_str());
	vargs.pop_front();
	int sLevel = atoi(vargs.front().c_str());
	vargs.pop_front();
	uint8 zoneID = (uint8)atoi(vargs.front().c_str());

	if (sClass == 0)
	{
		g_pMain->SendHelpDescription(this, "Hatali sinif. Kullanilabilir siniflar: warrior, rogue, mage, priest, kurian");
		return true;
	}

	if (sTime <= 0)
		sTime = 60;

	if (sLevel <= 0)
		sLevel = 1;

	if (g_pMain->GetZoneByID(zoneID) == nullptr)
	{
		g_pMain->SendHelpDescription(this, string_format("Hatali ZoneID: %u", zoneID));
		return true;
	}

	const uint8 nations[] = { (uint8)Nation::KARUS, (uint8)Nation::ELMORAD };
	uint16 spawned = 0;

	for (int n = 0; n < int(_countof(nations)); n++)
	{
		for (int i = 0; i < 100; i++)
		{
			uint16 botID = g_pMain->SpawnEventBotPk(sTime, zoneID, 0.0f, 0.0f, 0.0f, sLevel, nations[n], sClass, this, true);
			if (botID != 0)
				spawned++;
		}
	}

	g_pMain->SendHelpDescription(this, string_format("PKBoxSingle tamamlandi. Sinif=%s, Spawn=%u, Sure=%d dk, Level=%d+, Zone=%u", GetPkBoxSingleClassName(sClass), spawned, sTime, sLevel, zoneID));
	return true;
}


COMMAND_HANDLER(CGameServerDlg::HandleServerPkBoxSingleCommand)
{
	if (vargs.size() < 4)
	{
		printf("[ServerCommand] Using: /pkboxsingle Class Minute MinLevel ZoneID\n");
		printf("[ServerCommand] Classes: warrior, rogue, mage, priest, kurian\n");
		printf("[ServerCommand] Example: /pkboxsingle warrior 500 83 71\n");
		return true;
	}

	uint8 sClass = GetPkBoxSingleClassCode(vargs.front());
	vargs.pop_front();
	int sTime = atoi(vargs.front().c_str());
	vargs.pop_front();
	int sLevel = atoi(vargs.front().c_str());
	vargs.pop_front();
	uint8 zoneID = (uint8)atoi(vargs.front().c_str());

	if (sClass == 0)
	{
		printf("[ServerCommand] /pkboxsingle failed. invalid class. Use: warrior, rogue, mage, priest, kurian\n");
		return true;
	}

	if (sTime <= 0) sTime = 60;
	if (sLevel <= 0) sLevel = 1;

	if (GetZoneByID(zoneID) == nullptr)
	{
		printf("[ServerCommand] /pkboxsingle failed. invalid zone=%u\n", zoneID);
		return true;
	}

	const uint8 nations[] = { (uint8)Nation::KARUS, (uint8)Nation::ELMORAD };
	uint16 spawned = 0;

	for (int n = 0; n < int(_countof(nations)); n++)
	{
		for (int i = 0; i < 100; i++)
		{
			uint16 botID = SpawnEventBotPk(sTime, zoneID, 0.0f, 0.0f, 0.0f, sLevel, nations[n], sClass, nullptr, true);
			if (botID != 0)
				spawned++;
		}
	}

	printf("[ServerCommand] /pkboxsingle done. class=%s spawned=%u zone=%u minute=%d minLevel=%d\n", GetPkBoxSingleClassName(sClass), spawned, zoneID, sTime, sLevel);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleServerPkBotMixCommand)
{
	int sTime = 60;
	int sLevel = 1;
	uint8 zoneID = ZONE_RONARK_LAND;

	if (!vargs.empty()) { sTime = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { sLevel = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { zoneID = (uint8)atoi(vargs.front().c_str()); vargs.pop_front(); }

	if (sTime <= 0) sTime = 60;
	if (sLevel <= 0) sLevel = 1;
	if (GetZoneByID(zoneID) == nullptr)
	{
		printf("[ServerCommand] /pkbotmix failed. invalid zone=%u (example CZ:71, CZ base:73, Ardream:72)\n", zoneID);
		return true;
	}

	const uint8 classes[] = { 1, 2, 3, 4, 14 };
	const uint8 nations[] = { (uint8)Nation::KARUS, (uint8)Nation::ELMORAD };
	const int classCount = int(_countof(classes));
	const int nationCount = int(_countof(nations));

	uint16 spawned = 0;
	for (int n = 0; n < nationCount; n++)
	{
		int classOffset = myrand(0, classCount - 1);
		for (int i = 0; i < 100; i++)
		{
			uint8 sClass = classes[(i + classOffset) % classCount];
			uint16 botID = SpawnEventBotPk(sTime, zoneID, 0.0f, 0.0f, 0.0f, sLevel, nations[n], sClass, nullptr, true);
			if (botID != 0)
				spawned++;
		}
	}

	printf("[ServerCommand] /pkbotmix done. spawned=%u zone=%u minute=%d minLevel=%d\n", spawned, zoneID, sTime, sLevel);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleServerFarmBotMixCommand)
{
	// /farmbotmix Minute BotCount [ZoneID]
	int minute = 60;
	int botCount = 20;
	uint8 zoneID = 0; // 0 = all requested zones

	if (!vargs.empty()) { minute = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { botCount = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { zoneID = (uint8)atoi(vargs.front().c_str()); vargs.pop_front(); }

	if (minute <= 0) minute = 60;
	if (botCount <= 0) botCount = 20;
	if (botCount > 500) botCount = 500;

	uint16 totalSpawned = 0;
	if (zoneID == 0)
	{
		const uint8 zones[] = { ZONE_KARUS, ZONE_ELMORAD, ZONE_KARUS_ESLANT, ZONE_ELMORAD_ESLANT };
		for (int i = 0; i < (int)_countof(zones); i++)
			totalSpawned += SpawnMixedFarmBots(this, minute, zones[i], (uint16)botCount, 0.0f, 0.0f, 0.0f, true);

		printf("[ServerCommand] /farmbotmix done. spawned=%u zones=all minute=%d botCountPerZone=%d\n", totalSpawned, minute, botCount);
		return true;
	}

	if (GetZoneByID(zoneID) == nullptr)
	{
		printf("[ServerCommand] /farmbotmix failed. invalid zone=%u\n", zoneID);
		return true;
	}

	bool usePresetPoints = IsFarmBotTownZone(zoneID);
	totalSpawned = SpawnMixedFarmBots(this, minute, zoneID, (uint16)botCount, 0.0f, 0.0f, 0.0f, usePresetPoints);
	printf("[ServerCommand] /farmbotmix done. spawned=%u zone=%u minute=%d botcount=%d\n", totalSpawned, zoneID, minute, botCount);
	return true;
}

COMMAND_HANDLER(CGameServerDlg::HandleServerMerchantBotMixCommand)
{
	uint16 botCount = 100;
	uint32 botMinute = 9999;
	uint16 areaType = 0;
	uint8 zoneID = ZONE_MORADON;

	if (!vargs.empty()) { botCount = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { botMinute = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { areaType = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!vargs.empty()) { zoneID = (uint8)atoi(vargs.front().c_str()); vargs.pop_front(); }

	if (botCount == 0 || botCount > 1000)
		botCount = 100;
	if (botMinute == 0)
		botMinute = 9999;
	if (GetZoneByID(zoneID) == nullptr)
	{
		printf("[ServerCommand] /merchantbotmix failed. invalid zone=%u\n", zoneID);
		return true;
	}

	LoadBotMerchantTable();

	uint16 spawnedCount = 0;
	uint32 coordTotal = 0, coordNull = 0, coordUsed = 0, coordWrongZone = 0, coordWrongType = 0, coordOccupied = 0;
	uint32 merchantBuildFail = 0, spawnFail = 0;

	pBotInfo.mCoordinate.m_lock.lock();
	auto merchantCoords = pBotInfo.mCoordinate.m_UserTypeMap;
	pBotInfo.mCoordinate.m_lock.unlock();

	foreach(itr, merchantCoords)
	{
		if (spawnedCount >= botCount)
			break;

		coordTotal++;
		uint32 coordIndex = itr->first;
		_MERCHANT_BOT_INFO* pCoord = itr->second;
		if (pCoord == nullptr) { coordNull++; continue; }
		if (pCoord->used) { coordUsed++; continue; }
		if (pCoord->bZoneID != zoneID) { coordWrongZone++; continue; }
		if (areaType != 0 && pCoord->type != areaType) { coordWrongType++; continue; }
		if (IsMerchantSpawnOccupied(pCoord->bZoneID, (float)pCoord->setX, (float)pCoord->setZ)) { coordOccupied++; continue; }

		_bot_merchant merchant;
		if (!BuildMerchantBotData(coordIndex, pCoord, merchant))
		{
			merchantBuildFail++;
			continue;
		}

		pCoord->used = true;
		uint16 botID = SpawnUserBot(botMinute, pCoord->bZoneID, (float)pCoord->setX, (float)pCoord->setY, (float)pCoord->setZ, 6, 1, pCoord->direction, 0, 0, merchant);
		if (botID == 0)
		{
			pCoord->used = false;
			spawnFail++;
			continue;
		}
		spawnedCount++;
	}

	printf("[ServerCommand] /merchantbotmix done. spawned=%u zone=%u minute=%u type=%u\n", spawnedCount, zoneID, botMinute, areaType);
	if (spawnedCount == 0)
	{
		printf("[ServerCommand] MerchantMix debug: coords=%u null=%u used=%u zoneFail=%u typeFail=%u occupied=%u buildFail=%u spawnFail=%u\n",
			coordTotal, coordNull, coordUsed, coordWrongZone, coordWrongType, coordOccupied, merchantBuildFail, spawnFail);
	}

	return true;
}

COMMAND_HANDLER(CUser::HandleMerchantBotCommand)
{
	if (!isGM())
		return true;

	if (vargs.size() < 3) {
		g_pMain->SendHelpDescription(this, "Using Sample : /merchantbot Count Time AreaType.");
		return true;
	}

	uint16 botCount = 0;
	if (!vargs.empty()) { botCount = atoi(vargs.front().c_str()); vargs.pop_front(); }

	if (!botCount || botCount > 100) {
		g_pMain->SendHelpDescription(this, "The maximum number of robots you can buy cannot exceed 100.");
		return true;
	}

	uint32 botTime = 0;
	if (!vargs.empty()) { botTime = atoi(vargs.front().c_str()); vargs.pop_front(); }

	botTime = botTime * SECOND;

	uint32 botArea = 0;
	if (!vargs.empty()) { botArea = atoi(vargs.front().c_str()); vargs.pop_front(); }
	if (!botArea) {
		g_pMain->SendHelpDescription(this, "Bot Area Error.");
		return true;
	}

	BotMerchantAdd(botCount, botTime, botArea);
	return true;
}

void CUser::BotMerchantAdd(uint16 count, uint32 bTime, uint16 type)
{
	struct _list {
		uint32 index;
		_MERCHANT_BOT_INFO pInfo;
		_list(uint32 index, _MERCHANT_BOT_INFO pInfo) {
			this->index = index;
			this->pInfo = pInfo;
		}
	};
	std::vector<_list> mList;

	uint16 counter = 0; uint32 index = 0;
	g_pMain->pBotInfo.mCoordinate.m_lock.lock();
	auto m_bMerchantCoordinateStatus = g_pMain->pBotInfo.mCoordinate.m_UserTypeMap;
	g_pMain->pBotInfo.mCoordinate.m_lock.unlock();

	foreach(itr, m_bMerchantCoordinateStatus)
	{
		auto* pCoordinat = itr->second;
		if (!pCoordinat
			|| pCoordinat->used
			|| pCoordinat->type != type)
			continue;

		if (counter >= count)
			break;

		counter++;
		index = itr->first;
		pCoordinat->used = true;
		mList.push_back(_list(itr->first, *pCoordinat));
	}

	if (mList.empty())
	{
		if (index) {
			auto* pCoord = g_pMain->pBotInfo.mCoordinate.GetData(index);
			if (pCoord)
				pCoord->used = false;
		}
		g_pMain->SendHelpDescription(this, "List is empty!");
		return;
	}

	int atime = 1; ULONGLONG btime = UNIXTIME2 + (ULONGLONG)(atime * bTime);
	g_pMain->m_addbotlistLock.lock();
	for (auto& itr : mList) {
		g_pMain->m_addbotlist.insert(std::make_pair(itr.index, _botadd(itr.index, btime, (uint8)type)));
		btime += (ULONGLONG)(atime * bTime);
	}
	g_pMain->m_addbotlistLock.unlock();
}

void CBot::SendDeathNotice(Unit* pKiller, DeathNoticeType noticeType, bool isToZone /*= true*/)
{
	if (pKiller == nullptr)
		return;

	Packet result(WIZ_CHAT, uint8(DEATH_NOTICE));

	result.SByte();
	result << GetNation()
		<< uint8(noticeType)
		<< pKiller->GetID() // session ID?
		<< pKiller->GetName()
		<< GetID() // session ID?
		<< GetName()
		<< uint16(GetX()) << uint16(GetZ());

	if (isToZone)
		SendToZone(&result, RANGE_20M);
	else
	{
		SendToRegion(&result);
		if (pKiller->isPlayer())
			TO_USER(pKiller)->Send(&result);
	}
}

uint16 CGameServerDlg::SpawnEventAfkBotHandler(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel)
{
	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (byZone <= ZONE_ELMORAD && byZone != pBotInfo->m_bNation
			|| (byZone >= ZONE_KARUS_ESLANT
				&& byZone <= ZONE_ELMORAD_ESLANT
				&& byZone != (pBotInfo->m_bNation + 10)))
			continue;

		if (pBotInfo->m_bLevel < minlevel)
			continue;

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;

		int Random = myrand(0, 10000);
		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute * 60);
		pBot->m_pMap = GetZoneByID(byZone);
		pBot->m_bZone = byZone;
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->SetBotAbility();
		pBot->m_BotState = BOT_AFK;
		pBot->SetPosition(fX, fY, fZ);
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		pBot->StateChangeServerDirect(1, Random > 5000 ? USER_STANDING : USER_SITDOWN);
		return pBot->GetID();
	}
	return 0;
}

uint16 CGameServerDlg::SpawnEventBotMining(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel)
{
	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (byZone <= ZONE_ELMORAD && byZone != pBotInfo->m_bNation
			|| (byZone >= ZONE_KARUS_ESLANT
				&& byZone <= ZONE_ELMORAD_ESLANT
				&& byZone != (pBotInfo->m_bNation + 10)))
			continue;

		if (pBotInfo->m_bLevel < minlevel)
			continue;

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;

		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute * 60);
		pBot->m_pMap = GetZoneByID(byZone);
		pBot->m_bZone = byZone;

		_ITEM_DATA* pItem = pBot->GetItem(RIGHTHAND);
		if (pItem == nullptr)
		{
			_ITEM_TABLE pTable = GetItemPtr(MATTOCK);
			if (pTable.isnull())
				continue;

			_ITEM_DATA* pTItem = pBot->GetItem(LEFTHAND);
			if (pTItem != nullptr)
				memset(pTItem, 0x00, sizeof(_ITEM_DATA));

			memset(pItem, 0x00, sizeof(_ITEM_DATA));
			pItem->nNum = pTable.m_iNum;
			pItem->nSerialNum = GenerateItemSerial();
			pItem->sCount = 1;
			pItem->sDuration = pTable.m_sDuration;
		}
		else
		{
			_ITEM_TABLE pTable = GetItemPtr(MATTOCK);
			if (pTable.isnull())
				continue;

			_ITEM_DATA* pTItem = pBot->GetItem(LEFTHAND);
			if (pTItem != nullptr)
				memset(pTItem, 0x00, sizeof(_ITEM_DATA));

			memset(pItem, 0x00, sizeof(_ITEM_DATA));
			pItem->nNum = pTable.m_iNum;
			pItem->nSerialNum = GenerateItemSerial();
			pItem->sCount = 1;
			pItem->sDuration = pTable.m_sDuration;
		}

		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->m_bResHpType = USER_MINING;
		pBot->SetBotAbility();
		pBot->m_BotState = BOT_MINING;
		pBot->StateChangeServerDirect(1, USER_MINING);
		pBot->SetPosition(fX, fY, fZ);
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		return pBot->GetID();
	}
	return true;
}

uint16 CGameServerDlg::SpawnEventBotMerchant(int Minute, uint8 byZone, float fX, float fY, float fZ, int16 nDir, uint8 minlevel)
{
	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (byZone <= ZONE_ELMORAD && byZone != pBotInfo->m_bNation
			|| (byZone >= ZONE_KARUS_ESLANT
				&& byZone <= ZONE_ELMORAD_ESLANT
				&& byZone != (pBotInfo->m_bNation + 10)))
			continue;

		if (pBotInfo->m_bLevel < minlevel)
			continue;

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;

		int Random = myrand(0, 10000);
		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute * 60);
		pBot->m_pMap = GetZoneByID(byZone);
		pBot->m_bZone = byZone;
		pBot->m_sDirection = nDir;
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->SetBotAbility();
		pBot->m_BotState = BOT_MERCHANT;
		pBot->SetPosition(fX, fY, fZ);
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		pBot->StateChangeServerDirect(1, Random > 5000 ? USER_STANDING : USER_SITDOWN);
		return pBot->GetID();
	}
	return 0;
}

uint16 CGameServerDlg::SpawnEventBotFishing(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel)
{
	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (byZone <= ZONE_ELMORAD && byZone != pBotInfo->m_bNation
			|| (byZone >= ZONE_KARUS_ESLANT
				&& byZone <= ZONE_ELMORAD_ESLANT
				&& byZone != (pBotInfo->m_bNation + 10)))
			continue;

		if (pBotInfo->m_bLevel < minlevel)
			continue;

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;

		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute * 60);
		pBot->m_pMap = GetZoneByID(byZone);
		pBot->m_bZone = byZone;

		_ITEM_DATA* pItem = pBot->GetItem(RIGHTHAND);
		if (pItem == nullptr)
		{
			_ITEM_TABLE pTable = GetItemPtr(FISHING);
			if (pTable.isnull())
				continue;

			_ITEM_DATA* pTItem = pBot->GetItem(LEFTHAND);
			if (pTItem != nullptr)
				memset(pTItem, 0x00, sizeof(_ITEM_DATA));

			memset(pItem, 0x00, sizeof(_ITEM_DATA));
			pItem->nNum = pTable.m_iNum;
			pItem->nSerialNum = GenerateItemSerial();
			pItem->sCount = 1;
			pItem->sDuration = pTable.m_sDuration;
		}
		else
		{
			_ITEM_TABLE pTable = GetItemPtr(FISHING);
			if (pTable.isnull())
				continue;

			_ITEM_DATA* pTItem = pBot->GetItem(LEFTHAND);
			if (pTItem != nullptr)
				memset(pTItem, 0x00, sizeof(_ITEM_DATA));

			memset(pItem, 0x00, sizeof(_ITEM_DATA));
			pItem->nNum = pTable.m_iNum;
			pItem->nSerialNum = GenerateItemSerial();
			pItem->sCount = 1;
			pItem->sDuration = pTable.m_sDuration;
		}

		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->m_bResHpType = USER_FLASHING;
		pBot->SetBotAbility();
		pBot->m_BotState = BOT_FISHING;
		pBot->StateChangeServerDirect(1, USER_FLASHING);
		pBot->SetPosition(fX, fY, fZ);
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);

		return pBot->GetID();
	}
	return true;
}

uint16 CGameServerDlg::SpawnEventBotFarm(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel, uint8 sPartyLider, uint8 sGenie, uint8 sClass, uint8 maxlevel)
{
	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	// Once aralikli level secmeyi dener. Uygun bot sablonu yoksa eski sistemdeki gibi
	// sadece minimum level kontroluyle tekrar dener; bu sayede komut bos gorunup bot basmama yapmaz.
	for (uint8 pass = 0; pass < 2; pass++)
	{
		foreach(itr, m_sArtificialIntelligenceArray)
		{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (byZone <= ZONE_ELMORAD && byZone != pBotInfo->m_bNation
			|| (byZone >= ZONE_KARUS_ESLANT
				&& byZone <= ZONE_ELMORAD_ESLANT
				&& byZone != (pBotInfo->m_bNation + 10)))
			continue;

		if (pBotInfo->m_bLevel < minlevel)
			continue;

			if (pass == 0 && maxlevel > 0 && pBotInfo->m_bLevel > maxlevel)
				continue;

		_ITEM_DATA* pItem = pBotInfo->GetItem(RIGHTHAND);
		if (pItem == nullptr)
			continue;

		_ITEM_TABLE pTable = GetItemPtr(pItem->nNum);
		if (pTable.isnull())
			continue;

		if (pTable.isShield()
			|| pTable.isPickaxe()
			|| pTable.isFishing())
			continue;

		if (sClass > 0 && sClass < 15)
		{
			if (sClass == 1 && !pBotInfo->isWarrior())
				continue;

			if (sClass == 2 && !pBotInfo->isRogue())
				continue;

			if (sClass == 3 && !pBotInfo->isMage())
				continue;

			if (sClass == 4 && !pBotInfo->isPriest())
				continue;

			if (sClass == 14 && !pBotInfo->isPortuKurian())
				continue;

			if (sClass > 4 && sClass < 14)
				continue;
		}
		else
			continue;

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;		

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;

		pBot->m_PlayerKillingLoyaltyDaily = 0;
		pBot->m_PlayerKillingLoyaltyPremiumBonus = 0;

		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute * MINUTE);
		pBot->m_pMap = GetZoneByID(byZone);
		pBot->m_bZone = byZone;
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->SetBotAbility();
		pBot->m_BotState = BOT_FARMER;
		pBot->m_bPartyLeader = sPartyLider == 1 ? true : false;
		pBot->m_bGenieStatus = sGenie == 1 ? true : false;
		pBot->SetPosition(fX, fY, fZ);
		pBot->m_oldx = pBot->m_curx;
		pBot->m_oldy = pBot->m_cury;
		pBot->m_oldz = pBot->m_curz;
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		pBot->StateChangeServerDirect(1, USER_STANDING);

		if (byZone == ZONE_RONARK_LAND
			|| byZone == ZONE_RONARK_LAND_BASE
			|| byZone == ZONE_ARDREAM)
		{
			C3DMap* pMap = g_pMain->GetZoneByID(byZone);
			if (pMap == nullptr)
				continue;

			TRACE("%s In the Game Insert Player Ranking ZoneID %d\n", pBot->GetName().c_str(), pBot->GetZoneID());
			pBot->AddBotRank(pMap);
		}

		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		return pBot->GetID();
		}
	}
	return 0;
}

uint16 CGameServerDlg::SpawnEventBotMoveProcess(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel)
{
	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (byZone <= ZONE_ELMORAD && byZone != pBotInfo->m_bNation
			|| (byZone >= ZONE_KARUS_ESLANT
				&& byZone <= ZONE_ELMORAD_ESLANT
				&& byZone != (pBotInfo->m_bNation + 10)))
			continue;

		if (pBotInfo->m_bLevel < minlevel)
			continue;

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;

		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute * MINUTE);
		pBot->m_pMap = GetZoneByID(byZone);
		pBot->m_bZone = byZone;
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->SetBotAbility();
		pBot->m_BotState = BOT_MERCHANT_MOVE;
		pBot->SetPosition(fX, fY, fZ);
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		pBot->StateChangeServerDirect(1, USER_STANDING);
		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		return pBot->GetID();
	}
	return true;
}

uint16 CGameServerDlg::SpawnEventBotPk(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 minlevel, uint8 sNation, uint8 sClass, CUser* pUser, bool startup)
{
	if (!startup && pUser == nullptr)
		return true;

	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (!startup)
		{
			switch (byZone)
			{
			case ZONE_RONARK_LAND:
				if (pBotInfo->m_bLevel < pUser->GetMap()->GetMinLevelReq())
					continue;
				break;
			case ZONE_ARDREAM:
				if (pBotInfo->m_bLevel < pUser->GetMap()->GetMinLevelReq()
					|| pBotInfo->m_bLevel > pUser->GetMap()->GetMaxLevelReq())
					continue;
				break;
			case ZONE_RONARK_LAND_BASE:
				if (pBotInfo->m_bLevel < pUser->GetMap()->GetMinLevelReq()
					|| pBotInfo->m_bLevel > pUser->GetMap()->GetMaxLevelReq())
					continue;
				break;
			case ZONE_DELOS:
			case ZONE_BIFROST:
			case ZONE_SPBATTLE1:
			case ZONE_SPBATTLE2:
			case ZONE_SPBATTLE3:
			case ZONE_SPBATTLE4:
			case ZONE_SPBATTLE5:
			case ZONE_SPBATTLE6:
			case ZONE_SPBATTLE7:
			case ZONE_SPBATTLE8:
			case ZONE_SPBATTLE9:
			case ZONE_SPBATTLE10:
			case ZONE_SPBATTLE11:
			case ZONE_SPBATTLE12:
				if (pBotInfo->m_bLevel < pUser->GetMap()->GetMinLevelReq()
					|| pBotInfo->m_bLevel > pUser->GetMap()->GetMaxLevelReq())
					continue;
				break;
			default:
				continue;
				break;
			}
		}

		if (pBotInfo->m_bLevel < minlevel
			|| pBotInfo->m_bNation != sNation)
			continue;

		_ITEM_DATA* pItem = pBotInfo->GetItem(RIGHTHAND);
		if (pItem == nullptr)
			continue;

		_ITEM_TABLE pTable = GetItemPtr(pItem->nNum);
		if (pTable.isnull())
			continue;

		if (pTable.isShield()
			|| pTable.isPickaxe()
			|| pTable.isFishing())
			continue;

		if (sClass > 0 && sClass < 15)
		{
			if (sClass == 1 && !pBotInfo->isWarrior())
				continue;

			if (sClass == 2 && !pBotInfo->isRogue())
				continue;

			if (sClass == 3 && !pBotInfo->isMage())
				continue;

			if (sClass == 4 && !pBotInfo->isPriest())
				continue;

			if (sClass == 14 && !pBotInfo->isPortuKurian())
				continue;

			if (sClass > 4 && sClass < 14)
				continue;
		}
		else
			continue;

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;

		pBot->m_PlayerKillingLoyaltyDaily = 0;
		pBot->m_PlayerKillingLoyaltyPremiumBonus = 0;

		pBot->m_bMerchantState = MERCHANT_STATE_NONE;
		pBot->LastWarpTime = UNIXTIME + (Minute * MINUTE);
		pBot->m_pMap = GetZoneByID(byZone);
		pBot->m_bZone = byZone;
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		pBot->m_BotState = BOT_MOVE;
		pBot->m_bPartyLeader = false;
		pBot->m_bGenieStatus = false;
		pBot->m_bKnightsRank = pBotInfo->m_bKareli; //x6
		pBot->m_bPersonalRank = pBotInfo->m_bKaresiz; //x6
		if (fX <= 0.0f || fZ <= 0.0f)
		{
			short sx, sz;
			pBot->GetStartPosition(sx, sz);
			// Spawn more dispersed to avoid stacked/line movement around spawn anchors.
			fX = sx + (float)myrand(-80, 80);
			fZ = sz + (float)myrand(-80, 80);
		}

		pBot->isReset(false);
		pBot->SetBotAbility();
		pBot->SetPosition(fX, fY, fZ);
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		pBot->StateChangeServerDirect(1, USER_STANDING);

		//if (pBot->isInPKZone())
		if (byZone == ZONE_RONARK_LAND ||
			byZone == ZONE_RONARK_LAND_BASE ||
			byZone == ZONE_ARDREAM ||
			byZone >= ZONE_SPBATTLE1 && byZone <= ZONE_SPBATTLE12)
		{
			C3DMap* pMap = g_pMain->GetZoneByID(byZone);
			if (pMap == nullptr)
				continue;

			TRACE("%s In the Game Insert Player Ranking ZoneID %d\n", pBot->GetName().c_str(), pBot->GetZoneID());
			pBot->AddBotRank(pMap);
		}

		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);
		return pBot->GetID();

	}
	return true;
}

uint16 CGameServerDlg::SpawnUserBot(int Minute, uint8 byZone, float fX, float fY, float fZ, uint8 Restipi, uint8 minlevel /* = 1*/, int16 direction, uint32 SaveID, uint8 Class, _bot_merchant _merchant)
{

	struct _list {
		uint32 index;
		_MERCHANT_BOT_INFO pInfo;
		_list(uint32 index, _MERCHANT_BOT_INFO pInfo) {
			this->index = index;
			this->pInfo = pInfo;
		}
	};
	std::vector<_list> mList;

	m_ArtificialIntelligenceArray.m_lock.lock();
	auto m_sArtificialIntelligenceArray = m_ArtificialIntelligenceArray.m_UserTypeMap;
	m_ArtificialIntelligenceArray.m_lock.unlock();

	foreach(itr, m_sArtificialIntelligenceArray)
	{
		_BOT_DATA* pBotInfo = itr->second;
		if (pBotInfo == nullptr)
			continue;

		if (byZone <= ZONE_ELMORAD && byZone != pBotInfo->m_bNation
			|| (byZone >= ZONE_KARUS_ESLANT
				&& byZone <= ZONE_ELMORAD_ESLANT
				&& byZone != (pBotInfo->m_bNation + 10)))
			continue;

		if (pBotInfo->m_bLevel < minlevel)
			continue;

		// Check if bot already exist
		CBot* pBotCheck = GetBotPtr(pBotInfo->m_sSid);
		if (pBotCheck != nullptr)
			continue;

		CBot* pBot = new CBot();

		pBot->m_strUserID = pBotInfo->m_strUserID;
		pBot->m_bNation = pBotInfo->m_bNation;
		pBot->m_bRace = pBotInfo->m_bRace;
		pBot->m_sClass = pBotInfo->m_sClass;
		pBot->m_nHair = pBotInfo->m_nHair;
		pBot->m_bLevel = pBotInfo->m_bLevel;
		pBot->m_bFace = pBotInfo->m_bFace;
		pBot->m_bKnights = pBotInfo->m_bKnights;
		pBot->m_bFame = pBotInfo->m_bFame;

		memcpy(pBot->m_sItemArray, pBotInfo->m_sItemArray, sizeof(pBot->m_sItemArray));
		memcpy(pBot->m_bstrSkill, pBotInfo->m_bstrSkill, sizeof(pBot->m_bstrSkill));
		memcpy(pBot->m_bStats, pBotInfo->m_bStats, sizeof(pBot->m_bStats));

		pBot->m_sSid = pBotInfo->m_sSid;
		pBot->m_sAchieveCoverTitle = pBotInfo->m_sAchieveCoverTitle;
		pBot->m_reblvl = pBotInfo->m_reblvl;
		pBot->m_iGold = pBotInfo->m_iGold;
		pBot->m_sPoints = pBotInfo->m_sPoints;
		pBot->m_iLoyalty = pBotInfo->m_iLoyalty;
		pBot->m_iLoyaltyMonthly = pBotInfo->m_iLoyaltyMonthly;

		pBot->m_bMerchantState = (SaveID > 0 ? MERCHANT_STATE_SELLING : MERCHANT_STATE_NONE);
		pBot->LastWarpTime = 0;
		pBot->m_sMerchantAreaType = 0;

		if (Minute > 0)
			pBot->LastWarpTime = UNIXTIME + (Minute * 60);

		pBot->m_sDirection = direction;
		pBot->m_pMap = GetZoneByID(byZone);
		pBot->m_bZone = byZone;
		pBot->m_bMerchantIndex = 0;
		pBot->m_iGold = myrand(1000, 5000000);

		if (Restipi == 13 || Restipi == 14)
		{
			_ITEM_DATA* pItem = &pBot->m_sItemArray[RIGHTHAND];

			_ITEM_TABLE pTable = GetItemPtr(pItem->nNum);
			if (pTable.isnull())
				continue;

			if (Class == 1 && !pBot->isWarrior())
				continue;

			if (Class == 2 && !pBot->isRogue())
				continue;

			if (Class == 3 && !pBot->isMage())
				continue;

			if (Class == 4 && !pBot->isPriest())
				continue;

			if (pBot->isRogue() && !pTable.isBow())
				continue;

			if (pBot->isWarrior()
				|| pTable.isShield()
				|| pTable.isPickaxe()
				|| pTable.isFishing())
				continue;

			pBot->m_bGenieStatus = 1;

			if (Restipi == 14)
				pBot->m_bPartyLeader = true;
			else
				pBot->m_bPartyLeader = false;
		}
		else if (Restipi == 50)
		{
			_BOT_SAVE_DATA* pAuto = g_pMain->m_BotSaveDataArray.GetData(SaveID);
			if (pAuto == nullptr)
				return 0;

			uint16 bResult = 1;
			uint8 MerchantItemleri = 0;
			for (int i = 0; i < MAX_MERCH_ITEMS; i++) { if (pAuto->nNum[i] != 0)					MerchantItemleri++; }

			if (MerchantItemleri == 0)
				return false;

			_MERCH_DATA	m_arNewItems[MAX_MERCH_ITEMS]{};

			if (pAuto->sMerchanType == 0)
			{
				Packet result(WIZ_MERCHANT, uint8(MERCHANT_INSERT));

				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
				{
					int8 sItemSlot = pBot->FindSlotForItem(pAuto->nNum[i], pAuto->sCount[i]);
					if (sItemSlot < 0)
						continue;

					auto* pData = pBot->GetItem(sItemSlot);
					if (!pData
						|| pData->nNum != 0)
						continue;

					pData->nNum = pAuto->nNum[i];
					pData->sCount = pAuto->sCount[i];
					pData->sDuration = pAuto->sDuration[i];
					pData->nSerialNum = pAuto->nSerialNum[i];
					pData->MerchItem = true;

					m_arNewItems[i].sCount = pAuto->sCount[i];
					m_arNewItems[i].bCount = pAuto->sCount[i];
					m_arNewItems[i].orgcount = pAuto->sCount[i];
					m_arNewItems[i].nNum = pAuto->nNum[i];
					m_arNewItems[i].IsSoldOut = pAuto->IsSoldOut[i];
					m_arNewItems[i].sDuration = pAuto->sDuration[i];
					m_arNewItems[i].nPrice = pAuto->nPrice[i];
					m_arNewItems[i].nSerialNum = pAuto->nSerialNum[i];
					m_arNewItems[i].bOriginalSlot = sItemSlot;
					m_arNewItems[i].isKC = pAuto->isKc[i];
				}

				uint8 reqcount = 0;
				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
					if (m_arNewItems[i].nNum)
						reqcount++;

				if (!reqcount)
					return false;

				uint8 nRandom = 3;

				if (!pAuto->AdvertMessage.empty())
					pBot->MerchantChat = string_format("%s(Location:%d,%d)", pAuto->AdvertMessage.c_str(), uint16(pAuto->fX), uint16(pAuto->fZ));
				else
					pBot->MerchantChat.clear();


				pBot->m_iLoyalty = myrand(3000, 5000);
				pBot->m_bPremiumMerchant = 0;
				pBot->m_bMerchantState = MERCHANT_STATE_SELLING;
				pBot->m_BotState = BOT_MERCHANT;



				result << bResult << pAuto->AdvertMessage << pBot->GetID()
					<< pBot->m_bPremiumMerchant;

				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
					pBot->m_arMerchantItems[i] = m_arNewItems[i];

				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
					result << pBot->m_arMerchantItems[i].nNum;

				pBot->SendToRegion(&result);
			}

			if (pAuto->sMerchanType == 1)
			{
				Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_REGION_INSERT));

				for (int i = 0; i < MAX_MERCH_ITEMS; i++) { pBot->m_arMerchantItems[i].nNum = pAuto->nNum[i];					pBot->m_arMerchantItems[i].sCount = pAuto->sCount[i];					pBot->m_arMerchantItems[i].nPrice = pAuto->nPrice[i];					pBot->m_arMerchantItems[i].sDuration = pAuto->sDuration[i];					pBot->m_arMerchantItems[i].isKC = pAuto->isKc[i]; }
				pBot->m_bMerchantState = MERCHANT_STATE_BUYING;
				pBot->m_BotState = BOT_MERCHANT;
				result << pBot->GetID();

				for (int i = 0; i < 4; i++)
					result << pBot->m_arMerchantItems[i].nNum;

				pBot->SendToRegion(&result);
			}
		}
		else if (Restipi == 1)
		{
			_ITEM_DATA* pItem = &pBot->m_sItemArray[RIGHTHAND];
			if (pItem)
			{
				auto pTable = GetItemPtr(myrand(0, 100) > 50 ? GOLDEN_MATTOCK : MATTOCK);
				if (pTable.isnull())
					continue;

				_ITEM_DATA* pTItem = &pBot->m_sItemArray[LEFTHAND];
				if (pTItem) memset(pTItem, 0x00, sizeof(_ITEM_DATA));

				memset(pItem, 0x00, sizeof(_ITEM_DATA));
				pItem->nNum = pTable.m_iNum;
				pItem->nSerialNum = GenerateItemSerial();
				pItem->sCount = 1;
				pItem->sDuration = pTable.m_sDuration;
				pBot->m_bResHpType = USER_MINING;
			}
		}
		else if (Restipi == 2)
		{
			_ITEM_DATA* pItem = &pBot->m_sItemArray[RIGHTHAND];
			if (pItem)
			{
				auto pTable = GetItemPtr(myrand(0, 100) > 50 ? GOLDEN_FISHING : FISHING);
				if (pTable.isnull())
					continue;

				auto* pTItem = &pBot->m_sItemArray[LEFTHAND];
				if (pTItem != nullptr)
					memset(pTItem, 0x00, sizeof(_ITEM_DATA));

				memset(pItem, 0x00, sizeof(_ITEM_DATA));
				pItem->nNum = pTable.m_iNum;
				pItem->nSerialNum = GenerateItemSerial();
				pItem->sCount = 1;
				pItem->sDuration = pTable.m_sDuration;
				pBot->m_bResHpType = USER_FLASHING;
			}
		}
		else if (Restipi == 3 || Restipi == 4)
			pBot->m_bResHpType = Restipi == 3 ? USER_STANDING : USER_SITDOWN;
		else if (Restipi == 5)
			pBot->m_bResHpType = USER_STANDING;// Random > 5000 ? USER_STANDING : USER_SITDOWN;
		else if (Restipi == 6) {}
		else continue;

		if (Restipi == 6)
		{
			pBot->m_bMerchantIndex = _merchant.index;
			pBot->m_sMerchantAreaType = _merchant.areaType;
			pBot->m_BotState = BOT_MERCHANT;

			uint8 itemcount = 0;
			for (int i = 0; i < MAX_MERCH_ITEMS; i++)
			{
				if (_merchant.merc[i].itemid)
					itemcount++;
			}

			_MERCH_DATA	m_arNewItems[MAX_MERCH_ITEMS]{};
			memset(m_arNewItems, 0, sizeof(m_arNewItems));

			if (!_merchant.isBuy) {

				for (int i = 0; i < MAX_MERCH_ITEMS; i++) {
					if (!_merchant.merc[i].itemid)
						continue;

					int8 sItemSlot = pBot->FindSlotForItem(_merchant.merc[i].itemid, _merchant.merc[i].count);
					if (sItemSlot < 0)
						sItemSlot = SLOT_MAX + i;

					auto pItem = g_pMain->GetItemPtr(_merchant.merc[i].itemid);
					if (pItem.isnull())
						continue;

					auto* pData = pBot->GetItem(sItemSlot);
					if (!pData)
						continue;

					if (pData->nNum != 0)
						memset(pData, 0x00, sizeof(_ITEM_DATA));

					pData->nNum = _merchant.merc[i].itemid;
					pData->sCount += _merchant.merc[i].count;
					pData->sDuration = _merchant.merc[i].pTable.m_sDuration;
					pData->nSerialNum = g_pMain->GenerateItemSerial();
					pData->MerchItem = true;

					m_arNewItems[i].sCount = pData->sCount;
					m_arNewItems[i].bCount = pData->sCount;
					m_arNewItems[i].orgcount = pData->sCount;
					m_arNewItems[i].nNum = pData->nNum;
					m_arNewItems[i].IsSoldOut = false;
					m_arNewItems[i].sDuration = pData->sDuration;
					m_arNewItems[i].nPrice = _merchant.merc[i].price;
					m_arNewItems[i].nSerialNum = pData->nSerialNum;
					m_arNewItems[i].bOriginalSlot = sItemSlot;
					m_arNewItems[i].isKC = _merchant.merc[i].iskc;
				}

				uint8 reqcount = 0;
				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
					if (m_arNewItems[i].nNum)
						reqcount++;

				if (!reqcount)
				{
					if (_merchant.index) {
						auto* pCoord = g_pMain->pBotInfo.mCoordinate.GetData(_merchant.index);
						if (pCoord)
							pCoord->used = false;
					}
					return 0;
				}

				pBot->m_bPremiumMerchant = myrand(0, 100) < 15;
				pBot->m_bMerchantState = MERCHANT_STATE_SELLING;
			}
			else {

				uint32 total_price = 0;
				for (int i = 0; i < MAX_MERCH_ITEMS; i++) {
					m_arNewItems[i].sCount = _merchant.merc[i].count;
					m_arNewItems[i].bCount = _merchant.merc[i].count;
					m_arNewItems[i].orgcount = _merchant.merc[i].count;
					m_arNewItems[i].nNum = _merchant.merc[i].itemid;
					m_arNewItems[i].sDuration = _merchant.merc[i].pTable.m_sDuration;
					m_arNewItems[i].nPrice = _merchant.merc[i].price;
					m_arNewItems[i].isKC = _merchant.merc[i].iskc;
					m_arNewItems[i].bOriginalSlot = i;
					total_price += _merchant.merc[i].price;
				}

				uint8 reqcount = 0;
				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
					if (m_arNewItems[i].nNum)
						reqcount++;

				if (!reqcount)
				{
					if (_merchant.index) {
						auto* pCoord = g_pMain->pBotInfo.mCoordinate.GetData(_merchant.index);
						if (pCoord)
							pCoord->used = false;
					}
					return 0;
				}

				pBot->m_bPremiumMerchant = myrand(0, 100) < 15;
				pBot->m_bMerchantState = MERCHANT_STATE_BUYING;

				if (pBot->m_iGold < total_price)
					pBot->m_iGold = myrand(total_price, total_price + 5000000);
			}

			for (int i = 0; i < MAX_MERCH_ITEMS; i++)
				pBot->m_arMerchantItems[i] = m_arNewItems[i];

			pBot->MerchantChat = string_format("Merchant Bot(Location:%d,%d)", uint16(fX), uint16(fZ));
		}

		int Random = myrand(0, 10000);
		pBot->m_sTargetID = int16(-1);
		pBot->m_TargetChanged = false;
		pBot->m_bBlockPrivateChat = false;
		if (Restipi != 6)
			pBot->m_BotState = BOT_AFK;
		pBot->SetBotAbility();
		pBot->SetPosition(fX, fY, fZ);
		pBot->SetRegion(pBot->GetNewRegionX(), pBot->GetNewRegionZ());
		pBot->SetZoneAbilityChange(pBot->GetZoneID());
		AddMapBotList(pBot);
		pBot->UserInOut(INOUT_IN);

		if (Restipi == 6)
		{
			if (!_merchant.isBuy)
			{
				Packet result(WIZ_MERCHANT, uint8(MERCHANT_INSERT));
				std::string advertMessage = "Merchant Bot";
				result << uint16(1) << advertMessage << pBot->GetID() << pBot->m_bPremiumMerchant;
				for (int i = 0; i < MAX_MERCH_ITEMS; i++)
					result << pBot->m_arMerchantItems[i].nNum;

				pBot->SendToRegion(&result);
			}
			else
			{
				Packet result(WIZ_MERCHANT, uint8(MERCHANT_BUY_REGION_INSERT));
				result << pBot->GetID();
				for (int i = 0; i < 4; i++)
					result << pBot->m_arMerchantItems[i].nNum;

				pBot->SendToRegion(&result);
			}
		}

		pBot->StateChangeServerDirect(1, Random > 5000 ? USER_STANDING : USER_SITDOWN);
		return pBot->GetID();
	}
	return 0;
}

void CGameServerDlg::BotHandlerMainTimer()
{
	try
	{
		DWORD checknow = GetTickCount();
		time_t dwDiffTime = 0, dwTickTime = 0, fTime2 = 0, fType4Time = 0;
		std::vector<CBot*> willBeOut;
		fTime2 = getMSTime(); // the current time
		foreach_stlmap_nolock(itr, m_MapBotList)// Update bot sessions
		{
			CBot* pBot = itr->second;
			if (pBot == nullptr)
				continue;

			if (!pBot->isInGame())
				continue;

			if (pBot->LastWarpTime > 0)
			{
				if (pBot->LastWarpTime < UNIXTIME)
				{
					willBeOut.push_back(pBot);
					continue;
				}
			}

			if (pBot->m_tGameStartTimeSavedMagic != 0 && (UNIXTIME - pBot->m_tGameStartTimeSavedMagic) >= 2)
			{
				pBot->m_tGameStartTimeSavedMagic = 0;

				// Restore scrolls...
				pBot->InitType4();
				pBot->RecastSavedMagic();

				if (pBot->isInPKZone())
					pBot->Type4Change();
			}

			if (pBot->isRegionTargetUp())
				pBot->RegionFindAttackProcess();

			dwTickTime = fTime2 - pBot->m_fHPChangeTime;
			if (2 * SECOND < dwTickTime)
				pBot->HpMpChange();

			if (pBot->hasRival()
				&& pBot->hasRivalryExpired())
				pBot->RemoveRival();

			pBot->BotPartyInviteProcess();
			pBot->FarmBotPartyFollowProcess();

			if (pBot->ReplyStatus == 1 && checknow > pBot->ReplyTime)
			{
				CUser* pUser;
				pUser = g_pMain->GetUserPtr(pBot->ReplyID);
				if (pUser != nullptr)
				{
					std::string strUserID;
					std::string PMdetay = string_format("%s", pBot->ReplyChat.c_str());
					strUserID = pBot->GetName();
					Packet result1;
					ChatPacket::Construct(&result1, PRIVATE_CHAT, &PMdetay, &strUserID, pUser->GetNation());
					pUser->Send(&result1);
				}

				pBot->ReplyTime = 0;
				pBot->ReplyStatus = 0;
				pBot->ReplyID = 0;
				pBot->ReplyChat = "";
			}

			switch (pBot->GetBotState())
			{
			case BOT_MINING:
				pBot->BotMining();
				break;
			case BOT_FISHING:
				pBot->BotFishing();
				break;
			case BOT_MERCHANT:
				pBot->BotMerchant();
				break;
			case BOT_DEAD:
				pBot->Regene(INOUT_IN, pBot->isInPKZone() ? 0 : 112754);
				break;
			case BOT_FARMER:
			case BOT_FARMERS:
				pBot->FindMonsterAttackSlot();
				break;
			case BOT_MOVE:
				pBot->MoveProcessGoDeahTown();
				break;
			case BOT_MERCHANT_MOVE:
				pBot->MerchantMoveProcess();
				break;
			case BOT_AFK:
				break;
			}

			// This may not be necessary, but it keeps behaviour identical.
			if (pBot->GetBotState() != BOT_DEAD)
				pBot->m_fDelayTime = getMSTime();

			time_t dwTickTimeType4 = fTime2 - pBot->m_fHPType4CheckTime;
			if (1 * SECOND < dwTickTimeType4 && pBot->isAlive())
			{
				pBot->HPTimeChangeType3();
				pBot->Type4Duration();
				pBot->CheckSavedMagic();
				pBot->m_fHPType4CheckTime = getMSTime();
			}

			if (pBot->isInPKZone())
			{
				float nMaxSpeed = 34.0f;

				if (pBot->GetFame() == COMMAND_CAPTAIN
					|| pBot->isRogue())
					nMaxSpeed = 67.0f;
				else if (pBot->isWarrior()
					|| pBot->isMage()
					|| pBot->isPriest())
					nMaxSpeed = 50.0f;
				else if (pBot->isPortuKurian())
					nMaxSpeed = 50.0f;

				pBot->m_sSpeed = nMaxSpeed;
			}
			else
				pBot->m_sSpeed = 34.0f;
		}

		if (willBeOut.size() > 0)
		{
			foreach(itr, willBeOut)
			{
				(*itr)->UserInOut(INOUT_OUT);
				g_pMain->RemoveMapBotList((*itr)->GetID(), (*itr)->GetName());
			}
		}
	}
	
	catch (std::exception& ex)
	{
#ifdef _DEBUG
		std::string information = string_format("Fixed a Critical Error with Bot Theard system: %s\n", ex.what());
		printf(information.c_str());
		ASSERT(0); /* fix me */
#endif
	}
}
