// Drop chest next to the target bar: while a monster is targeted a small chest (the game's Dragon Box item icon)
// sits right of the target bar. A click asks the server for that monster's drops (drop_panel.cpp Drop_RequestMini)
// and shows them in a compact list: the monster's name and its drops with their chances, in the game's green
// message box with the loot window's item cells (skin: build_dropmini_assets.py -> dropbtn_ui\*.pus).
// The full search panel ('\' / Ctrl+D, drop_panel.cpp) is unchanged.
// Target: [player+0x660] (-1 none); character by id 0x50DEB0 (thiscall [0x1115840], id, 0); monster when
// [ch+0xB9C] == 1 (same reads as genie_hg.cpp). Checked on the game thread (DropBtn_Tick, ProxyWndProc).
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <map>
#include "dropbtn_layout.h"
#include "drop_shared.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
HWND Proxy_GameWnd();

namespace
{
    // the target bar sits at the top centre (its right ornament ends at centre + 105); the chest touches it
    const int CHEST_X = 106, CHEST_Y = 31, LIST_Y = 96;
    const UINT WM_DM_SHOW = WM_APP + 91;
    volatile LONG g_monster = 0;                       // a monster is targeted (game thread)
    volatile LONG g_targetId = -1;                     // the client's target id

    struct Sprite { HDC dc = nullptr; int w = 0, h = 0; };
    bool LoadPusPath(const char* p, Sprite& s)
    {
        FILE* f = nullptr;
        if (fopen_s(&f, p, "rb") != 0 || !f) return false;
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
    std::string GameDir() { char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH); *(strrchr(p, '\\') + 1) = 0; return p; }
    bool LoadSkin(const char* name, Sprite& s)
    {
        std::string p = GameDir() + "HopeGuard\\dropbtn_ui\\" + name + ".pus";
        if (LoadPusPath(p.c_str(), s)) return true;
        char b[MAX_PATH + 32]; sprintf_s(b, "DROPBTN: missing %s", p.c_str()); Log(b);
        return false;
    }
    void Blit(HDC dst, const Sprite& s, int x, int y, int w = -1, int h = -1)
    {
        if (!s.dc) return;
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(dst, x, y, w < 0 ? s.w : w, h < 0 ? s.h : h, s.dc, 0, 0, s.w, s.h, bf);
    }

    // ---------------------------------------------------------------- chest button
    HWND g_hBtn = nullptr; HDC g_btnDC = nullptr; void* g_btnBits = nullptr;
    bool g_btnHover = false, g_btnTrack = false;
    Sprite g_chestN, g_chestO;

