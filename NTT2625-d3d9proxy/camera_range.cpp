// "Camera Range" for the native options window (F10): how far the mouse wheel may pull the camera back.
//
// The game's zoom is one function, 0x7A9440 (float delta in xmm1, camera/engine object [0x111584C]):
//   view modes 0 / 2 : zoom factor  [eng+0x1B4], kept in 0.4 .. 1.6
//   view mode 3      : distance     [eng+0x1AC], kept in 2 .. 10 (20 / 30 in a few zones)
//   [eng+0x1EC] != 0 (observer)     : up to 200 in both
// That function is replaced by ZoomImpl below: the same rules with the upper limits multiplied by the Camera Range
// (100 .. 300 %). The value lives in Option.ini [HopeGuard] CameraRange.
//
// (100 .. 300 %, default 200).
// The slider is a small GDI layered window (same technique as login_remember.cpp) drawn with the native slider's
// own pixels (camrange_skin.h) one row above "Mouse Turn Sensitivity" on the Effect option page (see OptionsState).
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string.h>
#include <vector>
#include "camrange_skin.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
HWND Proxy_GameWnd();

namespace
{
    const DWORD KO_ENGINE = 0x0111584C, KO_PLAYER = 0x01115834, KO_GAMEMAIN = 0x011158FC, KO_FRAMETIME = 0x010FD1A0;
    const DWORD FN_ZOOM = 0x007A9440, VT_OPTIONS = 0x0103AAB8;
    const int   RANGE_MIN = 100, RANGE_MAX = 300;
    const int   RANGE_DEFAULT = 200;

    volatile LONG g_range = RANGE_DEFAULT;      // percent

    // ---------------------------------------------------------------- zoom (game thread)
    void __stdcall ZoomImpl(float delta)
    {
        BYTE* eng = *(BYTE**)KO_ENGINE;
        if (!eng || *(DWORD*)(eng + 0x158) == 0) return;
        const float dt = *(const float*)KO_FRAMETIME, k = g_range / 100.0f;
        const bool observer = *(eng + 0x1EC) != 0;
        const DWORD mode = *(DWORD*)(eng + 0x1E8);
        if (mode == 0 || mode == 2)
        {
            float f = *(float*)(eng + 0x1B4) - delta * dt;
            const float top = observer ? 200.0f : 1.6f * k;
            if (f < 0.4f) f = 0.4f; else if (f > top) f = top;
            *(float*)(eng + 0x1B4) = f;
        }
        else if (mode == 3)
        {
            float top = 10.0f;
            const BYTE* me = *(const BYTE**)KO_PLAYER;
            if (observer) top = 200.0f;
            else
            {
                if (me && *(me + 1761))
                {
                    const int zone = *(const int*)(me + 1764);
                    if (zone == 30060 || zone == 30061 || zone == 30070) top = 30.0f;
                    else if (zone == 30010 || zone == 30040 || zone == 30050 || zone == 30210) top = 20.0f;
                }
                top *= k;
            }
            float f = *(float*)(eng + 0x1AC) - delta * 4.0f * dt;
            if (f < 2.0f) f = 2.0f; else if (f > top) f = top;
            *(float*)(eng + 0x1AC) = f;
        }
    }

    // The original keeps every register but eax / ecx / xmm0-2 (the callers were compiled knowing that), so the
    // replacement saves all of them around the C++ code.
    __declspec(naked) void ZoomStub()
    {
        __asm {
            pushad
            sub    esp, 0x80
            movups xmmword ptr [esp], xmm0
            movups xmmword ptr [esp + 0x10], xmm1
            movups xmmword ptr [esp + 0x20], xmm2
            movups xmmword ptr [esp + 0x30], xmm3
            movups xmmword ptr [esp + 0x40], xmm4
            movups xmmword ptr [esp + 0x50], xmm5
            movups xmmword ptr [esp + 0x60], xmm6
            movups xmmword ptr [esp + 0x70], xmm7
            sub    esp, 4
            movss  dword ptr [esp], xmm1
            call   ZoomImpl
            movups xmm0, xmmword ptr [esp]
            movups xmm1, xmmword ptr [esp + 0x10]
            movups xmm2, xmmword ptr [esp + 0x20]
            movups xmm3, xmmword ptr [esp + 0x30]
            movups xmm4, xmmword ptr [esp + 0x40]
            movups xmm5, xmmword ptr [esp + 0x50]
            movups xmm6, xmmword ptr [esp + 0x60]
            movups xmm7, xmmword ptr [esp + 0x70]
            add    esp, 0x80
            popad
            mov    eax, dword ptr ds:[0x0111584C]
            ret
        }
    }

