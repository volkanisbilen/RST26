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
#include <ctime>
#include <shellapi.h>

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
bool Daily_CursorOverPanel();
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
bool ClanKasa_CursorOverPanel(); // clan_kasa.cpp
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

// --- Window: skinned from re_powerupstore.uif (sprites in <game>\JstKO\pus_ui, layout in pus_layout.h,
//     both built by build_pus_uif.py) ---------------------------------------------------------------------
#include "pus_layout.h"
#pragma comment(lib, "msimg32.lib")

static const int PER_PAGE = 16, REFUND_PER_PAGE = 18, TAB_COUNT = 5, CART_MAX = 7, MAX_QTY = 99;
static const UINT WM_PUS_REFRESH = WM_APP + 1;
static const UINT WM_PUS_TOGGLE = WM_APP + 2;

// one product card (shop item or refundable purchase)
struct SCard
{
    UINT32 pusId = 0, itemId = 0, price = 0, qty = 1, icon = 0;
    UINT64 serial = 0;          // refund entries
    BYTE ptype = 0;             // 0 = KC, 1 = TL
    std::string name;
    UINT32 expiry = 0;
};
struct SRefund { UINT64 serial; UINT32 itemId, price, expiry; BYTE ptype; UINT16 count; };

static CRITICAL_SECTION g_dataLock;
static std::vector<SPusItem> g_items;
static std::vector<SCard> g_cards;              // what the grid shows (filtered / refund list)
static std::vector<SPusCategory> g_cats;
static std::vector<SCard> g_cart;
static std::vector<SRefund> g_refunds;
static UINT32 g_kc = 0, g_tl = 0;
void Pus_GetBalances(UINT32* kc, UINT32* tl) { *kc = g_kc; *tl = g_tl; }
static int g_activeCat = 0, g_page = 0;
static bool g_refundView = false;
static bool g_listRequested = false;
static std::string g_status, g_search, g_esn, g_gift;
static bool g_basketConfirm = false;
enum Focus { F_NONE, F_SEARCH, F_ESN, F_GIFT };
static int g_focus = F_NONE;
static int g_confirmIdx = -1, g_qty = 1;        // confirm dialog: index into g_cards

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

