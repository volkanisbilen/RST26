// d3d9 proxy: forwards to the system d3d9.dll and loads HopeGuard.dll from the game folder.
// Goal: get our DLL into the 2625 client without touching the XIGNCODE-packed exe.
#include <windows.h>
#include <stdio.h>
#include <share.h>

static HMODULE g_real = nullptr;
static FARPROC g_fn[6] = {};
static const char* g_names[6] = {
    "Direct3DCreate9", "Direct3DCreate9Ex", "D3DPERF_BeginEvent",
    "D3DPERF_EndEvent", "D3DPERF_SetMarker", "D3DPERF_GetStatus"
};

void Pus_Init();
void Pus_OnRecv(const BYTE* buf, size_t len);
void Pus_FlushSendQueue();
void Rce_Init();
void Rce_OnRecv(const BYTE* buf, size_t len);
void Rce_FlushSendQueue();
void Cr_Init();
void Cr_OnRecv(const BYTE* buf, size_t len);
void Cape_Init();
void Sp_Init();
void Sp_OnRecv(const BYTE* buf, size_t len);
void Sp_FlushSendQueue();
void Cind_Init();
void Cind_OnRecv(const BYTE* buf, size_t len);
void Cind_FlushSendQueue();
void Csw_Init();
void Csw_OnRecv(const BYTE* buf, size_t len);
void Csw_FlushSendQueue();
void MerchCur_Init();
void Lottery_Init();
void Lottery_OnRecv(const BYTE* buf, size_t len);
void Lottery_FlushSendQueue();
void MannerStore_Init();
void MannerStore_OnRecv(const BYTE* buf, size_t len);
void MannerStore_FlushSendQueue();
void MannerStore_Toggle();
void DropBox_Init();
void DropBox_OnRecv(const BYTE* buf, size_t len);
void DropBox_FlushSendQueue();
void EvReward_Init();
void EventTitle_Init();   // Juraid title on the borrowed BDW join window (event_title.cpp)
void Anvil_OnRecv(const BYTE* buf, size_t len);
void Anvil_Tick();
void EvReward_OnRecv(const BYTE* buf, size_t len);
void CrashTrap_Init();
void IatFix_Init();
void ClientPatches_Init();
void Genie_Init();
void Genie_Tick(HWND h);
void Genie_OnMouseDown(int x, int y);
void Genie_OnRecv(const BYTE* pkt, int len);
void LoginRemember_Init();
void Reconnect_Init();
void HoldRepeat_Init();
void ResetBar_Init();              // KC / TL + Stat Reset (U), Skill Reset (K): native uif controls (reset_bar.cpp)
void ResetBar_Tick();
void HotkeyGuard_Init();
void NetTrace_Init();
void NetTrace_Packet(const char* dir, const BYTE* buf, int len);
void NetTrace_TestDrop();
void HoldRepeat_OnMouseDown(HWND h, int x, int y);
bool HoldRepeat_OnTimer(HWND h, UINT_PTR id);
void Reconnect_OnRecv(const BYTE* p, size_t len);
bool Reconnect_OnTimer(HWND h, UINT_PTR id);
void Drop_Init();
void Drop_OnRecv(const BYTE* buf, size_t len);
void Drop_FlushSendQueue();
void Drop_Toggle(bool loadTarget);
bool Drop_ForwardWheel(WPARAM wp);
void Cross_Init();
void LaunchGuard_Init();
void LaunchGuard_OnRecv(const BYTE* p, size_t len);
void LaunchGuard_FlushSendQueue();
void Tag_Init();
void HopeGuardBanner_Init();
void HopeGuardSplash_Wait();
void SoloCapeIcon_Init();
void DropBtn_Init();
void CamRange_Init();
void InfStone_Init();
void ItemTip_Init();
void ItemTip_Tick();
void NameFlag_Tick();   // client_patches.cpp
void ItemTip_OnRecv(const BYTE* p, size_t len);
void PriceFix_Init();
void PriceFix_OnRecv(const BYTE* p, size_t len);
void ItemTip_FlushSendQueue();
void DropBtn_Tick();
bool DropMini_ForwardWheel(WPARAM wp);
void SoloCapeIcon_OnRecv(const BYTE* p, size_t len);
void SoloCapeIcon_FlushSendQueue();
void Tag_OnRecv(const BYTE* p, size_t len);
void Tag_FlushSendQueue();
void JobChange_Init();             // Job Change scroll panel (jobchange_panel.cpp)
void JobChange_OnRecv(const BYTE* p, size_t len);
void JobChange_FlushSendQueue();
void Register_Init();
void Register_OnRecv(const BYTE* p, size_t len);
void Register_FlushSendQueue();   // Account Register on the login screen (register_panel.cpp)
void QuoteKeyBlock_Init();   // " key hidden from the game: no native panel (quote_key_block.cpp)
void QuoteKeyBlock_LayoutChanged();
void GmPanel_Init();   // GM panel, " key, GMs only (gm_panel.cpp)
void Discord_InitProxy();   // Discord Rich Presence (discord_presence.cpp)
void PackOverride_Init();   // HopeGuard\pack_override files replace pack entries while the game reads them (pack_override.cpp)
void Discord_OnRecv(const BYTE* p, size_t len);
void GmPanel_FlushSendQueue();
void GmPanel_OnRecv(const BYTE* p, size_t len);
bool GmPanel_OnGameChar(WPARAM ch);
bool GmPanel_CursorOverPanel();
bool GmPanel_ForwardWheel(WPARAM w, LPARAM l);
void Tag_Tick();
bool Tag_OnTimer(UINT_PTR id);