    // a smaller range must also pull an already far camera back in
    void ClampNow()
    {
        __try
        {
            BYTE* eng = *(BYTE**)KO_ENGINE;
            if (!eng || *(eng + 0x1EC)) return;
            const float top = 1.6f * g_range / 100.0f;
            if (*(float*)(eng + 0x1B4) > top) *(float*)(eng + 0x1B4) = top;
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
    }

    // ---------------------------------------------------------------- Option.ini
    void IniPath(char* p)
    {
        GetModuleFileNameA(nullptr, p, MAX_PATH);
        char* s = strrchr(p, '\\');
        if (s) strcpy_s(s + 1, MAX_PATH - (s + 1 - p), "Option.ini");
    }
    void LoadRange()
    {
        char p[MAX_PATH]; IniPath(p);
        int v = (int)GetPrivateProfileIntA("HopeGuard", "CameraRange", RANGE_DEFAULT, p);
        InterlockedExchange(&g_range, v < RANGE_MIN ? RANGE_MIN : v > RANGE_MAX ? RANGE_MAX : v);
    }
    void SaveRange()
    {
        char p[MAX_PATH], v[16]; IniPath(p);
        sprintf_s(v, "%ld", g_range);
        WritePrivateProfileStringA("HopeGuard", "CameraRange", v, p);
    }

    // ---------------------------------------------------------------- options window (read only, any thread)
    // The F10 window is CUISeedHelper (vtable 0x103AAB8), kept at [CGameProcMain + 0x35C] (built in 0x85CEA0). Its
    // settings page is the UI object at [window + 0x284], shown only on the settings tabs (0xD92540 / 0xD927B0).
    // "Mouse Turn Sensitivity" is the scroll bar [window + 0x454] (re_seed_helper.uif scroll_mouse_roate: arrows 23 px
    // at +1 and +343 of its left edge, label box 26 px above it); our slider sits one row (49 px) above that one and
    // is shown only while that scroll bar and all its parents are visible (= the Effect option page is open).
    // Memory is checked with VirtualQuery, never by faulting: crash_trap.cpp logs first-chance access violations.
    const DWORD OFF_MAIN_OPTIONS = 0x35C, OFF_OPT_PAGE = 0x284, OFF_OPT_MOUSE_SCROLL = 0x454;
    const DWORD OFF_UI_CHILDREN = 0xB0, OFF_UI_RECT = 0xC8, OFF_UI_VISIBLE = 0xEA;

    bool Readable(const void* p, SIZE_T n)
    {
        MEMORY_BASIC_INFORMATION mbi;
        if ((DWORD_PTR)p < 0x10000 || !VirtualQuery(p, &mbi, sizeof(mbi))) return false;
        if (mbi.State != MEM_COMMIT || (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD))) return false;
        return (const BYTE*)p + n <= (const BYTE*)mbi.BaseAddress + mbi.RegionSize;
    }

    // parents of `target` below `node` (CN3UI children: std::list, sentinel at [node+0xB0], object at [list node+8])
    bool FindParents(BYTE* node, BYTE* target, int depth, std::vector<BYTE*>& chain, int& budget)
    {
        if (depth > 8 || --budget < 0 || !Readable(node, 0xF0)) return false;
        BYTE* head = *(BYTE**)(node + OFF_UI_CHILDREN);
        if (!Readable(head, 12)) return false;
        chain.push_back(node);
        for (BYTE* n = *(BYTE**)head; n && n != head; n = *(BYTE**)n)
        {
            if (!Readable(n, 12)) break;
            BYTE* c = *(BYTE**)(n + 8);
            if (c == target) return true;
            if (c && FindParents(c, target, depth + 1, chain, budget)) return true;
        }
        chain.pop_back();
        return false;
    }

    std::vector<BYTE*> g_parents;               // of the mouse scroll bar, outermost first
    BYTE* g_parentsOf = nullptr; BYTE* g_parentsOpt = nullptr; DWORD g_lastSearch = 0;

