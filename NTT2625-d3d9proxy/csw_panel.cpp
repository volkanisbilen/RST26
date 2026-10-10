// Castle Siege War clan score board for the 2625 client, loaded through the d3d9 proxy.
// The client ships re_siege_warfare_challenge.uif ("Participant Clans": rank / icon / clan / kill / barracks) but the
// 2625 exe has no class that loads it, and its WIZ_SIEGE handler knows sub codes 1/2/3/4/6, not the old 5 = rank
// list. So the server pushes the board and this panel draws that window itself, with the uif's own images:
// build_csw_assets.py -> csw_ui\*.pus + csw_layout.h. Same technique as cind_panel.cpp (GDI layered overlay window
// fed from the recv hook).
//   mini  : small summary at the right edge (stage time, castle owner, top 10). Header drags, "-" folds,
//           "Ranking Board" opens the big window.
//   board : the uif window, 20 clans, own clan highlighted (img_myself_bg). The uif's "Barracks" column / "Remaining
//           Barracks" box (not a thing on this server) show the clan's deaths / the stage time. Close -> mini.
//
// Server: GameServer thyke_csw.cpp CastleSiegeWarfareBoard, WIZ_HSACS_HOOK 0xE9 / sub CSW 0xE1:
//   S->C op 2 board : [u8 status 1 preparation, 2 war][u32 secs left][str owner clan][str my clan]
//                     [u8 n] n * ([u16 clan id][str clan][u16 kills][u16 deaths][u16 mark version, 0 = none])
//        op 1 hide    (siege over / left Delos)          str = [u8 len][bytes]
//        op 3 mark  : [u16 clan id][u16 mark version][u16 len][the clan symbol as the server stores it: a game
//                     texture, [u32 n][name]"NTF"[ver][u32 w][u32 h][u32 fmt][u32 mip][pixels], 32x32 A1R5G5B5]
//   C->S op 3       : [u16 clan id]  asks for a mark (once per clan and version; drawn in the Icon column)
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include <map>
#include "csw_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();

static const DWORD KO_SND_FNC = 0x00704070;  // unpack: thiscall(CAPISocket*, BYTE* buf, int len)
static const DWORD KO_PTR_PKT = 0x01115914;  // unpack: CAPISocket*

static const BYTE WIZ_HSACS_HOOK = 0xE9;
static const BYTE CSW_SUBOP = 0xE1;          // HSACSXOpCodes::CSW
enum CswOp { WO_hide = 1, WO_board = 2, WO_mark = 3 };

struct Rc { int l, t, r, b; };
static Rc R4(const int a[4]) { return { a[0], a[1], a[2], a[3] }; }
static Rc Row(const int a[4], int i) { return { a[0], a[1] + i * CWS_ROW_PITCH, a[2], a[3] + i * CWS_ROW_PITCH }; }

static const int BUF_W = CWS_W, BUF_H = CWS_H;                 // back buffer = the big window
static const int MN_W = 236, MN_HEAD = 62, MN_ROW = 18, MN_MAXROWS = 10, MN_FOOT = 24;
static const Rc  MN_FOLD = { MN_W - 22, 5, MN_W - 6, 21 };
static const Rc  BD_CLOSE = R4(CWS_CLOSE);

// ---------------------------------------------------------------- state (g_lock)
struct CswRow { std::string clan; UINT16 id = 0, kills = 0, deaths = 0, mark = 0; };
struct CswMark { UINT16 ver = 0; DWORD askedAt = 0; int w = 0, h = 0; std::vector<BYTE> px; };   // px: straight BGRA
struct CswState
{
    bool   active = false, war = false;
    UINT32 secs = 0; DWORD secsAt = 0;       // remaining stage time at secsAt
    std::string owner, mine;
    std::vector<CswRow> rows;
};
enum Mode { M_MINI, M_BOARD };

static CRITICAL_SECTION g_lock;
static CswState g_st;
static std::map<UINT16, CswMark> g_marks;    // by clan id
static CRITICAL_SECTION g_sendLock;
static std::deque<std::vector<BYTE>> g_sendQ;
static bool  g_folded = false;
static int   g_mode = M_MINI;
static POINT g_miniPos = { -1, -1 }, g_boardPos = { -1, -1 };

