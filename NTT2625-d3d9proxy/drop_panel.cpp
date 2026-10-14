// Drop viewer for the 2625 client (d3d9 proxy GDI layered window, sibling of pus_store / rce_store).
// Server: XGuard.cpp DropView2625 (WIZ_HSACS_HOOK 0xE9 + DROP_REQUEST 0xAE).
//   C->S  [E9 AE 01 u32 npcId (0 = current target)] | [E9 AE 03 u16 protoId] | [E9 AE 05 u16 len str query]
//   S->C  [E9 AE 01 u16 proto str name u16 level u8 isMonster u16 n] n*[u32 item u16 pct(1/10000) u32 icon str name]
//         [E9 AE 05 u16 n] n*[u16 proto str name u16 level u8 hasDrops]
//   block list: S->C [E9 AE 09 str charName] (game start), C->S [E9 AE 09 u8 n] n*[u32 item]
// Open: '\' or Ctrl+D in game (d3d9proxy.cpp window hook). "Target" shows the selected monster's drops,
// typing a name + Enter searches monsters by name; click a result to see its drops.
#include <windows.h>
#include "drop_shared.h"
#include "drop_layout.h"
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <map>
#include <deque>
#include <algorithm>
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();
UINT32 DropBtn_CurrentTargetId();

static const DWORD KO_SND_FNC = 0x00704070;   // thiscall(CAPISocket*, BYTE* buf, int len)
static const DWORD KO_PTR_PKT = 0x01115914;   // CAPISocket* (game socket)
static const BYTE  WIZ_HSACS_HOOK = 0xE9, DROP_SUBOP = 0xAE;

// ---------------------------------------------------------------- layout (drop_layout.h, from re_search_monster_drop.uif)
static const int W = DP_W, H = DP_H;
struct DR { int l, t, r, b; };
static DR D4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }
static const int ROW_H = 22;
static const int LIST_ROWS = (DP_LIST[3] - DP_LIST[1]) / ROW_H;     // monster rows
static const int GRID_N = 25;                                       // drops per page (5 x 5)

// ---------------------------------------------------------------- data
struct Mob  { UINT16 proto; std::string name; UINT16 level; BYTE hasDrops; };
struct Drop { UINT32 item; UINT16 pct; UINT32 icon; std::string name; };
static CRITICAL_SECTION g_lock;
static std::vector<Mob>  g_mobs;
static std::vector<Drop> g_drops;
static std::string g_dropTitle, g_status = "Type a monster name and press Enter, or press Target.";
static std::string g_query;
static int g_selMob = -1, g_listTop = 0, g_gridPage = 0, g_hoverDrop = -1;
static DWORD g_statusAt = 0;
static void SetStatus(const std::string& s);

// ---------------------------------------------------------------- send queue (flushed on the game thread)
static CRITICAL_SECTION g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQueue;
static void QueueSend(const std::vector<BYTE>& p)
{
    EnterCriticalSection(&g_sendLock); g_sendQueue.push_back(p); LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();
}
void Drop_FlushSendQueue()
{
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_sendLock); pending.swap(g_sendQueue); LeaveCriticalSection(&g_sendLock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
    }
}
static void RequestTarget()
{
    UINT32 targetId = DropBtn_CurrentTargetId();
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, DROP_SUBOP, 1 };
    p.insert(p.end(), (BYTE*)&targetId, (BYTE*)&targetId + sizeof(targetId));
    QueueSend(p);
}
// drop_button.cpp (chest next to the target bar): the target's drops go to the compact list, not to this panel
static volatile LONG g_miniPending = 0;
void Drop_RequestMini(UINT32 npcId)
{
    InterlockedExchange(&g_miniPending, 1);
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, DROP_SUBOP, 1 };
    p.insert(p.end(), (BYTE*)&npcId, (BYTE*)&npcId + 4);   // the client's own target id (0 = the server's idea of it)
    QueueSend(p);
}
static void RequestProto(UINT16 proto)
{
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, DROP_SUBOP, 3, (BYTE)(proto & 0xFF), (BYTE)(proto >> 8) };
    QueueSend(p);
}
static void RequestSearch(const std::string& q)
{
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, DROP_SUBOP, 5, (BYTE)(q.size() & 0xFF), (BYTE)(q.size() >> 8) };
    p.insert(p.end(), q.begin(), q.end());
    QueueSend(p);
}

