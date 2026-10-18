#include "StdAfx.h"

#pragma region CUser::CindirilleHandler(Packet& pkt)
void CUser::CindirillaHandler(Packet& pkt) {
	uint8 opcode = pkt.read<uint8>();
	switch ((cindopcode)opcode)
	{
	case cindopcode::selectclass:
		CindirellaSelectClass(pkt);
		break;
	case cindopcode::nationchange:
		CindirellaNationChange(pkt);
		break;
	}
}
#pragma endregion

#pragma region CUser::SendCindSelectError(cindopcode a,bool nation)
void CUser::SendCindSelectError(cindopcode a, bool nation) {

	if (a != cindopcode::timewait) {
		if (nation)
			pCindWar.myselectnationtime = UNIXTIME + 5;
		else
			pCindWar.myselectlasttime = UNIXTIME + 5;
	}
		
	Packet newpkt(XSafe, uint8(XSafeOpCodes::CINDIRELLA));
	newpkt << uint8(nation ? cindopcode::nationchange : cindopcode::selectclass) << uint8(a);

	if (a == cindopcode::timewait) {
		uint32 remtime = 0;
		if (nation) {
			if (pCindWar.myselectnationtime > UNIXTIME)
				remtime = uint32(pCindWar.myselectnationtime - UNIXTIME);
		}
		else {
			if (pCindWar.myselectlasttime > UNIXTIME) 
				remtime = uint32(pCindWar.myselectlasttime - UNIXTIME);
		}
		newpkt << remtime;
	}
	Send(&newpkt);
}
#pragma endregion

#pragma region CUser::CindirellaNationChange(Packet& pkt)
void CUser::CindirellaNationChange(Packet& pkt) {

	if (isDead() || !pCindWar.isEventUser() || !g_pMain->isCindirellaZone(GetZoneID()))
		return;

	if (isInParty())
		return g_pMain->SendHelpDescription(this,"please exit the party.");

	Packet newpkt(XSafe, uint8(XSafeOpCodes::CINDIRELLA));
	newpkt << uint8(cindopcode::nationchange);

	if (pCindWar.myselectnationtime > UNIXTIME)
		return SendCindSelectError(cindopcode::timewait);

	uint8 selected_nation = pkt.read<uint8>();
	if (selected_nation != 1 && selected_nation != 2)
		return;

	// Fun/Cindirella icinde irk secimi her zaman orijinal irkin karsisi olmalidir.
	// Human/Elmorad oyuncu sadece Karus, Karus oyuncu sadece Human/Elmorad secebilir.
	// Boylece event icinde dost/dusman taraflari karismaz.
	if (selected_nation == pCindWar.m_oNation)
		return SendCindSelectError(cindopcode::alreadynation);

	if (selected_nation == pCindWar.m_eNation)
		return SendCindSelectError(cindopcode::alreadynation);

	pCindWar.m_eNation = selected_nation;

	CindirellaChaModify(false, true);
	Home(false);

	pCindWar.myselectnationtime = UNIXTIME + 90;
	newpkt << uint8(cindopcode::success);
	Send(&newpkt);
}
#pragma endregion

#pragma region CUser::CindirellaSelectClass(Packet& pkt)
void CUser::CindirellaSelectClass(Packet& pkt) {

	if (!pCindWar.isEventUser() || !g_pMain->isCindirellaZone(GetZoneID()))
		return;

	Packet newpkt(XSafe, uint8(XSafeOpCodes::CINDIRELLA));
	newpkt << uint8(cindopcode::selectclass);

	if (pCindWar.myselectlasttime > UNIXTIME)
		return SendCindSelectError(cindopcode::timewait);

	uint8 selected_class = pkt.read<uint8>();

	if (selected_class > 4)
		return SendCindSelectError(cindopcode::notchange);

	if (selected_class == pCindWar.myselectclass)
		return SendCindSelectError(cindopcode::alreadyclass);

	pCindWar.myselectclass = selected_class;
#if 0
	if (pCindWar.first_selected) {
		CindirellaChaModify(pCindWar.myselectclass);
		pCindWar.first_selected = false;
	}
#else
	CindirellaChaModify(pCindWar.myselectclass);
#endif
	pCindWar.myselectlasttime = UNIXTIME + 30;
	newpkt << uint8(cindopcode::success) << selected_class;
	Send(&newpkt);
}
#pragma endregion

enum class error { success, jointime, loyalty, money, maxlimit, notstarted, highlevel, minlevel, server };

#pragma region CUser::CindirellaGetNewClass(uint8 iclass,uint8 systemlevel)
uint8 CUser::CindirellaGetNewClass(uint8 iclass ,uint8 systemlevel) {
	uint8 newclass = 0;
	if (GetNation() == (uint8)Nation::ELMORAD) {
		if (iclass == 0) newclass = 206;
		else if (iclass == 1) newclass = 208;
		else if (iclass == 2) newclass = 210;
		else if (iclass == 3) newclass = 212;
		else if (iclass == 4) newclass = 215;
	}
	else {
		if (iclass == 0) newclass = 106;
		else if (iclass == 1) newclass = 108;
		else if (iclass == 2) newclass = 110;
		else if (iclass == 3) newclass = 112;
		else if (iclass == 4) newclass = 115;
	}

	if (newclass && systemlevel < 60)
		newclass -= 1;

	return newclass;
}
#pragma endregion

#pragma region CUser::CindirellaGetNewRace(uint8 iclass)
uint8 CUser::CindirellaGetNewRace(uint8 iclass) {
	uint8 newrace = 0;
	if (GetNation() == (uint8)Nation::ELMORAD) {
		if (iclass == 0) newrace = 12;
		else if (iclass == 1) newrace = 12;
		else if (iclass == 2) newrace = 13;
		else if (iclass == 3) newrace = 13;
		else if (iclass == 4) newrace = 14;
	}
	else {
		if (iclass == 0) newrace = 1;
		else if (iclass == 1) newrace = 2;
		else if (iclass == 2) newrace = 4;
		else if (iclass == 3) newrace = 4;
		else if (iclass == 4) newrace = 6;
	}
	return newrace;
}
#pragma endregion

