// Job Change scroll (700112000 master kept / 700113000), client side. Server: GameServer\JobChangePanel.cpp,
// [Vendor] Hemes lua (SendJobChangePanel).
// WIZ_HSACS_HOOK 0xE9 + JOBCHANGE 0xF7:
//   C->S 0 [u8 type]            open request (right click on the scroll, rce_store.cpp)
//   S->C 0 [u8 type][u8 job]    open the panel; job = our class 1 warrior, 2 rogue, 3 mage, 4 priest, 5 kurian / portu
//   C->S 1 [u8 type][u8 job]    change to job 1..4
//   S->C 2 [u8 result]          1 done (the server disconnects the character), 2 no scroll, 3 already this class,
//                               4 items equipped, 5 failed
// Panel: skin build_jobchange_assets.py (jc_ui\*.pus, the old HSACSX class selection window jobchangedm.uif).
// A class is picked with two clicks on its button (Select, then Confirm): the change cannot be undone.
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <map>
#include <vector>
#include <deque>
#include "jc_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();
HWND Proxy_GameWnd();

namespace
{
    const BYTE  JC_SUBOP = 0xF7;
    const DWORD KO_SND_FNC = 0x00704070, KO_PTR_PKT = 0x01115914;   // CAPISocket::Send, CAPISocket*
    const char* const JOB_NAME[5] = { "", "Warrior", "Rogue", "Mage", "Priest" };

