// Crash guard for the skill bar (CUIHotKeyDlg, vtable 0x102C0F4). Its row refresh 0xB46F10 (vtable slot 53, run for
// every bar page by 0xB4F940, e.g. after a WIZ_QUEST packet) shows / hides 10 rows; each row has 4 child windows kept
// in arrays at this+0x124 / +0x14C / +0x174 / +0x19C (10 pointers each). Rows whose child was never created can hold
// 0xFFFFFFFF; the game only skips null, so hiding such a row read address FFFFFFFF and crashed
// (KnightOnLine.exe+0x746FE9, 01.10.2026). The entry is detoured: invalid pointers become null before the game runs.
#include <windows.h>
#include <stdio.h>

void Log(const char* msg);

namespace
{
    const DWORD FN_ROWS = 0x00B46F10;
    const BYTE  kPrologue[10] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0x97, 0x87, 0xF4, 0x00 };   // push ebp / mov ebp,esp / push -1 / push F48797
    const DWORD kArrays[4] = { 0x124, 0x14C, 0x174, 0x19C };

    typedef void(__thiscall* FnRows)(void*);
    FnRows g_tramp = nullptr;
    volatile LONG g_logged = 0;

    void __fastcall HkRows(void* self, void*)
    {
        __try
        {
            int fixed = 0;
            for (DWORD a : kArrays)
            {
                DWORD* p = (DWORD*)((BYTE*)self + a);
                for (int i = 0; i < 10; i++)
                    if (p[i] == 0xFFFFFFFF || (p[i] && p[i] < 0x10000)) { p[i] = 0; fixed++; }
            }
            if (fixed && InterlockedIncrement(&g_logged) <= 5)
            {
                char b[96]; sprintf_s(b, "HOTKEY: %d invalid skill bar row window(s) cleared (crash guard)", fixed); Log(b);
            }
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
        g_tramp(self);
    }

    DWORD WINAPI InstallThread(LPVOID)
    {
        BYTE* at = (BYTE*)FN_ROWS;
        for (int i = 0; i < 240 && memcmp(at, kPrologue, sizeof(kPrologue)) != 0; i++) Sleep(250);   // exe unpacking
        if (memcmp(at, kPrologue, sizeof(kPrologue)) != 0) { Log("HOTKEY: row refresh differs, crash guard not installed"); return 0; }
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 32, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return 0;
        memcpy(tramp, kPrologue, sizeof(kPrologue));
        tramp[10] = 0xE9; *(DWORD*)(tramp + 11) = (FN_ROWS + 10) - (DWORD)(tramp + 15);
        g_tramp = (FnRows)tramp;
        DWORD old;
        VirtualProtect(at, 10, PAGE_EXECUTE_READWRITE, &old);
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)HkRows - (FN_ROWS + 5);
        memset(at + 5, 0x90, 5);
        VirtualProtect(at, 10, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 10);
        Log("HOTKEY: skill bar crash guard installed");
        return 0;
    }
}

void HotkeyGuard_Init()
{
    CloseHandle(CreateThread(nullptr, 0, InstallThread, nullptr, 0, nullptr));
}
