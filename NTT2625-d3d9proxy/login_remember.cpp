// "Remember Me" tick for the login screen (CUILoginIntro, vtable 0x102F7B8; UI el/ka_login_intro_us.uif).
// The native login box (Group_LogIn: Edit_ID, Edit_PW, btn_ok ...) has no such option, so:
//   vtable[35] +0x8C Tick (0xBD1D40)            -> game thread, every frame: fills the saved ID / password into
//                                                 the edits right after the box shows up, reports where the box is
//   vtable[33] +0x84 ReceiveMessage (0xBD2C70)  -> btn_ok click (msg 1, sender this+0x120) or Enter in an edit
//                                                 (msg 0x1000): saves ID / password when the tick is on
// The tick itself is a small GDI layered window (same technique as cind_panel.cpp) under btn_homepage.
// CUILoginIntro: Edit_ID +0x118, Edit_PW +0x11C, btn_ok +0x120. CN3UIEdit vtable +0xD0 GetString (password edits
// return the real text), +0xD4 SetString (masks password edits itself); SetFocus 0x6E0690 (thiscall), focused edit
// [0x111523C]. CN3UI: visible byte +0xEA, rect floats +0xC8 (window coordinates), id +0x58, children +0xB0.
// Stored in LoginRemember.dat next to the exe; the password is encrypted with DPAPI (current Windows user).
#include <windows.h>
#include <windowsx.h>
#include <wincrypt.h>
#include <stdio.h>
#include <string>
#include <vector>
#pragma comment(lib, "crypt32.lib")

void Log(const char* msg);
bool Reconnect_TakeLogin(std::string& id, std::string& pw);    // reconnect.cpp
void Register_LoginTick(const float* homepage, bool covered, int idMax, int pwMax, int nativeState);   // register_panel.cpp
void Register_Open();
bool Register_TakeFill(std::string& id, std::string& pw);
bool Register_WantCredentials();
void Register_SetCredentials(const std::string& id, const std::string& pw);
void Register_FlushSendQueue();

namespace
{
    const DWORD VT_LOGIN = 0x0102F7B8;
    const int   SLOT_RM = 33, SLOT_TICK = 35;
    const DWORD ORIG_RM = 0x00BD2C70, ORIG_TICK = 0x00BD1D40;
    const DWORD OFF_EDIT_ID = 0x118, OFF_EDIT_PW = 0x11C, OFF_BTN_OK = 0x120;
    const DWORD OFF_EDIT_MAXLEN = 0x130;
    const DWORD OFF_UI_STATE = 0xBC;            // CN3UIBase state (SetState 0x4DC2B0)        // CN3UIEdit max length (SetString 0x6DBC70 cuts longer text)
    const DWORD KO_FOCUSED_EDIT = 0x0111523C;
    const DWORD KO_EDIT_SETFOCUS = 0x006E0690;
    const DWORD MSG_BUTTON_CLICK = 1, MSG_EDIT_RETURN = 0x1000;
    const DWORD FILL_WINDOW_MS = 3000;          // edits are (re)filled while empty this long after the box shows
    const int   TICK_W = 150, TICK_H = 20;

    typedef void(__thiscall* FnTick)(void*);
    typedef bool(__thiscall* FnRM)(void*, void*, DWORD);
    typedef void(__thiscall* FnSetFocus)(void*);
    FnTick g_origTick = nullptr;
    FnRM   g_origRM = nullptr;

    // ---------------------------------------------------------------- settings (g_lock)
    CRITICAL_SECTION g_lock;
    bool        g_enabled = false;
    std::string g_id, g_pw;
    std::string g_lastId, g_lastPw;            // last login of this session (memory only) -> Re-Connect

    std::string DataPath()
    {
        char p[MAX_PATH];
        GetModuleFileNameA(nullptr, p, MAX_PATH);
        char* s = strrchr(p, '\\');
        if (s) strcpy_s(s + 1, MAX_PATH - (s + 1 - p), "LoginRemember.dat");
        return p;
    }

    const BYTE kEntropy[] = "NTT-LoginRemember";

