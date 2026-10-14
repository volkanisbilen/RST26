// Daily Quest panel for the 2625 client (d3d9 proxy): GDI layered overlay window fed from the recv hook, same
// technique as lottery_panel. Skin: build_daily_uif.py (from uif_in\re_dailyquest.uif) -> daily_ui\*.pus + daily_layout.h.
// Opened by the server (op 3) after it answers the taskbar event-menu trigger (genie_hg.cpp DailyQuest_Open: [0x9C 0x00]).
//
// WIZ_EXT_HOOK 0xE9, sub 0xD3, third byte = op; str = u16 len + CP1254 bytes; ints LE.
//   S->C op0 LIST    [u16 n] n x { id u8, timeType u8, minLv u8, maxLv u8, kill u16, replayHours u16, str name, str desc, str rewards }
//        op1 PROGRESS[u16 n] n x { id u8, status u8 (0 ongoing,1 cooldown,2 completed), kills u16, secsLeft u32 }
//        op2 KILL    [u8 id][u16 kills]
//        op3 OPEN    [u8 page]
//        op4 RESULT  [u8 requestOp][u8 id][u8 result]
//   C->S op3 refresh, op5 accept [id], op6 abandon [id], op7 claim [id]
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include <map>
#include "daily_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();

static const DWORD KO_SND_FNC = 0x00704070;
static const DWORD KO_PTR_PKT = 0x01115914;
static const BYTE  WIZ_EXT_HOOK = 0xE9;
static const BYTE  DQ_SUBOP = 0xD3;

struct Rc { int l, t, r, b; };
static Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }

struct Quest { BYTE id = 0, timeType = 0, minLv = 0, maxLv = 0; UINT16 kill = 0, replay = 0; std::string name, desc, rewards; };
struct Prog { BYTE status = 0; UINT16 kills = 0; UINT32 secs = 0; DWORD at = 0; };

static const int ROWS_VIS = DQ_ROWS;   // quests per page

static CRITICAL_SECTION g_lock, g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQ;
static std::vector<Quest> g_quests;
static std::map<BYTE, Prog> g_prog;
static int  g_sel = -1;            // quest id
static int  g_page = 0;            // visible page (15 quests each)
static int  g_lastRow = -1; static DWORD g_lastClick = 0;   // double click detection
static std::string g_msg; DWORD g_msgAt = 0; bool g_msgOk = false;
static bool g_visible = false;
static POINT g_panelPos = { -1, -1 };
static HWND g_hWnd = nullptr;
static HDC  g_memDC = nullptr;
static void* g_bits = nullptr;
static HCURSOR g_gameCursor = nullptr;
static const UINT WM_DQ_REFRESH = WM_APP + 47;

// ---------------------------------------------------------------- send
static void QueueSend(const std::vector<BYTE>& p)
{
    EnterCriticalSection(&g_sendLock);
    g_sendQ.push_back(p);
    LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();
}

void Daily_FlushSendQueue()
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
        Log("DAILY send");
    }
}

// ---------------------------------------------------------------- sprites
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<std::string, Sprite> g_skin;
static char g_dir[MAX_PATH];
static bool g_loaded = false;

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
    const char* names[] = { "bg", "row_a", "row_b", "close_n", "close_o", "close_d", "cancel_n", "cancel_o", "cancel_d" };
    int ok = 0;
    for (const char* n : names)
    {
        char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\daily_ui\\%s.pus", g_dir, n);
        Sprite s; if (LoadPus(p, s)) { g_skin[n] = s; ok++; }
    }
    {
        char p[MAX_PATH]; sprintf_s(p, "%sHopeGuard\\pus_ui\\noimage.pus", g_dir);
        Sprite s; if (LoadPus(p, s)) { g_skin["noimage"] = s; ok++; }
    }
    char b[64]; sprintf_s(b, "DAILY: skin %d/10", ok); Log(b);
}

