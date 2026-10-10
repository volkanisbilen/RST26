// TEMP diagnostics for the soft re-connect work: logs who calls the winsock connect / close / event-select
// functions (the protected exe reaches ws2_32 through its own thunks, so the native connect chain is found at
// run time). Each call writes "NET: <api> sock=... <ip:port> <- caller chain" to d3d9proxy.log.
#include <winsock2.h>
#include <windows.h>
#include <stdio.h>
#pragma comment(lib, "ws2_32.lib")

void Log(const char* msg);
void NetTrace_Arm();

namespace
{
    typedef int (WSAAPI* tConnect)(SOCKET, const sockaddr*, int);
    typedef int (WSAAPI* tWSAConnect)(SOCKET, const sockaddr*, int, LPWSABUF, LPWSABUF, LPQOS, LPQOS);
    typedef int (WSAAPI* tClose)(SOCKET);
    typedef int (WSAAPI* tEventSelect)(SOCKET, WSAEVENT, long);
    tConnect oConnect; tWSAConnect oWSAConnect; tClose oClose; tEventSelect oEventSelect;

    void Chain(char* out, size_t cb)
    {
        void* frames[10] = {};
        USHORT n = RtlCaptureStackBackTrace(2, 10, frames, nullptr);
        out[0] = 0;
        HMODULE exe = GetModuleHandleA(nullptr);
        for (USHORT i = 0; i < n; i++)
        {
            char b[40];
            HMODULE m = nullptr;
            GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, (LPCSTR)frames[i], &m);
            sprintf_s(b, " %s%08lX", m == exe ? "" : "?", (DWORD)frames[i]);
            strcat_s(out, cb, b);
        }
    }

    void LogCall(const char* api, SOCKET s, const sockaddr* a)
    {
        char addr[64] = "";
        if (a && a->sa_family == AF_INET)
        {
            const sockaddr_in* in = (const sockaddr_in*)a;
            sprintf_s(addr, " %u.%u.%u.%u:%u", in->sin_addr.S_un.S_un_b.s_b1, in->sin_addr.S_un.S_un_b.s_b2,
                      in->sin_addr.S_un.S_un_b.s_b3, in->sin_addr.S_un.S_un_b.s_b4, ntohs(in->sin_port));
        }
        char chain[256]; Chain(chain, sizeof(chain));
        char b[400]; sprintf_s(b, "NET: %s sock=%u%s tid=%lu <-%s", api, (unsigned)s, addr, GetCurrentThreadId(), chain);
        Log(b);
    }

    int WSAAPI hkConnect(SOCKET s, const sockaddr* a, int l) { LogCall("connect", s, a); NetTrace_Arm(); return oConnect(s, a, l); }
    int WSAAPI hkWSAConnect(SOCKET s, const sockaddr* a, int l, LPWSABUF c, LPWSABUF e, LPQOS q, LPQOS g)
    { LogCall("WSAConnect", s, a); NetTrace_Arm(); return oWSAConnect(s, a, l, c, e, q, g); }
    int WSAAPI hkClose(SOCKET s) { LogCall("closesocket", s, nullptr); return oClose(s); }
    int WSAAPI hkEventSelect(SOCKET s, WSAEVENT e, long ev) { char b[32]; sprintf_s(b, "WSAEventSelect(%lX)", ev); LogCall(b, s, nullptr); return oEventSelect(s, e, ev); }

    void* HookApi(HMODULE mod, const char* name, void* hook)
    {
        BYTE* p = (BYTE*)GetProcAddress(mod, name);
        static const BYTE kHot[5] = { 0x8B, 0xFF, 0x55, 0x8B, 0xEC };
        if (!p || memcmp(p, kHot, 5) != 0) { char b[96]; sprintf_s(b, "NET: %s not hooked", name); Log(b); return nullptr; }
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return nullptr;
        memcpy(tramp, p, 5);
        tramp[5] = 0xE9; *(DWORD*)(tramp + 6) = (DWORD)(p + 5) - (DWORD)(tramp + 10);
        DWORD old;
        VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old);
        p[0] = 0xE9; *(DWORD*)(p + 1) = (DWORD)hook - (DWORD)(p + 5);
        VirtualProtect(p, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), p, 5);
        return tramp;
    }
}

void NetTrace_Init()
{
    HMODULE ws = LoadLibraryA("ws2_32.dll");
    if (!ws) return;
    oConnect = (tConnect)HookApi(ws, "connect", (void*)hkConnect);
    oWSAConnect = (tWSAConnect)HookApi(ws, "WSAConnect", (void*)hkWSAConnect);
    oClose = (tClose)HookApi(ws, "closesocket", (void*)hkClose);
    oEventSelect = (tEventSelect)HookApi(ws, "WSAEventSelect", (void*)hkEventSelect);
    Log("NET: winsock trace hooks installed");
}

// packets right after a connect (plain, before the game encrypts): opcode + length + first bytes, for 60 s
static volatile DWORD g_traceUntil = 0;
void NetTrace_Arm() { g_traceUntil = GetTickCount() + 60000; }
void NetTrace_Packet(const char* dir, const BYTE* buf, int len)
{
    if (!buf || len <= 0) return;
    // Extension panels must remain traceable after the initial login minute.
    // Log only routing metadata: account forms and reconnect tokens are private.
    if (buf[0] == 0xE9)
    {
        char b[128];
        sprintf_s(b, "HOOK %s sub=%02X command=%02X len=%d", dir,
            len > 1 ? buf[1] : 0, len > 2 ? buf[2] : 0, len);
        Log(b);
        return;
    }
    if (!g_traceUntil || GetTickCount() > g_traceUntil) return;
    char hex[3 * 24 + 1] = ""; int n = len < 24 ? len : 24;
    if (dir[0] == 's' && (buf[0] == 0xF3 || buf[0] == 0x01 || buf[0] == 0x02)) n = 1;   // login packets carry the password
    for (int i = 0; i < n; i++) sprintf_s(hex + i * 3, 4, "%02X ", buf[i]);
    char b[200]; sprintf_s(b, "NET %s op=%02X len=%d  %s", dir, buf[0], len, hex);
    Log(b);
}

// TEMP test: Ctrl+Shift+F12 drops the game server connection (both directions) so the re-connect can be tried
void NetTrace_TestDrop()
{
    __try
    {
        BYTE* sock = *(BYTE**)0x01115914;
        SOCKET s = sock ? *(SOCKET*)(sock + 0x14) : INVALID_SOCKET;
        if (s == INVALID_SOCKET || s == 0) { Log("NET: test drop - no socket"); return; }
        shutdown(s, SD_BOTH);
        char b[64]; sprintf_s(b, "NET: test drop sock=%u", (unsigned)s); Log(b);
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { Log("NET: test drop exception"); }
}
