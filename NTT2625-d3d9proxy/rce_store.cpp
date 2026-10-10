// Right-Click Item Exchange panel for the XIGNCODE 2625 client, loaded through the
// d3d9 proxy as a sibling of the Power Up Store (pus_store.cpp). GDI layered overlay
// skinned from Desktop\uifler\re_rightclickexchange.uif (build_rce_assets.py -> rce_ui\*.pus).
// Server side: GameServer XGuard.cpp HandleRightClickExchange (WIZ_HSACS_HOOK 0xE9 + 0xE6).
//
// Flow: right-click an exchange coupon in the inventory -> hook on the inventory's native
// right-click slot handler (same technique as the recv hook: 6-byte jmp, found at runtime by
// signature) -> [E9 E6 sub 14+slot itemID] -> server answers with the reward list
// -> panel opens -> pick a reward (type 1) -> Exchange.
//
//   S->C op 0 : 28*[u8 enabled][u8 subCode][u32 itemID]    (inventory coupon slots)
//   S->C op 1 : mode 1 [u32 item] -> we request the list | mode 2 [u8 slot][u32 item] (generator)
//   S->C op 2 : [u8 type][u16 n] n*[u32 coupon]            (coupon -> exchange type)
//   S->C op 3 : [u8 type][u8 type][u32 coupon] 25*[u32 item][u32 time]
//               [u32 couponIcon] 25*[u32 icon] [u8 1][str coupon] 25*([str name][u32 count])
//               op 3 sub 5 = exchange done
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <map>
#include <deque>
#include <algorithm>

// two skins on one panel: right-click exchange (rce_ui, rce_layout.h) and Cross Exchange (cross_ui, cross_layout.h,
// opened by the inventory's Cross Exchange button). g_crossR picks one; it only changes on the window thread.
struct RceRect { int l, t, r, b; };
struct RceSprite { const char* name; RceRect rc; };
#define RCE_TYPES
namespace RceO {
#include "rce_layout.h"
}
namespace RceX {
#include "cross_layout.h"
}
static bool g_crossR = false;
static const int RCE_SLOT_COUNT = RceO::RCE_SLOT_COUNT;
static const int RCE_MAX_W = RceO::RCE_W > RceX::RCE_W ? RceO::RCE_W : RceX::RCE_W;
static const int RCE_MAX_H = RceO::RCE_H > RceX::RCE_H ? RceO::RCE_H : RceX::RCE_H;
#define RCE_W            (g_crossR ? RceX::RCE_W : RceO::RCE_W)
#define RCE_H            (g_crossR ? RceX::RCE_H : RceO::RCE_H)
#define RCE_COUPON       (g_crossR ? RceX::RCE_COUPON : RceO::RCE_COUPON)
#define RCE_TITLE        (g_crossR ? RceX::RCE_TITLE : RceO::RCE_TITLE)
#define RCE_BTN_EXCHANGE (g_crossR ? RceX::RCE_BTN_EXCHANGE : RceO::RCE_BTN_EXCHANGE)
#define RCE_BTN_CLOSE    (g_crossR ? RceX::RCE_BTN_CLOSE : RceO::RCE_BTN_CLOSE)
#define RCE_SLOTS        (g_crossR ? RceX::RCE_SLOTS : RceO::RCE_SLOTS)
using RceX::RCE_INV; using RceX::RCE_KC_LABEL; using RceX::RCE_KC; using RceX::RCE_TL_LABEL; using RceX::RCE_TL;
using RceX::RCE_GOLD; using RceX::RCE_RESULT; using RceX::RCE_WEIGHT; using RceX::RCE_NOAH;
static const RceSprite* SpritesR(int& n)
{
    if (g_crossR) { n = (int)(sizeof(RceX::RCE_SPRITES) / sizeof(RceX::RCE_SPRITES[0])); return RceX::RCE_SPRITES; }
    n = (int)(sizeof(RceO::RCE_SPRITES) / sizeof(RceO::RCE_SPRITES[0])); return RceO::RCE_SPRITES;
}
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();
void Rce_OpenCross();

// --- Verified addresses (shared with pus_store.cpp) ---------------------------
static const DWORD KO_SND_FNC = 0x00704070; // unpack: thiscall(CAPISocket*, BYTE* buf, int len)
static const DWORD KO_PTR_PKT = 0x01115914; // unpack: CAPISocket* (game socket)
static const DWORD KO_RECV_FNC = 0x0084D0B0; // unpack: CGameProcMain vtable[+8] ProcessPacket

static const BYTE WIZ_HSACS_HOOK = 0xE9;
static const BYTE RCE_SUBOP      = 0xE6;    // WIZ_ITEM_EXCHANGE_INFO
static const BYTE OP_OPEN      = 1;
static const BYTE OP_TYPELIST  = 2;
static const BYTE OP_OPENED    = 3;
static const BYTE OP_GIVE      = 4;
static const BYTE OP_GENGIVE   = 5;

static bool IsRewardType(BYTE t) { return t == 1 || t == 2 || t == 3 || t == 4 || t == 6 || t == 7; }

// --- Outgoing queue: flushed on the game thread (inside the recv hook) --------
static CRITICAL_SECTION g_sendLockR;
static std::deque<std::vector<BYTE>> g_sendQueueR;

static void QueueSendR(const std::vector<BYTE>& pkt)
{
    EnterCriticalSection(&g_sendLockR);
    g_sendQueueR.push_back(pkt);
    LeaveCriticalSection(&g_sendLockR);
    Proxy_RequestFlush();   // flush on the game thread now, not on the next received packet
}

void Rce_FlushSendQueue()
{
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_sendLockR);
    pending.swap(g_sendQueueR);
    LeaveCriticalSection(&g_sendLockR);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
        char buf[96];
        sprintf_s(buf, "RCE send: %02X %02X %02X len=%u", p[0], p.size() > 1 ? p[1] : 0, p.size() > 2 ? p[2] : 0, (unsigned)p.size());
        Log(buf);
    }
}

// --- Packet reader --------------------------------------------------------------
struct ReaderR
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
        if (!ok || pos + n > len) { ok = false; return std::string(); }
        std::string s((const char*)p + pos, n); pos += n; return s;
    }
    size_t remain() const { return pos <= len ? len - pos : 0; }
};

// --- State ----------------------------------------------------------------------
static const int SLOT_MAX_CLIENT = 14;      // inventory equip-slot offset (server SLOT_MAX)
static const int HAVE_MAX_CLIENT = 28;      // main inventory slot count

struct Reward { UINT32 itemID; UINT32 rentalTime; UINT32 iconID; UINT32 count; std::string name; std::vector<std::pair<BYTE, std::string>> lines; };

enum View { VIEW_REWARD = 1, VIEW_GENERATOR = 2 };

static CRITICAL_SECTION g_dataLockR;
static BYTE   g_slotEnabled[HAVE_MAX_CLIENT] = {};
static BYTE   g_slotSubCode[HAVE_MAX_CLIENT] = {};
static UINT32 g_slotItemID[HAVE_MAX_CLIENT]  = {};
static UINT32 g_invIcon[HAVE_MAX_CLIENT] = {};          // Cross Exchange: inventory drawn in the panel (op 0 tail)
static UINT16 g_invCount[HAVE_MAX_CLIENT] = {};
static UINT32 g_weight = 0, g_maxWeight = 0, g_gold = 0;
static DWORD  g_statusAtR = 0;
static std::vector<UINT32> g_typeList[8];             // OP_TYPELIST: type -> coupon itemIDs
static Reward g_rewards[RCE_SLOT_COUNT];
static UINT32 g_baseIcon = 0;
static std::string g_baseName;
static UINT32 g_openBaseItem = 0;
static BYTE   g_exchangeType = 0;
static UINT32 g_selectedReward = 0;
static int    g_sourceSlot = -1;                      // generator source slot
static UINT32 g_genCount = 0;                         // generator: pieces of the source item left
static UINT32 g_batchSize = 1;
static bool g_batchAll = false, g_runAll = false, g_generatorBusy = false;
static DWORD g_generatorSentAt = 0;
static volatile LONG g_inventoryRefreshPending = 0;
static UINT32 g_lastReward = 0, g_lastRewardIcon = 0; // generator: last item received
static std::string g_lastRewardName;
static int    g_view = VIEW_REWARD;
static std::string g_statusR;

