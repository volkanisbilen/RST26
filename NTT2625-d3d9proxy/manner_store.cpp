// Special Store [Loyalty & Manner] for the 2625 client (d3d9 proxy), same technique as lottery_panel.cpp:
// a GDI layered overlay window fed from the recv hook. Skin: build_manner_assets.py -> manner_ui\*.pus +
// manner_layout.h (pieces of the game's tournament_chaos_2021_a01 atlas); item icons + names: manner_ui\.
//
// Server: GameServer MannerStore.cpp, WIZ_HSACS_HOOK 0xE9 / sub MANNER 0xF0:
//   C->S 1 open                    S->C 1 [u32 manner][u32 loyalty][u32 daily secs left][u16 daily points][u16 n]
//                                          n*[u16 index][u8 tab][u32 item][u16 count][u32 price][u8 currency][u32 days]
//   C->S 2 [u16 index] buy         S->C 2 [u8 result][u32 manner][u32 loyalty]   1 ok, 2 points, 3 bag full, 4 invalid
//   C->S 3 daily reward            S->C 3 [u8 result][u32 manner][u32 daily secs left]   1 ok, 2 not yet
//   C->S 4 [u32 item] tooltip      S->C 4 [u32 item][str name][u32 icon][u8 n] n*([u8 colour][str line])
//                                  (asked once per item on hover and cached; also gives name / icon of new items)
// Opened from the taskbar event menu ('Opens the event' -> Manner Store, genie_hg.cpp HG_EventMenu entry 0).
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include <map>
#include "manner_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();

static const DWORD KO_SND_FNC = 0x00704070;
static const DWORD KO_PTR_PKT = 0x01115914;
static const BYTE  WIZ_HSACS_HOOK = 0xE9;
static const BYTE  MS_SUBOP = 0xF0;            // HSACSXOpCodes::MANNER
static const int   PAGE_SLOTS = 16;

struct Rc { int l, t, r, b; };
static Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }

// ---------------------------------------------------------------- state
struct MsItem { UINT16 index; BYTE tab; UINT32 item; UINT16 count; UINT32 price; BYTE currency; UINT32 days; };
struct MsState
{
    std::vector<MsItem> items;
    UINT32 manner = 0, loyalty = 0, dailyLeft = 0, dailyPoints = 250;
    DWORD  dailyAt = 0;
    int    tab = 0, page = 0;
    int    selected = -1;                       // index into items
    std::string msg; DWORD msgAt = 0; bool msgOk = false;
    bool   busy = false;                        // a buy / reward request is on the way
};

static CRITICAL_SECTION g_lock, g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQ;
static MsState g_st;
static bool  g_open = false;
static POINT g_panelPos = { -1, -1 };
static HWND  g_hWnd = nullptr;
static HDC   g_memDC = nullptr;
static void* g_bits = nullptr;
static HCURSOR g_gameCursor = nullptr;
static const UINT WM_MS_REFRESH = WM_APP + 61;

static UINT32 DailyLeft(const MsState& s)
{
    DWORD el = (GetTickCount() - s.dailyAt) / 1000;
    return el >= s.dailyLeft ? 0 : s.dailyLeft - el;
}

static std::vector<int> TabItems(const MsState& s)
{
    std::vector<int> v;
    for (int i = 0; i < (int)s.items.size(); i++) if (s.items[i].tab == s.tab) v.push_back(i);
    return v;
}

// ---------------------------------------------------------------- send
static void QueueSend(const std::vector<BYTE>& p)
{
    EnterCriticalSection(&g_sendLock);
    g_sendQ.push_back(p);
    LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();
}

void MannerStore_FlushSendQueue()
{
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_sendLock);
    pending.swap(g_sendQ);
    LeaveCriticalSection(&g_sendLock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
        char b[64]; sprintf_s(b, "MANNER send: op %u", p.size() > 2 ? p[2] : 0); Log(b);
    }
}

// ---------------------------------------------------------------- sprites / icons
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<std::string, Sprite> g_skin;
static std::map<UINT32, Sprite> g_icons;
static std::map<UINT32, std::pair<UINT32, std::string>> g_names;   // item -> icon, name
static char g_dir[MAX_PATH];
static bool g_loaded = false;

struct TipData { std::string name; UINT32 icon = 0; std::vector<std::pair<BYTE, std::string>> lines; };
static std::map<UINT32, TipData> g_tips;          // item -> server tooltip (g_lock)
static std::map<UINT32, DWORD> g_tipAsked;        // item -> when it was asked (window thread)
static HWND  g_hWndTip = nullptr;
static HDC   g_memDCTip = nullptr;
static void* g_bitsTip = nullptr;
static const int TIP_W = 280, TIP_MAXH = 760;
static UINT32 g_tipItem = 0;
static int g_tipSlot = -1;

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

