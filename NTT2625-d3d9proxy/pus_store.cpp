// Power Up Store for the XIGNCODE 2625 client, loaded through the d3d9 proxy.
// Port of HSACSX PusStore.cpp (GDI layered overlay window) onto the verified
// SND/RECV addresses of this exe. Server side: GameServer XGuard.cpp
// HSACSX_PusRequest / HSACSX_SendPUS (WIZ_HSACS_HOOK 0xE9).
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <map>
#include <deque>
#include <algorithm>

void Log(const char* msg);
void Proxy_RequestFlush();
// exchange panel (rce_store.cpp): taskbar routing + click-through cooperation
bool Rce_OnTaskbarClick(const char* name);
bool Rce_CursorOverPanel();
bool Cr_CursorOverPanel();   // cr_panel.cpp
bool Sp_CursorOverPanel();   // sp_panel.cpp
bool Cind_CursorOverPanel(); // cind_panel.cpp
bool Csw_CursorOverPanel();  // csw_panel.cpp
bool MerchCur_CursorOverPanel();  // merchant_currency.cpp
bool Lottery_CursorOverPanel(); // lottery_panel.cpp
bool MannerStore_CursorOverPanel(); // manner_store.cpp
bool DropBox_CursorOverPanel(); // dropbox_panel.cpp
bool EvReward_CursorOverPanel(); // event_reward_panel.cpp
bool Drop_CursorOverPanel(); // drop_panel.cpp
bool Cross_CursorOverPanel(); // cross_exch.cpp
bool Reconnect_CursorOverPanel(); // reconnect.cpp
bool SoloCapeIcon_CursorOverPanel(); // solocape_icon.cpp
bool Tag_CursorOverPanel(); // tag_panel.cpp
bool JobChange_CursorOverPanel(); // jobchange_panel.cpp
bool DropBtn_CursorOverPanel(); // drop_button.cpp
bool CamRange_CursorOverPanel(); // camera_range.cpp
bool GmPanel_CursorOverPanel(); // gm_panel.cpp
bool Register_CursorOverPanel(); // register_panel.cpp

// --- Verified addresses (from in-game memory dump, 2026-09-27) ----------------
static const DWORD KO_SND_FNC = 0x00704070; // unpack: thiscall(CAPISocket*, BYTE* buf, int len)
static const DWORD KO_PTR_PKT = 0x01115914; // unpack: CAPISocket* (game socket)

static const BYTE WIZ_HSACS_HOOK = 0xE9;
static const BYTE PUS_SUBOP = 0xA8;            // process 0 = list, 1 = buy
static const BYTE PUS_SUBOP_CASHCHANGE = 0xA9;
static const BYTE PUS_SUBOP_CATEGORY = 0xD6;

struct SPusItem
{
    UINT32 ID = 0, ItemID = 0, Price = 0, BuyCount = 0, IconID = 0;
    std::string Name;
    BYTE Cat = 0, PriceType = 0;
};

struct SPusCategory
{
    UINT32 ID = 0;
    std::string Name;
    BYTE Status = 0;
};

// --- Outgoing queue: flushed on the game thread (inside the recv hook) --------
static CRITICAL_SECTION g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQueue;

static void QueueSend(const std::vector<BYTE>& pkt)
{
    EnterCriticalSection(&g_sendLock);
    g_sendQueue.push_back(pkt);
    LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();   // flush on the game thread now, not on the next received packet
}

// reset_bar.cpp shows the balances on the character status page
void Pus_GetBalances(UINT32* kc, UINT32* tl);

void Pus_FlushSendQueue()
{
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_sendLock);
    pending.swap(g_sendQueue);
    LeaveCriticalSection(&g_sendLock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
        char buf[96];
        sprintf_s(buf, "PUS send: %02X %02X %02X len=%u", p[0], p.size() > 1 ? p[1] : 0, p.size() > 2 ? p[2] : 0, (unsigned)p.size());
        Log(buf);
    }
}

// --- Packet reader --------------------------------------------------------------
struct Reader
{
    const BYTE* p; size_t len, pos;
    bool ok = true;
    template <class T> T get()
    {
        T v{};
        if (pos + sizeof(T) > len) { ok = false; return v; }
        memcpy(&v, p + pos, sizeof(T)); pos += sizeof(T);
        return v;
    }
    std::string str()
    {
        UINT16 n = get<UINT16>();
        if (!ok || pos + n > len) { ok = false; return {}; }
        std::string s((const char*)p + pos, n); pos += n;
        return s;
    }
};

// --- Window: skinned from re_power_up_store.uif (sprites in <game>\pus_ui, layout in pus_layout.h) ---
#include "pus_layout.h"
#pragma comment(lib, "msimg32.lib")

static const int ITEMS_PER_PAGE = 12, TAB_COUNT = 6, MAX_QTY = 99;
static const UINT WM_PUS_REFRESH = WM_APP + 1;
static const UINT WM_PUS_TOGGLE = WM_APP + 2;

static CRITICAL_SECTION g_dataLock;
static std::vector<SPusItem> g_items, g_filtered;
static std::vector<SPusCategory> g_cats;
static UINT32 g_kc = 0, g_tl = 0;
void Pus_GetBalances(UINT32* kc, UINT32* tl) { *kc = g_kc; *tl = g_tl; }
static int g_activeCat = 0, g_page = 0, g_selected = -1;
static bool g_listRequested = false;
static std::string g_status, g_search;
static bool g_searchFocus = false;
static int g_confirmIdx = -1, g_qty = 1;          // confirm dialog: index into g_filtered

static HWND g_hWnd = nullptr;
static HDC g_memDC = nullptr;
static void* g_bits = nullptr;

