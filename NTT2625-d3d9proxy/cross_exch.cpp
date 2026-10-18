// "Cross Exchange" for the 2625 inventory (T-GUARD style): a button between the VIP vault chest and the
// search magnifier of the native inventory window. Pressing it marks every inventory item that can be
// exchanged (Right-Click Exchange coupons, server list from rce_store.cpp); clicking a marked item opens the
// exchange panel. Right-click or the button again leaves the mode.
// Two GDI layered windows that follow the native inventory (polled every 100 ms):
//   - the button (shown while the inventory is open)
//   - the bag overlay (only in cross mode): gold frames on exchangeable slots, other slots dimmed.
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
bool Rce_InventoryVisible();
bool Rce_InvSlotRect(int i, RECT* rc);
bool Rce_SlotExchangeable(int i, UINT32* item);
void Rce_OpenSlot(int i);
void Rce_OpenCross();
void* Rce_Inventory();

// The button itself is now a native control of re_inventory.uif ('btn_cross', build_cross_btn_uif.py): the game
// draws it under its tooltips. Its click reaches the inventory's ReceiveMessage (vtable slot 33, msg 1 =
// UIMSG_BUTTON_CLICK); that vtable entry is redirected here once the inventory object exists.
namespace
{
    const int SLOT_RM = 33;
    const DWORD UIMSG_BUTTON_CLICK = 1;
    typedef bool(__thiscall* FnRM)(void*, void*, DWORD);
    FnRM g_origInvRM = nullptr;
    void** g_invVt = nullptr;

    const char* IdOfUi(void* o)
    {
        const BYTE* s = (const BYTE*)o + 0x58;   // std::string id (MSVC SSO)
        return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s;
    }