    std::string Protect(const std::string& plain)
    {
        DATA_BLOB in = { (DWORD)plain.size(), (BYTE*)plain.data() }, ent = { sizeof(kEntropy), (BYTE*)kEntropy }, out = {};
        if (!CryptProtectData(&in, L"KO login", &ent, nullptr, nullptr, CRYPTPROTECT_UI_FORBIDDEN, &out)) return "";
        std::string hex;
        char b[3];
        for (DWORD i = 0; i < out.cbData; i++) { sprintf_s(b, "%02X", out.pbData[i]); hex += b; }
        LocalFree(out.pbData);
        return hex;
    }

    std::string Unprotect(const std::string& hex)
    {
        std::vector<BYTE> raw;
        for (size_t i = 0; i + 1 < hex.size(); i += 2) raw.push_back((BYTE)strtoul(hex.substr(i, 2).c_str(), nullptr, 16));
        if (raw.empty()) return "";
        DATA_BLOB in = { (DWORD)raw.size(), raw.data() }, ent = { sizeof(kEntropy), (BYTE*)kEntropy }, out = {};
        if (!CryptUnprotectData(&in, nullptr, &ent, nullptr, nullptr, CRYPTPROTECT_UI_FORBIDDEN, &out)) return "";
        std::string s((const char*)out.pbData, out.cbData);
        SecureZeroMemory(out.pbData, out.cbData);
        LocalFree(out.pbData);
        return s;
    }

    // file: line 1 "1"/"0", line 2 ID, line 3 DPAPI(password) as hex
    void LoadSettings()
    {
        FILE* f = nullptr;
        if (fopen_s(&f, DataPath().c_str(), "rb") != 0 || !f) return;
        char line[2048];
        std::vector<std::string> l;
        while (l.size() < 3 && fgets(line, sizeof(line), f))
        {
            std::string s = line;
            while (!s.empty() && (s.back() == '\n' || s.back() == '\r')) s.pop_back();
            l.push_back(s);
        }
        fclose(f);
        EnterCriticalSection(&g_lock);
        g_enabled = l.size() > 0 && l[0] == "1";
        g_id = l.size() > 1 ? l[1] : "";
        g_pw = l.size() > 2 ? Unprotect(l[2]) : "";
        LeaveCriticalSection(&g_lock);
    }

    void SaveSettings()
    {
        EnterCriticalSection(&g_lock);
        std::string body = std::string(g_enabled ? "1" : "0") + "\r\n" + g_id + "\r\n" + (g_pw.empty() ? "" : Protect(g_pw)) + "\r\n";
        LeaveCriticalSection(&g_lock);
        FILE* f = nullptr;
        if (fopen_s(&f, DataPath().c_str(), "wb") == 0 && f)
        {
            fwrite(body.data(), 1, body.size(), f);
            fclose(f);
        }
    }

    // ---------------------------------------------------------------- CN3UI helpers
    inline void* Ptr(void* o, DWORD off) { return *(void**)((BYTE*)o + off); }
    inline bool Visible(void* o) { return o && *((BYTE*)o + 0xEA) != 0; }
    inline const float* RectOf(void* o) { return (const float*)((BYTE*)o + 0xC8); }

    const char* MsvcStr(const BYTE* s)     // MSVC x86 std::string: buf[16] | size | capacity
    {
        return *(const DWORD*)(s + 0x14) > 15 ? *(const char* const*)s : (const char*)s;
    }
    const char* IdOf(void* o) { return MsvcStr((const BYTE*)o + 0x58); }

    void* Find(void* parent, const char* id)
    {
        if (!parent) return nullptr;
        BYTE* head = (BYTE*)Ptr(parent, 0xB0);
        if (!head) return nullptr;
        for (BYTE* n = *(BYTE**)head; n && n != head; n = *(BYTE**)n)
            if (void* c = *(void**)(n + 8))
                if (_stricmp(IdOf(c), id) == 0) return c;      // the game matches ids case-insensitively
                                                                // (re_login_intro.uif says "Group_Login")
        return nullptr;
    }

    std::string EditText(void* edit)
    {
        if (!edit) return "";
        void** vt = *(void***)edit;
        const BYTE* s = ((const BYTE* (__thiscall*)(void*))vt[0xD0 / 4])(edit);
        return s ? std::string(MsvcStr(s), *(const DWORD*)(s + 0x10)) : std::string();
    }

    void SetEditText(void* edit, const std::string& s)
    {
        if (!edit) return;
        void** vt = *(void***)edit;
        ((void(__thiscall*)(void*, const std::string*))vt[0xD4 / 4])(edit, &s);
    }