// ---------------------------------------------------------------- sprites
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<std::string, Sprite> g_sprites;
static std::map<UINT32, Sprite> g_icons;
static char g_uiDir[MAX_PATH];

static bool LoadPus(const char* path, Sprite& s)
{
    FILE* f = nullptr;
    if (fopen_s(&f, path, "rb") != 0 || !f) return false;
    char magic[4]; UINT32 w = 0, h = 0;
    bool ok = fread(magic, 1, 4, f) == 4 && memcmp(magic, "PUSI", 4) == 0 &&
              fread(&w, 4, 1, f) == 1 && fread(&h, 4, 1, f) == 1 && w && h && w < 4096 && h < 4096;
    if (ok)
    {
        BITMAPINFO bmi = {};
        bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), (LONG)w, -(LONG)h, 1, 32, BI_RGB };
        void* bits = nullptr;
        s.dc = CreateCompatibleDC(nullptr);
        s.bmp = CreateDIBSection(s.dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        ok = s.bmp && fread(bits, 4, (size_t)w * h, f) == (size_t)w * h;
        SelectObject(s.dc, s.bmp);
        s.w = (int)w; s.h = (int)h;
    }
    fclose(f);
    return ok;
}

static Sprite* Spr(const char* name)
{
    auto it = g_sprites.find(name);
    if (it != g_sprites.end()) return it->second.dc ? &it->second : nullptr;
    Sprite s;
    char path[MAX_PATH];
    sprintf_s(path, "%s\\%s.pus", g_uiDir, name);
    if (!LoadPus(path, s))
    {
        char msg[MAX_PATH + 32]; sprintf_s(msg, "PUS ui: missing sprite %s", path); Log(msg);
        s = Sprite();
    }
    g_sprites[name] = s;
    return s.dc ? &g_sprites[name] : nullptr;
}

static Sprite* Icon(UINT32 iconId)
{
    auto it = g_icons.find(iconId);
    if (it != g_icons.end()) return it->second.dc ? &it->second : Spr("noimage");
    Sprite s;
    char path[MAX_PATH];
    sprintf_s(path, "%s\\icons\\%u.pus", g_uiDir, iconId);
    if (!iconId || !LoadPus(path, s)) s = Sprite();
    g_icons[iconId] = s;
    return s.dc ? &g_icons[iconId] : Spr("noimage");
}

static void Blit(Sprite* s, int x, int y, int w = -1, int h = -1)
{
    if (!s) return;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_memDC, x, y, w < 0 ? s->w : w, h < 0 ? s->h : h, s->dc, 0, 0, s->w, s->h, bf);
}