    bool __fastcall HkInvRM(void* self, void*, void* sender, DWORD msg)
    {
        __try
        {
            if (sender && msg == UIMSG_BUTTON_CLICK && _stricmp(IdOfUi(sender), "btn_cross") == 0)
            {
                Log("CROSS: native button -> open Cross Exchange");
                Rce_OpenCross();
                return true;
            }
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
        return g_origInvRM(self, sender, msg);
    }

    void HookInventoryRM()
    {
        if (g_origInvRM) return;
        void* inv = Rce_Inventory();
        if (!inv) return;
        __try
        {
            void** vt = *(void***)inv;
            if (!vt) return;
            DWORD old;
            if (!VirtualProtect(&vt[SLOT_RM], 4, PAGE_READWRITE, &old)) return;
            g_origInvRM = (FnRM)vt[SLOT_RM];
            InterlockedExchange((volatile LONG*)&vt[SLOT_RM], (LONG)(DWORD)&HkInvRM);
            VirtualProtect(&vt[SLOT_RM], 4, old, &old);
            g_invVt = vt;
            char b[96]; sprintf_s(b, "CROSS: inventory ReceiveMessage hooked (vt %p, orig %p)", vt, g_origInvRM); Log(b);
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { g_origInvRM = nullptr; }
    }
}

static const int SLOTS = 28;
// re_inventory.uif coordinates: inventory slot 0 (AREA "0" of area_inv) and the free strip between
// Btn_VipVault [242,276,271,305] and Btn_search [390,282,414,306]
static const RECT UIF_SLOT0 = { 236, 373, 279, 416 };
static const RECT UIF_BUTTON = { 276, 279, 386, 305 };

static HWND g_btn = nullptr, g_ovl = nullptr;
static HDC g_btnDC = nullptr, g_ovlDC = nullptr;
static void* g_btnBits = nullptr; static void* g_ovlBits = nullptr;
static int g_btnW = UIF_BUTTON.right - UIF_BUTTON.left, g_btnH = UIF_BUTTON.bottom - UIF_BUTTON.top;
static const int OVL_MAXW = 600, OVL_MAXH = 400;
static int g_ovlW = 0, g_ovlH = 0;
static bool g_mode = false, g_btnHover = false, g_btnDown = false, g_tracking = false;
static RECT g_slot[SLOTS]; static bool g_slotOk[SLOTS]; static bool g_slotEx[SLOTS];
static POINT g_ovlOrigin;   // screen position of the overlay's top-left
static int g_hoverSlot = -1;

// ---------------------------------------------------------------- drawing into a premultiplied BGRA buffer
static void Fill(void* bits, int bw, int bh, RECT r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, (int)r.left), t = max(0, (int)r.top), rr = min(bw, (int)r.right), bb = min(bh, (int)r.bottom);
    for (int y = t; y < bb; y++)
    {
        BYTE* d = (BYTE*)bits + (y * bw + l) * 4;
        for (int x = l; x < rr; x++, d += 4)
        {
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}
static void Frame(void* bits, int bw, int bh, RECT r, BYTE cr, BYTE cg, BYTE cb, int th)
{
    Fill(bits, bw, bh, { r.left, r.top, r.right, r.top + th }, cr, cg, cb, 255);
    Fill(bits, bw, bh, { r.left, r.bottom - th, r.right, r.bottom }, cr, cg, cb, 255);
    Fill(bits, bw, bh, { r.left, r.top, r.left + th, r.bottom }, cr, cg, cb, 255);
    Fill(bits, bw, bh, { r.right - th, r.top, r.right, r.bottom }, cr, cg, cb, 255);
}
static void Text(void* bits, int bw, int bh, const char* s, RECT r, COLORREF color, int pt, bool bold)
{
    int w = r.right - r.left, h = r.bottom - r.top;
    if (w <= 0 || h <= 0) return;
    BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
    void* tb = nullptr;
    HDC dc = CreateCompatibleDC(nullptr);
    HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &tb, nullptr, 0);
    HGDIOBJ ob = SelectObject(dc, bmp);
    HFONT font = CreateFontA(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
    HGDIOBJ of = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
    RECT rc = { 0, 0, w, h };
    DrawTextA(dc, s, -1, &rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    GdiFlush();
    BYTE cr = GetRValue(color), cg = GetGValue(color), cb = GetBValue(color);
    for (int y = 0; y < h; y++)
    {
        int dy = r.top + y; if (dy < 0 || dy >= bh) continue;
        const BYTE* src = (const BYTE*)tb + y * w * 4;
        BYTE* d = (BYTE*)bits + (dy * bw + r.left) * 4;
        for (int x = 0; x < w; x++, src += 4, d += 4)
        {
            int a = max(src[0], max(src[1], src[2]));
            if (!a || r.left + x < 0 || r.left + x >= bw) continue;
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
    SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}
static void Present(HWND h, HDC dc, int w, int hgt, POINT at)
{
    POINT src = { 0, 0 }; SIZE sz = { w, hgt };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(h, nullptr, &at, &sz, dc, &src, 0, &bf, ULW_ALPHA);
}

static void PaintButton(POINT at)
{
    memset(g_btnBits, 0, (size_t)g_btnW * g_btnH * 4);
    RECT r = { 0, 0, g_btnW, g_btnH };
    BYTE base = g_btnDown ? 130 : g_btnHover ? 110 : 82;
    Fill(g_btnBits, g_btnW, g_btnH, r, g_mode ? 40 : base, g_mode ? 90 : 26, g_mode ? 40 : 26, 245);
    Frame(g_btnBits, g_btnW, g_btnH, r, 196, 158, 82, 1);
    Frame(g_btnBits, g_btnW, g_btnH, { 2, 2, g_btnW - 2, g_btnH - 2 }, 70, 52, 22, 1);
    Text(g_btnBits, g_btnW, g_btnH, "Cross Exchange", r, g_mode ? RGB(170, 255, 170) : RGB(255, 255, 255), 8, true);
    Present(g_btn, g_btnDC, g_btnW, g_btnH, at);
}

static void PaintOverlay()
{
    memset(g_ovlBits, 0, (size_t)OVL_MAXW * OVL_MAXH * 4);
    for (int i = 0; i < SLOTS; i++)
    {
        if (!g_slotOk[i]) continue;
        RECT s = { g_slot[i].left - g_ovlOrigin.x, g_slot[i].top - g_ovlOrigin.y, g_slot[i].right - g_ovlOrigin.x, g_slot[i].bottom - g_ovlOrigin.y };
        if (g_slotEx[i])
        {
            Fill(g_ovlBits, g_ovlW, g_ovlH, s, 255, 200, 60, i == g_hoverSlot ? 70 : 30);
            Frame(g_ovlBits, g_ovlW, g_ovlH, s, 255, 210, 80, i == g_hoverSlot ? 3 : 2);
            RECT badge = { s.right - 16, s.top + 1, s.right - 1, s.top + 14 };
            Fill(g_ovlBits, g_ovlW, g_ovlH, badge, 30, 120, 40, 230);
            Text(g_ovlBits, g_ovlW, g_ovlH, "<>", badge, RGB(255, 255, 255), 6, true);
        }
        else
            Fill(g_ovlBits, g_ovlW, g_ovlH, s, 0, 0, 0, 150);
    }
    Present(g_ovl, g_ovlDC, g_ovlW, g_ovlH, g_ovlOrigin);
}

static HWND FindGameWindow()
{
    struct Ctx { DWORD pid; HWND best; int area; } ctx = { GetCurrentProcessId(), nullptr, 0 };
    EnumWindows([](HWND h, LPARAM lp) -> BOOL {
        Ctx* c = (Ctx*)lp; DWORD pid; GetWindowThreadProcessId(h, &pid);
        if (pid != c->pid || !IsWindowVisible(h) || h == g_btn || h == g_ovl) return TRUE;
        RECT r; GetWindowRect(h, &r); int a = (r.right - r.left) * (r.bottom - r.top);
        if (a > c->area) { c->area = a; c->best = h; }
        return TRUE;
    }, (LPARAM)&ctx);
    return ctx.best;
}

// poll: place the button/overlay over the native inventory
static void Sync()
{
    HookInventoryRM();
    if (IsWindowVisible(g_btn)) ShowWindow(g_btn, SW_HIDE);
    static HWND game = nullptr;
    static DWORD lastFind = 0;
    if (!game || !IsWindow(game) || GetTickCount() - lastFind > 3000) { game = FindGameWindow(); lastFind = GetTickCount(); }
    DWORD pid = 0; GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool show = game && !IsIconic(game) && pid == GetCurrentProcessId() && Rce_InventoryVisible();

    RECT s0;
    if (show) show = Rce_InvSlotRect(0, &s0);
    if (!show)
    {
        if (IsWindowVisible(g_btn)) ShowWindow(g_btn, SW_HIDE);
        if (IsWindowVisible(g_ovl)) ShowWindow(g_ovl, SW_HIDE);
        return;
    }
    POINT origin = { 0, 0 };
    ClientToScreen(game, &origin);

    if (!g_mode) { if (IsWindowVisible(g_ovl)) ShowWindow(g_ovl, SW_HIDE); return; }
    RECT u = { LONG_MAX, LONG_MAX, LONG_MIN, LONG_MIN };
    for (int i = 0; i < SLOTS; i++)
    {
        RECT r;
        g_slotOk[i] = Rce_InvSlotRect(i, &r);
        if (!g_slotOk[i]) continue;
        g_slot[i] = { r.left + origin.x, r.top + origin.y, r.right + origin.x, r.bottom + origin.y };
        g_slotEx[i] = Rce_SlotExchangeable(i, nullptr);
        u.left = min(u.left, g_slot[i].left); u.top = min(u.top, g_slot[i].top);
        u.right = max(u.right, g_slot[i].right); u.bottom = max(u.bottom, g_slot[i].bottom);
    }
    if (u.left >= u.right) return;
    g_ovlOrigin = { u.left - 4, u.top - 4 };
    g_ovlW = min(OVL_MAXW, (int)(u.right - u.left) + 8); g_ovlH = min(OVL_MAXH, (int)(u.bottom - u.top) + 8);
    SetWindowPos(g_ovl, HWND_TOPMOST, g_ovlOrigin.x, g_ovlOrigin.y, g_ovlW, g_ovlH, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    PaintOverlay();
}

static void SetMode(bool on)
{
    g_mode = on;
    int n = 0;
    for (int i = 0; i < SLOTS; i++) if (Rce_SlotExchangeable(i, nullptr)) n++;
    char b[96]; sprintf_s(b, "CROSS: mode %s (%d exchangeable item%s)", on ? "on" : "off", n, n == 1 ? "" : "s"); Log(b);
    Sync();
}

static int SlotAt(POINT screen)
{
    for (int i = 0; i < SLOTS; i++)
        if (g_slotOk[i] && PtInRect(&g_slot[i], screen)) return i;
    return -1;
}

static HCURSOR g_cursor = nullptr;
static LRESULT CALLBACK BtnProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    switch (msg)
    {
    case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
    case WM_MOUSEMOVE:
        if (!g_tracking) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_tracking = TrackMouseEvent(&t) != FALSE; }
        if (!g_btnHover) { g_btnHover = true; Sync(); }
        return 0;
    case WM_MOUSELEAVE: g_tracking = false; g_btnHover = g_btnDown = false; Sync(); return 0;
    case WM_LBUTTONDOWN: g_btnDown = true; SetCapture(h); Sync(); return 0;
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        bool was = g_btnDown; g_btnDown = false;
        POINT p; GetCursorPos(&p); RECT wr; GetWindowRect(h, &wr);
        if (was && PtInRect(&wr, p)) { SetMode(false); Rce_OpenCross(); }   // reference: the button opens the Cross Exchange window
        else Sync();
        return 0;
    }
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_cursor ? g_cursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: Sync(); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static LRESULT CALLBACK OvlProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    switch (msg)
    {
    case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
    case WM_MOUSEMOVE:
    {
        POINT p; GetCursorPos(&p);
        int s = SlotAt(p);
        if (s >= 0 && !g_slotEx[s]) s = -1;
        if (s != g_hoverSlot) { g_hoverSlot = s; PaintOverlay(); }
        return 0;
    }
    case WM_LBUTTONUP:
    {
        POINT p; GetCursorPos(&p);
        int s = SlotAt(p);
        if (s >= 0 && g_slotEx[s]) { Rce_OpenSlot(s); char b[64]; sprintf_s(b, "CROSS: open exchange slot %d", s); Log(b); }
        return 0;
    }
    case WM_RBUTTONUP: SetMode(false); return 0;
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_cursor ? g_cursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static HWND MakeLayered(const wchar_t* cls, WNDPROC proc, int w, int h, HDC* dc, void** bits)
{
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = proc; wc.hInstance = GetModuleHandleW(nullptr); wc.hCursor = LoadCursor(nullptr, IDC_ARROW); wc.lpszClassName = cls;
    RegisterClassExW(&wc);
    HWND hw = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, cls, cls, WS_POPUP, 0, 0, w, h, nullptr, nullptr, wc.hInstance, nullptr);
    *dc = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
    SelectObject(*dc, CreateDIBSection(*dc, &bmi, DIB_RGB_COLORS, bits, nullptr, 0));
    return hw;
}

static DWORD WINAPI WindowThread(LPVOID)
{
    g_btn = MakeLayered(L"NTT_CrossBtn", BtnProc, g_btnW, g_btnH, &g_btnDC, &g_btnBits);
    g_ovl = MakeLayered(L"NTT_CrossOvl", OvlProc, OVL_MAXW, OVL_MAXH, &g_ovlDC, &g_ovlBits);
    if (!g_btn || !g_ovl) { Log("CROSS: window creation failed"); return 0; }
    SetTimer(g_btn, 1, 100, nullptr);
    Log("CROSS: ready");
    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// pus_store.cpp GetAsyncKeyState hook: clicks on our windows must not reach the game
bool Cross_CursorOverPanel()
{
    POINT p;
    if (!GetCursorPos(&p)) return false;
    HWND w = WindowFromPoint(p);
    return w && ((w == g_btn && IsWindowVisible(g_btn)) || (w == g_ovl && IsWindowVisible(g_ovl)));
}

void Cross_Init()
{
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}
