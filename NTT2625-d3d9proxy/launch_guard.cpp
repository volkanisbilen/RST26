// Launch guard, client side (see launch_ticket.h for the whole chain).
//   - DllMain (KnightOnLine.exe only): the one-shot ticket our xldr leaves when the original Launcher.exe runs it must be
//     there, fresh and intact; it is deleted right away. Otherwise: message box + exit.
//   - Game server challenge, WIZ_HSACS_HOOK + LAUNCHGUARD (0xF2), server LaunchGuard.cpp:
//       S->C 1 [u32 nonce]                  sent at game start (also after a soft re-connect)
//       C->S 2 [u32 answer][u32 proxyVer]   answer = LgAnswer(nonce); no / wrong answer -> the server disconnects
//   - HopeGuard ACS heartbeat (server LaunchGuard.cpp / HopeGuard.cpp), same channel:
//       S->C 3 [u32 nonce]                  every HEARTBEAT_INTERVAL s; no answer -> disconnect
//       C->S 4 [u32 answer][u32 tickMs][u32 qpcMs][u32 flags][u32 tblHash][str details]
//       S->C 6 [str reason]                 HopeGuard ACS disconnects us right after: no soft / hard re-connect
//         tickMs / qpcMs: our GetTickCount / QueryPerformanceCounter clocks (the server compares them with its own:
//         speed hack); flags 1 debugger, 2 cheat tool (process / window), 4 foreign DLLs in the game, 8 tables unreadable;
//         tblHash: FNV-1a over Data\*.tbl (names + contents). A scan thread refreshes the flags every 20 s.
#include <windows.h>
#include <stdio.h>
#include <vector>
#include <deque>
#include <string>
#include <algorithm>
#include <tlhelp32.h>
#include "launch_ticket.h"
static const DWORD KO_SND_FNC = 0x00704070;
static const DWORD KO_PTR_PKT = 0x01115914;   // CAPISocket*
#pragma comment(lib, "advapi32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();
void Reconnect_Block();

static const BYTE  LG_SUBOP = 0xF2;
static const DWORD LG_PROXY_VERSION = 20261004;   // raise with every d3d9.dll the server must insist on (GameServer.ini MIN_PROXY)

static DWORD LgAnswer(DWORD nonce) { return LtMix(LtMix(nonce ^ 0x3C6EF372) ^ LG_PROXY_VERSION); }
static DWORD HgAnswer(DWORD nonce, DWORD tick, DWORD qpc, DWORD flags, DWORD tbl)
{
    DWORD x = LtMix(LgAnswer(nonce ^ 0xA5C3E1F7));
    x = LtMix(x ^ tick); x = LtMix(x ^ (qpc * 3u)); x = LtMix(x ^ (flags * 0x9E3779B1u)); x = LtMix(x ^ tbl);
    return x;
}

// ---------------------------------------------------------------- HopeGuard ACS client checks
enum : DWORD { HG_DEBUGGER = 1, HG_CHEATTOOL = 2, HG_MODULE = 4, HG_NOTABLES = 8 };
static CRITICAL_SECTION g_hgLock;
static DWORD g_hgFlags = 0;
static std::string g_hgDetails;
static volatile DWORD g_hgTbl = 0;
static volatile LONG g_hgTblReady = 0;                // 1 hashed, 2 tables unreadable

static std::string Lower(std::string s) { for (char& c : s) c = (char)tolower((unsigned char)c); return s; }
static bool EndsWith(const std::string& s, const char* e) { size_t n = strlen(e); return s.size() >= n && s.compare(s.size() - n, n, e) == 0; }

// FNV-1a over every Data\*.tbl (sorted names + contents); "*.tbl" also matches the .tbl.before_* backups through
// their 8.3 names, those are skipped
static void HashTables()
{
    char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH); *(strrchr(dir, '\\') + 1) = 0;
    std::string base = std::string(dir) + "Data\\";
    std::vector<std::string> names;
    WIN32_FIND_DATAA fd; HANDLE h = FindFirstFileA((base + "*.tbl").c_str(), &fd);
    if (h != INVALID_HANDLE_VALUE)
    {
        do { std::string n = Lower(fd.cFileName); if (!(fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) && EndsWith(n, ".tbl")) names.push_back(n); }
        while (FindNextFileA(h, &fd));
        FindClose(h);
    }
    std::sort(names.begin(), names.end());
    DWORD hash = 0x811C9DC5; bool ok = !names.empty();
    std::vector<BYTE> buf(1 << 16);
    for (auto& n : names)
    {
        for (char c : n) { hash ^= (BYTE)c; hash *= 0x01000193; }
        FILE* f = nullptr;
        if (fopen_s(&f, (base + n).c_str(), "rb") != 0 || !f) { ok = false; continue; }
        size_t got;
        while ((got = fread(buf.data(), 1, buf.size(), f)) > 0)
            for (size_t i = 0; i < got; i++) { hash ^= buf[i]; hash *= 0x01000193; }
        fclose(f);
    }
    g_hgTbl = ok ? hash : 0;
    InterlockedExchange(&g_hgTblReady, ok ? 1 : 2);
    char b[96]; sprintf_s(b, "HOPEGUARD: %u tables, hash %08lX%s", (unsigned)names.size(), hash, ok ? "" : " (unreadable)"); Log(b);
}

