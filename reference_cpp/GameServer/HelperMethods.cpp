#include "stdafx.h"

WORD	g_increase_serial = 1;

#pragma region CGameServerDlg::GenerateItemSerial()

/**
* @brief	Generates a new item serial.
*/
uint64 CGameServerDlg::GenerateItemSerial()
{
	static std::recursive_mutex _mutex;

	MYINT64 serial;
	MYSHORT	increase;
	serial.i = 0;

	time_t t = UNIXTIME;
	struct tm * ptm;
	ptm = gmtime(&t);

	Guard lock(_mutex);
	increase.w = g_increase_serial++;

	serial.b[7] = (uint8)(m_nServerNo);
	serial.b[6] = (uint8)(ptm->tm_year % 100);
	serial.b[5] = (uint8)(ptm->tm_mon);
	serial.b[4] = (uint8)(ptm->tm_mday);
	serial.b[3] = (uint8)(ptm->tm_hour);
	serial.b[2] = (uint8)(ptm->tm_min);
	serial.b[1] = increase.b[1];
	serial.b[0] = increase.b[0];

	return serial.i + increase.w;
}

#pragma endregion 

#pragma region CGameServerDlg::GenerateItemUniqueID()

/**
* @brief	Generates a new item unique ID.
*/
uint32 CGameServerDlg::GenerateItemUniqueID()
{
	m_iItemUniqueID++;
	return m_iItemUniqueID;
}

#pragma endregion 