    struct Rc { int l, t, r, b; };
    Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }
    bool In(const int a[4], int x, int y) { return x >= a[0] && x < a[2] && y >= a[1] && y < a[3]; }

    CRITICAL_SECTION g_lock;                           // g_status, g_sendQ
    HWND  g_hWnd = nullptr;
    HDC   g_memDC = nullptr;
    void* g_bits = nullptr;
    enum { C_NONE, C_CLOSE, C_JOB1, C_JOB2, C_JOB3, C_JOB4 };   // C_JOB1 + n = card n (job n + 1)
    int   g_hover = C_NONE, g_pressed = C_NONE;
    bool  g_open = false, g_track = false, g_dragging = false;
    POINT g_dragFrom = {};
    HCURSOR g_gameCursor = nullptr;
    BYTE  g_type = 0, g_myJob = 0;                     // from the server's open packet
    int   g_armed = 0;                                 // job waiting for the second click
    std::string g_status; COLORREF g_statusColor = RGB(255, 255, 255);
    DWORD g_waitSince = 0;                             // request sent, waiting for the answer
    DWORD g_closeAt = 0;
    std::deque<std::vector<BYTE>> g_sendQ;
    const UINT WM_JC_SHOW = WM_APP + 71, WM_JC_REFRESH = WM_APP + 72;
    const DWORD ANSWER_WAIT_MS = 6000;

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
        for (const char* n : { "bg", "btn_n", "btn_o", "btn_d", "close_n", "close_o", "close_d" })
        {
            char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\jc_ui\\%s.pus", dir, n);
            Sprite s; if (LoadPus(p, s)) g_skin[n] = s;
            else { char b[MAX_PATH + 32]; sprintf_s(b, "JOBCHANGE: missing %s", p); Log(b); }
        }
    }
    void Skin(const char* n, int x, int y)
    {
        auto it = g_skin.find(n); if (it == g_skin.end()) return;
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(g_memDC, x, y, it->second.w, it->second.h, it->second.dc, 0, 0, it->second.w, it->second.h, bf);
    }
    // white text rendered on its own, coloured with a 1 px dark shadow into the panel bitmap (as tag_panel.cpp)
    void Text(const std::string& s, Rc r, COLORREF color, int pt, UINT fmt, bool bold = false)
    {
        int w = r.r - r.l, h = r.b - r.t;
        if (s.empty() || w <= 0 || h <= 0) return;
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
        void* bits = nullptr; HDC dc = CreateCompatibleDC(nullptr);
        HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        HGDIOBJ ob = SelectObject(dc, bmp);
        HFONT font = CreateFontW(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, L"Verdana");
        HGDIOBJ of = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
        RECT rc = { 0, 0, w, h };
        DrawTextA(dc, s.c_str(), (int)s.size(), &rc, fmt | DT_NOPREFIX);
        GdiFlush();
        for (int pass = 0; pass < 2; pass++)
        {
            int o = pass ? 0 : 1;
            BYTE cr = pass ? GetRValue(color) : 0, cg = pass ? GetGValue(color) : 0, cb = pass ? GetBValue(color) : 0;
            for (int y = 0; y < h; y++)
            {
                int dy = r.t + y + o; if (dy < 0 || dy >= JC_H) continue;
                const BYTE* src = (const BYTE*)bits + y * w * 4;
                for (int x = 0; x < w; x++, src += 4)
                {
                    int dx = r.l + x + o; int a = max(src[0], max(src[1], src[2]));
                    if (!a || dx < 0 || dx >= JC_W) continue;
                    if (!pass) a = a * 3 / 4;
                    BYTE* d = (BYTE*)g_bits + (dy * JC_W + dx) * 4;
                    d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                    d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
                }
            }
        }
        SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }
    void Dim(Rc r)                                     // our own class: card darkened
    {
        for (int y = max(r.t, 0); y < min(r.b, JC_H); y++)
        {
            BYTE* d = (BYTE*)g_bits + (y * JC_W + max(r.l, 0)) * 4;
            for (int x = max(r.l, 0); x < min(r.r, JC_W); x++, d += 4) { d[0] = d[0] * 2 / 5; d[1] = d[1] * 2 / 5; d[2] = d[2] * 2 / 5; }
        }
    }

    void Paint()
    {
        if (!g_hWnd || !g_memDC) return;
        memset(g_bits, 0, (size_t)JC_W * JC_H * 4);
        LoadSkin();
        Skin("bg", 0, 0);
        EnterCriticalSection(&g_lock);
        std::string status = g_status; COLORREF statusColor = g_statusColor;
        LeaveCriticalSection(&g_lock);
        const UINT mid = DT_CENTER | DT_VCENTER | DT_SINGLELINE;
        Text("Job Change", R4(JC_TITLE), RGB(255, 200, 90), 11, mid, true);
        Text(g_type == 1 ? "Select your new class. Your master skills will be closed; all equipped items must be taken off first."
                         : "Select your new class. All equipped items must be taken off first.",
             R4(JC_TEXT), RGB(255, 255, 255), 8, mid);
        for (int i = 0; i < 4; i++)
        {
            int job = i + 1, id = C_JOB1 + i;
            bool mine = job == g_myJob, off = mine || g_waitSince;
            Text(JOB_NAME[job], R4(JC_HEAD[i]), mine ? RGB(150, 150, 150) : RGB(255, 255, 255), 9, mid, true);
            Skin(g_hover == id && !off ? (g_pressed == id ? "btn_d" : "btn_o") : "btn_n", JC_BTN[i][0], JC_BTN[i][1]);
            Text(mine ? "Current" : g_armed == job ? "Confirm" : "Select", R4(JC_BTN[i]),
                 mine ? RGB(140, 140, 140) : g_armed == job ? RGB(255, 220, 90) : RGB(255, 255, 255), 8, mid, true);
            if (mine) Dim(R4(JC_CARD[i]));
        }
        Skin(g_hover == C_CLOSE ? (g_pressed == C_CLOSE ? "close_d" : "close_o") : "close_n", JC_CLOSE[0], JC_CLOSE[1]);
        Text(status, R4(JC_STATUS), statusColor, 9, mid);
        RECT wr; GetWindowRect(g_hWnd, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { JC_W, JC_H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
    }

    HWND FindGameWindow()
    {
        if (HWND h = Proxy_GameWnd()) return h;
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
    void Hide()
    {
        g_open = false; g_closeAt = 0; g_waitSince = 0; g_armed = 0;
        ShowWindow(g_hWnd, SW_HIDE);
    }
    void SetStatus(const char* s, COLORREF c)
    {
        EnterCriticalSection(&g_lock); g_status = s; g_statusColor = c; LeaveCriticalSection(&g_lock);
    }
    void Show()
    {
        g_open = true; g_closeAt = 0; g_waitSince = 0; g_armed = 0;
        SetStatus("", RGB(255, 255, 255));
        RECT gr = { 0, 0, 1024, 768 };
        if (HWND game = FindGameWindow()) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
        SetWindowPos(g_hWnd, HWND_TOPMOST, gr.left + ((gr.right - gr.left) - JC_W) / 2, gr.top + ((gr.bottom - gr.top) - JC_H) / 2,
                     JC_W, JC_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        Paint();
    }
    void SyncVisibility()
    {
        HWND game = FindGameWindow(); DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        bool want = g_open && game && !IsIconic(game) && pid == GetCurrentProcessId();
        if (want != (IsWindowVisible(g_hWnd) != FALSE))
        {
            if (want) { SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); Paint(); }
            else ShowWindow(g_hWnd, SW_HIDE);
        }
        POINT p; CURSORINFO ci = { sizeof(ci) };
        if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) && (ci.flags & CURSOR_SHOWING) && ci.hCursor)
            g_gameCursor = ci.hCursor;
        if (g_waitSince && GetTickCount() - g_waitSince > ANSWER_WAIT_MS)
        {
            g_waitSince = 0;
            SetStatus("No answer from the server, please try again.", RGB(255, 90, 70));
            Paint();
        }
        if (g_closeAt && GetTickCount() >= g_closeAt) Hide();
    }

    void Queue(std::vector<BYTE> out)
    {
        EnterCriticalSection(&g_lock);
        g_sendQ.push_back(std::move(out));
        LeaveCriticalSection(&g_lock);
        Proxy_RequestFlush();
    }

    void Click(int job)
    {
        if (g_waitSince || job == g_myJob) return;
        char b[160];
        if (g_armed != job)
        {
            g_armed = job;
            sprintf_s(b, "Change your class to %s? This cannot be undone. Click Confirm.", JOB_NAME[job]);
            SetStatus(b, RGB(255, 220, 90));
            return;
        }
        g_armed = 0;
        Queue({ 0xE9, JC_SUBOP, 1, g_type, (BYTE)job });
        g_waitSince = GetTickCount();
        SetStatus("Changing your class...", RGB(255, 255, 255));
        sprintf_s(b, "JOBCHANGE: request type=%u job=%d", g_type, job); Log(b);
    }

    int HitTest(int x, int y)
    {
        if (In(JC_CLOSE, x, y)) return C_CLOSE;
        for (int i = 0; i < 4; i++) if (In(JC_BTN[i], x, y)) return C_JOB1 + i;
        return C_NONE;
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
            if (int hit = HitTest(x, y); hit != g_hover) { g_hover = hit; Paint(); }
            return 0;
        case WM_MOUSELEAVE: g_track = false; if (g_hover != C_NONE) { g_hover = C_NONE; Paint(); } return 0;
        case WM_LBUTTONDOWN:
        {
            int hit = HitTest(x, y);
            SetCapture(h);
            if (hit == C_NONE) { g_dragging = true; GetCursorPos(&g_dragFrom); return 0; }
            g_pressed = hit; Paint();
            return 0;
        }
        case WM_LBUTTONUP:
        {
            ReleaseCapture();
            if (g_dragging) { g_dragging = false; return 0; }
            int hit = HitTest(x, y), p = g_pressed; g_pressed = C_NONE;
            if (p != C_NONE && p == hit)
            {
                if (hit == C_CLOSE) { Hide(); return 0; }
                Click(hit - C_JOB1 + 1);
            }
            Paint();
            return 0;
        }
        case WM_SETCURSOR:
            while (ShowCursor(TRUE) < 0) {}
            SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
            return TRUE;
        case WM_TIMER: SyncVisibility(); return 0;
        case WM_JC_SHOW: g_type = (BYTE)wp; g_myJob = (BYTE)lp; Show(); return 0;
        case WM_JC_REFRESH:
            if (wp == 1) g_closeAt = GetTickCount() + 2000;   // done: close a moment later
            if (g_open) Paint();
            return 0;
        case WM_CLOSE: Hide(); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        WNDCLASSEXW wc = { sizeof(wc) };
        wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr);
        wc.hCursor = LoadCursor(nullptr, IDC_ARROW); wc.lpszClassName = L"NTT_JobChangePanel";
        RegisterClassExW(&wc);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, wc.lpszClassName,
            L"Job Change", WS_POPUP, 0, 0, JC_W, JC_H, nullptr, nullptr, wc.hInstance, nullptr);
        if (!g_hWnd) { Log("JOBCHANGE: window creation failed"); return 0; }
        g_memDC = CreateCompatibleDC(nullptr);
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), JC_W, -JC_H, 1, 32, BI_RGB };
        SelectObject(g_memDC, CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
        SetTimer(g_hWnd, 1, 250, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }

    void Answer(const char* s, COLORREF c, bool done)
    {
        SetStatus(s, c);
        g_waitSince = 0;
        if (g_hWnd) PostMessageW(g_hWnd, WM_JC_REFRESH, done ? 1 : 0, 0);
    }
}