static HWND g_hWndR = nullptr;
static HDC  g_memDCR = nullptr;
static void* g_bitsR = nullptr;
// current drawing target (panel or tooltip) for Fill / TextR / BlitR
static HDC   g_cvDC = nullptr;
static void* g_cvBits = nullptr;
static int   g_cvW = 0, g_cvH = 0;
static HWND  g_hWndTip = nullptr;
static HDC   g_memDCTip = nullptr;
static void* g_bitsTip = nullptr;
static const int TIP_W = 280, TIP_MAXH = 760;
static char g_uiDirR[MAX_PATH];                       // pus_ui (item icons)
static char g_skinDirR[MAX_PATH];                     // rce_ui (right-click exchange skin)
static char g_skinDirX[MAX_PATH];                     // cross_ui (Cross Exchange skin)

static const UINT WM_RCE_REFRESH = WM_APP + 11;
static const UINT WM_RCE_SHOW    = WM_APP + 12;

// ---------------------------------------------------------------- sprites
struct SpriteR { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
static std::map<UINT32, SpriteR> g_iconsR;
static std::map<std::string, SpriteR> g_skinMapR[2];   // [0] rce_ui, [1] cross_ui
#define g_skinR g_skinMapR[g_crossR ? 1 : 0]
static SpriteR g_noImage;
static bool g_noImageTried = false, g_skinLoadedArr[2] = {};
#define g_skinLoadedR g_skinLoadedArr[g_crossR ? 1 : 0]

static bool LoadPusR(const char* path, SpriteR& s)
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

static SpriteR* NoImage()
{
    if (!g_noImageTried)
    {
        g_noImageTried = true;
        char path[MAX_PATH]; sprintf_s(path, "%s\\noimage.pus", g_uiDirR);
        if (!LoadPusR(path, g_noImage)) g_noImage = SpriteR();
    }
    return g_noImage.dc ? &g_noImage : nullptr;
}

static SpriteR* IconR(UINT32 iconId)
{
    if (!iconId) return NoImage();
    auto it = g_iconsR.find(iconId);
    if (it != g_iconsR.end()) return it->second.dc ? &it->second : NoImage();
    SpriteR s;
    char path[MAX_PATH];
    sprintf_s(path, "%s\\icons\\%u.pus", g_uiDirR, iconId);
    if (!LoadPusR(path, s)) s = SpriteR();
    g_iconsR[iconId] = s;
    return s.dc ? &g_iconsR[iconId] : NoImage();
}

static void LoadSkinR()
{
    if (g_skinLoadedR) return;
    g_skinLoadedR = true;
    int ok = 0;
    int n = 0; const RceSprite* sp0 = SpritesR(n);
    for (int k = 0; k < n; k++)
    {
        const RceSprite& s = sp0[k];
        char path[MAX_PATH]; sprintf_s(path, "%s\\%s.pus", g_crossR ? g_skinDirX : g_skinDirR, s.name);
        SpriteR sp;
        if (LoadPusR(path, sp)) { g_skinR[s.name] = sp; ok++; }
    }
    char msg[80]; sprintf_s(msg, "RCE: %s skin sprites loaded %d/%d", g_crossR ? "cross" : "rce", ok, n);
    Log(msg);
}

static void BlitR(SpriteR* s, int x, int y, int w, int h)
{
    if (!s) return;
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    AlphaBlend(g_cvDC, x, y, w, h, s->dc, 0, 0, s->w, s->h, bf);
}

static bool SkinR(const char* name)
{
    auto it = g_skinR.find(name);
    if (it == g_skinR.end()) return false;
    int n = 0; const RceSprite* sp0 = SpritesR(n);
    for (int k = 0; k < n; k++)
        if (const RceSprite& s = sp0[k]; strcmp(s.name, name) == 0)
        {
            BlitR(&it->second, s.rc.l, s.rc.t, s.rc.r - s.rc.l, s.rc.b - s.rc.t);
            return true;
        }
    return false;
}

// Premultiplied-BGRA src-over fill into the frame buffer (same convention as TextR).
static void Fill(RceRect r, BYTE cr, BYTE cg, BYTE cb, BYTE a)
{
    int l = max(0, r.l), t = max(0, r.t), rr = min(g_cvW, r.r), bb = min(g_cvH, r.b);
    for (int y = t; y < bb; y++)
    {
        BYTE* dst = (BYTE*)g_cvBits + (y * g_cvW + l) * 4;
        for (int x = l; x < rr; x++, dst += 4)
        {
            dst[0] = (BYTE)((cb * a + dst[0] * (255 - a)) / 255);
            dst[1] = (BYTE)((cg * a + dst[1] * (255 - a)) / 255);
            dst[2] = (BYTE)((cr * a + dst[2] * (255 - a)) / 255);
            dst[3] = (BYTE)(a + dst[3] * (255 - a) / 255);
        }
    }
}

static void Border(RceRect r, BYTE cr, BYTE cg, BYTE cb, BYTE a, int th = 1)
{
    Fill({ r.l, r.t, r.r, r.t + th }, cr, cg, cb, a);
    Fill({ r.l, r.b - th, r.r, r.b }, cr, cg, cb, a);
    Fill({ r.l, r.t, r.l + th, r.b }, cr, cg, cb, a);
    Fill({ r.r - th, r.t, r.r, r.b }, cr, cg, cb, a);
}

// GDI text into a scratch DIB, coverage used as alpha (see pus_store.cpp Text()).
static void TextR(const std::string& s, RceRect r, COLORREF color, int pt, UINT fmt, bool bold = false, const char* face = "Verdana")
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
        OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, face);
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
        if (dy < 0 || dy >= g_cvH) continue;
        const BYTE* src = (const BYTE*)bits + y * w * 4;
        BYTE* dst = (BYTE*)g_cvBits + (dy * g_cvW + r.l) * 4;
        for (int x = 0; x < w; x++, src += 4, dst += 4)
        {
            int a = max(src[0], max(src[1], src[2]));
            if (!a || r.l + x < 0 || r.l + x >= g_cvW) continue;
            dst[0] = (BYTE)((cb * a + dst[0] * (255 - a)) / 255);
            dst[1] = (BYTE)((cg * a + dst[1] * (255 - a)) / 255);
            dst[2] = (BYTE)((cr * a + dst[2] * (255 - a)) / 255);
            dst[3] = (BYTE)(a + dst[3] * (255 - a) / 255);
        }
    }
    SelectObject(dc, of); DeleteObject(font);
    SelectObject(dc, ob); DeleteObject(bmp); DeleteDC(dc);
}

// ---------------------------------------------------------------- controls / hit test
enum CtlR { R_NONE, R_CLOSE, R_EXCHANGE, R_SLOT0, R_SLOTN = R_SLOT0 + RCE_SLOT_COUNT, R_INV0 = R_SLOTN, R_INVN = R_INV0 + HAVE_MAX_CLIENT,
    R_BATCH1, R_BATCH10, R_BATCH100, R_BATCHALL };
static RceRect BatchRect(int index) { return { 110 + index * 52, RCE_BTN_EXCHANGE.t - 27, 160 + index * 52, RCE_BTN_EXCHANGE.t - 4 }; }
static int g_hoverR = R_NONE, g_pressedR = R_NONE;
static bool g_trackingR = false, g_draggingR = false;
static POINT g_dragFromR;

static bool In(RceRect r, int x, int y) { return x >= r.l && x < r.r && y >= r.t && y < r.b; }

static int HitTestR(int x, int y)
{
    if (g_view == VIEW_GENERATOR)
        for (int i = 0; i < 4; i++) if (In(BatchRect(i), x, y)) return R_BATCH1 + i;
    if (In(RCE_BTN_EXCHANGE, x, y)) return R_EXCHANGE;
    if (In(RCE_BTN_CLOSE, x, y)) return R_CLOSE;
    if (g_view == VIEW_REWARD)
        for (int i = 0; i < RCE_SLOT_COUNT; i++)
            if (g_rewards[i].itemID && In(RCE_SLOTS[i], x, y)) return R_SLOT0 + i;
    if (g_crossR)
        for (int i = 0; i < HAVE_MAX_CLIENT; i++)
            if (In(RCE_INV[i], x, y)) return R_INV0 + i;
    return R_NONE;
}

static std::string RewardLabel(const Reward& r)
{
    std::string s = r.name.empty() ? "Item " + std::to_string(r.itemID) : r.name;
    if (r.count > 1) s += "  x" + std::to_string(r.count);
    if (r.rentalTime) s += "  (" + std::to_string(r.rentalTime) + (r.rentalTime == 1 ? " day)" : " days)");
    return s;
}