static void ScanOnce()
{
    DWORD flags = 0; std::string details;
    BOOL remote = FALSE;
    if (IsDebuggerPresent() || (CheckRemoteDebuggerPresent(GetCurrentProcess(), &remote) && remote)) { flags |= HG_DEBUGGER; details += "debugger;"; }
    static const char* kTools[] = { "cheatengine", "artmoney", "speedhack", "speedgear", "wpe pro", "wpepro", "ollydbg", "x32dbg", "x64dbg",
                                    "windbg", "ida.exe", "ida64.exe", "idaq.exe", "idaq64.exe", "koxp", "kobot", "kohack", "rpe.exe" };
    HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snap != INVALID_HANDLE_VALUE)
    {
        PROCESSENTRY32 pe = { sizeof(pe) };
        for (BOOL more = Process32First(snap, &pe); more; more = Process32Next(snap, &pe))
        {
            std::string n = Lower(pe.szExeFile);
            for (const char* t : kTools)
                if (n.find(t) != std::string::npos) { flags |= HG_CHEATTOOL; details += "process:" + n + ";"; break; }
        }
        CloseHandle(snap);
    }
    if (FindWindowA(nullptr, "Cheat Engine") || FindWindowA("TfrmCheatEngine", nullptr)) { flags |= HG_CHEATTOOL; details += "window:cheat engine;"; }
    // DLLs in our process from outside Windows and the game folder (overlays show up too: the server only logs them)
    char win[MAX_PATH]; GetWindowsDirectoryA(win, MAX_PATH);
    char game[MAX_PATH]; GetModuleFileNameA(nullptr, game, MAX_PATH); *(strrchr(game, '\\') + 1) = 0;
    std::string lw = Lower(win), lg = Lower(game), mods;
    snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, GetCurrentProcessId());
    if (snap != INVALID_HANDLE_VALUE)
    {
        MODULEENTRY32 me = { sizeof(me) };
        for (BOOL more = Module32First(snap, &me); more; more = Module32Next(snap, &me))
        {
            std::string p = Lower(me.szExePath);
            if (p.compare(0, lw.size(), lw) == 0 || p.compare(0, lg.size(), lg) == 0) continue;
            if (mods.size() < 300) mods += Lower(me.szModule) + ",";
        }
        CloseHandle(snap);
    }
    if (!mods.empty()) { flags |= HG_MODULE; details += "modules:" + mods + ";"; }
    if (g_hgTblReady == 2) flags |= HG_NOTABLES;
    if (details.size() > 900) details.resize(900);
    EnterCriticalSection(&g_hgLock);
    if (flags != g_hgFlags || details != g_hgDetails)
    {
        char b[200]; sprintf_s(b, "HOPEGUARD: flags %lX %.150s", flags, details.c_str()); Log(b);
    }
    g_hgFlags = flags; g_hgDetails = details;
    LeaveCriticalSection(&g_hgLock);
}

static DWORD WINAPI ScanThread(LPVOID)
{
    HashTables();
    for (;;) { ScanOnce(); Sleep(20000); }
}

static DWORD QpcMs()
{
    static LARGE_INTEGER freq = {};
    if (!freq.QuadPart) QueryPerformanceFrequency(&freq);
    LARGE_INTEGER c; QueryPerformanceCounter(&c);
    return (DWORD)(unsigned long long)(c.QuadPart * 1000 / freq.QuadPart);
}

static CRITICAL_SECTION g_lgLock;
static std::deque<std::vector<BYTE>> g_lgQ;

static void Refuse(const char* why)
{
    char b[160]; sprintf_s(b, "LAUNCH: refused (%s)", why); Log(b);
    MessageBoxW(nullptr, L"Oyunu Launcher.exe ile başlatın.\n\nPlease start the game with Launcher.exe.",
        L"Knight Online", MB_OK | MB_ICONERROR | MB_TOPMOST | MB_SETFOREGROUND);
    TerminateProcess(GetCurrentProcess(), 0);
}