#pragma region CGameServerDlg::CindirellaGetTime()
uint32 CGameServerDlg::CindirellaGetTime() {
	uint32 remtime = 0;
	if (pCindWar.isPrepara() && pCindWar.preparetime > UNIXTIME)
		remtime = uint32(pCindWar.preparetime - UNIXTIME);
	else if (pCindWar.isStarted() && pCindWar.finishtime > UNIXTIME)
		remtime = uint32(pCindWar.finishtime - UNIXTIME);

	return remtime;
}
#pragma endregion

#pragma region CUser::get_cindclassindex()
int8 CUser::get_cindclassindex() {
	if (isWarrior())
		return 0;
	if (isRogue())
		return 1;
	if (isMage())
		return 2;
	if (isPriest())
		return 3;
	if (isPortuKurian())
		return 4;
	return (int8)-1;
}
#pragma endregion

#pragma region CUser::get_cindclassindex()
bool CUser::CindirellaSign() {

	int8 iclass = get_cindclassindex();
	if (iclass < 0 || iclass > 4) return false;

	auto& pSet = g_pMain->pCindWar.pSetting[g_pMain->pCindWar.settingid];

	pCindWar.gainexp = pCindWar.gainnoah = 0;
	pCindWar.m_sClass = GetClass(); pCindWar.m_bRace = GetRace();
	pCindWar.m_oNation = GetNation();
	// Event icindeki taraf, orijinal irkin karsisi olarak baslar.
	// Human/Elmorad oyuncu Karus tarafina, Karus oyuncu Human/Elmorad tarafina gecer.
	pCindWar.m_eNation = (pCindWar.m_oNation == (uint8)Nation::ELMORAD) ? (uint8)Nation::KARUS : (uint8)Nation::ELMORAD;
	m_bNation = pCindWar.m_eNation;

	uint8 newclass = CindirellaGetNewClass(iclass, pSet.beglevel);
	uint8 newrace = CindirellaGetNewRace(iclass);
	if (!newclass || !newrace)
	{
		m_bNation = pCindWar.m_oNation;
		return false;
	}

	pCindWar.m_QuestMap.clear();

	m_questMap.m_lock.lock();
	foreach_stlmap_nolock(itr, m_questMap) {
		if (itr->second) {
			_USER_QUEST_INFO a;
			for (int i = 0; i < 4; i++)
				a.m_bKillCounts[i] = itr->second->m_bKillCounts[i];
			a.QuestState = itr->second->QuestState;
			pCindWar.m_QuestMap.insert(std::make_pair(itr->first, a));
		}
	}
		
	m_questMap.m_lock.unlock();

	/*23.04.2024*/
	pCindWar.m_savedMagicMap.clear();
	m_savedMagicLock.lock();
	foreach(itr, m_savedMagicMap) {
		if (!itr->second)
			continue;

		pCindWar.m_savedMagicMap[itr->first] = (itr->second - UNIXTIME2);
	}
	m_savedMagicLock.unlock();
	m_savedMagicMap.clear();
	/*23.04.2024*/

	m_sClass = newclass; m_bRace = newrace;
	OpenEtcSkill(true);

	/*Packet test(XSafe, uint8(0xDF));
	test << itr->second->QuestState;
	Send(&test);*/

	memcpy(pCindWar.m_strSkillData, m_strSkillData, sizeof(m_strSkillData));
	pCindWar.m_strSkillCount = m_strSkillCount;
	pCindWar.myselectclass = iclass;

	std::string strUserID = GetName();
	STRTOUPPER(strUserID);
	if (GetNation() == (uint8)Nation::ELMORAD) {
		g_pMain->pCindWar.e_listlock.lock();
		auto itr = g_pMain->pCindWar.e_list.find(strUserID);
		if (itr != g_pMain->pCindWar.e_list.end()) {
			pCindWar.killcount = itr->second.killcount;
			pCindWar.deadcount = itr->second.deadcount;
		}
		else g_pMain->pCindWar.e_list.insert(std::make_pair(strUserID, _cindwarmycount(0, 0)));
		g_pMain->pCindWar.e_listlock.unlock();
	}
	else {
		g_pMain->pCindWar.k_listlock.lock();
		auto itr = g_pMain->pCindWar.k_list.find(strUserID);
		if (itr != g_pMain->pCindWar.k_list.end()) {
			pCindWar.killcount = itr->second.killcount;
			pCindWar.deadcount = itr->second.deadcount;
		}
		else g_pMain->pCindWar.k_list.insert(std::make_pair(strUserID, _cindwarmycount(0, 0)));
		g_pMain->pCindWar.k_listlock.unlock();
	}

	pCindWar.eventuser = true;
	pCindWar.m_bLevel = GetLevel();
	pCindWar.m_iExp = m_iExp;
	pCindWar.m_iGold = m_iGold;

	m_sClass = newclass; m_bRace = newrace;
	m_bLevel = pSet.beglevel;
	m_iExp = 0;
	m_iGold = 0;

	memcpy(pCindWar.m_sItemArray, m_sItemArray, sizeof(m_sItemArray));
	memcpy(pCindWar.m_bstrSkill, m_bstrSkill, sizeof(m_bstrSkill));
	memcpy(pCindWar.m_bStats, m_bStats, sizeof(m_bStats));
	memset(&m_sItemArray, 0, sizeof(m_sItemArray));

	if (iclass == 0) memcpy(m_sItemArray, g_pMain->pCindWar.m_warrior, sizeof(g_pMain->pCindWar.m_warrior));
	else if (iclass == 1) memcpy(m_sItemArray, g_pMain->pCindWar.m_rogue, sizeof(g_pMain->pCindWar.m_rogue));
	else if (iclass == 2) memcpy(m_sItemArray, g_pMain->pCindWar.m_mage, sizeof(g_pMain->pCindWar.m_mage));
	else if (iclass == 3) memcpy(m_sItemArray, g_pMain->pCindWar.m_priest, sizeof(g_pMain->pCindWar.m_priest));
	else if (iclass == 4) memcpy(m_sItemArray, g_pMain->pCindWar.m_kurian, sizeof(g_pMain->pCindWar.m_kurian));

	for (int i = 0; i < INVENTORY_TOTAL; i++) m_sItemArray[i].cindareaitem = true;
	pCindWar.m_sPoints = m_sPoints;

	m_sPoints = 0;

	_CINDWAR_STATSET pStatSet = _CINDWAR_STATSET();
	g_pMain->m_CindirellaStatArray.m_lock.lock();
	foreach_stlmap_nolock(itr, g_pMain->m_CindirellaStatArray) {
		if (!itr->second)
			continue;

		if (g_pMain->pCindWar.settingid == itr->second->settindid && (pCindWar.myselectclass + 1) == itr->second->iClass) {
			pStatSet = *itr->second;
			break;
		}
	}
	g_pMain->m_CindirellaStatArray.m_lock.unlock();

	if (pStatSet.id)
	{
		for (int i = 0; i < (uint8)StatType::STAT_COUNT; i++) SetStat((StatType)i, pStatSet.stat[i]);
		m_sPoints = pStatSet.Statfreepoint;
	}

	Packet newpkt(WIZ_PRESET);
	newpkt << uint8(1) << uint8(1);
	for (int i = 0; i < (uint8)StatType::STAT_COUNT; i++) newpkt << GetStat((StatType)i);
	newpkt << m_sPoints;
	Send(&newpkt);

	memset(m_bstrSkill, 0, sizeof(m_bstrSkill));

	if (pStatSet.id) {
		m_bstrSkill[0] = (uint8)pStatSet.Skillfreepoint;
		for (int i = 5; i < 9; i++)
			m_bstrSkill[i] = pStatSet.skill[i-5];
	}
	
	newpkt.clear();
	newpkt << uint8(2) << uint8(1);
	for (int i = 5; i < 9; i++) newpkt << m_bstrSkill[i];
	newpkt << m_bstrSkill[0];
	Send(&newpkt);

	newpkt.clear();
	newpkt.Initialize(WIZ_CLASS_CHANGE);
	newpkt << uint8(ALL_SKILLPT_CHANGE) << uint8(1) << GetCoins() << m_bstrSkill[0];
	Send(&newpkt);

	SendMyInfo(true);
	UserInOut(InOutType::INOUT_OUT);
	SetRegion(GetNewRegionX(), GetNewRegionZ());
	UserInOut(InOutType::INOUT_WARP);
	g_pMain->UserInOutForMe(this);
	NpcInOutForMe();
	g_pMain->MerchantUserInOutForMe(this);
	ResetWindows();
	
	//MerchantXClose();
	//InitType4(true); //23.04.2024
	//RecastSavedMagic();
	
	SetUserAbility();
	HpChange(GetMaxHealth());
	MSpChange(GetMaxMana());
	if (isPortuKurian()) SpChange(GetMaxStamina());

	newpkt.clear();
	newpkt.Initialize(XSafe);
	newpkt << uint8(XSafeOpCodes::CINDIRELLA) << uint8(cindopcode::joinevent) << g_pMain->pCindWar.isPrepara() << iclass;
	newpkt << g_pMain->CindirellaGetTime() << pCindWar.killcount << pCindWar.deadcount << g_pMain->pCindWar.k_killcount << g_pMain->pCindWar.e_killcount;
	Send(&newpkt);
	return true;
}
#pragma endregion

