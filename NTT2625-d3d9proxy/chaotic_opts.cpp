// Chaotic Generator window options (the supplied re_piecechange.uif has a count with - / +, "Auto Bank" and
// "Sell if Not Unique"). The game's own window ignores those controls, so this file runs them:
//  - game thread tick: finds the window (the object that owns btn_plus / btn_start), keeps the count text and the
//    two labels ("[x] Auto Bank") up to date,
//  - mouse up on the game window: - / + change the count (1..100), the checkbox or its label toggles the option,
//  - every change (and every time the window opens) sends hook 0xE9 / 0xE8 [count][bank][sell]; the server
//    (item_upgrade.rs bifrost_piece_exchange_opts) then repeats the exchange `count` times and banks / sells.
// Child rects at +0xC8 are in game-window client coordinates (same as the inventory slots in rce_store.cpp).
#include <windows.h>
#include <stdio.h>
#include <string>
#include <vector>

void Log(const char* msg);
void Drop_SendRaw(const std::vector<BYTE>& p);

namespace
{
    const BYTE WIZ_EXT_HOOK = 0xE9, SUB_OPTS = 0xE8;
    const DWORD VT_STRING = 0x01009EC8;
    const DWORD KO_PTR_GAMEMAIN = 0x011158FC;

    int g_count = 1;
    bool g_bank = false, g_sell = false;
    void* g_win = nullptr;
    bool g_wasVisible = false;

    void* Ptr(void* o, DWORD off) { return *(void**)((BYTE*)o + off); }
    const char* MsvcStr(const BYTE* s) { return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s; }
    const char* IdOf(void* o) { return MsvcStr((const BYTE*)o + 0x58); }
    bool Visible(void* o) { return o && *((BYTE*)o + 0xea) != 0; }

    void* ReadPtr(DWORD a)
    {
        __try { return (void*)*(DWORD*)a; }
        __except (EXCEPTION_EXECUTE_HANDLER) { return nullptr; }
    }
    bool LooksLikeWindowRaw(void* o);
    bool LooksLikeWindow(void* o)
    {
        __try { return LooksLikeWindowRaw(o); }
        __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
    }

    void* FindDeep(void* parent, const char* id, int depth = 0)
    {
        if (!parent || depth > 4) return nullptr;
        BYTE* head = (BYTE*)Ptr(parent, 0xb0);
        if (!head) return nullptr;
        int n = 0;
        for (BYTE* node = *(BYTE**)head; node && node != head && n++ < 400; node = *(BYTE**)node)
        {
            void* c = *(void**)(node + 8);
            if (!c) continue;
            if (strcmp(IdOf(c), id) == 0) return c;
            if (void* r = FindDeep(c, id, depth + 1)) return r;
        }
        return nullptr;
    }

    // first label (STRING element) whose text starts with `prefix` or with the checkbox mark + prefix
    void* FindLabel(void* parent, const char* prefix, int depth = 0)
    {
        if (!parent || depth > 4) return nullptr;
        BYTE* head = (BYTE*)Ptr(parent, 0xb0);
        if (!head) return nullptr;
        int n = 0;
        for (BYTE* node = *(BYTE**)head; node && node != head && n++ < 400; node = *(BYTE**)node)
        {
            void* c = *(void**)(node + 8);
            if (!c) continue;
            if (*(DWORD*)c == VT_STRING)
            {
                const char* t = MsvcStr((const BYTE*)c + 0x11c);
                if (!strncmp(t, prefix, strlen(prefix)) || !strncmp(t, "[x] ", 4) && !strncmp(t + 4, prefix, strlen(prefix)) ||
                    !strncmp(t, "[ ] ", 4) && !strncmp(t + 4, prefix, strlen(prefix))) return c;
            }
            if (void* r = FindLabel(c, prefix, depth + 1)) return r;
        }
        return nullptr;
    }

    void SetText(void* str, const std::string& s)
    {
        if (!str || strcmp(MsvcStr((const BYTE*)str + 0x11c), s.c_str()) == 0) return;
        void** vt = *(void***)str;
        ((void(__thiscall*)(void*, const std::string*))vt[0xd8 / 4])(str, &s);
    }