static void Blit(Sprite* s, int x, int y, int w = -1, int h = -1)
{
    if (!s || !s->dc) return;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_memDC, x, y, w < 0 ? s->w : w, h < 0 ? s->h : h, s->dc, 0, 0, s->w, s->h, bf);
}
static bool Skin(const char* n, int x, int y, int w = -1, int h = -1) { auto it = g_skin.find(n); if (it == g_skin.end()) return false; Blit(&it->second, x, y, w, h); return true; }

static void Fill(Rc r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(DQ_W, r.r), bb = min(DQ_H, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* d = (BYTE*)g_bits + (y * DQ_W + l) * 4;
        for (int x = l; x < rr; x++, d += 4)
        {
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
            d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}

static void Text(const std::string& s, Rc r, COLORREF color, int pt, UINT fmt, bool bold = false, bool multi = false)
{
    int w = r.r - r.l, h = r.b - r.t;
    if (s.empty() || w <= 0 || h <= 0) return;
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), w, -h, 1, 32, BI_RGB };
    void* bits = nullptr;
    HDC dc = CreateCompatibleDC(nullptr);
    HBITMAP bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    if (!bmp) { DeleteDC(dc); return; }
    HGDIOBJ ob = SelectObject(dc, bmp);
    HFONT font = CreateFontA(-MulDiv(pt, 96, 72), 0, 0, 0, bold ? FW_BOLD : FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, "Verdana");
    HGDIOBJ of = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, RGB(255, 255, 255));
    RECT rc = { 0, 0, w, h };
    DrawTextA(dc, s.c_str(), (int)s.size(), &rc, fmt | DT_NOPREFIX | (multi ? DT_WORDBREAK : (DT_SINGLELINE | DT_END_ELLIPSIS)));
    GdiFlush();
    for (int pass = 0; pass < 2; pass++)
    {
        const int o = pass == 0 ? 1 : 0;
        const BYTE cr = pass ? GetRValue(color) : 0, cg = pass ? GetGValue(color) : 0, cb = pass ? GetBValue(color) : 0;
        for (int y = 0; y < h; y++)
        {
            int dy = r.t + y + o;
            if (dy < 0 || dy >= DQ_H) continue;
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            for (int x = 0; x < w; x++, src += 4)
            {
                int dx = r.l + x + o;
                int a = max(src[0], max(src[1], src[2]));
                if (!a || dx < 0 || dx >= DQ_W) continue;
                if (!pass) a = a * 3 / 4;
                BYTE* d = (BYTE*)g_bits + (dy * DQ_W + dx) * 4;
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

// ---------------------------------------------------------------- helpers
static UINT32 SecsLeft(const Prog& p)
{
    DWORD el = (GetTickCount() - p.at) / 1000;
    return el >= p.secs ? 0 : p.secs - el;
}
static std::string FmtTime(UINT32 s)
{
    char b[24];
    if (s >= 3600) sprintf_s(b, "%uh %02um", s / 3600, (s % 3600) / 60);
    else sprintf_s(b, "%02u:%02u", s / 60, s % 60);
    return b;
}
static const char* ResultText(BYTE r, bool& ok)
{
    ok = r == 0;
    switch (r)
    {
    case 0: return "Done.";
    case 1: return "Your level does not fit this quest.";
    case 2: return "Requirement / zone not met.";
    case 3: return "This quest is already active.";
    case 4: return "Quest slots full (max 4).";
    case 5: return "Quest is on cooldown.";
    case 6: return "Quest is not completed yet.";
    default: return "Unknown quest.";
    }
}

enum Ctl { C_NONE = -1, C_CLOSE = 1, C_CANCEL, C_PREV, C_NEXT, C_TITLE, C_HUNT1, C_REW1, C_REW2, C_REW3, C_REW4, C_ROW0 = 20 };   // C_ROW0 + visible row
static int  g_hover = C_NONE, g_pressed = C_NONE;
static bool g_track = false, g_drag = false;
static POINT g_dragFrom;
static bool In(Rc r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }
static const int DQ_TITLE_BAR[4] = { 216, 10, 516, 42 };

static int HitTest(int x, int y)
{
    if (In(R4(DQ_BTN_CLOSE), x, y)) return C_CLOSE;
    if (In(R4(DQ_BTN_CANCEL), x, y)) return C_CANCEL;
    if (In(R4(DQ_BTN_PREV), x, y)) return C_PREV;
    if (In(R4(DQ_BTN_NEXT), x, y)) return C_NEXT;
    if (In(R4(DQ_TITLE_BAR), x, y)) return C_TITLE;
    if (In(R4(DQ_HUNT1_BOX), x, y)) return C_HUNT1;
    const int* rew[4] = { DQ_REW1_BOX, DQ_REW2_BOX, DQ_REW3_BOX, DQ_REW4_BOX };
    for (int i = 0; i < 4; i++) if (In(R4(rew[i]), x, y)) return C_REW1 + i;
    if (x >= DQ_ROW_L && x < DQ_ROW_R)
        for (int i = 0; i < DQ_ROWS; i++)
            if (y >= DQ_ROW_Y[i] && y < DQ_ROW_Y[i] + DQ_ROW_H) return C_ROW0 + i;
    return C_NONE;
}

static int PageCount(size_t n) { return max(1, (int)((n + DQ_ROWS - 1) / DQ_ROWS)); }

// "Experience 50, Blessed Upgrade Scroll x1, ..." -> up to 4 (name, count) pairs
static std::vector<std::pair<std::string, std::string>> SplitRewards(const std::string& rw)
{
    std::vector<std::pair<std::string, std::string>> out;
    size_t pos = 0;
    while (pos < rw.size() && out.size() < 4)
    {
        size_t c = rw.find(',', pos);
        std::string part = rw.substr(pos, c == std::string::npos ? std::string::npos : c - pos);
        pos = c == std::string::npos ? rw.size() : c + 1;
        while (!part.empty() && part.front() == ' ') part.erase(part.begin());
        while (!part.empty() && part.back() == ' ') part.pop_back();
        if (part.empty()) continue;
        std::string cnt;
        size_t x = part.rfind(" x");
        if (x != std::string::npos && x + 2 < part.size()) { cnt = "x" + part.substr(x + 2); part = part.substr(0, x); }
        out.push_back({ part, cnt });
    }
    return out;
}

// ---------------------------------------------------------------- rendering
static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    LoadSkin();
    memset(g_bits, 0, (size_t)DQ_W * DQ_H * 4);
    EnterCriticalSection(&g_lock);
    std::vector<Quest> quests = g_quests;
    std::map<BYTE, Prog> prog = g_prog;
    int sel = g_sel, page = g_page;
    std::string msg = g_msg; DWORD msgAt = g_msgAt; bool msgOk = g_msgOk;
    LeaveCriticalSection(&g_lock);
    char b[160];
    int pages = PageCount(quests.size());
    if (page >= pages) page = pages - 1;

    if (!Skin("bg", 0, 0)) Fill({ 0, 0, DQ_W, DQ_H }, 30, 36, 30, 245);

    // list rows
    for (int i = 0; i < DQ_ROWS; i++)
    {
        int qi = page * DQ_ROWS + i;
        if (qi >= (int)quests.size()) break;
        const Quest& q = quests[qi];
        int ry = DQ_ROW_Y[i];
        auto pi = prog.find(q.id);
        COLORREF col = RGB(160, 220, 255); std::string status = "Available", tm = "-";
        if (pi != prog.end())
        {
            BYTE st = pi->second.status;
            if (st == 0) { col = RGB(255, 220, 120); sprintf_s(b, "%u/%u", pi->second.kills, q.kill); status = b; }
            else if (st == 1) { col = RGB(170, 170, 170); status = "Cooldown"; UINT32 sl = SecsLeft(pi->second); sprintf_s(b, "%02u:%02u:%02u", sl / 3600, (sl % 3600) / 60, sl % 60); tm = b; }
            else { col = RGB(140, 255, 140); status = "Completed"; }
        }
        if (q.id == sel) Skin("row_a", DQ_ROW_L, ry, DQ_ROW_R - DQ_ROW_L, DQ_ROW_H);
        else if (g_hover == C_ROW0 + i) Skin("row_b", DQ_ROW_L, ry, DQ_ROW_R - DQ_ROW_L, DQ_ROW_H);
        Text(q.name, { DQ_COL_NAME[0], ry, DQ_COL_NAME[1], ry + DQ_ROW_H }, col, 8, DT_LEFT | DT_VCENTER, true);
        Text(status, { DQ_COL_STATUS[0], ry, DQ_COL_STATUS[1], ry + DQ_ROW_H }, col, 8, DT_CENTER | DT_VCENTER);
        Text(tm, { DQ_COL_TIME[0], ry, DQ_COL_TIME[1], ry + DQ_ROW_H }, RGB(235, 235, 235), 8, DT_CENTER | DT_VCENTER);
        sprintf_s(b, "Hunt  Lv%u-%u", q.minLv, q.maxLv);
        Text(b, { DQ_COL_TYPE[0], ry, DQ_COL_TYPE[1], ry + DQ_ROW_H }, RGB(235, 235, 235), 8, DT_CENTER | DT_VCENTER);
        Text("-", { DQ_COL_ZONE[0], ry, DQ_COL_ZONE[1], ry + DQ_ROW_H }, RGB(235, 235, 235), 8, DT_CENTER | DT_VCENTER);
    }
    if (quests.empty()) Text("No quests received.", { DQ_ROW_L, DQ_ROW_Y[0], DQ_ROW_R, DQ_ROW_Y[5] }, RGB(170, 170, 170), 9, DT_CENTER | DT_VCENTER);
    sprintf_s(b, "%d", page + 1);
    Text(b, R4(DQ_PAGE), RGB(255, 210, 60), 9, DT_CENTER | DT_VCENTER, true);

    // close buttons
    Skin(g_hover == C_CLOSE ? (g_pressed == C_CLOSE ? "close_d" : "close_o") : "close_n", DQ_BTN_CLOSE[0], DQ_BTN_CLOSE[1]);
    Skin(g_hover == C_CANCEL ? (g_pressed == C_CANCEL ? "cancel_d" : "cancel_o") : "cancel_n", DQ_BTN_CANCEL[0], DQ_BTN_CANCEL[1]);

    // selected quest: name, level, the hunt box and the rewards
    const Quest* sq = nullptr;
    for (auto& q : quests) if (q.id == sel) sq = &q;
    const Prog* sp = nullptr;
    if (sq) { auto pi = prog.find(sq->id); if (pi != prog.end()) sp = &pi->second; }
    struct Slot { const int* icon; const int* name; const int* count; };
    const Slot hunt[4] = { { DQ_HUNT1_ICON, DQ_HUNT1_NAME, DQ_HUNT1_COUNT }, { DQ_HUNT2_ICON, DQ_HUNT2_NAME, DQ_HUNT2_COUNT },
                           { DQ_HUNT3_ICON, DQ_HUNT3_NAME, DQ_HUNT3_COUNT }, { DQ_HUNT4_ICON, DQ_HUNT4_NAME, DQ_HUNT4_COUNT } };
    const Slot rew[4] = { { DQ_REW1_ICON, DQ_REW1_NAME, DQ_REW1_COUNT }, { DQ_REW2_ICON, DQ_REW2_NAME, DQ_REW2_COUNT },
                          { DQ_REW3_ICON, DQ_REW3_NAME, DQ_REW3_COUNT }, { DQ_REW4_ICON, DQ_REW4_NAME, DQ_REW4_COUNT } };
    std::string hint;
    if (sq)
    {
        Text(sq->name, R4(DQ_QNAME), RGB(255, 220, 120), 9, DT_CENTER | DT_VCENTER, true);
        sprintf_s(b, "Level %u-%u", sq->minLv, sq->maxLv);
        Text(b, R4(DQ_QZONE), RGB(255, 150, 30), 8, DT_CENTER | DT_VCENTER, true);

        UINT16 kills = sp ? sp->kills : 0;
        if (sp && sp->status == 2) kills = sq->kill;
        Skin("noimage", hunt[0].icon[0], hunt[0].icon[1], hunt[0].icon[2] - hunt[0].icon[0], hunt[0].icon[3] - hunt[0].icon[1]);
        Text("Hunt", R4(hunt[0].name), RGB(255, 255, 255), 8, DT_LEFT | DT_VCENTER, true);
        sprintf_s(b, "%u / %u", kills, (unsigned)sq->kill);
        Text(b, R4(hunt[0].count), RGB(255, 225, 120), 8, DT_LEFT | DT_VCENTER, true);
        Text(sq->desc, { DQ_HUNT2_ICON[0] - 18, DQ_HUNT2_BOX[1] + 6, DQ_HUNT2_NAME[2] + 16, DQ_HUNT4_BOX[3] - 6 }, RGB(225, 225, 225), 7, DT_LEFT | DT_TOP, false, true);

        auto rw = SplitRewards(sq->rewards);
        for (size_t i = 0; i < rw.size(); i++)
        {
            Skin("noimage", rew[i].icon[0], rew[i].icon[1], rew[i].icon[2] - rew[i].icon[0], rew[i].icon[3] - rew[i].icon[1]);
            Text(rw[i].first, R4(rew[i].name), RGB(160, 235, 160), 8, DT_LEFT | DT_VCENTER, true);
            Text(rw[i].second, R4(rew[i].count), RGB(255, 225, 120), 8, DT_LEFT | DT_VCENTER, true);
        }
        if (!sp) hint = "Double-click the quest (or click the Hunt box) to accept it.";
        else if (sp->status == 0) hint = "Ongoing - right-click the quest to abandon it.";
        else if (sp->status == 2) hint = "Completed - click a reward (or double-click the quest) to claim.";
        else { sprintf_s(b, "On cooldown: %s", FmtTime(SecsLeft(*sp)).c_str()); hint = b; }
    }
    else hint = "Select a quest from the list.";

    if (!msg.empty() && GetTickCount() - msgAt < 8000)
        Text(msg, R4(DQ_MSG), msgOk ? RGB(140, 255, 140) : RGB(255, 140, 120), 9, DT_CENTER | DT_VCENTER, true);
    else
        Text(hint, R4(DQ_MSG), RGB(200, 200, 170), 8, DT_CENTER | DT_VCENTER);

    RECT wr; GetWindowRect(g_hWnd, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { DQ_W, DQ_H };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
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
        g_panelPos.x = gr.left + ((gr.right - gr.left) - DQ_W) / 2;
        g_panelPos.y = gr.top + ((gr.bottom - gr.top) - DQ_H) / 2;
    }
    SetWindowPos(g_hWnd, HWND_TOPMOST, g_panelPos.x, g_panelPos.y, DQ_W, DQ_H, SWP_NOACTIVATE);
}

static void SetVisible(bool v)
{
    g_visible = v;
    g_hover = C_NONE; g_pressed = C_NONE;
    if (!v) { ShowWindow(g_hWnd, SW_HIDE); return; }
    Place();
    Paint();
}

static void SyncVisibility()
{
    HWND game = FindGameWindow();
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool fg = pid == GetCurrentProcessId();
    bool want = g_visible && game && !IsIconic(game) && fg;
    if (want != (IsWindowVisible(g_hWnd) != FALSE))
    {
        if (want) { SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); Paint(); }
        else ShowWindow(g_hWnd, SW_HIDE);
    }
    static bool escWas = false;
    bool esc = (GetAsyncKeyState(VK_ESCAPE) & 0x8000) != 0;
    if (esc && !escWas && want) SetVisible(false);
    escWas = esc;
    POINT p; CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) && (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursor = ci.hCursor;
    static DWORD lastSec = 0;
    if (want && g_visible && GetTickCount() - lastSec >= 1000) { lastSec = GetTickCount(); Paint(); }
}

static void SendQuestOp(BYTE op, int sel) { QueueSend({ WIZ_EXT_HOOK, DQ_SUBOP, op, (BYTE)sel }); }

// accept / claim the selected quest if its state allows it (same rules as the hints)
static void AcceptOrClaim(bool accept, bool claim)
{
    EnterCriticalSection(&g_lock);
    int sel = g_sel;
    auto pi = g_prog.find((BYTE)sel);
    bool none = pi == g_prog.end();
    bool done = !none && pi->second.status == 2;
    LeaveCriticalSection(&g_lock);
    if (sel < 0) return;
    if (accept && none) SendQuestOp(5, sel);
    else if (claim && done) SendQuestOp(7, sel);
}

static void SelectRow(int row)
{
    EnterCriticalSection(&g_lock);
    int qi = g_page * DQ_ROWS + row;
    if (qi >= 0 && qi < (int)g_quests.size()) g_sel = g_quests[qi].id;
    LeaveCriticalSection(&g_lock);
}

static void Activate(int ctl)
{
    if (ctl == C_CLOSE || ctl == C_CANCEL) SetVisible(false);
    else if (ctl == C_TITLE) QueueSend({ WIZ_EXT_HOOK, DQ_SUBOP, 3 });
    else if (ctl == C_PREV) { EnterCriticalSection(&g_lock); if (g_page > 0) g_page--; LeaveCriticalSection(&g_lock); }
    else if (ctl == C_NEXT)
    {
        EnterCriticalSection(&g_lock);
        if (g_page + 1 < PageCount(g_quests.size())) g_page++;
        LeaveCriticalSection(&g_lock);
    }
    else if (ctl == C_HUNT1) AcceptOrClaim(true, false);
    else if (ctl >= C_REW1 && ctl <= C_REW4) AcceptOrClaim(false, true);
    else if (ctl >= C_ROW0)
    {
        int row = ctl - C_ROW0;
        DWORD now = GetTickCount();
        bool dbl = g_lastRow == row && now - g_lastClick < 450;
        g_lastRow = row; g_lastClick = now;
        SelectRow(row);
        if (dbl) { AcceptOrClaim(true, true); g_lastRow = -1; }
    }
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
        if (int hit = HitTest(x, y); hit != g_hover) { g_hover = hit; Paint(); }
        return 0;
    case WM_MOUSELEAVE:
        g_track = false;
        if (g_hover != C_NONE) { g_hover = C_NONE; Paint(); }
        return 0;
    case WM_MOUSEWHEEL:
        EnterCriticalSection(&g_lock);
        g_page += GET_WHEEL_DELTA_WPARAM(wp) > 0 ? -1 : 1;
        if (g_page >= PageCount(g_quests.size())) g_page = PageCount(g_quests.size()) - 1;
        if (g_page < 0) g_page = 0;
        LeaveCriticalSection(&g_lock);
        Paint();
        return 0;
    case WM_RBUTTONUP:
    {
        int hit = HitTest(x, y);
        if (hit >= C_ROW0)
        {
            SelectRow(hit - C_ROW0);
            EnterCriticalSection(&g_lock);
            int sel = g_sel; auto pi = g_prog.find((BYTE)sel);
            bool ongoing = sel >= 0 && pi != g_prog.end() && pi->second.status == 0;
            LeaveCriticalSection(&g_lock);
            if (ongoing) SendQuestOp(6, sel);
            Paint();
        }
        return 0;
    }
    case WM_LBUTTONDOWN:
    {
        int hit = HitTest(x, y);
        if (hit == C_NONE) { g_drag = true; GetCursorPos(&g_dragFrom); SetCapture(h); return 0; }
        g_pressed = hit;
        SetCapture(h); Paint();
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
        if (g_visible) Paint();
        return 0;
    }
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: SyncVisibility(); return 0;
    case WM_DQ_REFRESH:
        if (wp == 1 && !g_visible) SetVisible(true);
        else if (g_visible && IsWindowVisible(h)) Paint();
        return 0;
    case WM_CLOSE: SetVisible(false); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThread(LPVOID)
{
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = L"NTT_DailyQuest";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Daily Quests", WS_POPUP, 0, 0, DQ_W, DQ_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("DAILY: window creation failed"); return 0; }
    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), DQ_W, -DQ_H, 1, 32, BI_RGB };
    HBITMAP bmp = CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0);
    SelectObject(g_memDC, bmp);
    Log("DAILY: window ready");
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
static bool GetStr(const BYTE* p, size_t len, size_t& pos, std::string& s)
{
    UINT16 n = 0;
    if (!Get(p, len, pos, n) || pos + n > len) return false;
    s.assign((const char*)p + pos, n); pos += n; return true;
}