// ---------------------------------------------------------------- rendering
static std::string ThousandsR(UINT32 v)
{
    char raw[16]; sprintf_s(raw, "%u", v);
    std::string t = raw, out;
    for (size_t i = 0; i < t.size(); i++) { if (i && (t.size() - i) % 3 == 0) out += '.'; out += t[i]; }
    return out;
}

static void ButtonR(const char* sprite, RceRect r, int ctl, const char* label)
{
    bool hov = g_hoverR == ctl, down = hov && g_pressedR == ctl;
    char name[32]; sprintf_s(name, "%s_%s", sprite, down ? "d" : hov ? "o" : "n");
    if (!SkinR(name))
    {
        Fill(r, hov ? 90 : 60, 30, 36, 255);
        Border(r, 200, 160, 90, 255);
    }
    TextR(label, r, RGB(255, 255, 255), 10, DT_CENTER | DT_VCENTER, true);
}

// the frame buffer is exactly RCE_W x RCE_H of the current skin (Paint and UpdateLayeredWindow use RCE_W as stride)
static HBITMAP g_bmpR = nullptr;
static void MakeDibR()
{
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), RCE_W, -RCE_H, 1, 32, BI_RGB };
    void* bits = nullptr;
    HBITMAP bmp = CreateDIBSection(g_memDCR, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    if (!bmp) return;
    SelectObject(g_memDCR, bmp);
    if (g_bmpR) DeleteObject(g_bmpR);
    g_bmpR = bmp; g_bitsR = bits;
}

static void SetModeR(bool cross)   // window thread only
{
    if (cross == g_crossR) return;
    g_crossR = cross;
    MakeDibR();
    RECT wr; GetWindowRect(g_hWndR, &wr);
    SetWindowPos(g_hWndR, nullptr, wr.left, wr.top, RCE_W, RCE_H, SWP_NOZORDER | SWP_NOACTIVATE);
}

static void Paint()
{
    if (!g_hWndR || !g_memDCR) return;
    memset(g_bitsR, 0, (size_t)RCE_W * RCE_H * 4);
    g_cvDC = g_memDCR; g_cvBits = g_bitsR; g_cvW = RCE_W; g_cvH = RCE_H;
    LoadSkinR();

    EnterCriticalSection(&g_dataLockR);
    if (!SkinR("bg"))
    {
        Fill({ 20, 20, RCE_W - 20, RCE_H - 20 }, 18, 20, 28, 235);
        Border({ 20, 20, RCE_W - 20, RCE_H - 20 }, 120, 96, 40, 255, 2);
    }

    // coupon + title
    SkinR("coupon");
    if (g_openBaseItem)
        BlitR(IconR(g_baseIcon), RCE_COUPON.l + 1, RCE_COUPON.t + 1, (RCE_COUPON.r - RCE_COUPON.l) - 2, (RCE_COUPON.b - RCE_COUPON.t) - 2);
    if (g_crossR)
        TextR("Cross Exchange", RCE_TITLE, RGB(255, 255, 255), 11, DT_CENTER | DT_VCENTER, true);
    else
    {
    std::string title = g_baseName.empty() ? "Item " + std::to_string(g_openBaseItem) : g_baseName;
    title += g_view == VIEW_GENERATOR ? "  Exchange" : "  Contents";
    TextR(title, RCE_TITLE, RGB(255, 255, 255), 12, DT_LEFT | DT_VCENTER, true, "Calibri");
    }

    const Reward* info = nullptr;
    if (g_view == VIEW_REWARD)
    {
        for (int i = 0; i < RCE_SLOT_COUNT; i++)
        {
            const Reward& rw = g_rewards[i];
            if (!rw.itemID) continue;
            RceRect s = RCE_SLOTS[i];
            BlitR(IconR(rw.iconID), s.l, s.t, s.r - s.l, s.b - s.t);
            bool sel = g_exchangeType == 1 && rw.itemID == g_selectedReward;
            bool hov = g_hoverR == R_SLOT0 + i;
            if (sel) Border({ s.l - 2, s.t - 2, s.r + 2, s.b + 2 }, 255, 210, 80, 255, 2);
            else if (hov) Border({ s.l - 1, s.t - 1, s.r + 1, s.b + 1 }, 230, 230, 230, 200);
            if (rw.count > 1)
                TextR(std::to_string(rw.count), { s.l, s.b - 14, s.r - 2, s.b }, RGB(255, 255, 255), 8, DT_RIGHT | DT_BOTTOM, true);
            if (hov) info = &rw;
            else if (sel && !info) info = &rw;
        }
        if (g_hoverR >= R_SLOT0 && g_hoverR < R_SLOTN) info = &g_rewards[g_hoverR - R_SLOT0];
    }
    else
    {
        // generator (gem / fragment break): one piece -> one random item; last result in the centre cell
        RceRect mid = RCE_SLOTS[RCE_SLOT_COUNT / 2];
        const char* labels[4] = { "1", "10", "100", "Tumu" };
        for (int i = 0; i < 4; i++)
            ButtonR("exchange", BatchRect(i), R_BATCH1 + i, labels[i]);
        TextR("Adet secin; Tumu kalan taslari sirayla kirar.", { 40, RCE_SLOTS[0].t, RCE_W - 40, RCE_SLOTS[0].t + 30 },
              RGB(230, 230, 230), 11, DT_CENTER | DT_VCENTER);
        TextR("Pieces left : " + std::to_string(g_genCount), { 40, RCE_SLOTS[0].t + 34, RCE_W - 40, RCE_SLOTS[0].t + 60 },
              g_genCount ? RGB(255, 255, 160) : RGB(255, 110, 100), 10, DT_CENTER | DT_VCENTER, true);
        if (g_lastReward)
        {
            BlitR(IconR(g_lastRewardIcon), mid.l, mid.t, mid.r - mid.l, mid.b - mid.t);
            Border({ mid.l - 2, mid.t - 2, mid.r + 2, mid.b + 2 }, 255, 210, 80, 255, 2);
            TextR(g_lastRewardName, { 40, mid.b + 6, RCE_W - 40, mid.b + 26 }, RGB(255, 210, 90), 10, DT_CENTER | DT_VCENTER, true);
        }
    }

    if (g_crossR)
    {
    // chosen reward in the right cell, KC / TL / noah fields, the inventory below, weight + noah at the bottom
    for (int i = 0; i < RCE_SLOT_COUNT; i++)
        if (g_selectedReward && g_rewards[i].itemID == g_selectedReward)
        {
            BlitR(IconR(g_rewards[i].iconID), RCE_RESULT.l + 3, RCE_RESULT.t + 3, (RCE_RESULT.r - RCE_RESULT.l) - 6, (RCE_RESULT.b - RCE_RESULT.t) - 6);
            break;
        }
    UINT32 kc = 0, tl = 0;
    if (g_exchangeType == 4 || g_exchangeType == 7)
        for (int i = 0; i < RCE_SLOT_COUNT; i++) if (g_rewards[i].itemID) (g_exchangeType == 4 ? kc : tl) += g_rewards[i].count;
    TextR("KC", RCE_KC_LABEL, RGB(255, 120, 140), 8, DT_RIGHT | DT_VCENTER, true);
    TextR(std::to_string(kc), RCE_KC, RGB(255, 120, 140), 8, DT_CENTER | DT_VCENTER, true);
    TextR("TL", RCE_TL_LABEL, RGB(120, 230, 120), 8, DT_RIGHT | DT_VCENTER, true);
    TextR(std::to_string(tl), RCE_TL, RGB(120, 230, 120), 8, DT_CENTER | DT_VCENTER, true);
    TextR("0", RCE_GOLD, RGB(255, 255, 255), 8, DT_CENTER | DT_VCENTER, true);
    for (int i = 0; i < HAVE_MAX_CLIENT; i++)
    {
        RceRect c = RCE_INV[i];
        if (!g_slotItemID[i]) continue;
        BlitR(IconR(g_invIcon[i]), c.l + 1, c.t + 1, (c.r - c.l) - 2, (c.b - c.t) - 2);
        if (g_invCount[i] > 1)
            TextR(std::to_string(g_invCount[i]), { c.l, c.b - 14, c.r - 2, c.b }, RGB(255, 255, 255), 8, DT_RIGHT | DT_BOTTOM, true);
        bool ex = g_slotEnabled[i] != 0;
        if (ex) Border({ c.l, c.t, c.r, c.b }, 255, 200, 70, g_hoverR == R_INV0 + i ? 255 : 170, g_hoverR == R_INV0 + i ? 2 : 1);
        else if (g_hoverR == R_INV0 + i) Border({ c.l, c.t, c.r, c.b }, 200, 200, 200, 160);
    }
    TextR(ThousandsR(g_gold), RCE_NOAH, RGB(255, 255, 255), 8, DT_RIGHT | DT_VCENTER, true);
    if (!g_statusR.empty() && GetTickCount() - g_statusAtR < 5000)
        TextR(g_statusR, RCE_WEIGHT, RGB(255, 220, 120), 8, DT_LEFT | DT_VCENTER, true);
    else if (info)
        TextR(RewardLabel(*info), RCE_WEIGHT, RGB(255, 255, 160), 8, DT_LEFT | DT_VCENTER, true);
    else
    {
        char wb[64]; sprintf_s(wb, "Weight   %.1f/%.1f", g_weight / 10.0, g_maxWeight / 10.0);
        TextR(wb, RCE_WEIGHT, RGB(220, 220, 200), 8, DT_LEFT | DT_VCENTER, true);
    }
    ButtonR("exchange", RCE_BTN_EXCHANGE, R_EXCHANGE, "Exchange");
    ButtonR("close", RCE_BTN_CLOSE, R_CLOSE, "");
    LeaveCriticalSection(&g_dataLockR);
    }
    else
    {
    // info line between the grid and the buttons: hovered / selected reward, else hint / status
    RceRect line = { 40, RCE_SLOTS[20].b + 14, RCE_W - 40, RCE_BTN_EXCHANGE.t - 4 };
    if (!g_statusR.empty())
        TextR(g_statusR, line, RGB(255, 220, 120), 10, DT_CENTER | DT_VCENTER, true);
    else if (info)
        TextR(RewardLabel(*info), line, RGB(255, 255, 160), 10, DT_CENTER | DT_VCENTER, true);
    else if (g_view == VIEW_REWARD)
        TextR(g_exchangeType == 1 ? "Select one reward, then press Exchange." : "You will receive all items shown.",
              line, RGB(200, 200, 200), 9, DT_CENTER | DT_VCENTER);

    ButtonR("exchange", RCE_BTN_EXCHANGE, R_EXCHANGE, "Exchange");
    ButtonR("close", RCE_BTN_CLOSE, R_CLOSE, "Close");
    LeaveCriticalSection(&g_dataLockR);
    }

    RECT wr; GetWindowRect(g_hWndR, &wr);
    POINT dst = { wr.left, wr.top }, src = { 0, 0 };
    SIZE sz = { RCE_W, RCE_H };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWndR, nullptr, &dst, &sz, g_memDCR, &src, 0, &bf, ULW_ALPHA);
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

