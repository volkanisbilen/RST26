// Hold to repeat for the stat / skill point "+" buttons (the game only gives one point per click).
// CUIState (vtable 0x103FF88, re_page_state.uif: Btn_Strength .. Btn_MagicAttack) and CUISkillTreeDlg
// (vtable 0x103C6F0, re_skill.uif: btn_4 .. btn_7, btn_master9): their Tick (vtable slot 35) is hooked only to
// learn the window objects. Left button down over one of those buttons arms a timer on the game window; after
// HOLD_DELAY_MS, while the button stays pressed and the cursor on it, the game's own click handler
// (ReceiveMessage, slot 33, msg 1) runs every REPEAT_MS - the game then sends the point packets itself.
#include <windows.h>
#include <stdio.h>
#include <string.h>

void Log(const char* msg);
HWND Proxy_GameWnd();                                   // d3d9proxy.cpp (subclassed game window)

namespace
{
    const DWORD VT_STATE = 0x0103FF88, VT_SKILL = 0x0103C6F0;
    const int   SLOT_RM = 33, SLOT_TICK = 35;
    const UINT_PTR TIMER_ID = 0x4801;
    const DWORD HOLD_DELAY_MS = 400, REPEAT_MS = 60;
    const char* const kStateIds[] = { "Btn_Strength", "Btn_Stamina", "Btn_Dexterity", "Btn_Intelligence", "Btn_MagicAttack" };
    const char* const kSkillIds[] = { "btn_4", "btn_5", "btn_6", "btn_7", "btn_master9" };

    typedef void(__thiscall* FnTick)(void*);
    typedef bool(__thiscall* FnRM)(void*, void*, DWORD);
    FnTick g_origTickState = nullptr, g_origTickSkill = nullptr;
    void* volatile g_state = nullptr;
    void* volatile g_skill = nullptr;

    void* g_armWin = nullptr;    // window + button being held
    void* g_armBtn = nullptr;
    DWORD g_armAt = 0, g_lastFire = 0;

    void* Ptr(void* o, DWORD off) { return *(void**)((BYTE*)o + off); }
    const char* MsvcStr(const BYTE* s) { return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s; }
    const char* IdOf(void* o) { return MsvcStr((const BYTE*)o + 0x58); }
    bool Visible(void* o) { return o && *((BYTE*)o + 0xea) != 0; }
    const float* RectOf(void* o) { return (const float*)((BYTE*)o + 0xc8); }

    void* FindDeep(void* parent, const char* id, int depth = 0)
    {
        if (!parent || depth > 5) return nullptr;
        BYTE* head = (BYTE*)Ptr(parent, 0xb0);
        if (!head) return nullptr;
        for (BYTE* n = *(BYTE**)head; n && n != head; n = *(BYTE**)n)
        {
            void* c = *(void**)(n + 8);
            if (!c) continue;
            if (_stricmp(IdOf(c), id) == 0) return c;
            if (void* r = FindDeep(c, id, depth + 1)) return r;
        }
        return nullptr;
    }

    bool Inside(void* btn, int x, int y)
    {
        if (!Visible(btn)) return false;
        const float* r = RectOf(btn);
        return x >= r[0] && x < r[2] && y >= r[1] && y < r[3];
    }

    void* ButtonAt(void* win, const char* const* ids, int n, int x, int y)
    {
        if (!Visible(win)) return nullptr;
        for (int i = 0; i < n; i++)
            if (void* b = FindDeep(win, ids[i]); b && Inside(b, x, y)) return b;
        return nullptr;
    }

    // The game reads the mouse through DirectInput, so WM_LBUTTONDOWN does not always reach the window: the
    // button is polled here, every frame of the window's own Tick (game thread).
    struct Poll { void* btn = nullptr; DWORD downAt = 0, last = 0; bool wasDown = false; };
    Poll g_pollState, g_pollSkill;

