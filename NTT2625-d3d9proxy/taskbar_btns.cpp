// New taskbar (the supplied re_taskbar_main.uif) has three buttons the game does not know:
//   btn_daily_quest -> Daily Quest panel   (server: hook 0xE9 / 0xE7 [0], same as the old event menu entry)
//   btn_wof         -> Wheel of Fun        (server: hook 0xE9 / 0xE7 [2])
//   btn_mobsearch   -> drop search panel   (drop_panel.cpp Drop_Toggle)
// The game ignores clicks on unknown buttons, so a mouse up on the game window that lands inside one of their rects
// (child rects at +0xC8 are in game-window client coordinates) runs the action.
#include <windows.h>
#include <stdio.h>
#include <string>
#include <vector>

void Log(const char* msg);
void Drop_SendRaw(const std::vector<BYTE>& p);
void Drop_Toggle(bool loadTarget);

namespace
{
    const DWORD KO_PTR_GAMEMAIN = 0x011158FC;
    void* g_win = nullptr;

    void* Ptr(void* o, DWORD off) { return *(void**)((BYTE*)o + off); }
    const char* MsvcStr(const BYTE* s) { return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s; }
    const char* IdOf(void* o) { return MsvcStr((const BYTE*)o + 0x58); }
    bool Visible(void* o) { return o && *((BYTE*)o + 0xea) != 0; }

    void* ReadPtr(DWORD a)
    {
        __try { return (void*)*(DWORD*)a; }
        __except (EXCEPTION_EXECUTE_HANDLER) { return nullptr; }
    }
    bool LooksLikeTaskbarRaw(void* o);
    bool LooksLikeTaskbar(void* o)
    {
        __try { return LooksLikeTaskbarRaw(o); }
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

    bool LooksLikeTaskbarRaw(void* o)
    {
        DWORD p = (DWORD)o;
        if (p < 0x10000 || p > 0x7FFF0000 || (p & 3)) return false;
        DWORD vt = *(DWORD*)o;
        if (vt < 0x400000 || vt > 0x1200000) return false;
        return FindDeep(o, "btn_daily_quest") && FindDeep(o, "btn_powerup");
    }

    void* FindTaskbar()
    {
        if (g_win && LooksLikeTaskbar(g_win)) return g_win;
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
            if (LooksLikeTaskbar(o)) { g_win = o; char b[64]; sprintf_s(b, "TASKBAR: new taskbar at GameMain+0x%X", off); Log(b); return o; }
        }
        }
        return nullptr;
    }

    bool Hit(void* btn, int x, int y)
    {
        if (!btn || !Visible(btn)) return false;
        const float* f = (const float*)((BYTE*)btn + 0xC8);
        return x >= (int)f[0] && x < (int)f[2] && y >= (int)f[1] && y < (int)f[3];
    }

    void ClickImpl(int x, int y)
    {
        void* win = FindTaskbar();
        if (!win) return;
        if (Hit(FindDeep(win, "btn_daily_quest"), x, y))
        {
            Drop_SendRaw({ 0xE9, 0xE7, 0 });
            Log("TASKBAR: Daily Quest");
        }
        else if (Hit(FindDeep(win, "btn_wof"), x, y))
        {
            Drop_SendRaw({ 0xE9, 0xE7, 2 });
            Log("TASKBAR: Wheel of Fun");
        }
        else if (Hit(FindDeep(win, "btn_eventnew"), x, y))
        {
            Drop_SendRaw({ 0x9C, 0xF0 });             // what the old star button sent: the server answers with the event menu
            Log("TASKBAR: event menu");
        }
        else if (Hit(FindDeep(win, "btn_mobsearch"), x, y))
        {
            Drop_Toggle(false);
            Log("TASKBAR: drop search");
        }
    }
}

// game thread (ProxyWndProc): mouse released over the game window
void Taskbar_OnClick(int x, int y)
{
    __try { ClickImpl(x, y); }
    __except (EXCEPTION_EXECUTE_HANDLER) { g_win = nullptr; }
}