// ---------------------------------------------------------------- drop block list
// Right click on a drop (here or in the compact list) = that item no longer drops for this character (solo kills;
// the auto looter skips it in a party). At most 10. Kept per character in JstKO\dropblock\<name>.txt
// ("item|icon|name" lines); the server keeps the list for the session only and asks for it at game start (E9 AE 09).
static const size_t BLOCK_MAX = 10;
static CRITICAL_SECTION g_blockLock;
static std::vector<DropMiniEntry> g_blocked;
static std::string g_blockChar;
static bool g_showBlocked = false;      // the grid shows the block list instead of a monster's drops
static const UINT WM_DROP_REFRESH = WM_APP + 31;

static std::string BlockFile()
{
    char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH);
    strcpy_s(strrchr(dir, '\\') + 1, 32, "JstKO\\dropblock");
    CreateDirectoryA(dir, nullptr);
    std::string name;
    for (char c : g_blockChar) if (isalnum((unsigned char)c) || c == '_') name.push_back(c);
    return std::string(dir) + "\\" + (name.empty() ? "default" : name) + ".txt";
}
static void BlockLoad()
{
    g_blocked.clear();
    FILE* f = nullptr;
    if (fopen_s(&f, BlockFile().c_str(), "r") != 0 || !f) return;
    char line[256];
    while (fgets(line, sizeof(line), f) && g_blocked.size() < BLOCK_MAX)
    {
        DropMiniEntry e = {}; char name[160] = {};
        if (sscanf_s(line, "%u|%u|%159[^\r\n]", &e.item, &e.icon, name, (unsigned)sizeof(name)) < 2 || !e.item) continue;
        e.name = name;
        g_blocked.push_back(e);
    }
    fclose(f);
}
static void BlockSave()
{
    FILE* f = nullptr;
    if (fopen_s(&f, BlockFile().c_str(), "w") != 0 || !f) return;
    for (auto& e : g_blocked) fprintf(f, "%u|%u|%s\n", e.item, e.icon, e.name.c_str());
    fclose(f);
}
static void BlockSend()
{
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, DROP_SUBOP, 9, (BYTE)g_blocked.size() };
    for (auto& e : g_blocked) p.insert(p.end(), (BYTE*)&e.item, (BYTE*)&e.item + 4);
    QueueSend(p);
}
bool DropBlock_Has(UINT32 item)
{
    EnterCriticalSection(&g_blockLock);
    bool has = false;
    for (auto& e : g_blocked) if (e.item == item) { has = true; break; }
    LeaveCriticalSection(&g_blockLock);
    return has;
}
int DropBlock_Count()
{
    EnterCriticalSection(&g_blockLock); int n = (int)g_blocked.size(); LeaveCriticalSection(&g_blockLock);
    return n;
}
static HWND g_hWnd;
int DropBlock_Toggle(const DropMiniEntry& e)
{
    int res = -1;
    EnterCriticalSection(&g_blockLock);
    auto it = std::find_if(g_blocked.begin(), g_blocked.end(), [&](const DropMiniEntry& b) { return b.item == e.item; });
    if (it != g_blocked.end()) { g_blocked.erase(it); res = 0; }
    else if (g_blocked.size() < BLOCK_MAX) { g_blocked.push_back({ e.item, 0, e.icon, e.name }); res = 1; }
    if (res >= 0) { BlockSave(); BlockSend(); }
    LeaveCriticalSection(&g_blockLock);
    if (g_hWnd) PostMessageW(g_hWnd, WM_DROP_REFRESH, 0, 0);
    return res;
}

// ---------------------------------------------------------------- reader
struct Reader
{
    const BYTE* p; size_t len, pos; bool ok = true;
    template <class T> T get() { T v{}; if (pos + sizeof(T) > len) { ok = false; return v; } memcpy(&v, p + pos, sizeof(T)); pos += sizeof(T); return v; }
    std::string str() { UINT16 n = get<UINT16>(); if (!ok || pos + n > len) { ok = false; return {}; } std::string s((const char*)p + pos, n); pos += n; return s; }
};

// ---------------------------------------------------------------- drawing
static HDC   g_memDC = nullptr;
static void* g_bits = nullptr;
static char  g_iconDir[MAX_PATH];
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<UINT32, Sprite> g_icons;
static Sprite g_noImage; static bool g_noImageTried = false;

