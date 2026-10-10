// Event rewards panel for the 2625 client (d3d9 proxy): while BDW / Chaos / Juraid signs up, shows what the
// winners and the losers get (EVENT_REWARDS). Same technique as lottery_panel.cpp; skin from
// build_evreward_assets.py (evreward_ui\*.pus + evreward_layout.h, the game's tournament atlas), icons ev_ui\.
//
// Server: GameServer EventMainSystem.cpp TempleEventRewardInfoSend, WIZ_HSACS_HOOK 0xE9 / sub EVREWARD 0xEE:
//   op 1: [str name][u16 signSecs] 2 x ([u8 n] n x [u32 item][u32 count] [u32 exp][u32 loyalty][u32 cash][u32 noah])
//   (str = u16 len + bytes; winner first, then loser). The panel closes itself when the sign-up time runs out.
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <map>
#include "evreward_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);

static const BYTE  WIZ_HSACS_HOOK = 0xE9;
static const BYTE  ER_SUBOP = 0xEE;            // HSACSXOpCodes::EVREWARD
static const UINT32 ITEM_GOLD = 900000000;
static const int LT_W = ER_W, LT_H = ER_H;     // names used by the shared drawing helpers

struct Rc { int l, t, r, b; };
static Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }

struct Side { std::vector<std::pair<UINT32, UINT32>> items; UINT32 exp = 0, loyalty = 0, cash = 0, noah = 0; };
struct ErState { bool active = false; std::string name; UINT32 secs = 0; DWORD secsAt = 0; Side side[2]; };
enum Mode { M_HIDDEN, M_OPEN };

static CRITICAL_SECTION g_lock;
static ErState g_st;
static int   g_mode = M_HIDDEN;
static POINT g_panelPos = { -1, -1 };
static HWND  g_hWnd = nullptr;
static HDC   g_memDC = nullptr;
static void* g_bits = nullptr;
static int   g_curW = ER_W, g_curH = ER_H;
static HCURSOR g_gameCursor = nullptr;
static const UINT WM_LT_REFRESH = WM_APP + 42;

static UINT32 SecsLeft(const ErState& s)
{
    DWORD el = (GetTickCount() - s.secsAt) / 1000;
    return el >= s.secs ? 0 : s.secs - el;
}

// ---------------------------------------------------------------- sprites / icons
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<std::string, Sprite> g_skin;
static std::map<UINT32, Sprite> g_icons;
static std::map<UINT32, std::pair<UINT32, std::string>> g_items;   // item -> icon, name
static char g_dir[MAX_PATH];
static bool g_loaded = false;

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
    if (g_loaded) return;
    g_loaded = true;
    GetModuleFileNameA(nullptr, g_dir, MAX_PATH);
    *(strrchr(g_dir, '\\') + 1) = 0;
    const char* names[] = { "bg", "close_n", "close_o", "close_d", "x_n", "x_o", "x_d" };
    int ok = 0;
    for (const char* n : names)
    {
        char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\evreward_ui\\%s.pus", g_dir, n);
        Sprite s; if (LoadPus(p, s)) { g_skin[n] = s; ok++; }
    }
    char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\ev_ui\\items.txt", g_dir);
    FILE* f = nullptr;
    if (fopen_s(&f, p, "r") == 0 && f)
    {
        char line[256];
        while (fgets(line, sizeof(line), f))
        {
            UINT32 item = 0, icon = 0; char name[160] = {};
            if (sscanf_s(line, "%u|%u|%159[^\r\n]", &item, &icon, name, (unsigned)sizeof(name)) >= 2)
                g_items[item] = { icon, name };
        }
        fclose(f);
    }
    char b[96]; sprintf_s(b, "EVREWARD: skin %d/7, items %u", ok, (unsigned)g_items.size()); Log(b);
}

static Sprite* Icon(UINT32 item)
{
    auto it = g_items.find(item);
    if (it == g_items.end() || !it->second.first) return nullptr;
    UINT32 icon = it->second.first;
    auto ic = g_icons.find(icon);
    if (ic != g_icons.end()) return ic->second.dc ? &ic->second : nullptr;
    char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\ev_ui\\icons\\%u.pus", g_dir, icon);
    Sprite s; if (!LoadPus(p, s)) s = Sprite();
    g_icons[icon] = s;
    return s.dc ? &g_icons[icon] : nullptr;
}

static std::string ItemName(UINT32 item)
{
    if (item == ITEM_GOLD) return "Coins";
    auto it = g_items.find(item);
    if (it != g_items.end() && !it->second.second.empty()) return it->second.second;
    char b[32]; sprintf_s(b, "Item %u", item); return b;
}

