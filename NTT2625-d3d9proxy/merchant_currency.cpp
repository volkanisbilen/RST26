// Merchant price currency (Coins / Knight Cash / TL) for the 2625 client, loaded through the d3d9 proxy.
// The native "Enter the item price" dialog (CUITradePrice, re_tradeprice.uif) only knows coins and the client's
// MERCHANT_ITEM_ADD packet has no currency byte at all (14 bytes: 68 03 [u32 item][u16 count][u32 price][u8 src]
// [u8 dst]); the server reads two more bytes after it, [u8 mode][u8 currency 0 coins, 1 Knight Cash, 2 TL].
//   - a small bar with the three choices sticks under the price dialog while it is open (GDI layered overlay, same
//     technique as the other panels); every time the dialog opens it is back on Coins
//   - the function that sends the packet (0xE12B20, thiscall(CUITradeInventory*, item*)) is replaced by ours, which
//     sends the same 14 bytes + [0][currency]
// Server: GameServer MerchantHandler.cpp MerchantItemAdd / MerchantItemUserBuy (_MERCH_DATA::bCurrency).
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>

void Log(const char* msg);

namespace
{
    const DWORD KO_SND_FNC = 0x00704070;       // thiscall(CAPISocket*, BYTE* buf, int len)
    const DWORD KO_PTR_PKT = 0x01115914;       // CAPISocket*
    const DWORD KO_GAMEMAIN = 0x011158FC;      // CGameProcMain*
    const DWORD FN_ITEM_ADD = 0x00E12B20;      // CUITradeInventory: send MERCHANT_ITEM_ADD
    const BYTE  kItemAddHead[10] = { 0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x24, 0xA1, 0x80, 0xC6, 0x0F };
    const DWORD VT_TRADE_INV = 0x0103E850, VT_TRADE_PRICE = 0x0103EC88;
    const DWORD OFF_MAIN_TRADE_INV = 0x298;    // [CGameProcMain + 664] (built in 0x85CEA0)
    const DWORD OFF_INV_SENT = 900;            // "add request on its way" flag the native function sets
    const DWORD OFF_UI_RECT = 0xC8, OFF_UI_VISIBLE = 0xEA;

    enum { CUR_COINS = 0, CUR_KC = 1, CUR_TL = 2, CUR_COUNT = 3 };
    const char* kCurName[CUR_COUNT] = { "Coins", "Knight Cash", "TL" };
    volatile LONG g_currency = CUR_COINS;

