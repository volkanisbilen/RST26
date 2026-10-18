#include "stdafx.h"
#include "../shared/signal_handler.h"

LoginServer * g_pMain;
static Condition s_hEvent;
bool g_bRunning = true;

BOOL WINAPI _ConsoleHandler(DWORD dwCtrlType);

int main()
{
	SetConsoleTitle("JstKO (LogIn  Server) v" STRINGIFY(__VERSION));

	// Override the console handler
	SetConsoleCtrlHandler(_ConsoleHandler, TRUE);

	HookSignals(&s_hEvent);

	g_pMain = new LoginServer();
	g_pMain->s_hEvent = &s_hEvent;

	// Startup server
	if (g_pMain->Startup())
	{
		printf("\nLogin Server Basariyla Baslatildi, Oyuna Giris Aktif \n\n");

		// Wait until console's signaled as closing
		s_hEvent.Wait();
	}
	else
	{
		system("pause");
	}

	printf("Server kapatiliyor, Lutfen bekleyin...\n");

	g_bRunning = false; 
	delete g_pMain;
	UnhookSignals();

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
