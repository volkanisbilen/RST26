// Crash trap: finds WHERE the client raises errors (client-side counterpart of the server BugTrap).
//  - MessageBoxA/W hook: logs text + caller stack for every dialog the game opens (dialog still shows).
//  - Vectored handler:   logs serious first-chance exceptions (AV, div0, ...) with module+offset + stack,
//                        deduplicated per address. Never swallows anything (CONTINUE_SEARCH).
//  - Unhandled filter:   on a real crash writes KO_crash_*.dmp + KO_crash_*.txt, then shows the location.
// Output: crash_trap.log next to KnightOnLine.exe. VA of KnightOnLine.exe matches the unpack build /
// ko_memdump addresses (image base 0x400000), so they can be looked up directly in the disassembly.
#include <windows.h>
#include <dbghelp.h>
#include <stdio.h>
#include <share.h>

void Log(const char* msg);

namespace
{
    CRITICAL_SECTION g_cs;
    char             g_dir[MAX_PATH];
    __declspec(thread) int t_inTrap = 0;       // recursion guard (our own MessageBox / logging)
    volatile LONG    g_crashing = 0;

    // ---- dbghelp (loaded on demand) ------------------------------------------------------
    typedef BOOL(WINAPI* tSymInitialize)(HANDLE, PCSTR, BOOL);
    typedef DWORD(WINAPI* tSymSetOptions)(DWORD);
    typedef BOOL(WINAPI* tSymFromAddr)(HANDLE, DWORD64, PDWORD64, PSYMBOL_INFO);
    typedef BOOL(WINAPI* tStackWalk64)(DWORD, HANDLE, HANDLE, LPSTACKFRAME64, PVOID, PREAD_PROCESS_MEMORY_ROUTINE64,
                                       PFUNCTION_TABLE_ACCESS_ROUTINE64, PGET_MODULE_BASE_ROUTINE64, PTRANSLATE_ADDRESS_ROUTINE64);
    typedef BOOL(WINAPI* tMiniDumpWriteDump)(HANDLE, DWORD, HANDLE, MINIDUMP_TYPE, PMINIDUMP_EXCEPTION_INFORMATION,
                                             PMINIDUMP_USER_STREAM_INFORMATION, PMINIDUMP_CALLBACK_INFORMATION);
    HMODULE            g_dbghelp = nullptr;
    tSymFromAddr       pSymFromAddr = nullptr;
    tStackWalk64       pStackWalk64 = nullptr;
    tMiniDumpWriteDump pMiniDumpWriteDump = nullptr;
    PFUNCTION_TABLE_ACCESS_ROUTINE64 pFuncTableAccess = nullptr;
    PGET_MODULE_BASE_ROUTINE64       pGetModuleBase = nullptr;

    void InitDbgHelp()
    {
        if (g_dbghelp) return;
        char sys[MAX_PATH];
        GetSystemDirectoryA(sys, MAX_PATH);
        strcat_s(sys, "\\dbghelp.dll");
        g_dbghelp = LoadLibraryA(sys);
        if (!g_dbghelp) return;
        auto init = (tSymInitialize)GetProcAddress(g_dbghelp, "SymInitialize");
        auto opts = (tSymSetOptions)GetProcAddress(g_dbghelp, "SymSetOptions");
        pSymFromAddr       = (tSymFromAddr)GetProcAddress(g_dbghelp, "SymFromAddr");
        pStackWalk64       = (tStackWalk64)GetProcAddress(g_dbghelp, "StackWalk64");
        pMiniDumpWriteDump = (tMiniDumpWriteDump)GetProcAddress(g_dbghelp, "MiniDumpWriteDump");
        pFuncTableAccess   = (PFUNCTION_TABLE_ACCESS_ROUTINE64)GetProcAddress(g_dbghelp, "SymFunctionTableAccess64");
        pGetModuleBase     = (PGET_MODULE_BASE_ROUTINE64)GetProcAddress(g_dbghelp, "SymGetModuleBase64");
        if (opts) opts(SYMOPT_UNDNAME | SYMOPT_DEFERRED_LOADS | SYMOPT_FAIL_CRITICAL_ERRORS | SYMOPT_NO_PROMPTS);
        if (init) init(GetCurrentProcess(), g_dir, TRUE);
    }

