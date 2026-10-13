// Level and HP numbers on the target's HP bar. The HP comes from the game's own WIZ_TARGET_HP answers
// (0x22: u32 id, u8 echo, u32 max, u32 current), the level from the server's hook packet
// 0xE9 / 0xB6 (u32 id, u16 level). The text sits in a click-through layered window centred on the
// target bar (top centre of the game window). Position tuning without a rebuild: HopeGuard\target_hp.ini
// ([text] x=, y=, w=, h=; x/y = offset of the text's centre from the window's top centre / top edge).
#include <windows.h>
#include <stdio.h>
#include <string>

void Log(const char* msg);
HWND Proxy_GameWnd();
UINT32 DropBtn_CurrentTargetId();

namespace
{
    CRITICAL_SECTION g_lock;
    UINT32 g_id = 0, g_max = 0, g_cur = 0, g_levelId = 0;
    UINT16 g_level = 0;
    int g_x = 0, g_y = 46, g_w = 240, g_h = 18;

    HWND g_hWnd = nullptr; HDC g_dc = nullptr; void* g_bits = nullptr;
    std::string g_shown;

    void LoadIni()
    {
        char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH); *(strrchr(p, '\\') + 1) = 0;
        strcat_s(p, "HopeGuard\\target_hp.ini");
        g_x = GetPrivateProfileIntA("text", "x", g_x, p); g_y = GetPrivateProfileIntA("text", "y", g_y, p);
        g_w = GetPrivateProfileIntA("text", "w", g_w, p); g_h = GetPrivateProfileIntA("text", "h", g_h, p);
    }

    std::string Num(UINT32 v)
    {
        char b[24]; sprintf_s(b, "%u", v);
        std::string s = b, out;
        for (size_t i = 0; i < s.size(); i++)
        {
            if (i && (s.size() - i) % 3 == 0) out += '.';
            out += s[i];
        }
        return out;
    }

    // white text into a premultiplied bitmap, then a dark shadow and the colour (same technique as drop_button.cpp)
    void Render(const std::string& text)
    {
        memset(g_bits, 0, (size_t)g_w * g_h * 4);
        if (text.empty()) return;
        std::wstring ws(text.begin(), text.end());
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), g_w, -g_h, 1, 32, BI_RGB };
        void* bits = nullptr; HDC dc = CreateCompatibleDC(nullptr);
        HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        HGDIOBJ ob = SelectObject(dc, bmp);
        HFONT font = CreateFontW(-11, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, L"Verdana");
        HGDIOBJ of = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
        RECT rc = { 0, 0, g_w, g_h };
        DrawTextW(dc, ws.c_str(), (int)ws.size(), &rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        GdiFlush();
        for (int pass = 0; pass < 2; pass++)
        {
            int o = pass ? 0 : 1;
            BYTE cr = pass ? 255 : 0, cg = pass ? 255 : 0, cb = pass ? 255 : 0;
            for (int y = 0; y < g_h; y++)
            {
                int dy = y + o; if (dy >= g_h) continue;
                const BYTE* src = (const BYTE*)bits + y * g_w * 4;
                for (int x = 0; x < g_w; x++, src += 4)
                {
                    int dx = x + o; int a = max(src[0], max(src[1], src[2]));
                    if (!a || dx >= g_w) continue;
                    BYTE* d = (BYTE*)g_bits + (dy * g_w + dx) * 4;
                    d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                    d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
                }
            }
        }
        SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }

    void Sync()
    {
        HWND game = Proxy_GameWnd(); DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        bool front = game && !IsIconic(game) && pid == GetCurrentProcessId();
        UINT32 target = DropBtn_CurrentTargetId();
        std::string text;
        EnterCriticalSection(&g_lock);
        if (front && target && target == g_id && g_max)
        {
            char lv[24] = "";
            if (g_levelId == g_id && g_level) sprintf_s(lv, "Lv %u   ", (unsigned)g_level);
            text = std::string(lv) + Num(g_cur) + " / " + Num(g_max);
        }
        LeaveCriticalSection(&g_lock);
        if (text.empty()) { if (IsWindowVisible(g_hWnd)) ShowWindow(g_hWnd, SW_HIDE); g_shown.clear(); return; }
        RECT gr; GetClientRect(game, &gr); MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
        int x = gr.left + (gr.right - gr.left) / 2 + g_x - g_w / 2, y = gr.top + g_y - g_h / 2;
        RECT wr; GetWindowRect(g_hWnd, &wr);
        bool moved = !IsWindowVisible(g_hWnd) || wr.left != x || wr.top != y;
        if (text != g_shown) { g_shown = text; Render(text); moved = true; }
        if (!moved) return;
        POINT dst = { x, y }, src = { 0, 0 }; SIZE sz = { g_w, g_h };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_dc, &src, 0, &bf, ULW_ALPHA);
        if (!IsWindowVisible(g_hWnd)) SetWindowPos(g_hWnd, HWND_TOPMOST, x, y, g_w, g_h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }

    LRESULT CALLBACK Proc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        if (msg == WM_TIMER) { Sync(); return 0; }
        if (msg == WM_MOUSEACTIVATE) return MA_NOACTIVATE;
        if (msg == WM_NCHITTEST) return HTTRANSPARENT;
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        HINSTANCE hi = GetModuleHandleW(nullptr);
        WNDCLASSEXW c = { sizeof(c) }; c.lpfnWndProc = Proc; c.hInstance = hi; c.lpszClassName = L"NTT_TargetHp";
        RegisterClassExW(&c);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            c.lpszClassName, L"TargetHp", WS_POPUP, 0, 0, g_w, g_h, nullptr, nullptr, hi, nullptr);
        if (!g_hWnd) return 0;
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), g_w, -g_h, 1, 32, BI_RGB };
        g_dc = CreateCompatibleDC(nullptr); SelectObject(g_dc, CreateDIBSection(g_dc, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
        SetTimer(g_hWnd, 1, 100, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }
}

void TargetHp_Init()
{
    InitializeCriticalSection(&g_lock);
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") != 0) return;
    LoadIni();
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// hkRecv (game thread): pkt[0] = opcode
void TargetHp_OnRecv(const BYTE* pkt, int len)
{
    if (pkt[0] == 0x22 && len >= 14)                      // WIZ_TARGET_HP
    {
        EnterCriticalSection(&g_lock);
        memcpy(&g_id, pkt + 1, 4); memcpy(&g_max, pkt + 6, 4); memcpy(&g_cur, pkt + 10, 4);
        LeaveCriticalSection(&g_lock);
    }
    else if (pkt[0] == 0xE9 && len >= 8 && pkt[1] == 0xB6) // server hook: level of the target
    {
        EnterCriticalSection(&g_lock);
        memcpy(&g_levelId, pkt + 2, 4); memcpy(&g_level, pkt + 6, 2);
        LeaveCriticalSection(&g_lock);
    }
}
