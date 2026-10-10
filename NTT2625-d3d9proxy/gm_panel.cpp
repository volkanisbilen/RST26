// GM panel = the cyber001 CyberACS GM tools window (DAT-19.uif "Knight Game Master Manage") drawn with its own images
// (build_gm_assets.py -> HopeGuard\gm_ui\*.pus + gm_layout.h). Server: GameServer\GmPanel.cpp.
// Opened with the " key (WM_CHAR, any keyboard layout) and only for GMs (player authority +0x6DC == 0), never
// while a game edit (chat) has the focus. Tabs: Home, User Editor, Start Event, Reload Tables, Lottery, GM Commands, Bots.
// The buttons send the server's existing "+command" / "/command" text; the server checks the GM itself. Slots the cyber
// window has for events our server does not have carry our events (labels renamed in build_gm_assets.py).
// WIZ_HSACS_HOOK 0xE9 + GMPANEL 0xF5 (strings u16 length prefixed, CP1254):
//   C->S [1] list | [2][str name] details ("" = me) | [3][str command] | [4][str name][u8 auth] | [5] close temple event
//   S->C [1][u16 n] n*[str name][u8 lv][u8 nation][u16 class][u8 zone][u8 auth]
//        [2][u8 found][str name][str account][u8 lv][u8 nation][u8 race][u16 class][u8 zone][u16 x][u16 z][u32 np]
//           [u32 monthly][u32 cash][u32 gold][str clan][u8 auth][u8 muted][u8 noattack][5*u8 stats][i32 hp][i32 maxhp][str ip]
//           [i64 exp][i64 maxexp][i16 points][u16 attack][u16 defence][u32 manner][i32 mp][i32 maxmp][6*i16 weapon ac][6*u16 resist]
//           [5*u8 skill points: free, 1, 2, 3, master]
//   C->S [8][str name][u8 field][i64 value]  user editor APPLY (fields: see kEditFields / GameServer GmPanel.cpp)
//        [6][u8 ok][str text]
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <deque>
#include <map>
#include <functional>
#include <algorithm>
#include "gm_layout.h"
#pragma comment(lib, "msimg32.lib")

void Log(const char* msg);
void Proxy_RequestFlush();
HWND Proxy_GameWnd();

namespace
{
    const BYTE  GM_SUBOP = 0xF5;
    const DWORD KO_SND_FNC = 0x00704070, KO_PTR_PKT = 0x01115914;   // CAPISocket::Send, CAPISocket*
    const DWORD KO_PTR_ME = 0x01115834;                // our player; authority at +0x6DC (0 = GM)
    const DWORD KO_FOCUSED_EDIT = 0x0111523C;          // CN3UIEdit::s_pFocusedEdit (KillFocus 0x6E0B10 clears it)
    const int W = GMU_W, H = GMU_H;
    const UINT WM_GM_SHOW = WM_APP + 71, WM_GM_REFRESH = WM_APP + 72, WM_GM_HIDE = WM_APP + 73;
    const RECT STATUS_RC = { 70, 566, 860, 590 };      // bottom strip of the frame
    const RECT TITLE_RC = { 300, 10, 620, 75 };        // drag handle (title plate)
    enum { P_HOME, P_USER, P_EVENT, P_RELOAD, P_LOTTERY, P_COMMANDS, P_BOTS, P_EXTRA };
    const COLORREF C_OK = RGB(120, 230, 140), C_ERR = RGB(255, 110, 90), C_INFO = RGB(230, 230, 230), C_HINT = RGB(120, 120, 120);

    // ------------------------------------------------------------------ state (window thread unless noted)
    CRITICAL_SECTION g_lock;                           // g_sendQ, g_users, g_info, g_self, g_status, g_expect
    std::deque<std::vector<BYTE>> g_sendQ;
    std::deque<bool> g_expect;                         // pending [2] replies: true = my own info
    HWND  g_hWnd = nullptr;
    HDC   g_memDC = nullptr;
    void* g_bits = nullptr;
    std::vector<BYTE> g_alpha;
    bool  g_open = false, g_track = false, g_dragging = false;
    POINT g_dragFrom = {};
    HCURSOR g_gameCursor = nullptr;
    int   g_page = P_HOME, g_hot = -1, g_pressed = -1, g_role = 0;
    std::string g_focus;                               // id of the focused edit ("" = none)
    std::map<std::string, std::wstring> g_edit;        // edit contents

    struct UserRow { std::string name; BYTE level, nation, zone, auth; WORD cls; };
    std::vector<UserRow> g_users;
    int   g_userScroll = 0;
    std::string g_selected;                            // CP1254 character name
    struct Info
    {
        bool valid = false, found = false, ext = false;
        std::string name, account, clan, ip;
        BYTE level = 0, nation = 0, race = 0, zone = 0, auth = 0, muted = 0, noattack = 0, stat[5] = {};
        WORD cls = 0, x = 0, z = 0, attack = 0, defence = 0, resist[6] = {};
        DWORD np = 0, monthly = 0, cash = 0, gold = 0, manner = 0;
        int hp = 0, maxhp = 0, mp = 0, maxmp = 0;
        short points = 0, weapon[6] = {};
        BYTE skill[5] = {};                            // free, 1, 2, 3, master
        long long exp = 0, maxexp = 0;
    } g_info, g_self;

    // user editor: values that can be typed over and saved with APPLY (server [8] field numbers)
    struct EditField { const char* id; BYTE field; };
    const EditField kEditFields[] = {
        { "Text_Level", 1 }, { "Text_Exp", 2 }, { "Text_RealmPoint", 3 }, { "Text_bonuscash", 4 }, { "Text_BonusPoint", 5 }, { "Text_kc", 6 },
        { "Text_Strength", 7 }, { "Text_Stamina", 8 }, { "Text_Dexterity", 9 }, { "Text_Intelligence", 10 }, { "Text_MagicAttack", 11 },
        { "Text_DaggerAc", 12 }, { "Text_SwordAc", 13 }, { "Text_MaceAc", 14 }, { "Text_SpearAc", 15 }, { "Text_AxeAc", 16 }, { "Text_Manner", 17 } };
    std::map<std::string, bool> g_dirty;               // typed over, not applied yet
    const EditField* FindField(const std::string& id)
    {
        for (const EditField& f : kEditFields) if (id == f.id) return &f;
        return nullptr;
    }
    long long RawValue(const Info& in, BYTE field)
    {
        switch (field)
        {
        case 1: return in.level;  case 2: return in.exp;  case 3: return in.np;  case 4: return in.monthly;
        case 5: return in.points; case 6: return in.cash;
        case 7: return in.stat[0]; case 8: return in.stat[1]; case 9: return in.stat[2]; case 10: return in.stat[3]; case 11: return in.stat[4];
        case 12: return in.skill[0]; case 13: return in.skill[1]; case 14: return in.skill[2]; case 15: return in.skill[3]; case 16: return in.skill[4];
        case 17: return in.manner;
        }
        return 0;
    }
    std::wstring g_status = L"\" ile açılır / kapanır.";
    COLORREF g_statusColor = C_INFO;

