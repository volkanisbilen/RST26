// Solo Cape top-bar icon: a round icon at the top of the screen next to the game's coin gauge while the character has
// a Solo Cape; hovering it shows a tooltip with the cape and its bonuses (like an item tooltip).
// Server: SoloCape.cpp SendSoloCapeIcon, WIZ_HSACS_HOOK + SOLOCAPEICON (0xF3), sent with every cape (re)attach:
//   [u8 show][i16 capeID][u8 r][u8 g][u8 b][str name][u8 n] n*([u8 colour][str line])
// Skin: build_solocape_icon.py (solocape_ui\icon_n.pus / icon_o.pus: the coin gauge's gold coin + the Solo Cape icon).
// Double-click on the icon: confirm box (the game's message box, reconnect_ui\*.pus) -> C->S [E9 F3 01] removes the
// character-bound cape (SoloCape.cpp HandleSoloCapeIcon).
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include "solocape_layout.h"
#include "reconnect_layout.h"   // the message box skin (RC_W / RC_H / button rects)
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
HWND Proxy_GameWnd();
void Proxy_RequestFlush();

namespace
{
    const BYTE SC_SUBOP = 0xF3;
    const int TIP_W = 230, TIP_MAXH = 220;
    // position: left of the coin gauge (1920-wide reference: coin centre x ~1357, top row)
    const double POS_X = 1274.0 / 1920.0;
    const int POS_Y = -4;
    const UINT WM_SC_UPDATE = WM_APP + 81;

    struct Info { bool show = false; short cape = -1; BYTE r = 0, g = 0, b = 0; std::string name; std::vector<std::pair<BYTE, std::string>> lines; };
    CRITICAL_SECTION g_lock;
    Info g_info;

    HWND g_hWnd = nullptr, g_hTip = nullptr;
    HDC  g_dc = nullptr, g_tipDC = nullptr;
    void* g_bits = nullptr; void* g_tipBits = nullptr;
    bool g_hover = false, g_track = false;
    struct Sprite { HDC dc = nullptr; int w = 0, h = 0; } g_n, g_o, g_boxBg, g_btnN, g_btnO, g_btnD;
    HWND g_hBox = nullptr; HDC g_boxDC = nullptr; void* g_boxBits = nullptr;
    int g_boxHover = 0, g_boxPressed = 0;              // 1 remove, 2 cancel
    std::deque<std::vector<BYTE>> g_sendQ;

    bool LoadPus(const char* name, Sprite& s, const char* dir = "HopeGuard\\solocape_ui")
    {
        char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH); sprintf_s(strrchr(p, '\\') + 1, 96, "%s\\%s.pus", dir, name);
        FILE* f = nullptr;
        if (fopen_s(&f, p, "rb") != 0 || !f) { char b[MAX_PATH + 32]; sprintf_s(b, "SOLOCAPE: missing %s", p); Log(b); return false; }
        char magic[4]; UINT32 w = 0, h = 0; bool ok = false;
        if (fread(magic, 1, 4, f) == 4 && !memcmp(magic, "PUSI", 4) && fread(&w, 4, 1, f) == 1 && fread(&h, 4, 1, f) == 1 && w && h && w < 2048 && h < 2048)
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