    void PollHold(void* win, const char* const* ids, int n, Poll& p)
    {
        __try
        {
            bool down = (GetAsyncKeyState(VK_LBUTTON) & 0x8000) != 0;
            HWND gw = Proxy_GameWnd();
            if (!gw || !Visible(win)) { p = Poll(); return; }
            POINT c; GetCursorPos(&c); ScreenToClient(gw, &c);
            DWORD now = GetTickCount();
            if (down && !p.wasDown)                                          // press: remember the button under it
            {
                p.btn = ButtonAt(win, ids, n, c.x, c.y);
                p.downAt = now; p.last = 0;
            }
            p.wasDown = down;
            if (!down || !p.btn) { if (!down) p.btn = nullptr; return; }
            if (!Inside(p.btn, c.x, c.y)) { p.btn = nullptr; return; }     // dragged off the button
            if (now - p.downAt < HOLD_DELAY_MS || now - p.last < REPEAT_MS) return;
            p.last = now;
            void** vt = *(void***)win;
            ((FnRM)vt[SLOT_RM])(win, p.btn, 1);                              // the game's own "+" click
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { p = Poll(); }
    }

    void __fastcall HkTickState(void* self, void*) { g_state = self; g_origTickState(self); PollHold(self, kStateIds, 5, g_pollState); }
    void __fastcall HkTickSkill(void* self, void*) { g_skill = self; g_origTickSkill(self); PollHold(self, kSkillIds, 5, g_pollSkill); }

    bool PatchSlot(DWORD vt, int slot, void* hook, void** orig)
    {
        DWORD* p = (DWORD*)(vt + slot * 4);
        DWORD old;
        if (!VirtualProtect(p, 4, PAGE_READWRITE, &old)) return false;
        *orig = (void*)*p;
        *p = (DWORD)hook;
        VirtualProtect(p, 4, old, &old);
        return true;
    }

    DWORD WINAPI InstallThread(LPVOID)
    {
        // the exe unpacks itself: wait until both vtables hold code addresses of the image
        auto ready = [](DWORD vt) { DWORD v = *(DWORD*)(vt + SLOT_TICK * 4); return v > 0x401000 && v < 0x00F90000; };
        for (int i = 0; i < 240 && !(ready(VT_STATE) && ready(VT_SKILL)); i++) Sleep(250);
        bool a = ready(VT_STATE) && PatchSlot(VT_STATE, SLOT_TICK, (void*)HkTickState, (void**)&g_origTickState);
        bool b = ready(VT_SKILL) && PatchSlot(VT_SKILL, SLOT_TICK, (void*)HkTickSkill, (void**)&g_origTickSkill);
        char m[96]; sprintf_s(m, "HOLD: stat / skill point repeat hooks state=%d skill=%d", a, b); Log(m);
        return 0;
    }
}

// game thread (ProxyWndProc): WM_LBUTTONDOWN at client coordinates
void HoldRepeat_OnMouseDown(HWND h, int x, int y)
{
    return;                                             // replaced by PollHold (Tick); kept so the window hook still links
    __try
    {
        void* win = nullptr; void* btn = nullptr;
        if (g_state && (btn = ButtonAt(g_state, kStateIds, 5, x, y))) win = g_state;
        else if (g_skill && (btn = ButtonAt(g_skill, kSkillIds, 5, x, y))) win = g_skill;
        if (!btn) return;
        g_armWin = win; g_armBtn = btn; g_armAt = GetTickCount(); g_lastFire = 0;
        SetTimer(h, TIMER_ID, REPEAT_MS / 2, nullptr);
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { g_armBtn = nullptr; }
}

// game thread: true when the WM_TIMER was ours (do not pass it on)
bool HoldRepeat_OnTimer(HWND h, UINT_PTR id)
{
    if (id != TIMER_ID) return false;
    __try
    {
        POINT p; GetCursorPos(&p); ScreenToClient(h, &p);
        bool held = (GetAsyncKeyState(VK_LBUTTON) & 0x8000) != 0;
        if (!g_armBtn || !held || !Inside(g_armBtn, p.x, p.y)) { KillTimer(h, TIMER_ID); g_armBtn = nullptr; return true; }
        DWORD now = GetTickCount();
        if (now - g_armAt < HOLD_DELAY_MS || now - g_lastFire < REPEAT_MS) return true;
        g_lastFire = now;
        void** vt = *(void***)g_armWin;
        ((FnRM)vt[SLOT_RM])(g_armWin, g_armBtn, 1);      // the game's own "+" click
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { KillTimer(h, TIMER_ID); g_armBtn = nullptr; }
    return true;
}

// reset_bar.cpp: the two windows, once their Tick has run (null before)
void* HoldRepeat_StateWnd() { return g_state; }
void* HoldRepeat_SkillWnd() { return g_skill; }

void HoldRepeat_Init()
{
    CloseHandle(CreateThread(nullptr, 0, InstallThread, nullptr, 0, nullptr));
}
