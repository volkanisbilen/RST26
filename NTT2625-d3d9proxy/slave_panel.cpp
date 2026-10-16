// Slave Merchant window (slavemerchant.uif -> slave_ui\*.pus, layout slave_layout.h, built by build_slave_uif.py) and its
// top-bar icon. The server (slave_merchant.rs, hook sub-op 0xDD) sends the icon flag once at game entry and the window
// state on request:
//   S2C [0][u8 show]   S2C [1][u8 active][u32 minutes][u32 gold][u32 kc] 12 x [u32 item][u32 icon][u16 cnt] (stall)
//                                                                      28 x [u32 item][u32 icon][u16 cnt] (bag)
//   C2S [1] refresh  [2] take KC  [3] take gold  [4] close the stall  [5] use the scroll (open a stall)
// The icon sits at the right of the game window (JstKO\slave_icon.ini: [Slave] OffsetRight / Top, defaults 130 / 165).
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <map>
#include "slave_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
HWND Proxy_GameWnd();
void Drop_SendRaw(const std::vector<BYTE>& p);
void ItemTip_Show(UINT32 item, RECT anchor);
void ItemTip_Hide();

namespace
{
    const BYTE SUB = 0xDD;
    const UINT WM_SM_REFRESH = WM_APP + 97, WM_SM_TOGGLE = WM_APP + 98;

    struct Cell { UINT32 item = 0, icon = 0; UINT16 cnt = 0; };
    CRITICAL_SECTION g_lock;
    bool g_show = false, g_active = false;
    UINT32 g_minutes = 0, g_gold = 0, g_kc = 0;
    Cell g_stall[12], g_bag[28];

    struct Sprite { HDC dc = nullptr; int w = 0, h = 0; };
    std::map<std::string, Sprite> g_spr;
    std::map<UINT32, Sprite> g_icons;
    char g_dir[MAX_PATH] = "";

