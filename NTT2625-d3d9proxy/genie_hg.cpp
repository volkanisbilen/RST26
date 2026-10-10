// HopeGuard Genie: two tabs (Main / Misc) on top of the game's five-tab Knight Genie (CUIGenie_Main),
// plus the extra controls of the HopeGuard window (ids hg_* in re_genie.uif, built by build_genie.py).
//
// Tabs (native tab switching shows exactly one base and one icon group):
//   Main = "attack" + "assist" bases, attack/recovery skill icons + support icons (0..15 by style, 20..31)
//   Misc = "etc" + "recovery" bases + the Misc part of "assist" (auto curse, transform, sub-base hg_ax),
//          potion icons (16..18)
// Patches (original bytes are checked first):
//   0xB1E950  explain tab fn (window open default)    -> jmp HG_ShowMain
//   0xB1D08A  btn_attack block: call RefreshTab(1)     -> call HG_RefreshMain
//   0xB1D321  btn_etc block:    call RefreshTab(4)     -> call HG_RefreshMisc
//   vtable 0x102B01C[33] ReceiveMessage(sender, msg)   -> HG_ReceiveMessage (hg_* controls, then native)
//   0x704070  CAPISocket::Send                          -> 3-5 Combo: every archer 3-arrow packet is
//                                                          followed by the same packet for the 5-arrow skill
// CUIGenie_Main offsets: 0x140 attack, 0x1b0 recovery, 0x20c assist, 0x28c grp_transform_list,
// 0x218[12] btn_party_N, 0x490 attack-style page, 0x49d[12] party flags, 0x574[32] skill icon holders.
// Native option flags driven by the mirrored hg_ controls:
//   0x4b6 "basic attack off" (btn_not_attack 0x1a4)     R Attack = !flag
//   0x495 move to monsters (hunting in place 0x168)     Go To Monster = flag
//   0x4b8 back to start (btn_no_enemy_stay 0x174)       Back to starting point = flag
//   0x4b9 combo (btn_rapid_hit 0x1ac)                   3-5 Combo = flag
// CN3UI: id std::string at +0x58, child list at +0xb0, button state at +0xbc, CN3UIString text at +0x11c,
// vtable +0x5c SetState, +0x64 SetVisible, +0xd8 SetString.
#include <windows.h>
#include <stdio.h>
#include <math.h>
#include <string>
#include <vector>

void Log(const char* msg);
void NetTrace_Packet(const char* dir, const BYTE* buf, int len);   // net_trace.cpp (TEMP)
bool Reconnect_FilterSend(void* sock, const BYTE* buf, int len, void(__thiscall* sendFn)(void*, BYTE*, int));   // reconnect.cpp
static void PatchGoRange();
static void PatchComboWalkUp();
static void PatchSkillPlacement();
static void PatchSlotHitTest();
static void PatchEventMenu();
extern "C" volatile int g_hgEventEntry = -1;   // last event menu entry picked (sub_7D7A60), -1 = none

namespace
{
    typedef void(__thiscall* Fn0)(void*);
    typedef void(__thiscall* Fn1)(void*, int);
    typedef bool(__thiscall* FnRM)(void*, void*, DWORD);
    typedef void(__thiscall* FnSend)(void*, BYTE*, int);
    const Fn0 AttackTab = (Fn0)0x00B1E840;
    const Fn0 FillTransformList = (Fn0)0x00B11660;
    const DWORD VT_GENIE_RM = 0x0102B01C + 33 * 4;
    const DWORD KO_SND_FNC = 0x00704070;
    const DWORD KO_PTR_PKT = 0x01115914;
    const BYTE WIZ_HSACS_HOOK = 0xE9, HG_SUBOP = 0xED;      // HSACSXOpCodes::HGGENIE (v4 packets.h)

    FnRM g_origRM = nullptr;
    FnSend g_sendTramp = nullptr;
    void* volatile g_self = nullptr;
    bool g_misc = false;

    // ---------------------------------------------------------------- settings
    // Stored where the game keeps its own genie options: the save packet (0x97 01 03, built by
    // CUIGenie_Main::SaveOptions 0xB1FFE0) gets the HopeGuard block appended at blob offset 180, the server
    // writes the 400-byte blob to USER_GENIE_DATA.strGenieOptions and sends it back on load (0x97 01 02).
    //   +0 'H' 'G' ver  +3 leader  +4 goRange  +5 autoParty  +6 partyHeal%  +7 codeLen  +8 code[20]
    //   +28 mobCount  +29 mobs (9 x [len][19 chars])
    const int HG_NATIVE = 180, HG_BLOCK = 220, HG_MOBS = 9;
    struct Settings { int leader = 0, goRange = 0, autoParty = 0, partyHeal = 50; std::string pt; std::vector<std::string> mobs; } g_set;
    CRITICAL_SECTION g_setLock;
    bool g_loaded = false;          // block received from the server (or defaults after the first load packet)
    bool g_dirty = false;           // our settings changed: ask the game to save its genie options
    typedef void(__thiscall* FnSave)(void*);
    const FnSave GenieSaveOptions = (FnSave)0x00B1FFE0;

    void BuildBlock(BYTE* p)
    {
        memset(p, 0, HG_BLOCK);
        EnterCriticalSection(&g_setLock);
        p[0] = 'H'; p[1] = 'G'; p[2] = 1;
        p[3] = (BYTE)g_set.leader; p[4] = (BYTE)g_set.goRange; p[5] = (BYTE)g_set.autoParty; p[6] = (BYTE)g_set.partyHeal;
        std::string code = g_set.pt.substr(0, 20);
        p[7] = (BYTE)code.size(); memcpy(p + 8, code.data(), code.size());
        size_t n = min(g_set.mobs.size(), (size_t)HG_MOBS);
        p[28] = (BYTE)n;
        for (size_t i = 0; i < n; i++)
        {
            std::string m = g_set.mobs[i].substr(0, 19);
            p[29 + i * 20] = (BYTE)m.size(); memcpy(p + 30 + i * 20, m.data(), m.size());
        }
        LeaveCriticalSection(&g_setLock);
    }
    void ParseBlock(const BYTE* p)
    {
        Settings s;
        if (p[0] == 'H' && p[1] == 'G')
        {
            s.leader = p[3]; s.goRange = p[4]; s.autoParty = p[5]; s.partyHeal = p[6] ? p[6] : 50;
            s.pt.assign((const char*)p + 8, min((int)p[7], 20));
            for (int i = 0; i < min((int)p[28], HG_MOBS); i++)
                s.mobs.emplace_back((const char*)p + 30 + i * 20, min((int)p[29 + i * 20], 19));
        }
        EnterCriticalSection(&g_setLock);
        g_set = s;
        LeaveCriticalSection(&g_setLock);
        g_loaded = true;
    }

    // ---------------------------------------------------------------- CN3UI helpers
    inline void* Ptr(void* o, DWORD off) { return *(void**)((BYTE*)o + off); }
    inline BYTE& Flag(void* self, DWORD off) { return *((BYTE*)self + off); }
    inline void VCall(void* o, DWORD slot, int v)
    {
        if (!o) return;
        void** vt = *(void***)o;
        ((Fn1)vt[slot / 4])(o, v);
    }
    inline void SetVisible(void* o, bool v) { VCall(o, 0x64, v ? 1 : 0); }
    inline void SetState(void* o, int s) { if (o && *(int*)((BYTE*)o + 0xbc) != s) VCall(o, 0x5c, s); }

    const char* MsvcStr(const BYTE* s)     // MSVC x86 std::string: buf[16] | size | capacity
    {
        return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s;
    }
    const char* IdOf(void* o) { return MsvcStr((const BYTE*)o + 0x58); }
    std::string TextOf(void* str) { return str ? std::string(MsvcStr((const BYTE*)str + 0x11c)) : std::string(); }