void Log(const char* msg)
{
    char path[MAX_PATH];
    GetModuleFileNameA(nullptr, path, MAX_PATH);
    char* slash = strrchr(path, '\\');
    if (slash) strcpy_s(slash + 1, MAX_PATH - (slash + 1 - path), "d3d9proxy.log");
    FILE* f = nullptr;
    for (int i = 0; i < 20 && !(f = _fsopen(path, "a", _SH_DENYNO)); i++) Sleep(5);   // shared: several threads log at once
    if (f)
    {
        SYSTEMTIME t; GetLocalTime(&t);
        fprintf(f, "[%02d:%02d:%02d] pid=%lu %s\n", t.wHour, t.wMinute, t.wSecond, GetCurrentProcessId(), msg);
        fclose(f);
    }
}

static void LoadReal()
{
    if (g_real) return;
    char sys[MAX_PATH];
    GetSystemDirectoryA(sys, MAX_PATH);
    strcat_s(sys, "\\d3d9.dll");
    g_real = LoadLibraryA(sys);
    for (int i = 0; i < 6; i++)
        g_fn[i] = g_real ? GetProcAddress(g_real, g_names[i]) : nullptr;
    Log(g_real ? "system d3d9 loaded" : "FAILED to load system d3d9");
}

static DWORD WINAPI LoadGuard(LPVOID)
{
    HMODULE h = LoadLibraryA("HopeGuard.dll");
    const char* loadedName = "HopeGuard.dll";
    if (!h)
    {
        // Existing client installs may still ship the asset/pack reader under its former name.
        // The proxy panels rely on its .pus/.txt and packed-UI decryption hooks.
        h = LoadLibraryA("OPSGUARD.dll");
        loadedName = "OPSGUARD.dll";
    }
    char buf[128];
    sprintf_s(buf, "%s load -> %p (err=%lu)", loadedName, h, h ? 0 : GetLastError());
    Log(buf);
    return 0;
}

// HopeGuardUI.dll (was OPSGUARD.dll: the old HSACSX client module ported to this exe, OPS_COEXIST build): optional, loaded only when the
// file is in the game folder. It installs no send / recv / tick hooks of its own and leaves every panel this proxy
// already has switched off, so the two do not meet. From a thread: nothing here waits for it.
static DWORD WINAPI LoadOpsGuard(LPVOID)
{
    HMODULE h = LoadLibraryA("HopeGuardUI.dll");
    char buf[128];
    sprintf_s(buf, "HopeGuardUI.dll load -> %p (err=%lu)", h, h ? 0 : GetLastError());
    Log(buf);
    return 0;
}