void LaunchGuard_Init()
{
    InitializeCriticalSection(&g_lgLock);
    InitializeCriticalSection(&g_hgLock);
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") != 0) return;   // Option.exe etc. load d3d9.dll too
    CloseHandle(CreateThread(nullptr, 0, ScanThread, nullptr, 0, nullptr));   // HopeGuard ACS checks

    LaunchTicket t = {};
    DWORD type = 0, cb = sizeof(t);
    HKEY k;
    LONG r = RegOpenKeyExW(HKEY_CURRENT_USER, LT_REG_KEY, 0, KEY_QUERY_VALUE | KEY_SET_VALUE, &k);
    if (r != ERROR_SUCCESS) return Refuse("no ticket key");
    r = RegQueryValueExW(k, LT_REG_VALUE, nullptr, &type, (BYTE*)&t, &cb);
    RegDeleteValueW(k, LT_REG_VALUE);   // one shot
    RegCloseKey(k);
    if (r != ERROR_SUCCESS || type != REG_BINARY || cb != sizeof(t)) return Refuse("no ticket");
    if (t.magic != LT_MAGIC || t.mac != LtMac(t)) return Refuse("bad ticket");
    DWORD now = LtNow();
    if (now + 5 < t.time || now - t.time > LT_MAX_AGE) return Refuse("old ticket");
    DWORD parent = LtParentPid(GetCurrentProcessId());
    char b[128]; sprintf_s(b, "LAUNCH: ticket ok (age %lus, launcher pid %lu, parent pid %lu)", now - t.time, t.pid, parent); Log(b);

    // Hard re-connect (reconnect.cpp DoReconnect): the old game started us and is still closing. The game leaves at
    // once while another instance is alive (both restarts in the logs ended in the same second), so wait for it.
    if (HANDLE h = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, t.pid))
    {
        wchar_t img[MAX_PATH] = L""; DWORD n = MAX_PATH;
        QueryFullProcessImageNameW(h, 0, img, &n);
        const wchar_t* base = wcsrchr(img, L'\\'); base = base ? base + 1 : img;
        if (_wcsicmp(base, L"KnightOnLine.exe") == 0)
        {
            DWORD t0 = GetTickCount();
            DWORD w = WaitForSingleObject(h, 8000);
            Sleep(300);   // its windows / named objects are gone a moment after the process object signals
            sprintf_s(b, "LAUNCH: re-connect restart, waited %lu ms for the old game (pid %lu, %s)", GetTickCount() - t0, t.pid,
                w == WAIT_OBJECT_0 ? "closed" : "STILL RUNNING");
            Log(b);
        }
        CloseHandle(h);
    }
}

// ---------------------------------------------------------------- server challenge
// HopeGuard ACS heartbeat (game thread)
static void HeartbeatAnswer(DWORD nonce)
{
    DWORD tick = GetTickCount(), qpc = QpcMs(), tbl = g_hgTbl, flags;
    std::string details;
    EnterCriticalSection(&g_hgLock); flags = g_hgFlags; details = g_hgDetails; LeaveCriticalSection(&g_hgLock);
    DWORD ans = HgAnswer(nonce, tick, qpc, flags, tbl);
    std::vector<BYTE> out = { 0xE9, LG_SUBOP, 4 };
    for (DWORD v : { ans, tick, qpc, flags, tbl }) out.insert(out.end(), (BYTE*)&v, (BYTE*)&v + 4);
    WORD n = (WORD)details.size();
    out.insert(out.end(), (BYTE*)&n, (BYTE*)&n + 2);
    out.insert(out.end(), details.begin(), details.end());
    EnterCriticalSection(&g_lgLock);
    g_lgQ.push_back(out);
    LeaveCriticalSection(&g_lgLock);
    Proxy_RequestFlush();
}

void LaunchGuard_OnRecv(const BYTE* p, size_t len)
{
    if (len >= 7 && p[0] == 0xE9 && p[1] == LG_SUBOP && p[2] == 3) { HeartbeatAnswer(*(const DWORD*)(p + 3)); return; }
    if (len >= 3 && p[0] == 0xE9 && p[1] == LG_SUBOP && p[2] == 6)          // HopeGuard ACS is about to disconnect us
    {
        std::string why;
        if (len >= 5) { WORD n = *(const WORD*)(p + 3); if (5 + (size_t)n <= len) why.assign((const char*)p + 5, n); }
        Reconnect_Block();
        char b[160]; sprintf_s(b, "HOPEGUARD: disconnected by the server (%.120s), re-connect off", why.c_str()); Log(b);
        return;
    }
    if (len < 7 || p[0] != 0xE9 || p[1] != LG_SUBOP || p[2] != 1) return;
    DWORD nonce = *(const DWORD*)(p + 3), ans = LgAnswer(nonce), ver = LG_PROXY_VERSION;
    std::vector<BYTE> out = { 0xE9, LG_SUBOP, 2 };
    out.insert(out.end(), (BYTE*)&ans, (BYTE*)&ans + 4);
    out.insert(out.end(), (BYTE*)&ver, (BYTE*)&ver + 4);
    EnterCriticalSection(&g_lgLock);
    g_lgQ.push_back(out);
    LeaveCriticalSection(&g_lgLock);
    Proxy_RequestFlush();
}

void LaunchGuard_FlushSendQueue()
{
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_lgLock);
    pending.swap(g_lgQ);
    LeaveCriticalSection(&g_lgLock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
        if (p[2] == 2) Log("LAUNCH: challenge answered");
    }
}