    bool LoadPus(const char* path, Sprite& s)
    {
        FILE* f = nullptr;
        if (fopen_s(&f, path, "rb") != 0 || !f) return false;
        char magic[4]; UINT32 w = 0, h = 0; bool ok = false;
        if (fread(magic, 1, 4, f) == 4 && !memcmp(magic, "PUSI", 4) && fread(&w, 4, 1, f) == 1 && fread(&h, 4, 1, f) == 1 && w && h && w < 4096 && h < 4096)
        {
            BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), (LONG)w, -(LONG)h, 1, 32, BI_RGB };
            void* bits = nullptr; s.dc = CreateCompatibleDC(nullptr);
            HBITMAP bmp = CreateDIBSection(s.dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
            ok = bmp && fread(bits, 4, (size_t)w * h, f) == (size_t)w * h;
            if (ok) { SelectObject(s.dc, bmp); s.w = (int)w; s.h = (int)h; }
        }
        fclose(f);
        return ok;
    }
    Sprite* Spr(const char* name)
    {
        auto it = g_spr.find(name);
        if (it != g_spr.end()) return it->second.dc ? &it->second : nullptr;
        Sprite s; char p[MAX_PATH]; sprintf_s(p, "%s\\slave_ui\\%s.pus", g_dir, name);
        if (!LoadPus(p, s)) { char m[MAX_PATH + 32]; sprintf_s(m, "SLAVEMERC: missing %s", p); Log(m); s = Sprite(); }
        g_spr[name] = s;
        return s.dc ? &g_spr[name] : nullptr;
    }
    Sprite* Icon(UINT32 icon)
    {
        auto it = g_icons.find(icon);
        if (it == g_icons.end())
        {
            Sprite s; char p[MAX_PATH]; sprintf_s(p, "%s\\pus_ui\\icons\\%u.pus", g_dir, icon);
            if (!icon || !LoadPus(p, s)) { sprintf_s(p, "%s\\pus_ui\\noimage.pus", g_dir); LoadPus(p, s); }
            it = g_icons.emplace(icon, s).first;
        }
        return it->second.dc ? &it->second : nullptr;
    }

    // ---------------------------------------------------------------- panel window
    HWND g_hWnd = nullptr, g_hIcon = nullptr;
    HDC g_dc = nullptr; void* g_bits = nullptr;
    HDC g_idc = nullptr; void* g_ibits = nullptr;
    bool g_open = false, g_track = false, g_dragging = false, g_iconHover = false, g_iconTrack = false;
    POINT g_dragFrom;
    int g_hover = 0, g_pressed = 0, g_tipCell = -1;
    DWORD g_lastReq = 0;
    enum { C_NONE, C_CLOSE, C_USE, C_CLOSEM, C_CASH, C_GOLD };

    bool In(const SmRect& r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

    void Blit(Sprite* s, int x, int y, int w = -1, int h = -1)
    {
        if (!s) return;
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(g_dc, x, y, w < 0 ? s->w : w, h < 0 ? s->h : h, s->dc, 0, 0, s->w, s->h, bf);
    }

    void Text(const std::string& s, SmRect r, COLORREF color, int pt, UINT fmt, bool bold = false)
    {
        int w = r.r - r.l, h = r.b - r.t;
        if (s.empty() || w <= 0 || h <= 0) return;
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
        void* bits = nullptr; HDC dc = CreateCompatibleDC(nullptr);
        HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        HGDIOBJ ob = SelectObject(dc, bmp);
        HFONT font = CreateFontA(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
        HGDIOBJ of = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
        RECT rc = { 0, 0, w, h };
        DrawTextA(dc, s.c_str(), (int)s.size(), &rc, fmt | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
        GdiFlush();
        const BYTE cr = GetRValue(color), cg = GetGValue(color), cb = GetBValue(color);
        for (int y = 0; y < h; y++)
        {
            int dy = r.t + y; if (dy < 0 || dy >= SM_H) continue;
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            BYTE* dst = (BYTE*)g_bits + (dy * SM_W + r.l) * 4;
            for (int x = 0; x < w; x++, src += 4, dst += 4)
            {
                int a = max(src[0], max(src[1], src[2]));
                if (!a || r.l + x < 0 || r.l + x >= SM_W) continue;
                dst[0] = (BYTE)((cb * a + dst[0] * (255 - a)) / 255);
                dst[1] = (BYTE)((cg * a + dst[1] * (255 - a)) / 255);
                dst[2] = (BYTE)((cr * a + dst[2] * (255 - a)) / 255);
                dst[3] = (BYTE)(a + dst[3] * (255 - a) / 255);
            }
        }
        SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }

    std::string Num(UINT32 v)
    {
        std::string s = std::to_string(v);
        for (int i = (int)s.size() - 3; i > 0; i -= 3) s.insert(i, ".");
        return s;
    }

    int Hit(int x, int y)
    {
        if (In(SM_CLOSE, x, y)) return C_CLOSE;
        if (In(SM_USE, x, y)) return C_USE;
        if (In(SM_CLOSEM, x, y)) return C_CLOSEM;
        if (In(SM_CASH, x, y)) return C_CASH;
        if (In(SM_GOLD, x, y)) return C_GOLD;
        return C_NONE;
    }

    void DrawBtn(const char* prefix, SmRect r, int ctl, bool disabled = false)
    {
        char n[32];
        int st = disabled ? 3 : (g_pressed == ctl && g_hover == ctl ? 1 : (g_hover == ctl ? 2 : 0));
        sprintf_s(n, "%s_%c", prefix, "ndox"[st]);
        Blit(Spr(n), r.l, r.t, r.r - r.l, r.b - r.t);
    }

    void DrawCell(const Cell& c, const SmRect& r)
    {
        if (!c.item) return;
        int side = min(r.r - r.l, r.b - r.t) - 2;
        Blit(Icon(c.icon), r.l + (r.r - r.l - side) / 2, r.t + (r.b - r.t - side) / 2, side, side);
        if (c.cnt > 1)
            Text(std::to_string(c.cnt), { r.l, r.b - 14, r.r - 2, r.b - 1 }, RGB(255, 255, 255), 8, DT_RIGHT | DT_VCENTER, true);
    }

    void Paint()
    {
        if (!g_hWnd || !g_dc) return;
        memset(g_bits, 0, (size_t)SM_W * SM_H * 4);
        Blit(Spr("bg"), 0, 0);
        EnterCriticalSection(&g_lock);
        Text(g_active ? "Slave merchant is open" : "Waiting for direct", SM_STATE, RGB(255, 255, 255), 9, DT_LEFT | DT_VCENTER);
        Text(std::to_string(g_minutes) + " minutes", SM_DELAY, RGB(255, 255, 255), 9, DT_LEFT | DT_VCENTER);
        Text(Num(g_kc), SM_CASHTXT, RGB(255, 235, 120), 9, DT_LEFT | DT_VCENTER, true);
        Text(Num(g_gold), SM_GOLDTXT, RGB(255, 235, 120), 9, DT_LEFT | DT_VCENTER, true);
        for (int i = 0; i < 12; i++) DrawCell(g_stall[i], SM_STALL[i]);
        for (int i = 0; i < 28; i++) DrawCell(g_bag[i], SM_BAG[i]);
        bool active = g_active;
        LeaveCriticalSection(&g_lock);
        DrawBtn("close", SM_CLOSE, C_CLOSE);
        DrawBtn("use", SM_USE, C_USE, active);
        Text("Use Slave Scroll", SM_USE, RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER, true);
        DrawBtn("closem", SM_CLOSEM, C_CLOSEM, !active);
        Text("Close Merchant", SM_CLOSEM, RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER, true);
        DrawBtn("cash", SM_CASH, C_CASH);
        DrawBtn("gold", SM_GOLD, C_GOLD);
        RECT wr; GetWindowRect(g_hWnd, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { SM_W, SM_H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_dc, &src, 0, &bf, ULW_ALPHA);
    }

    void PaintIcon()
    {
        if (!g_hIcon) return;
        memset(g_ibits, 0, (size_t)SM_ICON_W * SM_ICON_H * 4);
        Sprite* s = Spr(g_iconHover ? "icon_o" : "icon_n");
        if (s)
        {
            BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
            AlphaBlend(g_idc, 0, 0, s->w, s->h, s->dc, 0, 0, s->w, s->h, bf);
        }
        POINT src = { 0, 0 }; SIZE sz = { SM_ICON_W, SM_ICON_H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hIcon, nullptr, nullptr, &sz, g_idc, &src, 0, &bf, ULW_ALPHA);
    }

    void Request(BYTE op) { Drop_SendRaw({ 0xE9, SUB, op }); g_lastReq = GetTickCount(); }

    void OpenPanel()
    {
        HWND game = Proxy_GameWnd();
        RECT gr = { 0, 0, SM_W, SM_H }; if (game) GetWindowRect(game, &gr);
        int x = gr.left + max(0, (int)(gr.right - gr.left - SM_W) / 2), y = gr.top + max(0, (int)(gr.bottom - gr.top - SM_H) / 2);
        SetWindowPos(g_hWnd, HWND_TOPMOST, x, y, SM_W, SM_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        g_open = true;
        Request(1);
        Paint();
    }
    void ClosePanel() { g_open = false; ItemTip_Hide(); g_tipCell = -1; ShowWindow(g_hWnd, SW_HIDE); }

    void Sync()
    {
        HWND game = Proxy_GameWnd(); DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        bool front = game && !IsIconic(game) && pid == GetCurrentProcessId();
        EnterCriticalSection(&g_lock); bool show = g_show; LeaveCriticalSection(&g_lock);
        // the icon
        if (show && front)
        {
            char ini[MAX_PATH]; sprintf_s(ini, "%s\\slave_icon.ini", g_dir);
            int offR = GetPrivateProfileIntA("Slave", "OffsetRight", 130, ini), top = GetPrivateProfileIntA("Slave", "Top", 165, ini);
            RECT gr; GetClientRect(game, &gr); MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
            int x = gr.right - offR - SM_ICON_W, y = gr.top + top;
            RECT wr; GetWindowRect(g_hIcon, &wr);
            if (!IsWindowVisible(g_hIcon) || wr.left != x || wr.top != y)
            {
                SetWindowPos(g_hIcon, HWND_TOPMOST, x, y, SM_ICON_W, SM_ICON_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
                PaintIcon();
            }
        }
        else if (IsWindowVisible(g_hIcon)) ShowWindow(g_hIcon, SW_HIDE);
        // the window follows the game being in front
        if (g_open && !front) { if (IsWindowVisible(g_hWnd)) ShowWindow(g_hWnd, SW_HIDE); }
        else if (g_open && !IsWindowVisible(g_hWnd)) { SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); Paint(); }
        if (g_open && IsWindowVisible(g_hWnd) && GetTickCount() - g_lastReq > 3000) Request(1);
    }

    int CellAt(int x, int y, UINT32* item)
    {
        for (int i = 0; i < 12; i++) if (In(SM_STALL[i], x, y)) { *item = g_stall[i].item; return i; }
        for (int i = 0; i < 28; i++) if (In(SM_BAG[i], x, y)) { *item = g_bag[i].item; return 12 + i; }
        return -1;
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_MOUSEMOVE:
            if (g_dragging)
            {
                POINT p; GetCursorPos(&p); RECT wr; GetWindowRect(h, &wr);
                SetWindowPos(h, nullptr, wr.left + p.x - g_dragFrom.x, wr.top + p.y - g_dragFrom.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                g_dragFrom = p; return 0;
            }
            if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
            if (int hit = Hit(x, y); hit != g_hover) { g_hover = hit; Paint(); }
            {
                UINT32 item = 0; int cell = CellAt(x, y, &item);
                if (!item) cell = -1;
                if (cell != g_tipCell)
                {
                    g_tipCell = cell;
                    if (cell >= 0)
                    {
                        RECT wr; GetWindowRect(h, &wr);
                        const SmRect& r = cell < 12 ? SM_STALL[cell] : SM_BAG[cell - 12];
                        RECT a = { wr.left + r.l, wr.top + r.t, wr.left + r.r, wr.top + r.b };
                        ItemTip_Show(item, a);
                    }
                    else ItemTip_Hide();
                }
            }
            return 0;
        case WM_MOUSELEAVE: g_track = false; g_hover = 0; if (g_tipCell >= 0) { g_tipCell = -1; ItemTip_Hide(); } Paint(); return 0;
        case WM_LBUTTONDOWN:
        {
            int hit = Hit(x, y);
            if (!hit && y < 56) { g_dragging = true; GetCursorPos(&g_dragFrom); SetCapture(h); return 0; }
            g_pressed = hit; if (hit) { SetCapture(h); Paint(); }
            return 0;
        }
        case WM_LBUTTONUP:
        {
            ReleaseCapture();
            if (g_dragging) { g_dragging = false; return 0; }
            int hit = Hit(x, y), pr = g_pressed; g_pressed = 0;
            if (pr && pr == hit)
            {
                EnterCriticalSection(&g_lock); bool active = g_active; LeaveCriticalSection(&g_lock);
                if (hit == C_CLOSE) ClosePanel();
                else if (hit == C_USE && !active) Request(5);
                else if (hit == C_CLOSEM && active) Request(4);
                else if (hit == C_CASH) Request(2);
                else if (hit == C_GOLD) Request(3);
            }
            Paint();
            return 0;
        }
        case WM_SETCURSOR: SetCursor(LoadCursor(nullptr, IDC_ARROW)); return TRUE;
        case WM_TIMER: Sync(); return 0;
        case WM_SM_REFRESH: if (IsWindowVisible(h)) Paint(); return 0;
        case WM_SM_TOGGLE: if (g_open) ClosePanel(); else OpenPanel(); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    LRESULT CALLBACK IconProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_MOUSEMOVE:
            if (!g_iconTrack) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_iconTrack = TrackMouseEvent(&t) != FALSE; }
            if (!g_iconHover) { g_iconHover = true; PaintIcon(); }
            return 0;
        case WM_MOUSELEAVE: g_iconTrack = false; g_iconHover = false; PaintIcon(); return 0;
        case WM_LBUTTONUP: PostMessageW(g_hWnd, WM_SM_TOGGLE, 0, 0); return 0;
        case WM_SETCURSOR: SetCursor(LoadCursor(nullptr, IDC_ARROW)); return TRUE;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        GetModuleFileNameA(nullptr, g_dir, MAX_PATH);
        strcpy_s(strrchr(g_dir, '\\') + 1, 16, "JstKO");
        HINSTANCE hi = GetModuleHandleW(nullptr);
        WNDCLASSEXW wc = { sizeof(wc) }; wc.lpfnWndProc = WndProc; wc.hInstance = hi; wc.lpszClassName = L"NTT_SlaveMerc";
        WNDCLASSEXW ic = { sizeof(ic) }; ic.lpfnWndProc = IconProc; ic.hInstance = hi; ic.lpszClassName = L"NTT_SlaveMercIcon";
        RegisterClassExW(&wc); RegisterClassExW(&ic);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, wc.lpszClassName, L"Slave Merchant", WS_POPUP, 0, 0, SM_W, SM_H, nullptr, nullptr, hi, nullptr);
        g_hIcon = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, ic.lpszClassName, L"Slave Merchant Icon", WS_POPUP, 0, 0, SM_ICON_W, SM_ICON_H, nullptr, nullptr, hi, nullptr);
        if (!g_hWnd || !g_hIcon) { Log("SLAVEMERC: window creation failed"); return 0; }
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), SM_W, -SM_H, 1, 32, BI_RGB };
        g_dc = CreateCompatibleDC(nullptr); SelectObject(g_dc, CreateDIBSection(g_dc, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
        BITMAPINFO ibi = {}; ibi.bmiHeader = { sizeof(BITMAPINFOHEADER), SM_ICON_W, -SM_ICON_H, 1, 32, BI_RGB };
        g_idc = CreateCompatibleDC(nullptr); SelectObject(g_idc, CreateDIBSection(g_idc, &ibi, DIB_RGB_COLORS, &g_ibits, nullptr, 0));
        Log(Spr("bg") ? "SLAVEMERC: window ready" : "SLAVEMERC: skin MISSING (JstKO\\slave_ui)");
        SetTimer(g_hWnd, 1, 200, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }
}

void SlaveMerc_Init()
{
    InitializeCriticalSection(&g_lock);
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") != 0) return;
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// recv hook (game thread): E9 DD ...
void SlaveMerc_OnRecv(const BYTE* p, size_t len)
{
    if (len < 4 || p[0] != 0xE9 || p[1] != SUB) return;
    if (p[2] == 0)
    {
        EnterCriticalSection(&g_lock); g_show = p[3] != 0; LeaveCriticalSection(&g_lock);
        return;
    }
    if (p[2] != 1) return;
    size_t at = 3;
    if (len < 3 + 13 + 40 * 10) return;
    EnterCriticalSection(&g_lock);
    g_active = p[at++] != 0;
    memcpy(&g_minutes, p + at, 4); at += 4;
    memcpy(&g_gold, p + at, 4); at += 4;
    memcpy(&g_kc, p + at, 4); at += 4;
    auto cell = [&](Cell& c) { memcpy(&c.item, p + at, 4); memcpy(&c.icon, p + at + 4, 4); memcpy(&c.cnt, p + at + 8, 2); at += 10; };
    for (int i = 0; i < 12 && at + 10 <= len; i++) cell(g_stall[i]);
    for (int i = 0; i < 28 && at + 10 <= len; i++) cell(g_bag[i]);
    LeaveCriticalSection(&g_lock);
    if (g_hWnd) PostMessageW(g_hWnd, WM_SM_REFRESH, 0, 0);
}

// click gate: the game must not get clicks meant for the window / the icon (DirectInput)
bool SlaveMerc_CursorOverPanel()
{
    POINT p;
    if (!GetCursorPos(&p)) return false;
    HWND w = WindowFromPoint(p);
    return w && ((w == g_hWnd && IsWindowVisible(g_hWnd)) || (w == g_hIcon && IsWindowVisible(g_hIcon)));
}