    template <class F> void ForChildren(void* parent, F f)
    {
        if (!parent) return;
        BYTE* head = (BYTE*)Ptr(parent, 0xb0);
        if (!head) return;
        for (BYTE* n = *(BYTE**)head; n && n != head; n = *(BYTE**)n)
            if (void* c = *(void**)(n + 8)) f(c);
    }
    void* Find(void* parent, const char* id)
    {
        void* r = nullptr;
        ForChildren(parent, [&](void* c) { if (!r && strcmp(IdOf(c), id) == 0) r = c; });
        return r;
    }
    void* FindIn(void* self, const char* id)
    {
        static const DWORD bases[] = { 0x140, 0x20c, 0x2b4, 0x1b0 };
        for (DWORD b : bases)
            if (void* c = Find(Ptr(self, b), id)) return c;
        if (void* ax = Find(Ptr(self, 0x20c), "hg_ax"))
            return Find(ax, id);
        return nullptr;
    }

    // ---------------------------------------------------------------- saving
    // our settings ride on the game's genie save; Genie_Tick calls it on the game thread once things settle
    DWORD g_dirtyAt = 0;
    void MarkDirty() { g_dirty = true; g_dirtyAt = GetTickCount(); }

    // ---------------------------------------------------------------- tabs
    const char* const kMiscAssist[] = { "hg_ax", "btn_removecurse_onoff", "btn_transform_onoff", "str_transform", "btn_transform_list" };
    bool IsMiscAssist(const char* id)
    {
        for (const char* m : kMiscAssist) if (strcmp(id, m) == 0) return true;
        return false;
    }
    void AssistSplit(void* self, bool misc)
    {
        void* assist = Ptr(self, 0x20c);
        if (!assist) return;
        SetVisible(assist, true);
        ForChildren(assist, [&](void* c) {
            const char* id = IdOf(c);
            if (strcmp(id, "grp_transform_list") == 0) { SetVisible(c, false); return; }
            if (strncmp(id, "str_", 4) == 0 && strcmp(id, "str_transform") != 0) return;   // parked natives
            SetVisible(c, IsMiscAssist(id) == misc);
        });
        g_misc = misc;
    }

    void Icons(void* self, bool main)
    {
        BYTE* arr = (BYTE*)self + 0x574;
        for (int i = 0; i < 32; i++)
        {
            void* holder = *(void**)(arr + 4 * i);
            if (!holder) continue;
            void* icon = *(void**)holder;
            if (!icon) continue;
            bool vis;
            if (main) vis = i < 16 || (i >= 20 && i < 28);            // attack + recovery rows both shown, support 1..8
            else vis = i >= 16 && i <= 18;
            SetVisible(icon, vis);
        }
    }

    void SyncVisuals(void* self);

    void MainExtras(void* self)
    {
        AssistSplit(self, false);
        FillTransformList(self);
        Icons(self, true);
        for (int k = 0; k < 12; k++)                 // party ticks of the support slots
        {
            void* btn = Ptr(self, 0x218 + 4 * k);
            if (btn && Flag(self, 0x49d + k) == 1) SetState(btn, 3);
        }
        SyncVisuals(self);
    }

    void __fastcall HG_ShowMain(void* self)
    {
        g_self = self;
        AttackTab(self);
        MainExtras(self);
    }
    void __fastcall HG_RefreshMain(void* self, void*, int) { g_self = self; MainExtras(self); }
    void __fastcall HG_RefreshMisc(void* self, void*, int)
    {
        g_self = self;
        SetVisible(Ptr(self, 0x1b0), true);          // recovery base next to etc
        AssistSplit(self, true);
        Icons(self, false);
        SyncVisuals(self);
    }

    // ---------------------------------------------------------------- monster list = genie target slots
    // CUIGenie_Sub (mini bar, [CGameProcMain+0x540]): 3 slots, monster kind at +0x160+4i ([npc+0xB8C]),
    // on flag +0x178+i, extra +0x16c+4i; the genie's FindTarget 0x86FED0 attacks the kinds of the "on" slots.
    const int kSlots = 3;
    int g_mobSel = 0;
    std::string g_slotName[kSlots];
    const char* const kSlotName[kSlots] = { "One", "Two", "Three" };
    void* GameMain() { return *(void**)0x011158FC; }
    void* GenieSub() { BYTE* gm = (BYTE*)GameMain(); return gm ? Ptr(gm, 0x540) : nullptr; }
    void* FindDeep(void* parent, const char* id)
    {
        void* r = nullptr;
        ForChildren(parent, [&](void* c) {
            if (r) return;
            if (strcmp(IdOf(c), id) == 0) r = c;
            else r = FindDeep(c, id);
        });
        return r;
    }
    void SubClick(const char* what, int slot)
    {
        void* sub = GenieSub();
        if (!sub) return;
        char id[48]; sprintf_s(id, "btn_Target%s%s", kSlotName[slot], what);
        void* btn = FindDeep(sub, id);
        if (!btn) return;
        void** vt = *(void***)sub;
        ((FnRM)vt[33])(sub, btn, 1);
    }
    DWORD SlotMob(int slot) { void* sub = GenieSub(); return sub ? *(DWORD*)((BYTE*)sub + 0x160 + 4 * slot) : 0; }
    bool SlotOn(int slot) { void* sub = GenieSub(); return sub && *((BYTE*)sub + 0x178 + slot) != 0; }
    std::string SlotName(int slot)
    {
        void* sub = GenieSub();
        char id[32]; sprintf_s(id, "str_Name%s", kSlotName[slot]);
        return sub ? TextOf(FindDeep(sub, id)) : std::string();
    }

    // ---------------------------------------------------------------- hg_ controls
    void SetText(void* str, const std::string& s)
    {
        if (!str || TextOf(str) == s) return;
        void** vt = *(void***)str;
        ((void(__thiscall*)(void*, const std::string*))vt[0xd8 / 4])(str, &s);
    }

    void SetPair(void* self, const char* pid, bool on)
    {
        std::string a = std::string(pid) + "_on", b = std::string(pid) + "_off";
        SetState(FindIn(self, a.c_str()), on ? 3 : 2);
        SetState(FindIn(self, b.c_str()), on ? 2 : 3);
    }
    void SetCheck(void* self, const char* id, bool on) { SetState(FindIn(self, id), on ? 3 : 2); }

    // Native On/Off pairs (CUIGenie_Main button members -> option flag, from the game's RM 0xB1D34B..0xB1D98D).
    // Their lit state is normally set by the game's own tab refresh, which our Main / Misc pages don't run,
    // so both halves looked off after a load or a page switch; clicks are handled here as well (On = 1).
    struct NativePair { DWORD on, off, flag; };
    const NativePair kNativePairs[] = {
        { 0x150, 0x154, 0x491 },   // Attack Mode
        { 0x158, 0x15c, 0x492 },   // Recovery Mode
        { 0x1b8, 0x1bc, 0x496 },   // HP potion
        { 0x1c8, 0x1cc, 0x498 },   // MP potion
        { 0x1d8, 0x1dc, 0x49a },   // Pet potion
        { 0x210, 0x214, 0x49c },   // Support Mode (assist)
    };
    bool HandleNativePair(void* self, void* sender)
    {
        for (const NativePair& p : kNativePairs)
        {
            void* on = Ptr(self, p.on); void* off = Ptr(self, p.off);
            if (!on || !off || (sender != on && sender != off)) continue;
            Flag(self, p.flag) = sender == on ? 1 : 0;
            return true;
        }
        return false;
    }