    struct Hit { RECT r; std::function<void()> act; std::string edit; };
    std::vector<Hit> g_hits;                           // rebuilt on every paint

    // ------------------------------------------------------------------ helpers
    std::wstring Wide(const std::string& s)
    {
        if (s.empty()) return std::wstring();
        int n = MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), nullptr, 0);
        std::wstring w(n, L'\0'); MultiByteToWideChar(1254, 0, s.data(), (int)s.size(), &w[0], n);
        return w;
    }
    std::string Narrow(const std::wstring& w)
    {
        if (w.empty()) return std::string();
        int n = WideCharToMultiByte(1254, 0, w.data(), (int)w.size(), nullptr, 0, nullptr, nullptr);
        std::string s(n, '\0'); WideCharToMultiByte(1254, 0, w.data(), (int)w.size(), &s[0], n, nullptr, nullptr);
        return s;
    }
    std::wstring Trim(std::wstring s)
    {
        while (!s.empty() && iswspace(s.back())) s.pop_back();
        while (!s.empty() && iswspace(s.front())) s.erase(s.begin());
        return s;
    }
    std::string E(const char* id) { return Narrow(Trim(g_edit[id])); }   // edit value, CP1254
    std::wstring Fmt(const wchar_t* f, ...)
    {
        wchar_t b[512]; va_list a; va_start(a, f); _vsnwprintf_s(b, _TRUNCATE, f, a); va_end(a); return b;
    }
    std::wstring Num(long long v) { return Fmt(L"%lld", v); }

    bool IsGM()
    {
        __try
        {
            BYTE* me = *(BYTE**)KO_PTR_ME;
            return me && *(DWORD*)(me + 0x6DC) == 0;
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
    }
    bool GameEditFocused()
    {
        __try { return *(DWORD*)KO_FOCUSED_EDIT != 0; }
        __except (EXCEPTION_EXECUTE_HANDLER) { return true; }
    }

    void SetStatus(const std::wstring& s, COLORREF c)
    {
        EnterCriticalSection(&g_lock); g_status = s; g_statusColor = c; LeaveCriticalSection(&g_lock);
    }

    void Queue(std::vector<BYTE> p)
    {
        EnterCriticalSection(&g_lock); g_sendQ.push_back(std::move(p)); LeaveCriticalSection(&g_lock);
        Proxy_RequestFlush();
    }
    void PutStr(std::vector<BYTE>& p, const std::string& s)
    {
        WORD n = (WORD)min(s.size(), (size_t)400);
        p.push_back((BYTE)n); p.push_back((BYTE)(n >> 8));
        p.insert(p.end(), s.begin(), s.begin() + n);
    }
    void Command(const std::string& cmd)
    {
        std::vector<BYTE> p = { 0xE9, GM_SUBOP, 3 };
        PutStr(p, cmd);
        Queue(p);
        SetStatus(L"Gönderildi: " + Wide(cmd), C_INFO);
        char lb[256]; sprintf_s(lb, "GMPANEL: %s", cmd.c_str()); Log(lb);
    }
    // "+cmd <selected user> args": the user editor's selection
    void UserCommand(const char* cmd, const std::string& extra = std::string())
    {
        if (g_selected.empty()) { SetStatus(L"Önce User Editor sayfasında bir oyuncu seç.", C_ERR); return; }
        Command(std::string(cmd) + " " + g_selected + (extra.empty() ? "" : " " + extra));
    }
    void NeedValue(const char* id, const wchar_t* what, std::function<void(const std::string&)> f)
    {
        std::string v = E(id);
        if (v.empty()) { SetStatus(Fmt(L"%s alanını doldur.", what), C_ERR); return; }
        f(v);
    }
    std::string Or(const char* id, const char* def) { std::string v = E(id); return v.empty() ? def : v; }
    void RequestList() { Queue({ 0xE9, GM_SUBOP, 1 }); }
    void RequestInfo(const std::string& name, bool self = false)
    {
        EnterCriticalSection(&g_lock); g_expect.push_back(self); LeaveCriticalSection(&g_lock);
        std::vector<BYTE> p = { 0xE9, GM_SUBOP, 2 }; PutStr(p, self ? std::string() : name); Queue(p);
    }
    void SetAuth(BYTE auth)
    {
        if (g_selected.empty()) { SetStatus(L"Önce bir oyuncu seç.", C_ERR); return; }
        std::vector<BYTE> p = { 0xE9, GM_SUBOP, 4 }; PutStr(p, g_selected); p.push_back(auth); Queue(p);
    }

    const wchar_t* NationName(BYTE n) { return n == 1 ? L"Karus" : n == 2 ? L"El Morad" : L"-"; }
    const wchar_t* AuthName(BYTE a) { return a == 0 ? L"GAME MASTER" : a == 1 ? L"PLAYER" : a == 2 ? L"GM USER" : a == 255 ? L"BANNED" : L"?"; }

    std::vector<int> FilteredUsers()                   // indexes into g_users matching the search edit
    {
        std::wstring q = Trim(g_edit["edit_search"]);
        CharLowerBuffW(&q[0], (DWORD)q.size());
        std::vector<int> out;
        for (int i = 0; i < (int)g_users.size(); i++)
        {
            if (!q.empty())
            {
                std::wstring n = Wide(g_users[i].name); CharLowerBuffW(&n[0], (DWORD)n.size());
                if (n.find(q) == std::wstring::npos) continue;
            }
            out.push_back(i);
        }
        return out;
    }

    // ------------------------------------------------------------------ sprites (premultiplied BGRA)
    struct Sprite { HDC dc = nullptr; HBITMAP bmp = nullptr; int w = 0, h = 0; };
    std::map<std::string, Sprite> g_sprites;
    bool LoadPus(const char* path, Sprite& s)
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
    void LoadSkin()
    {
        char dir[MAX_PATH]; GetModuleFileNameA(nullptr, dir, MAX_PATH);
        strcpy_s(strrchr(dir, '\\') + 1, 48, "HopeGuard\\gm_ui");
        int ok = 0, all = 0;
        for (const GmuSprite& s : GMU_SPRITES)
        {
            char path[MAX_PATH]; sprintf_s(path, "%s\\%s.pus", dir, s.name);
            Sprite sp; all++;
            if (LoadPus(path, sp)) { g_sprites[s.name] = sp; ok++; }
        }
        char msg[96]; sprintf_s(msg, "GMPANEL: skin sprites loaded %d/%d", ok, all); Log(msg);
    }
    void Blit(const char* name, int x, int y, int w = 0, int h = 0)
    {
        if (!name) return;
        auto it = g_sprites.find(name);
        if (it == g_sprites.end()) return;
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        AlphaBlend(g_memDC, x, y, w ? w : it->second.w, h ? h : it->second.h, it->second.dc, 0, 0, it->second.w, it->second.h, bf);
    }

    // ------------------------------------------------------------------ text (uif fonts: Arial, point size)
    std::map<int, HFONT> g_fonts;
    HFONT Font(int pt)
    {
        auto it = g_fonts.find(pt);
        if (it != g_fonts.end()) return it->second;
        HFONT f = CreateFontW(-MulDiv(pt, 96, 72), 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY, DEFAULT_PITCH | FF_SWISS, L"Arial");
        g_fonts[pt] = f;
        return f;
    }
    // uif string style -> DrawText flags (UISTYLE_STRING_*)
    UINT Flags(unsigned style)
    {
        UINT f = DT_NOPREFIX | DT_END_ELLIPSIS;
        if (style & 0x00100000) f |= DT_SINGLELINE; else f |= DT_WORDBREAK;
        if (style & 0x00800000) f |= DT_CENTER; else if ((style & 0x00400000) && !(style & 0x00200000)) f |= DT_RIGHT;
        if (style & 0x04000000) f |= DT_VCENTER | DT_SINGLELINE; else if (style & 0x02000000) f |= DT_BOTTOM | DT_SINGLELINE;
        return f;
    }
    COLORREF Col(unsigned argb) { return RGB((argb >> 16) & 255, (argb >> 8) & 255, argb & 255); }
    void Text(const std::wstring& s, int l, int t, int r, int b, COLORREF c, int pt, UINT flags)
    {
        if (s.empty()) return;
        HGDIOBJ of = SelectObject(g_memDC, Font(pt));
        SetBkMode(g_memDC, TRANSPARENT);
        RECT rc = { l + 1, t + 1, r + 1, b + 1 };      // KO strings have a 1px shadow
        SetTextColor(g_memDC, RGB(0, 0, 0)); DrawTextW(g_memDC, s.c_str(), (int)s.size(), &rc, flags);
        rc = { l, t, r, b };
        SetTextColor(g_memDC, c); DrawTextW(g_memDC, s.c_str(), (int)s.size(), &rc, flags);
        SelectObject(g_memDC, of);
    }

    int AddHit(const RECT& r, std::function<void()> act, const std::string& edit = std::string())
    {
        g_hits.push_back({ r, std::move(act), edit });
        return (int)g_hits.size() - 1;
    }

    // ------------------------------------------------------------------ actions
    // user editor APPLY: every typed-over value -> [8][name][field][value]; then the fresh values
    void ApplyEdits()
    {
        if (g_selected.empty()) { SetStatus(L"Önce bir oyuncu seç.", C_ERR); return; }
        int sent = 0;
        for (const EditField& f : kEditFields)
        {
            if (!g_dirty[f.id]) continue;
            g_dirty[f.id] = false;
            std::string v = E(f.id);
            if (v.empty() || v.find_first_not_of("0123456789") != std::string::npos) continue;
            long long n = _atoi64(v.c_str());
            if (n == RawValue(g_info, f.field)) continue;
            std::vector<BYTE> p = { 0xE9, GM_SUBOP, 8 }; PutStr(p, g_selected); p.push_back(f.field);
            for (int i = 0; i < 8; i++) p.push_back((BYTE)(n >> (i * 8)));
            Queue(p); sent++;
        }
        RequestInfo(g_selected);
        SetStatus(sent ? Fmt(L"%d değer gönderildi.", sent) : L"Değişen değer yok; bilgiler yenilendi.", C_INFO);
    }
    void SelectUser(const std::string& name)
    {
        if (name != g_selected) g_dirty.clear();
        g_selected = name;
        RequestInfo(name);
        SetStatus(L"Seçildi: " + Wide(name), C_INFO);
    }
    void GoPage(int p)
    {
        g_page = p; g_focus.clear();
        if (p == P_USER) RequestList();
        if (p == P_HOME) RequestInfo(std::string(), true);
    }
    void SendNotice()
    {
        std::string m = E("edit_mesaj");
        if (m.empty()) { SetStatus(L"Mesaj yaz.", C_ERR); return; }
        if (m[0] == '+' || m[0] == '/') { Command(m); g_edit["edit_mesaj"].clear(); return; }
        int type = atoi(Or("edit_type", "1").c_str());
        switch (type)
        {
        case 2: Command("+pmall GM " + m); break;
        case 3: Command("/noticeall " + m); break;
        case 4: Command("/permanent " + m); break;
        case 5: Command("/offpermanent"); break;
        default: Command("/notice " + m); break;
        }
    }
    void SendBots()
    {
        // Leader / Genie / Class: HopeGuard tab, Bot Options
        std::string c = Or("edit_count", "1"), t = Or("edit_Time", "60"), l = Or("edit_lvl", "1"), pkl = Or("edit_lvl", "83");
        std::string lead = Or("x_bleader", "0"), genie = Or("x_bgenie", "1"), cls = Or("x_bclass", "0");
        switch (atoi(Or("edit_Type", "0").c_str()))
        {
        case 1: Command("+miningbotspawn " + c + " " + t + " " + l); break;
        case 2: Command("+fishingbotspawn " + c + " " + t + " " + l); break;
        case 3: Command("+farmbotspawn " + c + " " + t + " " + l + " " + lead + " " + genie + " " + cls); break;
        case 4: Command("+pkbotspawn " + c + " " + t + " " + pkl + " 1 " + cls); break;
        case 5: Command("+pkbotspawn " + c + " " + t + " " + pkl + " 2 " + cls); break;
        case 6: Command("+eventbotfill " + c); break;
        case 7: NeedValue("edit_count", L"Count (merchant index)", [](const std::string& v) { Command("+merchantbotspawn " + v); }); break;
        case 8: Command("+botkill"); break;
        case 9: Command("+allbotkill"); break;
        default: SetStatus(L"Bot Type: 1 mining, 2 fishing, 3 farm, 4 PK Karus, 5 PK Human, 6 event, 7 merchant, 8 seçili botu, 9 hepsini çıkar.", C_ERR); break;
        }
    }
    void QuickAction(int i)
    {
        switch (i)
        {
        case 0: UserCommand("+partytp"); break;
        case 1: UserCommand("+info"); break;
        case 2: UserCommand("+disable"); break;
        case 3: UserCommand("+allow"); break;
        case 4: UserCommand("+hapis"); break;
        case 5: UserCommand("+unblock"); break;
        case 6: Command("+exp_add 0"); break;
        case 7: Queue({ 0xE9, GM_SUBOP, 9 }); break;          // ride / dismount the commander horse
        case 8: Command("+count"); break;
        case 9: Command("+mode_gamemaster"); break;
        case 10: Command("/offpermanent"); break;
        case 11: Command("+beefclose"); break;
        case 12: Command("+juraidclose"); break;
        case 13: Command("+borderclose"); break;
        case 14: Command("+chaosclose"); break;
        case 15: Command("+ftclose"); break;
        case 16: Command("+lotteryclose"); break;
        case 17: Command("+exp_add 0"); Command("+np_add 0"); Command("+money_add 0"); Command("+drop_add 0"); break;
        case 18: Command("+kill"); break;
        case 19: Command("+botkill"); break;
        case 20: Command("+allbotkill"); break;
        case 21: Command("+cindclose"); break;
        case 22: Command("+crclose"); break;
        case 23: Command("+countzone"); break;
        case 24: Command("+close"); break;
        case 25: Command("+cswclose"); break;
        case 26: Command("+ultimaend"); break;
        case 27: Command("+manesend"); break;
        }
    }
    void OnButton(int page, const std::string& id)
    {
        if (page < 0)
        {
            if (id == "btn_close") { PostMessageW(g_hWnd, WM_GM_HIDE, 0, 0); return; }
            static const std::pair<const char*, int> tabs[] = { { "btn_home", P_HOME }, { "btn_user_editor", P_USER }, { "btn_events", P_EVENT },
                { "btn_table", P_RELOAD }, { "btn_lottery", P_LOTTERY }, { "btn_commands", P_COMMANDS }, { "btn_bot", P_BOTS }, { "btn_extra", P_EXTRA } };
            for (auto& t : tabs) if (id == t.first) { GoPage(t.second); return; }
            return;
        }
        switch (page)
        {
        case P_HOME:
            if (id == "btn_beuser") Command("+gm");
            else if (id == "btn_unvisible") Queue({ 0xE9, GM_SUBOP, 10 });   // visible / hidden for players, still GM
            break;
        case P_USER:
        {
            std::vector<int> list = FilteredUsers();
            if (id.size() == 3 && id[0] == 'I' && id[1] == 'D')
            {
                int i = g_userScroll + (id[2] - '1');
                if (i >= 0 && i < (int)list.size()) SelectUser(g_users[list[i]].name);
            }
            else if (id == "scroll_up") g_userScroll = max(0, g_userScroll - 1);
            else if (id == "scroll_down") g_userScroll = min(max(0, (int)list.size() - 4), g_userScroll + 1);
            else if (id == "btn_search")
            {
                RequestList(); g_userScroll = 0;
                // an exact name is the target even when that player is offline (unban, ban, ...)
                std::string q = E("edit_search");
                if (!q.empty() && q.find(' ') == std::string::npos)
                {
                    std::string pick = q;
                    for (auto& u : g_users) if (_stricmp(u.name.c_str(), q.c_str()) == 0) { pick = u.name; break; }
                    SelectUser(pick);
                }
            }
            else if (id == "btn_apply") { ApplyEdits(); RequestList(); }
            else if (id == "btn_ban") UserCommand("+block");
            else if (id == "btn_ban2") NeedValue("edit_banday", L"BAN DAYS", [](const std::string& v) { UserCommand("+block", v); });
            else if (id == "btn_mute") UserCommand("+mute");
            else if (id == "btn_unmute") UserCommand("+unmute");
            else if (id == "btn_kick") { if (g_selected.empty()) SetStatus(L"Önce bir oyuncu seç.", C_ERR); else Command("/kill " + g_selected); }
            else if (id == "btn_summon") UserCommand("+summonuser");
            else if (id == "btn_tpon") UserCommand("+tpon");
            else if (id == "USER") g_role = 0;
            else if (id == "GM") g_role = 1;
            else if (id == "SHERIFF") g_role = 2;
            else if (id == "btn_apply2") { if (g_role == 1) UserCommand("+changegm"); else SetAuth(g_role == 0 ? 1 : 2); }
            break;
        }
        case P_EVENT:
            if (id == "btn_royal") Command("+ultimaopen");
            else if (id == "btn_beef") Command("+beefopen");
            else if (id == "btn_jr") Command("+juraidopen");
            else if (id == "btn_bdw") Command("+borderopen");
            else if (id == "btn_chaos") Command("+chaosopen");
            else if (id == "btn_snow") Command("+snow");
            else if (id == "btn_csw") Command("+csw");
            else if (id == "btn_closecsw") Command("+cswclose");
            else if (id == "btn_adream") NeedValue("edit_minuteadream", L"FT type", [](const std::string& v) { Command("+ftopen " + v); });
            else if (id == "btn_closeadream") Command("+ftclose");
            else if (id == "btn_cz") NeedValue("edit_minutecz", L"Cindirella ID", [](const std::string& v) { Command("+cindopen " + v); });
            else if (id == "btn_closecz") Command("+cindclose");
            else if (id == "btn_base") Command("+manesopen");
            else if (id == "btn_closebase") Command("+manesend");
            else if (id == "btn_UTC") Command("+manesstart");
            else if (id == "btn_DM") Command("+ultimastart");
            else if (id == "btn_Clanws") Command("+ultimaend");
            else if (id == "btn_warresult") NeedValue("edit_warresult", L"War Result (1 Karus / 2 Human)", [](const std::string& v) { Command("+warresult " + v); });
            else if (id == "btn_open") NeedValue("edit_warzone", L"War Zone (1-6)", [](const std::string& v) { Command("+open" + v); });
            else if (id == "btn_captain") Command("+captain");
            else if (id == "btn_close") Command("+close");
            else if (id == "btn_CR") NeedValue("edit_hedefcr", L"Event ID", [](const std::string& v) { Command("+cropen " + v); });
            else if (id == "btn_CloseCR") Command("+crclose");
            else if (id == "btn_CloseEvents") { Queue({ 0xE9, GM_SUBOP, 5 }); Command("+beefclose"); }
            break;
        case P_RELOAD:
            if (id.compare(0, 6, "reload") == 0)
            {
                int n = atoi(id.c_str() + 6);
                if (n >= 0 && n < (int)(sizeof(GMU_RELOAD_CMDS) / sizeof(GMU_RELOAD_CMDS[0])) && *GMU_RELOAD_CMDS[n]) Command(GMU_RELOAD_CMDS[n]);
            }
            break;
        case P_LOTTERY:
            if (id == "btn_Start") NeedValue("Edit_ticket", L"LOTTERY ID", [](const std::string& v) { Command("+lottery " + v); });
            else if (id == "btn_Stop") Command("+lotteryclose");
            break;
        case P_COMMANDS:
            if (id == "btn_send") SendNotice();
            else if (id == "btn_Tall") NeedValue("edit_zone1", L"ZoneID 1", [](const std::string& z) { std::string t = E("edit_zone2"); Command("+tpall " + z + (t.empty() ? "" : " " + t)); });
            else if (id == "btn_alldiscount") Command("/alldiscount");
            else if (id == "btn_osanta") Command("/santa");
            else if (id == "btn_offsanta") Command("/santaclose");
            else if (id == "btn_oangel") Command("/angel");
            else if (id == "btn_discount") Command("/discount");
            else if (id == "btn_down") Command("/offdiscount");
            break;
        case P_BOTS:
            if (id == "btn_bot") SendBots();
            break;
        case P_EXTRA:
        {
            auto amount = [](const char* cmd) { NeedValue("x_amount", L"Amount", [cmd](const std::string& v) { UserCommand(cmd, v); }); };
            auto bonus = [](const char* cmd) { NeedValue("x_bonus", L"Percent", [cmd](const std::string& v) { Command(std::string(cmd) + " " + v); }); };
            if (id == "x_info") { if (g_selected.empty()) SetStatus(L"Önce User Editor sayfasında bir oyuncu seç.", C_ERR); else RequestInfo(g_selected); }
            else if (id == "x_level") amount("+level");
            else if (id == "x_np") amount("+np");
            else if (id == "x_kc") amount("+kc");
            else if (id == "x_exp") amount("+exp");
            else if (id == "x_giveitem") NeedValue("x_itemid", L"Item ID", [](const std::string& v) {
                std::string c = E("x_itemcount"), t = E("x_itemtime");
                UserCommand("+give", v + " " + (c.empty() ? "1" : c) + " " + (t.empty() ? "0" : t));
            });
            else if (id == "x_gozone") NeedValue("x_zone", L"Zone", [](const std::string& v) { Command("+zone " + v); });
            else if (id == "x_giveself") NeedValue("x_myitem", L"Item ID", [](const std::string& v) { std::string c = E("x_myitemcount"); Command("+item " + v + (c.empty() ? "" : " " + c)); });
            else if (id == "x_bowl")
            {
                std::string z = E("x_bowlzone"), t = E("x_bowltime");
                if (z.empty() || t.empty()) SetStatus(L"Bowl Event: zone ve dakika gir.", C_ERR); else Command("+bowlevent " + z + " " + t);
            }
            else if (id == "x_run")
            {
                std::string c = E("x_cmd");
                if (c.size() < 2 || (c[0] != '+' && c[0] != '/')) SetStatus(L"Komut + veya / ile başlamalı.", C_ERR);
                else { Command(c); g_edit["x_cmd"].clear(); }
            }
            else if (id == "x_bonusoff") { Command("+exp_add 0"); Command("+np_add 0"); Command("+money_add 0"); Command("+drop_add 0"); }
            else if (id == "x_bexp") bonus("+exp_add");
            else if (id == "x_bnp") bonus("+np_add");
            else if (id == "x_bcoin") bonus("+money_add");
            else if (id == "x_bdrop") bonus("+drop_add");
            else if (id == "x_pop") { std::string z = E("x_popzone"); Command(z.empty() ? std::string("+countzone") : "+countzone " + z); }
            else if (id == "x_pmsend")
            {
                std::string t = E("x_pmtitle"), m = E("x_pmmsg");
                if (t.empty() || m.empty()) { SetStatus(L"PM başlığı ve mesajı gir.", C_ERR); return; }
                for (auto& ch : t) if (ch == ' ') ch = '_';
                Command("+pmall " + t + " " + m);
            }
            else if (id == "x_r_social") Command("+reloadsocial");
            else if (id == "x_r_bug") Command("+reloadbug");
            else if (id == "x_r_clanp") Command("+reloadclanpnotice");
            else if (id == "x_r_item") Command("+reload_item");
            else if (id == "x_r_zoneon") Command("+reloadzoneon");
            break;
        }
        }
    }

    // ------------------------------------------------------------------ dynamic strings (by uif id)
    bool DynText(int page, const char* id, std::wstring& out)
    {
        if (!*id) return false;
        std::string s = id;
        EnterCriticalSection(&g_lock); Info me = g_self, in = g_info; LeaveCriticalSection(&g_lock);
        if (page == P_HOME)
        {
            if (!me.found) { if (s.compare(0, 4, "txt_") == 0) { out.clear(); return true; } return false; }
            if (s == "txt_level") out = Num(me.level);
            else if (s == "txt_authority") out = AuthName(me.auth);
            else if (s == "txt_uname") out = Wide(me.name);
            else if (s == "txt_nation") out = NationName(me.nation);
            else if (s == "txt_np") out = Num(me.np);
            else if (s == "txt_kcbonus") out = Num(me.monthly);
            else if (s == "txt_exp") out = Num(me.exp);
            else if (s == "txt_kc") out = Num(me.cash);
            else return false;
            return true;
        }
        if (page == P_USER && s.compare(0, 5, "Text_") == 0)
        {
            if (!in.found) { out = s == "Text_Id" ? (g_selected.empty() ? std::wstring(L"User Edit State") : Wide(g_selected)) : L""; return true; }
            if (FindField(s) && (g_dirty[s] || g_focus == s))   // being typed over
            {
                out = g_edit[s] + ((g_focus == s && (GetTickCount() / 500) % 2) ? L"|" : L"");
                return true;
            }
            if (s == "Text_Id") out = Wide(in.name);
            else if (s == "Text_Level") out = Num(in.level);
            else if (s == "Text_Nation") out = NationName(in.nation);
            else if (s == "Text_Exp") out = Fmt(L"%lld / %lld", in.exp, in.maxexp);
            else if (s == "Text_RealmPoint") out = Num(in.np);
            else if (s == "Text_bonuscash") out = Num(in.monthly);
            else if (s == "Text_BonusPoint") out = Num(in.points);
            else if (s == "Text_kc") out = Num(in.cash);
            else if (s == "Text_AP") out = Num(in.attack);
            else if (s == "Text_GP") out = Num(in.defence);
            else if (s == "Text_Manner") out = Num(in.manner);
            else if (s == "Text_Intelligence") out = Num(in.stat[3]);
            else if (s == "Text_Strength") out = Num(in.stat[0]);
            else if (s == "Text_Stamina") out = Num(in.stat[1]);              // label STA
            else if (s == "Text_Dexterity") out = Num(in.stat[2]);
            else if (s == "Text_MagicAttack") out = Num(in.stat[4]);          // label CHA
            else if (s == "Text_DaggerAc") out = Num(in.skill[0]);            // Free SP
            else if (s == "Text_SwordAc") out = Num(in.skill[1]);             // Skill 1
            else if (s == "Text_MaceAc") out = Num(in.skill[2]);              // Skill 2
            else if (s == "Text_SpearAc") out = Num(in.skill[3]);             // Skill 3
            else if (s == "Text_AxeAc") out = Num(in.skill[4]);               // Master
            else if (s == "Text_BowAc") out = Fmt(L"%d/%d", in.hp, in.maxhp); // HP
            else if (s == "Text_RegistFire") out = Num(in.resist[0]);
            else if (s == "Text_RegistIce") out = Num(in.resist[1]);
            else if (s == "Text_RegistLightR") out = Num(in.resist[2]);
            else if (s == "Text_RegistMagic") out = Num(in.resist[3]);
            else if (s == "Text_RegistCurse") out = Num(in.resist[4]);
            else if (s == "Text_RegistPoison") out = Num(in.resist[5]);
            else return false;
            return true;
        }
        return false;
    }

    // ------------------------------------------------------------------ paint
    void Paint()
    {
        if (!g_hWnd || !g_memDC) return;
        g_hits.clear();
        memset(g_bits, 0, (size_t)W * H * 4);
        for (const GmuBg& b : GMU_BG) if (b.page == -1 || b.page == g_page) Blit(b.spr, b.x, b.y);

        // buttons (tab of the open page and the chosen role stay pressed)
        static const char* tabOf[] = { "btn_home", "btn_user_editor", "btn_events", "btn_table", "btn_lottery", "btn_commands", "btn_bot", "btn_extra" };
        static const char* roleOf[] = { "USER", "GM", "SHERIFF" };
        std::vector<int> users = g_page == P_USER ? FilteredUsers() : std::vector<int>();
        g_userScroll = max(0, min(g_userScroll, max(0, (int)users.size() - 4)));
        struct Label { std::wstring s; RECT r; COLORREF c; int pt; UINT f; };
        std::vector<Label> labels;
        for (const GmuBtn& b : GMU_BTNS)
        {
            if (b.page != -1 && b.page != g_page) continue;
            RECT r = { b.l, b.t, b.r, b.b };
            std::string id = b.id; int page = b.page;
            std::wstring lab = b.label;
            bool sel = (page == -1 && id == tabOf[g_page]) || (page == P_USER && id == roleOf[g_role]);
            if (page == P_USER && id.size() == 3 && id[0] == 'I' && id[1] == 'D')
            {
                int i = g_userScroll + (id[2] - '1');
                if (i < (int)users.size())
                {
                    const UserRow& u = g_users[users[i]];
                    lab = Wide(u.name) + Fmt(L"  (Lv %u, %s, zone %u)", u.level, u.nation == 1 ? L"Karus" : u.nation == 2 ? L"Human" : L"-", u.zone);
                }
                else lab.clear();
                sel = i < (int)users.size() && g_users[users[i]].name == g_selected;
            }
            int hit = AddHit(r, [page, id] { OnButton(page, id); });
            int st = (g_pressed == hit || sel) ? 1 : g_hot == hit ? 2 : 0;
            Blit(b.spr[st] ? b.spr[st] : b.spr[0], b.l, b.t, b.r - b.l, b.b - b.t);
            labels.push_back({ lab, { b.ll, b.lt, b.lr, b.lb }, Col(b.color), b.fontH, Flags(b.style) });
        }
        // GDI text leaves alpha at 0: remember the image alpha and put it back after the text
        BYTE* px = (BYTE*)g_bits;
        GdiFlush();
        for (int i = 0; i < W * H; i++) g_alpha[i] = px[i * 4 + 3];

        for (const GmuStr& s : GMU_STRS)
        {
            if (s.page != -1 && s.page != g_page) continue;
            std::wstring t = s.text;
            DynText(s.page, s.id, t);
            COLORREF c = Col(s.color);
            if (s.page == P_USER && g_info.found && FindField(s.id))
            {
                AddHit({ s.l - 2, s.t - 3, s.r + 2, s.b + 3 }, nullptr, s.id);   // click = type a new value
                if (g_dirty[s.id] || g_focus == s.id) c = RGB(255, 230, 90);
            }
            Text(t, s.l, s.t, s.r, s.b, c, s.fontH, Flags(s.style));
        }
        for (auto& l : labels) Text(l.s, l.r.left, l.r.top, l.r.right, l.r.bottom, l.c, l.pt, l.f);
        for (const GmuEdit& e : GMU_EDITS)
        {
            if (e.page != g_page) continue;
            RECT r = { e.l, e.t, e.r, e.b };
            AddHit(r, nullptr, e.id);
            bool f = g_focus == e.id;
            const std::wstring& v = g_edit[e.id];
            bool caret = f && (GetTickCount() / 500) % 2;
            if (v.empty() && !f) Text(e.hint, e.l + 5, e.t, e.r - 4, e.b, C_HINT, 9, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
            else Text(v + (caret ? L"|" : L""), e.l + 5, e.t, e.r - 4, e.b, RGB(255, 255, 255), 9, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS);
        }
        if (g_page == P_HOME)
        {
            int n = 0;
            for (const GmuRow& r : GMU_ROWS)
            {
                RECT rc = { r.l, r.t, r.r, r.b };
                int i = n++;
                int hit = AddHit(rc, [i] { QuickAction(i); });
                bool hot = g_hot == hit, down = g_pressed == hit;
                Text(r.label, r.l + 3, r.t, r.r - 34, r.b, down ? RGB(255, 220, 120) : hot ? RGB(255, 255, 160) : RGB(255, 255, 255), 9, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS);
                Text(L"Run", r.r - 34, r.t, r.r - 3, r.b, hot ? RGB(120, 255, 120) : RGB(0, 200, 0), 9, DT_SINGLELINE | DT_VCENTER | DT_RIGHT | DT_NOPREFIX);
            }
            std::wstring who = g_selected.empty() ? L"Target: select a player in User Editor" : L"Target: " + Wide(g_selected);
            Text(who, 445, 496, 805, 512, RGB(255, 200, 120), 9, DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_NOPREFIX);
        }
        EnterCriticalSection(&g_lock); std::wstring st = g_status; COLORREF sc = g_statusColor; LeaveCriticalSection(&g_lock);
        Text(st, STATUS_RC.left, STATUS_RC.top, STATUS_RC.right, STATUS_RC.bottom, sc, 9, DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_NOPREFIX | DT_END_ELLIPSIS);

        GdiFlush();
        for (int i = 0; i < W * H; i++) px[i * 4 + 3] = g_alpha[i];
        RECT wr; GetWindowRect(g_hWnd, &wr);
        POINT dst = { wr.left, wr.top }, src = { 0, 0 }; SIZE sz = { W, H };
        BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
        UpdateLayeredWindow(g_hWnd, nullptr, &dst, &sz, g_memDC, &src, 0, &bf, ULW_ALPHA);
    }

    // ------------------------------------------------------------------ window
    HWND FindGameWindow() { return Proxy_GameWnd(); }
    void SetFocusEdit(const std::string& id)
    {
        if (g_focus == id) return;
        g_focus = id;
        if (const EditField* f = FindField(id))        // user editor value: start from the current one
            if (!g_dirty[id]) g_edit[id] = Num(RawValue(g_info, f->field));
        if (!id.empty()) SetForegroundWindow(g_hWnd);  // keyboard to us while typing
        else if (HWND game = FindGameWindow()) SetForegroundWindow(game);
    }
    void Hide()
    {
        SetFocusEdit(std::string());
        g_open = false;
        ShowWindow(g_hWnd, SW_HIDE);
        if (HWND game = FindGameWindow()) SetForegroundWindow(game);
    }
    void Show()
    {
        bool first = !g_open;
        g_open = true;
        if (first)
        {
            RECT gr = { 0, 0, 1024, 768 };
            if (HWND game = FindGameWindow()) GetClientRect(game, &gr), MapWindowPoints(game, nullptr, (POINT*)&gr, 2);
            static bool placed = false;
            if (!placed)
            {
                placed = true;
                SetWindowPos(g_hWnd, HWND_TOPMOST, gr.left + max(0, (int)((gr.right - gr.left) - W) / 2), gr.top + max(0, (int)((gr.bottom - gr.top) - H) / 2),
                    W, H, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            }
            else SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
            RequestInfo(std::string(), true);
            RequestList();
        }
        Paint();
    }
    void SyncVisibility()
    {
        HWND game = FindGameWindow(); DWORD pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &pid);
        if (g_open && !IsGM()) { Hide(); return; }     // authority dropped
        bool want = g_open && game && !IsIconic(game) && pid == GetCurrentProcessId();
        if (want != (IsWindowVisible(g_hWnd) != FALSE))
        {
            if (want) { SetWindowPos(g_hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW); Paint(); }
            else ShowWindow(g_hWnd, SW_HIDE);
        }
        POINT p; CURSORINFO ci = { sizeof(ci) };
        if (game && GetCursorPos(&p) && WindowFromPoint(p) == game && GetCursorInfo(&ci) && (ci.flags & CURSOR_SHOWING) && ci.hCursor)
            g_gameCursor = ci.hCursor;
        if (!g_focus.empty() && IsWindowVisible(g_hWnd)) Paint();   // caret blink
    }
    int HitTest(int x, int y)
    {
        for (int i = (int)g_hits.size() - 1; i >= 0; i--)
        {
            const RECT& r = g_hits[i].r;
            if (x >= r.left && x < r.right && y >= r.top && y < r.bottom) return i;
        }
        return -1;
    }
    bool Opaque(int x, int y)                          // transparent corners of the frame let clicks through visually only
    {
        return x >= 0 && y >= 0 && x < W && y < H && g_alpha.size() == (size_t)W * H && g_alpha[y * W + x] > 0;
    }
    // Enter in an edit: the edit's natural button
    void EnterIn(const std::string& id)
    {
        if (id == "edit_mesaj" || id == "edit_type") SendNotice();
        else if (id == "edit_search") OnButton(P_USER, "btn_search");
        else if (id == "edit_banday") OnButton(P_USER, "btn_ban2");
        else if (id == "Edit_ticket") OnButton(P_LOTTERY, "btn_Start");
        else if (id == "x_cmd") OnButton(P_EXTRA, "x_run");
        else if (id == "x_pmmsg") OnButton(P_EXTRA, "x_pmsend");
        else if (id == "x_zone") OnButton(P_EXTRA, "x_gozone");
        else if (FindField(id)) { g_focus.clear(); ApplyEdits(); }
        SetFocusEdit(std::string());
    }
    void OnChar(WPARAM wp)
    {
        if (g_focus.empty()) { if (wp == VK_ESCAPE || wp == '"') Hide(); return; }
        std::string id = g_focus;
        if (wp == VK_RETURN) { EnterIn(id); Paint(); return; }
        if (wp == VK_ESCAPE && FindField(id)) g_dirty[id] = false;            // drop the typed value
        if (wp == VK_ESCAPE || wp == VK_TAB) { SetFocusEdit(std::string()); Paint(); return; }
        std::wstring& v = g_edit[id];
        if (FindField(id))                             // user editor value: digits only
        {
            if (wp == VK_BACK) { if (!v.empty()) v.pop_back(); }
            else if (wp >= L'0' && wp <= L'9' && v.size() < 12) v.push_back((wchar_t)wp);
            else return;
            g_dirty[id] = true;
            Paint();
            return;
        }
        if (wp == VK_BACK) { if (!v.empty()) v.pop_back(); }
        else if (wp == 0x16)                           // Ctrl+V
        {
            if (OpenClipboard(g_hWnd))
            {
                if (HANDLE hd = GetClipboardData(CF_UNICODETEXT))
                    if (const wchar_t* t = (const wchar_t*)GlobalLock(hd)) { v += t; GlobalUnlock(hd); }
                CloseClipboard();
                for (auto& c : v) if (c == L'\r' || c == L'\n') c = L' ';
                if (v.size() > 120) v.resize(120);
            }
        }
        else if (wp >= 32 && v.size() < 120) v.push_back((wchar_t)wp);
        if (id == "edit_search") g_userScroll = 0;
        Paint();
    }

    LRESULT CALLBACK WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
    {
        int x = GET_X_LPARAM(lp), y = GET_Y_LPARAM(lp);
        switch (msg)
        {
        case WM_MOUSEACTIVATE: return MA_NOACTIVATE;
        case WM_NCHITTEST:
        {
            POINT p = { x, y }; ScreenToClient(h, &p);
            return Opaque(p.x, p.y) ? HTCLIENT : HTTRANSPARENT;
        }
        case WM_MOUSEMOVE:
            if (g_dragging)
            {
                POINT p; GetCursorPos(&p); RECT wr; GetWindowRect(h, &wr);
                SetWindowPos(h, nullptr, wr.left + p.x - g_dragFrom.x, wr.top + p.y - g_dragFrom.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                g_dragFrom = p; return 0;
            }
            if (!g_track) { TRACKMOUSEEVENT t = { sizeof(t), TME_LEAVE, h, 0 }; g_track = TrackMouseEvent(&t) != FALSE; }
            if (int hit = HitTest(x, y); hit != g_hot) { g_hot = hit; Paint(); }
            return 0;
        case WM_MOUSELEAVE: g_track = false; if (g_hot != -1) { g_hot = -1; Paint(); } return 0;
        case WM_MOUSEWHEEL:
            if (g_page == P_USER) { g_userScroll -= GET_WHEEL_DELTA_WPARAM(wp) / WHEEL_DELTA; Paint(); }
            return 0;
        case WM_LBUTTONDOWN:
        {
            int hit = HitTest(x, y);
            SetCapture(h);
            if (hit < 0)
            {
                SetFocusEdit(std::string());
                POINT p = { x, y };
                if (PtInRect(&TITLE_RC, p) || y < 80 || y > H - 40 || x < 50 || x > W - 50) { g_dragging = true; GetCursorPos(&g_dragFrom); }
                Paint();
                return 0;
            }
            if (!g_hits[hit].edit.empty()) { SetFocusEdit(g_hits[hit].edit); Paint(); return 0; }
            SetFocusEdit(std::string());
            g_pressed = hit; Paint();
            return 0;
        }
        case WM_LBUTTONUP:
        {
            ReleaseCapture();
            if (g_dragging) { g_dragging = false; return 0; }
            int hit = HitTest(x, y), p = g_pressed; g_pressed = -1;
            if (p >= 0 && p == hit && g_hits[hit].act)
            {
                auto act = g_hits[hit].act;            // Paint rebuilds g_hits
                act();
            }
            if (g_open) Paint();
            return 0;
        }
        case WM_CHAR: OnChar(wp); return 0;
        case WM_KILLFOCUS: if (!g_focus.empty()) { g_focus.clear(); Paint(); } return 0;
        case WM_SETCURSOR:
            while (ShowCursor(TRUE) < 0) {}
            SetCursor(g_gameCursor ? g_gameCursor : LoadCursor(nullptr, IDC_ARROW));
            return TRUE;
        case WM_TIMER: SyncVisibility(); return 0;
        case WM_GM_SHOW: Show(); return 0;
        case WM_GM_HIDE: Hide(); return 0;
        case WM_GM_REFRESH: if (g_open) Paint(); return 0;
        case WM_CLOSE: Hide(); return 0;
        }
        return DefWindowProcW(h, msg, wp, lp);
    }

    DWORD WINAPI WindowThread(LPVOID)
    {
        WNDCLASSEXW wc = { sizeof(wc) };
        wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr);
        wc.hCursor = LoadCursor(nullptr, IDC_ARROW); wc.lpszClassName = L"NTT_GmPanel";
        RegisterClassExW(&wc);
        g_hWnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, wc.lpszClassName,
            L"HopeGuard GM", WS_POPUP, 0, 0, W, H, nullptr, nullptr, wc.hInstance, nullptr);
        if (!g_hWnd) { Log("GMPANEL: window creation failed"); return 0; }
        g_memDC = CreateCompatibleDC(nullptr);
        BITMAPINFO bmi = {}; bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), W, -H, 1, 32, BI_RGB };
        SelectObject(g_memDC, CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));
        g_alpha.assign((size_t)W * H, 0);
        LoadSkin();
        SetTimer(g_hWnd, 1, 250, nullptr);
        MSG m;
        while (GetMessageW(&m, nullptr, 0, 0)) { TranslateMessage(&m); DispatchMessageW(&m); }
        return 0;
    }
}

