// "Clan Kasasi" button on the clan window. The client has no clan warehouse UI of its own: the button
// asks the server (hook 0xE9 / 0xB0) to open the clan warehouse in the native warehouse window.
// The button follows the clan window: [[0x111463C] + 1392] is the window, +0xEA its visible flag and
// +0xC8 its rect (4 floats, game-client coordinates). The position inside the window can be tuned
// without a rebuild in HopeGuard\clan_kasa.ini ([button] x=, y=, w=, h= relative to the window's top left).
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <vector>

void Log(const char* msg);
HWND Proxy_GameWnd();
void Drop_SendRaw(const std::vector<BYTE>& p);

namespace
{
    const BYTE WIZ_EXT_HOOK = 0xE9, CLANBANK_SUBOP = 0xB0;
    HWND g_hBtn = nullptr;
    bool g_hover = false, g_track = false;
    volatile LONG g_vis = 0, g_wl = 0, g_wt = 0, g_wr = 0, g_wb = 0;   // written on the game thread
    int g_x = 192, g_y = 203, g_w = 64, g_h = 22;

    void LoadIni()
    {
        char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH); *(strrchr(p, '\\') + 1) = 0;
        strcat_s(p, "HopeGuard\\clan_kasa.ini");
        g_x = GetPrivateProfileIntA("button", "x", g_x, p); g_y = GetPrivateProfileIntA("button", "y", g_y, p);
        g_w = GetPrivateProfileIntA("button", "w", g_w, p); g_h = GetPrivateProfileIntA("button", "h", g_h, p);
    }

    void Paint(HWND h)
    {
        PAINTSTRUCT ps; HDC dc = BeginPaint(h, &ps);
        RECT rc; GetClientRect(h, &rc);
        HBRUSH bg = CreateSolidBrush(g_hover ? RGB(120, 84, 24) : RGB(70, 48, 14));
        FillRect(dc, &rc, bg); DeleteObject(bg);
        HPEN pen = CreatePen(PS_SOLID, 1, RGB(230, 190, 90)); HGDIOBJ op = SelectObject(dc, pen);
        HGDIOBJ ob = SelectObject(dc, GetStockObject(NULL_BRUSH));
        Rectangle(dc, 0, 0, rc.right, rc.bottom);
        SelectObject(dc, ob); SelectObject(dc, op); DeleteObject(pen);
        HFONT font = CreateFontW(-12, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, L"Verdana");
        HGDIOBJ of = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 230, 150));
        DrawTextW(dc, L"Clan Kasasi", -1, &rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(dc, of); DeleteObject(font);
        EndPaint(h, &ps);
    }

    void Sync()
    {
        HWND game = Proxy_GameWnd(); DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        bool front = game && !IsIconic(game) && pid == GetCurrentProcessId();
        if (!front || !g_vis) { if (IsWindowVisible(g_hBtn)) ShowWindow(g_hBtn, SW_HIDE); return; }
        POINT o = { g_wl, g_wt }; ClientToScreen(game, &o);
        int x = o.x + g_x, y = o.y + g_y;
        RECT wr; GetWindowRect(g_hBtn, &wr);
        if (!IsWindowVisible(g_hBtn) || wr.left != x || wr.top != y)
            SetWindowPos(g_hBtn, HWND_TOPMOST, x, y, g_w, g_h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }

    LRESULT CALLBACK BtnProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_PAINT: Paint(h); return 0;
        case WM_ERASEBKGND: return 1;
        case WM_MOUSEMOVE:
            if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
            if (!g_hover) { g_hover = true; InvalidateRect(h, nullptr, FALSE); }
            return 0;
        case WM_MOUSELEAVE: g_track = false; g_hover = false; InvalidateRect(h, nullptr, FALSE); return 0;
        case WM_LBUTTONUP:
            Drop_SendRaw({ WIZ_EXT_HOOK, CLANBANK_SUBOP, 1 });
            Log("CLANKASA: open clan warehouse requested");
            return 0;
        case WM_SETCURSOR: SetCursor(LoadCursor(nullptr, IDC_ARROW)); return TRUE;
        case WM_TIMER: Sync(); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        HINSTANCE hi = GetModuleHandleW(nullptr);
        WNDCLASSEXW c = { sizeof(c) }; c.lpfnWndProc = BtnProc; c.hInstance = hi; c.lpszClassName = L"NTT_ClanKasa";
        RegisterClassExW(&c);
        g_hBtn = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, c.lpszClassName, L"ClanKasa",
            WS_POPUP, 0, 0, g_w, g_h, nullptr, nullptr, hi, nullptr);
        if (!g_hBtn) return 0;
        SetTimer(g_hBtn, 1, 100, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }

    bool ReadWindow()
    {
        DWORD mgr = *(DWORD*)0x0111463C;
        if (!mgr) return false;
        BYTE* w = (BYTE*)*(DWORD*)(mgr + 1392);
        if (!w || !w[0xEA]) return false;
        const float* f = (const float*)(w + 0xC8);
        InterlockedExchange(&g_wl, (LONG)f[0]); InterlockedExchange(&g_wt, (LONG)f[1]);
        InterlockedExchange(&g_wr, (LONG)f[2]); InterlockedExchange(&g_wb, (LONG)f[3]);
        return f[2] > f[0] && f[3] > f[1];
    }
}

void ClanKasa_Init()
{
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") != 0) return;
    LoadIni();
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// game thread (ProxyWndProc)
void ClanKasa_Tick()
{
    static DWORD last = 0;
    DWORD now = GetTickCount();
    if (now - last < 100) return;
    last = now;
    bool vis = false;
    __try { vis = ReadWindow(); } __except (EXCEPTION_EXECUTE_HANDLER) { vis = false; }
    static int lastVis = -1;
    if ((int)vis != lastVis)
    {
        lastVis = vis;
        char b[96]; sprintf_s(b, "CLANKASA: clan window %s rect %d,%d-%d,%d", vis ? "open" : "closed", (int)g_wl, (int)g_wt, (int)g_wr, (int)g_wb); Log(b);
    }
    InterlockedExchange(&g_vis, vis ? 1 : 0);
}

bool ClanKasa_CursorOverPanel()
{
    POINT p;
    if (!GetCursorPos(&p)) return false;
    HWND w = WindowFromPoint(p);
    return w && w == g_hBtn && IsWindowVisible(g_hBtn);
}