    void SyncVisuals(void* self)
    {
        for (const NativePair& p : kNativePairs)
        {
            bool v = Flag(self, p.flag) != 0;
            SetState(Ptr(self, p.on), v ? 3 : 2);
            SetState(Ptr(self, p.off), v ? 2 : 3);
        }
        SetPair(self, "hg_r_attack", Flag(self, 0x4b6) == 0);
        SetPair(self, "hg_leader_target", g_set.leader != 0);
        SetState(FindIn(self, "hg_combo_off"), Flag(self, 0x4b9) ? 2 : 3);
        // third row: "On" is the native tick button itself, "Off" lights when the option is off
        SetState(FindIn(self, "hg_hide_range_off"), Flag(self, 0x4b7) ? 2 : 3);
        SetState(FindIn(self, "hg_hunt_off"), Flag(self, 0x495) == 0 ? 2 : 3);           // hunting in place = !move
        SetState(FindIn(self, "hg_backstart_off"), Flag(self, 0x4b8) ? 2 : 3);
        SetCheck(self, "hg_goto_mob", Flag(self, 0x495) != 0);
        SetCheck(self, "hg_goto_range", g_set.goRange != 0);
        SetCheck(self, "hg_back_start", Flag(self, 0x4b8) != 0);
        SetCheck(self, "hg_auto_party", g_set.autoParty != 0);
        for (int i = 0; i < kSlots; i++)
        {
            char id[32]; sprintf_s(id, "hg_mob_%d", i + 1);
            bool filled = SlotMob(i) != 0;
            SetState(FindIn(self, id), filled && i == g_mobSel ? 3 : 2);     // selected row = lit
            std::string name;
            if (filled) name = !g_slotName[i].empty() ? g_slotName[i] : SlotName(i);
            if (filled && name.empty()) name = "monster";
            strcat_s(id, "_str");
            SetText(FindIn(self, id), name);
        }
    }

    // "+": the player's current target (Z / click) goes into a genie target slot.
    // Target id = [player+0x660] (-1 = none, see TargetSelect 0x8129B0); character by id = 0x50DEB0
    // (thiscall [0x1115840], id, 0), as CGameProcMain 0x812A80 does: monster when [ch+0xB9C] == 1,
    // kind [ch+0xB8C] (the value FindTarget compares), name std::string at [ch+0x6A4].
    // Already listed -> selected; else first free slot; else the selected row is replaced. Slot is switched on.
    typedef BYTE* (__thiscall* FnCharById)(void*, int, int);
    void AddSelectedMonster()
    {
        BYTE* sub = (BYTE*)GenieSub();
        BYTE* me = *(BYTE**)0x01115834;
        void* mgr = *(void**)0x01115840;
        if (!sub || !me || !mgr) return;
        int tid = *(int*)(me + 0x660);
        BYTE* ch = tid >= 0 ? ((FnCharById)0x0050DEB0)(mgr, tid, 0) : nullptr;
        if (!ch || *(ch + 0xB9C) != 1) { Log("GENIE: + without a selected monster"); return; }
        DWORD kind = *(DWORD*)(ch + 0xB8C);
        std::string name = MsvcStr(ch + 0x6A4);
        int slot = -1;
        for (int i = 0; i < kSlots && slot < 0; i++)
            if (SlotMob(i) == kind) slot = i;
        for (int i = 0; i < kSlots && slot < 0; i++)
            if (SlotMob(i) == 0) slot = i;
        if (slot < 0) slot = g_mobSel;
        *(DWORD*)(sub + 0x160 + 4 * slot) = kind;
        *(sub + 0x178 + slot) = 1;
        g_slotName[slot] = name;
        char id[32]; sprintf_s(id, "str_Name%s", kSlotName[slot]);
        SetText(FindDeep(sub, id), name);
        g_mobSel = slot;
        char b[128]; sprintf_s(b, "GENIE: slot %d = %s (kind %lu, id %d)", slot + 1, name.c_str(), kind, tid); Log(b);
    }

    // returns true when the sender was one of ours
    bool HandleHG(void* self, void* sender)
    {
        const char* id = IdOf(sender);
        if (strncmp(id, "hg_", 3) != 0) return false;
        if (!strcmp(id, "hg_r_attack_on"))       { Flag(self, 0x4b6) = 0; SetState(Ptr(self, 0x1a4), 2); }
        else if (!strcmp(id, "hg_r_attack_off")) { Flag(self, 0x4b6) = 1; SetState(Ptr(self, 0x1a4), 3); }
        else if (!strcmp(id, "hg_leader_target_on"))  { g_set.leader = 1; MarkDirty(); }
        else if (!strcmp(id, "hg_leader_target_off")) { g_set.leader = 0; MarkDirty(); }
        else if (!strcmp(id, "hg_combo_off"))    { Flag(self, 0x4b9) = 0; SetState(Ptr(self, 0x1ac), 2); }
        else if (!strcmp(id, "hg_hide_range_off")) { Flag(self, 0x4b7) = 0; SetState(Ptr(self, 0x16c), 0); }
        else if (!strcmp(id, "hg_hunt_off"))       { Flag(self, 0x495) = 1; SetState(Ptr(self, 0x168), 0); }
        else if (!strcmp(id, "hg_backstart_off"))  { Flag(self, 0x4b8) = 0; SetState(Ptr(self, 0x174), 0); }
        else if (!strcmp(id, "hg_goto_mob"))
        {
            BYTE v = Flag(self, 0x495) ? 0 : 1;
            Flag(self, 0x495) = v; SetState(Ptr(self, 0x168), v ? 0 : 3);     // hunting in place = !v
        }
        else if (!strcmp(id, "hg_back_start"))
        {
            BYTE v = Flag(self, 0x4b8) ? 0 : 1;
            Flag(self, 0x4b8) = v; SetState(Ptr(self, 0x174), v ? 3 : 0);
        }
        else if (!strcmp(id, "hg_goto_range"))   { g_set.goRange = !g_set.goRange; MarkDirty(); }
        else if (!strcmp(id, "hg_auto_party"))   { g_set.autoParty = !g_set.autoParty; MarkDirty(); }
        else if (!strcmp(id, "hg_save"))         { g_dirty = false; GenieSaveOptions(self); Log("GENIE: settings saved"); }
        else if (!strcmp(id, "hg_mob_add"))
            AddSelectedMonster();
        else if (!strcmp(id, "hg_mob_del"))
        {
            int i = g_mobSel;
            if (!SlotMob(i)) for (int k = kSlots - 1; k >= 0; k--) if (SlotMob(k)) { i = k; break; }
            if (BYTE* sub = (BYTE*)GenieSub(); sub && SlotMob(i))     // same fields the game's own clear (0xB2552E) resets
            {
                *(DWORD*)(sub + 0x160 + 4 * i) = 0;
                *(DWORD*)(sub + 0x16c + 4 * i) = 0;
                *(sub + 0x178 + i) = 0;
                char nid[32]; sprintf_s(nid, "str_Name%s", kSlotName[i]);
                SetText(FindDeep(sub, nid), std::string());
                g_slotName[i].clear();
            }
        }
        else if (!strcmp(id, "hg_mob_1") || !strcmp(id, "hg_mob_2") || !strcmp(id, "hg_mob_3"))
            g_mobSel = id[7] - '1';
        else return true;                        // other hg_ ids (labels, list buttons): nothing yet
        SyncVisuals(self);
        return true;
    }

