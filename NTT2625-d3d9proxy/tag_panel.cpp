// Tag Scroll (item 800099000 "Scroll of Tag ID"), client side. Server: GameServer\TagChange.cpp, Pontus lua EVENT 203.
// WIZ_HSACS_HOOK 0xE9 + TagInfo 0xD1, strings u16 length prefixed (CP1254):
//   S->C 0 open the panel
//   S->C 1 list [u16 n] n*[u32 id][str tag][u8 r][u8 g][u8 b]       (players coming into view, our own respawn)
//   S->C 3 done [u8 0][u32 id][str tag][r][g][b]                   (to us and to the region)
//   S->C 4 no scroll, 5 already this tag, 6 error
//   C->S 2 [str tag][r][g][b]                                      (max. 20 characters)
// Panel: skin build_tag_assets.py (tag_ui\*.pus, the game's message box, clan edit field and chat colour palette).
// Showing the tag: the character's info text (0x904D60, font char+0x104, text char+0x2E8) is set on the game thread
// (Tag_Tick, ~500 ms). The game uses that text itself (green): a tag only goes into an empty field and the game's
// own text is never overwritten. Ids come from the server's socket ids; WIZ_USER_INOUT out (07 02) drops the entry.
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <map>
#include <vector>
#include <deque>
#include "tag_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();
HWND Proxy_GameWnd();

namespace
{
    const BYTE  TAG_SUBOP = 0xD1;
    const size_t TAG_MAX = 20;                         // server MAX_ID_SIZE
    const DWORD KO_SND_FNC = 0x00704070, KO_PTR_PKT = 0x01115914;   // CAPISocket::Send, CAPISocket*
    const DWORD KO_PTR_ME = 0x01115834;                // our player; socket id at +0x6A0
    const DWORD FN_GET_CHAR = 0x00798DC0;              // __stdcall(int id, int alive) -> character
    const DWORD FN_SET_INFO = 0x00904D60;              // __thiscall(chr, const std::string&, DWORD argb), ret 8
    const DWORD CHR_FONT = 0x104, CHR_TEXT = 0x2E8, FONT_COLOR = 0x3C;
    const UINT_PTR TICK_TIMER = 0x4803;
    const DWORD TICK_MS = 500;