void JobChange_Init()
{
    InitializeCriticalSection(&g_lock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// game thread, right click on a Job Change scroll (rce_store.cpp): the server answers with the open packet
void JobChange_RequestOpen(BYTE type)
{
    Queue({ 0xE9, JC_SUBOP, 0, type });
}

// recv hook (game thread): E9 F7 ...
void JobChange_OnRecv(const BYTE* p, size_t len)
{
    if (len < 4 || p[0] != 0xE9 || p[1] != JC_SUBOP) return;
    if (p[2] == 0 && len == 5)
    {
        if (g_hWnd) PostMessageW(g_hWnd, WM_JC_SHOW, p[3], p[4]);
        Log("JOBCHANGE: panel opened by the server");
        return;
    }
    // Rust's legacy daily-login reward also uses F7 with a much larger payload.
    // Never interpret it as a job-change result.
    if (p[2] != 2 || len != 4) return;
    char lb[64]; sprintf_s(lb, "JOBCHANGE: result %u", p[3]); Log(lb);
    switch (p[3])
    {
    case 1: Answer("Your class has been changed. Please log in again.", RGB(120, 255, 120), true); break;
    case 2: Answer("You need a Job Change scroll.", RGB(255, 90, 70), false); break;
    case 3: Answer("You already are this class.", RGB(255, 200, 80), false); break;
    case 4: Answer("Take off all of your equipped items first.", RGB(255, 90, 70), false); break;
    default: Answer("Your class could not be changed.", RGB(255, 90, 70), false); break;
    }
}

void JobChange_FlushSendQueue()
{
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_lock);
    pending.swap(g_sendQ);
    LeaveCriticalSection(&g_lock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
    }
}

// pus_store.cpp click gate: clicks on the panel must not reach the game (DirectInput)
bool JobChange_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
