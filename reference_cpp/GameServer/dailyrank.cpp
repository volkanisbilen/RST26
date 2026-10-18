#include "stdafx.h"
#include "DBAgent.h"

void CUser::HandleDailyRank(Packet& pkt)
{
	if (!isInGame()) return;

	uint8 Opcode = pkt.read<uint8>();
	switch (Opcode)
	{
	case 1:
		HandleDailyRankShow(pkt);
		break;
	default:
		TRACE("HandleDailyRank unhandled opcode %d \n", Opcode);
		printf("HandleDailyRank unhandled opcode %d \n", Opcode);
		break;
	}
}

void CUser::HandleDailyRankShow(Packet& pkt)
{
	uint8 RankType = pkt.read<uint8>();

	Packet newpkt(WIZ_DAILYRANK, uint8(1));
	newpkt << RankType;

	struct _DAILY { std::string strUserID = ""; uint32 Rank = 0; _DAILY(std::string a, uint32 b) { strUserID = a; Rank = b; } };
	std::vector<_DAILY> pDailyList;

	switch ((DailyRankType)RankType)
	{
	case DailyRankType::GRAND_MERCHANT:
	{
		if (m_dailyrankrefresh[(int)DailyRankType::GRAND_MERCHANT]) return;
		g_pMain->m_DailyRank.m_lock.lock();
		foreach_stlmap_nolock(ita, g_pMain->m_DailyRank)
			if (ita->second)
				pDailyList.emplace_back(_DAILY(ita->second->strUserID, ita->second->GmRank[0]));
		g_pMain->m_DailyRank.m_lock.unlock();

		if (pDailyList.empty()) { newpkt << uint32(0) << uint32(0) << uint16(0); Send(&newpkt); return; }
		std::sort(pDailyList.begin(), pDailyList.end(), [](_DAILY const& a, _DAILY const& b) { return a.Rank < b.Rank; });

		auto* pShow = g_pMain->m_DailyRank.GetData(GetName());
		uint32 Rank = pShow == nullptr ? 0 : pShow->GmRank[0]; int32 Diff = pShow == nullptr ? 0 : pShow->GmRank[1] - pShow->GmRank[0];

		newpkt.ByteBuffer::SByte(); newpkt << Rank << Diff;

		uint16 Count = 0;
		size_t wpos = newpkt.wpos();
		newpkt << Count;
		foreach(text, pDailyList) { if (Count >= 100) break; newpkt << text->strUserID; Count++; }
		newpkt.put(wpos, Count);
		wpos = newpkt.wpos();
		Send(&newpkt);
	}
	break;
	case DailyRankType::MONSTER_HUNDER:
	{
		if (m_dailyrankrefresh[(int)DailyRankType::MONSTER_HUNDER]) return;
		g_pMain->m_DailyRank.m_lock.lock();
		foreach_stlmap_nolock(ita, g_pMain->m_DailyRank)
			if (ita->second)
				pDailyList.emplace_back(_DAILY(ita->second->strUserID, ita->second->MhRank[0]));
		g_pMain->m_DailyRank.m_lock.unlock();

		if (pDailyList.empty()) { newpkt << uint32(0) << uint32(0) << uint16(0); Send(&newpkt); return; }
		std::sort(pDailyList.begin(), pDailyList.end(), [](_DAILY const& a, _DAILY const& b) { return a.Rank < b.Rank; });

		auto* pShow = g_pMain->m_DailyRank.GetData(GetName());
		uint32 Rank = pShow == nullptr ? 0 : pShow->MhRank[0]; int32 Diff = pShow == nullptr ? 0 : pShow->MhRank[1] - pShow->MhRank[0];

		newpkt.ByteBuffer::SByte(); newpkt << Rank << Diff;

		uint16 Count = 0;
		size_t wpos = newpkt.wpos();
		newpkt << Count;
		foreach(text, pDailyList) { if (Count >= 100) break; newpkt << text->strUserID; Count++; }
		newpkt.put(wpos, Count);
		wpos = newpkt.wpos();
		Send(&newpkt);
	}
	break;
	case DailyRankType::SHOZIN:
	{
		if (m_dailyrankrefresh[(int)DailyRankType::SHOZIN]) return;
		g_pMain->m_DailyRank.m_lock.lock();
		foreach_stlmap_nolock(ita, g_pMain->m_DailyRank)
			if (ita->second)
				pDailyList.emplace_back(_DAILY(ita->second->strUserID, ita->second->ShRank[0]));
		g_pMain->m_DailyRank.m_lock.unlock();

		if (pDailyList.empty()) { newpkt << uint32(0) << uint32(0) << uint16(0); Send(&newpkt); return; }
		std::sort(pDailyList.begin(), pDailyList.end(), [](_DAILY const& a, _DAILY const& b) { return a.Rank < b.Rank; });

		auto* pShow = g_pMain->m_DailyRank.GetData(GetName());
		uint32 Rank = pShow == nullptr ? 0 : pShow->ShRank[0]; int32 Diff = pShow == nullptr ? 0 : pShow->ShRank[1] - pShow->ShRank[0];

		newpkt.ByteBuffer::SByte(); newpkt << Rank << Diff;

		uint16 Count = 0;
		size_t wpos = newpkt.wpos();
		newpkt << Count;
		foreach(text, pDailyList) { if (Count >= 100) break; newpkt << text->strUserID; Count++; }
		newpkt.put(wpos, Count);
		wpos = newpkt.wpos();
		Send(&newpkt);
	}
	break;
	case DailyRankType::KNIGHT_ADONIS:
	{
		if (m_dailyrankrefresh[(int)DailyRankType::KNIGHT_ADONIS]) return;
		g_pMain->m_DailyRank.m_lock.lock();
		foreach_stlmap_nolock(ita, g_pMain->m_DailyRank)
			if (ita->second)
				pDailyList.emplace_back(_DAILY(ita->second->strUserID, ita->second->AkRank[0]));
		g_pMain->m_DailyRank.m_lock.unlock();

		if (pDailyList.empty()) { newpkt << uint32(0) << uint32(0) << uint16(0); Send(&newpkt); return; }
		std::sort(pDailyList.begin(), pDailyList.end(), [](_DAILY const& a, _DAILY const& b) { return a.Rank < b.Rank; });

		auto* pShow = g_pMain->m_DailyRank.GetData(GetName());
		uint32 Rank = pShow == nullptr ? 0 : pShow->AkRank[0]; int32 Diff = pShow == nullptr ? 0 : pShow->AkRank[1] - pShow->AkRank[0];

		newpkt.ByteBuffer::SByte(); newpkt << Rank << Diff;

		uint16 Count = 0;
		size_t wpos = newpkt.wpos();
		newpkt << Count;
		foreach(text, pDailyList) { if (Count >= 100) break; newpkt << text->strUserID; Count++; }
		newpkt.put(wpos, Count);
		wpos = newpkt.wpos();
		Send(&newpkt);
	}
	break;
	case DailyRankType::HERO_OF_CHAOS:
	{
		if (m_dailyrankrefresh[(int)DailyRankType::HERO_OF_CHAOS]) return;
		g_pMain->m_DailyRank.m_lock.lock();
		foreach_stlmap_nolock(ita, g_pMain->m_DailyRank)
			if (ita->second)
				pDailyList.emplace_back(_DAILY(ita->second->strUserID, ita->second->CwRank[0]));
		g_pMain->m_DailyRank.m_lock.unlock();

		if (pDailyList.empty()) { newpkt << uint32(0) << uint32(0) << uint16(0); Send(&newpkt); return; }
		std::sort(pDailyList.begin(), pDailyList.end(), [](_DAILY const& a, _DAILY const& b) { return a.Rank < b.Rank; });

		auto* pShow = g_pMain->m_DailyRank.GetData(GetName());
		uint32 Rank = pShow == nullptr ? 0 : pShow->CwRank[0]; int32 Diff = pShow == nullptr ? 0 : pShow->CwRank[1] - pShow->CwRank[0];

		newpkt.ByteBuffer::SByte(); newpkt << Rank << Diff;

		uint16 Count = 0;
		size_t wpos = newpkt.wpos();
		newpkt << Count;
		foreach(text, pDailyList) { if (Count >= 100) break; newpkt << text->strUserID; Count++; }
		newpkt.put(wpos, Count);
		wpos = newpkt.wpos();
		Send(&newpkt);
	}
	break;
	case DailyRankType::DISCIPLE_KERON:
	{
		if (m_dailyrankrefresh[(int)DailyRankType::DISCIPLE_KERON]) return;
		g_pMain->m_DailyRank.m_lock.lock();
		foreach_stlmap_nolock(ita, g_pMain->m_DailyRank)
			if (ita->second)
				pDailyList.emplace_back(_DAILY(ita->second->strUserID, ita->second->UpRank[0]));
		g_pMain->m_DailyRank.m_lock.unlock();

		if (pDailyList.empty()) { newpkt << uint32(0) << uint32(0) << uint16(0); Send(&newpkt); return; }
		std::sort(pDailyList.begin(), pDailyList.end(), [](_DAILY const& a, _DAILY const& b) { return a.Rank < b.Rank; });

		auto* pShow = g_pMain->m_DailyRank.GetData(GetName());
		uint32 Rank = pShow == nullptr ? 0 : pShow->UpRank[0]; int32 Diff = pShow == nullptr ? 0 : pShow->UpRank[1] - pShow->UpRank[0];

		newpkt.ByteBuffer::SByte(); newpkt << Rank << Diff;

		uint16 Count = 0;
		size_t wpos = newpkt.wpos();
		newpkt << Count;
		foreach(text, pDailyList) { if (Count >= 100) break; newpkt << text->strUserID; Count++; }
		newpkt.put(wpos, Count);
		wpos = newpkt.wpos();
		Send(&newpkt);
	}
	break;
	case DailyRankType::KING_OF_FELANKOR:
	{
		if (m_dailyrankrefresh[(int)DailyRankType::KING_OF_FELANKOR]) return;
		struct _clanlist {
			std::string clanname = ""; uint32 points = 0; uint16 clanid; _clanlist(std::string a, uint32 b, uint16 c) {
				clanname = a; points = b; clanid = c;
			}
		};
		std::vector<_clanlist> pclanlist;

		for (int nation = KARUS_ARRAY; nation <= ELMORAD_ARRAY; nation++) {
			g_pMain->m_KnightsRatingArray[nation].m_lock.lock();
			foreach_stlmap_nolock(itr, g_pMain->m_KnightsRatingArray[nation]) {
				if (!itr->second) continue;
				if (CKnights *pknight = g_pMain->GetClanPtr(itr->second->sClanID))
					pclanlist.push_back(_clanlist(pknight->GetName(), itr->second->nPoints, itr->second->sClanID));
			}
			g_pMain->m_KnightsRatingArray[nation].m_lock.unlock();
		}
		if (pclanlist.empty()) { newpkt << uint32(0) << uint32(0) << uint16(0); Send(&newpkt); return; }
		std::sort(pclanlist.begin(), pclanlist.end(), [](_clanlist const& a, _clanlist const& b) { return a.points > b.points; });

		uint32 rank = 0;
		if (isInClan()) {
			int count = 0; bool found = false;
			foreach(text, pclanlist) {
				if (GetClanID() == text->clanid) {found = true;break;}
				count++; 
			}
			if (found) rank = count;
		}

		newpkt.ByteBuffer::SByte(); newpkt << rank << uint32(0);

		uint16 Count = 0;
		size_t wpos = newpkt.wpos();
		newpkt << Count;
		foreach(text, pclanlist) { if (Count >= 100) break; newpkt << text->clanname; Count++; }
		newpkt.put(wpos, Count);
		wpos = newpkt.wpos();
		Send(&newpkt);
	}
	break;
	case DailyRankType::DRAKI_RANK:
		if (m_dailyrankrefresh[(int)DailyRankType::DRAKI_RANK]) return;
		g_pMain->AddDatabaseRequest(newpkt, this);
		break;
	default:
		TRACE("HandleDailyRankShow unhandled RankType %d \n", RankType);
		printf("HandleDailyRankShow unhandled RankType %d \n", RankType);
		break;
	}
}

