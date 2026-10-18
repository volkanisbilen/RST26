#include "stdafx.h"
#include "DBAgent.h"

#pragma region CUser::HandleJobChangeScroll(Packet& pkt)
void CUser::HandleJobChangeScroll(Packet& pkt)
{
	if (!isInGame())
		return;

	uint8 selectedJob = 0;
	pkt >> selectedJob;

	if (selectedJob < 1 || selectedJob > 5)
	{
		XSafe_SendMessageBox("Class Change", "Invalid class selection.");
		return;
	}

	if (isDead()
		|| isTrading()
		|| isMerchanting()
		|| isStoreOpen()
		|| isSellingMerchant()
		|| isBuyingMerchant()
		|| isMining()
		|| isFishing())
	{
		XSafe_SendMessageBox("Class Change", "You cannot change class right now.");
		return;
	}

	if (isWarrior() && selectedJob == 1
		|| isRogue() && selectedJob == 2
		|| isMage() && selectedJob == 3
		|| isPriest() && selectedJob == 4
		|| isPortuKurian() && selectedJob == 5)
	{
		XSafe_SendMessageBox("Class Change", "You are already this class.");
		return;
	}

	if (!CheckExistItem(ITEM_JOB_CHANGE))
	{
		XSafe_SendMessageBox("Class Change", "Class Change Scroll is required.");
		return;
	}

	uint8 result = JobChange(0, selectedJob);
	switch (result)
	{
	case 1:
		XSafe_SendMessageBox("Class Change", "Class changed successfully. Please login again.");
		break;
	case 4:
		XSafe_SendMessageBox("Class Change", "Please unequip all items first.");
		break;
	case 5:
		XSafe_SendMessageBox("Class Change", "Class change failed.");
		break;
	case 6:
		XSafe_SendMessageBox("Class Change", "Class Change Scroll is required.");
		break;
	default:
		XSafe_SendMessageBox("Class Change", "Class change failed.");
		break;
	}
}
#pragma endregion
#pragma region GenderChangeV2(Packet & pkt)
void CUser::GenderChangeV2(Packet & pkt)
{
	Packet result(WIZ_GENDER_CHANGE);

	if (isDead()
		|| isTrading()
		|| isMerchanting()
		|| isStoreOpen()
		|| isSellingMerchant()
		|| isBuyingMerchant()
		|| isMining()
		|| isFishing())
		return;

	int8 opcode, gRace, gFace, Results = 0;
	uint32 bHair;

	pkt >> opcode >> gRace >> gFace >> bHair;

	if (!CheckExistItem(ITEM_GENDER_CHANGE))
		goto fail_return;

	if (gRace == 0 || gFace == 0 || bHair == 0)
		goto fail_return;

	if (gRace < 10 && GetNation() != 1 || (gRace > 10 && GetNation() != 2) || (gRace > 5 && GetNation() == 1))
		goto fail_return;



	m_bRace = gRace;
	m_bFace = gFace;
	m_nHair = bHair;
	if (GetHealth() < (GetMaxHealth() / 2))
		HpChange(GetMaxHealth());

	SendMyInfo();

	UserInOut(INOUT_OUT);
	SetRegion(GetNewRegionX(), GetNewRegionZ());
	UserInOut(INOUT_WARP);

	g_pMain->UserInOutForMe(this);
	NpcInOutForMe();
	g_pMain->MerchantUserInOutForMe(this);


	ResetWindows();

	InitType4();
	RecastSavedMagic();

	RobItem(ITEM_GENDER_CHANGE);

	result << uint8(1) << m_bRace
		<< m_bFace
		<< m_nHair
		<< GetClass();
	Send(&result);


	return;
fail_return:
	result << Results;
	Send(&result);
}
#pragma endregion


#pragma region Gender Change
bool CUser::GenderChange(uint8 Race /*= 0*/)
{
	if (Race == 0 || Race > 13)
		return false;

	if (!CheckExistItem(ITEM_GENDER_CHANGE))
		return false;

	RobItem(ITEM_GENDER_CHANGE);

	m_bRace = Race;

	if (GetHealth() < (GetMaxHealth() / 2))
		HpChange(GetMaxHealth());

	SendMyInfo();

	UserInOut(INOUT_OUT);
	SetRegion(GetNewRegionX(), GetNewRegionZ());
	UserInOut(INOUT_WARP);

	g_pMain->UserInOutForMe(this);
	NpcInOutForMe();
	g_pMain->MerchantUserInOutForMe(this);

	ResetWindows();

	InitType4();
	RecastSavedMagic();

	return true;
}
#pragma endregion