static bool LoadPus(const char* path, Sprite& s)
{
    FILE* f = nullptr;
    if (fopen_s(&f, path, "rb") != 0 || !f) return false;
    char magic[4]; UINT32 w = 0, h = 0;
    bool ok = fread(magic, 1, 4, f) == 4 && memcmp(magic, "PUSI", 4) == 0 && fread(&w, 4, 1, f) == 1 && fread(&h, 4, 1, f) == 1 && w && h && w < 4096 && h < 4096;
    if (ok)
    {
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), (LONG)w, -(LONG)h, 1, 32, BI_RGB };
        void* bits = nullptr;
        s.dc = CreateCompatibleDC(nullptr);
        s.bmp = CreateDIBSection(s.dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
        ok = s.bmp && fread(bits, 4, (size_t)w * h, f) == (size_t)w * h;
        SelectObject(s.dc, s.bmp); s.w = (int)w; s.h = (int)h;
    }
    fclose(f);
    return ok;
}
static Sprite* Icon(UINT32 id)
{
    if (!g_noImageTried) { g_noImageTried = true; char p[MAX_PATH]; sprintf_s(p, "%s\\..\\noimage.pus", g_iconDir); if (!LoadPus(p, g_noImage)) g_noImage = Sprite(); }
    if (!id) return g_noImage.dc ? &g_noImage : nullptr;
    auto it = g_icons.find(id);
    if (it == g_icons.end())
    {
        Sprite s; char p[MAX_PATH]; sprintf_s(p, "%s\\%u.pus", g_iconDir, id);
        if (!LoadPus(p, s)) s = Sprite();
        it = g_icons.emplace(id, s).first;
    }
    return it->second.dc ? &it->second : (g_noImage.dc ? &g_noImage : nullptr);
}
static void Fill(DR r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(W, r.r), bb = min(H, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* d = (BYTE*)g_bits + (y * W + l) * 4;
        for (int x = l; x < rr; x++, d += 4)
        {
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}
static void Border(DR r, BYTE cr, BYTE cg, BYTE cb, int th = 1)
{
    Fill({ r.l, r.t, r.r, r.t + th }, cr, cg, cb, 255); Fill({ r.l, r.b - th, r.r, r.b }, cr, cg, cb, 255);
    Fill({ r.l, r.t, r.l + th, r.b }, cr, cg, cb, 255); Fill({ r.r - th, r.t, r.r, r.b }, cr, cg, cb, 255);
}
static void Text(const std::string& s, DR r, COLORREF color, int pt, UINT fmt, bool bold = false)
{
    int w = r.r - r.l, h = r.b - r.t;
    if (s.empty() || w <= 0 || h <= 0) return;
    BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
    void* bits = nullptr;
    HDC dc = CreateCompatibleDC(nullptr);
    HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    HGDIOBJ ob = SelectObject(dc, bmp);
    HFONT font = CreateFontA(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
    HGDIOBJ of = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
    RECT rc = { 0, 0, w, h };
    DrawTextA(dc, s.c_str(), (int)s.size(), &rc, fmt | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    GdiFlush();
    BYTE cr = GetRValue(color), cg = GetGValue(color), cb = GetBValue(color);
    for (int y = 0; y < h; y++)
    {
        int dy = r.t + y; if (dy < 0 || dy >= H) continue;
        const BYTE* src = (const BYTE*)bits + y * w * 4;
        BYTE* d = (BYTE*)g_bits + (dy * W + r.l) * 4;
        for (int x = 0; x < w; x++, src += 4, d += 4)
        {
            int a = max(src[0], max(src[1], src[2]));
            if (!a || r.l + x < 0 || r.l + x >= W) continue;
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255); d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255); d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
    SelectObject(dc, of); DeleteObject(font); SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}
static void Blit(Sprite* s, int x, int y, int w, int h)
{
    if (!s) return;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_memDC, x, y, w, h, s->dc, 0, 0, s->w, s->h, bf);
}

// ---------------------------------------------------------------- skin
static std::map<std::string, Sprite> g_skin;
static bool g_skinLoaded = false;
static void LoadSkin()
{
    if (g_skinLoaded) return;
    g_skinLoaded = true;
    char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH); *(strrchr(dir, '\\') + 1) = 0;
    const char* base[] = { "next", "prev", "left", "right", "close", "search", "save", "reset" };
    int ok = 0;
    std::vector<std::string> names = { "bg" };
    for (const char* b : base) for (const char* k : { "n", "d", "o", "x" }) names.push_back(std::string(b) + "_" + k);
    for (auto& n : names)
    {
        char p[MAX_PATH]; sprintf_s(p, "%sJstKO\\drop_ui\\%s.pus", dir, n.c_str());
        Sprite s; if (LoadPus(p, s)) { g_skin[n] = s; ok++; }
    }
    char b[64]; sprintf_s(b, "DROP: skin %d/%d", ok, (int)names.size()); Log(b);
}
static bool SkinBlit(const std::string& n, int x, int y, int w = -1, int h = -1)
{
    auto it = g_skin.find(n);
    if (it == g_skin.end()) return false;
    Blit(&it->second, x, y, w < 0 ? it->second.w : w, h < 0 ? it->second.h : h);
    return true;
}

// ---------------------------------------------------------------- controls
enum { C_NONE, C_CLOSE, C_EDIT, C_SEARCH, C_PREV, C_NEXT, C_SAVE, C_RESET, C_ROW0 = 100, C_CELL0 = 300, C_BSLOT0 = 500 };
static int g_hover = C_NONE, g_pressed = C_NONE;
static bool g_dragging = false, g_tracking = false, g_searchFocus = false, g_open = false, g_placed = false;
static POINT g_dragFrom;
static HCURSOR g_gameCursor = nullptr;

static bool In(DR r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }
static int GridPages() { return max(1, ((int)g_drops.size() + GRID_N - 1) / GRID_N); }

static int HitTest(int x, int y)
{
    if (In(D4(DP_CLOSE), x, y)) return C_CLOSE;
    if (In(D4(DP_EDIT), x, y)) return C_EDIT;
    if (In(D4(DP_SEARCH), x, y)) return C_SEARCH;
    if (In(D4(DP_PREV), x, y)) return C_PREV;
    if (In(D4(DP_NEXT), x, y)) return C_NEXT;
    if (In(D4(DP_SAVE), x, y)) return C_SAVE;
    if (In(D4(DP_RESET), x, y)) return C_RESET;
    if (In(D4(DP_LIST), x, y))
    {
        int i = g_listTop + (y - DP_LIST[1]) / ROW_H;
        if (i < (int)g_mobs.size()) return C_ROW0 + i;
    }
    for (int v = 0; v < GRID_N; v++)
    {
        int i = g_gridPage * GRID_N + v;
        if (i >= (int)g_drops.size()) break;
        if (In(D4(DP_CELL[v]), x, y)) return C_CELL0 + i;
    }
    for (int k = 0; k < 10; k++) if (In(D4(DP_BSLOT[k]), x, y)) return C_BSLOT0 + k;
    return C_NONE;
}

// "%100" / "%2.5": the design prints the percent sign in front
static std::string PctText(UINT16 pct)
{
    char b[24];
    if (pct >= 10000) return "%100";
    if (pct % 100 == 0) sprintf_s(b, "%%%u", pct / 100); else sprintf_s(b, "%%%.2f", pct / 100.0);
    return b;
}

static void DrawButton(const char* base, const int rc[4], int ctl, const char* label = nullptr)
{
    std::string st = g_hover == ctl ? (g_pressed == ctl ? "d" : "o") : "n";
    if (!SkinBlit(std::string(base) + "_" + st, rc[0], rc[1], rc[2] - rc[0], rc[3] - rc[1]))
        Fill(D4(rc), 70, 40, 30, 255);
    if (label) Text(label, D4(rc), RGB(255, 255, 255), 9, DT_CENTER | DT_VCENTER, true);
}

static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    LoadSkin();
    memset(g_bits, 0, (size_t)W * H * 4);
    if (!SkinBlit("bg", 0, 0)) Fill({ 0, 0, W, H }, 16, 24, 22, 240);

    EnterCriticalSection(&g_lock);
    // search box
    DR edit = D4(DP_EDIT);
    std::string shown = g_query.empty() && !g_searchFocus ? "type here..." : g_query + (g_searchFocus && (GetTickCount() / 500) % 2 ? "|" : "");
    Text(shown, { edit.l + 8, edit.t, edit.r - 4, edit.b }, g_query.empty() && !g_searchFocus ? RGB(120, 190, 190) : RGB(180, 255, 255), 9, DT_LEFT | DT_VCENTER);
    DrawButton("search", DP_SEARCH, C_SEARCH, "Search");
    DrawButton("close", DP_CLOSE, C_CLOSE);

    // monster list
    for (int r = 0; r < LIST_ROWS; r++)
    {
        int i = g_listTop + r;
        if (i >= (int)g_mobs.size()) break;
        const Mob& m = g_mobs[i];
        DR row = { DP_LIST[0] + 2, DP_LIST[1] + r * ROW_H, DP_LIST[2] - 14, DP_LIST[1] + r * ROW_H + ROW_H - 1 };
        if (i == g_selMob) Fill(row, 20, 120, 120, 160);
        else if (g_hover == C_ROW0 + i) Fill(row, 20, 80, 80, 140);
        char lv[16]; sprintf_s(lv, "Lv%u", m.level);
        Text(m.name, { row.l + 4, row.t, row.r - 36, row.b }, m.hasDrops ? RGB(235, 235, 235) : RGB(130, 130, 130), 8, DT_LEFT | DT_VCENTER);
        Text(lv, { row.r - 36, row.t, row.r - 2, row.b }, RGB(255, 210, 90), 8, DT_RIGHT | DT_VCENTER);
    }

    // title of the shown monster (or the last message)
    std::string head = (GetTickCount() - g_statusAt < 4000 && !g_status.empty()) ? g_status : g_dropTitle;
    Text(head, D4(DP_MONNAME), RGB(255, 225, 120), 8, DT_LEFT | DT_VCENTER, true);

    // drop grid (page of 25)
    int pages = GridPages();
    if (g_gridPage >= pages) g_gridPage = pages - 1;
    for (int v = 0; v < GRID_N; v++)
    {
        int i = g_gridPage * GRID_N + v;
        if (i >= (int)g_drops.size()) break;
        const Drop& d = g_drops[i];
        DR c = D4(DP_CELL[v]);
        Blit(Icon(d.icon), c.l, c.t, c.r - c.l, c.b - c.t);
        bool blocked = DropBlock_Has(d.item);
        if (blocked) Fill(c, 150, 0, 0, 150);
        if (g_hover == C_CELL0 + i) Border({ c.l - 1, c.t - 1, c.r + 1, c.b + 1 }, 255, 235, 120, 2);
        Text(blocked ? "BLOCK" : PctText(d.pct), D4(DP_PCT[v]), blocked ? RGB(255, 110, 110) : RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER, true);
    }
    DrawButton("prev", DP_PREV, C_PREV);
    DrawButton("next", DP_NEXT, C_NEXT);
    { char b[32]; sprintf_s(b, "%d / %d", g_gridPage + 1, pages); Text(b, D4(DP_PAGE), RGB(255, 235, 150), 8, DT_CENTER | DT_VCENTER, true); }

    // block list (slots, page text, reset, save)
    EnterCriticalSection(&g_blockLock);
    for (int k = 0; k < 10 && k < (int)g_blocked.size(); k++)
    {
        DR c = D4(DP_BSLOT[k]);
        Blit(Icon(g_blocked[k].icon), c.l, c.t, c.r - c.l, c.b - c.t);
        if (g_hover == C_BSLOT0 + k) Border({ c.l - 1, c.t - 1, c.r + 1, c.b + 1 }, 255, 235, 120, 2);
    }
    LeaveCriticalSection(&g_blockLock);
    DrawButton("left", DP_BLEFT, C_NONE);
    DrawButton("right", DP_BRIGHT, C_NONE);
    Text("1 / 1", D4(DP_BPAGE), RGB(255, 235, 150), 8, DT_CENTER | DT_VCENTER, true);
    DrawButton("reset", DP_RESET, C_RESET);
    DrawButton("save", DP_SAVE, C_SAVE, "Save");
    LeaveCriticalSection(&g_lock);

    RECT wr; GetWindowRect(g_hWnd, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { W, H };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
}

// ---------------------------------------------------------------- window plumbing
static HWND FindGameWindow()
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
static void SetSearchFocus(bool on)
{
    if (g_searchFocus == on) return;
    g_searchFocus = on;
    if (on) SetForegroundWindow(g_hWnd);
    else if (HWND game = FindGameWindow()) SetForegroundWindow(game);
}
static void Hide()
{
    SetSearchFocus(false);
    g_open = false;
    ShowWindow(g_hWnd, SW_HIDE);
    if (HWND game = FindGameWindow()) SetForegroundWindow(game);
}
static void Show(bool loadTarget)
{
    g_open = true;
    if (!g_placed)
    {
        g_placed = true;
        RECT gr = { 0, 0, W, H };
        if (HWND game = FindGameWindow()) GetWindowRect(game, &gr);
        int x = gr.left + max(0, (int)(gr.right - gr.left - W) / 2), y = gr.top + max(0, (int)(gr.bottom - gr.top - H) / 2);
        SetWindowPos(g_hWnd, HWND_TOPMOST, x, y, W, H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    else SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    if (loadTarget) { RequestTarget(); SetStatus("Loading the drops of your target..."); }
    Paint();
}
static void SyncVisibility()
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
    if (g_searchFocus && IsWindowVisible(g_hWnd)) Paint();   // caret blink
}

static void Scroll(bool list, int delta)
{
    EnterCriticalSection(&g_lock);
    if (list)
    {
        int maxTop = max(0, (int)g_mobs.size() - LIST_ROWS);
        g_listTop = min(maxTop, max(0, g_listTop + delta));
    }
    else g_gridPage = min(GridPages() - 1, max(0, g_gridPage + delta));
    LeaveCriticalSection(&g_lock);
    Paint();
}

static void SetStatus(const std::string& s) { g_status = s; g_statusAt = GetTickCount(); }

static void DoSearch()
{
    std::string q = g_query;
    while (!q.empty() && q.back() == ' ') q.pop_back();
    if (q.size() < 2) { SetStatus("Type at least 2 letters."); Paint(); return; }
    RequestSearch(q);
    SetStatus("Searching...");
    SetSearchFocus(false);
    Paint();
}

static void Activate(int ctl)
{
    if (ctl == C_CLOSE) { Hide(); return; }
    if (ctl == C_EDIT) { SetSearchFocus(true); Paint(); return; }
    SetSearchFocus(false);
    if (ctl == C_SEARCH) { DoSearch(); return; }
    if (ctl == C_PREV) { Scroll(false, -1); return; }
    if (ctl == C_NEXT) { Scroll(false, 1); return; }
    if (ctl == C_SAVE)
    {
        EnterCriticalSection(&g_blockLock); BlockSave(); BlockSend(); LeaveCriticalSection(&g_blockLock);
        SetStatus("Block list saved."); Paint(); return;
    }
    if (ctl == C_RESET)
    {
        EnterCriticalSection(&g_blockLock); g_blocked.clear(); BlockSave(); BlockSend(); LeaveCriticalSection(&g_blockLock);
        SetStatus("Block list cleared."); Paint(); return;
    }
    if (ctl >= C_ROW0 && ctl < C_CELL0)
    {
        EnterCriticalSection(&g_lock);
        int i = ctl - C_ROW0;
        if (i < (int)g_mobs.size()) { g_selMob = i; RequestProto(g_mobs[i].proto); SetStatus("Loading " + g_mobs[i].name + "..."); }
        LeaveCriticalSection(&g_lock);
    }
    Paint();
}

static const UINT WM_DROP_TOGGLE = WM_APP + 32, WM_DROP_SHOW = WM_APP + 33, WM_DROP_WHEEL = WM_APP + 34;

static LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
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
        if (!g_tracking) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_tracking = TrackMouseEvent(&t) != FALSE; }
        if (int hit = HitTest(x, y); hit != g_hover)
        {
            g_hover = hit; Paint();
            UINT32 item = 0; RECT a = {};
            EnterCriticalSection(&g_lock);
            if (hit >= C_CELL0 && hit - C_CELL0 < (int)g_drops.size())
            {
                item = g_drops[hit - C_CELL0].item;
                int vis = hit - C_CELL0 - g_gridPage * GRID_N;
                DR c = D4(DP_CELL[vis]); RECT wr; GetWindowRect(h, &wr);
                a = { wr.left, wr.top + c.t, wr.right, wr.top + c.b };   // the game draws it beside the panel
            }
            LeaveCriticalSection(&g_lock);
            if (item) ItemTip_Show(item, a); else ItemTip_Hide();
        }
        return 0;
    case WM_MOUSELEAVE: g_tracking = false; ItemTip_Hide(); if (g_hover != C_NONE) { g_hover = C_NONE; Paint(); } return 0;
    case WM_LBUTTONDOWN:
    {
        int hit = HitTest(x, y);
        if (hit == C_NONE) { g_dragging = true; GetCursorPos(&g_dragFrom); SetCapture(h); return 0; }
        g_pressed = hit; SetCapture(h); Paint(); return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (g_dragging) { g_dragging = false; return 0; }
        int hit = HitTest(x, y), pressed = g_pressed; g_pressed = C_NONE;
        if (pressed != C_NONE && pressed == hit) Activate(hit); else Paint();
        return 0;
    }
    case WM_RBUTTONUP:
    {
        // on a drop: block / allow it; on a block slot: allow it again; anywhere else: close the panel
        int hit = HitTest(x, y);
        DropMiniEntry e = {};
        EnterCriticalSection(&g_lock);
        if (hit >= C_CELL0 && hit < C_BSLOT0 && hit - C_CELL0 < (int)g_drops.size()) { const Drop& d = g_drops[hit - C_CELL0]; e = { d.item, d.pct, d.icon, d.name }; }
        LeaveCriticalSection(&g_lock);
        if (hit >= C_BSLOT0)
        {
            EnterCriticalSection(&g_blockLock);
            if (hit - C_BSLOT0 < (int)g_blocked.size()) e = g_blocked[hit - C_BSLOT0];
            LeaveCriticalSection(&g_blockLock);
        }
        if (!e.item) { Hide(); return 0; }
        int res = DropBlock_Toggle(e);
        EnterCriticalSection(&g_lock);
        SetStatus(res == 1 ? e.name + " is blocked." : res == 0 ? e.name + " is allowed again." : "Block list is full (10).");
        LeaveCriticalSection(&g_lock);
        ItemTip_Hide(); g_hover = C_NONE;
        Paint();
        return 0;
    }
    case WM_MOUSEWHEEL:
    case WM_DROP_WHEEL:
    {
        POINT p; GetCursorPos(&p); ScreenToClient(h, &p);
        int d = GET_WHEEL_DELTA_WPARAM(wp) > 0 ? -1 : 1;
        if (In(D4(DP_LIST), p.x, p.y)) Scroll(true, d * 3); else Scroll(false, d);
        return 0;
    }
    case WM_CHAR:
        if (!g_searchFocus) return 0;
        if (wp == VK_RETURN) { DoSearch(); return 0; }
        if (wp == VK_ESCAPE) { SetSearchFocus(false); Paint(); return 0; }
        if (wp == VK_BACK) { if (!g_query.empty()) g_query.pop_back(); Paint(); return 0; }
        if (wp >= 32 && wp < 256 && g_query.size() < 30) { g_query.push_back((char)wp); Paint(); }
        return 0;
    case WM_KILLFOCUS: if (g_searchFocus) { g_searchFocus = false; Paint(); } return 0;
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: SyncVisibility(); return 0;
    case WM_DROP_REFRESH: if (IsWindowVisible(h)) Paint(); return 0;   // also after a block list change
    case WM_DROP_SHOW: Show(false); return 0;
    case WM_DROP_TOGGLE: if (g_open) Hide(); else Show(wp != 0); return 0;
    case WM_CLOSE: Hide(); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThread(LPVOID)
{
    GetModuleFileNameA(nullptr, g_iconDir, MAX_PATH);
    strcpy_s(strrchr(g_iconDir, '\\') + 1, 48, "JstKO\\pus_ui\\icons");
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr); wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = L"NTT_DropPanel";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, wc.lpszClassName, L"Drop Search",
        WS_POPUP, 0, 0, W, H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("DROP: window creation failed"); return 0; }
    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), W, -H, 1, 32, BI_RGB };
    SelectObject(g_memDC, CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
    SetTimer(g_hWnd, 1, 250, nullptr);
    Log("DROP: window ready ('\\' or Ctrl+D)");
    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// ---------------------------------------------------------------- game thread entry points
void Drop_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[1] != DROP_SUBOP) return;
    Reader r{ buf, len, 2 };
    BYTE op = r.get<BYTE>();
    if (op == 1)
    {
        UINT16 proto = r.get<UINT16>(); std::string name = r.str(); UINT16 level = r.get<UINT16>(); BYTE isMon = r.get<BYTE>();
        UINT16 n = r.get<UINT16>();
        std::vector<Drop> v;
        for (UINT16 i = 0; i < n && r.ok; i++) { Drop d; d.item = r.get<UINT32>(); d.pct = r.get<UINT16>(); d.icon = r.get<UINT32>(); d.name = r.str(); if (r.ok) v.push_back(d); }
        if (!r.ok) { Log("DROP recv: short drop list"); return; }
        // proto 0 = what a gem / chest turns into, sent on its right click (XGuard.cpp RceSendGeneratorContents):
        // switched off for now, the panel is for monsters only
        if (proto == 0) return;
        std::sort(v.begin(), v.end(), [](const Drop& a, const Drop& b) { return a.pct > b.pct; });
        if (InterlockedExchange(&g_miniPending, 0))
        {
            std::vector<DropMiniEntry> mini;
            for (auto& d : v) mini.push_back({ d.item, d.pct, d.icon, d.name });
            DropMini_Show(name, level, mini);
            return;
        }
        EnterCriticalSection(&g_lock);
        g_drops = v; g_gridPage = 0; g_showBlocked = false;
        char t[160]; sprintf_s(t, "%s  (Lv%u)  -  %u drop%s", name.c_str(), level, (unsigned)v.size(), v.size() == 1 ? "" : "s");
        g_dropTitle = t;
        g_status = v.empty() ? (isMon ? "This monster drops nothing." : "This NPC drops nothing.") : "Hover an item to see its name and drop chance.";
        for (int i = 0; i < (int)g_mobs.size(); i++) if (g_mobs[i].proto == proto) g_selMob = i;
        LeaveCriticalSection(&g_lock);
        char msg[160]; sprintf_s(msg, "DROP recv: %s proto=%u items=%u", name.c_str(), proto, (unsigned)v.size()); Log(msg);
        if (g_hWnd) PostMessageW(g_hWnd, WM_DROP_SHOW, 0, 0);
    }
    else if (op == 9)
    {
        // game start: the server asks for this character's block list
        std::string name = r.str();
        if (!r.ok) return;
        EnterCriticalSection(&g_blockLock);
        g_blockChar = name; BlockLoad(); BlockSend();
        unsigned n = (unsigned)g_blocked.size();
        LeaveCriticalSection(&g_blockLock);
        char msg[96]; sprintf_s(msg, "DROP: block list of %s sent (%u items)", name.c_str(), n); Log(msg);
        if (g_hWnd) PostMessageW(g_hWnd, WM_DROP_REFRESH, 0, 0);
    }
    else if (op == 5)
    {
        UINT16 n = r.get<UINT16>();
        std::vector<Mob> v;
        for (UINT16 i = 0; i < n && r.ok; i++) { Mob m; m.proto = r.get<UINT16>(); m.name = r.str(); m.level = r.get<UINT16>(); m.hasDrops = r.get<BYTE>(); if (r.ok) v.push_back(m); }
        if (!r.ok) { Log("DROP recv: short search result"); return; }
        EnterCriticalSection(&g_lock);
        g_mobs = v; g_listTop = 0; g_selMob = -1;
        g_status = v.empty() ? "No monster found." : "Click a monster to see its drops.";
        LeaveCriticalSection(&g_lock);
        if (!v.empty() && v[0].hasDrops && v.size() == 1) RequestProto(v[0].proto);
        if (g_hWnd) PostMessageW(g_hWnd, WM_DROP_REFRESH, 0, 0);
    }
}

// called from d3d9proxy.cpp's game-window subclass
void Drop_Toggle(bool loadTarget) { if (g_hWnd) PostMessageW(g_hWnd, WM_DROP_TOGGLE, loadTarget ? 1 : 0, 0); }

bool Drop_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}

// the wheel goes to the focused game window; forward it while the cursor is over the panel
bool Drop_ForwardWheel(WPARAM wp)
{
    if (!Drop_CursorOverPanel()) return false;
    PostMessageW(g_hWnd, WM_DROP_WHEEL, wp, 0);
    return true;
}

void Drop_Init()
{
    InitializeCriticalSection(&g_lock);
    InitializeCriticalSection(&g_sendLock);
    InitializeCriticalSection(&g_blockLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// other panels (clan_kasa.cpp) queue their own packets through the same game-thread flush
void Drop_SendRaw(const std::vector<BYTE>& p) { QueueSend(p); }
