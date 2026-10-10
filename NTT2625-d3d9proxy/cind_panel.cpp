// Cinderella War ("Fun Class") panel for the 2625 client, loaded through the d3d9 proxy.
// Port of the 2585 HSACSX CindirellaPanel: class select (warrior / rogue / mage / priest) + scoreboard
// (Karus / El Morad kills, own kill / death, remaining time, class change wait). Same technique as
// cr_panel.cpp: a GDI layered overlay window fed from the recv hook.
// Skin: build_cind_assets.py -> cind_ui\*.pus + cind_layout.h (2585 re_funclass layout; frame from the
// game's tournament_chaos_2021_a01 atlas, emblems from el_ka_symbol, portraits from 2585 funclass.dxt).
// Close collapses the panel into the "SELECT CLASS" tab (2585 re_war_icon btn_funclass) at the top right;
// the tab reopens it. Both only exist while the server says we are in the event.
//
// Server: GameServer CindirellaWar.cpp, WIZ_HSACS_HOOK 0xE9 / sub CINDIRELLA 0xE0 (GameDefine.h cindopcode):
//   S->C op 2 joinevent : [u8 prepare][u8 class][u32 secs][u16 myKill][u16 myDead][u16 karusKill][u16 elmoKill]
//        op 3 starting  : [u32 secs]
//        op 4 updatekda : [u8 0][u16 myKill][u16 myDead] | [u8 1][u16 elmoKill][u16 karusKill]
//        op 5 finish
//        op 0 selectclass / 1 nationchange : [u8 result] (6 success [u8 class] | 7 timewait [u32 secs] | 8,9,10)
//   C->S op 0 selectclass : [u8 class 0 warrior, 1 rogue, 2 mage, 3 priest]
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include <map>
#include "cind_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();

static const DWORD KO_SND_FNC = 0x00704070;  // unpack: thiscall(CAPISocket*, BYTE* buf, int len)
static const DWORD KO_PTR_PKT = 0x01115914;  // unpack: CAPISocket*
static const BYTE  WIZ_HSACS_HOOK = 0xE9;
static const BYTE  CIND_SUBOP = 0xE0;        // HSACSXOpCodes::CINDIRELLA
static const DWORD SELECT_WAIT = 300;        // server: myselectlasttime = UNIXTIME + 300

enum CindOp { CO_selectclass = 0, CO_nationchange, CO_joinevent, CO_starting, CO_updatekda, CO_finish,
              CO_success, CO_timewait, CO_notchange, CO_alreadyclass, CO_alreadynation };

struct Rc { int l, t, r, b; };
static Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }
static const int CI_W = CIS_BTN_CLOSE[2] > CIS_W ? CIS_BTN_CLOSE[2] : CIS_W, CI_H = CIS_H;
static const int CI_TAB_W = CIS_TAB_W, CI_TAB_H = CIS_TAB_H;
static const Rc CI_BTN[4] = { R4(CIS_BTN_WARRIOR), R4(CIS_BTN_ROGUE), R4(CIS_BTN_MAGE), R4(CIS_BTN_PRIEST) };
static const char* CI_BTN_NAME[4] = { "btn_warrior", "btn_rogue", "btn_mage", "btn_priest" };
static const Rc CI_CLOSE = R4(CIS_BTN_CLOSE);

// ---------------------------------------------------------------- state (g_lock)
struct CindState
{
    bool   active = false, prepare = false;
    int    myClass = -1;
    UINT32 secs = 0; DWORD secsAt = 0;          // remaining event time at secsAt
    UINT32 wait = 0; DWORD waitAt = 0;          // class change wait
    UINT16 myKill = 0, myDead = 0, karusKill = 0, elmoKill = 0;
};
enum Mode { M_HIDDEN, M_OPEN, M_TAB };

static CRITICAL_SECTION g_lock, g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQ;
static CindState g_st;
static int   g_mode = M_HIDDEN;
static POINT g_panelPos = { -1, -1 };

static HWND  g_hWnd = nullptr;
static HDC   g_memDC = nullptr;
static void* g_bits = nullptr;
static int   g_curW = CI_W, g_curH = CI_H;
static HCURSOR g_gameCursor = nullptr;
static const UINT WM_CI_REFRESH = WM_APP + 31;

static UINT32 Left(UINT32 v, DWORD at)
{
    DWORD el = (GetTickCount() - at) / 1000;
    return el >= v ? 0 : v - el;
}

// ---------------------------------------------------------------- send (flushed on the game thread)
static void QueueSend(const std::vector<BYTE>& p)
{
    EnterCriticalSection(&g_sendLock);
    g_sendQ.push_back(p);
    LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();
}

