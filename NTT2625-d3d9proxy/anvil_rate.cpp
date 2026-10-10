// Anvil instant rate: the game's Item Upgrade window (CUIItemUpgrade at [CGameProcMain+0x27C], our
// re_itemupgrade.uif from build_upgrade_uif.py) shows "Upgrade Rate : %45.00   Coins : 40.000".
// Server: ItemUpgradeSystem.cpp SendUpgradeRate on Preview / Confirm, WIZ_HSACS_HOOK 0xE9 + UPGRADE_RATE 0xC4:
//   [u32 rate 0..10000][u32 coins]   (0 / 0 = no matching recipe -> "-")
// The strings are written on the game thread (Anvil_Tick from the subclassed game window procedure).
#include <windows.h>
#include <stdio.h>
#include <string>

void Log(const char* msg);

namespace
{
    const BYTE WIZ_HSACS_HOOK = 0xE9, UPGRADE_RATE = 0xC4;
    volatile LONG g_rate = -1, g_coins = 0, g_dirty = 0;

    void* Ptr(void* o, DWORD off) { return *(void**)((BYTE*)o + off); }
    const char* MsvcStr(const BYTE* s) { return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s; }
    const char* IdOf(void* o) { return MsvcStr((const BYTE*)o + 0x58); }

    void* FindDeep(void* parent, const char* id, int depth = 0)
    {
        if (!parent || depth > 4) return nullptr;
        BYTE* head = (BYTE*)Ptr(parent, 0xb0);
        if (!head) return nullptr;
        for (BYTE* n = *(BYTE**)head; n && n != head; n = *(BYTE**)n)
        {
            void* c = *(void**)(n + 8);
            if (!c) continue;
            if (strcmp(IdOf(c), id) == 0) return c;
            if (void* r = FindDeep(c, id, depth + 1)) return r;
        }
        return nullptr;
    }

    void SetText(void* str, const std::string& s)
    {
        if (!str || strcmp(MsvcStr((const BYTE*)str + 0x11c), s.c_str()) == 0) return;
        void** vt = *(void***)str;
        ((void(__thiscall*)(void*, const std::string*))vt[0xd8 / 4])(str, &s);
    }

    std::string Thousands(UINT32 v)
    {
        char raw[16]; sprintf_s(raw, "%u", v);
        std::string s = raw, out;
        for (size_t i = 0; i < s.size(); i++) { if (i && (s.size() - i) % 3 == 0) out += '.'; out += s[i]; }
        return out;
    }
}

void Anvil_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 10 || buf[0] != WIZ_HSACS_HOOK || buf[1] != UPGRADE_RATE) return;   // E9 C4 + u32 rate + u32 coins = 10 bytes
    InterlockedExchange(&g_rate, (LONG)*(const UINT32*)(buf + 2));
    InterlockedExchange(&g_coins, (LONG)*(const UINT32*)(buf + 6));
    InterlockedExchange(&g_dirty, 1);
    char b[64]; sprintf_s(b, "ANVIL: rate %ld coins %ld", g_rate, g_coins); Log(b);
}

static void TickImpl()
{
    {
        BYTE* gm = *(BYTE**)0x011158FC;
        if (!gm) return;
        void* win = Ptr(gm, 0x27C);
        if (!win) return;
        static bool wasOpen = false;
        bool open = *((BYTE*)win + 0xea) != 0;
        if (open && !wasOpen) { InterlockedExchange(&g_rate, -1); InterlockedExchange(&g_dirty, 1); }   // fresh window: "-"
        wasOpen = open;
        if (!open || !InterlockedExchange(&g_dirty, 0)) return;
        LONG rate = g_rate, coins = g_coins;
        char b[32];
        if (rate <= 0) strcpy_s(b, "-");
        else sprintf_s(b, "%%%u.%02u", (unsigned)rate / 100, (unsigned)rate % 100);
        SetText(FindDeep(win, "hg_upg_rate"), b);
        SetText(FindDeep(win, "hg_upg_coins"), rate <= 0 ? std::string("-") : Thousands((UINT32)coins));
    }
}

// game thread
void Anvil_Tick()
{
    __try { TickImpl(); }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
}
