// Lottery Event panel for the 2625 client (d3d9 proxy), same technique as cr_panel / cind_panel:
// a GDI layered overlay window fed from the recv hook. Skin: build_lottery_assets.py -> lottery_ui\*.pus +
// lottery_layout.h (pieces of the game's tournament_chaos_2021_a01 atlas); item icons + names: ev_ui\.
// Close turns the panel into a small "Lottery" tab (top right, remaining time) while the event runs.
//
// Server: GameServer LotterySystem.cpp, WIZ_HSACS_HOOK 0xE9 / sub LOTTERY 0xC7:
//   S->C op 1 start : 5 x [u32 reqItem][u32 reqCount] 4 x [u32 rewardItem] [u32 limit][u32 secs][u32 sold][u32 myTickets]
//        op 2       : somebody bought a ticket (sold + 1)
//        op 3 result: [u8 0][str msg] | [u8 1][u32 myTickets]          (str = u16 len + bytes)
//        op 4       : event over
//   C->S op 3       : buy a ticket
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include <map>
#include "lottery_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();

static const DWORD KO_SND_FNC = 0x00704070;
static const DWORD KO_PTR_PKT = 0x01115914;
static const BYTE  WIZ_HSACS_HOOK = 0xE9;
static const BYTE  LT_SUBOP = 0xC7;            // HSACSXOpCodes::LOTTERY
static const UINT32 ITEM_GOLD = 900000000;

struct Rc { int l, t, r, b; };
static Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }

// ---------------------------------------------------------------- state
struct LtState
{
    bool   active = false;
    UINT32 req[5] = {}, reqCount[5] = {}, reward[4] = {};
    UINT32 limit = 0, secs = 0, sold = 0, mine = 0;
    DWORD  secsAt = 0;
    std::string msg; DWORD msgAt = 0; bool msgOk = false;
};
enum Mode { M_HIDDEN, M_OPEN, M_TAB };

static CRITICAL_SECTION g_lock, g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQ;
static LtState g_st;
static int   g_mode = M_HIDDEN;
static bool  g_userClosed = false;            // closed by the user: a resent "start" keeps the tab
static POINT g_panelPos = { -1, -1 };
static HWND  g_hWnd = nullptr;
static HDC   g_memDC = nullptr;
static void* g_bits = nullptr;
static int   g_curW = LT_W, g_curH = LT_H;
static HCURSOR g_gameCursor = nullptr;
static const UINT WM_LT_REFRESH = WM_APP + 41;

static UINT32 SecsLeft(const LtState& s)
{
    DWORD el = (GetTickCount() - s.secsAt) / 1000;
    return el >= s.secs ? 0 : s.secs - el;
}

// ---------------------------------------------------------------- send
static void QueueSend(const std::vector<BYTE>& p)
{
    EnterCriticalSection(&g_sendLock);
    g_sendQ.push_back(p);
    LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();
}

