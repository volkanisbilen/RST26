#include "stdafx.h"
#include "DBAgent.h"

using std::string;
using std::unique_ptr;
extern CDBAgent g_DBAgent;

#pragma region CUser::HandleGenie(Packet & pkt)
void CUser::HandleGenie(Packet & pkt)
{
	uint8 command = pkt.read<uint8>();

	switch (command)
	{
	case GenieInfoRequest:
		GenieNonAttackProgress(pkt);
		break;
	case GenieUpdateRequest:
		GenieAttackProgress(pkt);
		break;
	case 25:
		GenieNotice(pkt);
		break;
	default:
		//printf("Genie OpCode %u \n", command);				// Hata ! Buras? s?rekli uyar? veriyor. Opcode 3 veriyor s?rekli.
		TRACE("Genie OpCode %u \n", command);
		break;
	}
}
#pragma endregion

void CUser::GenieNotice(Packet& pkt)
{
	bool status = pkt.read<bool>();
	std::string notice = "Karakteriniz Genie ile hi? bir ?ekilde y?r?me fonksiyonu ger?ekle?tirmez.";
	if (status) notice = "Y?r?me fonksiyonu aktif edilmi?tir.";
	g_pMain->SendGameNotice(ChatType::GENERAL_CHAT, notice, "", false, this, false);
}

#pragma region CUser::GenieNonAttackProgress(Packet & pkt)
void CUser::GenieNonAttackProgress(Packet & pkt)
{
	uint8 command = pkt.read<uint8>();

	switch (command)
	{
	case GenieUseSpiringPotion:
		GenieUseGenieSpirint(pkt);
		break;
	case GenieLoadOptions:
		HandleGenieLoadOptions();
		break;
	case GenielSaveOptions:
		HandleGenieSaveOptions(pkt);
		break;
	case GenieStartHandle:
		GenieStart();
		break;
	case GenieStopHandle:
		GenieStop();
		break;
	default:
		TRACE("GenieNonAttackProgress Unknow Attack Handle %d\n", command);
		//printf("GenieNonAttackProgress Unknow Attack Handle %d\n", command);		// Hata ! Buras? s?rekli uyar? veriyor.  - Handle 155 & 22 & 130 & 90 & 87 Uyar?s?
		break;
	}

}
#pragma endregion

#pragma region CUser::HandleGenieLoadOptions()
void CUser::HandleGenieLoadOptions()
{
	// JstKO Genie se?eneklerinin y?klenmeye ba?land???n? bildirelim
	//g_pMain->SendHelpDescription(this, "Genie Se?enekleri Y?kleniyor...");

	Packet result(WIZ_GENIE, uint8(GenieInfoRequest));
	result << uint8(GenieLoadOptions);

	for (int i = 0; i < sizeof(m_GenieOptions); i++) {
		result << uint8(*(uint8*)(m_GenieOptions + i));
	}

	// JstKO Se?enekler ba?ar?yla g?nderildi
	Send(&result);

	// JstKO Kullan?c?ya ba?ar? mesaj? g?nder
	//g_pMain->SendHelpDescription(this, "Genie Se?enekleri Ba?ar?yla Y?klendi."); 
}
#pragma endregion

#pragma region CUser::HandleGenieSaveOptions(Packet & pkt)
void CUser::HandleGenieSaveOptions(Packet& pkt)
{
	// JstKO Gelen paket uzunlu?unu kontrol edelim
	if (pkt.size() < sizeof(m_GenieOptions)) 
	{
		//g_pMain->SendHelpDescription(this, "Genie Se?enekleri Kaydedilemedi : Eksik Veri.");
		return;
	}

	// JstKO Genie se?eneklerini paket i?eri?inden oku
	for (int i = 0; i < sizeof(m_GenieOptions); i++) 
	{
		m_GenieOptions[i] = pkt.read<uint8>();
	}

	// JstKO Kaydedildi?ini kullan?c?ya bildir
	//g_pMain->SendHelpDescription(this, "Genie Se?enekleri Ba?ar?yla Kaydedildi.");
}
#pragma endregion

