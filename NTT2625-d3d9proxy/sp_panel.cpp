// Slave Priest panels (d3d9 proxy). Server side: GameServer SlavePriest.cpp.
// Skin: only the game's own UI atlases (build_sp_assets.py -> sp_ui\*.pus + sp_layout.h):
//   mini window  = 2369 DAT-7.uif strip (ui\re_auto_02.dxt): label + Start / Stop / Settings buttons
//   settings page = our layout of DAT-32.uif pieces (re_akara_01 frame, re_skill04 plates/buttons) + priest skill icons
// The mini window opens when the "Slave Priest" scroll (skill 495146) is used and stays while the scroll buff runs.
//
//   WIZ_HSACS_HOOK 0xE9, sub SLAVEPRIEST 0xEC
//   C->S  [op 1 start/update | 2 stop | 3 state?] [healPct][hpBuff 0/1/2][ac][heal][cure][res][party]
//   S->C  [op 1 show | 2 hide | 3 state] (op 1/3:) [active][healPct][hpBuff][ac][heal][cure][res][party][u32 secondsLeft]
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include <map>
#include "sp_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();

static const DWORD KO_SND_FNC = 0x00704070;  // unpack: thiscall(CAPISocket*, BYTE* buf, int len)
static const DWORD KO_PTR_PKT = 0x01115914;  // unpack: CAPISocket*
static const BYTE  WIZ_HSACS_HOOK = 0xE9;
static const BYTE  SP_SUBOP = 0xEC;          // HSACSXOpCodes::SLAVEPRIEST (v4 packets.h)

static const UINT WM_SP_REFRESH = WM_APP + 21;
static const UINT WM_SP_MINI    = WM_APP + 22;
static const UINT WM_SP_HIDE    = WM_APP + 23;

// ---------------------------------------------------------------- state (g_lock)
static CRITICAL_SECTION g_lock, g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQ;
static bool  g_active = false;
static BYTE  g_healPct = 60, g_hpBuff = 1, g_ac = 1, g_heal = 1, g_cure = 1, g_res = 1, g_party = 1;
static DWORD g_secondsLeft = 0, g_secondsAt = 0;

struct Win
{
    HWND hWnd = nullptr; HDC dc = nullptr; void* bits = nullptr; int w = 0, h = 0;
    bool open = false, placed = false, dragging = false, tracking = false;
    POINT dragFrom{}; int hover = 0, pressed = 0;
};
static Win g_big, g_mini;
static Win* g_cv = nullptr;      // window being painted
static HCURSOR g_gameCursor = nullptr;

static DWORD SecondsLeft()
{
    DWORD left = g_secondsLeft;
    if (left) { DWORD el = (GetTickCount() - g_secondsAt) / 1000; left = el < left ? left - el : 0; }
    return left;
}

// ---------------------------------------------------------------- send (flushed on the game thread)
static void QueueSend(const std::vector<BYTE>& p)
{
    EnterCriticalSection(&g_sendLock);
    g_sendQ.push_back(p);
    LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();
}

void Sp_FlushSendQueue()
{
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_sendLock);
    pending.swap(g_sendQ);
    LeaveCriticalSection(&g_sendLock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
        char b[64]; sprintf_s(b, "SP send: op=%u len=%u", p[2], (unsigned)p.size()); Log(b);
    }
}

static void SendSettings(BYTE op)
{
    EnterCriticalSection(&g_lock);
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, SP_SUBOP, op, g_healPct, g_hpBuff, g_ac, g_heal, g_cure, g_res, g_party };
    LeaveCriticalSection(&g_lock);
    QueueSend(p);
}

// ---------------------------------------------------------------- sprites (sp_ui\*.pus, premultiplied BGRA)
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<std::string, Sprite> g_sprites;
static char g_skinDir[MAX_PATH];