static void LoadSkin()
{
    if (g_loaded) return;
    g_loaded = true;
    GetModuleFileNameA(nullptr, g_dir, MAX_PATH);
    *(strrchr(g_dir, '\\') + 1) = 0;
    const char* names[] = { "bg", "tab_n", "tab_o", "tab_s", "buy_n", "buy_o", "buy_d", "reward_n", "reward_o", "reward_d",
                            "reward_x", "close_n", "close_o", "close_d", "prev_n", "prev_o", "next_n", "next_o", "sel" };
    int ok = 0;
    for (const char* n : names)
    {
        char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\manner_ui\\%s.pus", g_dir, n);
        Sprite s; if (LoadPus(p, s)) { g_skin[n] = s; ok++; }
    }
    char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\manner_ui\\items.txt", g_dir);
    FILE* f = nullptr;
    if (fopen_s(&f, p, "r") == 0 && f)
    {
        char line[256];
        while (fgets(line, sizeof(line), f))
        {
            UINT32 item = 0, icon = 0; char name[160] = {};
            if (sscanf_s(line, "%u|%u|%159[^\r\n]", &item, &icon, name, (unsigned)sizeof(name)) >= 2)
                g_names[item] = { icon, name };
        }
        fclose(f);
    }
    char b[96]; sprintf_s(b, "MANNER: skin %d/%d, items %u", ok, (int)_countof(names), (unsigned)g_names.size()); Log(b);
}

static Sprite* Icon(UINT32 item)
{
    UINT32 icon = 0;
    auto it = g_names.find(item);
    if (it != g_names.end()) icon = it->second.first;
    if (!icon)
    {
        EnterCriticalSection(&g_lock);
        auto t = g_tips.find(item);
        if (t != g_tips.end()) icon = t->second.icon;
        LeaveCriticalSection(&g_lock);
    }
    if (!icon) return nullptr;
    auto ic = g_icons.find(icon);
    if (ic != g_icons.end()) return ic->second.dc ? &ic->second : nullptr;
    char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\manner_ui\\icons\\%u.pus", g_dir, icon);
    Sprite s;
    if (!LoadPus(p, s)) { sprintf_s(p, "%sHopeGuard\\pus_ui\\icons\\%u.pus", g_dir, icon); if (!LoadPus(p, s)) s = Sprite(); }
    g_icons[icon] = s;
    return s.dc ? &g_icons[icon] : nullptr;
}

static std::string ItemName(UINT32 item)
{
    EnterCriticalSection(&g_lock);
    auto t = g_tips.find(item);
    std::string server = t != g_tips.end() ? t->second.name : std::string();
    LeaveCriticalSection(&g_lock);
    if (!server.empty()) return server;
    auto it = g_names.find(item);
    if (it != g_names.end() && !it->second.second.empty()) return it->second.second;
    char b[32]; sprintf_s(b, "Item %u", item); return b;
}

static void Blit(Sprite* s, int x, int y, int w = -1, int h = -1)
{
    if (!s || !s->dc) return;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_memDC, x, y, w < 0 ? s->w : w, h < 0 ? s->h : h, s->dc, 0, 0, s->w, s->h, bf);
}
static bool Skin(const char* n, int x, int y) { auto it = g_skin.find(n); if (it == g_skin.end()) return false; Blit(&it->second, x, y); return true; }

// current drawing target: the panel, or the tooltip while it is painted
static void* g_cvBits = nullptr;
static int g_cvW = MS_W, g_cvH = MS_H;
static void Canvas(void* bits, int w, int h) { g_cvBits = bits; g_cvW = w; g_cvH = h; }