// GDI text ignores the alpha channel of a 32bpp DIB (it would punch holes in the
// layered window), so text is rendered white-on-black into a scratch DIB and its
// coverage is used as alpha when blending the colour into the frame buffer.
static void Text(const std::string& s, PusRect r, COLORREF color, int pt, UINT fmt, bool bold = false)
{
    int w = r.r - r.l, h = r.b - r.t;
    if (s.empty() || w <= 0 || h <= 0) return;
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
    void* bits = nullptr;
    HDC dc = CreateCompatibleDC(nullptr);
    HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    HGDIOBJ ob = SelectObject(dc, bmp);
    HFONT font = CreateFontA(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
    HGDIOBJ of = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, RGB(255, 255, 255));
    RECT rc = { 0, 0, w, h };
    DrawTextA(dc, s.c_str(), (int)s.size(), &rc, fmt | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    GdiFlush();
    const BYTE cr = GetRValue(color), cg = GetGValue(color), cb = GetBValue(color);
    for (int y = 0; y < h; y++)
    {
        int dy = r.t + y;
        if (dy < 0 || dy >= PUS_H) continue;
        const BYTE* src = (const BYTE*)bits + y * w * 4;
        BYTE* dst = (BYTE*)g_bits + (dy * PUS_W + r.l) * 4;
        for (int x = 0; x < w; x++, src += 4, dst += 4)
        {
            int a = max(src[0], max(src[1], src[2]));
            if (!a || r.l + x < 0 || r.l + x >= PUS_W) continue;
            dst[0] = (BYTE)((cb * a + dst[0] * (255 - a)) / 255);
            dst[1] = (BYTE)((cg * a + dst[1] * (255 - a)) / 255);
            dst[2] = (BYTE)((cr * a + dst[2] * (255 - a)) / 255);
            dst[3] = (BYTE)(a + dst[3] * (255 - a) / 255);
        }
    }
    SelectObject(dc, of); DeleteObject(font);
    SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}

// ---------------------------------------------------------------- controls / hit testing
enum Ctl { C_NONE, C_CLOSE, C_PREV, C_NEXT, C_SEARCH, C_TAB0, C_BUY0 = C_TAB0 + TAB_COUNT,
           C_DLG_UP = C_BUY0 + ITEMS_PER_PAGE, C_DLG_DOWN, C_DLG_OK, C_DLG_CANCEL, C_DLG_BODY };
static int g_hover = C_NONE, g_pressed = C_NONE;
static int g_tipSlot = -1;                       // product card under the mouse (item tooltip), -1 = none
void ItemTip_Show(UINT32 item, RECT anchor);     // item_tooltip.cpp: the game's own inventory tooltip
void ItemTip_Hide();
HWND Proxy_GameWnd();
static bool g_tracking = false, g_dragging = false;
static POINT g_dragFrom;

static bool In(PusRect r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

static int HitTest(int x, int y)
{
    if (g_confirmIdx >= 0)
    {
        if (In(PUS_DLG_UP, x, y)) return C_DLG_UP;
        if (In(PUS_DLG_DOWN, x, y)) return C_DLG_DOWN;
        if (In(PUS_DLG_OK, x, y)) return C_DLG_OK;
        if (In(PUS_DLG_CANCEL, x, y)) return C_DLG_CANCEL;
        return In(PUS_DLG, x, y) ? C_DLG_BODY : C_NONE;
    }
    if (In(PUS_CLOSE, x, y)) return C_CLOSE;
    if (In(PUS_PREV, x, y)) return C_PREV;
    if (In(PUS_NEXT, x, y)) return C_NEXT;
    if (In(PUS_TXT_SEARCH, x, y)) return C_SEARCH;
    for (int i = 0; i < TAB_COUNT; i++)
        if (In(PUS_TABS[i], x, y)) return C_TAB0 + i;
    for (int i = 0; i < ITEMS_PER_PAGE; i++)
        if (g_page * ITEMS_PER_PAGE + i < (int)g_filtered.size() && In(PUS_SLOTS[i].buy, x, y)) return C_BUY0 + i;
    return C_NONE;
}

static int PageCount() { return max(1, (int)((g_filtered.size() + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE)); }

static std::string Lower(std::string s) { for (auto& c : s) c = (char)tolower((unsigned char)c); return s; }

static void RebuildFiltered()
{
    g_filtered.clear();
    std::string needle = Lower(g_search);
    for (auto& it : g_items)
    {
        if (g_activeCat != 0 && (int)it.Cat != g_activeCat) continue;
        if (!needle.empty() && Lower(it.Name).find(needle) == std::string::npos) continue;
        g_filtered.push_back(it);
    }
    std::sort(g_filtered.begin(), g_filtered.end(), [](const SPusItem& a, const SPusItem& b) { return a.ID < b.ID; });
    g_page = min(max(g_page, 0), PageCount() - 1);
    g_selected = -1;
}

static std::string Money(UINT32 v)
{
    std::string s = std::to_string(v);
    for (int i = (int)s.size() - 3; i > 0; i -= 3) s.insert(i, ".");
    return s;
}

static int StateFor(int ctl) { return g_pressed == ctl && g_hover == ctl ? 1 : (g_hover == ctl ? 2 : 0); }

static void DrawButton(const char* prefix, PusRect r, int ctl)
{
    char name[32]; sprintf_s(name, "%s_%d", prefix, StateFor(ctl));
    Blit(Spr(name), r.l, r.t, r.r - r.l, r.b - r.t);
}

static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    memset(g_bits, 0, (size_t)PUS_W * PUS_H * 4);
    Blit(Spr("bg"), 0, 0);

    EnterCriticalSection(&g_dataLock);
    // tabs: 0 = all, 1..5 = server categories
    for (int i = 0; i < TAB_COUNT; i++)
    {
        bool active = i == 0 ? g_activeCat == 0 : (i - 1 < (int)g_cats.size() && (int)g_cats[i - 1].ID == g_activeCat);
        PusRect r = PUS_TABS[i];
        Blit(Spr(active || g_hover == C_TAB0 + i ? "tab_1" : "tab_0"), r.l, r.t, r.r - r.l, r.b - r.t);
        std::string label = i == 0 ? "All" : (i - 1 < (int)g_cats.size() ? g_cats[i - 1].Name : "");
        Text(label, r, RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER);
    }

    Text("KC: " + Money(g_kc), PUS_TXT_CASH, RGB(255, 255, 0), 9, DT_CENTER | DT_VCENTER);
    Text("TL: " + Money(g_tl), PUS_TXT_TL, RGB(0xC0, 0xC0, 0xC0), 9, DT_CENTER | DT_VCENTER);
    if (g_search.empty() && !g_searchFocus)
        Text("Search for items...", PUS_TXT_SEARCH, RGB(0xF4, 0xDF, 0x8C), 9, DT_LEFT | DT_VCENTER);
    else
        Text(g_search + (g_searchFocus ? "_" : ""), PUS_TXT_SEARCH, RGB(255, 255, 255), 9, DT_LEFT | DT_VCENTER);

    for (int i = 0; i < ITEMS_PER_PAGE; i++)
    {
        int idx = g_page * ITEMS_PER_PAGE + i;
        if (idx >= (int)g_filtered.size()) break;
        const SPusItem& it = g_filtered[idx];
        const PusSlot& s = PUS_SLOTS[i];
        Blit(Spr("card"), s.cardX, s.cardY);
        PusRect ic = { s.cardX + PUS_CARD_ICON.l, s.cardY + PUS_CARD_ICON.t, s.cardX + PUS_CARD_ICON.r, s.cardY + PUS_CARD_ICON.b };
        Blit(Icon(it.IconID), ic.l, ic.t, ic.r - ic.l, ic.b - ic.t);
        PusRect nr = { s.cardX + PUS_CARD_NAME.l, s.cardY + PUS_CARD_NAME.t, s.cardX + PUS_CARD_NAME.r, s.cardY + PUS_CARD_NAME.b };
        Text(it.Name, nr, RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER);
        PusRect pr = { s.cardX + PUS_CARD_PRICE.l, s.cardY + PUS_CARD_PRICE.t, s.cardX + PUS_CARD_PRICE.r, s.cardY + PUS_CARD_PRICE.b };
        Text(Money(it.Price) + (it.PriceType ? " TL" : ""), pr, it.PriceType ? RGB(0xC0, 0xC0, 0xC0) : RGB(255, 255, 0), 10, DT_CENTER | DT_VCENTER);
        DrawButton("purchase", s.buy, C_BUY0 + i);
        Text("Purchase", s.buy, RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER);
    }
    if (g_filtered.empty())
    {
        const char* msg = !g_items.empty() ? "No items found." : (g_listRequested ? "Loading..." : "");
        Text(msg, { 40, 300, 767, 330 }, RGB(200, 200, 200), 10, DT_CENTER | DT_VCENTER);
    }

    DrawButton("prev", PUS_PREV, C_PREV);
    DrawButton("next", PUS_NEXT, C_NEXT);
    DrawButton("close", PUS_CLOSE, C_CLOSE);
    char page[32]; sprintf_s(page, "%d / %d", g_page + 1, PageCount());
    Text(page, { PUS_TXT_PAGE.l - 10, PUS_TXT_PAGE.t, PUS_TXT_PAGE.r + 10, PUS_TXT_PAGE.b }, RGB(255, 255, 255), 10, DT_CENTER | DT_VCENTER);
    if (!g_status.empty())
        Text(g_status, { 460, 572, 760, 598 }, RGB(230, 200, 120), 8, DT_RIGHT | DT_VCENTER);

    // confirm dialog on top
    if (g_confirmIdx >= 0 && g_confirmIdx < (int)g_filtered.size())
    {
        const SPusItem& it = g_filtered[g_confirmIdx];
        Blit(Spr("confirm_bg"), PUS_DLG.l, PUS_DLG.t);
        Blit(Icon(it.IconID), PUS_DLG_ICON.l, PUS_DLG_ICON.t, PUS_DLG_ICON.r - PUS_DLG_ICON.l, PUS_DLG_ICON.b - PUS_DLG_ICON.t);
        Text(it.Name, PUS_DLG_NAME, RGB(255, 255, 255), 10, DT_CENTER | DT_VCENTER);
        Text("Quantity: " + std::to_string(g_qty * max(1u, it.BuyCount)), PUS_DLG_QTY, RGB(255, 255, 255), 10, DT_CENTER | DT_VCENTER);
        Text(Money(it.Price * g_qty) + (it.PriceType ? " TL Balance" : " Knight Cash"), PUS_DLG_PRICE, RGB(255, 0, 0), 10, DT_CENTER | DT_VCENTER);
        Text(std::to_string(g_qty), PUS_DLG_COUNT, RGB(255, 255, 0x80), 10, DT_CENTER | DT_VCENTER);
        DrawButton("up", PUS_DLG_UP, C_DLG_UP);
        DrawButton("down", PUS_DLG_DOWN, C_DLG_DOWN);
        DrawButton("okbtn", PUS_DLG_OK, C_DLG_OK);
        Text("Confirm", PUS_DLG_OK, RGB(255, 255, 255), 10, DT_CENTER | DT_VCENTER);
        DrawButton("okbtn", PUS_DLG_CANCEL, C_DLG_CANCEL);
        Text("Cancel", PUS_DLG_CANCEL, RGB(255, 255, 255), 10, DT_CENTER | DT_VCENTER);
    }
    LeaveCriticalSection(&g_dataLock);

    RECT wr; GetWindowRect(g_hWnd, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { PUS_W, PUS_H };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
}

static void RequestList()
{
    QueueSend({ WIZ_HSACS_HOOK, PUS_SUBOP, 0 });
    g_listRequested = true;
}

static void RequestPurchase(UINT32 pusId, BYTE count)
{
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, PUS_SUBOP, 1 };
    p.insert(p.end(), (BYTE*)&pusId, (BYTE*)&pusId + 4);
    p.push_back(count);
    QueueSend(p);
}

static HWND FindGameWindow()
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

static void SetSearchFocus(bool on)
{
    if (g_searchFocus == on) return;
    g_searchFocus = on;
    if (on) SetForegroundWindow(g_hWnd);              // keyboard to us while typing
    else if (HWND game = FindGameWindow()) SetForegroundWindow(game);
}

static bool g_open = false;   // user intent; actual visibility also needs the game in front

static void Hide()
{
    SetSearchFocus(false);
    g_confirmIdx = -1;
    g_open = false;
    g_tipSlot = -1; ItemTip_Hide();
    ShowWindow(g_hWnd, SW_HIDE);
}

// Keep the panel glued to the game: visible only while the game (or the panel itself)
// is the foreground window and the game is not minimized.
static HCURSOR g_gameCursor = nullptr, g_fallbackCursor = nullptr;

static void SyncVisibility()
{
    HWND game = FindGameWindow();
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool want = g_open && game && !IsIconic(game) && pid == GetCurrentProcessId();
    if (want != (IsWindowVisible(g_hWnd) != FALSE))
    {
        if (want)
        {
            SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
            Paint();
        }
        else
            ShowWindow(g_hWnd, SW_HIDE);
    }

    // remember the cursor the game shows, so the panel can use the same one
    POINT p;
    CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) &&
        (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursor = ci.hCursor;
}

static HCURSOR BuildFallbackCursor()
{
    // separate load: CreateIconIndirect wants a bitmap that is not selected into a DC
    Sprite s;
    char path[MAX_PATH];
    sprintf_s(path, "%s\\cursor.pus", g_uiDir);
    if (!LoadPus(path, s)) return LoadCursor(nullptr, IDC_ARROW);
    DeleteDC(s.dc);
    HBITMAP mask = CreateBitmap(s.w, s.h, 1, 1, nullptr);
    ICONINFO ii = { FALSE, 1, 1, mask, s.bmp };   // hotspot at the arrow tip
    HCURSOR c = (HCURSOR)CreateIconIndirect(&ii);
    DeleteObject(mask);
    DeleteObject(s.bmp);
    return c ? c : LoadCursor(nullptr, IDC_ARROW);
}

// action on mouse-up over the same control it went down on
static void Activate(int ctl)
{
    EnterCriticalSection(&g_dataLock);
    if (ctl == C_CLOSE) { LeaveCriticalSection(&g_dataLock); Hide(); return; }
    if (ctl == C_PREV && g_page > 0) g_page--;
    if (ctl == C_NEXT && g_page + 1 < PageCount()) g_page++;
    if (ctl >= C_TAB0 && ctl < C_TAB0 + TAB_COUNT)
    {
        int i = ctl - C_TAB0;
        int cat = (i > 0 && i - 1 < (int)g_cats.size()) ? (int)g_cats[i - 1].ID : 0;
        if (i == 0 || i - 1 < (int)g_cats.size()) { g_activeCat = cat; g_page = 0; RebuildFiltered(); }
    }
    if (ctl >= C_BUY0 && ctl < C_BUY0 + ITEMS_PER_PAGE)
    {
        int idx = g_page * ITEMS_PER_PAGE + (ctl - C_BUY0);
        if (idx < (int)g_filtered.size()) { g_confirmIdx = idx; g_qty = 1; }
    }
    if (ctl == C_DLG_UP && g_qty < MAX_QTY) g_qty++;
    if (ctl == C_DLG_DOWN && g_qty > 1) g_qty--;
    if (ctl == C_DLG_CANCEL) g_confirmIdx = -1;
    if (ctl == C_DLG_OK && g_confirmIdx >= 0 && g_confirmIdx < (int)g_filtered.size())
    {
        const SPusItem& it = g_filtered[g_confirmIdx];
        RequestPurchase(it.ID, (BYTE)g_qty);
        g_status = "Purchase sent: " + it.Name + (g_qty > 1 ? " x" + std::to_string(g_qty) : "");
        g_confirmIdx = -1;
    }
    LeaveCriticalSection(&g_dataLock);
    SetSearchFocus(ctl == C_SEARCH);
    Paint();
}

static void Toggle()
{
    if (g_open) { Hide(); return; }
    g_open = true;
    RECT gr = { 0, 0, PUS_W, PUS_H };
    if (HWND game = FindGameWindow()) GetWindowRect(game, &gr);
    int x = gr.left + max(0, (int)(gr.right - gr.left - PUS_W) / 2);
    int y = gr.top + max(0, (int)(gr.bottom - gr.top - PUS_H) / 2);
    SetWindowPos(g_hWnd, HWND_TOPMOST, x, y, PUS_W, PUS_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    if (!g_listRequested) RequestList();
    Paint();
}

static void OnChar(WPARAM ch)
{
    if (!g_searchFocus) return;
    if (ch == VK_RETURN || ch == VK_ESCAPE) { SetSearchFocus(false); Paint(); return; }
    EnterCriticalSection(&g_dataLock);
    if (ch == VK_BACK) { if (!g_search.empty()) g_search.pop_back(); }
    else if (ch >= 32 && ch < 256 && g_search.size() < 24) g_search += (char)ch;
    g_page = 0;
    RebuildFiltered();
    LeaveCriticalSection(&g_dataLock);
    Paint();
}

static LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
    switch (msg)
    {
    case WM_MOUSEACTIVATE:
        return MA_NOACTIVATE; // keep keyboard focus in the game (search box activates explicitly)
    case WM_MOUSEMOVE:
        if (g_dragging)
        {
            POINT p; GetCursorPos(&p);
            RECT wr; GetWindowRect(h, &wr);
            SetWindowPos(h, nullptr, wr.left + p.x - g_dragFrom.x, wr.top + p.y - g_dragFrom.y, 0, 0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
            g_dragFrom = p;
            return 0;
        }
        if (!g_tracking)
        {
            TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 };
            g_tracking = TrackMouseEvent(&t) != FALSE;
        }
        if (int hit = HitTest(x, y); hit != g_hover) { g_hover = hit; Paint(); }
        {
            // item tooltip: the card above its Buy button. The game draws it in its own window, which this panel
            // covers, so it goes beside the panel (right side, or left when there is no room on the right).
            int tip = -1;
            if (g_confirmIdx < 0)
                for (int i = 0; i < ITEMS_PER_PAGE && tip < 0; i++)
                {
                    const PusSlot& s = PUS_SLOTS[i];
                    PusRect card = { s.cardX, s.cardY, s.buy.r, s.buy.t };
                    if (In(card, x, y)) tip = i;
                }
            if (tip != g_tipSlot)
            {
                g_tipSlot = tip;
                UINT32 item = 0;
                EnterCriticalSection(&g_dataLock);
                int idx = g_page * ITEMS_PER_PAGE + tip;
                if (tip >= 0 && idx < (int)g_filtered.size()) item = g_filtered[idx].ItemID;
                LeaveCriticalSection(&g_dataLock);
                if (item)
                {
                    const int TIP_W = 302;   // item_tooltip.cpp window width + gap
                    RECT wr; GetWindowRect(h, &wr);
                    RECT gr = wr; if (HWND game = Proxy_GameWnd()) GetWindowRect(game, &gr);
                    RECT a = { wr.left, wr.top + PUS_SLOTS[tip].cardY, wr.right, wr.top + PUS_SLOTS[tip].buy.t };
                    if (wr.right + TIP_W > gr.right && wr.left - TIP_W >= gr.left) a.right = wr.left - TIP_W - 4;
                    ItemTip_Show(item, a);
                }
                else ItemTip_Hide();
            }
        }
        return 0;
    case WM_MOUSELEAVE:
        g_tracking = false;
        if (g_tipSlot >= 0) { g_tipSlot = -1; ItemTip_Hide(); }
        if (g_hover != C_NONE) { g_hover = C_NONE; Paint(); }
        return 0;
    case WM_LBUTTONDOWN:
    {
        int hit = HitTest(x, y);
        if (hit == C_NONE && g_confirmIdx < 0 && y < 110)   // header area drags the window
        {
            g_dragging = true;
            GetCursorPos(&g_dragFrom);
            SetCapture(h);
            return 0;
        }
        g_pressed = hit;
        if (hit != C_NONE && hit != C_DLG_BODY) { SetCapture(h); Paint(); }
        else SetSearchFocus(false);
        return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (g_dragging) { g_dragging = false; return 0; }
        int hit = HitTest(x, y), pressed = g_pressed;
        g_pressed = C_NONE;
        if (pressed != C_NONE && pressed == hit) Activate(hit);
        else Paint();
        return 0;
    }
    case WM_SETCURSOR:
        // the game hides the system cursor and draws its own; over the panel it draws nothing
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursor ? g_gameCursor : g_fallbackCursor);
        return TRUE;
    case WM_TIMER: SyncVisibility(); return 0;
    case WM_CHAR: OnChar(wp); return 0;
    case WM_KILLFOCUS: if (g_searchFocus) { g_searchFocus = false; Paint(); } return 0;
    case WM_PUS_REFRESH: if (IsWindowVisible(h)) Paint(); return 0;
    case WM_PUS_TOGGLE: Toggle(); return 0;
    case WM_CLOSE: Hide(); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThread(LPVOID)
{
    GetModuleFileNameA(nullptr, g_uiDir, MAX_PATH);
    strcpy_s(strrchr(g_uiDir, '\\') + 1, 48, "HopeGuard\\pus_ui");

    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = L"NTT_PusStore";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Power Up Store", WS_POPUP, 0, 0, PUS_W, PUS_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("PUS: window creation failed"); return 0; }

    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), PUS_W, -PUS_H, 1, 32, BI_RGB };
    HBITMAP bmp = CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0);
    SelectObject(g_memDC, bmp);
    Log(Spr("bg") ? "PUS: window ready (UIF skin loaded)" : "PUS: window ready (skin MISSING - check pus_ui folder)");
    g_fallbackCursor = BuildFallbackCursor();
    SetTimer(g_hWnd, 1, 150, nullptr);

    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// =============================================================================
// Taskbar btn_powerup -> our panel, without patching any game code.
// The taskbar's ReceiveMessage(this, CN3UIBase* sender, DWORD msg) compares the
// sender's name (std::string at sender+0x58) with "btn_powerup"; msg 1 = click.
// We locate that function at runtime from its string reference, copy its
// vtable, and point the taskbar instance's vptr (heap data) at the copy.
// =============================================================================
typedef char(__thiscall* tReceiveMessage)(void*, BYTE*, DWORD);
struct SVtHook { DWORD origVt; DWORD* copy; tReceiveMessage orig; int slot; };
// fixed array in the DLL image (MEM_IMAGE): the heap scan below must never see our own records
struct SVtHookList
{
    SVtHook items[8]; int count = 0;
    SVtHook* begin() { return items; } SVtHook* end() { return items + count; }
    bool empty() const { return count == 0; }
    void push_back(const SVtHook& h) { if (count < 8) items[count++] = h; }
};
static SVtHookList g_vtHooks;

static const char* SenderName(BYTE* sender, char* out, size_t outSize)
{
    out[0] = 0;
    __try
    {
        DWORD size = *(DWORD*)(sender + 0x68), cap = *(DWORD*)(sender + 0x6C);
        const char* s = cap > 0xF ? *(const char**)(sender + 0x58) : (const char*)(sender + 0x58);
        if (size < outSize) { memcpy(out, s, size); out[size] = 0; }
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { out[0] = 0; }
    return out;
}

static char __fastcall hkTaskbarReceiveMessage(void* self, void*, BYTE* sender, DWORD msg)
{
    DWORD vt = *(DWORD*)self;
    tReceiveMessage orig = nullptr;
    for (auto& h : g_vtHooks)
        if (h.origVt == vt) { orig = h.orig; break; } // in-place patch: instances keep original vptr
    if (sender && msg == 1)
    {
        char name[64];
        SenderName(sender, name, sizeof(name));
        if (strncmp(name, "btn_powerup", 11) == 0 || strncmp(name, "btn_clanadv", 11) == 0)
        {
            char msg[96];
            sprintf_s(msg, "PUS taskbar: click '%s' -> Power Up Store", name);
            Log(msg);
            if (g_hWnd) PostMessageW(g_hWnd, WM_PUS_TOGGLE, 0, 0);
            return 1; // handled: skip the native (web) Power Up Store
        }
    }
    return orig ? orig(self, sender, msg) : 0;
}

static int SwapVptrsInRegion(DWORD* d, size_t cnt, DWORD origVt, DWORD copyVt)
{
    int swapped = 0;
    __try
    {
        for (size_t i = 0; i < cnt; i++)
            if (d[i] == origVt)
            {
                d[i] = copyVt;
                swapped++;
                char buf[96];
                sprintf_s(buf, "PUS taskbar: instance %p hooked (vt %08lX)", &d[i], origVt);
                Log(buf);
            }
    }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
    return swapped;
}

// =============================================================================
// Mouse click-through: the game polls GetAsyncKeyState(VK_LBUTTON/RBUTTON/MBUTTON)
// every frame instead of using window messages, so clicks on the panel also moved
// the character. Redirect the exe's import slot (data, no code patch) and report
// the mouse buttons as released while the cursor is over the visible panel.
// =============================================================================
typedef SHORT(WINAPI* tGetAsyncKeyState)(int);
static tGetAsyncKeyState g_oGetAsyncKeyState = nullptr;

static bool CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}

static SHORT WINAPI hkGetAsyncKeyState(int vKey)
{
    if ((vKey == VK_LBUTTON || vKey == VK_RBUTTON || vKey == VK_MBUTTON) &&
        (CursorOverPanel() || Rce_CursorOverPanel() || Cr_CursorOverPanel() || Sp_CursorOverPanel() || Cind_CursorOverPanel() || Csw_CursorOverPanel() || MerchCur_CursorOverPanel() || Lottery_CursorOverPanel() || MannerStore_CursorOverPanel() || DropBox_CursorOverPanel() || EvReward_CursorOverPanel() || Drop_CursorOverPanel() || Cross_CursorOverPanel() || Reconnect_CursorOverPanel() || SoloCapeIcon_CursorOverPanel() || Tag_CursorOverPanel() || JobChange_CursorOverPanel() || DropBtn_CursorOverPanel() || CamRange_CursorOverPanel() || GmPanel_CursorOverPanel() || Register_CursorOverPanel()))
        return 0;
    return g_oGetAsyncKeyState(vKey);
}

static void HookMouseImport(DWORD imgLo, DWORD dataLo, DWORD imgHi)
{
    FARPROC real = GetProcAddress(GetModuleHandleA("user32.dll"), "GetAsyncKeyState");
    if (!real) return;
    g_oGetAsyncKeyState = (tGetAsyncKeyState)real;
    int n = 0;
    char buf[128];
    for (DWORD a = (dataLo + 3) & ~3u; a + 4 < imgHi; a += 4)
    {
        MEMORY_BASIC_INFORMATION mbi{};
        if (!VirtualQuery((void*)a, &mbi, sizeof(mbi)) || mbi.State != MEM_COMMIT || (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)))
        { a = (((DWORD)mbi.BaseAddress + (DWORD)mbi.RegionSize + 3) & ~3u) - 4; continue; }
        if (*(DWORD*)a != (DWORD)real) continue;
        DWORD old;
        if (VirtualProtect((void*)a, 4, PAGE_READWRITE, &old))
        {
            *(DWORD*)a = (DWORD)hkGetAsyncKeyState;
            VirtualProtect((void*)a, 4, old, &old);
            sprintf_s(buf, "PUS mouse: GetAsyncKeyState import slot %08lX redirected", a);
            Log(buf);
            n++;
        }
    }
    if (!n) Log("PUS mouse: GetAsyncKeyState import slot NOT found (clicks will pass through)");
    (void)imgLo;
}