    std::wstring Wide(const std::string& s)
    {
        if (s.empty()) return std::wstring();
        int n = MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), nullptr, 0);
        std::wstring w(n, L'\0'); MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), &w[0], n);
        return w;
    }

    COLORREF TipColor(BYTE c)
    {
        switch (c)
        {
        case 1: return RGB(120, 235, 120);
        case 2: return RGB(255, 210, 90);
        case 3: return RGB(110, 210, 255);
        case 4: return RGB(255, 100, 90);
        case 5: return RGB(170, 170, 170);
        default: return RGB(235, 235, 235);
        }
    }

    // tooltip drawing (premultiplied BGRA bitmap of TIP_W x h)
    void Fill(int x0, int y0, int x1, int y1, BYTE r, BYTE g, BYTE b, BYTE a)
    {
        for (int y = max(0, y0); y < min(TIP_MAXH, y1); y++)
            for (int x = max(0, x0); x < min(TIP_W, x1); x++)
            {
                BYTE* d = (BYTE*)g_tipBits + (y * TIP_W + x) * 4;
                d[0] = (BYTE)(b * a / 255); d[1] = (BYTE)(g * a / 255); d[2] = (BYTE)(r * a / 255); d[3] = a;
            }
    }
    void Border(int x0, int y0, int x1, int y1, BYTE r, BYTE g, BYTE b)
    {
        Fill(x0, y0, x1, y0 + 1, r, g, b, 255); Fill(x0, y1 - 1, x1, y1, r, g, b, 255);
        Fill(x0, y0, x0 + 1, y1, r, g, b, 255); Fill(x1 - 1, y0, x1, y1, r, g, b, 255);
    }
    void TextTo(void* target, int tw, int th, const std::string& s, int l, int t, int r, int bt, COLORREF color, int pt, bool bold, UINT fmt = DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    void Text(const std::string& s, int l, int t, int r, int bt, COLORREF color, int pt, bool bold)
    {
        TextTo(g_tipBits, TIP_W, TIP_MAXH, s, l, t, r, bt, color, pt, bold);
    }
    void TextTo(void* target, int tw, int th, const std::string& s, int l, int t, int r, int bt, COLORREF color, int pt, bool bold, UINT fmt)
    {
        int w = r - l, h = bt - t;
        if (s.empty() || w <= 0 || h <= 0) return;
        std::wstring ws = Wide(s);
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
        void* bits = nullptr; HDC dc = CreateCompatibleDC(nullptr);
        HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        HGDIOBJ ob = SelectObject(dc, bmp);
        HFONT font = CreateFontW(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, L"Verdana");
        HGDIOBJ of = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
        RECT rc = { 0, 0, w, h };
        DrawTextW(dc, ws.c_str(), (int)ws.size(), &rc, fmt | DT_NOPREFIX);
        GdiFlush();
        for (int y = 0; y < h; y++)
        {
            int dy = t + y; if (dy < 0 || dy >= th) continue;
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            for (int x = 0; x < w; x++, src += 4)
            {
                int dx = l + x; int a = max(src[0], max(src[1], src[2]));
                if (!a || dx < 0 || dx >= tw) continue;
                BYTE* d = (BYTE*)target + (dy * tw + dx) * 4;
                d[0] = (BYTE)((GetBValue(color) * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((GetGValue(color) * a + d[1] * (255 - a)) / 255);
                d[2] = (BYTE)((GetRValue(color) * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
            }
        }
        SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }

    void ShowTip()
    {
        EnterCriticalSection(&g_lock); Info in = g_info; LeaveCriticalSection(&g_lock);
        if (!in.show) return;
        const int pad = 10, titleH = 22, lineH = 17;
        int h = min(TIP_MAXH, pad + titleH + 8 + (int)in.lines.size() * lineH + pad);
        memset(g_tipBits, 0, (size_t)TIP_W * TIP_MAXH * 4);
        Fill(0, 0, TIP_W, h, 12, 12, 16, 235);
        Border(0, 0, TIP_W, h, 150, 120, 60);
        Border(2, 2, TIP_W - 2, h - 2, 60, 48, 26);
        Text(in.name, pad, pad, TIP_W - pad, pad + titleH, RGB(255, 210, 90), 10, true);
        Fill(pad, pad + titleH + 3, TIP_W - pad, pad + titleH + 4, 150, 120, 60, 255);
        int y = pad + titleH + 8;
        for (auto& l : in.lines)
        {
            if (y + lineH > h) break;
            Text(l.second, pad, y, TIP_W - pad, y + lineH, TipColor(l.first), 9, false);
            y += lineH;
        }
        RECT wr; GetWindowRect(g_hWnd, &wr);
        int x = wr.left + (SCI_SIZE - TIP_W) / 2, ty = wr.bottom + 2;
        HMONITOR mon = MonitorFromWindow(g_hWnd, MONITOR_DEFAULTTONEAREST);
        MONITORINFO mi = { sizeof(mi) }; GetMonitorInfo(mon, &mi);
        if (x + TIP_W > mi.rcWork.right) x = mi.rcWork.right - TIP_W;
        if (x < mi.rcWork.left) x = mi.rcWork.left;
        POINT dst = { x, ty }, src = { 0, 0 }; SIZE sz = { TIP_W, h };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hTip, nullptr, &dst, &sz, g_tipDC, &src, 0, &bf, ULW_ALPHA);
        SetWindowPos(g_hTip, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }

    // ---------------------------------------------------------------- remove confirm box
    void PaintBox()
    {
        if (!g_hBox || !g_boxDC) return;
        memset(g_boxBits, 0, (size_t)RC_W * RC_H * 4);
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        if (g_boxBg.dc) AlphaBlend(g_boxDC, 0, 0, g_boxBg.w, g_boxBg.h, g_boxBg.dc, 0, 0, g_boxBg.w, g_boxBg.h, bf);
        const int* br[2] = { RC_BTN_RECONNECT, RC_BTN_EXIT };
        for (int i = 0; i < 2; i++)
        {
            int id = i + 1;
            const Sprite& b = g_boxHover == id ? (g_boxPressed == id ? g_btnD : g_btnO) : g_btnN;
            if (b.dc) AlphaBlend(g_boxDC, br[i][0], br[i][1], b.w, b.h, b.dc, 0, 0, b.w, b.h, bf);
        }
        GdiFlush();
        TextTo(g_boxBits, RC_W, RC_H, "Remove Solo Cape", RC_TITLE[0], RC_TITLE[1], RC_TITLE[2], RC_TITLE[3], RGB(255, 170, 60), 10, true);
        TextTo(g_boxBits, RC_W, RC_H, "Your Solo Cape will be removed from your character.\nThis cannot be undone. Continue?",
            RC_TEXT[0] + 16, RC_TEXT[1] + 18, RC_TEXT[2] - 16, RC_TEXT[3] - 8, RGB(255, 255, 255), 9, false, DT_CENTER | DT_WORDBREAK);
        TextTo(g_boxBits, RC_W, RC_H, "Remove", br[0][0], br[0][1], br[0][2], br[0][3], RGB(255, 240, 210), 9, true);
        TextTo(g_boxBits, RC_W, RC_H, "Cancel", br[1][0], br[1][1], br[1][2], br[1][3], RGB(255, 240, 210), 9, true);
        RECT wr; GetWindowRect(g_hBox, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { RC_W, RC_H };
        UpdateLayeredWindow(g_hBox, nullptr, &dst, &sz, g_boxDC, &src, 0, &bf, ULW_ALPHA);
    }

    void OpenBox()
    {
        if (!g_hBox) return;
        RECT gr = { 0, 0, 1024, 768 };
        if (HWND game = Proxy_GameWnd()) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
        SetWindowPos(g_hBox, HWND_TOPMOST, gr.left + ((gr.right - gr.left) - RC_W) / 2, gr.top + ((gr.bottom - gr.top) - RC_H) / 2,
            RC_W, RC_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        g_boxHover = g_boxPressed = 0;
        PaintBox();
    }

    void RequestRemove()
    {
        std::vector<BYTE> p = { 0xE9, SC_SUBOP, 1 };
        EnterCriticalSection(&g_lock); g_sendQ.push_back(p); LeaveCriticalSection(&g_lock);
        Proxy_RequestFlush();
        Log("SOLOCAPE: remove requested");
    }

    int BoxHit(int x, int y)
    {
        auto in = [&](const int* r) { return x >= r[0] && x < r[2] && y >= r[1] && y < r[3]; };
        return in(RC_BTN_RECONNECT) ? 1 : in(RC_BTN_EXIT) ? 2 : 0;
    }

    LRESULT CALLBACK BoxProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_MOUSEMOVE:
        {
            TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; TrackMouseEvent(&t);
            int hit = BoxHit(x, y); if (hit != g_boxHover) { g_boxHover = hit; PaintBox(); }
            return 0;
        }
        case WM_MOUSELEAVE: g_boxHover = 0; PaintBox(); return 0;
        case WM_LBUTTONDOWN: g_boxPressed = BoxHit(x, y); if (g_boxPressed) SetCapture(h); PaintBox(); return 0;
        case WM_LBUTTONUP:
        {
            ReleaseCapture();
            int hit = BoxHit(x, y), p = g_boxPressed; g_boxPressed = 0;
            if (p && p == hit)
            {
                if (hit == 1) RequestRemove();
                ShowWindow(h, SW_HIDE);
                return 0;
            }
            PaintBox();
            return 0;
        }
        case WM_SETCURSOR: SetCursor(LoadCursor(nullptr, IDC_ARROW)); return TRUE;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    void PaintIcon()
    {
        memset(g_bits, 0, (size_t)SCI_SIZE * SCI_SIZE * 4);
        const Sprite& s = g_hover && g_o.dc ? g_o : g_n;
        if (s.dc)
        {
            BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
            AlphaBlend(g_dc, 0, 0, s.w, s.h, s.dc, 0, 0, s.w, s.h, bf);
        }
        POINT src = { 0, 0 }; SIZE sz = { SCI_SIZE, SCI_SIZE };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, nullptr, &sz, g_dc, &src, 0, &bf, ULW_ALPHA);
    }

    // shown while we have a cape and the game is in front; follows the game window
    void Sync()
    {
        EnterCriticalSection(&g_lock); bool show = g_info.show; LeaveCriticalSection(&g_lock);
        HWND game = Proxy_GameWnd(); DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        bool want = show && game && !IsIconic(game) && pid == GetCurrentProcessId();
        if (!want)
        {
            if (IsWindowVisible(g_hWnd)) ShowWindow(g_hWnd, SW_HIDE);
            if (IsWindowVisible(g_hTip)) ShowWindow(g_hTip, SW_HIDE);
            return;
        }
        RECT gr; GetClientRect(game, &gr); MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
        int x = gr.left + (int)((gr.right - gr.left) * POS_X), y = gr.top + POS_Y;
        RECT wr; GetWindowRect(g_hWnd, &wr);
        if (!IsWindowVisible(g_hWnd) || wr.left != x || wr.top != y)
        {
            SetWindowPos(g_hWnd, HWND_TOPMOST, x, y, SCI_SIZE, SCI_SIZE, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            PaintIcon();
        }
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_NCHITTEST:
        {
            // only the coin itself reacts (the corners of the 53x53 cell are see-through)
            POINT p = { GET_X_LPARAM(lp), GET_Y_LPARAM(lp) }; ScreenToClient(h, &p);
            int dx = p.x - SCI_SIZE / 2, dy = p.y - SCI_SIZE / 2;
            return dx * dx + dy * dy <= 18 * 18 ? HTCLIENT : HTTRANSPARENT;
        }
        case WM_MOUSEMOVE:
            if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
            if (!g_hover) { g_hover = true; PaintIcon(); ShowTip(); }
            return 0;
        case WM_MOUSELEAVE:
            g_track = false; g_hover = false; PaintIcon(); ShowWindow(g_hTip, SW_HIDE);
            return 0;
        case WM_LBUTTONDBLCLK: ShowWindow(g_hTip, SW_HIDE); OpenBox(); return 0;
        case WM_SETCURSOR: SetCursor(LoadCursor(nullptr, IDC_ARROW)); return TRUE;
        case WM_TIMER:
            Sync();
            if (g_hBox && IsWindowVisible(g_hBox) && !IsWindowVisible(g_hWnd)) ShowWindow(g_hBox, SW_HIDE);   // cape gone / game in back
            return 0;
        case WM_SC_UPDATE: Sync(); PaintIcon(); if (g_hover) ShowTip(); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    LRESULT CALLBACK TipProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        if (msg == WM_MOUSEACTIVATE) return MA_NOACTIVATE;
        if (msg == WM_NCHITTEST) return HTTRANSPARENT;
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        HINSTANCE hi = GetModuleHandleW(nullptr);
        WNDCLASSEXW wc = { sizeof(wc) }; wc.lpfnWndProc = WndProc; wc.hInstance = hi; wc.lpszClassName = L"NTT_SoloCapeIcon";
        wc.style = CS_DBLCLKS;
        WNDCLASSEXW bc = { sizeof(bc) }; bc.lpfnWndProc = BoxProc; bc.hInstance = hi; bc.lpszClassName = L"NTT_SoloCapeBox";
        RegisterClassExW(&bc);
        RegisterClassExW(&wc);
        WNDCLASSEXW tc = { sizeof(tc) }; tc.lpfnWndProc = TipProc; tc.hInstance = hi; tc.lpszClassName = L"NTT_SoloCapeTip";
        RegisterClassExW(&tc);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, wc.lpszClassName, L"Solo Cape",
            WS_POPUP, 0, 0, SCI_SIZE, SCI_SIZE, nullptr, nullptr, hi, nullptr);
        g_hTip = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT, tc.lpszClassName,
            L"Solo Cape Tip", WS_POPUP, 0, 0, TIP_W, TIP_MAXH, nullptr, nullptr, hi, nullptr);
        if (!g_hWnd || !g_hTip) { Log("SOLOCAPE: window creation failed"); return 0; }
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), SCI_SIZE, -SCI_SIZE, 1, 32, BI_RGB };
        g_dc = CreateCompatibleDC(nullptr); SelectObject(g_dc, CreateDIBSection(g_dc, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
        BITMAPINFO tbi = {}; tbi.bmiHeader = { sizeof(BITMAPINFOHEADER), TIP_W, -TIP_MAXH, 1, 32, BI_RGB };
        g_tipDC = CreateCompatibleDC(nullptr); SelectObject(g_tipDC, CreateDIBSection(g_tipDC, &tbi, DIB_RGB_COLORS, &g_tipBits, nullptr, 0));
        LoadPus("icon_n", g_n); LoadPus("icon_o", g_o);
        LoadPus("bg", g_boxBg, "HopeGuard\\reconnect_ui"); LoadPus("btn_n", g_btnN, "HopeGuard\\reconnect_ui");
        LoadPus("btn_o", g_btnO, "HopeGuard\\reconnect_ui"); LoadPus("btn_d", g_btnD, "HopeGuard\\reconnect_ui");
        g_hBox = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, bc.lpszClassName, L"Remove Solo Cape",
            WS_POPUP, 0, 0, RC_W, RC_H, nullptr, nullptr, hi, nullptr);
        BITMAPINFO bbi = {}; bbi.bmiHeader = { sizeof(BITMAPINFOHEADER), RC_W, -RC_H, 1, 32, BI_RGB };
        g_boxDC = CreateCompatibleDC(nullptr); SelectObject(g_boxDC, CreateDIBSection(g_boxDC, &bbi, DIB_RGB_COLORS, &g_boxBits, nullptr, 0));
        SetTimer(g_hWnd, 1, 250, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }
}

void SoloCapeIcon_Init()
{
    InitializeCriticalSection(&g_lock);
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") != 0) return;
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// recv hook (game thread)
void SoloCapeIcon_OnRecv(const BYTE* p, size_t len)
{
    if (len < 3 || p[0] != 0xE9 || p[1] != SC_SUBOP) return;
    size_t at = 2;
    auto u8 = [&](BYTE& v) { if (at + 1 > len) return false; v = p[at++]; return true; };
    auto str = [&](std::string& v) {
        if (at + 2 > len) return false;
        WORD n = *(const WORD*)(p + at); at += 2;
        if (n > 200 || at + n > len) return false;
        v.assign((const char*)p + at, n); at += n; return true;
    };
    Info in; BYTE show = 0, n = 0;
    if (!u8(show) || at + 2 > len) return;
    in.cape = *(const short*)(p + at); at += 2;
    if (!u8(in.r) || !u8(in.g) || !u8(in.b) || !str(in.name) || !u8(n)) return;
    for (BYTE i = 0; i < n; i++)
    {
        BYTE c; std::string s;
        if (!u8(c) || !str(s)) break;
        in.lines.push_back({ c, s });
    }
    in.show = show != 0;
    EnterCriticalSection(&g_lock); g_info = in; LeaveCriticalSection(&g_lock);
    if (g_hWnd) PostMessageW(g_hWnd, WM_SC_UPDATE, 0, 0);
    char b[128]; sprintf_s(b, "SOLOCAPE: icon %s (%d) %s", in.show ? "on" : "off", (int)in.cape, in.name.c_str()); Log(b);
}

// game thread: send our queued request
void SoloCapeIcon_FlushSendQueue()
{
    void* sock = *(void**)0x01115914;   // CAPISocket*
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_lock); pending.swap(g_sendQ); LeaveCriticalSection(&g_lock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)0x00704070)(sock, p.data(), (int)p.size());
    }
}

// pus_store.cpp click gate: the game must not get clicks meant for the icon / the confirm box (DirectInput)
bool SoloCapeIcon_CursorOverPanel()
{
    POINT p;
    if (!GetCursorPos(&p)) return false;
    HWND w = WindowFromPoint(p);
    return w && ((w == g_hWnd && IsWindowVisible(g_hWnd)) || (w == g_hBox && IsWindowVisible(g_hBox)));
}