    bool __fastcall HG_ReceiveMessage(void* self, void*, void* sender, DWORD msg)
    {
        g_self = self;
        bool r;
        if (sender && msg == 1 && (HandleNativePair(self, sender) || HandleHG(self, sender))) r = true;
        else r = g_origRM(self, sender, msg);
        if (sender && msg == 1 && sender == Ptr(self, 0x288))   // TEMP: transform dropdown diagnostics
        {
            void* grp = Ptr(self, 0x28c);
            int n = 0; ForChildren(grp, [&](void*) { n++; });
            char b[160]; sprintf_s(b, "GENIE: transform list button -> list %p visible=%d children=%d misc=%d",
                grp, grp ? *((BYTE*)grp + 0xea) : -1, n, (int)g_misc);
            Log(b);
        }
        SyncVisuals(self);
        return r;
    }

    // ---------------------------------------------------------------- 3-5 combo (outgoing packets)
    // archer 3-arrow (x515) or 5-arrow (x555) skill
    bool IsArcherMulti(DWORD id)
    {
        DWORD c = id / 1000, s = id % 1000;
        return (s == 515 || s == 555) && (c == 107 || c == 108 || c == 207 || c == 208);
    }

    // offset of the skill id when buf is an archer 3 / 5 arrow cast/fly/effect packet and combo is on, else -1
    int ComboSkillOffset(const BYTE* buf, int len)
    {
        __try
        {
            // the genie object straight from the game (g_self is only known once its window was opened)
            BYTE* proc = *(BYTE**)0x011158FC;
            void* self = proc ? *(void**)(proc + 0x53C) : nullptr;
            if (!self) self = g_self;
            if (!self || !Flag(self, 0x4b9) || !buf || len < 6) return -1;
            int at = -1;
            if (buf[0] == 0x31) at = 2;                                                  // WIZ_MAGIC_PROCESS
            else if (buf[0] == 0x97 && len >= 8 && buf[1] == 2 && buf[2] == 4) at = 4;   // WIZ_GENIE magic
            if (at > 0 && buf[at - 1] >= 1 && buf[at - 1] <= 3 && IsArcherMulti(*(const DWORD*)(buf + at)))
                return at;
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
        return -1;
    }

    bool IsGenieSave(const BYTE* buf, int len)
    {
        __try { return buf && len >= 3 && buf[0] == 0x97 && buf[1] == 1 && buf[2] == 3; }
        __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
    }

    // TEMP transformation debug (2026-09-30): outgoing item-skill magic packets + exe callers
    void T6LogSend(const BYTE* buf, int len)
    {
        __try
        {
            if (buf && len >= 6 && buf[0] == 0x31 && *(const DWORD*)(buf + 2) >= 400000)
            {
                char hb[600]; int n = sprintf_s(hb, "T6 send len=%d:", len);
                for (int i = 0; i < len && i < 40; i++) n += sprintf_s(hb + n, sizeof(hb) - n, " %02X", buf[i]);
                void* fr[24]; USHORT c = CaptureStackBackTrace(0, 24, fr, nullptr);
                n += sprintf_s(hb + n, sizeof(hb) - n, " | callers:");
                for (USHORT i = 0; i < c && n < 560; i++)
                    if ((DWORD)fr[i] >= 0x401000 && (DWORD)fr[i] < 0x1800000) n += sprintf_s(hb + n, sizeof(hb) - n, " %08lX", (DWORD)fr[i]);
                Log(hb);
            }
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
    }

    void __fastcall HG_Send(void* sock, void*, BYTE* buf, int len)
    {
        NetTrace_Packet("send", buf, len);
        if (Reconnect_FilterSend(sock, buf, len, g_sendTramp)) return;   // soft re-connect: CharacterSelect -> RESUME
        static bool seen = false;
        if (!seen) { seen = true; Log("GENIE: send hook active (first packet)"); }
        T6LogSend(buf, len);
        // event menu: the Word Puzzle and the Coin Shop entries both send CC 01; mark the Coin Shop one
        if (len == 2 && buf && buf[0] == 0xCC && buf[1] == 0x01)
        {
            int entry = g_hgEventEntry;
            g_hgEventEntry = -1;
            if (entry == 4)
            {
                BYTE open[3] = { 0xCC, 0x01, 0x02 };
                Log("COIN: open request (CC 01 02)");
                g_sendTramp(sock, open, 3);
                return;
            }
        }
        if (IsGenieSave(buf, len))
        {
            // 0x97 01 03 + game options (padded to 180) + HopeGuard block = the server's 400-byte blob
            std::vector<BYTE> p(3 + HG_NATIVE + HG_BLOCK, 0);
            memcpy(p.data(), buf, min(len, 3 + HG_NATIVE));
            BuildBlock(p.data() + 3 + HG_NATIVE);
            g_sendTramp(sock, p.data(), (int)p.size());
            return;
        }
        int at = ComboSkillOffset(buf, len);
        std::vector<BYTE> dup;
        int copies = 0;
        if (at > 0)
        {
            // Whichever of the two skills the genie shoots, the other one goes out with it. The cast packet is
            // NOT repeated: the server refuses a second arrow cast within 500 ms ("Casting failed", which also
            // breaks the shot being made). The other skill starts at the flying packet and gets one hit packet
            // per arrow: 3 hit packets become 5 for the 5-arrow skill, 5 become 3 for the 3-arrow one.
            static int s_hits = 0;                            // hit packets of the current shot so far
            const BYTE op = buf[at - 1];
            const bool three = *(const DWORD*)(buf + at) % 1000 == 515;
            if (op == 2) { s_hits = 0; copies = 1; }
            else if (op == 3)
            {
                s_hits++;
                if (three) copies = s_hits <= 2 ? 2 : 1;
                else copies = s_hits <= 3 ? 1 : 0;
            }
            if (copies)
            {
                dup.assign(buf, buf + len);                   // copy first: Send may encrypt in place
                if (three) *(DWORD*)(dup.data() + at) += 40;  // x515 -> x555
                else *(DWORD*)(dup.data() + at) -= 40;        // x555 -> x515
            }
            static DWORD s_log = 0; static unsigned s_n = 0;
            s_n += copies;
            if (copies && GetTickCount() - s_log > 5000)
            {
                s_log = GetTickCount();
                char b[112]; sprintf_s(b, "GENIE: 3-5 combo, %d-arrow packets added to the %d-arrow shot (%u so far, op %02X)",
                    three ? 5 : 3, three ? 3 : 5, s_n, buf[0]);
                Log(b);
            }
        }
        g_sendTramp(sock, buf, len);
        for (int i = 0; i < copies; i++)
        {
            std::vector<BYTE> p(dup);
            g_sendTramp(sock, p.data(), (int)p.size());
        }
    }

    bool HookSend()
    {
        static const BYTE kOrig[10] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0xF9, 0xF6, 0xEF, 0x00 };
        BYTE* at = (BYTE*)KO_SND_FNC;
        if (memcmp(at, kOrig, 10) != 0) { Log("GENIE: send prologue differs, 3-5 combo off"); return false; }
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 32, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return false;
        memcpy(tramp, kOrig, 10);
        tramp[10] = 0xE9; *(DWORD*)(tramp + 11) = (KO_SND_FNC + 10) - (DWORD)(tramp + 15);
        g_sendTramp = (FnSend)tramp;
        DWORD old;
        VirtualProtect(at, 10, PAGE_EXECUTE_READWRITE, &old);
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)HG_Send - (KO_SND_FNC + 5);
        memset(at + 5, 0x90, 5);
        VirtualProtect(at, 10, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 10);
        Log("GENIE: send hook (3-5 combo) installed");
        return true;
    }

    bool HookRM()
    {
        DWORD* slot = (DWORD*)VT_GENIE_RM;
        if (*slot != 0x00B1CED0) { Log("GENIE: ReceiveMessage slot differs, hg_ controls off"); return false; }
        g_origRM = (FnRM)*slot;
        DWORD old;
        VirtualProtect(slot, 4, PAGE_READWRITE, &old);
        *slot = (DWORD)HG_ReceiveMessage;
        VirtualProtect(slot, 4, old, &old);
        Log("GENIE: ReceiveMessage hooked");
        return true;
    }