#pragma region CUser::GenieAttackProgress(Packet & pkt)
void CUser::GenieAttackProgress(Packet& pkt)
{
	uint8 command = pkt.read<uint8>();

	// JstKO S?re dolmu?sa Genie'yi durdur ve kullan?c?ya haber ver
	if (UNIXTIME > m_1098GenieTime) 
	{
		g_pMain->SendHelpDescription(this, "Genie S?resi Doldu, Sald?r? Durduruluyor.");
		return SendGenieStop(true);
	}

	switch (command)
	{
	case GenieMove:
		MoveProcess(pkt);
		break;

	case GenieRotate:
		Rotate(pkt);
		break;

	case GenieMainAttack:
		Attack(pkt);
		break;

	case GenieMagic:
		CMagicProcess::MagicPacket(pkt, this);
		break;

	default:
		// JstKO Bilinmeyen komut tespit edildi?inde oyuncuya Notice g?nder
		TRACE("Genie Unknown Attack Handle %d\n", command);
		break;
	}
}
#pragma endregion

#pragma region CUser::GenieStart()
void CUser::GenieStart()
{
	// JstKO Premium kontrol? veya Genie s?resi dolmu?sa izin verme. 27.04.2025
	if ((g_pMain->pServerSetting.LootandGeniePremium && GetPremium() == 0) || UNIXTIME > m_1098GenieTime)
	{
		// JstKO Hile korumas? : Premium yoksa ya da s?resi bitmi?se ve Genie a?maya ?al???yorsa.
		TRACE("[GEN?E HACK] = Oyuncu : [ %s ] Genie Ba?latmaya ?al?s?yor Dc Edildi! Siktir Git.\n", GetName().c_str());
		g_pMain->SendHelpDescription(this, string_format("[GEN?E HACK] = Oyuncu : [ %s ] ?zinsiz Genie A?maya ?al?s?yor Dc Edildi! Siktir Git.", GetName().c_str()));
		Disconnect();
		return;
	}

	// JstKO Normal do?ru Genie a??l???
	Packet result(WIZ_GENIE, uint8(GenieStatusActive));
	result << uint8(4) << uint16(1) << GetGenieTime();
	m_bGenieStatus = true;
	Send(&result);
	SendGenieStart(true);

	bool NoticeGenie = true;

	// JstKO Genie Ba?l?kl? ve Renkli Notice Ekliyoruz. 27.04.2025
	if (NoticeGenie)
	{
		uint32 genieTime = GetGenieTime(); // JstKO Genie s?resini al?yoruz.
		g_pMain->SendHelpDescription(this, string_format("[GEN?E AKT?F] = Genie Kalan S?resi : [ -%d ] Kald?. Ba?ar?yla Aktif Edildi.", genieTime));
	}
}
#pragma endregion

#pragma region CUser::SendGenieStart(bool isToRegion /* = false */)
void CUser::SendGenieStart(bool isToRegion /* = false */)
{
	Packet result(WIZ_GENIE, uint8(GenieInfoRequest));
	result << uint8(GenieStartHandle) << uint16(1) << GetGenieTime();
	Send(&result);
	Packet newpkt(XSafe, uint8(0xDB));
	newpkt << uint8(1);
	Send(&newpkt);
	Packet result2(WIZ_GENIE, uint8(GenieInfoRequest));
	result2 << uint8(GenieActivated) << uint16(GetID()) << uint8(1);

	if (isToRegion) SendToRegion(&result2, nullptr, GetEventRoom());
	else Send(&result2);
	
	if (m_bGenieStatus == GenieStatusInactive) GenieStop();
	//SetOffCha(_choffstatus::ACTIVE, offcharactertype::genie);
}
#pragma endregion