static int g_tipSlot = -1;

static void HideTip()
{
    g_tipSlot = -1;
    if (g_hWndTip) ShowWindow(g_hWndTip, SW_HIDE);
}

// show the stats of reward `slot` next to its cell (right of the panel cell, flipped left near the screen edge)
static void UpdateTip()
{
    int slot = (g_view == VIEW_REWARD && g_hoverR >= R_SLOT0 && g_hoverR < R_SLOTN) ? g_hoverR - R_SLOT0 : -1;
    if (!g_hWndTip || !g_memDCTip || !IsWindowVisible(g_hWndR)) { HideTip(); return; }
    if (slot < 0 || !g_rewards[slot].itemID) { HideTip(); return; }
    if (slot == g_tipSlot && IsWindowVisible(g_hWndTip)) return;
    g_tipSlot = slot;

    EnterCriticalSection(&g_dataLockR);
    Reward rw = g_rewards[slot];
    LeaveCriticalSection(&g_dataLockR);

    const int pad = 10, titleH = 22, lineH = 17;
    int h = min(TIP_MAXH, pad + titleH + 8 + (int)rw.lines.size() * lineH + (rw.lines.empty() ? lineH : 0) + pad);
    memset(g_bitsTip, 0, (size_t)TIP_W * TIP_MAXH * 4);
    g_cvDC = g_memDCTip; g_cvBits = g_bitsTip; g_cvW = TIP_W; g_cvH = h;

    RceRect box = { 0, 0, TIP_W, h };
    Fill(box, 12, 12, 16, 235);
    Border(box, 150, 120, 60, 255);
    Border({ 2, 2, TIP_W - 2, h - 2 }, 60, 48, 26, 255);
    std::string title = rw.name.empty() ? "Item " + std::to_string(rw.itemID) : rw.name;
    if (rw.count > 1) title += "  x" + std::to_string(rw.count);
    TextR(title, { pad, pad, TIP_W - pad, pad + titleH }, RGB(255, 210, 90), 10, DT_CENTER | DT_VCENTER, true);
    Fill({ pad, pad + titleH + 3, TIP_W - pad, pad + titleH + 4 }, 150, 120, 60, 255);
    int y = pad + titleH + 8;
    if (rw.lines.empty())
        TextR("No additional information.", { pad, y, TIP_W - pad, y + lineH }, RGB(170, 170, 170), 9, DT_CENTER | DT_VCENTER);
    for (auto& l : rw.lines)
    {
        if (y + lineH > h - pad + 2) break;
        TextR(l.second, { pad, y, TIP_W - pad, y + lineH }, TipColor(l.first), 9, DT_CENTER | DT_VCENTER);
        y += lineH;
    }

    RECT wr; GetWindowRect(g_hWndR, &wr);
    RceRect s = RCE_SLOTS[slot];
    int x = wr.left + s.r + 8, ty = wr.top + s.t;
    HMONITOR mon = MonitorFromWindow(g_hWndR, MONITOR_DEFAULTTONEAREST);
    MONITORINFO mi = { sizeof(mi) }; GetMonitorInfo(mon, &mi);
    if (x + TIP_W > mi.rcWork.right) x = wr.left + s.l - 8 - TIP_W;
    if (ty + h > mi.rcWork.bottom) ty = mi.rcWork.bottom - h;
    if (ty < mi.rcWork.top) ty = mi.rcWork.top;

    POINT dst = { x, ty }, src = { 0, 0 };
    SIZE sz = { TIP_W, h };
    BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(g_hWndTip, nullptr, &dst, &sz, g_memDCTip, &src, 0, &bf, ULW_ALPHA);
    SetWindowPos(g_hWndTip, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
}

// ---------------------------------------------------------------- protocol out
static void SendRewardInfo(UINT32 itemID)
{
    if (!itemID) return;
    BYTE type = 1;
    for (BYTE t = 1; t < 8; t++)
        if (std::find(g_typeList[t].begin(), g_typeList[t].end(), itemID) != g_typeList[t].end()) { type = t; break; }
    if (!IsRewardType(type)) type = 1;
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, RCE_SUBOP, OP_OPENED, type };
    p.insert(p.end(), (BYTE*)&itemID, (BYTE*)&itemID + 4);
    QueueSendR(p);
}

static void SendGive()
{
    if (g_view == VIEW_GENERATOR)
    {
        if (g_generatorBusy) { g_runAll = false; g_statusR = "Toplu kirma durduruldu."; return; }
        if (!g_openBaseItem || g_sourceSlot < 0) return;
        if (g_genCount == 0) { g_statusR = "You have no pieces left."; g_statusAtR = GetTickCount(); return; }
        std::vector<BYTE> p = { WIZ_HSACS_HOOK, RCE_SUBOP, OP_GENGIVE };
        p.insert(p.end(), (BYTE*)&g_openBaseItem, (BYTE*)&g_openBaseItem + 4);
        p.push_back((BYTE)g_sourceSlot); p.push_back(0); p.push_back(0);
        p.push_back((BYTE)min(g_genCount, g_batchAll ? 100U : g_batchSize));
        g_generatorBusy = true; g_generatorSentAt = GetTickCount();
        QueueSendR(p);
        g_statusR = "Exchanging..."; g_statusAtR = GetTickCount();
        return;
    }
    if (!g_openBaseItem || !IsRewardType(g_exchangeType)) return;
    if (g_exchangeType == 1 && g_selectedReward == 0) { g_statusR = "Select a reward first."; g_statusAtR = GetTickCount(); return; }
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, RCE_SUBOP, OP_GIVE, g_exchangeType };
    p.insert(p.end(), (BYTE*)&g_openBaseItem, (BYTE*)&g_openBaseItem + 4);
    if (g_exchangeType == 1)
        p.insert(p.end(), (BYTE*)&g_selectedReward, (BYTE*)&g_selectedReward + 4);
    QueueSendR(p);
    g_statusR = "Exchanging..."; g_statusAtR = GetTickCount();
}

