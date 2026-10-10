// Drop box panel for the 2625 client (d3d9 proxy): the items of the last box a monster dropped, on the right
// side of the screen (reference F:\resimlers\resimlers\deneme1.png). The box lives 60 s on the server; after
// that it is gone. Click an item to take it (the game's own WIZ_ITEM_GET), the header button takes all.
// Skin: build_dropbox_assets.py -> dropbox_ui\*.pus + dropbox_layout.h; item icons pus_ui\icons\<icon>.pus.
//
// Server: GameServer DropBox.cpp, WIZ_HSACS_HOOK 0xE9 / sub DROPBOX 0xF1:
//   op 1 [u32 bundle][u16 seconds][u8 n] n*([u8 slot][u32 item][u16 count][u32 icon])   new box
//   op 2 [u32 bundle][u8 slot]                                                           item taken
//   op 3 [u32 bundle]                                                                    box expired
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include <map>
#include <math.h>
#include "dropbox_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();

static const DWORD KO_SND_FNC = 0x00704070;
static const DWORD KO_PTR_PKT = 0x01115914;
static const BYTE  WIZ_HSACS_HOOK = 0xE9, DB_SUBOP = 0xF1, WIZ_ITEM_GET = 0x26;
static const UINT32 ITEM_GOLD = 900000000;
static const int MAX_ITEMS = 8, MIN_CELLS = 4, MAX_H = DB_HEAD_H + MAX_ITEMS * (DB_CELL + 2) + 4;

struct DbItem { BYTE slot; UINT32 item; UINT16 count; UINT32 icon; bool taken; };
struct DbState { UINT32 bundle = 0; std::vector<DbItem> items; DWORD until = 0, total = 0; };

static CRITICAL_SECTION g_lock, g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQ;
static DbState g_st;
static HWND  g_hWnd = nullptr;
static HDC   g_memDC = nullptr;
static void* g_bits = nullptr;
static int   g_curH = DB_HEAD_H + MIN_CELLS * (DB_CELL + 2) + 4;
static POINT g_pos = { -1, -1 };
static HCURSOR g_gameCursor = nullptr;
static const UINT WM_DB_REFRESH = WM_APP + 71;

// ---------------------------------------------------------------- send
static void QueueSend(const std::vector<BYTE>& p)
{
    EnterCriticalSection(&g_sendLock);
    g_sendQ.push_back(p);
    LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();
}

void DropBox_FlushSendQueue()
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
    }
}

static void Take(UINT32 bundle, const DbItem& it)
{
    std::vector<BYTE> p = { WIZ_ITEM_GET };
    p.insert(p.end(), (BYTE*)&bundle, (BYTE*)&bundle + 4);
    p.insert(p.end(), (BYTE*)&it.item, (BYTE*)&it.item + 4);
    UINT16 slot = it.slot;
    p.insert(p.end(), (BYTE*)&slot, (BYTE*)&slot + 2);
    QueueSend(p);
}

// ---------------------------------------------------------------- sprites
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<std::string, Sprite> g_skin;
static std::map<UINT32, Sprite> g_icons;
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
    for (const char* n : { "cell", "head", "take_n", "take_o", "take_d" })
    {
        char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\dropbox_ui\\%s.pus", g_dir, n);
        Sprite s; if (LoadPus(p, s)) g_skin[n] = s;
    }
}

static Sprite* Icon(UINT32 icon)
{
    if (!icon) return nullptr;
    auto it = g_icons.find(icon);
    if (it != g_icons.end()) return it->second.dc ? &it->second : nullptr;
    char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\pus_ui\\icons\\%u.pus", g_dir, icon);
    Sprite s; if (!LoadPus(p, s)) s = Sprite();
    g_icons[icon] = s;
    return s.dc ? &g_icons[icon] : nullptr;
}

static void Blit(Sprite* s, int x, int y, int w = -1, int h = -1)
{
    if (!s || !s->dc) return;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_memDC, x, y, w < 0 ? s->w : w, h < 0 ? s->h : h, s->dc, 0, 0, s->w, s->h, bf);
}
static void Skin(const char* n, int x, int y) { auto it = g_skin.find(n); if (it != g_skin.end()) Blit(&it->second, x, y); }