#pragma region Gender Change Game Master
bool CUser::GenderChangeGM(uint8 Race /*= 0*/)
{
	if (Race == 0 || Race > 13)
		return false;

	m_bRace = Race;

	if (GetHealth() < (GetMaxHealth() / 2))
		HpChange(GetMaxHealth());

	SendMyInfo();

	UserInOut(INOUT_OUT);
	SetRegion(GetNewRegionX(), GetNewRegionZ());
	UserInOut(INOUT_WARP);

	g_pMain->UserInOutForMe(this);
	NpcInOutForMe();
	g_pMain->MerchantUserInOutForMe(this);

	ResetWindows();

	InitType4();
	RecastSavedMagic();

	return true;
}
#pragma endregion

uint8 CUser::JobChange(uint8 type, uint8 NewJob) 
{
	
	uint8 bNewClass = 0, bNewRace = 0;

	if (NewJob < 1 || NewJob > 5 || (type != 0 && type != 1))
		return 5;

	if (type == 0 && !CheckExistItem(ITEM_JOB_CHANGE))
		return 6;

	if (type == 1 && !CheckExistItem(ITEM_JOB_CHANGE2))
		return 6;

	if (isWarrior() && NewJob == 1) return 6;
	if (isRogue() && NewJob == 2) return 6;
	if (isMage() && NewJob == 3) return 6;
	if (isPriest() && NewJob == 4) return 6;
	if (isPortuKurian() && NewJob == 5) return 6;

	for (int i = 0; i < SLOT_MAX; i++) {
		if (m_sItemArray[i].nNum) {
			Packet result(WIZ_CLASS_CHANGE, uint8(ALL_POINT_CHANGE));
			result << uint8(4) << int(0);
			Send(&result);
			return 4;
		}
	}

	if (NewJob == 1)
	{
		if (isBeginnerRogue() || isBeginnerMage() || isBeginnerPriest() || isBeginnerKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = KARUWARRIOR;
				bNewRace = KARUS_BIG;
			}
			else
			{
				bNewClass = ELMORWARRRIOR;

				if (GetRace() == PORUTU)
					bNewRace = BABARIAN;
				else
					bNewRace = GetRace();
			}
		}
		else if (isNoviceRogue() || isNoviceMage() || isNovicePriest() || isNoviceKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = BERSERKER;
				bNewRace = KARUS_BIG;
			}
			else
			{
				bNewClass = BLADE;

				if (GetRace() == PORUTU)
					bNewRace = BABARIAN;
				else
					bNewRace = GetRace();
			}
		}
		else if (isMasteredRogue() || isMasteredMage() || isMasteredPriest() || isMasteredKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				if (type == 1)
					bNewClass = BERSERKER;
				else
					bNewClass = GUARDIAN;

				bNewRace = KARUS_BIG;
			}
			else
			{
				if (type == 1)
					bNewClass = BLADE;
				else
					bNewClass = PROTECTOR;

				if (GetRace() == PORUTU)
					bNewRace = BABARIAN;
				else
					bNewRace = GetRace();
			}
		}
	}
	else if (NewJob == 2)
	{
		if (isBeginnerWarrior() || isBeginnerMage() || isBeginnerPriest() || isBeginnerKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = KARUROGUE;
				bNewRace = KARUS_MIDDLE;
			}
			else
			{
				bNewClass = ELMOROGUE;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
		else if (isNoviceWarrior() || isNoviceMage() || isNovicePriest() || isNoviceKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = HUNTER;
				bNewRace = KARUS_MIDDLE;
			}
			else
			{
				bNewClass = RANGER;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
		else if (isMasteredWarrior() || isMasteredMage() || isMasteredPriest() || isMasteredKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				if (type == 1)
					bNewClass = HUNTER;
				else
					bNewClass = PENETRATOR;

				bNewRace = KARUS_MIDDLE;
			}
			else
			{
				if (type == 1)
					bNewClass = RANGER;
				else
					bNewClass = ASSASSIN;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
	}
	else if (NewJob == 3)
	{
		if (isBeginnerWarrior() || isBeginnerRogue() || isBeginnerPriest() || isBeginnerKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = KARUWIZARD;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_MIDDLE || GetRace() == KURIAN)
					bNewRace = KARUS_SMALL;
				else
					bNewRace = GetRace();
			}
			else
			{
				bNewClass = ELMOWIZARD;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
		else if (isNoviceWarrior() || isNoviceRogue() || isNovicePriest() || isNoviceKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = SORSERER;
				if (GetRace() == KARUS_BIG || GetRace() == KARUS_MIDDLE || GetRace() == KURIAN)
					bNewRace = KARUS_SMALL;
				else
					bNewRace = GetRace();
			}
			else
			{
				bNewClass = MAGE;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
		else if (isMasteredWarrior() || isMasteredRogue() || isMasteredPriest() || isMasteredKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				if (type == 1)
					bNewClass = SORSERER;
				else
					bNewClass = NECROMANCER;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_MIDDLE || GetRace() == KURIAN)
					bNewRace = KARUS_SMALL;
				else
					bNewRace = GetRace();
			}
			else
			{
				if (type == 1)
					bNewClass = MAGE;
				else
					bNewClass = ENCHANTER;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
	}
	else if (NewJob == 4)
	{
		if (isBeginnerWarrior() || isBeginnerRogue() || isBeginnerMage() || isBeginnerKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = KARUPRIEST;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_SMALL || GetRace() == KURIAN)
					bNewRace = KARUS_MIDDLE;
				else
					bNewRace = GetRace();
			}
			else
			{
				bNewClass = ELMOPRIEST;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
		else if (isNoviceWarrior() || isNoviceRogue() || isNoviceMage() || isNoviceKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = SHAMAN;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_SMALL || GetRace() == KURIAN)
					bNewRace = KARUS_MIDDLE;
				else
					bNewRace = GetRace();
			}
			else
			{
				bNewClass = CLERIC;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
		else if (isMasteredWarrior() || isMasteredRogue() || isMasteredMage() || isMasteredKurianPortu())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				if (type == 1)
					bNewClass = SHAMAN;
				else
					bNewClass = DARKPRIEST;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_SMALL || GetRace() == KURIAN)
					bNewRace = KARUS_MIDDLE;
				else
					bNewRace = GetRace();
			}
			else
			{
				if (type == 1)
					bNewClass = CLERIC;
				else
					bNewClass = DRUID;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}
	}
	else if (NewJob == 5)
	{
		if (isBeginnerWarrior() || isBeginnerRogue() || isBeginnerMage() || isBeginnerPriest())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = KURIANSTARTER;
				bNewRace = KURIAN;
			}
			else
			{
				bNewClass = PORUTUSTARTER;
				bNewRace = PORUTU;
			}
		}
		else if (isNoviceWarrior() || isNoviceRogue() || isNoviceMage() || isNovicePriest())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = KURIANNOVICE;
				bNewRace = KURIAN;
			}
			else
			{
				bNewClass = PORUTUNOVICE;
				bNewRace = PORUTU;
			}
		}
		else if (isMasteredWarrior() || isMasteredRogue() || isMasteredMage() || isMasteredPriest())
		{
			if (GetNation() == (uint8)Nation::KARUS)
			{
				bNewClass = KURIANMASTER;
				bNewRace = KURIAN;
			}
			else
			{
				bNewClass = PORUTUMASTER;
				bNewRace = PORUTU;
			}
		}
	}
	else return 7;

	if (!bNewClass || !bNewRace)
		return 5;

	if (type == 0 && !RobItem(ITEM_JOB_CHANGE))
		return 6;
	else if (type == 1 && !RobItem(ITEM_JOB_CHANGE2))
		return 6;

	JobChangeInsertLog(m_sClass, bNewClass, m_bRace, bNewRace);

	m_sClass = bNewClass; m_bRace = bNewRace;

	std::vector<uint16> metclist;
	_getEtcList(metclist, true);

	if (type == 0) {
		foreach(itr, metclist)
			SaveEvent(*itr, 4);
	}

	AllPointChange(true);
	AllSkillPointChange(true);

	SendMyInfo();
	UserInOut(INOUT_OUT);
	SetRegion(GetNewRegionX(), GetNewRegionZ());
	UserInOut(INOUT_WARP);


	g_pMain->UserInOutForMe(this);
	NpcInOutForMe();
	g_pMain->MerchantUserInOutForMe(this);

	/*RegionNpcInfoForMe();
	RegionUserInOutForMe();
	MerchantUserInOutForMe();*/

	ResetWindows();
	InitType4();
	RecastSavedMagic();
	goDisconnect("Character Changed Job.", __FUNCTION__);
	return 1;
}