// ---------------------------------------------------------------- window plumbing
static HWND FindGameWindowR()
{
    struct Ctx { DWORD pid; HWND best; int area; } ctx = { GetCurrentProcessId(), nullptr, 0 };
    EnumWindows([](HWND h, LPARAM lp) -> BOOL {
        Ctx* c = (Ctx*)lp;
        DWORD pid; GetWindowThreadProcessId(h, &pid);
        if (pid != c->pid || !IsWindowVisible(h) || h == g_hWndR) return TRUE;
        RECT r; GetWindowRect(h, &r);
        int a = (r.right - r.left) * (r.bottom - r.top);
        if (a > c->area) { c->area = a; c->best = h; }
        return TRUE;
    }, (LPARAM)&ctx);
    return ctx.best;
}

static bool g_openR = false, g_placedR = false;
static HCURSOR g_gameCursorR = nullptr;

static void SetModeR(bool cross);

static void HideR()
{
    EnterCriticalSection(&g_dataLockR); g_runAll = false; LeaveCriticalSection(&g_dataLockR);
    g_openR = false;
    SetModeR(false);   // the next right-click opens the normal exchange panel
    HideTip();
    ShowWindow(g_hWndR, SW_HIDE);
    if (HWND game = FindGameWindowR()) SetForegroundWindow(game);
}

static void SyncVisibilityR()
{
    HWND game = FindGameWindowR();
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    bool want = g_openR && game && !IsIconic(game) && pid == GetCurrentProcessId();
    if (want != (IsWindowVisible(g_hWndR) != FALSE))
    {
        if (want) { SetWindowPos(g_hWndR, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); Paint(); }
        else { HideTip(); ShowWindow(g_hWndR, SW_HIDE); }
    }
    POINT p; CURSORINFO ci = { sizeof(ci) };
    if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) &&
        (ci.flags & CURSOR_SHOWING) && ci.hCursor)
        g_gameCursorR = ci.hCursor;
}