// Launcher hand-over: a running Launcher.exe cannot overwrite itself, so a patch brings the new launcher as
// <game>\Launcher.upd. The launcher quits right after it starts the game; here, in the game, the file takes its place.
// (Our own launcher replaces itself, HopeGuardLauncher\hg_launcher.cpp; this is for clients that still run the stock one.)
static DWORD WINAPI LauncherSwap(LPVOID)
{
    wchar_t dir[MAX_PATH]; GetModuleFileNameW(nullptr, dir, MAX_PATH);
    wchar_t* s = wcsrchr(dir, L'\\'); if (!s) return 0; s[1] = 0;
    wchar_t upd[MAX_PATH], exe[MAX_PATH];
    swprintf_s(upd, L"%sLauncher.upd", dir); swprintf_s(exe, L"%sLauncher.exe", dir);
    if (GetFileAttributesW(upd) == INVALID_FILE_ATTRIBUTES) return 0;
    for (int i = 0; i < 60; i++)   // the old launcher may still be closing
    {
        if (MoveFileExW(upd, exe, MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) { Log("LAUNCHER: Launcher.upd is now Launcher.exe"); return 0; }
        Sleep(1000);
    }
    char buf[96]; sprintf_s(buf, "LAUNCHER: Launcher.upd could not replace Launcher.exe (err=%lu)", GetLastError());
    Log(buf);
    return 0;
}

extern "C" {
__declspec(dllexport) void* WINAPI Direct3DCreate9(UINT v)
{ HopeGuardSplash_Wait(); LoadReal(); return g_fn[0] ? ((void*(WINAPI*)(UINT))g_fn[0])(v) : nullptr; }   // HopeGuard ACS loading bar first
__declspec(dllexport) HRESULT WINAPI Direct3DCreate9Ex(UINT v, void** p)
{ HopeGuardSplash_Wait(); LoadReal(); return g_fn[1] ? ((HRESULT(WINAPI*)(UINT, void**))g_fn[1])(v, p) : E_FAIL; }
__declspec(dllexport) int WINAPI D3DPERF_BeginEvent(DWORD c, LPCWSTR n)
{ LoadReal(); return g_fn[2] ? ((int(WINAPI*)(DWORD, LPCWSTR))g_fn[2])(c, n) : 0; }
__declspec(dllexport) int WINAPI D3DPERF_EndEvent()
{ LoadReal(); return g_fn[3] ? ((int(WINAPI*)())g_fn[3])() : 0; }
__declspec(dllexport) void WINAPI D3DPERF_SetMarker(DWORD c, LPCWSTR n)
{ LoadReal(); if (g_fn[4]) ((void(WINAPI*)(DWORD, LPCWSTR))g_fn[4])(c, n); }
__declspec(dllexport) DWORD WINAPI D3DPERF_GetStatus()
{ LoadReal(); return g_fn[5] ? ((DWORD(WINAPI*)())g_fn[5])() : 0; }
}

// --- Recv hook (test 3): pass-through observer on CGameProcMain::ProcessPacket -------------
// Verified from the in-game memory dump: thiscall(this, RECV_DATA* a3, int* a4),
// prologue 55 8B EC 6A FF (5 bytes on an instruction boundary).
static const DWORD KO_RECV_FNC = 0x0084D0B0;   // unpack build: CGameProcMain vtable[+8] ProcessPacket
static const BYTE  kRecvPrologue[5] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF };
typedef char(__thiscall* tRecv)(void* self, DWORD* data, int* offset);
static tRecv g_oRecv = nullptr;
static volatile LONG g_opCount[256];
static volatile LONG g_recvTotal = 0;

// --- Immediate send flush -------------------------------------------------------------------
// Our queued packets (PUS buy, RCE exchange, ...) used to go out only on the next received
// packet -> noticeable delay when the server is quiet. The game window is subclassed; queueing
// a packet posts WM_PROXY_FLUSH to it, and the queues are flushed right away on the game thread.
static volatile DWORD g_gameTid = 0;           // thread that runs ProcessPacket (= game main loop)
static HWND    g_gameWnd = nullptr;
static WNDPROC g_oldWndProc = nullptr;
HWND Proxy_GameWnd() { return g_gameWnd; }
static const UINT WM_PROXY_FLUSH = WM_APP + 0x2F1;

