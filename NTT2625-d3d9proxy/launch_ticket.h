// Launch ticket shared by fake_xldr.cpp (writes it) and launch_guard.cpp (d3d9 proxy, checks it).
//
// The original Launcher.exe runs <game>\XIGNCODE\xldr_KnightOnline_NA_loader_win32.exe and WAITS for it, then starts
// KnightOnline.exe and quits. Our xldr checks that its parent is the original Launcher.exe of this client (path + FNV-1a
// hash of the file) and leaves a one-shot ticket in HKCU\Software\HopeGuard\Launch. The game (d3d9 proxy, DllMain)
// reads and deletes it; no ticket / old ticket / bad MAC -> "start the game with Launcher.exe" and exit.
// The game server checks the proxy itself (LaunchGuard.cpp, WIZ_HSACS_HOOK + LAUNCHGUARD 0xF2), so deleting d3d9.dll
// does not get around this either.
#pragma once
#include <windows.h>
#include <tlhelp32.h>
#include <wchar.h>

#define LT_REG_KEY    L"Software\\HopeGuard"
#define LT_REG_VALUE  L"Launch"
static const DWORD LT_MAGIC = 0x544C4748;           // 'HGLT'
static const DWORD LT_MAX_AGE = 90;                 // seconds between xldr and the game's d3d9.dll
// FNV-1a 64 of every Launcher.exe we accept (python: see launch_guard notes); add a line when the launcher changes
static const unsigned long long LT_LAUNCHER_HASHES[] = {
    0x18b625d68751a673ull,   // USKO Launcher2_USA, 5 322 384 bytes (2026-08-14)
};

#pragma pack(push, 1)
struct LaunchTicket { DWORD magic, time, pid; BYTE rnd[16]; DWORD mac; };
#pragma pack(pop)

static DWORD LtMix(DWORD x)
{
    x ^= 0x5A17C3E9; x = (x << 13) | (x >> 19);
    x = x * 0x2C1B3C6D + 0x297A2D39; x ^= x >> 15;
    x *= 0x85EBCA6B; x ^= x >> 13;
    return x;
}

static DWORD LtMac(const LaunchTicket& t)
{
    DWORD h = LtMix(t.magic ^ t.time);
    h = LtMix(h ^ t.pid);
    for (int i = 0; i < 16; i += 4) h = LtMix(h ^ *(const DWORD*)(t.rnd + i));
    return h;
}

static DWORD LtNow()   // unix seconds
{
    FILETIME ft; GetSystemTimeAsFileTime(&ft);
    ULARGE_INTEGER u; u.LowPart = ft.dwLowDateTime; u.HighPart = ft.dwHighDateTime;
    return (DWORD)((u.QuadPart - 116444736000000000ull) / 10000000ull);
}

static unsigned long long LtHashFile(const wchar_t* path)
{
    HANDLE f = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, nullptr, OPEN_EXISTING, 0, nullptr);
    if (f == INVALID_HANDLE_VALUE) return 0;
    unsigned long long h = 0xcbf29ce484222325ull;
    static BYTE buf[65536];
    DWORD n = 0;
    while (ReadFile(f, buf, sizeof(buf), &n, nullptr) && n)
        for (DWORD i = 0; i < n; i++) { h ^= buf[i]; h *= 0x100000001b3ull; }
    CloseHandle(f);
    return h;
}

static DWORD LtParentPid(DWORD pid)
{
    HANDLE s = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (s == INVALID_HANDLE_VALUE) return 0;
    PROCESSENTRY32W e = { sizeof(e) };
    DWORD parent = 0;
    for (BOOL ok = Process32FirstW(s, &e); ok; ok = Process32NextW(s, &e))
        if (e.th32ProcessID == pid) { parent = e.th32ParentProcessID; break; }
    CloseHandle(s);
    return parent;
}

static bool LtProcessPath(DWORD pid, wchar_t* out, DWORD cch)
{
    HANDLE p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (!p) return false;
    BOOL ok = QueryFullProcessImageNameW(p, 0, out, &cch);
    CloseHandle(p);
    return ok != FALSE;
}

static bool LtLauncherHashOk(unsigned long long h)
{
    for (unsigned long long a : LT_LAUNCHER_HASHES) if (a == h) return true;
    return false;
}