static void ShowR()
{
    g_openR = true;
    HideTip();
    if (!g_placedR)   // first open: centre on the game window, afterwards keep the dragged position
    {
        g_placedR = true;
        RECT gr = { 0, 0, RCE_W, RCE_H };
        if (HWND game = FindGameWindowR()) GetWindowRect(game, &gr);
        int x = gr.left + max(0, (int)(gr.right - gr.left - RCE_W) / 2);
        int y = gr.top + max(0, (int)(gr.bottom - gr.top - RCE_H) / 2);
        SetWindowPos(g_hWndR, HWND_TOPMOST, x, y, RCE_W, RCE_H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    else SetWindowPos(g_hWndR, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    Paint();
}

void Rce_OpenSlot(int i);

static void Activate(int ctl)
{
    if (ctl == R_CLOSE) { HideR(); return; }
    if (ctl >= R_INV0 && ctl < R_INVN) { Rce_OpenSlot(ctl - R_INV0); return; }
    EnterCriticalSection(&g_dataLockR);
    if (ctl >= R_BATCH1 && ctl <= R_BATCHALL)
    {
        g_batchAll = ctl == R_BATCHALL;
        g_batchSize = ctl == R_BATCH10 ? 10 : ctl == R_BATCH100 ? 100 : 1;
        g_statusR = g_batchAll ? "Tum taslar secildi." : "Secilen adet: " + std::to_string(g_batchSize);
    }
    else if (ctl == R_EXCHANGE) { g_runAll = g_batchAll; SendGive(); }
    else if (ctl >= R_SLOT0 && ctl < R_SLOTN && g_view == VIEW_REWARD && g_exchangeType == 1)
    {
        int i = ctl - R_SLOT0;
        if (g_rewards[i].itemID) { g_selectedReward = g_rewards[i].itemID; g_statusR.clear(); }
    }
    LeaveCriticalSection(&g_dataLockR);
    Paint();
}

static LRESULT CALLBACK WndProcR(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
    switch (msg)
    {
    case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
    case WM_MOUSEMOVE:
        if (g_draggingR)
        {
            POINT p; GetCursorPos(&p);
            RECT wr; GetWindowRect(h, &wr);
            SetWindowPos(h, nullptr, wr.left + p.x - g_dragFromR.x, wr.top + p.y - g_dragFromR.y, 0, 0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
            g_dragFromR = p;
            return 0;
        }
        if (!g_trackingR) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_trackingR = TrackMouseEvent(&t) != FALSE; }
        if (int hit = HitTestR(x, y); hit != g_hoverR) { g_hoverR = hit; Paint(); UpdateTip(); }
        return 0;
    case WM_MOUSELEAVE:
        g_trackingR = false;
        if (g_hoverR != R_NONE) { g_hoverR = R_NONE; Paint(); }
        HideTip();
        return 0;
    case WM_LBUTTONDOWN:
    {
        int hit = HitTestR(x, y);
        if (hit == R_NONE)   // anywhere outside a control drags the window
        {
            g_draggingR = true; GetCursorPos(&g_dragFromR); SetCapture(h); HideTip(); return 0;
        }
        g_pressedR = hit;
        SetCapture(h); Paint();
        return 0;
    }
    case WM_LBUTTONUP:
    {
        ReleaseCapture();
        if (g_draggingR) { g_draggingR = false; return 0; }
        int hit = HitTestR(x, y), pressed = g_pressedR;
        g_pressedR = R_NONE;
        if (pressed != R_NONE && pressed == hit) Activate(hit);
        else Paint();
        return 0;
    }
    case WM_RBUTTONUP:
        if (g_crossR)   // Cross Exchange: right-click on an inventory cell opens that coupon, elsewhere nothing
        {
            int hit = HitTestR(x, y);
            if (hit >= R_INV0 && hit < R_INVN) Activate(hit);
            return 0;
        }
        HideR();
        return 0;
    case WM_SETCURSOR:
        while (ShowCursor(TRUE) < 0) {}
        SetCursor(g_gameCursorR ? g_gameCursorR : LoadCursor(nullptr, IDC_ARROW));
        return TRUE;
    case WM_TIMER:
        EnterCriticalSection(&g_dataLockR);
        if (g_generatorBusy && GetTickCount() - g_generatorSentAt > 10000) {
            g_generatorBusy = false; g_runAll = false; g_statusR = "Islem yaniti alinamadi; toplu kirma durdu.";
        }
        LeaveCriticalSection(&g_dataLockR);
        SyncVisibilityR(); return 0;
    case WM_RCE_REFRESH: if (IsWindowVisible(h)) Paint(); return 0;
    case WM_RCE_SHOW: if (wp) SetModeR(true); ShowR(); return 0;
    case WM_CLOSE:
        if (wp == 1 && g_crossR) { Paint(); return 0; }   // exchange done: the Cross window stays open for the next coupon
        HideR();
        return 0;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

// ---------------------------------------------------------------- inventory right-click hook
// Native handler (2625 decompile sub_B6B860): thiscall(CUIInventory*, POINT pt, int, int) -> char.
// It walks the 28 inventory slots (this+slotOff; slot->icon element, rect floats at +0xC8) and only
// acts when the ticket-exchange window is open. We run it first, then hit-test the same slots.
static const BYTE  kInvSig[]  = { 0x55,0x8B,0xEC,0x83,0xE4,0xF8,0x83,0xEC,0x1C,0xA1,0,0,0,0,0x33,0xC4,0x89,0x44,0x24,0x18,
                                  0xA1,0,0,0,0,0x53,0x56,0x57,0x8B,0x80,0,0,0,0,0x89,0x4C,0x24,0x10,0x85,0xC0,0x74,0,
                                  0x80,0xB8,0,0,0,0,0x00,0x74,0,0x8B,0x5D,0x0C,0x8D,0xB9 };
static const char  kInvMask[] = "xxxxxxxxxx????xxxxxx" "x????xxxxx????xxxxxxx?xx????xx?xxxxx";
static_assert(sizeof(kInvMask) - 1 == sizeof(kInvSig), "signature/mask length");
static const int   kInvStolen = 6;   // 55 8B EC 83 E4 F8 (push ebp; mov ebp,esp; and esp,-8)
static DWORD g_invSlotOff = 0;
typedef char(__thiscall* tInvRClick)(void* self, POINT pt, int a3, int a4);
static tInvRClick g_oInvRClick = nullptr;

// game thread: send the open request right away (same thread the game sends from)
static bool OnInventoryRightClick(int slot)
{
    if (slot < 0 || slot >= HAVE_MAX_CLIENT) return false;
    EnterCriticalSection(&g_dataLockR);
    bool en = g_slotEnabled[slot] != 0 && g_slotItemID[slot] != 0;
    BYTE sub = g_slotSubCode[slot] ? g_slotSubCode[slot] : 1;
    UINT32 item = g_slotItemID[slot];
    LeaveCriticalSection(&g_dataLockR);
    if (!en) return false;
    if (item == 810521000) return false;   // Solo Cape Scroll: right click = the scroll's own skill (cape shop), never RCE
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, RCE_SUBOP, sub, (BYTE)(SLOT_MAX_CLIENT + slot) };
    p.insert(p.end(), (BYTE*)&item, (BYTE*)&item + 4);
    QueueSendR(p);
    Rce_FlushSendQueue();
    char buf[96]; sprintf_s(buf, "RCE right-click slot=%d sub=%u item=%u", slot, sub, item); Log(buf);
    return true;
}

// Solo Cape Scroll: the client would register its skill on the skill bar (like the Slave Priest scroll);
// instead ask the server for the cape selection (WIZ_HSACS_HOOK + HGGENIE 0xED, op 3) and eat the click.
void Cape_ExpectShop();
void Tag_OpenPanel();
void JobChange_RequestOpen(BYTE type);
static bool SoloCapeRightClick(void* self, POINT pt)
{
    for (int i = 0; i < HAVE_MAX_CLIENT; i++)
    {
        DWORD slotPtr = *(DWORD*)((BYTE*)self + g_invSlotOff + 4 * i);
        if (!slotPtr) continue;
        DWORD el = *(DWORD*)slotPtr;
        if (!el) continue;
        const float* f = (const float*)(el + 0xC8);
        RECT rc = { (LONG)f[0], (LONG)f[1], (LONG)f[2], (LONG)f[3] };
        if (!PtInRect(&rc, pt)) continue;
        EnterCriticalSection(&g_dataLockR);
        UINT32 item = g_slotItemID[i];
        LeaveCriticalSection(&g_dataLockR);
        if (item == 800099000)             // Scroll of Tag ID: open the tag panel (tag_panel.cpp), scroll checked on Confirm
        {
            Tag_OpenPanel();
            Log("RCE: tag scroll right-click -> tag panel");
            return true;
        }
        if (item == 700112000 || item == 700113000)   // Job Change scroll: the server opens the class panel (jobchange_panel.cpp)
        {
            JobChange_RequestOpen(item == 700112000 ? 0 : 1);
            Log("RCE: job change scroll right-click -> job change panel");
            return true;
        }
        if (item != 810521000) return false;
        Cape_ExpectShop();                // cape_gate.cpp: the NPC menu that follows opens the shop directly
        std::vector<BYTE> p = { WIZ_HSACS_HOOK, 0xED, 3 };
        QueueSendR(p);
        Rce_FlushSendQueue();
        Log("RCE: solo cape scroll right-click -> cape selection");
        return true;
    }
    return false;
}

static bool SoloCapeRightClickSafe(void* self, POINT pt)
{
    __try { return SoloCapeRightClick(self, pt); }
    __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
}

static char __fastcall hkInvRClick(void* self, void* /*edx*/, POINT pt, int a3, int a4)
{
    if (SoloCapeRightClickSafe(self, pt)) return 1;
    __try
    {
        for (int i = 0; i < HAVE_MAX_CLIENT; i++)
        {
            DWORD slotPtr = *(DWORD*)((BYTE*)self + g_invSlotOff + 4 * i);
            if (!slotPtr) continue;
            DWORD el = *(DWORD*)slotPtr;
            if (!el) continue;
            const float* f = (const float*)(el + 0xC8);
            RECT rc = { (LONG)f[0], (LONG)f[1], (LONG)f[2], (LONG)f[3] };
            if (PtInRect(&rc, pt) && OnInventoryRightClick(i)) return 1;
        }
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { Log("RCE: exception in inventory right-click hook"); }
    return g_oInvRClick(self, pt, a3, a4);
}

static BYTE* FindInvHandler()
{
    BYTE* base = (BYTE*)GetModuleHandleA(nullptr);
    auto nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
    DWORD size = nt->OptionalHeader.SizeOfImage;
    const size_t n = sizeof(kInvSig);
    BYTE* found = nullptr; int hits = 0;
    for (DWORD off = 0; off < size;)
    {
        MEMORY_BASIC_INFORMATION mbi{};
        if (!VirtualQuery(base + off, &mbi, sizeof(mbi))) break;
        DWORD regionEnd = (DWORD)((BYTE*)mbi.BaseAddress + mbi.RegionSize - base);
        bool exec = mbi.State == MEM_COMMIT && (mbi.Protect & (PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY)) &&
                    !(mbi.Protect & (PAGE_GUARD | PAGE_NOACCESS));
        if (exec)
        {
            BYTE* p = (BYTE*)mbi.BaseAddress;
            BYTE* end = p + mbi.RegionSize - n;
            for (; p < end; p++)
            {
                if (*p != 0x55) continue;
                size_t k = 1;
                for (; k < n; k++) if (kInvMask[k] == 'x' && p[k] != kInvSig[k]) break;
                if (k == n) { found = p; hits++; }
            }
        }
        off = min(regionEnd, size);
        if (regionEnd <= (DWORD)((BYTE*)mbi.BaseAddress - base)) break;
    }
    char buf[96]; sprintf_s(buf, "RCE: inventory right-click handler scan: %d hit(s) first=%p", hits, found); Log(buf);
    return hits == 1 ? found : nullptr;
}

static DWORD WINAPI InstallInvHook(LPVOID)
{
    static const BYTE kRecvPrologue[5] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF };
    // exe unpacked = recv prologue present OR already replaced by the recv hook's jmp (0xE9);
    // checking only the prologue made this wait the full 120 s once the recv hook was in.
    const BYTE* recv = (const BYTE*)KO_RECV_FNC;
    for (int i = 0; i < 1200 && memcmp(recv, kRecvPrologue, 5) != 0 && recv[0] != 0xE9; i++) Sleep(100);
    Sleep(300);
    BYTE* target = FindInvHandler();
    if (!target) { Log("RCE: inventory right-click hook NOT installed"); return 0; }
    g_invSlotOff = *(DWORD*)(target + sizeof(kInvSig));        // "lea edi, [ecx+slotOff]"
    if (g_invSlotOff < 0x40 || g_invSlotOff > 0x4000) { Log("RCE: odd slot offset, not hooking"); return 0; }

    BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 32, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    if (!tramp) return 0;
    memcpy(tramp, target, kInvStolen);
    tramp[kInvStolen] = 0xE9;
    *(DWORD*)(tramp + kInvStolen + 1) = (DWORD)(target + kInvStolen) - (DWORD)(tramp + kInvStolen + 5);
    g_oInvRClick = (tInvRClick)tramp;

    DWORD old;
    if (!VirtualProtect(target, kInvStolen, PAGE_EXECUTE_READWRITE, &old)) return 0;
    target[0] = 0xE9;
    *(DWORD*)(target + 1) = (DWORD)hkInvRClick - (DWORD)(target + 5);
    target[5] = 0x90;
    VirtualProtect(target, kInvStolen, old, &old);
    FlushInstructionCache(GetCurrentProcess(), target, kInvStolen);
    char buf[96]; sprintf_s(buf, "RCE: inventory right-click hook INSTALLED @%p slotOff=%lX", target, g_invSlotOff); Log(buf);
    return 0;
}

// ---------------------------------------------------------------- window thread
static DWORD WINAPI WindowThreadR(LPVOID)
{
    GetModuleFileNameA(nullptr, g_uiDirR, MAX_PATH);
    strcpy_s(g_skinDirR, g_uiDirR);
    strcpy_s(strrchr(g_uiDirR, '\\') + 1, 48, "HopeGuard\\pus_ui");   // reuse the PUS icon set
    strcpy_s(g_skinDirX, g_skinDirR);
    strcpy_s(strrchr(g_skinDirR, '\\') + 1, 48, "HopeGuard\\rce_ui");
    strcpy_s(strrchr(g_skinDirX, '\\') + 1, 48, "HopeGuard\\cross_ui");

    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProcR;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = L"NTT_RceStore";
    RegisterClassExW(&wc);
    g_hWndR = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        wc.lpszClassName, L"Item Exchange", WS_POPUP, 0, 0, RCE_W, RCE_H, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_hWndR) { Log("RCE: window creation failed"); return 0; }

    g_memDCR = CreateCompatibleDC(nullptr);
    MakeDibR();

    WNDCLASSEXW tc = { sizeof(tc) };
    tc.lpfnWndProc = DefWindowProcW;
    tc.hInstance = wc.hInstance;
    tc.lpszClassName = L"NTT_RceTip";
    RegisterClassExW(&tc);
    g_hWndTip = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        tc.lpszClassName, L"Item Info", WS_POPUP, 0, 0, TIP_W, 100, nullptr, nullptr, tc.hInstance, nullptr);
    g_memDCTip = CreateCompatibleDC(nullptr);
    BITMAPINFO tb = {};
    tb.bmiHeader = { sizeof(BITMAPINFOHEADER), TIP_W, -TIP_MAXH, 1, 32, BI_RGB };
    SelectObject(g_memDCTip, CreateDIBSection(g_memDCTip, &tb, DIB_RGB_COLORS, &g_bitsTip, nullptr, 0));
    Log("RCE: window ready");
    SetTimer(g_hWndR, 1, 150, nullptr);

    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
    return 0;
}

// ---------------------------------------------------------------- recv (game thread)
void Rce_OnRecv(const BYTE* buf, size_t len)
{
    // Lua rewards, letters and automatic loot can change inventory without a
    // client request. Request one fresh slot map only when an item ID changes.
    if (len >= 13 && buf[0] == 0x3D && buf[1] == 1 && buf[2] == 0 && buf[3] == 1 && buf[4] < HAVE_MAX_CLIENT) {
        UINT32 item = 0, count = 0;
        memcpy(&item, buf + 5, 4); memcpy(&count, buf + 9, 4);
        int slot = buf[4]; bool refresh = false;
        EnterCriticalSection(&g_dataLockR);
        if (!count) { g_slotItemID[slot] = 0; g_slotEnabled[slot] = 0; g_invIcon[slot] = 0; }
        else if (g_slotItemID[slot] != item) { g_slotItemID[slot] = item; g_slotEnabled[slot] = 0; refresh = true; }
        g_invCount[slot] = (UINT16)min(count, 65535U);
        LeaveCriticalSection(&g_dataLockR);
        if (refresh && InterlockedCompareExchange(&g_inventoryRefreshPending, 1, 0) == 0)
            QueueSendR({ 0xE9, RCE_SUBOP, 0 });
        if (g_hWndR) PostMessageW(g_hWndR, WM_RCE_REFRESH, 0, 0);
        return;
    }
    if (len >= 6 && buf[0] == 0x5B && buf[1] == 4) {
        Rce_OpenCross(); Log("RCE: Chaotic Generator -> batch exchange panel"); return;
    }
    if (len < 3 || buf[1] != RCE_SUBOP) return;   // buf[0]=0xE9, buf[1]=0xE6, buf[2]=op
    ReaderR r{ buf, len, 2 };
    BYTE op = r.get<BYTE>();
    char msg[160];
    bool show = false;

    if (op == 0)   // coupon slot list (28 x enabled,subCode,itemID)
    {
        InterlockedExchange(&g_inventoryRefreshPending, 0);
        BYTE en[HAVE_MAX_CLIENT], sc[HAVE_MAX_CLIENT]; UINT32 id[HAVE_MAX_CLIENT];
        for (int i = 0; i < HAVE_MAX_CLIENT; i++) { en[i] = r.get<BYTE>(); sc[i] = r.get<BYTE>(); id[i] = r.get<UINT32>(); }
        if (!r.ok) return;
        int cnt = 0;
        EnterCriticalSection(&g_dataLockR);
        for (int i = 0; i < HAVE_MAX_CLIENT; i++) { g_slotEnabled[i] = en[i]; g_slotSubCode[i] = sc[i]; g_slotItemID[i] = id[i]; if (en[i]) cnt++; }
        if (r.remain() >= 1 + HAVE_MAX_CLIENT * 6 + 12 && r.get<BYTE>() == 2)   // [2] 28*(icon,count) weight maxWeight noah
        {
            for (int i = 0; i < HAVE_MAX_CLIENT; i++) { g_invIcon[i] = r.get<UINT32>(); g_invCount[i] = r.get<UINT16>(); }
            g_weight = r.get<UINT32>(); g_maxWeight = r.get<UINT32>(); g_gold = r.get<UINT32>();
        }
        LeaveCriticalSection(&g_dataLockR);
        if (g_hWndR && IsWindowVisible(g_hWndR)) PostMessageW(g_hWndR, WM_RCE_REFRESH, 0, 0);
        sprintf_s(msg, "RCE recv: coupon slot list, %d enabled", cnt); Log(msg);
        return;
    }
    else if (op == OP_TYPELIST)
    {
        BYTE type = r.get<BYTE>(); UINT16 count = r.get<UINT16>();
        if (r.ok && type < 8)
        {
            EnterCriticalSection(&g_dataLockR);
            g_typeList[type].clear();
            for (UINT16 i = 0; i < count && r.ok; i++) { UINT32 v = r.get<UINT32>(); if (v) g_typeList[type].push_back(v); }
            LeaveCriticalSection(&g_dataLockR);
            sprintf_s(msg, "RCE recv: type list type=%u count=%u", type, count); Log(msg);
        }
        return;
    }
    else if (op == OP_OPEN)   // open response
    {
        BYTE mode = r.get<BYTE>();
        if (mode == 1)
        {
            UINT32 itemID = r.get<UINT32>();
            if (r.ok) { EnterCriticalSection(&g_dataLockR); SendRewardInfo(itemID); LeaveCriticalSection(&g_dataLockR); }
            sprintf_s(msg, "RCE recv: open mode1 item=%u -> request rewards", itemID); Log(msg);
            return;
        }
        else if (mode == 2)
        {
            // [u8 slot][u32 item] (+ newer server: [u32 icon][str name][u16 count])
            BYTE slot = r.get<BYTE>(); UINT32 itemID = r.get<UINT32>();
            UINT32 iconID = 0, count = 1; std::string name;
            if (r.ok && r.remain() >= 4)
            {
                ReaderR m = r;
                UINT32 ic = m.get<UINT32>(); std::string n = m.str(); UINT16 c = m.get<UINT16>();
                if (m.ok) { iconID = ic; name = n; count = c; }
            }
            if (r.ok)
            {
                EnterCriticalSection(&g_dataLockR);
                g_view = VIEW_GENERATOR; g_sourceSlot = slot; g_openBaseItem = itemID; g_baseIcon = iconID;
                g_generatorBusy = false; g_runAll = false;
                g_baseName = name; g_genCount = count; g_selectedReward = 0; g_statusR.clear();
                g_lastReward = g_lastRewardIcon = 0; g_lastRewardName.clear();
                for (auto& rw : g_rewards) rw = Reward{};
                LeaveCriticalSection(&g_dataLockR);
                show = true;
            }
            sprintf_s(msg, "RCE recv: open mode2 (generator) slot=%u item=%u count=%u ok=%d", slot, itemID, count, (int)r.ok); Log(msg);
        }
    }
    else if (op == OP_OPENED)   // reward list / success
    {
        BYTE sub = r.get<BYTE>();
        if (sub == 5)
        {
            EnterCriticalSection(&g_dataLockR);
            g_statusR = "Exchange successful!"; g_statusAtR = GetTickCount();
            LeaveCriticalSection(&g_dataLockR);
            Log("RCE recv: exchange success");
            if (g_hWndR) PostMessageW(g_hWndR, WM_CLOSE, 1, 0);   // done -> close the panel (like HSACSX); Cross stays open
            return;
        }
        else if (sub == 8)   // generator result: [u32 reward][u32 icon][str name][u16 left][str message]
        {
            UINT32 reward = r.get<UINT32>(), icon = r.get<UINT32>();
            std::string name = r.str(); UINT16 left = r.get<UINT16>(); std::string text = r.str();
            if (!r.ok) { Log("RCE recv: generator result (short packet)"); return; }
            UINT32 source = r.remain() >= 4 ? r.get<UINT32>() : 0;
            if (source && source != g_openBaseItem) { Log("RCE: stale generator result ignored"); return; }
            EnterCriticalSection(&g_dataLockR);
            g_genCount = left;
            g_generatorBusy = false;
            if (reward)
            {
                g_lastReward = reward; g_lastRewardIcon = icon; g_lastRewardName = name;
                g_statusR = "You received: " + name; g_statusAtR = GetTickCount();
            }
            else
                g_statusR = text.empty() ? "The item could not be exchanged." : text; g_statusAtR = GetTickCount();
            LeaveCriticalSection(&g_dataLockR);
            sprintf_s(msg, "RCE recv: generator result reward=%u left=%u", reward, left); Log(msg);
            EnterCriticalSection(&g_dataLockR);
            if (g_runAll && reward && left) SendGive(); else g_runAll = false;
            LeaveCriticalSection(&g_dataLockR);
            if (g_hWndR) PostMessageW(g_hWndR, WM_RCE_REFRESH, 0, 0);
            return;
        }
        else if (IsRewardType(sub))
        {
            BYTE exType = r.get<BYTE>(); UINT32 baseItem = r.get<UINT32>();
            if (!IsRewardType(exType)) exType = sub;
            Reward rw[RCE_SLOT_COUNT];
            for (int i = 0; i < RCE_SLOT_COUNT; i++) { rw[i].itemID = r.get<UINT32>(); rw[i].rentalTime = r.get<UINT32>(); rw[i].count = 1; }
            if (!r.ok) return;
            UINT32 baseIcon = 0; std::string baseName;
            if (r.remain() >= (size_t)4 + (size_t)RCE_SLOT_COUNT * 4)
            {
                baseIcon = r.get<UINT32>();
                for (int i = 0; i < RCE_SLOT_COUNT; i++) rw[i].iconID = r.get<UINT32>();
                BYTE ver = r.remain() ? r.get<BYTE>() : 0;
                if (ver == 1 || ver == 2)   // 2 = + per-reward stat lines
                {
                    ReaderR m = r;
                    std::string bn = m.str();
                    Reward tmp[RCE_SLOT_COUNT];
                    for (int i = 0; i < RCE_SLOT_COUNT && m.ok; i++)
                    {
                        tmp[i].name = m.str(); tmp[i].count = m.get<UINT32>();
                        if (ver == 2)
                            for (BYTE n = m.get<BYTE>(), k = 0; k < n && m.ok; k++) { BYTE c = m.get<BYTE>(); tmp[i].lines.push_back({ c, m.str() }); }
                    }
                    if (m.ok)
                    {
                        baseName = bn;
                        for (int i = 0; i < RCE_SLOT_COUNT; i++) { rw[i].name = tmp[i].name; rw[i].count = tmp[i].count ? tmp[i].count : 1; rw[i].lines = tmp[i].lines; }
                    }
                }
            }
            int filled = 0; UINT32 firstItem = 0;
            EnterCriticalSection(&g_dataLockR);
            g_view = VIEW_REWARD; g_exchangeType = exType; g_openBaseItem = baseItem; g_baseIcon = baseIcon; g_baseName = baseName;
            g_selectedReward = 0; g_statusR.clear();
            for (int i = 0; i < RCE_SLOT_COUNT; i++)
            {
                g_rewards[i] = rw[i];
                if (rw[i].itemID) { filled++; if (!firstItem) firstItem = rw[i].itemID; }
            }
            if (exType == 1) g_selectedReward = firstItem;   // type-1 = pick one; preselect the first
            LeaveCriticalSection(&g_dataLockR);
            show = true;
            sprintf_s(msg, "RCE recv: reward list type=%u base=%u filled=%d name=%s", exType, baseItem, filled, baseName.c_str()); Log(msg);
        }
    }
    if (g_hWndR) PostMessageW(g_hWndR, show ? WM_RCE_SHOW : WM_RCE_REFRESH, 0, 0);
}

// ---------------------------------------------------------------- exports
void Rce_Init()
{
    InitializeCriticalSection(&g_sendLockR);
    InitializeCriticalSection(&g_dataLockR);
    CloseHandle(CreateThread(nullptr, 0, WindowThreadR, nullptr, 0, nullptr));
    CloseHandle(CreateThread(nullptr, 0, InstallInvHook, nullptr, 0, nullptr));
}

// kept for pus_store.cpp's declaration; the panel now opens from the inventory right-click
bool Rce_OnTaskbarClick(const char*) { return false; }

// ---- accessors for cross_exch.cpp (inventory "Cross Exchange" button / slot markers) ----
static const DWORD KO_PTR_GAMEMAIN = 0x011158FC;   // CGameProcMain*; +472 = CUIInventory*, UI visible byte at +234

void* Rce_Inventory()
{
    __try
    {
        DWORD main = *(DWORD*)KO_PTR_GAMEMAIN;
        return main ? *(void**)(main + 472) : nullptr;
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { return nullptr; }
}

bool Rce_InventoryVisible()
{
    void* inv = Rce_Inventory();
    if (!inv) return false;
    __try { return *((BYTE*)inv + 234) != 0; }
    __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
}

// slot rect in game-window client coordinates (same floats the right-click hook hit-tests)
bool Rce_InvSlotRect(int i, RECT* rc)
{
    void* inv = Rce_Inventory();
    if (!inv || !g_invSlotOff || i < 0 || i >= HAVE_MAX_CLIENT) return false;
    __try
    {
        DWORD slotPtr = *(DWORD*)((BYTE*)inv + g_invSlotOff + 4 * i);
        if (!slotPtr) return false;
        DWORD el = *(DWORD*)slotPtr;
        if (!el) return false;
        const float* f = (const float*)(el + 0xC8);
        *rc = { (LONG)f[0], (LONG)f[1], (LONG)f[2], (LONG)f[3] };
        return rc->right > rc->left && rc->bottom > rc->top;
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
}

// exchangeable coupon in inventory slot i (server op 0 list)
bool Rce_SlotExchangeable(int i, UINT32* item)
{
    if (i < 0 || i >= HAVE_MAX_CLIENT) return false;
    EnterCriticalSection(&g_dataLockR);
    bool en = g_slotEnabled[i] != 0 && g_slotItemID[i] != 0 && g_slotItemID[i] != 810521000;
    if (item) *item = g_slotItemID[i];
    LeaveCriticalSection(&g_dataLockR);
    return en;
}

// Cross Exchange button: open the panel empty (the inventory below shows the coupons; click one to load it)
void Rce_OpenCross()
{
    EnterCriticalSection(&g_dataLockR);
    g_openBaseItem = 0; g_baseIcon = 0; g_baseName.clear(); g_exchangeType = 0; g_selectedReward = 0;
    g_generatorBusy = false; g_runAll = false;
    g_view = VIEW_REWARD;
    for (int i = 0; i < RCE_SLOT_COUNT; i++) g_rewards[i] = Reward();
    g_statusR = "Select a coupon from your inventory."; g_statusAtR = GetTickCount();
    LeaveCriticalSection(&g_dataLockR);
    if (g_hWndR) PostMessageW(g_hWndR, WM_RCE_SHOW, 1, 0);
}

// open the exchange panel for slot i from any thread (the request goes out on the game thread)
void Rce_OpenSlot(int i)
{
    if (i < 0 || i >= HAVE_MAX_CLIENT) return;
    EnterCriticalSection(&g_dataLockR);
    bool en = g_slotEnabled[i] != 0 && g_slotItemID[i] != 0;
    BYTE sub = g_slotSubCode[i] ? g_slotSubCode[i] : 1;
    UINT32 item = g_slotItemID[i];
    LeaveCriticalSection(&g_dataLockR);
    if (!en) return;
    std::vector<BYTE> p = { WIZ_HSACS_HOOK, RCE_SUBOP, sub, (BYTE)(SLOT_MAX_CLIENT + i) };
    p.insert(p.end(), (BYTE*)&item, (BYTE*)&item + 4);
    QueueSendR(p);
}

// called from pus_store.cpp's GetAsyncKeyState hook so clicks on our panel don't move the char
bool Rce_CursorOverPanel()
{
    if (!g_hWndR || !IsWindowVisible(g_hWndR)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWndR;
}