    enum { MODE_HIDDEN, MODE_INLINE, MODE_DOCKED };
    // MODE_INLINE: rc = the mouse scroll bar; MODE_DOCKED (scroll bar not found): rc = the options window
    int OptionsState(RECT* rc)
    {
        static bool logged = false;
        int mode = MODE_HIDDEN;
        __try
        {
#ifdef CAMRANGE_TEST_MAIN
            BYTE* main = CAMRANGE_TEST_MAIN;                // offline test (scratchpad stubtest\ui.cpp)
#else
            BYTE* main = *(BYTE**)KO_GAMEMAIN;
#endif
            if (!Readable(main, OFF_MAIN_OPTIONS + 4)) return MODE_HIDDEN;
            BYTE* opt = *(BYTE**)(main + OFF_MAIN_OPTIONS);
            if (!Readable(opt, OFF_OPT_MOUSE_SCROLL + 4) || *(DWORD*)opt != VT_OPTIONS) return MODE_HIDDEN;
            if (*(opt + OFF_UI_VISIBLE) == 0) return MODE_HIDDEN;
            BYTE* page = *(BYTE**)(opt + OFF_OPT_PAGE);
            if (page && Readable(page, 0xF0) && *(page + OFF_UI_VISIBLE) == 0) return MODE_HIDDEN;   // not a settings tab

            BYTE* scroll = *(BYTE**)(opt + OFF_OPT_MOUSE_SCROLL);
            if (scroll != g_parentsOf || opt != g_parentsOpt)
            {
                DWORD now = GetTickCount();
                if (g_lastSearch && now - g_lastSearch < 2000) return MODE_HIDDEN;
                g_lastSearch = now; g_parents.clear(); g_parentsOf = nullptr; g_parentsOpt = opt;
                int budget = 6000; bool found = false;
                if (Readable(scroll, 0xF0))
                {
                    found = FindParents(opt, scroll, 0, g_parents, budget);
                    if (!found && page) { g_parents.clear(); budget = 6000; found = FindParents(page, scroll, 0, g_parents, budget); }
                }
                if (found) g_parentsOf = scroll; else g_parents.clear();
                if (!logged)
                {
                    logged = true;
                    char b[160]; const float* r = found ? (const float*)(scroll + OFF_UI_RECT) : (const float*)(opt + OFF_UI_RECT);
                    sprintf_s(b, "CAMRANGE: options window found, mouse scroll bar %s (%d parents) rect %d,%d,%d,%d", found ? "found" : "NOT found -> docked under the window",
                        (int)g_parents.size(), (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
                    Log(b);
                }
            }
            if (g_parentsOf)
            {
                for (BYTE* p : g_parents) if (*(p + OFF_UI_VISIBLE) == 0) return MODE_HIDDEN;
                if (*(g_parentsOf + OFF_UI_VISIBLE) == 0) return MODE_HIDDEN;
                const float* r = (const float*)(g_parentsOf + OFF_UI_RECT);
                SetRect(rc, (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
                const int w = rc->right - rc->left, h = rc->bottom - rc->top;
                if (w >= 340 && w <= 420 && h >= 20 && h <= 44) return MODE_INLINE;
            }
            const float* r = (const float*)(opt + OFF_UI_RECT);
            SetRect(rc, (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
            mode = MODE_DOCKED;
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { mode = MODE_HIDDEN; g_parentsOf = nullptr; g_parents.clear(); }
        return mode;
    }

    // ---------------------------------------------------------------- slider window (the native slider's look)
    // row 1 (y 0..23): label box + value box; row 2 (y 26..48): left arrow, track with the knob, right arrow
    const int W = 365, H = 49, ROW2 = 26, ARROW = 23, KNOB_W = 8, LABEL_PAD = 7, BOX_GAP = 6;
    HWND g_hWnd = nullptr; HDC g_dc = nullptr; DWORD* g_bits = nullptr;
    bool g_drag = false; int g_labelW = 110;

    void Sprite(const DWORD* px, int w, int h, int x, int y)
    {
        for (int j = 0; j < h; j++)
        {
            if (y + j < 0 || y + j >= H) continue;
            for (int i = 0; i < w; i++) if (x + i >= 0 && x + i < W) g_bits[(y + j) * W + x + i] = px[j * w + i];
        }
    }
    // the value box stretched to any width: 6 px caps, the middle column repeated
    void Box(int x, int w)
    {
        for (int j = 0; j < SK_BOX_H; j++)
            for (int i = 0; i < w; i++)
            {
                int sx = i < 6 ? i : i >= w - 6 ? SK_BOX_W - (w - i) : 12;
                if (x + i >= 0 && x + i < W) g_bits[j * W + x + i] = SK_BOX[j * SK_BOX_W + sx];
            }
    }
    int KnobX() { return ARROW + (W - 2 * ARROW - KNOB_W) * (g_range - RANGE_MIN) / (RANGE_MAX - RANGE_MIN); }

    void Paint()
    {
        if (!g_hWnd || !g_dc) return;
        memset(g_bits, 0, (size_t)W * H * 4);
        HFONT font = CreateFontA(-11, 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET, 0, 0, NONANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
        HGDIOBJ of = SelectObject(g_dc, font);
        const char* label = "Camera Range";
        SIZE ts = {}; GetTextExtentPoint32A(g_dc, label, (int)strlen(label), &ts);
        g_labelW = ts.cx + 2 * LABEL_PAD;
        const int valueX = g_labelW + BOX_GAP;
        Box(0, g_labelW); Box(valueX, SK_BOX_W);
        Sprite(SK_ARROW_L, SK_ARROW_L_W, SK_ARROW_L_H, 0, ROW2);
        Sprite(SK_ARROW_R, SK_ARROW_R_W, SK_ARROW_R_H, W - ARROW, ROW2);
        for (int x = ARROW; x < W - ARROW; x++) Sprite(SK_TRACK, 1, SK_TRACK_H, x, ROW2 + 7);
        Sprite(SK_KNOB, SK_KNOB_W, SK_KNOB_H, KnobX(), ROW2 + 1);
        GdiFlush();
        SetBkMode(g_dc, TRANSPARENT);
        RECT lr = { LABEL_PAD, 3, g_labelW, SK_BOX_H - 2 };
        SetTextColor(g_dc, RGB(0, 0, 0));                               // the game's text has a black outline
        static const int ox[4] = { -1, 1, 0, 0 }, oy[4] = { 0, 0, -1, 1 };
        for (int k = 0; k < 4; k++) { RECT o = lr; OffsetRect(&o, ox[k], oy[k]); DrawTextA(g_dc, label, -1, &o, DT_LEFT | DT_VCENTER | DT_SINGLELINE); }
        SetTextColor(g_dc, RGB(214, 238, 164));
        DrawTextA(g_dc, label, -1, &lr, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        char v[16]; sprintf_s(v, "%ld", g_range);
        RECT vr = { valueX, 3, valueX + SK_BOX_W, SK_BOX_H - 2 };
        SetTextColor(g_dc, RGB(192, 192, 192));
        DrawTextA(g_dc, v, -1, &vr, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        SelectObject(g_dc, of); DeleteObject(font);
        GdiFlush();
        for (int j = 0; j < SK_BOX_H; j++)                              // GDI text clears alpha: both boxes are opaque
            for (int i = 0; i < valueX + SK_BOX_W; i++)
                if (i < g_labelW || i >= valueX) g_bits[j * W + i] |= 0xFF000000;
        RECT wr; GetWindowRect(g_hWnd, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { W, H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_dc, &src, 0, &bf, ULW_ALPHA);
    }

    void SetRange(int v)
    {
        v = (v + 2) / 5 * 5;                                            // steps of 5 %
        v = v < RANGE_MIN ? RANGE_MIN : v > RANGE_MAX ? RANGE_MAX : v;
        if (v == g_range) return;
        InterlockedExchange(&g_range, v);
        ClampNow();
        Paint();
    }
    void SetFromX(int x)
    {
        SetRange(RANGE_MIN + (x - ARROW - KNOB_W / 2) * (RANGE_MAX - RANGE_MIN) / (W - 2 * ARROW - KNOB_W));
    }
    void Commit()
    {
        SaveRange();
        char b[64]; sprintf_s(b, "CAMRANGE: %ld%%", g_range); Log(b);
    }

    void Follow()
    {
        HWND game = Proxy_GameWnd(); RECT rc = {}; DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        const int mode = game && !IsIconic(game) && pid == GetCurrentProcessId() ? OptionsState(&rc) : MODE_HIDDEN;
        if (mode == MODE_HIDDEN)
        {
            if (IsWindowVisible(g_hWnd)) { ShowWindow(g_hWnd, SW_HIDE); if (g_drag) { g_drag = false; ReleaseCapture(); Commit(); } }
            return;
        }
        POINT p;
        if (mode == MODE_INLINE) { p.x = rc.left + 1; p.y = rc.top - ROW2 - H; }   // one row above the mouse slider
        else
        {
            p.x = rc.left + ((rc.right - rc.left) - W) / 2; p.y = rc.bottom + 2;
            RECT cr; GetClientRect(game, &cr);
            if (p.y + H > cr.bottom) p.y = rc.top - H - 2;              // no room below: above the window
            if (p.y < 0) p.y = rc.bottom - H - 4;                       // nor above: inside, at its bottom edge
        }
        ClientToScreen(game, &p);
        RECT wr; GetWindowRect(g_hWnd, &wr);
        if (!IsWindowVisible(g_hWnd) || wr.left != p.x || wr.top != p.y)
        {
            SetWindowPos(g_hWnd, HWND_TOPMOST, p.x, p.y, W, H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            Paint();
        }
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        const int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_LBUTTONDOWN:
        case WM_LBUTTONDBLCLK:
            if (y < ROW2) return 0;
            if (x < ARROW) { SetRange(g_range - 5); Commit(); }
            else if (x >= W - ARROW) { SetRange(g_range + 5); Commit(); }
            else { g_drag = true; SetCapture(h); SetFromX(x); }
            return 0;
        case WM_MOUSEMOVE: if (g_drag) SetFromX(x); return 0;
        case WM_LBUTTONUP: if (g_drag) { g_drag = false; ReleaseCapture(); Commit(); } return 0;
        case WM_SETCURSOR: SetCursor(LoadCursor(nullptr, IDC_ARROW)); return TRUE;
        case WM_TIMER: Follow(); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        // the zoom function: wait until the exe is unpacked, then replace it
        BYTE* at = (BYTE*)FN_ZOOM;
        static const BYTE orig[9] = { 0x55, 0x8B, 0xEC, 0x51, 0xA1, 0x4C, 0x58, 0x11, 0x01 };
        for (int i = 0; i < 1200 && memcmp(at, orig, 9) != 0; i++) Sleep(100);
        if (memcmp(at, orig, 9) != 0) { Log("CAMRANGE: zoom function bytes differ, not installed"); return 0; }
        DWORD old;
        if (!VirtualProtect(at, 5, PAGE_EXECUTE_READWRITE, &old)) return 0;
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)ZoomStub - (DWORD)(at + 5);
        VirtualProtect(at, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 5);
        char b[96]; sprintf_s(b, "CAMRANGE: zoom limit hook installed @007A9440, range %ld%%", g_range); Log(b);

        HINSTANCE hi = GetModuleHandleW(nullptr);
        WNDCLASSEXW wc = { sizeof(wc) }; wc.style = CS_DBLCLKS; wc.lpfnWndProc = WndProc; wc.hInstance = hi; wc.lpszClassName = L"NTT_CameraRange";
        RegisterClassExW(&wc);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, wc.lpszClassName, L"Camera Range",
            WS_POPUP, 0, 0, W, H, nullptr, nullptr, hi, nullptr);
        if (!g_hWnd) return 0;
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), W, -H, 1, 32, BI_RGB };
        void* bits = nullptr;
        g_dc = CreateCompatibleDC(nullptr); SelectObject(g_dc, CreateDIBSection(g_dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0));
        g_bits = (DWORD*)bits;
        if (!g_bits) return 0;
        SetTimer(g_hWnd, 1, 150, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }
}

void CamRange_Init()
{
    LoadRange();
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// pus_store.cpp click gate: clicks on the slider must not reach the game
bool CamRange_CursorOverPanel()
{
    POINT p;
    if (!g_hWnd || !IsWindowVisible(g_hWnd) || !GetCursorPos(&p)) return false;
    return g_drag || WindowFromPoint(p) == g_hWnd;
}