void Cind_FlushSendQueue()
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
        char b[64]; sprintf_s(b, "CIND send: op=%u class=%u", p[2], p.size() > 3 ? p[3] : 0); Log(b);
    }
}

// ---------------------------------------------------------------- sprites (cind_ui\*.pus, premultiplied BGRA)
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<std::string, Sprite> g_skin;
static bool g_skinLoaded = false;

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
    if (g_skinLoaded) return;
    g_skinLoaded = true;
    char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH);
    strcpy_s(strrchr(dir, '\\') + 1, 48, "HopeGuard\\cind_ui");
    int ok = 0;
    for (const CisSprite& s : CIS_SPRITES)
    {
        char path[MAX_PATH]; sprintf_s(path, "%s\\%s.pus", dir, s.name);
        Sprite sp;
        if (LoadPus(path, sp)) { g_skin[s.name] = sp; ok++; }
    }
    char msg[96]; sprintf_s(msg, "CIND: skin sprites loaded %d/%d", ok, (int)(sizeof(CIS_SPRITES) / sizeof(CIS_SPRITES[0])));
    Log(msg);
}

static bool Skin(const std::string& name)
{
    auto it = g_skin.find(name);
    if (it == g_skin.end()) return false;
    for (const CisSprite& s : CIS_SPRITES)
        if (name == s.name)
        {
            BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
            AlphaBlend(g_memDC, s.l, s.t, s.r - s.l, s.b - s.t, it->second.dc, 0, 0, it->second.w, it->second.h, bf);
            return true;
        }
    return false;
}

