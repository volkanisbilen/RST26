// Fake XIGNCODE loader: <game>\XIGNCODE\xldr_KnightOnline_NA_loader_win32.exe
//
// The original Launcher.exe (START) runs "<dir>\XIGNCODE\xldr... <dir>\KnightOnLine.exe", WAITS for it,
// and on exit code 0 starts KnightOnline.exe itself (ShellExecuteEx, "runas") and quits
// (Launcher.exe 0x41E1B6..0x41E33E). So the loader must not start the game itself and must return 0 right away:
// starting the game here and waiting for it kept the launcher open and made it start the game a
// second time after the first one closed.
//
// Launch guard (launch_ticket.h): when our parent is this client's original Launcher.exe (path + file hash), leave the
// one-shot ticket the game's d3d9.dll asks for. Started any other way: no ticket -> the game refuses to start.
//
// Build (x86): cl /nologo /O2 /MT fake_xldr.cpp /link /SUBSYSTEM:WINDOWS /OUT:xldr_KnightOnline_NA_loader_win32.exe
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#include "launch_ticket.h"
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "bcrypt.lib")

static bool ParentIsLauncher(DWORD* launcherPid)
{
    // <game>\XIGNCODE\xldr.exe -> <game>\Launcher.exe
    wchar_t self[MAX_PATH], want[MAX_PATH], got[MAX_PATH];
    GetModuleFileNameW(nullptr, self, MAX_PATH);
    wchar_t* s = wcsrchr(self, L'\\'); if (!s) return false; *s = 0;
    s = wcsrchr(self, L'\\'); if (!s) return false; *s = 0;
    swprintf_s(want, L"%s\\Launcher.exe", self);
    DWORD parent = LtParentPid(GetCurrentProcessId());
    if (!parent || !LtProcessPath(parent, got, MAX_PATH)) return false;
    if (_wcsicmp(got, want) != 0) return false;
    *launcherPid = parent;
    return LtLauncherHashOk(LtHashFile(got));
}

int WINAPI wWinMain(HINSTANCE, HINSTANCE, LPWSTR, int)
{
    DWORD launcher = 0;
    if (!ParentIsLauncher(&launcher))
        return 0;   // not from the launcher: no ticket (the game will say so)
    LaunchTicket t = {};
    t.magic = LT_MAGIC; t.time = LtNow(); t.pid = launcher;
    BCryptGenRandom(nullptr, t.rnd, sizeof(t.rnd), BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    t.mac = LtMac(t);
    HKEY k;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, LT_REG_KEY, 0, nullptr, 0, KEY_SET_VALUE, nullptr, &k, nullptr) == ERROR_SUCCESS)
    {
        RegSetValueExW(k, LT_REG_VALUE, 0, REG_BINARY, (const BYTE*)&t, sizeof(t));
        RegCloseKey(k);
    }
    return 0;   // "XIGNCODE ok" -> Launcher.exe starts KnightOnline.exe and closes
}