    struct Rc { int l, t, r, b; };
    Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }
    bool In(const int a[4], int x, int y) { return x >= a[0] && x < a[2] && y >= a[1] && y < a[3]; }
    DWORD Argb(BYTE r, BYTE g, BYTE b) { return 0xFF000000 | (r << 16) | (g << 8) | b; }

    // ---------------------------------------------------------------- tags (game thread)
    struct Tag { std::string text; DWORD argb = 0; std::string shown; };   // shown: what we put on the character
    std::map<DWORD, Tag> g_tags;
    CRITICAL_SECTION g_lock;                           // g_tags, g_status, g_sendQ, panel inputs
    bool g_timerSet = false;

    // MSVC (2015+) x86 std::string as the client lays it out: 16-byte SSO buffer / pointer, size, capacity
    struct MsvcString { union { char buf[16]; char* ptr; }; size_t size, cap; };

    std::string ReadMsvc(const BYTE* s)
    {
        size_t n = *(const size_t*)(s + 0x10), cap = *(const size_t*)(s + 0x14);
        if (n > 256) return std::string();
        const char* p = cap > 15 ? *(const char* const*)s : (const char*)s;
        return std::string(p, n);
    }

    BYTE* GetChar(DWORD id)
    {
        typedef BYTE* (__stdcall* Fn)(int, int);
        return ((Fn)FN_GET_CHAR)((int)id, 0);          // 0: dead players too
    }

    void SetInfo(BYTE* chr, const std::string& text, DWORD argb)
    {
        MsvcString s = {};
        std::vector<char> heap;
        s.size = text.size();
        if (text.size() < 16) { memcpy(s.buf, text.data(), text.size()); s.buf[text.size()] = 0; s.cap = 15; }
        else { heap.assign(text.begin(), text.end()); heap.push_back(0); s.ptr = heap.data(); s.cap = text.size(); }
        typedef void(__thiscall* Fn)(void*, const MsvcString*, DWORD);
        ((Fn)FN_SET_INFO)(chr, &s, argb);              // only reads the string (copies it into chr+0x2E8)
    }

    DWORD MyId()
    {
        BYTE* me = *(BYTE**)KO_PTR_ME;
        return me ? *(DWORD*)(me + 0x6A0) : 0xFFFFFFFF;
    }

    void TickOne(DWORD id, Tag& t)
    {
        BYTE* chr = GetChar(id);
        if (!chr) return;
        std::string cur = ReadMsvc(chr + CHR_TEXT);
        BYTE* font = *(BYTE**)(chr + CHR_FONT);
        DWORD color = font ? *(DWORD*)(font + FONT_COLOR) : 0;
        if (cur.empty())
        {
            if (!t.text.empty()) { SetInfo(chr, t.text, t.argb); t.shown = t.text; }
            else t.shown.clear();
            return;
        }
        bool ours = (!t.shown.empty() && cur == t.shown) || cur == t.text;
        if (!ours) return;                             // the game's own text: leave it alone
        if (t.text.empty()) { SetInfo(chr, std::string(), 0); t.shown.clear(); }
        else if (cur != t.text || color != t.argb) { SetInfo(chr, t.text, t.argb); t.shown = t.text; }   // changed / re-set green by the game
    }

    void SetTag(DWORD id, std::string text, BYTE r, BYTE g, BYTE b)
    {
        if (text == "-") text.clear();
        EnterCriticalSection(&g_lock);
        Tag& t = g_tags[id];
        t.text = text; t.argb = Argb(r, g, b);
        LeaveCriticalSection(&g_lock);
    }

    // ---------------------------------------------------------------- panel (own window thread)
    HWND  g_hWnd = nullptr;
    HDC   g_memDC = nullptr;
    void* g_bits = nullptr;
    enum { C_NONE, C_OK, C_CANCEL, C_EDIT, C_PALETTE };
    int   g_hover = C_NONE, g_pressed = C_NONE;
    bool  g_open = false, g_focus = false, g_track = false, g_dragging = false, g_picking = false;
    POINT g_dragFrom = {};
    HCURSOR g_gameCursor = nullptr;
    std::string g_input;                               // CP1254, as the server stores it
    BYTE  g_r = 255, g_g = 255, g_b = 255;
    POINT g_pick = { -1, -1 };                         // marker on the palette
    std::string g_status; COLORREF g_statusColor = RGB(255, 255, 255);
    DWORD g_waitSince = 0;                             // request sent, waiting for the answer
    DWORD g_closeAt = 0;
    std::deque<std::vector<BYTE>> g_sendQ;
    const UINT WM_TG_SHOW = WM_APP + 61, WM_TG_REFRESH = WM_APP + 62;
    const DWORD ANSWER_WAIT_MS = 6000;

    struct Sprite { HDC dc = nullptr; int w = 0, h = 0; const BYTE* bits = nullptr; };
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
            SelectObject(s.dc, bmp); s.w = (int)w; s.h = (int)h; s.bits = (const BYTE*)bits;
        }
        fclose(f);
        return ok;
    }
    void LoadSkin()
    {
        if (!g_skin.empty()) return;
        char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH); *(strrchr(dir, '\\') + 1) = 0;
        for (const char* n : { "bg", "palette", "btn_n", "btn_o", "btn_d" })
        {
            char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\tag_ui\\%s.pus", dir, n);
            Sprite s; if (LoadPus(p, s)) g_skin[n] = s;
            else { char b[MAX_PATH + 32]; sprintf_s(b, "TAG: missing %s", p); Log(b); }
        }
    }
    void Skin(const char* n, int x, int y)
    {
        auto it = g_skin.find(n); if (it == g_skin.end()) return;
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(g_memDC, x, y, it->second.w, it->second.h, it->second.dc, 0, 0, it->second.w, it->second.h, bf);
    }
    std::wstring Wide(const std::string& s)
    {
        if (s.empty()) return std::wstring();
        int n = MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), nullptr, 0);
        std::wstring w(n, L'\0'); MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), &w[0], n);
        return w;
    }
    // white text rendered on its own, coloured with a 1 px dark shadow into the panel bitmap (as reconnect.cpp)
    void Text(const std::string& s, Rc r, COLORREF color, int pt, UINT fmt, bool bold = false)
    {
        int w = r.r - r.l, h = r.b - r.t;
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
        for (int pass = 0; pass < 2; pass++)
        {
            int o = pass ? 0 : 1;
            BYTE cr = pass ? GetRValue(color) : 0, cg = pass ? GetGValue(color) : 0, cb = pass ? GetBValue(color) : 0;
            for (int y = 0; y < h; y++)
            {
                int dy = r.t + y + o; if (dy < 0 || dy >= TG_H) continue;
                const BYTE* src = (const BYTE*)bits + y * w * 4;
                for (int x = 0; x < w; x++, src += 4)
                {
                    int dx = r.l + x + o; int a = max(src[0], max(src[1], src[2]));
                    if (!a || dx < 0 || dx >= TG_W) continue;
                    if (!pass) a = a * 3 / 4;
                    BYTE* d = (BYTE*)g_bits + (dy * TG_W + dx) * 4;
                    d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                    d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
                }
            }
        }
        SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }
    int TextWidth(const std::string& s, int pt, bool bold)
    {
        std::wstring ws = Wide(s);
        HDC dc = CreateCompatibleDC(nullptr);
        HFONT font = CreateFontW(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, L"Verdana");
        HGDIOBJ of = SelectObject(dc, font);
        SIZE sz = {}; GetTextExtentPoint32W(dc, ws.c_str(), (int)ws.size(), &sz);
        SelectObject(dc, of); DeleteObject(font); DeleteDC(dc);
        return sz.cx;
    }
    void Frame(int x0, int y0, int x1, int y1, BYTE r, BYTE g, BYTE b)   // 1 px opaque rectangle
    {
        auto px = [&](int x, int y) {
            if (x < 0 || y < 0 || x >= TG_W || y >= TG_H) return;
            BYTE* d = (BYTE*)g_bits + (y * TG_W + x) * 4; d[0] = b; d[1] = g; d[2] = r; d[3] = 255;
        };
        for (int x = x0; x <= x1; x++) { px(x, y0); px(x, y1); }
        for (int y = y0; y <= y1; y++) { px(x0, y); px(x1, y); }
    }

    void Paint()
    {
        if (!g_hWnd || !g_memDC) return;
        memset(g_bits, 0, (size_t)TG_W * TG_H * 4);
        LoadSkin();
        Skin("bg", 0, 0);
        Skin("palette", TG_PALETTE[0], TG_PALETTE[1]);
        EnterCriticalSection(&g_lock);
        std::string input = g_input, status = g_status;
        COLORREF statusColor = g_statusColor;
        BYTE r = g_r, g = g_g, b = g_b;
        LeaveCriticalSection(&g_lock);
        Text("Tag Change", R4(TG_TITLE), RGB(255, 170, 60), 10, DT_CENTER | DT_VCENTER | DT_SINGLELINE, true);
        Text("Enter your tag (max. 20 characters) and pick its colour.", R4(TG_TEXT), RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        Rc e = R4(TG_EDIT);
        bool caret = g_focus && (GetTickCount() / 500) % 2;
        if (input.empty() && !g_focus)
            Text("click here and type your tag", { e.l + 8, e.t, e.r - 6, e.b }, RGB(130, 130, 130), 9, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        else
            Text(input + (caret ? "|" : ""), { e.l + 8, e.t, e.r - 6, e.b }, RGB(r, g, b), 10, DT_LEFT | DT_VCENTER | DT_SINGLELINE, true);
        if (g_pick.x >= 0)
        {
            int x = TG_PALETTE[0] + g_pick.x, y = TG_PALETTE[1] + g_pick.y;
            Frame(x - 4, y - 4, x + 4, y + 4, 0, 0, 0);
            Frame(x - 3, y - 3, x + 3, y + 3, 255, 255, 255);
        }
        // preview: the tag in its colour, as it will appear over the character
        Rc p = R4(TG_PREVIEW);
        const std::string label = "Preview:";
        int lw = TextWidth(label, 9, false) + 8;
        Text(label, { p.l, p.t, p.l + lw, p.b }, RGB(200, 190, 160), 9, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        Text(input.empty() ? std::string("Tag") : input, { p.l + lw, p.t, p.r, p.b }, RGB(r, g, b), 10, DT_LEFT | DT_VCENTER | DT_SINGLELINE, true);
        Text(status, R4(TG_STATUS), statusColor, 9, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        const int* br[2] = { TG_BTN_OK, TG_BTN_CANCEL };
        const char* lbl[2] = { "Confirm", "Cancel" };
        for (int i = 0; i < 2; i++)
        {
            int id = i == 0 ? C_OK : C_CANCEL;
            bool waiting = id == C_OK && g_waitSince;
            Skin(g_hover == id && !waiting ? (g_pressed == id ? "btn_d" : "btn_o") : "btn_n", br[i][0], br[i][1]);
            Text(lbl[i], R4(br[i]), waiting ? RGB(140, 130, 110) : RGB(255, 240, 210), 9, DT_CENTER | DT_VCENTER | DT_SINGLELINE, true);
        }
        RECT wr; GetWindowRect(g_hWnd, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { TG_W, TG_H };
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
    void SetFocusEdit(bool on)
    {
        if (g_focus == on) return;
        g_focus = on;
        if (on) SetForegroundWindow(g_hWnd);           // keyboard to us while typing
        else if (HWND game = FindGameWindow()) SetForegroundWindow(game);
    }
    void Hide()
    {
        SetFocusEdit(false);
        g_open = false; g_closeAt = 0; g_waitSince = 0;
        ShowWindow(g_hWnd, SW_HIDE);
        if (HWND game = FindGameWindow()) SetForegroundWindow(game);
    }
    void SetStatus(const char* s, COLORREF c)
    {
        EnterCriticalSection(&g_lock); g_status = s; g_statusColor = c; LeaveCriticalSection(&g_lock);
    }
    void Show()
    {
        g_open = true; g_closeAt = 0; g_waitSince = 0;
        SetStatus("", RGB(255, 255, 255));
        RECT gr = { 0, 0, 1024, 768 };
        if (HWND game = FindGameWindow()) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
        SetWindowPos(g_hWnd, HWND_TOPMOST, gr.left + ((gr.right - gr.left) - TG_W) / 2, gr.top + ((gr.bottom - gr.top) - TG_H) / 2,
                     TG_W, TG_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        SetFocusEdit(true);
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
        if (g_closeAt && GetTickCount() >= g_closeAt) { Hide(); return; }
        if (g_focus && IsWindowVisible(g_hWnd)) Paint();   // caret blink
    }

    void Pick(int x, int y)
    {
        auto it = g_skin.find("palette");
        if (it == g_skin.end() || !it->second.bits) return;
        x = min(max(x - TG_PALETTE[0], 0), it->second.w - 1); y = min(max(y - TG_PALETTE[1], 0), it->second.h - 1);
        const BYTE* px = it->second.bits + ((size_t)y * it->second.w + x) * 4;
        if (px[3] < 200) return;                       // transparent corner of the art
        EnterCriticalSection(&g_lock);
        g_b = (BYTE)(px[0] * 255 / px[3]); g_g = (BYTE)(px[1] * 255 / px[3]); g_r = (BYTE)(px[2] * 255 / px[3]);
        LeaveCriticalSection(&g_lock);
        g_pick = { x, y };
        Paint();
    }

    void Confirm()
    {
        if (g_waitSince) return;
        EnterCriticalSection(&g_lock);
        std::string tag = g_input;
        BYTE r = g_r, g = g_g, b = g_b;
        LeaveCriticalSection(&g_lock);
        while (!tag.empty() && tag.back() == ' ') tag.pop_back();
        while (!tag.empty() && tag.front() == ' ') tag.erase(tag.begin());
        if (tag.empty()) { SetStatus("Please enter a tag.", RGB(255, 90, 70)); Paint(); return; }
        std::vector<BYTE> out = { 0xE9, TAG_SUBOP, 2 };
        WORD n = (WORD)tag.size();
        out.insert(out.end(), (BYTE*)&n, (BYTE*)&n + 2);
        out.insert(out.end(), tag.begin(), tag.end());
        out.push_back(r); out.push_back(g); out.push_back(b);
        EnterCriticalSection(&g_lock);
        g_sendQ.push_back(out);
        LeaveCriticalSection(&g_lock);
        Proxy_RequestFlush();
        g_waitSince = GetTickCount();
        SetFocusEdit(false);
        SetStatus("Changing your tag...", RGB(255, 255, 255));
        char lb[96]; sprintf_s(lb, "TAG: request \"%s\" %02X%02X%02X", tag.c_str(), r, g, b); Log(lb);
        Paint();
    }

    int HitTest(int x, int y)
    {
        if (In(TG_BTN_OK, x, y)) return C_OK;
        if (In(TG_BTN_CANCEL, x, y)) return C_CANCEL;
        if (In(TG_EDIT, x, y)) return C_EDIT;
        if (In(TG_PALETTE, x, y)) return C_PALETTE;
        return C_NONE;
    }

    void OnChar(WPARAM wp)
    {
        if (!g_focus) return;
        if (wp == VK_RETURN) { Confirm(); return; }
        if (wp == VK_ESCAPE) { Hide(); return; }
        EnterCriticalSection(&g_lock);
        if (wp == VK_BACK) { if (!g_input.empty()) g_input.pop_back(); }
        else if (wp >= 32 && g_input.size() < TAG_MAX)
        {
            wchar_t wc = (wchar_t)wp; char c = 0; BOOL lossy = FALSE;
            if (WideCharToMultiByte(1254, WC_NO_BEST_FIT_CHARS, &wc, 1, &c, 1, nullptr, &lossy) == 1 && !lossy && (BYTE)c >= 32)
                g_input.push_back(c);
        }
        LeaveCriticalSection(&g_lock);
        Paint();
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
            if (g_picking) { Pick(x, y); return 0; }
            if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
            if (int hit = HitTest(x, y); hit != g_hover) { g_hover = hit; Paint(); }
            return 0;
        case WM_MOUSELEAVE: g_track = false; if (g_hover != C_NONE) { g_hover = C_NONE; Paint(); } return 0;
        case WM_LBUTTONDOWN:
        {
            int hit = HitTest(x, y);
            SetCapture(h);
            if (hit == C_NONE) { g_dragging = true; GetCursorPos(&g_dragFrom); return 0; }
            if (hit == C_PALETTE) { g_picking = true; Pick(x, y); return 0; }
            if (hit == C_EDIT) { SetFocusEdit(true); Paint(); return 0; }
            g_pressed = hit; Paint();
            return 0;
        }
        case WM_LBUTTONUP:
        {
            ReleaseCapture();
            if (g_dragging) { g_dragging = false; return 0; }
            if (g_picking) { g_picking = false; return 0; }
            int hit = HitTest(x, y), p = g_pressed; g_pressed = C_NONE;
            if (p != C_NONE && p == hit)
            {
                if (hit == C_OK) Confirm();
                else if (hit == C_CANCEL) { Hide(); return 0; }
            }
            Paint();
            return 0;
        }
        case WM_CHAR: OnChar(wp); return 0;
        case WM_KILLFOCUS: if (g_focus) { g_focus = false; Paint(); } return 0;
        case WM_SETCURSOR:
            while (ShowCursor(TRUE) < 0) {}
            SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
            return TRUE;
        case WM_TIMER: SyncVisibility(); return 0;
        case WM_TG_SHOW: Show(); return 0;
        case WM_TG_REFRESH:
            if (wp == 1) g_closeAt = GetTickCount() + 1500;   // done: close a moment later
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
        wc.hCursor = LoadCursor(nullptr, IDC_ARROW); wc.lpszClassName = L"NTT_TagPanel";
        RegisterClassExW(&wc);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, wc.lpszClassName,
            L"Tag Change", WS_POPUP, 0, 0, TG_W, TG_H, nullptr, nullptr, wc.hInstance, nullptr);
        if (!g_hWnd) { Log("TAG: window creation failed"); return 0; }
        g_memDC = CreateCompatibleDC(nullptr);
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), TG_W, -TG_H, 1, 32, BI_RGB };
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
        if (g_hWnd) PostMessageW(g_hWnd, WM_TG_REFRESH, done ? 1 : 0, 0);
    }
}

void Tag_Init()
{
    InitializeCriticalSection(&g_lock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// game thread: open the panel with our current tag filled in. Called for the server's "open" (S->C 0) and for a
// right click on the Scroll of Tag ID (rce_store.cpp); the server checks the scroll when the new tag is confirmed.
void Tag_OpenPanel()
{
    DWORD me = MyId();
    EnterCriticalSection(&g_lock);
    auto it = g_tags.find(me);
    if (it != g_tags.end() && !it->second.text.empty())
    {
        g_input = it->second.text;
        g_r = (BYTE)(it->second.argb >> 16); g_g = (BYTE)(it->second.argb >> 8); g_b = (BYTE)it->second.argb;
    }
    LeaveCriticalSection(&g_lock);
    g_pick = { -1, -1 };
    if (g_hWnd) PostMessageW(g_hWnd, WM_TG_SHOW, 0, 0);
}

// recv hook (game thread): E9 D1 ... and WIZ_USER_INOUT out (07 02 00 [u32 id])
void Tag_OnRecv(const BYTE* p, size_t len)
{
    if (!g_timerSet)
        if (HWND w = Proxy_GameWnd()) g_timerSet = SetTimer(w, TICK_TIMER, TICK_MS, nullptr) != 0;   // keeps Tag_Tick going
    if (len >= 7 && p[0] == 0x07 && p[1] == 2)
    {
        DWORD id = *(const DWORD*)(p + 3);
        EnterCriticalSection(&g_lock); g_tags.erase(id); LeaveCriticalSection(&g_lock);
        return;
    }
    if (len < 3 || p[0] != 0xE9 || p[1] != TAG_SUBOP) return;
    size_t at = 3;
    auto u8 = [&](BYTE& v) { if (at + 1 > len) return false; v = p[at++]; return true; };
    auto u16 = [&](WORD& v) { if (at + 2 > len) return false; v = *(const WORD*)(p + at); at += 2; return true; };
    auto u32 = [&](DWORD& v) { if (at + 4 > len) return false; v = *(const DWORD*)(p + at); at += 4; return true; };
    auto str = [&](std::string& v) {
        WORD n; if (!u16(n) || n > 64 || at + n > len) return false;
        v.assign((const char*)p + at, n); at += n; return true;
    };
    switch (p[2])
    {
    case 0:
        Tag_OpenPanel();
        Log("TAG: panel opened by the server");
        break;
    case 1:
    {
        WORD n = 0;
        if (!u16(n)) return;
        for (WORD i = 0; i < n; i++)
        {
            DWORD id; std::string tag; BYTE r, g, b;
            if (!u32(id) || !str(tag) || !u8(r) || !u8(g) || !u8(b)) return;
            SetTag(id, tag, r, g, b);
        }
        break;
    }
    case 3:
    {
        BYTE zero, r, g, b; DWORD id; std::string tag;
        if (!u8(zero) || !u32(id) || !str(tag) || !u8(r) || !u8(g) || !u8(b)) return;
        SetTag(id, tag, r, g, b);
        if (id == MyId())
        {
            Answer("Your tag has been changed.", RGB(120, 255, 120), true);
            char lb[96]; sprintf_s(lb, "TAG: changed to \"%s\"", tag.c_str()); Log(lb);
        }
        break;
    }
    case 4: Answer("You need a Scroll of Tag ID.", RGB(255, 90, 70), false); break;
    case 5: Answer("You already have this tag and colour.", RGB(255, 200, 80), false); break;
    case 6: Answer("The tag could not be changed.", RGB(255, 90, 70), false); break;
    }
}

void Tag_FlushSendQueue()
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

// ProxyWndProc (game thread): puts the known tags on the characters in view
void Tag_Tick()
{
    static DWORD last = 0;
    DWORD now = GetTickCount();
    if (now - last < TICK_MS) return;
    last = now;
    EnterCriticalSection(&g_lock);
    for (auto& kv : g_tags)
    {
        __try { TickOne(kv.first, kv.second); }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
    }
    LeaveCriticalSection(&g_lock);
}

// ProxyWndProc: true when the WM_TIMER was ours (the tick itself runs for every message)
bool Tag_OnTimer(UINT_PTR id) { return id == TICK_TIMER; }

// pus_store.cpp click gate: clicks on the tag panel must not reach the game (DirectInput)
bool Tag_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