static void Fill(Rc r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(g_curW, r.r), bb = min(g_curH, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* d = (BYTE*)g_bits + (y * CI_W + l) * 4;
        for (int x = l; x < rr; x++, d += 4)
        {
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
            d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}

// text with a 1px dark shadow (the game's UI strings are drawn the same way)
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
        const int ox = pass == 0 ? 1 : 0;
        const BYTE cr = pass == 0 ? 0 : GetRValue(color), cg = pass == 0 ? 0 : GetGValue(color), cb = pass == 0 ? 0 : GetBValue(color);
        for (int y = 0; y < h; y++)
        {
            int dy = r.t + y + ox;
            if (dy < 0 || dy >= g_curH) continue;
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            for (int x = 0; x < w; x++, src += 4)
            {
                int dx = r.l + x + ox;
                int a = max(src[0], max(src[1], src[2]));
                if (!a || dx < 0 || dx >= g_curW) continue;
                if (pass == 0) a = a * 3 / 4;
                BYTE* d = (BYTE*)g_bits + (dy * CI_W + dx) * 4;
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

static std::string MMSS(UINT32 sec)
{
    char b[32]; sprintf_s(b, "%02u : %02u", sec / 60, sec % 60); return b;
}

// ---------------------------------------------------------------- controls
enum Ctl { C_NONE = -1, C_CLASS0 = 0, C_CLASS3 = 3, C_CLOSE = 10, C_TAB = 11 };
static int  g_hover = C_NONE, g_pressed = C_NONE;
static bool g_track = false, g_drag = false;
static POINT g_dragFrom;

static bool In(Rc r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

static int HitTest(int x, int y)
{
    if (g_mode == M_TAB) return C_TAB;
    if (In(CI_CLOSE, x, y)) return C_CLOSE;
    for (int i = 0; i < 4; i++) if (In(CI_BTN[i], x, y)) return i;
    return C_NONE;
}

// ---------------------------------------------------------------- rendering
static const COLORREF COL_LABEL = RGB(255, 255, 128);   // UIF 0xFFFFFF80
static const COLORREF COL_VALUE = RGB(255, 255, 255);
static const COLORREF COL_WAIT  = RGB(255, 255, 0);     // UIF 0xFFFFFF00

static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    memset(g_bits, 0, (size_t)CI_W * CI_H * 4);
    LoadSkin();

    EnterCriticalSection(&g_lock);
    CindState s = g_st;
    int mode = g_mode;
    LeaveCriticalSection(&g_lock);

    if (mode == M_TAB)
    {
        g_curW = CI_TAB_W; g_curH = CI_TAB_H;
        const char* st = g_pressed == C_TAB && g_hover == C_TAB ? "tab_d" : g_hover == C_TAB ? "tab_o" : "tab_n";
        if (!Skin(st))
        {
            Fill({ 0, 0, CI_TAB_W, CI_TAB_H }, 200, 110, 20, 240);
            Text("SELECT CLASS", { 0, 0, CI_TAB_W, CI_TAB_H }, RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER, true);
        }
    }
    else
    {
        g_curW = CI_W; g_curH = CI_H;
        if (!Skin("frame")) Fill({ 0, 0, CIS_W, CIS_H }, 24, 22, 18, 235);
        Skin("emb_karus"); Skin("emb_elmo");

        for (int i = 0; i < 4; i++)
        {
            const bool sel = s.myClass == i, hov = g_hover == i, down = hov && g_pressed == i;
            const char* st = (down || sel) ? "_d" : hov ? "_o" : "_n";
            Skin(std::string(CI_BTN_NAME[i]) + st);
            if (sel)   // selected class: gold outline
            {
                Rc b = CI_BTN[i];
                Fill({ b.l, b.t, b.r, b.t + 2 }, 240, 192, 96, 255); Fill({ b.l, b.b - 2, b.r, b.b }, 240, 192, 96, 255);
                Fill({ b.l, b.t, b.l + 2, b.b }, 240, 192, 96, 255); Fill({ b.r - 2, b.t, b.r, b.b }, 240, 192, 96, 255);
            }
        }
        const bool hc = g_hover == C_CLOSE;
        Skin(hc && g_pressed == C_CLOSE ? "btn_close_d" : hc ? "btn_close_o" : "btn_close_n");

        char b[32];
        Text("FUN CLASS EVENT", R4(CIS_LABEL_0), COL_LABEL, 9, DT_CENTER | DT_VCENTER, true);
        const UINT32 wait = Left(s.wait, s.waitAt);
        Text(wait ? "Class Change Wait  " + MMSS(wait) : (s.myClass < 0 ? "Select Your Class" : "You Can Change Your Class"),
             R4(CIS_TXT_SELECT_TIME), COL_WAIT, 9, DT_CENTER | DT_VCENTER);
        sprintf_s(b, "%u", s.karusKill); Text(b, R4(CIS_TXT_KARUS_KILL_COUNT), COL_VALUE, 12, DT_CENTER | DT_VCENTER, true);
        sprintf_s(b, "%u", s.elmoKill);  Text(b, R4(CIS_TXT_ELMO_KILL_COUNT), COL_VALUE, 12, DT_CENTER | DT_VCENTER, true);
        Text("Your Kill Count  :", R4(CIS_LABEL_2), COL_LABEL, 8, DT_RIGHT | DT_VCENTER);
        sprintf_s(b, "%u", s.myKill);    Text(b, R4(CIS_TXT_YOUR_KILL_COUNT), COL_VALUE, 11, DT_LEFT | DT_VCENTER, true);
        Text(":  Count Of Death", R4(CIS_LABEL_1), COL_LABEL, 8, DT_LEFT | DT_VCENTER);
        sprintf_s(b, "%u", s.myDead);    Text(b, R4(CIS_TXT_YOUR_DEATH_COUNT), COL_VALUE, 11, DT_RIGHT | DT_VCENTER, true);
        Text(s.prepare ? "Starts In :" : "Remaining Time :", R4(CIS_LABEL_3), COL_LABEL, 8, DT_CENTER | DT_VCENTER);
        const UINT32 left = Left(s.secs, s.secsAt);
        Text(MMSS(left), R4(CIS_TXT_REMAINING_TIME), left <= 60 ? RGB(255, 120, 100) : COL_VALUE, 10, DT_CENTER | DT_VCENTER, true);
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
        SetWindowPos(g_hWnd, HWND_TOPMOST, gr.right - CI_TAB_W - 14, gr.top + 96, CI_TAB_W, CI_TAB_H, SWP_NOACTIVATE);
        return;
    }
    if (g_panelPos.x < 0)
    {
        g_panelPos.x = gr.left + ((gr.right - gr.left) - CI_W) / 2;
        g_panelPos.y = gr.top + ((gr.bottom - gr.top) - CI_H) / 2;
    }
    SetWindowPos(g_hWnd, HWND_TOPMOST, g_panelPos.x, g_panelPos.y, CI_W, CI_H, SWP_NOACTIVATE);
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
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) &&
        (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursor = ci.hCursor;

    static DWORD lastSec = 0;   // countdowns
    if (want && g_mode == M_OPEN && GetTickCount() - lastSec >= 1000) { lastSec = GetTickCount(); Paint(); }
}

static void SelectClass(int cls)
{
    EnterCriticalSection(&g_lock);
    bool waiting = Left(g_st.wait, g_st.waitAt) > 0;
    LeaveCriticalSection(&g_lock);
    if (waiting) { Log("CIND: class change still waiting"); return; }
    QueueSend({ WIZ_HSACS_HOOK, CIND_SUBOP, CO_selectclass, (BYTE)cls });
}

static void Activate(int ctl)
{
    if (ctl == C_CLOSE)    SetMode(M_TAB);
    else if (ctl == C_TAB) SetMode(M_OPEN);
    else if (ctl >= C_CLASS0 && ctl <= C_CLASS3) SelectClass(ctl);
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
                SetWindowPos(h, nullptr, wr.left + p.x - g_dragFrom.x, wr.top + p.y - g_dragFrom.y, 0, 0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
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
        if (hit == C_NONE && g_mode == M_OPEN && y < CI_BTN[0].t)   // header drags the panel
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
    case WM_CI_REFRESH:
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
    wc.lpszClassName = L"NTT_FunClass";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Fun Class", WS_POPUP, 0, 0, CI_W, CI_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("CIND: window creation failed"); return 0; }

    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), CI_W, -CI_H, 1, 32, BI_RGB };
    HBITMAP bmp = CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0);
    SelectObject(g_memDC, bmp);
    Log("CIND: window ready");
    SetTimer(g_hWnd, 1, 150, nullptr);

    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// ---------------------------------------------------------------- recv (game thread: parse only, UI via PostMessage)
template <class T> static bool Get(const BYTE* p, size_t len, size_t& pos, T& v)
{
    if (pos + sizeof(T) > len) return false;
    memcpy(&v, p + pos, sizeof(T)); pos += sizeof(T); return true;
}

void Cind_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[0] != WIZ_HSACS_HOOK || buf[1] != CIND_SUBOP) return;
    size_t pos = 3;
    const BYTE op = buf[2];
    int wantMode = -1;
    char msg[160] = {};

    EnterCriticalSection(&g_lock);
    CindState& s = g_st;
    switch (op)
    {
    case CO_joinevent:
    {
        BYTE prep = 0, cls = 0; UINT32 secs = 0; UINT16 k = 0, d = 0, kk = 0, ek = 0;
        if (Get(buf, len, pos, prep) && Get(buf, len, pos, cls) && Get(buf, len, pos, secs) &&
            Get(buf, len, pos, k) && Get(buf, len, pos, d) && Get(buf, len, pos, kk) && Get(buf, len, pos, ek))
        {
            s.active = true; s.prepare = prep != 0; s.myClass = cls < 4 ? cls : -1;
            s.secs = secs; s.secsAt = GetTickCount();
            s.myKill = k; s.myDead = d; s.karusKill = kk; s.elmoKill = ek;
            wantMode = M_OPEN;
            sprintf_s(msg, "CIND recv: join prepare=%u class=%u secs=%u", prep, cls, secs);
        }
        break;
    }
    case CO_starting:
    {
        UINT32 secs = 0;
        if (Get(buf, len, pos, secs))
        {
            s.active = true; s.prepare = false; s.secs = secs; s.secsAt = GetTickCount();
            if (g_mode == M_HIDDEN) wantMode = M_OPEN;
            sprintf_s(msg, "CIND recv: starting secs=%u", secs);
        }
        break;
    }
    case CO_updatekda:
    {
        BYTE kind = 0; UINT16 a = 0, b = 0;
        if (Get(buf, len, pos, kind) && Get(buf, len, pos, a) && Get(buf, len, pos, b))
        {
            if (kind == 0) { s.myKill = a; s.myDead = b; }
            else           { s.elmoKill = a; s.karusKill = b; }
        }
        break;
    }
    case CO_finish:
        s = CindState();
        wantMode = M_HIDDEN;
        strcpy_s(msg, "CIND recv: finish -> hidden");
        break;
    case CO_selectclass:
    case CO_nationchange:
    {
        BYTE res = 0;
        if (!Get(buf, len, pos, res)) break;
        if (res == CO_success && op == CO_selectclass)
        {
            BYTE cls = 0;
            if (Get(buf, len, pos, cls) && cls < 4) s.myClass = cls;
            s.wait = SELECT_WAIT; s.waitAt = GetTickCount();
        }
        else if (res == CO_timewait)
        {
            UINT32 rem = 0;
            if (Get(buf, len, pos, rem)) { s.wait = rem; s.waitAt = GetTickCount(); }
        }
        sprintf_s(msg, "CIND recv: op=%u result=%u", op, res);
        break;
    }
    }
    LeaveCriticalSection(&g_lock);
    if (msg[0]) Log(msg);
    if (g_hWnd) PostMessageW(g_hWnd, WM_CI_REFRESH, (WPARAM)wantMode, 0);
}

// ---------------------------------------------------------------- exports
void Cind_Init()
{
    InitializeCriticalSection(&g_lock);
    InitializeCriticalSection(&g_sendLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// pus_store.cpp's GetAsyncKeyState hook: clicks on our panel must not move the character
bool Cind_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