static LRESULT CALLBACK ProxyWndProc(HWND h, UINT m, WPARAM w, LPARAM l)
{
    if (GetCurrentThreadId() == g_gameTid) { Genie_Tick(h); Anvil_Tick(); Tag_Tick(); DropBtn_Tick(); ItemTip_Tick(); ResetBar_Tick(); NameFlag_Tick(); }
    if ((m == WM_LBUTTONDOWN || m == WM_RBUTTONDOWN) && GetCurrentThreadId() == g_gameTid) Genie_OnMouseDown((short)LOWORD(l), (short)HIWORD(l));
    if (m == WM_LBUTTONDOWN && GetCurrentThreadId() == g_gameTid) HoldRepeat_OnMouseDown(h, (short)LOWORD(l), (short)HIWORD(l));   // stat / skill "+" hold to repeat
    if (m == WM_TIMER && GetCurrentThreadId() == g_gameTid && HoldRepeat_OnTimer(h, (UINT_PTR)w)) return 0;   // HopeGuard Genie field hint / settings push
    if (m == WM_TIMER && GetCurrentThreadId() == g_gameTid && Reconnect_OnTimer(h, (UINT_PTR)w)) return 0;   // soft re-connect steps
    if (m == WM_TIMER && GetCurrentThreadId() == g_gameTid && Tag_OnTimer((UINT_PTR)w)) return 0;   // tag tick timer (Tag_Tick ran above)
    // drop viewer: '\' (US layout) or Ctrl+D; first open also loads the current target's drops
    if (m == WM_KEYDOWN && !(l & 0x40000000) &&
        (w == VK_OEM_5 || (w == 'D' && (GetKeyState(VK_CONTROL) & 0x8000))))
        Drop_Toggle(true);
    if (m == WM_MOUSEWHEEL && Drop_ForwardWheel(w)) return 0;
    if (m == WM_MOUSEWHEEL && GmPanel_ForwardWheel(w, l)) return 0;   // GM panel user list
    if (m == WM_CHAR && GetCurrentThreadId() == g_gameTid && GmPanel_OnGameChar(w)) return 0;
    if (m == WM_INPUTLANGCHANGE) QuoteKeyBlock_LayoutChanged();   // " opens the GM panel (GMs only)
    if (m == WM_MOUSEWHEEL && DropMini_ForwardWheel(w)) return 0;   // compact drop list (drop_button.cpp)
    if (m == WM_KEYDOWN && w == VK_F12 && (GetKeyState(VK_CONTROL) & 0x8000) && (GetKeyState(VK_SHIFT) & 0x8000)) NetTrace_TestDrop();   // TEMP re-connect test
    if (m == WM_PROXY_FLUSH)
    {
        if (GetCurrentThreadId() == g_gameTid)
        {
            __try { Pus_FlushSendQueue(); Rce_FlushSendQueue(); Sp_FlushSendQueue(); Cind_FlushSendQueue(); Csw_FlushSendQueue(); Lottery_FlushSendQueue(); MannerStore_FlushSendQueue(); DropBox_FlushSendQueue(); Drop_FlushSendQueue(); LaunchGuard_FlushSendQueue(); Tag_FlushSendQueue(); JobChange_FlushSendQueue(); SoloCapeIcon_FlushSendQueue(); ItemTip_FlushSendQueue(); GmPanel_FlushSendQueue(); Register_FlushSendQueue(); }
            __except (EXCEPTION_EXECUTE_HANDLER) { Log("FLUSH: exception in send flush"); }
        }
        return 0;
    }
    return IsWindowUnicode(h) ? CallWindowProcW(g_oldWndProc, h, m, w, l) : CallWindowProcA(g_oldWndProc, h, m, w, l);
}

void Proxy_RequestFlush()
{
    if (HWND h = g_gameWnd) PostMessageW(h, WM_PROXY_FLUSH, 0, 0);
}

static DWORD WINAPI InstallFlushHook(LPVOID)
{
    // wait for the first packet (gives us the game thread), then its main window
    for (int i = 0; i < 1200 && !g_gameWnd; i++)
    {
        Sleep(250);
        if (!g_gameTid) continue;
        HWND best = nullptr; int bestArea = 0;
        struct Ctx { HWND* best; int* area; } ctx = { &best, &bestArea };
        EnumThreadWindows(g_gameTid, [](HWND h, LPARAM lp) -> BOOL {
            auto c = (Ctx*)lp;
            RECT r;
            if (IsWindowVisible(h) && GetWindowRect(h, &r))
            {
                int a = (r.right - r.left) * (r.bottom - r.top);
                if (a > *c->area) { *c->area = a; *c->best = h; }
            }
            return TRUE;
        }, (LPARAM)&ctx);
        if (!best) continue;
        g_oldWndProc = IsWindowUnicode(best)
            ? (WNDPROC)SetWindowLongPtrW(best, GWLP_WNDPROC, (LONG_PTR)ProxyWndProc)
            : (WNDPROC)SetWindowLongPtrA(best, GWLP_WNDPROC, (LONG_PTR)ProxyWndProc);
        if (!g_oldWndProc) { Log("FLUSH: subclass failed"); return 0; }
        g_gameWnd = best;
        char buf[96]; sprintf_s(buf, "FLUSH: game window %p (tid %lu) subclassed, sends go out immediately", best, g_gameTid);
        Log(buf);
    }
    return 0;
}