    bool PatchRel(DWORD at, BYTE op, const BYTE* orig, int n, void* target, const char* name)
    {
        char b[160];
        BYTE code[5] = { op };
        *(DWORD*)(code + 1) = (DWORD)target - (at + 5);
        if (memcmp((void*)at, code, 5) == 0) return true;
        if (memcmp((void*)at, orig, n) != 0)
        {
            sprintf_s(b, "GENIE: %s @%08lX bytes differ, skipped", name, at); Log(b);
            return false;
        }
        DWORD old;
        if (!VirtualProtect((void*)at, 5, PAGE_EXECUTE_READWRITE, &old)) return false;
        memcpy((void*)at, code, 5);
        VirtualProtect((void*)at, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), (void*)at, 5);
        sprintf_s(b, "GENIE: %s @%08lX patched", name, at); Log(b);
        return true;
    }

    DWORD WINAPI Apply(LPVOID)
    {
        const BYTE* recv = (const BYTE*)0x0084D0B0;
        static const BYTE kRecvPrologue[5] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF };
        for (int i = 0; i < 1200 && memcmp(recv, kRecvPrologue, 5) != 0 && recv[0] != 0xE9; i++) Sleep(100);

        static const BYTE kExplain[5] = { 0x56, 0x8B, 0xF1, 0x57, 0x8B };
        static const BYTE kCallAtt[5] = { 0xE8, 0x91, 0x68, 0xFF, 0xFF };
        static const BYTE kCallEtc[5] = { 0xE8, 0xFA, 0x65, 0xFF, 0xFF };
        // the three must go in together: a half-patched window would show empty tabs
        if (memcmp((void*)0x00B1E950, kExplain, 5) != 0 && *(BYTE*)0x00B1E950 != 0xE9) { Log("GENIE: client differs, HopeGuard tabs off"); return 0; }
        if (memcmp((void*)0x00B1D08A, kCallAtt, 5) != 0 && *(BYTE*)0x00B1D08A != 0xE8) return 0;
        PatchRel(0x00B1D08A, 0xE8, kCallAtt, 5, (void*)HG_RefreshMain, "Main tab refresh");
        PatchRel(0x00B1D321, 0xE8, kCallEtc, 5, (void*)HG_RefreshMisc, "Misc tab refresh");
        PatchRel(0x00B1E950, 0xE9, kExplain, 5, (void*)HG_ShowMain, "open on Main");
        HookRM();
        HookSend();
        PatchGoRange();
    PatchComboWalkUp();
        PatchSkillPlacement();
        PatchSlotHitTest();
        PatchEventMenu();
        return 0;
    }
}

static volatile bool g_applyLoaded = false;   // a load packet arrived: push the values into the controls
static volatile DWORD g_leaderTarget = 0;     // Party Leader Target: npc id sent by the server

// Go To Skill Range: the genie's "close enough to the target" check (0x86BE99 in the approach routine
// 0x86B8C0: threshold += xmm2; comiss threshold, distance; jb keep-moving) gets a minimum of 14 m.
// 3-5 Combo (genie main +0x4B9 on): the threshold is capped at target radius (xmm2) + 1.5 m, so the archer
// runs up to the monster before the 3 / 5 arrow shots (read straight from the genie, window open or not).
extern "C" BYTE g_hgGoRange = 0;
extern "C" float g_hgSkillRange = 14.0f;
extern "C" float g_hgMeleeExtra = 1.5f;
extern "C" float g_hgXmmSave = 0.0f;
extern "C" volatile DWORD g_hgCaveHits = 0, g_hgCaveCombo = 0, g_hgCaveMove = 0;   // approach checks / capped by combo / "keep moving"
__declspec(naked) static void GoRangeCave()
{
    __asm {
        inc     dword ptr [g_hgCaveHits]
        addss   xmm1, xmm2
        cmp     byte ptr [g_hgGoRange], 0
        je      combo
        maxss   xmm1, dword ptr [g_hgSkillRange]
    combo:
        push    eax
        mov     eax, dword ptr ds:[0x011158FC]
        test    eax, eax
        jz      no_combo
        mov     eax, dword ptr [eax + 0x53C]
        test    eax, eax
        jz      no_combo
        cmp     byte ptr [eax + 0x4B9], 0
        je      no_combo
        inc     dword ptr [g_hgCaveCombo]
        movss   dword ptr [g_hgXmmSave], xmm5
        movss   xmm5, xmm2
        addss   xmm5, dword ptr [g_hgMeleeExtra]
        minss   xmm1, xmm5
        movss   xmm5, dword ptr [g_hgXmmSave]
    no_combo:
        pop     eax
    check:
        comiss  xmm1, xmm4
        jb      keep_moving
        push    0x0086BEA6
        ret
    keep_moving:
        inc     dword ptr [g_hgCaveMove]
        push    0x0086BA40
        ret
    }
}

// 3-5 Combo walk-up: the genie fires its attack skills from 0x86DEF0 (called once, at 0x86EEEF, before the
// basic attack). Arrow skills reach far, so the archer never closed in. With 3-5 Combo and Go To Monster on,
// the skills are held back until the character stands on the monster (within g_hgComboReach of its centre,
// measured on the ground). If the character does not get closer for 4 s the skills go through again, so a
// monster that cannot be reached is still attacked.
static const float g_hgComboReach = 1.0f;   // fire from here in
static const float g_hgComboStop = 0.5f;    // walk target: this far from the centre (the shot needs a direction)
typedef void(__thiscall* FnAttackSkills)(void*, BYTE*);
static const FnAttackSkills GenieAttackSkills = (FnAttackSkills)0x0086DEF0;

