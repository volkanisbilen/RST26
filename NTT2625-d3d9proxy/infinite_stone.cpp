// Infinite Stone of Warrior / Rogue / Mage / Priest (479059000..479062000) alone is enough for the class stone skills.
// The client refuses those skills by itself when the class stone (379059000..379062000) is not in the inventory:
// both skill checks (0x88BD2A.. and 0x897955..) ask the inventory "how many of item X" through 0xB78880
// (thiscall(inventory, item id) -> count, ret 4) and want >= 1. We wrap that function: a class stone that is not
// there counts as the infinite stone of the same class. The server (MagicInstance.cpp) accepts the infinite stone
// in place of the class stone and takes nothing away.
#include <windows.h>
#include <stdio.h>

void Log(const char* msg);

static const DWORD KO_COUNT_FNC = 0x00B78880;
static const BYTE  kCountPrologue[6] = { 0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8 };   // push ebp ; mov ebp,esp ; and esp,-8
typedef int(__thiscall* tCount)(void* inv, DWORD id);
static tCount g_oCount = nullptr;

static int __fastcall hkCount(void* inv, void* /*edx*/, DWORD id)
{
    int n = g_oCount(inv, id);
    if (n < 1 && id >= 379059000 && id <= 379062000 && id % 1000 == 0)
        n = g_oCount(inv, id + 100000000);
    return n;
}

static DWORD WINAPI PatchThread(LPVOID)
{
    BYTE* target = (BYTE*)KO_COUNT_FNC;
    for (int i = 0; i < 1200 && memcmp(target, kCountPrologue, sizeof(kCountPrologue)) != 0; i++) Sleep(100);   // exe unpacked
    if (memcmp(target, kCountPrologue, sizeof(kCountPrologue)) != 0) { Log("INFSTONE: item count prologue differs, NOT hooked"); return 0; }

    // trampoline: original 6 bytes + jmp back to target+6
    BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    if (!tramp) return 0;
    memcpy(tramp, target, 6);
    tramp[6] = 0xE9;
    *(DWORD*)(tramp + 7) = (DWORD)(target + 6) - (DWORD)(tramp + 11);
    g_oCount = (tCount)tramp;

    DWORD old;
    if (!VirtualProtect(target, 6, PAGE_EXECUTE_READWRITE, &old)) { Log("INFSTONE: VirtualProtect failed"); return 0; }
    target[0] = 0xE9;
    *(DWORD*)(target + 1) = (DWORD)hkCount - (DWORD)(target + 5);
    target[5] = 0x90;
    VirtualProtect(target, 6, old, &old);
    FlushInstructionCache(GetCurrentProcess(), target, 6);
    Log("INFSTONE: item count hook installed (infinite stone counts as the class stone)");
    return 0;
}

void InfStone_Init()
{
    CloseHandle(CreateThread(nullptr, 0, PatchThread, nullptr, 0, nullptr));
}