#pragma region CUser::CindirellaChaModify(bool regene,bool nation)
bool CUser::CindirellaChaModify(bool regene, bool nation) {


	int8 my_classindex = get_cindclassindex();
	bool someclass = my_classindex == pCindWar.myselectclass;

	if (regene && someclass)
		return true;

	if (pCindWar.myselectclass > 4 || !pCindWar.isEventUser() || !g_pMain->isCindirellaZone(GetZoneID()))
		return false;

	if (pCindWar.m_eNation == 1 || pCindWar.m_eNation == 2)m_bNation = pCindWar.m_eNation;

	auto& pSet = g_pMain->pCindWar.pSetting[g_pMain->pCindWar.settingid];

	uint8 newclass = CindirellaGetNewClass(pCindWar.myselectclass, pSet.beglevel);
	uint8 newrace = CindirellaGetNewRace(pCindWar.myselectclass);
	if (!newclass || !newrace) return false;

	m_sClass = newclass; m_bRace = newrace;
	if (!someclass) OpenEtcSkill(true);
	_CINDWAR_STATSET pStatSet = _CINDWAR_STATSET();
	if (!someclass)
	{
		if (pCindWar.myselectclass == 0) memcpy(m_sItemArray, g_pMain->pCindWar.m_warrior, sizeof(g_pMain->pCindWar.m_warrior));
		else if (pCindWar.myselectclass == 1) memcpy(m_sItemArray, g_pMain->pCindWar.m_rogue, sizeof(g_pMain->pCindWar.m_rogue));
		else if (pCindWar.myselectclass == 2) memcpy(m_sItemArray, g_pMain->pCindWar.m_mage, sizeof(g_pMain->pCindWar.m_mage));
		else if (pCindWar.myselectclass == 3) memcpy(m_sItemArray, g_pMain->pCindWar.m_priest, sizeof(g_pMain->pCindWar.m_priest));
		else if (pCindWar.myselectclass == 4) memcpy(m_sItemArray, g_pMain->pCindWar.m_kurian, sizeof(g_pMain->pCindWar.m_kurian));

		for (int i = 0; i < INVENTORY_TOTAL; i++) m_sItemArray[i].cindareaitem = true;
		
		g_pMain->m_CindirellaStatArray.m_lock.lock();
		foreach_stlmap_nolock(itr, g_pMain->m_CindirellaStatArray) {
			if (!itr->second)
				continue;

			if (g_pMain->pCindWar.settingid == itr->second->settindid && (pCindWar.myselectclass + 1) == itr->second->iClass) {
				pStatSet = *itr->second;
				break;
			}
		}
		g_pMain->m_CindirellaStatArray.m_lock.unlock();
		if (pStatSet.id)
		{
			for (int i = 0; i < (uint8)StatType::STAT_COUNT; i++) SetStat((StatType)i, pStatSet.stat[i]);
			m_sPoints = pStatSet.Statfreepoint;
		}
	}

	Packet newpkt(WIZ_PRESET);
	newpkt << uint8(1) << uint8(1);
	for (int i = 0; i < (uint8)StatType::STAT_COUNT; i++) newpkt << GetStat((StatType)i);
	newpkt << m_sPoints;
	Send(&newpkt);

	if (!someclass && pStatSet.id)
	{
		memset(m_bstrSkill, 0, sizeof(m_bstrSkill));
		m_bstrSkill[0] = (uint8)pStatSet.Skillfreepoint;
		for (int i = 5; i < 9; i++)
			m_bstrSkill[i] = pStatSet.skill[i-5];
	}

	newpkt.clear();
	newpkt << uint8(2) << uint8(1);
	for (int i = 5; i < 9; i++) newpkt << m_bstrSkill[i];
	newpkt << m_bstrSkill[0];
	Send(&newpkt);

	if (!someclass || nation)
	{
		newpkt.clear();
		newpkt.Initialize(WIZ_CLASS_CHANGE);
		newpkt << uint8(ALL_SKILLPT_CHANGE) << uint8(1) << GetCoins() << m_bstrSkill[0];
		Send(&newpkt);
	}

	SendMyInfo(true);
	UserInOut(InOutType::INOUT_OUT);
	SetRegion(GetNewRegionX(), GetNewRegionZ());
	UserInOut(InOutType::INOUT_WARP);
	g_pMain->UserInOutForMe(this);
	NpcInOutForMe();
	g_pMain->MerchantUserInOutForMe(this);
	ResetWindows();
	
	InitType4();
	RecastSavedMagic();
	
	SetUserAbility();
	HpChange(GetMaxHealth());
	MSpChange(GetMaxMana());
	if (isPortuKurian()) SpChange(GetMaxStamina());
	OpenEtcSkill(false);
	return true;
}
#pragma endregion