    // ---------------------------------------------------------------- shared with the tick window
    volatile LONG g_anchorX = 0, g_anchorY = 0;     // window coordinates (top left of the tick)
    volatile DWORD g_lastSeen = 0;                  // GetTickCount of the last frame the box was visible
    HWND g_hWnd = nullptr;
    const UINT WM_RM_REFRESH = WM_APP + 41;

    // ---------------------------------------------------------------- game thread
    void* g_filledFor = nullptr;
    DWORD g_shownAt = 0;
    bool  g_wasVisible = false;

    void TickBody(void* self)
    {
        void* group = Find(self, "Group_LogIn");
        void* idE = Ptr(self, OFF_EDIT_ID);
        void* pwE = Ptr(self, OFF_EDIT_PW);
        bool visible = Visible(self) && Visible(group) && idE && pwE;
        if (!visible) { g_wasVisible = false; return; }
        // Account Register (register_panel.cpp): button under the box, ID / password on request, its send queue
        if (void* hpBtn = Find(group, "btn_homepage"))
        {
            // another group of the login screen (server list, notices, nation introductions) over the button
            static const char* kOver[] = { "Group_ServerList_01", "Group_Notice_1", "Group_Notice_2", "Group_Notice_3",
                                           "Group_introduction_el", "Group_introduction_ka" };
            const float* hr = RectOf(hpBtn);
            bool covered = false;
            for (const char* id : kOver)
            {
                void* g = Find(self, id);
                if (!Visible(g)) continue;
                const float* r = RectOf(g);
                if (r[0] < hr[2] && r[2] > hr[0] && r[1] < hr[3] && r[3] > hr[1]) { covered = true; break; }
            }
            Register_LoginTick(hr, covered, *(int*)((BYTE*)idE + OFF_EDIT_MAXLEN), *(int*)((BYTE*)pwE + OFF_EDIT_MAXLEN),
                *(int*)((BYTE*)hpBtn + OFF_UI_STATE));
        }
        if (Register_WantCredentials()) Register_SetCredentials(EditText(idE), EditText(pwE));
        Register_FlushSendQueue();
        {
            std::string nid, npw;
            if (Register_TakeFill(nid, npw))           // account just created: into the login box
            {
                SetEditText(idE, nid); SetEditText(pwE, npw);
                void* focused = *(void**)KO_FOCUSED_EDIT;
                if (focused == idE || focused == pwE) ((FnSetFocus)KO_EDIT_SETFOCUS)(focused);
                std::string backId = EditText(idE), backPw = EditText(pwE);
                bool okFill = backId == nid && backPw == npw;
                char lb[128]; sprintf_s(lb, "LOGIN: new account filled in (id %s, password %u chars, read back %s)",
                    nid.c_str(), (unsigned)npw.size(), okFill ? "ok" : "DIFFERENT");
                Log(lb);
                if (!okFill) SetEditText(pwE, std::string());   // never leave a wrong password: the player types it
                SecureZeroMemory(&npw[0], npw.size());
                if (!backPw.empty()) SecureZeroMemory(&backPw[0], backPw.size());
            }
        }

        DWORD now = GetTickCount();
        if (!g_wasVisible || g_filledFor != self) { g_wasVisible = true; g_filledFor = self; g_shownAt = now; }

        // where the tick goes: under btn_homepage, inside the box
        void* hp = Find(group, "btn_homepage");
        const float* r = RectOf(hp ? hp : pwE);
        const float* gr = RectOf(group);
        int x = (int)r[0], y = (int)r[3] + 8;
        if (!hp || y + TICK_H > (int)gr[3]) { x = (int)gr[0] + 16; y = (int)gr[3] + 4; }
        InterlockedExchange(&g_anchorX, x);
        InterlockedExchange(&g_anchorY, y);
        g_lastSeen = now;

        // Re-Connect: the previous process saved its login -> fill it in and press OK once
        static bool s_reconnectChecked = false;
        static DWORD s_submitAt = 0;
        if (!s_reconnectChecked)
        {
            s_reconnectChecked = true;
            std::string rid, rpw;
            if (Reconnect_TakeLogin(rid, rpw))
            {
                SetEditText(idE, rid); SetEditText(pwE, rpw);
                SecureZeroMemory(&rpw[0], rpw.size());
                void* focused = *(void**)KO_FOCUSED_EDIT;
                if (focused == idE || focused == pwE) ((FnSetFocus)KO_EDIT_SETFOCUS)(focused);
                s_submitAt = now + 800;
                Log("LOGIN: Re-Connect login filled in");
            }
        }
        if (s_submitAt && now >= s_submitAt)
        {
            s_submitAt = 0;
            if (void* ok = Ptr(self, OFF_BTN_OK)) { g_origRM(self, ok, MSG_BUTTON_CLICK); Log("LOGIN: Re-Connect login sent"); }
            return;
        }

        if (now - g_shownAt > FILL_WINDOW_MS) return;
        EnterCriticalSection(&g_lock);
        bool on = g_enabled;
        std::string id = g_id, pw = g_pw;
        LeaveCriticalSection(&g_lock);
        if (!on || id.empty() || !EditText(idE).empty() || !EditText(pwE).empty()) return;

        SetEditText(idE, id);
        SetEditText(pwE, pw);
        SecureZeroMemory(&pw[0], pw.size());
        // the focused edit mirrors a hidden Win32 edit: focus it again so that one gets the new text too
        void* focused = *(void**)KO_FOCUSED_EDIT;
        if (focused == idE || focused == pwE) ((FnSetFocus)KO_EDIT_SETFOCUS)(focused);
        Log("LOGIN: remembered ID / password filled in");
    }

