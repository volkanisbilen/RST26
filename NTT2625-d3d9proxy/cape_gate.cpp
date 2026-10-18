// Solo Cape: lets non-clan-leaders press Buy in the NATIVE Mantle Shop (CUIKnightsMantleShop).
// The Buy handler (2625 decompile sub_BBA540) has two client-only gates before it sends
// WIZ_CAPE [0x70][u8 ticket][i16 cape][u32 rgb]:
//     cmp [player+0x728], 2 ; jg  -> (paint needs clan grade <= 2)     else "cannot paint"
//     cmp [player+0x774], 1 ; je  -> send                              else "clan leader only"
// We turn the first jg into a jmp straight to the send block; the server (KnightCape.cpp /
// SoloCape.cpp) treats every non-leader purchase as a Solo Cape. Same runtime-signature
// technique as the recv / inventory hooks (no file patch, applied after the exe unpacks).
#include <windows.h>
#include <stdio.h>

void Log(const char* msg);

static const DWORD KO_RECV_FNC = 0x0084D0B0;   // unpack build; prologue check = "exe unpacked"

//  A1 <player>  83 B8 28 07 00 00 02  0F 8F <rel>  83 BE <ofs> 00  0F 84
static const BYTE kSig[]  = { 0xA1,0,0,0,0, 0x83,0xB8,0x28,0x07,0x00,0x00,0x02, 0x0F,0x8F,0,0,0,0, 0x83,0xBE,0,0,0,0,0x00, 0x0F,0x84 };
static const char kMask[] = "x????" "xxxxxxx" "xx????" "xx????x" "xx";
static_assert(sizeof(kMask) - 1 == sizeof(kSig), "signature/mask length");
static const BYTE kLeaderChk[] = { 0x83,0xB8,0x74,0x07,0x00,0x00,0x01, 0x0F,0x84 };   // cmp [eax+774h],1 ; je

static BYTE* Scan()
{
    BYTE* base = (BYTE*)GetModuleHandleA(nullptr);
    auto nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
    DWORD size = nt->OptionalHeader.SizeOfImage;
    const size_t n = sizeof(kSig);
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
                if (*p != 0xA1 || p[5] != 0x83) continue;
                size_t k = 1;
                for (; k < n; k++) if (kMask[k] == 'x' && p[k] != kSig[k]) break;
                if (k == n) { found = p; hits++; }
            }
        }
        if (regionEnd <= off) break;
        off = min(regionEnd, size);
    }
    char buf[96]; sprintf_s(buf, "CAPE: mantle shop gate scan: %d hit(s) first=%p", hits, found); Log(buf);
    return hits == 1 ? found : nullptr;
}

// ---------------------------------------------------------------- Solo-only cape list
// The shop's list fill (0xBBC030..) walks the Cloak table: design = id % 10000 % 100, skips 0 / 97..99 and adds every
// other design. At 0xBBC43E (test edi,edi ; je skip) we jump to HkListDesign, which also skips the designs that are
// not Solo Capes when the shop came from the Solo Cape Scroll or the player is not a clan leader (non-leaders can
// only buy Solo Capes anyway, SoloCape.cpp). Same design list as the server (SoloCape.cpp SOLO_CAPE_DESIGNS).
static const DWORD LIST_AT = 0x00BBC43E, LIST_CONT = 0x00BBC446, LIST_SKIP = 0x00BBC6F0;
static const BYTE kListOrig[8] = { 0x85, 0xFF, 0x0F, 0x84, 0xAA, 0x02, 0x00, 0x00 };
static const int kSoloDesigns[] = { 55, 56, 57, 58, 59, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94 };
static volatile LONG g_soloShop = 0;                 // shop opened by the Solo Cape Scroll
static DWORD g_listCont = LIST_CONT, g_listSkip = LIST_SKIP;

static bool __stdcall ShowDesign(DWORD design)
{
    bool leader = false;
    __try { BYTE* me = *(BYTE**)0x01115834; leader = me && *(int*)(me + 0x774) == 1; }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
    if (leader && !g_soloShop) return true;          // clan leader at the cape NPC: every cape
    for (int d : kSoloDesigns) if ((DWORD)d == design) return true;
    return false;
}

__declspec(naked) static void HkListDesign()
{
    __asm {
        test edi, edi
        jz skip
        pushad
        push edi
        call ShowDesign
        test al, al                                  // popad keeps the flags
        popad
        jz skip
        jmp dword ptr [g_listCont]
    skip:
        jmp dword ptr [g_listSkip]
    }
}

static void PatchList()
{
    BYTE* at = (BYTE*)LIST_AT;
    if (memcmp(at, kListOrig, sizeof(kListOrig)) != 0) { Log("CAPE: list fill differs, solo filter NOT installed"); return; }
    DWORD old;
    if (!VirtualProtect(at, 8, PAGE_EXECUTE_READWRITE, &old)) return;
    at[0] = 0xE9; *(INT32*)(at + 1) = (INT32)((BYTE*)HkListDesign - (at + 5));
    at[5] = at[6] = at[7] = 0x90;
    VirtualProtect(at, 8, old, &old);
    FlushInstructionCache(GetCurrentProcess(), at, 8);
    Log("CAPE: solo-only list filter installed");
}