#pragma region CUser::GenieStop()
void CUser::GenieStop()
{
	if (m_bGenieStatus == GenieStatusInactive) return;

	Packet newpkt(XSafe, uint8(0xDB));
	newpkt << uint8(0);
	Send(&newpkt);

	Packet result(WIZ_GENIE, uint8(1));
	result << uint8(5) << uint16(1) << GetGenieTime();
	m_bGenieStatus = false;
	SendGenieStop(true);
	Send(&result);

	{
		Packet newpkt(XSafe, uint8(0xDB));
		newpkt << uint8(0);
		Send(&newpkt);
	}

	bool NoticeGenie = true;

	// JstKO Genie Ba?l?kl? Notice Ekliyoruz. 27.04.2025
	if (NoticeGenie)
	{
		// JstKO Genie i?lemi Durdurulurken, i?lemin s?resiyle ilgili bilgiyi i?eren Notice ekliyoruz.
		uint32 genieTime = GetGenieTime();  // JstKO Genie s?resini al?yoruz.
		g_pMain->SendHelpDescription(this, string_format("[GEN?E DURDUR] = Genie Durduruluyor. Kalan S?resi : [ -%d ] Kald?.", genieTime));  // JstKO Genie s?resi, saniye cinsinden g?steriliyor.
	}
}
#pragma endregion

#pragma region CUser::SendGenieStop()
void CUser::SendGenieStop(bool isToRegion /* = false */)
{
	Packet result(WIZ_GENIE, uint8(GenieInfoRequest));
	if (GetGenieTime() > 0) 
		result << uint8(GenieStopHandle) << uint16(1) << GetGenieTime();
	Send(&result);
	Packet newpkt(XSafe, uint8(0xDB));
	newpkt << uint8(0);
	Send(&newpkt);
	Packet result2(WIZ_GENIE, uint8(GenieInfoRequest));
	result2 << uint8(GenieActivated) << uint16(GetID()) << uint8(0);
	if (isToRegion) SendToRegion(&result2, nullptr, GetEventRoom());
	else Send(&result2);
	//SetOffCha(_choffstatus::DEACTIVE, offcharactertype::genie);
}
#pragma endregion

#pragma region CUser::UpdateGenieTime(uint16 m_sTime)
void CUser::UpdateGenieTime(uint16 m_sTime)
{
	Packet result(WIZ_GENIE, uint8(GenieInfoRequest));
	result << uint8(GenieRemainingTime) << m_sTime;
	Send(&result);

	bool NoticeGenie = true;

	// JstKO Genie S?re Bitti?inde Notice Ekliyoruz. 27.04.2025
	if (m_sTime == GenieStatusInactive)
	{
		g_pMain->SendHelpDescription(this, string_format("[GEN?E S?RE B?TT?] = Genie S?reniz Doldu : [ -%d ], Sona Erdi.", m_sTime)); // Genie bitti?inde mesaj Eklendi.
		GenieStop();
	}
    else 
	{
		g_pMain->SendHelpDescription(this, string_format("[GEN?E KALAN] = Kullan?c? : %s Genie Kalan S?resi : [ -%d ] Ka? Kald?.", GetName().c_str(), m_sTime));
    }
}
#pragma endregion

#pragma region CUser::GenieUseGenieSpirint()
void CUser::GenieUseGenieSpirint(Packet & pkt)
{

#if 0
	if (isTrading() || isMerchanting() || isMining() || isFishing()) return;

	uint32 nItemID; uint16 GenieItem;
	pkt >> nItemID;

	_ITEM_TABLE* ItemTable = g_pMain->GetItemPtr(nItemID);
	if (ItemTable == nullptr)
		return;

	GenieItem = GetItemCount(nItemID);

	if (!CheckExistItem(nItemID))
		return;

	if (nItemID != 810305000 && nItemID != 810378000  && nItemID != 900772000)
		return;

	if (RobItem(nItemID))
		m_GenieTime = 120;

	if (m_sFirstUsingGenie <= 0)
		m_sFirstUsingGenie = 1;

	Packet result(WIZ_GENIE, uint8(GenieUseSpiringPotion));
	result << uint8(GenieUseSpiringPotion) << GetGenieTime();
	Send(&result);
#endif // 0
}
#pragma endregion