void CUser::ReqHandleDailyRank(Packet& pkt)
{
	uint8 Opcode, RankType;
	pkt >> Opcode >> RankType;
	switch ((DailyRankType)RankType)
	{
	case DailyRankType::DRAKI_RANK:
	{
		Packet DailyPacket(WIZ_DAILYRANK, uint8(1));
		DailyPacket << RankType;

		std::vector<_DRAKI_TOWER_FORDAILY_RANKING> DrakiTowerRanking;
		if (!g_DBAgent.LoadDrakiTowerDailyRank(DrakiTowerRanking)) {
			DailyPacket << uint32(0) << uint16(0);
			Send(&DailyPacket);
			return;
		}

		if (DrakiTowerRanking.size() <= 0)
		{
			DailyPacket << uint32(0) << uint16(0);
			Send(&DailyPacket);
			return;
		}

		struct _DRAKI_TOWER_LIST { std::string strUserID = ""; uint32 DrakiTime = 0; uint8 DrakiStage = 0; };
		std::vector<_DRAKI_TOWER_LIST> pDrakiRankList;

		uint32 PlayerClass = 0;
		if (isWarrior())	 PlayerClass = 1;
		if (isRogue())		 PlayerClass = 2;
		if (isMage())		 PlayerClass = 3;
		if (isPriest())		 PlayerClass = 4;
		if (isPortuKurian()) PlayerClass = 13;

		foreach(itr, DrakiTowerRanking) {
			if (itr->Class == PlayerClass) {
				_DRAKI_TOWER_LIST pList;
				pList.strUserID = itr->strUserID;
				pList.DrakiStage = itr->DrakiStage;
				pList.DrakiTime = itr->DrakiTime;
				pDrakiRankList.push_back(pList);
			}
		}

		if (pDrakiRankList.size() <= 0) {
			DailyPacket << uint32(0) << uint16(0);
			Send(&DailyPacket);
			return;
		}

		std::sort(pDrakiRankList.begin(), pDrakiRankList.end(), [](_DRAKI_TOWER_LIST const& a, _DRAKI_TOWER_LIST const& b)
		{ return a.DrakiStage == b.DrakiStage ? a.DrakiTime < b.DrakiTime : a.DrakiStage > b.DrakiStage; });

		uint32 MyRank = 0;
		foreach(test, pDrakiRankList) {
			MyRank++;
			if (test->strUserID == GetName()) break;
		}

		DailyPacket.SByte();
		DailyPacket << uint32(MyRank);

		uint16 Count = 0;
		size_t wpos = DailyPacket.wpos();
		DailyPacket << Count;

		foreach(test, pDrakiRankList) {
			if (Count >= 100) break;
			DailyPacket << test->strUserID;
			Count++;
		}
		DailyPacket.put(wpos, Count);
		wpos = DailyPacket.wpos();
		Send(&DailyPacket);
	}
	break;
	}
}