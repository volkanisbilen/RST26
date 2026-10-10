// Re-Connect for the 2625 client (d3d9 proxy). When the game loses the server it calls 0x7AFD90 (shows
// "Disconnected from server", text 1601; the only caller is the packet loop 0x83A86A). That entry is detoured:
// instead of the message our window "Reconnecting to the server" opens (skin build_reconnect_assets.py,
// reconnect_ui\*.pus, the game's tournament atlas) with a 10 s countdown, Reconnect and Exit.
// Reconnect: the last login (kept in memory by login_remember.cpp) is written DPAPI-encrypted to Reconnect.dat,
// the game is started again with the same command line and this process ends. The new process picks the
// file up (Reconnect_TakeLogin, max. 2 minutes old, deleted on read) and login_remember.cpp logs in by itself.
//
// Soft re-connect (tried first, the process stays open): the server hands out a one-shot token when the character
// enters the game (E9 EF 01 [16 token][str account][str character]). On a drop in the game the client's own
// server-change path is reused: CAPISocket::Connect 0x705D60 with the IP / port the socket object still holds,
// ~1 s later the version check 0x7AFB00. CGameProcMain answers the server's 0x2B by itself (crypto + CharacterSelect
// 0x04); that 0x04 is swapped for RESUME (E9 EF 02 [16 token][str account]) in the send hook. The server logs the
// account in and selects the character, Main's 0x04 handler loads the zone and starts the game, and at GameStart
// the server sends ACK (E9 EF 03 [u8 ok]). No ACK / ACK 0 / no connection within SOFT_LIMIT_MS -> the window
// above (hard re-connect).
// The window's Reconnect button and its countdown start the soft re-connect again (the game stays open). Restarting
// the game (hard) is only the last resort when a soft attempt is impossible (no valid token, e.g. the server refused it).
#include <windows.h>
#include <windowsx.h>
#include <wincrypt.h>
#include <stdio.h>
#include <time.h>
#include <string>
#include <map>
#include <vector>
#include "reconnect_layout.h"
#include "launch_ticket.h"
#include <bcrypt.h>
#pragma comment(lib, "bcrypt.lib")
#pragma comment(lib, "msimg32.lib")
#pragma comment(lib, "crypt32.lib")

void Log(const char* msg);
bool LoginRemember_GetLast(std::string& id, std::string& pw);   // login_remember.cpp
HWND Proxy_GameWnd();                                            // d3d9proxy.cpp (subclassed game window)

