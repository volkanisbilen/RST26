// Item tooltip for our panels (Power Up Store, drop lists ...), drawn by the proxy with the pieces of the game's own
// inventory tooltip (re_newtooltip.uif: background ui_message.dxt, separator co_pat_ui_02.dxt ->
// HopeGuard\itemtip_ui\*.pus, build_itemtip_assets.py). Calling the game's tooltip function (0x8DBF70) from here
// returned without an error but nothing was drawn, so the window is ours: a click-through layered window.
// Content:
//   name + description: the client's own item table in memory (basic row: map [0x1115804]+0x10, key num/1000*1000;
//     row +0x08 name, +0x20 description (std::string), +0x44 icon). Description lines are split by '|', a line may
//     carry <font color=@#RRGGBB@>..</font>.
//   stats: the server, E9 AE 07 [u32 item] -> [07][u32 item][str name][u32 icon][u8 n] n*([u8 colour][str line])
//     (GameServer XGuard.cpp DropView2625 / RceItemLines); asked once per item and kept.
#include <windows.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <map>
#include <deque>
#include "itemtip_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();

namespace
{
    const DWORD TBL_BASIC = 0x01115804, TBL_EXT = 0x01115750;
    const DWORD KO_SND_FNC = 0x00704070, KO_PTR_PKT = 0x01115914;   // as drop_panel.cpp
    const int TIP_W = IT_W, TIP_MAXH = 640, PAD = 12, ICON = 45, LINE_H = 17;
    const UINT WM_TIP_UPDATE = WM_APP + 71;

    struct Sprite { HDC dc = nullptr; int w = 0, h = 0; };
    struct Line { COLORREF color; std::wstring text; bool center; };
    struct Info { bool asked = false, answered = false; std::string name; UINT32 icon = 0; std::vector<std::pair<BYTE, std::string>> stats; };
    struct ClientRow { char name[256]; char remark[1024]; UINT32 icon; };

    CRITICAL_SECTION g_lock;
    volatile LONG g_init = 0;
    UINT32 g_wantItem = 0; RECT g_wantAnchor = {};     // g_lock
    std::map<UINT32, Info> g_info;                      // g_lock
    std::deque<std::vector<BYTE>> g_sendQueue;          // g_lock
    HWND g_hWnd = nullptr; HDC g_dc = nullptr; void* g_bits = nullptr;
    Sprite g_back, g_bar;
    std::map<UINT32, Sprite> g_icons;                   // tooltip thread only
    HFONT g_font = nullptr, g_fontBold = nullptr;