static char __fastcall hkRecv(void* self, void* /*edx*/, DWORD* data, int* offset)
{
    if (!g_gameTid) g_gameTid = GetCurrentThreadId();
    __try
    {
        // data[1] = length, data[2] = buffer; opcode = buffer[*offset]
        if (data && offset && (DWORD)*offset < data[1])
        {
            const BYTE* pkt = (const BYTE*)data[2] + *offset;
            InterlockedIncrement(&g_opCount[pkt[0]]);
            NetTrace_Packet("recv", pkt, (int)(data[1] - *offset));
            if (pkt[0] == 0x3D) Rce_OnRecv(pkt, data[1] - *offset);
            if (pkt[0] == 0x5B && data[1] - *offset >= 6 && pkt[1] == 4)
            {
                Rce_OnRecv(pkt, data[1] - *offset);
                *offset = (int)data[1];
                return 1;
            }
            if (pkt[0] == 0x0D) Reconnect_OnRecv(pkt, data[1] - *offset);   // soft re-connect: GameStart answer
            if (pkt[0] == 0x07 || pkt[0] == 0xE9) Tag_OnRecv(pkt, data[1] - *offset);   // tag list / player out of view
            if (pkt[0] == 0x97 || pkt[0] == 0xE9) Genie_OnRecv(pkt, (int)(data[1] - *offset));   // genie options load / leader target
            if (pkt[0] == 0xE9)
            {
                Pus_OnRecv(pkt, data[1] - *offset);
                Rce_OnRecv(pkt, data[1] - *offset);
                Cr_OnRecv(pkt, data[1] - *offset);
                Sp_OnRecv(pkt, data[1] - *offset);
                Cind_OnRecv(pkt, data[1] - *offset);
                Csw_OnRecv(pkt, data[1] - *offset);
                Lottery_OnRecv(pkt, data[1] - *offset);
                MannerStore_OnRecv(pkt, data[1] - *offset);
                DropBox_OnRecv(pkt, data[1] - *offset);
                EvReward_OnRecv(pkt, data[1] - *offset);
                Anvil_OnRecv(pkt, data[1] - *offset);
                Drop_OnRecv(pkt, data[1] - *offset);
                Reconnect_OnRecv(pkt, data[1] - *offset);
                LaunchGuard_OnRecv(pkt, data[1] - *offset);
                SoloCapeIcon_OnRecv(pkt, data[1] - *offset);
                ItemTip_OnRecv(pkt, data[1] - *offset);
                PriceFix_OnRecv(pkt, data[1] - *offset);
                GmPanel_OnRecv(pkt, data[1] - *offset);
                JobChange_OnRecv(pkt, data[1] - *offset);
                Discord_OnRecv(pkt, data[1] - *offset);   // Discord Rich Presence (discord_presence.cpp)
                Register_OnRecv(pkt, data[1] - *offset);   // account details form (register_panel.cpp)
            }
        }
        InterlockedIncrement(&g_recvTotal);
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { Log("HOOK recv: panel parser exception"); }
    __try { Pus_FlushSendQueue(); Rce_FlushSendQueue(); Sp_FlushSendQueue(); Cind_FlushSendQueue(); Csw_FlushSendQueue(); Lottery_FlushSendQueue(); MannerStore_FlushSendQueue(); DropBox_FlushSendQueue(); Drop_FlushSendQueue(); LaunchGuard_FlushSendQueue(); Tag_FlushSendQueue(); JobChange_FlushSendQueue(); SoloCapeIcon_FlushSendQueue(); ItemTip_FlushSendQueue(); GmPanel_FlushSendQueue(); Register_FlushSendQueue(); } // game thread: safe point for our own sends
    __except (EXCEPTION_EXECUTE_HANDLER) { Log("PUS: exception in send flush"); }
    return g_oRecv(self, data, offset);
}

static bool InstallRecvHook()
{
    BYTE* target = (BYTE*)KO_RECV_FNC;
    if (memcmp(target, kRecvPrologue, 5) != 0)
    {
        Log("recv hook: prologue mismatch, NOT hooking");
        return false;
    }
    // trampoline: original 5 bytes + jmp back to target+5
    BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    if (!tramp) return false;
    memcpy(tramp, target, 5);
    tramp[5] = 0xE9;
    *(DWORD*)(tramp + 6) = (DWORD)(target + 5) - (DWORD)(tramp + 10);
    g_oRecv = (tRecv)tramp;

    DWORD old;
    if (!VirtualProtect(target, 5, PAGE_EXECUTE_READWRITE, &old)) return false;
    target[0] = 0xE9;
    *(DWORD*)(target + 1) = (DWORD)hkRecv - (DWORD)(target + 5);
    VirtualProtect(target, 5, old, &old);
    FlushInstructionCache(GetCurrentProcess(), target, 5);
    return true;
}

static DWORD WINAPI RecvHookTest(LPVOID)
{
    // wait until the exe has unpacked itself before touching code
    for (int i = 0; i < 120 && memcmp((void*)KO_RECV_FNC, kRecvPrologue, 5) != 0; i++)
        Sleep(500);
    Log(InstallRecvHook() ? "recv hook INSTALLED @0084D0B0" : "recv hook FAILED");
    for (int round = 1; round <= 4; round++)
    {
        Sleep(60000);
        char buf[1024];
        int n = sprintf_s(buf, "recv stats t+%ds total=%ld:", round * 15, g_recvTotal);
        for (int op = 0; op < 256 && n < 900; op++)
            if (g_opCount[op]) n += sprintf_s(buf + n, sizeof(buf) - n, " %02X=%ld", op, g_opCount[op]);
        Log(buf);
    }
    return 0;
}

// Read-only probe: dump bytes at candidate hook targets once the exe is unpacked in memory.
static DWORD WINAPI ProbeAddresses(LPVOID)
{
    Sleep(30000);
    char buf[512];
    sprintf_s(buf, "exe base=%p", GetModuleHandleA(nullptr));
    Log(buf);
    struct { const char* name; DWORD addr; } targets[] = {
        { "KO_SND_FNC",  0x00704070 },
        { "KO_RECV_FNC", 0x0084D0B0 },
        { "KO_PTR_PKT",  0x01115914 },
        { "KO_PTR_CHR",  0x01115834 },
        { "KO_PTR_DLG",  0x0111591C },
    };
    // Wait until in-game, then dump the unpacked image for offline analysis.
    Sleep(60000);
    {
        BYTE* base = (BYTE*)GetModuleHandleA(nullptr);
        auto nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
        DWORD size = nt->OptionalHeader.SizeOfImage;
        char path[MAX_PATH];
        GetModuleFileNameA(nullptr, path, MAX_PATH);
        strcpy_s(strrchr(path, '\\') + 1, 64, "ko_memdump.bin");
        FILE* f = nullptr;
        DWORD written = 0, skipped = 0;
        if (fopen_s(&f, path, "wb") == 0 && f)
        {
            static BYTE zero[0x1000];
            for (DWORD off = 0; off < size; off += 0x1000)
            {
                MEMORY_BASIC_INFORMATION mbi{};
                bool ok = VirtualQuery(base + off, &mbi, sizeof(mbi)) && mbi.State == MEM_COMMIT &&
                          !(mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD));
                fwrite(ok ? base + off : zero, 1, 0x1000, f);
                ok ? written++ : skipped++;
            }
            fclose(f);
        }
        sprintf_s(buf, "memdump size=%lX pages ok=%lu skipped=%lu", size, written, skipped);
        Log(buf);
    }
    for (auto& t : targets)
    {
        MEMORY_BASIC_INFORMATION mbi{};
        if (!VirtualQuery((void*)t.addr, &mbi, sizeof(mbi)) || mbi.State != MEM_COMMIT ||
            (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)))
        {
            sprintf_s(buf, "%s @%08lX unreadable", t.name, t.addr);
            Log(buf);
            continue;
        }
        const BYTE* p = (const BYTE*)t.addr;
        int n = sprintf_s(buf, "%s @%08lX prot=%lX:", t.name, t.addr, mbi.Protect);
        for (int i = 0; i < 24; i++)
            n += sprintf_s(buf + n, sizeof(buf) - n, " %02X", p[i]);
        Log(buf);
    }
    return 0;
}

