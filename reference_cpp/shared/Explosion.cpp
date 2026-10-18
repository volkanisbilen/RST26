#include "stdafx.h"
#include "Explosion.h"

#include "BugTrap.h"

#pragma comment(lib, "../shared/BugTrapx64.lib")   

ExplosionHandle::ExplosionHandle()
{

}

#pragma region ConsoleColorHandle::SetupExceptionHandler()
void ExplosionHandle::SetupExceptionHandler()
{
	BT_InstallSehFilter();
	BT_SetAppName(_T("GameServer"));
	BT_SetSupportEMail(_T("kofilozof2021@gmail.com"));
	BT_SetFlags(BTF_DETAILEDMODE | BTF_EDITMAIL | BTF_ATTACHREPORT | BTF_SCREENCAPTURE);
	BT_SetSupportServer(_T("localhost"), 9999);
	BT_SetSupportURL(_T("https://www.rimaguard.com"));
}

#pragma endregion 
