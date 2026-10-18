#include "stdafx.h"
#include "../shared/Condition.h"
#include "ConsoleInputThread.h"
#include "../shared/signal_handler.h"
#include "../shared/CrashHandler.h"
#include "GameServerDlg.h" // SetConsoleColor fonksiyonunu i�eren ba�l�k dosyas�
#include <psapi.h> // GetProcessMemoryInfo fonksiyonu i�in gerekli
#include <direct.h>
#include <string>
#include <fstream>

CGameServerDlg* g_pMain;
static Condition s_hEvent;

BOOL WINAPI _ConsoleHandler(DWORD dwCtrlType);

bool g_bRunning = true;
static bool g_consoleThreadStarted = false;

static void BootTrace(const char* msg)
{
	std::ofstream f("boot_trace.log", std::ios::app);
	if (!f.is_open())
		return;
	SYSTEMTIME st;
	GetLocalTime(&st);
	f << "[" << st.wHour << ":" << st.wMinute << ":" << st.wSecond << "." << st.wMilliseconds << "] " << msg << std::endl;
}

static void SetupRuntimeLogRedirection()
{
	_mkdir("logs");
	_mkdir("logs\\crash");

	SYSTEMTIME st;
	GetLocalTime(&st);
	char fileName[256] = { 0 };
	sprintf_s(fileName, "logs\\gameserver_%04u%02u%02u_%02u%02u%02u.log",
		st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);

	FILE* fpOut = nullptr;
	FILE* fpErr = nullptr;
	if (freopen_s(&fpOut, fileName, "a", stdout) == 0 && fpOut != nullptr)
		setvbuf(stdout, nullptr, _IOLBF, 0);

	if (freopen_s(&fpErr, fileName, "a", stderr) == 0 && fpErr != nullptr)
		setvbuf(stderr, nullptr, _IOLBF, 0);
}

static SIZE_T GetCurrentProcessMemoryUsageMB()
{
	PROCESS_MEMORY_COUNTERS pmc = {};
	if (GetProcessMemoryInfo(GetCurrentProcess(), &pmc, sizeof(pmc)))
		return (pmc.WorkingSetSize / (1024 * 1024));
	return 0;
}


int main()
{
	BootTrace("main: enter");
    //SetupRuntimeLogRedirection();
	BootTrace("main: runtime log redirection skipped");

    // 20.10.2020 Gameserver a��l�� s�resi hesaplama start
    DateTime time;
    clock_t x = clock();
    // 20.10.2020 Gameserver a��l�� s�resi hesaplama end
    // Override the console handler
    SetConsoleCtrlHandler(_ConsoleHandler, TRUE);
	BootTrace("main: SetConsoleCtrlHandler ok");

    // Start up the time updater thread
    StartTimeThread();
	BootTrace("main: StartTimeThread ok");

    // NOTE:
    // Crash/log hook stack (BugTrap + signal + custom handlers) can crash at very early startup
    // on some hosts. Keep startup path minimal/stable; re-enable after stability is confirmed.
    //HookSignals(&s_hEvent);
    //ExplosionHandle::SetupExceptionHandler(); // BugTrap 27.09.2020

    g_pMain = new CGameServerDlg();
	BootTrace("main: new CGameServerDlg ok");
    g_pMain->s_hEvent = &s_hEvent;
	BootTrace("main: assign s_hEvent ok");

    // Start up server
	BootTrace("main: calling Startup");
    if (g_pMain->Startup())
    {
		BootTrace("main: Startup success");
        // Start up the console input thread after successful startup
        StartConsoleInputThread();
        g_consoleThreadStarted = true;
		BootTrace("main: StartConsoleInputThread ok");

        // Reset Battle Zone Variables.
        g_pMain->ResetBattleZone(BATTLEZONE_NONE);

        g_pMain->SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_INTENSITY); // Yellow color
        printf("***********************************************\n");
        printf("* Serveriniz Basarili Olarak Baslatildi Oyuna Giris Aktif.\n");
        printf("** > GameServer Baslama Hizi (%.2lf Seconds) on %04d-%02d-%02d at %02d:%02d\n",
            (clock() - x) / (double)CLOCKS_PER_SEC, time.GetYear(), time.GetMonth(),
            time.GetDay(), time.GetHour(), time.GetMinute());
        printf("* Eglenceli Oyunlar Dilerim\n");
        printf("***********************************************\n");
        g_pMain->SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color

        // Wait until console's signaled as closing
        s_hEvent.Wait();
    }
    else
    {
		BootTrace("main: Startup returned false");
        system("pause");
    }

    g_pMain->SetConsoleColor(FOREGROUND_RED | FOREGROUND_INTENSITY);
    printf("Server is shutting down, please wait...\n");
    g_pMain->SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE);

    g_bRunning = false;

    delete g_pMain;
	BootTrace("main: delete g_pMain ok");

    CleanupTimeThread();
	BootTrace("main: CleanupTimeThread ok");
    if (g_consoleThreadStarted)
        CleanupConsoleInputThread();
	BootTrace("main: CleanupConsoleInputThread ok");
    //UnhookSignals();

    return 0;
}

BOOL WINAPI _ConsoleHandler(DWORD dwCtrlType)
{
    s_hEvent.BeginSynchronized();
    s_hEvent.Signal();
    s_hEvent.EndSynchronized();
    sleep(10000); // Win7 onwards allows 10 seconds before it'll forcibly terminate
    return TRUE;
}