static HWND  g_hWnd = nullptr;
static HDC   g_memDC = nullptr;
static void* g_bits = nullptr;
static int   g_curW = MN_W, g_curH = MN_HEAD;
static HCURSOR g_gameCursor = nullptr;
static const UINT WM_CW_REFRESH = WM_APP + 47;

static UINT32 Left(UINT32 v, DWORD at)
{
    DWORD el = (GetTickCount() - at) / 1000;
    return el >= v ? 0 : v - el;
}

// ---------------------------------------------------------------- send (flushed on the game thread)
static void QueueSend(const std::vector<BYTE>& p)
{
    EnterCriticalSection(&g_sendLock);
    g_sendQ.push_back(p);
    LeaveCriticalSection(&g_sendLock);
    Proxy_RequestFlush();
}

void Csw_FlushSendQueue()
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
    }
}

// ---------------------------------------------------------------- clan marks
// the server's copy of the symbol is a game texture file; only the uncompressed formats are read
static bool DecodeMark(const BYTE* p, size_t len, CswMark& m)
{
    UINT32 nameLen = 0;
    if (len < 4) return false;
    memcpy(&nameLen, p, 4);
    size_t o = 4 + (size_t)nameLen;
    if (nameLen > 256 || o + 20 > len || memcmp(p + o, "NTF", 3) != 0 || p[o + 3] == 7) return false;   // v7 = encrypted rows
    UINT32 w = 0, h = 0, fmt = 0;
    memcpy(&w, p + o + 4, 4); memcpy(&h, p + o + 8, 4); memcpy(&fmt, p + o + 12, 4);
    o += 20;
    const int bpp = (fmt == 21 || fmt == 22) ? 4 : (fmt == 25 || fmt == 26) ? 2 : 0;
    if (!bpp || !w || !h || w > 64 || h > 64 || o + (size_t)w * h * bpp > len) return false;
    m.w = (int)w; m.h = (int)h; m.px.resize((size_t)w * h * 4);
    for (UINT32 i = 0; i < w * h; i++)
    {
        BYTE b, g, r, a;
        if (bpp == 4) { b = p[o + i * 4]; g = p[o + i * 4 + 1]; r = p[o + i * 4 + 2]; a = fmt == 21 ? p[o + i * 4 + 3] : 255; }
        else
        {
            UINT16 v; memcpy(&v, p + o + i * 2, 2);
            if (fmt == 25) { r = (BYTE)((v >> 10 & 31) * 255 / 31); g = (BYTE)((v >> 5 & 31) * 255 / 31); b = (BYTE)((v & 31) * 255 / 31); a = v & 0x8000 ? 255 : 0; }
            else           { a = (BYTE)((v >> 12 & 15) * 17); r = (BYTE)((v >> 8 & 15) * 17); g = (BYTE)((v >> 4 & 15) * 17); b = (BYTE)((v & 15) * 17); }
        }
        BYTE* d = &m.px[(size_t)i * 4];
        d[0] = b; d[1] = g; d[2] = r; d[3] = a;
    }
    return true;
}

// ---------------------------------------------------------------- sprites (csw_ui\*.pus, premultiplied BGRA)
struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<std::string, Sprite> g_skin;
static bool g_skinLoaded = false;

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
    if (g_skinLoaded) return;
    g_skinLoaded = true;
    char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH);
    strcpy_s(strrchr(dir, '\\') + 1, 48, "HopeGuard\\csw_ui");
    static const char* names[] = { "frame", "row_me", "close_n", "close_o", "close_d" };
    int ok = 0;
    for (const char* n : names)
    {
        char path[MAX_PATH]; sprintf_s(path, "%s\\%s.pus", dir, n);
        Sprite sp;
        if (LoadPus(path, sp)) { g_skin[n] = sp; ok++; }
    }
    char msg[96]; sprintf_s(msg, "CSW: skin sprites loaded %d/%d", ok, (int)(sizeof(names) / sizeof(names[0])));
    Log(msg);
}

static bool Skin(const char* name, int x, int y)
{
    auto it = g_skin.find(name);
    if (it == g_skin.end()) return false;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_memDC, x, y, it->second.w, it->second.h, it->second.dc, 0, 0, it->second.w, it->second.h, bf);
    return true;
}

