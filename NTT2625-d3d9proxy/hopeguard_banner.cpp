// HopeGuard ACS start splash. When KnightOnLine.exe starts (Launcher -> Start) the "HopeGuard ACS" banner shows in the
// middle of the screen with the game's loading bar right under it, filling in SPLASH_MS. The game waits for it:
// Direct3DCreate9(Ex) (d3d9proxy.cpp, game main thread, before the game draws anything) calls HopeGuardSplash_Wait(),
// which returns when the bar is full; then the splash closes and the game opens. Meanwhile the ACS scan thread
// (launch_guard.cpp) hashes the tables and does its first check.
// Skin: build_hopeguard_banner.py (hg_ui\banner.pus, bar_empty.pus, bar_full.pus: green message box + ui_loding.dxt).
#include <windows.h>
#include <stdio.h>
#include <string>
#include <vector>
#include "hg_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);

namespace
{
    const DWORD SPLASH_MS = 10000, HOLD_MS = 400, WAIT_LIMIT_MS = SPLASH_MS + 5000;
    const UINT WM_HG_CLOSE = WM_APP + 71;
    HWND  g_hWnd = nullptr;
    HDC   g_memDC = nullptr;
    void* g_bits = nullptr;
    DWORD g_start = 0;
    HANDLE g_done = nullptr;                        // set when the bar is full (or the splash could not start)
    volatile LONG g_active = 0;

    struct Sprite { HDC dc = nullptr; int w = 0, h = 0; };
    Sprite g_frame, g_empty, g_full;

    bool LoadPus(const char* name, Sprite& s)
    {
        char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH); sprintf_s(strrchr(p, '\\') + 1, 64, "HopeGuard\\hg_ui\\%s.pus", name);
        FILE* f = nullptr;
        if (fopen_s(&f, p, "rb") != 0 || !f) { char b[MAX_PATH + 32]; sprintf_s(b, "HOPEGUARD: missing %s", p); Log(b); return false; }
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

    void Blit(const Sprite& s, int x, int y, int w = -1)
    {
        if (!s.dc) return;
        if (w < 0) w = s.w;
        if (w <= 0) return;
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(g_memDC, x, y, w, s.h, s.dc, 0, 0, w, s.h, bf);
    }