    bool RectOf(void* o, RECT* rc)
    {
        if (!o) return false;
        const float* f = (const float*)((BYTE*)o + 0xC8);
        *rc = { (LONG)f[0], (LONG)f[1], (LONG)f[2], (LONG)f[3] };
        return rc->right > rc->left && rc->bottom > rc->top;
    }

    bool LooksLikeWindowRaw(void* o)
    {
        DWORD p = (DWORD)o;
        if (p < 0x10000 || p > 0x7FFF0000 || (p & 3)) return false;
        DWORD vt = *(DWORD*)o;
        if (vt < 0x400000 || vt > 0x1200000) return false;
        return FindDeep(o, "btn_plus") && FindDeep(o, "btn_start") && FindDeep(o, "text_count");
    }

    void* FindWindow()
    {
        if (g_win && LooksLikeWindow(g_win)) return g_win;
        g_win = nullptr;
        static DWORD lastScan = 0;
        if (GetTickCount() - lastScan < 1500) return nullptr;      // scanning is not free: at most every 1.5 s
        lastScan = GetTickCount();
        static const DWORD bases[2] = { KO_PTR_GAMEMAIN, 0x01115910 };
        for (DWORD bp : bases)
        {
        DWORD gm = *(DWORD*)bp;
        if (!gm) continue;
        for (DWORD off = 4; off < 0x2000; off += 4)
        {
            void* o = ReadPtr(gm + off);
            if (LooksLikeWindow(o)) { g_win = o; char b[64]; sprintf_s(b, "CHAOTIC: window found at GameMain+0x%X", off); Log(b); return o; }
        }
        }
        return nullptr;
    }

    void Send()
    {
        Drop_SendRaw({ WIZ_EXT_HOOK, SUB_OPTS, (BYTE)g_count, (BYTE)(g_bank ? 1 : 0), (BYTE)(g_sell ? 1 : 0) });
        char b[96]; sprintf_s(b, "CHAOTIC: options count=%d bank=%d sell=%d", g_count, g_bank, g_sell); Log(b);
    }

    void Refresh(void* win)
    {
        SetText(FindDeep(win, "text_count"), std::to_string(g_count));
        SetText(FindLabel(win, "Auto Bank"), std::string(g_bank ? "[x] " : "[ ] ") + "Auto Bank");
        void* sell = FindLabel(win, "Sell if Not Uniq");
        SetText(sell, std::string(g_sell ? "[x] " : "[ ] ") + "Sell if Not Unique");
    }

    bool In(const RECT& r, int x, int y) { return x >= r.left && x < r.right && y >= r.top && y < r.bottom; }

    void ClickImpl(int x, int y)
    {
        void* win = FindWindow();
        if (!win || !Visible(win)) return;
        RECT plus, down, bank, sell, lbank, lsell;
        bool changed = false;
        if (RectOf(FindDeep(win, "btn_plus"), &plus) && In(plus, x, y)) { if (g_count < 100) { g_count++; changed = true; } }
        else if (RectOf(FindDeep(win, "btn_down"), &down) && In(down, x, y)) { if (g_count > 1) { g_count--; changed = true; } }
        else if ((RectOf(FindDeep(win, "btn_bank"), &bank) && In(bank, x, y)) || (RectOf(FindLabel(win, "Auto Bank"), &lbank) && In(lbank, x, y)))
        { g_bank = !g_bank; changed = true; }
        else if ((RectOf(FindDeep(win, "btn_sell"), &sell) && In(sell, x, y)) || (RectOf(FindLabel(win, "Sell if Not Uniq"), &lsell) && In(lsell, x, y)))
        { g_sell = !g_sell; changed = true; }
        if (changed) { Refresh(win); Send(); }
    }

    void TickImpl()
    {
        void* win = FindWindow();
        bool vis = win && Visible(win);
        if (vis)
        {
            Refresh(win);
            if (!g_wasVisible) Send();         // the server starts every run with the options shown in the window
        }
        g_wasVisible = vis;
    }
}

// game thread (ProxyWndProc): mouse released over the game window
void ChaoticOpts_OnClick(int x, int y)
{
    __try { ClickImpl(x, y); }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
}

void ChaoticOpts_Tick()
{
    static DWORD last = 0;
    DWORD now = GetTickCount();
    if (now - last < 150) return;
    last = now;
    __try { TickImpl(); }
    __except (EXCEPTION_EXECUTE_HANDLER) { g_win = nullptr; }
}