// ---------------------------------------------------------------- drawing
static void Fill(Rc r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(g_curW, r.r), bb = min(g_curH, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* d = (BYTE*)g_bits + (y * BUF_W + l) * 4;
        for (int x = l; x < rr; x++, d += 4)
        {
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
            d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}

static void Frame(Rc r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    Fill({ r.l, r.t, r.r, r.t + 1 }, cr, cg, cb, a); Fill({ r.l, r.b - 1, r.r, r.b }, cr, cg, cb, a);
    Fill({ r.l, r.t, r.l + 1, r.b }, cr, cg, cb, a); Fill({ r.r - 1, r.t, r.r, r.b }, cr, cg, cb, a);
}

// text with a 1px dark shadow (the game's UI strings are drawn the same way)
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
        const int ox = pass == 0 ? 1 : 0;
        const BYTE cr = pass == 0 ? 0 : GetRValue(color), cg = pass == 0 ? 0 : GetGValue(color), cb = pass == 0 ? 0 : GetBValue(color);
        for (int y = 0; y < h; y++)
        {
            int dy = r.t + y + ox;
            if (dy < 0 || dy >= g_curH) continue;
            const BYTE* src = (const BYTE*)bits + y * w * 4;
            for (int x = 0; x < w; x++, src += 4)
            {
                int dx = r.l + x + ox;
                int a = max(src[0], max(src[1], src[2]));
                if (!a || dx < 0 || dx >= g_curW) continue;
                if (pass == 0) a = a * 3 / 4;
                BYTE* d = (BYTE*)g_bits + (dy * BUF_W + dx) * 4;
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

static std::string MMSS(UINT32 sec)
{
    char b[32]; sprintf_s(b, "%02u : %02u", sec / 60, sec % 60); return b;
}

static bool In(Rc r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

static const COLORREF COL_TITLE = RGB(255, 214, 120);
static const COLORREF COL_LABEL = RGB(255, 255, 128);
static const COLORREF COL_VALUE = RGB(255, 255, 255);
static const COLORREF COL_DIM   = RGB(190, 180, 160);

// clan mark scaled into r (box filter over the source texels of each destination pixel)
static void DrawMark(const CswMark& m, Rc r)
{
    const int dw = r.r - r.l, dh = r.b - r.t;
    if (m.px.empty() || dw <= 0 || dh <= 0) return;
    for (int y = 0; y < dh; y++)
    {
        const int dy = r.t + y;
        if (dy < 0 || dy >= g_curH) continue;
        const int sy0 = y * m.h / dh, sy1 = max(sy0 + 1, (y + 1) * m.h / dh);
        for (int x = 0; x < dw; x++)
        {
            const int dx = r.l + x;
            if (dx < 0 || dx >= g_curW) continue;
            const int sx0 = x * m.w / dw, sx1 = max(sx0 + 1, (x + 1) * m.w / dw);
            int sb = 0, sg = 0, sr = 0, sa = 0, n = 0;
            for (int sy = sy0; sy < sy1 && sy < m.h; sy++)
                for (int sx = sx0; sx < sx1 && sx < m.w; sx++, n++)
                {
                    const BYTE* t = &m.px[((size_t)sy * m.w + sx) * 4];
                    sb += t[0] * t[3]; sg += t[1] * t[3]; sr += t[2] * t[3]; sa += t[3];
                }
            if (!n || !sa) continue;
            const int a = sa / n;                       // coverage; colour = alpha weighted mean
            const int cb = sb / sa, cg = sg / sa, cr = sr / sa;
            BYTE* d = (BYTE*)g_bits + (dy * BUF_W + dx) * 4;
            d[0] = (BYTE)((cb * a + d[0] * (255 - a)) / 255);
            d[1] = (BYTE)((cg * a + d[1] * (255 - a)) / 255);
            d[2] = (BYTE)((cr * a + d[2] * (255 - a)) / 255);
            d[3] = (BYTE)(a + d[3] * (255 - a) / 255);
        }
    }
}

// ---------------------------------------------------------------- controls
enum Ctl { C_NONE = -1, C_FOLD, C_OPEN, C_CLOSE };
static int g_hover = C_NONE, g_pressed = C_NONE;

static int MiniRows(const CswState& s) { return min((int)s.rows.size(), MN_MAXROWS); }
static int MiniHeight(const CswState& s, bool folded)
{
    return folded ? MN_HEAD : MN_HEAD + 20 + max(MiniRows(s), 1) * MN_ROW + 6 + MN_FOOT;
}
static Rc MiniOpenBtn(int h) { return { 8, h - MN_FOOT, MN_W - 8, h - 5 }; }

static int HitTest(int x, int y)
{
    if (g_mode == M_BOARD) return In(BD_CLOSE, x, y) ? C_CLOSE : C_NONE;
    if (In(MN_FOLD, x, y)) return C_FOLD;
    if (!g_folded && In(MiniOpenBtn(g_curH), x, y)) return C_OPEN;
    return C_NONE;
}

// ---------------------------------------------------------------- rendering
static void PaintMini(const CswState& s, bool folded)
{
    const int rows = MiniRows(s);
    g_curW = MN_W; g_curH = MiniHeight(s, folded);

    Fill({ 0, 0, MN_W, g_curH }, 20, 16, 12, 205);
    Fill({ 0, 0, MN_W, 26 }, 70, 40, 14, 230);
    Frame({ 0, 0, MN_W, g_curH }, 168, 128, 64, 255);

    Text("CASTLE SIEGE WAR", { 8, 4, MN_W - 26, 24 }, COL_TITLE, 9, DT_LEFT | DT_VCENTER, true);
    const bool hf = g_hover == C_FOLD;
    Fill(MN_FOLD, hf ? 150 : 96, hf ? 100 : 62, 30, 255);
    Text(folded ? "+" : "-", MN_FOLD, COL_VALUE, 9, DT_CENTER | DT_VCENTER, true);

    const UINT32 left = Left(s.secs, s.secsAt);
    Text(s.war ? "War ends in" : "War starts in", { 8, 26, 118, 44 }, COL_LABEL, 8, DT_LEFT | DT_VCENTER);
    Text(MMSS(left), { 118, 26, MN_W - 8, 44 }, left <= 60 ? RGB(255, 120, 100) : COL_VALUE, 9, DT_RIGHT | DT_VCENTER, true);
    Text("Castle owner", { 8, 43, 100, 61 }, COL_LABEL, 8, DT_LEFT | DT_VCENTER);
    Text(s.owner.empty() ? "-" : s.owner, { 100, 43, MN_W - 8, 61 }, COL_VALUE, 8, DT_RIGHT | DT_VCENTER, true);
    if (folded) return;

    int y = MN_HEAD;
    Fill({ 1, y, MN_W - 1, y + 20 }, 52, 40, 24, 220);
    Text("#", { 8, y, 28, y + 20 }, COL_DIM, 8, DT_LEFT | DT_VCENTER);
    Text("Clan", { 30, y, 170, y + 20 }, COL_DIM, 8, DT_LEFT | DT_VCENTER);
    Text("Kills", { 170, y, MN_W - 8, y + 20 }, COL_DIM, 8, DT_RIGHT | DT_VCENTER);
    y += 20;
    if (!rows)
        Text(s.war ? "No kills yet" : "Waiting for the war", { 8, y, MN_W - 8, y + MN_ROW }, COL_DIM, 8, DT_CENTER | DT_VCENTER);
    for (int i = 0; i < rows; i++, y += MN_ROW)
    {
        const bool me = !s.mine.empty() && s.rows[i].clan == s.mine;
        if (me) Fill({ 1, y, MN_W - 1, y + MN_ROW }, 40, 170, 160, 70);
        else if (i & 1) Fill({ 1, y, MN_W - 1, y + MN_ROW }, 255, 255, 255, 14);
        const COLORREF c = i == 0 ? RGB(255, 214, 120) : i < 3 ? RGB(235, 235, 235) : RGB(200, 200, 200);
        char b[16];
        sprintf_s(b, "%d", i + 1);          Text(b, { 8, y, 28, y + MN_ROW }, c, 8, DT_LEFT | DT_VCENTER, i == 0);
        Text(s.rows[i].clan, { 30, y, 170, y + MN_ROW }, c, 8, DT_LEFT | DT_VCENTER, i == 0);
        sprintf_s(b, "%u", s.rows[i].kills); Text(b, { 170, y, MN_W - 8, y + MN_ROW }, c, 8, DT_RIGHT | DT_VCENTER, i == 0);
    }

    const Rc ob = MiniOpenBtn(g_curH);
    const bool ho = g_hover == C_OPEN, down = ho && g_pressed == C_OPEN;
    Fill(ob, down ? 60 : ho ? 150 : 96, down ? 40 : ho ? 100 : 62, down ? 20 : 30, 255);
    Frame(ob, 168, 128, 64, 255);
    Text("Ranking Board", ob, COL_VALUE, 8, DT_CENTER | DT_VCENTER, true);
}

static void PaintBoard(const CswState& s)
{
    g_curW = CWS_W; g_curH = CWS_H;
    if (!Skin("frame", 0, 0))
    {
        Fill({ 0, 0, CWS_W, CWS_H }, 20, 22, 18, 235);
        Frame({ 0, 0, CWS_W, CWS_H }, 120, 130, 110, 255);
    }
    const bool hc = g_hover == C_CLOSE;
    if (!Skin(hc && g_pressed == C_CLOSE ? "close_d" : hc ? "close_o" : "close_n", BD_CLOSE.l, BD_CLOSE.t))
    {
        Fill(BD_CLOSE, hc ? 200 : 150, 70, 30, 255);
        Text("x", BD_CLOSE, COL_VALUE, 10, DT_CENTER | DT_VCENTER, true);
    }

    // the uif's own static strings and colours ("Barracks" -> the clan's deaths, "Remaining Barracks" -> stage time)
    const COLORREF head = RGB(0xD3, 0x7A, 0x49);
    Text("Participant Clans", R4(CWS_TITLE), RGB(0xF9, 0xEC, 0xB3), 9, DT_CENTER | DT_VCENTER);
    Text("Rank", R4(CWS_H_RANK), head, 9, DT_CENTER | DT_VCENTER);
    Text("Icon", R4(CWS_H_ICON), head, 9, DT_CENTER | DT_VCENTER);
    Text("Clan", R4(CWS_H_CLAN), head, 9, DT_CENTER | DT_VCENTER);
    Text("Kill", R4(CWS_H_KILL), RGB(0x00, 0x80, 0xFF), 8, DT_CENTER | DT_VCENTER);
    Text("Death", { CWS_ROW_DEATH[0] - 12, CWS_H_DEATH[1], CWS_ROW_DEATH[2] + 12, CWS_H_DEATH[3] }, RGB(0xFF, 0x30, 0x30), 8, DT_CENTER | DT_VCENTER);

    const int rows = min((int)s.rows.size(), CWS_ROWS);
    for (int i = 0; i < rows; i++)
    {
        const CswRow& r = s.rows[i];
        const bool me = !s.mine.empty() && r.clan == s.mine;
        if (me)
        {
            const Rc bg = Row(CWS_ROW_MYSELF_BG, i);
            if (!Skin("row_me", bg.l, bg.t)) Fill(bg, 40, 170, 160, 90);
        }
        char b[16];
        sprintf_s(b, "%d", i + 1);
        Text(b, Row(CWS_ROW_RANK, i), i == 0 ? RGB(0xFF, 0xFF, 0x80) : COL_VALUE, 9, DT_CENTER | DT_VCENTER);
        CswMark mark;
        EnterCriticalSection(&g_lock);
        if (auto it = g_marks.find(r.id); it != g_marks.end() && it->second.ver == r.mark) mark = it->second;
        LeaveCriticalSection(&g_lock);
        DrawMark(mark, Row(CWS_ROW_MARK, i));
        const bool own = !s.owner.empty() && r.clan == s.owner;   // castle owner: gold name
        Text(r.clan, Row(CWS_ROW_NAME, i), own ? COL_TITLE : COL_VALUE, 9, DT_CENTER | DT_VCENTER, me);
        sprintf_s(b, "%u", r.kills);  Text(b, Row(CWS_ROW_KILL, i), RGB(0x80, 0x80, 0xFF), 9, DT_CENTER | DT_VCENTER);
        sprintf_s(b, "%u", r.deaths); Text(b, Row(CWS_ROW_DEATH, i), RGB(0xFF, 0x80, 0x80), 9, DT_CENTER | DT_VCENTER);
    }
    if (!rows)
        Text(s.war ? "No kills yet" : "Waiting for the war", Row(CWS_ROW_NAME, 0), COL_DIM, 9, DT_CENTER | DT_VCENTER);

    const UINT32 left = Left(s.secs, s.secsAt);
    Text(s.war ? "War Ends In:" : "War Starts In:", R4(CWS_FOOT_LABEL), RGB(0xFF, 0x9B, 0x6A), 9, DT_RIGHT | DT_VCENTER);
    Text(MMSS(left), R4(CWS_FOOT_VALUE), left <= 60 ? RGB(255, 120, 100) : RGB(0x80, 0xFF, 0x80), 8, DT_CENTER | DT_VCENTER);
}

static void Paint()
{
    if (!g_hWnd || !g_memDC) return;
    memset(g_bits, 0, (size_t)BUF_W * BUF_H * 4);
    LoadSkin();

    EnterCriticalSection(&g_lock);
    CswState s = g_st;
    bool folded = g_folded;
    int mode = g_mode;
    LeaveCriticalSection(&g_lock);

    if (mode == M_BOARD) PaintBoard(s); else PaintMini(s, folded);

    RECT wr; GetWindowRect(g_hWnd, &wr);
    if (wr.right - wr.left != g_curW || wr.bottom - wr.top != g_curH)
        SetWindowPos(g_hWnd, nullptr, 0, 0, g_curW, g_curH, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { g_curW, g_curH };
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
    if (g_miniPos.x < 0)    // right edge, under the premium list
    {
        g_miniPos.x = gr.right - MN_W - 14;
        g_miniPos.y = gr.top + 150;
    }
    if (g_boardPos.x < 0)   // centred
    {
        g_boardPos.x = gr.left + ((gr.right - gr.left) - CWS_W) / 2;
        g_boardPos.y = gr.top + max(0, ((gr.bottom - gr.top) - CWS_H) / 2);
    }
    const POINT p = g_mode == M_BOARD ? g_boardPos : g_miniPos;
    SetWindowPos(g_hWnd, HWND_TOPMOST, p.x, p.y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
}

static void SetMode(int mode)
{
    EnterCriticalSection(&g_lock);
    g_mode = mode;
    LeaveCriticalSection(&g_lock);
    g_hover = g_pressed = C_NONE;
    Place();
    Paint();
}

static void SyncVisibility()
{
    HWND game = FindGameWindow();
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    EnterCriticalSection(&g_lock);
    bool active = g_st.active;
    LeaveCriticalSection(&g_lock);
    bool want = active && game && !IsIconic(game) && pid == GetCurrentProcessId();
    if (want != (IsWindowVisible(g_hWnd) != FALSE))
    {
        if (want) { Place(); SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); Paint(); }
        else ShowWindow(g_hWnd, SW_HIDE);
    }
    POINT p; CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) &&
        (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursor = ci.hCursor;

    static DWORD lastSec = 0;   // countdown
    if (want && GetTickCount() - lastSec >= 1000) { lastSec = GetTickCount(); Paint(); }
}

static bool g_track = false, g_drag = false;
static POINT g_dragFrom;

static void Activate(int ctl)
{
    if (ctl == C_FOLD)
    {
        EnterCriticalSection(&g_lock);
        g_folded = !g_folded;
        LeaveCriticalSection(&g_lock);
        Paint();
    }
    else if (ctl == C_OPEN)  SetMode(M_BOARD);
    else if (ctl == C_CLOSE) SetMode(M_MINI);
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
                SetWindowPos(h, nullptr, wr.left + p.x - g_dragFrom.x, wr.top + p.y - g_dragFrom.y, 0, 0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
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
    case WM_LBUTTONDOWN:
    {
        int hit = HitTest(x, y);
        SetCapture(h);
        if (hit == C_NONE && y < (g_mode == M_BOARD ? CWS_H_RANK[1] : MN_HEAD))   // header drags the panel
        {
            g_drag = true; GetCursorPos(&g_dragFrom); return 0;
        }
        g_pressed = hit;
        if (hit != C_NONE) Paint();
        return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (g_drag)
        {
            g_drag = false;
            RECT wr; GetWindowRect(h, &wr);
            (g_mode == M_BOARD ? g_boardPos : g_miniPos) = { wr.left, wr.top };
            return 0;
        }
        int hit = HitTest(x, y), pressed = g_pressed;
        g_pressed = C_NONE;
        if (pressed != C_NONE && pressed == hit) Activate(hit);
        else Paint();
        return 0;
    }
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER: SyncVisibility(); return 0;
    case WM_CW_REFRESH:
        SyncVisibility();
        if (IsWindowVisible(h)) Paint();
        return 0;
    case WM_CLOSE: ShowWindow(h, SW_HIDE); return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static DWORD WINAPI WindowThread(LPVOID)
{
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = L"NTT_CswBoard";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Castle Siege War", WS_POPUP, 0, 0, MN_W, MN_HEAD, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWnd) { Log("CSW: window creation failed"); return 0; }

    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), BUF_W, -BUF_H, 1, 32, BI_RGB };
    HBITMAP bmp = CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0);
    SelectObject(g_memDC, bmp);
    Log("CSW: window ready");
    SetTimer(g_hWnd, 1, 150, nullptr);

    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// ---------------------------------------------------------------- recv (game thread: parse only, UI via PostMessage)
template <class T> static bool Get(const BYTE* p, size_t len, size_t& pos, T& v)
{
    if (pos + sizeof(T) > len) return false;
    memcpy(&v, p + pos, sizeof(T)); pos += sizeof(T); return true;
}

static bool GetStr(const BYTE* p, size_t len, size_t& pos, std::string& s)
{
    BYTE n = 0;
    if (!Get(p, len, pos, n) || pos + n > len) return false;
    s.assign((const char*)p + pos, n); pos += n;
    while (!s.empty() && (s.back() == ' ' || s.back() == '\0')) s.pop_back();
    return true;
}

void Csw_OnRecv(const BYTE* buf, size_t len)
{
    if (len < 3 || buf[0] != WIZ_HSACS_HOOK || buf[1] != CSW_SUBOP) return;
    size_t pos = 3;
    char msg[160] = {};

    EnterCriticalSection(&g_lock);
    if (buf[2] == WO_hide)
    {
        if (g_st.active) strcpy_s(msg, "CSW recv: hide");
        g_st = CswState();
        g_mode = M_MINI;
    }
    else if (buf[2] == WO_board)
    {
        CswState s; BYTE status = 0, n = 0;
        bool ok = Get(buf, len, pos, status) && Get(buf, len, pos, s.secs) && GetStr(buf, len, pos, s.owner) &&
                  GetStr(buf, len, pos, s.mine) && Get(buf, len, pos, n);
        for (int i = 0; ok && i < n; i++)
        {
            CswRow r;
            ok = Get(buf, len, pos, r.id) && GetStr(buf, len, pos, r.clan) && Get(buf, len, pos, r.kills) &&
                 Get(buf, len, pos, r.deaths) && Get(buf, len, pos, r.mark);
            if (ok) s.rows.push_back(r);
        }
        for (const CswRow& r : s.rows)   // marks we have not got (or an older version of): ask once, again after 15 s
        {
            if (!ok || !r.mark) continue;
            CswMark& m = g_marks[r.id];
            const DWORD now = GetTickCount();
            if (m.ver == r.mark && (!m.px.empty() || (m.askedAt && now - m.askedAt < 15000))) continue;
            m.ver = r.mark; m.px.clear(); m.askedAt = now ? now : 1;
            QueueSend({ WIZ_HSACS_HOOK, CSW_SUBOP, WO_mark, (BYTE)(r.id & 0xFF), (BYTE)(r.id >> 8) });
        }
        if (ok)
        {
            s.active = true; s.war = status == 2; s.secsAt = GetTickCount();
            if (!g_st.active || g_st.war != s.war)
                sprintf_s(msg, "CSW recv: board status=%u secs=%u clans=%u mine=%s", status, s.secs, n, s.mine.c_str());
            g_st = s;
        }
        else
            strcpy_s(msg, "CSW recv: short board packet");
    }
    else if (buf[2] == WO_mark)
    {
        UINT16 id = 0, ver = 0, n = 0;
        if (Get(buf, len, pos, id) && Get(buf, len, pos, ver) && Get(buf, len, pos, n) && pos + n <= len)
        {
            CswMark m; m.ver = ver;
            const bool ok = DecodeMark(buf + pos, n, m);
            m.askedAt = ok ? GetTickCount() : 0;
            g_marks[id] = m;
            if (!ok) g_marks[id].askedAt = GetTickCount() + 0x70000000;   // unreadable texture: do not ask again
            sprintf_s(msg, "CSW recv: mark clan=%u ver=%u len=%u -> %s", id, ver, n, ok ? "ok" : "unsupported texture");
        }
    }
    LeaveCriticalSection(&g_lock);
    if (msg[0]) Log(msg);
    if (g_hWnd) PostMessageW(g_hWnd, WM_CW_REFRESH, 0, 0);
}

// ---------------------------------------------------------------- exports
void Csw_Init()
{
    InitializeCriticalSection(&g_lock);
    InitializeCriticalSection(&g_sendLock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// pus_store.cpp's GetAsyncKeyState hook: clicks on our panel must not move the character
bool Csw_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