    // white text drawn alone, then coloured with a dark 1 px shadow into the splash bitmap (as the other panels)
    void Text(const wchar_t* s, const int r[4], COLORREF color, int pt, bool bold)
    {
        int w = r[2] - r[0], h = r[3] - r[1];
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
        void* bits = nullptr; HDC dc = CreateCompatibleDC(nullptr);
        HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        HGDIOBJ ob = SelectObject(dc, bmp);
        HFONT font = CreateFontW(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, L"Verdana");
        HGDIOBJ of = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
        RECT rc = { 0, 0, w, h };
        DrawTextW(dc, s, -1, &rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        GdiFlush();
        for (int pass = 0; pass < 2; pass++)
        {
            int o = pass ? 0 : 1;
            BYTE cr = pass ? GetRValue(color) : 0, cg = pass ? GetGValue(color) : 0, cb = pass ? GetBValue(color) : 0;
            for (int y = 0; y < h; y++)
            {
                int dy = r[1] + y + o; if (dy < 0 || dy >= HG_H) continue;
                const BYTE* src = (const BYTE*)bits + y * w * 4;
                for (int x = 0; x < w; x++, src += 4)
                {
                    int dx = r[0] + x + o; int a = max(src[0], max(src[1], src[2]));
                    if (!a || dx < 0 || dx >= HG_W) continue;
                    if (!pass) a = a * 3 / 4;
                    BYTE* d = (BYTE*)g_bits + (dy * HG_W + dx) * 4;
                    d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                    d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
                }
            }
        }
        SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }

    void Paint(int pct)
    {
        memset(g_bits, 0, (size_t)HG_W * HG_H * 4);
        Blit(g_frame, 0, 0);
        Blit(g_empty, HG_BAR[0], HG_BAR[1]);
        Blit(g_full, HG_BAR[0], HG_BAR[1], g_full.w * pct / 100);
        GdiFlush();
        Text(L"HopeGuard ACS", HG_TITLE, RGB(255, 170, 60), 13, true);
        wchar_t line[96];
        if (pct < 100) swprintf_s(line, L"Scanning your system...  %d%%", pct);
        else wcscpy_s(line, L"Activated - your game is protected");
        Text(line, HG_TEXT, pct < 100 ? RGB(255, 255, 255) : RGB(170, 255, 170), 10, false);
        POINT src = { 0, 0 }; SIZE sz = { HG_W, HG_H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, nullptr, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);   // position set once
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_TIMER:
        {
            DWORD el = GetTickCount() - g_start;
            Paint(el >= SPLASH_MS ? 100 : (int)(el * 100 / SPLASH_MS));
            if (el >= SPLASH_MS + HOLD_MS && g_done) SetEvent(g_done);              // the game may start
            if (el >= WAIT_LIMIT_MS) DestroyWindow(h);                              // nobody closed us: close anyway
            return 0;
        }
        case WM_HG_CLOSE: DestroyWindow(h); return 0;
        case WM_DESTROY: KillTimer(h, 1); PostQuitMessage(0); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI SplashThread(LPVOID)
    {
        WNDCLASSEXW wc = { sizeof(wc) };
        wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr); wc.lpszClassName = L"NTT_HopeGuardSplash";
        RegisterClassExW(&wc);
        // the middle of the screen
        int x = (GetSystemMetrics(SM_CXSCREEN) - HG_W) / 2, y = (GetSystemMetrics(SM_CYSCREEN) - HG_H) / 2;
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            wc.lpszClassName, L"HopeGuard ACS", WS_POPUP, x, y, HG_W, HG_H, nullptr, nullptr, wc.hInstance, nullptr);
        if (!g_hWnd) { SetEvent(g_done); InterlockedExchange(&g_active, 0); return 0; }
        g_memDC = CreateCompatibleDC(nullptr);
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), HG_W, -HG_H, 1, 32, BI_RGB };
        SelectObject(g_memDC, CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
        LoadPus("banner", g_frame); LoadPus("bar_empty", g_empty); LoadPus("bar_full", g_full);
        g_start = GetTickCount();
        Paint(0);
        SetWindowPos(g_hWnd, HWND_TOPMOST, x, y, HG_W, HG_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        SetTimer(g_hWnd, 1, 40, nullptr);
        Log("HOPEGUARD: splash shown");
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        SetEvent(g_done);
        g_hWnd = nullptr;
        InterlockedExchange(&g_active, 0);
        Log("HOPEGUARD: splash closed");
        return 0;
    }
}

void HopeGuardBanner_Init()
{
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") != 0) return;   // not for Option.exe etc.
    g_done = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    InterlockedExchange(&g_active, 1);
    CloseHandle(CreateThread(nullptr, 0, SplashThread, nullptr, 0, nullptr));
}

// Direct3DCreate9(Ex), game main thread: hold the game until the bar is full, then close the splash. The game's own
// windows are hidden meanwhile (nothing drawn yet; a visible window that does not answer would turn "not responding").
void HopeGuardSplash_Wait()
{
    static volatile LONG once = 0;
    if (!g_active || !g_done || InterlockedExchange(&once, 1)) return;   // only the first device creation waits
    std::vector<HWND> hidden;
    EnumThreadWindows(GetCurrentThreadId(), [](HWND h, LPARAM lp) -> BOOL {
        if (IsWindowVisible(h)) { ((std::vector<HWND>*)lp)->push_back(h); ShowWindow(h, SW_HIDE); }
        return TRUE;
    }, (LPARAM)&hidden);
    WaitForSingleObject(g_done, WAIT_LIMIT_MS);
    if (HWND w = g_hWnd) PostMessageW(w, WM_HG_CLOSE, 0, 0);
    for (HWND h : hidden) ShowWindow(h, SW_SHOW);
    if (!hidden.empty()) SetForegroundWindow(hidden.front());
}