static bool InText(DWORD a, DWORD textLo, DWORD textHi) { return a >= textLo && a < textHi; }

static DWORD WINAPI TaskbarHookThread(LPVOID)
{
    BYTE* base = (BYTE*)GetModuleHandleA(nullptr);
    auto nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
    DWORD imgLo = (DWORD)base, imgHi = imgLo + nt->OptionalHeader.SizeOfImage;
    DWORD textLo = imgLo + 0x1000, textHi = 0;
    char buf[256];

    Sleep(2000); // vtables are static image data, present at load; short safety wait only

    // code range = the section holding the first code page; logged for reference
    {
        auto sec = IMAGE_FIRST_SECTION(nt);
        for (int i = 0; i < nt->FileHeader.NumberOfSections; i++, sec++)
        {
            DWORD lo = imgLo + sec->VirtualAddress, hi = lo + sec->Misc.VirtualSize;
            char name[9] = {}; memcpy(name, sec->Name, 8);
            sprintf_s(buf, "PUS taskbar: section %-8s %08lX-%08lX", name, lo, hi);
            Log(buf);
            if (textLo >= lo && textLo < hi) textHi = hi;
        }
    }

    // 1) every "btn_powerup\0" in the image (data section, above .text)
    std::vector<DWORD> strAddrs;
    for (DWORD a = textLo; a + 12 < imgHi; a++)
    {
        MEMORY_BASIC_INFORMATION mbi{};
        if (!VirtualQuery((void*)a, &mbi, sizeof(mbi)) || mbi.State != MEM_COMMIT || (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)))
        { a = (DWORD)mbi.BaseAddress + (DWORD)mbi.RegionSize - 1; continue; }
        DWORD end = min((DWORD)mbi.BaseAddress + (DWORD)mbi.RegionSize, imgHi) - 12;
        for (; a < end; a++)
            if (memcmp((void*)a, "btn_powerup\0", 12) == 0 && *(BYTE*)(a - 1) == 0)
            {
                strAddrs.push_back(a);
                sprintf_s(buf, "PUS taskbar: 'btn_powerup' @%08lX", a);
                Log(buf);
            }
    }
    if (strAddrs.empty()) { Log("PUS taskbar: 'btn_powerup' string not found"); return 0; }
    HookMouseImport(imgLo, textHi ? textHi : textLo, imgHi); // import table sits right after .text
    if (!textHi || textHi > strAddrs[0]) textHi = strAddrs[0] & ~0xFFFu; // code lies below the string data
    sprintf_s(buf, "PUS taskbar: code range %08lX-%08lX", textLo, textHi);
    Log(buf);

    // 2) functions referencing any copy (any imm32 operand), walked back to their prologue
    std::vector<DWORD> funcs;
    for (DWORD a = textLo; a + 5 < textHi; a++)
    {
        if (std::find(strAddrs.begin(), strAddrs.end(), *(DWORD*)a) == strAddrs.end()) continue;
        DWORD f = a;
        while (f > a - 0x4000 && !(*(BYTE*)(f - 1) == 0xCC && *(BYTE*)f == 0x55 && *(WORD*)(f + 1) == 0xEC8B)) f--;
        if (f <= a - 0x4000 || std::find(funcs.begin(), funcs.end(), f) != funcs.end()) continue;
        // only ReceiveMessage: it compares the sender name via
        // sub(name, len, "btn_powerup", 11) -> "push 0Bh; push offset str" = 6A 0B 68 <str>
        bool isRecvMsg = *(BYTE*)(a - 3) == 0x6A && *(BYTE*)(a - 2) == 0x0B && *(BYTE*)(a - 1) == 0x68;
        const BYTE* q = (const BYTE*)a - 8;
        sprintf_s(buf, "PUS taskbar: ref @%08lX fn %08lX receiveMessage=%d  pre: %02X %02X %02X %02X %02X %02X %02X %02X",
            a, f, isRecvMsg, q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7]);
        Log(buf);
        if (isRecvMsg) funcs.push_back(f);
    }
    sprintf_s(buf, "PUS taskbar: %u ReceiveMessage function(s)", (unsigned)funcs.size());
    Log(buf);

    // 3) vtables holding those functions (dword-aligned pointers outside .text)
    for (DWORD f : funcs)
    {
        for (DWORD a = (textHi + 3) & ~3u; a + 4 < imgHi; a += 4) // vtable slots are dword-aligned
        {
            MEMORY_BASIC_INFORMATION mbi{};
            if (!VirtualQuery((void*)a, &mbi, sizeof(mbi)) || mbi.State != MEM_COMMIT || (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)))
            { a = ((DWORD)mbi.BaseAddress + (DWORD)mbi.RegionSize + 3 & ~3u) - 4; continue; }
            if (*(DWORD*)a != f) continue;
            // walk back to the vtable start: previous dword is not a code pointer
            DWORD vt = a;
            while (InText(*(DWORD*)(vt - 4), textLo, textHi)) vt -= 4;
            int slot = (int)((a - vt) / 4);
            int n = 0;
            while (n < 256 && InText(*(DWORD*)(vt + n * 4), textLo, textHi)) n++;
            // XIGNCODE kapalı: orijinal vtable slot'unu YERİNDE yama -> tüm instance'lar anında hook'a düşer, native açılmaz
            DWORD slotAddr = vt + slot * 4, old;
            if (VirtualProtect((void*)slotAddr, 4, PAGE_EXECUTE_READWRITE, &old))
            {
                g_vtHooks.push_back({ vt, nullptr, (tReceiveMessage)f, slot });
                *(DWORD*)slotAddr = (DWORD)hkTaskbarReceiveMessage;
                VirtualProtect((void*)slotAddr, 4, old, &old);
                FlushInstructionCache(GetCurrentProcess(), (void*)slotAddr, 4);
                sprintf_s(buf, "PUS taskbar: vtable %08lX slot %d in-place patched (fn %08lX)", vt, slot, f);
                Log(buf);
            }
            break; // one vtable per function: stop scanning the rest of the image (huge speedup)
        }
    }
    if (g_vtHooks.empty()) { Log("PUS taskbar: no vtable found"); return 0; }

    // yerinde vtable yaması mevcut+gelecek tüm instance'ları kapsar; instance-swap gerekmez
    Log("PUS taskbar: in-place hook active");
    return 0;
}