#pragma region CUser::CindirellaJoin(int8 iclass)
uint8 CUser::CindirellaJoin(int8 iclass) {
	return (uint8)error::server;
}
#pragma endregion

#pragma region CUser::CindireallaUpdateKDA(CUser *pKiller)
void CUser::CindireallaUpdateKDA(CUser* pKiller) {

	Packet newpkt(XSafe, uint8(XSafeOpCodes::CINDIRELLA));
	newpkt << uint8(cindopcode::updatekda) << uint8(0) << pCindWar.killcount << ++pCindWar.deadcount;
	Send(&newpkt);

	if (!pKiller)
		return;

	uint8 bNation = pKiller->GetNation();

	newpkt.clear();
	newpkt << uint8(XSafeOpCodes::CINDIRELLA) << uint8(cindopcode::updatekda) << uint8(0) << ++pKiller->pCindWar.killcount << pKiller->pCindWar.deadcount;
	pKiller->Send(&newpkt);

	newpkt.clear();
	newpkt << uint8(XSafeOpCodes::CINDIRELLA) << uint8(cindopcode::updatekda) << uint8(1);
	(Nation)bNation == Nation::ELMORAD ? g_pMain->pCindWar.e_killcount++ : g_pMain->pCindWar.k_killcount++;
	newpkt << g_pMain->pCindWar.e_killcount << g_pMain->pCindWar.k_killcount;
	g_pMain->Send_All(&newpkt, nullptr, (uint8)Nation::ALL, g_pMain->pCindWar.pSetting[g_pMain->pCindWar.settingid].zoneid);
}
#pragma endregion

#pragma region CUser::CindirellaSendFinish()
void CUser::CindirellaSendFinish() {
	Packet newpkt(XSafe, uint8(XSafeOpCodes::CINDIRELLA));
	newpkt << uint8(cindopcode::finish);
	Send(&newpkt);
}
#pragma endregion

bool CUser::IsCindIn(uint8 zoneid /*=0*/) {
	if (zoneid == 0) zoneid = GetZoneID();
	return g_pMain->isCindirellaZone(zoneid) && g_pMain->pCindWar.isON();
}