static bool LoadPus(const char* path, Sprite& s)
{
    FILE* f = nullptr;
    if (fopen_s(&f, path, "rb") != 0 || !f) return false;
    char magic[4]; UINT32 w = 0, h = 0;
    bool ok = fread(magic, 1, 4, f) == 4 && memcmp(magic, "PUSI", 4) == 0 &&
              fread(&w, 4, 1, f) == 1 && fread(&h, 4, 1, f) == 1 && w && h && w < 4096 && h < 4096;
    if (ok)
    {
        BITMAPINFO bmi = {};
        bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), (LONG)w, -(LONG)h, 1, 32, BI_RGB };
        void* bits = nullptr;
        s.dc = CreateCompatibleDC(nullptr);
        s.bmp = CreateDIBSection(s.dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        ok = s.bmp && fread(bits, 4, (size_t)w * h, f) == (size_t)w * h;
        SelectObject(s.dc, s.bmp);
        s.w = (int)w; s.h = (int)h;
    }
    fclose(f);
    return ok;
}

static void LoadSkin()
{
    GetModuleFileNameA(nullptr, g_skinDir, MAX_PATH);
    strcpy_s(strrchr(g_skinDir, '\\') + 1, 48, "HopeGuard\\sp_ui");
    int ok = 0;
    for (const SpSprite& s : SP_SPRITES)
    {
        char path[MAX_PATH]; sprintf_s(path, "%s\\%s.pus", g_skinDir, s.name);
        Sprite sp;
        if (LoadPus(path, sp)) { g_sprites[s.name] = sp; ok++; }
    }
    char msg[80]; sprintf_s(msg, "SP: skin sprites loaded %d/%d", ok, (int)(sizeof(SP_SPRITES) / sizeof(SP_SPRITES[0])));
    Log(msg);
}

static bool Blit(const char* name, int x, int y, int w = 0, int h = 0)
{
    auto it = g_sprites.find(name);
    if (it == g_sprites.end()) return false;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_cv->dc, x, y, w ? w : it->second.w, h ? h : it->second.h, it->second.dc, 0, 0, it->second.w, it->second.h, bf);
    return true;
}

// premultiplied src-over fill (fallback when a sprite is missing)
static void Fill(SpRect r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(g_cv->w, r.r), bb = min(g_cv->h, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* dst = (BYTE*)g_cv->bits + (y * g_cv->w + l) * 4;
        for (int x = l; x < rr; x++, dst += 4)
        {
            dst[0] = (BYTE)((cb * a + dst[0] * (255 - a)) / 255);
            dst[1] = (BYTE)((cg * a + dst[1] * (255 - a)) / 255);
            dst[2] = (BYTE)((cr * a + dst[2] * (255 - a)) / 255);
            dst[3] = (BYTE)(a + dst[3] * (255 - a) / 255);
        }
    }
}