#pragma region CDBAgent::UpdateGenieData(string& strCharID, CUser* pUser)
bool CDBAgent::UpdateGenieData(string& strCharID, CUser* pUser)
{
	// JstKO Karakter ID uyu?mazl???
	if (strCharID != pUser->GetName()) 
	{
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, ("Genie Verisi G?ncellemesi Ba?ar?s?z : Yetkisiz Karakter (%s).", strCharID.c_str()));
		else 
		printf("Genie Verisi G?ncellemesi Ba?ar?s?z: Yetkisiz Karakter (%s).\n", strCharID.c_str());
	}

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) 
	{
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "Genie Verisi G?ncellemesi Ba?ar?s?z : Veritaban? Komutu Olu?turulamad?.");
		else
		printf("Genie Verisi G?ncellemesi Ba?ar?s?z : Veritaban? Komutu Olu?turulamad?.\n");
		return false;
	}

	// JstKO Parametreleri ekle
	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, (char*)pUser->m_GenieOptions, sizeof(pUser->m_GenieOptions), SQL_BINARY);

	// JstKO Stored procedure ?a?r?s?
	string proc = string_format(_T("{CALL UPDATE_GENIE_DATA(?,?, ?, %d,%d)}"), (uint32)pUser->m_1098GenieTime, pUser->m_sFirstUsingGenie);

	if (!dbCommand->Execute(proc)) 
	{
		// JstKO Hata raporu
		ReportSQLError(m_GameDB->GetError());
		// JstKO Oyuncuya notice
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "Genie Verisi G?ncellemesi Ba?ar?s?z : Sunucu Hatas?.");
		else
		printf("Genie Verisi G?ncellemesi Ba?ar?s?z : Sunucu Hatas?.\n");
		return false;
	}

	return true;
}
#pragma endregion

#pragma region CDBAgent::LoadGenieData(string& strCharID, CUser* pUser)
bool CDBAgent::LoadGenieData(string& strCharID, CUser* pUser)
{
	if (pUser == nullptr || strCharID != pUser->GetName()) 
	{
		// JstKO Kullan?c? ge?erli de?il veya karakter ad? uyu?muyor
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "Genie Verisi Y?klenemedi : Ge?ersiz Kullan?c?.");
		else
		printf("Genie Verisi Y?klenemedi : Ge?ersiz Kullan?c?.\n");
		return false;
	}

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr) 
	{
		// JstKO Veritaban? komutu olu?turulamad?
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "Genie Verisi Y?klenemedi : Veritaban? Ba?lant? Hatas?.");
		else
		printf("Genie Verisi Y?klenemedi : Ge?ersiz Kullan?c?.\n");
		return false;
	}

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());
	dbCommand->AddParameter(SQL_PARAM_INPUT, pUser->GetAccountName().c_str(), pUser->GetAccountName().length());

	// JstKO Veritaban?nda veriyi y?klemeye ?al??
	if (!dbCommand->Execute(_T("{CALL LOAD_GENIE_DATA(?,?)}"))) 
	{
		ReportSQLError(m_GameDB->GetError());
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "Genie Verisi Y?klenemedi : Sunucu Hatas?.");
		else
		printf("Genie Verisi Y?klenemedi : Sunucu Hatas?.\n");
		return false;
	}

	// JstKO Veritaban?ndan veri yoksa
	if (!dbCommand->hasData()) 
	{
		if (pUser != nullptr) g_pMain->SendHelpDescription(pUser, "Genie Verisi Bulunamad?.");
		else
		printf("Genie Verisi Bulunamad?.\n");
		return false;
	}

	// JstKO Veri ba?ar?l? ?ekilde y?klendi
	int field = 1;
	uint32 genietime = 0;
	dbCommand->FetchUInt32(field++, genietime);
	dbCommand->FetchBinary(field++, (char*)pUser->m_GenieOptions, sizeof(pUser->m_GenieOptions));
	dbCommand->FetchByte(field++, pUser->m_sFirstUsingGenie);

	pUser->m_1098GenieTime = genietime;

	return true;
}
#pragma endregion