    std::string GameDir() { char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH); *(strrchr(p, '\\') + 1) = 0; return p; }

    bool LoadPus(const std::string& p, Sprite& s)
    {
        FILE* f = nullptr;
        if (fopen_s(&f, p.c_str(), "rb") != 0 || !f) return false;
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

    void Blit(const Sprite& s, int x, int y, int w, int h)
    {
        if (!s.dc) return;
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(g_dc, x, y, w, h, s.dc, 0, 0, s.w, s.h, bf);
    }

    std::wstring Wide(const std::string& s)
    {
        if (s.empty()) return std::wstring();
        int n = MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), nullptr, 0);
        std::wstring w(n, L'\0'); MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), &w[0], n);
        return w;
    }

    // ---------------------------------------------------------------- the client's item table
    bool Readable(const void* p, size_t n)
    {
        MEMORY_BASIC_INFORMATION mbi;
        if (!p || !VirtualQuery(p, &mbi, sizeof(mbi))) return false;
        if (mbi.State != MEM_COMMIT || (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD))) return false;
        return (const BYTE*)p + n <= (const BYTE*)mbi.BaseAddress + mbi.RegionSize;
    }

    // std::map lookup: nodes left 0, parent 4, right 8, isnil +0xD, key +0x10, value +0x14
    BYTE* MapFind(BYTE* table, DWORD key)
    {
        if (!Readable(table, 0x14)) return nullptr;
        BYTE* head = *(BYTE**)(table + 0x10);
        if (!Readable(head, 0x14)) return nullptr;
        BYTE* node = *(BYTE**)(head + 4), *best = head;
        for (int guard = 0; guard < 64 && Readable(node, 0x14) && !node[0xD]; guard++)
        {
            if (*(DWORD*)(node + 0x10) < key) node = *(BYTE**)(node + 8);
            else { best = node; node = *(BYTE**)node; }
        }
        return best != head && *(DWORD*)(best + 0x10) == key ? best + 0x14 : nullptr;
    }

    // std::string at s (buffer / pointer +0, size +0x10, capacity +0x14) -> C string
    void CopyStdString(const BYTE* s, char* out, size_t outSize)
    {
        out[0] = 0;
        if (!Readable(s, 0x18)) return;
        size_t size = *(size_t*)(s + 0x10), cap = *(size_t*)(s + 0x14);
        if (size == 0 || size >= outSize || cap < size) return;
        const char* data = cap > 15 ? *(const char**)s : (const char*)s;
        if (!Readable(data, size)) return;
        memcpy(out, data, size); out[size] = 0;
    }

    bool ClientItem(UINT32 num, ClientRow* row)
    {
        row->name[0] = row->remark[0] = 0; row->icon = 0;
        __try
        {
            if (!Readable((void*)TBL_BASIC, 4)) return false;
            BYTE* basic = MapFind(*(BYTE**)TBL_BASIC, num / 1000 * 1000);
            if (!basic || !Readable(basic, 0x60)) return false;
            row->icon = *(DWORD*)(basic + 0x44);
            BYTE extIdx = basic[4];
            UINT32 hi = num / 1000000000;
            if (extIdx < 0x2D && hi < 4 && Readable((void*)(TBL_EXT + extIdx * 4), 4))
            {
                BYTE* ext = MapFind(*(BYTE**)(TBL_EXT + extIdx * 4), hi * 1000 + num % 1000);
                if (ext && Readable(ext, 0x48) && *(DWORD*)(ext + 0x40)) row->icon = *(DWORD*)(ext + 0x40);
            }
            CopyStdString(basic + 0x08, row->name, sizeof(row->name));
            CopyStdString(basic + 0x20, row->remark, sizeof(row->remark));
            return true;
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
    }

    // "text | text | <font color=@#FF5E00@><b> text</b></font>" -> lines
    void RemarkLines(const std::string& remark, std::vector<Line>& out)
    {
        size_t at = 0;
        while (at <= remark.size())
        {
            size_t bar = remark.find('|', at);
            std::string part = remark.substr(at, bar == std::string::npos ? std::string::npos : bar - at);
            at = bar == std::string::npos ? remark.size() + 1 : bar + 1;
            COLORREF color = RGB(235, 235, 235);
            size_t c = part.find("color=@#");
            if (c != std::string::npos && c + 14 <= part.size())
            {
                unsigned rgb = strtoul(part.substr(c + 8, 6).c_str(), nullptr, 16);
                color = RGB((rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255);
            }
            std::string text;
            for (size_t i = 0; i < part.size(); i++)
            {
                if (part[i] == '<') { size_t e = part.find('>', i); if (e != std::string::npos) { i = e; continue; } }
                text += part[i];
            }
            size_t a = text.find_first_not_of(" \t"), b = text.find_last_not_of(" \t");
            if (a == std::string::npos) continue;
            out.push_back({ color, Wide(text.substr(a, b - a + 1)), true });
        }
    }

    COLORREF StatColor(BYTE c)
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

    const Sprite& Icon(UINT32 icon)
    {
        auto it = g_icons.find(icon);
        if (it != g_icons.end()) return it->second;
        Sprite s;
        LoadPus(GameDir() + "HopeGuard\\pus_ui\\icons\\" + std::to_string(icon) + ".pus", s);
        return g_icons[icon] = s;
    }

    int TextHeight(const std::wstring& t, int width, HFONT font)
    {
        RECT r = { 0, 0, width, 0 };
        SelectObject(g_dc, font);
        DrawTextW(g_dc, t.c_str(), (int)t.size(), &r, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
        return max((int)r.bottom, LINE_H - 3);
    }

    void DrawLine(const std::wstring& t, RECT r, COLORREF color, HFONT font, UINT flags)
    {
        SelectObject(g_dc, font);
        SetBkMode(g_dc, TRANSPARENT);
        SetTextColor(g_dc, color);
        DrawTextW(g_dc, t.c_str(), (int)t.size(), &r, flags | DT_NOPREFIX);
    }

    void Paint()
    {
        EnterCriticalSection(&g_lock);
        UINT32 item = g_wantItem; RECT anchor = g_wantAnchor;
        Info info; auto it = g_info.find(item); if (it != g_info.end()) info = it->second;
        LeaveCriticalSection(&g_lock);
        if (!item) { ShowWindow(g_hWnd, SW_HIDE); return; }

        static ClientRow row;
        ClientItem(item, &row);
        // The basic row only has the base item's name ("Hunter's Bow"); the item itself may be an extension of it
        // ("Chitin Bow", "Dark Vane(+5)"). The server's answer carries the real name, so it wins once it is here.
        std::string name = !info.name.empty() ? info.name : row.name;
        bool baseDiffers = !info.name.empty() && row.name[0] && info.name != row.name;
        UINT32 icon = row.icon ? row.icon : info.icon;
        if (name.empty()) name = "Item " + std::to_string(item);

        std::vector<Line> lines;
        for (auto& s : info.stats) lines.push_back({ StatColor(s.first), Wide(s.second), false });
        size_t statCount = lines.size();
        RemarkLines(row.remark, lines);
        // like the game's tooltip: the base item's name at the bottom ("*Hunter's Bow*") unless the description already is it
        if (baseDiffers && _stricmp(row.remark, row.name) != 0)
            lines.push_back({ RGB(235, 235, 235), Wide(std::string("*") + row.name + "*"), true });

        const int textW = TIP_W - 2 * PAD;
        std::wstring wname = Wide(name);
        int nameH = TextHeight(wname, textW - ICON - 8, g_fontBold);
        int headH = max(ICON, nameH);
        int h = PAD + headH + 8 + 6;
        std::vector<int> heights;
        for (size_t i = 0; i < lines.size(); i++)
        {
            if (i == statCount && statCount) h += 10;   // separator between the stats and the description
            int lh = TextHeight(lines[i].text, textW, g_font) + 3;
            heights.push_back(lh); h += lh;
        }
        h += PAD;
        if (h > TIP_MAXH) h = TIP_MAXH;

        memset(g_bits, 0, (size_t)TIP_W * TIP_MAXH * 4);
        if (g_back.dc) Blit(g_back, 0, 0, TIP_W, h);
        else { RECT r = { 0, 0, TIP_W, h }; HBRUSH b = CreateSolidBrush(RGB(10, 10, 12)); FillRect(g_dc, &r, b); DeleteObject(b); }

        const Sprite& ic = Icon(icon);
        if (ic.dc) Blit(ic, PAD, PAD + (headH - ICON) / 2, ICON, ICON);
        RECT rn = { PAD + ICON + 8, PAD + (headH - nameH) / 2, TIP_W - PAD, PAD + (headH - nameH) / 2 + nameH };
        DrawLine(wname, rn, RGB(255, 255, 255), g_fontBold, DT_WORDBREAK | DT_LEFT);

        int y = PAD + headH + 8;
        if (g_bar.dc) Blit(g_bar, 4, y - 2, TIP_W - 8, 4);
        y += 6;
        for (size_t i = 0; i < lines.size(); i++)
        {
            if (i == statCount && statCount) { if (g_bar.dc) Blit(g_bar, 4, y + 3, TIP_W - 8, 4); y += 10; }
            if (y + heights[i] > h - 4) break;
            RECT r = { PAD, y, TIP_W - PAD, y + heights[i] };
            DrawLine(lines[i].text, r, lines[i].color, g_font, DT_WORDBREAK | (lines[i].center ? DT_CENTER : DT_LEFT));
            y += heights[i];
        }
        // GDI writes text with alpha 0 into a 32 bit DIB: everything inside the box is opaque
        DWORD* px = (DWORD*)g_bits;
        for (int r = 2; r < h - 2; r++)
            for (int c = 2; c < TIP_W - 2; c++)
                px[r * TIP_W + c] |= 0xFF000000;

        // right of the anchor (the panel's edge), on the left when there is no room; kept on the monitor
        HMONITOR mon = MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST);
        MONITORINFO mi = { sizeof(mi) }; GetMonitorInfo(mon, &mi);
        int x = anchor.right + 4, ty = anchor.top;
        if (x + TIP_W > mi.rcWork.right) x = anchor.left - TIP_W - 4;
        if (x < mi.rcWork.left) x = mi.rcWork.left;
        if (ty + h > mi.rcWork.bottom) ty = mi.rcWork.bottom - h;
        if (ty < mi.rcWork.top) ty = mi.rcWork.top;

        POINT dst = { x, ty }, src = { 0, 0 }; SIZE sz = { TIP_W, h };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_dc, &src, 0, &bf, ULW_ALPHA);
        SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);

        static int s_logged = 0;
        if (s_logged < 20)
        {
            s_logged++;
            char b[400]; sprintf_s(b, "ITEMTIP: item %u '%s' icon %u, %d stat + %d text line(s) at %d,%d", item, name.c_str(), icon,
                (int)statCount, (int)(lines.size() - statCount), x, ty);
            Log(b);
        }
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        if (msg == WM_TIP_UPDATE) { Paint(); return 0; }
        if (msg == WM_MOUSEACTIVATE) return MA_NOACTIVATE;
        if (msg == WM_NCHITTEST) return HTTRANSPARENT;
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI TipThread(LPVOID)
    {
        WNDCLASSW wc = {}; wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr); wc.lpszClassName = L"HG_ItemTip";
        RegisterClassW(&wc);
        HWND wnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            wc.lpszClassName, L"", WS_POPUP, 0, 0, TIP_W, TIP_MAXH, nullptr, nullptr, wc.hInstance, nullptr);
        if (!wnd) { Log("ITEMTIP: window not created"); return 0; }
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), TIP_W, -TIP_MAXH, 1, 32, BI_RGB };
        g_dc = CreateCompatibleDC(nullptr);
        HBITMAP bmp = CreateDIBSection(g_dc, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0);
        if (!bmp) { Log("ITEMTIP: no frame buffer"); return 0; }
        SelectObject(g_dc, bmp);
        g_font = CreateFontW(-12, 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET, 0, 0, CLEARTYPE_QUALITY, 0, L"Tahoma");
        g_fontBold = CreateFontW(-13, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0, CLEARTYPE_QUALITY, 0, L"Tahoma");
        std::string dir = GameDir() + "HopeGuard\\itemtip_ui\\";
        bool a = LoadPus(dir + "back.pus", g_back), b = LoadPus(dir + "bar.pus", g_bar);
        Log(a && b ? "ITEMTIP: own tooltip window ready (game skin loaded)" : "ITEMTIP: own tooltip window ready, SKIN MISSING (HopeGuard\\itemtip_ui)");
        g_hWnd = wnd;
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0) > 0) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }

    void Update() { if (g_hWnd) PostMessageW(g_hWnd, WM_TIP_UPDATE, 0, 0); }
}

