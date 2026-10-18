// NPC shop prices as the server charges them.
// The client works every price out from its own tables: Item_Org price ([basic+0x60]) x Item_Ext multiplier
// (int16 [ext+0x50]); selling = that / 6 (/ 4 premium) unless [basic+0x64] == 1 (full price). The server charges
// ITEM.BuyPrice and pays SellNpcPrice / BuyPrice / 6, which need not match (job NPC items: 100m in the DB, the tables
// say 498,537). The multiplication is inlined in ~30 places, so the prices are fixed in the DATA the code reads:
//   server, right before a shop opens (NpcEventSystem.cpp SendNpcPriceList): WIZ_HSACS_HOOK + NPCPRICES (0xF4)
//     [u16 n] n*[u32 item][u32 buy][u8 sell mode: 0 = buy / 6, 1 = buy, 2 = fixed][u32 fixed sell]
//     for the NPC's selling group and the player's inventory;
//   an item icon object keeps its rows at +0x60 (basic) / +0x64 (ext). Where the server's price differs, the icon
//   gets private copies of both rows (price = the server's, multiplier 1) for the length of one call only:
//     0xC7C700 tooltip text (thiscall, arg = icon; this+526 price shown, +528 == 0 "Selling Price",
//              +529 / +532 other price kinds, else "Purchasing Price"),
//     0xE2AB50 / 0xE31210 CUITransactionDlg icon drop / count entered: these add to the basket total (0xE25400);
//              dlg+276 = 12 pages x 24 shop icons, dlg+1428 = 28 inventory icons, dlg+1572 = 14 basket icons.
//   The calls are wrapped by swapping their return address, so the rows are put back whatever the calling
//   convention; basket icons copy the row pointers, they are put back too. Nothing stays changed between calls.
#include <windows.h>
#include <stdio.h>
#include <unordered_map>

void Log(const char* msg);

namespace
{
    const DWORD FN_TIP = 0x00C7C700, FN_DROP = 0x00E2AB50, FN_COUNT = 0x00E31210;
    const BYTE  WIZ_HSACS_HOOK = 0xE9, SUB_NPCPRICES = 0xF4;
    const int IT_BASIC = 0x60, IT_EXT = 0x64;
    const int B_PRICE = 0x60, B_SALE = 0x64, B_SIZE = 0x90, E_MULT = 0x50, E_SIZE = 0xA4;
    const int DLG_SHOP = 276, SHOP_N = 288, DLG_INV = 1428, INV_N = 28, DLG_CART = 1572, CART_N = 14;
    const int TIP_SHOW = 526, TIP_BUYSIDE = 528, TIP_ALT = 529, TIP_SELLFULL = 532;
    enum { K_TIP, K_DROP, K_COUNT };
    enum { M_BUY, M_SELL };

    struct Price { UINT32 buy; BYTE sellMode; UINT32 sell; };
    struct Clone { BYTE basic[B_SIZE]; BYTE ext[E_SIZE]; BYTE* ob; BYTE* oe; };
    struct Frame { DWORD ret; DWORD* slot; int kind; BYTE* obj; };

    CRITICAL_SECTION g_lock;
    std::unordered_map<UINT32, Price>*  g_price = nullptr;    // g_lock
    std::unordered_map<UINT64, Clone*>* g_clones = nullptr;   // item | mode << 32; game thread
    std::unordered_map<BYTE*, Clone*>*  g_byRow = nullptr;    // copied basic / ext row -> its clone; game thread
    Frame g_frames[16]; int g_depth = 0;
    DWORD g_tid = 0;
    void* g_trampTip = nullptr, *g_trampDrop = nullptr, *g_trampCount = nullptr;
    int g_logged = 0;

    // an icon left with copied rows (a basket icon made during a wrapped call): back to the table rows
    void Normalize(BYTE* icon)
    {
        auto it = g_byRow->find(*(BYTE**)(icon + IT_BASIC));
        if (it == g_byRow->end()) it = g_byRow->find(*(BYTE**)(icon + IT_EXT));
        if (it == g_byRow->end()) return;
        *(BYTE**)(icon + IT_BASIC) = it->second->ob;
        *(BYTE**)(icon + IT_EXT) = it->second->oe;
    }