// Makes the character run to a point, the way the genie's own "back to starting point" does it (0x86F4B1..):
// normalize(dir) 0x4F0D20 (ecx = vec3), yaw = 0x55EE90(xmm0 = dir.x, xmm1 = dir.z), turn 0x90A240(ecx = player,
// xmm1 = yaw, 1), destination 0x91DBD0(ecx = player, vec3 by value), then the genie's "moving" flags, which
// make its tick (0x86F5B0 tail) send the move packets.
static void GenieWalkTo(BYTE* ai, BYTE* me, float x, float y, float z)
{
    float dir[3] = { x - *(float*)(me + 972), 0.0f, z - *(float*)(me + 980) };
    float yaw = 0.0f;
    __asm {
        lea     ecx, dir
        mov     eax, 0x004F0D20
        call    eax
        movss   xmm0, dword ptr [dir]
        movss   xmm1, dword ptr [dir + 8]
        mov     eax, 0x0055EE90
        call    eax
        movss   dword ptr [yaw], xmm0
        push    1
        movss   xmm1, dword ptr [yaw]
        mov     ecx, me
        mov     eax, 0x0090A240
        call    eax
        sub     esp, 12
        mov     edx, esp
        mov     eax, x
        mov     dword ptr [edx], eax
        mov     eax, y
        mov     dword ptr [edx + 4], eax
        mov     eax, z
        mov     dword ptr [edx + 8], eax
        mov     ecx, me
        mov     eax, 0x0091DBD0
        call    eax
    }
    ai[0x1C0] = 1;
    if (!ai[0x1C1]) ai[0x1C1] = 1;
}
static void __fastcall HG_AttackSkills(void* ai, void*, BYTE* target)
{
    // (ai = the genie logic object: +0x1C0 / +0x1C1 are its "moving" flags)
    static BYTE* s_target = nullptr; static DWORD s_since = 0, s_seen = 0, s_log = 0; static float s_from = 0; static bool s_giveUp = false;
    BYTE* me = *(BYTE**)0x01115834;
    BYTE* proc = *(BYTE**)0x011158FC;
    BYTE* genie = proc ? *(BYTE**)(proc + 0x53C) : nullptr;
    if (me && target && genie && genie[0x4B9] && genie[0x495])
    {
        float dx = *(float*)(target + 972) - *(float*)(me + 972), dz = *(float*)(target + 980) - *(float*)(me + 980);
        float dist = sqrtf(dx * dx + dz * dz);
        float reach = g_hgComboReach;
        DWORD now = GetTickCount();
        if (dist > reach)
        {
            if (target != s_target || now - s_seen > 1500) { s_target = target; s_since = now; s_from = dist; s_giveUp = false; }
            s_seen = now;
            if (!s_giveUp && now - s_since > 4000)
            {
                if (dist > s_from - 1.0f) s_giveUp = true;      // not walking up: stop holding the skills
                else { s_since = now; s_from = dist; }
            }
            if (now - s_log > 2000)
            {
                s_log = now;
                char b[200]; sprintf_s(b, "GENIE: combo walk-up dist=%.1f reach=%.1f %s (approach checks=%u combo=%u moving=%u)",
                    dist, reach, s_giveUp ? "NOT closing in -> skills released" : "holding skills",
                    (unsigned)g_hgCaveHits, (unsigned)g_hgCaveCombo, (unsigned)g_hgCaveMove);
                Log(b);
            }
            if (!s_giveUp)
            {
                // run into the monster: stop just short of its centre, on our side of it
                static DWORD s_walk = 0;
                if (now - s_walk > 300 && dist > 0.1f)
                {
                    s_walk = now;
                    float k = (dist - g_hgComboStop) / dist;
                    GenieWalkTo((BYTE*)ai, me, *(float*)(me + 972) + dx * k, *(float*)(target + 976), *(float*)(me + 980) + dz * k);
                }
                return;
            }
        }
    }
    GenieAttackSkills(ai, target);
}

static void PatchComboWalkUp()
{
    static const BYTE kOrig[5] = { 0xE8, 0xFC, 0xEF, 0xFF, 0xFF };      // call 0x86DEF0
    BYTE* at = (BYTE*)0x0086EEEF;
    if (memcmp(at, kOrig, 5) != 0) { Log("GENIE: attack skill call differs, combo walk-up off"); return; }
    DWORD old;
    VirtualProtect(at, 5, PAGE_EXECUTE_READWRITE, &old);
    *(DWORD*)(at + 1) = (DWORD)HG_AttackSkills - (0x0086EEEF + 5);
    VirtualProtect(at, 5, old, &old);
    FlushInstructionCache(GetCurrentProcess(), at, 5);
    Log("GENIE: combo walk-up patch installed");
}

static void PatchGoRange()
{
    static const BYTE kOrig[13] = { 0xF3, 0x0F, 0x58, 0xCA, 0x0F, 0x2F, 0xCC, 0x0F, 0x82, 0x9A, 0xFB, 0xFF, 0xFF };
    BYTE* at = (BYTE*)0x0086BE99;
    if (memcmp(at, kOrig, 13) != 0) { Log("GENIE: approach check differs, go to skill range off"); return; }
    DWORD old;
    VirtualProtect(at, 13, PAGE_EXECUTE_READWRITE, &old);
    at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)GoRangeCave - (0x0086BE99 + 5);
    memset(at + 5, 0x90, 8);
    VirtualProtect(at, 13, old, &old);
    FlushInstructionCache(GetCurrentProcess(), at, 13);
    Log("GENIE: go to skill range patch installed");
}

// ---------------------------------------------------------------- skill placement by skill type
// The game decides where a skill may go from the "current tab" (0xB1AFD0: first visible base of
// attack / recovery / assist) and the attack/recovery page (+0x490). With Main showing attack + assist
// that is always tab 1, so support skills and potions were refused ("This skill can be used on ...").
// Here the tab / page come from the skill itself (or from the slot it is dropped on):
//   type (0x86C4B0): 1,3,9 recovery -> tab 1 page 0 (slots 8..15) | 5,6 potion -> tab 2 (16..18)
//                    2,4 support -> tab 3 (20..31) | 8 not usable | other attack -> tab 1 page 1 (0..7)
// A skill dropped on the wrong row goes to the first free slot of its own row instead of a warning.
typedef int(__fastcall* FnSkillType)(int id, int flag);
typedef bool(__thiscall* FnCanPlace)(void*, int slot, void* skill, int a3, int a4);
typedef bool(__thiscall* FnFindFree)(void*, int* slot, void* skill, int flag);
typedef void(__thiscall* FnPlace)(void*, int* slot, void* skill, int a3);
typedef int(__thiscall* FnTab)(void*);
static int g_tabOverride = 0;
static FnCanPlace g_canTramp = nullptr;   // entry detours: every caller (skill window, inventory, drag & drop)
static FnFindFree g_findTramp = nullptr;  // goes through the hooks below

static int SkillTypeOf(void* skill)
{
    if (!skill) return 0;
    return ((FnSkillType)0x0086C4B0)(*(int*)skill, 0);
}
// wanted tab and page (1 attack, 0 recovery, -1 n/a) for a skill type
static int TabForType(int t, int* page, bool fromItem)
{
    *page = -1;
    // 1,3,9 from an item (inventory: HP potions etc.) = potion slot; from the skill window = recovery skill
    if ((t == 1 || t == 3 || t == 9) && fromItem) return 2;
    if (t == 1 || t == 3 || t == 9) { *page = 0; return 1; }
    if (t == 2 || t == 4) return 3;          // support (message 31048 "Support tab")
    if (t == 5 || t == 6) return 2;          // potions (message 31047 "Recover tab")
    if (t == 8) return 0;
    *page = 1; return 1;
}
static int __fastcall HG_GetTab(void* self, void*)
{
    return g_tabOverride ? g_tabOverride : ((FnTab)0x00B1AFD0)(self);
}
static bool __fastcall HG_FindFree(void* self, void*, int* slot, void* skill, int flag)
{
    int page, tab = TabForType(SkillTypeOf(skill), &page, flag != 0);
    if (tab) { g_tabOverride = tab; if (page >= 0) Flag(self, 0x490) = (BYTE)page; }
    bool r = g_findTramp(self, slot, skill, flag);
    g_tabOverride = 0;
    char b[128]; sprintf_s(b, "GENIE place: find skill=%d type=%d tab=%d page=%d -> %d slot=%d",
        skill ? *(int*)skill : 0, SkillTypeOf(skill), tab, page, (int)r, slot ? *slot : -1); Log(b);
    return r;
}
static bool __fastcall HG_CanPlace(void* self, void*, int slot, void* skill, int a3, int a4)
{
    int page, tab = TabForType(SkillTypeOf(skill), &page, a3 != 0);
    bool rowFits = (tab == 1 && page == 1 && slot >= 0 && slot < 8) || (tab == 1 && page == 0 && slot >= 8 && slot < 16)
                || (tab == 2 && slot >= 16 && slot <= 18) || (tab == 3 && slot >= 20 && slot < 32);
    if (tab && !rowFits && slot >= 0 && slot < 32)
    {
        // dropped on another row: put it into its own row, the caller then cancels its own placement
        { char b[96]; sprintf_s(b, "GENIE place: redirect skill=%d type=%d from slot %d", skill ? *(int*)skill : 0, SkillTypeOf(skill), slot); Log(b); }
        int free = -1;
        if (HG_FindFree(self, nullptr, &free, skill, a3) && free >= 0)
        {
            ((FnPlace)0x00B129A0)(self, &free, skill, 0);
            ((Fn0)0x00B13D20)(self);
        }
        return false;
    }
    if (tab) { g_tabOverride = tab; if (page >= 0) Flag(self, 0x490) = (BYTE)page; }
    bool r = g_canTramp(self, slot, skill, a3, a4);
    g_tabOverride = 0;
    char b[128]; sprintf_s(b, "GENIE place: can skill=%d type=%d slot=%d tab=%d page=%d a3=%d -> %d",
        skill ? *(int*)skill : 0, SkillTypeOf(skill), slot, tab, page, a3, (int)r); Log(b);
    return r;
}