#pragma region CUser::CindirellaLogOut(bool exitgame)
void CUser::CindirellaLogOut(bool exitgame) {

	memset(&m_sItemArray, 0, sizeof(m_sItemArray));
	memcpy(&m_sItemArray, pCindWar.m_sItemArray, sizeof(m_sItemArray));
	memcpy(&m_bstrSkill, pCindWar.m_bstrSkill, sizeof(m_bstrSkill));

	m_questMap.DeleteAllData();
	foreach(itr, pCindWar.m_QuestMap)
	{
		_USER_QUEST_INFO* a = new _USER_QUEST_INFO();
		a->QuestState = itr->second.QuestState;
		
		for (int i = 0; i < 4; i++)
			a->m_bKillCounts[i] = itr->second.m_bKillCounts[i];
		
		if (!m_questMap.PutData(itr->first, a))
			delete a;
	}

	/*23.04.2024*/
	m_savedMagicMap.clear();
	m_savedMagicLock.lock();
	foreach(itr, pCindWar.m_savedMagicMap) {
		if (!itr->second)
			continue;

		m_savedMagicMap[itr->first] = itr->second + UNIXTIME2;
	}
	m_savedMagicLock.unlock();
	/*23.04.2024*/

	// Eski kod sadece ilk byte'i geri kopyaliyordu. Bu durum event cikisinda
	// kullanicinin skill data bilgisinin DB'ye yanlis kaydedilmesine sebep olabilir.
	memcpy(m_strSkillData, pCindWar.m_strSkillData, sizeof(m_strSkillData));
	m_strSkillCount = pCindWar.m_strSkillCount;

	m_bLevel = pCindWar.m_bLevel;
	m_iExp = pCindWar.m_iExp;
	m_iGold = pCindWar.m_iGold;
	m_sClass = pCindWar.m_sClass;
	m_bRace = pCindWar.m_bRace;
	m_bNation = pCindWar.m_oNation;

	if (pCindWar.gainexp)m_iExp += pCindWar.gainexp;
	if (pCindWar.gainnoah)m_iGold += pCindWar.gainnoah;

	for (int i = 0; i < (uint8)StatType::STAT_COUNT; i++) SetStat((StatType)i, pCindWar.m_bStats[i]);
	m_sPoints = pCindWar.m_sPoints;

	pCindWar.eventuser = false;
	pCindWar.Initialize();

	if (exitgame)
	{
		m_bZone = ZONE_MORADON; m_curx = (float)26250 / 100.0f; m_cury = (float)800 / 100.0f; m_curz = (float)30160 / 100.0f;
	}

	//InitType4(true); //23.04.2024

	if (!exitgame) {

		std::string strUserID = GetName();
		if (GetNation() == (uint8)Nation::ELMORAD) {
			g_pMain->pCindWar.e_listlock.lock();
			auto itr = g_pMain->pCindWar.e_list.find(strUserID);
			if (itr != g_pMain->pCindWar.e_list.end())
				g_pMain->pCindWar.e_list.erase(strUserID);
			g_pMain->pCindWar.e_listlock.unlock();
		}
		else {
			g_pMain->pCindWar.k_listlock.lock();
			auto itr = g_pMain->pCindWar.k_list.find(strUserID);
			if (itr != g_pMain->pCindWar.k_list.end())
				g_pMain->pCindWar.k_list.erase(strUserID);
			g_pMain->pCindWar.k_listlock.unlock();
		}

		Packet newpkt(WIZ_PRESET);
		newpkt << uint8(1) << uint8(1);
		for (int i = 0; i < (uint8)StatType::STAT_COUNT; i++) newpkt << GetStat((StatType)i);
		newpkt << m_sPoints;
		Send(&newpkt);

		newpkt.clear();
		newpkt << uint8(2) << uint8(1);
		for (int i = 5; i < 9; i++) newpkt << m_bstrSkill[i];
		newpkt << m_bstrSkill[0];
		Send(&newpkt);

		SendMyInfo(true);
		UserInOut(InOutType::INOUT_OUT);
		SetRegion(GetNewRegionX(), GetNewRegionZ());
		UserInOut(InOutType::INOUT_WARP);
		g_pMain->UserInOutForMe(this);
		NpcInOutForMe();
		g_pMain->MerchantUserInOutForMe(this);
		ResetWindows();
		//MerchantXClose();

		//RecastSavedMagic();

		SetUserAbility();

		HpChange(GetMaxHealth());
		MSpChange(GetMaxMana());
		if (isPortuKurian()) SpChange(GetMaxStamina());

		newpkt.clear();
		newpkt.Initialize(WIZ_SKILLDATA);
		newpkt << uint8(SKILL_DATA_LOAD) << m_strSkillCount;
		for (uint32 i = 0; i < m_strSkillCount; i++) newpkt << *(uint32*)(m_strSkillData + (i * sizeof(uint32)));
		Send(&newpkt);

		newpkt.clear();
		newpkt.Initialize(XSafe);
		newpkt << uint8(XSafeOpCodes::CINDIRELLA) << uint8(cindopcode::finish);
		Send(&newpkt);
	}
}
#pragma endregion



static uint8 GetCindirellaForcedZone(uint8 originalZone)
{
	// Fun Class haritalari:
	// 107 = Mini Colony Zone, 115 = Mini Ardream.
	// Veritabaninda eski CZ/Ardream/RLB gelirse otomatik mini haritaya cevrilir.
	if (originalZone == ZONE_ARDREAM)
		return 115;

	if (originalZone == ZONE_RONARK_LAND || originalZone == ZONE_RONARK_LAND_BASE)
		return 107;

	if (originalZone == 107 || originalZone == 115)
		return originalZone;

	// Fun Class icin default Mini Colony Zone.
	return 107;
}

static uint16 SpawnCindirellaAutoBots(CGameServerDlg* pMain, uint8 zoneID, uint8 minLevel, int minute)
{
	if (pMain == nullptr || pMain->GetZoneByID(zoneID) == nullptr)
		return 0;

	const uint8 classes[] = { 1, 2, 3, 4, 14 }; // warrior, rogue, mage, priest, kurian
	const uint8 nations[] = { (uint8)Nation::KARUS, (uint8)Nation::ELMORAD };
	uint16 spawned = 0;

	for (int n = 0; n < int(_countof(nations)); n++)
	{
		for (int i = 0; i < 200; i++)
		{
			uint8 sClass = classes[(i + myrand(0, int(_countof(classes)) - 1)) % int(_countof(classes))];
			uint16 botID = pMain->SpawnEventBotPk(minute, zoneID, 0.0f, 0.0f, 0.0f, minLevel, nations[n], sClass, nullptr, true);
			if (botID == 0)
				continue;

			CBot* pBot = pMain->GetBotPtr(botID);
			if (pBot == nullptr)
				continue;

			// Cindirella/Fun Class botlari event ayarindaki level ile gorunmeli.
			// SpawnEventBotPk normalde min level ustu ilk uygun botu alabildigi icin
			// tum botlar 83 geliyordu; burada event leveline sabitliyoruz.
			pBot->m_bLevel = minLevel;
			pBot->SetBotAbility();
			pBot->HpChange(pBot->GetMaxHealth());
			pBot->MSpChange(pBot->GetMaxMana());

			// BOT_MOVE durumunda BotPartyInviteProcess() seek acik normal kullanicilari davet eder.
			pBot->m_BotState = BOT_MOVE;
			pBot->m_bPartyLeader = true;
			pBot->m_bNeedParty = 0x01;
			pBot->m_bGenieStatus = true;
			spawned++;
		}
	}
	return spawned;
}

