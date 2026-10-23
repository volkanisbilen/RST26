// Stall window tooltip: which currency an item is sold for (Coin / Cash / TL).
// The native stall window shows every price as "Purchasing Price : 2,222" whatever the stall is paid in.
// Server (merchant.rs, hook sub 0xF8, right after the stall list, empty when the window closes):
//   E9 F8 [u8 n] n * ([u32 price][u8 currency 0 coin / 1 Knight Cash / 2 TL])
// The tooltip line is written through CN3UIString::SetString (event_title.cpp's hook calls MerchTip_Label): the number is
// matched with the prices of the stall that is open; when every slot with that price uses the same currency the name is
// appended ("Purchasing Price : 2,222 Cash"). A price shared by slots of different currencies is left alone.
#include <windows.h>
#include <stdio.h>
#include <string>
#include <map>

void Log(const char* msg);

namespace
{
    CRITICAL_SECTION g_lock;
    bool g_ready = false;
    std::map<UINT32, int> g_mask;        // price -> bit mask of currencies seen (1 coin, 2 Cash, 4 TL)
    DWORD g_setAt = 0;
}

void MerchTip_Init()
{
    InitializeCriticalSection(&g_lock);
    g_ready = true;
}

void MerchTip_OnRecv(const BYTE* buf, size_t len)
{
    if (!g_ready || len < 3 || buf[0] != 0xE9 || buf[1] != 0xF8) return;
    size_t n = buf[2];
    if (len < 3 + n * 5) return;
    EnterCriticalSection(&g_lock);
    g_mask.clear();
    for (size_t i = 0; i < n; i++)
    {
        UINT32 price = *(const UINT32*)(buf + 3 + i * 5);
        BYTE cur = buf[3 + i * 5 + 4];
        g_mask[price] |= 1 << (cur < 3 ? cur : 0);
    }
    g_setAt = GetTickCount();
    LeaveCriticalSection(&g_lock);
    char m[64]; sprintf_s(m, "MERCHTIP: %u price(s) of the stall", (unsigned)n);
    Log(m);
}

// "Purchasing Price : 1,111" -> "Purchasing Price : 1,111 TL"; false = leave the text alone
bool MerchTip_Label(const std::string& s, std::string& out)
{
    if (!g_ready || s.size() < 18 || s.size() > 100 || _strnicmp(s.c_str(), "Purchasing Price", 16) != 0) return false;
    size_t colon = s.find(':');
    if (colon == std::string::npos) return false;
    UINT64 v = 0; int digits = 0;
    for (size_t i = colon + 1; i < s.size(); i++)
    {
        char c = s[i];
        if (c >= '0' && c <= '9') { v = v * 10 + (c - '0'); digits++; if (v > 0xFFFFFFFFull) return false; }
        else if (c == ',' || c == '.' || c == ' ') continue;
        else return false;                                     // anything else after the number: not the plain price line
    }
    if (!digits) return false;
    int mask = 0;
    EnterCriticalSection(&g_lock);
    if (GetTickCount() - g_setAt < 30 * 60 * 1000)
    {
        auto it = g_mask.find((UINT32)v);
        if (it != g_mask.end()) mask = it->second;
    }
    LeaveCriticalSection(&g_lock);
    const char* name = mask == 1 ? " Coin" : mask == 2 ? " Cash" : mask == 4 ? " TL" : nullptr;
    if (!name) return false;
    out = s + name;
    return true;
}