// 5-byte jmp detour at a function entry; the stolen bytes must be position independent
static void* DetourEntry(DWORD at, const BYTE* orig, int n, void* hook)
{
    BYTE* p = (BYTE*)at;
    if (memcmp(p, orig, n) != 0) return nullptr;
    BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 32, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    if (!tramp) return nullptr;
    memcpy(tramp, orig, n);
    tramp[n] = 0xE9; *(DWORD*)(tramp + n + 1) = (at + n) - (DWORD)(tramp + n + 5);
    DWORD old;
    VirtualProtect(p, n, PAGE_EXECUTE_READWRITE, &old);
    p[0] = 0xE9; *(DWORD*)(p + 1) = (DWORD)hook - (at + 5);
    memset(p + 5, 0x90, n - 5);
    VirtualProtect(p, n, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, n);
    return tramp;
}

// Icon pick-up (drag out = remove, drag to move): 0xB1BB30 / 0xB1BD10 find the icon under the mouse
// only among the slots of the "current tab" (tab 1: page +0x490 ? 0..7 : 8..15, tab 2: 16..18, tab 3: 20..31).
// Main shows attack + recovery + support at once, so support icons could never be picked up.
// On Main both are tried for attack, recovery and support in turn; Misc keeps the game's own tab.
typedef bool(__thiscall* FnHit)(void*);
static FnHit g_hitTrampA = nullptr, g_hitTrampB = nullptr;
static bool HitAllRows(FnHit orig, void* self)
{
    if (!orig) return false;
    if (g_misc) return orig(self);
    BYTE& page = *((BYTE*)self + 0x490);
    const BYTE savedPage = page;
    const struct { int tab; BYTE page; } tries[] = { { 1, 1 }, { 1, 0 }, { 3, 0 } };
    bool hit = false;
    for (const auto& t : tries)
    {
        g_tabOverride = t.tab; page = t.page;
        hit = orig(self);
        if (hit) break;
    }
    g_tabOverride = 0;
    page = savedPage;                   // the picked slot index is absolute (0..31), the page is UI state
    return hit;
}
static bool __fastcall HG_HitA(void* self, void*) { return HitAllRows(g_hitTrampA, self); }
static bool __fastcall HG_HitB(void* self, void*) { return HitAllRows(g_hitTrampB, self); }

static void PatchSkillPlacement()
{
    // CanPlace 0xB1B160: push ebx; mov ebx, esp; sub esp, 8        FindFree 0xB1B630: push ebp; mov ebp, esp; push -1; push imm32
    static const BYTE kCan[6] = { 0x53, 0x8B, 0xDC, 0x83, 0xEC, 0x08 };
    static const BYTE kFind[10] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0x77, 0x5E, 0xF4, 0x00 };
    g_canTramp = (FnCanPlace)DetourEntry(0x00B1B160, kCan, 6, (void*)HG_CanPlace);
    g_findTramp = (FnFindFree)DetourEntry(0x00B1B630, kFind, 10, (void*)HG_FindFree);
    int n = 0;
    static const BYTE kHit[6] = { 0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8 };   // push ebp; mov ebp, esp; and esp, -8
    g_hitTrampA = (FnHit)DetourEntry(0x00B1BB30, kHit, 6, (void*)HG_HitA);
    // 0x00B1BD10 (slot under the mouse) returns the slot index and belongs to HG_SlotAt (PatchSlotHitTest); a second
    // detour here made that one fail ("slot under mouse differs") -> skills could not be dragged onto the genie rows.
    g_hitTrampB = nullptr;
    for (DWORD at : { 0x00B1B1E4u, 0x00B1B6A6u, 0x00B1BB4Du, 0x00B1BD2Du })   // "current tab" in placement + pick-up
    {
        BYTE* p = (BYTE*)at;
        if (p[0] != 0xE8 || at + 5 + *(int*)(p + 1) != 0x00B1AFD0) continue;
        DWORD old;
        VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old);
        *(DWORD*)(p + 1) = (DWORD)HG_GetTab - (at + 5);
        VirtualProtect(p, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), p, 5);
        n++;
    }
    char b[96]; sprintf_s(b, "GENIE: skill placement by type: can=%d find=%d hit=%d/%d tab sites=%d/4", g_canTramp != nullptr, g_findTramp != nullptr, g_hitTrampA != nullptr, g_hitTrampB != nullptr, n); Log(b);
    if (!g_canTramp) g_canTramp = (FnCanPlace)0x00B1B160;       // never call through a null pointer
    if (!g_findTramp) g_findTramp = (FnFindFree)0x00B1B630;
}

// ---------------------------------------------------------------- slot under the mouse
// 0xB1BD10 only searches the slots of the current native tab (Main counts as tab 1 = attack or recovery
// page), so the Support row (20..27) could not be picked up, dragged out or dropped on. On Main every
// visible row is searched; area rects via 0xB1B050(self, -, index, -), mouse at [0x111591C]+0x1C/+0x20.
typedef void*(__thiscall* FnSlotElem)(void*, int, int, int);
static FnTab g_slotAtTramp = nullptr;
static int __fastcall HG_SlotAt(void* self, void*)
{
    if (g_misc || ((FnTab)0x00B1AFD0)(self) != 1) return g_slotAtTramp(self);
    __try
    {
        const BYTE* mouse = *(const BYTE**)0x0111591C;
        int mx = *(const int*)(mouse + 0x1C), my = *(const int*)(mouse + 0x20);
        for (int i = 0; i < 28; i++)
        {
            if (i >= 16 && i < 20) continue;                        // potions live on Misc
            const BYTE* e = (const BYTE*)((FnSlotElem)0x00B1B050)(self, 0, i, 0);
            if (!e) continue;
            const float* r = (const float*)(e + 0xc8);
            if (mx >= (int)r[0] && mx <= (int)r[2] && my >= (int)r[1] && my <= (int)r[3]) return i;
        }
    }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
    return -1;
}

static void PatchSlotHitTest()
{
    // push ebp; mov ebp, esp; and esp, -8
    static const BYTE kSlotAt[6] = { 0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8 };
    g_slotAtTramp = (FnTab)DetourEntry(0x00B1BD10, kSlotAt, 6, (void*)HG_SlotAt);
    Log(g_slotAtTramp ? "GENIE: slot under mouse hooked (all Main rows)" : "GENIE: slot under mouse differs, not hooked");
}

// ---------------------------------------------------------------- event menu (9C F0): Coin Shop vs Word Puzzle
// sub_7D7A60(entry) runs the picked menu entry; entries 3 (Word Puzzle) and 4 (Coin Shop) both send CC 01.
// Remember the entry (first stack argument) so HG_Send can turn the Coin Shop request into CC 01 02.
// Entry 0 (server menu type 0, the client's 'Board' entry, relabelled 'Manner Store') opens the Manner Store
// instead: the original is skipped (thiscall, 1 stack argument -> ret 4).
static void* g_eventMenuTramp = nullptr;
void MannerStore_Open();
static void DailyQuest_Open()
{
    BYTE packet[2] = { 0x9C, 0x00 };
    void* socket = *(void**)0x01115914;
    if (socket) ((void(__thiscall*)(void*, BYTE*, int))0x00704070)(socket, packet, 2);
    Log("DAILY: event hub selection sent");
}
__declspec(naked) static void HG_EventMenu()
{
    __asm
    {
        cmp     dword ptr [esp + 4], 0
        je      manner
        push    eax
        mov     eax, dword ptr [esp + 8]
        mov     dword ptr [g_hgEventEntry], eax
        pop     eax
        jmp     dword ptr [g_eventMenuTramp]
    manner:
        pushad
        call    DailyQuest_Open
        popad
        ret     4
    }
}