// GDI text into a scratch DIB, coverage used as alpha (same as rce_store.cpp TextR)
static void Text(const std::string& s, SpRect r, COLORREF color, int pt, UINT fmt, bool bold = false)
{
    int w = r.r - r.l, h = r.b - r.t;
    if (s.empty() || w <= 0 || h <= 0) return;
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
    void* bits = nullptr;
    HDC dc = CreateCompatibleDC(nullptr);
    HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    HGDIOBJ ob = SelectObject(dc, bmp);
    HFONT font = CreateFontA(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
    HGDIOBJ of = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, RGB(255, 255, 255));
    RECT rc = { 0, 0, w, h };
    DrawTextA(dc, s.c_str(), (int)s.size(), &rc, fmt | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    GdiFlush();
    const BYTE cr = GetRValue(color), cg = GetGValue(color), cb = GetBValue(color);
    for (int y = 0; y < h; y++)
    {
        int dy = r.t + y;
        if (dy < 0 || dy >= g_cv->h) continue;
        const BYTE* src = (const BYTE*)bits + y * w * 4;
        BYTE* dst = (BYTE*)g_cv->bits + (dy * g_cv->w + r.l) * 4;
        for (int x = 0; x < w; x++, src += 4, dst += 4)
        {
            int a = max(src[0], max(src[1], src[2]));
            if (!a || r.l + x < 0 || r.l + x >= g_cv->w) continue;
            dst[0] = (BYTE)((cb * a + dst[0] * (255 - a)) / 255);
            dst[1] = (BYTE)((cg * a + dst[1] * (255 - a)) / 255);
            dst[2] = (BYTE)((cr * a + dst[2] * (255 - a)) / 255);
            dst[3] = (BYTE)(a + dst[3] * (255 - a) / 255);
        }
    }
    SelectObject(dc, of); DeleteObject(font);
    SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}

// sprite button: <prefix>_n / _d (pressed) / _o (hover) / _x (disabled)
static void SkinButton(const char* prefix, SpRect r, int ctl, bool enabled, const std::string& label = "", int pt = 8)
{
    bool hov = enabled && g_cv->hover == ctl, down = hov && g_cv->pressed == ctl;
    char name[32]; sprintf_s(name, "%s_%s", prefix, !enabled ? "x" : down ? "d" : hov ? "o" : "n");
    if (!Blit(name, r.l, r.t, r.r - r.l, r.b - r.t))
    {
        sprintf_s(name, "%s_n", prefix);
        if (!Blit(name, r.l, r.t, r.r - r.l, r.b - r.t)) Fill(r, hov ? 90 : 60, 45, 30, 255);
    }
    if (!label.empty())
        Text(label, r, enabled ? RGB(255, 240, 200) : RGB(150, 140, 120), pt, DT_CENTER | DT_VCENTER, true);
}

static void Present(Win& w)
{
    RECT wr; GetWindowRect(w.hWnd, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { w.w, w.h };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(w.hWnd, nullptr, &dst, &sz, w.dc, &src, 0, &bf, ULW_ALPHA);
}

// ---------------------------------------------------------------- settings page: DAT-32 style skill boxes
// top: wide "Healing Recovery" box (slot + icon, slider, Enable/Disable); below: 2x3 boxes
// (title plate, skill slot + icon, two stacked choice buttons). Frames = 9-slice re_akara_01 pieces.
enum Ctl { C_NONE = 0, C_CLOSE, C_HEAL_ON, C_HEAL_OFF, C_PCT_DN, C_PCT_UP, C_TRACK,
           C_BOX0_A, C_BOX0_B, C_BOX1_A, C_BOX1_B, C_BOX2_A, C_BOX2_B, C_BOX3_A, C_BOX3_B, C_BOX4_A, C_BOX4_B, C_BOX5_A, C_BOX5_B,
           C_LAST };
static const int HX = 18, HY = 92, HW = 304, HH = 112;                           // healing box
static const int GX = 18, GY = 214, GW = 147, GH = 140, GDX = 157, GDY = 148;   // grid boxes
static SpRect GridBox(int i) { int x = GX + (i % 2) * GDX, y = GY + (i / 2) * GDY; return { x, y, x + GW, y + GH }; }
static SpRect CtlBox(int c)
{
    switch (c)
    {
    case C_CLOSE:    return { SP_SET_W - 44, 18, SP_SET_W - 22, 41 };
    case C_HEAL_ON:  return { HX + 236, HY + 40, HX + 294, HY + 60 };
    case C_HEAL_OFF: return { HX + 236, HY + 64, HX + 294, HY + 84 };
    case C_PCT_DN:   return { HX + 68, HY + 50, HX + 87, HY + 69 };
    case C_PCT_UP:   return { HX + 211, HY + 50, HX + 230, HY + 69 };
    case C_TRACK:    return { HX + 90, HY + 48, HX + 208, HY + 72 };
    }
    if (c >= C_BOX0_A && c <= C_BOX5_B)
    {
        int i = (c - C_BOX0_A) / 2, second = (c - C_BOX0_A) % 2;
        SpRect b = GridBox(i);
        int bx = b.l + (GW - 80) / 2, by = b.t + (second ? 110 : 86);
        return { bx, by, bx + 80, by + 20 };
    }
    return { 0, 0, 0, 0 };
}
static int HitBig(int x, int y)
{
    for (int c = C_CLOSE; c < C_LAST; c++) { SpRect b = CtlBox(c); if (x >= b.l && x < b.r && y >= b.t && y < b.b) return c; }
    return C_NONE;
}

static void SkillBox(SpRect b)
{
    int w = b.r - b.l, h = b.b - b.t;
    Blit("box_bg", b.l + 5, b.t + 5, w - 10, h - 10);
    Blit("fr_t", b.l + 16, b.t, w - 32, 6);
    Blit("fr_b", b.l + 16, b.b - 6, w - 32, 6);
    Blit("fr_l", b.l, b.t + 16, 8, h - 32);
    Blit("fr_r", b.r - 8, b.t + 16, 8, h - 32);
    Blit("fr_tl", b.l, b.t); Blit("fr_tr", b.r - 16, b.t); Blit("fr_bl", b.l, b.b - 16); Blit("fr_br", b.r - 16, b.b - 16);
}
static void Plate(int x, int y, int w, const char* title)
{
    Blit("plate", x, y, w, 22);
    Text(title, { x, y, x + w, y + 22 }, RGB(255, 225, 150), 8, DT_CENTER | DT_VCENTER, true);
}
static void Slot(int x, int y, const char* icon)
{
    Blit("slot", x, y, 45, 42);
    if (icon) Blit(icon, x + 6, y + 5, 32, 32);
}
// choice button: selected = the DAT-32 lit gold state (on_o), not selected = dark gold (off_n / off_d on hover).
// ("on_n" is DAT-32's grey idle look, which read as "not selected" - so it is not used here.)
static void Choice(int c, const char* label, bool selected, bool enabled = true)
{
    SpRect r = CtlBox(c);
    bool hov = enabled && g_cv->hover == c;
    const char* spr = !enabled ? "off_x" : selected ? (hov ? "on_d" : "on_o") : (hov ? "off_d" : "off_n");
    if (!Blit(spr, r.l, r.t, r.r - r.l, r.b - r.t)) Fill(r, selected ? 150 : 60, selected ? 110 : 45, 30, 255);
    COLORREF col = !enabled ? RGB(150, 140, 120) : selected ? RGB(255, 255, 210) : RGB(170, 160, 135);
    Text(label, r, col, 8, DT_CENTER | DT_VCENTER, true);
}

static void PaintBig()
{
    if (!g_big.hWnd || !g_big.dc) return;
    g_cv = &g_big;
    memset(g_big.bits, 0, (size_t)g_big.w * g_big.h * 4);
    if (!Blit("frame", 0, 0, SP_SET_W, SP_SET_H)) Fill({ 0, 0, SP_SET_W, SP_SET_H }, 20, 18, 14, 245);
    Blit("title", (SP_SET_W - 173) / 2, 10, 173, 41);
    Text("Slave Priest", { (SP_SET_W - 173) / 2, 10, (SP_SET_W + 173) / 2, 51 }, RGB(255, 225, 150), 10, DT_CENTER | DT_VCENTER, true);
    SkinButton("close", CtlBox(C_CLOSE), C_CLOSE, true);

    EnterCriticalSection(&g_lock);
    DWORD left = SecondsLeft();
    char st[96];
    if (g_active) sprintf_s(st, "Priest is with you  -  %lu:%02lu left", left / 60, left % 60);
    else if (left) sprintf_s(st, "Priest is resting  -  scroll %lu:%02lu", left / 60, left % 60);
    else sprintf_s(st, "Use a Slave Priest scroll");
    Fill({ 24, 62, SP_SET_W - 24, 82 }, 0, 0, 0, 150);	// readable over the header art
    Text(st, { 20, 62, SP_SET_W - 20, 82 }, g_active ? RGB(140, 235, 140) : RGB(225, 205, 150), 8, DT_CENTER | DT_VCENTER, true);

    // healing box
    SkillBox({ HX, HY, HX + HW, HY + HH });
    Plate(HX + (HW - 160) / 2, HY + 8, 160, "Healing Recovery");
    Slot(HX + 14, HY + 40, "ico_heal");
    SkinButton("left", CtlBox(C_PCT_DN), C_PCT_DN, true);
    SkinButton("right", CtlBox(C_PCT_UP), C_PCT_UP, true);
    Blit("track", HX + 90, HY + 53, 118, 14);
    int thumbX = HX + 90 + (int)((g_healPct - 5) * (118 - 9) / 90);
    Blit("thumb", thumbX, HY + 52, 9, 16);
    char hp[40]; sprintf_s(hp, "Heal below %d%% HP", g_healPct);
    Text(hp, { HX + 70, HY + 72, HX + 230, HY + 90 }, RGB(255, 236, 170), 8, DT_CENTER | DT_VCENTER, true);
    Choice(C_HEAL_ON, "Enable", g_heal != 0);
    Choice(C_HEAL_OFF, "Disable", g_heal == 0);

    // skill boxes
    struct BoxDef { const char* title; const char* icon; const char* a; const char* b; bool selA, selB, enA, enB; };
    BoxDef boxes[6] = {
        { "HP Buff", g_hpBuff == 2 ? "ico_undying" : "ico_superioris", "Superioris", "Undying", g_hpBuff == 1, g_hpBuff == 2, true, true },
        { "AC Buff", "ico_ac", "Enable", "Disable", g_ac != 0, g_ac == 0, true, true },
        { "Cure", "ico_cure", "Enable", "Disable", g_cure != 0, g_cure == 0, true, true },
        { "Resurrect", "ico_res", "Enable", "Disable", g_res != 0, g_res == 0, true, true },
        { "Party Support", "ico_group", "Enable", "Disable", g_party != 0, g_party == 0, true, true },
        { "Priest", "ico_heal", g_active ? "Apply" : "Start", "Stop", true, g_active, true, g_active },
    };
    for (int i = 0; i < 6; i++)
    {
        SpRect b = GridBox(i);
        SkillBox(b);
        Plate(b.l + (GW - 124) / 2, b.t + 8, 124, boxes[i].title);
        Slot(b.l + (GW - 45) / 2, b.t + 36, boxes[i].icon);
        Choice(C_BOX0_A + i * 2, boxes[i].a, boxes[i].selA, boxes[i].enA);
        Choice(C_BOX0_B + i * 2, boxes[i].b, boxes[i].selB, boxes[i].enB);
    }
    LeaveCriticalSection(&g_lock);
    Present(g_big);
}

// ---------------------------------------------------------------- mini window (DAT-7)
enum MCtl { M_NONE = 0, M_START, M_STOP, M_OPEN };
static SpRect MiniBox(int c)
{
    if (c == M_START) return SP_MINI_START;
    if (c == M_STOP)  return SP_MINI_STOP;
    if (c == M_OPEN)  return SP_MINI_OPEN;
    return { 0, 0, 0, 0 };
}
static int HitMini(int x, int y)
{
    for (int c = M_START; c <= M_OPEN; c++) { SpRect b = MiniBox(c); if (x >= b.l && x < b.r && y >= b.t && y < b.b) return c; }
    return M_NONE;
}
static void PaintMini()
{
    if (!g_mini.hWnd || !g_mini.dc) return;
    g_cv = &g_mini;
    memset(g_mini.bits, 0, (size_t)g_mini.w * g_mini.h * 4);
    if (!Blit("mini_bg", 0, 0, SP_MINI_W, SP_MINI_H)) Fill({ 0, 0, SP_MINI_W, SP_MINI_H }, 30, 26, 18, 245);
    EnterCriticalSection(&g_lock);
    DWORD left = SecondsLeft();
    bool active = g_active;
    LeaveCriticalSection(&g_lock);
    char t[32]; sprintf_s(t, "Priest %lu:%02lu", left / 60, left % 60);
    Text(t, SP_MINI_LABEL, active ? RGB(150, 240, 150) : RGB(235, 220, 180), 7, DT_LEFT | DT_VCENTER, true);
    SkinButton("mini_start", SP_MINI_START, M_START, !active);
    SkinButton("mini_stop", SP_MINI_STOP, M_STOP, active);
    SkinButton("mini_open", SP_MINI_OPEN, M_OPEN, true);
    Present(g_mini);
}

// ---------------------------------------------------------------- window plumbing
static HWND FindGameWindow()
{
    struct Ctx { DWORD pid; HWND best; int area; } ctx = { GetCurrentProcessId(), nullptr, 0 };
    EnumWindows([](HWND h, LPARAM lp) -> BOOL {
        Ctx* c = (Ctx*)lp;
        DWORD pid; GetWindowThreadProcessId(h, &pid);
        if (pid != c->pid || !IsWindowVisible(h) || h == g_big.hWnd || h == g_mini.hWnd) return TRUE;
        RECT r; GetWindowRect(h, &r);
        int a = (r.right - r.left) * (r.bottom - r.top);
        if (a > c->area) { c->area = a; c->best = h; }
        return TRUE;
    }, (LPARAM)&ctx);
    return ctx.best;
}

static void PaintWin(Win& w) { if (&w == &g_big) PaintBig(); else PaintMini(); }

static void ShowWin(Win& w, bool center)
{
    w.open = true;
    if (!w.placed)
    {
        w.placed = true;
        RECT gr = { 0, 0, w.w, w.h };
        if (HWND game = FindGameWindow()) GetWindowRect(game, &gr);
        int x = center ? gr.left + max(0, (int)(gr.right - gr.left - w.w) / 2) : gr.left + 320;
        int y = center ? gr.top + max(0, (int)(gr.bottom - gr.top - w.h) / 2) : gr.top + 60;
        SetWindowPos(w.hWnd, HWND_TOPMOST, x, y, w.w, w.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    else SetWindowPos(w.hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    PaintWin(w);
}

static void HideWin(Win& w)
{
    w.open = false;
    ShowWindow(w.hWnd, SW_HIDE);
    if (HWND game = FindGameWindow()) SetForegroundWindow(game);
}

static void SyncWin(Win& w, HWND game, bool fg)
{
    bool want = w.open && game && !IsIconic(game) && fg;
    if (want != (IsWindowVisible(w.hWnd) != FALSE))
    {
        if (want) SetWindowPos(w.hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        else ShowWindow(w.hWnd, SW_HIDE);
    }
    if (want) PaintWin(w);	// countdown
}

static void SyncVisibility()
{
    HWND game = FindGameWindow();
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool fg = pid == GetCurrentProcessId();
    SyncWin(g_mini, game, fg);
    SyncWin(g_big, game, fg);
    POINT p; CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) && (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursor = ci.hCursor;
}

static void ActivateBig(int c)
{
    if (c == C_CLOSE) { HideWin(g_big); return; }
    int send = 0;	// 1 = settings (start/update), 2 = stop
    EnterCriticalSection(&g_lock);
    bool active = g_active;
    bool changed = true;
    switch (c)
    {
    case C_HEAL_ON:  g_heal = 1; break;
    case C_HEAL_OFF: g_heal = 0; break;
    case C_PCT_DN:   g_healPct = g_healPct >= 10 ? g_healPct - 5 : 5; break;
    case C_PCT_UP:   g_healPct = g_healPct <= 90 ? g_healPct + 5 : 95; break;
    case C_TRACK:
    {
        POINT p; GetCursorPos(&p); ScreenToClient(g_big.hWnd, &p);
        int v = 5 + (p.x - (HX + 90)) * 90 / (118 - 9);
        g_healPct = (BYTE)max(5, min(95, (v + 2) / 5 * 5));
        break;
    }
    case C_BOX0_A: g_hpBuff = g_hpBuff == 1 ? 0 : 1; break;	// click the lit one again = off
    case C_BOX0_B: g_hpBuff = g_hpBuff == 2 ? 0 : 2; break;
    case C_BOX1_A: g_ac = 1; break;    case C_BOX1_B: g_ac = 0; break;
    case C_BOX2_A: g_cure = 1; break;  case C_BOX2_B: g_cure = 0; break;
    case C_BOX3_A: g_res = 1; break;   case C_BOX3_B: g_res = 0; break;
    case C_BOX4_A: g_party = 1; break; case C_BOX4_B: g_party = 0; break;
    case C_BOX5_A: send = 1; changed = false; break;
    case C_BOX5_B: if (active) send = 2; changed = false; break;
    default: changed = false; break;
    }
    LeaveCriticalSection(&g_lock);

    if (!send && changed && active) send = 1;	// live update while the priest is out
    if (send) SendSettings((BYTE)send);
    PaintBig();
}

static void ActivateMini(int c)
{
    if (c == M_OPEN) { if (g_big.open) HideWin(g_big); else ShowWin(g_big, true); return; }
    EnterCriticalSection(&g_lock);
    bool active = g_active;
    LeaveCriticalSection(&g_lock);
    if (c == M_START && !active) SendSettings(1);
    if (c == M_STOP && active) SendSettings(2);
    PaintMini();
}

static LRESULT HandleMouse(Win& w, HWND h, UINT msg, LPARAM lp)
{
    int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
    bool big = &w == &g_big;
    auto hit = [&](int xx, int yy) { return big ? HitBig(xx, yy) : HitMini(xx, yy); };
    switch (msg)
    {
    case WM_MOUSEMOVE:
        if (w.dragging)
        {
            POINT p; GetCursorPos(&p);
            RECT wr; GetWindowRect(h, &wr);
            SetWindowPos(h, nullptr, wr.left + p.x - w.dragFrom.x, wr.top + p.y - w.dragFrom.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
            w.dragFrom = p;
            return 0;
        }
        if (!w.tracking) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; w.tracking = TrackMouseEvent(&t) != FALSE; }
        if (int c = hit(x, y); c != w.hover) { w.hover = c; PaintWin(w); }
        return 0;
    case WM_MOUSELEAVE:
        w.tracking = false;
        if (w.hover) { w.hover = 0; PaintWin(w); }
        return 0;
    case WM_LBUTTONDOWN:
    {
        int c = hit(x, y);
        if (c == 0) { w.dragging = true; GetCursorPos(&w.dragFrom); SetCapture(h); return 0; }
        w.pressed = c; SetCapture(h); PaintWin(w);
        return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (w.dragging) { w.dragging = false; return 0; }
        int c = hit(x, y), pressed = w.pressed;
        w.pressed = 0;
        if (pressed && pressed == c) { if (big) ActivateBig(c); else ActivateMini(c); }
        else PaintWin(w);
        return 0;
    }
    }
    return 0;
}

static LRESULT CALLBACK WndProcCommon(Win& w, HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    switch (msg)
    {
    case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
    case WM_MOUSEMOVE: case WM_MOUSELEAVE: case WM_LBUTTONDOWN: case WM_LBUTTONUP:
        return HandleMouse(w, h, msg, lp);
    case WM_RBUTTONUP: if (&w == &g_big) HideWin(g_big); return 0;
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: SyncVisibility(); return 0;
    case WM_SP_REFRESH:
        if (IsWindowVisible(g_big.hWnd)) PaintBig();
        if (IsWindowVisible(g_mini.hWnd)) PaintMini();
        return 0;
    case WM_SP_MINI: ShowWin(g_mini, false); return 0;
    case WM_SP_HIDE: HideWin(g_big); HideWin(g_mini); return 0;
    case WM_CLOSE: HideWin(w); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}
static LRESULT CALLBACK WndProcBig(HWND h, UINT m, WPARAM w, LPARAM l) { return WndProcCommon(g_big, h, m, w, l); }
static LRESULT CALLBACK WndProcMini(HWND h, UINT m, WPARAM w, LPARAM l) { return WndProcCommon(g_mini, h, m, w, l); }

static bool CreateWin(Win& w, WNDPROC proc, const wchar_t* cls, int width, int height)
{
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = proc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = cls;
    RegisterClassExW(&wc);
    w.w = width; w.h = height;
    w.hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        cls, L"Slave Priest", WS_POPUP, 0, 0, width, height, nullptr, nullptr, wc.hInstance, nullptr);
    if (!w.hWnd) return false;
    w.dc = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), width, -height, 1, 32, BI_RGB };
    SelectObject(w.dc, CreateDIBSection(w.dc, &bmi, DIB_RGB_COLORS, &w.bits, nullptr, 0));
    return true;
}

static DWORD WINAPI WindowThread(LPVOID)
{
    LoadSkin();
    if (!CreateWin(g_big, WndProcBig, L"NTT_SlavePriest", SP_SET_W, SP_SET_H) ||
        !CreateWin(g_mini, WndProcMini, L"NTT_SlavePriestMini", SP_MINI_W, SP_MINI_H))
    { Log("SP: window creation failed"); return 0; }
    SetTimer(g_mini.hWnd, 1, 250, nullptr);
    Log("SP: windows ready");
    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// ---------------------------------------------------------------- recv (game thread: parse only, UI via PostMessage)
void Sp_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[0] != WIZ_HSACS_HOOK || buf[1] != SP_SUBOP)
        return;
    BYTE op = buf[2];
    HWND target = g_mini.hWnd;
    if (op == 2)
    {
        EnterCriticalSection(&g_lock);
        g_active = false; g_secondsLeft = 0;
        LeaveCriticalSection(&g_lock);
        if (target) PostMessageW(target, WM_SP_HIDE, 0, 0);
        Log("SP recv: hide");
        return;
    }
    if (len < 3 + 8 + 4)
        return;
    EnterCriticalSection(&g_lock);
    g_active = buf[3] != 0;
    g_healPct = buf[4]; g_hpBuff = buf[5]; g_ac = buf[6]; g_heal = buf[7];
    g_cure = buf[8]; g_res = buf[9]; g_party = buf[10];
    g_secondsLeft = *(const UINT32*)(buf + 11);
    g_secondsAt = GetTickCount();
    bool anyTime = g_secondsLeft > 0 || g_active;
    LeaveCriticalSection(&g_lock);
    char b[80]; sprintf_s(b, "SP recv: op=%u active=%u left=%lu", op, buf[3], g_secondsLeft); Log(b);
    if (!target) return;
    if (op == 1 || (op == 3 && anyTime)) PostMessageW(target, op == 1 ? WM_SP_MINI : WM_SP_REFRESH, 0, 0);
    if (op == 3 && !anyTime) PostMessageW(target, WM_SP_HIDE, 0, 0);
}

bool Sp_CursorOverPanel()
{
    POINT p;
    if (!GetCursorPos(&p)) return false;
    HWND h = WindowFromPoint(p);
    return (g_big.hWnd && h == g_big.hWnd && IsWindowVisible(g_big.hWnd)) || (g_mini.hWnd && h == g_mini.hWnd && IsWindowVisible(g_mini.hWnd));
}

void Sp_Init()
{
    InitializeCriticalSection(&g_lock);
    InitializeCriticalSection(&g_sendLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}
