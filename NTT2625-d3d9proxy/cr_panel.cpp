// Collection Race panel for the XIGNCODE 2625 client, loaded through the d3d9 proxy
// (sibling of pus_store.cpp / rce_store.cpp). Same technique: a GDI layered overlay
// window fed from the recv hook, no code patching of the packed exe.
//
// Layout = Desktop\uifler\re_collection_race.uif (control rects scanned from the UIF):
// header (event name, min/max, close), winners + remaining time, 3 kill targets with
// progress, 3 rewards. Close -> the panel collapses into a small "CR" button at the
// game's top-right while the event is still running; clicking it reopens the panel.
//
// Server: GameServer CollectionRaceHandler.cpp, WIZ_HSACS_HOOK 0xE9 / sub 0xAF:
//   op 0 start   : 3*[u16 proto][u16 need]            + common
//   op 1 state   : 3*[u16 proto][u16 need][u16 kills] + common
//   common       : 3*[u32 item][u32 count][u8 rate] [u32 secs][u16 winners][u16 limit]
//                  [u8 nation][str name][u8 zone] [u8 meta=1 3*[str target][u32 icon]
//                  3*[str reward][u32 icon] [str zone]]      (str = u16 len + bytes)
//                  [u8 ext=2 2*[u16 proto][u16 need][u16 kills][str target][u32 icon]
//                  2*[u32 item][u32 count][u8 rate][str reward][u32 icon]]   (slots 4-5)
//   op 2 kill    : [u16 proto][u16 k0][u16 k1][u16 k2] [u16 k3][u16 k4]
//   op 3 counter : [u16 winners][u16 limit]
//   op 4 finish / op 5 hide
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <map>
#include <algorithm>
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);

static const BYTE WIZ_HSACS_HOOK = 0xE9;
static const BYTE CR_SUBOP = 0xAF;

// ---------------------------------------------------------------- layout (generated from the UIF)
#include "cr_layout.h"   // build_cr_assets.py: CRS_* rects + CRS_SPRITES (cr_ui\*.pus)
struct CrRect { int l, t, r, b; };
static CrRect R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }
static const int CR_W = CRS_W, CR_H = CRS_H;
static const int CR_HEADER_H = 58;                       // minimized height (head sprite)
static const int CR_TAB = 46;                            // collapsed "CR" button size
static const CrRect CR_TITLE      = R4(CRS_TEXT_EVENT_NAME);
static const CrRect CR_BTN_MIN    = R4(CRS_BTN_MIN);
static const CrRect CR_BTN_CLOSE  = R4(CRS_BTN_CLOSE);
static const CrRect CR_WIN_LABEL  = R4(CRS_TEXT_WINNERS_LABEL);
static const CrRect CR_WIN_VALUE  = { CRS_TEXT_WINNERS[0], CRS_TEXT_WINNERS[1], CRS_TEXT_WINNERS[2] + 30, CRS_TEXT_WINNERS[3] };
static const CrRect CR_TIME_LABEL = R4(CRS_TEXT_TIME_LABEL);
static const CrRect CR_TIME_VALUE = R4(CRS_TEXT_TIME);
static const CrRect CR_TGT_ICON[3]  = { R4(CRS_ICON_0), R4(CRS_ICON_1), R4(CRS_ICON_2) };
static const CrRect CR_TGT_FIRST[3] = { R4(CRS_TXT_NEEDS_FIRST_0), R4(CRS_TXT_NEEDS_FIRST_1), R4(CRS_TXT_NEEDS_FIRST_2) };
static const CrRect CR_TGT_SECOND[3]= { R4(CRS_TXT_NEEDS_SECOND_0), R4(CRS_TXT_NEEDS_SECOND_1), R4(CRS_TXT_NEEDS_SECOND_2) };
static const CrRect CR_REW_HEADER   = R4(CRS_TEXT_HEADER);
static const CrRect CR_REW_ICON[3]  = { R4(CRS_RICON_0), R4(CRS_RICON_1), R4(CRS_RICON_2) };
static const CrRect CR_REW_NAME[3]  = { R4(CRS_RTXT_RETURN_FIRST_0), R4(CRS_RTXT_RETURN_FIRST_1), R4(CRS_RTXT_RETURN_FIRST_2) };
static const CrRect CR_REW_COUNT[3] = { R4(CRS_RTXT_RETURN_SECOND_0), R4(CRS_RTXT_RETURN_SECOND_1), R4(CRS_RTXT_RETURN_SECOND_2) };
static const CrRect CR_REW_RATE[3]  = { R4(CRS_RTXT_RATE_0), R4(CRS_RTXT_RATE_1), R4(CRS_RTXT_RATE_2) };