namespace
{
    const DWORD HOOK_AT = 0x007AFD90;
    const DWORD LOST_AT = 0x007AFF30;                       // tick: socket closed -> "disconnected" screen                       // show "Disconnected from server"
    const BYTE  kPrologue[10] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0x95, 0xBD, 0xF0, 0x00 };
    const int   COUNTDOWN = 10;
    const DWORD MAX_AGE_S = 120;
    const BYTE  kEntropy[] = "NTT-Reconnect";

    struct Rc { int l, t, r, b; };
    Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }

    HWND  g_hWnd = nullptr;
    HDC   g_memDC = nullptr;
    void* g_bits = nullptr;
    volatile LONG g_open = 0;
    volatile LONG g_blocked = 0;                            // HopeGuard ACS disconnected us: no re-connect
    volatile LONG g_softRequest = 0;                        // window thread asks the game thread for a soft attempt
    bool RequestSoft(const char* why);                      // below
    DWORD g_openedAt = 0;
    int   g_hover = 0, g_pressed = 0;                         // 1 reconnect, 2 exit
    bool  g_track = false, g_busy = false;
    HCURSOR g_cursor = nullptr;
    const UINT WM_RC_SHOW = WM_APP + 51, WM_RC_HIDE = WM_APP + 52;

    // soft re-connect
    const DWORD SOCK_PTR = 0x01115914, PROC_CUR = 0x01115910, PROC_MAIN = 0x011158FC;
    const DWORD FN_CONNECT = 0x00705D60, FN_VERSION = 0x007AFB00;
    const DWORD CHARSEL_SENT = 0x01111FCA, GS_CONNECTED = 0x011158DC;
    const UINT_PTR SOFT_TIMER = 0x4802;
    const DWORD CONNECT_ERR = 0x011158DD, CONNECT_ERR_CODE = 0x011158E0;   // socket thread: the connect failed (login screen message)
    const DWORD SOFT_LIMIT_MS = 45000, RETRY_MS = 3000, VERSION_DELAY_MS = 1000, ACK_WAIT_MS = 25000, LINK_WAIT_MS = 10000;
    const int   MAX_WAIT_DROPS = 3;
    enum { RS_IDLE, RS_CONNECT, RS_LINK, RS_VERSION, RS_WAIT };
    volatile LONG g_rs = RS_IDLE;
    volatile bool g_soft = false;                 // window shows the soft attempt (no countdown)
    DWORD g_rsStart = 0, g_rsStep = 0, g_lastTry = 0;
    int   g_tries = 0;
    int   g_waitDrops = 0;                        // closed by the server after the version check, per drop
    // the game server of the socket. CAPISocket::Disconnect 0x705FB0 (run by the socket thread when a connect
    // fails) clears the ip / port / window in the socket object, so the next try needs them from here
    struct { char ip[64], name[128]; DWORD port; HWND wnd; } g_ep = {};
    struct { BYTE token[16] = {}; bool valid = false; std::string account, character; } g_tok;
    DWORD g_resendAt = 0;                         // ACK 2 (old session still on the server): RESUME again
    CRITICAL_SECTION g_tokLock;
    typedef void(__thiscall* FnSendRaw)(void*, BYTE*, int);
    std::vector<BYTE> g_gameStart1;               // Main's 0D 01 of this attempt, replayed as 0D 02
    void* g_sendSock = nullptr;
    FnSendRaw g_sendFn = nullptr;

    std::string FilePath()
    {
        char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH);
        strcpy_s(strrchr(p, '\\') + 1, 32, "Reconnect.dat");
        return p;
    }

    // ---------------------------------------------------------------- skin
    struct Sprite { HDC dc = nullptr; int w = 0, h = 0; };
    std::map<std::string, Sprite> g_skin;
    bool LoadPus(const char* path, Sprite& s)
    {
        FILE* f = nullptr;
        if (fopen_s(&f, path, "rb") != 0 || !f) return false;
        char magic[4]; UINT32 w = 0, h = 0;
        bool ok = fread(magic, 1, 4, f) == 4 && !memcmp(magic, "PUSI", 4) && fread(&w, 4, 1, f) == 1 && fread(&h, 4, 1, f) == 1 && w && h && w < 4096 && h < 4096;
        if (ok)
        {
            BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), (LONG)w, -(LONG)h, 1, 32, BI_RGB };
            void* bits = nullptr; s.dc = CreateCompatibleDC(nullptr);
            HBITMAP bmp = CreateDIBSection(s.dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
            ok = bmp && fread(bits, 4, (size_t)w * h, f) == (size_t)w * h;
            SelectObject(s.dc, bmp); s.w = (int)w; s.h = (int)h;
        }
        fclose(f);
        return ok;
    }
    void LoadSkin()
    {
        if (!g_skin.empty()) return;
        char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH); *(strrchr(dir, '\\') + 1) = 0;
        for (const char* n : { "bg", "btn_n", "btn_o", "btn_d" })
        {
            char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\reconnect_ui\\%s.pus", dir, n);
            Sprite s; if (LoadPus(p, s)) g_skin[n] = s;
        }
    }
    void Skin(const char* n, int x, int y)
    {
        auto it = g_skin.find(n); if (it == g_skin.end()) return;
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(g_memDC, x, y, it->second.w, it->second.h, it->second.dc, 0, 0, it->second.w, it->second.h, bf);
    }
    void Text(const std::string& s, Rc r, COLORREF color, int pt, UINT fmt, bool bold = false)
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
        DrawTextA(dc, s.c_str(), (int)s.size(), &rc, fmt | DT_NOPREFIX | DT_WORDBREAK);
        GdiFlush();
        for (int pass = 0; pass < 2; pass++)
        {
            int o = pass ? 0 : 1;
            BYTE cr = pass ? GetRValue(color) : 0, cg = pass ? GetGValue(color) : 0, cb = pass ? GetBValue(color) : 0;
            for (int y = 0; y < h; y++)
            {
                int dy = r.t + y + o; if (dy < 0 || dy >= RC_H) continue;
                const BYTE* src = (const BYTE*)bits + y * w * 4;
                for (int x = 0; x < w; x++, src += 4)
                {
                    int dx = r.l + x + o; int a = max(src[0], max(src[1], src[2]));
                    if (!a || dx < 0 || dx >= RC_W) continue;
                    if (!pass) a = a * 3 / 4;
                    BYTE* d = (BYTE*)g_bits + (dy * RC_W + dx) * 4;
                    d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                    d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
                }
            }
        }
        SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }

    int SecondsLeft()
    {
        int el = (int)((GetTickCount() - g_openedAt) / 1000);
        return el >= COUNTDOWN ? 0 : COUNTDOWN - el;
    }

    void Paint()
    {
        if (!g_hWnd || !g_memDC) return;
        memset(g_bits, 0, (size_t)RC_W * RC_H * 4);
        LoadSkin();
        Skin("bg", 0, 0);
        Text("Reconnecting to the server", R4(RC_TITLE), RGB(255, 170, 60), 10, DT_CENTER | DT_VCENTER | DT_SINGLELINE, true);
        char b[160];
        sprintf_s(b, "We are trying to reconnect you to the server.\nAutomatic reconnect in 00:%02d seconds.", SecondsLeft());
        Rc t = R4(RC_TEXT);
        if (g_soft) sprintf_s(b, "Connection lost. Reconnecting your character...\nAttempt %d", g_tries < 1 ? 1 : g_tries);
        Text(g_busy ? std::string("Reconnecting...") : std::string(b), { t.l + 16, t.t + 18, t.r - 16, t.b - 8 }, RGB(255, 255, 255), 9, DT_CENTER);
        const int* br[2] = { RC_BTN_RECONNECT, RC_BTN_EXIT };
        bool waiting = g_soft || g_rs != RS_IDLE;           // soft attempt running: nothing to press
        const char* lbl[2] = { waiting ? "Please wait" : "Reconnect", "Exit" };
        for (int i = 0; i < 2; i++)
        {
            int id = i + 1;
            bool off = id == 1 && waiting;
            Skin(g_hover == id && !off ? (g_pressed == id ? "btn_d" : "btn_o") : "btn_n", br[i][0], br[i][1]);
            Text(lbl[i], R4(br[i]), off ? RGB(140, 130, 110) : RGB(255, 240, 210), 9, DT_CENTER | DT_VCENTER | DT_SINGLELINE, true);
        }
        RECT wr; GetWindowRect(g_hWnd, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { RC_W, RC_H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
    }

    HWND GameWindow()
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

    // ---------------------------------------------------------------- actions
    void SaveLogin()
    {
        std::string id, pw;
        if (!LoginRemember_GetLast(id, pw) || id.empty()) { Log("RECONNECT: no login in memory, manual login needed"); return; }
        std::string plain = id + '\n' + pw;
        DATA_BLOB in = { (DWORD)plain.size(), (BYTE*)plain.data() }, ent = { sizeof(kEntropy), (BYTE*)kEntropy }, out = {};
        BOOL ok = CryptProtectData(&in, L"KO reconnect", &ent, nullptr, nullptr, CRYPTPROTECT_UI_FORBIDDEN, &out);
        SecureZeroMemory(&plain[0], plain.size()); SecureZeroMemory(&pw[0], pw.size());
        if (!ok) return;
        FILE* f = nullptr;
        if (fopen_s(&f, FilePath().c_str(), "wb") == 0 && f)
        {
            UINT32 t = (UINT32)time(nullptr);
            fwrite(&t, 4, 1, f); fwrite(out.pbData, 1, out.cbData, f); fclose(f);
        }
        LocalFree(out.pbData);
    }

    void DoReconnect()
    {
        if (g_busy) return;
        g_busy = true; Paint();
        SaveLogin();
        wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
        std::wstring dir = exe; dir = dir.substr(0, dir.find_last_of(L'\\'));
        std::wstring cmd = GetCommandLineW();
        // the new client needs a launch ticket like a start from Launcher.exe (launch_guard.cpp checks it)
        LaunchTicket t = {};
        t.magic = LT_MAGIC; t.time = LtNow(); t.pid = GetCurrentProcessId();
        BCryptGenRandom(nullptr, t.rnd, sizeof(t.rnd), BCRYPT_USE_SYSTEM_PREFERRED_RNG);
        t.mac = LtMac(t);
        HKEY k;
        if (RegCreateKeyExW(HKEY_CURRENT_USER, LT_REG_KEY, 0, nullptr, 0, KEY_SET_VALUE, nullptr, &k, nullptr) == ERROR_SUCCESS)
        {
            RegSetValueExW(k, LT_REG_VALUE, 0, REG_BINARY, (const BYTE*)&t, sizeof(t));
            RegCloseKey(k);
        }
        STARTUPINFOW si = { sizeof(si) }; PROCESS_INFORMATION pi = {};
        if (CreateProcessW(exe, &cmd[0], nullptr, nullptr, FALSE, 0, nullptr, dir.c_str(), &si, &pi))
        {
            CloseHandle(pi.hThread); CloseHandle(pi.hProcess);
            Log("RECONNECT: new client started, closing this one");
            TerminateProcess(GetCurrentProcess(), 0);
        }
        Log("RECONNECT: starting the client failed");
        g_busy = false; Paint();
    }

    int HitTest(int x, int y)
    {
        auto in = [&](const int* r) { return x >= r[0] && x < r[2] && y >= r[1] && y < r[3]; };
        return in(RC_BTN_RECONNECT) ? 1 : in(RC_BTN_EXIT) ? 2 : 0;
    }

    bool GameInFront(HWND self)
    {
        HWND game = GameWindow(), fg = GetForegroundWindow();
        return game && !IsIconic(game) && (fg == game || fg == self);
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_MOUSEMOVE:
            if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
            if (int hit = HitTest(x, y); hit != g_hover) { g_hover = hit; Paint(); }
            return 0;
        case WM_MOUSELEAVE: g_track = false; g_hover = 0; Paint(); return 0;
        case WM_LBUTTONDOWN: g_pressed = HitTest(x, y); if (g_pressed) SetCapture(h); Paint(); return 0;
        case WM_LBUTTONUP:
        {
            ReleaseCapture();
            int hit = HitTest(x, y), p = g_pressed; g_pressed = 0;
            if (p && p == hit)
            {
                if (hit == 1)
                {
                    if (g_soft || g_rs != RS_IDLE) Log("RECONNECT: button ignored, soft attempt running");
                    else if (!RequestSoft("button")) DoReconnect();
                }
                else { Log("RECONNECT: exit"); TerminateProcess(GetCurrentProcess(), 0); }
            }
            Paint();
            return 0;
        }
        case WM_SETCURSOR:
            while (ShowCursor(TRUE) < 0) {}
            SetCursor(g_cursor ? g_cursor : LoadCursor(nullptr, IDC_ARROW));
            return TRUE;
        case WM_RC_SHOW:
        {
            if (!GameInFront(h)) return 0;                      // alt-tabbed: the timer shows it when the game is back
            RECT gr = { 0, 0, 1024, 768 };
            if (HWND game = GameWindow()) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
            SetWindowPos(h, HWND_TOPMOST, gr.left + ((gr.right - gr.left) - RC_W) / 2, gr.top + ((gr.bottom - gr.top) - RC_H) / 2,
                         RC_W, RC_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            Paint();
            return 0;
        }
        case WM_TIMER:
            if (g_open)
            {
                // topmost window: keep it on screen only while the game is the foreground window
                if (!GameInFront(h)) { if (IsWindowVisible(h)) ShowWindow(h, SW_HIDE); }
                else if (!IsWindowVisible(h)) SendMessageW(h, WM_RC_SHOW, 0, 0);
                else Paint();
                if (g_open && !g_soft && g_rs == RS_IDLE && SecondsLeft() == 0)
                {
                    g_openedAt = GetTickCount();                // next countdown if this attempt fails too
                    if (!RequestSoft("countdown")) DoReconnect();
                }
            }
            return 0;
        case WM_RC_HIDE:
            ShowWindow(h, SW_HIDE);
            return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        WNDCLASSEXW wc = { sizeof(wc) };
        wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr);
        wc.hCursor = LoadCursor(nullptr, IDC_ARROW); wc.lpszClassName = L"NTT_Reconnect";
        RegisterClassExW(&wc);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, wc.lpszClassName,
            L"Reconnect", WS_POPUP, 0, 0, RC_W, RC_H, nullptr, nullptr, wc.hInstance, nullptr);
        if (!g_hWnd) return 0;
        g_memDC = CreateCompatibleDC(nullptr);
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), RC_W, -RC_H, 1, 32, BI_RGB };
        SelectObject(g_memDC, CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
        SetTimer(g_hWnd, 1, 250, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }

    // ---------------------------------------------------------------- soft re-connect (game thread)
    typedef int(__thiscall* FnConnect)(void* sock, HWND wnd, const char* ip, DWORD port, const char* name);
    typedef void(__cdecl* FnVersion)();

    const char* MsvcStr(const BYTE* s) { return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s; }
    bool InMain() { return *(DWORD*)PROC_MAIN && *(DWORD*)PROC_CUR == *(DWORD*)PROC_MAIN; }

    void OpenWindow(bool soft)
    {
        g_soft = soft;
        g_openedAt = GetTickCount();
        if (InterlockedExchange(&g_open, 1) == 0)
        {
            CURSORINFO ci = { sizeof(ci) };
            if (GetCursorInfo(&ci)) g_cursor = ci.hCursor;
        }
        if (g_hWnd) PostMessageW(g_hWnd, WM_RC_SHOW, 0, 0);
    }

    // soft attempt over: hard re-connect window with the countdown
    void SoftFail(const char* why)
    {
        char b[160]; sprintf_s(b, "RECONNECT: soft re-connect failed (%s) -> reconnect window", why); Log(b);
        InterlockedExchange(&g_rs, RS_IDLE);
        if (HWND w = Proxy_GameWnd()) KillTimer(w, SOFT_TIMER);
        OpenWindow(false);
    }

    void InvalidateToken() { EnterCriticalSection(&g_tokLock); g_tok.valid = false; LeaveCriticalSection(&g_tokLock); }

    // socket object: +0x10 window, +0x14 handle, +0x3C ip, +0x54 name, +0x9C port, +0xA0 connected
    void KeepEndpoint()
    {
        __try
        {
            BYTE* sock = *(BYTE**)SOCK_PTR;
            if (!sock) return;
            const char* ip = MsvcStr(sock + 0x3C);
            DWORD port = *(DWORD*)(sock + 0x9C);
            if (!ip[0] || !port) return;                                     // cleared by Disconnect: keep the last one
            strncpy_s(g_ep.ip, ip, _TRUNCATE);
            strncpy_s(g_ep.name, MsvcStr(sock + 0x54), _TRUNCATE);
            g_ep.port = port;
            g_ep.wnd = *(HWND*)(sock + 0x10);
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
    }

    // 0 closed (Disconnect ran: the connect failed / the game closed it), 1 connecting, 2 connected
    int SockState()
    {
        __try
        {
            BYTE* sock = *(BYTE**)SOCK_PTR;
            if (!sock) return 0;
            DWORD h = *(DWORD*)(sock + 0x14);
            if (h == 0 || h == 0xFFFFFFFF) return 0;
            return *(DWORD*)(sock + 0xA0) ? 2 : 1;
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { return 0; }
    }

    // 1 connecting (the result comes from the socket thread, see SockState), 0 failed, -1 no server address
    int TryConnect()
    {
        __try
        {
            BYTE* sock = *(BYTE**)SOCK_PTR;
            if (!sock) return -1;
            KeepEndpoint();
            if (!g_ep.ip[0] || !g_ep.port) return -1;
            *(BYTE*)CHARSEL_SENT = 0;                                        // let Main send CharacterSelect again
            *(BYTE*)CONNECT_ERR = 0;
            int err = ((FnConnect)FN_CONNECT)(sock, g_ep.wnd, g_ep.ip, g_ep.port, g_ep.name);
            *(BYTE*)GS_CONNECTED = 1;
            char b[160]; sprintf_s(b, "RECONNECT: soft connect %s:%lu try %d -> %d", g_ep.ip, g_ep.port, g_tries, err); Log(b);
            return err == 0 ? 1 : 0;
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { Log("RECONNECT: exception in soft connect"); return 0; }
    }

    // the connection closed again during a soft attempt. After the version check it is the server closing it (e.g. it
    // was restarted and does not know the token): after MAX_WAIT_DROPS the game is restarted instead (hard re-connect)
    void DropInAttempt()
    {
        if (g_rs == RS_WAIT && ++g_waitDrops >= MAX_WAIT_DROPS)
        {
            InvalidateToken();
            return SoftFail("the server closes the connection");
        }
        Log("RECONNECT: connection lost during the soft attempt, retrying");
        InterlockedExchange(&g_rs, RS_CONNECT);
    }

    // RESUME: [16 token][str account]
    void SendResume(void* sock, FnSendRaw sendFn)
    {
        std::vector<BYTE> p = { 0xE9, 0xEF, 0x02 };
        EnterCriticalSection(&g_tokLock);
        p.insert(p.end(), g_tok.token, g_tok.token + 16);
        std::string acc = g_tok.account;
        LeaveCriticalSection(&g_tokLock);
        WORD n = (WORD)acc.size();
        p.insert(p.end(), (BYTE*)&n, (BYTE*)&n + 2);
        p.insert(p.end(), acc.begin(), acc.end());
        sendFn(sock, p.data(), (int)p.size());
    }

    void SoftTick()
    {
        DWORD now = GetTickCount();
        switch (g_rs)
        {
        case RS_CONNECT:
            if (now - g_rsStart > SOFT_LIMIT_MS) return SoftFail("no connection");
            if (g_lastTry && now - g_lastTry < RETRY_MS) return;
            g_lastTry = now; g_tries++;
            if (g_hWnd) PostMessageW(g_hWnd, WM_RC_SHOW, 0, 0);            // repaint "Attempt n"
            switch (TryConnect())
            {
            case 1: g_rsStep = now; InterlockedExchange(&g_rs, RS_LINK); break;
            case -1: InvalidateToken(); return SoftFail("no server address");   // soft is impossible: restart the game
            }
            return;
        case RS_LINK:                                                        // the connect is asynchronous
            switch (SockState())
            {
            case 2: g_rsStep = now; InterlockedExchange(&g_rs, RS_VERSION); return;
            case 0:                                                          // refused / unreachable: server still down
            {
                char b[96]; sprintf_s(b, "RECONNECT: connect failed (%lu), next try", *(DWORD*)CONNECT_ERR_CODE); Log(b);
                *(BYTE*)CONNECT_ERR = 0;
                InterlockedExchange(&g_rs, RS_CONNECT);
                return;
            }
            }
            if (now - g_rsStep > LINK_WAIT_MS) InterlockedExchange(&g_rs, RS_CONNECT);
            return;
        case RS_VERSION:                                                     // the game waits ~1 s too (0x7B91E3)
            if (SockState() != 2) return DropInAttempt();
            if (now - g_rsStep < VERSION_DELAY_MS) return;
            __try { ((FnVersion)FN_VERSION)(); }
            __except (EXCEPTION_EXECUTE_HANDLER) { return SoftFail("version check"); }
            g_rsStep = now; InterlockedExchange(&g_rs, RS_WAIT);
            Log("RECONNECT: version check sent, waiting for the character");
            return;
        case RS_WAIT:
            if (SockState() == 0) return DropInAttempt();
            if (g_resendAt && now >= g_resendAt && g_sendFn)
            {
                g_resendAt = 0; g_rsStep = now;
                SendResume(g_sendSock, g_sendFn);
                return;
            }
            if (now - g_rsStep > ACK_WAIT_MS) return SoftFail("no ACK");
            return;
        }
    }

    // ---------------------------------------------------------------- hook (game thread)
    typedef void(__thiscall* FnShow)(void*);
    typedef void(__cdecl* FnLost)();
    FnShow g_tramp = nullptr;
    FnLost g_trampLost = nullptr;

    bool TokenValid()
    {
        EnterCriticalSection(&g_tokLock); bool v = g_tok.valid; LeaveCriticalSection(&g_tokLock);
        return v;
    }

    // game thread: start a soft attempt (window shows "Reconnecting your character...")
    void StartSoft(HWND gw)
    {
        g_tries = 0; g_lastTry = 0; g_rsStart = GetTickCount();
        InterlockedExchange(&g_rs, RS_CONNECT);
        OpenWindow(true);
        SetTimer(gw, SOFT_TIMER, 250, nullptr);
    }

    // window thread (Reconnect button / countdown): soft attempt on the game thread through its SOFT_TIMER route.
    // false: soft is not possible (no valid token / no game window / not in the game) -> the caller restarts the game
    bool RequestSoft(const char* why)
    {
        HWND gw = Proxy_GameWnd();
        if (g_blocked || !gw || !TokenValid() || !InMain())
        {
            char m[128]; sprintf_s(m, "RECONNECT: %s: soft not possible (token=%d wnd=%d main=%d) -> restart", why, (int)TokenValid(), gw != nullptr, (int)InMain());
            Log(m);
            return false;
        }
        InterlockedExchange(&g_softRequest, 1);
        PostMessageW(gw, WM_TIMER, SOFT_TIMER, 0);
        char m[96]; sprintf_s(m, "RECONNECT: %s -> soft re-connect", why); Log(m);
        return true;
    }

    // true: the soft re-connect takes the drop (started, or the running attempt retries)
    bool SoftTakeDrop(const char* from)
    {
        bool haveToken;
        EnterCriticalSection(&g_tokLock); haveToken = g_tok.valid; LeaveCriticalSection(&g_tokLock);
        if (g_blocked) return false;                      // kicked by HopeGuard ACS: the game's own message
        if (g_rs != RS_IDLE)
        {
            DropInAttempt();
            return true;
        }
        HWND gw = Proxy_GameWnd();
        if (!haveToken || !gw || !InMain() || g_open)
        {
            char m[160]; sprintf_s(m, "RECONNECT: drop (%s) not soft: token=%d wnd=%d main=%d open=%d", from, (int)haveToken, gw != nullptr, (int)InMain(), (int)g_open);
            Log(m);
            return false;
        }
        char b[96]; sprintf_s(b, "RECONNECT: disconnected in the game (%s) -> soft re-connect", from); Log(b);
        g_waitDrops = 0;
        KeepEndpoint();
        StartSoft(gw);
        return true;
    }

    // 0x7AFF30: the tick saw the socket closed (server lost) -> the game's own "disconnected" screen
    void __cdecl HkLost()
    {
        if (SoftTakeDrop("tick")) return;
        if (g_trampLost) g_trampLost();
    }

    void __fastcall HkDisconnected(void* self, void*)
    {
        if (g_blocked) { Log("RECONNECT: kicked by HopeGuard ACS, no re-connect"); if (g_tramp) g_tramp(self); return; }
        if (SoftTakeDrop("message")) return;
        if (!g_open)
        {
            Log("RECONNECT: disconnected from the server -> reconnect window");
            OpenWindow(false);
        }
        if (!g_hWnd && g_tramp) g_tramp(self);           // no window (thread failed): the game's own message
    }

    DWORD WINAPI InstallThread(LPVOID)
    {
        BYTE* at = (BYTE*)HOOK_AT;
        for (int i = 0; i < 240 && memcmp(at, kPrologue, sizeof(kPrologue)) != 0; i++) Sleep(250);   // exe unpacking
        if (memcmp(at, kPrologue, sizeof(kPrologue)) != 0) { Log("RECONNECT: disconnect function differs, not installed"); return 0; }
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 32, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return 0;
        memcpy(tramp, kPrologue, sizeof(kPrologue));
        tramp[10] = 0xE9; *(DWORD*)(tramp + 11) = (HOOK_AT + 10) - (DWORD)(tramp + 15);
        g_tramp = (FnShow)tramp;
        DWORD old;
        VirtualProtect(at, 10, PAGE_EXECUTE_READWRITE, &old);
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)HkDisconnected - (HOOK_AT + 5);
        memset(at + 5, 0x90, 5);
        VirtualProtect(at, 10, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 10);
        Log("RECONNECT: disconnect hook installed");

        static const BYTE kLost[6] = { 0x53, 0x8B, 0xDC, 0x83, 0xEC, 0x08 };   // push ebx / mov ebx,esp / sub esp,8
        BYTE* lost = (BYTE*)LOST_AT;
        if (memcmp(lost, kLost, 6) == 0)
        {
            BYTE* t2 = (BYTE*)VirtualAlloc(nullptr, 32, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
            if (t2)
            {
                memcpy(t2, kLost, 6);
                t2[6] = 0xE9; *(DWORD*)(t2 + 7) = (LOST_AT + 6) - (DWORD)(t2 + 11);
                g_trampLost = (FnLost)t2;
                VirtualProtect(lost, 6, PAGE_EXECUTE_READWRITE, &old);
                lost[0] = 0xE9; *(DWORD*)(lost + 1) = (DWORD)HkLost - (LOST_AT + 5);
                lost[5] = 0x90;
                VirtualProtect(lost, 6, old, &old);
                FlushInstructionCache(GetCurrentProcess(), lost, 6);
                Log("RECONNECT: connection-lost hook installed");
            }
        }
        else Log("RECONNECT: connection-lost function differs, not hooked");
        WindowThread(nullptr);
        return 0;
    }
}

// new process: the login saved by the previous one (max. MAX_AGE_S old); the file is deleted on read
bool Reconnect_TakeLogin(std::string& id, std::string& pw)
{
    std::string path = FilePath();
    FILE* f = nullptr;
    if (fopen_s(&f, path.c_str(), "rb") != 0 || !f) return false;
    UINT32 t = 0; std::string blob;
    if (fread(&t, 4, 1, f) == 1) { char buf[4096]; size_t n = fread(buf, 1, sizeof(buf), f); blob.assign(buf, n); }
    fclose(f);
    DeleteFileA(path.c_str());
    if (!t || (UINT32)time(nullptr) - t > MAX_AGE_S || blob.empty()) return false;
    DATA_BLOB in = { (DWORD)blob.size(), (BYTE*)blob.data() }, ent = { sizeof(kEntropy), (BYTE*)kEntropy }, out = {};
    if (!CryptUnprotectData(&in, nullptr, &ent, nullptr, nullptr, CRYPTPROTECT_UI_FORBIDDEN, &out)) return false;
    std::string plain((const char*)out.pbData, out.cbData);
    SecureZeroMemory(out.pbData, out.cbData); LocalFree(out.pbData);
    size_t nl = plain.find('\n');
    if (nl == std::string::npos) return false;
    id = plain.substr(0, nl); pw = plain.substr(nl + 1);
    SecureZeroMemory(&plain[0], plain.size());
    return !id.empty();
}

void Reconnect_Init()
{
    InitializeCriticalSection(&g_tokLock);
    CloseHandle(CreateThread(nullptr, 0, InstallThread, nullptr, 0, nullptr));
}

// recv hook (game thread): E9 EF 01 token / E9 EF 03 ACK
void Reconnect_OnRecv(const BYTE* p, size_t len)
{
    // GameStart answer: the client of this build sends 0D 02 only on the normal login path, so it is sent here
    if (len >= 1 && p[0] == 0x0D && g_rs == RS_WAIT && !g_gameStart1.empty() && g_sendFn)
    {
        std::vector<BYTE> gs2 = g_gameStart1;
        gs2[1] = 2;
        g_gameStart1.clear();
        g_sendFn(g_sendSock, gs2.data(), (int)gs2.size());
        Log("RECONNECT: GameStart 2 sent");
        return;
    }
    if (len < 3 || p[0] != 0xE9 || p[1] != 0xEF) return;
    if (p[2] == 1 && len >= 3 + 16 + 4)
    {
        size_t at = 3;
        BYTE token[16]; memcpy(token, p + at, 16); at += 16;
        auto str = [&](std::string& out) {
            if (at + 2 > len) return false;
            WORD n = *(const WORD*)(p + at); at += 2;
            if (at + n > len || n > 64) return false;
            out.assign((const char*)p + at, n); at += n; return true;
        };
        std::string acc, chr;
        if (!str(acc) || !str(chr)) return;
        EnterCriticalSection(&g_tokLock);
        memcpy(g_tok.token, token, 16); g_tok.valid = true; g_tok.account = acc; g_tok.character = chr;
        LeaveCriticalSection(&g_tokLock);
        KeepEndpoint();                                  // in the game: the socket holds the game server
        Log("RECONNECT: token received");
    }
    else if (p[2] == 3 && len >= 4)
    {
        if (g_rs == RS_IDLE) return;
        if (!p[3]) { InvalidateToken(); return SoftFail("refused by the server"); }
        if (p[3] == 2) { g_resendAt = GetTickCount() + 2000; Log("RECONNECT: old session still on the server, RESUME again in 2 s"); return; }
        InterlockedExchange(&g_rs, RS_IDLE);
        if (HWND w = Proxy_GameWnd()) KillTimer(w, SOFT_TIMER);
        InterlockedExchange(&g_open, 0);
        g_soft = false;
        if (g_hWnd) PostMessageW(g_hWnd, WM_RC_HIDE, 0, 0);
        char b[96]; sprintf_s(b, "RECONNECT: soft re-connect done after %d tries", g_tries); Log(b);
    }
}

// ProxyWndProc (game thread): true when the WM_TIMER was ours
bool Reconnect_OnTimer(HWND, UINT_PTR id)
{
    if (id != SOFT_TIMER) return false;
    if (InterlockedExchange(&g_softRequest, 0) && g_rs == RS_IDLE)   // Reconnect button / countdown
    {
        if (HWND w = Proxy_GameWnd()) StartSoft(w);
        return true;
    }
    if (g_rs == RS_IDLE) { if (HWND w = Proxy_GameWnd()) KillTimer(w, SOFT_TIMER); return true; }
    SoftTick();
    return true;
}

// send hook (game thread): while waiting, Main's CharacterSelect (0x04) is replaced by RESUME. sendFn sends raw.
bool Reconnect_FilterSend(void* sock, const BYTE* buf, int len, void(__thiscall* sendFn)(void*, BYTE*, int))
{
    if (g_rs != RS_WAIT || !buf || len < 1) return false;
    if (buf[0] == 0x0D && len >= 2 && buf[1] == 1)
    {
        g_gameStart1.assign(buf, buf + len);
        g_sendSock = sock; g_sendFn = sendFn;
        Log("RECONNECT: GameStart 1 sent by the game");
        return false;
    }
    if (buf[0] != 0x04) return false;
    g_gameStart1.clear();
    g_sendSock = sock; g_sendFn = sendFn; g_resendAt = 0;
    SendResume(sock, sendFn);
    Log("RECONNECT: CharacterSelect replaced by RESUME");
    return true;
}

// launch_guard.cpp: the server announced a HopeGuard ACS disconnect
void Reconnect_Block() { InterlockedExchange(&g_blocked, 1); }

bool Reconnect_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