#pragma region CUser::GenieExchange(uint32 itemid, uint32 time, bool newChar) 
bool CUser::GenieExchange(uint32 itemid, uint32 time, bool newChar) 
{
	// JstKO Ge?ersiz s?re veya e?ya kontrol?
	if (!time || (!newChar && !itemid)) 
	{
		g_pMain->SendHelpDescription(this, "Genie De?i?imi Ba?ar?s?z : Ge?ersiz S?re Veya E?ya.");
		return false;
	}

	// JstKO E?ya kullan?m? gerekiyor, varsa kontrol et ve sil
	if (!newChar) 
	{
		if (!CheckExistItem(itemid)) 
		{
			g_pMain->SendHelpDescription(this, string_format("Genie De?i?imi Ba?ar?s?z: Item Bulunamad? (ID: %u).", itemid));
			return false;
		}
		if (!RobItem(itemid)) 
		{
			g_pMain->SendHelpDescription(this, string_format("Genie De?i?imi Ba?ar?s?z: Item Silinemedi (ID: %u).", itemid));
			return false;
		}
		g_pMain->SendHelpDescription(this, "E?ya Ba?ar?yla Kullan?ld?. G?le G?le");
	}

	// JstKO ?lk kullan?m flag'ini ayarla
	if (!m_sFirstUsingGenie) 
	{
		m_sFirstUsingGenie = 1;
		g_pMain->SendHelpDescription(this, "Genie ?lk Kez Kullan?ld?, Bonus S?reler Aktif.");
	}

	// JstKO Kalan s?renin hesaplanmas?
	int remtime = int(m_1098GenieTime > UNIXTIME ? m_1098GenieTime - UNIXTIME : 0);
	m_1098GenieTime = UNIXTIME + (time * HOUR) + (remtime > 0 ? remtime : 0);

	// JstKO Kullan?c?ya yeni s?re bilgisini g?nder
	uint32 newRemaining = GetGenieTime();
	g_pMain->SendHelpDescription(this, string_format("Genie Etkinle?tirildi! Kalan S?re : [ %d ] Oldunuz Tebrikler!", newRemaining));

	// JstKO ?stemciye WIZ_GENIE paketi
	if (!newChar) 
	{
		Packet result(WIZ_GENIE, uint8(GenieUseSpiringPotion));
		result << uint8(GenieUseSpiringPotion) << GetGenieTime();
		Send(&result);
	}

	return true;
}
#pragma endregion

#pragma region CUser::CheckGenieTime()
void CUser::CheckGenieTime() 
{
	// JstKO Genie'nin s?resi dolmu?sa i?lemi sonland?r
	if (UNIXTIME > m_1098GenieTime) 
	{
		g_pMain->SendHelpDescription(this, "Genie S?resi Dolmu?, Sistem Sonland?r?l?yor.");
		GenieStop();
	}

	// JstKO Genie zaman bilgisini istemciye g?nder
	Packet result(WIZ_GENIE, uint8(GenieInfoRequest));
	result << uint8(GenieRemainingTime) << GetGenieTime();
	Send(&result);
}
#pragma endregion

bool CDBAgent::LoadPriestBotGenieData(string& strCharID, CUser* pUser)
{
	if (pUser == nullptr
		|| strCharID != pUser->GetName())
		return false;

	unique_ptr<OdbcCommand> dbCommand(m_GameDB->CreateCommand());
	if (dbCommand.get() == nullptr)
		return false;

	dbCommand->AddParameter(SQL_PARAM_INPUT, strCharID.c_str(), strCharID.length());

	if (!dbCommand->Execute(_T("{CALL LOAD_GENIE_DATA(?)}")))
		ReportSQLError(m_GameDB->GetError());

	if (!dbCommand->hasData())
		return false;

	CBot* pPriest = nullptr;
	pPriest = g_pMain->m_MapBotList.GetData(pUser->m_bUserPriestBotID);
	if (!pPriest)
		return false;

	int field = 1;
	dbCommand->FetchUInt16(field++, pPriest->m_GenieTime);
	dbCommand->FetchBinary(field++, (char*)pPriest->m_GenieOptions, sizeof(pPriest->m_GenieOptions));
	dbCommand->FetchByte(field++, pPriest->m_sFirstUsingGenie);

	return true;
}