    // "KnightOnLine.exe+0x3C8080 [007C8080]" or "user32.dll!MessageBoxA+0x12 [75A1xxxx]"
    void FormatAddr(DWORD addr, char* out, size_t cb)
    {
        HMODULE mod = nullptr;
        char name[MAX_PATH] = "?";
        if (GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                               (LPCSTR)addr, &mod) && mod)
        {
            char full[MAX_PATH];
            GetModuleFileNameA(mod, full, MAX_PATH);
            const char* s = strrchr(full, '\\');
            strcpy_s(name, s ? s + 1 : full);
        }
        char sym[160] = "";
        if (pSymFromAddr)
        {
            BYTE buf[sizeof(SYMBOL_INFO) + 128] = {};
            auto si = (PSYMBOL_INFO)buf;
            si->SizeOfStruct = sizeof(SYMBOL_INFO);
            si->MaxNameLen = 127;
            DWORD64 disp = 0;
            if (pSymFromAddr(GetCurrentProcess(), addr, &disp, si))
                sprintf_s(sym, "!%s+0x%llX", si->Name, disp);
        }
        if (sym[0])
            sprintf_s(out, cb, "%s%s [%08lX]", name, sym, addr);
        else if (mod)
            sprintf_s(out, cb, "%s+0x%lX [%08lX]", name, addr - (DWORD)mod, addr);
        else
            sprintf_s(out, cb, "?? [%08lX]", addr);
    }

    FILE* OpenLog(const char* file)
    {
        char path[MAX_PATH];
        sprintf_s(path, "%s\\%s", g_dir, file);
        FILE* f = nullptr;
        for (int i = 0; i < 20 && !(f = _fsopen(path, "a", _SH_DENYNO)); i++) Sleep(5);
        return f;
    }

    void Stamp(FILE* f)
    {
        SYSTEMTIME t; GetLocalTime(&t);
        fprintf(f, "[%02d:%02d:%02d.%03d] tid=%lu ", t.wHour, t.wMinute, t.wSecond, t.wMilliseconds, GetCurrentThreadId());
    }

    void WriteFrames(FILE* f, const DWORD* frames, int n)
    {
        char line[400];
        for (int i = 0; i < n; i++)
        {
            FormatAddr(frames[i], line, sizeof(line));
            fprintf(f, "    #%02d %s\n", i, line);
        }
    }

    // Walks the stack of an exception context (works where there is no frame pointer, too).
    int WalkContext(const CONTEXT* ctx, DWORD* frames, int max)
    {
        int n = 0;
        if (pStackWalk64 && pFuncTableAccess && pGetModuleBase)
        {
            CONTEXT c = *ctx;
            STACKFRAME64 sf = {};
            sf.AddrPC.Offset = c.Eip;    sf.AddrPC.Mode = AddrModeFlat;
            sf.AddrFrame.Offset = c.Ebp; sf.AddrFrame.Mode = AddrModeFlat;
            sf.AddrStack.Offset = c.Esp; sf.AddrStack.Mode = AddrModeFlat;
            while (n < max && pStackWalk64(IMAGE_FILE_MACHINE_I386, GetCurrentProcess(), GetCurrentThread(), &sf, &c,
                                           nullptr, pFuncTableAccess, pGetModuleBase, nullptr))
            {
                if (!sf.AddrPC.Offset) break;
                frames[n++] = (DWORD)sf.AddrPC.Offset;
            }
        }
        if (n == 0) frames[n++] = ctx->Eip;
        return n;
    }

    // ---- MessageBox hooks -------------------------------------------------------------------
    typedef int(WINAPI* tMessageBoxA)(HWND, LPCSTR, LPCSTR, UINT);
    typedef int(WINAPI* tMessageBoxW)(HWND, LPCWSTR, LPCWSTR, UINT);
    tMessageBoxA oMessageBoxA = nullptr;
    tMessageBoxW oMessageBoxW = nullptr;

    void LogDialog(const char* caption, const char* text)
    {
        DWORD frames[32];
        int n = (int)RtlCaptureStackBackTrace(2, 32, (PVOID*)frames, nullptr);   // skip LogDialog + hook
        EnterCriticalSection(&g_cs);
        InitDbgHelp();
        if (FILE* f = OpenLog("crash_trap.log"))
        {
            Stamp(f);
            fprintf(f, "DIALOG caption=\"%s\"\n    text=\"%s\"\n    called from:\n", caption ? caption : "", text ? text : "");
            WriteFrames(f, frames, n);
            fclose(f);
        }
        LeaveCriticalSection(&g_cs);
    }

    int WINAPI hkMessageBoxA(HWND h, LPCSTR text, LPCSTR cap, UINT type)
    {
        if (!t_inTrap)
        {
            t_inTrap++;
            __try { LogDialog(cap, text); } __except (EXCEPTION_EXECUTE_HANDLER) {}
            t_inTrap--;
        }
        return oMessageBoxA(h, text, cap, type);
    }

    int WINAPI hkMessageBoxW(HWND h, LPCWSTR text, LPCWSTR cap, UINT type)
    {
        if (!t_inTrap)
        {
            t_inTrap++;
            __try
            {
                char a[1024] = "", c[256] = "";
                if (text) WideCharToMultiByte(CP_ACP, 0, text, -1, a, sizeof(a) - 1, nullptr, nullptr);
                if (cap)  WideCharToMultiByte(CP_ACP, 0, cap, -1, c, sizeof(c) - 1, nullptr, nullptr);
                LogDialog(c, a);
            }
            __except (EXCEPTION_EXECUTE_HANDLER) {}
            t_inTrap--;
        }
        return oMessageBoxW(h, text, cap, type);
    }

    // user32 exports start with the hot-patch prologue "mov edi,edi; push ebp; mov ebp,esp".
    void* HookExport(const char* name, void* detour)
    {
        static const BYTE kPrologue[5] = { 0x8B, 0xFF, 0x55, 0x8B, 0xEC };
        BYTE* target = (BYTE*)GetProcAddress(GetModuleHandleA("user32.dll"), name);
        if (!target || memcmp(target, kPrologue, 5) != 0)
        {
            char b[96]; sprintf_s(b, "CRASHTRAP: %s prologue mismatch, not hooked", name); Log(b);
            return nullptr;
        }
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return nullptr;
        memcpy(tramp, target, 5);
        tramp[5] = 0xE9;
        *(DWORD*)(tramp + 6) = (DWORD)(target + 5) - (DWORD)(tramp + 10);
        DWORD old;
        if (!VirtualProtect(target, 5, PAGE_EXECUTE_READWRITE, &old)) return nullptr;
        target[0] = 0xE9;
        *(DWORD*)(target + 1) = (DWORD)detour - (DWORD)(target + 5);
        VirtualProtect(target, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), target, 5);
        return tramp;
    }

    // ---- exceptions -------------------------------------------------------------------------
    const char* CodeName(DWORD code)
    {
        switch (code)
        {
        case EXCEPTION_ACCESS_VIOLATION:      return "ACCESS_VIOLATION";
        case EXCEPTION_ILLEGAL_INSTRUCTION:   return "ILLEGAL_INSTRUCTION";
        case EXCEPTION_INT_DIVIDE_BY_ZERO:    return "INT_DIVIDE_BY_ZERO";
        case EXCEPTION_PRIV_INSTRUCTION:      return "PRIV_INSTRUCTION";
        case EXCEPTION_STACK_OVERFLOW:        return "STACK_OVERFLOW";
        case EXCEPTION_ARRAY_BOUNDS_EXCEEDED: return "ARRAY_BOUNDS_EXCEEDED";
        case EXCEPTION_IN_PAGE_ERROR:         return "IN_PAGE_ERROR";
        case 0xC0000409:                      return "STACK_BUFFER_OVERRUN/FAST_FAIL";
        case 0xC0000374:                      return "HEAP_CORRUPTION";
        case 0xE06D7363:                      return "C++ EXCEPTION (throw)";
        default:                              return nullptr;   // not interesting (debug prints, thread names, ...)
        }
    }

    // dedupe: log each faulting address at most kPerAddr times, kMaxTotal entries overall
    const int kSlots = 512, kPerAddr = 2, kMaxTotal = 400;
    DWORD g_seenAddr[kSlots];
    LONG  g_seenCnt[kSlots];
    volatile LONG g_total = 0;

    bool ShouldLog(DWORD addr)
    {
        if (g_total >= kMaxTotal) return false;
        for (int i = 0; i < kSlots; i++)
        {
            if (g_seenAddr[i] == addr) return InterlockedIncrement(&g_seenCnt[i]) <= kPerAddr;
            if (g_seenAddr[i] == 0 && InterlockedCompareExchange((LONG*)&g_seenAddr[i], (LONG)addr, 0) == 0)
            {
                g_seenCnt[i] = 1;
                return true;
            }
        }
        return false;
    }

    void LogException(const char* kind, EXCEPTION_POINTERS* ep, FILE* f)
    {
        auto er = ep->ExceptionRecord;
        auto cx = ep->ContextRecord;
        char where[400];
        FormatAddr((DWORD)er->ExceptionAddress, where, sizeof(where));
        Stamp(f);
        fprintf(f, "%s %s (0x%08lX) at %s\n", kind, CodeName(er->ExceptionCode) ? CodeName(er->ExceptionCode) : "EXCEPTION",
                er->ExceptionCode, where);
        if (er->ExceptionCode == EXCEPTION_ACCESS_VIOLATION && er->NumberParameters >= 2)
            fprintf(f, "    %s address %08lX\n", er->ExceptionInformation[0] == 0 ? "read of" :
                    er->ExceptionInformation[0] == 1 ? "write to" : "execute at", (DWORD)er->ExceptionInformation[1]);
        fprintf(f, "    EAX=%08lX EBX=%08lX ECX=%08lX EDX=%08lX ESI=%08lX EDI=%08lX EBP=%08lX ESP=%08lX\n",
                cx->Eax, cx->Ebx, cx->Ecx, cx->Edx, cx->Esi, cx->Edi, cx->Ebp, cx->Esp);
        if (er->ExceptionCode != EXCEPTION_STACK_OVERFLOW)   // no stack left to walk with
        {
            DWORD frames[40];
            int n = WalkContext(cx, frames, 40);
            fprintf(f, "    stack:\n");
            WriteFrames(f, frames, n);
        }
    }

    // ---- fast fail ------------------------------------------------------------------------------
    // The CRT's __fastfail sites (int 0x29: __report_gsfailure, __report_rangecheckfailure, abort, ...)
    // end the process at once - no exception, no dialog, nothing in our logs ("game just closed").
    // Their "int 0x29" is replaced by "int3; int3" so the failure reaches the vectored handler, which
    // logs caller + stack, writes a dump and shows the crash dialog, then ends the process as before.
    const DWORD kFastFailSites[] = { 0x004FBD09, 0x004FD555, 0x005E99EB, 0x005E9AF3, 0x005EA87A,
                                     0x005EE4D8, 0x005EE505, 0x005EEDD0, 0x00602CE3 };
    bool g_fastFailPatched = false;

    bool IsFastFailSite(DWORD eip)
    {
        for (DWORD s : kFastFailSites)
            if (eip == s || eip == s + 1) return true;   // int3 reports the byte after it on x86
        return false;
    }

    void PatchFastFail()
    {
        int n = 0;
        for (DWORD s : kFastFailSites)
        {
            BYTE* p = (BYTE*)s;
            if (p[0] != 0xCD || p[1] != 0x29) continue;
            DWORD old;
            if (!VirtualProtect(p, 2, PAGE_EXECUTE_READWRITE, &old)) continue;
            p[0] = 0xCC; p[1] = 0xCC;
            VirtualProtect(p, 2, old, &old);
            FlushInstructionCache(GetCurrentProcess(), p, 2);
            n++;
        }
        g_fastFailPatched = n > 0;
        char b[96]; sprintf_s(b, "CRASHTRAP: fast-fail sites redirected %d/%d", n, (int)(sizeof(kFastFailSites) / sizeof(kFastFailSites[0])));
        Log(b);
    }

    LONG WINAPI UnhandledFilter(EXCEPTION_POINTERS* ep);

    const char* FastFailName(DWORD c)
    {
        switch (c)
        {
        case 2:  return "STACK_COOKIE_CHECK_FAILURE (stack buffer overrun)";
        case 5:  return "INVALID_ARG";
        case 7:  return "FATAL_APP_EXIT (abort)";
        case 8:  return "RANGE_CHECK_FAILURE (array index out of range)";
        case 13: return "GUARD_ICALL / invalid stack";
        default: return "?";
        }
    }

    LONG CALLBACK VectoredHandler(EXCEPTION_POINTERS* ep)
    {
        DWORD code = ep->ExceptionRecord->ExceptionCode;
        if (code == EXCEPTION_BREAKPOINT && g_fastFailPatched && IsFastFailSite(ep->ContextRecord->Eip))
        {
            const DWORD failCode = ep->ContextRecord->Ecx;
            ep->ContextRecord->Eip = (DWORD)ep->ExceptionRecord->ExceptionAddress;
            ep->ExceptionRecord->ExceptionCode = 0xC0000409;
            if (FILE* f = OpenLog("crash_trap.log"))
            {
                Stamp(f);
                fprintf(f, "FAST FAIL code=%lu %s  (caller of the CRT report fn: [ESP]=%08lX [EBP+4]=%08lX)\n", failCode, FastFailName(failCode),
                        !IsBadReadPtr((void*)ep->ContextRecord->Esp, 4) ? *(DWORD*)ep->ContextRecord->Esp : 0,
                        !IsBadReadPtr((void*)(ep->ContextRecord->Ebp + 4), 4) ? *(DWORD*)(ep->ContextRecord->Ebp + 4) : 0);
                fclose(f);
            }
            UnhandledFilter(ep);
            TerminateProcess(GetCurrentProcess(), 0xC0000409);
        }
        if (!CodeName(code) || t_inTrap || g_crashing) return EXCEPTION_CONTINUE_SEARCH;
        // C++ throws all raise from KernelBase!RaiseException -> dedupe those by thrown type (ThrowInfo*)
        DWORD key = (DWORD)ep->ExceptionRecord->ExceptionAddress;
        if (code == 0xE06D7363 && ep->ExceptionRecord->NumberParameters >= 3)
            key = (DWORD)ep->ExceptionRecord->ExceptionInformation[2] | 1;
        if (!ShouldLog(key)) return EXCEPTION_CONTINUE_SEARCH;
        InterlockedIncrement(&g_total);
        t_inTrap++;
        __try
        {
            EnterCriticalSection(&g_cs);
            InitDbgHelp();
            if (FILE* f = OpenLog("crash_trap.log"))
            {
                LogException("FIRST-CHANCE", ep, f);   // may still be handled by the game; see UNHANDLED for fatal ones
                fclose(f);
            }
            LeaveCriticalSection(&g_cs);
        }
        __except (EXCEPTION_EXECUTE_HANDLER) { LeaveCriticalSection(&g_cs); }
        t_inTrap--;
        return EXCEPTION_CONTINUE_SEARCH;
    }

    LONG WINAPI UnhandledFilter(EXCEPTION_POINTERS* ep)
    {
        if (InterlockedExchange(&g_crashing, 1) != 0) return EXCEPTION_EXECUTE_HANDLER;
        t_inTrap++;
        InitDbgHelp();

        SYSTEMTIME t; GetLocalTime(&t);
        char stamp[64];
        sprintf_s(stamp, "%04u%02u%02u_%02u%02u%02u", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond);
        char dmpName[96], txtName[96], dmpPath[MAX_PATH];
        sprintf_s(dmpName, "KO_crash_%s.dmp", stamp);
        sprintf_s(txtName, "KO_crash_%s.txt", stamp);
        sprintf_s(dmpPath, "%s\\%s", g_dir, dmpName);

        if (FILE* f = OpenLog(txtName)) { LogException("UNHANDLED", ep, f); fclose(f); }
        if (FILE* f = OpenLog("crash_trap.log")) { LogException("UNHANDLED (CRASH)", ep, f); fclose(f); }

        if (pMiniDumpWriteDump)
        {
            HANDLE h = CreateFileA(dmpPath, GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
            if (h != INVALID_HANDLE_VALUE)
            {
                MINIDUMP_EXCEPTION_INFORMATION mei = { GetCurrentThreadId(), ep, FALSE };
                pMiniDumpWriteDump(GetCurrentProcess(), GetCurrentProcessId(), h,
                                   (MINIDUMP_TYPE)(MiniDumpWithDataSegs | MiniDumpWithHandleData | MiniDumpWithThreadInfo |
                                                   MiniDumpWithIndirectlyReferencedMemory),
                                   &mei, nullptr, nullptr);
                CloseHandle(h);
            }
        }

        char where[400], msg[900];
        FormatAddr((DWORD)ep->ExceptionRecord->ExceptionAddress, where, sizeof(where));
        sprintf_s(msg, "Oyun coktu.\n\nKod: 0x%08lX %s\nYer: %s\n\nRapor: %s\nDump: %s\n(KnightOnLine.exe klasorunde)",
                  ep->ExceptionRecord->ExceptionCode,
                  CodeName(ep->ExceptionRecord->ExceptionCode) ? CodeName(ep->ExceptionRecord->ExceptionCode) : "",
                  where, txtName, dmpName);
        (oMessageBoxA ? oMessageBoxA : MessageBoxA)(nullptr, msg, "Crash Trap", MB_OK | MB_ICONERROR | MB_SYSTEMMODAL);
        return EXCEPTION_EXECUTE_HANDLER;
    }

    // Armed only after the exe has unpacked itself: the packer raises exceptions on purpose while
    // unpacking. The game then sets its own unhandled filter; keep ours on top.
    DWORD WINAPI KeepFilter(LPVOID)
    {
        // "exe unpacked": recv function has its prologue, or the recv hook already put a jmp there
        static const BYTE* KO_RECV_FNC = (const BYTE*)0x0084D0B0;
        static const BYTE  kRecvPrologue[5] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF };
        for (int i = 0; i < 1200 && memcmp(KO_RECV_FNC, kRecvPrologue, 5) != 0 && KO_RECV_FNC[0] != 0xE9; i++) Sleep(100);
        AddVectoredExceptionHandler(1, VectoredHandler);
        PatchFastFail();
        Log("CRASHTRAP: armed (exception logging + crash filter)");
        for (;;)
        {
            LPTOP_LEVEL_EXCEPTION_FILTER prev = SetUnhandledExceptionFilter(UnhandledFilter);
            if (prev && prev != UnhandledFilter)
            {
                char b[96], w[300];
                FormatAddr((DWORD)prev, w, sizeof(w));
                sprintf_s(b, "CRASHTRAP: game filter %.60s replaced", w);
                Log(b);
            }
            Sleep(2000);
        }
    }
}

void CrashTrap_Init()
{
    InitializeCriticalSection(&g_cs);
    GetModuleFileNameA(nullptr, g_dir, MAX_PATH);
    if (char* s = strrchr(g_dir, '\\')) *s = 0;

    oMessageBoxA = (tMessageBoxA)HookExport("MessageBoxA", hkMessageBoxA);
    oMessageBoxW = (tMessageBoxW)HookExport("MessageBoxW", hkMessageBoxW);
    CloseHandle(CreateThread(nullptr, 0, KeepFilter, nullptr, 0, nullptr));

    char b[128];
    sprintf_s(b, "CRASHTRAP: installed (MessageBoxA=%s MessageBoxW=%s) -> crash_trap.log",
              oMessageBoxA ? "ok" : "NO", oMessageBoxW ? "ok" : "NO");
    Log(b);
}