// Option.exe (and any other small tool in the client folder) loads this d3d9.dll too. All the hooks below use
// KnightOnLine.exe addresses, so there they crashed at once (the thread at RecvHookTest read 0x7AFD90 -> AV,
// "Option.exe acilmiyor" 2026-10-02). Only the game image is big: KnightOnLine.exe 0x1316000, Option.exe 0x220000.
// In any other host install nothing; Direct3DCreate9 still forwards to the system d3d9.
static bool IsGameHost()
{
    const BYTE* base = (const BYTE*)GetModuleHandleA(nullptr);
    const IMAGE_NT_HEADERS* nt = (const IMAGE_NT_HEADERS*)(base + ((const IMAGE_DOS_HEADER*)base)->e_lfanew);
    return nt->OptionalHeader.SizeOfImage >= 0x01000000;
}

BOOL APIENTRY DllMain(HMODULE, DWORD reason, LPVOID)
{
    if (reason == DLL_PROCESS_ATTACH && IsGameHost())
    {
        Log("d3d9 proxy attached");
        // HopeGuard.dll (was OPSGUARD.dll) first and at once: it redirects this DLL's file imports, so the encrypted HopeGuard\ skins read
        // plain by the time the panels load them (loaded from a thread it came after the first skin loads).
        if (GetFileAttributesA("HopeGuard.dll") != INVALID_FILE_ATTRIBUTES ||
            GetFileAttributesA("OPSGUARD.dll") != INVALID_FILE_ATTRIBUTES)
            LoadGuard(nullptr);
        else
            Log("HopeGuard.dll / OPSGUARD.dll not present, proxy-only test");
        LaunchGuard_Init();   // not started by Launcher.exe -> message + exit
        HopeGuardBanner_Init();   // HopeGuard ACS splash + 10 s loading bar (the game waits for it in Direct3DCreate9)
        CrashTrap_Init();
        PackOverride_Init();    // before the game reads its packs
        IatFix_Init();
        ClientPatches_Init();
        Genie_Init();
        Pus_Init();
        Rce_Init();
        Cr_Init();
        Cape_Init();
        Sp_Init();
        Cind_Init();
        Csw_Init();             // Castle Siege War clan score board (csw_panel.cpp)
        MerchCur_Init();        // Coins / Knight Cash / TL choice under the merchant price dialog (merchant_currency.cpp)
        Lottery_Init();
        MannerStore_Init();
        DropBox_Init();
        EvReward_Init();
        EventTitle_Init();
        LoginRemember_Init();
        Reconnect_Init();
        HoldRepeat_Init();
        ResetBar_Init();
        HotkeyGuard_Init();
        NetTrace_Init();
        Drop_Init();
        Cross_Init();
        Tag_Init();
        JobChange_Init();
        GmPanel_Init();
        Discord_InitProxy();
        QuoteKeyBlock_Init();
        Register_Init();
        SoloCapeIcon_Init();
        ItemTip_Init();         // inventory style item tooltip for our panels (item_tooltip.cpp)
        DropBtn_Init();         // "Drop" button next to the target bar (drop_button.cpp)
        CamRange_Init();        // Camera Range slider under the F10 options window (camera_range.cpp)
        InfStone_Init();        // Infinite Stone alone passes the client's class stone check (infinite_stone.cpp)
        if (GetFileAttributesA("HopeGuardUI.dll") != INVALID_FILE_ATTRIBUTES)
            CloseHandle(CreateThread(nullptr, 0, LoadOpsGuard, nullptr, 0, nullptr));   // optional, see LoadOpsGuard
        CloseHandle(CreateThread(nullptr, 0, LauncherSwap, nullptr, 0, nullptr));       // Launcher.upd -> Launcher.exe
        // PriceFix_Init();     // NPC shop prices as the server charges them (price_fix.cpp): switched off 2026-10-04, the shop shows the client's own prices again
        CloseHandle(CreateThread(nullptr, 0, RecvHookTest, nullptr, 0, nullptr));
        CloseHandle(CreateThread(nullptr, 0, InstallFlushHook, nullptr, 0, nullptr));
    }
    return TRUE;
}