void CGameServerDlg::CindirellaTown() {

	std::vector<std::string> mlist;
	g_pMain->pCindWar.e_listlock.lock();
	foreach(itr, g_pMain->pCindWar.e_list)
		if (&itr->second)
			mlist.push_back(itr->first);
	g_pMain->pCindWar.e_listlock.unlock();

	g_pMain->pCindWar.k_listlock.lock();
	foreach(itr, g_pMain->pCindWar.k_list)
		if (&itr->second)
			mlist.push_back(itr->first);
	g_pMain->pCindWar.k_listlock.unlock();

	foreach(itr, mlist) {
		auto* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (pUser && isCindirellaZone(pUser->GetZoneID())) pUser->Home(false);
	}
}

#pragma region CGameServerDlg::CindirellaTimer()
void CGameServerDlg::CindirellaTimer() {
	if (!pCindWar.isStarted() && !pCindWar.isPrepara()) return;

	uint32 remtime = 0;
	if (pCindWar.isPrepara()) {

		if (pCindWar.preparetime > UNIXTIME) remtime = uint32(pCindWar.preparetime - UNIXTIME);

		if (remtime > 0) {
			uint16 a_time = 0, b_time = 0;
			a_time = 30; if (remtime == a_time * MINUTE) b_time = a_time;
			a_time = 20; if (remtime == a_time * MINUTE) b_time = a_time;
			a_time = 10; if (remtime == a_time * MINUTE) b_time = a_time;
			a_time = 5; if (remtime == a_time * MINUTE) b_time = a_time;
			a_time = 4; if (remtime == a_time * MINUTE) b_time = a_time;
			a_time = 3; if (remtime == a_time * MINUTE) b_time = a_time;
			a_time = 2; if (remtime == a_time * MINUTE) b_time = a_time;
			a_time = 1; if (remtime == a_time * MINUTE) b_time = a_time;
			if (b_time) {
				std::string b_notice = string_format("%d ", b_time) + "minutes until the Fun Class Event begins. Please attend the event and make your preparations.";
				SendAnnouncement(b_notice.c_str(), (uint8)Nation::ALL);
			}
		}

		if (!remtime && pCindWar.prepare && UNIXTIME > pCindWar.preparetime) {
			uint8 setid = g_pMain->pCindWar.settingid;
			pCindWar.finishtime = UNIXTIME + (pCindWar.pSetting[setid].playtime * MINUTE);
			pCindWar.prepare = false;
			pCindWar.start = true;
			CindirellaStart();
			SendAnnouncement("Fun Class has Started. Have Fun.", (uint8)Nation::ALL);
		}
		return;
	}

	remtime = 0;
	if (pCindWar.finishtime > UNIXTIME) remtime = uint32(pCindWar.finishtime - UNIXTIME);

	if (remtime > 0) {
		uint16 a_time = 0, b_time = 0;
		a_time = 30; if (remtime == a_time * MINUTE) b_time = a_time;
		a_time = 20; if (remtime == a_time * MINUTE) b_time = a_time;
		a_time = 10; if (remtime == a_time * MINUTE) b_time = a_time;
		a_time = 5; if (remtime == a_time * MINUTE) b_time = a_time;
		a_time = 4; if (remtime == a_time * MINUTE) b_time = a_time;
		a_time = 3; if (remtime == a_time * MINUTE) b_time = a_time;
		a_time = 2; if (remtime == a_time * MINUTE) b_time = a_time;
		a_time = 1; if (remtime == a_time * MINUTE) b_time = a_time;
		if (b_time) {
			std::string b_notice = string_format("%d ", b_time) + "minutes until the end of the Fun Class event.";
			SendAnnouncement(b_notice.c_str(), (uint8)Nation::ALL);
		}
		return;
	}

	if (!remtime) {
		pCindWar.finishtime = 0;
		pCindWar.prepare = false; pCindWar.start = false;

		struct _list { CUser* puser; uint32 killcount, deadcount; _list(CUser* a, uint32 b, uint32 c) { puser = a; killcount = b; deadcount = c; } };

		std::vector<std::string> mlist;
		g_pMain->pCindWar.e_listlock.lock();
		foreach(itr, g_pMain->pCindWar.e_list)
			if (&itr->second)
				mlist.push_back(itr->first);
		g_pMain->pCindWar.e_listlock.unlock();

		g_pMain->pCindWar.k_listlock.lock();
		foreach(itr, g_pMain->pCindWar.k_list)
			if (&itr->second)
				mlist.push_back(itr->first);
		g_pMain->pCindWar.k_listlock.unlock();

		std::vector<_list> e_scoreboard, k_scoreboard;
		foreach(itr, mlist) {
			auto* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
			if (!pUser || !isCindirellaZone(pUser->GetZoneID()))
				continue;

			if ((Nation)pUser->GetNation() == Nation::ELMORAD)
				e_scoreboard.push_back(_list(pUser, pUser->pCindWar.killcount, pUser->pCindWar.deadcount));
			else
				k_scoreboard.push_back(_list(pUser, pUser->pCindWar.killcount, pUser->pCindWar.deadcount));
		}

		foreach(itr, e_scoreboard)
			if (itr->puser) {
				itr->puser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f, -1, false);
				itr->puser->CindirellaSendFinish();
			}

		foreach(itr, k_scoreboard) {
			if (itr->puser) {
				itr->puser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f, -1, false);
				itr->puser->CindirellaSendFinish();
			}
		}

		std::sort(e_scoreboard.begin(), e_scoreboard.end(), [](auto const& a, auto const& b) { return a.killcount > b.killcount && a.deadcount < b.deadcount; });
		std::sort(k_scoreboard.begin(), k_scoreboard.end(), [](auto const& a, auto const& b) { return a.killcount > b.killcount && a.deadcount < b.deadcount; });

		uint16 counter = 0;
		foreach(itr, e_scoreboard) {
			counter++;
			if (counter > 200) break;

			auto p = pCindWar.pReward[counter - 1];
			if (!itr->puser || !itr->puser->isInGame())
				continue;

			if (p.expcount) itr->puser->ExpChange("Fun Class",p.expcount, true);
			if (p.cashcount) itr->puser->GiveBalance(p.cashcount);
			if (p.moneycount) itr->puser->GoldGain(p.moneycount, true);
			if (p.loyaltycount) itr->puser->SendLoyaltyChange("Fun Class", p.loyaltycount, false, false, true);

			for (int i = 0; i < 10; i++) {
				if (!p.itemid[i])
					continue;

				Packet newpkt(WIZ_DB_SAVE_USER, uint8(ProcDbType::Letter));
				newpkt << uint8(lettersendtype::cindirella) << counter - 1;
				g_pMain->AddDatabaseRequest(newpkt, itr->puser);
				break;
			}
		}

		counter = 0;
		foreach(itr, k_scoreboard) {
			counter++;
			if (counter > 200) break;

			auto p = pCindWar.pReward[counter - 1];
			if (!itr->puser || !itr->puser->isInGame())
				continue;

			if (p.expcount) itr->puser->ExpChange("Fun Class", p.expcount, true);
			if (p.cashcount) itr->puser->GiveBalance(p.cashcount);
			if (p.moneycount) itr->puser->GoldGain(p.moneycount, true);
			if (p.loyaltycount) itr->puser->SendLoyaltyChange("Fun Class", p.loyaltycount, false, false, true);

			for (int i = 0; i < 10; i++) {
				if (!p.itemid[i])
					continue;

				Packet newpkt(WIZ_DB_SAVE_USER, uint8(ProcDbType::Letter));
				newpkt << uint8(lettersendtype::cindirella) << counter - 1;
				g_pMain->AddDatabaseRequest(newpkt, itr->puser);
				break;
			}
		}
		pCindWar.Initialize(false);
		SendAnnouncement("Fun Class has Finished. Have Fun.", (uint8)Nation::ALL);
		KickOutZoneUsers(pCindWar.pSetting[pCindWar.settingid].zoneid, ZONE_MORADON);
	}
}
#pragma endregion