void Lottery_FlushSendQueue()
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
        Log("LOTTERY send: buy ticket");
    }
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
    const char* names[] = { "bg", "buy_n", "buy_o", "buy_d", "close_n", "close_o", "close_d", "tab_n", "tab_o" };
    int ok = 0;
    for (const char* n : names)
    {
        char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\lottery_ui\\%s.pus", g_dir, n);
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
    char b[96]; sprintf_s(b, "LOTTERY: skin %d/9, items %u", ok, (unsigned)g_items.size()); Log(b);
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
enum Ctl { C_NONE = -1, C_CLOSE = 1, C_BUY, C_TAB, C_REQ, C_REW0 };     // C_REW0..C_REW0+3
static int  g_hover = C_NONE, g_pressed = C_NONE;
static bool g_track = false, g_drag = false;
static POINT g_dragFrom;
static bool In(Rc r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

static int HitTest(int x, int y)
{
    if (g_mode == M_TAB) return C_TAB;
    if (In(R4(LT_CLOSE), x, y)) return C_CLOSE;
    if (In(R4(LT_BUY), x, y)) return C_BUY;
    if (In(R4(LT_REQ_SLOT), x, y)) return C_REQ;
    const int* rw[4] = { LT_REWARD_0, LT_REWARD_1, LT_REWARD_2, LT_REWARD_3 };
    for (int i = 0; i < 4; i++) if (In(R4(rw[i]), x, y)) return C_REW0 + i;
    return C_NONE;
}

static void IconIn(Rc slot, UINT32 item)
{
    if (!item) return;
    Rc in = { slot.l + 4, slot.t + 4, slot.r - 4, slot.b - 4 };
    if (Sprite* s = Icon(item)) Blit(s, in.l, in.t, in.r - in.l, in.b - in.t);
    else Text("?", in, RGB(255, 220, 120), 14, DT_CENTER | DT_VCENTER, true);
}

// ---------------------------------------------------------------- rendering
static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    memset(g_bits, 0, (size_t)LT_W * LT_H * 4);
    LoadSkin();
    EnterCriticalSection(&g_lock);
    LtState s = g_st;
    int mode = g_mode;
    LeaveCriticalSection(&g_lock);
    const UINT32 left = SecsLeft(s);
    char b[96];

    if (mode == M_TAB)
    {
        g_curW = LT_TAB_W; g_curH = LT_TAB_H;
        if (!Skin(g_hover == C_TAB ? "tab_o" : "tab_n", 0, 0)) Fill({ 0, 0, LT_TAB_W, LT_TAB_H }, 30, 80, 160, 230);
        sprintf_s(b, "Lottery  %02u:%02u", left / 60, left % 60);
        Text(b, { 0, 0, LT_TAB_W, LT_TAB_H }, RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER, true);
    }
    else
    {
        g_curW = LT_W; g_curH = LT_H;
        if (!Skin("bg", 0, 0)) Fill({ 0, 0, LT_W, LT_H }, 30, 26, 20, 240);
        Text("Lottery Event", R4(LT_TITLE), RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER, true);
        Skin(g_hover == C_CLOSE ? (g_pressed == C_CLOSE ? "close_d" : "close_o") : "close_n", LT_CLOSE[0], LT_CLOSE[1]);

        const COLORREF lab = RGB(255, 210, 120), val = RGB(255, 255, 255);
        Rc r1 = R4(LT_ROW_TIME), r2 = R4(LT_ROW_SOLD), r3 = R4(LT_ROW_TICKET);
        Text("Time Left :", { r1.l + 8, r1.t, r1.l + 110, r1.b }, lab, 8, DT_LEFT | DT_VCENTER, true);
        sprintf_s(b, "%02u : %02u", left / 60, left % 60);
        Text(b, { r1.l + 100, r1.t, r1.r - 8, r1.b }, left <= 60 ? RGB(255, 140, 120) : val, 8, DT_RIGHT | DT_VCENTER, true);
        Text("Ticket Sold :", { r2.l + 8, r2.t, r2.l + 110, r2.b }, lab, 8, DT_LEFT | DT_VCENTER, true);
        sprintf_s(b, "%u / %u", s.sold, s.limit);
        Text(b, { r2.l + 100, r2.t, r2.r - 8, r2.b }, val, 8, DT_RIGHT | DT_VCENTER, true);
        Text("Ticket :", { r3.l + 8, r3.t, r3.l + 110, r3.b }, lab, 8, DT_LEFT | DT_VCENTER, true);
        if (s.mine) sprintf_s(b, "%u", s.mine); else strcpy_s(b, "-");
        Text(b, { r3.l + 100, r3.t, r3.r - 8, r3.b }, val, 8, DT_RIGHT | DT_VCENTER, true);

        // ticket price: first required entry (gold shown as an amount)
        UINT32 reqItem = 0, reqCnt = 0;
        for (int i = 0; i < 5; i++) if (s.req[i]) { reqItem = s.req[i]; reqCnt = s.reqCount[i]; break; }
        IconIn(R4(LT_REQ_SLOT), reqItem);
        std::string price = reqItem == ITEM_GOLD ? Thousands(reqCnt) : (reqItem ? std::to_string(reqCnt) + " x" : "");
        Text(price, R4(LT_REQ_FIELD), RGB(120, 255, 180), 8, DT_CENTER | DT_VCENTER, true);
        Rc bb = R4(LT_BUY);
        Skin(g_hover == C_BUY ? (g_pressed == C_BUY ? "buy_d" : "buy_o") : "buy_n", bb.l, bb.t);
        Text("Buy", bb, RGB(255, 235, 200), 8, DT_CENTER | DT_VCENTER, true);

        Text("Rewards", R4(LT_REWARDS_BAR), RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER, true);
        const int* rk[4] = { LT_RANK_0, LT_RANK_1, LT_RANK_2, LT_RANK_3 };
        const int* rw[4] = { LT_REWARD_0, LT_REWARD_1, LT_REWARD_2, LT_REWARD_3 };
        const char* place[4] = { "1st", "2nd", "3rd", "4th" };
        for (int i = 0; i < 4; i++)
        {
            Text(place[i], R4(rk[i]), RGB(255, 220, 90), 8, DT_CENTER | DT_VCENTER, true);
            IconIn(R4(rw[i]), s.reward[i]);
        }

        // bottom line: hovered item, else the last buy result for a few seconds
        std::string line; COLORREF lc = RGB(220, 220, 220);
        if (g_hover == C_REQ && reqItem)
        {
            line = ItemName(reqItem);
            for (int i = 0; i < 5; i++) if (s.req[i] && s.req[i] != reqItem) line += " + " + ItemName(s.req[i]);
        }
        else if (g_hover >= C_REW0 && g_hover <= C_REW0 + 3 && s.reward[g_hover - C_REW0]) line = ItemName(s.reward[g_hover - C_REW0]);
        else if (!s.msg.empty() && GetTickCount() - s.msgAt < 6000) { line = s.msg; lc = s.msgOk ? RGB(140, 255, 140) : RGB(255, 140, 120); }
        Text(line, R4(LT_MSG), lc, 7, DT_CENTER | DT_VCENTER);
    }

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
    if (g_mode == M_TAB)
    {
        SetWindowPos(g_hWnd, HWND_TOPMOST, gr.right - LT_TAB_W - 12, gr.top + 250, LT_TAB_W, LT_TAB_H, SWP_NOACTIVATE);
        return;
    }
    if (g_panelPos.x < 0)
    {
        g_panelPos.x = gr.left + ((gr.right - gr.left) - LT_W) / 2;
        g_panelPos.y = gr.top + ((gr.bottom - gr.top) - LT_H) / 2;
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
    if (want && GetTickCount() - lastSec >= 1000) { lastSec = GetTickCount(); Paint(); }
}

static void Activate(int ctl)
{
    if (ctl == C_CLOSE) { g_userClosed = true; SetMode(g_st.active ? M_TAB : M_HIDDEN); }
    else if (ctl == C_TAB) { g_userClosed = false; SetMode(M_OPEN); }
    else if (ctl == C_BUY) QueueSend({ WIZ_HSACS_HOOK, LT_SUBOP, 3 });
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
        if ((hit == C_NONE || hit == C_REQ || hit >= C_REW0) && g_mode == M_OPEN && y < LT_ROW_TIME[1])
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
    wc.lpszClassName = L"NTT_Lottery";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Lottery", WS_POPUP, 0, 0, LT_W, LT_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("LOTTERY: window creation failed"); return 0; }
    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), LT_W, -LT_H, 1, 32, BI_RGB };
    HBITMAP bmp = CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0);
    SelectObject(g_memDC, bmp);
    Log("LOTTERY: window ready");
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