    bool Readable(const void* p, SIZE_T n)
    {
        MEMORY_BASIC_INFORMATION mbi;
        if ((DWORD_PTR)p < 0x10000 || !VirtualQuery(p, &mbi, sizeof(mbi))) return false;
        if (mbi.State != MEM_COMMIT || (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD))) return false;
        return (const BYTE*)p + n <= (const BYTE*)mbi.BaseAddress + mbi.RegionSize;
    }

    // ---------------------------------------------------------------- the send (game thread)
    void __fastcall hkItemAdd(BYTE* self, void* /*edx*/, const BYTE* item)
    {
        if (!item || self[OFF_INV_SENT]) return;
        BYTE pkt[16];
        pkt[0] = 0x68; pkt[1] = 0x03;                       // WIZ_MERCHANT, MERCHANT_ITEM_ADD
        memcpy(pkt + 2, item, 4);                           // item id
        memcpy(pkt + 6, item + 4, 2);                       // count
        memcpy(pkt + 8, item + 0x10, 4);                    // price
        pkt[12] = item[8];                                  // inventory slot
        pkt[13] = item[0xC];                                // merchant slot
        pkt[14] = 0;                                        // mode
        pkt[15] = (BYTE)g_currency;
        void* sock = *(void**)KO_PTR_PKT;
        if (!sock) return;
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, pkt, sizeof(pkt));
        self[OFF_INV_SENT] = 1;
        char b[112]; sprintf_s(b, "MERCH: item add item=%u count=%u price=%u currency=%s", *(UINT32*)(pkt + 2), *(UINT16*)(pkt + 6),
                               *(UINT32*)(pkt + 8), kCurName[pkt[15] < CUR_COUNT ? pkt[15] : 0]);
        Log(b);
    }

    bool InstallHook()
    {
        BYTE* target = (BYTE*)FN_ITEM_ADD;
        if (!Readable(target, sizeof(kItemAddHead)) || memcmp(target, kItemAddHead, sizeof(kItemAddHead)) != 0) return false;
        DWORD old;
        if (!VirtualProtect(target, 5, PAGE_EXECUTE_READWRITE, &old)) return false;
        target[0] = 0xE9;                                   // whole function replaced: nothing of it runs again
        *(DWORD*)(target + 1) = (DWORD)hkItemAdd - (DWORD)(target + 5);
        VirtualProtect(target, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), target, 5);
        return true;
    }

    // ---------------------------------------------------------------- the price dialog (read only, any thread)
    BYTE* g_dlg = nullptr; DWORD g_lastSearch = 0;

    BYTE* FindByVtable(BYTE* base, DWORD span, DWORD vt)
    {
        if (!Readable(base, span)) return nullptr;
        for (DWORD o = 0; o + 4 <= span; o += 4)
        {
            BYTE* p = *(BYTE**)(base + o);
            if (Readable(p, 0xF0) && *(DWORD*)p == vt) return p;
        }
        return nullptr;
    }

    // true while the price dialog of the selling merchant window is on screen; rc = its rectangle (game client area)
    bool PriceDialog(RECT* rc)
    {
        bool open = false;
        __try
        {
            if (!g_dlg || !Readable(g_dlg, 0xF0) || *(DWORD*)g_dlg != VT_TRADE_PRICE)
            {
                g_dlg = nullptr;
                const DWORD now = GetTickCount();
                if (g_lastSearch && now - g_lastSearch < 2000) return false;
                g_lastSearch = now;
                BYTE* main = *(BYTE**)KO_GAMEMAIN;
                if (!Readable(main, 0x1000)) return false;
                BYTE* inv = *(BYTE**)(main + OFF_MAIN_TRADE_INV);
                if (!Readable(inv, 0x390) || *(DWORD*)inv != VT_TRADE_INV) inv = FindByVtable(main, 0x1000, VT_TRADE_INV);
                if (!inv) return false;
                g_dlg = FindByVtable(inv, 0x390, VT_TRADE_PRICE);
                static bool logged = false;
                if (g_dlg && !logged) { logged = true; char b[96]; sprintf_s(b, "MERCH: price dialog found %p (trade inventory %p)", g_dlg, inv); Log(b); }
                if (!g_dlg) return false;
            }
            if (*(g_dlg + OFF_UI_VISIBLE) != 0)
            {
                const float* r = (const float*)(g_dlg + OFF_UI_RECT);
                SetRect(rc, (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
                open = rc->right - rc->left >= 150 && rc->right - rc->left <= 500 && rc->bottom > rc->top;
            }
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { g_dlg = nullptr; open = false; }
        return open;
    }

    // ---------------------------------------------------------------- the bar
    const int W = 266, H = 30, PAD = 4;
    HWND g_hWnd = nullptr; HDC g_dc = nullptr; void* g_bits = nullptr;
    int g_hover = -1; bool g_track = false, g_wasOpen = false;
    HCURSOR g_gameCursor = nullptr;

    RECT Btn(int i)
    {
        const int w = (W - 2 * PAD - 2 * 3) / 3;
        RECT r = { PAD + i * (w + 3), PAD, PAD + i * (w + 3) + w, H - PAD };
        return r;
    }

    void Fill(RECT r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
    {
        for (int y = max(0, (int)r.top); y < min(H, (int)r.bottom); y++)
        {
            BYTE* d = (BYTE*)g_bits + (y * W + max(0, (int)r.left)) * 4;
            for (int x = max(0, (int)r.left); x < min(W, (int)r.right); x++, d += 4)
            {
                d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
                d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
                d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
            }
        }
    }

    void Frame(RECT r, BYTE cr, BYTE cg, BYTE cb)
    {
        Fill({ r.left, r.top, r.right, r.top + 1 }, cr, cg, cb, 255); Fill({ r.left, r.bottom - 1, r.right, r.bottom }, cr, cg, cb, 255);
        Fill({ r.left, r.top, r.left + 1, r.bottom }, cr, cg, cb, 255); Fill({ r.right - 1, r.top, r.right, r.bottom }, cr, cg, cb, 255);
    }

    void Text(const char* s, RECT r, COLORREF color, bool bold)
    {
        const int w = r.right - r.left, h = r.bottom - r.top;
        BITMAPINFO bmi = {};
        bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
        void* bits = nullptr;
        HDC dc = CreateCompatibleDC(nullptr);
        HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        HGDIOBJ ob = SelectObject(dc, bmp);
        HFONT font = CreateFontA(-MulDiv(8, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
        HGDIOBJ of = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
        RECT rc = { 0, 0, w, h };
        DrawTextA(dc, s, -1, &rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        GdiFlush();
        for (int y = 0; y < h; y++)
        {
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            for (int x = 0; x < w; x++, src += 4)
            {
                const int a = max(src[0], max(src[1], src[2]));
                const int dx = r.left + x, dy = r.top + y;
                if (!a || dx < 0 || dx >= W || dy < 0 || dy >= H) continue;
                BYTE* d = (BYTE*)g_bits + (dy * W + dx) * 4;
                d[0] = (BYTE)((GetBValue(color) * a + d[0] * (255 - a)) / 255);
                d[1] = (BYTE)((GetGValue(color) * a + d[1] * (255 - a)) / 255);
                d[2] = (BYTE)((GetRValue(color) * a + d[2] * (255 - a)) / 255);
                d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
            }
        }
        SelectObject(dc, of); DeleteObject(font);
        SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
    }

    void Paint()
    {
        if (!g_hWnd || !g_dc) return;
        memset(g_bits, 0, (size_t)W * H * 4);
        Fill({ 0, 0, W, H }, 16, 22, 17, 235);              // the message box green
        Frame({ 0, 0, W, H }, 120, 110, 70);
        const int cur = (int)g_currency;
        for (int i = 0; i < CUR_COUNT; i++)
        {
            const RECT b = Btn(i);
            const bool sel = i == cur, hov = i == g_hover;
            if (sel)      Fill(b, 150, 100, 30, 255);
            else if (hov) Fill(b, 70, 78, 60, 255);
            else          Fill(b, 38, 46, 38, 255);
            Frame(b, sel ? 240 : 96, sel ? 192 : 100, sel ? 96 : 80);
            Text(kCurName[i], b, sel ? RGB(255, 255, 255) : RGB(200, 200, 190), sel);
        }
        RECT wr; GetWindowRect(g_hWnd, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 };
        SIZE sz = { W, H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_dc, &src, 0, &bf, ULW_ALPHA);
    }

    HWND FindGameWindow()
    {
        struct Ctx { DWORD pid; HWND best; int area; } ctx = { GetCurrentProcessId(), nullptr, 0 };
        EnumWindows([](HWND h, LPARAM lp) -> BOOL {
            Ctx* c = (Ctx*)lp;
            DWORD pid; GetWindowThreadProcessId(h, &pid);
            if (pid != c->pid || !IsWindowVisible(h) || h == g_hWnd) return TRUE;
            wchar_t cls[32] = L""; GetClassNameW(h, cls, 32);
            if (wcsncmp(cls, L"NTT_", 4) == 0) return TRUE;         // our own overlay windows
            RECT r; GetWindowRect(h, &r);
            int a = (r.right - r.left) * (r.bottom - r.top);
            if (a > c->area) { c->area = a; c->best = h; }
            return TRUE;
        }, (LPARAM)&ctx);
        return ctx.best;
    }

    void Sync()
    {
        RECT rc = {};
        HWND game = FindGameWindow();
        DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        const bool open = PriceDialog(&rc);
        const bool want = open && game && !IsIconic(game) && pid == GetCurrentProcessId();
        if (open && !g_wasOpen) InterlockedExchange(&g_currency, CUR_COINS);    // every price starts as coins
        g_wasOpen = open;
        if (want)
        {
            POINT p = { rc.left + ((rc.right - rc.left) - W) / 2, rc.bottom + 2 };
            ClientToScreen(game, &p);
            RECT wr; GetWindowRect(g_hWnd, &wr);
            const bool vis = IsWindowVisible(g_hWnd) != FALSE;
            if (!vis || wr.left != p.x || wr.top != p.y)
            {
                SetWindowPos(g_hWnd, HWND_TOPMOST, p.x, p.y, W, H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
                Paint();
            }
        }
        else if (IsWindowVisible(g_hWnd)) ShowWindow(g_hWnd, SW_HIDE);

        POINT c; CURSORINFO ci = { sizeof(ci) };
        if (game && GetCursorPos(&c) && WindowFromPoint(c) == game && GetCursorInfo(&ci) && (ci.flags & CURSOR_SHOWING) && ci.hCursor)
            g_gameCursor = ci.hCursor;
    }

    int HitTest(int x, int y)
    {
        for (int i = 0; i < CUR_COUNT; i++) { const RECT b = Btn(i); if (x >= b.left && x < b.right && y >= b.top && y < b.bottom) return i; }
        return -1;
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_MOUSEMOVE:
            if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
            if (int hit = HitTest(GET_X_LPARAM(lp), GET_Y_LPARAM(lp)); hit != g_hover) { g_hover = hit; Paint(); }
            return 0;
        case WM_MOUSELEAVE:
            g_track = false;
            if (g_hover != -1) { g_hover = -1; Paint(); }
            return 0;
        case WM_LBUTTONDOWN:
            if (int hit = HitTest(GET_X_LPARAM(lp), GET_Y_LPARAM(lp)); hit >= 0) { InterlockedExchange(&g_currency, hit); Paint(); }
            return 0;
        case WM_SETCURSOR:
            while (ShowCursor(TRUE) < 0) {}
            SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
            return TRUE;
        case WM_TIMER: Sync(); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        // the exe unpacks itself first: wait for the function, then replace it
        bool hooked = false;
        for (int i = 0; i < 240 && !(hooked = InstallHook()); i++) Sleep(500);
        Log(hooked ? "MERCH: item add send replaced @00E12B20 (currency byte)" : "MERCH: item add function not found, currency choice OFF");
        if (!hooked) return 0;

        WNDCLASSEXW wc = { sizeof(wc) };
        wc.lpfnWndProc = WndProc;
        wc.hInstance = GetModuleHandleW(nullptr);
        wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
        wc.lpszClassName = L"NTT_MerchCurrency";
        RegisterClassExW(&wc);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            wc.lpszClassName, L"Merchant currency", WS_POPUP, 0, 0, W, H, nullptr, nullptr, wc.hInstance, nullptr);
        if (!g_hWnd) { Log("MERCH: window creation failed"); return 0; }
        g_dc = CreateCompatibleDC(nullptr);
        BITMAPINFO bmi = {};
        bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), W, -H, 1, 32, BI_RGB };
        SelectObject(g_dc, CreateDIBSection(g_dc, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
        SetTimer(g_hWnd, 1, 100, nullptr);

        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }
}

void MerchCur_Init()
{
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// pus_store.cpp's GetAsyncKeyState hook: clicks on our bar must not move the character
bool MerchCur_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