// Called on the game thread for every WIZ_HSACS_HOOK packet (buf[0] = 0xE9).
void Pus_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 2) return;
    Reader r{ buf, len, 2 };
    BYTE sub = buf[1];
    char msg[128];
    if (sub == PUS_SUBOP)
    {
        UINT32 n = r.get<UINT32>();
        if (!r.ok || n > 100000) return;
        std::vector<SPusItem> items;
        for (UINT32 i = 0; i < n && r.ok; i++)
        {
            SPusItem it;
            it.ID = r.get<UINT32>(); it.ItemID = r.get<UINT32>(); it.Name = r.str();
            it.Price = r.get<UINT32>(); it.Cat = r.get<BYTE>(); it.BuyCount = r.get<UINT32>();
            it.PriceType = r.get<BYTE>(); it.IconID = r.get<UINT32>();
            if (r.ok) items.push_back(it);
        }
        EnterCriticalSection(&g_dataLock);
        g_items = items; g_confirmIdx = -1; RebuildFiltered();
        LeaveCriticalSection(&g_dataLock);
        sprintf_s(msg, "PUS recv: item list %u (parsed %u, ok=%d)", n, (unsigned)items.size(), r.ok);
        Log(msg);
    }
    else if (sub == PUS_SUBOP_CATEGORY)
    {
        UINT32 n = r.get<UINT32>();
        if (!r.ok || n > 1000) return;
        std::vector<SPusCategory> cats;
        for (UINT32 i = 0; i < n && r.ok; i++)
        {
            SPusCategory c;
            c.ID = r.get<UINT32>(); c.Name = r.str(); c.Status = r.get<BYTE>();
            if (r.ok) cats.push_back(c);
        }
        EnterCriticalSection(&g_dataLock);
        g_cats = cats;
        LeaveCriticalSection(&g_dataLock);
        sprintf_s(msg, "PUS recv: categories %u", (unsigned)cats.size());
        Log(msg);
    }
    else if (sub == PUS_SUBOP_CASHCHANGE || sub == 0xB9)
    {
        UINT32 kc = r.get<UINT32>(), tl = r.get<UINT32>();
        EnterCriticalSection(&g_dataLock);
        g_kc = kc; g_tl = tl;
        if (!g_status.empty()) g_status = "Balance updated.";
        LeaveCriticalSection(&g_dataLock);
        sprintf_s(msg, "PUS recv: cash KC=%u TL=%u", kc, tl);
        Log(msg);
    }
    else
        return;
    if (g_hWnd) PostMessageW(g_hWnd, WM_PUS_REFRESH, 0, 0);
}

void Pus_Init()
{
    InitializeCriticalSection(&g_sendLock);
    InitializeCriticalSection(&g_dataLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
    CloseHandle(CreateThread(nullptr, 0, TaskbarHookThread, nullptr, 0, nullptr));
}