// an item icon is square (45x45 art): fitted into the box of the UIF, centered
static void BlitIcon(UINT32 id, PusRect r)
{
    int side = min(r.r - r.l, r.b - r.t);
    Blit(Icon(id), r.l + (r.r - r.l - side) / 2, r.t + (r.b - r.t - side) / 2, side, side);
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
    DrawTextA(dc, s.c_str(), (int)s.size(), &rc, (fmt & DT_WORDBREAK) ? (fmt | DT_NOPREFIX) : (fmt | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS));
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
enum Ctl { C_NONE, C_CLOSE, C_PREV, C_NEXT, C_SEARCHBTN, C_SEARCHEDIT, C_ESNEDIT, C_ESNBTN, C_REFUND, C_RESELLER,
           C_CLEAR, C_BUYALL, C_TAB0, C_CART0 = C_TAB0 + TAB_COUNT, C_BUY0 = C_CART0 + CART_MAX,
           C_BASK0 = C_BUY0 + PER_PAGE, C_RET0 = C_BASK0 + PER_PAGE,
           C_DLG_UP = C_RET0 + REFUND_PER_PAGE, C_DLG_DOWN, C_DLG_OK, C_DLG_CANCEL, C_DLG_GIFT, C_DLG_BODY,
           C_BD_OK, C_BD_CANCEL, C_BD_BODY };
static int g_hover = C_NONE, g_pressed = C_NONE;
static int g_tipSlot = -1;                       // product card under the mouse (item tooltip), -1 = none
void ItemTip_Show(UINT32 item, RECT anchor);     // item_tooltip.cpp: the game's own inventory tooltip
void ItemTip_Hide();
HWND Proxy_GameWnd();
static bool g_tracking = false, g_dragging = false;
static POINT g_dragFrom;

static bool In(PusRect r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

static int PerPage() { return g_refundView ? REFUND_PER_PAGE : PER_PAGE; }
static int PageCount() { return max(1, (int)((g_cards.size() + PerPage() - 1) / PerPage())); }
static int TabCount()   // tab buttons that have a category behind them
{
    return min(TAB_COUNT, (int)g_cats.size() + (g_cats.size() < (size_t)TAB_COUNT ? 1 : 0));
}
// tab i -> category id (0 = all); with fewer than 5 categories the first tab is "All"
static int TabCat(int i)
{
    bool all = g_cats.size() < (size_t)TAB_COUNT;
    if (all) return i == 0 ? 0 : (i - 1 < (int)g_cats.size() ? (int)g_cats[i - 1].ID : -1);
    return i < (int)g_cats.size() ? (int)g_cats[i].ID : -1;
}
static std::string TabName(int i)
{
    bool all = g_cats.size() < (size_t)TAB_COUNT;
    if (all) return i == 0 ? "All" : (i - 1 < (int)g_cats.size() ? g_cats[i - 1].Name : "");
    return i < (int)g_cats.size() ? g_cats[i].Name : "";
}

static int HitTest(int x, int y)
{
    if (g_confirmIdx >= 0)
    {
        if (In(PUS_DLG_UP, x, y)) return C_DLG_UP;
        if (In(PUS_DLG_DOWN, x, y)) return C_DLG_DOWN;
        if (In(PUS_DLG_OK, x, y)) return C_DLG_OK;
        if (In(PUS_DLG_CANCEL, x, y)) return C_DLG_CANCEL;
        if (In(PUS_DLG_GIFT, x, y)) return C_DLG_GIFT;
        return In(PUS_DLG, x, y) ? C_DLG_BODY : C_NONE;
    }
    if (g_basketConfirm)
    {
        if (In(PUS_BDLG_OK, x, y)) return C_BD_OK;
        if (In(PUS_BDLG_CANCEL, x, y)) return C_BD_CANCEL;
        return In(PUS_DLG, x, y) ? C_BD_BODY : C_NONE;
    }
    if (In(PUS_CLOSE, x, y)) return C_CLOSE;
    if (In(PUS_PREV, x, y)) return C_PREV;
    if (In(PUS_NEXT, x, y)) return C_NEXT;
    if (In(PUS_BTN_SEARCH, x, y)) return C_SEARCHBTN;
    if (In(PUS_BTN_ESN, x, y)) return C_ESNBTN;
    if (In(PUS_EDIT_ESN, x, y)) return C_ESNEDIT;
    if (In(PUS_EDIT_SEARCH, x, y)) return C_SEARCHEDIT;
    if (In(PUS_REFUND, x, y)) return C_REFUND;
    if (In(PUS_RESELLER, x, y)) return C_RESELLER;
    if (In(PUS_CLEAR, x, y)) return C_CLEAR;
    if (In(PUS_BUYALL, x, y)) return C_BUYALL;
    for (int i = 0; i < TAB_COUNT; i++)
        if (i < TabCount() && In(PUS_TABS[i], x, y)) return C_TAB0 + i;
    for (int i = 0; i < (int)g_cart.size() && i < CART_MAX; i++)
        if (In(PUS_CART[i].cancel, x, y)) return C_CART0 + i;
    if (g_refundView)
    {
        for (int i = 0; i < REFUND_PER_PAGE; i++)
        {
            if (g_page * REFUND_PER_PAGE + i >= (int)g_cards.size()) break;
            if (In(PUS_RROWS[i].ret, x, y)) return C_RET0 + i;
        }
        return C_NONE;
    }
    for (int i = 0; i < PER_PAGE; i++)
    {
        if (g_page * PER_PAGE + i >= (int)g_cards.size()) break;
        if (In(PUS_SLOTS[i].buy, x, y)) return C_BUY0 + i;
        if (In(PUS_SLOTS[i].bask, x, y)) return C_BASK0 + i;
    }
    return C_NONE;
}

static std::string Lower(std::string s) { for (auto& c : s) c = (char)tolower((unsigned char)c); return s; }

static std::string Money(UINT32 v)
{
    std::string s = std::to_string(v);
    for (int i = (int)s.size() - 3; i > 0; i -= 3) s.insert(i, ".");
    return s;
}

static SCard CardOf(const SPusItem& it)
{
    SCard c;
    c.pusId = it.ID; c.itemId = it.ItemID; c.price = it.Price; c.qty = max(1u, it.BuyCount);
    c.icon = it.IconID; c.ptype = it.PriceType; c.name = it.Name;
    return c;
}

// g_cards = what the grid shows: the refundable purchases, or the shop items of the tab / search
static void RebuildCards()
{
    g_cards.clear();
    if (g_refundView)
    {
        for (const SRefund& r : g_refunds)
        {
            SCard c;
            c.serial = r.serial; c.itemId = r.itemId; c.price = r.price; c.ptype = r.ptype; c.expiry = r.expiry;
            c.qty = r.count ? r.count : 1;
            c.name = "Item " + std::to_string(r.itemId);
            for (const SPusItem& it : g_items)
                if (it.ItemID == r.itemId) { c.name = it.Name; c.icon = it.IconID; break; }
            g_cards.push_back(c);
        }
    }
    else
    {
        std::string needle = Lower(g_search);
        std::vector<SPusItem> list;
        for (const SPusItem& it : g_items)
        {
            if (needle.empty() ? (g_activeCat != 0 && (int)it.Cat != g_activeCat) : Lower(it.Name).find(needle) == std::string::npos)
                continue;
            list.push_back(it);
        }
        std::sort(list.begin(), list.end(), [](const SPusItem& a, const SPusItem& b) { return a.ID < b.ID; });
        for (const SPusItem& it : list) g_cards.push_back(CardOf(it));
    }
    g_page = min(max(g_page, 0), PageCount() - 1);
}

static int StateFor(int ctl) { return g_pressed == ctl && g_hover == ctl ? 1 : (g_hover == ctl ? 2 : 0); }
static const char kStateChar[4] = { 'n', 'd', 'o', 'x' };

static void DrawButton(const char* prefix, PusRect r, int ctl, bool active = false)
{
    char name[40];
    int st = active ? 1 : StateFor(ctl);
    sprintf_s(name, "u_%s_%c", prefix, kStateChar[st]);
    Blit(Spr(name), r.l, r.t, r.r - r.l, r.b - r.t);
}

static const COLORREF WHITE = RGB(255, 255, 255), GOLD = RGB(255, 255, 0), SILVER = RGB(0xC0, 0xC0, 0xC0);

static std::string PriceText(UINT32 price, BYTE ptype) { return Money(price) + (ptype ? " TL" : " KC"); }

static std::string CartTotal()
{
    UINT32 kc = 0, tl = 0;
    for (const SCard& c : g_cart) (c.ptype ? tl : kc) += c.price;
    return (kc ? Money(kc) + " KC" : "") + (kc && tl ? " + " : "") + (tl ? Money(tl) + " TL" : "");
}

static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    memset(g_bits, 0, (size_t)PUS_W * PUS_H * 4);
    Blit(Spr("bg"), 0, 0);

    EnterCriticalSection(&g_dataLock);
    // left menu: category tabs, refund list, cash code
    for (int i = 0; i < TabCount(); i++)
    {
        bool active = !g_refundView && g_search.empty() && TabCat(i) == g_activeCat;
        DrawButton("tab", PUS_TABS[i], C_TAB0 + i, active);
        Text(TabName(i), PUS_TABS[i], WHITE, 9, DT_CENTER | DT_VCENTER, true);
    }
    DrawButton("tab", PUS_REFUND, C_REFUND, g_refundView);
    Text("Refund Pus Items", PUS_REFUND, WHITE, 9, DT_CENTER | DT_VCENTER, true);
    DrawButton("tab", PUS_RESELLER, C_RESELLER);
    Text("Buy Cash Code", PUS_RESELLER, WHITE, 9, DT_CENTER | DT_VCENTER, true);

    // balances
    Text(Money(g_kc), PUS_TXT_CASH, GOLD, 10, DT_CENTER | DT_VCENTER, true);
    Text(Money(g_tl), PUS_TXT_TL, SILVER, 10, DT_CENTER | DT_VCENTER, true);

    // search + ESN boxes
    if (g_search.empty() && g_focus != F_SEARCH)
        Text("Search for items...", { PUS_EDIT_SEARCH.l + 14, PUS_EDIT_SEARCH.t, PUS_EDIT_SEARCH.r - 30, PUS_EDIT_SEARCH.b }, RGB(0xF4, 0xDF, 0x8C), 9, DT_LEFT | DT_VCENTER);
    else
        Text(g_search + (g_focus == F_SEARCH ? "_" : ""), { PUS_EDIT_SEARCH.l + 14, PUS_EDIT_SEARCH.t, PUS_EDIT_SEARCH.r - 30, PUS_EDIT_SEARCH.b }, WHITE, 9, DT_LEFT | DT_VCENTER);
    DrawButton("search", PUS_BTN_SEARCH, C_SEARCHBTN);
    if (g_esn.empty() && g_focus != F_ESN)
        Text("E-pin type here...", { PUS_EDIT_ESN.l + 4, PUS_EDIT_ESN.t, PUS_EDIT_ESN.r - 4, PUS_EDIT_ESN.b }, RGB(0xF4, 0xDF, 0x8C), 9, DT_LEFT | DT_VCENTER);
    else
        Text(g_esn + (g_focus == F_ESN ? "_" : ""), { PUS_EDIT_ESN.l + 4, PUS_EDIT_ESN.t, PUS_EDIT_ESN.r - 4, PUS_EDIT_ESN.b }, WHITE, 9, DT_LEFT | DT_VCENTER);
    DrawButton("esn", PUS_BTN_ESN, C_ESNBTN);
    Text("Use ESN", PUS_BTN_ESN, WHITE, 9, DT_CENTER | DT_VCENTER, true);

    // product cards / refund list
    time_t now = time(nullptr);
    if (g_refundView)
    {
        Blit(Spr("u_rbg"), PUS_RBG.l, PUS_RBG.t);
        for (int i = 0; i < REFUND_PER_PAGE; i++)
        {
            int idx = g_page * REFUND_PER_PAGE + i;
            if (idx >= (int)g_cards.size()) break;
            const SCard& c = g_cards[idx];
            const PusRefundRow& r = PUS_RROWS[i];
            Blit(Spr("u_rframe"), r.icon.l - 4, r.icon.t - 4);
            BlitIcon(c.icon, r.icon);
            Text(c.name, r.name, WHITE, 9, DT_LEFT | DT_VCENTER, true);
            long left = (long)c.expiry - (long)now;
            char t[64];
            if (left > 0) sprintf_s(t, "Refund Price : %s %s  (%ld min)", Money(c.price).c_str(), c.ptype ? "TL" : "KC", left / 60 + 1);
            else sprintf_s(t, "Refund Price : %s %s  (expired)", Money(c.price).c_str(), c.ptype ? "TL" : "KC");
            Text(t, r.price, c.ptype ? SILVER : GOLD, 8, DT_LEFT | DT_VCENTER);
            Blit(Spr("u_rbtnframe"), r.ret.l - 5, r.ret.t - 5);
            DrawButton("ret", r.ret, C_RET0 + i);
        }
    }
    else
    for (int i = 0; i < PER_PAGE; i++)
    {
        int idx = g_page * PER_PAGE + i;
        if (idx >= (int)g_cards.size()) break;
        const SCard& c = g_cards[idx];
        const PusSlot& s = PUS_SLOTS[i];
        Blit(Spr("u_card"), s.card.l, s.card.t);
        BlitIcon(c.icon, s.icon);
        Text(c.name, s.name, WHITE, 8, DT_CENTER | DT_VCENTER);
        Text("Price: " + PriceText(c.price, c.ptype), s.price, c.ptype ? SILVER : GOLD, 8, DT_LEFT | DT_VCENTER);
        Text("Quantity: " + std::to_string(c.qty), s.qty, WHITE, 8, DT_LEFT | DT_VCENTER);
        DrawButton("buy", s.buy, C_BUY0 + i);
        Text("Buy", s.buy, WHITE, 9, DT_CENTER | DT_VCENTER, true);
        DrawButton("bask", s.bask, C_BASK0 + i);
        Text("Basket", s.bask, WHITE, 9, DT_CENTER | DT_VCENTER, true);
    }
    if (g_cards.empty())
    {
        const char* msg = g_refundView ? "No purchase can be refunded." : (!g_items.empty() ? "No items found." : (g_listRequested ? "Loading..." : ""));
        Text(msg, { 266, 300, 991, 330 }, RGB(200, 200, 200), 11, DT_CENTER | DT_VCENTER);
    }

    // basket
    for (int i = 0; i < (int)g_cart.size() && i < CART_MAX; i++)
    {
        const SCard& c = g_cart[i];
        const PusCartRow& r = PUS_CART[i];
        Blit(Spr("u_cartframe"), r.frame.l, r.frame.t);
        BlitIcon(c.icon, r.icon);
        Text(c.name, r.name, WHITE, 8, DT_LEFT | DT_VCENTER);
        Text(PriceText(c.price, c.ptype), r.price, c.ptype ? SILVER : GOLD, 8, DT_LEFT | DT_VCENTER);
        DrawButton("cancel", r.cancel, C_CART0 + i);
    }
    if (!g_cart.empty()) Text(CartTotal(), PUS_TXT_TOTAL, GOLD, 9, DT_CENTER | DT_VCENTER, true);
    DrawButton("buyall", PUS_BUYALL, C_BUYALL);
    Text("Buy All", PUS_BUYALL, WHITE, 9, DT_CENTER | DT_VCENTER, true);
    DrawButton("clear", PUS_CLEAR, C_CLEAR);
    Text("Clear List", PUS_CLEAR, WHITE, 8, DT_CENTER | DT_VCENTER);

    // paging, close, status
    DrawButton("prev", PUS_PREV, C_PREV);
    DrawButton("next", PUS_NEXT, C_NEXT);
    char page[32]; sprintf_s(page, "%d / %d", g_page + 1, PageCount());
    Text(page, { PUS_TXT_PAGE.l - 10, PUS_TXT_PAGE.t, PUS_TXT_PAGE.r + 10, PUS_TXT_PAGE.b }, WHITE, 9, DT_CENTER | DT_VCENTER);
    DrawButton("close", PUS_CLOSE, C_CLOSE);
    if (!g_status.empty())
        Text(g_status, { 690, 586, 1000, 608 }, RGB(230, 200, 120), 9, DT_RIGHT | DT_VCENTER);

    // purchase dialog on top
    if (g_confirmIdx >= 0 && g_confirmIdx < (int)g_cards.size())
    {
        const SCard& c = g_cards[g_confirmIdx];
        Blit(Spr("u_dlg"), PUS_DLG.l, PUS_DLG.t);
        BlitIcon(c.icon, PUS_DLG_ICON);
        Text(c.name, PUS_DLG_NAME, WHITE, 10, DT_LEFT | DT_VCENTER, true);
        Text("Price: " + PriceText(c.price, c.ptype), PUS_DLG_PRICE, c.ptype ? SILVER : GOLD, 9, DT_LEFT | DT_VCENTER);
        Text("Quantity: " + std::to_string(g_qty * c.qty), PUS_DLG_QTY, WHITE, 9, DT_LEFT | DT_VCENTER);
        Text(std::to_string(g_qty), PUS_DLG_COUNT, RGB(255, 255, 0x80), 10, DT_CENTER | DT_VCENTER, true);
        Text(PriceText(c.price * g_qty, c.ptype) + (c.ptype ? " TL Balance" : " Knight Cash"), PUS_DLG_AFTER, RGB(255, 80, 80), 10, DT_CENTER | DT_VCENTER, true);
        DrawButton("dup", PUS_DLG_UP, C_DLG_UP);
        DrawButton("ddown", PUS_DLG_DOWN, C_DLG_DOWN);
        DrawButton("dok", PUS_DLG_OK, C_DLG_OK);
        Text(g_gift.empty() ? "Confirm" : "Send Gift", PUS_DLG_OK, WHITE, 10, DT_CENTER | DT_VCENTER, true);
        DrawButton("dno", PUS_DLG_CANCEL, C_DLG_CANCEL);
        Text("Cancel", PUS_DLG_CANCEL, WHITE, 10, DT_CENTER | DT_VCENTER, true);
        Text(g_gift + (g_focus == F_GIFT ? "_" : ""), { PUS_DLG_GIFT.l + 7, PUS_DLG_GIFT.t, PUS_DLG_GIFT.r - 4, PUS_DLG_GIFT.b }, WHITE, 9, DT_LEFT | DT_VCENTER);
    }
    // basket dialog
    if (g_basketConfirm)
    {
        Blit(Spr("u_bdlg"), PUS_DLG.l, PUS_DLG.t);
        Text("You cannot undo the purchase. The total amount will be deducted from your balance. Do you want to buy the products in your cart?",
            PUS_BDLG_TEXT, WHITE, 10, DT_CENTER | DT_TOP | DT_WORDBREAK);
        Text(CartTotal(), PUS_BDLG_TOTAL, RGB(255, 80, 80), 10, DT_CENTER | DT_VCENTER, true);
        DrawButton("bok", PUS_BDLG_OK, C_BD_OK);
        Text("Confirm", PUS_BDLG_OK, WHITE, 10, DT_CENTER | DT_VCENTER, true);
        DrawButton("bno", PUS_BDLG_CANCEL, C_BD_CANCEL);
        Text("Cancel", PUS_BDLG_CANCEL, WHITE, 10, DT_CENTER | DT_VCENTER, true);
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

static void RequestRefund(UINT64 serial)
{
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, 0xD4, 0 };
    p.insert(p.end(), (BYTE*)&serial, (BYTE*)&serial + 8);
    p.push_back(0);
    QueueSend(p);
}

static void RequestGift(BYTE ptype, UINT32 pusId, const std::string& name)
{
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, PUS_SUBOP, 3, ptype };
    p.insert(p.end(), (BYTE*)&pusId, (BYTE*)&pusId + 4);
    p.push_back((BYTE)(name.size() & 0xFF)); p.push_back((BYTE)(name.size() >> 8));
    p.insert(p.end(), name.begin(), name.end());
    QueueSend(p);
}

static void RequestEsn(const std::string& code)
{
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, PUS_SUBOP, 4, (BYTE)(code.size() & 0xFF), (BYTE)(code.size() >> 8) };
    p.insert(p.end(), code.begin(), code.end());
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

static void SetFocusBox(int f)
{
    if (g_focus == f) return;
    bool was = g_focus != F_NONE;
    g_focus = f;
    if (f != F_NONE) SetForegroundWindow(g_hWnd);     // keyboard to us while typing
    else if (was) { if (HWND game = FindGameWindow()) SetForegroundWindow(game); }
}

static bool g_open = false;   // user intent; actual visibility also needs the game in front

static void Hide()
{
    SetFocusBox(F_NONE);
    g_confirmIdx = -1; g_basketConfirm = false; g_gift.clear();
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

// the "Buy Cash Code" page: JstKO\pus_buycode.txt (first line = URL) when present
static void OpenBuyCode()
{
    char path[MAX_PATH];
    sprintf_s(path, "%s\\..\\pus_buycode.txt", g_uiDir);
    FILE* f = nullptr;
    char url[512] = {};
    if (fopen_s(&f, path, "r") == 0 && f)
    {
        if (fgets(url, sizeof(url), f))
            for (char* p = url + strlen(url); p > url && (p[-1] == '\n' || p[-1] == '\r' || p[-1] == ' '); *--p = 0) {}
        fclose(f);
    }
    if (!strncmp(url, "http", 4)) ShellExecuteA(nullptr, "open", url, nullptr, nullptr, SW_SHOWNORMAL);
    else g_status = "Ask the server staff for a cash code (E-pin).";
}

static void ApplyEsn()
{
    if (g_esn.size() < 8) { g_status = "Type your E-pin first."; return; }
    RequestEsn(g_esn);
    g_status = "Checking the E-pin...";
}

// action on mouse-up over the same control it went down on
static void Activate(int ctl)
{
    EnterCriticalSection(&g_dataLock);
    if (ctl == C_CLOSE) { LeaveCriticalSection(&g_dataLock); Hide(); return; }
    int focus = F_NONE;
    if (ctl == C_SEARCHEDIT) focus = F_SEARCH;
    if (ctl == C_ESNEDIT) focus = F_ESN;
    if (ctl == C_PREV && g_page > 0) g_page--;
    if (ctl == C_NEXT && g_page + 1 < PageCount()) g_page++;
    if (ctl == C_SEARCHBTN) { g_page = 0; g_refundView = false; RebuildCards(); }
    if (ctl == C_ESNBTN) ApplyEsn();
    if (ctl >= C_TAB0 && ctl < C_TAB0 + TAB_COUNT)
    {
        int cat = TabCat(ctl - C_TAB0);
        if (cat >= 0) { g_activeCat = cat; g_page = 0; g_refundView = false; g_search.clear(); RebuildCards(); }
    }
    if (ctl == C_REFUND)
    {
        g_refundView = !g_refundView; g_page = 0; RebuildCards();
        if (g_refundView && g_refunds.empty()) g_status = "No purchase can be refunded.";
    }
    if (ctl == C_RESELLER) OpenBuyCode();
    if (ctl >= C_BUY0 && ctl < C_BUY0 + PER_PAGE)
    {
        int idx = g_page * PER_PAGE + (ctl - C_BUY0);
        if (idx < (int)g_cards.size()) { g_confirmIdx = idx; g_qty = 1; g_gift.clear(); }
    }
    if (ctl >= C_RET0 && ctl < C_RET0 + REFUND_PER_PAGE)
    {
        int idx = g_page * REFUND_PER_PAGE + (ctl - C_RET0);
        if (idx < (int)g_cards.size()) { RequestRefund(g_cards[idx].serial); g_status = "Refund requested: " + g_cards[idx].name; }
    }
    if (ctl >= C_BASK0 && ctl < C_BASK0 + PER_PAGE)
    {
        int idx = g_page * PER_PAGE + (ctl - C_BASK0);
        if (idx < (int)g_cards.size())
        {
            if ((int)g_cart.size() < CART_MAX) g_cart.push_back(g_cards[idx]);
            else g_status = "The basket is full.";
        }
    }
    if (ctl >= C_CART0 && ctl < C_CART0 + CART_MAX && ctl - C_CART0 < (int)g_cart.size())
        g_cart.erase(g_cart.begin() + (ctl - C_CART0));
    if (ctl == C_CLEAR) g_cart.clear();
    if (ctl == C_BUYALL) { if (!g_cart.empty()) g_basketConfirm = true; else g_status = "The basket is empty."; }
    if (ctl == C_BD_CANCEL) g_basketConfirm = false;
    if (ctl == C_BD_OK && g_basketConfirm)
    {
        for (const SCard& c : g_cart) RequestPurchase(c.pusId, 1);
        g_status = "Basket purchase sent (" + std::to_string(g_cart.size()) + " items)";
        g_cart.clear();
        g_basketConfirm = false;
    }
    if (ctl == C_DLG_GIFT) focus = F_GIFT;
    if (ctl == C_DLG_UP && g_qty < MAX_QTY) g_qty++;
    if (ctl == C_DLG_DOWN && g_qty > 1) g_qty--;
    if (ctl == C_DLG_CANCEL) { g_confirmIdx = -1; g_gift.clear(); }
    if (ctl == C_DLG_OK && g_confirmIdx >= 0 && g_confirmIdx < (int)g_cards.size())
    {
        const SCard& c = g_cards[g_confirmIdx];
        if (!g_gift.empty())
        {
            for (int k = 0; k < g_qty; k++) RequestGift(c.ptype, c.pusId, g_gift);
            g_status = "Gift sent to " + g_gift + ": " + c.name + (g_qty > 1 ? " x" + std::to_string(g_qty) : "");
        }
        else
        {
            RequestPurchase(c.pusId, (BYTE)g_qty);
            g_status = "Purchase sent: " + c.name + (g_qty > 1 ? " x" + std::to_string(g_qty) : "");
        }
        g_confirmIdx = -1; g_gift.clear();
    }
    LeaveCriticalSection(&g_dataLock);
    SetFocusBox(focus);
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
    RequestList();      // always fresh: prices, new items, the refund list
    Paint();
}

static void OnChar(WPARAM ch)
{
    if (g_focus == F_NONE) return;
    EnterCriticalSection(&g_dataLock);
    if (ch == VK_ESCAPE) { LeaveCriticalSection(&g_dataLock); SetFocusBox(F_NONE); Paint(); return; }
    if (g_focus == F_ESN)
    {
        if (ch == VK_RETURN) ApplyEsn();
        else if (ch == VK_BACK) { if (!g_esn.empty()) g_esn.pop_back(); }
        else if (isalnum((int)ch) && ch < 128 && g_esn.size() < 19) g_esn += (char)toupper((int)ch);
        else if (ch == '-' && g_esn.size() < 19) g_esn += '-';
        else if (ch == 22)   // Ctrl+V: paste the code
        {
            if (OpenClipboard(g_hWnd))
            {
                if (HANDLE h = GetClipboardData(CF_TEXT))
                    if (const char* t = (const char*)GlobalLock(h))
                    {
                        for (; *t && g_esn.size() < 19; t++)
                            if (isalnum((unsigned char)*t) || *t == '-') g_esn += (char)toupper((unsigned char)*t);
                        GlobalUnlock(h);
                    }
                CloseClipboard();
            }
        }
    }
    else if (g_focus == F_GIFT)
    {
        if (ch == VK_RETURN) { LeaveCriticalSection(&g_dataLock); SetFocusBox(F_NONE); Paint(); return; }
        if (ch == VK_BACK) { if (!g_gift.empty()) g_gift.pop_back(); }
        else if (ch >= 32 && ch < 256 && g_gift.size() < 20) g_gift += (char)ch;
    }
    else
    {
        if (ch == VK_RETURN) { LeaveCriticalSection(&g_dataLock); SetFocusBox(F_NONE); Paint(); return; }
        if (ch == VK_BACK) { if (!g_search.empty()) g_search.pop_back(); }
        else if (ch >= 32 && ch < 256 && g_search.size() < 24) g_search += (char)ch;
        g_page = 0; g_refundView = false;
        RebuildCards();
    }
    LeaveCriticalSection(&g_dataLock);
    Paint();
}

static LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
    switch (msg)
    {
    case WM_MOUSEACTIVATE:
        return MA_NOACTIVATE; // keep keyboard focus in the game (edit boxes activate explicitly)
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
            // item tooltip: the card under the mouse. The game draws it in its own window, which this panel
            // covers, so it goes beside the panel (right side, or left when there is no room on the right).
            int tip = -1;
            if (g_confirmIdx < 0 && !g_basketConfirm && !g_refundView)
                for (int i = 0; i < PER_PAGE && tip < 0; i++)
                    if (In(PUS_SLOTS[i].card, x, y)) tip = i;
            if (tip != g_tipSlot)
            {
                g_tipSlot = tip;
                UINT32 item = 0;
                EnterCriticalSection(&g_dataLock);
                int idx = g_page * PER_PAGE + tip;
                if (tip >= 0 && idx < (int)g_cards.size()) item = g_cards[idx].itemId;
                LeaveCriticalSection(&g_dataLock);
                if (item)
                {
                    const int TIP_W = 302;   // item_tooltip.cpp window width + gap
                    RECT wr; GetWindowRect(h, &wr);
                    RECT gr = wr; if (HWND game = Proxy_GameWnd()) GetWindowRect(game, &gr);
                    RECT a = { wr.left, wr.top + PUS_SLOTS[tip].card.t, wr.right, wr.top + PUS_SLOTS[tip].card.b };
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
        if (hit == C_NONE && g_confirmIdx < 0 && !g_basketConfirm && y < 36)   // title bar drags the window
        {
            g_dragging = true;
            GetCursorPos(&g_dragFrom);
            SetCapture(h);
            return 0;
        }
        g_pressed = hit;
        if (hit != C_NONE && hit != C_DLG_BODY && hit != C_BD_BODY) { SetCapture(h); Paint(); }
        else SetFocusBox(F_NONE);
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
    case WM_KILLFOCUS: if (g_focus != F_NONE) { g_focus = F_NONE; Paint(); } return 0;
    case WM_PUS_REFRESH: if (IsWindowVisible(h)) Paint(); return 0;
    case WM_PUS_TOGGLE: Toggle(); return 0;
    case WM_CLOSE: Hide(); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThread(LPVOID)
{
    GetModuleFileNameA(nullptr, g_uiDir, MAX_PATH);
    strcpy_s(strrchr(g_uiDir, '\\') + 1, 48, "JstKO\\pus_ui");

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
        (CursorOverPanel() || Rce_CursorOverPanel() || Cr_CursorOverPanel() || Sp_CursorOverPanel() || Cind_CursorOverPanel() || Csw_CursorOverPanel() || MerchCur_CursorOverPanel() || Lottery_CursorOverPanel() || Daily_CursorOverPanel() || MannerStore_CursorOverPanel() || DropBox_CursorOverPanel() || EvReward_CursorOverPanel() || Drop_CursorOverPanel() || Cross_CursorOverPanel() || Reconnect_CursorOverPanel() || SoloCapeIcon_CursorOverPanel() || Tag_CursorOverPanel() || JobChange_CursorOverPanel() || DropBtn_CursorOverPanel() || ClanKasa_CursorOverPanel() || CamRange_CursorOverPanel() || GmPanel_CursorOverPanel() || Register_CursorOverPanel()))
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
        g_items = items; g_confirmIdx = -1; RebuildCards();
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
        if (g_activeCat != 0)       // keep the open tab when it still exists
        {
            bool found = false;
            for (auto& c : cats) found |= (int)c.ID == g_activeCat;
            if (!found) g_activeCat = 0;
        }
        LeaveCriticalSection(&g_dataLock);
        sprintf_s(msg, "PUS recv: categories %u", (unsigned)cats.size());
        Log(msg);
    }
    else if (sub == PUS_SUBOP_CASHCHANGE || sub == 0xB9)
    {
        UINT32 kc = r.get<UINT32>(), tl = r.get<UINT32>();
        EnterCriticalSection(&g_dataLock);
        g_kc = kc; g_tl = tl;
        LeaveCriticalSection(&g_dataLock);
        sprintf_s(msg, "PUS recv: cash KC=%u TL=%u", kc, tl);
        Log(msg);
    }
    else if (sub == 0xDC)       // ESN redeem answer: [result][u32 kc][u32 tl]
    {
        BYTE res = r.get<BYTE>();
        UINT32 kc = r.get<UINT32>(), tl = r.get<UINT32>();
        static const char* texts[] = { "E-pin accepted. Your balance was updated.", "This E-pin is not valid.",
            "This E-pin was already used.", "This E-pin is disabled or expired.", "Too many wrong tries. Wait a few minutes.", "Try again in a moment." };
        EnterCriticalSection(&g_dataLock);
        if (r.ok) { g_kc = kc; g_tl = tl; }
        g_status = texts[min((int)res, 5)];
        if (res == 0) g_esn.clear();
        LeaveCriticalSection(&g_dataLock);
        sprintf_s(msg, "PUS recv: ESN result %u KC=%u TL=%u", (unsigned)res, kc, tl);
        Log(msg);
    }
    else if (sub == 0xD4)       // refund: 1 list, 2..6 errors, 7 done, 8 add
    {
        BYTE op = r.get<BYTE>();
        EnterCriticalSection(&g_dataLock);
        if (op == 1)
        {
            UINT16 n = r.get<UINT16>();
            std::vector<SRefund> list;
            for (UINT16 i = 0; i < n && r.ok; i++)
            {
                SRefund e = {};
                e.serial = r.get<UINT64>(); e.itemId = r.get<UINT32>(); e.price = r.get<UINT32>(); e.expiry = r.get<UINT32>();
                e.count = 1;
                if (r.ok) list.push_back(e);
            }
            g_refunds = list;
        }
        else if (op == 8)
        {
            SRefund e = {};
            e.serial = r.get<UINT64>(); e.itemId = r.get<UINT32>(); e.count = r.get<UINT16>(); e.price = r.get<UINT32>();
            r.get<UINT16>(); e.expiry = r.get<UINT32>(); e.ptype = r.get<BYTE>();
            if (r.ok) g_refunds.push_back(e);
        }
        else if (op == 7)
        {
            UINT64 serial = r.get<UINT64>();
            for (size_t i = 0; i < g_refunds.size(); i++)
                if (g_refunds[i].serial == serial) { g_refunds.erase(g_refunds.begin() + i); break; }
            g_status = "Item refunded.";
        }
        else if (op >= 2 && op <= 6)
        {
            static const char* texts[] = { "", "", "Refund: item not found.", "Refund: the time is over.", "Refund: wait a few seconds.",
                "Refund: the item is not in your bag.", "Refund: the item was used." };
            g_status = texts[op];
        }
        RebuildCards();
        LeaveCriticalSection(&g_dataLock);
        sprintf_s(msg, "PUS recv: refund op %u (list %u)", (unsigned)op, (unsigned)g_refunds.size());
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