// The UIF has 3 target + 3 reward rows; rows 4-5 repeat the third row's sprite one step lower.
static const int CR_MAX = 5, CR_UIF_ROWS = 3;
static const int CR_TGT_STEP = CRS_ICON_2[1] - CRS_ICON_1[1];
static const int CR_REW_STEP = CRS_RICON_1[1] - CRS_RICON_0[1];
static const int CR_SURF_H = CR_H + (CR_MAX - CR_UIF_ROWS) * (CR_TGT_STEP + CR_REW_STEP);   // surface height with every row used

// ---------------------------------------------------------------- state
struct CrState
{
    bool   active = false;
    UINT16 proto[CR_MAX] = {}, need[CR_MAX] = {}, kills[CR_MAX] = {};
    UINT32 rewardId[CR_MAX] = {}, rewardCount[CR_MAX] = {};
    BYTE   rewardRate[CR_MAX] = {};
    UINT32 secondsAtRecv = 0; DWORD recvTick = 0;
    UINT16 winners = 0, limit = 0;
    BYTE   nation = 0, zone = 0;
    std::string name, zoneName, targetName[CR_MAX], rewardName[CR_MAX];
    UINT32 targetIcon[CR_MAX] = {}, rewardIcon[CR_MAX] = {};
};

enum Mode { M_HIDDEN, M_OPEN, M_MINIMIZED, M_TAB };

static CRITICAL_SECTION g_lockC;
static CrState g_st;
static int     g_mode = M_HIDDEN;
static POINT   g_panelPos = { -1, -1 };                   // remembered screen position of the full panel

static HWND  g_hWndC = nullptr;
static HDC   g_memDCC = nullptr;
static void* g_bitsC = nullptr;
static char  g_uiDirC[MAX_PATH];

static const UINT WM_CR_REFRESH = WM_APP + 21;

// ---------------------------------------------------------------- packet reader
struct ReaderC
{
    const BYTE* p; size_t len, pos; bool ok = true;
    template <class T> T get()
    {
        T v{};
        if (pos + sizeof(T) > len) { ok = false; return v; }
        memcpy(&v, p + pos, sizeof(T)); pos += sizeof(T); return v;
    }
    std::string str()
    {
        UINT16 n = get<UINT16>();
        if (!ok || pos + n > len) { ok = false; return std::string(); }
        std::string s((const char*)p + pos, n); pos += n; return s;
    }
    size_t remain() const { return pos <= len ? len - pos : 0; }
};

// ---------------------------------------------------------------- drawing helpers
struct SpriteC { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<UINT32, SpriteC> g_iconsC;

static bool LoadPusC(const char* path, SpriteC& s)
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

static SpriteC* IconC(UINT32 iconId)
{
    UINT32 key = iconId ? iconId : 0xFFFFFFFF;
    auto it = g_iconsC.find(key);
    if (it != g_iconsC.end()) return it->second.dc ? &it->second : nullptr;
    SpriteC s; char path[MAX_PATH];
    if (iconId) sprintf_s(path, "%s\\icons\\%u.pus", g_uiDirC, iconId);
    else        sprintf_s(path, "%s\\noimage.pus", g_uiDirC);
    if (!LoadPusC(path, s)) s = SpriteC();
    g_iconsC[key] = s;
    return s.dc ? &g_iconsC[key] : (iconId ? IconC(0) : nullptr);
}

static int g_curW = CR_W, g_curH = CR_H;                  // current surface size

// UIF skin sprites (cr_ui\<name>.pus next to the exe), drawn at their UIF rect
static std::map<std::string, SpriteC> g_skin;
static bool g_skinLoaded = false, g_haveSkin = false;

static void LoadSkin()
{
    if (g_skinLoaded) return;
    g_skinLoaded = true;
    char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH);
    strcpy_s(strrchr(dir, '\\') + 1, 48, "HopeGuard\\cr_ui");
    int ok = 0;
    for (const CrsSprite& s : CRS_SPRITES)
    {
        char path[MAX_PATH]; sprintf_s(path, "%s\\%s.pus", dir, s.name);
        SpriteC sp;
        if (LoadPusC(path, sp)) { g_skin[s.name] = sp; ok++; }
    }
    g_haveSkin = ok >= 10;
    char msg[96]; sprintf_s(msg, "CR: skin sprites loaded %d/%d", ok, (int)(sizeof(CRS_SPRITES) / sizeof(CRS_SPRITES[0])));
    Log(msg);
}

