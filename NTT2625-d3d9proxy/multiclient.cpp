// Multi-client: lets up to `MaxClients` copies of KnightOnLine.exe run at once on the same
// machine, instead of the client's own single-instance guard silently exiting every copy after
// the first.
//
// The client's own guard (KnightOnline.exe.c, WinMain):
//   hMutex = CreateMutexA(nullptr, 0, "Knight OnLine Client");
//   if (GetLastError() == 183) return 0;          // 183 = ERROR_ALREADY_EXISTS
// That call happens at the very top of WinMain, before Direct3DCreate9 -- but d3d9.dll is an
// *implicit* import of KnightOnline.exe, so the loader runs our DllMain(DLL_PROCESS_ATTACH)
// before WinMain's first line, same as every other early hook in this project (crash_trap.cpp,
// iat_fix.cpp, ...). That's early enough to intercept this specific call.
//
// Technique: hot-patch kernelbase.dll's CreateMutexA export (same 5-byte prologue trick as
// pack_override.cpp's ReadFile/CloseHandle hooks) and, only for the exact mutex name the game
// uses, replace the single global mutex with one of `MaxClients` numbered slots: try slot 1, 2,
// 3, ...; the first one that doesn't already exist is "claimed" (the handle is kept open for the
// process's lifetime so the slot frees itself the instant that process exits, crash included --
// ordinary kernel object handle-count cleanup, no explicit release needed). If every slot is
// already taken, behave exactly like the unmodified client: report ERROR_ALREADY_EXISTS so it
// exits the same way it always did for a genuine second instance.
//
// MaxClients is read from JstKO\multiclient.ini ([Settings] MaxClients=3, default 3) so it can be
// retuned without a rebuild.
#include <windows.h>
#include <stdio.h>

void Log(const char* msg);

namespace
{
    typedef HANDLE(WINAPI* FnCreateMutexA)(LPSECURITY_ATTRIBUTES, BOOL, LPCSTR);
    FnCreateMutexA g_orig = nullptr;
    int g_maxClients = 3;

    const char* const kGameMutexName = "Knight OnLine Client";

    int LoadMaxClients()
    {
        char p[MAX_PATH];
        GetModuleFileNameA(nullptr, p, MAX_PATH);
        char* s = strrchr(p, '\\');
        if (s) strcpy_s(s + 1, MAX_PATH - (s + 1 - p), "JstKO\\multiclient.ini");
        int n = GetPrivateProfileIntA("Settings", "MaxClients", 3, p);
        return n < 1 ? 1 : (n > 20 ? 20 : n);
    }

    HANDLE WINAPI HkCreateMutexA(LPSECURITY_ATTRIBUTES attrs, BOOL initialOwner, LPCSTR name)
    {
        if (!name || strcmp(name, kGameMutexName) != 0)
            return g_orig(attrs, initialOwner, name);

        for (int i = 1; i <= g_maxClients; i++)
        {
            char slot[64];
            sprintf_s(slot, "JstKO_MultiClient_Slot_%d", i);
            HANDLE h = g_orig(attrs, initialOwner, slot);
            if (h && GetLastError() != ERROR_ALREADY_EXISTS)
            {
                char b[96];
                sprintf_s(b, "MULTICLIENT: slot %d/%d claimed", i, g_maxClients);
                Log(b);
                return h;   // kept open for this process's whole lifetime on purpose
            }
            if (h) CloseHandle(h);
        }
        char b[96];
        sprintf_s(b, "MULTICLIENT: all %d slot(s) taken, refusing like the client normally would", g_maxClients);
        Log(b);
        SetLastError(ERROR_ALREADY_EXISTS);
        return nullptr;
    }

    // hot-patchable prologue 8B FF 55 8B EC -> jmp hook, trampoline = prologue + jmp back
    // (same technique as pack_override.cpp's HookHot -- duplicated locally, this file must not
    // depend on load order relative to that one)
    void* HookHot(const char* name, void* hook)
    {
        BYTE* fn = (BYTE*)GetProcAddress(GetModuleHandleW(L"kernelbase.dll"), name);
        static const BYTE kPro[5] = { 0x8B, 0xFF, 0x55, 0x8B, 0xEC };
        if (!fn || memcmp(fn, kPro, 5) != 0) return nullptr;
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return nullptr;
        memcpy(tramp, kPro, 5);
        tramp[5] = 0xE9; *(DWORD*)(tramp + 6) = (DWORD)(fn + 5) - (DWORD)(tramp + 10);
        DWORD old;
        VirtualProtect(fn, 5, PAGE_EXECUTE_READWRITE, &old);
        fn[0] = 0xE9; *(DWORD*)(fn + 1) = (DWORD)hook - (DWORD)(fn + 5);
        VirtualProtect(fn, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), fn, 5);
        return tramp;
    }
}

void MultiClient_Init()
{
    g_maxClients = LoadMaxClients();
    g_orig = (FnCreateMutexA)HookHot("CreateMutexA", (void*)HkCreateMutexA);
    char b[96];
    sprintf_s(b, "MULTICLIENT: max %d, hook %s", g_maxClients, g_orig ? "on" : "FAILED");
    Log(b);
}
