// Knight Cash / TL balance + "Stat Reset" on the character status page (U) and "Skill Reset" in the skill window (K),
// as native controls of the game's own uifs (build_reset_uif.py: hg_kc, hg_tl, hg_stat_reset in re_page_state.uif,
// hg_skill_reset in re_skill.uif) - the game draws them itself, nothing of ours is on top of it.
//   - CUIState (vtable 0x103FF88) / CUISkillTreeDlg (vtable 0x103C6F0): ReceiveMessage (slot 33, msg 1 =
//     UIMSG_BUTTON_CLICK) is redirected here for the two buttons, everything else goes on to the game
//   - a reset needs two clicks (the caption turns into "Confirm?" for 3 s); it sends E9 CF 01 (stat) / E9 CF 02
//     (skill) = GameServer XGuard.cpp HSACSX_Send1299SkillAndStatReset -> AllPointChange / AllSkillPointChange, the
//     answer (WIZ_CLASS_CHANGE) is the game's own
//   - the balances are the ones pus_store.cpp keeps from the server's CASHCHANGE packets, written into the two
//     strings on the game thread (ResetBar_Tick)
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <string>

void Log(const char* msg);
void* HoldRepeat_StateWnd();                            // hold_repeat.cpp (the windows, once their Tick has run)
void* HoldRepeat_SkillWnd();
void Pus_GetBalances(UINT32* kc, UINT32* tl);           // pus_store.cpp

namespace
{
    const DWORD KO_SND_FNC = 0x00704070;       // thiscall(CAPISocket*, BYTE* buf, int len)
    const DWORD KO_PTR_PKT = 0x01115914;       // CAPISocket*
    const BYTE  WIZ_HSACS_HOOK = 0xE9, SUBOP_RESET = 0xCF;
    const DWORD VT_STATE = 0x0103FF88, VT_SKILL = 0x0103C6F0, VT_STRING = 0x01009EC8;
    const int   SLOT_RM = 33;
    const DWORD UIMSG_BUTTON_CLICK = 1, CONFIRM_MS = 3000;

    enum { R_STAT = 0, R_SKILL = 1, R_COUNT = 2 };
    struct Reset { const char* id; const char* label; BYTE op; DWORD armedAt; };
    Reset g_reset[R_COUNT] = { { "hg_stat_reset", "Stat Reset", 1, 0 }, { "hg_skill_reset", "Skill Reset", 2, 0 } };

    typedef bool(__thiscall* FnRM)(void*, void*, DWORD);
    FnRM g_origRM[R_COUNT] = {};

    // ---------------------------------------------------------------- CN3UI helpers (game thread)
    void* Ptr(void* o, DWORD off) { return *(void**)((BYTE*)o + off); }
    const char* MsvcStr(const BYTE* s) { return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s; }
    const char* IdOf(void* o) { return MsvcStr((const BYTE*)o + 0x58); }
    bool Visible(void* o) { return o && *((BYTE*)o + 0xea) != 0; }

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

    // a button's caption = the strings in it (directly, or one per state image)
    void SetCaption(void* btn, const std::string& s, int depth = 0)
    {
        if (!btn || depth > 2) return;
        BYTE* head = (BYTE*)Ptr(btn, 0xb0);
        if (!head) return;
        for (BYTE* n = *(BYTE**)head; n && n != head; n = *(BYTE**)n)
        {
            void* c = *(void**)(n + 8);
            if (!c) continue;
            if (*(DWORD*)c == VT_STRING) SetText(c, s);
            else SetCaption(c, s, depth + 1);
        }
    }

    void Click(Reset& r, void* btn)
    {
        if (!r.armedAt || GetTickCount() - r.armedAt >= CONFIRM_MS)
        {
            r.armedAt = GetTickCount() | 1;
            SetCaption(btn, "Confirm?");
            return;
        }
        r.armedAt = 0;
        SetCaption(btn, r.label);
        BYTE pkt[3] = { WIZ_HSACS_HOOK, SUBOP_RESET, r.op };
        if (void* sock = *(void**)KO_PTR_PKT)
        {
            ((void(__thiscall*)(void*, BYTE*, int))KO_SND_FNC)(sock, pkt, sizeof(pkt));
            Log(r.op == 1 ? "RESET: stat reset requested" : "RESET: skill reset requested");
        }
    }