static void Blit(Sprite* s, int x, int y, int w = -1, int h = -1)
{
    if (!s || !s->dc) return;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_memDC, x, y, w < 0 ? s->w : w, h < 0 ? s->h : h, s->dc, 0, 0, s->w, s->h, bf);
}
static bool Skin(const char* n, int x, int y) { auto it = g_skin.find(n); if (it == g_skin.end()) return false; Blit(&it->second, x, y); return true; }

static void Fill(Rc r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(g_curW, r.r), bb = min(g_curH, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* d = (BYTE*)g_bits + (y * LT_W + l) * 4;
        for (int x = l; x < rr; x++, d += 4)
        {
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
            d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}

static void Text(const std::string& s, Rc r, COLORREF color, int pt, UINT fmt, bool bold = false)
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
    for (int pass = 0; pass < 2; pass++)
    {
        const int o = pass == 0 ? 1 : 0;
        const BYTE cr = pass ? GetRValue(color) : 0, cg = pass ? GetGValue(color) : 0, cb = pass ? GetBValue(color) : 0;
        for (int y = 0; y < h; y++)
        {
            int dy = r.t + y + o;
            if (dy < 0 || dy >= g_curH) continue;
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            for (int x = 0; x < w; x++, src += 4)
            {
                int dx = r.l + x + o;
                int a = max(src[0], max(src[1], src[2]));
                if (!a || dx < 0 || dx >= g_curW) continue;
                if (!pass) a = a * 3 / 4;
                BYTE* d = (BYTE*)g_bits + (dy * LT_W + dx) * 4;
                d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
                d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
                d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
            }
        }
    }
    SelectObject(dc, of); DeleteObject(font);
    SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}

static std::string Thousands(UINT32 v)
{
    char raw[16]; sprintf_s(raw, "%u", v);
    std::string s = raw, out;
    for (size_t i = 0; i < s.size(); i++)
    {
        if (i && (s.size() - i) % 3 == 0) out += '.';
        out += s[i];
    }
    return out;
}

// ---------------------------------------------------------------- controls
enum Ctl { C_NONE = -1, C_CLOSE = 1, C_X, C_SLOT0 = 10 };      // C_SLOT0 + side * ER_ROWS + row
static int  g_hover = C_NONE, g_pressed = C_NONE;
static bool g_track = false, g_drag = false;
static POINT g_dragFrom;
static bool In(Rc r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

static const int* const kSlot[2][ER_ROWS]  = { { ER_WIN_SLOT_0, ER_WIN_SLOT_1, ER_WIN_SLOT_2 },    { ER_LOSE_SLOT_0, ER_LOSE_SLOT_1, ER_LOSE_SLOT_2 } };
static const int* const kName[2][ER_ROWS]  = { { ER_WIN_NAME_0, ER_WIN_NAME_1, ER_WIN_NAME_2 },    { ER_LOSE_NAME_0, ER_LOSE_NAME_1, ER_LOSE_NAME_2 } };
static const int* const kCount[2][ER_ROWS] = { { ER_WIN_COUNT_0, ER_WIN_COUNT_1, ER_WIN_COUNT_2 }, { ER_LOSE_COUNT_0, ER_LOSE_COUNT_1, ER_LOSE_COUNT_2 } };

static int HitTest(int x, int y)
{
    if (In(R4(ER_CLOSE), x, y)) return C_CLOSE;
    if (In(R4(ER_CLOSE_X), x, y)) return C_X;
    for (int s = 0; s < 2; s++)
        for (int i = 0; i < ER_ROWS; i++)
            if (In(R4(kSlot[s][i]), x, y)) return C_SLOT0 + s * ER_ROWS + i;
    return C_NONE;
}

static void IconIn(Rc slot, UINT32 item)
{
    if (!item) return;
    Rc in = { slot.l + 4, slot.t + 4, slot.r - 4, slot.b - 4 };
    if (Sprite* s = Icon(item)) Blit(s, in.l, in.t, in.r - in.l, in.b - in.t);
    else Text("?", in, RGB(255, 220, 120), 14, DT_CENTER | DT_VCENTER, true);
}

static std::string Num(UINT32 v)
{
    char raw[16]; sprintf_s(raw, "%u", v);
    std::string s = raw, out;
    for (size_t i = 0; i < s.size(); i++) { if (i && (s.size() - i) % 3 == 0) out += '.'; out += s[i]; }
    return out;
}

// ---------------------------------------------------------------- rendering
static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    memset(g_bits, 0, (size_t)ER_W * ER_H * 4);
    LoadSkin();
    EnterCriticalSection(&g_lock);
    ErState s = g_st;
    LeaveCriticalSection(&g_lock);
    const UINT32 left = SecsLeft(s);
    char b[96];
    g_curW = ER_W; g_curH = ER_H;
    if (!Skin("bg", 0, 0)) Fill({ 0, 0, ER_W, ER_H }, 30, 26, 20, 240);
    Text(s.name, R4(ER_TITLE), RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER, true);
    sprintf_s(b, "%02u:%02u", left / 60, left % 60);
    Text(b, R4(ER_TIME), left <= 60 ? RGB(255, 140, 120) : RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER, true);
    Skin(g_hover == C_X ? (g_pressed == C_X ? "x_d" : "x_o") : "x_n", ER_CLOSE_X[0], ER_CLOSE_X[1]);

    const int* bars[2] = { ER_WIN_BAR, ER_LOSE_BAR };
    const int* infos[2] = { ER_WIN_INFO, ER_LOSE_INFO };
    Text("Winner", R4(bars[0]), RGB(90, 255, 90), 9, DT_CENTER | DT_VCENTER, true);
    Text("Loser", R4(bars[1]), RGB(255, 120, 180), 9, DT_CENTER | DT_VCENTER, true);
    for (int k = 0; k < 2; k++)
    {
        const Side& sd = s.side[k];
        for (int i = 0; i < ER_ROWS && i < (int)sd.items.size(); i++)
        {
            IconIn(R4(kSlot[k][i]), sd.items[i].first);
            Rc n = R4(kName[k][i]);
            bool hov = g_hover == C_SLOT0 + k * ER_ROWS + i;
            Text(ItemName(sd.items[i].first), { n.l + 4, n.t, n.r - 4, n.b }, hov ? RGB(255, 255, 160) : RGB(255, 255, 255), 7, DT_CENTER | DT_VCENTER);
            Text(Num(sd.items[i].second), R4(kCount[k][i]), RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER, true);
        }
        std::string info;
        if (sd.exp) info += "EXP " + Num(sd.exp);
        if (sd.loyalty) info += (info.empty() ? "" : "  ") + std::string("NP ") + Num(sd.loyalty);
        if (sd.cash) info += (info.empty() ? "" : "  ") + std::string("KC ") + Num(sd.cash);
        if (sd.noah) info += (info.empty() ? "" : "  ") + std::string("Coins ") + Num(sd.noah);
        Text(info, R4(infos[k]), RGB(255, 225, 140), 7, DT_CENTER | DT_VCENTER);
    }
    Rc cb = R4(ER_CLOSE);
    Skin(g_hover == C_CLOSE ? (g_pressed == C_CLOSE ? "close_d" : "close_o") : "close_n", cb.l, cb.t);
    Text("Cancel", cb, RGB(255, 235, 200), 8, DT_CENTER | DT_VCENTER, true);

    RECT wr; GetWindowRect(g_hWnd, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { g_curW, g_curH };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
}

// ---------------------------------------------------------------- window plumbing
static HWND FindGameWindow()
{
    struct Ctx { DWORD pid; HWND best; int area; } ctx = { GetCurrentProcessId(), nullptr, 0 };
    EnumWindows([](HWND h, LPARAM lp) -> BOOL {
        Ctx* c = (Ctx*)lp;
        DWORD pid; GetWindowThreadProcessId(h, &pid);
        if (pid != c->pid || !IsWindowVisible(h) || h == g_hWnd) return TRUE;
        RECT r; GetWindowRect(h, &r);
        int a = (r.right - r.left) * (r.bottom - r.top);
        if (a > c->area) { c->area = a; c->best = h; }
        return TRUE;
    }, (LPARAM)&ctx);
    return ctx.best;
}

static void Place()
{
    RECT gr = { 0, 0, 1024, 768 };
    HWND game = FindGameWindow();
    if (game) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
    if (g_panelPos.x < 0)
    {
        g_panelPos.x = gr.left + ((gr.right - gr.left) - LT_W) / 2;
        g_panelPos.y = gr.top + 150;                    // under the game's own event enter window
    }
    SetWindowPos(g_hWnd, HWND_TOPMOST, g_panelPos.x, g_panelPos.y, LT_W, LT_H, SWP_NOACTIVATE);
}

static void SetMode(int mode)
{
    EnterCriticalSection(&g_lock);
    g_mode = mode;
    LeaveCriticalSection(&g_lock);
    g_hover = C_NONE;
    if (mode == M_HIDDEN) { ShowWindow(g_hWnd, SW_HIDE); return; }
    Place();
    Paint();
}

static void SyncVisibility()
{
    HWND game = FindGameWindow();
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool want = g_mode != M_HIDDEN && game && !IsIconic(game) && pid == GetCurrentProcessId();
    if (want != (IsWindowVisible(g_hWnd) != FALSE))
    {
        if (want) { SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); Paint(); }
        else ShowWindow(g_hWnd, SW_HIDE);
    }
    POINT p; CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) && (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursor = ci.hCursor;
    static DWORD lastSec = 0;
    if (want && GetTickCount() - lastSec >= 1000)
    {
        lastSec = GetTickCount();
        if (SecsLeft(g_st) == 0) { g_st.active = false; SetMode(M_HIDDEN); return; }   // sign-up over
        Paint();
    }
}

static void Activate(int ctl)
{
    if (ctl == C_CLOSE || ctl == C_X) SetMode(M_HIDDEN);
}

static LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
    switch (msg)
    {
    case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
    case WM_MOUSEMOVE:
        if (g_drag)
        {
            POINT p; GetCursorPos(&p);
            if (p.x != g_dragFrom.x || p.y != g_dragFrom.y)
            {
                RECT wr; GetWindowRect(h, &wr);
                SetWindowPos(h, nullptr, wr.left + p.x - g_dragFrom.x, wr.top + p.y - g_dragFrom.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                g_dragFrom = p;
            }
            return 0;
        }
        if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
        if (int hit = HitTest(x, y); hit != g_hover) { g_hover = hit; Paint(); }
        return 0;
    case WM_MOUSELEAVE:
        g_track = false;
        if (g_hover != C_NONE) { g_hover = C_NONE; Paint(); }
        return 0;
    case WM_LBUTTONDOWN:
    {
        int hit = HitTest(x, y);
        if ((hit == C_NONE || hit >= C_SLOT0) && g_mode == M_OPEN && y < ER_WIN_BAR[1])
        {
            g_drag = true; GetCursorPos(&g_dragFrom); SetCapture(h); return 0;
        }
        g_pressed = hit;
        if (hit != C_NONE) { SetCapture(h); Paint(); }
        return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (g_drag)
        {
            g_drag = false;
            RECT wr; GetWindowRect(h, &wr);
            g_panelPos.x = wr.left; g_panelPos.y = wr.top;
            return 0;
        }
        int hit = HitTest(x, y), pressed = g_pressed;
        g_pressed = C_NONE;
        if (pressed != C_NONE && pressed == hit) Activate(hit);
        Paint();
        return 0;
    }
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: SyncVisibility(); return 0;
    case WM_LT_REFRESH:
    {
        int want = (int)wp;
        if (want >= 0 && want != g_mode) SetMode(want);
        else if (IsWindowVisible(h)) Paint();
        return 0;
    }
    case WM_CLOSE: SetMode(M_HIDDEN); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThread(LPVOID)
{
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = L"NTT_EventReward";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Event Rewards", WS_POPUP, 0, 0, LT_W, LT_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("EVREWARD: window creation failed"); return 0; }
    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), LT_W, -LT_H, 1, 32, BI_RGB };
    HBITMAP bmp = CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0);
    SelectObject(g_memDC, bmp);
    Log("EVREWARD: window ready");
    SetTimer(g_hWnd, 1, 150, nullptr);
    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// ---------------------------------------------------------------- recv (game thread)
template <class T> static bool Get(const BYTE* p, size_t len, size_t& pos, T& v)
{
    if (pos + sizeof(T) > len) return false;
    memcpy(&v, p + pos, sizeof(T)); pos += sizeof(T); return true;
}

void EventTitle_SetJuraid(bool on);     // event_title.cpp

void EvReward_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[0] != WIZ_HSACS_HOOK || buf[1] != ER_SUBOP || buf[2] != 1) return;
    size_t pos = 3;
    ErState n;
    UINT16 sl = 0, secs = 0;
    if (!Get(buf, len, pos, sl) || pos + sl > len) return;
    n.name.assign((const char*)buf + pos, sl); pos += sl;
    if (!Get(buf, len, pos, secs)) return;
    for (int k = 0; k < 2; k++)
    {
        BYTE cnt = 0;
        if (!Get(buf, len, pos, cnt)) return;
        for (int i = 0; i < cnt; i++)
        {
            UINT32 item = 0, c = 0;
            if (!Get(buf, len, pos, item) || !Get(buf, len, pos, c)) return;
            n.side[k].items.push_back({ item, c });
        }
        if (!Get(buf, len, pos, n.side[k].exp) || !Get(buf, len, pos, n.side[k].loyalty) ||
            !Get(buf, len, pos, n.side[k].cash) || !Get(buf, len, pos, n.side[k].noah)) return;
    }
    n.active = true; n.secs = secs; n.secsAt = GetTickCount();
    EventTitle_SetJuraid(n.name.compare(0, 6, "Juraid") == 0);
    EnterCriticalSection(&g_lock);
    g_st = n;
    LeaveCriticalSection(&g_lock);
    char b[160]; sprintf_s(b, "EVREWARD recv: %s secs=%u win=%u lose=%u", n.name.c_str(), secs, (unsigned)n.side[0].items.size(), (unsigned)n.side[1].items.size()); Log(b);
    if (g_hWnd) PostMessageW(g_hWnd, WM_LT_REFRESH, (WPARAM)M_OPEN, 0);
}

void EvReward_Init()
{
    InitializeCriticalSection(&g_lock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

bool EvReward_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