static void Fill(Rc r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(g_cvW, r.r), bb = min(g_cvH, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* d = (BYTE*)g_cvBits + (y * g_cvW + l) * 4;
        for (int x = l; x < rr; x++, d += 4)
        {
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
            d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}

static void Text(const std::string& s, Rc r, COLORREF color, int pt, UINT fmt, bool bold = false)
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
    for (int pass = 0; pass < 2; pass++)
    {
        const int o = pass == 0 ? 1 : 0;
        const BYTE cr = pass ? GetRValue(color) : 0, cg = pass ? GetGValue(color) : 0, cb = pass ? GetBValue(color) : 0;
        for (int y = 0; y < h; y++)
        {
            int dy = r.t + y + o;
            if (dy < 0 || dy >= g_cvH) continue;
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            for (int x = 0; x < w; x++, src += 4)
            {
                int dx = r.l + x + o;
                int a = max(src[0], max(src[1], src[2]));
                if (!a || dx < 0 || dx >= g_cvW) continue;
                if (!pass) a = a * 3 / 4;
                BYTE* d = (BYTE*)g_cvBits + (dy * g_cvW + dx) * 4;
                d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
                d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
                d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
                d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
            }
        }
    }
    SelectObject(dc, of); DeleteObject(font);
    SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}

static std::string Thousands(UINT32 v)
{
    char raw[16]; sprintf_s(raw, "%u", v);
    std::string s = raw, out;
    for (size_t i = 0; i < s.size(); i++)
    {
        if (i && (s.size() - i) % 3 == 0) out += '.';
        out += s[i];
    }
    return out;
}

// ---------------------------------------------------------------- controls
enum Ctl { C_NONE = -1, C_CLOSE = 1, C_PREV, C_NEXT, C_BUY, C_REWARD, C_TAB0 = 10, C_SLOT0 = 20 };   // C_TAB0..+4, C_SLOT0..+15
static int  g_hover = C_NONE, g_pressed = C_NONE;
static bool g_track = false, g_drag = false;
static POINT g_dragFrom;
static bool In(Rc r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }
static const int* const kTabs[5] = { MS_TAB_0, MS_TAB_1, MS_TAB_2, MS_TAB_3, MS_TAB_4 };
static const int* const kSlots[16] = { MS_SLOT_0, MS_SLOT_1, MS_SLOT_2, MS_SLOT_3, MS_SLOT_4, MS_SLOT_5, MS_SLOT_6, MS_SLOT_7,
                                       MS_SLOT_8, MS_SLOT_9, MS_SLOT_10, MS_SLOT_11, MS_SLOT_12, MS_SLOT_13, MS_SLOT_14, MS_SLOT_15 };
static const char* const kTabNames[5] = { "Warrior Store", "Rogue Store", "Mage Store", "Priest Store", "Power Store" };

static int HitTest(int x, int y)
{
    if (In(R4(MS_CLOSE), x, y)) return C_CLOSE;
    if (In(R4(MS_PREV), x, y)) return C_PREV;
    if (In(R4(MS_NEXT), x, y)) return C_NEXT;
    if (In(R4(MS_BUY), x, y)) return C_BUY;
    if (In(R4(MS_REWARD), x, y)) return C_REWARD;
    for (int i = 0; i < 5; i++) if (In(R4(kTabs[i]), x, y)) return C_TAB0 + i;
    for (int i = 0; i < 16; i++) if (In(R4(kSlots[i]), x, y)) return C_SLOT0 + i;
    return C_NONE;
}

static void IconIn(Rc slot, UINT32 item)
{
    if (!item) return;
    Rc in = { slot.l + 4, slot.t + 4, slot.r - 4, slot.b - 4 };
    if (Sprite* s = Icon(item)) Blit(s, in.l, in.t, in.r - in.l, in.b - in.t);
    else Text("?", in, RGB(255, 220, 120), 14, DT_CENTER | DT_VCENTER, true);
}

static const char* Cur(BYTE c) { return c ? "LP" : "MP"; }

// ---------------------------------------------------------------- rendering
static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    Canvas(g_bits, MS_W, MS_H);
    memset(g_bits, 0, (size_t)MS_W * MS_H * 4);
    LoadSkin();
    EnterCriticalSection(&g_lock);
    MsState s = g_st;
    LeaveCriticalSection(&g_lock);
    char b[128];

    if (!Skin("bg", 0, 0)) Fill({ 0, 0, MS_W, MS_H }, 30, 26, 20, 240);
    Text("Special Store [Loyalty & Manner]", R4(MS_TITLE), RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER, true);
    Skin(g_hover == C_CLOSE ? (g_pressed == C_CLOSE ? "close_d" : "close_o") : "close_n", MS_CLOSE[0], MS_CLOSE[1]);

    for (int i = 0; i < 5; i++)
    {
        Rc t = R4(kTabs[i]);
        Skin(i == s.tab ? "tab_s" : g_hover == C_TAB0 + i ? "tab_o" : "tab_n", t.l, t.t);
        Text(kTabNames[i], t, i == s.tab ? RGB(255, 255, 255) : RGB(230, 210, 170), 7, DT_CENTER | DT_VCENTER, true);
    }

    std::vector<int> list = TabItems(s);
    int pages = max(1, ((int)list.size() + PAGE_SLOTS - 1) / PAGE_SLOTS);
    int page = min(s.page, pages - 1);
    for (int k = 0; k < PAGE_SLOTS; k++)
    {
        int at = page * PAGE_SLOTS + k;
        if (at >= (int)list.size()) break;
        const MsItem& it = s.items[list[at]];
        Rc slot = R4(kSlots[k]);
        IconIn(slot, it.item);
        if (it.count > 1)
        {
            sprintf_s(b, "%u", it.count);
            Text(b, { slot.l + 4, slot.b - 18, slot.r - 5, slot.b - 3 }, RGB(255, 255, 255), 7, DT_RIGHT | DT_BOTTOM, true);
        }
        if (list[at] == s.selected) Skin("sel", slot.l, slot.t);
        else if (g_hover == C_SLOT0 + k) Fill({ slot.l + 4, slot.t + 4, slot.r - 4, slot.b - 4 }, 255, 255, 255, 40);
    }
    Skin(g_hover == C_PREV && page > 0 ? "prev_o" : "prev_n", MS_PREV[0], MS_PREV[1]);
    Skin(g_hover == C_NEXT && page + 1 < pages ? "next_o" : "next_n", MS_NEXT[0], MS_NEXT[1]);
    sprintf_s(b, "%d / %d", page + 1, pages);
    Text(b, R4(MS_PAGE), RGB(255, 230, 180), 7, DT_CENTER | DT_VCENTER, true);

    // name bar: hovered slot, else the selected item
    int show = s.selected;
    if (g_hover >= C_SLOT0 && g_hover < C_SLOT0 + 16)
    {
        int at = page * PAGE_SLOTS + (g_hover - C_SLOT0);
        if (at < (int)list.size()) show = list[at];
    }
    if (show >= 0 && show < (int)s.items.size())
        Text(ItemName(s.items[show].item), R4(MS_NAME_BAR), RGB(255, 235, 160), 9, DT_CENTER | DT_VCENTER, true);
    else
        Text("Select an item", R4(MS_NAME_BAR), RGB(200, 190, 210), 8, DT_CENTER | DT_VCENTER);

    // balance
    Text("Your Balance", R4(MS_BAL_TITLE), RGB(255, 210, 120), 8, DT_CENTER | DT_VCENTER, true);
    Text(Thousands(s.manner) + " manner point", R4(MS_BAL_MANNER), RGB(255, 255, 255), 8, DT_LEFT | DT_VCENTER, true);
    Text(Thousands(s.loyalty) + " loyalty point", R4(MS_BAL_LOYALTY), RGB(255, 255, 255), 8, DT_LEFT | DT_VCENTER, true);

    // selected item
    const MsItem* sel = s.selected >= 0 && s.selected < (int)s.items.size() ? &s.items[s.selected] : nullptr;
    if (sel)
    {
        IconIn(R4(MS_SEL_SLOT), sel->item);
        UINT32 have = sel->currency ? s.loyalty : s.manner;
        sprintf_s(b, "%s %s", Thousands(sel->price).c_str(), Cur(sel->currency));
        Text(b, R4(MS_SEL_PRICE), have >= sel->price ? RGB(120, 255, 180) : RGB(255, 130, 110), 8, DT_CENTER | DT_VCENTER, true);
        sprintf_s(b, "Count %u", sel->count);
        Text(b, R4(MS_SEL_COUNT), RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER, true);
        if (sel->days) { sprintf_s(b, "%u days", sel->days); Text(b, R4(MS_SEL_DAYS), RGB(200, 220, 255), 7, DT_CENTER | DT_VCENTER); }
    }
    else
    {
        Text("Price", R4(MS_SEL_PRICE), RGB(160, 150, 130), 8, DT_CENTER | DT_VCENTER, true);
        Text("Count", R4(MS_SEL_COUNT), RGB(160, 150, 130), 8, DT_CENTER | DT_VCENTER, true);
    }
    Rc bb = R4(MS_BUY);
    Skin(!sel || s.busy ? "buy_d" : g_hover == C_BUY ? (g_pressed == C_BUY ? "buy_d" : "buy_o") : "buy_n", bb.l, bb.t);
    Text("BUY ITEM", bb, RGB(255, 240, 220), 8, DT_CENTER | DT_VCENTER, true);

    // daily reward
    UINT32 left = DailyLeft(s);
    sprintf_s(b, "Get Daily Free %u Manner Point", s.dailyPoints);
    Text(b, R4(MS_REWARD_TEXT), RGB(255, 210, 160), 7, DT_CENTER | DT_VCENTER);
    sprintf_s(b, "%02u:%02u:%02u time to get this.", left / 3600, left / 60 % 60, left % 60);
    Text(b, R4(MS_REWARD_TIME), left ? RGB(220, 220, 220) : RGB(140, 255, 140), 7, DT_CENTER | DT_VCENTER);
    Rc rb = R4(MS_REWARD);
    Skin(left || s.busy ? "reward_x" : g_hover == C_REWARD ? (g_pressed == C_REWARD ? "reward_d" : "reward_o") : "reward_n", rb.l, rb.t);
    Text("GET REWARD", rb, left ? RGB(170, 170, 170) : RGB(255, 240, 220), 8, DT_CENTER | DT_VCENTER, true);

    if (!s.msg.empty() && GetTickCount() - s.msgAt < 6000)
        Text(s.msg, R4(MS_MSG), s.msgOk ? RGB(140, 255, 140) : RGB(255, 140, 120), 8, DT_CENTER | DT_VCENTER, true);

    RECT wr; GetWindowRect(g_hWnd, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { MS_W, MS_H };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
}

// ---------------------------------------------------------------- tooltip (separate click-through window)
static COLORREF TipColor(BYTE c)
{
    switch (c)
    {
    case 1: return RGB(120, 235, 120);
    case 2: return RGB(255, 210, 90);
    case 3: return RGB(110, 210, 255);
    case 4: return RGB(255, 100, 90);
    case 5: return RGB(170, 170, 170);
    default: return RGB(235, 235, 235);
    }
}

static void HideTip()
{
    g_tipItem = 0; g_tipSlot = -1;
    if (g_hWndTip) ShowWindow(g_hWndTip, SW_HIDE);
}

// stats of the hovered slot next to it (flipped left near the screen edge); asks the server once per item
static void UpdateTip(bool force = false)
{
    if (!g_hWndTip || !g_memDCTip || !g_open || !IsWindowVisible(g_hWnd)) { HideTip(); return; }
    int slot = g_hover >= C_SLOT0 && g_hover < C_SLOT0 + 16 ? g_hover - C_SLOT0 : -1;
    UINT32 item = 0, price = 0, days = 0; UINT16 count = 1; BYTE currency = 0;
    TipData tip; bool have = false;
    EnterCriticalSection(&g_lock);
    if (slot >= 0)
    {
        std::vector<int> list = TabItems(g_st);
        int pages = max(1, ((int)list.size() + PAGE_SLOTS - 1) / PAGE_SLOTS);
        int at = min(g_st.page, pages - 1) * PAGE_SLOTS + slot;
        if (at < (int)list.size())
        {
            const MsItem& it = g_st.items[list[at]];
            item = it.item; count = it.count; price = it.price; currency = it.currency; days = it.days;
        }
    }
    auto t = g_tips.find(item);
    if (t != g_tips.end()) { tip = t->second; have = true; }
    LeaveCriticalSection(&g_lock);
    if (!item) { HideTip(); return; }
    if (!have)
    {
        DWORD now = GetTickCount();
        auto a = g_tipAsked.find(item);
        if (a == g_tipAsked.end() || now - a->second > 5000)
        {
            g_tipAsked[item] = now;
            QueueSend({ WIZ_HSACS_HOOK, MS_SUBOP, 4, (BYTE)item, (BYTE)(item >> 8), (BYTE)(item >> 16), (BYTE)(item >> 24) });
        }
    }
    if (!force && item == g_tipItem && slot == g_tipSlot && IsWindowVisible(g_hWndTip)) return;
    g_tipItem = item; g_tipSlot = slot;

    const int pad = 10, titleH = 22, lineH = 17;
    std::vector<std::pair<BYTE, std::string>> lines = tip.lines;
    char b[96];
    sprintf_s(b, "Price : %u %s", price, currency ? "loyalty point" : "manner point");
    lines.push_back({ 2, b });
    if (days) { sprintf_s(b, "Duration : %u days", days); lines.push_back({ 3, b }); }
    int h = min(TIP_MAXH, pad + titleH + 8 + (int)lines.size() * lineH + (have ? 0 : lineH) + pad);
    memset(g_bitsTip, 0, (size_t)TIP_W * TIP_MAXH * 4);
    Canvas(g_bitsTip, TIP_W, h);
    Fill({ 0, 0, TIP_W, h }, 12, 12, 16, 235);
    Fill({ 0, 0, TIP_W, 1 }, 150, 120, 60, 255); Fill({ 0, h - 1, TIP_W, h }, 150, 120, 60, 255);
    Fill({ 0, 0, 1, h }, 150, 120, 60, 255); Fill({ TIP_W - 1, 0, TIP_W, h }, 150, 120, 60, 255);
    std::string title = ItemName(item);
    if (count > 1) title += "  x" + std::to_string(count);
    Text(title, { pad, pad, TIP_W - pad, pad + titleH }, RGB(255, 210, 90), 10, DT_CENTER | DT_VCENTER, true);
    Fill({ pad, pad + titleH + 3, TIP_W - pad, pad + titleH + 4 }, 150, 120, 60, 255);
    int y = pad + titleH + 8;
    if (!have) { Text("Loading...", { pad, y, TIP_W - pad, y + lineH }, RGB(170, 170, 170), 9, DT_CENTER | DT_VCENTER); y += lineH; }
    for (auto& l : lines)
    {
        if (y + lineH > h - pad + 2) break;
        Text(l.second, { pad, y, TIP_W - pad, y + lineH }, TipColor(l.first), 9, DT_CENTER | DT_VCENTER);
        y += lineH;
    }
    Canvas(g_bits, MS_W, MS_H);

    RECT wr; GetWindowRect(g_hWnd, &wr);
    Rc s = R4(kSlots[slot]);
    int x = wr.left + s.r + 8, ty = wr.top + s.t;
    MONITORINFO mi = { sizeof(mi) }; GetMonitorInfo(MonitorFromWindow(g_hWnd, MONITOR_DEFAULTTONEAREST), &mi);
    if (x + TIP_W > mi.rcWork.right) x = wr.left + s.l - 8 - TIP_W;
    if (ty + h > mi.rcWork.bottom) ty = mi.rcWork.bottom - h;
    if (ty < mi.rcWork.top) ty = mi.rcWork.top;
    POINT dst = { x, ty }, src = { 0, 0 };
    SIZE sz = { TIP_W, h };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWndTip, nullptr, &dst, &sz, g_memDCTip, &src, 0, &bf, ULW_ALPHA);
    SetWindowPos(g_hWndTip, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
}

// ---------------------------------------------------------------- window plumbing
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

static void Place()
{
    RECT gr = { 0, 0, 1024, 768 };
    HWND game = FindGameWindow();
    if (game) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
    if (g_panelPos.x < 0)
    {
        g_panelPos.x = gr.left + ((gr.right - gr.left) - MS_W) / 2;
        g_panelPos.y = gr.top + ((gr.bottom - gr.top) - MS_H) / 2;
    }
    SetWindowPos(g_hWnd, HWND_TOPMOST, g_panelPos.x, g_panelPos.y, MS_W, MS_H, SWP_NOACTIVATE);
}

static void SetOpen(bool open)
{
    g_open = open;
    g_hover = C_NONE;
    HideTip();
    if (!open) { ShowWindow(g_hWnd, SW_HIDE); return; }
    Place();
    Paint();
}

static void SyncVisibility()
{
    HWND game = FindGameWindow();
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool want = g_open && game && !IsIconic(game) && pid == GetCurrentProcessId();
    if (want != (IsWindowVisible(g_hWnd) != FALSE))
    {
        if (want) { SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); Paint(); }
        else { ShowWindow(g_hWnd, SW_HIDE); HideTip(); }
    }
    POINT p; CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) && (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursor = ci.hCursor;
    static DWORD lastSec = 0;
    if (want && GetTickCount() - lastSec >= 1000) { lastSec = GetTickCount(); Paint(); }
}