    void __fastcall HkTick(void* self, void*)
    {
        g_origTick(self);
        __try { TickBody(self); }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
    }

    void CaptureLogin(void* self, void* sender, DWORD msg)
    {
        void* idE = Ptr(self, OFF_EDIT_ID);
        void* pwE = Ptr(self, OFF_EDIT_PW);
        bool login = (msg == MSG_BUTTON_CLICK && sender && sender == Ptr(self, OFF_BTN_OK))
                  || (msg == MSG_EDIT_RETURN && sender && (sender == idE || sender == pwE));
        if (!login || !idE || !pwE) return;
        std::string id = EditText(idE), pw = EditText(pwE);
        if (id.empty() || pw.empty()) return;
        EnterCriticalSection(&g_lock);
        g_lastId = id; g_lastPw = pw;
        bool on = g_enabled, changed = on && (id != g_id || pw != g_pw);
        if (changed) { g_id = id; g_pw = pw; }
        LeaveCriticalSection(&g_lock);
        SecureZeroMemory(&pw[0], pw.size());
        if (changed) { SaveSettings(); Log("LOGIN: ID / password remembered"); }
    }

    bool IsHomepage(void* self, void* sender)
    {
        return sender && sender == Find(Find(self, "Group_LogIn"), "btn_homepage");
    }

    bool __fastcall HkRM(void* self, void*, void* sender, DWORD msg)
    {
        __try { CaptureLogin(self, sender, msg); }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
        bool homepage = false;
        __try { homepage = msg == MSG_BUTTON_CLICK && IsHomepage(self, sender); }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
        if (homepage) { Register_Open(); return true; }   // the Homepage button is "Kayit Ol" now (register_panel.cpp)
        return g_origRM(self, sender, msg);
    }

    // ---------------------------------------------------------------- tick window
    HDC   g_memDC = nullptr;
    DWORD* g_bits = nullptr;
    bool  g_hover = false, g_track = false;