void Lottery_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[0] != WIZ_HSACS_HOOK || buf[1] != LT_SUBOP) return;
    size_t pos = 3;
    const BYTE op = buf[2];
    int wantMode = -1;
    char msg[200] = {};
    EnterCriticalSection(&g_lock);
    LtState& s = g_st;
    if (op == 1)
    {
        LtState n;
        bool ok = true;
        for (int i = 0; i < 5; i++) ok = ok && Get(buf, len, pos, n.req[i]) && Get(buf, len, pos, n.reqCount[i]);
        for (int i = 0; i < 4; i++) ok = ok && Get(buf, len, pos, n.reward[i]);
        ok = ok && Get(buf, len, pos, n.limit) && Get(buf, len, pos, n.secs) && Get(buf, len, pos, n.sold) && Get(buf, len, pos, n.mine);
        if (ok)
        {
            n.active = true; n.secsAt = GetTickCount();
            s = n;
            wantMode = g_userClosed ? M_TAB : M_OPEN;
            sprintf_s(msg, "LOTTERY recv: start limit=%u secs=%u sold=%u mine=%u", n.limit, n.secs, n.sold, n.mine);
        }
    }
    else if (op == 2) s.sold++;
    else if (op == 3)
    {
        BYTE ok = 0;
        if (Get(buf, len, pos, ok))
        {
            if (ok == 1) { UINT32 t = 0; if (Get(buf, len, pos, t)) s.mine = t; s.msg = "Ticket purchased."; s.msgOk = true; }
            else
            {
                UINT16 n = 0; std::string m;
                if (Get(buf, len, pos, n) && pos + n <= len) m.assign((const char*)buf + pos, n);
                s.msg = m.empty() ? "Purchase failed." : m; s.msgOk = false;
            }
            s.msgAt = GetTickCount();
            sprintf_s(msg, "LOTTERY recv: buy result %u", ok);
        }
    }
    else if (op == 4)
    {
        s = LtState();
        g_userClosed = false;
        wantMode = M_HIDDEN;
        strcpy_s(msg, "LOTTERY recv: event over");
    }
    LeaveCriticalSection(&g_lock);
    if (msg[0]) Log(msg);
    if (g_hWnd) PostMessageW(g_hWnd, WM_LT_REFRESH, (WPARAM)wantMode, 0);
}

// ---------------------------------------------------------------- exports
void Lottery_Init()
{
    InitializeCriticalSection(&g_lock);
    InitializeCriticalSection(&g_sendLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

bool Lottery_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