static void SetMsg(MsState& s, const char* m, bool ok) { s.msg = m; s.msgOk = ok; s.msgAt = GetTickCount(); }

static void Activate(int ctl)
{
    EnterCriticalSection(&g_lock);
    MsState& s = g_st;
    std::vector<int> list = TabItems(s);
    int pages = max(1, ((int)list.size() + PAGE_SLOTS - 1) / PAGE_SLOTS);
    std::vector<BYTE> pkt;
    if (ctl == C_CLOSE) { LeaveCriticalSection(&g_lock); SetOpen(false); return; }
    else if (ctl >= C_TAB0 && ctl < C_TAB0 + 5) { s.tab = ctl - C_TAB0; s.page = 0; }
    else if (ctl == C_PREV) { if (s.page > 0) s.page--; }
    else if (ctl == C_NEXT) { if (s.page + 1 < pages) s.page++; }
    else if (ctl >= C_SLOT0 && ctl < C_SLOT0 + 16)
    {
        int at = min(s.page, pages - 1) * PAGE_SLOTS + (ctl - C_SLOT0);
        if (at < (int)list.size()) s.selected = list[at];
    }
    else if (ctl == C_BUY && !s.busy && s.selected >= 0 && s.selected < (int)s.items.size())
    {
        const MsItem& it = s.items[s.selected];
        if ((it.currency ? s.loyalty : s.manner) < it.price) SetMsg(s, it.currency ? "Not enough loyalty points." : "Not enough manner points.", false);
        else { pkt = { WIZ_HSACS_HOOK, MS_SUBOP, 2, (BYTE)(it.index & 0xFF), (BYTE)(it.index >> 8) }; s.busy = true; }
    }
    else if (ctl == C_REWARD && !s.busy && DailyLeft(s) == 0) { pkt = { WIZ_HSACS_HOOK, MS_SUBOP, 3 }; s.busy = true; }
    LeaveCriticalSection(&g_lock);
    if (!pkt.empty()) QueueSend(pkt);
}

static LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
    switch (msg)
    {
    case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
    case WM_MOUSEMOVE:
        if (g_drag)
        {
            POINT p; GetCursorPos(&p);
            if (p.x != g_dragFrom.x || p.y != g_dragFrom.y)
            {
                RECT wr; GetWindowRect(h, &wr);
                SetWindowPos(h, nullptr, wr.left + p.x - g_dragFrom.x, wr.top + p.y - g_dragFrom.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                g_dragFrom = p;
            }
            return 0;
        }
        if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
        if (int hit = HitTest(x, y); hit != g_hover) { g_hover = hit; Paint(); UpdateTip(); }
        return 0;
    case WM_MOUSELEAVE:
        g_track = false;
        if (g_hover != C_NONE) { g_hover = C_NONE; Paint(); }
        HideTip();
        return 0;
    case WM_LBUTTONDOWN:
    {
        int hit = HitTest(x, y);
        if (hit == C_NONE && y < MS_TAB_0[1])                // title area drags the window
        {
            g_drag = true; GetCursorPos(&g_dragFrom); SetCapture(h); return 0;
        }
        g_pressed = hit;
        if (hit != C_NONE) { SetCapture(h); Paint(); }
        return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (g_drag)
        {
            g_drag = false;
            RECT wr; GetWindowRect(h, &wr);
            g_panelPos.x = wr.left; g_panelPos.y = wr.top;
            return 0;
        }
        int hit = HitTest(x, y), pressed = g_pressed;
        g_pressed = C_NONE;
        if (pressed != C_NONE && pressed == hit) Activate(hit);
        Paint();
        UpdateTip(true);
        return 0;
    }
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: SyncVisibility(); return 0;
    case WM_MS_REFRESH:
        if (wp == 1 && !g_open) SetOpen(true);
        else if (IsWindowVisible(h)) { Paint(); if (wp == 2) UpdateTip(true); }
        return 0;
    case WM_CLOSE: SetOpen(false); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThread(LPVOID)
{
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = L"NTT_MannerStore";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Special Store", WS_POPUP, 0, 0, MS_W, MS_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("MANNER: window creation failed"); return 0; }
    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), MS_W, -MS_H, 1, 32, BI_RGB };
    HBITMAP bmp = CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0);
    SelectObject(g_memDC, bmp);

    WNDCLASSEXW tc = { sizeof(tc) };
    tc.lpfnWndProc = DefWindowProcW;
    tc.hInstance = wc.hInstance;
    tc.lpszClassName = L"NTT_MannerTip";
    RegisterClassExW(&tc);
    g_hWndTip = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        tc.lpszClassName, L"Item Info", WS_POPUP, 0, 0, TIP_W, 100, nullptr, nullptr, tc.hInstance, nullptr);
    g_memDCTip = CreateCompatibleDC(nullptr);
    BITMAPINFO tb = {};
    tb.bmiHeader = { sizeof(BITMAPINFOHEADER), TIP_W, -TIP_MAXH, 1, 32, BI_RGB };
    SelectObject(g_memDCTip, CreateDIBSection(g_memDCTip, &tb, DIB_RGB_COLORS, &g_bitsTip, nullptr, 0));
    Log("MANNER: window ready (event menu)");
    SetTimer(g_hWnd, 1, 150, nullptr);
    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// ---------------------------------------------------------------- recv (game thread)