static const CrsSprite* SpriteRect(const char* name)
{
    for (const CrsSprite& s : CRS_SPRITES)
        if (strcmp(s.name, name) == 0) return &s;
    return nullptr;
}

// draw a skin sprite at its UIF position shifted down by dy (clipped to the current surface height)
static bool Skin(const char* name, int dy = 0)
{
    auto it = g_skin.find(name);
    const CrsSprite* s = SpriteRect(name);
    if (it == g_skin.end() || !s) return false;
    int t = s->t + dy;
    int h = min(s->b - s->t, g_curH - t);
    if (h <= 0 || t < 0) return true;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_memDCC, s->l, t, s->r - s->l, h, it->second.dc, 0, 0, s->r - s->l, h, bf);
    return true;
}

static CrRect Shift(CrRect r, int dy) { return { r.l, r.t + dy, r.r, r.b + dy }; }

static void FillC(CrRect r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(g_curW, r.r), bb = min(g_curH, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* d = (BYTE*)g_bitsC + (y * CR_W + l) * 4;
        for (int x = l; x < rr; x++, d += 4)
        {
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
            d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}

static void BorderC(CrRect r, BYTE cr, BYTE cg, BYTE cb, BYTE a, int th = 1)
{
    FillC({ r.l, r.t, r.r, r.t + th }, cr, cg, cb, a);
    FillC({ r.l, r.b - th, r.r, r.b }, cr, cg, cb, a);
    FillC({ r.l, r.t, r.l + th, r.b }, cr, cg, cb, a);
    FillC({ r.r - th, r.t, r.r, r.b }, cr, cg, cb, a);
}

static void TextC(const std::string& s, CrRect r, COLORREF color, int pt, UINT fmt, bool bold = false)
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
        if (dy < 0 || dy >= g_curH) continue;
        const BYTE* src = (const BYTE*)bits + y * w * 4;
        BYTE* d = (BYTE*)g_bitsC + (dy * CR_W + r.l) * 4;
        for (int x = 0; x < w; x++, src += 4, d += 4)
        {
            int a = max(src[0], max(src[1], src[2]));
            if (!a || r.l + x < 0 || r.l + x >= g_curW) continue;
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
            d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
    SelectObject(dc, of); DeleteObject(font);
    SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}

static void IconBox(CrRect r, UINT32 icon)
{
    FillC(r, 30, 28, 22, 255);
    BorderC(r, 150, 120, 60, 255);
    if (SpriteC* s = IconC(icon))
    {
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(g_memDCC, r.l + 1, r.t + 1, (r.r - r.l) - 2, (r.b - r.t) - 2, s->dc, 0, 0, s->w, s->h, bf);
    }
}

// ---------------------------------------------------------------- controls
enum CtlC { C_NONE, C_MIN, C_CLOSE, C_TAB };
static int g_hoverC = C_NONE, g_pressedC = C_NONE;
static bool g_trackC = false, g_dragC = false, g_dragMoved = false;
static POINT g_dragFromC;

static bool InC(CrRect r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

static int HitTestC(int x, int y)
{
    if (g_mode == M_TAB) return C_TAB;
    if (InC(CR_BTN_CLOSE, x, y)) return C_CLOSE;
    if (InC(CR_BTN_MIN, x, y)) return C_MIN;
    return C_NONE;
}

static UINT32 RemainingSeconds(const CrState& s)
{
    DWORD el = (GetTickCount() - s.recvTick) / 1000;
    return el >= s.secondsAtRecv ? 0 : s.secondsAtRecv - el;
}

static std::string TargetLabel(const CrState& s, int i)
{
    if (s.proto[i] == 1 && s.nation != 0)
        return s.nation == 1 ? "El Morad players" : "Karus players";   // PK race: kill the enemy nation
    if (!s.targetName[i].empty()) return s.targetName[i];
    char b[32]; sprintf_s(b, "Target %u", s.proto[i]); return b;
}

// ---------------------------------------------------------------- rendering
static void PaintC()
{
    if (!g_hWndC || !g_memDCC) return;
    memset(g_bitsC, 0, (size_t)CR_W * CR_SURF_H * 4);

    EnterCriticalSection(&g_lockC);
    CrState s = g_st;
    int mode = g_mode;
    LeaveCriticalSection(&g_lockC);

    if (mode == M_TAB)
    {
        g_curW = CR_TAB; g_curH = CR_TAB;
        CrRect b = { 0, 0, CR_TAB, CR_TAB };
        FillC(b, g_hoverC == C_TAB ? 70 : 44, g_hoverC == C_TAB ? 56 : 38, 22, 235);
        BorderC(b, 210, 170, 80, 255, 2);
        TextC("CR", { 0, 4, CR_TAB, 26 }, RGB(250, 225, 150), 11, DT_CENTER | DT_VCENTER, true);
        char t[16]; UINT32 sec = RemainingSeconds(s);
        sprintf_s(t, "%u:%02u", sec / 60, sec % 60);
        TextC(t, { 0, 26, CR_TAB, 42 }, RGB(230, 230, 230), 7, DT_CENTER | DT_VCENTER);
    }
    else
    {
        LoadSkin();
        g_curW = CR_W; g_curH = (mode == M_MINIMIZED) ? CR_HEADER_H : CR_SURF_H;
        const bool skin = g_haveSkin;
        const bool hovMin = g_hoverC == C_MIN, hovClose = g_hoverC == C_CLOSE;
        const bool downMin = hovMin && g_pressedC == C_MIN, downClose = hovClose && g_pressedC == C_CLOSE;

        if (skin)
        {
            Skin("head");
            const char* mn = mode == M_MINIMIZED ? (downMin ? "btn_max_d" : hovMin ? "btn_max_o" : "btn_max_n")
                                                 : (downMin ? "btn_min_d" : hovMin ? "btn_min_o" : "btn_min_n");
            Skin(mn);
            Skin(downClose ? "btn_close_d" : hovClose ? "btn_close_o" : "btn_close_n");
        }
        else
        {
            CrRect frame = { 6, 8, CR_W - 6, mode == M_MINIMIZED ? CR_HEADER_H - 2 : CR_H - 4 };
            FillC(frame, 18, 20, 28, 230);
            BorderC(frame, 120, 96, 40, 255, 2);
            FillC(CR_BTN_MIN, hovMin ? 90 : 60, hovMin ? 80 : 54, 34, 255);
            TextC(mode == M_MINIMIZED ? "+" : "-", CR_BTN_MIN, RGB(255, 235, 200), 11, DT_CENTER | DT_VCENTER, true);
            FillC(CR_BTN_CLOSE, hovClose ? 160 : 110, 30, 30, 255);
            TextC("X", CR_BTN_CLOSE, RGB(255, 230, 200), 10, DT_CENTER | DT_VCENTER, true);
        }
        TextC(s.name.empty() ? "Collection Race" : s.name, CR_TITLE, RGB(255, 255, 0), 10, DT_LEFT | DT_VCENTER, true);

        if (mode == M_OPEN)
        {
            char buf[96];
            if (skin) Skin("time");
            TextC("Limit :", CR_WIN_LABEL, RGB(0, 255, 255), 10, DT_LEFT | DT_VCENTER);
            sprintf_s(buf, "%u of %u", s.winners, s.limit);
            TextC(buf, CR_WIN_VALUE, RGB(0, 255, 255), 10, DT_LEFT | DT_VCENTER);
            TextC("Time :", CR_TIME_LABEL, RGB(255, 255, 255), 10, DT_LEFT | DT_VCENTER);
            UINT32 sec = RemainingSeconds(s);
            sprintf_s(buf, "%02u:%02u", sec / 60, sec % 60);
            TextC(buf, CR_TIME_VALUE, sec <= 60 ? RGB(255, 120, 100) : RGB(255, 255, 255), 10, DT_LEFT | DT_VCENTER);

            // visible targets / rewards are packed into consecutive UIF slots (no gaps for empty ones)
            static const char* tgtSprite[3] = { "tgt0", "tgt1", "tgt2" };
            int lastBottom = SpriteRect("time")->b;
            for (int i = 0, k = 0; i < CR_MAX; i++)
            {
                if (!s.proto[i] || !s.need[i]) continue;
                const int slot = min(k, CR_UIF_ROWS - 1), ex = (k - slot) * CR_TGT_STEP;
                k++;
                lastBottom = SpriteRect(tgtSprite[slot])->b + ex;
                if (!skin || !Skin(tgtSprite[slot], ex)) { IconBox(Shift(CR_TGT_ICON[slot], ex), s.targetIcon[i]); }
                else if (SpriteC* ic = IconC(s.targetIcon[i]))
                {
                    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
                    CrRect r = Shift(CR_TGT_ICON[slot], ex);
                    AlphaBlend(g_memDCC, r.l, r.t, r.r - r.l, r.b - r.t, ic->dc, 0, 0, ic->w, ic->h, bf);
                }
                bool done = s.kills[i] >= s.need[i];
                sprintf_s(buf, "%s  %u / %u", s.proto[i] == 1 ? "Kill" : "Hunt", s.kills[i], s.need[i]);
                // progress tint inside the first text strip
                CrRect f = Shift(CR_TGT_FIRST[slot], ex);
                int fillW = (int)((f.r - f.l - 2) * (double)min(s.kills[i], s.need[i]) / s.need[i]);
                if (fillW > 0) FillC({ f.l + 1, f.t + 2, f.l + 1 + fillW, f.b - 2 }, done ? 60 : 150, done ? 150 : 110, done ? 60 : 30, 110);
                TextC(buf, { f.l + 4, f.t, f.r - 2, f.b }, done ? RGB(140, 240, 130) : RGB(255, 255, 255), 9, DT_LEFT | DT_VCENTER);
                CrRect sc = Shift(CR_TGT_SECOND[slot], ex);
                TextC(TargetLabel(s, i), { sc.l + 4, sc.t, sc.r - 2, sc.b }, RGB(255, 255, 255), 9, DT_LEFT | DT_VCENTER);
            }

            // reward block follows directly under the last visible target
            const int dy = lastBottom - SpriteRect("rewhead")->t;
            if (skin) Skin("rewhead", dy);
            TextC("Reward of winner", Shift(CR_REW_HEADER, dy), RGB(255, 255, 128), 10, DT_CENTER | DT_VCENTER);
            lastBottom = SpriteRect("rewhead")->b + dy;
            static const char* rewSprite[3] = { "rew0", "rew1", "rew2" };
            for (int i = 0, k = 0; i < CR_MAX; i++)
            {
                if (!s.rewardId[i] || !s.rewardCount[i]) continue;
                const int slot = min(k, CR_UIF_ROWS - 1), ry = dy + (k - slot) * CR_REW_STEP;
                k++;
                lastBottom = SpriteRect(rewSprite[slot])->b + ry;
                if (!skin || !Skin(rewSprite[slot], ry)) IconBox(Shift(CR_REW_ICON[slot], ry), s.rewardIcon[i]);
                else if (SpriteC* ic = IconC(s.rewardIcon[i]))
                {
                    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
                    CrRect r = Shift(CR_REW_ICON[slot], ry);
                    AlphaBlend(g_memDCC, r.l, r.t, r.r - r.l, r.b - r.t, ic->dc, 0, 0, ic->w, ic->h, bf);
                }
                std::string nm = s.rewardName[i];
                if (nm.empty()) { char b[32]; sprintf_s(b, "Item %u", s.rewardId[i]); nm = b; }
                CrRect n = Shift(CR_REW_NAME[slot], ry);
                TextC(nm, { n.l + 4, n.t, n.r - 2, n.b }, RGB(255, 255, 255), 9, DT_LEFT | DT_VCENTER, true);
                sprintf_s(buf, "%u", s.rewardCount[i]);
                TextC(buf, Shift(CR_REW_COUNT[slot], ry), RGB(255, 255, 255), 9, DT_LEFT | DT_VCENTER);
                if (s.rewardRate[i] > 0 && s.rewardRate[i] < 100) sprintf_s(buf, "%% %u", s.rewardRate[i]);
                else strcpy_s(buf, "% 100");
                TextC(buf, Shift(CR_REW_RATE[slot], ry), RGB(255, 255, 255), 9, DT_LEFT | DT_VCENTER);
            }
            // shrink the window to the content
            g_curH = min(CR_SURF_H, lastBottom + (CR_H - SpriteRect("rew2")->b));
        }
    }

    RECT wr; GetWindowRect(g_hWndC, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { g_curW, g_curH };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWndC, nullptr, &dst, &sz, g_memDCC, &src, 0, &bf, ULW_ALPHA);
}

// ---------------------------------------------------------------- window plumbing
static HWND FindGameWindowC()
{
    struct Ctx { DWORD pid; HWND best; int area; } ctx = { GetCurrentProcessId(), nullptr, 0 };
    EnumWindows([](HWND h, LPARAM lp) -> BOOL {
        Ctx* c = (Ctx*)lp;
        DWORD pid; GetWindowThreadProcessId(h, &pid);
        if (pid != c->pid || !IsWindowVisible(h) || h == g_hWndC) return TRUE;
        RECT r; GetWindowRect(h, &r);
        int a = (r.right - r.left) * (r.bottom - r.top);
        if (a > c->area) { c->area = a; c->best = h; }
        return TRUE;
    }, (LPARAM)&ctx);
    return ctx.best;
}

static HCURSOR g_gameCursorC = nullptr;

// place the window for the current mode (full panel remembers where the user dragged it)
static void PlaceC()
{
    RECT gr = { 0, 0, 1024, 768 };
    HWND game = FindGameWindowC();
    if (game) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
    if (g_mode == M_TAB)
    {
        SetWindowPos(g_hWndC, HWND_TOPMOST, gr.right - CR_TAB - 12, gr.top + 190, CR_TAB, CR_TAB, SWP_NOACTIVATE);
        return;
    }
    if (g_panelPos.x < 0)
    {
        g_panelPos.x = gr.right - CR_W - 16;
        g_panelPos.y = gr.top + 150;
    }
    SetWindowPos(g_hWndC, HWND_TOPMOST, g_panelPos.x, g_panelPos.y, CR_W,
                 g_mode == M_MINIMIZED ? CR_HEADER_H : CR_H, SWP_NOACTIVATE);
}

static void SetModeC(int mode)
{
    EnterCriticalSection(&g_lockC);
    g_mode = mode;
    LeaveCriticalSection(&g_lockC);
    g_hoverC = C_NONE;
    if (mode == M_HIDDEN) { ShowWindow(g_hWndC, SW_HIDE); return; }
    PlaceC();
    PaintC();
}

static void SyncVisibilityC()
{
    HWND game = FindGameWindowC();
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool want = g_mode != M_HIDDEN && game && !IsIconic(game) && pid == GetCurrentProcessId();
    if (want != (IsWindowVisible(g_hWndC) != FALSE))
    {
        if (want) { SetWindowPos(g_hWndC, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); PaintC(); }
        else ShowWindow(g_hWndC, SW_HIDE);
    }
    POINT p; CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) &&
        (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursorC = ci.hCursor;

    // countdown: repaint once a second while visible
    static DWORD lastSec = 0;
    if (want && GetTickCount() - lastSec >= 1000) { lastSec = GetTickCount(); PaintC(); }
}

static void ActivateC(int ctl)
{
    if (ctl == C_CLOSE)      SetModeC(g_st.active ? M_TAB : M_HIDDEN);
    else if (ctl == C_MIN)   SetModeC(g_mode == M_MINIMIZED ? M_OPEN : M_MINIMIZED);
    else if (ctl == C_TAB)   SetModeC(M_OPEN);
}

static LRESULT CALLBACK WndProcC(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
    switch (msg)
    {
    case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
    case WM_MOUSEMOVE:
        if (g_dragC)
        {
            POINT p; GetCursorPos(&p);
            if (p.x != g_dragFromC.x || p.y != g_dragFromC.y)
            {
                RECT wr; GetWindowRect(h, &wr);
                SetWindowPos(h, nullptr, wr.left + p.x - g_dragFromC.x, wr.top + p.y - g_dragFromC.y, 0, 0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                g_dragFromC = p; g_dragMoved = true;
            }
            return 0;
        }
        if (!g_trackC) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_trackC = TrackMouseEvent(&t) != FALSE; }
        if (int hit = HitTestC(x, y); hit != g_hoverC) { g_hoverC = hit; PaintC(); }
        return 0;
    case WM_MOUSELEAVE:
        g_trackC = false;
        if (g_hoverC != C_NONE) { g_hoverC = C_NONE; PaintC(); }
        return 0;
    case WM_LBUTTONDOWN:
    {
        int hit = HitTestC(x, y);
        if (hit == C_NONE && y < 48 && g_mode != M_TAB)     // header drags the panel
        {
            g_dragC = true; g_dragMoved = false; GetCursorPos(&g_dragFromC); SetCapture(h); return 0;
        }
        g_pressedC = hit;
        if (hit != C_NONE) { SetCapture(h); PaintC(); }
        return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (g_dragC)
        {
            g_dragC = false;
            RECT wr; GetWindowRect(h, &wr);
            g_panelPos.x = wr.left; g_panelPos.y = wr.top;
            return 0;
        }
        int hit = HitTestC(x, y), pressed = g_pressedC;
        g_pressedC = C_NONE;
        if (pressed != C_NONE && pressed == hit) ActivateC(hit);
        else PaintC();
        return 0;
    }
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursorC ? g_gameCursorC : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: SyncVisibilityC(); return 0;
    case WM_CR_REFRESH:
    {
        int want = (int)wp;
        if (want >= 0 && want != g_mode) SetModeC(want);
        else if (IsWindowVisible(h)) PaintC();
        return 0;
    }
    case WM_CLOSE: SetModeC(M_HIDDEN); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThreadC(LPVOID)
{
    GetModuleFileNameA(nullptr, g_uiDirC, MAX_PATH);
    strcpy_s(strrchr(g_uiDirC, '\\') + 1, 48, "HopeGuard\\pus_ui");   // reuse the PUS icon set

    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProcC;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = L"NTT_CollectionRace";
    RegisterClassExW(&wc);
    g_hWndC = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Collection Race", WS_POPUP, 0, 0, CR_W, CR_SURF_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWndC) { Log("CR: window creation failed"); return 0; }

    g_memDCC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), CR_W, -CR_SURF_H, 1, 32, BI_RGB };
    HBITMAP bmp = CreateDIBSection(g_memDCC, &bmi, DIB_RGB_COLORS, &g_bitsC, nullptr, 0);
    SelectObject(g_memDCC, bmp);
    Log("CR: window ready");
    SetTimer(g_hWndC, 1, 150, nullptr);

    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// ---------------------------------------------------------------- recv (game thread)
static bool ReadCommon(ReaderC& r, CrState& s)
{
    for (int i = 0; i < 3; i++) { s.rewardId[i] = r.get<UINT32>(); s.rewardCount[i] = r.get<UINT32>(); s.rewardRate[i] = r.get<BYTE>(); }
    s.secondsAtRecv = r.get<UINT32>(); s.recvTick = GetTickCount();
    s.winners = r.get<UINT16>(); s.limit = r.get<UINT16>();
    s.nation = r.get<BYTE>();
    s.name = r.str();
    s.zone = r.get<BYTE>();
    if (!r.ok) return false;
    if (r.remain() >= 1 && r.get<BYTE>() == 1)
    {
        for (int i = 0; i < 3; i++) { s.targetName[i] = r.str(); s.targetIcon[i] = r.get<UINT32>(); }
        for (int i = 0; i < 3; i++) { s.rewardName[i] = r.str(); s.rewardIcon[i] = r.get<UINT32>(); }
        s.zoneName = r.str();
        if (!r.ok) { Log("CR: meta block truncated (names ignored)"); r.ok = true; }
        else if (r.remain() >= 1 && r.get<BYTE>() == 2)
        {
            CrState x = s;
            for (int i = CR_UIF_ROWS; i < CR_MAX; i++)
            {
                x.proto[i] = r.get<UINT16>(); x.need[i] = r.get<UINT16>(); x.kills[i] = r.get<UINT16>();
                x.targetName[i] = r.str(); x.targetIcon[i] = r.get<UINT32>();
            }
            for (int i = CR_UIF_ROWS; i < CR_MAX; i++)
            {
                x.rewardId[i] = r.get<UINT32>(); x.rewardCount[i] = r.get<UINT32>(); x.rewardRate[i] = r.get<BYTE>();
                x.rewardName[i] = r.str(); x.rewardIcon[i] = r.get<UINT32>();
            }
            if (r.ok) s = x;
            else { Log("CR: slot 4-5 block truncated (ignored)"); r.ok = true; }
        }
    }
    s.active = true;
    return true;
}

void Cr_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[1] != CR_SUBOP) return;   // buf[0]=0xE9, buf[1]=0xAF, buf[2]=op
    ReaderC r{ buf, len, 2 };
    BYTE op = r.get<BYTE>();
    char msg[160];
    int wantMode = -1;

    if (op == 0 || op == 1)
    {
        CrState s;
        for (int i = 0; i < 3; i++)
        {
            s.proto[i] = r.get<UINT16>(); s.need[i] = r.get<UINT16>();
            s.kills[i] = (op == 1) ? r.get<UINT16>() : 0;
        }
        if (!ReadCommon(r, s)) { sprintf_s(msg, "CR recv op=%u: parse failed (len=%u)", op, (unsigned)len); Log(msg); return; }
        EnterCriticalSection(&g_lockC);
        bool wasHidden = g_mode == M_HIDDEN;
        g_st = s;
        LeaveCriticalSection(&g_lockC);
        if (wasHidden) wantMode = M_OPEN;
        sprintf_s(msg, "CR recv op=%u '%s' targets=%u/%u/%u time=%u", op, s.name.c_str(), s.proto[0], s.proto[1], s.proto[2], s.secondsAtRecv);
        Log(msg);
    }
    else if (op == 2)
    {
        UINT16 proto = r.get<UINT16>(); UINT16 k[3] = { r.get<UINT16>(), r.get<UINT16>(), r.get<UINT16>() };
        if (!r.ok) return;
        const bool more = r.remain() >= 4;                 // slots 4-5 (absent from an older server)
        UINT16 k3 = more ? r.get<UINT16>() : 0, k4 = more ? r.get<UINT16>() : 0;
        EnterCriticalSection(&g_lockC);
        for (int i = 0; i < 3; i++) g_st.kills[i] = k[i];
        if (more) { g_st.kills[3] = k3; g_st.kills[4] = k4; }
        LeaveCriticalSection(&g_lockC);
        (void)proto;
    }
    else if (op == 3)
    {
        UINT16 w = r.get<UINT16>(), l = r.get<UINT16>();
        if (!r.ok) return;
        EnterCriticalSection(&g_lockC);
        g_st.winners = w; g_st.limit = l;
        LeaveCriticalSection(&g_lockC);
    }
    else if (op == 4 || op == 5)
    {
        EnterCriticalSection(&g_lockC);
        g_st.active = false;
        LeaveCriticalSection(&g_lockC);
        wantMode = M_HIDDEN;
        sprintf_s(msg, "CR recv op=%u -> panel hidden", op); Log(msg);
    }
    if (g_hWndC) PostMessageW(g_hWndC, WM_CR_REFRESH, (WPARAM)wantMode, 0);
}

// ---------------------------------------------------------------- exports
void Cr_Init()
{
    InitializeCriticalSection(&g_lockC);
    CloseHandle(CreateThread(nullptr, 0, WindowThreadC, nullptr, 0, nullptr));
}

// called from pus_store.cpp's GetAsyncKeyState hook so clicks on our panel don't move the char
bool Cr_CursorOverPanel()
{
    if (!g_hWndC || !IsWindowVisible(g_hWndC)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWndC;
}