    bool Apply(BYTE* icon, int mode)
    {
        BYTE* b = *(BYTE**)(icon + IT_BASIC), *e = *(BYTE**)(icon + IT_EXT);
        if (!b || !e) return false;
        UINT32 item = *(UINT32*)b + *(UINT32*)e % 1000;
        Price p;
        EnterCriticalSection(&g_lock);
        auto it = g_price->find(item);
        bool have = it != g_price->end();
        if (have) p = it->second;
        LeaveCriticalSection(&g_lock);
        if (!have) return false;

        INT64 client = (INT64)*(INT32*)(b + B_PRICE) * *(INT16*)(e + E_MULT);
        INT32 sale = *(INT32*)(b + B_SALE), price, flag;
        if (mode == M_BUY)            { if (client == p.buy) return false;               price = (INT32)p.buy;  flag = sale; }
        else if (p.sellMode == 2)     { if (sale == 1 && client == p.sell) return false; price = (INT32)p.sell; flag = 1; }
        else if (p.sellMode == 1)     { if (sale == 1 && client == p.buy) return false;  price = (INT32)p.buy;  flag = 1; }
        else                          { if (sale != 1 && client == p.buy) return false;  price = (INT32)p.buy;  flag = sale == 1 ? 0 : sale; }

        Clone*& c = (*g_clones)[item | ((UINT64)mode << 32)];
        if (!c)
        {
            c = new Clone;
            (*g_byRow)[c->basic] = c; (*g_byRow)[c->ext] = c;
            if (g_logged < 40)
            {
                g_logged++;
                char m[160]; sprintf_s(m, "PRICE: item %u %s: client %lld -> server %d%s", item, mode == M_BUY ? "buy" : "sell",
                    client, price, mode == M_BUY || flag == 1 ? "" : " / 6");
                Log(m);
            }
        }
        memcpy(c->basic, b, B_SIZE); memcpy(c->ext, e, E_SIZE);
        c->ob = b; c->oe = e;
        *(INT32*)(c->basic + B_PRICE) = price; *(INT32*)(c->basic + B_SALE) = flag; *(INT16*)(c->ext + E_MULT) = 1;
        *(BYTE**)(icon + IT_BASIC) = c->basic;
        *(BYTE**)(icon + IT_EXT) = c->ext;
        return true;
    }

    void RestoreDlg(BYTE* dlg)
    {
        static const int kArr[3][2] = { { DLG_SHOP, SHOP_N }, { DLG_INV, INV_N }, { DLG_CART, CART_N } };
        for (auto& a : kArr)
            for (int i = 0; i < a[1]; i++)
                if (BYTE* icon = *(BYTE**)(dlg + a[0] + 4 * i)) Normalize(icon);
    }

    void AfterStub();

    void PreImpl(int kind, BYTE* self, DWORD* slot)
    {
        // frames left behind by an exception: deeper than this call, or their slot no longer holds our address
        int keep = 0;
        for (int i = 0; i < g_depth; i++)
            if (g_frames[i].slot > slot && *g_frames[i].slot == (DWORD)AfterStub) g_frames[keep++] = g_frames[i];
        g_depth = keep;
        if (g_depth >= 16) g_depth = 0;
        if (!self) return;
        Frame f = { *slot, slot, kind, nullptr };
        bool any = false;
        if (kind == K_TIP)
        {
            BYTE* icon = (BYTE*)slot[1];
            if (!icon || !self[TIP_SHOW]) return;
            Normalize(icon);
            int mode = !self[TIP_BUYSIDE] ? M_SELL : (!self[TIP_ALT] && !self[TIP_SELLFULL]) ? M_BUY : -1;
            if (mode < 0) return;
            f.obj = icon;
            any = Apply(icon, mode);
        }
        else
        {
            f.obj = self;
            RestoreDlg(self);
            for (int i = 0; i < SHOP_N; i++)
                if (BYTE* icon = *(BYTE**)(self + DLG_SHOP + 4 * i)) any |= Apply(icon, M_BUY);
            for (int i = 0; i < INV_N; i++)
                if (BYTE* icon = *(BYTE**)(self + DLG_INV + 4 * i)) any |= Apply(icon, M_SELL);
        }
        if (!any) return;
        g_frames[g_depth++] = f;
        *slot = (DWORD)AfterStub;
    }

    void PostImpl(const Frame& f)
    {
        if (f.kind == K_TIP) Normalize(f.obj); else RestoreDlg(f.obj);
    }

    // function entry: slot = where its return address is
    void __stdcall PreHook(int kind, BYTE* self, DWORD* slot)
    {
        if (!g_tid) g_tid = GetCurrentThreadId();
        if (GetCurrentThreadId() != g_tid) return;
        __try { PreImpl(kind, self, slot); }
        __except (EXCEPTION_EXECUTE_HANDLER) { Log("PRICE: exception before the call"); }
    }

    // the wrapped function returned: put the rows back, hand out its real return address
    DWORD __stdcall PostHook(DWORD espAfter)
    {
        // the newest frame whose slot sits right under espAfter (return address + at most 16 bytes of arguments);
        // anything newer was left behind by an exception inside this call
        Frame f = {};
        while (g_depth > 0)
        {
            Frame t = g_frames[--g_depth];
            if ((DWORD)t.slot < espAfter && (DWORD)t.slot + 20 >= espAfter) { f = t; break; }
        }
        __try { if (f.obj) PostImpl(f); }
        __except (EXCEPTION_EXECUTE_HANDLER) { Log("PRICE: exception after the call"); }
        return f.ret;
    }