void Daily_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[0] != WIZ_EXT_HOOK || buf[1] != DQ_SUBOP) return;
    size_t pos = 3;
    const BYTE op = buf[2];
    bool open = false;
    EnterCriticalSection(&g_lock);
    if (op == 0)
    {
        UINT16 n = 0; std::vector<Quest> v; bool ok = Get(buf, len, pos, n);
        for (UINT16 i = 0; ok && i < n && i < 255; i++)
        {
            Quest q;
            ok = Get(buf, len, pos, q.id) && Get(buf, len, pos, q.timeType) && Get(buf, len, pos, q.minLv) && Get(buf, len, pos, q.maxLv) &&
                 Get(buf, len, pos, q.kill) && Get(buf, len, pos, q.replay) && GetStr(buf, len, pos, q.name) &&
                 GetStr(buf, len, pos, q.desc) && GetStr(buf, len, pos, q.rewards);
            if (ok) v.push_back(q);
        }
        if (ok)
        {
            g_quests.swap(v);
            bool found = false;
            for (auto& q : g_quests) if (q.id == g_sel) found = true;
            if (!found) g_sel = g_quests.empty() ? -1 : g_quests[0].id;
            if (g_page >= PageCount(g_quests.size())) g_page = 0;
        }
    }
    else if (op == 1)
    {
        UINT16 n = 0; std::map<BYTE, Prog> m; bool ok = Get(buf, len, pos, n);
        for (UINT16 i = 0; ok && i < n && i < 255; i++)
        {
            BYTE id = 0; Prog p;
            ok = Get(buf, len, pos, id) && Get(buf, len, pos, p.status) && Get(buf, len, pos, p.kills) && Get(buf, len, pos, p.secs);
            if (ok) { p.at = GetTickCount(); m[id] = p; }
        }
        if (ok) g_prog.swap(m);
    }
    else if (op == 2)
    {
        BYTE id = 0; UINT16 k = 0;
        if (Get(buf, len, pos, id) && Get(buf, len, pos, k)) { auto it = g_prog.find(id); if (it != g_prog.end()) it->second.kills = k; }
    }
    else if (op == 3) open = true;
    else if (op == 4)
    {
        BYTE rop = 0, id = 0, res = 0;
        if (Get(buf, len, pos, rop) && Get(buf, len, pos, id) && Get(buf, len, pos, res))
        {
            bool ok; g_msg = ResultText(res, ok); g_msgOk = ok; g_msgAt = GetTickCount();
        }
    }
    LeaveCriticalSection(&g_lock);
    if (g_hWnd) PostMessageW(g_hWnd, WM_DQ_REFRESH, open ? 1 : 0, 0);
}

// ---------------------------------------------------------------- exports
void Daily_Init()
{
    InitializeCriticalSection(&g_lock);
    InitializeCriticalSection(&g_sendLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

bool Daily_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