void ItemTip_Init()
{
    InitializeCriticalSection(&g_lock);
    InterlockedExchange(&g_init, 1);
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") == 0) CloseHandle(CreateThread(nullptr, 0, TipThread, nullptr, 0, nullptr));
}

// any thread: show `item`'s tooltip beside `anchor` (screen rect: the panel's left / right + the cell's top)
void ItemTip_Show(UINT32 item, RECT anchor)
{
    if (!g_init) return;
    bool ask = false;
    EnterCriticalSection(&g_lock);
    g_wantItem = item; g_wantAnchor = anchor;
    Info& info = g_info[item];
    if (!info.asked)
    {
        info.asked = ask = true;
        std::vector<BYTE> p = { 0xE9, 0xAE, 7 };
        p.insert(p.end(), (BYTE*)&item, (BYTE*)&item + 4);
        g_sendQueue.push_back(p);
    }
    LeaveCriticalSection(&g_lock);
    if (ask) Proxy_RequestFlush();
    Update();
}

void ItemTip_Hide()
{
    if (!g_init) return;
    EnterCriticalSection(&g_lock); g_wantItem = 0; LeaveCriticalSection(&g_lock);
    Update();
}

// game thread (nothing left to do there: the window has its own thread)
void ItemTip_Tick() {}