    void PaintBtn()
    {
        memset(g_btnBits, 0, (size_t)DB_CHEST * DB_CHEST * 4);
        Blit(g_btnDC, g_btnHover && g_chestO.dc ? g_chestO : g_chestN, 0, 0);
        POINT src = { 0, 0 }; SIZE sz = { DB_CHEST, DB_CHEST };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hBtn, nullptr, nullptr, &sz, g_btnDC, &src, 0, &bf, ULW_ALPHA);
    }

    // ---------------------------------------------------------------- compact list
    HWND g_hList = nullptr; HDC g_listDC = nullptr; void* g_listBits = nullptr;
    Sprite g_bg, g_closeN, g_closeO, g_closeD, g_noImage;
    std::map<UINT32, Sprite> g_icons;
    CRITICAL_SECTION g_lock;
    std::string g_title;
    std::vector<DropMiniEntry> g_drops;
    int g_top = 0, g_hoverCell = -1, g_closeState = 0;  // closeState 0 normal, 1 over, 2 down
    bool g_dragging = false, g_listTrack = false;
    POINT g_dragFrom = {};

    const Sprite& Icon(UINT32 icon)
    {
        auto it = g_icons.find(icon);
        if (it != g_icons.end()) return it->second;
        Sprite s;
        char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\pus_ui\\icons\\%u.pus", GameDir().c_str(), icon);
        LoadPusPath(p, s);
        return g_icons[icon] = s;
    }

    std::wstring Wide(const std::string& s)
    {
        if (s.empty()) return std::wstring();
        int n = MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), nullptr, 0);
        std::wstring w(n, L'\0'); MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), &w[0], n);
        return w;
    }

    // white text alone, then coloured with a dark shadow into a premultiplied bitmap of tw x th
    void TextTo(void* target, int tw, int th, const std::string& s, int l, int t, int r, int b, COLORREF color, int pt, bool bold, UINT fmt)
    {
        int w = r - l, h = b - t;
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
                int dy = t + y + o; if (dy < 0 || dy >= th) continue;
                const BYTE* src = (const BYTE*)bits + y * w * 4;
                for (int x = 0; x < w; x++, src += 4)
                {
                    int dx = l + x + o; int a = max(src[0], max(src[1], src[2]));
                    if (!a || dx < 0 || dx >= tw) continue;
                    if (!pass) a = a * 3 / 4;
                    BYTE* d = (BYTE*)target + (dy * tw + dx) * 4;
                    d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                    d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
                }
            }
        }
        SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }

    std::string Pct(UINT16 pct)
    {
        char b[16];
        if (pct >= 10000) return "100%";
        if (pct % 100 == 0) sprintf_s(b, "%u%%", pct / 100); else sprintf_s(b, "%.2f%%", pct / 100.0);
        return b;
    }

    int Rows(size_t n) { return (int)((n + DM_COLS - 1) / DM_COLS); }

    // translucent colour over a cell of the list bitmap (premultiplied)
    void Tint(int l, int t, int r, int b, BYTE cr, BYTE cg, BYTE cb, BYTE a)
    {
        for (int y = max(0, t); y < min(DM_H, b); y++)
        {
            BYTE* d = (BYTE*)g_listBits + (y * DM_W + max(0, l)) * 4;
            for (int x = max(0, l); x < min(DM_W, r); x++, d += 4)
            {
                d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
            }
        }
    }

    void PaintList()
    {
        if (!g_hList || !g_listDC) return;
        memset(g_listBits, 0, (size_t)DM_W * DM_H * 4);
        Blit(g_listDC, g_bg, 0, 0);
        Blit(g_listDC, g_closeState == 2 ? g_closeD : g_closeState == 1 ? g_closeO : g_closeN, DM_CLOSE[0], DM_CLOSE[1], DM_CLOSE[2] - DM_CLOSE[0], DM_CLOSE[3] - DM_CLOSE[1]);
        EnterCriticalSection(&g_lock);
        std::string title = g_title; std::vector<DropMiniEntry> drops = g_drops; int top = g_top, hover = g_hoverCell;
        LeaveCriticalSection(&g_lock);
        for (int vis = 0; vis < DM_COLS * DM_ROWS; vis++)
        {
            int i = top * DM_COLS + vis;
            if (i >= (int)drops.size()) break;
            int x = DM_GX + (vis % DM_COLS) * DM_PITCH, y = DM_GY + (vis / DM_COLS) * DM_PITCH;
            const Sprite& ic = Icon(drops[i].icon);
            if (ic.dc) Blit(g_listDC, ic, x + 5, y + 5, 45, 45);
            else if (g_noImage.dc) Blit(g_listDC, g_noImage, x + 5, y + 5, 45, 45);
        }
        GdiFlush();
        for (int vis = 0; vis < DM_COLS * DM_ROWS; vis++)   // drop block list (drop_panel.cpp): red over the blocked ones
        {
            int i = top * DM_COLS + vis;
            if (i >= (int)drops.size()) break;
            int x = DM_GX + (vis % DM_COLS) * DM_PITCH, y = DM_GY + (vis / DM_COLS) * DM_PITCH;
            if (DropBlock_Has(drops[i].item)) Tint(x + 5, y + 5, x + 50, y + 50, 150, 0, 0, 150);
        }
        TextTo(g_listBits, DM_W, DM_H, title, DM_TITLE[0], DM_TITLE[1], DM_TITLE[2], DM_TITLE[3], RGB(255, 170, 60), 10, true, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        for (int vis = 0; vis < DM_COLS * DM_ROWS; vis++)
        {
            int i = top * DM_COLS + vis;
            if (i >= (int)drops.size()) break;
            int x = DM_GX + (vis % DM_COLS) * DM_PITCH, y = DM_GY + (vis / DM_COLS) * DM_PITCH;
            bool blocked = DropBlock_Has(drops[i].item);
            TextTo(g_listBits, DM_W, DM_H, blocked ? std::string("BLOCK") : Pct(drops[i].pct), x + 2, y + 36, x + 53, y + 52, blocked ? RGB(255, 110, 110) : RGB(255, 225, 120), 7, true, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        }
        std::string info;
        if (hover >= 0 && hover < (int)drops.size())
            info = drops[hover].name + "  -  " + (DropBlock_Has(drops[hover].item) ? std::string("BLOCKED, right click to allow") : Pct(drops[hover].pct) + "  (right click to block)");
        else if (drops.empty()) info = "This monster drops nothing.";
        else
        {
            char b[96]; sprintf_s(b, "%u drop%s", (unsigned)drops.size(), drops.size() == 1 ? "" : "s");
            info = b;
            if (Rows(drops.size()) > DM_ROWS) info += "  -  mouse wheel to scroll";
        }
        TextTo(g_listBits, DM_W, DM_H, info, DM_INFO[0], DM_INFO[1], DM_INFO[2], DM_INFO[3], hover >= 0 ? RGB(255, 255, 255) : RGB(190, 190, 170), 8, false, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        RECT wr; GetWindowRect(g_hList, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { DM_W, DM_H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hList, nullptr, &dst, &sz, g_listDC, &src, 0, &bf, ULW_ALPHA);
    }

    int CellAt(int x, int y)
    {
        if (x < DM_GX || y < DM_GY) return -1;
        int c = (x - DM_GX) / DM_PITCH, r = (y - DM_GY) / DM_PITCH;
        if (c >= DM_COLS || r >= DM_ROWS) return -1;
        EnterCriticalSection(&g_lock);
        int i = (g_top + r) * DM_COLS + c, n = (int)g_drops.size();
        LeaveCriticalSection(&g_lock);
        return i < n ? i : -1;
    }
    bool InClose(int x, int y) { return x >= DM_CLOSE[0] && x < DM_CLOSE[2] && y >= DM_CLOSE[1] && y < DM_CLOSE[3]; }

    void Scroll(int d)
    {
        EnterCriticalSection(&g_lock);
        int maxTop = max(0, Rows(g_drops.size()) - DM_ROWS);
        g_top = min(maxTop, max(0, g_top + d));
        LeaveCriticalSection(&g_lock);
        PaintList();
    }

    LRESULT CALLBACK ListProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_MOUSEMOVE:
        {
            if (g_dragging)
            {
                POINT p; GetCursorPos(&p); RECT wr; GetWindowRect(h, &wr);
                SetWindowPos(h, nullptr, wr.left + p.x - g_dragFrom.x, wr.top + p.y - g_dragFrom.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                g_dragFrom = p; return 0;
            }
            if (!g_listTrack) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_listTrack = TrackMouseEvent(&t) != FALSE; }
            int cell = CellAt(x, y), cs = InClose(x, y) ? (g_closeState == 2 ? 2 : 1) : 0;
            if (cell != g_hoverCell)
            {
                UINT32 item = 0;
                EnterCriticalSection(&g_lock); if (cell >= 0 && cell < (int)g_drops.size()) item = g_drops[cell].item; int top = g_top; LeaveCriticalSection(&g_lock);
                if (item)
                {
                    int vis = cell - top * DM_COLS; RECT wr; GetWindowRect(h, &wr);
                    int cx = wr.left + DM_GX + (vis % DM_COLS) * DM_PITCH, cy = wr.top + DM_GY + (vis / DM_COLS) * DM_PITCH;
                    ItemTip_Show(item, { wr.left, cy, wr.right, cy + 55 });   // the game draws it beside the list
                }
                else ItemTip_Hide();
            }
            if (cell != g_hoverCell || cs != g_closeState) { g_hoverCell = cell; g_closeState = cs; PaintList(); }
            return 0;
        }
        case WM_MOUSELEAVE: g_listTrack = false; g_hoverCell = -1; g_closeState = 0; ItemTip_Hide(); PaintList(); return 0;
        case WM_LBUTTONDOWN:
            SetCapture(h);
            if (InClose(x, y)) { g_closeState = 2; PaintList(); return 0; }
            if (y < DM_GY) { g_dragging = true; GetCursorPos(&g_dragFrom); }   // drag by the title
            return 0;
        case WM_LBUTTONUP:
            ReleaseCapture();
            if (g_dragging) { g_dragging = false; return 0; }
            if (g_closeState == 2 && InClose(x, y)) { ShowWindow(h, SW_HIDE); ItemTip_Hide(); g_closeState = 0; return 0; }
            g_closeState = InClose(x, y) ? 1 : 0; PaintList();
            return 0;
        case WM_RBUTTONUP:
        {
            // on a drop: block / allow it (at most 10, drop_panel.cpp); anywhere else: close the list
            int cell = CellAt(x, y); DropMiniEntry e = {};
            EnterCriticalSection(&g_lock); if (cell >= 0 && cell < (int)g_drops.size()) e = g_drops[cell]; LeaveCriticalSection(&g_lock);
            if (!e.item) { ShowWindow(h, SW_HIDE); ItemTip_Hide(); return 0; }
            if (DropBlock_Toggle(e) < 0) Log("DROPBTN: block list full (10)");
            ItemTip_Hide(); PaintList();
            return 0;
        }
        case WM_MOUSEWHEEL: Scroll(GET_WHEEL_DELTA_WPARAM(wp) > 0 ? -1 : 1); return 0;
        case WM_SETCURSOR: SetCursor(LoadCursor(nullptr, IDC_ARROW)); return TRUE;
        case WM_DM_SHOW:
        {
            if (!IsWindowVisible(h))
            {
                RECT gr = { 0, 0, 1024, 768 };
                if (HWND game = Proxy_GameWnd()) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
                SetWindowPos(h, HWND_TOPMOST, gr.left + ((gr.right - gr.left) - DM_W) / 2, gr.top + LIST_Y, DM_W, DM_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            }
            else SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            g_hoverCell = -1;
            PaintList();
            return 0;
        }
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    // follows the game: chest only while a monster is targeted, both hidden when the game is not in front
    void Sync()
    {
        HWND game = Proxy_GameWnd(); DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        bool front = game && !IsIconic(game) && pid == GetCurrentProcessId();
        if (!front && g_hList && IsWindowVisible(g_hList)) ShowWindow(g_hList, SW_HIDE);
        if (!(front && g_monster)) { if (IsWindowVisible(g_hBtn)) ShowWindow(g_hBtn, SW_HIDE); return; }
        RECT gr; GetClientRect(game, &gr); MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
        int x = gr.left + (gr.right - gr.left) / 2 + CHEST_X, y = gr.top + CHEST_Y;
        RECT wr; GetWindowRect(g_hBtn, &wr);
        if (!IsWindowVisible(g_hBtn) || wr.left != x || wr.top != y)
        {
            SetWindowPos(g_hBtn, HWND_TOPMOST, x, y, DB_CHEST, DB_CHEST, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            PaintBtn();
        }
    }

    LRESULT CALLBACK BtnProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_MOUSEMOVE:
            if (!g_btnTrack) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_btnTrack = TrackMouseEvent(&t) != FALSE; }
            if (!g_btnHover) { g_btnHover = true; PaintBtn(); }
            return 0;
        case WM_MOUSELEAVE: g_btnTrack = false; g_btnHover = false; PaintBtn(); return 0;
        case WM_LBUTTONUP:
            Drop_RequestMini((UINT32)g_targetId);
            Log("DROPBTN: drop list of the target");
            return 0;
        case WM_SETCURSOR: SetCursor(LoadCursor(nullptr, IDC_ARROW)); return TRUE;
        case WM_TIMER: Sync(); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        HINSTANCE hi = GetModuleHandleW(nullptr);
        WNDCLASSEXW bc = { sizeof(bc) }; bc.lpfnWndProc = BtnProc; bc.hInstance = hi; bc.lpszClassName = L"NTT_DropChest";
        RegisterClassExW(&bc);
        WNDCLASSEXW lc = { sizeof(lc) }; lc.lpfnWndProc = ListProc; lc.hInstance = hi; lc.lpszClassName = L"NTT_DropMini";
        RegisterClassExW(&lc);
        g_hBtn = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, bc.lpszClassName, L"Drop",
            WS_POPUP, 0, 0, DB_CHEST, DB_CHEST, nullptr, nullptr, hi, nullptr);
        g_hList = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, lc.lpszClassName, L"Drops",
            WS_POPUP, 0, 0, DM_W, DM_H, nullptr, nullptr, hi, nullptr);
        if (!g_hBtn || !g_hList) return 0;
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), DB_CHEST, -DB_CHEST, 1, 32, BI_RGB };
        g_btnDC = CreateCompatibleDC(nullptr); SelectObject(g_btnDC, CreateDIBSection(g_btnDC, &bmi, DIB_RGB_COLORS, &g_btnBits, nullptr, 0));
        BITMAPINFO lbi = {}; lbi.bmiHeader = { sizeof(BITMAPINFOHEADER), DM_W, -DM_H, 1, 32, BI_RGB };
        g_listDC = CreateCompatibleDC(nullptr); SelectObject(g_listDC, CreateDIBSection(g_listDC, &lbi, DIB_RGB_COLORS, &g_listBits, nullptr, 0));
        LoadSkin("chest_n", g_chestN); LoadSkin("chest_o", g_chestO);
        LoadSkin("mini_bg", g_bg); LoadSkin("close_n", g_closeN); LoadSkin("close_o", g_closeO); LoadSkin("close_d", g_closeD);
        LoadPusPath((GameDir() + "HopeGuard\\pus_ui\\noimage.pus").c_str(), g_noImage);
        SetTimer(g_hBtn, 1, 150, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }

    typedef BYTE* (__thiscall* FnCharById)(void*, int, int);
    bool TargetIsMonster(int& id, int& flag)
    {
        BYTE* me = *(BYTE**)0x01115834;
        void* mgr = *(void**)0x01115840;
        id = -1; flag = -1;
        if (!me || !mgr) return false;
        id = *(int*)(me + 0x660);
        if (id < 0) return false;
        BYTE* ch = ((FnCharById)0x0050DEB0)(mgr, id, 0);
        if (!ch) return false;
        flag = *(ch + 0xB9C);                            // one byte: the three above it are not part of the flag
        return flag == 1;
    }
}