    __declspec(naked) void AfterStub()
    {
        __asm {
            push eax                            // room for the return address
            pushad
            pushfd
            lea  eax, [esp + 40]                // esp as the function left it
            push eax
            call PostHook
            mov  [esp + 36], eax
            popfd
            popad
            ret
        }
    }
    __declspec(naked) void TipStub()
    {
        __asm {
            pushad
            pushfd
            lea  eax, [esp + 36]
            push eax
            push ecx
            push 0
            call PreHook
            popfd
            popad
            jmp  dword ptr [g_trampTip]
        }
    }
    __declspec(naked) void DropStub()
    {
        __asm {
            pushad
            pushfd
            lea  eax, [esp + 36]
            push eax
            push ecx
            push 1
            call PreHook
            popfd
            popad
            jmp  dword ptr [g_trampDrop]
        }
    }
    __declspec(naked) void CountStub()
    {
        __asm {
            pushad
            pushfd
            lea  eax, [esp + 36]
            push eax
            push ecx
            push 2
            call PreHook
            popfd
            popad
            jmp  dword ptr [g_trampCount]
        }
    }

    bool Hook(DWORD fn, const BYTE* pro, int len, void* stub, void** tramp)
    {
        BYTE* at = (BYTE*)fn;
        if (memcmp(at, pro, len) != 0) return false;
        BYTE* t = (BYTE*)VirtualAlloc(nullptr, 32, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!t) return false;
        memcpy(t, pro, len); t[len] = 0xE9; *(DWORD*)(t + len + 1) = (fn + len) - (DWORD)(t + len + 5);
        *tramp = t;
        DWORD old;
        if (!VirtualProtect(at, len, PAGE_EXECUTE_READWRITE, &old)) return false;
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)stub - (fn + 5);
        for (int i = 5; i < len; i++) at[i] = 0x90;
        VirtualProtect(at, len, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, len);
        return true;
    }

    DWORD WINAPI HookThread(LPVOID)
    {
        static const BYTE kA[6] = { 0x53, 0x8B, 0xDC, 0x83, 0xEC, 0x08 };   // push ebx / mov ebx,esp / sub esp,8
        static const BYTE kB[5] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF };         // push ebp / mov ebp,esp / push -1
        for (int i = 0; i < 1200 && memcmp((void*)FN_TIP, kA, 6) != 0; i++) Sleep(100);   // exe unpacking
        // all three or none: a tooltip price without the matching basket total would be worse than neither
        if (memcmp((void*)FN_TIP, kA, 6) != 0 || memcmp((void*)FN_DROP, kB, 5) != 0 || memcmp((void*)FN_COUNT, kA, 6) != 0)
        { Log("PRICE: shop functions differ, hooks NOT installed"); return 0; }
        bool ok = Hook(FN_TIP, kA, 6, TipStub, &g_trampTip) && Hook(FN_DROP, kB, 5, DropStub, &g_trampDrop)
               && Hook(FN_COUNT, kA, 6, CountStub, &g_trampCount);
        Log(ok ? "PRICE: shop price hooks installed" : "PRICE: hook install failed");
        return 0;
    }
}

void PriceFix_Init()
{
    wchar_t exe[MAX_PATH]; GetModuleFileNameW(nullptr, exe, MAX_PATH);
    const wchar_t* name = wcsrchr(exe, L'\\'); name = name ? name + 1 : exe;
    if (_wcsicmp(name, L"KnightOnLine.exe") != 0) return;
    InitializeCriticalSection(&g_lock);
    g_price = new std::unordered_map<UINT32, Price>;
    g_clones = new std::unordered_map<UINT64, Clone*>;
    g_byRow = new std::unordered_map<BYTE*, Clone*>;
    CloseHandle(CreateThread(nullptr, 0, HookThread, nullptr, 0, nullptr));
}

void PriceFix_OnRecv(const BYTE* buf, size_t len)
{
    if (!g_price || len < 4 || buf[0] != WIZ_HSACS_HOOK || buf[1] != SUB_NPCPRICES) return;
    size_t n = *(const UINT16*)(buf + 2);
    if (len < 4 + n * 13) return;
    EnterCriticalSection(&g_lock);
    const BYTE* p = buf + 4;
    for (size_t i = 0; i < n; i++, p += 13)
        (*g_price)[*(const UINT32*)p] = { *(const UINT32*)(p + 4), p[8], *(const UINT32*)(p + 9) };
    LeaveCriticalSection(&g_lock);
    char m[64]; sprintf_s(m, "PRICE: %u shop prices from the server", (unsigned)n);
    Log(m);
}