// game thread (ProcessPacket): [E9][AE][07][u32 item][str name][u32 icon][u8 n] n*([u8 colour][str line]), strings u16 length
void ItemTip_OnRecv(const BYTE* p, size_t len)
{
    if (!g_init || len < 8 || p[1] != 0xAE || p[2] != 7) return;
    size_t o = 3;
    auto str = [&](std::string& s) -> bool
    {
        if (o + 2 > len) return false;
        size_t n = p[o] | (p[o + 1] << 8); o += 2;
        if (o + n > len) return false;
        s.assign((const char*)p + o, n); o += n; return true;
    };
    UINT32 item = *(const UINT32*)(p + o); o += 4;
    Info in; in.asked = in.answered = true;
    if (!str(in.name) || o + 5 > len) return;
    in.icon = *(const UINT32*)(p + o); o += 4;
    int n = p[o++];
    for (int i = 0; i < n; i++)
    {
        if (o + 1 > len) break;
        BYTE c = p[o++]; std::string s;
        if (!str(s)) break;
        in.stats.push_back(std::make_pair(c, s));
    }
    EnterCriticalSection(&g_lock);
    g_info[item] = in;
    bool current = g_wantItem == item;
    LeaveCriticalSection(&g_lock);
    if (current) Update();
}

// game thread
void ItemTip_FlushSendQueue()
{
    if (!g_init) return;
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_lock); pending.swap(g_sendQueue); LeaveCriticalSection(&g_lock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
    }
}