void DropBtn_Init()
{
    InitializeCriticalSection(&g_lock);
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") != 0) return;
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// drop_panel.cpp (game thread): the server's answer to Drop_RequestMini
void DropMini_Show(const std::string& name, unsigned level, const std::vector<DropMiniEntry>& drops)
{
    char t[160]; sprintf_s(t, "%s  (Lv%u)", name.c_str(), level);
    EnterCriticalSection(&g_lock);
    g_title = t; g_drops = drops; g_top = 0;
    LeaveCriticalSection(&g_lock);
    if (g_hList) PostMessageW(g_hList, WM_DM_SHOW, 0, 0);
}

// ProxyWndProc (game thread): is a monster targeted?
void DropBtn_Tick()
{
    static DWORD last = 0;
    DWORD now = GetTickCount();
    if (now - last < 150) return;
    last = now;
    bool mon = false; int id = -1, flag = -1;
    __try { mon = TargetIsMonster(id, flag); }
    __except (EXCEPTION_EXECUTE_HANDLER) { mon = false; }
    InterlockedExchange(&g_monster, mon ? 1 : 0);
    InterlockedExchange(&g_targetId, id);
    static int lastId = -2, lastFlag = -2;
    if (id != lastId || flag != lastFlag)                // diagnostics: why the chest shows / not
    {
        lastId = id; lastFlag = flag;
        char b[96]; sprintf_s(b, "DROPBTN: target %d flag %d -> chest %s", id, flag, mon ? "on" : "off"); Log(b);
    }
}

// Shared with the full drop viewer so its "Target" button requests the active
// monster runtime ID instead of sending 0 (which the server cannot resolve).
UINT32 DropBtn_CurrentTargetId()
{
    LONG id = InterlockedCompareExchange(&g_targetId, 0, 0);
    return id < 0 ? 0u : (UINT32)id;
}

// pus_store.cpp click gate: clicks on the chest / the list must not reach the game
bool DropBtn_CursorOverPanel()
{
    POINT p;
    if (!GetCursorPos(&p)) return false;
    HWND w = WindowFromPoint(p);
    return w && ((w == g_hBtn && IsWindowVisible(g_hBtn)) || (w == g_hList && IsWindowVisible(g_hList)));
}

// ProxyWndProc: the wheel goes to the focused game window; forward it while the cursor is over the list
bool DropMini_ForwardWheel(WPARAM wp)
{
    POINT p;
    if (!g_hList || !IsWindowVisible(g_hList) || !GetCursorPos(&p) || WindowFromPoint(p) != g_hList) return false;
    PostMessageW(g_hList, WM_MOUSEWHEEL, wp, MAKELPARAM(p.x, p.y));
    return true;
}