uint8 CUser::JobChangeGM(uint8 NewJob /*= 0*/)
{
	uint8 bNewClass = 0, bNewRace = 0;
	uint8 bResult = 0;

	if (NewJob < 1 || NewJob > 5)
		return 5; // Unknown job is selected...

	for (int i = 0; i < SLOT_MAX; i++)
	{
		if (m_sItemArray[i].nNum)
		{
			bResult = 7;
			break;
		}
	}

	if (bResult == 7)
	{
		Packet result(WIZ_CLASS_CHANGE, uint8(ALL_POINT_CHANGE));
		result << uint8(4) << int(0);
		Send(&result);
		return bResult; // While there are items equipped on you.
	}

	// If bResult between 1 and 4 character already selected job...

	// If selected a new job Warrior
	if (NewJob == 1)
	{
		if (GetNation() == (uint8)Nation::KARUS)
		{
			// Beginner Karus Rogue, Magician, Priest
			if (isBeginnerRogue() || isBeginnerMage() || isBeginnerPriest() || isBeginnerKurianPortu())
			{
				bNewClass = KARUWARRIOR;
				bNewRace = KARUS_BIG;
			}
			// Skilled Karus Rogue, Magician, Priest
			else if (isNoviceRogue() || isNoviceMage() || isNovicePriest() || isNoviceKurianPortu())
			{
				bNewClass = BERSERKER;
				bNewRace = KARUS_BIG;
			}
			// Mastered Karus Rogue, Magician, Priest
			else if (isMasteredRogue() || isMasteredMage() || isMasteredPriest() || isMasteredKurianPortu())
			{
				bNewClass = GUARDIAN;
				bNewRace = KARUS_BIG;
			}
		}
		else
		{
			// Beginner El Morad Rogue, Magician, Priest
			if (isBeginnerRogue() || isBeginnerMage() || isBeginnerPriest() || isBeginnerKurianPortu())
			{
				bNewClass = ELMORWARRRIOR;

				if (GetRace() == PORUTU)
					bNewRace = BABARIAN;
				else
					bNewRace = GetRace();
			}
			// Skilled El Morad Rogue, Magician, Priest
			else if (isNoviceRogue() || isNoviceMage() || isNovicePriest() || isNoviceKurianPortu())
			{
				bNewClass = BLADE;

				if (GetRace() == PORUTU)
					bNewRace = BABARIAN;
				else
					bNewRace = GetRace();
			}
			// Mastered El Morad Rogue, Magician, Priest
			else if (isMasteredRogue() || isMasteredMage() || isMasteredPriest() || isMasteredKurianPortu())
			{
				bNewClass = PROTECTOR;

				if (GetRace() == PORUTU)
					bNewRace = BABARIAN;
				else
					bNewRace = GetRace();
			}
		}

		// Character already Warrior.
		if (bNewClass == 0 || bNewRace == 0)
			bResult = NewJob;
	}

	// If selected a new job Rogue
	if (NewJob == 2)
	{
		if (GetNation() == (uint8)Nation::KARUS)
		{
			// Beginner Karus Warrior, Magician, Priest
			if (isBeginnerWarrior() || isBeginnerMage() || isBeginnerPriest() || isBeginnerKurianPortu())
			{
				bNewClass = KARUROGUE;
				bNewRace = KARUS_MIDDLE;
			}
			// Skilled Karus Warrior, Magician, Priest
			else if (isNoviceWarrior() || isNoviceMage() || isNovicePriest() || isNoviceKurianPortu())
			{
				bNewClass = HUNTER;
				bNewRace = KARUS_MIDDLE;
			}
			// Mastered Karus Warrior, Magician, Priest
			else if (isMasteredWarrior() || isMasteredMage() || isMasteredPriest() || isMasteredKurianPortu())
			{
				bNewClass = PENETRATOR;
				bNewRace = KARUS_MIDDLE;
			}
		}
		else
		{
			// Beginner El Morad Warrior, Magician, Priest
			if (isBeginnerWarrior() || isBeginnerMage() || isBeginnerPriest() || isBeginnerKurianPortu())
			{
				bNewClass = ELMOROGUE;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
			// Skilled El Morad Warrior, Magician, Priest
			else if (isNoviceWarrior() || isNoviceMage() || isNovicePriest() || isNoviceKurianPortu())
			{
				bNewClass = RANGER;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
			// Mastered El Morad Warrior, Magician, Priest
			else if (isMasteredWarrior() || isMasteredMage() || isMasteredPriest() || isMasteredKurianPortu())
			{
				bNewClass = ASSASSIN;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}

		// Character already Rogue.
		if (bNewClass == 0 || bNewRace == 0)
			bResult = NewJob;
	}

	// If selected a new job Magician
	if (NewJob == 3)
	{
		if (GetNation() == (uint8)Nation::KARUS)
		{
			// Beginner Karus Warrior, Rogue, Priest
			if (isBeginnerWarrior() || isBeginnerRogue() || isBeginnerPriest() || isBeginnerKurianPortu())
			{
				bNewClass = KARUWIZARD;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_MIDDLE || GetRace() == KURIAN)
					bNewRace = KARUS_SMALL;
				else
					bNewRace = GetRace();
			}
			// Skilled Karus Warrior, Rogue, Priest
			else if (isNoviceWarrior() || isNoviceRogue() || isNovicePriest() || isNoviceKurianPortu())
			{
				bNewClass = SORSERER;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_MIDDLE || GetRace() == KURIAN)
					bNewRace = KARUS_SMALL;
				else
					bNewRace = GetRace();
			}
			// Mastered Karus Warrior, Rogue, Priest
			else if (isMasteredWarrior() || isMasteredRogue() || isMasteredPriest() || isMasteredKurianPortu())
			{
				bNewClass = NECROMANCER;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_MIDDLE || GetRace() == KURIAN)
					bNewRace = KARUS_SMALL;
				else
					bNewRace = GetRace();
			}
		}
		else
		{
			// Beginner El Morad Warrior, Rogue, Priest
			if (isBeginnerWarrior() || isBeginnerRogue() || isBeginnerPriest() || isBeginnerKurianPortu())
			{
				bNewClass = ELMOWIZARD;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
			// Skilled El Morad Warrior, Rogue, Priest
			else if (isNoviceWarrior() || isNoviceRogue() || isNovicePriest() || isNoviceKurianPortu())
			{
				bNewClass = MAGE;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
			// Mastered El Morad Warrior, Rogue, Priest
			else if (isMasteredWarrior() || isMasteredRogue() || isMasteredPriest() || isMasteredKurianPortu())
			{
				bNewClass = ENCHANTER;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}

		// Character already Magician.
		if (bNewClass == 0 || bNewRace == 0)
			bResult = NewJob;
	}

	// If selected a new job Priest
	if (NewJob == 4)
	{
		if (GetNation() == (uint8)Nation::KARUS)
		{
			// Beginner Karus Warrior, Rogue, Magician
			if (isBeginnerWarrior() || isBeginnerRogue() || isBeginnerMage() || isBeginnerKurianPortu())
			{
				bNewClass = KARUPRIEST;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_SMALL || GetRace() == KURIAN)
					bNewRace = KARUS_MIDDLE;
				else
					bNewRace = GetRace();
			}
			// Skilled Karus Warrior, Rogue, Magician
			else if (isNoviceWarrior() || isNoviceRogue() || isNoviceMage() || isNoviceKurianPortu())
			{
				bNewClass = SHAMAN;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_SMALL || GetRace() == KURIAN)
					bNewRace = KARUS_MIDDLE;
				else
					bNewRace = GetRace();
			}
			// Mastered Karus Warrior, Rogue, Magician
			else if (isMasteredWarrior() || isMasteredRogue() || isMasteredMage() || isMasteredKurianPortu())
			{
				bNewClass = DARKPRIEST;

				if (GetRace() == KARUS_BIG || GetRace() == KARUS_SMALL || GetRace() == KURIAN)
					bNewRace = KARUS_MIDDLE;
				else
					bNewRace = GetRace();
			}
		}
		else
		{
			// Beginner El Morad Warrior, Rogue, Magician
			if (isBeginnerRogue() || isBeginnerWarrior() || isBeginnerMage() || isBeginnerKurianPortu())
			{
				bNewClass = ELMOPRIEST;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
			// Skilled El Morad Warrior, Rogue, Magician
			else if (isNoviceRogue() || isNoviceWarrior() || isNoviceMage() || isNoviceKurianPortu())
			{
				bNewClass = CLERIC;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
			// Mastered El Morad Warrior, Rogue, Magician
			else if (isMasteredRogue() || isMasteredWarrior() || isMasteredMage() || isMasteredKurianPortu())
			{
				bNewClass = DRUID;

				if (GetRace() == BABARIAN || GetRace() == PORUTU)
					bNewRace = ELMORAD_MAN;
				else
					bNewRace = GetRace();
			}
		}

		// Character already Priest.
		if (bNewClass == 0 || bNewRace == 0)
			bResult = NewJob;
	}

	// If selected a new job Priest
	if (NewJob == 5)
	{
		if (GetNation() == (uint8)Nation::KARUS)
		{
			// Beginner Karus Warrior, Rogue, Magician
			if (isBeginnerWarrior() || isBeginnerRogue() || isBeginnerMage() || isBeginnerPriest())
			{
				bNewClass = KURIANSTARTER;
				bNewRace = KURIAN;
			}
			// Skilled Karus Warrior, Rogue, Magician
			else if (isNoviceWarrior() || isNoviceRogue() || isNoviceMage() || isNovicePriest())
			{
				bNewClass = KURIANNOVICE;
				bNewRace = KURIAN;
			}
			// Mastered Karus Warrior, Rogue, Magician
			else if (isMasteredWarrior() || isMasteredRogue() || isMasteredMage() || isMasteredPriest())
			{
				bNewClass = KURIANMASTER;
				bNewRace = KURIAN;
			}
		}
		else
		{
			// Beginner El Morad Warrior, Rogue, Magician
			if (isBeginnerRogue() || isBeginnerWarrior() || isBeginnerMage() || isBeginnerPriest())
			{
				bNewClass = PORUTUSTARTER;
				bNewRace = PORUTU;
			}
			// Skilled El Morad Warrior, Rogue, Magician
			else if (isNoviceRogue() || isNoviceWarrior() || isNoviceMage() || isNovicePriest())
			{
				bNewClass = PORUTUNOVICE;
				bNewRace = PORUTU;
			}
			// Mastered El Morad Warrior, Rogue, Magician
			else if (isMasteredRogue() || isMasteredWarrior() || isMasteredMage() || isMasteredPriest())
			{
				bNewClass = PORUTUMASTER;
				bNewRace = PORUTU;
			}
		}
		// Character already Priest.
		if (bNewClass == 0 || bNewRace == 0)
			bResult = NewJob;
	}

	if (bResult == 0)
	{
		JobChangeInsertLog(m_sClass, bNewClass, m_bRace, bNewRace);

		m_sClass = bNewClass; m_bRace = bNewRace;

		std::vector<uint16> metclist;
		_getEtcList(metclist, true);

		foreach(itr, metclist)
			SaveEvent(*itr, 4);

		AllPointChange(true);
		AllSkillPointChange(true);

		SendMyInfo();
		UserInOut(INOUT_OUT);
		SetRegion(GetNewRegionX(), GetNewRegionZ());
		UserInOut(INOUT_WARP);

		g_pMain->UserInOutForMe(this);
		NpcInOutForMe();
		g_pMain->MerchantUserInOutForMe(this);

		/*RegionNpcInfoForMe();
		RegionUserInOutForMe();
		MerchantUserInOutForMe();*/

		ResetWindows();
		InitType4();
		RecastSavedMagic();
	}

	return bResult;
}