template <class T> static bool Get(const BYTE* p, size_t len, size_t& pos, T& v)
{
    if (pos + sizeof(T) > len) return false;
    memcpy(&v, p + pos, sizeof(T)); pos += sizeof(T); return true;
}

void MannerStore_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[0] != WIZ_HSACS_HOOK || buf[1] != MS_SUBOP) return;
    size_t pos = 3;
    const BYTE op = buf[2];
    WPARAM open = 0;
    char msg[160] = {};
    EnterCriticalSection(&g_lock);
    MsState& s = g_st;
    if (op == 1)
    {
        UINT32 manner = 0, loyalty = 0, left = 0; UINT16 points = 0, n = 0;
        if (Get(buf, len, pos, manner) && Get(buf, len, pos, loyalty) && Get(buf, len, pos, left) && Get(buf, len, pos, points) && Get(buf, len, pos, n))
        {
            std::vector<MsItem> items;
            for (UINT16 i = 0; i < n; i++)
            {
                MsItem it = {};
                if (!(Get(buf, len, pos, it.index) && Get(buf, len, pos, it.tab) && Get(buf, len, pos, it.item) && Get(buf, len, pos, it.count) &&
                      Get(buf, len, pos, it.price) && Get(buf, len, pos, it.currency) && Get(buf, len, pos, it.days))) break;
                if (it.tab > 4) it.tab = 4;
                items.push_back(it);
            }
            UINT16 keepIndex = s.selected >= 0 && s.selected < (int)s.items.size() ? s.items[s.selected].index : 0xFFFF;
            s.items = items; s.selected = -1;
            for (int i = 0; i < (int)items.size(); i++) if (items[i].index == keepIndex) s.selected = i;
            s.manner = manner; s.loyalty = loyalty; s.dailyLeft = left; s.dailyAt = GetTickCount(); s.dailyPoints = points;
            s.busy = false;
            open = 1;
            sprintf_s(msg, "MANNER recv: store %u items, manner=%u loyalty=%u daily=%us", (unsigned)items.size(), manner, loyalty, left);
        }
    }
    else if (op == 2)
    {
        BYTE r = 0; UINT32 manner = 0, loyalty = 0;
        if (Get(buf, len, pos, r) && Get(buf, len, pos, manner) && Get(buf, len, pos, loyalty))
        {
            s.manner = manner; s.loyalty = loyalty; s.busy = false;
            const char* text = r == 1 ? "Item purchased." : r == 2 ? "Not enough points." : r == 3 ? "Your inventory is full." : "This item can not be bought.";
            SetMsg(s, text, r == 1);
            sprintf_s(msg, "MANNER recv: buy result %u", r);
        }
    }
    else if (op == 3)
    {
        BYTE r = 0; UINT32 manner = 0, left = 0;
        if (Get(buf, len, pos, r) && Get(buf, len, pos, manner) && Get(buf, len, pos, left))
        {
            s.manner = manner; s.dailyLeft = left; s.dailyAt = GetTickCount(); s.busy = false;
            char t[96]; sprintf_s(t, "You received %u manner points.", s.dailyPoints);
            SetMsg(s, r == 1 ? t : "The reward is not ready yet.", r == 1);
            sprintf_s(msg, "MANNER recv: daily result %u", r);
        }
    }
    else if (op == 4)
    {
        UINT32 item = 0; UINT16 n = 0; BYTE cnt = 0; TipData t;
        auto str = [&](std::string& out) { if (!Get(buf, len, pos, n) || pos + n > len) return false; out.assign((const char*)buf + pos, n); pos += n; return true; };
        if (Get(buf, len, pos, item) && str(t.name) && Get(buf, len, pos, t.icon) && Get(buf, len, pos, cnt))
        {
            for (BYTE k = 0; k < cnt; k++)
            {
                BYTE c = 0; std::string line;
                if (!Get(buf, len, pos, c) || !str(line)) break;
                t.lines.push_back({ c, line });
            }
            g_tips[item] = t;
            open = 2;
        }
    }
    LeaveCriticalSection(&g_lock);
    if (msg[0]) Log(msg);
    if (g_hWnd) PostMessageW(g_hWnd, WM_MS_REFRESH, open, 0);
}

// ---------------------------------------------------------------- exports
// game thread: event menu entry -> ask the server (the answer opens the window)
void MannerStore_Open()
{
    if (g_open) return;
    QueueSend({ WIZ_HSACS_HOOK, MS_SUBOP, 1 });
}

void MannerStore_Toggle()
{
    if (g_open) { if (g_hWnd) PostMessageW(g_hWnd, WM_CLOSE, 0, 0); return; }
    QueueSend({ WIZ_HSACS_HOOK, MS_SUBOP, 1 });
}

void MannerStore_Init()
{
    InitializeCriticalSection(&g_lock);
    InitializeCriticalSection(&g_sendLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

bool MannerStore_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