    // draws with GDI (white on black) into a scratch mask, then blends one color through it (premultiplied)
    template <class F> void Blend(COLORREF col, BYTE alpha, int dx, int dy, F draw)
    {
        static HDC mdc = nullptr; static DWORD* mbits = nullptr;
        if (!mdc)
        {
            mdc = CreateCompatibleDC(nullptr);
            BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), TICK_W, -TICK_H, 1, 32, BI_RGB };
            SelectObject(mdc, CreateDIBSection(mdc, &bmi, DIB_RGB_COLORS, (void**)&mbits, nullptr, 0));
        }
        memset(mbits, 0, TICK_W * TICK_H * 4);
        draw(mdc);
        const int R = GetRValue(col), G = GetGValue(col), B = GetBValue(col);
        for (int y = 0; y < TICK_H; y++)
            for (int x = 0; x < TICK_W; x++)
            {
                int sx = x - dx, sy = y - dy;
                if (sx < 0 || sy < 0 || sx >= TICK_W || sy >= TICK_H) continue;
                DWORD m = mbits[sy * TICK_W + sx];
                int a = max(max((int)(m & 0xFF), (int)((m >> 8) & 0xFF)), (int)((m >> 16) & 0xFF)) * alpha / 255;
                if (!a) continue;
                DWORD& d = g_bits[y * TICK_W + x];
                int da = d >> 24, db = d & 0xFF, dg = (d >> 8) & 0xFF, dr = (d >> 16) & 0xFF;
                int ia = 255 - a;
                d = ((a + da * ia / 255) << 24) | ((R * a / 255 + dr * ia / 255) << 16) |
                    ((G * a / 255 + dg * ia / 255) << 8) | (B * a / 255 + db * ia / 255);
            }
    }

    void Paint()
    {
        if (!g_hWnd || !g_bits) return;
        EnterCriticalSection(&g_lock);
        bool on = g_enabled;
        LeaveCriticalSection(&g_lock);

        for (int i = 0; i < TICK_W * TICK_H; i++) g_bits[i] = 0x01000000;   // alpha 1: whole area stays clickable
        static HFONT font = CreateFontW(-12, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0, ANTIALIASED_QUALITY, 0, L"Tahoma");
        const int bx = 1, by = 3, bs = 14;
        auto box = [&](HDC dc) { RECT rc = { bx, by, bx + bs, by + bs }; FillRect(dc, &rc, (HBRUSH)GetStockObject(WHITE_BRUSH)); };
        auto frame = [&](HDC dc) { RECT rc = { bx, by, bx + bs, by + bs }; FrameRect(dc, &rc, (HBRUSH)GetStockObject(WHITE_BRUSH)); };
        auto text = [&](HDC dc) {
            SelectObject(dc, font); SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
            RECT rc = { bx + bs + 6, 0, TICK_W, TICK_H };
            DrawTextW(dc, L"Remember Me", -1, &rc, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        };
        auto check = [&](HDC dc) {
            HPEN pen = CreatePen(PS_SOLID, 2, RGB(255, 255, 255));
            HGDIOBJ old = SelectObject(dc, pen);
            POINT p[3] = { { bx + 3, by + 7 }, { bx + 6, by + 10 }, { bx + 11, by + 3 } };
            Polyline(dc, p, 3);
            SelectObject(dc, old); DeleteObject(pen);
        };

        Blend(RGB(10, 8, 6), 190, 0, 0, box);                                        // box background
        Blend(g_hover ? RGB(255, 222, 150) : RGB(196, 160, 96), 255, 0, 0, frame);  // gold frame
        if (on) Blend(RGB(255, 214, 120), 255, 0, 0, check);
        Blend(RGB(0, 0, 0), 220, 1, 1, text);                                         // shadow
        Blend(g_hover ? RGB(255, 236, 190) : RGB(254, 245, 209), 255, 0, 0, text);    // label (login text colour)

        RECT wr; GetWindowRect(g_hWnd, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 };
        SIZE sz = { TICK_W, TICK_H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
    }

    HWND FindGameWindow()
    {
        struct Ctx { DWORD pid; HWND best; int area; } ctx = { GetCurrentProcessId(), nullptr, 0 };
        EnumWindows([](HWND h, LPARAM lp) -> BOOL {
            Ctx* c = (Ctx*)lp;
            DWORD pid; GetWindowThreadProcessId(h, &pid);
            if (pid != c->pid || !IsWindowVisible(h) || h == g_hWnd) return TRUE;
            RECT r; GetWindowRect(h, &r);
            int a = (r.right - r.left) * (r.bottom - r.top);
            if (a > c->area) { c->area = a; c->best = h; }
            return TRUE;
        }, (LPARAM)&ctx);
        return ctx.best;
    }

    void Sync()
    {
        HWND game = FindGameWindow();
        DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        bool want = game && !IsIconic(game) && pid == GetCurrentProcessId() && GetTickCount() - g_lastSeen < 400;
        if (!want)
        {
            if (IsWindowVisible(g_hWnd)) ShowWindow(g_hWnd, SW_HIDE);
            return;
        }
        POINT p = { g_anchorX, g_anchorY };
        ClientToScreen(game, &p);
        RECT wr; GetWindowRect(g_hWnd, &wr);
        bool moved = wr.left != p.x || wr.top != p.y;
        if (moved || !IsWindowVisible(g_hWnd))
        {
            SetWindowPos(g_hWnd, HWND_TOPMOST, p.x, p.y, TICK_W, TICK_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            Paint();
        }
    }

    void Toggle()
    {
        EnterCriticalSection(&g_lock);
        g_enabled = !g_enabled;
        if (!g_enabled) { if (!g_pw.empty()) SecureZeroMemory(&g_pw[0], g_pw.size()); g_id.clear(); g_pw.clear(); }
        bool on = g_enabled;
        LeaveCriticalSection(&g_lock);
        SaveSettings();       // off = forget the saved ID / password right away
        Log(on ? "LOGIN: Remember Me on" : "LOGIN: Remember Me off (saved login removed)");
        Paint();
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_MOUSEMOVE:
            if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
            if (!g_hover) { g_hover = true; Paint(); }
            return 0;
        case WM_MOUSELEAVE:
            g_track = false;
            if (g_hover) { g_hover = false; Paint(); }
            return 0;
        case WM_LBUTTONDOWN: SetCapture(h); return 0;
        case WM_LBUTTONUP:
        {
            ReleaseCapture();
            POINT p = { GET_X_LPARAM(lp), GET_Y_LPARAM(lp) };
            if (p.x >= 0 && p.y >= 0 && p.x < TICK_W && p.y < TICK_H) Toggle();
            return 0;
        }
        case WM_SETCURSOR:
            while (ShowCursor(TRUE) < 0) {}
            SetCursor(LoadCursor(nullptr, IDC_ARROW));
            return TRUE;
        case WM_TIMER: Sync(); return 0;
        case WM_RM_REFRESH: Paint(); return 0;
        case WM_CLOSE: return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        WNDCLASSEXW wc = { sizeof(wc) };
        wc.lpfnWndProc = WndProc;
        wc.hInstance = GetModuleHandleW(nullptr);
        wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
        wc.lpszClassName = L"NTT_LoginRemember";
        RegisterClassExW(&wc);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            wc.lpszClassName, L"Remember Me", WS_POPUP, 0, 0, TICK_W, TICK_H, nullptr, nullptr, wc.hInstance, nullptr);
        if (!g_hWnd) { Log("LOGIN: tick window creation failed"); return 0; }
        g_memDC = CreateCompatibleDC(nullptr);
        BITMAPINFO bmi = {};
        bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), TICK_W, -TICK_H, 1, 32, BI_RGB };
        SelectObject(g_memDC, CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, (void**)&g_bits, nullptr, 0));
        SetTimer(g_hWnd, 1, 100, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }

    // ---------------------------------------------------------------- install
    bool PatchSlot(int slot, DWORD expect, void* hook, void** orig)
    {
        DWORD* p = (DWORD*)(VT_LOGIN + slot * 4);
        if (*p != expect) return false;
        DWORD old;
        if (!VirtualProtect(p, 4, PAGE_READWRITE, &old)) return false;
        *orig = (void*)*p;
        *p = (DWORD)hook;
        VirtualProtect(p, 4, old, &old);
        return true;
    }

    DWORD WINAPI InstallThread(LPVOID)
    {
        // the exe may still be unpacking itself: wait for the original vtable entries
        for (int i = 0; i < 240 && *(DWORD*)(VT_LOGIN + SLOT_TICK * 4) != ORIG_TICK; i++) Sleep(250);
        bool t = PatchSlot(SLOT_TICK, ORIG_TICK, (void*)HkTick, (void**)&g_origTick);
        bool r = PatchSlot(SLOT_RM, ORIG_RM, (void*)HkRM, (void**)&g_origRM);
        char b[96]; sprintf_s(b, "LOGIN: Remember Me hooks tick=%d rm=%d (saved=%d)", t, r, g_enabled ? 1 : 0); Log(b);
        if (!t || !r) return 0;
        WindowThread(nullptr);
        return 0;
    }
}

bool LoginRemember_GetLast(std::string& id, std::string& pw)
{
    EnterCriticalSection(&g_lock);
    id = g_lastId; pw = g_lastPw;
    LeaveCriticalSection(&g_lock);
    return !id.empty() && !pw.empty();
}

void LoginRemember_Init()
{
    InitializeCriticalSection(&g_lock);
    LoadSettings();
    CloseHandle(CreateThread(nullptr, 0, InstallThread, nullptr, 0, nullptr));
}