#pragma region CGameServerDlg::CindirellaCommand(bool start/* = false*/, int8 settingid, CUser *pUser)
bool CGameServerDlg::CindirellaCommand(bool start/* = false*/, int8 settingid, CUser* pUser) {

	if (start) {
		if (pCindWar.isStarted() || pCindWar.isPrepara()) {
			if (pUser)SendHelpDescription(pUser, "Fun Class event already open.");
			else printf("Fun Class event already open.\n");
			return true;
		}

		if (settingid < 0 || settingid > 4)
			return true;

		pCindWar.Initialize(false);

		if (m_CindirellaItemsArray[settingid].IsEmpty()) { printf("LoadCindirellaItemsTable error 1\n"); return false; }

		std::vector<_CINDWAR_ITEMS> mlist;
		m_CindirellaItemsArray[settingid].m_lock.lock();
		foreach_stlmap_nolock(thy, m_CindirellaItemsArray[settingid]) if (thy->second)mlist.push_back(*thy->second);
		m_CindirellaItemsArray[settingid].m_lock.unlock();

		if (mlist.size() != 375) { printf("Load Fun Class ItemsTable error 2\n"); return true; }

		int counter = 0;
		foreach(itr, mlist) {
			if (itr->slotid >= INVENTORY_TOTAL) continue;

			if (itr->iclass == 1) {
				pCindWar.m_warrior[itr->slotid].nNum = itr->itemid;
				pCindWar.m_warrior[itr->slotid].sDuration = itr->itemduration;
				pCindWar.m_warrior[itr->slotid].sCount = itr->itemcount;
				pCindWar.m_warrior[itr->slotid].bFlag = itr->itemflag;
				pCindWar.m_warrior[itr->slotid].nSerialNum = 0;
				pCindWar.m_warrior[itr->slotid].cindareaitem = true;
				pCindWar.m_warrior[itr->slotid].nExpirationTime = itr->itemexpire > 0 ? (uint32)UNIXTIME + (60 * 60 * itr->itemexpire) : 0;
			}
			if (itr->iclass == 2) {
				pCindWar.m_rogue[itr->slotid].nNum = itr->itemid;
				pCindWar.m_rogue[itr->slotid].sDuration = itr->itemduration;
				pCindWar.m_rogue[itr->slotid].sCount = itr->itemcount;
				pCindWar.m_rogue[itr->slotid].bFlag = itr->itemflag;
				pCindWar.m_rogue[itr->slotid].nSerialNum = 0;
				pCindWar.m_rogue[itr->slotid].cindareaitem = true;
				pCindWar.m_rogue[itr->slotid].nExpirationTime = itr->itemexpire > 0 ? (uint32)UNIXTIME + (60 * 60 * itr->itemexpire) : 0;
			}
			if (itr->iclass == 3) {
				pCindWar.m_mage[itr->slotid].nNum = itr->itemid;
				pCindWar.m_mage[itr->slotid].sDuration = itr->itemduration;
				pCindWar.m_mage[itr->slotid].sCount = itr->itemcount;
				pCindWar.m_mage[itr->slotid].bFlag = itr->itemflag;
				pCindWar.m_mage[itr->slotid].nSerialNum = 0;
				pCindWar.m_mage[itr->slotid].cindareaitem = true;
				pCindWar.m_mage[itr->slotid].nExpirationTime = itr->itemexpire > 0 ? (uint32)UNIXTIME + (60 * 60 * itr->itemexpire) : 0;
			}
			if (itr->iclass == 4) {
				pCindWar.m_priest[itr->slotid].nNum = itr->itemid;
				pCindWar.m_priest[itr->slotid].sDuration = itr->itemduration;
				pCindWar.m_priest[itr->slotid].sCount = itr->itemcount;
				pCindWar.m_priest[itr->slotid].bFlag = itr->itemflag;
				pCindWar.m_priest[itr->slotid].nSerialNum = 0;
				pCindWar.m_priest[itr->slotid].cindareaitem = true;
				pCindWar.m_priest[itr->slotid].nExpirationTime = itr->itemexpire > 0 ? (uint32)UNIXTIME + (60 * 60 * itr->itemexpire) : 0;
			}
			if (itr->iclass == 5) {
				pCindWar.m_kurian[itr->slotid].nNum = itr->itemid;
				pCindWar.m_kurian[itr->slotid].sDuration = itr->itemduration;
				pCindWar.m_kurian[itr->slotid].sCount = itr->itemcount;
				pCindWar.m_kurian[itr->slotid].bFlag = itr->itemflag;
				pCindWar.m_kurian[itr->slotid].nSerialNum = 0;
				pCindWar.m_kurian[itr->slotid].cindareaitem = true;
				pCindWar.m_kurian[itr->slotid].nExpirationTime = itr->itemexpire > 0 ? (uint32)UNIXTIME + (60 * 60 * itr->itemexpire) : 0;
			}
		}

		pCindWar.settingid = settingid;
		pCindWar.pSetting[pCindWar.settingid].zoneid = GetCindirellaForcedZone(pCindWar.pSetting[pCindWar.settingid].zoneid);
		pCindWar.starttime = UNIXTIME;
		pCindWar.preparetime = UNIXTIME + (pCindWar.pSetting[pCindWar.settingid].preparetime * MINUTE);
		pCindWar.prepare = true;

		KickOutZoneUsers(pCindWar.pSetting[pCindWar.settingid].zoneid, ZONE_MORADON);

		if(pCindWar.pSetting[pCindWar.settingid].zoneid != ZONE_RONARK_LAND)
			KickOutZoneUsers(ZONE_RONARK_LAND, ZONE_MORADON);

		if (pCindWar.pSetting[pCindWar.settingid].zoneid != ZONE_RONARK_LAND_BASE)
			KickOutZoneUsers(ZONE_RONARK_LAND_BASE, ZONE_MORADON);
		
		if (pCindWar.pSetting[pCindWar.settingid].zoneid != ZONE_ARDREAM)
			KickOutZoneUsers(ZONE_ARDREAM, ZONE_MORADON);

		{
			int botMinute = int(pCindWar.pSetting[pCindWar.settingid].preparetime + pCindWar.pSetting[pCindWar.settingid].playtime + 30);
			if (botMinute < 60)
				botMinute = 60;

			uint16 spawnedBots = SpawnCindirellaAutoBots(this, pCindWar.pSetting[pCindWar.settingid].zoneid, pCindWar.pSetting[pCindWar.settingid].beglevel, botMinute);
			printf("[Cindirella/FunClass] Auto bot spawned: %u zone=%u\n", spawnedBots, pCindWar.pSetting[pCindWar.settingid].zoneid);
		}

		SendAnnouncement("Fun Class Event Has Started. Have Fun.", (uint8)Nation::ALL);
		return true;
	}

	if (!pCindWar.isStarted() && !pCindWar.isPrepara()) {
		if (pUser)SendHelpDescription(pUser, "Fun Class event already close.");
		else printf("Fun Class event already close.\n");
		return true;
	}

	pCindWar.start = false;
	pCindWar.prepare = false;

	std::vector<std::string> mlist;
	pCindWar.e_listlock.lock();
	foreach(itr, pCindWar.e_list) {
		if (&itr->second)
			mlist.push_back(itr->first);
	}
	pCindWar.e_listlock.unlock();

	pCindWar.k_listlock.lock();
	foreach(itr, pCindWar.k_list)
		if (&itr->second)
			mlist.push_back(itr->first);
	pCindWar.k_listlock.unlock();

	foreach(itr, mlist) {
		auto* pUser = g_pMain->GetUserPtr(*itr, NameType::TYPE_CHARACTER);
		if (!pUser || !isCindirellaZone(pUser->GetZoneID()))
			continue;

		pUser->CindirellaSendFinish();
		pUser->ZoneChange(ZONE_MORADON, 0.0f, 0.0f, -1, false);
	}

	KickOutZoneUsers(pCindWar.pSetting[pCindWar.settingid].zoneid, ZONE_MORADON);
	pCindWar.Initialize(false);
	SendAnnouncement("Fun Class Event has Finished. Have Fun.", (uint8)Nation::ALL);
	return true;
}
#pragma endregion

void CGameServerDlg::CindirellaStart() {
	CindirellaTown();
	Packet newpkt(XSafe, uint8(XSafeOpCodes::CINDIRELLA));
	newpkt << uint8(cindopcode::starting) << CindirellaGetTime();
	Send_All(&newpkt, nullptr, (uint8)Nation::ALL, pCindWar.pSetting[pCindWar.settingid].zoneid);
}