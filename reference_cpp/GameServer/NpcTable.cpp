#include "stdafx.h"
#include "MagicInstance.h"
#include "../shared/DateTime.h"

using namespace std;

CNpcTable::CNpcTable() { Initialize(); }//x0x0x0

void CNpcTable::Initialize()//x0x0x0
{
	m_sSid = 0;
	m_strName = "";
	m_sPid = 0;
	m_sSize = 100;
	m_iWeapon_1 = 0;
	m_iWeapon_2 = 0;
	m_byGroup = 0;
	m_byActType = 0;
	m_tNpcType = 0;
	m_byFamilyType = 0;
	m_byRank = 0;
	m_byTitle = 0;
	m_iSellingGroup = 0;
	m_sLevel = 0;
	m_iExp = 0;
	m_iLoyalty = 0;
	m_MaxHP = 0;
	m_MaxMP = 0;
	m_sAttack = 0;
	m_sDefense = 0;
	m_sHitRate = 0;
	m_sEvadeRate = 0;
	m_sDamage = 0;
	m_sAttackDelay = 0;
	m_sSpeed = MONSTER_SPEED;
	m_bySpeed_1 = 0;
	m_bySpeed_2 = 0;
	m_sStandTime = 1000;
	m_iMagic1 = 0;
	m_iMagic2 = 0;
	m_iMagic3 = 0;
	m_byFireR = 0;
	m_byColdR = 0;
	m_byLightningR = 0;
	m_byMagicR = 0;
	m_byDiseaseR = 0;
	m_byPoisonR = 0;
	m_bySearchRange = 0;
	m_byAttackRange = 0;
	m_byTracingRange = 0;
	m_iMoney = 0;
	m_iItem = 0;
	m_byDirectAttack = 0;
	m_byMagicAttack = 0;
	m_byGroupSpecial = 0;
	m_fBulk = 0.0f;
}