    bool OnMessage(int which, void* self, void* sender, DWORD msg)
    {
        __try
        {
            Reset& r = g_reset[which];
            if (sender && msg == UIMSG_BUTTON_CLICK && strcmp(IdOf(sender), r.id) == 0) { Click(r, sender); return true; }
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
        return g_origRM[which](self, sender, msg);
    }
    bool __fastcall HkRMState(void* self, void*, void* sender, DWORD msg) { return OnMessage(R_STAT, self, sender, msg); }
    bool __fastcall HkRMSkill(void* self, void*, void* sender, DWORD msg) { return OnMessage(R_SKILL, self, sender, msg); }

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
        // the exe unpacks itself: wait until both vtables hold code addresses of the image (as hold_repeat.cpp)
        auto ready = [](DWORD vt) { DWORD v = *(DWORD*)(vt + SLOT_RM * 4); return v > 0x401000 && v < 0x00F90000; };
        for (int i = 0; i < 240 && !(ready(VT_STATE) && ready(VT_SKILL)); i++) Sleep(250);
        bool a = ready(VT_STATE) && PatchSlot(VT_STATE, SLOT_RM, (void*)HkRMState, (void**)&g_origRM[R_STAT]);
        bool b = ready(VT_SKILL) && PatchSlot(VT_SKILL, SLOT_RM, (void*)HkRMSkill, (void**)&g_origRM[R_SKILL]);
        char m[96]; sprintf_s(m, "RESET: stat / skill reset button hooks state=%d skill=%d", a, b); Log(m);
        return 0;
    }

    void RenameDailyEntry(void* parent, int depth = 0)
    {
        if (!parent || depth > 5) return;
        BYTE* head = (BYTE*)Ptr(parent, 0xb0);
        if (!head) return;
        int count = 0;
        for (BYTE* n = *(BYTE**)head; n && n != head && count++ < 200; n = *(BYTE**)n)
        {
            void* c = *(void**)(n + 8);
            if (!c) continue;
            if (*(DWORD*)c == VT_STRING)
            {
                const char* text = MsvcStr((const BYTE*)c + 0x11c);
                if (!_stricmp(text, "Board") || !_stricmp(text, "Event Board") || !_stricmp(text, "Manner Store"))
                    SetText(c, "Daily Quest");
            }
            RenameDailyEntry(c, depth + 1);
        }
    }

    void TickImpl()
    {
        void* main = *(void**)0x01115910;
        if (main) { void* menu = Ptr(main, 0x36c); if (Visible(menu)) RenameDailyEntry(menu); }
        void* win[R_COUNT] = { HoldRepeat_StateWnd(), HoldRepeat_SkillWnd() };
        for (int i = 0; i < R_COUNT; i++)                           // confirmation ran out / window closed: caption back
        {
            Reset& r = g_reset[i];
            if (!r.armedAt || (Visible(win[i]) && GetTickCount() - r.armedAt < CONFIRM_MS)) continue;
            r.armedAt = 0;
            if (win[i]) SetCaption(FindDeep(win[i], r.id), r.label);
        }
        if (!Visible(win[R_STAT])) return;
        void* kc = FindDeep(win[R_STAT], "hg_kc");
        static bool logged = false;
        if (!logged) { logged = true; Log(kc ? "RESET: KC / TL row of re_page_state.uif found" : "RESET: re_page_state.uif has no hg_kc (old uif in the pack)"); }
        if (!kc) return;
        UINT32 nKc = 0, nTl = 0; Pus_GetBalances(&nKc, &nTl);
        SetText(kc, std::to_string(nKc));
        SetText(FindDeep(win[R_STAT], "hg_tl"), std::to_string(nTl));
    }
}

void ResetBar_Init()
{
    CloseHandle(CreateThread(nullptr, 0, InstallThread, nullptr, 0, nullptr));
}

// game thread (d3d9proxy.cpp ProxyWndProc)
void ResetBar_Tick()
{
    static DWORD last = 0;
    DWORD now = GetTickCount();
    if (now - last < 150) return;
    last = now;
    __try { TickImpl(); }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
}