void GmPanel_Init()
{
    InitializeCriticalSection(&g_lock);
    CloseHandle(CreateThread(nullptr, 0, WindowThread, nullptr, 0, nullptr));
}

// ProxyWndProc (game thread), WM_CHAR: true when the " key opened / closed the panel (the game does not see it)
bool GmPanel_OnGameChar(WPARAM ch)
{
    if (ch != '"' || !g_hWnd) return false;
    if (GameEditFocused() || !IsGM()) return false;    // chat typing / players: the key is the game's
    PostMessageW(g_hWnd, g_open ? WM_GM_HIDE : WM_GM_SHOW, 0, 0);
    return true;
}

// recv hook (game thread): E9 F5 ...
void GmPanel_OnRecv(const BYTE* p, size_t len)
{
    if (len < 3 || p[0] != 0xE9 || p[1] != GM_SUBOP) return;
    size_t at = 3;
    auto u8 = [&](BYTE& v) { if (at + 1 > len) return false; v = p[at++]; return true; };
    auto u16 = [&](WORD& v) { if (at + 2 > len) return false; v = *(const WORD*)(p + at); at += 2; return true; };
    auto u32 = [&](DWORD& v) { if (at + 4 > len) return false; v = *(const DWORD*)(p + at); at += 4; return true; };
    auto u64 = [&](long long& v) { if (at + 8 > len) return false; v = *(const long long*)(p + at); at += 8; return true; };
    auto str = [&](std::string& v) {
        WORD n; if (!u16(n) || n > 512 || at + n > len) return false;
        v.assign((const char*)p + at, n); at += n; return true;
    };
    switch (p[2])
    {
    case 1:
    {
        WORD n = 0;
        if (!u16(n)) return;
        std::vector<UserRow> rows;
        for (WORD i = 0; i < n; i++)
        {
            UserRow r;
            if (!str(r.name) || !u8(r.level) || !u8(r.nation) || !u16(r.cls) || !u8(r.zone) || !u8(r.auth)) break;
            rows.push_back(r);
        }
        EnterCriticalSection(&g_lock);
        g_users.swap(rows);
        g_status = Fmt(L"%u oyuncu online.", (unsigned)g_users.size()); g_statusColor = C_INFO;
        LeaveCriticalSection(&g_lock);
        break;
    }
    case 2:
    {
        Info inf; BYTE found = 0;
        if (!u8(found) || !str(inf.name)) return;
        inf.valid = true; inf.found = found != 0;
        if (inf.found)
        {
            DWORD hp = 0, maxhp = 0;
            if (!str(inf.account) || !u8(inf.level) || !u8(inf.nation) || !u8(inf.race) || !u16(inf.cls) || !u8(inf.zone) || !u16(inf.x) || !u16(inf.z)
                || !u32(inf.np) || !u32(inf.monthly) || !u32(inf.cash) || !u32(inf.gold) || !str(inf.clan) || !u8(inf.auth) || !u8(inf.muted) || !u8(inf.noattack))
                return;
            for (int i = 0; i < 5; i++) if (!u8(inf.stat[i])) return;
            if (!u32(hp) || !u32(maxhp)) return;
            inf.hp = (int)hp; inf.maxhp = (int)maxhp;
            str(inf.ip);
            WORD pts = 0; DWORD mp = 0, maxmp = 0;
            if (u64(inf.exp) && u64(inf.maxexp) && u16(pts) && u16(inf.attack) && u16(inf.defence) && u32(inf.manner) && u32(mp) && u32(maxmp))
            {
                inf.points = (short)pts; inf.mp = (int)mp; inf.maxmp = (int)maxmp; inf.ext = true;
                for (int i = 0; i < 6; i++) { WORD v = 0; u16(v); inf.weapon[i] = (short)v; }
                for (int i = 0; i < 6; i++) u16(inf.resist[i]);
                for (int i = 0; i < 5; i++) u8(inf.skill[i]);
            }
        }
        EnterCriticalSection(&g_lock);
        bool self = !g_expect.empty() && g_expect.front();
        if (!g_expect.empty()) g_expect.pop_front();
        if (self) g_self = inf;
        else
        {
            g_info = inf;
            if (inf.found)
            {
                g_status = Wide(inf.name) + L"  ·  hesap " + Wide(inf.account) + Fmt(L"  ·  zone %u (%u, %u)", inf.zone, inf.x, inf.z)
                    + L"  ·  klan " + Wide(inf.clan) + L"  ·  " + Wide(inf.ip) + (inf.muted ? L"  ·  susturulmuş" : L"") + (inf.noattack ? L"  ·  saldırı kapalı" : L"");
                g_statusColor = C_INFO;
            }
            else { g_status = Wide(inf.name) + L" oyunda değil."; g_statusColor = C_ERR; }
        }
        LeaveCriticalSection(&g_lock);
        break;
    }
    case 6:
    {
        BYTE ok = 0; std::string text;
        if (!u8(ok) || !str(text)) return;
        SetStatus(Wide(text), ok ? C_OK : C_ERR);
        break;
    }
    default: return;
    }
    if (g_hWnd) PostMessageW(g_hWnd, WM_GM_REFRESH, 0, 0);
}

bool GmPanel_CursorOverPanel();

void GmPanel_FlushSendQueue()
{
    void* sock = *(void**)KO_PTR_PKT;
    if (!sock) return;
    std::deque<std::vector<BYTE>> pending;
    EnterCriticalSection(&g_lock);
    pending.swap(g_sendQ);
    LeaveCriticalSection(&g_lock);
    for (auto& p : pending)
    {
        typedef void(__thiscall* tSend)(void*, BYTE*, int);
        ((tSend)KO_SND_FNC)(sock, p.data(), (int)p.size());
    }
}

// ProxyWndProc: the wheel goes to the focused game window; forward it while the cursor is over the panel
bool GmPanel_ForwardWheel(WPARAM w, LPARAM l)
{
    if (!GmPanel_CursorOverPanel()) return false;
    PostMessageW(g_hWnd, WM_MOUSEWHEEL, w, l);
    return true;
}

// pus_store.cpp click gate: clicks on the panel must not reach the game (DirectInput)
bool GmPanel_CursorOverPanel()
{
    if (!g_hWnd || !IsWindowVisible(g_hWnd)) return false;
    POINT p;
    return GetCursorPos(&p) && WindowFromPoint(p) == g_hWnd;
}