static void Text(const std::string& s, RECT r, COLORREF color, int pt, UINT fmt)
{
    int w = r.right - r.left, h = r.bottom - r.top;
    if (s.empty() || w <= 0 || h <= 0) return;
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
    void* bits = nullptr;
    HDC dc = CreateCompatibleDC(nullptr);
    HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    HGDIOBJ ob = SelectObject(dc, bmp);
    HFONT font = CreateFontA(-MulDiv(pt, 96, 72), 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
    HGDIOBJ of = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
    RECT rc = { 0, 0, w, h };
    DrawTextA(dc, s.c_str(), (int)s.size(), &rc, fmt | DT_SINGLELINE | DT_NOPREFIX);
    GdiFlush();
    for (int pass = 0; pass < 2; pass++)
    {
        const int o = pass == 0 ? 1 : 0;
        const BYTE cr = pass ? GetRValue(color) : 0, cg = pass ? GetGValue(color) : 0, cb = pass ? GetBValue(color) : 0;
        for (int y = 0; y < h; y++)
        {
            int dy = r.top + y + o; if (dy < 0 || dy >= g_curH) continue;
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            for (int x = 0; x < w; x++, src += 4)
            {
                int dx = r.left + x + o; int a = max(src[0], max(src[1], src[2]));
                if (!a || dx < 0 || dx >= DB_W) continue;
                if (!pass) a = a * 3 / 4;
                BYTE* d = (BYTE*)g_bits + (dy * DB_W + dx) * 4;
                d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
            }
        }
    }
    SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}

// ---------------------------------------------------------------- layout / hit test
enum { H_NONE = -1, H_TAKE = 100, H_HEAD = 101 };            // 0..7 = cell index into the visible list
static int g_hover = H_NONE, g_pressed = H_NONE;
static bool g_track = false, g_drag = false;
static POINT g_dragFrom;

static std::vector<int> Visible(const DbState& s)
{
    std::vector<int> v;
    for (int i = 0; i < (int)s.items.size(); i++) if (!s.items[i].taken) v.push_back(i);
    return v;
}
static RECT CellRect(int k) { int y = DB_HEAD_H + 2 + k * (DB_CELL + 2); return { 2, y, 2 + DB_CELL, y + DB_CELL }; }

static int HitTest(int x, int y)
{
    if (x >= DB_TAKE[0] && x < DB_TAKE[2] && y >= DB_TAKE[1] && y < DB_TAKE[3]) return H_TAKE;
    if (y < DB_HEAD_H) return H_HEAD;
    EnterCriticalSection(&g_lock);
    int n = (int)Visible(g_st).size();
    LeaveCriticalSection(&g_lock);
    for (int k = 0; k < n; k++) { RECT r = CellRect(k); if (x >= r.left && x < r.right && y >= r.top && y < r.bottom) return k; }
    return H_NONE;
}

static std::string Short(UINT32 v)
{
    char b[16];
    if (v >= 1000000) sprintf_s(b, "%.1fm", v / 1000000.0); else if (v >= 10000) sprintf_s(b, "%uk", v / 1000); else sprintf_s(b, "%u", v);
    return b;
}

// ---------------------------------------------------------------- rendering
// time left over a cell: translucent red pie from 12 o'clock, clockwise, shrinking as the box gets older
static void TimePie(RECT r, double frac)
{
    if (frac <= 0) return;
    const double cx = (r.left + r.right) / 2.0, cy = (r.top + r.bottom) / 2.0, end = frac * 6.283185307179586;
    const int a = 110;                                         // red, premultiplied src-over
    for (int y = r.top; y < r.bottom; y++)
        for (int x = r.left; x < r.right; x++)
        {
            double ang = atan2(x + 0.5 - cx, -(y + 0.5 - cy));   // 0 at the top, clockwise
            if (ang < 0) ang += 6.283185307179586;
            if (ang > end) continue;
            BYTE* d = (BYTE*)g_bits + (y * DB_W + x) * 4;
            d[0] = (BYTE)(d[0] * (255 - a) / 255);
            d[1] = (BYTE)(d[1] * (255 - a) / 255);
            d[2] = (BYTE)((220 * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
}

static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    LoadSkin();
    EnterCriticalSection(&g_lock);
    DbState s = g_st;
    LeaveCriticalSection(&g_lock);
    std::vector<int> vis = Visible(s);
    int cells = max(MIN_CELLS, (int)vis.size());
    g_curH = DB_HEAD_H + cells * (DB_CELL + 2) + 4;
    memset(g_bits, 0, (size_t)DB_W * MAX_H * 4);

    Skin("head", 0, 0);
    DWORD now = GetTickCount();
    UINT32 left = s.until > now ? (s.until - now + 999) / 1000 : 0;
    char b[32]; sprintf_s(b, "%u", left);
    Text(b, { 4, 2, DB_TAKE[0] - 2, DB_HEAD_H - 2 }, left <= 10 ? RGB(255, 140, 120) : RGB(255, 255, 255), 7, DT_LEFT | DT_VCENTER);
    Skin(g_hover == H_TAKE ? (g_pressed == H_TAKE ? "take_d" : "take_o") : "take_n", DB_TAKE[0], DB_TAKE[1]);
    for (int k = 0; k < cells; k++)
    {
        RECT r = CellRect(k);
        Skin("cell", r.left, r.top);
        if (k >= (int)vis.size()) continue;
        const DbItem& it = s.items[vis[k]];
        if (it.item == ITEM_GOLD)
            Text(Short(it.count), { r.left + 4, r.top + 4, r.right - 4, r.bottom - 4 }, RGB(255, 220, 90), 8, DT_CENTER | DT_VCENTER);
        else
        {
            Blit(Icon(it.icon), r.left + 5, r.top + 5, DB_CELL - 10, DB_CELL - 10);
            if (it.count > 1)
                Text("+" + std::to_string(it.count), { r.left + 5, r.top + 4, r.right - 4, r.top + 18 }, RGB(255, 255, 255), 7, DT_LEFT | DT_TOP);
        }
        if (s.total) TimePie({ r.left + 5, r.top + 5, r.right - 5, r.bottom - 5 }, (double)(s.until > now ? s.until - now : 0) / s.total);
        if (g_hover == k)
        {
            BYTE* p; int a = 40;
            for (int y = r.top + 5; y < r.bottom - 5; y++)
                for (int x = r.left + 5; x < r.right - 5; x++)
                {
                    p = (BYTE*)g_bits + (y * DB_W + x) * 4;
                    p[0] = (BYTE)min(255, p[0] + a); p[1] = (BYTE)min(255, p[1] + a); p[2] = (BYTE)min(255, p[2] + a);
                }
        }
    }

    RECT wr; GetWindowRect(g_hWnd, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { DB_W, g_curH };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
}

// ---------------------------------------------------------------- window plumbing
static HWND FindGameWindow()
{
    struct Ctx { DWORD pid; HWND best; int area; } ctx = { GetCurrentProcessId(), nullptr, 0 };
    EnumWindows([](HWND h, LPARAM lp) -> BOOL {
        Ctx* c = (Ctx*)lp; DWORD pid; GetWindowThreadProcessId(h, &pid);
        if (pid != c->pid || !IsWindowVisible(h) || h == g_hWnd) return TRUE;
        RECT r; GetWindowRect(h, &r); int a = (r.right - r.left) * (r.bottom - r.top);
        if (a > c->area) { c->area = a; c->best = h; }
        return TRUE;
    }, (LPARAM)&ctx);
    return ctx.best;
}

static bool Active()
{
    EnterCriticalSection(&g_lock);
    bool on = g_st.bundle && GetTickCount() < g_st.until && !Visible(g_st).empty();
    LeaveCriticalSection(&g_lock);
    return on;
}

static void SyncVisibility()
{
    HWND game = FindGameWindow();
    DWORD pid = 0; GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool want = Active() && game && !IsIconic(game) && pid == GetCurrentProcessId();
    if (want)
    {
        if (g_pos.x < 0)
        {
            RECT gr; GetClientRect(game, &gr); MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
            g_pos.x = gr.left + (gr.right - gr.left) * 77 / 100;          // where the reference shows it
            g_pos.y = gr.top + (gr.bottom - gr.top) * 38 / 100;
        }
        if (!IsWindowVisible(g_hWnd))
            SetWindowPos(g_hWnd, HWND_TOPMOST, g_pos.x, g_pos.y, DB_W, MAX_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        Paint();
    }
    else if (IsWindowVisible(g_hWnd)) ShowWindow(g_hWnd, SW_HIDE);
    POINT p; CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) && (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursor = ci.hCursor;
}

static void Activate(int hit)
{
    EnterCriticalSection(&g_lock);
    DbState s = g_st;
    LeaveCriticalSection(&g_lock);
    std::vector<int> vis = Visible(s);
    if (hit == H_TAKE) { for (int i : vis) Take(s.bundle, s.items[i]); }
    else if (hit >= 0 && hit < (int)vis.size()) Take(s.bundle, s.items[vis[hit]]);
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
            RECT wr; GetWindowRect(h, &wr);
            g_pos.x = wr.left + p.x - g_dragFrom.x; g_pos.y = wr.top + p.y - g_dragFrom.y;
            SetWindowPos(h, nullptr, g_pos.x, g_pos.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
            g_dragFrom = p;
            return 0;
        }
        if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
        if (int hit = HitTest(x, y); hit != g_hover) { g_hover = hit; Paint(); }
        return 0;
    case WM_MOUSELEAVE: g_track = false; g_hover = H_NONE; Paint(); return 0;
    case WM_LBUTTONDOWN:
    {
        int hit = HitTest(x, y);
        if (hit == H_HEAD) { g_drag = true; GetCursorPos(&g_dragFrom); SetCapture(h); return 0; }
        g_pressed = hit; if (hit != H_NONE) SetCapture(h);
        Paint(); return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (g_drag) { g_drag = false; return 0; }
        int hit = HitTest(x, y), pressed = g_pressed; g_pressed = H_NONE;
        if (pressed != H_NONE && pressed == hit) Activate(hit);
        Paint(); return 0;
    }
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: SyncVisibility(); return 0;
    case WM_DB_REFRESH: SyncVisibility(); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThread(LPVOID)
{
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW); wc.lpszClassName = L"NTT_DropBox";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Drop Box", WS_POPUP, 0, 0, DB_W, MAX_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("DROPBOX: window creation failed"); return 0; }
    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), DB_W, -MAX_H, 1, 32, BI_RGB };
    SelectObject(g_memDC, CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
    Log("DROPBOX: window ready");
    SetTimer(g_hWnd, 1, 100, nullptr);                     // smooth pie
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

void DropBox_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[0] != WIZ_HSACS_HOOK || buf[1] != DB_SUBOP) return;
    size_t pos = 3;
    UINT32 bundle = 0;
    if (!Get(buf, len, pos, bundle)) return;
    EnterCriticalSection(&g_lock);
    if (buf[2] == 1)
    {
        UINT16 secs = 0; BYTE n = 0;
        if (Get(buf, len, pos, secs) && Get(buf, len, pos, n))
        {
            DbState s; s.bundle = bundle; s.total = secs * 1000u; s.until = GetTickCount() + s.total;
            for (BYTE i = 0; i < n && i < MAX_ITEMS; i++)
            {
                DbItem it = {};
                if (!(Get(buf, len, pos, it.slot) && Get(buf, len, pos, it.item) && Get(buf, len, pos, it.count) && Get(buf, len, pos, it.icon))) break;
                s.items.push_back(it);
            }
            if (!s.items.empty()) g_st = s;               // the last box replaces the previous one
        }
    }
    else if (buf[2] == 2 && bundle == g_st.bundle)
    {
        BYTE slot = 0;
        if (Get(buf, len, pos, slot))
            for (auto& it : g_st.items) if (it.slot == slot) it.taken = true;
    }
    else if (buf[2] == 3 && bundle == g_st.bundle) g_st = DbState();
    LeaveCriticalSection(&g_lock);
    if (g_hWnd) PostMessageW(g_hWnd, WM_DB_REFRESH, 0, 0);
}

void DropBox_Init()
{
    InitializeCriticalSection(&g_lock);
    InitializeCriticalSection(&g_sendLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

bool DropBox_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