static DWORD WINAPI PatchThread(LPVOID)
{
    static const BYTE kRecvPrologue[5] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF };
    // exe unpacked = recv prologue present OR already replaced by the recv hook's jmp (0xE9)
    const BYTE* recv = (const BYTE*)KO_RECV_FNC;
    for (int i = 0; i < 1200 && memcmp(recv, kRecvPrologue, 5) != 0 && recv[0] != 0xE9; i++) Sleep(100);
    Sleep(300);

    PatchList();
    BYTE* p = Scan();
    if (!p) { Log("CAPE: gate NOT patched"); return 0; }
    BYTE* jg = p + 12;                                            // 0F 8F rel32
    BYTE* grade = jg + 6 + *(INT32*)(jg + 2);                     // -> cmp [eax+774h],1 ; je send
    if (memcmp(grade, kLeaderChk, sizeof(kLeaderChk)) != 0) { Log("CAPE: leader check not where expected, NOT patched"); return 0; }
    BYTE* send = grade + 13 + *(INT32*)(grade + 9);               // je target = the WIZ_CAPE send block

    DWORD old;
    if (!VirtualProtect(jg, 6, PAGE_EXECUTE_READWRITE, &old)) { char e[64]; sprintf_s(e, "CAPE: VirtualProtect failed %lu", GetLastError()); Log(e); return 0; }
    jg[0] = 0xE9;
    *(INT32*)(jg + 1) = (INT32)(send - (jg + 5));
    jg[5] = 0x90;
    VirtualProtect(jg, 6, old, &old);
    FlushInstructionCache(GetCurrentProcess(), jg, 6);
    char buf[96]; sprintf_s(buf, "CAPE: mantle shop gate patched @%p -> %p", jg, send); Log(buf);
    return 0;
}

// Solo Cape Scroll: the server answers with KNIGHTS_CAPE_NPC, which shows the cape NPC menu
// (re_cloakshopnpc: "Cape" / "Register Knights Symbol" / "Walk away"). Right after a scroll click
// (rce_store.cpp calls Cape_ExpectShop) the menu's SetVisible(true) opens the Mantle Shop instead,
// exactly what its "Cape" button does (0xBC3930: hide menu, [[CGameProcMain]+0x2F4]->SetVisible(1)).
// Menu class vtable 0x102ECC8, slot 25 (+0x64) SetVisible = 0xBC38E0. Talking to the NPC is unchanged.
static volatile DWORD g_capeShopUntil = 0;
typedef void(__thiscall* tSetVis)(void*, int);
static tSetVis g_oMenuSetVisible = nullptr;

void Cape_ExpectShop() { g_capeShopUntil = GetTickCount() + 3000; }

static void __fastcall hkMenuSetVisible(void* self, void*, int visible)
{
    if (visible) InterlockedExchange(&g_soloShop, 0);  // the NPC menu: a normal visit (scroll path sets it below)
    if (visible && g_capeShopUntil && (int)(g_capeShopUntil - GetTickCount()) > 0)
    {
        g_capeShopUntil = 0;
        BYTE* gm = *(BYTE**)0x011158FC;
        void* shop = gm ? *(void**)(gm + 0x2F4) : nullptr;
        if (shop)
        {
            g_oMenuSetVisible(self, 0);
            InterlockedExchange(&g_soloShop, 1);         // Solo Cape Scroll: only Solo designs in the list
            void** vt = *(void***)shop;
            ((tSetVis)vt[0x64 / 4])(shop, 1);
            Log("CAPE: scroll -> mantle shop (npc menu skipped)");
            return;
        }
    }
    g_oMenuSetVisible(self, visible);
}

static void HookCapeMenu()
{
    DWORD* slot = (DWORD*)(0x0102ECC8 + 25 * 4);
    if (*slot != 0x00BC38E0) { Log("CAPE: npc menu SetVisible slot differs, menu not skipped"); return; }
    g_oMenuSetVisible = (tSetVis)*slot;
    DWORD old;
    VirtualProtect(slot, 4, PAGE_READWRITE, &old);
    *slot = (DWORD)hkMenuSetVisible;
    VirtualProtect(slot, 4, old, &old);
    Log("CAPE: npc menu hook installed");
}

static DWORD WINAPI MenuThread(LPVOID)
{
    static const BYTE kRecvPrologue[5] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF };
    const BYTE* recv = (const BYTE*)KO_RECV_FNC;
    for (int i = 0; i < 1200 && memcmp(recv, kRecvPrologue, 5) != 0 && recv[0] != 0xE9; i++) Sleep(100);
    HookCapeMenu();
    return 0;
}

void Cape_Init()
{
    CloseHandle(CreateThread(nullptr, 0, PatchThread, nullptr, 0, nullptr));
    CloseHandle(CreateThread(nullptr, 0, MenuThread, nullptr, 0, nullptr));
}