static void PatchEventMenu()
{
    // push ebx; mov ebx, esp; sub esp, 8
    static const BYTE kMenu[6] = { 0x53, 0x8B, 0xDC, 0x83, 0xEC, 0x08 };
    g_eventMenuTramp = DetourEntry(0x007D7A60, kMenu, 6, (void*)HG_EventMenu);
    Log(g_eventMenuTramp ? "COIN: event menu hooked" : "COIN: event menu differs, not hooked");
}

static void LeaderTargetTick()
{
    DWORD id = g_leaderTarget;
    if (!id || !g_set.leader) return;
    BYTE* me = *(BYTE**)0x01115834;
    void* gm = GameMain();
    if (!me || !gm) return;
    if (*(DWORD*)(me + 0x660) == id) { g_leaderTarget = 0; return; }
    void* ch = ((void*(__stdcall*)(int, int))0x00798DC0)((int)id, 1);      // character by id (alive)
    if (!ch) return;
    ((void(__thiscall*)(void*, void*))0x00812440)(gm, ch);                // same as clicking it
    g_leaderTarget = 0;
}

// party heal slider (a copy of the self heal one): trackbar at +0x110, min +0x118, max +0x11c, pos +0x120
static void* PartyHealTrack(void* self)
{
    void* sb = FindIn(self, "hg_party_heal");
    return sb ? Ptr(sb, 0x110) : nullptr;
}

// game thread (subclassed window proc)
static void GenieTickBody(void* self)
{
    void* assist = Ptr(self, 0x20c);
    void* edit = Find(assist, "hg_pt_code");
    void* editStr = Find(edit, "hg_pt_code_str");
    BYTE* tb = (BYTE*)PartyHealTrack(self);

    if (g_applyLoaded)
    {
        g_applyLoaded = false;
        SetText(editStr, g_set.pt);
        if (tb)
        {
            *(int*)(tb + 0x118) = 0; *(int*)(tb + 0x11c) = 100; *(int*)(tb + 0x120) = g_set.partyHeal;
            ((void(__thiscall*)(void*))0x006F5380)(tb);
        }
        SyncVisuals(self);
    }

    // PT code field: hint only while empty; the code follows the field
    std::string code = TextOf(editStr);
    if (!g_misc)
        if (void* hint = Find(assist, "hg_pt_hint")) SetVisible(hint, code.empty());
    if (g_loaded && code != g_set.pt)
    {
        EnterCriticalSection(&g_setLock); g_set.pt = code.substr(0, 20); LeaveCriticalSection(&g_setLock);
        MarkDirty();
    }

    // party heal %
    if (tb)
    {
        if (*(int*)(tb + 0x11c) != 100)
        {
            *(int*)(tb + 0x118) = 0; *(int*)(tb + 0x11c) = 100; *(int*)(tb + 0x120) = g_set.partyHeal;
            ((void(__thiscall*)(void*))0x006F5380)(tb);
        }
        int v = *(int*)(tb + 0x120);
        if (v >= 0 && v <= 100 && v != g_set.partyHeal && g_loaded) { g_set.partyHeal = v; MarkDirty(); }
        static int shown = -1;
        if (shown != g_set.partyHeal)
        {
            shown = g_set.partyHeal;
            char b[16]; sprintf_s(b, "%d%%", shown);
            SetText(FindIn(self, "hg_party_heal_str"), b);
        }
    }

    g_hgGoRange = g_set.goRange ? 1 : 0;
    if (!g_misc) Icons(self, true);          // the style buttons hide one skill row; keep both
    LeaderTargetTick();

    // save once the user stops changing things (the game's own genie save carries our block)
    if (g_dirty && g_loaded && GetTickCount() - g_dirtyAt > 1500)
    {
        g_dirty = false;
        GenieSaveOptions(self);
    }
}

// recv hook (game thread): 0x97 01 02 = genie options loaded from the server
void Genie_OnRecv(const BYTE* pkt, int len)
{
    if (len >= 7 && pkt[0] == WIZ_HSACS_HOOK && pkt[1] == HG_SUBOP && pkt[2] == 2)
    {
        g_leaderTarget = *(const DWORD*)(pkt + 3);           // leader's monster, selected on the game thread
        return;
    }
    // HopeGuard block, sent by the server right after the game's own genie options (which stay 100 bytes:
    // the client copies them into a fixed buffer)
    if (len >= 3 + HG_BLOCK && pkt[0] == WIZ_HSACS_HOOK && pkt[1] == HG_SUBOP && pkt[2] == 4)
    {
        ParseBlock(pkt + 3);
        g_applyLoaded = true;
        Log("GENIE: options loaded");
    }
}

// Attack and Recovery skills have their own rows now, but the game's drop / hit test only looks at the
// "page" in +0x490 (1 = attack slots 1..8, 0 = recovery slots 9..16): follow the mouse so a skill lands
// in the row it is dropped on. Area rects (+0xc8 l, +0xcc t, +0xd0 r, +0xd4 b) are in window coordinates.
static void HoverPage(void* self, HWND h)
{
    if (g_misc || !h) return;
    void* a1 = FindIn(self, "1");
    void* a9 = FindIn(self, "9");
    if (!a1 || !a9) return;
    POINT pt; GetCursorPos(&pt); ScreenToClient(h, &pt);
    const float* r1 = (const float*)((BYTE*)a1 + 0xc8);
    const float* r9 = (const float*)((BYTE*)a9 + 0xc8);
    if (pt.x < r1[0] - 60 || pt.x > r1[0] + 400) return;               // not over the slot rows
    if (pt.y >= r1[1] - 4 && pt.y <= r1[3] + 4) Flag(self, 0x490) = 1;
    else if (pt.y >= r9[1] - 4 && pt.y <= r9[3] + 4) Flag(self, 0x490) = 0;
}

static void HoverPageSafe(HWND h)
{
    void* self = g_self;
    if (!self) return;
    __try { HoverPage(self, h); }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
}

// click anywhere outside the PT code field: leave the field (the game keeps an edit focused until Enter).
// 0x6E0B10 = CN3UIEdit::KillFocus (the game calls it on the genie's own edits when switching tabs).
static void PtFieldClick(void* self, int x, int y)
{
    void* edit = Find(Ptr(self, 0x20c), "hg_pt_code");
    if (!edit) return;
    const float* r = (const float*)((BYTE*)edit + 0xc8);
    if (x >= r[0] && x <= r[2] && y >= r[1] && y <= r[3]) return;
    ((Fn0)0x006E0B10)(edit);
}

void Genie_OnMouseDown(int x, int y)
{
    void* self = g_self;
    if (!self) return;
    __try { PtFieldClick(self, x, y); }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
}

void Genie_Tick(HWND h)
{
    HoverPageSafe(h);
    static DWORD last = 0;
    DWORD now = GetTickCount();
    if (now - last < 150) return;
    last = now;
    void* self = g_self;
    if (!self) return;
    __try { GenieTickBody(self); }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
}

void Genie_Init()
{
    InitializeCriticalSection(&g_setLock);
    CloseHandle(CreateThread(nullptr, 0, Apply, nullptr, 0, nullptr));
}
